//! ELARA MSP Demo - Full version with presence, voice, degradation

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use elara_core::{DegradationLevel, PresenceVector};

/// Peer information
#[derive(Clone, Debug)]
struct Peer {
    name: String,
    voice_active: bool,
}

/// Node state
struct NodeState {
    name: String,
    node_id: u64,
    peers: HashMap<SocketAddr, Peer>,
    presence: PresenceVector,
    degradation: DegradationLevel,
    voice_active: bool,
}

impl NodeState {
    fn new(name: String) -> Self {
        Self {
            name,
            node_id: rand::random(),
            peers: HashMap::new(),
            presence: PresenceVector::full(),
            degradation: DegradationLevel::L0_FullPerception,
            voice_active: false,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           ELARA MSP Demo - Text + Voice                    ║");
    println!("║     Minimal Survival Profile Demonstration                 ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Get name
    print!("Your name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("Name cannot be empty!");
        return Ok(());
    }

    // Get port
    print!("Your port (default 9000): ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port: u16 = port_str.trim().parse().unwrap_or(9000);

    // Get peer
    print!("Peer address (or empty to wait): ");
    io::stdout().flush()?;
    let mut peer_str = String::new();
    io::stdin().read_line(&mut peer_str)?;
    let initial_peer: Option<SocketAddr> = peer_str.trim().parse().ok();

    // Bind socket
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            println!("Failed to bind port {}: {}", port, e);
            println!("Try a different port");
            return Ok(());
        }
    };

    println!("\nListening on port {}", port);
    if initial_peer.is_none() {
        println!("Waiting for connections... Share: 127.0.0.1:{}", port);
    }

    // State
    let state = Arc::new(Mutex::new(NodeState::new(name.clone())));
    let peer_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(initial_peer));

    // Send initial hello if peer specified
    if let Some(addr) = initial_peer {
        let node_id = state.lock().await.node_id;
        let hello = format!("JOIN:{}:{:016x}", name, node_id);
        socket.send_to(hello.as_bytes(), addr).await?;
        println!("Connecting to {}...", addr);
    }

    // Spawn receiver
    let socket_recv = socket.clone();
    let peer_addr_recv = peer_addr.clone();
    let state_recv = state.clone();
    let my_name = name.clone();

    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            match socket_recv.recv_from(&mut buf).await {
                Ok((len, from)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    let parts: Vec<&str> = msg.splitn(3, ':').collect();

                    if parts.is_empty() {
                        continue;
                    }

                    match parts[0] {
                        "JOIN" if parts.len() >= 2 => {
                            let peer_name = parts[1].to_string();

                            // Add peer
                            {
                                let mut s = state_recv.lock().await;
                                let is_new = !s.peers.contains_key(&from);

                                s.peers.insert(
                                    from,
                                    Peer {
                                        name: peer_name.clone(),
                                        voice_active: false,
                                    },
                                );

                                if is_new {
                                    println!("\n✅ {} joined from {}", peer_name, from);
                                    print!("> ");
                                    let _ = io::stdout().flush();

                                    // Send welcome back
                                    let welcome = format!("WELCOME:{}:{:016x}", my_name, s.node_id);
                                    let _ = socket_recv.send_to(welcome.as_bytes(), from).await;
                                }
                            }

                            *peer_addr_recv.lock().await = Some(from);
                        }
                        "WELCOME" if parts.len() >= 2 => {
                            let peer_name = parts[1].to_string();

                            {
                                let mut s = state_recv.lock().await;
                                s.peers.insert(
                                    from,
                                    Peer {
                                        name: peer_name.clone(),
                                        voice_active: false,
                                    },
                                );
                            }

                            *peer_addr_recv.lock().await = Some(from);
                            println!("\n✅ Connected to {}", peer_name);
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                        "MSG" if parts.len() >= 2 => {
                            let content = parts[1];
                            println!("\n💬 {}", content);
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                        "TYPING" if parts.len() >= 3 => {
                            let peer_name = parts[1];
                            let typing = parts[2] == "1";

                            if typing {
                                println!("\n✏️  {} is typing...", peer_name);
                                print!("> ");
                                let _ = io::stdout().flush();
                            }
                        }
                        "VOICE" if parts.len() >= 3 => {
                            let peer_name = parts[1];
                            let energy: u8 = parts[2].parse().unwrap_or(0);

                            if energy > 0 {
                                let bars = "█".repeat((energy / 25) as usize);
                                println!("\n🎤 {} speaking: [{}]", peer_name, bars);
                            } else {
                                println!("\n🔇 {} stopped speaking", peer_name);
                            }
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                        "PRESENCE" if parts.len() >= 3 => {
                            let peer_name = parts[1];
                            let score: f32 = parts[2].parse().unwrap_or(1.0);
                            println!("\n👤 {} presence: {:.0}%", peer_name, score * 100.0);
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                        "DEGRADE" if parts.len() >= 3 => {
                            let peer_name = parts[1];
                            let level: u8 = parts[2].parse().unwrap_or(0);
                            let level_name = match level {
                                0 => "L0: Full Perception",
                                1 => "L1: Distorted",
                                2 => "L2: Fragmented",
                                3 => "L3: Symbolic",
                                4 => "L4: Minimal",
                                _ => "L5: Latent",
                            };
                            println!("\n⚠️  {} degraded to {}", peer_name, level_name);
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    eprintln!("Receive error: {}", e);
                    break;
                }
            }
        }
    });

    // Print help
    println!();
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  Commands:                                                 ║");
    println!("║    <text>       - Send a message                          ║");
    println!("║    /voice       - Toggle voice (simulated)                ║");
    println!("║    /presence    - Show presence info                      ║");
    println!("║    /degrade     - Simulate network degradation            ║");
    println!("║    /recover     - Recover from degradation                ║");
    println!("║    /status      - Show node status                        ║");
    println!("║    /peers       - List connected peers                    ║");
    println!("║    /help        - Show this help                          ║");
    println!("║    /quit        - Exit                                    ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    print!("> ");
    io::stdout().flush()?;

    // Input loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        if line == "/quit" || line == "quit" {
            break;
        }

        let peer = *peer_addr.lock().await;

        match line {
            "/voice" => {
                let mut s = state.lock().await;
                s.voice_active = !s.voice_active;
                let energy = if s.voice_active { 200u8 } else { 0u8 };

                if let Some(addr) = peer {
                    let msg = format!("VOICE:{}:{}", s.name, energy);
                    socket.send_to(msg.as_bytes(), addr).await?;
                }

                if s.voice_active {
                    println!("🎤 Voice ON - speaking...");
                } else {
                    println!("🔇 Voice OFF");
                }
            }
            "/presence" => {
                let s = state.lock().await;
                println!("┌─────────────────────────────────────┐");
                println!("│         Presence Vector             │");
                println!("├─────────────────────────────────────┤");
                println!(
                    "│ Liveness:     {:.0}%                  │",
                    s.presence.liveness * 100.0
                );
                println!(
                    "│ Immediacy:    {:.0}%                  │",
                    s.presence.immediacy * 100.0
                );
                println!(
                    "│ Coherence:    {:.0}%                  │",
                    s.presence.coherence * 100.0
                );
                println!(
                    "│ Relational:   {:.0}%                  │",
                    s.presence.relational_continuity * 100.0
                );
                println!(
                    "│ Emotional:    {:.0}%                  │",
                    s.presence.emotional_bandwidth * 100.0
                );
                println!("├─────────────────────────────────────┤");
                println!(
                    "│ Overall Score: {:.0}%                 │",
                    s.presence.score() * 100.0
                );
                println!("└─────────────────────────────────────┘");
            }
            "/degrade" => {
                let mut s = state.lock().await;
                let new_level = s
                    .degradation
                    .degrade()
                    .unwrap_or(DegradationLevel::L5_LatentPresence);
                s.degradation = new_level;

                // Update presence based on degradation
                let factor = 1.0 - (new_level.level() as f32 * 0.15);
                s.presence = PresenceVector::new(factor, factor, factor, factor, factor);

                let level_num = new_level.level();

                if let Some(addr) = peer {
                    let msg = format!("DEGRADE:{}:{}", s.name, level_num);
                    socket.send_to(msg.as_bytes(), addr).await?;
                }

                print_degradation_ladder(new_level);
            }
            "/recover" => {
                let mut s = state.lock().await;
                s.degradation = DegradationLevel::L0_FullPerception;
                s.presence = PresenceVector::full();

                if let Some(addr) = peer {
                    let msg = format!("DEGRADE:{}:0", s.name);
                    socket.send_to(msg.as_bytes(), addr).await?;
                }

                println!("✅ Recovered to L0: Full Perception");
                print_degradation_ladder(DegradationLevel::L0_FullPerception);
            }
            "/status" => {
                let s = state.lock().await;
                println!("┌─────────────────────────────────────┐");
                println!("│           Node Status               │");
                println!("├─────────────────────────────────────┤");
                println!("│ Name: {}                            ", s.name);
                println!("│ Node ID: {:016x}        ", s.node_id);
                println!("│ Degradation: {:?}      ", s.degradation);
                println!(
                    "│ Voice: {}                           ",
                    if s.voice_active { "ON" } else { "OFF" }
                );
                println!("│ Peers: {}                           ", s.peers.len());
                println!("└─────────────────────────────────────┘");
            }
            "/peers" => {
                let s = state.lock().await;
                if s.peers.is_empty() {
                    println!("No peers connected");
                } else {
                    println!("Connected peers:");
                    for (addr, peer) in &s.peers {
                        println!(
                            "  - {} @ {} (voice: {})",
                            peer.name,
                            addr,
                            if peer.voice_active { "ON" } else { "OFF" }
                        );
                    }
                }
            }
            "/help" => {
                println!("Commands:");
                println!("  <text>     - Send a message");
                println!("  /voice     - Toggle voice");
                println!("  /presence  - Show presence");
                println!("  /degrade   - Simulate degradation");
                println!("  /recover   - Recover to L0");
                println!("  /status    - Show status");
                println!("  /peers     - List peers");
                println!("  /quit      - Exit");
            }
            _ => {
                // Send message
                if let Some(addr) = peer {
                    let s = state.lock().await;
                    let msg = format!("MSG:[{}] {}", s.name, line);
                    socket.send_to(msg.as_bytes(), addr).await?;
                    println!("📤 You: {}", line);
                } else {
                    println!("⚠️  No peer connected");
                }
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }

    println!("Goodbye!");
    Ok(())
}

fn print_degradation_ladder(current: DegradationLevel) {
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│              Degradation Ladder                         │");
    println!("├─────────────────────────────────────────────────────────┤");

    let levels = [
        (
            DegradationLevel::L0_FullPerception,
            "L0: Full Perception",
            20,
        ),
        (
            DegradationLevel::L1_DistortedPerception,
            "L1: Distorted",
            16,
        ),
        (
            DegradationLevel::L2_FragmentedPerception,
            "L2: Fragmented",
            12,
        ),
        (DegradationLevel::L3_SymbolicPresence, "L3: Symbolic", 8),
        (DegradationLevel::L4_MinimalPresence, "L4: Minimal", 4),
        (DegradationLevel::L5_LatentPresence, "L5: Latent", 2),
    ];

    for (level, name, bars) in levels {
        let marker = if level == current { "→" } else { " " };
        let bar = "█".repeat(bars);
        let empty = "░".repeat(20 - bars);
        println!("│ {} {:20} [{}{}] │", marker, name, bar, empty);
    }

    println!("└─────────────────────────────────────────────────────────┘");
}
