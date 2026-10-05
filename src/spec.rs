// -*- coding: utf-8 -*-
// 官方权威规格注册表 (spec)
//
// 来源：ETS 客户端反编译源码（QuestionSymbol / SyncWordQuestionBean / PaperUtils /
// ChildPaperItemBean / TemplateResBean / JsQuestionParamBean 等）。
// 这里把"官方定义域"固化成机器可读数据，用途有三：
//   1. capabilities 能对外给出权威题型全集（客户端不再各自硬编码枚举）；
//   2. 覆盖率自证：每个官方题型是"已显式适配 / 靠结构嗅探兜住 / 未适配"一目了然；
//   3. diagnose 能把"官方枚举里没有的 structure_type"标成新东西（跟进官方更新）。
//
// 只放"事实"，不放行为：解析行为仍在 collectors.rs / template.rs。

use serde_json::{json, Value};

/// 官方 structure_type / 引擎标识 的规格。
#[derive(Debug, Clone, Copy)]
pub struct StructureTypeSpec {
    /// 官方取值（content.json 的 `structure_type`，或 res.json 的 `exam_type_collect`）
    pub id: &'static str,
    /// 官方中文名 / 模块定位
    pub official_name: &'static str,
    /// 对应核心解析模型（反编译来源，便于回溯）
    pub model: &'static str,
    /// 典型适用场景
    pub scenario: &'static str,
    /// 当前实现命中的解析家族（None = 没有显式适配，只能靠结构嗅探）
    pub handler: Option<&'static str>,
    /// 备注 / 已知边界
    pub note: &'static str,
}

/// 官方全量题型表（★P0-1 权威定义域）。
pub const STRUCTURE_TYPES: &[StructureTypeSpec] = &[
    StructureTypeSpec {
        id: "collector.read",
        official_name: "朗读训练集合",
        model: "SyncTextListBody / PaperUtilsKt",
        scenario: "单词/句子/短文朗读集合",
        handler: Some("collector.read"),
        note: "朗读文本同时作为 answers 输出（官方不展示\"参考答案\"面板）",
    },
    StructureTypeSpec {
        id: "collector.repeat",
        official_name: "跟读/复述训练集合",
        model: "DialoguePaperBean / PaperUtilsKt",
        scenario: "对话角色扮演、多轮跟读",
        handler: Some("collector.repeat"),
        note: "",
    },
    StructureTypeSpec {
        id: "collector.sentence",
        official_name: "句子专项训练",
        model: "SyncTextListBody",
        scenario: "句子听读、模仿",
        handler: Some("collector.read"),
        note: "文本列表形态，按朗读/文本集合处理",
    },
    StructureTypeSpec {
        id: "collector.word",
        official_name: "词汇专项训练",
        model: "SyncTextListBody",
        scenario: "单词认读与拼读",
        handler: Some("collector.read"),
        note: "这是\"词汇文本训练\"通道，不是词汇PK通道（PK 走 word_question/words 载荷）",
    },
    StructureTypeSpec {
        id: "repeat",
        official_name: "课本同步跟读",
        model: "BookRepeatRepository",
        scenario: "教材单元课文跟读",
        handler: Some("repeat"),
        note: "",
    },
    StructureTypeSpec {
        id: "repeat_dialogue",
        official_name: "对话角色扮演朗读",
        model: "DialoguePaperItemBean",
        scenario: "角色扮演、对话交际",
        handler: Some("repeat_dialogue"),
        note: "",
    },
    StructureTypeSpec {
        id: "essay",
        official_name: "篇章朗读/背诵",
        model: "TextEssayBean",
        scenario: "长篇故事、散文背诵",
        handler: Some("essay"),
        note: "",
    },
    StructureTypeSpec {
        id: "templet_common",
        official_name: "通用中高考题型模板",
        model: "TemplateCtrlBean / ContentJson",
        scenario: "广东等各省标准化模拟卷",
        handler: None,
        note: "题目结构由 res.json 模板驱动，需要 template.rs 协同（M2）",
    },
    StructureTypeSpec {
        id: "listen_choice_standard",
        official_name: "标准听力选择题",
        model: "SectionItemBean",
        scenario: "听选信息 / 听短文选择",
        handler: Some("collector.choose"),
        note: "",
    },
    StructureTypeSpec {
        id: "listen_fill_standard",
        official_name: "标准听力填空题",
        model: "ChildHtmlParam (InfoArrayBean)",
        scenario: "听选填空 / 听填信息",
        handler: Some("collector.fill"),
        note: "小题空号权威来源是 std[].xth",
    },
    StructureTypeSpec {
        id: "read_passage_standard",
        official_name: "标准短文朗读",
        model: "ContentJson (st_nr)",
        scenario: "模仿朗读 / 短文朗读",
        handler: Some("collector.read"),
        note: "",
    },
    StructureTypeSpec {
        id: "read_sentence_standard",
        official_name: "标准句子朗读",
        model: "ContentJson (value)",
        scenario: "句子朗读",
        handler: Some("collector.read"),
        note: "",
    },
    StructureTypeSpec {
        id: "read_word_standard",
        official_name: "标准单词朗读",
        model: "ContentJson (value)",
        scenario: "单词跟读 / 朗读",
        handler: Some("collector.read"),
        note: "",
    },
    StructureTypeSpec {
        id: "read_and_choose_standard",
        official_name: "看读选择题",
        model: "InfoObjBean (xtlist/xxlist)",
        scenario: "看图选句 / 阅读理解选择",
        handler: Some("collector.choose"),
        note: "",
    },
    // ---- content.json 里长期在用的 collector.* 规范取值 ----
    // 官方 res.json 的 `exam_type_collect` 也会直接用这些名字，所以它们同属权威定义域；
    // 不收录会导致 `spec_known` 对最常见的题型误报为"未知"。
    StructureTypeSpec {
        id: "collector.choose",
        official_name: "听后选择",
        model: "InfoArrayBean (xtlist/xxlist)",
        scenario: "北京初中/高中 听后选择；广东 听选信息",
        handler: Some("collector.choose"),
        note: "答案在 info.xtlist[].answer，选项在 xtlist[].xxlist",
    },
    StructureTypeSpec {
        id: "collector.role",
        official_name: "听选信息 / 回答问题 / 提问",
        model: "InfoArrayBean (question[].std[])",
        scenario: "广东初中 听选信息、回答问题、询问信息",
        handler: Some("collector.role"),
        note: "同族三题型靠 res.json 的 exam_type_name 区分，无模板时按 askaudio/中文兜底",
    },
    StructureTypeSpec {
        id: "collector.picture",
        official_name: "信息转述",
        model: "InfoObjBean (std[].value)",
        scenario: "各地区 信息转述 / 听后转述",
        handler: Some("collector.picture"),
        note: "",
    },
    StructureTypeSpec {
        id: "collector.fill",
        official_name: "听后记录",
        model: "InfoArrayBean (std[].xth/value)",
        scenario: "北京高中 听后记录/填空",
        handler: Some("collector.fill"),
        note: "空号权威来源是 std[].xth；卷面大题号另有 th",
    },
    StructureTypeSpec {
        id: "collector.3q5a",
        official_name: "三问五答",
        model: "InfoArrayBean (question[].std[])",
        scenario: "广东高中",
        handler: Some("collector.3q5a"),
        note: "每题多个参考答案，原样不排序",
    },
    StructureTypeSpec {
        id: "collector.dialogue",
        official_name: "回答问题",
        model: "InfoArrayBean (question[].std[])",
        scenario: "北京高中 回答问题",
        handler: Some("collector.dialogue"),
        note: "",
    },
    StructureTypeSpec {
        id: "collector.repeat_essay",
        official_name: "短文复述",
        model: "InfoObjBean (sublist[])",
        scenario: "短文复述 / 篇章背诵",
        handler: Some("collector.repeat_essay"),
        note: "每条 sublist 一项成题",
    },
    StructureTypeSpec {
        id: "collector.repeat_dialogue",
        official_name: "对话角色扮演（跟读）",
        model: "DialoguePaperItemBean",
        scenario: "角色扮演、对话交际（模板 exam_type_collect 实测取值）",
        handler: Some("repeat_dialogue"),
        note: "官方模板里以 collector.repeat_dialogue 出现（对话跟读类）",
    },
    StructureTypeSpec {
        id: "collector.essay",
        official_name: "篇章朗读/背诵",
        model: "TextEssayBean",
        scenario: "长篇故事、散文背诵（模板 exam_type_collect 实测取值）",
        handler: Some("essay"),
        note: "",
    },
    StructureTypeSpec {
        id: "word_pk",
        official_name: "词汇 / PK 在线题",
        model: "SyncWordQuestionBean",
        scenario: "单词对战、词汇在线作业（无资源包）",
        handler: Some("word_pk"),
        note: "解析器内部统一标识；question_type 语义见 word_symbols（双通道）",
    },
];

/// 词汇侧题型（双套系统）。`ambiguous = true` 表示同一 `question_type` 在两个系统里语义不同，
/// **必须按通道判定**，不能只看数字。
#[derive(Debug, Clone, Copy)]
pub struct WordSymbolSpec {
    pub qtype: i64,
    /// `symbol` 新版超能单词 (QuestionSymbol) / `sync_pk` 同步词汇与PK (SyncWordQuestionBean)
    pub channel: &'static str,
    pub symbol: &'static str,
    pub official_name: &'static str,
    pub code: &'static str,
    pub ambiguous: bool,
}

/// 官方词汇题型全集（★P0-1）。
pub const WORD_SYMBOLS: &[WordSymbolSpec] = &[
    // --- 新版超能单词 QuestionSymbol.java（0~13）---
    WordSymbolSpec { qtype: 0, channel: "symbol", symbol: "Symbol_None", official_name: "默认", code: "", ambiguous: false },
    WordSymbolSpec { qtype: 1, channel: "symbol", symbol: "Symbol_01_ktxy", official_name: "看图选义", code: "ktxy", ambiguous: false },
    WordSymbolSpec { qtype: 2, channel: "symbol", symbol: "Symbol_02_dcllk", official_name: "单词连连看", code: "dcllk", ambiguous: false },
    WordSymbolSpec { qtype: 3, channel: "symbol", symbol: "Symbol_03_tyxc", official_name: "听音选词", code: "tyxc", ambiguous: false },
    WordSymbolSpec { qtype: 4, channel: "symbol", symbol: "Symbol_04_tydc", official_name: "听音读词", code: "tydc", ambiguous: false },
    WordSymbolSpec { qtype: 5, channel: "symbol", symbol: "Symbol_05_fytx", official_name: "单词朗读", code: "dcld", ambiguous: false },
    WordSymbolSpec { qtype: 6, channel: "symbol", symbol: "Symbol_06_dcgd", official_name: "单词跟读", code: "dcgd", ambiguous: true },
    WordSymbolSpec { qtype: 7, channel: "symbol", symbol: "Symbol_07_jzgd", official_name: "句子跟读", code: "jzgd", ambiguous: false },
    WordSymbolSpec { qtype: 8, channel: "symbol", symbol: "Symbol_08_cytz", official_name: "单词选义", code: "cytz", ambiguous: false },
    WordSymbolSpec { qtype: 9, channel: "symbol", symbol: "Symbol_09_cfxhc", official_name: "拆分货运车（自然拼读音节拆分）", code: "cfxhc", ambiguous: false },
    WordSymbolSpec { qtype: 10, channel: "symbol", symbol: "Symbol_10_pdxhc", official_name: "拼读货运车（自然拼读拼读合成）", code: "pdxhc", ambiguous: false },
    WordSymbolSpec { qtype: 11, channel: "symbol", symbol: "Symbol_11_dcpx", official_name: "单词拼写", code: "dcpx", ambiguous: false },
    WordSymbolSpec { qtype: 12, channel: "symbol", symbol: "Symbol_12_qjyy", official_name: "情景应用", code: "qjyy", ambiguous: false },
    WordSymbolSpec { qtype: 13, channel: "symbol", symbol: "Symbol_13_xdc", official_name: "学单词", code: "xdc", ambiguous: false },
    // --- 同步词汇 / PK 对战 SyncWordQuestionBean（6、16~20）---
    WordSymbolSpec { qtype: 6, channel: "sync_pk", symbol: "isTypeListenChosen", official_name: "听音选词（4选1）", code: "listen_chosen", ambiguous: true },
    WordSymbolSpec { qtype: 16, channel: "sync_pk", symbol: "isTypeSpell", official_name: "单词拼写", code: "spell", ambiguous: false },
    WordSymbolSpec { qtype: 17, channel: "sync_pk", symbol: "isTypeEnToCn", official_name: "英译中（看英文选中文）", code: "en_cn", ambiguous: false },
    WordSymbolSpec { qtype: 18, channel: "sync_pk", symbol: "isTypeCnToEn", official_name: "中译英（看中文选英文）", code: "cn_en", ambiguous: false },
    WordSymbolSpec { qtype: 19, channel: "sync_pk", symbol: "isTypeWriteWord", official_name: "看中文写单词", code: "write_word", ambiguous: false },
    WordSymbolSpec { qtype: 20, channel: "sync_pk", symbol: "isTypeDictationWord", official_name: "听音拼写（单词听写）", code: "dictation", ambiguous: false },
];

/// `question_type` 是否在两个通道里语义冲突（当前只有 6）。
pub fn is_ambiguous_qtype(qtype: i64) -> bool {
    WORD_SYMBOLS
        .iter()
        .any(|s| s.qtype == qtype && s.ambiguous)
}

/// 按通道取词汇题型规格；`channel` 为 `symbol` / `sync_pk`。
pub fn word_symbol(qtype: i64, channel: &str) -> Option<&'static WordSymbolSpec> {
    WORD_SYMBOLS
        .iter()
        .find(|s| s.qtype == qtype && s.channel == channel)
}

/// 广东省中考听说 `res.json` 的 `exam_type_name` 官方全集（★P0-3）。
pub const EXAM_TYPE_NAMES_AUTHORITATIVE: [&str; 5] =
    ["模仿朗读", "听选信息", "回答问题", "信息转述", "询问信息"];

/// 部分地市 / 校本卷常见别名（官方同一大类的不同叫法）。
pub const EXAM_TYPE_NAME_ALIASES: [&str; 7] = [
    "短文朗读",
    "情景反应",
    "故事复述",
    "听后选择",
    "听后填空",
    "听填信息",
    "口头表达",
];

/// 每类题型的官方展示要素（★P0-2）。`image` 取 `yes` / `no` / `optional`。
#[derive(Debug, Clone, Copy)]
pub struct DisplayElements {
    pub group: &'static str,
    pub question: bool,
    pub audio: bool,
    pub image: &'static str,
    pub options: bool,
    pub answers: bool,
    pub ref_text: bool,
    pub keypoints: bool,
    pub analyze: bool,
    /// 答案形态：`text` 文本 / `letter` 选项字母 / `text_list` 多条文本
    pub answer_kind: &'static str,
}

pub const DISPLAY_ELEMENTS: &[DisplayElements] = &[
    DisplayElements { group: "模仿朗读/短文朗读", question: true, audio: true, image: "no", options: false, answers: true, ref_text: false, keypoints: true, analyze: true, answer_kind: "text_list" },
    DisplayElements { group: "听选信息(单选)", question: true, audio: true, image: "optional", options: true, answers: true, ref_text: true, keypoints: false, analyze: true, answer_kind: "letter" },
    DisplayElements { group: "回答问题(简答)", question: true, audio: true, image: "optional", options: false, answers: true, ref_text: true, keypoints: true, analyze: true, answer_kind: "text_list" },
    DisplayElements { group: "信息转述(复述)", question: true, audio: true, image: "yes", options: false, answers: true, ref_text: true, keypoints: true, analyze: true, answer_kind: "text_list" },
    DisplayElements { group: "询问信息(提问)", question: true, audio: false, image: "no", options: false, answers: true, ref_text: false, keypoints: true, analyze: true, answer_kind: "text_list" },
    DisplayElements { group: "听填信息(填空)", question: true, audio: true, image: "optional", options: false, answers: true, ref_text: true, keypoints: false, analyze: true, answer_kind: "text_list" },
    DisplayElements { group: "单词PK/选择类", question: false, audio: true, image: "optional", options: true, answers: true, ref_text: false, keypoints: false, analyze: true, answer_kind: "letter" },
    DisplayElements { group: "单词PK/拼写类", question: false, audio: true, image: "no", options: true, answers: true, ref_text: false, keypoints: false, analyze: true, answer_kind: "text" },
];

/// `question_list[].evaluation_type` 的官方取值域（实测自 221 套真实模板）。
///
/// 用途：客户端据此选择展示/评分通道（朗读类给原文、选择题给选项…），
/// 解析器只做搬运与归一，不解释评分。
pub const EVALUATION_TYPES: [&str; 11] = [
    "read_word",              // 单词朗读
    "read_sentence",          // 句子朗读
    "read_chapter",           // 篇章朗读
    "read_choice",            // 看读选择
    "choose",                 // 选择题
    "Fill_in_the_blanks",     // 填空/听后记录（注意官方首字母大写）
    "simple_expression",      // 简答/口头表达
    "simple_expression_ufk",  // 简答变体（口语评分引擎分组）
    "simple_expression_ufi",
    "simple_expression_ufj",
    "topic",                  // 话题/转述
];

/// `question_list[].engine_type` 的官方取值域（实测）。
pub const ENGINE_TYPES: [&str; 2] = ["study", "Simulation"];

/// res.json 模板的权威字段路径（★P0-4）。
pub const RES_JSON_PATHS: &[(&str, &str)] = &[
    ("section_order", "exam_type_list[] 数组下标顺序"),
    ("section_collect", "exam_type_list[].exam_type_collect（对应 content.json 的 structure_type）"),
    ("section_score", "exam_type_list[].exam_list[].exam_score（大题总分）"),
    ("question_score", "exam_type_list[].exam_list[].question_list[].question_score（小题满分）"),
    ("question_order", "exam_type_list[].exam_list[].question_list[].question_order"),
    ("blank_number", "info.std[].xth（小题/空格号）"),
    ("paper_number", "info.std[].th（卷面大题号，存在才带）"),
    ("prompt_text", "exam_info[].value（是大题卷面指示文本，不是题号）"),
    ("evaluation", "question_list[].evaluation_type / engine_type"),
];

/// 按 id 查题型规格（也接受别名匹配，大小写不敏感）。
pub fn find_structure_type(id: &str) -> Option<&'static StructureTypeSpec> {
    let want = id.trim();
    STRUCTURE_TYPES
        .iter()
        .find(|s| s.id.eq_ignore_ascii_case(want))
}

/// 覆盖率自证：官方题型里有多少已经显式适配。
pub fn coverage() -> Value {
    let total = STRUCTURE_TYPES.len();
    let mut adapted: Vec<Value> = Vec::new();
    let mut sniff_only: Vec<Value> = Vec::new();
    for s in STRUCTURE_TYPES {
        let entry = json!({
            "id": s.id,
            "official_name": s.official_name,
            "handler": s.handler,
        });
        if s.handler.is_some() {
            adapted.push(entry);
        } else {
            sniff_only.push(entry);
        }
    }
    json!({
        "total": total,
        "explicitly_adapted": adapted.len(),
        "not_explicitly_adapted": sniff_only.len(),
        "explicitly_adapted_list": adapted,
        "not_explicitly_adapted_list": sniff_only,
        "note": "未显式适配不等于解析不了：仍会走结构嗅探(collect_generic)，但需要 res.json/样例确认",
    })
}

/// 完整权威规格（capabilities 与 op `spec` 都返回这个）。
pub fn taxonomy() -> Value {
    json!({
        "source": "ETS 客户端反编译: QuestionSymbol / SyncWordQuestionBean / PaperUtils / \
                   ChildPaperItemBean / TemplateResBean / JsQuestionParamBean",
        "structure_types": STRUCTURE_TYPES.iter().map(|s| json!({
            "id": s.id,
            "official_name": s.official_name,
            "model": s.model,
            "scenario": s.scenario,
            "handler": s.handler,
            "note": s.note,
        })).collect::<Vec<_>>(),
        "word_symbols": WORD_SYMBOLS.iter().map(|w| json!({
            "question_type": w.qtype,
            "channel": w.channel,
            "symbol": w.symbol,
            "official_name": w.official_name,
            "code": w.code,
            "ambiguous_across_channels": w.ambiguous,
        })).collect::<Vec<_>>(),
        "word_channels": {
            "symbol": "新版超能单词 QuestionSymbol（0~13，自主训练题型）",
            "sync_pk": "同步词汇/PK 对战 SyncWordQuestionBean（6, 16~20）",
            "warning": "question_type=6 在两个通道语义不同（单词跟读 vs 听音选词），必须按通道判定",
        },
        "exam_type_names": {
            "authoritative_guangdong": EXAM_TYPE_NAMES_AUTHORITATIVE,
            "aliases": EXAM_TYPE_NAME_ALIASES,
        },
        "display_elements": DISPLAY_ELEMENTS.iter().map(|d| json!({
            "group": d.group,
            "question": d.question,
            "audio": d.audio,
            "image": d.image,
            "options": d.options,
            "answers": d.answers,
            "reference_text": d.ref_text,
            "keypoints": d.keypoints,
            "analyze": d.analyze,
            "answer_kind": d.answer_kind,
        })).collect::<Vec<_>>(),
        "res_json_authority": RES_JSON_PATHS.iter().map(|(k, v)| json!({"key": k, "path": v})).collect::<Vec<_>>(),
        "evaluation_types": EVALUATION_TYPES,
        "engine_types": ENGINE_TYPES,
        "type_name_inventory": {
            "note": "官方题库实测：2157 个不同大题名 / 167 个 type_name（数据源 laststudio/ets100_resources）",
            "fixture": "tests/fixtures/official_type_inventory.json",
            "exam_type_map_fixture": "tests/fixtures/official_exam_type_map.json",
        },
        "boundary_rules": {
            "multi_answer_order": "原样不排序：保持与 JSON 数组原序一致（源码中只有 append，无 sort）",
            "blank_number_source": "小题/空格号取 info.std[].xth；卷面大题号取 th；exam_info[].value 是指示文本不是题号",
            "reading_answers": "朗读类 answers 直接给朗读原文（官方界面不展示参考答案面板）",
            "score_authority": "逐题满分取 res.json 的 question_list[].question_score，不用 group_name 权重推算",
            "order_authority": "大题顺序取 exam_type_list 下标（并用 exam_type_order 校验）；小题按 question_order 结合 xth/th 递增",
            "collect_authority": "解析家族必须取自模板的 exam_type_collect（或 contents[].code_id ↔ exam_list[].exam_id 挂载），**不能靠中文大题名推断**：实测 118 个官方大题名里有 9 个在不同卷型下归属不同家族（如「回答问题」→ dialogue/role、「情景反应」→ dialogue/role）",
            "name_space_reality": "官方库共 2157 个不同的大题名（group_name），其中 924 个只出现 1 次 —— 按名字适配不可行，正解是 9 个 exam_type_collect 家族 + 结构嗅探",
            "total_score_authority": "卷面总分优先 res.json 的 set_score；模板覆盖不全时用已合并大题之和并告警",
        },
        "coverage": coverage(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_type_table_covers_official_ids() {
        for id in [
            "collector.read",
            "collector.repeat",
            "collector.sentence",
            "collector.word",
            "repeat",
            "repeat_dialogue",
            "essay",
            "templet_common",
            "listen_choice_standard",
            "listen_fill_standard",
            "read_passage_standard",
            "read_sentence_standard",
            "read_word_standard",
            "read_and_choose_standard",
        ] {
            assert!(find_structure_type(id).is_some(), "缺少题型: {id}");
        }
    }

    #[test]
    fn only_templet_common_is_not_explicitly_adapted() {
        let cov = coverage();
        assert_eq!(cov["total"], STRUCTURE_TYPES.len());
        assert_eq!(cov["explicitly_adapted"], STRUCTURE_TYPES.len() - 1);
        let missing = cov["not_explicitly_adapted_list"].as_array().unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0]["id"], "templet_common");
    }

    #[test]
    fn word_symbol_qtype_6_is_marked_ambiguous() {
        assert!(is_ambiguous_qtype(6));
        assert!(!is_ambiguous_qtype(19));
        let symbol = word_symbol(6, "symbol").unwrap();
        let sync = word_symbol(6, "sync_pk").unwrap();
        assert_eq!(symbol.official_name, "单词跟读");
        assert_eq!(sync.official_name, "听音选词（4选1）");
    }

    #[test]
    fn word_symbols_have_no_valid_type_above_20() {
        assert!(WORD_SYMBOLS.iter().all(|w| w.qtype <= 20));
    }

    #[test]
    fn taxonomy_exposes_boundary_rules() {
        let spec = taxonomy();
        assert!(spec["boundary_rules"]["multi_answer_order"]
            .as_str()
            .unwrap()
            .contains("不排序"));
        assert!(spec["boundary_rules"]["blank_number_source"]
            .as_str()
            .unwrap()
            .contains("xth"));
        assert_eq!(spec["exam_type_names"]["authoritative_guangdong"][0], "模仿朗读");
    }
}
