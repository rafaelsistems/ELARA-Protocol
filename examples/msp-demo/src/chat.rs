//! Simple Chat Demo - Minimal working version

use std::io::{self, BufRead, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════╗");
    println!("║     ELARA Simple Chat Demo             ║");
    println!("╚════════════════════════════════════════╝");
    println!();

    // Get name
    print!("Your name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();

    // Get port
    print!("Your port: ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port: u16 = port_str.trim().parse().unwrap_or(9000);

    // Get peer
    print!("Peer address (or empty): ");
    io::stdout().flush()?;
    let mut peer_str = String::new();
    io::stdin().read_line(&mut peer_str)?;
    let peer: Option<SocketAddr> = peer_str.trim().parse().ok();

    // Bind socket
    let socket = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", port)).await?);
    println!("\nListening on port {}", port);

    // Store peer address
    let peer_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(peer));

    // Send initial hello if peer specified
    if let Some(addr) = peer {
        let hello = format!("JOIN:{}", name);
        socket.send_to(hello.as_bytes(), addr).await?;
        println!("Sent JOIN to {}", addr);
    }

    // Spawn receiver
    let socket_recv = socket.clone();
    let peer_addr_recv = peer_addr.clone();
    let my_name = name.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, from)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);

                    if let Some(peer_name) = msg.strip_prefix("JOIN:") {
                        println!("\n✅ {} joined from {}", peer_name, from);

                        // Store peer and send welcome back
                        *peer_addr_recv.lock().await = Some(from);
                        let welcome = format!("WELCOME:{}", my_name);
                        let _ = socket_recv.send_to(welcome.as_bytes(), from).await;

                        print!("> ");
                        let _ = io::stdout().flush();
                    } else if let Some(peer_name) = msg.strip_prefix("WELCOME:") {
                        println!("\n✅ Connected to {}", peer_name);
                        *peer_addr_recv.lock().await = Some(from);
                        print!("> ");
                        let _ = io::stdout().flush();
                    } else if let Some(content) = msg.strip_prefix("MSG:") {
                        println!("\n💬 {}", content);
                        print!("> ");
                        let _ = io::stdout().flush();
                    }
                }
                Err(e) => {
                    eprintln!("Receive error: {}", e);
                    break;
                }
            }
        }
    });

    println!("\nType messages and press Enter to send. Type 'quit' to exit.\n");
    print!("> ");
    io::stdout().flush()?;

    // Read input in blocking thread
    let socket_send = socket.clone();
    let peer_addr_send = peer_addr.clone();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();

        if line == "quit" || line == "/quit" {
            break;
        }

        if !line.is_empty() {
            let peer = peer_addr_send.lock().await;
            if let Some(addr) = *peer {
                let msg = format!("MSG:[{}] {}", name, line);
                socket_send.send_to(msg.as_bytes(), addr).await?;
                println!("📤 You: {}", line);
            } else {
                println!("⚠️  No peer connected yet");
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }

    println!("Goodbye!");
    Ok(())
}
