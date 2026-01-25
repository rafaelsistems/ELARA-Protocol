//! Voice Frame - Time-sliced voice state
//!
//! Voice is transmitted as frames (10-20ms each).
//! Each frame contains the parametric state for that time slice.

use crate::{SpeechEmotion, VoiceActivity, VoiceParams};
use elara_core::{NodeId, StateTime};

/// Frame duration in milliseconds
pub const FRAME_DURATION_MS: u32 = 20;

/// Voice frame - a single time slice of voice
#[derive(Debug, Clone)]
pub struct VoiceFrame {
    /// Frame sequence number
    pub sequence: u64,

    /// Timestamp of frame start
    pub timestamp: StateTime,

    /// Source node
    pub source: NodeId,

    /// Voice activity
    pub activity: VoiceActivity,

    /// Voiced/unvoiced flag
    pub voiced: bool,

    /// Pitch in Hz (0 if unvoiced)
    pub pitch: f32,

    /// Energy [0.0 - 1.0]
    pub energy: f32,

    /// Spectral envelope index (references a codebook)
    pub spectral_index: u16,

    /// Residual noise seed (for synthesis variation)
    pub noise_seed: u16,

    /// Delta from previous frame (for compression)
    pub is_delta: bool,
}

impl VoiceFrame {
    /// Create a silent frame
    pub fn silent(source: NodeId, timestamp: StateTime, sequence: u64) -> Self {
        Self {
            sequence,
            timestamp,
            source,
            activity: VoiceActivity::Silent,
            voiced: false,
            pitch: 0.0,
            energy: 0.0,
            spectral_index: 0,
            noise_seed: 0,
            is_delta: false,
        }
    }

    /// Create a voiced frame
    pub fn voiced(
        source: NodeId,
        timestamp: StateTime,
        sequence: u64,
        pitch: f32,
        energy: f32,
    ) -> Self {
        Self {
            sequence,
            timestamp,
            source,
            activity: VoiceActivity::Speaking,
            voiced: true,
            pitch,
            energy,
            spectral_index: 0,
            noise_seed: rand::random(),
            is_delta: false,
        }
    }

    /// Create from full voice params
    pub fn from_params(
        source: NodeId,
        timestamp: StateTime,
        sequence: u64,
        params: &VoiceParams,
    ) -> Self {
        Self {
            sequence,
            timestamp,
            source,
            activity: VoiceActivity::Speaking,
            voiced: params.voicing > 0.5,
            pitch: params.pitch,
            energy: params.energy,
            spectral_index: Self::params_to_spectral_index(params),
            noise_seed: rand::random(),
            is_delta: false,
        }
    }

    /// Convert params to spectral index (simplified codebook lookup)
    fn params_to_spectral_index(params: &VoiceParams) -> u16 {
        // Simplified: encode formant ratios into index
        let f1_ratio = (params.formants[0].frequency / 1000.0).clamp(0.0, 1.0);
        let f2_ratio = (params.formants[1].frequency / 3000.0).clamp(0.0, 1.0);

        (f1_ratio * 255.0) as u16 | ((f2_ratio * 255.0) as u16) << 8
    }

    /// Estimate encoded size in bytes
    pub fn encoded_size(&self) -> usize {
        // sequence(8) + timestamp(8) + source(8) + activity(1) + voiced(1)
        // + pitch(4) + energy(4) + spectral(2) + noise(2) + is_delta(1)
        8 + 8 + 8 + 1 + 1 + 4 + 4 + 2 + 2 + 1
    }

    /// Check if this frame represents speech
    pub fn is_speech(&self) -> bool {
        self.energy > 0.01 && matches!(self.activity, VoiceActivity::Speaking)
    }

    /// Interpolate between two frames
    pub fn lerp(&self, other: &VoiceFrame, t: f32) -> VoiceFrame {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;

        VoiceFrame {
            sequence: if t < 0.5 {
                self.sequence
            } else {
                other.sequence
            },
            timestamp: if t < 0.5 {
                self.timestamp
            } else {
                other.timestamp
            },
            source: self.source,
            activity: if t < 0.5 {
                self.activity
            } else {
                other.activity
            },
            voiced: if t < 0.5 { self.voiced } else { other.voiced },
            pitch: self.pitch * inv + other.pitch * t,
            energy: self.energy * inv + other.energy * t,
            spectral_index: if t < 0.5 {
                self.spectral_index
            } else {
                other.spectral_index
            },
            noise_seed: rand::random(),
            is_delta: false,
        }
    }
}

/// Voice frame buffer for jitter compensation
#[derive(Debug)]
pub struct VoiceFrameBuffer {
    /// Buffered frames
    frames: Vec<VoiceFrame>,

    /// Maximum buffer size
    max_size: usize,

    /// Target delay in frames
    target_delay: usize,

    /// Last played sequence
    last_played: u64,
}

impl VoiceFrameBuffer {
    /// Create a new buffer
    pub fn new(max_size: usize, target_delay: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_size),
            max_size,
            target_delay,
            last_played: 0,
        }
    }

    /// Add a frame to the buffer
    pub fn push(&mut self, frame: VoiceFrame) {
        // Insert in order by sequence
        let pos = self
            .frames
            .iter()
            .position(|f| f.sequence > frame.sequence)
            .unwrap_or(self.frames.len());

        self.frames.insert(pos, frame);

        // Remove old frames
        while self.frames.len() > self.max_size {
            self.frames.remove(0);
        }
    }

    /// Get the next frame to play
    pub fn pop(&mut self) -> Option<VoiceFrame> {
        if self.frames.len() < self.target_delay {
            return None; // Not enough buffered
        }

        let frame = self.frames.remove(0);
        self.last_played = frame.sequence;
        Some(frame)
    }

    /// Get frame at specific sequence (for interpolation)
    pub fn get(&self, sequence: u64) -> Option<&VoiceFrame> {
        self.frames.iter().find(|f| f.sequence == sequence)
    }

    /// Get interpolated frame at a specific time
    pub fn get_at(&self, time: StateTime) -> Option<VoiceFrame> {
        if self.frames.is_empty() {
            return None;
        }

        let time_ms = time.as_millis();

        // Find surrounding frames
        let mut before: Option<&VoiceFrame> = None;
        let mut after: Option<&VoiceFrame> = None;

        for frame in &self.frames {
            if frame.timestamp.as_millis() <= time_ms {
                before = Some(frame);
            } else {
                after = Some(frame);
                break;
            }
        }

        match (before, after) {
            (Some(b), Some(a)) => {
                let range = a.timestamp.as_millis() - b.timestamp.as_millis();
                if range <= 0 {
                    return Some(b.clone());
                }
                let t = (time_ms - b.timestamp.as_millis()) as f32 / range as f32;
                Some(b.lerp(a, t))
            }
            (Some(b), None) => Some(b.clone()),
            (None, Some(a)) => Some(a.clone()),
            (None, None) => None,
        }
    }

    /// Number of buffered frames
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Is buffer empty?
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.frames.clear();
        self.last_played = 0;
    }

    /// Get buffer fill ratio [0.0 - 1.0]
    pub fn fill_ratio(&self) -> f32 {
        self.frames.len() as f32 / self.max_size as f32
    }
}

/// Voice frame generator (simulates voice capture)
#[derive(Debug)]
pub struct VoiceFrameGenerator {
    /// Source node
    source: NodeId,

    /// Current sequence
    sequence: u64,

    /// Base pitch
    base_pitch: f32,

    /// Current emotion
    emotion: SpeechEmotion,

    /// Is currently speaking
    speaking: bool,

    /// Phase for pitch modulation
    phase: f32,
}

impl VoiceFrameGenerator {
    /// Create a new generator
    pub fn new(source: NodeId, base_pitch: f32) -> Self {
        Self {
            source,
            sequence: 0,
            base_pitch,
            emotion: SpeechEmotion::Neutral,
            speaking: false,
            phase: 0.0,
        }
    }

    /// Set speaking state
    pub fn set_speaking(&mut self, speaking: bool) {
        self.speaking = speaking;
    }

    /// Set emotion
    pub fn set_emotion(&mut self, emotion: SpeechEmotion) {
        self.emotion = emotion;
    }

    /// Generate next frame
    pub fn next_frame(&mut self, timestamp: StateTime) -> VoiceFrame {
        self.sequence += 1;
        self.phase += 0.1;

        if !self.speaking {
            return VoiceFrame::silent(self.source, timestamp, self.sequence);
        }

        // Simulate natural pitch variation
        let pitch_mod = match self.emotion {
            SpeechEmotion::Happy | SpeechEmotion::Excited => 1.1 + 0.1 * self.phase.sin(),
            SpeechEmotion::Sad => 0.9 - 0.05 * self.phase.sin(),
            SpeechEmotion::Angry => 1.2 + 0.15 * (self.phase * 2.0).sin(),
            _ => 1.0 + 0.05 * self.phase.sin(),
        };

        let pitch = self.base_pitch * pitch_mod;

        // Simulate energy variation
        let energy = 0.5 + 0.3 * (self.phase * 0.7).sin().abs();

        VoiceFrame::voiced(self.source, timestamp, self.sequence, pitch, energy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_frame_silent() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let frame = VoiceFrame::silent(node, time, 1);

        assert!(!frame.is_speech());
        assert!(!frame.voiced);
    }

    #[test]
    fn test_voice_frame_voiced() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let frame = VoiceFrame::voiced(node, time, 1, 120.0, 0.5);

        assert!(frame.is_speech());
        assert!(frame.voiced);
    }

    #[test]
    fn test_frame_buffer() {
        let mut buffer = VoiceFrameBuffer::new(10, 2);
        let node = NodeId::new(1);

        // Add frames
        for i in 0..5 {
            let frame =
                VoiceFrame::voiced(node, StateTime::from_millis(i * 20), i as u64, 120.0, 0.5);
            buffer.push(frame);
        }

        assert_eq!(buffer.len(), 5);

        // Pop should work after target_delay frames
        let frame = buffer.pop();
        assert!(frame.is_some());
    }

    #[test]
    fn test_frame_generator() {
        let node = NodeId::new(1);
        let mut gen = VoiceFrameGenerator::new(node, 120.0);

        // Silent by default
        let frame = gen.next_frame(StateTime::from_millis(0));
        assert!(!frame.is_speech());

        // Start speaking
        gen.set_speaking(true);
        let frame = gen.next_frame(StateTime::from_millis(20));
        assert!(frame.is_speech());
    }
}
