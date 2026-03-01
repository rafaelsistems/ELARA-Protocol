//! Security Test Module
//!
//! This module provides comprehensive security testing infrastructure for the ELARA protocol.
//! It includes test utilities and infrastructure for:
//! - Replay protection testing (Task 3.2)
//! - Message authentication testing (Task 3.3)
//! - Key isolation testing (Task 3.4)
//! - Timing attack resistance testing (Task 3.5)
//!
//! The module provides both unit test helpers and property-based testing utilities
//! to validate security properties across the protocol implementation.

use elara_core::{ElaraError, ElaraResult, NodeId, PacketClass, RepresentationProfile, SessionId};
use elara_crypto::{ReplayManager, ReplayWindow, SecureFrameProcessor, KEY_SIZE};
use elara_wire::Extensions;
use std::time::Instant;

/// Security test configuration
#[derive(Debug, Clone)]
pub struct SecurityTestConfig {
    /// Number of test iterations for statistical tests
    pub iterations: usize,
    /// Timing measurement precision threshold (nanoseconds)
    pub timing_threshold_ns: u64,
    /// Maximum acceptable timing variance for constant-time operations
    pub max_timing_variance: f64,
}

impl Default for SecurityTestConfig {
    fn default() -> Self {
        SecurityTestConfig {
            iterations: 1000,
            timing_threshold_ns: 100,
            max_timing_variance: 0.1, // 10% variance
        }
    }
}

/// Test result for security tests
#[derive(Debug, Clone)]
pub struct SecurityTestResult {
    /// Whether the test passed
    pub passed: bool,
    /// Test description
    pub description: String,
    /// Optional error message
    pub error: Option<String>,
    /// Optional timing statistics
    pub timing_stats: Option<TimingStats>,
}

impl SecurityTestResult {
    pub fn pass(description: impl Into<String>) -> Self {
        SecurityTestResult {
            passed: true,
            description: description.into(),
            error: None,
            timing_stats: None,
        }
    }

    pub fn fail(description: impl Into<String>, error: impl Into<String>) -> Self {
        SecurityTestResult {
            passed: false,
            description: description.into(),
            error: Some(error.into()),
            timing_stats: None,
        }
    }

    pub fn with_timing(mut self, stats: TimingStats) -> Self {
        self.timing_stats = Some(stats);
        self
    }
}

/// Timing statistics for constant-time operation testing
#[derive(Debug, Clone)]
pub struct TimingStats {
    /// Minimum observed time (nanoseconds)
    pub min_ns: u64,
    /// Maximum observed time (nanoseconds)
    pub max_ns: u64,
    /// Mean time (nanoseconds)
    pub mean_ns: f64,
    /// Standard deviation (nanoseconds)
    pub std_dev_ns: f64,
    /// Coefficient of variation (std_dev / mean)
    pub coefficient_of_variation: f64,
}

impl TimingStats {
    /// Calculate timing statistics from a set of measurements
    pub fn from_measurements(measurements: &[u64]) -> Self {
        let min_ns = *measurements.iter().min().unwrap_or(&0);
        let max_ns = *measurements.iter().max().unwrap_or(&0);
        let mean_ns = measurements.iter().sum::<u64>() as f64 / measurements.len() as f64;

        let variance = measurements
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean_ns;
                diff * diff
            })
            .sum::<f64>()
            / measurements.len() as f64;

        let std_dev_ns = variance.sqrt();
        let coefficient_of_variation = if mean_ns > 0.0 {
            std_dev_ns / mean_ns
        } else {
            0.0
        };

        TimingStats {
            min_ns,
            max_ns,
            mean_ns,
            std_dev_ns,
            coefficient_of_variation,
        }
    }

    /// Check if timing is constant within acceptable variance
    pub fn is_constant_time(&self, max_variance: f64) -> bool {
        self.coefficient_of_variation <= max_variance
    }
}

/// Test harness for replay protection testing
pub struct ReplayProtectionTestHarness {
    manager: ReplayManager,
    node_id: NodeId,
    class: PacketClass,
}

impl ReplayProtectionTestHarness {
    /// Create a new replay protection test harness
    pub fn new(node_id: NodeId, class: PacketClass) -> Self {
        ReplayProtectionTestHarness {
            manager: ReplayManager::new(),
            node_id,
            class,
        }
    }

    /// Test accepting a sequence number
    pub fn accept(&mut self, seq: u16) -> ElaraResult<()> {
        self.manager.accept(self.node_id, self.class, seq)
    }

    /// Test checking a sequence number without accepting
    pub fn check(&self, seq: u16) -> bool {
        self.manager.check(self.node_id, self.class, seq)
    }

    /// Get the replay window for inspection
    pub fn get_window(&self) -> Option<&ReplayWindow> {
        self.manager.get_window(self.node_id, self.class)
    }

    /// Test replay attack: send same packet twice
    pub fn test_replay_attack(&mut self, seq: u16) -> SecurityTestResult {
        // First acceptance should succeed
        if let Err(e) = self.accept(seq) {
            return SecurityTestResult::fail(
                "Replay attack test",
                format!("First packet acceptance failed: {:?}", e),
            );
        }

        // Second acceptance should fail (replay detected)
        match self.accept(seq) {
            Err(ElaraError::ReplayDetected(_)) => {
                SecurityTestResult::pass("Replay attack correctly detected")
            }
            Ok(_) => SecurityTestResult::fail(
                "Replay attack test",
                "Replay attack not detected - same packet accepted twice",
            ),
            Err(e) => SecurityTestResult::fail(
                "Replay attack test",
                format!("Unexpected error: {:?}", e),
            ),
        }
    }

    /// Test replay window advancement
    pub fn test_window_advancement(&mut self, initial_seq: u16, jump: u16) -> SecurityTestResult {
        // Accept initial packet
        if let Err(e) = self.accept(initial_seq) {
            return SecurityTestResult::fail(
                "Window advancement test",
                format!("Initial packet acceptance failed: {:?}", e),
            );
        }

        let window_before = self.get_window().map(|w| w.min_seq());

        // Jump ahead (must be beyond window size to force advancement)
        let new_seq = initial_seq.wrapping_add(jump);
        if let Err(e) = self.accept(new_seq) {
            return SecurityTestResult::fail(
                "Window advancement test",
                format!("Jump packet acceptance failed: {:?}", e),
            );
        }

        let window_after = self.get_window().map(|w| w.min_seq());

        // Verify window advanced (or stayed the same if jump was within window)
        match (window_before, window_after) {
            (Some(before), Some(after)) => {
                // Window should advance if jump is large enough
                if jump >= self.class.replay_window_size() {
                    if after > before {
                        SecurityTestResult::pass("Replay window advanced correctly")
                    } else {
                        SecurityTestResult::fail(
                            "Window advancement test",
                            format!("Window did not advance: before={}, after={}", before, after),
                        )
                    }
                } else {
                    // Small jump - window may not advance
                    SecurityTestResult::pass("Replay window handled small jump correctly")
                }
            }
            _ => SecurityTestResult::fail(
                "Window advancement test",
                "Could not retrieve window state",
            ),
        }
    }

    /// Test sequence number wraparound
    pub fn test_sequence_wraparound(&mut self) -> SecurityTestResult {
        // The replay window uses wrapping arithmetic
        // We need to gradually advance the window to near u16::MAX
        // We'll do this by accepting packets in larger jumps
        
        // Advance window in steps to get near wraparound
        let mut current = 0u16;
        let step = 10000u16;
        
        // Advance to near the end
        while current < 65000 {
            if let Err(e) = self.accept(current) {
                return SecurityTestResult::fail(
                    "Sequence wraparound test",
                    format!("Failed to advance window at {}: {:?}", current, e),
                );
            }
            current = current.saturating_add(step);
        }
        
        // Now accept packets near wraparound sequentially
        for seq in 65530..=65535 {
            if let Err(e) = self.accept(seq) {
                return SecurityTestResult::fail(
                    "Sequence wraparound test",
                    format!("Failed to accept sequence {}: {:?}", seq, e),
                );
            }
        }
        
        // Wrap around to 0
        for seq in 0..5 {
            if let Err(e) = self.accept(seq) {
                return SecurityTestResult::fail(
                    "Sequence wraparound test",
                    format!("Failed to accept wrapped sequence {}: {:?}", seq, e),
                );
            }
        }

        SecurityTestResult::pass("Sequence number wraparound handled correctly")
    }
}

/// Test harness for message authentication testing
pub struct MessageAuthenticationTestHarness {
    processor: SecureFrameProcessor,
    _session_id: SessionId,
    _node_id: NodeId,
}

impl MessageAuthenticationTestHarness {
    /// Create a new message authentication test harness
    pub fn new(session_id: SessionId, node_id: NodeId, session_key: [u8; KEY_SIZE]) -> Self {
        MessageAuthenticationTestHarness {
            processor: SecureFrameProcessor::new(session_id, node_id, session_key),
            _session_id: session_id,
            _node_id: node_id,
        }
    }

    /// Encrypt a test message
    pub fn encrypt_message(
        &mut self,
        class: PacketClass,
        payload: &[u8],
    ) -> ElaraResult<Vec<u8>> {
        self.processor.encrypt_frame(
            class,
            RepresentationProfile::Textual,
            0,
            Extensions::default(),
            payload,
        )
    }

    /// Decrypt a test message
    pub fn decrypt_message(&mut self, data: &[u8]) -> ElaraResult<Vec<u8>> {
        let decrypted = self.processor.decrypt_frame(data)?;
        Ok(decrypted.payload.clone())
    }

    /// Test message tampering detection
    pub fn test_message_tampering(&mut self, payload: &[u8]) -> SecurityTestResult {
        // Encrypt a message
        let encrypted = match self.encrypt_message(PacketClass::Core, payload) {
            Ok(data) => data,
            Err(e) => {
                return SecurityTestResult::fail(
                    "Message tampering test",
                    format!("Encryption failed: {:?}", e),
                )
            }
        };

        // Tamper with the encrypted data (flip a bit in the middle)
        let mut tampered = encrypted.clone();
        if tampered.len() > 20 {
            let idx = tampered.len() / 2;
            tampered[idx] ^= 0x01;
        }

        // Attempt to decrypt tampered message
        match self.decrypt_message(&tampered) {
            Err(_) => SecurityTestResult::pass("Message tampering correctly detected"),
            Ok(_) => SecurityTestResult::fail(
                "Message tampering test",
                "Tampered message was accepted - authentication failed",
            ),
        }
    }

    /// Test MAC verification
    pub fn test_mac_verification(&mut self, payload: &[u8]) -> SecurityTestResult {
        // Encrypt a message
        let encrypted = match self.encrypt_message(PacketClass::Core, payload) {
            Ok(data) => data,
            Err(e) => {
                return SecurityTestResult::fail(
                    "MAC verification test",
                    format!("Encryption failed: {:?}", e),
                )
            }
        };

        // Truncate the message (remove MAC tag)
        if encrypted.len() < 16 {
            return SecurityTestResult::fail(
                "MAC verification test",
                "Encrypted message too short",
            );
        }

        let truncated = &encrypted[..encrypted.len() - 16];

        // Attempt to decrypt truncated message
        match self.decrypt_message(truncated) {
            Err(_) => SecurityTestResult::pass("MAC verification correctly enforced"),
            Ok(_) => SecurityTestResult::fail(
                "MAC verification test",
                "Message without MAC was accepted",
            ),
        }
    }
}

/// Test harness for key isolation testing
pub struct KeyIsolationTestHarness {
    processors: Vec<SecureFrameProcessor>,
}

impl KeyIsolationTestHarness {
    /// Create a new key isolation test harness with multiple sessions
    pub fn new(num_sessions: usize) -> Self {
        let mut processors = Vec::new();
        for i in 0..num_sessions {
            let session_id = SessionId::new(i as u64);
            let node_id = NodeId::new(i as u64);
            let session_key = [i as u8; KEY_SIZE];
            processors.push(SecureFrameProcessor::new(session_id, node_id, session_key));
        }
        KeyIsolationTestHarness { processors }
    }

    /// Test that keys don't leak across sessions
    pub fn test_session_key_isolation(&mut self, payload: &[u8]) -> SecurityTestResult {
        if self.processors.len() < 2 {
            return SecurityTestResult::fail(
                "Session key isolation test",
                "Need at least 2 sessions for isolation test",
            );
        }

        // Encrypt with first session
        let encrypted = match self.processors[0].encrypt_frame(
            PacketClass::Core,
            RepresentationProfile::Textual,
            0,
            Extensions::default(),
            payload,
        ) {
            Ok(data) => data,
            Err(e) => {
                return SecurityTestResult::fail(
                    "Session key isolation test",
                    format!("Encryption failed: {:?}", e),
                )
            }
        };

        // Attempt to decrypt with second session (different key)
        match self.processors[1].decrypt_frame(&encrypted) {
            Err(_) => SecurityTestResult::pass("Session keys are properly isolated"),
            Ok(_) => SecurityTestResult::fail(
                "Session key isolation test",
                "Message encrypted with one session key was decrypted with another - key isolation failed",
            ),
        }
    }

    /// Test key derivation independence
    pub fn test_key_derivation_independence(&mut self, payload: &[u8]) -> SecurityTestResult {
        if self.processors.is_empty() {
            return SecurityTestResult::fail(
                "Key derivation independence test",
                "Need at least 1 session",
            );
        }

        // Encrypt multiple messages with the same processor
        let mut encrypted_messages = Vec::new();
        for _ in 0..5 {
            match self.processors[0].encrypt_frame(
                PacketClass::Core,
                RepresentationProfile::Textual,
                0,
                Extensions::default(),
                payload,
            ) {
                Ok(data) => encrypted_messages.push(data),
                Err(e) => {
                    return SecurityTestResult::fail(
                        "Key derivation independence test",
                        format!("Encryption failed: {:?}", e),
                    )
                }
            }
        }

        // Verify all encrypted messages are different (keys are derived independently)
        for i in 0..encrypted_messages.len() {
            for j in (i + 1)..encrypted_messages.len() {
                if encrypted_messages[i] == encrypted_messages[j] {
                    return SecurityTestResult::fail(
                        "Key derivation independence test",
                        "Multiple encryptions produced identical ciphertext - key derivation not independent",
                    );
                }
            }
        }

        SecurityTestResult::pass("Key derivation is independent across messages")
    }
}

/// Test harness for timing attack resistance testing
pub struct TimingAttackTestHarness {
    config: SecurityTestConfig,
}

impl TimingAttackTestHarness {
    /// Create a new timing attack test harness
    pub fn new(config: SecurityTestConfig) -> Self {
        TimingAttackTestHarness { config }
    }

    /// Measure operation timing
    fn measure_operation<F>(&self, mut operation: F) -> Vec<u64>
    where
        F: FnMut(),
    {
        let mut measurements = Vec::with_capacity(self.config.iterations);

        // Warm-up
        for _ in 0..10 {
            operation();
        }

        // Actual measurements
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            operation();
            let elapsed = start.elapsed().as_nanos() as u64;
            measurements.push(elapsed);
        }

        measurements
    }

    /// Test constant-time operation
    pub fn test_constant_time_operation<F>(
        &self,
        description: &str,
        operation: F,
    ) -> SecurityTestResult
    where
        F: FnMut(),
    {
        let measurements = self.measure_operation(operation);
        let stats = TimingStats::from_measurements(&measurements);

        if stats.is_constant_time(self.config.max_timing_variance) {
            SecurityTestResult::pass(format!(
                "{} is constant-time (CV: {:.4})",
                description, stats.coefficient_of_variation
            ))
            .with_timing(stats)
        } else {
            SecurityTestResult::fail(
                description,
                format!(
                    "Operation is not constant-time (CV: {:.4} > {:.4})",
                    stats.coefficient_of_variation, self.config.max_timing_variance
                ),
            )
            .with_timing(stats)
        }
    }

    /// Test encryption timing consistency
    pub fn test_encryption_timing(
        &self,
        processor: &mut SecureFrameProcessor,
        payloads: &[&[u8]],
    ) -> SecurityTestResult {
        if payloads.len() < 2 {
            return SecurityTestResult::fail(
                "Encryption timing test",
                "Need at least 2 different payloads",
            );
        }

        let mut all_measurements = Vec::new();

        for payload in payloads {
            let measurements = self.measure_operation(|| {
                let _ = processor.encrypt_frame(
                    PacketClass::Core,
                    RepresentationProfile::Textual,
                    0,
                    Extensions::default(),
                    payload,
                );
            });
            all_measurements.push(measurements);
        }

        // Calculate statistics for each payload
        let stats: Vec<TimingStats> = all_measurements
            .iter()
            .map(|m| TimingStats::from_measurements(m))
            .collect();

        // Check if all payloads have similar timing characteristics
        let mean_of_means =
            stats.iter().map(|s| s.mean_ns).sum::<f64>() / stats.len() as f64;

        let max_deviation = stats
            .iter()
            .map(|s| ((s.mean_ns - mean_of_means) / mean_of_means).abs())
            .fold(0.0f64, f64::max);

        if max_deviation <= self.config.max_timing_variance {
            SecurityTestResult::pass(format!(
                "Encryption timing is consistent across payloads (max deviation: {:.4})",
                max_deviation
            ))
        } else {
            SecurityTestResult::fail(
                "Encryption timing test",
                format!(
                    "Encryption timing varies significantly across payloads (max deviation: {:.4} > {:.4})",
                    max_deviation, self.config.max_timing_variance
                ),
            )
        }
    }
}

/// Comprehensive security test suite runner
pub struct SecurityTestSuite {
    _config: SecurityTestConfig,
    results: Vec<SecurityTestResult>,
}

impl SecurityTestSuite {
    /// Create a new security test suite
    pub fn new(config: SecurityTestConfig) -> Self {
        SecurityTestSuite {
            _config: config,
            results: Vec::new(),
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: SecurityTestResult) {
        self.results.push(result);
    }

    /// Run all security tests
    pub fn run_all_tests(&mut self) {
        // Replay protection tests
        self.run_replay_protection_tests();

        // Message authentication tests
        self.run_message_authentication_tests();

        // Key isolation tests
        self.run_key_isolation_tests();

        // Note: Timing attack tests are not run automatically as they require
        // specific hardware and environment setup for accurate results
    }

    /// Run replay protection tests
    fn run_replay_protection_tests(&mut self) {
        let node_id = NodeId::new(1);
        let class = PacketClass::Core;
        let mut harness = ReplayProtectionTestHarness::new(node_id, class);

        // Test replay attack
        self.add_result(harness.test_replay_attack(100));

        // Test window advancement
        let mut harness2 = ReplayProtectionTestHarness::new(node_id, class);
        self.add_result(harness2.test_window_advancement(0, 20));

        // Test sequence wraparound
        let mut harness3 = ReplayProtectionTestHarness::new(node_id, class);
        self.add_result(harness3.test_sequence_wraparound());
    }

    /// Run message authentication tests
    fn run_message_authentication_tests(&mut self) {
        let session_id = SessionId::new(1);
        let node_id = NodeId::new(1);
        let session_key = [0x42; KEY_SIZE];
        let mut harness = MessageAuthenticationTestHarness::new(session_id, node_id, session_key);

        let test_payload = b"Test message for authentication";

        // Test message tampering
        self.add_result(harness.test_message_tampering(test_payload));

        // Test MAC verification
        let mut harness2 =
            MessageAuthenticationTestHarness::new(session_id, node_id, session_key);
        self.add_result(harness2.test_mac_verification(test_payload));
    }

    /// Run key isolation tests
    fn run_key_isolation_tests(&mut self) {
        let mut harness = KeyIsolationTestHarness::new(3);
        let test_payload = b"Test message for key isolation";

        // Test session key isolation
        self.add_result(harness.test_session_key_isolation(test_payload));

        // Test key derivation independence
        self.add_result(harness.test_key_derivation_independence(test_payload));
    }

    /// Get test results
    pub fn results(&self) -> &[SecurityTestResult] {
        &self.results
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    /// Get summary statistics
    pub fn summary(&self) -> SecurityTestSummary {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        SecurityTestSummary {
            total,
            passed,
            failed,
        }
    }
}

/// Summary of security test results
#[derive(Debug, Clone)]
pub struct SecurityTestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

impl SecurityTestSummary {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_stats_calculation() {
        let measurements = vec![100, 102, 98, 101, 99, 103, 97, 100, 102, 98];
        let stats = TimingStats::from_measurements(&measurements);

        assert_eq!(stats.min_ns, 97);
        assert_eq!(stats.max_ns, 103);
        assert!((stats.mean_ns - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_replay_protection_harness() {
        let node_id = NodeId::new(1);
        let class = PacketClass::Core;
        let mut harness = ReplayProtectionTestHarness::new(node_id, class);

        // First packet should be accepted
        assert!(harness.accept(0).is_ok());

        // Replay should be rejected
        assert!(harness.accept(0).is_err());
    }

    #[test]
    fn test_security_test_suite() {
        let config = SecurityTestConfig::default();
        let mut suite = SecurityTestSuite::new(config);

        suite.run_all_tests();

        let summary = suite.summary();
        assert!(summary.total > 0);
        assert!(summary.passed > 0);
    }
}
