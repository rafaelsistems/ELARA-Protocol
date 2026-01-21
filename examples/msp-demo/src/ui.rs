//! Demo UI - Terminal-based user interface

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::node::{DemoNode, DemoMessage};

/// Demo UI handler
pub struct DemoUI {
    node: Arc<Mutex<DemoNode>>,
}

impl DemoUI {
    pub fn new(node: Arc<Mutex<DemoNode>>) -> Self {
        Self { node }
    }

    /// Run the main UI loop
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = self.node.clone();
        
        // Spawn network receiver task
        let node_recv = node.clone();
        let recv_handle = tokio::spawn(async move {
            let socket = {
                let n = node_recv.lock().await;
                n.socket()
            };
            
            let mut buf = [0u8; 1500];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, from)) => {
                        let mut n = node_recv.lock().await;
                        let _ = n.process_packet(&buf[..len], from).await;
                    }
                    Err(_) => break,
                }
            }
        });

        // Spawn event display task
        let node_events = node.clone();
        let events_handle = tokio::spawn(async move {
            loop {
                let msg = {
                    let mut n = node_events.lock().await;
                    n.event_rx.recv().await
                };
                
                match msg {
                    Some(DemoMessage::Text { from, content }) => {
                        println!("\n💬 [{}]: {}", from, content);
                        print!("> ");
                        let _ = io::stdout().flush();
                    }
                    Some(DemoMessage::Typing { from, typing }) => {
                        if typing {
                            println!("\n✏️  {} is typing...", from);
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                    }
                    Some(DemoMessage::Voice { from, energy }) => {
                        if energy > 0 {
                            let bars = "█".repeat((energy / 25) as usize);
                            println!("\n🎤 {} speaking: [{}]", from, bars);
                            print!("> ");
                            let _ = io::stdout().flush();
                        }
                    }
                    Some(DemoMessage::Presence { from, presence }) => {
                        println!("\n👤 {} presence: {:.0}%", from, presence.score() * 100.0);
                        print!("> ");
                        let _ = io::stdout().flush();
                    }
                    Some(DemoMessage::PeerJoined { name }) => {
                        println!("\n✅ {} joined the session", name);
                        print!("> ");
                        let _ = io::stdout().flush();
                    }
                    Some(DemoMessage::PeerLeft { name }) => {
                        println!("\n❌ {} left the session", name);
                        print!("> ");
                        let _ = io::stdout().flush();
                    }
                    Some(DemoMessage::Degradation { level }) => {
                        println!("\n⚠️  Degradation level: {:?}", level);
                        print!("> ");
                        let _ = io::stdout().flush();
                    }
                    None => break,
                }
            }
        });

        // Spawn presence broadcast task
        let node_presence = node.clone();
        let presence_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let n = node_presence.lock().await;
                let _ = n.broadcast_presence().await;
            }
        });

        // Main input loop
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        print!("> ");
        io::stdout().flush()?;

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let line = line.trim();
                    
                    if line.is_empty() {
                        print!("> ");
                        io::stdout().flush()?;
                        continue;
                    }

                    if line.starts_with('/') {
                        let parts: Vec<&str> = line.splitn(2, ' ').collect();
                        let cmd = parts[0];
                        let arg = parts.get(1).map(|s| *s).unwrap_or("");

                        match cmd {
                            "/quit" | "/exit" | "/q" => {
                                break;
                            }
                            "/msg" | "/m" => {
                                if !arg.is_empty() {
                                    let mut n = node.lock().await;
                                    n.send_message(arg).await?;
                                    println!("📤 You: {}", arg);
                                } else {
                                    println!("Usage: /msg <text>");
                                }
                            }
                            "/voice" | "/v" => {
                                let mut n = node.lock().await;
                                n.toggle_voice().await?;
                                if n.voice_active {
                                    println!("🎤 Voice activated");
                                } else {
                                    println!("🔇 Voice deactivated");
                                }
                            }
                            "/presence" | "/p" => {
                                let n = node.lock().await;
                                println!("📊 Presence Status:");
                                println!("   Liveness: {:.0}%", n.presence.liveness * 100.0);
                                println!("   Immediacy: {:.0}%", n.presence.immediacy * 100.0);
                                println!("   Coherence: {:.0}%", n.presence.coherence * 100.0);
                                println!("   Relational: {:.0}%", n.presence.relational_continuity * 100.0);
                                println!("   Emotional: {:.0}%", n.presence.emotional_bandwidth * 100.0);
                                println!("   Overall: {:.0}%", n.presence.score() * 100.0);
                            }
                            "/degrade" | "/d" => {
                                let mut n = node.lock().await;
                                n.simulate_degradation();
                                println!("⚠️  Degradation: {:?}", n.degradation);
                                println!("   Presence now: {:.0}%", n.presence.score() * 100.0);
                                self.show_degradation_visual(&n);
                            }
                            "/recover" | "/r" => {
                                let mut n = node.lock().await;
                                n.recover();
                                println!("✅ Recovered to full perception");
                                println!("   Presence: {:.0}%", n.presence.score() * 100.0);
                            }
                            "/status" | "/s" => {
                                let n = node.lock().await;
                                println!("{}", n.status());
                            }
                            "/peers" => {
                                let n = node.lock().await;
                                println!("Connected peers:");
                                println!("{}", n.peers_list());
                            }
                            "/help" | "/h" => {
                                self.show_help();
                            }
                            _ => {
                                println!("Unknown command: {}", cmd);
                                println!("Type /help for available commands");
                            }
                        }
                    } else {
                        // Treat as message
                        let mut n = node.lock().await;
                        n.send_message(line).await?;
                        println!("📤 You: {}", line);
                    }

                    print!("> ");
                    io::stdout().flush()?;
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }

        // Cleanup
        recv_handle.abort();
        events_handle.abort();
        presence_handle.abort();

        Ok(())
    }

    fn show_help(&self) {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║  Available Commands:                                       ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║  /msg <text>  - Send a message (or just type text)        ║");
        println!("║  /voice       - Toggle voice (simulated)                  ║");
        println!("║  /presence    - Show detailed presence info               ║");
        println!("║  /degrade     - Simulate network degradation              ║");
        println!("║  /recover     - Recover from degradation                  ║");
        println!("║  /status      - Show node status                          ║");
        println!("║  /peers       - List connected peers                      ║");
        println!("║  /help        - Show this help                            ║");
        println!("║  /quit        - Exit                                      ║");
        println!("╚════════════════════════════════════════════════════════════╝");
    }

    fn show_degradation_visual(&self, node: &DemoNode) {
        println!();
        println!("┌─────────────────────────────────────────────────────────┐");
        println!("│              Degradation Ladder Visualization           │");
        println!("├─────────────────────────────────────────────────────────┤");
        
        let levels = [
            ("L0: Full Perception", "████████████████████", elara_core::DegradationLevel::L0_FullPerception),
            ("L1: Distorted", "████████████████░░░░", elara_core::DegradationLevel::L1_DistortedPerception),
            ("L2: Fragmented", "████████████░░░░░░░░", elara_core::DegradationLevel::L2_FragmentedPerception),
            ("L3: Symbolic", "████████░░░░░░░░░░░░", elara_core::DegradationLevel::L3_SymbolicPresence),
            ("L4: Minimal", "████░░░░░░░░░░░░░░░░", elara_core::DegradationLevel::L4_MinimalPresence),
            ("L5: Latent", "██░░░░░░░░░░░░░░░░░░", elara_core::DegradationLevel::L5_LatentPresence),
        ];

        for (name, bar, level) in levels {
            let marker = if level == node.degradation { "→" } else { " " };
            println!("│ {} {:20} [{}] │", marker, name, bar);
        }
        
        println!("└─────────────────────────────────────────────────────────┘");
        println!();
        
        // Show what's available at current level
        match node.degradation {
            elara_core::DegradationLevel::L0_FullPerception => {
                println!("Available: Text ✓ | Voice ✓ | Presence ✓ | Typing ✓");
            }
            elara_core::DegradationLevel::L1_DistortedPerception => {
                println!("Available: Text ✓ | Voice ~ | Presence ✓ | Typing ✓");
                println!("Note: Voice quality reduced");
            }
            elara_core::DegradationLevel::L2_FragmentedPerception => {
                println!("Available: Text ✓ | Voice ~ | Presence ~ | Typing ✓");
                println!("Note: Voice parameters only, presence delayed");
            }
            elara_core::DegradationLevel::L3_SymbolicPresence => {
                println!("Available: Text ✓ | Voice ✗ | Presence ~ | Typing ~");
                println!("Note: Voice reduced to speaking indicator");
            }
            elara_core::DegradationLevel::L4_MinimalPresence => {
                println!("Available: Text ~ | Voice ✗ | Presence ~ | Typing ✗");
                println!("Note: Only essential messages, presence heartbeat");
            }
            elara_core::DegradationLevel::L5_LatentPresence => {
                println!("Available: Text ✗ | Voice ✗ | Presence ~ | Typing ✗");
                println!("Note: Identity alive, waiting for recovery");
            }
        }
    }
}
