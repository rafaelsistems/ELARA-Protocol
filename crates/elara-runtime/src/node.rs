//! ELARA Node - Runtime loop implementation

use std::collections::VecDeque;
use std::time::Duration;

use elara_core::{Event, NodeId, SessionId};
use elara_crypto::{Identity, MultiRatchet, ReplayManager};
use elara_state::ReconciliationEngine;
use elara_time::TimeEngine;
use elara_wire::Frame;

/// ELARA Node configuration
#[derive(Clone, Debug)]
pub struct NodeConfig {
    /// Tick interval
    pub tick_interval: Duration,
    /// Maximum incoming packet buffer
    pub max_packet_buffer: usize,
    /// Maximum outgoing packet buffer
    pub max_outgoing_buffer: usize,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            tick_interval: Duration::from_millis(10),
            max_packet_buffer: 1000,
            max_outgoing_buffer: 1000,
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
    /// Multi-ratchet for crypto
    ratchet: Option<MultiRatchet>,
    /// Replay protection
    replay_manager: ReplayManager,
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
    /// Running flag
    running: bool,
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
            ratchet: None,
            replay_manager: ReplayManager::new(),
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            local_events: Vec::new(),
            event_seq: 0,
            config,
            running: false,
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
        self.ratchet = Some(MultiRatchet::new(&session_key));
    }

    /// Leave current session
    pub fn leave_session(&mut self) {
        self.session_id = None;
        self.ratchet = None;
    }

    /// Queue an incoming frame for processing
    pub fn queue_incoming(&mut self, frame: Frame) {
        if self.incoming.len() < self.config.max_packet_buffer {
            self.incoming.push_back(frame);
        }
    }

    /// Get next outgoing frame (if any)
    pub fn pop_outgoing(&mut self) -> Option<Frame> {
        self.outgoing.pop_front()
    }

    /// Queue a local event to send
    pub fn queue_local_event(&mut self, event: Event) {
        self.local_events.push(event);
    }

    /// Execute one tick of the runtime loop
    /// This is the core 12-stage loop
    pub fn tick(&mut self) {
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

        // Stage 7: Generate predictions
        self.generate_predictions();

        // Stage 8: Project to representation (handled externally)
        // Stage 9: Collect local events (already queued)

        // Stage 10: Authorize and sign
        let authorized = self.authorize_and_sign();

        // Stage 11: Build packets
        self.build_packets(authorized);

        // Stage 12: Schedule transmission (handled externally via pop_outgoing)
    }

    /// Stage 2: Ingest packets from buffer
    fn ingest_packets(&mut self) -> Vec<Frame> {
        self.incoming.drain(..).collect()
    }

    /// Stage 3: Decrypt and validate packets
    fn decrypt_and_validate(&mut self, packets: Vec<Frame>) -> Vec<Frame> {
        // For now, just pass through
        // TODO: Implement crypto validation
        packets
            .into_iter()
            .filter(|frame| {
                // Check replay
                let seq = frame.header.seq();
                let class = frame.header.class;
                let node = frame.header.node_id;

                self.replay_manager.accept(node, class, seq).is_ok()
            })
            .collect()
    }

    /// Stage 4: Extract events from validated packets
    fn classify_events(&self, _packets: Vec<Frame>) -> Vec<Event> {
        // TODO: Parse event blocks from payload
        Vec::new()
    }

    /// Stage 5: Update time model from events
    fn update_time_model(&mut self, _events: &[Event]) {
        // TODO: Extract timing info from events
    }

    /// Stage 7: Generate predictions for missing state
    fn generate_predictions(&mut self) {
        let needs_prediction = self
            .state_engine
            .field()
            .atoms_needing_prediction(100); // 100ms threshold

        for _state_id in needs_prediction {
            // TODO: Generate predictions based on state type
        }
    }

    /// Stage 10: Authorize and sign local events
    fn authorize_and_sign(&mut self) -> Vec<Event> {
        let events: Vec<Event> = self.local_events.drain(..).collect();

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
        // TODO: Build frames from events
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
