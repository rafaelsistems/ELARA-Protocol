//! ELARA Hard Invariants
//!
//! These are system laws, not guidelines.
//! Violation of any invariant means the system is not ELARA.
//!
//! # The Five Invariants
//!
//! 1. **Reality Never Waits** - System never blocks reality for synchronization
//! 2. **Presence Over Packets** - Existence matters more than data perfection
//! 3. **Experience Degrades, Never Collapses** - Quality reduces, never fails
//! 4. **Event Is Truth, State Is Projection** - Events are authoritative
//! 5. **Identity Survives Transport** - Identity persists beyond connections
//!
//! # Usage
//!
//! Every component must be evaluated against all invariants.
//! Use [`validate_invariant`] to check compliance at runtime.
//!
//! ```rust
//! use elara_core::invariants::{Invariant, validate_invariant};
//!
//! // Check if a design decision violates invariants
//! assert!(validate_invariant(Invariant::RealityNeverWaits, || {
//!     // This operation does not block on network
//!     true
//! }));
//! ```

use std::fmt;

/// The five hard invariants of ELARA Protocol.
///
/// If any single invariant falls, the system is not ELARA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Invariant {
    /// INV-1: Reality Never Waits
    ///
    /// Reality does not wait for the network.
    /// The system must never block reality for synchronization.
    ///
    /// Technical requirements:
    /// - No design assumes connection-first
    /// - Events always allowed even when socket dead, peer not found, state not synced
    /// - All layers must be event-driven, not session-driven
    ///
    /// Architectural consequences:
    /// - Local-first capture
    /// - Append-only event log
    /// - Opportunistic sync
    /// - No hard dependency on continuous channels
    RealityNeverWaits = 1,

    /// INV-2: Presence Over Packets
    ///
    /// What is preserved is presence (existence), not data perfection.
    ///
    /// Protocol prioritizes:
    /// - Liveness (proof of existence)
    /// - Immediacy (real-time awareness)
    /// - Continuity of perception (unbroken experience)
    ///
    /// Over:
    /// - Absolute reliability
    /// - Absolute ordering
    /// - Absolute completeness
    ///
    /// Consequences:
    /// - Partial data is valid state
    /// - Silence ≠ failure
    /// - Lost packet ≠ lost presence
    PresenceOverPackets = 2,

    /// INV-3: Experience Degrades, Never Collapses
    ///
    /// Experience quality may decrease, but it must never collapse.
    ///
    /// No binary failures:
    /// - No "call dropped"
    /// - No "session dead"
    /// - No "connection lost"
    ///
    /// All subsystems must have:
    /// - Fallback mode
    /// - Perceptual smoothing
    /// - Prediction / concealment / approximation
    ExperienceDegradesNeverCollapses = 3,

    /// INV-4: Event Is Truth, State Is Projection
    ///
    /// Truth is the occurrence, not the condition.
    ///
    /// Event Log > Synchronized State
    ///
    /// State is merely:
    /// - A cache
    /// - A projection
    /// - An approximation
    ///
    /// There is no "single source of truth" in the form of server state.
    ///
    /// Consequences:
    /// - CRDT-based reconciliation
    /// - Causal graph for events
    /// - Vector time for ordering
    /// - Reconciliation is normal
    /// - Divergence is expected
    /// - Convergence is eventual, not prerequisite
    EventIsTruth = 4,

    /// INV-5: Identity Survives Transport
    ///
    /// Identity and relationships must not depend on communication channels.
    ///
    /// Identity ≠ Socket
    /// Presence ≠ Session
    /// Trust ≠ IP / Server
    ///
    /// Consequences:
    /// - Self-authenticating identity
    /// - Cryptographic continuity
    /// - Session can die, relationship cannot
    IdentitySurvivesTransport = 5,
}

impl Invariant {
    /// Get the invariant code (e.g., "INV-1")
    pub fn code(&self) -> &'static str {
        match self {
            Invariant::RealityNeverWaits => "INV-1",
            Invariant::PresenceOverPackets => "INV-2",
            Invariant::ExperienceDegradesNeverCollapses => "INV-3",
            Invariant::EventIsTruth => "INV-4",
            Invariant::IdentitySurvivesTransport => "INV-5",
        }
    }

    /// Get the short name of the invariant
    pub fn name(&self) -> &'static str {
        match self {
            Invariant::RealityNeverWaits => "Reality Never Waits",
            Invariant::PresenceOverPackets => "Presence Over Packets",
            Invariant::ExperienceDegradesNeverCollapses => "Experience Degrades, Never Collapses",
            Invariant::EventIsTruth => "Event Is Truth, State Is Projection",
            Invariant::IdentitySurvivesTransport => "Identity Survives Transport",
        }
    }

    /// Get all invariants
    pub fn all() -> &'static [Invariant] {
        &[
            Invariant::RealityNeverWaits,
            Invariant::PresenceOverPackets,
            Invariant::ExperienceDegradesNeverCollapses,
            Invariant::EventIsTruth,
            Invariant::IdentitySurvivesTransport,
        ]
    }
}

impl fmt::Display for Invariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.name())
    }
}

/// Invariant violation error
#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub invariant: Invariant,
    pub context: String,
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ELARA Invariant Violation: {} - {}",
            self.invariant, self.context
        )
    }
}

impl std::error::Error for InvariantViolation {}

/// Validate that an operation complies with an invariant.
///
/// Returns `true` if the invariant is satisfied, `false` if violated.
///
/// # Example
///
/// ```rust
/// use elara_core::invariants::{Invariant, validate_invariant};
///
/// let compliant = validate_invariant(Invariant::RealityNeverWaits, || {
///     // Check: does this operation block on network?
///     // Return true if compliant, false if it blocks
///     true
/// });
///
/// assert!(compliant);
/// ```
pub fn validate_invariant<F>(invariant: Invariant, check: F) -> bool
where
    F: FnOnce() -> bool,
{
    check()
}

/// Assert that an invariant is satisfied, panicking if violated.
///
/// Use this in tests and debug builds to catch invariant violations early.
///
/// # Panics
///
/// Panics if the invariant check returns `false`.
#[track_caller]
pub fn assert_invariant<F>(invariant: Invariant, context: &str, check: F)
where
    F: FnOnce() -> bool,
{
    if !check() {
        panic!(
            "{}",
            InvariantViolation {
                invariant,
                context: context.to_string(),
            }
        );
    }
}

/// Check all invariants for a component.
///
/// Returns a list of violated invariants.
pub fn check_all_invariants<F>(mut checker: F) -> Vec<InvariantViolation>
where
    F: FnMut(Invariant) -> Result<(), String>,
{
    let mut violations = Vec::new();

    for &invariant in Invariant::all() {
        if let Err(context) = checker(invariant) {
            violations.push(InvariantViolation { invariant, context });
        }
    }

    violations
}

/// Marker trait for types that comply with ELARA invariants.
///
/// Implementing this trait is a declaration that the type
/// has been designed and reviewed for invariant compliance.
pub trait InvariantCompliant {
    /// Verify that this instance complies with all invariants.
    ///
    /// Returns `Ok(())` if compliant, or a list of violations.
    fn verify_invariants(&self) -> Result<(), Vec<InvariantViolation>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_codes() {
        assert_eq!(Invariant::RealityNeverWaits.code(), "INV-1");
        assert_eq!(Invariant::PresenceOverPackets.code(), "INV-2");
        assert_eq!(Invariant::ExperienceDegradesNeverCollapses.code(), "INV-3");
        assert_eq!(Invariant::EventIsTruth.code(), "INV-4");
        assert_eq!(Invariant::IdentitySurvivesTransport.code(), "INV-5");
    }

    #[test]
    fn test_all_invariants() {
        assert_eq!(Invariant::all().len(), 5);
    }

    #[test]
    fn test_validate_invariant_pass() {
        let result = validate_invariant(Invariant::RealityNeverWaits, || true);
        assert!(result);
    }

    #[test]
    fn test_validate_invariant_fail() {
        let result = validate_invariant(Invariant::RealityNeverWaits, || false);
        assert!(!result);
    }

    #[test]
    #[should_panic(expected = "ELARA Invariant Violation")]
    fn test_assert_invariant_panics() {
        assert_invariant(
            Invariant::RealityNeverWaits,
            "Test violation",
            || false,
        );
    }

    #[test]
    fn test_check_all_invariants() {
        let violations = check_all_invariants(|inv| {
            if inv == Invariant::EventIsTruth {
                Err("Test violation".to_string())
            } else {
                Ok(())
            }
        });

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].invariant, Invariant::EventIsTruth);
    }

    #[test]
    fn test_invariant_display() {
        let inv = Invariant::RealityNeverWaits;
        let display = format!("{}", inv);
        assert!(display.contains("INV-1"));
        assert!(display.contains("Reality Never Waits"));
    }
}
