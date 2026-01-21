//! ELARA Voice State - Parametric voice encoding
//!
//! This is NOT audio PCM or Opus codec.
//! This is the STATE OF SPEECH for reality synchronization.
//!
//! # Philosophy
//!
//! Traditional voice: Encode audio samples → transmit → decode
//! ELARA voice: Extract speech parameters → transmit state → synthesize
//!
//! Key concepts:
//! - Voice as parametric state (pitch, energy, spectral envelope)
//! - Prediction under packet loss (voice continues)
//! - Graceful degradation (L0: full voice → L5: presence only)
//!
//! # Degradation Ladder (Voice)
//!
//! - L0: Full parametric voice (pitch, formants, prosody, emotion)
//! - L1: Reduced formants, simplified prosody
//! - L2: Pitch + energy only, robotic quality
//! - L3: Symbolic (speaking/silent indicator)
//! - L4: Presence pulse only
//! - L5: Identity heartbeat

pub mod state;
pub mod frame;
pub mod prediction;
pub mod encoding;
pub mod synthesis;

pub use state::*;
pub use frame::*;
pub use prediction::*;
pub use encoding::*;
pub use synthesis::*;
