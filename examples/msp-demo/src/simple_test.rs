//! Simple UDP test to verify networking works

use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: simple_test <port> [peer_addr]");
        println!("Example server: simple_test 9000");
        println!("Example client: simple_test 9001 127.0.0.1:9000");
        return Ok(());
    }

    let port: u16 = args[1].parse()?;
    let peer: Option<SocketAddr> = args.get(2).and_then(|s| s.parse().ok());

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Listening on port {}", port);

    if let Some(peer_addr) = peer {
        println!("Sending HELLO to {}", peer_addr);
        socket.send_to(b"HELLO", peer_addr).await?;
        println!("Sent!");
    }

    println!("Waiting for messages...");
    let mut buf = [0u8; 1024];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, from)) => {
                let msg = String::from_utf8_lossy(&buf[..len]);
                println!("Received from {}: {}", from, msg);

                // Echo back
                let reply = format!("ACK: {}", msg);
                socket.send_to(reply.as_bytes(), from).await?;
                println!("Sent reply to {}", from);
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
