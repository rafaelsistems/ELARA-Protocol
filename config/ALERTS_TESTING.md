# ELARA Protocol Alert Testing Guide

This guide provides comprehensive instructions for testing the Prometheus alerting rules to ensure they work correctly before deploying to production.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Validation Testing](#validation-testing)
3. [Unit Testing](#unit-testing)
4. [Integration Testing](#integration-testing)
5. [End-to-End Testing](#end-to-end-testing)
6. [Load Testing Alerts](#load-testing-alerts)
7. [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Tools

```bash
# Install Prometheus tools
wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz
tar xvfz prometheus-2.45.0.linux-amd64.tar.gz
cd prometheus-2.45.0.linux-amd64

# promtool should now be available
./promtool --version
```

### Test Environment Setup

```bash
# Create test directory
mkdir -p /tmp/alert-testing
cd /tmp/alert-testing

# Copy alert rules
cp /path/to/config/alerts.yml .

# Create test data directory
mkdir -p test-data
```

## Validation Testing

### Step 1: Validate Alert Rule Syntax

```bash
# Validate that alert rules are syntactically correct
promtool check rules alerts.yml
```

Expected output:
```
Checking alerts.yml
  SUCCESS: 15 rules found
```

If there are errors, fix them before proceeding.

### Step 2: Validate Alert Expressions

Test that PromQL expressions are valid:

```bash
# Test each alert expression individually
promtool query instant http://prometheus:9090 \
  'rate(elara_messages_dropped_total[5m]) > 0.01'

promtool query instant http://prometheus:9090 \
  'histogram_quantile(0.95, rate(elara_message_latency_ms_bucket[5m])) > 1000'

# Add more expressions as needed
```

## Unit Testing

### Create Test Cases

Create `test-alerts.yml`:

```yaml
# test-alerts.yml
rule_files:
  - alerts.yml

evaluation_interval: 1m

tests:
  # Test HighMessageDropRate alert
  - interval: 1m
    input_series:
      - series: 'elara_messages_dropped_total{node_id="test-node-1"}'
        values: '0+60x10'  # 60 messages/min for 10 minutes = 1 msg/sec
      - series: 'elara_messages_sent_total{node_id="test-node-1"}'
        values: '0+6000x10'  # 6000 messages/min = 100 msg/sec
    
    alert_rule_test:
      - eval_time: 5m
        alertname: HighMessageDropRate
        exp_alerts:
          - exp_labels:
              severity: warning
              component: message_processing
              node_id: test-node-1
            exp_annotations:
              summary: "High message drop rate on ELARA node test-node-1"

  # Test HighMemoryUsage alert
  - interval: 1m
    input_series:
      - series: 'elara_memory_usage_bytes{node_id="test-node-2"}'
        values: '1.9e9+0x10'  # 1.9GB constant for 10 minutes
    
    alert_rule_test:
      - eval_time: 5m
        alertname: HighMemoryUsage
        exp_alerts:
          - exp_labels:
              severity: warning
              component: resource_management
              node_id: test-node-2

  # Test CriticalMemoryUsage alert
  - interval: 1m
    input_series:
      - series: 'elara_memory_usage_bytes{node_id="test-node-3"}'
        values: '1.96e9+0x10'  # 1.96GB constant for 10 minutes
    
    alert_rule_test:
      - eval_time: 2m
        alertname: CriticalMemoryUsage
        exp_alerts:
          - exp_labels:
              severity: critical
              component: resource_management
              node_id: test-node-3

  # Test TimeDriftExceeded alert
  - interval: 1m
    input_series:
      - series: 'elara_time_drift_ms{node_id="test-node-4"}'
        values: '150+0x10'  # 150ms drift constant for 10 minutes
    
    alert_rule_test:
      - eval_time: 5m
        alertname: TimeDriftExceeded
        exp_alerts:
          - exp_labels:
              severity: warning
              component: time_engine
              node_id: test-node-4

  # Test SevereTimeDrift alert
  - interval: 1m
    input_series:
      - series: 'elara_time_drift_ms{node_id="test-node-5"}'
        values: '600+0x10'  # 600ms drift constant for 10 minutes
    
    alert_rule_test:
      - eval_time: 2m
        alertname: SevereTimeDrift
        exp_alerts:
          - exp_labels:
              severity: critical
              component: time_engine
              node_id: test-node-5

  # Test NodeUnhealthy alert
  - interval: 1m
    input_series:
      - series: 'elara_health_status{node_id="test-node-6"}'
        values: '0+0x10'  # Unhealthy (0) for 10 minutes
    
    alert_rule_test:
      - eval_time: 2m
        alertname: NodeUnhealthy
        exp_alerts:
          - exp_labels:
              severity: critical
              component: health_check
              node_id: test-node-6

  # Test HighLatency alert
  - interval: 1m
    input_series:
      # Simulate histogram buckets for high latency
      - series: 'elara_message_latency_ms_bucket{node_id="test-node-7",le="100"}'
        values: '0+10x10'
      - series: 'elara_message_latency_ms_bucket{node_id="test-node-7",le="500"}'
        values: '0+20x10'
      - series: 'elara_message_latency_ms_bucket{node_id="test-node-7",le="1000"}'
        values: '0+30x10'
      - series: 'elara_message_latency_ms_bucket{node_id="test-node-7",le="2000"}'
        values: '0+50x10'
      - series: 'elara_message_latency_ms_bucket{node_id="test-node-7",le="+Inf"}'
        values: '0+60x10'
    
    alert_rule_test:
      - eval_time: 5m
        alertname: HighLatency
        exp_alerts:
          - exp_labels:
              severity: warning
              component: message_processing
              node_id: test-node-7

  # Test LowConnectionCount alert
  - interval: 1m
    input_series:
      - series: 'elara_active_connections{node_id="test-node-8"}'
        values: '1+0x10'  # Only 1 connection for 10 minutes
    
    alert_rule_test:
      - eval_time: 5m
        alertname: LowConnectionCount
        exp_alerts:
          - exp_labels:
              severity: warning
              component: connection_management
              node_id: test-node-8

  # Test ReplayAttackDetected alert
  - interval: 1m
    input_series:
      - series: 'elara_replay_attacks_detected_total{node_id="test-node-9"}'
        values: '0+5x10'  # 5 attacks/min for 10 minutes
    
    alert_rule_test:
      - eval_time: 1m
        alertname: ReplayAttackDetected
        exp_alerts:
          - exp_labels:
              severity: critical
              component: security
              node_id: test-node-9

  # Test that alerts don't fire below threshold
  - interval: 1m
    input_series:
      - series: 'elara_memory_usage_bytes{node_id="test-node-10"}'
        values: '1.5e9+0x10'  # 1.5GB - below threshold
    
    alert_rule_test:
      - eval_time: 10m
        alertname: HighMemoryUsage
        exp_alerts: []  # No alerts expected

  # Test that alerts resolve
  - interval: 1m
    input_series:
      - series: 'elara_memory_usage_bytes{node_id="test-node-11"}'
        values: '1.9e9+0x5 1.5e9+0x5'  # High then normal
    
    alert_rule_test:
      - eval_time: 5m
        alertname: HighMemoryUsage
        exp_alerts:
          - exp_labels:
              severity: warning
              node_id: test-node-11
      
      - eval_time: 10m
        alertname: HighMemoryUsage
        exp_alerts: []  # Should be resolved
```

### Run Unit Tests

```bash
# Run all unit tests
promtool test rules test-alerts.yml
```

Expected output:
```
Unit Testing: test-alerts.yml
  SUCCESS
```

## Integration Testing

### Step 1: Set Up Test Prometheus Instance

```bash
# Create test Prometheus configuration
cat > prometheus-test.yml <<EOF
global:
  scrape_interval: 5s
  evaluation_interval: 5s

rule_files:
  - alerts.yml

scrape_configs:
  - job_name: 'test-node'
    static_configs:
      - targets: ['localhost:9090']
EOF

# Start test Prometheus instance
prometheus --config.file=prometheus-test.yml \
  --storage.tsdb.path=/tmp/prometheus-test \
  --web.listen-address=:9091
```

### Step 2: Generate Test Metrics

Create a script to expose test metrics:

```python
#!/usr/bin/env python3
# test-metrics-server.py

from prometheus_client import start_http_server, Gauge, Counter, Histogram
import time
import random

# Create metrics
memory_usage = Gauge('elara_memory_usage_bytes', 'Memory usage', ['node_id'])
health_status = Gauge('elara_health_status', 'Health status', ['node_id'])
messages_dropped = Counter('elara_messages_dropped_total', 'Dropped messages', ['node_id'])
time_drift = Gauge('elara_time_drift_ms', 'Time drift', ['node_id'])
message_latency = Histogram('elara_message_latency_ms', 'Message latency', ['node_id'])

def simulate_high_memory():
    """Simulate high memory usage"""
    memory_usage.labels(node_id='test-node-1').set(1.9e9)  # 1.9GB

def simulate_unhealthy():
    """Simulate unhealthy node"""
    health_status.labels(node_id='test-node-2').set(0)  # Unhealthy

def simulate_message_drops():
    """Simulate message drops"""
    for _ in range(100):
        messages_dropped.labels(node_id='test-node-3').inc()
        time.sleep(0.1)

def simulate_time_drift():
    """Simulate time drift"""
    time_drift.labels(node_id='test-node-4').set(150)  # 150ms drift

def simulate_high_latency():
    """Simulate high latency"""
    for _ in range(100):
        message_latency.labels(node_id='test-node-5').observe(1500)  # 1500ms
        time.sleep(0.1)

if __name__ == '__main__':
    # Start metrics server
    start_http_server(9090)
    print("Metrics server started on :9090")
    
    # Run simulations
    while True:
        simulate_high_memory()
        simulate_unhealthy()
        simulate_message_drops()
        simulate_time_drift()
        simulate_high_latency()
        time.sleep(10)
```

Run the test server:

```bash
python3 test-metrics-server.py
```

### Step 3: Verify Alerts Fire

```bash
# Check pending alerts
curl http://localhost:9091/api/v1/alerts | jq '.data.alerts[] | select(.state=="pending")'

# Check firing alerts (after duration threshold)
curl http://localhost:9091/api/v1/alerts | jq '.data.alerts[] | select(.state=="firing")'
```

## End-to-End Testing

### Step 1: Set Up Full Stack

```bash
# Start Alertmanager
alertmanager --config.file=alertmanager-test.yml \
  --storage.path=/tmp/alertmanager-test \
  --web.listen-address=:9093

# Start Prometheus with Alertmanager integration
prometheus --config.file=prometheus-test.yml \
  --storage.tsdb.path=/tmp/prometheus-test \
  --web.listen-address=:9091

# Start test metrics server
python3 test-metrics-server.py
```

### Step 2: Configure Test Alertmanager

Create `alertmanager-test.yml`:

```yaml
global:
  resolve_timeout: 1m

route:
  receiver: 'test-webhook'
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h

receivers:
  - name: 'test-webhook'
    webhook_configs:
      - url: 'http://localhost:8080/alerts'
        send_resolved: true
```

### Step 3: Create Webhook Receiver

```python
#!/usr/bin/env python3
# test-webhook-receiver.py

from flask import Flask, request
import json

app = Flask(__name__)

@app.route('/alerts', methods=['POST'])
def receive_alert():
    data = request.json
    print("=" * 80)
    print("ALERT RECEIVED:")
    print(json.dumps(data, indent=2))
    print("=" * 80)
    return '', 200

if __name__ == '__main__':
    app.run(port=8080)
```

Run the webhook receiver:

```bash
python3 test-webhook-receiver.py
```

### Step 4: Verify End-to-End Flow

1. Wait for alerts to fire (check duration thresholds)
2. Verify alerts appear in Prometheus UI: `http://localhost:9091/alerts`
3. Verify alerts are sent to Alertmanager: `http://localhost:9093`
4. Verify webhook receives alerts (check webhook receiver output)

## Load Testing Alerts

### Test Alert Volume

```bash
# Generate high volume of alerts
for i in {1..100}; do
  curl -X POST http://localhost:9090/api/v1/admin/tsdb/snapshot
done
```

### Test Alert Grouping

Verify that alerts are properly grouped by `alertname`, `cluster`, and `node_id`.

### Test Alert Inhibition

1. Trigger `NodeUnhealthy` alert
2. Trigger other alerts on the same node
3. Verify that other alerts are inhibited

## Troubleshooting

### Alerts Not Firing

**Check 1: Verify metrics are being scraped**
```bash
curl http://localhost:9091/api/v1/query?query=elara_memory_usage_bytes
```

**Check 2: Verify alert rules are loaded**
```bash
curl http://localhost:9091/api/v1/rules
```

**Check 3: Check Prometheus logs**
```bash
tail -f /var/log/prometheus/prometheus.log
```

### Alerts Not Reaching Alertmanager

**Check 1: Verify Alertmanager is configured**
```bash
curl http://localhost:9091/api/v1/alertmanagers
```

**Check 2: Test Alertmanager connectivity**
```bash
curl http://localhost:9093/-/healthy
```

**Check 3: Check Prometheus logs for alerting errors**
```bash
grep -i "alertmanager" /var/log/prometheus/prometheus.log
```

### Alerts Not Sending Notifications

**Check 1: Verify Alertmanager configuration**
```bash
curl http://localhost:9093/api/v1/status
```

**Check 2: Check Alertmanager logs**
```bash
tail -f /var/log/alertmanager/alertmanager.log
```

**Check 3: Test notification channel manually**
```bash
# For Slack
curl -X POST -H 'Content-type: application/json' \
  --data '{"text":"Test alert"}' \
  YOUR_SLACK_WEBHOOK_URL
```

## Automated Testing Script

Create `test-all-alerts.sh`:

```bash
#!/bin/bash
set -e

echo "=== ELARA Alert Testing Suite ==="

# Step 1: Validate syntax
echo "Step 1: Validating alert rule syntax..."
promtool check rules alerts.yml
echo "✓ Syntax validation passed"

# Step 2: Run unit tests
echo "Step 2: Running unit tests..."
promtool test rules test-alerts.yml
echo "✓ Unit tests passed"

# Step 3: Start test environment
echo "Step 3: Starting test environment..."
docker-compose -f docker-compose-test.yml up -d
sleep 10
echo "✓ Test environment started"

# Step 4: Wait for alerts to fire
echo "Step 4: Waiting for alerts to fire (5 minutes)..."
sleep 300

# Step 5: Verify alerts
echo "Step 5: Verifying alerts..."
FIRING_ALERTS=$(curl -s http://localhost:9091/api/v1/alerts | jq '.data.alerts[] | select(.state=="firing") | .labels.alertname' | wc -l)
echo "Found $FIRING_ALERTS firing alerts"

if [ "$FIRING_ALERTS" -gt 0 ]; then
    echo "✓ Alerts are firing"
else
    echo "✗ No alerts firing"
    exit 1
fi

# Step 6: Cleanup
echo "Step 6: Cleaning up..."
docker-compose -f docker-compose-test.yml down
echo "✓ Cleanup complete"

echo "=== All tests passed! ==="
```

Run the automated test suite:

```bash
chmod +x test-all-alerts.sh
./test-all-alerts.sh
```

## Continuous Testing

Add alert testing to CI/CD pipeline:

```yaml
# .github/workflows/test-alerts.yml
name: Test Prometheus Alerts

on:
  pull_request:
    paths:
      - 'config/alerts.yml'
      - 'config/test-alerts.yml'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install promtool
        run: |
          wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz
          tar xvfz prometheus-2.45.0.linux-amd64.tar.gz
          sudo mv prometheus-2.45.0.linux-amd64/promtool /usr/local/bin/
      
      - name: Validate alert rules
        run: promtool check rules config/alerts.yml
      
      - name: Run unit tests
        run: promtool test rules config/test-alerts.yml
```

---

**Remember**: Always test alerts in a non-production environment before deploying to production!
