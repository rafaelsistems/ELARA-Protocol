# ELARA Protocol Monitoring Guide

**Version**: 0.2.0  
**Last Updated**: 2024-01  
**Status**: Production  
**Audience**: SREs, Operations Teams, Platform Engineers

---

## Table of Contents

1. [Overview](#overview)
2. [Monitoring Architecture](#monitoring-architecture)
3. [Key Metrics](#key-metrics)
4. [Alert Setup](#alert-setup)
5. [Dashboard Recommendations](#dashboard-recommendations)
6. [Prometheus Configuration](#prometheus-configuration)
7. [Grafana Setup](#grafana-setup)
8. [Alertmanager Configuration](#alertmanager-configuration)
9. [Troubleshooting Monitoring](#troubleshooting-monitoring)

---

## Overview

This guide provides comprehensive instructions for setting up monitoring and alerting for ELARA Protocol deployments. It covers metrics collection, alert configuration, dashboard creation, and integration with popular monitoring tools.

### Monitoring Stack

ELARA Protocol uses industry-standard observability tools:

- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards
- **Alertmanager**: Alert routing and notification
- **Jaeger/Zipkin**: Distributed tracing (optional)
- **Elasticsearch/Loki**: Log aggregation (optional)

### Monitoring Principles

1. **Four Golden Signals**: Latency, Traffic, Errors, Saturation
2. **RED Method**: Rate, Errors, Duration
3. **USE Method**: Utilization, Saturation, Errors
4. **Proactive Alerting**: Alert on symptoms, not causes
5. **Actionable Alerts**: Every alert should require action

---

## Monitoring Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     ELARA Nodes                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │  Node 1  │  │  Node 2  │  │  Node 3  │                  │
│  │  :9090   │  │  :9090   │  │  :9090   │  Metrics         │
│  │  :8080   │  │  :8080   │  │  :8080   │  Health          │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                  │
└───────┼─────────────┼─────────────┼────────────────────────┘
        │             │             │
        │ Scrape      │ Scrape      │ Scrape
        │ /metrics    │ /metrics    │ /metrics
        ▼             ▼             ▼
┌─────────────────────────────────────────────────────────────┐
│                      Prometheus                              │
│  - Scrapes metrics every 15s                                 │
│  - Evaluates alert rules                                     │
│  - Stores time series data                                   │
│  - Provides query API                                        │
└────────┬────────────────────────────────┬───────────────────┘
         │                                │
         │ PromQL Queries                 │ Alerts
         ▼                                ▼
┌──────────────────────┐        ┌──────────────────────┐
│       Grafana        │        │    Alertmanager      │
│  - Dashboards        │        │  - Alert routing     │
│  - Visualizations    │        │  - Deduplication     │
│  - Annotations       │        │  - Notifications     │
└──────────────────────┘        └──────┬───────────────┘
                                       │
                                       │ Notifications
                                       ▼
                        ┌──────────────────────────────┐
                        │  PagerDuty / Slack / Email   │
                        └──────────────────────────────┘
```

---

## Key Metrics

### Connection Metrics

#### `elara_active_connections`
- **Type**: Gauge
- **Description**: Number of currently active peer connections
- **Labels**: `node_id`
- **Normal Range**: ≥ min_connections (typically 2-5)
- **Alert Threshold**: < 2 for 5 minutes

**Query Examples**:
```promql
# Current active connections per node
elara_active_connections

# Average active connections across cluster
avg(elara_active_connections)

# Nodes with low connection count
elara_active_connections < 2
```

#### `elara_total_connections`
- **Type**: Counter
- **Description**: Total number of connection attempts since start
- **Labels**: `node_id`
- **Use**: Track connection churn rate

**Query Examples**:
```promql
# Connection rate (connections/sec)
rate(elara_total_connections[5m])

# Total connections in last hour
increase(elara_total_connections[1h])
```

#### `elara_failed_connections_total`
- **Type**: Counter
- **Description**: Total number of failed connection attempts
- **Labels**: `node_id`, `reason`
- **Alert Threshold**: rate > 0.1/sec for 5 minutes

**Query Examples**:
```promql
# Connection failure rate
rate(elara_failed_connections_total[5m])

# Connection failure percentage
rate(elara_failed_connections_total[5m]) / rate(elara_total_connections[5m]) * 100

# Failures by reason
sum by (reason) (rate(elara_failed_connections_total[5m]))
```

### Message Metrics

#### `elara_messages_sent_total`
- **Type**: Counter
- **Description**: Total number of messages sent
- **Labels**: `node_id`, `message_type`
- **Use**: Track outbound message rate

**Query Examples**:
```promql
# Message send rate (messages/sec)
rate(elara_messages_sent_total[5m])

# Total messages sent in last hour
increase(elara_messages_sent_total[1h])

# Messages by type
sum by (message_type) (rate(elara_messages_sent_total[5m]))
```

#### `elara_messages_received_total`
- **Type**: Counter
- **Description**: Total number of messages received
- **Labels**: `node_id`, `message_type`
- **Use**: Track inbound message rate

**Query Examples**:
```promql
# Message receive rate
rate(elara_messages_received_total[5m])

# Message throughput (sent + received)
rate(elara_messages_sent_total[5m]) + rate(elara_messages_received_total[5m])
```

#### `elara_messages_dropped_total`
- **Type**: Counter
- **Description**: Total number of messages dropped
- **Labels**: `node_id`, `reason`
- **Alert Threshold**: rate > 1% of sent messages for 5 minutes

**Query Examples**:
```promql
# Message drop rate
rate(elara_messages_dropped_total[5m])

# Drop percentage
rate(elara_messages_dropped_total[5m]) / rate(elara_messages_sent_total[5m]) * 100

# Drops by reason
sum by (reason) (rate(elara_messages_dropped_total[5m]))
```

#### `elara_message_size_bytes`
- **Type**: Histogram
- **Description**: Distribution of message sizes
- **Labels**: `node_id`
- **Buckets**: 64, 256, 1024, 4096, 16384, 65536

**Query Examples**:
```promql
# Average message size
rate(elara_message_size_bytes_sum[5m]) / rate(elara_message_size_bytes_count[5m])

# P95 message size
histogram_quantile(0.95, rate(elara_message_size_bytes_bucket[5m]))

# Messages by size bucket
sum by (le) (rate(elara_message_size_bytes_bucket[5m]))
```

### Latency Metrics

#### `elara_message_latency_ms`
- **Type**: Histogram
- **Description**: End-to-end message latency in milliseconds
- **Labels**: `node_id`
- **Buckets**: 1, 5, 10, 25, 50, 100, 250, 500, 1000, 5000
- **Alert Threshold**: P95 > 1000ms for 5 minutes

**Query Examples**:
```promql
# P50 latency
histogram_quantile(0.50, rate(elara_message_latency_ms_bucket[5m]))

# P95 latency
histogram_quantile(0.95, rate(elara_message_latency_ms_bucket[5m]))

# P99 latency
histogram_quantile(0.99, rate(elara_message_latency_ms_bucket[5m]))

# Average latency
rate(elara_message_latency_ms_sum[5m]) / rate(elara_message_latency_ms_count[5m])

# Latency by node
histogram_quantile(0.95, sum by (node_id, le) (rate(elara_message_latency_ms_bucket[5m])))
```

#### `elara_state_sync_latency_ms`
- **Type**: Histogram
- **Description**: State synchronization latency
- **Labels**: `node_id`
- **Use**: Monitor state reconciliation performance

**Query Examples**:
```promql
# P95 state sync latency
histogram_quantile(0.95, rate(elara_state_sync_latency_ms_bucket[5m]))

# Average state sync latency
rate(elara_state_sync_latency_ms_sum[5m]) / rate(elara_state_sync_latency_ms_count[5m])
```

### Resource Metrics

#### `elara_memory_usage_bytes`
- **Type**: Gauge
- **Description**: Current memory usage in bytes
- **Labels**: `node_id`
- **Alert Threshold**: > 1.8GB (90% of 2GB) for 5 minutes

**Query Examples**:
```promql
# Current memory usage (MB)
elara_memory_usage_bytes / 1024 / 1024

# Memory usage percentage (assuming 2GB limit)
elara_memory_usage_bytes / (2 * 1024 * 1024 * 1024) * 100

# Peak memory usage in last hour
max_over_time(elara_memory_usage_bytes[1h])
```

#### `elara_cpu_usage_percent`
- **Type**: Gauge
- **Description**: Current CPU usage percentage
- **Labels**: `node_id`
- **Alert Threshold**: > 80% for 10 minutes

**Query Examples**:
```promql
# Current CPU usage
elara_cpu_usage_percent

# Average CPU usage across cluster
avg(elara_cpu_usage_percent)

# Peak CPU usage in last hour
max_over_time(elara_cpu_usage_percent[1h])
```

### Protocol-Specific Metrics

#### `elara_time_drift_ms`
- **Type**: Gauge
- **Description**: Time drift from network consensus in milliseconds
- **Labels**: `node_id`
- **Alert Threshold**: abs(value) > 100ms for 5 minutes

**Query Examples**:
```promql
# Current time drift
elara_time_drift_ms

# Absolute time drift
abs(elara_time_drift_ms)

# Nodes with excessive drift
abs(elara_time_drift_ms) > 100
```

#### `elara_state_divergence`
- **Type**: Gauge
- **Description**: Number of pending events not yet reconciled
- **Labels**: `node_id`
- **Alert Threshold**: > 1000 for 10 minutes

**Query Examples**:
```promql
# Current state divergence
elara_state_divergence

# Maximum divergence across cluster
max(elara_state_divergence)

# Nodes with high divergence
elara_state_divergence > 1000
```

#### `elara_replay_window_size`
- **Type**: Gauge
- **Description**: Current replay window size
- **Labels**: `node_id`
- **Use**: Monitor replay protection mechanism

**Query Examples**:
```promql
# Current replay window size
elara_replay_window_size

# Average replay window size
avg(elara_replay_window_size)
```

### Health Metrics

#### `elara_health_status`
- **Type**: Gauge
- **Description**: Overall health status (1=healthy, 0.5=degraded, 0=unhealthy)
- **Labels**: `node_id`
- **Alert Threshold**: != 1 for 2 minutes

**Query Examples**:
```promql
# Current health status
elara_health_status

# Unhealthy nodes
elara_health_status < 1

# Count of unhealthy nodes
count(elara_health_status < 1)
```

#### `elara_health_check_duration_ms`
- **Type**: Histogram
- **Description**: Health check execution duration
- **Labels**: `node_id`, `check_name`
- **Use**: Monitor health check performance

**Query Examples**:
```promql
# P95 health check duration
histogram_quantile(0.95, rate(elara_health_check_duration_ms_bucket[5m]))

# Slow health checks
histogram_quantile(0.95, rate(elara_health_check_duration_ms_bucket[5m])) > 50
```

---

## Alert Setup

### Alert Philosophy

**Good Alerts**:
- Actionable: Require immediate human intervention
- Symptomatic: Alert on user-visible symptoms, not causes
- Specific: Clear what's wrong and where
- Documented: Include runbook links

**Bad Alerts**:
- Noisy: Fire frequently without requiring action
- Vague: Unclear what action to take
- Predictive: Alert on things that might happen
- Redundant: Multiple alerts for same issue

### Critical Alerts

#### HighMessageDropRate

**Severity**: Warning  
**Threshold**: Message drop rate > 1% for 5 minutes  
**Impact**: Degraded communication quality, potential data loss  
**Action**: Investigate resource saturation, network issues, or rate limiting

```yaml
- alert: HighMessageDropRate
  expr: |
    rate(elara_messages_dropped_total[5m]) / rate(elara_messages_sent_total[5m]) * 100 > 1
  for: 5m
  labels:
    severity: warning
    component: messaging
  annotations:
    summary: "High message drop rate on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} is dropping {{ $value | humanizePercentage }} of messages.
      Current drop rate: {{ $value | printf "%.2f" }}%
      
      Possible causes:
      - CPU saturation
      - Memory pressure
      - Network congestion
      - Rate limiting
      
      Runbook: https://docs.example.com/runbook#high-message-drop-rate
    dashboard: https://grafana.example.com/d/elara-overview
```

#### CriticalMessageDropRate

**Severity**: Critical  
**Threshold**: Message drop rate > 5% for 2 minutes  
**Impact**: Severe communication degradation  
**Action**: Immediate investigation and remediation

```yaml
- alert: CriticalMessageDropRate
  expr: |
    rate(elara_messages_dropped_total[2m]) / rate(elara_messages_sent_total[2m]) * 100 > 5
  for: 2m
  labels:
    severity: critical
    component: messaging
    page: "true"
  annotations:
    summary: "CRITICAL: Severe message drop rate on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} is dropping {{ $value | printf "%.2f" }}% of messages.
      This is a critical issue requiring immediate attention.
      
      Immediate actions:
      1. Check node health: curl http://{{ $labels.node_id }}:8080/health
      2. Check resource usage: top, free -h
      3. Check logs: journalctl -u elara-node -n 100
      4. Consider restarting node if unresponsive
      
      Runbook: https://docs.example.com/runbook#critical-message-drop-rate
```

#### HighLatency

**Severity**: Warning  
**Threshold**: P95 latency > 1000ms for 5 minutes  
**Impact**: Poor user experience, slow communication  
**Action**: Investigate performance bottlenecks

```yaml
- alert: HighLatency
  expr: |
    histogram_quantile(0.95, rate(elara_message_latency_ms_bucket[5m])) > 1000
  for: 5m
  labels:
    severity: warning
    component: performance
  annotations:
    summary: "High message latency on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} P95 latency is {{ $value | printf "%.0f" }}ms.
      Target: < 100ms, Warning: > 1000ms
      
      Possible causes:
      - Network latency
      - CPU saturation
      - Message queue backlog
      - Time drift
      
      Investigation steps:
      1. Check network latency: ping peer-nodes
      2. Check CPU usage: top
      3. Check message queue depth
      4. Check time drift metric
      
      Runbook: https://docs.example.com/runbook#high-latency
```

#### CriticalLatency

**Severity**: Critical  
**Threshold**: P95 latency > 5000ms for 2 minutes  
**Impact**: Severe performance degradation  
**Action**: Immediate investigation

```yaml
- alert: CriticalLatency
  expr: |
    histogram_quantile(0.95, rate(elara_message_latency_ms_bucket[2m])) > 5000
  for: 2m
  labels:
    severity: critical
    component: performance
    page: "true"
  annotations:
    summary: "CRITICAL: Severe latency on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} P95 latency is {{ $value | printf "%.0f" }}ms.
      This is critically high and requires immediate attention.
      
      Immediate actions:
      1. Check if node is responsive
      2. Check system resources
      3. Check network connectivity
      4. Consider restarting node
      
      Runbook: https://docs.example.com/runbook#critical-latency
```

#### NodeUnhealthy

**Severity**: Critical  
**Threshold**: Health status != healthy for 2 minutes  
**Impact**: Node not serving traffic  
**Action**: Investigate health check failures

```yaml
- alert: NodeUnhealthy
  expr: elara_health_status < 1
  for: 2m
  labels:
    severity: critical
    component: health
    page: "true"
  annotations:
    summary: "Node {{ $labels.node_id }} is unhealthy"
    description: |
      Node {{ $labels.node_id }} health status: {{ $value }}
      (1=healthy, 0.5=degraded, 0=unhealthy)
      
      Immediate actions:
      1. Check health endpoint: curl http://{{ $labels.node_id }}:8080/health
      2. Identify failing health check
      3. Check logs for errors
      4. Remediate based on failing check
      
      Common causes:
      - Low connection count
      - High memory usage
      - Excessive time drift
      - State divergence
      
      Runbook: https://docs.example.com/runbook#node-unhealthy
```

#### HighMemoryUsage

**Severity**: Warning  
**Threshold**: Memory usage > 90% for 5 minutes  
**Impact**: Risk of OOM, node instability  
**Action**: Investigate memory usage, consider scaling

```yaml
- alert: HighMemoryUsage
  expr: |
    elara_memory_usage_bytes / (2 * 1024 * 1024 * 1024) * 100 > 90
  for: 5m
  labels:
    severity: warning
    component: resources
  annotations:
    summary: "High memory usage on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} memory usage: {{ $value | printf "%.1f" }}%
      Current: {{ $value | humanize }}B / 2GB limit
      
      Actions:
      1. Check for memory leaks
      2. Check message buffer sizes
      3. Consider increasing memory limit
      4. Consider restarting node
      
      Runbook: https://docs.example.com/runbook#high-memory-usage
```

#### CriticalMemoryUsage

**Severity**: Critical  
**Threshold**: Memory usage > 95% for 2 minutes  
**Impact**: Imminent OOM, node crash risk  
**Action**: Immediate action required

```yaml
- alert: CriticalMemoryUsage
  expr: |
    elara_memory_usage_bytes / (2 * 1024 * 1024 * 1024) * 100 > 95
  for: 2m
  labels:
    severity: critical
    component: resources
    page: "true"
  annotations:
    summary: "CRITICAL: Memory exhaustion on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} memory usage: {{ $value | printf "%.1f" }}%
      Risk of OOM killer activation.
      
      Immediate actions:
      1. Restart node to reclaim memory
      2. Increase memory limit
      3. Investigate memory leak
      
      Runbook: https://docs.example.com/runbook#critical-memory-usage
```

#### TimeDriftExceeded

**Severity**: Warning  
**Threshold**: abs(time_drift) > 100ms for 5 minutes  
**Impact**: Protocol instability, message rejection  
**Action**: Fix time synchronization

```yaml
- alert: TimeDriftExceeded
  expr: abs(elara_time_drift_ms) > 100
  for: 5m
  labels:
    severity: warning
    component: time
  annotations:
    summary: "Time drift exceeded on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} time drift: {{ $value | printf "%.0f" }}ms
      Target: < 50ms, Warning: > 100ms
      
      Actions:
      1. Check chrony status: chronyc tracking
      2. Check NTP server connectivity
      3. Restart chrony: systemctl restart chronyd
      4. Restart node if drift persists
      
      Runbook: https://docs.example.com/runbook#time-drift
```

#### SevereTimeDrift

**Severity**: Critical  
**Threshold**: abs(time_drift) > 500ms for 2 minutes  
**Impact**: Protocol failure, message rejection  
**Action**: Immediate time sync fix

```yaml
- alert: SevereTimeDrift
  expr: abs(elara_time_drift_ms) > 500
  for: 2m
  labels:
    severity: critical
    component: time
    page: "true"
  annotations:
    summary: "CRITICAL: Severe time drift on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} time drift: {{ $value | printf "%.0f" }}ms
      This is critically high and will cause protocol failures.
      
      Immediate actions:
      1. Force time sync: chronyc makestep
      2. Restart chrony: systemctl restart chronyd
      3. Restart node: systemctl restart elara-node
      4. Check NTP infrastructure
      
      Runbook: https://docs.example.com/runbook#severe-time-drift
```

#### LowConnectionCount

**Severity**: Warning  
**Threshold**: Active connections < 2 for 5 minutes  
**Impact**: Reduced redundancy, potential isolation  
**Action**: Investigate connectivity issues

```yaml
- alert: LowConnectionCount
  expr: elara_active_connections < 2
  for: 5m
  labels:
    severity: warning
    component: connectivity
  annotations:
    summary: "Low connection count on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} has only {{ $value }} active connections.
      Minimum recommended: 2
      
      Actions:
      1. Check peer node health
      2. Check network connectivity
      3. Check firewall rules
      4. Check peer configuration
      
      Runbook: https://docs.example.com/runbook#low-connection-count
```

#### NodeIsolated

**Severity**: Critical  
**Threshold**: Active connections = 0 for 2 minutes  
**Impact**: Node completely isolated  
**Action**: Immediate connectivity restoration

```yaml
- alert: NodeIsolated
  expr: elara_active_connections == 0
  for: 2m
  labels:
    severity: critical
    component: connectivity
    page: "true"
  annotations:
    summary: "CRITICAL: Node {{ $labels.node_id }} is isolated"
    description: |
      Node {{ $labels.node_id }} has no active connections.
      Node is completely isolated from the cluster.
      
      Immediate actions:
      1. Check if node is running
      2. Check network connectivity to peers
      3. Check firewall rules
      4. Restart node if necessary
      
      Runbook: https://docs.example.com/runbook#node-isolated
```

#### HighConnectionFailureRate

**Severity**: Warning  
**Threshold**: Connection failure rate > 10% for 5 minutes  
**Impact**: Connectivity issues, reduced reliability  
**Action**: Investigate connection failures

```yaml
- alert: HighConnectionFailureRate
  expr: |
    rate(elara_failed_connections_total[5m]) / rate(elara_total_connections[5m]) * 100 > 10
  for: 5m
  labels:
    severity: warning
    component: connectivity
  annotations:
    summary: "High connection failure rate on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} connection failure rate: {{ $value | printf "%.1f" }}%
      
      Actions:
      1. Check connection failure reasons
      2. Check network connectivity
      3. Check peer node health
      4. Check authentication issues
      
      Runbook: https://docs.example.com/runbook#high-connection-failure-rate
```

#### HighStateDivergence

**Severity**: Warning  
**Threshold**: State divergence > 1000 for 10 minutes  
**Impact**: State synchronization issues  
**Action**: Investigate state reconciliation

```yaml
- alert: HighStateDivergence
  expr: elara_state_divergence > 1000
  for: 10m
  labels:
    severity: warning
    component: state
  annotations:
    summary: "High state divergence on {{ $labels.node_id }}"
    description: |
      Node {{ $labels.node_id }} has {{ $value }} pending events.
      This indicates state synchronization issues.
      
      Actions:
      1. Check peer connectivity
      2. Check network latency
      3. Check state sync latency metric
      4. Monitor for self-healing
      
      Runbook: https://docs.example.com/runbook#high-state-divergence
```

### Alert Grouping

Group related alerts to reduce noise:

```yaml
# Alertmanager configuration
route:
  group_by: ['alertname', 'node_id']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'default'
  
  routes:
  # Critical alerts go to PagerDuty
  - match:
      severity: critical
    receiver: 'pagerduty'
    group_wait: 10s
    repeat_interval: 5m
  
  # Warning alerts go to Slack
  - match:
      severity: warning
    receiver: 'slack'
    group_wait: 30s
    repeat_interval: 1h
```

---

## Dashboard Recommendations

### Overview Dashboard

**Purpose**: High-level cluster health and performance  
**Audience**: Operations team, management  
**Refresh**: 30 seconds

**Panels**:

1. **Cluster Health Status**
   - Visualization: Stat panel
   - Query: `count(elara_health_status == 1) / count(elara_health_status) * 100`
   - Thresholds: Green > 90%, Yellow > 70%, Red ≤ 70%

2. **Total Message Throughput**
   - Visualization: Graph
   - Query: `sum(rate(elara_messages_sent_total[5m])) + sum(rate(elara_messages_received_total[5m]))`
   - Unit: messages/sec

3. **P95 Latency**
   - Visualization: Graph
   - Query: `histogram_quantile(0.95, sum(rate(elara_message_latency_ms_bucket[5m])) by (le))`
   - Unit: milliseconds
   - Thresholds: Green < 100ms, Yellow < 1000ms, Red ≥ 1000ms

4. **Message Drop Rate**
   - Visualization: Graph
   - Query: `sum(rate(elara_messages_dropped_total[5m])) / sum(rate(elara_messages_sent_total[5m])) * 100`
   - Unit: percent
   - Thresholds: Green < 0.1%, Yellow < 1%, Red ≥ 1%

5. **Active Connections**
   - Visualization: Graph
   - Query: `sum(elara_active_connections)`
   - Unit: connections

6. **Memory Usage**
   - Visualization: Graph
   - Query: `elara_memory_usage_bytes / 1024 / 1024`
   - Unit: MB
   - Thresholds: Green < 1600MB, Yellow < 1800MB, Red ≥ 1800MB

7. **CPU Usage**
   - Visualization: Graph
   - Query: `avg(elara_cpu_usage_percent)`
   - Unit: percent
   - Thresholds: Green < 70%, Yellow < 80%, Red ≥ 80%

8. **Active Alerts**
   - Visualization: Alert list
   - Query: `ALERTS{alertstate="firing"}`

### Node Detail Dashboard

**Purpose**: Detailed metrics for individual nodes  
**Audience**: SREs, on-call engineers  
**Refresh**: 15 seconds

**Variables**:
- `$node_id`: Node selector (from `elara_health_status` metric)

**Panels**:

1. **Node Health Status**
   - Query: `elara_health_status{node_id="$node_id"}`
   - Visualization: Stat panel with color coding

2. **Message Rates**
   - Sent: `rate(elara_messages_sent_total{node_id="$node_id"}[5m])`
   - Received: `rate(elara_messages_received_total{node_id="$node_id"}[5m])`
   - Dropped: `rate(elara_messages_dropped_total{node_id="$node_id"}[5m])`

3. **Latency Percentiles**
   - P50: `histogram_quantile(0.50, rate(elara_message_latency_ms_bucket{node_id="$node_id"}[5m]))`
   - P95: `histogram_quantile(0.95, rate(elara_message_latency_ms_bucket{node_id="$node_id"}[5m]))`
   - P99: `histogram_quantile(0.99, rate(elara_message_latency_ms_bucket{node_id="$node_id"}[5m]))`

4. **Connection Status**
   - Active: `elara_active_connections{node_id="$node_id"}`
   - Failed: `rate(elara_failed_connections_total{node_id="$node_id"}[5m])`

5. **Resource Usage**
   - Memory: `elara_memory_usage_bytes{node_id="$node_id"} / 1024 / 1024`
   - CPU: `elara_cpu_usage_percent{node_id="$node_id"}`

6. **Protocol Metrics**
   - Time Drift: `elara_time_drift_ms{node_id="$node_id"}`
   - State Divergence: `elara_state_divergence{node_id="$node_id"}`
   - Replay Window: `elara_replay_window_size{node_id="$node_id"}`

7. **Health Check Details**
   - Query: `elara_health_check_status{node_id="$node_id"}`
   - Visualization: Table with check name, status, reason

### Performance Dashboard

**Purpose**: Deep dive into performance metrics  
**Audience**: Performance engineers, developers  
**Refresh**: 10 seconds

**Panels**:

1. **Latency Heatmap**
   - Query: `sum(rate(elara_message_latency_ms_bucket[5m])) by (le)`
   - Visualization: Heatmap

2. **Message Size Distribution**
   - Query: `sum(rate(elara_message_size_bytes_bucket[5m])) by (le)`
   - Visualization: Histogram

3. **Throughput by Message Type**
   - Query: `sum by (message_type) (rate(elara_messages_sent_total[5m]))`
   - Visualization: Stacked graph

4. **State Sync Performance**
   - Query: `histogram_quantile(0.95, rate(elara_state_sync_latency_ms_bucket[5m]))`
   - Visualization: Graph

5. **CPU Usage by Node**
   - Query: `elara_cpu_usage_percent`
   - Visualization: Graph with multiple series

6. **Memory Usage Trend**
   - Query: `elara_memory_usage_bytes / 1024 / 1024`
   - Visualization: Graph with prediction

### Capacity Planning Dashboard

**Purpose**: Long-term trends and capacity planning  
**Audience**: Capacity planners, architects  
**Refresh**: 5 minutes

**Panels**:

1. **Message Throughput Trend (7 days)**
   - Query: `sum(rate(elara_messages_sent_total[1h]))`
   - Visualization: Graph with trend line

2. **Peak Resource Usage (30 days)**
   - Memory: `max_over_time(elara_memory_usage_bytes[30d])`
   - CPU: `max_over_time(elara_cpu_usage_percent[30d])`

3. **Connection Growth**
   - Query: `sum(elara_active_connections)`
   - Visualization: Graph with 7-day and 30-day averages

4. **Storage Growth**
   - Query: `sum(elara_storage_bytes)`
   - Visualization: Graph with projection

5. **Scaling Recommendations**
   - Custom panel with capacity calculations

---

## Prometheus Configuration

### Complete Prometheus Setup

See `config/prometheus-example.yml` for a complete Prometheus configuration example. Key sections:

#### Scrape Configuration

```yaml
scrape_configs:
  # ELARA nodes - static configuration
  - job_name: 'elara-nodes'
    static_configs:
      - targets:
          - 'node-1.example.com:9090'
          - 'node-2.example.com:9090'
          - 'node-3.example.com:9090'
    scrape_interval: 15s
    scrape_timeout: 10s
    metrics_path: '/metrics'
```

#### Service Discovery

**Kubernetes**:
```yaml
- job_name: 'elara-nodes-k8s'
  kubernetes_sd_configs:
    - role: pod
      namespaces:
        names:
          - elara-production
  relabel_configs:
    - source_labels: [__meta_kubernetes_pod_label_app]
      action: keep
      regex: elara-node
```

**Consul**:
```yaml
- job_name: 'elara-nodes-consul'
  consul_sd_configs:
    - server: 'consul.example.com:8500'
      services:
        - elara-node
```

#### Recording Rules

Create recording rules for frequently used queries:

```yaml
# recording-rules.yml
groups:
  - name: elara_aggregations
    interval: 30s
    rules:
      # Cluster-wide message throughput
      - record: elara:messages_total:rate5m
        expr: sum(rate(elara_messages_sent_total[5m])) + sum(rate(elara_messages_received_total[5m]))
      
      # Cluster-wide P95 latency
      - record: elara:message_latency_ms:p95
        expr: histogram_quantile(0.95, sum(rate(elara_message_latency_ms_bucket[5m])) by (le))
      
      # Cluster-wide drop rate
      - record: elara:messages_dropped:rate5m
        expr: sum(rate(elara_messages_dropped_total[5m])) / sum(rate(elara_messages_sent_total[5m])) * 100
      
      # Per-node CPU usage average
      - record: elara:cpu_usage:avg1m
        expr: avg_over_time(elara_cpu_usage_percent[1m])
      
      # Per-node memory usage average
      - record: elara:memory_usage_mb:avg1m
        expr: avg_over_time(elara_memory_usage_bytes[1m]) / 1024 / 1024
```

---

## Grafana Setup

### Installation

```bash
# Add Grafana repository
cat <<EOF | sudo tee /etc/yum.repos.d/grafana.repo
[grafana]
name=grafana
baseurl=https://packages.grafana.com/oss/rpm
repo_gpgcheck=1
enabled=1
gpgcheck=1
gpgkey=https://packages.grafana.com/gpg.key
sslverify=1
sslcacert=/etc/pki/tls/certs/ca-bundle.crt
EOF

# Install Grafana
sudo yum install grafana

# Start Grafana
sudo systemctl start grafana-server
sudo systemctl enable grafana-server

# Access Grafana at http://localhost:3000
# Default credentials: admin/admin
```

### Data Source Configuration

```yaml
# datasources.yml
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: false
    jsonData:
      timeInterval: 15s
      queryTimeout: 60s
      httpMethod: POST
```

### Dashboard Provisioning

```yaml
# dashboards.yml
apiVersion: 1

providers:
  - name: 'ELARA Dashboards'
    orgId: 1
    folder: 'ELARA'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /var/lib/grafana/dashboards/elara
```

### Dashboard JSON Export

Export dashboards as JSON for version control:

```bash
# Export dashboard
curl -H "Authorization: Bearer $GRAFANA_API_KEY" \
  http://grafana:3000/api/dashboards/uid/elara-overview \
  | jq '.dashboard' > elara-overview.json

# Import dashboard
curl -X POST -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GRAFANA_API_KEY" \
  -d @elara-overview.json \
  http://grafana:3000/api/dashboards/db
```

---

## Alertmanager Configuration

### Installation

```bash
# Download Alertmanager
VERSION="0.26.0"
wget https://github.com/prometheus/alertmanager/releases/download/v${VERSION}/alertmanager-${VERSION}.linux-amd64.tar.gz

# Extract
tar -xzf alertmanager-${VERSION}.linux-amd64.tar.gz
cd alertmanager-${VERSION}.linux-amd64

# Install
sudo cp alertmanager /usr/local/bin/
sudo cp amtool /usr/local/bin/

# Create configuration directory
sudo mkdir -p /etc/alertmanager
```

### Configuration

See `config/alertmanager-example.yml` for a complete configuration. Key sections:

#### Global Configuration

```yaml
global:
  resolve_timeout: 5m
  smtp_smarthost: 'smtp.example.com:587'
  smtp_from: 'alertmanager@example.com'
  smtp_auth_username: 'alertmanager'
  smtp_auth_password: 'password'
  slack_api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
  pagerduty_url: 'https://events.pagerduty.com/v2/enqueue'
```

#### Route Configuration

```yaml
route:
  group_by: ['alertname', 'cluster', 'node_id']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'default'
  
  routes:
    # Critical alerts to PagerDuty
    - match:
        severity: critical
      receiver: 'pagerduty'
      group_wait: 10s
      repeat_interval: 5m
      continue: true
    
    # Critical alerts also to Slack
    - match:
        severity: critical
      receiver: 'slack-critical'
      group_wait: 10s
      repeat_interval: 5m
    
    # Warning alerts to Slack
    - match:
        severity: warning
      receiver: 'slack-warnings'
      group_wait: 30s
      repeat_interval: 1h
    
    # Maintenance window (silence all)
    - match:
        maintenance: 'true'
      receiver: 'null'
```

#### Receiver Configuration

```yaml
receivers:
  - name: 'default'
    email_configs:
      - to: 'ops-team@example.com'
  
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_SERVICE_KEY'
        description: '{{ .GroupLabels.alertname }}: {{ .CommonAnnotations.summary }}'
        details:
          firing: '{{ .Alerts.Firing | len }}'
          resolved: '{{ .Alerts.Resolved | len }}'
          details: '{{ .CommonAnnotations.description }}'
  
  - name: 'slack-critical'
    slack_configs:
      - channel: '#elara-alerts-critical'
        title: ':fire: {{ .GroupLabels.alertname }}'
        text: '{{ .CommonAnnotations.description }}'
        color: 'danger'
        send_resolved: true
  
  - name: 'slack-warnings'
    slack_configs:
      - channel: '#elara-alerts-warnings'
        title: ':warning: {{ .GroupLabels.alertname }}'
        text: '{{ .CommonAnnotations.description }}'
        color: 'warning'
        send_resolved: true
  
  - name: 'null'
```

#### Inhibition Rules

Prevent alert spam by inhibiting related alerts:

```yaml
inhibit_rules:
  # Inhibit warning if critical is firing
  - source_match:
      severity: 'critical'
    target_match:
      severity: 'warning'
    equal: ['alertname', 'node_id']
  
  # Inhibit all alerts if node is down
  - source_match:
      alertname: 'NodeDown'
    target_match_re:
      alertname: '.*'
    equal: ['node_id']
  
  # Inhibit latency alerts if drop rate is high
  - source_match:
      alertname: 'HighMessageDropRate'
    target_match:
      alertname: 'HighLatency'
    equal: ['node_id']
```

### Systemd Service

```bash
# Create systemd service
sudo tee /etc/systemd/system/alertmanager.service <<EOF
[Unit]
Description=Alertmanager
Documentation=https://prometheus.io/docs/alerting/alertmanager/
After=network-online.target

[Service]
Type=simple
User=alertmanager
Group=alertmanager
ExecStart=/usr/local/bin/alertmanager \\
  --config.file=/etc/alertmanager/alertmanager.yml \\
  --storage.path=/var/lib/alertmanager \\
  --web.listen-address=:9093 \\
  --cluster.listen-address=:9094
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
EOF

# Create alertmanager user
sudo useradd -r -s /bin/false alertmanager

# Create data directory
sudo mkdir -p /var/lib/alertmanager
sudo chown -R alertmanager:alertmanager /var/lib/alertmanager

# Start service
sudo systemctl daemon-reload
sudo systemctl start alertmanager
sudo systemctl enable alertmanager
```

### Testing Alerts

```bash
# Send test alert
amtool alert add test_alert \
  alertname=TestAlert \
  severity=warning \
  node_id=test-node \
  summary="This is a test alert"

# Check alert status
amtool alert query

# Silence alert
amtool silence add \
  alertname=TestAlert \
  --duration=1h \
  --comment="Testing silence"

# List silences
amtool silence query
```

---

## Troubleshooting Monitoring

### Issue: Prometheus not scraping targets

**Symptoms**:
- Targets show as "down" in Prometheus UI
- No metrics data for nodes
- Scrape errors in Prometheus logs

**Diagnosis**:
```bash
# Check Prometheus targets
curl http://prometheus:9090/api/v1/targets | jq '.data.activeTargets[] | select(.health=="down")'

# Test metrics endpoint manually
curl http://node-1:9090/metrics

# Check Prometheus logs
journalctl -u prometheus -n 100
```

**Resolution**:
```bash
# Verify network connectivity
nc -zv node-1 9090

# Check firewall rules
iptables -L -n | grep 9090

# Verify Prometheus configuration
promtool check config /etc/prometheus/prometheus.yml

# Reload Prometheus configuration
curl -X POST http://prometheus:9090/-/reload
```

### Issue: Alerts not firing

**Symptoms**:
- Expected alerts not appearing
- Alert rules not evaluating
- No notifications received

**Diagnosis**:
```bash
# Check alert rules
curl http://prometheus:9090/api/v1/rules | jq '.data.groups[] | select(.name=="elara_node")'

# Check alert status
curl http://prometheus:9090/api/v1/alerts | jq '.data.alerts[] | select(.labels.alertname=="HighLatency")'

# Check Alertmanager status
curl http://alertmanager:9093/api/v2/status

# Check Alertmanager logs
journalctl -u alertmanager -n 100
```

**Resolution**:
```bash
# Validate alert rules
promtool check rules /etc/prometheus/alerts.yml

# Test alert query manually
curl -G http://prometheus:9090/api/v1/query \
  --data-urlencode 'query=histogram_quantile(0.95, rate(elara_message_latency_ms_bucket[5m])) > 1000'

# Reload Prometheus rules
curl -X POST http://prometheus:9090/-/reload

# Check Alertmanager configuration
amtool check-config /etc/alertmanager/alertmanager.yml
```

### Issue: Grafana dashboards not loading data

**Symptoms**:
- Dashboards show "No data"
- Queries timeout
- Slow dashboard loading

**Diagnosis**:
```bash
# Test Prometheus data source
curl -H "Authorization: Bearer $GRAFANA_API_KEY" \
  http://grafana:3000/api/datasources/proxy/1/api/v1/query?query=up

# Check Grafana logs
journalctl -u grafana-server -n 100

# Test query directly in Prometheus
curl -G http://prometheus:9090/api/v1/query \
  --data-urlencode 'query=elara_health_status'
```

**Resolution**:
```bash
# Verify data source configuration
curl -H "Authorization: Bearer $GRAFANA_API_KEY" \
  http://grafana:3000/api/datasources

# Test connectivity from Grafana to Prometheus
docker exec grafana curl http://prometheus:9090/api/v1/query?query=up

# Increase query timeout in Grafana data source
# Edit data source, set "Query timeout" to 60s
```

### Issue: High cardinality metrics

**Symptoms**:
- Prometheus memory usage growing
- Slow query performance
- "too many samples" errors

**Diagnosis**:
```bash
# Check cardinality
curl http://prometheus:9090/api/v1/status/tsdb | jq '.data.seriesCountByMetricName'

# Check memory usage
curl http://prometheus:9090/api/v1/status/runtimeinfo | jq '.data.storageRetention'

# Identify high-cardinality metrics
promtool tsdb analyze /var/lib/prometheus/data
```

**Resolution**:
```bash
# Reduce label cardinality
# - Avoid high-cardinality labels (user IDs, timestamps)
# - Use recording rules for aggregations
# - Increase retention period or add more storage

# Add relabel config to drop high-cardinality labels
# In prometheus.yml:
relabel_configs:
  - source_labels: [__name__]
    regex: 'high_cardinality_metric.*'
    action: drop
```

---

## Best Practices

### Monitoring Best Practices

1. **Monitor the Four Golden Signals**: Latency, Traffic, Errors, Saturation
2. **Use Recording Rules**: Pre-compute expensive queries
3. **Set Appropriate Retention**: Balance storage cost vs. historical data needs
4. **Use Service Discovery**: Avoid manual target configuration
5. **Label Consistently**: Use consistent label names across metrics
6. **Avoid High Cardinality**: Don't use unbounded values as labels
7. **Test Alerts**: Regularly test alert firing and notification delivery
8. **Document Runbooks**: Every alert should have a runbook link
9. **Review Dashboards**: Regularly review and update dashboards
10. **Monitor the Monitors**: Ensure Prometheus/Grafana/Alertmanager are healthy

### Alert Best Practices

1. **Alert on Symptoms**: Alert on user-visible issues, not causes
2. **Make Alerts Actionable**: Every alert should require action
3. **Avoid Alert Fatigue**: Don't alert on things that don't require action
4. **Use Severity Levels**: Critical (page), Warning (ticket), Info (log)
5. **Group Related Alerts**: Reduce noise with proper grouping
6. **Set Appropriate Thresholds**: Balance false positives vs. false negatives
7. **Include Context**: Add runbook links, dashboard links, and descriptions
8. **Test Alert Routing**: Verify alerts reach the right people
9. **Use Inhibition Rules**: Prevent alert storms
10. **Review Alert History**: Regularly review fired alerts and adjust

### Dashboard Best Practices

1. **Start with Overview**: Create high-level overview dashboard first
2. **Drill Down**: Provide links to detailed dashboards
3. **Use Variables**: Make dashboards reusable with variables
4. **Show Trends**: Include historical data and trends
5. **Use Appropriate Visualizations**: Choose the right chart type
6. **Set Thresholds**: Use color coding for thresholds
7. **Add Annotations**: Mark deployments and incidents
8. **Keep It Simple**: Don't overcrowd dashboards
9. **Version Control**: Store dashboard JSON in git
10. **Document Panels**: Add descriptions to panels

---

## Additional Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Alertmanager Documentation](https://prometheus.io/docs/alerting/latest/alertmanager/)
- [PromQL Tutorial](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- [Configuration Guide](CONFIGURATION.md) - ELARA configuration reference
- [Deployment Guide](DEPLOYMENT.md) - Deployment procedures
- [Operational Runbook](RUNBOOK.md) - Day-to-day operations

---

**Document Version**: 1.0  
**Last Updated**: 2024  
**Maintained By**: ELARA Operations Team
