//! Keyframe and Delta Encoding - ELARA-native visual compression
//!
//! This is NOT H.264/VP8/AV1. This is state-based keyframe/delta encoding.
//! We encode the CHANGE in visual state, not pixel differences.

use elara_core::StateTime;

use crate::{FaceState, PoseState, SceneState, VisualState, VisualStateId};

/// Keyframe - Complete visual state snapshot
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Keyframe identifier
    pub id: VisualStateId,

    /// Timestamp
    pub timestamp: StateTime,

    /// Complete visual state
    pub state: VisualState,

    /// Keyframe interval (how often keyframes are sent)
    pub interval_ms: u32,

    /// Sequence number
    pub sequence: u64,
}

impl Keyframe {
    /// Create a new keyframe
    pub fn new(state: VisualState, interval_ms: u32) -> Self {
        Self {
            id: state.id,
            timestamp: state.timestamp,
            sequence: state.sequence,
            state,
            interval_ms,
        }
    }

    /// Estimated size in bytes (for bandwidth calculation)
    pub fn estimated_size(&self) -> usize {
        let mut size = 32; // Base header

        if self.state.face.is_some() {
            size += 128; // Face state
        }
        if self.state.pose.is_some() {
            size += 256; // Pose state (21 joints)
        }
        if self.state.scene.is_some() {
            size += 64; // Scene state
        }

        size
    }
}

/// Delta type - what changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaType {
    /// No change
    None,
    /// Face state changed
    Face,
    /// Pose state changed
    Pose,
    /// Scene state changed
    Scene,
    /// Multiple things changed
    Multiple,
}

/// Delta - Changes since last keyframe or delta
#[derive(Debug, Clone)]
pub struct Delta {
    /// Delta identifier
    pub id: VisualStateId,

    /// Reference to keyframe this delta is based on
    pub keyframe_ref: VisualStateId,

    /// Reference to previous delta (if any)
    pub prev_delta_ref: Option<VisualStateId>,

    /// Timestamp
    pub timestamp: StateTime,

    /// What changed
    pub delta_type: DeltaType,

    /// Face delta (if changed)
    pub face_delta: Option<FaceDelta>,

    /// Pose delta (if changed)
    pub pose_delta: Option<PoseDelta>,

    /// Scene delta (if changed)
    pub scene_delta: Option<SceneDelta>,

    /// Sequence number
    pub sequence: u64,
}

impl Delta {
    /// Create a delta from two visual states
    pub fn from_states(
        prev: &VisualState,
        curr: &VisualState,
        keyframe_ref: VisualStateId,
    ) -> Self {
        let face_delta = match (&prev.face, &curr.face) {
            (Some(prev_face), Some(curr_face)) => FaceDelta::compute(prev_face, curr_face),
            (None, Some(curr_face)) => Some(FaceDelta::full(curr_face.clone())),
            (Some(_), None) => Some(FaceDelta::removed()),
            (None, None) => None,
        };

        let pose_delta = match (&prev.pose, &curr.pose) {
            (Some(prev_pose), Some(curr_pose)) => PoseDelta::compute(prev_pose, curr_pose),
            (None, Some(curr_pose)) => Some(PoseDelta::full(curr_pose.clone())),
            (Some(_), None) => Some(PoseDelta::removed()),
            (None, None) => None,
        };

        let scene_delta = match (&prev.scene, &curr.scene) {
            (Some(prev_scene), Some(curr_scene)) => SceneDelta::compute(prev_scene, curr_scene),
            (None, Some(curr_scene)) => Some(SceneDelta::full(curr_scene.clone())),
            (Some(_), None) => Some(SceneDelta::removed()),
            (None, None) => None,
        };

        let delta_type = match (
            face_delta.is_some(),
            pose_delta.is_some(),
            scene_delta.is_some(),
        ) {
            (false, false, false) => DeltaType::None,
            (true, false, false) => DeltaType::Face,
            (false, true, false) => DeltaType::Pose,
            (false, false, true) => DeltaType::Scene,
            _ => DeltaType::Multiple,
        };

        Self {
            id: curr.id,
            keyframe_ref,
            prev_delta_ref: Some(prev.id),
            timestamp: curr.timestamp,
            delta_type,
            face_delta,
            pose_delta,
            scene_delta,
            sequence: curr.sequence,
        }
    }

    /// Is this an empty delta (no changes)?
    pub fn is_empty(&self) -> bool {
        self.delta_type == DeltaType::None
    }

    /// Estimated size in bytes
    pub fn estimated_size(&self) -> usize {
        let mut size = 24; // Base header

        if let Some(ref fd) = self.face_delta {
            size += fd.estimated_size();
        }
        if let Some(ref pd) = self.pose_delta {
            size += pd.estimated_size();
        }
        if let Some(ref sd) = self.scene_delta {
            size += sd.estimated_size();
        }

        size
    }

    /// Apply this delta to a visual state
    pub fn apply(&self, base: &VisualState) -> VisualState {
        let mut result = base.clone();
        result.id = self.id;
        result.timestamp = self.timestamp;
        result.sequence = self.sequence;
        result.is_keyframe = false;
        result.keyframe_ref = Some(self.keyframe_ref);

        if let Some(ref fd) = self.face_delta {
            result.face = fd.apply(base.face.as_ref());
        }
        if let Some(ref pd) = self.pose_delta {
            result.pose = pd.apply(base.pose.as_ref());
        }
        if let Some(ref sd) = self.scene_delta {
            result.scene = sd.apply(base.scene.as_ref());
        }

        result
    }
}

/// Face delta - changes in face state
#[derive(Debug, Clone)]
pub enum FaceDelta {
    /// No face anymore
    Removed,
    /// Full face state (new face appeared)
    Full(FaceState),
    /// Partial update
    Partial {
        /// Head rotation change (if significant)
        head_rotation: Option<(f32, f32, f32)>,
        /// Emotion change (if significant)
        emotion_change: Option<(String, f32)>, // (emotion_name, new_value)
        /// Mouth openness change
        mouth_openness: Option<f32>,
        /// Speaking state change
        speaking: Option<bool>,
        /// Gaze change
        gaze_yaw: Option<f32>,
        gaze_pitch: Option<f32>,
    },
}

impl FaceDelta {
    /// Compute delta between two face states
    pub fn compute(prev: &FaceState, curr: &FaceState) -> Option<FaceDelta> {
        // Check if there are significant changes
        let head_changed = (prev.head_rotation.0 - curr.head_rotation.0).abs() > 0.05
            || (prev.head_rotation.1 - curr.head_rotation.1).abs() > 0.05
            || (prev.head_rotation.2 - curr.head_rotation.2).abs() > 0.05;

        let mouth_changed = (prev.mouth.openness - curr.mouth.openness).abs() > 0.1;
        let speaking_changed = prev.speaking != curr.speaking;
        let gaze_changed = (prev.gaze.yaw - curr.gaze.yaw).abs() > 0.1
            || (prev.gaze.pitch - curr.gaze.pitch).abs() > 0.1;

        // Check emotion changes
        let prev_dom = prev.emotion.dominant();
        let curr_dom = curr.emotion.dominant();
        let emotion_changed = prev_dom.0 != curr_dom.0 || (prev_dom.1 - curr_dom.1).abs() > 0.2;

        if !head_changed && !mouth_changed && !speaking_changed && !gaze_changed && !emotion_changed
        {
            return None;
        }

        Some(FaceDelta::Partial {
            head_rotation: if head_changed {
                Some(curr.head_rotation)
            } else {
                None
            },
            emotion_change: if emotion_changed {
                Some((curr_dom.0.to_string(), curr_dom.1))
            } else {
                None
            },
            mouth_openness: if mouth_changed {
                Some(curr.mouth.openness)
            } else {
                None
            },
            speaking: if speaking_changed {
                Some(curr.speaking)
            } else {
                None
            },
            gaze_yaw: if gaze_changed {
                Some(curr.gaze.yaw)
            } else {
                None
            },
            gaze_pitch: if gaze_changed {
                Some(curr.gaze.pitch)
            } else {
                None
            },
        })
    }

    /// Full face state
    pub fn full(face: FaceState) -> FaceDelta {
        FaceDelta::Full(face)
    }

    /// Face removed
    pub fn removed() -> FaceDelta {
        FaceDelta::Removed
    }

    /// Estimated size
    pub fn estimated_size(&self) -> usize {
        match self {
            FaceDelta::Removed => 1,
            FaceDelta::Full(_) => 128,
            FaceDelta::Partial { .. } => 32,
        }
    }

    /// Apply delta to base face state
    pub fn apply(&self, base: Option<&FaceState>) -> Option<FaceState> {
        match self {
            FaceDelta::Removed => None,
            FaceDelta::Full(face) => Some(face.clone()),
            FaceDelta::Partial {
                head_rotation,
                emotion_change,
                mouth_openness,
                speaking,
                gaze_yaw,
                gaze_pitch,
            } => {
                let mut face = base?.clone();

                if let Some(rot) = head_rotation {
                    face.head_rotation = *rot;
                }
                if let Some((_, val)) = emotion_change {
                    // Simplified: just update joy for now
                    face.emotion.joy = *val;
                }
                if let Some(openness) = mouth_openness {
                    face.mouth.openness = *openness;
                }
                if let Some(spk) = speaking {
                    face.speaking = *spk;
                }
                if let Some(yaw) = gaze_yaw {
                    face.gaze.yaw = *yaw;
                }
                if let Some(pitch) = gaze_pitch {
                    face.gaze.pitch = *pitch;
                }

                Some(face)
            }
        }
    }
}

/// Pose delta - changes in pose state
#[derive(Debug, Clone)]
pub enum PoseDelta {
    /// No pose anymore
    Removed,
    /// Full pose state
    Full(PoseState),
    /// Partial update - only changed joints
    Partial {
        /// Changed joint indices and their new states
        changed_joints: Vec<(usize, crate::JointState)>,
        /// Gesture change
        gesture: Option<crate::Gesture>,
        /// Activity change
        activity: Option<crate::ActivityState>,
    },
}

impl PoseDelta {
    /// Compute delta between two pose states
    pub fn compute(prev: &PoseState, curr: &PoseState) -> Option<PoseDelta> {
        let mut changed_joints = Vec::new();

        // Find changed joints
        for (i, (prev_joint, curr_joint)) in prev.joints.iter().zip(curr.joints.iter()).enumerate()
        {
            let pos_changed = prev_joint.position.distance(&curr_joint.position) > 0.01;
            if pos_changed {
                changed_joints.push((i, *curr_joint));
            }
        }

        let gesture_changed = prev.gesture != curr.gesture;
        let activity_changed = prev.activity != curr.activity;

        if changed_joints.is_empty() && !gesture_changed && !activity_changed {
            return None;
        }

        Some(PoseDelta::Partial {
            changed_joints,
            gesture: if gesture_changed {
                Some(curr.gesture)
            } else {
                None
            },
            activity: if activity_changed {
                Some(curr.activity)
            } else {
                None
            },
        })
    }

    pub fn full(pose: PoseState) -> PoseDelta {
        PoseDelta::Full(pose)
    }

    pub fn removed() -> PoseDelta {
        PoseDelta::Removed
    }

    pub fn estimated_size(&self) -> usize {
        match self {
            PoseDelta::Removed => 1,
            PoseDelta::Full(_) => 256,
            PoseDelta::Partial { changed_joints, .. } => 8 + changed_joints.len() * 16,
        }
    }

    pub fn apply(&self, base: Option<&PoseState>) -> Option<PoseState> {
        match self {
            PoseDelta::Removed => None,
            PoseDelta::Full(pose) => Some(pose.clone()),
            PoseDelta::Partial {
                changed_joints,
                gesture,
                activity,
            } => {
                let mut pose = base?.clone();

                for (idx, joint_state) in changed_joints {
                    if *idx < pose.joints.len() {
                        pose.joints[*idx] = *joint_state;
                    }
                }

                if let Some(g) = gesture {
                    pose.gesture = *g;
                }
                if let Some(a) = activity {
                    pose.activity = *a;
                }

                Some(pose)
            }
        }
    }
}

/// Scene delta - changes in scene state
#[derive(Debug, Clone)]
pub enum SceneDelta {
    /// No scene anymore
    Removed,
    /// Full scene state
    Full(SceneState),
    /// Partial update
    Partial {
        /// Background color change
        background_color: Option<crate::Color>,
        /// Lighting change
        lighting: Option<crate::LightingCondition>,
        /// Detail level change
        detail_level: Option<f32>,
    },
}

impl SceneDelta {
    pub fn compute(prev: &SceneState, curr: &SceneState) -> Option<SceneDelta> {
        let color_changed = (prev.background_color.r - curr.background_color.r).abs() > 0.1
            || (prev.background_color.g - curr.background_color.g).abs() > 0.1
            || (prev.background_color.b - curr.background_color.b).abs() > 0.1;

        let lighting_changed = prev.lighting != curr.lighting;
        let detail_changed = (prev.detail_level - curr.detail_level).abs() > 0.1;

        if !color_changed && !lighting_changed && !detail_changed {
            return None;
        }

        Some(SceneDelta::Partial {
            background_color: if color_changed {
                Some(curr.background_color)
            } else {
                None
            },
            lighting: if lighting_changed {
                Some(curr.lighting)
            } else {
                None
            },
            detail_level: if detail_changed {
                Some(curr.detail_level)
            } else {
                None
            },
        })
    }

    pub fn full(scene: SceneState) -> SceneDelta {
        SceneDelta::Full(scene)
    }

    pub fn removed() -> SceneDelta {
        SceneDelta::Removed
    }

    pub fn estimated_size(&self) -> usize {
        match self {
            SceneDelta::Removed => 1,
            SceneDelta::Full(_) => 64,
            SceneDelta::Partial { .. } => 16,
        }
    }

    pub fn apply(&self, base: Option<&SceneState>) -> Option<SceneState> {
        match self {
            SceneDelta::Removed => None,
            SceneDelta::Full(scene) => Some(scene.clone()),
            SceneDelta::Partial {
                background_color,
                lighting,
                detail_level,
            } => {
                let mut scene = base?.clone();

                if let Some(color) = background_color {
                    scene.background_color = *color;
                }
                if let Some(light) = lighting {
                    scene.lighting = *light;
                }
                if let Some(detail) = detail_level {
                    scene.detail_level = *detail;
                }

                Some(scene)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elara_core::NodeId;

    #[test]
    fn test_keyframe_creation() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let state = VisualState::keyframe(node, time, 1);
        let keyframe = Keyframe::new(state, 1000);

        assert_eq!(keyframe.interval_ms, 1000);
        assert!(keyframe.estimated_size() > 0);
    }

    #[test]
    fn test_delta_computation() {
        let node = NodeId::new(1);
        let time1 = StateTime::from_millis(0);
        let time2 = StateTime::from_millis(100);

        let state1 = VisualState::keyframe(node, time1, 1);
        let state2 = VisualState::keyframe(node, time2, 2);

        let delta = Delta::from_states(&state1, &state2, state1.id);

        // Both states have no face/pose/scene, so delta should be empty
        assert!(delta.is_empty());
    }
}
