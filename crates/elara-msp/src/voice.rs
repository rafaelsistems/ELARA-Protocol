//! MSP Voice Profile - profile:voice-minimal implementation
//!
//! Voice state atom: ω:voice:user_id
//! 
//! Voice is NOT audio PCM - it's STATE OF SPEECH:
//! - voiced/unvoiced flag
//! - pitch (quantized)
//! - energy (quantized)
//! - spectral envelope index
//! - residual noise seed

use elara_core::{DeltaLaw, InterpolationType, NodeId, StateAtom, StateBounds, StateId, StateTime, StateType};

/// State type prefix for voice
pub const STATE_TYPE_VOICE: u16 = 0x0010;

/// Create a voice state ID
pub fn voice_id(user_id: NodeId) -> StateId {
    StateId::from_type_instance(STATE_TYPE_VOICE, user_id.0)
}

/// Voice frame - parametric representation of speech
/// Total size: 9 bytes per frame
#[derive(Clone, Copy, Debug, Default)]
pub struct VoiceFrame {
    /// Voiced (true) or unvoiced (false)
    pub voiced: bool,
    /// Pitch (F0) - quantized 0-255 (maps to ~50-500 Hz)
    pub pitch: u8,
    /// Energy - quantized 0-255 (log scale)
    pub energy: u8,
    /// Spectral envelope codebook index
    pub spectral_index: u16,
    /// Residual noise seed for synthesis
    pub residual_seed: u16,
    /// Frame timestamp (relative to session)
    pub timestamp: StateTime,
    /// Frame duration in milliseconds (typically 10-20)
    pub duration_ms: u8,
}

impl VoiceFrame {
    /// Wire size in bytes
    pub const WIRE_SIZE: usize = 9;

    /// Create a new voice frame
    pub fn new(timestamp: StateTime) -> Self {
        VoiceFrame {
            voiced: false,
            pitch: 128,
            energy: 0,
            spectral_index: 0,
            residual_seed: 0,
            timestamp,
            duration_ms: 10,
        }
    }

    /// Create a silence frame
    pub fn silence(timestamp: StateTime) -> Self {
        VoiceFrame {
            voiced: false,
            pitch: 0,
            energy: 0,
            spectral_index: 0,
            residual_seed: 0,
            timestamp,
            duration_ms: 10,
        }
    }

    /// Encode frame for wire format (9 bytes)
    pub fn encode(&self, buf: &mut [u8]) {
        debug_assert!(buf.len() >= Self::WIRE_SIZE);

        // Byte 0: voiced flag (bit 7) + duration (bits 0-6)
        buf[0] = if self.voiced { 0x80 } else { 0x00 } | (self.duration_ms & 0x7F);
        // Byte 1: pitch
        buf[1] = self.pitch;
        // Byte 2: energy
        buf[2] = self.energy;
        // Bytes 3-4: spectral index (LE)
        buf[3..5].copy_from_slice(&self.spectral_index.to_le_bytes());
        // Bytes 5-6: residual seed (LE)
        buf[5..7].copy_from_slice(&self.residual_seed.to_le_bytes());
        // Bytes 7-8: timestamp offset (LE, 100μs units, truncated to u16)
        let ts_offset = (self.timestamp.as_micros() / 100) as u16;
        buf[7..9].copy_from_slice(&ts_offset.to_le_bytes());
    }

    /// Decode frame from wire format
    pub fn decode(buf: &[u8], reference_time: StateTime) -> Option<Self> {
        if buf.len() < Self::WIRE_SIZE {
            return None;
        }

        let voiced = buf[0] & 0x80 != 0;
        let duration_ms = buf[0] & 0x7F;
        let pitch = buf[1];
        let energy = buf[2];
        let spectral_index = u16::from_le_bytes([buf[3], buf[4]]);
        let residual_seed = u16::from_le_bytes([buf[5], buf[6]]);
        let ts_offset = u16::from_le_bytes([buf[7], buf[8]]) as i64 * 100;
        let timestamp = StateTime::from_micros(reference_time.as_micros() + ts_offset);

        Some(VoiceFrame {
            voiced,
            pitch,
            energy,
            spectral_index,
            residual_seed,
            timestamp,
            duration_ms,
        })
    }

    /// Convert pitch value to Hz (approximate)
    pub fn pitch_hz(&self) -> f32 {
        if !self.voiced || self.pitch == 0 {
            0.0
        } else {
            // Map 1-255 to ~50-500 Hz (log scale)
            50.0 * (500.0_f32 / 50.0).powf(self.pitch as f32 / 255.0)
        }
    }

    /// Convert energy to linear scale (0.0 - 1.0)
    pub fn energy_linear(&self) -> f32 {
        self.energy as f32 / 255.0
    }

    /// Blend two frames (for correction)
    pub fn blend(&self, other: &VoiceFrame, weight: f64) -> VoiceFrame {
        let w = weight as f32;
        let inv_w = 1.0 - w;

        VoiceFrame {
            // Voiced: threshold at 0.5
            voiced: if weight > 0.5 { other.voiced } else { self.voiced },
            // Pitch: linear interpolation
            pitch: ((self.pitch as f32 * inv_w) + (other.pitch as f32 * w)) as u8,
            // Energy: linear interpolation
            energy: ((self.energy as f32 * inv_w) + (other.energy as f32 * w)) as u8,
            // Spectral: take the one with higher weight
            spectral_index: if weight > 0.3 {
                other.spectral_index
            } else {
                self.spectral_index
            },
            // Residual: take from actual data
            residual_seed: other.residual_seed,
            // Timestamp: from actual
            timestamp: other.timestamp,
            duration_ms: other.duration_ms,
        }
    }
}

/// Voice state - current voice parameters for a user
#[derive(Clone, Debug)]
pub struct VoiceState {
    /// User ID
    pub user_id: NodeId,
    /// Current frame
    pub current_frame: VoiceFrame,
    /// Frame history for prediction
    pub history: Vec<VoiceFrame>,
    /// Maximum history size
    pub max_history: usize,
    /// Prediction depth (how many frames predicted)
    pub prediction_depth: u32,
    /// Last actual frame timestamp
    pub last_actual: StateTime,
    /// Is muted
    pub muted: bool,
}

impl VoiceState {
    pub fn new(user_id: NodeId) -> Self {
        VoiceState {
            user_id,
            current_frame: VoiceFrame::new(StateTime::ZERO),
            history: Vec::new(),
            max_history: 100,
            prediction_depth: 0,
            last_actual: StateTime::ZERO,
            muted: false,
        }
    }

    /// Update with actual frame data
    pub fn update(&mut self, frame: VoiceFrame) {
        self.history.push(self.current_frame);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        self.current_frame = frame;
        self.prediction_depth = 0;
        self.last_actual = frame.timestamp;
    }

    /// Apply correction (blend actual with current)
    pub fn apply_correction(&mut self, frame: VoiceFrame, weight: f64) {
        self.current_frame = self.current_frame.blend(&frame, weight);
        self.history.push(frame);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Predict next frame
    pub fn predict(&mut self, target_time: StateTime) -> VoiceFrame {
        self.prediction_depth += 1;

        let mut predicted = VoiceFrame {
            timestamp: target_time,
            duration_ms: 10,
            ..self.current_frame
        };

        // Predict pitch: linear extrapolation with damping
        if self.history.len() >= 2 {
            let recent: Vec<u8> = self.history.iter().rev().take(5).map(|f| f.pitch).collect();
            if recent.len() >= 2 {
                let slope = (recent[0] as f32 - recent[recent.len() - 1] as f32) / recent.len() as f32;
                let extrapolated = recent[0] as f32 + slope * self.prediction_depth as f32;

                // Damping toward mean (128)
                let damping = 0.9_f32.powi(self.prediction_depth as i32);
                let damped = extrapolated * damping + 128.0 * (1.0 - damping);

                predicted.pitch = damped.clamp(0.0, 255.0) as u8;
            }
        }

        // Predict energy: decay toward silence
        let decay = 0.95_f32.powi(self.prediction_depth as i32);
        predicted.energy = (self.current_frame.energy as f32 * decay) as u8;

        // Voiced state tends to persist
        predicted.voiced = self.current_frame.voiced && predicted.energy > 10;

        self.current_frame = predicted;
        predicted
    }

    /// Get degradation level based on prediction depth
    pub fn degradation_level(&self) -> VoiceDegradationLevel {
        match self.prediction_depth {
            0..=5 => VoiceDegradationLevel::Full,
            6..=15 => VoiceDegradationLevel::ParameterOnly,
            16..=30 => VoiceDegradationLevel::Symbolic,
            31..=60 => VoiceDegradationLevel::PresenceOnly,
            _ => VoiceDegradationLevel::Heartbeat,
        }
    }

    /// Encode current state for wire
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 8 + VoiceFrame::WIRE_SIZE + 5];

        // User ID (8 bytes)
        buf[0..8].copy_from_slice(&self.user_id.to_bytes());

        // Current frame (9 bytes)
        self.current_frame.encode(&mut buf[8..17]);

        // Prediction depth (4 bytes)
        buf[17..21].copy_from_slice(&self.prediction_depth.to_le_bytes());

        // Muted flag (1 byte)
        buf[21] = if self.muted { 1 } else { 0 };

        buf
    }
}

/// Voice degradation levels
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceDegradationLevel {
    /// Full quality - all parameters
    Full,
    /// Parameter only - pitch + energy, default spectral
    ParameterOnly,
    /// Symbolic - just voiced/unvoiced + energy
    Symbolic,
    /// Presence only - just "speaking" indicator
    PresenceOnly,
    /// Heartbeat - just "alive" signal
    Heartbeat,
}

impl VoiceDegradationLevel {
    /// Apply degradation to a frame
    pub fn apply(&self, frame: &mut VoiceFrame) {
        match self {
            VoiceDegradationLevel::Full => {
                // No degradation
            }
            VoiceDegradationLevel::ParameterOnly => {
                // Use default spectral
                frame.spectral_index = 0;
            }
            VoiceDegradationLevel::Symbolic => {
                // Just voiced/unvoiced and energy
                frame.pitch = if frame.voiced { 128 } else { 0 };
                frame.spectral_index = 0;
            }
            VoiceDegradationLevel::PresenceOnly => {
                // Convert to presence indicator
                frame.voiced = frame.energy > 20;
                frame.pitch = 0;
                frame.spectral_index = 0;
            }
            VoiceDegradationLevel::Heartbeat => {
                // Minimal alive signal
                frame.voiced = false;
                frame.pitch = 0;
                frame.energy = 1; // Non-zero to indicate alive
                frame.spectral_index = 0;
            }
        }
    }
}

/// Create a voice state atom
pub fn create_voice_atom(user_id: NodeId) -> StateAtom {
    let mut atom = StateAtom::new(voice_id(user_id), StateType::Perceptual, user_id);
    atom.delta_law = DeltaLaw::ContinuousBlend {
        interpolation: InterpolationType::Linear,
        max_deviation: 0.3,
    };
    atom.bounds = StateBounds {
        max_size: 1024,
        rate_limit: Some(elara_core::RateLimit::new(100, 1000)), // 100 frames/sec
        max_entropy: 1.0,
    };
    atom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_frame_encode_decode() {
        let frame = VoiceFrame {
            voiced: true,
            pitch: 150,
            energy: 200,
            spectral_index: 1234,
            residual_seed: 5678,
            timestamp: StateTime::from_millis(1000),
            duration_ms: 10,
        };

        let mut buf = [0u8; VoiceFrame::WIRE_SIZE];
        frame.encode(&mut buf);

        let decoded = VoiceFrame::decode(&buf, StateTime::ZERO).unwrap();

        assert_eq!(decoded.voiced, frame.voiced);
        assert_eq!(decoded.pitch, frame.pitch);
        assert_eq!(decoded.energy, frame.energy);
        assert_eq!(decoded.spectral_index, frame.spectral_index);
        assert_eq!(decoded.residual_seed, frame.residual_seed);
        assert_eq!(decoded.duration_ms, frame.duration_ms);
    }

    #[test]
    fn test_voice_frame_blend() {
        let frame1 = VoiceFrame {
            voiced: true,
            pitch: 100,
            energy: 200,
            spectral_index: 10,
            residual_seed: 1,
            timestamp: StateTime::from_millis(1000),
            duration_ms: 10,
        };

        let frame2 = VoiceFrame {
            voiced: false,
            pitch: 200,
            energy: 100,
            spectral_index: 20,
            residual_seed: 2,
            timestamp: StateTime::from_millis(1010),
            duration_ms: 10,
        };

        let blended = frame1.blend(&frame2, 0.5);

        // Pitch should be interpolated
        assert!(blended.pitch > 100 && blended.pitch < 200);
        // Energy should be interpolated
        assert!(blended.energy > 100 && blended.energy < 200);
    }

    #[test]
    fn test_voice_state_prediction() {
        let mut state = VoiceState::new(NodeId::new(1));

        // Add some history
        for i in 0..10 {
            state.update(VoiceFrame {
                voiced: true,
                pitch: 100 + i as u8,
                energy: 200,
                spectral_index: 0,
                residual_seed: 0,
                timestamp: StateTime::from_millis(i as i64 * 10),
                duration_ms: 10,
            });
        }

        // Predict
        let predicted = state.predict(StateTime::from_millis(100));

        // Should have predicted
        assert_eq!(state.prediction_depth, 1);
        // Energy should decay
        assert!(predicted.energy < 200);
    }

    #[test]
    fn test_degradation_levels() {
        let state = VoiceState::new(NodeId::new(1));
        assert_eq!(state.degradation_level(), VoiceDegradationLevel::Full);

        let mut state = VoiceState::new(NodeId::new(1));
        state.prediction_depth = 10;
        assert_eq!(state.degradation_level(), VoiceDegradationLevel::ParameterOnly);

        state.prediction_depth = 50;
        assert_eq!(state.degradation_level(), VoiceDegradationLevel::PresenceOnly);
    }
}
