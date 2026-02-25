# elara-test

Testing harness and chaos testing framework for the ELARA Protocol - providing comprehensive testing tools, benchmarks, and protocol validation utilities.

## Features

- **Chaos Testing**: Network failure simulation and recovery testing
- **Protocol Validation**: Hard invariant verification
- **Performance Benchmarks**: Comprehensive benchmarking suite
- **Property-Based Testing**: Automated test case generation
- **Integration Testing**: End-to-end protocol testing
- **Stress Testing**: Load and resilience testing

## Quick Start

```rust
use elara_test::{TestHarness, ChaosConfig, ProtocolValidator};

// Create test harness
let mut harness = TestHarness::new();

// Configure chaos testing
let chaos_config = ChaosConfig {
    network_failure_rate: 0.1,
    packet_loss_rate: 0.05,
    latency_variance: Duration::from_millis(100),
};

// Run chaos test
let result = harness.run_chaos_test(chaos_config)?;

// Validate protocol invariants
let validator = ProtocolValidator::new();
validator.validate_hard_invariants()?;
```

## Chaos Testing

### Network Simulation
```rust
// Simulate network failures
let network_chaos = NetworkChaos {
    partition_duration: Duration::from_secs(30),
    recovery_time: Duration::from_secs(5),
    max_simultaneous_failures: 3,
};

// Inject chaos
harness.inject_network_chaos(network_chaos)?;
```

### Protocol Disruption
```rust
// Test protocol resilience
let protocol_chaos = ProtocolChaos {
    message_corruption_rate: 0.01,
    out_of_order_delivery: true,
    duplicate_messages: true,
};

harness.inject_protocol_chaos(protocol_chaos)?;
```

## Benchmarking

### Wire Protocol Benchmarks
```bash
cargo bench --bench wire_bench
```

### Cryptographic Operations
```bash
cargo bench --bench crypto_bench
```

### Time Engine Performance
```bash
cargo bench --bench time_bench
```

### State Reconciliation
```bash
cargo bench --bench state_bench
```

## Property-Based Testing

### Hard Invariants
```rust
// Test the five hard invariants
proptest! {
    #[test]
    fn test_reality_never_waits(events in event_sequence()) {
        let result = harness.test_reality_never_waits(events);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn test_presence_over_packets(scenario in network_scenario()) {
        let result = harness.test_presence_over_packets(scenario);
        prop_assert!(result.is_ok());
    }
}
```

### Temporal Properties
```rust
// Test time convergence
proptest! {
    #[test]
    fn test_eventual_convergence(clocks in clock_drift()) {
        let result = harness.test_eventual_convergence(clocks);
        prop_assert!(result.is_ok());
    }
}
```

## Integration Testing

### End-to-End Scenarios
```rust
// Test complete protocol stack
let scenario = IntegrationScenario {
    node_count: 10,
    network_conditions: NetworkConditions::Adversarial,
    test_duration: Duration::from_secs(300),
    success_criteria: SuccessCriteria::AllNodesConverge,
};

let result = harness.run_integration_test(scenario)?;
```

### Multi-Modal Testing
```rust
// Test text, voice, and visual together
let multi_modal = MultiModalTest {
    text_stream_rate: 10.0,  // messages per second
    voice_stream_rate: 1.0,   // streams per second
    visual_stream_rate: 0.5,  // streams per second
    degradation_test: true,
};

harness.run_multi_modal_test(multi_modal)?;
```

## Stress Testing

### Load Generation
```rust
// Generate high load
let load_test = LoadTest {
    target_rps: 1000.0,       // requests per second
    concurrent_streams: 100,
    duration: Duration::from_secs(600),
    ramp_up_time: Duration::from_secs(60),
};

harness.run_load_test(load_test)?;
```

### Resource Constraints
```rust
// Test under resource pressure
let resource_test = ResourceConstraintTest {
    memory_limit: Some(1024 * 1024 * 100),  // 100MB
    cpu_limit: Some(0.5),                    // 50% CPU
    network_bandwidth: Some(1024 * 1024),  // 1MB/s
};

harness.run_resource_constraint_test(resource_test)?;
```

## Validation Tools

### Protocol Compliance
```rust
// Validate protocol compliance
let validator = ProtocolValidator::new();
validator.validate_wire_format()?;
validator.validate_crypto_primitives()?;
validator.validate_time_semantics()?;
validator.validate_state_convergence()?;
```

### Performance Validation
```rust
// Check performance requirements
let perf_validator = PerformanceValidator::new();
perf_validator.validate_latency_requirements()?;
perf_validator.validate_throughput_requirements()?;
perf_validator.validate_resource_usage()?;
```

## Test Categories

### Unit Tests
- Individual component testing
- Function-level validation
- Edge case coverage

### Integration Tests
- Multi-component interaction
- Protocol sequence validation
- Error handling verification

### System Tests
- End-to-end scenarios
- Real-world conditions
- Performance benchmarking

### Chaos Tests
- Network failures
- Node crashes
- Adversarial conditions

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.