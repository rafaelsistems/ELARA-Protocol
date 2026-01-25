//! Visual State - The core representation of visual reality
//!
//! This is NOT a video frame. This is the STATE of what is visually happening.

use elara_core::{DegradationLevel, NodeId, StateTime};

use crate::{FaceState, PoseState, SceneState};

/// Visual state identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualStateId(pub u64);

impl VisualStateId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// The complete visual state of a node
///
/// This captures WHAT is visually happening, not HOW it looks pixel-by-pixel.
/// The renderer reconstructs visuals from this state.
#[derive(Debug, Clone)]
pub struct VisualState {
    /// Unique identifier for this visual state
    pub id: VisualStateId,

    /// The node this visual state belongs to
    pub source: NodeId,

    /// Timestamp of this state
    pub timestamp: StateTime,

    /// Face state (expression, gaze, landmarks)
    pub face: Option<FaceState>,

    /// Body pose state (skeleton, gestures)
    pub pose: Option<PoseState>,

    /// Scene/environment state
    pub scene: Option<SceneState>,

    /// Current degradation level
    pub degradation: DegradationLevel,

    /// Is this a keyframe (full state) or delta (changes only)?
    pub is_keyframe: bool,

    /// Reference to previous keyframe if this is a delta
    pub keyframe_ref: Option<VisualStateId>,

    /// Sequence number for ordering
    pub sequence: u64,
}

impl VisualState {
    /// Create a new keyframe visual state
    pub fn keyframe(source: NodeId, timestamp: StateTime, sequence: u64) -> Self {
        Self {
            id: VisualStateId::new(sequence),
            source,
            timestamp,
            face: None,
            pose: None,
            scene: None,
            degradation: DegradationLevel::L0_FullPerception,
            is_keyframe: true,
            keyframe_ref: None,
            sequence,
        }
    }

    /// Create a delta state referencing a keyframe
    pub fn delta(
        source: NodeId,
        timestamp: StateTime,
        sequence: u64,
        keyframe: VisualStateId,
    ) -> Self {
        Self {
            id: VisualStateId::new(sequence),
            source,
            timestamp,
            face: None,
            pose: None,
            scene: None,
            degradation: DegradationLevel::L0_FullPerception,
            is_keyframe: false,
            keyframe_ref: Some(keyframe),
            sequence,
        }
    }

    /// Set face state
    pub fn with_face(mut self, face: FaceState) -> Self {
        self.face = Some(face);
        self
    }

    /// Set pose state
    pub fn with_pose(mut self, pose: PoseState) -> Self {
        self.pose = Some(pose);
        self
    }

    /// Set scene state
    pub fn with_scene(mut self, scene: SceneState) -> Self {
        self.scene = Some(scene);
        self
    }

    /// Set degradation level
    pub fn with_degradation(mut self, level: DegradationLevel) -> Self {
        self.degradation = level;
        self
    }

    /// Get the visual complexity score (0.0 - 1.0)
    /// Higher = more data to transmit
    pub fn complexity(&self) -> f32 {
        let mut score: f32 = 0.0;

        if self.face.is_some() {
            score += 0.3;
        }
        if self.pose.is_some() {
            score += 0.3;
        }
        if self.scene.is_some() {
            score += 0.4;
        }

        if self.is_keyframe {
            score *= 1.5; // Keyframes are larger
        }

        score.min(1.0)
    }

    /// Degrade this visual state to a lower level
    /// Returns a simplified version appropriate for the degradation level
    pub fn degrade(&self, target: DegradationLevel) -> VisualState {
        let mut degraded = self.clone();
        degraded.degradation = target;

        match target {
            DegradationLevel::L0_FullPerception => {
                // Keep everything
            }
            DegradationLevel::L1_DistortedPerception => {
                // Reduce scene detail
                if let Some(ref mut scene) = degraded.scene {
                    scene.reduce_detail(0.7);
                }
            }
            DegradationLevel::L2_FragmentedPerception => {
                // Remove scene, keep face and pose
                degraded.scene = None;
            }
            DegradationLevel::L3_SymbolicPresence => {
                // Keep only face (for avatar)
                degraded.scene = None;
                degraded.pose = None;
            }
            DegradationLevel::L4_MinimalPresence => {
                // Minimal face only
                degraded.scene = None;
                degraded.pose = None;
                if let Some(ref mut face) = degraded.face {
                    face.reduce_to_minimal();
                }
            }
            DegradationLevel::L5_LatentPresence => {
                // Just presence indicator
                degraded.scene = None;
                degraded.pose = None;
                degraded.face = degraded.face.map(|f| f.to_latent());
            }
        }

        degraded
    }
}

/// Visual state for livestream (asymmetric authority)
#[derive(Debug, Clone)]
pub struct LivestreamState {
    /// The broadcaster (authority)
    pub broadcaster: NodeId,

    /// Current visual state from broadcaster
    pub visual: VisualState,

    /// Number of active viewers
    pub viewer_count: u32,

    /// Is the stream live?
    pub is_live: bool,

    /// Stream title/description
    pub title: String,

    /// Stream start time
    pub started_at: StateTime,
}

impl LivestreamState {
    /// Create a new livestream
    pub fn new(broadcaster: NodeId, title: String, started_at: StateTime) -> Self {
        Self {
            broadcaster,
            visual: VisualState::keyframe(broadcaster, started_at, 0),
            viewer_count: 0,
            is_live: true,
            title,
            started_at,
        }
    }

    /// Update visual state (only broadcaster can do this)
    pub fn update_visual(&mut self, visual: VisualState) -> Result<(), &'static str> {
        if visual.source != self.broadcaster {
            return Err("Only broadcaster can update visual state");
        }
        self.visual = visual;
        Ok(())
    }

    /// Add a viewer
    pub fn add_viewer(&mut self) {
        self.viewer_count += 1;
    }

    /// Remove a viewer
    pub fn remove_viewer(&mut self) {
        self.viewer_count = self.viewer_count.saturating_sub(1);
    }

    /// End the stream
    pub fn end_stream(&mut self) {
        self.is_live = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_state_keyframe() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(1000);
        let state = VisualState::keyframe(node, time, 1);

        assert!(state.is_keyframe);
        assert!(state.keyframe_ref.is_none());
        assert_eq!(state.sequence, 1);
    }

    #[test]
    fn test_visual_state_delta() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(1000);
        let keyframe_id = VisualStateId::new(1);
        let state = VisualState::delta(node, time, 2, keyframe_id);

        assert!(!state.is_keyframe);
        assert_eq!(state.keyframe_ref, Some(keyframe_id));
        assert_eq!(state.sequence, 2);
    }

    #[test]
    fn test_visual_state_degradation() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(1000);
        let state = VisualState::keyframe(node, time, 1);

        let degraded = state.degrade(DegradationLevel::L3_SymbolicPresence);
        assert_eq!(degraded.degradation, DegradationLevel::L3_SymbolicPresence);
        assert!(degraded.scene.is_none());
        assert!(degraded.pose.is_none());
    }

    #[test]
    fn test_livestream_state() {
        let broadcaster = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let mut stream = LivestreamState::new(broadcaster, "Test Stream".to_string(), time);

        assert!(stream.is_live);
        assert_eq!(stream.viewer_count, 0);

        stream.add_viewer();
        stream.add_viewer();
        assert_eq!(stream.viewer_count, 2);

        stream.remove_viewer();
        assert_eq!(stream.viewer_count, 1);

        stream.end_stream();
        assert!(!stream.is_live);
    }
}
