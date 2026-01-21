//! ELARA Test Harness - Chaos testing and protocol validation
//!
//! This crate provides:
//! - Jitter chaos testing
//! - Packet loss torture testing
//! - NAT swarm testing
//! - Network simulation
//! - Time engine simulation
//! - State engine fuzzing
//! - End-to-end integration testing

pub mod chaos;
pub mod simulator;
pub mod time_simulator;
pub mod state_fuzzer;
pub mod integration;

pub use chaos::*;
pub use simulator::*;
pub use time_simulator::*;
pub use state_fuzzer::*;
pub use integration::*;
