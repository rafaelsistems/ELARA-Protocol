//! Time Engine Simulator - Full simulation harness for temporal testing
//!
//! Simulates:
//! - Multiple nodes with independent clocks
//! - Clock drift and skew
//! - Network-induced timing variations
//! - Reality window behavior under stress

use std::collections::HashMap;
use std::time::Duration;

use elara_core::{NodeId, PerceptualTime, RealityWindow, StateTime, TimePosition};
use elara_time::{TimeEngine, TimeEngineConfig};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::chaos::{ChaosConfig, ChaosNetwork};

/// Clock drift model for a simulated node
#[derive(Clone, Debug)]
pub struct ClockDriftModel {
    /// Drift rate (1.0 = perfect, >1.0 = fast, <1.0 = slow)
    pub drift_rate: f64,
    /// Random jitter per tick (microseconds)
    pub jitter_us: u32,
    /// Accumulated drift
    accumulated_drift: i64,
}

impl ClockDriftModel {
    pub fn new(drift_rate: f64, jitter_us: u32) -> Self {
        ClockDriftModel {
            drift_rate,
            jitter_us,
            accumulated_drift: 0,
        }
    }

    /// Perfect clock (no drift)
    pub fn perfect() -> Self {
        Self::new(1.0, 0)
    }

    /// Slightly fast clock
    pub fn fast() -> Self {
        Self::new(1.0001, 50)
    }

    /// Slightly slow clock
    pub fn slow() -> Self {
        Self::new(0.9999, 50)
    }

    /// Unstable clock with high jitter
    pub fn unstable() -> Self {
        Self::new(1.0, 500)
    }

    /// Apply drift to a duration
    pub fn apply(&mut self, dt: Duration, rng: &mut StdRng) -> Duration {
        let base_us = dt.as_micros() as f64;
        let drifted_us = base_us * self.drift_rate;
        let jitter = if self.jitter_us > 0 {
            rng.gen_range(-(self.jitter_us as i32)..=self.jitter_us as i32) as f64
        } else {
            0.0
        };
        let final_us = (drifted_us + jitter).max(0.0) as u64;
        self.accumulated_drift += final_us as i64 - dt.as_micros() as i64;
        Duration::from_micros(final_us)
    }

    /// Get accumulated drift
    pub fn accumulated_drift(&self) -> Duration {
        Duration::from_micros(self.accumulated_drift.unsigned_abs())
    }
}

/// Simulated node with time engine
pub struct SimulatedNode {
    /// Node ID
    pub node_id: NodeId,
    /// Time engine
    pub time_engine: TimeEngine,
    /// Clock drift model
    pub drift_model: ClockDriftModel,
    /// Local RNG
    rng: StdRng,
    /// Tick count
    tick_count: u64,
}

impl SimulatedNode {
    pub fn new(
        node_id: NodeId,
        config: TimeEngineConfig,
        drift: ClockDriftModel,
        seed: u64,
    ) -> Self {
        SimulatedNode {
            node_id,
            time_engine: TimeEngine::with_config(config),
            drift_model: drift,
            rng: StdRng::seed_from_u64(seed),
            tick_count: 0,
        }
    }

    /// Advance the node's time by one tick
    pub fn tick(&mut self, real_dt: Duration) {
        // Apply drift to get local perception of time
        let _local_dt = self.drift_model.apply(real_dt, &mut self.rng);

        // Advance time engine (it uses its own internal tick)
        self.time_engine.tick();
        self.tick_count += 1;
    }

    /// Get current perceptual time
    pub fn tau_p(&self) -> PerceptualTime {
        self.time_engine.tau_p()
    }

    /// Get current state time
    pub fn tau_s(&self) -> StateTime {
        self.time_engine.tau_s()
    }

    /// Get reality window
    pub fn reality_window(&self) -> RealityWindow {
        self.time_engine.reality_window()
    }

    /// Classify a time position
    pub fn classify(&self, t: StateTime) -> TimePosition {
        self.time_engine.classify_time(t)
    }

    /// Get tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
}

/// Time simulation scenario
pub struct TimeSimulator {
    /// Simulated nodes
    nodes: HashMap<NodeId, SimulatedNode>,
    /// Network links between nodes
    networks: HashMap<(NodeId, NodeId), ChaosNetwork>,
    /// Global simulation time
    global_time: Duration,
    /// Tick interval
    tick_interval: Duration,
    /// RNG seed counter
    seed_counter: u64,
}

impl TimeSimulator {
    /// Create a new time simulator
    pub fn new(tick_interval: Duration) -> Self {
        TimeSimulator {
            nodes: HashMap::new(),
            networks: HashMap::new(),
            global_time: Duration::ZERO,
            tick_interval,
            seed_counter: 0,
        }
    }

    /// Add a node with default configuration
    pub fn add_node(&mut self, node_id: NodeId) {
        self.add_node_with_drift(node_id, ClockDriftModel::perfect());
    }

    /// Add a node with specific drift model
    pub fn add_node_with_drift(&mut self, node_id: NodeId, drift: ClockDriftModel) {
        let seed = self.seed_counter;
        self.seed_counter += 1;

        let node = SimulatedNode::new(node_id, TimeEngineConfig::default(), drift, seed);
        self.nodes.insert(node_id, node);
    }

    /// Set network conditions between two nodes
    pub fn set_network(&mut self, from: NodeId, to: NodeId, config: ChaosConfig) {
        let seed = self.seed_counter;
        self.seed_counter += 1;
        self.networks
            .insert((from, to), ChaosNetwork::with_seed(config, seed));
    }

    /// Run simulation for a duration
    pub fn run(&mut self, duration: Duration) -> SimulationResult {
        let mut result = SimulationResult::new();
        let ticks = (duration.as_micros() / self.tick_interval.as_micros()) as u64;

        for _ in 0..ticks {
            self.tick(&mut result);
        }

        result
    }

    /// Execute one simulation tick
    fn tick(&mut self, result: &mut SimulationResult) {
        self.global_time += self.tick_interval;

        // Advance all nodes
        for node in self.nodes.values_mut() {
            node.tick(self.tick_interval);
        }

        // Record state
        result.record_tick(self);

        // Simulate time sync messages between nodes
        self.simulate_time_sync();
    }

    /// Simulate time synchronization between nodes
    fn simulate_time_sync(&mut self) {
        let node_ids: Vec<NodeId> = self.nodes.keys().copied().collect();

        for from in &node_ids {
            for to in &node_ids {
                if from == to {
                    continue;
                }

                // Get sender's state time
                let sender_time = self.nodes.get(from).map(|n| n.tau_s());

                if let Some(sender_time) = sender_time {
                    // Update receiver's network model
                    if let Some(receiver) = self.nodes.get_mut(to) {
                        receiver.time_engine.update_from_packet(
                            *from,
                            sender_time,
                            0, // seq
                        );
                    }
                }
            }
        }
    }

    /// Get a node
    pub fn node(&self, id: NodeId) -> Option<&SimulatedNode> {
        self.nodes.get(&id)
    }

    /// Get mutable node
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut SimulatedNode> {
        self.nodes.get_mut(&id)
    }

    /// Get global time
    pub fn global_time(&self) -> Duration {
        self.global_time
    }

    /// Get all node IDs
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }
}

/// Simulation result and statistics
#[derive(Debug, Default)]
pub struct SimulationResult {
    /// Total ticks executed
    pub total_ticks: u64,
    /// Maximum clock divergence observed (microseconds)
    pub max_divergence_us: i64,
    /// Average clock divergence (microseconds)
    pub avg_divergence_us: f64,
    /// Divergence samples
    divergence_samples: Vec<i64>,
    /// Horizon adaptation events
    pub horizon_changes: u32,
    /// Time position classifications
    pub classifications: HashMap<TimePosition, u64>,
}

impl SimulationResult {
    pub fn new() -> Self {
        SimulationResult::default()
    }

    fn record_tick(&mut self, sim: &TimeSimulator) {
        self.total_ticks += 1;

        // Calculate divergence between all node pairs
        let nodes: Vec<_> = sim.nodes.values().collect();
        if nodes.len() >= 2 {
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    let t1 = nodes[i].tau_s().as_micros();
                    let t2 = nodes[j].tau_s().as_micros();
                    let divergence = (t1 - t2).abs();

                    self.divergence_samples.push(divergence);
                    self.max_divergence_us = self.max_divergence_us.max(divergence);
                }
            }
        }
    }

    /// Calculate final statistics
    pub fn finalize(&mut self) {
        if !self.divergence_samples.is_empty() {
            let sum: i64 = self.divergence_samples.iter().sum();
            self.avg_divergence_us = sum as f64 / self.divergence_samples.len() as f64;
        }
    }

    /// Get divergence in milliseconds
    pub fn max_divergence_ms(&self) -> f64 {
        self.max_divergence_us as f64 / 1000.0
    }

    /// Get average divergence in milliseconds
    pub fn avg_divergence_ms(&self) -> f64 {
        self.avg_divergence_us / 1000.0
    }
}

/// Predefined test scenarios
pub mod scenarios {
    use super::*;

    /// Two nodes with perfect clocks
    pub fn perfect_pair() -> TimeSimulator {
        let mut sim = TimeSimulator::new(Duration::from_millis(10));
        sim.add_node(NodeId::new(1));
        sim.add_node(NodeId::new(2));
        sim
    }

    /// Two nodes with drifting clocks
    pub fn drifting_pair() -> TimeSimulator {
        let mut sim = TimeSimulator::new(Duration::from_millis(10));
        sim.add_node_with_drift(NodeId::new(1), ClockDriftModel::fast());
        sim.add_node_with_drift(NodeId::new(2), ClockDriftModel::slow());
        sim
    }

    /// Small swarm with varying clock quality
    pub fn small_swarm(count: usize) -> TimeSimulator {
        let mut sim = TimeSimulator::new(Duration::from_millis(10));

        for i in 0..count {
            let drift = match i % 4 {
                0 => ClockDriftModel::perfect(),
                1 => ClockDriftModel::fast(),
                2 => ClockDriftModel::slow(),
                _ => ClockDriftModel::unstable(),
            };
            sim.add_node_with_drift(NodeId::new(i as u64), drift);
        }

        sim
    }

    /// Hostile network scenario
    pub fn hostile_network() -> TimeSimulator {
        let mut sim = TimeSimulator::new(Duration::from_millis(10));

        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        sim.add_node_with_drift(node1, ClockDriftModel::unstable());
        sim.add_node_with_drift(node2, ClockDriftModel::unstable());

        sim.set_network(node1, node2, ChaosConfig::hostile());
        sim.set_network(node2, node1, ChaosConfig::hostile());

        sim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_clocks() {
        let mut sim = scenarios::perfect_pair();
        let mut result = sim.run(Duration::from_secs(10));
        result.finalize();

        println!(
            "Perfect clocks - Max divergence: {:.3}ms",
            result.max_divergence_ms()
        );
        // Perfect clocks should have minimal divergence
        assert!(result.max_divergence_ms() < 100.0);
    }

    #[test]
    fn test_drifting_clocks() {
        let mut sim = scenarios::drifting_pair();
        let mut result = sim.run(Duration::from_secs(60));
        result.finalize();

        println!(
            "Drifting clocks - Max divergence: {:.3}ms",
            result.max_divergence_ms()
        );
        // Drifting clocks will diverge but time engine should limit it
    }

    #[test]
    fn test_small_swarm() {
        let mut sim = scenarios::small_swarm(5);
        let mut result = sim.run(Duration::from_secs(30));
        result.finalize();

        println!(
            "Small swarm - Max divergence: {:.3}ms, Avg: {:.3}ms",
            result.max_divergence_ms(),
            result.avg_divergence_ms()
        );
    }

    #[test]
    fn test_clock_drift_model() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut drift = ClockDriftModel::fast();

        // Apply 1000 ticks of 10ms each
        for _ in 0..1000 {
            drift.apply(Duration::from_millis(10), &mut rng);
        }

        // Fast clock should have positive accumulated drift
        println!("Accumulated drift: {:?}", drift.accumulated_drift());
        assert!(drift.accumulated_drift() > Duration::ZERO);
    }

    #[test]
    fn test_reality_window_classification() {
        let mut sim = scenarios::perfect_pair();
        sim.run(Duration::from_secs(1));

        let node = sim.node(NodeId::new(1)).unwrap();
        let rw = node.reality_window();

        // Current time should be in Current position
        let current = node.tau_s();
        assert!(rw.contains(current));
        assert_eq!(node.classify(current), TimePosition::Current);

        // Far past should be TooLate
        let past = StateTime::from_millis(current.as_millis() - 10000);
        assert_eq!(node.classify(past), TimePosition::TooLate);

        // Far future should be TooEarly
        let future = StateTime::from_millis(current.as_millis() + 10000);
        assert_eq!(node.classify(future), TimePosition::TooEarly);
    }
}
