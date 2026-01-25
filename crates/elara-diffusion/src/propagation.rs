//! Propagation - How state flows through the network
//!
//! State propagation rules and scheduling.

use crate::{InterestLevel, InterestMap, PropagationTopology};
use elara_core::{NodeId, StateTime};

/// Propagation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PropagationPriority {
    /// Background - send when bandwidth available
    Background = 0,
    /// Normal - regular priority
    Normal = 1,
    /// High - prioritize over normal
    High = 2,
    /// Urgent - send immediately
    Urgent = 3,
}

/// State update to propagate
#[derive(Debug, Clone)]
pub struct StateUpdate {
    /// State ID
    pub state_id: u64,
    /// Source node (authority)
    pub source: NodeId,
    /// Sequence number
    pub sequence: u64,
    /// Timestamp
    pub timestamp: StateTime,
    /// Priority
    pub priority: PropagationPriority,
    /// Payload size in bytes
    pub size: usize,
    /// Is this a keyframe?
    pub is_keyframe: bool,
}

impl StateUpdate {
    pub fn new(state_id: u64, source: NodeId, sequence: u64, timestamp: StateTime) -> Self {
        Self {
            state_id,
            source,
            sequence,
            timestamp,
            priority: PropagationPriority::Normal,
            size: 0,
            is_keyframe: false,
        }
    }

    pub fn with_priority(mut self, priority: PropagationPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub fn keyframe(mut self) -> Self {
        self.is_keyframe = true;
        self
    }
}

/// Propagation decision for a specific node
#[derive(Debug, Clone)]
pub struct PropagationDecision {
    /// Target node
    pub target: NodeId,
    /// Should we send to this node?
    pub should_send: bool,
    /// Priority for this target
    pub priority: PropagationPriority,
    /// Delay before sending (for rate limiting)
    pub delay_ms: u32,
    /// Quality level (for degradation)
    pub quality_level: u8,
}

/// Propagation scheduler
#[derive(Debug)]
pub struct PropagationScheduler {
    /// Interest map
    interests: InterestMap,
    /// Topology
    topology: PropagationTopology,
    /// Bandwidth budget per node (bytes per second)
    bandwidth_budget: u32,
    /// Current bandwidth usage per node
    bandwidth_usage: std::collections::HashMap<NodeId, u32>,
}

impl PropagationScheduler {
    /// Create a new scheduler
    pub fn new(interests: InterestMap, topology: PropagationTopology) -> Self {
        Self {
            interests,
            topology,
            bandwidth_budget: 1_000_000, // 1 MB/s default
            bandwidth_usage: std::collections::HashMap::new(),
        }
    }

    /// Set bandwidth budget
    pub fn set_bandwidth_budget(&mut self, bytes_per_second: u32) {
        self.bandwidth_budget = bytes_per_second;
    }

    /// Decide how to propagate an update
    pub fn schedule(&self, update: &StateUpdate) -> Vec<PropagationDecision> {
        let mut decisions = Vec::new();

        // Get all interested nodes
        let interested = self.interests.interested_nodes(update.state_id);

        for (node, interest_level) in interested {
            // Skip the source
            if node == update.source {
                continue;
            }

            // Check if node is reachable in topology
            if !self.topology.has_node(node) {
                continue;
            }

            // Determine priority based on interest level
            let priority = match interest_level {
                InterestLevel::Critical => PropagationPriority::Urgent,
                InterestLevel::High => PropagationPriority::High,
                InterestLevel::Medium => PropagationPriority::Normal,
                InterestLevel::Low => PropagationPriority::Background,
                InterestLevel::None => continue,
            };

            // Determine quality level based on interest
            let quality_level = match interest_level {
                InterestLevel::Critical | InterestLevel::High => 0, // Full quality
                InterestLevel::Medium => 1,                         // Slight reduction
                InterestLevel::Low => 2,                            // Significant reduction
                InterestLevel::None => continue,
            };

            // Calculate delay based on priority
            let delay_ms = match priority {
                PropagationPriority::Urgent => 0,
                PropagationPriority::High => 10,
                PropagationPriority::Normal => 50,
                PropagationPriority::Background => 200,
            };

            decisions.push(PropagationDecision {
                target: node,
                should_send: true,
                priority,
                delay_ms,
                quality_level,
            });
        }

        // Sort by priority (highest first)
        decisions.sort_by(|a, b| b.priority.cmp(&a.priority));

        decisions
    }

    /// Update bandwidth usage
    pub fn record_send(&mut self, target: NodeId, bytes: u32) {
        *self.bandwidth_usage.entry(target).or_insert(0) += bytes;
    }

    /// Reset bandwidth usage (call periodically)
    pub fn reset_bandwidth(&mut self) {
        self.bandwidth_usage.clear();
    }

    /// Check if we have bandwidth for a send
    pub fn has_bandwidth(&self, target: NodeId, bytes: u32) -> bool {
        let used = self.bandwidth_usage.get(&target).copied().unwrap_or(0);
        used + bytes <= self.bandwidth_budget
    }
}

/// Propagation statistics
#[derive(Debug, Clone, Default)]
pub struct PropagationStats {
    /// Total updates sent
    pub updates_sent: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Updates dropped (bandwidth limit)
    pub updates_dropped: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f32,
    /// Peak latency in milliseconds
    pub peak_latency_ms: u32,
}

impl PropagationStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_send(&mut self, bytes: u64, latency_ms: u32) {
        self.updates_sent += 1;
        self.bytes_sent += bytes;

        // Update average latency
        let n = self.updates_sent as f32;
        self.avg_latency_ms = ((n - 1.0) * self.avg_latency_ms + latency_ms as f32) / n;

        if latency_ms > self.peak_latency_ms {
            self.peak_latency_ms = latency_ms;
        }
    }

    pub fn record_drop(&mut self) {
        self.updates_dropped += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InterestDeclaration;

    #[test]
    fn test_propagation_scheduler() {
        let mut interests = InterestMap::new();
        let source = NodeId::new(1);
        let viewer1 = NodeId::new(2);
        let viewer2 = NodeId::new(3);

        interests.register(InterestDeclaration::new(viewer1, 100, InterestLevel::High));
        interests.register(InterestDeclaration::new(viewer2, 100, InterestLevel::Low));

        let mut topology = PropagationTopology::new();
        topology.add_node(source);
        topology.add_node(viewer1);
        topology.add_node(viewer2);

        let scheduler = PropagationScheduler::new(interests, topology);

        let update = StateUpdate::new(100, source, 1, StateTime::from_millis(0));
        let decisions = scheduler.schedule(&update);

        assert_eq!(decisions.len(), 2);
        // High interest should be first
        assert_eq!(decisions[0].target, viewer1);
        assert_eq!(decisions[0].priority, PropagationPriority::High);
    }

    #[test]
    fn test_propagation_stats() {
        let mut stats = PropagationStats::new();

        stats.record_send(1000, 50);
        stats.record_send(1000, 100);
        stats.record_send(1000, 75);

        assert_eq!(stats.updates_sent, 3);
        assert_eq!(stats.bytes_sent, 3000);
        assert_eq!(stats.peak_latency_ms, 100);
    }
}
