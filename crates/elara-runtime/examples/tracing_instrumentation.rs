//! Example demonstrating distributed tracing instrumentation in ELARA nodes.
//!
//! This example shows how the tracing spans are automatically created during
//! node operations, providing end-to-end visibility into message flow and
//! state synchronization.
//!
//! # Running the Example
//!
//! First, start a Jaeger instance:
//! ```bash
//! docker run -d --name jaeger \
//!   -p 6831:6831/udp \
//!   -p 16686:16686 \
//!   jaegertracing/all-in-one:latest
//! ```
//!
//! Then run the example:
//! ```bash
//! cargo run --example tracing_instrumentation
//! ```
//!
//! View traces at: http://localhost:16686

use elara_core::{Event, EventType, MutationOp, SessionId, StateId};
use elara_runtime::node::{Node, NodeConfig};
use elara_runtime::observability::tracing::{init_tracing, TracingConfig, TracingExporter};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with Jaeger exporter
    let tracing_config = TracingConfig {
        service_name: "elara-tracing-demo".to_string(),
        exporter: TracingExporter::Jaeger {
            endpoint: "http://localhost:14268/api/traces".to_string(),
        },
        sampling_rate: 1.0, // Sample all traces for demo
        resource_attributes: vec![
            ("environment".to_string(), "demo".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ],
    };

    let _tracing_handle = init_tracing(tracing_config).await?;

    println!("Distributed tracing initialized!");
    println!("View traces at: http://localhost:16686");
    println!();

    // Create two nodes
    let mut node1 = Node::with_config(NodeConfig {
        tick_interval: Duration::from_millis(100),
        ..Default::default()
    });

    let mut node2 = Node::with_config(NodeConfig {
        tick_interval: Duration::from_millis(100),
        ..Default::default()
    });

    println!("Created nodes:");
    println!("  Node 1: {:?}", node1.node_id());
    println!("  Node 2: {:?}", node2.node_id());
    println!();

    // Join session (creates connection_establish span)
    let session_id = SessionId(42);
    let session_key = [0u8; 32];

    println!("Joining session...");
    node1.join_session(session_id, session_key);
    node2.join_session(session_id, session_key);
    println!("  ✓ Both nodes joined session {}", session_id.0);
    println!();

    // Queue some local events on node1
    println!("Queueing events on Node 1...");
    for i in 0..3 {
        let event = Event::new(
            node1.node_id(),
            i,
            EventType::StateUpdate,
            StateId::new(1),
            MutationOp::Set(vec![i as u8; 10]),
        );
        node1.queue_local_event(event);
    }
    println!("  ✓ Queued 3 events");
    println!();

    // Perform ticks (creates node_tick spans with nested operation spans)
    println!("Performing node ticks (this creates distributed traces)...");
    for tick_num in 0..5 {
        println!("  Tick {}:", tick_num + 1);

        // Node 1 tick (creates spans for all operations)
        node1.tick();
        println!("    - Node 1 tick complete");

        // Transfer packets from node1 to node2
        while let Some(frame) = node1.pop_outgoing() {
            node2.queue_incoming(frame);
        }

        // Node 2 tick (creates spans for receiving and processing)
        node2.tick();
        println!("    - Node 2 tick complete");

        // Transfer packets from node2 to node1
        while let Some(frame) = node2.pop_outgoing() {
            node1.queue_incoming(frame);
        }

        sleep(Duration::from_millis(100)).await;
    }
    println!();

    // Leave session (creates connection_teardown span)
    println!("Leaving session...");
    node1.leave_session();
    node2.leave_session();
    println!("  ✓ Both nodes left session");
    println!();

    // Give time for traces to be exported
    println!("Waiting for traces to be exported...");
    sleep(Duration::from_secs(2)).await;

    println!("✓ Example complete!");
    println!();
    println!("Trace hierarchy created:");
    println!("  node_tick");
    println!("    ├─ ingest_packets");
    println!("    ├─ decrypt_and_validate");
    println!("    ├─ classify_events");
    println!("    ├─ update_time_model");
    println!("    ├─ state_reconciliation");
    println!("    ├─ authorize_and_sign");
    println!("    └─ build_packets");
    println!();
    println!("View the traces in Jaeger UI at: http://localhost:16686");
    println!("  - Service: elara-tracing-demo");
    println!("  - Operation: node_tick");

    Ok(())
}
