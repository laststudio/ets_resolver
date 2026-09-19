// -*- coding: utf-8 -*-
// 统一接口: 单任务解析 / 批量解析 / 时间聚类 / 元信息 (与 resolver_py/api.py 行为一致)

use serde_json::{json, Value};

use crate::collectors::{parse_content, stid_of};

fn get_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// 单任务解析: 返回题目列表
pub fn parse_task(content: &Value, content2: Option<&Value>, info: Option<&Value>) -> Vec<Value> {
    parse_content(content, content2, info)
}

/// 元信息: {stid, structure_type, question_count}
pub fn extract_meta(content: &Value) -> Value {
    let questions = parse_content(content, None, None);
    if let Some(arr) = content.as_array() {
        let stid = arr.first().and_then(|item| item.get("word_id").or_else(|| item.get("id"))).and_then(|v| v.as_i64());
        json!({
            "stid": stid,
            "structure_type": "word_pk",
            "question_count": questions.len(),
        })
    } else {
        let stid = stid_of(content)
            .or_else(|| content.get("id").and_then(|v| v.as_i64()))
            .or_else(|| content.get("word_id").and_then(|v| v.as_i64()));
        let mut stype = content.get("structure_type").map(|v| get_str(v)).unwrap_or_default();
        if stype.is_empty() && (content.get("word_questions").is_some() || content.get("data").is_some()) {
            stype = "word_pk".to_string();
        }
        json!({
            "stid": stid,
            "structure_type": stype,
            "question_count": questions.len(),
        })
    }
}

fn assign_fallback_numbers(questions: &mut Vec<Value>) {
    let mut next_no: i64 = 1;
    for q in questions.iter_mut() {
        let no_source = q.get("_no_source").and_then(|v| v.as_str()).map(|s| s.to_string());
        if let Some(src) = &no_source {
            q.as_object_mut().map(|o| o.remove("_no_source"));
            if src == "global" || src == "local" {
                if let Some(no) = q.get("no").and_then(|v| v.as_i64()) {
                    next_no = next_no.max(no + 1);
                }
                continue;
            }
        }
        if let Some(no) = q.get("no").and_then(|v| v.as_i64()) {
            next_no = next_no.max(no + 1);
        } else {
            if let Some(o) = q.as_object_mut() {
                o.insert("no".into(), json!(next_no));
            }
            next_no += 1;
        }
    }
}

fn order_questions_in_section(questions: &mut Vec<Value>) {
    let mut indexed: Vec<(usize, Value)> = questions.drain(..).enumerate().collect();
    indexed.sort_by_key(|(i, q)| {
        match q.get("no").and_then(|v| v.as_i64()) {
            Some(no) => (0, no, *i as i64),
            None => (1, 0, *i as i64),
        }
    });
    *questions = indexed.into_iter().map(|(_, q)| q).collect();
}

/// 批量解析: tasks = [{content, info?, content2?, order?}, ...]
/// 返回 {sections: [{type, questions: [...]}]}
pub fn parse_batch(tasks: &[Value], sort_by_stid: bool) -> Value {
    let mut parsed: Vec<(Vec<Value>, i64)> = Vec::new();
    for (idx, task) in tasks.iter().enumerate() {
        if !task.is_object() {
            continue;
        }
        let content = task.get("content");
        if content.is_none() {
            continue;
        }
        let content = match content.unwrap() {
            Value::String(s) => match serde_json::from_str::<Value>(s) {
                Ok(v) => v,
                Err(_) => continue,
            },
            v => v.clone(),
        };
        let content2 = task
            .get("content2")
            .filter(|v| !v.is_null())
            .and_then(|v| match v {
                Value::String(s) => serde_json::from_str::<Value>(s).ok(),
                v => Some(v.clone()),
            });
        let info = task.get("info").filter(|v| !v.is_null()).and_then(|v| match v {
            Value::String(s) => serde_json::from_str::<Value>(s).ok(),
            v => Some(v.clone()),
        });
        let questions = parse_task(&content, content2.as_ref(), info.as_ref());
        let order = task
            .get("order")
            .and_then(|v| v.as_i64())
            .unwrap_or(idx as i64);
        parsed.push((questions, order));
    }
    if sort_by_stid {
        parsed.sort_by_key(|(questions, order)| {
            let stid = questions
                .first()
                .and_then(|q| q.get("stid"))
                .and_then(|v| v.as_i64())
                .unwrap_or(i64::MAX);
            (stid, *order)
        });
    }

    let mut sections: Vec<Value> = Vec::new();
    for (questions, _) in &parsed {
        for q in questions {
            let qtype = get_str(q.get("type").unwrap_or(&Value::Null));
            let last_is_same = sections
                .last()
                .map(|s| get_str(s.get("type").unwrap_or(&Value::Null)) == qtype)
                .unwrap_or(false);
            if last_is_same {
                if let Some(last) = sections.last_mut() {
                    if let Some(arr) = last.get_mut("questions").and_then(|v| v.as_array_mut()) {
                        arr.push(q.clone());
                    }
                }
            } else {
                sections.push(json!({"type": qtype, "questions": [q.clone()]}));
            }
        }
    }

    for section in sections.iter_mut() {
        if let Some(arr) = section.get_mut("questions").and_then(|v| v.as_array_mut()) {
            assign_fallback_numbers(arr);
            order_questions_in_section(arr);
            for q in arr.iter_mut() {
                if let Some(o) = q.as_object_mut() {
                    o.remove("_order");
                }
            }
        }
    }
    json!({"sections": sections})
}

/// 时间聚类: items = [[id, mtime_ms], ...], 相邻时间差 <= threshold 同组
/// 返回 [[id, ...], ...]
pub fn group_by_time(items: &[Value], threshold_ms: i64) -> Value {
    if items.is_empty() {
        return json!([]);
    }
    let mut ordered: Vec<(Value, i64)> = Vec::new();
    for item in items {
        let arr = match item.as_array() {
            Some(a) if a.len() >= 2 => a,
            _ => continue,
        };
        let mtime = match arr[1].as_i64() {
            Some(t) => t,
            None => match arr[1].as_str() {
                Some(s) => match s.trim().parse() {
                    Ok(t) => t,
                    Err(_) => continue,
                },
                None => continue,
            },
        };
        ordered.push((arr[0].clone(), mtime));
    }
    ordered.sort_by_key(|(_, t)| *t);

    let mut groups: Vec<Vec<Value>> = Vec::new();
    let mut current = vec![ordered[0].0.clone()];
    let mut prev_time = ordered[0].1;
    for (ident, mtime) in ordered.iter().skip(1) {
        if (mtime - prev_time).abs() <= threshold_ms {
            current.push(ident.clone());
        } else {
            groups.push(std::mem::take(&mut current));
            current = vec![ident.clone()];
        }
        prev_time = *mtime;
    }
    groups.push(current);
    json!(groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_time_basic() {
        let items = json!([["a", 1000], ["b", 1030], ["c", 1060], ["d", 5000], ["e", 5030]]);
        let arr = items.as_array().unwrap();
        assert_eq!(group_by_time(arr, 2000), json!([["a", "b", "c"], ["d", "e"]]));
    }

    #[test]
    fn test_parse_batch_sections() {
        let tasks = json!([
            {"content": {"structure_type": "collector.fill", "info": {"stid": "2", "std": [{"xth": "1", "value": "a"}]}}, "order": 2},
            {"content": {"structure_type": "collector.fill", "info": {"stid": "1", "std": [{"xth": "2", "value": "b"}]}}, "order": 1},
        ]);
        let result = parse_batch(tasks.as_array().unwrap(), true);
        let sections = result["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 1);
        let qs = sections[0]["questions"].as_array().unwrap();
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0]["no"], 1);
        assert_eq!(qs[1]["no"], 2);
    }

    #[test]
    fn test_extract_meta_pk() {
        let pk_data = json!({
            "word_questions": [
                {"id": 1, "question_type": 19, "word": "book", "translate": "书", "answer": ""}
            ],
            "id": 555
        });
        let meta = extract_meta(&pk_data);
        assert_eq!(meta["stid"], 555);
        assert_eq!(meta["structure_type"], "word_pk");
        assert_eq!(meta["question_count"], 1);
    }
}
