# 词汇题型官方语义 + 考试流程/结果/提交规则（PC 端静态逆向）
> 分析对象：官方「E听说」PC 端前端明文源码。**全程仅静态阅读，未运行客户端、未抓包、未联网。**
> 源码根目录：`/home/shuaiqiu/Projects/ets源码恢复/pc_client_web`
> 主 bundle：`index-5543d811.js`（3.48 MB，minified，`chunks/index-5543d811.js` 与它 **MD5 完全相同**：`28bdb58380a09ff79f501728c055c998`）。
## 0. 材料完整性声明（先读，影响可信度判断）
- 主 bundle 内 `import("./X.js")` 引用了 **193** 个 chunk，本地 `chunks/` 只有 **195** 个文件，**缺失 5 个**。
- 与词汇题型 UI 直接相关的缺失 chunk（**结论中凡涉及这些页面的具体渲染/提交，本文只能给到调用点，不能给到实现**）：

- 已在**全部**明文文件中检索且 **0 命中**的关键词（用于证明「PC 端没有中文题型名」而不是「我没找」）：
  `听音选义`、`默写词汇`、`朗读词汇`、`听写词汇`、`英译中`、`中译英`、`听音`、`默写`、`听写`、`选义`、`拼写`、`释义` —— 全部 0 命中。
  → 中文题型名只存在于移动端 / 服务端 / 缺失的 `WordQuestion*` chunk 中；PC 端只用**数字 `question_type`**。

---

# 第一部分：`collector.word` 族语义

## 1.1 族判定与分发点
`exam_type_collect` 是「套题（听说考试模板）」的题型族标识。PC 端一共出现这些取值：
`collector.read` / `collector.picture` / `collector.word` / `collector.choose` / `collector.fill` / `collector.3q5a` / `collector.role` / `collector.dialogue` / `collector.repeat`

### (a) 结果页总分派发：`useExamResult-*.js`
**结果页按 exam_type_collect 分发** — `chunks/useExamResult-d9fb4671.js`

```javascript
switch("collector.read"!==n.examType.exam_type_collect&&(I()||c.showScore(_,d,0)),n.examType.exam_type_collect){case"collector.3q5a":case"collector.role":case"collector.dialogue":case"collector.word":case"collector.repeat":I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);break;case"collector.read":n.examType.exam_list.length>1?I()||setTimeout(()=>{c.showScore(_,d,0);const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0):function(e,t,o,i,r){setTimeout(async()=>{const{htmlData:s,hasLogScore:n,set_module_id:c}=t,l=t.exam_type_score??5,d=s[0];if(d){const s=o.find(e=>e.entity_id===d.info.stid)||{},n=await x({set_id:D,entity_id:s.entity_id,order:s.order,mistake_id:t.mistake_id,homework_id:t.homework_id,remote_url:s.detail_file});let c="";c=n&&!n.includes("http")?await C(n):await p(s.detail_file);const{real_score:_,detail:u,standard_score:m,manual_modified:f}=s,h=y(u),g=t.examType.exam_list.find(e=>e.exam_id===d.info.stid);let w=d.info.value;-1===w.indexOf("<\br>")&&(w="</br>"+w.replace(/<\/p>/g,"</p></br>"));const{scoreDetail:k,essayData:S}=await $(w,c,{first_column_id:i,stu_total:m,stu_real_score:m,multidimensional_results:h,exam_type_score:l,manual_modified:!0===f}),b="number"==typeof _?_:k.stu_real_score,{scoreFilter:q}=a(),[T,v]=q(l,b,r);if(I()||e.showScore(v,T,0),g&&g.question_list){const t=g.question_list[0];if(!I()&&("read_chapter"===t.evaluation_type?e.showEssay_2.apply(null,S):"read_sentence"===t.evaluation_type&&e.showPhrase_2.apply(null,S),"read_word"!==t.evaluation_type)){const t=k?.dimension_result||[];e.showScoreDetail.apply(null,t.slice(0,3))}}}},0)}(c,n,e,o,i);break;
```
注意：`collector.word` 与 `3q5a/role/dialogue/repeat` **共用同一段代码** —— 调 `set3q5aScore`（答案对比协议），而不是 `collector.choose` 那套 `setPCChoose`。

**collector.word 的录音回填与音频 id 规则** — `chunks/useExamResult-d9fb4671.js`

```javascript
getRecordAudios:async function(e,t,o){const i={};try{for(const r of e){let e,s,a;switch(r.structure_type){case"collector.read":case"collector.picture":e=`${D}_${r.info.stid}_1`,a=t.find(e=>e.entity_id===r.info.stid&&"1"===String(e.order)),s=a&&await w({set_id:D,entity_id:r.info.stid,order:1,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;case"collector.word":e=`${D}_${r.info.stid}_${r.info.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===r.info.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:r.info.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;case"collector.3q5a":case"collector.role":case"collector.dialogue":for(const n of r.info.question)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:n.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??"";break;case"collector.repeat":for(const n of r.info.fj)e=`${D}_${r.info.stid}_${n.xh}`,a=t.find(e=>e.entity_id===r.info.stid&&String(e.order)===n.xh),s=a&&await w({set_id:D,entity_id:r.info.stid,order:n.xh,mistake_id:o.mistake_id,homework_id:o.homework_id,remote_url:a?.audio}),i[e]=s??a?.audio??""}}}catch(r){}return i},
```
**collector.word 的 playback id：`${set_id}_${stid}_${xh}`** — `chunks/useExamResult-d9fb4671.js`

```javascript
getAudioIds:function(e){let t={original:"",playback:""};if(e.length){const o=e[0];switch(t.original=`${D}_${o.info.stid}${o.info.audio}`,o.structure_type){case"collector.read":case"collector.picture":t.playback=`${D}_${o.info.stid}_1`;break;case"collector.word":t.playback=`${D}_${o.info.stid}_${o.info.xh}`}}return t},
```
→ **词汇族的一条记录主键是 `set_id + info.stid + info.xh`**（`xh`=小题序号），不是 `order`。

### (b) 作业入口派发：`HomeWorkPractice.startHomework` 的 `case FirstColumnType.VocabularyModule`
词汇作业/练习的 `first_column_id` 是 **127 = `VocabularyModule`**（注意：`first_column_id` 与 `question_type` 是两套完全不同的枚举，勿混）：
**FirstColumnType 全量枚举（节选）** — `index-5543d811.js`

```javascript
var FirstColumnType=(e=>(e[e.SimulationExam=1]="SimulationExam",e[e.ListeningSpeakingSpecial=2]="ListeningSpeakingSpecial",e[e.SpecialTraining=3]="SpecialTraining",e[e.SynchronousPractice=4]="SynchronousPractice",e[e.ListeningSpeakingSynchronous=5]="ListeningSpeakingSynchronous",e[e.Phonetics=6]="Phonetics",e[e.Composition=8]="Composition",e[e.Shadowing=9]="Shadowing",e[e.ReadingWriting=10]="ReadingWriting",e[e.UnscoredComposition=11]="UnscoredComposition",e[e.ReadingWritingSimulation=12]="ReadingWritingSimulation",e[e.ListeningSimulation=88]="ListeningSimulation",e[e.SynchronousListeningSpeakingUnit=92]="SynchronousListeningSpeakingUnit",e[e.Mistake=103]="Mistake",e[e.SynchronousListening=104]="SynchronousListening",e[e.SynchronousListeningSpeakingClass=108]="SynchronousListeningSpeakingClass",e[e.SynchronousDubbing=109]="SynchronousDubbing",e[e.FunDubbing=110]="FunDubbing",e[e.HolidayHomework=112]="HolidayHomework",e[e.VocabularyModule=127]="VocabularyModule",e[e.GradedReading=119]="GradedReading",e[e.SynchronousTextbook=159]="SynchronousTextbook",e[e.SynchronousTextbookRecite=215]="SynchronousTextbookRecite",e[e.StudyPlan=168]="StudyPlan",e[e.PhoneticNew=331]="PhoneticNew",e))(FirstColumnType||{})
```
**VocabularyModule 分支 → v()** — `index-5543d811.js`

```javascript
v=(e,t="0",n,r)=>{if(e.pk)if(n){const t=e.mine.scores[0].word;t.map(e=>{e.point=e.score_rate}),h({name:"pkResult",state:{data:JSON.stringify({...e,my:t,useTime:e.mine.scores[0].use_time,from:e.from,opponent:e.opponent,resource_id:e.resource_id,datas:e.wordInfo.word.map(e=>({material_id:e.material_id,struct:e})),question:e.wordInfo.word_question})}},r)}else router.push({name:"pkMatch",query:{title:e.name||""},state:{data:JSON.stringify(e)}});else n?((e,t)=>{h({name:"homeworkWordQuestionResult",state:{data:JSON.stringify({...e})}},t)})(e,r):h({name:"homeworkWordQuestion",query:{selectIndex:0},state:{data:JSON.stringify(e)}},r)},
```
`v(e,t,n,r)` 的 `i` 参数 = `reviseAssignment`（订正），`e.pk` 决定走 PK 还是普通词汇题；非 pk 且非订正 → `homeworkWordQuestion`。

**PracticeType 枚举（决定单例走作业还是复习分支）** — `index-5543d811.js`

```javascript
var PracticeType=(e=>(e[e.homework=1]="homework",e[e.review=2]="review",e[e.default=0]="default",e))(PracticeType||{})
```
**ModuleIDType（旧套题 set_module_id，与 first_column_id 并列参与分值换算）** — `index-5543d811.js`

```javascript
var ModuleIDType=(e=>(e.TBXL_ID="1",e.TSZX_ID="2",e.DXZX_ID="3",e.MNKS_ID="4",e.TLZX_ID="5",e))(ModuleIDType||{})
```
**路由清单（与词汇/考试相关）** — `index-5543d811.js`

```text
paths: ["/homeworkList", "/mockExamList", "/mockExamDetail", "/mockExamResult", "/modelTestMode", "/pkMatch", "/pkMatch_resource", "/pk", "/pkRank", "/pkResult", "/wordStudy", "/wordPractice", "/wordPracticeResult", "/wordQuestion", "/homeworkWordQuestion", "/homeworkWordQuestionResult", "/wordQuestionResult", "/wordList", "/wordList_resource", "/wordPractice_resource", "/wordQuestion_resource", "/wordQuestionResult_resource", "/wordPracticeResult_resource"]

name : ["homeworkList", "mockExamList", "mockExamDetail", "mockExamResult", "modelTestMode", "pkMatch", "pkMatch_resource", "pk", "pkRank", "pkResult", "wordStudy", "wordPractice", "wordPracticeResult", "wordQuestion", "homeworkWordQuestion", "homeworkWordQuestionResult", "wordQuestionResult", "wordList", "wordList_resource", "wordPractice_resource", "wordQuestion_resource", "wordQuestionResult_resource", "wordPracticeResult_resource"]
```


## 1.2 题目数据来源（字段层级）
词汇题有三条独立的数据来源，**层级结构不同**，这是最容易搞错的地方：

### 来源 A：词汇作业 struct（`g/homework/struct`）
**HomeworkPractice.requestHomeworkStructDetail** — `index-5543d811.js`

```javascript
E=async e=>{if(e.first_column_id==FirstColumnType.PhoneticNew)return requestPhoneticHomeworkStruct({homework_id:e.homework_id||e.id,resource_id:e.resource_id});if(e.first_column_id==FirstColumnType.VocabularyModule){const t=await requestHomeworkWordStruct({...e});return 0==t?.code&&isBcHomeworkStruct(t.body)&&await useWordHomeworkSource().applyResourceContentToStruct(t.body),t}return requestHomeworkStruct({...e})}
```
`g/homework/struct`（`requestHomeworkWordStruct`）响应体：
```text
body = {
  source: 1?,                       // 1 = 资源平台(BC)版本词汇作业
  unit_code: "...",
  word:         [ { id, material_id, item_id, content, translate, eg, phonics, audio_url, resourceResId, resourceCategory, grasp, in_class, collect, ... } ],
  word_question:[ { id, item_id, question_item_id, question_type, question_type_name, option, answer, translate, content, audio_url, phonics, stem, resourceResId, resourceCategory, ... } ]
}
```

判定与字段搬运的完整代码：
**isBcHomeworkStruct / collectWordItemIds / buildRpQuestionMap** — `index-5543d811.js`

```javascript
MAX_WORD_BATCH=100,isBlank=e=>null==e||""==`${e}`;function chunk(e,t=MAX_WORD_BATCH){const n=e||[],r=[];for(let o=0;o<n.length;o+=t)r.push(n.slice(o,o+t));return r}function isBcHomeworkStruct(e){return 1===Number(e?.source)}function collectWordItemIds(e){const t=new Set,n=[];for(const r of e?.word||[]){const e=r?.item_id;if(isBlank(e))continue;const o=`${e}`;t.has(o)||(t.add(o),n.push(o))}return n}function buildRpQuestionMap(e){const t={};for(const n of e||[])for(const e of n?.questions||[]){const n=e?.resourceQuestionId;if(isBlank(n))continue;const r=e?.details?.[0]||{};t[`${n}`]={questionTypeId:e?.questionTypeId,answer:r.answer,option:r.option}}return t}function applyRpQuestionsToWordQuestion(e,t){let n=0;const r=[],o=t||{};for(const i of e||[]){const e=i?.question_item_id;if(isBlank(e))continue;const t=o[`${e}`];t?(null!==t.option&&void 0!==t.option&&(i.option=t.option),null!==t.answer&&void 0!==t.answer&&(i.answer=t.answer),n++):r.push(`${e}`)}return{replaced:n,missing:r}}function toBracketPhonics(e){const t=`${e??""}`.trim();if(!t)return"";if(t.startsWith("[")&&t.endsWith("]"))return t;const n=t.replace(/^\/+/,"").replace(/\/+$/,"").trim();return n?`[${n}]`:""}function wordFindId(e){return e?null!==e.id&&void 0!==e.id?e.id:null!==e.itemId&&void 0!==e.itemId?e.itemId:null!==e.item_id&&void 0!==e.item_id?e.item_id:e.itemCode:null}function buildExampleEgJson(e,t){const n=e?.content?.groups?.[0]?.questions?.[0]?.exampleSentences||[],r=n.length?n:t?.exampleSentences||[],o=[];for(const i of r){const e=`${i?.sentenceText?.value??""}`.trim(),t=`${i?.sentenceContent?.value??""}`.trim();""===e&&""===t||o.push(""===e?{en:t,cn:""}:{en:e,cn:t})}return o.length?JSON.stringify(o):""}function pickReadQuestion(e){const t=e?.content?.groups||[];let n=null,r=null;for(const o of t)for(const e of o?.questions||[]){r||(r=e);const t=e?.answer;if(t?.pprFile){if("read_word"===t.type)return e;n||(n=e)}}return n||r||null}function buildItemIdToWordContent(e){const t={};for(const n of e||[]){const e=wordFindId(n);if(isBlank(e))continue;const r=pickReadQuestion(n),o=r?.ukSentences?.[0]?.sentenceText?.value,i=r?.usSentences?.[0]?.sentenceText?.value,a=toBracketPhonics(o||i||""),s=[];for(const t of r?.meanSentences||[]){const e=t?.sentenceText?.value,n=[(Array.isArray(e)?e.join(" "):`${e??""}`).trim(),`${t?.sentenceContent?.value??""}`.trim()].filter(Boolean).join(" ");n&&s.push(n)}const l=r?.questionAudio?.value,c=isBlank(l)?"":`${n?.baseUrl??""}${l}${n?.baseSign??""}`,u=r?.answer?.pprFile,d=isBlank(u)?"":`${u}`.replace(".ppr",""),p=`${r?.answer?.type??""}`,f=buildExampleEgJson(n,r);t[`${e}`]={phonics:a,translate:s.join("，"),audioUrl:c,eg:f,resId:d,category:p}}return t}function applyWordContentToStruct(e,t){if(!e||!t)return 0;let n=0;const r=e=>{if(!e||isBlank(e.item_id))return;const r=t[`${e.item_id}`];r&&(r.translate&&(e.translate=r.translate),r.eg&&(e.eg=r.eg),e.phonics=r.phonics,r.audioUrl&&(e.audio_url=r.audioUrl),r.resId&&(e.resourceResId=r.resId),r.category&&(e.resourceCategory=r.category),n++)};for(const o of e.word||[])r(o);for(const o of e.word_question||[])r(o);return n}
```
**applyRpQuestionsToWordQuestion** — `index-5543d811.js`

```javascript
function applyRpQuestionsToWordQuestion(e,t){let n=0;const r=[],o=t||{};for(const i of e||[]){const e=i?.question_item_id;if(isBlank(e))continue;const t=o[`${e}`];t?(null!==t.option&&void 0!==t.option&&(i.option=t.option),null!==t.answer&&void 0!==t.answer&&(i.answer=t.answer),n++):r.push(`${e}`)}return{replaced:n,missing:r}}
```
**applyWordContentToStruct / pickBcIds（字段层级不动，只做覆盖）** — `index-5543d811.js`

```javascript
function applyWordContentToStruct(e,t){if(!e||!t)return 0;let n=0;const r=e=>{if(!e||isBlank(e.item_id))return;const r=t[`${e.item_id}`];r&&(r.translate&&(e.translate=r.translate),r.eg&&(e.eg=r.eg),e.phonics=r.phonics,r.audioUrl&&(e.audio_url=r.audioUrl),r.resId&&(e.resourceResId=r.resId),r.category&&(e.resourceCategory=r.category),n++)};for(const o of e.word||[])r(o);for(const o of e.word_question||[])r(o);return n}
```
**useWordHomeworkSource：资源平台直连（/rop/open/...）** — `index-5543d811.js`

```javascript
const useWordHomeworkSource=()=>{const{getResourceData:e}=useSourcePlatform(),t=async(t,n,r)=>{const o=(r||[]).filter(e=>null!=e&&""!=`${e}`);if(!o.length)return[];const i=[];for(const a of chunk(o,MAX_WORD_BATCH))try{const r=await e(t,{[n]:a}),o=r?.data||[];Array.isArray(o)&&i.push(...o)}catch(e3){LoggerUtils.logCustom(`资源平台词汇作业直连失败 ${t} 批量=${a.length}: ${e3?.message||e3}`,{type:2})}return i},n=async e=>await t("/rop/open/v1/wordItem/listByWordIds","wordIds",e),r=async e=>await t("/rop/open/v2/word/findByIds","ids",e);return{requestWordItemsByWordIds:n,requestWordDetailsByIds:r,applyResourceContentToStruct:async e=>{if(!isBcHomeworkStruct(e))return!1;const t=collectWordItemIds(e);if(!t.length)return LoggerUtils.logCustom("词汇作业 source=1 但 struct.word 无 item_id，降级用易听说内容",{type:2}),!1;try{const[o,i]=await Promise.all([n(t),r(t)]),{replaced:a,missing:s}=applyRpQuestionsToWordQuestion(e?.word_question||[],buildRpQuestionMap(o));return s.length&&LoggerUtils.logCustom(`词汇作业资源平台题目未命中 ${s.length} 题（沿用易听说题面）：${s.slice(0,10).join(",")}`,{type:2}),applyWordContentToStruct(e,buildItemIdToWordContent(i)),a>0}catch(e3){return LoggerUtils.logCustom(`词汇作业资源平台内容替换失败，降级用易听说内容: ${e3?.message||e3}`,{type:2}),!1}}}},
```
→ 资源平台接口：`/rop/open/v1/wordItem/listByWordIds`（body `{wordIds:[...]}`）、`/rop/open/v2/word/findByIds`（body `{ids:[...]}`），**每批 100 个**（`MAX_WORD_BATCH=100`）。
→ 资源平台题目结构：`item → questions[] → { resourceQuestionId, questionTypeId, details[0].{option,answer,translate} }`。

### 来源 B：词汇练习 struct（`g/word/struct`，PK / 单元练习）
**PKMatch 取词 + 取题** — `chunks/PKMatch-d8a7c872.js`

```javascript
const Z=async()=>{if(M.value=1,null!=J.wordInfo)return void(L=setTimeout(()=>{X.value={serial_id:"",opponent:J.opponent},M.value=2,L=setTimeout(()=>{o.logCustom("leave PK-match.vue",{tag:"UI_PAGE"}),k.replace({name:"pk",query:{title:Q},state:{data:JSON.stringify({...J,question:z(J.wordInfo.word_question),opponent:X.value,resource_id:J.resource_id,datas:null!=J.wordInfo?Y.value:H(t)})}})},2e3)},0));let e=[],a=[],t=[];if(J.word)e=J.word.map(e=>e.id),t={...J.word,question:[J.word_question]},a=J.word_question;else{e=Y.value.map(e=>e.struct.id);try{let s=await I({word_ids:e});if(0==s.code&&(t=s.body.data.filter(e=>e.question.length>=6),a=t.map(e=>e.question.filter(e=>19==e.question_type)[0])),0!=s.code||!a.length)return M.value=3,void o.logCustom("没有符合的单词列表",{type:2})}catch(s){return void(M.value=3)}}try{let s=await q({unit_id:B.getUnitId,book_id:B.getBookId,question_ids:a.map(e=>e.id).join("_"),word_ids:e.join("_"),name:D.getDisplayName});L=setTimeout(()=>{0==s.code?(X.value=s.body,M.value=2,L=setTimeout(()=>{o.logCustom("leave PK-match.vue",{tag:"UI_PAGE"}),k.replace({name:"pk",query:{title:Q},state:{data:JSON.stringify({...J,question:t,opponent:s.body,datas:null!=J.wordInfo?Y.value:H(t),list:F})}})},2e3)):M.value=3},2e3)}catch(i){M.value=3}},z=e=>b.shuffle(e),H=e=>{let a=[];return e.forEach(e=>{let t=Y.value.filter(a=>e.material_id==a.material_id)[0];a.push(t)}),a};
```
`g/word/struct` 请求体只有 `{word_ids: [...]}`；响应 `body.data` 是数组，元素形如：
```text
[ { id,                     // 单词 id（= 后文的 word_id）
    material_id, struct:{ id, content, translate, audio_url, phonics, eg, ... },
    grasp,                   // 非 0 = 已掌握
    question: [ { id,        // 题目 id（= 后文的 question_id）
                  question_type, question_type_name, option, answer, translate,
                  content, audio_url, audio_size, phonics, stem, ... } ] } ]
```

**wordsPractice store（startWordPractice / startWordPracticeResource）** — `chunks/wordsPractice-857c490e.js`

```javascript
import{eI as t,hI as e,gm as r,at as s,r as o,dL as n,aS as i}from"./index-5543d811.js";const d=t("wordsPracticeStore",{state:()=>({questions:[],unorderedQuestions:[],words:[],questionTypeM:{},currentIndex:0,currentStep:0,unit_id:0,type:0,wholeWords:[],defaultWordCount:2,showContent:"2"}),getters:{getQuestions:t=>t.questions,getWords:t=>t.words,getWholeWords:t=>t.wholeWords,getCurrentIndex:t=>t.currentIndex,getCurrentStep:t=>t.currentStep,getUnitId:t=>t.unit_id,getType:t=>t.type},actions:{setQuestions(t){this.questions=t},setWords(t){this.words=t},setCurrentStep(t){this.currentStep=t},setCurrentIndex(t){this.currentIndex=t},setWholeWords(t){this.wholeWords=t},setUnitId(t){this.unit_id=t},getWordCountList(t){let e=0;e=Math.floor(t/5);let r=[{value:"5",label:"5个"},{value:"10",label:"10个"},{value:"15",label:"15个"},{value:"20",label:"20个"},{value:"25",label:"25个"},{value:"30",label:"30个"}].slice(0,e);return r.push({value:`${t}`,label:"全部"}),r},async startWordPractice(t,n,i,d,u=0,a=0,l=0,h=null){this.currentIndex=l,this.type=d,this.currentStep=0==d?u:2,this.wholeWords=t,this.unit_id=n;let c=t.filter(t=>!t.grasp).slice(0,i);a>0&&(c=e(t,i));let p=h?h.wordArr:[],w=p.length>0?p:c.map(t=>t?.struct?.id),y=(await r({word_ids:w})).body.data,_={};if(p.length>0){let t=[];for(let e=0;e<p.length;e++){const r=p[e];t.push(y.filter(t=>t.id==r)[0])}y=t}this.unorderedQuestions=s.cloneDeep(y),h||(y=y.sort(()=>Math.random()-.5)),y.forEach(t=>{_[t.id]=Math.round(Math.random())}),this.questionTypeM=h?h.questionTypeM:_,this.questions=y,this.words=c,p.length>0||1==d?o.replace({name:"wordQuestion",state:{data:JSON.stringify({w:t,unit_id:n,wordCount:i,type:d,step:u,whole:a,currentIndex:l,wordsData:h})},query:{type:d}}):o.replace({name:"wordPractice",state:{data:JSON.stringify({w:t,unit_id:n,wordCount:i,type:d,step:u,whole:a,currentIndex:l,wordsData:h})},query:{type:d}})},async startWordPracticeResource(t,r,d,u,a=0,l=0,h=0,c=null){this.currentIndex=h,this.type=u,this.currentStep=0==u?a:2,this.wholeWords=t,this.unit_id=r;let p=t.filter(t=>!t.grasp).slice(0,d);l>0&&(p=e(t,d));let w=c?c.wordArr:[];const{default:y}=await n(()=>import("./useWordCourseSource-9e541476.js"),["./useWordCourseSource-9e541476.js","./index-5543d811.js","./wordCourseSourceHelpers-f63ae392.js"],import.meta.url),_=y(),g=i(),{book_code:f="",unit_code:W=""}=g.sourceBook||{};let I=await _.fetchResourceWordStructs(p,{bookCode:f,unitCode:W});if(!I.length){const{showWordResourceError:t}=await n(()=>import("./useWordResourceError-06c28489.js"),["./useWordResourceError-06c28489.js","./index-5543d811.js"],import.meta.url);return void t()}if(w.length>0){let t=[];for(let e=0;e<w.length;e++){const r=w[e],s=I.filter(t=>t.id==r)[0];s&&t.push(s)}I=t}this.unorderedQuestions=s.cloneDeep(I),c||(I=I.sort(()=>Math.random()-.5));let m={};I.forEach(t=>{m[t.id]=Math.round(Math.random())}),this.questionTypeM=c?c.questionTypeM:m,this.questions=I,this.words=p;const C=w.length>0||1==u?"wordQuestion_resource":"wordPractice_resource";o.replace({name:C,state:{data:JSON.stringify({w:t,unit_id:r,wordCount:d,type:u,step:a,whole:l,currentIndex:h,wordsData:c})},query:{type:u}})}},persist:{afterRestore:async t=>{},paths:["questions","words","currentIndex","currentStep","startIndex","unit_id","type","wholeWords","defaultWordCount","showContent"]}});export{d as u};

```
要点：`type==1` → 路由 `wordQuestion*`（做题模式），否则 `wordPractice*`（练习模式）；`questionTypeM` 是 **per-word 随机 0/1**（`Math.round(Math.random())`），`unorderedQuestions` 是打乱前的原始顺序副本。

**wordPractice 本地 IndexedDB 表（userUnitId 维度断点续练）** — `chunks/wordPractice-8d79e17b.js`

```javascript
import{A as a}from"./index-5543d811.js";async function t(t){return await a.wordPractice.get(t)}async function r(r){return await t(r.userUnitId)?await a.wordPractice.where("userUnitId").equals(r.userUnitId).modify(r):await async function(t){return await a.wordPractice.add(t)}(r)}async function e(t){return await a.wordPractice.delete(t)}export{e as d,t as g,r as u};

```
### 来源 C：单元单词表（`g/set/word-list`）
**WordList 取词（并带资源平台 source:1 变体）** — `chunks/WordList-0c83bc9d.js`

```javascript
re=()=>{I({unit_id:ae.unit_id,parent_account_id:ee?.parent_id,resource_id:te.getResourceId()}).then(async e=>{0==e.code&&(se.value=e.body.data,await j(),window.triggerBuriedPoint("m_word_practice_word_list",{i_wordcount:oe.value-ie.value,i_bookId:ae.book_id,i_unitId:ae.unit_id}))})},
```
**AbilityExpand 单词PK 入口：取前 10 词 → pkMatch** — `chunks/AbilityExpand-b7ff6ed6.js`

```javascript
V=()=>{if(0==d.expand_unit_id)return void C.open({message:"请先选择单元"});const e=1===d.expand_source,{unit_code:a=""}=d.expand_sourceBook||{},t={unit_id:d.expand_unit_id,parent_account_id:c.userInfo?.parent_id,resource_id:c.userInfo?.resource_id},s=e?E(t,a):t;I(s).then(a=>{if(0==a.code&&a.body){let t=(a.body.data||[]).reduce((e,a)=>e=[...e,...a.words],[]);t=t.slice(0,10),t.length>0?w.push({name:e?"pkMatch_resource":"pkMatch",query:{title:d.expand_unit_name,...e?{from:"expand"}:{}},state:{data:JSON.stringify(t)}}):C.open({message:"该单元下无练习单词，请重新选择"})}})};
```
**WordList → startWordPractice / pkMatch** — `chunks/WordList-0c83bc9d.js`

```javascript
const _e=async(e,a,t)=>{r.logCustom("leave Word-List.vue push to lianxi.vue ",{tag:"UI_PAGE"}),pe.startWordPractice(ne.value,parseInt(ae.unit_id),e,a,0,t)},ge=()=>{r.logCustom("leave Word-List.vue push to lianxi1.vue ",{tag:"UI_PAGE"}),pe.startWordPractice(ne.value,parseInt(ae.unit_id),me?.count??0,me?.type??0,me?.currentStep,0,me?.currentIndex??0,JSON.parse(me?.words??""))},he=()=>{U.open({message:"重新练习将删除上一次练习保存<br>确定继续？",onConfirm:e=>{Q.value=!0;const a=t();J(`${a.userInfo?.id}-${ae.unit_id}`),e()}})};return n(()=>{r.logCustom("enter Word-List.vue ",{tag:"UI_PAGE"}),re()}),(e,a)=>{const t=W,s=S,l=x,o=$,i=A;return u(),d(i,{header:{title:g(ae).unit_name}},{default:c(()=>[v("div",K,[v("div",R,[p(t,{percentage:(oe.value-ie.value)/oe.value*100,class:"top-progress"},null,8,["percentage"]),v("div",X,[a[5]||(a[5]=v("div",{class:"top-container-master"},null,-1)),v("div",null,m(`已掌握单词${oe.value-ie.value}个`),1),a[6]||(a[6]=v("div",{class:"top-container-master-not"},null,-1)),v("div",null,m(`未掌握单词${ie.value}个`),1),a[7]||(a[7]=v("img",{class:"top-container-setting",src:G,width:"18",height:"18",alt:""},null,-1)),v("div",{class:"top-setting",onClick:a[0]||(a[0]=e=>M.value=!0)},"单词设置")]),p(s,{undeline:"",modelValue:L.value,"onUpdate:modelValue":a[1]||(a[1]=e=>L.value=e),onChange:ue,navs:[{title:"未掌握",value:"first"},{title:"全 部",value:"second"}]},null,8,["modelValue"])]),"first"==L.value&&0==le.value.length?(u(),d(_,{key:0,title:"太棒了，单元单词已经全部掌握",icon:g(N)},null,8,["icon"])):(u(),h("div",Z,
```
`g/set/word-list` 请求体：`{unit_id, parent_account_id, resource_id}`，BC 变体额外加 `{source:1, unit_code}`：
**buildWordListBCParams** — `index-5543d811.js`

```javascript
const BC_UNIT_CODE_FIELD="unit_code",buildWordListBCParams=(e,t)=>({...e,source:1,[BC_UNIT_CODE_FIELD]:t}),
```
响应 `body.data` = `[{ name, words:[ { id, struct:{content, material_id, translate, audio_url}, in_class, collect, grasp } ] }]`。

**生词本加入（`g/word/add-new`，带 source/item_id/unit_code）** — `chunks/WordPageItem-b11312ce.js`

```javascript
I=async()=>{await A(`[${j.material_id}]`,P.value?1:0),P.value=P.value?0:1},A=async(a,e)=>{const t=1===j.source&&j.item_id?{source:1,item_id:j.item_id,unit_code:j.unit_code}:{};let s=await f({material_id:a,parent_account_id:S.getUserInfo?.parent_id,add_new:e,...t});return 0==s.code&&(W("updataWordCont",s.body.count),e?y.open({message:"单词移出生词本",duration:2e3}):y.open({message:"单词已加入生词本",duration:2e3})),{add_new:e,res:s}},
```
**useWordCollect（移动端风格的精简版）** — `chunks/useWordCollect-3f40ea97.js`

```javascript
import{j as e,eX as a,ao as n}from"./index-5543d811.js";function t(){const t=e();return{requestWordAdd:async function(e,r,o){return 0==(await a({material_id:e,parent_account_id:t.getUserInfo?.parent_id,add_new:r,...o??{}})).code&&(r||n.open({message:"单词已加入生词本",duration:2e3})),r}}}export{t as u};

```
## 1.3 【核心】词汇题型的关键区分字段
### 判定字段 = `question_type`（不是 `type`、不是 `word_type`、不是 `sub_type`）
**唯一权威证据**：`chunks/wordCourseSourceHelpers-f63ae392.js` 里硬编码了两个数组，并被导出为公共符号：
**wordCourseSourceHelpers 全部导出与常量（原样全文）** — `chunks/wordCourseSourceHelpers-f63ae392.js`

```javascript
import{L as n}from"./index-5543d811.js";const e=[6,16,17,18,19,20],t=[6,19,20],o=100,r=19;function s(n,e=100){if(!Array.isArray(n)||0===n.length)return[];const t=[];for(let o=0;o<n.length;o+=e)t.push(n.slice(o,o+e));return t}function i(n){return e.includes(Number(n))}function d(n){return t.includes(Number(n))}function u(n,e){if(!d(n))return null;if("string"!=typeof e||""===e.trim())return null;try{const n=JSON.parse(e);return Array.isArray(n)?e:null}catch(t){return null}}function c(n){const e=n?.item_id??n?.struct?.item_id;return null==e?"":`${e}`.trim()}const a={itemCode:n=>n?.itemCode,questions:n=>n?.questions||[],resourceQuestionId:n=>n?.resourceQuestionId,questionTypeId:n=>n?.questionTypeId,option:n=>n?.details?.[0]?.option??n?.option??null,answer:n=>n?.details?.[0]?.answer??n?.answer??null,translate:n=>n?.details?.[0]?.translate??n?.translate??"",wordFindId:n=>n?.id??n?.itemId??n?.item_id??n?.itemCode,wordFindCode:n=>n?.code??n?.wordCode??n?.itemCode,wordFindReadAnswer:n=>{const e=n?.content?.groups||[];let t=null;for(const o of e)for(const n of o?.questions||[]){const e=n?.answer;if(e?.pprFile){if("read_word"===e?.type)return e;t||(t=e)}}return t},wordFindReadPpr:n=>`${a.wordFindReadAnswer(n)?.pprFile??""}`,wordFindReadCategory:n=>`${a.wordFindReadAnswer(n)?.type??""}`,wordFindReadQuestion:n=>{const e=n?.content?.groups||[];let t=null,o=null;for(const r of e)for(const n of r?.questions||[]){o||(o=n);const e=n?.answer;if(e?.pprFile){if("read_word"===e?.type)return n;t||(t=n)}}return t||o},wordFindPhonicsEntry:n=>{const e=a.wordFindReadQuestion(n),t=e?.ukSentences||[],o=e?.usSentences||[],r=t.length?t[0]:o.length?o[0]:null;return{text:`${r?.sentenceText?.value??""}`.trim(),audio:`${r?.sentenceAudio?.value??""}`.trim()}},wordFindBaseUrl:n=>`${n?.baseUrl??""}`,wordFindBaseSign:n=>`${n?.baseSign??""}`,wordFindExampleSentences:n=>{const e=n?.content?.groups?.[0]?.questions?.[0]?.exampleSentences||[];return e.length?e:a.wordFindReadQuestion(n)?.exampleSentences||[]},wordFindMeanSentences:n=>a.wordFindReadQuestion(n)?.meanSentences||[],wordFindWordAudio:n=>`${a.wordFindReadQuestion(n)?.questionAudio?.value??""}`};function l(n,e,t){const o=`${e??""}`.trim();return o?/^https?:\/\//.test(o)?o:`${n??""}${o}${t??""}`:""}function f(n){return null==n?"":`${n}`.replace(".ppr","")}function p(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t),o=f(a.wordFindReadPpr(t));null!=n&&""!=`${n}`&&o&&(e[`${n}`]=o)}return e}function w(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t),o=a.wordFindReadCategory(t);null!=n&&""!=`${n}`&&o&&(e[`${n}`]=o)}return e}function $(n){const e=`${n??""}`.trim();if(!e)return"";if(e.startsWith("[")&&e.endsWith("]"))return e;const t=e.replace(/^\/+/,"").replace(/\/+$/,"").trim();return t?`[${t}]`:""}function m(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t),o=$(a.wordFindPhonicsEntry(t).text);null!=n&&""!=`${n}`&&o&&(e[`${n}`]=o)}return e}function F(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t),{audio:o}=a.wordFindPhonicsEntry(t),r=l(a.wordFindBaseUrl(t),o,a.wordFindBaseSign(t));null!=n&&""!=`${n}`&&r&&(e[`${n}`]=r)}return e}function g(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t),o=l(a.wordFindBaseUrl(t),a.wordFindWordAudio(t),a.wordFindBaseSign(t));null!=n&&""!=`${n}`&&o&&(e[`${n}`]=o)}return e}function y(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t);if(null==n||""==`${n}`)continue;const o=[];for(const e of a.wordFindExampleSentences(t)){const n=`${e?.sentenceText?.value??""}`.trim(),t=`${e?.sentenceContent?.value??""}`.trim();""===n&&""===t||o.push(""===n?{en:t,cn:""}:{en:n,cn:t})}o.length&&(e[`${n}`]=JSON.stringify(o))}return e}function h(n){const e={};for(const t of n||[]){const n=a.wordFindCode(t);if(null==n||""==`${n}`)continue;const o=[];for(const e of a.wordFindMeanSentences(t)){const n=e?.sentenceText?.value,t=[(Array.isArray(n)?n.join(" "):`${n??""}`).trim(),`${e?.sentenceContent?.value??""}`.trim()].filter(Boolean).join(" ");t&&o.push(t)}o.length&&(e[`${n}`]=o.join("，"))}return e}function _(n,e){const t={},o=e||[],r=new Set((n||[]).map(n=>`${n}`));let s=!1;for(const i of o){const n=a.wordFindId(i),e=a.wordFindCode(i);null!=n&&r.has(`${n}`)&&e&&(t[`${n}`]=`${e}`,s=!0)}return s||o.length!==(n||[]).length||(n||[]).forEach((n,e)=>{const r=a.wordFindCode(o[e]);r&&(t[`${n}`]=`${r}`)}),t}function C(e){const t=a.questionTypeId(e);if(!i(t))return n.logCustom(`资源平台未知题型 questionTypeId=${t}，跳过该题`,{type:2}),null;const o=a.option(e);let r=null;return d(t)&&(r=u(t,o),null==r)?(n.logCustom(`资源平台选择题 option 解析失败 resourceQuestionId=${a.resourceQuestionId(e)} type=${t}，跳过该题`,{type:2}),null):{id:a.resourceQuestionId(e),question_type:Number(t),option:r??"",answer:a.answer(e)??"",translate:a.translate(e),content:"",audio_url:"",audio_size:0,phonics:"",stem:"",question_type_name:""}}function q(n,e,t=c,o={},r={},s={},i={},d={},u={},l={}){const f=new Map;for(const c of e||[]){const n=`${a.itemCode(c)}`,e=a.questions(c).map(C).filter(n=>null!=n).slice(0,6);e.length&&f.set(n,e)}const p=[];for(const c of n||[]){const n=`${t(c)}`,e=f.get(n);if(!e||!e.length)continue;const a=c.struct||{};p.push({id:a.id,material_id:a.material_id,content:a.content,audio_url:u[n]||l[n]||"",audio_size:a.audio_size,eg:i[n]||a.eg,engine_area:a.engine_area,engine_type:a.engine_type,image_url:a.image_url??"",mark_type:a.mark_type,phonics:s[n]||"",translate:d[n]||a.translate,resourceResId:o[n]??"",resourceCategory:r[n]??"",question:e})}return p}function S(n,e,t=19){const o=[],r=[],s=new Map;e.forEach(n=>s.set(n.id,n));for(const i of n||[]){const n=s.get(i.struct?.id);if(!n||!n.question?.length)continue;const e=n.question.find(n=>n.question_type===t)??n.question[0];e&&null!=e.id&&(o.push(`${c(i)}`),r.push(`${e.id}`))}return{item_ids:o,question_item_ids:r}}export{e as K,o as M,t as O,r as P,a as R,q as a,S as b,s as c,_ as d,p as e,w as f,c as g,m as h,F as i,g as j,y as k,h as l,C as m,i as n,d as o,u as p,l as q,f as r,$ as t};

```
从 `export{e as K,o as M,t as O,r as P,a as R,q as a,S as b,s as c,_ as d,p as e,w as f,c as g,m as h,F as i,g as j,y as k,h as l,C as m,i as n,d as o,u as p,l as q,f as r,$ as t}` 逐一对回：

| 局部变量 | 值 | 含义（按移动端已知映射 / 代码用法推定） |
|---|---|---|
| `e` | `[6,16,17,18,19,20]` | **词汇族全部 `question_type`**，函数 `i(n)=e.includes(Number(n))` 就是 `isWordQuestionType` |
| `t` | `[6,19,20]` | **选项型（有 `option`）子集**，函数 `d(n)=t.includes(Number(n))` 就是 `isChoiceType` |
| `o` | `100` | 单选题默认满分（`export ... o as M`，同时被 `useExamStepper` 当 `ModuleIDType` 无关系数用 → 见 §2.4） |
| `r` | `19` | **默认/首选题型 = 19（英译中）**，`S(n,e,t=19)` 的默认参数就是它 |
| `u(n,e)` | 解析 `option` | `JSON.parse` 后必须是数组，否则返回 `null` → 该题被判为非法并跳过 |
| `C(e)` | 造题面 | 写 `question_type:Number(t)`、`question_type_name:""`（PC 端留空，中文名在移动端） |

**isWordQuestionType / isChoiceType / option 解析** — `chunks/wordCourseSourceHelpers-f63ae392.js`

```javascript
function u(n,e){if(!d(n))return null;if("string"!=typeof e||""===e.trim())return null;try{const n=JSON.parse(e);return Array.isArray(n)?e:null}catch(t){return null}}
```
**资源平台题型 → 词表题面的构造（question_type 就写在这里）** — `chunks/wordCourseSourceHelpers-f63ae392.js`

```javascript
function C(e){const t=a.questionTypeId(e);if(!i(t))return n.logCustom(`资源平台未知题型 questionTypeId=${t}，跳过该题`,{type:2}),null;const o=a.option(e);let r=null;return d(t)&&(r=u(t,o),null==r)?(n.logCustom(`资源平台选择题 option 解析失败 resourceQuestionId=${a.resourceQuestionId(e)} type=${t}，跳过该题`,{type:2}),null):{id:a.resourceQuestionId(e),question_type:Number(t),option:r??"",answer:a.answer(e)??"",translate:a.translate(e),content:"",audio_url:"",audio_size:0,phonics:"",stem:"",question_type_name:""}}
```
**S()：优先挑 19，挑不到再退第一题** — `chunks/wordCourseSourceHelpers-f63ae392.js`

```javascript
function S(n,e,t=19){const o=[],r=[],s=new Map;e.forEach(n=>s.set(n.id,n));for(const i of n||[]){const n=s.get(i.struct?.id);if(!n||!n.question?.length)continue;const e=n.question.find(n=>n.question_type===t)??n.question[0];e&&null!=e.id&&(o.push(`${c(i)}`),r.push(`${e.id}`))}return{item_ids:o,question_item_ids:r}}
```
```text
function S(n,e,t=19){
  ... for(const i of n||[]){ const n=s.get(i.struct?.id);
      if(!n||!n.question?.length) continue;
      const e=n.question.find(n=>n.question_type===t) ?? n.question[0];   // <== 默认 t=19
      ... o.push(`${c(i)}`)  // item_ids
          r.push(`${e.id}`) // question_item_ids
  } return {item_ids:o, question_item_ids:r} }
```

→ `wordCourseSourceHelpers.S` 被导出为 `b`，用于 **BC 词汇作业跳转 PK**：`item_ids = word.item_id`、`question_item_ids = 该词 question_type===19 的题 id`。

### 直接消费 `question_type` 的两处运行时证据
**PK：从 6 道题里挑 19 当主问题** — `chunks/PK-7ae1b52b.js`

```javascript
ve=a(()=>{if(1==S.getInstance().practiceType){let e=j[ee.value];return me.value=e,e}{me.value=j[ee.value].question.filter(e=>19==e.question_type)[0];let e={};return e=j[ee.value],e.word_id=e.id,e.id=me.value.id,e}}),me=k(null),Ae=1===Number(O.query.word_source),De=U(m.wordInfo),Ee=(e,a)=>{const t={question_id:ve.value.id,word_id:ve.value.word_id,point:e,answer:a};if(De){const e=E(ve.value);e.item_id&&e.question_item_id&&(t.item_id=e.item_id,t.question_item_id=e.question_item_id)}else Ae&&(t.question_id="",t.item_id=z(ie[ee.value]),t.question_item_id=ve.value.id);return t},
```
```text
// PK-7ae1b52b.js
me.value = j[ee.value].question.filter(e=>19==e.question_type)[0];   // 19 = 英译中
let e={}; e = j[ee.value]; e.word_id = e.id; e.id = me.value.id;
```
→ 说明：**一个单词挂 6 道题（对应 6 个 question_type）**，PK/练习模式只用 `question_type===19`。

**PKMatch：要求 6 题齐全 + 取 19** — `chunks/PKMatch-d8a7c872.js`

```javascript
let e=[],a=[],t=[];if(J.word)e=J.word.map(e=>e.id),t={...J.word,question:[J.word_question]},a=J.word_question;else{e=Y.value.map(e=>e.struct.id);try{let s=await I({word_ids:e});if(0==s.code&&(t=s.body.data.filter(e=>e.question.length>=6),a=t.map(e=>e.question.filter(e=>19==e.question_type)[0])),0!=s.code||!a.length)return M.value=3,void o.logCustom("没有符合的单词列表",{type:2})}catch(s){return void(M.valu
```
```text
let s=await I({word_ids:e});                                  // I = requestWordStruct = g/word/struct
if(0==s.code&&(t=s.body.data.filter(e=>e.question.length>=6),  // 只保留「题目数 >= 6」的词
               a=t.map(e=>e.question.filter(e=>19==e.question_type)[0])),
   0!=s.code||!a.length) return M.value=3, void o.logCustom("没有符合的单词列表",{type:2});
```
→ **`question.length >= 6` 是「词表完整性」的官方门槛**，正好对应 `[6,16,17,18,19,20]` 六个题型。

### 各题型在 PC 端的呈现差异（19 vs 其它）
**题面/音标/音频开关：只有 19 用 content/phonics/audio** — `chunks/PK-7ae1b52b.js`

```javascript
l("div",Ke,[(t(!0),n(N,null,_(Me.value,(e,a)=>(t(),s(X,{onClick:t=>((e,a)=>{-1!=te.value||0==ea.value||$.value||ze.value||(clearInterval(aa),me.value?.answer==e.option_name?ne.value++:ne.value=0,te.value=a,Qe.value=Qe.value+(me.value?.answer==e.option_name?10:0),me.value?.answer!=e.option_name&&Ze(),ae.push(Ee(me.value?.answer==e.option_name?1e4:0,e.option_name)),q.logCustom(`PK.vue select-option-user  question_id=${ve.value.id}-word_id=${ve.value.word_id}-answer:${e.option_name}`,{tag:"FLOW_PRAC"}),Ge())})(e,a),class:"question-items-item",title:e.option_value,"show-status":(V(te)==a||je.value==e.option_name||me.value?.answer==e.option_name)&&!x.value,name:V(Q).opponent.name,answered:V(te)>-1||0==ea.value,"is-corrent":me.value?.answer==e.option_name,"me-select":V(te)==a&&!x.value,"opponent-select":je.value==e.option_name&&!x.value},null,8,["onClick","title","show-status","name","answered","is-corrent","me-select","opponent-select"]))),256))])]),ze.value?r("",!0):(t(),s(ue,{key:0,class:"tip","is-corrent":me.value?.answer==Me.value[V(te)]?.option_name,visible:!x.value&&(-1!=V(te)||0==ea.value||ze.value),"un-answer":0==ea.value,count:V(ne),canPlaySound:Xe.value,isHomeWork:la.value},null,8,["is-corrent","visible","un-answer","count","canPlaySound","isHomeWork"]))])]),_:1},8,["header"])}}}),[["__scopeId","data-v-577e3fb6"]]);export{De as default};

```
```text
// PK.vue 题面区（节选）
{content: () => [ o(19==me.value?.question_type ? ve.value.content : me.value?.translate, 1) ]}
... o(19==me.value?.question_type ? ve.value.phonics : " ")
... ve.value.audio_url && 19==me.value?.question_type ? (播放按钮) : 空
```
→ **19（英译中）**：题面 = 英文单词 `content`，带音标 `phonics`，带单词发音 `audio_url`；
→ **其它选择题（6 听音选义 / 20 中译英）**：题面 = `translate`（中文），不显示音标、不显示单词音频按钮。

### PC 端可确证 / 不可确证清单
| `question_type` | 移动端名 | PC 端可确证的内容 |
|---|---|---|
| 6  | 听音选义 | 属于选项型（`t=[6,19,20]` 含 6）；走 PK/练习同一套 `option` 解析；PC 无独立代码分支 |
| 16 | 默写词汇 | 只出现在 `e=[6,16,17,18,19,20]` 与 `question.length>=6` 门槛里；**PC 端无任何专门分支/中文名**（实现于缺失的 `WordQuestion*` chunk） |
| 17 | 朗读词汇 | 同上；PC 端无专门分支 |
| 18 | 听写词汇 | 同上；PC 端无专门分支 |
| 19 | 英译中 | **PC 端默认题型**（`S(...,t=19)`、`filter(e=>19==e.question_type)`）；题面/音标/音频规则唯一命中的题型 |
| 20 | 中译英 | 属于选项型（`t` 含 20）；题面走 `translate` 分支 |

> 判据：`i(t)=e.includes(Number(t))` 用于「是不是词汇题」；`d(t)=t.includes(Number(t))` 用于「是不是选项型（必须有合法 `option` JSON 数组）」。**非选项型的 16/17/18 走 `question_type` 分支后由缺失的 Word 练习页处理，`answer` 形态在本地可见代码中查不到** —— 见 §1.4 的「未找到」说明。

## 1.4 答案在哪里 / 取值形态
### (a) 选项型（6 / 19 / 20）：答案是 **`option_name`（选项键）** —— 既不是下标，也不是选项文本
`option` 字段是 **JSON 字符串**，解析后是对象数组 `[{option_name, option_value}]`：
**option 解析 & 打乱** — `chunks/PK-7ae1b52b.js`

```javascript
Me=a(()=>{let e=JSON.parse(me.value?.option??"[]");return 1!==S.getInstance().practiceType&&(e=P.shuffle(e)),e}),Ze=()=>{ie[ee.value].material_id&&(ie[ee.value].collect=1)},We=a(()=>{const e=ve.value.
```
**选项渲染 + 判分 + 记录** — `chunks/PK-7ae1b52b.js`

```javascript
l("div",Ke,[(t(!0),n(N,null,_(Me.value,(e,a)=>(t(),s(X,{onClick:t=>((e,a)=>{-1!=te.value||0==ea.value||$.value||ze.value||(clearInterval(aa),me.value?.answer==e.option_name?ne.value++:ne.value=0,te.value=a,Qe.value=Qe.value+(me.value?.answer==e.option_name?10:0),me.value?.answer!=e.option_name&&Ze(),ae.push(Ee(me.value?.answer==e.option_name?1e4:0,e.option_name)),q.logCustom(`PK.vue select-option-user  question_id=${ve.value.id}-word_id=${ve.value.word_id}-answer:${e.option_name}`,{tag:"FLOW_PRAC"}),Ge())})(e,a),class:"question-items-item",title:e.option_value,"show-status":(V(te)==a||je.value==e.option_name||me.value?.answer==e.option_name)&&!x.value,name:V(Q).opponent.name,answered:V(te)>-1||0==ea.value,"is-corrent":me.value?.answer==e.option_name,"me-select":V(te)==a&&!x.value,"opponent-select":je.value==e.option_name&&!x.value},null,8,["onClick","title","show-status","name","answered
```
```text
Me.value = JSON.parse(me.value?.option ?? "[]")                     // option 是 JSON 串，不是数组
  ... 1!==practiceType && (Me.value = P.shuffle(Me.value))            // PK 时打乱（练习模式不打乱）

select-option 点击：
  me.value?.answer == e.option_name ? ne.value++ : ne.value = 0       // 正确性判据
  Qe.value += (me.value?.answer == e.option_name ? 10 : 0)            // UI 计分 10/题
  ae.push(Ee(me.value?.answer == e.option_name ? 1e4 : 0, e.option_name))   // ★ 提交 point = 10000 / 0

模板：
  X({ title: e.option_value,        // ← 显示文本用 option_value
      "is-corrent": me.value?.answer == e.option_name,   // ← 正确性用 option_name
      answered / me-select / opponent-select ... })
```

**结论（选项型答案形态）：**
- `answer` = 一个 **`option_name`**（选项键），与 `options[i].option_name` 严格相等比较；
- 选项**文本**在 `option_value`，**只用于显示**；
- **下标 `i` 不参与判分**（`te.value=a` 仅用于 UI 高亮）；
- 提交时 `answer` 里装的也是 `option_name`（`Ee(point, e.option_name)`）。

> ⚠️ 注意 `option_name` 的具体取值字母表（`A/B/C/D` 或 `0/1/2/3`）**在本地 82 个 chunk 里找不到样例数据** —— 所有渲染/比较都只做 `== e.option_name`，不打印不枚举。带真题数据才能定，见「最可能不一致」第 1 条。

**PK 提交体的题目记录项（Ee）** — `chunks/PK-7ae1b52b.js`

```javascript
Ee=(e,a)=>{const t={question_id:ve.value.id,word_id:ve.value.word_id,point:e,answer:a};if(De){const e=E(ve.value);e.item_id&&e.question_item_id&&(t.item_id=e.item_id,t.question_item_id=e.question_item_id)}else Ae&&(t.question_id="",t.item_id=z(ie[ee.value]),t.question_item_id=ve.value.id);return t},Me=a(()=>{let e=JSON.parse(me.
```
```text
Ee=(e,a)=>{
  const t={ question_id: ve.value.id, word_id: ve.value.word_id, point:e, answer:a };
  if(De){                                   // De = isBcHomeworkStruct(state.wordInfo)
     const e = E(ve.value);                 // E = pickBcIds
     e.item_id && e.question_item_id && (t.item_id=e.item_id, t.question_item_id=e.question_item_id)
  } else Ae && (Ae = 1===Number(route.query.word_source))  // 资源平台词源
     && (t.question_id="", t.item_id=z(datas[i]), t.question_item_id=ve.value.id);  // z = itemCode
  return t
}
```
→ **拼写型/资源平台词源的题目标识三件套：`item_id` + `question_item_id`（`question_id` 置空字符串）**。

**超时未作答的记录：point=0, answer=""** — `chunks/PK-7ae1b52b.js`

```javascript
He=()=>{clearInterval(aa),clearTimeout(re.value),re.value=setTimeout(()=>{te.value=-1,ee.value>=j.length-1?(ze.value=!0,Je()):(ee.value++,ea.value=6,na())},1e3)},
```
**对手/历史作答的读取方式（证明 score_detail 是 JSON 串且含 choose）** — `chunks/PK-7ae1b52b.js`

```javascript
je=a(()=>1==S.getInstance().practiceType?0==Q.opponent.scores.length?"":JSON.parse(Q.opponent.scores[0].word.filter(e=>ve.value.word_id==e.word_id)[0].score_detail).choose:We.value.answer),Xe=k(!0),Qe=k(0),$e=k(0),ea=k(6);let aa=null;const ta=k(!1);B(Xe,e=>{Xe.value?p.resume():p.pause()});const na=()=>{clearInterval(aa),ea.value=6,aa=setInterval(()=>{ea.value--,0==ea.value&&(clearInterval(aa),ae.push(Ee(0,"")),q.logC
```
```text
// 练习模式：从对手/历史分数里读我的旧答案
je = () => 1==practiceType
  ? (0==Q.opponent.scores.length ? ""
     : JSON.parse(Q.opponent.scores[0].word.filter(e=>ve.value.word_id==e.word_id)[0].score_detail).choose)
  : We.value.answer;
```
→ **`score_detail` 是 JSON 字符串，里面 `choose` 字段就是作答内容**；历史成绩的 `word[]` 里同时有 `word_id` 与 `score_rate`。

**PK 计分 UI 侧（10 分/题，最后换算百分比）** — `chunks/PK-7ae1b52b.js`

```javascript
Je=async()=>{q.logCustom(`PK.vue submit  currentTime=${(new Date).getTime()}`,{tag:"FLOW_PRAC"}),S.getInstance().endCount();const e=(new Date).getTime()-ce;de.value=Math.floor(e/1e3);try{let e=JSON.stringify(ae);q.logCustom(`PK.vue submit  currentTime=${(new Date).getTime()}-answers=${e}`,{tag:"FLOW_PRAC"})}catch(t){}let a=await S.getInstance().requestWordHomeworkSubmit({serial_id:Q.serial_id,use_time:Math.floor(e/1e3),score_detail:ae,resource_id:m.resource_id,type:6,...m.wordInfo?{wordInfo:m.wordInfo}:{}});if(0==a.code)return function(e){let a=Math.ceil(Qe.value/(10*ie.length)*100);S.getInstance().handleResultScoreAction(a,{failConfirmButtonText:"查看PK详情",showModalSuccess:!1,multipleShowModal:!0,isPkModal:!0,fail_callback:()=>{Ye()},pass_callback:({pass:a})=>{if(!a)return!1;e.body.add_bean>0&&(ta.value=!0),$.value=!0},close_callback:()=>{T.back()}})}(a),void(le.value=!0);De&&H(a.code,a.msg)};
```
```text
// PK 结算
let a = await S.getInstance().requestWordHomeworkSubmit({
  serial_id: Q.serial_id, use_time: Math.floor(e/1e3), score_detail: ae,
  resource_id: m.resource_id, type: 6, ...(m.wordInfo?{wordInfo:m.wordInfo}:{})
});
if(0==a.code) return (function(e){
  let a = Math.ceil(Qe.value/(10*ie.length)*100);   // ← 本地百分比 = ceil(得分/(10*题数)*100)
  S.getInstance().handleResultScoreAction(a, {...isPkModal:!0...})
})(a)
```
→ **提交用 `type:6`**（注意这里的 `6` 是 PK 场景常量，不是 `first_column_id`，也不是 `question_type`）。

### (b) 旧套题「选择题族」（`collector.choose`，**非词汇族**）：答案 = 选项**文本** + 选项**序号**
这条要特别注意，**和词汇族规则相反**，不要互相套用：
**collector.choose 的 XML 选项解析与判分** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
async function G(e,t,a,o){const{getFileJson:n}=f(),{UNZIP_LOCAL_PATH:l}=d();let s=[],r=(await n(p(`${l}\\${e}\\content_${t}\\content.json`))).info.xtlist;try{s=o.split(",")}catch(i){}return s.map(e=>{const[t,o,n]=e.split("_");let l="0";if("0"===n)return{entity_id:t,order:o,score_detail:{total_score:l,choose:"",category:a.evaluation_type}};{const e=r.find(e=>e.xt_xh===o);if(!e)return null;const s=e.xxlist.find(e=>e.xx_xh==n);return s?(e.answer===s.xx_mc&&(l=m(a.question_score/a.question_count)),{entity_id:t,order:o,score_detail:{total_score:l,choose:s.xx_mc,choose_id:n,category:a.evaluation_type}}):null}}).filter(Boolean)}
```
```text
// xxlist 来自 content.json：{ xx_xh: 选项序号, xx_mc: 选项文本 }
const s = e.xxlist.find(e=>e.xx_xh==n);
return s ? (e.answer === s.xx_mc && (l = m(a.question_score/a.question_count)),   // ★ 用选项文本比
            { entity_id:t, order:o,
              score_detail:{ total_score:l, choose:s.xx_mc, choose_id:n, category:a.evaluation_type } })
         : null;
// m = scoreFormater
```

```text
// useExamResult-d9fb4671.js
function b(e){                                  // 字母 → 数字串：A→1, B→2, ... AB→"12"
  if(!e) return "0";
  for(var t=e.toUpperCase(), o="", i=0; i<t.length; i++){
    var r=t.charCodeAt(i); if(r>=65&&r<=90) o+=(r-64).toString(); else o+=t[i];
  } return o;
}
// collector.choose 回填：
`${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}`   // i = b(xtlist 正确 answer)
```
→ 旧族：`detail.choose` 与 `xtlist[].answer` 都是**选项字母**，下发到 iframe 时 `A→1`；无作答时下发 `${entity_id}_${order}\x01${答案}`。

### (c) 拼写型（16 默写 / 18 听写）：**未找到**
已查关键词（全部文件、含主 bundle）：`question_type==16`、`question_type==18`、`默写`、`听写`、`拼写`、`spell`、`dictation`、`answer`、`stem`、`word_blank`。
结果：**本地可见代码中没有 16/18 的专门分支**。可确定的只有：
- 16/18 属于词汇族（在 `[6,16,17,18,19,20]` 中）；
- **不属于选项型**（不在 `[6,19,20]` 中 → `isChoiceType()===false` → `q()` 不会要求 `option` 是合法 JSON 数组，`option` 被写成 `""`）；
- 题面对象里预留了 `answer`（字符串）、`content`、`stem` 字段（见 `C(e)` 的返回对象）。
→ **因此「拼写型答案是纯字符串还是别的」在本批材料里无代码证据**；实现于缺失的 `WordQuestion-*.js` / `HomeWorkWordQuestion-*.js`。建议由父代理补抓这 2 个 chunk 后再定稿。

### (d) 朗读型（17 朗读词汇）：**未找到专门分支**
可确定的间接证据：
- 17 不在选项型集合，也不是拼写类；
- 录音类题目在 PC 端的作答协议走「录音 → 评测引擎 → `score_detail`」，`score_detail.category` 承载题型（`EvaluationTypes`，见下）；
- **朗读型没有「正确答案」，只有评测分**：`useRecordResult` 的 `score_detail` 里没有 `choose`（`choose` 只由选择题写），只有 `total_score/real_score/accuracy/fluency/standard/integrity`。

**EvaluationTypes（朗读/评测类 category 白名单，决定 real_score 是否按 5 分制换算）** — `index-5543d811.js`

```javascript
const EvaluationTypes=["read_word","read_chapter","read_sentence","read_choice","simple_expression_ufi","simple_expression_ufk","simple_expression_ufj","topic","simple_expression","free_expression"];var SchoolLevel=(e=>(e[e.Primary=2]="Primary",e[e.Middle=1]="Middle",e[e.High=3]="High",e))(SchoolLevel||{}),EvalResultSummaryTotal=(e=>(e.De
```

## 1.5 分值规则 / 提交时的题目标识字段
### 词表题目标识字段（汇总，均有代码出处）
| 字段 | 出现位置 | 含义 |
|---|---|---|
| `word_id` | `requestHomeworkWordCommit` 的 `detail[]`、PK 的 `Ee()` | 单词 id（`g/word/struct` 的 `struct.id`；作业族来自 `word_question.id`/`word.id`） |
| `question_id` | 同上 | 题目 id（`question[].id`）；资源平台词源时**被置为 `""`** |
| `item_id` | `pickBcIds` / `buildRpQuestionMap` | 资源平台词条 id（BC 词汇作业） |
| `question_item_id` | 同上 | 资源平台题目 id（BC 词汇作业） |
| `material_id` | `g/word/struct` 元素、`g/word/add-new` | 词条素材 id |
| `stid` | `collector.word` 的 `info.stid`、`/m/audio/sync-v2` 的 `entity_id` | 套题里的「实体(=exam_id)」 |
| `entity_id` | `/m/audio/sync-v2`、`collector.*` 全部族 | 大题/实体 id（= `exam.exam_id`） |
| `order` | `/m/audio/sync-v2` | 小题序号（`question_order` / `xh`） |
| `score_rate` | `g/homework/score-sync` 的 `detail[]` | **0/1 比率**（`point>0?1:0`），不是分数 |
| `seq_id` | 同上 | `md5(raw_phone + homework_id + new Date().toISOString(), 16)` |

**★ 词汇作业最终提交体构造（全文，§2.3 详析）** — `index-5543d811.js`

```javascript
requestHomeworkWordCommit:e=>{const t=useUserInfoStore$1().userInfo,n=a.value??"";try{let t=JSON.stringify(e);LoggerUtils.logCustom(`submit-score currentTime=${(new Date).getTime()}-params=${t}`)}catch(s){}const r=e.id||e.homework_id;if(r&&String(r)!==String(n))return LoggerUtils.logCustom(`submit-score blocked: params.id=${r} != store.homework_id=${n}，数据不属于当前作业，阻止提交`),Promise.resolve({code:-1,msg:"homework_id mismatch, submission blocked"});const o=isBcHomeworkStruct(e.wordInfo)?`${e.wordInfo?.unit_code??""}`:"",i={detail:0==e.pk?e.detail.map(e=>({...e,client_time:currentTimestampToDate()})):e.score_detail.map(e=>({word_id:e.word_id,question_id:e.question_id,...e.item_id&&e.question_item_id?{item_id:e.item_id,question_item_id:e.question_item_id}:{},score_rate:e.point>0?1:0,score_detail:JSON.stringify({choose:e.answer,score:e.point>0?100:0}),client_time:currentTimestampToDate()})),homework_id:parseInt(n),resource_id:e.resource_id,type:e.type,use_time:e.use_time,mark_service:1,seq_id:e.seq_id?e.seq_id:md5((t?.raw_phone??"")+n+new Date(Date.now()).toISOString(),16)};return requestHomeworkScoreSync(buildWordHomeworkScoreSyncBCParams(i,o))
```
### 分值字段
| 字段 | 来源 | 说明 |
|---|---|---|
| `question_type_score` | 选择题：`step.exam.exam_score`；录音题：`question.question_score` | 该题（或该大题）满分 |
| `total_score` / `total_score_init` | 评测引擎结果（`useRecordResult` 的 `I`） | 5 分制约分；`init` 是未取整原始值 |
| `real_score` | `calculateRealScore(question_type_score, total_score)` = `scoreFormater(qts/5*total)` | 我方可得分 |
| `rate_scale` | `useUserInfoStore().ecardInfo.rate_scale`（来自 `m/ecard/get`） | 电子卡倍率 |
| `graduation` | 计算得出，`c&&u ? "0.125" : undefined` | 125 细粒度标记 |
| `point` | PK/练习内部作答得分（正确 `10000`；PK UI 另算 `10/题`） | 仅用于换算 `score_rate` |

**单题满分取值（选择题 vs 录音题）** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
"action.record":async t=>{ue.value=!0;let a=Number(t.playtime??0);"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&(a=1e4,h?.pause());const o=B.cloneDeep(t);if(o&&o.question){const{pprInfo:t}=o.question||{},{engine_type:n}=t||{},l={record_time:a,category:o.question.evaluation_type,res_id:o.question.res_id,region:o.question?.engine_area,recordStep:o,set_id:e,engine_type:n,pprInfo:t,first_column_id:g};if(ne.value)return;ne.value=!0;const s=Z.record(l,{onReady:()=>{oe.value=!0,le.value=!1,ne.value=!1,"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&h?.play()},onReadyLoading:()=>{le.value=!0}}),r=()=>{const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};return new Promise((t,a)=>{s.then(async a=>{L.trigger(async()=>{const t=await re(l.category,a,g,o.question.question_score,e=>e&&se.addTask(e));return x.trigger({...t,...e}),t}),t(a)}).catch(t=>{a({...t,logParams:e,recordStep:o})})})};F.trigger(r)}},
```
```text
// action.record 步骤：
question_type_score: o.question.question_score        // ← 录音题：小题满分
...
const e={ entity_id:o.exam.exam_id, order:o.question.question_order,
          first_column_id:g, question_type_score:o.question.question_score };

// choose / choose-check / practise 步骤（同文件 practise 与 r()）：
a.forEach(e=>{ e.first_column_id=g; e.question_type_score = _e.exam.exam_score })   // ← 选择题：大题满分
```

**exam_type 满分来自本地 res.json 的 set_score 累加** — `index-5543d811.js`

```javascript
async processPaperResources(){let e=[];this.totalScore=0;for(let n=0;n<this.paperList.length;n++){const t=this.paperList[n],r=await this.getJson(replacePathSeparator(`${this.getPackageId()}\\template_${t.template.id}\\res.json`));this.totalScore+=Number(r.set_score);const o=lodashExports.cloneDeep(r),i=await Promise.all(o.exam_type_list.map(async e=>{let n=0;return e.exam_list=await Promise.all(e.exam_list.map(async r=>{const o=t.contents.find(e=>`{{${e.code_id}}}`===r.exam_id);if(o&&(r={...r,exam_id:replaceRealId(r.exam_id,o.real_id),...o}),"collector.repeat"===e.exam_type_collect){let e=function(e,t){const n=(e/t).toFixed(2),r=(e-(t-1)*n).toFixed(2);let o=[];for(let i=0;i<t-1;i++)o.push(n)
```

---

# 第二部分：考试流程 / 结果 / 提交

## 2.1 答题推进流程：进入一题 → 播放 → 录音 → 提交该题 → 下一题
### 核心：`useExamStepper-8ad3fbd7.js`（导出 `u` = `useExamSteps`，导出 `_` = `IFRAME_LOCAL_PATH`）
调用签名 `me(set_id, steps, arg3, examItem, arg5)`：`steps` 是 `ProcessPaper.processPaperSteps()` 的结果数组，`examItem`（第 4 参，代码里叫 `a`/`l`）提供 `first_column_id`/`super_set_id`/`type`/`redoIndex`/`choice_order`。
**步进器初始化与 first_column_id 推导** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
K={};t&&t.length&&t.forEach(e=>{K[e.order]=e});
```
```text
K={}; t&&t.length&&t.forEach(e=>{K[e.order]=e});      // key = "${order_num}_custom_order_${subid}_${order_num}"
const Q = I(K)                                              // I = useStepper(Object) → Object.keys 插入序
const{first_column_id:r="",super_set_id:i="",redoIndex:c,choice_order:v}=l||{},
      g = l?.type || r || ""                                // ★ first_column_id 兜底链：type || first_column_id || ""
```

### 步骤派发表：`filetype` → handler（`He()` = 进入当前步骤；`De()` = 离开当前步骤）
**步骤进入/离开分发表 Oe / We / Ee + He / De** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
Oe={"view.html":e=>{de(),_e=e,ge.trigger({type:e.filetype,key:Date.now(),displayAnswerContent:he.value,htmlData:B.cloneDeep(e.model_pc.extractedJsParam),htmlUrl:`${Y}/${V.PcListen}`}),b.once(()=>{N.logCustom("渲染视图完成触发nextStep",{tag:"FLOW_PRAC"}),pe(),setTimeout(()=>{Ce()},100)})},wait:Fe,"file.audio":e=>{const t=p(`${z}\\${e.filename}`);"pause"===W.playStatus.value?W.resume():W.play(t)},"file.video":e=>{const{fileparams:t}=e;if("1"==t){const t=p(`${z}\\${e.filename}`);"pause"===W.playStatus.value?W.resume():W.play(t)}else h&&h.$el.currentTime?h.play():ge.trigger({type:e.filetype,data:e,key:Date.now(),src:p(`${z}\\${e.filename}`)})},pause:()=>{N.logCustom("pause触发nextStep",{tag:"FLOW_PRAC"}),ee.value&&Ce()},practise:async()=>{clearTimeout(me.value),me.value=setTimeout(async()=>{if(y){if("collector.choose"===w.value.exam_type_collect){y.ReturnChoose=e=>{N.logCustom(`操作选择的answers:${JSON.stringify(e)}`,{tag:"FLOW_PRAC"})},await _();const t=y.getChoice(),a=await G(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_choose"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}}"collector.fill"===w.value.exam_type_collect&&(y.returnPcBlank=async t=>{const a=await async function(t,a,o,n){const{getFileJson:l}=f(),{UNZIP_LOCAL_PATH:s}=d(),r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std,i=JSON.parse(n);try{return i.map(e=>{const[t,a]=e.id.split("_"),n=e.answer;let l="0";if(""===n)return{entity_id:t,order:a,score_detail:{total_score:l,choose:"",category:o.evaluation_type}};{const e=r.find(e=>e.xth===a);return s=X(n),X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),{entity_id:t,order:a,score_detail:{total_score:l,choose:n,category:o.evaluation_type}}}var s})}catch(e){return[]}}(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_fill"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:U.getInstance().getIsHomework()?1:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}},y.getPcBlank())}N.logCustom("practiseHandler触发nextStep",{tag:"FLOW_PRAC"}),Ce()},50)},"choose-check":Fe,changetext:e=>{if(y){const t=y[e.filename];if("function"==typeof t){const a=e.fileparams.slice(1,-1);t.call(null,a)}}N.logCustom("changetextHandler触发nextStep",{tag:"FLOW_PRAC"}),Ce()},"action.record":async t=>{ue.value=!0;let a=Number(t.playtime??0);"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&(a=1e4,h?.pause());const o=B.cloneDeep(t);if(o&&o.question){const{pprInfo:t}=o.question||{},{engine_type:n}=t||{},l={record_time:a,category:o.question.evaluation_type,res_id:o.question.res_id,region:o.question?.engine_area,recordStep:o,set_id:e,engine_type:n,pprInfo:t,first_column_id:g};if(ne.value)return;ne.value=!0;const s=Z.record(l,{onReady:()=>{oe.value=!0,le.value=!1,ne.value=!1,"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&h?.play()},onReadyLoading:()=>{le.value=!0}}),r=()=>{const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};return new Promise((t,a)=>{s.then(async a=>{L.trigger(async()=>{const t=await re(l.category,a,g,o.question.question_score,e=>e&&se.addTask(e));return x.trigger({...t,...e}),t}),t(a)}).catch(t=>{a({...t,logParams:e,recordStep:o})})})};F.trigger(r)}},script:e=>{if(y)try{const a=e.filename;if(a.includes("viewpng")){var t=a.match(/viewpng\('([^']+)'\)/);y.setBackgroundColor(t[1])}else a.includes("hidepng")}catch(a){}N.logCustom("scriptHandler触发nextStep",{tag:"FLOW_PRAC"}),Ce()}},
```
```text
Oe = {                          // 进入某一步
 "view.html":   e => { 渲染 iframe（htmlUrl = IFRAME_LOCAL_PATH/PcListen）；
                        once 渲染完成 → setTimeout(Ce,100)  // Ce = nextStep
 },
 wait:          Fe,             // 倒计时/等待步骤（wait/choose-check 共用）
 "file.audio":  e => 播放本地音频（W = useSound）,
 "file.video":  e => synchro=="1" ? 播放 : 挂到 <video> 上播,
 pause:         () => { isPlaying && Ce() },     // 暂停步骤 = 直接下一步
 practise:      async () => {...},               // 大题的「做题/提交」入口
 "choose-check": Fe,
 changetext:    e => { y[e.filename](...); Ce() },
 "action.record": async t => {...},              // ★ 录音 + 评测 + 逐题上报
 script:        e => {...; Ce()},
},
We = { wait/b":$e, "choose-check":$e, "file.audio": stop, "action.record": stopRecord, "file.video": stop },
Ee = { wait/choose-check: pause, "file.audio": pause, "file.video": pause, "action.record": () => isRecording=false },
De = e => { const t=Q.current.value?.filetype; (We[t]||e=>e())(e) },
He = async () => { if(ie.value) return; const{filetype:e,synchro:t}=Q.current.value??{};
                   const a=Oe[e]; "function"==typeof a && a(Q.current.value);
                   if("file.video"==e && "1"==t){ await xe(); He() } },
```

### 「下一步」：`Ce()` / `xe()`
**Ce（当前步完成）/ xe（真正推进）** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
He=async()=>{if(ie.value)return;const{filetype:e,synchro:t}=Q.current.value??{},a=Oe[e];"function"==typeof a&&a(Q.current.value),"file.video"==e&&"1"==t&&(await xe(),He())},Ie=()=>{N.logCustom("套题流程触发pause暂停",{tag:"FLOW_PRAC"}),ee.value=!1,(()=>{const e=Q.current.value?.filetype,t=Ee[e];if("function"==typeof t&&t(Q?.current.value),"1"==Se.value?.synchro){const e=Ee[Se.value.filetype];e&&e(Se.value)}})()},Ne=()=>{N.logCustom(`套题流程触发stopRecord停止录音, 当前步骤order: ${Q?.current?.value?.order}`,{tag:"FLOW_PRAC"}),ke()};const Be=o(()=>ue.value||"action.record"==fe.value?.filetype),Je=o(()=>ue.value||"action.record"==fe.value?.filetype);return n(Q.current,t=>{t&&(N.logCustom({title:"套题流程: ",set_id:e,entity_id:t.exam?.exam_id,order:t.order,filename:t.filename,playtime:t.playtime,filetype:t.filetype,log_name:"exam_step"},{tag:"FLOW_PRAC"}),R.trigger(t),h&&(h.$el.currentTime=0))},{immediate:!0}),{initView:async(e=!0)=>{await ce(),ee.value=e,ae.value=e,He()},updateView:ge.on,answerHandler:x.on,chooseCheckHandler:P.on,chooseHandler:S.on,completeHandler:q.on,htmlLoad:O.trigger,prev:async(e=!1)=>{if(await new Promise(e=>setTimeout(e,200)),N.logCustom(`prev触发prevStep, prevDisabled: ${Je.value}`,{tag:"FLOW_PRAC"}),!Je.value)if(ee.value=!0,e)we();else{const e=t.findIndex(e=>e.order===fe.value?.order),a=function(e,t){for(let a=t-1;a>=0;a--)if(e[a].subid!==e[t].subid&&"view.html"===e[a].filetype)return a;return 0}(t,e),o=t[a].order;we(o)}},next:async()=>{await new Promise(e=>setTimeout(e,200)),N.lo
```
```text
const Ce = async () => {            // = "nextStep"
  if(ie.value) return;                              // ie = 已完成(最后一步已过)
  ue.value=!0;                                      // 步骤切换中
  "choose-check"==fe.value?.filetype && P.trigger(!0);   // 检查选择题答案
  De(async () => { await xe(); ee.value && He(); M(()=>{ue.value=!1},200) });
};
async function xe(){                // 真正的 index++
  try{ if(await ce(), ie.value) return;      // ce() = 等待网络恢复
       if(Q.isLast.value) return ie.value=!0, void q.trigger(!0);   // q = completeHandler
       Q?.goToNext();                        // I = useStepper → goToNext()
       "pause"===fe.value?.filetype && (ee.value=!1);
  }catch(e){}
}
```

### 录音步骤：`action.record`（播放 → 录音 → 评测 → 上报 → 下一步）
**action.record 全文（含 record/evaluate/upload 三段任务队列）** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
"action.record":async t=>{ue.value=!0;let a=Number(t.playtime??0);"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&(a=1e4,h?.pause());const o=B.cloneDeep(t);if(o&&o.question){const{pprInfo:t}=o.question||{},{engine_type:n}=t||{},l={record_time:a,category:o.question.evaluation_type,res_id:o.question.res_id,region:o.question?.engine_area,recordStep:o,set_id:e,engine_type:n,pprInfo:t,first_column_id:g};if(ne.value)return;ne.value=!0;const s=Z.record(l,{onReady:()=>{oe.value=!0,le.value=!1,ne.value=!1,"1"==Se.value?.synchro&&"file.video"===Se.value?.filetype&&h?.play()},onReadyLoading:()=>{le.value=!0}}),r=()=>{const e={entity_id:o.exam.exam_id,order:o.question.question_order,first_column_id:g,question_type_score:o.question.question_score};return new Promise((t,a)=>{s.then(async a=>{L.trigger(async()=>{const t=await re(l.category,a,g,o.question.question_score,e=>e&&se.addTask(e));return x.trigger({...t,...e}),t}),t(a)}).catch(t=>{a({...t,logParams:e,recordStep:o})})})};F.trigger(r)}},
```
流程逐句拆解：
1. **进入**：`ue=true`（禁止下一步）；若「下一步」是 `synchro=="1"` 的 `file.video`，把录音时长强制为 `1e4ms` 并暂停视频。
2. **组 record 参数**：`{record_time, category, res_id, region, recordStep, set_id, engine_type, pprInfo, first_column_id}`（`pprInfo = o.question.pprInfo` 来自 `mergePPRes` 注入的逐题 PPR 资源）。
3. **开始录音**：`Z.record(l, {onReady, onReadyLoading})`；`onReady` 里再判断是否 `synchro` 播放视频。
4. **评测 + 逐题上报**：`L.trigger(async()=>{ const t = await re(l.category, a, g, o.question.question_score, e=>e&&se.addTask(e)); x.trigger({...t, ...e}); })`
   - `re` = `useRecordResult().getResult`
   - `x` = `answerHandler`（**这一 trigger 就是「答完一题就上报」的起点**）
   - 第 5 个参数是「上传任务注册回调」，把需要在提交前完成的 OSS 上传塞进 `realUploadFileTasks` 队列。
5. **离开**：`We["action.record"] = ke` 停录音；`Ee["action.record"]` 把 `isRecording=false`；然后 `xe()` 前进。

**录音结束回调（onRecordEnd → 下一步）** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
Oe={"view.html":e=>{de(),_e=e,ge.trigger({type:e.filetype,key:Date.now(),displayAnswerContent:he.value,htmlData:B.cloneDeep(e.model_pc.extractedJsParam),htmlUrl:`${Y}/${V.PcListen}`}),b.once(()=>{N.logCustom("渲染视图完成触发nextStep",{tag:"FLOW_PRAC"}),pe(),setTimeout(()=>{Ce()},100)})},wait:Fe,"file.audio":e=>{const t=p(`${z}\\${e.filename}`);"pause"===W.playStatus.value?W.resume():W.play(t)},"file.video":e=>{const{fileparams:t}=e;if("1"==t){const t=p(`${z}\\${e.filename}`);"pause"===W.playStatus.value?W.resume():W.play(t)}else h&&h.$el.currentTime?h.play():ge.trigger({type:e.filetype,data:e,key:Date.now(),src:p(`${z}\\${e.filename}`)})},pause:()=>{N.logCustom("pause触发nextStep",{tag:"FLOW_PRAC"}),ee.value&&Ce()},practise:async()=>{clearTimeout(me.value),me.value=setTimeout(async()=>{if(y){if("collector.choose"===w.value.exam_type_collect){y.ReturnChoose=e=>{N.logCustom(`操作选择的answers:${JSON.str
```
```text
Z.onRecordEnd(async e=>{
  N.logCustom(`录音器onRecordEnd被触发，当前状态: isPlaying=${ee.value}, isRecording=${oe.value}`,
              {tag:"FLOW_PRAC"}),
  T.trigger({...e, first_column_id:g}),        // T = afterRecorderHandler（页面侧收集，用于超时重评）
  ee.value && (await xe(), He(), ue.value=!1, oe.value=!1, N.logCustom("录音完毕，跳下一步骤",{tag:"FLOW_PRAC"}))
});
```

### 选择题步骤（旧族）：`practise` + `choose-check`
**practise handler（collector.choose / collector.fill）** — `chunks/useExamStepper-8ad3fbd7.js`

```javascript
practise:async()=>{clearTimeout(me.value),me.value=setTimeout(async()=>{if(y){if("collector.choose"===w.value.exam_type_collect){y.ReturnChoose=e=>{N.logCustom(`操作选择的answers:${JSON.stringify(e)}`,{tag:"FLOW_PRAC"})},await _();const t=y.getChoice(),a=await G(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_choose"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}}"collector.fill"===w.value.exam_type_collect&&(y.returnPcBlank=async t=>{const a=await async function(t,a,o,n){const{getFileJson:l}=f(),{UNZIP_LOCAL_PATH:s}=d(),r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std,i=JSON.parse(n);try{return i.map(e=>{const[t,a]=e.id.split("_"),n=e.answer;let l="0";if(""===n)return{entity_id:t,order:a,score_detail:{total_score:l,choose:"",category:o.evaluation_type}};{const e=r.find(e=>e.xth===a);return s=X(n),X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),{entity_id:t,order:a,score_detail:{total_score:l,choose:n,category:o.evaluation_type}}}var s})}catch(e){return[]}}(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),N.logCustom({answers:t,result:a,log_name:"user_fill"},{tag:"FLOW_PRAC"});try{window.triggerBuriedPoint("m_listening_specialization",{i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),i_type:U.getInstance().getIsHomework()?1:0,i_totalScore:m(_e.exam.question_list[0].question_score)})}catch(e){}},y.getPcBlank())}N.logCustom("practiseHandler触发nextStep",{tag:"FLOW_PRAC"}),Ce()},50)},
```
```text
practise: async () => { setTimeout(async () => {
  if(y){
    if("collector.choose"===w.value.exam_type_collect){
      y.ReturnChoose = e => log(...);  await _();             // 等 iframe 回传
      const t = y.getChoice();
      const a = await G(w.value.super_set_id||w.value.set_id, _e.exam.exam_id, _e.exam.question_list[0], t);
      a.forEach(e=>{ e.first_column_id=g; e.question_type_score=_e.exam.exam_score; x.trigger(e) });  // ★ 逐题上报
      try{ window.triggerBuriedPoint("m_listening_specialization",{ i_score:a.reduce((e,t)=>e+m(t.score_detail.total_score),0),
           i_type:0, i_totalScore:m(_e.exam.question_list[0].question_score) }) }catch(e){}
    }
    if("collector.fill"===w.value.exam_type_collect){ y.returnPcBlank = async t => {...} ; y.getPcBlank() }
  }
  N.logCustom("practiseHandler触发nextStep",{tag:"FLOW_PRAC"}), Ce();
},50)},
```

### 步进顺序与步骤集合
**useStepper（Object.keys 顺序即步骤顺序）** — `index-5543d811.js`

```javascript
function useStepper(e,t){const n=ref(e),r=computed(()=>Array.isArray(n.value)?n.value:Object.keys(n.value)),o=ref(r.value.indexOf(null!=t?t:r.value[0])),i=computed(()=>u(o.value)),a=computed(()=>0===o.value),s=computed(()=>o.value===r.value.length-1),l=computed(()=>r.value[o.value+1]),c=computed(()=>r.value[o.value-1]);function u(e){return Array.isArray(n.value)?n.value[e]:n.value[r.value[e]]}function d(e){r.value.includes(e)&&(o.value=r.value.indexOf(e))}function p(e){return o.value>r.value.indexOf(e)}return{steps:n,stepNames:r,index:o,current:i,next:l,previous:c,isFirst:a,isLast:s,at:u,get:function(e){if(r.value.includes(e))return u(r.value.indexOf(e))},goTo:d,goToNext:function(){s.value||o.value++},goToPrevious:function(){a.value||o.value--},goBackTo:function(e){p(e)&&d(e)},isNext:function(e){return r.value.indexOf(e)===o.value+1},isPrevious:function(e){return r.value.indexOf(e)===o.value-1},isCurrent:function(e){return r.value.indexOf(e)===o.value},isBefore:function(e){return o.value<r.value.indexOf(e)},isAfter:p}}
```
**ProcessPaper.processPaperSteps（生成 order = "${order_num}_custom_order_${subid}_${order_num}"）** — `index-5543d811.js`

```javascript
async processPaperSteps(e){let t={};for(const r of e){t[r.set_id]||(t[r.set_id]={steps:[],examType:r});const e=replacePathSeparator(`${this.UNZIP_LOCAL_PATH}\\${this.getTemplateFilePath(r.template.id)}`),n=replacePathSeparator(`${e}\\ctrl_check.json`),o=await existFileOrFloder(n)?"ctrl_check.json":"ctrl.json",i=await this.getJson(replacePathSeparator(`${this.getTemplateFilePath(r.template.id)}\\${o}`)),a=lodashExports.cloneDeep(i);for(const s of r.exam_list){const e=a.filter(e=>{const t=e.subid.match(matchRegex);return t&&t[1]===s.code_id}).map(e=>(e.subid=e.subid.replace(matchRegex,s.real_id),e.order_num=e.order,e.order=`${e.order_num}_custom_order_${e.subid}_${e.order_num}`,{...e,exam:s}));if("collector.repeat"===r.exam_type_collect){const n=await this.getJson(replacePathSeparator(`${this.getContentFilePath(s.real_id)}\\info_extra.json`)),o=e.filter(e=>e.subid.includes("_0")),i=e.filter(e=>!e.subid.includes("_0"));let a=0;for(const e of n){let t=JSON.stringify(i);for(const o of e)t=t.replace(new RegExp(o.id,"g"),o.value);let n=JSON.parse(t);const r=o[o.length-1];let s=Number(r.order)||Number(r.order_num);const l=Number(s)+1,c=(e,t)=>e.replace(/_(\d+)$/,(e,n)=>`_${Number(n)+t}`);n=n.map(e=>{let t=e.subid;return s++,{...e,subid:c(t,a),order:`${l}_custom_order_${e.subid}_${s}`,order_num:String(s)}}),a++,o.push(...n)}t[r.set_id].steps=t[r.set_id].steps.concat(o)}else t[r.set_id].steps=t[r.set_id].steps.concat(e)}}let n={};for(const[r,o]of Object.entries(t)){const{steps:e,examType:t}=o;n[r]=[];for(const o of e){const e={...o};if("view.html"===e.filetype&&e.model_pc){const n=await this.processJsParam(e.model_pc.jsparam,e.exam,t);e.model_pc.extractedJsParam=n}"file.audio"!==e.filetype&&"file.video"!==e.filetype||(e.filename=replacePathSeparator(`${this.generateFilePath(e.dirtype,e.exam.real_id,t.template.id)}\\material\\${e.filename}`)),"script"===e.filetype&&(e.filename=replaceRealId(e.filename,e.exam.real_id));const i=e.subid.split("_")[1];if("action.record"===e.filetype){const t=e.exam.question_list.find(e=>e.question_order===i);e.question=t}n[r].push(e)}}return n}
```
```text
// 关键行
let e=a.filter(e=>{ const t=e.subid.match(matchRegex); return t&&t[1]===s.code_id })
        .map(e=>( e.subid = e.subid.replace(matchRegex, s.real_id),
                  e.order_num = e.order,
                  e.order = `${e.order_num}_custom_order_${e.subid}_${e.order_num}`,   // ★ 排序键
                  {...e, exam:s} ));
// action.record 步骤绑定小题：
if("action.record"===e.filetype){ const t=e.exam.question_list.find(e=>e.question_order===i); e.question=t }
   // i = e.subid.split("_")[1]
```
→ **`filetype==="action.record"` 的步骤才绑定 `step.question`**，其余步骤只有 `step.exam`。

### 页面级编排：Examine / MockExamDetails
**Examine 启动（考试/模考页）** — `chunks/Examine-d229829d.js`

```javascript
function Ga(){const e=Fe.query.homework_id;be({homework_id:e,set_id:je,mock:1}),K.getInstance().startCountAdd(),Ra()}
```
**MockExamDetails 事件绑定（answerHandler/chooseCheckHandler/chooseHandler/completeHandler）** — `chunks/MockExamDetails-2a7d03a6.js`

```javascript
ua.value.uploadFileTaskHandler(e=>{Ga.addTask(e)}),ua.value.answerHandler(async e=>{$.logCustom(`MockExamDetails.vue->answerHandler:${JSON.stringify(e)}`,{tag:"FLOW_PRAC"}),dt(e)}),ua.value.chooseCheckHandler(async()=>{Tt.value.forEach(e=>{$.logCustom(`MockExamDetails.vue检查答案步骤的提交${JSON.stringify(e)}`,{tag:"FLOW_PRAC"}),dt(e)})}),ua.value.chooseHandler(async e=>{const a=R.getInstance().handleRequestParams({resource_id:aa,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:Ye,entity_id:e.entity_id,order:e.order,serial_id:Ie.serial_id,mistake_id:Fe??"",upload_file_id:"",upload_file_url:"",detail_file_id:"",detail_file_url:"",choose_id:e.score_detail.choose_id,score_detail:e.score_detail});a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score,Rt({detail_file:"",audio:"",detail:a.score_detail,score_detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,choose_id:a.score_detail.choose_id,standard_score:Oe(a)});const t=`${a.entity_id}_${a.order}`;Ta.value.forEach(e=>{const s=e.questions.find(e=>e.id==t);s&&(s.answer=void 0!==a.score_detail.choose_id&&""!=a.score_detail.choose_id)})}),ua.value.afterRecorderHandler(e=>{Ua.value.push(e)}),ua.value.recorderTasksHandler(e=>{const{id:a,promise:t}=Ba.addTask(e);t.catch(e=>{const a=()=>ua.value.retryRecord(e);e.recordError||Va.addTask(a,!1)}),ka=a}),
```
```text
ua.value.uploadFileTaskHandler(e=>{ Ga.addTask(e) }),                       // 上传任务队列
ua.value.answerHandler(async e=>{ log(...); dt(e) }),                          // ★ 单题上报
ua.value.chooseCheckHandler(async()=>{ Tt.value.forEach(e=>{ log(...); dt(e) }) }),  // ★ 检查选择题答案 → 全量上报
ua.value.chooseHandler(async e=>{ ...build a...; Rt({...}); ...选中的题打勾... }),
ua.value.afterRecorderHandler(e=>{ Ua.value.push(e) }),                        // 供超时强制重评
ua.value.recorderTasksHandler(e=>{ const{id,promise}=Ba.addTask(e);
      promise.catch(e=>{ const a=()=>ua.value.retryRecord(e); e.recordError||Va.addTask(a,!1) }); ka=id }),
ua.value.onStepperChange(e=>{ "2"==e.Extendedfield2 && (Ea.value=!0);        // Extendedfield2==2 → 显示答案卡
      Ke.value = na.value.findIndex(...);
      if(Ia.value != ua.value.currentContent.Extendedfield3){ Ia.value=...;  // 该 view.html 允许的 ctrl order 白名单
        const e=Ia.value.split(","),
              a=Object.keys(ua.value.steps).filter(a=>{const t=a.includes("custom_order_")?a.split("_")[0]:a; return e.some(e=>e==t)});
        $a.value = Z(a) } });
```

- `Extendedfield2`: `"2"` = 选择题/答案卡组；`"1"` = wait 型（见 `Re` 计算式）。
- `Extendedfield3`: 逗号分隔的 `ctrl order_num` 白名单，决定该 `view.html` 里可翻页的步。

## 2.2 逐题同步：`/m/audio/sync-v2`
### (1) 唯一出口 `requestAnswerLogs`（主 bundle，导出别名 `fr`）
**requestAnswerLogs 原样全文（★ 逐字段标注见下）** — `index-5543d811.js`

```javascript
const requestAnswerLogs=async e=>{const{ecardInfo:t}=useUserInfoStore$1(),n=useSpecialStore(),{first_column_id:r,question_type_score:o}=e;let i={};try{i="string"==typeof e.score_detail?JSON.parse(e.score_detail):e.score_detail}catch(e3){}const{total_score_init:a,total_score:s}=i||{},l=a||s,c=await n.getColumnsNeed125(r);c&&t?.rate_scale&&l&&o&&(e.real_score=scoreFormatter_125(o/5*l));const u=c?t?.rate_scale:void 0,d=c&&u?"0.125":void 0;i.rate_scale=u||0,delete i.total_score_init,delete i.xml_score,delete i.question_type_score,delete i.is_rejected,delete i.except_info,delete i.first_column_id,e.score_detail=JSON.stringify(i);const p=Object.assign(e,{rate_scale:u,graduation:d,client_time:currentTimestampToDate(),mark_service:1,version:"2"});return await hyRequest.postIt("/m/audio/sync-v2",Object.assign(p))
```

逐字段来源标注：

| 字段 | 来源 |
|---|---|
| `resource_id` | 路由 `query.resource_id`（`Oe`/`aa`） |
| `set_id` | 路由 `query.set_id`（`je`/`Ye`） |
| `homework_id` | `HomeworkReviewSingleton.getId()`；非作业时为 `""`（`handleRequestParams` 注入 `homework_id: a.value`） |
| `entity_id` | 答题 handler 的 `entity_id` = `step.exam.exam_id` |
| `order` | `question.question_order`（录音题）/ 小题 `order` |
| `serial_id` | 页面 state 的 `serial_id`（PK/续做场景） |
| `mistake_id` | `route.query.mistake_id ?? ""` |
| `upload_file_id` / `upload_file_url` / `spx_file_url` / `mark_trace_id` / `detail_file_id` / `detail_file_url` | `useRecordResult.getResult()` 返回值（本地录音 + OSS 上传结果 + 评测 XML） |
| `score` | `score_detail.total_score` |
| `real_score` | `score_detail.real_score ?? score_detail.total_score`，再被 125 规则覆盖 |
| `score_detail` | 单题评测 `I` 对象（录音题）或选择题 `{total_score, choose, choose_id, category}`，**序列化成 JSON 字符串** |
| `first_column_id` | 顶层，来自 stepper 的 `g = examItem.type || examItem.first_column_id || ""` |
| `question_type_score` | 顶层，选择题 = `exam.exam_score`，录音题 = `question.question_score` |
| `correct` | 仅订正场景：`route.query.redoIndex ? 1 : undefined` |
| `question_total_point` | 仅 `Examine`：`state.total_point` |
| `rate_scale` | `getColumnsNeed125(first_column_id) ? ecardInfo.rate_scale : undefined` |
| `graduation` | `rate_scale ? "0.125" : undefined`，两者同时成立才给 |
| `client_time` | `currentTimestampToDate()` |
| `mark_service` | 常量 `1` |
| `version` | 常量 `"2"` |
| `exam_id` | **被 `handleRequestParams` 强制置 `null`** |

**handleRequestParams（清 exam_id、补 homework_id）** — `index-5543d811.js`

```javascript
handleRequestParams:e=>(e.exam_id=null,{...e,homework_id:a.value}),requestHomeworkStructDetail:E,requestScoreDetail:e=>e.first_column_id==FirstColumnType.PhoneticNew?requestPhoneticHomeworkGetScore({homework_id:e.homewor
```
**useAudioSync：默认空壳 body + postAudioSyncLogs（唯一调用 fr 的地方）** — `chunks/useAudioSync-0603b875.js`

```javascript
import{fr as e}from"./index-5543d811.js";function i(){const i={resource_id:"",real_score:0,score:0,set_id:"",entity_id:"",order:"",serial_id:"",mistake_id:0,upload_file_id:"",upload_file_url:"",spx_file_url:"",mark_trace_id:"",detail_file_id:"",detail_file_url:"",score_detail:""};return{postAudioSyncLogs:function(r={}){const t={...i,...r};return new Promise((i,r)=>{e(t).then(e=>{i(e)}).catch(e=>{r(e)})})}}}export{i as u};

```
```text
const i={ resource_id:"", real_score:0, score:0, set_id:"", entity_id:"", order:"",
           serial_id:"", mistake_id:0, upload_file_id:"", upload_file_url:"",
           spx_file_url:"", mark_trace_id:"", detail_file_id:"", detail_file_url:"",
           score_detail:"" };
postAudioSyncLogs: function(r={}){ const t={...i,...r}; ... e(t) }   // e = requestAnswerLogs
```

### (2) 页面侧逐题请求体构造（选择题：`chooseHandler`）
**Examine.chooseHandler** — `chunks/Examine-d229829d.js`

```javascript
Ze.value.chooseHandler(async e=>{const a=K.getInstance().handleRequestParams({resource_id:Oe,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:je,entity_id:e.entity_id,order:e.order,serial_id:R.serial_id,upload_file_id:"",upload_file_url:"",detail_file_id:"",detail_file_url:"",choose_id:e.score_detail.choose_id,score_detail:e.score_detail});a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score,ka({detail_file:"",audio:"",detail:a.score_detail,score_detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,choose_id:a.score_detail.choose_id,standard_score:""});const l=`${a.entity_id}_${a.order}`;da.value.forEach(e=>{const t=e.questions.find(e=>e.id==l);t&&(t.answer=void 0!==a.score_detail.choose_id&&""!=a.score_detail.choose_id)})}),
```
**MockExamDetails.chooseHandler** — `chunks/MockExamDetails-2a7d03a6.js`

```javascript
ua.value.chooseHandler(async e=>{const a=R.getInstance().handleRequestParams({resource_id:aa,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:Ye,entity_id:e.entity_id,order:e.order,serial_id:Ie.serial_id,mistake_id:Fe??"",upload_file_id:"",upload_file_url:"",detail_file_id:"",detail_file_url:"",choose_id:e.score_detail.choose_id,score_detail:e.score_detail});a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score,Rt({detail_file:"",audio:"",detail:a.score_detail,score_detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,choose_id:a.score_detail.choose_id,standard_score:Oe(a)});const t=`${a.entity_id}_${a.order}`;Ta.value.forEach(e=>{const s=e.questions.find(e=>e.id==t);s&&(s.answer=void 0!==a.score_detail.choose_id&&""!=a.score_detail.choose_id)})}),
```
```text
const a = K.getInstance().handleRequestParams({
  resource_id: Oe,                                             // 路由 query
  real_score : e.score_detail.real_score ?? e.score_detail.total_score,
  score      : e.score_detail.total_score,
  set_id     : je,
  entity_id  : e.entity_id,
  order      : e.order,
  serial_id  : R.serial_id,
  upload_file_id:"", upload_file_url:"", detail_file_id:"", detail_file_url:"",
  choose_id  : e.score_detail.choose_id,                       // 旧族：选项序号
  score_detail: e.score_detail
});
a.first_column_id = e.first_column_id;
a.question_type_score = e.question_type_score;
ka({ detail_file:"", audio:"", detail:a.score_detail, score_detail:a.score_detail, entity_id:a.entity_id,
     order:a.order, real_score:a.real_score, choose_id:a.score_detail.choose_id, standard_score:"" });
// 选中态回写：
const l = `${a.entity_id}_${a.order}`;
na.value.forEach(e=>{ const t=e.questions.find(e=>e.id==l); t && (t.answer = void 0!==a.score_detail.choose_id && ""!=a.score_detail.choose_id) });
```

### (3) 页面侧逐题请求体构造（录音题：`wt()` / `Qa()`）★ 完整请求体
**MockExamDetails.wt(e) —— 逐题上报完整构造（原样）** — `chunks/MockExamDetails-2a7d03a6.js`

```javascript
async function wt(e){const a=R.getInstance().handleRequestParams({resource_id:aa,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:Ye,entity_id:e.entity_id,order:e.order,serial_id:Ie.serial_id,mistake_id:Fe??"",upload_file_id:e.upload_file_id??"",upload_file_url:e.upload_file_url??"",spx_file_url:e.spx_file_url??"",mark_trace_id:e.mark_trace_id??"",detail_file_id:e.detail_file_id??"",detail_file_url:e.detail_file_url??"",score_detail:e.score_detail,correct:Xe.query.redoIndex?1:void 0});if(a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score,Rt({detail_file:e.xml_local_path??"",audio:e.audio_local_path??"",detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,standard_score:Oe(a),score_detail:a.score_detail,choose_id:a.score_detail.choose_id}),e.noPost)return;const{postAudioSyncLogs:t}=qe();$.logCustom(`MockExamDetails.vue获取上报选择题数:${JSON.stringify(a)}`,{tag:"FLOW_PRAC"});return await t(a)}
```
**Examine.Qa(e) —— 逐题上报完整构造（原样）** — `chunks/Examine-d229829d.js`

```javascript
async function Qa(e){let a=K.getInstance().handleRequestParams({resource_id:Oe,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:je,entity_id:e.entity_id,order:e.order,upload_file_id:e.upload_file_id??"",upload_file_url:e.upload_file_url??"",spx_file_url:e.spx_file_url??"",mark_trace_id:e.mark_trace_id??"",detail_file_id:e.detail_file_id??"",detail_file_url:e.detail_file_url??"",score_detail:e.score_detail,question_total_point:R.total_point});if(ka({detail_file:e.xml_local_path??"",audio:e.audio_local_path??"",detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,standard_score:"",choose_id:a.score_detail.choose_id,score_detail:a.score_detail}),e.noPost)return;const{postAudioSyncLogs:l}=xe();V.logCustom(`Examine.vue获取上报选择题数:${JSON.stringify(a)}`,{tag:"FLOW_PRAC"});return await l(a)}
```
两者差别只在 `Examine` 多传一个 `question_total_point: R.total_point`，且 `Examine` 没有 `mistake_id`（旧族字段）。

### (4) 单题 `score_detail` 的生成（录音题）
**useRecordResult.getResult → score_detail `I`（原样）** — `chunks/useRecordResult-fbe9e75b.js`

```javascript
const I={...R,total_score:_(w),total_score_init:w,real_score:G,category:e,accuracy_score:$,fluency_score:b,standard_score:O,first_column_id:p,question_type_score:h};try{const e=t?.rawFilePath??t.name;let r=[];const[a,c]=await g(e,t=>{i.logCustom(`上传评测本地录音文件，${e}`,{tag:"ENG_SCORE"}),r.push(t)});let l="",s="";t.data.xml&&(s=await o(e,t.data.xml),l=await m(s,t.data.xml,e=>{i.logCustom(`上传评测本地xml文件，${s}`,{tag:"ENG_SCORE"}),r.push(e)})),y?y(()=>Promise.allSettled(r.map(e=>e()))):r.forEach(e=>e());const _={sdk_recode:t.code,engine_recode:k,total_score:C,is_rejected:F,sid:t.sid,channel_type:t.data.channelType,wav_url:a,spx_url:c,mark_trace_id:t?.traceId||t?.sid,xml_url:l};i.logCustom(`评测解析结果：${JSON.stringify(_)}`,{tag:"ENG_SCORE"});try{window.triggerBuriedPoint(x,{i_score:R.xml_score,i_totalScore:I.question_type_score,i_fluency:R.fluency_score,i_accuracy:R.accuracy_score,i_completion:R.integrity_score})}catch(L){}return{xml_local_path:s,audio_local_path:e,upload_file_url:a,spx_file_url:c,mark_trace_id:t?.traceId||t?.sid,detail_file_url:l,upload_file_id:"",detail_file_id:"",score_detail:I}
```
```text
const I={...R,                       // R = 引擎返回的 xml 解析结果（total_score/accuracy/fluency/standard/integrity/is_rejected/except_info...）
  total_score: _(w),                 // _( = scoreFormater) → 2 位取整
  total_score_init: w,               // 未取整原始分（0~5）
  real_score: G,                     // G = h ? n(h,w) : 0；n = calculateRealScore, h = question_type_score
  category: e,                       // = evaluation_type（read_word/read_sentence/...）
  accuracy_score:$, fluency_score:b, standard_score:O,
  first_column_id: p,                // 来自调用方 g
  question_type_score: h             // 来自调用方 o.question.question_score
};
```

**评测异常降零规则（0 分/忽略拒绝）** — `chunks/useRecordResult-fbe9e75b.js`

```javascript
shouldZero:S,ignoreReject:j,hitRule:E}=function(e,t,o){const a=String(e?.engine_code??"");if(a&&u.includes(a))return{shouldZero:!0,ignoreReject:!1,hitRule:1};const c=Number(t?.except_info??0),l=1===Number(t?.is_rejected??0);if(c===r.IgnorableException)return{shouldZero:!("1"===String(o?.ignore_28707_error??"")),ignoreReject:!1,hitRule:2};if(l||d.includes(c)){const e=parseFloat(String(o?.pc_accept_reject_score??"0"))||0;return(parseFloat(String(t?.total_score??0))||0)<=e?{shouldZero:!0,ignoreReject:!1,hitRule:3}:{shouldZero:!1,ignoreReject:!0,hitRule:3}}return{shouldZero:!1,ignoreReject:!1,hitRule:0}}(t,R,N);S&&(R={...R,total_score:0,xml_score:0,accuracy_score:0,fluency_score:0,standard_score:0,integrity_score:0}),E>0&&i.logCustom(`评测错误码命中规则 [rule=${E}] shouldZero=${S} ignoreReject=${j} engine_code=${t?.engine_code??""} except_info=${R.except_info} is_rejected=${R.is_rejected}`,{tag:"ENG_SCORE"});
```
```text
const u=[Silent,NoData,LowVolume], d=[NoSpeechOrLowVolume,Nonsense,Abnormal];
// hitRule 1：引擎码命中静音/无数据/音量低 → total_score=0 全维度清零
// hitRule 2：except_info == IgnorableException 且系统配置 ignore_28707_error != "1" → 清零
// hitRule 3：is_rejected==1 或 unknown 异常 → 若 total_score <= pc_accept_reject_score 则清零，否则 ignoreReject
```

### (5) 逐题上报的任务队列语义
**AsyncQueue.addTask / processQueue / onComplete（提交顺序与并发）** — `index-5543d811.js`

```javascript
addTask(e,t=!0){const n=this.nextId++,r=new Promise((r,o)=>{this.queue.push({taskFunction:e,resolve:r,reject:o,id:n}),this.totalTasks.push({taskFunction:e,id:n}),this.onHandlerAddTaskCallback(),setTimeout(()=>{t&&this.processQueue()})});return{id:n,promise:r}}runAllTasks(){this.processQueue()}async processQueue(){const e=async()=>{if(0!==this.queue.length||0!==this.activeTasks.size)for(;this.queue.length>0&&this.activeTasks.size<this.maxConcurrent;){const t=this.queue.shift();if(!t||t.isCancelled)continue;const{taskFunction:n,resolve:r,reject:o,id:i}=t,a=async()=>{try{const e=await n();this.successTasks.push(e),this.updateTotalTasks(i,{taskFunction:n,result:e,id:i}),r(e)}catch(a){this.failedTasks.push({taskFunction:n,error:a,id:i}),this.updateTotalTasks(i,{taskFunction:n,error:a,id:i}),o(a)}finally{this.activeTasks.delete(i),this.onHandlerTaskCallback?.(t),e()}};this.activeTasks.add(i),a()}else"function"==typeof this.onCompleteCallback&&this.onCompleteCallback(this.failedTasks)};e()}getFailedTasks(){return this.failedTasks}getActiveTasks(){return this.activeTasks}getQueueList(){return this.queue}
```
队列使用（Examine / MockExamDetails 完全一致）：
- `He/Ba`（`recorderTasks`）并发 5：录音评测；失败入 `Me/Va`（`retryRecordTasks`）
- `Ge/Ga`（`uploadFileTasks`）并发 5：OSS 上传
- `Qe/Wa`（`postAnswerTasks`）并发 5：**逐题 sync**
- `实现在录音页面关闭前 `await Qe/Wa.waitForComplete()` 再收尾。

**收尾流程（Examine）** — `chunks/Examine-d229829d.js`

```javascript
async function La(e){const{goBack:a=!1,callback:l}=e||{};V.logCustom(`触发了taskQueueHandler, uploadFileTasks: total: ${Ge.getTotalList().length}, success: ${Ge.getSuccessList().length}, failed: ${Ge.getFailedTasks().length}`,{tag:"FLOW_PRAC"}),await se(),Ge.onHandlerTask(()=>{V.logCustom(`评测完毕uploadFileTasks任务详情: total: ${Ge.getTotalList().length}, success: ${Ge.getSuccessList().length}, failed: ${Ge.getFailedTasks().length}`,{tag:"FLOW_PRAC"})}),Ge.onComplete(async e=>{if(V.logCustom(`预上传任务处理完成，失败任务uploadFailTasks: ${e.length}`,{tag:"FLOW_PRAC"}),e&&e.length){Z.close(Ne);const a=`<div>\n                    <div>评分失败, 请检查配置及网络</div>\n                    <div>存在<span class="color-danger">${e.length}</span>道小题上传成绩失败</div>\n                  </div>\n                `;return void(await Ge.askUserToRetry(a,{cancelButtonText:"取消"})?(Sa("上传中，请稍等"),Ge.retryFailedTasks(e)):za().catch(()=>{Sa("上传中，请稍等"),Ge.retryFailedTasks(e)}))}await Qe.waitForComplete(),Qe.addTask(()=>r.update({type:String(re.listeningSpeaking),set_id:je,homework_id:K.getInstance().getId(),resource_id:Oe,mistake_id:d})),Qe.onComplete(async e=>{if(V.logCustom(`成绩上报任务处理完成，失败任务failedTasks: ${e.length}`,{tag:"FLOW_PRAC"}),e.length>0){Z.close(Ne);const a=`<div>\n                      <div>评分失败, 请检查配置及网络</div>\n                      <div>存在<span class="color-danger">${e.length}</span>道小题上传成绩失败</div>\n                    </div>\n                  `;return void(await Qe.askUserToRetry(a,{cancelButtonText:"取消"})?(Sa("上传中，请稍等"),Qe.retryFailedTasks(e)):za().catch(()=>{Sa("上传中，请稍等"),Qe.retryFailedTasks(e)}))}V.logCustom("taskQueueHandler触发"+(a?"返回上一页":"跳转结果页"),{tag:"FLOW_PRAC"}),Z.close(Ne),l?l():a?Pe.go(-1):(Ae({set_id:je,homework_id:Fe.query.homework_id??""}),Pe.replace({name:"mockExamResult",query:Fe.query,state:{data:JSON.stringify(R)}}))})})}
```
**收尾流程（MockExamDetails）** — `chunks/MockExamDetails-2a7d03a6.js`

```javascript
async function ut(e){const{goBack:a=!1,callback:t}=e||{};$.logCustom(`触发了taskQueueHandler, uploadFileTasks: total: ${Ga.getTotalList().length}, success: ${Ga.getSuccessList().length}, failed: ${Ga.getFailedTasks().length}`,{tag:"FLOW_PRAC"}),await z(),Ga.onHandlerTask(()=>{$.logCustom(`评测完毕uploadFileTasks任务详情: total: ${Ga.getTotalList().length}, success: ${Ga.getSuccessList().length}, failed: ${Ga.getFailedTasks().length}`,{tag:"FLOW_PRAC"})}),Ga.onComplete(async e=>{if($.logCustom(`预上传任务处理完成，失败任务uploadFailTasks: ${e.length}`,{tag:"FLOW_PRAC"}),e&&e.length){j.close(Qa);const a=`<div>\n                    <div>评分失败, 请检查配置及网络</div>\n                    <div>存在<span class="color-danger">${e.length}</span>道小题上传成绩失败</div>\n                  </div>\n                `;return void(await Ga.askUserToRetry(a,{cancelButtonText:"取消"})?(nt("上传中，请稍等"),Ga.retryFailedTasks(e)):gt().catch(()=>{nt("上传中，请稍等"),Ga.retryFailedTasks(e)}))}await Wa.waitForComplete(),Wa.addTask(()=>$e.update({type:String(U.listeningSpeaking),set_id:Ye,homework_id:R.getInstance().getId(),resource_id:aa,mistake_id:Fe})),Wa.onComplete(async e=>{if($.logCustom(`成绩上报任务处理完成，失败任务failedTasks: ${e.length}`,{tag:"FLOW_PRAC"}),e.length>0){j.close(Qa);const a=`<div>\n                      <div>评分失败, 请检查配置及网络</div>\n                      <div>存在<span class="color-danger">${e.length}</span>道小题上传成绩失败</div>\n                    </div>\n                  `;return void(await Wa.askUserToRetry(a,{cancelButtonText:"取消"})?(nt("上传中，请稍等"),Wa.retryFailedTasks(e)):gt().catch(()=>{nt("上传中，请稍等"),Wa.retryFailedTasks(e)}))}$.logCustom("taskQueueHandler触发"+(a?"返回上一页":"跳转结果页"),{tag:"FLOW_PRAC"}),j.close(Qa),t?t():a?Ze.go(-1):pt()})})}
```
```text
// 关键：录音失败/超时的兜底重评
await Ba/He.onComplete(async e => { if(e.length>0) return void(t.value=e) ; ut() })
// 强制完成 forcedCompletion：把未评测的录音重新入队，并回退到「上传」态
```

## 2.3 最终上报（练习/考试结束）
### ★ 词汇族最终上报 = `g/homework/score-sync`（**不是** `g/homework/report-score`）
**requestHomeworkWordCommit（原样全文，★ 词汇总分上报构造）** — `index-5543d811.js`

```javascript
requestHomeworkWordCommit:e=>{const t=useUserInfoStore$1().userInfo,n=a.value??"";try{let t=JSON.stringify(e);LoggerUtils.logCustom(`submit-score currentTime=${(new Date).getTime()}-params=${t}`)}catch(s){}const r=e.id||e.homework_id;if(r&&String(r)!==String(n))return LoggerUtils.logCustom(`submit-score blocked: params.id=${r} != store.homework_id=${n}，数据不属于当前作业，阻止提交`),Promise.resolve({code:-1,msg:"homework_id mismatch, submission blocked"});const o=isBcHomeworkStruct(e.wordInfo)?`${e.wordInfo?.unit_code??""}`:"",i={detail:0==e.pk?e.detail.map(e=>({...e,client_time:currentTimestampToDate()})):e.score_detail.map(e=>({word_id:e.word_id,question_id:e.question_id,...e.item_id&&e.question_item_id?{item_id:e.item_id,question_item_id:e.question_item_id}:{},score_rate:e.point>0?1:0,score_detail:JSON.stringify({choose:e.answer,score:e.point>0?100:0}),client_time:currentTimestampToDate()})),homework_id:parseInt(n),resource_id:e.resource_id,type:e.type,use_time:e.use_time,mark_service:1,seq_id:e.seq_id?e.seq_id:md5((t?.raw_phone??"")+n+new Date(Date.now()).toISOString(),16)};return requestHomeworkScoreSync(buildWordHomeworkScoreSyncBCParams(i,o))
```
字段来源逐条标注：

| 输出字段 | 来源 |
|---|---|
| `detail` | `pk==0` 时直接用入参 `e.detail`（每题已就绪，仅补 `client_time`）；`pk!=0` 时把 PK 的 `e.score_detail`（`{word_id,question_id,point,answer,...}` 数组）映射为同步格式 |
| `detail[i].word_id` | PK 里的 `word_id`（源自 `g/word/struct` 的 `struct.id`） |
| `detail[i].question_id` | PK 里的 `question_id`（源自 `question[].id`）；BC 词源时为空串 |
| `detail[i].item_id` / `question_item_id` | 仅当两者同时存在才写入（`pickBcIds`） |
| `detail[i].score_rate` | `e.point > 0 ? 1 : 0` —— **0/1 比率，不是分数** |
| `detail[i].score_detail` | `JSON.stringify({choose:e.answer, score:e.point>0?100:0})` —— **每题 100 分制** |
| `detail[i].client_time` | `currentTimestampToDate()` |
| `homework_id` | `parseInt(useHomeworkStore.homework_id)`；提交前用 `e.id||e.homework_id` 与 store 比对，不一致直接拒绝 |
| `resource_id` | 入参透传（路由 `query.resource_id`） |
| `type` | 入参透传（PK 场景 = `6`） |
| `use_time` | `HomeworkReviewSingleton.getSeconds()`（`requestWordHomeworkSubmit` 注入） |
| `mark_service` | 常量 `1` |
| `seq_id` | 入参有则用，否则 `md5((raw_phone??"") + homework_id + new Date().toISOString(), 16)` |
| `source` / `unit_code` | 当 `isBcHomeworkStruct(e.wordInfo)` 为真，追加 `{source:1, unit_code}` |

**buildWordHomeworkScoreSyncBCParams** — `index-5543d811.js`

```javascript
buildWordHomeworkScoreSyncBCParams=(e,t)=>t?{...e,source:1,[BC_UNIT_CODE_FIELD]:t}:e,requestHomeworkList=async e=>await hyRequest.postIt("g/homework/list",{version:10,...e,use_https:1,use_homework_pri
```
**接口定义：g/homework/score-sync** — `index-5543d811.js`

```javascript
requestHomeworkScoreSync=async e=>await hyRequest.postIt("g/homework/score-sync",e),requestHomeworkScoreDetail=async e=>await hyRequest.postIt("g/homework/score-detail",e),requestWordScoreDrop=async e=>await hyRequest.postIt("g/homework/score-drop",e),
```
**词汇成绩清空：g/homework/score-drop** — `index-5543d811.js`

```javascript
requestWordScoreDrop=async e=>await hyRequest.postIt("g/homework/score-drop",e),requestWordIsInNotebook=async e=>await hyRequest.p
```
**HomeworkList：重练词汇作业前先 drop 旧成绩** — `chunks/HomeworkList-f02d250d.js`

```javascript
da=async(e,t)=>{let a=!1;"result"==e&&(a=!0,We.from="homeworkList",sa.value="0",la.value="0"),"repair"==e&&(la.value="0",sa.value="1"),"1"==sa.value&&(We.first_column_id==u.VocabularyModule?(await I({resource_id:Ze.query.resource_id,seq_id:Ge.value.seq_id,homework_id:parseInt(r.getInstance().getId())}),Ge.value.seq_id=null,Ge.value.record=null):[u.ListeningSpeakingSynchronous,u.Phonetics,u.PhoneticNew].includes(Number(We.first_column_id))||await R(r.getInstance().handleRequestParams({resource_id:Ze.query.resource_id,set_id:We.set_id})));let o=C.cloneDeep(We);if(We.first_column_id==u.Mistake){let e=ft.value.filter(e=>!(e.hasOwnProperty("real_score")||0===e.real_score));if(o=t||e[0],o.structs=t?t.structs:e[0].structs,o.struct=t?t.structs[0]:e[0].structs[0],o.first_column_id=o.first_column_id,o.isHomeworkMistake=!0,o.mistakeFinish=e.length<=1,4==parseInt(o.first_column_id)){let e=o.structs.length?o.structs:[o.struct];return ct=new c(e),void ct.processPaperResources().then(async e=>{const t=await ct.processPaperDataJson(e.exam_type_list[0]);o.contentJson=await ct.processPaperContentJson(e.exam_type_list[0]),o.lastRes=t,o.historeScore=Ge.value,r.getInstance().startHomework(Object.assign({},We,{...o,isRestart:sa.value,seq_id:Ge.value.seq_id,record:Ge.value.record}),la.value,a)})}}"1"==r.getInstance().needDownloadRes(We)||(o=ut.value?{...o,...Ge.value??{},dub:ut.value}:nt.value?{...o,...Ge.value??{},wordInfo:nt.value}:{answerCount:it.value,totalScore:st.value,combination:ot.value,composition:lt.value}),o.historeScore=Ge.value,o.isContinue=la.value,r.getInstance().startHomework(Object.assign({},We,{...o,exam:We.exam,homeworkExam:We.exam,isRestart:"1"==sa.value?"1":"0",seq_id:Ge.value.seq_id,record:Ge.value.record}),la.value,a)},_a=e=>{e&&e.innerHTML&&pa(e.innerHTML,e)},pa=(e,t)=>{g(()=>{const a=Q(e)?.color;let o=ee(a,t);t.style.color=o.value})};let fa=null;const ha=T();let ga=P(!1);
```
```text
"1"==sa.value && ( We.first_column_id==u.VocabularyModule
  ? ( await I({ resource_id: Ze.query.resource_id, seq_id: Ge.value.seq_id, homework_id: parseInt(r.getInstance().getId()) }),
      Ge.value.seq_id = null, Ge.value.record = null )      // I = requestWordScoreDrop = g/homework/score-drop
  : [ListeningSpeakingSynchronous,Phonetics,PhoneticNew].includes(Number(We.first_column_id))
      || await R(r.getInstance().handleRequestParams({ resource_id:..., set_id... })) )  // R = requestResetScore = /m/set/reset-score-v2
```

**词汇成绩提交失败错误码 60001** — `chunks/useWordResourceError-06c28489.js`

```javascript
import{L as e,U as r,gq as s}from"./index-5543d811.js";const o=["BC_WORD_MAPPING_NOT_FOUND","BC_QUESTION_MAPPING_NOT_FOUND"],n=[2001,2002,2003,2004,2005],i={5001:"资源未找到，请稍后重试",5003:"该单词包下暂无单词，请稍后重试",3002:"请求参数越界，请重新进入",9001:"系统繁忙，请稍后重试"};function t(t,c){const a=`${t??""}`;if(e.logCustom(`资源版同步词汇错误 code=${a} msg=${c??""}`,{type:2}),o.some(e=>a.includes(e)||(c??"").includes(e)))r.warning("资源同步中，请稍后重试");else if(60001!==Number(t))if(n.includes(Number(t))){try{s().getResourcePlatformToken()}catch(u){}r.warning("登录态已更新，请重新进入")}else i[a]?r.warning(i[a]):r.warning(c||"资源加载失败，请稍后重试");else r.error("本次成绩提交失败（作业资源同步中），请稍后重试")}const c={showWordResourceError:t};export{c as default,t as showWordResourceError};

```
```text
i = { 5001:"资源未找到，请稍后重试", 5003:"该单词包下暂无单词，请稍后重试",
      3002:"请求参数越界，请重新进入", 9001:"系统繁忙，请稍后重试" };
... else if(60001!==Number(t)) ... else r.error("本次成绩提交失败（作业资源同步中），请稍后重试")
// 另有 BC_WORD_MAPPING_NOT_FOUND / BC_QUESTION_MAPPING_NOT_FOUND → "资源同步中，请稍后重试"
// 2001~2005 → 刷 token + "登录态已更新，请重新进入"
```

### 提交入口分派（`HomeworkReviewSingleton`）
**requestWordHomeworkSubmit / requestHomeworkSubmit 分派** — `index-5543d811.js`

```javascript
requestWordHomeworkSubmit=e=>{if(1==this.practiceType){const t=this.getSeconds();return this.homeworkPractice.requestHomeworkWordCommit({...e,use_time:t})}return requestPKSubmitWord(e)};requestReadWritingScoreDetail=e=>1==this.practiceType?this.homeworkPractic
```
```text
requestHomeworkSubmit  = e => 1==practiceType ? homeworkPractice.requestHomeworkSub(e)
                                              : requestSubmitReadingWritingExam(e);   // /m/exam/submit
requestHomeworkSub     = e => requestHomeworkSubmit({...e, id:a.value, version:2,
                                                      use_time: e.use_time || l.seconds.value});  // m/homework/submit
requestWordHomeworkSubmit = e => { if(1==practiceType){ const t=this.getSeconds();
                                      return this.homeworkPractice.requestHomeworkWordCommit({...e, use_time:t}) }
                                   return requestPKSubmitWord(e) };   // g/pk/submit-word（非作业=PK）
```

> ⚠️ 因此：**词汇 PK（非作业）走 `g/pk/submit-word`，词汇作业走 `g/homework/score-sync`**，两者请求体不同（后者由 `requestHomeworkWordCommit` 包装）。

### `g/homework/report-score` / `g/set/report-score` 的真实归属
**report-score 家族定义（音标/PhoneticNew 流程）** — `index-5543d811.js`

```javascript
requestPhoneticSectionList=async e=>await hyRequest.postIt("/proxy/phoneticForStudent/sectionList",e),requestPhoneticResourcePageItems=async e=>await hyRequest.postIt("/proxy/resourcePlatformInterface/pageItems",e),requestPhoneticResourceFindByIds=async e=>await hyRequest.postIt("/proxy/resourcePlatformInterface/findByIds",e),requestPhoneticGetScore=async e=>await hyRequest.postIt("g/set/get-score",e),requestPhoneticReportScore=async e=>await hyRequest.postIt("g/set/report-score",e),requestPhoneticHomeworkStruct=async e=>await hyRequest.postIt("v2/homework/struct",e),requestPhoneticHomeworkGetScore=async e=>await hyRequest.postIt("g/homework/get-score",e),requestPhoneticHomeworkReportScore=async
```
```text
requestPhoneticGetScore            = g/set/get-score
requestPhoneticReportScore         = g/set/report-score            // 音标（旧）
requestPhoneticHomeworkStruct      = v2/homework/struct
requestPhoneticHomeworkGetScore    = g/homework/get-score
requestPhoneticHomeworkReportScore = g/homework/report-score      // 音标新（PhoneticNew）作业
```
→ **词汇族完全不使用 report-score**。`g/homework/report-score` 只被 `requestPhoneticHomeworkReportScore` 引用（在缺失的 `PhoneticNew*` chunk 中）。

### 其它结束态动作
- **错题本上报**（不是成绩上报）：`mistakeAdderStore.update()` → `m/mistake/update`：
**mistakeAdder.update（含 sync 开关）** — `chunks/mistakeAdder-2da3b6d6.js`

```javascript
i=>{i.question.forEach(i=>{n.push(i.id)})}),n},check:async function(n,e,r){if(c(),0==i.ecardInfo?.mistake_support)return;if(1==s?.show_answer_time)return;const d={entity_id:[],composition_id:[],question_id:[],parent_account_id:i.userInfo?.parent_id??"",...r};if(n===t.composition?d.composition_id=e:n===t.readingWriting?d.question_id=e:n===t.listeningSpeaking&&(d.entity_id=e),!n)return;function u(i){return i.filter(i=>0===i.is_mistake)}const g=await o(d);0===g.code&&(n===t.composition?a[t.composition]=u(g.body.composition):n===t.readingWriting?a[t.readingWriting]=u(g.body.question):n===t.listeningSpeaking&&(a[t.listeningSpeaking]=u(g.body.entity)))},update:function(n){if(0===i.ecardInfo?.mistake_support)return Promise.resolve();if(1===s?.show_answer_time&&0===s.expire)return Promise.resolve();const e={homework_id:"",set_id:"",exam_id:"",mistake_id:"",combination_id:"",composition_id:"",...n,resource_id:n?.resource_id||i?.ecardInfo?.resource_id||"",type:String(n.type),sync:n.mistake_id?"1":"0"};return r(e)},checkAndGetTooltipStatus:function(n,e){if(0===i.ecardInfo?.mistake_support)return!1;if(1===s?.show_answer_time&&0===s.expire)return!1;let o=!1;function r(i){return!i.length&&!!e.length||i.filter(i=>e.find(n=>n===i.id)).length>0}return n===t.composition?(o=r(a[t.composition]),a[t.composition]=[]):n===t.readingWriting?(o=r(a[t.readingWriting]),a[t.readingWriting]=[]):n===t.listeningSpeaking&&(o=r(a[t.listeningSpeaking]),a[t.listeningSpeaking]=[]),o},$reset:c}});export{s as u};

```
- `Examine` 结束：`r.update({type:String(MistakeType.listeningSpeaking), set_id, homework_id, resource_id, mistake_id})`
- `MockExamDetails` 结束：`Wa.addTask(()=>$e.update({type:..., set_id:Ye, homework_id:..., resource_id:aa, mistake_id:Fe}))`
- 计时/使用时长：`m/homework/set-use-time`（`requestSetUseTime`），`resolveUseTimeFromSetUseTimeResp` 读 `body.user_time ?? body.use_time`。

## 2.4 分数计算（含 `scoreFormatter_125` / `calculateRealScore` 的真实调用链）
### (1) 四个基础函数（主 bundle，全局唯一定义）
**scoreFormater / scoreFormatter_125 / ConvertToPercentScore / calculateRealScore** — `index-5543d811.js`

```javascript
function scoreFormater(e,t=2){let n=Number(e)*Math.pow(10,t);return Math.round(Number(n))/Math.pow(10,t)}function scoreFormatter_125(e,t=2){return scoreFormater(.125*Math.ceil(Number(e)/.125))}function ConvertToPercentScore(e,t,n=0){return e&&t?scoreFormater(Number(e)/Number(t)*100,n):0}function calculateRealScore(e,t){return scoreFormater(Number(e)/5*Number(t))}function calculateRealScoreWithCategory(e,t,n){return EvaluationTypes.includes(n)?calculateRealScore(e,t):e}function getScoreLevelInfo(e){return void 0===e?{level:"",color:""}:e<60?{level:"C",color:"--score-color-c"}:e<80?{level:"B",color:"--score-color-b"}:e<=100?{level:"A",color:"--score-color-a"}:{level:"",color:""}}
```
```text
function scoreFormater(e,t=2){ let n=Number(e)*Math.pow(10,t); return Math.round(Number(n))/Math.pow(10,t) }
function scoreFormatter_125(e,t=2){ return scoreFormater(.125*Math.ceil(Number(e)/.125)) }
                                   // ★ 向上取整到 1/8 (0.125) 的整数倍
function ConvertToPercentScore(e,t,n=0){ return e&&t ? scoreFormater(Number(e)/Number(t)*100,n) : 0 }
function calculateRealScore(e,t){ return scoreFormater(Number(e)/5*Number(t)) }
                                   // ★ 5 分制原始分 → 实际分
function calculateRealScoreWithCategory(e,t,n){ return EvaluationTypes.includes(n)?calculateRealScore(e,t):e }
```

### (2) `getColumnsNeed125` / `percentageQuestion` —— 触发 125 拉伸的唯一条件
**specialStore.getSpecialColumn / getColumnsNeed125** — `index-5543d811.js`

```javascript
defineStore("specialStore",{state:()=>({loading:!0,resource_id:0,columnModels:[],specialColumn:[]}),actions:{async handleGradeItem(){this.loading=!0;let e=await requestExamMsg({resource_id:this.resource_id,tag_id:2});0==e.code&&(this.columnModels=e.body.data),this.loading=!1},async getSpecialColumn(){if(this.specialColumn&&this.specialColumn.length)return[];const e=["2","3","5"];let t=await requestSpecialColumn(this.resource_id);if(0==t.code){let n=[];e.map(e=>{let r=Object.values(t.body).filter(t=>t.id==e);n.push(...r)}),this.specialColumn=n}},async getColumnsNeed125(e){const t=[FirstColumnType.SimulationExam,FirstColumnType.ListeningSimulation,FirstColumnType.SynchronousListening];await this.getSpecialColumn();return this.specialColumn.reduce((e,t)=>[...e,...t.first_column.map(e=>e.id)],t).some(t=>t==e)}},persist:{afterRestore:async e=>{const t=useUserInfoStore$1();e.store.resource_id=0==e.store.resource_id?t.userInfo?.resource_id:e.store.resource_id},paths:["book_id","resource_id","unit_id","pkIndex"]
```
```text
async getSpecialColumn(){ ... const e=["2","3","5"];              // id 2/3/5 的专题列
    let t = await requestSpecialColumn(this.resource_id);          // POST /m/set/column {resource_id}
    if(0==t.code){ let n=[]; e.map(e=>{ let r=Object.values(t.body).filter(t=>t.id==e); n.push(...r) }), this.specialColumn=n } }
async getColumnsNeed125(e){
    const t=[FirstColumnType.SimulationExam(1), FirstColumnType.ListeningSimulation(88), FirstColumnType.SynchronousListening(104)];
    await this.getSpecialColumn();
    return this.specialColumn.reduce((e,t)=>[...e,...t.first_column.map(e=>e.id)],t).some(t=>t==e) }
```

**useExamScore：scoreFilter / percentageQuestion** — `index-5543d811.js`

```javascript
const useExamScore=()=>{const e=[FirstColumnType.SynchronousListening,FirstColumnType.ReadingWriting,FirstColumnType.ListeningSpeakingSpecial,FirstColumnType.SpecialTraining,FirstColumnType.SimulationExam,FirstColumnType.ListeningSimulation,FirstColumnType.ReadingWritingSimulation];function t(t,n){return t&&e.every(e=>e!=t)||n&&5!=n}return{scoreFilter:function(e,t,n){return[n?scoreFormater(t):ConvertToPercentScore(t,e,0),n?e:100]},scoreFilterHomework:function(e,n,r,o,i){return[t(r,o)?scoreFormater(n):ConvertToPercentScore(n,e,0),i?e:100]},percentageQuestion:t}
```
```text
const useExamScore = () => {
  const e = [SynchronousListening(104), ReadingWriting(10), ListeningSpeakingSpecial(2),
             SpecialTraining(3), SimulationExam(1), ListeningSimulation(88), ReadingWritingSimulation(12)];
  function t(t,n){ return (t && e.every(e=>e!=t)) || (n && 5!=n) }    // = percentageQuestion
  return {
    // [显示分, 总分]
    scoreFilter: (rawTotal, score, isAbsolute) =>
        [ isAbsolute ? scoreFormater(score) : ConvertToPercentScore(score, rawTotal, 0), isAbsolute ? rawTotal : 100 ],
    scoreFilterHomework: (rawTotal,score,first_column_id,set_module_id,abs) =>
        [ t(first_column_id,set_module_id) ? scoreFormater(score) : ConvertToPercentScore(score, rawTotal, 0), abs ? rawTotal : 100 ],
    percentageQuestion: t,
  };
};
```
→ `percentageQuestion(first_column_id, set_module_id)` = 「该题是否按百分比展示」：
  `${first_column_id}` 不在 7 个「绝对分」模块里 → true；或者传了 `set_module_id` 且 ≠ 5（TLZX 听力专项）→ true。

### (3) `scoreFormatter_125` + `calculateRealScore` 的调用链（逐题 sync 里）
```text
调用方：requestAnswerLogs(e)                        // 唯一调用点（全库）
  输入 e.first_column_id、e.question_type_score
  1. i = JSON.parse(e.score_detail)
  2. l = i.total_score_init || i.total_score
  3. c = await useSpecialStore().getColumnsNeed125(e.first_column_id)
  4. if (c && ecardInfo?.rate_scale && l && e.question_type_score)
         e.real_score = scoreFormatter_125( question_type_score / 5 * l )       // ★ 125 拉伸
  5. u = c ? ecardInfo.rate_scale : undefined
     d = (c && u) ? "0.125" : undefined
  6. i.rate_scale = u || 0
     delete i.total_score_init / xml_score / question_type_score / is_rejected / except_info / first_column_id
  7. e.score_detail = JSON.stringify(i)
  8. POST /m/audio/sync-v2  with { ...e, rate_scale:u, graduation:d,
                                   client_time:currentTimestampToDate(), mark_service:1, version:"2" }
```

**触发条件（4 个 AND）：**
1. `first_column_id ∈ {1, 88, 104} ∪ specialColumn(2/3/5).first_column[].id`（`getColumnsNeed125`）；
2. `ecardInfo.rate_scale` 为真（电子卡配置，来自 `m/ecard/get`）；
3. `score_detail.total_score_init || score_detail.total_score` 为真；
4. `question_type_score` 为真。
→ 命中则：`real_score = ceil_to_1_8( question_type_score/5 × 原始5分制分 )`，且 `graduation="0.125"`。

### (4) 本地总分合成
#### 4a. 整套题（套装/模拟考）：`useExamResult.E`
**calculateAndFormatScoresWithExamType（总分 = Σ real_score，scoreFormater 取整）** — `chunks/useExamResult-d9fb4671.js`

```javascript
function E(e,t){const o=[];let i=0;for(const r of e.exam_list){const e=t.filter(e=>e.entity_id==r.exam_id);for(const t of e){const e=N(t.detail);if(t.detail=e,"number"==typeof t.real_score){i+=Number(t.real_score)||0,o.push(t);continue}if(0==t.standard_score){o.push({...t,real_score:0});continue}let s;if(e.real_score||0==e.real_score)s=e.real_score;else if(m.includes(e.choose)){const e=r.question_list.find(e=>e.question_order==t.order);s=f(t.standard_score,e.question_score)}else s=t.standard_score;i+=Number(s)||0,o.push({...t,real_score:s})}}return{updatedLogs:o,totalScore:h(i)}}
```
```text
function E(e,t){                       // e = examType, t = 该套题的 logs[]
  const o=[]; let i=0;
  for(const r of e.exam_list){
    const arr = t.filter(e=>e.entity_id==r.exam_id);
    for(const t of arr){
      const e=N(t.detail); if(t.detail=e, "number"==typeof t.real_score){ i+=Number(t.real_score)||0; o.push(t); continue }
      if(0==t.standard_score){ o.push({...t, real_score:0}); continue }
      let s;
      if(e.real_score || 0==e.real_score)            s = e.real_score;
      else if(m.includes(e.choose)){                 // m = EvaluationTypes
        const e = r.question_list.find(e=>e.question_order==t.order);
        s = f(t.standard_score, e.question_score);   // f = calculateRealScore
      } else s = t.standard_score;
      i += Number(s)||0; o.push({...t, real_score:s});
    }
  }
  return { updatedLogs:o, totalScore: h(i) };        // h = scoreFormater, 2 位
}
```
→ **本地总分 = Σ 各题 `real_score`**；缺失时用 `calculateRealScore(standard_score, question_score)` 兜底；全无则 0。

#### 4b. 同族（`collector.word` / `3q5a` / `role` / `dialogue` / `repeat`）逐小题字符串协议：`j()`
**j()：同族逐小题分数串构造（含余数吸收）** — `chunks/useExamResult-d9fb4671.js`

```javascript
function j(e,t,o,i){return t.reduce((t,r)=>{let s=o.filter(e=>e.entity_id===r.info.stid),n=0;const c=s.length,l=s.reduce((t,o)=>{n++;const r=function(e,t){const o=e.exam_list.find(e=>e.exam_id===t.entity_id);return o?o.question_list.find(e=>e.question_order===String(t.order)):{}}(e,o),{scoreFilter:s}=a(),[l,d]=s(r.question_score,o.real_score,i);let _=l,u=d;if(100==u&&c>1){let t=Number(r.question_score)/Number(e.exam_type_score);if(_=O(l,t,i,c),u=O(d,t,i,c),n==c){const t=u;u=(i?e.exam_type_score:100)-u*(c-1),_=_*u/t,_=h(_,i)}}const m=`${D}_${o.entity_id}_${o.order}${q}${_??0}${q}${u}${q}0`;return t.push(m),t},[]);return t=t.concat(l)},[])}
```
```text
function j(e,t,o,i){                        // e=examType, t=htmlData, o=logs, i=isAbsolute
  return t.reduce((t,r)=>{
    let s = o.filter(e=>e.entity_id===r.info.stid), n=0;
    const c = s.length;
    const l = s.reduce((t,o)=>{
      n++;
      const r = <按 entity_id+order 找 question_score 的那个小题>;
      const {scoreFilter:s} = useExamScore();
      let [l,d] = s(r.question_score, o.real_score, i);       // l=分, d=满分
      let _=l, u=d;
      if(100==u && c>1){                                      // ★ 同题多小题：按分值比缩放
        let t = Number(r.question_score)/Number(e.exam_type_score);
        _ = O(l,t,i,c), u = O(d,t,i,c);                       // O = t?e:scoreFormater(e*t, i>1?2:0)
        if(n==c){ const t=u; u = (i?e.exam_type_score:100) - u*(c-1);   // ★ 最后一题吸收余数
                  _ = _*u/t, _ = h(_,i) }
      }
      t.push(`${D}_${o.entity_id}_${o.order}${q}${_??0}${q}${u}${q}0`);  // q = \x01
      return t;
    },[]);
    return t.concat(l);
  },[]);
}
```
→ 传给 iframe 的格式：`{set_id}_{entity_id}_{order}\x01{得分}\x01{满分}\x01{0}`。

#### 4c. 结果页展示：`scoreFilter`
```text
// useExamResult 内（原样片段）
const{scoreFilter:l}=a(), [d,_]=l(n.examType.exam_type_score, t, i);
"collector.read"!==n.examType.exam_type_collect && (I()||c.showScore(_,d,0));
//   d = 得分, _ = 满分
```

#### 4d. PK 本地百分比
```text
let a = Math.ceil(Qe.value/(10*ie.length)*100);    // Qe 为「每题 10 分」的累计正确分
```
→ PK UI 口径是「10 分/题」，提交口径是「point=10000/题 且 score_rate=0/1」，**两套尺度并存**。

### (5) `question_type_score` / `total_score` / `rate_scale` / `first_column_id` 各自「在哪个接口的响应里」
| 字段 | 出处接口 | 响应位置 | 参与计算 |
|---|---|---|---|
| `first_column_id` | `g/homework/list` / `g/set/list-v2` / `m/homework/struct` / 本地 ctrl | 题目/作业对象顶层 | `getColumnsNeed125`、`percentageQuestion`、`scoreFilterHomework`、`needDownloadRes` |
| `exam_score`（→ `question_type_score`） | 本地 `content_*/res.json` 的 `exam_type_list[].exam_type_score` | `ProcessPaper.examTypeList` | 125 拉伸的分子基准；`real_score` 计算 |
| `question_score`（→ `question_type_score`，录音题） | 本地 `content.json` 的 `question_list[].question_score` / `info_extra.json` | `step.question` | `calculateRealScore(qts, total)` |
| `total_score` / `total_score_init` | 评测引擎（ISE），经 `useRecordResult` 落到 `score_detail` | `score_detail` | 125 拉伸的 `l` |
| `rate_scale` / `graduation` | `m/ecard/get`（`ecardInfo.rate_scale`） | store | `rate_scale` 顶层字段 + `graduation="0.125"` |
| `set_score` | 本地 `template_*/res.json` | `ProcessPaper.totalScore`（累加） | 展示/满分 |
| `is_mistake` / `mistake_support` | `m/mistake/check` / ecardInfo | — | 错题判定 |

**统计累加 set_score** — `index-5543d811.js`

```javascript
async processPaperResources(){let e=[];this.totalScore=0;for(let n=0;n<this.paperList.length;n++){const t=this.paperList[n],r=await this.getJson(replacePathSeparator(`${this.getPackageId()}\\template_${t.template.id}\\res.json`));this.totalScore+=Number(r.set_score);const o=lodashExports.cloneDeep(r),i=await Promise.all(o.exam_type_list.map(async e=>{let n=0;return e.exam_list=await Promise.all(e.exam_list.map(async r=>{const o=t.contents.find(e=>`{{${e.code_id}}}`===r.exam_id);if(o&&(r={...r,exam_id:replaceRealId(r.exam_id,o.real_id),...o}),"collector.repeat"===e.exam_type_collect){let e=function(e,t){const n=(e/t).toFixed(2),r=(e-(t-1)*n).toFixed(2);let o=[];for(let i=0;i<t-1;i++)o.push(n)
```
**setScore/detail 的权威来源（HomeworkList / job）** — `chunks/HomeworkList-f02d250d.js`

```javascript
=await r.getInstance().requestHomeworkStruct({id:parseInt(e.id),resource_id:e.resource_id,first_column_id:e.first_column_id});if(0==t?.code){if(Number(e.first_column_id)===u.PhoneticNew)return e.phoneticHomeworkStruct=t.body,e.phoneticCategoryName=await(async e=>{try{return await Dt(X(e))}catch(t){return""}})(t.body),Ot([],e),void(st.value=100);t.body.word_question&&t.body.word_question.length>0?(nt.value=t.body,Ot(nt.value?.word_question,e),st.value=100):t.body.dub&&t.body.dub.length>0?(ut.value=t.body.dub[0],Ot([],e)):(ot.value=t.body.combination,lt.value=t.body.composition,it.value=t.body.combination.length+t.body.composition.length,et.value=ot.value?.concat(lt.value))}},
```
## 2.5 语料：题型/评测类型/错题实体枚举
**exam-6ff42190.js（作答状态 + 考试接口类型）** — `chunks/exam-6ff42190.js`

```javascript
var n=(n=>(n[n.Unanswered=0]="Unanswered",n[n.Answered=1]="Answered",n[n.Correct=2]="Correct",n[n.Incorrect=3]="Incorrect",n))(n||{}),e=(n=>(n[n.Guangdong=1]="Guangdong",n[n.UTInterface=2]="UTInterface",n[n.HYTInterface=3]="HYTInterface",n[n.Shenzhen=4]="Shenzhen",n[n.Yunnan=5]="Yunnan",n))(e||{});export{n as A,e as P};

```
```text
// 作答状态
Unanswered=0, Answered=1, Correct=2, Incorrect=3
// 考试客户端类型（决定 Examine 用哪套皮肤/流程）
Guangdong=1, UTInterface=2, HYTInterface=3, Shenzhen=4, Yunnan=5
```
→ `Examine` 里 `const U = ecardInfo?.mock_mode_version ?? 1; const pe = I[U];`，`I = {1:Guangdong, 4:Shenzhen, 2:UTInterface, 3:HYTInterface, 5:Yunnan}`。

**MistakeListeningSpeakingEntityType（错题族实体）** — `index-5543d811.js`

```javascript
var MistakeListeningSpeakingEntityType=(e=>(e[e.listeningSimulated=1]="listeningSimulated",e[e.listeningSpecial=3]="listeningSpecial",e[e.textReading=4]="textReading",e[e.listeningSync=5]="listeningSync",e[e.phoneticStudy=6]="phoneticStudy",e[e.textShadowing=9]="textShadowing",e[e.listeningSimulated2=88]="listeningSimulated2",e[e.listeningSpecial2=89]="listeningSpecial2",e[e.unitListeningSpeaking=92]="unitListeningSpeaking",e[e.listeningSync2=104]="listeningSync2",e))(MistakeListeningSpeakingEntityType||{}),Mistake
```
## 2.6 接口清单（本流程出现过的全部路径）
| POST | `/m/audio/sync-v2` | 逐题成绩/作答同步（requestAnswerLogs，body 见 §2.2） |
| GET | `/m/set/get-score-detail` | 取分数明细（非作业练习；query 透传） |
| POST | `/m/set/reset-score-v2` | 重置成绩（requestResetScore，固定 version:"2"） |
| POST | `/m/set/column` | 专题列（requestSpecialColumn，{resource_id}）→ getColumnsNeed125 |
| GET | `g/set/column-v2` | requestExamMsg（自动补 hide_set_count:1） |
| POST | `/m/exam/struct` | 考试结构（requestExamStruct） |
| GET | `/m/exam/get-score-detail` | 听说考试分数明细 |
| POST | `/m/exam/sync` | 听说考试逐题同步（requestPostReadingWritingScore） |
| POST | `/m/exam/submit` | 听说考试交卷（requestSubmitReadingWritingExam） |
| POST | `/m/homework/info` | 作业信息（requestGetHomeworkInfo，version:2） |
| POST | `m/homework/struct` | 通用作业结构（requestHomeworkStruct） |
| POST | `g/homework/struct` | 词汇作业结构（requestHomeworkWordStruct） |
| POST | `g/homework/list` | 作业列表（requestHomeworkList，固定 version:10,use_https:1,use_homework_privilege:1） |
| POST | `g/homework/score-detail` | 词汇/通用作业分数明细（requestHomeworkScoreDetail） |
| POST | `g/homework/score-sync` | ★ 词汇作业最终成绩上报（requestHomeworkScoreSync） |
| POST | `g/homework/score-drop` | ★ 词汇作业成绩清空/重练（requestWordScoreDrop） |
| POST | `g/homework/report-score` | 音标新作业总分上报（requestPhoneticHomeworkReportScore，词汇族不用） |
| POST | `g/set/report-score` | 音标旧总分上报（requestPhoneticReportScore，词汇族不用） |
| POST | `g/homework/get-score` | 音标新作业取分 |
| POST | `g/set/get-score` | 音标旧取分 |
| POST | `v2/homework/struct` | 音标新作业结构 |
| POST | `m/homework/submit` | 通用作业交卷（requestHomeworkSubmit） |
| POST | `m/homework/set-use-time` | 用时回写（requestSetUseTime） |
| POST | `g/homework/checkstart` | 开始校验（requestCheckStart） |
| POST | `g/homework/pk-info` | PK 对手/成绩信息（requestGetHomeworkPKInfo） |
| POST | `g/homework/plan-list / g/homework/plan-detail / g/es/plan-excellent` | 学习计划 |
| POST | `g/pk/start-word-pk` | 发起单词 PK（requestStartWordPK） |
| POST | `g/pk/submit-word` | 单词 PK 交卷（requestPKSubmitWord） |
| POST | `g/pk/count` | PK 计数/排名人数 |
| POST | `g/pk/ranklist` | PK 榜单 |
| POST | `g/word/struct` | 词汇练习/PK 取词取题（requestWordStruct，{word_ids}） |
| POST | `g/word/score-report` | 词汇成绩上报（requestScoreReport）— 本地 82 个 chunk 无调用者 |
| POST | `g/word/add-new` | 加入生词本（requestWordAddNew） |
| POST | `g/word/is-in-notebook` | 生词本判定 |
| POST | `g/word/master` | 标记已掌握（requestWordMaster） |
| POST | `g/word/new-notebook` | 生词本新建 |
| POST | `g/set/word-list` | 单元单词表（requestWordDetailList；BC 变体加 source:1,unit_code） |
| POST | `g/set/material-res` | 素材资源 |
| POST | `g/set/ppr-res` | 逐题 PPR 资源（requestPPRRes，配合 mergePPRes 注入 question.pprInfo） |
| POST | `/g/set/mark-txt` | 开放题 mark.txt（requestMarkTxt） |
| POST | `g/data/audio-task-submit` | 录音任务提交（requestAudioTask，带自定义 callbackId） |
| POST | `m/homework/audio-task-retry / m/homework/audio-task-cancel` | 录音任务重试/取消 |
| POST | `m/file/pre-upload` | 文件预上传 |
| POST | `m/mistake/check` | 错题检查（mistakeAdder.check） |
| POST | `m/mistake/update` | 错题上报（mistakeAdder.update，type=String(MistakeType)，sync=mistake_id?"1":"0"） |
| POST | `m/mistake/detail / m/mistake/list / m/mistake/summary` | 错题详情/列表/统计 |
| POST | `m/score/detail` | 配音类分数明细（requestMScoreDetail） |
| GET | `/point-service/homework/rank/list` | 作业排行榜（requestHomeworkRankList） |
| POST | `g/resource/book-list / grade-list / login-list / expand-module` | 资源/教材 |
| POST | `/proxy/resourcePlatformInterface/pageItems` | 资源平台分页 |
| POST | `/proxy/resourcePlatformInterface/findByIds` | 资源平台按 id 取（音标） |
| POST | `/rop/open/v1/wordItem/listByWordIds` | 词汇作业资源平台直连（wordIds，100/批） |
| POST | `/rop/open/v2/word/findByIds` | 词汇作业资源平台直连（ids，100/批） |
| POST | `/config/config/getResourcePlatformToken` | 资源平台 token |
| POST | `system/time` | 服务器时间（show_score_answer 判定） |
| POST | `m/ecard/get / m/ecard/list` | 电子卡信息（rate_scale / mistake_support） |
| POST | `m/file/* , file/download` | STIC 侧文件下载/上传 |

## 2.7 提交时的题目标识速查（把三层 id 分开）
| 层级 | 主键 | 出自 |
|---|---|---|
| 套题（模板） | `set_id` + `super_set_id` | 路由 query / `g/set/list*` |
| 大题（exam_type 实体） | `entity_id` (= `exam.exam_id` / `info.stid`) | `ProcessPaper` / htmlData |
| 小题 | `order` (= `question_order` / `info.xh`) | 同上 |
| 词汇题（作业族） | `word_id` + `question_id` (+ `item_id`/`question_item_id`) | `word_question[]` / `pickBcIds` |
| 词汇题（练习/PK 族） | `word_id`(`struct.id`) + `question_id`(`question[].id`) | `g/word/struct` |
| 旧选择题族 | `entity_id` + `order`，选项用 `choose`(文本) + `choose_id`(序号) | content.json `xtlist/xxlist` |

---

# 第三部分：我方实现最可能不一致的 5 个点

### 1. 选项型答案的「键」用错（最致命）
- 官方：`option` 是 **JSON 字符串**，元素是 `{option_name, option_value}`；正确性判据是 **`answer == options[i].option_name`**；提交的 `answer` 也是 `option_name`。
- 文本在 `option_value`，**只显示**；下标只做 UI 高亮。
- 如果我方用「选项文本」或「下标 0/1/2/3」比对，会**全错但看起来正常**。
- 证据：`chunks/PK-7ae1b52b.js`（`me.value?.answer==e.option_name`、`title:e.option_value`、`ae.push(Ee(...,e.option_name))`）。
- 残留不确定：`option_name` 的字母表（`A/B/C` 还是 `0/1/2`）需要真题数据确认。

### 2. 三套「分数尺度」混淆
| 场景 | 尺度 | 代码 |
|---|---|---|
| PK/练习作答记录 | `point` = **10000**（正确）/ 0 | `Ee(me.value?.answer==e.option_name?1e4:0, ...)` |
| PK UI 计分 | **10/题** | `Qe.value+(...?10:0)` |
| 作业提交 | `score_rate` = **0/1 比率**，`score_detail.score` = **100/0** | `score_rate:e.point>0?1:0, score_detail:JSON.stringify({choose:e.answer, score:e.point>0?100:0})` |
| 展示 | 百分比（`ConvertToPercentScore` → 0~100）或绝对分（`scoreFormater`） | `useExamScore.scoreFilter` |

→ 若我方把 `point`（10000）直接当百分制提交，或被 `score_rate` 名字误导当成「分数率」写入分值，都会导致分数口径错误。

### 3. 最终上报接口选错
- 词汇作业 = **`g/homework/score-sync`**（`requestHomeworkWordCommit` 构造，`detail[]` + `mark_service:1` + `seq_id` + 可选 `source:1&unit_code`）；
- 词汇 PK（非作业）= `g/pk/submit-word`；
- `g/homework/report-score` / `g/set/report-score` **只属于音标流程**，词汇族完全不用。
- 且重练前必须先 `g/homework/score-drop`（`{resource_id, seq_id, homework_id}`）。
- 证据：`index-5543d811.js` 的 `requestHomeworkWordCommit` / `requestWordHomeworkSubmit` / `requestPhoneticHomeworkReportScore`。

### 4. 125 拉伸的触发条件与取整方式
- 必须**四个条件同时成立**（§2.4(3)），少一个都不能拉伸；
- 取整是 **向上取整到 0.125 的整数倍**（`0.125*Math.ceil(x/0.125)`），**不是**普通的 2 位四舍五入；
- 同时还要下发 `rate_scale` 与 `graduation:"0.125"`；
- 而单题 `real_score` 在 `useRecordResult` 里已经用 `calculateRealScore(qts, total)=scoreFormater(qts/5*total)` **四舍五入到 2 位**算过一次，sync 里可能被**再算一次覆盖** —— 两处口径不同。

### 5. `collector.word` 结果页协议与「多小题余数吸收」
- `collector.word` 在结果页与 `3q5a/role/dialogue/repeat` **共用 `set3q5aScore`**，格式是 `{set_id}_{entity_id}_{order}\x01{得分}\x01{满分}\x01{0}`；
- 同一 `entity_id` 有多个小题且每题满分 100 时，会按 `question_score/exam_type_score` 缩放，并让**最后一题吸收余数**使整组和恰好为 100（或 `exam_type_score`）；
- 如果我方按 `collector.choose`（`setPCChoose` + 字母→数字 `b()`）那套下发词汇题，iframe 端必然对不上。
- 证据：`chunks/useExamResult-d9fb4671.js` 的 `j()` 与 switch 分支。

---

## 附：本文所有证据文件清单
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/index-5543d811.js`（主 bundle，含所有 API 定义 / store / 分数函数 / 单例）
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/useExamStepper-8ad3fbd7.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/useExamResult-d9fb4671.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/MockExamDetails-2a7d03a6.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/Examine-d229829d.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/useAudioSync-0603b875.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/useRecordResult-fbe9e75b.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/PK-7ae1b52b.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/PKMatch-d8a7c872.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/ratingPage-27750263.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/wordCourseSourceHelpers-f63ae392.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/wordsPractice-857c490e.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/wordPractice-8d79e17b.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/WordList-0c83bc9d.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/WordPageItem-b11312ce.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/useWordCollect-3f40ea97.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/useWordResourceError-06c28489.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/HomeworkList-f02d250d.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/AbilityExpand-b7ff6ed6.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/exam-6ff42190.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/mistakeAdder-2da3b6d6.js`
- `/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/adjustWordColor-bccb26cb.js`

