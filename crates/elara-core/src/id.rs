//! Identity types for ELARA protocol
//!
//! All identifiers are 64-bit for wire efficiency while maintaining
//! sufficient uniqueness for practical swarm sizes.

use std::fmt;

/// Node identity - cryptographic fingerprint (truncated hash of public key)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeId(pub u64);

impl NodeId {
    pub const ZERO: NodeId = NodeId(0);

    #[inline]
    pub fn new(id: u64) -> Self {
        NodeId(id)
    }

    #[inline]
    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    #[inline]
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        NodeId(u64::from_le_bytes(bytes))
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({:016x})", self.0)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Session identity - shared reality space binding
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SessionId(pub u64);

impl SessionId {
    pub const ZERO: SessionId = SessionId(0);

    #[inline]
    pub fn new(id: u64) -> Self {
        SessionId(id)
    }

    #[inline]
    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    #[inline]
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        SessionId(u64::from_le_bytes(bytes))
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Session({:016x})", self.0)
    }
}

/// State atom identity - unique within a session
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StateId(pub u64);

impl StateId {
    pub const ZERO: StateId = StateId(0);

    #[inline]
    pub fn new(id: u64) -> Self {
        StateId(id)
    }

    /// Create a state ID from type prefix and instance ID
    /// Format: \[type:16\]\[instance:48\]
    #[inline]
    pub fn from_type_instance(state_type: u16, instance: u64) -> Self {
        let id = ((state_type as u64) << 48) | (instance & 0x0000_FFFF_FFFF_FFFF);
        StateId(id)
    }

    #[inline]
    pub fn state_type(self) -> u16 {
        (self.0 >> 48) as u16
    }

    #[inline]
    pub fn instance(self) -> u64 {
        self.0 & 0x0000_FFFF_FFFF_FFFF
    }

    #[inline]
    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    #[inline]
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        StateId(u64::from_le_bytes(bytes))
    }
}

impl fmt::Debug for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "State({:04x}:{:012x})",
            self.state_type(),
            self.instance()
        )
    }
}

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:012x}", self.state_type(), self.instance())
    }
}

/// Event identity - unique within a session, used for causal ordering
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EventId {
    pub node: NodeId,
    pub seq: u64,
}

impl EventId {
    #[inline]
    pub fn new(node: NodeId, seq: u64) -> Self {
        EventId { node, seq }
    }
}

impl fmt::Debug for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event({:016x}:{})", self.node.0, self.seq)
    }
}

/// Message identity for text streams
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MessageId(pub u64);

impl MessageId {
    #[inline]
    pub fn new(id: u64) -> Self {
        MessageId(id)
    }

    #[inline]
    pub fn from_event(event_id: &EventId) -> Self {
        // Combine node and seq into message ID
        let id = (event_id.node.0 ^ event_id.seq).wrapping_mul(0x517cc1b727220a95);
        MessageId(id)
    }
}

impl fmt::Debug for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Msg({:016x})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_roundtrip() {
        let id = NodeId::new(0xDEADBEEF_CAFEBABE);
        let bytes = id.to_bytes();
        let recovered = NodeId::from_bytes(bytes);
        assert_eq!(id, recovered);
    }

    #[test]
    fn test_state_id_type_instance() {
        let state_type = 0x0001; // e.g., text stream
        let instance = 0x0000_1234_5678_9ABC;
        let id = StateId::from_type_instance(state_type, instance);

        assert_eq!(id.state_type(), state_type);
        assert_eq!(id.instance(), instance);
    }

    #[test]
    fn test_state_id_instance_truncation() {
        // Instance should be truncated to 48 bits
        let state_type = 0x0002;
        let instance = 0xFFFF_FFFF_FFFF_FFFF; // All bits set
        let id = StateId::from_type_instance(state_type, instance);

        assert_eq!(id.state_type(), state_type);
        assert_eq!(id.instance(), 0x0000_FFFF_FFFF_FFFF); // Truncated
    }
}
