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
        if stype.is_empty() && (
            content.get("word_question").is_some()
            || content.get("word_questions").is_some()
            || content.get("data").is_some()
            || content.get("word").is_some()
            || content.get("words").is_some()
        ) {
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

    // word 类题型（单词 PK / 词汇）才启用"任务顺序优先"。
    //
    // 原因：PK 的 no 是**各任务内部**的局部编号（一个任务一道题时全是 1），
    // 多个任务的题会被合并进同一个分节，只按 no 排就会退化成"按原始下标交叉"，
    // 顺序看起来完全乱掉。其它题型（collector.*）的 no 是全局题号，且由
    // assign_fallback_numbers 按数组顺序补号，改用任务顺序会和补号顺序打架，
    // 所以它们维持原有行为不动。
    let word_like = questions
        .iter()
        .any(|q| is_word_type(&get_str(q.get("type").unwrap_or(&Value::Null))));

    indexed.sort_by_key(|(i, q)| {
        let task_order = if word_like {
            q.get("_order").and_then(|v| v.as_i64()).unwrap_or(i64::MAX)
        } else {
            0
        };
        match q.get("no").and_then(|v| v.as_i64()) {
            Some(no) => (0, task_order, no, *i as i64),
            None => (1, task_order, 0, *i as i64),
        }
    });
    *questions = indexed.into_iter().map(|(_, q)| q).collect();
}

/// 是否为单词 PK / 词汇类题型（这些题型的 stid 是 word_id，不代表卷面结构顺序）
fn is_word_type(qtype: &str) -> bool {
    qtype.starts_with("word.") || qtype == crate::collectors::TYPE_WORD
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
            let first = questions.first();
            let stid = first
                .and_then(|q| q.get("stid"))
                .and_then(|v| v.as_i64())
                .unwrap_or(i64::MAX);
            let qtype = first
                .and_then(|q| q.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if is_word_type(qtype) {
                // 单词 PK / 词汇：stid 是 word_id，跟卷面顺序无关。
                // 用调用方给的任务顺序，否则用户会看到"顺序完全乱了"。
                (*order, stid)
            } else {
                // 其它题型：stid 是官方结构 id，用它还原卷面分节顺序。
                (stid, *order)
            }
        });
    }

    let mut sections: Vec<Value> = Vec::new();
    for (questions, task_order) in &parsed {
        for q in questions {
            // 打上来源任务序号，供分节内排序使用（解析结束时会被清掉）
            let q = {
                let mut q = q.clone();
                if let Some(o) = q.as_object_mut() {
                    o.insert("_order".into(), json!(*task_order));
                }
                q
            };
            let q = &q;
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

    /// 单词 PK：调用方给的任务顺序必须胜出。
    /// 回归背景：stid 在 PK 里是 word_id，早期用 (stid, order) 排序会把对战顺序打乱，
    /// 用户看到的是"按 word_id 从小到大"而不是实际对战顺序。
    #[test]
    fn test_parse_batch_pk_keeps_caller_order() {
        let mk = |wid: i64, word: &str| {
            json!({
                "content": {"data": [{"id": wid, "content": word, "translate": "译",
                    "question": [{"id": wid + 100, "question_type": 6, "word": word,
                                  "answer": "A", "option": [{"name": "A", "value": "甲"}]}]}]},
                "order": 0
            })
        };
        // 对战顺序故意不是 word_id 升序
        let order = [(1003, "orange"), (1001, "apple"), (1005, "grape"), (1002, "banana")];
        let mut tasks = Vec::new();
        for (i, (wid, word)) in order.iter().enumerate() {
            let mut t = mk(*wid, word);
            t["order"] = json!(i);
            tasks.push(t);
        }
        let result = parse_batch(&tasks, true);
        let qs = result["sections"][0]["questions"].as_array().unwrap();
        let got: Vec<i64> = qs.iter().map(|q| q["word_id"].as_i64().unwrap()).collect();
        assert_eq!(got, vec![1003, 1001, 1005, 1002]);
        // word_id / question_id 必须都在，提交时要按它们对应
        assert_eq!(qs[0]["question_id"], 1103);   // wid + 100
    }

    /// 同一任务内部的题号顺序仍然生效（不能被 _order 抹平）。
    #[test]
    fn test_parse_batch_pk_keeps_intra_task_numbering() {
        let tasks = vec![json!({
            "content": {"word_questions": [
                {"id": 11, "question_type": 6, "word": "a", "no": 2, "answer": "A", "option": [{"name": "A", "value": "x"}]},
                {"id": 12, "question_type": 6, "word": "b", "no": 1, "answer": "A", "option": [{"name": "A", "value": "y"}]},
            ]},
            "order": 0
        })];
        let result = parse_batch(&tasks, true);
        let qs = result["sections"][0]["questions"].as_array().unwrap();
        assert_eq!(qs[0]["question_id"], 12);   // no=1 的在前
        assert_eq!(qs[1]["question_id"], 11);
    }

    /// 非 word 题型维持原有行为：分节内按 stid 还原的卷面顺序，不受任务顺序影响。
    #[test]
    fn test_parse_batch_non_word_keeps_stid_order() {
        let tasks = json!([
            {"content": {"structure_type": "collector.fill", "info": {"stid": "30", "std": [{"xth": "1", "value": "c"}]}}, "order": 0},
            {"content": {"structure_type": "collector.fill", "info": {"stid": "10", "std": [{"xth": "1", "value": "a"}]}}, "order": 1},
            {"content": {"structure_type": "collector.fill", "info": {"stid": "20", "std": [{"xth": "1", "value": "b"}]}}, "order": 2},
        ]);
        let result = parse_batch(tasks.as_array().unwrap(), true);
        let sections = result["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 1);   // 同类型合并成一个分节
        let stids: Vec<i64> = sections[0]["questions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["stid"].as_i64().unwrap())
            .collect();
        assert_eq!(stids, vec![10, 20, 30]);   // 仍按 stid 还原卷面顺序
    }

    /// 合并后 _order 必须被清掉，不能泄漏给调用方。
    #[test]
    fn test_parse_batch_strips_internal_order_field() {
        let tasks = vec![json!({
            "content": {"word_questions": [{"id": 1, "question_type": 6, "word": "a", "answer": "A", "option": [{"name": "A", "value": "x"}]}]},
            "order": 0
        })];
        let result = parse_batch(&tasks, true);
        let q = &result["sections"][0]["questions"][0];
        assert!(q.get("_order").is_none());
        assert!(q.get("_no_source").is_none());
    }

    /// 载荷带显式 word_ids / question_ids 时，以它为准（客户端对战顺序的权威来源）。
    #[test]
    fn test_pk_honours_explicit_word_ids_sequence() {
        let data = (1001..=1005)
            .map(|wid| {
                json!({"id": wid, "content": format!("w{}", wid), "translate": "译",
                       "question": [{"id": wid + 100, "question_type": 6, "word": format!("w{}", wid),
                                     "answer": "A", "option": [{"name": "A", "value": "甲"}]}]})
            })
            .collect::<Vec<_>>();
        // 服务端按 id 升序返回，但真实对战顺序是 1003_1001_1005_1002_1004
        let content = json!({
            "data": data,
            "word_ids": "1003_1001_1005_1002_1004",
            "question_ids": "203_201_205_202_204",
        });
        let questions = crate::collectors::collect_word_pk(&content);
        let got: Vec<i64> = questions.iter().map(|q| q["word_id"].as_i64().unwrap()).collect();
        assert_eq!(got, vec![1003, 1001, 1005, 1002, 1004]);
        // 重排后题号重新连续编号
        let nos: Vec<i64> = questions.iter().map(|q| q["no"].as_i64().unwrap()).collect();
        assert_eq!(nos, vec![1, 2, 3, 4, 5]);
    }

    /// 数组形式也认；且对不上号时不能把顺序搞得更乱。
    #[test]
    fn test_pk_explicit_order_array_form_and_safe_fallback() {
        let data = (7..=8)
            .map(|wid| {
                json!({"id": wid, "content": format!("w{}", wid), "translate": "译",
                       "question": [{"id": wid + 100, "question_type": 6, "word": format!("w{}", wid),
                                     "answer": "A", "option": [{"name": "A", "value": "甲"}]}]})
            })
            .collect::<Vec<_>>();
        let ok = json!({"data": data.clone(), "word_ids": [8, 7]});
        let got: Vec<i64> = crate::collectors::collect_word_pk(&ok)
            .iter()
            .map(|q| q["word_id"].as_i64().unwrap())
            .collect();
        assert_eq!(got, vec![8, 7]);

        // 序列里的 id 跟题目完全对不上：保持原顺序，不重排
        let bad = json!({"data": data, "word_ids": "999_998"});
        let got2: Vec<i64> = crate::collectors::collect_word_pk(&bad)
            .iter()
            .map(|q| q["word_id"].as_i64().unwrap())
            .collect();
        assert_eq!(got2, vec![7, 8]);
    }

    /// 对战 PK 只保留题型 19（英选汉）——官方 B0() 对听选 6 / 拼写 16 / 听读 17/18 直接 remove。
    /// 不过滤就会解析出官方 PK 根本不问的题，题量/顺序全对不上。
    #[test]
    fn test_pk_battle_filters_to_en_to_cn_only() {
        let content = json!({
            "pk_mode": "battle",
            "data": [{"id": 1001, "content": "abandon", "translate": "放弃",
                "question": [
                    {"id": 2001, "question_type": 19, "word": "abandon", "translate": "放弃",
                     "answer": "放弃", "option": [{"name": "A", "value": "放弃"}]},
                    {"id": 2002, "question_type": 16, "word": "abandon", "translate": "放弃", "answer": "abandon"},
                    {"id": 2003, "question_type": 6, "word": "abandon", "translate": "放弃",
                     "answer": "放弃", "option": [{"name": "A", "value": "放弃"}]},
                    {"id": 2004, "question_type": 17, "word": "abandon", "translate": "放弃", "answer": "abandon"}
                ]}]
        });
        let qs = crate::collectors::collect_word_pk(&content);
        let ids: Vec<i64> = qs.iter().map(|q| q["question_id"].as_i64().unwrap()).collect();
        assert_eq!(ids, vec![2001]);           // 只剩英选汉
    }

    /// 竞赛模式保留 19 + 20（isTypeEnToCn() || isTypeCnToEn()）。
    #[test]
    fn test_pk_match_mode_keeps_both_translate_types() {
        let content = json!({
            "pk_mode": "match",
            "data": [{"id": 1, "content": "book", "translate": "书",
                "question": [
                    {"id": 11, "question_type": 19, "word": "book", "translate": "书", "answer": "书"},
                    {"id": 12, "question_type": 20, "word": "book", "translate": "书", "answer": "book"},
                    {"id": 13, "question_type": 16, "word": "book", "translate": "书", "answer": "book"}
                ]}]
        });
        let qs = crate::collectors::collect_word_pk(&content);
        let ids: Vec<i64> = qs.iter().map(|q| q["question_id"].as_i64().unwrap()).collect();
        assert_eq!(ids, vec![11, 12]);
    }

    /// 词汇作业（非对战）绝不能过滤：拼写/朗读/听写正是它的正题。
    #[test]
    fn test_vocabulary_homework_is_not_filtered() {
        let content = json!({
            "data": [{"id": 1, "content": "book", "translate": "书",
                "question": [
                    {"id": 11, "question_type": 16, "word": "book", "translate": "书", "answer": "book"},
                    {"id": 12, "question_type": 17, "word": "book", "translate": "书", "answer": "book"},
                    {"id": 13, "question_type": 19, "word": "book", "translate": "书", "answer": "书"}
                ]}]
        });
        let qs = crate::collectors::collect_word_pk(&content);
        assert_eq!(qs.len(), 3);
    }

    /// 过滤后一条不剩时不许把题目清空（安全阀）。
    #[test]
    fn test_pk_filter_keeps_original_when_everything_would_be_removed() {
        let content = json!({
            "pk_mode": "battle",
            "data": [{"id": 1, "content": "book", "translate": "书",
                "question": [{"id": 11, "question_type": 16, "word": "book", "translate": "书", "answer": "book"}]}]
        });
        let qs = crate::collectors::collect_word_pk(&content);
        assert_eq!(qs.len(), 1);   // 宁可不过滤，也不能返回空
    }

    /// 没有显式顺序时给确定性排序（/word/struct 不承诺顺序，两次调用不该不同）。
    #[test]
    fn test_pk_deterministic_order_without_explicit_ids() {
        let mk = |wid: i64| json!({"id": wid, "content": "w", "translate": "译",
            "question": [{"id": wid * 10, "question_type": 19, "word": "w", "translate": "译", "answer": "译"}]});
        let content = json!({"pk_mode": "battle", "data": [mk(1003), mk(1001), mk(1002)]});
        let qs = crate::collectors::collect_word_pk(&content);
        let got: Vec<i64> = qs.iter().map(|q| q["word_id"].as_i64().unwrap()).collect();
        assert_eq!(got, vec![1001, 1002, 1003]);
    }

    /// 朗读/复述题的答案必须带上朗读文本（后端靠 answers 取文本，早期这里是空的）。
    #[test]
    fn test_read_and_repeat_essay_emit_answer_text() {
        let passage = "Hello, this is the reading passage.";
        for stype in ["collector.read", "collector.repeat_essay"] {
            let content = json!({"structure_type": stype, "info": {"stid": 11, "value": passage}});
            let qs = crate::collectors::collect_word_pk(&content); // 走通用入口不方便，这里直接验 collect_read/essay
            let _ = qs;
        }
        let read = json!({"structure_type": "collector.read", "info": {"stid": 11, "value": passage}});
        let qs = crate::collectors::collect_read(&read, None, None);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0]["answers"][0], passage);
        assert_eq!(qs[0]["question"], passage);

        let essay = json!({"structure_type": "collector.repeat_essay",
                           "info": {"stid": 12, "sublist": [{"text": passage}]}});
        let qs2 = crate::collectors::collect_repeat_essay(&essay, None, None);
        assert_eq!(qs2.len(), 1);
        assert_eq!(qs2[0]["answers"][0], passage);
    }

    /// collector.read 带 sublist 时一个条目一题（与后端 Python 实现一致）。
    #[test]
    fn test_read_sublist_produces_one_question_per_item() {
        let content = json!({"structure_type": "collector.read",
                             "info": {"stid": 13, "sublist": [{"text": "First."}, {"text": "Second."}]}});
        let qs = crate::collectors::collect_read(&content, None, None);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0]["answers"][0], "First.");
        assert_eq!(qs[1]["answers"][0], "Second.");
        assert_eq!(qs[0]["no"], 1);
        assert_eq!(qs[1]["no"], 2);
    }

    /// 3q5a 这类只有 info.std[] 的资源包不能整个题型解析为空。
    #[test]
    fn test_3q5a_falls_back_to_info_std() {
        let content = json!({"structure_type": "collector.3q5a",
                             "info": {"stid": 17, "std": [{"xth": 1, "value": "Answer one."},
                                                          {"xth": 2, "value": "Answer two."}]}});
        let qs = parse_task(&content, None, None);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0]["answers"][0], "Answer one.");
        assert_eq!(qs[0]["question"], "问题 1");
    }

    /// 题干清洗：<br>/<p> 变空格、折叠空白 —— 不能残留标签，也不能把单词粘一起。
    #[test]
    fn test_question_text_cleans_break_tags() {
        let content = json!({"structure_type": "collector.role", "info": {"stid": 21, "question": [
            {"ask": "ets_th1 What is Lisa reading?<br>(A / B / C)", "std": [{"value": "A"}]}]}});
        let qs = parse_task(&content, None, None);
        assert_eq!(qs[0]["question"], "What is Lisa reading? (A / B / C)");
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
