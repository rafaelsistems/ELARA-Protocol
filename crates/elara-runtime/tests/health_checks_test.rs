//! Comprehensive tests for built-in health checks
//!
//! **Validates: Requirements 10.2**
//!
//! This test suite validates the four built-in health checks:
//! - ConnectionHealthCheck: Monitors active connection count
//! - MemoryHealthCheck: Monitors memory usage
//! - TimeDriftCheck: Monitors time drift
//! - StateDivergenceCheck: Monitors state convergence

use elara_runtime::health::{
    ConnectionHealthCheck, HealthCheck, HealthCheckResult, HealthChecker, MemoryHealthCheck,
    StateDivergenceCheck, TimeDriftCheck,
};
use elara_runtime::node::Node;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// ConnectionHealthCheck Tests
// ============================================================================

#[test]
fn test_connection_health_check_creation() {
    let node = Arc::new(Node::new());
    let check = ConnectionHealthCheck::new(node, 5);

    assert_eq!(check.name(), "connections");
    assert_eq!(check.min_connections(), 5);
}

#[test]
fn test_connection_health_check_with_different_thresholds() {
    let node = Arc::new(Node::new());

    let check1 = ConnectionHealthCheck::new(node.clone(), 1);
    assert_eq!(check1.min_connections(), 1);

    let check2 = ConnectionHealthCheck::new(node.clone(), 10);
    assert_eq!(check2.min_connections(), 10);

    let check3 = ConnectionHealthCheck::new(node, 100);
    assert_eq!(check3.min_connections(), 100);
}

#[test]
fn test_connection_health_check_returns_result() {
    let node = Arc::new(Node::new());
    let check = ConnectionHealthCheck::new(node, 3);

    // Should return a result (currently degraded since active_connections is not implemented)
    let result = check.check();
    assert!(
        result.is_degraded() || result.is_healthy(),
        "Check should return a valid result"
    );
}

// ============================================================================
// MemoryHealthCheck Tests
// ============================================================================

#[test]
fn test_memory_health_check_creation() {
    let check = MemoryHealthCheck::new(2048);

    assert_eq!(check.name(), "memory");
    assert_eq!(check.max_memory_mb(), 2048);
}

#[test]
fn test_memory_health_check_with_high_threshold() {
    // Very high threshold should result in healthy status
    let check = MemoryHealthCheck::new(100_000); // 100GB
    let result = check.check();

    assert!(
        result.is_healthy(),
        "Should be healthy with very high threshold"
    );
}

#[test]
fn test_memory_health_check_with_low_threshold() {
    // Very low threshold should result in unhealthy status
    let check = MemoryHealthCheck::new(1); // 1MB
    let result = check.check();

    assert!(
        result.is_unhealthy(),
        "Should be unhealthy with very low threshold"
    );
}

#[test]
fn test_memory_health_check_threshold_boundary() {
    // Test with reasonable thresholds
    let check_low = MemoryHealthCheck::new(10);
    let check_high = MemoryHealthCheck::new(10_000);

    let result_low = check_low.check();
    let result_high = check_high.check();

    // Low threshold should be unhealthy, high should be healthy
    assert!(result_low.is_unhealthy());
    assert!(result_high.is_healthy());
}

#[test]
fn test_memory_health_check_reason_format() {
    let check = MemoryHealthCheck::new(1); // Will be unhealthy
    let result = check.check();

    if let HealthCheckResult::Unhealthy { reason } = result {
        assert!(reason.contains("Memory usage"));
        assert!(reason.contains("MB"));
        assert!(reason.contains("exceeds limit"));
    } else {
        panic!("Expected Unhealthy result");
    }
}

// ============================================================================
// TimeDriftCheck Tests
// ============================================================================

#[test]
fn test_time_drift_check_creation() {
    let node = Arc::new(Node::new());
    let check = TimeDriftCheck::new(node, 100);

    assert_eq!(check.name(), "time_drift");
    assert_eq!(check.max_drift_ms(), 100);
}

#[test]
fn test_time_drift_check_with_different_thresholds() {
    let node = Arc::new(Node::new());

    let check1 = TimeDriftCheck::new(node.clone(), 50);
    assert_eq!(check1.max_drift_ms(), 50);

    let check2 = TimeDriftCheck::new(node.clone(), 100);
    assert_eq!(check2.max_drift_ms(), 100);

    let check3 = TimeDriftCheck::new(node, 500);
    assert_eq!(check3.max_drift_ms(), 500);
}

#[test]
fn test_time_drift_check_returns_result() {
    let node = Arc::new(Node::new());
    let check = TimeDriftCheck::new(node, 100);

    // Should return a result (currently healthy since drift_ms returns 0)
    let result = check.check();
    assert!(
        result.is_healthy() || result.is_degraded(),
        "Check should return a valid result"
    );
}

#[test]
fn test_time_drift_check_with_zero_drift() {
    let node = Arc::new(Node::new());
    let check = TimeDriftCheck::new(node, 100);

    // With current implementation (drift_ms returns 0), should be healthy
    let result = check.check();
    assert!(result.is_healthy(), "Should be healthy with zero drift");
}

// ============================================================================
// StateDivergenceCheck Tests
// ============================================================================

#[test]
fn test_state_divergence_check_creation() {
    let node = Arc::new(Node::new());
    let check = StateDivergenceCheck::new(node);

    assert_eq!(check.name(), "state_convergence");
    assert_eq!(check.max_pending_events(), 1000); // Default threshold
}

#[test]
fn test_state_divergence_check_with_custom_threshold() {
    let node = Arc::new(Node::new());
    let check = StateDivergenceCheck::with_threshold(node, 500);

    assert_eq!(check.max_pending_events(), 500);
}

#[test]
fn test_state_divergence_check_with_different_thresholds() {
    let node = Arc::new(Node::new());

    let check1 = StateDivergenceCheck::with_threshold(node.clone(), 100);
    assert_eq!(check1.max_pending_events(), 100);

    let check2 = StateDivergenceCheck::with_threshold(node.clone(), 1000);
    assert_eq!(check2.max_pending_events(), 1000);

    let check3 = StateDivergenceCheck::with_threshold(node, 5000);
    assert_eq!(check3.max_pending_events(), 5000);
}

#[test]
fn test_state_divergence_check_returns_result() {
    let node = Arc::new(Node::new());
    let check = StateDivergenceCheck::new(node);

    // Should return a result (currently healthy since pending_count returns 0)
    let result = check.check();
    assert!(
        result.is_healthy() || result.is_degraded(),
        "Check should return a valid result"
    );
}

#[test]
fn test_state_divergence_check_with_zero_pending() {
    let node = Arc::new(Node::new());
    let check = StateDivergenceCheck::new(node);

    // With current implementation (pending_count returns 0), should be healthy
    let result = check.check();
    assert!(
        result.is_healthy(),
        "Should be healthy with zero pending events"
    );
}

// ============================================================================
// Integration Tests - All Checks Together
// ============================================================================

#[test]
fn test_all_builtin_checks_with_health_checker() {
    let node = Arc::new(Node::new());
    let mut checker = HealthChecker::new(Duration::from_secs(30));

    // Add all four built-in checks
    checker.add_check(Box::new(ConnectionHealthCheck::new(node.clone(), 3)));
    checker.add_check(Box::new(MemoryHealthCheck::new(100_000))); // High threshold
    checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 100)));
    checker.add_check(Box::new(StateDivergenceCheck::new(node)));

    assert_eq!(checker.check_count(), 4);

    // Run health check
    let status = checker.check_health();
    assert_eq!(status.checks.len(), 4);

    // Verify all checks are present
    assert!(status.checks.contains_key("connections"));
    assert!(status.checks.contains_key("memory"));
    assert!(status.checks.contains_key("time_drift"));
    assert!(status.checks.contains_key("state_convergence"));
}

#[test]
fn test_builtin_checks_aggregation_all_healthy() {
    let node = Arc::new(Node::new());
    let mut checker = HealthChecker::new(Duration::from_secs(30));

    // Add checks with thresholds that should all be healthy
    checker.add_check(Box::new(MemoryHealthCheck::new(100_000))); // High threshold
    checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 1000))); // High threshold
    checker.add_check(Box::new(StateDivergenceCheck::with_threshold(
        node,
        10000,
    ))); // High threshold

    let status = checker.check_health();

    // Memory should be healthy with high threshold
    assert!(status.checks.get("memory").unwrap().is_healthy());

    // Overall status should be healthy or degraded (depending on connections)
    assert!(
        status.is_healthy() || status.is_degraded(),
        "Overall status should be healthy or degraded"
    );
}

#[test]
fn test_builtin_checks_aggregation_with_unhealthy() {
    let node = Arc::new(Node::new());
    let mut checker = HealthChecker::new(Duration::from_secs(30));

    // Add a check that will be unhealthy
    checker.add_check(Box::new(MemoryHealthCheck::new(1))); // Very low threshold
    checker.add_check(Box::new(TimeDriftCheck::new(node, 1000)));

    let status = checker.check_health();

    // Memory check should be unhealthy
    assert!(status.checks.get("memory").unwrap().is_unhealthy());

    // Overall status should be unhealthy
    assert!(
        status.is_unhealthy(),
        "Overall status should be unhealthy when any check is unhealthy"
    );
}

#[test]
fn test_builtin_checks_caching() {
    let node = Arc::new(Node::new());
    let mut checker = HealthChecker::new(Duration::from_millis(100));

    checker.add_check(Box::new(MemoryHealthCheck::new(100_000)));
    checker.add_check(Box::new(TimeDriftCheck::new(node, 100)));

    // First call
    let status1 = checker.check_health();
    let timestamp1 = status1.timestamp;

    // Second call within TTL should return cached result
    let status2 = checker.check_health();
    let timestamp2 = status2.timestamp;

    assert_eq!(
        timestamp1, timestamp2,
        "Should return cached result within TTL"
    );

    // Wait for cache to expire
    std::thread::sleep(Duration::from_millis(150));

    // Third call after TTL should execute checks again
    let status3 = checker.check_health();
    let timestamp3 = status3.timestamp;

    assert!(
        timestamp3 > timestamp1,
        "Should execute checks again after TTL"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_memory_check_with_zero_threshold() {
    let check = MemoryHealthCheck::new(0);
    let result = check.check();

    // Should be unhealthy since any memory usage exceeds 0
    assert!(result.is_unhealthy());
}

#[test]
fn test_time_drift_check_with_zero_threshold() {
    let node = Arc::new(Node::new());
    let check = TimeDriftCheck::new(node, 0);

    // With zero threshold, even 0 drift should be healthy (< 0 is false)
    let result = check.check();
    assert!(result.is_healthy() || result.is_degraded());
}

#[test]
fn test_state_divergence_check_with_zero_threshold() {
    let node = Arc::new(Node::new());
    let check = StateDivergenceCheck::with_threshold(node, 0);

    // With zero threshold and 0 pending events, 0 < 0 is false, so it's not converging
    // This should be degraded
    let result = check.check();
    assert!(result.is_degraded());
}

#[test]
fn test_multiple_instances_of_same_check_type() {
    let _node = Arc::new(Node::new());
    let mut checker = HealthChecker::new(Duration::from_secs(30));

    // Add multiple memory checks with different thresholds
    // Note: This will cause name collisions, but tests the system handles it
    checker.add_check(Box::new(MemoryHealthCheck::new(100_000)));
    checker.add_check(Box::new(MemoryHealthCheck::new(1)));

    let _status = checker.check_health();

    // Should have 2 checks, but only one "memory" key (last one wins)
    assert_eq!(checker.check_count(), 2);
}

// ============================================================================
// Documentation Tests
// ============================================================================

#[test]
fn test_check_names_are_valid_identifiers() {
    let node = Arc::new(Node::new());

    let checks: Vec<Box<dyn HealthCheck>> = vec![
        Box::new(ConnectionHealthCheck::new(node.clone(), 3)),
        Box::new(MemoryHealthCheck::new(2048)),
        Box::new(TimeDriftCheck::new(node.clone(), 100)),
        Box::new(StateDivergenceCheck::new(node)),
    ];

    for check in checks {
        let name = check.name();
        // Names should be lowercase, alphanumeric, and underscores only
        assert!(
            name.chars()
                .all(|c| c.is_lowercase() || c.is_numeric() || c == '_'),
            "Check name '{}' should be a valid identifier",
            name
        );
        assert!(!name.is_empty(), "Check name should not be empty");
    }
}

#[test]
fn test_all_checks_implement_send_sync() {
    // This test verifies that all checks can be used across threads
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<ConnectionHealthCheck>();
    assert_send_sync::<MemoryHealthCheck>();
    assert_send_sync::<TimeDriftCheck>();
    assert_send_sync::<StateDivergenceCheck>();
}
