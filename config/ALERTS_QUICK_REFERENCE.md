# ELARA Protocol Alerts - Quick Reference

This is a quick reference guide for on-call engineers responding to ELARA Protocol alerts.

## Critical Alerts (Immediate Action Required)

### NodeUnhealthy
**Threshold**: Health status != 1 for 2 minutes  
**Quick Fix**:
```bash
# Check health endpoint
curl http://node:port/health

# Check logs
tail -f /var/log/elara/node.log

# Restart if necessary
systemctl restart elara-node
```

### MessageProcessingStalled
**Threshold**: No messages sent/received for 3 minutes  
**Quick Fix**:
```bash
# Check if process is running
ps aux | grep elara

# Check network connectivity
ping peer-node

# Check for deadlocks in logs
grep -i "deadlock\|hang\|stuck" /var/log/elara/node.log

# Restart node
systemctl restart elara-node
```

### CriticalMemoryUsage
**Threshold**: >1.95GB (97.5% of 2GB) for 2 minutes  
**Quick Fix**:
```bash
# Check memory usage
free -h
ps aux --sort=-%mem | head

# Capture heap dump (if available)
kill -USR1 $(pidof elara-node)

# Drain traffic and restart
systemctl restart elara-node
```

### SevereTimeDrift
**Threshold**: >500ms drift for 2 minutes  
**Quick Fix**:
```bash
# Check NTP status
ntpq -p
timedatectl status

# Force NTP sync
sudo systemctl restart ntp
sudo ntpdate -s time.nist.gov

# Verify sync
timedatectl status
```

### ReplayAttackDetected
**Threshold**: Any replay attacks for 1 minute  
**Quick Fix**:
```bash
# Identify attacker from logs
grep "replay attack" /var/log/elara/node.log

# Block malicious peer
# Add to firewall or peer blocklist

# Rotate session keys
# Follow key rotation procedure

# Alert security team immediately
```

### HighAuthenticationFailureRate
**Threshold**: >1 failure/sec for 3 minutes  
**Quick Fix**:
```bash
# Check auth logs
grep "authentication failed" /var/log/elara/node.log

# Identify source
# Check if legitimate peer or attack

# Block if attack
# Add to firewall rules

# Verify credentials if legitimate peer
```

## Warning Alerts (Investigation Needed)

### HighMessageDropRate
**Threshold**: >1% drop rate for 5 minutes  
**Investigation**:
```bash
# Check drop rate
curl http://node:port/metrics | grep elara_messages_dropped

# Check network
ping peer-node
mtr peer-node

# Check resources
top
free -h

# Check queue depths
curl http://node:port/metrics | grep queue
```

### HighLatency
**Threshold**: P95 >1000ms for 5 minutes  
**Investigation**:
```bash
# Check latency metrics
curl http://node:port/metrics | grep latency

# Check network latency
ping peer-node
mtr peer-node

# Check CPU
top
mpstat 1 10

# Check for slow peers
# Review peer latency metrics
```

### HighMemoryUsage
**Threshold**: >1.8GB (90% of 2GB) for 5 minutes  
**Investigation**:
```bash
# Check memory usage
free -h
ps aux --sort=-%mem | head

# Check memory trends
curl http://node:port/metrics | grep memory

# Check for leaks
# Review memory usage over time in Grafana

# Plan restart during maintenance window
```

### HighCPUUsage
**Threshold**: >80% CPU for 10 minutes  
**Investigation**:
```bash
# Check CPU usage
top
mpstat 1 10

# Check CPU trends
curl http://node:port/metrics | grep cpu

# Profile if needed
perf top -p $(pidof elara-node)

# Check message rate
curl http://node:port/metrics | grep messages
```

### TimeDriftExceeded
**Threshold**: >100ms drift for 5 minutes  
**Investigation**:
```bash
# Check time drift
curl http://node:port/metrics | grep time_drift

# Check NTP status
ntpq -p
timedatectl status

# Check NTP servers
cat /etc/ntp.conf

# Verify NTP connectivity
ntpdate -q time.nist.gov
```

### StateDivergence
**Threshold**: >10 divergent events for 5 minutes  
**Investigation**:
```bash
# Check state divergence
curl http://node:port/metrics | grep state_divergence

# Check peer connectivity
ping peer-node

# Check state sync logs
grep "state sync" /var/log/elara/node.log

# Check message drop rate
curl http://node:port/metrics | grep dropped
```

### LowConnectionCount
**Threshold**: <2 active connections for 5 minutes  
**Investigation**:
```bash
# Check connection count
curl http://node:port/metrics | grep active_connections

# Check peer connectivity
ping peer-node-1
ping peer-node-2

# Check connection logs
grep "connection" /var/log/elara/node.log

# Check firewall rules
sudo iptables -L
```

### HighConnectionFailureRate
**Threshold**: >0.1 failures/sec for 5 minutes  
**Investigation**:
```bash
# Check failure rate
curl http://node:port/metrics | grep failed_connections

# Check connection error logs
grep "connection failed" /var/log/elara/node.log

# Check network
ping peer-node
telnet peer-node port

# Check TLS certificates
openssl s_client -connect peer-node:port
```

## Common Commands

### Check Node Status
```bash
# Health check
curl http://node:port/health

# Metrics
curl http://node:port/metrics

# Logs
tail -f /var/log/elara/node.log

# Process status
systemctl status elara-node
```

### Check Metrics
```bash
# All metrics
curl http://node:port/metrics

# Specific metric
curl http://node:port/metrics | grep metric_name

# Connection metrics
curl http://node:port/metrics | grep connection

# Message metrics
curl http://node:port/metrics | grep message

# Resource metrics
curl http://node:port/metrics | grep -E "memory|cpu"
```

### Restart Node
```bash
# Graceful restart
systemctl restart elara-node

# Check status
systemctl status elara-node

# Verify health
curl http://node:port/health

# Check logs
tail -f /var/log/elara/node.log
```

### Check Network
```bash
# Ping peer
ping peer-node

# Trace route
mtr peer-node

# Check port
telnet peer-node port
nc -zv peer-node port

# Check DNS
nslookup peer-node
dig peer-node
```

### Check Resources
```bash
# Memory
free -h
ps aux --sort=-%mem | head

# CPU
top
mpstat 1 10

# Disk
df -h
iostat -x 1 10

# Network
iftop
nethogs
```

## Escalation

### Level 1: On-Call Engineer
- All critical alerts
- Initial investigation
- Immediate mitigation

### Level 2: Engineering Lead
- Escalate after 30 minutes if unresolved
- Complex issues requiring deep expertise
- Coordination of multiple engineers

### Level 3: Engineering Manager
- Escalate after 1 hour if unresolved
- Major incidents affecting multiple nodes
- Decision on emergency procedures

### Level 4: CTO
- Major outages
- Security incidents
- Business-critical decisions

## Contact Information

- **On-Call Engineer**: Check PagerDuty schedule
- **Engineering Lead**: [Contact info]
- **Engineering Manager**: [Contact info]
- **Security Team**: [Contact info]
- **Incident Channel**: #elara-incidents (Slack)

## Useful Links

- [Full Alert Documentation](ALERTS_README.md)
- [Operational Runbook](../docs/operations/RUNBOOK.md)
- [Monitoring Guide](../docs/operations/MONITORING.md)
- [Prometheus Dashboard](http://prometheus:9090)
- [Grafana Dashboard](http://grafana:3000)
- [Alertmanager](http://alertmanager:9093)

## Tips for On-Call

1. **Acknowledge quickly**: Acknowledge alerts within 5 minutes
2. **Assess impact**: Determine if service is degraded
3. **Communicate**: Update incident channel
4. **Investigate systematically**: Follow the investigation steps
5. **Mitigate first**: Stop the bleeding before finding root cause
6. **Document**: Keep notes for post-mortem
7. **Escalate early**: Don't wait too long to escalate
8. **Stay calm**: Panic doesn't help

## Common Pitfalls

- **Don't restart blindly**: Investigate first, restart as last resort
- **Don't ignore warnings**: Warnings often precede critical issues
- **Don't forget to communicate**: Keep team informed
- **Don't skip documentation**: Document for post-mortem
- **Don't work alone**: Ask for help when needed

---

**Keep this guide handy during on-call shifts!**
