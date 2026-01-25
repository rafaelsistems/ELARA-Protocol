//! ELARA Crypto Engine - Cryptographic primitives and ratchet management
//!
//! ELARA Cryptographic Engine
//!
//! Provides cryptographic primitives for the ELARA protocol:
//! - Identity management (Ed25519)
//! - AEAD encryption (ChaCha20-Poly1305)
//! - Multi-ratchet key derivation
//! - Replay protection
//! - Secure frame encryption/decryption

pub mod aead;
pub mod identity;
pub mod ratchet;
pub mod replay;
pub mod secure_frame;

pub use aead::*;
pub use identity::*;
pub use ratchet::*;
pub use replay::*;
pub use secure_frame::*;
