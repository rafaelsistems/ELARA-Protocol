# ELARA State Reconciliation Specification v0

State reconciliation ensures all nodes eventually reach **equivalent reality** under loss, partition, and adversarial conditions.

## Design Goals

1. **Convergence**: All nodes reach equivalent state
2. **Availability**: Operations continue during partitions
3. **Consistency**: Causal ordering preserved
4. **Resilience**: Graceful handling of Byzantine behavior

## State Atom Structure

```rust
struct StateAtom {
    // Identity
    id: StateId,
    state_type: StateType,  // Core | Perceptual | Enhancement | Cosmetic
    
    // Authority
    authority_set: HashSet<NodeId>,
    
    // Versioning
    version_vector: VersionVector,
    
    // Behavior
    delta_law: DeltaLaw,
    bounds: StateBounds,
    entropy_model: EntropyModel,
    
    // Value
    value: StateValue,
}
```

### State Types

| Type | Priority | Drop Policy | Example |
|------|----------|-------------|---------|
| Core | Highest | Never | Identity, membership |
| Perceptual | High | Drop old | Voice, video |
| Enhancement | Medium | Under pressure | HD, effects |
| Cosmetic | Low | Free | Reactions, typing |

### Delta Laws

```rust
enum DeltaLaw {
    // Replace entirely
    LastWriteWins,
    
    // Append-only log
    AppendOnly,
    
    // Set operations
    SetCRDT { add_wins: bool },
    
    // Counter
    PNCounter,
    
    // Custom merge function
    Custom { merge_fn: MergeFn },
    
    // Time-limited
    Ephemeral { ttl: Duration },
    
    // Frame-based (for continuous state)
    FrameBased { interval: Duration },
}
```

## Event Structure

```rust
struct Event {
    // Identity
    id: EventId,
    event_type: EventType,
    
    // Source
    source: NodeId,
    
    // Target
    target_state: StateId,
    version_ref: VersionVector,
    
    // Mutation
    mutation: MutationOp,
    
    // Temporal
    time_intent: TimeIntent,
    
    // Authority
    authority_proof: AuthorityProof,
    
    // Hints
    entropy_hint: EntropyHint,
}
```

### Mutation Operations

```rust
enum MutationOp {
    Set(Vec<u8>),
    Increment(i64),
    Append(Vec<u8>),
    Merge(Vec<u8>),
    Delete,
    Blend { value: Vec<u8>, weight: f32 },
}
```

## Reconciliation Pipeline

Every incoming event passes through six stages:

```
┌─────────────────────────────────────────────────────────────┐
│                    INCOMING EVENT                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 1: AUTHORITY CHECK                                     │
│ - Is source in authority_set for target state?               │
│ - Is authority_proof valid?                                  │
│ - Is delegation chain valid (if present)?                    │
│                                                              │
│ REJECT if unauthorized                                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 2: CAUSALITY CHECK                                     │
│ - Does version_ref match or precede current version?         │
│ - Are all causal dependencies satisfied?                     │
│                                                              │
│ QUARANTINE if dependencies missing                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 3: TEMPORAL PLACEMENT                                  │
│ - Map event time to local τs via peer time model             │
│ - Classify: TooOld | Correctable | Current | Predicted       │
│                                                              │
│ ARCHIVE if too old, QUARANTINE if too future                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 4: DELTA MERGE                                         │
│ - Apply mutation according to delta_law                      │
│ - Non-destructive: merge, don't replace                      │
│ - Bounded: respect state bounds                              │
│ - Reversible: can undo if needed                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 5: DIVERGENCE CONTROL                                  │
│ - Check entropy against threshold                            │
│ - If diverging: reduce detail, increase entropy tolerance    │
│ - Simplify, don't disconnect                                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 6: SWARM DIFFUSION                                     │
│ - Update relevance/heat maps                                 │
│ - Determine which peers need this update                     │
│ - Queue for transmission                                     │
└─────────────────────────────────────────────────────────────┘
```

## Stage Details

### Stage 1: Authority Check

```rust
fn check_authority(event: &Event, state: &StateAtom) -> AuthorityResult {
    // Direct authority
    if state.authority_set.contains(&event.source) {
        if verify_signature(&event.authority_proof, &event) {
            return AuthorityResult::Authorized;
        }
        return AuthorityResult::InvalidProof;
    }
    
    // Delegated authority
    if let Some(chain) = &event.authority_proof.delegation_chain {
        if verify_delegation_chain(chain, &state.authority_set, &event.source) {
            return AuthorityResult::Authorized;
        }
        return AuthorityResult::InvalidDelegation;
    }
    
    AuthorityResult::Unauthorized
}
```

### Stage 2: Causality Check

```rust
fn check_causality(event: &Event, state: &StateAtom) -> CausalityResult {
    let event_version = &event.version_ref;
    let current_version = &state.version_vector;
    
    if event_version.happens_before(current_version) 
       || event_version == current_version {
        // Event is causally valid
        CausalityResult::Valid
    } else if current_version.happens_before(event_version) {
        // We're behind - need to catch up
        CausalityResult::NeedSync
    } else {
        // Concurrent - can merge
        CausalityResult::Concurrent
    }
}
```

### Stage 3: Temporal Placement

```rust
fn place_temporally(
    event: &Event,
    time_engine: &TimeEngine
) -> TemporalResult {
    let local_time = time_engine.map_to_local_time(
        event.time_intent.timestamp,
        event.source
    );
    
    let position = time_engine.classify_time(local_time);
    
    match position {
        TimePosition::TooOld => TemporalResult::Archive,
        TimePosition::Correctable => TemporalResult::ApplyWithCorrection,
        TimePosition::Current => TemporalResult::ApplyNow,
        TimePosition::Predicted => TemporalResult::ApplyNow,
        TimePosition::TooFuture => TemporalResult::Quarantine,
    }
}
```

### Stage 4: Delta Merge

```rust
fn merge_delta(
    state: &mut StateAtom,
    event: &Event
) -> MergeResult {
    match &state.delta_law {
        DeltaLaw::LastWriteWins => {
            state.value = event.mutation.apply_to(&state.value);
            state.version_vector.increment(event.source);
        }
        
        DeltaLaw::AppendOnly => {
            if let MutationOp::Append(data) = &event.mutation {
                state.value.append(data);
                state.version_vector.increment(event.source);
            }
        }
        
        DeltaLaw::SetCRDT { add_wins } => {
            merge_set_crdt(state, event, *add_wins);
        }
        
        DeltaLaw::FrameBased { interval } => {
            // For continuous state, blend rather than replace
            if let MutationOp::Blend { value, weight } = &event.mutation {
                state.value.blend(value, *weight);
            }
        }
        
        // ... other laws
    }
    
    MergeResult::Success
}
```

### Stage 5: Divergence Control

```rust
fn control_divergence(
    state: &mut StateAtom,
    entropy: f64
) -> DivergenceResult {
    let threshold = state.entropy_model.threshold;
    
    if entropy < threshold {
        return DivergenceResult::Stable;
    }
    
    // Diverging - simplify state
    match state.state_type {
        StateType::Cosmetic => {
            // Drop cosmetic entirely
            state.value = StateValue::default();
            DivergenceResult::Simplified
        }
        StateType::Enhancement => {
            // Reduce quality
            state.value.reduce_quality(0.5);
            DivergenceResult::Simplified
        }
        StateType::Perceptual => {
            // Increase tolerance, keep essentials
            state.entropy_model.threshold *= 1.5;
            DivergenceResult::ToleranceIncreased
        }
        StateType::Core => {
            // Never simplify core - flag for manual resolution
            DivergenceResult::NeedsResolution
        }
    }
}
```

### Stage 6: Swarm Diffusion

```rust
fn update_diffusion(
    state: &StateAtom,
    event: &Event,
    interest_map: &InterestMap
) -> Vec<NodeId> {
    let mut recipients = Vec::new();
    
    for (node_id, interest) in interest_map.iter() {
        if interest.matches(&state.id) {
            let priority = calculate_priority(state, interest);
            recipients.push((node_id, priority));
        }
    }
    
    // Sort by priority, limit fanout
    recipients.sort_by_key(|(_, p)| std::cmp::Reverse(*p));
    recipients.truncate(MAX_FANOUT);
    
    recipients.into_iter().map(|(n, _)| n).collect()
}
```

## Version Vectors

ELARA uses version vectors for causal ordering:

```rust
struct VersionVector {
    entries: HashMap<NodeId, u64>,
}

impl VersionVector {
    fn increment(&mut self, node: NodeId) {
        *self.entries.entry(node).or_insert(0) += 1;
    }
    
    fn happens_before(&self, other: &Self) -> bool {
        // self < other iff all entries in self <= other
        // and at least one is strictly <
        let mut dominated = false;
        
        for (node, &count) in &self.entries {
            let other_count = other.entries.get(node).copied().unwrap_or(0);
            if count > other_count {
                return false;
            }
            if count < other_count {
                dominated = true;
            }
        }
        
        // Check entries in other not in self
        for (node, &count) in &other.entries {
            if !self.entries.contains_key(node) && count > 0 {
                dominated = true;
            }
        }
        
        dominated
    }
    
    fn merge(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (node, &count) in &other.entries {
            let entry = result.entries.entry(*node).or_insert(0);
            *entry = (*entry).max(count);
        }
        result
    }
    
    fn concurrent_with(&self, other: &Self) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}
```

## Partition Handling

### During Partition

Each subgraph operates independently:

```
Normal:  A ←→ B ←→ C ←→ D

Partition:
         A ←→ B       C ←→ D
         (subgraph 1) (subgraph 2)
         
Both subgraphs continue operating with local state.
```

### Merge Protocol

When partition heals:

```rust
fn merge_partitions(local: &StateField, remote: &StateField) -> MergeResult {
    // 1. Exchange state summaries
    let local_summary = local.summarize();
    let remote_summary = remote.summarize();
    
    // 2. Detect divergence points
    let divergent_states = find_divergent(&local_summary, &remote_summary);
    
    // 3. Exchange and replay missing deltas
    for state_id in divergent_states {
        let local_version = local.get_version(&state_id);
        let remote_version = remote.get_version(&state_id);
        
        if local_version.concurrent_with(&remote_version) {
            // True conflict - merge according to delta_law
            let merged = merge_concurrent(
                local.get(&state_id),
                remote.get(&state_id)
            );
            local.set(&state_id, merged);
        } else if remote_version.happens_before(&local_version) {
            // Remote is behind - send our updates
            send_updates(remote, &state_id, &remote_version);
        } else {
            // We're behind - apply remote updates
            apply_updates(local, &state_id, &local_version);
        }
    }
    
    // 4. Normalize time references
    synchronize_time_models();
    
    MergeResult::Success
}
```

## Byzantine-Light Containment

ELARA doesn't aim for full Byzantine fault tolerance, but provides containment:

### Authority Bounding

```rust
// Malicious node can only affect states it has authority over
fn bound_authority(node: NodeId, event: &Event) -> bool {
    // Check authority is properly scoped
    let scope = get_authority_scope(node);
    scope.contains(&event.target_state)
}
```

### Influence Bounding

```rust
// Limit rate of changes from any single node
struct InfluenceLimiter {
    windows: HashMap<NodeId, RateWindow>,
    max_events_per_second: u32,
}

impl InfluenceLimiter {
    fn allow(&mut self, node: NodeId) -> bool {
        let window = self.windows.entry(node).or_default();
        window.check_and_increment(self.max_events_per_second)
    }
}
```

### Anomaly Detection

```rust
fn detect_anomaly(event: &Event, history: &EventHistory) -> Option<Anomaly> {
    // Rapid-fire events
    if history.rate(event.source) > RATE_THRESHOLD {
        return Some(Anomaly::RateLimitExceeded);
    }
    
    // Version vector manipulation
    if event.version_ref.is_suspicious() {
        return Some(Anomaly::SuspiciousVersion);
    }
    
    // Entropy spike
    if event.entropy_hint.entropy > ENTROPY_THRESHOLD {
        return Some(Anomaly::HighEntropy);
    }
    
    None
}
```

### Containment Response

```rust
fn contain_malicious(node: NodeId, anomaly: Anomaly) {
    match anomaly.severity() {
        Severity::Low => {
            // Rate limit
            rate_limiter.throttle(node);
        }
        Severity::Medium => {
            // Mute temporarily
            mute_node(node, Duration::from_secs(60));
        }
        Severity::High => {
            // Isolate completely
            isolate_node(node);
            notify_peers(node, IsolationReason::Byzantine);
        }
    }
}
```

## Quarantine Buffer

Events that can't be immediately processed go to quarantine:

```rust
struct QuarantineBuffer {
    events: Vec<QuarantinedEvent>,
    max_size: usize,
    max_age: Duration,
}

struct QuarantinedEvent {
    event: Event,
    reason: QuarantineReason,
    received_at: Instant,
    retry_count: u32,
}

enum QuarantineReason {
    MissingDependency(Vec<EventId>),
    TooFuture,
    PendingAuthority,
}

impl QuarantineBuffer {
    fn try_release(&mut self, state_field: &StateField) -> Vec<Event> {
        let mut released = Vec::new();
        
        self.events.retain(|qe| {
            match &qe.reason {
                QuarantineReason::MissingDependency(deps) => {
                    if deps.iter().all(|d| state_field.has_event(d)) {
                        released.push(qe.event.clone());
                        false  // Remove from quarantine
                    } else {
                        true   // Keep in quarantine
                    }
                }
                // ... other reasons
            }
        });
        
        released
    }
}
```

## Key Insight

> Traditional systems: "Conflict → Error → Manual resolution"
> 
> ELARA: "Conflict → Automatic merge → Equivalent reality"

The goal is not identical bits across all nodes, but **equivalent reality** - all nodes agree on what matters, even if internal representations differ.
