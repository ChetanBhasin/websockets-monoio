use base64::{Engine as _, engine::general_purpose::STANDARD as b64};
use httparse::Status;
use memchr::memmem::Finder;
use monoio_compat::{AsyncReadExt, AsyncWriteExt};
use rand::RngCore;
use sha1::{Digest, Sha1};
use smallvec::SmallVec;
use std::io::{Error as IoError, ErrorKind};

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(thiserror::Error, Debug)]
pub enum UpgradeErr {
    #[error("eof during handshake")]
    Eof,
    #[error("oversized handshake")]
    Oversized,
    #[error("non-101 status line")]
    Status,
    #[error("missing upgrade headers")]
    Headers,
    #[error("bad Sec-WebSocket-Accept")]
    Accept,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

pub struct ClientKey {
    pub sec_websocket_key: String,
    pub expected_accept: String,
}

pub fn generate_client_key() -> ClientKey {
    let mut key_bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut key_bytes);
    let sec_websocket_key = b64.encode(key_bytes);

    let mut sha1 = Sha1::new();
    sha1.update(sec_websocket_key.as_bytes());
    sha1.update(WS_GUID.as_bytes());
    let expected_accept = b64.encode(sha1.finalize());

    ClientKey {
        sec_websocket_key,
        expected_accept,
    }
}

pub async fn write_request<S>(
    stream: &mut S,
    host: &str,
    path_and_query: &str,
    sec_websocket_key: &str,
    extra_headers: &[(&str, &str)],
) -> Result<(), UpgradeErr>
where
    S: AsyncWriteExt + Unpin,
{
    const REQUEST_PREFIX: &[u8] = b"GET ";
    const REQUEST_SUFFIX: &[u8] = b" HTTP/1.1\r\nHost: ";
    const UPGRADE_HEADERS: &[u8] = b"\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: ";
    const HEADER_SEPARATOR: &[u8] = b": ";
    const CRLF: &[u8] = b"\r\n";

    let extra_headers_len = extra_headers.iter().try_fold(0usize, |acc, (k, v)| {
        acc.checked_add(k.as_bytes().len())
            .and_then(|len| len.checked_add(HEADER_SEPARATOR.len()))
            .and_then(|len| len.checked_add(v.as_bytes().len()))
            .and_then(|len| len.checked_add(CRLF.len()))
            .ok_or_else(|| {
                UpgradeErr::Io(IoError::new(
                    ErrorKind::Other,
                    "extra headers exceed maximum buffer size",
                ))
            })
    })?;

    let base_len = REQUEST_PREFIX.len()
        + path_and_query.as_bytes().len()
        + REQUEST_SUFFIX.len()
        + host.as_bytes().len()
        + UPGRADE_HEADERS.len()
        + sec_websocket_key.as_bytes().len()
        + CRLF.len() // after Sec-WebSocket-Key
        + CRLF.len(); // terminating CRLF

    let total_len = base_len.checked_add(extra_headers_len).ok_or_else(|| {
        UpgradeErr::Io(IoError::new(
            ErrorKind::Other,
            "request headers exceed maximum buffer size",
        ))
    })?;

    let mut buffer = SmallVec::<[u8; 512]>::new();
    buffer.try_reserve(total_len).map_err(|_| {
        UpgradeErr::Io(IoError::new(
            ErrorKind::Other,
            "failed to reserve request buffer",
        ))
    })?;

    buffer.extend_from_slice(REQUEST_PREFIX);
    buffer.extend_from_slice(path_and_query.as_bytes());
    buffer.extend_from_slice(REQUEST_SUFFIX);
    buffer.extend_from_slice(host.as_bytes());
    buffer.extend_from_slice(UPGRADE_HEADERS);
    buffer.extend_from_slice(sec_websocket_key.as_bytes());
    buffer.extend_from_slice(CRLF);

    for (k, v) in extra_headers {
        buffer.extend_from_slice(k.as_bytes());
        buffer.extend_from_slice(HEADER_SEPARATOR);
        buffer.extend_from_slice(v.as_bytes());
        buffer.extend_from_slice(CRLF);
    }

    buffer.extend_from_slice(CRLF);

    stream.write_all(&buffer).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn read_response<S>(stream: &mut S, expected_accept: &str) -> Result<(), UpgradeErr>
where
    S: AsyncReadExt + Unpin,
{
    let mut hdr = Vec::with_capacity(2048);
    let mut chunk = [0u8; 1024];
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let finder = Finder::new(b"\r\n\r\n");
    let mut scan_pos = 0;

    loop {
        if finder.find(&hdr[scan_pos..]).is_some() {
            break;
        }

        scan_pos = hdr.len().saturating_sub(3);

        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Err(UpgradeErr::Eof);
        }

        hdr.extend_from_slice(&chunk[..n]);
        if hdr.len() > 16 * 1024 {
            return Err(UpgradeErr::Oversized);
        }
    }

    let mut response = httparse::Response::new(&mut headers);
    let status = {
        let data: &[u8] = &hdr;
        response.parse(data)
    };
    match status {
        Ok(Status::Complete(_header_len)) => {
            if response.code != Some(101) {
                return Err(UpgradeErr::Status);
            }

            let connection =
                find_header(response.headers, "Connection").ok_or(UpgradeErr::Headers)?;
            if !header_has_token(connection, "upgrade")? {
                return Err(UpgradeErr::Headers);
            }

            let upgrade = find_header(response.headers, "Upgrade").ok_or(UpgradeErr::Headers)?;
            if !value_eq_ascii(upgrade, "websocket")? {
                return Err(UpgradeErr::Headers);
            }

            let accept =
                find_header(response.headers, "Sec-WebSocket-Accept").ok_or(UpgradeErr::Headers)?;
            let accept_str = std::str::from_utf8(accept)?;
            if accept_str != expected_accept {
                return Err(UpgradeErr::Accept);
            }

            Ok(())
        }
        _ => Err(UpgradeErr::Headers),
    }
}

fn find_header<'a>(headers: &'a [httparse::Header<'a>], name: &str) -> Option<&'a [u8]> {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value)
}

fn value_eq_ascii(value: &[u8], token: &str) -> Result<bool, std::str::Utf8Error> {
    Ok(std::str::from_utf8(value)?.eq_ignore_ascii_case(token))
}

fn header_has_token(value: &[u8], token: &str) -> Result<bool, std::str::Utf8Error> {
    let text = std::str::from_utf8(value)?;
    Ok(text
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case(token)))
}
