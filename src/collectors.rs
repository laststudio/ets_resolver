// -*- coding: utf-8 -*-
// 各题型解析器 (与 legacy/resolver_py/collectors.py 行为一致)

use std::collections::HashMap;
use serde_json::{json, Map, Value};

use crate::cleaner::{
    clean, clean_lines, extract_ets_th, extract_global_no, split_br, split_keywords,
    strip_ets_th, strip_global_no,
};

pub const TYPE_CHOOSE: &str = "collector.choose";
pub const TYPE_READ: &str = "collector.read";
#[allow(dead_code)]
pub const TYPE_WORD: &str = "collector.word";
pub const TYPE_REPEAT_ESSAY: &str = "collector.repeat_essay";
pub const TYPE_ROLE: &str = "collector.role";
pub const TYPE_PICTURE: &str = "collector.picture";
pub const TYPE_FILL: &str = "collector.fill";
pub const TYPE_3Q5A: &str = "collector.3q5a";

// 单词 PK 与词汇作业题型 (逆向自 SyncWordQuestionBean)
pub const TYPE_PK_CHOOSE: &str = "word.choice";            // 6: 听选/词义选择
pub const TYPE_PK_WRITE: &str = "word.write";              // 16: 看释义写单词 (拼写)
pub const TYPE_PK_READ: &str = "word.read";                // 17: 朗读单词
pub const TYPE_PK_DICTATION: &str = "word.dictation";      // 18: 听写单词
pub const TYPE_PK_EN_CN: &str = "word.translate_en_cn";    // 19: 英译中
pub const TYPE_PK_CN_EN: &str = "word.translate_cn_en";    // 20: 中译英

pub fn pk_type_str(qtype: i64) -> &'static str {
    match qtype {
        6 => TYPE_PK_CHOOSE,
        16 => TYPE_PK_WRITE,
        17 => TYPE_PK_READ,
        18 => TYPE_PK_DICTATION,
        19 => TYPE_PK_EN_CN,
        20 => TYPE_PK_CN_EN,
        _ => "word.unknown",
    }
}

/// 递归寻找"新版超能单词"通道标记（`symbol` / `question_symbol` / 值以 `Symbol_` 开头）。
fn has_symbol_marker_deep(value: &Value, depth: usize) -> bool {
    if depth > 5 {
        return false;
    }
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key_lower = k.to_lowercase();
                if key_lower == "symbol"
                    || key_lower == "question_symbol"
                    || key_lower == "symbol_type"
                    || key_lower == "symboltype"
                {
                    return true;
                }
                if let Value::String(s) = v {
                    if s.starts_with("Symbol_") {
                        return true;
                    }
                }
                if has_symbol_marker_deep(v, depth + 1) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => items.iter().any(|v| has_symbol_marker_deep(v, depth + 1)),
        _ => false,
    }
}

/// 判断词汇载荷来自哪个通道。
///
/// 官方存在**双套系统**，且 `question_type=6` 在两套里语义不同（见 `spec::WORD_SYMBOLS`）：
///   * `symbol`   新版超能单词 `QuestionSymbol`（0~13，自主训练）
///   * `sync_pk`  同步词汇 / PK 对战 `SyncWordQuestionBean`（6、16~20）
pub fn detect_word_channel(content: &Value, root: &Value) -> &'static str {
    if has_symbol_marker_deep(content, 0) || has_symbol_marker_deep(root, 0) {
        return "symbol";
    }
    // 旧版超能单词/自训通道常见 `dcld`/`dcgd` 这类 code 字段
    let has_code_marker = |v: &Value| {
        ["code", "code_id", "qtype_code"]
            .iter()
            .any(|k| v.get(k).map_or(false, |c| c.is_string()))
    };
    if content.get("word_question").is_none()
        && content.get("word_questions").is_none()
        && content.get("words").is_none()
        && (has_code_marker(content) || has_code_marker(root))
    {
        return "symbol";
    }
    "sync_pk"
}

/// 词汇题型的 `type` 字符串。`channel` 见 [`detect_word_channel`]。
///
/// `symbol` 通道用官方 code 生成稳定标识（如 `word.symbol.ktxy`），
/// `sync_pk` 通道保持既有标识不变（`word.choice` 等）。
pub fn word_type_str(qtype: i64, channel: &str) -> String {
    match channel {
        "symbol" => crate::spec::word_symbol(qtype, "symbol")
            .map(|s| {
                if s.code.is_empty() {
                    "word.symbol.none".to_string()
                } else {
                    format!("word.symbol.{}", s.code)
                }
            })
            .unwrap_or_else(|| "word.unknown".to_string()),
        _ => pk_type_str(qtype).to_string(),
    }
}

pub const QUESTION_TYPES: [&str; 7] = [
    TYPE_ROLE,
    "collector.dialogue",
    TYPE_3Q5A,
    "collector.repeat",
    "collector.repeat_dialogue",
    "collector.essay",
    "collector.dialogue",
];

fn get_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn get_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn stid_of(content: &Value) -> Option<i64> {
    let info = content.get("info")?;
    if !info.is_object() {
        return None;
    }
    get_i64(info.get("stid")?)
}

fn make_question(
    no: Option<i64>,
    no_source: Option<&str>,
    qtype: &str,
    stid: Option<i64>,
    question: String,
    answers: Vec<String>,
    options: Vec<String>,
    important: Vec<String>,
    extra: Option<Map<String, Value>>,
) -> Value {
    let mut q = Map::new();
    q.insert("no".into(), no.map_or(Value::Null, |n| json!(n)));
    q.insert("type".into(), Value::String(qtype.to_string()));
    q.insert("stid".into(), stid.map_or(Value::Null, |s| json!(s)));
    // 题干统一在这里清洗：早期只有部分题型走了 clean()，导致题干里残留 <br>，
    // 而后端 Python 侧统一 clean_html_text —— 两边解析同一份资源会得到不同文本。
    q.insert("question".into(), Value::String(clean(&question)));
    q.insert("answers".into(), json!(answers));
    q.insert("options".into(), json!(options));
    q.insert("important".into(), json!(important));
    if let Some(extra) = extra {
        for (k, v) in extra {
            q.insert(k, v);
        }
    }
    if let Some(src) = no_source {
        q.insert("_no_source".into(), Value::String(src.to_string()));
    }
    Value::Object(q)
}

fn pick_question_text(primary: &str, secondary: &str) -> String {
    if !primary.is_empty() {
        let text = strip_ets_th(&strip_global_no(primary));
        if !text.is_empty() {
            return text;
        }
    }
    if !secondary.is_empty() {
        return strip_ets_th(secondary);
    }
    String::new()
}

pub fn collect_choose(content: &Value, content2: Option<&Value>, _info: Option<&Value>) -> Vec<Value> {
    let stid = stid_of(content);
    let info = content.get("info").and_then(|v| v.as_object());
    let xtlist = info
        .and_then(|o| o.get("xtlist"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let xtlist2 = content2
        .and_then(|c| c.get("info"))
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("xtlist"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut questions = Vec::new();
    for (i, xt) in xtlist.iter().enumerate() {
        let xt2 = xtlist2.get(i);
        let text_src = xt2
            .and_then(|v| v.get("xt_nr"))
            .map(|v| get_str(v))
            .filter(|s| !s.is_empty())
            .or_else(|| xt.get("xt_nr").map(|v| get_str(v)))
            .unwrap_or_default();
        let (no, no_source) = extract_global_no(&text_src)
            .map(|n| (n, "global"))
            .or_else(|| {
                extract_ets_th(&text_src)
                    .map(|n| (n, "local"))
                    .or_else(|| get_i64(xt.get("xt_xh").unwrap_or(&Value::Null)).map(|n| (n, "local")))
                    .or(Some((i as i64 + 1, "local")))
            })
            .unwrap();

        let question_text = pick_question_text(
            &xt2.and_then(|v| v.get("xt_nr")).map(|v| get_str(v)).unwrap_or_default(),
            &xt.get("xt_nr").map(|v| get_str(v)).unwrap_or_default(),
        );

        let mut options = Vec::new();
        // 结构化选项：官方 payload 的 score_detail 里除了 `choose`（= xx_mc，选项字母）
        // 还有 `choose_id`（= xx_xh，选项序号）。见前端 useExamStepper::G()：
        //   score_detail: { total_score, choose: s.xx_mc, choose_id: n, category }
        // 且判分是 `info.xtlist[].answer === 所选选项的 xxlist[].xx_mc`（两边都是字母，
        // 已用真实语料证实：xx_mc="A"、xx_nr="选项文本"、answer="C"）。
        let mut options_detail: Vec<Value> = Vec::new();
        if let Some(xxlist) = xt.get("xxlist").and_then(|v| v.as_array()) {
            for o in xxlist {
                let mc = get_str(o.get("xx_mc").unwrap_or(&Value::Null));
                let nr = clean(&get_str(o.get("xx_nr").unwrap_or(&Value::Null)));
                let xh = get_str(o.get("xx_xh").unwrap_or(&Value::Null));
                if !nr.is_empty() || !mc.is_empty() {
                    options.push(format!("{mc}. {nr}"));
                }
                if !mc.is_empty() || !xh.is_empty() {
                    options_detail.push(json!({"xx_xh": xh, "xx_mc": mc, "xx_nr": nr}));
                }
            }
        }
        let raw_answer = clean(&get_str(xt.get("answer").unwrap_or(&Value::Null)));
        // 正确答案对应的选项序号（官方 choose_id = xx_xh）
        let choose_id = options_detail
            .iter()
            .find(|o| {
                let mc = o.get("xx_mc").and_then(|v| v.as_str()).unwrap_or("");
                !mc.is_empty() && mc.eq_ignore_ascii_case(&raw_answer)
            })
            .and_then(|o| o.get("xx_xh").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();
        let mut extra = Map::new();
        extra.insert("choose_id".into(), Value::String(choose_id));
        extra.insert("options_detail".into(), Value::Array(options_detail));

        questions.push(make_question(
            Some(no),
            Some(no_source),
            TYPE_CHOOSE,
            stid,
            question_text,
            vec![raw_answer],
            options,
            Vec::new(),
            Some(extra),
        ));
    }
    questions
}

pub fn collect_question_based(
    content: &Value,
    content2: Option<&Value>,
    _info: Option<&Value>,
    qtype: Option<&str>,
) -> Vec<Value> {
    let stid = stid_of(content);
    let info = content.get("info").and_then(|v| v.as_object());
    let questions_arr = info
        .and_then(|o| o.get("question").or_else(|| o.get("questions")))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let questions2 = content2
        .and_then(|c| c.get("info"))
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("question").or_else(|| o.get("questions")))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::new();
    for (i, q) in questions_arr.iter().enumerate() {
        let q2 = questions2.get(i);
        let ask_src = q2
            .and_then(|v| v.get("ask"))
            .map(|v| get_str(v))
            .filter(|s| !s.is_empty())
            .or_else(|| q.get("ask").map(|v| get_str(v)))
            .unwrap_or_default();
        let (no, no_source) = extract_global_no(&ask_src)
            .map(|n| (n, "global"))
            .or_else(|| extract_ets_th(&ask_src).map(|n| (n, "local")))
            .or_else(|| get_i64(q.get("xh").unwrap_or(&Value::Null)).map(|n| (n, "local")))
            .or(Some((i as i64 + 1, "local")))
            .unwrap();

        let question_text = pick_question_text(
            &q2.and_then(|v| v.get("ask")).map(|v| get_str(v)).unwrap_or_default(),
            &q.get("ask").map(|v| get_str(v)).unwrap_or_default(),
        );

        let mut answers = Vec::new();
        if let Some(std) = q.get("std").and_then(|v| v.as_array()) {
            for s in std {
                let val = clean(&get_str(s.get("value").unwrap_or(&Value::Null)));
                if !val.is_empty() {
                    answers.push(val);
                }
            }
        }

        let important = split_keywords(&get_str(q.get("keywords").unwrap_or(&Value::Null)), "|");

        let mut extra = Map::new();
        if let Some(role) = q.get("role") {
            let role_str = get_str(role);
            if !role_str.is_empty() {
                extra.insert("role".into(), Value::String(role_str));
            }
        }
        if let Some(answer) = q.get("answer") {
            let answer_str = clean(&get_str(answer));
            if !answer_str.is_empty() {
                extra.insert("ref_answer".into(), Value::String(answer_str));
            }
        }

        result.push(make_question(
            Some(no),
            Some(no_source),
            qtype.unwrap_or(TYPE_ROLE),
            stid,
            question_text,
            answers,
            Vec::new(),
            important,
            if extra.is_empty() { None } else { Some(extra) },
        ));
    }
    result
}

/// `info.std[]` 直接成题的兜底（用于 info.question 缺失的 3q5a 等题型）。
fn collect_std_only(content: &Value, qtype: &str) -> Vec<Value> {
    let stid = stid_of(content);
    let info = content.get("info").and_then(|v| v.as_object());
    let std = info
        .and_then(|o| o.get("std"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::new();
    for (i, s) in std.iter().enumerate() {
        let value = clean(&get_str(s.get("value").unwrap_or(&Value::Null)));
        if value.is_empty() {
            continue;
        }
        let no = std_no(s, i as i64 + 1);
        result.push(make_question(
            Some(no),
            Some("local"),
            qtype,
            stid,
            format!("问题 {}", i + 1),
            vec![value],
            Vec::new(),
            Vec::new(),
            None,
        ));
    }
    result
}


/// `info.std[]` 的题号：官方两处实现用的键**不一样**——
/// - 套题链路（`useExamStepper::G()`）：`std.find(e => e.xth === a)`
/// - 听说同步 `Practice`：`std.find(e => e.th === l[1])`
/// 且示例数据里出现过 `"th":"1."`（带尾点）。这里统一兼容，避免漏题/错题号。
fn std_no(s: &Value, fallback: i64) -> i64 {
    for key in ["xth", "th"] {
        if let Some(v) = s.get(key) {
            if let Some(n) = get_i64(v) {
                return n;
            }
            let raw = get_str(v);
            let trimmed = raw.trim().trim_end_matches('.').trim();
            if let Ok(n) = trimmed.parse::<i64>() {
                return n;
            }
        }
    }
    fallback
}

/// `info.std[].value` 的多个可接受答案：官方用 `/` 分隔并逐项 `trim().toLowerCase()`
/// （PC：`X(e.value).split("/").map(e=>e.trim().toLowerCase())`；
/// 另一处 iframe 模板：`slashToArr: str.replace(/<br \/>/g,'').split('/')`）。
/// `X()` 还会做中英标点归一（’→'、：→:、。→.、——→-、_→-）。
pub fn std_accepted_answers(value: &str) -> Vec<String> {
    value
        .replace("<br />", "")
        .replace("<br/>", "")
        .split('/')
        .map(|s| normalize_answer_text(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// 官方 `X()`：中英标点归一 + 转小写（仅用于**比较**，不改变对外输出）。
fn normalize_answer_text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{ff1a}' => ':',
            '\u{ff0c}' => ',',
            '\u{3002}' => '.',
            '\u{2014}' | '\u{ff0d}' => '-',
            '_' => '-',
            other => other,
        })
        .collect::<String>()
        .to_lowercase()
}

pub fn collect_fill(content: &Value, _content2: Option<&Value>, _info: Option<&Value>) -> Vec<Value> {
    let stid = stid_of(content);
    let info = content.get("info").and_then(|v| v.as_object());
    let std = info
        .and_then(|o| o.get("std"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::new();
    for (i, s) in std.iter().enumerate() {
        let no = std_no(s, i as i64 + 1);
        result.push(make_question(
            Some(no),
            Some("local"),
            TYPE_FILL,
            stid,
            String::new(),
            vec![clean(&get_str(s.get("value").unwrap_or(&Value::Null)))],
            Vec::new(),
            Vec::new(),
            None,
        ));
    }
    result
}

pub fn collect_picture(content: &Value, _content2: Option<&Value>, _info: Option<&Value>) -> Vec<Value> {
    let stid = stid_of(content);
    let info = content.get("info").and_then(|v| v.as_object());
    let std = info
        .and_then(|o| o.get("std"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut answers = Vec::new();
    for s in &std {
        let val = clean(&get_str(s.get("value").unwrap_or(&Value::Null)));
        if !val.is_empty() {
            answers.push(val);
        }
    }

    let mut important = split_keywords(
        &get_str(info.and_then(|o| o.get("keypoint")).unwrap_or(&Value::Null)),
        "</br>",
    );
    important.extend(split_br(
        &get_str(info.and_then(|o| o.get("analyze")).unwrap_or(&Value::Null)),
    ));

    let topic = clean(&get_str(info.and_then(|o| o.get("topic")).unwrap_or(&Value::Null)));
    let question = if topic.is_empty() {
        let value = clean(&get_str(info.and_then(|o| o.get("value")).unwrap_or(&Value::Null)));
        value.chars().take(200).collect()
    } else {
        topic
    };

    vec![make_question(
        None,
        None,
        TYPE_PICTURE,
        stid,
        question,
        answers,
        Vec::new(),
        important,
        None,
    )]
}

pub fn collect_read(content: &Value, _content2: Option<&Value>, _info: Option<&Value>) -> Vec<Value> {
    let stid = stid_of(content);
    let info = content.get("info").and_then(|v| v.as_object());
    let important = split_br(&get_str(info.and_then(|o| o.get("analyze")).unwrap_or(&Value::Null)));

    // 朗读/复述类：答案就是朗读文本本身。
    // 早期只把文本放进 question、answers 留空，后端（依赖 answers 取朗读文本）切换过来会
    // 直接把"模仿朗读"的答案丢空 —— 双解析比对时就是这么发现的。
    let mut texts: Vec<String> = Vec::new();
    for key in ["sublist", "subList", "sentences"] {
        if let Some(arr) = info.and_then(|o| o.get(key)).and_then(|v| v.as_array()) {
            for item in arr {
                let raw = match item {
                    Value::Object(o) => o
                        .get("text")
                        .or_else(|| o.get("value"))
                        .or_else(|| o.get("sentence"))
                        .map(|v| get_str(v))
                        .unwrap_or_default(),
                    Value::String(s) => s.clone(),
                    _ => String::new(),
                };
                let cleaned = clean(&raw);
                if !cleaned.is_empty() {
                    texts.push(cleaned);
                }
            }
            if !texts.is_empty() {
                break;
            }
        }
    }
    if texts.is_empty() {
        // 单篇朗读：正文在 info.value
        texts.push(clean(
            &get_str(info.and_then(|o| o.get("value")).unwrap_or(&Value::Null)),
        ));
    }

    texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            make_question(
                Some((i + 1) as i64),
                Some("local"),
                TYPE_READ,
                stid,
                text.clone(),
                vec![text],
                Vec::new(),
                important.clone(),
                None,
            )
        })
        .collect()
}

pub fn parse_pk_options(opt_val: &Value) -> Vec<String> {
    let arr = match opt_val {
        Value::Array(a) => a.clone(),
        Value::String(s) => {
            if s.trim().is_empty() {
                return Vec::new();
            }
            serde_json::from_str::<Vec<Value>>(s).unwrap_or_default()
        }
        _ => return Vec::new(),
    };
    let mut options = Vec::new();
    for opt in arr {
        if let Value::Object(o) = opt {
            let name = clean(&get_str(o.get("option_name").or_else(|| o.get("name")).unwrap_or(&Value::Null)));
            let val = clean(&get_str(o.get("option_value").or_else(|| o.get("value")).unwrap_or(&Value::Null)));
            if !name.is_empty() && !val.is_empty() {
                options.push(format!("{}. {}", name, val));
            } else if !val.is_empty() {
                options.push(val);
            } else if !name.is_empty() {
                options.push(name);
            }
        }
    }
    options
}


/// `collector.repeat` / 语篇听读类：**`info.fj[]` 每句 = 一小题**。
///
/// 官方语义（静态提取，附证据）：
/// - PC 前端 `useExamStepper`：`case"collector.repeat": for(const n of r.info.fj) e = \`${D}_${r.info.stid}_${n.xh}\``
///   —— 逐句生成同步键，即**一句一题**，题号取 `fj[].xh`。
/// - 答案页：`v-for='i in one.info.fj'`。
/// - 真实内容包实测（tests/corpus/guangdong_hs_monitor_a/2_课内语篇听读）：
///   `info.fj` 有 10 项，元素 = `{value: 'It boils down to this:', xh: '1', fs: '0.40',
///   ai: '...', time: '15', audio: 'fj1.mp3'}`；同目录 `info_extra.json` 也正好 10 行
///   （`[{id:'fj_xh',value:'1'},{id:'wt_xh',value:'1'},{id:'record_time',value:'15'}]`），
///   行数与小题数一致 —— 与"按行数切题/均分"的官方行为吻合。
///
/// 旧行为把整段 `info.value` 出成 **1 题**，等于丢掉 N-1 题（实测 10/12 句只剩 1 题）。
pub fn collect_repeat_by_sentence(
    content: &Value,
    content2: Option<&Value>,
    info: Option<&Value>,
) -> Vec<Value> {
    let stid = stid_of(content);
    let info_obj = info
        .and_then(|v| v.get("info"))
        .or_else(|| content.get("info"))
        .and_then(|v| v.as_object());
    let fj = info_obj
        .and_then(|o| o.get("fj"))
        .and_then(|v| v.as_array())
        .or_else(|| {
            content2
                .and_then(|c| c.get("info"))
                .and_then(|o| o.get("fj"))
                .and_then(|v| v.as_array())
        })
        .cloned()
        .unwrap_or_default();
    if fj.is_empty() {
        return Vec::new();
    }
    let passage = info_obj
        .and_then(|o| o.get("value"))
        .map(|v| clean(&get_str(v)))
        .unwrap_or_default();
    let mut questions = Vec::new();
    for (i, item) in fj.iter().enumerate() {
        let obj = item.as_object();
        let text = obj
            .and_then(|o| o.get("value"))
            .or_else(|| obj.and_then(|o| o.get("text")))
            .map(|v| clean(&get_str(v)))
            .unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        let no = obj
            .map(|o| std_no(&Value::Object(o.clone()), i as i64 + 1))
            .unwrap_or(i as i64 + 1);
        let mut extra = Map::new();
        // 逐句音频文件名（官方 fj[].audio，如 "fj1.mp3"）：客户端可据此播放句子音频
        if let Some(audio) = obj
            .and_then(|o| o.get("audio"))
            .map(|v| get_str(v))
            .filter(|s| !s.is_empty())
        {
            extra.insert("audio".into(), Value::String(audio));
        }
        // 录音时长（秒）：官方 info_extra.json 的 record_time，实测与 fj[].time 一致
        if let Some(secs) = obj
            .and_then(|o| o.get("time"))
            .map(|v| get_str(v))
            .filter(|s| !s.is_empty())
        {
            extra.insert("record_time".into(), Value::String(secs));
        }
        // 整段原文（供上下文展示；答案仍是逐句）
        if !passage.is_empty() {
            extra.insert("passage".into(), Value::String(passage.clone()));
        }
        // 跟读/听读类：上传音频由引擎评分，提交值官方为空串
        questions.push(make_question(
            Some(no),
            Some("local"),
            "collector.repeat",
            stid,
            text.clone(),
            vec![text],
            Vec::new(),
            Vec::new(),
            Some(extra),
        ));
    }
    questions
}

pub fn collect_repeat_essay(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Vec<Value> {
    let stid = stid_of(content);
    let info_obj = content.get("info");
    if let Some(sublist) = info_obj.and_then(|o| o.get("sublist")).and_then(|v| v.as_array()) {
        let mut questions = Vec::new();
        for (i, sub) in sublist.iter().enumerate() {
            let text = clean_lines(&get_str(sub.get("text").or_else(|| sub.get("value")).or_else(|| sub.get("content")).unwrap_or(&Value::Null)));
            if !text.is_empty() {
                questions.push(make_question(
                    Some((i + 1) as i64),
                    Some("local"),
                    TYPE_REPEAT_ESSAY,
                    stid,
                    text.clone(),
                    vec![text],   // 复述题的答案就是复述文本

                    Vec::new(),
                    Vec::new(),
                    None,
                ));
            }
        }
        if !questions.is_empty() {
            return questions;
        }
    }
    if let Some(io) = info_obj {
        if io.get("question").map_or(false, |v| v.is_array()) || io.get("questions").map_or(false, |v| v.is_array()) {
            return collect_question_based(content, content2, info, Some(TYPE_REPEAT_ESSAY));
        }
    }
    collect_read(content, content2, info)
}

pub fn collect_word_pk(content: &Value) -> Vec<Value> {
    let root = if let Some(info) = content.get("info") {
        if info.is_object() || info.is_array() {
            info
        } else {
            content
        }
    } else {
        content
    };

    let mut word_map: HashMap<i64, (String, String)> = HashMap::new();
    let mut raw_questions: Vec<Value> = Vec::new();

    // 1. 提取 words 字典
    let words = root.get("words").or_else(|| root.get("word")).or_else(|| content.get("words")).or_else(|| content.get("word"));
    if let Some(words_arr) = words.and_then(|v| v.as_array()) {
        for w in words_arr {
            let wid = get_i64(w.get("id").unwrap_or(&Value::Null))
                .or_else(|| w.get("struct").and_then(|s| get_i64(s.get("id").unwrap_or(&Value::Null))));
            if let Some(id) = wid {
                let cnt = clean(&get_str(w.get("content").or_else(|| w.get("struct").and_then(|s| s.get("content"))).unwrap_or(&Value::Null)));
                let trs = clean(&get_str(w.get("translate").or_else(|| w.get("struct").and_then(|s| s.get("translate"))).unwrap_or(&Value::Null)));
                word_map.insert(id, (cnt, trs));
            }
        }
    }

    // 2. 提取 word/struct 嵌套格式
    let data_arr = root.get("data").and_then(|v| v.as_array()).or_else(|| root.as_array());
    let mut is_nested = false;
    if let Some(data) = data_arr {
        if let Some(first) = data.first() {
            if first.get("question").map_or(false, |q| q.is_array()) {
                is_nested = true;
                for item in data {
                    let wid = get_i64(item.get("id").unwrap_or(&Value::Null));
                    let cnt = clean(&get_str(item.get("content").unwrap_or(&Value::Null)));
                    let trs = clean(&get_str(item.get("translate").unwrap_or(&Value::Null)));
                    if let Some(id) = wid {
                        word_map.insert(id, (cnt.clone(), trs.clone()));
                    }
                    if let Some(qs) = item.get("question").and_then(|v| v.as_array()) {
                        for q in qs {
                            if let Value::Object(mut q_map) = q.clone() {
                                if let Some(id) = wid {
                                    if !q_map.contains_key("word_id") {
                                        q_map.insert("word_id".into(), json!(id));
                                    }
                                }
                                if !cnt.is_empty() && !q_map.contains_key("word") {
                                    q_map.insert("word".into(), Value::String(cnt.clone()));
                                }
                                if !trs.is_empty() && !q_map.contains_key("translate") {
                                    q_map.insert("translate".into(), Value::String(trs.clone()));
                                }
                                raw_questions.push(Value::Object(q_map));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. 提取扁平列表格式
    if !is_nested {
        let qs = root.get("word_question")
            .or_else(|| root.get("word_questions"))
            .or_else(|| root.get("questions"))
            .or_else(|| root.get("list"))
            .or_else(|| root.as_array().map(|_| root))
            .or_else(|| content.get("word_question"))
            .or_else(|| content.get("word_questions"))
            .or_else(|| content.get("questions"));
        if let Some(qs_arr) = qs.and_then(|v| v.as_array()) {
            for q in qs_arr {
                if q.is_object() {
                    raw_questions.push(q.clone());
                }
            }
        }
    }

    // 4. 构建题目结构
    // 官方词汇存在双套系统，question_type=6 在两套里语义不同（单词跟读 vs 听音选词），
    // 因此先判定通道，再决定 type 字符串与语义（见 spec::WORD_SYMBOLS）。
    let word_channel = detect_word_channel(content, root);
    let mut questions = Vec::new();
    for (i, q) in raw_questions.iter().enumerate() {
        let qtype_num = get_i64(q.get("question_type").or_else(|| q.get("type")).or_else(|| q.get("q_type")).unwrap_or(&Value::Null)).unwrap_or(0);
        // 官方名优先：真实载荷自带 `question_type_name`（"听音选义/默写词汇/朗读词汇/…"），
        // 比数字映射更可靠，且能消除"同一数字跨通道语义不同"的歧义。
        let qtype_name = clean(&get_str(
            q.get("question_type_name").or_else(|| q.get("type_name")).unwrap_or(&Value::Null),
        ));
        let qtype_str = crate::spec::word_type_by_name(&qtype_name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| word_type_str(qtype_num, word_channel));
        let word_id = get_i64(q.get("word_id").unwrap_or(&Value::Null)).unwrap_or(0);
        let q_id = get_i64(q.get("id").or_else(|| q.get("q_id")).or_else(|| q.get("question_id")).unwrap_or(&Value::Null)).unwrap_or(0);

        let ans = clean(&get_str(q.get("answer").unwrap_or(&Value::Null)));
        let word_info = word_map.get(&word_id);
        let w_content = word_info.map(|x| x.0.clone()).unwrap_or_else(|| clean(&get_str(q.get("word").or_else(|| q.get("content")).unwrap_or(&Value::Null))));
        let w_trans = word_info.map(|x| x.1.clone()).unwrap_or_else(|| clean(&get_str(q.get("translate").unwrap_or(&Value::Null))));

        let ans_text = match qtype_num {
            6 => ans.clone(),
            16 | 17 | 18 | 20 => if !w_content.is_empty() { w_content.clone() } else { ans.clone() },
            19 => if !w_trans.is_empty() { w_trans.clone() } else { ans.clone() },
            _ => if !ans.is_empty() { ans.clone() } else { w_content.clone() },
        };

        let question_text = match qtype_num {
            19 => if !w_content.is_empty() { w_content.clone() } else { "英译中".to_string() },
            20 => if !w_trans.is_empty() { w_trans.clone() } else { "中译英".to_string() },
            16 => if !w_trans.is_empty() { w_trans.clone() } else { "拼写单词".to_string() },
            17 => if !w_content.is_empty() { w_content.clone() } else { "朗读单词".to_string() },
            18 => if !w_trans.is_empty() { w_trans.clone() } else { "听写单词".to_string() },
            6 => {
                let q_custom = clean(&get_str(q.get("xt_nr").or_else(|| q.get("question")).unwrap_or(&Value::Null)));
                if !q_custom.is_empty() {
                    q_custom
                } else if !w_content.is_empty() && !w_trans.is_empty() {
                    format!("{} ({})", w_content, w_trans)
                } else if !w_content.is_empty() {
                    w_content.clone()
                } else if !w_trans.is_empty() {
                    w_trans.clone()
                } else {
                    "选择题".to_string()
                }
            }
            _ => {
                let q_custom = clean(&get_str(q.get("question").or_else(|| q.get("xt_nr")).unwrap_or(&Value::Null)));
                if !q_custom.is_empty() {
                    q_custom
                } else if !w_content.is_empty() {
                    w_content.clone()
                } else {
                    w_trans.clone()
                }
            }
        };

        let options = if qtype_num == 6 || q.get("option").is_some() || q.get("options").is_some() {
            let opt_val = q.get("option").or_else(|| q.get("options")).unwrap_or(&Value::Null);
            parse_pk_options(opt_val)
        } else {
            Vec::new()
        };

        let mut important = Vec::new();
        if !w_trans.is_empty() && qtype_num != 19 {
            important.push(w_trans.clone());
        } else if !w_content.is_empty() && qtype_num != 20 {
            important.push(w_content.clone());
        }

        let mut extra = Map::new();
        // 官方词汇双通道：把判定结果带出去，客户端/后端能看到型别是按哪套系统解释的
        extra.insert("channel".into(), Value::String(word_channel.to_string()));
        if !qtype_name.is_empty() {
            // 官方自带的题型名：原样带出（对账/排错用，不做归一化）
            extra.insert("question_type_name".into(), Value::String(qtype_name.clone()));
        }
        let raw_answer = get_str(q.get("answer").unwrap_or(&Value::Null));
        if !raw_answer.is_empty() {
            // 官方原始答案（选择题是选项字母 "A"/"B"…）。我们的 answers 是归一化后的
            // 可读答案，这里保留官方原值以便逐字节对账。
            extra.insert("official_answer".into(), Value::String(raw_answer.clone()));
        }

        // ── 上交值（submit_answer）：**提交给官方时应填的那个值** ─────────────
        // 官方判分比的是 option_name（选项字母）：`optionName == question.getAnswer()`
        // （WordBattleFragment.java:1061、jadx_exam/.../fragment/h.java:230-234）。
        // 我们的 `answers` 是给人看的可读答案（19 给中文释义、20 给单词），直接上交会判错 ——
        // 所以单独给一个"上交值"，各端（后端/Android/Fe_web）统一用它，避免每端各修一遍。
        //   6 听音选义 / 19 英译中 / 20 中译英 -> 官方选项字母
        //   16 默写 / 18 听写            -> 单词
        //   17 朗读                     -> 空串（官方 answer 为空，分数由服务端评测给出）
        let submit_answer = match qtype_num {
            6 | 19 | 20 => raw_answer.clone(),
            16 | 18 => {
                if !w_content.is_empty() {
                    w_content.clone()
                } else {
                    raw_answer.clone()
                }
            }
            17 => String::new(),
            _ => {
                if !raw_answer.is_empty() {
                    raw_answer.clone()
                } else {
                    ans_text.clone()
                }
            }
        };
        extra.insert("submit_answer".into(), Value::String(submit_answer));
        if crate::spec::is_ambiguous_qtype(qtype_num) {
            // 该 question_type 在两个通道语义不同，客户端如需精确语义请自行判定通道
            extra.insert("ambiguous_across_channels".into(), Value::Bool(true));
        }
        if let Some(spec) = crate::spec::word_symbol(qtype_num, word_channel) {
            extra.insert("symbol".into(), Value::String(spec.symbol.to_string()));
            extra.insert("symbol_name".into(), Value::String(spec.official_name.to_string()));
        }
        if word_id > 0 {
            extra.insert("word_id".into(), json!(word_id));
        }
        if q_id > 0 {
            extra.insert("question_id".into(), json!(q_id));
        }
        if !w_content.is_empty() {
            extra.insert("word".into(), Value::String(w_content));
        }
        if !w_trans.is_empty() {
            extra.insert("translate".into(), Value::String(w_trans));
        }

        let answers = if !ans_text.is_empty() { vec![ans_text] } else { Vec::new() };

        // 题号：载荷里显式给了就用它（有些端会把全局题号带下来），否则用本任务内的下标。
        // 关键点是 no_source 保持 "local"：parse_batch 合并多任务时会按任务顺序 _order 重排，
        // 局部编号不会把不同任务的题目搅在一起。
        let explicit_no = get_i64(
            q.get("no")
                .or_else(|| q.get("order"))
                .or_else(|| q.get("sort"))
                .unwrap_or(&Value::Null),
        )
        .filter(|n| *n > 0)
        .unwrap_or((i + 1) as i64);

        questions.push(make_question(
            Some(explicit_no),
            Some("local"),
            &qtype_str,
            // stid 是"这道题的身份"。词汇题官方按 (word_id, question_id) 成对配对
            // （SyncWordRepository.java:1338），所以身份只能是 question_id：
            // 旧实现用 word_id 顶替，会把 (wid, wid) 配成对 -> 判错（"PK 顺序全乱"的真因之一）。
            // 取不到 question_id 就留空，由调用方决定跳过（word_id 仍在 extras 里，不丢信息）。
            if q_id > 0 { Some(q_id) } else { None },
            question_text,
            answers,
            options,
            important,
            if extra.is_empty() { None } else { Some(extra) },
        ));
    }

    apply_explicit_pk_order(&mut questions, content, root);
    apply_pk_type_filter(&mut questions, content, root);
    apply_deterministic_pk_order(&mut questions, content, root);
    questions
}

/// 没有显式顺序时的确定性排序。
///
/// `/word/struct` 在服务端是按 id 集合（WHERE id IN …）或缓存切片批量取的，
/// **不承诺与传入顺序一致**；官方客户端自己也只把它当 Map 用（SyncWordResourceHelper.o）
/// 再按本地列表回填。所以这里给一个稳定可复现的顺序（word_id → question_id），
/// 免得同一份数据两次解析给出不同顺序，看起来就像"乱序"。
fn apply_deterministic_pk_order(questions: &mut Vec<Value>, content: &Value, root: &Value) {
    if has_explicit_pk_order(content, root) {
        return;   // 调用方给了顺序就别覆盖
    }
    let mut keyed: Vec<(i64, i64, usize, Value)> = questions
        .drain(..)
        .enumerate()
        .map(|(i, q)| {
            let wid = q.get("word_id").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
            let qid = q.get("question_id").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
            (wid, qid, i, q)
        })
        .collect();
    keyed.sort_by_key(|(wid, qid, i, _)| (*wid, *qid, *i));
    *questions = keyed.into_iter().map(|(_, _, _, q)| q).collect();
}

fn has_explicit_pk_order(content: &Value, root: &Value) -> bool {
    for holder in [content, root] {
        for key in ["word_ids", "question_ids"] {
            if let Some(v) = holder.get(key) {
                let non_empty = match v {
                    Value::Array(a) => !a.is_empty(),
                    Value::String(s) => !s.trim().is_empty(),
                    _ => false,
                };
                if non_empty {
                    return true;
                }
            }
        }
    }
    false
}

/// PK 模式：官方客户端 `WordBattleViewModel.B0()` 里 `this.k`（是否比赛模式）
fn pk_is_match_mode(content: &Value, root: &Value) -> bool {
    for holder in [content, root] {
        if let Some(v) = holder.get("pk_mode").or_else(|| holder.get("mode")) {
            if let Some(s) = v.as_str() {
                if s.eq_ignore_ascii_case("match") || s.eq_ignore_ascii_case("battle_match") {
                    return true;
                }
                if s.eq_ignore_ascii_case("homework") || s.eq_ignore_ascii_case("pk") {
                    return false;
                }
            }
        }
        if let Some(v) = holder.get("is_match").and_then(|v| v.as_bool()) {
            return v;
        }
        // start-word-pk 的 source：1 = 普通/词汇作业，3 = 竞赛
        if let Some(src) = get_i64(holder.get("source").unwrap_or(&Value::Null)) {
            return src == 3;
        }
    }
    false
}

/// 是否处于"对战 PK"场景（只有对战才做题型过滤，词汇作业不能过滤）。
fn pk_is_battle(content: &Value, root: &Value) -> bool {
    for holder in [content, root] {
        if let Some(v) = holder.get("pk_mode").or_else(|| holder.get("mode")).and_then(|v| v.as_str()) {
            let v = v.to_ascii_lowercase();
            if v.contains("battle") || v.contains("pk") || v == "match" || v == "homework" {
                return true;
            }
        }
        // pk-info 的响应特征：有 serialId / opponent 就是对战
        if holder.get("serialId").is_some()
            || holder.get("serial_id").is_some()
            || holder.get("opponent").is_some()
            || holder.get("mine").is_some()
        {
            return true;
        }
    }
    false
}

/// 按官方客户端一致的口径过滤 PK 题型。
///
/// `WordBattleViewModel.B0()` 对 structList 做了强行过滤：
///   普通 PK 作业：只保留 `isTypeEnToCn`（题型 19 英选汉），听选 6 / 拼写 16 / 听读 17/18
///                 直接从题目列表 `it2.remove()` 移除 —— 它们**根本不会出现在 PK 里**；
///   竞赛模式（this.k）：保留 19（英选汉）+ 20（汉选英）。
///
/// 不过滤的后果：解析出来的题量比官方 PK 实际问的更多，题号/顺序自然对不上，
/// 用户看到的就是"顺序完全乱了"。词汇作业（非对战）不做过滤。
fn apply_pk_type_filter(questions: &mut Vec<Value>, content: &Value, root: &Value) {
    if !pk_is_battle(content, root) {
        return;
    }
    let is_match = pk_is_match_mode(content, root);
    let keep = |q: &Value| -> bool {
        let qtype = get_i64(q.get("question_type").unwrap_or(&Value::Null))
            .or_else(|| {
                // 解析后 question_type 可能已经不在，退回看 type 字符串
                match q.get("type").and_then(|v| v.as_str()) {
                    Some(TYPE_PK_EN_CN) => Some(19),
                    Some(TYPE_PK_CN_EN) => Some(20),
                    _ => None,
                }
            })
            .unwrap_or(0);
        if is_match {
            qtype == 19 || qtype == 20
        } else {
            qtype == 19
        }
    };

    let filtered: Vec<Value> = questions.iter().filter(|q| keep(q)).cloned().collect();
    // 安全阀：过滤后一条不剩说明判断有误（例如纯拼写词汇作业被当成对战），
    // 此时保持原样，宁可多解析也不能把整份作业清空。
    if !filtered.is_empty() {
        *questions = filtered;
    }
}

/// 按载荷里显式给出的 id 顺序重排（客户端对战顺序的权威来源）。
///
/// 依据逆向出的 PK 链路：`/g/pk/start-word-pk` 用 `word_ids` / `question_ids`
/// （下划线或逗号拼接，也接受数组）确定本轮对战顺序；服务端 `/word/struct` 返回的
/// data 顺序并不保证与之相同，所以只要载荷带了这个序列，就以它为准。
fn apply_explicit_pk_order(questions: &mut Vec<Value>, content: &Value, root: &Value) {
    let pick_seq = |key: &str| -> Vec<i64> {
        for holder in [content, root] {
            if let Some(v) = holder.get(key) {
                if let Some(arr) = v.as_array() {
                    let ids: Vec<i64> = arr.iter().filter_map(|x| get_i64(x)).collect();
                    if !ids.is_empty() {
                        return ids;
                    }
                }
                if let Some(s) = v.as_str() {
                    let ids: Vec<i64> = s
                        .split(|c| c == '_' || c == ',' || c == ' ')
                        .filter_map(|t| t.trim().parse::<i64>().ok())
                        .collect();
                    if !ids.is_empty() {
                        return ids;
                    }
                }
            }
        }
        Vec::new()
    };

    let word_order = pick_seq("word_ids");
    let question_order = pick_seq("question_ids");
    if word_order.is_empty() && question_order.is_empty() {
        return;
    }

    let rank_of = |id: i64, seq: &[i64]| -> i64 {
        seq.iter().position(|x| *x == id).map(|p| p as i64).unwrap_or(i64::MAX)
    };

    let mut keyed: Vec<(i64, i64, usize, Value)> = questions
        .drain(..)
        .enumerate()
        .map(|(i, q)| {
            let wid = q.get("word_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let qid = q.get("question_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let w_rank = if word_order.is_empty() { 0 } else { rank_of(wid, &word_order) };
            let q_rank = if question_order.is_empty() { 0 } else { rank_of(qid, &question_order) };
            (w_rank, q_rank, i, q)
        })
        .collect();
    keyed.sort_by_key(|(w_rank, q_rank, i, _)| (*w_rank, *q_rank, *i));

    // 只有真的能对上号（不是所有 rank 都是 MAX）才采用，避免用了错字段把顺序搞得更乱
    let matched = keyed
        .iter()
        .any(|(w, q, _, _)| *w != i64::MAX || *q != i64::MAX);
    if !matched {
        return;
    }

    *questions = keyed.into_iter().map(|(_, _, _, q)| q).collect();
    for (i, q) in questions.iter_mut().enumerate() {
        if let Some(o) = q.as_object_mut() {
            o.insert("no".into(), json!((i + 1) as i64));
        }
    }
}

/// 结构嗅探结果：题目 + 命中的形状家族名。
/// 家族名用于诊断与覆盖率统计（`generic:*`），空结果统一记为 `unmatched`。
pub fn collect_generic_routed(
    content: &Value,
    content2: Option<&Value>,
    info: Option<&Value>,
    qtype: Option<&str>,
) -> (Vec<Value>, &'static str) {
    if content.is_array() {
        return (collect_word_pk(content), "word_pk");
    }
    let info_obj = content.get("info");
    if !info_obj.map_or(false, |v| v.is_object()) {
        if content.get("word_question").is_some()
            || content.get("word_questions").is_some()
            || content.get("data").is_some()
            || content.get("word_id").is_some()
            || content.get("words").is_some()
            || content.get("word").is_some()
        {
            return (collect_word_pk(content), "word_pk");
        }
        return (Vec::new(), "unmatched");
    }
    let info_obj = info_obj.unwrap();
    if info_obj.get("word_question").is_some()
        || info_obj.get("word_questions").is_some()
        || info_obj.get("words").is_some()
        || info_obj.get("word").is_some()
    {
        return (collect_word_pk(content), "word_pk");
    }
    // 逐句类两条路径，**优先 sublist**（线上 `collect_repeat_essay` 走 sublist 已被生产验证：
    // 真实作业 107576379「短文跟读」解析出 27 题、107576881 出 31 题，成绩正常）。
    // 只有在没有 sublist 时才用 `info.fj[]`（一句一题，官方前端 `for(const n of r.info.fj)`）——
    // 否则整段 `info.value` 会被当成 1 题（实测那种包里 10/12 句只剩 1 题）。
    if info_obj.get("sublist").map_or(false, |v| v.is_array()) {
        return (collect_repeat_essay(content, content2, info), "repeat_essay");
    }
    if info_obj.get("fj").map_or(false, |v| v.is_array()) {
        let qs = collect_repeat_by_sentence(content, content2, info);
        if !qs.is_empty() {
            return (qs, "repeat_by_sentence");
        }
    }
    if info_obj.get("question").map_or(false, |v| v.is_array())
        || info_obj.get("questions").map_or(false, |v| v.is_array())
    {
        return (
            collect_question_based(content, content2, info, qtype),
            "question_based",
        );
    }
    if let Some(std) = info_obj.get("std").and_then(|v| v.as_array()) {
        if std
            .first()
            .and_then(|s| s.get("xth").or_else(|| s.get("th")))
            .map_or(false, |_| true)
        {
            return (collect_fill(content, content2, info), "std_fill");
        }
        return (collect_picture(content, content2, info), "std_picture");
    }
    if info_obj.get("xtlist").map_or(false, |v| v.is_array()) {
        return (collect_choose(content, content2, info), "xtlist_choose");
    }
    if info_obj.get("value").map_or(false, |v| !v.is_null()) {
        return (collect_read(content, content2, info), "read_value");
    }
    if content.get("word_question").is_some()
        || content.get("word_questions").is_some()
        || content.get("data").is_some()
        || content.get("words").is_some()
        || content.get("word").is_some()
    {
        return (collect_word_pk(content), "word_pk");
    }
    (Vec::new(), "unmatched")
}

/// 结构嗅探（只取题目，不关心命中的家族名）。诊断与覆盖率统计请用
/// [`collect_generic_routed`]，它额外返回形状家族名。
#[allow(dead_code)]
pub fn collect_generic(
    content: &Value,
    content2: Option<&Value>,
    info: Option<&Value>,
    qtype: Option<&str>,
) -> Vec<Value> {
    collect_generic_routed(content, content2, info, qtype).0
}

/// 解析路由结果：题目 + 命中的路由层 + 命中的家族名。
///
/// `layer` 用于回答"这题是靠显式 structure_type 认出来的，还是靠结构嗅探兜住的，
/// 还是根本没认出来"，是覆盖率统计与线上排错的基础（见 `src/diagnose.rs`）。
#[derive(Debug, Clone)]
pub struct RouteResult {
    pub questions: Vec<Value>,
    /// `explicit` 显式 structure_type 路由 / `generic` 结构嗅探 / `none` 两者都没命中
    pub layer: &'static str,
    /// 命中的解析家族，如 `collector.choose` / `generic:std_fill` / `word_pk` / `unmatched`
    pub family: String,
}

/// 带路由信息的解析（`parse_content` 的内部实现，行为完全一致）。
/// 判断这份载荷是否属于"文本跟读/朗读"形态：文本在 `content.info.value`
/// （真实 `collector.repeat` 载荷就是这样，且此时入参的 `info` 往往是 list，取不到 value）。
fn has_text_value(content: &Value, info: Option<&Value>) -> bool {
    let non_empty = |v: &Value| {
        v.get("value")
            .and_then(|x| x.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    };
    if let Some(inner) = content.get("info") {
        if inner.is_object() && non_empty(inner) {
            return true;
        }
    }
    info.map(|i| i.is_object() && non_empty(i)).unwrap_or(false)
}


// ============================================================================
// L3 兜底抽取（salvage）：名字不认识、形状也没见过时，尽量把答案抢救出来
//
// 定位：L1 显式路由 / L2 结构嗅探 都失败时的最后一道网。它**不理解官方语义**，
// 只按"哪里像答案"递归找，因此输出一律标记 confidence=low + salvage_path，
// 让调用方自己决定是否展示（绝不假装与官方一致）。
// ============================================================================

const SALVAGE_MAX_DEPTH: usize = 8;
const SALVAGE_MAX_QUESTIONS: usize = 200;

fn salvage_text_of(obj: &Map<String, Value>) -> String {
    for key in ["question", "ask", "stem", "xt_nr", "text", "title", "topic", "content"] {
        if let Some(s) = obj.get(key).and_then(|v| v.as_str()) {
            let cleaned = clean(s);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    String::new()
}

fn salvage_answers_of(obj: &Map<String, Value>) -> Vec<String> {
    // 1) answer / answers
    if let Some(v) = obj.get("answers").or_else(|| obj.get("answer")) {
        let list = match v {
            Value::Array(items) => items
                .iter()
                .map(|x| clean(&get_str(x)))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            other => {
                let s = clean(&get_str(other));
                if s.is_empty() { Vec::new() } else { vec![s] }
            }
        };
        if !list.is_empty() {
            return list;
        }
    }
    // 2) std[] 里的 value
    if let Some(std) = obj.get("std").and_then(|v| v.as_array()) {
        let list: Vec<String> = std
            .iter()
            .filter_map(|item| item.as_object())
            .map(|o| clean(&get_str(o.get("value").unwrap_or(&Value::Null))))
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    Vec::new()
}

fn salvage_options_of(obj: &Map<String, Value>) -> Vec<String> {
    for key in ["options", "xxlist", "option"] {
        if let Some(v) = obj.get(key) {
            let opts = parse_pk_options(v);
            if !opts.is_empty() {
                return opts;
            }
        }
    }
    Vec::new()
}

#[allow(clippy::too_many_arguments)]
fn salvage_walk(
    value: &Value,
    path: &str,
    depth: usize,
    out: &mut Vec<Value>,
    seen: &mut std::collections::HashSet<String>,
) {
    if depth > SALVAGE_MAX_DEPTH || out.len() >= SALVAGE_MAX_QUESTIONS {
        return;
    }
    match value {
        Value::Object(obj) => {
            // question[]：每个条目自成一道题（官方 question/std 语义）
            if let Some(qs) = obj.get("question").and_then(|v| v.as_array()) {
                for (i, item) in qs.iter().enumerate() {
                    if let Some(child) = item.as_object() {
                        let answers = salvage_answers_of(child);
                        if !answers.is_empty() {
                            let text = {
                                let t = salvage_text_of(child);
                                if t.is_empty() { format!("题目 {}", out.len() + 1) } else { t }
                            };
                            push_salvage(out, seen, &text, answers, salvage_options_of(child),
                                         &format!("{path}.question[{i}]"));
                        }
                    }
                    salvage_walk(item, &format!("{path}.question[{i}]"), depth + 1, out, seen);
                }
            } else {
                // std[]：每个条目一道题（填空/转述类语义）
                if let Some(stds) = obj.get("std").and_then(|v| v.as_array()) {
                    for (i, item) in stds.iter().enumerate() {
                        if let Some(child) = item.as_object() {
                            let answer = clean(&get_str(child.get("value").unwrap_or(&Value::Null)));
                            if !answer.is_empty() {
                                let text = {
                                    let t = salvage_text_of(child);
                                    if t.is_empty() {
                                        let xth = clean(&get_str(child.get("xth").unwrap_or(&Value::Null)));
                                        if xth.is_empty() { format!("题目 {}", out.len() + 1) } else { format!("第 {xth} 题") }
                                    } else { t }
                                };
                                push_salvage(out, seen, &text, vec![answer.clone()], Vec::new(),
                                             &format!("{path}.std[{i}].value"));
                            }
                        }
                    }
                }
                // xtlist[]：选择题语义
                if let Some(xts) = obj.get("xtlist").and_then(|v| v.as_array()) {
                    for (i, item) in xts.iter().enumerate() {
                        if let Some(child) = item.as_object() {
                            let answers = salvage_answers_of(child);
                            if !answers.is_empty() {
                                let text = {
                                    let t = salvage_text_of(child);
                                    if t.is_empty() { format!("题目 {}", out.len() + 1) } else { t }
                                };
                                push_salvage(out, seen, &text, answers, salvage_options_of(child),
                                             &format!("{path}.xtlist[{i}]"));
                            }
                        }
                    }
                }
                // 平铺的 answer / answers（跳过已按容器处理的 question/std/xtlist 对象，
                // 否则同一个 std[] 会被当成"平铺答案"再产出一道重复题）
                let has_container = obj.contains_key("question")
                    || obj.contains_key("std")
                    || obj.contains_key("xtlist");
                let answers = if has_container { Vec::new() } else { salvage_answers_of(obj) };
                if !answers.is_empty() {
                    let text = {
                        let t = salvage_text_of(obj);
                        if t.is_empty() { format!("题目 {}", out.len() + 1) } else { t }
                    };
                    push_salvage(out, seen, &text, answers, salvage_options_of(obj), path);
                }
            }
            for (k, v) in obj {
                salvage_walk(v, &format!("{path}.{k}"), depth + 1, out, seen);
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                salvage_walk(item, &format!("{path}[{i}]"), depth + 1, out, seen);
            }
        }
        _ => {}
    }
}

fn push_salvage(
    out: &mut Vec<Value>,
    seen: &mut std::collections::HashSet<String>,
    text: &str,
    answers: Vec<String>,
    options: Vec<String>,
    path: &str,
) {
    let fingerprint = format!("{text}\u{1}{}", answers.join("\u{1}"));
    if !seen.insert(fingerprint) || out.len() >= SALVAGE_MAX_QUESTIONS {
        return;
    }
    let mut extra = Map::new();
    extra.insert("confidence".into(), Value::String("low".into()));
    extra.insert("salvage_path".into(), Value::String(path.to_string()));
    out.push(make_question(
        None,
        None,
        "collector.salvaged",
        None,
        text.to_string(),
        answers,
        options,
        Vec::new(),
        Some(extra),
    ));
}

/// L3 兜底抽取：递归扫描整份载荷，抢救出"像题目"的条目。
/// 输出每道题都带 `confidence=low` 与 `salvage_path`，不冒充官方结果。
pub fn collect_salvaged(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let empty = Vec::new();
    let sources: Vec<(&str, &Value)> = vec![
        ("content", content),
        ("content2", content2.unwrap_or(&Value::Null)),
        ("info", info.unwrap_or(&Value::Null)),
    ];
    for (name, root) in sources {
        if root.is_null() {
            continue;
        }
        if root.is_array() {
            for (i, item) in root.as_array().unwrap_or(&empty).iter().enumerate() {
                salvage_walk(item, &format!("{name}[{i}]"), 0, &mut out, &mut seen);
            }
        } else {
            salvage_walk(root, name, 0, &mut out, &mut seen);
        }
    }
    for (i, q) in out.iter_mut().enumerate() {
        if let Some(obj) = q.as_object_mut() {
            obj.insert("stid".into(), Value::Null);
            obj.insert("no".into(), json!((i + 1) as i64));
        }
    }
    out
}

pub fn parse_content_routed(
    content: &Value,
    content2: Option<&Value>,
    info: Option<&Value>,
) -> RouteResult {
    let explicit = |questions: Vec<Value>, family: &str| RouteResult {
        questions,
        layer: "explicit",
        family: family.to_string(),
    };

    if content.is_array() {
        return explicit(collect_word_pk(content), "word_pk");
    }
    let structure_type = content
        .get("structure_type")
        .map(|v| get_str(v))
        .unwrap_or_default();
    let st = structure_type.as_str();
    // `listen_choice_standard`（官方标准听力选择）不含 "choose"，必须单独认；
    // 注意不能用 contains("choice")：那会把 `word.choice`（词汇PK）误路由到选择题。
    if st.contains("choose") || st.contains("listen_choice") {
        return explicit(collect_choose(content, content2, info), TYPE_CHOOSE);
    }
    // 同上：structure_type 明确是 repeat/听读类且带 info.fj 时，按逐句切题
    if st == "collector.repeat" || st.contains("repeat") {
        let info_obj = info
            .and_then(|v| v.get("info"))
            .or_else(|| content.get("info"));
        let has_sublist = info_obj
            .and_then(|o| o.get("sublist"))
            .map_or(false, |v| v.is_array());
        let has_fj = !has_sublist
            && info_obj
                .and_then(|o| o.get("fj"))
                .map_or(false, |v| v.is_array());
        if has_fj {
            let qs = collect_repeat_by_sentence(content, content2, info);
            if !qs.is_empty() {
                return explicit(qs, "collector.repeat");
            }
        }
    }
    if st == TYPE_REPEAT_ESSAY {
        return explicit(
            collect_repeat_essay(content, content2, info),
            TYPE_REPEAT_ESSAY,
        );
    }
    if st.contains("pk") || st.contains("word_pk") || st.contains("vocabulary") || st.contains("word_homework")
    {
        return explicit(collect_word_pk(content), "word_pk");
    }
    if content.get("word_question").is_some() || content.get("word_questions").is_some() {
        return explicit(collect_word_pk(content), "word_pk");
    }
    // `collector.word` 是**独立族**（词汇课，题目带 question_type ∈ {6,16,17,18,19,20}），
    // 但历史上也有"文本集合形态"的 word 资源包。先按**结构**判定：
    // 出现词汇题标记就按词汇题处理，否则维持"文本集合 = 朗读"的既有行为。
    if st == TYPE_WORD || st.contains("word") {
        let wordish = info
            .and_then(|v| v.get("word_question").or_else(|| v.get("word_questions")))
            .is_some()
            || info
                .and_then(|v| v.get("question"))
                .and_then(|v| v.as_array())
                .map_or(false, |qs| {
                    qs.iter().any(|q| q.get("question_type").is_some() || q.get("option").is_some())
                });
        if wordish {
            let parsed = collect_word_pk(content);
            if !parsed.is_empty() {
                return explicit(parsed, "word_pk");
            }
        }
    }
    // 朗读家族：collector.read / collector.sentence / collector.word（文本集合形态）/
    // read_passage_standard / read_sentence_standard / read_word_standard
    if st.contains("read") || st.contains("word") || st.contains("sentence") {
        return explicit(collect_read(content, content2, info), TYPE_READ);
    }
    if st.contains("fill") {
        return explicit(collect_fill(content, content2, info), TYPE_FILL);
    }
    if st.contains("picture") {
        return explicit(collect_picture(content, content2, info), TYPE_PICTURE);
    }
    if QUESTION_TYPES.contains(&st) {
        let parsed = collect_question_based(content, content2, info, Some(st));
        if !parsed.is_empty() {
            return explicit(parsed, st);
        }
        // 兜底 1：有些资源包把题目直接放在 info.std[]（没有 info.question[]），
        // 早期这里会整个题型解析为空 —— 答案取 std[].value。
        let std_only = collect_std_only(content, st);
        if !std_only.is_empty() {
            return explicit(std_only, st);
        }
        // 兜底 2：文本跟读/听读/朗读类（官方 `collector.repeat`、`collector.repeat_dialogue`，
        // 实测"课内/课外语篇听读"就是这种）—— 载荷只有 `info.value` 原文，没有 question/std。
        // 官方语义与 collector.read 一致：原文即答案（朗读比对学生录音与原文）。
        // 修之前这类包会解析出 0 题（高中听说能力监测 20 分卷直接少 8 分）。
        if has_text_value(content, info) {
            return explicit(collect_read(content, content2, info), st);
        }
        return explicit(Vec::new(), st);
    }
    if st.contains("repeat") || st.contains("essay") || st.contains("dialogue") || st.contains("role") {
        return explicit(collect_question_based(content, content2, info, Some(st)), st);
    }

    let (questions, family) = collect_generic_routed(content, content2, info, Some(st));
    if !questions.is_empty() {
        return RouteResult {
            questions,
            layer: "generic",
            family: format!("generic:{family}"),
        };
    }

    // L3：名字不认识、形状也没见过 —— 最后一道网，尽量把答案抢救出来（低置信）
    let salvaged = collect_salvaged(content, content2, info);
    if !salvaged.is_empty() {
        return RouteResult {
            questions: salvaged,
            layer: "salvage",
            family: "salvage:structural_scan".to_string(),
        };
    }

    RouteResult {
        questions: Vec::new(),
        layer: "none",
        family: format!("generic:{family}"),
    }
}

pub fn parse_content(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Vec<Value> {
    parse_content_routed(content, content2, info).questions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_generic_question_based() {
        let content = json!({
            "structure_type": "collector.unknown",
            "info": {"stid": "999", "question": [
                {"xh": "1", "ask": "Q1", "std": [{"value": "A1"}]}
            ]}
        });
        let qs = parse_content(&content, None, None);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0]["answers"][0], "A1");
        assert_eq!(qs[0]["type"], "collector.unknown");
    }

    #[test]
    fn test_collect_generic_std_based() {
        let content = json!({
            "structure_type": "collector.weird",
            "info": {"stid": "998", "std": [
                {"xth": "1", "value": "v1"}, {"xth": "2", "value": "v2"}
            ]}
        });
        let qs = parse_content(&content, None, None);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0]["answers"][0], "v1");
        assert_eq!(qs[0]["no"], 1);
    }

    #[test]
    fn test_word_channel_disambiguates_qtype_6() {
        // 同步词汇/PK 通道：6 = 听音选词（word.choice）
        let sync = json!({
            "words": [{"id": 1, "content": "apple", "translate": "苹果"}],
            "word_questions": [
                {"question_type": 6, "word_id": 1, "answer": "A",
                 "options": ["A. apple", "B. banana"]}
            ]
        });
        let qs = collect_word_pk(&sync);
        assert_eq!(qs[0]["type"], "word.choice");
        assert_eq!(qs[0]["channel"], "sync_pk");
        assert_eq!(qs[0]["symbol_name"], "听音选义");

        // 新版超能单词通道：6 = 单词跟读（word.symbol.dcgd）
        let symbol = json!({
            "words": [{"id": 1, "content": "apple", "translate": "苹果"}],
            "word_questions": [
                {"question_type": 6, "word_id": 1, "symbol": "Symbol_06_dcgd", "answer": "apple"}
            ]
        });
        let qs = collect_word_pk(&symbol);
        assert_eq!(qs[0]["type"], "word.symbol.dcgd");
        assert_eq!(qs[0]["channel"], "symbol");
        assert_eq!(qs[0]["symbol_name"], "单词跟读");
        assert_eq!(qs[0]["ambiguous_across_channels"], true);
    }

    #[test]
    fn test_official_standard_identifiers_are_routed() {
        for (st, family) in [
            ("listen_choice_standard", TYPE_CHOOSE),
            ("read_and_choose_standard", TYPE_CHOOSE),
            ("listen_fill_standard", TYPE_FILL),
            ("read_passage_standard", TYPE_READ),
            ("read_sentence_standard", TYPE_READ),
            ("read_word_standard", TYPE_READ),
            ("collector.sentence", TYPE_READ),
        ] {
            let content = json!({"structure_type": st, "info": {"stid": "1", "value": "hello"}});
            let routed = parse_content_routed(&content, None, None);
            assert_eq!(routed.layer, "explicit", "{st} 应显式命中");
            assert_eq!(routed.family, family, "{st} 应路由到 {family}");
        }
    }

    #[test]
    fn test_word_choice_still_routes_to_read_family_not_choose() {
        // 回归保护：`word.choice` 含 "choice"，不能因为加了听选识别而被当成选择题
        let content = json!({"structure_type": "word.choice", "info": {"stid": "1", "value": "apple"}});
        let routed = parse_content_routed(&content, None, None);
        assert_eq!(routed.family, TYPE_READ);
    }

    #[test]
    fn test_collect_word_pk_nested() {
        let content = json!({
            "data": [
                {
                    "id": 101,
                    "content": "apple",
                    "translate": "苹果",
                    "question": [
                        {
                            "id": 1001,
                            "question_type": 19,
                            "answer": "苹果"
                        },
                        {
                            "id": 1002,
                            "question_type": 20,
                            "answer": "apple"
                        }
                    ]
                },
                {
                    "id": 102,
                    "content": "banana",
                    "translate": "香蕉",
                    "question": [
                        {
                            "id": 1003,
                            "question_type": 6,
                            "answer": "B",
                            "option": "[{\"option_name\":\"A\",\"option_value\":\"苹果\"},{\"option_name\":\"B\",\"option_value\":\"香蕉\"}]"
                        }
                    ]
                }
            ]
        });
        let qs = parse_content(&content, None, None);
        assert_eq!(qs.len(), 3);
        assert_eq!(qs[0]["type"], TYPE_PK_EN_CN);
        assert_eq!(qs[0]["question"], "apple");
        assert_eq!(qs[0]["answers"][0], "苹果");
        assert_eq!(qs[0]["word_id"], 101);

        assert_eq!(qs[1]["type"], TYPE_PK_CN_EN);
        assert_eq!(qs[1]["question"], "苹果");
        assert_eq!(qs[1]["answers"][0], "apple");

        assert_eq!(qs[2]["type"], TYPE_PK_CHOOSE);
        assert_eq!(qs[2]["answers"][0], "B");
        assert_eq!(qs[2]["options"][1], "B. 香蕉");
    }

    #[test]
    fn test_collect_word_pk_flat() {
        let content = json!({
            "word_questions": [
                {"id": 1, "question_type": 6, "word": "dog", "translate": "狗", "answer": "A", "option": [{"name": "A", "value": "狗"}]},
                {"id": 2, "question_type": 16, "word": "cat", "translate": "猫", "answer": ""},
                {"id": 3, "question_type": 17, "word": "bird", "translate": "鸟", "answer": ""},
                {"id": 4, "question_type": 18, "word": "fish", "translate": "鱼", "answer": ""},
                {"id": 5, "question_type": 19, "word": "book", "translate": "书", "answer": ""},
                {"id": 6, "question_type": 20, "word": "pen", "translate": "钢笔", "answer": ""},
            ]
        });
        let qs = parse_content(&content, None, None);
        assert_eq!(qs.len(), 6);
        assert_eq!(qs[0]["type"], TYPE_PK_CHOOSE);
        assert_eq!(qs[0]["answers"][0], "A");
        assert_eq!(qs[1]["type"], TYPE_PK_WRITE);
        assert_eq!(qs[1]["answers"][0], "cat");
        assert_eq!(qs[2]["type"], TYPE_PK_READ);
        assert_eq!(qs[2]["answers"][0], "bird");
        assert_eq!(qs[3]["type"], TYPE_PK_DICTATION);
        assert_eq!(qs[3]["answers"][0], "fish");
        assert_eq!(qs[4]["type"], TYPE_PK_EN_CN);
        assert_eq!(qs[4]["answers"][0], "书");
        assert_eq!(qs[5]["type"], TYPE_PK_CN_EN);
        assert_eq!(qs[5]["answers"][0], "pen");
    }

    #[test]
    fn test_collect_repeat_essay_sublist() {
        let content = json!({
            "structure_type": "collector.repeat_essay",
            "info": {
                "stid": 1234,
                "sublist": [
                    {"text": "Hello world."},
                    {"text": "Good morning."}
                ]
            }
        });
        let qs = parse_content(&content, None, None);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0]["question"], "Hello world.");
        assert_eq!(qs[1]["question"], "Good morning.");
        assert_eq!(qs[0]["type"], TYPE_REPEAT_ESSAY);
    }

    #[test]
    fn test_collect_word_pk_homework_struct() {
        let content = json!({
            "word": [
                {"id": 635560, "content": "apple", "translate": "苹果"},
                {"id": 635561, "content": "banana", "translate": "香蕉"}
            ],
            "word_question": [
                {
                    "id": 1722562,
                    "word_id": 635560,
                    "question_type": 6,
                    "answer": "B",
                    "option": "[{\"option_name\":\"A\",\"option_value\":\"香蕉\"},{\"option_name\":\"B\",\"option_value\":\"苹果\"}]",
                    "translate": "苹果"
                },
                {
                    "id": 1722563,
                    "word_id": 635561,
                    "question_type": 19,
                    "answer": "",
                    "translate": "香蕉"
                }
            ]
        });
        let qs = parse_content(&content, None, None);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0]["type"], TYPE_PK_CHOOSE);
        assert_eq!(qs[0]["answers"][0], "B");
        assert_eq!(qs[0]["options"][0], "A. 香蕉");
        assert_eq!(qs[0]["options"][1], "B. 苹果");
        assert_eq!(qs[1]["type"], TYPE_PK_EN_CN);
        assert_eq!(qs[1]["answers"][0], "香蕉");
    }
}

