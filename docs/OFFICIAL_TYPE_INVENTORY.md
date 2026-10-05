# 官方题型盘点（数据源：laststudio/ets100_resources）

> 目的：把"适配全部题目"的定义域从猜测变成**可核对的清单**。
> 数据：139,314 套题 / 441,931 条小题明细（SQLite 分库）+ 221 套真实模板包内的 `res.json`。

## 一、结论先行

| 问题 | 实测答案 |
|---|---|
| 官方有多少种"大题名"（`group_name`）？ | **2,157 种**，其中 **924 种只出现过 1 次**；只有 **217 种**出现 ≥100 次 |
| 官方有多少种 `type_name`？ | 167 种 |
| 官方有多少种**解析家族**（`exam_type_collect`）？ | **只有 9 种**，且**全部已被 spec 收录** ✅ |
| 靠中文大题名推断解析家族可行吗？ | **不可行**：118 个官方大题名里有 **9 个**在不同卷型下归属不同家族（如「回答问题」→ `dialogue`/`role`、「情景反应」→ `dialogue`/`role`） |
| `res.json` 跨地区兼容性 | 221/221 套模板**全部解析成功、零失败**，卷面总分全部来自官方 `set_score` |
| 官方模板里出现过、但 spec 未收录的家族 | **0 个**（门禁测试 `official_inventory.rs` 会持续盯住） |

**因此适配策略是明确的**：不要按 2,157 个名字去适配，而是
① 认 **9 个 `exam_type_collect` 家族**（模板权威）＋
② 认 **结构形状**（`collect_generic` 结构嗅探，完全不看名字）＋
③ 用 `code_id ↔ exam_id` 做官方挂载键。

## 二、9 个官方解析家族（实测分布，221 套模板）

| `exam_type_collect` | 出现次数 | spec 中的处理器 |
|---|---|---|
| `collector.choose` | 110 | `collect_choose` |
| `collector.read` | 42 | `collect_read` |
| `collector.role` | 40 | `collect_question_based`（听选/回答/提问同族） |
| `collector.picture` | 37 | `collect_picture` |
| `collector.fill` | 25 | `collect_fill` |
| `collector.dialogue` | 20 | `collect_question_based` |
| `collector.word` | 3 | `collect_read`（词汇文本训练） |
| `collector.repeat_dialogue` | 3 | `collect_question_based` |
| `collector.repeat_essay` | 3 | `collect_repeat_essay` |

## 三、其余权威取值域（实测）

| 字段 | 取值 |
|---|---|
| `question_list[].evaluation_type` | `read_word` / `read_sentence` / `read_chapter` / `read_choice` / `choose` / `Fill_in_the_blanks` / `simple_expression` / `simple_expression_ufk` / `simple_expression_ufi` / `simple_expression_ufj` / `topic` |
| `question_list[].engine_type` | `study` / `Simulation`（另有空值） |
| `res.json` 顶层 | `set_score`（权威卷面分）/ `set_intro` |
| `exam_type_list[]` | `exam_type_order` / `exam_type_name` / `exam_type_intro` / `exam_type_collect` / `exam_type_score` / `exam_type_count` / `exam_list[]` |
| `exam_list[]` | `exam_order` / `exam_id`（占位 `{{st1}}`）/ `exam_info[]` / `exam_score` / `exam_count` / `question_list[]` |
| `question_list[]` | `question_order` / `question_count` / `question_score` / `evaluation_type` / `engine_area` / `engine_type` |

## 四、Top 30 官方大题名（按小题明细条数）

```
16364 短文跟读      12610 情景反应      11676 模仿朗读      11474 听后选择
11168 Vocabulary    10315 听对话选答案   9849 短对话理解      9668 朗读短文
 9496 短文朗读       9331 朗读           8327 听对话回答问题   8229 Words
 7690 信息转述       7080 对话跟读        6777 情景问答        6729 听选信息
 6719 听长对话或独白选答案                6628 回答问题        6609 朗读句子
 6599 听后回答       6444 听说训练1       6433 询问信息        6431 听对话或独白选答案
 6303 听对话和短文答题                   6289 长对话理解      6124 听长对话选答案
 5629 情景应答       5404 听短对话回答问题 5390 句子理解       5384 听短对话选答案
```

完整清单（2,157 个名字 + 计数）：[`tests/fixtures/official_type_inventory.json`](../tests/fixtures/official_type_inventory.json)
实证映射（127 组名字 × 家族）：[`tests/fixtures/official_exam_type_map.json`](../tests/fixtures/official_exam_type_map.json)

## 五、如何复现 / 更新

```bash
git clone https://github.com/laststudio/ets100_resources.git   # 约 473MB
python3 tools/mine_official_map.py /path/to/ets100_resources \
        tests/fixtures/official_exam_type_map.json 3
cargo test --test official_inventory
```

脚本会：抽样各分类的真实套题 → 用官方模板包地址下载 `res.json`（CDN 主机名例外已内置）
→ 通过 `exam_id({{st1}}) ↔ code_id(st1)` 关联出"官方大题名 → 解析家族"→ 写出 fixture。

**门禁**：`tests/official_inventory.rs` 会断言
① 官方出现的每个 `collect` 都在 spec 里（新增家族 → 测试失败 → 提示补适配）；
② `evaluation_type` / `engine_type` 取值域不超出 spec 登记；
③ 名字空间依然巨大且长尾（提醒不要走"按名字适配"的路）。

## 六、广东高中（高考听说 60 分卷）—— `collector.3q5a` 已用真实数据闭环

此前唯一没有被真实数据覆盖的家族就是 `collector.3q5a`。用广东高中账号实测 `模拟试题1`
（作业 id 102024948）后已闭环：

| 项 | 实测 |
|---|---|
| 试卷集名 | 高一（上）模拟考试一（模板包 `25acf95e…zip`） |
| 官方 res.json | `set_score=60`，3 个大题：模仿朗读 **20** + 角色扮演 **16** + 故事复述 **24** |
| `collector.3q5a` 分值 | 8 小题 × 2 分 = 16 分，`evaluation_type = simple_expression` |
| 角色扮演载荷 | `content.json` 只有 `info[].code_id/code_value`（对话原文/图片/ask1–ask8）；**真正的题目结构在 `content2.json`**（`info.question[].std[].value`） |
| 解析结果 | **8 题**：三问（ask1–3，中文题干→英文参考问句，多条）+ 五答（ask4–8，英文问题→多条参考答案 + `important` 采分点） ✅ |
| 合并结果 | 3 大题 10 题，卷面分 **60.0**（`res_template_set_score`），逐题满分 `[20, 2×8, 24]`，3/3 包挂上模板、0 未匹配 |

**顺带发现两个真实数据特征（都不做归一化，以保持 E2/E3 逐字节一致）**：
1. 官方参考答案用**弯引号** `U+2019`（`don’t` / `Ali’s`）—— 绝不能替换成 `'`；
2. `group_name` 里可能嵌 `\r\n`（如 `必修一U4A\r\n模仿朗读`）—— 匹配时需把空白折叠后比较
   （已实现 `name_match_key`），否则挂不上模板。

## 七、还没覆盖到的部分

- **其他地区/学段的 `res.json` 字段差异**：本次抽样已跨 107 个大类、221 套模板且零失败，
  但仍建议按地区（北京/江苏/上海/山东/广东）各抽一批复核；
- ~~`collector.3q5a`（广东高中三问五答）~~：**已用真实广东高中数据闭环** ✅（见第六节）；
- **词汇双通道**（`Symbol_*` vs `sync_pk`）：题库只有听说卷，词汇通道需要从作业接口取真实载荷核对。
