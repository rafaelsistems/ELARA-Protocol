# elara-time

Time convergence engine for the ELARA Protocol - featuring dual clock system, reality windows, and temporal prediction algorithms.

## Features

- **Dual Clock System**: Perceptual time (smooth UX) + State time (network consensus)
- **Reality Windows**: Temporal boundaries for event processing
- **Horizon Adaptation**: Dynamic adjustment based on network conditions
- **Non-Destructive Correction**: Time bending without timeline breaks
- **Peer Time Modeling**: Distributed time synchronization

## Quick Start

```rust
use elara_time::{TimeEngine, PerceptualClock, StateClock};

// Create time engine
let mut engine = TimeEngine::new(node_id);

// Get current times
let perceptual = engine.perceptual_time();
let state = engine.state_time();

// Process network time update
engine.update_peer_time(peer_id, network_time, rtt_estimate)?;

// Adapt horizons based on network quality
engine.adapt_horizons(network_quality);
```

## Dual Clock System

### Perceptual Clock (τp)
- **Purpose**: User experience smoothness
- **Properties**: Monotonic, local, smooth interpolation
- **Use Case**: Media playback, UI updates

### State Clock (τs)
- **Purpose**: Network consensus
- **Properties**: Correctable, distributed, eventual consistency
- **Use Case**: Event ordering, state reconciliation

## Reality Window

```
Past ←──────────────────────────────────────────→ Future
      │                    │                    │
      τs - Hc              τs                   τs + Hp
      │                    │                    │
      └── Correction ──────┴──── Prediction ────┘
          Horizon                 Horizon
```

- **Correction Horizon (Hc)**: How far back we can correct
- **Prediction Horizon (Hp)**: How far forward we can predict
- **Dynamic Adjustment**: Based on network jitter and stability

## Network Time Model

```rust
pub struct PeerTimeModel {
    pub offset: Duration,        // Clock offset from peer
    pub round_trip_time: Duration,
    pub confidence: f64,         // 0.0 to 1.0
    pub stability_score: f64,    // Network stability metric
}
```

## Horizon Adaptation

The engine automatically adjusts horizons based on network conditions:

```
Good Network → Tight Horizons → Sharp Reality
Bad Network  → Wide Horizons  → Flexible Reality
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.