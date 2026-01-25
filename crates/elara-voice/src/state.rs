//! Voice State - The state of speech
//!
//! This represents the semantic state of voice, not audio samples.

use elara_core::{DegradationLevel, NodeId, StateTime};

/// Unique identifier for a voice state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceStateId(pub u64);

impl VoiceStateId {
    pub fn new(seq: u64) -> Self {
        Self(seq)
    }
}

/// Voice activity state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceActivity {
    /// Silent - no speech
    Silent,
    /// Speaking - active speech
    Speaking,
    /// Breathing - audible breath
    Breathing,
    /// Laughing
    Laughing,
    /// Crying
    Crying,
    /// Sighing
    Sighing,
}

impl Default for VoiceActivity {
    fn default() -> Self {
        Self::Silent
    }
}

/// Speech emotion (affects prosody)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechEmotion {
    Neutral,
    Happy,
    Sad,
    Angry,
    Fearful,
    Surprised,
    Disgusted,
    Excited,
    Calm,
    Whisper,
}

impl Default for SpeechEmotion {
    fn default() -> Self {
        Self::Neutral
    }
}

/// Pitch contour type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchContour {
    /// Flat/monotone
    Flat,
    /// Rising (question)
    Rising,
    /// Falling (statement)
    Falling,
    /// Rise-fall (emphasis)
    RiseFall,
    /// Fall-rise (uncertainty)
    FallRise,
}

impl Default for PitchContour {
    fn default() -> Self {
        Self::Flat
    }
}

/// Complete voice state for a moment in time
#[derive(Debug, Clone)]
pub struct VoiceState {
    /// Unique identifier
    pub id: VoiceStateId,

    /// Source node (speaker)
    pub source: NodeId,

    /// Timestamp
    pub timestamp: StateTime,

    /// Sequence number
    pub sequence: u64,

    /// Voice activity
    pub activity: VoiceActivity,

    /// Is this a keyframe?
    pub is_keyframe: bool,

    /// Reference to keyframe (for deltas)
    pub keyframe_ref: Option<VoiceStateId>,

    /// Current degradation level
    pub degradation: DegradationLevel,

    /// Parametric voice data (None if silent)
    pub params: Option<VoiceParams>,

    /// Confidence in this state [0.0 - 1.0]
    pub confidence: f32,
}

impl VoiceState {
    /// Create a new silent voice state
    pub fn silent(source: NodeId, timestamp: StateTime, sequence: u64) -> Self {
        Self {
            id: VoiceStateId::new(sequence),
            source,
            timestamp,
            sequence,
            activity: VoiceActivity::Silent,
            is_keyframe: true,
            keyframe_ref: None,
            degradation: DegradationLevel::L0_FullPerception,
            params: None,
            confidence: 1.0,
        }
    }

    /// Create a speaking voice state
    pub fn speaking(
        source: NodeId,
        timestamp: StateTime,
        sequence: u64,
        params: VoiceParams,
    ) -> Self {
        Self {
            id: VoiceStateId::new(sequence),
            source,
            timestamp,
            sequence,
            activity: VoiceActivity::Speaking,
            is_keyframe: true,
            keyframe_ref: None,
            degradation: DegradationLevel::L0_FullPerception,
            params: Some(params),
            confidence: 1.0,
        }
    }

    /// Create a delta (non-keyframe)
    pub fn delta(
        source: NodeId,
        timestamp: StateTime,
        sequence: u64,
        keyframe: VoiceStateId,
    ) -> Self {
        Self {
            id: VoiceStateId::new(sequence),
            source,
            timestamp,
            sequence,
            activity: VoiceActivity::Silent,
            is_keyframe: false,
            keyframe_ref: Some(keyframe),
            degradation: DegradationLevel::L0_FullPerception,
            params: None,
            confidence: 1.0,
        }
    }

    /// Set voice parameters
    pub fn with_params(mut self, params: VoiceParams) -> Self {
        self.params = Some(params);
        self.activity = VoiceActivity::Speaking;
        self
    }

    /// Set activity
    pub fn with_activity(mut self, activity: VoiceActivity) -> Self {
        self.activity = activity;
        self
    }

    /// Apply degradation
    pub fn degrade(&mut self, level: DegradationLevel) {
        self.degradation = level;

        if let Some(ref mut params) = self.params {
            params.degrade(level);
        }

        // At L3+, we only keep activity indicator
        if level >= DegradationLevel::L3_SymbolicPresence {
            self.params = None;
        }
    }

    /// Check if speaking
    pub fn is_speaking(&self) -> bool {
        matches!(
            self.activity,
            VoiceActivity::Speaking | VoiceActivity::Laughing | VoiceActivity::Crying
        )
    }

    /// Interpolate between two voice states
    pub fn lerp(&self, other: &VoiceState, t: f32) -> VoiceState {
        let t = t.clamp(0.0, 1.0);

        let mut result = if t < 0.5 { self.clone() } else { other.clone() };

        // Interpolate params if both have them
        if let (Some(p1), Some(p2)) = (&self.params, &other.params) {
            result.params = Some(p1.lerp(p2, t));
        }

        result.confidence = self.confidence * (1.0 - t) + other.confidence * t;

        result
    }
}

/// Parametric voice data
#[derive(Debug, Clone)]
pub struct VoiceParams {
    /// Fundamental frequency (pitch) in Hz [50 - 500]
    pub pitch: f32,

    /// Pitch variation/jitter
    pub pitch_variation: f32,

    /// Energy/loudness [0.0 - 1.0]
    pub energy: f32,

    /// Spectral envelope (formants) - simplified to 4 formants
    pub formants: [Formant; 4],

    /// Voicing ratio [0.0 = unvoiced, 1.0 = fully voiced]
    pub voicing: f32,

    /// Speech rate multiplier [0.5 - 2.0]
    pub rate: f32,

    /// Pitch contour
    pub contour: PitchContour,

    /// Emotion affecting prosody
    pub emotion: SpeechEmotion,

    /// Breathiness [0.0 - 1.0]
    pub breathiness: f32,

    /// Nasality [0.0 - 1.0]
    pub nasality: f32,
}

impl VoiceParams {
    /// Create default voice parameters
    pub fn new() -> Self {
        Self {
            pitch: 120.0, // Average male pitch
            pitch_variation: 0.1,
            energy: 0.5,
            formants: [
                Formant::new(500.0, 100.0, 1.0),  // F1
                Formant::new(1500.0, 150.0, 0.8), // F2
                Formant::new(2500.0, 200.0, 0.6), // F3
                Formant::new(3500.0, 250.0, 0.4), // F4
            ],
            voicing: 1.0,
            rate: 1.0,
            contour: PitchContour::Flat,
            emotion: SpeechEmotion::Neutral,
            breathiness: 0.1,
            nasality: 0.1,
        }
    }

    /// Create female voice parameters
    pub fn female() -> Self {
        Self {
            pitch: 220.0,
            pitch_variation: 0.15,
            energy: 0.5,
            formants: [
                Formant::new(550.0, 100.0, 1.0),
                Formant::new(1650.0, 150.0, 0.8),
                Formant::new(2750.0, 200.0, 0.6),
                Formant::new(3850.0, 250.0, 0.4),
            ],
            voicing: 1.0,
            rate: 1.0,
            contour: PitchContour::Flat,
            emotion: SpeechEmotion::Neutral,
            breathiness: 0.15,
            nasality: 0.1,
        }
    }

    /// Apply degradation
    pub fn degrade(&mut self, level: DegradationLevel) {
        match level {
            DegradationLevel::L0_FullPerception => {
                // Full quality - no changes
            }
            DegradationLevel::L1_DistortedPerception => {
                // Reduce formant precision
                for f in &mut self.formants {
                    f.bandwidth *= 1.5;
                }
                self.pitch_variation = 0.0;
            }
            DegradationLevel::L2_FragmentedPerception => {
                // Only pitch + energy, robotic
                self.formants = [Formant::default(); 4];
                self.breathiness = 0.0;
                self.nasality = 0.0;
                self.contour = PitchContour::Flat;
            }
            _ => {
                // L3+ handled at VoiceState level
            }
        }
    }

    /// Interpolate between two parameter sets
    pub fn lerp(&self, other: &VoiceParams, t: f32) -> VoiceParams {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;

        VoiceParams {
            pitch: self.pitch * inv + other.pitch * t,
            pitch_variation: self.pitch_variation * inv + other.pitch_variation * t,
            energy: self.energy * inv + other.energy * t,
            formants: [
                self.formants[0].lerp(&other.formants[0], t),
                self.formants[1].lerp(&other.formants[1], t),
                self.formants[2].lerp(&other.formants[2], t),
                self.formants[3].lerp(&other.formants[3], t),
            ],
            voicing: self.voicing * inv + other.voicing * t,
            rate: self.rate * inv + other.rate * t,
            contour: if t < 0.5 { self.contour } else { other.contour },
            emotion: if t < 0.5 { self.emotion } else { other.emotion },
            breathiness: self.breathiness * inv + other.breathiness * t,
            nasality: self.nasality * inv + other.nasality * t,
        }
    }

    /// Estimate encoded size in bytes
    pub fn encoded_size(&self) -> usize {
        // pitch(4) + pitch_var(4) + energy(4) + formants(4*12) + voicing(4) + rate(4)
        // + contour(1) + emotion(1) + breathiness(4) + nasality(4)
        4 + 4 + 4 + 48 + 4 + 4 + 1 + 1 + 4 + 4
    }
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Formant (resonance peak in vocal tract)
#[derive(Debug, Clone, Copy, Default)]
pub struct Formant {
    /// Center frequency in Hz
    pub frequency: f32,
    /// Bandwidth in Hz
    pub bandwidth: f32,
    /// Amplitude [0.0 - 1.0]
    pub amplitude: f32,
}

impl Formant {
    pub fn new(frequency: f32, bandwidth: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            bandwidth,
            amplitude,
        }
    }

    pub fn lerp(&self, other: &Formant, t: f32) -> Formant {
        let inv = 1.0 - t;
        Formant {
            frequency: self.frequency * inv + other.frequency * t,
            bandwidth: self.bandwidth * inv + other.bandwidth * t,
            amplitude: self.amplitude * inv + other.amplitude * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_state_silent() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let state = VoiceState::silent(node, time, 1);

        assert!(!state.is_speaking());
        assert!(state.params.is_none());
    }

    #[test]
    fn test_voice_state_speaking() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let params = VoiceParams::new();
        let state = VoiceState::speaking(node, time, 1, params);

        assert!(state.is_speaking());
        assert!(state.params.is_some());
    }

    #[test]
    fn test_voice_params_lerp() {
        let p1 = VoiceParams::new();
        let mut p2 = VoiceParams::female();
        p2.energy = 1.0;

        let mid = p1.lerp(&p2, 0.5);

        assert!((mid.pitch - 170.0).abs() < 1.0); // (120 + 220) / 2
        assert!((mid.energy - 0.75).abs() < 0.01); // (0.5 + 1.0) / 2
    }

    #[test]
    fn test_voice_degradation() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let params = VoiceParams::new();
        let mut state = VoiceState::speaking(node, time, 1, params);

        state.degrade(DegradationLevel::L3_SymbolicPresence);

        // At L3, params should be removed
        assert!(state.params.is_none());
        // But activity should remain
        assert!(state.is_speaking());
    }
}
