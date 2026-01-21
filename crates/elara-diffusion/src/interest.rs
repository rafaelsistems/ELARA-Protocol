//! Interest Model - Who wants to observe what state
//!
//! Interest determines which nodes receive state updates.
//! This enables efficient propagation - only send to those who care.

use elara_core::NodeId;
use std::collections::{HashMap, HashSet};

/// Interest level - how much a node cares about a state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterestLevel {
    /// No interest - don't send updates
    None = 0,
    /// Low interest - send major updates only
    Low = 1,
    /// Medium interest - send regular updates
    Medium = 2,
    /// High interest - send all updates with low latency
    High = 3,
    /// Critical interest - prioritize above all else
    Critical = 4,
}

impl Default for InterestLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Interest declaration from a node
#[derive(Debug, Clone)]
pub struct InterestDeclaration {
    /// The node declaring interest
    pub node: NodeId,
    /// The state they're interested in
    pub state_id: u64,
    /// Level of interest
    pub level: InterestLevel,
    /// Timestamp of declaration
    pub timestamp: i64,
    /// Time-to-live in milliseconds (0 = permanent)
    pub ttl_ms: u32,
}

impl InterestDeclaration {
    /// Create a new interest declaration
    pub fn new(node: NodeId, state_id: u64, level: InterestLevel) -> Self {
        Self {
            node,
            state_id,
            level,
            timestamp: 0,
            ttl_ms: 0,
        }
    }
    
    /// Set TTL
    pub fn with_ttl(mut self, ttl_ms: u32) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }
    
    /// Check if this declaration has expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        if self.ttl_ms == 0 {
            return false;
        }
        current_time > self.timestamp + self.ttl_ms as i64
    }
}

/// Interest map - tracks who is interested in what
#[derive(Debug, Clone, Default)]
pub struct InterestMap {
    /// State ID -> (Node ID -> Interest Level)
    interests: HashMap<u64, HashMap<NodeId, InterestLevel>>,
    
    /// Node ID -> Set of state IDs they're interested in
    node_interests: HashMap<NodeId, HashSet<u64>>,
}

impl InterestMap {
    /// Create a new interest map
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register interest
    pub fn register(&mut self, decl: InterestDeclaration) {
        // Add to state -> nodes map
        self.interests
            .entry(decl.state_id)
            .or_default()
            .insert(decl.node, decl.level);
        
        // Add to node -> states map
        if decl.level != InterestLevel::None {
            self.node_interests
                .entry(decl.node)
                .or_default()
                .insert(decl.state_id);
        } else {
            // Remove if interest is None
            if let Some(states) = self.node_interests.get_mut(&decl.node) {
                states.remove(&decl.state_id);
            }
        }
    }
    
    /// Unregister interest
    pub fn unregister(&mut self, node: NodeId, state_id: u64) {
        if let Some(nodes) = self.interests.get_mut(&state_id) {
            nodes.remove(&node);
        }
        if let Some(states) = self.node_interests.get_mut(&node) {
            states.remove(&state_id);
        }
    }
    
    /// Get all nodes interested in a state
    pub fn interested_nodes(&self, state_id: u64) -> Vec<(NodeId, InterestLevel)> {
        self.interests
            .get(&state_id)
            .map(|nodes| {
                nodes.iter()
                    .filter(|(_, level)| **level != InterestLevel::None)
                    .map(|(node, level)| (*node, *level))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get nodes with at least a certain interest level
    pub fn nodes_with_interest(&self, state_id: u64, min_level: InterestLevel) -> Vec<NodeId> {
        self.interests
            .get(&state_id)
            .map(|nodes| {
                nodes.iter()
                    .filter(|(_, level)| **level >= min_level)
                    .map(|(node, _)| *node)
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get all states a node is interested in
    pub fn node_states(&self, node: NodeId) -> Vec<u64> {
        self.node_interests
            .get(&node)
            .map(|states| states.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// Get interest level for a specific node and state
    pub fn get_interest(&self, node: NodeId, state_id: u64) -> InterestLevel {
        self.interests
            .get(&state_id)
            .and_then(|nodes| nodes.get(&node))
            .copied()
            .unwrap_or(InterestLevel::None)
    }
    
    /// Count interested nodes for a state
    pub fn interest_count(&self, state_id: u64) -> usize {
        self.interests
            .get(&state_id)
            .map(|nodes| nodes.values().filter(|l| **l != InterestLevel::None).count())
            .unwrap_or(0)
    }
    
    /// Remove a node entirely (they disconnected)
    pub fn remove_node(&mut self, node: NodeId) {
        // Remove from all state interest maps
        for nodes in self.interests.values_mut() {
            nodes.remove(&node);
        }
        
        // Remove their interest set
        self.node_interests.remove(&node);
    }
}

/// Livestream interest - specialized for streaming scenarios
#[derive(Debug, Clone)]
pub struct LivestreamInterest {
    /// Stream ID
    pub stream_id: u64,
    
    /// Interest map for this stream
    pub interests: InterestMap,
    
    /// Active viewers (high interest in visual/audio)
    pub active_viewers: HashSet<NodeId>,
    
    /// Lurkers (low interest, just presence)
    pub lurkers: HashSet<NodeId>,
}

impl LivestreamInterest {
    /// Create a new livestream interest tracker
    pub fn new(stream_id: u64) -> Self {
        Self {
            stream_id,
            interests: InterestMap::new(),
            active_viewers: HashSet::new(),
            lurkers: HashSet::new(),
        }
    }
    
    /// Add an active viewer
    pub fn add_viewer(&mut self, node: NodeId) {
        self.active_viewers.insert(node);
        self.lurkers.remove(&node);
        
        // Register high interest in visual and audio
        self.interests.register(InterestDeclaration::new(
            node, self.stream_id, InterestLevel::High
        ));
        self.interests.register(InterestDeclaration::new(
            node, self.stream_id + 1, InterestLevel::High
        ));
    }
    
    /// Add a lurker (low bandwidth mode)
    pub fn add_lurker(&mut self, node: NodeId) {
        self.lurkers.insert(node);
        self.active_viewers.remove(&node);
        
        // Register low interest
        self.interests.register(InterestDeclaration::new(
            node, self.stream_id, InterestLevel::Low
        ));
    }
    
    /// Remove a viewer
    pub fn remove_viewer(&mut self, node: NodeId) {
        self.active_viewers.remove(&node);
        self.lurkers.remove(&node);
        self.interests.remove_node(node);
    }
    
    /// Get total viewer count
    pub fn viewer_count(&self) -> usize {
        self.active_viewers.len() + self.lurkers.len()
    }
    
    /// Get active viewer count
    pub fn active_count(&self) -> usize {
        self.active_viewers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interest_registration() {
        let mut map = InterestMap::new();
        
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);
        let state_id = 100;
        
        map.register(InterestDeclaration::new(node1, state_id, InterestLevel::High));
        map.register(InterestDeclaration::new(node2, state_id, InterestLevel::Medium));
        
        assert_eq!(map.interest_count(state_id), 2);
        assert_eq!(map.get_interest(node1, state_id), InterestLevel::High);
        assert_eq!(map.get_interest(node2, state_id), InterestLevel::Medium);
    }
    
    #[test]
    fn test_nodes_with_interest() {
        let mut map = InterestMap::new();
        
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);
        let node3 = NodeId::new(3);
        let state_id = 100;
        
        map.register(InterestDeclaration::new(node1, state_id, InterestLevel::High));
        map.register(InterestDeclaration::new(node2, state_id, InterestLevel::Medium));
        map.register(InterestDeclaration::new(node3, state_id, InterestLevel::Low));
        
        let high_nodes = map.nodes_with_interest(state_id, InterestLevel::High);
        assert_eq!(high_nodes.len(), 1);
        
        let medium_nodes = map.nodes_with_interest(state_id, InterestLevel::Medium);
        assert_eq!(medium_nodes.len(), 2);
    }
    
    #[test]
    fn test_livestream_interest() {
        let mut stream = LivestreamInterest::new(1000);
        
        let viewer1 = NodeId::new(1);
        let viewer2 = NodeId::new(2);
        let lurker = NodeId::new(3);
        
        stream.add_viewer(viewer1);
        stream.add_viewer(viewer2);
        stream.add_lurker(lurker);
        
        assert_eq!(stream.viewer_count(), 3);
        assert_eq!(stream.active_count(), 2);
        
        stream.remove_viewer(viewer1);
        assert_eq!(stream.viewer_count(), 2);
    }
}
