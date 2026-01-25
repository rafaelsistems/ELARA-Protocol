//! ELARA Transport Layer - UDP and multi-path transport
//!
//! This crate provides:
//! - UDP transport
//! - Packet scheduling
//! - Multi-path support (future)
//! - NAT traversal (STUN)

pub mod stun;
pub mod udp;

pub use stun::{NatType, StunClient, StunResult, STUN_SERVERS};
pub use udp::*;
