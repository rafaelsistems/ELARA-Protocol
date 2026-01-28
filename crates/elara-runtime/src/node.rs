//! ELARA Node - Runtime loop implementation

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use elara_core::{
    Event, EventType, MessageId, MutationOp, NodeId, PacketClass, RepresentationProfile, SessionId,
    StateId, StateTime, TimeIntent, VersionVector,
};
use elara_crypto::{Identity, SecureFrameProcessor};
use elara_state::ReconciliationEngine;
use elara_time::TimeEngine;
use elara_visual::{
    livestream_state_id, stream_visual_state_id, visual_state_id, PredictionConfig, VisualEncoder,
    VisualPredictor, VisualState, VisualStateBuffer,
};
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

#[derive(Clone, Debug)]
pub struct FeedItem {
    pub id: MessageId,
    pub author: NodeId,
    pub content: Vec<u8>,
    pub timestamp: StateTime,
    pub deleted: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FeedStream {
    pub items: Vec<FeedItem>,
}

#[derive(Clone, Debug)]
pub struct StreamMetadata {
    pub source: NodeId,
    pub started_at: StateTime,
    pub data: Vec<u8>,
}

impl FeedItem {
    fn decode(buf: &[u8]) -> Option<(Self, usize)> {
        if buf.len() < 27 {
            return None;
        }

        let id = MessageId(u64::from_le_bytes(buf[0..8].try_into().ok()?));
        let author = NodeId::from_bytes(buf[8..16].try_into().ok()?);
        let timestamp = StateTime::from_micros(i64::from_le_bytes(buf[16..24].try_into().ok()?));
        let deleted = buf[24] != 0;
        let mut offset = 25;

        if buf.len() < offset + 2 {
            return None;
        }
        let content_len = u16::from_le_bytes(buf[offset..offset + 2].try_into().ok()?) as usize;
        offset += 2;

        if buf.len() < offset + content_len {
            return None;
        }
        let content = buf[offset..offset + content_len].to_vec();
        offset += content_len;

        Some((
            FeedItem {
                id,
                author,
                content,
                timestamp,
                deleted,
            },
            offset,
        ))
    }
}

impl FeedStream {
    fn from_bytes(data: &[u8]) -> Self {
        let mut stream = FeedStream::default();
        let mut offset = 0;
        while offset < data.len() {
            let Some((item, used)) = FeedItem::decode(&data[offset..]) else {
                break;
            };
            stream.apply_item(item);
            offset += used;
        }
        stream
    }

    fn apply_item(&mut self, item: FeedItem) {
        if let Some(existing) = self.items.iter_mut().find(|m| m.id == item.id) {
            existing.deleted = item.deleted;
            existing.content = item.content;
            existing.timestamp = item.timestamp;
            existing.author = item.author;
            return;
        }

        let pos = self
            .items
            .binary_search_by(|m| m.timestamp.cmp(&item.timestamp))
            .unwrap_or_else(|p| p);
        self.items.insert(pos, item);
    }
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
    stream_metadata: HashMap<u64, StreamMetadata>,
    visual_buffers: HashMap<NodeId, VisualStateBuffer>,
    visual_predictors: HashMap<NodeId, VisualPredictor>,
    stream_visual_buffers: HashMap<u64, VisualStateBuffer>,
    stream_visual_predictors: HashMap<u64, VisualPredictor>,
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
            stream_metadata: HashMap::new(),
            visual_buffers: HashMap::new(),
            visual_predictors: HashMap::new(),
            stream_visual_buffers: HashMap::new(),
            stream_visual_predictors: HashMap::new(),
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
            stream_metadata: HashMap::new(),
            visual_buffers: HashMap::new(),
            visual_predictors: HashMap::new(),
            stream_visual_buffers: HashMap::new(),
            stream_visual_predictors: HashMap::new(),
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

    pub fn queue_visual_keyframe(&mut self, state: &VisualState) {
        self.queue_visual_event(
            visual_state_id(state.source),
            state,
            EventType::VisualKeyframe,
        );
    }

    pub fn queue_visual_delta(&mut self, state: &VisualState) {
        self.queue_visual_event(visual_state_id(state.source), state, EventType::VisualDelta);
    }

    pub fn queue_stream_visual_keyframe(&mut self, stream_id: u64, state: &VisualState) {
        self.queue_visual_event(
            stream_visual_state_id(stream_id),
            state,
            EventType::VisualKeyframe,
        );
    }

    pub fn queue_stream_visual_delta(&mut self, stream_id: u64, state: &VisualState) {
        self.queue_visual_event(
            stream_visual_state_id(stream_id),
            state,
            EventType::VisualDelta,
        );
    }

    pub fn queue_stream_start(&mut self, stream_id: u64, metadata: Vec<u8>, timestamp: StateTime) {
        self.stream_metadata.insert(
            stream_id,
            StreamMetadata {
                source: self.node_id(),
                started_at: timestamp,
                data: metadata.clone(),
            },
        );
        let target_state = livestream_state_id(stream_id);
        let seq = self.next_event_seq();
        let time_intent = self.time_intent_for(timestamp);
        let event = Event::new(
            self.node_id(),
            seq,
            EventType::StreamStart,
            target_state,
            MutationOp::Set(metadata),
        )
        .with_time_intent(time_intent);
        self.queue_local_event(event);
    }

    pub fn queue_stream_end(&mut self, stream_id: u64, timestamp: StateTime) {
        self.stream_metadata.remove(&stream_id);
        self.stream_visual_buffers.remove(&stream_id);
        self.stream_visual_predictors.remove(&stream_id);
        let target_state = livestream_state_id(stream_id);
        let seq = self.next_event_seq();
        let time_intent = self.time_intent_for(timestamp);
        let event = Event::new(
            self.node_id(),
            seq,
            EventType::StreamEnd,
            target_state,
            MutationOp::Delete,
        )
        .with_time_intent(time_intent);
        self.queue_local_event(event);

        let visual_state = stream_visual_state_id(stream_id);
        let seq = self.next_event_seq();
        let event = Event::new(
            self.node_id(),
            seq,
            EventType::StreamEnd,
            visual_state,
            MutationOp::Delete,
        )
        .with_time_intent(time_intent);
        self.queue_local_event(event);
    }

    pub fn queue_feed_append(&mut self, feed_state: StateId, data: Vec<u8>, timestamp: StateTime) {
        let seq = self.next_event_seq();
        let time_intent = self.time_intent_for(timestamp);
        let event = Event::new(
            self.node_id(),
            seq,
            EventType::FeedAppend,
            feed_state,
            MutationOp::Append(data),
        )
        .with_time_intent(time_intent);
        self.queue_local_event(event);
    }

    pub fn queue_feed_delete(&mut self, feed_state: StateId, timestamp: StateTime) {
        let seq = self.next_event_seq();
        let time_intent = self.time_intent_for(timestamp);
        let event = Event::new(
            self.node_id(),
            seq,
            EventType::FeedDelete,
            feed_state,
            MutationOp::Delete,
        )
        .with_time_intent(time_intent);
        self.queue_local_event(event);
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
    fn classify_events(&mut self, packets: Vec<Frame>) -> Vec<Event> {
        let mut events = Vec::new();

        for frame in packets {
            let source = frame.header.node_id;
            let time_hint = frame.header.time_hint;
            let frame_events = self.decode_event_blocks(&frame.payload, source, time_hint);
            for event in &frame_events {
                self.handle_event_side_effects(event);
            }
            events.extend(frame_events);
        }

        events
    }

    fn handle_event_side_effects(&mut self, event: &Event) {
        match event.event_type {
            EventType::StreamStart => {
                let stream_id = event.target_state.instance();
                let started_at = event.absolute_time(self.time_engine.tau_s());

                if let MutationOp::Set(data) = &event.mutation {
                    self.stream_metadata.insert(
                        stream_id,
                        StreamMetadata {
                            source: event.source,
                            started_at,
                            data: data.clone(),
                        },
                    );
                }

                let field = self.state_engine_mut().field_mut();
                if field.get(event.target_state).is_none() {
                    field.create_atom(
                        event.target_state,
                        elara_core::StateType::Core,
                        event.source,
                    );
                }

                let visual_state = stream_visual_state_id(stream_id);
                if field.get(visual_state).is_none() {
                    field.create_atom(
                        visual_state,
                        elara_core::StateType::Perceptual,
                        event.source,
                    );
                }
            }
            EventType::StreamEnd => {
                let stream_id = event.target_state.instance();
                self.stream_metadata.remove(&stream_id);
                self.stream_visual_buffers.remove(&stream_id);
                self.stream_visual_predictors.remove(&stream_id);

                let field = self.state_engine_mut().field_mut();
                field.remove(livestream_state_id(stream_id));
                field.remove(stream_visual_state_id(stream_id));
            }
            EventType::VisualKeyframe | EventType::VisualDelta => {
                if let MutationOp::Set(data) = &event.mutation {
                    if let Ok(state) = VisualEncoder::decode(data) {
                        let stream_id = if self
                            .stream_metadata
                            .contains_key(&event.target_state.instance())
                        {
                            Some(event.target_state.instance())
                        } else {
                            None
                        };
                        self.update_visual_state(state, stream_id);
                    }
                }
            }
            _ => {}
        }
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
            EventType::VisualKeyframe | EventType::VisualDelta => PacketClass::Perceptual,
            EventType::TextAppend
            | EventType::TextEdit
            | EventType::TextDelete
            | EventType::TextReact => PacketClass::Core,
            EventType::FeedAppend | EventType::FeedDelete => PacketClass::Core,
            EventType::StreamStart | EventType::StreamEnd => PacketClass::Core,
            _ => PacketClass::Core,
        }
    }

    fn profile_for_event(event: &Event) -> RepresentationProfile {
        match event.event_type {
            EventType::VoiceFrame | EventType::VoiceMute => RepresentationProfile::VoiceMinimal,
            EventType::VisualKeyframe | EventType::VisualDelta => {
                RepresentationProfile::VideoStandard
            }
            EventType::StreamStart | EventType::StreamEnd => {
                RepresentationProfile::StreamAsymmetric
            }
            _ => RepresentationProfile::Textual,
        }
    }

    fn queue_visual_event(
        &mut self,
        target_state: StateId,
        state: &VisualState,
        event_type: EventType,
    ) {
        let seq = self.next_event_seq();
        let time_intent = self.time_intent_for(state.timestamp);
        let payload = VisualEncoder::encode(state);
        let event = Event::new(
            self.node_id(),
            seq,
            event_type,
            target_state,
            MutationOp::Set(payload),
        )
        .with_time_intent(time_intent);
        self.queue_local_event(event);
    }

    fn time_intent_for(&self, timestamp: StateTime) -> TimeIntent {
        let reference = self.time_engine.tau_s();
        let offset = timestamp.to_wire_offset(reference);
        TimeIntent::new(offset)
    }

    fn visual_buffer_config(&self) -> (usize, u32) {
        let stability = self.time_engine.stability_score().clamp(0.1, 1.0);
        let instability = 1.0 - stability;
        let buffer_size = (24.0 + instability * 40.0).round() as usize;
        let buffer_delay_ms = (40.0 + instability * 140.0).round() as u32;
        (buffer_size.clamp(16, 64), buffer_delay_ms.clamp(30, 200))
    }

    fn visual_predictor_config(&self) -> PredictionConfig {
        let stability = self.time_engine.stability_score().clamp(0.1, 1.0);
        let instability = (1.0 - stability) as f32;
        PredictionConfig {
            max_horizon_ms: ((400.0 + instability as f64 * 800.0).round() as u32).clamp(300, 1200),
            confidence_decay: (0.08 + instability * 0.12).clamp(0.06, 0.2),
            min_confidence: (0.25 + instability * 0.2).clamp(0.2, 0.5),
            ..PredictionConfig::default()
        }
    }

    fn update_visual_state(&mut self, state: VisualState, stream_id: Option<u64>) {
        let (buffer_size, buffer_delay_ms) = self.visual_buffer_config();
        let predictor_config = self.visual_predictor_config();

        let buffer = self
            .visual_buffers
            .entry(state.source)
            .or_insert_with(|| VisualStateBuffer::new(buffer_size, buffer_delay_ms));
        buffer.push(state.clone());
        let predictor = self
            .visual_predictors
            .entry(state.source)
            .or_insert_with(|| VisualPredictor::new(predictor_config.clone()));
        predictor.update(state.clone());

        if let Some(stream_id) = stream_id {
            let stream_buffer = self
                .stream_visual_buffers
                .entry(stream_id)
                .or_insert_with(|| VisualStateBuffer::new(buffer_size, buffer_delay_ms));
            stream_buffer.push(state.clone());
            let stream_predictor = self
                .stream_visual_predictors
                .entry(stream_id)
                .or_insert_with(|| VisualPredictor::new(predictor_config));
            stream_predictor.update(state);
        }
    }

    pub fn visual_state(&self, node_id: NodeId) -> Option<VisualState> {
        let atom = self.state_engine.field().get(visual_state_id(node_id))?;
        VisualEncoder::decode(&atom.value).ok()
    }

    pub fn visual_state_at(
        &mut self,
        node_id: NodeId,
        target_time: StateTime,
    ) -> Option<VisualState> {
        let state = self
            .visual_buffers
            .get(&node_id)
            .and_then(|buffer| buffer.get_at(target_time));
        if state.is_some() {
            return state;
        }
        self.visual_predictors
            .get_mut(&node_id)
            .and_then(|predictor| predictor.predict(target_time))
    }

    pub fn visual_state_now(&mut self, node_id: NodeId) -> Option<VisualState> {
        let now = self.time_engine.tau_s();
        self.visual_state_at(node_id, now)
    }

    pub fn stream_visual_state(&self, stream_id: u64) -> Option<VisualState> {
        let atom = self
            .state_engine
            .field()
            .get(stream_visual_state_id(stream_id))?;
        VisualEncoder::decode(&atom.value).ok()
    }

    pub fn stream_visual_state_at(
        &mut self,
        stream_id: u64,
        target_time: StateTime,
    ) -> Option<VisualState> {
        let state = self
            .stream_visual_buffers
            .get(&stream_id)
            .and_then(|buffer| buffer.get_at(target_time));
        if state.is_some() {
            return state;
        }
        self.stream_visual_predictors
            .get_mut(&stream_id)
            .and_then(|predictor| predictor.predict(target_time))
    }

    pub fn stream_visual_state_now(&mut self, stream_id: u64) -> Option<VisualState> {
        let now = self.time_engine.tau_s();
        self.stream_visual_state_at(stream_id, now)
    }

    pub fn feed_stream(&self, feed_state: StateId) -> FeedStream {
        self.state_engine
            .field()
            .get(feed_state)
            .map(|atom| FeedStream::from_bytes(&atom.value))
            .unwrap_or_default()
    }

    pub fn stream_metadata(&self, stream_id: u64) -> Option<&StreamMetadata> {
        self.stream_metadata.get(&stream_id)
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
    use elara_core::{PacketClass, RepresentationProfile};
    use elara_msp::text::{feed_stream_id as feed_id, FeedItem as MspFeedItem};

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
        let config = NodeConfig {
            max_local_events: 1,
            ..Default::default()
        };
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

    fn build_payload(event_type: EventType, state_id: StateId, mutation: MutationOp) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(event_type.to_byte());
        buf.extend_from_slice(&state_id.to_bytes());

        // Empty version vector
        buf.extend_from_slice(&(0u16).to_le_bytes());

        let delta = mutation.encode();
        buf.extend_from_slice(&(delta.len() as u16).to_le_bytes());
        buf.extend_from_slice(&delta);

        buf
    }

    fn incoming_frame_for(
        session: SessionId,
        source: NodeId,
        class: PacketClass,
        profile: RepresentationProfile,
        time_hint: i32,
        payload: Vec<u8>,
    ) -> Frame {
        let mut header = FixedHeader::new(session, source);
        header.class = class;
        header.profile = profile;
        header.time_hint = time_hint;
        FrameBuilder::new(header).payload(payload).build()
    }

    #[test]
    fn test_stream_start_side_effects_on_incoming_frame() {
        let mut node = Node::new();
        let stream_id = 42u64;
        let target_state = livestream_state_id(stream_id);

        let payload = build_payload(
            EventType::StreamStart,
            target_state,
            MutationOp::Set(vec![1, 2, 3]),
        );

        let frame = incoming_frame_for(
            SessionId::new(1),
            NodeId::new(9000),
            PacketClass::Core,
            RepresentationProfile::StreamAsymmetric,
            0,
            payload,
        );

        node.queue_incoming(frame);
        node.tick();

        let field = node.state_engine().field();
        assert!(field.contains(livestream_state_id(stream_id)));
        assert!(field.contains(stream_visual_state_id(stream_id)));
        assert!(node.stream_metadata(stream_id).is_some());
    }

    #[test]
    fn test_stream_end_removes_atoms() {
        let mut node = Node::new();
        let stream_id = 99u64;

        // Start
        let start_payload = build_payload(
            EventType::StreamStart,
            livestream_state_id(stream_id),
            MutationOp::Set(vec![9, 9, 9]),
        );
        let start_frame = incoming_frame_for(
            SessionId::new(1),
            NodeId::new(7000),
            PacketClass::Core,
            RepresentationProfile::StreamAsymmetric,
            0,
            start_payload,
        );
        node.queue_incoming(start_frame);
        node.tick();

        // End
        let end_payload = build_payload(
            EventType::StreamEnd,
            livestream_state_id(stream_id),
            MutationOp::Delete,
        );
        let end_frame = incoming_frame_for(
            SessionId::new(1),
            NodeId::new(7000),
            PacketClass::Core,
            RepresentationProfile::StreamAsymmetric,
            0,
            end_payload,
        );
        node.queue_incoming(end_frame);
        node.tick();

        let field = node.state_engine().field();
        assert!(!field.contains(livestream_state_id(stream_id)));
        assert!(!field.contains(stream_visual_state_id(stream_id)));
        assert!(node.stream_metadata(stream_id).is_none());
    }

    #[test]
    fn test_feed_append_roundtrip() {
        let mut node = Node::new();
        let feed_state = feed_id(7);

        // Build a feed item and encode via MSP
        let item = MspFeedItem::new(
            elara_core::MessageId(1),
            NodeId::new(123),
            b"hello feed".to_vec(),
            elara_core::StateTime::from_millis(0),
        );
        let encoded_item = item.encode();

        let payload = build_payload(
            EventType::FeedAppend,
            feed_state,
            MutationOp::Append(encoded_item),
        );
        let frame = incoming_frame_for(
            SessionId::new(1),
            NodeId::new(5000),
            PacketClass::Core,
            RepresentationProfile::Textual,
            0,
            payload,
        );

        node.queue_incoming(frame);
        node.tick();

        let stream = node.feed_stream(feed_state);
        assert_eq!(stream.items.len(), 1);
        let first = &stream.items[0];
        assert_eq!(first.id.0, 1);
        assert_eq!(first.author, NodeId::new(123));
        assert_eq!(first.content, b"hello feed".to_vec());
        assert!(!first.deleted);
    }
}
