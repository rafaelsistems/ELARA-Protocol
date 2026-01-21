# The Four Pillars of ELARA

ELARA is built on four unified pillars that work as **one engine**, not a stack of layers.

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    ELARA UNIFIED ENGINE                      │
├─────────────────┬─────────────────┬─────────────────┬───────┤
│  Cryptographic  │      Time       │   State Field   │ Packet│
│    Reality      │   Convergence   │    & Swarm      │Ecology│
│    Physics      │     Engine      │   Diffusion     │& Wire │
├─────────────────┼─────────────────┼─────────────────┼───────┤
│   Identity      │   Dual Clocks   │  Reconciliation │ Frame │
│   Session       │   Reality       │  Authority      │ Class │
│   Authority     │   Window        │  Causality      │ Header│
│   Encryption    │   Horizons      │  Convergence    │ AEAD  │
└─────────────────┴─────────────────┴─────────────────┴───────┘
```

## Pillar 1: Cryptographic Reality Physics

Everything in ELARA is cryptographically bound. There is no "trust" - only mathematical proof.

### Identity Binding

```rust
Identity = {
    signing_key: Ed25519SigningKey,
    encryption_key: X25519StaticSecret,
    node_id: NodeId  // Derived from public key
}

// NodeId = truncated hash of public key
NodeId = SHA256(public_key)[0..8]
```

### Session Binding

```rust
Session = {
    session_id: SessionId,
    root_key: [u8; 32],
    participants: Set<NodeId>,
    epoch: u32
}

// All keys derived from session root
K_class = HKDF(root_key, "elara-class-" || class_id)
```

### Multi-Ratchet Key Hierarchy

```
K_session_root
├── K_core (strongest protection, never dropped)
│   └── Ratchet: slow, high security
├── K_perceptual (fast ratchet, loss tolerant)
│   └── Ratchet: fast, forward secrecy per frame
├── K_enhancement (standard protection)
│   └── Ratchet: medium pace
└── K_cosmetic (light protection, free to drop)
    └── Ratchet: minimal overhead
```

### AEAD Encryption

- **Algorithm**: ChaCha20-Poly1305
- **Nonce**: Derived from (NodeId, Sequence, PacketClass)
- **AAD**: Frame header (authenticated but not encrypted)

## Pillar 2: Time Convergence Engine

Time is not a passive timestamp. Time is an active protocol participant.

### Dual Clock System

| Clock | Symbol | Properties | Purpose |
|-------|--------|------------|---------|
| Perceptual | τp | Monotonic, smooth, local | User experience |
| State | τs | Elastic, correctable | Network consensus |

### Reality Window

```
Past ←──────────────────────────────────────────→ Future
      │                    │                    │
      τs - Hc              τs                   τs + Hp
      │                    │                    │
      └── Correction ──────┴──── Prediction ────┘
          Horizon                 Horizon

Events outside RW → Quarantine buffer
Events inside RW → Immediate processing
```

### Horizon Adaptation

```rust
// Bad network → Expand prediction horizon
if network.jitter > threshold {
    Hp = Hp.expand(factor);
}

// Good network → Tighten for sharper reality
if network.stable() {
    Hp = Hp.contract(factor);
}
```

### Non-Destructive Correction

**NEVER:**
- Hard rewind
- Full reset
- Freeze timeline

**ALWAYS:**
- Curve deformation
- Parameter interpolation
- Envelope reshaping
- Predictive path bending

## Pillar 3: State Field & Swarm Diffusion

State exists in a **field** that propagates through the network like a physical phenomenon.

### State Field Structure

```rust
StateField = {
    atoms: Map<StateId, StateAtom>,
    quarantine: Vec<QuarantinedEvent>,
    heat_map: Map<StateId, f64>,  // Activity/relevance
    authority_graph: AuthorityGraph
}
```

### Reconciliation Pipeline

```
Incoming Event
      │
      ▼
┌─────────────────┐
│ Authority Check │ → Reject if unauthorized
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Causality Check │ → Quarantine if deps missing
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Temporal Place  │ → Map to τs via peer model
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Delta Merge    │ → Non-destructive, bounded
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│Divergence Ctrl  │ → Reduce detail if needed
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Swarm Diffusion │ → Emit to interested peers
└─────────────────┘
```

### Convergence Guarantee

All nodes eventually reach **equivalent reality** (not identical bits):

```rust
// Convergence check
fn converged(nodes: &[Node]) -> bool {
    let reference = nodes[0].state_hash();
    nodes.iter().all(|n| n.state_hash() == reference)
}
```

### Partition Handling

```
Normal Operation:
    A ←→ B ←→ C ←→ D

Partition:
    A ←→ B    C ←→ D
    (subgraph) (subgraph)

Merge:
    1. Exchange state summaries
    2. Detect divergence points
    3. Replay missing deltas
    4. Normalize time references
    5. Resume unified operation
```

## Pillar 4: Packet Ecology & Wire Semantics

Packets are not just data carriers. They form an **ecology** with different survival priorities.

### Packet Classes

| Class | Priority | Drop Policy | Use Case |
|-------|----------|-------------|----------|
| Core | Highest | Never drop | Identity, session |
| Perceptual | High | Drop old | Voice, video |
| Enhancement | Medium | Drop under pressure | HD, effects |
| Cosmetic | Low | Free to drop | Reactions, typing |
| Repair | Variable | Context-dependent | Gap fill, sync |

### Frame Structure

```
┌────────────────────────────────────────────┐
│           Fixed Header (24-40 bytes)        │
├────────────────────────────────────────────┤
│     Variable Header Extensions (TLV)        │
├────────────────────────────────────────────┤
│           Encrypted Payload                 │
├────────────────────────────────────────────┤
│           Auth Tag (16 bytes)               │
└────────────────────────────────────────────┘
```

### Graceful Degradation

```
Full Quality
    │
    ▼ (bandwidth pressure)
Voice + Video
    │
    ▼ (more pressure)
Voice Only
    │
    ▼ (severe pressure)
Voice Parameters Only
    │
    ▼ (critical)
Symbolic State
    │
    ▼ (emergency)
Presence Only
    │
    ▼ (survival mode)
Identity Heartbeat

Session NEVER drops. Reality simplifies.
```

## Unified Operation

The four pillars don't operate independently. They form a **single coherent engine**:

1. **Crypto** provides the trust foundation
2. **Time** provides the temporal framework
3. **State** provides the reality model
4. **Wire** provides the transport substrate

Every operation involves all four:

```rust
// Sending an event
fn emit_event(event: Event) {
    // 1. Crypto: Sign and encrypt
    let signed = crypto.sign(event);
    let encrypted = crypto.encrypt(signed, class);
    
    // 2. Time: Stamp with temporal intent
    let timed = time.stamp(encrypted);
    
    // 3. State: Validate against local field
    state.validate_outgoing(timed)?;
    
    // 4. Wire: Frame and send
    wire.send(timed);
}
```

## Key Insight

> Traditional protocols have layers that can fail independently.
> 
> ELARA has pillars that support each other - if one weakens, others compensate.

This is why ELARA can survive network chaos that would break traditional protocols.
