#![allow(clippy::missing_safety_doc)]
//! FFI Identity functions

use std::ffi::c_int;
use std::ptr;

use crate::error::*;
use crate::types::*;

/// Opaque handle to an ELARA identity
pub struct ElaraIdentityHandle {
    pub(crate) identity: elara_crypto::Identity,
}

/// Generate a new identity
/// Returns a handle that must be freed with elara_identity_free
#[no_mangle]
pub extern "C" fn elara_identity_generate() -> *mut ElaraIdentityHandle {
    let identity = elara_crypto::Identity::generate();
    let handle = Box::new(ElaraIdentityHandle { identity });
    Box::into_raw(handle)
}

/// Free an identity handle
#[no_mangle]
pub unsafe extern "C" fn elara_identity_free(handle: *mut ElaraIdentityHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Get the node ID from an identity
#[no_mangle]
pub unsafe extern "C" fn elara_identity_node_id(handle: *const ElaraIdentityHandle) -> ElaraNodeId {
    if handle.is_null() {
        return ElaraNodeId { value: 0 };
    }

    let identity = &(*handle).identity;
    ElaraNodeId::from(identity.node_id())
}

/// Get the public key as bytes (32 bytes)
/// Returns the number of bytes written, or negative on error
#[no_mangle]
pub unsafe extern "C" fn elara_identity_public_key(
    handle: *const ElaraIdentityHandle,
    out_buf: *mut u8,
    buf_len: usize,
) -> c_int {
    if handle.is_null() || out_buf.is_null() {
        set_last_error("Null pointer");
        return ElaraErrorCode::InvalidArgument as c_int;
    }

    let identity = &(*handle).identity;
    let pubkey = identity.verifying_key_bytes();

    if buf_len < pubkey.len() {
        set_last_error("Buffer too small");
        return ElaraErrorCode::BufferTooSmall as c_int;
    }

    ptr::copy_nonoverlapping(pubkey.as_ptr(), out_buf, pubkey.len());
    pubkey.len() as c_int
}

/// Export identity to bytes (32 bytes signing key)
/// Returns an ElaraBytes that must be freed with elara_free_bytes
#[no_mangle]
pub unsafe extern "C" fn elara_identity_export(handle: *const ElaraIdentityHandle) -> ElaraBytes {
    if handle.is_null() {
        set_last_error("Null pointer");
        return ElaraBytes::empty();
    }

    let identity = &(*handle).identity;
    let bytes = identity.signing_key_bytes();
    ElaraBytes::from_vec(bytes.to_vec())
}

/// Import identity from bytes (32 bytes signing key)
/// Returns a handle that must be freed with elara_identity_free
#[no_mangle]
pub unsafe extern "C" fn elara_identity_import(
    data: *const u8,
    len: usize,
) -> *mut ElaraIdentityHandle {
    if data.is_null() || len != 32 {
        set_last_error("Invalid data: expected 32 bytes");
        return ptr::null_mut();
    }

    let slice = std::slice::from_raw_parts(data, len);
    let bytes: [u8; 32] = slice.try_into().unwrap();
    let identity = elara_crypto::Identity::from_bytes(&bytes);
    let handle = Box::new(ElaraIdentityHandle { identity });
    Box::into_raw(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generate_free() {
        let handle = elara_identity_generate();
        assert!(!handle.is_null());

        let node_id = unsafe { elara_identity_node_id(handle) };
        assert!(node_id.value != 0);

        unsafe { elara_identity_free(handle) };
    }

    #[test]
    fn test_identity_export_import() {
        let handle = elara_identity_generate();
        let original_id = unsafe { elara_identity_node_id(handle) };

        let exported = unsafe { elara_identity_export(handle) };
        assert!(!exported.is_empty());

        let imported = unsafe { elara_identity_import(exported.data, exported.len) };
        assert!(!imported.is_null());

        let imported_id = unsafe { elara_identity_node_id(imported) };
        assert_eq!(original_id.value, imported_id.value);

        unsafe { elara_identity_free(handle) };
        unsafe { elara_identity_free(imported) };

        // Free the exported bytes
        unsafe {
            if !exported.data.is_null() {
                drop(Vec::from_raw_parts(
                    exported.data,
                    exported.len,
                    exported.capacity,
                ));
            }
        }
    }
}
