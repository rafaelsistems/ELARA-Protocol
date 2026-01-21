# ELARA System Science — Foundational Constructs v1

**Whitepaper-grade conceptual specification**

This document defines the scientific foundations of ELARA. Not marketing. Not motivation. A framework that can spawn papers, experiments, and implementations.

---

## Document Structure

| Section | Topic |
|---------|-------|
| I | Node Ontology |
| II | Event Algebra |
| III | Presence Metric |
| IV | Degradation Ladder |
| V | Chaos Test Specification |

---

## I. ELARA Node Ontology

*What is considered to "exist" in the ELARA world*

ELARA does not recognize "client", "server", "peer" as ontological classes.

ELARA only recognizes:

> **Entity that emits, receives, and transforms reality events.**

### Ontological Node Classes

#### 1. Origin Nodes

Entities that generate primary events.

**Examples:**
- Human (mic, camera, touch, bio-signal)
- Sensor
- AI agents
- Simulation

**Properties:**
| Property | Description |
|----------|-------------|
| Private perception | Has internal state not shared |
| Local clock | Time is self-referenced |
| Cryptographic root | Self-authenticating identity |
| Event genesis capability | Can create new reality |

#### 2. Transmutation Nodes

Entities that transform the shape of reality.

**Examples:**
- Perceptual codec
- AI reconstructor
- Noise shaper
- Summarizer
- Emotion extractor

**Properties:**
| Property | Description |
|----------|-------------|
| Never source of truth | Cannot create authoritative events |
| Always produce derivative events | Output depends on input |
| Must preserve lineage | Transformation history maintained |

#### 3. Propagation Nodes

Entities that facilitate reality continuity.

**Examples:**
- Relay
- Mesh router
- Buffer swarm
- Temporal mirror
- Opportunistic cache

**Properties:**
| Property | Description |
|----------|-------------|
| No authority | Cannot modify event meaning |
| No final state | Stateless or ephemeral |
| Replaceable | Can be substituted |
| Blind by default | Cannot read content |

#### 4. Witness Nodes

Entities that give meaning and memory.

**Examples:**
- User device
- Archive
- Timeline builder
- Forensic node
- Auditor AI

**Properties:**
| Property | Description |
|----------|-------------|
| Perspective-bound | Has a point of view |
| Builds projections | Creates state from events |
| Evaluates trust | Judges event validity |
| Measures presence | Assesses reality quality |

### Node Class Composition

> **A single node can belong to multiple classes simultaneously.**

```
┌─────────────────────────────────────────────────────────┐
│                    ELARA NODE                            │
├─────────────┬─────────────┬─────────────┬───────────────┤
│   Origin    │Transmutation│ Propagation │   Witness     │
│   [O]       │    [T]      │    [P]      │     [W]       │
└─────────────┴─────────────┴─────────────┴───────────────┘

Examples:
- Smartphone: [O, T, W]
- Relay server: [P]
- AI agent: [O, T, W]
- Archive: [P, W]
```

---

## II. ELARA Event Algebra

*Internal laws of occurrence*

ELARA views the world as an **event algebra space**, not a packet stream.

### Event as Fundamental Element

An ELARA event is not a "message", but:

> **ΔReality = signed transformation of perceived world**

### Basic Operators

Let `e₁`, `e₂` be events:

| Operator | Name | Definition |
|----------|------|------------|
| `⊕` | **Composition** | Combining partial realities. `e₁ ⊕ e₂ → new experience` (not necessarily commutative) |
| `⊗` | **Transmutation** | Shape change without changing origin. `f ⊗ e → e′` (voice → emotion → text → avatar motion) |
| `≺` | **Causal Precedence** | Not time, but reality dependency. `e₁ ≺ e₂ → e₂ meaningless without e₁` |
| `∥` | **Co-presence** | Events that coexist though not synchronized. `e₁ ∥ e₂ → perceptual simultaneity` |
| `⊘` | **Degradation** | Form reduction without meaning destruction. `e ⊘ δ → e′` (voice → noise → breath → silence) |

### Algebraic Properties

| Property | Description |
|----------|-------------|
| Non-total order | Events are not totally ordered |
| Perspective-dependent associativity | `(e₁ ⊕ e₂) ⊕ e₃` may differ by observer |
| Meaning-preserving degradation | Degradation preserves semantic core |
| Lineage invariance | Origin chain is immutable |

### Fundamental Law

> **There is no "delete reality". There is only transformation.**

### Formal Notation

```
Event Space: E = {e | e is signed, immutable, causally linked}

Composition: ⊕ : E × E → E
Transmutation: ⊗ : (E → E) × E → E
Precedence: ≺ ⊆ E × E (partial order)
Co-presence: ∥ ⊆ E × E (symmetric)
Degradation: ⊘ : E × Δ → E

Axioms:
1. ∀e ∈ E: lineage(e) is immutable
2. ∀e ∈ E: ∃ minimal form m such that e ⊘ δ* → m, m ≠ ∅
3. e₁ ≺ e₂ ∧ e₂ ≺ e₃ → e₁ ≺ e₃ (transitivity)
4. ¬(e ≺ e) (irreflexivity)
```

---

## III. ELARA Presence Metric

*How "presence" is measured*

ELARA does not measure quality with:
- ❌ Bitrate
- ❌ FPS
- ❌ Jitter
- ❌ Packet loss

ELARA measures:

> **Whether reality still feels alive.**

### Presence as Vector

Presence is not boolean. Presence is a vector:

```
P = ⟨L, I, C, R, E⟩
```

| Component | Name | Question |
|-----------|------|----------|
| **L** | Liveness | Is there still a sign of existence? |
| **I** | Immediacy | How close to "now"? |
| **C** | Coherence | Can it still be understood? |
| **R** | Relational Continuity | Does the relationship still feel intact? |
| **E** | Emotional Bandwidth | Is emotional meaning still carried? |

### Presence Measurement

Each component is measured on scale [0.0, 1.0]:

```rust
struct PresenceVector {
    /// Is there still a sign of existence? [0.0 - 1.0]
    pub liveness: f32,
    
    /// How close to "now"? [0.0 - 1.0]
    pub immediacy: f32,
    
    /// Can it still be understood? [0.0 - 1.0]
    pub coherence: f32,
    
    /// Does the relationship still feel intact? [0.0 - 1.0]
    pub relational_continuity: f32,
    
    /// Is emotional meaning still carried? [0.0 - 1.0]
    pub emotional_bandwidth: f32,
}

impl PresenceVector {
    /// Overall presence score
    pub fn score(&self) -> f32 {
        (self.liveness + self.immediacy + self.coherence 
         + self.relational_continuity + self.emotional_bandwidth) / 5.0
    }
    
    /// Presence is alive if any component > 0
    pub fn is_alive(&self) -> bool {
        self.liveness > 0.0 || self.immediacy > 0.0 || self.coherence > 0.0
            || self.relational_continuity > 0.0 || self.emotional_bandwidth > 0.0
    }
}
```

### Examples

**Voice call with 200ms delay, but natural:**
```
P = ⟨0.9, 0.8, 0.9, 0.9, 0.9⟩ → High presence
```

**Real-time text but spam, noise, out of context:**
```
P = ⟨0.9, 0.9, 0.2, 0.2, 0.1⟩ → Low presence despite high L and I
```

### Presence Priority

> **ELARA prefers complete presence at low bitrate over high bitrate with dead presence.**

```
Preferred: P = ⟨0.7, 0.7, 0.7, 0.7, 0.7⟩ @ 10kbps
Over:      P = ⟨0.9, 0.9, 0.2, 0.2, 0.1⟩ @ 1Mbps
```

---

## IV. ELARA Degradation Ladder

*How the system descends without collapsing*

ELARA defines an official degradation ladder. No improvisation per engineer.

### Reality Levels

| Level | Name | Description | Example |
|-------|------|-------------|---------|
| **L0** | Full Perception | Continuous voice/video, rich semantics | HD video call |
| **L1** | Distorted Perception | Noise, blur, jitter, but continuous | Choppy audio |
| **L2** | Fragmented Perception | Chunks, drops, approximations | Missing frames |
| **L3** | Symbolic Presence | Text, emotion tokens, activity traces | "User is speaking..." |
| **L4** | Minimal Presence | Breath, pulse, alive-signal, echoes | Heartbeat indicator |
| **L5** | Latent Presence | Signed silence, delayed reality, memory | "Last seen 5m ago" |

### Visual Representation

```
L0 ████████████████████████████ Full Perception
   │
   ▼ (network pressure)
L1 ██████████████████████░░░░░░ Distorted Perception
   │
   ▼ (more pressure)
L2 ██████████████░░░░░░░░░░░░░░ Fragmented Perception
   │
   ▼ (severe pressure)
L3 ████████░░░░░░░░░░░░░░░░░░░░ Symbolic Presence
   │
   ▼ (critical pressure)
L4 ████░░░░░░░░░░░░░░░░░░░░░░░░ Minimal Presence
   │
   ▼ (extreme pressure)
L5 █░░░░░░░░░░░░░░░░░░░░░░░░░░░ Latent Presence

⚠️ NEVER: ░░░░░░░░░░░░░░░░░░░░░░░░░░░░ Disconnected
```

### Hard Rules

| Rule | Description |
|------|-------------|
| **Gradual transition** | Must move one level at a time |
| **No jump to zero** | Cannot skip directly to disconnected |
| **Presence > 0** | Presence metric must always be positive |
| **Lineage intact** | Event lineage must never break |

### Level Transitions

```rust
enum DegradationLevel {
    L0_FullPerception,
    L1_DistortedPerception,
    L2_FragmentedPerception,
    L3_SymbolicPresence,
    L4_MinimalPresence,
    L5_LatentPresence,
}

impl DegradationLevel {
    /// Get next lower level (more degraded)
    fn degrade(&self) -> Option<Self> {
        match self {
            Self::L0_FullPerception => Some(Self::L1_DistortedPerception),
            Self::L1_DistortedPerception => Some(Self::L2_FragmentedPerception),
            Self::L2_FragmentedPerception => Some(Self::L3_SymbolicPresence),
            Self::L3_SymbolicPresence => Some(Self::L4_MinimalPresence),
            Self::L4_MinimalPresence => Some(Self::L5_LatentPresence),
            Self::L5_LatentPresence => None, // Cannot degrade further
        }
    }
    
    /// Get next higher level (less degraded)
    fn improve(&self) -> Option<Self> {
        match self {
            Self::L0_FullPerception => None, // Already at best
            Self::L1_DistortedPerception => Some(Self::L0_FullPerception),
            Self::L2_FragmentedPerception => Some(Self::L1_DistortedPerception),
            Self::L3_SymbolicPresence => Some(Self::L2_FragmentedPerception),
            Self::L4_MinimalPresence => Some(Self::L3_SymbolicPresence),
            Self::L5_LatentPresence => Some(Self::L4_MinimalPresence),
        }
    }
}
```

### Fundamental Law

> **There is no "disconnected". There is only: the most minimal form of reality.**

---

## V. ELARA Chaos Test Specification

*How ELARA is tested*

ELARA is not valid if tested only with:
- ❌ Latency
- ❌ Throughput
- ❌ Packet loss

ELARA must be tested with:

> **Existential chaos.**

### Chaos Categories

#### 1. Ontological Chaos

Testing identity and reality coherence.

| Test | Description |
|------|-------------|
| Node identity change | Node changes keys mid-session |
| Forked history | Conflicting event chains |
| AI hallucination | Synthetic events injected |
| Sensor spoofing | Fake origin data |

#### 2. Temporal Chaos

Testing time model resilience.

| Test | Description |
|------|-------------|
| Massive clock skew | Hours/days of drift |
| Delayed days | Events arriving days late |
| Brutal reordering | Completely shuffled events |
| Causal holes | Missing causal dependencies |

#### 3. Topological Chaos

Testing network resilience.

| Test | Description |
|------|-------------|
| Extended partition | Days of disconnection |
| Swarm join/leave | Rapid membership changes |
| Mobile blackout | Complete connectivity loss |
| Relay death storms | All relays failing |

#### 4. Adversarial Chaos

Testing security model.

| Test | Description |
|------|-------------|
| Malicious injection | Fake events from attackers |
| Replay attack | Old events replayed |
| Perception poisoning | Corrupted media streams |
| Presence forgery | Fake presence signals |

#### 5. Perceptual Chaos

Testing human experience continuity.

| Test | Description |
|------|-------------|
| Extreme jitter | 0-2000ms random delay |
| Half streams | Only partial data arriving |
| Emotional desync | Mismatched emotional cues |
| Semantic loss | Meaning lost in transmission |

### Success Criteria

ELARA success is NOT:
> "Connection survived"

ELARA success IS:

| Criterion | Measurement |
|-----------|-------------|
| Presence metric > 0 | `P.is_alive() == true` |
| Degradation ladder maintained | Level transitions are gradual |
| Event lineage intact | All events traceable to origin |
| Identity continuity alive | Cryptographic chain unbroken |

### Chaos Test Matrix

| Chaos Type | Must Survive | Presence Floor |
|------------|--------------|----------------|
| Ontological | Identity coherence | L4 |
| Temporal | Causal reconstruction | L3 |
| Topological | Eventual reconnection | L5 |
| Adversarial | Trust isolation | L3 |
| Perceptual | Experience continuity | L2 |

### Formal Test Specification

```rust
struct ChaosTest {
    category: ChaosCategory,
    duration: Duration,
    intensity: f32,  // 0.0 - 1.0
    success_criteria: SuccessCriteria,
}

struct SuccessCriteria {
    /// Minimum presence that must be maintained
    min_presence: PresenceVector,
    
    /// Maximum degradation level allowed
    max_degradation: DegradationLevel,
    
    /// Lineage must remain intact
    lineage_intact: bool,
    
    /// Identity must remain continuous
    identity_continuous: bool,
}

fn run_chaos_test(test: &ChaosTest, system: &mut ElaraSystem) -> TestResult {
    // Apply chaos
    apply_chaos(test.category, test.intensity, test.duration);
    
    // Measure
    let presence = system.measure_presence();
    let level = system.degradation_level();
    let lineage = system.check_lineage();
    let identity = system.check_identity();
    
    // Evaluate
    TestResult {
        passed: presence.is_alive() 
            && level <= test.success_criteria.max_degradation
            && lineage == test.success_criteria.lineage_intact
            && identity == test.success_criteria.identity_continuous,
        presence,
        level,
        lineage,
        identity,
    }
}
```

---

## Summary

At this point, ELARA is no longer a "communication protocol".

It is:

> **A model of how digital reality survives in chaos.**

### What ELARA Is

| Not | But |
|-----|-----|
| App | Reality substrate |
| Protocol | Existence framework |
| Library | Domain science |

### The Five Constructs

| Construct | Defines |
|-----------|---------|
| Node Ontology | What exists |
| Event Algebra | How things change |
| Presence Metric | What matters |
| Degradation Ladder | How to survive |
| Chaos Test | How to validate |

---

## Relationship to Other Documents

| Document | Relationship |
|----------|--------------|
| Hard Invariants | System laws that these constructs must obey |
| Protocol Model | Operational models derived from invariants |
| **System Science** | **Foundational constructs for research and implementation** |

---

<p align="center">
<b>If you continue this consistently,<br>
you are not building an app.<br>
You are not building a protocol.<br>
You are birthing a new domain.</b>
</p>
