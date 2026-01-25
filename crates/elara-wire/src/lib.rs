//! ELARA Wire Protocol - Binary packet format
//!
//! This crate implements the wire format for ELARA packets:
//! - Fixed header (30 bytes)
//! - Variable header extensions (TLV)
//! - Encrypted payload
//! - Auth tag (AEAD)

pub mod extensions;
pub mod flags;
pub mod frame;
pub mod header;

pub use extensions::*;
pub use flags::*;
pub use frame::*;
pub use header::*;
