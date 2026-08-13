// -*- coding: utf-8 -*-
// 各题型解析器 (与 resolver_py/collectors.py 行为一致)

use serde_json::{json, Map, Value};

use crate::cleaner::{
    clean, clean_lines, extract_ets_th, extract_global_no, split_br, split_keywords,
    strip_ets_th, strip_global_no,
};

pub const TYPE_CHOOSE: &str = "collector.choose";
pub const TYPE_READ: &str = "collector.read";
#[allow(dead_code)]
pub const TYPE_WORD: &str = "collector.word";
pub const TYPE_ROLE: &str = "collector.role";
pub const TYPE_PICTURE: &str = "collector.picture";
pub const TYPE_FILL: &str = "collector.fill";
pub const TYPE_3Q5A: &str = "collector.3q5a";

pub const QUESTION_TYPES: [&str; 8] = [
    TYPE_ROLE,
    "collector.dialogue",
    TYPE_3Q5A,
    "collector.repeat",
    "collector.repeat_essay",
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
    q.insert("question".into(), Value::String(question));
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
        if let Some(xxlist) = xt.get("xxlist").and_then(|v| v.as_array()) {
            for o in xxlist {
                let mc = get_str(o.get("xx_mc").unwrap_or(&Value::Null));
                let nr = clean(&get_str(o.get("xx_nr").unwrap_or(&Value::Null)));
                if !nr.is_empty() || !mc.is_empty() {
                    options.push(format!("{mc}. {nr}"));
                }
            }
        }

        questions.push(make_question(
            Some(no),
            Some(no_source),
            TYPE_CHOOSE,
            stid,
            question_text,
            vec![clean(&get_str(xt.get("answer").unwrap_or(&Value::Null)))],
            options,
            Vec::new(),
            None,
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
        let no = get_i64(s.get("xth").unwrap_or(&Value::Null)).unwrap_or(i as i64 + 1);
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
    vec![make_question(
        None,
        None,
        TYPE_READ,
        stid,
        clean_lines(&get_str(info.and_then(|o| o.get("value")).unwrap_or(&Value::Null))),
        Vec::new(),
        Vec::new(),
        important,
        None,
    )]
}

pub fn collect_generic(
    content: &Value,
    content2: Option<&Value>,
    info: Option<&Value>,
    qtype: Option<&str>,
) -> Vec<Value> {
    let info_obj = content.get("info");
    if !info_obj.map_or(false, |v| v.is_object()) {
        return Vec::new();
    }
    let info_obj = info_obj.unwrap();
    if info_obj.get("question").map_or(false, |v| v.is_array())
        || info_obj.get("questions").map_or(false, |v| v.is_array())
    {
        return collect_question_based(content, content2, info, qtype);
    }
    if let Some(std) = info_obj.get("std").and_then(|v| v.as_array()) {
        if std
            .first()
            .and_then(|s| s.get("xth"))
            .map_or(false, |_| true)
        {
            return collect_fill(content, content2, info);
        }
        return collect_picture(content, content2, info);
    }
    if info_obj.get("xtlist").map_or(false, |v| v.is_array()) {
        return collect_choose(content, content2, info);
    }
    if info_obj.get("value").map_or(false, |v| !v.is_null()) {
        return collect_read(content, content2, info);
    }
    Vec::new()
}

pub fn parse_content(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Vec<Value> {
    let structure_type = content.get("structure_type").map(|v| get_str(v)).unwrap_or_default();
    let st = structure_type.as_str();
    if st.contains("choose") {
        return collect_choose(content, content2, info);
    }
    if st.contains("read") || st.contains("word") {
        return collect_read(content, content2, info);
    }
    if st.contains("fill") {
        return collect_fill(content, content2, info);
    }
    if st.contains("picture") {
        return collect_picture(content, content2, info);
    }
    if QUESTION_TYPES.contains(&st) {
        return collect_question_based(content, content2, info, Some(st));
    }
    if st.contains("repeat") || st.contains("essay") || st.contains("dialogue") || st.contains("role") {
        return collect_question_based(content, content2, info, Some(st));
    }
    collect_generic(content, content2, info, Some(st))
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
}
