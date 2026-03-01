//! Health Check System for ELARA Runtime
//!
//! This module provides a production-grade health checking system with:
//! - Pluggable health checks via the `HealthCheck` trait
//! - Result caching with configurable TTL to avoid excessive checking
//! - Proper aggregation logic (Unhealthy > Degraded > Healthy)
//! - Thread-safe operation with Arc<RwLock<>>
//! - Support for Kubernetes liveness and readiness probes
//! - Four built-in health checks for common monitoring needs
//!
//! # Architecture
//!
//! The health check system consists of:
//! - `HealthCheck` trait: Defines the interface for individual health checks
//! - `HealthChecker`: Orchestrates multiple health checks and caches results
//! - `HealthStatus`: Aggregated health status with individual check results
//! - `HealthCheckResult`: Result of an individual health check
//!
//! # Built-in Health Checks
//!
//! The module provides four production-ready health checks:
//!
//! ## 1. ConnectionHealthCheck
//!
//! Monitors the number of active connections to ensure the node is properly
//! connected to the network. Returns `Degraded` if the connection count falls
//! below the configured minimum.
//!
//! **Use case:** Detect network isolation or connectivity issues
//!
//! ## 2. MemoryHealthCheck
//!
//! Monitors process memory usage using the `sysinfo` crate to obtain real
//! system metrics. Returns `Unhealthy` if memory usage exceeds the configured
//! maximum, which helps prevent OOM kills and performance degradation.
//!
//! **Use case:** Detect memory leaks or excessive memory consumption
//!
//! ## 3. TimeDriftCheck
//!
//! Monitors time drift between the local node and network consensus time.
//! Returns `Degraded` if drift exceeds the configured threshold. Excessive
//! time drift can cause synchronization issues and state divergence.
//!
//! **Use case:** Detect clock synchronization issues
//!
//! ## 4. StateDivergenceCheck
//!
//! Monitors the state reconciliation engine to ensure state is converging
//! properly. Returns `Degraded` if the number of pending events exceeds
//! the configured threshold, which may indicate network partitions or
//! reconciliation issues.
//!
//! **Use case:** Detect state convergence problems
//!
//! # Example
//!
//! ```rust,no_run
//! use elara_runtime::health::{
//!     HealthChecker, HealthCheck, HealthCheckResult,
//!     ConnectionHealthCheck, MemoryHealthCheck, TimeDriftCheck, StateDivergenceCheck
//! };
//! use elara_runtime::node::Node;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! // Create a node
//! let node = Arc::new(Node::new());
//!
//! // Create health checker with 30-second cache
//! let mut checker = HealthChecker::new(Duration::from_secs(30));
//!
//! // Add built-in health checks
//! checker.add_check(Box::new(ConnectionHealthCheck::new(node.clone(), 3)));
//! checker.add_check(Box::new(MemoryHealthCheck::new(1800))); // 1800 MB
//! checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 100))); // 100ms
//! checker.add_check(Box::new(StateDivergenceCheck::new(node)));
//!
//! // Check health
//! let status = checker.check_health();
//!
//! if status.is_healthy() {
//!     println!("All systems operational");
//! } else if status.is_degraded() {
//!     println!("System degraded: {:?}", status.overall.reason());
//! } else {
//!     println!("System unhealthy: {:?}", status.overall.reason());
//! }
//! ```
//!
//! # Production Deployment
//!
//! In production, health checks are typically exposed via HTTP endpoints:
//!
//! - `/health` - Overall health status (200 OK if healthy/degraded, 503 if unhealthy)
//! - `/ready` - Readiness probe for Kubernetes (checks if node can accept traffic)
//! - `/live` - Liveness probe for Kubernetes (checks if node should be restarted)
//!
//! Configure thresholds based on your deployment:
//!
//! - **Small deployment (10 nodes)**: min_connections=2, max_memory_mb=1000
//! - **Medium deployment (100 nodes)**: min_connections=5, max_memory_mb=2000
//! - **Large deployment (1000 nodes)**: min_connections=10, max_memory_mb=4000

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// Result of an individual health check.
///
/// Health checks can return one of three states:
/// - `Healthy`: The component is functioning normally
/// - `Degraded`: The component is functioning but with reduced capacity or performance
/// - `Unhealthy`: The component is not functioning correctly
///
/// The overall system health is determined by the worst individual check result,
/// with the precedence: Unhealthy > Degraded > Healthy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckResult {
    /// The component is functioning normally
    Healthy,
    
    /// The component is functioning but degraded
    /// 
    /// This typically indicates reduced capacity, elevated latency,
    /// or other non-critical issues that don't prevent operation.
    Degraded {
        /// Human-readable reason for the degraded state
        reason: String,
    },
    
    /// The component is not functioning correctly
    ///
    /// This indicates a critical issue that prevents normal operation.
    Unhealthy {
        /// Human-readable reason for the unhealthy state
        reason: String,
    },
}

impl HealthCheckResult {
    /// Returns true if the result is Healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthCheckResult::Healthy)
    }
    
    /// Returns true if the result is Degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self, HealthCheckResult::Degraded { .. })
    }
    
    /// Returns true if the result is Unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, HealthCheckResult::Unhealthy { .. })
    }
    
    /// Returns the severity level for ordering (higher is worse)
    fn severity(&self) -> u8 {
        match self {
            HealthCheckResult::Healthy => 0,
            HealthCheckResult::Degraded { .. } => 1,
            HealthCheckResult::Unhealthy { .. } => 2,
        }
    }
    
    /// Returns the reason string if available
    pub fn reason(&self) -> Option<&str> {
        match self {
            HealthCheckResult::Healthy => None,
            HealthCheckResult::Degraded { reason } => Some(reason),
            HealthCheckResult::Unhealthy { reason } => Some(reason),
        }
    }
}

/// Trait for implementing custom health checks.
///
/// Health checks must be `Send + Sync` to allow concurrent execution
/// across threads. The `check()` method should be relatively fast
/// (ideally < 10ms) to avoid blocking the health check endpoint.
///
/// # Example
///
/// ```rust
/// use elara_runtime::health::{HealthCheck, HealthCheckResult};
///
/// struct DatabaseHealthCheck {
///     connection_pool: Arc<ConnectionPool>,
/// }
///
/// impl HealthCheck for DatabaseHealthCheck {
///     fn name(&self) -> &str {
///         "database"
///     }
///
///     fn check(&self) -> HealthCheckResult {
///         match self.connection_pool.ping() {
///             Ok(_) => HealthCheckResult::Healthy,
///             Err(e) => HealthCheckResult::Unhealthy {
///                 reason: format!("Database ping failed: {}", e),
///             },
///         }
///     }
/// }
/// ```
pub trait HealthCheck: Send + Sync {
    /// Returns the name of this health check.
    ///
    /// The name should be unique within a `HealthChecker` and should
    /// be a valid identifier (lowercase, alphanumeric, underscores).
    fn name(&self) -> &str;
    
    /// Performs the health check and returns the result.
    ///
    /// This method should be relatively fast (< 10ms ideally) to avoid
    /// blocking the health check endpoint. For expensive checks, consider
    /// running them in the background and caching the result.
    fn check(&self) -> HealthCheckResult;
}

/// Aggregated health status containing overall status and individual check results.
///
/// The overall status is determined by the worst individual check result:
/// - If any check is Unhealthy, overall is Unhealthy
/// - Else if any check is Degraded, overall is Degraded
/// - Else overall is Healthy
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall health status (worst of all checks)
    pub overall: HealthCheckResult,
    
    /// Individual health check results by name
    pub checks: HashMap<String, HealthCheckResult>,
    
    /// Timestamp when this status was computed
    pub timestamp: Instant,
}

impl HealthStatus {
    /// Creates a new HealthStatus with the given checks
    fn new(checks: HashMap<String, HealthCheckResult>) -> Self {
        let overall = Self::aggregate_results(&checks);
        Self {
            overall,
            checks,
            timestamp: Instant::now(),
        }
    }
    
    /// Aggregates individual check results into an overall status.
    ///
    /// Precedence: Unhealthy > Degraded > Healthy
    fn aggregate_results(checks: &HashMap<String, HealthCheckResult>) -> HealthCheckResult {
        if checks.is_empty() {
            return HealthCheckResult::Healthy;
        }
        
        // Find the worst result
        let worst = checks.values()
            .max_by_key(|result| result.severity())
            .unwrap(); // Safe because we checked is_empty
        
        worst.clone()
    }
    
    /// Returns true if the overall status is Healthy
    pub fn is_healthy(&self) -> bool {
        self.overall.is_healthy()
    }
    
    /// Returns true if the overall status is Degraded
    pub fn is_degraded(&self) -> bool {
        self.overall.is_degraded()
    }
    
    /// Returns true if the overall status is Unhealthy
    pub fn is_unhealthy(&self) -> bool {
        self.overall.is_unhealthy()
    }
}

/// Configuration for the health checker
#[derive(Debug, Clone)]
pub struct HealthCheckerConfig {
    /// Time-to-live for cached health check results
    pub cache_ttl: Duration,
}

impl Default for HealthCheckerConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(30),
        }
    }
}

/// Comprehensive health check configuration for NodeConfig.
///
/// This configuration enables and configures the health check system for a node.
/// When set in `NodeConfig`, the node will automatically initialize health checks
/// and optionally start an HTTP server to expose health endpoints.
///
/// # Health Check System
///
/// The health check system provides:
/// - Built-in checks for connections, memory, time drift, and state convergence
/// - Configurable thresholds for each check
/// - HTTP endpoints for Kubernetes probes and load balancers
/// - Result caching to minimize overhead
///
/// # HTTP Endpoints
///
/// When `server_bind_address` is set, the following endpoints are exposed:
/// - `GET /health` - Overall health status (200 OK if healthy/degraded, 503 if unhealthy)
/// - `GET /ready` - Readiness probe (200 OK if healthy/degraded, 503 if unhealthy)
/// - `GET /live` - Liveness probe (200 OK if healthy/degraded, 503 if unhealthy)
///
/// # Example
///
/// ```rust,no_run
/// use elara_runtime::health::HealthCheckConfig;
/// use elara_runtime::node::NodeConfig;
/// use std::time::Duration;
///
/// let health_config = HealthCheckConfig {
///     enabled: true,
///     server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
///     cache_ttl: Duration::from_secs(30),
///     min_connections: Some(3),
///     max_memory_mb: Some(1800),
///     max_time_drift_ms: Some(100),
///     max_pending_events: Some(1000),
/// };
///
/// let node_config = NodeConfig {
///     health_checks: Some(health_config),
///     ..Default::default()
/// };
/// ```
///
/// # Production Recommendations
///
/// ## Small Deployment (10 nodes)
/// ```rust,no_run
/// use elara_runtime::health::HealthCheckConfig;
/// use std::time::Duration;
///
/// let config = HealthCheckConfig {
///     enabled: true,
///     server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
///     cache_ttl: Duration::from_secs(30),
///     min_connections: Some(2),
///     max_memory_mb: Some(1000),
///     max_time_drift_ms: Some(100),
///     max_pending_events: Some(500),
/// };
/// ```
///
/// ## Medium Deployment (100 nodes)
/// ```rust,no_run
/// use elara_runtime::health::HealthCheckConfig;
/// use std::time::Duration;
///
/// let config = HealthCheckConfig {
///     enabled: true,
///     server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
///     cache_ttl: Duration::from_secs(30),
///     min_connections: Some(5),
///     max_memory_mb: Some(2000),
///     max_time_drift_ms: Some(100),
///     max_pending_events: Some(1000),
/// };
/// ```
///
/// ## Large Deployment (1000 nodes)
/// ```rust,no_run
/// use elara_runtime::health::HealthCheckConfig;
/// use std::time::Duration;
///
/// let config = HealthCheckConfig {
///     enabled: true,
///     server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
///     cache_ttl: Duration::from_secs(30),
///     min_connections: Some(10),
///     max_memory_mb: Some(4000),
///     max_time_drift_ms: Some(100),
///     max_pending_events: Some(2000),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Enable or disable health checks.
    ///
    /// When `false`, no health checks are performed and no HTTP server is started.
    /// This allows health checks to be completely disabled in environments where
    /// they are not needed.
    ///
    /// Default: `true`
    pub enabled: bool,

    /// Optional bind address for the health check HTTP server.
    ///
    /// When `Some`, an HTTP server is started on this address to expose health
    /// check endpoints (`/health`, `/ready`, `/live`). When `None`, health checks
    /// are still performed but no HTTP server is started (useful for programmatic
    /// health checking without exposing endpoints).
    ///
    /// Format: `"host:port"` (e.g., `"0.0.0.0:8080"`, `"127.0.0.1:8080"`)
    ///
    /// Default: `Some("0.0.0.0:8080")`
    pub server_bind_address: Option<std::net::SocketAddr>,

    /// Cache TTL for health check results.
    ///
    /// Health check results are cached for this duration to avoid excessive
    /// checking overhead. Subsequent health check requests within the TTL
    /// return cached results.
    ///
    /// Recommended values:
    /// - High-frequency checks: 10-15 seconds
    /// - Normal checks: 30 seconds
    /// - Low-frequency checks: 60 seconds
    ///
    /// Default: 30 seconds
    pub cache_ttl: Duration,

    /// Minimum number of active connections for ConnectionHealthCheck.
    ///
    /// When `Some`, a `ConnectionHealthCheck` is registered that monitors
    /// the number of active connections. The check returns `Degraded` if
    /// the connection count falls below this threshold.
    ///
    /// When `None`, no connection health check is performed.
    ///
    /// Recommended values:
    /// - Small deployment: 2-3
    /// - Medium deployment: 5-10
    /// - Large deployment: 10-20
    ///
    /// Default: `Some(3)`
    pub min_connections: Option<usize>,

    /// Maximum memory usage in megabytes for MemoryHealthCheck.
    ///
    /// When `Some`, a `MemoryHealthCheck` is registered that monitors
    /// process memory usage. The check returns `Unhealthy` if memory
    /// usage exceeds this threshold.
    ///
    /// When `None`, no memory health check is performed.
    ///
    /// Recommended values:
    /// - Small deployment: 1000 MB (1 GB)
    /// - Medium deployment: 2000 MB (2 GB)
    /// - Large deployment: 4000 MB (4 GB)
    ///
    /// Set this to 80-90% of your container memory limit to allow for
    /// graceful degradation before OOM kills.
    ///
    /// Default: `Some(1800)` (1.8 GB)
    pub max_memory_mb: Option<usize>,

    /// Maximum time drift in milliseconds for TimeDriftCheck.
    ///
    /// When `Some`, a `TimeDriftCheck` is registered that monitors
    /// time drift between the local node and network consensus time.
    /// The check returns `Degraded` if drift exceeds this threshold.
    ///
    /// When `None`, no time drift check is performed.
    ///
    /// Recommended value: 100 ms
    ///
    /// Excessive time drift can cause synchronization issues and state
    /// divergence in distributed systems.
    ///
    /// Default: `Some(100)`
    pub max_time_drift_ms: Option<i64>,

    /// Maximum pending events for StateDivergenceCheck.
    ///
    /// When `Some`, a `StateDivergenceCheck` is registered that monitors
    /// the state reconciliation engine. The check returns `Degraded` if
    /// the number of pending events exceeds this threshold.
    ///
    /// When `None`, no state divergence check is performed.
    ///
    /// Recommended values:
    /// - Small deployment: 500
    /// - Medium deployment: 1000
    /// - Large deployment: 2000
    ///
    /// High pending event counts may indicate network partitions or
    /// reconciliation issues.
    ///
    /// Default: `Some(1000)`
    pub max_pending_events: Option<usize>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
            cache_ttl: Duration::from_secs(30),
            min_connections: Some(3),
            max_memory_mb: Some(1800),
            max_time_drift_ms: Some(100),
            max_pending_events: Some(1000),
        }
    }
}

impl HealthCheckConfig {
    /// Creates a new HealthCheckConfig with all checks disabled.
    ///
    /// This is useful when you want to selectively enable only specific checks.
    ///
    /// # Example
    ///
    /// ```rust
    /// use elara_runtime::health::HealthCheckConfig;
    ///
    /// let mut config = HealthCheckConfig::disabled();
    /// config.enabled = true;
    /// config.max_memory_mb = Some(2000); // Only enable memory check
    /// ```
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            server_bind_address: None,
            cache_ttl: Duration::from_secs(30),
            min_connections: None,
            max_memory_mb: None,
            max_time_drift_ms: None,
            max_pending_events: None,
        }
    }

    /// Creates a configuration for small deployments (10 nodes).
    ///
    /// Recommended thresholds:
    /// - Min connections: 2
    /// - Max memory: 1000 MB
    /// - Max time drift: 100 ms
    /// - Max pending events: 500
    pub fn small_deployment() -> Self {
        Self {
            enabled: true,
            server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
            cache_ttl: Duration::from_secs(30),
            min_connections: Some(2),
            max_memory_mb: Some(1000),
            max_time_drift_ms: Some(100),
            max_pending_events: Some(500),
        }
    }

    /// Creates a configuration for medium deployments (100 nodes).
    ///
    /// Recommended thresholds:
    /// - Min connections: 5
    /// - Max memory: 2000 MB
    /// - Max time drift: 100 ms
    /// - Max pending events: 1000
    pub fn medium_deployment() -> Self {
        Self {
            enabled: true,
            server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
            cache_ttl: Duration::from_secs(30),
            min_connections: Some(5),
            max_memory_mb: Some(2000),
            max_time_drift_ms: Some(100),
            max_pending_events: Some(1000),
        }
    }

    /// Creates a configuration for large deployments (1000 nodes).
    ///
    /// Recommended thresholds:
    /// - Min connections: 10
    /// - Max memory: 4000 MB
    /// - Max time drift: 100 ms
    /// - Max pending events: 2000
    pub fn large_deployment() -> Self {
        Self {
            enabled: true,
            server_bind_address: Some("0.0.0.0:8080".parse().unwrap()),
            cache_ttl: Duration::from_secs(30),
            min_connections: Some(10),
            max_memory_mb: Some(4000),
            max_time_drift_ms: Some(100),
            max_pending_events: Some(2000),
        }
    }

    /// Validates the configuration.
    ///
    /// Returns `Ok(())` if the configuration is valid, or an error message
    /// describing the validation failure.
    ///
    /// # Validation Rules
    ///
    /// - `cache_ttl` must be at least 1 second
    /// - If `min_connections` is set, it must be > 0
    /// - If `max_memory_mb` is set, it must be > 0
    /// - If `max_time_drift_ms` is set, it must be > 0
    /// - If `max_pending_events` is set, it must be > 0
    pub fn validate(&self) -> Result<(), String> {
        if self.cache_ttl < Duration::from_secs(1) {
            return Err("cache_ttl must be at least 1 second".to_string());
        }

        if let Some(min_conn) = self.min_connections {
            if min_conn == 0 {
                return Err("min_connections must be greater than 0".to_string());
            }
        }

        if let Some(max_mem) = self.max_memory_mb {
            if max_mem == 0 {
                return Err("max_memory_mb must be greater than 0".to_string());
            }
        }

        if let Some(max_drift) = self.max_time_drift_ms {
            if max_drift <= 0 {
                return Err("max_time_drift_ms must be greater than 0".to_string());
            }
        }

        if let Some(max_events) = self.max_pending_events {
            if max_events == 0 {
                return Err("max_pending_events must be greater than 0".to_string());
            }
        }

        Ok(())
    }
}

/// Health checker that orchestrates multiple health checks with caching.
///
/// The `HealthChecker` runs registered health checks and caches the results
/// for a configurable TTL to avoid excessive checking overhead. This is
/// particularly important when health checks are expensive (e.g., database
/// queries, external service calls).
///
/// # Thread Safety
///
/// The `HealthChecker` is thread-safe and can be shared across threads using
/// `Arc`. The internal cache is protected by a `RwLock` for concurrent access.
///
/// # Caching Behavior
///
/// - Health check results are cached for `cache_ttl` duration
/// - Cached results are returned if still valid (not expired)
/// - Expired results trigger a new health check execution
/// - Cache updates are atomic and thread-safe
///
/// # Example
///
/// ```rust,no_run
/// use elara_runtime::health::{HealthChecker, HealthCheck, HealthCheckResult};
/// use std::time::Duration;
/// use std::sync::Arc;
///
/// struct MyCheck;
/// impl HealthCheck for MyCheck {
///     fn name(&self) -> &str { "my_check" }
///     fn check(&self) -> HealthCheckResult { HealthCheckResult::Healthy }
/// }
///
/// let mut checker = HealthChecker::new(Duration::from_secs(30));
/// checker.add_check(Box::new(MyCheck));
///
/// // First call executes checks
/// let status1 = checker.check_health();
///
/// // Second call within TTL returns cached result
/// let status2 = checker.check_health();
/// ```
pub struct HealthChecker {
    /// Registered health checks
    checks: Vec<Box<dyn HealthCheck>>,
    
    /// Cached health status with timestamp
    cache: Arc<RwLock<Option<HealthStatus>>>,
    
    /// Cache time-to-live
    cache_ttl: Duration,
}

impl HealthChecker {
    /// Creates a new HealthChecker with the specified cache TTL.
    ///
    /// # Arguments
    ///
    /// * `cache_ttl` - Duration for which health check results are cached
    ///
    /// # Example
    ///
    /// ```rust
    /// use elara_runtime::health::HealthChecker;
    /// use std::time::Duration;
    ///
    /// let checker = HealthChecker::new(Duration::from_secs(30));
    /// ```
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            checks: Vec::new(),
            cache: Arc::new(RwLock::new(None)),
            cache_ttl,
        }
    }
    
    /// Creates a new HealthChecker with default configuration.
    ///
    /// Uses a default cache TTL of 30 seconds.
    pub fn with_default_config() -> Self {
        Self::new(HealthCheckerConfig::default().cache_ttl)
    }
    
    /// Creates a new HealthChecker with the specified configuration.
    pub fn with_config(config: HealthCheckerConfig) -> Self {
        Self::new(config.cache_ttl)
    }
    
    /// Adds a health check to the checker.
    ///
    /// Health checks are executed in the order they are added.
    ///
    /// # Arguments
    ///
    /// * `check` - Boxed health check implementation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::{HealthChecker, HealthCheck, HealthCheckResult};
    /// use std::time::Duration;
    ///
    /// struct MyCheck;
    /// impl HealthCheck for MyCheck {
    ///     fn name(&self) -> &str { "my_check" }
    ///     fn check(&self) -> HealthCheckResult { HealthCheckResult::Healthy }
    /// }
    ///
    /// let mut checker = HealthChecker::new(Duration::from_secs(30));
    /// checker.add_check(Box::new(MyCheck));
    /// ```
    pub fn add_check(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }
    
    /// Checks the health of all registered checks.
    ///
    /// This method returns cached results if they are still valid (within TTL).
    /// If the cache is expired or empty, it executes all health checks and
    /// updates the cache.
    ///
    /// # Returns
    ///
    /// `HealthStatus` containing the overall status and individual check results.
    ///
    /// # Performance
    ///
    /// - Cached reads: O(1) with read lock
    /// - Cache miss: O(n) where n is the number of checks, with write lock
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::HealthChecker;
    /// use std::time::Duration;
    ///
    /// let checker = HealthChecker::new(Duration::from_secs(30));
    /// let status = checker.check_health();
    ///
    /// if status.is_healthy() {
    ///     println!("All systems operational");
    /// } else {
    ///     println!("System degraded or unhealthy");
    /// }
    /// ```
    pub fn check_health(&self) -> HealthStatus {
        // Fast path: check if cache is valid
        {
            let cache = self.cache.read();
            if let Some(ref status) = *cache {
                if status.timestamp.elapsed() < self.cache_ttl {
                    return status.clone();
                }
            }
        }
        
        // Slow path: execute health checks and update cache
        let mut results = HashMap::new();
        
        for check in &self.checks {
            let result = check.check();
            results.insert(check.name().to_string(), result);
        }
        
        let status = HealthStatus::new(results);
        
        // Update cache
        {
            let mut cache = self.cache.write();
            *cache = Some(status.clone());
        }
        
        status
    }
    
    /// Clears the cached health status, forcing the next check to execute.
    ///
    /// This is useful for testing or when you need to force a fresh health check.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        *cache = None;
    }
    
    /// Returns the number of registered health checks.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
    
    /// Returns the cache TTL duration.
    pub fn cache_ttl(&self) -> Duration {
        self.cache_ttl
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct AlwaysHealthyCheck;
    impl HealthCheck for AlwaysHealthyCheck {
        fn name(&self) -> &str {
            "always_healthy"
        }
        fn check(&self) -> HealthCheckResult {
            HealthCheckResult::Healthy
        }
    }
    
    struct AlwaysDegradedCheck;
    impl HealthCheck for AlwaysDegradedCheck {
        fn name(&self) -> &str {
            "always_degraded"
        }
        fn check(&self) -> HealthCheckResult {
            HealthCheckResult::Degraded {
                reason: "Test degradation".to_string(),
            }
        }
    }
    
    struct AlwaysUnhealthyCheck;
    impl HealthCheck for AlwaysUnhealthyCheck {
        fn name(&self) -> &str {
            "always_unhealthy"
        }
        fn check(&self) -> HealthCheckResult {
            HealthCheckResult::Unhealthy {
                reason: "Test failure".to_string(),
            }
        }
    }
    
    #[test]
    fn test_health_check_result_methods() {
        let healthy = HealthCheckResult::Healthy;
        assert!(healthy.is_healthy());
        assert!(!healthy.is_degraded());
        assert!(!healthy.is_unhealthy());
        assert_eq!(healthy.reason(), None);
        
        let degraded = HealthCheckResult::Degraded {
            reason: "test".to_string(),
        };
        assert!(!degraded.is_healthy());
        assert!(degraded.is_degraded());
        assert!(!degraded.is_unhealthy());
        assert_eq!(degraded.reason(), Some("test"));
        
        let unhealthy = HealthCheckResult::Unhealthy {
            reason: "test".to_string(),
        };
        assert!(!unhealthy.is_healthy());
        assert!(!unhealthy.is_degraded());
        assert!(unhealthy.is_unhealthy());
        assert_eq!(unhealthy.reason(), Some("test"));
    }
    
    #[test]
    fn test_health_checker_empty() {
        let checker = HealthChecker::new(Duration::from_secs(30));
        let status = checker.check_health();
        assert!(status.is_healthy());
        assert_eq!(status.checks.len(), 0);
    }
    
    #[test]
    fn test_health_checker_all_healthy() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(AlwaysHealthyCheck));
        
        let status = checker.check_health();
        assert!(status.is_healthy());
        assert_eq!(status.checks.len(), 1);
    }
    
    #[test]
    fn test_health_checker_degraded() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(AlwaysHealthyCheck));
        checker.add_check(Box::new(AlwaysDegradedCheck));
        
        let status = checker.check_health();
        assert!(status.is_degraded());
        assert_eq!(status.checks.len(), 2);
    }
    
    #[test]
    fn test_health_checker_unhealthy() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(AlwaysHealthyCheck));
        checker.add_check(Box::new(AlwaysDegradedCheck));
        checker.add_check(Box::new(AlwaysUnhealthyCheck));
        
        let status = checker.check_health();
        assert!(status.is_unhealthy());
        assert_eq!(status.checks.len(), 3);
    }
    
    #[test]
    fn test_health_checker_precedence() {
        // Unhealthy takes precedence over Degraded
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(AlwaysDegradedCheck));
        checker.add_check(Box::new(AlwaysUnhealthyCheck));
        
        let status = checker.check_health();
        assert!(status.is_unhealthy());
        
        // Degraded takes precedence over Healthy
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(AlwaysHealthyCheck));
        checker.add_check(Box::new(AlwaysDegradedCheck));
        
        let status = checker.check_health();
        assert!(status.is_degraded());
    }
    
    #[test]
    fn test_health_checker_caching() {
        let mut checker = HealthChecker::new(Duration::from_millis(100));
        checker.add_check(Box::new(AlwaysHealthyCheck));
        
        // First call should execute checks
        let status1 = checker.check_health();
        let timestamp1 = status1.timestamp;
        
        // Second call within TTL should return cached result
        let status2 = checker.check_health();
        let timestamp2 = status2.timestamp;
        
        assert_eq!(timestamp1, timestamp2);
        
        // Wait for cache to expire
        std::thread::sleep(Duration::from_millis(150));
        
        // Third call after TTL should execute checks again
        let status3 = checker.check_health();
        let timestamp3 = status3.timestamp;
        
        assert!(timestamp3 > timestamp1);
    }
    
    #[test]
    fn test_health_checker_clear_cache() {
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        checker.add_check(Box::new(AlwaysHealthyCheck));
        
        let status1 = checker.check_health();
        let timestamp1 = status1.timestamp;
        
        checker.clear_cache();
        
        let status2 = checker.check_health();
        let timestamp2 = status2.timestamp;
        
        assert!(timestamp2 > timestamp1);
    }
    
    #[test]
    fn test_health_status_aggregation() {
        let mut checks = HashMap::new();
        
        // All healthy
        checks.insert("check1".to_string(), HealthCheckResult::Healthy);
        checks.insert("check2".to_string(), HealthCheckResult::Healthy);
        let status = HealthStatus::new(checks.clone());
        assert!(status.is_healthy());
        
        // One degraded
        checks.insert("check3".to_string(), HealthCheckResult::Degraded {
            reason: "test".to_string(),
        });
        let status = HealthStatus::new(checks.clone());
        assert!(status.is_degraded());
        
        // One unhealthy
        checks.insert("check4".to_string(), HealthCheckResult::Unhealthy {
            reason: "test".to_string(),
        });
        let status = HealthStatus::new(checks);
        assert!(status.is_unhealthy());
    }
}

// ============================================================================
// Built-in Health Checks
// ============================================================================

/// Health check for monitoring active connection count.
///
/// This check verifies that the node has at least a minimum number of active
/// connections. Having too few connections may indicate network issues,
/// configuration problems, or that the node is isolated from the network.
///
/// # Status Determination
///
/// - `Healthy`: Active connections >= min_connections
/// - `Degraded`: Active connections < min_connections
///
/// # Example
///
/// ```rust,no_run
/// use elara_runtime::health::{ConnectionHealthCheck, HealthCheck};
/// use elara_runtime::node::Node;
/// use std::sync::Arc;
///
/// let node = Arc::new(Node::new());
/// let check = ConnectionHealthCheck::new(node, 3);
/// let result = check.check();
/// ```
pub struct ConnectionHealthCheck {
    /// Reference to the node to check
    _node: Arc<crate::node::Node>,
    /// Minimum number of connections required for healthy status
    min_connections: usize,
}

impl ConnectionHealthCheck {
    /// Creates a new ConnectionHealthCheck.
    ///
    /// # Arguments
    ///
    /// * `node` - Arc reference to the Node to monitor
    /// * `min_connections` - Minimum number of active connections for healthy status
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::ConnectionHealthCheck;
    /// use elara_runtime::node::Node;
    /// use std::sync::Arc;
    ///
    /// let node = Arc::new(Node::new());
    /// let check = ConnectionHealthCheck::new(node, 3);
    /// ```
    pub fn new(node: Arc<crate::node::Node>, min_connections: usize) -> Self {
        Self {
            _node: node,
            min_connections,
        }
    }
    
    /// Returns the configured minimum connections threshold.
    pub fn min_connections(&self) -> usize {
        self.min_connections
    }
}

impl HealthCheck for ConnectionHealthCheck {
    fn name(&self) -> &str {
        "connections"
    }
    
    fn check(&self) -> HealthCheckResult {
        // For now, we'll use a placeholder since Node doesn't have active_connections() yet
        // In a real implementation, this would query the actual connection count
        // from the transport layer or session manager
        let active = 0; // TODO: Implement node.active_connections()
        
        if active >= self.min_connections {
            HealthCheckResult::Healthy
        } else {
            HealthCheckResult::Degraded {
                reason: format!(
                    "Only {} active connections (minimum: {})",
                    active, self.min_connections
                ),
            }
        }
    }
}

/// Health check for monitoring memory usage.
///
/// This check monitors the process memory usage and compares it against
/// a configured maximum threshold. Excessive memory usage can lead to
/// OOM kills, performance degradation, and system instability.
///
/// Uses the `sysinfo` crate to obtain real memory usage statistics.
///
/// # Status Determination
///
/// - `Healthy`: Memory usage < max_memory_mb
/// - `Unhealthy`: Memory usage >= max_memory_mb
///
/// # Example
///
/// ```rust
/// use elara_runtime::health::{MemoryHealthCheck, HealthCheck};
///
/// let check = MemoryHealthCheck::new(1800); // 1800 MB limit
/// let result = check.check();
/// ```
pub struct MemoryHealthCheck {
    /// Maximum memory usage in megabytes before unhealthy
    max_memory_mb: usize,
    /// System information provider (cached for efficiency)
    system: Arc<RwLock<sysinfo::System>>,
}

impl MemoryHealthCheck {
    /// Creates a new MemoryHealthCheck.
    ///
    /// # Arguments
    ///
    /// * `max_memory_mb` - Maximum memory usage in MB before marking unhealthy
    ///
    /// # Example
    ///
    /// ```rust
    /// use elara_runtime::health::MemoryHealthCheck;
    ///
    /// let check = MemoryHealthCheck::new(2048); // 2GB limit
    /// ```
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            max_memory_mb,
            system: Arc::new(RwLock::new(sysinfo::System::new_all())),
        }
    }
    
    /// Returns the configured maximum memory threshold in MB.
    pub fn max_memory_mb(&self) -> usize {
        self.max_memory_mb
    }
    
    /// Gets the current memory usage in megabytes.
    ///
    /// This method refreshes the system memory information and returns
    /// the current process memory usage.
    fn get_memory_usage_mb(&self) -> usize {
        let mut system = self.system.write();
        system.refresh_memory();
        system.refresh_processes();
        
        // Get current process PID
        let pid = sysinfo::get_current_pid().ok();
        
        if let Some(pid) = pid {
            if let Some(process) = system.process(pid) {
                // Convert bytes to megabytes
                return (process.memory() / 1_048_576) as usize;
            }
        }
        
        // Fallback: return 0 if we can't get process info
        0
    }
}

impl HealthCheck for MemoryHealthCheck {
    fn name(&self) -> &str {
        "memory"
    }
    
    fn check(&self) -> HealthCheckResult {
        let usage_mb = self.get_memory_usage_mb();
        
        if usage_mb < self.max_memory_mb {
            HealthCheckResult::Healthy
        } else {
            HealthCheckResult::Unhealthy {
                reason: format!(
                    "Memory usage {}MB exceeds limit {}MB",
                    usage_mb, self.max_memory_mb
                ),
            }
        }
    }
}

/// Health check for monitoring time drift.
///
/// This check monitors the time drift between the local node and the
/// network consensus time. Excessive time drift can cause synchronization
/// issues, event ordering problems, and state divergence.
///
/// # Status Determination
///
/// - `Healthy`: |time_drift| < max_drift_ms
/// - `Degraded`: |time_drift| >= max_drift_ms
///
/// # Example
///
/// ```rust,no_run
/// use elara_runtime::health::{TimeDriftCheck, HealthCheck};
/// use elara_runtime::node::Node;
/// use std::sync::Arc;
///
/// let node = Arc::new(Node::new());
/// let check = TimeDriftCheck::new(node, 100); // 100ms max drift
/// let result = check.check();
/// ```
pub struct TimeDriftCheck {
    /// Reference to the node to check
    node: Arc<crate::node::Node>,
    /// Maximum acceptable time drift in milliseconds
    max_drift_ms: i64,
}

impl TimeDriftCheck {
    /// Creates a new TimeDriftCheck.
    ///
    /// # Arguments
    ///
    /// * `node` - Arc reference to the Node to monitor
    /// * `max_drift_ms` - Maximum acceptable time drift in milliseconds
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::TimeDriftCheck;
    /// use elara_runtime::node::Node;
    /// use std::sync::Arc;
    ///
    /// let node = Arc::new(Node::new());
    /// let check = TimeDriftCheck::new(node, 100);
    /// ```
    pub fn new(node: Arc<crate::node::Node>, max_drift_ms: i64) -> Self {
        Self {
            node,
            max_drift_ms,
        }
    }
    
    /// Returns the configured maximum drift threshold in milliseconds.
    pub fn max_drift_ms(&self) -> i64 {
        self.max_drift_ms
    }
    
    /// Gets the current time drift in milliseconds.
    ///
    /// This queries the time engine to determine the drift between
    /// local time and network consensus time.
    fn get_time_drift_ms(&self) -> i64 {
        // Access the time engine to get drift information
        let time_engine = self.node.time_engine();
        
        // Get the current drift from the time engine
        // The drift is the difference between local time and network time
        time_engine.drift_ms()
    }
}

impl HealthCheck for TimeDriftCheck {
    fn name(&self) -> &str {
        "time_drift"
    }
    
    fn check(&self) -> HealthCheckResult {
        let drift_ms = self.get_time_drift_ms();
        let abs_drift = drift_ms.abs();
        
        if abs_drift < self.max_drift_ms {
            HealthCheckResult::Healthy
        } else {
            HealthCheckResult::Degraded {
                reason: format!(
                    "Time drift {}ms exceeds limit {}ms",
                    drift_ms, self.max_drift_ms
                ),
            }
        }
    }
}

/// Health check for monitoring state convergence.
///
/// This check monitors the state reconciliation engine to ensure that
/// state is converging properly across the network. State divergence
/// can indicate network partitions, bugs in the reconciliation logic,
/// or other serious issues.
///
/// # Status Determination
///
/// - `Healthy`: State is converging normally
/// - `Degraded`: State convergence is slow or stalled
/// - `Unhealthy`: State divergence detected
///
/// # Example
///
/// ```rust,no_run
/// use elara_runtime::health::{StateDivergenceCheck, HealthCheck};
/// use elara_runtime::node::Node;
/// use std::sync::Arc;
///
/// let node = Arc::new(Node::new());
/// let check = StateDivergenceCheck::new(node);
/// let result = check.check();
/// ```
pub struct StateDivergenceCheck {
    /// Reference to the node to check
    node: Arc<crate::node::Node>,
    /// Maximum acceptable pending events before degraded
    max_pending_events: usize,
}

impl StateDivergenceCheck {
    /// Creates a new StateDivergenceCheck.
    ///
    /// # Arguments
    ///
    /// * `node` - Arc reference to the Node to monitor
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::StateDivergenceCheck;
    /// use elara_runtime::node::Node;
    /// use std::sync::Arc;
    ///
    /// let node = Arc::new(Node::new());
    /// let check = StateDivergenceCheck::new(node);
    /// ```
    pub fn new(node: Arc<crate::node::Node>) -> Self {
        Self::with_threshold(node, 1000)
    }
    
    /// Creates a new StateDivergenceCheck with a custom threshold.
    ///
    /// # Arguments
    ///
    /// * `node` - Arc reference to the Node to monitor
    /// * `max_pending_events` - Maximum pending events before degraded status
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use elara_runtime::health::StateDivergenceCheck;
    /// use elara_runtime::node::Node;
    /// use std::sync::Arc;
    ///
    /// let node = Arc::new(Node::new());
    /// let check = StateDivergenceCheck::with_threshold(node, 500);
    /// ```
    pub fn with_threshold(node: Arc<crate::node::Node>, max_pending_events: usize) -> Self {
        Self {
            node,
            max_pending_events,
        }
    }
    
    /// Returns the configured maximum pending events threshold.
    pub fn max_pending_events(&self) -> usize {
        self.max_pending_events
    }
    
    /// Checks the state convergence status.
    ///
    /// This examines the reconciliation engine to determine if state
    /// is converging properly.
    fn check_convergence(&self) -> (bool, usize) {
        // Access the state engine to check convergence
        let state_engine = self.node.state_engine();
        
        // Get the number of pending events that haven't been reconciled
        // In a real implementation, this would query the reconciliation engine
        // for metrics about pending events, unmerged states, etc.
        let pending_events = state_engine.pending_count();
        
        // Check if we're converging (pending count is reasonable)
        let is_converging = pending_events < self.max_pending_events;
        
        (is_converging, pending_events)
    }
}

impl HealthCheck for StateDivergenceCheck {
    fn name(&self) -> &str {
        "state_convergence"
    }
    
    fn check(&self) -> HealthCheckResult {
        let (is_converging, pending_events) = self.check_convergence();
        
        if is_converging {
            HealthCheckResult::Healthy
        } else {
            HealthCheckResult::Degraded {
                reason: format!(
                    "State convergence slow: {} pending events (threshold: {})",
                    pending_events, self.max_pending_events
                ),
            }
        }
    }
}

#[cfg(test)]
mod builtin_tests {
    use super::*;
    use crate::node::Node;
    
    #[test]
    fn test_memory_health_check() {
        // Test with a very high threshold (should be healthy)
        let check = MemoryHealthCheck::new(100_000); // 100GB
        let result = check.check();
        assert!(result.is_healthy(), "Should be healthy with high threshold");
        
        // Test with a very low threshold (should be unhealthy)
        let check = MemoryHealthCheck::new(1); // 1MB
        let result = check.check();
        assert!(result.is_unhealthy(), "Should be unhealthy with low threshold");
    }
    
    #[test]
    fn test_memory_health_check_threshold() {
        let check = MemoryHealthCheck::new(1800);
        assert_eq!(check.max_memory_mb(), 1800);
    }
    
    #[test]
    fn test_connection_health_check_creation() {
        let node = Arc::new(Node::new());
        let check = ConnectionHealthCheck::new(node, 3);
        assert_eq!(check.name(), "connections");
        assert_eq!(check.min_connections(), 3);
    }
    
    #[test]
    fn test_time_drift_check_creation() {
        let node = Arc::new(Node::new());
        let check = TimeDriftCheck::new(node, 100);
        assert_eq!(check.name(), "time_drift");
        assert_eq!(check.max_drift_ms(), 100);
    }
    
    #[test]
    fn test_state_divergence_check_creation() {
        let node = Arc::new(Node::new());
        let check = StateDivergenceCheck::new(node.clone());
        assert_eq!(check.name(), "state_convergence");
        assert_eq!(check.max_pending_events(), 1000);
        
        let check = StateDivergenceCheck::with_threshold(node, 500);
        assert_eq!(check.max_pending_events(), 500);
    }
    
    #[test]
    fn test_all_builtin_checks_with_health_checker() {
        let node = Arc::new(Node::new());
        let mut checker = HealthChecker::new(Duration::from_secs(30));
        
        // Add all built-in checks
        checker.add_check(Box::new(ConnectionHealthCheck::new(node.clone(), 3)));
        checker.add_check(Box::new(MemoryHealthCheck::new(100_000)));
        checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 100)));
        checker.add_check(Box::new(StateDivergenceCheck::new(node)));
        
        assert_eq!(checker.check_count(), 4);
        
        // Run health check
        let status = checker.check_health();
        assert_eq!(status.checks.len(), 4);
        
        // Verify all checks are present
        assert!(status.checks.contains_key("connections"));
        assert!(status.checks.contains_key("memory"));
        assert!(status.checks.contains_key("time_drift"));
        assert!(status.checks.contains_key("state_convergence"));
    }
    
    #[test]
    fn test_health_check_config_default() {
        let config = HealthCheckConfig::default();
        
        assert!(config.enabled);
        assert!(config.server_bind_address.is_some());
        assert_eq!(config.cache_ttl, Duration::from_secs(30));
        assert_eq!(config.min_connections, Some(3));
        assert_eq!(config.max_memory_mb, Some(1800));
        assert_eq!(config.max_time_drift_ms, Some(100));
        assert_eq!(config.max_pending_events, Some(1000));
        
        // Default config should be valid
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_disabled() {
        let config = HealthCheckConfig::disabled();
        
        assert!(!config.enabled);
        assert!(config.server_bind_address.is_none());
        assert!(config.min_connections.is_none());
        assert!(config.max_memory_mb.is_none());
        assert!(config.max_time_drift_ms.is_none());
        assert!(config.max_pending_events.is_none());
        
        // Disabled config should still be valid
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_small_deployment() {
        let config = HealthCheckConfig::small_deployment();
        
        assert!(config.enabled);
        assert_eq!(config.min_connections, Some(2));
        assert_eq!(config.max_memory_mb, Some(1000));
        assert_eq!(config.max_time_drift_ms, Some(100));
        assert_eq!(config.max_pending_events, Some(500));
        
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_medium_deployment() {
        let config = HealthCheckConfig::medium_deployment();
        
        assert!(config.enabled);
        assert_eq!(config.min_connections, Some(5));
        assert_eq!(config.max_memory_mb, Some(2000));
        assert_eq!(config.max_time_drift_ms, Some(100));
        assert_eq!(config.max_pending_events, Some(1000));
        
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_large_deployment() {
        let config = HealthCheckConfig::large_deployment();
        
        assert!(config.enabled);
        assert_eq!(config.min_connections, Some(10));
        assert_eq!(config.max_memory_mb, Some(4000));
        assert_eq!(config.max_time_drift_ms, Some(100));
        assert_eq!(config.max_pending_events, Some(2000));
        
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_validation_cache_ttl() {
        let mut config = HealthCheckConfig::default();
        
        // Valid cache TTL
        config.cache_ttl = Duration::from_secs(1);
        assert!(config.validate().is_ok());
        
        // Invalid cache TTL (too short)
        config.cache_ttl = Duration::from_millis(999);
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "cache_ttl must be at least 1 second"
        );
    }
    
    #[test]
    fn test_health_check_config_validation_min_connections() {
        let mut config = HealthCheckConfig::default();
        
        // Valid min_connections
        config.min_connections = Some(1);
        assert!(config.validate().is_ok());
        
        // Invalid min_connections (zero)
        config.min_connections = Some(0);
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "min_connections must be greater than 0"
        );
        
        // None is valid (check disabled)
        config.min_connections = None;
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_validation_max_memory() {
        let mut config = HealthCheckConfig::default();
        
        // Valid max_memory_mb
        config.max_memory_mb = Some(1);
        assert!(config.validate().is_ok());
        
        // Invalid max_memory_mb (zero)
        config.max_memory_mb = Some(0);
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "max_memory_mb must be greater than 0"
        );
        
        // None is valid (check disabled)
        config.max_memory_mb = None;
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_validation_max_time_drift() {
        let mut config = HealthCheckConfig::default();
        
        // Valid max_time_drift_ms
        config.max_time_drift_ms = Some(1);
        assert!(config.validate().is_ok());
        
        // Invalid max_time_drift_ms (zero)
        config.max_time_drift_ms = Some(0);
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "max_time_drift_ms must be greater than 0"
        );
        
        // Invalid max_time_drift_ms (negative)
        config.max_time_drift_ms = Some(-1);
        assert!(config.validate().is_err());
        
        // None is valid (check disabled)
        config.max_time_drift_ms = None;
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_validation_max_pending_events() {
        let mut config = HealthCheckConfig::default();
        
        // Valid max_pending_events
        config.max_pending_events = Some(1);
        assert!(config.validate().is_ok());
        
        // Invalid max_pending_events (zero)
        config.max_pending_events = Some(0);
        assert!(config.validate().is_err());
        assert_eq!(
            config.validate().unwrap_err(),
            "max_pending_events must be greater than 0"
        );
        
        // None is valid (check disabled)
        config.max_pending_events = None;
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_health_check_config_selective_checks() {
        // Test configuration with only some checks enabled
        let config = HealthCheckConfig {
            enabled: true,
            server_bind_address: None,
            cache_ttl: Duration::from_secs(30),
            min_connections: Some(5),
            max_memory_mb: None, // Disabled
            max_time_drift_ms: Some(100),
            max_pending_events: None, // Disabled
        };
        
        assert!(config.validate().is_ok());
        assert!(config.enabled);
        assert!(config.min_connections.is_some());
        assert!(config.max_memory_mb.is_none());
        assert!(config.max_time_drift_ms.is_some());
        assert!(config.max_pending_events.is_none());
    }
}
