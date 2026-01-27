#![allow(clippy::missing_safety_doc)]
//! ELARA FFI - Foreign Function Interface
//!
//! C-compatible bindings for mobile SDKs (Android/iOS).
//! This crate provides a stable ABI for Kotlin and Swift wrappers.

pub mod error;
pub mod identity;
pub mod session;
pub mod types;

use std::ffi::{c_char, c_int, CString};

pub use error::*;
pub use identity::*;
pub use session::*;
pub use types::*;

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
pub unsafe extern "C" fn elara_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Free a byte buffer allocated by ELARA
#[no_mangle]
pub unsafe extern "C" fn elara_free_bytes(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[cfg(target_os = "android")]
use jni::objects::{JByteArray, JClass, JGlobalRef, JObject, JValue};
#[cfg(target_os = "android")]
use jni::sys::{jbyteArray, jfloatArray, jint, jlong, jobject, jstring};
#[cfg(target_os = "android")]
use jni::{JNIEnv, JavaVM};
#[cfg(target_os = "android")]
use std::collections::HashMap;
#[cfg(target_os = "android")]
use std::ffi::c_void;
#[cfg(target_os = "android")]
use std::ffi::CStr;
#[cfg(target_os = "android")]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "android")]
struct AndroidCallbackState {
    vm: JavaVM,
    callback: JGlobalRef,
}

#[cfg(target_os = "android")]
static ANDROID_CALLBACKS: OnceLock<Mutex<HashMap<usize, *mut AndroidCallbackState>>> =
    OnceLock::new();

#[cfg(target_os = "android")]
fn android_callbacks() -> &'static Mutex<HashMap<usize, *mut AndroidCallbackState>> {
    ANDROID_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "android")]
fn clear_android_callbacks(handle: *mut ElaraSessionHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        elara_session_clear_callbacks(handle);
    }
    let mut map = android_callbacks().lock().unwrap();
    if let Some(ptr) = map.remove(&(handle as usize)) {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_message_callback(
    user_data: *mut c_void,
    source: ElaraNodeId,
    data: *const u8,
    len: usize,
) {
    if user_data.is_null() || data.is_null() {
        return;
    }
    let state = &*(user_data as *mut AndroidCallbackState);
    let env = match state.vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return,
    };
    let bytes = std::slice::from_raw_parts(data, len);
    let array = match env.byte_array_from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return,
    };
    let array_obj = JObject::from(array);
    let _ = env.call_method(
        state.callback.as_obj(),
        "onMessage",
        "(J[B)V",
        &[
            JValue::Long(source.value as jlong),
            JValue::Object(&array_obj),
        ],
    );
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_presence_callback(
    user_data: *mut c_void,
    node: ElaraNodeId,
    presence: ElaraPresence,
) {
    if user_data.is_null() {
        return;
    }
    let state = &*(user_data as *mut AndroidCallbackState);
    let env = match state.vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return,
    };
    let values = [
        presence.liveness,
        presence.immediacy,
        presence.coherence,
        presence.relational_continuity,
        presence.emotional_bandwidth,
    ];
    let array = match env.new_float_array(values.len() as i32) {
        Ok(value) => value,
        Err(_) => return,
    };
    if env.set_float_array_region(array, 0, &values).is_err() {
        return;
    }
    let array_obj = JObject::from(array);
    let _ = env.call_method(
        state.callback.as_obj(),
        "onPresence",
        "(J[F)V",
        &[
            JValue::Long(node.value as jlong),
            JValue::Object(&array_obj),
        ],
    );
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_degradation_callback(
    user_data: *mut c_void,
    level: ElaraDegradationLevel,
) {
    if user_data.is_null() {
        return;
    }
    let state = &*(user_data as *mut AndroidCallbackState);
    let env = match state.vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return,
    };
    let _ = env.call_method(
        state.callback.as_obj(),
        "onDegradation",
        "(I)V",
        &[JValue::Int(level as jint)],
    );
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Elara_nativeVersion(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let c_str = unsafe { CStr::from_ptr(elara_version()) };
    match env.new_string(c_str.to_string_lossy()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Elara_nativeInit(_env: JNIEnv, _class: JClass) -> jint {
    elara_init() as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Elara_nativeShutdown(_env: JNIEnv, _class: JClass) {
    elara_shutdown();
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Identity_nativeGenerate(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let handle = elara_identity_generate();
    handle as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Identity_nativeFree(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { elara_identity_free(handle as *mut ElaraIdentityHandle) };
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Identity_nativeNodeId(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jlong {
    if handle == 0 {
        return 0;
    }
    let node_id = unsafe { elara_identity_node_id(handle as *const ElaraIdentityHandle) };
    node_id.value as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Identity_nativePublicKey(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jbyteArray {
    if handle == 0 {
        return env
            .byte_array_from_slice(&[])
            .map(|v| v.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }
    let mut buf = [0u8; 32];
    let written = unsafe {
        elara_identity_public_key(
            handle as *const ElaraIdentityHandle,
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    if written <= 0 {
        return env
            .byte_array_from_slice(&[])
            .map(|v| v.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }
    env.byte_array_from_slice(&buf[..written as usize])
        .map(|v| v.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Identity_nativeExport(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jbyteArray {
    if handle == 0 {
        return env
            .byte_array_from_slice(&[])
            .map(|v| v.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }
    let bytes = unsafe { elara_identity_export(handle as *const ElaraIdentityHandle) };
    if bytes.is_empty() {
        return env
            .byte_array_from_slice(&[])
            .map(|v| v.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) };
    let result = env.byte_array_from_slice(slice);
    unsafe { elara_free_bytes(bytes.data, bytes.len) };
    result.map(|v| v.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Identity_nativeImport(
    env: JNIEnv,
    _class: JClass,
    data: jbyteArray,
) -> jlong {
    let array = unsafe { JByteArray::from_raw(data) };
    let bytes = match env.convert_byte_array(array) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let handle = unsafe { elara_identity_import(bytes.as_ptr(), bytes.len()) };
    handle as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
    identity_handle: jlong,
    session_id: jlong,
) -> jlong {
    if identity_handle == 0 {
        return 0;
    }
    let session = unsafe {
        elara_session_create(
            identity_handle as *const ElaraIdentityHandle,
            session_id as u64,
        )
    };
    session as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeFree(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        clear_android_callbacks(handle as *mut ElaraSessionHandle);
        unsafe { elara_session_free(handle as *mut ElaraSessionHandle) };
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeSessionId(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jlong {
    if handle == 0 {
        return 0;
    }
    let session_id = unsafe { elara_session_id(handle as *const ElaraSessionHandle) };
    session_id.value as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeNodeId(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jlong {
    if handle == 0 {
        return 0;
    }
    let node_id = unsafe { elara_session_node_id(handle as *const ElaraSessionHandle) };
    node_id.value as jlong
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativePresence(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jfloatArray {
    if handle == 0 {
        return env
            .new_float_array(0)
            .map(|v| v.into_raw())
            .unwrap_or(std::ptr::null_mut());
    }
    let presence = unsafe { elara_session_presence(handle as *const ElaraSessionHandle) };
    let values = [
        presence.liveness,
        presence.immediacy,
        presence.coherence,
        presence.relational_continuity,
        presence.emotional_bandwidth,
    ];
    let array = match env.new_float_array(values.len() as i32) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    if env.set_float_array_region(array, 0, &values).is_err() {
        return std::ptr::null_mut();
    }
    array.into_raw()
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeDegradation(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    if handle == 0 {
        return ElaraDegradationLevel::L5_LatentPresence as jint;
    }
    let level = unsafe { elara_session_degradation(handle as *const ElaraSessionHandle) };
    level as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeSend(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    dest: jlong,
    data: jbyteArray,
) -> jint {
    if handle == 0 {
        return ElaraErrorCode::InvalidArgument as jint;
    }
    let array = unsafe { JByteArray::from_raw(data) };
    let bytes = match env.convert_byte_array(array) {
        Ok(value) => value,
        Err(_) => return ElaraErrorCode::InvalidArgument as jint,
    };
    unsafe {
        elara_session_send(
            handle as *mut ElaraSessionHandle,
            ElaraNodeId { value: dest as u64 },
            bytes.as_ptr(),
            bytes.len(),
        ) as jint
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeReceive(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    data: jbyteArray,
) -> jint {
    if handle == 0 {
        return ElaraErrorCode::InvalidArgument as jint;
    }
    let array = unsafe { JByteArray::from_raw(data) };
    let bytes = match env.convert_byte_array(array) {
        Ok(value) => value,
        Err(_) => return ElaraErrorCode::InvalidArgument as jint,
    };
    let result = unsafe {
        elara_session_receive(
            handle as *mut ElaraSessionHandle,
            bytes.as_ptr(),
            bytes.len(),
        )
    };
    result as jint
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeSetSessionKey(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    session_id: jlong,
    key: jbyteArray,
) -> jint {
    if handle == 0 {
        return ElaraErrorCode::InvalidArgument as jint;
    }
    let array = unsafe { JByteArray::from_raw(key) };
    let bytes = match env.convert_byte_array(array) {
        Ok(value) => value,
        Err(_) => return ElaraErrorCode::InvalidArgument as jint,
    };
    unsafe {
        elara_session_set_session_key(
            handle as *mut ElaraSessionHandle,
            session_id as u64,
            bytes.as_ptr(),
            bytes.len(),
        ) as jint
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeTick(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    if handle == 0 {
        return ElaraErrorCode::InvalidArgument as jint;
    }
    unsafe { elara_session_tick(handle as *mut ElaraSessionHandle) as jint }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_elara_sdk_Session_nativeSetCallback(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    callback: jobject,
) -> jint {
    if handle == 0 {
        return ElaraErrorCode::InvalidArgument as jint;
    }
    let handle_ptr = handle as *mut ElaraSessionHandle;
    if callback.is_null() {
        clear_android_callbacks(handle_ptr);
        return 0;
    }
    let vm = match env.get_java_vm() {
        Ok(value) => value,
        Err(_) => return ElaraErrorCode::InternalError as jint,
    };
    let callback_obj = unsafe { JObject::from_raw(callback) };
    let global = match env.new_global_ref(callback_obj) {
        Ok(value) => value,
        Err(_) => return ElaraErrorCode::InternalError as jint,
    };
    clear_android_callbacks(handle_ptr);
    let state = Box::new(AndroidCallbackState {
        vm,
        callback: global,
    });
    let state_ptr = Box::into_raw(state);
    let user_data = state_ptr as *mut c_void;
    let result = unsafe {
        elara_session_set_message_callback(handle_ptr, android_message_callback, user_data)
    };
    if result != 0 {
        unsafe { drop(Box::from_raw(state_ptr)) };
        return result as jint;
    }
    let result = unsafe {
        elara_session_set_presence_callback(handle_ptr, android_presence_callback, user_data)
    };
    if result != 0 {
        unsafe { drop(Box::from_raw(state_ptr)) };
        return result as jint;
    }
    let result = unsafe {
        elara_session_set_degradation_callback(handle_ptr, android_degradation_callback, user_data)
    };
    if result != 0 {
        unsafe { drop(Box::from_raw(state_ptr)) };
        return result as jint;
    }
    let mut map = android_callbacks().lock().unwrap();
    map.insert(handle_ptr as usize, state_ptr);
    0
}
