# B 组：读写类四族官方解析语义（静态提取）

> **只做静态代码阅读**：未运行客户端、未抓包、未联网。
>
> 覆盖族：`collector.choose` / `collector.fill` / `collector.picture` / `collector.3q5a`
>
> 主证据（PC 端远程 web app 明文源码）：
> - `ets源码恢复/pc_client_web/index-5543d811.js`（3 479 433 B）
> - `ets源码恢复/pc_client_web/chunks/useExamStepper-8ad3fbd7.js`（19 556 B）
> - `ets源码恢复/pc_client_web/chunks/useExamResult-d9fb4671.js`（8 390 B）
> - `ets源码恢复/pc_client_web/chunks/html-8e3719a4.js`、`DynamicIframe-bafff5bc.js`、`useAudioSync-0603b875.js`、`mockExam-f7dae7ee.js`、`exam-6ff42190.js`、`Examine-d229829d.js`
>
> 旁证（同一官方产品的其它静态资产，**与主证据分开标注**）：
> - 真实官方资源包 `Fe-pro/ets读取/北京初中resoure/*/content.json`（含 `collector.choose` 真样本）
> - 真实官方资源包 `Fe-pro/ets读取/广东初中resoure/*/content.json`
> - 真实官方语料 `ets_resolver/tests/corpus/guangdong_hs_mock1/01_角色扮演/content.json`（含 `collector.3q5a` 真样本）
> - 另一代读写 iframe 模板应用 `Fe-pro/ets读取/广东初中resoure/common/rw-index-1.html`、`rw-index-2.html`、`js/rw-index-2.js`
> - Android 侧 bean：`ets源码恢复/jadx_template_info/sources/com/ets100/secondary/model/mock/InfoObjBean.java`

---

## 0. 证据基线与已确认的缺口（先读这一节）

### 0.1 族常量在 PC 前端里的全部出现点（穷举）

```bash
$ cd ets源码恢复/pc_client_web
$ grep -oh 'collector\.[A-Za-z0-9_]*' index-5543d811.js chunks/*.js | sort | uniq -c
      1 collector.choose      (chunks/useExamStepper-8ad3fbd7.js)
      1 collector.choose      (chunks/useExamResult-d9fb4671.js)
      1 collector.fill        (chunks/useExamStepper-8ad3fbd7.js)
      1 collector.fill        (chunks/useExamResult-d9fb4671.js)
      2 collector.picture     (chunks/useExamResult-d9fb4671.js)
      2 collector.3q5a        (chunks/useExamResult-d9fb4671.js)
      4 collector.read / 3 collector.word / 2 collector.role / 2 collector.dialogue
      2 collector.repeat      (chunks/useExamResult-d9fb4671.js)
      3 collector.repeat      (index-5543d811.js)
```

`exam_type_collect` 字段的**全部**读取点（4 处）：

```bash
$ grep -l 'exam_type_collect' index-5543d811.js chunks/*.js
index-5543d811.js                       # 3 处，全部 === "collector.repeat"（ProcessPaper 里）
chunks/useExamResult-d9fb4671.js        # 1 处 switch（9 个 case）
chunks/useExamStepper-8ad3fbd7.js       # 2 处 if（practise 步骤）
```

> 结论：**`collector.choose` / `collector.fill` 的判分逻辑只存在于 `useExamStepper-8ad3fbd7.js`**；
> `collector.3q5a` / `collector.picture` 在 `useExamResult-d9fb4671.js` 的 `switch`、`getRecordAudios`、`getAudioIds` 里出现。

### 0.2 关键运行期对象：答案契约的两端都在 iframe 页面里

`exam_type_collect` 的族名是 **`res.json → exam_type_list[]`** 的字段（见 §1.1 的 `processPaperResources`）。

前端所有族相关的回调都挂在 **iframe 的 `contentWindow`** 上，宿主只负责算分/拼串：

`chunks/useExamStepper-8ad3fbd7.js`（`O.on` 的第一个语句与 `practise` 步骤）：

```js
O.on(t=>{const a=t.win;y=a;const o=t.logs??[];C.value=o;
```

| 方向 | 函数 | 定义位置 |
|---|---|---|
| 宿主 → iframe | `y.setData(htmlData, paths, examId, "", isMock, choice_order)` | iframe 页面（本地，未下载） |
| 宿主 → iframe | `y.setPCChoose2(...items)` / `c.setPCChoose(...items)` / `c.set3q5aScore(...items)` / `c.setPcBlank(json)` / `c.showScore(real,max,0)` | iframe 页面 |
| iframe → 宿主 | `y.getChoice()` / `y.ReturnChoose` / `y.getPcBlank()` / `y.returnPcBlank` / `y.getChooseChange` | iframe 页面 |

```bash
$ grep -c 'setPCChoose\|setPcBlank\|getChoice\|set3q5aScore' index-5543d811.js
0
```

即：**这 8 个桥接函数在整个 PC web bundle 与 82 个已下载 chunk 中都不存在定义**，
它们是 iframe 页面里的函数；而 iframe 页面是**客户端本地静态资源**，不在本次下载范围内：

`index-5543d811.js`：
```js
function useDownloadConfig(){const e=["http://","https://"].some(e=>cef.message.getHomeUrl().includes(e))?STIC_PATH:"", ... return{EXE_PATH:EXE_PATH,LOCAL_HOST:e,DOWNLOAD_PATH:n,FS_UNZIP_LOCAL_PATH:`${e}${r}`,IFRAME_LOCAL_BASE_PATH:`${e}${r}`,IFRAME_LOCAL_PATH:"./common",IMAGE_LOCAL_PATH:`${e}${r}`,...}}
```

`chunks/html-8e3719a4.js`（iframe 页面清单）：
```js
var e=(e=>(e.PcBannerSkillAdvance="pc-banner-skill-advance.html",...,e.PcListen="pc-listen.html",e.PcListenAnswer="pc-listen-answer.html",...,e.ReadWrite="read-write.html",e.ReadWriteAnswer="read-write-answer.html",e))(e||{});
```

`chunks/useExamStepper-8ad3fbd7.js`（PC 端所有 `view.html` 步骤都指向 `pc-listen.html`）：
```js
"view.html":e=>{de(),_e=e,ge.trigger({type:e.filetype,key:Date.now(),displayAnswerContent:he.value,htmlData:B.cloneDeep(e.model_pc.extractedJsParam),htmlUrl:`${Y}/${V.PcListen}`}),...
```

### 0.3 缺口 1：下载不完整（114 / 194 个 chunk 缺失）

```bash
$ python3 -c "...从 bundle 抽 ./Xxx-hash.js 与 chunks/ 目录求差..."
refs 194 have 82 missing 114
```

与本任务**直接相关**的缺失 chunk：

```
MISSING ReadingWritingDetails-1b8d0987.js
MISSING ReadingWritingList-35f22288.js
MISSING ReadingWritingResult-93fd0909.js
MISSING ReadingWritingStart-6a005565.js
MISSING SpecialReadingWritingStart-d2af4bf6.js
MISSING useReadingWritingStructConverter-0e20e02c.js      ← 读写结构转换器，最关键
MISSING readAloud-0a41f595.js / readAloud-b5e53cda.js
MISSING readSentence-80965549.js / readSentence-acaf6c32.js
MISSING QuestionListItem-229d65bc.js
MISSING MockExamResult-789f2f57.js
```

路由依赖关系（`index-5543d811.js` 的 router 定义，原文节选）：

```js
{path:"/readingWritingDetails",name:"readingWritingDetails",component:async()=>await __vitePreload(()=>import("./ReadingWritingDetails-1b8d0987.js"),["./ReadingWritingDetails-1b8d0987.js","./ExamActionBar-3192b928.js","./ExamActionTextBtn-a34958a4.js","./ExamActionTips-665a88fe.js","./DynamicIframe-bafff5bc.js","./TabNav-c173904e.js","./useRouteState-97bcffb8.js","./useMistakeInspector-7f434456.js","./FooterToolbar-45675d9e.js","./ExamResultHeader-6cc64b72.js","./StarBar-50dbb0bc.js","./rate-4ed993c7.js","./QuestionListItem-229d65bc.js","./exam-6ff42190.js","./useReadingWritingStructConverter-0e20e02c.js","./html-8e3719a4.js","./mistakeAdder-2da3b6d6.js","./composition-9877659a.js"],import.meta.url)
```

> 因此：**PC 端"读写"专用页面的渲染/判分分支无法从本次下载中提取**。
> 但 `collector.choose` / `collector.fill` 的判分函数 `G()` 与 `returnPcBlank()` 已在
> `useExamStepper` 中完整可读，四族语义仍可定案（见 §1–§4）。

### 0.4 缺口 2：`IFRAME_LOCAL_PATH="./common"` 下的 iframe 应用未下载

需要补齐的本地文件（来自上面 `html-8e3719a4.js` 与 `DynamicIframe` 的失败资源清单）：

```
common/pc-listen.html  common/pc-listen-answer.html  common/read-write.html  common/read-write-answer.html
common/js/PC-choose.js  common/js/choosePc-hack.js  common/js/pc-listen.js  common/js/pc-listen-choose.js
common/js/pc-listen-data.js  common/js/pc-listen-fn.js  common/js/read-write.js  common/js/read-write-data.js
common/js/read-write-answer.js  common/css/choosePc.css  common/css/read-write-style.css
```

`objects/chunks/DynamicIframe-bafff5bc.js` 里这段资源白名单逐字列出了它们：

```js
,i=["config","css/android_page.css","css/answer.css","css/choosePc-hack.css","css/choosePc.css",...,"css/read-write-style.css",...,"js/PC-choose.js","js/answer1.js",...,"js/choosePc-hack.js",...,"js/pc-listen-answer.js","js/pc-listen-choose.js","js/pc-listen-data.js","js/pc-listen-fn.js","js/pc-listen.js",...,"js/read-write-answer.js","js/read-write-data.js","js/read-write-default.js","js/read-write.js",...,"read-write-answer.html","read-write.html","word-symbol.html"]
```

### 0.5 已确认：两处"不是"（避免误判）

| 曾被怀疑 | 结论 | 证据 |
|---|---|---|
| stic 桥接 JS 定义了 `setPCChoose` 等 | **否** | `grep -c 'setPCChoose' httphelper.js etshelper.js clientDB.js appcontroller.js shellhelper.js` → 全 0（这些是 CEF `cef.message` 消息桥，只有 HttpHelper/EtsHelper 等） |
| APK 里有 `3q5a` 明文 | **否，是二进制噪声** | `python3` 按字节扫 `E听说中学_7.0.65.2439.apk`：`b'3q5a'` 命中 1 次（offset 93778538），周围全是不可读 base64 噪声；`b'collector.3q5a'` / `b'三问五答'` 命中 0 次；解压后的 `jadx_*/sources` 中 `3q5a` 命中 0 次 |

---

## 1. `collector.choose`（选择类）

### 1.1 族判定与分发点

**分发点 A（作答/判分）** — `chunks/useExamStepper-8ad3fbd7.js`，`ctrl.json` 步骤 `filetype === "practise"` 的处理器：

```js
practise:async()=>{clearTimeout(me.value),me.value=setTimeout(async()=>{if(y){if("collector.choose"===w.value.exam_type_collect){y.ReturnChoose=e=>{N.logCustom(`操作选择的answers:${JSON.stringify(e)}`,{tag:"FLOW_PRAC"})},await _();const t=y.getChoice(),a=await G(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_choose"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}}"collector.fill"===w.value.exam_type_collect&&(...)}...},50)}
```

要点：
- `w.value` = `examType`（`res.json` 的某个 `exam_type_list[i]`），`_e` = 当前 `ctrl.json` 步骤。
- `y` = iframe 的 `contentWindow`。
- `await _()` 中 `_` 是 `nextTick`（`useExamStepper` 的 `W as _`，而 bundle 导出 `nextTick as W`）。
- `x.trigger(e)` = `answerHandler`，把每题结果推给答题队列（→ `requestAnswerLogs` → `/m/audio/sync-v2`）。
- `m` = `scoreFormater`（`d3 as m`）。

**分发点 B（结果页回填）** — `chunks/useExamResult-d9fb4671.js` 的 switch：

```js
switch("collector.read"!==n.examType.exam_type_collect&&(I()||c.showScore(_,d,0)),n.examType.exam_type_collect){case"collector.3q5a":...break;case"collector.read":...break;case"collector.choose":setTimeout(()=>{const t=(o=n.htmlData,i=e,o.reduce((e,t)=>{const o=i.filter(e=>e.entity_id===t.info.stid).reduce((e,o)=>{const i=b(t.info.xtlist.find(e=>e.xt_xh===""+o.order)?.answer);return o.detail.choose?e.push(`${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}`):e.push(`${o.entity_id}_${o.order}${q}${i}`),e},[]);return e.concat(o)},[]));var o,i;c.setPCChoose.apply(null,t)},0);break;...
```

**分发点 C（族级资源加工）** — `index-5543d811.js`：`exam_type_collect` 只对 `collector.repeat` 做了特殊加工（选择族无族级加工）：

```js
async processPaperResources(){let e=[];this.totalScore=0;for(let n=0;n<this.paperList.length;n++){const t=this.paperList[n],r=await this.getJson(replacePathSeparator(`${this.getPackageId()}\\template_${t.template.id}\\res.json`));this.totalScore+=Number(r.set_score);const o=lodashExports.cloneDeep(r),i=await Promise.all(o.exam_type_list.map(async e=>{let n=0;return e.exam_list=await Promise.all(e.exam_list.map(async r=>{const o=t.contents.find(e=>`{{${e.code_id}}}`===r.exam_id);if(o&&(r={...r,exam_id:replaceRealId(r.exam_id,o.real_id),...o}),"collector.repeat"===e.exam_type_collect){...}})),e.exam_type_count=String(n),e.group_name=e.exam_type_name,{...t,...e}}));e.push(...i)}
```

### 1.2 数据来源

| 层 | 文件 | 字段路径 | 说明 |
|---|---|---|---|
| 题型族 | `template_<id>/res.json` | `exam_type_list[].exam_type_collect` = `"collector.choose"`；`exam_type_name` / `exam_type_score` / `exam_type_order` | 族判定 |
| 套题 | `res.json` | `exam_type_list[].exam_list[].{exam_id, real_id, exam_score, exam_count, exam_order}` | `real_id` 经 `t.contents[].code_id` 的 `{{code_id}}` 替换 |
| 小题元数据 | `res.json` | `exam_list[].question_list[].{question_order, question_score, question_count, evaluation_type, res_id, engine_area}` | 判分取分/取 `category` 用 |
| 题面 | `content_<real_id>/content.json` | `structure_type` = `"collector.choose"`，`info.{stid, st_sm, st_nr, audio, st_pic, xtlist}` | **题干与选项 + 标准答案** |
| 选项 | 同上 | `info.xtlist[].{xt_xh, xt_nr, xt_wj, xt_wj2, xt_value, xt_analy, xt_pic, answer, xxlist}`；`xxlist[].{xx_xh, xx_mc, xx_nr, xx_wj}` | |
| 步骤 | `template_<id>/ctrl.json`（或 `ctrl_check.json`） | `[].{subid, order, order_num, filetype, Extendedfield2/3, model_pc.jsparam}` | `filetype:"practise"` 触发判分 |
| 运行时 | — | `htmlData = contents[]`（`processPaperContentJson` 的产物） | 见下 |

`index-5543d811.js`（content.json 装载，逐字）：

```js
async processPaperContentJson(e,t){try{const n=e.exam_list.filter(e=>t?e.exam_id===t:e).map(async e=>{const t=await this.getJson(replacePathSeparator(`${this.getContentFilePath(e.real_id)}\\content.json`)),n=lodashExports.cloneDeep(t),r=JSON.parse((o=JSON.stringify(n),e.exam_info.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),o)));var o;const i=this.extractMp3Content(r.info,[]).reduce((t,n)=>(t[`${this.getPackageId()}_${r.info.stid}${n}`]=replacePathSeparator(`${this.UNZIP_LOCAL_PATH}\\${this.getPackageId()}\\content_${e.real_id}\\material\\${n}`),t),{}),a=replacePathSeparator(`${this.IMAGE_LOCAL_PATH}\\${this.getPackageId()}\\content_${e.real_id}\\material\\`);return{contents:r,paths:a,audios:i}}),r=await Promise.all(n),o=[],i={},a=[];return r.forEach(e=>{o.push(e.contents),a.push(e.paths),Object.assign(i,e.audios)}),{contents:o,paths:a,audios:i}}catch(n){throw n}}
```

- `contents[]` 每一项就是一个 `content.json` 全文（含 `structure_type` 与 `info`）。
- `paths` = 图片目录；`audios` 键 = `${packageId}_${info.stid}${mp3文件名}`。
- `exam_info` 是占位符替换表（`{id, value}[]`），把 `info` 里的所有 mp3/占位符解析成真实地址。

### 1.3 逐题抽出规则

**一题 = `content.json → info.xtlist[]` 的一个元素**（即"小题"）；选项 = 该元素的 `xxlist[]`。

**题号**：`xtlist[].xt_xh`（原文数字串，如 `"1"`）。运行时日志里的 `order` 与它相等。
**选项号**：`xxlist[].xx_xh`（`"1"`/`"2"`/`"3"`，**从 1 开始的下标串**）。

判分函数（`chunks/useExamStepper-8ad3fbd7.js`，逐字）：

```js
function X(e){const t={"‘":"'","’":"'","：":":","，":",","。":".","——":"-","—":"-",_:"-"};return(e=e.replace(/[‘’：，。———_]/g,e=>t[e])).toLowerCase()}async function G(e,t,a,o){const{getFileJson:n}=f(),{UNZIP_LOCAL_PATH:l}=d();let s=[],r=(await n(p(`${l}\\${e}\\content_${t}\\content.json`))).info.xtlist;try{s=o.split(",")}catch(i){}return s.map(e=>{const[t,o,n]=e.split("_");let l="0";if("0"===n)return{entity_id:t,order:o,score_detail:{total_score:l,choose:"",category:a.evaluation_type}};{const e=r.find(e=>e.xt_xh===o);if(!e)return null;const s=e.xxlist.find(e=>e.xx_xh==n);return s?(e.answer===s.xx_mc&&(l=m(a.question_score/a.question_count)),{entity_id:t,order:o,score_detail:{total_score:l,choose:s.xx_mc,choose_id:n,category:a.evaluation_type}}):null}}).filter(Boolean)}
```

拆解：
1. `o = y.getChoice()`，是 iframe 给出的**逗号分隔串**；每段 `"<entity_id>_<order>_<选项号>"`。
   多题/多空 → 多段。`s = o.split(",")`。
2. 每段 `e.split("_")` → `[entity_id, order, xx_xh]`。
3. `n === "0"` → **未作答**：`total_score:"0"`、`choose:""`（无 `choose_id`）。
4. `xtlist.find(x => x.xt_xh === order)` → 命中该小题。
5. `xxlist.find(x => x.xx_xh == 选项号)` → 命中所选选项。
6. 判分：`xt.answer === xx.xx_mc` → 正确则 `l = scoreFormater(question.question_score / question.question_count)`。
7. 输出：`{entity_id, order, score_detail:{total_score, choose: xx.xx_mc, choose_id: xx_xh, category: question.evaluation_type}}`。
   `question` = `_e.exam.question_list[0]`（**取该套题的第 0 个小题的 `evaluation_type` / `question_score` / `question_count` 作为整族参数**）。

### 1.4 答案在哪里（核心）

| 项 | 字段 | 形态 |
|---|---|---|
| **正确答案** | `content.json → info.xtlist[].answer` | **字母串**（`"B"` / `"C"`；多人多选可为 `"AC"`） |
| 学生选择（日志） | `score_detail.choose` = 所选选项的 `xxlist[].xx_mc` | **字母串**（与 `answer` 同域，因为判分是 `answer === xx_mc`） |
| 选项下标 | `score_detail.choose_id` = 所选选项的 `xxlist[].xx_xh` | **数字下标串**（`"1"`/`"2"`/`"3"`） |
| 选项文本 | `xxlist[].xx_nr` | 纯文本（客户端**不**用它判分，也不用它提交） |
| 是否服务端下发答案 | **否**。答案随 content.json 资源包本地下发，客户端本地判分 | 见 `G()` |

**真实官方样本（决定性旁证）** —— `Fe-pro/ets读取/北京初中resoure/4cd4f71f0e7b205b705b1ebedfe80ac5/content.json`：

```json
{"structure_type":"collector.choose",
 "info":{"stid":"729062","st_sm":"...","st_nr":"...","audio":"...","st_pic":"","xtlist":[
   {"xt_xh":"1","xt_nr":"...","xt_wj":"...","xt_wj2":"...","xt_value":"...","xt_analy":"...","xt_pic":"",
    "answer":"B",
    "xxlist":[{"xx_xh":"1","xx_mc":"A","xx_nr":" This Friday.","xx_wj":""},
              {"xx_xh":"2","xx_mc":"B","xx_nr":" Next week.","xx_wj":""},
              {"xx_xh":"3","xx_mc":"C","xx_nr":" Next month.","xx_wj":""}]},
   {"xt_xh":"2","answer":"C","xxlist":[{"xx_xh":"1","xx_mc":"A","xx_nr":" The food.","xx_wj":""},
              {"xx_xh":"2","xx_mc":"B","xx_nr":" The guests.","xx_wj":""},
              {"xx_xh":"3","xx_mc":"C","xx_nr":" The music.","xx_wj":""}]}]}}
```

另两个同构真样本：`a1a211102349900f381c4d6ff00d58c9`（answer=`C`/`B`）、`e0e718fa3547f95e42cf58e623a8767d`（answer=`B`/`B`）。

**Android 侧同名字段（旁证）** —— `jadx_template_info/sources/com/ets100/secondary/model/mock/InfoObjBean.java`：

```java
public static class CodeXxBean implements Serializable {
    @SerializedName("xx_mc") private String xxMc;      // 选项名称
    @SerializedName("xx_nr") private String xxNr;      // 选项内容
    @SerializedName("xx_wj") private String xxWj;
    @SerializedName("xx_xh") private String xxXh;      // 选项序号
```
`CodeXtBean` 的 `answer` 是**无 `@SerializedName` 的直名段**，即 JSON 里就叫 `answer`。

> **因此：`xx_mc` = 选项字母（A/B/C…），`xx_nr` = 选项文本，`answer` = 字母，`xx_xh` = 数字下标（1/2/3…）。**

**关于 `b()` 的关键辨析**（`chunks/useExamResult-d9fb4671.js`）：

```js
function b(e){if(!e)return"0";for(var t=e.toUpperCase(),o="",i=0;i<t.length;i++){var r=t.charCodeAt(i);if(r>=65&&r<=90)o+=(r-64).toString();else o+=t[i]}return o}
const q=String.fromCharCode(1);
```

`b("A")="1"`、`b("B")="2"`、`b("AC")="13"`。
它**只**出现在 `collector.choose` 的**结果页回填**分支里：

- `b(detail.choose)` —— 把日志里的字母转成下标，喂给 iframe 的 `setPCChoose`；
- `b(xtlist[].answer)` —— 同上，喂正确答案下标。

```js
const i=b(t.info.xtlist.find(e=>e.xt_xh===""+o.order)?.answer);
return o.detail.choose?e.push(`${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}`):e.push(`${o.entity_id}_${o.order}${q}${i}`)
```

> ⚠️ **`b()` 是"给 iframe 显示用的下标编码"，不是 HTTP 载荷形态。**
> 载荷里 `score_detail.choose` 永远是 **`xx_mc` 字母**（由 `G()` 写入），`score_detail.choose_id` 才是下标。
> `setPCChoose` 的参数格式：`"<entity_id>_<order>[_<学生答案下标>]\x01<正确答案下标>"`；`\x01` 是 `String.fromCharCode(1)`。

**回显另一个通道（`setPCChoose2`）** — `chunks/useExamStepper-8ad3fbd7.js`：

```js
a.setData(s,l,e,"",n,1==v),clearTimeout(me.value),me.value=setTimeout(()=>{if(1==t.displayAnswerContent&&y.setPCChoose2){const e=o.map(e=>`${e.entity_id}_${e.order}_${e.choose_id}`);y.setPCChoose2.apply(null,e),r()}},500)}
```
即回填学生答案时用的是 `choose_id`（下标），格式 `"<entity_id>_<order>_<xx_xh>"`，随后立刻 `r()` 重新 `getChoice()` 并重算分。

**"未作答"的形态**：`xx_xh === "0"`（iframe 约定），此时 `choose:""` 且 `total_score:"0"`，**无 `choose_id` 键**。

### 1.5 分值

| 层级 | 来源 | 计算 |
|---|---|---|
| 试卷总分 | `res.json → set_score` | `this.totalScore += Number(r.set_score)`（累加所有 paper） |
| 题型族满分 | `exam_type_list[].exam_type_score` | 直接字段 |
| 套题满分 | `exam_list[].exam_score`；题数 `exam_count` | 直接字段（`collector.repeat` 才重算） |
| 单题满分 | `exam_list[].question_list[].question_score` | 直接字段 |
| 单题分空数 | `exam_list[].question_list[].question_count` | 直接字段 |
| **选择题每题得分** | — | `scoreFormater(question_score / question_count)`，`question = _e.exam.question_list[0]` |
| 题型族总分（结果页） | — | `calculateAndFormatScoresWithexamType`（`E()`）按 `standard_score`/`real_score` 累加后 `scoreFormater` |
| 显示用 `[得分,分母]` | — | `scoreFilter(exam_type_score, totalScore, isHomework)` ⇒ `[isHomework? scoreFormater(total): ConvertToPercentScore(total, exam_type_score, 0), isHomework? exam_type_score: 100]` |

```js
const useExamScore=()=>{const e=[FirstColumnType.SynchronousListening,FirstColumnType.ReadingWriting,FirstColumnType.ListeningSpeakingSpecial,FirstColumnType.SpecialTraining,FirstColumnType.SimulationExam,FirstColumnType.ListeningSimulation,FirstColumnType.ReadingWritingSimulation];function t(t,n){return t&&e.every(e=>e!=t)||n&&5!=n}return{scoreFilter:function(e,t,n){return[n?scoreFormater(t):ConvertToPercentScore(t,e,0),n?e:100]},scoreFilterHomework:function(e,n,r,o,i){return[t(r,o)?scoreFormater(n):ConvertToPercentScore(n,e,0),i?e:100]},percentageQuestion:t}};
```
```js
function scoreFormater(e,t=2){let n=Number(e)*Math.pow(10,t);return Math.round(Number(n))/Math.pow(10,t)}
function ConvertToPercentScore(e,t,n=0){return e&&t?scoreFormater(Number(e)/Number(t)*100,n):0}
```

> 注意：`G()` 用的是 **`question_score / question_count`**（"该题满分 ÷ 空数/小题数"），
> **不是** `exam_score / exam_count`。

### 1.6 提交时每题的标识字段

作答日志对象（`chunks/Examine-d229829d.js` 的 `chooseHandler`，与 `MockExamDetails` 同构）：

```js
Ze.value.chooseHandler(async e=>{const a=K.getInstance().handleRequestParams({resource_id:Oe,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:je,entity_id:e.entity_id,order:e.order,serial_id:R.serial_id,upload_file_id:"",upload_file_url:"",detail_file_id:"",detail_file_url:"",choose_id:e.score_detail.choose_id,score_detail:e.score_detail});a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score,ka({detail_file:"",audio:"",detail:a.score_detail,score_detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,choose_id:a.score_detail.choose_id,standard_score:""});const l=`${a.entity_id}_${a.order}`;da.value.forEach(e=>{const t=e.questions.find(e=>e.id==l);t&&(t.answer=void 0!==a.score_detail.choose_id&&""!=a.score_detail.choose_id)})})
```

`index-5543d811.js`：
```js
handleRequestParams:e=>(e.exam_id=null,{...e,homework_id:a.value})
```

日志 → HTTP 的最终装配（`index-5543d811.js`，`requestAnswerLogs` = bundle 导出 `fr`）：

```js
const requestAnswerLogs=async e=>{const{ecardInfo:t}=useUserInfoStore$1(),n=useSpecialStore(),{first_column_id:r,question_type_score:o}=e;let i={};try{i="string"==typeof e.score_detail?JSON.parse(e.score_detail):e.score_detail}catch(e3){}const{total_score_init:a,total_score:s}=i||{},l=a||s,c=await n.getColumnsNeed125(r);c&&t?.rate_scale&&l&&o&&(e.real_score=scoreFormatter_125(o/5*l));const u=c?t?.rate_scale:void 0,d=c&&u?"0.125":void 0;i.rate_scale=u||0,delete i.total_score_init,delete i.xml_score,delete i.question_type_score,delete i.is_rejected,delete i.except_info,delete i.first_column_id,e.score_detail=JSON.stringify(i);const p=Object.assign(e,{rate_scale:u,graduation:d,client_time:currentTimestampToDate(),mark_service:1,version:"2"});return await hyRequest.postIt("/m/audio/sync-v2",Object.assign(p))},
```

`chunks/useAudioSync-0603b875.js`（默认字段全集，逐字）：

```js
const i={resource_id:"",real_score:0,score:0,set_id:"",entity_id:"",order:"",serial_id:"",mistake_id:0,upload_file_id:"",upload_file_url:"",spx_file_url:"",mark_trace_id:"",detail_file_id:"",detail_file_url:"",score_detail:""};
```

**选择题日志的字段清单**：

| 字段 | 值 | 来源 |
|---|---|---|
| `entity_id` | content.json 的 `info.stid` | `G()` 第 1 段 |
| `order` | `xtlist[].xt_xh` | `G()` 第 2 段 |
| `set_id` | `examType.super_set_id \|\| set_id` | `G(第一个实参)` |
| `choose_id` | 所选选项 `xx_xh` | `score_detail.choose_id` |
| `score_detail`（JSON 串） | `{total_score, choose, choose_id, category}` | `G()` |
| `first_column_id` | 来自 `ctrl.json` 步骤（读写=10） | `e.first_column_id=g` |
| `question_type_score` | `_e.exam.exam_score` | `e.question_type_score=_e.exam.exam_score` |
| `real_score` / `score` | `score_detail.real_score ?? score_detail.total_score` / `score_detail.total_score` | `chooseHandler` |
| `resource_id` / `serial_id` / `mistake_id` / `homework_id` | 页面 query | `chooseHandler` / `handleRequestParams` |
| `version` `client_time` `mark_service` `rate_scale` `graduation` | `"2"` / 服务器时间串 / `1` / 倍率 / `"0.125"`或缺省 | `requestAnswerLogs` |
| 提交端点 | **`/m/audio/sync-v2`**（作业另走 `m/homework/submit`，自由练习 `/m/exam/submit`，读写同步 `/m/exam/sync`） | `hyRequest.postIt` |

```js
requestHomeworkSubmit=async e=>await hyRequest.postIt("m/homework/submit",e),...
requestGetReadingWritingScoreDetail=async e=>await hyRequest.postIt("/m/exam/get-score-detail",e),requestPostReadingWritingScore=async e=>await hyRequest.postIt("/m/exam/sync",e),requestSubmitReadingWritingExam=async e=>await hyRequest.postIt("/m/exam/submit",e),...
```

### 1.7 与其它族的差异

- `choose` **不做族级资源重算**（只有 `repeat` 会读 `info_extra.json` 扩题）；
- `choose` 是**唯一**走 `getChoice()`/`ReturnChoose`/`setPCChoose`/`setPCChoose2` 这条"选项下标"通道的族；
- 与 `fill` 的差别：`choose` 的 `score_detail` **有 `choose_id`**，`fill` **没有**；`choose` 的 `choose` 是字母，`fill` 的 `choose` 是自由文本；
- `collector.read` 且 `exam_list.length > 1` 时改走 `set3q5aScore`；`3q5a/role/dialogue/word/repeat` 走 `set3q5aScore`；`picture` 在 switch 里**没有 case**；
- `filetype !== "practise"` 时完全不进入本族逻辑（改走 `action.record` / `file.audio` / `file.video` 等步骤处理器）。

---

## 2. `collector.fill`（填空类）

### 2.1 族判定与分发点

同一个 `practise` 处理器里的第二个 `if`（`chunks/useExamStepper-8ad3fbd7.js`，逐字）：

```js
"collector.fill"===w.value.exam_type_collect&&(y.returnPcBlank=async t=>{const a=await async function(t,a,o,n){const{getFileJson:l}=f(),{UNZIP_LOCAL_PATH:s}=d(),r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std,i=JSON.parse(n);try{return i.map(e=>{const[t,a]=e.id.split("_"),n=e.answer;let l="0";if(""===n)return{entity_id:t,order:a,score_detail:{total_score:l,choose:"",category:o.evaluation_type}};{const e=r.find(e=>e.xth===a);return s=X(n),X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),{entity_id:t,order:a,score_detail:{total_score:l,choose:n,category:o.evaluation_type}}}var s})}catch(e){return[]}}(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_fill"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:U.getInstance().getIsHomework()?1:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}},y.getPcBlank())}
```

**结果页回填**（`chunks/useExamResult-d9fb4671.js`）：

```js
case"collector.fill":setTimeout(()=>{const t=(o=n.htmlData,i=e,o.reduce((e,t)=>{const o=i.filter(e=>e.entity_id===t.info.stid).reduce((e,t)=>(t.detail.choose?e.push({id:`${t.entity_id}_${t.order}`,answer:t.detail.choose,status:((t.real_score||t.score_rate)??0)>0?1:0}):e.push({id:`${t.entity_id}_${t.order}`,answer:"",status:0}),e),[]);return e=e.concat(o),JSON.stringify(e)},[]));var o,i;c.setPcBlank(t)},0)}
```

> 注意 `fill` 的桥接函数是 **`getPcBlank()` / `returnPcBlank` / `setPcBlank`**（Pc* 与 PC 的大小写不同）：
> - `getPcBlank()` — iframe 交出**学生填的答案**（JSON 串）；
> - `returnPcBlank = async t => {...}` — 宿主把判分结果交回 iframe；
> - `setPcBlank(json)` — 结果页把"每题 `{id, answer, status}`"喂回 iframe 回显。

### 2.2 数据来源

| 层 | 文件 | 字段 |
|---|---|---|
| 题型族 | `res.json → exam_type_list[].exam_type_collect` = `"collector.fill"` | |
| 题面 | `content_<real_id>/content.json` | `structure_type` = `"collector.fill"`，`info.{stid, ...}` |
| **标准答案** | `content.json → info.std[]` | 每项 `{xth, value}`；`xth` = 空号/题号，`value` = 答案（`/` 分隔多解） |
| 回显开关 | `info.json → model_pc.jsparam` 解析出的 `code_id` 名 | `content_fill_answer_enable`、`show_topic_blank` |

判分读取路径逐字：`` r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std `` —— **`info.std`**。

`content_fill_answer_enable` 注入（`chunks/useExamStepper-8ad3fbd7.js`）：

```js
O.on(t=>{const a=t.win;y=a;const o=t.logs??[];C.value=o;const n=t.isMock??!1,l=JSON.stringify([p(`${z}\\${i.length>0?i:e}\\content_${fe.value?.exam?.real_id}\\material\\`)]),s=B.cloneDeep(_e.model_pc.extractedJsParam);if(a.setData){if(s.hasOwnProperty("content_fill_answer_enable"))try{if("string"==typeof s.content_fill_answer_enable){const e=JSON.parse(s.content_fill_answer_enable);e.forEach(e=>{const t=o.find(t=>String(t.entity_id)===e.stid&&String(t.order)===e.xth);t&&t.detail&&(e.answer=("string"==typeof t.detail?JSON.parse(t.detail):t.detail)?.choose)}),s.content_fill_answer_enable=JSON.stringify(e),s.show_topic_blank="1"}}catch(u){}...
```

**这一点极其关键**：`content_fill_answer_enable` 是一个 `info.json` 里以 `code_id` 命名的"填空答案载体"，其单项结构是 **`{stid, xth, answer}`** ——
- `stid` 对 `log.entity_id`
- `xth` 对 `log.order`（也正是 `info.std[].xth`）
- `answer` 被写成日志里的 `detail.choose`（学生答案）
- 随后 `s.show_topic_blank="1"` ⇒ 让 iframe 在题面里显示填空结果。

`htmlData` 里的 `_e.model_pc.extractedJsParam` 来自 `index-5543d811.js::processJsParam`：

```js
async processJsParam(e,t,n){try{const r=n?.template?.id,o={};for(const n of e){const e=replacePathSeparator(`${this.generateFilePath(n.source,t.real_id,r)}\\info.json`),i=await this.getJson(e),a=lodashExports.cloneDeep(i).find(e=>e.code_id===n.code_id),s=t.exam_info,l=e=>s.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),e);if(a){let e;e=["txt","txxt"].includes(a.code_type)?l(a.code_value):"json_array"===a.code_type?l(a.code_json_array):"json_obj"===a.code_type?l(a.code_json_obj):"image"===a.code_type?[{url:...}]:"";o[n.classname]=e}}return o.content_highlight&&(o.content=o.content.replace(o.content_highlight,`<span class="color_repeat">${o.content_highlight}</span>`)),o}catch(r){throw r}}
```

> 即 `model_pc.jsparam[]` 的每项 `{source, classname, code_id, code_type}` 从
> `info.json` 里按 `code_id` 取出 `code_value` / `code_json_array` / `code_json_obj` / `code_type:"image"`，
> 按 `code_type` 解析成字符串/数组/对象，落到 `extractedJsParam[classname]`。
> **所以 `content_fill_answer_enable` 与 `show_topic_blank` 都是 info.json 的 code_id，运行期才知道具体内容。**

### 2.3 逐题抽出规则

**一空/一题 = 一个 `<entity_id>_<order>` 对**：

1. iframe 交出 `y.getPcBlank()` → JSON 串，形如 `[{"id":"<entity_id>_<order>","answer":"<学生文本>"}, ...]`（`e.id.split("_")` → `[entity_id, order]`，说明 id 恰有两段）。
2. `JSON.parse(n)` → `i`；`i.map(...)` 逐空处理。
3. `order` 与 `content.json → info.std[].xth` 配对（`r.find(e=>e.xth===a)`，注意这里是 **`===`** 严格比较，而 `choose` 用 `==`）。
4. 空答案（`answer === ""`）→ `{entity_id, order, score_detail:{total_score:"0", choose:"", category}}`。
5. 否则：`s = X(学生答案)`；`X(std.value).split("/").map(t=>t.trim().toLowerCase()).includes(s)` → 命中即得 `scoreFormater(question_score / question_count)`。
6. 输出：`{entity_id, order, score_detail:{total_score, choose:<学生原文>, category}}` —— **没有 `choose_id`**。

归一化 `X()`（同 §1.3）把中文全角标点、长破折号、下划线都折叠掉：
```js
function X(e){const t={"‘":"'","’":"'","：":":","，":",","。":".","——":"-","—":"-",_:"-"};return(e=e.replace(/[‘’：，。———_]/g,e=>t[e])).toLowerCase()}
```

### 2.4 答案在哪里（核心）

| 项 | 字段 | 形态 |
|---|---|---|
| **正确答案** | `content.json → info.std[].value` | **纯文本**；支持 `/` 分隔多解（逐个 `trim().toLowerCase()` 后比较） |
| 空号 | `info.std[].xth` | 字符串题号（与 `log.order` 相等） |
| 学生答案 | `score_detail.choose`（= iframe `getPcBlank()` 里的 `answer`） | **纯文本原文**（不做归一化入库） |
| 是否服务端下发 | **否**，本地判分 | `returnPcBlank` 闭包 |
| 是否有 `choose_id` | **无** | 与 `choose` 的硬差异 |

**完全不存在**任何"字母 / 下标"语义：`fill` 的答案既不查 `xtlist`，也不用 `b()`。

**旁证（另一代读写 iframe 模板应用）**：`Fe-pro/ets读取/广东初中resoure/common/js/rw-index-2.js` 第 309 行：

```js
slashToArr: function(str) {
    return str.replace(/<br \/>/g, '').split('/')
},
```
与 `rw-index-2.html` 第 85/147 行 `v-for='(standItem,standIdx) in slashToArr(subItem.answer)'` ——
同一产品另一代实现里，标准答案也是 `answer` 字段 + `<br />` 清洗 + **`/` 分隔多解**，
与 PC 端 `X(value).split("/")` 语义一致。

> **本地未找到真实 `collector.fill` 的官方 content.json 样本**（已查路径与关键词见 §6），
> 因此 `info.std[].value` 的字段名**直接来自前端代码**，而非样本。

### 2.5 分值

| 层级 | 来源 |
|---|---|
| 每题得分 | `scoreFormater(question.question_score / question.question_count)`，`question = _e.exam.question_list[0]` |
| 埋点总分 | `a.reduce((e,t)=>e+m(t.score_detail.total_score),0)`，分母 `m(_e.exam.question_list[0].question_score)` |
| 结果页 `status` | `(real_score \|\| score_rate) > 0 ? 1 : 0`（仅"对/错"标记，非分数） |
| 族/套题/试卷满分 | 同 §1.5（`exam_type_score` / `exam_score` / `set_score`） |

### 2.6 提交时每题的标识字段

与 `choose` **同一条**上报通道，差别只在 `score_detail` 的键：

| 字段 | 值 |
|---|---|
| `entity_id` | `info.stid` |
| `order` | 空号，等于 `info.std[].xth` / iframe `id` 的第二段 |
| `set_id` | `super_set_id \|\| set_id` |
| `score_detail`（JSON 串） | `{total_score, choose, category}` —— **无 `choose_id`** |
| `choose` | 学生原文（**不是**模板答案） |
| `category` | `question.evaluation_type` |
| `first_column_id` / `question_type_score` | `ctrl.json` 步骤 / `exam.exam_score` |
| `resource_id` `serial_id` `mistake_id` `homework_id` `version` `client_time` `mark_service` | 同 §1.6 |
| 端点 | `/m/audio/sync-v2`（作业 `m/homework/submit`） |

回填到 iframe 的形状（`setPcBlank` 的实参，逐字取自 useExamResult）：

```
[{"id":"<entity_id>_<order>","answer":"<学生答案或\"\">","status":0|1}, ...]
```

### 2.7 与其它族的差异

- **`fill` 无 `choose_id`**（服务端如需"选第几个"的语义将不可用）；
- `fill` 的 `xtlist`/`xxlist` 一概不涉及；用 `info.std`；
- `fill` 的判分是**文本归一化 + 多解匹配**，`choose` 是**字母相等**；
- `fill` 的 `order` 用 `===` 匹配 `xth`（字符串严格相等），`choose` 用 `==` 匹配 `xx_xh`（宽松，兼容数字/字符串）；
- `fill` 在结果页额外走 `setPcBlank` 回显（`choose` 走 `setPCChoose2`），二者是**不同的桥接通道**。

---

## 3. `collector.picture`（图片/看题说话类）

### 3.1 族判定与分发点

**没有独立分支。** `collector.picture` 在 PC 前端只出现 2 次，且都是与 `collector.read` 合并：

`chunks/useExamResult-d9fb4671.js`（`getRecordAudios`）：
```js
case"collector.read":case"collector.picture":e=`${D}_${r.info.stid}_1`,a=t.find(e=>e.entity_id===r.info.stid&&"1"===String(e.order)),s=a&&await w({set_id:D,entity_id:r.info.stid,order:1,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;
```

同文件（`getAudioIds`）：
```js
getAudioIds:function(e){let t={original:"",playback:""};if(e.length){const o=e[0];switch(t.original=`${D}_${o.info.stid}${o.info.audio}`,o.structure_type){case"collector.read":case"collector.picture":t.playback=`${D}_${o.info.stid}_1`;break;case"collector.word":t.playback=`${D}_${o.info.stid}_${o.info.xh}`}}return t},
```

`useExamResult` 的 `switch(n.examType.exam_type_collect)` 里**没有 `case "collector.picture"`** ——
即结果页**不会**为 picture 调 `setPCChoose` / `setPcBlank` / `set3q5aScore` 中的任何一个。
（`"collector.read"` 的通用前置 `I()||c.showScore(_,d,0)` 也不覆盖 picture。）

### 3.2 数据来源

| 层 | 文件 | 字段 |
|---|---|---|
| 族 | `res.json → exam_type_list[].exam_type_collect` = `"collector.picture"` | |
| 内容 | `content_<real_id>/content.json` | `structure_type` = `"collector.picture"` |
| 客户端实际读取 | `info.stid`、`info.audio` | 只此两项 |
| 客户端**未**读取（但真实存在） | `info.{value, image, video, analyze, std, ref, keypoint, topic}` | |

**真实官方样本（旁证）** —— `ets_resolver/tests/corpus/guangdong_mock1/05_信息转述/content.json`（`meta.json` 记 `"type":"collector.picture"`，`"group_name":"信息转述"`，来源=官方云端作业真实下载包）：

```
structure_type: collector.picture
info keys: ['stid', 'value', 'image', 'audio', 'analyze', 'std', 'ref', 'keypoint', 'topic']
example: stid=49065, image="content.jpg", audio="content.mp3"
info.value   = "</p><p>Good afternoon, boys and girls. Here's a notice. ..."      ← 听力材料原文
info.std     = [{"value":"</p><p>Here's the notice about our school trip. ..."}]   ← 参考转述文本
info.topic   = "The School Trip"
info.keypoint= "1. Here's the notice about our school trip. </br>2. We're going to visit the Space Museum ..."
info.ref     = []
```

同构真样本另有 `tests/corpus/guangdong_hs_mock1/02_故事复述/`（`collector.picture`）、
`Fe-pro/ets读取/北京初中resoure/01087533a728f9cd941b5b2efe99eab3/`、
`Fe-pro/ets读取/广东初中resoure/2ec447e23123d218d23e48a4b5e6538d/`（`collector.picture`）。

### 3.3 逐题抽出规则

- **一题 = 一个 `content.json`（即一个 `exam_list` 条目）**，无子题数组（对比 3q5a 的 `info.question[]`）。
- **题号固定为字符串 `"1"`**：`e=\`${D}_${r.info.stid}_1\``、`order:1`、`String(e.order) === "1"`。
- 关键字段：`info.stid`（题 ID）、`info.audio`（原始音频文件名）、`info.image`、`info.value`。
- 音频键：
  - 原声：`` `${D}_${o.info.stid}${o.info.audio}` ``（注意 `stid` 与文件名之间**无**下划线）
  - 学生录音：`` `${D}_${o.info.stid}_1` ``
  其中 `D` = iframe 侧传入的 `setData(htmlData, paths, D, ...)` 里的 `D`（也就是页面的 set/package id）。

### 3.4 答案在哪里（核心）

**客户端不取答案、不判分。** 明确结论与证据：

1. `collector.picture` 在整个 PC 前端（bundle + 82 chunks）只出现 2 次，均在 `getRecordAudios` / `getAudioIds` 里，**只用于定位音频**，从未读取任何 `answer` / `std` / `correct` 字段。
2. `content.json → info.std` / `info.ref` / `info.keypoint` / `info.topic` 在真实样本中确实存在，
   但**前端代码中不存在任何读取点**（`grep -n 'info.std' / '\.std' / 'ref' / 'keypoint' / 'topic'` 在 bundle 与全部 chunk 中命中 0；`info.std` 仅出现在 `fill` 分支）。
3. 判分链完全走**语音评测引擎**：`chunks/useRecordResult-fbe9e75b.js` 里客户端只做

```js
let{xml_score:C,accuracy_score:$,fluency_score:b,standard_score:O,integrity_score:Z,is_rejected:F,except_info:k}=R; ...
x=t.sdk==s.AI?"m_platform_engine":"m_aicloud_engine",h&&(G=n(h,w));
const I={...R,total_score:_(w),total_score_init:w,real_score:G,category:e,accuracy_score:$,fluency_score:b,standard_score:O,first_column_id:p,question_type_score:h};
... const _={sdk_recode:t.code,engine_recode:k,total_score:C,is_rejected:F,sid:t.sid,channel_type:t.data.channelType,wav_url:a,spx_url:c,mark_trace_id:t?.traceId||t?.sid,xml_url:l};
```
   得分 `total_score` 来自引擎 XML（`parseIseResult` / `parseEtsOpenIseResult`，bundle 导出 `fo` / `fp`）。
4. 因此：**`collector.picture` 是开放式口语题，答案（`info.std` 参考文本）由服务端/评测引擎使用，客户端不取、不判、不提交答案文本。**

**答案取值形态：不适用 / 未找到**（无选项，无本地答案字段）。

### 3.5 分值

| 层级 | 来源 |
|---|---|
| 试卷/族/套题满分 | 同 §1.5（`set_score` / `exam_type_score` / `exam_score`） |
| 单题满分 | `exam_list[].question_list[0].question_score`；录音步骤把它作为 `question_type_score` 送评测：`{"entity_id":o.exam.exam_id,"order":o.question.question_order,"first_column_id":g,"question_type_score":o.question.question_score}` |
| 客户端侧计算 | **无本地计算**（picture 不在 switch 的任一 case 内）；总分由 `calculateAndFormatScoresWithExamType`（`E()`）按回传的 `standard_score`/`real_score` 累加 |
| 5 分制换算 | `calculateRealScore(e,t)=scoreFormater(Number(e)/5*Number(t))`；`calculateRealScoreWithCategory` 仅对 `EvaluationTypes` 生效 |

```js
const EvaluationTypes=["read_word","read_chapter","read_sentence","read_choice","simple_expression_ufi","simple_expression_ufk","simple_expression_ufj","topic","simple_expression","free_expression"];
```
（`picture` 的 `evaluation_type` 是否落在这 10 个里，取决于服务端返回的 `question_list[].evaluation_type`；前端不写死族名。）

### 3.6 提交时每题的标识字段

录音步骤（`chunks/useExamStepper-8ad3fbd7.js`，`"action.record"` 处理器）构造的日志参数：

```js
"action.record":async t=>{ue.value=!0;...const o=B.cloneDeep(t);if(o&&o.question){const{pprInfo:t}=o.question||{}, {engine_type:n}=t||{}, l={record_time:a,category:o.question.evaluation_type,res_id:o.question.res_id,region:o.question?.engine_area,recordStep:o,set_id:e,engine_type:n,pprInfo:t,first_column_id:g};...const r=()=>{const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};...
```

| 字段 | 值 |
|---|---|
| `entity_id` | `exam.exam_id`（= content.json 的 `info.stid`，因 `real_id` 替换过） |
| `order` | `"1"`（固定） |
| `set_id` | 页面 set_id / `super_set_id` |
| `category` | `question.evaluation_type` |
| `res_id` / `region` | `question.res_id` / `question.engine_area` |
| `engine_type` | `question.pprInfo.engine_type` |
| `question_type_score` | `question.question_score` |
| 音频键 | 录音 `${D}_${stid}_1`；原声 `${D}_${stid}${info.audio}` |
| 端点 | `/m/audio/sync-v2`（`requestAnswerLogs`），另加 `wav_url`/`spx_url`/`xml_url`/`mark_trace_id`/`sid`/`channel_type` |

### 3.7 与其它族的差异

- **`picture` 在结果页 switch 中没有 case** → 不参与任何客户端分数回填；
- 与 `read` 的区别：`read` 单题时会读 `info.value` 并调 `adjustWordColor`（`$(w,c,{...})`）做逐句/多维展示，`picture` 没有这一步；
- 与 `3q5a/role/dialogue` 的区别：后者有 `info.question[]` 子题数组（多 order），`picture` 只有单 order `"1"`；
- 与 `read` 的共同点：`getRecordAudios` / `getAudioIds` 里同组处理（音频键规则完全一致）。

---

## 4. `collector.3q5a`（三问五答 / 角色扮演）

### 4.1 命名含义：代码里查不到，语料里可定案

**前端源码层：未找到。**

```bash
$ grep -c '3q5a' index-5543d811.js                       # 0
$ grep -ril '三问\|问答\|threeQ\|fiveAnswer\|3Q5A' pc_client_web/   # 0
```
（`index-5543d811.js` 里唯一 1 处 `q5a` 命中是 base64 图片数据里的巧合子串）

**结构层 + 官方语料层：可定案 = 「三问五答」，即高考听说「角色扮演」。**

官方真实资源包（`ets_resolver/tests/corpus/guangdong_hs_mock1/01_角色扮演/`，来源标注为"ETS100 官方云端作业真实下载包（广东高中账号）"）：

```json
// meta.json
{"region":"广东高中（高考听说 60 分卷）","type":"collector.3q5a","group_name":"角色扮演",
 "source":"ETS100 官方云端作业真实下载包（广东高中账号）；ZIP 经官方 MSTCHINA 尾部+64位密码解密；音频不入库"}
```

```json
// content.json
{"structure_type":"collector.3q5a",
 "info":{"question":[...8 项...],"image":"...","video":"","value":"...","stid":"13606","audio":"content.mp3"}}
// info.question[] 每项 keys:
["std","ref","ask","answer","askaudio","aswaudio","xh","analyze","sucai","keywords","role"]
// xh   = ["1","2","3","4","5","6","7","8"]
// role = ["a","a","a","b","b","b","b","b"]      ← 3×a + 5×b  ⇒「三问五答」
// 前 3 项 ask 为中文提示（学生要说出英文问句）：
//   "你为什么不和父母在一起？" / "你每个月有多少零花钱呢？" / "你父母知道你跑出来了吗？"
// 后 5 项 ask 为英文问题（学生要作答）：
//   "How much does Ali pay fo..." / "Whose jacket is Ali wear..." / ...
```

逐项字段含义（同一真样本）：

```json
{"std":[{"value":"Why don’t you stay with your parents?","ai":"...","audio":"ques1Std1.mp3"}, ... 共 6~16 个变体],
 "ref":[],
 "ask":"你为什么不和父母在一起？",
 "answer":"Because they are visiting a modern art exhibition. I know nothing about it and I think it’s boring.",
 "askaudio":"",
 "aswaudio":"ques1aswaudio.mp3",
 "xh":"1",
 "analyze":"",
 "sucai":"ques1Std1.mp3",
 "keywords":"Why don’t stay parents|why aren’t parents|...",
 "role":"a"}
```

> **结论（三层证据）**：
> 1. `3q5a` = 字面「3 questions + 5 answers」；
> 2. 官方语料里 `collector.3q5a` ↔ `group_name:"角色扮演"`，`info.question[]` 恰好 `role` 为 3×`a` + 5×`b`，8 个子题；
> 3. 前端把 `collector.3q5a` 与 `collector.role` / `collector.dialogue` 并列（同一 `info.question[]` 结构、同一条判分通道）。
>    两者的**结构完全相同**，差别只在题目出处（`collector.role` 见广东初中"听选信息/回答问题/询问信息"；`collector.3q5a` 见广东高中"角色扮演"）。
> 4. 旁证：`chunks/exam-6ff42190.js` 里存在 `Guangdong=1` 的地区枚举 `P`（`var e=(e=>(e[e.Guangdong=1]="Guangdong",...))`），说明广东系题型是官方一等公民。

### 4.2 族判定与分发点

`chunks/useExamResult-d9fb4671.js` switch 的第一个 case 组：

```js
case"collector.3q5a":case"collector.role":case"collector.dialogue":case"collector.word":case"collector.repeat":I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);break;
```

`getRecordAudios`（子题音频键）：

```js
case"collector.3q5a":case"collector.role":case"collector.dialogue":for(const n of r.info.question)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:n.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;
```

`chunks/useExamStepper-8ad3fbd7.js` 的 `"action.record"` 步骤负责录音（见 §3.6），族名不参与该步骤判定。

### 4.3 逐题抽出规则

- **一题（一个 log）= `info.question[]` 的一个元素**（子题）。
- **题号 = `info.question[].xh`**（`"1"`…`"8"`），与 iframe 交回的 `order` 相等（`String(e.order) === n.xh`）。
- **`entity_id` = `info.stid`**（整个 content.json 一个 stid，8 个子题共用）。
- 每个子题的关键字段：`xh` / `role` / `ask` / `answer` / `std[]` / `askaudio` / `aswaudio` / `sucai` / `keywords` / `analyze` / `ref`。
- 录音键：`${D}_${stid}_${xh}`（与 picture/read 的 `_1` 形成对比）。

### 4.4 答案在哪里（核心）

**客户端不取答案、不判分。**

证据：
1. `collector.3q5a` 只出现在两个位置：结果页 switch（→ `set3q5aScore`）与 `getRecordAudios`。二者都不读 `answer` / `std`。
2. `set3q5aScore` 的参数里**只有分数**，没有任何答案字段（见 §4.5 的 `j()`）。
3. 每子题都是 `action.record` 录音步骤 → `useRecordResult` → 语音评测引擎（同 §3.4）。
4. `info.question[].std[]` / `.answer` / `.keywords` / `.ref` **在 PC 前端无读取点**（`grep` 在 bundle + 全部 chunk 中命中 0）。

> **`info.question[].std[]`（6~16 个可接受答法变体）、`answer`（参考答句）、`keywords`（关键词串）都是给服务端评测/答案页用的，客户端不取。**

**答案取值形态：不适用 / 未找到**。

### 4.5 分值

结果页的分数拼装函数 `j()`（`chunks/useExamResult-d9fb4671.js`，逐字）：

```js
function O(e,t,o,i){return t=t||1,o?e:h(e*t,i>1?2:0)}
function j(e,t,o,i){return t.reduce((t,r)=>{let s=o.filter(e=>e.entity_id===r.info.stid),n=0;const c=s.length,l=s.reduce((t,o)=>{n++;const r=function(e,t){const o=e.exam_list.find(e=>e.exam_id===t.entity_id);return o?o.question_list.find(e=>e.question_order===String(t.order)):{}}(e,o),{scoreFilter:s}=a(),[l,d]=s(r.question_score,o.real_score,i);let _=l,u=d;if(100==u&&c>1){let t=Number(r.question_score)/Number(e.exam_type_score);if(_=O(l,t,i,c),u=O(d,t,i,c),n==c){const t=u;u=(i?e.exam_type_score:100)-u*(c-1),_=_*u/t,_=h(_,i)}}const m=`${D}_${o.entity_id}_${o.order}${q}${_??0}${q}${u}${q}0`;return t.push(m),t},[]);return t=t.concat(l)},[])}
```

拆解：
1. `e` = `examType`，`t` = `htmlData`（contents 数组），`o` = 日志数组，`i` = 是否百分比计分。
2. 对每个 content 项，取 `entity_id === info.stid` 的所有日志（**该 content 下的所有子题日志**）。
3. 每个子题的满分来自 `exam_list.find(exam_id === entity_id).question_list.find(question_order === String(order)).question_score`。
   ⇒ **子题题号 `xh` 必须能在 `res.json` 的 `question_list[].question_order` 里找到，否则 `r = {}`、`question_score` 为 `undefined`。**
4. `scoreFilter(question_score, real_score, isPercentage)` → `[显示分, 分母]`。
5. 若 `分母 == 100` 且子题数 `> 1`，做"按 `question_score/exam_type_score` 比例 + 末题吸收余数"的再分配（`O()`）。
6. 拼串格式：`` `${D}_${o.entity_id}_${o.order}\x01${显示分}\x01${分母}\x01 0` ``，用 `apply(null, t)` 展开成多参传给 iframe 的 `set3q5aScore`。

| 项 | 规则 |
|---|---|
| 子题满分 | `exam_list[].question_list[].question_score`（按 `question_order == xh` 找） |
| 子题得分 | `scoreFilter` 换算（百分比制 → `ConvertToPercentScore(real_score, question_score, 0)`；否则 `scoreFormater(real_score)`） |
| 族总分 | `calculateAndFormatScoresWithExamType`（`E()`）；`exam_type_score` 为分母 |
| 试卷总分 | `res.json → set_score` 累加 |
| 结果页显示 | `showScore(分母, 显示分, 0)`；`set3q5aScore(...)` |

> 注意 `q` = `String.fromCharCode(1)`（`\x01`）是这里的分隔符，`setPCChoose` 用的也是它。

### 4.6 提交时每题的标识字段

| 字段 | 值 |
|---|---|
| `entity_id` | `info.stid`（content.json 级，8 个子题共用） |
| `order` | `info.question[].xh`（`"1"`…`"8"`） |
| `set_id` | 页面 set_id / `super_set_id` |
| `question_type_score` | `exam.exam_score`（`x.trigger` 时统一附加） |
| `first_column_id` | `ctrl.json` 步骤的 `first_column_id`（`e.first_column_id=g`） |
| `category` / `res_id` / `region` / `engine_type` | `question.{evaluation_type, res_id, engine_area, pprInfo.engine_type}` |
| 各题独立提交 | 每个子题一条 log，**不需要选/填答案字段**；录音日志附加 `wav_url`/`spx_url`/`xml_url` |
| 端点 | `/m/audio/sync-v2`；分数展示走 iframe `set3q5aScore` |

### 4.7 与其它族的差异

| 对比 | 差异 |
|---|---|
| vs `collector.role` / `collector.dialogue` | **代码路径完全相同**（同一个 `case` 组、同一个 `getRecordAudios` 分支、同样的 `info.question[].xh`）；差别只在题目来源/地区 |
| vs `collector.word` | `word` 的 order 是 `info.xh`（**单值**，非数组），故不同分支 |
| vs `collector.repeat` | `repeat` 用 `info.fj[]` 的子题数组，而且在 `index-5543d811.js` 里额外读 `info_extra.json` 扩题、重算 `question_score`——**唯一做族级资源重算的族** |
| vs `collector.picture` / `read` | 单 order（`"1"`）而非子题数组；且 `picture` 不进 switch |
| vs `collector.choose` / `fill` | 选择题/填空题是**本地判分**；`3q5a` 是**纯录音 + 服务端评测**，不需要 `score_detail.choose` / `choose_id` |

---

## 5. 选项型答案的取值形态总结（提交字段最关键）

| 概念 | 官方字段 | 取值形态 | 证据（前端代码） | 证据（官方样本） |
|---|---|---|---|---|
| **选择题正确答案** | `content.json → info.xtlist[].answer` | **字母**（`"A"`/`"B"`/`"C"`/`"D"`；多选可 `"AC"`） | `e.answer===s.xx_mc`（`useExamStepper` `G()`） | `"answer":"B"` / `"answer":"C"`（北京初中 3 个真样本） |
| **选择题选项字母** | `info.xtlist[].xxlist[].xx_mc` | **字母** | `e.answer===s.xx_mc`；`choose:s.xx_mc`；`setPCChoose2` 用 `choose_id` | `{"xx_xh":"1","xx_mc":"A","xx_nr":" This Friday."}` |
| **选择题选项下标** | `info.xtlist[].xxlist[].xx_xh` | **数字串，从 1 开始**（`"1"`/`"2"`/`"3"`） | `e.xxlist.find(e=>e.xx_xh==n)`；`choose_id:n` | `"xx_xh":"1"/"2"/"3"` |
| **选择题选项文本** | `info.xtlist[].xxlist[].xx_nr` | 纯文本（客户端**不**用于判分/提交） | `grep xx_nr` 在 PC 前端命中 0 | `"xx_nr":" Next week."` |
| **选择题学生作答（载荷）** | `score_detail.choose` | **同 `xx_mc`，即字母** | `choose:s.xx_mc` | 与 `answer` 同域，故为字母 |
| **选择题学生作答（下标）** | `score_detail.choose_id` | **`xx_xh`，数字串** | `choose_id:n`（`n` 来自 iframe 的第三段） | — |
| **选择题未作答** | `score_detail.choose = ""`，**无 `choose_id`** · iframe 约定第 3 段 = `"0"` | | `if("0"===n)return{...choose:"",...}` | — |
| **字母→下标 的转换 `b()`** | 仅用于**给 iframe 显示/回填**（`setPCChoose` / `setPCChoose2`） | `A→"1"`、`B→"2"`…`Z→"26"`，`AC→"13"` | `function b(e){...o+=(r-64).toString()...}` | ⚠️ **不是 HTTP 载荷** |
| **填空题正确答案** | `content.json → info.std[].value` | **纯文本**；`/` 分隔多解；比较前 `X()` 归一（全角→半角、`—`/`_`→`-`、小写、`trim`） | `X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)` | 本地无 fill 真样本（见 §6） |
| **填空题空号** | `info.std[].xth` | 数字串 | `r.find(e=>e.xth===a)` | — |
| **填空题学生作答** | `score_detail.choose`（**无 `choose_id`**） | **纯文本原文** | `choose:n`（`n` = iframe 的 `answer`） | — |
| **填空题回填形状** | — | `[{"id":"<entity_id>_<order>","answer":...,"status":0\|1}]` 的 JSON 串 | `c.setPcBlank(JSON.stringify(e))` | — |
| **picture / 3q5a** | 不适用 | 客户端不取答案、不提交答案文本 | switch/handler 中无读取点 | `info.std`/`info.question[].std[]` 存在但无前端读取点 |

**一句话**：
**选择题 → 提交字母（`xx_mc`），下标另存 `choose_id`（`xx_xh`）；填空题 → 提交纯文本，无 `choose_id`；`b()` 只是给 iframe 的下标编码，别写进载荷。**

---

## 6. 「未找到 / 不确定」清单（含已查关键词与文件）

### 6.1 未找到

| # | 内容 | 已查关键词 | 已查文件/命令 | 结论 |
|---|---|---|---|---|
| 1 | PC 端 iframe 页面（`getChoice`/`setPCChoose`/`getPcBlank`/`returnPcBlank`/`set3q5aScore` 的定义体） | `setPCChoose` `setPCChoose2` `setPcBlank` `set3q5aScore` `getChoice` `ReturnChoose` `returnPcBlank` `getPcBlank` `getChooseChange` | `index-5543d811.js`、`chunks/*.js`（82）、`httphelper.js` `etshelper.js` `clientDB.js` `appcontroller.js` `shellhelper.js` | **全部命中 0**。这些函数是 `IFRAME_LOCAL_PATH="./common"` 下的本地静态页面（`pc-listen.html` / `read-write.html` 等）里的，不在本次下载内 |
| 2 | 114 个缺失 chunk（含 `useReadingWritingStructConverter-0e20e02c.js`、`ReadingWritingDetails-1b8d0987.js`、`ReadingWritingResult-93fd0909.js`、`ReadingWritingList-35f22288.js`、`ReadingWritingStart-6a005565.js`、`SpecialReadingWritingStart-d2af4bf6.js`、`QuestionListItem-229d65bc.js`、`MockExamResult-789f2f57.js`、`readAloud-*.js`、`readSentence-*.js`） | 用 bundle 里 `./Xxx-hash.js` 引用集与 `chunks/` 目录求差 | 194 refs / 82 have / 114 missing | **缺失**。PC 端"读写专用页面"的渲染分支无法提取 |
| 3 | `3q5a` 命名明文解释 | `3q5a` `3Q5A` `三问` `五答` `问答` `threeQ` `fiveAnswer` `q5a` | PC bundle + 82 chunks + `jadx_*/sources` + APK 字节扫描 | **前端源码 0 命中**；APK 里 1 次命中经字节取证为二进制噪声。命名结论来自官方语料（`group_name:"角色扮演"` + `role` 3×a/5×b） |
| 4 | 真实 `collector.fill` 官方 content.json 样本 | `"structure_type": "collector.fill"`、`xtlist`、`info.std`、`std[].xth`、`"xth"` | `Fe-pro/ets读取/{北京初中,广东初中}resoure/**/content.json`（24 份）、`ets_resolver/tests/corpus/**` | **本地未找到**。`collector.fill` 的字段结论**完全来自前端代码** |
| 5 | `xx_mc` 的取值域在**官方** content.json 中是否只可能是字母 | `xx_mc` `xx_nr` `xx_xh` `answer` | 3 份北京初中 choose 真样本（`4cd4f71f…`/`a1a21110…`/`e0e718fa…`）+ `InfoObjBean.java` | 3/3 样本中 `xx_mc` 均为 `A`/`B`/`C`，`answer` 均为字母；`b()` 只对字母有意义。**高置信，但样本量小** |
| 6 | `collector.picture` / `collector.3q5a` 在结果页是否有其它族专属回填 | `set3q5aScore` `showScore` `setPCChoose` `setPcBlank` | `useExamResult` switch 全文 | `picture` **无 case**；`3q5a` 只有 `set3q5aScore`（纯分数，无答案） |
| 7 | `xx_wj2` / `xt_analy` / `st_sm` / `st_pic` 的语义 | 字段名原文 | 3 份北京 choose 样本 + PC 前端 | PC 前端**无读取点**（`grep` 命中 0），只有 Android bean 有 `xt_pic`/`xt_wj`；`xt_analy`/`xt_wj2` 连 bean 都没有 → **未找到** |

### 6.2 不确定

| # | 内容 | 不确定点 |
|---|---|---|
| 1 | `choose` 的多选/联答串顺序 | `G()` 只按逗号切分逐段判，前端不排序、不去重；最终顺序 = iframe `getChoice()` 给出的顺序 → **未找到** iframe 侧的排序保证 |
| 2 | `question_list[0]` 假设 | `G()` 与 `returnPcBlank()` 都用 `_e.exam.question_list[0]` 的 `question_score`/`question_count`/`evaluation_type` 作为**整个套题**的参数。若同一 `exam` 下各子题满分不同，前端会用第 0 题的参数算全部 → **这是官方实现的行为，不是我方推测**，但我方是否要复刻需自行决定 |
| 3 | `fill` 的 `order` 严格相等 | 代码是 `r.find(e=>e.xth===a)`（`===`）而 `choose` 是 `e.xx_xh==n`（`==`）。若服务端 `std[].xth` 是数字而 iframe `id` 给字符串，将匹配失败 → 前端不做数字兼容，**未找到**任何兜底 |
| 4 | `3q5a` 子题数是否恒为 8 | 真样本为 8（3+5）；前端代码只按 `info.question[]` 长度循环，**不校验 3/5** → 不同题型可能不是 8 |
| 5 | `choose_id` 是否服务端必需 | 前端一律带 `score_detail.choose_id` 与顶层 `choose_id`，但**看不到服务端校验** → 是否必需**未找到** |

### 6.3 检索命令留档（可复现）

```bash
cd /home/shuaiqiu/Projects/ets源码恢复/pc_client_web
grep -oE '.{200}exam_type_collect.{400}' *.js chunks/*.js
for f in *.js chunks/*.js; do echo "$f $(grep -o 'collector\.choose' $f|wc -l)"; done
grep -oh 'collector\.[A-Za-z0-9_]*' index-5543d811.js chunks/*.js | sort | uniq -c
grep -c 'setPCChoose\|setPcBlank\|set3q5aScore\|getChoice\|ReturnChoose\|returnPcBlank\|getChooseChange\|setPCChoose2' index-5543d811.js   # → 0
grep -l 'exam_type_collect' index-5543d811.js chunks/*.js
# 缺失 chunk 统计
python3 -c "import re,os,glob;d=open('index-5543d811.js',encoding='utf-8',errors='replace').read();refs=set(re.findall(r'\./([A-Za-z0-9_.\-]+\.js)',d));have=set(os.path.basename(p) for p in glob.glob('chunks/*.js'));print(len(refs),len(have),len([r for r in refs if r not in have]))"
# 旁证：真实官方资源样本
python3 - <<'EOF'
import json,glob,os
for f in sorted(glob.glob('/home/shuaiqiu/Projects/Fe-pro/ets读取/*/content.json')):
    d=json.load(open(f,encoding='utf-8')); print(os.path.dirname(f), d.get('structure_type'), sorted((d.get('info') or {}).keys()))
EOF
# 旁证：APK 字节取证（证明 3q5a 是噪声）
python3 -c "import re;d=open('/home/shuaiqiu/Projects/ets源码恢复/E听说中学_7.0.65.2439.apk','rb').read();print([m.start() for m in re.finditer(rb'3q5a',d)])"
```
