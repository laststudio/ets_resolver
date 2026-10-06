# E. iframe web 应用（明文 JS）中的题型数据结构与标准答案字段

分析对象（纯静态只读，未运行客户端 / 未抓包 / 未联网）：

- iframe 应用根目录：`/home/shuaiqiu/Projects/ets源码恢复/iframe_web/`（276 个文件，6.8MB）
- 交叉参照：`/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/`（`index-5543d811.js` + `chunks/`）
- 我方实现：`/home/shuaiqiu/Projects/ets_resolver/src/collectors.rs`、`src/spec.rs`

## 0. iframe 与「题目页 / 答案页」的职责边界（前置事实）

iframe 侧只有两个页面承载听力口语套题：

| 文件 | `<title>` | 角色 |
|---|---|---|
| `pc-listen.html` | `听说题目`（`pc-listen.html:6`） | 做题页（**不显示标准答案**，除 `content_fill_answer_enable` 回显模式） |
| `pc-listen-answer.html` | `听说答案`（`pc-listen-answer.html:6`） | **答案页**，按 `structure_type` 分块渲染参考答案 |
| `pc-listen-warn-result.html` | `通知答案截止后公布页面` | 作业未截止：`<div class="tell-result">答案将在作业截止后公布</div>` |

数据入口（原生 → JS，均为全局函数，非 `window.` 前缀赋值）：

```
// iframe_web/js/pc-listen-default.js:20
function setData(set,page,set_id,order,type) {
	//试题 文件路径 试题id 要显示的order
	//order 示例 1,2/2,3
	//type 0 普通 1 模考
```

```js
// iframe_web/js/pc-listen-default.js:104
	window.showData = set;
	if(vue){
		vue.set = set;
		vue.page = JSON.parse(page);
		vue.set_id = set_id;
		vue.order = order;
	}
```

- `set` = `[{structure_type, info}, ...]`
- `page` = 素材目录绝对路径**数组**（`JSON.parse(page)`，模板里用法 `:src="page+set.choose_con.st_pic"`（`pc-listen.html:159`）、`:src="page[theIndex]+one.info.image"`（`pc-listen-answer.html:124`））
- 原生侧构造这两个参数的位置（PC bundle）：
  `chunks/useExamStepper-8ad3fbd7.js`：`a.setData(s,l,e,"",n,1==v)`，其中 `l=JSON.stringify([p(\`${z}\\${...}\\content_${fe.value?.exam?.real_id}\\material\\\`)])`

JS → 原生（`window.*`，全量枚举自 iframe 的 `js/`，排除 jquery/vue/echarts）：

```
PlayRecord / StopRecord / PlayWord / StopWord / PlayAnswerVideo   (pc-listen-show-wordinfo.js, PC-choose.js, pc-listen-choose.js)
ReturnChoose            (pc-listen-choose.js:88, PC-choose.js:45)
returnPcBlank            (pc-listen-fn.js:808)
kttb_ReturnChoose / kttb_returnPcBlank   (pc-listen-fn.js)
ck_ReturnChoose          (pc-view-kttb5.js)
```

**关键结论（本任务问题二的前置）**：iframe 的 HTML 里**从未读取** `info.xtlist[].answer`（`grep -rn "\.answer" iframe_web/` 的 21 处命中全部是 `js/fn.js` / `js/pc-listen-fn.js` 里**原生回传值**的 `item.answer` / `ans_arr[0].answer`，以及 read-write 相关的 `js/read-write-answer.js`）。答案作答对错由原生把「正确选项序号」通过 `setPCChoose()` / `kttb_setPcChoise()` 灌进来，或由 PC bundle 自己读内容包判分。

---

## 一、每族「题目 / 答案字段表」

数据来源三条：
1. **示例数据**（最有价值）：`iframe_web/js/pc-listen-data.js`，第 59 行有官方自己的家族编号注释：
   ```js
   //A1 模仿朗读 A2 三问五答 A3 故事复述 A4 选择题 A5 填空题 A6 跟读与模仿 A7 朗读句子
   var A1 = [{"structure_type":"collector.read","info":{...}}];
   var A2 = [{"structure_type":"collector.3q5a",...}];
   var A3 = [{"structure_type":"collector.picture",...}];
   var A4 = [{"structure_type":"collector.choose",...}];
   var A5 = [{"structure_type":"collector.fill",...}];
   var A6 = [{"structure_type":"collector.repeat",...}];
   var A7 = [{"structure_type":"collector.word",...}];
   ```
   （`collector.role` / `collector.dialogue` **无示例数据**，见各族说明。）
2. **答案页渲染表达式**：`pc-listen-answer.html`（字段的官方消费者）。
3. **PC bundle 判分/桥接代码**：`chunks/useExamStepper-8ad3fbd7.js`、`chunks/useExamResult-d9fb4671.js`、`chunks/Practice-1694d88a.js`。

字段归三类：
- **[答]** = 标准答案（判分依据）
- **[范]** = 示范 / 参考（供对照，非判分依据）
- **[UI]** = 仅界面 / 素材

### 1. `collector.read` 模仿朗读

一题 = `set` 数组的一个元素（多小题时 `set.length>1`，答案页用 `:class="{'height_100_percent': set.length == 1 }"` 与 `v-if="set.length>1"` 切换「小题」视图）。**没有子数组容器**，字段直接挂在 `info` 上。

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `info.stid` | 题目实体 id（DOM id / 提交 entity_id） | [UI] | `pc-listen-answer.html:36` `:id="set_id+'_'+one.info.stid+'_'+1"` |
| `info.value` | **朗读原文**（HTML 片段，`</p><p>` 分隔） | **[答]** | A1：`"value":"</p><p>At this time of year, there are still only five hours of daylight. ..."`；答案页 `<p class="original" v-html='one.info.value'>`（`pc-listen-answer.html:37`） |
| `info.audio` | 原文标准音（`content.mp3`） | [UI] | A1：`"audio":"content.mp3"` |
| `info.video` | 视频素材 | [UI] | A1：`"video":"content.mp4"` |
| `info.image` | 图片素材 | [UI] | A1：`"image":""` |
| `info.symbol` | 原文的**逐词标注版**（与 value 同文本，含空格差异） | 未找到消费者 | A1：`"symbol":"</p><p>At this time of year, ..."`；答案页**已注释**：`pc-listen-answer.html:41` `<!-- <p v-html="one.info.symbol"></p> -->` |
| `info.ai` | value 的**断句版**（在每行尾插入 `</br>`，行宽约 43 字符） | 未找到消费者 | A1：`"ai":"</p><p>At this time of year, there are still only five hours of </br>daylight. ..."`；`grep -rn "\.ai\b" iframe_web/` → 0 命中 |
| `info.videotime` | **带时间戳文本**：`"<00:00><00:01> At this time of year, </br>..."` | 未找到消费者 | 全仓 `grep -rn videotime` 只命中示例数据本身 |
| `info.translate` | 译文 | [UI]（read 页面未绑定） | A1：`"translate":""` |
| `info.analyze` | 解析 | [范] | A1：`"analyze":""` |
| `info.xh` | 小题号 | **read 无此字段** | A1 无 `xh`；小题号取数组下标 `theIndex+1` |
| `new_xh` | 运行时注入的显示号（非内容包字段） | [UI] | `pc-listen-default.js` 注入，答案页 `i.new_xh \|\| i.xh` |

### 2. `collector.repeat` 跟读与模仿

一题 = 一个 `info`，但**小题 = `info.fj[]` 一项**（分句）。

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `info.value` | 整段原文 | [范] | A6 |
| `info.fj[]` | **分句数组（小题容器）** | [答] | 答案页 `pc-listen-answer.html:484` `<div class="mb45 q3a5" v-for='i in one.info.fj'>` |
| `fj[].value` | 分句原文 | **[答]** | A6：`{"value":"One day in ancient Greece, King Hiero asked a crown maker to make him a golden crown.","xh":"1",...}`；答案页 `pc-listen-answer.html:507` `<p v-html='i.value'>` 在「分句N」标题下 |
| `fj[].xh` | 分句号 = **提交 order** | [UI]/判分键 | `chunks/useExamResult-d9fb4671.js`：`case"collector.repeat":for(const n of r.info.fj)e=\`${D}_${r.info.stid}_${n.xh}\`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh)` |
| `fj[].audio` | 该句标准音（`fj1.mp3`…`fj5.mp3`），UI 标签「原文试听」 | [范] | A6；答案页 `:id="set_id+'_'+one.info.stid+i.audio"` |
| `fj[].ai` | 与 `value` 同值 | 未找到消费者 | A6 |
| `fj[].fs` | 数值字符串 `"0.8"` | **未找到语义**（两端 0 消费者） | A6：`"fs":"0.8"`；`grep -roE "\.fs\b" iframe_web/ pc_client_web/` 无命中（PC 侧 2 处 `\.fs\b` 属压缩后无关标识符） |
| `fj[].time` | 数值秒（`"11"`/`"7"`/`"8"`），与分句长度相关 | **未找到语义**（两端 0 消费者） | A6：`"time":"11"` |
| `info.symbol` / `info.ai` / `info.translate` / `info.analyze` / `info.image` / `info.video` / `info.audio` | 同 read | [UI] | A6：`"audio":"content.mp3"` |
| `info.stid` | 实体 id | [UI] | - |

### 3. `collector.word` 朗读句子 / 单词

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `info.xh` | 小题号 = **提交 order** | [UI]/判分键 | A7：`"xh":"1"`；`chunks/useExamResult-d9fb4671.js`：`case"collector.word":e=\`${D}_${r.info.stid}_${r.info.xh}\`` |
| `info.value` | 原文（**带 `ets_thN` 前缀**） | **[答]** | A7：`"value":"ets_th1 He is lucky to have such nice parents who always support him in his decisions."`；答案页 `pc-listen-answer.html:580` `<p v-html='one.info.value'>` |
| `info.value_bz` | 与 `value` 同值 | **未找到语义**（两端 0 消费者） | A7：`"value_bz":"ets_th1 He is lucky..."`；`grep -ro value_bz pc_client_web/ iframe_web/` → 仅示例数据 |
| `info.value_pf` | 与 `value` 同值 | **未找到语义**（两端 0 消费者） | A7：`"value_pf":"ets_th1 He is lucky..."` |
| `info.read_title` | 朗读提示标题（音标/字母组合） | [UI] | A7：`"read_title":""` |
| `info.image` | `"N"`（非 URL，疑为开关） | [UI] | A7：`"image":"N"` |
| `info.mpg` | 空字符串 | **未找到语义** | A7：`"mpg":""` |
| `info.translate` / `info.analyze` | 同族通用 | [范] | A7 |
| `new_xh` | 运行时注入显示号（多小题同 `stid` 时按序重编号） | [UI] | `pc-listen-default.js:29-49`：`for(var i in set){ if(... 'collector.word'){ if(stid==""){stid=set[i].info.stid;}else{if(stid!=set[i].info.stid){same=0;}}}} if(same==0){ ... set[i].info.new_xh = index; index++; }` |

### 4. `collector.3q5a` / `collector.role` / `collector.dialogue`

**三族共用同一个渲染块**（`pc-listen-answer.html:105`）：

```html
<div class="one" v-if='one.structure_type=="collector.3q5a"||one.structure_type=="collector.role"||one.structure_type=="collector.dialogue"'>
```

- `collector.3q5a` 有官方示例数据 **A2**；
- `collector.role` / `collector.dialogue` 在整个 iframe 目录里**各只出现 1 次**（就是上面这行 `v-if`），**无示例数据、无独立字段处理分支**：

```
$ grep -rn --include=*.js --include=*.html -oE "collector\.[a-z0-9_]+" iframe_web/ | awk -F: '{print $3}' | sort | uniq -c
      4 collector.word
      3 collector.choose
      2 collector.repeat
      2 collector.read
      2 collector.picture
      2 collector.fill
      2 collector.3q5a
      1 collector.role
      1 collector.dialogue
```

因此 role / dialogue 的字段结构**按同块推定 = 与 3q5a 的 `info.question[]` 完全一致**（这是渲染代码唯一依据；**`collector.role`/`collector.dialogue` 的示例内容包未找到**）。

一题 = 一个 `info`；**小题 = `info.question[]` 一项**。

| 字段 | 含义 | 类别 | 证据（A2 的 `xh=1` / `xh=4` 两条对照） |
|---|---|---|---|
| `info.question[]` | 小题容器 | — | `pc-listen-answer.html:132` `v-for='i in one.info.question'` |
| `q.xh` | 小题号 = **提交 order** | [UI]/判分键 | `chunks/useExamResult-d9fb4671.js`：`case"collector.3q5a":case"collector.role":case"collector.dialogue":for(const n of r.info.question)e=\`${D}_${r.info.stid}_${n.xh}\`` |
| `q.new_xh` | 运行时显示号 | [UI] | `pc-listen-answer.html:127` `v-text='i.new_xh \|\| i.xh'` |
| `q.std[]` | **参考答案数组**（`{value, ai, audio}`） | **[答]** | 答案页 `pc-listen-answer.html:149-155`：`<p>参考答案:</p><p v-for='s in i.std' v-if='$index<3'><span v-html="'● '+s.value">` |
| `std[].value` | 标准答案文本（角色 a 时是「标准问句」，角色 b 时是「标准答句」） | **[答]** | A2 xh=1：`{"value":"Did you graduate from Cambridge?","ai":"Did you graduate from Cambridge?","audio":""}`；A2 xh=4：`{"value":"She is a supermodel.",...}` |
| `std[].ai` | 与 `value` 同值 | 未找到消费者 | A2；`grep -rn "\.ai\b" iframe_web/` → 0 |
| `std[].audio` | 该参考答句的标准音（样本全为 `""`） | [范] | 答案页 `pc-listen-answer.html:152` `:id="set_id+'_'+one.info.stid+s.audio" v-show="s.audio!=''"` |
| `q.answer` | **示范回答（model answer）**，UI 标签就是「回答:」 | **[范]** | `pc-listen-answer.html:157-166`：`<div class="hd mb20" v-show="i.answer!=''"><p>回答:</p>...<p v-html='i.answer'>`；A2 xh=1 `"answer":"Yes. I graduated from King’s College. So did Hsu Chih-mo. ..."`，xh=4 `"answer":""` |
| `q.aswaudio` | **示范回答音频** | **[范]** | A2 xh=1：`"aswaudio":"ques1aswaudio.mp3"`，xh=4 `""` |
| `q.ask` | 题干（角色 a = 中文提示；角色 b = 英文问题） | [UI] | A2 xh=1 `"ask":"你是剑桥大学毕业的吗？"`；xh=4 `"ask":"What does Lily Cole do?"` |
| `q.askaudio` | **问题音频** | [UI]/[范] | A2 xh=1 `""`；xh=4 `"askaudio":"ques4askaudio.mp3"` |
| `q.askimg` | 问题配图 | [UI] | 答案页 `pc-listen-answer.html:143` `<img v-show="i.askimg" :src="page[theIndex]+i.askimg">`（**A2 样本无此字段**） |
| `q.keywords` | **`\|` 分隔的判分关键词** | 判分辅助（客户端 0 消费者） | A2 xh=1 `"keywords":"did graduate Cambridge\|are graduate Cambridge\|were graduate Cambridge"` |
| `q.role` | 角色：`"a"`（学生发问）/`"b"`（学生作答） | 判分辅助（客户端 0 消费者） | A2：xh1-3 `"role":"a"`，xh4-8 `"role":"b"` |
| `q.analyze` | 解析，**答案页是 ACTIVE 展示项**（非注释） | [范] | `pc-listen-answer.html:135-137` `<div class="mb20" v-show="i.analyze!=''"><p v-html='i.analyze'></p></div>`；A2 全为 `""` |
| `q.sucai` | 素材标记，样本全 `""` | **未找到语义**（两端 0 消费者） | A2；`grep -rn "sucai" iframe_web/ pc_client_web/` → 仅示例数据 |
| `q.ref[]` | 空数组 | **未找到语义**（两端 0 消费者） | A2 每题 `"ref":[]`；`grep -roE "\.ref\b"` → 0 |
| `info.value` | 整段听力原文（对话/独白） | [范] | A2；答案页 `pc-listen-answer.html:130` `<p class="original noIndent" v-html='one.info.value'>` |
| `info.audio` / `info.image` / `info.video` | 原文录音 / 配图 / 视频 | [UI] | A2：`"audio":""`、`"image":""`、`"video":"content.mp4"` |
| `info.stid` | 实体 id | [UI] | `pc-listen-answer.html:118` |

> 注：答案页对 `std` 用 `v-if='$index<3'` **只显示前 3 条**，但数据里 A2 xh=5/6/7 等有 8~16 条 —— 展示截断，不是数据截断。

### 5. `collector.picture` 故事复述

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `info.topic` | 故事标题 | [UI] | A3：`"topic":"A King and His Servant"`；答案页 `pc-listen-answer.html:230` `<p align="center" v-html='one.info.topic'>` |
| `info.value` | 听力短文原文 | [范] | A3；答案页 `:231` `<p class="original noIndent" v-html='one.info.value'>` |
| `info.std[]` | **参考答案（3 篇范文）** | **[答]** | 答案页 `:245-250`：`<p class="bold">参考答案</p><p v-for='i in one.info.std' v-if='$index<3'><span v-html="'● '+i.value">` |
| `std[].value` | 范文全文（**含 `<\/p><p>` 转义 HTML**） | **[答]** | A3：`{"value":"<\/p><p>There was once a king who was very proud of his boots. ...","ai":"...","audio":""}` |
| `std[].ai` | 与 `value` 同值 | 未找到消费者 | A3 |
| `std[].audio` | 范文音频（样本 `""`） | [范] | 答案页 `:248` |
| `info.keypoint` | 叙述要点（编号列表，`</br>` 分隔） | **[范]**（答案页**已注释**） | A3：`"keypoint":"1. There was once a king who was prouder of his boots than of anything else that he wore. </br>2. ..."`；`pc-listen-answer.html:239-244` 整块被 `<!-- -->` 包住，标题是 `叙述要点` |
| `info.analyze` | **同一份「叙述要点」列表**（带 `叙述要点：` 前缀） | **[范]**（答案页**已注释**） | A3：`"analyze":"叙述要点：</br>1. There was once a king who was very proud of his boots.</br>2. ..."`；`pc-listen-answer.html:234-238` 注释块，标题是 `提示句` |
| `info.image` / `info.audio` | 配图（`content.jpg`）/ 原文录音 | [UI] | A3：`"image":"content.jpg"`,`"audio":"content.mp3"` |
| `info.ref[]` | 空数组 | 未找到语义 | A3 |

### 6. `collector.choose` 选择题（**客户端本地判分**）

一题 = 一个 `info`（一个大题，`stid` 一个大题共享）；**小题 = `xtlist[]`**；**选项 = `xtlist[].xxlist[]`**。

```json
{"structure_type":"collector.choose","info":{"stid":"115852","st_sm":"{{sm1}}","st_nr":"W: Hi, John. Would you like to attend a lecture with me this Friday?</br>M: Yes, I’d love to. ...","audio":"content.mp3","st_pic":"","xtlist":[{"xt_xh":"1","xt_nr":"{{th1}} What is the lecture about?","xt_wj":"","xt_value":"","xt_pic":"","answer":"A","xxlist":[{"xx_xh":"1","xx_mc":"A","xx_nr":" The history of TV.","xx_wj":""},{"xx_xh":"2","xx_mc":"B","xx_nr":" The history of computer.","xx_wj":""},{"xx_xh":"3","xx_mc":"C","xx_nr":" The history of telephone.","xx_wj":""}]},{"xt_xh":"2","xt_nr":"{{th2}} What time will the lecture begin?","xt_wj":"","xt_value":"","xt_pic":"","answer":"B",...}]}}
```
（`iframe_web/js/pc-listen-data.js:67`，A4）

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `info.stid` | 大题实体 id | [UI] | `pc-listen.html:167` `:id="set.choose_con.stid+'_'+j.xt_xh"` |
| `info.st_sm` | 大题说明（含 `{{smN}}` 占位符） | [UI] | `pc-listen.html:158` `<div class="description" v-html="set.choose_con.st_sm">` |
| `info.st_nr` | 对话/独白原文 | **[范]** | `pc-listen.html:194` / 答案页 `:322` `v-html='one.info.st_nr'` |
| `info.audio` | 大题录音（`content.mp3`） | [UI] | `pc-listen.html:196` |
| `info.st_pic` | 大题配图 | [UI] | `pc-listen.html:159` |
| `xtlist[].xt_xh` | 小题号（DOM id 尾段 / order） | [UI]/判分键 | `pc-listen-answer.html:326` `v-show='orderFormat(i.xt_xh,theIndex)'` |
| `xtlist[].xt_nr` | 小题题干（含 `{{thN}}` 占位符） | [UI] | A4：`"{{th1}} What is the lecture about?"` |
| **`xtlist[].answer`** | **标准答案 = 选项字母** | **[答]** | A4：`"answer":"A"` / `"answer":"B"`；判分见下 |
| `xtlist[].xt_value` | **小题「录音原文」**（答案页唯一额外展示的信息） | **[范]** | A4 第二题：`"xt_value":"W: What would you like to do, Walter?</br>M: Oh, I want to buy some food for lunch."`；答案页 `pc-listen-answer.html:339-345` `<div v-show="i.xt_value!=''">录音原文:</br><span v-html='i.xt_value'>` |
| `xtlist[].xt_wj` | 小题音频文件名（`content_1.mp3`） | [UI] | A4 第二题：`"xt_wj":"content_1.mp3"` |
| `xtlist[].xt_pic` | 小题配图 | [UI] | `pc-listen-answer.html:352` |
| `xxlist[].xx_xh` | 选项序号 `"1"/"2"/"3"` = DOM id 尾段 = `choose_id` | [UI]/判分键 | A4；PC bundle：`score_detail:{...,choose_id:n}` |
| `xxlist[].xx_mc` | 选项字母 `"A"/"B"/"C"` | 判分键 | A4 |
| `xxlist[].xx_nr` | 选项文本 | [UI] | A4：`" The history of TV."` |
| `xxlist[].xx_wj` | 选项图片文件名（`content_1_1.jpg`） | [UI] | A4 第二题；是否有图由 `isChooseImg`（`pc-listen.js:37`）判定：`if(val.xxlist[0].xx_wj!='')` |

### 7. `collector.fill` 填空题（**客户端本地判分**）

```json
{"structure_type":"collector.fill","info":{"stid":"115862","value":"</p><p>The wheel may be the most important invention of all time. ...","image":"content.jpg","audio":"content.mp3","analyze":"","std":[{"xth":"1","th":"1","value":"middle","ai":"middle","audio":""},{"xth":"2","th":"2","value":"Asia",...},{"xth":"4","th":"4","value":"19th/nineteenth/19<sup>th</sup>","ai":"19th/nineteenth/19<sup>th</sup>","audio":""},...],"ref":[],"topic":""}}
```
（`iframe_web/js/pc-listen-data.js:69`，A5）

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `info.stid` | 实体 id | [UI]/判分键 | 答案页输入框 `:id='one.info.stid+"_"+i.xth'` |
| `info.value` | 短文原文（挖空处需自行按空号定位） | [范] | 答案页 `pc-listen-answer.html:399` `<p v-html='one.info.value'>` |
| `info.image` / `info.audio` | 配图 / 原文录音（UI 标签「原文录音」） | [UI]/[范] | 答案页 `:389-394` |
| `info.analyze` | 解析 | [范] | A5：`""` |
| `info.topic` | 空 | 未找到语义 | A5：`"topic":""` |
| `info.ref[]` | 空数组 | 未找到语义 | A5 |
| `info.std[]` | **答案数组（=小题容器）** | **[答]** | 答案页 `pc-listen-answer.html:403` `<span v-for='i in one.info.std'>`（每个空一个 input）；`:411` `<p v-for='i in one.info.std'><span v-html="'● '+i.th+'.'+i.value">` |
| `std[].xth` | 空序号 = **input id 后缀 / 提交 order** | [UI]/判分键 | A5：`"xth":"1"`；答案页 `:405` `:id='one.info.stid+"_"+i.xth'`；PC bundle fill 判分：`const[t,a]=e.id.split("_");...r.find(e=>e.xth===a)` |
| `std[].th` | 显示题号（可带 `.`） | [UI] | A5：`"th":"1"`；**另一处官方代码用 `th` 做键**：`chunks/Practice-1694d88a.js` `info.std.find(e=>e.th===l[1]).value` |
| **`std[].value`** | **标准答案，`/` 分隔多解** | **[答]** | A5：`"value":"19th/nineteenth/19<sup>th</sup>"`；判分见下 |
| `std[].ai` | 与 `value` 同值 | 未找到消费者 | A5 |
| `std[].audio` | 空（预留标准音） | [范] | A5 |

同一族还有一条**独立**的「听说同步填空题」通道（PC bundle 注入的非 `info` 字段，值是 **JSON 字符串**，`pc-listen-default.js:60-68` 里 `JSON.parse`）：

| 字段 | 含义 | 类别 | 证据 |
|---|---|---|---|
| `content_fill_answer` | `[{stid, xth, answer, th}]` | **[答]** | `pc-listen-data.js:19`：`content_fill_answer: '[{"stid":"115862","xth":"1","answer":"middle","th":"1."},{"stid":"115862","xth":"2","answer":"Asia","th":"2."},...]'`；`pc-listen.html:298-304` `v-for='j in set.content_fill_answer'`，`:id='j.stid+"_"+j.xth'`、`:name='j.stid+"_"+j.th'` |
| `content_fill_answer_enable` | 同上结构，**不做题时回显答案** | **[答]**（可被原生覆盖） | `pc-listen.html:308-313`：`v-if="ifThis(set.content_fill_answer_enable) && set.show_topic_blank==1"`，`<input ... disabled="disabled" :value="j.answer"/>` |
| `show_topic_blank` | 开关，`"1"` 时进入回显模式 | [UI] | `chunks/useExamStepper-8ad3fbd7.js`：PC 侧 `s.show_topic_blank="1"` |
| `content_choose` / `content_choose2` | 选择题的木模板注入形态（同上，值是 JSON 字符串，内含 `answer`） | **[答]** | `pc-listen-data.js:12`：`content_choose: '{\"stid\":\"115852\",...,\"answer\":\"A\",\"xxlist\":[{\"xx_xh\":\"1\",\"xx_mc\":\"A\",...`；`pc-listen.html:244-286` 渲染 |

### 8. 家族清单核对（iframe 侧）

iframe 的 `pc-listen-answer.html` 只按 8 个 `structure_type` 值分支渲染：

```
collector.read                                      (pc-listen-answer.html:15)
collector.3q5a | collector.role | collector.dialogue (pc-listen-answer.html:105)
collector.picture                                   (pc-listen-answer.html:212)
collector.choose                                    (pc-listen-answer.html:288)
collector.fill                                      (pc-listen-answer.html:372)
collector.repeat                                    (pc-listen-answer.html:437)
collector.word                                      (pc-listen-answer.html:522)
```

→ **共 9 个族**（role/dialogue 共用），与 PC bundle 的 9 族一致；
→ `collector.repeat_essay` / `collector.repeat_dialogue`：`grep -rn "repeat_essay\|repeat_dialogue"` 在 **iframe 与 PC bundle 均 0 命中**（`iframe_web/js/*` 无、`pc_client_web/index-5543d811.js` + `chunks/*` 无）。

---

## 二、各族标准答案：存在不存在、存在哪

| 族 | 客户端可得 | 答案字段 | 判分位置 | 证据 |
|---|---|---|---|---|
| `collector.read` | **存在** | `info.value`（原文即答案） | 服务端/评测引擎 | 答案页 `v-html='one.info.value'`；PC bundle 走 `detail_file` + `Score_Content/<textName>.txt`：`chunks/index-4d5eea96.js` `if(["read_sentence","read_word","read_chapter"].includes(y.category)){...let l=\`${c}/${y.pageId}/Score_Content/${y.textName}.txt\`;...r=await f(y.xmlPath);const t=await k(e,r,y.category,100);N.showEssay_2.apply(null,t)}` |
| `collector.repeat` | **存在** | `info.fj[].value`（逐句） | 服务端/评测引擎 | 答案页 `pc-listen-answer.html:507` `v-html='i.value'`；PC bundle 逐句遍历 `info.fj[].xh` 生成录音 id |
| `collector.word` | **存在** | `info.value` | 服务端/评测引擎 | 答案页 `:580`；PC bundle `case"collector.word"` 只取 id |
| `collector.3q5a` | **存在** | `info.question[].std[].value`（参考答案）+ `.answer`（示范回答） | 服务端（关键词/AI 判分） | 答案页 `:149-166` 同时渲染 `std` 与 `answer`；PC bundle **只读 `info.question[].xh`** 用于录音 id，从不读 `std`/`answer` → 判分在服务端 |
| `collector.role` | **存在（同 3q5a 结构）** | 同上 | 服务端 | 渲染块共用；族名只在 iframe 出现 1 次 |
| `collector.dialogue` | **存在（同 3q5a 结构）** | 同上 | 服务端 | 同上 |
| `collector.picture` | **存在** | `info.std[].value`（3 篇范文） | 服务端/评测引擎 | 答案页 `:245-250` |
| `collector.choose` | **存在** | `info.xtlist[].answer`（**选项字母**） | **客户端本地判分（两处实现）** | 见下 |
| `collector.fill` | **存在** | `info.std[].value`（`/` 多解）；另 `content_fill_answer[].answer` | **客户端本地判分（两处实现）** | 见下 |

### 2.1 `collector.choose` 判分（逐字引用）

PC bundle 套题通道（`chunks/useExamStepper-8ad3fbd7.js`，函数 `G`）：

```js
let s=[],r=(await n(p(`${l}\\${e}\\content_${t}\\content.json`))).info.xtlist;
...
const e=r.find(e=>e.xt_xh===o);
...
const s=e.xxlist.find(e=>e.xx_xh==n);
return s?(e.answer===s.xx_mc&&(l=m(a.question_score/a.question_count)),
  {entity_id:t,order:o,score_detail:{total_score:l,choose:s.xx_mc,choose_id:n,category:a.evaluation_type}}):null
```

PC bundle 听说同步通道（`chunks/Practice-1694d88a.js`，函数 `_t`）：

```js
const i=a.contents.find(e=>e.info.stid===n).info.xtlist.find(e=>e.xt_xh===o),
      r=i.xxlist.find(a=>a.xx_mc===e.answer),      // 用户选项（按字母）
      u=i.xxlist.find(e=>e.xx_mc===i.answer);      // 正确选项（按字母）
t.push({entity_id:n,order:o,score_detail:{total_score:i.answer===e.answer?c.question_score/c.question_count:0,choose:e.answer,category:c.evaluation_type}}),
s.push({id:e.id,answer:u?.xx_xh??"",user_answer:r?.xx_xh??""})
```

**结论：`answer` 与 `xx_mc` 都是选项字母，判分数据源是内容包里的 `content_<real_id>/content.json` 的 `info.xtlist`，完全在客户端**。

### 2.2 `collector.fill` 判分（逐字引用）

套题通道（`chunks/useExamStepper-8ad3fbd7.js`）：

```js
const r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std,i=JSON.parse(n);
return i.map(e=>{const[t,a]=e.id.split("_"),n=e.answer; ... 
  const e=r.find(e=>e.xth===a);
  return s=X(n), X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),
  {entity_id:t,order:a,score_detail:{total_score:l,choose:n,category:o.evaluation_type}}})
```
其中归一化函数：
```js
function X(e){const t={"‘":"'","’":"'","：":":","，":",","。":".","——":"-","—":"-",_:"-"};
  return(e=e.replace(/[‘’：，。———_]/g,e=>t[e])).toLowerCase()}
```

听说同步通道（`chunks/Practice-1694d88a.js`，函数 `mt`）：

```js
const i=a.contents.find(e=>e.info.stid===n).info.std.find(e=>e.th===l[1]).value.split("/").map(e=>encodeURIComponent(e)),
      r=ra.value.question_list[0],u=i.includes(e.answer)?1:0;
```

**结论：`std[].value` 是标准答案，`/` 分隔多解，比较前 `trim().toLowerCase()`（且先做中英标点归一化）。答案在客户端。**（注意官方两条通道的键不一致：套题用 `std[].xth`，同步用 `std[].th`——见第五节。）

### 2.3 只在服务端出现的东西

- 评测细节分：`detail_file`（内容包内相对路径，如 `s.detail_file`，PC bundle 20 处引用；`chunks/useExamResult-d9fb4671.js`：`remote_url:s.detail_file`，再由 `C(n)` / `p(s.detail_file)` 取回 JSON）。iframe 侧 `grep -rn "detail_file" iframe_web/` → **0 命中**。
- 班级结果页参考答案：`js/pc-class-result-index.js:171` `window.setAnswerPart = function (text) { $('.answer-text').html("参考答案 <br>"); ... }`，调用方（`chunks/index-4d5eea96.js`）从服务端文件读：`let e=await v(l); const{choice:n,audio:a,content:s}=e; ... N.setAnswerPart(n), N.setRecordPart(s,o)`，`l=\`${c}/${y.pageId}/Score_Content/${y.textName}.txt\``。
  → **朗读类/选择题的「参考答案 + 录音原文」在班级结果页来自服务端 `Score_Content/*.txt`，而不是内容包字段。**
- 作业未截止保护：`pc-listen-warn-result.html` 只显示「答案将在作业截止后公布」。

### 2.4 关于 role/dialogue「消费者可能是 iframe 或原生插件」的查实结果

- 内容包字段 `info.question[]`（含 `std`/`answer`/`keywords`/`role`）的**客户端唯一消费者 = `iframe_web/pc-listen-answer.html`**（另一处 `chunks/useExamResult-d9fb4671.js` 只用到 `info.question[].xh` 生成录音回放 id，出现在 `getRecordAudios`）。
- iframe 通过 `chunks/html-8e3719a4.js` 的枚举被 PC 客户端加载：
  ```js
  e.PcListen="pc-listen.html",e.PcListenAnswer="pc-listen-answer.html",e.PcListenWarnResult="pc-listen-warn-result.html",...
  ```
  且 `chunks/useExamResult-d9fb4671.js`：`const P=\`${L}/${k.PcListenAnswer}\``，`htmlUrl:P`。
- **所以：`std`/`answer` 的展示消费者是 iframe（原生把 iframe 当答案页加载）；`keywords`/`role`/`sucai`/`ref` 在客户端（PC bundle + iframe）里 0 消费者，只能是原生 Java/SO 或服务端在用。**

---

## 三、`answer1.js` / `answer2.js` / `answer3.js` 是什么

**结论：三个文件互为逐字节副本，且是「已被弃用的答案页 v1/v2/v3」残留 —— 不是数据模板，不含任何答案数据；当前无任何页面或脚本引用它们。**

证据：

```
$ md5sum iframe_web/js/answer1.js iframe_web/js/answer2.js iframe_web/js/answer3.js
b3049c8bdafe5689cd9b1f4f7c0e1ce5  iframe_web/js/answer1.js
b3049c8bdafe5689cd9b1f4f7c0e1ce5  iframe_web/js/answer2.js
b3049c8bdafe5689cd9b1f4f7c0e1ce5  iframe_web/js/answer3.js
```

全部内容（578 字节，只有 3 个事件处理器）：

```js
// iframe_web/js/answer1.js（= answer2.js = answer3.js）
$(function(){
	$(document).on("click",".show_m",function(){
		$(".test_title").css("height","initial");
		$(this).removeClass("show_m").addClass("hide_m");
	})
	$(document).on("click",".hide_m",function(){
		$(".test_title").css("height","25px");
		$(this).removeClass("hide_m").addClass("show_m");
	})	
	$(document).on("click",".my_record",function(){
		var id = $(this).parent().parent().attr("id");
		var fileId = $(this).attr("id");
		user_audio(id,fileId);
	})
	$(document).on("click",".stander_record",function(){
		var id = $(this).attr("id");
		stander_audio(id);
	})
})
```

关键佐证：

1. 无引用：
   ```
   $ for f in answer1.js answer2.js answer3.js view1.js view2.js view3.js PC-choose.js method.js \
            pc-ch1.js pc-ch2.js bj_title_text.js zoom.js pc-view-kttb2.js pc-view-kttb3.js \
            pc-view-kttb5.js paperview.js; do grep -rl --include=*.html --include=*.js "$f" . | grep -v "/$f"; done
   （全部无输出）
   ```
   `pc-listen.html` 的 `<script>` 列表 = `jquery.min.js / disabled-user-select.js / vue.min.js / pc-listen-fn.js / pc-listen-show-wordinfo.js / pc-listen-choose.js / pc-listen-data.js / pc-listen-default.js / pc-listen.js / pc-view-kttb4.js / pc-view-kttb1.js / word-translation.js`；`pc-listen-answer.html` = `... / pc-listen-answer.js / word-translation.js`。**都不含 answer1/2/3.js。**
2. 调用的 `user_audio` / `stander_audio` **在全仓无定义**（`grep -rn "user_audio\|stander_audio"` 仅命中这三个文件内部的调用）。
3. 它们依赖的 DOM 类只存在于**未被任何 HTML 加载**的 `css/answer.css`：
   ```
   $ grep -rn "stander_record" css/      → 无输出
   $ grep -rn "my_record" css/           → css/answer.css:212:.my_record{
   $ grep -rl "answer.css" --include=*.html .   → 无输出（无 HTML 加载 answer.css）
   $ grep -rl "view.css"   --include=*.html .   → 无输出
   ```
4. 同一批残留还有 `view1.js` / `view2.js` / `view3.js`（`view.css` 同样无人加载），它们按「选择题有图 / 无图 / 无选择题」三种老模板绑定 `.choose1`/`.choose2` 点击。

**性质判定**：`answer1/2/3.js` = 老版「答案页」三套版式（v1/v2/v3）的脚本残留，功能是「题面折叠 + 我的录音回放 + 标准录音回放」；`view1/2/3.js` = 老版「题目页」三套版式的脚本残留。它们**不提供也不消费任何答案字段**。

---

## 四、iframe 与 PC bundle 的关系

### 4.1 是否同一构建

**不是同一构建产物，是「同一产品内的两个不同技术栈、由 PC bundle 加载」的关系。**

| 维度 | iframe web | PC bundle |
|---|---|---|
| 技术栈 | jQuery 1.8.3 / jQuery min + Vue 2 (`vue.min.js`) 多页 HTML | Vue 3 + Vite 打包（`index-5543d811.js` + `chunks/*.js`，含 `__vitePreload`、`_withScopeId`） |
| 交付形态 | 明文未压缩，含中文注释 | 压缩混淆，`chunks/` 带 hash 文件名 |
| 版本标识 | **未找到版本号**（`config` 文件只有一行 `trans = {"translate": 1}`；HTML/JS 头部无版本串） | `index-5543d811.js` 内含 `version:"2.0.1"` / `version:"3.0.0"` / `version:"3.16.0"`（多为第三方组件版本，非 App 版本号） |
| 文件时间戳 | 全部为容器内解包时间 `10月 6日 19:53`，无原始 mtime | 全部 `10月 6日 19:44`，无原始 mtime |

**加载关系（硬证据）**：

```js
// pc_client_web/chunks/html-8e3719a4.js （全文）
var e=(e=>(e.PcBannerSkillAdvance="pc-banner-skill-advance.html",e.PcBannerStudyCenter="pc-banner-study-center.html",
e.PcClassResultIndex="pc-class-result-index.html",e.PcEIntroduce="pc-E-introduce.html",
e.PcListen="pc-listen.html",e.PcListenAnswer="pc-listen-answer.html",
e.PcListenWarnResult="pc-listen-warn-result.html",e.PcPurchase="pc-v5-purchase.html",
e.PcPaper="pc-paper.html",e.PcGrow="pc-v5-grow.html",e.PcScoreStudy="pc-score-study.html",
e.PcScoreRank="pc-score-rank.html",e.PhonogramMap="phonogramMap.html",e.ReadWrite="read-write.html",
e.ReadWriteAnswer="read-write-answer.html",e))(e||{});
```

```js
// pc_client_web/chunks/useExamStepper-8ad3fbd7.js
const{IFRAME_LOCAL_PATH:Y}=d(); ... htmlUrl:`${Y}/${V.PcListen}`
// pc_client_web/chunks/useExamResult-d9fb4671.js
const{IFRAME_LOCAL_PATH:L}=e(),P=`${L}/${k.PcListenAnswer}`, ... {htmlUrl:P, ...}
```

→ 题目页 = `pc-listen.html`，答案页 = `pc-listen-answer.html`，**由 PC bundle 通过 `<iframe>` 加载**。

### 4.2 数据是怎么进 iframe 的（决定了字段解释权归属）

`processJsParam`（`index-5543d811.js`）把模板 `ctrl.json` 的 `model_pc.jsparam[]`（每项 `{source, real_id, code_id, classname}`）映射成 iframe 的 `set` 对象：

```js
async processJsParam(e,t,n){ ... for(const n of e){
  const e=replacePathSeparator(`${this.generateFilePath(n.source,t.real_id,r)}\\info.json`),
        i=await this.getJson(e),
        a=lodashExports.cloneDeep(i).find(e=>e.code_id===n.code_id),
        s=t.exam_info,
        l=e=>s.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),e);
  if(a){ let e;
    e=["txt","txxt"].includes(a.code_type)?l(a.code_value)
     :"json_array"===a.code_type?l(a.code_json_array)
     :"json_obj"===a.code_type?l(a.code_json_obj)
     :"image"===a.code_type?[{url:replacePathSeparator(`${this.IMAGE_LOCAL_PATH}\\...\\material\\${a.code_value}`)}]:"";
    o[n.classname]=e } } }
```

要点：
- iframe 里的 **`content_choose` / `content_fill_answer` / `content_fill_answer_enable` / `words_con` / `gap_mixture` 等都是 `info.json` 里 `code_type=json_obj/json_array` 的字符串**，因此 iframe 端必须 `JSON.parse`（`pc-listen-default.js:57-70`），这正好解释了示例里它们是**字符串字面量**：
  ```js
  content_choose: '{\"stid\":\"115852\",...,\"answer\":\"A\",...}'
  ```
- **`{{thN}}` / `{{smN}}` 占位符由原生侧替换**（`s.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),e)`，`s = exam_info`），**iframe 自己不做替换**：`thFormat` 在 `pc-listen.js:45` 与 `pc-listen-answer.js:46` 里**定义了但从未被使用**（`grep -rn "thFormat" iframe_web/` 只命中这两行定义）。
- 选择/填空判分读的是**另一个文件** `content_<real_id>/content.json`：
  ```js
  // index-5543d811.js  processPaperContentJson
  const t=await this.getJson(replacePathSeparator(`${this.getContentFilePath(e.real_id)}\\content.json`)),
        n=lodashExports.cloneDeep(t),
        r=JSON.parse((o=JSON.stringify(n), e.exam_info.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),o)));
  ```
  即内容包同时有 `info.json`（给 iframe 的 classname 字段）和 `content.json`（`info.xtlist` / `info.std`，给判分）。

### 4.3 同一族的字段解释冲突点（明确列出）

| # | 冲突 | iframe 侧 | PC bundle 侧 | 影响 |
|---|---|---|---|---|
| 1 | **`xtlist[].answer` 的取值域** | 内容包里是**选项字母**（`"answer":"A"`），iframe **从不读取**它；从原生灌进来的 `right_answer` 是**选项序号**：`pc-listen-choose.js:98-126` `var x = String.fromCharCode(0x01); ... var arr = item.split(x); var id = arr[0], right_answer = arr[1]; var right_answer_index = parseInt(right_answer,10)-1; $(...).find('.choose2:eq('+right_answer_index+')')` | **字母 ↔ 字母**：`e.answer===s.xx_mc`（`useExamStepper` 的 `G`） | 同一字段在**数据层=字母**、**iframe 桥接层=序号（`xx_xh`）**。转换发生在 PC bundle：`function b(e){...if(r>=65&&r<=90)o+=(r-64).toString()...}`（A→"1"），并把 `u?.xx_xh` 随 `${entity_id}_${order}_${xx_xh}` 传给 iframe |
| 2 | **`setPCChoose2` 缺失** | iframe 只定义 `setPCChoose`（`pc-listen-choose.js:96`、`PC-choose.js:52`）、`setPCCheckChoose`（`PC-choose.js:94`），**没有 `setPCChoose2`** | 答案展示模式会调用它：`if(1==t.displayAnswerContent&&y.setPCChoose2){const e=o.map(e=>\`${e.entity_id}_${e.order}_${e.choose_id}\`);y.setPCChoose2.apply(null,e),r()}` | 选择题「答案展示」模式下 iframe 无法标记正确项（PC 侧有 `&&y.setPCChoose2` 保护，静默跳过）。**是否为本次解包缺文件，未找到决定性证据** |
| 3 | **`content_highlight` 只在 PC 侧存在** | iframe 的 `set.content` 是**原样**渲染（`pc-listen.html:230` `<div class="content" v-show="ifThis(set.content)" v-html="formatString(set.content)">`），`grep -rn content_highlight iframe_web/` → **0 命中** | `o.content_highlight&&(o.content=o.content.replace(o.content_highlight,\`<span class="color_repeat">${o.content_highlight}</span>\`))`（`index-5543d811.js`） | 由 PC 侧在 setData 前把高亮片段写回 `content`；**若我方直接读内容包的 `content` 而不应用 `content_highlight`，会丢掉「跟读高亮片段」** |
| 4 | **`std` 的定位键不一致（官方自身两处代码不一致）** | iframe 不判分 | 套题：`r.find(e=>e.xth===a)`（`a` 来自 `id.split("_")[1]`）；同步：`info.std.find(e=>e.th===l[1])` | 同一族两条官方判分通道分别用 `xth` / `th`。若包的 `th` 带点（`"th":"1."`，见 `content_fill_answer` 示例）则同步通道会找不到答案 → 我方必须**同时**接受 `xth` 与 `th`（并容忍尾部 `.`） |
| 5 | **`collector.repeat` 小题扩容来自另一个文件** | iframe 只认 `info.fj[]`（答案页）；**不读 `info_extra.json`** | `index-5543d811.js`：`if("collector.repeat"===e.exam_type_collect){ ... const t=await this.getJson(replacePathSeparator(\`${this.getContentFilePath(r.real_id)}\\info_extra.json\`)); let o=cloneDeep(r.question_list), i=Number(t.length)-o.length; for(;i>0;){...} const a=e(r.exam_score,o.length); o.forEach((e,t)=>{e.question_count="1",e.question_score=a[t],s+=parseFloat(a[t])}); r.exam_count=String(l),r.exam_score=String(s),r.question_list=o }`；`processPaperSteps` 里还用 `info_extra.json` 的 `{id,value}` 对 ctrl 步骤做正则替换 | 跟读题的**小题数/每题分值由 `info_extra.json` 的长度决定**；我方只读单个 `info.value` 时会严重少题 |
| 6 | **`answer` 在回显模式下被 PC 侧覆写** | iframe 只做 `:value="j.answer"` 展示 | `e.forEach(e=>{const t=o.find(t=>String(t.entity_id)===e.stid&&String(t.order)===e.xth); t&&t.detail&&(e.answer=(typeof t.detail==="string"?JSON.parse(t.detail):t.detail)?.choose)}),s.content_fill_answer_enable=JSON.stringify(e),s.show_topic_blank="1"` | `content_fill_answer_enable[].answer` 到达 iframe 时**可能已被替换成用户作答**；内容包原值才是标准答案 |
| 7 | **`collector.role` / `collector.dialogue` 无任何示例与差异处理** | 仅 1 行 `v-if` 共用 3q5a 块 | `grep -ro "collector.role\|collector.dialogue" pc_client_web/` → 只出现在 `useExamResult` 的 `switch case` 聚合分支里 | 两端都无法证实 role/dialogue 与 3q5a 在字段上是否有差异；**只能确认渲染路径完全相同** |

---

## 五、与我方 `src/collectors.rs` 的逐条差异

对照文件：`/home/shuaiqiu/Projects/ets_resolver/src/collectors.rs`（1704 行）。

### 5.1 `collect_choose`（`collectors.rs:197-284`）

我方已做对：读 `info.xtlist[].answer`、映射成 `choose_id`（= 官方 `xx_xh`）、拼 `options` 为 `"{xx_mc}. {xx_nr}"`、输出 `options_detail`。官方 PC judge `e.answer===s.xx_mc` 与 `choose_id:n`（= `xx_xh`）**完全对得上**。注释里写的「两边都是字母，已用真实语料证实」是对的。

差异：

| 差异 | 我方 | 官方 |
|---|---|---|
| 未读 `info.st_nr`（对话原文） | 丢弃 | 答案页 + 题目页都渲染：`pc-listen.html:194` / `pc-listen-answer.html:322` `v-html='one.info.st_nr'` |
| 未读 `info.st_sm`（大题说明） | 丢弃 | `pc-listen.html:158` |
| **未读 `xtlist[].xt_value`（录音原文）** | 丢弃 | 答案页唯一的「题目相关补充文本」：`pc-listen-answer.html:339-345` |
| 未读 `xtlist[].xt_wj` / `xt_pic` / `info.st_pic` / `xx_wj` | 丢弃 | UI 与音频定位 |
| `xt_nr` 的 `{{thN}}` 占位符 | 依赖 `cleaner::strip_global_no`/`strip_ets_th`，**二者都不认 `{{th1}}`**（`extract_global_no` 要求首位即数字，`extract_ets_th` 要求 `ets_th` 前缀；见 `cleaner.rs:309-330`） | 原生在 setData 前替换：`exam_info.reduce(...)` |
| 未读 `content_choose` / `content_choose2` 里的 `answer` | 未处理这两个字段 | `pc-listen-default.js:57-70` 会 `JSON.parse` 它们并渲染（其中含 `answer`） |

### 5.2 `collect_fill`（`collectors.rs:401-427`）

| 差异 | 我方 | 官方 |
|---|---|---|
| `std[].value` 的 `/` 多解 | 原样塞进 `answers[0]`，**未拆解**、未标注比值语义 | `X(e.value).split("/").map(e=>e.trim().toLowerCase())`，逐项比对 |
| `question` 恒为空字符串 | `String::new()` | 官方题干来自 `info.value`（短文）或 `std[].th`：答案页 `'● '+i.th+'.'+i.value` |
| `std[].th` / `xth` | 只把 `xth` 用作题号（`collectors.rs:410`），`th` 丢弃 | 两处都用：套题 `xth`、同步 `th`；`th` 也用于 `input` 的 `name` |
| `std[].ai` / `std[].audio` | 丢弃 | `ai` 两端 0 消费者；`audio` 是预留标准音 |
| `content_fill_answer` / `content_fill_answer_enable` / `show_topic_blank` | 完全未处理 | `pc-listen.html:298-313` 渲染；PC 侧会覆写 `answer` |
| `info.value` / `info.image` / `info.audio` / `info.analyze` / `info.topic` / `info.ref` | 全部丢弃 | 渲染 / 素材 |
| 路由判定 | `collect_generic_routed` 用「`std[0]` 有 `xth` → fill」；`parse_content_routed` 用 `st.contains("fill")` | — |

### 5.3 `collect_picture`（`collectors.rs:428-473`）

| 差异 | 我方 | 官方 |
|---|---|---|
| `important` 来源 | `keypoint`（按 `</br>` 拆）+ `analyze`（按 `</br>` 拆）**拼接** | 二者在 A3 里是**同一份 10 条要点**（一个带 `叙述要点：` 前缀），且答案页**两块都被注释**（`pc-listen-answer.html:234-244`）→ 我方会**重复一倍要点** |
| `question` 取 `topic` | 与官方一致（答案页 `v-html='one.info.topic'`） | ✓ |
| `std[].value` → answers | ✓ 全量 | 答案页 `v-if='$index<3'` **只展示前 3 篇**（数据仍是 3 篇，此处一致） |
| `info.image` / `info.audio` / `info.ref` | 丢弃 | UI / 空数组 |

### 5.4 `collect_read`（`collectors.rs:474-531`）— 同时被 `collector.word` / `collector.repeat` 复用

| 差异 | 我方 | 官方 |
|---|---|---|
| `info.sublist` 优先分支（`collectors.rs:487-510`） | 有 | **iframe 的 9 族里不存在 `sublist`**；`sublist` 属于另一套模块（朗读/背诵页）：`pc_client_web/chunks/textInfo-82764d4d.js`、`readSentence-80965549.js`、`readSentenceScoreInfo-407c9299.js`、`chunks/index-68ad25e0.js`。**这解释了 `collect_repeat_essay` 里 `sublist` 的来源（不是 collector 系列）** |
| 答案=原文 | ✓ `vec![text]` | ✓ `v-html='one.info.value'` |
| `info.analyze` → important | ✓ | read 的 `analyze` 在答案页未展示（read 块无 analyze，只有 3q5a 块有） |
| `info.symbol` / `info.ai` / `info.videotime` | 丢弃 | `symbol` 答案页注释（`pc-listen-answer.html:41`）；`ai` / `videotime` 两端 0 消费者 |
| `evaluation_type` 子型（`read_chapter`/`read_sentence`/`read_word`/`topic`） | 未建模 | PC bundle 用 `question_list[0].evaluation_type` 决定评分/展示分支：`if(["read_sentence","read_word","read_chapter"].includes(y.category)){...}else{...N.setAnswerPart(n)...}`；`showDynamicOriginalButton` 里 `["read_chapter","topic","read_word","read_sentence"].includes(e.evaluation_type)` |

### 5.5 `collector.word` 的路由与字段

| 差异 | 我方 | 官方 |
|---|---|---|
| 族名 | `parse_content_routed`：`st.contains("read") || st.contains("word") || st.contains("sentence")` → **`collector.word` 被归到 `TYPE_READ`**，`TYPE_WORD` 常量（`collectors.rs:15`）定义了却没用于路由 | iframe 有独立块：`pc-listen-answer.html:522` `v-if='one.structure_type=="collector.word"'` |
| `new_xh` 重编号 | **未实现** | `pc-listen-default.js:29-49`：同批 `collector.word` 的 `stid` 不一致时，按数组序注入 `info.new_xh = 1,2,3...`（答案页 `v-text='one.info.new_xh \|\| one.info.xh'`） |
| `info.value_bz` / `info.value_pf` | 丢弃 | **两端 0 消费者**（见第一节表）→ 语义未找到 |
| `info.read_title` / `info.mpg` / `info.image` | 丢弃 | `read_title` 在题目页 `pc-listen.html:63` 渲染（`v-html="formatString(set.read_title)"`，走 `set.read_*` 通道） |

### 5.6 `collector.repeat` — **最严重的一处差异**

| 差异 | 我方 | 官方 |
|---|---|---|
| 小题粒度 | `parse_content_routed` 把 `collector.repeat` 归入 `QUESTION_TYPES` → `collect_question_based`（`repeat` 无 `info.question` → 空）→ 兜底 `has_text_value` → `collect_read` → **得到 1 题（`info.value` 整段）** | **`info.fj[]` 每句 = 1 小题**（`pc-listen-answer.html:484` + `pc-bundle: case"collector.repeat":for(const n of r.info.fj)...n.xh`） |
| `fj[].value` | **完全未读取** → 逐句标准答案丢失 | 逐句答案 + 逐句音频 |
| `fj[].audio` / `xj` 顺序 | 未读 | `:id="set_id+'_'+one.info.stid+i.audio"` |
| 题数/分值修正 | 未做 | 需读 `content_<real_id>/info_extra.json` 并以 `Number(t.length)` 决定小题数（`index-5543d811.js`） |
| `fj[].fs` / `fj[].time` | 未读 | 两端 0 消费者 → 语义未找到（不要凭形状猜） |

> 影响面：中考/高考「跟读与模仿」题按分句计分，我方按 1 题解析会**少 N-1 题、丢 N-1 份答案**。

### 5.7 `collect_question_based`（`collectors.rs:286-366`，承载 3q5a / role / dialogue

| 差异 | 我方 | 官方 |
|---|---|---|
| `std[].value` → answers | ✓ | ✓ `参考答案:` |
| `q.answer` → `extra.ref_answer` | 取名为 `ref_answer` | 官方 UI 标签是「**回答:**」，语义 = **示范回答**，不是 ref（`pc-listen-answer.html:157-166`）。建议改名 `model_answer` / `demo_answer` |
| **`q.analyze`** | **未采集** | 3q5a/role/dialogue 块里是 **ACTIVE 展示项**：`pc-listen-answer.html:135-137` `<div class="mb20" v-show="i.analyze!=''"><p v-html='i.analyze'>` |
| `q.keywords` → important | ✓ 按 `\|` 拆 | ✓ 官方格式 `did graduate Cambridge\|are graduate Cambridge\|...` |
| `q.role` | ✓ 存 `extra.role` | ✓ `"a"`/`"b"` |
| `q.askaudio` / `aswaudio` / `askimg` | **未读** | 答案页各有一个播放按钮 / 图片 |
| `q.std[].ai` / `std[].audio` | 未读 | `audio` 用于答案音频按钮（样本 `""`） |
| `q.sucai` / `q.ref[]` | 未读 | 两端 0 消费者 → 语义未找到 |
| `q.new_xh` | 未读 | `v-text='i.new_xh \|\| i.xh'` |
| 题号来源 | `extract_global_no(ask)` → `extract_ets_th` → `xh` | 官方只用 `xh`（=提交 order）+ 运行时 `new_xh` |

### 5.8 `collect_repeat_essay`（`collectors.rs:560-592`）

| 差异 | 我方 | 官方 |
|---|---|---|
| `collector.repeat_essay` 族 | 我方有常量 `TYPE_REPEAT_ESSAY`（`:16`）+ 路由分支（`:1405`）+ `sublist` 解析 | **两端 0 引用**：`grep -rn "repeat_essay" iframe_web/ pc_client_web/` → 无命中；`repeat_dialogue` 同样 0 |
| `info.sublist` | 作为该族容器 | `sublist` 只属于朗读/背诵模块（`textInfo-*` / `readSentence-*`），不是 collector 系列 |
| `collector.picture`（故事复述） | 我方另有独立 `collect_picture` | ✓ 一致；**注意官方复述题的"参考答案"就是 `info.std[].value`，不是 `sublist`** |

---

## 六、我方最该补的 5 个字段

按「答案正确性收益 / 实现成本」排序：

1. **`collector.repeat` 的 `info.fj[].value`（+ `fj[].xh`）** —— 逐句标准答案与小题粒度。当前我方把整段当 1 题，直接丢 N-1 个小题和逐句答案。同时补读 `content_<real_id>/info_extra.json` 的 `length` 决定小题数/分值。
2. **`collector.fill` 的 `info.std[].value` 的 `/` 多解拆分（`split('/')` + `trim().toLowerCase()` + 中英标点归一化）** —— 当前 `19th/nineteenth/19<sup>th</sup>` 被当成单个答案串；同时补 `std[].th`（与 `xth` 并存），因为官方两条通道分别用 `xth` 和 `th` 定位。
3. **`collector.choose` 的 `info.xtlist[].xt_value`（录音原文）** —— 答案页对该族唯一额外展示的题目相关文本；配合 `info.st_nr`（对话原文）一起补，才与官方答案页信息量对齐。
4. **`collector.3q5a` / `role` / `dialogue` 的 `info.question[].analyze`** —— 该块是答案页 **ACTIVE** 展示项（不是注释），我方完全未采集；同时把 `answer` 的输出键名从 `ref_answer` 改为语义正确的 `model_answer`（示范回答），并补 `askaudio` / `aswaudio` / `askimg`。
5. **`collector.word` 的 `info.value_bz` / `info.value_pf` 与 `new_xh` 重编号** —— `value_bz`/`value_pf` 是内容包里与 `value` 并列的两个同值候选（**两端 0 消费者，语义未找到**，但极可能是「标注/评分用文本」），先原样保留以免丢失；`new_xh` 的官方重编号逻辑（`pc-listen-default.js:29-49`）决定多小题单词题的显示序号，与我方题号输出一致性直接相关。

**附（不属 5 个之内但需登记的风险）**：`content_highlight`（PC 侧对 `set.content` 的高亮回写）与 `info.symbol`（`ai` / `videotime` / `fj.fs` / `fj.time` / `sucai` / `ref` 同列）当前两端均无消费者，属「未找到语义」，不要凭形状补语义，只需原样透传。

---

## 七、检索关键词与「未找到」清单

- 已检索（`iframe_web/` 与 `pc_client_web/` 全量，排除 `jquery*` / `vue.min.js` / `echarts.min.js`）：
  `structure_type` `collector.*` `xtlist` `xt_xh` `xt_nr` `xt_wj` `xt_value` `xt_pic` `xxlist` `xx_xh` `xx_mc` `xx_nr` `xx_wj` `xth` `th` `fj` `std` `ref` `answer` `aswaudio` `askaudio` `askimg` `keywords` `analyze` `sucai` `role` `topic` `keypoint` `content_highlight` `audio_url` `detail_file` `info_extra` `content.json` `info.json` `sublist` `value_bz` `value_pf` `videotime` `symbol` `fs` `time` `new_xh` `st_sm` `st_nr` `st_pic` `content_fill_answer` `content_fill_answer_enable` `show_topic_blank` `repeat_essay` `repeat_dialogue` `setPCChoose2` `setPCChoose` `getChoice` `ReturnChoose` `kttb_setPcChoise`
- **未找到**：
  1. `collector.role` / `collector.dialogue` 的**示例内容包**（两端都没有；只有渲染分支）。
  2. `config/` 目录 —— `iframe_web/config` 是**24 字节文件**，内容 `trans = {"translate": 1}`，不是目录，**没有版本号/示例数据**。
  3. iframe 侧的任何版本号字符串。
  4. `value_bz` / `value_pf` / `mpg` / `sucai` / `ref` / `fj.fs` / `fj.time` / `videotime` / `info.ai` / `std[].ai` 的**语义与消费者**（两端 0 命中，只有示例数据本身）。
  5. `audio_url` 在听力口语套题链路的任何使用（`iframe_web/` 内只有 `word-symbol.html:84-86` 的单词发音，与 9 族无关）。
  6. `content_highlight` 在 iframe 侧的任何使用（仅在 PC bundle 3 处）。
  7. `repeat_essay` / `repeat_dialogue` 在官方两端的任何引用（0）。
