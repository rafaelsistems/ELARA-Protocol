# ELARA Runtime Examples

This directory contains examples demonstrating various features of the ELARA runtime.

## Health Check Examples

### health_checks.rs

Demonstrates the built-in health check system with all four production-grade health checks:
- **ConnectionHealthCheck**: Monitors active connection count
- **MemoryHealthCheck**: Monitors memory usage with real system metrics (via sysinfo)
- **TimeDriftCheck**: Monitors time drift between local and network time
- **StateDivergenceCheck**: Monitors state convergence status

**Features demonstrated:**
- Creating and configuring built-in health checks
- Aggregating multiple health checks with HealthChecker
- Cache behavior with configurable TTL
- Overall health status determination (Healthy/Degraded/Unhealthy)
- Individual check results and reasons

**Running the example:**

```bash
cargo run --example health_checks
```

### health_check_config.rs

Demonstrates health check configuration through NodeConfig, showing how to integrate health checks into node initialization with production-grade configuration options.

**Features demonstrated:**
- Configuring health checks via NodeConfig
- Using preset configurations for different deployment sizes (small/medium/large)
- Custom configuration with specific thresholds
- Selective health check enablement
- HTTP server for Kubernetes probes and load balancers
- Configuration validation
- Programmatic health checking without HTTP server

**Running the example:**

```bash
cargo run --example health_check_config
```

The example will:
1. Demonstrate medium deployment preset configuration
2. Show custom threshold configuration
3. Demonstrate selective health checks (only memory check)
4. Compare all three deployment size presets
5. Show disabled health checks
6. Validate configurations

**HTTP endpoints:**

When the HTTP server is enabled, the following endpoints are exposed:

- `GET /health` - Overall health status (200 OK if healthy/degraded, 503 if unhealthy)
- `GET /ready` - Readiness probe for Kubernetes (200 OK if ready)
- `GET /live` - Liveness probe for Kubernetes (200 OK if alive)

**Testing the endpoints:**

While the example is running (it runs for 10 seconds):

```bash
# Check overall health
curl http://localhost:8080/health

# Check readiness (Kubernetes readiness probe)
curl http://localhost:8080/ready

# Check liveness (Kubernetes liveness probe)
curl http://localhost:8080/live
```

**Deployment size presets:**

The example demonstrates three preset configurations optimized for different deployment sizes:

**Small Deployment (10 nodes):**
```rust
let config = NodeConfig {
    health_checks: Some(HealthCheckConfig::small_deployment()),
    ..Default::default()
};
```
- Min connections: 2
- Max memory: 1000 MB
- Max time drift: 100 ms
- Max pending events: 500

**Medium Deployment (100 nodes):**
```rust
let config = NodeConfig {
    health_checks: Some(HealthCheckConfig::medium_deployment()),
    ..Default::default()
};
```
- Min connections: 5
- Max memory: 2000 MB
- Max time drift: 100 ms
- Max pending events: 1000

**Large Deployment (1000 nodes):**
```rust
let config = NodeConfig {
    health_checks: Some(HealthCheckConfig::large_deployment()),
    ..Default::default()
};
```
- Min connections: 10
- Max memory: 4000 MB
- Max time drift: 100 ms
- Max pending events: 2000

**Custom configuration:**

For custom requirements, configure thresholds explicitly:

```rust
let config = NodeConfig {
    health_checks: Some(HealthCheckConfig {
        enabled: true,
        server_bind_address: Some("127.0.0.1:9090".parse().unwrap()),
        cache_ttl: Duration::from_secs(15),
        min_connections: Some(5),
        max_memory_mb: Some(2500),
        max_time_drift_ms: Some(50),
        max_pending_events: Some(1500),
    }),
    ..Default::default()
};
```

**Selective health checks:**

Enable only specific checks by setting others to `None`:

```rust
let config = NodeConfig {
    health_checks: Some(HealthCheckConfig {
        enabled: true,
        server_bind_address: None, // No HTTP server
        cache_ttl: Duration::from_secs(30),
        min_connections: None, // Disabled
        max_memory_mb: Some(2000), // Only memory check
        max_time_drift_ms: None, // Disabled
        max_pending_events: None, // Disabled
    }),
    ..Default::default()
};
```

**Kubernetes integration:**

For Kubernetes deployments, configure liveness and readiness probes:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: elara-node
spec:
  containers:
  - name: elara
    image: elara-node:latest
    ports:
    - containerPort: 8080
      name: health
    livenessProbe:
      httpGet:
        path: /live
        port: health
      initialDelaySeconds: 30
      periodSeconds: 10
      timeoutSeconds: 5
      failureThreshold: 3
    readinessProbe:
      httpGet:
        path: /ready
        port: health
      initialDelaySeconds: 10
      periodSeconds: 5
      timeoutSeconds: 3
      failureThreshold: 2
```

**Production recommendations:**

1. **Choose appropriate preset**: Start with a preset matching your deployment size
2. **Adjust thresholds**: Fine-tune based on observed behavior and requirements
3. **Set memory limit**: Set max_memory_mb to 80-90% of container memory limit
4. **Enable HTTP server**: Required for Kubernetes probes and load balancer health checks
5. **Configure cache TTL**: Balance between freshness and overhead (15-30 seconds typical)
6. **Monitor health status**: Set up alerts based on health check results
7. **Test failure scenarios**: Verify health checks detect actual problems

**Disabling health checks:**

For development or when health checks are not needed:

```rust
let config = NodeConfig {
    health_checks: None, // Completely disabled
    ..Default::default()
};
```

This ensures zero overhead from health checking.

## Logging Examples

### logging_with_env_filter.rs

Demonstrates the structured logging system with per-module log levels and contextual fields.

**Features demonstrated:**
- Per-module log level configuration via `RUST_LOG` environment variable
- Contextual fields (node_id, session_id, peer_id) in log entries
- Multiple output formats (JSON, Pretty, Compact)
- Different log levels (Trace, Debug, Info, Warn, Error)

**Running the example:**

```bash
# Basic usage with default settings
cargo run --example logging_with_env_filter

# Enable debug logging for all modules
RUST_LOG=debug cargo run --example logging_with_env_filter

# Per-module log levels
RUST_LOG=info,elara_runtime=debug cargo run --example logging_with_env_filter

# JSON output format
cargo run --example logging_with_env_filter -- --format json

# Combine RUST_LOG with JSON format
RUST_LOG=trace cargo run --example logging_with_env_filter -- --format json
```

**RUST_LOG syntax examples:**

```bash
# Set all modules to info level
RUST_LOG=info

# Set default to info, but enable debug for specific module
RUST_LOG=info,elara_wire=debug

# Multiple module overrides
RUST_LOG=info,elara_wire=debug,elara_crypto=trace,elara_state=warn

# Enable trace for everything
RUST_LOG=trace

# Target specific submodules
RUST_LOG=info,elara_runtime::observability=debug
```

**Output formats:**

- `--format pretty` (default): Human-readable format with colors and indentation
- `--format json`: Machine-readable JSON format for log aggregation systems
- `--format compact`: Compact format for high-throughput scenarios

**Contextual fields:**

The example demonstrates how to add contextual information to log entries:

```rust
use tracing::info;

info!(
    node_id = "node-1",
    session_id = "session-xyz",
    peer_id = "peer-abc",
    "Connection established"
);
```

This produces structured logs where the fields can be queried and filtered by log aggregation systems.

## Metrics Examples

### metrics_server.rs

Demonstrates the Prometheus metrics server with HTTP endpoint for metrics scraping.

**Features demonstrated:**
- Creating and configuring a metrics registry
- Registering standard ELARA node metrics
- Starting an HTTP server for Prometheus scraping
- Updating metrics in real-time
- Prometheus text exposition format

**Running the example:**

```bash
# Start the metrics server
cargo run --example metrics_server
```

The server will start on `http://127.0.0.1:9090` and expose metrics at the `/metrics` endpoint.

**Accessing metrics:**

While the example is running, you can access the metrics in several ways:

```bash
# Using curl
curl http://127.0.0.1:9090/metrics

# Using wget
wget -qO- http://127.0.0.1:9090/metrics

# Using a web browser
# Navigate to: http://127.0.0.1:9090/metrics
```

**Prometheus configuration:**

To scrape these metrics with Prometheus, add this to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'elara-node'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

**Available metrics:**

The example exposes the following standard ELARA metrics:

**Connection metrics:**
- `elara_active_connections` (gauge): Number of currently active connections
- `elara_total_connections` (counter): Total connections established since start
- `elara_failed_connections` (counter): Total failed connection attempts

**Message metrics:**
- `elara_messages_sent` (counter): Total messages sent
- `elara_messages_received` (counter): Total messages received
- `elara_messages_dropped` (counter): Total messages dropped
- `elara_message_size_bytes` (histogram): Distribution of message sizes

**Latency metrics:**
- `elara_message_latency_ms` (histogram): Message processing latency distribution
- `elara_state_sync_latency_ms` (histogram): State synchronization latency distribution

**Resource metrics:**
- `elara_memory_usage_bytes` (gauge): Current memory usage
- `elara_cpu_usage_percent` (gauge): Current CPU usage percentage

**Protocol metrics:**
- `elara_time_drift_ms` (gauge): Time drift from reference time
- `elara_state_divergence_count` (gauge): Number of state divergences detected

**Example output:**

```
# HELP elara_active_connections Gauge metric
# TYPE elara_active_connections gauge
elara_active_connections 5

# HELP elara_messages_sent Counter metric
# TYPE elara_messages_sent counter
elara_messages_sent 1250

# HELP elara_message_latency_ms Histogram metric
# TYPE elara_message_latency_ms histogram
elara_message_latency_ms_bucket{le="1.0"} 10
elara_message_latency_ms_bucket{le="5.0"} 45
elara_message_latency_ms_bucket{le="10.0"} 80
elara_message_latency_ms_bucket{le="+Inf"} 100
elara_message_latency_ms_sum 425.5
elara_message_latency_ms_count 100
```


## Tracing Examples

### tracing_instrumentation.rs

Demonstrates automatic distributed tracing instrumentation of ELARA node operations.

**Features demonstrated:**
- Automatic span creation for all node operations
- Hierarchical trace structure (parent/child spans)
- Span attributes (node_id, session_id, packet_count, etc.)
- End-to-end visibility into message flow
- Integration with Jaeger for trace visualization

**Running the example:**

First, start a Jaeger instance:

```bash
docker run -d --name jaeger \
  -p 6831:6831/udp \
  -p 14268:14268 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

Then run the example:

```bash
cargo run --example tracing_instrumentation
```

**Viewing traces:**

Open your browser to http://localhost:16686 to view the Jaeger UI.

1. Select service: `elara-tracing-demo`
2. Select operation: `node_tick`
3. Click "Find Traces"

**Trace hierarchy:**

The example creates traces with the following structure:

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

**Instrumented operations:**

The following operations are automatically instrumented:

- **Connection operations**: `join_session`, `leave_session`
- **Message operations**: `ingest_packets`, `build_packets`
- **Crypto operations**: `decrypt_and_validate`, `authorize_and_sign`
- **State operations**: `classify_events`, `state_reconciliation`
- **Time operations**: `update_time_model`

**Span attributes:**

Each span includes relevant attributes:

- `node_id`: The ID of the node performing the operation
- `session_id`: The session ID (if in a session)
- `packet_count`: Number of packets processed
- `event_count`: Number of events processed
- `validated_count`: Number of successfully validated packets
- `failed_count`: Number of failed validations
- `accepted`: Number of events accepted by state engine
- `rejected`: Number of events rejected by state engine
- `packets_built`: Number of packets successfully built

**Use cases:**

1. **Performance analysis**: Identify slow operations and bottlenecks
2. **Debugging**: Trace message flow across nodes
3. **Monitoring**: Set up alerts for high latency operations
4. **Optimization**: Find operations that can be parallelized or optimized

**Stopping Jaeger:**

```bash
docker stop jaeger
docker rm jaeger
```


## Unified Observability Examples

### unified_observability.rs

Demonstrates the unified observability system that initializes all observability components (logging, tracing, metrics) with a single function call.

**Features demonstrated:**
- Single-call initialization of all observability components
- Structured logging with JSON format
- Distributed tracing with OpenTelemetry
- Metrics server with Prometheus endpoint
- Graceful shutdown of all components
- Production-grade configuration

**Running the example:**

```bash
cargo run --example unified_observability
```

**What it does:**

1. Initializes all observability components with `init_observability()`
2. Starts a metrics server on port 9090
3. Emits structured logs in JSON format
4. Registers and updates example metrics
5. Gracefully shuts down all components

**Testing the metrics endpoint:**

While the example is running:

```bash
curl http://localhost:9090/metrics
```

**Configuration options:**

The example demonstrates a full configuration:

```rust
let config = ObservabilityConfig {
    // Structured logging
    logging: Some(LoggingConfig {
        level: LogLevel::Info,
        format: LogFormat::Json,
        output: LogOutput::Stdout,
    }),
    
    // Distributed tracing
    tracing: Some(TracingConfig {
        service_name: "elara-node".to_string(),
        exporter: TracingExporter::Otlp {
            endpoint: "http://localhost:4317".to_string(),
        },
        sampling_rate: 0.1, // Sample 10% of traces
        resource_attributes: vec![
            ("environment".to_string(), "production".to_string()),
        ],
    }),
    
    // Metrics server
    metrics_server: Some(MetricsServerConfig {
        bind_address: "0.0.0.0".to_string(),
        port: 9090,
    }),
};

let handle = init_observability(config).await?;
```

**Production recommendations:**

1. **Logging**: Use JSON format for log aggregation systems
2. **Tracing**: Set sampling rate to 0.1 (10%) or lower for high-traffic systems
3. **Metrics**: Bind to `0.0.0.0` to allow external scraping
4. **Resource attributes**: Add environment, version, region for filtering
5. **Graceful shutdown**: Always call `handle.shutdown().await?` before exit

**Accessing metrics:**

The metrics registry is always available through the handle:

```rust
let counter = handle.metrics_registry()
    .register_counter("my_counter", vec![]);
counter.inc();
```

### observability_minimal.rs

Demonstrates a minimal observability setup with only logging enabled. This is useful for development or when you only need basic logging without the overhead of tracing and metrics.

**Features demonstrated:**
- Minimal configuration (logging only)
- Pretty format for human-readable output
- Per-module log levels via RUST_LOG
- Structured logging with fields
- Low overhead for development

**Running the example:**

```bash
# Basic usage
cargo run --example observability_minimal

# With per-module log levels
RUST_LOG=debug,elara_wire=trace cargo run --example observability_minimal
```

**Configuration:**

```rust
let config = ObservabilityConfig {
    logging: Some(LoggingConfig {
        level: LogLevel::Debug,
        format: LogFormat::Pretty,
        output: LogOutput::Stdout,
    }),
    tracing: None,        // Disabled
    metrics_server: None, // Disabled
};

let handle = init_observability(config).await?;
```

**When to use:**

- Development and debugging
- Local testing without external dependencies
- Minimal overhead scenarios
- When you only need logs, not metrics or traces

**Advantages:**

- No external dependencies (Jaeger, Prometheus, etc.)
- Lower resource usage
- Simpler setup
- Faster startup time

## Observability Best Practices

### Development

For local development, use minimal configuration:

```rust
ObservabilityConfig {
    logging: Some(LoggingConfig {
        level: LogLevel::Debug,
        format: LogFormat::Pretty,
        output: LogOutput::Stdout,
    }),
    tracing: None,
    metrics_server: None,
}
```

### Production

For production deployments, enable all components:

```rust
ObservabilityConfig {
    logging: Some(LoggingConfig {
        level: LogLevel::Info,
        format: LogFormat::Json,
        output: LogOutput::Stdout,
    }),
    tracing: Some(TracingConfig {
        service_name: "elara-node".to_string(),
        exporter: TracingExporter::Otlp {
            endpoint: "http://otel-collector:4317".to_string(),
        },
        sampling_rate: 0.1, // 10% sampling
        resource_attributes: vec![
            ("environment".to_string(), "production".to_string()),
            ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
            ("region".to_string(), "us-west-2".to_string()),
        ],
    }),
    metrics_server: Some(MetricsServerConfig {
        bind_address: "0.0.0.0".to_string(),
        port: 9090,
    }),
}
```

### Kubernetes

For Kubernetes deployments:

1. **Logging**: Use JSON format and let Kubernetes collect stdout
2. **Tracing**: Point to OpenTelemetry Collector service
3. **Metrics**: Expose on port 9090 for Prometheus scraping
4. **Health checks**: Use separate health check endpoints (not covered in these examples)

Example Kubernetes service for metrics:

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

Example Prometheus ServiceMonitor:

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
```

### Graceful Shutdown

Always shut down observability gracefully to ensure all telemetry is flushed:

```rust
// At application shutdown
handle.shutdown().await?;
```

This ensures:
- All pending log entries are written
- All pending trace spans are exported
- Metrics server stops accepting requests
- Resources are cleaned up properly
