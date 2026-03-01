# ELARA Protocol Performance Baselines

## Overview

This document establishes performance baselines for the ELARA Protocol. These baselines serve as reference points for:
- Performance regression detection in CI
- Capacity planning for deployments
- Performance optimization efforts
- Production monitoring thresholds

**Last Updated**: [To be updated when run on reference hardware]

## Reference Hardware Specifications

**Note**: The baseline numbers in this document are placeholders and should be updated when benchmarks are run on actual reference hardware.

### Recommended Reference Hardware
- **CPU**: Intel i7-10700K or AMD Ryzen 7 5800X (8 cores, 16 threads)
- **RAM**: 32GB DDR4-3200
- **Storage**: NVMe SSD
- **OS**: Linux (Ubuntu 22.04 LTS or similar)
- **Rust**: Latest stable version

### Test Environment Configuration
- **System Load**: Minimal (no other intensive processes)
- **CPU Governor**: Performance mode
- **Turbo Boost**: Enabled
- **Hyperthreading**: Enabled
- **Benchmark Mode**: Release build with optimizations

## Benchmark Baselines

### Wire Protocol Performance

Frame encoding and decoding performance for various payload sizes.

| Operation | Payload Size | Throughput | Latency (Mean) | Latency (P95) | Notes |
|-----------|--------------|------------|----------------|---------------|-------|
| Frame Encoding | 64B | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1.5M ops/sec |
| Frame Encoding | 256B | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| Frame Encoding | 1KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >500K ops/sec |
| Frame Encoding | 4KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >200K ops/sec |
| Frame Encoding | 16KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >50K ops/sec |
| Frame Decoding | 64B | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| Frame Decoding | 256B | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >800K ops/sec |
| Frame Decoding | 1KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >400K ops/sec |
| Frame Decoding | 4KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >150K ops/sec |
| Frame Decoding | 16KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >40K ops/sec |
| Header Parse | - | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |
| Header Serialize | - | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |

**Expected Scaling Behavior**:
- Encoding/decoding throughput decreases linearly with payload size
- Small payloads (64-256B) are dominated by fixed overhead
- Large payloads (4KB+) are dominated by memory copy operations

### Cryptographic Operations Performance

Performance of cryptographic primitives used in the ELARA Protocol.

| Operation | Payload Size | Throughput | Latency (Mean) | Latency (P95) | Notes |
|-----------|--------------|------------|----------------|---------------|-------|
| Identity Generation | - | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >5K ops/sec |
| Encryption | 64B | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >100K ops/sec |
| Encryption | 256B | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >80K ops/sec |
| Encryption | 1KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >50K ops/sec |
| Encryption | 4KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >20K ops/sec |
| Encryption | 16KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >5K ops/sec |
| Decryption | 64B | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >100K ops/sec |
| Decryption | 256B | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >80K ops/sec |
| Decryption | 1KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >50K ops/sec |
| Decryption | 4KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >20K ops/sec |
| Decryption | 16KB | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >5K ops/sec |
| Sign | - | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >20K ops/sec |
| Verify | - | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >10K ops/sec |
| Key Derivation | - | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >10K ops/sec |
| Session Key Gen | - | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >10K ops/sec |

**Expected Scaling Behavior**:
- Encryption/decryption throughput decreases with payload size
- Signature operations are constant-time (independent of payload)
- Key generation operations have fixed cost

**Security Notes**:
- All cryptographic operations use constant-time implementations
- Timing measurements should show minimal variance across different inputs
- Any significant timing variance may indicate timing attack vulnerabilities

### State Reconciliation Performance

Performance of version vector operations and state merge algorithms.

| Operation | Event Count | Throughput | Latency (Mean) | Latency (P95) | Notes |
|-----------|-------------|------------|----------------|---------------|-------|
| VV Increment | - | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >10M ops/sec |
| VV Get | 100 entries | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >10M ops/sec |
| VV Merge | 10 events | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| VV Merge | 100 events | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >500K ops/sec |
| VV Merge | 1000 events | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >50K ops/sec |
| Happens-Before | 10 events | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |
| Happens-Before | 100 events | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| Happens-Before | 1000 events | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >100K ops/sec |
| Concurrent Check | 10 events | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |
| Concurrent Check | 100 events | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| Concurrent Check | 1000 events | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >100K ops/sec |

**Expected Scaling Behavior**:
- VV operations scale linearly with event count
- Small event counts (10-100) have minimal overhead
- Large event counts (1000+) show linear degradation
- Merge operations are more expensive than read operations

**Design Requirements**:
- State merge must complete in <10ms for 100 events (Requirement 9.3)
- Operations must be deterministic and commutative
- Memory usage should scale linearly with event count

### Time Engine Performance

Performance of time classification and clock operations.

| Operation | Throughput | Latency (Mean) | Latency (P95) | Notes |
|-----------|------------|----------------|---------------|-------|
| Engine Tick | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >10M ops/sec |
| Time Classify | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |
| Perceptual Clock Tick | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >20M ops/sec |
| Perceptual Clock Read | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >50M ops/sec |
| State Clock Advance | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |
| State Clock Blend | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >5M ops/sec |
| State Clock Read | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >50M ops/sec |
| Network Model Update | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >2M ops/sec |
| Drift Estimation | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| Horizon Adaptation | **[TBD]** ops/sec | **[TBD]** ns | **[TBD]** ns | Target: >1M ops/sec |
| Time Sync Workflow | **[TBD]** ops/sec | **[TBD]** μs | **[TBD]** μs | Target: >100K ops/sec |

**Expected Scaling Behavior**:
- Clock read operations are extremely fast (lock-free)
- Clock update operations have moderate overhead
- Full sync workflow includes multiple operations
- Time classification is constant-time

**Design Requirements**:
- Time operations must be monotonic
- Clock drift must be bounded
- Time classification must be deterministic

## Load Test Baselines

### Small Deployment (10 Nodes)

**Configuration**:
- Nodes: 10
- Connections per node: 5
- Message rate: 100 msg/sec
- Duration: 60 seconds
- Total connections: 50

**Expected Performance**:

| Metric | Target Value | Acceptable Range | Notes |
|--------|--------------|------------------|-------|
| Total Messages | 6,000 | 5,800 - 6,200 | 100 msg/sec × 60s |
| Success Rate | >99% | 98% - 100% | <1% message loss |
| Throughput | ~100 msg/sec | 95 - 105 msg/sec | Sustained rate |
| Avg Latency | **[TBD]** ms | **[TBD]** ms | Target: <20ms |
| P50 Latency | **[TBD]** ms | **[TBD]** ms | Target: <15ms |
| P95 Latency | **[TBD]** ms | **[TBD]** ms | Target: <50ms |
| P99 Latency | **[TBD]** ms | **[TBD]** ms | Target: <100ms |
| Max Latency | **[TBD]** ms | **[TBD]** ms | Target: <200ms |
| Peak Memory | **[TBD]** MB | **[TBD]** MB | Target: <500MB |
| Avg CPU Usage | **[TBD]** % | **[TBD]** % | Target: <20% |

**Use Cases**:
- Development and testing
- Small team collaboration (5-10 users)
- Initial deployment validation
- CI/CD integration testing

### Medium Deployment (100 Nodes)

**Configuration**:
- Nodes: 100
- Connections per node: 10
- Message rate: 1,000 msg/sec
- Duration: 300 seconds (5 minutes)
- Total connections: 1,000

**Expected Performance**:

| Metric | Target Value | Acceptable Range | Notes |
|--------|--------------|------------------|-------|
| Total Messages | 300,000 | 290,000 - 310,000 | 1000 msg/sec × 300s |
| Success Rate | >98% | 96% - 100% | <2% message loss |
| Throughput | ~1,000 msg/sec | 950 - 1,050 msg/sec | Sustained rate |
| Avg Latency | **[TBD]** ms | **[TBD]** ms | Target: <50ms |
| P50 Latency | **[TBD]** ms | **[TBD]** ms | Target: <40ms |
| P95 Latency | **[TBD]** ms | **[TBD]** ms | Target: <100ms |
| P99 Latency | **[TBD]** ms | **[TBD]** ms | Target: <200ms |
| Max Latency | **[TBD]** ms | **[TBD]** ms | Target: <500ms |
| Peak Memory | **[TBD]** GB | **[TBD]** GB | Target: <4GB |
| Avg CPU Usage | **[TBD]** % | **[TBD]** % | Target: <50% |

**Use Cases**:
- Production baseline deployment
- Multi-team collaboration (50-100 users)
- Regional deployment
- Performance validation

**Design Requirements**:
- Must handle medium deployment without message loss (Requirement 9.4)
- P95 latency must remain below 100ms under normal load (Requirement 9.5)

### Large Deployment (1000 Nodes)

**Configuration**:
- Nodes: 1,000
- Connections per node: 20
- Message rate: 10,000 msg/sec
- Duration: 600 seconds (10 minutes)
- Total connections: 20,000

**Expected Performance**:

| Metric | Target Value | Acceptable Range | Notes |
|--------|--------------|------------------|-------|
| Total Messages | 6,000,000 | 5,700,000 - 6,300,000 | 10000 msg/sec × 600s |
| Success Rate | >95% | 90% - 100% | <5% message loss |
| Throughput | ~10,000 msg/sec | 9,000 - 11,000 msg/sec | Sustained rate |
| Avg Latency | **[TBD]** ms | **[TBD]** ms | Target: <100ms |
| P50 Latency | **[TBD]** ms | **[TBD]** ms | Target: <80ms |
| P95 Latency | **[TBD]** ms | **[TBD]** ms | Target: <200ms |
| P99 Latency | **[TBD]** ms | **[TBD]** ms | Target: <500ms |
| Max Latency | **[TBD]** ms | **[TBD]** ms | Target: <1000ms |
| Peak Memory | **[TBD]** GB | **[TBD]** GB | Target: <16GB |
| Avg CPU Usage | **[TBD]** % | **[TBD]** % | Target: <80% |

**Use Cases**:
- Large-scale production deployment
- Enterprise-wide collaboration (500-1000 users)
- Global deployment
- Stress testing and capacity planning

**Design Requirements**:
- System must degrade gracefully when approaching resource limits (Requirement 9.6)
- Must maintain core functionality under high load
- Should provide clear signals when approaching capacity

## Baseline Establishment Procedure

To establish or update baselines:

### 1. Prepare Reference Hardware

```bash
# Ensure system is idle
top  # Verify no intensive processes running

# Set CPU governor to performance mode
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable CPU frequency scaling (optional, for most consistent results)
sudo cpupower frequency-set -g performance

# Clear system caches
sync && echo 3 | sudo tee /proc/sys/vm/drop_caches
```

### 2. Run Benchmarks

```bash
# Build in release mode
cargo build --release --package elara-bench

# Run all benchmarks
cargo bench --package elara-bench

# Results are saved to target/criterion/
```

### 3. Run Load Tests

```bash
# Build in release mode
cargo build --release --package elara-loadtest

# Run small deployment
cargo run --release -p elara-loadtest --example small_deployment > small_results.txt

# Run medium deployment
cargo run --release -p elara-loadtest --example medium_deployment > medium_results.txt

# Run large deployment (requires sufficient resources)
cargo run --release -p elara-loadtest --example large_deployment > large_results.txt
```

### 4. Extract and Document Results

```bash
# Extract benchmark results from criterion HTML reports
# Located in target/criterion/*/report/index.html

# Extract load test results from output files
cat small_results.txt
cat medium_results.txt
cat large_results.txt
```

### 5. Update This Document

- Replace **[TBD]** placeholders with actual measured values
- Update "Last Updated" date at the top
- Document any deviations from expected performance
- Add notes about environmental factors if relevant

### 6. Commit Baseline Updates

```bash
git add docs/performance/BASELINES.md
git commit -m "Update performance baselines with measurements from [hardware spec]"
```

## Performance Regression Detection

### CI Integration

Benchmarks run automatically in CI and compare against these baselines:

- **Regression Threshold**: >10% performance degradation
- **Statistical Significance**: p < 0.05
- **Action**: CI fails and requires investigation

### Manual Regression Checks

```bash
# Compare current performance against baseline
cargo bench --package elara-bench -- --save-baseline main

# Make changes...

# Compare against baseline
cargo bench --package elara-bench -- --baseline main
```

### Investigating Regressions

When a regression is detected:

1. **Verify Reproducibility**: Run benchmarks multiple times
2. **Check Recent Changes**: Review commits since last baseline
3. **Profile Hot Paths**: Use profiling tools (perf, flamegraph)
4. **Compare Assembly**: Check for unexpected code generation changes
5. **Document Findings**: Update this document with analysis

## Performance Tuning Guidelines

### Wire Protocol Optimization

- Use zero-copy serialization where possible
- Batch small messages to amortize overhead
- Consider compression for large payloads (>4KB)
- Pre-allocate buffers for known message sizes

### Cryptographic Optimization

- Reuse session keys to avoid key derivation overhead
- Batch signature operations when possible
- Use hardware acceleration (AES-NI) when available
- Consider async crypto for non-blocking operations

### State Reconciliation Optimization

- Limit version vector size through garbage collection
- Use incremental merge for large state updates
- Cache causality check results for frequently compared events
- Consider delta-based state sync for large states

### Time Engine Optimization

- Use lock-free atomic operations for clock reads
- Batch time updates to reduce synchronization overhead
- Adjust horizon adaptation parameters based on network conditions
- Use coarse-grained time for non-critical operations

## Resource Requirements

### Minimum Requirements

Based on small deployment baseline:

- **CPU**: 2 cores (4 threads recommended)
- **RAM**: 2GB (4GB recommended)
- **Network**: 10 Mbps (100 Mbps recommended)
- **Disk**: 10GB for logs and metrics

### Recommended Requirements

Based on medium deployment baseline:

- **CPU**: 4 cores (8 threads recommended)
- **RAM**: 8GB (16GB recommended)
- **Network**: 100 Mbps (1 Gbps recommended)
- **Disk**: 50GB for logs and metrics

### Large-Scale Requirements

Based on large deployment baseline:

- **CPU**: 8+ cores (16+ threads recommended)
- **RAM**: 16GB (32GB recommended)
- **Network**: 1 Gbps (10 Gbps recommended)
- **Disk**: 100GB+ for logs and metrics

### Scaling Guidelines

**Per 100 Additional Nodes**:
- CPU: +1 core
- RAM: +2GB
- Network: +100 Mbps
- Disk: +10GB

**Per 1000 Additional Messages/sec**:
- CPU: +5% utilization
- RAM: +500MB
- Network: +10 Mbps (depends on message size)

## Monitoring Thresholds

Based on these baselines, recommended monitoring thresholds:

### Performance Alerts

- **High Latency**: P95 latency >2× baseline for 5 minutes
- **Low Throughput**: Throughput <80% of baseline for 5 minutes
- **High Message Loss**: Message loss >5% for 5 minutes

### Resource Alerts

- **High CPU**: CPU usage >80% for 5 minutes
- **High Memory**: Memory usage >90% of limit for 5 minutes
- **Disk Space**: Disk usage >85% of capacity

### Protocol Alerts

- **Time Drift**: Time drift >100ms for 5 minutes
- **State Divergence**: State divergence detected for 2 minutes
- **Connection Loss**: Active connections <50% of expected for 2 minutes

## Conclusion

These baselines provide reference points for:
- ✅ Performance regression detection in CI
- ✅ Capacity planning for deployments
- ✅ Performance optimization efforts
- ✅ Production monitoring thresholds

**Action Required**: Run benchmarks and load tests on reference hardware to replace **[TBD]** placeholders with actual measurements.

## References

- [Benchmark Suite Documentation](../../crates/elara-bench/README.md)
- [Load Testing Framework Documentation](../../crates/elara-loadtest/README.md)
- [Performance Guide](./PERFORMANCE_GUIDE.md)
- [Requirements Document](../../.kiro/specs/production-readiness-implementation/requirements.md)
