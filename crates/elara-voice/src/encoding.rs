//! Voice Encoding - Wire format for voice state
//!
//! This is NOT audio codec. This is ELARA-native voice state encoding.

use crate::{
    Formant, PitchContour, SpeechEmotion, VoiceActivity, VoiceFrame, VoiceParams, VoiceState,
    VoiceStateId,
};
use elara_core::{DegradationLevel, NodeId, StateTime};

/// Encoding error
#[derive(Debug, Clone)]
pub enum VoiceEncodingError {
    BufferTooSmall,
    InvalidData,
    UnsupportedVersion,
}

/// Voice state encoder
pub struct VoiceEncoder;

impl VoiceEncoder {
    /// Encode a voice state to bytes
    pub fn encode_state(state: &VoiceState) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);

        // Header
        buf.push(0x01); // Version
        buf.push(state.activity as u8);
        buf.push(if state.is_keyframe { 0x01 } else { 0x00 });
        buf.push(state.degradation.level());

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

        // Confidence (4 bytes)
        buf.extend_from_slice(&state.confidence.to_le_bytes());

        // Has params flag
        buf.push(if state.params.is_some() { 0x01 } else { 0x00 });

        // Encode params if present
        if let Some(ref params) = state.params {
            Self::encode_params(params, &mut buf);
        }

        buf
    }

    fn encode_params(params: &VoiceParams, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&params.pitch.to_le_bytes());
        buf.extend_from_slice(&params.pitch_variation.to_le_bytes());
        buf.extend_from_slice(&params.energy.to_le_bytes());

        // Formants (4 x 12 bytes = 48 bytes)
        for formant in &params.formants {
            buf.extend_from_slice(&formant.frequency.to_le_bytes());
            buf.extend_from_slice(&formant.bandwidth.to_le_bytes());
            buf.extend_from_slice(&formant.amplitude.to_le_bytes());
        }

        buf.extend_from_slice(&params.voicing.to_le_bytes());
        buf.extend_from_slice(&params.rate.to_le_bytes());
        buf.push(params.contour as u8);
        buf.push(params.emotion as u8);
        buf.extend_from_slice(&params.breathiness.to_le_bytes());
        buf.extend_from_slice(&params.nasality.to_le_bytes());
    }

    /// Decode a voice state from bytes
    pub fn decode_state(data: &[u8]) -> Result<VoiceState, VoiceEncodingError> {
        if data.len() < 49 {
            return Err(VoiceEncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        // Header
        let version = data[pos];
        pos += 1;
        if version != 0x01 {
            return Err(VoiceEncodingError::UnsupportedVersion);
        }

        let activity = match data[pos] {
            0 => VoiceActivity::Silent,
            1 => VoiceActivity::Speaking,
            2 => VoiceActivity::Breathing,
            3 => VoiceActivity::Laughing,
            4 => VoiceActivity::Crying,
            5 => VoiceActivity::Sighing,
            _ => VoiceActivity::Silent,
        };
        pos += 1;

        let is_keyframe = data[pos] == 0x01;
        pos += 1;
        let degradation_level = data[pos];
        pos += 1;

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
            Some(VoiceStateId(keyframe_ref_val))
        };

        // Confidence
        let confidence = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        // Has params
        let has_params = data[pos] == 0x01;
        pos += 1;

        let params = if has_params && data.len() >= pos + 78 {
            Some(Self::decode_params(&data[pos..])?)
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

        Ok(VoiceState {
            id: VoiceStateId(id),
            source: NodeId::new(source),
            timestamp: StateTime::from_millis(timestamp),
            sequence,
            activity,
            is_keyframe,
            keyframe_ref,
            degradation,
            params,
            confidence,
        })
    }

    fn decode_params(data: &[u8]) -> Result<VoiceParams, VoiceEncodingError> {
        if data.len() < 78 {
            return Err(VoiceEncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        let pitch = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let pitch_variation = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let energy = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        let mut formants = [Formant::default(); 4];
        for formant in &mut formants {
            *formant = Formant {
                frequency: f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()),
                bandwidth: f32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
                amplitude: f32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
            };
            pos += 12;
        }

        let voicing = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let rate = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        let contour = match data[pos] {
            0 => PitchContour::Flat,
            1 => PitchContour::Rising,
            2 => PitchContour::Falling,
            3 => PitchContour::RiseFall,
            4 => PitchContour::FallRise,
            _ => PitchContour::Flat,
        };
        pos += 1;

        let emotion = match data[pos] {
            0 => SpeechEmotion::Neutral,
            1 => SpeechEmotion::Happy,
            2 => SpeechEmotion::Sad,
            3 => SpeechEmotion::Angry,
            4 => SpeechEmotion::Fearful,
            5 => SpeechEmotion::Surprised,
            6 => SpeechEmotion::Disgusted,
            7 => SpeechEmotion::Excited,
            8 => SpeechEmotion::Calm,
            9 => SpeechEmotion::Whisper,
            _ => SpeechEmotion::Neutral,
        };
        pos += 1;

        let breathiness = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;
        let nasality = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());

        Ok(VoiceParams {
            pitch,
            pitch_variation,
            energy,
            formants,
            voicing,
            rate,
            contour,
            emotion,
            breathiness,
            nasality,
        })
    }

    /// Encode a voice frame to bytes (compact format)
    pub fn encode_frame(frame: &VoiceFrame) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);

        // Compact header
        buf.push(0x02); // Frame marker
        buf.push(frame.activity as u8);
        buf.push(if frame.voiced { 0x01 } else { 0x00 });
        buf.push(if frame.is_delta { 0x01 } else { 0x00 });

        // Sequence (8 bytes)
        buf.extend_from_slice(&frame.sequence.to_le_bytes());

        // Timestamp (8 bytes)
        buf.extend_from_slice(&frame.timestamp.as_millis().to_le_bytes());

        // Source (8 bytes)
        buf.extend_from_slice(&frame.source.0.to_le_bytes());

        // Voice data
        buf.extend_from_slice(&frame.pitch.to_le_bytes());
        buf.extend_from_slice(&frame.energy.to_le_bytes());
        buf.extend_from_slice(&frame.spectral_index.to_le_bytes());
        buf.extend_from_slice(&frame.noise_seed.to_le_bytes());

        buf
    }

    /// Decode a voice frame from bytes
    pub fn decode_frame(data: &[u8]) -> Result<VoiceFrame, VoiceEncodingError> {
        if data.len() < 40 {
            return Err(VoiceEncodingError::BufferTooSmall);
        }

        let mut pos = 0;

        let marker = data[pos];
        pos += 1;
        if marker != 0x02 {
            return Err(VoiceEncodingError::InvalidData);
        }

        let activity = match data[pos] {
            0 => VoiceActivity::Silent,
            1 => VoiceActivity::Speaking,
            2 => VoiceActivity::Breathing,
            3 => VoiceActivity::Laughing,
            4 => VoiceActivity::Crying,
            5 => VoiceActivity::Sighing,
            _ => VoiceActivity::Silent,
        };
        pos += 1;

        let voiced = data[pos] == 0x01;
        pos += 1;
        let is_delta = data[pos] == 0x01;
        pos += 1;

        let sequence = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        let timestamp = i64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        let source = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        let pitch = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        let energy = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
        pos += 4;

        let spectral_index = u16::from_le_bytes(data[pos..pos + 2].try_into().unwrap());
        pos += 2;

        let noise_seed = u16::from_le_bytes(data[pos..pos + 2].try_into().unwrap());

        Ok(VoiceFrame {
            sequence,
            timestamp: StateTime::from_millis(timestamp),
            source: NodeId::new(source),
            activity,
            voiced,
            pitch,
            energy,
            spectral_index,
            noise_seed,
            is_delta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_state() {
        let node = NodeId::new(12345);
        let time = StateTime::from_millis(1000);
        let params = VoiceParams::new();
        let state = VoiceState::speaking(node, time, 1, params);

        let encoded = VoiceEncoder::encode_state(&state);
        let decoded = VoiceEncoder::decode_state(&encoded).unwrap();

        assert_eq!(decoded.id.0, state.id.0);
        assert_eq!(decoded.source.0, state.source.0);
        assert_eq!(decoded.sequence, state.sequence);
        assert!(decoded.params.is_some());
    }

    #[test]
    fn test_encode_decode_frame() {
        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let frame = VoiceFrame::voiced(node, time, 1, 120.0, 0.5);

        let encoded = VoiceEncoder::encode_frame(&frame);
        let decoded = VoiceEncoder::decode_frame(&encoded).unwrap();

        assert_eq!(decoded.sequence, frame.sequence);
        assert!((decoded.pitch - frame.pitch).abs() < 0.01);
        assert!((decoded.energy - frame.energy).abs() < 0.01);
    }
}
