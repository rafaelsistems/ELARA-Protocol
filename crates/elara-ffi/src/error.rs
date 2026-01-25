//! FFI Error handling

use std::ffi::{c_char, c_int, CString};

/// Error codes for FFI functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElaraErrorCode {
    /// Success
    Ok = 0,
    /// Invalid argument
    InvalidArgument = -1,
    /// Not initialized
    NotInitialized = -2,
    /// Already initialized
    AlreadyInitialized = -3,
    /// Out of memory
    OutOfMemory = -4,
    /// Network error
    NetworkError = -5,
    /// Crypto error
    CryptoError = -6,
    /// Timeout
    Timeout = -7,
    /// Session not found
    SessionNotFound = -8,
    /// Node not found
    NodeNotFound = -9,
    /// Buffer too small
    BufferTooSmall = -10,
    /// Internal error
    InternalError = -99,
}

impl From<ElaraErrorCode> for c_int {
    fn from(code: ElaraErrorCode) -> Self {
        code as c_int
    }
}

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<CString>> =
        const { std::cell::RefCell::new(None) };
}

/// Set the last error message
pub fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

/// Get the last error message
/// Returns NULL if no error
#[no_mangle]
pub extern "C" fn elara_get_last_error() -> *const c_char {
    LAST_ERROR.with(|e| match e.borrow().as_ref() {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    })
}

/// Clear the last error
#[no_mangle]
pub extern "C" fn elara_clear_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

/// Result type for FFI functions
pub type ElaraResult<T> = Result<T, ElaraErrorCode>;

/// Convert an ElaraResult to a C int, setting error message on failure
pub fn result_to_int<T>(result: ElaraResult<T>) -> c_int {
    match result {
        Ok(_) => 0,
        Err(code) => code as c_int,
    }
}
