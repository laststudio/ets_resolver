# 语料库（parity corpus）

> 需要哪些官方素材来填充它：见 [`docs/OFFICIAL_INPUTS.md`](../../docs/OFFICIAL_INPUTS.md)

这里放**一致性回归用例**：每个 case 是一份真实的官方资源包，加上"官方结果"作为基准（golden）。
测试代码见 [`tests/corpus_parity.rs`](../corpus_parity.rs)，方法论见 [`docs/PARITY.md`](../../docs/PARITY.md)。

## 目录结构

```
tests/corpus/<地区>/<题型>/<case-id>/
    content.json     必需 —— 官方原始 content.json（不解码、不脱敏）
    info.json        可选 —— 官方原始 info.json
    content2.json    可选 —— 官方原始 content2.json
    golden.json      必需 —— 官方结果（期望输出）
    meta.json        可选 —— 元信息
    source.zip       可选 —— 官方原始加密包（含音频与真实 CDN 地址，存档用，测试不解析）
```

## meta.json

```json
{
  "region": "广东初中",
  "type": "collector.role",
  "homework_name": "Unit 3 听选信息",
  "source": "official_client 截图核对 + 官方 res.json",
  "captured_at": "2026-10-05"
}
```

## golden.json

两种写法都支持，**推荐带来源标注**：

```json
{
  "captured_from": "official_client",
  "note": "官方客户端答案页逐题核对",
  "structure_type": "collector.role",
  "questions": [ { "no": 1, "type": "collector.role", "question": "...", "answers": ["..."], "options": [], "important": [] } ]
}
```

`captured_from` 取值与含义：

| 取值 | 含义 | 算不算一致性证据 |
|---|---|---|
| `official_client` | 官方客户端 UI 实际渲染结果 | ✅ |
| `official_api` | 官方接口（struct / pk-info 等）返回 | ✅ |
| `official_res_json` | 官方 `res.json`（结构与逐题满分） | ✅ |
| `manual_reviewed` | 人工复核 | ✅ |
| `resolver_self_recorded` | resolver 自录（跑通用） | ❌ 不计入 official_ratio |

比对字段：`no / type / question / answers / options / important`。

## 约定

- **音频与 CDN 地址保持原样**，不做脱敏 —— 需要它们来验证 E7（加密 ZIP → 密码 → 解压 → 解析）全链路。
- 体积大的 `source.zip` 与整体语料库（成千上万个 case）**不放 git**：
  仓库里只留 `manifest.json` 与少量最小样例（`_examples/`），全量语料用
  `PARITY_CORPUS=/path/to/corpus` 指向本地盘或对象存储。
- `_examples/` 里是手写的最小结构样例，只用于演示目录结构、让门禁有数据可跑，
  **不是**一致性证据（状态会显示为 `self_recorded`）。

## 用法

```bash
# 比对（默认读 tests/corpus）
cargo test --test corpus_parity -- --nocapture

# 指向全量语料库
PARITY_CORPUS=/data/ets_corpus cargo test --test corpus_parity -- --nocapture

# 录制 golden（自录，正式门禁请用官方结果替换 questions）
PARITY_UPDATE=1 PARITY_CORPUS=/data/ets_corpus cargo test --test corpus_parity

# CI 门禁
PARITY_REQUIRE_CORPUS=1 PARITY_STRICT_OFFICIAL=1 cargo test --test corpus_parity

# 输出机器可读报告
PARITY_REPORT=/tmp/parity.json cargo test --test corpus_parity -- --nocapture
```

报告里会打印 **路由家族分布**（L1 显式 vs L2 嗅探）与 **形状指纹聚类**
（同一指纹=同一种结构；出现新指纹就是要跟的新结构）。
