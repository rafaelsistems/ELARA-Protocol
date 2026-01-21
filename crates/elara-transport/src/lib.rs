//! ELARA Transport Layer - UDP and multi-path transport
//!
//! This crate provides:
//! - UDP transport
//! - Packet scheduling
//! - Multi-path support (future)
//! - NAT traversal (future)

pub mod udp;

pub use udp::*;
