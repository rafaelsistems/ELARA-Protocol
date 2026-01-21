//! ELARA Minimal Survival Profile - Text + Voice baseline
//!
//! MSP is the first living organism of ELARA:
//! - profile:textual - chat, presence, typing
//! - profile:voice-minimal - parametric voice state

pub mod text;
pub mod voice;

pub use text::*;
pub use voice::*;
