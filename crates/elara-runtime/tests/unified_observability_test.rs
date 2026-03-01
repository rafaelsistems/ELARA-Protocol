//! Integration tests for unified observability initialization

use elara_runtime::observability::{
    init_observability, LogFormat, LogLevel, LogOutput, LoggingConfig, MetricsServerConfig,
    ObservabilityConfig, ObservabilityError, TracingConfig, TracingExporter,
};
use serial_test::serial;

#[tokio::test]
async fn test_observability_disabled_by_default() {
    // Default config should have all components disabled
    let config = ObservabilityConfig::default();
    assert!(config.logging.is_none());
    assert!(config.tracing.is_none());
    assert!(config.metrics_server.is_none());
}

#[tokio::test]
#[serial]
async fn test_observability_logging_only() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    let config = ObservabilityConfig {
        logging: Some(LoggingConfig {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Stdout,
        }),
        tracing: None,
        metrics_server: None,
    };

    let handle = init_observability(config).await;
    assert!(handle.is_ok());

    let handle = handle.unwrap();
    assert!(!handle.is_metrics_server_running());

    // Cleanup
    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_observability_metrics_server_only() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    let config = ObservabilityConfig {
        logging: None,
        tracing: None,
        metrics_server: Some(MetricsServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0, // Use port 0 to let OS assign a free port
        }),
    };

    let handle = init_observability(config).await;
    assert!(handle.is_ok());

    let handle = handle.unwrap();
    assert!(handle.is_metrics_server_running());

    // Verify metrics registry is accessible
    let counter = handle
        .metrics_registry()
        .register_counter("test_counter", vec![]);
    counter.inc();
    assert_eq!(counter.get(), 1);

    // Cleanup (shutdown consumes the handle)
    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_observability_tracing_disabled() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    let config = ObservabilityConfig {
        logging: None,
        tracing: Some(TracingConfig {
            service_name: "test".to_string(),
            exporter: TracingExporter::None, // Disabled
            sampling_rate: 1.0,
            resource_attributes: vec![],
        }),
        metrics_server: None,
    };

    let handle = init_observability(config).await;
    assert!(handle.is_ok());

    // Cleanup
    handle.unwrap().shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_observability_full_configuration() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    let config = ObservabilityConfig {
        logging: Some(LoggingConfig {
            level: LogLevel::Debug,
            format: LogFormat::Compact,
            output: LogOutput::Stdout,
        }),
        tracing: Some(TracingConfig {
            service_name: "test-service".to_string(),
            exporter: TracingExporter::None, // Disabled for testing
            sampling_rate: 0.5,
            resource_attributes: vec![
                ("environment".to_string(), "test".to_string()),
                ("version".to_string(), "1.0.0".to_string()),
            ],
        }),
        metrics_server: Some(MetricsServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0,
        }),
    };

    let handle = init_observability(config).await;
    assert!(handle.is_ok());

    let handle = handle.unwrap();
    assert!(handle.is_metrics_server_running());

    // Test metrics
    let counter = handle
        .metrics_registry()
        .register_counter("full_test_counter", vec![]);
    counter.inc_by(5);
    assert_eq!(counter.get(), 5);

    // Cleanup
    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_observability_metrics_registry_always_available() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    // Even without metrics server, registry should be available
    let config = ObservabilityConfig {
        logging: None,
        tracing: None,
        metrics_server: None,
    };

    let handle = init_observability(config).await.unwrap();

    // Metrics registry should still be accessible
    let counter = handle
        .metrics_registry()
        .register_counter("registry_test", vec![]);
    counter.inc();
    assert_eq!(counter.get(), 1);

    handle.shutdown().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_observability_graceful_shutdown() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    let config = ObservabilityConfig {
        logging: Some(LoggingConfig {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
        }),
        tracing: Some(TracingConfig {
            service_name: "shutdown-test".to_string(),
            exporter: TracingExporter::None,
            sampling_rate: 1.0,
            resource_attributes: vec![],
        }),
        metrics_server: Some(MetricsServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 0,
        }),
    };

    let handle = init_observability(config).await.unwrap();
    assert!(handle.is_metrics_server_running());

    // Shutdown should succeed
    let result = handle.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_observability_invalid_sampling_rate() {
    // Reset global state for testing
    elara_runtime::observability::reset_observability_for_testing();
    
    let config = ObservabilityConfig {
        logging: None,
        tracing: Some(TracingConfig {
            service_name: "test".to_string(),
            exporter: TracingExporter::None,
            sampling_rate: 1.5, // Invalid: > 1.0
            resource_attributes: vec![],
        }),
        metrics_server: None,
    };

    let result = init_observability(config).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, ObservabilityError::TracingInit(_)));
    }
}

#[tokio::test]
async fn test_node_config_observability_field() {
    use elara_runtime::node::NodeConfig;
    use std::time::Duration;

    // Test that NodeConfig has observability field
    let config = NodeConfig {
        tick_interval: Duration::from_millis(100),
        max_packet_buffer: 1000,
        max_outgoing_buffer: 1000,
        max_local_events: 1000,
        metrics: None,
        health_checks: None,
        observability: Some(ObservabilityConfig {
            logging: Some(LoggingConfig {
                level: LogLevel::Info,
                format: LogFormat::Json,
                output: LogOutput::Stdout,
            }),
            tracing: None,
            metrics_server: None,
        }),
    };

    assert!(config.observability.is_some());
}

#[tokio::test]
async fn test_node_config_default_observability_disabled() {
    use elara_runtime::node::NodeConfig;

    // Default NodeConfig should have observability disabled
    let config = NodeConfig::default();
    assert!(config.observability.is_none());
}
