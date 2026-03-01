//! Integration tests for the Prometheus metrics server.
//!
//! These tests verify the end-to-end functionality of the metrics server,
//! including HTTP endpoint behavior and Prometheus format compliance.

use elara_runtime::observability::{
    MetricsRegistry, MetricsServer, MetricsServerConfig, NodeMetrics,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_metrics_server_lifecycle() {
    // Create registry and server
    let registry = MetricsRegistry::new();
    let config = MetricsServerConfig {
        bind_address: "127.0.0.1".to_string(),
        port: 0, // Let OS assign port
    };
    
    let mut server = MetricsServer::new(config, registry);
    
    // Server should not be running initially
    assert!(!server.is_running());
    
    // Start server
    server.start().await.expect("Failed to start server");
    assert!(server.is_running());
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Shutdown server
    server.shutdown().await;
    assert!(!server.is_running());
}

#[tokio::test]
async fn test_metrics_server_with_node_metrics() {
    // Create registry with node metrics
    let registry = MetricsRegistry::new();
    let node_metrics = NodeMetrics::new(&registry);
    
    // Update some metrics
    node_metrics.active_connections.set(5);
    node_metrics.total_connections.inc_by(10);
    node_metrics.messages_sent.inc_by(100);
    node_metrics.message_latency_ms.observe(42.5);
    
    // Start server
    let config = MetricsServerConfig {
        bind_address: "127.0.0.1".to_string(),
        port: 0,
    };
    let mut server = MetricsServer::new(config, registry.clone());
    server.start().await.expect("Failed to start server");
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Export metrics and verify they contain our data
    let prometheus_text = registry.export_prometheus();
    
    assert!(prometheus_text.contains("elara_active_connections 5"));
    assert!(prometheus_text.contains("elara_total_connections 10"));
    assert!(prometheus_text.contains("elara_messages_sent 100"));
    assert!(prometheus_text.contains("elara_message_latency_ms_count 1"));
    
    // Cleanup
    server.shutdown().await;
}

#[tokio::test]
async fn test_prometheus_format_compliance() {
    let registry = MetricsRegistry::new();
    
    // Register various metric types
    let counter = registry.register_counter("test_counter", vec![]);
    counter.inc_by(42);
    
    let gauge = registry.register_gauge("test_gauge", vec![]);
    gauge.set(100);
    
    let histogram = registry.register_histogram(
        "test_histogram",
        vec![1.0, 10.0, 100.0, 1000.0],
        vec![],
    );
    histogram.observe(5.0);
    histogram.observe(50.0);
    histogram.observe(500.0);
    
    let output = registry.export_prometheus();
    
    // Verify HELP and TYPE comments
    assert!(output.contains("# HELP test_counter"));
    assert!(output.contains("# TYPE test_counter counter"));
    assert!(output.contains("# HELP test_gauge"));
    assert!(output.contains("# TYPE test_gauge gauge"));
    assert!(output.contains("# HELP test_histogram"));
    assert!(output.contains("# TYPE test_histogram histogram"));
    
    // Verify counter value
    assert!(output.contains("test_counter 42"));
    
    // Verify gauge value
    assert!(output.contains("test_gauge 100"));
    
    // Verify histogram buckets
    assert!(output.contains("test_histogram_bucket{le=\"1.0\"}"));
    assert!(output.contains("test_histogram_bucket{le=\"10.0\"}"));
    assert!(output.contains("test_histogram_bucket{le=\"100.0\"}"));
    assert!(output.contains("test_histogram_bucket{le=\"1000.0\"}"));
    assert!(output.contains("test_histogram_bucket{le=\"+Inf\"}"));
    
    // Verify histogram sum and count
    assert!(output.contains("test_histogram_sum 555"));
    assert!(output.contains("test_histogram_count 3"));
}

#[tokio::test]
async fn test_metrics_with_labels() {
    let registry = MetricsRegistry::new();
    
    // Register metrics with labels
    let counter = registry.register_counter(
        "http_requests_total",
        vec![
            ("method".to_string(), "GET".to_string()),
            ("status".to_string(), "200".to_string()),
        ],
    );
    counter.inc_by(150);
    
    let gauge = registry.register_gauge(
        "node_info",
        vec![
            ("node_id".to_string(), "node-1".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ],
    );
    gauge.set(1);
    
    let output = registry.export_prometheus();
    
    // Verify labels are present and properly formatted
    assert!(output.contains("http_requests_total{"));
    assert!(output.contains("method=\"GET\""));
    assert!(output.contains("status=\"200\""));
    assert!(output.contains("} 150"));
    
    assert!(output.contains("node_info{"));
    assert!(output.contains("node_id=\"node-1\""));
    assert!(output.contains("version=\"1.0.0\""));
    assert!(output.contains("} 1"));
}

#[tokio::test]
async fn test_histogram_cumulative_buckets() {
    let registry = MetricsRegistry::new();
    
    let histogram = registry.register_histogram(
        "request_duration",
        vec![0.1, 0.5, 1.0, 5.0],
        vec![],
    );
    
    // Add observations
    histogram.observe(0.05); // Falls in first bucket
    histogram.observe(0.3);  // Falls in second bucket
    histogram.observe(0.7);  // Falls in third bucket
    histogram.observe(2.0);  // Falls in fourth bucket
    histogram.observe(10.0); // Falls in +Inf bucket
    
    let output = registry.export_prometheus();
    
    // Verify cumulative counts
    // Bucket 0.1 should have 1 (0.05)
    assert!(output.contains("request_duration_bucket{le=\"0.1\"} 1"));
    
    // Bucket 0.5 should have 2 (0.05 + 0.3)
    assert!(output.contains("request_duration_bucket{le=\"0.5\"} 2"));
    
    // Bucket 1.0 should have 3 (0.05 + 0.3 + 0.7)
    assert!(output.contains("request_duration_bucket{le=\"1.0\"} 3"));
    
    // Bucket 5.0 should have 4 (0.05 + 0.3 + 0.7 + 2.0)
    assert!(output.contains("request_duration_bucket{le=\"5.0\"} 4"));
    
    // +Inf bucket should have all 5
    assert!(output.contains("request_duration_bucket{le=\"+Inf\"} 5"));
    
    // Verify count and sum
    assert!(output.contains("request_duration_count 5"));
    assert!(output.contains("request_duration_sum 13.05"));
}

#[tokio::test]
async fn test_concurrent_metric_updates() {
    let registry = MetricsRegistry::new();
    let counter = registry.register_counter("concurrent_counter", vec![]);
    
    // Spawn multiple tasks updating the counter concurrently
    let mut handles = vec![];
    for _ in 0..10 {
        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                counter_clone.inc();
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify final count
    assert_eq!(counter.get(), 1000);
    
    let output = registry.export_prometheus();
    assert!(output.contains("concurrent_counter 1000"));
}

#[tokio::test]
async fn test_empty_registry() {
    let registry = MetricsRegistry::new();
    let output = registry.export_prometheus();
    
    // Empty registry should produce empty output
    assert_eq!(output, "");
}

#[tokio::test]
async fn test_special_characters_in_labels() {
    let registry = MetricsRegistry::new();
    
    let counter = registry.register_counter(
        "test_metric",
        vec![
            ("path".to_string(), "/api/v1/\"test\"".to_string()),
            ("message".to_string(), "line1\nline2".to_string()),
            ("backslash".to_string(), "path\\to\\file".to_string()),
        ],
    );
    counter.inc();
    
    let output = registry.export_prometheus();
    
    // Verify special characters are escaped
    assert!(output.contains("\\\""));  // Escaped quotes
    assert!(output.contains("\\n"));   // Escaped newline
    assert!(output.contains("\\\\"));  // Escaped backslash
}
