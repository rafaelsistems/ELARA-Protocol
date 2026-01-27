//! ELARA Node - Runtime loop implementation

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use elara_core::{
    Event, EventType, MutationOp, NodeId, PacketClass, RepresentationProfile, SessionId, StateId,
    TimeIntent, VersionVector,
};
use elara_crypto::{Identity, SecureFrameProcessor};
use elara_state::ReconciliationEngine;
use elara_time::TimeEngine;
use elara_wire::{Extensions, FixedHeader, Frame, FrameBuilder, AUTH_TAG_SIZE};

/// ELARA Node configuration
#[derive(Clone, Debug)]
pub struct NodeConfig {
    /// Tick interval
    pub tick_interval: Duration,
    /// Maximum incoming packet buffer
    pub max_packet_buffer: usize,
    /// Maximum outgoing packet buffer
    pub max_outgoing_buffer: usize,
    pub max_local_events: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeStats {
    pub ticks: u64,
    pub incoming_queued: u64,
    pub outgoing_popped: u64,
    pub local_events_queued: u64,
    pub events_signed: u64,
    pub packets_in: u64,
    pub packets_out: u64,
    pub last_tick_duration: Duration,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            tick_interval: Duration::from_millis(10),
            max_packet_buffer: 1000,
            max_outgoing_buffer: 1000,
            max_local_events: 1000,
        }
    }
}

/// ELARA Node - the runtime entity
pub struct Node {
    /// Node identity
    identity: Identity,
    /// Current session (if any)
    session_id: Option<SessionId>,
    /// Time engine
    time_engine: TimeEngine,
    /// State reconciliation engine
    state_engine: ReconciliationEngine,
    secure_processor: Option<SecureFrameProcessor>,
    /// Incoming packet buffer
    incoming: VecDeque<Frame>,
    /// Outgoing packet buffer
    outgoing: VecDeque<Frame>,
    /// Local events to send
    local_events: Vec<Event>,
    /// Event sequence counter
    event_seq: u64,
    /// Configuration
    config: NodeConfig,
    stats: RuntimeStats,
}

impl Node {
    /// Create a new node with generated identity
    pub fn new() -> Self {
        Self::with_config(NodeConfig::default())
    }

    /// Create a new node with custom configuration
    pub fn with_config(config: NodeConfig) -> Self {
        Node {
            identity: Identity::generate(),
            session_id: None,
            time_engine: TimeEngine::new(),
            state_engine: ReconciliationEngine::new(),
            secure_processor: None,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            local_events: Vec::new(),
            event_seq: 0,
            config,
            stats: RuntimeStats::default(),
        }
    }

    pub fn with_identity(identity: Identity, config: NodeConfig) -> Self {
        Node {
            identity,
            session_id: None,
            time_engine: TimeEngine::new(),
            state_engine: ReconciliationEngine::new(),
            secure_processor: None,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            local_events: Vec::new(),
            event_seq: 0,
            config,
            stats: RuntimeStats::default(),
        }
    }

    /// Get node ID
    pub fn node_id(&self) -> NodeId {
        self.identity.node_id()
    }

    /// Get session ID (if in session)
    pub fn session_id(&self) -> Option<SessionId> {
        self.session_id
    }

    /// Join a session
    pub fn join_session(&mut self, session_id: SessionId, session_key: [u8; 32]) {
        self.session_id = Some(session_id);
        self.secure_processor = Some(SecureFrameProcessor::new(
            session_id,
            self.node_id(),
            session_key,
        ));
    }

    pub fn join_session_unsecured(&mut self, session_id: SessionId) {
        self.session_id = Some(session_id);
        self.secure_processor = None;
    }

    /// Leave current session
    pub fn leave_session(&mut self) {
        self.session_id = None;
        self.secure_processor = None;
    }

    /// Queue an incoming frame for processing
    pub fn queue_incoming(&mut self, frame: Frame) {
        if self.incoming.len() < self.config.max_packet_buffer {
            self.incoming.push_back(frame);
            self.stats.incoming_queued += 1;
        }
    }

    /// Get next outgoing frame (if any)
    pub fn pop_outgoing(&mut self) -> Option<Frame> {
        let frame = self.outgoing.pop_front();
        if frame.is_some() {
            self.stats.outgoing_popped += 1;
            self.stats.packets_out += 1;
        }
        frame
    }

    /// Queue a local event to send
    pub fn queue_local_event(&mut self, event: Event) {
        if self.local_events.len() < self.config.max_local_events {
            self.local_events.push(event);
            self.stats.local_events_queued += 1;
        }
    }

    /// Execute one tick of the runtime loop
    /// This is the core 12-stage loop
    pub fn tick(&mut self) {
        let start = Instant::now();
        self.stats.ticks += 1;

        // Stage 1: Advance clocks (τp, τs) - NEVER SKIP
        self.time_engine.tick();

        // Stage 2: Ingest packets
        let packets = self.ingest_packets();

        // Stage 3: Decrypt and validate
        let validated = self.decrypt_and_validate(packets);

        // Stage 4: Classify events
        let events = self.classify_events(validated);

        // Stage 5: Update time model
        self.update_time_model(&events);

        // Stage 6: Reconcile state
        let _result = self.state_engine.process_events(events, &self.time_engine);
        self.state_engine.control_divergence();

        // Stage 7: Generate predictions
        self.generate_predictions();

        // Stage 8: Project to representation (handled externally)
        // Stage 9: Collect local events (already queued)

        // Stage 10: Authorize and sign
        let authorized = self.authorize_and_sign();

        // Stage 11: Build packets
        self.build_packets(authorized);

        // Stage 12: Schedule transmission (handled externally via pop_outgoing)

        self.stats.last_tick_duration = start.elapsed();
    }

    /// Stage 2: Ingest packets from buffer
    fn ingest_packets(&mut self) -> Vec<Frame> {
        let packets: Vec<Frame> = self.incoming.drain(..).collect();
        self.stats.packets_in += packets.len() as u64;
        packets
    }

    /// Stage 3: Decrypt and validate packets
    fn decrypt_and_validate(&mut self, packets: Vec<Frame>) -> Vec<Frame> {
        let Some(processor) = self.secure_processor.as_mut() else {
            return packets;
        };

        packets
            .into_iter()
            .filter_map(|frame| {
                let data = frame.serialize().ok()?;
                let decrypted = processor.decrypt_frame(&data).ok()?;
                let auth_tag = [0u8; AUTH_TAG_SIZE];
                Some(Frame {
                    header: decrypted.header,
                    extensions: decrypted.extensions,
                    payload: decrypted.payload,
                    auth_tag,
                })
            })
            .collect()
    }

    /// Stage 4: Extract events from validated packets
    fn classify_events(&self, packets: Vec<Frame>) -> Vec<Event> {
        let mut events = Vec::new();

        for frame in packets {
            let source = frame.header.node_id;
            let time_hint = frame.header.time_hint;
            events.extend(self.decode_event_blocks(&frame.payload, source, time_hint));
        }

        events
    }

    /// Stage 5: Update time model from events
    fn update_time_model(&mut self, events: &[Event]) {
        let reference = self.time_engine.tau_s();
        for event in events {
            let remote_time = event.time_intent.to_absolute(reference);
            let seq = (event.id.seq & 0xFFFF) as u16;
            self.time_engine
                .update_from_packet(event.source, remote_time, seq);
        }
    }

    /// Stage 7: Generate predictions for missing state
    fn generate_predictions(&mut self) {
        let dt_us = self.config.tick_interval.as_micros() as u64;
        {
            let field = self.state_engine.field_mut();
            for atom in field.atoms.values_mut() {
                atom.entropy.time_since_actual =
                    atom.entropy.time_since_actual.saturating_add(dt_us);
            }
        }

        let needs_prediction = self.state_engine.field().atoms_needing_prediction(100);
        if needs_prediction.is_empty() {
            return;
        }

        let field = self.state_engine.field_mut();
        for state_id in needs_prediction {
            if let Some(atom) = field.atoms.get_mut(&state_id) {
                let increase = match atom.state_type {
                    elara_core::StateType::Core => 0.01,
                    elara_core::StateType::Perceptual => 0.03,
                    elara_core::StateType::Enhancement => 0.05,
                    elara_core::StateType::Cosmetic => 0.07,
                };
                atom.entropy.increase(increase);
            }
        }
    }

    /// Stage 10: Authorize and sign local events
    fn authorize_and_sign(&mut self) -> Vec<Event> {
        let events: Vec<Event> = self.local_events.drain(..).collect();
        self.stats.events_signed += events.len() as u64;

        events
            .into_iter()
            .map(|mut event| {
                // Sign event
                let signature = self.identity.sign(&event.mutation.encode());
                event.authority_proof.signature = signature;
                event
            })
            .collect()
    }

    /// Stage 11: Build packets from authorized events
    fn build_packets(&mut self, _events: Vec<Event>) {
        let Some(processor) = self.secure_processor.as_mut() else {
            self.build_plain_packets(_events);
            return;
        };

        for event in _events {
            if self.outgoing.len() >= self.config.max_outgoing_buffer {
                break;
            }

            let class = Self::class_for_event(&event);
            let profile = Self::profile_for_event(&event);
            let time_hint = event.time_intent.ts_offset();
            let payload = Self::encode_event_block(&event);

            if let Ok(bytes) =
                processor.encrypt_frame(class, profile, time_hint, Extensions::new(), &payload)
            {
                if let Ok(frame) = Frame::parse(&bytes) {
                    self.outgoing.push_back(frame);
                }
            }
        }
    }

    fn build_plain_packets(&mut self, events: Vec<Event>) {
        for event in events {
            if self.outgoing.len() >= self.config.max_outgoing_buffer {
                break;
            }

            let class = Self::class_for_event(&event);
            let profile = Self::profile_for_event(&event);
            let time_hint = event.time_intent.ts_offset();
            let payload = Self::encode_event_block(&event);

            let session_id = self.session_id.unwrap_or(SessionId::ZERO);
            let mut header = FixedHeader::new(session_id, self.node_id());
            header.class = class;
            header.profile = profile;
            header.time_hint = time_hint;

            let frame = FrameBuilder::new(header).payload(payload).build();
            self.outgoing.push_back(frame);
        }
    }

    fn decode_event_blocks(&self, payload: &[u8], source: NodeId, time_hint: i32) -> Vec<Event> {
        let mut events = Vec::new();
        let mut offset = 0;

        while payload.len().saturating_sub(offset) >= 13 {
            let event_type = match EventType::from_byte(payload[offset]) {
                Some(t) => t,
                None => break,
            };
            offset += 1;

            let state_end = offset + 8;
            if state_end > payload.len() {
                break;
            }
            let state_id = match payload[offset..state_end].try_into() {
                Ok(bytes) => StateId::from_bytes(bytes),
                Err(_) => break,
            };
            offset = state_end;

            let version_len_end = offset + 2;
            if version_len_end > payload.len() {
                break;
            }
            let version_len = match payload[offset..version_len_end].try_into() {
                Ok(bytes) => u16::from_le_bytes(bytes) as usize,
                Err(_) => break,
            };
            offset = version_len_end;

            let version_end = offset + version_len;
            if version_end > payload.len() {
                break;
            }
            let version_ref = match Self::decode_version_vector(&payload[offset..version_end]) {
                Some(v) => v,
                None => break,
            };
            offset = version_end;

            let delta_len_end = offset + 2;
            if delta_len_end > payload.len() {
                break;
            }
            let delta_len = match payload[offset..delta_len_end].try_into() {
                Ok(bytes) => u16::from_le_bytes(bytes) as usize,
                Err(_) => break,
            };
            offset = delta_len_end;

            let delta_end = offset + delta_len;
            if delta_end > payload.len() {
                break;
            }
            let delta = &payload[offset..delta_end];
            let (mutation, used) = match MutationOp::decode(delta) {
                Some(decoded) => decoded,
                None => break,
            };
            if used != delta_len {
                break;
            }
            offset = delta_end;

            let seq = version_ref.get(source).saturating_add(1);
            let event = Event::new(source, seq, event_type, state_id, mutation)
                .with_version(version_ref)
                .with_time_intent(TimeIntent::new(time_hint));
            events.push(event);
        }

        events
    }

    fn encode_event_block(event: &Event) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(event.event_type.to_byte());
        buf.extend_from_slice(&event.target_state.to_bytes());

        let version = Self::encode_version_vector(&event.version_ref);
        buf.extend_from_slice(&(version.len() as u16).to_le_bytes());
        buf.extend_from_slice(&version);

        let delta = event.mutation.encode();
        buf.extend_from_slice(&(delta.len() as u16).to_le_bytes());
        buf.extend_from_slice(&delta);

        buf
    }

    fn encode_version_vector(version: &VersionVector) -> Vec<u8> {
        let mut entries = version.to_compact();
        entries.sort_by_key(|(node, _)| node.0);
        let mut buf = Vec::with_capacity(entries.len() * 16);
        for (node, count) in entries {
            buf.extend_from_slice(&node.to_bytes());
            buf.extend_from_slice(&count.to_le_bytes());
        }
        buf
    }

    fn decode_version_vector(buf: &[u8]) -> Option<VersionVector> {
        if !buf.len().is_multiple_of(16) {
            return None;
        }
        let mut entries = Vec::new();
        for chunk in buf.chunks_exact(16) {
            let node = match chunk[0..8].try_into() {
                Ok(bytes) => NodeId::from_bytes(bytes),
                Err(_) => return None,
            };
            let count = match chunk[8..16].try_into() {
                Ok(bytes) => u64::from_le_bytes(bytes),
                Err(_) => return None,
            };
            entries.push((node, count));
        }
        Some(VersionVector::from_compact(entries))
    }

    fn class_for_event(event: &Event) -> PacketClass {
        match event.event_type {
            EventType::StateRequest | EventType::StateResponse | EventType::GapFill => {
                PacketClass::Repair
            }
            EventType::VoiceFrame | EventType::VoiceMute => PacketClass::Perceptual,
            EventType::TypingStart | EventType::TypingStop | EventType::PresenceUpdate => {
                PacketClass::Perceptual
            }
            EventType::TextAppend
            | EventType::TextEdit
            | EventType::TextDelete
            | EventType::TextReact => PacketClass::Core,
            _ => PacketClass::Core,
        }
    }

    fn profile_for_event(event: &Event) -> RepresentationProfile {
        match event.event_type {
            EventType::VoiceFrame | EventType::VoiceMute => RepresentationProfile::VoiceMinimal,
            _ => RepresentationProfile::Textual,
        }
    }

    /// Get reference to time engine
    pub fn time_engine(&self) -> &TimeEngine {
        &self.time_engine
    }

    /// Get reference to state engine
    pub fn state_engine(&self) -> &ReconciliationEngine {
        &self.state_engine
    }

    /// Get mutable reference to state engine
    pub fn state_engine_mut(&mut self) -> &mut ReconciliationEngine {
        &mut self.state_engine
    }

    pub fn stats(&self) -> &RuntimeStats {
        &self.stats
    }

    /// Check if node is in a session
    pub fn in_session(&self) -> bool {
        self.session_id.is_some()
    }

    /// Get next event sequence number
    pub fn next_event_seq(&mut self) -> u64 {
        let seq = self.event_seq;
        self.event_seq += 1;
        seq
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new();
        assert!(!node.in_session());
    }

    #[test]
    fn test_node_tick() {
        let mut node = Node::new();

        // Should not panic
        node.tick();
        node.tick();
        node.tick();
    }

    #[test]
    fn test_local_event_buffer_limit() {
        let mut config = NodeConfig::default();
        config.max_local_events = 1;
        let mut node = Node::with_config(config);

        let event_a = Event::new(
            NodeId::new(1),
            1,
            EventType::TextAppend,
            StateId::new(1),
            MutationOp::Append(b"a".to_vec()),
        );
        let event_b = Event::new(
            NodeId::new(1),
            2,
            EventType::TextAppend,
            StateId::new(1),
            MutationOp::Append(b"b".to_vec()),
        );

        node.queue_local_event(event_a);
        node.queue_local_event(event_b);

        assert_eq!(node.stats().local_events_queued, 1);
    }

    #[test]
    fn test_prediction_entropy_advances() {
        let mut node = Node::new();
        let state_id = StateId::new(42);
        let node_id = node.node_id();

        {
            let field = node.state_engine_mut().field_mut();
            let atom = field.create_atom(state_id, elara_core::StateType::Core, node_id);
            atom.entropy.time_since_actual = 0;
        }

        node.tick();

        let field = node.state_engine().field();
        let atom = field.get(state_id).unwrap();
        assert!(atom.entropy.time_since_actual > 0);
    }

    #[test]
    fn test_node_session() {
        let mut node = Node::new();
        let session_id = SessionId::new(12345);
        let session_key = [0x42u8; 32];

        node.join_session(session_id, session_key);
        assert!(node.in_session());
        assert_eq!(node.session_id(), Some(session_id));

        node.leave_session();
        assert!(!node.in_session());
    }
}
