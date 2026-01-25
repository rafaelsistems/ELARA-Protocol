//! Network simulator for ELARA protocol testing

use std::collections::HashMap;
use std::time::Duration;

use elara_core::NodeId;

use crate::chaos::{ChaosConfig, ChaosNetwork};

/// Simulated network link between two nodes
pub struct NetworkLink {
    /// Source node
    pub from: NodeId,
    /// Destination node
    pub to: NodeId,
    /// Chaos network for this link
    pub network: ChaosNetwork,
}

/// Network simulator for multi-node testing
pub struct NetworkSimulator {
    /// Links between nodes (keyed by (from, to))
    links: HashMap<(NodeId, NodeId), ChaosNetwork>,
    /// Default chaos config for new links
    default_config: ChaosConfig,
    /// Current simulation time
    current_time: Duration,
    /// RNG seed counter
    seed_counter: u64,
}

impl NetworkSimulator {
    /// Create a new network simulator
    pub fn new(default_config: ChaosConfig) -> Self {
        NetworkSimulator {
            links: HashMap::new(),
            default_config,
            current_time: Duration::ZERO,
            seed_counter: 0,
        }
    }

    /// Create with good network conditions
    pub fn good() -> Self {
        Self::new(ChaosConfig::good())
    }

    /// Create with poor network conditions
    pub fn poor() -> Self {
        Self::new(ChaosConfig::poor())
    }

    /// Create with hostile network conditions
    pub fn hostile() -> Self {
        Self::new(ChaosConfig::hostile())
    }

    /// Get or create a link between two nodes
    fn get_or_create_link(&mut self, from: NodeId, to: NodeId) -> &mut ChaosNetwork {
        let seed = self.seed_counter;
        self.seed_counter += 1;

        self.links
            .entry((from, to))
            .or_insert_with(|| ChaosNetwork::with_seed(self.default_config.clone(), seed))
    }

    /// Send a packet from one node to another
    pub fn send(&mut self, from: NodeId, to: NodeId, data: Vec<u8>) {
        let link = self.get_or_create_link(from, to);
        link.send(data);
    }

    /// Advance simulation time and collect delivered packets
    pub fn tick(&mut self, dt: Duration) -> Vec<(NodeId, NodeId, Vec<u8>)> {
        self.current_time += dt;

        let mut delivered = Vec::new();

        for ((from, to), link) in &mut self.links {
            let packets = link.tick(dt);
            for data in packets {
                delivered.push((*from, *to, data));
            }
        }

        delivered
    }

    /// Get current simulation time
    pub fn current_time(&self) -> Duration {
        self.current_time
    }

    /// Set custom config for a specific link
    pub fn set_link_config(&mut self, from: NodeId, to: NodeId, config: ChaosConfig) {
        let seed = self.seed_counter;
        self.seed_counter += 1;
        self.links
            .insert((from, to), ChaosNetwork::with_seed(config, seed));
    }

    /// Get statistics for a link
    pub fn link_stats(&self, from: NodeId, to: NodeId) -> Option<&crate::chaos::ChaosStats> {
        self.links.get(&(from, to)).map(|l| l.stats())
    }

    /// Get all link statistics
    pub fn all_stats(&self) -> Vec<((NodeId, NodeId), &crate::chaos::ChaosStats)> {
        self.links.iter().map(|(k, v)| (*k, v.stats())).collect()
    }
}

/// Test scenario builder
pub struct ScenarioBuilder {
    nodes: Vec<NodeId>,
    config: ChaosConfig,
    duration: Duration,
    tick_interval: Duration,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        ScenarioBuilder {
            nodes: Vec::new(),
            config: ChaosConfig::default(),
            duration: Duration::from_secs(60),
            tick_interval: Duration::from_millis(10),
        }
    }

    /// Add nodes to the scenario
    pub fn with_nodes(mut self, count: usize) -> Self {
        self.nodes = (0..count).map(|i| NodeId::new(i as u64)).collect();
        self
    }

    /// Set network conditions
    pub fn with_config(mut self, config: ChaosConfig) -> Self {
        self.config = config;
        self
    }

    /// Set test duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set tick interval
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = interval;
        self
    }

    /// Build the simulator
    pub fn build(self) -> (NetworkSimulator, Vec<NodeId>) {
        (NetworkSimulator::new(self.config), self.nodes)
    }

    /// Get duration
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Get tick interval
    pub fn tick_interval(&self) -> Duration {
        self.tick_interval
    }
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_simulator_basic() {
        let mut sim = NetworkSimulator::good();

        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // Send packets both ways
        for i in 0..10 {
            sim.send(node1, node2, vec![i]);
            sim.send(node2, node1, vec![i + 100]);
        }

        // Advance time
        let mut total_delivered = 0;
        for _ in 0..100 {
            let delivered = sim.tick(Duration::from_millis(10));
            total_delivered += delivered.len();
        }

        // Most should be delivered
        assert!(total_delivered >= 18);
    }

    #[test]
    fn test_scenario_builder() {
        let (mut sim, nodes) = ScenarioBuilder::new()
            .with_nodes(5)
            .with_config(ChaosConfig::poor())
            .with_duration(Duration::from_secs(10))
            .build();

        assert_eq!(nodes.len(), 5);

        // Send between all pairs
        for from in &nodes {
            for to in &nodes {
                if from != to {
                    sim.send(*from, *to, vec![1, 2, 3]);
                }
            }
        }

        // Advance and collect
        for _ in 0..200 {
            sim.tick(Duration::from_millis(10));
        }

        // Check stats
        let stats = sim.all_stats();
        assert!(!stats.is_empty());
    }
}
