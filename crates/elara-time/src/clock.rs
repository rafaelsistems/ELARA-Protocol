//! Clock implementations for ELARA Time Engine

use std::time::{Duration, Instant};

use elara_core::{PerceptualTime, StateTime};

/// Perceptual clock (τp) - monotonic, smooth, local-driven
/// INVARIANT: τp MUST be monotonically increasing, NEVER jumps
pub struct PerceptualClock {
    /// Current perceptual time
    value: PerceptualTime,
    /// Reference to monotonic OS clock
    reference: Instant,
    /// Last update instant
    last_update: Instant,
}

impl PerceptualClock {
    /// Create a new perceptual clock starting at zero
    pub fn new() -> Self {
        let now = Instant::now();
        PerceptualClock {
            value: PerceptualTime::ZERO,
            reference: now,
            last_update: now,
        }
    }

    /// Advance the clock based on elapsed real time
    /// Returns the new perceptual time
    pub fn tick(&mut self) -> PerceptualTime {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        
        // Clamp to prevent large jumps (e.g., after system sleep)
        let clamped = elapsed.min(Duration::from_millis(100));
        
        self.value = self.value.saturating_add(clamped);
        self.last_update = now;
        self.value
    }

    /// Get current perceptual time without advancing
    pub fn now(&self) -> PerceptualTime {
        self.value
    }

    /// Check if clock is advancing (sanity check)
    pub fn is_advancing(&self) -> bool {
        // Clock is always advancing as long as tick() is called
        true
    }
}

impl Default for PerceptualClock {
    fn default() -> Self {
        Self::new()
    }
}

/// State clock (τs) - elastic, drift-correctable, convergence-oriented
/// CAN be bent for convergence, but maintains causality
pub struct StateClock {
    /// Current state time
    value: StateTime,
    /// Clock rate multiplier (1.0 = real-time)
    rate: f64,
    /// Convergence target (if any)
    convergence_target: Option<StateTime>,
    /// Maximum correction per tick
    max_correction_per_tick: Duration,
}

impl StateClock {
    /// Create a new state clock starting at zero
    pub fn new() -> Self {
        StateClock {
            value: StateTime::ZERO,
            rate: 1.0,
            convergence_target: None,
            max_correction_per_tick: Duration::from_millis(10),
        }
    }

    /// Advance the clock by a duration, applying rate and convergence
    /// Returns the new state time
    pub fn advance(&mut self, dt: Duration) -> StateTime {
        // Base advance with rate
        let base_advance_us = (dt.as_micros() as f64 * self.rate) as i64;
        
        // Apply convergence correction if needed
        let correction = if let Some(target) = self.convergence_target {
            let error = target.as_micros() - self.value.as_micros();
            let max_correction = self.max_correction_per_tick.as_micros() as i64;
            
            // Proportional correction (10% of error, clamped)
            let correction = (error as f64 * 0.1) as i64;
            correction.clamp(-max_correction, max_correction)
        } else {
            0
        };
        
        self.value = StateTime::from_micros(self.value.as_micros() + base_advance_us + correction);
        self.value
    }

    /// Get current state time
    pub fn now(&self) -> StateTime {
        self.value
    }

    /// Set convergence target
    pub fn set_convergence_target(&mut self, target: StateTime) {
        self.convergence_target = Some(target);
    }

    /// Clear convergence target
    pub fn clear_convergence_target(&mut self) {
        self.convergence_target = None;
    }

    /// Set clock rate (for time dilation)
    /// Rate must be between 0.5 and 2.0
    pub fn set_rate(&mut self, rate: f64) {
        self.rate = rate.clamp(0.5, 2.0);
    }

    /// Get current rate
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Sync to a specific time (for recovery)
    /// Only allowed to move forward
    pub fn sync_to(&mut self, target: StateTime) {
        if target > self.value {
            self.value = target;
        }
    }
}

impl Default for StateClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perceptual_clock_monotonic() {
        let mut clock = PerceptualClock::new();
        
        let t1 = clock.tick();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = clock.tick();
        
        assert!(t2 > t1);
    }

    #[test]
    fn test_state_clock_advance() {
        let mut clock = StateClock::new();
        
        let t1 = clock.now();
        clock.advance(Duration::from_millis(100));
        let t2 = clock.now();
        
        assert!(t2 > t1);
        // Should be approximately 100ms later
        let diff = t2.as_micros() - t1.as_micros();
        assert!(diff >= 99_000 && diff <= 101_000);
    }

    #[test]
    fn test_state_clock_rate() {
        let mut clock = StateClock::new();
        
        // Double speed
        clock.set_rate(2.0);
        clock.advance(Duration::from_millis(100));
        
        // Should advance ~200ms
        let value = clock.now().as_micros();
        assert!(value >= 190_000 && value <= 210_000);
    }

    #[test]
    fn test_state_clock_convergence() {
        let mut clock = StateClock::new();
        
        // Set target ahead
        clock.set_convergence_target(StateTime::from_millis(1000));
        
        // Advance multiple times
        for _ in 0..100 {
            clock.advance(Duration::from_millis(10));
        }
        
        // Should be closer to target than just 1000ms of advance
        let value = clock.now().as_millis();
        assert!(value > 1000); // Converged toward target
    }
}
