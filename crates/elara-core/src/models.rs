//! ELARA Formal Protocol Models v1
//!
//! The five foundational models that govern how ELARA perceives and handles reality.
//! Every module must be explainable through these models.
//!
//! # The Five Models
//!
//! | Model | ELARA Treats As |
//! |-------|-----------------|
//! | Failure | Distortion field |
//! | Timing | Local perception axis |
//! | Trust | Cryptographic continuity |
//! | Event | Ontological truth |
//! | Media | Perceptual fabric |
//!
//! # Derived from Hard Invariants
//!
//! - Failure Model ← INV-3: Experience Degrades, Never Collapses
//! - Timing Model ← INV-1: Reality Never Waits
//! - Trust Model ← INV-5: Identity Survives Transport
//! - Event Model ← INV-4: Event Is Truth, State Is Projection
//! - Media Model ← INV-2: Presence Over Packets

use std::fmt;

/// The five formal protocol models of ELARA.
///
/// Every module must be explainable through these models.
/// If a module cannot answer the model's core question correctly,
/// it is not part of ELARA core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ProtocolModel {
    /// Failure Model: Failure is distortion, not termination.
    ///
    /// Core question: "What is your failure model?"
    /// Valid answer: Distortion, not termination
    ///
    /// ELARA assumes all failure conditions are normal:
    /// - Extreme jitter
    /// - Arbitrary packet loss
    /// - Reordering, duplication
    /// - Network partition
    /// - Byzantine nodes
    /// - Device death
    /// - Clock drift
    /// - Server death
    Failure = 1,

    /// Timing Model: Time is local. Order is reconstructed.
    ///
    /// Core question: "Whose clock do you depend on?"
    /// Valid answer: Local only, reconstructed order
    ///
    /// Properties:
    /// - No protocol decision depends on wall-clock correctness
    /// - Causality > Timestamp
    /// - Liveness > Consistency
    /// - Reconstruction > Synchronization
    Timing = 2,

    /// Trust Model: Identity is cryptographic, not topological.
    ///
    /// Core question: "Where does trust come from?"
    /// Valid answer: Cryptographic continuity
    ///
    /// Trust is built from:
    /// - Continuity of keys
    /// - Behavior history
    /// - Event lineage
    ///
    /// NOT from: IP, server, TLS channel, login session
    Trust = 3,

    /// Event Model: Event is truth. State is a story we tell about it.
    ///
    /// Core question: "Where is your event-truth?"
    /// Valid answer: In signed, immutable events
    ///
    /// Properties:
    /// - Append-only reality
    /// - Forks are legal
    /// - Convergence is negotiated, not enforced
    /// - History > Snapshot
    Event = 4,

    /// Media Model: Media is perception, not data.
    ///
    /// Core question: "If media has holes, what still lives?"
    /// Valid answer: Perception continues
    ///
    /// Properties:
    /// - Silence is valid media
    /// - Approximation is first-class
    /// - Prediction is legal
    /// - Pipelines support stretching, concealment, hallucination
    Media = 5,
}

impl ProtocolModel {
    /// Get the model code (e.g., "MODEL-1")
    pub fn code(&self) -> &'static str {
        match self {
            ProtocolModel::Failure => "MODEL-1",
            ProtocolModel::Timing => "MODEL-2",
            ProtocolModel::Trust => "MODEL-3",
            ProtocolModel::Event => "MODEL-4",
            ProtocolModel::Media => "MODEL-5",
        }
    }

    /// Get the model name
    pub fn name(&self) -> &'static str {
        match self {
            ProtocolModel::Failure => "Failure Model",
            ProtocolModel::Timing => "Timing Model",
            ProtocolModel::Trust => "Trust Model",
            ProtocolModel::Event => "Event Model",
            ProtocolModel::Media => "Media Model",
        }
    }

    /// Get the model axiom
    pub fn axiom(&self) -> &'static str {
        match self {
            ProtocolModel::Failure => "Failure is distortion, not termination",
            ProtocolModel::Timing => "Time is local. Order is reconstructed",
            ProtocolModel::Trust => "Identity is cryptographic, not topological",
            ProtocolModel::Event => "Event is truth. State is a story we tell about it",
            ProtocolModel::Media => "Media is perception, not data",
        }
    }

    /// Get the core compliance question for this model
    pub fn compliance_question(&self) -> &'static str {
        match self {
            ProtocolModel::Failure => "What is your failure model?",
            ProtocolModel::Timing => "Whose clock do you depend on?",
            ProtocolModel::Trust => "Where does trust come from?",
            ProtocolModel::Event => "Where is your event-truth?",
            ProtocolModel::Media => "If media has holes, what still lives?",
        }
    }

    /// Get the valid answer pattern for this model
    pub fn valid_answer(&self) -> &'static str {
        match self {
            ProtocolModel::Failure => "Distortion, not termination",
            ProtocolModel::Timing => "Local only, reconstructed order",
            ProtocolModel::Trust => "Cryptographic continuity",
            ProtocolModel::Event => "In signed, immutable events",
            ProtocolModel::Media => "Perception continues",
        }
    }

    /// Get the related Hard Invariant
    pub fn related_invariant(&self) -> crate::Invariant {
        match self {
            ProtocolModel::Failure => crate::Invariant::ExperienceDegradesNeverCollapses,
            ProtocolModel::Timing => crate::Invariant::RealityNeverWaits,
            ProtocolModel::Trust => crate::Invariant::IdentitySurvivesTransport,
            ProtocolModel::Event => crate::Invariant::EventIsTruth,
            ProtocolModel::Media => crate::Invariant::PresenceOverPackets,
        }
    }

    /// Get all models
    pub fn all() -> &'static [ProtocolModel] {
        &[
            ProtocolModel::Failure,
            ProtocolModel::Timing,
            ProtocolModel::Trust,
            ProtocolModel::Event,
            ProtocolModel::Media,
        ]
    }
}

impl fmt::Display for ProtocolModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.name())
    }
}

/// Reconstructability class for media atoms.
///
/// Defines how a media unit should be handled if missing or incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ReconstructabilityClass {
    /// Critical for experience - must be felt
    /// Example: Voice onset, key visual frame
    MustFeel = 0,

    /// Can be estimated/approximated
    /// Example: Mid-syllable, motion blur
    MayApproximate = 1,

    /// Can be synthesized from context
    /// Example: Silence, static background
    Reconstructable = 2,

    /// Can be dropped without impact
    /// Example: Enhancement, cosmetic effects
    Ignorable = 3,
}

impl ReconstructabilityClass {
    /// Get the priority (lower = more important)
    pub fn priority(&self) -> u8 {
        *self as u8
    }

    /// Check if this class can be dropped under pressure
    pub fn droppable(&self) -> bool {
        matches!(self, Self::Ignorable | Self::Reconstructable)
    }

    /// Check if this class requires reconstruction if missing
    pub fn requires_reconstruction(&self) -> bool {
        matches!(self, Self::MustFeel | Self::MayApproximate)
    }
}

/// Perceptual weight for media atoms.
///
/// Indicates the importance of a media unit for human perception.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerceptualWeight {
    /// Emotional importance (0.0 - 1.0)
    pub emotional: f32,

    /// Presence importance (0.0 - 1.0)
    pub presence: f32,

    /// Continuity importance (0.0 - 1.0)
    pub continuity: f32,

    /// Reconstructability class
    pub class: ReconstructabilityClass,
}

impl PerceptualWeight {
    /// Create a new perceptual weight
    pub fn new(
        emotional: f32,
        presence: f32,
        continuity: f32,
        class: ReconstructabilityClass,
    ) -> Self {
        Self {
            emotional: emotional.clamp(0.0, 1.0),
            presence: presence.clamp(0.0, 1.0),
            continuity: continuity.clamp(0.0, 1.0),
            class,
        }
    }

    /// Critical media - must be delivered
    pub fn critical() -> Self {
        Self::new(1.0, 1.0, 1.0, ReconstructabilityClass::MustFeel)
    }

    /// Important media - should be delivered
    pub fn important() -> Self {
        Self::new(0.7, 0.7, 0.7, ReconstructabilityClass::MayApproximate)
    }

    /// Normal media - can be approximated
    pub fn normal() -> Self {
        Self::new(0.5, 0.5, 0.5, ReconstructabilityClass::Reconstructable)
    }

    /// Cosmetic media - can be dropped
    pub fn cosmetic() -> Self {
        Self::new(0.2, 0.2, 0.2, ReconstructabilityClass::Ignorable)
    }

    /// Calculate overall importance score
    pub fn importance(&self) -> f32 {
        (self.emotional + self.presence + self.continuity) / 3.0
    }
}

impl Default for PerceptualWeight {
    fn default() -> Self {
        Self::normal()
    }
}

/// Model compliance result for a module.
#[derive(Debug, Clone)]
pub struct ModelCompliance {
    pub model: ProtocolModel,
    pub compliant: bool,
    pub answer: String,
}

/// Check module compliance against all protocol models.
///
/// Returns a list of compliance results for each model.
pub fn check_model_compliance<F>(mut checker: F) -> Vec<ModelCompliance>
where
    F: FnMut(ProtocolModel) -> (bool, String),
{
    ProtocolModel::all()
        .iter()
        .map(|&model| {
            let (compliant, answer) = checker(model);
            ModelCompliance {
                model,
                compliant,
                answer,
            }
        })
        .collect()
}

/// Marker trait for types that comply with ELARA protocol models.
pub trait ModelCompliant {
    /// Answer the failure model question
    fn failure_model(&self) -> &'static str {
        "Distortion field - no termination"
    }

    /// Answer the timing model question
    fn timing_model(&self) -> &'static str {
        "Local time only - order reconstructed"
    }

    /// Answer the trust model question
    fn trust_model(&self) -> &'static str {
        "Cryptographic continuity"
    }

    /// Answer the event model question
    fn event_model(&self) -> &'static str {
        "Signed immutable events"
    }

    /// Answer the media model question
    fn media_model(&self) -> &'static str {
        "Perception continues"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_codes() {
        assert_eq!(ProtocolModel::Failure.code(), "MODEL-1");
        assert_eq!(ProtocolModel::Timing.code(), "MODEL-2");
        assert_eq!(ProtocolModel::Trust.code(), "MODEL-3");
        assert_eq!(ProtocolModel::Event.code(), "MODEL-4");
        assert_eq!(ProtocolModel::Media.code(), "MODEL-5");
    }

    #[test]
    fn test_all_models() {
        assert_eq!(ProtocolModel::all().len(), 5);
    }

    #[test]
    fn test_model_invariant_relationship() {
        use crate::Invariant;

        assert_eq!(
            ProtocolModel::Failure.related_invariant(),
            Invariant::ExperienceDegradesNeverCollapses
        );
        assert_eq!(
            ProtocolModel::Timing.related_invariant(),
            Invariant::RealityNeverWaits
        );
        assert_eq!(
            ProtocolModel::Trust.related_invariant(),
            Invariant::IdentitySurvivesTransport
        );
        assert_eq!(
            ProtocolModel::Event.related_invariant(),
            Invariant::EventIsTruth
        );
        assert_eq!(
            ProtocolModel::Media.related_invariant(),
            Invariant::PresenceOverPackets
        );
    }

    #[test]
    fn test_reconstructability_priority() {
        assert!(
            ReconstructabilityClass::MustFeel.priority()
                < ReconstructabilityClass::Ignorable.priority()
        );
    }

    #[test]
    fn test_perceptual_weight() {
        let critical = PerceptualWeight::critical();
        let cosmetic = PerceptualWeight::cosmetic();

        assert!(critical.importance() > cosmetic.importance());
        assert!(!critical.class.droppable());
        assert!(cosmetic.class.droppable());
    }

    #[test]
    fn test_check_model_compliance() {
        let results =
            check_model_compliance(|model| (true, format!("Compliant with {}", model.name())));

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.compliant));
    }

    #[test]
    fn test_model_display() {
        let model = ProtocolModel::Failure;
        let display = format!("{}", model);
        assert!(display.contains("MODEL-1"));
        assert!(display.contains("Failure Model"));
    }
}
