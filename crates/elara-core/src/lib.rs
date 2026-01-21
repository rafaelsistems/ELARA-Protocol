//! ELARA Core - Fundamental types and primitives
//!
//! This crate defines the core types used throughout the ELARA protocol:
//! - Identifiers (NodeId, SessionId, StateId)
//! - Time primitives (StateTime, PerceptualTime)
//! - State atoms and events
//! - Protocol constants and configuration

pub mod id;
pub mod time;
pub mod state;
pub mod event;
pub mod class;
pub mod error;

pub use id::*;
pub use time::*;
pub use state::*;
pub use event::*;
pub use class::*;
pub use error::*;
