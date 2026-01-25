#![allow(mixed_script_confusables)]
//! ELARA State Engine - State reconciliation and divergence control
//!
//! This crate implements the State Reconciliation Engine:
//! - State field management
//! - Authority checking
//! - Causality validation
//! - Delta merge operations
//! - Divergence control
//! - Partition handling

pub mod field;
pub mod reconcile;

pub use field::*;
pub use reconcile::*;
