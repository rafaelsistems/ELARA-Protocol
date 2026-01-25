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
//! - Comprehensive chaos harness (5 categories)

pub mod chaos;
pub mod chaos_harness;
pub mod integration;
pub mod network_test;
pub mod simulator;
pub mod state_fuzzer;
pub mod time_simulator;

pub use chaos::*;
pub use chaos_harness::*;
pub use integration::{
    test_basic_convergence, test_convergence_under_stress, test_convergence_with_chaos,
    test_degradation_ladder, test_presence_floor, IntegrationTestConfig, IntegrationTestHarness,
    IntegrationTestResult, SimulatedMessage,
};
pub use simulator::*;
pub use state_fuzzer::*;
pub use time_simulator::*;
