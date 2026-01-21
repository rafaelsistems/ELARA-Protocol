//! ELARA Runtime - Node orchestration and main loop
//!
//! This crate implements the 12-stage runtime loop:
//! 1. Advance clocks (τp, τs)
//! 2. Ingest packets
//! 3. Decrypt and validate
//! 4. Classify events
//! 5. Update time model
//! 6. Reconcile state
//! 7. Generate predictions
//! 8. Project to representation
//! 9. Collect local events
//! 10. Authorize and sign
//! 11. Build packets
//! 12. Schedule transmission

pub mod node;

pub use node::*;
