# 我需要你提供的官方内容（按优先级）

> 状态图例：✅ 已收到并已落地 ｜ 🟡 部分收到 ｜ ⬜ 仍缺
> 每一项写清楚：**要什么 / 为什么 / 期望形态 / 数量**。

## 进度总览

| 编号 | 项目 | 状态 | 落地位置 |
|---|---|---|---|
| P0-1 | 官方题型全集（`structure_type` + 词汇 `question_type`） | ✅ | `src/spec.rs` → `op: spec` |
| P0-2 | 每类题型的官方展示要素表 | ✅ | `src/spec.rs::DISPLAY_ELEMENTS` |
| P0-3 | 关键边界判定（多答案顺序 / 填空题号 / 朗读 answers / 广东 exam_type_name） | ✅ | `src/spec.rs::boundary_rules` |
| P0-4 | `res.json` 题号与满分权威结构 | 🟡 结构已确认，**仍缺真实样本** | `src/template.rs` → `op: res_template` |
| P1-1 | 习题集（一整套多卷）样例 | ⬜ | 待 M2 `parse_homework_set` |
| P1-2 | 官方接口响应样例 | ⬜ | 待定稿 PK/词汇顺序 |
| P1-3 | 字段字典（`set_type`/`kind`/`mock`/`exam`/`set_id`/`resource_id`） | 🟡 词汇侧已给，作业侧仍缺 | `src/homework.rs` 仍是名字关键词兜底 |
| P2-1 | 加密 ZIP 样本 | ⬜ | 待 E7 端到端回归 |
| P2-2 | 异常/边界包 + 官方表现 | ⬜ | 待三端兜底统一 |
| P2-3 | CDN 主机与证书异常清单 | ⬜ | 待 TLS 精确放行定稿 |
| P3-1 | 官方更新的"变更前后对照" | ⬜ | 待新形状自动发现流程 |
| P4-1 | 能力边界选择（解析库 / +评分TTS / +提交） | ⬜ | 待你确认 |

---

## ✅ 已收到并已落地（本轮）

### P0-1 官方题型全集

- **收到**：14 个 `structure_type` / 引擎标识（`collector.read`、`collector.repeat`、`collector.sentence`、
  `collector.word`、`repeat`、`repeat_dialogue`、`essay`、`templet_common`、`listen_choice_standard`、
  `listen_fill_standard`、`read_passage_standard`、`read_sentence_standard`、`read_word_standard`、
  `read_and_choose_standard`）；词汇双套系统（`QuestionSymbol` 0~13 + `SyncWordQuestionBean` 6/16~20）。
- **已落地**：`src/spec.rs` 固化为机器可读注册表；`op: spec` 对外输出；覆盖率自证
  （14 项里 **13 项已显式适配**，仅 `templet_common` 依赖 res.json 模板）；
  新增标识全部接入显式路由（`listen_choice`/`sentence`/`read_*_standard`），
  `parse_task.meta.spec_known` 可直接判断"是否官方已知题型"。
- **顺带解决的真问题**：`question_type=6` 双语义（同步通道=听音选词、超能单词通道=单词跟读），
  现在按通道判定并输出 `channel` / `symbol_name` / `ambiguous_across_channels`。

### P0-2 官方展示要素表

- **已落地**：`spec.display_elements`（8 行：题干/音频/图片/选项/答案/原文/评分点/解析 + 答案形态）。
  用途：决定每类题型我们输出哪些字段——例如朗读类 `answer_kind = text_list`（原文即答案），
  听选类 `answer_kind = letter`。

### P0-3 关键边界判定

- **已落地**（`spec.boundary_rules` + 代码行为）：
  - 多参考答案**原样不排序**：我们本来就只做追加，现在这条被固化为契约并可回归验证；
  - 填空小题号取 `info.std[].xth`、卷面大题号取 `th`、`exam_info[].value` 是**指示文本不是题号**；
  - 朗读类 `answers` 直接给朗读原文（与官方界面一致，不打"参考答案"面板）。

### P0-4 `res.json` 权威结构（结构部分）

- **已落地**：`src/template.rs` + `op: res_template` / `op: apply_res_template`：
  - 大题顺序 = `exam_type_list` 下标；
  - `exam_type_collect` ↔ `content.json` 的 `structure_type` 作为挂载钥匙；
  - 逐题满分 = `question_list[].question_score`（**不再用 `group_name` 权重猜**）；
  - `apply_res_template` 把满分/题号盖到解析结果上（**只补不缺、不改内容**），
    并支持用 `structure_type` 自动定位分节（不必传下标）；
  - 兼容 snake_case 与 camelCase 两种字段命名。

---

## ⬜ 仍然需要你提供的（按优先级）

### 1. 真实 `res.json` 样例 ★（P0-4 的验证件）

- **要什么**：每地区/学段 2–3 份**真实** `res.json`（广东初中、广东高中、北京初中、北京高中），
  特别要含 `exam_type_name` / `exam_type_collect` / `exam_score` / `question_list` 的真实取值。
- **为什么**：结构我已按你给的模板实现，但真实文件里字段大小写、缺省字段、多 `exam_list` 合并方式
  都可能不同。有真实样本我才能确认解析没有偏差，并把 `group_name_weights` 彻底废掉。
- **形态**：`res.json` 原文 + 对应作业名/地区。

### 2. 一个真实习题集（P1-1）★

- **要什么**：作业列表 item 原始 JSON（`set_type/set_id/resource_id/kind/mock/exam/pk/contents[].order|real_id`）
  + **全部** resource ZIP + 官方客户端里这张卷的展示（大题顺序/题号/总分）。最好 2 卷 / 4 卷 / 6 卷各一份。
- **为什么**：`parse_homework_set`（多包合并）就等这个；也是把后端 `format_parsed_paper`
  与 Fe-pro `normalizeCloudParsedSections` 合成一份实现的前提。

### 3. 作业侧字段字典（P1-3 剩余部分）★

- **要什么**：`set_type`（1/5/6 各是什么，还有吗）、`kind`（`pk`/`vocab`/`composition`/`mock`/…）、
  `mock`、`exam`、`pk`、`set_id`、`resource_id` 的**取值域与含义**。
- **为什么**：`classify_homework` 里"词汇/模拟考"的判定目前仍有一半靠名字关键词兜底
  （已因此踩过"普通作业被误判成模拟考"的坑）。给我字典，我就能全部换成官方字段。

### 4. 官方接口响应样例（P1-2）

- `/g/homework/list`(status 1/2/3)、`/g/homework/struct`(带/不带 `resource_id`)、
  `/g/homework/pk-info`、`/g/pk/start-word-pk`、`/g/word/struct`、`/g/homework/score-detail`、
  `/m/homework/info`。各 1–2 份即可（**请脱 token**）。
- **为什么**：PK/词汇的真实作答顺序、`struct` 多级回退的真实形态只能从原始响应确认。

### 5. 加密 ZIP 样本（P2-1）与异常包（P2-2）

- 10–20 个真实 ZIP（含尾部格式不同的老包/机考包）+ 异常包清单及其官方表现。
- **为什么**：E7 端到端回归 + 三端异常兜底行为统一。

### 6. CDN 与证书清单（P2-3）

- 全部 CDN 主机名 + 已知证书异常主机（目前只知 `cdn.subject.ets100.com`）。
- **为什么**：TLS 只能按主机精确放行，不能全局 trust-all。

### 7. 官方更新的"变更前后对照"（P3-1）

- 每次官方更新后，受影响题型的 `content.json` 变更前后对照（哪怕只改一个字段）。
- **为什么**：这是"24 小时内跟上官方"的燃料。

### 8. 能力边界确认（P4-1）

三选一：**A（推荐）** 只做识别+解析一致+固定接口；B = A + 评分/TTS；C = A + B + 提交作业/回读分数。

---

## 最小启动集（现在只剩三样）

1. **真实 `res.json` 2–3 份**（验证 P0-4，废掉满分猜测）
2. **一个真实习题集**（列表 item + 全部 ZIP + 官方展示截图）
3. **作业侧字段字典**（`set_type`/`kind`/`mock`/`exam` 取值域）

拿到这三样，我就能：把 E5/E6 用官方口径闭环、实现 `parse_homework_set`、
把 `classify_homework` 的名字关键词彻底换成官方字段。
