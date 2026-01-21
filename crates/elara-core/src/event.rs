//! Event definitions
//!
//! Events (ε) are lawful mutations of state in ELARA.
//! Each event carries source identity, target state, mutation delta,
//! temporal intent, and authority proof.

use crate::{EventId, NodeId, StateId, StateTime, TimeIntent, VersionVector};

/// Event type classification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EventType {
    // Core state events
    StateCreate = 0x01,
    StateUpdate = 0x02,
    StateDelete = 0x03,

    // Authority events
    AuthorityGrant = 0x10,
    AuthorityRevoke = 0x11,

    // Session events
    SessionJoin = 0x20,
    SessionLeave = 0x21,
    SessionSync = 0x22,

    // Time events
    TimeSync = 0x30,
    TimeCorrection = 0x31,

    // Repair events
    StateRequest = 0x40,
    StateResponse = 0x41,
    GapFill = 0x42,

    // Profile-specific events (0x80+)
    TextAppend = 0x80,
    TextEdit = 0x81,
    TextDelete = 0x82,
    TextReact = 0x83,

    VoiceFrame = 0x90,
    VoiceMute = 0x91,

    PresenceUpdate = 0xA0,
    TypingStart = 0xA1,
    TypingStop = 0xA2,
}

impl EventType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(EventType::StateCreate),
            0x02 => Some(EventType::StateUpdate),
            0x03 => Some(EventType::StateDelete),
            0x10 => Some(EventType::AuthorityGrant),
            0x11 => Some(EventType::AuthorityRevoke),
            0x20 => Some(EventType::SessionJoin),
            0x21 => Some(EventType::SessionLeave),
            0x22 => Some(EventType::SessionSync),
            0x30 => Some(EventType::TimeSync),
            0x31 => Some(EventType::TimeCorrection),
            0x40 => Some(EventType::StateRequest),
            0x41 => Some(EventType::StateResponse),
            0x42 => Some(EventType::GapFill),
            0x80 => Some(EventType::TextAppend),
            0x81 => Some(EventType::TextEdit),
            0x82 => Some(EventType::TextDelete),
            0x83 => Some(EventType::TextReact),
            0x90 => Some(EventType::VoiceFrame),
            0x91 => Some(EventType::VoiceMute),
            0xA0 => Some(EventType::PresenceUpdate),
            0xA1 => Some(EventType::TypingStart),
            0xA2 => Some(EventType::TypingStop),
            _ => None,
        }
    }

    #[inline]
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Is this a core protocol event (vs profile-specific)?
    pub fn is_core_protocol(self) -> bool {
        (self as u8) < 0x80
    }
}

/// Mutation operation
#[derive(Clone, Debug)]
pub enum MutationOp {
    /// Set/replace value
    Set(Vec<u8>),
    /// Increment counter
    Increment(i64),
    /// Append to list
    Append(Vec<u8>),
    /// Merge (CRDT-style)
    Merge(Vec<u8>),
    /// Delete
    Delete,
    /// Blend (for continuous state)
    Blend { value: Vec<u8>, weight: f32 },
}

impl MutationOp {
    /// Encode mutation operation for wire format
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            MutationOp::Set(data) => {
                buf.push(0x01);
                buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
                buf.extend_from_slice(data);
            }
            MutationOp::Increment(delta) => {
                buf.push(0x02);
                buf.extend_from_slice(&delta.to_le_bytes());
            }
            MutationOp::Append(data) => {
                buf.push(0x03);
                buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
                buf.extend_from_slice(data);
            }
            MutationOp::Merge(data) => {
                buf.push(0x04);
                buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
                buf.extend_from_slice(data);
            }
            MutationOp::Delete => {
                buf.push(0x05);
            }
            MutationOp::Blend { value, weight } => {
                buf.push(0x06);
                buf.extend_from_slice(&weight.to_le_bytes());
                buf.extend_from_slice(&(value.len() as u16).to_le_bytes());
                buf.extend_from_slice(value);
            }
        }
        buf
    }

    /// Decode mutation operation from wire format
    pub fn decode(buf: &[u8]) -> Option<(Self, usize)> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {
            0x01 => {
                // Set
                if buf.len() < 3 {
                    return None;
                }
                let len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
                if buf.len() < 3 + len {
                    return None;
                }
                Some((MutationOp::Set(buf[3..3 + len].to_vec()), 3 + len))
            }
            0x02 => {
                // Increment
                if buf.len() < 9 {
                    return None;
                }
                let delta = i64::from_le_bytes(buf[1..9].try_into().ok()?);
                Some((MutationOp::Increment(delta), 9))
            }
            0x03 => {
                // Append
                if buf.len() < 3 {
                    return None;
                }
                let len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
                if buf.len() < 3 + len {
                    return None;
                }
                Some((MutationOp::Append(buf[3..3 + len].to_vec()), 3 + len))
            }
            0x04 => {
                // Merge
                if buf.len() < 3 {
                    return None;
                }
                let len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
                if buf.len() < 3 + len {
                    return None;
                }
                Some((MutationOp::Merge(buf[3..3 + len].to_vec()), 3 + len))
            }
            0x05 => {
                // Delete
                Some((MutationOp::Delete, 1))
            }
            0x06 => {
                // Blend
                if buf.len() < 7 {
                    return None;
                }
                let weight = f32::from_le_bytes(buf[1..5].try_into().ok()?);
                let len = u16::from_le_bytes([buf[5], buf[6]]) as usize;
                if buf.len() < 7 + len {
                    return None;
                }
                Some((
                    MutationOp::Blend {
                        value: buf[7..7 + len].to_vec(),
                        weight,
                    },
                    7 + len,
                ))
            }
            _ => None,
        }
    }
}

/// Authority proof - cryptographic proof of mutation authority
#[derive(Clone, Debug)]
pub struct AuthorityProof {
    /// Signature over event content
    pub signature: [u8; 64],
    /// Optional delegation chain
    pub delegation_chain: Option<Vec<DelegationLink>>,
}

impl AuthorityProof {
    pub fn new(signature: [u8; 64]) -> Self {
        AuthorityProof {
            signature,
            delegation_chain: None,
        }
    }

    pub fn with_delegation(mut self, chain: Vec<DelegationLink>) -> Self {
        self.delegation_chain = Some(chain);
        self
    }
}

/// Link in a delegation chain
#[derive(Clone, Debug)]
pub struct DelegationLink {
    pub delegator: NodeId,
    pub delegate: NodeId,
    pub scope: Vec<u8>, // Encoded AuthorityScope
    pub signature: [u8; 64],
}

/// Entropy hint for divergence control
#[derive(Clone, Copy, Debug, Default)]
pub struct EntropyHint {
    /// Estimated entropy contribution
    pub entropy: f32,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
}

impl EntropyHint {
    pub fn new(entropy: f32, confidence: f32) -> Self {
        EntropyHint { entropy, confidence }
    }

    pub fn certain() -> Self {
        EntropyHint {
            entropy: 0.0,
            confidence: 1.0,
        }
    }

    pub fn predicted(entropy: f32) -> Self {
        EntropyHint {
            entropy,
            confidence: 0.5,
        }
    }
}

/// Event - a lawful mutation of state
#[derive(Clone, Debug)]
pub struct Event {
    /// Event identity (for causal ordering)
    pub id: EventId,
    /// Event type
    pub event_type: EventType,
    /// Source node identity
    pub source: NodeId,
    /// Target state atom
    pub target_state: StateId,
    /// Expected version before mutation
    pub version_ref: VersionVector,
    /// The mutation operation
    pub mutation: MutationOp,
    /// Temporal intent
    pub time_intent: TimeIntent,
    /// Authority proof
    pub authority_proof: AuthorityProof,
    /// Entropy hint
    pub entropy_hint: EntropyHint,
}

impl Event {
    /// Create a new event
    pub fn new(
        source: NodeId,
        seq: u64,
        event_type: EventType,
        target_state: StateId,
        mutation: MutationOp,
    ) -> Self {
        Event {
            id: EventId::new(source, seq),
            event_type,
            source,
            target_state,
            version_ref: VersionVector::new(),
            mutation,
            time_intent: TimeIntent::default(),
            authority_proof: AuthorityProof::new([0u8; 64]),
            entropy_hint: EntropyHint::certain(),
        }
    }

    /// Set version reference
    pub fn with_version(mut self, version: VersionVector) -> Self {
        self.version_ref = version;
        self
    }

    /// Set time intent
    pub fn with_time_intent(mut self, intent: TimeIntent) -> Self {
        self.time_intent = intent;
        self
    }

    /// Set authority proof
    pub fn with_authority_proof(mut self, proof: AuthorityProof) -> Self {
        self.authority_proof = proof;
        self
    }

    /// Set entropy hint
    pub fn with_entropy_hint(mut self, hint: EntropyHint) -> Self {
        self.entropy_hint = hint;
        self
    }

    /// Get the absolute time intent given a reference
    pub fn absolute_time(&self, reference: StateTime) -> StateTime {
        self.time_intent.to_absolute(reference)
    }
}

/// Validated event - passed all checks
#[derive(Clone, Debug)]
pub struct ValidatedEvent {
    pub event: Event,
    pub validated_at: StateTime,
}

impl ValidatedEvent {
    pub fn new(event: Event, validated_at: StateTime) -> Self {
        ValidatedEvent { event, validated_at }
    }
}

/// Event processing result
#[derive(Clone, Debug)]
pub enum EventResult {
    /// Event was applied successfully
    Applied,
    /// Event was merged with existing state
    Merged,
    /// Event was used for late correction
    LateCorrected,
    /// Event was buffered for future processing
    Buffered,
    /// Event was a duplicate (already processed)
    Duplicate,
    /// Event was rejected
    Rejected(RejectReason),
}

/// Reason for event rejection
#[derive(Clone, Debug)]
pub enum RejectReason {
    /// Source not authorized
    Unauthorized,
    /// Invalid signature
    InvalidSignature,
    /// Authority was revoked
    AuthorityRevoked,
    /// Causality violation
    CausalityViolation,
    /// Missing dependency
    MissingDependency(EventId),
    /// Out of bounds
    OutOfBounds,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Entropy exceeded
    EntropyExceeded,
    /// Too late (beyond correction horizon)
    TooLate,
    /// Replay detected
    ReplayDetected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_roundtrip() {
        for event_type in [
            EventType::StateCreate,
            EventType::StateUpdate,
            EventType::TextAppend,
            EventType::VoiceFrame,
        ] {
            let byte = event_type.to_byte();
            let recovered = EventType::from_byte(byte).unwrap();
            assert_eq!(event_type, recovered);
        }
    }

    #[test]
    fn test_mutation_op_encode_decode() {
        let ops = vec![
            MutationOp::Set(vec![1, 2, 3, 4]),
            MutationOp::Increment(42),
            MutationOp::Append(vec![5, 6, 7]),
            MutationOp::Delete,
            MutationOp::Blend {
                value: vec![8, 9],
                weight: 0.5,
            },
        ];

        for op in ops {
            let encoded = op.encode();
            let (decoded, len) = MutationOp::decode(&encoded).unwrap();
            assert_eq!(len, encoded.len());

            // Compare encoded forms (since MutationOp doesn't implement Eq)
            assert_eq!(op.encode(), decoded.encode());
        }
    }

    #[test]
    fn test_event_creation() {
        let source = NodeId::new(1);
        let target = StateId::new(100);
        let event = Event::new(
            source,
            1,
            EventType::TextAppend,
            target,
            MutationOp::Append(b"Hello".to_vec()),
        );

        assert_eq!(event.source, source);
        assert_eq!(event.target_state, target);
        assert_eq!(event.id.seq, 1);
    }
}
