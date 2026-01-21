# ELARA MSP Demo

Demonstration of the **Minimal Survival Profile** - ELARA's first living organism.

## Features

- **Text Chat** - Real-time messaging with causal ordering
- **Presence** - 5-dimensional presence vector visualization
- **Typing Indicators** - See when peers are typing
- **Voice State** - Simulated parametric voice (energy levels)
- **Degradation Ladder** - Visualize graceful degradation from L0 to L5

## Running

### Single Node (Wait for Connection)

```bash
cargo run -p msp-demo
# Enter name: Alice
# Enter port: 9000
# Leave peer empty to wait
```

### Two Nodes (Local Test)

Terminal 1:
```bash
cargo run -p msp-demo
# Name: Alice
# Port: 9000
# Peer: (empty)
```

Terminal 2:
```bash
cargo run -p msp-demo
# Name: Bob
# Port: 9001
# Peer: 127.0.0.1:9000
```

## Commands

| Command | Description |
|---------|-------------|
| `/msg <text>` | Send a message (or just type text) |
| `/voice` | Toggle voice (simulated) |
| `/presence` | Show detailed presence info |
| `/degrade` | Simulate network degradation |
| `/recover` | Recover from degradation |
| `/status` | Show node status |
| `/peers` | List connected peers |
| `/help` | Show help |
| `/quit` | Exit |

## Degradation Ladder

```
┌─────────────────────────────────────────────────────────┐
│              Degradation Ladder Visualization           │
├─────────────────────────────────────────────────────────┤
│ → L0: Full Perception      [████████████████████]       │
│   L1: Distorted            [████████████████░░░░]       │
│   L2: Fragmented           [████████████░░░░░░░░]       │
│   L3: Symbolic             [████████░░░░░░░░░░░░]       │
│   L4: Minimal              [████░░░░░░░░░░░░░░░░]       │
│   L5: Latent               [██░░░░░░░░░░░░░░░░░░]       │
└─────────────────────────────────────────────────────────┘
```

### What's Available at Each Level

| Level | Text | Voice | Presence | Typing |
|-------|------|-------|----------|--------|
| L0 | ✓ | ✓ | ✓ | ✓ |
| L1 | ✓ | ~ | ✓ | ✓ |
| L2 | ✓ | ~ | ~ | ✓ |
| L3 | ✓ | ✗ | ~ | ~ |
| L4 | ~ | ✗ | ~ | ✗ |
| L5 | ✗ | ✗ | ~ | ✗ |

Legend: ✓ = Full, ~ = Reduced, ✗ = Unavailable

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     MSP Demo App                         │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │   DemoUI    │  │  DemoNode   │  │   Network   │     │
│  │  (ui.rs)    │──│  (node.rs)  │──│ (UDP Socket)│     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│         │                │                │             │
│         ▼                ▼                ▼             │
│  ┌─────────────────────────────────────────────────┐   │
│  │                  ELARA Crates                    │   │
│  │  elara-core | elara-msp | elara-crypto | ...    │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Protocol Demonstration

This demo validates:

1. **Cryptographic Identity** - Each node has unique Ed25519 identity
2. **State Synchronization** - Messages sync across nodes
3. **Presence Tracking** - 5D presence vector updates
4. **Graceful Degradation** - Smooth transition through degradation levels
5. **Session Continuity** - Identity persists even at L5

## Next Steps

- Add actual voice codec integration
- Implement encrypted wire protocol
- Add NAT traversal (STUN)
- Mobile SDK integration
