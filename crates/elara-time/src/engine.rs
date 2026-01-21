//! Time Engine - orchestrates dual clocks, horizons, and temporal control

use std::time::Duration;

use elara_core::{NodeId, PerceptualTime, RealityWindow, StateTime, TimePosition};

use crate::{NetworkModel, PerceptualClock, StateClock};

/// Time Engine configuration
#[derive(Clone, Debug)]
pub struct TimeEngineConfig {
    /// Minimum prediction horizon
    pub Hp_min: Duration,
    /// Maximum prediction horizon
    pub Hp_max: Duration,
    /// Minimum correction horizon
    pub Hc_min: Duration,
    /// Maximum correction horizon
    pub Hc_max: Duration,
    /// Jitter sensitivity for Hp
    pub k1_jitter: f64,
    /// Reorder sensitivity for Hp
    pub k2_reorder: f64,
    /// Loss sensitivity for Hp
    pub k3_loss: f64,
    /// Jitter sensitivity for Hc
    pub k4_jitter_correct: f64,
    /// Tick interval
    pub tick_interval: Duration,
}

impl Default for TimeEngineConfig {
    fn default() -> Self {
        // MSP defaults
        TimeEngineConfig {
            Hp_min: Duration::from_millis(40),
            Hp_max: Duration::from_millis(300),
            Hc_min: Duration::from_millis(80),
            Hc_max: Duration::from_millis(600),
            k1_jitter: 2.5,
            k2_reorder: 15.0,
            k3_loss: 150.0,
            k4_jitter_correct: 2.0,
            tick_interval: Duration::from_millis(10),
        }
    }
}

impl TimeEngineConfig {
    /// Configuration for low-bandwidth networks (2G)
    pub fn low_bandwidth() -> Self {
        TimeEngineConfig {
            Hp_min: Duration::from_millis(80),
            Hp_max: Duration::from_millis(500),
            Hc_min: Duration::from_millis(150),
            Hc_max: Duration::from_millis(1000),
            k1_jitter: 3.0,
            k2_reorder: 20.0,
            k3_loss: 200.0,
            k4_jitter_correct: 2.5,
            tick_interval: Duration::from_millis(15),
        }
    }
}

/// Time Engine - manages dual clocks and temporal control
pub struct TimeEngine {
    /// Perceptual clock (τp)
    perceptual: PerceptualClock,
    /// State clock (τs)
    state: StateClock,
    /// Network model
    network: NetworkModel,
    /// Current prediction horizon
    Hp: Duration,
    /// Current correction horizon
    Hc: Duration,
    /// Configuration
    config: TimeEngineConfig,
}

impl TimeEngine {
    /// Create a new Time Engine with default configuration
    pub fn new() -> Self {
        Self::with_config(TimeEngineConfig::default())
    }

    /// Create a new Time Engine with custom configuration
    pub fn with_config(config: TimeEngineConfig) -> Self {
        TimeEngine {
            perceptual: PerceptualClock::new(),
            state: StateClock::new(),
            network: NetworkModel::new(),
            Hp: config.Hp_min,
            Hc: config.Hc_min,
            config,
        }
    }

    /// Advance clocks by one tick
    /// This is the core tick function - MUST be called every tick
    pub fn tick(&mut self) {
        // τp ALWAYS advances - this is the never-freeze guarantee
        self.perceptual.tick();

        // τs advances with potential convergence correction
        self.state.advance(self.config.tick_interval);

        // Adjust horizons based on network quality
        self.adjust_horizons();
    }

    /// Get current perceptual time (τp)
    pub fn τp(&self) -> PerceptualTime {
        self.perceptual.now()
    }

    /// Get current state time (τs)
    pub fn τs(&self) -> StateTime {
        self.state.now()
    }

    /// Get current prediction horizon
    pub fn Hp(&self) -> Duration {
        self.Hp
    }

    /// Get current correction horizon
    pub fn Hc(&self) -> Duration {
        self.Hc
    }

    /// Get current reality window
    pub fn reality_window(&self) -> RealityWindow {
        RealityWindow::new(self.state.now(), self.Hc, self.Hp)
    }

    /// Classify a time relative to the reality window
    pub fn classify_time(&self, t: StateTime) -> TimePosition {
        self.reality_window().classify(t)
    }

    /// Update network model from a received packet
    pub fn update_from_packet(&mut self, peer: NodeId, remote_time: StateTime, seq: u16) {
        let local_time = self.state.now().as_secs_f64();
        let remote_time_f = remote_time.as_secs_f64();
        self.network.update_from_packet(peer, local_time, remote_time_f, seq);
    }

    /// Record packet reorder
    pub fn record_reorder(&mut self, depth: u32) {
        self.network.record_reorder(depth);
    }

    /// Record packet loss
    pub fn record_loss(&mut self, lost: u32, total: u32) {
        self.network.record_loss(lost, total);
    }

    /// Get network stability score
    pub fn stability_score(&self) -> f64 {
        self.network.stability_score
    }

    /// Adjust horizons based on network quality
    fn adjust_horizons(&mut self) {
        let net = &self.network;
        let cfg = &self.config;

        // Prediction horizon: expands with network degradation
        let Hp_raw = cfg.Hp_min.as_secs_f64()
            + cfg.k1_jitter * net.jitter
            + cfg.k2_reorder * net.reorder_depth as f64 * 0.001
            + cfg.k3_loss * net.loss_rate;

        self.Hp = Duration::from_secs_f64(
            Hp_raw.clamp(cfg.Hp_min.as_secs_f64(), cfg.Hp_max.as_secs_f64()),
        );

        // Correction horizon: expands with jitter
        let Hc_raw = cfg.Hc_min.as_secs_f64() + cfg.k4_jitter_correct * net.jitter;

        self.Hc = Duration::from_secs_f64(
            Hc_raw.clamp(cfg.Hc_min.as_secs_f64(), cfg.Hc_max.as_secs_f64()),
        );
    }

    /// Calculate correction weight for a late event
    /// Weight decreases as event gets older
    pub fn correction_weight(&self, delay: Duration) -> f64 {
        let Hc = self.Hc.as_secs_f64();
        let delay_secs = delay.as_secs_f64();
        (1.0 - delay_secs / Hc).clamp(0.0, 1.0)
    }

    /// Get reference to network model
    pub fn network(&self) -> &NetworkModel {
        &self.network
    }

    /// Get mutable reference to state clock (for convergence control)
    pub fn state_clock_mut(&mut self) -> &mut StateClock {
        &mut self.state
    }
}

impl Default for TimeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_engine_tick() {
        let mut engine = TimeEngine::new();

        let τp1 = engine.τp();
        let τs1 = engine.τs();

        engine.tick();

        let τp2 = engine.τp();
        let τs2 = engine.τs();

        // Both clocks should advance
        assert!(τp2 >= τp1);
        assert!(τs2 >= τs1);
    }

    #[test]
    fn test_reality_window() {
        let engine = TimeEngine::new();
        let rw = engine.reality_window();

        // Window should be centered around τs
        assert!(rw.left() < rw.τs);
        assert!(rw.right() > rw.τs);
    }

    #[test]
    fn test_horizon_adaptation() {
        let mut engine = TimeEngine::new();

        let initial_Hp = engine.Hp();

        // Simulate bad network
        engine.network.jitter = 0.1; // 100ms jitter
        engine.network.loss_rate = 0.1; // 10% loss
        engine.adjust_horizons();

        // Hp should increase
        assert!(engine.Hp() > initial_Hp);
    }

    #[test]
    fn test_correction_weight() {
        let engine = TimeEngine::new();

        // No delay = full weight
        assert!((engine.correction_weight(Duration::ZERO) - 1.0).abs() < 0.01);

        // At Hc = zero weight
        assert!(engine.correction_weight(engine.Hc()) < 0.01);

        // Half Hc = half weight
        let half_Hc = engine.Hc() / 2;
        let weight = engine.correction_weight(half_Hc);
        assert!((weight - 0.5).abs() < 0.1);
    }
}
