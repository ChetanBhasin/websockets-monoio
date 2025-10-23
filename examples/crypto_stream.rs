//! Cryptocurrency exchange WebSocket streaming example.
//!
//! This example demonstrates connecting to a cryptocurrency exchange
//! WebSocket API and streaming real-time trade data.

use anyhow::Result;
use fastwebsockets::{Frame, OpCode};
use websockets_monoio::WsClient;

#[monoio::main]
async fn main() -> Result<()> {
    println!("Connecting to Binance WebSocket stream...");

    // Connect to Binance ticker stream
    let mut client =
        WsClient::connect("wss://stream.binance.com:9443/ws/btcusdt@trade", &[]).await?;

    println!("Connected! Subscribing to BTC/USDT trades...");

    // Subscribe to trade stream
    let subscribe = r#"{"method":"SUBSCRIBE","params":["btcusdt@trade"],"id":1}"#;
    client
        .ws
        .write_frame(Frame::text(subscribe.as_bytes().into()))
        .await?;

    println!("Subscription sent. Streaming trade data (Ctrl+C to stop):");

    let mut count = 0;
    // Stream trade data for a limited time in the example
    loop {
        let frame = client.ws.read_frame().await?;
        match frame.opcode {
            OpCode::Text => {
                let text = std::str::from_utf8(&frame.payload)?;

                // Parse and display trade data (simplified)
                if text.contains("\"T\"") {
                    count += 1;
                    println!("Trade #{}: {}", count, text);

                    // Stop after 10 trades for example purposes
                    if count >= 10 {
                        println!("Received 10 trades, stopping example.");
                        break;
                    }
                } else {
                    println!("Subscription response: {}", text);
                }
            }
            OpCode::Binary => {
                println!("Received binary frame ({} bytes)", frame.payload.len());
            }
            OpCode::Close => {
                println!("Stream closed by server");
                break;
            }
            OpCode::Ping | OpCode::Pong => {
                // Auto-handled by fastwebsockets
            }
            _ => {}
        }
    }

    println!("Example completed!");
    Ok(())
}
