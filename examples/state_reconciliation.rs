//! State Reconciliation Example
//!
//! This example demonstrates ELARA's state reconciliation with
//! version vectors, authority checking, and convergence.

use elara_core::{
    Event, EventId, EventType, MutationOp, NodeId, StateId, StateTime,
    TimeIntent, VersionVector,
};
use elara_state::{ReconciliationEngine, StateField};
use std::collections::HashSet;

fn main() {
    println!("=== ELARA State Reconciliation Example ===\n");

    // 1. Create nodes
    println!("1. Creating nodes...");
    let node_a = NodeId::new(0xAAAA);
    let node_b = NodeId::new(0xBBBB);
    let node_c = NodeId::new(0xCCCC);
    println!("   Node A: {:016x}", node_a.0);
    println!("   Node B: {:016x}", node_b.0);
    println!("   Node C: {:016x}", node_c.0);

    // 2. Version vector demonstration
    println!("\n2. Version Vector Operations");
    
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();
    
    // Node A makes changes
    vv_a.increment(node_a);
    vv_a.increment(node_a);
    println!("   After A makes 2 changes: A={}", vv_a.get(node_a));
    
    // Node B makes changes
    vv_b.increment(node_b);
    println!("   After B makes 1 change: B={}", vv_b.get(node_b));
    
    // Check causality
    println!("\n   Causality check:");
    println!("   VV_A happens-before VV_B: {}", vv_a.happens_before(&vv_b));
    println!("   VV_B happens-before VV_A: {}", vv_b.happens_before(&vv_a));
    println!("   Concurrent: {}", vv_a.concurrent_with(&vv_b));
    
    // Merge
    let merged = vv_a.merge(&vv_b);
    println!("\n   After merge:");
    println!("   Merged[A]={}, Merged[B]={}", merged.get(node_a), merged.get(node_b));

    // 3. Reconciliation pipeline stages
    println!("\n3. Reconciliation Pipeline (6 Stages)");
    println!("   ┌─────────────────────────────────────┐");
    println!("   │ Stage 1: Authority Check            │");
    println!("   │   Is source authorized for state?   │");
    println!("   ├─────────────────────────────────────┤");
    println!("   │ Stage 2: Causality Check            │");
    println!("   │   Does version_ref match/precede?   │");
    println!("   ├─────────────────────────────────────┤");
    println!("   │ Stage 3: Temporal Placement         │");
    println!("   │   Map to local τs, classify time    │");
    println!("   ├─────────────────────────────────────┤");
    println!("   │ Stage 4: Delta Merge                │");
    println!("   │   Apply mutation per delta_law      │");
    println!("   ├─────────────────────────────────────┤");
    println!("   │ Stage 5: Divergence Control         │");
    println!("   │   Check entropy, simplify if needed │");
    println!("   ├─────────────────────────────────────┤");
    println!("   │ Stage 6: Swarm Diffusion            │");
    println!("   │   Determine who needs this update   │");
    println!("   └─────────────────────────────────────┘");

    // 4. Authority demonstration
    println!("\n4. Authority System");
    let mut authority_set: HashSet<NodeId> = HashSet::new();
    authority_set.insert(node_a);
    authority_set.insert(node_b);
    
    println!("   State authority set: [A, B]");
    println!("   Node A authorized: {}", authority_set.contains(&node_a));
    println!("   Node B authorized: {}", authority_set.contains(&node_b));
    println!("   Node C authorized: {}", authority_set.contains(&node_c));
    println!("   → Node C's events would be REJECTED");

    // 5. Concurrent edit resolution
    println!("\n5. Concurrent Edit Resolution");
    println!("   Scenario: A and B edit same state concurrently");
    println!("   ");
    println!("   Timeline:");
    println!("   A: ──●────────────●──────────────────→");
    println!("        edit1       receives B's edit");
    println!("   ");
    println!("   B: ────────●────────────●────────────→");
    println!("              edit1       receives A's edit");
    println!("   ");
    println!("   Resolution: Merge according to delta_law");
    println!("   - LastWriteWins: Higher timestamp wins");
    println!("   - AppendOnly: Both appended (causal order)");
    println!("   - SetCRDT: Union of both sets");

    // 6. Partition and merge
    println!("\n6. Partition Handling");
    println!("   ");
    println!("   Normal:    A ←→ B ←→ C");
    println!("   ");
    println!("   Partition: A ←→ B     C (isolated)");
    println!("              (group 1)  (group 2)");
    println!("   ");
    println!("   Both groups continue operating independently!");
    println!("   ");
    println!("   Merge protocol when partition heals:");
    println!("   1. Exchange state summaries");
    println!("   2. Detect divergence points");
    println!("   3. Exchange missing deltas");
    println!("   4. Merge concurrent changes");
    println!("   5. Normalize time references");
    println!("   ");
    println!("   Result: All nodes converge to equivalent reality");

    // 7. Divergence control
    println!("\n7. Divergence Control");
    println!("   When entropy exceeds threshold:");
    println!("   ");
    println!("   Cosmetic state    → Drop entirely");
    println!("   Enhancement state → Reduce quality");
    println!("   Perceptual state  → Increase tolerance");
    println!("   Core state        → Flag for resolution");
    println!("   ");
    println!("   Goal: Simplify, don't disconnect");

    // 8. Key insight
    println!("\n8. Key Insight");
    println!("   ┌─────────────────────────────────────────────────┐");
    println!("   │ Traditional: Conflict → Error → Manual fix      │");
    println!("   │ ELARA:       Conflict → Auto merge → Equivalent │");
    println!("   │                                       reality   │");
    println!("   └─────────────────────────────────────────────────┘");
    println!("   ");
    println!("   The goal is not identical bits across all nodes,");
    println!("   but EQUIVALENT REALITY - all nodes agree on what");
    println!("   matters, even if internal representations differ.");

    println!("\n=== Example Complete ===");
}
