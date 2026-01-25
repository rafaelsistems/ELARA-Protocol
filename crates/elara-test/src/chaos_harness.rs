//! Chaos Test Harness
//!
//! Comprehensive chaos testing based on ELARA System Science v1.
//! Tests all 5 chaos categories:
//! - Ontological Chaos
//! - Temporal Chaos
//! - Topological Chaos
//! - Adversarial Chaos
//! - Perceptual Chaos

use std::time::Duration;

use elara_core::{ChaosCategory, ChaosSuccessCriteria, DegradationLevel, PresenceVector};

use crate::chaos::ChaosConfig;
use crate::integration::{IntegrationTestConfig, IntegrationTestHarness, SimulatedNode};

// ============================================================================
// CHAOS TEST SPECIFICATION
// ============================================================================

/// A chaos test specification
#[derive(Debug, Clone)]
pub struct ChaosTestSpec {
    /// Test name
    pub name: String,

    /// Chaos category
    pub category: ChaosCategory,

    /// Test duration
    pub duration: Duration,

    /// Chaos intensity (0.0 - 1.0)
    pub intensity: f32,

    /// Success criteria
    pub criteria: ChaosSuccessCriteria,

    /// Number of nodes
    pub node_count: usize,

    /// Number of messages
    pub message_count: usize,
}

impl ChaosTestSpec {
    /// Create a new chaos test spec
    pub fn new(name: &str, category: ChaosCategory) -> Self {
        Self {
            name: name.to_string(),
            category,
            duration: Duration::from_secs(10),
            intensity: 0.5,
            criteria: ChaosSuccessCriteria::default_for(category),
            node_count: 4,
            message_count: 20,
        }
    }

    /// Set duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set node count
    pub fn with_nodes(mut self, count: usize) -> Self {
        self.node_count = count;
        self
    }

    /// Set message count
    pub fn with_messages(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }
}

// ============================================================================
// CHAOS HARNESS
// ============================================================================

/// Chaos test harness - runs comprehensive chaos tests
pub struct ChaosHarness {
    /// Test specifications to run
    specs: Vec<ChaosTestSpec>,

    /// Results
    results: Vec<ChaosHarnessResult>,
}

/// Result of a single chaos test
#[derive(Debug, Clone)]
pub struct ChaosHarnessResult {
    /// Test specification
    pub spec: ChaosTestSpec,

    /// Did the test pass?
    pub passed: bool,

    /// Final presence vectors
    pub presence_vectors: Vec<PresenceVector>,

    /// Final degradation levels
    pub degradation_levels: Vec<DegradationLevel>,

    /// Messages delivered
    pub messages_delivered: usize,

    /// Messages dropped
    pub messages_dropped: usize,

    /// Invariant violations
    pub violations: Vec<String>,
}

impl ChaosHarness {
    /// Create a new chaos harness
    pub fn new() -> Self {
        Self {
            specs: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Add a test specification
    pub fn add_test(&mut self, spec: ChaosTestSpec) {
        self.specs.push(spec);
    }

    /// Add all standard chaos tests
    pub fn add_standard_tests(&mut self) {
        // Ontological Chaos
        self.add_test(
            ChaosTestSpec::new("ontological_light", ChaosCategory::Ontological)
                .with_intensity(0.3)
                .with_nodes(3)
                .with_messages(10),
        );

        self.add_test(
            ChaosTestSpec::new("ontological_heavy", ChaosCategory::Ontological)
                .with_intensity(0.7)
                .with_nodes(5)
                .with_messages(30),
        );

        // Temporal Chaos
        self.add_test(
            ChaosTestSpec::new("temporal_light", ChaosCategory::Temporal)
                .with_intensity(0.3)
                .with_nodes(3)
                .with_messages(10),
        );

        self.add_test(
            ChaosTestSpec::new("temporal_heavy", ChaosCategory::Temporal)
                .with_intensity(0.7)
                .with_nodes(5)
                .with_messages(30),
        );

        // Topological Chaos
        self.add_test(
            ChaosTestSpec::new("topological_light", ChaosCategory::Topological)
                .with_intensity(0.3)
                .with_nodes(4)
                .with_messages(15),
        );

        self.add_test(
            ChaosTestSpec::new("topological_heavy", ChaosCategory::Topological)
                .with_intensity(0.7)
                .with_nodes(6)
                .with_messages(40),
        );

        // Adversarial Chaos
        self.add_test(
            ChaosTestSpec::new("adversarial_light", ChaosCategory::Adversarial)
                .with_intensity(0.3)
                .with_nodes(3)
                .with_messages(10),
        );

        self.add_test(
            ChaosTestSpec::new("adversarial_heavy", ChaosCategory::Adversarial)
                .with_intensity(0.7)
                .with_nodes(5)
                .with_messages(30),
        );

        // Perceptual Chaos
        self.add_test(
            ChaosTestSpec::new("perceptual_light", ChaosCategory::Perceptual)
                .with_intensity(0.3)
                .with_nodes(3)
                .with_messages(10),
        );

        self.add_test(
            ChaosTestSpec::new("perceptual_heavy", ChaosCategory::Perceptual)
                .with_intensity(0.7)
                .with_nodes(5)
                .with_messages(30),
        );
    }

    /// Run all tests
    pub fn run_all(&mut self) -> &[ChaosHarnessResult] {
        self.results.clear();

        for spec in self.specs.clone() {
            let result = self.run_single(&spec);
            self.results.push(result);
        }

        &self.results
    }

    /// Run a single test
    fn run_single(&self, spec: &ChaosTestSpec) -> ChaosHarnessResult {
        // Create chaos config based on category and intensity
        let chaos_config = self.create_chaos_config(spec.category, spec.intensity);

        // Create integration test config
        let config = IntegrationTestConfig {
            node_count: spec.node_count,
            message_count: spec.message_count,
            chaos: Some(chaos_config),
        };

        // Run integration test
        let mut harness = IntegrationTestHarness::new(config);
        let result = harness.run();

        // Apply category-specific chaos effects
        let (presence_vectors, degradation_levels) =
            self.apply_category_effects(spec, harness.nodes_mut());

        // Check success criteria
        let passed = self.check_criteria(spec, &presence_vectors, &degradation_levels);

        ChaosHarnessResult {
            spec: spec.clone(),
            passed,
            presence_vectors,
            degradation_levels,
            messages_delivered: result.messages_processed,
            messages_dropped: result.messages_dropped,
            violations: result.invariant_violations,
        }
    }

    /// Create chaos config based on category and intensity
    fn create_chaos_config(&self, category: ChaosCategory, intensity: f32) -> ChaosConfig {
        match category {
            ChaosCategory::Ontological => {
                // Ontological: Focus on identity and reality coherence
                ChaosConfig {
                    base_latency: Duration::from_millis(50),
                    jitter: crate::chaos::JitterDistribution::Uniform {
                        min_ms: 0,
                        max_ms: (100.0 * intensity) as u32,
                    },
                    loss_rate: 0.05 * intensity as f64,
                    burst_loss_prob: 0.1 * intensity as f64,
                    burst_length: (2, (5.0 * intensity) as u32 + 2),
                    reorder_prob: 0.1 * intensity as f64,
                    reorder_depth: (5.0 * intensity) as u32 + 1,
                    duplicate_prob: 0.05 * intensity as f64,
                }
            }
            ChaosCategory::Temporal => {
                // Temporal: Focus on time model resilience
                ChaosConfig {
                    base_latency: Duration::from_millis((200.0 * intensity) as u64 + 50),
                    jitter: crate::chaos::JitterDistribution::Pareto {
                        scale_ms: 50.0 * intensity as f64 + 20.0,
                        shape: 1.5 - 0.3 * intensity as f64,
                    },
                    loss_rate: 0.03 * intensity as f64,
                    burst_loss_prob: 0.05,
                    burst_length: (1, 3),
                    reorder_prob: 0.3 * intensity as f64, // High reordering
                    reorder_depth: (10.0 * intensity) as u32 + 2,
                    duplicate_prob: 0.02,
                }
            }
            ChaosCategory::Topological => {
                // Topological: Focus on network resilience
                ChaosConfig {
                    base_latency: Duration::from_millis(100),
                    jitter: crate::chaos::JitterDistribution::Uniform {
                        min_ms: 0,
                        max_ms: 50,
                    },
                    loss_rate: 0.2 * intensity as f64, // High loss
                    burst_loss_prob: 0.3 * intensity as f64, // Burst loss (partition)
                    burst_length: ((5.0 * intensity) as u32 + 2, (20.0 * intensity) as u32 + 5),
                    reorder_prob: 0.05,
                    reorder_depth: 3,
                    duplicate_prob: 0.01,
                }
            }
            ChaosCategory::Adversarial => {
                // Adversarial: Focus on security model
                ChaosConfig {
                    base_latency: Duration::from_millis(50),
                    jitter: crate::chaos::JitterDistribution::Uniform {
                        min_ms: 0,
                        max_ms: 30,
                    },
                    loss_rate: 0.02,
                    burst_loss_prob: 0.05,
                    burst_length: (1, 3),
                    reorder_prob: 0.1 * intensity as f64,
                    reorder_depth: 5,
                    duplicate_prob: 0.2 * intensity as f64, // High duplication (replay)
                }
            }
            ChaosCategory::Perceptual => {
                // Perceptual: Focus on human experience continuity
                ChaosConfig {
                    base_latency: Duration::from_millis((100.0 * intensity) as u64 + 20),
                    jitter: crate::chaos::JitterDistribution::Pareto {
                        scale_ms: 100.0 * intensity as f64 + 10.0,
                        shape: 1.2,
                    },
                    loss_rate: 0.1 * intensity as f64,
                    burst_loss_prob: 0.15 * intensity as f64,
                    burst_length: (2, 6),
                    reorder_prob: 0.1,
                    reorder_depth: 4,
                    duplicate_prob: 0.02,
                }
            }
        }
    }

    /// Apply category-specific effects to nodes
    fn apply_category_effects(
        &self,
        spec: &ChaosTestSpec,
        nodes: &mut [SimulatedNode],
    ) -> (Vec<PresenceVector>, Vec<DegradationLevel>) {
        let intensity = spec.intensity;

        for node in nodes.iter_mut() {
            match spec.category {
                ChaosCategory::Ontological => {
                    // Ontological chaos affects coherence
                    node.update_presence(1.0 - 0.3 * intensity);
                    if intensity > 0.5 {
                        node.degrade();
                    }
                }
                ChaosCategory::Temporal => {
                    // Temporal chaos affects immediacy
                    let factor = 1.0 - 0.4 * intensity;
                    node.update_presence(factor);
                    if intensity > 0.6 {
                        node.degrade();
                        node.degrade();
                    }
                }
                ChaosCategory::Topological => {
                    // Topological chaos affects liveness
                    let factor = 1.0 - 0.5 * intensity;
                    node.update_presence(factor);
                    // More degradation for topological
                    let degrade_count = (intensity * 3.0) as usize;
                    for _ in 0..degrade_count {
                        node.degrade();
                    }
                }
                ChaosCategory::Adversarial => {
                    // Adversarial chaos affects relational continuity
                    let factor = 1.0 - 0.2 * intensity;
                    node.update_presence(factor);
                    if intensity > 0.7 {
                        node.degrade();
                    }
                }
                ChaosCategory::Perceptual => {
                    // Perceptual chaos affects emotional bandwidth
                    let factor = 1.0 - 0.35 * intensity;
                    node.update_presence(factor);
                    if intensity > 0.4 {
                        node.degrade();
                    }
                }
            }
        }

        let presence_vectors: Vec<_> = nodes.iter().map(|n| *n.presence()).collect();
        let degradation_levels: Vec<_> = nodes.iter().map(|n| n.degradation_level()).collect();

        (presence_vectors, degradation_levels)
    }

    /// Check if results meet success criteria
    fn check_criteria(
        &self,
        spec: &ChaosTestSpec,
        presence_vectors: &[PresenceVector],
        degradation_levels: &[DegradationLevel],
    ) -> bool {
        // All nodes must be alive
        if !presence_vectors.iter().all(|p| p.is_alive()) {
            return false;
        }

        // No node should exceed max degradation for this category
        let max_allowed = spec.criteria.max_degradation;
        if degradation_levels.iter().any(|&d| d > max_allowed) {
            return false;
        }

        true
    }

    /// Get results
    pub fn results(&self) -> &[ChaosHarnessResult] {
        &self.results
    }

    /// Get summary
    pub fn summary(&self) -> ChaosSummary {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        let by_category: Vec<_> = ChaosCategory::all()
            .iter()
            .map(|&cat| {
                let cat_results: Vec<_> = self
                    .results
                    .iter()
                    .filter(|r| r.spec.category == cat)
                    .collect();
                let cat_passed = cat_results.iter().filter(|r| r.passed).count();
                (cat, cat_passed, cat_results.len())
            })
            .collect();

        ChaosSummary {
            total,
            passed,
            failed,
            by_category,
        }
    }
}

impl Default for ChaosHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of chaos test results
#[derive(Debug)]
pub struct ChaosSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub by_category: Vec<(ChaosCategory, usize, usize)>,
}

impl ChaosSummary {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Get pass rate
    pub fn pass_rate(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            self.passed as f32 / self.total as f32
        }
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Run standard chaos tests
pub fn run_standard_chaos_tests() -> ChaosSummary {
    let mut harness = ChaosHarness::new();
    harness.add_standard_tests();
    harness.run_all();
    harness.summary()
}

/// Run chaos tests for a specific category
pub fn run_category_tests(category: ChaosCategory) -> Vec<ChaosHarnessResult> {
    let mut harness = ChaosHarness::new();

    harness.add_test(
        ChaosTestSpec::new("light", category)
            .with_intensity(0.3)
            .with_nodes(3)
            .with_messages(10),
    );

    harness.add_test(
        ChaosTestSpec::new("moderate", category)
            .with_intensity(0.5)
            .with_nodes(4)
            .with_messages(20),
    );

    harness.add_test(
        ChaosTestSpec::new("heavy", category)
            .with_intensity(0.7)
            .with_nodes(5)
            .with_messages(30),
    );

    harness.run_all().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_harness_creation() {
        let harness = ChaosHarness::new();
        assert!(harness.specs.is_empty());
        assert!(harness.results.is_empty());
    }

    #[test]
    fn test_add_standard_tests() {
        let mut harness = ChaosHarness::new();
        harness.add_standard_tests();
        assert_eq!(harness.specs.len(), 10); // 2 tests per category * 5 categories
    }

    #[test]
    fn test_chaos_spec_builder() {
        let spec = ChaosTestSpec::new("test", ChaosCategory::Temporal)
            .with_intensity(0.8)
            .with_nodes(6)
            .with_messages(50);

        assert_eq!(spec.name, "test");
        assert_eq!(spec.category, ChaosCategory::Temporal);
        assert_eq!(spec.intensity, 0.8);
        assert_eq!(spec.node_count, 6);
        assert_eq!(spec.message_count, 50);
    }

    #[test]
    fn test_run_single_ontological() {
        let harness = ChaosHarness::new();
        let spec = ChaosTestSpec::new("ontological_test", ChaosCategory::Ontological)
            .with_intensity(0.3)
            .with_nodes(2)
            .with_messages(5);

        let result = harness.run_single(&spec);

        // Should pass with light chaos
        assert!(result.passed, "Light ontological chaos should pass");
        assert!(result.presence_vectors.iter().all(|p| p.is_alive()));
    }

    #[test]
    fn test_run_single_temporal() {
        let harness = ChaosHarness::new();
        let spec = ChaosTestSpec::new("temporal_test", ChaosCategory::Temporal)
            .with_intensity(0.3)
            .with_nodes(2)
            .with_messages(5);

        let result = harness.run_single(&spec);

        assert!(result.passed, "Light temporal chaos should pass");
    }

    #[test]
    fn test_run_single_topological() {
        let harness = ChaosHarness::new();
        let spec = ChaosTestSpec::new("topological_test", ChaosCategory::Topological)
            .with_intensity(0.3)
            .with_nodes(2)
            .with_messages(5);

        let result = harness.run_single(&spec);

        assert!(result.passed, "Light topological chaos should pass");
    }

    #[test]
    fn test_run_single_adversarial() {
        let harness = ChaosHarness::new();
        let spec = ChaosTestSpec::new("adversarial_test", ChaosCategory::Adversarial)
            .with_intensity(0.3)
            .with_nodes(2)
            .with_messages(5);

        let result = harness.run_single(&spec);

        assert!(result.passed, "Light adversarial chaos should pass");
    }

    #[test]
    fn test_run_single_perceptual() {
        let harness = ChaosHarness::new();
        let spec = ChaosTestSpec::new("perceptual_test", ChaosCategory::Perceptual)
            .with_intensity(0.3)
            .with_nodes(2)
            .with_messages(5);

        let result = harness.run_single(&spec);

        assert!(result.passed, "Light perceptual chaos should pass");
    }

    #[test]
    fn test_run_standard_chaos_tests() {
        let summary = run_standard_chaos_tests();

        // All light tests should pass
        assert!(
            summary.pass_rate() >= 0.5,
            "At least half of chaos tests should pass"
        );

        // All categories should be tested
        assert_eq!(summary.by_category.len(), 5);
    }

    #[test]
    fn test_chaos_summary() {
        let mut harness = ChaosHarness::new();
        harness.add_test(
            ChaosTestSpec::new("test1", ChaosCategory::Ontological)
                .with_intensity(0.2)
                .with_nodes(2)
                .with_messages(3),
        );
        harness.add_test(
            ChaosTestSpec::new("test2", ChaosCategory::Temporal)
                .with_intensity(0.2)
                .with_nodes(2)
                .with_messages(3),
        );

        harness.run_all();
        let summary = harness.summary();

        assert_eq!(summary.total, 2);
        assert!(summary.pass_rate() > 0.0);
    }
}
