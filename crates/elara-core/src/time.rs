//! Time primitives for ELARA protocol
//!
//! ELARA uses a dual-clock system:
//! - τp (Perceptual Time): monotonic, smooth, local-driven
//! - τs (State Time): elastic, drift-correctable, convergence-oriented

use std::ops::{Add, Sub};
use std::time::Duration;

/// State time (τs) - elastic, convergence-oriented
/// Represented as microseconds since session epoch
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StateTime(pub i64);

impl StateTime {
    pub const ZERO: StateTime = StateTime(0);
    pub const MAX: StateTime = StateTime(i64::MAX);
    pub const MIN: StateTime = StateTime(i64::MIN);

    #[inline]
    pub fn from_micros(micros: i64) -> Self {
        StateTime(micros)
    }

    #[inline]
    pub fn from_millis(millis: i64) -> Self {
        StateTime(millis * 1000)
    }

    #[inline]
    pub fn from_secs_f64(secs: f64) -> Self {
        StateTime((secs * 1_000_000.0) as i64)
    }

    #[inline]
    pub fn as_micros(self) -> i64 {
        self.0
    }

    #[inline]
    pub fn as_millis(self) -> i64 {
        self.0 / 1000
    }

    #[inline]
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    /// Convert to wire format (100μs units, 32-bit offset)
    #[inline]
    pub fn to_wire_offset(self, reference: StateTime) -> i32 {
        let diff_100us = (self.0 - reference.0) / 100;
        diff_100us.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }

    /// Convert from wire format (100μs units, 32-bit offset)
    #[inline]
    pub fn from_wire_offset(reference: StateTime, offset_100us: i32) -> Self {
        StateTime(reference.0 + (offset_100us as i64 * 100))
    }

    #[inline]
    pub fn saturating_add(self, duration: Duration) -> Self {
        StateTime(self.0.saturating_add(duration.as_micros() as i64))
    }

    #[inline]
    pub fn saturating_sub(self, duration: Duration) -> Self {
        StateTime(self.0.saturating_sub(duration.as_micros() as i64))
    }
}

impl Add<Duration> for StateTime {
    type Output = StateTime;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        StateTime(self.0 + rhs.as_micros() as i64)
    }
}

impl Sub<Duration> for StateTime {
    type Output = StateTime;

    #[inline]
    fn sub(self, rhs: Duration) -> Self::Output {
        StateTime(self.0 - rhs.as_micros() as i64)
    }
}

impl Sub<StateTime> for StateTime {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: StateTime) -> Self::Output {
        let diff = self.0 - rhs.0;
        if diff >= 0 {
            Duration::from_micros(diff as u64)
        } else {
            Duration::ZERO
        }
    }
}

impl std::fmt::Debug for StateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "τs({:.3}ms)", self.as_millis() as f64)
    }
}

/// Perceptual time (τp) - monotonic, smooth, local-driven
/// Represented as microseconds since node start
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PerceptualTime(pub u64);

impl PerceptualTime {
    pub const ZERO: PerceptualTime = PerceptualTime(0);

    #[inline]
    pub fn from_micros(micros: u64) -> Self {
        PerceptualTime(micros)
    }

    #[inline]
    pub fn from_millis(millis: u64) -> Self {
        PerceptualTime(millis * 1000)
    }

    #[inline]
    pub fn from_secs_f64(secs: f64) -> Self {
        PerceptualTime((secs * 1_000_000.0) as u64)
    }

    #[inline]
    pub fn as_micros(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn as_millis(self) -> u64 {
        self.0 / 1000
    }

    #[inline]
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    #[inline]
    pub fn saturating_add(self, duration: Duration) -> Self {
        PerceptualTime(self.0.saturating_add(duration.as_micros() as u64))
    }
}

impl Add<Duration> for PerceptualTime {
    type Output = PerceptualTime;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        PerceptualTime(self.0 + rhs.as_micros() as u64)
    }
}

impl Sub<PerceptualTime> for PerceptualTime {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: PerceptualTime) -> Self::Output {
        Duration::from_micros(self.0.saturating_sub(rhs.0))
    }
}

impl std::fmt::Debug for PerceptualTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "τp({:.3}ms)", self.as_millis() as f64)
    }
}

/// Reality Window - defines the temporal bounds for event processing
/// RW = [τs - Hc, τs + Hp]
#[derive(Clone, Copy, Debug)]
pub struct RealityWindow {
    /// Current state time
    pub τs: StateTime,
    /// Correction horizon (how far back we can correct)
    pub Hc: Duration,
    /// Prediction horizon (how far ahead we predict)
    pub Hp: Duration,
}

impl RealityWindow {
    pub fn new(τs: StateTime, Hc: Duration, Hp: Duration) -> Self {
        RealityWindow { τs, Hc, Hp }
    }

    /// Left bound of reality window (oldest correctable time)
    #[inline]
    pub fn left(&self) -> StateTime {
        self.τs.saturating_sub(self.Hc)
    }

    /// Right bound of reality window (furthest predicted time)
    #[inline]
    pub fn right(&self) -> StateTime {
        self.τs.saturating_add(self.Hp)
    }

    /// Check if a time is within the reality window
    #[inline]
    pub fn contains(&self, t: StateTime) -> bool {
        t >= self.left() && t <= self.right()
    }

    /// Classify a time relative to the reality window
    pub fn classify(&self, t: StateTime) -> TimePosition {
        if t < self.left() {
            TimePosition::TooLate
        } else if t < self.τs {
            TimePosition::Correctable
        } else if t <= self.τs + Duration::from_millis(5) {
            TimePosition::Current
        } else if t <= self.right() {
            TimePosition::Predictable
        } else {
            TimePosition::TooEarly
        }
    }
}

/// Position of a time relative to the reality window
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimePosition {
    /// Before correction horizon - too late to process
    TooLate,
    /// Within correction horizon - can be blended in
    Correctable,
    /// At current time - apply directly
    Current,
    /// Within prediction horizon - replace prediction
    Predictable,
    /// Beyond prediction horizon - buffer for later
    TooEarly,
}

/// Time intent carried by events
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeIntent {
    /// Intended state time (relative offset in wire format)
    pub τs_offset: i32,
    /// Optional deadline for perceptual events
    pub deadline: Option<i32>,
}

impl TimeIntent {
    pub fn new(τs_offset: i32) -> Self {
        TimeIntent {
            τs_offset,
            deadline: None,
        }
    }

    pub fn with_deadline(mut self, deadline: i32) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Convert to absolute state time given a reference
    pub fn to_absolute(&self, reference: StateTime) -> StateTime {
        StateTime::from_wire_offset(reference, self.τs_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_time_wire_roundtrip() {
        let reference = StateTime::from_millis(1000);
        let time = StateTime::from_millis(1050);
        
        let offset = time.to_wire_offset(reference);
        let recovered = StateTime::from_wire_offset(reference, offset);
        
        // Should be within 100μs precision
        assert!((time.0 - recovered.0).abs() < 100);
    }

    #[test]
    fn test_reality_window_classification() {
        let τs = StateTime::from_millis(1000);
        let Hc = Duration::from_millis(100);
        let Hp = Duration::from_millis(50);
        let rw = RealityWindow::new(τs, Hc, Hp);

        // Too late (before Hc)
        assert_eq!(rw.classify(StateTime::from_millis(850)), TimePosition::TooLate);
        
        // Correctable (within Hc)
        assert_eq!(rw.classify(StateTime::from_millis(950)), TimePosition::Correctable);
        
        // Current (at τs)
        assert_eq!(rw.classify(StateTime::from_millis(1000)), TimePosition::Current);
        
        // Predictable (within Hp)
        assert_eq!(rw.classify(StateTime::from_millis(1030)), TimePosition::Predictable);
        
        // Too early (beyond Hp)
        assert_eq!(rw.classify(StateTime::from_millis(1100)), TimePosition::TooEarly);
    }

    #[test]
    fn test_perceptual_time_monotonic() {
        let t1 = PerceptualTime::from_millis(100);
        let t2 = t1 + Duration::from_millis(10);
        
        assert!(t2 > t1);
        assert_eq!(t2 - t1, Duration::from_millis(10));
    }
}
