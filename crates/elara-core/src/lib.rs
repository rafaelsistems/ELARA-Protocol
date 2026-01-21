//! ELARA Core - Fundamental types and primitives
//!
//! This crate defines the core types used throughout the ELARA protocol:
//! - Identifiers (NodeId, SessionId, StateId)
//! - Time primitives (StateTime, PerceptualTime)
//! - State atoms and events
//! - Protocol constants and configuration
//! - **Hard Invariants** - System laws that all layers must obey
//!
//! # Hard Invariants
//!
//! ELARA is governed by five hard invariants. These are not guidelines—they are
//! system laws. If any single invariant falls, the system is not ELARA.
//!
//! 1. **Reality Never Waits** - System never blocks reality for synchronization
//! 2. **Presence Over Packets** - Existence matters more than data perfection
//! 3. **Experience Degrades, Never Collapses** - Quality reduces, never fails
//! 4. **Event Is Truth, State Is Projection** - Events are authoritative
//! 5. **Identity Survives Transport** - Identity persists beyond connections
//!
//! See [`invariants`] module for details.

pub mod id;
pub mod time;
pub mod state;
pub mod event;
pub mod class;
pub mod error;
pub mod invariants;

pub use id::*;
pub use time::*;
pub use state::*;
pub use event::*;
pub use class::*;
pub use error::*;
pub use invariants::{Invariant, InvariantViolation, InvariantCompliant};
