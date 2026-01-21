//! Chaos testing for ELARA protocol
//!
//! Simulates hostile network conditions:
//! - Jitter
//! - Packet loss
//! - Reordering
//! - Duplication

use std::collections::VecDeque;
use std::time::Duration;

use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Jitter distribution type
#[derive(Clone, Debug)]
pub enum JitterDistribution {
    /// Uniform distribution
    Uniform { min_ms: u32, max_ms: u32 },
    /// Normal distribution (mean, stddev)
    Normal { mean_ms: f64, stddev_ms: f64 },
    /// Pareto distribution (heavy tail)
    Pareto { scale_ms: f64, shape: f64 },
}

impl JitterDistribution {
    /// Sample a jitter value
    pub fn sample(&self, rng: &mut StdRng) -> Duration {
        match self {
            JitterDistribution::Uniform { min_ms, max_ms } => {
                let dist = Uniform::new(*min_ms, *max_ms);
                Duration::from_millis(dist.sample(rng) as u64)
            }
            JitterDistribution::Normal { mean_ms, stddev_ms } => {
                // Box-Muller transform for normal distribution
                let u1: f64 = rng.gen();
                let u2: f64 = rng.gen();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let value = mean_ms + stddev_ms * z;
                Duration::from_millis(value.max(0.0) as u64)
            }
            JitterDistribution::Pareto { scale_ms, shape } => {
                let u: f64 = rng.gen();
                let value = scale_ms / u.powf(1.0 / shape);
                Duration::from_millis(value.min(1000.0) as u64) // Cap at 1 second
            }
        }
    }
}

/// Network chaos configuration
#[derive(Clone, Debug)]
pub struct ChaosConfig {
    /// Base latency
    pub base_latency: Duration,
    /// Jitter distribution
    pub jitter: JitterDistribution,
    /// Packet loss rate (0.0 - 1.0)
    pub loss_rate: f64,
    /// Burst loss probability
    pub burst_loss_prob: f64,
    /// Burst loss length range
    pub burst_length: (u32, u32),
    /// Reorder probability
    pub reorder_prob: f64,
    /// Reorder depth (max packets to reorder)
    pub reorder_depth: u32,
    /// Duplicate probability
    pub duplicate_prob: f64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        ChaosConfig {
            base_latency: Duration::from_millis(50),
            jitter: JitterDistribution::Uniform {
                min_ms: 0,
                max_ms: 50,
            },
            loss_rate: 0.01,
            burst_loss_prob: 0.1,
            burst_length: (2, 5),
            reorder_prob: 0.05,
            reorder_depth: 3,
            duplicate_prob: 0.01,
        }
    }
}

impl ChaosConfig {
    /// Good network conditions
    pub fn good() -> Self {
        ChaosConfig {
            base_latency: Duration::from_millis(20),
            jitter: JitterDistribution::Uniform {
                min_ms: 0,
                max_ms: 10,
            },
            loss_rate: 0.001,
            burst_loss_prob: 0.01,
            burst_length: (1, 2),
            reorder_prob: 0.01,
            reorder_depth: 2,
            duplicate_prob: 0.001,
        }
    }

    /// Poor network conditions
    pub fn poor() -> Self {
        ChaosConfig {
            base_latency: Duration::from_millis(100),
            jitter: JitterDistribution::Pareto {
                scale_ms: 50.0,
                shape: 1.5,
            },
            loss_rate: 0.05,
            burst_loss_prob: 0.2,
            burst_length: (3, 8),
            reorder_prob: 0.1,
            reorder_depth: 5,
            duplicate_prob: 0.02,
        }
    }

    /// Hostile network conditions (2G-class)
    pub fn hostile() -> Self {
        ChaosConfig {
            base_latency: Duration::from_millis(200),
            jitter: JitterDistribution::Pareto {
                scale_ms: 100.0,
                shape: 1.2,
            },
            loss_rate: 0.15,
            burst_loss_prob: 0.3,
            burst_length: (5, 15),
            reorder_prob: 0.2,
            reorder_depth: 10,
            duplicate_prob: 0.05,
        }
    }
}

/// Packet in the chaos network
#[derive(Clone, Debug)]
pub struct ChaosPacket {
    /// Packet data
    pub data: Vec<u8>,
    /// Scheduled delivery time (relative to start)
    pub delivery_time: Duration,
    /// Original send time
    pub send_time: Duration,
    /// Sequence number (for tracking)
    pub seq: u64,
}

/// Chaos network simulator
pub struct ChaosNetwork {
    config: ChaosConfig,
    rng: StdRng,
    /// Packets in flight
    in_flight: VecDeque<ChaosPacket>,
    /// Current time
    current_time: Duration,
    /// Burst loss counter
    burst_remaining: u32,
    /// Sequence counter
    next_seq: u64,
    /// Statistics
    stats: ChaosStats,
}

/// Chaos network statistics
#[derive(Clone, Debug, Default)]
pub struct ChaosStats {
    pub packets_sent: u64,
    pub packets_delivered: u64,
    pub packets_lost: u64,
    pub packets_reordered: u64,
    pub packets_duplicated: u64,
    pub total_latency_ms: u64,
    pub max_latency_ms: u64,
}

impl ChaosStats {
    pub fn loss_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            0.0
        } else {
            self.packets_lost as f64 / self.packets_sent as f64
        }
    }

    pub fn avg_latency_ms(&self) -> f64 {
        if self.packets_delivered == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.packets_delivered as f64
        }
    }
}

impl ChaosNetwork {
    /// Create a new chaos network with seed
    pub fn new(config: ChaosConfig, seed: u64) -> Self {
        ChaosNetwork {
            config,
            rng: StdRng::seed_from_u64(seed),
            in_flight: VecDeque::new(),
            current_time: Duration::ZERO,
            burst_remaining: 0,
            next_seq: 0,
            stats: ChaosStats::default(),
        }
    }

    /// Send a packet into the chaos network
    pub fn send(&mut self, data: Vec<u8>) {
        self.stats.packets_sent += 1;
        let seq = self.next_seq;
        self.next_seq += 1;

        // Check for loss
        if self.should_drop() {
            self.stats.packets_lost += 1;
            return;
        }

        // Calculate delivery time
        let jitter = self.config.jitter.sample(&mut self.rng);
        let latency = self.config.base_latency + jitter;
        let delivery_time = self.current_time + latency;

        let packet = ChaosPacket {
            data: data.clone(),
            delivery_time,
            send_time: self.current_time,
            seq,
        };

        // Check for reorder
        if self.rng.gen::<f64>() < self.config.reorder_prob && !self.in_flight.is_empty() {
            // Insert at random position within reorder depth
            let depth = self.config.reorder_depth.min(self.in_flight.len() as u32);
            let pos = self.rng.gen_range(0..=depth) as usize;
            let insert_pos = self.in_flight.len().saturating_sub(pos);
            self.in_flight.insert(insert_pos, packet.clone());
            self.stats.packets_reordered += 1;
        } else {
            self.in_flight.push_back(packet.clone());
        }

        // Check for duplicate
        if self.rng.gen::<f64>() < self.config.duplicate_prob {
            let dup_jitter = self.config.jitter.sample(&mut self.rng);
            let dup_packet = ChaosPacket {
                data,
                delivery_time: delivery_time + dup_jitter,
                send_time: self.current_time,
                seq,
            };
            self.in_flight.push_back(dup_packet);
            self.stats.packets_duplicated += 1;
        }
    }

    /// Check if packet should be dropped
    fn should_drop(&mut self) -> bool {
        // Burst loss
        if self.burst_remaining > 0 {
            self.burst_remaining -= 1;
            return true;
        }

        // Start new burst?
        if self.rng.gen::<f64>() < self.config.burst_loss_prob {
            let (min, max) = self.config.burst_length;
            self.burst_remaining = self.rng.gen_range(min..=max);
            return true;
        }

        // Random loss
        self.rng.gen::<f64>() < self.config.loss_rate
    }

    /// Advance time and receive delivered packets
    pub fn tick(&mut self, dt: Duration) -> Vec<Vec<u8>> {
        self.current_time += dt;

        let mut delivered = Vec::new();

        // Collect packets that should be delivered
        while let Some(packet) = self.in_flight.front() {
            if packet.delivery_time <= self.current_time {
                let packet = self.in_flight.pop_front().unwrap();
                let latency = (packet.delivery_time - packet.send_time).as_millis() as u64;

                self.stats.packets_delivered += 1;
                self.stats.total_latency_ms += latency;
                self.stats.max_latency_ms = self.stats.max_latency_ms.max(latency);

                delivered.push(packet.data);
            } else {
                break;
            }
        }

        delivered
    }

    /// Get current statistics
    pub fn stats(&self) -> &ChaosStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = ChaosStats::default();
    }

    /// Get current time
    pub fn current_time(&self) -> Duration {
        self.current_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_network_basic() {
        let config = ChaosConfig::good();
        let mut network = ChaosNetwork::new(config, 12345);

        // Send some packets
        for i in 0..100 {
            network.send(vec![i as u8]);
        }

        // Advance time to deliver
        let mut delivered = 0;
        for _ in 0..100 {
            let packets = network.tick(Duration::from_millis(10));
            delivered += packets.len();
        }

        // Most should be delivered
        assert!(delivered > 90);
        println!("Stats: {:?}", network.stats());
    }

    #[test]
    fn test_chaos_network_hostile() {
        let config = ChaosConfig::hostile();
        let mut network = ChaosNetwork::new(config, 12345);

        // Send packets
        for i in 0..1000 {
            network.send(vec![i as u8]);
        }

        // Advance time
        for _ in 0..500 {
            network.tick(Duration::from_millis(10));
        }

        let stats = network.stats();
        println!("Hostile stats: {:?}", stats);

        // Should have significant loss
        assert!(stats.loss_rate() > 0.05);
    }

    #[test]
    fn test_jitter_distribution() {
        let mut rng = StdRng::seed_from_u64(42);

        let pareto = JitterDistribution::Pareto {
            scale_ms: 50.0,
            shape: 1.5,
        };

        let samples: Vec<Duration> = (0..1000).map(|_| pareto.sample(&mut rng)).collect();

        let avg = samples.iter().map(|d| d.as_millis()).sum::<u128>() / 1000;
        println!("Pareto avg jitter: {}ms", avg);

        // Should have heavy tail
        let max = samples.iter().map(|d| d.as_millis()).max().unwrap();
        assert!(max > avg * 2);
    }
}
