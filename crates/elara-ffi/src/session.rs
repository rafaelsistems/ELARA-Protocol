#![allow(clippy::missing_safety_doc)]
//! FFI Session functions

use std::ffi::{c_int, c_void};
use std::ptr;
use std::time::{Duration, Instant};

use crate::error::*;
use crate::identity::ElaraIdentityHandle;
use crate::types::*;

/// Opaque handle to an ELARA session
pub struct ElaraSessionHandle {
    pub(crate) session_id: elara_core::SessionId,
    pub(crate) node_id: elara_core::NodeId,
    pub(crate) presence: elara_core::PresenceVector,
    pub(crate) degradation: elara_core::DegradationLevel,
    pub(crate) last_activity: Instant,
    pub(crate) received_messages: u64,
    pub(crate) received_bytes: u64,
    pub(crate) message_callback: Option<(ElaraMessageCallback, *mut c_void)>,
    pub(crate) presence_callback: Option<(ElaraPresenceCallback, *mut c_void)>,
    pub(crate) degradation_callback: Option<(ElaraDegradationCallback, *mut c_void)>,
    pub(crate) node: elara_runtime::Node,
}

// Session handles are Send but not Sync (must be accessed from one thread at a time)
unsafe impl Send for ElaraSessionHandle {}

/// Create a new session
/// Returns a handle that must be freed with elara_session_free
#[no_mangle]
pub unsafe extern "C" fn elara_session_create(
    identity: *const ElaraIdentityHandle,
    session_id: u64,
) -> *mut ElaraSessionHandle {
    if identity.is_null() {
        set_last_error("Null identity");
        return ptr::null_mut();
    }

    let identity_ref = &(*identity).identity;
    let mut node = elara_runtime::Node::with_identity(
        identity_ref.clone(),
        elara_runtime::NodeConfig::default(),
    );
    node.join_session_unsecured(elara_core::SessionId::new(session_id));
    let handle = Box::new(ElaraSessionHandle {
        session_id: elara_core::SessionId::new(session_id),
        node_id: identity_ref.node_id(),
        presence: elara_core::PresenceVector::full(),
        degradation: elara_core::DegradationLevel::L0_FullPerception,
        last_activity: Instant::now(),
        received_messages: 0,
        received_bytes: 0,
        message_callback: None,
        presence_callback: None,
        degradation_callback: None,
        node,
    });
    Box::into_raw(handle)
}

/// Set session key and join encrypted session
/// key must be 32 bytes
#[no_mangle]
pub unsafe extern "C" fn elara_session_set_session_key(
    handle: *mut ElaraSessionHandle,
    session_id: u64,
    key: *const u8,
    len: usize,
) -> c_int {
    if handle.is_null() || key.is_null() {
        set_last_error("Null pointer");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    if len != 32 {
        set_last_error("Invalid key length: expected 32 bytes");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    let slice = std::slice::from_raw_parts(key, len);
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(slice);

    (*handle)
        .node
        .join_session(elara_core::SessionId::new(session_id), key_bytes);

    (*handle).session_id = elara_core::SessionId::new(session_id);
    (*handle).last_activity = Instant::now();
    (*handle).presence = elara_core::PresenceVector::full();
    (*handle).degradation = elara_core::DegradationLevel::L0_FullPerception;

    0
}

/// Free a session handle
#[no_mangle]
pub unsafe extern "C" fn elara_session_free(handle: *mut ElaraSessionHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get the session ID
#[no_mangle]
pub unsafe extern "C" fn elara_session_id(handle: *const ElaraSessionHandle) -> ElaraSessionId {
    if handle.is_null() {
        return ElaraSessionId { value: 0 };
    }

    ElaraSessionId::from((*handle).session_id)
}

/// Get the local node ID for this session
#[no_mangle]
pub unsafe extern "C" fn elara_session_node_id(handle: *const ElaraSessionHandle) -> ElaraNodeId {
    if handle.is_null() {
        return ElaraNodeId { value: 0 };
    }

    ElaraNodeId::from((*handle).node_id)
}

/// Get current presence
#[no_mangle]
pub unsafe extern "C" fn elara_session_presence(
    handle: *const ElaraSessionHandle,
) -> ElaraPresence {
    if handle.is_null() {
        return ElaraPresence {
            liveness: 0.0,
            immediacy: 0.0,
            coherence: 0.0,
            relational_continuity: 0.0,
            emotional_bandwidth: 0.0,
        };
    }

    ElaraPresence::from((*handle).presence)
}

/// Get current degradation level
#[no_mangle]
pub unsafe extern "C" fn elara_session_degradation(
    handle: *const ElaraSessionHandle,
) -> ElaraDegradationLevel {
    if handle.is_null() {
        return ElaraDegradationLevel::L5_LatentPresence;
    }

    ElaraDegradationLevel::from((*handle).degradation)
}

/// Set message callback
#[no_mangle]
pub unsafe extern "C" fn elara_session_set_message_callback(
    handle: *mut ElaraSessionHandle,
    callback: ElaraMessageCallback,
    user_data: *mut c_void,
) -> c_int {
    if handle.is_null() {
        set_last_error("Null handle");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    (*handle).message_callback = Some((callback, user_data));

    0
}

/// Set presence callback
#[no_mangle]
pub unsafe extern "C" fn elara_session_set_presence_callback(
    handle: *mut ElaraSessionHandle,
    callback: ElaraPresenceCallback,
    user_data: *mut c_void,
) -> c_int {
    if handle.is_null() {
        set_last_error("Null handle");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    (*handle).presence_callback = Some((callback, user_data));

    0
}

/// Set degradation callback
#[no_mangle]
pub unsafe extern "C" fn elara_session_set_degradation_callback(
    handle: *mut ElaraSessionHandle,
    callback: ElaraDegradationCallback,
    user_data: *mut c_void,
) -> c_int {
    if handle.is_null() {
        set_last_error("Null handle");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    (*handle).degradation_callback = Some((callback, user_data));

    0
}

#[no_mangle]
pub unsafe extern "C" fn elara_session_clear_callbacks(handle: *mut ElaraSessionHandle) -> c_int {
    if handle.is_null() {
        set_last_error("Null handle");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    (*handle).message_callback = None;
    (*handle).presence_callback = None;
    (*handle).degradation_callback = None;

    0
}

/// Send a message to a peer
#[no_mangle]
pub unsafe extern "C" fn elara_session_send(
    handle: *mut ElaraSessionHandle,
    _dest: ElaraNodeId,
    data: *const u8,
    len: usize,
) -> c_int {
    if handle.is_null() || data.is_null() {
        set_last_error("Null pointer");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    if len == 0 {
        set_last_error("Empty message");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    let slice = std::slice::from_raw_parts(data, len);

    let event = if len == elara_msp::voice::VoiceFrame::WIRE_SIZE {
        let state_id = elara_msp::voice::voice_id((*handle).node_id);
        let seq = (*handle).node.next_event_seq();
        elara_core::Event::new(
            (*handle).node_id,
            seq,
            elara_core::EventType::VoiceFrame,
            state_id,
            elara_core::MutationOp::Set(slice.to_vec()),
        )
    } else {
        let state_id = elara_msp::text::text_stream_id(1);
        let seq = (*handle).node.next_event_seq();
        elara_core::Event::new(
            (*handle).node_id,
            seq,
            elara_core::EventType::TextAppend,
            state_id,
            elara_core::MutationOp::Append(slice.to_vec()),
        )
    };

    (*handle).node.queue_local_event(event);
    (*handle).node.tick();

    if let Some(frame) = (*handle).node.pop_outgoing() {
        if let Ok(bytes) = frame.serialize() {
            if let Some((callback, user_data)) = (*handle).message_callback {
                callback(
                    user_data,
                    (*handle).node_id.into(),
                    bytes.as_ptr(),
                    bytes.len(),
                );
            }
        } else {
            set_last_error("Failed to serialize frame");
            return ElaraErrorCode::InternalError as c_int;
        }
    }

    (*handle).last_activity = Instant::now();
    (*handle).presence = elara_core::PresenceVector::full();
    (*handle).degradation = elara_core::DegradationLevel::L0_FullPerception;

    0
}

/// Process incoming data
/// Call this when data is received from the network
#[no_mangle]
pub unsafe extern "C" fn elara_session_receive(
    handle: *mut ElaraSessionHandle,
    data: *const u8,
    len: usize,
) -> c_int {
    if handle.is_null() || data.is_null() {
        set_last_error("Null pointer");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    if len == 0 {
        set_last_error("Empty message");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    (*handle).last_activity = Instant::now();
    (*handle).received_bytes = (*handle).received_bytes.saturating_add(len as u64);
    (*handle).presence = elara_core::PresenceVector::full();
    (*handle).degradation = elara_core::DegradationLevel::L0_FullPerception;

    let bytes = std::slice::from_raw_parts(data, len);
    let mut callbacks_sent = 0;

    if let Ok(frame) = elara_wire::Frame::parse(bytes) {
        let events =
            decode_event_blocks(&frame.payload, frame.header.node_id, frame.header.time_hint);
        for event in events {
            if let Some(payload) = mutation_payload(&event.mutation) {
                if let Some((callback, user_data)) = (*handle).message_callback {
                    callback(
                        user_data,
                        event.source.into(),
                        payload.as_ptr(),
                        payload.len(),
                    );
                    callbacks_sent += 1;
                }
            }
        }
    }

    if callbacks_sent == 0 {
        if let Some((callback, user_data)) = (*handle).message_callback {
            callback(user_data, (*handle).node_id.into(), data, len);
            callbacks_sent = 1;
        }
    }

    (*handle).received_messages = (*handle).received_messages.saturating_add(callbacks_sent);

    if let Some((callback, user_data)) = (*handle).presence_callback {
        callback(
            user_data,
            (*handle).node_id.into(),
            (*handle).presence.into(),
        );
    }

    0
}

/// Tick the session (advance time, process predictions)
/// Should be called regularly (e.g., every 10ms)
#[no_mangle]
pub unsafe extern "C" fn elara_session_tick(handle: *mut ElaraSessionHandle) -> c_int {
    if handle.is_null() {
        set_last_error("Null handle");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    (*handle).node.tick();
    let elapsed = (*handle).last_activity.elapsed();
    let new_level = if elapsed <= Duration::from_secs(2) {
        elara_core::DegradationLevel::L0_FullPerception
    } else if elapsed <= Duration::from_secs(5) {
        elara_core::DegradationLevel::L1_DistortedPerception
    } else if elapsed <= Duration::from_secs(10) {
        elara_core::DegradationLevel::L2_FragmentedPerception
    } else if elapsed <= Duration::from_secs(20) {
        elara_core::DegradationLevel::L3_SymbolicPresence
    } else if elapsed <= Duration::from_secs(40) {
        elara_core::DegradationLevel::L4_MinimalPresence
    } else {
        elara_core::DegradationLevel::L5_LatentPresence
    };

    if new_level != (*handle).degradation {
        (*handle).degradation = new_level;
        (*handle).presence = presence_for_level(new_level);

        if let Some((callback, user_data)) = (*handle).degradation_callback {
            callback(user_data, (*handle).degradation.into());
        }

        if let Some((callback, user_data)) = (*handle).presence_callback {
            callback(
                user_data,
                (*handle).node_id.into(),
                (*handle).presence.into(),
            );
        }
    }

    0
}

fn presence_for_level(level: elara_core::DegradationLevel) -> elara_core::PresenceVector {
    match level {
        elara_core::DegradationLevel::L0_FullPerception => elara_core::PresenceVector::full(),
        elara_core::DegradationLevel::L1_DistortedPerception => {
            elara_core::PresenceVector::new(0.9, 0.8, 0.8, 0.9, 0.8)
        }
        elara_core::DegradationLevel::L2_FragmentedPerception => {
            elara_core::PresenceVector::new(0.7, 0.6, 0.6, 0.8, 0.6)
        }
        elara_core::DegradationLevel::L3_SymbolicPresence => {
            elara_core::PresenceVector::new(0.5, 0.3, 0.4, 0.6, 0.3)
        }
        elara_core::DegradationLevel::L4_MinimalPresence => {
            elara_core::PresenceVector::new(0.3, 0.1, 0.2, 0.4, 0.1)
        }
        elara_core::DegradationLevel::L5_LatentPresence => elara_core::PresenceVector::minimal(),
    }
}

fn decode_event_blocks(
    payload: &[u8],
    source: elara_core::NodeId,
    time_hint: i32,
) -> Vec<elara_core::Event> {
    let mut events = Vec::new();
    let mut offset = 0;

    while payload.len().saturating_sub(offset) >= 13 {
        let event_type = match elara_core::EventType::from_byte(payload[offset]) {
            Some(t) => t,
            None => break,
        };
        offset += 1;

        let state_end = offset + 8;
        if state_end > payload.len() {
            break;
        }
        let state_id = match payload[offset..state_end].try_into() {
            Ok(bytes) => elara_core::StateId::from_bytes(bytes),
            Err(_) => break,
        };
        offset = state_end;

        let version_len_end = offset + 2;
        if version_len_end > payload.len() {
            break;
        }
        let version_len = match payload[offset..version_len_end].try_into() {
            Ok(bytes) => u16::from_le_bytes(bytes) as usize,
            Err(_) => break,
        };
        offset = version_len_end;

        let version_end = offset + version_len;
        if version_end > payload.len() {
            break;
        }
        let version_ref = match decode_version_vector(&payload[offset..version_end]) {
            Some(v) => v,
            None => break,
        };
        offset = version_end;

        let delta_len_end = offset + 2;
        if delta_len_end > payload.len() {
            break;
        }
        let delta_len = match payload[offset..delta_len_end].try_into() {
            Ok(bytes) => u16::from_le_bytes(bytes) as usize,
            Err(_) => break,
        };
        offset = delta_len_end;

        let delta_end = offset + delta_len;
        if delta_end > payload.len() {
            break;
        }
        let delta = &payload[offset..delta_end];
        let (mutation, used) = match elara_core::MutationOp::decode(delta) {
            Some(decoded) => decoded,
            None => break,
        };
        if used != delta_len {
            break;
        }
        offset = delta_end;

        let seq = version_ref.get(source).saturating_add(1);
        let event = elara_core::Event::new(source, seq, event_type, state_id, mutation)
            .with_version(version_ref)
            .with_time_intent(elara_core::TimeIntent::new(time_hint));
        events.push(event);
    }

    events
}

fn decode_version_vector(buf: &[u8]) -> Option<elara_core::VersionVector> {
    if !buf.len().is_multiple_of(16) {
        return None;
    }
    let mut entries = Vec::new();
    for chunk in buf.chunks_exact(16) {
        let node = match chunk[0..8].try_into() {
            Ok(bytes) => elara_core::NodeId::from_bytes(bytes),
            Err(_) => return None,
        };
        let count = match chunk[8..16].try_into() {
            Ok(bytes) => u64::from_le_bytes(bytes),
            Err(_) => return None,
        };
        entries.push((node, count));
    }
    Some(elara_core::VersionVector::from_compact(entries))
}

fn mutation_payload(mutation: &elara_core::MutationOp) -> Option<&[u8]> {
    match mutation {
        elara_core::MutationOp::Set(data)
        | elara_core::MutationOp::Append(data)
        | elara_core::MutationOp::Merge(data) => Some(data.as_slice()),
        elara_core::MutationOp::Blend { value, .. } => Some(value.as_slice()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::elara_identity_generate;

    #[test]
    fn test_session_create_free() {
        let identity = elara_identity_generate();
        let session = unsafe { elara_session_create(identity, 12345) };

        assert!(!session.is_null());

        let session_id = unsafe { elara_session_id(session) };
        assert_eq!(session_id.value, 12345);

        unsafe { elara_session_free(session) };
        unsafe { crate::identity::elara_identity_free(identity) };
    }

    #[test]
    fn test_session_presence() {
        let identity = elara_identity_generate();
        let session = unsafe { elara_session_create(identity, 12345) };

        let presence = unsafe { elara_session_presence(session) };
        assert_eq!(presence.liveness, 1.0);

        unsafe { elara_session_free(session) };
        unsafe { crate::identity::elara_identity_free(identity) };
    }

    #[test]
    fn test_session_degradation() {
        let identity = elara_identity_generate();
        let session = unsafe { elara_session_create(identity, 12345) };

        let level = unsafe { elara_session_degradation(session) };
        assert_eq!(
            level as i32,
            ElaraDegradationLevel::L0_FullPerception as i32
        );

        unsafe { elara_session_free(session) };
        unsafe { crate::identity::elara_identity_free(identity) };
    }
}
