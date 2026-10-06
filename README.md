# Ets Resolver (Rust)

本项目的**唯一事实源**是 Rust 实现：高性能动态链接库 `libresolver.so`（答案解析）与 `libcloud.so`（ETS100 云端协议），可被 Python、C/C++、C#、Java 等以 FFI/JNI 方式调用。

> 下游 `Fe-pro`（Android）与 `lastudio-platform`（后端）都从本仓库的 GitHub Release 拉取 so，
> 因此**修改只在本仓库做，其他端自动同步**。
>
> 2026-07 时代的 Python demo 实现（`demo.py` / `resolver_py/` 等）已归档到 [`legacy/`](./legacy/README.md)，
> 仅作历史参考与行为比对，下游与 CI 都不使用。

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

### 1. 旧 Python demo（已归档，不再维护）

历史实现位于 [`legacy/demo.py`](./legacy/demo.py)，仅在需要行为比对时使用：
```bash
python3 legacy/demo.py
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

## 单词 PK 与词汇作业解析

针对 ETS 客户端中的单词 PK 对战与词汇训练作业，支持原生双端（Python / Rust FFI）高效解析。

### 1. 支持题型

逆向自 `SyncWordQuestionBean`：

| question_type | 标准题型代码 `type` | 说明 | 题干 (question) | 答案 (answers) | 选项 (options) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **6** | `word.choice` | 听选 / 词义选择题 | 英文 (中文释义) | 正确选项编号/值 | `["A. ...", "B. ..."]` |
| **16** | `word.write` | 看释义写单词 (拼写) | 中文释义 | 英文单词 | 空 |
| **17** | `word.read` | 朗读单词 | 英文单词 | 英文单词 | 空 |
| **18** | `word.dictation` | 听写单词 | 音频 / 提示 | 英文单词 | 空 |
| **19** | `word.translate_en_cn` | 英译中 (En→Cn) | 英文单词 | 中文释义 | 空 |
| **20** | `word.translate_cn_en` | 中译英 (Cn→En) | 中文释义 | 英文单词 | 空 |

### 2. 多形态输入自动兼容
- **`word/struct` 格式**：`{"data": [{"id": 1, "content": "apple", "translate": "苹果", "question": [...]}]}`
- **`pk_info` 格式**：`{"word_questions": [...], "words": [...]}`
- **题目数组**：`[{"question_type": 19, "word": "apple", "translate": "苹果", "answer": ""}, ...]`
- **统一任务格式**：`{"structure_type": "word_pk", "info": ...}`

### 3. API 与工具函数

#### Rust FFI 接口
- `char* parse_word_pk(const char* pk_data)`：解析词汇作业 JSON 并返回统一题目数组 JSON。
- `char* parse_task(const char* content, ...)`：直接传入词汇作业 JSON 数组或对象自动分发。

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

## ETS100 云端模式独立 SO 接口

云端链路已从解析器拆分为独立 `libcloud.so`。`libresolver.so` 只包含答案解析和 `answer_parser` 授权；`libcloud.so` 只包含 ETS100 云端协议和 `cloud_mode` 授权：

```text
生成设备码
  → 手机号密码登录 / 设备绑定 / 短信绑定
  → 获取 ecard 和 parent_account_id
  → 获取当前、历史、过期作业
  → HTTPS 下载加密 ZIP
  → 计算 ZIP 密码并在内存中读取 content/info/content2
  → 返回云端资源，由 libresolver.so 单独解析答案
```

调用方仍负责凭据和 token 的安全持久化，SO 不保存手机号、密码或 ETS100 token。API 和任意第三方下载地址保持完整 TLS 校验。ETS100 默认 CDN 当前返回 `default.chinanetcenter.com` 证书，因此仅对精确主机 `cdn.subject.ets100.com` 使用这个固定证书名校验，证书链、有效期和握手签名仍正常验证；不会全局 trust-all。

每个 `cloud_*` 操作都会在 `libcloud.so` 内请求并验签 LA Studio `cloud_mode` 功能授权；答案解析由 `libresolver.so` 独立校验 `answer_parser`。两个 so 不互相链接，grant 和解析核心不会跨库复用。

所有 `cloud_*` 字符串接口统一返回 JSON，调用方使用 `cloud_free_string` 释放：

```json
{"ok":true,"result":{}}
```

```json
{"ok":false,"error":{"code":"device_bind_required","api_code":30014,"message":"..."}}
```

核心接口：

| C FFI | 说明 |
| --- | --- |
| `cloud_generate_device_code()` | 生成与 Android 旧实现兼容的 33 字符设备码。调用方应持久化并复用。 |
| `cloud_authorize_feature()` | 单独检查并验签 `cloud_mode` 授权，用于诊断。 |
| `cloud_login(phone, password, device_code)` | `/user/login`，返回 token。 |
| `cloud_login_changyan(login, user_id, captcha_result, device_code, preferred_id)` | `cloud_mode` 授权后调用 `/user/login-zt` 并查询 ecard；一次性断言不持久化。 |
| `cloud_send_device_binding_code(phone)` | `/code/send-user-exist`。 |
| `cloud_bind_device(phone, password, device_code, code)` | `/user/rebind-code`；code 传空串时使用 `0`。 |
| `cloud_get_ecard_selection(token, preferred_id)` | `/m/ecard/list`，返回选中账号、全部账号及 `parent_account_id`。 |
| `cloud_get_homework_list(token, parent_id, status)` | `/g/homework/list`；status 为 `1`、`2`、`3`。资源包含绝对 `download_url`。 |
| `cloud_download_file(url, destination)` | 使用 HTTPS 和 `.part` 文件下载，校验 ZIP magic 后替换目标文件。 |
| `cloud_zip_password(path)` | 从 ETS100 ZIP 尾部计算 64 字符密码。 |

Android 使用两个 JNI 类：`ResolverNative` 加载 `libresolver.so`，`CloudNative` 加载 `libcloud.so`。所有云端授权、网络和文件接口都是阻塞调用，必须从 `Dispatchers.IO` 调用。

详细输入输出和接入顺序见 [`CLOUD_API.md`](./CLOUD_API.md)。讯飞/畅言页面交互仍由 Android 隔离 WebView 完成；SO 只接收一次性断言并负责 token 交换和账号查询。

---

## 诊断、形状指纹与作业分类（parity 工具）

> 方法论与采集管道见 [`docs/PARITY.md`](./docs/PARITY.md)；
> **官方题型全量盘点见 [`docs/OFFICIAL_TYPE_INVENTORY.md`](./docs/OFFICIAL_TYPE_INVENTORY.md)**（2157 个大题名 / 9 个解析家族 / 221 套模板实测）；
> **跨平台固定接口契约见 [`docs/INTERFACE.md`](./docs/INTERFACE.md)**；
> 需要官方提供哪些素材见 [`docs/OFFICIAL_INPUTS.md`](./docs/OFFICIAL_INPUTS.md)。

### 推荐入口：`resolve_request`（跨平台唯一入口）

新接入方只需实现一次"JSON 进 / JSON 出"，后续新能力通过新增 `op` 交付，不用改 ABI：

```c
/* 请求: {"version":1,"op":"parse_task","payload":{"content":{...},"info":{...}}} */
char* resp = resolve_request("{\"op\":\"capabilities\"}");
resolver_free_string(resp);
```

```json
{ "ok": true, "version": 1, "op": "parse_task", "result": [ /* question[] */ ],
  "meta": { "layer": "explicit", "family": "collector.choose", "shape": "df3956bcdc86ea06" } }
```

失败时统一为 `{"ok":false,"op":"...","error":{"code","message","hint"}}`，
错误码：`invalid_input` / `unsupported_version` / `unknown_op` / `not_implemented` / `internal_error`。
Android 端对应 `ResolverNative.resolveRequest(json)` 与 `ResolverNative.capabilities()`。

| op | 状态 | 说明 |
| --- | --- | --- |
| `capabilities` | ✅ | 能力清单（ops / 题型 / 分类 / 错误码 / 覆盖率） |
| `parse_task` / `parse_batch` / `parse_word_pk` / `extract_meta` | ✅ | 解析（`meta` 带 `layer` / `family` / `shape` / `spec_known`） |
| `diagnose` / `shape_fingerprint` | ✅ | 诊断与结构指纹（发现官方新题型） |
| `classify_homework` / `classify_homework_list` | ✅ | 作业分类（含习题集识别） |
| `spec` | ✅ | **官方权威规格**：22 个题型全集、词汇双通道、展示要素、边界规则、覆盖率自证 |
| `res_template` / `apply_res_template` | ✅ | `res.json` 权威题号与逐题满分（E5/E6） |
| `parse_homework_set` | ✅ | **习题集多包合并**：官方 `res.json` 定大题顺序与逐题满分，同大题分包自动合并、全局题号连续 |

### 官方权威规格（客户端反编译固化）

`{"op":"spec"}` 返回 `src/spec.rs` 里固化的官方定义域，客户端**不要再硬编码枚举**：

| 内容 | 数量/要点 |
| --- | --- |
| `structure_types` | **22 项** = 官方引擎标识 **14 项**（`collector.read/repeat/sentence/word`、`repeat`、`repeat_dialogue`、`essay`、`templet_common`、`listen_choice_standard`、`listen_fill_standard`、`read_passage_standard`、`read_sentence_standard`、`read_word_standard`、`read_and_choose_standard`）+ content.json 常见 `collector.*` **8 项**（`choose/role/picture/fill/3q5a/dialogue/repeat_essay`、`word_pk`），其中 **21 项已显式适配** |
| `word_symbols` | **20 项**（超能单词 `QuestionSymbol` 0~13 + 同步PK `SyncWordQuestionBean` 6/16~20） |
| 双通道警戒 | `question_type=6` 在超能单词通道是「单词跟读」、在同步PK通道是「听音选词」→ 输出 `channel` / `symbol_name` / `ambiguous_across_channels` |
| `boundary_rules` | 多答案**原样不排序**；填空小题号取 `std[].xth`、卷面大题号取 `th`（`exam_info[].value` 是指示文本）；朗读类 `answers` 直接给原文 |
| `res_json_authority` | 大题顺序取 `exam_type_list` 下标；逐题满分取 `question_list[].question_score` |

### 单独可调的 C FFI（老调用方兼容保留）

这三个能力是"与官方客户端完全一致"的基础设施，Android 端同样有 JNI 入口
（`ResolverNative.diagnose` / `classifyHomework` / `classifyHomeworkList`）。

| C FFI | 说明 |
| --- | --- |
| `char* diagnose(content, info, content2)` | 命中层（`explicit`/`generic`/`none`）、命中的解析家族、形状指纹、告警、输入在场情况。**输出不含题目文本**，可安全进日志 |
| `char* shape_fingerprint(content, info, content2)` | 结构指纹：不含取值、不含 `structure_type`。官方改名不改结构 → 指纹不变；结构变了 → 指纹变化 |
| `char* classify_homework(homework_json)` | 作业分类 + 卷面满分口径。`is_exercise_set=true` 即"老师布置的一整套多卷（习题集）"，需要多包合并成一张卷 |
| `char* classify_homework_list(list_json)` | 作业列表批量分类，返回 `category_counts` 与 `exercise_set_ids` 清单 |

`diagnose` 示例输出：

```json
{
  "structure_type": "collector.choose",
  "layer": "explicit",
  "family": "collector.choose",
  "question_count": 1,
  "answer_count": 1,
  "shape": { "hash": "df3956bcdc86ea06", "path_count": 11, "paths": ["info.stid", "info.xtlist[]", "..."] },
  "warnings": [],
  "preview": [{ "no": 1, "type": "collector.choose", "question_len": 38, "answers": 1, "options": 3 }]
}
```

`classify_homework` 示例输出（习题集）：

```json
{
  "category": "paper_set",
  "is_exercise_set": true,
  "package_count": 6,
  "expected_total_score": 11.5,
  "score_source": "group_name_weights",
  "set_id": "set-9981",
  "reasons": ["packages:6", "set_id:set-9981"],
  "warnings": []
}
```

判定原则：模拟考只信官方显式字段（`kind=mock` / `mock=1` / `exam=1` / `set_type=1`），
名字关键词（套题/模考/…）降级为 `is_mock_suspected` 告警——历史启发式曾把普通作业误判成模拟考。

---

# 测试

Rust 核心测试（CI 与本地都应保持全绿）：
```bash
cargo test --workspace          # 148 个用例：解析分发、PK 顺序/过滤与双通道、清洗、诊断指纹、作业分类、res.json 模板、契约层、授权签名校验
```

一致性回归（与官方客户端对齐的门禁，方法论见 [`docs/PARITY.md`](./docs/PARITY.md)）：
```bash
cargo test --test corpus_parity -- --nocapture                    # 打印覆盖率/路由家族/形状指纹报告
PARITY_CORPUS=/path/to/corpus cargo test --test corpus_parity     # 指向仓库外的全量语料库
PARITY_REQUIRE_CORPUS=1 PARITY_STRICT_OFFICIAL=1 cargo test --test corpus_parity   # CI 门禁
PARITY_UPDATE=1 cargo test --test corpus_parity                   # 录制 golden（自录，不算一致性证据）
```

线下联调（需要本地后端已运行并 seed `answer_parser` 功能）：
```bash
python3 test_auth_e2e.py
```

历史实现的自测（已归档，需要外部数据集，缺失时自动跳过）：
```bash
python3 legacy/test_resolver_py.py
python3 legacy/test_demo.py ..\downloaded
python3 legacy/test_lib.py ..\downloaded
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

- 防护清单（含自校验/灰度开关）：[docs/HARDENING.md](docs/HARDENING.md)
