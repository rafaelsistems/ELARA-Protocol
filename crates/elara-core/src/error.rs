//! Error types for ELARA protocol

use thiserror::Error;

use crate::{EventId, NodeId, StateId};

/// Core ELARA errors
#[derive(Error, Debug)]
pub enum ElaraError {
    // Wire errors
    #[error("Invalid wire format: {0}")]
    InvalidWireFormat(String),

    #[error("Buffer too short: expected {expected}, got {actual}")]
    BufferTooShort { expected: usize, actual: usize },

    #[error("Unknown packet class: {0}")]
    UnknownPacketClass(u8),

    #[error("Unknown event type: {0}")]
    UnknownEventType(u8),

    // Crypto errors
    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Replay detected: seq {0}")]
    ReplayDetected(u32),

    #[error("Ratchet out of sync")]
    RatchetOutOfSync,

    // Authority errors
    #[error("Unauthorized: node {node} cannot mutate state {state}")]
    Unauthorized { node: NodeId, state: StateId },

    #[error("Authority revoked for node {0}")]
    AuthorityRevoked(NodeId),

    // Causality errors
    #[error("Causality violation")]
    CausalityViolation,

    #[error("Missing dependency: {0:?}")]
    MissingDependency(EventId),

    // State errors
    #[error("State not found: {0:?}")]
    StateNotFound(StateId),

    #[error("State bounds exceeded")]
    StateBoundsExceeded,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Entropy exceeded")]
    EntropyExceeded,

    // Time errors
    #[error("Event too late: beyond correction horizon")]
    EventTooLate,

    #[error("Event too early: beyond prediction horizon")]
    EventTooEarly,

    // Session errors
    #[error("Session not found")]
    SessionNotFound,

    #[error("Session mismatch")]
    SessionMismatch,

    #[error("Node not in session")]
    NodeNotInSession,

    // Transport errors
    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Connection failed")]
    ConnectionFailed,
}

/// Result type for ELARA operations
pub type ElaraResult<T> = Result<T, ElaraError>;
