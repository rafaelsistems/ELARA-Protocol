# elara-crypto

Cryptographic engine for the ELARA Protocol - providing identity binding, multi-ratchet encryption, and post-quantum ready security.

## Features

- **Identity Binding**: Cryptographic proof of identity with Ed25519 signatures
- **Multi-Ratchet Encryption**: Forward secrecy with ChaCha20-Poly1305 AEAD
- **Post-Quantum Ready**: Designed for future cryptographic upgrades
- **Session Security**: Secure frame processing with replay protection
- **Performance Optimized**: Efficient key derivation and encryption

## Quick Start

```rust
use elara_crypto::{Identity, SecureFrame};
use elara_core::NodeId;

// Generate a new identity
let identity = Identity::generate();
let node_id = identity.node_id();

// Create a secure frame
let frame = SecureFrame::new(
    node_id,
    session_id,
    payload,
    PacketClass::Core
)?;

// Process incoming frames
let processed = frame_processor.process_frame(frame)?;
```

## Cryptographic Primitives

- **Signing**: Ed25519 for identity and message authentication
- **Encryption**: ChaCha20-Poly1305 for authenticated encryption
- **Key Exchange**: X25519 for ECDH key agreement
- **Key Derivation**: HKDF-SHA256 for secure key derivation
- **Hashing**: SHA256 for integrity and identification

## Security Features

### Multi-Ratchet System
```
K_session_root
├── K_core (strongest protection, never dropped)
├── K_perceptual (fast ratchet, loss tolerant)
├── K_enhancement (standard protection)
└── K_cosmetic (light protection, free to drop)
```

### Replay Protection
- Sliding window for sequence number validation
- Automatic window advancement
- Out-of-order packet handling
- Wraparound protection

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.