//! Test node infrastructure for load testing
//!
//! This module provides the `TestNode` struct which simulates an ELARA Protocol node
//! for load testing purposes. Test nodes can spawn, connect to each other, and
//! generate realistic message patterns.

use elara_runtime::{Node, NodeConfig};
use elara_core::{NodeId, SessionId, Event, EventType, StateId, MutationOp};
use std::time::Instant;
use std::collections::HashMap;

/// A simulated ELARA Protocol node for load testing
pub struct TestNode {
    /// The underlying ELARA node
    node: Node,
    /// Node identifier
    node_id: NodeId,
    /// Connected peer nodes (for message routing)
    pub(crate) peers: HashMap<NodeId, usize>,
    /// Message counter for tracking
    messages_sent: u64,
    /// Message counter for tracking
    messages_received: u64,
}

impl TestNode {
    /// Spawn a new test node with the given configuration
    pub fn spawn(config: NodeConfig) -> Result<Self, String> {
        let node = Node::with_config(config);
        let node_id = node.node_id();
        
        Ok(Self {
            node,
            node_id,
            peers: HashMap::new(),
            messages_sent: 0,
            messages_received: 0,
        })
    }
    
    /// Spawn a new test node with default configuration
    pub fn spawn_default() -> Result<Self, String> {
        Self::spawn(NodeConfig::default())
    }
    
    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
    
    /// Join a session (required for message exchange)
    pub fn join_session(&mut self, session_id: SessionId, session_key: [u8; 32]) {
        self.node.join_session(session_id, session_key);
    }
    
    /// Join a session without encryption (for testing)
    pub fn join_session_unsecured(&mut self, session_id: SessionId) {
        self.node.join_session_unsecured(session_id);
    }
    
    /// Connect to another test node
    pub fn connect_to(&mut self, peer: &TestNode) -> Result<(), String> {
        let peer_id = peer.node_id();
        let peer_index = self.peers.len();
        self.peers.insert(peer_id, peer_index);
        Ok(())
    }
    
    /// Generate and send a test message
    ///
    /// Returns the time taken to queue the message (for latency measurement)
    pub fn send_message(&mut self, payload: Vec<u8>) -> Result<Instant, String> {
        let start = Instant::now();
        
        let seq = self.node.next_event_seq();
        
        // Create a test event with the payload
        let event = Event::new(
            self.node_id,
            seq,
            EventType::TextAppend,
            StateId::new(0),
            MutationOp::Append(payload),
        );
        
        // Queue the event
        self.node.queue_local_event(event);
        
        // Process the node tick to generate outgoing frames
        self.node.tick();
        
        self.messages_sent += 1;
        
        Ok(start)
    }
    
    /// Receive and process incoming frames from another node
    pub fn receive_from(&mut self, peer: &mut TestNode) -> usize {
        let mut received_count = 0;
        
        // Pop all outgoing frames from the peer
        while let Some(frame) = peer.node.pop_outgoing() {
            // Queue as incoming on this node
            self.node.queue_incoming(frame);
            received_count += 1;
        }
        
        // Process incoming frames
        if received_count > 0 {
            self.node.tick();
            self.messages_received += received_count as u64;
        }
        
        received_count
    }
    
    /// Tick the node to process queued events
    pub fn tick(&mut self) {
        self.node.tick();
    }
    
    /// Get the number of messages sent by this node
    pub fn messages_sent(&self) -> u64 {
        self.messages_sent
    }
    
    /// Get the number of messages received by this node
    pub fn messages_received(&self) -> u64 {
        self.messages_received
    }
    
    /// Get a reference to the underlying node
    pub fn node(&self) -> &Node {
        &self.node
    }
    
    /// Get a mutable reference to the underlying node
    pub fn node_mut(&mut self) -> &mut Node {
        &mut self.node
    }
    
    /// Shutdown the test node and cleanup resources
    pub fn shutdown(self) {
        // Node will be dropped, cleaning up resources
        drop(self);
    }
}

/// Generate a test message payload with the given size
pub fn generate_test_message(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

/// Generate a test message payload with random data
#[allow(dead_code)]
pub fn generate_random_message(size: usize) -> Vec<u8> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    
    let mut data = Vec::with_capacity(size);
    let hasher_builder = RandomState::new();
    
    for i in 0..size {
        let mut hasher = hasher_builder.build_hasher();
        i.hash(&mut hasher);
        data.push((hasher.finish() & 0xFF) as u8);
    }
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_spawn() {
        let node = TestNode::spawn_default();
        assert!(node.is_ok());
    }

    #[test]
    fn test_node_connection() {
        let mut node1 = TestNode::spawn_default().unwrap();
        let node2 = TestNode::spawn_default().unwrap();
        
        let result = node1.connect_to(&node2);
        assert!(result.is_ok());
        assert_eq!(node1.peers.len(), 1);
    }

    #[test]
    fn test_message_generation() {
        let msg = generate_test_message(100);
        assert_eq!(msg.len(), 100);
        
        let random_msg = generate_random_message(100);
        assert_eq!(random_msg.len(), 100);
    }

    #[test]
    fn test_send_message() {
        let mut node = TestNode::spawn_default().unwrap();
        let session_id = SessionId::new(1);
        node.join_session_unsecured(session_id);
        
        let payload = generate_test_message(64);
        let result = node.send_message(payload);
        
        assert!(result.is_ok());
        assert_eq!(node.messages_sent(), 1);
    }

    #[test]
    fn test_message_exchange() {
        let mut node1 = TestNode::spawn_default().unwrap();
        let mut node2 = TestNode::spawn_default().unwrap();
        
        let session_id = SessionId::new(1);
        node1.join_session_unsecured(session_id);
        node2.join_session_unsecured(session_id);
        
        node1.connect_to(&node2).unwrap();
        
        // Send message from node1
        let payload = generate_test_message(64);
        node1.send_message(payload).unwrap();
        
        // Receive on node2
        let _received = node2.receive_from(&mut node1);
        
        assert_eq!(node1.messages_sent(), 1);
        // Note: received count depends on frame generation
    }
}
