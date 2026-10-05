# 跨平台固定接口契约（contract v1）

> 目标：**任何平台只实现一次"JSON 进 / JSON 出"，之后解析器升级不需要各端跟着改。**
> 新能力一律通过新增 `op` 交付，不改 ABI；只有破坏性字段变更才涨 `version`，
> 且旧版本请求必须继续可用。

---

## 一、信封

### 请求

```json
{
  "version": 1,
  "op": "parse_task",
  "payload": {
    "content": { "structure_type": "collector.choose", "info": { } },
    "info": { },
    "content2": { }
  }
}
```

宽容规则（跨语言最省事）：
- `version` 可省略（等价于 1）；
- `payload` 可省略，字段直接放请求根上；
- **任何 JSON 字段都接受「对象」或「JSON 文本字符串」**两种形态。

### 响应

```json
{ "ok": true, "version": 1, "op": "parse_task", "result": [ ... ], "meta": { "layer": "explicit", "family": "collector.choose" } }
```

```json
{ "ok": false, "version": 1, "op": "parse_task",
  "error": { "code": "invalid_input", "message": "缺少必填字段 `content`", "hint": "..." } }
```

客户端逻辑只需两步：判断 `ok`；非成功时看 `error.code`。

---

## 二、op 清单（`capabilities` 实时返回，这里是快照）

| op | 状态 | 入参 | 出参 | 说明 |
|---|---|---|---|---|
| `capabilities` | stable | — | 能力清单 | 启动时读一次 |
| `parse_task` | stable | `content`, `info?`, `content2?` | `question[]` + `meta` | 解析单份资源包 |
| `parse_batch` | stable | `tasks[]` | `{sections[]}` | 批量解析并合并分节 |
| `parse_word_pk` | stable | `data` | `question[]` | 词汇 / 单词 PK |
| `extract_meta` | stable | `content` | `{stid, structure_type, question_count}` | 轻量元信息 |
| `diagnose` | stable | `content`, `info?`, `content2?` | 诊断报告 | **不含题目文本**，可进日志 |
| `shape_fingerprint` | stable | 同上 | `{hash, paths[]}` | 发现官方新结构 |
| `classify_homework` | stable | `homework` | 分类结论 | 含 `is_exercise_set`（习题集） |
| `classify_homework_list` | stable | `homeworks` | 批量结论 | 返回 `exercise_set_ids` |
| `parse_homework_set` | **planned** | `homeworks[]` | `{paper, coverage}` | 习题集多包合并（M2） |

题库侧（`libcloud.so`）的 op 命名沿用同一信封，前缀 `cloud.`：`cloud.login` / `cloud.homework_list` / `cloud.download` …（M2 统一）。

`question` 的稳定字段：

```json
{
  "no": 1,                                   // 卷内题号，官方口径
  "type": "collector.choose",                // 题型
  "stid": 3,                                 // 大题/资源组 id
  "question": "What did the man forget?",    // 题面（已清洗，客户端不要再清洗）
  "answers": ["C"],                          // 答案（已清洗，顺序即官方顺序）
  "options": ["A. Take the bag.", "..."],    // 选项（已清洗）
  "important": []                            // 关键词/要点
}
```

---

## 三、错误码

| code | 含义 | 客户端建议处置 |
|---|---|---|
| `invalid_input` | 字段缺失或形态不对 | 修调用；`hint` 里有格式示例 |
| `unsupported_version` | 请求 `version` 高于本库 | 升级解析器 or 降低 version |
| `unknown_op` | 未知 op | 先调 `capabilities` 取清单 |
| `not_implemented` | 已规划未实现 | 降级到 `parse_batch` 自行合并 |
| `internal_error` | 内部异常（出现即 bug） | 上报 `op` + 入参摘要 |

---

## 四、各平台接入（都只写一次桥）

### Android / Kotlin（`ResolverNative`）

```kotlin
object ResolverNative {
    init { System.loadLibrary("resolver") }
    external fun resolveRequest(requestJson: String): String   // 唯一入口
    external fun capabilities(): String
}

// 用法
val resp = JSONObject(ResolverNative.resolveRequest("""{"op":"parse_task","payload":{"content":$contentJson}}"""))
if (resp.getBoolean("ok")) render(resp.getJSONArray("result")) else showError(resp.getJSONObject("error"))
```

### C / C++ / C#（FFI）

```c
#include "resolver.h"
char* resp = resolve_request("{\"op\":\"capabilities\"}");
/* 用完必须 free */
resolver_free_string(resp);
```

### Python

```python
import ctypes, json
lib = ctypes.CDLL("./libresolver.so")
lib.resolve_request.argtypes = [ctypes.c_char_p]
lib.resolve_request.restype = ctypes.c_void_p
lib.resolver_free_string.argtypes = [ctypes.c_void_p]

def call(req: dict) -> dict:
    ptr = lib.resolve_request(json.dumps(req).encode())
    try:
        return json.loads(ctypes.cast(ptr, ctypes.c_char_p).value)
    finally:
        lib.resolver_free_string(ptr)

print(call({"op": "parse_task", "payload": {"content": content}}))
```

### Web / WASM

```js
const req = JSON.stringify({ op: "parse_task", payload: { content } });
const resp = JSON.parse(resolverMod.resolve_request(req));   // 同名字段，同一信封
if (!resp.ok) throw new Error(resp.error.code);
```

---

## 五、给接入方的三条硬约定

1. **只判断 `ok` 与 `error.code`**，不要解析 `message`（文案会变）。
2. **`meta.layer == "none"` 时必须上报 `meta.shape.hash`**（不含题目文本），
   这是官方出新结构时我们唯一的发现渠道。
3. **答案/题面已清洗完毕**（HTML、零宽字符、`ets_th{n}` 前缀），客户端**不要再清洗**，
   否则两端文本又会对不上。

## 六、演进规则（保证"不改 ABI"）

- 新增能力 → 新增 `op`（客户端 `capabilities` 里探测 `status`）。
- 新增字段 → 只加不改不删；客户端忽略未知字段。
- 错误码 → 只增不改含义；新增码必须进 `capabilities.error_codes`。
- 破坏性变更 → 涨 `CONTRACT_VERSION`，且**同时支持旧版本请求至少一个发布周期**。
