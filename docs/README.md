# ELARA Protocol Documentation

**ELARA** = **E**mylton **L**eunufna **A**daptive **R**eality **A**rchitecture

A universal real-time communication substrate for cryptographic reality synchronization.

## What ELARA Is NOT

- ❌ A chat protocol
- ❌ A voice/video protocol  
- ❌ A streaming protocol
- ❌ Another WebRTC wrapper

## What ELARA IS

✅ A **foundational communication substrate** where all modalities (chat, voice, video, presence, AI agents) are configurations on top of unified protocol laws.

## Core Philosophy

> "Communication is not message passing. Communication is reality synchronization."

ELARA treats all communication as **state synchronization** across distributed nodes, where:
- **State (ω)** is living reality
- **Events (ε)** are lawful mutations
- **Time (τ)** is a protocol object, not just a timestamp
- **Authority** determines who can change what
- **Interest** determines who needs to see what

## Documentation Index

### Architecture
- [Core Concepts](./architecture/core-concepts.md) - Fundamental primitives
- [Four Pillars](./architecture/four-pillars.md) - The unified engine
- [Representation Profiles](./architecture/representation-profiles.md) - Modality configurations

### Protocol Specifications
- [Wire Protocol](./specs/wire-protocol.md) - Binary frame format
- [Cryptographic Binding](./specs/crypto-binding.md) - Multi-ratchet AEAD
- [Time Engine](./specs/time-engine.md) - Dual clock system
- [State Reconciliation](./specs/state-reconciliation.md) - Convergence under chaos

### Implementation
- [Crate Structure](./implementation/crate-structure.md) - Rust workspace layout
- [API Reference](./implementation/api-reference.md) - Public interfaces
- [Testing Strategy](./implementation/testing-strategy.md) - Verification approach

### MSP (Minimum Survivable Product)
- [MSP v0 Specification](./msp/msp-v0.md) - First organism

## Quick Start

```rust
use elara_runtime::Node;
use elara_core::{SessionId, NodeId};

// Create a node
let node = Node::new(NodeId::generate(), config);

// Join a session (reality space)
node.join_session(session_id).await?;

// Emit state changes
node.emit_event(event).await?;
```

## Core Invariants

1. **Cryptographic Continuity** - Identity is mathematically bound
2. **Temporal Coherence** - Time flows, never jumps
3. **Eventual Convergence** - All nodes reach equivalent reality
4. **Resource-Bounded Survival** - Graceful degradation, never crash
5. **Representation Independence** - Same laws, different views
6. **Server Blindness** - Infrastructure cannot read content

## License

[To be determined]

## Contributing

[To be determined]
