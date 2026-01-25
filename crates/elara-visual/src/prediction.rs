//! Visual Prediction Engine - Continuity under packet loss
//!
//! This is the ELARA difference: when packets are lost, we PREDICT visual state
//! instead of freezing or showing artifacts. Reality continues.

use elara_core::StateTime;

use crate::{VisualState, VisualStateId};

/// Prediction configuration
#[derive(Debug, Clone)]
pub struct PredictionConfig {
    /// Maximum prediction horizon in milliseconds
    pub max_horizon_ms: u32,

    /// Confidence decay rate per 100ms
    pub confidence_decay: f32,

    /// Minimum confidence before prediction stops
    pub min_confidence: f32,

    /// Enable motion prediction
    pub predict_motion: bool,

    /// Enable expression prediction
    pub predict_expression: bool,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            max_horizon_ms: 500,
            confidence_decay: 0.1,
            min_confidence: 0.3,
            predict_motion: true,
            predict_expression: true,
        }
    }
}

/// Visual state predictor
#[derive(Debug)]
pub struct VisualPredictor {
    /// Configuration
    config: PredictionConfig,

    /// Last known good state
    last_state: Option<VisualState>,

    /// Previous state (for velocity estimation)
    prev_state: Option<VisualState>,

    /// Current prediction (if any)
    current_prediction: Option<VisualState>,

    /// How many consecutive predictions we've made
    prediction_count: u32,
}

impl VisualPredictor {
    /// Create a new predictor
    pub fn new(config: PredictionConfig) -> Self {
        Self {
            config,
            last_state: None,
            prev_state: None,
            current_prediction: None,
            prediction_count: 0,
        }
    }

    /// Update with a new received state
    pub fn update(&mut self, state: VisualState) {
        self.prev_state = self.last_state.take();
        self.last_state = Some(state);
        self.current_prediction = None;
        self.prediction_count = 0;
    }

    /// Get the current best state (received or predicted)
    pub fn current_state(&self) -> Option<&VisualState> {
        self.current_prediction
            .as_ref()
            .or(self.last_state.as_ref())
    }

    /// Predict state at a future time
    /// Returns None if prediction is not possible or confidence is too low
    pub fn predict(&mut self, target_time: StateTime) -> Option<VisualState> {
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
        let decay_steps = delta_ms as f32 / 100.0;
        let confidence = 1.0 - (decay_steps * self.config.confidence_decay);

        if confidence < self.config.min_confidence {
            return None;
        }

        // Create predicted state
        let mut predicted = last.clone();
        predicted.timestamp = target_time;
        predicted.sequence = last.sequence + 1;
        predicted.id = VisualStateId::new(predicted.sequence);

        // Predict face
        if self.config.predict_expression {
            if let (Some(ref prev), Some(ref mut face)) = (&self.prev_state, &mut predicted.face) {
                if let Some(ref prev_face) = prev.face {
                    // Predict mouth movement (for speech)
                    if face.speaking {
                        // Oscillate mouth openness for natural speech
                        let phase = (delta_ms as f32 / 150.0).sin();
                        face.mouth.openness = 0.3 + 0.2 * phase.abs();
                    }

                    // Predict head movement (smooth continuation)
                    let dt = delta_ms as f32 / 1000.0;
                    let head_vel = (
                        (last.face.as_ref().map(|f| f.head_rotation.0).unwrap_or(0.0)
                            - prev_face.head_rotation.0)
                            / 0.1,
                        (last.face.as_ref().map(|f| f.head_rotation.1).unwrap_or(0.0)
                            - prev_face.head_rotation.1)
                            / 0.1,
                        (last.face.as_ref().map(|f| f.head_rotation.2).unwrap_or(0.0)
                            - prev_face.head_rotation.2)
                            / 0.1,
                    );

                    face.head_rotation.0 += head_vel.0 * dt * 0.5; // Damped
                    face.head_rotation.1 += head_vel.1 * dt * 0.5;
                    face.head_rotation.2 += head_vel.2 * dt * 0.5;
                }

                // Reduce confidence
                face.confidence *= confidence;
            }
        }

        // Predict pose
        if self.config.predict_motion {
            if let Some(ref mut pose) = predicted.pose {
                // Use stored velocity for prediction
                let dt = delta_ms as f32 / 1000.0;

                for joint in &mut pose.joints {
                    joint.position.x += pose.velocity.x * dt;
                    joint.position.y += pose.velocity.y * dt;
                    joint.position.z += pose.velocity.z * dt;
                }

                pose.confidence *= confidence;
            }
        }

        // Scene doesn't need much prediction (relatively static)

        self.current_prediction = Some(predicted.clone());
        self.prediction_count += 1;

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

    /// Get estimated confidence of current state
    pub fn confidence(&self) -> f32 {
        if let Some(ref pred) = self.current_prediction {
            // Use face confidence as proxy
            pred.face.as_ref().map(|f| f.confidence).unwrap_or(0.5)
        } else if self.last_state.is_some() {
            1.0
        } else {
            0.0
        }
    }
}

/// Interpolation between two visual states
pub struct VisualInterpolator;

impl VisualInterpolator {
    /// Interpolate between two visual states
    pub fn interpolate(from: &VisualState, to: &VisualState, t: f32) -> VisualState {
        let t = t.clamp(0.0, 1.0);

        let mut result = to.clone();

        // Interpolate face
        result.face = match (&from.face, &to.face) {
            (Some(f1), Some(f2)) => Some(f1.lerp(f2, t)),
            (None, Some(f)) => Some(f.clone()),
            (Some(f), None) => {
                if t < 0.5 {
                    Some(f.clone())
                } else {
                    None
                }
            }
            (None, None) => None,
        };

        // Interpolate pose
        result.pose = match (&from.pose, &to.pose) {
            (Some(p1), Some(p2)) => Some(p1.lerp(p2, t)),
            (None, Some(p)) => Some(p.clone()),
            (Some(p), None) => {
                if t < 0.5 {
                    Some(p.clone())
                } else {
                    None
                }
            }
            (None, None) => None,
        };

        // Interpolate scene
        result.scene = match (&from.scene, &to.scene) {
            (Some(s1), Some(s2)) => Some(s1.lerp(s2, t)),
            (None, Some(s)) => Some(s.clone()),
            (Some(s), None) => {
                if t < 0.5 {
                    Some(s.clone())
                } else {
                    None
                }
            }
            (None, None) => None,
        };

        result
    }
}

/// Jitter buffer for visual states (NOT traditional jitter buffer)
/// This is a state buffer that enables smooth interpolation
#[derive(Debug)]
pub struct VisualStateBuffer {
    /// Buffered states
    states: Vec<VisualState>,

    /// Maximum buffer size
    max_size: usize,

    /// Target delay in milliseconds (for smoothing)
    target_delay_ms: u32,
}

impl VisualStateBuffer {
    /// Create a new buffer
    pub fn new(max_size: usize, target_delay_ms: u32) -> Self {
        Self {
            states: Vec::with_capacity(max_size),
            max_size,
            target_delay_ms,
        }
    }

    /// Add a state to the buffer
    pub fn push(&mut self, state: VisualState) {
        // Insert in order by timestamp
        let pos = self
            .states
            .iter()
            .position(|s| s.timestamp.as_millis() > state.timestamp.as_millis())
            .unwrap_or(self.states.len());

        self.states.insert(pos, state);

        // Remove old states if buffer is full
        while self.states.len() > self.max_size {
            self.states.remove(0);
        }
    }

    /// Get interpolated state at a specific time
    pub fn get_at(&self, time: StateTime) -> Option<VisualState> {
        if self.states.is_empty() {
            return None;
        }

        // Find surrounding states
        let target_ms = time.as_millis() - self.target_delay_ms as i64;

        let mut before: Option<&VisualState> = None;
        let mut after: Option<&VisualState> = None;

        for state in &self.states {
            if state.timestamp.as_millis() <= target_ms {
                before = Some(state);
            } else {
                after = Some(state);
                break;
            }
        }

        match (before, after) {
            (Some(b), Some(a)) => {
                // Interpolate
                let range = a.timestamp.as_millis() - b.timestamp.as_millis();
                if range <= 0 {
                    return Some(b.clone());
                }
                let t = (target_ms - b.timestamp.as_millis()) as f32 / range as f32;
                Some(VisualInterpolator::interpolate(b, a, t))
            }
            (Some(b), None) => Some(b.clone()),
            (None, Some(a)) => Some(a.clone()),
            (None, None) => None,
        }
    }

    /// Get the latest state
    pub fn latest(&self) -> Option<&VisualState> {
        self.states.last()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Number of buffered states
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Is buffer empty?
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elara_core::NodeId;

    #[test]
    fn test_predictor_update() {
        let mut predictor = VisualPredictor::new(PredictionConfig::default());

        let node = NodeId::new(1);
        let time = StateTime::from_millis(0);
        let state = VisualState::keyframe(node, time, 1);

        predictor.update(state);

        assert!(predictor.current_state().is_some());
        assert!(!predictor.is_predicting());
    }

    #[test]
    fn test_predictor_predict() {
        let mut predictor = VisualPredictor::new(PredictionConfig::default());

        let node = NodeId::new(1);
        let time1 = StateTime::from_millis(0);
        let time2 = StateTime::from_millis(100);

        predictor.update(VisualState::keyframe(node, time1, 1));
        predictor.update(VisualState::keyframe(node, time2, 2));

        let predicted = predictor.predict(StateTime::from_millis(200));
        assert!(predicted.is_some());
        assert!(predictor.is_predicting());
    }

    #[test]
    fn test_state_buffer() {
        let mut buffer = VisualStateBuffer::new(10, 50);

        let node = NodeId::new(1);

        buffer.push(VisualState::keyframe(node, StateTime::from_millis(0), 1));
        buffer.push(VisualState::keyframe(node, StateTime::from_millis(100), 2));
        buffer.push(VisualState::keyframe(node, StateTime::from_millis(200), 3));

        assert_eq!(buffer.len(), 3);

        // Get interpolated state at t=100 (with 50ms delay = t=50)
        let state = buffer.get_at(StateTime::from_millis(100));
        assert!(state.is_some());
    }
}
