use base64::{Engine as _, engine::general_purpose::STANDARD as b64};
use httparse::Status;
use monoio_compat::{AsyncReadExt, AsyncWriteExt};
use rand::RngCore;
use sha1::{Digest, Sha1};

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
    // Write HTTP request line by line to avoid string allocation
    stream.write_all(b"GET ").await?;
    stream.write_all(path_and_query.as_bytes()).await?;
    stream.write_all(b" HTTP/1.1\r\nHost: ").await?;
    stream.write_all(host.as_bytes()).await?;
    stream
        .write_all(
            b"\r\nUpgrade: websocket\r\n\
          Connection: Upgrade\r\n\
          Sec-WebSocket-Version: 13\r\n\
          Sec-WebSocket-Key: ",
        )
        .await?;
    stream.write_all(sec_websocket_key.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;

    // Write extra headers
    for (k, v) in extra_headers {
        stream.write_all(k.as_bytes()).await?;
        stream.write_all(b": ").await?;
        stream.write_all(v.as_bytes()).await?;
        stream.write_all(b"\r\n").await?;
    }

    // End headers
    stream.write_all(b"\r\n").await?;
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

    while !hdr.windows(4).any(|w| w == b"\r\n\r\n") {
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
