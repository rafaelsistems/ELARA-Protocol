//! State reconciliation pipeline

use elara_core::{
    AuthorityScope, Event, EventResult, EventType, RejectReason, StateAtom, StateTime, StateType,
    TimePosition,
};
use elara_time::TimeEngine;

use crate::StateField;

/// Reconciliation result for a batch of events
#[derive(Debug, Default)]
pub struct ReconciliationResult {
    pub applied: u32,
    pub merged: u32,
    pub late_corrected: u32,
    pub buffered: u32,
    pub rejected: u32,
}

/// State reconciliation engine
pub struct ReconciliationEngine {
    /// State field
    field: StateField,
    /// Divergence threshold
    divergence_threshold: f64,
}

impl ReconciliationEngine {
    pub fn new() -> Self {
        ReconciliationEngine {
            field: StateField::new(),
            divergence_threshold: 0.5,
        }
    }

    /// Get reference to state field
    pub fn field(&self) -> &StateField {
        &self.field
    }

    /// Get mutable reference to state field
    pub fn field_mut(&mut self) -> &mut StateField {
        &mut self.field
    }

    /// Process a batch of events
    pub fn process_events(
        &mut self,
        events: Vec<Event>,
        time_engine: &TimeEngine,
    ) -> ReconciliationResult {
        let mut result = ReconciliationResult::default();

        for event in events {
            match self.process_single_event(event, time_engine) {
                EventResult::Applied => result.applied += 1,
                EventResult::Merged => result.merged += 1,
                EventResult::LateCorrected => result.late_corrected += 1,
                EventResult::Buffered => result.buffered += 1,
                EventResult::Duplicate => {}
                EventResult::Rejected(_) => result.rejected += 1,
            }
        }

        result
    }

    /// Process a single event through the reconciliation pipeline
    fn process_single_event(&mut self, event: Event, time_engine: &TimeEngine) -> EventResult {
        // Stage 1: Authority Check
        if !self.check_authority(&event) {
            return EventResult::Rejected(RejectReason::Unauthorized);
        }

        // Stage 2: Causality Check
        if !self.check_causality(&event) {
            // Quarantine for later
            self.field.quarantine(
                vec![], // Would serialize event here
                vec![event.target_state],
                time_engine.tau_s(),
            );
            return EventResult::Rejected(RejectReason::CausalityViolation);
        }

        // Special case: deletions should apply immediately to avoid stale atoms
        if matches!(event.mutation, elara_core::MutationOp::Delete) {
            self.apply_event(&event, time_engine.tau_s());
            return EventResult::Applied;
        }

        // Stage 3: Temporal Placement
        let τ_event = event.absolute_time(time_engine.tau_s());
        let position = time_engine.classify_time(τ_event);

        // Stage 4: Handle based on temporal position
        match position {
            TimePosition::TooLate => EventResult::Rejected(RejectReason::TooLate),
            TimePosition::Correctable => {
                self.apply_late_correction(&event, time_engine);
                EventResult::LateCorrected
            }
            TimePosition::Current => {
                self.apply_event(&event, time_engine.tau_s());
                EventResult::Applied
            }
            TimePosition::Predictable => {
                self.replace_prediction(&event, time_engine.tau_s());
                EventResult::Merged
            }
            TimePosition::TooEarly => {
                // Buffer for future
                EventResult::Buffered
            }
        }
    }

    /// Check if event source has authority over target state
    fn check_authority(&self, event: &Event) -> bool {
        if let Some(atom) = self.field.get(event.target_state) {
            atom.authority
                .has_authority(event.source, &AuthorityScope::Full)
        } else {
            // New state - source becomes owner
            true
        }
    }

    /// Check causality using version vectors
    fn check_causality(&self, event: &Event) -> bool {
        if let Some(atom) = self.field.get(event.target_state) {
            // Event's version ref should not be ahead of current version
            !event.version_ref.happens_before(&atom.version)
                || event.version_ref == atom.version
                || event.version_ref.concurrent(&atom.version)
        } else {
            // New state - always valid
            true
        }
    }

    /// Apply event directly to state
    fn apply_event(&mut self, event: &Event, now: StateTime) {
        if matches!(event.mutation, elara_core::MutationOp::Delete) {
            self.field.remove(event.target_state);
            return;
        }

        if let Some(atom) = self.field.get_mut(event.target_state) {
            atom.version = atom.version.merge(&event.version_ref);
            atom.version.increment(event.source);
            atom.last_modified = now;

            match &event.mutation {
                elara_core::MutationOp::Set(data) => {
                    atom.value = data.clone();
                }
                elara_core::MutationOp::Append(data) => {
                    atom.value.extend_from_slice(data);
                }
                _ => {}
            }

            atom.entropy.reset();
        } else {
            let state_type = Self::state_type_for_event(event.event_type);
            let mut atom = StateAtom::new(event.target_state, state_type, event.source);
            atom.version.increment(event.source);
            atom.last_modified = now;

            match &event.mutation {
                elara_core::MutationOp::Set(data) => {
                    atom.value = data.clone();
                }
                elara_core::MutationOp::Append(data) => {
                    atom.value.extend_from_slice(data);
                }
                _ => {}
            }

            self.field.insert(atom);
        }
    }

    fn state_type_for_event(event_type: EventType) -> StateType {
        match event_type {
            EventType::VoiceFrame
            | EventType::VoiceMute
            | EventType::PresenceUpdate
            | EventType::TypingStart
            | EventType::TypingStop
            | EventType::VisualKeyframe
            | EventType::VisualDelta => StateType::Perceptual,
            EventType::TextAppend
            | EventType::TextEdit
            | EventType::TextDelete
            | EventType::TextReact
            | EventType::FeedAppend
            | EventType::FeedDelete
            | EventType::StreamStart
            | EventType::StreamEnd => StateType::Core,
            _ => StateType::Core,
        }
    }

    /// Apply late correction with blending
    fn apply_late_correction(&mut self, event: &Event, time_engine: &TimeEngine) {
        let τ_event = event.absolute_time(time_engine.tau_s());
        let delay = time_engine.tau_s() - τ_event;
        let weight = time_engine.correction_weight(delay);

        if weight > 0.1 {
            // Apply with reduced weight
            self.apply_event(event, time_engine.tau_s());
        }
    }

    /// Replace prediction with actual data
    fn replace_prediction(&mut self, event: &Event, now: StateTime) {
        self.apply_event(event, now);
    }

    /// Control divergence across all atoms
    pub fn control_divergence(&mut self) {
        for (_, atom) in self.field.atoms.iter_mut() {
            if atom.entropy.level > self.divergence_threshold {
                // Reduce detail based on state type
                match atom.state_type {
                    StateType::Enhancement | StateType::Cosmetic => {
                        // Can drop these entirely
                        atom.value.clear();
                    }
                    StateType::Perceptual => {
                        // Reduce precision
                        atom.entropy.increase(0.1);
                    }
                    StateType::Core => {
                        // Keep core, just mark uncertainty
                        atom.entropy.increase(0.05);
                    }
                }
            }
        }
    }
}

impl Default for ReconciliationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elara_core::{Event, EventType, MutationOp, NodeId, StateId};

    #[test]
    fn test_reconciliation_basic() {
        let mut engine = ReconciliationEngine::new();
        let time_engine = TimeEngine::new();

        let event = Event::new(
            NodeId::new(1),
            1,
            EventType::StateCreate,
            StateId::new(100),
            MutationOp::Set(vec![1, 2, 3]),
        );

        let result = engine.process_events(vec![event], &time_engine);
        assert_eq!(result.applied, 1);
        assert!(engine.field().contains(StateId::new(100)));
    }

    #[test]
    fn test_stream_start_end_creates_and_deletes_state() {
        let mut engine = ReconciliationEngine::new();
        let time_engine = TimeEngine::new();

        let state_id = StateId::new(200);
        let source = NodeId::new(7);

        let start_event = Event::new(
            source,
            1,
            EventType::StreamStart,
            state_id,
            MutationOp::Set(vec![9, 9, 9]),
        );

        let _ = engine.process_events(vec![start_event], &time_engine);
        assert!(engine.field().contains(state_id));

        let current_version = engine
            .field()
            .get(state_id)
            .expect("atom exists")
            .version
            .clone();
        let end_event = Event::new(
            source,
            2,
            EventType::StreamEnd,
            state_id,
            MutationOp::Delete,
        )
        .with_version(current_version);
        let _ = engine.process_events(vec![end_event], &time_engine);
        assert!(!engine.field().contains(state_id));
    }

    #[test]
    fn test_visual_keyframe_creates_perceptual_atom() {
        let mut engine = ReconciliationEngine::new();
        let time_engine = TimeEngine::new();

        let state_id = StateId::new(300);
        let source = NodeId::new(8);

        let visual_event = Event::new(
            source,
            1,
            EventType::VisualKeyframe,
            state_id,
            MutationOp::Set(vec![1, 2, 3, 4]),
        );

        let result = engine.process_events(vec![visual_event], &time_engine);
        assert_eq!(result.applied, 1);
        let atom = engine.field().get(state_id).expect("atom exists");
        assert_eq!(atom.state_type, StateType::Perceptual);
    }
}
