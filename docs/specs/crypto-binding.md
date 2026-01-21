# ELARA Cryptographic Binding Specification v0

All communication in ELARA is cryptographically bound. There is no trust - only mathematical proof.

## Design Principles

1. **Identity is Key-Derived**: NodeId comes from public key
2. **Session Keys are Hierarchical**: One root, multiple class keys
3. **Forward Secrecy**: Ratcheting prevents past compromise
4. **Class Isolation**: Compromising one class doesn't affect others
5. **Server Blindness**: Infrastructure cannot read content

## Cryptographic Primitives

| Purpose | Algorithm | Parameters |
|---------|-----------|------------|
| Signing | Ed25519 | 256-bit keys |
| Key Exchange | X25519 | 256-bit keys |
| AEAD | ChaCha20-Poly1305 | 256-bit key, 96-bit nonce |
| KDF | HKDF-SHA256 | Variable output |
| Hash | SHA-256 | 256-bit output |

## Identity System

### Key Generation

```rust
// Generate new identity
let signing_key = Ed25519SigningKey::generate(&mut OsRng);
let verifying_key = signing_key.verifying_key();

let encryption_secret = X25519StaticSecret::random_from_rng(&mut OsRng);
let encryption_public = X25519PublicKey::from(&encryption_secret);

// Derive NodeId from public keys
let node_id = derive_node_id(&verifying_key, &encryption_public);
```

### NodeId Derivation

```rust
fn derive_node_id(
    verifying_key: &Ed25519VerifyingKey,
    encryption_public: &X25519PublicKey
) -> NodeId {
    let mut hasher = Sha256::new();
    hasher.update(b"elara-node-id-v0");
    hasher.update(verifying_key.as_bytes());
    hasher.update(encryption_public.as_bytes());
    
    let hash = hasher.finalize();
    NodeId::from_bytes(&hash[0..8])
}
```

### Identity Serialization

```rust
struct SerializedIdentity {
    version: u8,                    // 0x00
    verifying_key: [u8; 32],        // Ed25519 public
    encryption_public: [u8; 32],    // X25519 public
    signature: [u8; 64],            // Self-signature
}
```

## Session Establishment

### Key Exchange Protocol

```
Alice                                    Bob
──────                                   ───
1. Generate ephemeral X25519 keypair
2. Send: (ephemeral_public, node_id, signature)
                            ──────────────────>
                                         3. Verify signature
                                         4. Generate ephemeral keypair
                                         5. Compute shared secret
                            <──────────────────
6. Receive: (ephemeral_public, node_id, signature)
7. Verify signature
8. Compute shared secret
9. Derive session root key
```

### Session Root Key Derivation

```rust
fn derive_session_root(
    shared_secret: &[u8; 32],
    alice_node_id: NodeId,
    bob_node_id: NodeId,
    session_id: SessionId
) -> [u8; 32] {
    // Canonical ordering
    let (first, second) = if alice_node_id < bob_node_id {
        (alice_node_id, bob_node_id)
    } else {
        (bob_node_id, alice_node_id)
    };
    
    let mut info = Vec::new();
    info.extend_from_slice(b"elara-session-root-v0");
    info.extend_from_slice(&session_id.to_bytes());
    info.extend_from_slice(&first.to_bytes());
    info.extend_from_slice(&second.to_bytes());
    
    hkdf_sha256(shared_secret, &[], &info)
}
```

## Multi-Ratchet Key Hierarchy

### Class Key Derivation

```rust
fn derive_class_key(session_root: &[u8; 32], class: PacketClass) -> [u8; 32] {
    let label = match class {
        PacketClass::Core => b"elara-class-core-v0",
        PacketClass::Perceptual => b"elara-class-perceptual-v0",
        PacketClass::Enhancement => b"elara-class-enhancement-v0",
        PacketClass::Cosmetic => b"elara-class-cosmetic-v0",
        PacketClass::Repair => b"elara-class-repair-v0",
    };
    
    hkdf_sha256(session_root, &[], label)
}
```

### Ratchet Structure

```rust
struct ClassRatchet {
    chain_key: [u8; 32],
    epoch: u32,
    message_index: u32,
}

impl ClassRatchet {
    fn advance_message(&mut self) -> [u8; 32] {
        // Derive message key
        let message_key = hkdf_sha256(
            &self.chain_key,
            &[],
            &format!("msg-{}", self.message_index).as_bytes()
        );
        
        // Advance chain
        self.chain_key = hkdf_sha256(
            &self.chain_key,
            &[],
            b"chain-advance"
        );
        self.message_index += 1;
        
        // Auto-advance epoch if needed
        if self.message_index >= EPOCH_THRESHOLD {
            self.advance_epoch();
        }
        
        message_key
    }
    
    fn advance_epoch(&mut self) {
        self.chain_key = hkdf_sha256(
            &self.chain_key,
            &[],
            &format!("epoch-{}", self.epoch + 1).as_bytes()
        );
        self.epoch += 1;
        self.message_index = 0;
    }
}
```

### Ratchet Rates by Class

| Class | Epoch Threshold | Rekey Trigger | Security Level |
|-------|-----------------|---------------|----------------|
| Core | 1000 messages | Manual | Highest |
| Perceptual | 100 messages | Time-based | High |
| Enhancement | 500 messages | Auto | Medium |
| Cosmetic | 1000 messages | Lazy | Standard |

## AEAD Encryption

### Nonce Construction

```rust
fn derive_nonce(node_id: NodeId, seq: u16, class: PacketClass) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    
    // Bytes 0-7: Node ID (ensures uniqueness across nodes)
    nonce[0..8].copy_from_slice(&node_id.to_bytes());
    
    // Bytes 8-9: Sequence number (ensures uniqueness per node)
    nonce[8..10].copy_from_slice(&seq.to_le_bytes());
    
    // Byte 10: Packet class (ensures uniqueness across classes)
    nonce[10] = class as u8;
    
    // Byte 11: Reserved (zero)
    nonce[11] = 0;
    
    nonce
}
```

### Encryption Process

```rust
fn encrypt_frame(
    ratchet: &mut ClassRatchet,
    header: &FixedHeader,
    payload: &[u8]
) -> Vec<u8> {
    // 1. Get message key from ratchet
    let message_key = ratchet.advance_message();
    
    // 2. Create cipher
    let cipher = ChaCha20Poly1305::new(&message_key.into());
    
    // 3. Derive nonce
    let nonce = derive_nonce(header.node_id, header.seq(), header.class);
    
    // 4. Serialize header as AAD
    let mut aad = Vec::new();
    header.serialize(&mut aad);
    
    // 5. Encrypt with AAD
    let ciphertext = cipher.encrypt(&nonce.into(), Payload {
        msg: payload,
        aad: &aad,
    }).expect("encryption failed");
    
    // 6. Assemble frame
    let mut frame = aad;
    frame.extend_from_slice(&ciphertext);
    frame
}
```

### Decryption Process

```rust
fn decrypt_frame(
    ratchet: &mut ClassRatchet,
    replay_window: &mut ReplayWindow,
    frame: &[u8]
) -> Result<Vec<u8>, CryptoError> {
    // 1. Parse header
    let header = FixedHeader::parse(&frame[..HEADER_SIZE])?;
    
    // 2. Check replay
    if !replay_window.accept(header.seq()) {
        return Err(CryptoError::Replay);
    }
    
    // 3. Get message key (may need to sync ratchet)
    let message_key = ratchet.message_key();
    
    // 4. Create cipher
    let cipher = ChaCha20Poly1305::new(&message_key.into());
    
    // 5. Derive nonce
    let nonce = derive_nonce(header.node_id, header.seq(), header.class);
    
    // 6. Decrypt with AAD verification
    let plaintext = cipher.decrypt(&nonce.into(), Payload {
        msg: &frame[HEADER_SIZE..],
        aad: &frame[..HEADER_SIZE],
    })?;
    
    // 7. Advance ratchet
    ratchet.advance_message();
    
    Ok(plaintext)
}
```

## Replay Protection

### Window Management

```rust
struct ReplayWindow {
    min_seq: u16,
    bitmap: u64,
    window_size: u16,
}

impl ReplayWindow {
    fn accept(&mut self, seq: u16) -> bool {
        let offset = seq.wrapping_sub(self.min_seq);
        
        // Handle wraparound (offset > 32768 means "before" min_seq)
        if offset > 32768 {
            return false;  // Too old
        }
        
        if offset >= self.window_size {
            // Advance window
            let advance = offset - self.window_size + 1;
            if advance >= 64 {
                self.bitmap = 0;
            } else {
                self.bitmap >>= advance;
            }
            self.min_seq = seq.wrapping_sub(self.window_size - 1);
        }
        
        // Check and set bit
        let bit_offset = seq.wrapping_sub(self.min_seq);
        if bit_offset < 64 {
            let mask = 1u64 << bit_offset;
            if self.bitmap & mask != 0 {
                return false;  // Replay
            }
            self.bitmap |= mask;
        }
        
        true
    }
}
```

### Per-Class Windows

Each (node_id, class) pair has independent replay protection:

```rust
struct ReplayManager {
    windows: HashMap<(NodeId, PacketClass), ReplayWindow>,
}
```

## Authority Proofs

### Event Signing

```rust
fn sign_event(signing_key: &Ed25519SigningKey, event: &Event) -> AuthorityProof {
    let mut message = Vec::new();
    message.extend_from_slice(b"elara-event-v0");
    message.extend_from_slice(&event.id.to_bytes());
    message.extend_from_slice(&event.target_state.to_bytes());
    message.extend_from_slice(&event.version_ref.encode());
    message.extend_from_slice(&event.mutation.encode());
    
    let signature = signing_key.sign(&message);
    
    AuthorityProof {
        signature: signature.to_bytes(),
        delegation_chain: None,
    }
}
```

### Delegation Chains

```rust
struct DelegationLink {
    delegator: NodeId,
    delegate: NodeId,
    scope: AuthorityScope,
    expiry: Option<StateTime>,
    signature: [u8; 64],
}

fn verify_delegation_chain(
    chain: &[DelegationLink],
    original_authority: NodeId,
    claimed_authority: NodeId
) -> bool {
    let mut current = original_authority;
    
    for link in chain {
        if link.delegator != current {
            return false;
        }
        if !verify_delegation_signature(link) {
            return false;
        }
        if let Some(expiry) = link.expiry {
            if expiry < current_time() {
                return false;
            }
        }
        current = link.delegate;
    }
    
    current == claimed_authority
}
```

## Security Considerations

### Key Compromise

| Compromised | Impact | Mitigation |
|-------------|--------|------------|
| Signing key | Full identity compromise | Revocation mechanism |
| Session root | All session traffic | Session rotation |
| Class key | One class traffic | Class isolation |
| Message key | One message | Forward secrecy |

### Threat Model

**Protected against:**
- Passive eavesdropping
- Message replay
- Message modification
- Server-side content access
- Cross-session correlation (with proper key management)

**Not protected against:**
- Endpoint compromise
- Traffic analysis (metadata)
- Denial of service
- Quantum computers (future concern)

## Future Considerations

- Post-quantum key exchange (Kyber/NTRU)
- Threshold signatures for group authority
- Zero-knowledge proofs for privacy-preserving authority
