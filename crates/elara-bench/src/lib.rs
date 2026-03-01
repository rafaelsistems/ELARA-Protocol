//! # ELARA Benchmark Suite
//!
//! Production-grade benchmark suite for the ELARA Protocol using criterion for
//! statistical analysis and performance tracking.
//!
//! ## Overview
//!
//! This crate provides comprehensive benchmarks for all critical components:
//! - Wire protocol encoding/decoding
//! - Cryptographic operations
//! - State reconciliation
//! - Time engine operations
//!
//! ## Usage
//!
//! Run all benchmarks:
//! ```bash
//! cargo bench --package elara-bench
//! ```
//!
//! Run specific benchmark:
//! ```bash
//! cargo bench --package elara-bench --bench wire_protocol
//! ```
//!
//! ## Configuration
//!
//! Benchmarks can be configured via `BenchmarkConfig` for custom scenarios.

use std::time::Duration;

/// Configuration for benchmark execution
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of iterations for warmup
    pub warmup_iterations: usize,
    /// Duration of warmup phase
    pub warmup_time: Duration,
    /// Duration of measurement phase
    pub measurement_time: Duration,
    /// Sample size for statistical analysis
    pub sample_size: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 100,
            warmup_time: Duration::from_secs(3),
            measurement_time: Duration::from_secs(5),
            sample_size: 100,
        }
    }
}

impl BenchmarkConfig {
    /// Create a quick benchmark configuration for CI
    pub fn quick() -> Self {
        Self {
            warmup_iterations: 10,
            warmup_time: Duration::from_secs(1),
            measurement_time: Duration::from_secs(2),
            sample_size: 50,
        }
    }

    /// Create a thorough benchmark configuration for baseline establishment
    pub fn thorough() -> Self {
        Self {
            warmup_iterations: 500,
            warmup_time: Duration::from_secs(10),
            measurement_time: Duration::from_secs(30),
            sample_size: 200,
        }
    }
}

/// Standard payload sizes for benchmarking (64B to 1KB for wire protocol)
pub const PAYLOAD_SIZES: &[usize] = &[64, 256, 1024, 4096, 16384];

/// Wire protocol payload sizes (limited by MTU)
pub const WIRE_PAYLOAD_SIZES: &[usize] = &[64, 256, 1024];

/// Standard event counts for state benchmarks
pub const EVENT_COUNTS: &[usize] = &[10, 100, 1000];
