# ELARA Protocol Prometheus Alerting Guide

This document provides comprehensive guidance on using the Prometheus alerting rules defined in `alerts.yml` for monitoring ELARA Protocol nodes in production.

## Table of Contents

1. [Overview](#overview)
2. [Alert Categories](#alert-categories)
3. [Integration with Prometheus](#integration-with-prometheus)
4. [Integration with Alertmanager](#integration-with-alertmanager)
5. [Alert Descriptions](#alert-descriptions)
6. [Response Procedures](#response-procedures)
7. [Tuning Alert Thresholds](#tuning-alert-thresholds)
8. [Testing Alerts](#testing-alerts)

## Overview

The ELARA Protocol alerting rules are designed to provide early warning of operational issues before they impact service availability. The rules are organized into six categories:

- **Node Health**: Overall node health status
- **Message Processing**: Message throughput, latency, and drop rates
- **Resource Utilization**: Memory and CPU usage
- **Protocol Health**: Time drift, state divergence
- **Connection Health**: Connection counts and failure rates
- **Security**: Replay attacks and authentication failures

### Alert Severity Levels

- **critical**: Immediate action required. Service degradation or outage is occurring or imminent.
- **warning**: Investigation needed. Potential issues or approaching thresholds that may lead to problems.

## Alert Categories

### 1. Node Health Alerts

| Alert | Severity | Threshold | Description |
|-------|----------|-----------|-------------|
| `NodeUnhealthy` | critical | Health status != 1 for 2m | Node health check is failing |

### 2. Message Processing Alerts

| Alert | Severity | Threshold | Description |
|-------|----------|-----------|-------------|
| `HighMessageDropRate` | warning | >1% drop rate for 5m | Messages are being dropped |
| `HighLatency` | warning | P95 >1000ms for 5m | Message processing latency is high |
| `MessageProcessingStalled` | critical | No messages for 3m | Message processing has completely stalled |

### 3. Resource Utilization Alerts

| Alert | Severity | Threshold | Description |
|-------|----------|-----------|-------------|
| `HighMemoryUsage` | warning | >1.8GB (90%) for 5m | Memory usage is approaching limit |
| `CriticalMemoryUsage` | critical | >1.95GB (97.5%) for 2m | Memory usage is critically high, OOM imminent |
| `HighCPUUsage` | warning | >80% for 10m | CPU usage is high |

### 4. Protocol Health Alerts

| Alert | Severity | Threshold | Description |
|-------|----------|-----------|-------------|
| `TimeDriftExceeded` | warning | >100ms drift for 5m | Time drift from cluster is high |
| `SevereTimeDrift` | critical | >500ms drift for 2m | Severe time drift will break protocol |
| `StateDivergence` | warning | >10 events for 5m | State is diverging from peers |

### 5. Connection Health Alerts

| Alert | Severity | Threshold | Description |
|-------|----------|-----------|-------------|
| `LowConnectionCount` | warning | <2 connections for 5m | Node has too few active connections |
| `HighConnectionFailureRate` | warning | >0.1 failures/sec for 5m | Connection establishment is failing |

### 6. Security Alerts

| Alert | Severity | Threshold | Description |
|-------|----------|-----------|-------------|
| `ReplayAttackDetected` | critical | Any replay attacks for 1m | Replay attacks are being detected |
| `HighAuthenticationFailureRate` | critical | >1 failure/sec for 3m | High rate of authentication failures |

## Integration with Prometheus

### Step 1: Configure Prometheus to Load Alert Rules

Add the following to your `prometheus.yml` configuration:

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

# Load alerting rules
rule_files:
  - "alerts.yml"

# Scrape ELARA node metrics
scrape_configs:
  - job_name: 'elara-nodes'
    static_configs:
      - targets:
          - 'node1.example.com:9090'
          - 'node2.example.com:9090'
          - 'node3.example.com:9090'
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
      - source_labels: [__address__]
        regex: '([^:]+):.*'
        target_label: node_id
        replacement: '$1'
```

### Step 2: Validate Alert Rules

Before deploying, validate the alert rules syntax:

```bash
promtool check rules alerts.yml
```

Expected output:
```
Checking alerts.yml
  SUCCESS: 15 rules found
```

### Step 3: Reload Prometheus Configuration

After updating the configuration, reload Prometheus:

```bash
# Send SIGHUP to reload configuration
kill -HUP $(pidof prometheus)

# Or use the HTTP API
curl -X POST http://localhost:9090/-/reload
```

### Step 4: Verify Rules are Loaded

Check that the rules are loaded in the Prometheus UI:

1. Navigate to `http://prometheus:9090/rules`
2. Verify all rule groups are present
3. Check that rules are evaluating correctly

## Integration with Alertmanager

### Step 1: Configure Alertmanager

Create an `alertmanager.yml` configuration:

```yaml
# alertmanager.yml
global:
  resolve_timeout: 5m
  slack_api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'

# Route alerts based on severity
route:
  receiver: 'default'
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  
  routes:
    # Critical alerts go to PagerDuty and Slack
    - match:
        severity: critical
      receiver: 'pagerduty-critical'
      continue: true
    
    - match:
        severity: critical
      receiver: 'slack-critical'
    
    # Warning alerts go to Slack only
    - match:
        severity: warning
      receiver: 'slack-warnings'

receivers:
  - name: 'default'
    slack_configs:
      - channel: '#elara-alerts'
        title: 'ELARA Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_SERVICE_KEY'
        description: '{{ .GroupLabels.alertname }}: {{ .CommonAnnotations.summary }}'

  - name: 'slack-critical'
    slack_configs:
      - channel: '#elara-critical'
        title: ':rotating_light: CRITICAL ALERT'
        text: |
          *Alert:* {{ .GroupLabels.alertname }}
          *Summary:* {{ .CommonAnnotations.summary }}
          *Description:* {{ .CommonAnnotations.description }}
          *Runbook:* {{ .CommonAnnotations.runbook_url }}
        color: 'danger'

  - name: 'slack-warnings'
    slack_configs:
      - channel: '#elara-warnings'
        title: ':warning: Warning Alert'
        text: |
          *Alert:* {{ .GroupLabels.alertname }}
          *Summary:* {{ .CommonAnnotations.summary }}
          *Description:* {{ .CommonAnnotations.description }}
        color: 'warning'

# Inhibition rules to reduce alert noise
inhibit_rules:
  # If node is unhealthy, suppress other alerts from that node
  - source_match:
      alertname: 'NodeUnhealthy'
    target_match_re:
      alertname: '.*'
    equal: ['node_id']
  
  # If critical memory alert is firing, suppress warning memory alert
  - source_match:
      alertname: 'CriticalMemoryUsage'
    target_match:
      alertname: 'HighMemoryUsage'
    equal: ['node_id']
  
  # If severe time drift is firing, suppress regular time drift alert
  - source_match:
      alertname: 'SevereTimeDrift'
    target_match:
      alertname: 'TimeDriftExceeded'
    equal: ['node_id']
```

### Step 2: Configure Prometheus to Use Alertmanager

Add Alertmanager configuration to `prometheus.yml`:

```yaml
# prometheus.yml
alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - 'alertmanager:9093'
```

### Step 3: Start Alertmanager

```bash
alertmanager --config.file=alertmanager.yml
```

### Step 4: Verify Integration

1. Navigate to `http://alertmanager:9093`
2. Check that Alertmanager is receiving alerts from Prometheus
3. Test alert routing by triggering a test alert

## Alert Descriptions

### NodeUnhealthy

**What it means**: The node's health check endpoint is reporting an unhealthy status.

**Impact**: The node is unable to serve traffic properly and should be removed from load balancing.

**Common causes**:
- Failed health checks (connection, memory, time drift, state convergence)
- Resource exhaustion
- Network connectivity issues
- Application errors

**Investigation**:
1. Check the health endpoint: `curl http://node:port/health`
2. Review node logs for errors
3. Check resource utilization (memory, CPU)
4. Verify network connectivity to peers

**Resolution**:
- Fix the underlying health check failure
- Restart the node if necessary
- Remove from cluster if issue persists

### HighMessageDropRate

**What it means**: The node is dropping more than 1% of messages.

**Impact**: Message loss can lead to state inconsistencies and degraded user experience.

**Common causes**:
- Network congestion or packet loss
- Insufficient buffer capacity
- Resource exhaustion (CPU, memory)
- Peer connectivity issues

**Investigation**:
1. Check network metrics: `rate(elara_messages_dropped_total[5m])`
2. Review CPU and memory utilization
3. Check peer connection health
4. Review message queue depths

**Resolution**:
- Increase buffer capacity if needed
- Scale resources (CPU, memory)
- Fix network issues
- Implement backpressure mechanisms

### HighLatency

**What it means**: The 95th percentile message latency exceeds 1000ms.

**Impact**: Degraded user experience and potential timeout issues.

**Common causes**:
- Network congestion or high RTT
- CPU saturation
- Disk I/O bottlenecks
- Large message queue backlog

**Investigation**:
1. Check latency histogram: `histogram_quantile(0.95, rate(elara_message_latency_ms_bucket[5m]))`
2. Review network latency to peers
3. Check CPU utilization and load average
4. Review message processing queue depths

**Resolution**:
- Optimize network routing
- Scale CPU resources
- Reduce message queue backlog
- Optimize message processing code

### HighMemoryUsage

**What it means**: Memory usage exceeds 90% of the configured limit (1.8GB of 2GB).

**Impact**: Risk of OOM kill and node failure.

**Common causes**:
- Memory leak in the application
- Excessive message buffering
- Large state size
- Insufficient memory allocation

**Investigation**:
1. Check memory trends: `elara_memory_usage_bytes`
2. Review memory usage over time
3. Analyze heap dumps if available
4. Check message queue sizes

**Resolution**:
- Fix memory leaks
- Increase memory limits
- Optimize state storage
- Implement message backpressure
- Restart node during maintenance window

### TimeDriftExceeded

**What it means**: The node's clock has drifted more than 100ms from the cluster.

**Impact**: Message ordering issues, state synchronization problems, incorrect causality detection.

**Common causes**:
- NTP synchronization failure
- System clock drift
- Hardware clock issues
- Network issues preventing NTP sync

**Investigation**:
1. Check time drift: `abs(elara_time_drift_ms)`
2. Verify NTP synchronization: `ntpq -p`
3. Check system clock: `timedatectl status`
4. Review time drift trends

**Resolution**:
- Ensure NTP daemon is running and synchronized
- Configure reliable NTP servers
- Fix hardware clock issues
- Consider using PTP for higher accuracy

### ReplayAttackDetected

**What it means**: The node has detected replay attacks (duplicate messages with same sequence number).

**Impact**: Active security threat, potential malicious peer.

**Common causes**:
- Malicious peer attempting replay attack
- Network tampering or MITM attack
- Misconfigured peer replaying old messages

**Investigation**:
1. Check replay attack counter: `rate(elara_replay_attacks_detected_total[5m])`
2. Identify source peer from logs
3. Review security logs for attack patterns
4. Verify replay window configuration

**Resolution**:
- Block the malicious peer immediately
- Rotate session keys
- Alert security team
- Investigate attack source
- Review security policies

## Response Procedures

### Critical Alert Response

When a critical alert fires:

1. **Acknowledge**: Acknowledge the alert in PagerDuty/Alertmanager
2. **Assess**: Quickly assess the impact and scope
3. **Communicate**: Notify the team via incident channel
4. **Investigate**: Follow the investigation steps in the alert description
5. **Mitigate**: Take immediate action to mitigate the issue
6. **Resolve**: Fix the root cause
7. **Document**: Document the incident and resolution
8. **Post-mortem**: Conduct a post-mortem if significant impact

### Warning Alert Response

When a warning alert fires:

1. **Review**: Review the alert description and context
2. **Investigate**: Follow the investigation steps
3. **Monitor**: Monitor the situation for escalation
4. **Plan**: Plan remediation during next maintenance window
5. **Document**: Document findings and planned actions

### Escalation Path

1. **Level 1**: On-call engineer (all critical alerts)
2. **Level 2**: Engineering lead (if unresolved after 30 minutes)
3. **Level 3**: Engineering manager (if unresolved after 1 hour)
4. **Level 4**: CTO (if major outage or security incident)

## Tuning Alert Thresholds

Alert thresholds should be tuned based on your specific deployment and SLOs. Here's how to adjust them:

### Adjusting Thresholds

Edit `alerts.yml` and modify the `expr` field:

```yaml
# Example: Increase memory threshold from 1.8GB to 1.9GB
- alert: HighMemoryUsage
  expr: elara_memory_usage_bytes > 1.9e9  # Changed from 1.8e9
  for: 5m
```

### Adjusting Duration

Modify the `for` field to change how long the condition must be true:

```yaml
# Example: Increase duration from 5m to 10m
- alert: HighMemoryUsage
  expr: elara_memory_usage_bytes > 1.8e9
  for: 10m  # Changed from 5m
```

### Testing Threshold Changes

After modifying thresholds:

1. Validate syntax: `promtool check rules alerts.yml`
2. Reload Prometheus: `curl -X POST http://localhost:9090/-/reload`
3. Monitor for false positives/negatives
4. Adjust as needed based on operational experience

### Recommended Tuning Process

1. **Start conservative**: Begin with sensitive thresholds to catch all issues
2. **Monitor for noise**: Track false positive rate
3. **Adjust gradually**: Increase thresholds to reduce noise while maintaining coverage
4. **Document changes**: Keep a changelog of threshold adjustments
5. **Review regularly**: Review alert effectiveness quarterly

## Testing Alerts

### Manual Testing

Test alerts by manually triggering conditions:

```bash
# Test HighMemoryUsage alert
# Allocate memory to trigger the alert
stress-ng --vm 1 --vm-bytes 1.9G --vm-method all --verify -t 10m

# Test HighCPUUsage alert
# Generate CPU load
stress-ng --cpu 4 --cpu-method all -t 15m

# Test TimeDriftExceeded alert
# Manually adjust system time (requires root)
sudo date -s "+200 seconds"
```

### Automated Testing

Create a test script to verify alert rules:

```bash
#!/bin/bash
# test-alerts.sh

# Test that alert rules are valid
promtool check rules alerts.yml

# Test that specific alerts would fire with test data
promtool test rules test-alerts.yml
```

Create `test-alerts.yml`:

```yaml
# test-alerts.yml
rule_files:
  - alerts.yml

evaluation_interval: 1m

tests:
  - interval: 1m
    input_series:
      - series: 'elara_memory_usage_bytes{node_id="test-node"}'
        values: '1.9e9+0x10'  # 1.9GB for 10 minutes
    
    alert_rule_test:
      - eval_time: 5m
        alertname: HighMemoryUsage
        exp_alerts:
          - exp_labels:
              severity: warning
              node_id: test-node
            exp_annotations:
              summary: "High memory usage on ELARA node test-node"
```

Run the test:

```bash
promtool test rules test-alerts.yml
```

### Integration Testing

Test the full alerting pipeline:

1. **Trigger alert**: Manually trigger a condition
2. **Verify Prometheus**: Check alert appears in Prometheus UI
3. **Verify Alertmanager**: Check alert is received by Alertmanager
4. **Verify notification**: Check notification is sent (Slack, PagerDuty)
5. **Verify resolution**: Resolve condition and verify alert clears

## Best Practices

1. **Start with defaults**: Use the provided thresholds as a starting point
2. **Tune based on SLOs**: Align alert thresholds with your SLOs
3. **Reduce noise**: Adjust thresholds to minimize false positives
4. **Document changes**: Keep a changelog of threshold adjustments
5. **Review regularly**: Review alert effectiveness quarterly
6. **Test thoroughly**: Test alerts before deploying to production
7. **Use inhibition**: Configure inhibition rules to reduce alert noise
8. **Provide context**: Ensure alerts have clear descriptions and runbook links
9. **Monitor alert fatigue**: Track alert volume and on-call burden
10. **Iterate continuously**: Continuously improve alerts based on operational experience

## Troubleshooting

### Alerts Not Firing

**Problem**: Expected alerts are not firing.

**Possible causes**:
- Alert rules not loaded in Prometheus
- Metrics not being scraped
- Threshold not reached
- Duration (`for`) not elapsed

**Investigation**:
1. Check Prometheus rules page: `http://prometheus:9090/rules`
2. Verify metrics are being scraped: `http://prometheus:9090/targets`
3. Query metrics manually to check values
4. Check Prometheus logs for errors

### Too Many Alerts

**Problem**: Alert volume is too high (alert fatigue).

**Solutions**:
1. Increase thresholds to reduce sensitivity
2. Increase duration (`for`) to require longer condition
3. Add inhibition rules to suppress related alerts
4. Group related alerts together
5. Review and remove unnecessary alerts

### Alerts Not Reaching Alertmanager

**Problem**: Alerts fire in Prometheus but don't reach Alertmanager.

**Investigation**:
1. Check Prometheus alerting configuration
2. Verify Alertmanager is running and accessible
3. Check Prometheus logs for alerting errors
4. Test Alertmanager connectivity: `curl http://alertmanager:9093/-/healthy`

### Notifications Not Sent

**Problem**: Alerts reach Alertmanager but notifications aren't sent.

**Investigation**:
1. Check Alertmanager configuration
2. Verify receiver configuration (Slack webhook, PagerDuty key)
3. Check Alertmanager logs for errors
4. Test notification channels manually

## Additional Resources

- [Prometheus Alerting Documentation](https://prometheus.io/docs/alerting/latest/overview/)
- [Alertmanager Documentation](https://prometheus.io/docs/alerting/latest/alertmanager/)
- [ELARA Protocol Operational Runbook](../docs/operations/RUNBOOK.md)
- [ELARA Protocol Monitoring Guide](../docs/operations/MONITORING.md)

## Support

For questions or issues with alerting:

1. Check the [ELARA Protocol documentation](https://docs.elara-protocol.io)
2. Review the [operational runbook](../docs/operations/RUNBOOK.md)
3. Contact the ELARA operations team
4. File an issue on GitHub

---

**Last Updated**: 2024
**Version**: 1.0
**Maintainer**: ELARA Protocol Operations Team
