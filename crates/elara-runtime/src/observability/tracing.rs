//! Distributed tracing system with OpenTelemetry support.
//!
//! This module provides distributed tracing capabilities for the ELARA Protocol,
//! enabling end-to-end request tracing across nodes. It supports multiple exporter
//! backends (Jaeger, Zipkin, OTLP) and configurable sampling rates.
//!
//! # Examples
//!
//! ```no_run
//! use elara_runtime::observability::tracing::{TracingConfig, TracingExporter, init_tracing};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = TracingConfig {
//!     service_name: "elara-node".to_string(),
//!     exporter: TracingExporter::Jaeger {
//!         endpoint: "http://localhost:14268/api/traces".to_string(),
//!     },
//!     sampling_rate: 1.0,
//!     resource_attributes: vec![
//!         ("environment".to_string(), "production".to_string()),
//!     ],
//! };
//!
//! let handle = init_tracing(config).await?;
//!
//! // Use tracing throughout your application
//! tracing::info!("Application started");
//!
//! // Graceful shutdown
//! handle.shutdown().await?;
//! # Ok(())
//! # }
//! ```

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, TracerProvider};
use opentelemetry_sdk::Resource;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::Span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

/// Configuration for the distributed tracing system.
///
/// This struct defines all parameters needed to initialize OpenTelemetry tracing,
/// including the service name, exporter backend, sampling rate, and resource attributes.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name to identify this node in traces
    pub service_name: String,

    /// Exporter backend configuration
    pub exporter: TracingExporter,

    /// Sampling rate (0.0 to 1.0)
    /// - 0.0: No traces are sampled
    /// - 1.0: All traces are sampled
    /// - 0.1: 10% of traces are sampled
    pub sampling_rate: f64,

    /// Additional resource attributes (key-value pairs)
    /// These are attached to all spans and can be used for filtering/grouping
    pub resource_attributes: Vec<(String, String)>,
}

/// Exporter backend configuration.
///
/// Supports multiple OpenTelemetry-compatible backends for trace export.
#[derive(Debug, Clone)]
pub enum TracingExporter {
    /// Jaeger exporter (good for development/testing)
    /// Endpoint format: "http://localhost:14268/api/traces"
    Jaeger { endpoint: String },

    /// Zipkin exporter (for compatibility with Zipkin infrastructure)
    /// Endpoint format: "http://localhost:9411/api/v2/spans"
    Zipkin { endpoint: String },

    /// OTLP (OpenTelemetry Protocol) exporter (production standard)
    /// Endpoint format: "http://localhost:4317" (gRPC)
    Otlp { endpoint: String },

    /// No tracing (disabled)
    None,
}

/// Handle for managing the tracing system lifecycle.
///
/// This handle allows for graceful shutdown of the tracing system,
/// ensuring all pending spans are flushed before termination.
#[derive(Clone)]
pub struct TracingHandle {
    initialized: Arc<AtomicBool>,
}

impl TracingHandle {
    /// Shutdown the tracing system gracefully.
    ///
    /// This flushes all pending spans and shuts down the exporter.
    /// After calling this, no more traces will be exported.
    pub async fn shutdown(self) -> Result<(), TracingError> {
        if self.initialized.load(Ordering::SeqCst) {
            global::shutdown_tracer_provider();
            self.initialized.store(false, Ordering::SeqCst);
        }
        Ok(())
    }
}

/// Errors that can occur during tracing initialization or operation.
#[derive(Debug, Error)]
pub enum TracingError {
    /// Tracing system has already been initialized
    #[error("Tracing system already initialized")]
    AlreadyInitialized,

    /// Invalid sampling rate (must be between 0.0 and 1.0)
    #[error("Invalid sampling rate: {0} (must be between 0.0 and 1.0)")]
    InvalidSamplingRate(f64),

    /// Failed to initialize the exporter
    #[error("Failed to initialize exporter: {0}")]
    ExporterInitialization(String),

    /// Failed to set global tracer provider
    #[error("Failed to set global tracer provider: {0}")]
    GlobalTracerSetup(String),
}

/// Global flag to track if tracing has been initialized.
/// This ensures idempotency - init_tracing can only be called once.
static TRACING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the distributed tracing system.
///
/// This function sets up OpenTelemetry tracing with the specified configuration.
/// It is idempotent - calling it multiple times will return an error after the first call.
///
/// # Arguments
///
/// * `config` - Tracing configuration including service name, exporter, and sampling rate
///
/// # Returns
///
/// * `Ok(TracingHandle)` - Handle for graceful shutdown
/// * `Err(TracingError)` - If initialization fails or tracing is already initialized
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::{TracingConfig, TracingExporter, init_tracing};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = TracingConfig {
///     service_name: "elara-node".to_string(),
///     exporter: TracingExporter::Otlp {
///         endpoint: "http://localhost:4317".to_string(),
///     },
///     sampling_rate: 0.1, // Sample 10% of traces
///     resource_attributes: vec![
///         ("node.id".to_string(), "node-1".to_string()),
///         ("environment".to_string(), "production".to_string()),
///     ],
/// };
///
/// let handle = init_tracing(config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_tracing(config: TracingConfig) -> Result<TracingHandle, TracingError> {
    // Check if already initialized (idempotency)
    if TRACING_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(TracingError::AlreadyInitialized);
    }

    // Validate sampling rate
    if config.sampling_rate < 0.0 || config.sampling_rate > 1.0 {
        TRACING_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(TracingError::InvalidSamplingRate(config.sampling_rate));
    }

    // Handle disabled tracing
    if matches!(config.exporter, TracingExporter::None) {
        TRACING_INITIALIZED.store(false, Ordering::SeqCst);
        return Ok(TracingHandle {
            initialized: Arc::new(AtomicBool::new(false)),
        });
    }

    // Build resource with service name and custom attributes
    let mut resource_kvs = vec![KeyValue::new("service.name", config.service_name.clone())];
    for (key, value) in config.resource_attributes {
        resource_kvs.push(KeyValue::new(key, value));
    }
    let resource = Resource::new(resource_kvs);

    // Configure sampler based on sampling rate
    let sampler = if config.sampling_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sampling_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sampling_rate)
    };

    // Initialize the appropriate exporter
    let tracer_provider = match config.exporter {
        TracingExporter::Jaeger { endpoint } => {
            init_jaeger_exporter(&endpoint, resource, sampler).await?
        }
        TracingExporter::Zipkin { endpoint } => {
            init_zipkin_exporter(&endpoint, resource, sampler).await?
        }
        TracingExporter::Otlp { endpoint } => {
            init_otlp_exporter(&endpoint, resource, sampler).await?
        }
        TracingExporter::None => unreachable!(), // Already handled above
    };

    // Set as global tracer provider
    global::set_tracer_provider(tracer_provider.clone());

    // Create OpenTelemetry tracing layer using a concrete tracer from the provider
    // Note: We use the provider's tracer() method instead of global::tracer()
    // to get a concrete Tracer type that implements PreSampledTracer
    let tracer = tracer_provider.tracer("elara-runtime");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Install the layer with the existing subscriber
    // Note: This assumes a subscriber is already set up (e.g., by logging module)
    // If not, we create a basic one
    let subscriber = Registry::default().with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber).map_err(|e| {
        TRACING_INITIALIZED.store(false, Ordering::SeqCst);
        TracingError::GlobalTracerSetup(e.to_string())
    })?;

    Ok(TracingHandle {
        initialized: Arc::new(AtomicBool::new(true)),
    })
}

/// Initialize Jaeger exporter.
async fn init_jaeger_exporter(
    endpoint: &str,
    resource: Resource,
    sampler: Sampler,
) -> Result<TracerProvider, TracingError> {
    let exporter = opentelemetry_jaeger::new_agent_pipeline()
        .with_endpoint(endpoint)
        .with_service_name("elara-node")
        .build_async_agent_exporter(opentelemetry_sdk::runtime::Tokio)
        .map_err(|e| TracingError::ExporterInitialization(e.to_string()))?;

    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .build();

    Ok(tracer_provider)
}

/// Initialize Zipkin exporter.
async fn init_zipkin_exporter(
    endpoint: &str,
    resource: Resource,
    sampler: Sampler,
) -> Result<TracerProvider, TracingError> {
    let exporter = opentelemetry_zipkin::new_pipeline()
        .with_collector_endpoint(endpoint)
        .init_exporter()
        .map_err(|e| TracingError::ExporterInitialization(e.to_string()))?;

    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .build();

    Ok(tracer_provider)
}

/// Initialize OTLP exporter.
async fn init_otlp_exporter(
    endpoint: &str,
    resource: Resource,
    sampler: Sampler,
) -> Result<TracerProvider, TracingError> {
    use opentelemetry_otlp::WithExportConfig;

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .build_span_exporter()
        .map_err(|e| TracingError::ExporterInitialization(e.to_string()))?;

    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .build();

    Ok(tracer_provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tracing_config_validation() {
        // Test invalid sampling rate
        let config = TracingConfig {
            service_name: "test".to_string(),
            exporter: TracingExporter::None,
            sampling_rate: 1.5,
            resource_attributes: vec![],
        };

        let result = init_tracing(config).await;
        assert!(matches!(result, Err(TracingError::InvalidSamplingRate(_))));
    }

    #[tokio::test]
    async fn test_disabled_tracing() {
        let config = TracingConfig {
            service_name: "test".to_string(),
            exporter: TracingExporter::None,
            sampling_rate: 1.0,
            resource_attributes: vec![],
        };

        let result = init_tracing(config).await;
        assert!(result.is_ok());
    }
}

/// Span helper functions for common tracing patterns.
///
/// These functions create pre-configured spans for common operations in the ELARA Protocol,
/// making it easier to maintain consistent tracing across the codebase.

/// Create a span for message send operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node sending the message
/// * `session_id` - Optional session ID
/// * `message_count` - Number of messages being sent
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_message_send;
/// use elara_core::NodeId;
///
/// let span = span_message_send(NodeId(1), Some(42), 5);
/// let _enter = span.enter();
/// // ... send messages ...
/// ```
pub fn span_message_send(node_id: u64, session_id: Option<u64>, message_count: usize) -> Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "message_send",
        node_id = node_id,
        session_id = ?session_id,
        message_count = message_count,
    )
}

/// Create a span for message receive operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node receiving the message
/// * `session_id` - Optional session ID
/// * `message_count` - Number of messages being received
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_message_receive;
/// use elara_core::NodeId;
///
/// let span = span_message_receive(NodeId(1), Some(42), 3);
/// let _enter = span.enter();
/// // ... receive messages ...
/// ```
pub fn span_message_receive(node_id: u64, session_id: Option<u64>, message_count: usize) -> Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "message_receive",
        node_id = node_id,
        session_id = ?session_id,
        message_count = message_count,
    )
}

/// Create a span for state synchronization operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node performing state sync
/// * `session_id` - Optional session ID
/// * `event_count` - Number of events being processed
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_state_sync;
/// use elara_core::NodeId;
///
/// let span = span_state_sync(NodeId(1), Some(42), 10);
/// let _enter = span.enter();
/// // ... synchronize state ...
/// ```
pub fn span_state_sync(node_id: u64, session_id: Option<u64>, event_count: usize) -> Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "state_sync",
        node_id = node_id,
        session_id = ?session_id,
        event_count = event_count,
    )
}

/// Create a span for connection establishment operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node establishing the connection
/// * `session_id` - The session ID being joined
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_connection_establish;
/// use elara_core::NodeId;
///
/// let span = span_connection_establish(NodeId(1), 42);
/// let _enter = span.enter();
/// // ... establish connection ...
/// ```
pub fn span_connection_establish(node_id: u64, session_id: u64) -> Span {
    tracing::span!(
        tracing::Level::INFO,
        "connection_establish",
        node_id = node_id,
        session_id = session_id,
    )
}

/// Create a span for connection teardown operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node tearing down the connection
/// * `session_id` - Optional session ID being left
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_connection_teardown;
/// use elara_core::NodeId;
///
/// let span = span_connection_teardown(NodeId(1), Some(42));
/// let _enter = span.enter();
/// // ... teardown connection ...
/// ```
pub fn span_connection_teardown(node_id: u64, session_id: Option<u64>) -> Span {
    tracing::span!(
        tracing::Level::INFO,
        "connection_teardown",
        node_id = node_id,
        session_id = ?session_id,
    )
}

/// Create a span for node tick operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node performing the tick
/// * `session_id` - Optional session ID
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_node_tick;
/// use elara_core::NodeId;
///
/// let span = span_node_tick(NodeId(1), Some(42));
/// let _enter = span.enter();
/// // ... perform tick ...
/// ```
pub fn span_node_tick(node_id: u64, session_id: Option<u64>) -> Span {
    tracing::span!(
        tracing::Level::INFO,
        "node_tick",
        node_id = node_id,
        session_id = ?session_id,
    )
}

/// Create a span for decryption operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node performing decryption
/// * `packet_count` - Number of packets being decrypted
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_decrypt;
/// use elara_core::NodeId;
///
/// let span = span_decrypt(NodeId(1), 5);
/// let _enter = span.enter();
/// // ... decrypt packets ...
/// ```
pub fn span_decrypt(node_id: u64, packet_count: usize) -> Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "decrypt",
        node_id = node_id,
        packet_count = packet_count,
    )
}

/// Create a span for event classification operations.
///
/// # Arguments
///
/// * `node_id` - The ID of the node classifying events
/// * `packet_count` - Number of packets being classified
///
/// # Examples
///
/// ```no_run
/// use elara_runtime::observability::tracing::span_classify_events;
/// use elara_core::NodeId;
///
/// let span = span_classify_events(NodeId(1), 3);
/// let _enter = span.enter();
/// // ... classify events ...
/// ```
pub fn span_classify_events(node_id: u64, packet_count: usize) -> Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "classify_events",
        node_id = node_id,
        packet_count = packet_count,
    )
}
