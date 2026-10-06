# 官方核心语义：族分发 / 判分 / 提交字段（静态提取）

> 来源：官方 PC 端前端**明文源码**（远程 web app，已完整静态下载）
> - 主 bundle：`ets源码恢复/pc_client_web/index-5543d811.js`（3.48 MB）
> - chunk：`ets源码恢复/pc_client_web/chunks/*.js`（82 个）
> - 本文所有代码片段**逐字取自上述文件**，不美化、不改写。
> 提取方式：纯静态 grep/切片，未运行客户端、未抓包。

---

## 1. 族清单：官方只有 9 个族（PC 前端）

```bash
$ grep -ohE '"collector\.[a-z_0-9]+"' chunks/*.js | sort -u
collector.3q5a  collector.choose  collector.dialogue  collector.fill  collector.picture
collector.read  collector.repeat  collector.role     collector.word
```

**重要**：PC 前端**没有 `collector.repeat_essay` / `collector.repeat_dialogue`**。
官方 Android 侧另见 `collector.sentence`（见 `jadx_paper_utils/.../PaperUtilsKt.java`），
说明"逐句跟读"等类型是**派生族**（由一个映射函数产生），不是模板里的原始族。

我方 `src/collectors.rs` 的族常量（10 个 + 6 个 PK 子型）与之对照见 `README.md` 的对照表。

---

## 2. 族分发与判分：`useExamResult-d9fb4671.js` 的 switch

```js
switch("collector.read"!==n.examType.exam_type_collect&&(I()||c.showScore(_,d,0)),n.examType.exam_type_collect){
  case"collector.3q5a":
  case"collector.role":
  case"collector.dialogue":
  case"collector.word":
  case"collector.repeat":
    I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);
    break;
  case"collector.read":
    n.examType.exam_list.length>1 ? /* 多题朗读：同 3q5a 路径 */ : /* 单题朗读：见下 */ ;
    break;
  case"collector.choose":
    setTimeout(()=>{const t=(o=n.htmlData,i=e,o.reduce((e,t)=>{
      const o=i.filter(e=>e.entity_id===t.info.stid).reduce((e,o)=>{
        const i=b(t.info.xtlist.find(e=>e.xt_xh===""+o.order)?.answer);
        return o.detail.choose
          ? e.push(`${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}`)
          : e.push(`${o.entity_id}_${o.order}${q}${i}`),e},[]);
      return e.concat(o)},[]));var o,i;c.setPCChoose.apply(null,t)},0);
    break;
  case"collector.fill":
    setTimeout(()=>{const t=(o=n.htmlData,i=e,o.reduce((e,t)=>{
      const o=i.filter(e=>e.entity_id===t.info.stid).reduce((e,t)=>(
        t.detail.choose
          ? e.push({id:`${t.entity_id}_${t.order}`,answer:t.detail.choose,status:((t.real_score||t.score_rate)??0)>0?1:0})
          : e.push({id:`${t.entity_id}_${t.order}`,answer:"",status:0}),e),[]);
      return e=e.concat(o),JSON.stringify(e)},[]));var o,i;c.setPcBlank(t)},0)
}
```

### 由此确立的语义

| 结论 | 证据 |
|---|---|
**5 个族共用一条判分路径**：`3q5a / role / dialogue / word / repeat` → `set3q5aScore` | 上表 `case` 并列 |
`read` 族按题数分流：`exam_list.length > 1` 走共用路径，否则走单题朗读分支 | `case"collector.read"` |
**choose 族的正确答案来自模板**：`info.xtlist[].answer`，用 `xt_xh == order` 匹配 | `t.info.xtlist.find(e=>e.xt_xh===""+o.order)?.answer` |
**fill 族的答案字段名是 `choose`**（不是 answer），且 `status` 由 `(real_score\|\|score_rate)>0` 决定 | `answer:t.detail.choose,status:((t.real_score\|\|t.score_rate)??0)>0?1:0` |
choose 族拼串形态：`<entity_id>_<order>_<归一化学生答案>\x01<归一化正确答案>` | `${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}` |
fill 族送出的结构：`[{id:"<entity_id>_<order>", answer, status}]` 的 JSON 字符串 | `JSON.stringify(e)` |

---

## 3. 答案归一化 `b()` 与分隔符 `\x01`

```js
function b(e){if(!e)return"0";for(var t=e.toUpperCase(),o="",i=0;i<t.length;i++){var r=t.charCodeAt(i);
  if(r>=65&&r<=90)o+=(r-64).toString();else o+=t[i];}return o}
const q=String.fromCharCode(1);
```

**含义**：官方把**选项字母转成序号**（A→`1`、B→`2`、…、Z→`26`），多位单选/多选直接拼接
（如 `AC` → `13`）；非字母字符原样保留。分隔符是 `\x01`。

⚠️ 这一点直接决定我方提交时"选项答案"的取值形态：**官方内部用的是序号串，不是字母**。
（我方 `src/collectors.rs::submit_answer` 目前给 6/19/20 型返回**字母**——需按此核对，
见 §6 待办；同时要区分"本地判分用的内部编码"与"HTTP 载荷里的 `score_detail.choose`"两种情况。）

---

## 4. 判分与倍率：`index-5543d811.js`

```js
function scoreFormater(e,t=2){let n=Number(e)*Math.pow(10,t);return Math.round(Number(n))/Math.pow(10,t)}
function scoreFormatter_125(e,t=2){return scoreFormater(.125*Math.ceil(Number(e)/.125))}
function ConvertToPercentScore(e,t,n=0){return e&&t?scoreFormater(Number(e)/Number(t)*100,n):0}
function calculateRealScore(e,t){return scoreFormater(Number(e)/5*Number(t))}
function calculateRealScoreWithCategory(e,t,n){return EvaluationTypes.includes(n)?calculateRealScore(e,t):e}
```

```js
// scoreFilter / scoreFilterHomework / percentageQuestion 的完整定义
const SimulationColumns=[FirstColumnType.SimulationExam,FirstColumnType.ListeningSimulation,
                         FirstColumnType.ReadingWritingSimulation];
function t(t,n){ return t&&e.every(e=>e!=t) || n&&5!=n }
return {
  scoreFilter: function(e,t,n){ return [n?scoreFormater(t):ConvertToPercentScore(t,e,0), n?e:100] },
  scoreFilterHomework: function(e,n,r,o,i){
    return [t(r,o)?scoreFormater(n):ConvertToPercentScore(n,e,0), i?e:100] },
  percentageQuestion: t
};
```

**读法**：
- `scoreFilter(question_score, real_score, isPercentage)` → `[得分, 分母]`
  - 百分比题型：`得分 = scoreFormater(real_score)`，`分母 = question_score`
  - 否则：`得分 = real_score / question_score * 100`，`分母 = 100`
- **`percentageQuestion(columnType, questionType)`**（是否按百分比计分）
  `= (columnType 且 columnType ∉ 三类模拟考列) 或 (questionType 且 questionType ≠ 5)`
  → 即"**三类模拟考列且题型=5**时才是原始分，其余都是百分比制"。
  ⚠️ 我方目前**完全没有建模这个判定**（见 §6）。
- `scoreFormater` = **JS `Math.round`（.5 向上）**，不是 Python 银行家舍入 —— 我方已按此实现。

### 关键：`Math.round` 与"五位制→卷面分"

```js
function calculateRealScore(e,t){return scoreFormater(Number(e)/5*Number(t))}
```
ISE 是 5 分制：`real_score = 原始分/5 * 该题满分`，再按 `scoreFormater` 取两位。

---

## 5. `sync-v2` 字段的权威构造（比我方现有实现更精确）

```js
const i={};try{i="string"==typeof e.score_detail?JSON.parse(e.score_detail):e.score_detail}catch(e3){}
const{total_score_init:a,total_score:s}=i||{},l=a||s;
const c=await n.getColumnsNeed125(r);
c&&t?.rate_scale&&l&&o&&(e.real_score=scoreFormatter_125(o/5*l));
const u=c?t?.rate_scale:void 0;
const d=c&&u?"0.125":void 0;
i.rate_scale=u||0;
delete i.total_score_init;delete i.xml_score;delete i.question_type_score;
delete i.is_rejected;delete i.except_info;delete i.first_column_id;
e.score_detail=JSON.stringify(i);
const p=Object.assign(e,{rate_scale:u,graduation:d,client_time:currentTimestampToDate(),m…
```

| 字段 | 官方规则 | 我方状态 |
|---|---|---|
`real_score` | 仅当 **1.25 倍率列** 且有 `rate_scale` 且有 `total_score_init\|\|total_score` 时 = `scoreFormatter_125(o/5*l)` | 已实现（= `real_score_125(得分)`，等价）✓ |
`rate_scale` | `c ? t.rate_scale : undefined`；最终 `i.rate_scale = u \|\| 0` | 已实现（非 125 列发 0）✓ |
`graduation` | **仅 `c && u` 时**为字符串 `"0.125"`，否则不出现 | 已实现（条件性）✓ |
`score_detail` | 发送前删除 6 个键：`total_score_init / xml_score / question_type_score / is_rejected / except_info / first_column_id`，并写入 `rate_scale` | 已实现 ✓ |
`client_time` | `currentTimestampToDate()` | 我方用时间戳字符串（需核对格式） |

---

## 6. 由本次提取得出的**待办清单**（与我方实现对照）

1. **`percentageQuestion` 判定完全缺失**：`(columnType ∉ {模拟考,听说模拟,读写模拟}) || (questionType ≠ 5)`
   → 决定"按百分比计分"还是"按原始分"，直接影响 `score`/`score_rate`/`score_detail` 的数值。
   **需要**: 在 resolver/后端建模 `questionType == 5` 与三类模拟考列的判定。
2. **答案取值形态**：官方内部把字母归一化为序号（A→`1`）。我方 `submit_answer` 对 6/19/20 返回**字母**。
   **需要**: 确认 HTTP 载荷里 `score_detail.choose` 到底用字母还是序号（区分"本地判分编码"与"线上载荷"）。
3. **choose 族正确答案来源** = 模板 `info.xtlist[].answer`（按 `xt_xh == order` 匹配）。
   **需要**: 核对 resolver 解析模板时是否保留 `xtlist` 的 `xt_xh ↔ answer` 映射。
4. **fill 族字段名是 `choose`**（我方若发 `answer` 可能被忽略）。
5. **`collector.sentence`**（Android 派生族，如"逐句跟读"）我方无显式实现，目前靠 substring 兜底。
6. **五族共用判分路径**（3q5a/role/dialogue/word/repeat）：我方若对每族分别建模评分，可能与官方不一致。

---

## 7. 交叉验证：签名算法另有独立实现（已 100% 验证）

`head.sign` 已从原生插件静态逆向（Ghidra）并验证 150/150：
```
body_b64 = base64(JSON.stringify([{r,params}]))
sign     = md5_hex(PID + str(time) + body_b64 + SECRET_KEY)
v1: pid=grlx  key=555ff…bb8  time=秒     v2: pid=pcWin key=94cf…e82 time=毫秒 (+key_id/sign_version)
```
参考工程 `ets_get_answer/ets_client_optimized_platform.py` 用的正是 v1 的 pid/key → 独立佐证。

---

## 8. 【已用我方真实语料定案】客观题的判分与提交字段

### 8.1 官方本地判分函数（`useExamStepper-8ad3fbd7.js::G()` 原文）

```js
function X(e){const t={"‘":"'","’":"'","：":":","，":",","。":".","——":"-","—":"-","_":"-"};
  return(e=e.replace(/[‘’：，。———_]/g,e=>t[e])).toLowerCase()}
async function G(e,t,a,o){
  const{getFileJson:n}=f(),{UNZIP_LOCAL_PATH:l}=d();
  let s=[],r=(await n(p(`${l}\\${e}\\content_${t}\\content.json`))).info.xtlist;
  try{s=o.split(",")}catch(i){}
  return s.map(e=>{const[t,o,n]=e.split("_");let l="0";
    if("0"===n)return{entity_id:t,order:o,score_detail:{total_score:l,choose:"",category:a.evaluation_type}};
    {const e=r.find(e=>e.xt_xh===o);if(!e)return null;
     const s=e.xxlist.find(e=>e.xx_xh==n);
     return s?(e.answer===s.xx_mc&&(l=m(a.question_score/a.question_count)),
       {entity_id:t,order:o,score_detail:{total_score:l,choose:s.xx_mc,choose_id:n,category:a.evaluation_type}}):null}
  }).filter(Boolean)}
```

**判分规则（客观题）**：
```
正确 ⟺ 模板 info.xtlist[].answer === 所选选项的 xxlist[].xx_mc
得分 = question_score / question_count        （题型总分 ÷ 题数，经 m()）
未作答（xx_xh == "0"）→ choose: ""，total_score: "0"
```

**提交字段**：`score_detail = { total_score, choose, choose_id, category }`
- `choose` = 所选选项的 **`xx_mc`**
- `choose_id` = 所选选项的 **`xx_xh`**（选项序号）
- `category` = 该题的 **`evaluation_type`**
- 多选题：UI 传的是**逗号分隔**的 `entity_id_order_xx_xh` 串

### 8.2 真实模板证据（**真实官方内容包**，非推测）

**主要证据**（随官方客户端下发的真实内容包，收录于 `Fe-pro/ets读取/北京初中resoure/`，
现已收入语料 `tests/corpus/beijing_junior_real/choose_0{1,2,3}_stid7290{61,62,63}/content.json`）：

```json
{ "structure_type": "collector.choose",
  "info": { "stid": "729062", "xtlist": [
    { "xt_xh": "1", "answer": "B",
      "xxlist": [ {"xx_xh":"1","xx_mc":"A","xx_nr":" This Friday."},
                  {"xx_xh":"2","xx_mc":"B","xx_nr":" Next week."},
                  {"xx_xh":"3","xx_mc":"C","xx_nr":" Next month."} ] },
    { "xt_xh": "2", "answer": "C",
      "xxlist": [ {"xx_xh":"1","xx_mc":"A","xx_nr":" The food."}, … ] } ] } }
```

**Android bean 佐证**：`InfoObjBean.CodeXxBean` 的 `@SerializedName("xx_mc")`=选项名称、
`("xx_nr")`=选项内容、`("xx_xh")`=选项序号。

**解析结果核对**（我方 resolver 在真实样本上的输出）：

| 样本 | `xtlist[].answer` | 我方 `choose_id` | 应为 |
|---|---|---|---|
| choose_01 stid729062 | B / C | `2` / `3` | 正确选项的 `xx_xh` ✓ |
| choose_02 stid729063 | C / B | `3` / `2` | ✓ |
| choose_03 stid729061 | B / B | `2` / `2` | ✓ |

**辅助证据**（官方 iframe 内嵌示例数据，同样带 `xx_xh`）：`iframe_web/js/*.js` 内可见
`{"xx_xh":"1","xx_mc":"A","xx_nr":" The history of TV.","xx_wj":"aaa"}`、`"answer":"A"`。

> ⚠️ 早期版本的本节曾引用 `tests/corpus/_examples/choose_xtlist/`——该目录 `meta.json` 自标
> `"region":"synthetic"` / `"source":"手写最小样例"`，**不是官方证据**（现仅作 schema 演示）。

另一个手写 fixture `tests/corpus/_examples/choose_xtlist/mock-001/content.json`：
```json
{ "xt_xh": "1", "answer": "C",
  "xxlist": [ {"xx_mc":"A","xx_nr":"Take the bag."},
              {"xx_mc":"B","xx_nr":"Lock the door."},
              {"xx_mc":"C","xx_nr":"Leave food for the cat."} ] }
```
→ **`xx_mc` 就是选项字母**（`xx_nr` 是选项文本），`answer` 也是字母。
→ **因此：我方 `submit_answer` 对 6/19/20 返回字母是正确的**；
   PC 前端里那个 `b()`（A→"1"）只是**原生插件（`setPCChoose`）内部判分编码**，**不是线上载荷**。

填空族（`tests/corpus/_examples/std_fill/mock-001/content.json`）：
```json
"std": [ {"xth":"15", "value":"答案1"}, {"xth":"16", "value":"答案2"} ]
```
→ 填空题在 **`info.std[]`**，题号 `xth`，答案 `value`；
   且官方比较时会 **`X(value).split("/")`** —— **支持 `/` 分隔的多个可接受答案**，比较前做标点归一 + 转小写。

### 8.3 与我方实现的具体偏差（待修）

| # | 官方 | 我方 | 影响 |
|---|---|---|---|
1 | 客观题 `score_detail` 含 **`choose_id`**（= 选项序号 `xx_xh`） | `build_score_detail()` **不产出 `choose_id`** | 服务端可能据此还原"选了第几个选项"，缺失可能导致成绩/错题记录不准 |
2 | `category` = 该题 **`evaluation_type`** | 我方客观题写死 `category="choose"` | 语义不符 |
3 | 客观题得分 = **`question_score / question_count`**（精确均分） | 我方用"捏造的口语 profile.total"参与 | 客观题给分可能偏离官方 |
4 | 填空答案支持 **`/` 分隔多解** + 标点归一 + 小写比较 | 需核对 resolver 是否保留 `std[].value` 原文 | 填空题多解时会判错 |
5 | 客观题联答串用 **逗号分隔**、单题 `entity_id_order_xx_xh` | 需核对 | 多选/联答场景 |

---

## 9. 【真实 API 载荷定案】词汇族答案形态（解决 C/D 两组的冲突）

C 组（PC 前端代码）与 D 组（Android 代码）对"选项型正确答案是什么"给出**相反**结论：
C 认为 `option_name` 是选项**键**、D 认为它是选项**文本**。用我方留存的**真实服务端载荷**
`tests/fixtures/word_struct_guangdong_hs_pk.json` 直接定案：

```json
{ "question_type": 6, "question_type_name": "听音选义", "answer": "D",
  "option": "[{\"option_name\":\"A\",\"option_value\":\"n.政治，政治活动\"},
              {\"option_name\":\"B\",\"option_value\":\"vt.缺乏\"}, …]" }
{ "question_type": 16, "question_type_name": "默写词汇", "answer": "trait", "option": "" }
{ "question_type": 17, "question_type_name": "朗读词汇", "answer": "",      "option": "" }
{ "question_type": 18, "question_type_name": "听写词汇", "answer": "trait", "option": "" }
```

**结论（权威）**：
1. **`option_name` = 选项字母**（"A"/"B"/"C"/"D"），`option_value` = 选项文本（释义）。
   → D 组"`option_name` 是选项文本"的说法**不成立**；C 组"它是键"正确，且键就是**字母**。
   → 与旧套题族 `collector.choose`（`xx_mc` 字母、`xx_nr` 文本）**形态一致**，两族其实是同一套约定。
2. **`answer` / 提交值**：选项型（6/19/20）= 字母；拼写型（16/18）= 单词文本；朗读型（17）= 空串。
   → **完全印证我方 `collectors.rs::submit_answer` 的口径** ✓
3. **`question_type_name` 确实存在**（在**服务端载荷**里）：6=听音选义 / 16=默写词汇 /
   17=朗读词汇 / 18=听写词汇（D 组只看了客户端代码里的 `type_name`，故判断"不存在"——
   客户端代码里确实只有 `type_name` 且零使用，但**载荷里有**，而载荷才是我们要对齐的对象）。
4. 判分依据（Android 代码）：`optionName.equals(answer)`（`fragment/h.java:304-321`）；
   拼写/听写：`trim(answer).toLowerCase()`，**用户答案只 toLowerCase 不 trim**（不对称，注意别踩）。

> 方法论教训：**客户端代码**与**真实载荷**可能给出不同答案。凡涉及"我们该发什么"的问题，
> 以**真实载荷**为准；客户端代码用于解释**为什么**。

---

## 10. 已交付的静态规格清单（本目录）

| 文件 | 覆盖范围 |
|---|---|
`00_core_scoring_and_dispatch.md`（本文） | 族分发 switch、判分核心、sync-v2 字段、客观题判分与提交、词汇族答案定案 |
`A_listening_speaking.md`（760 行） | `read` / `repeat` / `role` / `dialogue` 四族 |
`B_reading_writing.md` | `choose` / `fill` / `picture` / `3q5a` 四族 |
`C_word_and_exam_flow.md`（140 KB） | `word` 族 + 考试流程/提交构造/125 倍率/端点清单 |
`D_android_only_families.md`（1167 行） | Android 侧族名核查（推翻 repeat_essay/repeat_dialogue）+ 18 条差异 |
`E_*` / `F_*` | iframe 应用：题目/答案字段、作答交互与桥接契约（进行中） |

**静态资料库**（已固化，不再依赖 `/tmp`）：
- `ets源码恢复/pc_client_web/index-5543d811.js` + `chunks/`（**185/193** 个 chunk）
- `ets源码恢复/iframe_web/`（6.8 MB，55 个 JS，含带中文注释的示例数据）

---

## 11. ⚠️ 未决口径差异：`rate_scale` / `graduation`（两个官方客户端自己就不一致）

| 来源 | `graduation` | `rate_scale` |
|---|---|---|
**官方 PC 端**（`index-5543d811.js::requestAnswerLogs` 原文） | **条件字符串** `"0.125"`，仅 1.25 倍率列且 `rate_scale` 存在时才给 | `c ? t.rate_scale : undefined`，最终 `u \|\| 0`（非 125 列 = 0） |
**官方移动端**（`ReadFollowAnswerBean.java:28`） | `private int graduation`（**整数**） | 需另查 |
**我方线上（旧路径）** | 无条件数字 `0.125` | 恒为 `1` |
**我方官方模式（`AUTO_COMPLETE_OFFICIAL_PAYLOAD=1`）** | 条件字符串 `"0.125"` ✓ 同 PC | 非 125 列 = 0 ✓ 同 PC |

**处置**：**不动线上旧路径**（服务端长期实测可用、回读官方 point 能对上），把它作为已知口径差异
就地注释；要切 PC 口径时打开 flag。**待验证项**：服务端是否真的在意这两个字段
（需要一次"仅改这两字段"的 A/B 提交才能定论）。

> 附带教训（已落测试）：`runner.py` 的官方/旧两条分支**必须保持关键字参数兼容**——
> 曾因只给 `official_payload.build_score_detail` 加 `choose_id` 导致默认分支逐题 `TypeError`，
> 而 runner 路径测试依赖数据库、CI 跑不全。现加 `test_both_score_detail_builders_accept_the_same_kwargs`
> 用签名一致性兜住这类问题。

---

## 12. 客观题与朗读类的 `score_detail` 是**两套形态**（已按官方收紧）

| 题型 | 官方 `score_detail` 键 | 证据 |
|---|---|---|
**客观题**（choose / fill） | `{total_score, choose, choose_id, category}`（**仅 4 个**） | `useExamStepper::G()`：`score_detail:{ total_score:l, choose:s.xx_mc, choose_id:n, category:a.evaluation_type }`；未作答 `{total_score:"0", choose:"", category}`（无 `choose_id`） |
**朗读/口语类** | `{total_score, accuracy/fluency/integrity/standard_score, …}` + 引擎写入的 `real_score` | `useRecordResult`；C 组 §"朗读型没有正确答案，只有评测分" |

**我方此前的问题**：客观题复用了朗读类的构造，于是 `score_detail` 里塞进了
`fluency_score: q_score*0.99 / accuracy_score: q_score*0.98 / …` 这类**捏造维度分**
（客观题不存在"流畅度/准确度"）。现已改为 `build_objective_score_detail()`，
客观题只发官方那 4 个键。

### 逐题得分的官方规则（客观题）
```js
e.answer === s.xx_mc && (l = m(a.question_score / a.question_count))   // 答对 = 大题分 ÷ 题数
```
即**答对就给满该题小分**（不是按口语 profile 折算）——我方客观题 `profile.total = q_score` 与之等价 ✓。

---

## 13. 全语料"丢题"扫描结果（27 例真实包，修完 repeat 后）

对每个真实内容包，比较"自然题数"（`info.fj` / `info.question` / `info.xtlist` / `info.std` / `info.sublist`）
与解析题数，结果：

- **无丢题**：除已修的 `collector.repeat`（1 → 10/12 题）外，其余全部一一对应。
- 3 处"看似不符"经核实**不是 bug**：`collector.picture` 的 `info.std` 是**多篇参考范文**
  （1 题 + N 条 `answers`，官方只渲染前 3 篇），并非多题。

| 族 | 自然题数来源 | 解析题数 | 结论 |
|---|---|---|---|
`collector.repeat` | `info.fj`（10 / 12） | 10 / 12 | 已修（原先 1） |
`collector.choose` | `info.xtlist` | 一致 | ✓ |
`collector.fill` | `info.std` | 一致 | ✓ |
`collector.role` / `3q5a` | `info.question`（1~8） | 一致 | ✓ |
`collector.picture` | `info.std`＝范文数（1~5） | 1 题 + N answers | ✓ 不是丢题 |
`collector.read` / `word` | 单值 `info.value` | 1 | ✓ |

---

## 14. 仍未解的两项（证据不足，先记录不猜）

1. **`{{thN}}` / `{{smN}}` 占位符**：官方用 `exam_info` 的 `{id,value}` 列表对**整个 content JSON**
   做全局字符串替换（iframe 原文：`e.exam_info.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value), e)`）。
   但**我们的语料里没有任何 `exam_info`**，也没有 `{{th…}}` 实例 → 无法验证。
   469 份官方模板里只有 `{{st1}}…{{st7}}`（挂载键，我方已正确处理）。
   → 什么时候拿到带 `exam_info` 的真实包，再照官方规则实现。
2. **`percentageQuestion`**（百分比制 vs 绝对分展示）：
   `isAbsolute = (first_column_id != ListeningSpeakingSynchronous)`；错题本用
   `![listeningSync,textReading,textShadowing].includes(entity_type)`。
   `scoreFilter(rawTotal, score, isAbsolute)` → `[isAbsolute?scoreFormater(score):ConvertToPercentScore(score,rawTotal), isAbsolute?rawTotal:100]`。
   它影响**展示/分母**，对"我们该提交什么"的直接影响未验证 → 先记录。

---

## 15. 词汇族线路完整核对（真实载荷 + 官方前端逐字代码）

### 15.1 真实载荷形态（`tests/fixtures/word_struct_guangdong_hs_pk.json`：3 词 × 6 题型）

```json
{ "question_type": 6,  "question_type_name": "听音选义", "answer": "D",
  "option": "[{\"option_name\":\"A\",\"option_value\":\"n.政治，政治活动\"}, …]" }
{ "question_type": 16, "question_type_name": "默写词汇", "answer": "trait", "option": "" }
{ "question_type": 17, "question_type_name": "朗读词汇", "answer": "",      "option": "" }
{ "question_type": 18, "question_type_name": "听写词汇", "answer": "trait", "option": "" }
{ "question_type": 19, "question_type_name": "英译中",   "answer": "B",     "option": "[{option_name:A, option_value:'n.燃料'}, …]" }
{ "question_type": 20, "question_type_name": "中译英",   "answer": "A",     "option": "[{option_name:A, option_value:'trait'}, …]" }
```
- **选项型（6/19/20）**：`option_name` = **选项字母**、`option_value` = 展示文本；**answer 是字母**。
- **拼写型（16/18）**：answer = **单词原文**。
- **朗读型（17）**：answer **为空**（分数来自引擎评测）。

我方解析输出核对（`parse_word_pk_value`，3 词 × 6 题型 = 18 题）：
`word.choice→"D"`、`word.write→"trait"`、`word.read→原文（官方为空，供展示）`、
`word.dictation→"trait"`、`word.translate_en_cn→中文释义（官方为字母）`、
`word.translate_cn_en→英文单词（官方为字母）` —— `answers` 给"给人看的可读答案"，
**上交值另由 `submit_answer` 给官方字母** ✓（这一分工是既定设计）。

### 15.2 两个端点的分流（**两个官方客户端一致**）

```js
// PC 前端 index-5543d811.js
requestWordHomeworkSubmit = e => {
  if (1 == this.practiceType) {                      // 作业
    const t = this.getSeconds();
    return this.homeworkPractice.requestHomeworkWordCommit({...e, use_time: t});
  }
  return requestPKSubmitWord(e);                     // PK：原样 POST g/pk/submit-word
};
```
- **词汇作业 → `POST g/homework/score-sync`**（Android 侧同结论：`SyncWordRepository.java:1255-1280`）
- **词汇 PK → `POST g/pk/submit-word`**

两种端点 `detail[]` 形态**不同**：

| 端点 | 逐题明细 |
|---|---|
`g/pk/submit-word` | `{word_id, question_id, point: 10000\|0, answer: option_name}`（官方 PK.vue：`ae.push(Ee(answer==option_name ? 1e4 : 0, option_name))`） |
`g/homework/score-sync` | `{word_id, question_id, [item_id, question_item_id], score_rate: point>0?1:0, score_detail: JSON.stringify({choose: answer, score: point>0?100:0}), client_time}` + 信封 `{homework_id, resource_id, type, use_time, mark_service:1, seq_id}` |

**我方核对结果**：
- PK 明细 `{point: 10000|0, answer}` **与官方完全一致** ✓（此前的 `10000` 修复正确）
- 但**词汇作业此前也走 PK 端点**（且多调了 `start-word-pk`）—— 与官方分流不符。
  已按官方实现 `g/homework/score-sync` 路径（构造器 + 客户端 + runner 分支），
  由环境变量 `ETS_WORD_HOMEWORK_SCORE_SYNC` 控制，**默认关闭**（不动线上行为，待 A/B）。
