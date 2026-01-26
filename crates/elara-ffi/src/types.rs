//! FFI Types - C-compatible type definitions

/// Node ID (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElaraNodeId {
    pub value: u64,
}

impl From<elara_core::NodeId> for ElaraNodeId {
    fn from(id: elara_core::NodeId) -> Self {
        Self { value: id.0 }
    }
}

impl From<ElaraNodeId> for elara_core::NodeId {
    fn from(id: ElaraNodeId) -> Self {
        elara_core::NodeId::new(id.value)
    }
}

/// Session ID (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElaraSessionId {
    pub value: u64,
}

impl From<elara_core::SessionId> for ElaraSessionId {
    fn from(id: elara_core::SessionId) -> Self {
        Self { value: id.0 }
    }
}

impl From<ElaraSessionId> for elara_core::SessionId {
    fn from(id: ElaraSessionId) -> Self {
        elara_core::SessionId::new(id.value)
    }
}

/// Byte buffer for passing data across FFI
#[repr(C)]
pub struct ElaraBytes {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl ElaraBytes {
    /// Create from a `Vec<u8>`, transferring ownership
    pub fn from_vec(mut v: Vec<u8>) -> Self {
        let result = Self {
            data: v.as_mut_ptr(),
            len: v.len(),
            capacity: v.capacity(),
        };
        std::mem::forget(v);
        result
    }

    /// Create an empty buffer
    pub fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0 || self.data.is_null()
    }
}

/// Presence vector for FFI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElaraPresence {
    pub liveness: f32,
    pub immediacy: f32,
    pub coherence: f32,
    pub relational_continuity: f32,
    pub emotional_bandwidth: f32,
}

impl From<elara_core::PresenceVector> for ElaraPresence {
    fn from(p: elara_core::PresenceVector) -> Self {
        Self {
            liveness: p.liveness,
            immediacy: p.immediacy,
            coherence: p.coherence,
            relational_continuity: p.relational_continuity,
            emotional_bandwidth: p.emotional_bandwidth,
        }
    }
}

/// Degradation level
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum ElaraDegradationLevel {
    L0_FullPerception = 0,
    L1_DistortedPerception = 1,
    L2_FragmentedPerception = 2,
    L3_SymbolicPresence = 3,
    L4_MinimalPresence = 4,
    L5_LatentPresence = 5,
}

impl From<elara_core::DegradationLevel> for ElaraDegradationLevel {
    fn from(level: elara_core::DegradationLevel) -> Self {
        match level {
            elara_core::DegradationLevel::L0_FullPerception => Self::L0_FullPerception,
            elara_core::DegradationLevel::L1_DistortedPerception => Self::L1_DistortedPerception,
            elara_core::DegradationLevel::L2_FragmentedPerception => Self::L2_FragmentedPerception,
            elara_core::DegradationLevel::L3_SymbolicPresence => Self::L3_SymbolicPresence,
            elara_core::DegradationLevel::L4_MinimalPresence => Self::L4_MinimalPresence,
            elara_core::DegradationLevel::L5_LatentPresence => Self::L5_LatentPresence,
        }
    }
}

/// Callback function type for receiving messages
pub type ElaraMessageCallback = extern "C" fn(
    user_data: *mut std::ffi::c_void,
    source: ElaraNodeId,
    data: *const u8,
    len: usize,
);

/// Callback function type for presence updates
pub type ElaraPresenceCallback =
    extern "C" fn(user_data: *mut std::ffi::c_void, node: ElaraNodeId, presence: ElaraPresence);

/// Callback function type for degradation changes
pub type ElaraDegradationCallback =
    extern "C" fn(user_data: *mut std::ffi::c_void, level: ElaraDegradationLevel);
