// -*- coding: utf-8 -*-
// 内部后端授权 (仅非 wasm 目标)
//
// 解析前由库内部调用 lastudio-platform 的 /client-auth/features/authorize:
//   1. 生成本次操作的随机 operation_nonce
//   2. 用当前 access_token 请求实时功能授权 (answer_parser)
//   3. 校验后端返回的 Ed25519 签名授权 (绑定 software/feature/nonce/时间)
//   4. 通过后才真正执行解析; 拒绝则返回结构化错误
//
// 这样即使外部 Java 层被 patch 绕过, 库内部仍然强制联网校验, 无法直接解析。

use std::collections::HashMap;
use std::ffi::c_char;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_compact::{PublicKey, Signature};
use serde_json::{json, Map, Value};

/// 默认服务器 Ed25519 公钥 (urlsafe base64 no-pad, 32 字节)。
/// 通过 `set_auth_config` 可覆盖 (开发环境/换钥)。
const DEFAULT_PUBLIC_KEY_B64: &str = "iIsgUPrBJidorQGIs-BJCt5EiLrRHVbAWStnNaPn3cI";

const FEATURE_AUTHORIZE_PATH: &str = "/client-auth/features/authorize";
pub const ANSWER_PARSER_FEATURE: &str = "answer_parser";
pub const CLOUD_MODE_FEATURE: &str = "cloud_mode";

/// issued_at 允许的最大时钟偏差 (秒)。
const MAX_ISSUE_SKEW_SECONDS: i64 = 60;
/// expires_at - issued_at 的最大合法有效期 (秒), 超出视为异常授权。
const MAX_GRANT_VALIDITY_SECONDS: i64 = 600;

#[derive(Debug, Clone)]
pub struct AuthError {
    pub code: String,
    pub message: String,
}

impl AuthError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

pub struct AuthState {
    pub enabled: bool,
    pub base_url: String,
    pub software_key: String,
    pub feature_key: String,
    pub app_version_code: i64,
    pub public_key_b64: String,
    pub access_token: String,
    /// 每个功能最近一次成功的授权 grant (有效期约 120s)。
    /// 每次复用前仍会本地重新验签 + 校验过期。
    pub cached_grants: HashMap<String, Value>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            software_key: "fe".to_string(),
            feature_key: ANSWER_PARSER_FEATURE.to_string(),
            app_version_code: 0,
            public_key_b64: DEFAULT_PUBLIC_KEY_B64.to_string(),
            access_token: String::new(),
            cached_grants: HashMap::new(),
        }
    }
}

fn state() -> &'static Mutex<AuthState> {
    static STATE: OnceLock<Mutex<AuthState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AuthState::default()))
}

// ============== 配置 / 令牌 (FFI) ==============

/// 配置内部授权: base_url / software_key / feature_key / 服务器公钥 / version_code。
/// public_key_b64 可传 NULL 或空串以保留内置默认公钥。
/// 返回 0 成功, 非 0 参数错误。
#[no_mangle]
pub extern "C" fn set_auth_config(
    base_url: *const c_char,
    software_key: *const c_char,
    feature_key: *const c_char,
    public_key_b64: *const c_char,
    app_version_code: i64,
) -> i32 {
    let base_url = crate::read_str(base_url);
    let software_key = crate::read_str(software_key).trim().to_lowercase();
    let feature_key = crate::read_str(feature_key).trim().to_lowercase();
    let public_key_b64 = crate::read_str(public_key_b64);

    if base_url.trim().is_empty() {
        return 1;
    }
    if software_key.len() < 2 || feature_key.len() < 2 {
        return 2;
    }
    if !public_key_b64.trim().is_empty() {
        match URL_SAFE_NO_PAD.decode(public_key_b64.trim().as_bytes()) {
            Ok(bytes) if bytes.len() == 32 => {}
            _ => return 3,
        }
    }
    let mut st = state().lock().unwrap();
    let base_url = base_url.trim_end_matches('/').to_string();
    let public_key_b64 = if public_key_b64.trim().is_empty() {
        st.public_key_b64.clone()
    } else {
        public_key_b64.trim().to_string()
    };
    let changed = st.base_url != base_url
        || st.software_key != software_key
        || st.feature_key != feature_key
        || st.app_version_code != app_version_code
        || st.public_key_b64 != public_key_b64;
    st.base_url = base_url;
    st.software_key = software_key;
    st.feature_key = feature_key;
    st.app_version_code = app_version_code;
    st.public_key_b64 = public_key_b64;
    if changed {
        st.cached_grants.clear();
    }
    0
}

/// 设置当前访问令牌 (仅存内存, 不清除零化交给调用方选择)。
#[no_mangle]
pub extern "C" fn set_access_token(token: *const c_char) {
    let token = crate::read_str(token);
    let mut st = state().lock().unwrap();
    if st.access_token != token {
        st.access_token = token;
        st.cached_grants.clear();
    }
}

/// 清除当前访问令牌。
#[no_mangle]
pub extern "C" fn clear_access_token() {
    let mut st = state().lock().unwrap();
    st.access_token.clear();
    st.cached_grants.clear();
}

/// 启用/关闭内部授权。关闭时 authorized 接口一律拒绝。
#[no_mangle]
pub extern "C" fn set_auth_enabled(enabled: bool) {
    let mut st = state().lock().unwrap();
    st.enabled = enabled;
    if !enabled {
        st.cached_grants.clear();
    }
}

/// 查询内部授权是否启用。
#[no_mangle]
pub extern "C" fn is_auth_enabled() -> bool {
    state().lock().unwrap().enabled
}

/// 仅执行内部授权 (联调/诊断), 返回授权 JSON 或错误 JSON。
#[no_mangle]
pub extern "C" fn authorize_feature() -> *mut c_char {
    crate::json_to_ptr(authorize_json())
}

/// 授权结果 JSON (成功返回 grant, 失败返回错误对象)。JNI/FFI 共用。
pub fn authorize_json() -> Value {
    match authorize() {
        Ok(grant) => grant,
        Err(e) => error_json(&e),
    }
}

// ============== 核心授权逻辑 ==============

fn error_json(e: &AuthError) -> Value {
    json!({"ok": false, "error": {"code": e.code, "message": e.message}})
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn generate_nonce() -> String {
    let mut buf = [0u8; 24];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// 与后端一致的类型化授权请求。
///
/// 优先复用未过期的 cached grant (本地重新验签), 避免批量解析时每文件一次
/// 网络授权触发后端限流。缓存失效/缺失时才发起网络请求; 授权被拒时清除缓存。
pub fn authorize() -> Result<Value, AuthError> {
    let feature_key = state().lock().unwrap().feature_key.clone();
    authorize_for_feature(&feature_key)
}

/// 使用同一后端、访问令牌和签名公钥授权指定功能。
pub fn authorize_for_feature(feature_key: &str) -> Result<Value, AuthError> {
    let feature_key = feature_key.trim().to_lowercase();
    if feature_key.len() < 2 {
        return Err(AuthError::new("auth_not_configured", "授权功能标识无效"));
    }
    let (base_url, token, software_key, app_version_code, public_key_b64) = {
        let st = state().lock().unwrap();
        if !st.enabled {
            return Err(AuthError::new("auth_disabled", "内部授权未启用"));
        }
        if st.base_url.is_empty() {
            return Err(AuthError::new("auth_not_configured", "未配置后端地址"));
        }
        if st.access_token.is_empty() {
            return Err(AuthError::new("auth_not_configured", "未设置访问令牌"));
        }
        (
            st.base_url.clone(),
            st.access_token.clone(),
            st.software_key.clone(),
            st.app_version_code,
            st.public_key_b64.clone(),
        )
    };

    if let Some(cached) = state()
        .lock()
        .unwrap()
        .cached_grants
        .get(&feature_key)
        .cloned()
    {
        let nonce = cached
            .get("operation_nonce")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if verify_grant(&cached, nonce, &public_key_b64, &software_key, &feature_key).is_ok() {
            return Ok(cached);
        }
        state().lock().unwrap().cached_grants.remove(&feature_key);
    }

    let nonce = generate_nonce();
    let body = json!({
        "software_key": software_key,
        "feature_key": feature_key,
        "operation_nonce": nonce,
        "app_version_code": app_version_code,
    });
    let url = format!("{base_url}{FEATURE_AUTHORIZE_PATH}");

    let response = ureq::post(&url)
        .timeout(std::time::Duration::from_secs(10))
        .set("Accept", "application/json")
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("SoftwareAccess {token}"))
        .send_string(&body.to_string());

    let (status, text) = match response {
        Ok(resp) => (resp.status(), resp.into_string().unwrap_or_default()),
        Err(ureq::Error::Status(code, resp)) => (code, resp.into_string().unwrap_or_default()),
        Err(e) => {
            return Err(AuthError::new(
                "network_error",
                format!("请求后端失败: {e}"),
            ))
        }
    };

    if status != 200 {
        return Err(http_error(status, &text));
    }

    let value: Value = serde_json::from_str(&text)
        .map_err(|_| AuthError::new("invalid_response", "后端响应无法解析"))?;
    let allowed = value
        .get("allowed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !allowed {
        state().lock().unwrap().cached_grants.remove(&feature_key);
        let reason = value
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("denied")
            .to_string();
        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("功能未授权")
            .to_string();
        return Err(AuthError::new(&reason, message));
    }

    verify_grant(&value, &nonce, &public_key_b64, &software_key, &feature_key)?;
    state()
        .lock()
        .unwrap()
        .cached_grants
        .insert(feature_key, value.clone());
    Ok(value)
}

fn http_error(status: u16, text: &str) -> AuthError {
    let code = match status {
        401 => "invalid_token",
        403 => "forbidden",
        404 => "not_found",
        429 => "rate_limited",
        500..=599 => "server_error",
        _ => return AuthError::new("http_error", format!("后端返回 {status}")),
    };
    let message = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| {
            let detail = v.get("detail");
            match detail {
                Some(Value::Object(obj)) => obj
                    .get("message")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string()),
                Some(Value::String(s)) => Some(s.clone()),
                _ => None,
            }
        })
        .unwrap_or_else(|| format!("后端返回 {status}"));
    AuthError::new(code, message)
}

/// 校验服务器签名的功能授权。
fn verify_grant(
    response: &Value,
    nonce: &str,
    public_key_b64: &str,
    software_key: &str,
    feature_key: &str,
) -> Result<(), AuthError> {
    let obj = response
        .as_object()
        .ok_or_else(|| AuthError::new("signature_invalid", "授权响应不是 JSON 对象"))?;

    let mut payload = Map::new();
    for (key, value) in obj {
        match key.as_str() {
            "grant" | "grant_key_id" | "reason" | "message" | "server_time" => {}
            _ => {
                payload.insert(key.clone(), value.clone());
            }
        }
    }
    if payload.get("allowed").and_then(|v| v.as_bool()) != Some(true) {
        return Err(AuthError::new("not_allowed", "授权结果为拒绝"));
    }

    let signature_b64 = obj
        .get("grant")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuthError::new("signature_invalid", "授权响应缺少 grant 签名"))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| AuthError::new("signature_invalid", "grant 签名格式非法"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| AuthError::new("signature_invalid", "grant 签名长度非法"))?;

    let public_key_bytes = URL_SAFE_NO_PAD
        .decode(public_key_b64.as_bytes())
        .map_err(|_| AuthError::new("signature_invalid", "服务器公钥格式非法"))?;
    let public_key = PublicKey::from_slice(&public_key_bytes)
        .map_err(|_| AuthError::new("signature_invalid", "服务器公钥长度非法"))?;

    let canonical = canonical_json(&Value::Object(payload));
    public_key
        .verify(canonical.as_bytes(), &signature)
        .map_err(|_| AuthError::new("signature_invalid", "授权签名校验失败"))?;

    if response.get("software_key").and_then(|v| v.as_str()) != Some(software_key) {
        return Err(AuthError::new("grant_mismatch", "授权软件不匹配"));
    }
    if response.get("feature_key").and_then(|v| v.as_str()) != Some(feature_key) {
        return Err(AuthError::new("grant_mismatch", "授权功能不匹配"));
    }
    if response.get("operation_nonce").and_then(|v| v.as_str()) != Some(nonce) {
        return Err(AuthError::new("nonce_mismatch", "授权随机数不匹配"));
    }

    let now = now_unix();
    let issued_at = response
        .get("issued_at")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AuthError::new("grant_invalid", "授权缺少签发时间"))?;
    let expires_at = response
        .get("expires_at")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AuthError::new("grant_invalid", "授权缺少过期时间"))?;
    if issued_at > now + MAX_ISSUE_SKEW_SECONDS {
        return Err(AuthError::new("grant_not_yet_valid", "授权尚未生效"));
    }
    if expires_at <= now {
        return Err(AuthError::new("grant_expired", "授权已过期"));
    }
    if expires_at - issued_at > MAX_GRANT_VALIDITY_SECONDS {
        return Err(AuthError::new("grant_invalid", "授权有效期异常"));
    }
    Ok(())
}

// ============== 规范化 JSON (与后端 sign_grant 对齐) ==============

/// 按 Python json.dumps(sort_keys=True, separators=(",",":"), ensure_ascii=False)
/// 的规则序列化, 用于还原被签名内容。
fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_canonical_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical_string(key, out);
                out.push(':');
                write_canonical(&map[*key], out);
            }
            out.push('}');
        }
    }
}

fn write_canonical_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// ============== 单元测试 ==============

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_compact::{KeyPair, Seed};

    fn test_grant(signing: &KeyPair, nonce: &str, feature_key: &str) -> Value {
        let now = now_unix();
        let payload = json!({
            "allowed": true,
            "decision_id": "dec_test",
            "software_key": "fe",
            "feature_key": feature_key,
            "operation_nonce": nonce,
            "policy_revision": 1,
            "user_id": 123,
            "session_id": 456,
            "device_id": 789,
            "client_key": "fe-android",
            "issued_at": now,
            "expires_at": now + 120,
        });
        let canonical = canonical_json(&payload);
        let signature = signing.sk.sign(canonical.as_bytes(), None);
        let mut result = payload.as_object().unwrap().clone();
        result.insert(
            "grant".to_string(),
            Value::String(URL_SAFE_NO_PAD.encode(signature)),
        );
        result.insert(
            "grant_key_id".to_string(),
            Value::String("test-key-id".to_string()),
        );
        Value::Object(result)
    }

    fn verify_with(signing: &KeyPair, grant: &Value, nonce: &str) -> Result<(), AuthError> {
        verify_grant(
            grant,
            nonce,
            &URL_SAFE_NO_PAD.encode(signing.pk.as_ref()),
            "fe",
            "answer_parser",
        )
    }

    #[test]
    fn test_canonical_json_matches_python_style() {
        let value = json!({"b": 2, "a": {"d": [1, true, null], "c": "中文😀"}, "z": 1.5});
        let canonical = canonical_json(&value);
        assert_eq!(
            canonical,
            "{\"a\":{\"c\":\"中文😀\",\"d\":[1,true,null]},\"b\":2,\"z\":1.5}"
        );
    }

    #[test]
    fn test_verify_valid_grant() {
        let seed = Seed::new([7u8; 32]);
        let signing = KeyPair::from_seed(seed);
        let nonce = "0123456789abcdef";
        let grant = test_grant(&signing, nonce, ANSWER_PARSER_FEATURE);
        assert!(verify_with(&signing, &grant, nonce).is_ok());
    }

    #[test]
    fn test_cloud_grant_is_feature_bound() {
        let signing = KeyPair::from_seed(Seed::new([12u8; 32]));
        let nonce = "0123456789abcdef";
        let grant = test_grant(&signing, nonce, CLOUD_MODE_FEATURE);
        let public_key = URL_SAFE_NO_PAD.encode(signing.pk.as_ref());
        assert!(verify_grant(&grant, nonce, &public_key, "fe", CLOUD_MODE_FEATURE,).is_ok());
        assert!(verify_grant(&grant, nonce, &public_key, "fe", ANSWER_PARSER_FEATURE,).is_err());
    }

    #[test]
    fn test_verify_tampered_grant_rejected() {
        let seed = Seed::new([8u8; 32]);
        let signing = KeyPair::from_seed(seed);
        let nonce = "0123456789abcdef";
        let mut grant = test_grant(&signing, nonce, ANSWER_PARSER_FEATURE);
        grant.as_object_mut().unwrap().insert(
            "operation_nonce".to_string(),
            Value::String("tampered-xxxx".to_string()),
        );
        assert!(verify_with(&signing, &grant, nonce).is_err());
    }

    #[test]
    fn test_verify_wrong_key_rejected() {
        let seed = Seed::new([9u8; 32]);
        let signing = KeyPair::from_seed(seed);
        let other = KeyPair::from_seed(Seed::new([10u8; 32]));
        let nonce = "0123456789abcdef";
        let grant = test_grant(&signing, nonce, ANSWER_PARSER_FEATURE);
        assert!(verify_with(&other, &grant, nonce).is_err());
    }

    #[test]
    fn test_verify_expired_grant_rejected() {
        let seed = Seed::new([11u8; 32]);
        let signing = KeyPair::from_seed(seed);
        let nonce = "0123456789abcdef";
        let mut grant = test_grant(&signing, nonce, ANSWER_PARSER_FEATURE);
        let obj = grant.as_object_mut().unwrap();
        obj.insert("issued_at".to_string(), json!(now_unix() - 200));
        obj.insert("expires_at".to_string(), json!(now_unix() - 100));
        assert!(verify_with(&signing, &grant, nonce).is_err());
    }
}
