//! Pose State - Body pose and gestures as state
//!
//! This is NOT motion capture data or skeleton tracking.
//! This is the STATE of body pose for reality synchronization.

use elara_core::StateTime;

/// Joint identifier for body skeleton
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Joint {
    // Head
    Head,
    Neck,

    // Torso
    Spine,
    Chest,
    Hips,

    // Left arm
    LeftShoulder,
    LeftElbow,
    LeftWrist,
    LeftHand,

    // Right arm
    RightShoulder,
    RightElbow,
    RightWrist,
    RightHand,

    // Left leg
    LeftHip,
    LeftKnee,
    LeftAnkle,
    LeftFoot,

    // Right leg
    RightHip,
    RightKnee,
    RightAnkle,
    RightFoot,
}

impl Joint {
    /// All joints in order
    pub fn all() -> &'static [Joint] {
        &[
            Joint::Head,
            Joint::Neck,
            Joint::Spine,
            Joint::Chest,
            Joint::Hips,
            Joint::LeftShoulder,
            Joint::LeftElbow,
            Joint::LeftWrist,
            Joint::LeftHand,
            Joint::RightShoulder,
            Joint::RightElbow,
            Joint::RightWrist,
            Joint::RightHand,
            Joint::LeftHip,
            Joint::LeftKnee,
            Joint::LeftAnkle,
            Joint::LeftFoot,
            Joint::RightHip,
            Joint::RightKnee,
            Joint::RightAnkle,
            Joint::RightFoot,
        ]
    }

    /// Number of joints
    pub fn count() -> usize {
        21
    }
}

/// 3D position (normalized coordinates)
#[derive(Debug, Clone, Copy, Default)]
pub struct Position3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position3D {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    /// Linear interpolation
    pub fn lerp(&self, other: &Position3D, t: f32) -> Position3D {
        Position3D {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    /// Distance to another position
    pub fn distance(&self, other: &Position3D) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Joint rotation (quaternion representation)
#[derive(Debug, Clone, Copy)]
pub struct Rotation3D {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Rotation3D {
    fn default() -> Self {
        Self::identity()
    }
}

impl Rotation3D {
    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn from_euler(yaw: f32, pitch: f32, roll: f32) -> Self {
        let cy = (yaw * 0.5).cos();
        let sy = (yaw * 0.5).sin();
        let cp = (pitch * 0.5).cos();
        let sp = (pitch * 0.5).sin();
        let cr = (roll * 0.5).cos();
        let sr = (roll * 0.5).sin();

        Self {
            w: cr * cp * cy + sr * sp * sy,
            x: sr * cp * cy - cr * sp * sy,
            y: cr * sp * cy + sr * cp * sy,
            z: cr * cp * sy - sr * sp * cy,
        }
    }

    /// Spherical linear interpolation
    pub fn slerp(&self, other: &Rotation3D, t: f32) -> Rotation3D {
        let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;

        let other = if dot < 0.0 {
            dot = -dot;
            Rotation3D {
                w: -other.w,
                x: -other.x,
                y: -other.y,
                z: -other.z,
            }
        } else {
            *other
        };

        if dot > 0.9995 {
            // Linear interpolation for very close quaternions
            let result = Rotation3D {
                w: self.w + (other.w - self.w) * t,
                x: self.x + (other.x - self.x) * t,
                y: self.y + (other.y - self.y) * t,
                z: self.z + (other.z - self.z) * t,
            };
            return result.normalize();
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        Rotation3D {
            w: self.w * s0 + other.w * s1,
            x: self.x * s0 + other.x * s1,
            y: self.y * s0 + other.y * s1,
            z: self.z * s0 + other.z * s1,
        }
    }

    fn normalize(&self) -> Rotation3D {
        let len = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len < 0.0001 {
            return Rotation3D::identity();
        }
        Rotation3D {
            w: self.w / len,
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }
}

/// Joint state (position + rotation + confidence)
#[derive(Debug, Clone, Copy, Default)]
pub struct JointState {
    pub position: Position3D,
    pub rotation: Rotation3D,
    pub confidence: f32,
}

impl JointState {
    pub fn new(position: Position3D, rotation: Rotation3D, confidence: f32) -> Self {
        Self {
            position,
            rotation,
            confidence,
        }
    }

    pub fn lerp(&self, other: &JointState, t: f32) -> JointState {
        JointState {
            position: self.position.lerp(&other.position, t),
            rotation: self.rotation.slerp(&other.rotation, t),
            confidence: self.confidence + (other.confidence - self.confidence) * t,
        }
    }
}

/// Gesture recognition result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    None,
    Wave,
    ThumbsUp,
    ThumbsDown,
    Peace,
    Pointing,
    OpenPalm,
    Fist,
    Clapping,
    Shrugging,
    Nodding,
    HeadShake,
}

/// Activity state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityState {
    Unknown,
    Sitting,
    Standing,
    Walking,
    Running,
    Jumping,
    Lying,
}

/// Complete pose state
#[derive(Debug, Clone)]
pub struct PoseState {
    /// Timestamp
    pub timestamp: StateTime,

    /// Is a body detected?
    pub present: bool,

    /// Joint states (indexed by Joint enum)
    pub joints: Vec<JointState>,

    /// Current gesture (if any)
    pub gesture: Gesture,

    /// Current activity
    pub activity: ActivityState,

    /// Overall confidence
    pub confidence: f32,

    /// Velocity estimate (for prediction)
    pub velocity: Position3D,
}

impl PoseState {
    /// Create a new pose state
    pub fn new(timestamp: StateTime) -> Self {
        Self {
            timestamp,
            present: true,
            joints: vec![JointState::default(); Joint::count()],
            gesture: Gesture::None,
            activity: ActivityState::Unknown,
            confidence: 1.0,
            velocity: Position3D::zero(),
        }
    }

    /// No body present
    pub fn absent(timestamp: StateTime) -> Self {
        Self {
            timestamp,
            present: false,
            joints: Vec::new(),
            gesture: Gesture::None,
            activity: ActivityState::Unknown,
            confidence: 0.0,
            velocity: Position3D::zero(),
        }
    }

    /// Get joint state by joint type
    pub fn joint(&self, joint: Joint) -> Option<&JointState> {
        self.joints.get(joint as usize)
    }

    /// Set joint state
    pub fn set_joint(&mut self, joint: Joint, state: JointState) {
        let idx = joint as usize;
        if idx < self.joints.len() {
            self.joints[idx] = state;
        }
    }

    /// Interpolate between two pose states
    pub fn lerp(&self, other: &PoseState, t: f32) -> PoseState {
        let t = t.clamp(0.0, 1.0);

        let joints = if self.joints.len() == other.joints.len() {
            self.joints
                .iter()
                .zip(other.joints.iter())
                .map(|(a, b)| a.lerp(b, t))
                .collect()
        } else if t < 0.5 {
            self.joints.clone()
        } else {
            other.joints.clone()
        };

        PoseState {
            timestamp: other.timestamp,
            present: if t < 0.5 { self.present } else { other.present },
            joints,
            gesture: if t < 0.5 { self.gesture } else { other.gesture },
            activity: if t < 0.5 {
                self.activity
            } else {
                other.activity
            },
            confidence: self.confidence + (other.confidence - self.confidence) * t,
            velocity: self.velocity.lerp(&other.velocity, t),
        }
    }

    /// Predict future pose based on velocity
    pub fn predict(&self, delta_ms: i64) -> PoseState {
        let dt = delta_ms as f32 / 1000.0;
        let mut predicted = self.clone();

        predicted.timestamp = StateTime::from_millis(self.timestamp.as_millis() + delta_ms);

        // Apply velocity to all joints
        for joint in &mut predicted.joints {
            joint.position.x += self.velocity.x * dt;
            joint.position.y += self.velocity.y * dt;
            joint.position.z += self.velocity.z * dt;
        }

        // Reduce confidence for predictions
        predicted.confidence *= 0.95;

        predicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_lerp() {
        let a = Position3D::new(0.0, 0.0, 0.0);
        let b = Position3D::new(10.0, 10.0, 10.0);

        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 5.0).abs() < 0.01);
        assert!((mid.y - 5.0).abs() < 0.01);
        assert!((mid.z - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_pose_state() {
        let time = StateTime::from_millis(0);
        let pose = PoseState::new(time);

        assert!(pose.present);
        assert_eq!(pose.joints.len(), Joint::count());
    }

    #[test]
    fn test_pose_prediction() {
        let time = StateTime::from_millis(0);
        let mut pose = PoseState::new(time);
        pose.velocity = Position3D::new(1.0, 0.0, 0.0);

        let predicted = pose.predict(1000); // 1 second

        // Joints should have moved by velocity * time
        assert!(predicted.timestamp.as_millis() == 1000);
    }
}
