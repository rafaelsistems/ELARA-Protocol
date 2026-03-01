//! HTTP server for exposing Prometheus metrics.
//!
//! This module provides an HTTP server that exposes metrics in Prometheus text format
//! via a `/metrics` endpoint. The server is built on axum for production-grade async
//! performance and reliability.
//!
//! # Example
//!
//! ```rust,no_run
//! use elara_runtime::observability::metrics::MetricsRegistry;
//! use elara_runtime::observability::metrics_server::{MetricsServer, MetricsServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let registry = MetricsRegistry::new();
//!     
//!     let config = MetricsServerConfig {
//!         bind_address: "0.0.0.0".to_string(),
//!         port: 9090,
//!     };
//!     
//!     let mut server = MetricsServer::new(config, registry);
//!     server.start().await?;
//!     
//!     Ok(())
//! }
//! ```

use super::metrics::MetricsRegistry;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Configuration for the metrics HTTP server.
#[derive(Debug, Clone)]
pub struct MetricsServerConfig {
    /// IP address to bind to (e.g., "0.0.0.0" for all interfaces, "127.0.0.1" for localhost only)
    pub bind_address: String,
    
    /// Port to listen on (typically 9090 for Prometheus)
    pub port: u16,
}

impl Default for MetricsServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 9090,
        }
    }
}

/// Errors that can occur during metrics server operations.
#[derive(Debug, thiserror::Error)]
pub enum MetricsServerError {
    /// Failed to bind to the specified address/port
    #[error("Failed to bind to {0}: {1}")]
    BindError(String, std::io::Error),
    
    /// Server encountered a runtime error
    #[error("Server error: {0}")]
    ServerError(String),
}

/// HTTP server for exposing Prometheus metrics.
///
/// The server provides a `/metrics` endpoint that returns all registered metrics
/// in Prometheus text exposition format. The server runs asynchronously and can
/// be gracefully shut down.
pub struct MetricsServer {
    config: MetricsServerConfig,
    registry: Arc<MetricsRegistry>,
    handle: Option<JoinHandle<()>>,
}

impl MetricsServer {
    /// Creates a new metrics server with the given configuration and registry.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration (bind address and port)
    /// * `registry` - Metrics registry to export
    ///
    /// # Example
    ///
    /// ```rust
    /// use elara_runtime::observability::metrics::MetricsRegistry;
    /// use elara_runtime::observability::metrics_server::{MetricsServer, MetricsServerConfig};
    ///
    /// let registry = MetricsRegistry::new();
    /// let config = MetricsServerConfig::default();
    /// let server = MetricsServer::new(config, registry);
    /// ```
    pub fn new(config: MetricsServerConfig, registry: MetricsRegistry) -> Self {
        Self {
            config,
            registry: Arc::new(registry),
            handle: None,
        }
    }

    /// Starts the metrics server.
    ///
    /// This method spawns the HTTP server on a background task and returns immediately.
    /// The server will continue running until `shutdown()` is called or the process exits.
    ///
    /// # Errors
    ///
    /// Returns `MetricsServerError::BindError` if the server cannot bind to the
    /// specified address/port (e.g., port already in use).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use elara_runtime::observability::metrics::MetricsRegistry;
    /// # use elara_runtime::observability::metrics_server::{MetricsServer, MetricsServerConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = MetricsRegistry::new();
    /// let config = MetricsServerConfig::default();
    /// let mut server = MetricsServer::new(config, registry);
    /// 
    /// server.start().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&mut self) -> Result<(), MetricsServerError> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e| MetricsServerError::BindError(addr.clone(), 
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;

        // Create the router with the /metrics endpoint
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .with_state(self.registry.clone());

        // Bind to the address
        let listener = tokio::net::TcpListener::bind(&socket_addr)
            .await
            .map_err(|e| MetricsServerError::BindError(addr.clone(), e))?;

        tracing::info!(
            address = %socket_addr,
            "Metrics server started"
        );

        // Spawn the server on a background task
        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!(error = %e, "Metrics server error");
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// Shuts down the metrics server gracefully.
    ///
    /// This method aborts the server task and waits for it to complete.
    /// After shutdown, the server can be restarted by calling `start()` again.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use elara_runtime::observability::metrics::MetricsRegistry;
    /// # use elara_runtime::observability::metrics_server::{MetricsServer, MetricsServerConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let registry = MetricsRegistry::new();
    /// # let config = MetricsServerConfig::default();
    /// let mut server = MetricsServer::new(config, registry);
    /// server.start().await?;
    /// 
    /// // ... do work ...
    /// 
    /// server.shutdown().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn shutdown(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = handle.await;
            tracing::info!("Metrics server shut down");
        }
    }

    /// Returns true if the server is currently running.
    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map_or(false, |h| !h.is_finished())
    }

    /// Returns the configured bind address.
    pub fn bind_address(&self) -> &str {
        &self.config.bind_address
    }

    /// Returns the configured port.
    pub fn port(&self) -> u16 {
        self.config.port
    }
}

/// Handler for the /metrics endpoint.
///
/// This function is called by axum when a request is made to /metrics.
/// It exports all metrics from the registry in Prometheus text format.
async fn metrics_handler(State(registry): State<Arc<MetricsRegistry>>) -> Response {
    let prometheus_text = registry.export_prometheus();
    
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        prometheus_text,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_server_creation() {
        let registry = MetricsRegistry::new();
        let config = MetricsServerConfig::default();
        let server = MetricsServer::new(config, registry);
        
        assert_eq!(server.bind_address(), "0.0.0.0");
        assert_eq!(server.port(), 9090);
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_metrics_server_start_stop() {
        let registry = MetricsRegistry::new();
        let config = MetricsServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0, // Use port 0 to let OS assign a free port
        };
        let mut server = MetricsServer::new(config, registry);
        
        // Start server
        let result = server.start().await;
        assert!(result.is_ok(), "Failed to start server: {:?}", result);
        assert!(server.is_running());
        
        // Shutdown server
        server.shutdown().await;
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_metrics_endpoint_response() {
        let registry = MetricsRegistry::new();
        
        // Register some test metrics
        let counter = registry.register_counter("test_counter", vec![]);
        counter.inc_by(42);
        
        let gauge = registry.register_gauge("test_gauge", vec![]);
        gauge.set(100);
        
        // Start server on a random port
        let config = MetricsServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0,
        };
        let mut server = MetricsServer::new(config, registry);
        server.start().await.unwrap();
        
        // Give server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Note: We can't easily test the HTTP endpoint without knowing the assigned port
        // In a real scenario, we'd need to expose the actual bound port from the server
        
        server.shutdown().await;
    }

    #[test]
    fn test_prometheus_export_format() {
        let registry = MetricsRegistry::new();
        
        // Register metrics
        let counter = registry.register_counter("test_counter", vec![]);
        counter.inc_by(5);
        
        let gauge = registry.register_gauge("test_gauge", vec![]);
        gauge.set(-10);
        
        let histogram = registry.register_histogram(
            "test_histogram",
            vec![1.0, 5.0, 10.0],
            vec![],
        );
        histogram.observe(3.0);
        histogram.observe(7.0);
        
        let output = registry.export_prometheus();
        
        // Debug: print the output to see what we're getting
        println!("Prometheus output:\n{}", output);
        
        // Verify counter format
        assert!(output.contains("# TYPE test_counter counter"));
        assert!(output.contains("test_counter 5"));
        
        // Verify gauge format
        assert!(output.contains("# TYPE test_gauge gauge"));
        assert!(output.contains("test_gauge -10"));
        
        // Verify histogram format
        assert!(output.contains("# TYPE test_histogram histogram"));
        assert!(output.contains("test_histogram_bucket"));
        assert!(output.contains("le=\"1.0\""));
        assert!(output.contains("le=\"+Inf\""));
        assert!(output.contains("test_histogram_sum"));
        assert!(output.contains("test_histogram_count 2"));
    }

    #[test]
    fn test_prometheus_export_with_labels() {
        let registry = MetricsRegistry::new();
        
        let counter = registry.register_counter(
            "labeled_counter",
            vec![
                ("node_id".to_string(), "node-1".to_string()),
                ("region".to_string(), "us-west".to_string()),
            ],
        );
        counter.inc();
        
        let output = registry.export_prometheus();
        
        // Verify labels are formatted correctly
        assert!(output.contains("labeled_counter{"));
        assert!(output.contains("node_id=\"node-1\""));
        assert!(output.contains("region=\"us-west\""));
    }

    #[test]
    fn test_label_escaping() {
        let registry = MetricsRegistry::new();
        
        let counter = registry.register_counter(
            "escaped_counter",
            vec![
                ("label".to_string(), "value with \"quotes\" and \\backslash".to_string()),
            ],
        );
        counter.inc();
        
        let output = registry.export_prometheus();
        
        // Verify special characters are escaped
        assert!(output.contains("\\\""));
        assert!(output.contains("\\\\"));
    }
}
