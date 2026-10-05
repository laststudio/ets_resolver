// -*- coding: utf-8 -*-
// 解析诊断 (parity 基础设施):
//   * 形状指纹 shape fingerprint —— 把一份资源包的结构压成一个稳定哈希，
//     用于"官方出了新题型"时的自动发现（新旧形状一眼可分，且不泄露题目文本）。
//   * diagnose() —— 回答四个问题：这是什么 structure_type / 命中了哪条路由 /
//     出了几道题 / 有没有可疑点。丢一个真实包进去即可定位"为什么没出题"。
//
// 设计约束：
//   * 纯函数、零 IO，可被 FFI / JNI / 测试直接调用。
//   * 输出里不含题目文本（preview 只给长度与计数），便于后端把诊断上报到日志。

use std::collections::BTreeSet;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::collectors::parse_content_routed;

/// 键路径递归的最大深度（更深的层级对形状区分没有新增信息）。
const MAX_DEPTH: usize = 8;
/// 单个报告里最多保留的键路径数（超出只计数，避免输出爆炸）。
const MAX_PATHS: usize = 512;
/// preview 里最多列出的题目数。
const PREVIEW_LIMIT: usize = 3;

fn walk(value: &Value, prefix: &str, depth: usize, out: &mut BTreeSet<String>) {
    if depth > MAX_DEPTH {
        return;
    }
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                out.insert(path.clone());
                walk(v, &path, depth + 1, out);
            }
        }
        Value::Array(items) => {
            let path = format!("{prefix}[]");
            out.insert(path.clone());
            // 数组只按元素键名并集记录，不区分下标 —— 形状相同、数量不同视为同种形状。
            for item in items {
                walk(item, &path, depth + 1, out);
            }
        }
        _ => {}
    }
}

/// 收集 JSON 里出现的全部键路径。数组折叠成 `prefix[]`。
pub fn key_paths(value: &Value) -> Vec<String> {
    let mut out = BTreeSet::new();
    walk(value, "", 0, &mut out);
    out.into_iter().collect()
}

/// 形状指纹：对排序后的键路径做 sha256，取前 16 位十六进制。
///
/// 不含任何取值、也不含 `structure_type`，因此：
///   * 官方只改了题型名字、结构没变 -> 指纹不变（可判定"仅是命名变化"）；
///   * 官方加了新的字段/层级 -> 指纹改变（可判定"结构变了，需要跟"）。
pub fn shape_fingerprint(paths: &[String]) -> String {
    let mut hasher = Sha256::new();
    for p in paths {
        hasher.update(p.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// 三份输入合并后的形状报告。
pub fn shape_report(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Value {
    let mut paths: BTreeSet<String> = BTreeSet::new();
    paths.extend(key_paths(content));
    if let Some(i) = info {
        paths.extend(key_paths(i));
    }
    if let Some(c) = content2 {
        paths.extend(key_paths(c));
    }

    let all_paths: Vec<String> = paths.into_iter().collect();
    let total = all_paths.len();
    let listed: Vec<String> = all_paths.iter().take(MAX_PATHS).cloned().collect();

    json!({
        "hash": shape_fingerprint(&all_paths),
        "path_count": total,
        "paths": listed,
        "truncated": total > listed.len(),
    })
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

/// 诊断一份资源包。输出结构稳定，可直接进日志或后台看板。
pub fn diagnose(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Value {
    let routed = parse_content_routed(content, content2, info);
    let questions = &routed.questions;

    let structure_type = string_field(content, "structure_type");
    let mut warnings: Vec<&'static str> = Vec::new();

    if structure_type.is_empty() {
        warnings.push("structure_type_missing");
    }
    match routed.layer {
        "generic" => warnings.push("generic_fallback"),
        "none" => warnings.push("unmatched_shape"),
        _ => {}
    }
    if questions.is_empty() {
        warnings.push("no_questions");
    }

    let mut answer_count = 0usize;
    let mut empty_answer_questions = 0usize;
    let mut dirty_text_questions = 0usize;
    let mut unnumbered = 0usize;
    for q in questions {
        let answers = q
            .get("answers")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        answer_count += answers.len();
        if answers.is_empty() {
            empty_answer_questions += 1;
        }
        if q.get("no").map_or(true, |n| n.is_null()) {
            unnumbered += 1;
        }
        let text = q.get("question").and_then(|v| v.as_str()).unwrap_or("");
        if text.contains('<') || text.contains('\u{200b}') {
            dirty_text_questions += 1;
        }
    }
    if empty_answer_questions > 0 {
        warnings.push("answers_empty");
    }
    if unnumbered > 0 {
        warnings.push("no_numbering");
    }
    if dirty_text_questions > 0 {
        warnings.push("dirty_text");
    }

    let preview: Vec<Value> = questions
        .iter()
        .take(PREVIEW_LIMIT)
        .map(|q| {
            json!({
                "no": q.get("no").cloned().unwrap_or(Value::Null),
                "type": q.get("type").cloned().unwrap_or(Value::Null),
                "question_len": q.get("question").and_then(|v| v.as_str()).map(|s| s.chars().count()).unwrap_or(0),
                "answers": q.get("answers").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0),
                "options": q.get("options").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0),
            })
        })
        .collect();

    json!({
        "structure_type": structure_type,
        "layer": routed.layer,
        "family": routed.family,
        "question_count": questions.len(),
        "answer_count": answer_count,
        "shape": shape_report(content, content2, info),
        "warnings": warnings,
        "preview": preview,
        "input": input_presence(content, content2, info),
    })
}

/// 输入三元组的在场情况（排错时最先要确认的就是"该给的 JSON 给了没有"）。
fn input_presence(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Value {
    let describe = |v: Option<&Value>| -> Value {
        match v {
            None => json!({"present": false, "bytes": 0}),
            Some(val) => json!({
                "present": true,
                "bytes": serde_json::to_string(val).map(|s| s.len()).unwrap_or(0),
                "keys": val.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            }),
        }
    };
    json!({
        "content": describe(Some(content)),
        "info": describe(info),
        "content2": describe(content2),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_ignores_values_and_counts() {
        let a = json!({"info": {"stid": "1", "std": [{"xth": "1", "value": "x"}]}});
        let b = json!({"info": {"stid": "999", "std": [{"xth": "7", "value": "y"}, {"xth": "8", "value": "z"}]}});
        assert_eq!(
            shape_report(&a, None, None)["hash"],
            shape_report(&b, None, None)["hash"]
        );
    }

    #[test]
    fn fingerprint_changes_when_structure_changes() {
        let a = json!({"info": {"std": [{"xth": "1", "value": "x"}]}});
        let b = json!({"info": {"std": [{"xth": "1", "value": "x"}], "extra": 1}});
        assert_ne!(
            shape_report(&a, None, None)["hash"],
            shape_report(&b, None, None)["hash"]
        );
    }

    #[test]
    fn fingerprint_ignores_structure_type_name() {
        let a = json!({"structure_type": "collector.picture", "info": {"std": [{"value": "x"}]}});
        let b = json!({"structure_type": "collector.brand_new_2027", "info": {"std": [{"value": "x"}]}});
        assert_eq!(
            shape_report(&a, None, None)["hash"],
            shape_report(&b, None, None)["hash"]
        );
    }

    #[test]
    fn diagnose_marks_explicit_route() {
        let content = json!({
            "structure_type": "collector.choose",
            "info": {"stid": "3", "xtlist": [
                {"xt_xh": "1", "xt_nr": "Q", "answer": "B", "xxlist": [
                    {"xx_mc": "A", "xx_nr": "a"}, {"xx_mc": "B", "xx_nr": "b"}
                ]}
            ]}
        });
        let report = diagnose(&content, None, None);
        assert_eq!(report["layer"], "explicit");
        assert_eq!(report["family"], "collector.choose");
        assert_eq!(report["question_count"], 1);
        assert_eq!(report["answer_count"], 1);
    }

    #[test]
    fn diagnose_flags_unknown_but_sniffable_shape() {
        let content = json!({
            "structure_type": "collector.unknown_2027",
            "info": {"stid": "9", "question": [
                {"ask": "Q1", "std": [{"value": "A1"}]}
            ]}
        });
        let report = diagnose(&content, None, None);
        assert_eq!(report["layer"], "generic");
        assert_eq!(report["family"], "generic:question_based");
        let warnings = report["warnings"].as_array().unwrap();
        assert!(warnings.iter().any(|w| w == "generic_fallback"));
        assert_eq!(report["question_count"], 1);
    }

    #[test]
    fn diagnose_flags_unmatched_shape() {
        let content = json!({"structure_type": "collector.mystery", "info": {"stid": "1"}});
        let report = diagnose(&content, None, None);
        assert_eq!(report["layer"], "none");
        assert_eq!(report["question_count"], 0);
        let warnings = report["warnings"].as_array().unwrap();
        assert!(warnings.iter().any(|w| w == "unmatched_shape"));
        assert!(warnings.iter().any(|w| w == "no_questions"));
    }

    #[test]
    fn diagnose_never_leaks_question_text() {
        let content = json!({
            "structure_type": "collector.role",
            "info": {"stid": "1", "question": [
                {"ask": "SECRET_QUESTION_TEXT", "std": [{"value": "SECRET_ANSWER"}]}
            ]}
        });
        let report = diagnose(&content, None, None).to_string();
        assert!(!report.contains("SECRET_QUESTION_TEXT"));
        assert!(!report.contains("SECRET_ANSWER"));
    }
}
