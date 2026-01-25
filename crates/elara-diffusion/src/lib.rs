//! ELARA Swarm Diffusion
//!
//! State propagation for livestream and group communication.
//! This is NOT a CDN or traditional streaming relay.
//!
//! # Philosophy
//!
//! Traditional streaming: Broadcaster → CDN → Viewers (one-way pipe)
//! ELARA diffusion: Authority → Interest-based propagation → Observers
//!
//! Key concepts:
//! - Authority: Who can mutate state (broadcaster in livestream)
//! - Interest: Who wants to observe state (viewers)
//! - Diffusion: How state propagates through the swarm
//! - Topology: The shape of the propagation network
//!
//! # Asymmetric Authority
//!
//! In a livestream, the broadcaster has AUTHORITY over visual/audio state.
//! Viewers have INTEREST in that state but cannot mutate it.
//! Chat is a separate state where viewers have authority over their messages.

pub mod authority;
pub mod interest;
pub mod propagation;
pub mod swarm;
pub mod topology;

pub use authority::*;
pub use interest::*;
pub use propagation::*;
pub use swarm::*;
pub use topology::*;
