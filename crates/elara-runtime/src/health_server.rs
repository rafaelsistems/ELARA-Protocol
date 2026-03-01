//! Health Check HTTP Server for ELARA Runtime
//!
//! This module provides a production-grade HTTP server for exposing health check
//! endpoints. It is designed to integrate with Kubernetes liveness and readiness
//! probes, load balancers, and monitoring systems.
//!
//! # Endpoints
//!
//! ## `/health` - Overall Health Status
//!
//! Returns the overall health status of the node, including all registered health
//! checks. This endpoint is suitable for general health monitoring and alerting.
//!
//! **Response Codes:**
//! - `200 OK` - Node is Healthy or Degraded (can serve traffic)
//! - `503 Service Unavailable` - Node is Unhealthy (should not serve traffic)
//!
//! **Response Body:**
//! ```json
//! {
//!   "status": "healthy" | "degraded" | "unhealthy",
//!   "timestamp": "2024-01-15T10:30:00Z",
//!   "checks": {
//!     "connections": {
//!       "status": "healthy",
//!       "reason": null
//!     },
//!     "memory": {
//!       "status": "healthy",
//!       "reason": null
//!     }
//!   }
//! }
//! ```
//!
//! ## `/ready` - Readiness Probe
//!
//! Kubernetes readiness probe endpoint. Indicates whether the node is ready to
//! accept traffic. A node may be alive but not ready (e.g., still initializing,
//! warming up caches, establishing connections).
//!
//! **Response Codes:**
//! - `200 OK` - Node is ready to accept traffic
//! - `503 Service Unavailable` - Node is not ready
//!
//! ## `/live` - Liveness Probe
//!
//! Kubernetes liveness probe endpoint. Indicates whether the node is alive and
//! functioning. If this check fails, Kubernetes will restart the pod.
//!
//! **Response Codes:**
//! - `200 OK` - Node is alive
//! - `503 Service Unavailable` - Node is deadlocked or unresponsive
//!
//! # Architecture
//!
//! The health server is designed to be:
//! - **Non-blocking**: Uses async/await and Tokio runtime
//! - **Fast**: Leverages cached health check results (no expensive checks on request)
//! - **Lightweight**: Minimal overhead, suitable for high-frequency polling
//! - **Production-ready**: Proper error handling, logging, and graceful shutdown
//!
//! # Example
//!
//! ```rust,no_run
//! use elara_runtime::health::{HealthChecker, MemoryHealthCheck};
//! use elara_runtime::health_server::{HealthServer, HealthServerConfig};
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create health checker
//!     let mut checker = HealthChecker::new(Duration::from_secs(30));
//!     checker.add_check(Box::new(MemoryHealthCheck::new(1800)));
//!     let checker = Arc::new(checker);
//!
//!     // Configure and start health server
//!     let config = HealthServerConfig {
//!         bind_address: "0.0.0.0:8080".parse()?,
//!     };
//!
//!     let server = HealthServer::new(checker, config);
//!     server.serve().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Kubernetes Integration
//!
//! Example Kubernetes deployment configuration:
//!
//! ```yaml
//! apiVersion: v1
//! kind: Pod
//! metadata:
//!   name: elara-node
//! spec:
//!   containers:
//!   - name: elara
//!     image: elara-node:latest
//!     ports:
//!     - containerPort: 8080
//!       name: health
//!     livenessProbe:
//!       httpGet:
//!         path: /live
//!         port: health
//!       initialDelaySeconds: 30
//!       periodSeconds: 10
//!       timeoutSeconds: 5
//!       failureThreshold: 3
//!     readinessProbe:
//!       httpGet:
//!         path: /ready
//!         port: health
//!       initialDelaySeconds: 10
//!       periodSeconds: 5
//!       timeoutSeconds: 3
//!       failureThreshold: 2
//! ```

use crate::health::{HealthChecker, HealthCheckResult, HealthStatus};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{error, info, warn};

/// Configuration for the health check HTTP server.
#[derive(Debug, Clone)]
pub struct HealthServerConfig {
    /// Address to bind the HTTP server to (e.g., "0.0.0.0:8080")
    pub bind_address: SocketAddr,
}

impl Default for HealthServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".parse().unwrap(),
        }
    }
}

/// Health check HTTP server.
///
/// Provides HTTP endpoints for health monitoring, Kubernetes probes,
/// and load balancer health checks.
pub struct HealthServer {
    /// Health checker instance
    checker: Arc<HealthChecker>,
    /// Server configuration
    config: HealthServerConfig,
}

impl HealthServer {
    /// Creates a new HealthServer.
    ///
    /// # Arguments
    ///
    /// * `checker` - Arc reference to the HealthChecker
    /// * `config` - Server configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::HealthChecker;
    /// use elara_runtime::health_server::{HealthServer, HealthServerConfig};
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// let checker = Arc::new(HealthChecker::new(Duration::from_secs(30)));
    /// let config = HealthServerConfig::default();
    /// let server = HealthServer::new(checker, config);
    /// ```
    pub fn new(checker: Arc<HealthChecker>, config: HealthServerConfig) -> Self {
        Self { checker, config }
    }
    
    /// Creates a new HealthServer with default configuration.
    ///
    /// Binds to `0.0.0.0:8080` by default.
    pub fn with_default_config(checker: Arc<HealthChecker>) -> Self {
        Self::new(checker, HealthServerConfig::default())
    }
    
    /// Starts the health check HTTP server.
    ///
    /// This method runs the server until it is shut down. It should be
    /// spawned as a background task in production deployments.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the server shuts down gracefully, or an error
    /// if the server fails to start or encounters a fatal error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::HealthChecker;
    /// use elara_runtime::health_server::HealthServer;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let checker = Arc::new(HealthChecker::new(Duration::from_secs(30)));
    ///     let server = HealthServer::with_default_config(checker);
    ///     
    ///     // Run server (blocks until shutdown)
    ///     server.serve().await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn serve(self) -> Result<(), std::io::Error> {
        let app = self.create_router();
        let addr = self.config.bind_address;
        
        info!("Starting health check server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        
        info!("Health check server listening on {}", addr);
        info!("  - /health - Overall health status");
        info!("  - /ready  - Readiness probe");
        info!("  - /live   - Liveness probe");
        
        axum::serve(listener, app).await?;
        
        info!("Health check server shut down");
        Ok(())
    }
    
    /// Creates the Axum router with all health check endpoints.
    ///
    /// This method is public to allow testing and custom server configurations.
    pub fn create_router(&self) -> Router {
        Router::new()
            .route("/health", get(health_handler))
            .route("/ready", get(ready_handler))
            .route("/live", get(live_handler))
            .with_state(self.checker.clone())
    }
}

/// JSON response for health check endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status: "healthy", "degraded", or "unhealthy"
    pub status: String,
    
    /// ISO 8601 timestamp when the health check was performed
    pub timestamp: String,
    
    /// Individual health check results
    pub checks: std::collections::HashMap<String, CheckResponse>,
}

/// JSON response for an individual health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    /// Status: "healthy", "degraded", or "unhealthy"
    pub status: String,
    
    /// Optional reason for degraded/unhealthy status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<&HealthStatus> for HealthResponse {
    fn from(status: &HealthStatus) -> Self {
        let checks = status
            .checks
            .iter()
            .map(|(name, result)| {
                let check_response = CheckResponse {
                    status: result_to_status_string(result),
                    reason: result.reason().map(|s| s.to_string()),
                };
                (name.clone(), check_response)
            })
            .collect();
        
        Self {
            status: result_to_status_string(&status.overall),
            timestamp: format_timestamp(status.timestamp),
            checks,
        }
    }
}

/// Converts a HealthCheckResult to a status string.
fn result_to_status_string(result: &HealthCheckResult) -> String {
    match result {
        HealthCheckResult::Healthy => "healthy".to_string(),
        HealthCheckResult::Degraded { .. } => "degraded".to_string(),
        HealthCheckResult::Unhealthy { .. } => "unhealthy".to_string(),
    }
}

/// Formats a timestamp as ISO 8601 string.
fn format_timestamp(instant: std::time::Instant) -> String {
    // Convert Instant to SystemTime for ISO 8601 formatting
    // Note: This is an approximation since Instant doesn't have a fixed epoch
    let now = SystemTime::now();
    let elapsed = instant.elapsed();
    
    // Subtract elapsed time from now to get approximate timestamp
    let timestamp = now
        .checked_sub(elapsed)
        .unwrap_or(now);
    
    // Format as ISO 8601
    humantime::format_rfc3339(timestamp).to_string()
}

/// Determines the HTTP status code based on health check result.
fn result_to_status_code(result: &HealthCheckResult) -> StatusCode {
    match result {
        HealthCheckResult::Healthy => StatusCode::OK,
        HealthCheckResult::Degraded { .. } => StatusCode::OK,
        HealthCheckResult::Unhealthy { .. } => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Handler for `/health` endpoint - Overall health status.
///
/// Returns the complete health status including all registered checks.
/// This endpoint is suitable for general health monitoring and alerting.
///
/// **Response Codes:**
/// - `200 OK` - Node is Healthy or Degraded
/// - `503 Service Unavailable` - Node is Unhealthy
async fn health_handler(
    State(checker): State<Arc<HealthChecker>>,
) -> Response {
    let status = checker.check_health();
    let status_code = result_to_status_code(&status.overall);
    let response = HealthResponse::from(&status);
    
    // Log unhealthy status for monitoring
    if status.is_unhealthy() {
        warn!(
            status = "unhealthy",
            reason = ?status.overall.reason(),
            "Health check failed"
        );
    } else if status.is_degraded() {
        warn!(
            status = "degraded",
            reason = ?status.overall.reason(),
            "Health check degraded"
        );
    }
    
    (status_code, Json(response)).into_response()
}

/// Handler for `/ready` endpoint - Readiness probe.
///
/// Kubernetes readiness probe endpoint. Indicates whether the node is ready
/// to accept traffic. A node may be alive but not ready (e.g., still
/// initializing, warming up caches, establishing connections).
///
/// **Response Codes:**
/// - `200 OK` - Node is ready to accept traffic
/// - `503 Service Unavailable` - Node is not ready
///
/// **Implementation Note:**
/// Currently uses the same logic as `/health`. In a production deployment,
/// you may want to implement separate readiness checks that verify:
/// - All required connections are established
/// - Caches are warmed up
/// - Initial state synchronization is complete
async fn ready_handler(
    State(checker): State<Arc<HealthChecker>>,
) -> Response {
    let status = checker.check_health();
    let status_code = result_to_status_code(&status.overall);
    let response = HealthResponse::from(&status);
    
    if status.is_unhealthy() {
        warn!("Readiness probe failed: node not ready");
    }
    
    (status_code, Json(response)).into_response()
}

/// Handler for `/live` endpoint - Liveness probe.
///
/// Kubernetes liveness probe endpoint. Indicates whether the node is alive
/// and functioning. If this check fails, Kubernetes will restart the pod.
///
/// **Response Codes:**
/// - `200 OK` - Node is alive
/// - `503 Service Unavailable` - Node is deadlocked or unresponsive
///
/// **Implementation Note:**
/// Currently uses the same logic as `/health`. In a production deployment,
/// you may want to implement separate liveness checks that verify:
/// - Event loop is not deadlocked
/// - Critical threads are responsive
/// - No fatal errors have occurred
///
/// Liveness checks should be more lenient than readiness checks to avoid
/// unnecessary restarts.
async fn live_handler(
    State(checker): State<Arc<HealthChecker>>,
) -> Response {
    let status = checker.check_health();
    
    // For liveness, we're more lenient - only fail if truly unhealthy
    // Degraded status is still considered "alive"
    let status_code = if status.is_unhealthy() {
        error!("Liveness probe failed: node unhealthy");
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    
    let response = HealthResponse::from(&status);
    
    (status_code, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{HealthCheck, HealthCheckResult};
    use std::time::Duration;
    
    struct TestHealthyCheck;
    impl HealthCheck for TestHealthyCheck {
        fn name(&self) -> &str {
            "test_healthy"
        }
        fn check(&self) -> HealthCheckResult {
            HealthCheckResult::Healthy
        }
    }
    
    struct TestDegradedCheck;
    impl HealthCheck for TestDegradedCheck {
        fn name(&self) -> &str {
            "test_degraded"
        }
        fn check(&self) -> HealthCheckResult {
            HealthCheckResult::Degraded {
                reason: "Test degradation".to_string(),
            }
        }
    }
    
    struct TestUnhealthyCheck;
    impl HealthCheck for TestUnhealthyCheck {
        fn name(&self) -> &str {
            "test_unhealthy"
        }
        fn check(&self) -> HealthCheckResult {
            HealthCheckResult::Unhealthy {
                reason: "Test failure".to_string(),
            }
        }
    }
    
    #[test]
    fn test_result_to_status_string() {
        assert_eq!(
            result_to_status_string(&HealthCheckResult::Healthy),
            "healthy"
        );
        assert_eq!(
            result_to_status_string(&HealthCheckResult::Degraded {
                reason: "test".to_string()
            }),
            "degraded"
        );
        assert_eq!(
            result_to_status_string(&HealthCheckResult::Unhealthy {
                reason: "test".to_string()
            }),
            "unhealthy"
        );
    }
    
    #[test]
    fn test_result_to_status_code() {
        assert_eq!(
            result_to_status_code(&HealthCheckResult::Healthy),
            StatusCode::OK
        );
        assert_eq!(
            result_to_status_code(&HealthCheckResult::Degraded {
                reason: "test".to_string()
            }),
            StatusCode::OK
        );
        assert_eq!(
            result_to_status_code(&HealthCheckResult::Unhealthy {
                reason: "test".to_string()
            }),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    
    #[test]
    fn test_health_response_from_status() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestHealthyCheck));
        checker.add_check(Box::new(TestDegradedCheck));
        
        let status = checker.check_health();
        let response = HealthResponse::from(&status);
        
        assert_eq!(response.status, "degraded");
        assert_eq!(response.checks.len(), 2);
        assert_eq!(response.checks["test_healthy"].status, "healthy");
        assert_eq!(response.checks["test_degraded"].status, "degraded");
        assert_eq!(
            response.checks["test_degraded"].reason,
            Some("Test degradation".to_string())
        );
    }
    
    #[tokio::test]
    async fn test_health_handler_healthy() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestHealthyCheck));
        let checker = Arc::new(checker);
        
        let response = health_handler(State(checker)).await;
        let status = response.status();
        
        assert_eq!(status, StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_health_handler_degraded() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestDegradedCheck));
        let checker = Arc::new(checker);
        
        let response = health_handler(State(checker)).await;
        let status = response.status();
        
        // Degraded still returns 200 OK
        assert_eq!(status, StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_health_handler_unhealthy() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestUnhealthyCheck));
        let checker = Arc::new(checker);
        
        let response = health_handler(State(checker)).await;
        let status = response.status();
        
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[tokio::test]
    async fn test_ready_handler() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestHealthyCheck));
        let checker = Arc::new(checker);
        
        let response = ready_handler(State(checker)).await;
        let status = response.status();
        
        assert_eq!(status, StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_live_handler_degraded_is_alive() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestDegradedCheck));
        let checker = Arc::new(checker);
        
        let response = live_handler(State(checker)).await;
        let status = response.status();
        
        // Degraded is still considered "alive" for liveness probe
        assert_eq!(status, StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_live_handler_unhealthy() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(TestUnhealthyCheck));
        let checker = Arc::new(checker);
        
        let response = live_handler(State(checker)).await;
        let status = response.status();
        
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[test]
    fn test_health_server_config_default() {
        let config = HealthServerConfig::default();
        assert_eq!(config.bind_address.to_string(), "0.0.0.0:8080");
    }
    
    #[test]
    fn test_health_server_creation() {
        let checker = Arc::new(HealthChecker::new(Duration::from_secs(30)));
        let config = HealthServerConfig::default();
        let _server = HealthServer::new(checker, config);
    }
}
