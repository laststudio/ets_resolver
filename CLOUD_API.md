# ETS100 Cloud SO API

本文档描述 `libresolver.so` 的 ETS100 云端接口。手机号密码协议完全在 SO 内；讯飞/畅言页面交互由 Android WebView 完成，登录结果交换在 SO 内。

## 设计边界

- SO 负责 ETS100 请求签名、网络请求、响应解析、资源下载、ZIP 密码、ZIP 内 JSON 读取和 resolver 调用。
- 调用方负责手机号/密码输入、短信验证码 UI、token 与设备码持久化、LA Studio 登录和页面状态。
- SO 不持久化任何凭据。
- 所有 `cloud_*` 操作都会在 SO 内固定校验 LA Studio `cloud_mode` 功能授权。
- `cloud_download_and_parse_homework` 还会调用 `parse_batch_authorized`，独立校验 `answer_parser`；两种 grant 不能互相替代。
- 调用云端接口前必须完成 `set_auth_config`、`set_access_token` 和 `set_auth_enabled(true)`。
- 网络及文件函数是阻塞接口。Android 必须在 `Dispatchers.IO` 中调用。

## 统一返回值

成功：

```json
{
  "ok": true,
  "result": {}
}
```

失败：

```json
{
  "ok": false,
  "error": {
    "code": "device_bind_required",
    "api_code": 30014,
    "message": "设备需要绑定"
  }
}
```

常用错误码：

| code | 含义 |
| --- | --- |
| `invalid_input` | 参数为空或格式错误 |
| `network_error` | DNS、连接或 TLS 失败 |
| `http_error` | ETS100/CDN 返回非成功 HTTP 状态 |
| `api_error` | 未单独映射的 ETS100 业务错误 |
| `sms_verification_required` | ETS100 code `30008` |
| `device_bind_required` | ETS100 code `30014` |
| `invalid_verification_code` | ETS100 code `60001` |
| `invalid_response` | 响应不是预期 JSON 结构 |
| `invalid_zip` | 下载结果或 ETS100 ZIP 尾部无效 |
| `zip_password_error` | ZIP 解密失败 |
| `missing_content` | ZIP 内没有 `content.json` |
| `no_parseable_resource` | 所有资源均下载或读取失败 |
| `resolver_error` | LA Studio 授权或答案解析失败 |

## C FFI 顺序

所有返回的 `char*` 均由 `resolver_free_string` 释放。

### 0. 配置 LA Studio 远程授权

```c
set_auth_config(auth_base_url, "fe", "answer_parser", public_key, version_code);
set_access_token(lastudio_access_token);
set_auth_enabled(true);
```

`set_auth_config` 中的 feature key 为兼容旧解析接口保留。云端函数不会信任调用方传入的 feature key，而是在 SO 内固定请求并验签 `cloud_mode`。可以用以下接口单独诊断授权：

```c
char* grant = cloud_authorize_feature();
```

成功时返回 `{"ok":true,"result":{...cloud_mode grant...}}`。

后端需要为 `fe` 软件配置 `cloud_mode` 功能。仓库中的 Alembic 迁移 `c3d4e5f6a7b8_seed_fe_cloud_mode_feature.py` 会创建默认启用、非会员限制的初始规则，之后可在后台独立启停或调整会员、版本和限流策略。

### 1. 生成并保存设备码

```c
char* json = cloud_generate_device_code();
```

结果：

```json
{"ok":true,"result":{"device_code":"0123456789abcdef|0123456789abcdef"}}
```

设备码只应生成一次，之后由调用方持久化并复用。频繁更换设备码可能触发设备绑定。

### 2. 登录

```c
char* json = cloud_login(phone, password, device_code);
```

成功结果：

```json
{"ok":true,"result":{"token":"..."}}
```

如果错误是 `device_bind_required`，可以先用空验证码调用 `cloud_bind_device`。如果错误是 `sms_verification_required`，先发送验证码，再携带用户输入的验证码绑定。

### 3. 短信和设备绑定

```c
cloud_send_device_binding_code(phone);
cloud_bind_device(phone, password, device_code, verification_code);
```

`verification_code` 传 `""` 时内部使用 `"0"`，与 Fe-pro 旧流程一致。

### 3.1 讯飞/畅言网页登录交换

Android WebView 在可信 `pass.changyan.com` 页面捕获一次性的 `uid` 与 `captchaResult` 后调用：

```c
char* json = cloud_login_changyan(
    login,
    user_id,
    captcha_result,
    device_code,
    preferred_account_id
);
```

SO 会先校验 `cloud_mode`，然后调用 `/user/login-zt` 和 `/m/ecard/list`。返回结果包含 token、选中账号及全部有效账号。SO 不保存 `captcha_result`、Cookie 或 token；调用方应在接口返回后立即丢弃一次性断言，并用系统安全存储保存 token。

### 4. 获取 ecard

```c
char* json = cloud_get_ecard_selection(token, preferred_account_id);
```

结果包含：

```json
{
  "ok": true,
  "result": {
    "parent_account_id": "...",
    "selected_account": {},
    "valid_accounts": [],
    "accounts": []
  }
}
```

选择规则与 Fe-pro 一致：优先匹配有效的 `preferred_account_id`，否则选择第一个有效班级账号，最后回退第一个账号。

### 5. 获取作业列表

```c
char* json = cloud_get_homework_list(token, parent_account_id, "1");
```

状态：

| status | 含义 |
| --- | --- |
| `1` | 当前作业 |
| `2` | 历史作业 |
| `3` | 过期作业 |

每个 content 同时返回 ETS100 原始 `url` 和已解析的 HTTPS `download_url`。

### 6. 一步下载并解析

完成第 0 步的 LA Studio 授权配置后，把某个作业的 `contents` 数组传给：

```c
char* json = cloud_download_and_parse_homework(base_url, contents_json, cache_directory);
```

成功结果的 `result` 包含：

- `parse`：与 `parse_batch_authorized` 的 `result` 相同。
- `grant`：本次解析使用的 LA Studio 授权。
- `resources`：成功下载并读取的资源包。
- `failures`：单个资源失败列表。部分资源失败不会阻止其他资源解析。

资源不会解压到目录。SO 使用计算出的密码直接从 ZIP 中读取 `content.json`、`info.json` 和 `content2.json`，减少临时文件和路径穿越风险。

## Android JNI

Kotlin 声明示例：

```kotlin
internal object ResolverNative {
    init { System.loadLibrary("resolver") }

    external fun cloudAuthorizeFeature(): String
    external fun cloudGenerateDeviceCode(): String
    external fun cloudLogin(phone: String, password: String, deviceCode: String): String
    external fun cloudLoginChangyan(
        login: String,
        userId: String,
        captchaResult: String,
        deviceCode: String,
        preferredAccountId: String,
    ): String
    external fun cloudSendDeviceBindingCode(phone: String): String
    external fun cloudBindDevice(
        phone: String,
        password: String,
        deviceCode: String,
        verificationCode: String,
    ): String
    external fun cloudGetEcardSelection(token: String, preferredAccountId: String): String
    external fun cloudGetHomeworkList(token: String, parentAccountId: String, status: String): String
    external fun cloudDownloadFile(url: String, destination: String): String
    external fun cloudZipPassword(path: String): String
    external fun cloudDownloadAndParseHomework(
        baseUrl: String,
        contentsJson: String,
        cacheDirectory: String,
    ): String
}
```

JNI 符号当前绑定 `com.shuaiqiu.fuckets100.ResolverNative`。如果 Android 包名或类名变化，需要同步修改 `src/jni_bridge.rs` 或改为 `RegisterNatives`。

## 安全说明

- 账号密码仅作为函数参数使用，不存入 Rust 全局状态。
- `cloud_mode` 和 `answer_parser` 都由 SO 内部请求后端并校验 Ed25519 签名 grant；外层 Kotlin 判断不能代替该校验。
- 调用方不应把密码、token 或完整 API 响应写入日志。
- API 和第三方资源地址使用完整 TLS 验证。ETS100 默认 CDN 目前返回 `default.chinanetcenter.com` 证书，因此仅对精确主机 `cdn.subject.ets100.com` 使用该固定证书名校验；证书链、有效期和握手签名仍会验证，不会全局 trust-all。
- 缓存文件先写入 `.part`，确认 ZIP magic 后才替换正式文件。
- 单个下载最大 512 MiB；ZIP 内只读取三个固定 JSON 文件，并限制单个 JSON 最大 32 MiB。
