//! Voice Prediction - Continuity under packet loss
//!
//! When voice packets are lost, we PREDICT the voice state
//! instead of producing silence or glitches.

use elara_core::StateTime;
use crate::{VoiceFrame, VoiceState, VoiceStateId, VoiceActivity, VoiceParams};

/// Prediction configuration
#[derive(Debug, Clone)]
pub struct VoicePredictionConfig {
    /// Maximum prediction horizon in milliseconds
    pub max_horizon_ms: u32,
    
    /// Confidence decay rate per frame
    pub confidence_decay: f32,
    
    /// Minimum confidence before prediction stops
    pub min_confidence: f32,
    
    /// Fade out duration in frames when stopping prediction
    pub fade_out_frames: u32,
}

impl Default for VoicePredictionConfig {
    fn default() -> Self {
        Self {
            max_horizon_ms: 200,
            confidence_decay: 0.05,
            min_confidence: 0.3,
            fade_out_frames: 5,
        }
    }
}

/// Voice predictor
#[derive(Debug)]
pub struct VoicePredictor {
    /// Configuration
    config: VoicePredictionConfig,
    
    /// Last known good state
    last_state: Option<VoiceState>,
    
    /// Previous state (for velocity estimation)
    prev_state: Option<VoiceState>,
    
    /// Last few frames for pattern analysis
    recent_frames: Vec<VoiceFrame>,
    
    /// Current prediction (if any)
    current_prediction: Option<VoiceState>,
    
    /// How many consecutive predictions we've made
    prediction_count: u32,
    
    /// Estimated pitch velocity (Hz per frame)
    pitch_velocity: f32,
    
    /// Estimated energy velocity
    energy_velocity: f32,
}

impl VoicePredictor {
    /// Create a new predictor
    pub fn new(config: VoicePredictionConfig) -> Self {
        Self {
            config,
            last_state: None,
            prev_state: None,
            recent_frames: Vec::with_capacity(10),
            current_prediction: None,
            prediction_count: 0,
            pitch_velocity: 0.0,
            energy_velocity: 0.0,
        }
    }
    
    /// Update with a new received state
    pub fn update_state(&mut self, state: VoiceState) {
        // Estimate velocities from state transition
        if let (Some(ref prev), Some(ref prev_params), Some(ref curr_params)) = 
            (&self.last_state, self.last_state.as_ref().and_then(|s| s.params.as_ref()), state.params.as_ref()) 
        {
            let dt = (state.timestamp.as_millis() - prev.timestamp.as_millis()) as f32;
            if dt > 0.0 {
                self.pitch_velocity = (curr_params.pitch - prev_params.pitch) / dt * 20.0; // per frame
                self.energy_velocity = (curr_params.energy - prev_params.energy) / dt * 20.0;
            }
        }
        
        self.prev_state = self.last_state.take();
        self.last_state = Some(state);
        self.current_prediction = None;
        self.prediction_count = 0;
    }
    
    /// Update with a new received frame
    pub fn update_frame(&mut self, frame: VoiceFrame) {
        // Estimate velocities
        if let Some(last) = self.recent_frames.last() {
            self.pitch_velocity = frame.pitch - last.pitch;
            self.energy_velocity = frame.energy - last.energy;
        }
        
        self.recent_frames.push(frame);
        
        // Keep only recent frames
        while self.recent_frames.len() > 10 {
            self.recent_frames.remove(0);
        }
        
        self.prediction_count = 0;
    }
    
    /// Get the current best state (received or predicted)
    pub fn current_state(&self) -> Option<&VoiceState> {
        self.current_prediction.as_ref()
            .or(self.last_state.as_ref())
    }
    
    /// Predict state at a future time
    pub fn predict(&mut self, target_time: StateTime) -> Option<VoiceState> {
        let last = self.last_state.as_ref()?;
        
        let delta_ms = target_time.as_millis() - last.timestamp.as_millis();
        
        // Don't predict backwards
        if delta_ms <= 0 {
            return Some(last.clone());
        }
        
        // Don't predict beyond horizon
        if delta_ms > self.config.max_horizon_ms as i64 {
            return None;
        }
        
        // Calculate confidence decay
        let frames_ahead = delta_ms as f32 / 20.0; // 20ms per frame
        let confidence = 1.0 - (frames_ahead * self.config.confidence_decay);
        
        if confidence < self.config.min_confidence {
            return None;
        }
        
        // Create predicted state
        let mut predicted = last.clone();
        predicted.timestamp = target_time;
        predicted.sequence = last.sequence + 1;
        predicted.id = VoiceStateId::new(predicted.sequence);
        predicted.confidence = confidence;
        
        // Predict voice parameters
        if let Some(ref mut params) = predicted.params {
            // Apply velocity-based prediction
            params.pitch = (params.pitch + self.pitch_velocity * frames_ahead)
                .clamp(50.0, 500.0);
            params.energy = (params.energy + self.energy_velocity * frames_ahead)
                .clamp(0.0, 1.0);
            
            // Add slight randomness for naturalness
            params.pitch += (rand::random::<f32>() - 0.5) * 5.0;
        }
        
        // Fade out if predicting for too long
        if self.prediction_count > self.config.fade_out_frames {
            let fade = 1.0 - (self.prediction_count - self.config.fade_out_frames) as f32 
                / self.config.fade_out_frames as f32;
            if let Some(ref mut params) = predicted.params {
                params.energy *= fade.max(0.0);
            }
        }
        
        self.current_prediction = Some(predicted.clone());
        self.prediction_count += 1;
        
        Some(predicted)
    }
    
    /// Predict next frame
    pub fn predict_frame(&mut self, target_time: StateTime) -> Option<VoiceFrame> {
        if self.recent_frames.is_empty() {
            return None;
        }
        
        let last = self.recent_frames.last()?;
        
        let delta_ms = target_time.as_millis() - last.timestamp.as_millis();
        if delta_ms > self.config.max_horizon_ms as i64 {
            return None;
        }
        
        let frames_ahead = delta_ms as f32 / 20.0;
        let confidence = 1.0 - (frames_ahead * self.config.confidence_decay);
        
        if confidence < self.config.min_confidence {
            return None;
        }
        
        // Predict based on recent pattern
        let mut predicted = last.clone();
        predicted.timestamp = target_time;
        predicted.sequence = last.sequence + 1;
        
        // Apply velocity
        predicted.pitch = (predicted.pitch + self.pitch_velocity * frames_ahead)
            .clamp(50.0, 500.0);
        predicted.energy = (predicted.energy + self.energy_velocity * frames_ahead)
            .clamp(0.0, 1.0);
        
        // Fade out if predicting too long
        self.prediction_count += 1;
        if self.prediction_count > self.config.fade_out_frames {
            let fade = 1.0 - (self.prediction_count - self.config.fade_out_frames) as f32 
                / self.config.fade_out_frames as f32;
            predicted.energy *= fade.max(0.0);
        }
        
        Some(predicted)
    }
    
    /// Check if we're currently in prediction mode
    pub fn is_predicting(&self) -> bool {
        self.prediction_count > 0
    }
    
    /// Get prediction count
    pub fn prediction_count(&self) -> u32 {
        self.prediction_count
    }
    
    /// Get estimated confidence
    pub fn confidence(&self) -> f32 {
        if let Some(ref pred) = self.current_prediction {
            pred.confidence
        } else if self.last_state.is_some() {
            1.0
        } else {
            0.0
        }
    }
    
    /// Reset predictor
    pub fn reset(&mut self) {
        self.last_state = None;
        self.prev_state = None;
        self.recent_frames.clear();
        self.current_prediction = None;
        self.prediction_count = 0;
        self.pitch_velocity = 0.0;
        self.energy_velocity = 0.0;
    }
}

/// Packet loss concealment
#[derive(Debug)]
pub struct PacketLossConcealer {
    /// Predictor
    predictor: VoicePredictor,
    
    /// Last good frame
    last_good_frame: Option<VoiceFrame>,
    
    /// Consecutive lost packets
    lost_count: u32,
    
    /// Maximum concealment duration in frames
    max_conceal_frames: u32,
}

impl PacketLossConcealer {
    /// Create a new concealer
    pub fn new(max_conceal_frames: u32) -> Self {
        Self {
            predictor: VoicePredictor::new(VoicePredictionConfig::default()),
            last_good_frame: None,
            lost_count: 0,
            max_conceal_frames,
        }
    }
    
    /// Process a received frame
    pub fn receive(&mut self, frame: VoiceFrame) {
        self.predictor.update_frame(frame.clone());
        self.last_good_frame = Some(frame);
        self.lost_count = 0;
    }
    
    /// Conceal a lost frame
    pub fn conceal(&mut self, expected_time: StateTime) -> Option<VoiceFrame> {
        self.lost_count += 1;
        
        if self.lost_count > self.max_conceal_frames {
            // Too many lost, return silence
            return self.last_good_frame.as_ref().map(|f| {
                let mut silent = f.clone();
                silent.timestamp = expected_time;
                silent.activity = VoiceActivity::Silent;
                silent.energy = 0.0;
                silent
            });
        }
        
        self.predictor.predict_frame(expected_time)
    }
    
    /// Get lost packet count
    pub fn lost_count(&self) -> u32 {
        self.lost_count
    }
    
    /// Reset concealer
    pub fn reset(&mut self) {
        self.predictor.reset();
        self.last_good_frame = None;
        self.lost_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elara_core::NodeId;
    
    #[test]
    fn test_voice_predictor() {
        let mut predictor = VoicePredictor::new(VoicePredictionConfig::default());
        
        let node = NodeId::new(1);
        let params = VoiceParams::default();
        
        // Add initial state
        let state1 = VoiceState::speaking(node, StateTime::from_millis(0), 1, params.clone());
        predictor.update_state(state1);
        
        // Add second state
        let state2 = VoiceState::speaking(node, StateTime::from_millis(20), 2, params);
        predictor.update_state(state2);
        
        // Predict future
        let predicted = predictor.predict(StateTime::from_millis(40));
        assert!(predicted.is_some());
        assert!(predictor.is_predicting());
    }
    
    #[test]
    fn test_packet_loss_concealer() {
        let mut concealer = PacketLossConcealer::new(10);
        
        let node = NodeId::new(1);
        
        // Receive some frames
        for i in 0..5 {
            let frame = VoiceFrame::voiced(node, StateTime::from_millis(i * 20), i as u64, 120.0, 0.5);
            concealer.receive(frame);
        }
        
        // Conceal lost frame
        let concealed = concealer.conceal(StateTime::from_millis(100));
        assert!(concealed.is_some());
        assert_eq!(concealer.lost_count(), 1);
    }
}
