//! Time Engine Simulation Example
//!
//! This example demonstrates the ELARA Time Engine with clock drift
//! simulation and horizon adaptation.

use elara_core::NodeId;
use elara_time::{NetworkModel, PerceptualClock, StateClock, TimeEngineConfig};
use std::time::Duration;

fn main() {
    println!("=== ELARA Time Engine Simulation ===\n");

    // 1. Demonstrate dual clock system
    println!("1. Dual Clock System");
    println!("   τp (Perceptual): Monotonic, smooth, local-driven");
    println!("   τs (State): Elastic, correctable, convergent\n");

    let perceptual = PerceptualClock::new();
    let mut state = StateClock::new();

    println!("   Initial τp: {:?}", perceptual.now());
    println!("   Initial τs: {:?}", state.now());

    // 2. Simulate clock correction
    println!("\n2. Clock Correction Simulation");
    println!("   Applying correction of +50ms with weight 0.3...");
    
    state.apply_correction(0.050, 0.3); // 50ms correction, 30% weight
    
    println!("   τs after correction: {:?}", state.now());
    println!("   Note: Correction is blended, not jumped");

    // 3. Network model demonstration
    println!("\n3. Network Model Learning");
    let mut network = NetworkModel::new();
    
    // Simulate packets from peer
    let peer = NodeId::new(0x1234);
    println!("   Simulating packets from peer {:016x}...", peer.0);
    
    for i in 0..20 {
        let local_time = i as f64 * 0.1; // 100ms intervals
        let remote_time = local_time - 0.05 + (i as f64 * 0.001); // ~50ms offset with slight drift
        network.update_from_packet(peer, local_time, remote_time, i as u16);
    }
    
    if let Some(peer_model) = network.get_peer(peer) {
        println!("   Learned peer model:");
        println!("     Offset: {:.3}ms", peer_model.offset * 1000.0);
        println!("     Jitter envelope: {:.3}ms", peer_model.jitter_envelope * 1000.0);
        println!("     Samples: {}", peer_model.sample_count);
    }

    // 4. Horizon adaptation
    println!("\n4. Horizon Adaptation");
    let config = TimeEngineConfig::default();
    
    println!("   Good network conditions:");
    let (hp_good, hc_good) = simulate_horizons(&config, 0.01, 0.01);
    println!("     Hp: {:?}", hp_good);
    println!("     Hc: {:?}", hc_good);
    
    println!("\n   Bad network conditions (high jitter, loss):");
    let (hp_bad, hc_bad) = simulate_horizons(&config, 0.1, 0.15);
    println!("     Hp: {:?}", hp_bad);
    println!("     Hc: {:?}", hc_bad);
    
    println!("\n   Note: Bad network → expanded horizons → more prediction/correction room");

    // 5. Reality window demonstration
    println!("\n5. Reality Window");
    println!("   RW = [τs - Hc, τs + Hp]");
    println!("   ");
    println!("   Past ←─────────────────────────────────────→ Future");
    println!("        │           │           │           │");
    println!("        τs-Hc       τs          τs+Hp       ");
    println!("        │           │           │           ");
    println!("        └── Correction ──┴── Prediction ──┘");
    println!("             Zone              Zone");

    // 6. Time position classification
    println!("\n6. Event Time Classification");
    let positions = [
        ("Event at τs - 2*Hc", "TooOld → Archive"),
        ("Event at τs - 0.5*Hc", "Correctable → Apply with blend"),
        ("Event at τs", "Current → Apply now"),
        ("Event at τs + 0.5*Hp", "Predicted → Apply now"),
        ("Event at τs + 2*Hp", "TooFuture → Quarantine"),
    ];
    
    for (event, result) in positions {
        println!("   {} → {}", event, result);
    }

    // 7. Non-destructive correction principle
    println!("\n7. Non-Destructive Correction Law");
    println!("   NEVER: Hard rewind, full reset, freeze timeline");
    println!("   ALWAYS: Curve deformation, parameter interpolation, envelope reshaping");
    println!("   ");
    println!("   Traditional: value = corrected_value  // JARRING!");
    println!("   ELARA:       value = value * 0.7 + corrected_value * 0.3  // SMOOTH");

    println!("\n=== Simulation Complete ===");
}

fn simulate_horizons(
    config: &TimeEngineConfig,
    jitter: f64,
    loss_rate: f64,
) -> (Duration, Duration) {
    let jitter_factor = 1.0 + jitter * 10.0;
    let loss_factor = 1.0 + loss_rate * 5.0;
    let instability = jitter_factor * loss_factor;
    
    let expansion = (instability - 1.0).clamp(0.0, 1.0);
    
    let hp = config.Hp_min + Duration::from_secs_f64(
        (config.Hp_max - config.Hp_min).as_secs_f64() * expansion
    );
    
    let hc = config.Hc_min + Duration::from_secs_f64(
        (config.Hc_max - config.Hc_min).as_secs_f64() * expansion
    );
    
    (hp, hc)
}
