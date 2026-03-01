//! Health Check HTTP Server Example
//!
//! This example demonstrates how to set up and run the health check HTTP server
//! with all built-in health checks. The server exposes three endpoints:
//!
//! - `/health` - Overall health status (all checks)
//! - `/ready` - Readiness probe (Kubernetes)
//! - `/live` - Liveness probe (Kubernetes)
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example health_server
//! ```
//!
//! # Testing the Endpoints
//!
//! Once the server is running, you can test the endpoints using curl:
//!
//! ```bash
//! # Check overall health
//! curl http://localhost:8080/health | jq
//!
//! # Check readiness
//! curl http://localhost:8080/ready | jq
//!
//! # Check liveness
//! curl http://localhost:8080/live | jq
//! ```
//!
//! # Expected Output
//!
//! ```json
//! {
//!   "status": "healthy",
//!   "timestamp": "2024-01-15T10:30:00Z",
//!   "checks": {
//!     "connections": {
//!       "status": "degraded",
//!       "reason": "Only 0 active connections (minimum: 3)"
//!     },
//!     "memory": {
//!       "status": "healthy"
//!     },
//!     "time_drift": {
//!       "status": "healthy"
//!     },
//!     "state_convergence": {
//!       "status": "healthy"
//!     }
//!   }
//! }
//! ```

use elara_runtime::health::{
    ConnectionHealthCheck, HealthChecker, MemoryHealthCheck, StateDivergenceCheck, TimeDriftCheck,
};
use elara_runtime::health_server::{HealthServer, HealthServerConfig};
use elara_runtime::node::Node;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting ELARA Health Check Server Example");

    // Create a node instance
    let node = Arc::new(Node::new());

    // Create health checker with 30-second cache TTL
    let mut checker = HealthChecker::new(Duration::from_secs(30));

    // Add built-in health checks
    info!("Registering health checks...");

    // 1. Connection health check - requires at least 3 active connections
    checker.add_check(Box::new(ConnectionHealthCheck::new(node.clone(), 3)));
    info!("  ✓ Connection health check (min: 3 connections)");

    // 2. Memory health check - fails if memory usage exceeds 1800 MB
    checker.add_check(Box::new(MemoryHealthCheck::new(1800)));
    info!("  ✓ Memory health check (max: 1800 MB)");

    // 3. Time drift check - fails if time drift exceeds 100ms
    checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 100)));
    info!("  ✓ Time drift check (max: 100ms)");

    // 4. State divergence check - fails if too many pending events
    checker.add_check(Box::new(StateDivergenceCheck::new(node)));
    info!("  ✓ State convergence check");

    let checker = Arc::new(checker);

    // Configure health server
    let config = HealthServerConfig {
        bind_address: "0.0.0.0:8080".parse()?,
    };

    info!("Health checks registered: {}", checker.check_count());
    info!("Cache TTL: {:?}", checker.cache_ttl());

    // Create and start the health server
    let server = HealthServer::new(checker, config);

    info!("");
    info!("Health check server starting...");
    info!("Endpoints:");
    info!("  - http://localhost:8080/health - Overall health status");
    info!("  - http://localhost:8080/ready  - Readiness probe");
    info!("  - http://localhost:8080/live   - Liveness probe");
    info!("");
    info!("Test with: curl http://localhost:8080/health | jq");
    info!("");
    info!("Press Ctrl+C to stop");

    // Run the server (blocks until shutdown)
    server.serve().await?;

    Ok(())
}
