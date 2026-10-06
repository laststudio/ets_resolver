// -*- coding: utf-8 -*-
// ETS100 cloud API client. The library owns protocol signing and response
// normalization, while callers own credential/session persistence and UI.

use std::ffi::c_char;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use ureq::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use ureq::rustls::client::WebPkiServerVerifier;
use ureq::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use ureq::rustls::{DigitallySignedStruct, SignatureScheme};
use url::Url;

use crate::auth;

const DEFAULT_API_BASE_URL: &str = "https://api.ets100.com";
const DEFAULT_CDN_BASE_URL: &str = "https://cdn.subject.ets100.com";
const DEFAULT_PID: &str = "grlx";
const DEFAULT_SECRET_KEY: &str = "555ffbe95ccf4e9535a110170b445ab8";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const ZIP_FOOTER_SIZE: usize = 336;
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const ETS_CDN_HOST: &str = "cdn.subject.ets100.com";
const ETS_CDN_CERT_NAME: &str = "default.chinanetcenter.com";

#[derive(Clone)]
struct CloudConfig {
    api_base_url: String,
    cdn_base_url: String,
    pid: String,
    secret_key: String,
    timeout: Duration,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            api_base_url: DEFAULT_API_BASE_URL.to_string(),
            cdn_base_url: DEFAULT_CDN_BASE_URL.to_string(),
            pid: DEFAULT_PID.to_string(),
            secret_key: DEFAULT_SECRET_KEY.to_string(),
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        }
    }
}

fn config() -> &'static Mutex<CloudConfig> {
    static CONFIG: OnceLock<Mutex<CloudConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| Mutex::new(CloudConfig::default()))
}

fn config_guard() -> std::sync::MutexGuard<'static, CloudConfig> {
    config()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug)]
struct CloudError {
    code: &'static str,
    message: String,
    api_code: Option<i64>,
    http_status: Option<u16>,
}

impl CloudError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            api_code: None,
            http_status: None,
        }
    }

    fn api(code: &'static str, api_code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            api_code: Some(api_code),
            http_status: None,
        }
    }

    fn http(status: u16, message: impl Into<String>) -> Self {
        Self {
            code: "http_error",
            message: message.into(),
            api_code: None,
            http_status: Some(status),
        }
    }

    fn to_json(&self) -> Value {
        let mut error = Map::new();
        error.insert("code".to_string(), Value::String(self.code.to_string()));
        error.insert("message".to_string(), Value::String(self.message.clone()));
        if let Some(api_code) = self.api_code {
            error.insert("api_code".to_string(), Value::Number(api_code.into()));
        }
        if let Some(http_status) = self.http_status {
            error.insert("http_status".to_string(), Value::Number(http_status.into()));
        }
        json!({"ok": false, "error": error})
    }
}

fn success(result: Value) -> Value {
    json!({"ok": true, "result": result})
}

fn response(result: Result<Value, CloudError>) -> Value {
    match result {
        Ok(value) => success(value),
        Err(error) => error.to_json(),
    }
}

/// Override cloud endpoints/protocol keys. Empty strings preserve current values.
/// Returns 0 on success, 1 for an invalid URL, 2 for an invalid protocol key.
#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn set_cloud_config(
    api_base_url: *const c_char,
    cdn_base_url: *const c_char,
    pid: *const c_char,
    secret_key: *const c_char,
    timeout_ms: u64,
) -> i32 {
    let api_base_url = crate::read_str(api_base_url);
    let cdn_base_url = crate::read_str(cdn_base_url);
    let pid = crate::read_str(pid);
    let secret_key = crate::read_str(secret_key);

    if (!api_base_url.is_empty() && !is_http_url(&api_base_url))
        || (!cdn_base_url.is_empty() && !is_http_url(&cdn_base_url))
    {
        return 1;
    }
    if (!pid.is_empty() && pid.trim().len() < 2)
        || (!secret_key.is_empty() && secret_key.trim().len() < 8)
    {
        return 2;
    }

    let mut cfg = config_guard();
    if !api_base_url.trim().is_empty() {
        cfg.api_base_url = api_base_url.trim().trim_end_matches('/').to_string();
    }
    if !cdn_base_url.trim().is_empty() {
        cfg.cdn_base_url = cdn_base_url.trim().trim_end_matches('/').to_string();
    }
    if !pid.trim().is_empty() {
        cfg.pid = pid.trim().to_string();
    }
    if !secret_key.trim().is_empty() {
        cfg.secret_key = secret_key.trim().to_string();
    }
    if timeout_ms > 0 {
        cfg.timeout = Duration::from_millis(timeout_ms);
    }
    0
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_generate_device_code() -> *mut c_char {
    crate::json_to_ptr(cloud_generate_device_code_json())
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_authorize_feature() -> *mut c_char {
    crate::json_to_ptr(cloud_authorize_feature_json())
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_login(
    phone: *const c_char,
    password: *const c_char,
    device_code: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_login_json(
        &crate::read_str(phone),
        &crate::read_str(password),
        &crate::read_str(device_code),
    ))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_login_changyan(
    login: *const c_char,
    user_id: *const c_char,
    captcha_result: *const c_char,
    device_code: *const c_char,
    preferred_account_id: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_login_changyan_json(
        &crate::read_str(login),
        &crate::read_str(user_id),
        &crate::read_str(captcha_result),
        &crate::read_str(device_code),
        &crate::read_str(preferred_account_id),
    ))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_send_device_binding_code(phone: *const c_char) -> *mut c_char {
    crate::json_to_ptr(cloud_send_device_binding_code_json(&crate::read_str(phone)))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_bind_device(
    phone: *const c_char,
    password: *const c_char,
    device_code: *const c_char,
    verification_code: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_bind_device_json(
        &crate::read_str(phone),
        &crate::read_str(password),
        &crate::read_str(device_code),
        &crate::read_str(verification_code),
    ))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_get_ecard_selection(
    token: *const c_char,
    preferred_account_id: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_get_ecard_selection_json(
        &crate::read_str(token),
        &crate::read_str(preferred_account_id),
    ))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_get_homework_list(
    token: *const c_char,
    parent_account_id: *const c_char,
    status: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_get_homework_list_json(
        &crate::read_str(token),
        &crate::read_str(parent_account_id),
        &crate::read_str(status),
    ))
}

/// 试卷集结构 (含模板包地址，用于取官方 res.json)。
#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_get_homework_struct(
    token: *const c_char,
    homework_id: *const c_char,
    resource_id: *const c_char,
    parent_account_id: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_get_homework_struct_json(
        &crate::read_str(token),
        &crate::read_str(homework_id),
        &crate::read_str(resource_id),
        &crate::read_str(parent_account_id),
    ))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_download_file(
    url: *const c_char,
    destination: *const c_char,
) -> *mut c_char {
    crate::json_to_ptr(cloud_download_file_json(
        &crate::read_str(url),
        &crate::read_str(destination),
    ))
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn cloud_zip_password(path: *const c_char) -> *mut c_char {
    crate::json_to_ptr(cloud_zip_password_json(&crate::read_str(path)))
}

pub fn cloud_login_json(phone: &str, password: &str, device_code: &str) -> Value {
    cloud_authorized_response(|| login(phone, password, device_code))
}

pub fn cloud_login_changyan_json(
    login: &str,
    user_id: &str,
    captcha_result: &str,
    device_code: &str,
    preferred_account_id: &str,
) -> Value {
    cloud_authorized_response(|| {
        login_changyan(
            login,
            user_id,
            captcha_result,
            device_code,
            preferred_account_id,
        )
    })
}

pub fn cloud_generate_device_code_json() -> Value {
    cloud_authorized_response(|| {
        generate_device_code().map(|device_code| json!({"device_code": device_code}))
    })
}

pub fn cloud_authorize_feature_json() -> Value {
    match auth::authorize_for_feature(auth::CLOUD_MODE_FEATURE) {
        Ok(grant) => success(grant),
        Err(error) => json!({
            "ok": false,
            "error": {"code": error.code, "message": error.message}
        }),
    }
}

pub fn cloud_send_device_binding_code_json(phone: &str) -> Value {
    cloud_authorized_response(|| send_device_binding_code(phone))
}

pub fn cloud_bind_device_json(
    phone: &str,
    password: &str,
    device_code: &str,
    verification_code: &str,
) -> Value {
    cloud_authorized_response(|| bind_device(phone, password, device_code, verification_code))
}

pub fn cloud_get_ecard_selection_json(token: &str, preferred_account_id: &str) -> Value {
    cloud_authorized_response(|| get_ecard_selection(token, preferred_account_id))
}

pub fn cloud_get_homework_list_json(token: &str, parent_account_id: &str, status: &str) -> Value {
    cloud_authorized_response(|| get_homework_list(token, parent_account_id, status))
}

/// 试卷集结构: 拿官方"试卷集名 + 模板包地址 + 每个大题的资源包清单"。
///
/// 这是获取官方 **res.json（逐题满分/大题顺序）** 的唯一入口：
///   `/m/homework/struct` → `audio[0].template.url`（模板包，内含 res.json）
///                        → `audio[0].contents[].code_id`（`st1`… 与模板 exam_id `{{st1}}` 对应）
pub fn cloud_get_homework_struct_json(
    token: &str,
    homework_id: &str,
    resource_id: &str,
    parent_account_id: &str,
) -> Value {
    cloud_authorized_response(|| {
        get_homework_struct(token, homework_id, resource_id, parent_account_id)
    })
}

pub fn cloud_download_file_json(url: &str, destination: &str) -> Value {
    cloud_authorized_response(|| download_file(url, destination))
}

pub fn cloud_zip_password_json(path: &str) -> Value {
    cloud_authorized_response(|| {
        zip_password(Path::new(path)).map(|password| json!({"password": password}))
    })
}

fn cloud_authorized_response(operation: impl FnOnce() -> Result<Value, CloudError>) -> Value {
    if !auth::is_auth_enabled() {
        return response(operation());
    }
    match auth::authorize_for_feature(auth::CLOUD_MODE_FEATURE) {
        Ok(_) => response(operation()),
        Err(error) => json!({
            "ok": false,
            "error": {"code": error.code, "message": error.message}
        }),
    }
}

fn validate_required(value: &str, name: &'static str) -> Result<(), CloudError> {
    if value.trim().is_empty() {
        Err(CloudError::new("invalid_input", format!("{name} 不能为空")))
    } else {
        Ok(())
    }
}

fn login(phone: &str, password: &str, device_code: &str) -> Result<Value, CloudError> {
    validate_required(phone, "phone")?;
    validate_required(password, "password")?;
    validate_required(device_code, "device_code")?;
    let value = post_request(
        "/user/login",
        "user/login",
        json!({
            "sn": "test",
            "phone": phone,
            "password": password,
            "device_code": device_code,
            "device_name": "DESKTOP",
            "version": "3",
            "local_ip": "127.0.0.1",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        }),
    )?;
    check_api_code(&value, true)?;
    let token = value
        .pointer("/body/token")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| CloudError::new("invalid_response", "登录响应缺少 token"))?;
    Ok(json!({"token": token}))
}

fn login_changyan(
    login: &str,
    user_id: &str,
    captcha_result: &str,
    device_code: &str,
    preferred_account_id: &str,
) -> Result<Value, CloudError> {
    validate_changyan_input(login, "login", 128)?;
    validate_changyan_input(user_id, "user_id", 128)?;
    validate_changyan_input(captcha_result, "captcha_result", 4096)?;
    validate_changyan_input(device_code, "device_code", 128)?;

    let value = post_request(
        "/user/login-zt",
        "user/login-zt",
        json!({
            "sn": "test",
            "login": login,
            "user_id": user_id,
            "device_name": "Android",
            "device_code": device_code,
            "suc_user_token": captcha_result,
            "suc_device_name": "Other",
            "os_type": "android",
            "system": "4",
            "global_client_version": "",
            "sign_response": 1
        }),
    )?;
    check_api_code(&value, true)?;
    let body = value
        .get("body")
        .and_then(Value::as_object)
        .ok_or_else(|| CloudError::new("invalid_response", "讯飞登录响应缺少 body"))?;
    let token = body
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| CloudError::new("invalid_response", "讯飞登录响应缺少 token"))?;
    let recent_account_id = body
        .get("recent_account_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let preferred = if preferred_account_id.trim().is_empty() {
        recent_account_id
    } else {
        preferred_account_id.trim()
    };
    let selection = get_ecard_selection(token, preferred)?;
    let mut result = selection
        .as_object()
        .cloned()
        .ok_or_else(|| CloudError::new("invalid_response", "账号选择结果格式错误"))?;
    result.insert("token".to_string(), Value::String(token.to_string()));
    if !recent_account_id.is_empty() {
        result.insert(
            "recent_account_id".to_string(),
            Value::String(recent_account_id.to_string()),
        );
    }
    Ok(Value::Object(result))
}

fn validate_changyan_input(
    value: &str,
    name: &'static str,
    max_bytes: usize,
) -> Result<(), CloudError> {
    validate_required(value, name)?;
    if value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(CloudError::new(
            "invalid_input",
            format!("{name} 格式错误"),
        ));
    }
    Ok(())
}

fn send_device_binding_code(phone: &str) -> Result<Value, CloudError> {
    validate_required(phone, "phone")?;
    let value = post_request(
        "/code/send-user-exist",
        "code/send-user-exist",
        json!({
            "sn": "test",
            "phone": phone,
            "email": "",
            "class_id": "1",
            "use": "3",
            "system": "4",
            "global_client_version": "",
            "sign_response": 1
        }),
    )?;
    check_api_code(&value, false)?;
    Ok(json!({"sent": true}))
}

fn bind_device(
    phone: &str,
    password: &str,
    device_code: &str,
    verification_code: &str,
) -> Result<Value, CloudError> {
    validate_required(phone, "phone")?;
    validate_required(password, "password")?;
    validate_required(device_code, "device_code")?;
    let code = if verification_code.trim().is_empty() {
        "0"
    } else {
        verification_code.trim()
    };
    let value = post_request(
        "/user/rebind-code",
        "user/rebind-code",
        json!({
            "sn": "test",
            "phone": phone,
            "email": "",
            "password": password,
            "code": code,
            "version": "2",
            "device_name": "1337",
            "device_code": device_code,
            "local_ip": "127.0.0.1",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        }),
    )?;
    check_api_code(&value, false)?;
    let token = value
        .pointer("/body/token")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| CloudError::new("invalid_response", "设备绑定响应缺少 token"))?;
    Ok(json!({"token": token}))
}

fn get_ecard_selection(token: &str, preferred_account_id: &str) -> Result<Value, CloudError> {
    validate_required(token, "token")?;
    let value = post_request(
        "/m/ecard/list",
        "m/ecard/list",
        json!({
            "sn": "test",
            "token": token,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        }),
    )?;
    check_api_code(&value, true)?;
    parse_ecard_selection(&value, preferred_account_id)
}

fn get_homework_list(
    token: &str,
    parent_account_id: &str,
    status: &str,
) -> Result<Value, CloudError> {
    validate_required(token, "token")?;
    validate_required(parent_account_id, "parent_account_id")?;
    let status = if status.trim().is_empty() {
        "1"
    } else {
        status.trim()
    };
    if !matches!(status, "1" | "2" | "3") {
        return Err(CloudError::new("invalid_input", "status 只能是 1、2 或 3"));
    }
    let value = post_request(
        "/g/homework/list",
        "g/homework/list",
        json!({
            "sn": "test",
            "token": token,
            "parent_account_id": parent_account_id,
            "limit": "0",
            "status": status,
            "offset": "0",
            "max_end_time": "",
            "max_homework_id": "",
            "min_end_time": "",
            "min_homework_id": "",
            "get_to_do_count": 1,
            "show_old_homework": 1,
            "parent_homework_id": "",
            "get_all_count": 1,
            "check_pass": 1,
            "get_to_overtime_count": 1,
            "version": "3",
            "system": "4",
            "global_client_version": "5.4.5",
            "sign_response": 1
        }),
    )?;
    check_api_code(&value, true)?;
    parse_homework_list(&value, status)
}

fn post_request(endpoint: &str, route: &str, params: Value) -> Result<Value, CloudError> {
    let cfg = config_guard().clone();
    let timestamp = now_unix();
    let payload = build_request_payload(route, params, timestamp, &cfg)?;
    let url = format!("{}{}", cfg.api_base_url, endpoint);
    let request = ureq::post(&url)
        .timeout(cfg.timeout)
        .set("User-Agent", "libcurl-agent/1.0")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("Accept", "*/*");
    let response = request.send_string(&payload);
    let text = match response {
        Ok(response) => response
            .into_string()
            .map_err(|error| CloudError::new("network_error", format!("读取响应失败: {error}")))?,
        Err(ureq::Error::Status(status, response)) => {
            let text = response.into_string().unwrap_or_default();
            return Err(CloudError::http(
                status,
                response_message(&text).unwrap_or_else(|| format!("ETS100 返回 HTTP {status}")),
            ));
        }
        Err(error) => {
            return Err(CloudError::new(
                "network_error",
                format!("请求 ETS100 失败: {error}"),
            ))
        }
    };
    parse_response(&text)
}

fn build_request_payload(
    route: &str,
    params: Value,
    timestamp: i64,
    cfg: &CloudConfig,
) -> Result<String, CloudError> {
    let body = serde_json::to_string(&json!([{"r": route, "params": params}]))
        .map_err(|_| CloudError::new("invalid_input", "无法序列化请求"))?;
    let body_base64 = STANDARD.encode(body.as_bytes());
    let sign_input = format!("{}{}{}{}", cfg.pid, timestamp, body_base64, cfg.secret_key);
    let sign = format!("{:x}", md5::compute(sign_input.as_bytes()));
    serde_json::to_string(&json!({
        "body": body_base64,
        "head": {
            "version": "1.0",
            "sign": sign,
            "pid": cfg.pid,
            "time": timestamp
        }
    }))
    .map_err(|_| CloudError::new("invalid_input", "无法序列化签名请求"))
}

fn parse_response(text: &str) -> Result<Value, CloudError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|_| CloudError::new("invalid_response", "ETS100 响应不是有效 JSON"))?;
    match value {
        Value::Array(values) => values
            .into_iter()
            .next()
            .ok_or_else(|| CloudError::new("invalid_response", "ETS100 返回空数组")),
        Value::Object(_) => Ok(value),
        _ => Err(CloudError::new("invalid_response", "ETS100 响应格式错误")),
    }
}

fn check_api_code(value: &Value, allow_missing: bool) -> Result<(), CloudError> {
    let code = value.get("code").and_then(value_as_i64);
    if code.is_none() && allow_missing {
        return Ok(());
    }
    let code = code.unwrap_or(-1);
    if code == 0 {
        return Ok(());
    }
    let message = value
        .get("msg")
        .and_then(Value::as_str)
        .filter(|message| !message.is_empty())
        .unwrap_or("ETS100 请求失败");
    let error_code = match code {
        30008 => "sms_verification_required",
        30014 => "device_bind_required",
        60001 => "invalid_verification_code",
        _ => "api_error",
    };
    Err(CloudError::api(error_code, code, message))
}

fn parse_ecard_selection(value: &Value, preferred_account_id: &str) -> Result<Value, CloudError> {
    let body = value
        .get("body")
        .and_then(Value::as_object)
        .ok_or_else(|| CloudError::new("invalid_response", "ecard/list 响应缺少 body"))?;
    let accounts: Vec<Value> = body
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_object()
                .map(|account| normalize_account(key, account))
        })
        .collect();
    if accounts.is_empty() {
        return Err(CloudError::new("no_account", "未找到账户信息"));
    }
    let preferred = preferred_account_id.trim();
    let selected = accounts
        .iter()
        .find(|account| {
            !preferred.is_empty()
                && account.get("id").and_then(Value::as_str) == Some(preferred)
                && account.get("is_valid").and_then(Value::as_bool) == Some(true)
        })
        .or_else(|| {
            accounts
                .iter()
                .find(|account| account.get("is_valid").and_then(Value::as_bool) == Some(true))
        })
        .unwrap_or(&accounts[0])
        .clone();
    let parent_account_id = selected
        .get("parent_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| CloudError::new("invalid_response", "所选账户缺少 parent_id"))?;
    let valid_accounts = accounts
        .iter()
        .filter(|account| account.get("is_valid").and_then(Value::as_bool) == Some(true))
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({
        "parent_account_id": parent_account_id,
        "selected_account": selected,
        "valid_accounts": valid_accounts,
        "accounts": accounts
    }))
}

fn normalize_account(key: &str, account: &Map<String, Value>) -> Value {
    let get = |name: &str| value_as_string(account.get(name));
    let status = get("status");
    let mobile_status = get("mobile_status");
    let out_of_date = get("out_of_date");
    let class_id = get("class_id");
    json!({
        "key": key,
        "id": get("id"),
        "parent_id": get("parent_id"),
        "user_account_id": get("user_account_id"),
        "name": get("name"),
        "grade": get("grade"),
        "status": status,
        "mobile_status": mobile_status,
        "out_of_date": out_of_date,
        "class_id": class_id,
        "class_name": get("class_name"),
        "machine_code_status": get("machine_code_status"),
        "is_valid": status == "0" && out_of_date == "0" && !class_id.trim().is_empty() && mobile_status == "1"
    })
}

fn get_homework_struct(
    token: &str,
    homework_id: &str,
    resource_id: &str,
    parent_account_id: &str,
) -> Result<Value, CloudError> {
    validate_required(token, "token")?;
    validate_required(homework_id, "homework_id")?;
    let mut params = json!({
        "sn": "test",
        "token": token,
        "id": homework_id,
        "resource_id": resource_id,
        "version": "3",
        "system": "4",
        "global_client_version": "5.4.5",
        "sign_response": 1
    });
    if !parent_account_id.trim().is_empty() {
        params["parent_account_id"] = json!(parent_account_id);
    }
    let value = post_request("/m/homework/struct", "m/homework/struct", params)?;
    check_api_code(&value, true)?;
    parse_homework_struct(&value)
}

/// 解析 `/m/homework/struct` 响应。
///
/// 真实响应形态（广东 30 分卷实测）：
/// ```jsonc
/// {"body": {
///   "audio": [{
///     "set_name": "广州8年级离线个人版模拟试题2(copy)",
///     "follow_type": 0,
///     "template": {"id": "5", "size": 1798943, "url": "834c62a1....zip"},
///     "contents": [{"code_id": "st1", "group_name": "模仿朗读", "real_id": "49062",
///                   "url": "6799....zip", "size": 577212, "min_unit": 1,
///                   "read_type": 5, "ppr": "7c11....zip"}]
///   }],
///   "combination": []
/// }}
/// ```
/// `contents[].code_id`（`st1`…）与模板 `exam_list[].exam_id`（`{{st1}}`）一一对应，
/// 这就是"资源包 → 官方大题"的官方挂载键。
fn parse_homework_struct(value: &Value) -> Result<Value, CloudError> {
    let body = value
        .get("body")
        .ok_or_else(|| CloudError::new("invalid_response", "homework/struct 响应缺少 body"))?;
    let default_cdn = format!("{}/made", config_guard().cdn_base_url.trim_end_matches('/'));
    let audio = body
        .get("audio")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let first = audio.first().and_then(Value::as_object);
    if first.is_none() {
        return Err(CloudError::new(
            "invalid_response",
            "homework/struct 未返回 audio（该作业可能没有试卷集模板）",
        ));
    }
    let first = first.unwrap();
    let template = first.get("template").and_then(Value::as_object);
    let template_url = template
        .map(|t| value_as_string(t.get("url")))
        .unwrap_or_default();
    let contents = first
        .get("contents")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_object)
                .enumerate()
                .map(|(index, c)| {
                    let url = value_as_string(c.get("url"));
                    json!({
                        "order": index,
                        "code_id": value_as_string(c.get("code_id")),
                        "group_name": value_as_string(c.get("group_name")),
                        "real_id": value_as_string(
                            c.get("real_id").or_else(|| c.get("entity_id"))
                        ),
                        "url": url,
                        "download_url": resolve_download_url(&default_cdn, &url),
                        "ppr": value_as_string(c.get("ppr")),
                        "size": c.get("size").and_then(value_as_i64).unwrap_or(0),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(json!({
        "set_name": value_as_string(first.get("set_name")),
        "follow_type": first.get("follow_type").and_then(value_as_i64).unwrap_or(0),
        "template": {
            "id": template.map(|t| value_as_string(t.get("id"))).unwrap_or_default(),
            "url": template_url,
            "download_url": if template_url.is_empty() {
                String::new()
            } else {
                resolve_download_url(&default_cdn, &template_url)
            },
            "size": template
                .and_then(|t| t.get("size"))
                .and_then(value_as_i64)
                .unwrap_or(0),
        },
        "contents": contents,
        "combination": body.get("combination").cloned().unwrap_or_else(|| json!([])),
        // res.json 在模板包里，需要调用方下载解压后交给解析器的 res_template/parse_homework_set
        "res_json_hint": "下载 template.download_url → 解压 → 读取 res.json（模板包未加密）"
    }))
}

fn parse_homework_list(value: &Value, status: &str) -> Result<Value, CloudError> {
    let body = value
        .get("body")
        .and_then(Value::as_object)
        .ok_or_else(|| CloudError::new("invalid_response", "homework/list 响应缺少 body"))?;
    let default_cdn = config_guard().cdn_base_url.clone();
    let base_url = body
        .get("base_url")
        .and_then(Value::as_str)
        .filter(|url| !url.is_empty())
        .unwrap_or(&default_cdn)
        .replace("http://", "https://");
    let data = body
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let homeworks = data
        .iter()
        .filter_map(Value::as_object)
        .map(|homework| {
            let struct_obj = homework.get("struct").and_then(Value::as_object);
            let contents = struct_obj
                .and_then(|value| value.get("contents"))
                .and_then(Value::as_array)
                .map(|contents| {
                    contents
                        .iter()
                        .filter_map(Value::as_object)
                        .enumerate()
                        .map(|(index, content)| {
                            let url = value_as_string(content.get("url"));
                            json!({
                                "group_name": value_as_string(content.get("group_name")),
                                "url": url,
                                "download_url": resolve_download_url(&base_url, &url),
                                // 新增：分节挂载与顺序所需字段（原先被裁掉，导致
                                // Fe-pro/后端 只能各自重写一份列表解析）
                                "real_id": value_as_string(
                                    content.get("real_id").or_else(|| content.get("entity_id"))
                                ),
                                "order": content
                                    .get("order")
                                    .and_then(value_as_i64)
                                    .unwrap_or(index as i64),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            // 顶层取不到就落到 struct（官方不同接口字段层级不一致）
            let pick = |key: &str| -> Option<&Value> {
                homework
                    .get(key)
                    .filter(|v| !v.is_null())
                    .or_else(|| struct_obj.and_then(|s| s.get(key)).filter(|v| !v.is_null()))
            };
            let pick_str = |key: &str| -> String { value_as_string(pick(key)) };
            let pick_i64 = |key: &str| -> i64 { pick(key).and_then(value_as_i64).unwrap_or(0) };
            let pick_f64 = |key: &str| -> f64 {
                pick(key)
                    .and_then(|v| match v {
                        Value::Number(n) => n.as_f64(),
                        Value::String(s) => s.trim().parse().ok(),
                        _ => None,
                    })
                    .unwrap_or(0.0)
            };
            json!({
                "id": value_as_string(homework.get("id")),
                "name": value_as_string(homework.get("name")),
                "contents": contents,
                // ---- 分类与卷面满分所需的全部字段（与后端/Fe-pro 既有口径对齐）----
                "subject_name": pick_str("subject_name"),
                "set_type": pick_i64("set_type"),
                "set_id": pick_str("set_id"),
                "resource_id": pick_str("resource_id"),
                "pk": pick_i64("pk"),
                "kind": pick_str("kind"),
                "mock": pick_i64("mock"),
                "exam": pick_i64("exam"),
                "status": pick("status").cloned().unwrap_or(Value::Null),
                "create_time": pick_str("create_time"),
                "end_time": pick_str("end_time"),
                "point": pick_f64("point"),
                "total_point": pick_f64("total_point"),
                "total_score": pick_f64("total_score"),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({"status": status, "base_url": base_url, "homeworks": homeworks}))
}

fn download_file(url: &str, destination: &str) -> Result<Value, CloudError> {
    validate_required(url, "url")?;
    validate_required(destination, "destination")?;
    if !Url::parse(url)
        .ok()
        .is_some_and(|url| url.scheme() == "https")
    {
        return Err(CloudError::new(
            "invalid_input",
            "下载 url 必须是 HTTPS 地址",
        ));
    }
    let destination = PathBuf::from(destination);
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| CloudError::new("io_error", format!("创建下载目录失败: {error}")))?;
    }
    let temporary = PathBuf::from(format!("{}.part", destination.display()));
    let _ = fs::remove_file(&temporary);
    let timeout = config_guard().timeout;
    let result = (|| {
        let response = match download_agent(url)?
            .get(url)
            .timeout(timeout)
            .set("User-Agent", "libcurl-agent/1.0")
            .set("Accept", "*/*")
            .call()
        {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                let text = response.into_string().unwrap_or_default();
                return Err(CloudError::http(
                    status,
                    response_message(&text).unwrap_or_else(|| format!("下载返回 HTTP {status}")),
                ));
            }
            Err(error) => {
                return Err(CloudError::new(
                    "network_error",
                    format!("下载失败: {error}"),
                ))
            }
        };
        let total_bytes = response
            .header("Content-Length")
            .and_then(|value| value.parse::<u64>().ok());
        if total_bytes.is_some_and(|bytes| bytes > MAX_DOWNLOAD_BYTES) {
            return Err(CloudError::new(
                "resource_too_large",
                "下载资源超过 512 MiB",
            ));
        }
        let mut reader = response.into_reader();
        let mut file = File::create(&temporary)
            .map_err(|error| CloudError::new("io_error", format!("创建临时文件失败: {error}")))?;
        let downloaded_bytes =
            std::io::copy(&mut reader.by_ref().take(MAX_DOWNLOAD_BYTES + 1), &mut file).map_err(
                |error| CloudError::new("io_error", format!("写入下载文件失败: {error}")),
            )?;
        if downloaded_bytes > MAX_DOWNLOAD_BYTES {
            return Err(CloudError::new(
                "resource_too_large",
                "下载资源超过 512 MiB",
            ));
        }
        file.flush()
            .map_err(|error| CloudError::new("io_error", format!("刷新下载文件失败: {error}")))?;
        if !has_zip_magic(&temporary)? {
            return Err(CloudError::new("invalid_zip", "下载结果不是 ZIP 文件"));
        }
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|error| CloudError::new("io_error", format!("替换旧文件失败: {error}")))?;
        }
        fs::rename(&temporary, &destination)
            .map_err(|error| CloudError::new("io_error", format!("保存下载文件失败: {error}")))?;
        Ok(json!({
            "path": destination.to_string_lossy(),
            "downloaded_bytes": downloaded_bytes,
            "total_bytes": total_bytes
        }))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn download_agent(url: &str) -> Result<ureq::Agent, CloudError> {
    let parsed =
        Url::parse(url).map_err(|_| CloudError::new("invalid_input", "下载 URL 格式错误"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some(ETS_CDN_HOST) {
        return Ok(ureq::AgentBuilder::new().redirects(0).build());
    }

    let roots =
        ureq::rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let verifier = WebPkiServerVerifier::builder(roots.into())
        .build()
        .map_err(|error| CloudError::new("tls_error", format!("无法配置 CDN TLS: {error}")))?;
    let tls = ureq::rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(EtsCdnVerifier { verifier }))
        .with_no_client_auth();
    Ok(ureq::AgentBuilder::new()
        .redirects(0)
        .tls_config(std::sync::Arc::new(tls))
        .build())
}

#[derive(Debug)]
struct EtsCdnVerifier {
    verifier: std::sync::Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for EtsCdnVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, ureq::rustls::Error> {
        if server_name.to_str() != ETS_CDN_HOST {
            return Err(ureq::rustls::Error::General(
                "unexpected ETS CDN server name".into(),
            ));
        }
        let certificate_name = ServerName::try_from(ETS_CDN_CERT_NAME)
            .map_err(|_| ureq::rustls::Error::General("invalid ETS CDN certificate name".into()))?;
        self.verifier.verify_server_cert(
            end_entity,
            intermediates,
            &certificate_name,
            ocsp_response,
            now,
        )
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, ureq::rustls::Error> {
        self.verifier.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, ureq::rustls::Error> {
        self.verifier.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.verifier.supported_verify_schemes()
    }
}

fn zip_password(path: &Path) -> Result<String, CloudError> {
    let mut file = File::open(path)
        .map_err(|error| CloudError::new("io_error", format!("无法打开 ZIP: {error}")))?;
    let length = file
        .metadata()
        .map_err(|error| CloudError::new("io_error", format!("无法读取 ZIP 信息: {error}")))?
        .len();
    if length < ZIP_FOOTER_SIZE as u64 {
        return Err(CloudError::new("invalid_zip", "ZIP 文件过小"));
    }
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::End(-(ZIP_FOOTER_SIZE as i64)))
        .map_err(|error| CloudError::new("io_error", format!("无法定位 ZIP 尾部: {error}")))?;
    let mut footer = [0u8; ZIP_FOOTER_SIZE];
    file.read_exact(&mut footer)
        .map_err(|error| CloudError::new("io_error", format!("无法读取 ZIP 尾部: {error}")))?;
    zip_password_from_footer(&footer)
}

fn zip_password_from_footer(footer: &[u8]) -> Result<String, CloudError> {
    if footer.len() < ZIP_FOOTER_SIZE {
        return Err(CloudError::new("invalid_zip", "ZIP 尾部长度不足"));
    }
    if &footer[0..8] != b"MSTCHINA" && &footer[144..149] != b"EPLAT" {
        return Err(CloudError::new("invalid_zip", "ZIP 缺少 ETS100 尾部标记"));
    }
    let first = format!("{:X}", md5::compute(&footer[16..144]));
    let second = format!("{:X}", md5::compute(first.as_bytes()));
    Ok(format!("{first}{second}"))
}

fn has_zip_magic(path: &Path) -> Result<bool, CloudError> {
    let mut file = File::open(path)
        .map_err(|error| CloudError::new("io_error", format!("无法验证下载文件: {error}")))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|error| CloudError::new("invalid_zip", format!("ZIP 文件不完整: {error}")))?;
    Ok(matches!(
        magic,
        [0x50, 0x4b, 0x03, 0x04] | [0x50, 0x4b, 0x05, 0x06] | [0x50, 0x4b, 0x07, 0x08]
    ))
}

fn generate_device_code() -> Result<String, CloudError> {
    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random)
        .map_err(|_| CloudError::new("random_error", "无法生成设备码"))?;
    let first = random[0..8]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .take(120)
        .collect::<String>();
    let second = random[8..16]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    let first_md5 = format!("{:x}", md5::compute(first.as_bytes()));
    let second_md5 = format!("{:x}", md5::compute(second.as_bytes()));
    Ok(format!("{}|{}", &first_md5[8..24], &second_md5[8..24]))
}

fn resolve_download_url(base_url: &str, url: &str) -> String {
    if url.starts_with("https://") {
        return url.to_string();
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return format!("https://{rest}");
    }
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        url.trim_start_matches('/')
    )
}

fn value_as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn value_as_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn response_message(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text).ok().and_then(|value| {
        value
            .get("msg")
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn is_http_url(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    /// 真实 /m/homework/struct 响应（contents 裁到 2 项）
    const REAL_HOMEWORK_STRUCT: &str = r#"{"body": {"audio": [{"template": {"size": 1798943, "id": "5", "url": "834c62a165faa2e86182c97bba2ab925.zip"}, "follow_type": 0, "set_name": "广州8年级离线个人版模拟试题2(copy)", "group_name": "", "set_id": "4030", "zip_url": "", "contents": [{"read_type": 5, "is_repeat_type": 0, "entity_foreign_id": "", "ppr": "7c1188510e0edea75dbc6c7603383bc0.zip", "entity_type": "1", "size": 577212, "min_unit": 1, "real_id": "49062", "group_name": "模仿朗读", "code_id": "st1", "url": "6799094efc8204a4adc040076e8fedf1.zip", "first_type_name": "模仿朗读"}, {"read_type": 5, "is_repeat_type": 0, "entity_foreign_id": "", "ppr": "c19699c63b3a1a10629999a7733f39fa.zip", "entity_type": "1", "size": 448728, "min_unit": 1, "real_id": "49063", "group_name": "听选信息1", "code_id": "st2", "url": "aa1f8fdfba8704a6ee90f7be48659b60.zip", "first_type_name": "听选信息"}]}], "combination": []}}"#;

    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn device_code_matches_android_shape() {
        let value = generate_device_code().unwrap();
        assert_eq!(value.len(), 33);
        assert_eq!(value.as_bytes()[16], b'|');
        assert!(value
            .replace('|', "")
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn cloud_operation_does_not_run_without_remote_auth() {
        auth::set_auth_enabled(true);
        let called = AtomicBool::new(false);
        let result = cloud_authorized_response(|| {
            called.store(true, Ordering::SeqCst);
            Ok(Value::Null)
        });
        assert_eq!(result["ok"], false);
        assert_eq!(result["error"]["code"], "auth_not_configured");
        assert!(!called.load(Ordering::SeqCst));
        auth::set_auth_enabled(false);
    }

    #[test]
    fn changyan_inputs_are_bounded_and_reject_controls() {
        assert!(validate_changyan_input("user", "login", 128).is_ok());
        assert!(validate_changyan_input("", "login", 128).is_err());
        assert!(validate_changyan_input("bad\nvalue", "login", 128).is_err());
        assert!(validate_changyan_input(&"x".repeat(129), "login", 128).is_err());
    }

    #[test]
    fn api_errors_are_normalized() {
        let error = check_api_code(&json!({"code": 30014, "msg": "bind"}), true).unwrap_err();
        assert_eq!(error.code, "device_bind_required");
        assert_eq!(error.api_code, Some(30014));
    }

    #[test]
    fn ecard_prefers_requested_valid_account() {
        let value = json!({"body": {
            "0": {"id": "first", "parent_id": "p1", "status": "0", "out_of_date": "0", "class_id": "1", "mobile_status": "1"},
            "1": {"id": "wanted", "parent_id": "p2", "status": "0", "out_of_date": "0", "class_id": "2", "mobile_status": "1"}
        }});
        let parsed = parse_ecard_selection(&value, "wanted").unwrap();
        assert_eq!(parsed["parent_account_id"], "p2");
        assert_eq!(parsed["selected_account"]["id"], "wanted");
    }

    #[test]
    fn ecard_preserves_server_order_and_rejects_blank_class() {
        let value = json!({"body": {
            "second": {"id": "blank", "parent_id": "p1", "status": "0", "out_of_date": "0", "class_id": "   ", "mobile_status": "1"},
            "first": {"id": "fallback", "parent_id": "p2", "status": "1", "out_of_date": "0", "class_id": "2", "mobile_status": "1"}
        }});
        let parsed = parse_ecard_selection(&value, "").unwrap();
        assert_eq!(parsed["accounts"][0]["id"], "blank");
        assert_eq!(parsed["accounts"][0]["is_valid"], false);
        assert_eq!(parsed["selected_account"]["id"], "blank");
    }

    #[test]
    fn homework_response_contains_safe_absolute_urls() {
        let value = json!({"body": {
            "base_url": "http://cdn.example.test/",
            "data": [{"id": "1", "name": "paper", "struct": {"contents": [{"group_name": "read", "url": "/a.zip"}]}}]
        }});
        let parsed = parse_homework_list(&value, "1").unwrap();
        assert_eq!(parsed["base_url"], "https://cdn.example.test/");
        assert_eq!(
            parsed["homeworks"][0]["contents"][0]["download_url"],
            "https://cdn.example.test/a.zip"
        );
    }

    #[test]
    fn homework_struct_exposes_template_url_and_code_ids() {
        // 真实 /m/homework/struct 响应（广东 30 分卷，contents 裁到 2 项）
        let fixture: Value = serde_json::from_str(REAL_HOMEWORK_STRUCT).unwrap();
        let parsed = parse_homework_struct(&fixture).unwrap();

        assert_eq!(parsed["set_name"], "广州8年级离线个人版模拟试题2(copy)");
        assert_eq!(parsed["template"]["id"], "5");
        assert_eq!(parsed["template"]["url"], "834c62a165faa2e86182c97bba2ab925.zip");
        assert_eq!(parsed["template"]["size"], 1798943);
        assert!(parsed["template"]["download_url"]
            .as_str()
            .unwrap()
            .ends_with("/made/834c62a165faa2e86182c97bba2ab925.zip"));

        let contents = parsed["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0]["code_id"], "st1");
        assert_eq!(contents[0]["group_name"], "模仿朗读");
        assert_eq!(contents[0]["real_id"], "49062");
        assert!(contents[0]["download_url"].as_str().unwrap().ends_with(".zip"));
        assert_eq!(contents[1]["code_id"], "st2");
    }

    #[test]
    fn homework_struct_without_audio_is_a_clear_error() {
        let value = json!({"body": {"audio": [], "combination": []}});
        let err = parse_homework_struct(&value).unwrap_err();
        assert_eq!(err.code, "invalid_response");
        assert!(err.message.contains("模板"));
    }

    #[test]
    fn homework_item_exposes_classification_fields() {
        // 分类所需字段原先被裁掉，导致各端只能自己重写一份列表解析 —— 这里锁住回归
        let value = json!({"body": {
            "base_url": "https://cdn.example.test",
            "data": [{
                "id": "9527",
                "name": "Unit 3 综合练习",
                "subject_name": "英语",
                "set_type": 1,
                "set_id": "set-9981",
                "resource_id": "58",
                "pk": 0,
                "kind": "mock",
                "mock": 1,
                "exam": 0,
                "status": 1,
                "end_time": "2026-10-10 20:00:00",
                "point": 22.5,
                "total_point": 30,
                "struct": {"contents": [
                    {"group_name": "模仿朗读", "url": "a.zip", "real_id": "e-1", "order": 0},
                    {"group_name": "听选信息", "url": "b.zip", "entity_id": "e-2", "order": 1}
                ]}
            }]
        }});
        let parsed = parse_homework_list(&value, "1").unwrap();
        let hw = &parsed["homeworks"][0];
        assert_eq!(hw["set_type"], 1);
        assert_eq!(hw["set_id"], "set-9981");
        assert_eq!(hw["resource_id"], "58");
        assert_eq!(hw["kind"], "mock");
        assert_eq!(hw["mock"], 1);
        assert_eq!(hw["total_point"], 30.0);
        assert_eq!(hw["point"], 22.5);
        assert_eq!(hw["subject_name"], "英语");
        // contents 的顺序与挂载键
        assert_eq!(hw["contents"][0]["real_id"], "e-1");
        assert_eq!(hw["contents"][0]["order"], 0);
        assert_eq!(hw["contents"][1]["real_id"], "e-2");
        assert_eq!(hw["contents"][1]["order"], 1);
    }

    #[test]
    fn homework_item_reads_fields_from_struct_fallback() {
        let value = json!({"body": {
            "base_url": "https://cdn.example.test",
            "data": [{"id": "1", "name": "x", "struct": {
                "set_type": 6, "resource_id": "77", "contents": []
            }}]
        }});
        let parsed = parse_homework_list(&value, "1").unwrap();
        assert_eq!(parsed["homeworks"][0]["set_type"], 6);
        assert_eq!(parsed["homeworks"][0]["resource_id"], "77");
    }

    #[test]
    fn zip_password_matches_two_stage_md5_algorithm() {
        let mut footer = [0u8; ZIP_FOOTER_SIZE];
        footer[0..8].copy_from_slice(b"MSTCHINA");
        for (index, byte) in footer[16..144].iter_mut().enumerate() {
            *byte = index as u8;
        }
        let expected_first = format!("{:X}", md5::compute(&footer[16..144]));
        let expected_second = format!("{:X}", md5::compute(expected_first.as_bytes()));
        assert_eq!(
            zip_password_from_footer(&footer).unwrap(),
            format!("{expected_first}{expected_second}")
        );
    }

    #[test]
    fn request_signature_is_stable() {
        let cfg = CloudConfig::default();
        let payload =
            build_request_payload("user/login", json!({"phone": "1"}), 1_700_000_000, &cfg)
                .unwrap();
        let value: Value = serde_json::from_str(&payload).unwrap();
        let body = value["body"].as_str().unwrap();
        let expected = format!(
            "{:x}",
            md5::compute(format!(
                "{}{}{}{}",
                cfg.pid, 1_700_000_000i64, body, cfg.secret_key
            ))
        );
        assert_eq!(value["head"]["sign"], expected);
    }

}
