//! Example demonstrating the built-in health checks
//!
//! This example shows how to use the four built-in health checks:
//! - ConnectionHealthCheck: Monitors active connection count
//! - MemoryHealthCheck: Monitors memory usage
//! - TimeDriftCheck: Monitors time drift
//! - StateDivergenceCheck: Monitors state convergence
//!
//! Run with:
//! ```
//! cargo run --example health_checks
//! ```

use elara_runtime::health::{
    ConnectionHealthCheck, HealthChecker, MemoryHealthCheck, StateDivergenceCheck, TimeDriftCheck,
};
use elara_runtime::node::Node;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    println!("=== ELARA Health Checks Example ===\n");

    // Create a node
    let node = Arc::new(Node::new());
    println!("Created ELARA node with ID: {}", node.node_id().0);

    // Create a health checker with 30-second cache TTL
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    println!("Created health checker with 30s cache TTL\n");

    // Add built-in health checks
    println!("Adding built-in health checks:");

    // 1. Connection Health Check
    // Monitors that we have at least 3 active connections
    checker.add_check(Box::new(ConnectionHealthCheck::new(node.clone(), 3)));
    println!("  ✓ ConnectionHealthCheck (min: 3 connections)");

    // 2. Memory Health Check
    // Monitors that memory usage stays below 1800 MB
    checker.add_check(Box::new(MemoryHealthCheck::new(1800)));
    println!("  ✓ MemoryHealthCheck (max: 1800 MB)");

    // 3. Time Drift Check
    // Monitors that time drift stays below 100ms
    checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 100)));
    println!("  ✓ TimeDriftCheck (max: 100ms drift)");

    // 4. State Divergence Check
    // Monitors that pending events stay below 1000
    checker.add_check(Box::new(StateDivergenceCheck::new(node)));
    println!("  ✓ StateDivergenceCheck (max: 1000 pending events)\n");

    // Run health checks
    println!("Running health checks...\n");
    let status = checker.check_health();

    // Display overall status
    println!("=== Overall Health Status ===");
    if status.is_healthy() {
        println!("Status: ✓ HEALTHY");
    } else if status.is_degraded() {
        println!("Status: ⚠ DEGRADED");
    } else {
        println!("Status: ✗ UNHEALTHY");
    }
    println!();

    // Display individual check results
    println!("=== Individual Check Results ===");
    for (name, result) in &status.checks {
        let status_icon = if result.is_healthy() {
            "✓"
        } else if result.is_degraded() {
            "⚠"
        } else {
            "✗"
        };

        print!("{} {}: ", status_icon, name);

        if result.is_healthy() {
            println!("Healthy");
        } else if let Some(reason) = result.reason() {
            println!("{}", reason);
        }
    }
    println!();

    // Demonstrate caching
    println!("=== Cache Demonstration ===");
    let timestamp1 = status.timestamp;
    println!("First check timestamp: {:?}", timestamp1);

    // Second check should use cache
    let status2 = checker.check_health();
    let timestamp2 = status2.timestamp;
    println!("Second check timestamp: {:?}", timestamp2);

    if timestamp1 == timestamp2 {
        println!("✓ Cache hit! Second check used cached results");
    } else {
        println!("✗ Cache miss - checks were re-executed");
    }
    println!();

    // Clear cache and check again
    checker.clear_cache();
    println!("Cache cleared");

    let status3 = checker.check_health();
    let timestamp3 = status3.timestamp;
    println!("Third check timestamp: {:?}", timestamp3);

    if timestamp3 > timestamp1 {
        println!("✓ Cache cleared successfully - checks were re-executed");
    }
    println!();

    // Display configuration
    println!("=== Configuration ===");
    println!("Total checks registered: {}", checker.check_count());
    println!("Cache TTL: {:?}", checker.cache_ttl());
    println!();

    println!("=== Example Complete ===");
    println!("\nIn a production deployment, you would:");
    println!("1. Expose these checks via HTTP endpoints (/health, /ready, /live)");
    println!("2. Configure Kubernetes liveness and readiness probes");
    println!("3. Set up Prometheus alerts based on health status");
    println!("4. Adjust thresholds based on your deployment size and requirements");
}
