//! State field - collection of state atoms

use std::collections::HashMap;

use elara_core::{NodeId, StateAtom, StateId, StateTime, StateType};

/// State field - the local reality
#[derive(Debug, Default)]
pub struct StateField {
    /// State atoms indexed by ID
    pub atoms: HashMap<StateId, StateAtom>,
    /// Quarantined events (awaiting dependencies)
    quarantine: Vec<QuarantinedEvent>,
}

/// Event waiting for dependencies
#[derive(Debug)]
pub struct QuarantinedEvent {
    pub event_data: Vec<u8>,
    pub missing_deps: Vec<StateId>,
    pub quarantined_at: StateTime,
}

impl StateField {
    pub fn new() -> Self {
        StateField::default()
    }

    /// Get a state atom by ID
    pub fn get(&self, id: StateId) -> Option<&StateAtom> {
        self.atoms.get(&id)
    }

    /// Get a mutable state atom by ID
    pub fn get_mut(&mut self, id: StateId) -> Option<&mut StateAtom> {
        self.atoms.get_mut(&id)
    }

    /// Insert or update a state atom
    pub fn insert(&mut self, atom: StateAtom) {
        self.atoms.insert(atom.id, atom);
    }

    /// Remove a state atom
    pub fn remove(&mut self, id: StateId) -> Option<StateAtom> {
        self.atoms.remove(&id)
    }

    /// Check if a state atom exists
    pub fn contains(&self, id: StateId) -> bool {
        self.atoms.contains_key(&id)
    }

    /// Get number of atoms
    pub fn len(&self) -> usize {
        self.atoms.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }

    /// Iterate over all atoms
    pub fn iter(&self) -> impl Iterator<Item = (&StateId, &StateAtom)> {
        self.atoms.iter()
    }

    /// Iterate over atoms of a specific type
    pub fn iter_by_type(&self, state_type: StateType) -> impl Iterator<Item = &StateAtom> {
        self.atoms.values().filter(move |a| a.state_type == state_type)
    }

    /// Get atoms needing prediction (high entropy or no recent data)
    pub fn atoms_needing_prediction(&self, threshold_ms: u64) -> Vec<StateId> {
        self.atoms
            .iter()
            .filter(|(_, atom)| atom.needs_prediction(threshold_ms))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Add event to quarantine
    pub fn quarantine(&mut self, event_data: Vec<u8>, missing_deps: Vec<StateId>, now: StateTime) {
        self.quarantine.push(QuarantinedEvent {
            event_data,
            missing_deps,
            quarantined_at: now,
        });
    }

    /// Get quarantined events that can now be processed
    pub fn release_quarantine(&mut self) -> Vec<Vec<u8>> {
        let atoms = &self.atoms;
        let (ready, still_waiting): (Vec<_>, Vec<_>) = self
            .quarantine
            .drain(..)
            .partition(|e| e.missing_deps.iter().all(|dep| atoms.contains_key(dep)));

        self.quarantine = still_waiting;
        ready.into_iter().map(|e| e.event_data).collect()
    }

    /// Calculate total memory usage
    pub fn memory_size(&self) -> usize {
        self.atoms.values().map(|a| a.memory_size()).sum()
    }

    /// Create a state atom for a node
    pub fn create_atom(&mut self, id: StateId, state_type: StateType, owner: NodeId) -> &mut StateAtom {
        let atom = StateAtom::new(id, state_type, owner);
        self.atoms.insert(id, atom);
        self.atoms.get_mut(&id).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_field_basic() {
        let mut field = StateField::new();
        let owner = NodeId::new(1);
        let id = StateId::new(100);

        field.create_atom(id, StateType::Core, owner);

        assert!(field.contains(id));
        assert_eq!(field.len(), 1);

        let atom = field.get(id).unwrap();
        assert_eq!(atom.state_type, StateType::Core);
    }

    #[test]
    fn test_state_field_iter_by_type() {
        let mut field = StateField::new();
        let owner = NodeId::new(1);

        field.create_atom(StateId::new(1), StateType::Core, owner);
        field.create_atom(StateId::new(2), StateType::Perceptual, owner);
        field.create_atom(StateId::new(3), StateType::Core, owner);

        let core_atoms: Vec<_> = field.iter_by_type(StateType::Core).collect();
        assert_eq!(core_atoms.len(), 2);
    }
}
