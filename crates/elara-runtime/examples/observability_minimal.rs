//! Example: Minimal Observability Configuration
//!
//! This example demonstrates a minimal observability setup with only logging enabled.
//! This is useful for development or when you only need basic logging without the
//! overhead of tracing and metrics.
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example observability_minimal
//! ```
//!
//! # Per-Module Log Levels
//!
//! You can control log levels per module using the RUST_LOG environment variable:
//!
//! ```bash
//! # Set all to info, but elara_wire to debug
//! RUST_LOG=info,elara_wire=debug cargo run --example observability_minimal
//!
//! # Set elara_crypto to trace, everything else to warn
//! RUST_LOG=warn,elara_crypto=trace cargo run --example observability_minimal
//! ```

use elara_runtime::observability::{
    init_observability, LogFormat, LogLevel, LogOutput, LoggingConfig, ObservabilityConfig,
};
use tracing::{debug, error, info, trace, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Minimal Observability Example ===\n");

    // Minimal configuration - only logging enabled
    let config = ObservabilityConfig {
        logging: Some(LoggingConfig {
            level: LogLevel::Debug,
            format: LogFormat::Pretty, // Human-readable for development
            output: LogOutput::Stdout,
        }),
        tracing: None,        // Tracing disabled
        metrics_server: None, // Metrics server disabled
    };

    println!("Initializing logging...");

    // Initialize observability (only logging in this case)
    let handle = init_observability(config).await?;

    println!("✓ Logging initialized\n");

    // Demonstrate different log levels
    trace!("This is a trace message (very detailed)");
    debug!("This is a debug message");
    info!("This is an info message");
    warn!("This is a warning message");
    error!("This is an error message");

    println!();

    // Demonstrate structured logging with fields
    info!(
        node_id = "node-1",
        session_id = 42,
        peer_count = 5,
        "Node status update"
    );

    info!(
        message_id = 12345,
        size_bytes = 1024,
        latency_ms = 42,
        "Message processed"
    );

    // Simulate some work
    for i in 0..5 {
        info!(iteration = i, "Processing item");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("\nShutting down...");

    // Graceful shutdown
    handle.shutdown().await?;

    println!("✓ Logging shut down");

    Ok(())
}
