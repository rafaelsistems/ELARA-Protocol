# Distributed Tracing System

This module provides distributed tracing capabilities for the ELARA Protocol using OpenTelemetry. It enables end-to-end request tracing across nodes, helping with debugging, performance analysis, and understanding system behavior.

## Features

- **Multiple Exporter Backends**: Support for Jaeger, Zipkin, OTLP, or disabled tracing
- **Configurable Sampling**: Control what percentage of traces are exported (0.0 to 1.0)
- **Resource Attributes**: Attach custom key-value pairs to all spans for filtering/grouping
- **Idempotent Initialization**: Can only be initialized once to prevent conflicts
- **Graceful Shutdown**: Ensures all pending spans are flushed before termination

## Supported Exporters

### Jaeger
Good for development and testing. Provides a comprehensive UI for trace visualization.

```rust
TracingExporter::Jaeger {
    endpoint: "http://localhost:14268/api/traces".to_string(),
}
```

**Running Jaeger locally:**
```bash
docker run -d --name jaeger \
  -p 14268:14268 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

Access UI at: http://localhost:16686

### Zipkin
Compatible with existing Zipkin infrastructure.

```rust
TracingExporter::Zipkin {
    endpoint: "http://localhost:9411/api/v2/spans".to_string(),
}
```

**Running Zipkin locally:**
```bash
docker run -d --name zipkin \
  -p 9411:9411 \
  openzipkin/zipkin:latest
```

Access UI at: http://localhost:9411

### OTLP (OpenTelemetry Protocol)
Production-standard protocol, works with OpenTelemetry Collector and various backends.

```rust
TracingExporter::Otlp {
    endpoint: "http://localhost:4317".to_string(),
}
```

**Running OTLP Collector locally:**
```bash
docker run -d --name otel-collector \
  -p 4317:4317 \
  -p 4318:4318 \
  otel/opentelemetry-collector:latest
```

### None
Disables tracing entirely.

```rust
TracingExporter::None
```

## Usage

### Basic Initialization

```rust
use elara_runtime::observability::tracing::{init_tracing, TracingConfig, TracingExporter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TracingConfig {
        service_name: "elara-node".to_string(),
        exporter: TracingExporter::Jaeger {
            endpoint: "http://localhost:14268/api/traces".to_string(),
        },
        sampling_rate: 1.0, // Sample all traces
        resource_attributes: vec![
            ("environment".to_string(), "production".to_string()),
            ("node.id".to_string(), "node-1".to_string()),
        ],
    };

    let handle = init_tracing(config).await?;

    // Your application code here
    tracing::info!("Application started");

    // Graceful shutdown
    handle.shutdown().await?;
    Ok(())
}
```

### Creating Spans

```rust
use tracing::{info, span, Level};

// Create a span
let span = span!(Level::INFO, "operation_name", key = "value");
let _enter = span.enter();

info!("Doing work inside span");

// Nested spans
{
    let child_span = span!(Level::INFO, "child_operation");
    let _child_enter = child_span.enter();
    info!("Doing work in child span");
}
```

### Sampling Configuration

Control what percentage of traces are exported:

```rust
// Sample all traces (development)
sampling_rate: 1.0

// Sample 10% of traces (production with high traffic)
sampling_rate: 0.1

// Sample 1% of traces (production with very high traffic)
sampling_rate: 0.01

// Disable sampling (no traces exported)
sampling_rate: 0.0
```

### Resource Attributes

Add custom attributes to all spans for filtering and grouping:

```rust
resource_attributes: vec![
    ("environment".to_string(), "production".to_string()),
    ("region".to_string(), "us-west-2".to_string()),
    ("node.id".to_string(), "node-1".to_string()),
    ("version".to_string(), "1.0.0".to_string()),
]
```

## Integration with Logging

The tracing system integrates with the logging module. Initialize logging first, then tracing:

```rust
use elara_runtime::observability::logging::{init_logging, LoggingConfig, LogLevel, LogFormat, LogOutput};
use elara_runtime::observability::tracing::{init_tracing, TracingConfig, TracingExporter};

// Initialize logging first
let log_config = LoggingConfig {
    level: LogLevel::Info,
    format: LogFormat::Json,
    output: LogOutput::Stdout,
};
init_logging(log_config)?;

// Then initialize tracing
let trace_config = TracingConfig {
    service_name: "elara-node".to_string(),
    exporter: TracingExporter::Otlp {
        endpoint: "http://localhost:4317".to_string(),
    },
    sampling_rate: 0.1,
    resource_attributes: vec![],
};
let handle = init_tracing(trace_config).await?;
```

## Error Handling

The module defines several error types:

- `AlreadyInitialized`: Tracing has already been initialized (idempotency check)
- `InvalidSamplingRate`: Sampling rate must be between 0.0 and 1.0
- `ExporterInitialization`: Failed to initialize the exporter backend
- `GlobalTracerSetup`: Failed to set the global tracer provider

```rust
match init_tracing(config).await {
    Ok(handle) => {
        // Tracing initialized successfully
    }
    Err(TracingError::AlreadyInitialized) => {
        // Tracing was already initialized
    }
    Err(TracingError::InvalidSamplingRate(rate)) => {
        eprintln!("Invalid sampling rate: {}", rate);
    }
    Err(e) => {
        eprintln!("Failed to initialize tracing: {}", e);
    }
}
```

## Production Recommendations

1. **Use OTLP exporter** for production deployments
2. **Set appropriate sampling rate** based on traffic volume:
   - Low traffic (<100 req/s): 1.0 (sample all)
   - Medium traffic (100-1000 req/s): 0.1 (sample 10%)
   - High traffic (>1000 req/s): 0.01 (sample 1%)
3. **Add resource attributes** for filtering:
   - environment (production, staging, development)
   - region/datacenter
   - node ID
   - version
4. **Use graceful shutdown** to ensure all traces are exported
5. **Monitor exporter health** to ensure traces are being delivered

## Example

See `examples/tracing_example.rs` for a complete working example:

```bash
cargo run --example tracing_example -- jaeger
```

## Dependencies

The tracing module requires the following dependencies:

```toml
tracing-opentelemetry = "0.22"
opentelemetry = { version = "0.21", features = ["trace"] }
opentelemetry_sdk = { version = "0.21", features = ["trace", "rt-tokio"] }
opentelemetry-jaeger = { version = "0.20", features = ["rt-tokio"] }
opentelemetry-zipkin = "0.19"
opentelemetry-otlp = { version = "0.14", features = ["trace", "grpc-tonic"] }
```

## Architecture

The tracing system follows this flow:

1. **Initialization**: `init_tracing()` sets up the OpenTelemetry pipeline
2. **Span Creation**: Application code creates spans using `tracing` macros
3. **Context Propagation**: Trace context is automatically propagated across async boundaries
4. **Export**: Spans are batched and exported to the configured backend
5. **Shutdown**: `handle.shutdown()` flushes pending spans and cleans up resources

## Troubleshooting

### Traces not appearing in backend

1. Check that the exporter endpoint is correct and reachable
2. Verify the backend is running and accepting traces
3. Check sampling rate - if too low, traces may not be sampled
4. Ensure `handle.shutdown()` is called to flush pending spans

### Performance impact

- Tracing has minimal overhead when sampling rate is low
- Use async exporters (default) to avoid blocking application threads
- Batch exporting reduces network overhead
- Consider sampling rate based on traffic volume

### Memory usage

- Spans are buffered before export
- Adjust batch size if memory usage is a concern
- Lower sampling rate reduces memory usage
- Ensure regular shutdown/flush to prevent unbounded growth


## Instrumented Operations

The ELARA runtime automatically creates spans for key operations, providing end-to-end visibility into message flow and state synchronization.

### Node Operations

#### `node_tick`
Root span for each node tick cycle. Contains all other operation spans as children.

**Attributes:**
- `node_id`: The ID of the node performing the tick
- `session_id`: Optional session ID if the node is in a session

**Child spans:**
- `ingest_packets`
- `decrypt_and_validate`
- `classify_events`
- `update_time_model`
- `state_reconciliation`
- `authorize_and_sign`
- `build_packets`

#### `ingest_packets`
Span for ingesting incoming packets from the buffer.

**Attributes:**
- `node_id`: The ID of the node
- `packet_count`: Number of packets ingested (logged)

#### `decrypt_and_validate`
Span for decrypting and validating packets.

**Attributes:**
- `node_id`: The ID of the node
- `packet_count`: Number of packets to decrypt
- `validated_count`: Number of successfully validated packets (logged)
- `failed_count`: Number of packets that failed validation (logged)

#### `classify_events`
Span for extracting events from validated packets.

**Attributes:**
- `node_id`: The ID of the node
- `packet_count`: Number of packets to classify
- `event_count`: Number of events extracted (logged)

For each frame processed:
- `source`: Source node ID (trace level)
- `event_count`: Events decoded from frame (trace level)
- `packet_class`: Packet classification (trace level)

#### `update_time_model`
Span for updating the time synchronization model.

**Attributes:**
- `node_id`: The ID of the node
- `event_count`: Number of events used for time update

#### `state_reconciliation`
Span for reconciling state with received events.

**Attributes:**
- `node_id`: The ID of the node
- `accepted`: Number of events accepted (logged)
- `rejected`: Number of events rejected (logged)

#### `authorize_and_sign`
Span for authorizing and signing local events.

**Attributes:**
- `node_id`: The ID of the node
- `event_count`: Number of events to sign

#### `build_packets`
Span for building packets from authorized events.

**Attributes:**
- `node_id`: The ID of the node
- `event_count`: Number of events to encode
- `packets_built`: Number of packets successfully built (logged)

### Connection Operations

#### `join_session`
Span for establishing a connection to a session.

**Attributes:**
- `node_id`: The ID of the node joining
- `session_id`: The session ID being joined

#### `join_session_unsecured`
Span for joining a session without encryption (unsecured mode).

**Attributes:**
- `node_id`: The ID of the node joining
- `session_id`: The session ID being joined

#### `leave_session`
Span for leaving a session and tearing down the connection.

**Attributes:**
- `node_id`: The ID of the node leaving
- `session_id`: Optional session ID being left

## Span Helper Functions

The module provides helper functions for creating common span patterns:

### `span_message_send(node_id, session_id, message_count)`
Creates a span for message send operations.

```rust
use elara_runtime::observability::tracing::span_message_send;

let span = span_message_send(node_id.0, Some(session_id.0), 5);
let _enter = span.enter();
// ... send messages ...
```

### `span_message_receive(node_id, session_id, message_count)`
Creates a span for message receive operations.

```rust
use elara_runtime::observability::tracing::span_message_receive;

let span = span_message_receive(node_id.0, Some(session_id.0), 3);
let _enter = span.enter();
// ... receive messages ...
```

### `span_state_sync(node_id, session_id, event_count)`
Creates a span for state synchronization operations.

```rust
use elara_runtime::observability::tracing::span_state_sync;

let span = span_state_sync(node_id.0, Some(session_id.0), 10);
let _enter = span.enter();
// ... synchronize state ...
```

### `span_connection_establish(node_id, session_id)`
Creates a span for connection establishment.

```rust
use elara_runtime::observability::tracing::span_connection_establish;

let span = span_connection_establish(node_id.0, session_id.0);
let _enter = span.enter();
// ... establish connection ...
```

### `span_connection_teardown(node_id, session_id)`
Creates a span for connection teardown.

```rust
use elara_runtime::observability::tracing::span_connection_teardown;

let span = span_connection_teardown(node_id.0, Some(session_id.0));
let _enter = span.enter();
// ... teardown connection ...
```

### `span_node_tick(node_id, session_id)`
Creates a span for node tick operations.

```rust
use elara_runtime::observability::tracing::span_node_tick;

let span = span_node_tick(node_id.0, Some(session_id.0));
let _enter = span.enter();
// ... perform tick ...
```

### `span_decrypt(node_id, packet_count)`
Creates a span for decryption operations.

```rust
use elara_runtime::observability::tracing::span_decrypt;

let span = span_decrypt(node_id.0, 5);
let _enter = span.enter();
// ... decrypt packets ...
```

### `span_classify_events(node_id, packet_count)`
Creates a span for event classification.

```rust
use elara_runtime::observability::tracing::span_classify_events;

let span = span_classify_events(node_id.0, 3);
let _enter = span.enter();
// ... classify events ...
```

## Trace Visualization

When viewing traces in Jaeger or Zipkin, you'll see a hierarchical structure like this:

```
node_tick (node_id=1, session_id=42)
├─ ingest_packets (packet_count=3)
├─ decrypt_and_validate (packet_count=3, validated_count=3, failed_count=0)
├─ classify_events (packet_count=3, event_count=5)
├─ update_time_model (event_count=5)
├─ state_reconciliation (accepted=5, rejected=0)
├─ authorize_and_sign (event_count=2)
└─ build_packets (event_count=2, packets_built=2)
```

This provides complete visibility into:
- **Timing**: How long each operation takes
- **Throughput**: How many messages/events are processed
- **Failures**: Where decryption or validation fails
- **Bottlenecks**: Which operations are slowest

## Example: Viewing Traces

See `examples/tracing_instrumentation.rs` for a complete example demonstrating the instrumented operations:

```bash
# Start Jaeger
docker run -d --name jaeger \
  -p 14268:14268 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest

# Run the example
cargo run --example tracing_instrumentation

# View traces at http://localhost:16686
# - Service: elara-tracing-demo
# - Operation: node_tick
```

## Best Practices

1. **Use appropriate log levels**:
   - `INFO`: Connection operations, node lifecycle
   - `DEBUG`: Message processing, state sync
   - `TRACE`: Detailed per-message information

2. **Add context to spans**:
   - Always include `node_id`
   - Include `session_id` when in a session
   - Add counts (packet_count, event_count) for visibility

3. **Keep spans focused**:
   - One span per logical operation
   - Nest spans for sub-operations
   - Use RAII guards (`_enter`) for automatic cleanup

4. **Monitor span duration**:
   - Set up alerts for slow operations
   - Use percentiles (p50, p95, p99) for SLOs
   - Identify bottlenecks in the trace view

5. **Correlate with metrics**:
   - Use the same `node_id` in metrics and traces
   - Cross-reference high latency in metrics with traces
   - Use traces to debug metric anomalies
