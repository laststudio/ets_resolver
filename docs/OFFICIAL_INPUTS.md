# 我需要你提供的官方内容（按优先级）

> 状态图例：✅ 已收到并已落地 ｜ 🟡 部分收到 ｜ ⬜ 仍缺
> 每一项写清楚：**要什么 / 为什么 / 期望形态 / 数量**。

## 进度总览

| 编号 | 项目 | 状态 | 落地位置 |
|---|---|---|---|
| P0-1 | 官方题型全集（`structure_type` + 词汇 `question_type`） | ✅ | `src/spec.rs` → `op: spec` |
| P0-2 | 每类题型的官方展示要素表 | ✅ | `src/spec.rs::DISPLAY_ELEMENTS` |
| P0-3 | 关键边界判定（多答案顺序 / 填空题号 / 朗读 answers / 广东 exam_type_name） | ✅ | `src/spec.rs::boundary_rules` |
| P0-4 | `res.json` 题号与满分权威结构 | ✅ 真实 res.json 已获取、已入库、已闭环 | `src/template.rs` + `src/cloud.rs::cloud_get_homework_struct` |
| P1-1 | 习题集（一整套多卷）样例 | 🟡 真实 `模拟试题1`（7 包）已跑通并入库 | `src/homework_set.rs` + `tests/corpus/guangdong_mock1/`；仍缺官方客户端展示截图做最终核对 |
| P1-2 | 官方接口响应样例 | 🟡 `g/homework/list`(status 1/2/3) 真实响应已用于校准 | `tests/fixtures/homework_list_guangdong.json`；PK/词汇响应仍缺 |
| P1-3 | 字段字典（`set_type`/`kind`/`mock`/`exam`/`set_id`/`resource_id`） | 🟡 已用真实数据反推校准（`set_type=1` 不是模拟卷标记） | `src/homework.rs`；仍缺官方明文定义 |
| P2-1 | 加密 ZIP 样本 | ✅ 7 个真实 ZIP 全链路通过（MSTCHINA 尾部 + 64 位密码） | `src/cloud.rs::zip_password_from_footer` 实测一致 |
| P2-2 | 异常/边界包 + 官方表现 | ⬜ | 待三端兜底统一 |
| P2-3 | CDN 主机与证书异常清单 | ⬜ | 待 TLS 精确放行定稿 |
| P3-1 | 官方更新的"变更前后对照" | ⬜ | 待新形状自动发现流程 |
| P4-1 | 能力边界选择（解析库 / +评分TTS / +提交） | ⬜ | 待你确认 |

---

## 📊 官方题库盘点已完成（数据源：laststudio/ets100_resources）

收到 `ets100_resources` 后一次性解决了两件大事：

| 项 | 结果 |
|---|---|
| 规模 | 139,314 套题 / **441,931 条小题明细** / 37,100 套有效套题 |
| 全量"大题名" | **2,157 种**（其中 924 种只出现 1 次）—— 按名字适配不可行 |
| 官方解析家族 `exam_type_collect` | **只有 9 种，且全部已被 spec 收录** ✅ |
| `res.json` 跨地区兼容性 | 抽样 **221 套真实模板，221/221 解析成功、零失败**，卷面分全部取自官方 `set_score` |
| 新发现 | `collector.repeat_dialogue` / `collector.essay` 是官方模板真实取值，已补录 spec |
| 重要结论 | 118 个官方大题名里 **9 个跨家族**（「回答问题」→ dialogue/role 等）→ **中文名不可单独用于推断解析器** |
| 新取值域 | `evaluation_type` 11 种、`engine_type` 2 种（含 `Simulation`/`study`），已登记进 spec |

产物：`tests/fixtures/official_type_inventory.json`、`tests/fixtures/official_exam_type_map.json`、
可复现脚本 `tools/mine_official_map.py`、分析报告 [OFFICIAL_TYPE_INVENTORY.md](./OFFICIAL_TYPE_INVENTORY.md)、
门禁测试 `tests/official_inventory.rs`（官方新增家族 → CI 立刻失败）。

---

## 📍 res.json 在哪里获取（链路已在你后端里，无需去客户端找）

**结论：不用去官方客户端找，你后端已经实现了这条链路** ——
`homework.py::fetch_official_res_json(template_url)` + `parse_official_res_json(bytes)`。

完整的四步（已实测跑通，广东 30 分卷真实数据）：

```text
① POST /m/homework/struct   {id: homework_id, resource_id, parent_account_id, token, ...}
   → body.audio[0].set_name                    # 试卷集名："广州8年级离线个人版模拟试题2(copy)"
   → body.audio[0].template.url                # 模板包："834c62a165faa2e86182c97bba2ab925.zip"
   → body.audio[0].contents[].code_id          # "st1"… 与模板 exam_list[].exam_id 的 "{{st1}}" 对应
   → body.audio[0].contents[].group_name/real_id/url

② GET https://cdn.subject.ets100.com/made/<template.url>     （1.8 MB，**未加密**）
   ⚠️ 该 CDN 返回 default.chinanetcenter.com 证书 —— 必须做**主机级**证书名例外
      （解析器 cloud_download_file 就是这么做的；别用全局 trust-all）

③ 解压模板包 → res.json（+ ctrl.json / info.json / material/*）

④ res.json 真实结构（全部字段已实测）：
   set_score: "30"
   exam_type_list[]: exam_type_order, exam_type_name, exam_type_intro,
                     exam_type_collect, exam_type_score, exam_type_count, exam_list[]
     exam_list[]:    exam_order, exam_id("{{st1}}"), exam_info[], exam_score, exam_count, question_list[]
       question_list[]: question_order, question_count, question_score,
                        evaluation_type, engine_area, engine_type
```

**解析器侧已把这条链路补齐**（`libcloud.so`）：
新增 `cloud_get_homework_struct(token, homework_id, resource_id, parent_account_id)`
一次返回 `set_name` / `template{url,download_url,size}` / `contents[{code_id,group_name,real_id,download_url}]`，
调用方下载模板包（未加密）解压后把 `res.json` 交给 `op: res_template` / `op: parse_homework_set` 即可。

**实测结果（真实广东 30 分卷）**：`set_score=30`，7 个大题、
逐题满分合计 30.0；用真实 7 个资源包 + 真实 res.json 合并 → **7/7 包全部挂上模板、0 未匹配、
逐题满分全部来自模板（`template_per_question`）、卷面分 30.0（`res_template_set_score`）**。

**真实 res.json 也纠正了一个假设**：官方把「听选信息」拆成 **听选信息1/2/3 三个大题**
（各 3 分、2 题），资源包 group_name 同样带序号 —— 所以分节必须**保留序号**，
不能按名字去重合并（此前的"合并成 6 题一节"是错的，已修）。

---

## 🧪 真实数据实测（2026-10-05，广东 2026 测试账号）

用真实云端账号跑通「历史作业 → 下载 → 解密 → 解析 → 合并」全链路，并因此修掉两个真 bug：

| 项 | 结果 |
|---|---|
| 作业列表 | 76 个作业（status 1/2/3）全部拉到，已存为分类回归 fixture |
| 目标作业 | `模拟试题1`（历史作业，广东中考听说 30 分卷形态，**7 个资源包**） |
| 下载解密 | 7/7 成功：MSTCHINA 尾部 → 64 位密码 → zipfile 解压 → `content/info/content2` + 音频 |
| 解析一致性 | Rust vs 后端 Python 兜底：**14 题 / 题干 / 答案 / 选项 零差异** |
| 合并结果 | 5 大题 == 官方结构：模仿朗读 1 + 听选信息 6 + 回答问题 4 + 信息转述 1 + 询问信息 2 = 14 题，卷面题号连续 1..14 |
| 老客户端对比 | `ets_get_answer` 的提取器只认「听后选择/听后回答/听后转述/短文朗读」4 个名字，对这份广东卷**7 个包全部返回空** |

**由真实数据修掉的 bug**
1. `set_type == 1` 被当成"模拟卷"标记 → 真实列表里它对几乎每个作业都成立（含单包朗读），
   76 个作业里 **64 个被误判成模拟卷**。已改为只信 `kind=mock`/`mock=1`/`exam=1`，
   名字关键词需配合"多包"；修复后模拟卷 8 个（九年级模拟卷01/02、期中期末测试A/B、模拟试题1/40）。
2. 无模板合并时按 `structure_type` 分节 → 广东卷的 `听选信息/回答问题/询问信息` 同为
   `collector.role`，被错误并成"一节 12 题"。已改为按**归一化大题名**分组，
   并使 `听选信息1/2/3` 正确合并成"听选信息"一节（6 题）。

---

## ✅ 已收到并已落地

### P0-1 官方题型全集

- **收到**：14 个官方 `structure_type` / 引擎标识（`collector.read`、`collector.repeat`、`collector.sentence`、
  `collector.word`、`repeat`、`repeat_dialogue`、`essay`、`templet_common`、`listen_choice_standard`、
  `listen_fill_standard`、`read_passage_standard`、`read_sentence_standard`、`read_word_standard`、
  `read_and_choose_standard`）；词汇双套系统（`QuestionSymbol` 0~13 + `SyncWordQuestionBean` 6/16~20）。
- **已落地**：`src/spec.rs` 固化为机器可读注册表；`op: spec` 对外输出；覆盖率自证
  连同 content.json 长期在用的 `collector.*` 规范性标识一起收录为 **22 项**，其中 **21 项已显式适配**
  （仅 `templet_common` 依赖 res.json 模板）；
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

### 1. 其他地区/学段的 `res.json`（P0-4 已完成广东初中，仍想看差异）

- **要什么**：**北京初中 / 北京高中 / 广东高中** 各 1–2 份真实 `res.json`
  （走上面那条 `/m/homework/struct → template.url` 链路即可自己取，不必人工翻客户端）。
- **为什么**：广东初中已闭环（7 大题、30 分、`collector.*`）。想确认其它地区是否：
  ① `exam_type_collect` 出现新的取值；② 是否存在一个 `exam_type_list` 里多个 `exam_list`；
  ③ `evaluation_type` / `engine_type` 的取值域；④ 卷面分是否只有 `exam_type_score` 没有 `exam_score`。
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
