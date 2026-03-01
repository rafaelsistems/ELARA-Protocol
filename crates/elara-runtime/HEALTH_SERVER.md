# Health Check HTTP Server

Production-grade health check HTTP server for ELARA Runtime with support for Kubernetes liveness and readiness probes.

## Overview

The health check HTTP server exposes three endpoints for monitoring node health:

- **`/health`** - Overall health status (all checks)
- **`/ready`** - Readiness probe (is the node ready to accept traffic?)
- **`/live`** - Liveness probe (is the node alive and not deadlocked?)

## Features

- ✅ **Production-Ready**: Proper error handling, logging, and graceful shutdown
- ✅ **Non-Blocking**: Uses async/await and Tokio runtime
- ✅ **Fast**: Leverages cached health check results (no expensive checks on request)
- ✅ **Kubernetes Integration**: Native support for liveness and readiness probes
- ✅ **JSON Responses**: Structured JSON responses with detailed status information
- ✅ **Proper HTTP Status Codes**: 200 OK for healthy/degraded, 503 for unhealthy
- ✅ **Comprehensive Testing**: Unit tests and integration tests included

## Quick Start

```rust
use elara_runtime::health::{HealthChecker, MemoryHealthCheck};
use elara_runtime::health_server::{HealthServer, HealthServerConfig};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create health checker
    let mut checker = HealthChecker::new(Duration::from_secs(30));
    checker.add_check(Box::new(MemoryHealthCheck::new(1800)));
    let checker = Arc::new(checker);

    // Configure and start health server
    let config = HealthServerConfig {
        bind_address: "0.0.0.0:8080".parse()?,
    };

    let server = HealthServer::new(checker, config);
    server.serve().await?;

    Ok(())
}
```

## Endpoints

### `/health` - Overall Health Status

Returns the overall health status of the node, including all registered health checks.

**Response Codes:**
- `200 OK` - Node is Healthy or Degraded (can serve traffic)
- `503 Service Unavailable` - Node is Unhealthy (should not serve traffic)

**Example Request:**
```bash
curl http://localhost:8080/health | jq
```

**Example Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": {
    "connections": {
      "status": "healthy"
    },
    "memory": {
      "status": "healthy"
    },
    "time_drift": {
      "status": "healthy"
    },
    "state_convergence": {
      "status": "healthy"
    }
  }
}
```

**Degraded Example:**
```json
{
  "status": "degraded",
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": {
    "connections": {
      "status": "degraded",
      "reason": "Only 2 active connections (minimum: 3)"
    },
    "memory": {
      "status": "healthy"
    }
  }
}
```

**Unhealthy Example:**
```json
{
  "status": "unhealthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": {
    "memory": {
      "status": "unhealthy",
      "reason": "Memory usage 1850MB exceeds limit 1800MB"
    }
  }
}
```

### `/ready` - Readiness Probe

Kubernetes readiness probe endpoint. Indicates whether the node is ready to accept traffic.

**Response Codes:**
- `200 OK` - Node is ready to accept traffic
- `503 Service Unavailable` - Node is not ready

**Use Case:** A node may be alive but not ready (e.g., still initializing, warming up caches, establishing connections). Kubernetes will not route traffic to pods that fail the readiness probe.

**Example Request:**
```bash
curl http://localhost:8080/ready
```

### `/live` - Liveness Probe

Kubernetes liveness probe endpoint. Indicates whether the node is alive and functioning.

**Response Codes:**
- `200 OK` - Node is alive
- `503 Service Unavailable` - Node is deadlocked or unresponsive

**Use Case:** If this check fails, Kubernetes will restart the pod. Liveness checks should be more lenient than readiness checks to avoid unnecessary restarts. A degraded node is still considered "alive".

**Example Request:**
```bash
curl http://localhost:8080/live
```

## Kubernetes Integration

### Example Deployment Configuration

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

### Probe Configuration Guidelines

**Liveness Probe:**
- `initialDelaySeconds: 30` - Give the node time to start up
- `periodSeconds: 10` - Check every 10 seconds
- `timeoutSeconds: 5` - Allow 5 seconds for response
- `failureThreshold: 3` - Restart after 3 consecutive failures (30 seconds)

**Readiness Probe:**
- `initialDelaySeconds: 10` - Start checking readiness early
- `periodSeconds: 5` - Check more frequently
- `timeoutSeconds: 3` - Shorter timeout for readiness
- `failureThreshold: 2` - Remove from service after 2 failures (10 seconds)

## Built-in Health Checks

The health server works with the following built-in health checks:

### 1. ConnectionHealthCheck

Monitors the number of active connections to ensure the node is properly connected to the network.

```rust
use elara_runtime::health::ConnectionHealthCheck;

checker.add_check(Box::new(ConnectionHealthCheck::new(node.clone(), 3)));
```

**Status:**
- `Healthy`: Active connections >= min_connections
- `Degraded`: Active connections < min_connections

### 2. MemoryHealthCheck

Monitors process memory usage using the `sysinfo` crate.

```rust
use elara_runtime::health::MemoryHealthCheck;

checker.add_check(Box::new(MemoryHealthCheck::new(1800))); // 1800 MB
```

**Status:**
- `Healthy`: Memory usage < max_memory_mb
- `Unhealthy`: Memory usage >= max_memory_mb

### 3. TimeDriftCheck

Monitors time drift between the local node and network consensus time.

```rust
use elara_runtime::health::TimeDriftCheck;

checker.add_check(Box::new(TimeDriftCheck::new(node.clone(), 100))); // 100ms
```

**Status:**
- `Healthy`: |time_drift| < max_drift_ms
- `Degraded`: |time_drift| >= max_drift_ms

### 4. StateDivergenceCheck

Monitors the state reconciliation engine to ensure state is converging properly.

```rust
use elara_runtime::health::StateDivergenceCheck;

checker.add_check(Box::new(StateDivergenceCheck::new(node)));
```

**Status:**
- `Healthy`: State is converging normally
- `Degraded`: State convergence is slow or stalled

## Configuration

### Server Configuration

```rust
use elara_runtime::health_server::HealthServerConfig;

let config = HealthServerConfig {
    bind_address: "0.0.0.0:8080".parse()?,
};
```

### Health Checker Configuration

```rust
use elara_runtime::health::HealthChecker;
use std::time::Duration;

// Cache health check results for 30 seconds
let checker = HealthChecker::new(Duration::from_secs(30));
```

**Cache TTL Guidelines:**
- **High-frequency polling (< 1s)**: Use 5-10 second cache
- **Normal polling (5-10s)**: Use 30 second cache
- **Low-frequency polling (> 30s)**: Use 60 second cache

## Performance Considerations

### Response Time

The health server is designed to respond quickly:

- **Cached responses**: < 1ms (read lock only)
- **Cache miss**: < 50ms (depends on health checks)
- **Target**: < 10ms for 99th percentile

### Caching

Health check results are cached to avoid excessive checking overhead:

1. First request executes all health checks
2. Subsequent requests within TTL return cached results
3. Expired cache triggers new health check execution
4. Cache updates are atomic and thread-safe

### Resource Usage

- **Memory**: < 1MB for health server
- **CPU**: < 0.1% when idle
- **Network**: Minimal (HTTP responses are small)

## Monitoring and Alerting

### Prometheus Integration

The health endpoints can be monitored by Prometheus using the `blackbox_exporter`:

```yaml
scrape_configs:
  - job_name: 'elara-health'
    metrics_path: /probe
    params:
      module: [http_2xx]
    static_configs:
      - targets:
        - http://elara-node:8080/health
    relabel_configs:
      - source_labels: [__address__]
        target_label: __param_target
      - source_labels: [__param_target]
        target_label: instance
      - target_label: __address__
        replacement: blackbox-exporter:9115
```

### Alert Rules

Example Prometheus alert rules:

```yaml
groups:
  - name: elara_health
    rules:
      - alert: ElaraNodeUnhealthy
        expr: probe_success{job="elara-health"} == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "ELARA node {{ $labels.instance }} is unhealthy"
          description: "Health check has been failing for 2 minutes"
      
      - alert: ElaraNodeDegraded
        expr: probe_http_status_code{job="elara-health"} == 200 and probe_http_content_length{job="elara-health"} > 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "ELARA node {{ $labels.instance }} is degraded"
          description: "Node is operational but degraded for 5 minutes"
```

## Load Balancer Integration

### HAProxy Configuration

```haproxy
backend elara_nodes
    option httpchk GET /health
    http-check expect status 200
    server node1 10.0.1.10:8080 check inter 5s fall 3 rise 2
    server node2 10.0.1.11:8080 check inter 5s fall 3 rise 2
    server node3 10.0.1.12:8080 check inter 5s fall 3 rise 2
```

### NGINX Configuration

```nginx
upstream elara_nodes {
    server 10.0.1.10:8080 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:8080 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:8080 max_fails=3 fail_timeout=30s;
}

server {
    location /health {
        proxy_pass http://elara_nodes/health;
        proxy_connect_timeout 3s;
        proxy_read_timeout 5s;
    }
}
```

## Troubleshooting

### Health Check Always Returns Degraded

**Symptom:** `/health` returns 200 OK but status is "degraded"

**Possible Causes:**
1. Connection count below minimum threshold
2. Time drift exceeds threshold
3. State convergence is slow

**Solution:**
- Check individual check results in the JSON response
- Adjust thresholds if they're too strict
- Investigate underlying issues (network, time sync, etc.)

### Health Check Returns 503

**Symptom:** `/health` returns 503 Service Unavailable

**Possible Causes:**
1. Memory usage exceeds threshold
2. Critical component failure

**Solution:**
- Check the `reason` field in the JSON response
- Investigate memory leaks or resource exhaustion
- Review logs for errors

### Health Check Timeout

**Symptom:** Health check requests timeout

**Possible Causes:**
1. Server not running
2. Network issues
3. Server deadlocked

**Solution:**
- Verify server is running: `netstat -an | grep 8080`
- Check server logs for errors
- Restart the node if deadlocked

## Examples

### Running the Example

```bash
cargo run --example health_server
```

### Testing with curl

```bash
# Check overall health
curl http://localhost:8080/health | jq

# Check readiness
curl http://localhost:8080/ready | jq

# Check liveness
curl http://localhost:8080/live | jq

# Check HTTP status code
curl -I http://localhost:8080/health
```

### Testing with httpie

```bash
# Pretty-printed JSON
http :8080/health

# Show headers
http -h :8080/health
```

## Best Practices

1. **Set Appropriate Thresholds**: Configure health check thresholds based on your deployment size and requirements

2. **Use Caching**: Set cache TTL to match your polling frequency to avoid excessive checking

3. **Monitor Health Endpoints**: Use Prometheus or similar tools to monitor health check status over time

4. **Separate Liveness and Readiness**: In production, consider implementing separate checks for liveness and readiness

5. **Graceful Degradation**: Design health checks to return "degraded" for non-critical issues rather than "unhealthy"

6. **Log Health Changes**: The server automatically logs when health status changes to degraded or unhealthy

7. **Test Failure Scenarios**: Regularly test that health checks correctly detect failures

8. **Document Thresholds**: Document why specific thresholds were chosen for your deployment

## Security Considerations

1. **No Authentication**: The health endpoints do not require authentication by default. Consider adding authentication if exposing publicly.

2. **Rate Limiting**: Consider adding rate limiting to prevent abuse of health endpoints.

3. **No Sensitive Data**: Health check responses do not include sensitive data (keys, passwords, etc.).

4. **Internal Network**: Deploy health endpoints on an internal network or use firewall rules to restrict access.

## References

- [Kubernetes Liveness and Readiness Probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/)
- [Health Check API Design](https://microservices.io/patterns/observability/health-check-api.html)
- [Prometheus Blackbox Exporter](https://github.com/prometheus/blackbox_exporter)
