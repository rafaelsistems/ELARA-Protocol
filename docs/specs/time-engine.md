# ELARA Time Engine Specification v0

Time in ELARA is not a passive timestamp. It is a **first-class protocol object** that actively participates in reality synchronization.

## Design Philosophy

> "Network affects the SHAPE of reality, not its CONTINUITY."

Traditional protocols freeze or reset when network conditions degrade. ELARA bends time instead.

## Dual Clock System

ELARA maintains two distinct clocks that serve different purposes:

### Perceptual Time (τp)

The clock that drives user experience.

| Property | Value |
|----------|-------|
| Monotonic | Always increases |
| Smooth | No jumps or stutters |
| Local-driven | Based on local system clock |
| Frame-aligned | Synced to rendering/audio frames |

```rust
struct PerceptualClock {
    base: Instant,
    rate: f64,  // Always 1.0 for τp
}

impl PerceptualClock {
    fn now(&self) -> PerceptualTime {
        // NEVER jumps, NEVER goes backward
        PerceptualTime(self.base.elapsed())
    }
}
```

**τp guarantees:**
- User sees smooth, continuous experience
- Audio/video never stutters due to network
- Local interactions feel instant

### State Time (τs)

The clock that drives network consensus.

| Property | Value |
|----------|-------|
| Elastic | Can stretch or compress |
| Correctable | Can be adjusted based on peer data |
| Convergence-oriented | Aims to match network consensus |
| Drift-tolerant | Handles clock skew between nodes |

```rust
struct StateClock {
    base: Instant,
    offset: f64,      // Correction offset
    rate: f64,        // Can vary (0.9 - 1.1 typical)
    drift_estimate: f64,
}

impl StateClock {
    fn now(&self) -> StateTime {
        let raw = self.base.elapsed().as_secs_f64();
        let adjusted = raw * self.rate + self.offset;
        StateTime::from_secs_f64(adjusted)
    }
    
    fn apply_correction(&mut self, correction: f64, weight: f64) {
        // Blend correction, don't jump
        self.offset += correction * weight;
    }
}
```

**τs guarantees:**
- All nodes eventually agree on event ordering
- Corrections are smooth, not jarring
- Network chaos doesn't break causality

## Reality Window

The Reality Window defines what events are "live" vs "historical":

```
Past ←────────────────────────────────────────────→ Future
     │                      │                      │
     τs - Hc                τs                     τs + Hp
     │                      │                      │
     └──── Correction ──────┴───── Prediction ─────┘
           Horizon                  Horizon
```

### Horizons

| Horizon | Symbol | Purpose | Typical Range |
|---------|--------|---------|---------------|
| Correction | Hc | How far back we can fix | 80-600ms |
| Prediction | Hp | How far ahead we predict | 40-300ms |

### Event Classification

```rust
fn classify_event(event_time: StateTime, τs: StateTime, Hc: Duration, Hp: Duration) -> TimePosition {
    let delta = event_time.0 - τs.0;
    
    if delta < -Hc {
        TimePosition::TooOld  // Reject or archive
    } else if delta < Duration::ZERO {
        TimePosition::Correctable  // Can still fix
    } else if delta < Hp {
        TimePosition::Predicted  // In prediction zone
    } else {
        TimePosition::TooFuture  // Quarantine
    }
}
```

### Horizon Adaptation

Horizons adapt to network conditions:

```rust
fn adapt_horizons(
    config: &HorizonConfig,
    network: &NetworkModel
) -> (Duration, Duration) {
    let jitter_factor = 1.0 + network.jitter * 10.0;
    let loss_factor = 1.0 + network.loss_rate * 5.0;
    let instability = jitter_factor * loss_factor;
    
    // Bad network → expand horizons
    let Hp = config.Hp_min + (config.Hp_max - config.Hp_min) 
        * (instability - 1.0).clamp(0.0, 1.0);
    
    let Hc = config.Hc_min + (config.Hc_max - config.Hc_min)
        * (instability - 1.0).clamp(0.0, 1.0);
    
    (Hp, Hc)
}
```

## Network Model

The Time Engine passively learns network characteristics from traffic:

```rust
struct NetworkModel {
    peers: HashMap<NodeId, PeerNetworkModel>,
    latency_mean: f64,
    jitter: f64,
    reorder_depth: u32,
    loss_rate: f64,
    stability_score: f64,
}

struct PeerNetworkModel {
    offset: f64,           // Clock offset estimate
    skew: f64,             // Clock rate difference
    jitter_envelope: f64,  // Jitter range
    sample_count: u32,
}
```

### Passive Learning

```rust
fn update_from_packet(
    &mut self,
    peer: NodeId,
    local_time: f64,
    remote_time: f64
) {
    let peer_model = self.peers.entry(peer).or_default();
    
    // One-way delay estimate
    let delay = local_time - remote_time;
    
    // Exponential moving average
    let alpha = 0.1;
    peer_model.offset = peer_model.offset * (1.0 - alpha) + delay * alpha;
    
    // Jitter estimation
    let jitter_sample = (delay - peer_model.offset).abs();
    peer_model.jitter_envelope = peer_model.jitter_envelope * 0.9 + jitter_sample * 0.1;
    
    peer_model.sample_count += 1;
}
```

## Four Internal Loops

The Time Engine runs four concurrent loops:

### 1. Drift Estimation Loop

Estimates clock drift relative to each peer:

```rust
fn drift_estimation_tick(&mut self) {
    for (peer_id, peer_model) in &mut self.network.peers {
        if peer_model.sample_count < MIN_SAMPLES {
            continue;
        }
        
        // Estimate skew from offset trend
        let skew = estimate_skew(peer_model);
        peer_model.skew = peer_model.skew * 0.95 + skew * 0.05;
    }
}
```

### 2. Prediction Loop

Predicts future state based on current trajectory:

```rust
fn prediction_tick(&mut self, state_field: &mut StateField) {
    let prediction_horizon = self.Hp;
    
    for atom in state_field.atoms_mut() {
        if atom.state_type == StateType::Perceptual {
            // Extrapolate continuous state
            let predicted = atom.extrapolate(prediction_horizon);
            atom.set_predicted(predicted);
        }
    }
}
```

**Prediction rules:**
- Bounded: Never predict beyond Hp
- Reversible: Can be corrected without artifacts
- Non-authoritative: Predictions don't become truth

### 3. Correction Loop

Applies corrections from authoritative events:

```rust
fn correction_tick(&mut self, events: &[Event]) {
    for event in events {
        let position = self.classify_time(event.time_intent.timestamp);
        
        match position {
            TimePosition::Correctable => {
                // Blend correction into current state
                let weight = self.correction_weight(event);
                self.apply_correction(event, weight);
            }
            TimePosition::TooOld => {
                // Archive for history, don't apply
                self.archive_event(event);
            }
            _ => {}
        }
    }
}

fn correction_weight(&self, event: &Event) -> f64 {
    // Older corrections get less weight
    let age = (self.τs() - event.time_intent.timestamp).as_secs_f64();
    let Hc = self.Hc.as_secs_f64();
    
    // Linear decay from 1.0 to 0.0 over Hc
    (1.0 - age / Hc).clamp(0.0, 1.0)
}
```

### 4. Compression Loop

Reduces detail under resource pressure:

```rust
fn compression_tick(&mut self, state_field: &mut StateField, pressure: f64) {
    if pressure < COMPRESSION_THRESHOLD {
        return;
    }
    
    for atom in state_field.atoms_mut() {
        match atom.state_type {
            StateType::Cosmetic => {
                // Drop cosmetic state first
                atom.reduce_detail(pressure);
            }
            StateType::Enhancement => {
                if pressure > 0.7 {
                    atom.reduce_detail(pressure - 0.5);
                }
            }
            StateType::Perceptual => {
                if pressure > 0.9 {
                    // Only reduce perceptual under extreme pressure
                    atom.reduce_detail(pressure - 0.8);
                }
            }
            StateType::Core => {
                // Never compress core state
            }
        }
    }
}
```

## Non-Destructive Correction Law

**NEVER:**
- Hard rewind (jump backward)
- Full reset (lose state)
- Freeze timeline (stop time)

**ALWAYS:**
- Curve deformation (smooth adjustment)
- Parameter interpolation (blend values)
- Envelope reshaping (adjust bounds)
- Predictive path bending (adjust trajectory)

### Example: Voice Correction

```rust
// Wrong: Hard correction
voice_state.pitch = corrected_pitch;  // JARRING!

// Right: Blended correction
let blend_factor = 0.3;
voice_state.pitch = voice_state.pitch * (1.0 - blend_factor) 
    + corrected_pitch * blend_factor;  // SMOOTH
```

## Time Engine Configuration

### MSP v0 Defaults

```rust
const MSP_TIME_CONFIG: TimeEngineConfig = TimeEngineConfig {
    // Horizons
    Hp_min: Duration::from_millis(40),
    Hp_max: Duration::from_millis(300),
    Hc_min: Duration::from_millis(80),
    Hc_max: Duration::from_millis(600),
    
    // Tick rates
    drift_tick_interval: Duration::from_millis(100),
    prediction_tick_interval: Duration::from_millis(16),  // ~60fps
    correction_tick_interval: Duration::from_millis(10),
    compression_tick_interval: Duration::from_millis(100),
    
    // Thresholds
    min_samples_for_drift: 10,
    compression_threshold: 0.6,
};
```

### Profile-Specific Tuning

| Profile | Hp Range | Hc Range | Notes |
|---------|----------|----------|-------|
| Textual | 100-500ms | 5-30s | Relaxed, edit window |
| VoiceMinimal | 40-100ms | 80-200ms | Tight, real-time |
| VideoPerceptual | 50-150ms | 100-500ms | Balanced |
| GroupSwarm | 60-200ms | 100-400ms | Multi-peer |

## Integration with State Engine

The Time Engine provides temporal context for state reconciliation:

```rust
impl TimeEngine {
    // Used by reconciliation pipeline
    fn map_to_local_time(&self, remote_time: StateTime, peer: NodeId) -> StateTime {
        let peer_model = self.network.get_peer(peer)?;
        StateTime(remote_time.0 + Duration::from_secs_f64(peer_model.offset))
    }
    
    // Used by event processing
    fn is_within_reality_window(&self, time: StateTime) -> bool {
        let delta = time.0.as_secs_f64() - self.τs().0.as_secs_f64();
        delta >= -self.Hc.as_secs_f64() && delta <= self.Hp.as_secs_f64()
    }
}
```

## Key Insight

> Traditional protocols: "Network is bad → freeze/reset"
> 
> ELARA: "Network is bad → bend time, simplify reality, keep flowing"

This is why ELARA can maintain continuous communication even under severe network chaos.
