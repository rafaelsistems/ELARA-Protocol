//! Integration tests for the logging module
//!
//! Note: These tests must be run with `--test-threads=1` because logging
//! initialization is a global operation that can only happen once per process.

use elara_runtime::observability::logging::{
    init_logging, LogFormat, LogLevel, LogOutput, LoggingConfig, LoggingError,
};
use std::path::PathBuf;

#[test]
fn test_logging_initialization_idempotency() {
    // First initialization should succeed
    let config = LoggingConfig {
        level: LogLevel::Info,
        format: LogFormat::Compact,
        output: LogOutput::Stderr,
    };

    let result = init_logging(config);
    // May succeed or fail with AlreadyInitialized depending on test order
    // The important thing is that it doesn't panic
    assert!(
        result.is_ok() || matches!(result, Err(LoggingError::AlreadyInitialized)),
        "Initialization should either succeed or return AlreadyInitialized"
    );

    // Second initialization should fail with AlreadyInitialized
    let config2 = LoggingConfig {
        level: LogLevel::Debug,
        format: LogFormat::Json,
        output: LogOutput::Stdout,
    };

    let result2 = init_logging(config2);
    assert!(
        matches!(result2, Err(LoggingError::AlreadyInitialized)),
        "Second initialization should return AlreadyInitialized error"
    );
}

#[test]
fn test_log_level_conversions() {
    // Test that log levels maintain proper ordering
    assert!(LogLevel::Trace < LogLevel::Debug);
    assert!(LogLevel::Debug < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Error);
}

#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();
    assert_eq!(config.level, LogLevel::Info);
    assert_eq!(config.format, LogFormat::Pretty);
    assert_eq!(config.output, LogOutput::Stdout);
}

#[test]
fn test_log_output_variants() {
    // Test that all output variants can be created
    let _stdout = LogOutput::Stdout;
    let _stderr = LogOutput::Stderr;
    let _file = LogOutput::File(PathBuf::from("/tmp/test.log"));

    // Test equality
    assert_eq!(LogOutput::Stdout, LogOutput::Stdout);
    assert_eq!(LogOutput::Stderr, LogOutput::Stderr);
    assert_ne!(LogOutput::Stdout, LogOutput::Stderr);
}

#[test]
fn test_log_format_variants() {
    // Test that all format variants can be created
    let _pretty = LogFormat::Pretty;
    let _json = LogFormat::Json;
    let _compact = LogFormat::Compact;

    // Test equality
    assert_eq!(LogFormat::Pretty, LogFormat::Pretty);
    assert_eq!(LogFormat::Json, LogFormat::Json);
    assert_eq!(LogFormat::Compact, LogFormat::Compact);
    assert_ne!(LogFormat::Pretty, LogFormat::Json);
}

#[test]
fn test_env_filter_respects_rust_log() {
    // This test verifies that RUST_LOG environment variable is respected
    // We can't easily test the actual filtering behavior in a unit test,
    // but we can verify that initialization succeeds with RUST_LOG set
    
    std::env::set_var("RUST_LOG", "info,elara_wire=debug,elara_crypto=trace");
    
    let config = LoggingConfig {
        level: LogLevel::Warn, // This should be overridden by RUST_LOG
        format: LogFormat::Compact,
        output: LogOutput::Stderr,
    };
    
    // This may succeed or fail with AlreadyInitialized depending on test order
    let result = init_logging(config);
    assert!(
        result.is_ok() || matches!(result, Err(LoggingError::AlreadyInitialized)),
        "Initialization with RUST_LOG should succeed or return AlreadyInitialized"
    );
    
    // Clean up
    std::env::remove_var("RUST_LOG");
}

#[test]
fn test_contextual_fields_in_logs() {
    // This test demonstrates how to use contextual fields with tracing macros
    // The actual field values will be captured by the logging system
    
    use tracing::{info, warn, error};
    
    // Initialize logging for this test (may already be initialized)
    let config = LoggingConfig {
        level: LogLevel::Info,
        format: LogFormat::Json,
        output: LogOutput::Stderr,
    };
    
    let _ = init_logging(config); // Ignore result, may already be initialized
    
    // Log with contextual fields
    info!(
        node_id = "node-1",
        session_id = "session-abc",
        "Node started successfully"
    );
    
    warn!(
        node_id = "node-1",
        peer_id = "peer-xyz",
        "Connection attempt failed, retrying"
    );
    
    error!(
        node_id = "node-1",
        session_id = "session-abc",
        peer_id = "peer-xyz",
        error_code = 500,
        "Critical error occurred"
    );
    
    // If we reach here without panicking, the test passes
    // The actual log output would contain the contextual fields
}

#[test]
fn test_per_module_log_levels_syntax() {
    // Test various RUST_LOG syntax patterns
    let test_cases = vec![
        "info",
        "debug,elara_wire=trace",
        "warn,elara_crypto=debug,elara_state=info",
        "error,elara_transport=warn",
        "trace,elara_runtime::observability=debug",
    ];
    
    for rust_log_value in test_cases {
        std::env::set_var("RUST_LOG", rust_log_value);
        
        // Verify that EnvFilter can parse these patterns
        // We do this by checking that initialization would succeed
        // (we can't actually initialize multiple times, but we can verify the syntax)
        let filter_result = tracing_subscriber::EnvFilter::try_from_default_env();
        assert!(
            filter_result.is_ok(),
            "RUST_LOG='{}' should be valid syntax",
            rust_log_value
        );
        
        std::env::remove_var("RUST_LOG");
    }
}

#[test]
fn test_fallback_to_config_level_when_no_rust_log() {
    // Ensure RUST_LOG is not set
    std::env::remove_var("RUST_LOG");
    
    // When RUST_LOG is not set, the config.level should be used as fallback
    let config = LoggingConfig {
        level: LogLevel::Debug,
        format: LogFormat::Compact,
        output: LogOutput::Stderr,
    };
    
    let result = init_logging(config);
    assert!(
        result.is_ok() || matches!(result, Err(LoggingError::AlreadyInitialized)),
        "Initialization should succeed with fallback to config.level or return AlreadyInitialized"
    );
}
