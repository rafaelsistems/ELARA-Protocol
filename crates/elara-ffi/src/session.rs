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
    });
    Box::into_raw(handle)
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
    (*handle).received_messages = (*handle).received_messages.saturating_add(1);
    (*handle).received_bytes = (*handle).received_bytes.saturating_add(len as u64);
    (*handle).presence = elara_core::PresenceVector::full();
    (*handle).degradation = elara_core::DegradationLevel::L0_FullPerception;

    if let Some((callback, user_data)) = (*handle).message_callback {
        callback(user_data, (*handle).node_id.into(), data, len);
    }

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
