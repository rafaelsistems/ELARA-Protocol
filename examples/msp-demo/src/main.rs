//! ELARA MSP Demo Application
//!
//! Demonstrates the Minimal Survival Profile:
//! - Text chat (real-time)
//! - Presence indicators
//! - Typing indicators
//! - Voice state (simulated)
//! - Graceful degradation visualization

mod node;
mod ui;
mod network;

use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use node::DemoNode;
use ui::DemoUI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           ELARA MSP Demo - Text + Voice                    ║");
    println!("║     Minimal Survival Profile Demonstration                 ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Get user name
    print!("Enter your name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("Name cannot be empty!");
        return Ok(());
    }

    // Get local port
    print!("Enter local port (default 9000): ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port: u16 = port_str.trim().parse().unwrap_or(9000);

    // Get peer address (optional)
    print!("Enter peer address to connect (e.g., 127.0.0.1:9001, or empty to wait): ");
    io::stdout().flush()?;
    let mut peer_str = String::new();
    io::stdin().read_line(&mut peer_str)?;
    let peer_addr: Option<SocketAddr> = peer_str.trim().parse().ok();

    println!();
    println!("Starting node on port {}...", port);

    // Create the demo node
    let node = Arc::new(Mutex::new(DemoNode::new(name.clone(), port).await?));

    // Connect to peer if specified
    if let Some(addr) = peer_addr {
        println!("Connecting to peer at {}...", addr);
        node.lock().await.connect_peer(addr).await?;
    }

    println!();
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  Commands:                                                 ║");
    println!("║    /msg <text>  - Send a message                          ║");
    println!("║    /voice       - Toggle voice (simulated)                ║");
    println!("║    /presence    - Show presence info                      ║");
    println!("║    /degrade     - Simulate network degradation            ║");
    println!("║    /recover     - Recover from degradation                ║");
    println!("║    /status      - Show node status                        ║");
    println!("║    /peers       - List connected peers                    ║");
    println!("║    /quit        - Exit                                    ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Create UI
    let mut ui = DemoUI::new(node.clone());

    // Run the main loop
    ui.run().await?;

    println!("Goodbye!");
    Ok(())
}
