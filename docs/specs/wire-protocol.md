# ELARA Wire Protocol Specification v0

The wire protocol defines the binary contract between all ELARA nodes.

## Design Constraints

- **Endian-neutral**: Little-endian canonical
- **Zero-copy friendly**: Aligned fields, no complex parsing
- **UDP/QUIC friendly**: Self-contained frames
- **Unordered & lossy tolerant**: No implicit ordering assumptions
- **Mobile safe**: Minimal memory allocation
- **Swarm scalable**: No central coordination required
- **Crypto-bound**: All payloads authenticated and encrypted
- **Future extensible**: Version and extension fields
- **NO platform API dependency**
- **NO codec dependency**

## Frame Structure

```
┌─────────────────────────────────────────────────────┐
│              Fixed Header (24 bytes)                 │
├─────────────────────────────────────────────────────┤
│         Variable Header Extensions (TLV)             │
├─────────────────────────────────────────────────────┤
│              Encrypted Payload                       │
├─────────────────────────────────────────────────────┤
│              Auth Tag (16 bytes)                     │
└─────────────────────────────────────────────────────┘
```

## Fixed Header Format

```
Offset  Size  Field         Description
──────────────────────────────────────────────────────
0       1     VERSION       [V:4][C:4] Version + Crypto suite
1       1     FLAGS         Frame flags
2       2     HEADER_LEN    Total header length (for extensions)
4       8     SESSION_ID    Reality space binding
12      8     NODE_ID       Source identity fingerprint
20      1     CLASS         Packet class
21      1     PROFILE       Representation profile hint
22      2     SEQ_WINDOW    [SEQ:16] Sequence number
24      4     TIME_HINT     Offset relative to τs (signed i32)
──────────────────────────────────────────────────────
Total: 28 bytes (minimum)
```

### VERSION Byte

```
Bits 7-4: Wire protocol version (0-15)
Bits 3-0: Crypto suite ID (0-15)

Current values:
- Version: 0 (draft)
- Crypto: 0 (ChaCha20-Poly1305 + Ed25519 + X25519)
```

### FLAGS Byte

```
Bit 7: MULTIPATH  - Frame may arrive via multiple paths
Bit 6: RELAY      - Frame is being relayed
Bit 5: FRAGMENT   - Frame is a fragment
Bit 4: REPAIR     - Frame is a repair/retransmit
Bit 3: PRIORITY   - High priority frame
Bit 2: EXTENSION  - Has header extensions
Bit 1: Reserved
Bit 0: Reserved
```

### Packet Class

```rust
#[repr(u8)]
pub enum PacketClass {
    Core = 0x00,        // Essential state, never drop
    Perceptual = 0x01,  // Real-time sensory, drop old
    Enhancement = 0x02, // Quality boost, drop under pressure
    Cosmetic = 0x03,    // Non-essential, free to drop
    Repair = 0x04,      // Gap fill, context-dependent
}
```

### Representation Profile

```rust
#[repr(u8)]
pub enum RepresentationProfile {
    Raw = 0x00,
    Textual = 0x01,
    VoiceMinimal = 0x02,
    VoiceRich = 0x03,
    VideoPerceptual = 0x04,
    GroupSwarm = 0x05,
    LivestreamAsymmetric = 0x06,
    Agent = 0x07,
    // 0x80+ reserved for custom profiles
}
```

### TIME_HINT

Signed 32-bit offset in milliseconds relative to sender's τs:

```rust
// Encoding
let time_hint: i32 = (event_time - τs).as_millis() as i32;

// Decoding
let event_time = receiver_τs + Duration::from_millis(time_hint as i64);
```

### SEQ_WINDOW

16-bit sequence number for replay protection:

```rust
// Sequence wraps at 65536
let seq: u16 = self.next_seq;
self.next_seq = self.next_seq.wrapping_add(1);
```

## Header Extensions (TLV)

When FLAGS.EXTENSION is set, variable extensions follow the fixed header:

```
┌─────────┬─────────┬─────────────────┐
│ Type(1) │ Len(1)  │ Value(Len)      │
└─────────┴─────────┴─────────────────┘
```

### Extension Types

| Type | Name | Description |
|------|------|-------------|
| 0x01 | FRAGMENT_INFO | Fragment sequence info |
| 0x02 | RELAY_PATH | Relay routing info |
| 0x03 | PRIORITY_HINT | Detailed priority |
| 0x04 | TIMESTAMP_FULL | Full 64-bit timestamp |
| 0x05 | ACK_VECTOR | Selective acknowledgment |
| 0x06 | EPOCH_SYNC | Key epoch synchronization |
| 0xFF | END | End of extensions |

### Fragment Info Extension

```
┌─────────┬─────────┬─────────┬─────────┐
│ Frag ID │ Frag Seq│ Total   │ Flags   │
│ (2)     │ (1)     │ (1)     │ (1)     │
└─────────┴─────────┴─────────┴─────────┘
```

## Encrypted Payload

The payload contains **Event Blocks**, not raw media:

```
┌─────────────────────────────────────────┐
│ Event Block 1                           │
├─────────────────────────────────────────┤
│ Event Block 2                           │
├─────────────────────────────────────────┤
│ ...                                     │
└─────────────────────────────────────────┘
```

### Event Block Format

```
Offset  Size  Field           Description
──────────────────────────────────────────────────────
0       1     EVENT_TYPE      Event type code
1       8     STATE_ID        Target state identifier
9       2     VERSION_LEN     Version vector length
11      var   VERSION_VEC     Encoded version vector
var     2     DELTA_LEN       Delta payload length
var     var   DELTA           Encoded mutation delta
──────────────────────────────────────────────────────
```

### Event Types

```rust
#[repr(u8)]
pub enum EventType {
    // Core state events
    StateCreate = 0x01,
    StateUpdate = 0x02,
    StateDelete = 0x03,
    
    // Authority events
    AuthorityGrant = 0x10,
    AuthorityRevoke = 0x11,
    
    // Session events
    SessionJoin = 0x20,
    SessionLeave = 0x21,
    SessionSync = 0x22,
    
    // Time events
    TimeSync = 0x30,
    TimeCorrection = 0x31,
    
    // Repair events
    StateRequest = 0x40,
    StateResponse = 0x41,
    GapFill = 0x42,
    
    // Profile-specific (0x80+)
    TextAppend = 0x80,
    TextEdit = 0x81,
    VoiceFrame = 0x90,
    // ...
}
```

## Cryptographic Binding

### AEAD Construction

```rust
// Encryption
let nonce = derive_nonce(node_id, seq, class);
let aad = &frame[0..HEADER_SIZE];  // Header as AAD
let ciphertext = ChaCha20Poly1305::encrypt(
    key: class_key,
    nonce: nonce,
    aad: aad,
    plaintext: payload
);

// Nonce derivation (12 bytes)
fn derive_nonce(node_id: NodeId, seq: u16, class: PacketClass) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[0..8].copy_from_slice(&node_id.to_bytes());
    nonce[8..10].copy_from_slice(&seq.to_le_bytes());
    nonce[10] = class as u8;
    nonce[11] = 0;
    nonce
}
```

### Key Hierarchy

```
K_session_root (established via key exchange)
    │
    ├── K_core = HKDF(root, "elara-core-v0")
    │   └── Per-message: Ratchet with slow advance
    │
    ├── K_perceptual = HKDF(root, "elara-perceptual-v0")
    │   └── Per-message: Ratchet with fast advance
    │
    ├── K_enhancement = HKDF(root, "elara-enhancement-v0")
    │   └── Per-message: Ratchet with medium advance
    │
    └── K_cosmetic = HKDF(root, "elara-cosmetic-v0")
        └── Per-message: Ratchet with minimal advance
```

### Replay Protection

Each (node_id, class) pair maintains independent replay window:

```rust
struct ReplayWindow {
    min_seq: u16,
    bitmap: u64,  // 64-packet window
}

impl ReplayWindow {
    fn accept(&mut self, seq: u16) -> bool {
        // Handle wraparound with signed comparison
        let offset = seq.wrapping_sub(self.min_seq);
        if offset > 32768 {
            return false;  // Too old
        }
        // ... bitmap check and update
    }
}
```

## Frame Size Limits

| Constraint | Value | Rationale |
|------------|-------|-----------|
| Min frame | 28 bytes | Header only |
| Max frame | 1200 bytes | UDP-safe MTU |
| Max payload | ~1150 bytes | After header + tag |
| Max extensions | 256 bytes | Practical limit |

## Cross-Platform Parsing

All parsing uses explicit byte operations:

```rust
// Portable read functions
fn read_u8(buf: &[u8], offset: usize) -> u8;
fn read_u16_le(buf: &[u8], offset: usize) -> u16;
fn read_u32_le(buf: &[u8], offset: usize) -> u32;
fn read_u64_le(buf: &[u8], offset: usize) -> u64;
fn read_i32_le(buf: &[u8], offset: usize) -> i32;
```

Compatible implementations required for:
- Rust
- Kotlin (Android)
- Swift (iOS)
- C++ (Desktop/Embedded)
- Go (Server)
- JavaScript/WASM (Web)

## Wire Version Evolution

- **v0**: Draft specification (current)
- **v1**: First stable release (planned)

Version negotiation occurs during session establishment, not per-frame.
