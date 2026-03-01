# ELARA Benchmark Suite

Production-grade benchmark suite for the ELARA Protocol using [criterion](https://github.com/bheisler/criterion.rs) for statistical analysis and performance tracking.

## Overview

This crate provides comprehensive benchmarks for all critical ELARA Protocol components:

- **Wire Protocol**: Frame encoding/decoding, header parsing, packet serialization
- **Cryptographic Operations**: Encryption/decryption, signatures, key derivation
- **State Reconciliation**: Version vector operations, causality checking, state merge
- **Time Engine**: Time classification, clock operations, drift estimation

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench --package elara-bench
```

### Run Specific Benchmark Suite

```bash
# Wire protocol benchmarks
cargo bench --package elara-bench --bench wire_protocol

# Crypto benchmarks
cargo bench --package elara-bench --bench crypto_operations

# State benchmarks
cargo bench --package elara-bench --bench state_reconciliation

# Time engine benchmarks
cargo bench --package elara-bench --bench time_engine
```

### Run Specific Benchmark

```bash
cargo bench --package elara-bench --bench wire_protocol -- frame_encoding
```

## Benchmark Results

Criterion generates detailed HTML reports in `target/criterion/`. Open `target/criterion/report/index.html` in a browser to view:

- Statistical analysis (mean, median, std dev)
- Performance trends over time
- Regression detection
- Detailed plots and charts

## Performance Baselines

### Wire Protocol

Expected performance on reference hardware (Intel i7-10700K, 32GB RAM):

| Operation | Payload Size | Throughput | Latency |
|-----------|--------------|------------|---------|
| Frame Encoding | 64B | ~2M ops/sec | ~500ns |
| Frame Encoding | 1KB | ~500K ops/sec | ~2μs |
| Frame Encoding | 16KB | ~50K ops/sec | ~20μs |
| Frame Decoding | 64B | ~1.5M ops/sec | ~650ns |
| Frame Decoding | 1KB | ~400K ops/sec | ~2.5μs |
| Frame Decoding | 16KB | ~40K ops/sec | ~25μs |
| Header Parse | - | ~10M ops/sec | ~100ns |
| Header Serialize | - | ~8M ops/sec | ~125ns |

### Cryptographic Operations

| Operation | Payload Size | Throughput | Latency |
|-----------|--------------|------------|---------|
| Identity Generation | - | ~5K ops/sec | ~200μs |
| Encryption | 64B | ~200K ops/sec | ~5μs |
| Encryption | 1KB | ~100K ops/sec | ~10μs |
| Encryption | 16KB | ~10K ops/sec | ~100μs |
| Decryption | 64B | ~180K ops/sec | ~5.5μs |
| Decryption | 1KB | ~90K ops/sec | ~11μs |
| Decryption | 16KB | ~9K ops/sec | ~110μs |
| Sign | - | ~50K ops/sec | ~20μs |
| Verify | - | ~20K ops/sec | ~50μs |
| Key Derivation | - | ~10K ops/sec | ~100μs |

### State Reconciliation

| Operation | Event Count | Throughput | Latency |
|-----------|-------------|------------|---------|
| VV Increment | - | ~50M ops/sec | ~20ns |
| VV Get | 100 entries | ~20M ops/sec | ~50ns |
| VV Merge | 10 events | ~5M ops/sec | ~200ns |
| VV Merge | 100 events | ~1M ops/sec | ~1μs |
| VV Merge | 1000 events | ~100K ops/sec | ~10μs |
| Happens-Before | 10 events | ~10M ops/sec | ~100ns |
| Happens-Before | 100 events | ~2M ops/sec | ~500ns |
| Happens-Before | 1000 events | ~200K ops/sec | ~5μs |

### Time Engine

| Operation | Throughput | Latency |
|-----------|------------|---------|
| Engine Tick | ~20M ops/sec | ~50ns |
| Time Classify | ~10M ops/sec | ~100ns |
| Perceptual Clock Tick | ~50M ops/sec | ~20ns |
| State Clock Advance | ~10M ops/sec | ~100ns |
| Network Model Update | ~5M ops/sec | ~200ns |
| Drift Estimation | ~2M ops/sec | ~500ns |

**Note**: These are approximate baseline numbers. Actual performance varies based on hardware, system load, and configuration.

## CI Integration

Benchmarks run automatically in CI on:

- Push to `main` or `develop` branches (for performance-critical paths)
- Pull requests affecting core components
- Weekly schedule (every Monday at 2 AM UTC)
- Manual workflow dispatch

### Regression Detection

The CI workflow compares benchmark results against the baseline and detects regressions >10%. If a significant regression is detected, the workflow fails and requires investigation.

### Baseline Updates

When changes are merged to `main`, the benchmark results become the new baseline for future comparisons.

## Configuration

Benchmarks can be configured via `BenchmarkConfig`:

```rust
use elara_bench::BenchmarkConfig;

// Quick benchmarks for CI
let config = BenchmarkConfig::quick();

// Thorough benchmarks for baseline establishment
let config = BenchmarkConfig::thorough();

// Custom configuration
let config = BenchmarkConfig {
    warmup_iterations: 100,
    warmup_time: Duration::from_secs(3),
    measurement_time: Duration::from_secs(5),
    sample_size: 100,
};
```

## Interpreting Results

### Statistical Measures

- **Mean**: Average execution time
- **Median**: Middle value (50th percentile)
- **Std Dev**: Variability in measurements
- **MAD**: Median Absolute Deviation (robust measure of variability)

### Performance Trends

Criterion tracks performance over time and generates plots showing:

- Performance changes across commits
- Regression/improvement detection
- Statistical confidence intervals

### Regression Threshold

A regression is considered significant if:

- Performance degrades by >10%
- Change is statistically significant (p < 0.05)
- Consistent across multiple runs

## Best Practices

### Running Benchmarks

1. **Minimize system load**: Close unnecessary applications
2. **Consistent environment**: Use the same hardware for comparisons
3. **Multiple runs**: Run benchmarks multiple times for reliability
4. **Warm cache**: Criterion handles warmup automatically

### Investigating Regressions

1. **Verify reproducibility**: Run benchmarks multiple times
2. **Check recent changes**: Review commits since last baseline
3. **Profile hot paths**: Use profiling tools (perf, flamegraph)
4. **Compare assembly**: Check for unexpected code generation changes

### Adding New Benchmarks

1. Add benchmark function to appropriate file
2. Use `black_box()` to prevent compiler optimizations
3. Set appropriate throughput for size-based benchmarks
4. Document expected performance in this README

## Profiling

For detailed profiling, use:

```bash
# Generate flamegraph
cargo flamegraph --bench wire_protocol

# Use perf for detailed analysis
perf record --call-graph dwarf cargo bench --bench crypto_operations
perf report
```

## Contributing

When adding new features:

1. Add corresponding benchmarks
2. Run benchmarks before and after changes
3. Document expected performance
4. Update baseline if intentional performance changes

## License

MIT OR Apache-2.0
