# elara-runtime

Runtime engine for the ELARA Protocol - providing node orchestration, event loop processing, and stream management.

## Features

- **Event Loop**: Async event processing with tokio
- **Node Orchestration**: Multi-node session management
- **Stream Processing**: Real-time media stream handling
- **Session Management**: Secure session lifecycle
- **Feed Processing**: Text and media feed handling

## Quick Start

```rust
use elara_runtime::{Node, NodeConfig};
use elara_core::NodeId;

// Create node configuration
let config = NodeConfig {
    node_id: NodeId::generate(),
    bind_addr: "0.0.0.0:0".parse()?,
    ..Default::default()
};

// Create and start node
let mut node = Node::new(config).await?;

// Start event loop
node.run().await?;
```

## Architecture

### Event Processing Loop
```
Event → Validation → Processing → State Update → Propagation
  ↓         ↓            ↓            ↓            ↓
Network → Invariants → Business → CRDT Merge → Network
Input     Check      Logic       Apply        Output
```

### Node Components

- **Core Engine**: Event processing and state management
- **Network Layer**: UDP transport with STUN/TURN
- **Crypto Layer**: Identity and session security
- **Time Engine**: Dual clock synchronization
- **State Field**: Distributed state reconciliation
- **Stream Manager**: Media stream lifecycle

### Session Management

```rust
pub struct Session {
    pub session_id: SessionId,
    pub participants: HashMap<NodeId, Participant>,
    pub state: SessionState,
    pub streams: HashMap<StreamId, Stream>,
}
```

## Stream Processing

### Stream Types
- **Text Streams**: Chat messages, typing indicators
- **Voice Streams**: Audio frames with degradation
- **Visual Streams**: Video keyframes and deltas
- **Presence Streams**: Availability and status

### Stream Lifecycle
```
Start → Active → Update → ... → Update → End
  ↓       ↓        ↓             ↓      ↓
Create  Process  Transform     Process  Cleanup
```

## Configuration

```rust
pub struct NodeConfig {
    pub node_id: NodeId,
    pub bind_addr: SocketAddr,
    pub max_peers: usize,
    pub event_buffer_size: usize,
    pub prediction_entropy: u64,
    pub degradation_config: DegradationConfig,
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.