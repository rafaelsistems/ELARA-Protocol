# ELARA Protocol — Hard Invariants v0.1

**These are not slogans. These are system laws.**

Every layer of ELARA (network, crypto, media, client, AI, UI) must obey these invariants. If any single invariant falls, the system is not ELARA.

---

## The Litmus Test

Before any technical decision, ask:

> **"If this is removed, do the invariants still hold?"**

If the answer is **no**, then that decision is not part of ELARA.

---

## 🥇 Invariant #1 — Reality Never Waits

**Reality does not wait for the network. The system must never block reality for synchronization.**

### Technical Meaning

- No design assumes connection-first
- Events are always allowed to occur even when:
  - Socket is dead
  - Peer is not found
  - State is not synchronized
- All layers must be **event-driven**, not session-driven

### Architectural Consequences

| Requirement | Implementation |
|-------------|----------------|
| Local-first capture | Events recorded locally before sync |
| Append-only event log | Immutable event history |
| Opportunistic sync | Sync when possible, not required |
| No hard channel dependency | Works without continuous connection |

### Violation Test

> If ELARA requires "stable connection before it can function" → **INVARIANT VIOLATED**

---

## 🥈 Invariant #2 — Presence Over Packets

**What is preserved is presence (existence), not data perfection.**

### Technical Meaning

The protocol prioritizes:
- ✅ **Liveness** — proof of existence
- ✅ **Immediacy** — real-time awareness
- ✅ **Continuity of perception** — unbroken experience

Over:
- ❌ Absolute reliability
- ❌ Absolute ordering
- ❌ Absolute completeness

### Architectural Consequences

| Principle | Implementation |
|-----------|----------------|
| Partial data is valid state | Incomplete ≠ Invalid |
| Silence ≠ failure | No data is still presence |
| Lost packet ≠ lost presence | Entity persists beyond packets |
| Liveness semantics in protocol | Not just payload delivery |

### Violation Test

> If ELARA only cares about "packet delivery" → **That's RTP, not ELARA**

---

## 🥉 Invariant #3 — Experience Degrades, Never Collapses

**Experience quality may decrease, but it must never collapse.**

### Technical Meaning

No binary failures:
- ❌ "Call dropped"
- ❌ "Session dead"
- ❌ "Connection lost"

All subsystems must have:
- ✅ Fallback mode
- ✅ Perceptual smoothing
- ✅ Prediction / concealment / approximation

### Degradation Path

```
Full Quality
    ↓
Reduced Quality
    ↓
Minimal Quality
    ↓
Symbolic Representation
    ↓
Presence Indicator
    ↓
Identity Heartbeat

⚠️ NEVER: Disconnection / Collapse / Fatal Error
```

### Concrete Example (Voice)

```
HD Voice → Standard Voice → Low Quality → Robot Voice → 
Choppy Audio → Digital Breath → Meaningful Silence

NEVER: "Call Ended" / "Connection Lost"
```

### Violation Test

> If ELARA has many "fatal errors" → **INVARIANT VIOLATED**

---

## 🏅 Invariant #4 — Event Is Truth, State Is Projection

**Truth is the occurrence, not the condition.**

### Technical Meaning

```
Event Log > Synchronized State
```

State is merely:
- A cache
- A projection
- An approximation

There is **no "single source of truth"** in the form of server state.

### Architectural Consequences

| Principle | Implementation |
|-----------|----------------|
| CRDT-based | Conflict-free replicated data types |
| Causal graph | Events linked by causality |
| Vector time | Logical clocks, not wall clocks |
| Hash-chain | Cryptographic event linking |
| Reconciliation is normal | Expected, not exceptional |
| Divergence is expected | Not an error condition |
| Convergence is eventual | Not a prerequisite |

### Violation Test

> If ELARA depends on central state → **That's a backend system, not ELARA**

---

## 🏆 Invariant #5 — Identity Survives Transport

**Identity and relationships must not depend on communication channels.**

### Technical Meaning

```
Identity ≠ Socket
Presence ≠ Session
Trust ≠ IP / Server
```

### Architectural Consequences

| Principle | Implementation |
|-----------|----------------|
| Self-authenticating identity | Cryptographic proof of identity |
| Cryptographic continuity | Keys persist across sessions |
| Session death ≠ relationship death | Connections are ephemeral, identity is not |

### Violation Test

> If when connection dies "entity dies" → **INVARIANT VIOLATED**

---

## Formal Constraints

If ELARA truly holds these invariants, then:

| Layer | Constraint |
|-------|------------|
| **Packet Model** | Must NOT assume reliability |
| **Protocol** | Must NOT be session-centric |
| **Client** | Must be local-sovereign |
| **Media Stack** | Must be perceptual-first |
| **Crypto Layer** | Must be identity-first |
| **Server** | Only relay/catalyst, NOT brain |

---

## What Makes ELARA Different

| System | Nature |
|--------|--------|
| WebRTC | Media transport |
| QUIC | Reliable transport |
| Signal | Secure messaging |
| SIP | Session control |
| **ELARA** | **Substrate for distributed reality** |

---

## Invariant Compliance Matrix

Every component must be evaluated against all invariants:

| Component | INV-1 | INV-2 | INV-3 | INV-4 | INV-5 |
|-----------|-------|-------|-------|-------|-------|
| elara-core | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-wire | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-crypto | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-time | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-state | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-transport | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-runtime | ✅ | ✅ | ✅ | ✅ | ✅ |
| elara-msp | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## Code-Level Enforcement

These invariants are encoded in `elara-core` as compile-time and runtime checks:

```rust
/// ELARA Hard Invariants
/// 
/// These are system laws, not guidelines.
/// Violation of any invariant means the system is not ELARA.
pub mod invariants {
    /// INV-1: Reality Never Waits
    /// - No blocking on network
    /// - Event-driven, not session-driven
    /// - Local-first capture
    pub const REALITY_NEVER_WAITS: &str = "INV-1";
    
    /// INV-2: Presence Over Packets
    /// - Liveness > reliability
    /// - Partial data is valid
    /// - Silence ≠ failure
    pub const PRESENCE_OVER_PACKETS: &str = "INV-2";
    
    /// INV-3: Experience Degrades, Never Collapses
    /// - No binary failures
    /// - Graceful degradation required
    /// - No fatal errors
    pub const EXPERIENCE_DEGRADES_NEVER_COLLAPSES: &str = "INV-3";
    
    /// INV-4: Event Is Truth, State Is Projection
    /// - Event log > synchronized state
    /// - No central source of truth
    /// - Eventual convergence
    pub const EVENT_IS_TRUTH: &str = "INV-4";
    
    /// INV-5: Identity Survives Transport
    /// - Identity ≠ socket
    /// - Cryptographic continuity
    /// - Relationships persist
    pub const IDENTITY_SURVIVES_TRANSPORT: &str = "INV-5";
}
```

---

## Review Checklist

For every PR, code review, and design decision:

- [ ] **INV-1**: Does this block on network? Does this assume connection-first?
- [ ] **INV-2**: Does this prioritize packet delivery over presence?
- [ ] **INV-3**: Does this introduce binary failure modes?
- [ ] **INV-4**: Does this depend on central state?
- [ ] **INV-5**: Does this tie identity to transport?

If any answer is **yes**, the change violates ELARA invariants.

---

<p align="center">
<b>If one falls, ELARA is not ELARA.</b>
</p>
