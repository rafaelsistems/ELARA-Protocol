# ELARA Core Concepts

## Fundamental Primitives

ELARA recognizes only five fundamental primitives. Everything else is derived.

### 1. State (ω) - Living Reality

State is not data. State is **living reality** that exists, evolves, and can be observed.

```
ω = {
    id: StateId,
    type: StateType,           // core | perceptual | enhancement | cosmetic
    authority_set: Set<NodeId>,
    clock_model: ClockModel,
    version_vector: VersionVector,
    delta_law: DeltaLaw,
    bounds: StateBounds,
    entropy_model: EntropyModel,
    value: StateValue
}
```

**State Types:**
- **Core** - Essential state that must survive (identity, session membership)
- **Perceptual** - Real-time sensory state (voice parameters, video frames)
- **Enhancement** - Quality improvements (HD details, effects)
- **Cosmetic** - Non-essential decorations (reactions, typing indicators)

### 2. Event (ε) - Lawful Mutation

Events are the **only** way state can change. No direct mutation allowed.

```
ε = {
    id: EventId,
    source: NodeId,
    target_state: StateId,
    version_ref: VersionVector,
    mutation: MutationOp,
    time_intent: TimeIntent,
    authority_proof: AuthorityProof,
    entropy_hint: EntropyHint
}
```

**Event Laws:**
1. Events must reference valid state
2. Events must carry authority proof
3. Events must declare temporal intent
4. Events are immutable once created

### 3. Time (τ) - Protocol Object

Time in ELARA is not a timestamp. It's a **first-class protocol object**.

```
Dual Clock System:
├── τp (Perceptual Time)
│   ├── Monotonic
│   ├── Smooth
│   ├── Local-driven
│   └── NEVER jumps
│
└── τs (State Time)
    ├── Elastic
    ├── Drift-correctable
    ├── Convergence-oriented
    └── CAN be bent/corrected
```

**Reality Window:**
```
RW = [τs - Hc, τs + Hp]

Where:
- Hc = Correction Horizon (how far back we can fix)
- Hp = Prediction Horizon (how far ahead we predict)
```

### 4. Authority - Mutation Rights

Authority determines **who can change what**.

```
AuthorityScope = {
    state_pattern: StatePattern,  // Which states
    operations: Set<MutationOp>,  // Which operations
    constraints: Constraints,      // Under what conditions
    delegation: DelegationRules    // Can it be delegated?
}
```

**Authority Principles:**
- Authority is cryptographically bound to identity
- Authority can be delegated with proof chain
- Authority violations are detected and rejected
- No implicit authority - everything explicit

### 5. Interest - Observation Rights

Interest determines **who needs to see what**.

```
InterestScope = {
    state_pattern: StatePattern,
    granularity: Granularity,
    priority: Priority,
    decay: DecayModel
}
```

**Interest Principles:**
- Interest drives state diffusion
- Interest affects resource allocation
- Interest can be inferred or declared
- Interest enables efficient routing

## Communication Modalities as State

All communication modalities are just **configurations of state**:

| Modality | State Type | Delta Law |
|----------|------------|-----------|
| Chat | Discrete semantic | Append-only, causal |
| Voice | Continuous perceptual | Frame-based, lossy |
| Video | Visual projection | Keyframe + delta |
| Presence | Identity existence | Last-write-wins |
| Typing | Transient indicator | Ephemeral |
| AI Agent | Autonomous authority | Event-driven |

## Version Vectors

ELARA uses version vectors for causal ordering:

```rust
VersionVector = Map<NodeId, u64>

// Comparison
v1.happens_before(v2)  // v1 causally precedes v2
v1.concurrent_with(v2) // Neither precedes the other
v1.dominates(v2)       // v1 >= v2 for all entries

// Merge
v3 = v1.merge(v2)      // Element-wise max
```

## State Lifecycle

```
┌─────────────┐
│   Created   │ ← StateCreate event
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Active    │ ← StateUpdate events
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Archived   │ ← StateDelete event (soft)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Purged    │ ← Garbage collection
└─────────────┘
```

## Key Insight

> Traditional protocols ask: "How do I send this message?"
> 
> ELARA asks: "How do I synchronize this reality?"

The shift from message-passing to reality-synchronization enables:
- Natural handling of network chaos
- Graceful degradation under resource pressure
- Unified treatment of all modalities
- True end-to-end encryption without protocol awareness
