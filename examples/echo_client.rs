//! A simple echo client example demonstrating basic WebSocket usage.
//!
//! This example connects to a WebSocket echo server, sends a message,
//! and prints the echoed response.

use anyhow::Result;
use fastwebsockets::{Frame, OpCode};
use websockets_monoio::WsClient;

#[monoio::main]
async fn main() -> Result<()> {
    println!("Connecting to WebSocket echo server...");

    // Connect to the echo server
    let mut client = WsClient::connect("wss://echo.websocket.org/", &[]).await?;

    println!("Connected! Sending message...");

    // Send a test message
    let message = "Hello from websockets-monoio!";
    client
        .ws
        .write_frame(Frame::text(message.as_bytes().into()))
        .await?;

    println!("Message sent: {}", message);
    println!("Waiting for echo...");

    // Read the echoed response
    let frame = client.ws.read_frame().await?;
    match frame.opcode {
        OpCode::Text => {
            let text = std::str::from_utf8(&frame.payload)?;
            println!("Echo received: {}", text);
        }
        OpCode::Binary => {
            println!("Received binary frame ({} bytes)", frame.payload.len());
        }
        OpCode::Close => {
            println!("Server closed the connection");
        }
        _ => {
            println!("Received frame type: {:?}", frame.opcode);
        }
    }

    println!("Example completed successfully!");
    Ok(())
}
