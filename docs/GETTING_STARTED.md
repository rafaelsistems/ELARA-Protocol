# Getting Started with ELARA

This guide will help you get started with ELARA Protocol for research, proof of concept, and development.

## Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Git** - For version control

## Installation

### Clone the Repository

```bash
git clone https://github.com/rafaelsistems/ELARA-Protocol.git
cd ELARA-Protocol
```

### Build

```bash
# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release
```

### Verify Installation

```bash
# Run all tests
cargo test --workspace

# Expected output: 86 tests passing
```

## Quick Tour

### 1. Run Examples

```bash
# Basic node example
cargo run --example basic_node

# Time engine simulation
cargo run --example time_simulation

# State reconciliation concepts
cargo run --example state_reconciliation
```

### 2. Explore the Crates

| Crate | Purpose | Start Here |
|-------|---------|------------|
| `elara-core` | Core types | `src/lib.rs` |
| `elara-wire` | Wire protocol | `src/header.rs` |
| `elara-crypto` | Cryptography | `src/secure_frame.rs` |
| `elara-time` | Time engine | `src/engine.rs` |
| `elara-state` | Reconciliation | `src/reconcile.rs` |

### 3. Generate Documentation

```bash
cargo doc --workspace --no-deps --open
```

## Use Case Guides

### Protocol Research

Study the architecture and specifications:

1. Read [Core Concepts](architecture/core-concepts.md)
2. Read [Four Pillars](architecture/four-pillars.md)
3. Explore the implementation in `crates/`

### Proof of Concept

Validate ELARA concepts in your domain:

```rust
use elara_core::{NodeId, SessionId, PacketClass};
use elara_crypto::SecureFrameProcessor;

// Create a secure frame processor
let session_key = [0u8; 32]; // Derive from key exchange
let mut processor = SecureFrameProcessor::new(
    SessionId::new(1),
    NodeId::generate(),
    session_key,
);

// Encrypt a frame
let encrypted = processor.encrypt_frame(
    PacketClass::Core,
    RepresentationProfile::Textual,
    0,
    Extensions::new(),
    b"Hello, ELARA!",
)?;
```

### Internal Testing

Use the test harness for experiments:

```rust
use elara_test::{TimeSimulator, ClockDriftModel, StateFuzzer, FuzzerConfig};

// Time simulation
let mut sim = TimeSimulator::new();
let node_a = sim.add_node(ClockDriftModel::fast(100.0));
let node_b = sim.add_node(ClockDriftModel::slow(50.0));
sim.add_link(node_a, node_b);
let result = sim.run(Duration::from_secs(60), Duration::from_millis(100));

// State fuzzing
let config = FuzzerConfig::default();
let mut fuzzer = StateFuzzer::new(config);
let result = fuzzer.run();
assert!(result.converged);
```

### Development Contribution

Set up for development:

```bash
# Install development tools
cargo install cargo-watch cargo-nextest

# Watch mode
cargo watch -x "test --workspace"

# Run specific tests
cargo test -p elara-crypto -- --nocapture
```

## Key Concepts

### Five Primitives

| Primitive | Symbol | Description |
|-----------|--------|-------------|
| State | ω | Living reality |
| Event | ε | Valid mutation |
| Time | τ | Protocol object |
| Authority | - | Who can change what |
| Interest | - | Who needs to see what |

### Dual Clock System

```
τp (Perceptual) - Smooth, monotonic, local
τs (State)      - Elastic, correctable, convergent
```

### Packet Classes

```
Core        - Essential, never drop
Perceptual  - Real-time, drop old
Enhancement - Quality, drop under pressure
Cosmetic    - Non-essential, free drop
Repair      - Gap fill
```

## Common Tasks

### Create a Node Identity

```rust
use elara_crypto::Identity;

let identity = Identity::generate();
let node_id = identity.node_id();
let public_identity = identity.public();
```

### Build a Frame

```rust
use elara_wire::{FrameBuilder, Extensions};
use elara_core::{SessionId, NodeId, PacketClass};

let frame = FrameBuilder::new(session_id, node_id, PacketClass::Core)
    .with_profile(RepresentationProfile::Textual)
    .with_time_hint(1000)
    .with_payload(payload.to_vec())
    .build();
```

### Process Events

```rust
use elara_state::ReconciliationEngine;
use elara_time::TimeEngine;

let mut engine = ReconciliationEngine::new(local_node_id);
let time_engine = TimeEngine::new(TimeEngineConfig::default());

let result = engine.process_event(event, &time_engine);
match result {
    EventResult::Applied => println!("Event applied"),
    EventResult::Quarantined(reason) => println!("Quarantined: {:?}", reason),
    EventResult::Rejected(reason) => println!("Rejected: {:?}", reason),
}
```

## Next Steps

1. **Read the specs** - `docs/specs/` for detailed specifications
2. **Run the tests** - Understand behavior through tests
3. **Modify examples** - Experiment with the code
4. **Join discussions** - [GitHub Discussions](https://github.com/rafaelsistems/ELARA-Protocol/discussions)

## Troubleshooting

### Build Errors

```bash
# Clean and rebuild
cargo clean
cargo build --workspace
```

### Test Failures on Windows

```bash
# Run tests single-threaded to avoid file locking
cargo test --workspace -- --test-threads=1
```

### Missing Dependencies

```bash
# Update dependencies
cargo update
```

## Resources

- [README](../README.md) - Project overview
- [Architecture](architecture/) - Design documents
- [Specifications](specs/) - Protocol specs
- [API Reference](implementation/api-reference.md) - Public APIs
- [Contributing](../CONTRIBUTING.md) - How to contribute
