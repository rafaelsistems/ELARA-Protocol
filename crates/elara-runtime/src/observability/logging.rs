//! Structured logging system for ELARA Protocol
//!
//! This module provides structured, queryable logging with support for:
//! - Multiple log levels (Trace, Debug, Info, Warn, Error)
//! - Multiple output formats (JSON, Pretty, Compact)
//! - Multiple output destinations (Stdout, Stderr, File)
//! - Per-module log level configuration via RUST_LOG environment variable
//! - Contextual fields (node_id, session_id, peer_id) in all log entries
//!
//! # Per-Module Log Levels
//!
//! You can configure different log levels for different modules using the `RUST_LOG`
//! environment variable. This is useful for debugging specific components without
//! overwhelming logs from other parts of the system.
//!
//! ## Examples
//!
//! ```bash
//! # Set all modules to info, but elara_wire to debug
//! RUST_LOG=info,elara_wire=debug
//!
//! # Set elara_crypto to trace, everything else to warn
//! RUST_LOG=warn,elara_crypto=trace
//!
//! # Multiple module overrides
//! RUST_LOG=info,elara_wire=debug,elara_state=trace,elara_transport=warn
//! ```
//!
//! # Contextual Fields
//!
//! The logging system supports attaching contextual fields to log entries. These fields
//! provide additional context about the operation being logged:
//!
//! - `node_id`: Identifier of the node generating the log
//! - `session_id`: Current session identifier
//! - `peer_id`: Identifier of the peer involved in the operation
//!
//! Use the `tracing` macros with field syntax to add contextual information:
//!
//! ```no_run
//! use tracing::info;
//!
//! info!(
//!     node_id = "node-1",
//!     session_id = "session-abc",
//!     peer_id = "peer-xyz",
//!     "Connection established"
//! );
//! ```
//!
//! # Basic Example
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

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Global flag to track if logging has been initialized
static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Reset the logging initialization flag.
///
/// **WARNING**: This function is only for testing purposes and should never be used
/// in production code. It allows tests to re-initialize the logging system.
///
/// # Safety
///
/// This function is only available when running tests. Using this in production
/// could lead to undefined behavior as it allows multiple initializations of global state.
#[doc(hidden)]
pub fn reset_logging_for_testing() {
    LOGGING_INITIALIZED.store(false, Ordering::SeqCst);
}

/// Configuration for the logging system
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level threshold
    pub level: LogLevel,
    /// Output format
    pub format: LogFormat,
    /// Output destination
    pub output: LogOutput,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
        }
    }
}

/// Log level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Trace level - most verbose
    Trace,
    /// Debug level - detailed information
    Debug,
    /// Info level - general information
    Info,
    /// Warn level - warnings
    Warn,
    /// Error level - errors only
    Error,
}

impl LogLevel {
    /// Convert to filter directive string for EnvFilter
    fn to_filter_directive(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable format for development
    Pretty,
    /// JSON format for production log aggregation
    Json,
    /// Compact format for high-throughput scenarios
    Compact,
}

/// Log output destination
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogOutput {
    /// Write to stdout
    Stdout,
    /// Write to stderr
    Stderr,
    /// Write to a file
    File(PathBuf),
}

/// Errors that can occur during logging initialization
#[derive(Debug, Error)]
pub enum LoggingError {
    /// Logging has already been initialized
    #[error("Logging system has already been initialized")]
    AlreadyInitialized,

    /// Failed to set global default subscriber
    #[error("Failed to set global default subscriber: {0}")]
    SetGlobalDefaultFailed(String),

    /// Failed to open log file
    #[error("Failed to open log file: {0}")]
    FileOpenFailed(#[from] std::io::Error),
}

/// Initialize the logging system with the given configuration
///
/// This function sets up the tracing subscriber with the specified configuration.
/// It can only be called once - subsequent calls will return `LoggingError::AlreadyInitialized`.
///
/// # Per-Module Log Levels
///
/// The logging system respects the `RUST_LOG` environment variable for per-module
/// log level configuration. If `RUST_LOG` is set, it takes precedence over the
/// `config.level` parameter for fine-grained control.
///
/// The `config.level` serves as the default/fallback level when `RUST_LOG` is not set.
///
/// # Arguments
///
/// * `config` - Logging configuration specifying level, format, and output
///
/// # Returns
///
/// * `Ok(())` - Logging initialized successfully
/// * `Err(LoggingError)` - Initialization failed
///
/// # Example
///
/// ```no_run
/// use elara_runtime::observability::logging::{LoggingConfig, LogLevel, LogFormat, LogOutput, init_logging};
///
/// let config = LoggingConfig {
///     level: LogLevel::Info,
///     format: LogFormat::Json,
///     output: LogOutput::Stdout,
/// };
///
/// init_logging(config).expect("Failed to initialize logging");
/// ```
///
/// # Idempotency
///
/// This function is idempotent in the sense that calling it multiple times will not
/// reinitialize the logging system. The second and subsequent calls will return
/// `Err(LoggingError::AlreadyInitialized)`.
pub fn init_logging(config: LoggingConfig) -> Result<(), LoggingError> {
    // Check if already initialized (atomic operation)
    if LOGGING_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(LoggingError::AlreadyInitialized);
    }

    // Build EnvFilter that respects RUST_LOG environment variable
    // Falls back to config.level if RUST_LOG is not set
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| {
            // If RUST_LOG is not set, use the configured level as default
            EnvFilter::try_new(format!("{}", config.level.to_filter_directive()))
        })
        .map_err(|e| LoggingError::SetGlobalDefaultFailed(format!("Failed to create EnvFilter: {}", e)))?;

    // Build the fmt layer based on format and output configuration
    let result = match (config.format, config.output) {
        // JSON format to stdout
        (LogFormat::Json, LogOutput::Stdout) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // JSON format to stderr
        (LogFormat::Json, LogOutput::Stderr) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // JSON format to file
        (LogFormat::Json, LogOutput::File(path)) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // Pretty format to stdout
        (LogFormat::Pretty, LogOutput::Stdout) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // Pretty format to stderr
        (LogFormat::Pretty, LogOutput::Stderr) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // Pretty format to file
        (LogFormat::Pretty, LogOutput::File(path)) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            let fmt_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_writer(file)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_file(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // Compact format to stdout
        (LogFormat::Compact, LogOutput::Stdout) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // Compact format to stderr
        (LogFormat::Compact, LogOutput::Stderr) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }

        // Compact format to file
        (LogFormat::Compact, LogOutput::File(path)) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            let fmt_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_writer(file)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
        }
    };

    result.map_err(|e| {
        let err_msg = e.to_string();
        // If the error is that a global default is already set, and we're in test mode,
        // we can safely ignore it since tests run serially with #[serial]
        if err_msg.contains("global default trace dispatcher has already been set") {
            // Don't reset the flag - logging is effectively initialized
            return LoggingError::AlreadyInitialized;
        }
        // For other errors, reset flag
        LOGGING_INITIALIZED.store(false, Ordering::SeqCst);
        LoggingError::SetGlobalDefaultFailed(err_msg)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Pretty);
        assert_eq!(config.output, LogOutput::Stdout);
    }
}
