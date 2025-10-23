# websockets-monoio

[![Crates.io](https://img.shields.io/crates/v/websockets-monoio.svg)](https://crates.io/crates/websockets-monoio)
[![lib.rs](https://img.shields.io/badge/lib.rs-v0.1.1-orange.svg)](https://lib.rs/crates/websockets-monoio)
[![Documentation](https://docs.rs/websockets-monoio/badge.svg)](https://docs.rs/websockets-monoio)
[![License](https://img.shields.io/crates/l/websockets-monoio.svg)](https://github.com/ChetanBhasin/websockets-monoio#license)
[![Downloads](https://img.shields.io/crates/d/websockets-monoio.svg)](https://crates.io/crates/websockets-monoio)

A high-performance WebSocket client for the [`monoio`](https://docs.rs/monoio) async runtime. It dials both `ws://` and `wss://` endpoints, performs the HTTP upgrade handshake, and hands you a fully configured [`fastwebsockets_monoio::WebSocket`] client stream.

**Note:** Documentation is AI generated.

## Highlights

- **Monoio-first**: Uses `io_uring` on Linux via monoio for low-latency networking.
- **TLS out of the box**: `wss://` connections use `monoio-rustls` with the Mozilla root store.
- **Zero-copy friendly**: Frame writes avoid intermediate allocations whenever possible.
- **Safe defaults**: Auto close and auto pong are enabled; TLS writev is disabled for compatibility.
- **Minimal surface area**: One `WsClient::connect` helper plus re-exported stream types if you want lower-level control.

## Install

Add the crate and its companion dependencies to your project:

```toml
[dependencies]
websockets-monoio = "0.1.1"
monoio = "0.2"
fastwebsockets-monoio = "0.10"
anyhow = "1.0"
```

The crate targets Rust 1.90.0 or newer (see `Cargo.toml`).

## Usage

### Basic WebSocket connection

```rust
use fastwebsockets_monoio::{Frame, OpCode};
use websockets_monoio::WsClient;

#[monoio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = WsClient::connect("wss://echo.websocket.org/", &[]).await?;

    client
        .ws
        .write_frame(Frame::text("Hello, WebSocket!".as_bytes().into()))
        .await?;

    let frame = client.ws.read_frame().await?;
    if let OpCode::Text = frame.opcode {
        println!("Received: {}", std::str::from_utf8(&frame.payload)?);
    }

    Ok(())
}
```

### Streaming example

```rust
use fastwebsockets_monoio::{Frame, OpCode};
use websockets_monoio::WsClient;

#[monoio::main]
async fn main() -> anyhow::Result<()> {
    let mut client =
        WsClient::connect("wss://stream.binance.com:9443/ws/btcusdt@trade", &[]).await?;

    let subscribe = r#"{"method":"SUBSCRIBE","params":["btcusdt@trade"],"id":1}"#;
    client
        .ws
        .write_frame(Frame::text(subscribe.as_bytes().into()))
        .await?;

    loop {
        let frame = client.ws.read_frame().await?;
        match frame.opcode {
            OpCode::Text => println!("{}", std::str::from_utf8(&frame.payload)?),
            OpCode::Binary => println!("Binary frame ({} bytes)", frame.payload.len()),
            OpCode::Close => break,
            _ => {}
        }
    }

    Ok(())
}
```

### Custom request headers

```rust
use websockets_monoio::WsClient;

#[monoio::main]
async fn main() -> anyhow::Result<()> {
    let client = WsClient::connect(
        "wss://api.example.com/socket",
        &[
            ("Authorization", "Bearer your-token"),
            ("User-Agent", "your-app/1.0"),
        ],
    )
    .await?;

    // Use client.ws ...
    drop(client);
    Ok(())
}
```

Further examples live in `examples/`:

- `cargo run --example echo_client`
- `cargo run --example crypto_stream`

## API overview

- `WsClient::connect(url, extra_headers)` performs DNS resolution, TCP/TLS setup, and the HTTP upgrade handshake before returning a `WebSocket<WsStream>`.
- `WsClient::into_inner()` gives direct access to the underlying `fastwebsockets_monoio::WebSocket`.
- `WsStream` is the enum used by the client (`Plain` TCP or `Tls` over TCP). It implements `monoio_compat::AsyncRead` and `AsyncWrite`.
- Supporting modules such as `http_upgrade`, `tls`, and `url` are re-exported for advanced use-cases if you want to build your own handshake flow.

Errors from `WsClient::connect` use `anyhow::Result`, allowing full context while still being compatible with other error handling strategies.

## Benchmarks

Benchmarks live in `benches/perf.rs` and run locally without external services. Launch them with:

```bash
cargo bench
```

What you get:

- `connect/ws_connect` measures full handshake latency against an in-process monoio echo server.
- `round_trip/*` tests send-and-receive latency for text and binary frames of varying sizes.

Results depend on kernel support for `io_uring`; Linux 5.1+ is recommended for representative numbers.

## Platform notes

- **Linux**: Full support with `io_uring`. This is the primary target.
- **macOS / Windows**: Works via monoio’s fallback driver, but without `io_uring` optimisations.

TLS connections use `rustls` with the Mozilla CA bundle (`webpki-roots`). A global `TlsConnector` is reused across calls to keep setup cheap.

## Contributing

Issues and PRs are welcome. By contributing you agree to license your work under MIT OR Apache-2.0, the same as the rest of the project.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
