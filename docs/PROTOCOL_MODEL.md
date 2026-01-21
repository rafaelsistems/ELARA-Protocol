# ELARA Protocol — Formal Protocol Model v1

**Derived strictly from ELARA Hard Invariants**

This document defines the five foundational models that govern how ELARA perceives and handles reality. Every module must be explainable through these models.

---

## The Five Models

| Model | ELARA Treats As |
|-------|-----------------|
| **Failure** | Distortion field |
| **Timing** | Local perception axis |
| **Trust** | Cryptographic continuity |
| **Event** | Ontological truth |
| **Media** | Perceptual fabric |

---

## 1️⃣ Failure Model

*How ELARA perceives failure*

### Axiom

> **Failure is distortion, not termination.**

In ELARA, failure is NOT:
- Connection dropped
- Peer lost
- State not synchronized
- Data incomplete

Failure IS:
- **Change in the shape of reality**

### Hard Assumptions

ELARA always assumes:

| Condition | Status |
|-----------|--------|
| Extreme jitter | Normal |
| Arbitrary packet loss | Normal |
| Reordering | Normal |
| Duplication | Normal |
| Extended network partition | Normal |
| Malicious/Byzantine nodes | Normal |
| Device sleep/kill/reboot | Normal |
| Large clock drift | Normal |
| Total server death | Normal |

**None of these conditions are considered exceptional state.**

### Protocol Consequences

- No "connection failure" — only **perception degradation**
- No "session invalidation" — only **presence uncertainty**
- All layers must be:
  - **Resumable** — can continue from any point
  - **Idempotent** — same operation, same result
  - **Causally tolerant** — handles out-of-order events

### Formal Properties

```
P1: No single point of experiential failure
P2: No fatal dependency on continuous transport
P3: All subsystems must support partial reality mode
```

### Module Compliance Question

> "What is your failure model?"

If a module cannot answer this in terms of distortion rather than termination, it is not part of ELARA core.

---

## 2️⃣ Timing Model

*How ELARA treats time*

### Axiom

> **Time is local. Order is reconstructed.**

ELARA never considers time as global.

### Hard Assumptions

| Assumption | Implication |
|------------|-------------|
| Clocks between nodes are not synchronized | No wall-clock dependency |
| Delay is unpredictable | No timing guarantees |
| Events can arrive late | Must handle retroactive events |
| Events can arrive reversed | Must reconstruct order |
| Events can be duplicated | Must be idempotent |
| Events can arrive after context changed | Must handle stale context |

### Protocol Consequences

All events carry:
- **Logical time** (Lamport / Hybrid / Vector)
- **Causal parent / frontier hash**

Ordering is:
- **Contextual** — depends on use case
- **Approximate** — not exact
- **Purpose-driven** — media ≠ state ≠ UI

### Formal Properties

```
P1: No protocol decision depends on wall-clock correctness
P2: Causality > Timestamp
P3: Liveness > Consistency
P4: Reconstruction > Synchronization
```

### Module Compliance Question

> "Whose clock does your time depend on?"

If a module depends on wall-clock correctness, it is not part of ELARA core.

---

## 3️⃣ Trust Model

*How ELARA perceives identity and trust*

### Axiom

> **Identity is cryptographic, not topological.**

### Hard Assumptions

| Assumption | Implication |
|------------|-------------|
| All nodes are potentially malicious | Zero trust by default |
| Infrastructure is not trusted | Server blindness |
| Network location is meaningless | IP ≠ identity |
| Session does not prove identity | Session ≠ trust |

### Protocol Consequences

All entities = **self-authenticating identity**

All events are:
- **Signed** — cryptographic proof of authorship
- **Hash-chained** — immutable history

Trust is built from:
- ✅ Continuity of keys
- ✅ Behavior history
- ✅ Event lineage

Trust is NOT built from:
- ❌ IP address
- ❌ Server attestation
- ❌ TLS channel
- ❌ Login session

### Formal Properties

```
P1: Compromised transport ≠ Compromised reality
P2: Server blindness by construction
P3: Trust is event-derived, not authority-granted
```

### Module Compliance Question

> "Where does your trust come from?"

If a module derives trust from topology (IP, server, session), it is not part of ELARA core.

---

## 4️⃣ Event Model

*What is "truth" in ELARA*

### Axiom

> **Event is truth. State is a story we tell about it.**

### Basic Structure

ELARA considers only one thing sacred:

**Signed, immutable, causally linked events**

Every event minimally carries:

```rust
struct ElaraEvent {
    /// H(payload + parent_refs)
    event_id: EventHash,
    
    /// Self-authenticating author
    author_identity: NodeId,
    
    /// Logical time (not wall clock)
    logical_time: LogicalTime,
    
    /// Causal dependencies
    causal_frontier: Vec<EventHash>,
    
    /// What kind of event
    semantic_type: EventType,
    
    /// For media: perceptual importance
    perceptual_weight: Option<PerceptualWeight>,
}
```

### Protocol Consequences

- No "final state"
- No "hard sync"
- All state is:
  - **Projection** — derived from events
  - **Merge** — combined from multiple sources
  - **Lens** — viewed from a perspective

**Conflict is not error. Conflict is difference of perspective.**

### Formal Properties

```
P1: Append-only reality
P2: Forks are legal
P3: Convergence is negotiated, not enforced
P4: History > Snapshot
```

### Module Compliance Question

> "Where is your event-truth?"

If a module treats state as authoritative rather than events, it is not part of ELARA core.

---

## 5️⃣ Media Semantics Model

*How voice/video/stream is understood*

### Axiom

> **Media is perception, not data.**

### Hard Assumptions

| Assumption | Implication |
|------------|-------------|
| Media stream is never complete | Partial is normal |
| Emotional continuity > bit-perfect | Perception first |
| Latency is more dangerous than loss | Immediacy matters |

### Protocol Consequences

Media unit in ELARA is not "packet" but:

**Perceptual Atoms**

Each atom carries:
- **Temporal intent** — when it should be perceived
- **Continuity hint** — how it connects to neighbors
- **Emotional/presence weight** — importance for experience
- **Reconstructability class** — how to handle if missing

### Reconstructability Classes

| Class | Meaning | Example |
|-------|---------|---------|
| `must-feel` | Critical for experience | Voice onset, key visual |
| `may-approximate` | Can be estimated | Mid-syllable, motion blur |
| `reconstructable` | Can be synthesized | Silence, static background |
| `ignorable` | Can be dropped | Enhancement, cosmetic |

### Formal Properties

```
P1: Silence is valid media
P2: Approximation is first-class
P3: Prediction is legal
P4: Media pipelines must support:
    - Stretching
    - Concealment
    - Hallucination
    - Semantic fallback
```

### Module Compliance Question

> "If media has holes, what still lives?"

If a module cannot answer this, it is not part of ELARA core.

---

## Formal Summary

| Model | Core Question | Valid Answer Pattern |
|-------|---------------|---------------------|
| **Failure** | What is your failure model? | Distortion, not termination |
| **Timing** | Whose clock do you depend on? | Local only, reconstructed order |
| **Trust** | Where does trust come from? | Cryptographic continuity |
| **Event** | Where is your truth? | In signed, immutable events |
| **Media** | What lives when media has holes? | Perception, approximation |

---

## What This Means Practically

If this model is followed, then ELARA:

| Cannot Be | Reason |
|-----------|--------|
| Built like WebRTC | WebRTC assumes reliable transport |
| Modeled like SIP | SIP is session-centric |
| Implemented like chat server | Chat servers have central state |
| Tested only with latency/loss | Must test chaos, perception, partition |

### ELARA Must Be Tested With

- ✅ Chaos (random failures)
- ✅ Perception (human experience)
- ✅ Long partition (extended disconnection)
- ✅ Adversarial nodes (malicious actors)
- ✅ Device death (sudden termination)

---

## Module Compliance Checklist

For every module, answer these five questions:

| # | Question | Required Answer Pattern |
|---|----------|------------------------|
| 1 | What is your failure model? | Distortion field |
| 2 | Where is your event-truth? | Signed immutable events |
| 3 | Whose clock do you depend on? | Local only |
| 4 | Where does your trust come from? | Cryptographic continuity |
| 5 | If media has holes, what lives? | Perception continues |

**If a module cannot be explained through these 5 models → it is not part of ELARA core.**

---

## Relationship to Hard Invariants

| Model | Derived From Invariant |
|-------|----------------------|
| Failure Model | INV-3: Experience Degrades, Never Collapses |
| Timing Model | INV-1: Reality Never Waits |
| Trust Model | INV-5: Identity Survives Transport |
| Event Model | INV-4: Event Is Truth, State Is Projection |
| Media Model | INV-2: Presence Over Packets |

---

<p align="center">
<b>Every module must answer the five questions.<br>
If it cannot, it is not ELARA.</b>
</p>
