// ETS100 云端模式独立动态库 (libcloud.so)。
// 不包含答案解析器；仅负责云端授权、登录、账号、列表、下载和 ZIP 协议。

#[path = "../../src/auth.rs"]
mod auth;
#[path = "../../src/cloud.rs"]
mod cloud;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde_json::Value;

#[cfg(target_os = "android")]
mod jni_bridge;

pub(crate) fn read_str(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(value) => value.to_string(),
        Err(_) => String::new(),
    }
}

pub(crate) fn json_to_ptr(result: Value) -> *mut c_char {
    let json = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
    CString::new(json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn cloud_set_auth_config(
    base_url: *const c_char,
    software_key: *const c_char,
    feature_key: *const c_char,
    public_key_b64: *const c_char,
    app_version_code: i64,
) -> i32 {
    auth::set_auth_config(
        base_url,
        software_key,
        feature_key,
        public_key_b64,
        app_version_code,
    )
}

#[no_mangle]
pub extern "C" fn cloud_set_access_token(token: *const c_char) {
    auth::set_access_token(token);
}

#[no_mangle]
pub extern "C" fn cloud_clear_access_token() {
    auth::clear_access_token();
}

#[no_mangle]
pub extern "C" fn cloud_set_auth_enabled(enabled: bool) {
    auth::set_auth_enabled(enabled);
}

#[no_mangle]
pub extern "C" fn cloud_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
