//! # websockets-monoio
//!
//! A high-performance WebSocket client library built for the [`monoio`] async runtime
//! using `io_uring` on Linux. This library provides both `ws://` and `wss://` (TLS)
//! support with optimized low-allocation operations and efficient memory usage.
//!
//! ## Features
//!
//! - **🚀 High Performance**: Built on `monoio` runtime with `io_uring` for maximum efficiency on Linux
//! - **🔒 TLS Support**: Full `wss://` support via `monoio-rustls`
//! - **📦 Built for monoio**: Optimized for monoio async runtime
//! - **🛡️ Secure**: Uses `rustls` with `webpki-roots` for certificate validation
//! - **⚡ Low-Allocation**: Zero-copy message sending and optimized connection setup
//! - **🔧 Simple API**: Easy-to-use client interface
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! websockets-monoio = "0.1"
//! monoio = "0.2"
//! fastwebsockets-monoio = "0.10"
//! anyhow = "1.0"
//! ```
//!
//! ## Basic Example
//!
//! ```no_run
//! use fastwebsockets_monoio::{Frame, OpCode};
//! use websockets_monoio::WsClient;
//!
//! #[monoio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to a WebSocket server
//!     let mut client = WsClient::connect(
//!         "wss://echo.websocket.org/",
//!         &[],
//!     ).await?;
//!
//!     // Send a text message
//!     client
//!         .ws
//!         .write_frame(Frame::text("Hello, WebSocket!".as_bytes().into()))
//!         .await?;
//!
//!     // Read the response
//!     let frame = client.ws.read_frame().await?;
//!     match frame.opcode {
//!         OpCode::Text => {
//!             let text = std::str::from_utf8(&frame.payload)?;
//!             println!("Received: {}", text);
//!         }
//!         OpCode::Close => {
//!             println!("Connection closed by server");
//!         }
//!         _ => {}
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Cryptocurrency Exchange Example
//!
//! ```no_run
//! use fastwebsockets_monoio::{Frame, OpCode};
//! use websockets_monoio::WsClient;
//!
//! #[monoio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to Binance ticker stream
//!     let mut client = WsClient::connect(
//!         "wss://stream.binance.com:9443/ws/btcusdt@trade",
//!         &[],
//!     ).await?;
//!
//!     // Subscribe to trades
//!     let subscribe = r#"{"method":"SUBSCRIBE","params":["btcusdt@trade"],"id":1}"#;
//!     client
//!         .ws
//!         .write_frame(Frame::text(subscribe.as_bytes().into()))
//!         .await?;
//!
//!     // Stream trade data
//!     loop {
//!         let frame = client.ws.read_frame().await?;
//!         match frame.opcode {
//!             OpCode::Text => {
//!                 let text = std::str::from_utf8(&frame.payload)?;
//!                 println!("Trade: {}", text);
//!             }
//!             OpCode::Binary => {
//!                 println!("Binary frame ({} bytes)", frame.payload.len());
//!             }
//!             OpCode::Close => {
//!                 println!("Stream closed");
//!                 break;
//!             }
//!             OpCode::Ping | OpCode::Pong => {
//!                 // Auto-handled by fastwebsockets
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Platform Support
//!
//! - **Linux**: Full support with `io_uring` (recommended)
//! - **macOS/Windows**: Limited support (falls back to standard async I/O)
//!
//! For maximum performance, deploy on Linux with kernel version 5.1+ for full `io_uring` support.
//!
//! [`monoio`]: https://docs.rs/monoio

pub mod client;
pub mod http_upgrade;
pub mod tls;
pub mod url;

pub use client::{WsClient, WsStream};
