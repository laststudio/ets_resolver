# E听说 PC 端「听说类」四题型族官方解析语义规格

**范围**：`collector.read`（模仿朗读）、`collector.repeat`（跟读/重复）、`collector.role`（角色扮演/问题+回答）、`collector.dialogue`（对话）
**方法**：**纯静态源码阅读**。不运行客户端、不抓包、不联网、不 sudo、不修改任何已有文件。
**证据约定**：所有代码片段均从原文件**逐字复制**（minified 代码保持原样，未美化；仅长行按原样整段引用）。片段均已核对「唯一匹配」。
**基线文件**：`/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/`

---

## 0. 证据基线与检索记录（先讲清"没找到什么"）

### 0.1 参与结论的文件

| 文件 | 在本四族中的角色 |
|---|---|
| `index-5543d811.js` | `class ProcessPaper`：res.json/ctrl.json/content.json/info.json/info_extra.json/data.json 的读取与题目展开；`useRecorder`（原生录音+评测 SDK 桥）；讯飞 ISE XML 解析（`parseIseResult` / `parseXMLPaper` / `EvaluationTypes`） |
| `chunks/index-5543d811.js` | **与 `index-5543d811.js` 完全相同**（md5 均为 `28bdb58380a09ff79f501728c055c998`），重复文件，不是独立 chunk |
| `chunks/useExamResult-d9fb4671.js` | 结果页/算分：`switch(exam_type_collect)`、`getRecordAudios`、`getAudioIds`、本地重评显示 |
| `chunks/useExamStepper-8ad3fbd7.js` | 逐题流程：`action.record` → ISE 评测 → 上报；**仅** `collector.choose` / `collector.fill` 有本地答案校验 |
| `chunks/Examine-d229829d.js` | 模考/作业作答页：`answerHandler`/`chooseHandler` → 本地日志 → `postAudioSyncLogs` |
| `chunks/Comprehensive-1968af87.js` | 一键练/整卷页：`processPaperContentJson` + `useExamResult.htmlLoad` |
| `chunks/adjustWordColor-bccb26cb.js` | `collector.read` 专用：解析服务端评测 XML，按词着色 + `read_chapter/read_sentence/read_word/topic` 判定 |
| `chunks/useAudioSync-0603b875.js` | 成绩同步上报的**最终载荷字段表** |
| `chunks/useRecordResult-fbe9e75b.js` | 评测结果 → `score_detail` 组装 |
| `chunks/mockExam-f7dae7ee.js` | `getLocalOrRemoteFile(params,"xml"/"audio")` 本地缓存优先/远端兜底 |
| `chunks/html-8e3719a4.js` | iframe 页面名枚举（`PcListenAnswer="pc-listen-answer.html"` 等） |
| `chunks/DynamicIframe-bafff5bc.js` | iframe 应用的资源白名单（列出 `js/pc-listen*.js` 等） |

### 0.2 族常量机器枚举（全量，去重按文件）

```
$ grep -o 'collector\.[a-zA-Z_0-9]*' index-5543d811.js chunks/*.js | sort | uniq -c
      4 chunks/useExamResult-d9fb4671.js:collector.read
      3 index-5543d811.js:collector.repeat
      3 chunks/useExamResult-d9fb4671.js:collector.word
      3 chunks/index-5543d811.js:collector.repeat      # 重复文件
      2 chunks/useExamResult-d9fb4671.js:collector.role
      2 chunks/useExamResult-d9fb4671.js:collector.repeat
      2 chunks/useExamResult-d9fb4671.js:collector.picture
      2 chunks/useExamResult-d9fb4671.js:collector.dialogue
      2 chunks/useExamResult-d9fb4671.js:collector.3q5a
      1 chunks/useExamStepper-8ad3fbd7.js:collector.fill
      1 chunks/useExamStepper-8ad3fbd7.js:collector.choose
      1 chunks/useExamResult-d9fb4671.js:collector.fill
      1 chunks/useExamResult-d9fb4671.js:collector.choose
```
确认：**没有 `collector.repeat_essay` / `collector.repeat_dialogue`**（全量枚举中不存在）。

### 0.3 `exam_type_collect` 的全部出现点（全文检索）

| 文件 | 次数 | 出现形式 |
|---|---|---|
| `index-5543d811.js` | 3 | 均为 `"collector.repeat"===<examType>.exam_type_collect`（`processPaperResources` / `processPaperSteps` / `processPaperContent`） |
| `chunks/index-5543d811.js` | 3 | 同上（重复文件） |
| `chunks/useExamResult-d9fb4671.js` | 2 | ① 前置判断 `"collector.read"!==n.examType.exam_type_collect`；② `switch(n.examType.exam_type_collect)` |
| `chunks/useExamStepper-8ad3fbd7.js` | 2 | `"collector.choose"===w.value.exam_type_collect` 与 `"collector.fill"===w.value.exam_type_collect`（同一个 `practise` 处理器内） |

计数方式：`python3 -c "import re;print(len(re.findall('exam_type_collect', open(F,encoding='utf-8').read())))"`（逐文件执行；注意 `grep -oE '.{200}exam_type_collect.{200}'` 因文件是单行、窗口重叠会少报，不可用于计数）。

**关键结论**：`exam_type_collect` 只出现在这 4 个（实为 3 个不同）文件里；`ProcessPaper`（index）里**只有 repeat 一族有分支**，read/role/dialogue/word/3q5a/picture/choose/fill 在 index 中**一律走同一条通用管线**。

### 0.4 检索过但**未命中**的项（不要推测）

| 检索关键词 | 结果 |
|---|---|
| `content2` / `content2.json` | **未找到**。JS 中引用到的全部 `.json` 文件名只有：`ctrl.json`(8)、`ctrl_check.json`(8)、`info_extra.json`(6)、`content.json`(6)、`data.json`(4)、`res.json`(2)、`info.json`(2)、`update.json`(2)、`pc_xst_dict.json`(2)、`worddict_data.json`(1)。**没有 content2** |
| `standard_answer` | **0 命中**（index + 全部 82 个 chunk） |
| `info.answer` / `.info.answer` | **0 命中** |
| `askaudio` / `aswaudio` / `value_bz` / `value_pf` | **0 命中**（这 4 个字段在 PC 端 bundle 里从未被读取，属 iframe/内容侧） |
| `keywords` | index 命中 1 次，位于 WebRTC SDP 属性解析（`{name:"keywords",reg:/^keywds:(.+)$/...}`），**与题目 keywords 无关** |
| `.info.` 全量字段读取 | 仅：`video`(53)、`audio`(31)、`stid`(21)、`xh`(4)、`xtlist`(2)、`std`(1)、`value`(1)、`question`(1)、`fj`(1)。其中 `.info.video` 的 53 次全部是 WebRTC `stream.info.video`，**与 content.json 无关** |
| `answer` 的全部命中（index+chunks） | ① `useExamStepper` 的 `collector.choose`：`e.answer===s.xx_mc`（读 `content.json → info.xtlist[].answer`，选择题本地判分）；② `useExamStepper` 的 `collector.fill`：`e.answer`（**用户输入**）对比 `info.std[].value`；③ `Examine`/`MockExamDetails`：`t.answer=` UI 布尔"是否已作答"；④ `DynamicIframe`/`index` 的 `wordCourseSourceHelpers` 等：单词课/作文 PPR（`r?.answer?.pprFile`），与本四族无关 |
| `HomeworkList-f02d250d.js` | 不含 `question_list` / `exam_type_collect` / `listeningSpeaking` → **与本题无关**（已在 §0.1 表外） |
| `useWordCollect-3f40ea97.js` / `collect_Normal-1ab3690e.js` / `exam-6ff42190.js` / `ExamQuestionListItem-af3089e6.js` | 分别是生词本接口、两张 base64 图片、状态枚举(`Unanswered/Answered/Correct/Incorrect`)、题目列表 UI 组件（`name/score/totalScore` props）→ 均与本四族的**解析语义**无关 |
| `pc_xst_dict.json` | 是**系统单词词典**（`wordDict.updateList(l)` 写入 IndexedDB），不是答案表 |
| `useMistakeInspector` / `useRemark` | 只处理上传/重评与错题，不含四族的答案字段 |

---

## 1. 全部"按族分发/分叉"点（总览，共 5 处）

### 分发点 ①：结果页算分 —— `chunks/useExamResult-d9fb4671.js`

```javascript
switch("collector.read"!==n.examType.exam_type_collect&&(I()||c.showScore(_,d,0)),n.examType.exam_type_collect){case"collector.3q5a":case"collector.role":case"collector.dialogue":case"collector.word":case"collector.repeat":I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);break;case"collector.read":
```

- `3q5a / role / dialogue / word / repeat` 五族 → **同一支**：调 `set3q5aScore(...)`（把每题分推给 iframe）。
- `read` → **独立一支**（见 §2）。
- `choose / fill` → 各自独立支（把选择/填空结果推给 iframe）。
- 前置条件：`"collector.read"!==exam_type_collect && (I()||c.showScore(_,d,0))` → 非 read 族先直接显示总分；read 族的总分显示被推迟到分支内部。

### 分发点 ②：录音回放取值 —— `chunks/useExamResult-d9fb4671.js` `getRecordAudios`

```javascript
getRecordAudios:async function(e,t,o){const i={};try{for(const r of e){let e,s,a;switch(r.structure_type){case"collector.read":case"collector.picture":e=`${D}_${r.info.stid}_1`,a=t.find(e=>e.entity_id===r.info.stid&&"1"===String(e.order)),s=a&&await w({set_id:D,entity_id:r.info.stid,order:1,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;case"collector.word":e=`${D}_${r.info.stid}_${r.info.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===r.info.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:r.info.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;case"collector.3q5a":case"collector.role":case"collector.dialogue":for(const n of r.info.question)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:n.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;case"collector.repeat":for(const n of r.info.fj)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:n.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??""}}}catch(r){}return i},getAudioIds:function(e){
```

### 分发点 ③：原声/回放音频 id —— `chunks/useExamResult-d9fb4671.js` `getAudioIds`

```javascript
getAudioIds:function(e){let t={original:"",playback:""};if(e.length){const o=e[0];switch(t.original=`${D}_${o.info.stid}${o.info.audio}`,o.structure_type){case"collector.read":case"collector.picture":t.playback=`${D}_${o.info.stid}_1`;break;case"collector.word":t.playback=`${D}_${o.info.stid}_${o.info.xh}`}}return t}
```

### 分发点 ④：题目展开（**只有 repeat**）—— `index-5543d811.js`，三处

```javascript
matchRegex=/{{(.*?)}}/;function replaceRealId(e,t){return e.replace(matchRegex,t)}
```
```javascript
getTemplateFilePath(e){return replacePathSeparator(`${this.getPackageId()}\\template_${e??this.template.id}`)}getContentFilePath(e){return replacePathSeparator(`${this.getPackageId()}\\content_${e}`)}generateFilePath(e,t,n){const r="templet"===e?`template_${n??this.template.id}`:`content_${t}`;return replacePathSeparator(`${this.getPackageId()}\\${r}`)}getPackageId=()=>this.super_set_id?this.super_set_id:this.setId;
```

- `getContentFilePath(real_id)` → `<packageId>\content_<real_id>`；`getTemplateFilePath` → `<packageId>\template_<templateId>`；
- `packageId = super_set_id || set_id`；
- `matchRegex = /{{(.*?)}}/`：ctrl.json 的 `subid` 形如 `{{code_id}}_N`，用 `replaceRealId` 把 `{{code_id}}` 换成真实 `real_id`。

### 分发点 ⑤：本地答案校验（**只有 choose / fill**）—— `chunks/useExamStepper-8ad3fbd7.js`

```javascript
async function G(e,t,a,o){const{getFileJson:n}=f(),{UNZIP_LOCAL_PATH:l}=d();let s=[],r=(await n(p(`${l}\\${e}\\content_${t}\\content.json`))).info.xtlist;try{s=o.split(",")}catch(i){}return s.map(e=>{const[t,o,n]=e.split("_");let l="0";if("0"===n)return{entity_id:t,order:o,score_detail:{total_score:l,choose:"",category:a.evaluation_type}};{const e=r.find(e=>e.xt_xh===o);if(!e)return null;const s=e.xxlist.find(e=>e.xx_xh==n);return s?(e.answer===s.xx_mc&&(l=m(a.question_score/a.question_count)),{entity_id:t,order:o,score_detail:{total_score:l,choose:s.xx_mc,choose_id:n,category:a.evaluation_type}}):null}}).filter(Boolean)}
```
```javascript
"collector.fill"===w.value.exam_type_collect&&(y.returnPcBlank=async t=>{const a=await async function(t,a,o,n){const{getFileJson:l}=f(),{UNZIP_LOCAL_PATH:s}=d(),r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std,i=JSON.parse(n);try{return i.map(e=>{const[t,a]=e.id.split("_"),n=e.answer;let l="0";if(""===n)return{entity_id:t,order:a,score_detail:{total_score:l,choose:"",category:o.evaluation_type}};{const e=r.find(e=>e.xth===a);return s=X(n),X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),{entity_id:t,order:a,score_detail:{total_score:l,choose:n,category:o.evaluation_type}}}var s})}catch(e){return[]}}(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_fill"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:U.getInstance().getIsHomework()?1:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}},y.getPcBlank())}
```

> 这两处是本四族**唯一**的"客户端读答案"行为，但它们属于 `collector.choose` / `collector.fill`，**不属于 read/repeat/role/dialogue**。

---

## 2. `collector.read`（模仿朗读）

### 2.1 族判定与分发点

`exam_type_collect` 的值来自 **`template_<id>/res.json` 的 `exam_type_list[].exam_type_collect`**（index 中从不写死，只读；见 §1 分发点④及 §2.2）。read 有**唯一专属分支**：

```javascript
case"collector.read":n.examType.exam_list.length>1?I()||setTimeout(()=>{c.showScore(_,d,0);const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0):function(e,t,o,i,r){setTimeout(async()=>{const{htmlData:s,hasLogScore:n,set_module_id:c}=t,l=t.exam_type_score??5,d=s[0];if(d){const s=o.find(e=>e.entity_id===d.info.stid)||{},n=await x({set_id:D,entity_id:s.entity_id,order:s.order,mistake_id:t.mistake_id,homework_id:t.homework_id,remote_url:s.detail_file});let c="";c=n&&!n.includes("http")?await C(n):await p(s.detail_file);const{real_score:_,detail:u,standard_score:m,manual_modified:f}=s,h=y(u),g=t.examType.exam_list.find(e=>e.exam_id===d.info.stid);let w=d.info.value;-1===w.indexOf("<\br>")&&(w="</br>"+w.replace(/<\/p>/g,"</p></br>"));const{scoreDetail:k,essayData:S}=await $(w,c,{first_column_id:i,stu_total:m,stu_real_score:m,multidimensional_results:h,exam_type_score:l,manual_modified:!0===f}),b="number"==typeof _?_:k.stu_real_score,{scoreFilter:q}=a(),[T,v]=q(l,b,r);if(I()||e.showScore(v,T,0),g&&g.question_list){const t=g.question_list[0];if(!I()&&("read_chapter"===t.evaluation_type?e.showEssay_2.apply(null,S):"read_sentence"===t.evaluation_type&&e.showPhrase_2.apply(null,S),"read_word"!==t.evaluation_type)){const t=k?.dimension_result||[];e.showScoreDetail.apply(null,t.slice(0,3))}}}},0)}(c,n,e,o,i);break;
```
（`chunks/useExamResult-d9fb4671.js`）

以及两处结构分发：

```javascript
// chunks/useExamResult-d9fb4671.js  getRecordAudios 内
case"collector.read":case"collector.picture":e=`${D}_${r.info.stid}_1`,a=t.find(e=>e.entity_id===r.info.stid&&"1"===String(e.order)),s=a&&await w({set_id:D,entity_id:r.info.stid,order:1,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";
```
```javascript
// chunks/useExamResult-d9fb4671.js  getAudioIds 内
case"collector.read":case"collector.picture":t.playback=`${D}_${o.info.stid}_1`;break;
```
→ **read 的 order 恒为 `1`**（硬编码），与 `collector.picture` 完全同构。

read 走 `pc-listen-answer.html`（结果/答案页），而逐题练习流走 `pc-listen.html`：

```javascript
PcListenAnswer="pc-listen-answer.html",e.PcListenWarnResult="pc-listen-warn-result.html"
```
```javascript
P=`${L}/${k.PcListenAnswer}`,A=c({}),I=()=>l().getFoshanNonVipActive&&d.getInstance().getIsHomework()&&1!==_().currentExamItem?.free;
```
（`chunks/html-8e3719a4.js`；`chunks/useExamResult-d9fb4671.js`）

### 2.2 数据来源（每层字段与嵌套）

**(a) `template_<templateId>/ctrl.json`（若存在 `ctrl_check.json` 则优先用它）** —— 只取 steps 骨架：

```javascript
if("collector.repeat"===r.exam_type_collect){const n=await this.getJson(replacePathSeparator(`${this.getContentFilePath(s.real_id)}\\info_extra.json`)),o=e.filter(e=>e.subid.includes("_0")),i=e.filter(e=>!e.subid.includes("_0"));let a=0;for(const e of n){let t=JSON.stringify(i);for(const o of e)t=t.replace(new RegExp(o.id,"g"),o.value);let n=JSON.parse(t);const r=o[o.length-1];let s=Number(r.order)||Number(r.order_num);const l=Number(s)+1,c=(e,t)=>e.replace(/_(\d+)$/,(e,n)=>`_${Number(n)+t}`);n=n.map(e=>{let t=e.subid;return s++,{...e,subid:c(t,a),order:`${l}_custom_order_${e.subid}_${s}`,order_num:String(s)}}),a++,o.push(...n)}t[r.set_id].steps=t[r.set_id].steps.concat(o)}else t[r.set_id].steps=t[r.set_id].steps.concat(e)}
```
其中非 repeat 分支是 `else t[r.set_id].steps=t[r.set_id].steps.concat(e)`。`e` 由：

```javascript
const o=t.contents.find(e=>`{{${e.code_id}}}`===r.exam_id);if(o&&(r={...r,exam_id:replaceRealId(r.exam_id,o.real_id),...o})
```
→ 把 `struct.contents[]` 里的 `{code_id, real_id, url, size}` 合并进 exam。

**(b) `template_<id>/res.json`** —— 题型/题量/分值/题目清单：

```javascript
res.json`));this.totalScore+=Number(r.set_score);const o=lodashExports.cloneDeep(r),i=await Promise.all(o.exam_type_list.map(async e=>{let n=0;return e.exam_list=await Promise.all(e.exam_list.map(async r=>{const o=t.contents.find(e=>`{{${e.code_id}}}`===r.exam_id);if(o&&(r={...r,exam_id:replaceRealId(r.exam_id,o.real_id),...o}),"collector.repeat"===e.exam_type_collect){let e=function(e,t){const n=(e/t).toFixed(2),r=(e-(t-1)*n).toFixed(2);let o=[];for(let i=0;i<t-1;i++)o.push(n);return o.push(r),o};const t=await this.getJson(replacePathSeparator(`${this.getContentFilePath(r.real_id)}\\info_extra.json`));let o=lodashExports.cloneDeep(r.question_list),i=Number(t.length)-o.length;for(;i>0;){const e=o[o.length-1];o.push({...e,question_order:String(Number(e.question_order)+1)}),i--}const a=e(r.exam_score,o.length);let s=0;o.forEach((e,t)=>{e.question_count="1",e.question_score=a[t],s+=parseFloat(a[t])});const l=o.length;r.exam_count=String(l),r.exam_score=String(s),r.question_list=o,n+=l}return r})),e.exam_type_count=String(n),e.group_name=e.exam_type_name,{...t,...e}}));e.push(...i)}const t={exam_type_list:e};this.examTypeList=e;return mergePPRes(t,await this.initPPRRes(t))}
```

**(c) `content_<real_id>/content.json`** —— 题目实体（read 的题面就在这里）：

```javascript
async processPaperContentJson(e,t){try{const n=e.exam_list.filter(e=>t?e.exam_id===t:e).map(async e=>{const t=await this.getJson(replacePathSeparator(`${this.getContentFilePath(e.real_id)}\\content.json`)),n=lodashExports.cloneDeep(t),r=JSON.parse((o=JSON.stringify(n),e.exam_info.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),o)));var o;const i=this.extractMp3Content(r.info,[]).reduce((t,n)=>(t[`${this.getPackageId()}_${r.info.stid}${n}`]=replacePathSeparator(`${this.UNZIP_LOCAL_PATH}\\${this.getPackageId()}\\content_${e.real_id}\\material\\${n}`),t),{}),a=replacePathSeparator(`${this.IMAGE_LOCAL_PATH}\\${this.getPackageId()}\\content_${e.real_id}\\material\\`);return{contents:r,paths:a,audios:i}}),r=await Promise.all(n),o=[],i={},a=[];return r.forEach(e=>{o.push(e.contents),a.push(e.paths),Object.assign(i,e.audios)}),{contents:o,paths:a,audios:i}}catch(n){throw n}}
```
→ 返回 `{contents:[content.json...], paths, audios}`；`audios` 的 key 是 `` `${packageId}_${content.info.stid}${mp3文件名}` ``（由 `extractMp3Content(r.info,[])` 递归提取 `info` 下所有含 `.mp3` 的字符串）。

**(d) `content_<real_id>/info.json`** —— `view.html` 的 jsparam 字段（read 的展示文本走这里）：

```javascript
async processJsParam(e,t,n){try{const r=n?.template?.id,o={};for(const n of e){const e=replacePathSeparator(`${this.generateFilePath(n.source,t.real_id,r)}\\info.json`),i=await this.getJson(e),a=lodashExports.cloneDeep(i).find(e=>e.code_id===n.code_id),s=t.exam_info,l=e=>s.reduce((e,t)=>e.replace(new RegExp(t.id,"g"),t.value),e);if(a){let e;e=["txt","txxt"].includes(a.code_type)?l(a.code_value):"json_array"===a.code_type?l(a.code_json_array):"json_obj"===a.code_type?l(a.code_json_obj):"image"===a.code_type?[{url:replacePathSeparator(`${this.IMAGE_LOCAL_PATH}\\${this.generateFilePath(n.source,t.real_id,r)}\\material\\${a.code_value}`)}]:"",o[n.classname]=e}}return o.content_highlight&&(o.content=o.content.replace(o.content_highlight,`<span class="color_repeat">${o.content_highlight}</span>`)),o}catch(r){throw r}}
```
注意末句：`o.content_highlight && (o.content = o.content.replace(o.content_highlight, '<span class="color_repeat">...</span>'))` —— **`content_highlight` 是 repeat 族的"高亮跟读片段"机制**。

**(e) `content_<real_id>/data.json`** —— `processPaperDataJson` / `processPaperExamDataJson` 读 `model_pc.jsparam`（同上 `processJsParam`）。

**(f) `info_extra.json` 只在 repeat 读**，read 全流程无引用（见 §0.3）。
**(g) `content2.json` 不存在**（见 §0.4）。

### 2.3 逐题抽出规则

- **一题 = 一个 `content_<real_id>/content.json` = `exam_list[]` 的一项**。题干实体 id = `info.stid`。
- **题号（order）恒为 `1`**：`order:1` / `_1` 硬编码（见 §2.1）。`info.xh` 对 read 也存在，但**在 PC bundle 中 read 分支不读 xh** —— `info.xh` 的全部 4 次读取都在 `collector.word` 分支（`getRecordAudios` 中 3 次 + `getAudioIds` 中 1 次），repeat/role/dialogue 用的是子元素的 `xh`（`info.fj[].xh` / `info.question[].xh`，写法是 `n.xh` 而非 `info.xh`）。
- **子题型（朗读形态）由 `exam.question_list[0].evaluation_type` 决定**，PC 端枚举白名单：

```javascript
showDynamicOriginalButton:e=>{if(!e)return!1;const{getCurrentHomework:t}=_();if(u.logCustom(`showDynamicOriginalButton: show_answer_time:${t?.show_answer_time}, exam_list_len: ${e?.exam_list?.length}, questionList: ${JSON.stringify(e?.exam_list?.[0]?.question_list??[])}`),1==e.exam_list.length){const t=e.exam_list[0].question_list;if(1==t.length){const e=t[0];return["read_chapter","topic","read_word","read_sentence"].includes(e.evaluation_type)&&Object.values(A.value).length}}return!1},
```
```javascript
showDynamicPlaybackButton:e=>{if(!e)return!1;const{getCurrentHomework:t}=_();if(u.logCustom(`showDynamicPlaybackButton: show_answer_time:${t?.show_answer_time}, exam_list_len: ${e?.exam_list?.length}, questionList: ${JSON.stringify(e?.exam_list?.[0]?.question_list??[])}`),1==e.exam_list.length){const t=e.exam_list[0].question_list;if(1==t.length){const e=t[0];return["read_chapter","topic","read_word","read_sentence"].includes(e.evaluation_type)}}return!1},
```
→ read 的 4 个取值：`read_chapter` / `read_sentence` / `read_word` / `topic`（`topic` = 话题/自由表达型朗读）。展示形态分派：
```javascript
if(!I()&&("read_chapter"===t.evaluation_type?e.showEssay_2.apply(null,S):"read_sentence"===t.evaluation_type&&e.showPhrase_2.apply(null,S),"read_word"!==t.evaluation_type)){const t=k?.dimension_result||[];e.showScoreDetail.apply(null,t.slice(0,3))}
```
（`showEssay_2`/`showPhrase_2`/`showScoreDetail` 是 iframe（`win`）注入的方法，**不在本 bundle 内**，未找到其实现。）

- 一题的字段（PC 端 JS 实际读取的）：`info.stid`（实体 id）、`info.value`（朗读原文）、`info.audio`（原声文件名，仅用于拼播放 key）、`exam.question_list[0].evaluation_type`、`question_score`、`real_id`。

### 2.4 答案在哪里（核心）

> **结论：客户端完全不持有、也不读取 read 族的"正确答案"。read 是"送音频给服务端/原生引擎打分"型。**

证据链：

**(1) `content.json → info` 被读取的字段全集里没有 `answer`。**
全库 `.info.xxx` 读取枚举（见 §0.4）只有 `stid / xh / xtlist / std / value / question / fj / audio`；`info.answer` 命中 0 次。

**(2) read 分支只做"取评测 XML + 正则文本着色"，不比对任何答案：**

```javascript
:function(e,t,o,i,r){setTimeout(async()=>{const{htmlData:s,hasLogScore:n,set_module_id:c}=t,l=t.exam_type_score??5,d=s[0];if(d){const s=o.find(e=>e.entity_id===d.info.stid)||{},n=await x({set_id:D,entity_id:s.entity_id,order:s.order,mistake_id:t.mistake_id,homework_id:t.homework_id,remote_url:s.detail_file});let c="";c=n&&!n.includes("http")?await C(n):await p(s.detail_file);const{real_score:_,detail:u,standard_score:m,manual_modified:f}=s,h=y(u),g=t.examType.exam_list.find(e=>e.exam_id===d.info.stid);let w=d.info.value;-1===w.indexOf("<\br>")&&(w="</br>"+w.replace(/<\/p>/g,"</p></br>"));const{scoreDetail:k,essayData:S}=await $(w,c,{first_column_id:i,stu_total:m,stu_real_score:m,multidimensional_results:h,exam_type_score:l,manual_modified:!0===f}),b="number"==typeof _?_:k.stu_real_score,{scoreFilter:q}=a(),[T,v]=q(l,b,r);if(I()||e.showScore(v,T,0),g&&g.question_list){const t=g.question_list[0];if(!I()&&("read_chapter"===t.evaluation_type?e.showEssay_2.apply(null,S):"read_sentence"===t.evaluation_type&&e.showPhrase_2.apply(null,S),"read_word"!==t.evaluation_type)){const t=k?.dimension_result||[];e.showScoreDetail.apply(null,t.slice(0,3))}}}},0)}(c,n,e,o,i);break;
```
逐步解读（全部逐字来自上段）：
- `d = s[0]`：只取 `htmlData`（即 `contents`，content.json 数组）第 0 个 —— read 单篇；
- `x({set_id, entity_id, order, mistake_id, homework_id, remote_url: s.detail_file})`：优先取**本地缓存的评测 XML**（`mockExam-f7dae7ee.js` 的 `getLocalOrRemoteFile(params,"xml")`），否则用日志里的 `detail_file`（远端 XML URL）；
- `c = n && !n.includes("http") ? await C(n) : await p(s.detail_file)`：`C`=`useFileManager().getFileData`（读本地文件文本），`p`=`requestXml`（拉远端 XML）；
- `w = d.info.value`（**朗读原文**，仅作着色底本）；
- `await $(w, c, {...})`：`$` = `chunks/adjustWordColor-bccb26cb.js` 的默认逻辑 —— **用服务端评测 XML 给原文逐词着色**；
- 得分 `b = "number"==typeof _ ? _ : k.stu_real_score`，其中 `_ = s.real_score`（日志里的真实分）。

**(3) `adjustWordColor` 解析的是服务端评测 XML，不是答案表：**

```javascript
let A="read_chapter";h=t.unescape(h);const C=e(),S=C.getSystemConfig||{},L=Number(S.mutil_dimension_adjust),F=P(S.res_color_adjust)||[],E=function(e){e=e||"";const r=(new DOMParser).parseFromString(e,"text/xml").getElementsByTagName("xml_result");let t="";const o=F?.[0]?.rule;if(r&&r[0]){for(let o of r[0].children){const e=o.attributes;if(t=e&&e.version&&e.version.value?e.version.value:"",t){A=o.tagName;break}}const e=function(e){const r=F;let t={bad:0,middle:60,good:80};for(var o in r){for(var s in r[o].engine_version)if(r[o].engine_version[s]==e){t=r[o].rule;break}for(var s in r[o].engine_version_keyword)if(0==e.indexOf(r[o].engine_version_keyword[s])){t=r[o].rule;break}}return t}(t);return Object.keys(e).length?e:o}return o}(w)||{}
```
```javascript
const D=function(e,t){e=e||"";var o=(new DOMParser).parseFromString(e,"text/xml"),s={};if(o){var n=null,c=[];o.hasChildNodes("read_chapter")&&o.getElementsByTagName("read_chapter")[0]?c=o.getElementsByTagName("read_chapter"):o.hasChildNodes("read_sentence")&&o.getElementsByTagName("read_sentence")[0]?c=o.getElementsByTagName("read_sentence"):o.hasChildNodes("read_word")&&o.getElementsByTagName("read_word")[0]&&(c=o.getElementsByTagName("read_word"));let e=[],r=[],t=[];if(n=c.length?c[c.length-1]:null){var a=n.getElementsByTagName("sentence"),l=n.getElementsByTagName("word");if(a&&a.length>0)for(var i=0;i<a.length;i++){var _=a[i],d=_.getElementsByTagName("word");const o=_.getAttribute("word_count")||d?.length;var[u,g]=G(d,o);t=[...t,...g],r.push(u),e=e.concat(u)}else{const[r,o]=G(l);e=r,t=[...t,...o]}s.is_rejected=n.getAttribute("is_rejected"),s.accuracy_score=n.getAttribute("accuracy_score")||0,s.standard_score=n.getAttribute("standard_score")||0,s.fluency_score=n.getAttribute("fluency_score")||0,s.integrity_score=n.getAttribute("integrity_score")||0,s.stress_score=n.getAttribute("stress_score")||0,s.total_score=n.getAttribute("total_score")||0,s.words_per_min=n.getAttribute("words_per_min")||0,s.effective_words=e,s.effective_sentence=r,s.read_more_words=t}s.effective_words&&s.effective_words.forEach((e,r)=>{e.index=r})}
```
```javascript
return{scoreDetail:I,essayData:J(B),wordsList:B,html:M,category:A,multiDimensionalAnalysis:W({scoreDetail:I,xml_res:D})}
```
→ 从 XML 里取 `read_chapter` / `read_sentence` / `read_word` 节点的属性：`total_score, accuracy_score, fluency_score, standard_score, integrity_score, stress_score, words_per_min, is_rejected`，并按 `sentence/word` 子节点做逐词评分。**全程没有答案键比对。**

**(4) 引擎与 XML 的来源是讯飞 ISE（原生插件），字段名固定：**

```javascript
const EvaluationTypes=["read_word","read_chapter","read_sentence","read_choice","simple_expression_ufi","simple_expression_ufk","simple_expression_ufj","topic","simple_expression","free_expression"];
```
```javascript
function parseXMLPaper(e,t){switch(e){case"read_word":const e=t?.xml_result?.read_word?.rec_paper?.read_word;return e;case"read_chapter":const n=t?.xml_result?.read_chapter?.rec_paper?.read_chapter;return n;case"read_sentence":const r=t?.xml_result?.read_sentence?.rec_paper?.read_chapter;return r;case"read_choice":const o=t?.xml_result?.read_choice?.rec_paper?.free_choice;return o;case"simple_expression_ufi":const i=t?.xml_result?.simple_expression_ufi?.rec_paper?.rec_paper;return i;case"simple_expression_ufk":const a=t?.xml_result?.simple_expression_ufk?.rec_paper?.rec_paper;return a;case"simple_expression_ufj":const s=t?.xml_result?.simple_expression_ufj?.rec_paper?.rec_paper;return s;case"topic":const l=t?.xml_result?.topic?.rec_paper?.rec_paper;return l;case"simple_expression":const c=t?.xml_result?.simple_expression?.rec_paper?.rec_paper;return c;case"free_expression":const u=t?.xml_result?.free_expression?.rec_paper?.rec_paper;return u;default:return{}}}
```
```javascript
function parseIseXML(e,t){return parseXMLPaper(e,new fxp.XMLParser({attributeNamePrefix:"",ignoreAttributes:!1,parseTagValue:!1,parseAttributeValue:!1}).parse(t))}function parseIseResult(e,t){const n=parseIseXML(e,t)??{};return[{name:"准确度",attr:"accuracy_score"},{name:"流畅度",attr:"fluency_score"},{name:"完整度",attr:"integrity_score"},{name:"标准度",attr:"standard_score"},{name:"总分",attr:"total_score"},{name:"总分(原始分)",attr:"xml_score"},{name:"except_info",attr:"except_info"},{name:"is_rejected",attr:"is_rejected"}].reduce((e,t)=>{const r=n[t.attr]??"0";return e[t.attr]=r,"xml_score"==t.attr&&(e[t.attr]=n.total_score),e},{})}function parseEtsOpenIseResult(e){
```
```javascript
function scoreFormater(e,t=2){let n=Number(e)*Math.pow(10,t);return Math.round(Number(n))/Math.pow(10,t)}function scoreFormatter_125(e,t=2){return scoreFormater(.125*Math.ceil(Number(e)/.125))}function ConvertToPercentScore(e,t,n=0){return e&&t?scoreFormater(Number(e)/Number(t)*100,n):0}function calculateRealScore(e,t){return scoreFormater(Number(e)/5*Number(t))}function calculateRealScoreWithCategory(e,t,n){return EvaluationTypes.includes(n)?calculateRealScore(e,t):e}
```
- `EvaluationTypes` 里 `read_word/read_chapter/read_sentence` 与 `topic/simple_expression*/free_expression` 并列 → 说明 read 与"开放性表达"同样走 ISE 引擎评分；
- 录音+评测调用在 index 的 `useRecorder.record`：
```javascript
const t={category:U&&g?.category||e.category,res_id:e.res_id,region:F,ent:I,mark_local_file:L,isEtsOpen:U,set_id:e.set_id,first_column_id:e.first_column_id,aioeFlag:e.aioeFlag};LoggerUtils.logCustom(`engine_type: ${e.engine_type}, ent: ${I}, ppr开放题: ${U}, 录音参数recordParam: ${JSON.stringify({...t,mark_local_file:!!L})}`,{tag:"ENG_SCORE"}),d={...e,...t,params:t,sdk:E.value,name:R},S.onmessage(n=>{LoggerUtils.logCustom(`recorder iseManager.onmessage成功: filePath:${R}, sdk:${E.value}`,{tag:"ENG_SCORE"});const r={name:R,sdk:E.value,params:{...e,...t},...n,sid:n.sid||n.traceId};"stop"!==c[D].status?c[D].successResult=r:c[D].promiseControl?.resolve(r)}),S.onerror(e=>{const n={...e,name:R,sdk:E.value,params:t,errorResult:e?.engine_code??e?.code??"network_error"},r=[ENGINE_CODE.NoData,ENGINE_CODE.Silent,ENGINE_CODE.LowVolume].some(t=>t==e?.engine_code)||isEmptyAudioByEngineError(e);if(LoggerUtils.logCustom(`recorder iseManager.onerror失败, sdk:${E.value}, error: ${JSON.stringify(e)}, status:${c[D].status}, result: ${JSON.stringify(n)}`,{tag:"ENG_SCORE",type:2}),r)return LoggerUtils.logCustom(`recorder评测无声，无数据，出0分, sdk:${E.value}, result: ${JSON.stringify(e)}`,{tag:"ENG_SCORE",type:2}),"stop"===c[D].status?void c[D].promiseControl?.resolve(n):void(c[D].successResult=n);"stop"!==c[D].status?c[D].errorResult=n:c[D].promiseControl?.reject(n)}),S.start(t)}
```
（`category = evaluation_type`，`res_id`，`set_id`，`first_column_id`，`ent`，`region`，`isEtsOpen`，`mark_local_file` → 送原生 `sndInst.record` / ISE）
- `calculateRealScore(e,t)=scoreFormater(Number(e)/5*Number(t))` → **引擎分是 5 分制**，`真实分 = 引擎总分 / 5 × question_score`。
- 引擎分数修补（read 专有）：

```javascript
function adjustStudyEngineXml(e,t,n){const r=new fxp.XMLParser({ignoreAttributes:!1,attributeNamePrefix:"",parseTagValue:!1,parseAttributeValue:!1}),o=new fxp.XMLBuilder({ignoreAttributes:!1,attributeNamePrefix:"",suppressBooleanAttributes:!1}),i=r.parse(t),a=r.parse(n),s=parseXMLPaper(e,i),l=parseXMLPaper(e,a);LoggerUtils.logCustom(`原始模考引擎分数：${JSON.stringify({total_score:s.total_score,accuracy_score:s.accuracy_score,fluency_score:s.fluency_score,standard_score:s.standard_score,integrity_score:s.integrity_score})}`,{tag:"ENG_SCORE"}),LoggerUtils.logCustom(`原始学习引擎分数：${JSON.stringify({total_score:l.total_score,accuracy_score:l.accuracy_score,fluency_score:l.fluency_score,standard_score:l.standard_score,integrity_score:l.integrity_score})}`,{tag:"ENG_SCORE"});const c=parseFloat(s.total_score),u=parseFloat(l.total_score),d=parseFloat(l.integrity_score);if("read_chapter"===e&&d<4){const e=d<.25?d:d+1;s.total_score=Math.min(e,c).toFixed(6)}const p=["accuracy_score","fluency_score","standard_score"];if(d>=parseFloat(s.total_score)&&u>0){const e=parseFloat(s.total_score)/u;p.forEach(t=>{const n=parseFloat(l[t])||0;l[t]=(n*e).toFixed(6)})}return l.total_score=s.total_score,
```
→ `if("read_chapter"===e && integrity_score<4)` 时用学习引擎完整度纠正模考引擎总分；再把双引擎的准确性/流畅性/标准度按比例对齐。

**(5) role/repeat/read 均在 `useExamStepper` 里走同一条 `action.record` 通道：**

```javascript
"action.record":async t=>{ue.value=!0;let a=Number(t.playtime??0);"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&(a=1e4,h?.pause());const o=B.cloneDeep(t);if(o&&o.question){const{pprInfo:t}=o.question||{},{engine_type:n}=t||{},l={record_time:a,category:o.question.evaluation_type,res_id:o.question.res_id,region:o.question?.engine_area,recordStep:o,set_id:e,engine_type:n,pprInfo:t,first_column_id:g};if(ne.value)return;ne.value=!0;const s=Z.record(l,{onReady:()=>{oe.value=!0,le.value=!1,ne.value=!1,"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&h?.play()},onReadyLoading:()=>{le.value=!0}}),r=()=>{const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};return new Promise((t,a)=>{s.then(async a=>{L.trigger(async()=>{const t=await re(l.category,a,g,o.question.question_score,e=>e&&se.addTask(e));return x.trigger({...t,...e}),t}),t(a)}).catch(t=>{a({...t,logParams:e,recordStep:o})})})};F.trigger(r)}
```
→ `category: o.question.evaluation_type`、`res_id: o.question.res_id`、`engine_type: pprInfo.engine_type`（`pprInfo` 来自 PPR 资源请求，见 §6）；返回 `score_detail` 后只组装标识，不比对答案。

### 2.5 分值

| 分值来源 | 说明 |
|---|---|
| `res.json → set_score` | 整套总分（`this.totalScore += Number(r.set_score)`） |
| `res.json → exam_type_list[].exam_type_score` | 该题型满分；useExamResult 里 `l = t.exam_type_score ?? 5`（**兜底 5**） |
| `res.json → exam_list[].exam_score` | 该套题满分（`action.record` 里 `question_type_score` 直接用 `exam.question_score`；`Comprehensive` 用 `We.value.exam_type_score`） |
| `res.json → exam_list[].question_list[].question_score` | 单题满分 |
| `res.json → exam_list[].question_list[].question_count` | 小问数（choose/fill 本地判分用它：`m(a.question_score/a.question_count)`） |
| `scoreFilter(满分, 实得, isNotSync)` | 返回 `[显示分, 显示满分]`：`[n?scoreFormater(t):ConvertToPercentScore(t,e,0), n?e:100]` |
| 逐题推送 | 当 `exam_list.length>1` 时走 `j()`（见 §3.5），单篇时走上面的 read 专属分支 |

read 单篇分支的总分显示：
```javascript
const{scoreFilter:q}=a(),[T,v]=q(l,b,r);if(I()||e.showScore(v,T,0),...
```
（`l = exam_type_score ?? 5`，`b = real_score ?? scoreDetail.stu_real_score`）

### 2.6 提交时每题的标识字段

**结果同步的最终载荷（唯一"字段表"级证据）** —— `chunks/useAudioSync-0603b875.js`：

```javascript
const i={resource_id:"",real_score:0,score:0,set_id:"",entity_id:"",order:"",serial_id:"",mistake_id:0,upload_file_id:"",upload_file_url:"",spx_file_url:"",mark_trace_id:"",detail_file_id:"",detail_file_url:"",score_detail:""}
```

**逐题组装（模考/作业作答页）** —— `chunks/Examine-d229829d.js` `Qa()`：

```javascript
async function Qa(e){let a=K.getInstance().handleRequestParams({resource_id:Oe,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:je,entity_id:e.entity_id,order:e.order,upload_file_id:e.upload_file_id??"",upload_file_url:e.upload_file_url??"",spx_file_url:e.spx_file_url??"",mark_trace_id:e.mark_trace_id??"",detail_file_id:e.detail_file_id??"",detail_file_url:e.detail_file_url??"",score_detail:e.score_detail,question_total_point:R.total_point});if(ka({detail_file:e.xml_local_path??"",audio:e.audio_local_path??"",detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,standard_score:"",choose_id:a.score_detail.choose_id,score_detail:a.score_detail}),e.noPost)return;const{postAudioSyncLogs:l}=xe();V.logCustom(`Examine.vue获取上报选择题数:${JSON.stringify(a)}`,{tag:"FLOW_PRAC"});return await l(a)}
```

**本地日志累加（一键练页）** —— `chunks/Comprehensive-1968af87.js`：

```javascript
a={detail:e.score_detail,entity_id:e.entity_id,order:Number(e.order),standard_score:e.score_detail.total_score,real_score:e.score_detail.real_score??e.score_detail.total_score}
```

**录音评测结果里的标识回溯** —— `chunks/useExamStepper-8ad3fbd7.js`：

```javascript
const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};
```

**PPR 资源请求的标识（与提交保持同一套口径）** —— `index-5543d811.js`：

```javascript
function processExamPPRParams(e){const t=[];return e.exam_type_list.forEach(e=>{e.exam_list.forEach(e=>{e.question_list.forEach(n=>{t.push({entity_id:e.real_id,order:n.question_order})})})}),t}
```
```javascript
function mergePPRes(e,t){return{...e,exam_type_list:e.exam_type_list.map(({exam_list:e,...n})=>({...n,exam_list:e.map(({question_list:e,...n})=>({...n,question_list:e.map(e=>{const{question_id:r,question_order:o}=e,i=t.find(e=>e.entity_id===n.exam_id&&r===e.question_id&&o==e.order)||{};return{...e,pprInfo:i,res_id:i.res_id??"",mark_type:i.mark_type??""}})}))}))}}
```
→ `mergePPRes` 用 `entity_id===exam_id && question_id===question.question_id && order==question.question_order` 把 PPR 结果（`res_id`、`mark_type`、`pprInfo`）回填到 `question_list[]`。

**read 的标识取值汇总**：`entity_id = exam.exam_id = real_id`（= content.json 的 `info.stid`，由 `res.json`/`struct.contents[].real_id` 对齐）、`order = 1`（硬编码）、`local_audio_key = "<setId>_<info.stid>_1"`、`original_key = "<setId>_<info.stid><info.audio>"`。

### 2.7 与其它族的差异

| 条件 | 走向 |
|---|---|
| `exam_type_collect === "collector.read"` 且 `exam_list.length === 1` | 专属单篇分支：本地缓存/远端 **XML** + `info.value` + `adjustWordColor`；用 `evaluation_type` 决定 `showEssay_2`/`showPhrase_2`/`showScoreDetail` |
| `exam_type_collect === "collector.read"` 且 `exam_list.length > 1` | **退化为 3q5a/generic 分支**：`c.showScore(_,d,0)` + `j()` + `set3q5aScore` |
| `collector.picture` | 在 `getRecordAudios`/`getAudioIds` 中与 read **完全合并**（同 case） |
| `collector.word` | 同组但 order = `info.xh`，播放 key 用 `info.xh` |
| `collector.3q5a/role/dialogue` | 同组，order = 子题 `xh` |
| `collector.repeat` | 同组，order = 子题 `xh`，但**题目展开阶段有 info_extra.json 特判** |
| `collector.choose/fill` | 不录音；走 `setPCChoose`/`setPcBlank`，且 `useExamStepper` 中有**本地答案校验** |

---

## 3. `collector.repeat`（跟读/重复，含 info_extra.json 与 subid 分组语义）

### 3.1 族判定与分发点

**(a) 结果页**：与 `3q5a/role/dialogue/word` 同支（`set3q5aScore`），见 §1 分发点①的 `switch("collector.read"!==n.examType.exam_type_collect&&(I()||c.showScore(_,d,0)),n.examType.exam_type_collect){case"collector.3q5a":case"collector.role":case"collector.dialogue":case"collector.word":case"collector.repeat":I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);break;case"collector.read":` 片段。
**(b) 录音回放 / 播放 id**：`getRecordAudios` 里 `case"collector.repeat":for(const n of r.info.fj)...`（见 §1 分发点②）；`getAudioIds` **没有** repeat 分支（只处理 read/picture/word → repeat 不产生 playback key）。
**(c) 题目展开（唯一在 index 里按族分叉的族）**：三处 `if("collector.repeat"===<examType>.exam_type_collect)`。

```javascript
"collector.repeat"===e.exam_type_collect){let e=function(e,t){const n=(e/t).toFixed(2),r=(e-(t-1)*n).toFixed(2);let o=[];for(let i=0;i<t-1;i++)o.push(n);return o.push(r),o};const t=await this.getJson(replacePathSeparator(`${this.getContentFilePath(r.real_id)}\\info_extra.json`));let o=lodashExports.cloneDeep(r.question_list),i=Number(t.length)-o.length;for(;i>0;){const e=o[o.length-1];o.push({...e,question_order:String(Number(e.question_order)+1)}),i--}const a=e(r.exam_score,o.length);let s=0;o.forEach((e,t)=>{e.question_count="1",e.question_score=a[t],s+=parseFloat(a[t])});const l=o.length;r.exam_count=String(l),r.exam_score=String(s),r.question_list=o,n+=l}
```
```javascript
if("collector.repeat"===r.exam_type_collect){const n=await this.getJson(replacePathSeparator(`${this.getContentFilePath(s.real_id)}\\info_extra.json`)),o=e.filter(e=>e.subid.includes("_0")),i=e.filter(e=>!e.subid.includes("_0"));let a=0;for(const e of n){let t=JSON.stringify(i);for(const o of e)t=t.replace(new RegExp(o.id,"g"),o.value);let n=JSON.parse(t);const r=o[o.length-1];let s=Number(r.order)||Number(r.order_num);const l=Number(s)+1,c=(e,t)=>e.replace(/_(\d+)$/,(e,n)=>`_${Number(n)+t}`);n=n.map(e=>{let t=e.subid;return s++,{...e,subid:c(t,a),order:`${l}_custom_order_${e.subid}_${s}`,order_num:String(s)}}),a++,o.push(...n)}t[r.set_id].steps=t[r.set_id].steps.concat(o)}else t[r.set_id].steps=t[r.set_id].steps.concat(e)}
```
```javascript
if("collector.repeat"===e.exam_type_collect){const e=await this.getJson(replacePathSeparator(`${this.getContentFilePath(u.real_id)}\\info_extra.json`)),n=t.filter(e=>e.subid.includes("_0")),r=t.filter(e=>!e.subid.includes("_0"));let o=0;for(const t of e){let e=JSON.stringify(r);for(const n of t)e=e.replace(new RegExp(n.id,"g"),n.value);let i=JSON.parse(e);const a=n[n.length-1];let s=Number(a.order)||Number(a.order_num);const l=Number(s)+1,c=(e,t)=>e.replace(/_(\d+)$/,(e,n)=>`_${Number(n)+t}`);i=i.map(e=>{let t=e.subid;return s++,{...e,subid:c(t,o),order:`${l}_custom_order_${e.subid}_${s}`,order_num:String(s)}}),o++,n.push(...i)}s=s.concat(n)}else s=s.concat(t)}
```

### 3.2 数据来源

| 层 | 文件/字段 |
|---|---|
| 题目骨架 | `template_<id>/ctrl.json`（优先 `ctrl_check.json`）→ `[{subid, order, filetype, model_pc, filename, dirtype, Extendedfield2/3, playtime, playhint, synchro, fileparams, ...}]` |
| 教材/题型清单 | `template_<id>/res.json` → `set_score`, `exam_type_list[].{exam_type_collect, exam_type_name, exam_type_score, exam_list[]}`，`exam_list[].{exam_id, exam_score, exam_count, question_list[]}`，`question_list[].{question_id, question_order, question_score, question_count, evaluation_type}` |
| 题面实体 | `content_<real_id>/content.json` → `{structure_type, info:{stid, value, ai, audio, video, translate, analyze, symbol, fj:[...]}}` |
| **分组/计数权威** | `content_<real_id>/info_extra.json` —— **只在 repeat 分支读取**，`t.length` 决定实际子题数 |
| 展示字段 | `content_<real_id>/info.json`（jsparam：`code_value` / `code_json_array` / `code_json_obj` / `image`）+ `content_highlight → <span class="color_repeat">` |

### 3.3 逐题抽出规则（repeat 的全套算法）

**数量改写（§3.1 片段 A / resjson）**：
1. 读 `info_extra.json`（数组），长度记为 `t.length`；
2. 克隆 `exam.question_list`；若 `t.length > question_list.length`，则**反复复制最后一题并把 `question_order` 递增 1**，直到数量相等：
   ```javascript
   let o=lodashExports.cloneDeep(r.question_list),i=Number(t.length)-o.length;for(;i>0;){const e=o[o.length-1];o.push({...e,question_order:String(Number(e.question_order)+1)}),i--}
   ```
3. 把 `exam_score` **按题数均分**（见 §3.5）；每题 `question_count="1"`；
4. 回写 `exam_count`、`exam_score`、`question_list`。

**步骤改写（§3.1 片段 B/C，两处逻辑逐字相同，仅变量名不同）**：
1. 目标模板的步骤集 `t` 按 `subid` 的 `{{code_id}}` 与 `exam.code_id` 匹配，并把 `subid` 里的占位替换为 `real_id`、`order_num = order`、`order = "<order>_custom_order_<subid>_<order>"`（见 §1 分发点④片段）；
2. **按 `subid` 是否含 `_0` 分成两组**：
   ```javascript
   o=e.filter(e=>e.subid.includes("_0")),i=e.filter(e=>!e.subid.includes("_0"));
   ```
   （B/C 两片段中变量名互换，语义一致：一组是"模板行"，一组是"基准步骤"）
3. 遍历 `info_extra.json` 的每一条 `e`，对**不含 `_0` 的组**做字符串级变量替换（`id` → `value`，`new RegExp(o.id,"g")` 全局替换），得到 `n`（一个新"跟读段"的步骤集）；
4. 取"模板组"最后一个元素的 `order/order_num` 作为基准 `s`，新步骤的 `order_num` 从 `s+1` 连续递增，`subid` 的**尾部数字偏移**：
   ```javascript
   const c=(e,t)=>e.replace(/_(\d+)$/,(e,n)=>`_${Number(n)+t}`);
   ```
   （`t` = 当前 info_extra 行序号 `a`/`o`），`order` 形如 `` `${l}_custom_order_${e.subid}_${s}` ``（`l = s+1` 的基准）；
5. 把新步骤**追加到含 `_0` 的组**（`o.push(...n)` / `n.push(...i)`），最终 `steps = steps.concat(<该组>)`。

**`action.record` 与题目的绑定（每个子题一个录音步骤）**：

```javascript
const i=e.subid.split("_")[1];if("action.record"===e.filetype){const t=e.exam.question_list.find(e=>e.question_order===i);e.question=t}
```
```javascript
const o=r.subid.split("_")[1];if("action.record"===r.filetype){const e=r.exam.question_list.find(e=>e.question_order===o);r.question=e}
```
→ `subid.split("_")[1]` 是题号，用它在 `exam.question_list` 里 `find(question_order===题号)`，把题目对象挂到 `step.question`。

**`info.fj[]`（跟读分段）字段** —— 客户端只读 `.xh`：
```javascript
case"collector.repeat":for(const n of r.info.fj)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),...
```
→ 一题 = `info.fj[]` 的一个元素，**题号 = `fj[].xh`**。

### 3.4 答案在哪里（核心）

> **结论：客户端完全不取 repeat 的答案。** repeat 是"跟读评测"：把学生录音送 ISE 引擎，引擎给出相似度/总分。

- `info` 被读字段：`stid`、`fj[].xh`、`value`（原文，用于 `info.json` 的 `content_highlight` 高亮）、`audio`；**没有 `answer`**（§0.4）。
- 全库对 repeat 相关的"参考答案"字段（`aswaudio`/`askaudio`/`value_bz`/`value_pf`/`std`/`keywords`）在 PC bundle 中 **0 命中**（§0.4）。
- 唯一与"答案内容"沾边的是 `processJsParam` 的 `content_highlight`：把 `info.json` 的 `content` 中命中 `content_highlight` 的子串包成 `<span class="color_repeat">` —— 这是**高亮待跟读句**，不是判分键：

```javascript
return o.content_highlight&&(o.content=o.content.replace(o.content_highlight,`<span class="color_repeat">${o.content_highlight}</span>`)),o
```
- 判分仍走 §2.4 的 ISE 通道（`category = evaluation_type`，repeat 常见 `read_sentence`），分值由 `calculateRealScore(引擎分(0–5), question_score)` 换算。

### 3.5 分值（含"按题数均分"完整逻辑）

```javascript
let e=function(e,t){const n=(e/t).toFixed(2),r=(e-(t-1)*n).toFixed(2);let o=[];for(let i=0;i<t-1;i++)o.push(n);return o.push(r),o};
...
const a=e(r.exam_score,o.length);let s=0;o.forEach((e,t)=>{e.question_count="1",e.question_score=a[t],s+=parseFloat(a[t])});const l=o.length;r.exam_count=String(l),r.exam_score=String(s),r.question_list=o
```
语义：`t = question_list.length`（= info_extra.json 长度）
- 前 `t-1` 题每题 `question_score = (exam_score / t).toFixed(2)`（**字符串两位小数**）；
- 最后一题 `question_score = (exam_score - (t-1)*n).toFixed(2)`（吞掉所有尾差）；
- `question_count = "1"`（每题一小问）；
- `exam_score` 被回写成 `parseFloat` 累加后的字符串（因此可能≠原始 `exam_score`，存在**尾差漂移**）；
- `exam_count = t`。

**百分比模式下的跨子题分值切分**（`useExamResult.j()`，read/repeat/role/dialogue/word/3q5a 共用）：

```javascript
function j(e,t,o,i){return t.reduce((t,r)=>{let s=o.filter(e=>e.entity_id===r.info.stid),n=0;const c=s.length,l=s.reduce((t,o)=>{n++;const r=function(e,t){const o=e.exam_list.find(e=>e.exam_id===t.entity_id);return o?o.question_list.find(e=>e.question_order===String(t.order)):{}}(e,o),{scoreFilter:s}=a(),[l,d]=s(r.question_score,o.real_score,i);let _=l,u=d;if(100==u&&c>1){let t=Number(r.question_score)/Number(e.exam_type_score);if(_=O(l,t,i,c),u=O(d,t,i,c),n==c){const t=u;u=(i?e.exam_type_score:100)-u*(c-1),_=_*u/t,_=h(_,i)}}const m=`${D}_${o.entity_id}_${o.order}${q}${_??0}${q}${u}${q}0`;return t.push(m),t},[]);return t=t.concat(l)},[])}
```
```javascript
function O(e,t,o,i){return t=t||1,o?e:h(e*t,i>1?2:0)}
```
语义：若某 `entity_id`（= `info.stid`）下有 `c` 条子题日志且满分被算成 100（百分比模式），则
- `t = question_score / exam_type_score`（该子题占题型满分的比例）；
- `_ = O(l,t,i,c)`（`O(e,t,o,i){return t=t||1,o?e:h(e*t,i>1?2:0)}`）：第 3 个实参 `i`（同步标志）为**真**时原样返回 `l`；为**假**时返回 `h(l*t, c>1?2:0)`（把显示分按 `t` 缩放、保留 2 位小数）；同理 `u = O(d,t,i,c)`；
- 最后一题（`n==c`）的满分改为 `(i?exam_type_score:100) - u*(c-1)`，并把该题得分按 `u/t` 反算 → **保证 c 个子题满分之和恰为 100（或 exam_type_score）**；
- 推送串格式：`` `${setId}_${entity_id}_${order}\x01${显示分??0}\x01${满分}\x010` ``（`q = String.fromCharCode(1)`，即 `\x01`；三段用 `\x01` 分隔，末段恒为 `0`）。

### 3.6 提交时每题的标识字段

- `entity_id = exam.exam_id`（= content.json `info.stid`）
- `order = info.fj[].xh`（回放查找与推送都用它）
- `set_id`、`resource_id`、`homework_id`、`mistake_id`、`serial_id`
- `source`：`action.record` 步骤的 `subid.split("_")[1]`（= `question_order`）
- 其余同 §2.6 的 16 字段载荷表。

### 3.7 与其它族的差异

| 条件 | 走向 |
|---|---|
| `exam_type_collect === "collector.repeat"` | index 的**三处**题目/步骤展开都进 repeat 分支：读 `info_extra.json`、按 `subid.includes("_0")` 分组、按题数均分 `exam_score` |
| 其它族 | `else` 直接 `concat`，不读 `info_extra.json`，不改写 `exam_count/exam_score/question_list` |
| 回放/播放 id | repeat 走 `info.fj[].xh`；`getAudioIds` 无 repeat 分支（**不产生 playback key**） |
| 是否存在 `collector.repeat_essay` / `repeat_dialogue` | **不存在**。PC 端表示"长文重复/对话重复"的方式就是：`exam_type_collect="collector.repeat"` + 用 `info_extra.json` 的条目数把 `question_list` **扩成 N 个子题**、再用 `subid` 的 `_0` 组决定插入位置；即**用"数据行数 + subid 后缀"表达，而不是新增族常量** |

---

## 4. `collector.role`（角色扮演／听选信息／回答问题型）

### 4.1 族判定与分发点

> **结论：PC 端 bundle 中 `collector.role` 没有任何独立分支。** 它在三处分发点全部与 `collector.3q5a` / `collector.dialogue` 写在同一个 `case` 列表里，成员间**逐字不可区分**。

```javascript
// chunks/useExamResult-d9fb4671.js（分发点①）
case"collector.3q5a":case"collector.role":case"collector.dialogue":case"collector.word":case"collector.repeat":I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);break;
```
```javascript
// chunks/useExamResult-d9fb4671.js  getRecordAudios（分发点②）
case"collector.3q5a":case"collector.role":case"collector.dialogue":for(const n of r.info.question)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:n.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;
```
- `getAudioIds`：**没有 role 分支**（只 read/picture/word）。
- index：**没有 role 分支**（只有 repeat）。
- `useExamStepper`：**没有 role 分支**（只有 choose/fill）。

### 4.2 数据来源

**(a) `content_<real_id>/content.json → info.question[]`** —— 子题数组（唯一结构来源）：

```javascript
case"collector.3q5a":case"collector.role":case"collector.dialogue":for(const n of r.info.question)e=`${D}_${r.info.stid}_${n.xh}`,...
```

**(b) `template_<id>/res.json → exam_list[].question_list[]`** —— 每个子题的评分口径（`question_order` / `question_score` / `question_count` / `evaluation_type` / `res_id`）。

**(c) `template_<id>/ctrl.json`** —— 每个子题一个 `action.record` step；step 与题目的绑定：

```javascript
const i=e.subid.split("_")[1];if("action.record"===e.filetype){const t=e.exam.question_list.find(e=>e.question_order===i);e.question=t}
```

**(d) `content_<real_id>/info.json` / `data.json`** —— `view.html` 的展示参数（如上用例的问句/提示）。

### 4.3 逐题抽出规则

- **一题 = `info.question[]` 的一个元素**；
- **题号 = 该元素的 `.xh`**（`getRecordAudios` 用 `n.xh` 作为 `order`）；
- 上报/录音时用的是 `question.question_order`（来自 `question_list`，见 §4.6），两者由内容包保证对齐；
- PC 端 bundle **只读取 `.xh`** 这一个子字段（`.info.question` 全库命中 1 次，即上面那句）。

**子题元素的实际字段形态（补充证据，见 §6 附录）**：
`{xh, ask, askaudio, answer, aswaudio, std:[{value,ai,audio}], ref:[], keywords, analyze, sucai, role}` —— 其中 `role:"a"/"b"` 区分角色（A/B 方）。该形态来自客户端内嵌 iframe 资源的示例数据（§6），**PC bundle 本身不读除 `xh` 以外的任何子字段**。

### 4.4 答案在哪里（核心）

> **结论：PC bundle 中 `collector.role` 的答案是"未找到"——客户端既不读 `answer`，也不读 `std`/`keywords`。**

证据：
- `.info.question` 在全库只有 1 次读取，且只取 `.xh`（§0.4 与 §4.3）；
- `.info.answer` 0 命中；`aswaudio`/`askaudio`/`keywords`（题目语义）0 命中；
- role 族的判分同 §2.4/§3.4：`action.record` → ISE 引擎（`category = question.evaluation_type`，role 常见 `simple_expression_ufi/ufk/ufj` 或 `topic`，见 `EvaluationTypes`）→ `score_detail.total_score`（0–5）→ `calculateRealScore(总分, question_score)`；
- 因此 role 的"语义参考答案"（`std[]` 的多个可接受问句、`answer` 的示范回答、`keywords` 关键词）**在 PC 客户端只作为内容包数据存在，由服务端评测引擎消费**；本地 bundle 里没有引用点。

**不确定性声明**：`std[]`（`{value, ai, audio}` 多条同义问句）与 `keywords` 的实际消费者是**原生 ISE 插件 + 服务端**；在本次只读的 bundle 内**未找到**读取点，故标注"客户端不读 / 消费方在 bundle 之外"，而非断言"服务端一定用"。

### 4.5 分值

- 题型满分：`res.json → exam_type_list[].exam_type_score`（useExamResult `l = t.exam_type_score ?? 5`）；
- 整套满分：`exam_list[].exam_score`（`action.record` 把它作为 `question_type_score` 上报）；
- 单题满分：`question_list[].question_score`；
- 引擎分换真实分：`calculateRealScore(引擎分, question_score) = 引擎分/5*question_score`（§2.5）；
- 多子题同一 `stid` 时的百分比切分兜底：`j()` + `O()`（§3.5），**role 与 3q5a/dialogue/word/repeat 共用**；
- 本地判分兜底（**不适用于 role**）：`m(a.question_score/a.question_count)` 只出现在 choose/fill 的本地答案校验里。

### 4.6 提交时每题的标识字段

```javascript
// chunks/useExamStepper-8ad3fbd7.js  action.record
const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};
```
→ 上报标识：**`entity_id = exam.exam_id`（= content.json `info.stid`）、`order = question.question_order`**（**注意不是 `xh`**）、`first_column_id`、`question_type_score = exam.exam_score`；
回放查找用 `order = xh`（`getRecordAudios`）。最终同步载荷字段同 §2.6（`useAudioSync` 的 16 字段）。

### 4.7 与其它族的差异

| 条件 | 走向 |
|---|---|
| `collector.role` / `collector.dialogue` / `collector.3q5a` | 三者**合并为同一 case**；唯一的族级差异体现在内容包（`structure_type`/`evaluation_type`/`info.question[]` 的字段）与 `res.json` 的分值 |
| `collector.word` | 同组，但 order 用 `info.xh`（单值，不用子题数组） |
| `collector.repeat` | 同组，但用 `info.fj[]` 且题目展开阶段有 `info_extra.json` 特判 |
| `collector.read` | 独立分支（XML 着色、`evaluation_type`→`showEssay_2/showPhrase_2`） |
| `collector.choose/fill` | 独立分支，且**有本地答案校验** |
| `collector.picture` | 只在 `getRecordAudios`/`getAudioIds` 与 read 合并；在结果页 switch 中**未出现**（未找到 picture 的 case；若内容包给出 `collector.picture`，在 `useExamResult` 的 switch 中会走 `default`（无分支）→ 标注为不确定/未找到） |

---

## 5. `collector.dialogue`（对话类）

### 5.1 族判定与分发点

> **结论：与 `collector.role` 完全同构 —— PC bundle 中不存在任何仅针对 `collector.dialogue` 的代码。** 全部 2 次出现都在 `chunks/useExamResult-d9fb4671.js`，且均与 `3q5a`/`role` 同 case（§1 分发点①、§4.1）。

`collector.dialogue` 在 index / useExamStepper / Examine / Comprehensive 中**0 命中**。

### 5.2 数据来源

与 §4.2 逐项相同（`content.json → info.question[]`、`res.json → exam_list[].question_list[]`、`ctrl.json → action.record` 步骤、`info.json`/`data.json`）。`collector.dialogue` **没有** `info_extra.json` 特判、**没有** `info.fj` 路径。

**已检索但未找到**：`collector.dialogue` 专属的数据文件、专属字段、专属 `evaluation_type`。`EvaluationTypes` 枚举（§0.4 `L` 片段）中与对话最接近的是 `topic` / `simple_expression*` / `free_expression`，但**bundle 内没有任何代码把 `collector.dialogue` 映射到其中某个 `evaluation_type`**（映射来自内容包）。

### 5.3 逐题抽出规则

同 §4.3：**一题 = `info.question[]` 的一个元素，题号 = `.xh`**（逐字同一句代码，见 §4.1 分发点②片段）。
`role` 字段（`"a"`/`"b"`）在 PC bundle 中**未被读取**，因此"对话的双方轮次"完全由内容包/iframe 负责。

### 5.4 答案在哪里（核心）

**未找到 —— 与 role 相同：客户端不读、也无法读取 dialogue 的答案。**
证据同 §4.4：`.info.question` 只取 `.xh`；`.info.answer` 0 命中；`askaudio`/`aswaudio` 0 命中；判分走 ISE（`action.record`）。

### 5.5 分值 // 5.6 提交标识 // 5.7 差异

- 分值：与 §4.5 **完全一致**（共用 `j()` + `set3q5aScore` + `scoreFilter` + `calculateRealScore`）。
- 提交标识：与 §4.6 **完全一致**（`entity_id=exam.exam_id`、`order=question.question_order`、回放用 `xh`）。
- 差异：**在 PC bundle 层面，`collector.role` 与 `collector.dialogue` 没有任何可观测差异**；差异只存在于内容包（`content.json` 的子题字段、`ctrl.json` 的步骤编排、`res.json` 的分值）与 iframe/引擎侧。

---

## 6. 补充证据：客户端内嵌 iframe web 应用的静态引用

> 说明：上面四条结论中"`info.question[]` 的子字段形态"无法从 PC bundle 取证（bundle 只读 `.xh`）。同一台机器上存在**该客户端下载的 iframe 资源**（`pc-listen*.js` 等，见 `chunks/DynamicIframe-bafff5bc.js` 的资源白名单）：
> ```javascript
> "js/pc-listen-answer.js","js/pc-listen-choose.js","js/pc-listen-data.js","js/pc-listen-default.js","js/pc-listen-fn.js","js/pc-listen-show-wordinfo.js","js/pc-listen.js",
> ```
> 我以**只读**方式查看了这份明文资源（未运行任何客户端/未抓包/未联网），路径 `/tmp/ets_web`（另有同一副本在 wine 前缀 `ETS/common/` 下）。**版本差异警告**：该资源的时间戳（2024-06/07）与 bundle 并非同一构建，仅作字段形态的**旁证**，不替代 bundle 证据。

**(a) iframe 侧同样按 `structure_type` 分叉，且 `set` 就是 `content.json` 数组**：

```javascript
	//前端处理序号 20210830
	var stid = "";
	var same = 1;
	for(var i in set){
		if( set[i] && set[i].structure_type && set[i].structure_type == 'collector.word'){
			if(stid == ""){
				stid = set[i].info.stid;
			}else{
				if(stid != set[i].info.stid){
					same = 0;
				}
			}
		}
	}
	if(same == 0){
		var index = 1;
		for(var i in set){
			if( set[i] && set[i].structure_type && set[i].structure_type == 'collector.word'){
				set[i].info.new_xh = index;

				index++;
			}
		}
	}
```
（`/tmp/ets_web/js/pc-listen-default.js`，`setData(set,page,set_id,order,type)` —— 与 `useExamResult` 的 `htmlData` 参数顺序一致）

**(b) 示例数据文件里对族的中文注释（原文）**：

```javascript
//A1 模仿朗读 A2 三问五答 A3 故事复述 A4 选择题 A5 填空题 A6 跟读与模仿 A7 朗读句子

var A1 = [{"
```

**(c) `collector.read` 的 content.json 实体形态（节选）**：

```javascript
var A1 = [{"structure_type":"collector.read","info":{"stid":"100110","value":"</p><p>At this time of year, there are still only five hours of daylight. But as the sun rises higher, each day lasts 40 minutes longer. It’s March, and with increasing sunlight, the first in a huge wave of migrants are arriving. Millions of seabirds a
```

**(d) `collector.repeat` 的 content.json 实体形态 + `info.fj[]` 字段（节选）**：

```javascript
var A6 = [{"structure_type":"collector.repeat","info":{"stid":"43436","value":"</p><p>One day in ancient Greece, King Hiero asked a crown maker to make him a golden crown. At first, he was very happy with it. Later, however, he began to doubt that it was a real golden crown. “Is it made completely o
```
```javascript
"fj":[{"value":"One day in ancient Greece, King Hiero asked a crown maker to make him a golden crown.","xh":"1","fs":"0.8","ai":"One day in ancient Greece, King Hiero asked a crown maker to make him a golden crown.","time":"11","audio":"fj1
```
→ `fj[]` 元素字段：`{value, xh, fs, ai, time, audio}`。**`value`（跟读文本）与 `ai`（同文本）是参考文本，不是判分键**；`time` 是每段的限定秒数。

**(e) "问题+回答"型（`collector.3q5a`）的子题字段形态 —— 供 role/dialogue 参照（节选）**：

```javascript
var A2 = [{"structure_type":"collector.3q5a","info":{"stid":"100113","value":"W: Mike, I just can’t believe supermodel Lily Cole graduated from the University of Cambridge.</br>M: That’s not surprising at all, Jane. Many
```
```javascript
"std":[{"value":"Did you graduate from Cambridge?","ai":"Did you graduate from Cambridge?","audio":""},{"value":"Are you a graduate from Cambridge?","ai":"Are you a graduate from Cambridge?","audio":""},{"value":"Did you graduate from the University of Cambridge?","ai":"Did you graduate from the University of Cambridge?","audio":""},{"value":"Are you a graduate from the University of Cambridge?","ai":"Are you a graduate from the University of Cambridge?","audio":""},{"value":"Were you a graduate from Cambridge?","ai":"Were you a graduate from Cambridge?"
```
→ 子题字段：`{xh, ask, askaudio, answer, aswaudio, std:[{value,ai,audio}], ref:[], keywords, analyze, sucai, role}`
→ `std[]` 是**多条可接受问句**（语义等价集合），`answer` 是**示范回答**，`keywords` 是**关键词串（`|` 分隔）**，`aswaudio` 是**示范回答音频**。
→ **注意**：这些字段在 PC bundle 中 **0 引用**（§0.4），因此它们的消费者只能是服务端评测引擎或 iframe/原生插件，**不能在客户端源码层面确认**，标注为"未在 bundle 内找到"。

**(f) 未找到项**：`/tmp/ets_web` 中**没有 `collector.role` / `collector.dialogue` 的示例数据**，也没有任何读取 `info.question[].answer/std/keywords` 的 JS（`grep -l 'aswaudio|askaudio' js/*.js` 无命中）。因此 role/dialogue 的子题字段语义**只能由与 3q5a 的分支合并关系 + 示例结构推断**，属"不确定"。

---

## 7. 与常见直觉不同 / 值得注意的点

1. **"答案"这个词在客户端里不是答案**：`Examine`/`MockExamDetails` 的 `questions[].answer` 是 UI 布尔"是否已作答"（由 `score_detail.choose_id` 存在性决定）：
   ```javascript
   t&&(t.answer=void 0!==a.score_detail.choose_id&&""!=a.score_detail.choose_id)
   ```
   不要误当成标准答案。

2. **`standard_score` 是一个"双关"字段名**：
   - 在 ISE XML 里 `standard_score` = **"标准度"维度分**（`parseIseResult` 中 `{name:"标准度",attr:"standard_score"}`）；
   - 在本地日志里 `standard_score` = **引擎原始总分（0–5）**（`Comprehensive`：`standard_score:e.score_detail.total_score`）；
   - `calculateAndFormatScoresWithExamType` 用 `f(t.standard_score, e.question_score)`（`f = calculateRealScore`）把它换算成真实分。
   同一个名字，两种语义，读代码时极易误判。

3. **`calculateRealScore(e,t) = e/5*t`** ⇒ 讯飞 ISE 的分数是 **5 分制**，不是百分制。`ConvertToPercentScore(e,t)=e/t*100` 才做百分比化。`j()` 里的 `100==u` 判断正是"是否处于百分比模式"。

4. **`collector.read` 会因为 `exam_list.length` 换分支**：单篇走 XML 着色专属路径，多篇**退化成 generic 的 `set3q5aScore` 路径**（`case"collector.read":n.examType.exam_list.length>1?...:function(...)`）。

5. **read 单篇分支里硬编码兜底 `??5`**：`l=t.exam_type_score??5` —— 题型满分缺失时按 5 分算。

6. **read 的 `<br>` 修补判断写的是 `"<\br>"`**（字符串字面量里带反斜杠），与 `info.value` 里实际出现的 `</br>` 不同，因此该 `if` 分支**几乎总是成立**：
   ```javascript
   let w=d.info.value;-1===w.indexOf("<\br>")&&(w="</br>"+w.replace(/<\/p>/g,"</p></br>"));
   ```

7. **repeat 的"分值均分"用 `toFixed(2)` 字符串而非数值**，且把 `exam_score` 回写成累加结果 ⇒ **尾差会给最后一题，且总分可能漂移**（`(e-(t-1)*n)` 用字符串 `n` 参与算术）。

8. **`subid` 是 repeat 的隐藏主键**：`subid` 形如 `{{code_id}}_N`，`_0` 组是"模板行"，非 `_0` 组是"基准行"，`_(\d+)$` 的数字是**行偏移**。`order` 最终形如 `"<order>_custom_order_<subid>_<order>"`。**删除 `_0` 判定或改 `order` 格式会直接破坏 repeat 的题序。**

9. **`getAudioIds` 只覆盖 read/picture/word**：role/dialogue/repeat 的"回放"不走 `getAudioIds`，而是 `getRecordAudios`（`info.question[].xh` / `info.fj[].xh`）。混用会导致回放键对不上。

10. **只有 choose/fill 在客户端本地判分**：
    - choose：`content.json → info.xtlist[].answer === xxlist[].xx_mc`
    - fill：`content.json → info.std[].value`，且**支持 `/` 分隔的多解**并做 `trim().toLowerCase()` 归一：
    ```javascript
    return s=X(n),X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),...
    ```
    read/repeat/role/dialogue **没有**对应逻辑。

11. **`pc_xst_dict.json` 不是答案表**，是系统单词词典（写入 IndexedDB 的 `wordDict`）。"本地判分表"在 bundle 内**不存在**。

12. **`chunks/index-5543d811.js` 与 `index-5543d811.js` 字节完全相同**（md5 一致），统计族常量时不要把它的计数当成额外证据。

13. **read 的 `read_chapter` 有一段引擎分数修补**：`integrity_score<4` 时用"学习引擎"完整度去压低"模考引擎"总分（`adjustStudyEngineXml`）。这是 read 独有的分数后处理。

14. **`EvaluationTypes` 是"引擎型题型"的总开关**：`["read_word","read_chapter","read_sentence","read_choice","simple_expression_ufi","simple_expression_ufk","simple_expression_ufj","topic","simple_expression","free_expression"]`。`calculateRealScoreWithCategory(e,t,n)` 只对这 10 种做 5 分制换算。role/dialogue 的真实 `evaluation_type` 若不在其中，分值换算路径会不同 —— 这也是判断 role/dialogue 到底走哪种引擎的**关键抓手**（但具体取值在内容包，bundle 内未找到）。

15. **版本/一致性风险（必须知道）**：`chunks/useExamStepper-8ad3fbd7.js` 以 `import{u as Z}from"./useRecordResult-fbe9e75b.js"` 引入 `Z`，却调用了 `Z.record / Z.stop / Z.destroy / Z.onRecordEnd / Z.onRecordError / Z.countdownSeconds / Z.evaluateAudioFile`；而该 chunk **只导出 `getResult`**（`export{m as u}`）。这些方法在 `index-5543d811.js` 的 `useRecorder`（`dX as useRecorder`）中定义。⇒ **该快照的 useExamStepper 与 useRecordResult 不是同一构建产物**，引用这类调用时需回到 index 的 `useRecorder` 取证。

16. **`showEssay_2` / `showPhrase_2` / `showScoreDetail` / `set3q5aScore` / `setPCChoose` / `setPcBlank` 都是 iframe（`win`）注入的方法**，均在 bundle 外。bundle 只负责组织参数顺序（如 `\x01` 分隔的推送串）。

17. **PPR（题源/评分资源）与提交用同一套 id 口径**：`processExamPPRParams` 用 `{entity_id: exam.real_id, order: question.question_order}` 请求 PPR，`mergePPRes` 再用 `entity_id===exam_id && question_id===question.question_id && order==question.question_order` 回填 `res_id`/`mark_type`/`pprInfo`（`pprInfo` 进而决定 `engine_type`/`isEtsOpen`/`mark_local_file`）。**漏掉 `question_id` 会导致 role/dialogue 这类多子题题型回填错题。**

---

## 8. 结论速查表

| 项 | `collector.read` | `collector.repeat` | `collector.role` | `collector.dialogue` |
|---|---|---|---|---|
| 结果页分发 | **独立 case** | 与 3q5a/role/dialogue/word 合并 | 与 3q5a/dialogue/word/repeat 合并 | 同 role（同一 case） |
| index 题目展开特判 | 无 | **有（3 处，读 info_extra.json）** | 无 | 无 |
| useExamStepper 特判 | 无 | 无 | 无 | 无 |
| 题目容器 | content.json 本体（1 篇 = 1 题） | `info.fj[]` + info_extra.json 行数 | `info.question[]` | `info.question[]`（同 role） |
| 题号 | 恒为 `1`（硬编码） | `info.fj[].xh`（回放）/ `question_order`（上报） | `.xh`（回放）/ `question_order`（上报） | 同 role |
| 客户端取答案 | **否**（仅取评测 XML + `info.value` 着色） | **否** | **否**（`.xh` 之外不读） | **否** |
| 判分来源 | 服务端/原生 ISE XML（`read_chapter/read_sentence/read_word/topic`） | ISE（跟读） | ISE（`simple_expression*`/`topic` 等，取值在内容包） | 同 role |
| 分值均分机制 | 多篇时走通用 `j()` | **`info_extra` 行数均分 exam_score + `j()` 百分比切分** | `j()` 百分比切分 | 同 role |
| 提交 id | `entity_id`+`order=1` | `entity_id`+`order=xh` | `entity_id`+`order=question_order` | 同 role |
| 专属渲染 HTML | `pc-listen-answer.html`（结果）/`pc-listen.html`（练习） | `pc-listen.html` | `pc-listen.html` | `pc-listen.html` |
