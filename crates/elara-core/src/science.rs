//! ELARA System Science — Foundational Constructs v1
//!
//! Whitepaper-grade conceptual specification defining:
//! - Node Ontology
//! - Event Algebra
//! - Presence Metric
//! - Degradation Ladder
//! - Chaos Test Specification
//!
//! This module provides the scientific foundations of ELARA.

use std::fmt;
use std::ops::{BitOr, BitAnd};

// ============================================================================
// I. NODE ONTOLOGY
// ============================================================================

/// Ontological class of an ELARA node.
///
/// ELARA does not recognize "client", "server", "peer" as ontological classes.
/// A node is classified by its relationship to reality events.
///
/// A single node can belong to multiple classes simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeClass {
    /// Origin Node: Generates primary events.
    /// Examples: Human, sensor, AI agent, simulation.
    /// Properties: private perception, local clock, cryptographic root, event genesis.
    Origin = 0b0001,

    /// Transmutation Node: Transforms the shape of reality.
    /// Examples: Codec, AI reconstructor, summarizer, emotion extractor.
    /// Properties: never source of truth, produces derivatives, preserves lineage.
    Transmutation = 0b0010,

    /// Propagation Node: Facilitates reality continuity.
    /// Examples: Relay, mesh router, buffer swarm, cache.
    /// Properties: no authority, no final state, replaceable, blind by default.
    Propagation = 0b0100,

    /// Witness Node: Gives meaning and memory.
    /// Examples: User device, archive, timeline builder, auditor.
    /// Properties: perspective-bound, builds projections, evaluates trust.
    Witness = 0b1000,
}

impl NodeClass {
    /// Get the class name
    pub fn name(&self) -> &'static str {
        match self {
            NodeClass::Origin => "Origin",
            NodeClass::Transmutation => "Transmutation",
            NodeClass::Propagation => "Propagation",
            NodeClass::Witness => "Witness",
        }
    }

    /// Get all classes
    pub fn all() -> &'static [NodeClass] {
        &[
            NodeClass::Origin,
            NodeClass::Transmutation,
            NodeClass::Propagation,
            NodeClass::Witness,
        ]
    }
}

/// A set of node classes (a node can belong to multiple classes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeClassSet(u8);

impl NodeClassSet {
    /// Empty set
    pub fn empty() -> Self {
        Self(0)
    }

    /// Add a class to the set
    pub fn with(self, class: NodeClass) -> Self {
        Self(self.0 | class as u8)
    }

    /// Check if the set contains a class
    pub fn contains(&self, class: NodeClass) -> bool {
        (self.0 & class as u8) != 0
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Common configurations
    pub fn smartphone() -> Self {
        Self::empty()
            .with(NodeClass::Origin)
            .with(NodeClass::Transmutation)
            .with(NodeClass::Witness)
    }

    pub fn relay_server() -> Self {
        Self::empty().with(NodeClass::Propagation)
    }

    pub fn ai_agent() -> Self {
        Self::empty()
            .with(NodeClass::Origin)
            .with(NodeClass::Transmutation)
            .with(NodeClass::Witness)
    }

    pub fn archive() -> Self {
        Self::empty()
            .with(NodeClass::Propagation)
            .with(NodeClass::Witness)
    }
}

impl BitOr for NodeClass {
    type Output = NodeClassSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        NodeClassSet::empty().with(self).with(rhs)
    }
}

impl BitOr<NodeClass> for NodeClassSet {
    type Output = NodeClassSet;

    fn bitor(self, rhs: NodeClass) -> Self::Output {
        self.with(rhs)
    }
}

// ============================================================================
// II. EVENT ALGEBRA (Conceptual - operators defined symbolically)
// ============================================================================

/// Event algebra operators.
///
/// These are symbolic representations of the algebraic operations on events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOperator {
    /// ⊕ Composition: Combining partial realities
    /// e₁ ⊕ e₂ → new experience (not necessarily commutative)
    Composition,

    /// ⊗ Transmutation: Shape change without changing origin
    /// f ⊗ e → e′ (voice → emotion → text → avatar motion)
    Transmutation,

    /// ≺ Causal Precedence: Reality dependency, not time
    /// e₁ ≺ e₂ → e₂ meaningless without e₁
    CausalPrecedence,

    /// ∥ Co-presence: Events coexisting though not synchronized
    /// e₁ ∥ e₂ → perceptual simultaneity
    CoPresence,

    /// ⊘ Degradation: Form reduction without meaning destruction
    /// e ⊘ δ → e′ (voice → noise → breath → silence)
    Degradation,
}

impl EventOperator {
    /// Get the mathematical symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            EventOperator::Composition => "⊕",
            EventOperator::Transmutation => "⊗",
            EventOperator::CausalPrecedence => "≺",
            EventOperator::CoPresence => "∥",
            EventOperator::Degradation => "⊘",
        }
    }

    /// Get the operator name
    pub fn name(&self) -> &'static str {
        match self {
            EventOperator::Composition => "Composition",
            EventOperator::Transmutation => "Transmutation",
            EventOperator::CausalPrecedence => "Causal Precedence",
            EventOperator::CoPresence => "Co-presence",
            EventOperator::Degradation => "Degradation",
        }
    }
}

// ============================================================================
// III. PRESENCE METRIC
// ============================================================================

/// Presence vector measuring "whether reality still feels alive".
///
/// ELARA does not measure quality with bitrate, FPS, jitter, or packet loss.
/// ELARA measures presence as a 5-dimensional vector.
///
/// P = ⟨L, I, C, R, E⟩
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresenceVector {
    /// L - Liveness: Is there still a sign of existence? [0.0 - 1.0]
    pub liveness: f32,

    /// I - Immediacy: How close to "now"? [0.0 - 1.0]
    pub immediacy: f32,

    /// C - Coherence: Can it still be understood? [0.0 - 1.0]
    pub coherence: f32,

    /// R - Relational Continuity: Does the relationship still feel intact? [0.0 - 1.0]
    pub relational_continuity: f32,

    /// E - Emotional Bandwidth: Is emotional meaning still carried? [0.0 - 1.0]
    pub emotional_bandwidth: f32,
}

impl PresenceVector {
    /// Create a new presence vector
    pub fn new(liveness: f32, immediacy: f32, coherence: f32, relational: f32, emotional: f32) -> Self {
        Self {
            liveness: liveness.clamp(0.0, 1.0),
            immediacy: immediacy.clamp(0.0, 1.0),
            coherence: coherence.clamp(0.0, 1.0),
            relational_continuity: relational.clamp(0.0, 1.0),
            emotional_bandwidth: emotional.clamp(0.0, 1.0),
        }
    }

    /// Full presence (all components at maximum)
    pub fn full() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0, 1.0)
    }

    /// Minimal presence (barely alive)
    pub fn minimal() -> Self {
        Self::new(0.1, 0.0, 0.0, 0.0, 0.0)
    }

    /// Zero presence (dead - should never happen in ELARA)
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0)
    }

    /// Calculate overall presence score [0.0 - 1.0]
    pub fn score(&self) -> f32 {
        (self.liveness + self.immediacy + self.coherence
            + self.relational_continuity + self.emotional_bandwidth) / 5.0
    }

    /// Check if presence is alive (any component > 0)
    pub fn is_alive(&self) -> bool {
        self.liveness > 0.0
            || self.immediacy > 0.0
            || self.coherence > 0.0
            || self.relational_continuity > 0.0
            || self.emotional_bandwidth > 0.0
    }

    /// Get the components as an array [L, I, C, R, E]
    pub fn as_array(&self) -> [f32; 5] {
        [
            self.liveness,
            self.immediacy,
            self.coherence,
            self.relational_continuity,
            self.emotional_bandwidth,
        ]
    }

    /// Get the minimum component value
    pub fn min_component(&self) -> f32 {
        self.as_array().into_iter().fold(f32::MAX, f32::min)
    }

    /// Get the maximum component value
    pub fn max_component(&self) -> f32 {
        self.as_array().into_iter().fold(f32::MIN, f32::max)
    }
}

impl Default for PresenceVector {
    fn default() -> Self {
        Self::full()
    }
}

impl fmt::Display for PresenceVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "P = ⟨L:{:.2}, I:{:.2}, C:{:.2}, R:{:.2}, E:{:.2}⟩ (score: {:.2})",
            self.liveness,
            self.immediacy,
            self.coherence,
            self.relational_continuity,
            self.emotional_bandwidth,
            self.score()
        )
    }
}

// ============================================================================
// IV. DEGRADATION LADDER
// ============================================================================

/// Degradation level in the ELARA reality ladder.
///
/// ELARA defines an official degradation ladder. No improvisation per engineer.
/// There is no "disconnected" - only the most minimal form of reality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DegradationLevel {
    /// L0: Full Perception - Continuous voice/video, rich semantics
    L0_FullPerception = 0,

    /// L1: Distorted Perception - Noise, blur, jitter, but continuous
    L1_DistortedPerception = 1,

    /// L2: Fragmented Perception - Chunks, drops, approximations
    L2_FragmentedPerception = 2,

    /// L3: Symbolic Presence - Text, emotion tokens, activity traces
    L3_SymbolicPresence = 3,

    /// L4: Minimal Presence - Breath, pulse, alive-signal, echoes
    L4_MinimalPresence = 4,

    /// L5: Latent Presence - Signed silence, delayed reality, memory
    L5_LatentPresence = 5,
}

impl DegradationLevel {
    /// Get the level number (0-5)
    pub fn level(&self) -> u8 {
        *self as u8
    }

    /// Get the level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::L0_FullPerception => "Full Perception",
            Self::L1_DistortedPerception => "Distorted Perception",
            Self::L2_FragmentedPerception => "Fragmented Perception",
            Self::L3_SymbolicPresence => "Symbolic Presence",
            Self::L4_MinimalPresence => "Minimal Presence",
            Self::L5_LatentPresence => "Latent Presence",
        }
    }

    /// Get the next lower level (more degraded)
    /// Returns None if already at L5 (cannot degrade further)
    pub fn degrade(&self) -> Option<Self> {
        match self {
            Self::L0_FullPerception => Some(Self::L1_DistortedPerception),
            Self::L1_DistortedPerception => Some(Self::L2_FragmentedPerception),
            Self::L2_FragmentedPerception => Some(Self::L3_SymbolicPresence),
            Self::L3_SymbolicPresence => Some(Self::L4_MinimalPresence),
            Self::L4_MinimalPresence => Some(Self::L5_LatentPresence),
            Self::L5_LatentPresence => None, // Cannot degrade further - this is the floor
        }
    }

    /// Get the next higher level (less degraded)
    /// Returns None if already at L0 (best quality)
    pub fn improve(&self) -> Option<Self> {
        match self {
            Self::L0_FullPerception => None, // Already at best
            Self::L1_DistortedPerception => Some(Self::L0_FullPerception),
            Self::L2_FragmentedPerception => Some(Self::L1_DistortedPerception),
            Self::L3_SymbolicPresence => Some(Self::L2_FragmentedPerception),
            Self::L4_MinimalPresence => Some(Self::L3_SymbolicPresence),
            Self::L5_LatentPresence => Some(Self::L4_MinimalPresence),
        }
    }

    /// Check if this level is worse than another
    pub fn is_worse_than(&self, other: Self) -> bool {
        self.level() > other.level()
    }

    /// Check if this level is better than another
    pub fn is_better_than(&self, other: Self) -> bool {
        self.level() < other.level()
    }

    /// Get all levels from best to worst
    pub fn all() -> &'static [Self] {
        &[
            Self::L0_FullPerception,
            Self::L1_DistortedPerception,
            Self::L2_FragmentedPerception,
            Self::L3_SymbolicPresence,
            Self::L4_MinimalPresence,
            Self::L5_LatentPresence,
        ]
    }
}

impl fmt::Display for DegradationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}: {}", self.level(), self.name())
    }
}

// ============================================================================
// V. CHAOS TEST SPECIFICATION
// ============================================================================

/// Category of chaos test.
///
/// ELARA must be tested with existential chaos, not just latency/throughput.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaosCategory {
    /// Ontological Chaos: Identity and reality coherence
    /// Tests: node identity change, forked history, AI hallucination, sensor spoofing
    Ontological,

    /// Temporal Chaos: Time model resilience
    /// Tests: massive clock skew, delayed days, brutal reordering, causal holes
    Temporal,

    /// Topological Chaos: Network resilience
    /// Tests: extended partition, swarm join/leave, mobile blackout, relay death
    Topological,

    /// Adversarial Chaos: Security model
    /// Tests: malicious injection, replay attack, perception poisoning, presence forgery
    Adversarial,

    /// Perceptual Chaos: Human experience continuity
    /// Tests: extreme jitter, half streams, emotional desync, semantic loss
    Perceptual,
}

impl ChaosCategory {
    /// Get the category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ontological => "Ontological Chaos",
            Self::Temporal => "Temporal Chaos",
            Self::Topological => "Topological Chaos",
            Self::Adversarial => "Adversarial Chaos",
            Self::Perceptual => "Perceptual Chaos",
        }
    }

    /// Get the minimum presence floor for this chaos type
    pub fn presence_floor(&self) -> DegradationLevel {
        match self {
            Self::Ontological => DegradationLevel::L4_MinimalPresence,
            Self::Temporal => DegradationLevel::L3_SymbolicPresence,
            Self::Topological => DegradationLevel::L5_LatentPresence,
            Self::Adversarial => DegradationLevel::L3_SymbolicPresence,
            Self::Perceptual => DegradationLevel::L2_FragmentedPerception,
        }
    }

    /// Get all categories
    pub fn all() -> &'static [Self] {
        &[
            Self::Ontological,
            Self::Temporal,
            Self::Topological,
            Self::Adversarial,
            Self::Perceptual,
        ]
    }
}

/// Success criteria for a chaos test.
#[derive(Debug, Clone)]
pub struct ChaosSuccessCriteria {
    /// Minimum presence that must be maintained
    pub min_presence: PresenceVector,

    /// Maximum degradation level allowed
    pub max_degradation: DegradationLevel,

    /// Lineage must remain intact
    pub lineage_intact: bool,

    /// Identity must remain continuous
    pub identity_continuous: bool,
}

impl ChaosSuccessCriteria {
    /// Default criteria: presence alive, lineage and identity intact
    pub fn default_for(category: ChaosCategory) -> Self {
        Self {
            min_presence: PresenceVector::minimal(),
            max_degradation: category.presence_floor(),
            lineage_intact: true,
            identity_continuous: true,
        }
    }
}

/// Result of a chaos test.
#[derive(Debug, Clone)]
pub struct ChaosTestResult {
    /// Did the test pass?
    pub passed: bool,

    /// Final presence vector
    pub presence: PresenceVector,

    /// Final degradation level
    pub level: DegradationLevel,

    /// Was lineage maintained?
    pub lineage_intact: bool,

    /// Was identity continuous?
    pub identity_continuous: bool,
}

impl ChaosTestResult {
    /// Check if the result meets the success criteria
    pub fn meets_criteria(&self, criteria: &ChaosSuccessCriteria) -> bool {
        self.presence.is_alive()
            && self.level <= criteria.max_degradation
            && (!criteria.lineage_intact || self.lineage_intact)
            && (!criteria.identity_continuous || self.identity_continuous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_class_set() {
        let smartphone = NodeClassSet::smartphone();
        assert!(smartphone.contains(NodeClass::Origin));
        assert!(smartphone.contains(NodeClass::Transmutation));
        assert!(smartphone.contains(NodeClass::Witness));
        assert!(!smartphone.contains(NodeClass::Propagation));

        let relay = NodeClassSet::relay_server();
        assert!(relay.contains(NodeClass::Propagation));
        assert!(!relay.contains(NodeClass::Origin));
    }

    #[test]
    fn test_presence_vector() {
        let full = PresenceVector::full();
        assert_eq!(full.score(), 1.0);
        assert!(full.is_alive());

        let minimal = PresenceVector::minimal();
        assert!(minimal.is_alive());
        assert!(minimal.score() < 0.1);

        let zero = PresenceVector::zero();
        assert!(!zero.is_alive());
    }

    #[test]
    fn test_degradation_ladder() {
        let mut level = DegradationLevel::L0_FullPerception;

        // Degrade through all levels
        let mut count = 0;
        while let Some(next) = level.degrade() {
            assert!(next.is_worse_than(level));
            level = next;
            count += 1;
        }
        assert_eq!(count, 5);
        assert_eq!(level, DegradationLevel::L5_LatentPresence);

        // Improve back up
        while let Some(prev) = level.improve() {
            assert!(prev.is_better_than(level));
            level = prev;
        }
        assert_eq!(level, DegradationLevel::L0_FullPerception);
    }

    #[test]
    fn test_chaos_categories() {
        for category in ChaosCategory::all() {
            let floor = category.presence_floor();
            // All floors should be valid degradation levels
            assert!(floor.level() <= 5);
        }
    }

    #[test]
    fn test_event_operators() {
        assert_eq!(EventOperator::Composition.symbol(), "⊕");
        assert_eq!(EventOperator::CausalPrecedence.symbol(), "≺");
        assert_eq!(EventOperator::Degradation.symbol(), "⊘");
    }

    #[test]
    fn test_presence_display() {
        let p = PresenceVector::new(0.9, 0.8, 0.7, 0.6, 0.5);
        let display = format!("{}", p);
        assert!(display.contains("L:0.90"));
        assert!(display.contains("score:"));
    }
}
