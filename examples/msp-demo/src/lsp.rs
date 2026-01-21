//! LSP Demo - Livestream State Profile (Livestream Simulation)
//!
//! This demonstrates ELARA-native livestreaming:
//! - Asymmetric authority (broadcaster -> viewers)
//! - State diffusion through swarm
//! - Interest-based propagation
//! - Graceful degradation for viewers
//!
//! NOTE: This is a simulation - no actual video capture.

use std::collections::HashMap;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};

use elara_core::{DegradationLevel, NodeId, StateTime};
use elara_visual::{
    EmotionVector, FaceState, GazeState, MouthState,
    VisualEncoder, VisualPredictor, VisualState,
    PredictionConfig, Viseme,
};
use elara_diffusion::{
    LivestreamSwarm, SwarmConfig, SwarmState,
    InterestLevel,
};

/// Message types for LSP
#[derive(Debug, Clone)]
enum LspMessage {
    /// Broadcaster announcement
    StreamStart { title: String, broadcaster: String },
    /// Stream ended
    StreamEnd,
    /// Viewer join
    ViewerJoin { name: String },
    /// Viewer leave
    ViewerLeave,
    /// Visual state update (from broadcaster)
    VisualUpdate { data: Vec<u8>, sequence: u64 },
    /// Chat message (from anyone)
    Chat { sender: String, message: String },
    /// Viewer count update
    ViewerCount { count: u32 },
    /// Request keyframe (from viewer)
    RequestKeyframe,
}

impl LspMessage {
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            LspMessage::StreamStart { title, broadcaster } => {
                buf.push(0x01);
                buf.push(title.len() as u8);
                buf.extend(title.as_bytes());
                buf.push(broadcaster.len() as u8);
                buf.extend(broadcaster.as_bytes());
            }
            LspMessage::StreamEnd => {
                buf.push(0x02);
            }
            LspMessage::ViewerJoin { name } => {
                buf.push(0x03);
                buf.extend(name.as_bytes());
            }
            LspMessage::ViewerLeave => {
                buf.push(0x04);
            }
            LspMessage::VisualUpdate { data, sequence } => {
                buf.push(0x05);
                buf.extend(&sequence.to_le_bytes());
                buf.extend(data);
            }
            LspMessage::Chat { sender, message } => {
                buf.push(0x06);
                buf.push(sender.len() as u8);
                buf.extend(sender.as_bytes());
                buf.extend(message.as_bytes());
            }
            LspMessage::ViewerCount { count } => {
                buf.push(0x07);
                buf.extend(&count.to_le_bytes());
            }
            LspMessage::RequestKeyframe => {
                buf.push(0x08);
            }
        }
        buf
    }
    
    fn decode(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        
        match data[0] {
            0x01 => {
                let title_len = data.get(1).copied()? as usize;
                let title = String::from_utf8_lossy(&data[2..2+title_len]).to_string();
                let broadcaster_len = data.get(2+title_len).copied()? as usize;
                let broadcaster = String::from_utf8_lossy(&data[3+title_len..3+title_len+broadcaster_len]).to_string();
                Some(LspMessage::StreamStart { title, broadcaster })
            }
            0x02 => Some(LspMessage::StreamEnd),
            0x03 => {
                let name = String::from_utf8_lossy(&data[1..]).to_string();
                Some(LspMessage::ViewerJoin { name })
            }
            0x04 => Some(LspMessage::ViewerLeave),
            0x05 => {
                let sequence = u64::from_le_bytes(data[1..9].try_into().ok()?);
                Some(LspMessage::VisualUpdate { data: data[9..].to_vec(), sequence })
            }
            0x06 => {
                let sender_len = data.get(1).copied()? as usize;
                let sender = String::from_utf8_lossy(&data[2..2+sender_len]).to_string();
                let message = String::from_utf8_lossy(&data[2+sender_len..]).to_string();
                Some(LspMessage::Chat { sender, message })
            }
            0x07 => {
                let count = u32::from_le_bytes(data[1..5].try_into().ok()?);
                Some(LspMessage::ViewerCount { count })
            }
            0x08 => Some(LspMessage::RequestKeyframe),
            _ => None,
        }
    }
}

/// Broadcaster state
struct Broadcaster {
    name: String,
    title: String,
    node_id: NodeId,
    socket: Arc<UdpSocket>,
    swarm: LivestreamSwarm,
    viewers: HashMap<SocketAddr, ViewerInfo>,
    face_phase: f32,
    sequence: u64,
    start_time: Instant,
    is_speaking: bool,
    chat_messages: Vec<(String, String)>,
}

struct ViewerInfo {
    name: String,
    interest: InterestLevel,
    joined_at: Instant,
}

impl Broadcaster {
    async fn new(name: String, title: String, port: u16) -> io::Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&addr).await?;
        let node_id = NodeId::new(rand::random());
        
        let config = SwarmConfig {
            star_to_tree_threshold: 50,
            tree_fanout: 5,
            bandwidth_per_viewer: 500_000,
            keyframe_interval_ms: 2000,
        };
        
        let swarm = LivestreamSwarm::new(1000, node_id, config);
        
        Ok(Self {
            name,
            title,
            node_id,
            socket: Arc::new(socket),
            swarm,
            viewers: HashMap::new(),
            face_phase: 0.0,
            sequence: 0,
            start_time: Instant::now(),
            is_speaking: false,
            chat_messages: Vec::new(),
        })
    }
    
    fn current_time(&self) -> StateTime {
        StateTime::from_millis(self.start_time.elapsed().as_millis() as i64)
    }
    
    fn generate_visual_state(&mut self) -> VisualState {
        self.sequence += 1;
        let timestamp = self.current_time();
        
        let mut face = FaceState::new(timestamp);
        face.present = true;
        face.speaking = self.is_speaking;
        face.confidence = 0.95;
        
        // Simulate expressions
        face.emotion = EmotionVector {
            joy: 0.4 + 0.1 * (self.face_phase * 0.3).sin(),
            sadness: 0.0,
            anger: 0.0,
            fear: 0.0,
            surprise: 0.1 * (self.face_phase * 0.5).sin().abs(),
            disgust: 0.0,
            contempt: 0.0,
        };
        
        face.gaze = GazeState {
            yaw: 0.1 * (self.face_phase * 0.2).sin(),
            pitch: 0.05 * (self.face_phase * 0.3).cos(),
            looking_at_camera: true,
            blink: if (self.face_phase % 4.0) < 0.15 { 1.0 } else { 0.0 },
        };
        
        if self.is_speaking {
            face.mouth = MouthState {
                openness: 0.3 + 0.2 * (self.face_phase * 8.0).sin().abs(),
                smile: 0.2,
                viseme: Viseme::Neutral,
            };
        }
        
        face.head_rotation = (
            0.05 * (self.face_phase * 0.4).sin(),
            0.03 * (self.face_phase * 0.5).cos(),
            0.0,
        );
        
        let is_keyframe = self.swarm.needs_keyframe(timestamp);
        
        if is_keyframe {
            VisualState::keyframe(self.node_id, timestamp, self.sequence)
                .with_face(face)
        } else {
            VisualState::delta(self.node_id, timestamp, self.sequence, 
                elara_visual::VisualStateId::new(self.sequence - 1))
                .with_face(face)
        }
    }
    
    async fn broadcast_visual(&mut self) -> io::Result<()> {
        let state = self.generate_visual_state();
        let data = VisualEncoder::encode(&state);
        
        let msg = LspMessage::VisualUpdate { 
            data, 
            sequence: self.sequence 
        };
        let encoded = msg.encode();
        
        for addr in self.viewers.keys() {
            self.socket.send_to(&encoded, addr).await?;
        }
        
        Ok(())
    }
    
    async fn broadcast_viewer_count(&self) -> io::Result<()> {
        let msg = LspMessage::ViewerCount { count: self.viewers.len() as u32 };
        let encoded = msg.encode();
        
        for addr in self.viewers.keys() {
            self.socket.send_to(&encoded, addr).await?;
        }
        
        Ok(())
    }
    
    async fn handle_message(&mut self, data: &[u8], from: SocketAddr) -> io::Result<()> {
        let msg = match LspMessage::decode(data) {
            Some(m) => m,
            None => return Ok(()),
        };
        
        match msg {
            LspMessage::ViewerJoin { name } => {
                println!("\n👁️ {} joined the stream", name);
                
                self.viewers.insert(from, ViewerInfo {
                    name: name.clone(),
                    interest: InterestLevel::High,
                    joined_at: Instant::now(),
                });
                
                self.swarm.add_viewer(NodeId::new(rand::random()));
                
                // Send stream info
                let start_msg = LspMessage::StreamStart {
                    title: self.title.clone(),
                    broadcaster: self.name.clone(),
                };
                self.socket.send_to(&start_msg.encode(), from).await?;
                
                // Send viewer count
                self.broadcast_viewer_count().await?;
            }
            
            LspMessage::ViewerLeave => {
                if let Some(viewer) = self.viewers.remove(&from) {
                    println!("\n👋 {} left the stream", viewer.name);
                    self.broadcast_viewer_count().await?;
                }
            }
            
            LspMessage::Chat { sender, message } => {
                println!("\n💬 {}: {}", sender, message);
                self.chat_messages.push((sender.clone(), message.clone()));
                if self.chat_messages.len() > 10 {
                    self.chat_messages.remove(0);
                }
                
                // Relay to all viewers
                let relay = LspMessage::Chat { sender, message };
                let encoded = relay.encode();
                for addr in self.viewers.keys() {
                    self.socket.send_to(&encoded, addr).await?;
                }
            }
            
            LspMessage::RequestKeyframe => {
                // Force send a keyframe
                let state = VisualState::keyframe(self.node_id, self.current_time(), self.sequence);
                let data = VisualEncoder::encode(&state);
                let msg = LspMessage::VisualUpdate { data, sequence: self.sequence };
                self.socket.send_to(&msg.encode(), from).await?;
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    fn render_ui(&self) {
        print!("\x1B[2J\x1B[H");
        
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║         ELARA LSP Demo - Livestream (BROADCASTER)            ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 📺 Stream: {}", self.title);
        println!("║ 🎤 {}: {}", self.name, if self.is_speaking { "Speaking" } else { "Silent" });
        println!("║ 👁️ Viewers: {} | State: {:?}", self.viewers.len(), self.swarm.state);
        println!("║ 📊 Sequence: {} | Uptime: {}s", self.sequence, self.start_time.elapsed().as_secs());
        println!("╠══════════════════════════════════════════════════════════════╣");
        
        println!("║ Recent Chat:");
        for (sender, msg) in self.chat_messages.iter().rev().take(5) {
            println!("║   {}: {}", sender, msg);
        }
        
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Commands: [s]peak toggle, [e]nd stream, [q]uit               ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        
        io::stdout().flush().ok();
    }
}

/// Viewer state
struct Viewer {
    name: String,
    socket: Arc<UdpSocket>,
    broadcaster_addr: SocketAddr,
    stream_title: String,
    broadcaster_name: String,
    predictor: VisualPredictor,
    last_sequence: u64,
    last_update: Instant,
    viewer_count: u32,
    chat_messages: Vec<(String, String)>,
    start_time: Instant,
    degradation: DegradationLevel,
}

impl Viewer {
    async fn new(name: String, port: u16, broadcaster_addr: SocketAddr) -> io::Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&addr).await?;
        
        Ok(Self {
            name,
            socket: Arc::new(socket),
            broadcaster_addr,
            stream_title: String::new(),
            broadcaster_name: String::new(),
            predictor: VisualPredictor::new(PredictionConfig::default()),
            last_sequence: 0,
            last_update: Instant::now(),
            viewer_count: 0,
            chat_messages: Vec::new(),
            start_time: Instant::now(),
            degradation: DegradationLevel::L0_FullPerception,
        })
    }
    
    fn current_time(&self) -> StateTime {
        StateTime::from_millis(self.start_time.elapsed().as_millis() as i64)
    }
    
    async fn join_stream(&self) -> io::Result<()> {
        let msg = LspMessage::ViewerJoin { name: self.name.clone() };
        self.socket.send_to(&msg.encode(), self.broadcaster_addr).await?;
        Ok(())
    }
    
    async fn leave_stream(&self) -> io::Result<()> {
        let msg = LspMessage::ViewerLeave;
        self.socket.send_to(&msg.encode(), self.broadcaster_addr).await?;
        Ok(())
    }
    
    async fn send_chat(&self, message: String) -> io::Result<()> {
        let msg = LspMessage::Chat { 
            sender: self.name.clone(), 
            message 
        };
        self.socket.send_to(&msg.encode(), self.broadcaster_addr).await?;
        Ok(())
    }
    
    async fn handle_message(&mut self, data: &[u8]) -> io::Result<()> {
        let msg = match LspMessage::decode(data) {
            Some(m) => m,
            None => return Ok(()),
        };
        
        match msg {
            LspMessage::StreamStart { title, broadcaster } => {
                self.stream_title = title;
                self.broadcaster_name = broadcaster;
            }
            
            LspMessage::StreamEnd => {
                println!("\n📺 Stream ended!");
            }
            
            LspMessage::VisualUpdate { data, sequence } => {
                if let Ok(mut state) = VisualEncoder::decode(&data) {
                    // Apply local degradation
                    state.degrade(self.degradation);
                    self.predictor.update(state);
                    self.last_sequence = sequence;
                    self.last_update = Instant::now();
                }
            }
            
            LspMessage::Chat { sender, message } => {
                self.chat_messages.push((sender, message));
                if self.chat_messages.len() > 10 {
                    self.chat_messages.remove(0);
                }
            }
            
            LspMessage::ViewerCount { count } => {
                self.viewer_count = count;
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    fn render_ui(&mut self) {
        print!("\x1B[2J\x1B[H");
        
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           ELARA LSP Demo - Livestream (VIEWER)               ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 📺 {}", self.stream_title);
        println!("║ 🎤 Broadcaster: {}", self.broadcaster_name);
        println!("║ 👁️ Viewers: {} | You: {}", self.viewer_count, self.name);
        println!("║ 📊 Degradation: {:?}", self.degradation);
        println!("╠══════════════════════════════════════════════════════════════╣");
        
        // Visual state display
        let time_since = self.last_update.elapsed().as_millis();
        let status = if time_since < 100 {
            "🟢 Live"
        } else if time_since < 500 {
            "🟡 Predicting"
        } else {
            "🔴 Stale"
        };
        
        let current_time = self.current_time();
        if let Some(state) = self.predictor.predict(current_time) {
            if let Some(face) = &state.face {
                let (emotion_name, _) = face.emotion.dominant();
                let speaking = if face.speaking { "🗣️ Speaking" } else { "😶 Silent" };
                println!("║ {} Broadcaster: {} {}", status, speaking, emotion_name);
                println!("║   Confidence: {:.0}% | Predictions: {}", 
                    face.confidence * 100.0,
                    self.predictor.prediction_count()
                );
            }
        } else {
            println!("║ {} Waiting for stream data...", status);
        }
        
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Chat:");
        for (sender, msg) in self.chat_messages.iter().rev().take(5) {
            println!("║   {}: {}", sender, msg);
        }
        
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Commands: [c]hat, [d]egrade, [u]pgrade, [q]uit               ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        
        io::stdout().flush().ok();
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ELARA LSP Demo - Livestream Simulation             ║");
    println!("║                                                              ║");
    println!("║  This demonstrates ELARA-native livestreaming:               ║");
    println!("║  - Asymmetric authority (broadcaster -> viewers)             ║");
    println!("║  - Visual state diffusion                                    ║");
    println!("║  - Prediction under packet loss                              ║");
    println!("║  - Graceful degradation                                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    
    print!("Mode: [b]roadcaster or [v]iewer? ");
    io::stdout().flush()?;
    let mut mode = String::new();
    io::stdin().read_line(&mut mode)?;
    let is_broadcaster = mode.trim().to_lowercase().starts_with('b');
    
    print!("Your name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();
    
    print!("Port: ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port: u16 = port_str.trim().parse().unwrap_or(9000);
    
    if is_broadcaster {
        print!("Stream title: ");
        io::stdout().flush()?;
        let mut title = String::new();
        io::stdin().read_line(&mut title)?;
        let title = title.trim().to_string();
        
        run_broadcaster(name, title, port).await
    } else {
        print!("Broadcaster address: ");
        io::stdout().flush()?;
        let mut addr_str = String::new();
        io::stdin().read_line(&mut addr_str)?;
        let broadcaster_addr: SocketAddr = addr_str.trim().parse()
            .expect("Invalid address");
        
        run_viewer(name, port, broadcaster_addr).await
    }
}

async fn run_broadcaster(name: String, title: String, port: u16) -> io::Result<()> {
    let mut broadcaster = Broadcaster::new(name, title, port).await?;
    broadcaster.swarm.start();
    
    println!("\n✅ Broadcasting on port {}. Waiting for viewers...", port);
    
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<char>(10);
    
    let cmd_tx_clone = cmd_tx.clone();
    std::thread::spawn(move || {
        loop {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                if let Some(c) = input.chars().next() {
                    let _ = cmd_tx_clone.blocking_send(c);
                }
            }
        }
    });
    
    let socket = broadcaster.socket.clone();
    let mut recv_buf = [0u8; 4096];
    let mut broadcast_interval = interval(Duration::from_millis(33));
    let mut ui_interval = interval(Duration::from_millis(100));
    
    loop {
        tokio::select! {
            result = socket.recv_from(&mut recv_buf) => {
                if let Ok((len, from)) = result {
                    broadcaster.handle_message(&recv_buf[..len], from).await?;
                }
            }
            
            _ = broadcast_interval.tick() => {
                broadcaster.face_phase += 0.033;
                if !broadcaster.viewers.is_empty() {
                    broadcaster.broadcast_visual().await?;
                }
            }
            
            _ = ui_interval.tick() => {
                broadcaster.render_ui();
            }
            
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    's' | 'S' => {
                        broadcaster.is_speaking = !broadcaster.is_speaking;
                    }
                    'e' | 'E' => {
                        let end_msg = LspMessage::StreamEnd;
                        let encoded = end_msg.encode();
                        for addr in broadcaster.viewers.keys() {
                            socket.send_to(&encoded, addr).await?;
                        }
                        broadcaster.swarm.end();
                        println!("\n📺 Stream ended.");
                    }
                    'q' | 'Q' => {
                        let end_msg = LspMessage::StreamEnd;
                        let encoded = end_msg.encode();
                        for addr in broadcaster.viewers.keys() {
                            socket.send_to(&encoded, addr).await?;
                        }
                        println!("\n👋 Goodbye!");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    
    Ok(())
}

async fn run_viewer(name: String, port: u16, broadcaster_addr: SocketAddr) -> io::Result<()> {
    let mut viewer = Viewer::new(name, port, broadcaster_addr).await?;
    
    println!("\n📺 Joining stream at {}...", broadcaster_addr);
    viewer.join_stream().await?;
    
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(10);
    
    let cmd_tx_clone = cmd_tx.clone();
    std::thread::spawn(move || {
        loop {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let _ = cmd_tx_clone.blocking_send(input.trim().to_string());
            }
        }
    });
    
    let socket = viewer.socket.clone();
    let mut recv_buf = [0u8; 4096];
    let mut ui_interval = interval(Duration::from_millis(100));
    
    loop {
        tokio::select! {
            result = socket.recv_from(&mut recv_buf) => {
                if let Ok((len, _)) = result {
                    viewer.handle_message(&recv_buf[..len]).await?;
                }
            }
            
            _ = ui_interval.tick() => {
                viewer.render_ui();
            }
            
            Some(cmd) = cmd_rx.recv() => {
                if cmd.len() == 1 {
                    match cmd.chars().next().unwrap() {
                        'c' | 'C' => {
                            print!("\nChat message: ");
                            io::stdout().flush()?;
                        }
                        'd' | 'D' => {
                            viewer.degradation = viewer.degradation.degrade().unwrap_or(DegradationLevel::L5_LatentPresence);
                        }
                        'u' | 'U' => {
                            viewer.degradation = match viewer.degradation {
                                DegradationLevel::L5_LatentPresence => DegradationLevel::L4_MinimalPresence,
                                DegradationLevel::L4_MinimalPresence => DegradationLevel::L3_SymbolicPresence,
                                DegradationLevel::L3_SymbolicPresence => DegradationLevel::L2_FragmentedPerception,
                                DegradationLevel::L2_FragmentedPerception => DegradationLevel::L1_DistortedPerception,
                                _ => DegradationLevel::L0_FullPerception,
                            };
                        }
                        'q' | 'Q' => {
                            viewer.leave_stream().await?;
                            println!("\n👋 Left stream.");
                            break;
                        }
                        _ => {}
                    }
                } else if !cmd.is_empty() {
                    // Treat as chat message
                    viewer.send_chat(cmd).await?;
                }
            }
        }
    }
    
    Ok(())
}
