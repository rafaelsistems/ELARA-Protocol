//! ELARA Time Engine - Dual clock system and temporal control
//!
//! This crate implements the Time Engine:
//! - τp (Perceptual Time): monotonic, smooth, local-driven
//! - τs (State Time): elastic, drift-correctable, convergence-oriented
//! - Reality Window management
//! - Network model and horizon adaptation
//! - Prediction and correction loops

pub mod clock;
pub mod network;
pub mod engine;

pub use clock::*;
pub use network::*;
pub use engine::*;
