//! Example demonstrating health check configuration in NodeConfig
//!
//! This example shows how to configure health checks through NodeConfig,
//! including:
//! - Using preset configurations for different deployment sizes
//! - Custom configuration with specific thresholds
//! - Initializing health checks with HTTP server
//! - Programmatic health checking
//!
//! Run with:
//! ```
//! cargo run --example health_check_config
//! ```

use elara_runtime::health::HealthCheckConfig;
use elara_runtime::node::{Node, NodeConfig};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("=== ELARA Health Check Configuration Example ===\n");

    // Example 1: Using preset configuration for medium deployment
    println!("Example 1: Medium Deployment Preset");
    println!("====================================");
    
    let config = NodeConfig {
        health_checks: Some(HealthCheckConfig::medium_deployment()),
        ..Default::default()
    };
    
    println!("Configuration:");
    if let Some(ref hc) = config.health_checks {
        println!("  Enabled: {}", hc.enabled);
        println!("  Server: {:?}", hc.server_bind_address);
        println!("  Cache TTL: {:?}", hc.cache_ttl);
        println!("  Min connections: {:?}", hc.min_connections);
        println!("  Max memory MB: {:?}", hc.max_memory_mb);
        println!("  Max time drift ms: {:?}", hc.max_time_drift_ms);
        println!("  Max pending events: {:?}", hc.max_pending_events);
    }
    println!();

    let node = Arc::new(Node::with_config(config.clone()));
    
    if let Some((checker, server_handle)) = config.init_health_checks(node.clone()) {
        println!("✓ Health checks initialized");
        println!("✓ Registered {} checks", checker.check_count());
        
        if server_handle.is_some() {
            println!("✓ HTTP server started on 0.0.0.0:8080");
            println!("  Endpoints:");
            println!("    - GET http://localhost:8080/health");
            println!("    - GET http://localhost:8080/ready");
            println!("    - GET http://localhost:8080/live");
        }
        
        // Check health programmatically
        let status = checker.check_health();
        println!("\nHealth Status:");
        if status.is_healthy() {
            println!("  Overall: ✓ HEALTHY");
        } else if status.is_degraded() {
            println!("  Overall: ⚠ DEGRADED");
        } else {
            println!("  Overall: ✗ UNHEALTHY");
        }
        
        println!("\n  Individual Checks:");
        for (name, result) in &status.checks {
            let icon = if result.is_healthy() {
                "✓"
            } else if result.is_degraded() {
                "⚠"
            } else {
                "✗"
            };
            
            if let Some(reason) = result.reason() {
                println!("    {} {}: {}", icon, name, reason);
            } else {
                println!("    {} {}: Healthy", icon, name);
            }
        }
    }
    
    println!("\n");

    // Example 2: Custom configuration
    println!("Example 2: Custom Configuration");
    println!("================================");
    
    let custom_config = NodeConfig {
        health_checks: Some(HealthCheckConfig {
            enabled: true,
            server_bind_address: Some("127.0.0.1:9090".parse().unwrap()),
            cache_ttl: Duration::from_secs(15),
            min_connections: Some(5),
            max_memory_mb: Some(2500),
            max_time_drift_ms: Some(50),
            max_pending_events: Some(1500),
        }),
        ..Default::default()
    };
    
    println!("Custom thresholds:");
    if let Some(ref hc) = custom_config.health_checks {
        println!("  Server: {:?}", hc.server_bind_address);
        println!("  Cache TTL: {:?}", hc.cache_ttl);
        println!("  Min connections: {:?}", hc.min_connections);
        println!("  Max memory MB: {:?}", hc.max_memory_mb);
        println!("  Max time drift ms: {:?}", hc.max_time_drift_ms);
        println!("  Max pending events: {:?}", hc.max_pending_events);
    }
    println!();

    // Example 3: Selective health checks
    println!("Example 3: Selective Health Checks");
    println!("===================================");
    
    let selective_config = NodeConfig {
        health_checks: Some(HealthCheckConfig {
            enabled: true,
            server_bind_address: None, // No HTTP server
            cache_ttl: Duration::from_secs(30),
            min_connections: None, // Disabled
            max_memory_mb: Some(2000), // Only memory check
            max_time_drift_ms: None, // Disabled
            max_pending_events: None, // Disabled
        }),
        ..Default::default()
    };
    
    println!("Only memory health check enabled");
    println!("No HTTP server (programmatic checking only)");
    
    let node3 = Arc::new(Node::with_config(selective_config.clone()));
    
    if let Some((checker, server_handle)) = selective_config.init_health_checks(node3) {
        println!("✓ Health checks initialized");
        println!("✓ Registered {} check(s)", checker.check_count());
        
        if server_handle.is_none() {
            println!("✓ No HTTP server (as configured)");
        }
    }
    
    println!("\n");

    // Example 4: Deployment size presets
    println!("Example 4: Deployment Size Presets");
    println!("===================================");
    
    println!("\nSmall Deployment (10 nodes):");
    let small = HealthCheckConfig::small_deployment();
    println!("  Min connections: {:?}", small.min_connections);
    println!("  Max memory MB: {:?}", small.max_memory_mb);
    println!("  Max pending events: {:?}", small.max_pending_events);
    
    println!("\nMedium Deployment (100 nodes):");
    let medium = HealthCheckConfig::medium_deployment();
    println!("  Min connections: {:?}", medium.min_connections);
    println!("  Max memory MB: {:?}", medium.max_memory_mb);
    println!("  Max pending events: {:?}", medium.max_pending_events);
    
    println!("\nLarge Deployment (1000 nodes):");
    let large = HealthCheckConfig::large_deployment();
    println!("  Min connections: {:?}", large.min_connections);
    println!("  Max memory MB: {:?}", large.max_memory_mb);
    println!("  Max pending events: {:?}", large.max_pending_events);
    
    println!("\n");

    // Example 5: Disabled health checks
    println!("Example 5: Disabled Health Checks");
    println!("==================================");
    
    let disabled_config = NodeConfig {
        health_checks: None, // Completely disabled
        ..Default::default()
    };
    
    let node5 = Arc::new(Node::with_config(disabled_config.clone()));
    
    if let Some(_) = disabled_config.init_health_checks(node5) {
        println!("Health checks initialized");
    } else {
        println!("✓ Health checks disabled (as configured)");
        println!("  No overhead from health checking");
    }
    
    println!("\n");

    // Example 6: Configuration validation
    println!("Example 6: Configuration Validation");
    println!("====================================");
    
    let valid_config = HealthCheckConfig::default();
    match valid_config.validate() {
        Ok(()) => println!("✓ Default configuration is valid"),
        Err(e) => println!("✗ Validation error: {}", e),
    }
    
    let invalid_config = HealthCheckConfig {
        enabled: true,
        server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
        cache_ttl: Duration::from_millis(500), // Too short!
        min_connections: Some(0), // Invalid!
        max_memory_mb: Some(2000),
        max_time_drift_ms: Some(100),
        max_pending_events: Some(1000),
    };
    
    match invalid_config.validate() {
        Ok(()) => println!("Configuration is valid"),
        Err(e) => println!("✗ Expected validation error: {}", e),
    }
    
    println!("\n=== Example Complete ===");
    println!("\nKey Takeaways:");
    println!("1. Health checks are opt-in via NodeConfig.health_checks");
    println!("2. Use preset configurations for common deployment sizes");
    println!("3. Customize thresholds based on your requirements");
    println!("4. HTTP server is optional (set server_bind_address to None)");
    println!("5. Selectively enable only the checks you need");
    println!("6. Configuration is validated at initialization time");
    
    println!("\nProduction Deployment:");
    println!("1. Choose appropriate preset or customize thresholds");
    println!("2. Configure Kubernetes probes to use /ready and /live endpoints");
    println!("3. Set up Prometheus to scrape health status");
    println!("4. Configure alerts based on health check results");
    println!("5. Monitor health check cache hit rate and adjust TTL");
    
    // Keep server running for a bit to allow manual testing
    println!("\nServer will run for 10 seconds for manual testing...");
    println!("Try: curl http://localhost:8080/health");
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    println!("\nShutting down...");
}
