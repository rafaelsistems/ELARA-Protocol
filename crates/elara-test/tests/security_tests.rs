//! Integration tests for the security test module
//!
//! This file demonstrates how to use the security testing infrastructure
//! for comprehensive security validation.

use elara_core::{NodeId, PacketClass, SessionId};
use elara_crypto::KEY_SIZE;
use elara_test::security::{
    KeyIsolationTestHarness, MessageAuthenticationTestHarness, ReplayProtectionTestHarness,
    SecurityTestConfig, SecurityTestSuite,
};

#[test]
fn test_replay_protection_basic() {
    let node_id = NodeId::new(1);
    let class = PacketClass::Core;
    let mut harness = ReplayProtectionTestHarness::new(node_id, class);

    // Test basic replay attack detection
    let result = harness.test_replay_attack(100);
    assert!(result.passed, "Replay attack test failed: {:?}", result.error);
}

#[test]
fn test_replay_window_advancement() {
    let node_id = NodeId::new(1);
    let class = PacketClass::Core;
    let mut harness = ReplayProtectionTestHarness::new(node_id, class);

    // Test window advancement with a jump larger than window size
    // Core class has window size of 64, so jump by 100
    let result = harness.test_window_advancement(0, 100);
    assert!(
        result.passed,
        "Window advancement test failed: {:?}",
        result.error
    );
}

#[test]
fn test_sequence_wraparound() {
    let node_id = NodeId::new(1);
    let class = PacketClass::Core;
    let mut harness = ReplayProtectionTestHarness::new(node_id, class);

    // Test sequence number wraparound
    let result = harness.test_sequence_wraparound();
    assert!(
        result.passed,
        "Sequence wraparound test failed: {:?}",
        result.error
    );
}

#[test]
fn test_message_tampering_detection() {
    let session_id = SessionId::new(1);
    let node_id = NodeId::new(1);
    let session_key = [0x42; KEY_SIZE];
    let mut harness = MessageAuthenticationTestHarness::new(session_id, node_id, session_key);

    let test_payload = b"Test message for tampering detection";

    // Test message tampering detection
    let result = harness.test_message_tampering(test_payload);
    assert!(
        result.passed,
        "Message tampering test failed: {:?}",
        result.error
    );
}

#[test]
fn test_mac_verification() {
    let session_id = SessionId::new(1);
    let node_id = NodeId::new(1);
    let session_key = [0x42; KEY_SIZE];
    let mut harness = MessageAuthenticationTestHarness::new(session_id, node_id, session_key);

    let test_payload = b"Test message for MAC verification";

    // Test MAC verification
    let result = harness.test_mac_verification(test_payload);
    assert!(
        result.passed,
        "MAC verification test failed: {:?}",
        result.error
    );
}

#[test]
fn test_session_key_isolation() {
    let mut harness = KeyIsolationTestHarness::new(3);
    let test_payload = b"Test message for key isolation";

    // Test session key isolation
    let result = harness.test_session_key_isolation(test_payload);
    assert!(
        result.passed,
        "Session key isolation test failed: {:?}",
        result.error
    );
}

#[test]
fn test_key_derivation_independence() {
    let mut harness = KeyIsolationTestHarness::new(2);
    let test_payload = b"Test message for key derivation";

    // Test key derivation independence
    let result = harness.test_key_derivation_independence(test_payload);
    assert!(
        result.passed,
        "Key derivation independence test failed: {:?}",
        result.error
    );
}

#[test]
fn test_comprehensive_security_suite() {
    let config = SecurityTestConfig::default();
    let mut suite = SecurityTestSuite::new(config);

    // Run all security tests
    suite.run_all_tests();

    // Get summary
    let summary = suite.summary();

    println!("Security Test Summary:");
    println!("  Total: {}", summary.total);
    println!("  Passed: {}", summary.passed);
    println!("  Failed: {}", summary.failed);
    println!("  Success Rate: {:.2}%", summary.success_rate() * 100.0);

    // Print individual results
    for result in suite.results() {
        if result.passed {
            println!("  ✓ {}", result.description);
        } else {
            println!(
                "  ✗ {} - {:?}",
                result.description,
                result.error.as_ref().unwrap_or(&"Unknown error".to_string())
            );
        }
    }

    // All tests should pass
    assert!(
        suite.all_passed(),
        "Some security tests failed. See output above."
    );
}
