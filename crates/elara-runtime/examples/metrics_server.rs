//! Example demonstrating the Prometheus metrics server.
//!
//! This example shows how to:
//! 1. Create a metrics registry
//! 2. Register and update various metrics
//! 3. Start the metrics HTTP server
//! 4. Access metrics via HTTP endpoint
//!
//! Run this example with:
//! ```bash
//! cargo run --example metrics_server
//! ```
//!
//! Then access metrics at: http://localhost:9090/metrics

use elara_runtime::observability::{
    MetricsRegistry, MetricsServer, MetricsServerConfig, NodeMetrics,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .init();

    println!("Starting Prometheus metrics server example...");
    println!();

    // Create metrics registry
    let registry = MetricsRegistry::new();

    // Create node metrics (registers all standard ELARA metrics)
    let node_metrics = NodeMetrics::new(&registry);

    // Configure and start metrics server
    let config = MetricsServerConfig {
        bind_address: "127.0.0.1".to_string(),
        port: 9090,
    };

    let mut server = MetricsServer::new(config, registry.clone());
    server.start().await?;

    println!("✓ Metrics server started on http://127.0.0.1:9090");
    println!("✓ Access metrics at: http://127.0.0.1:9090/metrics");
    println!();
    println!("Simulating node activity...");
    println!("(Press Ctrl+C to stop)");
    println!();

    // Simulate node activity
    let mut iteration = 0;
    loop {
        iteration += 1;

        // Simulate connection activity
        if iteration % 10 == 0 {
            node_metrics.active_connections.inc();
            node_metrics.total_connections.inc();
            println!("  [+] New connection established (total: {})", 
                node_metrics.active_connections.get());
        }

        if iteration % 25 == 0 && node_metrics.active_connections.get() > 0 {
            node_metrics.active_connections.dec();
            println!("  [-] Connection closed (active: {})", 
                node_metrics.active_connections.get());
        }

        // Simulate message activity
        let messages_to_send = (iteration % 5) + 1;
        node_metrics.messages_sent.inc_by(messages_to_send);

        let messages_to_receive = (iteration % 4) + 1;
        node_metrics.messages_received.inc_by(messages_to_receive);

        // Simulate message latency (varies between 1-100ms)
        let latency = ((iteration * 7) % 100) as f64 + 1.0;
        node_metrics.message_latency_ms.observe(latency);

        // Simulate message size (varies between 100-5000 bytes)
        let size = ((iteration * 13) % 4900) as f64 + 100.0;
        node_metrics.message_size_bytes.observe(size);

        // Occasionally simulate dropped messages
        if iteration % 50 == 0 {
            node_metrics.messages_dropped.inc();
            println!("  [!] Message dropped");
        }

        // Simulate resource usage
        let memory_mb = 256 + ((iteration * 3) % 512);
        node_metrics.memory_usage_bytes.set((memory_mb * 1024 * 1024) as i64);

        let cpu_percent = 20 + ((iteration * 5) % 60);
        node_metrics.cpu_usage_percent.set(cpu_percent as i64);

        // Simulate time drift (oscillates between -50ms and +50ms)
        let drift = ((iteration * 11) % 100) as i64 - 50;
        node_metrics.time_drift_ms.set(drift);

        // Simulate state sync latency occasionally
        if iteration % 15 == 0 {
            let sync_latency = ((iteration * 17) % 1000) as f64 + 50.0;
            node_metrics.state_sync_latency_ms.observe(sync_latency);
            println!("  [~] State sync completed ({:.1}ms)", sync_latency);
        }

        // Print summary every 30 iterations
        if iteration % 30 == 0 {
            println!();
            println!("=== Metrics Summary (iteration {}) ===", iteration);
            println!("  Connections: {} active, {} total", 
                node_metrics.active_connections.get(),
                node_metrics.total_connections.get());
            println!("  Messages: {} sent, {} received, {} dropped",
                node_metrics.messages_sent.get(),
                node_metrics.messages_received.get(),
                node_metrics.messages_dropped.get());
            println!("  Resources: {} MB memory, {}% CPU",
                memory_mb,
                cpu_percent);
            println!("  Time drift: {}ms", drift);
            println!();
        }

        // Wait before next iteration
        sleep(Duration::from_millis(500)).await;
    }
}
