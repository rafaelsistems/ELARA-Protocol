//! State atom definitions
//!
//! State atoms (ω) are the fundamental units of reality in ELARA.
//! Each atom has identity, type, authority, versioning, and merge behavior.

use std::collections::{HashMap, HashSet};

use crate::{NodeId, StateId, StateTime, StateType};

/// Version vector for causal ordering (not total ordering)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionVector {
    clocks: HashMap<NodeId, u64>,
}

impl VersionVector {
    pub fn new() -> Self {
        VersionVector {
            clocks: HashMap::new(),
        }
    }

    /// Get the clock value for a node
    #[inline]
    pub fn get(&self, node: NodeId) -> u64 {
        self.clocks.get(&node).copied().unwrap_or(0)
    }

    /// Increment the clock for a node
    pub fn increment(&mut self, node: NodeId) {
        *self.clocks.entry(node).or_insert(0) += 1;
    }

    /// Set the clock for a node
    pub fn set(&mut self, node: NodeId, value: u64) {
        self.clocks.insert(node, value);
    }

    /// Check if self happens-before other
    pub fn happens_before(&self, other: &VersionVector) -> bool {
        if self == other {
            return false;
        }

        // self ≤ other for all nodes, and strictly < for at least one
        let mut strictly_less = false;

        for (node, &clock) in &self.clocks {
            let other_clock = other.get(*node);
            if clock > other_clock {
                return false;
            }
            if clock < other_clock {
                strictly_less = true;
            }
        }

        // Check nodes in other but not in self
        for (node, &clock) in &other.clocks {
            if !self.clocks.contains_key(node) && clock > 0 {
                strictly_less = true;
            }
        }

        strictly_less
    }

    /// Check if two version vectors are concurrent (neither happens-before)
    pub fn concurrent(&self, other: &VersionVector) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self != other
    }

    /// Merge two version vectors (element-wise max)
    pub fn merge(&self, other: &VersionVector) -> VersionVector {
        let mut merged = self.clocks.clone();

        for (node, &clock) in &other.clocks {
            merged
                .entry(*node)
                .and_modify(|c| *c = (*c).max(clock))
                .or_insert(clock);
        }

        VersionVector { clocks: merged }
    }

    /// Compact representation for wire format
    pub fn to_compact(&self) -> Vec<(NodeId, u64)> {
        self.clocks.iter().map(|(&n, &c)| (n, c)).collect()
    }

    /// Restore from compact representation
    pub fn from_compact(entries: Vec<(NodeId, u64)>) -> Self {
        VersionVector {
            clocks: entries.into_iter().collect(),
        }
    }
}

/// Authority set - who can mutate a state atom
#[derive(Clone, Debug, Default)]
pub struct AuthoritySet {
    /// Nodes with full authority
    pub owners: HashSet<NodeId>,
    /// Nodes with delegated authority (with scope)
    pub delegates: HashMap<NodeId, AuthorityScope>,
    /// Explicitly revoked nodes
    pub revoked: HashSet<NodeId>,
}

impl AuthoritySet {
    pub fn new() -> Self {
        AuthoritySet::default()
    }

    pub fn with_owner(owner: NodeId) -> Self {
        let mut set = AuthoritySet::new();
        set.owners.insert(owner);
        set
    }

    /// Check if a node has authority to perform an operation
    pub fn has_authority(&self, node: NodeId, operation: &AuthorityScope) -> bool {
        if self.revoked.contains(&node) {
            return false;
        }

        if self.owners.contains(&node) {
            return true;
        }

        if let Some(scope) = self.delegates.get(&node) {
            return scope.allows(operation);
        }

        false
    }

    /// Add an owner
    pub fn add_owner(&mut self, node: NodeId) {
        self.owners.insert(node);
        self.revoked.remove(&node);
    }

    /// Add a delegate with limited scope
    pub fn add_delegate(&mut self, node: NodeId, scope: AuthorityScope) {
        self.delegates.insert(node, scope);
        self.revoked.remove(&node);
    }

    /// Revoke authority from a node
    pub fn revoke(&mut self, node: NodeId) {
        self.owners.remove(&node);
        self.delegates.remove(&node);
        self.revoked.insert(node);
    }

    /// Check if a node is in the authority set (owner or delegate)
    pub fn contains(&self, node: &NodeId) -> bool {
        !self.revoked.contains(node) && (self.owners.contains(node) || self.delegates.contains_key(node))
    }

    /// Check if a node is revoked
    pub fn is_revoked(&self, node: &NodeId) -> bool {
        self.revoked.contains(node)
    }
}

/// Authority scope - what operations a delegate can perform
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorityScope {
    /// Full authority (same as owner)
    Full,
    /// Read and append only
    Append,
    /// Read only
    ReadOnly,
    /// Custom scope with specific operations
    Custom(HashSet<String>),
}

impl AuthorityScope {
    /// Check if this scope allows a given operation
    pub fn allows(&self, operation: &AuthorityScope) -> bool {
        match (self, operation) {
            (AuthorityScope::Full, _) => true,
            (AuthorityScope::Append, AuthorityScope::Append) => true,
            (AuthorityScope::Append, AuthorityScope::ReadOnly) => true,
            (AuthorityScope::ReadOnly, AuthorityScope::ReadOnly) => true,
            (AuthorityScope::Custom(allowed), AuthorityScope::Custom(requested)) => {
                requested.is_subset(allowed)
            }
            _ => false,
        }
    }
}

/// Delta law - how to merge conflicting mutations
#[derive(Clone, Debug)]
pub enum DeltaLaw {
    /// Last-writer-wins based on timestamp
    LastWriterWins,
    /// Append-only list (CRDT)
    AppendOnly { max_size: usize },
    /// Counter with specified merge strategy
    Counter { merge: CounterMerge },
    /// Multi-value register (keep all concurrent values)
    MultiValueRegister,
    /// Continuous blend for audio/motion
    ContinuousBlend {
        interpolation: InterpolationType,
        max_deviation: f64,
    },
}

impl Default for DeltaLaw {
    fn default() -> Self {
        DeltaLaw::LastWriterWins
    }
}

/// Counter merge strategy
#[derive(Clone, Copy, Debug)]
pub enum CounterMerge {
    Max,
    Sum,
    Average,
}

/// Interpolation type for continuous blending
#[derive(Clone, Copy, Debug)]
pub enum InterpolationType {
    Linear,
    Cubic,
    Catmull,
}

/// State bounds - constraints on state values
#[derive(Clone, Debug)]
pub struct StateBounds {
    /// Maximum size in bytes
    pub max_size: usize,
    /// Rate limit (events per second)
    pub rate_limit: Option<RateLimit>,
    /// Maximum entropy before compression
    pub max_entropy: f64,
}

impl Default for StateBounds {
    fn default() -> Self {
        StateBounds {
            max_size: 65536,
            rate_limit: None,
            max_entropy: 1.0,
        }
    }
}

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimit {
    pub max_events: u32,
    pub window_ms: u32,
}

impl RateLimit {
    pub fn new(max_events: u32, window_ms: u32) -> Self {
        RateLimit {
            max_events,
            window_ms,
        }
    }
}

/// Entropy model - tracks uncertainty/divergence
#[derive(Clone, Debug, Default)]
pub struct EntropyModel {
    /// Current entropy level (0.0 = certain, 1.0 = maximum uncertainty)
    pub level: f64,
    /// Accumulated entropy from predictions
    pub accumulated: f64,
    /// Time since last actual data
    pub time_since_actual: u64,
}

impl EntropyModel {
    pub fn new() -> Self {
        EntropyModel::default()
    }

    /// Increase entropy (during prediction)
    pub fn increase(&mut self, amount: f64) {
        self.level = (self.level + amount).min(1.0);
        self.accumulated += amount;
    }

    /// Decrease entropy (when actual data arrives)
    pub fn decrease(&mut self, amount: f64) {
        self.level = (self.level - amount).max(0.0);
    }

    /// Reset entropy (full actual data received)
    pub fn reset(&mut self) {
        self.level = 0.0;
        self.accumulated = 0.0;
        self.time_since_actual = 0;
    }
}

/// State atom - the fundamental unit of reality
#[derive(Clone, Debug)]
pub struct StateAtom {
    /// Unique identifier
    pub id: StateId,
    /// State type classification
    pub state_type: StateType,
    /// Authority set
    pub authority: AuthoritySet,
    /// Version vector for causal ordering
    pub version: VersionVector,
    /// Delta law for merging
    pub delta_law: DeltaLaw,
    /// Bounds and constraints
    pub bounds: StateBounds,
    /// Entropy/uncertainty model
    pub entropy: EntropyModel,
    /// Last modification time
    pub last_modified: StateTime,
    /// The actual value (opaque bytes for now)
    pub value: Vec<u8>,
}

impl StateAtom {
    pub fn new(id: StateId, state_type: StateType, owner: NodeId) -> Self {
        StateAtom {
            id,
            state_type,
            authority: AuthoritySet::with_owner(owner),
            version: VersionVector::new(),
            delta_law: DeltaLaw::default(),
            bounds: StateBounds::default(),
            entropy: EntropyModel::new(),
            last_modified: StateTime::ZERO,
            value: Vec::new(),
        }
    }

    /// Check if this atom needs prediction (no recent actual data)
    pub fn needs_prediction(&self, threshold_ms: u64) -> bool {
        self.entropy.time_since_actual > threshold_ms * 1000
    }

    /// Get memory size estimate
    pub fn memory_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.value.len()
            + self.authority.owners.len() * std::mem::size_of::<NodeId>()
            + self.version.clocks.len() * (std::mem::size_of::<NodeId>() + 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_vector_happens_before() {
        let mut v1 = VersionVector::new();
        v1.set(NodeId::new(1), 1);
        v1.set(NodeId::new(2), 2);

        let mut v2 = VersionVector::new();
        v2.set(NodeId::new(1), 1);
        v2.set(NodeId::new(2), 3);

        assert!(v1.happens_before(&v2));
        assert!(!v2.happens_before(&v1));
    }

    #[test]
    fn test_version_vector_concurrent() {
        let mut v1 = VersionVector::new();
        v1.set(NodeId::new(1), 2);
        v1.set(NodeId::new(2), 1);

        let mut v2 = VersionVector::new();
        v2.set(NodeId::new(1), 1);
        v2.set(NodeId::new(2), 2);

        assert!(v1.concurrent(&v2));
        assert!(v2.concurrent(&v1));
    }

    #[test]
    fn test_version_vector_merge() {
        let mut v1 = VersionVector::new();
        v1.set(NodeId::new(1), 2);
        v1.set(NodeId::new(2), 1);

        let mut v2 = VersionVector::new();
        v2.set(NodeId::new(1), 1);
        v2.set(NodeId::new(2), 3);

        let merged = v1.merge(&v2);
        assert_eq!(merged.get(NodeId::new(1)), 2);
        assert_eq!(merged.get(NodeId::new(2)), 3);
    }

    #[test]
    fn test_authority_set() {
        let owner = NodeId::new(1);
        let delegate = NodeId::new(2);
        let outsider = NodeId::new(3);

        let mut auth = AuthoritySet::with_owner(owner);
        auth.add_delegate(delegate, AuthorityScope::Append);

        assert!(auth.has_authority(owner, &AuthorityScope::Full));
        assert!(auth.has_authority(delegate, &AuthorityScope::Append));
        assert!(!auth.has_authority(delegate, &AuthorityScope::Full));
        assert!(!auth.has_authority(outsider, &AuthorityScope::Append));

        auth.revoke(delegate);
        assert!(!auth.has_authority(delegate, &AuthorityScope::Append));
    }
}
