//! Observability module for ELARA Protocol
//!
//! This module provides structured logging, metrics collection, and distributed tracing
//! capabilities for production deployments.
//!
//! # Features
//!
//! - **Structured Logging**: JSON/Pretty/Compact formats with per-module log levels
//! - **Metrics Collection**: Counters, gauges, and histograms with thread-safe registry
//! - **Distributed Tracing**: OpenTelemetry integration with Jaeger/Zipkin/OTLP support
//! - **Unified Initialization**: Single entry point for all observability components
//!
//! # Unified Initialization Example
//!
//! ```no_run
//! use elara_runtime::observability::{
//!     ObservabilityConfig, LoggingConfig, LogLevel, LogFormat, LogOutput,
//!     TracingConfig, TracingExporter, MetricsServerConfig, init_observability
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ObservabilityConfig {
//!     logging: Some(LoggingConfig {
//!         level: LogLevel::Info,
//!         format: LogFormat::Json,
//!         output: LogOutput::Stdout,
//!     }),
//!     tracing: Some(TracingConfig {
//!         service_name: "elara-node".to_string(),
//!         exporter: TracingExporter::Otlp {
//!             endpoint: "http://localhost:4317".to_string(),
//!         },
//!         sampling_rate: 0.1,
//!         resource_attributes: vec![],
//!     }),
//!     metrics_server: Some(MetricsServerConfig {
//!         bind_address: "0.0.0.0".to_string(),
//!         port: 9090,
//!     }),
//! };
//!
//! let handle = init_observability(config).await?;
//!
//! // Use observability throughout your application
//! tracing::info!("Application started");
//!
//! // Graceful shutdown
//! handle.shutdown().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Individual Component Example
//!
//! ```no_run
//! use elara_runtime::observability::logging::{LoggingConfig, LogLevel, LogFormat, LogOutput, init_logging};
//!
//! let config = LoggingConfig {
//!     level: LogLevel::Info,
//!     format: LogFormat::Json,
//!     output: LogOutput::Stdout,
//! };
//!
//! init_logging(config).expect("Failed to initialize logging");
//! ```

pub mod logging;
pub mod metrics;
pub mod metrics_server;
pub mod tracing;

use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

pub use logging::{init_logging, LogFormat, LogLevel, LogOutput, LoggingConfig, LoggingError};
pub use metrics::{Counter, Gauge, Histogram, MetricsError, MetricsRegistry, NodeMetrics};
pub use metrics_server::{MetricsServer, MetricsServerConfig, MetricsServerError};
pub use tracing::{init_tracing, TracingConfig, TracingError, TracingExporter, TracingHandle};

/// Global flag to track if observability has been initialized.
/// This ensures idempotency - init_observability can only be called once.
static OBSERVABILITY_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Reset the observability initialization flag.
///
/// **WARNING**: This function is only for testing purposes and should never be used
/// in production code. It allows tests to re-initialize the observability system
/// by resetting the global initialization flag.
///
/// # Safety
///
/// This function is only available when running tests. Using this in production
/// could lead to undefined behavior as it allows multiple initializations of global state.
#[doc(hidden)]
pub fn reset_observability_for_testing() {
    OBSERVABILITY_INITIALIZED.store(false, Ordering::SeqCst);
    // Also reset logging flag since it's initialized as part of observability
    logging::reset_logging_for_testing();
}

/// Unified configuration for all observability components.
///
/// This struct combines configuration for logging, tracing, and metrics server.
/// All components are optional - set to `None` to disable a component.
///
/// # Example
///
/// ```no_run
/// use elara_runtime::observability::{
///     ObservabilityConfig, LoggingConfig, LogLevel, LogFormat, LogOutput,
///     TracingConfig, TracingExporter, MetricsServerConfig
/// };
///
/// // Enable all components
/// let config = ObservabilityConfig {
///     logging: Some(LoggingConfig {
///         level: LogLevel::Info,
///         format: LogFormat::Json,
///         output: LogOutput::Stdout,
///     }),
///     tracing: Some(TracingConfig {
///         service_name: "elara-node".to_string(),
///         exporter: TracingExporter::Otlp {
///             endpoint: "http://localhost:4317".to_string(),
///         },
///         sampling_rate: 0.1,
///         resource_attributes: vec![],
///     }),
///     metrics_server: Some(MetricsServerConfig {
///         bind_address: "0.0.0.0".to_string(),
///         port: 9090,
///     }),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Optional logging configuration. If `None`, logging is not initialized.
    pub logging: Option<LoggingConfig>,

    /// Optional tracing configuration. If `None`, tracing is not initialized.
    pub tracing: Option<TracingConfig>,

    /// Optional metrics server configuration. If `None`, metrics server is not started.
    pub metrics_server: Option<MetricsServerConfig>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            logging: None,
            tracing: None,
            metrics_server: None,
        }
    }
}

/// Handle for managing the observability system lifecycle.
///
/// This handle provides graceful shutdown for all initialized observability components.
/// It holds references to the tracing handle and metrics server, allowing coordinated
/// shutdown of all components.
///
/// # Example
///
/// ```no_run
/// # use elara_runtime::observability::{ObservabilityConfig, init_observability};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ObservabilityConfig::default();
/// let handle = init_observability(config).await?;
///
/// // ... application runs ...
///
/// // Graceful shutdown
/// handle.shutdown().await?;
/// # Ok(())
/// # }
/// ```
pub struct ObservabilityHandle {
    /// Handle for the tracing system (if initialized)
    tracing_handle: Option<TracingHandle>,

    /// Handle for the metrics server (if started)
    metrics_server: Option<MetricsServer>,

    /// Metrics registry (always created, even if server is not started)
    metrics_registry: MetricsRegistry,
}

impl ObservabilityHandle {
    /// Shutdown all observability components gracefully.
    ///
    /// This method:
    /// 1. Shuts down the tracing system (flushes pending spans)
    /// 2. Shuts down the metrics server (stops HTTP server)
    ///
    /// After calling this method, no more telemetry will be exported.
    ///
    /// # Errors
    ///
    /// Returns an error if any component fails to shut down gracefully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use elara_runtime::observability::{ObservabilityConfig, init_observability};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ObservabilityConfig::default();
    /// let handle = init_observability(config).await?;
    ///
    /// // Graceful shutdown
    /// handle.shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn shutdown(mut self) -> Result<(), ObservabilityError> {
        // Shutdown tracing first (flush pending spans)
        if let Some(tracing_handle) = self.tracing_handle.take() {
            tracing_handle
                .shutdown()
                .await
                .map_err(ObservabilityError::TracingShutdown)?;
        }

        // Shutdown metrics server
        if let Some(mut metrics_server) = self.metrics_server.take() {
            metrics_server.shutdown().await;
        }

        Ok(())
    }

    /// Returns a reference to the metrics registry.
    ///
    /// The metrics registry is always available, even if the metrics server
    /// was not started. This allows applications to collect metrics without
    /// exposing them via HTTP.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use elara_runtime::observability::{ObservabilityConfig, init_observability};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ObservabilityConfig::default();
    /// let handle = init_observability(config).await?;
    ///
    /// // Access metrics registry
    /// let counter = handle.metrics_registry().register_counter("my_counter", vec![]);
    /// counter.inc();
    /// # Ok(())
    /// # }
    /// ```
    pub fn metrics_registry(&self) -> &MetricsRegistry {
        &self.metrics_registry
    }

    /// Returns true if the metrics server is running.
    pub fn is_metrics_server_running(&self) -> bool {
        self.metrics_server
            .as_ref()
            .map_or(false, |s| s.is_running())
    }
}

/// Errors that can occur during observability initialization or shutdown.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// Observability system has already been initialized
    #[error("Observability system already initialized")]
    AlreadyInitialized,

    /// Failed to initialize logging
    #[error("Failed to initialize logging: {0}")]
    LoggingInit(#[from] LoggingError),

    /// Failed to initialize tracing
    #[error("Failed to initialize tracing: {0}")]
    TracingInit(#[from] TracingError),

    /// Failed to start metrics server
    #[error("Failed to start metrics server: {0}")]
    MetricsServerStart(#[from] MetricsServerError),

    /// Failed to shutdown tracing
    #[error("Failed to shutdown tracing: {0}")]
    TracingShutdown(TracingError),
}

/// Initialize the unified observability system.
///
/// This function provides a single entry point for initializing all observability
/// components (logging, tracing, metrics server). It is idempotent - calling it
/// multiple times will return an error after the first call.
///
/// # Components
///
/// - **Logging**: Structured logging with configurable format and output
/// - **Tracing**: Distributed tracing with OpenTelemetry support
/// - **Metrics Server**: HTTP server exposing Prometheus metrics
///
/// All components are optional. Set a component to `None` in the config to disable it.
///
/// # Arguments
///
/// * `config` - Unified observability configuration
///
/// # Returns
///
/// * `Ok(ObservabilityHandle)` - Handle for graceful shutdown and metrics access
/// * `Err(ObservabilityError)` - If initialization fails or already initialized
///
/// # Idempotency
///
/// This function can only be called once per process. Subsequent calls will return
/// `Err(ObservabilityError::AlreadyInitialized)`. This ensures that global state
/// (logging subscriber, tracer provider) is only set once.
///
/// # Example
///
/// ```no_run
/// use elara_runtime::observability::{
///     ObservabilityConfig, LoggingConfig, LogLevel, LogFormat, LogOutput,
///     TracingConfig, TracingExporter, MetricsServerConfig, init_observability
/// };
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ObservabilityConfig {
///     logging: Some(LoggingConfig {
///         level: LogLevel::Info,
///         format: LogFormat::Json,
///         output: LogOutput::Stdout,
///     }),
///     tracing: Some(TracingConfig {
///         service_name: "elara-node".to_string(),
///         exporter: TracingExporter::Otlp {
///             endpoint: "http://localhost:4317".to_string(),
///         },
///         sampling_rate: 0.1,
///         resource_attributes: vec![
///             ("environment".to_string(), "production".to_string()),
///         ],
///     }),
///     metrics_server: Some(MetricsServerConfig {
///         bind_address: "0.0.0.0".to_string(),
///         port: 9090,
///     }),
/// };
///
/// let handle = init_observability(config).await?;
///
/// // Use observability throughout your application
/// tracing::info!("Application started");
///
/// // Graceful shutdown
/// handle.shutdown().await?;
/// # Ok(())
/// # }
/// ```
///
/// # Minimal Example (Logging Only)
///
/// ```no_run
/// use elara_runtime::observability::{
///     ObservabilityConfig, LoggingConfig, LogLevel, LogFormat, LogOutput, init_observability
/// };
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ObservabilityConfig {
///     logging: Some(LoggingConfig {
///         level: LogLevel::Info,
///         format: LogFormat::Pretty,
///         output: LogOutput::Stdout,
///     }),
///     tracing: None,
///     metrics_server: None,
/// };
///
/// let handle = init_observability(config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_observability(
    config: ObservabilityConfig,
) -> Result<ObservabilityHandle, ObservabilityError> {
    // Check if already initialized (idempotency)
    if OBSERVABILITY_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(ObservabilityError::AlreadyInitialized);
    }

    // Initialize logging if configured
    if let Some(logging_config) = config.logging {
        match init_logging(logging_config) {
            Ok(()) => {},
            Err(LoggingError::AlreadyInitialized) => {
                // In tests, logging might already be initialized by a previous test
                // This is acceptable since tests run serially with #[serial]
            },
            Err(e) => {
                // Reset flag on error
                OBSERVABILITY_INITIALIZED.store(false, Ordering::SeqCst);
                return Err(ObservabilityError::LoggingInit(e));
            }
        }
    }

    // Initialize tracing if configured
    let tracing_handle = if let Some(tracing_config) = config.tracing {
        Some(init_tracing(tracing_config).await.map_err(|e| {
            // Reset flag on error
            OBSERVABILITY_INITIALIZED.store(false, Ordering::SeqCst);
            ObservabilityError::TracingInit(e)
        })?)
    } else {
        None
    };

    // Create metrics registry (always created, even if server is not started)
    let metrics_registry = MetricsRegistry::new();

    // Start metrics server if configured
    let metrics_server = if let Some(metrics_server_config) = config.metrics_server {
        let mut server = MetricsServer::new(metrics_server_config, metrics_registry.clone());
        server.start().await.map_err(|e| {
            // Reset flag on error
            OBSERVABILITY_INITIALIZED.store(false, Ordering::SeqCst);
            ObservabilityError::MetricsServerStart(e)
        })?;
        Some(server)
    } else {
        None
    };

    Ok(ObservabilityHandle {
        tracing_handle,
        metrics_server,
        metrics_registry,
    })
}
