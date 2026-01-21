//! ELARA Visual State
//!
//! Video as STATE, not stream. This is NOT a video codec.
//!
//! # Philosophy
//!
//! Traditional video: Camera → Frames → Encode → Network → Decode → Display
//! ELARA video: Camera → Visual State → Sync Reality → Render
//!
//! Visual state captures the MEANING of what's seen, not the pixels.
//! When network degrades, we predict and interpolate state, not freeze frames.
//!
//! # Degradation Ladder
//!
//! - L0: Full visual state (keyframes + deltas + all details)
//! - L1: Reduced visual (lower resolution, fewer updates)
//! - L2: Simplified visual (face + pose only)
//! - L3: Symbolic visual (avatar representation)
//! - L4: Minimal visual (static avatar + activity)
//! - L5: Latent visual (last known state)

pub mod state;
pub mod face;
pub mod pose;
pub mod scene;
pub mod keyframe;
pub mod prediction;
pub mod encoding;

pub use state::*;
pub use face::*;
pub use pose::*;
pub use scene::*;
pub use keyframe::*;
pub use prediction::*;
pub use encoding::*;
