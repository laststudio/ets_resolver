# D — 官方 Android 客户端侧「移动端独有题型族」语义核查

> 范围：**只做 Android 官方客户端静态分析**（已脱壳的 dex / jadx 明文 Java），全程不运行客户端、不抓包、不连网。
> 交叉对照对象：我方 `src/collectors.rs` / `src/spec.rs` / `src/legacy.rs`。
> 关联文档：同目录 `00_core_scoring_and_dispatch.md`（PC 前端侧，9 个族）。

## 0. 一句话结论

**Android 官方客户端里根本不存在 `repeat_essay` / `repeat_dialogue` 这两个族名，也不存在 `collector.essay` /
`collector.dialogue` / `collector.3q5a` / `collector.choose` / `collector.role` / `collector.picture` / `collector.fill`。**
全 APK 11 个 dex 的字符串表里，`collector.` 前缀的族名**只有 4 个**：
`collector.read`、`collector.repeat`、`collector.sentence`、`collector.word`。

- 「故事复述 / 语篇复述」在移动端的落点是 **`collector.repeat`**（按 extra_data 子表切分成 N 题，整题分平均摊派），
  参考文本来自**下发字段**（`sublist[].text` / `info.value`），**不是模板固定**。
- 「对话跟读 / 角色扮演」在移动端的落点是 **`DialoguePaperBean` + `paper.Jason` 的 `sublist[]`**（每行一个 `role`/`seq`/`begintime`/`endtime`），
  以及**教程模块（tutorial/cc）里独立存在的 `CcExamSpeakDialogueFrg`**；它**不叫** `repeat_dialogue`。
- 词汇判分口径：选择题比的是**选项文本 `option_name`**（不是选项字母、不是选项 id）；拼写/听写比的是
  `trim+toLowerCase` 后的**文本**；朗读类客户端**不做文本比对**，分数全部来自评测引擎 XML（`total_score` 等）。
- 题目命中的**族名 `exam_type_collect` 是服务端下发的不透明字符串**，客户端只在 4 处硬编码比较；其余分派逻辑在
  native 壳（`com.fort.andjni.JniLib`）里，Java 侧是空桩。

---

## 1. 取证方法与「未找到」的检索清单

### 1.1 检索途径

| 途径 | 命令 / 对象 |
|---|---|
| 明文 Java 全树 grep | `grep -rn "<kw>" /home/shuaiqiu/Projects/ets源码恢复/ --include="*.java"`（42 个目录） |
| **dex 字符串表**（不依赖是否被 jadx 覆盖） | 自写 Python 读 `string_ids`：11 个 dex → 266,136 条唯一串，落盘 `/tmp/ets_dex_strings.txt` |
| dex 精确串命中 | `grep -x "<token>" /tmp/ets_dex_strings.txt` |
| **补齐 jadx 缺失的类** | 用 `~/.local/bin/jadx` 1.5.6 把 **11 个 dex 全部**反编译到 `/tmp/jadx_c11`、`/tmp/jadx_rest`、`/tmp/jadx_rest2`（原始 `jadx_*` 目录只是子集，缺 `com.ets100.secondary.tutorial.*`、`h7`、`CcStruct*` 等） |

> 说明：`jadx_*` 目录里没有的类（例如 `CcExamSpeakDialogueFrg`、`com.ets100.secondary.utils.h7`、
> `model.bean.cc.*`）我**没有改动**任何已有目录，而是把 dex 重新反编译到 `/tmp` 再查。产物仅供参考，不属交付物。

### 1.2 「未找到」结论及其检索方式

| 目标 | 结果 | 已查方式 |
|---|---|---|
| `repeat_essay` | **未找到** | ① 42 目录 `grep -rn`；② 11 dex 字符串表 `grep -cx` = 0；③ 全量反编译树 `grep -rl` = 0 |
| `repeat_dialogue` | **未找到** | 同上 |
| `REPEAT_ESSAY` / `REPEAT_DIALOGUE`（大写下划线） | **未找到** | `grep -rn "REPEAT_" --include="*.java"` = 0；dex 字符串表 = 0 |
| `question_type_name` | **未找到**（客户端无此字段） | 11 dex 字符串表 `grep -cx` = 0；全量反编译树 `grep -rn` = 0 |
| `collector.essay` / `collector.dialogue` / `collector.3q5a` / `collector.role` / `collector.picture` / `collector.fill` / `collector.choose` | **未找到** | `grep -rhoE '"collector\.[a-z_0-9]+"'` 于全量反编译树 → 只有 4 个 |
| 中文「复述」作为题型名 | **未找到**（唯一命中是超能单词文案里的「跟读复述训练」） | `grep -rn "复述"` 全量反编译树 |
| 「故事复述 / 短文复述」中文 | **未找到** | dex 字符串表 grep |
| `story` / `retell` 作为族名 token | `retell` 存在，`story` 不存在；二者都**无 Java 比较点** | dex 字符串表 `grep -cx` |
| `fj`（PC 前端 `collector.repeat` 用的 `info.fj`） | dex 里 `fj` 存在但**没有任何 Java 引用**（唯一命中是 `com.googlecode.mp4parser.util.Iso639` 的语种码表） | `grep -rn '"fj"'` 全量反编译树 |

---

## 2. 族一：`collector.repeat_essay`（故事复述 / 语篇复述）

### 2.1 Android 侧**是否存在独立族**：不存在

**证据 1 — 全 APK dex 字符串表中 `collector.` 族名只有 4 个：**

```bash
$ cd /home/shuaiqiu/Projects/ets源码恢复/7c049f4b45298848ef1137b58e393915
$ for f in *.dex; do strings -n 6 "$f"; done | grep -o "collector\.[a-z_0-9]*" | sort -u
collector.read
collector.repeat
collector.sentence
collector.word
```

**证据 2 — Java 侧唯一会去比较族名的地方只有 `PaperUtils`，而且只比较 `"collector.repeat"`：**

`ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1826`（类 `PaperUtils`，方法 `p0`）

```java
public final PaperStructBean p0(PaperStructBean paperStructBean, Map<String, List<List<InfoExtraBean>>> contentExtraMap) {
    for (TemplateResBean.ExamTypeListBean examTypeListBean : paperStructBean.getTemplateResBean().getExamTypeList()) {
        if (examTypeListBean.getExamTypeCollect() != null && "collector.repeat".equals(examTypeListBean.getExamTypeCollect().toLowerCase())) {
```

同文件 `:1878`（方法 `q0`）同样只比 `"collector.repeat"`。

**推论（有据）**：`exam_type_collect` 对客户端是**服务端下发的不透明字符串**（`TemplateResBean.ExamTypeListBean.examTypeCollect` 就是一个 `String` 字段），
客户端只对 4 个取值做了分支。因此即使服务端下发 `collector.repeat_essay`，客户端也**没有任何代码路径会认得这个名字**。

```java
// ets源码恢复/jadx_paper_utils/sources/com/ets100/secondary/model/mock/TemplateResBean.java:87
@SerializedName("exam_type_collect")
private String examTypeCollect;
```

**证据 3 — 唯一的总分派 `PaperUtils.d()` 是 native 桩，不在 Java 里：**

```java
// ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1285
public final PaperStructBean d(PaperStructBean paperStructBean, Map<String, List<List<InfoExtraBean>>> contentExtraMap) {
    Object objCL = JniLib.cL(PaperUtils.class, this, paperStructBean, contentExtraMap, 14747);
    ...
}
```

`p0` / `q0` 在全量反编译树里**没有任何 Java 调用点**（`grep -rn "\.p0(\|\.q0("` 只命中同名无关方法），
说明它们是被 native（`JniLib` id 14747 那条链）回调的。

### 2.2 数据来源字段与层级（官方 `collector.repeat`）

`PaperUtils.p0` 的入参就是数据来源：`Map<examId, List<List<InfoExtraBean>>> contentExtraMap`。

```java
// ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1826-1842
for (TemplateResBean.ExamTypeListBean examTypeListBean : paperStructBean.getTemplateResBean().getExamTypeList()) {
    if (examTypeListBean.getExamTypeCollect() != null && "collector.repeat".equals(examTypeListBean.getExamTypeCollect().toLowerCase())) {
        Iterator<TemplateResBean.ExamListBean> it = examTypeListBean.getExamList().iterator();
        while (it.hasNext()) {
            String examId = it.next().getExamId();
            List<List<InfoExtraBean>> list = contentExtraMap.get(examId);
            if (list == null || list.size() == 0) {
                FileLogUtils.t(b, "resetTemplateCtrBean extraData == null codeId == " + examId);
                return null;
            }
            List<TemplateCtrlBean> templateCtrlData = paperStructBean.getTemplateCtrlData();
            ArrayList arrayList = new ArrayList();
            ArrayList arrayList2 = new ArrayList();
            for (TemplateCtrlBean templateCtrlBean : templateCtrlData) {
                if (TextUtils.equals(examId + "_1", templateCtrlBean.getSubid())) {
                    arrayList2.add(templateCtrlBean);
                } else {
                    arrayList.add(templateCtrlBean);
                }
            }
            int i = 0;
            while (i < list.size()) {
                List<InfoExtraBean> list2 = list.get(i);
                StringBuilder sb = new StringBuilder();
                sb.append(examId);
                sb.append("_");
                i++;
                sb.append(i);
                h(arrayList, arrayList2, list2, sb.toString());
            }
```

**层级（已被代码证实）**：

```
contentExtraMap : Map<examId, List< List<InfoExtraBean> >>
                    │        │
                    │        └── 外层 list = 该大题下的「小题」序列  → 一题一条
                    └─────────── key = exam_list[].exam_id（与 content.json 的 code_id 挂载）
templateCtrlData : List<TemplateCtrlBean>   ← 模板控制数据
    └── subid == "<examId>_1" 的那批被当作「重复单元」（arrayList2），其余保留
```

`InfoExtraBean` 类本身在 `jadx_*` 里**未找到**（`find / -name InfoExtraBean.java` = 空），
只在 `PaperUtils` 里被 import（`jadx_paper/.../PaperUtils.java:22`），本体应在未导出到 `jadx_*` 的 dex 里；
但它的用法被上面这段固定住了：`getId()` / `getValue()` 作为**占位符替换对**：

```java
// ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1511-1520（方法 h）
for (InfoExtraBean infoExtraBean : infoExtraBeans) {
    filename  = filename.replaceAll(infoExtraBean.getId(), infoExtraBean.getValue() + "");
    playtime  = playtime.replaceAll(infoExtraBean.getId(), infoExtraBean.getValue() + "");
    playhint  = playhint.replaceAll(infoExtraBean.getId(), infoExtraBean.getValue() + "");
    for (TemplateCtrlBean.JsparamBeanX jsparamBeanX3 : modelMobileBean.getJsparam()) {
        jsparamBeanX3.setCodeId(jsparamBeanX3.getCodeId().replaceAll(infoExtraBean.getId(), infoExtraBean.getValue() + ""));
    }
}
```

### 2.3 一题如何切分 / 题号如何生成 / 分值规则（原文）

全部在 `PaperUtils.q0`（`:1878-1901`）：

```java
public final PaperStructBean q0(PaperStructBean paperStructBean, Map<String, List<List<InfoExtraBean>>> contentExtraMap) {
    for (TemplateResBean.ExamTypeListBean examTypeListBean : paperStructBean.getTemplateResBean().getExamTypeList()) {
        if (examTypeListBean.getExamTypeCollect() != null && "collector.repeat".equals(examTypeListBean.getExamTypeCollect().toLowerCase())) {
            for (TemplateResBean.ExamListBean examListBean : examTypeListBean.getExamList()) {
                float fE2 = s7.e2(examListBean.getExamScore());          // 大题总分
                String examId = examListBean.getExamId();
                List<List<InfoExtraBean>> list = contentExtraMap.get(examId);
                if (list == null || list.size() == 0) { ... return null; }
                int size = list.size();                                   // ← 题数 = extra_data 子表条数
                float fK2 = s7.k2(Float.valueOf(fE2 / size), 2);          // 每题分 = round2(总分/题数)
                int i10 = size - 1;
                float fD0 = d0(Float.valueOf(fE2 - (i10 * fK2)));         // 末题分 = 总分 - (size-1)*每题分（吸收舍入误差）
                int i11 = 0;
                TemplateResBean.QuestionListBean questionListBean = examListBean.getQuestionList().get(0);
                ArrayList arrayList = new ArrayList(size);
                while (i11 < size) {
                    TemplateResBean.QuestionListBean questionListBean2 = new TemplateResBean.QuestionListBean();
                    StringBuilder sb2 = new StringBuilder();
                    int i12 = i11 + 1;
                    sb2.append(i12);
                    sb2.append("");
                    questionListBean2.setQuestionOrder(sb2.toString());   // ← 题号 = "1".."size"
                    questionListBean2.setQuestionCount("1");              // ← 每题恒为 1 小问
                    if (i11 == i10) {
                        questionListBean2.setQuestionScore(fD0 + "");     // 末题用余数
                    } else {
                        questionListBean2.setQuestionScore(fK2 + "");     // 其余用均分
                    }
                    questionListBean2.setEvaluationType(questionListBean.getEvaluationType());
                    questionListBean2.setEngineArea(questionListBean.getEngineArea());
                    questionListBean2.setEngineType(questionListBean.getEngineType());
                    arrayList.add(questionListBean2);
                    i11 = i12;
                }
                examListBean.getQuestionList().clear();
                examListBean.getQuestionList().addAll(arrayList);
```

| 口径 | 官方取值 |
|---|---|
| 切分依据 | `contentExtraMap[exam_id]` 的外层条数 = 题数 |
| 题号 | `question_order = "1" … "<size>"`（从 1 连续，字符串） |
| 题内小问数 | `question_count = "1"` |
| 每题分 | `round2(大题分 / 题数)`；**最后一题** = `大题分 - (题数-1)*每题分` |
| ctrl 分组子 id | `subid = "<exam_id>_<n>"`（n 从 1 起）；模板里原生的 `"<exam_id>_1"` 那批被复制成每题的模板 |
| r 复制来源 | `PaperUtils.h(allCtrlData, repeatCtrlData, infoExtraBeans, subId)`（`:1497`），把模板 ctrl 的占位符用当期 extra_data 替换后 `setSubid(subId)` |

### 2.4 答案 / 参考文本从哪来：**下发字段**，不是模板固定

移动端「复述/跟读」的**参考文本就是下发的原文**，有两条下发路径：

**路径 A — `content.json` 的 `info.value`（篇章原文）+ `info.translate`（译文）+ `info.audio`（范读音频）**

```java
// ets源码恢复/jadx_bean/sources/com/ets100/secondary/model/bean/ContentJsonInfoBean.java:14-33
private final String analyze;
private final String audio;
private final String image;
private final String stid;
private final String translate;
private String value;
private final String video;
```

```java
// ets源码恢复/jadx_paper_utils/sources/com/ets100/secondary/utils/PaperUtilsKt.java:1189-1194
str = fileName2;
str2 = "collector.read";
...
paperStructBean.setContentJsonBean(new ContentJsonBean(str2, new ContentJsonInfoBean("", str25, "", "", str12, str26, "")));
//                                              structureType ↑        stid↑  value↑        audio↑ translate↑
```

**路径 B — `paper.Jason` 的 `txlist[0].stlist[0].sublist[].text`（角色/句子级原文）** — 见 §3.2。

**路径 C — 教程模块 `pe.p.U()` 把同一份原文注入 H5 的 classname**：

```java
// ets源码恢复/jadx_models/sources/pe/p.java:572-586（类 pe.p，方法 U(Custom2StructBean)）
if (custom2StructBean.isReadWord()) {
    JsQuestionParamBean jsQuestionParamBean  = new JsQuestionParamBean("words_con", custom2StructBean.getText());
    JsQuestionParamBean jsQuestionParamBean2 = new JsQuestionParamBean("words_tla", custom2StructBean.getTranslate());
    ...
} else if (custom2StructBean.isReadSentence()) {
    JsQuestionParamBean jsQuestionParamBean3 = new JsQuestionParamBean("phrase_con", custom2StructBean.getText());
    JsQuestionParamBean jsQuestionParamBean4 = new JsQuestionParamBean("sen_tla",    custom2StructBean.getTranslate());
    ...
} else if (custom2StructBean.isReadChapter()) {
    JsQuestionParamBean jsQuestionParamBean5 = new JsQuestionParamBean("essay_con", custom2StructBean.getText());
    JsQuestionParamBean jsQuestionParamBean6 = new JsQuestionParamBean("essay_tla", R(custom2StructBean.getText(), custom2StructBean.getTranslate()));
    ...
}
```

```java
// ets源码恢复/jadx_custom2/sources/com/ets100/secondary/ui/custom2/bean/JsQuestionParamBean.java:12-34
public static final String CHAPTER_TEXT      = "essay_con";
public static final String CHAPTER_TRANSLATE = "essay_tla";
public static final String SENTENCE_TEXT     = "phrase_con";
public static final String SENTENCE_TRANSLATE= "sen_tla";
public static final String WORD_TEXT         = "words_con";
public static final String WORD_TRANSLATE    = "words_tla";
```

即：**篇章（essay）与句子/单词共用同一套"原文=答案"结构，只换 classname 前缀**（`essay_*` / `phrase_*` / `words_*`），
文本一律来自 `Custom2StructBean.getText()`（下发），**没有"参考范文模板"**。

### 2.5 分值规则 / 提交标识字段

| 项 | 官方取值 / 来源 | 证据 |
|---|---|---|
| 题干分值 | 见 §2.3（均分 + 末题余数） | `PaperUtils.q0` |
| 单行最大分 | `dialoguePaperItemBean.setMaxScore(1.0f)` | `PaperUtils.java:1251` |
| 提交标识 | `stid`（资源 id）、`entity_id`、`order`、`seq`、`seq_id`、`homework_id`、`resource_id`、`set_id` | 见下 |
| 朗读/跟读类不走 `score_rate` 提交，走 `xml` 评测结果 | `category` = `read_word` / `read_sentence` / `read_chapter` | `jadx_score/.../xml/a.java:37-43` |

```java
// ets源码恢复/jadx_score/sources/com/ets100/secondary/xml/a.java:33-55（类 com.ets100.secondary.xml.a）
public static final String f40644i = "rec_paper";
public static final String f40645j = "read_word";
public static final String f40646k = "read_chapter";
public static final String f40647l = "read_sentence";
public static final String f40648m = "word";
public static final String f40649n = "phone";
public static final String f40650o = "sentence";
public static final String f40651p = "xml_result";
```

### 2.6 我方 `collect_repeat_essay` 的落点（供 §5 对照）

```rust
// src/collectors.rs:16
pub const TYPE_REPEAT_ESSAY: &str = "collector.repeat_essay";
```

```rust
// src/collectors.rs:537-569
pub fn collect_repeat_essay(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Vec<Value> {
    let stid = stid_of(content);
    let info_obj = content.get("info");
    if let Some(sublist) = info_obj.and_then(|o| o.get("sublist")).and_then(|v| v.as_array()) {
        ... // 每条 sublist 一项成题，answers = [text]
    }
    ...
    collect_read(content, content2, info)
}
```

---

## 3. 族二：`collector.repeat_dialogue`（对话 / 语篇听读）

### 3.1 Android 侧**是否存在独立族**：不存在同名族；但存在**独立的对话数据结构与独立 Fragment**

- 族名 `collector.repeat_dialogue`：**未找到**（同 §1.2）。
- 独立数据结构：`DialoguePaperBean` / `DialoguePaperItemBean`（`jadx_*` 里只导出到使用点，未导出 bean 本体）。
- 独立 UI：`com/ets100/secondary/tutorial/ui/cc/frg/CcExamSpeakDialogueFrg`（教程模块，dex 有类定义，`jadx_*` 未导出）。

**证据 — dex 类名与 Kotlin 源映射：**

```text
$ cd /home/shuaiqiu/Projects/ets源码恢复/7c049f4b45298848ef1137b58e393915
$ for f in *.dex; do strings -n 4 "$f"; done | grep -o "com/ets100/secondary/tutorial/ui/cc/frg/[A-Za-z0-9]*" | sort -u
com/ets100/secondary/tutorial/ui/cc/frg/BaseCcFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamChooseDoubleFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamChooseSingleFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamSentenceSortFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamSpeakCompositionFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamSpeakDialogueFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamSpeakSentenceFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamWordBlankFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamWordSortFrg
com/ets100/secondary/tutorial/ui/cc/frg/CcExamWordTableFrg
```

**这 10 个（1 基类 + 9 子类）正是移动端**独立于 PC 前端 9 族之外**的一套题面类型**：

| CC Fragment | 布局 id | Kotlin 源（`@SourceDebugExtension` 内核） |
|---|---|---|
| `CcExamChooseSingleFrg` | `frg_cc_exam_choose_single` | `CcExamChooseSingleFrg.kt` |
| `CcExamChooseDoubleFrg` | `frg_cc_exam_choose_double` | `CcExamChooseDoubleFrg.kt` |
| `CcExamWordBlankFrg` | `frg_cc_exam_word_blank` | `CcExamWordBlankFrg.kt` |
| `CcExamWordSortFrg` | `frg_cc_exam_word_sort` | `CcExamWordSortFrg.kt` |
| `CcExamWordTableFrg` | `frg_cc_exam_word_table` | `CcExamWordTableFrg.kt` |
| `CcExamSentenceSortFrg` | `frg_cc_exam_sentence_sort` | `CcExamSentenceSortFrg.kt` |
| **`CcExamSpeakCompositionFrg`** | **`frg_cc_exam_speak_composition`** | `CcExamSpeakCompositionFrg.kt` |
| **`CcExamSpeakDialogueFrg`** | **`frg_cc_exam_speak_dialogue`** | `CcExamSpeakDialogueFrg.kt` |
| **`CcExamSpeakSentenceFrg`** | **`frg_cc_exam_speak_sentence`** | `CcExamSpeakSentenceFrg.kt` |

```java
// /tmp/jadx_c11/.../tutorial/ui/cc/frg/h.java:1-8   ← 即 CcExamSpeakDialogueFrg（jadx 把类名混淆成 h）
@kotlin.jvm.internal.SourceDebugExtension({"SMAP\nCcExamSpeakDialogueFrg.kt\nKotlin ... + 1 CcExamSpeakDialogueFrg.kt\ncom/ets100/secondary/tutorial/ui/cc/frg/CcExamSpeakDialogueFrg ..."})
public final class h extends com.ets100.secondary.tutorial.ui.cc.frg.BaseCcFrg<com.ets100.secondary.tutorial.databinding.FrgCcExamSpeakDialogueBinding> {
    @Override // com.ets100.secondary.tutorial.ui.cc.frg.BaseCcFrg
    public int B() {
        return com.ets100.secondary.tutorial.R.layout.frg_cc_exam_speak_dialogue;
    }
```

```java
// /tmp/jadx_c11/.../tutorial/ui/cc/frg/g.java:1-8   ← CcExamSpeakCompositionFrg
public final class g extends ...BaseCcFrg<...FrgCcExamSpeakCompositionBinding> {
    public int B() { return com.ets100.secondary.tutorial.R.layout.frg_cc_exam_speak_composition; }
}
```

```java
// /tmp/jadx_c11/.../tutorial/ui/cc/frg/i.java:1-8   ← CcExamSpeakSentenceFrg
public final class i extends ...BaseCcFrg<...FrgCcExamSpeakSentenceBinding> {
    public int B() { return com.ets100.secondary.tutorial.R.layout.frg_cc_exam_speak_sentence; }
}
```

> ⚠️ **重要边界**：这三个 Fragment 的**方法体全部是 native 桩**，`B()`（布局 id）是唯一的纯 Java 逻辑。
> 因此「哪个 `category` 值 → 哪个 Fragment」的映射**无法从 Java 静态还原**：

```java
// /tmp/jadx_c11/sources/com/ets100/secondary/tutorial/ui/cc/frg/BaseCcFrg.java（逐字原文，节选）
public abstract class BaseCcFrg<DB extends androidx.databinding.ViewDataBinding> extends androidx.fragment.app.Fragment {
    @cu.k public final kotlin.Lazy a;
    @cu.l public CcStructQuestion b;            // 当前题（setter N(...)）
    @cu.l public CcStructChildren c;            // 当前小题（setter K(...)）
    public int d;
    @cu.k public String e = A();                // A() 走 JniLib.cL
    public DB f;

    public final String A() {
        Object objCL = JniLib.cL(new Object[]{BaseCcFrg.class, this, 5669});
        if (objCL == null) { return null; }
        return (String) objCL;
    }
    public abstract int B();                    // ← 唯一"纯 Java"的抽象点：返回布局 id
    public void C() { JniLib.cV(new Object[]{BaseCcFrg.class, this, 5670}); }
    public void D() { JniLib.cV(new Object[]{BaseCcFrg.class, this, 5671}); }
    public abstract void E();
    public void F() { JniLib.cV(new Object[]{BaseCcFrg.class, this, 5672}); }
    ...
    public abstract void z();
}
```

`BaseCcFrg` 的 `A()/C()/D()/F()/G()/H()/I()/J()/L()/M()/N()/O()/P()/Q()/R()/p()/q()/r()/s()/x()/y()`
**全部是 `JniLib.cL/cV/cZ/cF` 空桩**，Java 侧只有 `B()`（抽象，子类返回布局 id）与几个字段访问器有实体。

### 3.2 数据来源字段与层级（对话族，`paper.Jason`）

**来源文件不是 `content.json`，而是 `paper.Jason`** —— 见 `PaperUtils.c0`：

```java
// ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1186-1224
public final DialoguePaperBean c0(String paperFileName, String engineArea, String resId, String paperId, String entityId) {
    DialoguePaperBean dialoguePaperBean;
    String str = ChartRecordDB.columnRole;
    String strN1 = EtsUtils2.n1(resId);
    FileLogUtils.t(f36752b, "parseDialoguePaperBean resEngineType = " + strN1 + ", paperId = " + paperId);
    List<ResEngineBean> listC = c5.c(strN1, new TypeToken<List<ResEngineBean>>() {}.getType());
    StringBuilder sb2 = new StringBuilder();
    sb2.append(r4.p());
    sb2.append(paperFileName);
    String string = sb2.toString();
    File file = new File(string, "paper.Jason");          // ← 资源文件名
    if (file.exists()) {
        try {
            String strQ = w4.q(file);
            int iIndexOf = strQ.indexOf(o6.c.f80858d);
            int iLastIndexOf = strQ.lastIndexOf(o6.c.f80859e);
            ...
            JSONObject jSONObject = new JSONObject(strQ.substring(iIndexOf, iLastIndexOf + 1));
            DialoguePaperBean dialoguePaperBean2 = new DialoguePaperBean();
            dialoguePaperBean2.setFileName(paperFileName);
            dialoguePaperBean2.setPaperId(paperId);
            JSONArray jSONArray = jSONObject.getJSONArray("txlist");
            if (jSONArray.length() > 0) {
                boolean z10 = false;
                JSONArray jSONArray2 = jSONArray.getJSONObject(0).getJSONArray("stlist");
                if (jSONArray2.length() > 0) {
                    JSONObject jSONObject2 = jSONArray2.getJSONObject(0);
                    dialoguePaperBean2.setAudioName(jSONObject2.getString("audio"));
                    ...
                    dialoguePaperBean2.setEntityId(... jSONObject2.getString("stid") ...);
                    dialoguePaperBean2.setTitle(jSONObject2.has("title") ? jSONObject2.getString("title") : "");
                    JSONArray jSONArray3 = jSONObject2.getJSONArray("sublist");       // ★ 对话行数组
```

**每行（一题）的字段 —— 原文（参考文本）就是 `sublist[].text`，是下发字段：**

```java
// ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1229-1266
while (i10 < jSONArray3.length()) {
    JSONObject jSONObject3 = jSONArray3.getJSONObject(i10);
    DialoguePaperItemBean dialoguePaperItemBean = new DialoguePaperItemBean();
    dialoguePaperItemBean.setFileName(paperFileName);
    dialoguePaperItemBean.setEngineArea(engineArea);
    dialoguePaperItemBean.setNewStruct(z10);
    dialoguePaperItemBean.setPaperId(dialoguePaperBean2.getPaperId());
    dialoguePaperItemBean.setEntityId(dialoguePaperBean2.getEntityId());           // ← entity_id 继承自 stlist[0].stid
    dialoguePaperItemBean.setOrder(s7.m2(jSONObject3.getString("seq")));           // ← 题号 = sublist[].seq
    dialoguePaperItemBean.setRole(jSONObject3.has(str) ? jSONObject3.getString(str) : "");  // str = ChartRecordDB.columnRole
    String role = dialoguePaperItemBean.getRole() != null ? dialoguePaperItemBean.getRole() : "";
    if (!map.containsKey(role)) { map.put(role, Integer.valueOf(i11)); i11++; }
    dialoguePaperItemBean.setRoleIndex(((Integer) map.get(role)).intValue());      // ← 角色序号（按首次出现顺序 0,1,2…）
    dialoguePaperItemBean.setTextContent(jSONObject3.getString("text"));           // ★ 参考文本 = 下发 text
    ...
    dialoguePaperItemBean.setBeginTime((float) jSONObject3.getDouble("begintime"));
    dialoguePaperItemBean.setEndTime((float) jSONObject3.getDouble("endtime"));
    dialoguePaperItemBean.setTranslate(jSONObject3.getString("translate"));
    dialoguePaperItemBean.setMaxScore(1.0f);                                       // ★ 单行满分 = 1.0
    dialoguePaperItemBean.setCategory(com.ets100.secondary.xml.a.f40647l);         // ★ category = "read_sentence"
    String entityOrder = dialoguePaperItemBean.getEntityOrder();                   // ← 复合 key（native 计算）
    ChildPaperBean childPaperBeanZ = Z(new File(string, entityOrder + File.separator + entityOrder + ".ctrl"));
    dialoguePaperItemBean.setChildPaperBean(childPaperBeanZ);
```

```java
// ets源码恢复/jadx_paper/sources/com/ets100/secondary/utils/PaperUtils.java:1281
r0(dialoguePaperBean, listC);   // 用 ResEngineBean 决定 practiceType（isUseMock）
```

**层级汇总：**

```
paper.Jason  (JSON，被 o6.c 的起止标记切出)
└── txlist[0]
    └── stlist[0]
        ├── audio    → DialoguePaperBean.audioName（整段范读音频）
        ├── stid     → DialoguePaperBean.entityId（★ 提交用 entity_id）
        ├── title    → DialoguePaperBean.title
        └── sublist[]  ← 一行一题
            ├── seq        → order（题号）
            ├── role       → role / roleIndex（角色扮演）
            ├── text       → textContent（★ 参考文本 = 答案）
            ├── translate  → translate
            ├── begintime / endtime → 时间轴（角色扮演逐句高亮/对齐）
            └── (每行) childPaperBean = file("<entityOrder>/<entityOrder>.ctrl")  ← 行级模板
```

### 3.3 一题如何切分 / 题号如何生成（对话族）

| 项 | 官方口径 | 证据 |
|---|---|---|
| 一题 = | `sublist[]` 的**一条** | `PaperUtils.java:1229` while 循环 |
| 题号 | `order = int(sublist[].seq)` | `PaperUtils.java:1237` |
| 复合键 | `entityOrder`（native 计算，`getEntityOrder()` 走 `JniLib`） | `DialoguePaperItemBean.getEntityOrder()` |
| 角色 | `role`（字符串） + `roleIndex`（按首次出现顺序编号） | `PaperUtils.java:1238-1244` |
| 行级模板 | `<entityOrder>/<entityOrder>.ctrl` | `PaperUtils.java:1253-1254` |
| 单行满分 | `1.0f` | `PaperUtils.java:1251` |
| 单行 category | `"read_sentence"`（`xml.a.f40647l`） | `PaperUtils.java:1252` |

### 3.4 与 `collector.repeat` 的区别 —— **有字段能区分**

| 维度 | `collector.repeat`（§2） | 对话族（§3.2 `DialoguePaperBean`） |
|---|---|---|
| 来源文件 | `content.json` / content 资源（`contentExtraMap`） | **`paper.Jason`** |
| 切分依据 | `contentExtraMap[exam_id]` 外层条数 | `txlist[0].stlist[0].sublist[]` 条数 |
| 题号 | `question_order = "1".."N"`（连续，重新生成） | `order = sublist[].seq`（**沿用下发 seq**） |
| 分值 | 大题分均摊 + 末题余数 | 每行固定 `maxScore = 1.0f` |
| 角色/时间轴 | **无** | `role` / `roleIndex` / `begintime` / `endtime` |
| 单题 category | 由模板 `evaluation_type` 决定 | 硬编码 `read_sentence` |
| 模板挂载 | `TemplateCtrlBean.subid = "<exam_id>_n"` | `<entityOrder>.ctrl`（按行单独给模板） |
| 家族标识字段 | `exam_type_collect == "collector.repeat"` | 客户端**不比对族名**；靠调用哪个解析入口（`c0`）区分 |

**并且 `collector.repeat` 与对话族在集合分组上是被合并处理的** —— PC 前端 switch 里
`collector.3q5a / role / dialogue / word / repeat` 走同一条 `set3q5aScore` 分支
（见 `00_core_scoring_and_dispatch.md` §2），移动端同样只对 `collector.repeat` 建了 ctrl 切分。

### 3.5 对话族的提交标识字段（教程/CC 通道）

CC 通道的提交参数类是纯 Java POJO（字段名即 JSON key，无 `@SerializedName`）：

```java
// /tmp/jadx_c11/.../model/bean/cc/CcSyncScoreParams.java
private final String audioFileId;
private final String audioFileUrl;
private final String detail;
private final int index;
private final int markService;
private final int maxIndex;
private final int order;
private final int parentAccountId;
private final int questionId;
private final float scoreRate;
private final String sequence;
private final int setId;
private final String xmlFileId;
private final String xmlFileUrl;
```

```java
// /tmp/jadx_c11/.../model/bean/cc/CcSyncScoreData.java
private final int avgPoint;
private final CcSyncScoreParams ccParams;
private final CcStructChildren child;
private final String entityOrder;      // ★ 与 DialoguePaperItemBean.getEntityOrder() 同一语义
private final boolean isRecord;
private final String userAnswer;
private final String userChoose;
private final EtsXmlScoreBean xmlScoreBean;
```

```java
// /tmp/jadx_c11/.../model/bean/cc/CcSyncScoreResult.java
private final int completeRate;
private final int scoreRate;
```

### 3.6 数据模型（CC 通道，层级清晰且全在 Java 里）

```java
// /tmp/jadx_c11/.../model/bean/cc/CcStructSetBean.java
private final List<CcStructQuestion> question;

// /tmp/jadx_c11/.../model/bean/cc/CcStructQuestion.java
private final int category;                                  // ★ 题型分类（int）
private final List<CcStructChildren> children;               // ★ 小题列表
private final String description;
private final int id;

// /tmp/jadx_c11/.../model/bean/cc/CcStructChildren.java
private String content;            // 题面/原文
private String standardAnswer;     // ★ 参考答案（下发）
private List<CcStructOption> option;   // 选项
private String audioUrl; private int audioSize; private int duration;
private String engineType; private String engineArea; private String markType;
private int questionCategory;      // ★ 小题级题型分类
private int questionId; private String questionSetId;
private int order;                 // ★ 题号
private String tips; private String colorText;
private String userAudio; private String userChoose;
private boolean hasScore; private boolean statusRecord; private boolean statusPoint;
private String detailFileUrl; private String uploadFileUrl;
private int indexCurrent; private int indexMax; private int avgPoint;
private SetPprResBean pprResBean;
// + native 计算属性：getEntityOrder() / getPprResId() / getPprMarkType()

// /tmp/jadx_c11/.../model/bean/cc/CcStructOption.java
private String option;   // 选项标识
private String text;     // 选项文本

// /tmp/jadx_c11/.../model/bean/cc/CcTypeAnswer.java
private String answer;   // ★ 答案
private String option;   // ★ 选项
private String type;
```

> **`CcStructChildren.standardAnswer` 是移动端 CC 通道的"参考答案"下发字段**，与 `DialoguePaperItemBean.textContent`
> （`paper.Jason` 通道）并列——两者**都是下发字段**，不是模板固定。

---

## 4. 交叉核对：官方 Android vs 我方 `src/collectors.rs`

### 4.1 差异总表

| # | 项 | 官方 Android（证据） | 我方 | 差异性质 |
|---|---|---|---|---|
| 1 | 族名 `collector.repeat_essay` | **不存在**（11 dex 串表 0 命中；Java 0 命中） | `collectors.rs:16` 定义常量并在 `parse_content_routed` 显式路由（`:1382`） | **我方多出的名字**：官方无此名，我方把它当作官方族 |
| 2 | 族名 `collector.repeat_dialogue` | **不存在** | `collectors.rs:123` 列入 `QUESTION_TYPES`；`spec.rs:206` 单列一条 spec | 同上 |
| 3 | 族名 `collector.essay` | **不存在** | `collectors.rs:124` 列入 `QUESTION_TYPES`；`spec.rs:214` 单列 spec | 同上 |
| 4 | 族名 `collector.dialogue` | **不存在** | `collectors.rs:120,125` 列入 `QUESTION_TYPES`（且重复两次） | 同上 |
| 5 | 实际存在的移动端族名 | `collector.read` / `collector.repeat` / `collector.sentence` / `collector.word`（4 个） | 我方 `spec.rs` 有 `collector.read`/`repeat`/`sentence`/`word` 的 spec，但 handler 会被 4 个不存在的族名抢路由 | 需要纠正：**移动端只有 4 个族名** |
| 6 | 复述/跟读的**切分依据** | `contentExtraMap[exam_id]` 子表条数（`PaperUtils.p0/q0`） | `content.info.sublist[]` 条数（`collectors.rs:540`） | **字段路径不同**：官方是 `exam_id → List<List<InfoExtraBean>>`（按 exam_id 挂载），我方直接读 `info.sublist` |
| 7 | 复述/跟读的**题号** | `question_order = "1".."N"`，由客户端**重新生成** | `no = i+1`（`collectors.rs:546`） | 语义**一致** |
| 8 | 复述/跟读的**分值** | 大题分均摊 + 末题余数（`PaperUtils.q0`） | **不输出分值**（`make_question` 无 score 字段） | **我方缺失** |
| 9 | 对话族来源文件 | **`paper.Jason`** 的 `txlist[0].stlist[0].sublist[]` | 我方没有 `paper.Jason` 路径；`collect_read` 兼容 `info.sublist`（`:460`） | **来源文件不同**；对话族行级字段（role/seq/begintime/endtime）**我方全部未解析** |
| 10 | 对话族角色扮演 | `role` + `roleIndex` + `begintime`/`endtime`（`PaperUtils.java:1238-1249`） | **无对应实现** | **能力缺口** |
| 11 | 对话族单题 category | 硬编码 `"read_sentence"`（`:1252`） | 无 | **能力缺口** |
| 12 | `legacy.rs` 的族分派 | — | `repeat` / `repeat_essay` / `repeat_dialogue` / `essay` / `dialogue` **五个族名全部路由到同一个 `collector_repeat_essay_dialogue`**，而该函数读 `info.question[].std[].value`（`legacy.rs:151-196`） | **与官方完全不重合**：官方复述是 `extra_data` 子表切分 + 下发 `text`；legacy 走 `question/std` |
| 13 | `spec.rs` 的 model 标注 | `InfoObjBean` 字段实为 `audio / choose_type / st_content / st_nr / st_pic / st_sm / stid / xtlist` —— **没有 `sublist`** | `spec.rs:200` 写 `model: "InfoObjBean (sublist[])"` | **注释与官方类不符** |
| 14 | `spec.rs` 的 model 标注（对话） | `DialoguePaperItemBean` 来自 `paper.Jason`，字段是 `order/role/roleIndex/textContent/translate/begintime/endtime/entityId/category/maxScore` | `spec.rs:208` 只写 `model: "DialoguePaperItemBean"` | 缺少来源文件与字段说明（信息不完整，非错误） |
| 15 | `QUESTION_TYPES` 去重 | — | `collectors.rs:118-126` 里 `"collector.dialogue"` **出现两次**（下标 1 与 6） | 冗余（不影响 `contains`） |
| 16 | PC 前端有而移动端**没有**的族 | 移动端缺 `3q5a / choose / dialogue / fill / picture / role` 共 6 个 | 我方 `QUESTION_TYPES` 里有 `3q5a`（`collectors.rs:20`、`spec.rs:183`） | **`collector.3q5a` 在移动端不存在**（dex 串表 0 命中，连 `3q5a` 子串都没有） |

### 4.2 一致性核对（不必改）

| 项 | 官方 | 我方 | 结论 |
|---|---|---|---|
| 「原文即答案」 | `ChapterText`/`phrase_con`/`words_con` 都把 `getText()` 当原文下发；`info.value` 就是朗读文本 | `collect_read` 把文本同时放进 `question` 与 `answers`（`collectors.rs:493-505`） | **一致** |
| `stid` 取 `info.stid` | `ContentJsonInfoBean.stid`；`DialoguePaperBean.stid` | `stid_of()` 读 `content.info.stid`（`collectors.rs:144-150`） | **一致** |
| 词汇 PK 的 `option_name`/`option_value` 结构 | `SyncWordQuestionBean.getOptionData()` 逐个取 `option_name` / `option_value` | `parse_pk_options()` 同上（`collectors.rs:520-535`） | **一致** |

---

## 5. 判分口径核对

### 5.1 `question_type_name` 的存在性：**Android 客户端里没有这个字段**

- 11 dex 字符串表：`grep -cx "question_type_name" /tmp/ets_dex_strings.txt` → **0**
- 全量反编译树（11 dex）`grep -rn "question_type_name"` → **0**
- Android 的真实字段名是 **`question_type`（int）+ `type_name`（String）**，两者独立存在：

```java
// ets源码恢复/jadx_word_struct/sources/com/ets100/secondary/request/word/SyncWordQuestionBean.java:18-35
@SerializedName("material_id")
private int materialId;
private boolean needSync;
private String option;
@SerializedName("question_item_id")
private String questionItemId;
@SerializedName("question_type")
private int questionType;          // ★ 题型数字
private transient SyncWordStructBean syncWordStructBean;
private String translate;
@SerializedName("type_name")
private String typeName;           // ★ 题型名（服务端下发）
private String userAnswer;
private String userAudio;
private float userScoreRate;
@SerializedName("word_id")
private int wordId;
```

```java
// ets源码恢复/jadx_models/sources/com/ets100/superword/api/data/WordQuestionEntity.java:53-70
private final String mark_type;
private final int material_id;
private final String option;
private final String phonics;
private final int question_type;   // ★
private final int rate;
private final int total_point;
private final String translate;
private final String type_name;    // ★
private final int word_id;
```

- `type_name` 在 Android 侧**没有任何使用点**（`getTypeName()` 只有定义，无调用者；
  唯一的 `String typeName = activeNetworkInfo.getTypeName()` 是讯飞 SDK 的网络类型，无关）。
- 客户端也**不含**任何中文题型名（`grep -c "听音选义\|默写词汇\|朗读词汇\|听写词汇\|英译中\|中译英" /tmp/ets_dex_strings.txt` → 0）。
- PC 前端才有 `question_type_name`，而且在词课资源助手这一条路径上**恒为空串**：

```js
// ets源码恢复/pc_client_web/chunks/wordCourseSourceHelpers-f63ae392.js
... {id:a.resourceQuestionId(e), question_type:Number(t), option:r??"", answer:a.answer(e)??"",
     translate:a.translate(e), content:"", audio_url:"", audio_size:0, phonics:"", stem:"",
     question_type_name:""}
```

**结论（严格）**：
- 「6=听音选义 / 16=默写词汇 / 17=朗读词汇 / 18=听写词汇 / 19=英译中 / 20=中译英」
  这组映射**不是客户端硬编码的**，而是服务端下发的 `question_type_name`（PC）/ `type_name`（Android）里的字符串。
  单靠反编译**无法证实**这 6 个名字，只能证实数字字段与 UI 分支。
- 我方 `spec.rs:263-272` 的注释「官方名以真实载荷里的 `question_type_name` 为准」，
  在 Android 侧应改写为 `type_name`；`question_type_name` 只存在于 PC/web 数据模型。

### 5.2 数字 `question_type` → UI 分支（**唯一能静态证实的映射**）

`SyncWordQuestionBean` 的 9 个 `isType*()` 谓词里，**全 Java 树中只有 5 个有调用点**：

| 谓词 | 调用点 | 含义（由 UI 分支反推） |
|---|---|---|
| `isTypeListenChosen()` | `h.java:100,444,519,530` | 只显示音频按钮、不显示题干 → 听音类选择 |
| `isTypeEnToCn()` | `h.java:91,444,501`；`WordBattleViewModel.java:248` | 显示音标 + 英文题干（`tv_question_text`） |
| `isTypeCnToEn()` | `h.java:96,449,509`；`WordBattleFragment.java:1005,1020` | 显示中文题干（`tv_question_cn`） |
| `isTypeDictationWord()` | `q.java:272,319,491` | 显示**音频**输入区 → 听写 |
| `isTypeWriteWord()` | `q.java:288,326` | 显示**文字题干 + 提示**输入区 → 默写 |
| ~~`isTypeReadWord()`~~ | **0 调用点** | 名称疑似"朗读词汇" |
| ~~`isTypeSpell()`~~ | **0 调用点** | 名称疑似"拼写" |
| ~~`isTypeHearSpoken()`~~ | **0 调用点** | 名称疑似"听音说词" |
| ~~`isTypeAnalysis()`~~ | **0 调用点** | 名称疑似"错题解析" |
| （无 `isTypeScoreGreat()` 方法） | — | 只有 `isScoreGreat()`，同样 0 调用点 |

```java
// ets源码恢复/jadx_word_struct/.../SyncWordQuestionBean.java:144-178
public boolean isTypeAnalysis()      { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4960}); }
public boolean isTypeCnToEn()        { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4961}); }
public boolean isTypeDictationWord() { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4962}); }
public boolean isTypeEnToCn()        { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4963}); }
public boolean isTypeHearSpoken()    { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4964}); }
public boolean isTypeListenChosen()  { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4965}); }
public boolean isTypeReadWord()      { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4966}); }
public boolean isTypeSpell()         { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4967}); }
public boolean isTypeWriteWord()     { return JniLib.cZ(new Object[]{SyncWordQuestionBean.class, this, 4968}); }
```

**边界（必须承认）**：这 9 个谓词的**函数体是 native（`JniLib.cZ`）**，
所以 **`question_type == N` ↔ 谓词** 的对应关系**无法从 Java 侧静态证实**。
只能证实"UI 用了这几个谓词"，以及"`question_type` 是 int 字段"。

→ 我方 `spec.rs:267-272` 把 `symbol` 写成 `isTypeSpell` / `isTypeSpell/read` / `isTypeDictation`
**与官方实际调用的 `isTypeWriteWord` / `isTypeDictationWord` 不一致**（且 `isTypeSpell` / `isTypeReadWord` 在官方零调用），
建议把 `symbol` 列换成"官方有调用点的谓词名"或标注为不可证实。

### 5.3 官方**答案比较**原文（选择题 / 听音选义 / 英译中 / 中译英）

**比较的是「选项文本 `option_name`」，不是选项字母，也不是选项 id。**

```java
// ets源码恢复/jadx_exam/sources/com/ets100/syncword/ui/fragment/h.java:304-321（类 h，选择型题面）
this.i = this.H.get(i).getOptionName();          // 用户点选的 option_name
answer = this.e.getAnswer();                     // SyncWordQuestionBean.getAnswer()
if (I()) {                                       // I() = 答案已揭示/回看模式
    this.e.setUserAnswer(this.i);
    this.e.setUserScoreRate(this.i.equals(answer) ? 1.0f : 0.0f);   // ★ 文本相等即满分
    this.e.setNeedSync(true);
    ...
}
imageView2.setVisibility(0);
if (this.i.equals(answer)) {                     // ★ 文本相等即判对
    this.e.setUserAnswer(this.i);
    this.e.setUserScoreRate(1.0f);
```

```java
// ets源码恢复/jadx_exam/sources/com/ets100/syncword/ui/fragment/h.java:369-389（回看模式：按 option_name 找正确项下标）
public final void f0() {
    if (J()) {
        this.i = this.e.getUserAnswer();
        int i = -1; int i2 = -1;
        for (int i3 = 0; i3 < this.H.size(); i3++) {
            String optionName = this.H.get(i3).getOptionName();
            if (optionName.equals(this.i)) {
                if (this.i.equals(this.e.getAnswer())) { i = i3; break; }
                i2 = i3;
            } else if (optionName.equals(this.e.getAnswer())) {   // ★ 正确项 = option_name == answer
                i = i3;
            }
        }
        o0(i, true);
        o0(i2, false);
    }
}
```

选项数据的确来源（`option` 是 JSON 字符串，逐项取 `option_name` / `option_value`）：

```java
// ets源码恢复/jadx_word_struct/.../SyncWordQuestionBean.java:69-90
public List<SyncWordOptionBean> getOptionData() {
    ArrayList arrayList = new ArrayList();
    if (!s7.I2(getOption())) {
        try {
            JSONArray jSONArray = new JSONArray(getOption());
            for (int i = 0; i < jSONArray.length(); i++) {
                JSONObject jSONObject = jSONArray.getJSONObject(i);
                if (jSONObject.has("option_name")) {
                    String string = jSONObject.getString("option_name");
                    String string2 = jSONObject.getString("option_value");
                    SyncWordOptionBean syncWordOptionBean = new SyncWordOptionBean();
                    syncWordOptionBean.setOptionName(string);
                    syncWordOptionBean.setOptionValue(string2);
                    arrayList.add(syncWordOptionBean);
                }
            }
        } catch (Exception e) { e.printStackTrace(); }
    }
    return arrayList;
}
```

```java
// /tmp/jadx_c11/.../secondary/model/bean/SyncWordOptionBean.java
public class SyncWordOptionBean {
    private String optionName;
    private String optionValue;
}
```

**PK 对战侧同口径（选项文本比对）：**

```java
// ets源码恢复/jadx_exam/sources/com/ets100/syncword/ui/fragment/WordBattleFragment.java:670-692
SyncWordStructBean syncWordStructBean = b0().t0().get(b0().O());
String answer = ((SyncWordQuestionBean) syncWordStructBean.getQuestion().get(0)).getAnswer();
HomeworkScoreWordBean homeworkScoreWordBean = b0().U().get(Integer.valueOf(syncWordStructBean.getId()));
Iterator it = this.h.getData().iterator();
int i2 = 0;
while (true) {
    if (!it.hasNext()) { i2 = -1; break; }
    else {
        if (Intrinsics.areEqual(((SyncWordOptionBean) it.next()).getOptionName(),
                                homeworkScoreWordBean != null ? homeworkScoreWordBean.getAnswer() : null)) { break; }
        else { i2++; }
    }
}
...
if (Intrinsics.areEqual(homeworkScoreWordBean != null ? homeworkScoreWordBean.getAnswer() : null, answer)) {
    this.l += 10; z = true;
} else { z = false; }
```

```java
// ets源码恢复/jadx_exam/sources/com/ets100/syncword/ui/fragment/WordBattleFragment.java:1059-1064
String answer = ((SyncWordQuestionBean) syncWordStructBean.getQuestion().get(0)).getAnswer();
SyncWordOptionBean syncWordOptionBean = (SyncWordOptionBean) this.this$0.h.getData().get(i);
if (Intrinsics.areEqual(syncWordOptionBean.getOptionName(), answer)) {   // ★ 仍是 option_name
    this.this$0.j++; this.this$0.k += 10;
    iN = this.this$0.b0().N();
} else { this.this$0.j = 0; iN = 0; }
```

### 5.4 官方答案比较原文（默写 16 / 听写 18）

**比较的是「文本」，两边 `toLowerCase()`，且标准答案额外 `trim()`（用户答案**没有** trim）。**

```java
// ets源码恢复/jadx_exam/sources/com/ets100/syncword/ui/fragment/q.java:78-116（类 q，拼写型题面）
private final void f0() {
    this.g++;
    String answer = this.e.getAnswer();
    Intrinsics.checkNotNullExpressionValue(answer, "getAnswer(...)");
    String string = StringsKt.trim(answer).toString();          // ★ 标准答案先 trim
    Locale locale = Locale.ROOT;
    String lowerCase = string.toLowerCase(locale);              // ★ 再 toLowerCase
    Intrinsics.checkNotNullExpressionValue(lowerCase, "toLowerCase(...)");
    if (I()) {                                                  // 回看模式
        this.e.setUserAnswer(this.i);
        ...
        String lowerCase2 = str.toLowerCase(locale);            // ★ 用户答案只 toLowerCase（不 trim）
        syncWordQuestionBean.setUserScoreRate(Intrinsics.areEqual(lowerCase2, lowerCase) ? 1.0f : 0.0f);
        this.e.setNeedSync(true);
        ...
        return;
    }
    ...
    String lowerCase3 = str2.toLowerCase(locale);
    if (Intrinsics.areEqual(lowerCase3, lowerCase)) {           // ★ 文本（忽略大小写）相等即满分
        this.e.setUserAnswer(this.i);
        this.e.setUserScoreRate(1.0f);
```

### 5.5 词汇作业的**提交与判分回传**（`g/homework/score-sync`）

```java
// ets源码恢复/jadx_syncword/sources/com/ets100/syncword/request/SyncWordRepository.java:1255-1280
for (HomeworkScoreWordBean homeworkScoreWordBean : list) {
    Pair pair = TuplesKt.to("score_rate", Boxing.boxInt(homeworkScoreWordBean.getPoint() > 0.0f ? 1 : 0));  // ★ 0/1
    String answer = homeworkScoreWordBean.getAnswer();
    Intrinsics.checkNotNullExpressionValue(answer, "getAnswer(...)");
    Map mapMutableMapOf = MapsKt.mutableMapOf(new Pair[]{
        pair,
        TuplesKt.to("score_detail", syncWordRepository.getWordOptionDetail(answer)),   // ★ {"choose":"<answer>","version":"..."}
        TuplesKt.to("client_time", a0.m())
    });
    t.d(mapMutableMapOf, homeworkScoreWordBean, zB);                                   // 补充 word_id / question_id / item_id 等
    arrayList.add(mapMutableMapOf);
}
Pair pair2 = TuplesKt.to("homework_id", Boxing.boxLong(s7.s2(this.$homeworkId)));
Pair pair3 = TuplesKt.to("resource_id", this.$resourceId);
Pair pair4 = TuplesKt.to("use_time", Boxing.boxInt(this.$useTime));
String strK = this.$seqId;
String str = this.$homeworkId;
if (strK.length() == 0) { strK = EtsUtils2.k(str); }
Map<String, ? extends Object> mapMutableMapOf2 = MapsKt.mutableMapOf(new Pair[]{
    pair2, pair3, pair4,
    TuplesKt.to("seq_id", strK),
    TuplesKt.to("type", Boxing.boxInt(6)),          // ★ 固定 6
    TuplesKt.to("mark_service", Boxing.boxInt(1)),
    TuplesKt.to("detail", arrayList)
});
```

```java
// ets源码恢复/jadx_syncword/sources/com/ets100/syncword/request/SyncWordRepository.java:1732-1749
private final String getWordOptionDetail(String str) {
    StringBuilder sb2 = new StringBuilder();
    sb2.append("{\"choose\": \"");
    sb2.append(str);
    sb2.append("\",\"version\": \"");
    ... // app 版本
    sb2.append("\"}");
    return StringsKt.trimIndent(sb2.toString());
}
```

**服务端回包 → 客户端答案（`dealPkScore`）：**

```java
// ets源码恢复/jadx_syncword/sources/com/ets100/syncword/request/SyncWordRepository.java:1436-1444
homeworkScoreWordBean.setPoint(homeworkScoreWordBean.getScoreRate());     // ★ point = score_rate
JSONObject jSONObject = new JSONObject(homeworkScoreWordBean.getScoreDetail());
if (jSONObject.has("choose")) {
    homeworkScoreWordBean.setAnswer(jSONObject.getString("choose"));      // ★ answer = score_detail.choose
}
```

```java
// /tmp/jadx_c11/.../secondary/request/homework/HomeworkScoreWordBean.java
private String answer;         // ★ 答案（来自 score_detail.choose）
private String audioUrl;
private String clientTime;
private String itemId;
private float point;           // ★ 得分（= scoreRate）
private int questionId;        // ★ 题 id
private String questionItemId; // ★ 题项 id（组卷用）
private String scoreDetail;    // ★ 原始 score_detail JSON
private float scoreRate;
private int wordId;
private String xmlUrl;
```

> 判分口径结论：词汇选择题/拼写题的**判定发生在服务端**（`score_rate` 回传），
> 客户端本地只用 `option_name` / `trim+toLowerCase(text)` 做**即时 UI 反馈**，
> 回看模式下用 `option_name == answer` 定位正确项。

### 5.6 朗读类（17 朗读词汇 / 朗读句子 / 朗读短文）的判分口径：**不是文本比对，是引擎 XML**

```java
// ets源码恢复/jadx_score/sources/com/ets100/secondary/xml/a.java:423-457（类 a，方法 r()）
public final void r() {
    if (this.f40653b == null) { return; }
    int attributeCount = this.f40652a.getAttributeCount();
    for (int i10 = 0; i10 < attributeCount; i10++) {
        this.f40654c = this.f40652a.getAttributeName(i10);
        this.f40655d = this.f40652a.getAttributeValue(i10);
        if ("content".equals(this.f40654c))          { this.f40653b.setContent(this.f40655d); }
        else if ("total_score".equals(this.f40654c))     { this.f40653b.setTotalScore(j(this.f40655d)); }
        else if ("accuracy_score".equals(this.f40654c))  { this.f40653b.setAccuracyScore(j(this.f40655d)); }
        else if ("fluency_score".equals(this.f40654c))   { this.f40653b.setFluencyScore(j(this.f40655d)); }
        else if ("integrity_score".equals(this.f40654c)) { this.f40653b.setIntegrityScore(j(this.f40655d)); }
        else if ("standard_score".equals(this.f40654c))  { this.f40653b.setStandardScore(j(this.f40655d)); }
        else if ("stress_score".equals(this.f40654c))    { this.f40653b.setStressScore(j(this.f40655d)); }
        else if ("is_rejected".equals(this.f40654c))     { this.f40653b.setReject(b.V.equals(this.f40655d)); }
        else if ("except_info".equals(this.f40654c))     { this.f40653b.setExceptInfo(this.f40655d); }
        else if ("words_per_min".equals(this.f40654c))   { this.f40653b.setWordsPerMin(s7.q2(this.f40655d)); }
        else if ("beg_pos".equals(this.f40654c))         { this.f40653b.setBegPos(s7.m2(this.f40655d)); }
        else if ("end_pos".equals(this.f40654c))         { this.f40653b.setEndPos(s7.m2(this.f40655d)); }
    }
}
```

```java
// ets源码恢复/jadx_score/sources/com/ets100/secondary/xml/a.java:274-300（同一个 XML 按根标签分流 category）
while (eventType != 1) {
    if (eventType == 2) {
        String name = this.f40652a.getName();
        if (f40651p.equals(name)) {            // "xml_result" → 第二层标签名就是 category
            this.f40652a.nextTag();
            String name2 = this.f40652a.getName();
            ...
            str = name2;                        // ★ category = read_word / read_chapter / read_sentence / sentence / word
        } else if (f40644i.equals(name)) {      // "rec_paper"
            EtsXmlResultBean etsXmlResultBean = new EtsXmlResultBean();
            this.f40653b = etsXmlResultBean;
            etsXmlResultBean.setVersion(attributeValue);
            this.f40653b.setCategory(str);
            r();
        } else if (f40645j.equals(name) || f40646k.equals(name)) {  // "read_word" || "read_chapter"
            r();
        } else if (f40650o.equals(name)) {      // "sentence"
            t();
        } else if (f40648m.equals(name)) {      // "word"
            u();
        }
    }
    eventType = this.f40652a.next();
}
```

配合 §3.2 的 `dialoguePaperItemBean.setCategory(com.ets100.secondary.xml.a.f40647l)`（=`"read_sentence"`）以及
`pe.p.U()` 的 `words_con / phrase_con / essay_con`，可以确认：**朗读/跟读/复述类全部复用同一套评测 XML 口径**
（`total_score` 为主分，`accuracy/fluency/integrity/standard/stress` 为维度分，`is_rejected`/`except_info` 为异常，
`words_per_min`/`beg_pos`/`end_pos` 为副信息），**客户端不保存"参考范文"用于比对**。

### 5.7 PC 前端的对照（供口径统一）

PC 前端 `collector.choose` 比较的是**选项字母换算后的数字串**（与我方 Android 证实的"选项文本"口径不同）：

```js
// ets源码恢复/pc_client_web/chunks/useExamResult-d9fb4671.js
function b(e){if(!e)return"0";var t=e.toUpperCase(),o="",i=0;
  for(;i<t.length;i++){var r=t.charCodeAt(i);r>=65&&r<=90?o+=(r-64).toString():o+=t[i]}return o}
...
case"collector.choose":
  ... const i=b(t.info.xtlist.find(e=>e.xt_xh===""+o.order)?.answer);      // 标准答案 → 数字
      return o.detail.choose
        ? e.push(`${o.entity_id}_${o.order}_${b(o.detail.choose)}${String.fromCharCode(1)}${i}`)   // 用户答案 → 数字
        : e.push(`${o.entity_id}_${o.order}${q}${i}`)
```

**注意**：PC 的 `collector.choose`（题目答案在 `info.xtlist[].answer`）与 Android 的同步词汇选择题
（答案在 `SyncWordQuestionBean.answer` + `option_name`）**是两套不同的载荷**，不要混用口径。

---

## 6. PC 端 vs 移动端的族模型差异

| | PC 前端（`pc_client_web`） | Android 官方客户端 |
|---|---|---|
| `collector.*` 族名字面量 | 9 个：`3q5a / choose / dialogue / fill / picture / read / repeat / role / word` | **4 个**：`read / repeat / sentence / word` |
| `repeat_essay` / `repeat_dialogue` | 无 | **无** |
| `collector.sentence` | **无** | **有**（`PaperUtilsKt.java:421`） |
| `collector.3q5a` | 有 | **无**（dex 中连 `3q5a` 子串都不存在） |
| 分派实现 | 明文 JS `switch(exam_type_collect)` | Java 只比较 4 个；总分派 `PaperUtils.d()` → **native** |
| 逐句/跟读数据 | `info.fj[]`（PC JS） | `paper.Jason` → `sublist[]`（`PaperUtils.c0`） |
| 移动端独有 UI | — | 教程模块 9 个 `CcExam*Frg`（含 `SpeakComposition`/`SpeakDialogue`/`SpeakSentence`） |

### 6.1 移动端是否存在"独立族"

**存在，但不是以 `collector.*` 名字存在的。** 移动端有两套并行的题型体系：

1. **资源平台族**（`exam_type_collect`，服务端下发，客户端只认 4 个字面量）。
2. **教程 / CC 模块**（`com.ets100.secondary.tutorial.ui.cc`）：9 个独立 Fragment，
   由 `CcStructQuestion.category`（int）+ `CcStructChildren.questionCategory`（int）驱动，
   **与 `collector.*` 完全无关**，数据走 `g/set/*`（`CcStructSetBean`）与 `CcListBody`/`CcSyncScore*`。

并且 `ChildPaperItemBean.type` 的取值域（dex 中精确存在的 token）是：

```text
choose  essay  fill  picture  read  repeat  role  sentence  topic  word
（注意：不存在 dialogue / 3q5a / repeat_essay / repeat_dialogue）
```

这些 token 的比较逻辑全在 `com.ets100.secondary.utils.h7` 的 native 桩里，**Java 侧无任何比较点**：

```java
// /tmp/jadx_c11/sources/com/ets100/secondary/utils/h7.java:9-21
public static boolean a(java.lang.String type) { return com.fort.andjni.JniLib.cZ(new Object[]{h7.class, type, 15812}); }
public static boolean b(java.lang.String type) { return com.fort.andjni.JniLib.cZ(new Object[]{h7.class, type, 15813}); }
public static boolean c(java.lang.String type) { return com.fort.andjni.JniLib.cZ(new Object[]{h7.class, type, 15814}); }
public static boolean d(java.lang.String type) { return com.fort.andjni.JniLib.cZ(new Object[]{h7.class, type, 15815}); }
// ... a()~y() 共 25 个，全部 native
```

### 6.2 "是否有字段能区分它们与 `repeat`"

**有，且有三层判别信号（按可靠性排序）：**

1. **文件层**：`paper.Jason` vs `content.json` —— `PaperUtils.c0` 只解析 `paper.Jason`，
   `PaperUtils.d/e/p0/q0` 只处理 `content.json` / `SetMockStructBean`。**这是最硬的判别点。**
2. **字段层**：出现 `sublist[].role` / `roleIndex` / `begintime` / `endtime` → 对话/角色扮演；
   只出现 `contentExtraMap` 子表 + `subid="<exam_id>_n"` → `collector.repeat`。
3. **分值层**：每行 `maxScore = 1.0f` + `category = "read_sentence"` → 对话族；
   `question_score = round2(大题分/题数)` → `collector.repeat`。

**不成立的判别信号**：`exam_type_collect == "collector.repeat_dialogue"` —— 官方客户端从不下发也不比较这个名字。

---

## 7. 交付物边界 / 未解决项

| # | 未解决项 | 原因 | 已尝试 |
|---|---|---|---|
| 1 | `question_type(int) ↔ isTypeXxx()` 的精确对应 | 9 个谓词体全在 `JniLib`（native 壳），Java 是空桩 | 全量反编译 11 dex；找 `JniLib` 定义（未导出）；找 `.so` 反汇编（不在本次范围） |
| 2 | `ChildPaperItemBean.type` 各 token 归哪个族 | `h7.a()~y()` 25 个静态方法全 native | 全量 Java 反编译树无比较点；dex 串表只能证明 token 存在 |
| 3 | CC 模块 `category` 数字 → Fragment 的映射 | `BaseCcFrg`/`CcExamAct`/`CcExamViewModel` 方法体全 native | `CcExamAct.java` 240 行、`CcExamViewModel.java` 463 行，逐行确认全为 `JniLib.cV/cL` |
| 4 | `InfoExtraBean` 字段定义 | 类本体在 `jadx_*` 未导出；但我已用全量 dex 反编译确认它**不在** classes/classes2..11 的任何 java 输出里可读（仅被引用） | `find / -name "InfoExtraBean.java"`；全量反编译树 grep |
| 5 | 「6=听音选义 … 20=中译英」中文名 | 客户端**不含**这些中文字符串（dex 串表 0 命中）；名字由服务端下发 | dex 串表精确 grep；全量反编译树 grep；PC 前端 JS grep |
| 6 | `fj`（PC `collector.repeat` 的 `info.fj`）在移动端对应物 | dex 中 `fj` 无任何 Java 引用 | 全量反编译树 `grep -rn '"fj"'` → 唯一命中是 mp4parser 语种码表 |

---

## 8. 复现命令（全部离线、只读）

```bash
# 0) 目录概览
ls -d /home/shuaiqiu/Projects/ets源码恢复/*/

# 1) 明文 Java 全树检索
cd /home/shuaiqiu/Projects/ets源码恢复
grep -rn "repeat_essay\|repeatEssay\|REPEAT_ESSAY" --include="*.java" .          # → 空
grep -rn "repeat_dialogue\|repeatDialogue\|REPEAT_DIALOGUE" --include="*.java" .  # → 空
grep -rn "question_type_name" --include="*.java" .                                # → 空
grep -rhoE '"collector\.[a-z_0-9]+"' --include="*.java" . | sort -u
#   → collector.read / collector.repeat / collector.sentence / collector.word

# 2) dex 字符串表（最硬证据，不受 jadx 覆盖范围影响）
cd /home/shuaiqiu/Projects/ets源码恢复/7c049f4b45298848ef1137b58e393915
for s in repeat_essay repeat_dialogue question_type_name collector.repeat collector.read; do
  echo -n "$s -> "; grep -a -l "$s" *.dex | tr '\n' ' '; echo
done
for f in *.dex; do strings -n 6 "$f"; done | grep -o "collector\.[a-z_0-9]*" | sort -u

# 3) 补齐 jadx 缺失类（写到 /tmp，不动任何已有目录）
jadx -d /tmp/jadx_c11 -j 4 --no-res --no-imports .../classes11.dex
grep -rn "CcExamSpeak" /tmp/jadx_c11/sources | head
```

---

## 9. 对我方实现的**具体**修改建议（仅建议，未改动任何文件）

1. `src/spec.rs`
   - `STRUCTURE_TYPES` 中 `collector.repeat_essay` 的 `model` 改为
     "PaperUtils.p0/q0 + contentExtraMap（List<List<InfoExtraBean>>）"，**不要**写 `InfoObjBean (sublist[])`
     —— 官方 `InfoObjBean` 无 `sublist` 字段。
   - `collector.repeat_dialogue` 的 `model` 补上来源：`paper.Jason → txlist[0].stlist[0].sublist[]`，
     并注明客户端**不比较**该族名。
   - `collector.dialogue` / `collector.essay` / `collector.3q5a` 增加 `note`：
     "PC 前端族名；Android 客户端 dex 中不存在该字面量"（`3q5a` 在移动端完全不存在）。
   - `WORD_SYMBOLS` 的 `symbol` 列：16/17/18 改成官方**有调用点**的谓词
     （`isTypeWriteWord` / `isTypeReadWord`(零调用，标注) / `isTypeDictationWord`），
     或直接把该列标注为"官方 native 谓词名，数字对应不可静态证实"。
   - `WORD_TYPE_NAME_MAP` 的注释：`question_type_name` → Android 侧字段名为 `type_name`。
2. `src/collectors.rs`
   - `collect_repeat_essay` 增加**官方路径**兼容：`info.txlist[0].stlist[0].sublist[]`（`paper.Jason` 形态），
     并解析 `seq`（题号）/ `role` / `roleIndex` / `begintime` / `endtime` / `translate`。
   - `collect_repeat_essay`/`collect_read` 输出**分值字段**（均分 + 末题余数），与官方 `PaperUtils.q0` 对齐。
   - `QUESTION_TYPES` 去掉重复的 `"collector.dialogue"`（下标 1 与 6 相同）。
   - 显式路由 `st == TYPE_REPEAT_ESSAY` 应**降级为别名**（因为官方无此名），
     并把 `st == "collector.repeat"` 提到与之一致的位置，避免依赖不存在的族名。
3. `src/legacy.rs`
   - `collector_repeat_essay_dialogue` 目前读 `info.question[].std[].value`，与官方 `extra_data/sublist` 路径无关；
     建议在文档/注释中标注"legacy 路径，非官方口径"，或在 `resolve_inner` 里把 5 个族名收窄到官方 2 个族
     （`collector.repeat` / `collector.read`）。
