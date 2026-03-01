//! Example demonstrating distributed tracing with OpenTelemetry
//!
//! This example shows how to initialize and use the tracing system with different exporters.
//!
//! # Running the example
//!
//! ```bash
//! # With Jaeger (requires Jaeger running on localhost:14268)
//! cargo run --example tracing_example -- jaeger
//!
//! # With Zipkin (requires Zipkin running on localhost:9411)
//! cargo run --example tracing_example -- zipkin
//!
//! # With OTLP (requires OTLP collector on localhost:4317)
//! cargo run --example tracing_example -- otlp
//!
//! # Disabled (no exporter)
//! cargo run --example tracing_example -- none
//! ```

use elara_runtime::observability::tracing::{init_tracing, TracingConfig, TracingExporter};
use std::env;
use tracing::{info, span, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line argument for exporter type
    let args: Vec<String> = env::args().collect();
    let exporter_type = args.get(1).map(|s| s.as_str()).unwrap_or("none");

    // Configure the exporter based on command line argument
    let exporter = match exporter_type {
        "jaeger" => TracingExporter::Jaeger {
            endpoint: "http://localhost:14268/api/traces".to_string(),
        },
        "zipkin" => TracingExporter::Zipkin {
            endpoint: "http://localhost:9411/api/v2/spans".to_string(),
        },
        "otlp" => TracingExporter::Otlp {
            endpoint: "http://localhost:4317".to_string(),
        },
        _ => TracingExporter::None,
    };

    // Initialize tracing
    let config = TracingConfig {
        service_name: "elara-tracing-example".to_string(),
        exporter,
        sampling_rate: 1.0, // Sample all traces for the example
        resource_attributes: vec![
            ("environment".to_string(), "development".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ],
    };

    println!("Initializing tracing with {:?} exporter...", exporter_type);
    let handle = init_tracing(config).await?;
    println!("Tracing initialized successfully!");

    // Create some example spans
    let root_span = span!(Level::INFO, "example_operation");
    let _enter = root_span.enter();

    info!("Starting example operation");

    // Nested span
    {
        let child_span = span!(Level::INFO, "child_operation", operation = "processing");
        let _child_enter = child_span.enter();

        info!("Processing data in child span");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        info!("Child operation complete");
    }

    // Another nested span
    {
        let child_span = span!(Level::INFO, "another_operation", count = 42);
        let _child_enter = child_span.enter();

        info!("Performing another operation");
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        info!("Another operation complete");
    }

    info!("Example operation complete");

    // Give time for traces to be exported
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Graceful shutdown
    println!("Shutting down tracing...");
    handle.shutdown().await?;
    println!("Tracing shutdown complete!");

    Ok(())
}
