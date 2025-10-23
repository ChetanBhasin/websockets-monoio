use anyhow::Result;
use fastwebsockets::{Role, WebSocket};
use monoio::net::TcpStream;
use monoio_compat::{AsyncRead, AsyncWrite, StreamWrapper};

use crate::http_upgrade::{generate_client_key, read_response, write_request};
use crate::tls::{connect_wss, default_connector};
use crate::url::{Scheme, parse_ws_or_wss};

/// A unified IO stream that can be plain TCP or TLS over TCP, both wrapped
/// in `monoio_compat::StreamWrapper` to provide AsyncRead/AsyncWrite.
#[allow(clippy::large_enum_variant)]
pub enum AnyStream {
    Plain(StreamWrapper<TcpStream>),
    Tls(StreamWrapper<monoio_rustls::ClientTlsStream<TcpStream>>),
}

impl monoio_compat::AsyncRead for AnyStream {
    fn poll_read(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> core::task::Poll<std::io::Result<()>> {
        unsafe {
            match self.get_unchecked_mut() {
                AnyStream::Plain(s) => core::pin::Pin::new_unchecked(s).poll_read(cx, buf),
                AnyStream::Tls(s) => core::pin::Pin::new_unchecked(s).poll_read(cx, buf),
            }
        }
    }
}

impl monoio_compat::AsyncWrite for AnyStream {
    fn poll_write(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &[u8],
    ) -> core::task::Poll<Result<usize, std::io::Error>> {
        unsafe {
            match self.get_unchecked_mut() {
                AnyStream::Plain(s) => core::pin::Pin::new_unchecked(s).poll_write(cx, buf),
                AnyStream::Tls(s) => core::pin::Pin::new_unchecked(s).poll_write(cx, buf),
            }
        }
    }

    fn poll_flush(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Result<(), std::io::Error>> {
        unsafe {
            match self.get_unchecked_mut() {
                AnyStream::Plain(s) => core::pin::Pin::new_unchecked(s).poll_flush(cx),
                AnyStream::Tls(s) => core::pin::Pin::new_unchecked(s).poll_flush(cx),
            }
        }
    }

    fn poll_shutdown(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Result<(), std::io::Error>> {
        unsafe {
            match self.get_unchecked_mut() {
                AnyStream::Plain(s) => core::pin::Pin::new_unchecked(s).poll_shutdown(cx),
                AnyStream::Tls(s) => core::pin::Pin::new_unchecked(s).poll_shutdown(cx),
            }
        }
    }
}

/// Exposed stream type used by `WsClient`.
pub type WsStream = AnyStream;

pub struct WsClient {
    pub ws: WebSocket<WsStream>,
}

impl WsClient {
    /// Connect to a `ws://` or `wss://` URL and complete the WebSocket handshake.
    pub async fn connect(url: &str, extra_headers: &[(&str, &str)]) -> Result<Self> {
        let u = parse_ws_or_wss(url)?;

        // Establish underlying transport (TCP or TLS over TCP)
        let mut stream = match u.scheme {
            Scheme::Ws => {
                let tcp = TcpStream::connect((u.host, u.port)).await?;
                AnyStream::Plain(StreamWrapper::new(tcp))
            }
            Scheme::Wss => {
                let connector = default_connector();
                let tls = connect_wss(u.host, u.port, connector).await?;
                AnyStream::Tls(StreamWrapper::new(tls))
            }
        };

        // HTTP Upgrade handshake
        let key = generate_client_key();
        write_request(
            &mut stream,
            u.host,
            u.path_and_query,
            &key.sec_websocket_key,
            extra_headers,
        )
        .await?;
        read_response(&mut stream, &key.expected_accept).await?;

        // Switch to WebSocket
        let mut ws = WebSocket::after_handshake(stream, Role::Client);
        ws.set_auto_close(true);
        ws.set_auto_pong(true);
        if matches!(u.scheme, Scheme::Wss) {
            // TLS backends generally buffer writes, so gathering is less effective.
            ws.set_writev(false);
        }

        Ok(Self { ws })
    }

    pub fn into_inner(self) -> WebSocket<WsStream> {
        self.ws
    }
}

// Convenience trait bound if you want to reuse upgrade for different streams.
pub trait TokioIo: AsyncRead + AsyncWrite + Unpin {}
impl<T: AsyncRead + AsyncWrite + Unpin> TokioIo for T {}
