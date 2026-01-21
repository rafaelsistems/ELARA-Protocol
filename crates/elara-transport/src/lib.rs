//! ELARA Transport Layer - UDP and multi-path transport
//!
//! This crate provides:
//! - UDP transport
//! - Packet scheduling
//! - Multi-path support (future)
//! - NAT traversal (STUN)

pub mod udp;
pub mod stun;

pub use udp::*;
pub use stun::{StunClient, StunResult, NatType, STUN_SERVERS};
