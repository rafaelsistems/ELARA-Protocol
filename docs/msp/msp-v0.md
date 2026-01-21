# ELARA MSP v0 Specification

**MSP** = **M**inimum **S**urvivable **P**roduct

The first living organism of ELARA - a baseline for protocol validation.

## Philosophy

> "Build the smallest thing that proves the laws work."

MSP v0 is not a feature-complete product. It's a **protocol validation vehicle**.

## Scope

### Included

| Feature | Description |
|---------|-------------|
| Text | Real-time chat + async messages |
| Voice | Real-time speech state (minimal) |
| 1-1 | Direct communication |
| Small Group | Up to 8 participants |
| NAT Hostile | Works behind restrictive NATs |
| Full Crypto | Complete cryptographic physics |

### Explicitly Excluded

| Feature | Reason |
|---------|--------|
| Video | Complexity, bandwidth |
| Neural Codec | Research dependency |
| AI Rendering | Future enhancement |
| Large Groups | Swarm complexity |
| Federation | Protocol maturity |

## Representation Profiles

### Profile: Textual (0x01)

For chat, system messages, presence, and typing.

#### State Atoms

```rust
// Message stream
ω:text:{stream_id} = {
    type: Core,
    authority: stream_participants,
    delta_law: AppendOnly {
        causal_ordered: true,
        idempotent: true,
        mergeable: true
    }
}

// User presence
ω:presence:{user_id} = {
    type: Perceptual,
    authority: user_only,
    delta_law: LastWriteWins
}

// Typing indicator
ω:typing:{user_id} = {
    type: Cosmetic,
    authority: user_only,
    delta_law: Ephemeral { ttl: 5_000ms }
}
```

#### Message Format

```rust
struct TextMessage {
    id: MessageId,
    author: NodeId,
    timestamp: StateTime,
    content: String,
    reply_to: Option<MessageId>,
    edit_of: Option<MessageId>,
}

struct TextDelta {
    Append { message: TextMessage },
    Edit { message_id: MessageId, new_content: String },
    React { message_id: MessageId, reaction: String },
    Delete { message_id: MessageId },  // Soft delete
}
```

### Profile: VoiceMinimal (0x02)

For voice calls and voice rooms.

#### State Atoms

```rust
// Voice state per user
ω:voice:{user_id} = {
    type: Perceptual,
    authority: user_only,
    delta_law: FrameBased { interval: 20ms }
}
```

#### Voice Frame Format

**NOT audio PCM!** Voice is encoded as **speech state**:

```rust
struct VoiceFrame {
    // Timing
    frame_seq: u16,
    timestamp_offset: i16,  // ms offset from expected
    
    // Speech parameters
    voiced: bool,           // Voiced or unvoiced segment
    pitch: u8,              // F0 index (0-255 → 50-500Hz)
    energy: u8,             // dB level (0-255)
    spectral_env: [u8; 10], // LPC coefficients or similar
    residual_seed: u16,     // For excitation regeneration
}
```

#### Why Not Audio?

| Approach | Bandwidth | Latency | Degradation |
|----------|-----------|---------|-------------|
| Raw PCM | 128 kbps | Low | Cliff |
| Opus | 6-32 kbps | Low | Cliff |
| **Speech State** | 2-4 kbps | Low | **Graceful** |

Speech state allows:
- Extreme bandwidth efficiency
- Graceful degradation (parameters → symbolic → presence)
- Reconstruction at receiver with local synthesis
- Network-independent quality

## Time Engine Configuration

```rust
const MSP_TIME_CONFIG: TimeEngineConfig = TimeEngineConfig {
    // Prediction horizon (how far ahead we predict)
    Hp_min: Duration::from_millis(40),
    Hp_max: Duration::from_millis(300),
    
    // Correction horizon (how far back we can fix)
    Hc_min: Duration::from_millis(80),
    Hc_max: Duration::from_millis(600),
    
    // Tick intervals
    drift_tick: Duration::from_millis(100),
    prediction_tick: Duration::from_millis(16),
    correction_tick: Duration::from_millis(10),
    
    // Thresholds
    jitter_expansion_threshold: 0.02,  // 20ms
    loss_expansion_threshold: 0.05,    // 5%
};
```

## Graceful Degradation

MSP v0 implements the full degradation path:

```
┌─────────────────────────────────────────────────────────┐
│                    FULL QUALITY                          │
│  Voice: All parameters + enhancement                     │
│  Text: Full formatting + reactions                       │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼ (bandwidth pressure)
┌─────────────────────────────────────────────────────────┐
│                  REDUCED QUALITY                         │
│  Voice: Core parameters only                             │
│  Text: Plain text only                                   │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼ (more pressure)
┌─────────────────────────────────────────────────────────┐
│                    SYMBOLIC                              │
│  Voice: Speaking/silent indicator                        │
│  Text: Message count only                                │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼ (severe pressure)
┌─────────────────────────────────────────────────────────┐
│                   PRESENCE ONLY                          │
│  Voice: Online/offline                                   │
│  Text: Online/offline                                    │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼ (critical)
┌─────────────────────────────────────────────────────────┐
│                 IDENTITY HEARTBEAT                       │
│  Minimal keepalive proving session alive                 │
└─────────────────────────────────────────────────────────┘

         ⚠️ SESSION NEVER DROPS ⚠️
         Reality simplifies, connection persists.
```

## Target Hardware

MSP v0 must run on constrained devices:

| Constraint | Limit |
|------------|-------|
| CPU | ≤2 cores |
| RAM | ≤2 GB |
| GPU | None required |
| Network | 2G-class (50-200 kbps) |

### Target Devices

- Android Go phones
- Old iPhones (iPhone 6s+)
- Linux headless servers
- Raspberry Pi 3+
- Low-end Chromebooks

## Network Requirements

| Metric | Minimum | Recommended |
|--------|---------|-------------|
| Bandwidth | 50 kbps | 200 kbps |
| Latency | <500ms | <100ms |
| Packet Loss | <20% | <5% |
| Jitter | <200ms | <50ms |

## Validation Tests

### 1. Jitter Chaos Test

```rust
fn jitter_chaos_test() {
    let config = ChaosConfig {
        jitter_range: 0..500,  // 0-500ms random jitter
        jitter_distribution: Distribution::Exponential,
        duration: Duration::from_secs(300),
    };
    
    // Voice call must remain intelligible
    // Text must maintain causal ordering
    // No session drops
}
```

### 2. Packet Loss Torture Test

```rust
fn packet_loss_torture_test() {
    let scenarios = vec![
        LossPattern::Random { rate: 0.10 },      // 10% random
        LossPattern::Burst { length: 10, gap: 50 }, // Burst loss
        LossPattern::Asymmetric { up: 0.05, down: 0.15 },
    ];
    
    // Graceful degradation must engage
    // Recovery must be smooth
    // No data corruption
}
```

### 3. NAT Swarm Test

```rust
fn nat_swarm_test() {
    let nat_types = vec![
        NatType::FullCone,
        NatType::RestrictedCone,
        NatType::PortRestricted,
        NatType::Symmetric,
    ];
    
    // All combinations must establish connection
    // Relay fallback must work
    // Hole punching success rate > 80%
}
```

### 4. Convergence Test

```rust
fn convergence_test() {
    let scenario = PartitionScenario {
        nodes: 8,
        partition_duration: Duration::from_secs(60),
        concurrent_edits: 100,
    };
    
    // All nodes must converge after partition heals
    // No data loss
    // Causal ordering preserved
}
```

### 5. Resource Exhaustion Test

```rust
fn resource_exhaustion_test() {
    let constraints = ResourceConstraints {
        max_memory: 50 * 1024 * 1024,  // 50 MB
        max_cpu: 0.5,                   // 50% of one core
        max_bandwidth: 50_000,          // 50 kbps
    };
    
    // Must degrade gracefully
    // Must not crash
    // Must maintain session
}
```

## Deliverables

### Core Components

| Component | Status | Description |
|-----------|--------|-------------|
| elara-core | ✅ | Core types and primitives |
| elara-wire | ✅ | Wire protocol implementation |
| elara-crypto | ✅ | Cryptographic binding |
| elara-time | ✅ | Time engine |
| elara-state | ✅ | State reconciliation |
| elara-transport | ✅ | Network transport |
| elara-runtime | ✅ | Node runtime |
| elara-msp | ✅ | MSP profiles |
| elara-test | ✅ | Test harness |

### Validation Artifacts

| Artifact | Description |
|----------|-------------|
| Time Simulator | Clock drift and network simulation |
| State Fuzzer | Property-based state testing |
| Chaos Harness | Network chaos injection |
| Benchmark Suite | Performance validation |

### Documentation

| Document | Description |
|----------|-------------|
| Core Concepts | Fundamental primitives |
| Four Pillars | Architecture overview |
| Wire Protocol | Binary format spec |
| Crypto Binding | Security specification |
| Time Engine | Temporal mechanics |
| State Reconciliation | Convergence spec |
| MSP v0 | This document |

## Success Criteria

MSP v0 is successful when:

1. **Text works**: Messages delivered in causal order under chaos
2. **Voice works**: Intelligible speech under 10% loss, 100ms jitter
3. **Degradation works**: Smooth transition through all levels
4. **Convergence works**: All nodes agree after partition
5. **Crypto works**: No plaintext leakage, replay protection
6. **Resources bounded**: Runs on target hardware

## Non-Goals for v0

- Perfect voice quality (intelligible is enough)
- Beautiful UI (protocol validation only)
- Production deployment (research prototype)
- Backward compatibility (will break)
- Documentation completeness (evolving)

## Next Steps After v0

1. **v0.1**: Stability fixes, test coverage
2. **v0.2**: Performance optimization
3. **v0.3**: Voice quality improvements
4. **v1.0**: First stable release
5. **v1.x**: Video, larger groups, federation
