// -*- coding: utf-8 -*-
// Android JNI 桥 (仅 target_os = "android")
//
// 为 Fe-pro (com.shuaiqiu.fuckets100.ResolverNative) 提供 JNI 入口,
// 复用内部授权 + 解析逻辑。同一 .so 既导出普通 C FFI (Python/桌面),
// 也导出 JNI 符号 (Android)。

use std::ffi::CString;

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;

use crate::{authorized_parse_batch, authorized_parse_task};
use crate::auth;

// ============== 配置 / 令牌 ==============

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_setAuthConfig(
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
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_setAccessToken(
    mut env: JNIEnv,
    _class: JClass,
    token: JString,
) {
    let token = CString::new(get_string(&mut env, &token)).unwrap_or_default();
    auth::set_access_token(token.as_ptr());
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_clearAccessToken(
    _env: JNIEnv,
    _class: JClass,
) {
    auth::clear_access_token();
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_setAuthEnabled(
    _env: JNIEnv,
    _class: JClass,
    enabled: jboolean,
) {
    auth::set_auth_enabled(enabled != 0);
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_isAuthEnabled(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if auth::is_auth_enabled() {
        1
    } else {
        0
    }
}

// ============== 授权 / 解析 ==============

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_authorizeFeature(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    to_jstring(&mut env, &auth::authorize_json().to_string())
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_parseTaskAuthorized(
    mut env: JNIEnv,
    _class: JClass,
    content: JString,
    info: JString,
    content2: JString,
) -> jstring {
    let content = get_string(&mut env, &content);
    let info = get_string(&mut env, &info);
    let content2 = get_string(&mut env, &content2);
    to_jstring(&mut env, &authorized_parse_task(&content, &info, &content2).to_string())
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_parseBatchAuthorized(
    mut env: JNIEnv,
    _class: JClass,
    tasks_json: JString,
) -> jstring {
    let tasks_json = get_string(&mut env, &tasks_json);
    to_jstring(&mut env, &authorized_parse_batch(&tasks_json).to_string())
}

// ============== 工具 ==============

fn get_string(env: &mut JNIEnv, value: &JString) -> String {
    env.get_string(value)
        .map(|s| s.into())
        .unwrap_or_default()
}

fn to_jstring(env: &mut JNIEnv, value: &str) -> jstring {
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
