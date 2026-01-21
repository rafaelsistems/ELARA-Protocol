//! Network model for passive jitter/latency estimation

use std::collections::HashMap;
use std::time::Duration;

use elara_core::NodeId;

/// Network statistics for a single peer
#[derive(Clone, Debug)]
pub struct PeerNetworkModel {
    /// Estimated clock offset (local - remote)
    pub offset: f64,
    /// Estimated clock skew (drift rate)
    pub skew: f64,
    /// Jitter envelope (max deviation)
    pub jitter_envelope: f64,
    /// Recent latency samples
    samples: Vec<f64>,
    /// Maximum samples to keep
    max_samples: usize,
}

impl PeerNetworkModel {
    pub fn new() -> Self {
        PeerNetworkModel {
            offset: 0.0,
            skew: 0.0,
            jitter_envelope: 0.0,
            samples: Vec::new(),
            max_samples: 100,
        }
    }

    /// Update with a new timing sample
    pub fn update(&mut self, local_time: f64, remote_time: f64) {
        let sample = local_time - remote_time;
        self.samples.push(sample);

        // Trim old samples
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }

        // Update estimates
        if self.samples.len() >= 5 {
            self.offset = Self::median(&self.samples);
            self.jitter_envelope = self.samples.iter()
                .map(|s| (s - self.offset).abs())
                .fold(0.0, f64::max);
        }
    }

    fn median(values: &[f64]) -> f64 {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
}

impl Default for PeerNetworkModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate network model across all peers
#[derive(Debug, Default)]
pub struct NetworkModel {
    /// Per-peer models
    pub peers: HashMap<NodeId, PeerNetworkModel>,
    /// Aggregate latency mean (seconds)
    pub latency_mean: f64,
    /// Aggregate jitter (seconds)
    pub jitter: f64,
    /// Estimated reorder depth
    pub reorder_depth: u32,
    /// Estimated loss rate (0.0 - 1.0)
    pub loss_rate: f64,
    /// Overall stability score (0.0 - 1.0)
    pub stability_score: f64,
}

impl NetworkModel {
    pub fn new() -> Self {
        NetworkModel::default()
    }

    /// Update model from a received packet
    pub fn update_from_packet(&mut self, peer: NodeId, local_time: f64, remote_time: f64, seq: u16) {
        let peer_model = self.peers.entry(peer).or_default();
        peer_model.update(local_time, remote_time);

        // Update aggregate statistics
        self.update_aggregates();
    }

    /// Record a detected reorder
    pub fn record_reorder(&mut self, depth: u32) {
        self.reorder_depth = self.reorder_depth.max(depth);
    }

    /// Record packet loss
    pub fn record_loss(&mut self, lost_count: u32, total_count: u32) {
        if total_count > 0 {
            let new_rate = lost_count as f64 / total_count as f64;
            // Exponential moving average
            self.loss_rate = self.loss_rate * 0.9 + new_rate * 0.1;
        }
    }

    fn update_aggregates(&mut self) {
        if self.peers.is_empty() {
            return;
        }

        // Average jitter across peers
        let total_jitter: f64 = self.peers.values().map(|p| p.jitter_envelope).sum();
        self.jitter = total_jitter / self.peers.len() as f64;

        // Compute stability score
        let jitter_factor = 1.0 / (1.0 + self.jitter * 10.0);
        let loss_factor = 1.0 - self.loss_rate;
        let reorder_factor = 1.0 / (1.0 + self.reorder_depth as f64 * 0.1);

        self.stability_score = (jitter_factor * loss_factor * reorder_factor).clamp(0.0, 1.0);
    }

    /// Get peer model
    pub fn get_peer(&self, peer: NodeId) -> Option<&PeerNetworkModel> {
        self.peers.get(&peer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_model_update() {
        let mut model = PeerNetworkModel::new();

        // Simulate samples with ~50ms offset and some jitter
        for i in 0..20 {
            let jitter = (i % 5) as f64 * 0.005; // 0-20ms jitter
            model.update(1.0 + i as f64 * 0.1, 0.95 + i as f64 * 0.1 + jitter);
        }

        // Offset should be approximately 0.05 (50ms)
        assert!((model.offset - 0.05).abs() < 0.02);
    }

    #[test]
    fn test_network_model_stability() {
        let mut model = NetworkModel::new();

        // Good network
        for i in 0..10 {
            model.update_from_packet(NodeId::new(1), i as f64, i as f64 - 0.05, i as u16);
        }

        assert!(model.stability_score > 0.8);

        // Add significant loss (50%)
        model.record_loss(50, 100);
        model.update_aggregates();

        // With 50% loss recorded (EMA gives ~5% loss_rate), stability should drop
        assert!(model.stability_score < 1.0);
    }
}
