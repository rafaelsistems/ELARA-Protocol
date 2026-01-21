# ELARA Crate Structure

The ELARA implementation is organized as a Rust workspace with modular crates.

## Workspace Layout

```
r:/ELARA/
├── Cargo.toml              # Workspace root
├── docs/                   # Documentation
│   ├── README.md
│   ├── architecture/
│   ├── specs/
│   ├── implementation/
│   └── msp/
└── crates/
    ├── elara-core/         # Core types and primitives
    ├── elara-wire/         # Wire protocol
    ├── elara-crypto/       # Cryptographic binding
    ├── elara-time/         # Time engine
    ├── elara-state/        # State reconciliation
    ├── elara-transport/    # Network transport
    ├── elara-runtime/      # Node runtime
    ├── elara-msp/          # MSP profiles
    └── elara-test/         # Test harness
```

## Crate Dependency Graph

```
                    ┌─────────────┐
                    │ elara-core  │
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
           ▼               ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │ elara-wire  │ │ elara-time  │ │elara-crypto │
    └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
           │               │               │
           └───────────────┼───────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ elara-state │
                    └──────┬──────┘
                           │
                           ▼
                   ┌──────────────┐
                   │elara-transport│
                   └──────┬───────┘
                          │
                          ▼
                   ┌─────────────┐
                   │elara-runtime│
                   └──────┬──────┘
                          │
            ┌─────────────┴─────────────┐
            ▼                           ▼
     ┌─────────────┐             ┌─────────────┐
     │  elara-msp  │             │ elara-test  │
     └─────────────┘             └─────────────┘
```

## Crate Details

### elara-core

**Purpose**: Core types, primitives, and error definitions.

**Key Types**:
- `NodeId`, `SessionId`, `StateId`, `EventId` - Identifiers
- `PacketClass`, `RepresentationProfile` - Classifications
- `StateAtom`, `Event`, `MutationOp` - State primitives
- `VersionVector`, `TimeIntent` - Versioning
- `PerceptualTime`, `StateTime`, `RealityWindow` - Time types
- `ElaraError`, `ElaraResult` - Error handling

**Dependencies**: Minimal (thiserror, proptest for testing)

```toml
[dependencies]
thiserror = "1.0"

[dev-dependencies]
proptest = "1.0"
```

### elara-wire

**Purpose**: Wire protocol frame encoding/decoding.

**Key Types**:
- `FixedHeader` - 28-byte frame header
- `Extensions` - TLV header extensions
- `Frame` - Complete frame structure
- `FrameBuilder` - Frame construction
- `FrameFlags` - Flag bit manipulation

**Features**:
- Zero-copy parsing where possible
- Little-endian encoding
- Fragment support
- Extension mechanism

```toml
[dependencies]
elara-core = { path = "../elara-core" }
```

### elara-crypto

**Purpose**: Cryptographic operations and key management.

**Key Types**:
- `Identity`, `PublicIdentity` - Node identity
- `AeadCipher` - ChaCha20-Poly1305 encryption
- `ClassRatchet`, `MultiRatchet` - Key ratcheting
- `ReplayWindow`, `ReplayManager` - Replay protection
- `SecureFrameProcessor` - Integrated frame encryption

**Dependencies**:
```toml
[dependencies]
elara-core = { path = "../elara-core" }
elara-wire = { path = "../elara-wire" }
chacha20poly1305 = "0.10"
ed25519-dalek = { version = "2.0", features = ["rand_core"] }
x25519-dalek = "2.0"
sha2 = "0.10"
hkdf = "0.12"
rand = "0.8"
```

### elara-time

**Purpose**: Time engine implementation.

**Key Types**:
- `PerceptualClock`, `StateClock` - Dual clocks
- `TimeEngine` - Main engine
- `TimeEngineConfig` - Configuration
- `NetworkModel`, `PeerNetworkModel` - Network estimation

**Features**:
- Horizon adaptation
- Drift estimation
- Correction blending
- Compression control

```toml
[dependencies]
elara-core = { path = "../elara-core" }
```

### elara-state

**Purpose**: State field and reconciliation.

**Key Types**:
- `StateField` - State atom container
- `ReconciliationEngine` - Event processing pipeline
- `QuarantineBuffer` - Pending events

**Features**:
- Six-stage reconciliation pipeline
- Version vector management
- Authority checking
- Divergence control

```toml
[dependencies]
elara-core = { path = "../elara-core" }
elara-time = { path = "../elara-time" }
```

### elara-transport

**Purpose**: Network transport abstraction.

**Key Types**:
- `Transport` trait - Transport abstraction
- `UdpTransport` - UDP implementation
- `Connection` - Peer connection
- `ConnectionPool` - Connection management

**Features**:
- UDP transport
- Connection pooling
- NAT traversal helpers
- Relay support (planned)

```toml
[dependencies]
elara-core = { path = "../elara-core" }
elara-wire = { path = "../elara-wire" }
tokio = { version = "1.0", features = ["net", "rt"] }
```

### elara-runtime

**Purpose**: Node runtime and session management.

**Key Types**:
- `Node` - Main node structure
- `NodeConfig` - Node configuration
- `Session` - Session state
- `EventLoop` - Main processing loop

**Features**:
- Node lifecycle management
- Session join/leave
- Event emission
- Tick scheduling

```toml
[dependencies]
elara-core = { path = "../elara-core" }
elara-wire = { path = "../elara-wire" }
elara-crypto = { path = "../elara-crypto" }
elara-time = { path = "../elara-time" }
elara-state = { path = "../elara-state" }
elara-transport = { path = "../elara-transport" }
tokio = { version = "1.0", features = ["full"] }
```

### elara-msp

**Purpose**: MSP v0 profile implementations.

**Key Types**:
- `TextProfile` - Textual profile
- `VoiceProfile` - Voice minimal profile
- `TextMessage`, `TextDelta` - Text types
- `VoiceFrame` - Voice frame

**Features**:
- Profile-specific state atoms
- Delta encoding/decoding
- Degradation paths

```toml
[dependencies]
elara-core = { path = "../elara-core" }
elara-state = { path = "../elara-state" }
```

### elara-test

**Purpose**: Testing utilities and simulation.

**Key Types**:
- `NetworkSimulator` - Network simulation
- `ChaosNetwork` - Chaos injection
- `TimeSimulator` - Time simulation
- `StateFuzzer` - Property-based testing

**Features**:
- Clock drift simulation
- Network chaos (jitter, loss, reorder)
- Property-based fuzzing
- Convergence testing

```toml
[dependencies]
elara-core = { path = "../elara-core" }
elara-time = { path = "../elara-time" }
elara-state = { path = "../elara-state" }
rand = "0.8"
proptest = "1.0"
```

## Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/elara-core",
    "crates/elara-wire",
    "crates/elara-crypto",
    "crates/elara-time",
    "crates/elara-state",
    "crates/elara-transport",
    "crates/elara-runtime",
    "crates/elara-msp",
    "crates/elara-test",
]

[workspace.dependencies]
thiserror = "1.0"
tokio = { version = "1.0", features = ["full"] }
rand = "0.8"
proptest = "1.0"
chacha20poly1305 = "0.10"
ed25519-dalek = { version = "2.0", features = ["rand_core"] }
x25519-dalek = "2.0"
sha2 = "0.10"
hkdf = "0.12"
```

## Build Commands

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p elara-core
cargo test -p elara-crypto

# Check without building
cargo check --workspace

# Build release
cargo build --workspace --release

# Generate docs
cargo doc --workspace --no-deps
```

## Feature Flags

Currently minimal feature flags. Future considerations:

```toml
[features]
default = []
std = []           # Standard library (default on)
no_std = []        # No standard library support
simd = []          # SIMD optimizations
async = ["tokio"]  # Async runtime
```

## Testing Strategy

Each crate has:
1. **Unit tests**: In `src/*.rs` files
2. **Integration tests**: In `tests/` directory (if needed)
3. **Property tests**: Using proptest for invariant checking
4. **Benchmarks**: In `benches/` (planned)

Run all tests:
```bash
cargo test --workspace -- --test-threads=1
```

## Code Organization Principles

1. **Minimal dependencies**: Each crate only depends on what it needs
2. **Clear boundaries**: Public API is explicit and documented
3. **Testability**: All components can be tested in isolation
4. **No circular deps**: Dependency graph is acyclic
5. **Feature isolation**: Optional features don't affect core
