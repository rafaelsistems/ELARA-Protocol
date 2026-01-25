//! Visual State Encoding - Wire format for visual state
//!
//! This is NOT H.264/VP8/AV1. This is ELARA-native state encoding.
//! We encode semantic visual state, not pixel data.

use crate::{
    BackgroundComplexity, Color, EmotionVector, EnvironmentType, FaceState, GazeState, JointState,
    LightingCondition, MouthState, PoseState, Position3D, Rotation3D, SceneState, Viseme,
    VisualState, VisualStateId,
};
use elara_core::{DegradationLevel, NodeId, StateTime};

/// Encoding error
#[derive(Debug, Clone)]
pub enum EncodingError {
    BufferTooSmall,
    InvalidData,
    UnsupportedVersion,
}

/// Visual state encoder
pub struct VisualEncoder;

impl VisualEncoder {
    /// Encode a visual state to bytes
    pub fn encode(state: &VisualState) -> Vec<u8> {
        let mut buf = Vec::with_capacity(512);

        // Header
        buf.push(0x01); // Version
        buf.push(if state.is_keyframe { 0x01 } else { 0x00 });
        buf.push(state.degradation.level());
        buf.push(0x00); // Reserved

        // State ID (8 bytes)
        buf.extend_from_slice(&state.id.0.to_le_bytes());

        // Source node ID (8 bytes)
        buf.extend_from_slice(&state.source.0.to_le_bytes());

        // Timestamp (8 bytes)
        buf.extend_from_slice(&state.timestamp.as_millis().to_le_bytes());

        // Sequence (8 bytes)
        buf.extend_from_slice(&state.sequence.to_le_bytes());

        // Keyframe reference (8 bytes, 0 if none)
        let keyframe_ref = state.keyframe_ref.map(|k| k.0).unwrap_or(0);
        buf.extend_from_slice(&keyframe_ref.to_le_bytes());

        // Flags for what's present
        let mut flags: u8 = 0;
        if state.face.is_some() {
            flags |= 0x01;
        }
        if state.pose.is_some() {
            flags |= 0x02;
        }
        if state.scene.is_some() {
            flags |= 0x04;
        }
        buf.push(flags);

        // Encode face if present
        if let Some(ref face) = state.face {
            Self::encode_face(face, &mut buf);
        }

        // Encode pose if present
        if let Some(ref pose) = state.pose {
            Self::encode_pose(pose, &mut buf);
        }

        // Encode scene if present
        if let Some(ref scene) = state.scene {
            Self::encode_scene(scene, &mut buf);
        }

        buf
    }

    fn encode_face(face: &FaceState, buf: &mut Vec<u8>) {
        // Face flags
        let mut flags: u8 = 0;
        if face.present {
            flags |= 0x01;
        }
        if face.speaking {
            flags |= 0x02;
        }
        buf.push(flags);

        // Confidence
        buf.extend_from_slice(&face.confidence.to_le_bytes());

        // Head rotation (3 x f32 = 12 bytes)
        buf.extend_from_slice(&face.head_rotation.0.to_le_bytes());
        buf.extend_from_slice(&face.head_rotation.1.to_le_bytes());
        buf.extend_from_slice(&face.head_rotation.2.to_le_bytes());

        // Emotion vector (7 x f32 = 28 bytes)
        buf.extend_from_slice(&face.emotion.joy.to_le_bytes());
        buf.extend_from_slice(&face.emotion.sadness.to_le_bytes());
        buf.extend_from_slice(&face.emotion.anger.to_le_bytes());
        buf.extend_from_slice(&face.emotion.fear.to_le_bytes());
        buf.extend_from_slice(&face.emotion.surprise.to_le_bytes());
        buf.extend_from_slice(&face.emotion.disgust.to_le_bytes());
        buf.extend_from_slice(&face.emotion.contempt.to_le_bytes());

        // Gaze (4 x f32 + 1 bool = 17 bytes)
        buf.extend_from_slice(&face.gaze.yaw.to_le_bytes());
        buf.extend_from_slice(&face.gaze.pitch.to_le_bytes());
        buf.push(if face.gaze.looking_at_camera { 1 } else { 0 });
        buf.extend_from_slice(&face.gaze.blink.to_le_bytes());

        // Mouth (2 x f32 + 1 viseme = 9 bytes)
        buf.extend_from_slice(&face.mouth.openness.to_le_bytes());
        buf.extend_from_slice(&face.mouth.smile.to_le_bytes());
        buf.push(face.mouth.viseme as u8);
    }

    fn encode_pose(pose: &PoseState, buf: &mut Vec<u8>) {
        // Pose flags
        let mut flags: u8 = 0;
        if pose.present {
            flags |= 0x01;
        }
        buf.push(flags);

        // Confidence
        buf.extend_from_slice(&pose.confidence.to_le_bytes());

        // Gesture and activity
        buf.push(pose.gesture as u8);
        buf.push(pose.activity as u8);

        // Velocity
        buf.extend_from_slice(&pose.velocity.x.to_le_bytes());
        buf.extend_from_slice(&pose.velocity.y.to_le_bytes());
        buf.extend_from_slice(&pose.velocity.z.to_le_bytes());

        // Number of joints
        buf.push(pose.joints.len() as u8);

        // Encode each joint (position + rotation + confidence = 32 bytes each)
        for joint in &pose.joints {
            buf.extend_from_slice(&joint.position.x.to_le_bytes());
            buf.extend_from_slice(&joint.position.y.to_le_bytes());
            buf.extend_from_slice(&joint.position.z.to_le_bytes());
            buf.extend_from_slice(&joint.rotation.w.to_le_bytes());
            buf.extend_from_slice(&joint.rotation.x.to_le_bytes());
            buf.extend_from_slice(&joint.rotation.y.to_le_bytes());
            buf.extend_from_slice(&joint.rotation.z.to_le_bytes());
            buf.extend_from_slice(&joint.confidence.to_le_bytes());
        }
    }

    fn encode_scene(scene: &SceneState, buf: &mut Vec<u8>) {
        // Background color (3 x f32 = 12 bytes)
        buf.extend_from_slice(&scene.background_color.r.to_le_bytes());
        buf.extend_from_slice(&scene.background_color.g.to_le_bytes());
        buf.extend_from_slice(&scene.background_color.b.to_le_bytes());

        // Lighting, environment, complexity
        buf.push(scene.lighting as u8);
        buf.push(scene.environment as u8);
        buf.push(scene.complexity as u8);

        // Flags
        let mut flags: u8 = 0;
        if scene.background_motion {
            flags |= 0x01;
        }
        buf.push(flags);

        // Blur, noise, detail
        buf.extend_from_slice(&scene.blur.to_le_bytes());
        buf.extend_from_slice(&scene.noise.to_le_bytes());
        buf.extend_from_slice(&scene.detail_level.to_le_bytes());

        // Objects count (simplified - just count for now)
        buf.push(scene.objects.len().min(255) as u8);
    }

    /// Decode a visual state from bytes
    pub fn decode(data: &[u8]) -> Result<VisualState, EncodingError> {
        if data.len() < 41 {
            return Err(EncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        // Header
        let version = data[pos];
        pos += 1;
        if version != 0x01 {
            return Err(EncodingError::UnsupportedVersion);
        }

        let is_keyframe = data[pos] == 0x01;
        pos += 1;
        let degradation_level = data[pos];
        pos += 1;
        pos += 1; // Reserved

        // State ID
        let id = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        // Source node ID
        let source = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        // Timestamp
        let timestamp = i64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        // Sequence
        let sequence = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        // Keyframe reference
        let keyframe_ref_val = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let keyframe_ref = if keyframe_ref_val == 0 {
            None
        } else {
            Some(VisualStateId(keyframe_ref_val))
        };

        // Flags
        let flags = data[pos];
        pos += 1;
        let has_face = flags & 0x01 != 0;
        let has_pose = flags & 0x02 != 0;
        let has_scene = flags & 0x04 != 0;

        // Decode face
        let face = if has_face {
            Some(Self::decode_face(&data[pos..])?)
        } else {
            None
        };
        if has_face {
            pos += 75;
        } // Face size

        // Decode pose (variable size based on joints)
        let pose = if has_pose {
            let (p, size) = Self::decode_pose(&data[pos..])?;
            pos += size;
            Some(p)
        } else {
            None
        };

        // Decode scene
        let scene = if has_scene {
            Some(Self::decode_scene(&data[pos..])?)
        } else {
            None
        };

        let degradation = match degradation_level {
            0 => DegradationLevel::L0_FullPerception,
            1 => DegradationLevel::L1_DistortedPerception,
            2 => DegradationLevel::L2_FragmentedPerception,
            3 => DegradationLevel::L3_SymbolicPresence,
            4 => DegradationLevel::L4_MinimalPresence,
            _ => DegradationLevel::L5_LatentPresence,
        };

        Ok(VisualState {
            id: VisualStateId(id),
            source: NodeId::new(source),
            timestamp: StateTime::from_millis(timestamp),
            face,
            pose,
            scene,
            degradation,
            is_keyframe,
            keyframe_ref,
            sequence,
        })
    }

    fn decode_face(data: &[u8]) -> Result<FaceState, EncodingError> {
        if data.len() < 75 {
            return Err(EncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        let flags = data[pos];
        pos += 1;
        let present = flags & 0x01 != 0;
        let speaking = flags & 0x02 != 0;

        let confidence = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        let head_rotation = (
            f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
            f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
            f32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
        );
        pos += 12;

        let emotion = EmotionVector {
            joy: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
            sadness: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
            anger: f32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
            fear: f32::from_le_bytes(data[pos + 12..pos + 16].try_into().unwrap()),
            surprise: f32::from_le_bytes(data[pos + 16..pos + 20].try_into().unwrap()),
            disgust: f32::from_le_bytes(data[pos + 20..pos + 24].try_into().unwrap()),
            contempt: f32::from_le_bytes(data[pos + 24..pos + 28].try_into().unwrap()),
        };
        pos += 28;

        let gaze = GazeState {
            yaw: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
            pitch: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
            looking_at_camera: data[pos + 8] != 0,
            blink: f32::from_le_bytes(data[pos + 9..pos + 13].try_into().unwrap()),
        };
        pos += 13;

        let mouth = MouthState {
            openness: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
            smile: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
            viseme: Viseme::Neutral, // Simplified
        };

        Ok(FaceState {
            timestamp: StateTime::from_millis(0), // Will be set from parent
            present,
            head_rotation,
            emotion,
            gaze,
            mouth,
            speaking,
            confidence,
        })
    }

    fn decode_pose(data: &[u8]) -> Result<(PoseState, usize), EncodingError> {
        if data.len() < 20 {
            return Err(EncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        let flags = data[pos];
        pos += 1;
        let present = flags & 0x01 != 0;

        let confidence = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        let gesture = crate::Gesture::None; // Simplified
        let activity = crate::ActivityState::Unknown; // Simplified
        pos += 2;

        let velocity = Position3D {
            x: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
            y: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
            z: f32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
        };
        pos += 12;

        let num_joints = data[pos] as usize;
        pos += 1;

        let mut joints = Vec::with_capacity(num_joints);
        for _ in 0..num_joints {
            if pos + 32 > data.len() {
                return Err(EncodingError::BufferTooSmall);
            }

            let joint = JointState {
                position: Position3D {
                    x: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
                    y: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
                    z: f32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
                },
                rotation: Rotation3D {
                    w: f32::from_le_bytes(data[pos + 12..pos + 16].try_into().unwrap()),
                    x: f32::from_le_bytes(data[pos + 16..pos + 20].try_into().unwrap()),
                    y: f32::from_le_bytes(data[pos + 20..pos + 24].try_into().unwrap()),
                    z: f32::from_le_bytes(data[pos + 24..pos + 28].try_into().unwrap()),
                },
                confidence: f32::from_le_bytes(data[pos + 28..pos + 32].try_into().unwrap()),
            };
            joints.push(joint);
            pos += 32;
        }

        Ok((
            PoseState {
                timestamp: StateTime::from_millis(0),
                present,
                joints,
                gesture,
                activity,
                confidence,
                velocity,
            },
            pos,
        ))
    }

    fn decode_scene(data: &[u8]) -> Result<SceneState, EncodingError> {
        if data.len() < 25 {
            return Err(EncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        let background_color = Color {
            r: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
            g: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
            b: f32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
        };
        pos += 12;

        let lighting = LightingCondition::Normal; // Simplified
        let environment = EnvironmentType::Unknown; // Simplified
        let complexity = BackgroundComplexity::Simple; // Simplified
        pos += 3;

        let flags = data[pos];
        pos += 1;
        let background_motion = flags & 0x01 != 0;

        let blur = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let noise = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let detail_level = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());

        Ok(SceneState {
            timestamp: StateTime::from_millis(0),
            background_color,
            lighting,
            environment,
            complexity,
            objects: Vec::new(),
            background_motion,
            blur,
            noise,
            detail_level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let node = NodeId::new(12345);
        let time = StateTime::from_millis(1000);
        let state = VisualState::keyframe(node, time, 1);

        let encoded = VisualEncoder::encode(&state);
        let decoded = VisualEncoder::decode(&encoded).unwrap();

        assert_eq!(decoded.id.0, state.id.0);
        assert_eq!(decoded.source.0, state.source.0);
        assert_eq!(decoded.sequence, state.sequence);
        assert_eq!(decoded.is_keyframe, state.is_keyframe);
    }

    #[test]
    fn test_encode_with_face() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let face = FaceState::new(time);
        let state = VisualState::keyframe(node, time, 1).with_face(face);

        let encoded = VisualEncoder::encode(&state);
        // Just verify encoding works and produces reasonable size
        assert!(encoded.len() > 41); // Header + face data
    }
}
