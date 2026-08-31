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

use crate::auth;
use crate::cloud;
use crate::{authorized_parse_batch, authorized_parse_task};

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
    to_jstring(
        &mut env,
        &authorized_parse_task(&content, &info, &content2).to_string(),
    )
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

// ============== ETS100 cloud mode (password login) ==============

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudAuthorizeFeature(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    to_jstring(&mut env, &cloud::cloud_authorize_feature_json().to_string())
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_setCloudConfig(
    mut env: JNIEnv,
    _class: JClass,
    api_base_url: JString,
    cdn_base_url: JString,
    pid: JString,
    secret_key: JString,
    timeout_ms: jlong,
) -> jint {
    let api_base_url = CString::new(get_string(&mut env, &api_base_url)).unwrap_or_default();
    let cdn_base_url = CString::new(get_string(&mut env, &cdn_base_url)).unwrap_or_default();
    let pid = CString::new(get_string(&mut env, &pid)).unwrap_or_default();
    let secret_key = CString::new(get_string(&mut env, &secret_key)).unwrap_or_default();
    cloud::set_cloud_config(
        api_base_url.as_ptr(),
        cdn_base_url.as_ptr(),
        pid.as_ptr(),
        secret_key.as_ptr(),
        timeout_ms.max(0) as u64,
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudGenerateDeviceCode(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    to_jstring(
        &mut env,
        &cloud::cloud_generate_device_code_json().to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudLogin(
    mut env: JNIEnv,
    _class: JClass,
    phone: JString,
    password: JString,
    device_code: JString,
) -> jstring {
    let phone = get_string(&mut env, &phone);
    let password = get_string(&mut env, &password);
    let device_code = get_string(&mut env, &device_code);
    to_jstring(
        &mut env,
        &cloud::cloud_login_json(&phone, &password, &device_code).to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudLoginChangyan(
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
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudSendDeviceBindingCode(
    mut env: JNIEnv,
    _class: JClass,
    phone: JString,
) -> jstring {
    let phone = get_string(&mut env, &phone);
    to_jstring(
        &mut env,
        &cloud::cloud_send_device_binding_code_json(&phone).to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudBindDevice(
    mut env: JNIEnv,
    _class: JClass,
    phone: JString,
    password: JString,
    device_code: JString,
    verification_code: JString,
) -> jstring {
    let phone = get_string(&mut env, &phone);
    let password = get_string(&mut env, &password);
    let device_code = get_string(&mut env, &device_code);
    let verification_code = get_string(&mut env, &verification_code);
    to_jstring(
        &mut env,
        &cloud::cloud_bind_device_json(&phone, &password, &device_code, &verification_code)
            .to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudGetEcardSelection(
    mut env: JNIEnv,
    _class: JClass,
    token: JString,
    preferred_account_id: JString,
) -> jstring {
    let token = get_string(&mut env, &token);
    let preferred_account_id = get_string(&mut env, &preferred_account_id);
    to_jstring(
        &mut env,
        &cloud::cloud_get_ecard_selection_json(&token, &preferred_account_id).to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudGetHomeworkList(
    mut env: JNIEnv,
    _class: JClass,
    token: JString,
    parent_account_id: JString,
    status: JString,
) -> jstring {
    let token = get_string(&mut env, &token);
    let parent_account_id = get_string(&mut env, &parent_account_id);
    let status = get_string(&mut env, &status);
    to_jstring(
        &mut env,
        &cloud::cloud_get_homework_list_json(&token, &parent_account_id, &status).to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudDownloadFile(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    destination: JString,
) -> jstring {
    let url = get_string(&mut env, &url);
    let destination = get_string(&mut env, &destination);
    to_jstring(
        &mut env,
        &cloud::cloud_download_file_json(&url, &destination).to_string(),
    )
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudZipPassword(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    let path = get_string(&mut env, &path);
    to_jstring(&mut env, &cloud::cloud_zip_password_json(&path).to_string())
}

#[no_mangle]
pub extern "system" fn Java_com_shuaiqiu_fuckets100_ResolverNative_cloudDownloadAndParseHomework(
    mut env: JNIEnv,
    _class: JClass,
    base_url: JString,
    contents_json: JString,
    cache_directory: JString,
) -> jstring {
    let base_url = get_string(&mut env, &base_url);
    let contents_json = get_string(&mut env, &contents_json);
    let cache_directory = get_string(&mut env, &cache_directory);
    to_jstring(
        &mut env,
        &cloud::cloud_download_and_parse_homework_json(&base_url, &contents_json, &cache_directory)
            .to_string(),
    )
}

// ============== 工具 ==============

fn get_string(env: &mut JNIEnv, value: &JString) -> String {
    env.get_string(value).map(|s| s.into()).unwrap_or_default()
}

fn to_jstring(env: &mut JNIEnv, value: &str) -> jstring {
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
