//! ELARA FFI - Foreign Function Interface
//!
//! C-compatible bindings for mobile SDKs (Android/iOS).
//! This crate provides a stable ABI for Kotlin and Swift wrappers.

pub mod types;
pub mod identity;
pub mod session;
pub mod error;

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

pub use types::*;
pub use identity::*;
pub use session::*;
pub use error::*;

/// Library version
#[no_mangle]
pub extern "C" fn elara_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Initialize the ELARA library
/// Must be called before any other functions
/// Returns 0 on success, negative on error
#[no_mangle]
pub extern "C" fn elara_init() -> c_int {
    // Initialize logging, runtime, etc.
    0
}

/// Shutdown the ELARA library
/// Should be called when done using the library
#[no_mangle]
pub extern "C" fn elara_shutdown() {
    // Cleanup resources
}

/// Free a string allocated by ELARA
#[no_mangle]
pub extern "C" fn elara_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Free a byte buffer allocated by ELARA
#[no_mangle]
pub extern "C" fn elara_free_bytes(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Vec::from_raw_parts(ptr, len, len));
        }
    }
}
