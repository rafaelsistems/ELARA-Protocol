//! Face State - Facial expression and gaze as state
//!
//! This is NOT facial recognition or tracking data.
//! This is the STATE of facial expression for reality synchronization.

use elara_core::StateTime;

/// Face landmark indices (simplified 68-point model concept)
/// We don't store raw coordinates - we store SEMANTIC state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacialRegion {
    LeftEye,
    RightEye,
    LeftEyebrow,
    RightEyebrow,
    Nose,
    UpperLip,
    LowerLip,
    LeftCheek,
    RightCheek,
    Jaw,
    Forehead,
}

/// Emotion vector - continuous blend of basic emotions
#[derive(Debug, Clone, Copy, Default)]
pub struct EmotionVector {
    /// Joy/happiness [0.0 - 1.0]
    pub joy: f32,
    /// Sadness [0.0 - 1.0]
    pub sadness: f32,
    /// Anger [0.0 - 1.0]
    pub anger: f32,
    /// Fear [0.0 - 1.0]
    pub fear: f32,
    /// Surprise [0.0 - 1.0]
    pub surprise: f32,
    /// Disgust [0.0 - 1.0]
    pub disgust: f32,
    /// Contempt [0.0 - 1.0]
    pub contempt: f32,
}

impl EmotionVector {
    /// Neutral expression
    pub fn neutral() -> Self {
        Self::default()
    }
    
    /// Dominant emotion
    pub fn dominant(&self) -> (&'static str, f32) {
        let emotions = [
            ("joy", self.joy),
            ("sadness", self.sadness),
            ("anger", self.anger),
            ("fear", self.fear),
            ("surprise", self.surprise),
            ("disgust", self.disgust),
            ("contempt", self.contempt),
        ];
        
        emotions.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(name, val)| (*name, *val))
            .unwrap_or(("neutral", 0.0))
    }
    
    /// Blend with another emotion vector
    pub fn blend(&self, other: &EmotionVector, factor: f32) -> EmotionVector {
        let f = factor.clamp(0.0, 1.0);
        let inv = 1.0 - f;
        
        EmotionVector {
            joy: self.joy * inv + other.joy * f,
            sadness: self.sadness * inv + other.sadness * f,
            anger: self.anger * inv + other.anger * f,
            fear: self.fear * inv + other.fear * f,
            surprise: self.surprise * inv + other.surprise * f,
            disgust: self.disgust * inv + other.disgust * f,
            contempt: self.contempt * inv + other.contempt * f,
        }
    }
    
    /// Normalize so all values sum to 1.0
    pub fn normalize(&self) -> EmotionVector {
        let sum = self.joy + self.sadness + self.anger + self.fear 
                + self.surprise + self.disgust + self.contempt;
        
        if sum < 0.001 {
            return EmotionVector::neutral();
        }
        
        EmotionVector {
            joy: self.joy / sum,
            sadness: self.sadness / sum,
            anger: self.anger / sum,
            fear: self.fear / sum,
            surprise: self.surprise / sum,
            disgust: self.disgust / sum,
            contempt: self.contempt / sum,
        }
    }
}

/// Gaze direction state
#[derive(Debug, Clone, Copy, Default)]
pub struct GazeState {
    /// Horizontal angle in radians (-π to π, 0 = forward)
    pub yaw: f32,
    /// Vertical angle in radians (-π/2 to π/2, 0 = forward)
    pub pitch: f32,
    /// Is the person looking at the camera/screen?
    pub looking_at_camera: bool,
    /// Blink state (0.0 = open, 1.0 = closed)
    pub blink: f32,
}

impl GazeState {
    /// Looking straight ahead
    pub fn forward() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            looking_at_camera: true,
            blink: 0.0,
        }
    }
    
    /// Interpolate between two gaze states
    pub fn lerp(&self, other: &GazeState, t: f32) -> GazeState {
        let t = t.clamp(0.0, 1.0);
        GazeState {
            yaw: self.yaw + (other.yaw - self.yaw) * t,
            pitch: self.pitch + (other.pitch - self.pitch) * t,
            looking_at_camera: if t < 0.5 { self.looking_at_camera } else { other.looking_at_camera },
            blink: self.blink + (other.blink - self.blink) * t,
        }
    }
}

/// Mouth state for speech visualization
#[derive(Debug, Clone, Copy, Default)]
pub struct MouthState {
    /// Mouth openness (0.0 = closed, 1.0 = fully open)
    pub openness: f32,
    /// Smile amount (-1.0 = frown, 0.0 = neutral, 1.0 = smile)
    pub smile: f32,
    /// Current viseme (mouth shape for speech)
    pub viseme: Viseme,
}

/// Viseme - mouth shapes for speech
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Viseme {
    #[default]
    Neutral,    // Closed/neutral
    AA,         // "ah" as in "father"
    AO,         // "aw" as in "bought"
    EH,         // "eh" as in "bed"
    IY,         // "ee" as in "see"
    UW,         // "oo" as in "boot"
    OW,         // "oh" as in "boat"
    AE,         // "a" as in "cat"
    AW,         // "ow" as in "cow"
    EY,         // "ay" as in "say"
    ER,         // "er" as in "bird"
    PP,         // "p", "b", "m" (lips together)
    FF,         // "f", "v" (teeth on lip)
    TH,         // "th" (tongue between teeth)
    DD,         // "d", "t", "n" (tongue on ridge)
    KK,         // "k", "g" (back of tongue)
    CH,         // "ch", "j", "sh" (lips rounded)
    SS,         // "s", "z" (teeth together)
    RR,         // "r" (lips slightly rounded)
    NN,         // "n", "ng" (nasal)
}

impl Viseme {
    /// Get viseme from phoneme hint
    pub fn from_phoneme(phoneme: &str) -> Self {
        match phoneme.to_lowercase().as_str() {
            "aa" | "ah" => Viseme::AA,
            "ao" | "aw" => Viseme::AO,
            "eh" | "e" => Viseme::EH,
            "iy" | "ee" | "i" => Viseme::IY,
            "uw" | "oo" | "u" => Viseme::UW,
            "ow" | "oh" | "o" => Viseme::OW,
            "ae" | "a" => Viseme::AE,
            "p" | "b" | "m" => Viseme::PP,
            "f" | "v" => Viseme::FF,
            "th" => Viseme::TH,
            "d" | "t" | "n" => Viseme::DD,
            "k" | "g" => Viseme::KK,
            "ch" | "j" | "sh" => Viseme::CH,
            "s" | "z" => Viseme::SS,
            "r" => Viseme::RR,
            _ => Viseme::Neutral,
        }
    }
}

/// Complete face state
#[derive(Debug, Clone)]
pub struct FaceState {
    /// Timestamp of this face state
    pub timestamp: StateTime,
    
    /// Is a face detected/present?
    pub present: bool,
    
    /// Head rotation (yaw, pitch, roll in radians)
    pub head_rotation: (f32, f32, f32),
    
    /// Emotion vector
    pub emotion: EmotionVector,
    
    /// Gaze state
    pub gaze: GazeState,
    
    /// Mouth state
    pub mouth: MouthState,
    
    /// Is the person speaking?
    pub speaking: bool,
    
    /// Confidence of face detection [0.0 - 1.0]
    pub confidence: f32,
}

impl FaceState {
    /// Create a new face state with defaults
    pub fn new(timestamp: StateTime) -> Self {
        Self {
            timestamp,
            present: true,
            head_rotation: (0.0, 0.0, 0.0),
            emotion: EmotionVector::neutral(),
            gaze: GazeState::forward(),
            mouth: MouthState::default(),
            speaking: false,
            confidence: 1.0,
        }
    }
    
    /// No face present
    pub fn absent(timestamp: StateTime) -> Self {
        Self {
            timestamp,
            present: false,
            head_rotation: (0.0, 0.0, 0.0),
            emotion: EmotionVector::neutral(),
            gaze: GazeState::forward(),
            mouth: MouthState::default(),
            speaking: false,
            confidence: 0.0,
        }
    }
    
    /// Reduce to minimal state (for L4 degradation)
    pub fn reduce_to_minimal(&mut self) {
        self.emotion = EmotionVector::neutral();
        self.head_rotation = (0.0, 0.0, 0.0);
        // Keep only: present, speaking, basic gaze
    }
    
    /// Convert to latent state (for L5 degradation)
    pub fn to_latent(self) -> FaceState {
        FaceState {
            timestamp: self.timestamp,
            present: self.present,
            head_rotation: (0.0, 0.0, 0.0),
            emotion: EmotionVector::neutral(),
            gaze: GazeState::forward(),
            mouth: MouthState::default(),
            speaking: false,
            confidence: 0.1,
        }
    }
    
    /// Interpolate between two face states
    pub fn lerp(&self, other: &FaceState, t: f32) -> FaceState {
        let t = t.clamp(0.0, 1.0);
        
        FaceState {
            timestamp: other.timestamp,
            present: if t < 0.5 { self.present } else { other.present },
            head_rotation: (
                self.head_rotation.0 + (other.head_rotation.0 - self.head_rotation.0) * t,
                self.head_rotation.1 + (other.head_rotation.1 - self.head_rotation.1) * t,
                self.head_rotation.2 + (other.head_rotation.2 - self.head_rotation.2) * t,
            ),
            emotion: self.emotion.blend(&other.emotion, t),
            gaze: self.gaze.lerp(&other.gaze, t),
            mouth: MouthState {
                openness: self.mouth.openness + (other.mouth.openness - self.mouth.openness) * t,
                smile: self.mouth.smile + (other.mouth.smile - self.mouth.smile) * t,
                viseme: if t < 0.5 { self.mouth.viseme } else { other.mouth.viseme },
            },
            speaking: if t < 0.5 { self.speaking } else { other.speaking },
            confidence: self.confidence + (other.confidence - self.confidence) * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_emotion_vector() {
        let mut emotion = EmotionVector::neutral();
        emotion.joy = 0.8;
        emotion.surprise = 0.2;
        
        let (dominant, value) = emotion.dominant();
        assert_eq!(dominant, "joy");
        assert_eq!(value, 0.8);
    }
    
    #[test]
    fn test_emotion_blend() {
        let happy = EmotionVector { joy: 1.0, ..Default::default() };
        let sad = EmotionVector { sadness: 1.0, ..Default::default() };
        
        let blended = happy.blend(&sad, 0.5);
        assert!((blended.joy - 0.5).abs() < 0.01);
        assert!((blended.sadness - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_face_state_lerp() {
        let time1 = StateTime::from_millis(0);
        let time2 = StateTime::from_millis(100);
        
        let mut face1 = FaceState::new(time1);
        face1.mouth.openness = 0.0;
        
        let mut face2 = FaceState::new(time2);
        face2.mouth.openness = 1.0;
        
        let interpolated = face1.lerp(&face2, 0.5);
        assert!((interpolated.mouth.openness - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_viseme_from_phoneme() {
        assert_eq!(Viseme::from_phoneme("aa"), Viseme::AA);
        assert_eq!(Viseme::from_phoneme("p"), Viseme::PP);
        assert_eq!(Viseme::from_phoneme("s"), Viseme::SS);
    }
}
