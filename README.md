# Ets Resolver (Python & Rust)

本项目提供了作业答案解析的两种实现：原生 Python 脚本 (`demo.py`) 与高性能 Rust 动态链接库 (`libresolver.so` / `resolver.dll` 等)。通过 Rust 编译的动态库可以被 Python、C/C++、C#、Java 等其他程序以 FFI 的方式调用。

## 依赖与环境

### Python 版本
- Python 3.10+ (支持 `match-case` 语法)
- 无需额外第三方依赖（仅使用标准库 `os, json, html, logging, time`）

### Rust 版本
- Rust 1.60+
- 依赖：`serde`, `serde_json`

## 在本机编译 Rust 动态库

在项目根目录下执行：
```bash
cargo build --release
```
编译产物位于 `target/release/`：
- Windows: `resolver.dll`
- Android: `libresolver.so`
- Wasm: `resolver.wasm`

## 交叉编译 Rust 动态库

在项目根目录下执行：
```bash
rustup target add aarch64-linux-android wasm32-unknown-unknown x86_64-pc-windows-msvc
cargo install cargo-ndk
cargo build --target x86_64-pc-windows-msvc --release
cargo build --target wasm32-unknown-unknown --release
cargo ndk --target arm64-v8a build --release
```
编译产物位于 `target/release/`：
- Windows: `resolver.dll`
- Android: `libresolver.so`
- Wasm: `resolver.wasm`

---

## 使用方法

### 1. Python 程序 (`demo.py`) 使用

直接运行脚本，根据提示输入：
```bash
python demo.py
```

### 2. Rust 动态链接库使用 (以 Python 为例)

通过 Python 的 `ctypes` 调用 Rust 动态链接库：

```python
import ctypes
import json

lib = ctypes.CDLL("./target/release/libresolver.so")

lib.debug_mode.argtypes = [ctypes.c_bool]
lib.debug_mode.restype = None

lib.nofile_mode.argtypes = [ctypes.c_bool]
lib.nofile_mode.restype = None

lib.resolve.argtypes = [ctypes.c_char_p]
lib.resolve.restype = ctypes.c_void_p

lib.resolver_free_string.argtypes = [ctypes.c_void_p]
lib.resolver_free_string.restype = None

def call(fn, path: str):
    ptr = fn(path.encode("utf-8"))
    if not ptr:
        return None
    try:
        s = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
        return json.loads(s)
    finally:
        lib.resolver_free_string(ptr)

# 1. 路径模式
lib.nofile_mode(False)
print(call(lib.resolve, "./answer"))

# 2. NoFile 模式 (直接传入 JSON 字符串)
lib.nofile_mode(True)
content_json = '{"structure_type": "collector.read", "info": {"stid": "1", "value": "hello", "analyze": "world</br>!"}}'
info_json = "[]"
input_str = json.dumps([content_json, info_json])
print(call(lib.resolve, input_str))
```

注意: cpp引用头文件, 使用`resolver.h`, 动态链接库和静态链接库在`Releases`内

---

## 函数输入输出结构

### 1. `debug_mode(bool mode)` / `nofile_mode(bool mode)`
- **输入**：布尔值，`True` 开启模式，`False` 关闭。
- **输出**：无。需在其他函数调用前先调用此函数配置全局状态。

### 2. `resolve(str target)` -> `dict | null`
- **输入**：
  - `NOFILE_MODE = False`：包含 `content.json` 和 `info.json` 的文件夹路径。
  - `NOFILE_MODE = True`：JSON 字符串，格式为 `"[content.json_str, info.json_str]"`。
- **输出**：JSON 字符串，解析失败时返回 `"null"`。解析成功的结构为：
```json
{
  "stid1_xth1": {
    "answer": ["ans1", "ans2"],
    "important": ["keyword1", "keyword2"]
  },
  "type": "collector.xxx"
}
```

### 3. `resolve_by_homework(str target)` -> `list[dict]`
- **输入**：
  - `NOFILE_MODE = False`：包含多个作业子文件夹的目录路径。
  - `NOFILE_MODE = True`：JSON 字符串，格式为 `"[[content1, info1], [content2, info2], ...]"`。
- **输出**：JSON 字符串，解析失败时返回 `"[]"`。结构为包含多个 `resolve` 返回对象的数组。

### 4. `resolve_by_all_homework(str target)` -> `dict[str, list[dict]]`
- **输入**：
  - `NOFILE_MODE = False`：包含多个大作业文件夹的总目录路径。
  - `NOFILE_MODE = True`：JSON 字符串，格式为 `"[[[c1,i1],[c2,i2]], [[c3,i3], ...], ...]"`。
- **输出**：JSON 字符串，解析失败时返回 `"{}"`。结构为以作业名为键、对应作业解析结果（数组）为值的字典。

---

## 内部后端授权接口 (answer_parser 功能门控)

> 面向 Fe (Android) 客户端接入 lastudio-platform 后端的实时授权方案。
> 解析前由 **库内部** 调用 `/client-auth/features/authorize` 并校验 Ed25519 签名授权，
> 通过后才真正解析；即使外部 Java 层被 patch 绕过，库内部仍强制联网校验。
> 授权相关符号仅在非 wasm 目标导出（wasm 构建不受影响）。

### 使用流程

```text
Fe 外部登录 (/client-auth/login) 拿到 access_token
  → set_auth_config(base_url, "fe", "answer_parser", public_key_b64, version_code)
  → set_access_token(token)
  → set_auth_enabled(true)
  → parse_task_authorized(content, info, content2)   # 内部先授权再解析
后台关闭 answer_parser 功能 → 解析器下一次授权请求立即返回 feature_disabled
```

### 配置接口

| 函数 | 说明 |
| --- | --- |
| `int set_auth_config(base_url, software_key, feature_key, public_key_b64, app_version_code)` | 配置后端地址、软件/功能 key、Ed25519 公钥 (urlsafe base64)。`public_key_b64` 可传 NULL 保留内置默认公钥。返回 0 成功。 |
| `void set_access_token(token)` | 设置当前访问令牌（仅存内存，不持久化）。 |
| `void clear_access_token()` | 清除访问令牌。 |
| `void set_auth_enabled(bool)` | 启用/关闭内部授权。关闭时 `parse_*_authorized` 一律拒绝。 |
| `bool is_auth_enabled()` | 查询内部授权是否启用。 |
| `char* authorize_feature()` | 仅执行一次内部授权（联调用），返回授权 JSON 或错误 JSON。 |

### 授权解析接口

- `parse_task_authorized(content, info, content2)`：内部授权后单任务解析。
- `parse_batch_authorized(tasks_json)`：内部授权后批量解析（`[{content, info?, content2?, order?}, ...]`）。

**成功返回**：

```json
{
  "ok": true,
  "result": [ ...同 parse_task 输出... ],
  "grant": { "allowed": true, "decision_id": "...", "feature_key": "answer_parser",
             "operation_nonce": "...", "expires_at": 1710000000, "grant": "...", "grant_key_id": "..." }
}
```

**被拒 / 错误返回**：

```json
{ "ok": false, "error": { "code": "feature_disabled", "message": "该功能已停用" } }
```

错误码：`auth_disabled` / `auth_not_configured` / `network_error` / `invalid_token` /
`feature_disabled` / `software_kill_switch` / `override_denied` / `membership_required` /
`rate_limited` / `device_blocked` / `signature_invalid` / `grant_expired` / `nonce_mismatch` 等。

### 权限校验内容（库内部）

1. 每次解析生成随机 `operation_nonce` 提交授权。
2. 校验服务器 Ed25519 签名（与后端 `sign_grant` 同款规范化 JSON：`sort_keys` + 紧凑 + 非 ASCII 原样）。
3. 校验 `software_key` / `feature_key` / `operation_nonce` 与请求一致。
4. 校验 `issued_at` 时钟偏差与 `expires_at` 有效期（授权仅 120 秒有效，即时开关生效）。
5. 校验通过后才执行解析；授权结果仅作用于本次操作，不作长期离线凭证。

### 单元 / 集成测试

```bash
cargo test                 # Rust 单元测试 (含签名校验、篡改/过期拒绝)
python3 test_auth_e2e.py   # 端到端: 需本地后端已运行并 seed answer_parser 功能
```

### 编译产物

```bash
cargo build --release                          # x86_64 Linux: target/release/libresolver.so
cargo ndk -t arm64-v8a build --release         # Android arm64: target/aarch64-linux-android/release/libresolver.so
```

> 交叉编译需要安装 `cargo-ndk` 并配置 `ANDROID_NDK_HOME`。

---

# 测试

在项目根目录下执行：
```bash
python3 test_demo.py ..\downloaded
python3 test_demo.py ..\downloaded_res
python3 test_lib.py ..\downloaded
python3 test_lib.py ..\downloaded_res
```

## 系统和环境信息

Python: 3.14.5  
System: Windows 11(25H2,26200.8037)  
CPU: AMD(R) Ryzen(TM) 3700X @ 4.3GHz  
RAM: G.SKILL Trident Z DDR4 8G*2 3600MHz (Samsung B-DIE)  
Disk: WD SN550 1TB  

## 数据

### 纯python实现

最终汇总:  
逐个调用:  14402 成功 / 0 失败  |  23348 答案  |  9.6807s  |  2411.81 answers/s  
批量调用:  14402 成功 / 0 失败  |  23348 答案  |  5.9476s  |  3925.60 answers/s  
加速比:    1.63x

### rust动态链接库

最终汇总:  
逐个调用:  14402 成功 / 0 失败  |  23348 答案  |  4.3968s  |  5310.23 answers/s  
批量调用:  14402 成功 / 0 失败  |  23348 答案  |  4.5328s  |  5150.93 answers/s  
加速比:    0.97x

# 关于与致谢

作者: leitianshuo1337  
感谢以下项目的支持:  
[Rust Programming Language](https://rust-lang.org/)  
[Python](https://python.org/)  
[ETS100](https://ets100.com/)  
本项目的产生离不开各个开源项目和开源开发者的支持, 向其他开源开发者致敬!  