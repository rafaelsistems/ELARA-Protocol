//! VRP Demo - Visual Reality Profile (Video Call Simulation)
//!
//! This demonstrates ELARA-native video calling:
//! - Video as Visual State (not pixel stream)
//! - Face/Pose/Scene state synchronization
//! - Prediction under packet loss
//! - Graceful degradation (L0-L5)
//!
//! NOTE: This is a simulation - no actual camera capture.
//! Real implementation would use ML models for face/pose extraction.

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
    EmotionVector, FaceState, GazeState, MouthState, PredictionConfig, Viseme, VisualEncoder,
    VisualPredictor, VisualState, VisualStateBuffer,
};

/// Message types for VRP
#[derive(Debug, Clone)]
enum VrpMessage {
    /// Join request
    Join { name: String },
    /// Join acknowledgment
    JoinAck { participants: Vec<String> },
    /// Visual state update
    VisualUpdate { data: Vec<u8> },
    /// Leave notification
    Leave,
    /// Ping for latency measurement
    Ping { timestamp: i64 },
    /// Pong response
    Pong { timestamp: i64 },
}

impl VrpMessage {
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            VrpMessage::Join { name } => {
                buf.push(0x01);
                buf.extend(name.as_bytes());
            }
            VrpMessage::JoinAck { participants } => {
                buf.push(0x02);
                buf.push(participants.len() as u8);
                for p in participants {
                    buf.push(p.len() as u8);
                    buf.extend(p.as_bytes());
                }
            }
            VrpMessage::VisualUpdate { data } => {
                buf.push(0x03);
                buf.extend(data);
            }
            VrpMessage::Leave => {
                buf.push(0x04);
            }
            VrpMessage::Ping { timestamp } => {
                buf.push(0x05);
                buf.extend(&timestamp.to_le_bytes());
            }
            VrpMessage::Pong { timestamp } => {
                buf.push(0x06);
                buf.extend(&timestamp.to_le_bytes());
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
                let name = String::from_utf8_lossy(&data[1..]).to_string();
                Some(VrpMessage::Join { name })
            }
            0x02 => {
                let count = data.get(1).copied()? as usize;
                let mut participants = Vec::new();
                let mut pos = 2;
                for _ in 0..count {
                    let len = data.get(pos).copied()? as usize;
                    pos += 1;
                    let name = String::from_utf8_lossy(&data[pos..pos + len]).to_string();
                    participants.push(name);
                    pos += len;
                }
                Some(VrpMessage::JoinAck { participants })
            }
            0x03 => Some(VrpMessage::VisualUpdate {
                data: data[1..].to_vec(),
            }),
            0x04 => Some(VrpMessage::Leave),
            0x05 => {
                let timestamp = i64::from_le_bytes(data[1..9].try_into().ok()?);
                Some(VrpMessage::Ping { timestamp })
            }
            0x06 => {
                let timestamp = i64::from_le_bytes(data[1..9].try_into().ok()?);
                Some(VrpMessage::Pong { timestamp })
            }
            _ => None,
        }
    }
}

/// Simulated face state generator
struct FaceSimulator {
    emotion_phase: f32,
    blink_timer: f32,
    speaking: bool,
    mouth_phase: f32,
}

impl FaceSimulator {
    fn new() -> Self {
        Self {
            emotion_phase: 0.0,
            blink_timer: 0.0,
            speaking: false,
            mouth_phase: 0.0,
        }
    }

    fn update(&mut self, dt: f32, is_speaking: bool) {
        self.emotion_phase += dt * 0.5;
        self.blink_timer += dt;
        self.speaking = is_speaking;
        if is_speaking {
            self.mouth_phase += dt * 10.0;
        }
    }

    fn generate(&self, timestamp: StateTime) -> FaceState {
        let mut face = FaceState::new(timestamp);

        // Simulate subtle emotion changes
        face.emotion = EmotionVector {
            joy: 0.3 + 0.1 * (self.emotion_phase * 0.3).sin(),
            sadness: 0.0,
            anger: 0.0,
            fear: 0.0,
            surprise: 0.05 * (self.emotion_phase * 0.7).sin().abs(),
            disgust: 0.0,
            contempt: 0.0,
        };

        // Simulate blinking
        let blink = if (self.blink_timer % 4.0) < 0.15 {
            1.0
        } else {
            0.0
        };
        face.gaze = GazeState {
            yaw: 0.1 * (self.emotion_phase * 0.2).sin(),
            pitch: 0.05 * (self.emotion_phase * 0.3).cos(),
            looking_at_camera: true,
            blink,
        };

        // Simulate mouth for speech
        face.speaking = self.speaking;
        if self.speaking {
            face.mouth = MouthState {
                openness: 0.3 + 0.2 * self.mouth_phase.sin().abs(),
                smile: 0.1,
                viseme: Viseme::Neutral,
            };
        }

        // Subtle head movement
        face.head_rotation = (
            0.05 * (self.emotion_phase * 0.4).sin(),
            0.03 * (self.emotion_phase * 0.5).cos(),
            0.02 * (self.emotion_phase * 0.3).sin(),
        );

        face.present = true;
        face.confidence = 0.95;

        face
    }
}

/// Remote participant state
struct RemoteParticipant {
    name: String,
    addr: SocketAddr,
    node_id: NodeId,
    predictor: VisualPredictor,
    buffer: VisualStateBuffer,
    last_update: Instant,
    latency_ms: u32,
    degradation: DegradationLevel,
}

impl RemoteParticipant {
    fn new(name: String, addr: SocketAddr, node_id: u64) -> Self {
        Self {
            name,
            addr,
            node_id: NodeId::new(node_id),
            predictor: VisualPredictor::new(PredictionConfig::default()),
            buffer: VisualStateBuffer::new(10, 50),
            last_update: Instant::now(),
            latency_ms: 0,
            degradation: DegradationLevel::L0_FullPerception,
        }
    }

    fn update_visual(&mut self, state: VisualState) {
        self.buffer.push(state.clone());
        self.predictor.update(state);
        self.last_update = Instant::now();
    }

    fn get_current_visual(&mut self, current_time: StateTime) -> Option<VisualState> {
        // Try to get from buffer first
        if let Some(state) = self.buffer.get_at(current_time) {
            return Some(state);
        }

        // Fall back to prediction
        self.predictor.predict(current_time)
    }

    fn time_since_update(&self) -> Duration {
        self.last_update.elapsed()
    }
}

/// VRP Demo Node
struct VrpNode {
    name: String,
    node_id: NodeId,
    socket: Arc<UdpSocket>,
    participants: HashMap<SocketAddr, RemoteParticipant>,
    face_sim: FaceSimulator,
    sequence: u64,
    start_time: Instant,
    is_speaking: bool,
    degradation: DegradationLevel,
}

impl VrpNode {
    async fn new(name: String, port: u16) -> io::Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&addr).await?;

        Ok(Self {
            name,
            node_id: NodeId::new(rand::random()),
            socket: Arc::new(socket),
            participants: HashMap::new(),
            face_sim: FaceSimulator::new(),
            sequence: 0,
            start_time: Instant::now(),
            is_speaking: false,
            degradation: DegradationLevel::L0_FullPerception,
        })
    }

    fn current_time(&self) -> StateTime {
        StateTime::from_millis(self.start_time.elapsed().as_millis() as i64)
    }

    async fn join_call(&self, peer_addr: SocketAddr) -> io::Result<()> {
        let msg = VrpMessage::Join {
            name: self.name.clone(),
        };
        self.socket.send_to(&msg.encode(), peer_addr).await?;
        Ok(())
    }

    async fn send_visual_state(&mut self) -> io::Result<()> {
        self.sequence += 1;
        let timestamp = self.current_time();

        // Generate visual state
        let face = self.face_sim.generate(timestamp);
        let state = VisualState::keyframe(self.node_id, timestamp, self.sequence)
            .with_face(face)
            .with_degradation(self.degradation);

        // Apply degradation
        let state = state.degrade(self.degradation);

        // Encode
        let data = VisualEncoder::encode(&state);
        let msg = VrpMessage::VisualUpdate { data };
        let encoded = msg.encode();

        // Send to all participants
        for participant in self.participants.values() {
            self.socket.send_to(&encoded, participant.addr).await?;
        }

        Ok(())
    }

    async fn handle_message(&mut self, data: &[u8], from: SocketAddr) -> io::Result<()> {
        let msg = match VrpMessage::decode(data) {
            Some(m) => m,
            None => return Ok(()),
        };

        match msg {
            VrpMessage::Join { name } => {
                println!("\n📹 {} joined the call from {}", name, from);

                // Add participant
                let node_id = rand::random();
                self.participants
                    .insert(from, RemoteParticipant::new(name.clone(), from, node_id));

                // Send ack with participant list
                let names: Vec<String> =
                    self.participants.values().map(|p| p.name.clone()).collect();
                let ack = VrpMessage::JoinAck {
                    participants: names,
                };
                self.socket.send_to(&ack.encode(), from).await?;
            }

            VrpMessage::JoinAck { participants } => {
                println!("\n✅ Joined call. Participants: {:?}", participants);

                // Add the peer we connected to
                self.participants.entry(from).or_insert_with(|| {
                    RemoteParticipant::new(format!("Peer-{}", from.port()), from, rand::random())
                });
            }

            VrpMessage::VisualUpdate { data } => {
                if let Ok(state) = VisualEncoder::decode(&data) {
                    if let Some(participant) = self.participants.get_mut(&from) {
                        participant.update_visual(state);
                    }
                }
            }

            VrpMessage::Leave => {
                if let Some(p) = self.participants.remove(&from) {
                    println!("\n👋 {} left the call", p.name);
                }
            }

            VrpMessage::Ping { timestamp } => {
                let pong = VrpMessage::Pong { timestamp };
                self.socket.send_to(&pong.encode(), from).await?;
            }

            VrpMessage::Pong { timestamp } => {
                let now = self.current_time().as_millis();
                let latency = (now - timestamp) as u32;
                if let Some(p) = self.participants.get_mut(&from) {
                    p.latency_ms = latency;
                }
            }
        }

        Ok(())
    }

    fn render_ui(&mut self) {
        print!("\x1B[2J\x1B[H"); // Clear screen

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           ELARA VRP Demo - Video Call Simulation             ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║ You: {} ({})",
            self.name,
            if self.is_speaking {
                "🎤 Speaking"
            } else {
                "🔇 Muted"
            }
        );
        println!("║ Degradation: {:?}", self.degradation);
        println!("╠══════════════════════════════════════════════════════════════╣");

        if self.participants.is_empty() {
            println!("║ No participants yet. Waiting for others to join...          ║");
        } else {
            println!("║ Participants:                                                ║");

            let current_time = self.current_time();
            for p in self.participants.values_mut() {
                let time_since = p.time_since_update().as_millis();
                let status = if time_since < 100 {
                    "🟢 Live"
                } else if time_since < 500 {
                    "🟡 Predicting"
                } else {
                    "🔴 Stale"
                };

                // Get visual state (real or predicted)
                let visual_info = if let Some(state) = p.get_current_visual(current_time) {
                    if let Some(face) = &state.face {
                        let (emotion_name, _) = face.emotion.dominant();
                        let speaking = if face.speaking { "🗣️" } else { "😶" };
                        format!(
                            "{} {} conf:{:.0}%",
                            speaking,
                            emotion_name,
                            face.confidence * 100.0
                        )
                    } else {
                        "No face".to_string()
                    }
                } else {
                    "No data".to_string()
                };

                let predicting = if p.predictor.is_predicting() {
                    format!(" (pred:{})", p.predictor.prediction_count())
                } else {
                    String::new()
                };

                println!(
                    "║  {} {} ({:016x}) - {} {}ms {}{}",
                    status,
                    p.name,
                    p.node_id.0,
                    visual_info,
                    p.latency_ms,
                    format!("{:?}", p.degradation)
                        .chars()
                        .take(10)
                        .collect::<String>(),
                    predicting
                );
            }
        }

        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Commands: [s]peak toggle, [d]egrade, [u]pgrade, [q]uit       ║");
        println!("╚══════════════════════════════════════════════════════════════╝");

        io::stdout().flush().ok();
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ELARA VRP Demo - Video Call Simulation             ║");
    println!("║                                                              ║");
    println!("║  This demonstrates ELARA-native video calling:               ║");
    println!("║  - Video as Visual State (face/pose/scene)                   ║");
    println!("║  - Prediction under packet loss                              ║");
    println!("║  - Graceful degradation (L0-L5)                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Get user input
    print!("Your name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();

    print!("Port to listen on: ");
    io::stdout().flush()?;
    let mut port_str = String::new();
    io::stdin().read_line(&mut port_str)?;
    let port: u16 = port_str.trim().parse().unwrap_or(9000);

    print!("Peer address (or empty to wait): ");
    io::stdout().flush()?;
    let mut peer_str = String::new();
    io::stdin().read_line(&mut peer_str)?;
    let peer_addr: Option<SocketAddr> = peer_str.trim().parse().ok();

    // Create node
    let mut node = VrpNode::new(name, port).await?;
    println!("\n✅ VRP node started on port {}", port);

    // Join peer if specified
    if let Some(addr) = peer_addr {
        node.join_call(addr).await?;
        println!("📞 Joining call at {}...", addr);
    }

    // Create channels
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<char>(10);

    // Spawn input handler
    let cmd_tx_clone = cmd_tx.clone();
    std::thread::spawn(move || loop {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            if let Some(c) = input.chars().next() {
                let _ = cmd_tx_clone.blocking_send(c);
            }
        }
    });

    // Main loop
    let socket = node.socket.clone();
    let mut recv_buf = [0u8; 4096];
    let mut update_interval = interval(Duration::from_millis(33)); // ~30 FPS
    let mut ui_interval = interval(Duration::from_millis(100));
    let mut ping_interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            // Receive network messages
            result = socket.recv_from(&mut recv_buf) => {
                if let Ok((len, from)) = result {
                    node.handle_message(&recv_buf[..len], from).await?;
                }
            }

            // Send visual state updates
            _ = update_interval.tick() => {
                node.face_sim.update(0.033, node.is_speaking);
                if !node.participants.is_empty() {
                    node.send_visual_state().await?;
                }
            }

            // Update UI
            _ = ui_interval.tick() => {
                node.render_ui();
            }

            // Ping for latency
            _ = ping_interval.tick() => {
                let timestamp = node.current_time().as_millis();
                let ping = VrpMessage::Ping { timestamp };
                let encoded = ping.encode();
                for p in node.participants.values() {
                    socket.send_to(&encoded, p.addr).await?;
                }
            }

            // Handle commands
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    's' | 'S' => {
                        node.is_speaking = !node.is_speaking;
                    }
                    'd' | 'D' => {
                        node.degradation = node.degradation.degrade().unwrap_or(DegradationLevel::L5_LatentPresence);
                    }
                    'u' | 'U' => {
                        node.degradation = match node.degradation {
                            DegradationLevel::L5_LatentPresence => DegradationLevel::L4_MinimalPresence,
                            DegradationLevel::L4_MinimalPresence => DegradationLevel::L3_SymbolicPresence,
                            DegradationLevel::L3_SymbolicPresence => DegradationLevel::L2_FragmentedPerception,
                            DegradationLevel::L2_FragmentedPerception => DegradationLevel::L1_DistortedPerception,
                            _ => DegradationLevel::L0_FullPerception,
                        };
                    }
                    'q' | 'Q' => {
                        // Send leave message
                        let leave = VrpMessage::Leave;
                        let encoded = leave.encode();
                        for p in node.participants.values() {
                            socket.send_to(&encoded, p.addr).await?;
                        }
                        println!("\n👋 Leaving call...");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
