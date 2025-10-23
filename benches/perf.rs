use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use criterion::{Criterion, criterion_group, criterion_main};
use fastwebsockets::{Frame, OpCode, Role, WebSocket};
use monoio::net::{TcpListener, TcpStream};
use monoio_compat::{AsyncReadExt, AsyncWriteExt, StreamWrapper};
use sha1::{Digest, Sha1};
use websockets_monoio::WsClient;

const LISTEN_ADDR: &str = "127.0.0.1:0";
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

struct EchoServer {
    addr: SocketAddr,
    running: Arc<AtomicBool>,
    handle: monoio::task::JoinHandle<()>,
}

impl EchoServer {
    fn addr(&self) -> SocketAddr {
        self.addr
    }

    async fn shutdown(self) {
        self.running.store(false, Ordering::Release);
        let addr = self.addr;
        let _ = TcpStream::connect(addr).await;
        let _ = self.handle.await;
    }
}

async fn start_echo_server() -> Result<EchoServer> {
    let listener = TcpListener::bind(LISTEN_ADDR)?;
    let addr = listener.local_addr()?;
    let running = Arc::new(AtomicBool::new(true));
    let accept_flag = running.clone();

    let handle = monoio::spawn(async move {
        let listener = listener;
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    if !accept_flag.load(Ordering::Relaxed) {
                        drop(stream);
                        break;
                    }
                    monoio::spawn(async move {
                        if let Err(err) = handle_connection(stream).await {
                            eprintln!("benchmark echo connection error: {err:#}");
                        }
                    });
                }
                Err(err) => {
                    eprintln!("benchmark echo accept error: {err:#}");
                    break;
                }
            }
        }
    });

    Ok(EchoServer {
        addr,
        running,
        handle,
    })
}

async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut stream = StreamWrapper::new(stream);
    let mut header_bytes = Vec::with_capacity(1024);
    let mut buf = [0u8; 1024];

    loop {
        let read = stream.read(&mut buf).await?;
        if read == 0 {
            bail!("unexpected eof during websocket handshake");
        }
        header_bytes.extend_from_slice(&buf[..read]);
        if header_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if header_bytes.len() > 16 * 1024 {
            bail!("received oversized websocket handshake");
        }
    }

    let header_text =
        std::str::from_utf8(&header_bytes).context("handshake bytes were not valid utf-8")?;
    let sec_key = extract_sec_websocket_key(header_text)
        .context("handshake missing Sec-WebSocket-Key header")?;
    let accept = compute_accept_key(sec_key);

    // Minimal HTTP 101 response
    stream
        .write_all(b"HTTP/1.1 101 Switching Protocols\r\n")
        .await?;
    stream.write_all(b"Connection: Upgrade\r\n").await?;
    stream.write_all(b"Upgrade: websocket\r\n").await?;
    stream.write_all(b"Sec-WebSocket-Accept: ").await?;
    stream.write_all(accept.as_bytes()).await?;
    stream.write_all(b"\r\n\r\n").await?;
    stream.flush().await?;

    let mut ws = WebSocket::after_handshake(stream, Role::Server);
    ws.set_auto_close(true);
    ws.set_auto_pong(true);
    ws.set_writev(false);

    while let Ok(frame) = ws.read_frame().await {
        match frame.opcode {
            OpCode::Text | OpCode::Binary => {
                if let Err(err) = ws.write_frame(frame).await {
                    eprintln!("benchmark echo write error: {err:#}");
                    break;
                };
            }
            OpCode::Close => break,
            _ => {}
        }
    }

    Ok(())
}

fn extract_sec_websocket_key(text: &str) -> Option<&str> {
    text.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("Sec-WebSocket-Key") {
            Some(value.trim())
        } else {
            None
        }
    })
}

fn compute_accept_key(key: &str) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(key.as_bytes());
    sha1.update(WS_GUID.as_bytes());
    BASE64.encode(sha1.finalize())
}

fn bench_connect(c: &mut Criterion) {
    let mut group = c.benchmark_group("connect");
    group.bench_function("ws_connect", |b| {
        let mut runtime = monoio::RuntimeBuilder::<monoio::FusionDriver>::new()
            .enable_all()
            .build()
            .expect("failed to build monoio runtime");
        let server = runtime
            .block_on(start_echo_server())
            .expect("failed to start echo server");
        let url = format!("ws://{}/bench", server.addr());

        b.iter_custom(|iters| {
            runtime.block_on(async {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let start = Instant::now();
                    let mut client = WsClient::connect(&url, &[])
                        .await
                        .expect("websocket connect");
                    total += start.elapsed();

                    let _ = client.ws.write_frame(Frame::close(1000, &[])).await;
                }
                total
            })
        });

        runtime.block_on(server.shutdown());
    });
    group.finish();
}

fn bench_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip");

    run_round_trip_case(&mut group, "text_32b", 32, FrameKind::Text);
    run_round_trip_case(&mut group, "binary_1kb", 1024, FrameKind::Binary);
    run_round_trip_case(&mut group, "binary_64kb", 64 * 1024, FrameKind::Binary);

    group.finish();
}

enum FrameKind {
    Text,
    Binary,
}

fn run_round_trip_case(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    label: &str,
    payload_size: usize,
    frame_kind: FrameKind,
) {
    let mut runtime = monoio::RuntimeBuilder::<monoio::FusionDriver>::new()
        .enable_all()
        .build()
        .expect("failed to build monoio runtime");
    let server = runtime
        .block_on(start_echo_server())
        .expect("failed to start echo server");
    let url = format!("ws://{}/bench", server.addr());

    let mut ws = runtime.block_on(async {
        WsClient::connect(&url, &[])
            .await
            .expect("websocket connect")
            .into_inner()
    });

    let payload = vec![b'x'; payload_size];

    group.bench_function(label, |b| {
        b.iter_custom(|iters| {
            runtime.block_on(async {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let start = Instant::now();
                    match frame_kind {
                        FrameKind::Text => {
                            ws.write_frame(Frame::text(payload.as_slice().into()))
                                .await
                                .expect("write text frame");
                        }
                        FrameKind::Binary => {
                            ws.write_frame(Frame::binary(payload.as_slice().into()))
                                .await
                                .expect("write binary frame");
                        }
                    }

                    let frame = ws.read_frame().await.expect("read frame");
                    let opcode = frame.opcode;
                    match frame_kind {
                        FrameKind::Text => assert_eq!(opcode, OpCode::Text),
                        FrameKind::Binary => assert_eq!(opcode, OpCode::Binary),
                    }
                    assert_eq!(frame.payload.len(), payload.len());

                    total += start.elapsed();
                }
                total
            })
        });
    });

    runtime.block_on(async {
        let _ = ws.write_frame(Frame::close(1000, &[])).await;
        let _ = ws.read_frame().await;
    });

    runtime.block_on(server.shutdown());
}

criterion_group!(benches, bench_connect, bench_round_trip);
criterion_main!(benches);
