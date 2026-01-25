//! Authority Model - Who can mutate what state
//!
//! In ELARA, authority determines who can change specific state atoms.
//! This is cryptographically enforced, not just policy.

use elara_core::NodeId;
use std::collections::HashSet;

/// Authority level for a state atom
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthorityLevel {
    /// Only one node can mutate (e.g., broadcaster's visual state)
    Exclusive,
    /// Multiple nodes can mutate with coordination (e.g., shared whiteboard)
    Shared,
    /// Any interested node can mutate (e.g., chat messages)
    Open,
    /// No one can mutate (read-only, historical)
    Frozen,
}

/// Authority proof - cryptographic evidence of mutation rights
#[derive(Debug, Clone)]
pub struct AuthorityProof {
    /// The node claiming authority
    pub claimer: NodeId,
    /// The state being claimed
    pub state_id: u64,
    /// Signature over (claimer, state_id, timestamp)
    pub signature: Vec<u8>,
    /// Timestamp of the claim
    pub timestamp: i64,
}

impl AuthorityProof {
    /// Create a new authority proof (signature would be computed externally)
    pub fn new(claimer: NodeId, state_id: u64, timestamp: i64) -> Self {
        Self {
            claimer,
            state_id,
            signature: Vec::new(), // Would be filled by crypto layer
            timestamp,
        }
    }

    /// Check if this proof is for a specific state
    pub fn is_for_state(&self, state_id: u64) -> bool {
        self.state_id == state_id
    }
}

/// Authority set - who has authority over a state
#[derive(Debug, Clone)]
pub struct AuthoritySet {
    /// The state this authority set governs
    pub state_id: u64,

    /// Authority level
    pub level: AuthorityLevel,

    /// Nodes with authority (empty for Open level)
    pub authorities: HashSet<NodeId>,

    /// Delegation chain (who granted authority to whom)
    pub delegations: Vec<(NodeId, NodeId)>,
}

impl AuthoritySet {
    /// Create an exclusive authority set (single owner)
    pub fn exclusive(state_id: u64, owner: NodeId) -> Self {
        let mut authorities = HashSet::new();
        authorities.insert(owner);

        Self {
            state_id,
            level: AuthorityLevel::Exclusive,
            authorities,
            delegations: Vec::new(),
        }
    }

    /// Create a shared authority set
    pub fn shared(state_id: u64, owners: Vec<NodeId>) -> Self {
        Self {
            state_id,
            level: AuthorityLevel::Shared,
            authorities: owners.into_iter().collect(),
            delegations: Vec::new(),
        }
    }

    /// Create an open authority set (anyone can mutate)
    pub fn open(state_id: u64) -> Self {
        Self {
            state_id,
            level: AuthorityLevel::Open,
            authorities: HashSet::new(),
            delegations: Vec::new(),
        }
    }

    /// Create a frozen authority set (no mutations)
    pub fn frozen(state_id: u64) -> Self {
        Self {
            state_id,
            level: AuthorityLevel::Frozen,
            authorities: HashSet::new(),
            delegations: Vec::new(),
        }
    }

    /// Check if a node has authority
    pub fn has_authority(&self, node: NodeId) -> bool {
        match self.level {
            AuthorityLevel::Exclusive | AuthorityLevel::Shared => self.authorities.contains(&node),
            AuthorityLevel::Open => true,
            AuthorityLevel::Frozen => false,
        }
    }

    /// Add authority for a node (for shared level)
    pub fn grant(&mut self, granter: NodeId, grantee: NodeId) -> bool {
        if self.level != AuthorityLevel::Shared {
            return false;
        }

        if !self.authorities.contains(&granter) {
            return false;
        }

        self.authorities.insert(grantee);
        self.delegations.push((granter, grantee));
        true
    }

    /// Remove authority from a node
    pub fn revoke(&mut self, revoker: NodeId, revokee: NodeId) -> bool {
        if self.level != AuthorityLevel::Shared {
            return false;
        }

        if !self.authorities.contains(&revoker) {
            return false;
        }

        // Can't revoke from yourself if you're the last one
        if self.authorities.len() == 1 && self.authorities.contains(&revokee) {
            return false;
        }

        self.authorities.remove(&revokee);
        true
    }
}

/// Livestream authority model
#[derive(Debug, Clone)]
pub struct LivestreamAuthority {
    /// The broadcaster (exclusive authority over visual/audio)
    pub broadcaster: NodeId,

    /// Visual state authority
    pub visual_authority: AuthoritySet,

    /// Audio state authority
    pub audio_authority: AuthoritySet,

    /// Chat state authority (open to all viewers)
    pub chat_authority: AuthoritySet,

    /// Moderators (can mute viewers in chat)
    pub moderators: HashSet<NodeId>,
}

impl LivestreamAuthority {
    /// Create a new livestream authority model
    pub fn new(broadcaster: NodeId, stream_id: u64) -> Self {
        Self {
            broadcaster,
            visual_authority: AuthoritySet::exclusive(stream_id, broadcaster),
            audio_authority: AuthoritySet::exclusive(stream_id + 1, broadcaster),
            chat_authority: AuthoritySet::open(stream_id + 2),
            moderators: HashSet::new(),
        }
    }

    /// Add a moderator
    pub fn add_moderator(&mut self, moderator: NodeId) {
        self.moderators.insert(moderator);
    }

    /// Check if a node can mutate visual state
    pub fn can_mutate_visual(&self, node: NodeId) -> bool {
        self.visual_authority.has_authority(node)
    }

    /// Check if a node can mutate audio state
    pub fn can_mutate_audio(&self, node: NodeId) -> bool {
        self.audio_authority.has_authority(node)
    }

    /// Check if a node can send chat
    pub fn can_chat(&self, node: NodeId) -> bool {
        self.chat_authority.has_authority(node)
    }

    /// Check if a node is a moderator
    pub fn is_moderator(&self, node: NodeId) -> bool {
        self.moderators.contains(&node) || node == self.broadcaster
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclusive_authority() {
        let owner = NodeId::new(1);
        let other = NodeId::new(2);
        let auth = AuthoritySet::exclusive(100, owner);

        assert!(auth.has_authority(owner));
        assert!(!auth.has_authority(other));
    }

    #[test]
    fn test_shared_authority() {
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);
        let node3 = NodeId::new(3);

        let mut auth = AuthoritySet::shared(100, vec![node1, node2]);

        assert!(auth.has_authority(node1));
        assert!(auth.has_authority(node2));
        assert!(!auth.has_authority(node3));

        // Grant authority to node3
        assert!(auth.grant(node1, node3));
        assert!(auth.has_authority(node3));
    }

    #[test]
    fn test_open_authority() {
        let auth = AuthoritySet::open(100);

        assert!(auth.has_authority(NodeId::new(1)));
        assert!(auth.has_authority(NodeId::new(999)));
    }

    #[test]
    fn test_frozen_authority() {
        let auth = AuthoritySet::frozen(100);

        assert!(!auth.has_authority(NodeId::new(1)));
        assert!(!auth.has_authority(NodeId::new(999)));
    }

    #[test]
    fn test_livestream_authority() {
        let broadcaster = NodeId::new(1);
        let viewer = NodeId::new(2);

        let auth = LivestreamAuthority::new(broadcaster, 1000);

        assert!(auth.can_mutate_visual(broadcaster));
        assert!(!auth.can_mutate_visual(viewer));

        assert!(auth.can_mutate_audio(broadcaster));
        assert!(!auth.can_mutate_audio(viewer));

        // Both can chat
        assert!(auth.can_chat(broadcaster));
        assert!(auth.can_chat(viewer));
    }
}
