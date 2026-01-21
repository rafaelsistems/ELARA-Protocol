# ELARA Testing Strategy

Comprehensive testing approach for validating ELARA protocol implementation.

## Testing Pyramid

```
                    ┌─────────────────┐
                    │   End-to-End    │  ← Full system tests
                    │     Tests       │
                    └────────┬────────┘
                             │
                    ┌────────┴────────┐
                    │  Integration    │  ← Cross-crate tests
                    │     Tests       │
                    └────────┬────────┘
                             │
           ┌─────────────────┴─────────────────┐
           │           Unit Tests              │  ← Per-module tests
           └─────────────────┬─────────────────┘
                             │
    ┌────────────────────────┴────────────────────────┐
    │              Property-Based Tests               │  ← Invariant verification
    └─────────────────────────────────────────────────┘
```

## Test Categories

### 1. Unit Tests

Located in each module's source file.

```rust
// Example: elara-core/src/time.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_time_wire_roundtrip() {
        let base = StateTime::from_millis(1000);
        let time = StateTime::from_millis(1500);
        let wire = time.to_wire(base);
        let recovered = StateTime::from_wire(base, wire);
        assert_eq!(time, recovered);
    }
}
```

**Coverage targets:**
- All public functions
- Edge cases (zero, max, overflow)
- Error conditions

### 2. Property-Based Tests

Using `proptest` for invariant verification.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn version_vector_merge_commutative(
        entries1 in prop::collection::hash_map(any::<u64>(), any::<u64>(), 0..10),
        entries2 in prop::collection::hash_map(any::<u64>(), any::<u64>(), 0..10)
    ) {
        let v1 = VersionVector::from_entries(entries1);
        let v2 = VersionVector::from_entries(entries2);
        
        let merge1 = v1.merge(&v2);
        let merge2 = v2.merge(&v1);
        
        assert_eq!(merge1, merge2);  // Commutative
    }
    
    #[test]
    fn version_vector_merge_associative(
        e1 in version_vector_strategy(),
        e2 in version_vector_strategy(),
        e3 in version_vector_strategy()
    ) {
        let v1 = VersionVector::from_entries(e1);
        let v2 = VersionVector::from_entries(e2);
        let v3 = VersionVector::from_entries(e3);
        
        let left = v1.merge(&v2).merge(&v3);
        let right = v1.merge(&v2.merge(&v3));
        
        assert_eq!(left, right);  // Associative
    }
}
```

**Key properties to verify:**
- Version vector CRDT properties (commutative, associative, idempotent)
- Replay window correctness (no false positives/negatives)
- Frame serialization roundtrip
- Encryption/decryption roundtrip
- Time monotonicity

### 3. Integration Tests

Cross-crate functionality tests.

```rust
// tests/integration/secure_frame_test.rs
#[test]
fn test_secure_frame_full_pipeline() {
    // Setup
    let session_key = [0x42u8; 32];
    let mut sender = SecureFrameProcessor::new(
        SessionId::new(1),
        NodeId::new(1),
        session_key
    );
    let mut receiver = SecureFrameProcessor::new(
        SessionId::new(1),
        NodeId::new(2),
        session_key
    );
    
    // Encrypt
    let payload = b"Hello, ELARA!";
    let encrypted = sender.encrypt_frame(
        PacketClass::Core,
        RepresentationProfile::Textual,
        1000,
        Extensions::new(),
        payload
    ).unwrap();
    
    // Decrypt
    let decrypted = receiver.decrypt_frame(&encrypted).unwrap();
    
    // Verify
    assert_eq!(decrypted.payload, payload);
    assert_eq!(decrypted.class(), PacketClass::Core);
}
```

### 4. Simulation Tests

Using `elara-test` harness.

```rust
#[test]
fn test_convergence_under_partition() {
    let config = FuzzerConfig {
        node_count: 4,
        event_count: 100,
        partition_probability: 0.3,
        ..FuzzerConfig::default()
    };
    
    let mut fuzzer = StateFuzzer::new(config);
    let result = fuzzer.run();
    
    assert!(result.converged, "Nodes should converge after partition heals");
    assert_eq!(result.authority_violations, 0);
    assert_eq!(result.causality_violations, 0);
}

#[test]
fn test_time_drift_compensation() {
    let mut sim = TimeSimulator::new();
    
    // Add nodes with different drift characteristics
    let node_a = sim.add_node(ClockDriftModel::fast(100.0));  // +100 ppm
    let node_b = sim.add_node(ClockDriftModel::slow(50.0));   // -50 ppm
    sim.add_link(node_a, node_b);
    
    // Run simulation
    let result = sim.run(Duration::from_secs(60), Duration::from_millis(100));
    
    // Check drift was compensated
    assert!(result.max_divergence < Duration::from_millis(50));
}
```

### 5. Chaos Tests

Network chaos injection.

```rust
#[test]
fn test_voice_under_jitter_chaos() {
    let chaos = ChaosConfig {
        latency_base: Duration::from_millis(50),
        jitter_range: Duration::ZERO..Duration::from_millis(200),
        loss_rate: 0.05,
        reorder_rate: 0.1,
        duplicate_rate: 0.01,
    };
    
    let mut network = ChaosNetwork::new(chaos);
    let mut voice_session = VoiceSession::new();
    
    // Send 1000 voice frames through chaos
    for i in 0..1000 {
        let frame = VoiceFrame::generate(i);
        
        if !network.should_drop() {
            let delay = network.delay();
            voice_session.receive_frame(frame, delay);
        }
    }
    
    // Check quality metrics
    let metrics = voice_session.quality_metrics();
    assert!(metrics.intelligibility > 0.8, "Voice should remain intelligible");
    assert!(metrics.continuity > 0.9, "Voice should be continuous");
}
```

## Test Scenarios

### Scenario 1: Jitter Chaos

```rust
fn jitter_chaos_scenario() -> ChaosConfig {
    ChaosConfig {
        latency_base: Duration::from_millis(20),
        jitter_range: Duration::ZERO..Duration::from_millis(500),
        loss_rate: 0.0,
        reorder_rate: 0.2,
        duplicate_rate: 0.0,
    }
}
```

**Validates:**
- Time engine horizon adaptation
- Jitter buffer behavior
- Out-of-order handling

### Scenario 2: Packet Loss Torture

```rust
fn packet_loss_scenario() -> ChaosConfig {
    ChaosConfig {
        latency_base: Duration::from_millis(50),
        jitter_range: Duration::ZERO..Duration::from_millis(20),
        loss_rate: 0.15,  // 15% loss
        reorder_rate: 0.0,
        duplicate_rate: 0.0,
    }
}
```

**Validates:**
- Graceful degradation
- State recovery
- Gap filling

### Scenario 3: Network Partition

```rust
fn partition_scenario() -> PartitionConfig {
    PartitionConfig {
        nodes: 8,
        partition_groups: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
        partition_duration: Duration::from_secs(30),
        concurrent_events_per_group: 50,
    }
}
```

**Validates:**
- Independent operation during partition
- Merge protocol
- Convergence after heal

### Scenario 4: Byzantine Node

```rust
fn byzantine_scenario() -> ByzantineConfig {
    ByzantineConfig {
        total_nodes: 8,
        byzantine_nodes: 2,
        behaviors: vec![
            ByzantineBehavior::RapidFire,      // Flood events
            ByzantineBehavior::VersionLie,     // Fake version vectors
            ByzantineBehavior::ReplayOld,      // Replay old events
        ],
    }
}
```

**Validates:**
- Authority containment
- Rate limiting
- Anomaly detection

## Running Tests

### All Tests

```bash
cargo test --workspace
```

### Specific Crate

```bash
cargo test -p elara-core
cargo test -p elara-crypto
cargo test -p elara-test
```

### With Output

```bash
cargo test --workspace -- --nocapture
```

### Single-Threaded (for determinism)

```bash
cargo test --workspace -- --test-threads=1
```

### Property Tests with More Cases

```bash
PROPTEST_CASES=10000 cargo test --workspace
```

## Test Coverage

### Current Status

| Crate | Tests | Status |
|-------|-------|--------|
| elara-core | 16 | ✅ |
| elara-wire | 13 | ✅ |
| elara-crypto | 23 | ✅ |
| elara-time | 10 | ✅ |
| elara-state | 3 | ✅ |
| elara-transport | 2 | ✅ |
| elara-runtime | 2 | ✅ |
| elara-msp | 2 | ✅ |
| elara-test | 15 | ✅ |
| **Total** | **86** | ✅ |

### Coverage Goals

- Line coverage: >80%
- Branch coverage: >70%
- Critical paths: 100%

## Continuous Integration

### CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build
        run: cargo build --workspace
      - name: Test
        run: cargo test --workspace -- --test-threads=1
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
      - name: Format
        run: cargo fmt --all -- --check
```

### Pre-commit Checks

```bash
#!/bin/bash
# .git/hooks/pre-commit

cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace -- --test-threads=1
```

## Benchmarks (Planned)

```rust
// benches/crypto_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_aead_encrypt(c: &mut Criterion) {
    let cipher = AeadCipher::new(&[0u8; 32]);
    let nonce = [0u8; 12];
    let payload = vec![0u8; 1024];
    
    c.bench_function("aead_encrypt_1kb", |b| {
        b.iter(|| cipher.encrypt(&nonce, &[], &payload))
    });
}

fn bench_frame_parse(c: &mut Criterion) {
    let frame = create_test_frame();
    
    c.bench_function("frame_parse", |b| {
        b.iter(|| Frame::parse(&frame))
    });
}

criterion_group!(benches, bench_aead_encrypt, bench_frame_parse);
criterion_main!(benches);
```

## Test Data Generators

```rust
// Test data generation utilities
pub mod generators {
    use super::*;
    
    pub fn random_node_id() -> NodeId {
        NodeId::new(rand::random())
    }
    
    pub fn random_event(source: NodeId, target: StateId) -> Event {
        Event {
            id: EventId { source, seq: rand::random() },
            event_type: EventType::StateUpdate,
            source,
            target_state: target,
            version_ref: VersionVector::new(),
            mutation: MutationOp::Set(vec![rand::random(); 32]),
            time_intent: TimeIntent::now(),
            authority_proof: AuthorityProof::new([0u8; 64]),
            entropy_hint: EntropyHint::default(),
        }
    }
    
    pub fn random_voice_frame(seq: u16) -> VoiceFrame {
        VoiceFrame {
            frame_seq: seq,
            timestamp_offset: 0,
            voiced: rand::random(),
            pitch: rand::random(),
            energy: rand::random(),
            spectral_env: rand::random(),
            residual_seed: rand::random(),
        }
    }
}
```

## Debugging Failed Tests

### Enable Logging

```rust
#[test]
fn test_with_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
    
    // Test code...
}
```

### Deterministic Seeds

```rust
#[test]
fn test_with_seed() {
    let seed = 12345u64;  // Fixed seed for reproducibility
    let mut rng = StdRng::seed_from_u64(seed);
    
    // Test with deterministic randomness...
}
```

### Snapshot Testing

```rust
#[test]
fn test_frame_serialization_snapshot() {
    let frame = create_canonical_frame();
    let bytes = frame.serialize();
    
    // Compare against known-good snapshot
    assert_eq!(bytes, include_bytes!("snapshots/canonical_frame.bin"));
}
```
