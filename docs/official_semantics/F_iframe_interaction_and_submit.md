# F. 官方「E听说」PC 端 iframe 作答交互与提交/判分语义

> 只读静态分析。数据源：
> - iframe web 应用：`/home/shuaiqiu/Projects/ets源码恢复/iframe_web/`（`js/`、`*.html`）
> - 官方 PC 外层壳：`/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/`（`index-5543d811.js`、`chunks/*.js`）
> - 我方实现：`ets_resolver/src/collectors.rs`、`lastudio-backend/auto_complete/{runner.py,official_payload.py,scoring.py}`、`homework.py`
>
> 外层壳的 chunk 是**单行压缩**的，本文引用时统一标注 `文件 + 字节 offset`（offset 用 `str.find()` 定位，可复现）。
> 未运行客户端、未抓包、未联网。

---

## 0. 结论速览（先看这一段）

1. **题面上的那批“原生桥”不是原生插件桥。** `setPCChoose / setPCChoose2 / setPcBlank / set3q5aScore /
   showEssay_2 / showPhrase_2 / showScoreDetail / getChoice / ReturnChoose` 没有一个是 native plugin 导出；
   它们是**外层壳 Vue 组件在 iframe 的 `load` 事件里拿到 `contentWindow` 后直接挂载/直接调用的普通 JS 函数**
   （证据见 §2.0）。iframe 到外壳方向**只有三种**通道：
   `postMessage`（仅 `getWordTranslate` / `disabledUserSelect` / `wordTranslation`，见 `DynamicIframe-bafff5bc.js` offset 1000±）、
   **外壳直接读 iframe 全局函数返回值**、以及 **外壳把回调写进 iframe `window`**（`window.ReturnChoose` / `window.returnPcBlank` /
   `window.kttb_ReturnChoose` / `window.kttb_returnPcBlank` / `window.ck_ReturnChoose` → 外壳赋值给 `contentWindow` 上同名属性）。
2. **客观题（choose / fill）的判分发生在外层壳 JS 里**，用本地 `content_{real_id}.json` 的答案键比对，
   与 iframe、原生插件、服务端都无关；**语音题（read/3q5a/role/dialogue/word/repeat）的分数来自云端
   音频评测接口**（`g/data/audio-task-submit`），外层只做"清零/拒识/维度拉伸"等再归一。
3. iframe **内部没有任何判分逻辑**，它对 `user_answer == right_answer` 的比较只用于**上色**，不产生分数。
4. 逐题上报只有一个出口：`requestAnswerLogs` → `POST /m/audio/sync-v2`；
   逐题请求体由**三个不同的页面**各自拼装（Examine / MockExamDetails / Comprehensive+Practice），字段集合并不一致。
5. 我方实现有 2 处差异需要处置，见 §5 的 D1（**已于 19:59 由实现方修复**）与 D2（**实现方确认为"移动端口径、有意保留"**）。

---

## 一、各族作答交互流程

### 1.0 三个页面、三套桥（这是理解全部交互的钥匙）

同一个 iframe 页面 `pc-listen.html` 会在**三种入口**下被加载，桥的用法完全不同：

| 入口 | 外层 chunk | iframe html | 用到的桥 |
|---|---|---|---|
| 逐题练习/作业/听说专练（做题态） | `useExamStepper-8ad3fbd7.js` | `pc-listen.html` | `getChoice()`（返回值）/ `getPcBlank()`（回调 `returnPcBlank`）/ `setPCChoose2` |
| 错题重做·单题练习 | `Practice-1694d88a.js` | `pc-listen.html` | `kttb_getPcChoise()`→`window.kttb_ReturnChoose`；`kttb_getPcBlank()`→`window.kttb_returnPcBlank`；回显 `kttb_setPcChoise` / `kttb_setPcBlank` |
| 答案展示（对答案） | `useExamResult-d9fb4671.js` | `pc-listen-answer.html` | `setPCChoose` / `setPcBlank` / `set3q5aScore` / `showEssay_2` / `showPhrase_2` / `showScoreDetail` / `setUserrecord` |

iframe→外壳的“作答上交”只在**做题态**发生，且**语音题不经过 iframe**（见 1.2）。

`pc-listen.html` 的脚本装配（`iframe_web/pc-listen.html:397-413`）：

```html
<script src="js/jquery.min.js"></script>
<script src="js/disabled-user-select.js"></script>
	<script src="js/vue.min.js"></script>

	<script src="js/pc-listen-fn.js"></script>
	<script src="js/pc-listen-show-wordinfo.js"></script>
	<script src="js/pc-listen-choose.js"></script>

	<script src="js/pc-listen-data.js"></script>
	<script src="js/pc-listen-default.js"></script>
	<script src="js/pc-listen.js"></script>

	<script src="js/pc-view-kttb4.js"></script>

	<script src="js/pc-view-kttb1.js"></script>
```

`pc-listen-answer.html:669-676` 同样用 `pc-listen-fn.js`（**不是** `fn.js`）。
`js/fn.js` 只被 `phonogramMap.html` 加载（`grep -ln "js/fn.js" *.html` → `phonogramMap.html`），
它里面那套完整 `postMessage` 派发器（`fn.js:1109-1356`）在听说页面上**根本不会被用到**。
`js/method.js`、`js/PC-choose.js` **没有任何 html 引用**（死文件）。

### 1.1 collector.choose（选择题 / 听后选择）

**用户怎么作答**：点击 `li.choose1`（图片选项）或 `div.choose2`（文字选项）。
`pc-listen.html:165-206`：

```html
<div class="question1 listen-say" :id="set.choose_con.stid+'_'+j.xt_xh" v-if='orderFormat(j.xt_xh,0) && isChooseImg(j)'>
	...
	<li class="choose1" :id="set.choose_con.stid+'_'+j.xt_xh+'_'+c.xx_xh" v-for='c in j.xxlist'>
	...
	<div class="next_icon next_icon_choose fl">提交</div>
```

点击只加 class：`pc-view-kttb1.js:5-14`

```js
	$(document).on("click",".choose1:not('.choose_wrong,.choose_right')",function(e){
		$(this).addClass("choose_selected").siblings().removeClass("choose_selected");
	})
	$(document).on("click",".choose2:not('.choose_wrong,.choose_right')",function(e){
		var that = $(this);
		if (that.hasClass('choose_selected')) {
			return
		};
		that.addClass("choose_selected").siblings().removeClass("choose_selected");
	})
```

**作答存成什么形态**：不做任何中间存储，**dom 上的 class + id 就是唯一状态**。
元素 id 恒为 `"{stid}_{xt_xh}_{xx_xh}"`（`stid`=大题 entity_id、`xt_xh`=小题号、`xx_xh`=选项序号，均为官方结构里的原始值）。

**何时/如何交给外层**：两条互斥的路径。

- 路径 A（逐题练习/考试）：外层**主动调** `getChoice()`，iframe 把答案拼成逗号串，
  未作答的项把最后一段替换成 `0`。`pc-listen-choose.js:46-83`（**逐字**）：

```js
function getChoice(){
    var str = '';
    if ($(".choose1").length!==0) {
        $(".question1").each(function(index,item){
            var id1 = '';
            if ($(item).find('.choose1.choose_selected').length>0) {//与kkt1.js的选项点击事件冲突冲突原本代码为$(item).find('.choose1_selected') choose2也一样
                if($(item).find('.choose1.choose_selected').parent().attr('id')==undefined){//第二次修改代码逻辑图片还是需要用老逻辑
                    id1 = $(item).find('.choose1_selected').parent().attr('id');
                }else {
                    id1 = $(item).find('.choose1.choose_selected').parent().attr('id');
                }
            }else{
                id1 = $(item).find(".choose1").eq(0).attr("id");
                id1 = id1.substring(0,id1.length-1)+'0';
            }
            str+=id1+',';
        })
    }
    if ($(".choose2").length!==0) {
        $(".question2").each(function(index,item){
            var id2 = '';
            if($(item).find('.choose2.choose_selected').length>0){//原本代码为$(item).find(".choose2_selected")
                if($(item).find('.choose2.choose_selected').attr('id')==undefined){
                    id2 = $(item).find(".choose2_selected").attr('id');
                }else {
                    id2 = $(item).find('.choose2.choose_selected').attr('id');
                }
            }else{
                id2 = $(item).find(".choose2").eq(0).attr("id");
                id2 = id2.substring(0,id2.length-1)+'0';
            }
            str+=id2+',';
        })
    };
    var ret = str.substring(0,str.length-1);
    window.ReturnChoose(ret);
    return ret;
}
```

  **值形态**：`"115852_1_2,115852_2_0,121271_1_1"` —— 逗号分隔的 `entity_id_order_xx_xh`；未作答为 `entity_id_order_0`。
  同时**先调 `window.ReturnChoose(ret)` 再 return**（双通道：回调 or 返回值，外层两处用法各取一个）。
  外层用法（`useExamStepper-8ad3fbd7.js` offset 12499 / 9258，**逐字**）：

```js
practise:async()=>{clearTimeout(me.value),me.value=setTimeout(async()=>{if(y){
  if("collector.choose"===w.value.exam_type_collect){
    y.ReturnChoose=e=>{N.logCustom(`操作选择的answers:${JSON.stringify(e)}`,{tag:"FLOW_PRAC"})},await _();
    const t=y.getChoice(),
    a=await G(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);
    a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),...
```

- 路径 B（错题重做）：iframe 侧 `pc-view-kttb1.js:19-41` 把“提交”按钮绑到 `kttb_getPcChoise()`，
  它把结果交给 `window.kttb_ReturnChoose`（`pc-listen-fn.js:898-950`，**逐字**关键行）：

```js
// 获取选择题答案
function kttb_getPcChoise(){
    var ans_arr = [];
    if ($('.questions').length>0) {
        if($("li.choose1").length>0){
            $(".question1").each(function(index,item){
                var id = $(item).attr("id");
                var c = $(item).find('li.choose1.choose_selected');
                if (c.length==0) { ans = '' }else{ var ans = numtoa($(c).index()+1); }
                var o = { id: id, answer: ans };
                ans_arr.push(o);
            })
        }
        if ($(".choose2").length>0) {
            $(".question2").each(function(index,item){
                var id = $(item).attr("id");
                var c = $(item).find('.choose2.choose_selected');
                if (c.length==0) { ans = '' }else{ ans = numtoa($(c).index()); }
                var o = { id: id, answer: ans };
                ans_arr.push(o);
            })
        }
    }
    else{ ... }   // 判断题
    var ansStr = JSON.stringify(ans_arr);
    window.kttb_ReturnChoose(ansStr);
}
```

  **值形态**：JSON 字符串 `[{"id":"115852_1","answer":"A"}]`（`numtoa` 把 1/2/3 映射成 A/B/C，见 `pc-listen-fn.js:886-896`）。
  ⚠️ 事实观察（非推测）：`.choose2` 分支用的是 `numtoa($(c).index())`（**0 基**），
  而 `numtoa(0)` 第一步 `if(str)` 就为假，直接 `return false` ⇒ **第一个选项时 `answer` 是布尔 `false`**，
  下游 `i.answer===e.answer`（`Practice-1694d88a.js` offset 12422 的 `_t`）永远不成立。图片题（`.choose1`）用 `index()+1` 正常。

### 1.2 语音五族（collector.3q5a / role / dialogue / word / repeat）与朗读两族（collector.read / read_word、read_sentence）

**用户怎么作答**：在 **iframe 外部**的录音控件上录音。
证据：`pc-listen.html` 里 `grep -c "record" pc-listen.html` = **0**；`pc-listen-answer.html` = 17（那里的 record 是**回放**按钮）。
录音由外层步骤机驱动（`useExamStepper-8ad3fbd7.js` offset 15400±，**逐字**）：

```js
"action.record":async t=>{ue.value=!0;let a=Number(t.playtime??0);
  ... const o=B.cloneDeep(t);
  if(o&&o.question){const{pprInfo:t}=o.question||{},{engine_type:n}=t||{},
    l={record_time:a,category:o.question.evaluation_type,res_id:o.question.res_id,
       region:o.question?.engine_area,recordStep:o,set_id:e,engine_type:n,pprInfo:t,first_column_id:g};
  ... const s=Z.record(l,{onReady:()=>{...},onReadyLoading:()=>{...}}),
    r=()=>{const e={entity_id:o.exam.exam_id,order:o.question.question_order,
                    first_column_id:g,question_type_score:o.question.question_score};
      return new Promise((t,a)=>{s.then(async a=>{L.trigger(async()=>{
        const t=await re(l.category,a,g,o.question.question_score,e=>e&&se.addTask(e));
        return x.trigger({...t,...e}),t}),t(a)}).catch(t=>{a({...t,logParams:e,recordStep:o})})})};
  F.trigger(r)}}
```

**作答存成什么形态**：**音频文件**（`wav`）+ 引擎返回的**评分 XML**，再被 `useRecordResult-fbe9e75b.js`
归一成一个 `score_detail` 对象。没有“文本答案”参与。

**何时/如何交给外层**：录音结束 → 云端评测 → `answerHandler`（`x.trigger`）→ 向外层 `Qa/Ea/ct` 流动。
iframe 只在**展示**时被写入用户录音地址：

```js
/*设置用户回放录音*/
function setUserrecord(){
    var x = String.fromCharCode(0x01);
    $(arguments).each(function(index,item){
        var arr = item.split(x);
        $("#"+arr[0]).siblings("audio").attr(arr[1]);  
    })
}
```
（`pc-listen-fn.js:757-764`）。注意：`setUserrecord` 在**整个外层壳里搜不到调用点**（检索关键词见 §附录 A）→ 未找到。

**答案展示**：`useExamResult-d9fb4671.js` offset 3569-4650（单大题朗读）与 3200（多小题）逐字：

```js
case"collector.3q5a":case"collector.role":case"collector.dialogue":case"collector.word":case"collector.repeat":
  I()||setTimeout(()=>{const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0);break;
case"collector.read":n.examType.exam_list.length>1?I()||setTimeout(()=>{c.showScore(_,d,0);
  const t=j(n.examType,n.htmlData,e,i);c.set3q5aScore.apply(null,t)},0):function(e,t,o,i,r){setTimeout(async()=>{
  const{htmlData:s,hasLogScore:n,set_module_id:c}=t,l=t.exam_type_score??5,d=s[0];
  if(d){const s=o.find(e=>e.entity_id===d.info.stid)||{},
    n=await x({set_id:D,entity_id:s.entity_id,order:s.order,mistake_id:t.mistake_id,homework_id:t.homework_id,remote_url:s.detail_file});
  let c="";c=n&&!n.includes("http")?await C(n):await p(s.detail_file);
  const{real_score:_,detail:u,standard_score:m,manual_modified:f}=s,h=y(u),
    g=t.examType.exam_list.find(e=>e.exam_id===d.info.stid);
  let w=d.info.value;-1===w.indexOf("<\br>")&&(w="</br>"+w.replace(/<\/p>/g,"</p></br>"));
  const{scoreDetail:k,essayData:S}=await $(w,c,{first_column_id:i,stu_total:m,stu_real_score:m,
    multidimensional_results:h,exam_type_score:l,manual_modified:!0===f}),
    b="number"==typeof _?_:k.stu_real_score,{scoreFilter:q}=a(),[T,v]=q(l,b,r);
  if(I()||e.showScore(v,T,0),g&&g.question_list){const t=g.question_list[0];
    if(!I()&&("read_chapter"===t.evaluation_type?e.showEssay_2.apply(null,S)
      :"read_sentence"===t.evaluation_type&&e.showPhrase_2.apply(null,S),"read_word"!==t.evaluation_type)){
      const t=k?.dimension_result||[];e.showScoreDetail.apply(null,t.slice(0,3))}}}},0)}(c,n,e,o,i);break;
```

要点（**evidence，非推断**）：
- `showEssay_2` 只给 `read_chapter`；`showPhrase_2` 只给 `read_sentence`；`read_word` **两者都不给**，只给 `showScoreDetail`。
- `showScoreDetail` 的入参是 `dimension_result.slice(0,3)` —— 官方 `dimension_result = [accuracy, fluency, integrity, standard]`（见 §4.4）。
- `structure_type` 与 `evaluation_type` **不是一回事**：`collector.read` 大题下的题目 `evaluation_type` 可能是
  `read_chapter` / `read_sentence` / `read_word` 三者之一。

**答案展示用的入参形态**（`adjustWordColor-bccb26cb.js` 的 `J()`，**逐字**）：

```js
function J(e){e.findIndex(e=>!!e.content);const r=[];for(let t=0,o=e.length;t<o;t++){let{is_role:o=!1,is_br:s=!1,content:n="",score:c,AudioUrl_EN:a="",AudioUrl_US:l="",Spell_EN:i="",Spell_US:_="",Trans:d="",score_class:u="color-bad",dp_message:g=0}=e[t];if(s){r.push("<br>");continue}c=R(u),o?c=-20:16==g?c=200:g>16&&(c=0);const p=[n,i,_,a,l,c,100,d];r.push(p.join(f))}return r}
function R(e){return"color-bad"==e?0:"color-good"==e?81:61}
```
其中 `f=String.fromCharCode(1)`。即每个元素是 8 段 `\x01` 拼接：
`词 | 英标 | 美标 | 英音url | 美音url | 分 | 100 | 译文`，与 iframe 侧 `arr[0..6]` 完全对应（`pc-listen-fn.js:88-98`）。
`score` 取值域：`color-bad→0`、`color-good→81`、`color-mid→61`、角色句 `→-20`、`dp_message==16→200`、`dp_message>16→0`。

### 1.3 collector.fill（填空题 / 听后记录）

**作答**：在 `input` 里打字，id 为 `stid_xth`（`pc-listen-answer.html:407`
`:id='one.info.stid+"_"+i.xth'`；练习页 `pc-listen.html:300-310` 为 `set.content_fill_answer*` 系列）。

**什么时候交**：点“提交”（`pc-view-kttb1.js` / `pc-view-kttb3.js` / `pc-view-kttb4.js` 把
`.next_icon_gap`、`.next_icon` 绑到 `kttb_getPcBlank()`）；逐题练习态则由外层主动调 `getPcBlank()`。

**值形态**（`pc-listen-fn.js:796-809`，**逐字**）：

```js
// 获取多个填空题答案
function getPcBlank(){
    var ans_arr = [];
    $('.fill input').each(function(index,item){
        var id = $(item).attr('id').trim();
        var ans = $(item).val().trim();
        var o = {};
        o.id = id;
        o.answer = ans;
        ans_arr.push(o);
    })
    var ansStr = JSON.stringify(ans_arr);
    window.returnPcBlank(ansStr);
}
```

即 **JSON 字符串**：`[{"id":"115862_1","answer":"middle"}, ...]`。
注意：练习态的 `getPcBlank` **把结果交给 `window.returnPcBlank` 回调、且不 return**（与 choose 的双通道不同）。
kttb 版（`pc-listen-fn.js:836-858`）多做了 `encodeURIComponent`：

```js
function kttb_getPcBlank(){
    ...
        var ans = encodeURIComponent($('.write_input').val().trim());
    ...
    var ansStr = JSON.stringify(ans_arr);
    window.kttb_returnPcBlank(ansStr);
}
```

**回显**（外层→iframe）：`setPcBlank(str)`（`pc-listen-fn.js:811-822`，**逐字**）：

```js
function setPcBlank(str){
    $('input').attr('disabled','disabled').css('border-color','#aaa');
    var ans_arr = JSON.parse(str);
    $(ans_arr).each(function(index,item){
        $('#'+item.id).val(item.answer);
        if (item.status == '0') {
            $('#'+item.id).css('color','#e65151');
        }else if(item.status == '1'){
            $('#'+item.id).css('color','#43cb75');
        }
    })
}
```
即 `str = JSON.stringify([{id, answer, status}])`，`status`：`'0'` 错、`'1'` 对（字符串比较）。
kttb 版 `kttb_setPcBlank`（`pc-listen-fn.js:860-885`）还会 `decodeURIComponent(item.answer)` 并打
`fill_word_wrong / fill_word_right`，且 `showOrHideAnalysis()` 后 `$('.next_icon').hide()`。

### 1.4 collector.picture（故事复述）与 collector.word 的“朗读单词”形态

- `collector.picture`：无选项、无填空；作答同样是外壳录音。答案页 `pc-listen-answer.html:218-300` 只渲染
  `one.info.topic / value / std[]`，无任何交互桥 —— **未找到**该族专属桥（检索关键词见附录 A）。
- `collector.word`：`pc-listen-answer.html:561-642`，同族展示；逐小题分数走 `set3q5aScore`（见 1.5）。

### 1.5 `set3q5aScore` 的入参是怎么算出来的

`useExamResult-d9fb4671.js` offset 928-1574（**逐字**）：

```js
function j(e,t,o,i){return t.reduce((t,r)=>{let s=o.filter(e=>e.entity_id===r.info.stid),n=0;const c=s.length,l=s.reduce((t,o)=>{n++;const r=function(e,t){const o=e.exam_list.find(e=>e.exam_id===t.entity_id);return o?o.question_list.find(e=>e.question_order===String(t.order)):{}}(e,o),{scoreFilter:s}=a(),[l,d]=s(r.question_score,o.real_score,i);let _=l,u=d;if(100==u&&c>1){let t=Number(r.question_score)/Number(e.exam_type_score);if(_=O(l,t,i,c),u=O(d,t,i,c),n==c){const t=u;u=(i?e.exam_type_score:100)-u*(c-1),_=_*u/t,_=h(_,i)}}const m=`${D}_${o.entity_id}_${o.order}${q}${_??0}${q}${u}${q}0`;return t.push(m),t},[]);return t=t.concat(l)},[])}
```

- 每个参数 = `"{set_id}_{entity_id}_{order}\x01{score}\x01{totalScore}\x01""0"`（第 4 段**恒为 `"0"`**）。
- `D` = 本题的 `set_id`（`html-8e3719a4.js` 的 `PcListen/PcListenAnswer` 文件名常量旁的 `set_id` 变量，见 §4.1 的 `const D=D`）。
- `O` 与 `scoreFilter`（`useExamResult` offset 700± / `index-5543d811.js` offset 854783，**逐字**）：

```js
function O(e,t,o,i){return t=t||1,o?e:h(e*t,i>1?2:0)}
```
```js
const useExamScore=()=>{const e=[FirstColumnType.SynchronousListening,FirstColumnType.ReadingWriting,
  FirstColumnType.ListeningSpeakingSpecial,FirstColumnType.SpecialTraining,FirstColumnType.SimulationExam,
  FirstColumnType.ListeningSimulation,FirstColumnType.ReadingWritingSimulation];
  function t(t,n){return t&&e.every(e=>e!=t)||n&&5!=n}
  return{scoreFilter:function(e,t,n){return[n?scoreFormater(t):ConvertToPercentScore(t,e,0),n?e:100]},
         scoreFilterHomework:function(e,n,r,o,i){return[t(r,o)?scoreFormater(n):ConvertToPercentScore(n,e,0),i?e:100]},
         percentageQuestion:t}};
```
即：作业口径 → `[scoreFormater(实得分), 该题满分]`；非作业 → `[round(实得/满分*100,0), 100]`。

### 1.6 iframe 侧的“消息派发器”只有两个 case

`pc-listen-fn.js:1109-1123`（**逐字**，这是听说页面真实的派发器）：

```js
$(window).on('message', function(e){ 
    var originalEventData = e && e.originalEvent && e.originalEvent.data ? e.originalEvent.data : {};
    var type = originalEventData.type || '';
    var data = originalEventData.data || '';
    switch( type ){        
        case 'canListen':
            canListen();
            break;
        case 'setChallengeStar':
            setChallengeStar();
            break;        
        default:
            break;
    }
})
```

结论：`showEssay_2 / showPhrase_2 / showScoreDetail / set3q5aScore / setPcBlank / setPCChoose` 等
**不可能**通过 `postMessage` 到达（`fn.js` 里那套 50+ case 的派发器只服务于 `phonogramMap.html`）。

### 1.7 唯一的 postMessage 桥接页：`phonogramMap.html`（音标练习）
`phonogramMap.html:313-315` 加载 `js/fn.js`；调用方是 `PhoneticPracticeView-a54655d6.js`（**逐字**）：

```js
Y.postMessage({type:"showPhrase_OldStruct",data:t},"*"),
Y.postMessage({type:"showScoreDetail_1",data:Object.values({accuracyScore:g(a.score_detail.accuracy_score,5,0),fluencyScore:g(a.score_detail.fluency_score,5,0),integrityScore:g(a.score_detail.integrity_score,5,0)})},"*")
```
```js
postMessage({type:"setZoom",data:J.getZoomSize...
```
信封 = `{type, data}`，派发器把 `data` **展开成参数**（`fn.js:1198-1200`、`:1222-1223`）：
```js
        case 'showPhrase_OldStruct':
            showPhrase_OldStruct.apply(null, data);
            break; 
        ...
        case 'showScoreDetail_1':
            showScoreDetail_1.apply(null, data);
            break;
```
这是与本任务 9 族**无关**的另一条链（音标地图），列在此处是为了说明「为什么 `fn.js` 存在却用不上」。

---

## 二、桥函数的调用契约

### 2.0 通道机制（先确定“由谁调用”）

`chunks/DynamicIframe-bafff5bc.js` offset 310 / 865（**逐字**）：

```js
props:{src:{type:String,default:""},isWriting:{type:Boolean,default:!1},sandbox:{type:String,default:"allow-scripts"}}
...
async function I(){const s=x.value;M("load",s.contentWindow),await u(), ...}
```

- `sandbox` 默认 `"allow-scripts"`，**外层调用点没有覆盖过它**（`grep -o "sandbox[^,}]\{0,60\}" chunks/*.js` 只有 DynamicIframe 自身）。
- `M("load", s.contentWindow)` 把 `contentWindow` 抛给上层；`useExamStepper` 里 `O.on(t=>{const a=t.win;y=a;...})` 接住并存到 `y`。
- 之后所有 `y.setXXX(...)`、`y.getChoice()`、`y.ReturnChoose=...` 都是**跨 window 直接属性访问**。
  这在“跨源隔离”语义下本应抛 `SecurityError`（sandbox 无 `allow-same-origin`）——**静态分析看不到 Electron `webSecurity` 配置**
  （在 `pc_client_web/*.js` 与 `chunks/*.js` 里 grep `webSecurity|nodeIntegration` 无命中，检索关键词见附录 A），
  故“为何能跨源访问”**未确认**；但调用点代码本身是确凿的。

### 2.1 `getChoice()`
| 项 | 内容 |
|---|---|
| 定义 | `iframe_web/js/pc-listen-choose.js:46-83`（另有旧版 `js/PC-choose.js:20-49`，**未被任何 html 加载**） |
| 调用者 | `useExamStepper-8ad3fbd7.js` offset 12499（`const t=y.getChoice()`）、offset 9349 附近的答案回填分支 |
| 入参 | 无 |
| 返回 | `string`，逗号分隔 `entity_id_order_xx_xh`，未作答尾巴为 `0` |
| 副作用 | 调用前置 `window.ReturnChoose(ret)` |

外层把回调设成“只打日志”，说明**外层只吃返回值**：

```js
y.ReturnChoose=e=>{N.logCustom(`操作选择的answers:${JSON.stringify(e)}`,{tag:"FLOW_PRAC"})},await _();const t=y.getChoice(),
```
（`useExamStepper` offset 12499；另一处答案页分支是 `y.ReturnChoose=()=>{}`，offset 9319。）

### 2.2 `ReturnChoose`（**无 iframe 内实现，纯回调位**）
- iframe 只做 `window.ReturnChoose(ret)`（`pc-listen-choose.js:81`、`PC-choose.js:47`）。
- 定义者=外层：`useExamStepper` offset 12499（写日志）与 offset 9319（`()=>{}`）。
- 返回/回调：无返回值语义（外层不 return）。

### 2.3 `setPCChoose(...)`
| 项 | 内容 |
|---|---|
| 定义 | `iframe_web/js/pc-listen-choose.js:96-140` |
| 调用者 | **只有** `useExamResult-d9fb4671.js` offset 4748 的 `case"collector.choose"`（答案展示态） |
| 入参 | 变长参数，每个 = `"{entity_id}_{order}_{学生选项序号}\x01{正确选项序号}"`；**未作答**时 = `"{entity_id}_{order}\x01{正确选项序号}"` |
| 返回 | 无（DOM 上色 + `未作答` 红字） |

入参构造（`useExamResult` offset 4748-5180，**逐字**）：

```js
case"collector.choose":setTimeout(()=>{const t=(o=n.htmlData,i=e,o.reduce((e,t)=>{const o=i.filter(e=>e.entity_id===t.info.stid).reduce((e,o)=>{const i=b(t.info.xtlist.find(e=>e.xt_xh===""+o.order)?.answer);return o.detail.choose?e.push(`${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}`):e.push(`${o.entity_id}_${o.order}${q}${i}`),e},[]);return e.concat(o)},[]));var o,i;c.setPCChoose.apply(null,t)},0);break;
```

`b()` 的定义（`useExamResult` offset 367，**逐字**）：

```js
function b(e){if(!e)return"0";for(var t=e.toUpperCase(),o="",i=0;i<t.length;i++){var r=t.charCodeAt(i);if(r>=65&&r<=90)o+=(r-64).toString();else o+=t[i]}return o}
```

⇒ `b("A")="1"`、`b("B")="2"`：**字母被转成 1 基序号**，正好等于 `xx_xh`（真实模板 `xx_mc` 恒为 A/B/C…且 `xx_xh` 为 1/2/3…）。
iframe 端用 id 尾段反解学生答案并**只用于上色**：

```js
var xIndex = id.lastIndexOf('_')+1;
var user_answer = id.substr(xIndex);
if ($("#"+id).hasClass('choose2')) {
    if (user_answer == right_answer) {
        $("#"+id).siblings('.choose_title').addClass('c_green');
    }else{
        $("#"+id).siblings('.choose_title').addClass('c_red');
    };
```

### 2.4 `setPCChoose2(...)` —— **未找到实现**
- **唯一命中**：`useExamStepper-8ad3fbd7.js` offset 9258（调用点，**逐字**）：

```js
me.value=setTimeout(()=>{if(1==t.displayAnswerContent&&y.setPCChoose2){const e=o.map(e=>`${e.entity_id}_${e.order}_${e.choose_id}`);y.setPCChoose2.apply(null,e),r()}},500)
```
- 入参形态：`"{entity_id}_{order}_{choose_id}"` 数组（**只有 3 段，无 `\x01`，无正确答案**）。
- 返回/回调：无。
- **定义处**：在 `iframe_web/**` 与 `pc_client_web/**` 全量 grep `setPCChoose2` 后**只有上面这一处**，
  即：**调用点存在、实现找不到**（且调用前有 `&& y.setPCChoose2` 守卫）。检索关键词：`setPCChoose2`、`setPcChoose2`、`PCChoose2`。
- 影响：答案展示态下这条分支**静默不执行**；实际回填靠紧随其后的 `r()`（`getChoice` + `G`）。

### 2.5 `setPcBlank(str)`
| 项 | 内容 |
|---|---|
| 定义 | `iframe_web/js/pc-listen-fn.js:811-822` |
| 调用者 | `useExamResult-d9fb4671.js` offset 5187（`case"collector.fill"`，答案展示态） |
| 入参 | **字符串**（不是数组）：`JSON.stringify([{id, answer, status}])`，`status` 为 `'0'/'1'` 字符串 |
| 返回 | 无 |

入参构造（**逐字**）：

```js
case"collector.fill":setTimeout(()=>{const t=(o=n.htmlData,i=e,o.reduce((e,t)=>{const o=i.filter(e=>e.entity_id===t.info.stid).reduce((e,t)=>(t.detail.choose?e.push({id:`${t.entity_id}_${t.order}`,answer:t.detail.choose,status:((t.real_score||t.score_rate)??0)>0?1:0}):e.push({id:`${t.entity_id}_${t.order}`,answer:"",status:0}),e),[]);return e=e.concat(o),JSON.stringify(e)},[]));var o,i;c.setPcBlank(t)},0)
```

### 2.6 `getPcBlank()` / `returnPcBlank`
- 定义：`pc-listen-fn.js:797-809`；调用者 = `useExamStepper` offset 14229（`y.getPcBlank()`）。
- 入参：无。返回：无（结果走 `window.returnPcBlank(ansStr)`）。
- 回调位由外层赋值（`useExamStepper` offset 13118，**逐字**）：

```js
"collector.fill"===w.value.exam_type_collect&&(y.returnPcBlank=async t=>{const a=await async function(t,a,o,n){...}(w.value.super_set_id||w.value.set_id,_e.exam.exam_id,_e.exam.question_list[0],t);a.forEach(e=>{e.first_column_id=g,e.question_type_score=_e.exam.exam_score,x.trigger(e)}),...,y.getPcBlank())
```

### 2.7 `set3q5aScore(...)`
| 项 | 内容 |
|---|---|
| 定义 | `iframe_web/js/pc-listen-fn.js:776-794`（`fn.js:832-850` 同文） |
| 调用者 | `useExamResult-d9fb4671.js` offset 3200（5 族共用） |
| 入参 | 变长，每个 = `"{set_id}_{entity_id}_{order}\x01{分}\x01{满分}\x01\"0\""` |
| 返回 | 无（写 `.qa-u-score` / `.qa-t-score` 文本与颜色） |

定义原文（**逐字**）：

```js
/*3q5a设置分数*/
function set3q5aScore(){
    $(arguments).each(function(index,item){
        var x = String.fromCharCode(0x01);
        var arr = item.split(x);
        var id = arr[0],
            score = arr[1],
            all_score = arr[2],
            statu = arr[3];
        var color = '';
        var rate = score/all_score;
        if (statu=='0') {
            color = rate<0.6&&rate>=0?'f54b4b':(rate<0.8&&rate>=0.6?'c88d31':'01cf6b');
        }else if(statu=='1'){
            color = rate<0.6&&rate>=0?'f54b4b':(rate<0.8&&rate>=0.6?'c88d31':'01cf6b');  
        };
        $("#"+id).parent().siblings('.qa-score').find('.qa-u-score').text(score+'分').css('color','#'+color);
        $("#"+id).parent().siblings('.qa-score').find('.qa-t-score').text(all_score+'分');
    })
}
```

DOM 侧 id（`pc-listen-answer.html:130`）：`:id="set_id+'_'+one.info.stid+'_'+i.xh"`，元素是 `.img-span > img`，
`.parent()`=`.img-span`，`.siblings('.qa-score')` 命中得分 span —— id 三段结构得到确认。

### 2.8 `showPhrase_2(...)` / `showEssay_2(...)`
| 项 | 内容 |
|---|---|
| 定义 | `pc-listen-fn.js:85-148`（showPhrase_2）、`:269-343`（showEssay_2） |
| 调用者 | `useExamResult-d9fb4671.js` offset 4520 附近（**只有答案展示态**） |
| 入参 | 变长字符串，元素 = 8 段 `\x01`；另允许原样字符串 `"<br>"` / `"<tab>"`（仅 showEssay_2 判这两个） |
| 返回 | 无（渲染进 `.word_txt` / `.article_txt`） |

段序（`pc-listen-fn.js:89-98`）：

```js
        var _word = arr[0],
            _en = arr[1],
            _us = arr[2],

            _en_audio = arr[3],
            _us_audio = arr[4],

            _score = arr[5],
            _t_score = arr[6];
```

`showEssay_2` 比 `showPhrase_2` 多两处差异（**逐字**）：

```js
function showEssay_2() {
    ...
        if (item == "<tab>") { var _tab = $("<span>").addClass("_tab"); span.append(_tab);
        } else if (item == "<br>") { var _br = $("</br>"); span.append(_br);
        } else { ... }
    ...
            if (num == -20) {
                //角色
                color = '#979797';
            } else if (radio >= 0 && radio < 0.6) {
```

（`showPhrase_2` 没有 `<tab>/<br>` 分支、也没有 `-20` 灰色角色分支；`pc-listen-fn.js:85-148` vs `:269-343`。注意 **`pc-listen-fn.js` 版的 `showEssay_2` 有这个 `num == -20` 分支，而 `fn.js:390-461` 版没有**。）

### 2.9 `showScoreDetail(...)`
| 项 | 内容 |
|---|---|
| 定义 | `pc-listen-fn.js:406-482`（另有 `showScoreDetail_1` = `:484-563`） |
| 调用者 | `showScoreDetail` → `useExamResult-d9fb4671.js`（**直接调用** `e.showScoreDetail.apply(null,t.slice(0,3))`）；`showScoreDetail_1` → **`PhoneticPracticeView-a54655d6.js`（音标练习，走 postMessage）**，见 §1.7 |
| 入参 | 位置语义固定：`index0=准确度`、`index1=流畅度`、`index2=完整度`、`index3=标准度` |
| 返回 | 无 |

```js
    $(arguments).each(function(index, item) {
        if (index == 2) {
            dim1 += '<span class="score_detail_item">完整度：' + item + '</span>';
```

### 2.10 `kttb_*` / `ck_ReturnChoose` / `webInteraction`
| 函数 | iframe 定义 | 外层定义 | 备注 |
|---|---|---|---|
| `kttb_getPcChoise()` | `pc-listen-fn.js:898-950` | — | 交出到 `window.kttb_ReturnChoose` |
| `kttb_ReturnChoose` | **无实现**（只被调） | `Practice-1694d88a.js` offset 13568 处 `dt.kttb_ReturnChoose=T(async e=>{...})` | 错题重做选择族主通道 |
| `kttb_getPcBlank()` | `pc-listen-fn.js:836-858` | — | 交出到 `window.kttb_returnPcBlank` |
| `kttb_returnPcBlank` | **无实现** | `Practice-1694d88a.js` offset 13646 处 | |
| `kttb_setPcChoise(str)` / `kttb_setPcBlank(str)` | `pc-listen-fn.js:952-1013` / `:860-885` | 由 `Practice` 调（offset 13568/13646 的 `vt`/`pt`） | 回显判对错 |
| `ck_ReturnChoose` | **无实现**（`pc-view-kttb5.js:20` 只被调） | 全量 grep 无命中 | **未找到** |
| `webInteraction.getAnswerList` | **无实现**（`method.js:56` 只被调） | 全量 grep 无命中 | 旧 Android WebView 注入对象名；`method.js` 自身也**无人加载** |
| `getChooseChange(cb)` | **无实现** | 只被调：`useExamStepper` offset 9564 `a.getChooseChange(()=>{...})` | **未找到** |

---

## 三、判分发生的位置

### 3.1 choose / fill：**外层壳本地判**（不是 iframe、不是原生、不是服务端）
`useExamStepper-8ad3fbd7.js` offset 2046（**逐字**）：

```js
async function G(e,t,a,o){const{getFileJson:n}=f(),{UNZIP_LOCAL_PATH:l}=d();let s=[],r=(await n(p(`${l}\\${e}\\content_${t}\\content.json`))).info.xtlist;try{s=o.split(",")}catch(i){}return s.map(e=>{const[t,o,n]=e.split("_");let l="0";if("0"===n)return{entity_id:t,order:o,score_detail:{total_score:l,choose:"",category:a.evaluation_type}};{const e=r.find(e=>e.xt_xh===o);if(!e)return null;const s=e.xxlist.find(e=>e.xx_xh==n);return s?(e.answer===s.xx_mc&&(l=m(a.question_score/a.question_count)),{entity_id:t,order:o,score_detail:{total_score:l,choose:s.xx_mc,choose_id:n,category:a.evaluation_type}}):null}}).filter(Boolean)}
```

- **比较表达式原文**：`e.answer===s.xx_mc`（`e` = `xtlist[]` 项，`answer` 是正确选项字母；`s` = `xxlist[]` 项，`xx_mc` 是选项字母）。
- **分值**：`l = m(a.question_score/a.question_count)`，`m = d3 = scoreFormater`（`useExamStepper` 头部 `d3 as m`）；
  未作答 (`n==="0"`) → `total_score: "0"`、`choose: ""`、**不带 `choose_id`**。
- **答案键来源**：本地已解压的 `content_{real_id}.json`（`getFileJson`），**不是服务端**。

fill 的本地判分（同一文件 offset 13118 的内联 IIFE，**逐字**）：

```js
const a=await async function(t,a,o,n){const{getFileJson:l}=f(),{UNZIP_LOCAL_PATH:s}=d(),r=(await l(p(`${s}\\${t}\\content_${a}\\content.json`))).info.std,i=JSON.parse(n);try{return i.map(e=>{const[t,a]=e.id.split("_"),n=e.answer;let l="0";if(""===n)return{entity_id:t,order:a,score_detail:{total_score:l,choose:"",category:o.evaluation_type}};{const e=r.find(e=>e.xth===a);return s=X(n),X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(s)&&(l=m(o.question_score/o.question_count)),{entity_id:t,order:a,score_detail:{total_score:l,choose:n,category:o.evaluation_type}}}var s})}catch(e){return[]}}(...)
```

- **比较表达式原文**：`X(e.value).split("/").map(e=>e.trim().toLowerCase()).includes(X(用户输入))`
- 归一化 `X()`（同文件 offset 1901，**逐字**）：

```js
function X(e){const t={"‘":"'","’":"'","：":":","，":",","。":".","——":"-","—":"-",_:"-"};return(e=e.replace(/[‘’：，。———_]/g,e=>t[e])).toLowerCase()}
```
- fill **不带 `choose_id`**；`choose` 放**用户原始输入**（未归一化）。

错题重做页判分（`Practice-1694d88a.js` offset 12422，**逐字**）：

```js
t.push({entity_id:n,order:o,score_detail:{total_score:i.answer===e.answer?c.question_score/c.question_count:0,choose:e.answer,category:c.evaluation_type}})
```
`mt`（填空，offset 13010）：
```js
const i=a.contents.find(e=>e.info.stid===n).info.std.find(e=>e.th===l[1]).value.split("/").map(e=>encodeURIComponent(e)),...,u=i.includes(e.answer)?1:0;
t.push({entity_id:n,order:o,score_detail:{total_score:1==u?r.question_score/r.question_count:0,choose:e.answer,category:r.evaluation_type}})
```
⚠️ 该页两条路径都**不发 `choose_id`** —— 说明 `choose_id` 并非所有客观题上报都必有。

### 3.2 语音六族：**服务端（云 ISE）判**
- 录音 → 音频任务接口（`index-5543d811.js` offset 2360136 附近，**逐字**）：

```js
requestAudioTask=async(e,t)=>await hyRequest.postIt("g/data/audio-task-submit",e,!0,t),requestAudioTaskRetry=async e=>await hyRequest.postIt("m/homework/audio-task-retry",e),requestAudioTaskCancel=async e=>await hyRequest.postIt("m/homework/audio-task-cancel",e)
```
- 评测结果（含 `data.xml`）回到前端后，由 `useRecordResult-fbe9e75b.js` 解析/归一（**逐字**摘录核心）：

```js
const I={...R,total_score:_(w),total_score_init:w,real_score:G,category:e,accuracy_score:$,fluency_score:b,standard_score:O,first_column_id:p,question_type_score:h};
```
其中 `G = calculateRealScore(question_type_score, total_score)`，`calculateRealScore(e,t)=scoreFormater(e/5*t)`（`index-5543d811.js` offset 845549，**逐字**）：

```js
function scoreFormater(e,t=2){let n=Number(e)*Math.pow(10,t);return Math.round(Number(n))/Math.pow(10,t)}
function scoreFormatter_125(e,t=2){return scoreFormater(.125*Math.ceil(Number(e)/.125))}
function ConvertToPercentScore(e,t,n=0){return e&&t?scoreFormater(Number(e)/Number(t)*100,n):0}
function calculateRealScore(e,t){return scoreFormater(Number(e)/5*Number(t))}
```

- **客户端零分规则**（`useRecordResult` 头部，**逐字**）：

```js
function(e,t,o){const a=String(e?.engine_code??"");if(a&&u.includes(a))return{shouldZero:!0,ignoreReject:!1,hitRule:1};const c=Number(t?.except_info??0),l=1===Number(t?.is_rejected??0);if(c===r.IgnorableException)return{shouldZero:!("1"===String(o?.ignore_28707_error??"")),ignoreReject:!1,hitRule:2};if(l||d.includes(c)){const e=parseFloat(String(o?.pc_accept_reject_score??"0"))||0;return(parseFloat(String(t?.total_score??0))||0)<=e?{shouldZero:!0,ignoreReject:!1,hitRule:3}:{shouldZero:!1,ignoreReject:!0,hitRule:3}}return{shouldZero:!1,ignoreReject:!1,hitRule:0}}
S&&(R={...R,total_score:0,xml_score:0,accuracy_score:0,fluency_score:0,standard_score:0,integrity_score:0})
```

- **维度分拉伸**（`adjustWordColor-bccb26cb.js`，`mutil_dimension_adjust` 开关，**逐字**）：

```js
if(1==L&&void 0!==t){r.logCustom("开始维度分拉伸");const o=t;let s=o/(e.total_score>0?e.total_score:1)||1;
  e.accuracy_score=Q(e.accuracy_score,s),e.fluency_score=Q(e.fluency_score,s),e.standard_score=Q(o,e.total_score),e.has_adjust_dwd=!0,e.stu_total_score=parseFloat(t),...}
else e.accuracy_score=e.accuracy_score,e.fluency_score=e.fluency_score,e.standard_score=e.standard_score;
```

**结论**：语音题的“原始分”由云端引擎给出，前端只做清零/拒识/维度拉伸/**卷面换算**，
**没有任何本地发音评测实现**（`pc_client_web` 里 grep `ise`/`sdk` 只命中接口调用与结果解析）。

### 3.3 iframe 内部**不判分**
iframe 里仅有的“相等比较”全部是**上色**，不产出分数：

| 位置 | 原文 | 作用 |
|---|---|---|
| `pc-listen-choose.js:121`、`:129` | `if (user_answer == right_answer) { ...addClass('c_green') } else { ...addClass('c_red') }` | 上色 |
| `pc-listen-fn.js:974`、`:982`、`:997` | `if (answer === user_answer) {...select_right} else {...select_wrong}`（`kttb_setPcChoise`） | 上色 |
| `pc-listen-fn.js:816-820` | `if (item.status == '0'){红} else if(item.status=='1'){绿}` | 按**外层给的 status** 上色 |

---

## 四、提交载荷的构造点（逐字摘录）

### 4.1 唯一出口：`requestAnswerLogs` → `POST /m/audio/sync-v2`
`index-5543d811.js` offset 2436589（**逐字全文**）：

```js
const requestAnswerLogs=async e=>{const{ecardInfo:t}=useUserInfoStore$1(),n=useSpecialStore(),{first_column_id:r,question_type_score:o}=e;let i={};try{i="string"==typeof e.score_detail?JSON.parse(e.score_detail):e.score_detail}catch(e3){}const{total_score_init:a,total_score:s}=i||{},l=a||s,c=await n.getColumnsNeed125(r);c&&t?.rate_scale&&l&&o&&(e.real_score=scoreFormatter_125(o/5*l));const u=c?t?.rate_scale:void 0,d=c&&u?"0.125":void 0;i.rate_scale=u||0,delete i.total_score_init,delete i.xml_score,delete i.question_type_score,delete i.is_rejected,delete i.except_info,delete i.first_column_id,e.score_detail=JSON.stringify(i);const p=Object.assign(e,{rate_scale:u,graduation:d,client_time:currentTimestampToDate(),mark_service:1,version:"2"});return await hyRequest.postIt("/m/audio/sync-v2",Object.assign(p))}
```

字段来源逐条：

| 字段 | 来源（原文） | 说明 |
|---|---|---|
| `score_detail` | 入参 `e.score_detail`（对象或 JSON 串）→ 删 6 个键 → `JSON.stringify` | **最终是字符串** |
| `i.rate_scale` | `u||0`，`u = need125 ? ecardInfo.rate_scale : undefined` | 非 1.25 列 → `0` |
| `total_score` | 不动，保留原值 | 语音题=0~5 原始分；客观题=卷面分（0 或 `question_score/question_count`） |
| `real_score` | 仅当 `need125 && rate_scale && (total_score_init\|\|total_score) && question_type_score` 时 **覆盖**为 `scoreFormatter_125(question_type_score/5*total_score)` | 非 1.25 列时**保持入参原样** |
| `rate_scale`（顶层） | `u` | 非 1.25 列 → `undefined`（JSON 里消失） |
| `graduation` | `c&&u ? "0.125" : undefined` | **字符串**，且只在 1.25 列 |
| `client_time` | `currentTimestampToDate()` | |
| `mark_service` | 常量 `1` | |
| `version` | 常量字符串 `"2"` | |

`getColumnsNeed125`（`index-5543d811.js` offset 3389285，**逐字**）：

```js
async getColumnsNeed125(e){const t=[FirstColumnType.SimulationExam,FirstColumnType.ListeningSimulation,FirstColumnType.SynchronousListening];await this.getSpecialColumn();return this.specialColumn.reduce((e,t)=>[...e,...t.first_column.map(e=>e.id)],t).some(t=>t==e)}
```

### 4.2 请求体的“默认骨架”
`chunks/useAudioSync-0603b875.js`（**整文件**，426B）：

```js
import{fr as e}from"./index-5543d811.js";function i(){const i={resource_id:"",real_score:0,score:0,set_id:"",entity_id:"",order:"",serial_id:"",mistake_id:0,upload_file_id:"",upload_file_url:"",spx_file_url:"",mark_trace_id:"",detail_file_id:"",detail_file_url:"",score_detail:""};return{postAudioSyncLogs:function(r={}){const t={...i,...r};return new Promise((i,r)=>{e(t).then(e=>{i(e)}).catch(e=>{r(e)})})}}}export{i as u};
```

### 4.3 三个页面各自的拼装点

**(a) 考试/练习页**：`Examine-d229829d.js` offset 57915 `Qa(e)`（**逐字**）：

```js
function Qa(e){let a=K.getInstance().handleRequestParams({resource_id:Oe,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:je,entity_id:e.entity_id,order:e.order,upload_file_id:e.upload_file_id??"",upload_file_url:e.upload_file_url??"",spx_file_url:e.spx_file_url??"",mark_trace_id:e.mark_trace_id??"",detail_file_id:e.detail_file_id??"",detail_file_url:e.detail_file_url??"",score_detail:e.score_detail,question_total_point:R.total_point});if(ka({detail_file:e.xml_local_path??"",audio:e.audio_local_path??"",detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,standard_score:"",choose_id:a.score_detail.choose_id,score_detail:a.score_detail}),e.noPost)return;const{postAudioSyncLogs:l}=xe();V.logCustom(`Examine.vue获取上报选择题数:${JSON.stringify(a)}`,{tag:"FLOW_PRAC"});return await l(a)}
```

- `handleRequestParams`（`index-5543d811.js` offset 2430660，**逐字**）：`handleRequestParams:e=>(e.exam_id=null,{...e,homework_id:a.value})`。
- **注意**：`Qa` 重新构造**字面量**，所以上游 `chooseHandler` 里的 `serial_id` / `choose_id` / `mistake_id`
  **不会**进请求体；请求体是「14 个固定键 + `homework_id` + `exam_id:null` + requestAnswerLogs 追加的 4 键」。
- 选择题的 `chooseHandler`（offset 51557，**逐字**）：

```js
Ze.value.chooseHandler(async e=>{const a=K.getInstance().handleRequestParams({resource_id:Oe,real_score:e.score_detail.real_score??e.score_detail.total_score,score:e.score_detail.total_score,set_id:je,entity_id:e.entity_id,order:e.order,serial_id:R.serial_id,upload_file_id:"",upload_file_url:"",detail_file_id:"",detail_file_url:"",choose_id:e.score_detail.choose_id,score_detail:e.score_detail});a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score,ka({detail_file:"",audio:"",detail:a.score_detail,score_detail:a.score_detail,entity_id:a.entity_id,order:a.order,real_score:a.real_score,choose_id:a.score_detail.choose_id,standard_score:""});const l=`${a.entity_id}_${a.order}`;da.value.forEach(e=>{const t=e.questions.find(e=>e.id==l);t&&(t.answer=void 0!==a.score_detail.choose_id&&""!=a.score_detail.choose_id)})}),
```

  即 **选择题在 `choose-check` 步骤才真正上报**：`Ze.value.chooseCheckHandler(async()=>{Xe.value.forEach(e=>{...Ea(e)})})`，
  `Ea`（offset 55148）→ `Qe.addTask(...)` → `Qa(e)`。

**(b) 模考页**：`MockExamDetails-2a7d03a6.js` offset 7393，与 (a) 同构，只是多了
`serial_id:Ie.serial_id`、`mistake_id:Fe??""`，且本地缓存写 `standard_score:Oe(a)`（`Oe` 取 `score_detail.total_score ?? score`）。

**(c) 听说专练/综合页**：`Comprehensive-1968af87.js` offset 7633（**逐字**）：

```js
const l=t.getInstance().handleRequestParams({resource_id:ze,score:e.score_detail.total_score,set_id:Ge,entity_id:e.entity_id,order:e.order,upload_file_id:e.upload_file_id??"",upload_file_url:e.upload_file_url??"",spx_file_url:e.spx_file_url??"",mark_trace_id:e.mark_trace_id??"",detail_file_id:e.detail_file_id??"",detail_file_url:e.detail_file_url??"",mistake_id:Be.query.mistake_id??"",score_detail:JSON.stringify(e.score_detail)});l.first_column_id=e.first_column_id,l.question_type_score=e.question_type_score;const{postAudioSyncLogs:s}=pe();return await s(l)
```
**该路径没有顶层 `real_score`**（与 (a) 不同）。

**(d) 错题重做页**：`Practice-1694d88a.js` offset 11495 `ct(e)`（**逐字**）：

```js
function ct(e){!function(e){const a=J.cloneDeep(pa.value),t=a.findIndex(a=>a.entity_id===e.entity_id&&a.order===e.order);t>-1&&a.splice(t,1),a.push(e),pa.value=a}({audio:"",detail:e.score_detail,detail_file:"",entity_id:e.entity_id,order:Number(e.order),real_score:ut(e),standard_score:e.score_detail.total_score});const a=o.getInstance().handleRequestParams({resource_id:ma,score:e.score_detail.total_score,set_id:_a,entity_id:e.entity_id,order:e.order,mistake_id:na.query.mistake_id??"",upload_file_id:e.upload_file_id??"",upload_file_url:e.upload_file_url??"",spx_file_url:e.spx_file_url??"",mark_trace_id:e.mark_trace_id??"",detail_file_id:e.detail_file_id??"",detail_file_url:e.detail_file_url??"",score_detail:JSON.stringify(e.score_detail),question_total_point:Y.total_point});a.first_column_id=e.first_column_id,a.question_type_score=e.question_type_score;const{postAudioSyncLogs:t}=ke();retu...
```
同样**没有顶层 `real_score`**。

### 4.4 `score_detail` 的字段从哪来（语音题）
`useRecordResult-fbe9e75b.js`（**逐字**全文要点）：

```js
const I={...R,total_score:_(w),total_score_init:w,real_score:G,category:e,accuracy_score:$,fluency_score:b,standard_score:O,first_column_id:p,question_type_score:h};
```
```js
return{xml_local_path:s,audio_local_path:e,upload_file_url:a,spx_file_url:c,mark_trace_id:t?.traceId||t?.sid,detail_file_url:l,upload_file_id:"",detail_file_id:"",score_detail:I}
```
`R` 是解析 XML 得到的属性集（`c(e,t.data.xml)`），`_(w)=scoreFormater(w)`，`G=calculateRealScore(question_type_score,total_score)`。

`adjustWordColor-bccb26cb.js` 的 `I()`（**逐字**，答案页展示用的 `dimension_result` 与 `stu_real_score`）：

```js
const f={stu_real_score:null,stu_real_percentage_score:0,accuracy_score:g.accuracy_score||0,fluency_score:g.fluency_score||0,integrity_score:s,standard_score:g.standard_score||0,dimension_result:[],stu_total_score:d,stu_total_score_init:d,xml_total_score:l};
for(let p in f){const e=f[p];
  if("integrity_score"==p){const r=n(20*e,0)||0;f[p]=r>100?100:r}
  if("stu_total_score"==p){const r=v?v/5*e:e;f.stu_real_score=x??n(r,2),f[p]=y??n(e,2),f.stu_real_percentage_score=v?c(f.stu_real_score,v):n(20*f[p])}}
return f.dimension_result=O?N:[f.accuracy_score,f.fluency_score,f.integrity_score,f.standard_score],f}
```
⇒ `dimension_result = [准确度, 流畅度, 完整度, 标准度]`，与 `showScoreDetail` 的 index 约定一致（`.slice(0,3)` 只取前三维）。

---

## 五、与我方实现的差异

对比基线：先说**快照变动**，因为 `scoring.py` 在本次分析期间被并发修复过：

| 文件 | 19:56 快照 md5 | 20:0x 复核 md5 | 说明 |
|---|---|---|---|
| `runner.py` | `cae64f4d18b793b06ae40910894dba54` | 同 | 未变 |
| `official_payload.py` | `08c2228bb96742fd3f0dbf84214e2b9c` | 同 | 未变 |
| `scoring.py` | `688172578a5de8ded693498efca50af5` | `b1e08cd81336ca0bd764f3e68e677200`（19:59:13） | **已修复 D1** |

### D1 【已修复】`scoring.build_score_detail` 曾不接受 `choose_id`
- 19:56 状态（`runner.py:1618-1624` 调用 vs `scoring.py` 旧签名）会 **`TypeError`**：
  `AUTO_COMPLETE_OFFICIAL_PAYLOAD` 默认关闭时走 `else` 分支，而旧 `scoring.build_score_detail`
  只有 `(profile, *, category, choose, repeat_mode, graduation, clamp)`。
- **20:0x 复核**：`scoring.py:519-528` 已新增 `choose_id: str = ""` 并写入 payload；函数 docstring 亦注明
  “本函数与 `official_payload.build_score_detail` 必须保持关键字参数兼容”。
- 现状：**该差异已消解，无需再动**。

### D2 【口径差异·实现方确认为有意保留】`rate_scale` / `graduation`
| | 官方 PC（`requestAnswerLogs`） | 我方默认（`scoring.build_score_detail`） | 我方 PC 口径（`official_payload` + env 开关） |
|---|---|---|---|
| `score_detail.rate_scale` | `u||0`，非 1.25 列 = `0` | `"rate_scale": 1 if graduation==0.125 else 0` → **恒 1** | `0`（不足 125 列） |
| 顶层 `graduation` | 仅 1.25 列给，**字符串 `"0.125"`** | **无条件**给**数字** `0.125`（`runner.py:1633`） | 仅 1.25 列给字符串 `"0.125"` |
| 顶层 `rate_scale` | 非 1.25 列 → 不出现 | 不出现 | `0` 或 `ecardInfo.rate_scale` |

官方两套口径本身就不一致，这一点实现方已在 `scoring.py` 注释中固化：
> `⚠️ 口径差异（**故意保持现状，勿随手改**）… 官方**移动端** bean 是 private int graduation（ReadFollowAnswerBean.java:28），
> 即两个官方客户端本身就不一致（PC: 条件字符串；移动端: 整数）。我们这条旧路径发浮点 0.125 且 rate_scale 恒 1
> —— 与服务端长期实测可用（回读官方 point 能对上），因此**不在线改**；要切 PC 口径请开 `AUTO_COMPLETE_OFFICIAL_PAYLOAD=1`。`

⇒ 该差异**不再是"会判错"级**（有实测反证），但仍是**默认路径与官方 PC 口径不一致**这一事实；
切到 PC 口径时该字段才是关键，不要在无 A/B 的情况下改默认路径。

### D3 【只影响记录】`score_detail.category` 的取值域不同
- 官方：`category: <question.evaluation_type>`（`G()` / fill IIFE / `Practice._t` / `useRecordResult` 都是这个来源）。
- 我方：`runner.py:1612/1620` 客观题给字面量 `"choose"`，语音题给 `homework.py:422-435 STRUCTURE_TYPE_META` 的粗粒度
  （`"collector.read" → ("read_sentence", "模仿朗读")`，而官方 `evaluation_type` 是 `read_chapter`）。

### D4 【只影响记录】客观题 `score_detail.total_score` 数值域
- 官方客观题：`question_score/question_count`（**小于 5 的卷面分**，或 0）。
- 我方：`profile.total`（0~5 的"ISE 语义分"）格式化两位 → **量纲可能不同**（取决于 `ScoreProfile.total` 代表的量）。

### D5 【只影响记录 / 可能 400】`score`、`total_score` 是**字符串**而非数字
- 官方：`score:e.score_detail.total_score`（number）；`score_detail.total_score` 也是 number（`scoreFormater` 返回值）。
- 我方：`runner.py:1634` `"score": f"{profile.total:.2f}"`；`official_payload.build_score_detail`
  与 `scoring.build_score_detail` 都写 `f"{profile.total:.2f}"` → **JSON 字符串**。

### D6 【只影响记录】语音题 `score_detail` 缺 `real_score`
- 官方 `score_detail.real_score = scoreFormater(question_type_score/5*total_score)`（`useRecordResult` 的 `I`），
  且 `requestAnswerLogs` 的删键清单**不含** `real_score` → 一定送到服务端。
- 我方 `official_payload.build_score_detail` 的 payload 里**没有 `real_score` 键**（只写维度分 + rate_scale + choose/choose_id/category）。
  顶层 `real_score` 也只在 `AUTO_COMPLETE_OFFICIAL_PAYLOAD=1` 时由 `sync_payload_extras` 补。

### D7 【只影响记录】`mark_trace_id` 语义不同
- 官方：`t?.traceId||t?.sid`（评测引擎 trace）。
- 我方：`runner.py:1677` `json.dumps({"engine": "cloud"})`。

### D8 【只影响记录】多余/缺失字段
- 我方多：`correct`、`auto_submit`、`use_time`、`serial_id`、`mistake_id`、`seq_id`；`score_detail` 里多 `stress_score`、`repeat_mode`、`version`。
- 我方缺：`question_total_point`（官方 `Qa` 必带）。
- 官方顶层**没有** `use_time`（用时只在 `m/homework/submit` 出现）。

### D9 【可能需要 A/B 验证 · 潜在会判错】`order` 的取法
- 官方：`order` = 官方结构里的 `xt_xh` / `xth` / `question_order`（大题内**官方题号**，字符串）。
- 我方：`runner.py:1626-1627` `sync_order = str(sub_order + 1)`；`sub_order` 来自 `homework.py` 的
  「content 内下标」（`homework.py:552` `"sub_order": idx`）。
- 等价条件：题号从 1 连续、且一个 content ⇔ 一个大题。`collectors.rs::collect_choose` 是按 `xtlist` 顺序遍历，
  实践上等价；但 `homework.py` 的分节是**按 type 聚合**的（`homework.py:1194-1249` 用 `q.get("sub_order")`），
  若某节跨多个 `stid`，`sub_order` 会在第二个 content 处**重新从 0 开始**（`entity_id` 不同，主键仍唯一）。
  我方未使用 `q["order"]`（= `no` = `{{th}}` 全局题号）——两者都可能与官方 `xt_xh` 不等，**需 A/B 验证**。

### D10 【无影响】`choose` 的取值
- 官方 `choose` = 学生所选选项的 `xx_mc`（字母）；我方 `runner.py:1602` 取正确答案字母。因我们恒答对，数值一致。
- 未作答时官方给 `choose:""`；我方不会出现未作答。

---

## 六. 「会判错的差异」清单（按严重度排序）

> 复核时间 20:0x（`scoring.py` md5 `b1e08cd8…`）。D1 已在 19:59 修复，保留在表中是为了留下"曾经会炸"的痕迹。

| 序 | 差异 | 位置 | 严重度 | 理由（推断处已标注） |
|---|---|---|---|---|
| 1 | ~~非官方分支把 `choose_id=` 传给 `scoring.build_score_detail`~~ | ~~`runner.py:1618-1624` / `scoring.py:519-527`~~ | **已修复（19:59）** | 旧签名缺 `choose_id` ⇒ 默认分支逐题 `TypeError`；现签名与调用已对齐，`scoring.py` docstring 已固化"两分支必须关键字兼容"的约束 |
| 2 | `order` 用「节内下标+1」而非官方题号 | `runner.py:1626-1627` | **可能判错（需 A/B）** | 官方以 `(entity_id, order)` 落库（`getChoice` 串的中间段就是 `xt_xh`）。等价条件见 D9，跨 content 聚合节时不保证 |
| 3 | `score_detail.rate_scale` 默认恒 1；顶层 `graduation` 无条件给数字 `0.125` | `scoring.py` 的 `"rate_scale"` 行；`runner.py:1633` | **口径不一致（实现方已确认有意保留）** | 官方 PC 仅 1.25 倍率列给这两个键、且 `graduation` 是字符串；官方移动端 bean 是 `int graduation` ⇒ 两官方端本身不一致；我方旧路径"服务端长期实测可用（回读 point 能对上）"，故不在线改 |
| 4 | `score_detail.category` 值域不同（`"choose"` / 粗粒度族名 vs `evaluation_type`） | `runner.py:1612`、`homework.py:422-435` | 只影响记录（若服务端按 category 分派则升为"会判错"） | |
| 5 | `score`、`score_detail.total_score` 是字符串 | `runner.py:1634`、两个 `build_score_detail` | 只影响记录（**推断**：服务端若严格按 number 反序列化可能 400） | 官方为 number |
| 6 | `score_detail` 缺 `real_score` | `official_payload.py:150-174` | 只影响记录 | 官方必送；答案页 `E()` 读 `e.real_score` 决定聚合口径 |
| 7 | 默认载荷无顶层 `real_score` | `runner.py:1652-1657`（仅 official 开关打开才给） | 只影响记录 | 官方 `Qa` 路径必给 |
| 8 | `mark_trace_id` 非引擎 trace | `runner.py:1677` | 只影响记录（审计） | |
| 9 | 缺 `question_total_point`；多 `correct/auto_submit/use_time/serial_id/mistake_id/seq_id`、`repeat_mode/version/stress_score` | `runner.py:1628-1680` | 无影响（**推断**：多给字段官方解析器一般忽略） | `use_time` 可能被服务端累计 → 保守起见可只影响记录 |

---

## 附录 A. 「未找到」清单 + 检索关键词

| 目标 | 状态 | 检索关键词（在 `iframe_web/**` + `pc_client_web/**` 全量 grep） |
|---|---|---|
| `setPCChoose2` 实现 | **未找到**（只有调用点 `useExamStepper` offset 9258） | `setPCChoose2`、`setPcChoose2`、`PCChoose2`、`Choose2` |
| `getChooseChange` 实现 | **未找到**（只有调用点 `useExamStepper` offset 9564） | `getChooseChange`、`chooseChange`、`ChooseChange` |
| `ck_ReturnChoose` 实现 | **未找到**（只有调用点 `pc-view-kttb5.js:20`） | `ck_ReturnChoose`、`ReturnChoose` |
| `webInteraction.getAnswerList` 实现 | **未找到**（只有调用点 `method.js:56`，且 `method.js` 无 html 加载） | `webInteraction`、`getAnswerList` |
| `setPCCheckChoose` 调用点 | 定义在 `pc-listen-choose.js:94-107` 与 `PC-choose.js:94-107`，**外层无调用** | `setPCCheckChoose`、`PCCheckChoose` |
| `setUserrecord` 调用点 | iframe 有定义（`pc-listen-fn.js:757-764`），**外层无调用** | `setUserrecord`、`Userrecord`、`userrecord` |
| `showScoreDetail_1` 调用点 | iframe 有定义（`pc-listen-fn.js:484-563`）；**调用者是音标练习页** `chunks/PhoneticPracticeView-a54655d6.js`（postMessage → `phonogramMap.html` 的 `fn.js`），**与 9 族无关** | `showScoreDetail_1` |
| `collector.picture` 专属桥 | **未找到**（该族无任何桥） | `collector.picture`、`picture`、`topic` |
| `showEssay_2` / `showPhrase_2` 的 `kttb` 变体 | **未找到** | `showEssay`、`showPhrase` |
| iframe 侧 `evaluation_type` 使用 | **未找到**（`grep -rn "read_chapter\|read_sentence\|read_word\|evaluation_type" iframe_web` 无命中） | `evaluation_type`、`read_chapter`、`read_sentence` |
| Electron `webSecurity` 配置 | **未找到**（无法解释 sandbox 下如何跨 window 调函数） | `webSecurity`、`nodeIntegration`、`preload`、`allow-same-origin` |
| `fn.js` 的使用页面 | 仅 `phonogramMap.html` | `grep -ln "js/fn.js" *.html` |
| `PC-choose.js` / `method.js` 的使用页面 | **无任何 html 引用** | `grep -ln "PC-choose.js\|method.js" *.html` |
| 服务端如何用 `choose_id` / `rate_scale` / `graduation` | **不可知（纯前端静态范围）** | 需抓包/服务端侧验证 |

## 附录 B. 关键文件索引

| 文件 | 关键行/offset |
|---|---|
| `iframe_web/js/pc-listen-choose.js` | 46-83 `getChoice`；96-140 `setPCChoose`；142-147 `setBackgroundColor` |
| `iframe_web/js/PC-choose.js` | 20-49 `getChoice`；52-91 `setPCChoose`；94-107 `setPCCheckChoose`（**未加载**） |
| `iframe_web/js/pc-listen-fn.js` | 85-148 `showPhrase_2`；269-343 `showEssay_2`；406-482 `showScoreDetail`；484-563 `showScoreDetail_1`；757-764 `setUserrecord`；776-794 `set3q5aScore`；797-809 `getPcBlank`；811-822 `setPcBlank`；836-858 `kttb_getPcBlank`；860-885 `kttb_setPcBlank`；898-950 `kttb_getPcChoise`；952-1013 `kttb_setPcChoise`；1109-1123 message 派发器 |
| `iframe_web/js/fn.js` | 79-142 `showPhrase_2`；390-461 `showEssay_2`；463-539 `showScoreDetail`；832-850 `set3q5aScore`；868-879 `setPcBlank`；864-865/914-915/1006-1007 `postMessage` 变体；1167-1356 完整派发器（**仅 phonogramMap.html 使用**） |
| `iframe_web/js/pc-listen-default.js` | 15-88 `setData`（6 参：`set,page,set_id,order,type`） |
| `iframe_web/js/pc-view-kttb1.js` | 5-14 选项点击；19-60 提交/`kttb_getPcChoise`/`kttb_getPcBlank` 绑定 |
| `iframe_web/pc-listen.html` | 165-206 choose 结构；296-318 fill；397-413 脚本装配 |
| `iframe_web/pc-listen-answer.html` | 15-101 read；103-216 3q5a/role/dialogue；218-300 picture；302-382 choose；384-464 fill；466-559 repeat；561-642 word；665-676 脚本装配 |
| `pc_client_web/chunks/DynamicIframe-bafff5bc.js` | 310 sandbox 默认；865 `M("load",s.contentWindow)` |
| `pc_client_web/chunks/useExamStepper-8ad3fbd7.js` | 1901 `X`；2046 `G`；9258 `setPCChoose2` 调用；9319/12499 `ReturnChoose`；13118 `returnPcBlank`；14229 `getPcBlank()`；15400± `action.record` |
| `pc_client_web/chunks/useExamResult-d9fb4671.js` | 367 `b`；529 `q`；700 `O`；928 `j`；1574 `E`；3200 3q5a 分支；3569 read 分支；4748 choose 分支；5187 fill 分支 |
| `pc_client_web/chunks/useAudioSync-0603b875.js` | 全文件（默认骨架 + `postAudioSyncLogs`） |
| `pc_client_web/chunks/useRecordResult-fbe9e75b.js` | 全文件（评测结果归一 / 零分规则 / `I` 结构） |
| `pc_client_web/chunks/adjustWordColor-bccb26cb.js` | `J`（essayData 8 段）、`R`、`I`（dimension_result）、成绩拉伸 `p` |
| `pc_client_web/chunks/Examine-d229829d.js` | 49243 `ka`；51557 `chooseHandler`；55148 `Ea`；57915 `Qa` |
| `pc_client_web/chunks/MockExamDetails-2a7d03a6.js` | 7393 `chooseHandler`（含 `standard_score:Oe(a)`） |
| `pc_client_web/chunks/Practice-1694d88a.js` | 11495 `ct`；12422 `_t`（choose）；13010 `mt`（fill）；13568 `kttb_ReturnChoose`；13646 `kttb_returnPcBlank` |
| `pc_client_web/chunks/Comprehensive-1968af87.js` | 7633 上报字面量（无顶层 `real_score`） |
| `pc_client_web/chunks/html-8e3719a4.js` | 234 `PcListenAnswer`（= `pc-listen-answer.html`） |
| `pc_client_web/index-5543d811.js` | 845549 `scoreFormater/calculateRealScore`；854783 `scoreFilter`；2360136 `requestAudioTask`；2430660 `handleRequestParams`；2436589 `requestAnswerLogs`；3389285 `getColumnsNeed125` |

## 附录 C. 复现命令（只读）

```bash
# iframe 侧桥定义
grep -n "setPCChoose\|setPcBlank\|set3q5aScore\|showEssay_2\|showPhrase_2\|showScoreDetail\|getChoice\|ReturnChoose\|getPcBlank" \
  "/home/shuaiqiu/Projects/ets源码恢复/iframe_web/js/"*.js
# 外层调用点
grep -rno "setPCChoose2\|getChooseChange\|postAudioSyncLogs\|requestAnswerLogs" \
  "/home/shuaiqiu/Projects/ets源码恢复/pc_client_web/chunks/"*.js
# 判断 iframe 是否被判分使用
grep -c "record" "/home/shuaiqiu/Projects/ets源码恢复/iframe_web/pc-listen.html"   # -> 0
```
