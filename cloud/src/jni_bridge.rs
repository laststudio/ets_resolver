use std::ffi::CString;

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;

use crate::{auth, cloud};

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_setAuthConfig(
    mut env: JNIEnv,
    _class: JClass,
    base_url: JString,
    software_key: JString,
    feature_key: JString,
    public_key_b64: JString,
    app_version_code: jlong,
) -> jint {
    let base_url = CString::new(get_string(&mut env, &base_url)).unwrap_or_default();
    let software_key = CString::new(get_string(&mut env, &software_key)).unwrap_or_default();
    let feature_key = CString::new(get_string(&mut env, &feature_key)).unwrap_or_default();
    let public_key_b64 = CString::new(get_string(&mut env, &public_key_b64)).unwrap_or_default();
    auth::set_auth_config(
        base_url.as_ptr(),
        software_key.as_ptr(),
        feature_key.as_ptr(),
        public_key_b64.as_ptr(),
        app_version_code,
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_setAccessToken(
    mut env: JNIEnv,
    _class: JClass,
    token: JString,
) {
    let token = CString::new(get_string(&mut env, &token)).unwrap_or_default();
    auth::set_access_token(token.as_ptr());
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_clearAccessToken(
    _env: JNIEnv,
    _class: JClass,
) {
    auth::clear_access_token();
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_setAuthEnabled(
    _env: JNIEnv,
    _class: JClass,
    enabled: jboolean,
) {
    auth::set_auth_enabled(enabled != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_cloudAuthorizeFeature(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    to_jstring(&mut env, &cloud::cloud_authorize_feature_json().to_string())
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_cloudLoginChangyan(
    mut env: JNIEnv,
    _class: JClass,
    login: JString,
    user_id: JString,
    captcha_result: JString,
    device_code: JString,
    preferred_account_id: JString,
) -> jstring {
    let login = get_string(&mut env, &login);
    let user_id = get_string(&mut env, &user_id);
    let captcha_result = get_string(&mut env, &captcha_result);
    let device_code = get_string(&mut env, &device_code);
    let preferred_account_id = get_string(&mut env, &preferred_account_id);
    to_jstring(
        &mut env,
        &cloud::cloud_login_changyan_json(
            &login,
            &user_id,
            &captcha_result,
            &device_code,
            &preferred_account_id,
        )
        .to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_CloudNative_selfIntegrityCheck(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    to_jstring(&mut env, &self_integrity_json().to_string())
}

fn self_integrity_json() -> serde_json::Value {
    let tracer_pid = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("TracerPid:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .map(str::to_string)
        })
        .unwrap_or_default();
    let mut has_exec = false;
    let mut maps_count = 0usize;
    if let Ok(maps) = std::fs::read_to_string("/proc/self/maps") {
        for line in maps.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("libcloud") {
                maps_count += 1;
                has_exec |= lower.contains("r-xp");
            }
        }
    }
    serde_json::json!({
        "tracer_pid": tracer_pid,
        "maps_count": maps_count,
        "has_exec": has_exec,
        "debugged": !tracer_pid.is_empty() && tracer_pid != "0",
        "ok": maps_count > 0 && has_exec && (tracer_pid.is_empty() || tracer_pid == "0"),
    })
}

fn get_string(env: &mut JNIEnv, value: &JString) -> String {
    env.get_string(value).map(|value| value.into()).unwrap_or_default()
}

fn to_jstring(env: &mut JNIEnv, value: &str) -> jstring {
    env.new_string(value)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
