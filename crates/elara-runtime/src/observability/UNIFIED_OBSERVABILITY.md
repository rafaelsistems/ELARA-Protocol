# Unified Observability System

The unified observability system provides a single entry point for initializing all observability components in the ELARA Protocol runtime. This document describes the architecture, usage, and best practices for the unified observability system.

## Overview

The unified observability system combines three core components:

1. **Structured Logging**: JSON/Pretty/Compact formats with per-module log levels
2. **Distributed Tracing**: OpenTelemetry integration with Jaeger/Zipkin/OTLP support
3. **Metrics Server**: HTTP server exposing Prometheus metrics

All components are optional and can be enabled/disabled independently through configuration.

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                  init_observability()                        │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Logging    │  │   Tracing    │  │  Metrics Server  │  │
│  │              │  │              │  │                  │  │
│  │ - JSON       │  │ - Jaeger     │  │ - Prometheus     │  │
│  │ - Pretty     │  │ - Zipkin     │  │ - HTTP /metrics  │  │
│  │ - Compact    │  │ - OTLP       │  │ - Port 9090      │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
│                                                              │
│                  ObservabilityHandle                         │
│                  - shutdown()                                │
│                  - metrics_registry()                        │
└─────────────────────────────────────────────────────────────┘
```

### Key Types

- **`ObservabilityConfig`**: Unified configuration for all components
- **`ObservabilityHandle`**: Handle for graceful shutdown and metrics access
- **`ObservabilityError`**: Error type for initialization and shutdown failures

## Usage

### Basic Usage

```rust
use elara_runtime::observability::{
    init_observability, LoggingConfig, LogLevel, LogFormat, LogOutput,
    ObservabilityConfig
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure observability
    let config = ObservabilityConfig {
        logging: Some(LoggingConfig {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Stdout,
        }),
        tracing: None,
        metrics_server: None,
    };

    // Initialize
    let handle = init_observability(config).await?;

    // Use observability throughout your application
    tracing::info!("Application started");

    // Graceful shutdown
    handle.shutdown().await?;

    Ok(())
}
```

### Production Configuration

For production deployments, enable all components:

```rust
use elara_runtime::observability::{
    init_observability, LoggingConfig, LogLevel, LogFormat, LogOutput,
    TracingConfig, TracingExporter, MetricsServerConfig, ObservabilityConfig
};

let config = ObservabilityConfig {
    // Structured logging in JSON format
    logging: Some(LoggingConfig {
        level: LogLevel::Info,
        format: LogFormat::Json,
        output: LogOutput::Stdout,
    }),

    // Distributed tracing with OTLP
    tracing: Some(TracingConfig {
        service_name: "elara-node".to_string(),
        exporter: TracingExporter::Otlp {
            endpoint: "http://otel-collector:4317".to_string(),
        },
        sampling_rate: 0.1, // Sample 10% of traces
        resource_attributes: vec![
            ("environment".to_string(), "production".to_string()),
            ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
            ("region".to_string(), "us-west-2".to_string()),
        ],
    }),

    // Metrics server for Prometheus scraping
    metrics_server: Some(MetricsServerConfig {
        bind_address: "0.0.0.0".to_string(),
        port: 9090,
    }),
};

let handle = init_observability(config).await?;
```

### Development Configuration

For local development, use minimal configuration:

```rust
let config = ObservabilityConfig {
    logging: Some(LoggingConfig {
        level: LogLevel::Debug,
        format: LogFormat::Pretty, // Human-readable
        output: LogOutput::Stdout,
    }),
    tracing: None,        // Disabled
    metrics_server: None, // Disabled
};

let handle = init_observability(config).await?;
```

### Integration with NodeConfig

The unified observability system integrates with `NodeConfig`:

```rust
use elara_runtime::node::NodeConfig;
use elara_runtime::observability::{
    ObservabilityConfig, LoggingConfig, LogLevel, LogFormat, LogOutput
};
use std::time::Duration;

let config = NodeConfig {
    tick_interval: Duration::from_millis(100),
    max_packet_buffer: 1000,
    max_outgoing_buffer: 1000,
    max_local_events: 1000,
    metrics: None,
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
```

## Features

### Idempotency

The `init_observability()` function is idempotent - it can only be called once per process:

```rust
let config = ObservabilityConfig::default();

// First call succeeds
let handle1 = init_observability(config.clone()).await?;

// Second call fails with AlreadyInitialized error
let result = init_observability(config).await;
assert!(matches!(result, Err(ObservabilityError::AlreadyInitialized)));
```

This ensures that global state (logging subscriber, tracer provider) is only set once.

### Graceful Shutdown

The `ObservabilityHandle` provides graceful shutdown:

```rust
let handle = init_observability(config).await?;

// ... application runs ...

// Graceful shutdown - flushes all pending telemetry
handle.shutdown().await?;
```

Shutdown ensures:
- All pending log entries are written
- All pending trace spans are exported
- Metrics server stops accepting requests
- Resources are cleaned up properly

### Metrics Registry Access

The metrics registry is always available, even if the metrics server is not started:

```rust
let config = ObservabilityConfig {
    logging: None,
    tracing: None,
    metrics_server: None, // Server disabled
};

let handle = init_observability(config).await?;

// Metrics registry is still accessible
let counter = handle.metrics_registry()
    .register_counter("my_counter", vec![]);
counter.inc();
```

This allows applications to collect metrics without exposing them via HTTP.

## Configuration Options

### Logging Configuration

```rust
pub struct LoggingConfig {
    /// Log level threshold (Trace, Debug, Info, Warn, Error)
    pub level: LogLevel,
    
    /// Output format (Pretty, Json, Compact)
    pub format: LogFormat,
    
    /// Output destination (Stdout, Stderr, File)
    pub output: LogOutput,
}
```

**Log Levels:**
- `Trace`: Most verbose, includes all details
- `Debug`: Detailed information for debugging
- `Info`: General informational messages
- `Warn`: Warning messages
- `Error`: Error messages only

**Log Formats:**
- `Pretty`: Human-readable with colors and indentation (development)
- `Json`: Machine-readable JSON format (production)
- `Compact`: Compact format for high-throughput scenarios

**Per-Module Log Levels:**

Use the `RUST_LOG` environment variable for fine-grained control:

```bash
# Set all to info, but elara_wire to debug
RUST_LOG=info,elara_wire=debug

# Multiple module overrides
RUST_LOG=info,elara_wire=debug,elara_crypto=trace,elara_state=warn
```

### Tracing Configuration

```rust
pub struct TracingConfig {
    /// Service name to identify this node in traces
    pub service_name: String,
    
    /// Exporter backend (Jaeger, Zipkin, OTLP, None)
    pub exporter: TracingExporter,
    
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
    
    /// Additional resource attributes (key-value pairs)
    pub resource_attributes: Vec<(String, String)>,
}
```

**Exporters:**
- `Jaeger`: Good for development/testing
  - Endpoint format: `http://localhost:14268/api/traces`
- `Zipkin`: For compatibility with Zipkin infrastructure
  - Endpoint format: `http://localhost:9411/api/v2/spans`
- `OTLP`: Production standard (OpenTelemetry Protocol)
  - Endpoint format: `http://localhost:4317` (gRPC)
- `None`: Tracing disabled

**Sampling Rates:**
- `1.0`: Sample all traces (100%)
- `0.1`: Sample 10% of traces
- `0.01`: Sample 1% of traces
- `0.0`: No sampling (disabled)

**Resource Attributes:**

Add metadata to all spans for filtering/grouping:

```rust
resource_attributes: vec![
    ("environment".to_string(), "production".to_string()),
    ("version".to_string(), "1.0.0".to_string()),
    ("region".to_string(), "us-west-2".to_string()),
    ("node.id".to_string(), "node-1".to_string()),
]
```

### Metrics Server Configuration

```rust
pub struct MetricsServerConfig {
    /// IP address to bind to
    pub bind_address: String,
    
    /// Port to listen on
    pub port: u16,
}
```

**Bind Addresses:**
- `0.0.0.0`: All interfaces (production)
- `127.0.0.1`: Localhost only (development)

**Ports:**
- `9090`: Standard Prometheus port
- `0`: Let OS assign a free port (testing)

## Error Handling

The unified observability system uses the `ObservabilityError` type:

```rust
pub enum ObservabilityError {
    /// Observability system already initialized
    AlreadyInitialized,
    
    /// Failed to initialize logging
    LoggingInit(LoggingError),
    
    /// Failed to initialize tracing
    TracingInit(TracingError),
    
    /// Failed to start metrics server
    MetricsServerStart(MetricsServerError),
    
    /// Failed to shutdown tracing
    TracingShutdown(TracingError),
}
```

### Error Recovery

If initialization fails, the global initialization flag is reset, allowing retry:

```rust
let config = ObservabilityConfig { /* ... */ };

match init_observability(config.clone()).await {
    Ok(handle) => {
        // Success
    }
    Err(e) => {
        eprintln!("Failed to initialize observability: {}", e);
        // Can retry with different config
        let fallback_config = ObservabilityConfig {
            logging: Some(LoggingConfig::default()),
            tracing: None,
            metrics_server: None,
        };
        let handle = init_observability(fallback_config).await?;
    }
}
```

## Best Practices

### 1. Always Shut Down Gracefully

```rust
let handle = init_observability(config).await?;

// Register shutdown handler
tokio::spawn(async move {
    tokio::signal::ctrl_c().await.unwrap();
    handle.shutdown().await.unwrap();
    std::process::exit(0);
});
```

### 2. Use Appropriate Sampling Rates

For high-traffic systems, use lower sampling rates:

```rust
tracing: Some(TracingConfig {
    service_name: "elara-node".to_string(),
    exporter: TracingExporter::Otlp { /* ... */ },
    sampling_rate: 0.01, // 1% sampling for high traffic
    resource_attributes: vec![],
}),
```

### 3. Add Resource Attributes

Always add environment, version, and region attributes:

```rust
resource_attributes: vec![
    ("environment".to_string(), std::env::var("ENV").unwrap_or_else(|_| "development".to_string())),
    ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
    ("region".to_string(), std::env::var("REGION").unwrap_or_else(|_| "unknown".to_string())),
],
```

### 4. Use JSON Format in Production

```rust
logging: Some(LoggingConfig {
    level: LogLevel::Info,
    format: LogFormat::Json, // Machine-readable
    output: LogOutput::Stdout,
}),
```

### 5. Disable Unnecessary Components

Only enable what you need:

```rust
// Development: logging only
let config = ObservabilityConfig {
    logging: Some(LoggingConfig { /* ... */ }),
    tracing: None,
    metrics_server: None,
};

// Production: all components
let config = ObservabilityConfig {
    logging: Some(LoggingConfig { /* ... */ }),
    tracing: Some(TracingConfig { /* ... */ }),
    metrics_server: Some(MetricsServerConfig { /* ... */ }),
};
```

## Kubernetes Integration

### Deployment Configuration

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: elara-node
spec:
  template:
    spec:
      containers:
      - name: elara-node
        image: elara-node:latest
        ports:
        - name: metrics
          containerPort: 9090
        env:
        - name: RUST_LOG
          value: "info,elara_wire=debug"
        - name: ENV
          value: "production"
        - name: REGION
          valueFrom:
            fieldRef:
              fieldPath: metadata.labels['topology.kubernetes.io/region']
```

### Service for Metrics

```yaml
apiVersion: v1
kind: Service
metadata:
  name: elara-node-metrics
  labels:
    app: elara-node
spec:
  ports:
  - name: metrics
    port: 9090
    targetPort: 9090
  selector:
    app: elara-node
```

### ServiceMonitor for Prometheus

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: elara-node
spec:
  selector:
    matchLabels:
      app: elara-node
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

### OpenTelemetry Collector

```yaml
apiVersion: v1
kind: Service
metadata:
  name: otel-collector
spec:
  ports:
  - name: otlp-grpc
    port: 4317
    targetPort: 4317
  selector:
    app: otel-collector
```

## Examples

See the following examples for complete usage demonstrations:

- `examples/unified_observability.rs`: Full configuration with all components
- `examples/observability_minimal.rs`: Minimal configuration (logging only)
- `examples/logging_with_env_filter.rs`: Advanced logging configuration
- `examples/metrics_server.rs`: Metrics server usage
- `examples/tracing_example.rs`: Distributed tracing usage

## Testing

The unified observability system includes comprehensive integration tests:

```bash
cargo test --test unified_observability_test
```

Tests cover:
- Default configuration (all disabled)
- Individual component initialization
- Full configuration
- Idempotency
- Graceful shutdown
- Error handling
- NodeConfig integration

## Troubleshooting

### Issue: "AlreadyInitialized" Error

**Cause**: `init_observability()` was called multiple times.

**Solution**: Only call `init_observability()` once per process. Store the handle and reuse it.

### Issue: Metrics Server Fails to Start

**Cause**: Port already in use or insufficient permissions.

**Solution**: 
- Check if another process is using the port: `netstat -an | grep 9090`
- Use a different port or port 0 (OS-assigned)
- Ensure the process has permission to bind to the port

### Issue: Traces Not Appearing

**Cause**: Exporter endpoint unreachable or sampling rate too low.

**Solution**:
- Verify exporter endpoint is accessible
- Increase sampling rate for testing (1.0 = 100%)
- Check exporter logs for errors

### Issue: Logs Not Appearing

**Cause**: Log level too high or RUST_LOG filtering logs.

**Solution**:
- Lower log level (Debug or Trace)
- Check RUST_LOG environment variable
- Verify output destination is correct

## Performance Considerations

### Logging

- **JSON format**: ~10% overhead compared to Pretty
- **Compact format**: ~5% overhead, best for high-throughput
- **File output**: Slightly faster than stdout

### Tracing

- **Sampling**: Use 0.1 (10%) or lower for production
- **OTLP**: Most efficient exporter
- **Batch export**: Configured automatically

### Metrics

- **Registry**: Lock-free atomic operations
- **HTTP server**: Async, non-blocking
- **Scraping**: Minimal impact (<1ms per scrape)

## Security Considerations

### Metrics Endpoint

- Bind to `127.0.0.1` for localhost-only access
- Use network policies in Kubernetes to restrict access
- Consider authentication for public endpoints

### Logging

- Avoid logging sensitive data (passwords, tokens, PII)
- Use structured logging to control what gets logged
- Sanitize user input before logging

### Tracing

- Be cautious with trace attributes (may contain sensitive data)
- Use sampling to reduce data volume
- Configure retention policies on trace backends

## Future Enhancements

Planned improvements for the unified observability system:

1. **Health Checks**: Integrate health check endpoints
2. **Alerting**: Built-in alerting rules
3. **Profiling**: CPU and memory profiling support
4. **Custom Exporters**: Plugin system for custom exporters
5. **Dynamic Configuration**: Runtime configuration updates
6. **Correlation IDs**: Automatic correlation ID propagation

## References

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Tracing Crate Documentation](https://docs.rs/tracing/)
- [ELARA Protocol Specification](../../docs/PROTOCOL.md)
