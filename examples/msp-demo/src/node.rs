//! Demo Node - Core node implementation for MSP demo

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};

use elara_core::{
    DegradationLevel, MessageId, NodeId, PresenceVector, SessionId, StateTime,
};
use elara_crypto::Identity;
use elara_msp::text::{TextMessage, TextStream, TypingIndicator};
use elara_msp::voice::{VoiceFrame, VoiceState};

/// Peer information
#[derive(Debug, Clone)]
pub struct Peer {
    pub node_id: NodeId,
    pub name: String,
    pub addr: SocketAddr,
    pub presence: PresenceVector,
    pub degradation: DegradationLevel,
    pub last_seen: Instant,
    pub typing: bool,
    pub voice_active: bool,
}

/// Message types for internal communication
#[derive(Debug, Clone)]
pub enum DemoMessage {
    Text { from: String, content: String },
    Presence { from: String, presence: PresenceVector },
    Typing { from: String, typing: bool },
    Voice { from: String, energy: u8 },
    PeerJoined { name: String },
    PeerLeft { name: String },
    Degradation { level: DegradationLevel },
}

/// Demo node state
pub struct DemoNode {
    /// Local identity
    pub identity: Identity,
    /// User name
    pub name: String,
    /// Local port
    pub port: u16,
    /// UDP socket
    socket: Arc<UdpSocket>,
    /// Connected peers
    pub peers: HashMap<SocketAddr, Peer>,
    /// Text stream
    pub text_stream: TextStream,
    /// Local presence
    pub presence: PresenceVector,
    /// Current degradation level
    pub degradation: DegradationLevel,
    /// Voice state (simulated)
    pub voice_active: bool,
    pub voice_energy: u8,
    /// Typing state
    pub typing: bool,
    /// Message sequence
    msg_seq: u64,
    /// Session start time
    start_time: Instant,
    /// Event receiver
    pub event_rx: mpsc::Receiver<DemoMessage>,
    /// Event sender (for internal use)
    event_tx: mpsc::Sender<DemoMessage>,
}

impl DemoNode {
    /// Create a new demo node
    pub async fn new(name: String, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let identity = Identity::generate();
        let addr = format!("0.0.0.0:{}", port);
        let socket = Arc::new(UdpSocket::bind(&addr).await?);
        
        let (event_tx, event_rx) = mpsc::channel(100);

        let node = Self {
            identity,
            name,
            port,
            socket,
            peers: HashMap::new(),
            text_stream: TextStream::new(100),
            presence: PresenceVector::full(),
            degradation: DegradationLevel::L0_FullPerception,
            voice_active: false,
            voice_energy: 0,
            typing: false,
            msg_seq: 0,
            start_time: Instant::now(),
            event_rx,
            event_tx,
        };

        Ok(node)
    }

    /// Get node ID
    pub fn node_id(&self) -> NodeId {
        self.identity.node_id()
    }

    /// Get current state time
    pub fn state_time(&self) -> StateTime {
        StateTime::from_millis(self.start_time.elapsed().as_millis() as i64)
    }

    /// Connect to a peer
    pub async fn connect_peer(&mut self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        // Send hello message
        let hello = format!("HELLO:{}:{:016x}", self.name, self.node_id().0);
        println!("[DEBUG] Sending HELLO to {}: {}", addr, hello);
        let bytes_sent = self.socket.send_to(hello.as_bytes(), addr).await?;
        println!("[DEBUG] Sent {} bytes to {}", bytes_sent, addr);
        
        Ok(())
    }

    /// Send a text message
    pub async fn send_message(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.msg_seq += 1;
        let msg_id = MessageId(self.msg_seq);
        
        let message = TextMessage::new(
            msg_id,
            self.node_id(),
            content.as_bytes().to_vec(),
            self.state_time(),
        );
        
        // Add to local stream
        self.text_stream.append(message);
        
        // Broadcast to peers
        let packet = format!("MSG:{}:{}", self.name, content);
        for (addr, _) in &self.peers {
            let _ = self.socket.send_to(packet.as_bytes(), addr).await;
        }
        
        // Clear typing
        self.typing = false;
        self.broadcast_typing().await?;
        
        Ok(())
    }

    /// Set typing state
    pub async fn set_typing(&mut self, typing: bool) -> Result<(), Box<dyn std::error::Error>> {
        if self.typing != typing {
            self.typing = typing;
            self.broadcast_typing().await?;
        }
        Ok(())
    }

    /// Broadcast typing state
    async fn broadcast_typing(&self) -> Result<(), Box<dyn std::error::Error>> {
        let packet = format!("TYPING:{}:{}", self.name, if self.typing { "1" } else { "0" });
        for (addr, _) in &self.peers {
            let _ = self.socket.send_to(packet.as_bytes(), addr).await;
        }
        Ok(())
    }

    /// Toggle voice
    pub async fn toggle_voice(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.voice_active = !self.voice_active;
        self.voice_energy = if self.voice_active { 200 } else { 0 };
        
        let packet = format!("VOICE:{}:{}", self.name, self.voice_energy);
        for (addr, _) in &self.peers {
            let _ = self.socket.send_to(packet.as_bytes(), addr).await;
        }
        
        Ok(())
    }

    /// Simulate network degradation
    pub fn simulate_degradation(&mut self) {
        self.degradation = match self.degradation {
            DegradationLevel::L0_FullPerception => DegradationLevel::L1_DistortedPerception,
            DegradationLevel::L1_DistortedPerception => DegradationLevel::L2_FragmentedPerception,
            DegradationLevel::L2_FragmentedPerception => DegradationLevel::L3_SymbolicPresence,
            DegradationLevel::L3_SymbolicPresence => DegradationLevel::L4_MinimalPresence,
            DegradationLevel::L4_MinimalPresence => DegradationLevel::L5_LatentPresence,
            DegradationLevel::L5_LatentPresence => DegradationLevel::L5_LatentPresence,
        };
        
        // Update presence based on degradation
        self.presence = match self.degradation {
            DegradationLevel::L0_FullPerception => PresenceVector::full(),
            DegradationLevel::L1_DistortedPerception => PresenceVector {
                liveness: 0.9,
                immediacy: 0.8,
                coherence: 0.7,
                relational_continuity: 0.8,
                emotional_bandwidth: 0.6,
            },
            DegradationLevel::L2_FragmentedPerception => PresenceVector {
                liveness: 0.7,
                immediacy: 0.5,
                coherence: 0.4,
                relational_continuity: 0.6,
                emotional_bandwidth: 0.3,
            },
            DegradationLevel::L3_SymbolicPresence => PresenceVector {
                liveness: 0.5,
                immediacy: 0.3,
                coherence: 0.2,
                relational_continuity: 0.4,
                emotional_bandwidth: 0.1,
            },
            DegradationLevel::L4_MinimalPresence => PresenceVector {
                liveness: 0.3,
                immediacy: 0.1,
                coherence: 0.1,
                relational_continuity: 0.2,
                emotional_bandwidth: 0.0,
            },
            DegradationLevel::L5_LatentPresence => PresenceVector {
                liveness: 0.1,
                immediacy: 0.0,
                coherence: 0.0,
                relational_continuity: 0.1,
                emotional_bandwidth: 0.0,
            },
        };
    }

    /// Recover from degradation
    pub fn recover(&mut self) {
        self.degradation = DegradationLevel::L0_FullPerception;
        self.presence = PresenceVector::full();
    }

    /// Process incoming packet
    pub async fn process_packet(&mut self, data: &[u8], from: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let text = String::from_utf8_lossy(data);
        let parts: Vec<&str> = text.splitn(3, ':').collect();
        
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "HELLO" if parts.len() >= 3 => {
                let name = parts[1].to_string();
                let node_id_str = parts[2];
                let node_id = u64::from_str_radix(node_id_str, 16).unwrap_or(0);
                
                println!("[DEBUG] Received HELLO from {} - name: {}, node_id: {}", from, name, node_id_str);
                
                // Check if already known peer
                let is_new = !self.peers.contains_key(&from);
                println!("[DEBUG] is_new peer: {}", is_new);
                
                // Add or update peer
                let peer = Peer {
                    node_id: NodeId::new(node_id),
                    name: name.clone(),
                    addr: from,
                    presence: PresenceVector::full(),
                    degradation: DegradationLevel::L0_FullPerception,
                    last_seen: Instant::now(),
                    typing: false,
                    voice_active: false,
                };
                self.peers.insert(from, peer);
                println!("[DEBUG] Added peer, total peers: {}", self.peers.len());
                
                // Send hello back only if this is a new peer (to avoid loop)
                if is_new {
                    let hello = format!("HELLO:{}:{:016x}", self.name, self.node_id().0);
                    println!("[DEBUG] Sending HELLO back to {}", from);
                    self.socket.send_to(hello.as_bytes(), from).await?;
                    
                    // Notify UI
                    println!("[DEBUG] Notifying UI about peer joined: {}", name);
                    let _ = self.event_tx.send(DemoMessage::PeerJoined { name }).await;
                }
            }
            "MSG" if parts.len() >= 3 => {
                let from_name = parts[1].to_string();
                let content = parts[2].to_string();
                
                // Update last seen
                if let Some(peer) = self.peers.get_mut(&from) {
                    peer.last_seen = Instant::now();
                    peer.typing = false;
                }
                
                // Notify UI
                let _ = self.event_tx.send(DemoMessage::Text { 
                    from: from_name, 
                    content 
                }).await;
            }
            "TYPING" if parts.len() >= 3 => {
                let from_name = parts[1].to_string();
                let typing = parts[2] == "1";
                
                if let Some(peer) = self.peers.get_mut(&from) {
                    peer.typing = typing;
                    peer.last_seen = Instant::now();
                }
                
                let _ = self.event_tx.send(DemoMessage::Typing { 
                    from: from_name, 
                    typing 
                }).await;
            }
            "VOICE" if parts.len() >= 3 => {
                let from_name = parts[1].to_string();
                let energy: u8 = parts[2].parse().unwrap_or(0);
                
                if let Some(peer) = self.peers.get_mut(&from) {
                    peer.voice_active = energy > 0;
                    peer.last_seen = Instant::now();
                }
                
                let _ = self.event_tx.send(DemoMessage::Voice { 
                    from: from_name, 
                    energy 
                }).await;
            }
            "PRESENCE" if parts.len() >= 3 => {
                let from_name = parts[1].to_string();
                // Parse presence (simplified)
                let score: f32 = parts[2].parse().unwrap_or(1.0);
                let presence = PresenceVector {
                    liveness: score,
                    immediacy: score,
                    coherence: score,
                    relational_continuity: score,
                    emotional_bandwidth: score,
                };
                
                if let Some(peer) = self.peers.get_mut(&from) {
                    peer.presence = presence.clone();
                    peer.last_seen = Instant::now();
                }
                
                let _ = self.event_tx.send(DemoMessage::Presence { 
                    from: from_name, 
                    presence 
                }).await;
            }
            _ => {}
        }
        
        Ok(())
    }

    /// Get socket for receiving
    pub fn socket(&self) -> Arc<UdpSocket> {
        self.socket.clone()
    }

    /// Broadcast presence
    pub async fn broadcast_presence(&self) -> Result<(), Box<dyn std::error::Error>> {
        let score = self.presence.score();
        let packet = format!("PRESENCE:{}:{:.2}", self.name, score);
        for (addr, _) in &self.peers {
            let _ = self.socket.send_to(packet.as_bytes(), addr).await;
        }
        Ok(())
    }

    /// Get status string
    pub fn status(&self) -> String {
        format!(
            "Node: {} ({:016x})\n\
             Port: {}\n\
             Peers: {}\n\
             Degradation: {:?}\n\
             Presence: {:.2}\n\
             Voice: {}\n\
             Messages: {}",
            self.name,
            self.node_id().0,
            self.port,
            self.peers.len(),
            self.degradation,
            self.presence.score(),
            if self.voice_active { "Active" } else { "Inactive" },
            self.text_stream.messages.len(),
        )
    }

    /// Get peers list
    pub fn peers_list(&self) -> String {
        if self.peers.is_empty() {
            return "No peers connected".to_string();
        }
        
        let mut result = String::new();
        for (addr, peer) in &self.peers {
            result.push_str(&format!(
                "  {} ({}) - {:?} {}{}\n",
                peer.name,
                addr,
                peer.degradation,
                if peer.typing { "[typing] " } else { "" },
                if peer.voice_active { "[speaking]" } else { "" },
            ));
        }
        result
    }
}
