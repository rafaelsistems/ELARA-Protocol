//! ELARA Wire Protocol - Binary packet format
//!
//! This crate implements the wire format for ELARA packets:
//! - Fixed header (30 bytes)
//! - Variable header extensions (TLV)
//! - Encrypted payload
//! - Auth tag (AEAD)

pub mod header;
pub mod extensions;
pub mod frame;
pub mod flags;

pub use header::*;
pub use extensions::*;
pub use frame::*;
pub use flags::*;
