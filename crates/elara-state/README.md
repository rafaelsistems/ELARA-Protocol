# elara-state

State field engine for the ELARA Protocol - featuring CRDT reconciliation, divergence control, and partition tolerance.

## Features

- **State Field**: Distributed state as a physical field phenomenon
- **CRDT Reconciliation**: Conflict-free replicated data types
- **Divergence Control**: Graceful degradation under network stress
- **Authority System**: Fine-grained permission model
- **Partition Tolerance**: Continues operating during network splits

## Quick Start

```rust
use elara_state::{StateField, StateAtom, Authority};
use elara_core::{NodeId, StateId};

// Create a state field
let mut field = StateField::new(node_id);

// Create a state atom
let atom = StateAtom::new(
    state_id,
    authority,
    data,
    version_vector
);

// Apply state change
field.apply_atom(atom)?;

// Reconcile with remote state
field.reconcile(remote_field)?;
```

## State Field Concept

State exists as a **field** that propagates through the network like a physical phenomenon:

```
Node A ←→ Node B ←→ Node C
  ↕        ↕        ↕
Field propagation with divergence control
```

## CRDT Reconciliation

### Properties
- **Commutative**: Order of operations doesn't matter
- **Associative**: Grouping of operations doesn't matter  
- **Idempotent**: Duplicate operations have no effect

### Process
```
1. Authority Check → Validate permissions
2. Causality Check → Ensure dependencies exist
3. Temporal Placement → Map to state time
4. Delta Merge → Non-destructive combination
5. Divergence Control → Reduce detail if needed
6. Swarm Diffusion → Propagate to interested peers
```

## Authority System

```rust
pub enum Authority {
    Open,           // Anyone can modify
    Shared(Set),    // Specific nodes can modify
    Exclusive(NodeId), // Only one node can modify
    Frozen,         // No modifications allowed
}
```

## Divergence Control

When network conditions degrade:

```
Full Detail → Reduced Detail → Symbolic → Presence Only
     ↓              ↓              ↓           ↓
  Complete      Essential     Minimal     Existence
  State         Data           State       Proof
```

## Partition Handling

```
Normal:     A ←→ B ←→ C ←→ D
Partition:  A ←→ B    C ←→ D
            (sub1)    (sub2)
Merge:      Automatic reconciliation when partition heals
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.