//! Swarm - Complete diffusion system for livestream/group
//!
//! Combines authority, interest, topology, and propagation.

use elara_core::{NodeId, StateTime};
use std::collections::HashMap;

use crate::{
    AuthoritySet, InterestDeclaration, InterestLevel, InterestMap,
    LivestreamAuthority, LivestreamInterest, PropagationScheduler,
    PropagationTopology, StarTopology, TreeTopology, StateUpdate,
};

/// Swarm configuration
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Maximum viewers before switching to tree topology
    pub star_to_tree_threshold: usize,
    /// Maximum fan-out for tree topology
    pub tree_fanout: usize,
    /// Bandwidth budget per viewer (bytes/second)
    pub bandwidth_per_viewer: u32,
    /// Keyframe interval in milliseconds
    pub keyframe_interval_ms: u32,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            star_to_tree_threshold: 50,
            tree_fanout: 5,
            bandwidth_per_viewer: 500_000, // 500 KB/s
            keyframe_interval_ms: 2000,
        }
    }
}

/// Swarm state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmState {
    /// Initializing
    Initializing,
    /// Active and streaming
    Active,
    /// Paused (broadcaster paused)
    Paused,
    /// Ended
    Ended,
}

/// Livestream swarm - complete system for a single livestream
#[derive(Debug)]
pub struct LivestreamSwarm {
    /// Stream ID
    pub stream_id: u64,
    /// Configuration
    pub config: SwarmConfig,
    /// Current state
    pub state: SwarmState,
    /// Authority model
    pub authority: LivestreamAuthority,
    /// Interest tracking
    pub interest: LivestreamInterest,
    /// Current topology (star or tree)
    topology: SwarmTopology,
    /// Last keyframe time
    last_keyframe: StateTime,
    /// Sequence counter
    sequence: u64,
    /// Statistics
    pub stats: SwarmStats,
}

/// Topology type
#[derive(Debug)]
enum SwarmTopology {
    Star(StarTopology),
    Tree(TreeTopology),
}

impl LivestreamSwarm {
    /// Create a new livestream swarm
    pub fn new(stream_id: u64, broadcaster: NodeId, config: SwarmConfig) -> Self {
        Self {
            stream_id,
            config: config.clone(),
            state: SwarmState::Initializing,
            authority: LivestreamAuthority::new(broadcaster, stream_id),
            interest: LivestreamInterest::new(stream_id),
            topology: SwarmTopology::Star(StarTopology::new(broadcaster)),
            last_keyframe: StateTime::from_millis(0),
            sequence: 0,
            stats: SwarmStats::new(),
        }
    }
    
    /// Start the stream
    pub fn start(&mut self) {
        self.state = SwarmState::Active;
    }
    
    /// Pause the stream
    pub fn pause(&mut self) {
        self.state = SwarmState::Paused;
    }
    
    /// Resume the stream
    pub fn resume(&mut self) {
        self.state = SwarmState::Active;
    }
    
    /// End the stream
    pub fn end(&mut self) {
        self.state = SwarmState::Ended;
    }
    
    /// Add a viewer
    pub fn add_viewer(&mut self, viewer: NodeId) {
        self.interest.add_viewer(viewer);
        
        match &mut self.topology {
            SwarmTopology::Star(star) => {
                star.add_leaf(viewer);
                
                // Check if we need to switch to tree
                if star.leaf_count() > self.config.star_to_tree_threshold {
                    self.switch_to_tree();
                }
            }
            SwarmTopology::Tree(tree) => {
                tree.add_node(viewer);
            }
        }
        
        self.stats.peak_viewers = self.stats.peak_viewers.max(self.viewer_count() as u32);
    }
    
    /// Remove a viewer
    pub fn remove_viewer(&mut self, viewer: NodeId) {
        self.interest.remove_viewer(viewer);
        
        match &mut self.topology {
            SwarmTopology::Star(star) => {
                star.remove_leaf(viewer);
            }
            SwarmTopology::Tree(tree) => {
                tree.remove_node(viewer);
            }
        }
    }
    
    /// Switch from star to tree topology
    fn switch_to_tree(&mut self) {
        if let SwarmTopology::Star(star) = &self.topology {
            let mut tree = TreeTopology::new(star.center, self.config.tree_fanout);
            
            // Add all existing leaves
            for &leaf in &star.leaves {
                tree.add_node(leaf);
            }
            
            self.topology = SwarmTopology::Tree(tree);
        }
    }
    
    /// Get viewer count
    pub fn viewer_count(&self) -> usize {
        self.interest.viewer_count()
    }
    
    /// Get broadcaster
    pub fn broadcaster(&self) -> NodeId {
        self.authority.broadcaster
    }
    
    /// Check if a node can broadcast
    pub fn can_broadcast(&self, node: NodeId) -> bool {
        self.authority.can_mutate_visual(node)
    }
    
    /// Create a state update for broadcasting
    pub fn create_update(&mut self, timestamp: StateTime, size: usize, is_keyframe: bool) -> StateUpdate {
        self.sequence += 1;
        
        let mut update = StateUpdate::new(
            self.stream_id,
            self.broadcaster(),
            self.sequence,
            timestamp,
        ).with_size(size);
        
        if is_keyframe {
            update = update.keyframe();
            self.last_keyframe = timestamp;
        }
        
        update
    }
    
    /// Check if we need a keyframe
    pub fn needs_keyframe(&self, current_time: StateTime) -> bool {
        let elapsed = current_time.as_millis() - self.last_keyframe.as_millis();
        elapsed >= self.config.keyframe_interval_ms as i64
    }
    
    /// Get propagation targets for an update
    pub fn get_targets(&self) -> Vec<NodeId> {
        match &self.topology {
            SwarmTopology::Star(star) => star.leaves.iter().copied().collect(),
            SwarmTopology::Tree(tree) => tree.topology.downstream(self.broadcaster()),
        }
    }
}

/// Swarm statistics
#[derive(Debug, Clone, Default)]
pub struct SwarmStats {
    /// Total updates sent
    pub updates_sent: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Peak viewer count
    pub peak_viewers: u32,
    /// Total unique viewers
    pub total_viewers: u32,
    /// Stream duration in seconds
    pub duration_seconds: u32,
}

impl SwarmStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Group swarm - for video calls (symmetric authority)
#[derive(Debug)]
pub struct GroupSwarm {
    /// Group ID
    pub group_id: u64,
    /// All participants
    participants: HashMap<NodeId, ParticipantState>,
    /// Interest map
    interests: InterestMap,
    /// Topology (mesh for small groups)
    topology: PropagationTopology,
    /// Maximum participants
    pub max_participants: usize,
}

/// Participant state in a group
#[derive(Debug, Clone)]
pub struct ParticipantState {
    /// Node ID
    pub node: NodeId,
    /// Is video enabled?
    pub video_enabled: bool,
    /// Is audio enabled?
    pub audio_enabled: bool,
    /// Is screen sharing?
    pub screen_sharing: bool,
    /// Join time
    pub joined_at: StateTime,
}

impl GroupSwarm {
    /// Create a new group swarm
    pub fn new(group_id: u64, max_participants: usize) -> Self {
        Self {
            group_id,
            participants: HashMap::new(),
            interests: InterestMap::new(),
            topology: PropagationTopology::new(),
            max_participants,
        }
    }
    
    /// Add a participant
    pub fn add_participant(&mut self, node: NodeId, joined_at: StateTime) -> bool {
        if self.participants.len() >= self.max_participants {
            return false;
        }
        
        let state = ParticipantState {
            node,
            video_enabled: true,
            audio_enabled: true,
            screen_sharing: false,
            joined_at,
        };
        
        // Add edges to/from all existing participants (mesh)
        for &existing in self.participants.keys() {
            self.topology.add_edge(crate::PropagationEdge::new(existing, node));
            self.topology.add_edge(crate::PropagationEdge::new(node, existing));
        }
        
        // Register interest in all other participants' states
        for &existing in self.participants.keys() {
            self.interests.register(InterestDeclaration::new(
                node, existing.0, InterestLevel::High
            ));
            self.interests.register(InterestDeclaration::new(
                existing, node.0, InterestLevel::High
            ));
        }
        
        self.participants.insert(node, state);
        self.topology.add_node(node);
        
        true
    }
    
    /// Remove a participant
    pub fn remove_participant(&mut self, node: NodeId) {
        self.participants.remove(&node);
        self.topology.remove_node(node);
        self.interests.remove_node(node);
    }
    
    /// Get participant count
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }
    
    /// Toggle video for a participant
    pub fn toggle_video(&mut self, node: NodeId, enabled: bool) {
        if let Some(p) = self.participants.get_mut(&node) {
            p.video_enabled = enabled;
        }
    }
    
    /// Toggle audio for a participant
    pub fn toggle_audio(&mut self, node: NodeId, enabled: bool) {
        if let Some(p) = self.participants.get_mut(&node) {
            p.audio_enabled = enabled;
        }
    }
    
    /// Start screen sharing
    pub fn start_screen_share(&mut self, node: NodeId) -> bool {
        // Only one person can screen share at a time
        if self.participants.values().any(|p| p.screen_sharing) {
            return false;
        }
        
        if let Some(p) = self.participants.get_mut(&node) {
            p.screen_sharing = true;
            return true;
        }
        
        false
    }
    
    /// Stop screen sharing
    pub fn stop_screen_share(&mut self, node: NodeId) {
        if let Some(p) = self.participants.get_mut(&node) {
            p.screen_sharing = false;
        }
    }
    
    /// Get all participants
    pub fn participants(&self) -> Vec<&ParticipantState> {
        self.participants.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_livestream_swarm() {
        let broadcaster = NodeId::new(1);
        let mut swarm = LivestreamSwarm::new(1000, broadcaster, SwarmConfig::default());
        
        swarm.start();
        assert_eq!(swarm.state, SwarmState::Active);
        
        // Add viewers
        for i in 2..=10 {
            swarm.add_viewer(NodeId::new(i));
        }
        
        assert_eq!(swarm.viewer_count(), 9);
        assert!(swarm.can_broadcast(broadcaster));
        assert!(!swarm.can_broadcast(NodeId::new(2)));
    }
    
    #[test]
    fn test_livestream_topology_switch() {
        let broadcaster = NodeId::new(1);
        let config = SwarmConfig {
            star_to_tree_threshold: 5,
            ..Default::default()
        };
        let mut swarm = LivestreamSwarm::new(1000, broadcaster, config);
        
        // Add viewers until we switch to tree
        for i in 2..=10 {
            swarm.add_viewer(NodeId::new(i));
        }
        
        // Should have switched to tree
        assert!(matches!(swarm.topology, SwarmTopology::Tree(_)));
    }
    
    #[test]
    fn test_group_swarm() {
        let mut group = GroupSwarm::new(2000, 10);
        
        let time = StateTime::from_millis(0);
        
        assert!(group.add_participant(NodeId::new(1), time));
        assert!(group.add_participant(NodeId::new(2), time));
        assert!(group.add_participant(NodeId::new(3), time));
        
        assert_eq!(group.participant_count(), 3);
        
        // Test screen sharing
        assert!(group.start_screen_share(NodeId::new(1)));
        assert!(!group.start_screen_share(NodeId::new(2))); // Already sharing
        
        group.stop_screen_share(NodeId::new(1));
        assert!(group.start_screen_share(NodeId::new(2))); // Now allowed
    }
}
