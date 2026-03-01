//! Example: Unified Observability Initialization
//!
//! This example demonstrates how to use the unified observability system to initialize
//! all observability components (logging, tracing, metrics) with a single function call.
//!
//! # Features Demonstrated
//!
//! - Unified initialization of all observability components
//! - Structured logging with JSON format
//! - Distributed tracing with OTLP exporter
//! - Metrics server with Prometheus endpoint
//! - Graceful shutdown of all components
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example unified_observability
//! ```
//!
//! # Testing the Endpoints
//!
//! While the example is running, you can test the metrics endpoint:
//!
//! ```bash
//! curl http://localhost:9090/metrics
//! ```
//!
//! # Production Configuration
//!
//! For production deployments, consider:
//! - Using JSON log format for log aggregation
//! - Configuring appropriate sampling rates for tracing (e.g., 0.1 for 10%)
//! - Setting resource attributes for filtering/grouping
//! - Using environment-specific exporter endpoints

use elara_runtime::observability::{
    init_observability, LogFormat, LogLevel, LogOutput, LoggingConfig, MetricsServerConfig,
    ObservabilityConfig, TracingConfig, TracingExporter,
};
use std::time::Duration;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Unified Observability Example ===\n");

    // Configure all observability components
    let config = ObservabilityConfig {
        // Enable structured logging with JSON format
        logging: Some(LoggingConfig {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Stdout,
        }),

        // Enable distributed tracing with OTLP exporter
        // Note: This requires an OTLP collector running at localhost:4317
        // For testing without a collector, set this to None
        tracing: Some(TracingConfig {
            service_name: "elara-observability-example".to_string(),
            exporter: TracingExporter::None, // Change to Otlp for real tracing
            sampling_rate: 1.0, // Sample all traces in this example
            resource_attributes: vec![
                ("environment".to_string(), "development".to_string()),
                ("version".to_string(), "1.0.0".to_string()),
            ],
        }),

        // Enable metrics server on port 9090
        metrics_server: Some(MetricsServerConfig {
            bind_address: "0.0.0.0".to_string(),
            port: 9090,
        }),
    };

    println!("Initializing observability system...");

    // Initialize all components with a single call
    let handle = init_observability(config).await?;

    println!("✓ Observability system initialized");
    println!("✓ Metrics server running at http://localhost:9090/metrics");
    println!();

    // Demonstrate structured logging
    info!("Application started successfully");
    info!(
        node_id = "node-1",
        session_id = 42,
        "Node joined session"
    );

    // Register and use some metrics
    let counter = handle
        .metrics_registry()
        .register_counter("example_requests_total", vec![]);
    let gauge = handle
        .metrics_registry()
        .register_gauge("example_active_connections", vec![]);
    let histogram = handle.metrics_registry().register_histogram(
        "example_request_duration_ms",
        vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0],
        vec![],
    );

    println!("Simulating application activity...");
    println!("(Press Ctrl+C to stop)\n");

    // Simulate some application activity
    for i in 0..10 {
        // Update metrics
        counter.inc();
        gauge.set(i as i64);
        histogram.observe((i * 10) as f64);

        // Log some events
        info!(iteration = i, "Processing request");

        if i % 3 == 0 {
            warn!(iteration = i, "Simulated warning");
        }

        if i == 7 {
            error!(iteration = i, "Simulated error");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("\nShutting down gracefully...");

    // Graceful shutdown - flushes all pending telemetry
    handle.shutdown().await?;

    println!("✓ Observability system shut down");
    println!("✓ All telemetry flushed");

    Ok(())
}
