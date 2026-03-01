# ELARA Protocol Operational Runbook

**Version**: 0.2.0  
**Last Updated**: 2024-01  
**Status**: Production  
**Audience**: On-call engineers, SREs, Operations teams

---

## Table of Contents

1. [Quick Reference](#quick-reference)
2. [System Overview](#system-overview)
3. [Common Operational Scenarios](#common-operational-scenarios)
4. [Troubleshooting Procedures](#troubleshooting-procedures)
5. [Emergency Procedures](#emergency-procedures)
6. [Diagnostic Commands](#diagnostic-commands)
7. [Recovery Procedures](#recovery-procedures)
8. [Escalation Paths](#escalation-paths)

---

## Quick Reference

### Critical Contacts

| Role | Contact | Escalation Time |
|------|---------|-----------------|
| Primary On-Call | PagerDuty rotation | Immediate |
| Secondary On-Call | PagerDuty rotation | After 15 minutes |
| Engineering Lead | [Contact info] | After 30 minutes |
| Security Team | [Contact info] | For security incidents |

### Key Endpoints

| Endpoint | Purpose | Expected Response |
|----------|---------|-------------------|
| `/health` | Overall health status | 200 OK (healthy/degraded), 503 (unhealthy) |
| `/ready` | Readiness probe | 200 OK (ready), 503 (not ready) |
| `/live` | Liveness probe | 200 OK (alive), 503 (dead) |
| `/metrics` | Prometheus metrics | 200 OK with metrics |

### Critical Metrics

| Metric | Normal Range | Warning Threshold | Critical Threshold |
|--------|--------------|-------------------|-------------------|
| `elara_messages_dropped_total` rate | < 0.1% | > 1% for 5m | > 5% for 2m |
| `elara_message_latency_ms` P95 | < 100ms | > 1000ms for 5m | > 5000ms for 2m |
| `elara_memory_usage_bytes` | < 1.6GB | > 1.8GB for 5m | > 1.95GB for 2m |
| `elara_time_drift_ms` | < 50ms | > 100ms for 5m | > 500ms for 2m |
| `elara_active_connections` | ≥ 3 | < 2 for 5m | 0 for 2m |

### Emergency Commands

```bash
# Check node health
curl http://localhost:8080/health | jq

# View recent logs
journalctl -u elara-node -n 100 --no-pager

# Graceful restart
systemctl restart elara-node

# Force kill (last resort)
systemctl kill -s SIGKILL elara-node

# Check metrics
curl http://localhost:9090/metrics | grep elara_

# View active connections
ss -tunap | grep elara
```

---

## System Overview

### Architecture Summary

ELARA Protocol is a distributed real-time communication system with the following key components:

- **Node**: Single instance of the ELARA runtime
- **Session**: Communication context between nodes
- **Connection**: Network transport between two nodes (UDP-based)
- **Health Server**: HTTP server exposing health check endpoints (port 8080)
- **Metrics Server**: HTTP server exposing Prometheus metrics (port 9090)

### Key Characteristics

- **Protocol**: UDP-based with custom reliability layer
- **Encryption**: ChaCha20-Poly1305 AEAD with per-class key ratcheting
- **State Model**: Eventually consistent CRDT-like state reconciliation
- **Time Model**: Dual clock system (perceptual + state clocks)
- **Failure Mode**: Graceful degradation (never hard failure)

### Resource Requirements

| Resource | Minimum | Recommended | Maximum |
|----------|---------|-------------|---------|
| Memory | 1GB | 2GB | 4GB |
| CPU | 1 core | 2 cores | 4 cores |
| Network | 1 Mbps | 10 Mbps | 100 Mbps |
| Disk | 1GB | 10GB | 100GB |

---

## Common Operational Scenarios

### Scenario 1: Node Startup

**When to use**: Starting a new node or restarting after maintenance.

**Prerequisites**:
- Configuration file is valid (`/etc/elara/config.toml`)
- Network ports are available (8080 for health, 9090 for metrics, custom for protocol)
- Sufficient system resources available

**Steps**:

1. **Validate configuration**:
   ```bash
   # Check config syntax
   elara-node --config /etc/elara/config.toml --validate
   
   # Expected output: "Configuration is valid"
   ```

2. **Check port availability**:
   ```bash
   # Check if ports are free
   ss -tuln | grep -E ':(8080|9090|7777)'
   
   # Expected: No output (ports are free)
   ```

3. **Start the node**:
   ```bash
   # Using systemd
   systemctl start elara-node
   
   # Check status
   systemctl status elara-node
   
   # Expected: "active (running)"
   ```

4. **Verify startup**:
   ```bash
   # Wait 10 seconds for initialization
   sleep 10
   
   # Check health endpoint
   curl http://localhost:8080/health | jq
   
   # Expected: {"status": "healthy", ...}
   ```

5. **Monitor logs for errors**:
   ```bash
   journalctl -u elara-node -f
   
   # Look for:
   # - "Node started successfully"
   # - "Health server listening on 0.0.0.0:8080"
   # - "Metrics server listening on 0.0.0.0:9090"
   ```

**Expected Duration**: 10-30 seconds

**Success Criteria**:
- Health endpoint returns 200 OK
- Metrics endpoint returns 200 OK
- No ERROR logs in journalctl
- Process is running (`systemctl status elara-node`)

**Rollback**: If startup fails, check logs and fix configuration issues before retrying.

---

### Scenario 2: Graceful Shutdown

**When to use**: Planned maintenance, configuration changes, version upgrades.

**Prerequisites**:
- Node is currently running
- No critical operations in progress (check metrics)

**Steps**:

1. **Check current load**:
   ```bash
   # Check active connections
   curl http://localhost:9090/metrics | grep elara_active_connections
   
   # Check message rate
   curl http://localhost:9090/metrics | grep elara_messages_sent_total
   ```

2. **Initiate graceful shutdown**:
   ```bash
   # Send SIGTERM (graceful shutdown signal)
   systemctl stop elara-node
   
   # Monitor shutdown progress
   journalctl -u elara-node -f
   ```

3. **Wait for connection draining**:
   ```bash
   # Shutdown should complete within 30 seconds
   # Watch for "Graceful shutdown complete" log message
   
   # If shutdown hangs after 60 seconds, check:
   systemctl status elara-node
   ```

4. **Verify shutdown**:
   ```bash
   # Check process is stopped
   systemctl status elara-node
   
   # Expected: "inactive (dead)"
   
   # Verify ports are released
   ss -tuln | grep -E ':(8080|9090|7777)'
   
   # Expected: No output
   ```

**Expected Duration**: 10-30 seconds

**Success Criteria**:
- Process exits cleanly (exit code 0)
- All connections closed gracefully
- No ERROR logs during shutdown
- Ports are released

**Troubleshooting**:
- If shutdown hangs > 60 seconds, proceed to force kill (see Emergency Procedures)

---

### Scenario 3: Node Restart

**When to use**: Configuration changes, minor issues, routine maintenance.

**Prerequisites**:
- Node is running or stopped
- New configuration is validated

**Steps**:

1. **Validate new configuration** (if changed):
   ```bash
   elara-node --config /etc/elara/config.toml --validate
   ```

2. **Restart the node**:
   ```bash
   # Graceful restart (stop + start)
   systemctl restart elara-node
   
   # Monitor restart
   journalctl -u elara-node -f
   ```

3. **Verify restart**:
   ```bash
   # Wait for initialization
   sleep 10
   
   # Check health
   curl http://localhost:8080/health | jq
   
   # Check metrics
   curl http://localhost:9090/metrics | grep elara_active_connections
   ```

**Expected Duration**: 20-60 seconds

**Success Criteria**:
- Node starts successfully
- Health endpoint returns healthy
- Connections re-establish
- No ERROR logs

---

### Scenario 4: Scaling Up (Adding Nodes)

**When to use**: Increasing capacity, handling more load, improving redundancy.

**Prerequisites**:
- Infrastructure provisioned (VMs, containers, etc.)
- Configuration files prepared
- Network connectivity between nodes

**Steps**:

1. **Prepare new node configuration**:
   ```bash
   # Copy base configuration
   cp /etc/elara/config.toml /etc/elara/config-new.toml
   
   # Update node-specific settings:
   # - node_id (must be unique)
   # - bind_address
   # - peer_addresses (add existing nodes)
   ```

2. **Start new node**:
   ```bash
   systemctl start elara-node@new
   
   # Monitor startup
   journalctl -u elara-node@new -f
   ```

3. **Verify new node joins cluster**:
   ```bash
   # Check connections on new node
   curl http://new-node:9090/metrics | grep elara_active_connections
   
   # Expected: Connections to existing nodes
   ```

4. **Update existing nodes** (if needed):
   ```bash
   # Add new node to peer list in existing nodes
   # Restart existing nodes one at a time
   ```

5. **Verify cluster health**:
   ```bash
   # Check all nodes are connected
   for node in node1 node2 node3 new-node; do
     echo "=== $node ==="
     curl http://$node:8080/health | jq .status
   done
   ```

**Expected Duration**: 5-10 minutes per node

**Success Criteria**:
- New node is healthy
- New node has connections to existing nodes
- Existing nodes have connections to new node
- No increase in error rates

---

### Scenario 5: Scaling Down (Removing Nodes)

**When to use**: Reducing capacity, cost optimization, decommissioning.

**Prerequisites**:
- Remaining nodes can handle the load
- Node to remove is not critical for quorum (if applicable)

**Steps**:

1. **Verify remaining capacity**:
   ```bash
   # Check current load across all nodes
   for node in node1 node2 node3; do
     echo "=== $node ==="
     curl http://$node:9090/metrics | grep -E 'elara_(active_connections|memory_usage_bytes|cpu_usage_percent)'
   done
   
   # Ensure remaining nodes can handle redistributed load
   ```

2. **Gracefully shutdown target node**:
   ```bash
   # On the node to remove
   systemctl stop elara-node
   
   # Monitor connection draining
   journalctl -u elara-node -f
   ```

3. **Remove from peer lists**:
   ```bash
   # Update configuration on remaining nodes
   # Remove the decommissioned node from peer_addresses
   
   # Reload configuration (no restart needed if dynamic)
   systemctl reload elara-node
   ```

4. **Verify cluster stability**:
   ```bash
   # Check remaining nodes are healthy
   for node in node1 node2 node3; do
     echo "=== $node ==="
     curl http://$node:8080/health | jq .status
   done
   
   # Monitor for 5 minutes to ensure stability
   ```

5. **Decommission infrastructure**:
   ```bash
   # Stop and disable service
   systemctl disable elara-node
   
   # Remove from monitoring/alerting
   # Deallocate infrastructure resources
   ```

**Expected Duration**: 10-15 minutes

**Success Criteria**:
- Target node shuts down cleanly
- Remaining nodes remain healthy
- No increase in error rates or latency
- Connections redistribute successfully

---

## Troubleshooting Procedures

### Issue 1: High Message Drop Rate

**Symptoms**:
- Alert: `HighMessageDropRate` firing
- Metric: `rate(elara_messages_dropped_total[5m]) > 0.01`
- User impact: Degraded communication quality

**Diagnostic Decision Tree**:

```
High Message Drop Rate
├─ Check CPU usage
│  ├─ > 80% → CPU saturation (see CPU Saturation)
│  └─ < 80% → Continue
├─ Check memory usage
│  ├─ > 90% → Memory pressure (see Memory Pressure)
│  └─ < 90% → Continue
├─ Check network
│  ├─ High packet loss → Network issues (see Network Issues)
│  └─ Normal → Continue
└─ Check message rate
   ├─ Exceeds capacity → Rate limiting (see Rate Limiting)
   └─ Normal → Investigate application logic
```

**Investigation Steps**:

1. **Check system resources**:
   ```bash
   # CPU usage
   top -bn1 | grep elara-node
   
   # Memory usage
   curl http://localhost:9090/metrics | grep elara_memory_usage_bytes
   
   # Network stats
   netstat -s | grep -E '(dropped|errors)'
   ```

2. **Check message rates**:
   ```bash
   # Current message rate
   curl http://localhost:9090/metrics | grep elara_messages_sent_total
   
   # Drop rate
   curl http://localhost:9090/metrics | grep elara_messages_dropped_total
   
   # Calculate drop percentage
   ```

3. **Check logs for errors**:
   ```bash
   journalctl -u elara-node -n 1000 | grep -i 'drop\|error\|fail'
   ```

4. **Check peer connectivity**:
   ```bash
   # Active connections
   curl http://localhost:9090/metrics | grep elara_active_connections
   
   # Connection failures
   curl http://localhost:9090/metrics | grep elara_failed_connections_total
   ```

**Resolution Steps**:

**If CPU saturation**:
- Scale horizontally (add more nodes)
- Optimize message processing (check for inefficient operations)
- Consider upgrading to faster CPUs

**If memory pressure**:
- Increase memory allocation
- Check for memory leaks (monitor over time)
- Reduce buffer sizes in configuration

**If network issues**:
- Check network infrastructure (switches, routers)
- Verify MTU settings (should be ≥ 1500)
- Check for packet loss: `ping -c 100 peer-node`

**If rate limiting**:
- Increase rate limits in configuration
- Add more nodes to distribute load
- Implement backpressure in application

**Expected Resolution Time**: 15-30 minutes

**Escalation Criteria**:
- Drop rate > 5% for > 10 minutes
- Unable to identify root cause within 30 minutes
- Multiple nodes affected simultaneously

---

### Issue 2: High Latency

**Symptoms**:
- Alert: `HighLatency` firing
- Metric: `histogram_quantile(0.95, elara_message_latency_ms) > 1000`
- User impact: Slow communication, poor user experience

**Diagnostic Decision Tree**:

```
High Latency
├─ Check network latency
│  ├─ High RTT → Network path issues
│  └─ Normal RTT → Continue
├─ Check CPU usage
│  ├─ > 80% → CPU saturation
│  └─ < 80% → Continue
├─ Check message queue depth
│  ├─ Growing → Backlog building up
│  └─ Stable → Continue
└─ Check time drift
   ├─ > 100ms → Time synchronization issues
   └─ < 100ms → Application-level latency
```

**Investigation Steps**:

1. **Measure network latency**:
   ```bash
   # Ping peer nodes
   for peer in peer1 peer2 peer3; do
     echo "=== $peer ==="
     ping -c 10 $peer | tail -1
   done
   
   # Expected: RTT < 50ms for local network
   ```

2. **Check latency metrics**:
   ```bash
   # Get latency percentiles
   curl http://localhost:9090/metrics | grep elara_message_latency_ms
   
   # Look at histogram buckets to understand distribution
   ```

3. **Check CPU and load**:
   ```bash
   # System load
   uptime
   
   # CPU usage per process
   top -bn1 | head -20
   ```

4. **Check time drift**:
   ```bash
   # Time drift metric
   curl http://localhost:9090/metrics | grep elara_time_drift_ms
   
   # System time sync
   timedatectl status
   ```

**Resolution Steps**:

**If network latency**:
- Check network path: `traceroute peer-node`
- Verify network infrastructure health
- Check for bandwidth saturation
- Consider geographic proximity of nodes

**If CPU saturation**:
- Scale horizontally (add nodes)
- Optimize hot paths (profile with perf)
- Upgrade to faster CPUs

**If message queue backlog**:
- Increase worker threads (if configurable)
- Reduce message rate temporarily
- Scale horizontally

**If time drift**:
- Check NTP/chrony status: `chronyc tracking`
- Restart time sync: `systemctl restart chronyd`
- Verify time source is reachable

**Expected Resolution Time**: 20-45 minutes

**Escalation Criteria**:
- P95 latency > 5000ms for > 10 minutes
- Latency increasing continuously
- Affecting multiple nodes

---

### Issue 3: Node Unhealthy

**Symptoms**:
- Alert: `NodeUnhealthy` firing
- Health endpoint returns 503
- User impact: Node not serving traffic

**Diagnostic Decision Tree**:

```
Node Unhealthy
├─ Check which health check failed
│  ├─ connections → Connection issues
│  ├─ memory → Memory pressure
│  ├─ time_drift → Time sync issues
│  └─ state_convergence → State sync issues
└─ Check if node is responsive
   ├─ Yes → Specific subsystem failure
   └─ No → Node deadlock/crash
```

**Investigation Steps**:

1. **Check health endpoint details**:
   ```bash
   # Get detailed health status
   curl http://localhost:8080/health | jq
   
   # Look for which check is failing and why
   # Example output:
   # {
   #   "status": "unhealthy",
   #   "checks": {
   #     "memory": {
   #       "status": "unhealthy",
   #       "reason": "Memory usage 1850MB exceeds limit 1800MB"
   #     }
   #   }
   # }
   ```

2. **Check process status**:
   ```bash
   # Is process running?
   systemctl status elara-node
   
   # Process details
   ps aux | grep elara-node
   ```

3. **Check recent logs**:
   ```bash
   # Last 100 log lines
   journalctl -u elara-node -n 100 --no-pager
   
   # Look for ERROR or WARN messages
   journalctl -u elara-node -p err -n 50
   ```

4. **Check system resources**:
   ```bash
   # Memory
   free -h
   
   # Disk space
   df -h
   
   # CPU
   top -bn1 | head -20
   ```

**Resolution Steps**:

**If connection health check failed**:
```bash
# Check active connections
curl http://localhost:9090/metrics | grep elara_active_connections

# Check connection failures
curl http://localhost:9090/metrics | grep elara_failed_connections_total

# Check network connectivity to peers
for peer in peer1 peer2 peer3; do
  nc -zv $peer 7777
done

# Resolution: Fix network connectivity or restart node
```

**If memory health check failed**:
```bash
# Check memory usage
curl http://localhost:9090/metrics | grep elara_memory_usage_bytes

# Check for memory leaks (compare over time)
# Resolution: Restart node or increase memory limit
systemctl restart elara-node
```

**If time drift health check failed**:
```bash
# Check time drift
curl http://localhost:9090/metrics | grep elara_time_drift_ms

# Check system time sync
chronyc tracking

# Resolution: Fix time synchronization
systemctl restart chronyd
sleep 5
systemctl restart elara-node
```

**If state convergence health check failed**:
```bash
# Check state divergence
curl http://localhost:9090/metrics | grep elara_state_divergence

# Check peer connectivity
curl http://localhost:9090/metrics | grep elara_active_connections

# Resolution: Usually self-heals, but may need restart
systemctl restart elara-node
```

**Expected Resolution Time**: 10-30 minutes

**Escalation Criteria**:
- Node remains unhealthy after restart
- Multiple nodes unhealthy simultaneously
- Health checks failing for unknown reasons

---

### Issue 4: Connection Failures

**Symptoms**:
- Alert: `HighConnectionFailureRate` or `LowConnectionCount`
- Metric: `rate(elara_failed_connections_total[5m]) > 0.1`
- User impact: Reduced redundancy, potential isolation

**Investigation Steps**:

1. **Check connection metrics**:
   ```bash
   # Active connections
   curl http://localhost:9090/metrics | grep elara_active_connections
   
   # Failed connections
   curl http://localhost:9090/metrics | grep elara_failed_connections_total
   
   # Total connections attempted
   curl http://localhost:9090/metrics | grep elara_total_connections
   ```

2. **Check network connectivity**:
   ```bash
   # Test UDP connectivity to peers
   nc -u -zv peer1 7777
   nc -u -zv peer2 7777
   nc -u -zv peer3 7777
   
   # Check firewall rules
   iptables -L -n | grep 7777
   ```

3. **Check peer node status**:
   ```bash
   # Check if peer nodes are healthy
   for peer in peer1 peer2 peer3; do
     echo "=== $peer ==="
     curl http://$peer:8080/health | jq .status
   done
   ```

4. **Check logs for connection errors**:
   ```bash
   journalctl -u elara-node -n 500 | grep -i 'connection\|peer\|handshake'
   ```

**Resolution Steps**:

**If firewall blocking**:
```bash
# Add firewall rule (example for iptables)
iptables -A INPUT -p udp --dport 7777 -j ACCEPT
iptables -A OUTPUT -p udp --dport 7777 -j ACCEPT

# Save rules
iptables-save > /etc/iptables/rules.v4

# Verify
iptables -L -n | grep 7777
```

**If peer nodes down**:
```bash
# Restart peer nodes
ssh peer1 'systemctl restart elara-node'

# Or remove from peer list temporarily
# Edit /etc/elara/config.toml and remove unreachable peers
systemctl reload elara-node
```

**If network path issues**:
```bash
# Check routing
traceroute peer1

# Check MTU
ping -M do -s 1472 peer1

# If MTU issues, adjust in config
# mtu = 1200  # in /etc/elara/config.toml
```

**If authentication failures**:
```bash
# Check logs for auth errors
journalctl -u elara-node | grep -i 'auth\|verify\|signature'

# May indicate:
# - Clock skew (check time sync)
# - Key mismatch (verify peer keys)
# - Replay attack (check security logs)
```

**Expected Resolution Time**: 15-30 minutes

**Escalation Criteria**:
- Unable to establish any connections
- Authentication failures persist after time sync
- Network infrastructure issues

---

### Issue 5: Memory Pressure

**Symptoms**:
- Alert: `HighMemoryUsage` or `CriticalMemoryUsage`
- Metric: `elara_memory_usage_bytes > 1.8e9`
- User impact: Potential OOM, node instability

**Investigation Steps**:

1. **Check current memory usage**:
   ```bash
   # Node memory usage
   curl http://localhost:9090/metrics | grep elara_memory_usage_bytes
   
   # System memory
   free -h
   
   # Process memory details
   ps aux | grep elara-node | awk '{print $6}'
   ```

2. **Check memory growth over time**:
   ```bash
   # Query Prometheus for memory trend (if available)
   # Or check historical metrics
   
   # Check if memory is growing continuously (leak)
   # vs. stable high usage (legitimate load)
   ```

3. **Check for memory-intensive operations**:
   ```bash
   # Check message rate
   curl http://localhost:9090/metrics | grep elara_messages_sent_total
   
   # Check connection count
   curl http://localhost:9090/metrics | grep elara_active_connections
   
   # Check state size (if exposed)
   curl http://localhost:9090/metrics | grep elara_state
   ```

4. **Check logs for memory warnings**:
   ```bash
   journalctl -u elara-node | grep -i 'memory\|oom\|allocation'
   ```

**Resolution Steps**:

**If legitimate high load**:
```bash
# Option 1: Increase memory limit
# Edit systemd service file
systemctl edit elara-node

# Add:
# [Service]
# MemoryMax=4G

systemctl daemon-reload
systemctl restart elara-node
```

**If memory leak suspected**:
```bash
# Restart node to reclaim memory
systemctl restart elara-node

# Monitor memory growth
watch -n 10 'curl -s http://localhost:9090/metrics | grep elara_memory_usage_bytes'

# If leak confirmed, escalate to engineering
```

**If buffer bloat**:
```bash
# Reduce buffer sizes in configuration
# Edit /etc/elara/config.toml
# [buffers]
# max_send_buffer = 1048576  # 1MB instead of default
# max_recv_buffer = 1048576

systemctl restart elara-node
```

**If state size too large**:
```bash
# Check state metrics
curl http://localhost:9090/metrics | grep elara_state

# May need to:
# - Implement state pruning
# - Reduce retention period
# - Split into multiple sessions
```

**Expected Resolution Time**: 10-20 minutes

**Escalation Criteria**:
- Memory usage > 95% for > 5 minutes
- OOM killer activated
- Memory leak confirmed (continuous growth)
- Multiple nodes affected

---

### Issue 6: Time Drift

**Symptoms**:
- Alert: `TimeDriftExceeded` or `SevereTimeDrift`
- Metric: `abs(elara_time_drift_ms) > 100`
- User impact: Protocol instability, potential message rejection

**Investigation Steps**:

1. **Check time drift metric**:
   ```bash
   # ELARA time drift
   curl http://localhost:9090/metrics | grep elara_time_drift_ms
   
   # Positive = local ahead, Negative = local behind
   ```

2. **Check system time synchronization**:
   ```bash
   # Check chrony/NTP status
   chronyc tracking
   
   # Expected output:
   # Reference ID    : <NTP server>
   # Stratum         : 2 or 3
   # System time     : 0.000000XXX seconds slow/fast of NTP time
   
   # Check time sources
   chronyc sources
   ```

3. **Check peer time drift**:
   ```bash
   # Check time drift on peer nodes
   for peer in peer1 peer2 peer3; do
     echo "=== $peer ==="
     curl http://$peer:9090/metrics | grep elara_time_drift_ms
   done
   ```

4. **Check logs for time-related errors**:
   ```bash
   journalctl -u elara-node | grep -i 'time\|drift\|clock'
   ```

**Resolution Steps**:

**If system time out of sync**:
```bash
# Force time sync
chronyc makestep

# Restart chrony
systemctl restart chronyd

# Wait for sync (30-60 seconds)
sleep 60

# Verify sync
chronyc tracking

# Restart ELARA node
systemctl restart elara-node
```

**If NTP server unreachable**:
```bash
# Check NTP server connectivity
chronyc sources -v

# If unreachable, add backup NTP servers
# Edit /etc/chrony/chrony.conf
# Add:
# server time.google.com iburst
# server time.cloudflare.com iburst

systemctl restart chronyd
```

**If peer nodes have drift**:
```bash
# Check if all nodes use same NTP source
# Ensure consistent time source across cluster

# If one node is outlier, restart it
ssh outlier-node 'systemctl restart chronyd && systemctl restart elara-node'
```

**If drift persists**:
```bash
# Check for virtualization issues
# VMs can have time drift issues

# For VMware: Enable time sync
# For AWS: Use chrony with AWS time service
# For Azure: Use chrony with Azure time service

# Adjust time sync configuration
# Edit /etc/chrony/chrony.conf
# makestep 1 3  # Allow larger steps
# maxdistance 16.0  # Allow more distant sources
```

**Expected Resolution Time**: 10-20 minutes

**Escalation Criteria**:
- Time drift > 500ms persists after sync
- NTP infrastructure issues
- Multiple nodes with severe drift
- Time going backwards (critical)

---

## Emergency Procedures

### Emergency 1: Complete Node Failure

**Symptoms**:
- Node unresponsive to all requests
- Health endpoint timeout
- Process crashed or deadlocked

**Immediate Actions**:

1. **Verify node is truly down**:
   ```bash
   # Try health endpoint (5 second timeout)
   curl -m 5 http://localhost:8080/health
   
   # Check process status
   systemctl status elara-node
   
   # Check if process exists
   ps aux | grep elara-node
   ```

2. **Collect diagnostic information** (if possible):
   ```bash
   # Capture logs before restart
   journalctl -u elara-node -n 1000 > /tmp/elara-crash-$(date +%s).log
   
   # Capture metrics snapshot
   curl http://localhost:9090/metrics > /tmp/elara-metrics-$(date +%s).txt
   
   # Capture core dump (if available)
   ls -lh /var/lib/systemd/coredump/
   ```

3. **Force kill if necessary**:
   ```bash
   # If graceful stop hangs
   systemctl kill -s SIGKILL elara-node
   
   # Verify process is gone
   ps aux | grep elara-node
   ```

4. **Restart node**:
   ```bash
   # Start fresh
   systemctl start elara-node
   
   # Monitor startup
   journalctl -u elara-node -f
   ```

5. **Verify recovery**:
   ```bash
   # Wait for initialization
   sleep 15
   
   # Check health
   curl http://localhost:8080/health | jq
   
   # Check connections
   curl http://localhost:9090/metrics | grep elara_active_connections
   ```

6. **Post-incident**:
   ```bash
   # Analyze crash logs
   # Create incident report
   # Escalate to engineering if crash is reproducible
   ```

**Expected Recovery Time**: 2-5 minutes

**Escalation**: Immediate if crash is reproducible or affects multiple nodes

---

### Emergency 2: Network Partition

**Symptoms**:
- Multiple nodes unable to connect to each other
- Split-brain scenario
- Cluster fragmentation

**Immediate Actions**:

1. **Identify partition scope**:
   ```bash
   # Check connectivity matrix
   for node1 in node1 node2 node3; do
     for node2 in node1 node2 node3; do
       echo "$node1 -> $node2:"
       ssh $node1 "nc -zv -w 2 $node2 7777 2>&1"
     done
   done
   ```

2. **Determine partition groups**:
   ```bash
   # Group nodes by connectivity
   # Example: [node1, node2] can talk, [node3] isolated
   ```

3. **Check network infrastructure**:
   ```bash
   # Check switches, routers, firewalls
   # Check for network maintenance
   # Check for DDoS or network attack
   ```

4. **Isolate affected nodes** (if necessary):
   ```bash
   # Stop nodes in minority partition
   ssh node3 'systemctl stop elara-node'
   
   # Prevents split-brain issues
   ```

5. **Wait for network recovery**:
   ```bash
   # Monitor network connectivity
   watch -n 5 'nc -zv node3 7777'
   ```

6. **Restart isolated nodes**:
   ```bash
   # Once network is restored
   ssh node3 'systemctl start elara-node'
   
   # Verify reconnection
   ssh node3 'curl http://localhost:9090/metrics | grep elara_active_connections'
   ```

**Expected Recovery Time**: Depends on network issue (5-60 minutes)

**Escalation**: Immediate to network team

---

### Emergency 3: Data Corruption

**Symptoms**:
- State divergence alerts
- Inconsistent state across nodes
- Validation errors in logs

**Immediate Actions**:

1. **Identify affected nodes**:
   ```bash
   # Check state divergence metric
   for node in node1 node2 node3; do
     echo "=== $node ==="
     curl http://$node:9090/metrics | grep elara_state_divergence
   done
   ```

2. **Check for corruption indicators**:
   ```bash
   # Look for validation errors
   journalctl -u elara-node | grep -i 'corrupt\|invalid\|checksum\|verify'
   
   # Check for disk errors
   dmesg | grep -i 'error\|fail'
   ```

3. **Isolate corrupted node**:
   ```bash
   # Stop affected node
   systemctl stop elara-node
   
   # Prevent further corruption spread
   ```

4. **Backup current state** (if possible):
   ```bash
   # Backup state directory
   tar -czf /tmp/elara-state-backup-$(date +%s).tar.gz /var/lib/elara/state/
   ```

5. **Restore from backup or resync**:
   ```bash
   # Option 1: Restore from backup
   systemctl stop elara-node
   rm -rf /var/lib/elara/state/*
   tar -xzf /backup/elara-state-latest.tar.gz -C /var/lib/elara/state/
   systemctl start elara-node
   
   # Option 2: Resync from peers (if supported)
   systemctl stop elara-node
   rm -rf /var/lib/elara/state/*
   # Edit config to enable full resync
   systemctl start elara-node
   ```

6. **Verify state consistency**:
   ```bash
   # Wait for resync (may take several minutes)
   sleep 300
   
   # Check state divergence
   curl http://localhost:9090/metrics | grep elara_state_divergence
   
   # Should be 0 or very low
   ```

**Expected Recovery Time**: 10-30 minutes (depending on state size)

**Escalation**: Immediate to engineering team

---

### Emergency 4: Security Incident (Replay Attack)

**Symptoms**:
- Alert: `ReplayAttackDetected` firing
- Metric: `elara_replay_attacks_detected_total` increasing
- Potential malicious activity

**Immediate Actions**:

1. **Verify attack is real**:
   ```bash
   # Check replay attack metric
   curl http://localhost:9090/metrics | grep elara_replay_attacks_detected_total
   
   # Check logs for details
   journalctl -u elara-node | grep -i 'replay'
   ```

2. **Identify attack source**:
   ```bash
   # Check logs for source IP/peer
   journalctl -u elara-node | grep -i 'replay' | grep -oE '[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}'
   
   # Check connection metrics by peer
   curl http://localhost:9090/metrics | grep elara_connections
   ```

3. **Block malicious peer** (if identified):
   ```bash
   # Add firewall rule to block source
   iptables -A INPUT -s <malicious-ip> -j DROP
   
   # Or remove from peer list
   # Edit /etc/elara/config.toml
   systemctl reload elara-node
   ```

4. **Rotate keys** (if compromise suspected):
   ```bash
   # Generate new keys
   elara-keygen --output /etc/elara/new-keys.pem
   
   # Update configuration
   # Edit /etc/elara/config.toml
   # key_file = "/etc/elara/new-keys.pem"
   
   # Restart with new keys
   systemctl restart elara-node
   ```

5. **Notify security team**:
   ```bash
   # Collect evidence
   journalctl -u elara-node > /tmp/security-incident-$(date +%s).log
   
   # Create security incident ticket
   # Include: timestamp, source IP, attack pattern
   ```

6. **Monitor for continued attacks**:
   ```bash
   # Watch replay attack metric
   watch -n 5 'curl -s http://localhost:9090/metrics | grep elara_replay_attacks_detected_total'
   ```

**Expected Resolution Time**: 15-30 minutes

**Escalation**: Immediate to security team

---

## Diagnostic Commands

### Health and Status

```bash
# Overall health status
curl http://localhost:8080/health | jq

# Readiness probe
curl http://localhost:8080/ready

# Liveness probe
curl http://localhost:8080/live

# Process status
systemctl status elara-node

# Process details
ps aux | grep elara-node
```

### Metrics

```bash
# All metrics
curl http://localhost:9090/metrics

# Connection metrics
curl http://localhost:9090/metrics | grep elara_active_connections
curl http://localhost:9090/metrics | grep elara_failed_connections_total

# Message metrics
curl http://localhost:9090/metrics | grep elara_messages_sent_total
curl http://localhost:9090/metrics | grep elara_messages_received_total
curl http://localhost:9090/metrics | grep elara_messages_dropped_total

# Latency metrics
curl http://localhost:9090/metrics | grep elara_message_latency_ms

# Resource metrics
curl http://localhost:9090/metrics | grep elara_memory_usage_bytes
curl http://localhost:9090/metrics | grep elara_cpu_usage_percent

# Protocol metrics
curl http://localhost:9090/metrics | grep elara_time_drift_ms
curl http://localhost:9090/metrics | grep elara_state_divergence

# Security metrics
curl http://localhost:9090/metrics | grep elara_replay_attacks_detected_total
curl http://localhost:9090/metrics | grep elara_authentication_failures_total
```

### Logs

```bash
# Recent logs (last 100 lines)
journalctl -u elara-node -n 100 --no-pager

# Follow logs in real-time
journalctl -u elara-node -f

# Logs since timestamp
journalctl -u elara-node --since "2024-01-15 10:00:00"

# Error logs only
journalctl -u elara-node -p err

# Logs with specific pattern
journalctl -u elara-node | grep -i 'error\|warn\|fail'

# Export logs to file
journalctl -u elara-node -n 10000 > /tmp/elara-logs.txt
```

### Network

```bash
# Active connections
ss -tunap | grep elara

# UDP statistics
netstat -su

# Test connectivity to peer
nc -zv peer1 7777

# Test UDP connectivity
nc -u -zv peer1 7777

# Trace route to peer
traceroute peer1

# Check MTU
ping -M do -s 1472 peer1

# Network interface statistics
ip -s link show

# Firewall rules
iptables -L -n -v
```

### System Resources

```bash
# Memory usage
free -h

# Detailed memory info
cat /proc/meminfo

# CPU usage
top -bn1 | head -20

# Load average
uptime

# Disk usage
df -h

# Disk I/O
iostat -x 1 5

# Process resource usage
pidstat -p $(pgrep elara-node) 1 5
```

### Time Synchronization

```bash
# Chrony tracking status
chronyc tracking

# Time sources
chronyc sources -v

# NTP statistics
chronyc sourcestats

# System time
timedatectl status

# Force time sync
chronyc makestep
```

### Configuration

```bash
# Validate configuration
elara-node --config /etc/elara/config.toml --validate

# Show current configuration
cat /etc/elara/config.toml

# Check configuration syntax
toml-lint /etc/elara/config.toml

# Show systemd service configuration
systemctl cat elara-node
```

---

## Recovery Procedures

### Procedure 1: Recover from Backup

**When to use**: Data corruption, accidental deletion, disaster recovery.

**Prerequisites**:
- Valid backup available
- Backup is recent enough (check timestamp)
- Sufficient disk space for restore

**Steps**:

1. **Stop the node**:
   ```bash
   systemctl stop elara-node
   ```

2. **Backup current state** (just in case):
   ```bash
   mv /var/lib/elara/state /var/lib/elara/state.old.$(date +%s)
   ```

3. **Restore from backup**:
   ```bash
   # Extract backup
   tar -xzf /backup/elara-state-2024-01-15.tar.gz -C /var/lib/elara/
   
   # Verify ownership
   chown -R elara:elara /var/lib/elara/state
   
   # Verify permissions
   chmod -R 750 /var/lib/elara/state
   ```

4. **Start the node**:
   ```bash
   systemctl start elara-node
   
   # Monitor startup
   journalctl -u elara-node -f
   ```

5. **Verify recovery**:
   ```bash
   # Check health
   curl http://localhost:8080/health | jq
   
   # Check state metrics
   curl http://localhost:9090/metrics | grep elara_state
   
   # Monitor for 10 minutes
   ```

**Expected Duration**: 10-20 minutes

**Rollback**: If restore fails, restore the old state:
```bash
systemctl stop elara-node
rm -rf /var/lib/elara/state
mv /var/lib/elara/state.old.* /var/lib/elara/state
systemctl start elara-node
```

---

### Procedure 2: Rolling Restart of Cluster

**When to use**: Configuration updates, version upgrades, routine maintenance.

**Prerequisites**:
- Cluster has ≥ 3 nodes
- Load is distributed across nodes
- No ongoing incidents

**Steps**:

1. **Verify cluster health**:
   ```bash
   for node in node1 node2 node3; do
     echo "=== $node ==="
     curl http://$node:8080/health | jq .status
   done
   
   # All should be "healthy"
   ```

2. **Restart first node**:
   ```bash
   ssh node1 'systemctl restart elara-node'
   
   # Wait for startup (30 seconds)
   sleep 30
   
   # Verify health
   curl http://node1:8080/health | jq .status
   ```

3. **Wait for stabilization**:
   ```bash
   # Wait 2 minutes for connections to re-establish
   sleep 120
   
   # Check connections
   curl http://node1:9090/metrics | grep elara_active_connections
   ```

4. **Restart remaining nodes** (one at a time):
   ```bash
   for node in node2 node3; do
     echo "=== Restarting $node ==="
     ssh $node 'systemctl restart elara-node'
     sleep 30
     curl http://$node:8080/health | jq .status
     sleep 120
   done
   ```

5. **Verify cluster health**:
   ```bash
   for node in node1 node2 node3; do
     echo "=== $node ==="
     curl http://$node:8080/health | jq
     curl http://$node:9090/metrics | grep elara_active_connections
   done
   ```

**Expected Duration**: 15-20 minutes (for 3-node cluster)

**Rollback**: If issues occur, restart affected nodes with previous configuration.

---

### Procedure 3: Emergency Cluster Shutdown

**When to use**: Critical security incident, infrastructure maintenance, emergency.

**Prerequisites**:
- Authorization from engineering lead
- Notification sent to stakeholders
- Backup plan for service continuity

**Steps**:

1. **Notify stakeholders**:
   ```bash
   # Send notification via PagerDuty/Slack
   # Include: reason, expected duration, impact
   ```

2. **Stop all nodes gracefully**:
   ```bash
   for node in node1 node2 node3; do
     echo "=== Stopping $node ==="
     ssh $node 'systemctl stop elara-node'
   done
   ```

3. **Verify all nodes stopped**:
   ```bash
   for node in node1 node2 node3; do
     echo "=== $node ==="
     ssh $node 'systemctl status elara-node'
   done
   
   # All should show "inactive (dead)"
   ```

4. **Perform emergency maintenance**:
   ```bash
   # Execute required maintenance tasks
   ```

5. **Restart cluster**:
   ```bash
   # Start all nodes simultaneously
   for node in node1 node2 node3; do
     ssh $node 'systemctl start elara-node' &
   done
   wait
   
   # Wait for initialization
   sleep 30
   ```

6. **Verify cluster recovery**:
   ```bash
   for node in node1 node2 node3; do
     echo "=== $node ==="
     curl http://$node:8080/health | jq .status
   done
   ```

**Expected Duration**: Depends on maintenance (10-60 minutes)

---

## Escalation Paths

### Level 1: On-Call Engineer (You)

**Responsibilities**:
- Initial triage and diagnosis
- Execute standard procedures from this runbook
- Resolve common issues independently
- Escalate if unable to resolve within 30 minutes

**Escalation Criteria**:
- Issue not covered in runbook
- Multiple nodes affected
- Security incident
- Data loss risk
- Unable to resolve within 30 minutes

---

### Level 2: Senior On-Call / Engineering Lead

**Contact**: PagerDuty escalation policy

**Responsibilities**:
- Complex troubleshooting
- Non-standard recovery procedures
- Coordination with other teams
- Decision on emergency changes

**Escalation Criteria**:
- Cluster-wide outage
- Data corruption
- Performance degradation > 1 hour
- Security breach confirmed

---

### Level 3: Engineering Team / Architect

**Contact**: Engineering lead + team channel

**Responsibilities**:
- Code-level debugging
- Architecture decisions
- Emergency patches
- Post-incident analysis

**Escalation Criteria**:
- Bug in ELARA Protocol code
- Design flaw identified
- Need for emergency patch
- Incident requires code changes

---

### Level 4: Security Team

**Contact**: Security team on-call

**Responsibilities**:
- Security incident response
- Forensic analysis
- Threat assessment
- Compliance reporting

**Escalation Criteria**:
- Confirmed security breach
- Replay attacks persist
- Authentication bypass
- Data exfiltration suspected

---

## Appendix

### A. Configuration Reference

See `docs/operations/CONFIGURATION.md` for detailed configuration options.

### B. Metrics Reference

See `config/ALERTS_README.md` for complete metrics documentation.

### C. Alert Reference

See `config/ALERTS_README.md` for alert definitions and response procedures.

### D. Deployment Guide

See `docs/operations/DEPLOYMENT.md` for deployment procedures.

### E. Monitoring Guide

See `docs/operations/MONITORING.md` for monitoring setup and dashboard configuration.

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2024-01-15 | Operations Team | Initial production release |

---

**End of Runbook**
