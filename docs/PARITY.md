# 与官方客户端完全一致（Parity）实施蓝图

> 相关文档：[跨平台固定接口契约](./INTERFACE.md) ｜ [需要官方提供的素材清单](./OFFICIAL_INPUTS.md)

> 目标：**解析结果与官方客户端等价，并且这个等价性可被持续验证**。
> 前提：我们持有官方客户端的接口与实时数据，可以随时取到"官方基准（oracle）"。
> 原则：所有逻辑只写在 `ets_resolver` 一处，`Fe-pro`（Android）与 `lastudio-platform`（后端）只消费。

---

## 一、"完全一致"的可操作定义

把"和官方一样"拆成 8 个可测的等价类（E1–E8）。任何一次改动都必须让这 8 项保持绿色：

| 编号 | 等价类 | 判定内容 |
|---|---|---|
| **E1** | 题目切分一致 | 题数相同；每题归属的 `stid` / 官方题号相同；一题不多、一题不少。**词汇侧必须在正确通道下切分**（`question_type=6` 在超能单词通道是"单词跟读"、在同步PK通道是"听音选词"） |
| **E2** | 题面文本一致 | 清洗后的最终字符串逐字节相同（含 `<br>`/`<p>` 转空格、零宽字符去除、`ets_th{n}` 前缀去除） |
| **E3** | 答案集合一致 | 每个题目的答案数组内容与顺序相同（**原样不排序**，与官方源码一致：只 append、不 sort） |
| **E4** | 选项集合一致 | 选项文本与顺序相同（`A. xxx` 形式） |
| **E5** | 卷面顺序一致 | 大题顺序取 `res.json` 的 `exam_type_list` 下标；小题按 `question_order` 结合 `xth`/`th` 递增（**权威来源已接入**：`op: res_template`） |
| **E6** | 作业分类一致 | 作业类别（词汇PK/模拟套卷/习题集/作文/在线/常规）、**卷面总分取 `exam_score` / `question_score`**（不再用 `group_name` 权重推算）、大题名与官方一致 |
| **E7** | 端到端一致 | 官方加密 ZIP → 尾部密码 → 解压 → 读取 `content/info/content2` → 解析，全链路一致（**音频与 CDN 保持原样，不做脱敏**） |
| **E8** | 不退化 | 官方新增结构时**不静默丢**：`spec_known=false`（不在官方权威题型表里）与新形状指纹都能被发现并进入待适配清单 |

---

## 二、三层防线架构

官方结构会演进，所以任何"只靠 `structure_type` 字符串"的方案都迟早漏。解析器采用三层，每层都输出 `layer` 与置信度，客户端可据此决定是否提示"未完全适配"：

| 层 | 名称 | 依据 | 现状 |
|---|---|---|---|
| **L1** | 显式路由 | `structure_type` 字符串 | ✅ 已实现（`collectors.rs::parse_content_routed`） |
| **L2** | 结构嗅探 | JSON **形状**：`info.question[]/questions[]`、`info.sublist[]`、`info.std[]`(有 `xth`=填空/否则转述)、`info.xtlist[]`、`info.value`、word 形状 | ✅ 已实现（`collect_generic_routed`，输出 `generic:*` 家族名） |
| **L3** | 兜底抽取 | 全 JSON 递归找 `std[].value` / `answer` / `answers` 等语义键，抢救出答案并标记 `confidence=low` | ⏳ 待实现（E8 的最后一道保险） |

L1/L2 命中情况已经可查：`diagnose()` 返回 `layer ∈ {explicit, generic, none}` 与 `family`。
**`layer == "none"` 就是必须处理的信号**——它表示官方结构已经超出认知范围。

---

## 三、Oracle（官方基准）层级

`golden.json` 的 `captured_from` 必须标明来源；一致性测试只把下面这些来源算作"已证明"：

| 优先级 | 来源 | `captured_from` 取值 |
|---|---|---|
| 1 | 官方客户端 UI 实际渲染出来的题目/答案 | `official_client` |
| 2 | 官方接口返回：`res.json`（结构与逐题满分）、`homework/score-detail`（真实作答顺序）、`homework/struct`、`pk-info` | `official_api` / `official_res_json` |
| 3 | 人工复核过的结果 | `manual_reviewed` |
| — | resolver 自己录的（**不构成证据**，只用于跑通流程） | `resolver_self_recorded` |

> 这一点已经写进代码：自录 golden 会得到 `self_recorded` 状态，**不计入** official_ratio；
> `PARITY_STRICT_OFFICIAL=1` 时它会直接让 CI 失败。

---

## 四、采集管道（实时更新的关键）

你手上已有官方接口且能实时更新，管道应当自动化成"每天自动新增用例、自动发现新结构"：

```
① 拉列表     官方 /g/homework/list status=1/2/3 全量遍历
② 下载原包   原始加密 ZIP（含音频，保留真实 CDN 地址，不脱敏）
③ 算密码     解析器 cloud_zip_password（与官方客户端同一算法）
④ 解压落盘   content.json / info.json / content2.json + source.zip + meta.json
⑤ 取 oracle  官方客户端 UI 或官方接口 → golden.json（captured_from=official_*）
⑥ 跑诊断     diagnose() → shape.hash、layer、family、warnings
⑦ 分流
     · shape 未见过 → 生成"待适配"条目（形状路径 + 样例 id + 影响作业数）
     · shape 已见过 → 直接进回归集
⑧ 出报告     PARITY_REPORT=xxx.json → 看板（official_ratio / layer 分布 / 新增形状）
```

**存储约定（重要）**：音频与原始 ZIP 体积大，**不进 git**。
- 仓库内：`tests/corpus/manifest.json`（用例清单：id、sha256、地区、题型、来源、抓取时间）+ 少量最小样例 `tests/corpus/_examples/`
- 仓库外：`corpus/`（本地盘或对象存储），通过 `PARITY_CORPUS=/path/to/corpus` 指向

---

## 五、覆盖率指标（把"完美"变成数字）

一致性测试每次运行都会打印并写入报告：

| 指标 | 含义 | 目标 |
|---|---|---|
| `official_ratio` | `verified / total`，即有多少用例被官方来源证明 | **≥ 99%** |
| 未识别率 | `layer == "none"` 占比 | **0** |
| 嗅探兜底率 | `layer == "generic"` 占比 | 逐步下降（不求 0，L2 是正当防线） |
| 新形状增量 | 每周新增 shape hash 数量 | 出现即处理，**24 小时内** |
| 端到端通过率 | E7（ZIP 链路）通过的用例占比 | 100% |

---

## 六、C 类：老师布置的一整套多卷（习题集）

这是本次新增的识别能力，已在解析器内落地（三端只需消费结论）：

```jsonc
// classify_homework(homework_json) 的输出
{
  "category": "paper_set",          // vocab_pk | mock_suite | paper_set | composition | online | oral
  "is_exercise_set": true,          // ← 老师布置的一整套多卷（需要多包合并成一张卷）
  "is_vocab_pk": false,
  "is_mock_suite": false,
  "is_mock_suspected": true,        // 名字像模考但没有官方显式标记（仅提示，不擅自判定）
  "is_composition": false,
  "is_online": false,
  "package_count": 6,
  "expected_total_score": 30.0,
  "score_source": "mock_suite_default",  // 口径来源，便于核对两端分母
  "set_id": "set-9981",
  "resource_id": "58",
  "reasons": ["packages:6", "set_id:set-9981"],  // 命中规则，误判时靠它排错
  "warnings": ["mock_suspected_by_name"]
}
```

**判定原则（踩过坑之后的结论）**：
- 模拟考只信官方显式字段（`kind=mock` / `mock=1` / `exam=1` / `set_type=1`）；
  名字关键词（套题/模考/…）降级为 `is_mock_suspected` 告警 —— 后端曾因这类启发式把普通作业误判成"模拟考封存"。
- `is_exercise_set` 的判据是"**要不要合并多包**"（`package_count >= 2`，或带 `set_id` 的单包），
  所以模拟套卷同样为 `true`：下游要的是这个语义。

**下一步（M2）**：新增 `parse_homework_set(packages[])`，把后端 `format_parsed_paper` 与
Fe-pro `normalizeCloudParsedSections` 的合并逻辑收口进来（`res.json` 的权威顺序/满分已就绪，
直接复用 `template::parse` + `apply_res_template`）：

```
多包 → 按 res.json 的 exam_type_list 下标排大题（无模板时退回 contents[].order / real_id）
     → 逐包解析 → 合并分节（含听后选择分包合并）→ 全局题号（xth/th + question_order）
     → 逐题满分（question_score）→ 输出 paper
```

---

## 七、接口清单

| 接口 | 状态 | 说明 |
|---|---|---|
| `resolve_request(op)` / `capabilities()` | ✅ 已完成 | 跨平台固定契约（见 [INTERFACE.md](./INTERFACE.md)）：新能力只加 op，不改 ABI |
| `spec` (op) | ✅ 已完成 | **官方权威规格**：22 个题型全集（引擎标识 14 + content.json 常见 `collector.*` 8）的官方名/模型/场景/处理器、词汇双通道（`question_type=6` 双语义）、展示要素表、res.json 权威路径、边界规则、覆盖率自证 |
| `diagnose(content, info, content2)` | ✅ 已完成 | 命中层/家族/形状指纹/告警；**不含题目文本**；新增 `structure_type_not_in_official_spec` 告警 |
| `shape_fingerprint(content, info, content2)` | ✅ 已完成 | 结构指纹（不含取值、不含 `structure_type`）：改名不改结构 → 指纹不变 |
| `classify_homework` / `classify_homework_list` | ✅ 已完成 | 作业分类 + 卷面总分口径，返回 `exercise_set_ids` |
| `res_template(res)` | ✅ 本次新增 | 规范化官方 `res.json`：大题顺序（`exam_type_list` 下标）、大题分、**逐题满分** |
| `apply_res_template(res, questions)` | ✅ 本次新增 | 把满分/题号盖到解析结果（只补不改内容），支持按 `structure_type` 自动定位分节 |
| 语料库一致性测试 | ✅ 已完成 | `tests/corpus_parity.rs` + `PARITY_*` 环境变量 + 机器可读报告 |
| `cloud_get_homework_list` 字段透传 | ✅ 已完成 | 已补 `pk/set_type/set_id/resource_id/kind/mock/exam/order/real_id/point/total_point`，三端不再各写一份列表解析 |
| `parse_homework_set(packages[], res?, homework?)` | ✅ 已完成 | 习题集多包合并：官方 res.json 定大题顺序与逐题满分；同 `exam_type_collect` 的分包合并成一个大题；全局题号连续 |
| L3 兜底抽取 | ⏳ M2 | 名字与形状都不认识时的最后抢救 + 低置信标记 |

以上能力在 **Android 端统一走 `ResolverNative.resolveRequest(json)`**（新增 op 无需改客户端声明）。

### 官方权威规格带来的修正（本轮）

- `listen_choice_standard` 不含 "choose"，原先会掉进结构嗅探 → 现已显式路由到选择题；
  同时加回归保护：`word.choice`（含 "choice"）**不得**被误判为选择题。
- `collector.sentence` / `read_*_standard` 归入朗读家族（文本集合形态）。
- 逐题满分与题号改由 `res.json` 权威提供，`group_name_weights` 降级为**无模板时的兜底**。
- 词汇双通道：同一 `question_type` 在不同通道语义不同，输出 `channel` / `symbol_name` /
  `ambiguous_across_channels` 三类元信息，杜绝"只看数字"的误判。

---

## 八、里程碑与验收

| 阶段 | 内容 | 验收 |
|---|---|---|
| **M1（已完成）** | 诊断接口 + 形状指纹 + 作业分类 + 语料库门禁 | `cargo test --workspace` 全绿；门禁能区分 official 与自录 golden |
| **M2** | 列表字段透传 + `resolve_homework_set` + L3 兜底 | 习题集多包在两端（Android/后端）用同一实现合并；`official_ratio` 有真实数据 |
| **M3** | 接入你的实时采集管道 + 看板 + 24h 新结构响应流程 | `official_ratio ≥ 99%`；新形状出现能在 24h 内被发现并处理 |

**最终验收**：E1–E8 全部绿色，且每次官方更新后 24 小时内恢复全绿。

---

## 九、命令速查

```bash
# 全量测试（含一致性门禁）
cargo test --workspace

# 看一致性报告（含路由家族分布与形状指纹聚类）
cargo test --test corpus_parity -- --nocapture

# 录制/更新 golden（自录，仅用于跑通流程；正式 golden 必须来自官方）
PARITY_UPDATE=1 cargo test --test corpus_parity

# CI 门禁：语料库为空即失败 + 只认官方来源的 golden
PARITY_REQUIRE_CORPUS=1 PARITY_STRICT_OFFICIAL=1 cargo test --test corpus_parity

# 指向仓库外的语料库（含音频与原始 ZIP，不进 git）
PARITY_CORPUS=/path/to/corpus cargo test --test corpus_parity -- --nocapture

# 产出机器可读报告给后台看板
PARITY_REPORT=/tmp/parity.json cargo test --test corpus_parity -- --nocapture
```

Python（ctypes）快速诊断一份资源包：

```python
import ctypes, json
lib = ctypes.CDLL("./target/release/libresolver.so")
lib.diagnose.argtypes = [ctypes.c_char_p] * 3
lib.diagnose.restype = ctypes.c_void_p
lib.resolver_free_string.argtypes = [ctypes.c_void_p]

content = open("content.json", "rb").read()
info = open("info.json", "rb").read() if __import__("os").path.exists("info.json") else b""
ptr = lib.diagnose(content, info, b"")
print(json.dumps(json.loads(ctypes.cast(ptr, ctypes.c_char_p).value), ensure_ascii=False, indent=2))
lib.resolver_free_string(ptr)
```
