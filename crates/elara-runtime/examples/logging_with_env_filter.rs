//! Example demonstrating per-module log levels and contextual fields
//!
//! This example shows how to:
//! 1. Use RUST_LOG environment variable for per-module log levels
//! 2. Add contextual fields to log entries
//! 3. Configure different output formats
//!
//! # Running the example
//!
//! ## Basic usage (default log level from config)
//! ```bash
//! cargo run --example logging_with_env_filter
//! ```
//!
//! ## With per-module log levels
//! ```bash
//! # Set all modules to info, but enable debug for specific modules
//! RUST_LOG=info,elara_runtime=debug cargo run --example logging_with_env_filter
//!
//! # Enable trace for all modules
//! RUST_LOG=trace cargo run --example logging_with_env_filter
//!
//! # Complex filtering: info by default, debug for runtime, trace for crypto
//! RUST_LOG=info,elara_runtime=debug,elara_crypto=trace cargo run --example logging_with_env_filter
//! ```
//!
//! ## With JSON output format
//! ```bash
//! cargo run --example logging_with_env_filter -- --format json
//! ```

use elara_runtime::observability::logging::{
    init_logging, LogFormat, LogLevel, LogOutput, LoggingConfig,
};
use tracing::{debug, error, info, trace, warn};

fn main() {
    // Parse command line arguments for format selection
    let args: Vec<String> = std::env::args().collect();
    let format = if args.len() > 2 && args[1] == "--format" {
        match args[2].as_str() {
            "json" => LogFormat::Json,
            "compact" => LogFormat::Compact,
            "pretty" => LogFormat::Pretty,
            _ => {
                eprintln!("Unknown format. Using pretty format.");
                LogFormat::Pretty
            }
        }
    } else {
        LogFormat::Pretty
    };

    // Initialize logging
    // The level here serves as a fallback if RUST_LOG is not set
    let config = LoggingConfig {
        level: LogLevel::Info,
        format,
        output: LogOutput::Stdout,
    };

    init_logging(config).expect("Failed to initialize logging");

    println!("\n=== Logging with EnvFilter and Contextual Fields Example ===\n");

    // Check if RUST_LOG is set
    match std::env::var("RUST_LOG") {
        Ok(rust_log) => {
            println!("RUST_LOG is set to: {}", rust_log);
            println!("Per-module log levels will be used.\n");
        }
        Err(_) => {
            println!("RUST_LOG is not set.");
            println!("Using default log level: Info");
            println!("Try setting RUST_LOG to see per-module filtering in action!\n");
        }
    }

    // Demonstrate different log levels with contextual fields
    demonstrate_log_levels();

    // Simulate some operations with contextual logging
    simulate_node_operations();

    println!("\n=== Example Complete ===\n");
}

fn demonstrate_log_levels() {
    info!("=== Demonstrating Log Levels ===");

    // Trace level (most verbose)
    trace!(
        module = "example",
        operation = "trace_demo",
        "This is a TRACE level message - very detailed debugging info"
    );

    // Debug level
    debug!(
        module = "example",
        operation = "debug_demo",
        "This is a DEBUG level message - detailed debugging info"
    );

    // Info level
    info!(
        module = "example",
        operation = "info_demo",
        "This is an INFO level message - general information"
    );

    // Warn level
    warn!(
        module = "example",
        operation = "warn_demo",
        "This is a WARN level message - something to be aware of"
    );

    // Error level
    error!(
        module = "example",
        operation = "error_demo",
        "This is an ERROR level message - something went wrong"
    );
}

fn simulate_node_operations() {
    info!("\n=== Simulating Node Operations ===");

    // Simulate node startup
    info!(
        node_id = "node-1",
        event = "startup",
        "Node starting up"
    );

    // Simulate connection establishment
    info!(
        node_id = "node-1",
        peer_id = "peer-abc",
        event = "connection_established",
        "Connection established with peer"
    );

    // Simulate session creation
    info!(
        node_id = "node-1",
        session_id = "session-xyz",
        peer_id = "peer-abc",
        event = "session_created",
        "New session created"
    );

    // Simulate message sending
    debug!(
        node_id = "node-1",
        session_id = "session-xyz",
        peer_id = "peer-abc",
        message_type = "text",
        message_size = 1024,
        event = "message_sent",
        "Message sent to peer"
    );

    // Simulate message receiving
    debug!(
        node_id = "node-1",
        session_id = "session-xyz",
        peer_id = "peer-abc",
        message_type = "text",
        message_size = 512,
        event = "message_received",
        "Message received from peer"
    );

    // Simulate a warning condition
    warn!(
        node_id = "node-1",
        session_id = "session-xyz",
        peer_id = "peer-abc",
        event = "high_latency",
        latency_ms = 250,
        "High latency detected on connection"
    );

    // Simulate connection closure
    info!(
        node_id = "node-1",
        session_id = "session-xyz",
        peer_id = "peer-abc",
        event = "connection_closed",
        reason = "normal_shutdown",
        "Connection closed"
    );

    // Simulate an error condition
    error!(
        node_id = "node-1",
        peer_id = "peer-def",
        event = "connection_failed",
        error = "timeout",
        retry_count = 3,
        "Failed to establish connection after retries"
    );

    // Simulate node shutdown
    info!(
        node_id = "node-1",
        event = "shutdown",
        "Node shutting down gracefully"
    );
}
