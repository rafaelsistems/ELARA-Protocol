//! Voice Synthesis - Reconstruct voice from parametric state
//!
//! This is a simplified synthesis model for demonstration.
//! Real implementation would use neural vocoder or LPC synthesis.

use crate::{Formant, VoiceActivity, VoiceFrame, VoiceParams};

/// Synthesis configuration
#[derive(Debug, Clone)]
pub struct SynthesisConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Frame size in samples
    pub frame_size: usize,

    /// Enable formant synthesis
    pub use_formants: bool,

    /// Enable noise component
    pub use_noise: bool,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            frame_size: 320, // 20ms at 16kHz
            use_formants: true,
            use_noise: true,
        }
    }
}

/// Voice synthesizer
#[derive(Debug)]
pub struct VoiceSynthesizer {
    /// Configuration
    config: SynthesisConfig,

    /// Current phase for oscillator
    phase: f32,

    /// Noise generator state
    noise_state: u32,

    /// Last output for smoothing
    last_output: f32,
}

impl VoiceSynthesizer {
    /// Create a new synthesizer
    pub fn new(config: SynthesisConfig) -> Self {
        Self {
            config,
            phase: 0.0,
            noise_state: 12345,
            last_output: 0.0,
        }
    }

    /// Synthesize audio samples from voice parameters
    pub fn synthesize_params(&mut self, params: &VoiceParams) -> Vec<f32> {
        let mut samples = Vec::with_capacity(self.config.frame_size);

        let pitch_inc = params.pitch / self.config.sample_rate as f32;

        for _ in 0..self.config.frame_size {
            let mut sample = 0.0;

            // Generate source signal
            if params.voicing > 0.5 {
                // Voiced: pulse train approximation
                sample = self.generate_pulse(pitch_inc);
            }

            // Add noise component
            if self.config.use_noise {
                let noise = self.generate_noise() * params.breathiness;
                sample = sample * (1.0 - params.breathiness) + noise;
            }

            // Apply formant filtering (simplified)
            if self.config.use_formants {
                sample = self.apply_formants(sample, &params.formants);
            }

            // Apply energy
            sample *= params.energy;

            // Smooth output
            sample = self.last_output * 0.1 + sample * 0.9;
            self.last_output = sample;

            samples.push(sample);
        }

        samples
    }

    /// Synthesize from a voice frame
    pub fn synthesize_frame(&mut self, frame: &VoiceFrame) -> Vec<f32> {
        if !frame.voiced || frame.energy < 0.01 {
            // Return silence
            return vec![0.0; self.config.frame_size];
        }

        // Create simplified params from frame
        let params = VoiceParams {
            pitch: frame.pitch,
            pitch_variation: 0.0,
            energy: frame.energy,
            formants: self.spectral_index_to_formants(frame.spectral_index),
            voicing: if frame.voiced { 1.0 } else { 0.0 },
            rate: 1.0,
            contour: crate::PitchContour::Flat,
            emotion: crate::SpeechEmotion::Neutral,
            breathiness: 0.1,
            nasality: 0.1,
        };

        self.synthesize_params(&params)
    }

    /// Generate pulse train for voiced speech
    fn generate_pulse(&mut self, pitch_inc: f32) -> f32 {
        self.phase += pitch_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Simple sawtooth approximation
        let saw = 2.0 * self.phase - 1.0;

        // Shape into pulse-like waveform
        saw - saw.powi(3) / 3.0
    }

    /// Generate white noise
    fn generate_noise(&mut self) -> f32 {
        // Simple LCG noise generator
        self.noise_state = self
            .noise_state
            .wrapping_mul(1103515245)
            .wrapping_add(12345);
        (self.noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Apply formant filtering (very simplified)
    fn apply_formants(&self, input: f32, formants: &[Formant; 4]) -> f32 {
        // Simplified: just weight by formant amplitudes
        // Real implementation would use resonant filters
        let mut output = input;

        for formant in formants {
            // Approximate resonance boost
            let boost = formant.amplitude * 0.5;
            output *= 1.0 + boost;
        }

        output.clamp(-1.0, 1.0)
    }

    /// Convert spectral index back to formants
    fn spectral_index_to_formants(&self, index: u16) -> [Formant; 4] {
        let f1_ratio = (index & 0xFF) as f32 / 255.0;
        let f2_ratio = ((index >> 8) & 0xFF) as f32 / 255.0;

        [
            Formant::new(f1_ratio * 1000.0, 100.0, 1.0),
            Formant::new(f2_ratio * 3000.0, 150.0, 0.8),
            Formant::new(2500.0, 200.0, 0.6),
            Formant::new(3500.0, 250.0, 0.4),
        ]
    }

    /// Reset synthesizer state
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.noise_state = 12345;
        self.last_output = 0.0;
    }
}

/// Voice activity detector (simplified)
#[derive(Debug)]
pub struct VoiceActivityDetector {
    /// Energy threshold
    threshold: f32,

    /// Smoothed energy
    smoothed_energy: f32,

    /// Hangover counter (frames to stay active after speech)
    hangover: u32,

    /// Current hangover count
    hangover_count: u32,
}

impl VoiceActivityDetector {
    /// Create a new VAD
    pub fn new(threshold: f32, hangover: u32) -> Self {
        Self {
            threshold,
            smoothed_energy: 0.0,
            hangover,
            hangover_count: 0,
        }
    }

    /// Process a frame and return activity
    pub fn process(&mut self, samples: &[f32]) -> VoiceActivity {
        // Calculate frame energy
        let energy: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

        // Smooth energy
        self.smoothed_energy = self.smoothed_energy * 0.9 + energy * 0.1;

        if self.smoothed_energy > self.threshold {
            self.hangover_count = self.hangover;
            VoiceActivity::Speaking
        } else if self.hangover_count > 0 {
            self.hangover_count -= 1;
            VoiceActivity::Speaking
        } else {
            VoiceActivity::Silent
        }
    }

    /// Get current energy level
    pub fn energy(&self) -> f32 {
        self.smoothed_energy
    }

    /// Reset VAD
    pub fn reset(&mut self) {
        self.smoothed_energy = 0.0;
        self.hangover_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesizer() {
        let config = SynthesisConfig::default();
        let mut synth = VoiceSynthesizer::new(config);

        let params = VoiceParams::new();
        let samples = synth.synthesize_params(&params);

        assert_eq!(samples.len(), 320);

        // Check samples are in valid range
        for s in &samples {
            assert!(*s >= -1.0 && *s <= 1.0);
        }
    }

    #[test]
    fn test_vad() {
        let mut vad = VoiceActivityDetector::new(0.01, 5);

        // Silent frame
        let silent = vec![0.0; 320];
        assert_eq!(vad.process(&silent), VoiceActivity::Silent);

        // Active frame
        let active: Vec<f32> = (0..320).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        assert_eq!(vad.process(&active), VoiceActivity::Speaking);
    }
}
