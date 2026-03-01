# ELARA Load Testing Framework

Comprehensive load testing framework for the ELARA Protocol to validate performance under realistic deployment scenarios with multiple nodes.

## Features

- **Realistic Node Simulation**: Simulate multiple ELARA Protocol nodes in a single process
- **Flexible Configuration**: Customize node count, connection topology, message rates, and test duration
- **Comprehensive Metrics**: Track throughput, latency percentiles (p50, p95, p99), success/failure rates
- **Predefined Scenarios**: Ready-to-use configurations for small (10), medium (100), and large (1000) node deployments
- **CI Integration**: Automated nightly load tests with result artifact storage
- **Production-Grade**: Accurate latency measurement, statistical analysis, and detailed reporting

## Quick Start

### Running Predefined Scenarios

```bash
# Small deployment (10 nodes, 60 seconds)
cargo run --release -p elara-loadtest --example small_deployment

# Medium deployment (100 nodes, 5 minutes)
cargo run --release -p elara-loadtest --example medium_deployment

# Large deployment (1000 nodes, 10 minutes)
cargo run --release -p elara-loadtest --example large_deployment
```

### Custom Load Test

```rust
use elara_loadtest::{LoadTestScenario, LoadTestConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LoadTestConfig {
        num_nodes: 50,
        num_connections_per_node: 8,
        message_rate_per_second: 500,
        test_duration: Duration::from_secs(120),
        ramp_up_duration: Duration::from_secs(20),
    };
    
    let mut scenario = LoadTestScenario::new(config);
    let result = scenario.run().await?;
    
    println!("{}", result.report());
    
    Ok(())
}
```

### Using the Scenario Builder

```rust
use elara_loadtest::scenarios::ScenarioBuilder;
use std::time::Duration;

let config = ScenarioBuilder::new()
    .with_nodes(50)
    .with_connections_per_node(8)
    .with_message_rate(500)
    .with_duration(Duration::from_secs(120))
    .with_ramp_up(Duration::from_secs(20))
    .build();
```

## Predefined Scenarios

### Small Deployment
- **Nodes**: 10
- **Connections per node**: 5
- **Message rate**: 100 msg/sec
- **Duration**: 60 seconds
- **Use case**: Development, testing, initial validation

### Medium Deployment
- **Nodes**: 100
- **Connections per node**: 10
- **Message rate**: 1000 msg/sec
- **Duration**: 300 seconds (5 minutes)
- **Use case**: Production baseline, multi-team collaboration

### Large Deployment
- **Nodes**: 1000
- **Connections per node**: 20
- **Message rate**: 10000 msg/sec
- **Duration**: 600 seconds (10 minutes)
- **Use case**: Stress testing, capacity planning, performance limits

## Load Test Workflow

1. **Configuration Validation**: Ensures all parameters are valid
2. **Node Spawning**: Creates the specified number of test nodes
3. **Session Setup**: All nodes join a common session for message exchange
4. **Ramp-Up Phase**: Gradually establishes connections between nodes
5. **Load Generation**: Generates sustained message load at the target rate
6. **Metrics Collection**: Tracks latency, throughput, and errors in real-time
7. **Cleanup**: Gracefully shuts down all nodes and releases resources
8. **Reporting**: Generates comprehensive results with statistics

## Metrics Collected

### Throughput
- **Messages per second**: Actual achieved throughput
- **Success rate**: Percentage of successfully sent messages

### Latency
- **Average**: Mean latency across all messages
- **P50 (Median)**: 50th percentile latency
- **P95**: 95th percentile latency
- **P99**: 99th percentile latency
- **Max**: Maximum observed latency

### Reliability
- **Total messages**: Total number of messages attempted
- **Successful messages**: Number of successfully sent messages
- **Failed messages**: Number of failed messages
- **Errors**: Detailed error log with timestamps

## CI Integration

Load tests run automatically every night at 2 AM UTC via GitHub Actions. Results are stored as artifacts for 90 days.

### Manual Trigger

You can manually trigger load tests from the GitHub Actions UI:

1. Go to Actions → Load Tests
2. Click "Run workflow"
3. Select the scenario (all, small, medium, or large)
4. Click "Run workflow"

### Viewing Results

Results are uploaded as artifacts and can be downloaded from the workflow run page. Each scenario produces a detailed report with all metrics.

### Failure Notifications

If any load test fails, an automated issue is created with:
- Link to the failed workflow run
- Date of failure
- Labels for easy filtering

## Performance Expectations

### Small Deployment (10 nodes)
- **Expected throughput**: ~100 msg/sec
- **Expected P95 latency**: < 50ms
- **Expected success rate**: > 99%

### Medium Deployment (100 nodes)
- **Expected throughput**: ~1000 msg/sec
- **Expected P95 latency**: < 100ms
- **Expected success rate**: > 98%

### Large Deployment (1000 nodes)
- **Expected throughput**: ~10000 msg/sec
- **Expected P95 latency**: < 200ms
- **Expected success rate**: > 95%

*Note: Actual performance depends on hardware and system resources.*

## Troubleshooting

### High Failure Rate

If you see a failure rate > 5%:
- Check system resources (CPU, memory)
- Reduce node count or message rate
- Increase test duration for more stable results
- Check for network issues

### High Latency

If latency is higher than expected:
- Verify no other processes are consuming resources
- Check if running in release mode (`--release`)
- Consider reducing message rate
- Monitor system load during test

### Out of Memory

For large deployments:
- Ensure sufficient RAM (recommend 8GB+ for 1000 nodes)
- Use a machine with more cores
- Reduce node count or connection count

## Development

### Running Tests

```bash
# Run unit tests
cargo test -p elara-loadtest

# Run with output
cargo test -p elara-loadtest -- --nocapture
```

### Building

```bash
# Debug build
cargo build -p elara-loadtest

# Release build (recommended for load tests)
cargo build --release -p elara-loadtest
```

## Architecture

The load testing framework consists of:

- **TestNode**: Simulates an ELARA Protocol node with message generation
- **LoadTestScenario**: Orchestrates test execution and node coordination
- **LoadTestMetrics**: Collects and analyzes performance metrics
- **Scenarios**: Predefined configurations for common deployment sizes

## License

Same as the ELARA Protocol project.
