//! Basic ELARA Node Example
//!
//! This example demonstrates how to create a basic ELARA node,
//! generate identity, and process events.

use elara_core::{
    Event, EventId, EventType, MutationOp, NodeId, PacketClass,
    RepresentationProfile, SessionId, StateId, StateTime, TimeIntent,
    VersionVector,
};
use elara_crypto::{Identity, SecureFrameProcessor};
use elara_wire::{Extensions, FixedHeader, Frame, FrameBuilder};

fn main() {
    println!("=== ELARA Basic Node Example ===\n");

    // 1. Generate node identity
    println!("1. Generating node identity...");
    let identity = Identity::generate();
    let node_id = identity.node_id();
    println!("   Node ID: {:016x}", node_id.0);

    // 2. Create a session
    println!("\n2. Creating session...");
    let session_id = SessionId::new(0x1234_5678_9ABC_DEF0);
    let session_key = [0x42u8; 32]; // In real use, derive from key exchange
    println!("   Session ID: {:016x}", session_id.0);

    // 3. Initialize secure frame processor
    println!("\n3. Initializing secure frame processor...");
    let mut processor = SecureFrameProcessor::new(session_id, node_id, session_key);
    println!("   Processor ready");

    // 4. Create and encrypt a frame
    println!("\n4. Creating and encrypting a frame...");
    let payload = b"Hello, ELARA!";
    let encrypted = processor
        .encrypt_frame(
            PacketClass::Core,
            RepresentationProfile::Textual,
            1000, // time hint
            Extensions::new(),
            payload,
        )
        .expect("Encryption failed");
    println!("   Original payload: {} bytes", payload.len());
    println!("   Encrypted frame: {} bytes", encrypted.len());

    // 5. Parse the frame header
    println!("\n5. Parsing frame header...");
    let frame = Frame::parse(&encrypted).expect("Parse failed");
    println!("   Version: {}", frame.header.version);
    println!("   Class: {:?}", frame.header.class);
    println!("   Profile: {:?}", frame.header.profile);
    println!("   Sequence: {}", frame.header.seq());
    println!("   Time hint: {}ms", frame.header.time_hint);

    // 6. Create an event
    println!("\n6. Creating an event...");
    let event = create_sample_event(node_id);
    println!("   Event ID: {:?}", event.id);
    println!("   Target state: {:?}", event.target_state);
    println!("   Event type: {:?}", event.event_type);

    // 7. Version vector demonstration
    println!("\n7. Version vector operations...");
    let mut vv1 = VersionVector::new();
    vv1.increment(NodeId::new(1));
    vv1.increment(NodeId::new(1));
    vv1.increment(NodeId::new(2));

    let mut vv2 = VersionVector::new();
    vv2.increment(NodeId::new(2));
    vv2.increment(NodeId::new(3));

    println!("   VV1: node1=2, node2=1");
    println!("   VV2: node2=1, node3=1");
    println!("   VV1 happens-before VV2: {}", vv1.happens_before(&vv2));
    println!("   Concurrent: {}", vv1.concurrent_with(&vv2));

    let merged = vv1.merge(&vv2);
    println!("   Merged: node1={}, node2={}, node3={}",
        merged.get(NodeId::new(1)),
        merged.get(NodeId::new(2)),
        merged.get(NodeId::new(3))
    );

    println!("\n=== Example Complete ===");
}

fn create_sample_event(source: NodeId) -> Event {
    Event {
        id: EventId { source, seq: 1 },
        event_type: EventType::StateUpdate,
        source,
        target_state: StateId {
            state_type: 0x01, // Text
            instance: 0x1234,
        },
        version_ref: VersionVector::new(),
        mutation: MutationOp::Append(b"Hello, World!".to_vec()),
        time_intent: TimeIntent {
            timestamp: StateTime::from_millis(1000),
            tolerance: std::time::Duration::from_millis(100),
        },
        authority_proof: elara_core::AuthorityProof::new([0u8; 64]),
        entropy_hint: elara_core::EntropyHint::default(),
    }
}
