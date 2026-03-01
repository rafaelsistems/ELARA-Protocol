# ELARA Protocol

[![Build Status](https://github.com/rafaelsistems/ELARA-Protocol/workflows/CI/badge.svg)](https://github.com/rafaelsistems/ELARA-Protocol/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

**ELARA** = **E**mylton **L**eunufna **A**daptive **R**eality **A**rchitecture

A universal real-time communication substrate for cryptographic reality synchronization.

## What is ELARA?

ELARA is **NOT** another chat/voice/video protocol. It's a **foundational communication substrate** where all communication modalities (text, voice, video, presence, AI agents) are configurations on top of unified protocol laws.

> "Communication is not message delivery. Communication is reality synchronization."

### Core Philosophy

Traditional protocols treat network problems as errors to handle. ELARA treats time as a first-class protocol object that bends under pressure rather than breaking.

```
Traditional: Network bad → Freeze → Reset → Reconnect
ELARA:       Network bad → Bend time → Simplify reality → Keep flowing
```

## Hard Invariants

ELARA is governed by five **hard invariants**. These are not guidelines—they are system laws. If any single invariant falls, the system is not ELARA.

| # | Invariant | Meaning |
|---|-----------|---------|
| 1 | **Reality Never Waits** | System never blocks reality for synchronization |
| 2 | **Presence Over Packets** | Existence matters more than data perfection |
| 3 | **Experience Degrades, Never Collapses** | Quality reduces, never fails |
| 4 | **Event Is Truth, State Is Projection** | Events are authoritative, state is cache |
| 5 | **Identity Survives Transport** | Identity persists beyond connections |

See [HARD_INVARIANTS.md](docs/HARD_INVARIANTS.md) for the complete specification.

## Features

- 🔐 **Cryptographic Reality Physics** - Identity-bound, server-blind encryption
- ⏱️ **Dual Clock System** - Perceptual time (smooth) + State time (convergent)
- 🔄 **Eventual Convergence** - All nodes reach equivalent reality
- 📉 **Graceful Degradation** - Quality reduces, connection persists
- 🌐 **NAT Hostile Ready** - Works behind restrictive firewalls
- 📱 **Resource Bounded** - Runs on 2-core CPU, 2GB RAM, no GPU

## Project Status

| Component | Status | Description |
|-----------|--------|-------------|
| Architecture | ✅ Complete | Full specification |
| Core Implementation | ✅ Complete | 16 crates, comprehensive tests |
| Documentation | ✅ Complete | Comprehensive docs |
| MSP v0 Spec | ✅ Complete | Text + Voice minimal |
| Production Features | ✅ Complete | Observability, security, performance |
| Production Ready | ✅ Yes | v0.2.0 |

**Current Version: v0.2.0 (Production)**

## Quick Start

### Prerequisites

- Rust 1.70+ ([install](https://rustup.rs/))
- Git

### Clone and Build

```bash
git clone https://github.com/rafaelsistems/ELARA-Protocol.git
cd ELARA-Protocol

# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Generate documentation
cargo doc --workspace --no-deps --open
```

### Run Examples

```bash
# Basic node example
cargo run --example basic_node

# Time engine simulation
cargo run --example time_simulation

# State reconciliation demo
cargo run --example state_reconciliation
```

## Architecture Overview

### Five Fundamental Primitives

| Primitive | Symbol | Description |
|-----------|--------|-------------|
| **State** | ω | Living reality |
| **Event** | ε | Valid mutation |
| **Time** | τ | Protocol object |
| **Authority** | - | Who can change what |
| **Interest** | - | Who needs to see what |

### Four Pillars

```
┌─────────────────────────────────────────────────────────────┐
│                    ELARA PROTOCOL                            │
├─────────────────┬─────────────────┬─────────────────────────┤
│  Cryptographic  │      Time       │    State Field &        │
│ Reality Physics │  Convergence    │   Swarm Diffusion       │
│                 │    Engine       │                         │
├─────────────────┴─────────────────┴─────────────────────────┤
│              Packet Ecology & Wire Semantics                 │
└─────────────────────────────────────────────────────────────┘
```

### Crate Structure

```
crates/
├── elara-core      # Core types and primitives
├── elara-wire      # Wire protocol encoding
├── elara-crypto    # Cryptographic binding
├── elara-time      # Time engine
├── elara-state     # State reconciliation
├── elara-transport # Network transport
├── elara-runtime   # Node runtime with observability
├── elara-msp       # MSP profiles
├── elara-test      # Testing harness and security tests
├── elara-ffi       # Foreign function interface
├── elara-visual    # Visual state encoding
├── elara-diffusion # Swarm diffusion
├── elara-voice     # Voice encoding
├── elara-fuzz      # Fuzzing infrastructure
├── elara-bench     # Performance benchmarks
└── elara-loadtest  # Load testing framework
```

## Documentation

| Document | Description |
|----------|-------------|
| [Core Concepts](docs/architecture/core-concepts.md) | Fundamental primitives |
| [Four Pillars](docs/architecture/four-pillars.md) | Architecture overview |
| [Wire Protocol](docs/specs/wire-protocol.md) | Binary frame format |
| [Crypto Binding](docs/specs/crypto-binding.md) | Security specification |
| [Time Engine](docs/specs/time-engine.md) | Temporal mechanics |
| [State Reconciliation](docs/specs/state-reconciliation.md) | Convergence spec |
| [MSP v0](docs/msp/msp-v0.md) | Minimum Survivable Product |
| [API Reference](docs/implementation/api-reference.md) | Public APIs |
| [Crate Structure](docs/implementation/crate-structure.md) | Code organization |
| [Testing Strategy](docs/implementation/testing-strategy.md) | Test approach |

## Use Cases

### ✅ Suitable For Now

- **Production Deployment** - Real-world use
- **User-Facing Applications** - Direct end-user delivery
- **Mission-Critical Systems** - High-reliability workloads
- **Internal Platforms** - Team and org-wide infrastructure

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone with submodules
git clone --recursive https://github.com/rafaelsistems/ELARA-Protocol.git

# Install development tools
cargo install cargo-watch cargo-nextest

# Run tests with watch
cargo watch -x "test --workspace"

# Run specific crate tests
cargo test -p elara-core
cargo test -p elara-crypto
```

### Areas for Contribution

- 🧪 **Testing** - More test coverage, chaos testing
- 📚 **Documentation** - Improve clarity, add examples
- 🔧 **Implementation** - Bug fixes, optimizations
- 🌐 **Transport** - NAT traversal hardening
- 📱 **Bindings** - Mobile SDKs (Kotlin, Swift)

## Roadmap

```
v0.2 (Current) - Production Readiness
    ✅ Standardized version management
    ✅ Observability infrastructure
    ✅ Security hardening
    ✅ Performance validation
    ✅ Operational tooling

v0.1 (Completed) - Alpha
    ✅ End-to-end integration tests
    ✅ Real network testing
    ✅ Basic NAT traversal (STUN + hole punching)
    ✅ Basic performance benchmarks

v0.3 (Planned) - Enhanced Features
    ⏳ Advanced NAT traversal (TURN)
    ⏳ Enhanced mobile SDK
    ⏳ Additional MSP profiles

v1.0 (Future) - Production
    ⏳ External security audit
    ⏳ Performance optimization
    ⏳ Formal stability guarantees

v0.0 (Completed) - Research Prototype
    ✅ Core implementation
    ✅ Documentation
    ✅ Unit tests
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Emylton Leunufna - Protocol design and architecture
- All contributors who help make ELARA better

## Contact

- **Repository**: [github.com/rafaelsistems/ELARA-Protocol](https://github.com/rafaelsistems/ELARA-Protocol)
- **Issues**: [GitHub Issues](https://github.com/rafaelsistems/ELARA-Protocol/issues)
- **Discussions**: [GitHub Discussions](https://github.com/rafaelsistems/ELARA-Protocol/discussions)

---

<p align="center">
  <i>"Network affects the SHAPE of reality, not its CONTINUITY."</i>
</p>
