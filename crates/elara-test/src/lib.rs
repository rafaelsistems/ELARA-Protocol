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
pub mod simulator;
pub mod time_simulator;
pub mod state_fuzzer;
pub mod integration;
pub mod chaos_harness;
pub mod network_test;

pub use chaos::*;
pub use simulator::*;
pub use time_simulator::*;
pub use state_fuzzer::*;
pub use integration::{
    IntegrationTestConfig, IntegrationTestHarness, IntegrationTestResult,
    SimulatedMessage, test_basic_convergence, test_convergence_with_chaos,
    test_convergence_under_stress, test_degradation_ladder, test_presence_floor,
};
pub use chaos_harness::*;
