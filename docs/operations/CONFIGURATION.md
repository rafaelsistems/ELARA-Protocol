# ELARA Protocol Configuration Guide

**Version**: 0.2.0  
**Last Updated**: 2024-01  
**Status**: Production  
**Audience**: DevOps Engineers, SREs, System Administrators

---

## Table of Contents

1. [Overview](#overview)
2. [Configuration File Format](#configuration-file-format)
3. [Node Configuration](#node-configuration)
4. [Runtime Configuration](#runtime-configuration)
5. [Observability Configuration](#observability-configuration)
6. [Health Check Configuration](#health-check-configuration)
7. [Production-Recommended Settings](#production-recommended-settings)
8. [Tuning Guidelines](#tuning-guidelines)
9. [Environment Variables](#environment-variables)
10. [Configuration Validation](#configuration-validation)

---

## Overview

ELARA Protocol uses TOML configuration files for all runtime settings. This guide provides comprehensive documentation of all configuration options, production-recommended settings, and tuning guidelines for different deployment scenarios.

### Configuration Philosophy

- **Sensible Defaults**: Most settings have reasonable defaults
- **Opt-In Features**: Advanced features (observability, health checks) are opt-in
- **Environment-Specific**: Different settings for dev, staging, production
- **Validation**: Configuration is validated at startup
- **Hot Reload**: Some settings support hot reload (SIGHUP)

### Configuration Locations

| Environment | Location | Priority |
|-------------|----------|----------|
| Development | `./config.toml` | 1 (highest) |
| System | `/etc/elara/config.toml` | 2 |
| User | `~/.config/elara/config.toml` | 3 |
| Default | Built-in defaults | 4 (lowest) |

---

## Configuration File Format

ELARA uses TOML (Tom's Obvious, Minimal Language) for configuration files.

### Basic Structure

```toml
# ELARA Node Configuration
# Format: TOML v1.0.0

[node]
# Node identity and network settings

[runtime]
# Runtime behavior and performance settings

[observability]
# Logging, metrics, and tracing configuration

[health_checks]
# Health check system configuration
```

### TOML Syntax Reference

```toml
# Comments start with #

# Strings
string_value = "hello world"
multiline_string = """
This is a
multiline string
"""

# Numbers
integer_value = 42
float_value = 3.14

# Booleans
boolean_value = true

# Arrays
array_value = ["item1", "item2", "item3"]
multiline_array = [
    "item1",
    "item2",
    "item3",
]

# Tables (sections)
[table_name]
key = "value"

# Nested tables
[parent.child]
key = "value"

# Array of tables
[[array_of_tables]]
name = "first"

[[array_of_tables]]
name = "second"
```

---

## Node Configuration

### `[node]` Section

Core node identity and network settings.

#### `node_id` (required)

**Type**: String  
**Description**: Unique identifier for this node  
**Constraints**: Must be unique across the cluster  
**Example**: `"node-1"`, `"prod-us-east-1-node-01"`

```toml
[node]
node_id = "node-1"
```

**Best Practices**:
- Use descriptive names: `{environment}-{region}-{role}-{number}`
- Keep it short (< 64 characters)
- Use only alphanumeric characters and hyphens
- Don't use IP addresses or hostnames (they may change)

#### `bind_address` (required)

**Type**: String  
**Description**: Network address to bind for protocol communication  
**Format**: `{ip}:{port}` or `:{port}` (binds to all interfaces)  
**Default Port**: 7777  
**Example**: `"0.0.0.0:7777"`, `"192.168.1.10:7777"`

```toml
[node]
bind_address = "0.0.0.0:7777"
```

**Best Practices**:
- Use `0.0.0.0` to bind to all interfaces (recommended for containers)
- Use specific IP for multi-homed hosts
- Ensure port is not in use by other services
- Use non-privileged port (> 1024) to avoid running as root

#### `peers` (optional)

**Type**: Array of strings  
**Description**: List of peer node addresses to connect to  
**Format**: `["{host}:{port}", ...]`  
**Default**: `[]` (no peers)

```toml
[node]
peers = [
    "node-2.example.com:7777",
    "node-3.example.com:7777",
    "192.168.1.11:7777",
]
```

**Best Practices**:
- Use DNS names instead of IP addresses (easier to update)
- Include at least 2-3 peers for redundancy
- Don't include self in peer list
- Use internal/private network addresses when possible
- For Kubernetes, use service discovery instead of static list

**Dynamic Peer Discovery**:
```toml
# For Kubernetes, leave peers empty and use service discovery
peers = []

# Peers will be discovered via:
# - Kubernetes service: elara-node.elara-production.svc.cluster.local
# - Consul service: elara-node.service.consul
# - DNS SRV records: _elara._udp.example.com
```

---

## Runtime Configuration

### `[runtime]` Section

Runtime behavior and performance settings.

#### `tick_interval_ms` (optional)

**Type**: Integer  
**Description**: Main event loop tick interval in milliseconds  
**Default**: 100  
**Range**: 10-1000  
**Unit**: milliseconds

```toml
[runtime]
tick_interval_ms = 100
```

**Tuning Guidelines**:
- **Low Latency** (10-50ms): Higher CPU usage, lower latency
- **Balanced** (100ms): Recommended for most deployments
- **High Throughput** (200-500ms): Lower CPU usage, higher latency
- **Low Power** (500-1000ms): Minimal CPU usage, acceptable for low-traffic scenarios

**Impact**:
- Lower values: More responsive, higher CPU usage
- Higher values: Less responsive, lower CPU usage

#### `max_packet_buffer` (optional)

**Type**: Integer  
**Description**: Maximum number of incoming packets to buffer  
**Default**: 1000  
**Range**: 100-10000  
**Unit**: packets

```toml
[runtime]
max_packet_buffer = 1000
```

**Tuning Guidelines**:
- **Small Deployment** (100-500): Low memory, suitable for < 10 nodes
- **Medium Deployment** (1000): Recommended for 10-100 nodes
- **Large Deployment** (5000-10000): High throughput, > 100 nodes

**Memory Impact**:
- Each packet: ~1-2 KB
- 1000 packets: ~1-2 MB
- 10000 packets: ~10-20 MB

**Symptoms of Too Small**:
- Packet drops under load
- `elara_messages_dropped_total` increasing
- "Buffer full" warnings in logs

**Symptoms of Too Large**:
- High memory usage
- Increased latency (more buffering)

#### `max_outgoing_buffer` (optional)

**Type**: Integer  
**Description**: Maximum number of outgoing packets to buffer  
**Default**: 1000  
**Range**: 100-10000  
**Unit**: packets

```toml
[runtime]
max_outgoing_buffer = 1000
```

**Tuning Guidelines**: Same as `max_packet_buffer`

#### `max_local_events` (optional)

**Type**: Integer  
**Description**: Maximum number of local events to queue  
**Default**: 1000  
**Range**: 100-10000  
**Unit**: events

```toml
[runtime]
max_local_events = 1000
```

**Tuning Guidelines**:
- **Low Traffic** (100-500): Minimal memory usage
- **Medium Traffic** (1000): Recommended default
- **High Traffic** (5000-10000): High event generation rate

**Symptoms of Too Small**:
- Event drops
- "Event queue full" warnings
- Application-level errors

---

## Observability Configuration

### `[observability]` Section

Unified observability configuration for logging, metrics, and tracing.

#### `enabled` (optional)

**Type**: Boolean  
**Description**: Enable observability features  
**Default**: `false`

```toml
[observability]
enabled = true
```

**Note**: When `false`, all observability features are disabled (zero overhead).

### `[observability.logging]` Section

Structured logging configuration.

#### `level` (optional)

**Type**: String  
**Description**: Log level filter  
**Values**: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`  
**Default**: `"info"`

```toml
[observability.logging]
level = "info"
```

**Level Guidelines**:
- **`trace`**: Extremely verbose, development only
- **`debug`**: Verbose, troubleshooting
- **`info`**: Normal operations (recommended for production)
- **`warn`**: Warnings and errors only
- **`error`**: Errors only

**Performance Impact**:
- `trace`/`debug`: High overhead (10-20% CPU)
- `info`: Low overhead (< 1% CPU)
- `warn`/`error`: Minimal overhead (< 0.1% CPU)

#### `format` (optional)

**Type**: String  
**Description**: Log output format  
**Values**: `"json"`, `"pretty"`, `"compact"`  
**Default**: `"json"`

```toml
[observability.logging]
format = "json"
```

**Format Examples**:

**JSON** (recommended for production):
```json
{"timestamp":"2024-01-15T10:30:00Z","level":"INFO","target":"elara_runtime","fields":{"node_id":"node-1","message":"Node started"}}
```

**Pretty** (recommended for development):
```
2024-01-15T10:30:00Z  INFO elara_runtime: Node started
    node_id: node-1
```

**Compact**:
```
2024-01-15T10:30:00Z INFO elara_runtime node_id=node-1 message="Node started"
```

#### `output` (optional)

**Type**: String  
**Description**: Log output destination  
**Values**: `"stdout"`, `"stderr"`, `"file"`, `"syslog"`  
**Default**: `"stdout"`

```toml
[observability.logging]
output = "stdout"
```

**Output Options**:

**stdout** (recommended for containers):
```toml
output = "stdout"
```

**stderr**:
```toml
output = "stderr"
```

**file**:
```toml
output = "file"
file_path = "/var/log/elara/node.log"
file_rotation = "daily"  # daily, hourly, size
file_max_size_mb = 100
file_max_files = 10
```

**syslog**:
```toml
output = "syslog"
syslog_address = "localhost:514"
syslog_facility = "local0"
```

### `[observability.metrics_server]` Section

Prometheus metrics HTTP server configuration.

#### `bind_address` (optional)

**Type**: String  
**Description**: IP address to bind metrics server  
**Default**: `"0.0.0.0"`

```toml
[observability.metrics_server]
bind_address = "0.0.0.0"
```

#### `port` (optional)

**Type**: Integer  
**Description**: Port for metrics HTTP server  
**Default**: 9090  
**Range**: 1024-65535

```toml
[observability.metrics_server]
port = 9090
```

**Best Practices**:
- Use standard port 9090 for consistency
- Ensure port is not blocked by firewall
- Use `0.0.0.0` to allow Prometheus to scrape from any interface

### `[observability.tracing]` Section

Distributed tracing configuration (optional).

#### `enabled` (optional)

**Type**: Boolean  
**Description**: Enable distributed tracing  
**Default**: `false`

```toml
[observability.tracing]
enabled = true
```

#### `service_name` (optional)

**Type**: String  
**Description**: Service name for tracing  
**Default**: `"elara-node"`

```toml
[observability.tracing]
service_name = "elara-node-production"
```

#### `exporter` (optional)

**Type**: String  
**Description**: Tracing exporter type  
**Values**: `"jaeger"`, `"zipkin"`, `"otlp"`  
**Default**: `"otlp"`

```toml
[observability.tracing]
exporter = "otlp"
```

#### `otlp_endpoint` (optional)

**Type**: String  
**Description**: OTLP collector endpoint  
**Format**: `http://{host}:{port}`  
**Default**: `"http://localhost:4317"`

```toml
[observability.tracing]
exporter = "otlp"
otlp_endpoint = "http://jaeger-collector:4317"
```

#### `jaeger_endpoint` (optional)

**Type**: String  
**Description**: Jaeger agent endpoint  
**Format**: `{host}:{port}`  
**Default**: `"localhost:6831"`

```toml
[observability.tracing]
exporter = "jaeger"
jaeger_endpoint = "jaeger-agent:6831"
```

#### `zipkin_endpoint` (optional)

**Type**: String  
**Description**: Zipkin collector endpoint  
**Format**: `http://{host}:{port}/api/v2/spans`  
**Default**: `"http://localhost:9411/api/v2/spans"`

```toml
[observability.tracing]
exporter = "zipkin"
zipkin_endpoint = "http://zipkin:9411/api/v2/spans"
```

#### `sample_rate` (optional)

**Type**: Float  
**Description**: Trace sampling rate (0.0 to 1.0)  
**Default**: 1.0 (100%)  
**Range**: 0.0-1.0

```toml
[observability.tracing]
sample_rate = 0.01  # 1% sampling
```

**Sampling Guidelines**:
- **Development**: 1.0 (100%) - trace everything
- **Staging**: 0.1 (10%) - trace 10% of requests
- **Production Low Traffic**: 0.1 (10%)
- **Production High Traffic**: 0.01 (1%) or lower

**Performance Impact**:
- 100% sampling: ~5-10% overhead
- 10% sampling: ~0.5-1% overhead
- 1% sampling: ~0.05-0.1% overhead

---

## Health Check Configuration

### `[health_checks]` Section

Health check system configuration.

#### `enabled` (optional)

**Type**: Boolean  
**Description**: Enable health check system  
**Default**: `false`

```toml
[health_checks]
enabled = true
```

#### `server_bind_address` (optional)

**Type**: String  
**Description**: Address for health check HTTP server  
**Format**: `{ip}:{port}`  
**Default**: `"0.0.0.0:8080"`

```toml
[health_checks]
server_bind_address = "0.0.0.0:8080"
```

**Best Practices**:
- Use standard port 8080 for health checks
- Use `0.0.0.0` to allow load balancers to access from any interface
- Ensure port is not blocked by firewall

#### `cache_ttl_secs` (optional)

**Type**: Integer  
**Description**: Cache TTL for health check results in seconds  
**Default**: 30  
**Range**: 5-300  
**Unit**: seconds

```toml
[health_checks]
cache_ttl_secs = 30
```

**Tuning Guidelines**:
- **Frequent Checks** (5-10s): More responsive, higher overhead
- **Balanced** (30s): Recommended default
- **Infrequent Checks** (60-300s): Lower overhead, less responsive

**Impact**:
- Lower values: More accurate, higher CPU usage
- Higher values: Less accurate, lower CPU usage

#### `min_connections` (optional)

**Type**: Integer or null  
**Description**: Minimum required active connections  
**Default**: `null` (check disabled)  
**Range**: 0-1000

```toml
[health_checks]
min_connections = 2
```

**Deployment-Specific Guidelines**:
- **Single Node**: `null` (no minimum)
- **Small Cluster** (3-10 nodes): 1-2
- **Medium Cluster** (10-100 nodes): 2-5
- **Large Cluster** (100+ nodes): 5-10

**Health Status**:
- `connections >= min_connections`: Healthy
- `connections < min_connections`: Degraded
- `connections == 0`: Unhealthy

#### `max_memory_mb` (optional)

**Type**: Integer or null  
**Description**: Maximum memory usage in megabytes  
**Default**: `null` (check disabled)  
**Range**: 100-10000  
**Unit**: MB

```toml
[health_checks]
max_memory_mb = 1800
```

**Sizing Guidelines**:
- Set to 90% of memory limit
- Example: 2GB limit → 1800MB threshold
- Example: 4GB limit → 3600MB threshold

**Health Status**:
- `memory < max_memory_mb`: Healthy
- `memory >= max_memory_mb`: Unhealthy

#### `max_time_drift_ms` (optional)

**Type**: Integer or null  
**Description**: Maximum acceptable time drift in milliseconds  
**Default**: `null` (check disabled)  
**Range**: 10-1000  
**Unit**: milliseconds

```toml
[health_checks]
max_time_drift_ms = 100
```

**Drift Guidelines**:
- **Strict** (50ms): High-precision requirements
- **Normal** (100ms): Recommended default
- **Relaxed** (200-500ms): Less critical deployments

**Health Status**:
- `abs(drift) < max_time_drift_ms`: Healthy
- `abs(drift) >= max_time_drift_ms`: Degraded

#### `max_pending_events` (optional)

**Type**: Integer or null  
**Description**: Maximum pending events (state divergence)  
**Default**: `null` (check disabled)  
**Range**: 100-10000  
**Unit**: events

```toml
[health_checks]
max_pending_events = 1000
```

**Tuning Guidelines**:
- **Low Traffic** (100-500): Tight threshold
- **Medium Traffic** (1000): Recommended default
- **High Traffic** (5000-10000): Loose threshold

**Health Status**:
- `pending < max_pending_events`: Healthy
- `pending >= max_pending_events`: Degraded

---

## Production-Recommended Settings

### Small Deployment (3-10 nodes)

**Use Case**: Small teams, development, staging environments  
**Resources**: 1 CPU, 1GB RAM per node  
**Traffic**: < 100 messages/sec per node

```toml
# Small Deployment Configuration

[node]
node_id = "small-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    "small-node-2:7777",
    "small-node-3:7777",
]

[runtime]
tick_interval_ms = 100
max_packet_buffer = 500
max_outgoing_buffer = 500
max_local_events = 500

[observability]
enabled = true

[observability.logging]
level = "info"
format = "json"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = false  # Optional for small deployments

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 30
min_connections = 1
max_memory_mb = 900  # 90% of 1GB
max_time_drift_ms = 100
max_pending_events = 500
```

### Medium Deployment (10-100 nodes)

**Use Case**: Production deployments, medium-scale applications  
**Resources**: 2 CPUs, 2GB RAM per node  
**Traffic**: 100-1000 messages/sec per node

```toml
# Medium Deployment Configuration

[node]
node_id = "medium-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    "medium-node-2:7777",
    "medium-node-3:7777",
    "medium-node-4:7777",
    "medium-node-5:7777",
]

[runtime]
tick_interval_ms = 100
max_packet_buffer = 1000
max_outgoing_buffer = 1000
max_local_events = 1000

[observability]
enabled = true

[observability.logging]
level = "info"
format = "json"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = true
service_name = "elara-node-production"
exporter = "otlp"
otlp_endpoint = "http://jaeger-collector:4317"
sample_rate = 0.1  # 10% sampling

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 30
min_connections = 3
max_memory_mb = 1800  # 90% of 2GB
max_time_drift_ms = 100
max_pending_events = 1000
```

### Large Deployment (100-1000+ nodes)

**Use Case**: Large-scale production, high-traffic applications  
**Resources**: 4 CPUs, 4GB RAM per node  
**Traffic**: 1000-10000 messages/sec per node

```toml
# Large Deployment Configuration

[node]
node_id = "large-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    "large-node-2:7777",
    "large-node-3:7777",
    "large-node-4:7777",
    "large-node-5:7777",
    "large-node-6:7777",
    "large-node-7:7777",
    "large-node-8:7777",
    "large-node-9:7777",
    "large-node-10:7777",
]

[runtime]
tick_interval_ms = 50  # Lower latency
max_packet_buffer = 5000
max_outgoing_buffer = 5000
max_local_events = 5000

[observability]
enabled = true

[observability.logging]
level = "info"
format = "json"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = true
service_name = "elara-node-production"
exporter = "otlp"
otlp_endpoint = "http://jaeger-collector:4317"
sample_rate = 0.01  # 1% sampling for high traffic

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 30
min_connections = 5
max_memory_mb = 3600  # 90% of 4GB
max_time_drift_ms = 100
max_pending_events = 5000
```

### High-Performance Configuration

**Use Case**: Low-latency requirements, high-throughput  
**Resources**: 8 CPUs, 8GB RAM per node  
**Traffic**: 10000+ messages/sec per node

```toml
# High-Performance Configuration

[node]
node_id = "hp-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    # ... peer list
]

[runtime]
tick_interval_ms = 10  # Very low latency
max_packet_buffer = 10000
max_outgoing_buffer = 10000
max_local_events = 10000

[observability]
enabled = true

[observability.logging]
level = "warn"  # Reduce logging overhead
format = "compact"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = true
service_name = "elara-node-hp"
exporter = "otlp"
otlp_endpoint = "http://jaeger-collector:4317"
sample_rate = 0.001  # 0.1% sampling

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 10  # More frequent checks
min_connections = 10
max_memory_mb = 7200  # 90% of 8GB
max_time_drift_ms = 50  # Stricter time requirements
max_pending_events = 10000
```

### Low-Resource Configuration

**Use Case**: Edge devices, IoT, resource-constrained environments  
**Resources**: 1 CPU, 512MB RAM per node  
**Traffic**: < 10 messages/sec per node

```toml
# Low-Resource Configuration

[node]
node_id = "edge-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    "edge-node-2:7777",
]

[runtime]
tick_interval_ms = 500  # Reduce CPU usage
max_packet_buffer = 100
max_outgoing_buffer = 100
max_local_events = 100

[observability]
enabled = true

[observability.logging]
level = "warn"  # Minimal logging
format = "compact"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = false  # Disable tracing to save resources

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 60  # Less frequent checks
min_connections = 1
max_memory_mb = 450  # 90% of 512MB
max_time_drift_ms = 200  # Relaxed requirements
max_pending_events = 100
```

---

## Tuning Guidelines

### Performance Tuning

#### Optimizing for Latency

**Goal**: Minimize message latency (P95 < 50ms)

**Configuration Changes**:
```toml
[runtime]
tick_interval_ms = 10  # Very responsive
max_packet_buffer = 10000  # Large buffers to avoid drops
max_outgoing_buffer = 10000

[observability.logging]
level = "warn"  # Reduce logging overhead

[observability.tracing]
sample_rate = 0.001  # Minimal tracing overhead
```

**System Tuning**:
```bash
# Increase network buffer sizes
sysctl -w net.core.rmem_max=134217728
sysctl -w net.core.wmem_max=134217728
sysctl -w net.core.rmem_default=16777216
sysctl -w net.core.wmem_default=16777216

# Reduce network latency
ethtool -C eth0 rx-usecs 0 tx-usecs 0

# CPU affinity (pin to specific cores)
taskset -c 0,1 elara-node --config /etc/elara/config.toml
```

#### Optimizing for Throughput

**Goal**: Maximize message throughput (> 10000 msg/sec)

**Configuration Changes**:
```toml
[runtime]
tick_interval_ms = 100  # Balanced
max_packet_buffer = 10000  # Large buffers
max_outgoing_buffer = 10000
max_local_events = 10000

[observability.logging]
level = "warn"  # Reduce overhead

[observability.tracing]
sample_rate = 0.01  # Low sampling
```

**System Tuning**:
```bash
# Increase file descriptor limit
ulimit -n 65536

# Increase network queue length
sysctl -w net.core.netdev_max_backlog=5000

# Enable TCP fast open
sysctl -w net.ipv4.tcp_fastopen=3
```

#### Optimizing for Memory

**Goal**: Minimize memory usage (< 500MB)

**Configuration Changes**:
```toml
[runtime]
tick_interval_ms = 500  # Less frequent processing
max_packet_buffer = 100  # Small buffers
max_outgoing_buffer = 100
max_local_events = 100

[observability.logging]
level = "error"  # Minimal logging

[observability.tracing]
enabled = false  # Disable tracing
```

**System Tuning**:
```bash
# Limit memory usage via cgroups
echo "500M" > /sys/fs/cgroup/memory/elara/memory.limit_in_bytes

# Or via systemd
systemctl set-property elara-node.service MemoryMax=500M
```

#### Optimizing for CPU

**Goal**: Minimize CPU usage (< 10%)

**Configuration Changes**:
```toml
[runtime]
tick_interval_ms = 1000  # Infrequent processing
max_packet_buffer = 500
max_outgoing_buffer = 500
max_local_events = 500

[observability.logging]
level = "error"

[observability.tracing]
enabled = false

[health_checks]
cache_ttl_secs = 300  # Infrequent health checks
```

### Network Tuning

#### High-Latency Networks

**Scenario**: Nodes across WAN, high latency (> 100ms)

**Configuration Changes**:
```toml
[runtime]
tick_interval_ms = 200  # Less sensitive to latency
max_packet_buffer = 5000  # Large buffers for retransmissions

[health_checks]
max_time_drift_ms = 500  # Relaxed time requirements
```

#### Lossy Networks

**Scenario**: Packet loss > 1%

**Configuration Changes**:
```toml
[runtime]
max_packet_buffer = 10000  # Large buffers for retransmissions
max_outgoing_buffer = 10000
```

**System Tuning**:
```bash
# Increase UDP buffer sizes
sysctl -w net.core.rmem_max=268435456
sysctl -w net.core.wmem_max=268435456
```

#### Low-Bandwidth Networks

**Scenario**: Limited bandwidth (< 1 Mbps)

**Configuration Changes**:
```toml
[runtime]
tick_interval_ms = 500  # Reduce traffic
max_packet_buffer = 500
max_outgoing_buffer = 500

[observability.tracing]
enabled = false  # Reduce network traffic
```

### Observability Tuning

#### High-Volume Logging

**Scenario**: Need detailed logs for debugging

**Configuration Changes**:
```toml
[observability.logging]
level = "debug"
format = "json"
output = "file"
file_path = "/var/log/elara/node.log"
file_rotation = "hourly"
file_max_size_mb = 1000
file_max_files = 24
```

**Storage Requirements**:
- Debug level: ~100-500 MB/hour
- Info level: ~10-50 MB/hour
- Warn level: ~1-5 MB/hour

#### High-Cardinality Metrics

**Scenario**: Many unique metric labels

**Best Practices**:
- Avoid unbounded labels (user IDs, timestamps)
- Use recording rules for aggregations
- Increase Prometheus retention or storage

#### Distributed Tracing at Scale

**Scenario**: High traffic (> 10000 req/sec)

**Configuration Changes**:
```toml
[observability.tracing]
sample_rate = 0.001  # 0.1% sampling
```

**Calculation**:
- 10000 req/sec × 0.001 = 10 traces/sec
- 10 traces/sec × 3600 sec/hour = 36000 traces/hour
- Manageable for most tracing backends

---

## Environment Variables

### Supported Environment Variables

ELARA supports environment variable substitution in configuration files.

#### Syntax

```toml
# Use ${VAR_NAME} or ${VAR_NAME:default_value}
node_id = "${NODE_ID}"
bind_address = "${BIND_ADDRESS:0.0.0.0:7777}"
```

#### Common Environment Variables

```bash
# Node identity
export NODE_ID="prod-node-1"

# Network settings
export BIND_ADDRESS="0.0.0.0:7777"
export PEER_ADDRESSES="node-2:7777,node-3:7777"

# Observability
export LOG_LEVEL="info"
export METRICS_PORT="9090"
export TRACING_ENDPOINT="http://jaeger:4317"

# Health checks
export HEALTH_PORT="8080"
export MIN_CONNECTIONS="3"
export MAX_MEMORY_MB="1800"
```

#### Kubernetes ConfigMap + Environment Variables

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: elara-config
data:
  config.toml: |
    [node]
    node_id = "${POD_NAME}"
    bind_address = "0.0.0.0:7777"
    
    [observability.logging]
    level = "${LOG_LEVEL:info}"
---
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: elara-node
    env:
    - name: POD_NAME
      valueFrom:
        fieldRef:
          fieldPath: metadata.name
    - name: LOG_LEVEL
      value: "info"
```

---

## Configuration Validation

### Validation at Startup

ELARA validates configuration at startup and exits with an error if invalid.

```bash
# Validate configuration
elara-node --config /etc/elara/config.toml --validate

# Expected output: "Configuration is valid"
```

### Common Validation Errors

#### Missing Required Fields

```
Error: Missing required field 'node_id' in [node] section
```

**Fix**: Add `node_id` to `[node]` section

#### Invalid Values

```
Error: Invalid value for 'tick_interval_ms': must be between 10 and 1000
```

**Fix**: Set `tick_interval_ms` to a value in the valid range

#### Port Conflicts

```
Error: Port 9090 is already in use
```

**Fix**: Change port or stop conflicting service

#### TOML Syntax Errors

```
Error: TOML parse error at line 15: expected '=' after key
```

**Fix**: Check TOML syntax at specified line

### Validation Checklist

Before deploying:

- [ ] Configuration file exists and is readable
- [ ] `node_id` is unique across cluster
- [ ] `bind_address` port is available
- [ ] Peer addresses are reachable
- [ ] All required ports are open in firewall
- [ ] Time synchronization is configured
- [ ] Resource limits are appropriate
- [ ] Observability endpoints are accessible
- [ ] Health check thresholds are reasonable
- [ ] Configuration passes validation: `elara-node --validate`

---

## Additional Resources

- [Deployment Guide](DEPLOYMENT.md) - Deployment procedures
- [Monitoring Guide](MONITORING.md) - Monitoring and alerting
- [Operational Runbook](RUNBOOK.md) - Day-to-day operations
- [Performance Guide](../performance/PERFORMANCE_GUIDE.md) - Performance tuning
- [Architecture Documentation](../architecture/COMPREHENSIVE_ARCHITECTURE.md) - System architecture

---

**Document Version**: 1.0  
**Last Updated**: 2024  
**Maintained By**: ELARA Operations Team
