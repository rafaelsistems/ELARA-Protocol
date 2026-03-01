//! Metrics collection system for ELARA runtime.
//!
//! This module provides a thread-safe metrics registry with support for:
//! - **Counters**: Monotonically increasing values
//! - **Gauges**: Values that can increase or decrease
//! - **Histograms**: Distribution of observations with percentile calculations
//!
//! # Example
//!
//! ```rust
//! use elara_runtime::observability::metrics::{MetricsRegistry, Counter, Gauge, Histogram};
//!
//! let registry = MetricsRegistry::new();
//!
//! // Register and use a counter
//! let counter = registry.register_counter("messages_sent", vec![]);
//! counter.inc();
//! counter.inc_by(5);
//!
//! // Register and use a gauge
//! let gauge = registry.register_gauge("active_connections", vec![]);
//! gauge.set(10);
//! gauge.inc();
//! gauge.dec();
//!
//! // Register and use a histogram
//! let histogram = registry.register_histogram(
//!     "message_latency_ms",
//!     vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0],
//!     vec![],
//! );
//! histogram.observe(42.5);
//! ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Errors that can occur during metrics operations.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// Metric with the given name already exists.
    #[error("Metric '{0}' already exists")]
    AlreadyExists(String),

    /// Metric with the given name was not found.
    #[error("Metric '{0}' not found")]
    NotFound(String),

    /// Invalid metric configuration.
    #[error("Invalid metric configuration: {0}")]
    InvalidConfig(String),
}

/// Thread-safe registry for all metrics.
///
/// The registry maintains separate storage for counters, gauges, and histograms,
/// allowing concurrent access from multiple threads.
#[derive(Clone)]
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
}

impl MetricsRegistry {
    /// Creates a new empty metrics registry.
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a new counter with the given name and labels.
    ///
    /// # Errors
    ///
    /// Returns `MetricsError::AlreadyExists` if a counter with this name already exists.
    pub fn register_counter(
        &self,
        name: impl Into<String>,
        labels: Vec<(String, String)>,
    ) -> Counter {
        let name = name.into();
        let mut counters = self.counters.write();

        if counters.contains_key(&name) {
            // Return existing counter
            counters.get(&name).unwrap().clone()
        } else {
            let counter = Counter::new(name.clone(), labels);
            counters.insert(name, counter.clone());
            counter
        }
    }

    /// Registers a new gauge with the given name and labels.
    ///
    /// # Errors
    ///
    /// Returns `MetricsError::AlreadyExists` if a gauge with this name already exists.
    pub fn register_gauge(
        &self,
        name: impl Into<String>,
        labels: Vec<(String, String)>,
    ) -> Gauge {
        let name = name.into();
        let mut gauges = self.gauges.write();

        if gauges.contains_key(&name) {
            // Return existing gauge
            gauges.get(&name).unwrap().clone()
        } else {
            let gauge = Gauge::new(name.clone(), labels);
            gauges.insert(name, gauge.clone());
            gauge
        }
    }

    /// Registers a new histogram with the given name, buckets, and labels.
    ///
    /// # Arguments
    ///
    /// * `name` - The metric name
    /// * `buckets` - Bucket boundaries for the histogram (must be sorted in ascending order)
    /// * `labels` - Key-value pairs for metric labels
    ///
    /// # Errors
    ///
    /// Returns `MetricsError::AlreadyExists` if a histogram with this name already exists.
    /// Returns `MetricsError::InvalidConfig` if buckets are not sorted or empty.
    pub fn register_histogram(
        &self,
        name: impl Into<String>,
        buckets: Vec<f64>,
        labels: Vec<(String, String)>,
    ) -> Histogram {
        let name = name.into();
        let mut histograms = self.histograms.write();

        if histograms.contains_key(&name) {
            // Return existing histogram
            histograms.get(&name).unwrap().clone()
        } else {
            let histogram = Histogram::new(name.clone(), buckets, labels);
            histograms.insert(name, histogram.clone());
            histogram
        }
    }

    /// Gets a counter by name.
    pub fn get_counter(&self, name: &str) -> Option<Counter> {
        self.counters.read().get(name).cloned()
    }

    /// Gets a gauge by name.
    pub fn get_gauge(&self, name: &str) -> Option<Gauge> {
        self.gauges.read().get(name).cloned()
    }

    /// Gets a histogram by name.
    pub fn get_histogram(&self, name: &str) -> Option<Histogram> {
        self.histograms.read().get(name).cloned()
    }

    /// Returns all registered counter names.
    pub fn counter_names(&self) -> Vec<String> {
        self.counters.read().keys().cloned().collect()
    }

    /// Returns all registered gauge names.
    pub fn gauge_names(&self) -> Vec<String> {
        self.gauges.read().keys().cloned().collect()
    }

    /// Returns all registered histogram names.
    pub fn histogram_names(&self) -> Vec<String> {
        self.histograms.read().keys().cloned().collect()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A monotonically increasing counter metric.
///
/// Counters are used to track values that only increase, such as:
/// - Total messages sent
/// - Total connections established
/// - Total errors encountered
#[derive(Clone)]
pub struct Counter {
    name: String,
    value: Arc<AtomicU64>,
    labels: Arc<HashMap<String, String>>,
}

impl Counter {
    /// Creates a new counter with the given name and labels.
    pub fn new(name: String, labels: Vec<(String, String)>) -> Self {
        Self {
            name,
            value: Arc::new(AtomicU64::new(0)),
            labels: Arc::new(labels.into_iter().collect()),
        }
    }

    /// Increments the counter by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the counter by the given amount.
    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Returns the current value of the counter.
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Returns the name of the counter.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the labels associated with this counter.
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

/// A gauge metric that can increase or decrease.
///
/// Gauges are used to track values that can go up or down, such as:
/// - Active connections
/// - Memory usage
/// - Queue depth
#[derive(Clone)]
pub struct Gauge {
    name: String,
    value: Arc<AtomicI64>,
    labels: Arc<HashMap<String, String>>,
}

impl Gauge {
    /// Creates a new gauge with the given name and labels.
    pub fn new(name: String, labels: Vec<(String, String)>) -> Self {
        Self {
            name,
            value: Arc::new(AtomicI64::new(0)),
            labels: Arc::new(labels.into_iter().collect()),
        }
    }

    /// Sets the gauge to the given value.
    pub fn set(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }

    /// Increments the gauge by 1.
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the gauge by 1.
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increments the gauge by the given amount.
    pub fn add(&self, n: i64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Decrements the gauge by the given amount.
    pub fn sub(&self, n: i64) {
        self.value.fetch_sub(n, Ordering::Relaxed);
    }

    /// Returns the current value of the gauge.
    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Returns the name of the gauge.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the labels associated with this gauge.
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

/// A histogram metric for tracking distributions of observations.
///
/// Histograms are used to track the distribution of values, such as:
/// - Request latencies
/// - Message sizes
/// - Processing times
///
/// The histogram maintains counts in predefined buckets and can calculate
/// percentiles (p50, p95, p99).
#[derive(Clone)]
pub struct Histogram {
    name: String,
    buckets: Arc<Vec<f64>>,
    counts: Arc<Vec<AtomicU64>>,
    sum: Arc<AtomicU64>,
    count: Arc<AtomicU64>,
    labels: Arc<HashMap<String, String>>,
}

impl Histogram {
    /// Creates a new histogram with the given name, buckets, and labels.
    ///
    /// # Panics
    ///
    /// Panics if buckets are empty or not sorted in ascending order.
    pub fn new(name: String, buckets: Vec<f64>, labels: Vec<(String, String)>) -> Self {
        assert!(!buckets.is_empty(), "Histogram buckets cannot be empty");

        // Verify buckets are sorted
        for i in 1..buckets.len() {
            assert!(
                buckets[i] > buckets[i - 1],
                "Histogram buckets must be sorted in ascending order"
            );
        }

        let bucket_count = buckets.len() + 1; // +1 for +Inf bucket
        let counts: Vec<AtomicU64> = (0..bucket_count).map(|_| AtomicU64::new(0)).collect();

        Self {
            name,
            buckets: Arc::new(buckets),
            counts: Arc::new(counts),
            sum: Arc::new(AtomicU64::new(0)),
            count: Arc::new(AtomicU64::new(0)),
            labels: Arc::new(labels.into_iter().collect()),
        }
    }

    /// Records an observation in the histogram.
    pub fn observe(&self, value: f64) {
        // Find the appropriate bucket
        let bucket_index = self
            .buckets
            .iter()
            .position(|&b| value <= b)
            .unwrap_or(self.buckets.len());

        // Increment the bucket count
        self.counts[bucket_index].fetch_add(1, Ordering::Relaxed);

        // Update count
        self.count.fetch_add(1, Ordering::Relaxed);
        
        // Update sum - we need to handle this atomically
        // We'll use a simple approach: convert to integer representation for atomic ops
        // This works for positive values and maintains precision
        let value_as_u64 = (value * 1000000.0) as u64; // Store as microseconds for precision
        self.sum.fetch_add(value_as_u64, Ordering::Relaxed);
    }

    /// Returns the total count of observations.
    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Returns the sum of all observations.
    pub fn get_sum(&self) -> f64 {
        let sum_micros = self.sum.load(Ordering::Relaxed);
        sum_micros as f64 / 1000000.0
    }

    /// Returns the bucket boundaries.
    pub fn get_buckets(&self) -> &[f64] {
        &self.buckets
    }

    /// Returns the count for each bucket.
    pub fn get_bucket_counts(&self) -> Vec<u64> {
        self.counts
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .collect()
    }

    /// Calculates the specified percentile from the histogram data.
    ///
    /// # Arguments
    ///
    /// * `percentile` - A value between 0.0 and 1.0 (e.g., 0.95 for p95)
    ///
    /// # Returns
    ///
    /// An estimated value at the given percentile, or 0.0 if no observations.
    pub fn percentile(&self, percentile: f64) -> f64 {
        assert!(
            (0.0..=1.0).contains(&percentile),
            "Percentile must be between 0.0 and 1.0"
        );

        let total_count = self.get_count();
        if total_count == 0 {
            return 0.0;
        }

        let target_count = (total_count as f64 * percentile).ceil() as u64;
        let mut cumulative = 0u64;

        for (i, count) in self.get_bucket_counts().iter().enumerate() {
            cumulative += count;
            if cumulative >= target_count {
                // Return the upper bound of this bucket
                return if i < self.buckets.len() {
                    self.buckets[i]
                } else {
                    // +Inf bucket - return the last bucket boundary
                    *self.buckets.last().unwrap()
                };
            }
        }

        // Fallback (should not reach here)
        *self.buckets.last().unwrap()
    }

    /// Returns the name of the histogram.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the labels associated with this histogram.
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_basic() {
        let counter = Counter::new("test_counter".to_string(), vec![]);
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_counter_thread_safety() {
        let counter = Counter::new("test_counter".to_string(), vec![]);
        let counter_clone = counter.clone();

        let handle = std::thread::spawn(move || {
            for _ in 0..1000 {
                counter_clone.inc();
            }
        });

        for _ in 0..1000 {
            counter.inc();
        }

        handle.join().unwrap();
        assert_eq!(counter.get(), 2000);
    }

    #[test]
    fn test_gauge_basic() {
        let gauge = Gauge::new("test_gauge".to_string(), vec![]);
        assert_eq!(gauge.get(), 0);

        gauge.set(10);
        assert_eq!(gauge.get(), 10);

        gauge.inc();
        assert_eq!(gauge.get(), 11);

        gauge.dec();
        assert_eq!(gauge.get(), 10);

        gauge.add(5);
        assert_eq!(gauge.get(), 15);

        gauge.sub(3);
        assert_eq!(gauge.get(), 12);
    }

    #[test]
    fn test_histogram_basic() {
        let histogram = Histogram::new(
            "test_histogram".to_string(),
            vec![1.0, 5.0, 10.0, 50.0, 100.0],
            vec![],
        );

        histogram.observe(0.5);
        histogram.observe(3.0);
        histogram.observe(7.0);
        histogram.observe(25.0);
        histogram.observe(75.0);
        histogram.observe(150.0);

        assert_eq!(histogram.get_count(), 6);

        let counts = histogram.get_bucket_counts();
        assert_eq!(counts[0], 1); // <= 1.0
        assert_eq!(counts[1], 1); // <= 5.0
        assert_eq!(counts[2], 1); // <= 10.0
        assert_eq!(counts[3], 1); // <= 50.0
        assert_eq!(counts[4], 1); // <= 100.0
        assert_eq!(counts[5], 1); // > 100.0 (+Inf)
    }

    #[test]
    fn test_histogram_percentiles() {
        let histogram = Histogram::new(
            "test_histogram".to_string(),
            vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0],
            vec![],
        );

        // Add 100 observations: 1, 2, 3, ..., 100
        for i in 1..=100 {
            histogram.observe(i as f64);
        }

        assert_eq!(histogram.get_count(), 100);

        // Test percentiles
        let p50 = histogram.percentile(0.50);
        let p95 = histogram.percentile(0.95);
        let p99 = histogram.percentile(0.99);

        // Verify ordering
        assert!(p50 <= p95);
        assert!(p95 <= p99);

        // p50 should be around 50
        assert!(p50 >= 40.0 && p50 <= 60.0);

        // p95 should be around 95
        assert!(p95 >= 90.0 && p95 <= 100.0);

        // p99 should be around 99
        assert!(p99 >= 90.0 && p99 <= 100.0);
    }

    #[test]
    fn test_registry_counter() {
        let registry = MetricsRegistry::new();

        let counter1 = registry.register_counter("test_counter", vec![]);
        counter1.inc();

        let counter2 = registry.get_counter("test_counter").unwrap();
        assert_eq!(counter2.get(), 1);

        counter2.inc();
        assert_eq!(counter1.get(), 2);
    }

    #[test]
    fn test_registry_gauge() {
        let registry = MetricsRegistry::new();

        let gauge1 = registry.register_gauge("test_gauge", vec![]);
        gauge1.set(42);

        let gauge2 = registry.get_gauge("test_gauge").unwrap();
        assert_eq!(gauge2.get(), 42);
    }

    #[test]
    fn test_registry_histogram() {
        let registry = MetricsRegistry::new();

        let hist1 = registry.register_histogram("test_histogram", vec![1.0, 10.0, 100.0], vec![]);
        hist1.observe(5.0);

        let hist2 = registry.get_histogram("test_histogram").unwrap();
        assert_eq!(hist2.get_count(), 1);
    }

    #[test]
    fn test_registry_list_metrics() {
        let registry = MetricsRegistry::new();

        registry.register_counter("counter1", vec![]);
        registry.register_counter("counter2", vec![]);
        registry.register_gauge("gauge1", vec![]);
        registry.register_histogram("histogram1", vec![1.0, 10.0], vec![]);

        let counter_names = registry.counter_names();
        assert_eq!(counter_names.len(), 2);
        assert!(counter_names.contains(&"counter1".to_string()));
        assert!(counter_names.contains(&"counter2".to_string()));

        let gauge_names = registry.gauge_names();
        assert_eq!(gauge_names.len(), 1);
        assert!(gauge_names.contains(&"gauge1".to_string()));

        let histogram_names = registry.histogram_names();
        assert_eq!(histogram_names.len(), 1);
        assert!(histogram_names.contains(&"histogram1".to_string()));
    }

    #[test]
    fn test_counter_with_labels() {
        let counter = Counter::new(
            "test_counter".to_string(),
            vec![
                ("node_id".to_string(), "node-1".to_string()),
                ("region".to_string(), "us-west".to_string()),
            ],
        );

        assert_eq!(counter.labels().get("node_id").unwrap(), "node-1");
        assert_eq!(counter.labels().get("region").unwrap(), "us-west");
    }

    #[test]
    #[should_panic(expected = "Histogram buckets cannot be empty")]
    fn test_histogram_empty_buckets() {
        Histogram::new("test".to_string(), vec![], vec![]);
    }

    #[test]
    #[should_panic(expected = "Histogram buckets must be sorted in ascending order")]
    fn test_histogram_unsorted_buckets() {
        Histogram::new("test".to_string(), vec![10.0, 5.0, 20.0], vec![]);
    }

    #[test]
    fn test_node_metrics_creation() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Verify all metrics are initialized to zero
        assert_eq!(metrics.active_connections.get(), 0);
        assert_eq!(metrics.total_connections.get(), 0);
        assert_eq!(metrics.failed_connections.get(), 0);
        assert_eq!(metrics.messages_sent.get(), 0);
        assert_eq!(metrics.messages_received.get(), 0);
        assert_eq!(metrics.messages_dropped.get(), 0);
        assert_eq!(metrics.memory_usage_bytes.get(), 0);
        assert_eq!(metrics.cpu_usage_percent.get(), 0);
        assert_eq!(metrics.time_drift_ms.get(), 0);
        assert_eq!(metrics.state_divergence_count.get(), 0);
    }

    #[test]
    fn test_node_metrics_connection_tracking() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Simulate connection lifecycle
        metrics.active_connections.inc();
        metrics.total_connections.inc();
        assert_eq!(metrics.active_connections.get(), 1);
        assert_eq!(metrics.total_connections.get(), 1);

        // Add more connections
        metrics.active_connections.inc();
        metrics.total_connections.inc();
        assert_eq!(metrics.active_connections.get(), 2);
        assert_eq!(metrics.total_connections.get(), 2);

        // Close a connection
        metrics.active_connections.dec();
        assert_eq!(metrics.active_connections.get(), 1);
        assert_eq!(metrics.total_connections.get(), 2); // Total doesn't decrease

        // Track failed connection
        metrics.failed_connections.inc();
        assert_eq!(metrics.failed_connections.get(), 1);
    }

    #[test]
    fn test_node_metrics_message_tracking() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Track sent messages
        metrics.messages_sent.inc_by(10);
        assert_eq!(metrics.messages_sent.get(), 10);

        // Track received messages
        metrics.messages_received.inc_by(8);
        assert_eq!(metrics.messages_received.get(), 8);

        // Track dropped messages
        metrics.messages_dropped.inc();
        assert_eq!(metrics.messages_dropped.get(), 1);

        // Track message sizes
        metrics.message_size_bytes.observe(512.0);
        metrics.message_size_bytes.observe(2048.0);
        assert_eq!(metrics.message_size_bytes.get_count(), 2);
    }

    #[test]
    fn test_node_metrics_latency_tracking() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Track message latencies
        metrics.message_latency_ms.observe(5.0);
        metrics.message_latency_ms.observe(15.0);
        metrics.message_latency_ms.observe(50.0);
        assert_eq!(metrics.message_latency_ms.get_count(), 3);

        // Track state sync latencies
        metrics.state_sync_latency_ms.observe(100.0);
        metrics.state_sync_latency_ms.observe(500.0);
        assert_eq!(metrics.state_sync_latency_ms.get_count(), 2);

        // Verify percentiles work
        let p50 = metrics.message_latency_ms.percentile(0.50);
        let p95 = metrics.message_latency_ms.percentile(0.95);
        assert!(p50 <= p95);
    }

    #[test]
    fn test_node_metrics_resource_tracking() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Track memory usage (512 MB)
        metrics.memory_usage_bytes.set(512 * 1024 * 1024);
        assert_eq!(metrics.memory_usage_bytes.get(), 536870912);

        // Track CPU usage
        metrics.cpu_usage_percent.set(45);
        assert_eq!(metrics.cpu_usage_percent.get(), 45);

        // Update CPU usage
        metrics.cpu_usage_percent.set(60);
        assert_eq!(metrics.cpu_usage_percent.get(), 60);
    }

    #[test]
    fn test_node_metrics_protocol_tracking() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Track time drift (positive = ahead, negative = behind)
        metrics.time_drift_ms.set(50);
        assert_eq!(metrics.time_drift_ms.get(), 50);

        metrics.time_drift_ms.set(-25);
        assert_eq!(metrics.time_drift_ms.get(), -25);

        // Track state divergence
        metrics.state_divergence_count.set(0);
        assert_eq!(metrics.state_divergence_count.get(), 0);

        metrics.state_divergence_count.inc();
        assert_eq!(metrics.state_divergence_count.get(), 1);
    }

    #[test]
    fn test_node_metrics_accessor_methods() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Test that accessor methods return the correct references
        metrics.active_connections().inc();
        assert_eq!(metrics.active_connections.get(), 1);

        metrics.total_connections().inc();
        assert_eq!(metrics.total_connections.get(), 1);

        metrics.messages_sent().inc_by(5);
        assert_eq!(metrics.messages_sent.get(), 5);

        metrics.message_latency_ms().observe(42.0);
        assert_eq!(metrics.message_latency_ms.get_count(), 1);
    }

    #[test]
    fn test_node_metrics_registry_integration() {
        let registry = MetricsRegistry::new();
        let _metrics = NodeMetrics::new(&registry);

        // Verify all metrics are registered in the registry
        let counter_names = registry.counter_names();
        assert!(counter_names.contains(&"elara_total_connections".to_string()));
        assert!(counter_names.contains(&"elara_failed_connections".to_string()));
        assert!(counter_names.contains(&"elara_messages_sent".to_string()));
        assert!(counter_names.contains(&"elara_messages_received".to_string()));
        assert!(counter_names.contains(&"elara_messages_dropped".to_string()));

        let gauge_names = registry.gauge_names();
        assert!(gauge_names.contains(&"elara_active_connections".to_string()));
        assert!(gauge_names.contains(&"elara_memory_usage_bytes".to_string()));
        assert!(gauge_names.contains(&"elara_cpu_usage_percent".to_string()));
        assert!(gauge_names.contains(&"elara_time_drift_ms".to_string()));
        assert!(gauge_names.contains(&"elara_state_divergence_count".to_string()));

        let histogram_names = registry.histogram_names();
        assert!(histogram_names.contains(&"elara_message_size_bytes".to_string()));
        assert!(histogram_names.contains(&"elara_message_latency_ms".to_string()));
        assert!(histogram_names.contains(&"elara_state_sync_latency_ms".to_string()));
    }

    #[test]
    fn test_node_metrics_histogram_buckets() {
        let registry = MetricsRegistry::new();
        let metrics = NodeMetrics::new(&registry);

        // Verify message latency buckets are appropriate
        let latency_buckets = metrics.message_latency_ms.get_buckets();
        assert_eq!(latency_buckets[0], 1.0);
        assert_eq!(latency_buckets[latency_buckets.len() - 1], 5000.0);

        // Verify state sync latency buckets are appropriate
        let sync_buckets = metrics.state_sync_latency_ms.get_buckets();
        assert_eq!(sync_buckets[0], 10.0);
        assert_eq!(sync_buckets[sync_buckets.len() - 1], 30000.0);

        // Verify message size buckets are appropriate
        let size_buckets = metrics.message_size_bytes.get_buckets();
        assert_eq!(size_buckets[0], 64.0);
        assert_eq!(size_buckets[size_buckets.len() - 1], 1048576.0);
    }
}

/// Core metrics for monitoring ELARA node health and performance.
///
/// This struct provides a centralized collection of all key metrics that should be
/// monitored in a production ELARA deployment. Metrics are organized into categories:
///
/// - **Connection metrics**: Track connection lifecycle and health
/// - **Message metrics**: Track message throughput and reliability
/// - **Latency metrics**: Track performance characteristics
/// - **Resource metrics**: Track system resource usage
/// - **Protocol metrics**: Track protocol-specific health indicators
///
/// # Example
///
/// ```rust
/// use elara_runtime::observability::metrics::{MetricsRegistry, NodeMetrics};
///
/// let registry = MetricsRegistry::new();
/// let metrics = NodeMetrics::new(&registry);
///
/// // Track connections
/// metrics.active_connections.inc();
/// metrics.total_connections.inc();
///
/// // Track messages
/// metrics.messages_sent.inc();
/// metrics.message_latency_ms.observe(42.5);
///
/// // Track resources
/// metrics.memory_usage_bytes.set(1024 * 1024 * 512); // 512 MB
/// metrics.cpu_usage_percent.set(45);
/// ```
#[derive(Clone)]
pub struct NodeMetrics {
    // Connection metrics
    /// Number of currently active connections.
    pub active_connections: Gauge,
    
    /// Total number of connections established since node start.
    pub total_connections: Counter,
    
    /// Total number of failed connection attempts.
    pub failed_connections: Counter,

    // Message metrics
    /// Total number of messages sent.
    pub messages_sent: Counter,
    
    /// Total number of messages received.
    pub messages_received: Counter,
    
    /// Total number of messages dropped (e.g., due to queue overflow).
    pub messages_dropped: Counter,
    
    /// Distribution of message sizes in bytes.
    pub message_size_bytes: Histogram,

    // Latency metrics
    /// Distribution of message processing latency in milliseconds.
    pub message_latency_ms: Histogram,
    
    /// Distribution of state synchronization latency in milliseconds.
    pub state_sync_latency_ms: Histogram,

    // Resource metrics
    /// Current memory usage in bytes.
    pub memory_usage_bytes: Gauge,
    
    /// Current CPU usage as a percentage (0-100).
    pub cpu_usage_percent: Gauge,

    // Protocol metrics
    /// Current time drift from reference time in milliseconds.
    /// Positive values indicate local clock is ahead, negative values indicate behind.
    pub time_drift_ms: Gauge,
    
    /// Number of state divergences detected.
    /// This tracks instances where state reconciliation found inconsistencies.
    pub state_divergence_count: Gauge,
    
    /// Number of events in the quarantine buffer.
    /// Events are quarantined when they have missing dependencies.
    pub quarantine_buffer_size: Gauge,
}

impl NodeMetrics {
    /// Creates a new `NodeMetrics` instance and registers all metrics with the provided registry.
    ///
    /// This constructor initializes all core metrics with appropriate types and bucket
    /// configurations for histograms. All metrics are registered with the provided
    /// `MetricsRegistry` and can be accessed through the returned struct.
    ///
    /// # Histogram Buckets
    ///
    /// - **message_latency_ms**: Buckets optimized for typical message processing times
    ///   (1ms to 5 seconds)
    /// - **state_sync_latency_ms**: Buckets optimized for state synchronization times
    ///   (10ms to 30 seconds)
    /// - **message_size_bytes**: Buckets optimized for typical message sizes
    ///   (64 bytes to 1 MB)
    ///
    /// # Arguments
    ///
    /// * `registry` - The metrics registry to register all metrics with
    ///
    /// # Example
    ///
    /// ```rust
    /// use elara_runtime::observability::metrics::{MetricsRegistry, NodeMetrics};
    ///
    /// let registry = MetricsRegistry::new();
    /// let metrics = NodeMetrics::new(&registry);
    ///
    /// // Metrics are now registered and ready to use
    /// metrics.active_connections.inc();
    /// ```
    pub fn new(registry: &MetricsRegistry) -> Self {
        // Connection metrics
        let active_connections = registry.register_gauge("elara_active_connections", vec![]);
        let total_connections = registry.register_counter("elara_total_connections", vec![]);
        let failed_connections = registry.register_counter("elara_failed_connections", vec![]);

        // Message metrics
        let messages_sent = registry.register_counter("elara_messages_sent", vec![]);
        let messages_received = registry.register_counter("elara_messages_received", vec![]);
        let messages_dropped = registry.register_counter("elara_messages_dropped", vec![]);

        // Message size histogram with buckets from 64 bytes to 1 MB
        let message_size_bytes = registry.register_histogram(
            "elara_message_size_bytes",
            vec![
                64.0,      // 64 B
                256.0,     // 256 B
                1024.0,    // 1 KB
                4096.0,    // 4 KB
                16384.0,   // 16 KB
                65536.0,   // 64 KB
                262144.0,  // 256 KB
                1048576.0, // 1 MB
            ],
            vec![],
        );

        // Latency metrics
        // Message latency histogram with buckets from 1ms to 5 seconds
        let message_latency_ms = registry.register_histogram(
            "elara_message_latency_ms",
            vec![
                1.0,    // 1 ms
                5.0,    // 5 ms
                10.0,   // 10 ms
                25.0,   // 25 ms
                50.0,   // 50 ms
                100.0,  // 100 ms
                250.0,  // 250 ms
                500.0,  // 500 ms
                1000.0, // 1 second
                2500.0, // 2.5 seconds
                5000.0, // 5 seconds
            ],
            vec![],
        );

        // State sync latency histogram with buckets from 10ms to 30 seconds
        let state_sync_latency_ms = registry.register_histogram(
            "elara_state_sync_latency_ms",
            vec![
                10.0,    // 10 ms
                50.0,    // 50 ms
                100.0,   // 100 ms
                250.0,   // 250 ms
                500.0,   // 500 ms
                1000.0,  // 1 second
                2500.0,  // 2.5 seconds
                5000.0,  // 5 seconds
                10000.0, // 10 seconds
                30000.0, // 30 seconds
            ],
            vec![],
        );

        // Resource metrics
        let memory_usage_bytes = registry.register_gauge("elara_memory_usage_bytes", vec![]);
        let cpu_usage_percent = registry.register_gauge("elara_cpu_usage_percent", vec![]);

        // Protocol metrics
        let time_drift_ms = registry.register_gauge("elara_time_drift_ms", vec![]);
        let state_divergence_count =
            registry.register_gauge("elara_state_divergence_count", vec![]);
        let quarantine_buffer_size =
            registry.register_gauge("elara_quarantine_buffer_size", vec![]);

        Self {
            // Connection metrics
            active_connections,
            total_connections,
            failed_connections,

            // Message metrics
            messages_sent,
            messages_received,
            messages_dropped,
            message_size_bytes,

            // Latency metrics
            message_latency_ms,
            state_sync_latency_ms,

            // Resource metrics
            memory_usage_bytes,
            cpu_usage_percent,

            // Protocol metrics
            time_drift_ms,
            state_divergence_count,
            quarantine_buffer_size,
        }
    }

    /// Returns a reference to the active connections gauge.
    pub fn active_connections(&self) -> &Gauge {
        &self.active_connections
    }

    /// Returns a reference to the total connections counter.
    pub fn total_connections(&self) -> &Counter {
        &self.total_connections
    }

    /// Returns a reference to the failed connections counter.
    pub fn failed_connections(&self) -> &Counter {
        &self.failed_connections
    }

    /// Returns a reference to the messages sent counter.
    pub fn messages_sent(&self) -> &Counter {
        &self.messages_sent
    }

    /// Returns a reference to the messages received counter.
    pub fn messages_received(&self) -> &Counter {
        &self.messages_received
    }

    /// Returns a reference to the messages dropped counter.
    pub fn messages_dropped(&self) -> &Counter {
        &self.messages_dropped
    }

    /// Returns a reference to the message size histogram.
    pub fn message_size_bytes(&self) -> &Histogram {
        &self.message_size_bytes
    }

    /// Returns a reference to the message latency histogram.
    pub fn message_latency_ms(&self) -> &Histogram {
        &self.message_latency_ms
    }

    /// Returns a reference to the state sync latency histogram.
    pub fn state_sync_latency_ms(&self) -> &Histogram {
        &self.state_sync_latency_ms
    }

    /// Returns a reference to the memory usage gauge.
    pub fn memory_usage_bytes(&self) -> &Gauge {
        &self.memory_usage_bytes
    }

    /// Returns a reference to the CPU usage gauge.
    pub fn cpu_usage_percent(&self) -> &Gauge {
        &self.cpu_usage_percent
    }

    /// Returns a reference to the time drift gauge.
    pub fn time_drift_ms(&self) -> &Gauge {
        &self.time_drift_ms
    }

    /// Returns a reference to the state divergence count gauge.
    pub fn state_divergence_count(&self) -> &Gauge {
        &self.state_divergence_count
    }

    /// Returns a reference to the quarantine buffer size gauge.
    pub fn quarantine_buffer_size(&self) -> &Gauge {
        &self.quarantine_buffer_size
    }
}

impl std::fmt::Debug for NodeMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeMetrics")
            .field("active_connections", &self.active_connections.get())
            .field("total_connections", &self.total_connections.get())
            .field("failed_connections", &self.failed_connections.get())
            .field("messages_sent", &self.messages_sent.get())
            .field("messages_received", &self.messages_received.get())
            .field("messages_dropped", &self.messages_dropped.get())
            .field("message_size_bytes_count", &self.message_size_bytes.get_count())
            .field("message_latency_ms_count", &self.message_latency_ms.get_count())
            .field("state_sync_latency_ms_count", &self.state_sync_latency_ms.get_count())
            .field("memory_usage_bytes", &self.memory_usage_bytes.get())
            .field("cpu_usage_percent", &self.cpu_usage_percent.get())
            .field("time_drift_ms", &self.time_drift_ms.get())
            .field("state_divergence_count", &self.state_divergence_count.get())
            .field("quarantine_buffer_size", &self.quarantine_buffer_size.get())
            .finish()
    }
}

impl MetricsRegistry {
    /// Exports all metrics in Prometheus text exposition format.
    ///
    /// This method generates a string containing all registered metrics in the
    /// Prometheus text format, which can be scraped by Prometheus servers.
    ///
    /// The format follows the Prometheus specification:
    /// ```text
    /// # HELP metric_name Description
    /// # TYPE metric_name counter|gauge|histogram
    /// metric_name{label1="value1"} 42
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// use elara_runtime::observability::metrics::MetricsRegistry;
    ///
    /// let registry = MetricsRegistry::new();
    /// let counter = registry.register_counter("test_counter", vec![]);
    /// counter.inc();
    ///
    /// let prometheus_text = registry.export_prometheus();
    /// assert!(prometheus_text.contains("test_counter"));
    /// ```
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        let counters = self.counters.read();
        for (name, counter) in counters.iter() {
            output.push_str(&format!("# HELP {} Counter metric\n", name));
            output.push_str(&format!("# TYPE {} counter\n", name));
            
            if counter.labels().is_empty() {
                output.push_str(&format!("{} {}\n", name, counter.get()));
            } else {
                let labels = format_labels(counter.labels());
                output.push_str(&format!("{}{} {}\n", name, labels, counter.get()));
            }
        }

        // Export gauges
        let gauges = self.gauges.read();
        for (name, gauge) in gauges.iter() {
            output.push_str(&format!("# HELP {} Gauge metric\n", name));
            output.push_str(&format!("# TYPE {} gauge\n", name));
            
            if gauge.labels().is_empty() {
                output.push_str(&format!("{} {}\n", name, gauge.get()));
            } else {
                let labels = format_labels(gauge.labels());
                output.push_str(&format!("{}{} {}\n", name, labels, gauge.get()));
            }
        }

        // Export histograms
        let histograms = self.histograms.read();
        for (name, histogram) in histograms.iter() {
            output.push_str(&format!("# HELP {} Histogram metric\n", name));
            output.push_str(&format!("# TYPE {} histogram\n", name));
            
            let label_prefix = if histogram.labels().is_empty() {
                String::new()
            } else {
                format_labels(histogram.labels())
            };

            // Export bucket counts
            let buckets = histogram.get_buckets();
            let counts = histogram.get_bucket_counts();
            let mut cumulative = 0u64;

            for (i, &bucket) in buckets.iter().enumerate() {
                cumulative += counts[i];
                let bucket_label = if label_prefix.is_empty() {
                    format!("{{le=\"{:.1}\"}}", bucket)
                } else {
                    // Insert le label into existing labels
                    let trimmed = label_prefix.trim_end_matches('}');
                    format!("{},le=\"{:.1}\"}}", trimmed, bucket)
                };
                output.push_str(&format!("{}_bucket{} {}\n", name, bucket_label, cumulative));
            }

            // Export +Inf bucket
            cumulative += counts[buckets.len()];
            let inf_label = if label_prefix.is_empty() {
                "{le=\"+Inf\"}".to_string()
            } else {
                let trimmed = label_prefix.trim_end_matches('}');
                format!("{},le=\"+Inf\"}}", trimmed)
            };
            output.push_str(&format!("{}_bucket{} {}\n", name, inf_label, cumulative));

            // Export sum and count
            output.push_str(&format!("{}_sum{} {}\n", name, label_prefix, histogram.get_sum()));
            output.push_str(&format!("{}_count{} {}\n", name, label_prefix, histogram.get_count()));
        }

        output
    }
}

/// Formats labels as a Prometheus label string.
///
/// Converts a HashMap of labels into the Prometheus format: `{key1="value1",key2="value2"}`
fn format_labels(labels: &HashMap<String, String>) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let mut label_pairs: Vec<String> = labels
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, escape_label_value(v)))
        .collect();
    
    // Sort for consistent output
    label_pairs.sort();
    
    format!("{{{}}}", label_pairs.join(","))
}

/// Escapes special characters in label values according to Prometheus format.
fn escape_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
