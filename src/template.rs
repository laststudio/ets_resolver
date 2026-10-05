// -*- coding: utf-8 -*-
// res.json 模板权威解析 (TemplateResBean)
//
// 反编译结论（TemplateResBean / ChildPaperItemBean）：
//   * 大题顺序 = `exam_type_list[]` 的数组下标顺序；
//   * `exam_type_list[].exam_type_collect` 与 content.json 的 `structure_type` 对应，
//     是把"资源包"挂到"卷面大题"上的唯一钥匙；
//   * 小题满分 = `exam_list[].question_list[].question_score`（**权威**，不要再用
//     group_name 权重推算）；
//   * 小题顺序 = `question_order`，结合 info 的 `xth` / `th` 递增；
//   * `exam_info[].value` 是卷面指示文本，不是题号。
//
// 本模块把这些结构规范化，并提供"把满分/题号盖到已解析题目上"的能力，
// 用于闭合《PARITY.md》的 E5（卷面顺序）与 E6（卷面总分/分类）。

use serde_json::{json, Map, Value};

/// 宽容取字段：同时接受 snake_case 与 camelCase（官方不同版本/接口大小写不一致）。
fn get<'a>(obj: &'a Value, snake: &str, camel: &str) -> Option<&'a Value> {
    obj.get(snake)
        .filter(|v| !v.is_null())
        .or_else(|| obj.get(camel).filter(|v| !v.is_null()))
}

fn get_str(obj: &Value, snake: &str, camel: &str) -> String {
    match get(obj, snake, camel) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn get_f64(obj: &Value, snake: &str, camel: &str) -> Option<f64> {
    match get(obj, snake, camel) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
}

fn as_array(value: Option<&Value>) -> Vec<Value> {
    value.and_then(|v| v.as_array()).cloned().unwrap_or_default()
}

/// 规范化一份 res.json 模板。
///
/// 真实字段（实测 `834c62a165faa2e86182c97bba2ab925.zip` 内的 res.json，广东 30 分卷）：
/// ```jsonc
/// {
///   "set_score": "30",
///   "exam_type_list": [{
///     "exam_type_order": "1", "exam_type_name": "模仿朗读",
///     "exam_type_collect": "collector.read", "exam_type_score": "6", "exam_type_count": "1",
///     "exam_list": [{
///       "exam_order": "1", "exam_id": "{{st1}}", "exam_score": "6", "exam_count": "1",
///       "question_list": [{
///         "question_order": "1", "question_count": "1", "question_score": "6",
///         "evaluation_type": "read_chapter", "engine_area": "", "engine_type": "study"
///       }]
///     }]
///   }]
/// }
/// ```
pub fn parse(res: &Value) -> Value {
    let type_list = as_array(get(res, "exam_type_list", "examTypeList"));
    let mut sections: Vec<Value> = Vec::with_capacity(type_list.len());
    let mut section_order: Vec<String> = Vec::with_capacity(type_list.len());
    let mut section_names: Vec<String> = Vec::with_capacity(type_list.len());
    let mut warnings: Vec<String> = Vec::new();
    let mut exam_score_sum = 0.0f64;
    let mut question_score_sum = 0.0f64;
    let mut question_total = 0usize;

    for (index, exam_type) in type_list.iter().enumerate() {
        let name = get_str(exam_type, "exam_type_name", "examTypeName");
        let collect = get_str(exam_type, "exam_type_collect", "examTypeCollect");
        // 大题序号：官方字段优先（数组顺序与它一致，但它是显式的、可用于校验）
        let type_order = get_f64(exam_type, "exam_type_order", "examTypeOrder")
            .map(|v| v as i64)
            .unwrap_or(index as i64 + 1);
        let type_score = get_f64(exam_type, "exam_type_score", "examTypeScore");
        if collect.is_empty() {
            warnings.push(format!("section[{index}] 缺少 exam_type_collect，无法与资源包对应"));
        }
        section_order.push(collect.clone());
        section_names.push(name.clone());

        let mut questions: Vec<Value> = Vec::new();
        let mut section_score_from_exams = 0.0f64;
        let mut section_has_exam_score = false;

        for exam in as_array(get(exam_type, "exam_list", "examList")) {
            let exam_id = get_str(&exam, "exam_id", "examId");
            if let Some(score) = get_f64(&exam, "exam_score", "examScore") {
                section_has_exam_score = true;
                section_score_from_exams += score;
            }
            for (q_index, q) in as_array(get(&exam, "question_list", "questionList"))
                .iter()
                .enumerate()
            {
                let order = get_f64(q, "question_order", "questionOrder")
                    .map(|v| v as i64)
                    // 缺 question_order 时退回数组下标，保证顺序仍然稳定
                    .unwrap_or(q_index as i64);
                let count = get_f64(q, "question_count", "questionCount")
                    .map(|v| v as i64)
                    .unwrap_or(1);
                let score = get_f64(q, "question_score", "questionScore").unwrap_or(0.0);
                if score > 0.0 {
                    question_score_sum += score;
                }
                question_total += 1;
                questions.push(json!({
                    "order": order,
                    "count": count,
                    "score": score,
                    "evaluation_type": get_str(q, "evaluation_type", "evaluationType"),
                    "engine_type": get_str(q, "engine_type", "engineType"),
                    "engine_area": get_str(q, "engine_area", "engineArea"),
                    "exam_id": exam_id,
                }));
            }
        }

        // 大题分优先级：exam_list[].exam_score 求和 → exam_type_score（真实文件两级都有）
        let (section_score, section_score_source) = if section_has_exam_score {
            (section_score_from_exams, "exam_score_sum")
        } else if let Some(score) = type_score {
            (score, "exam_type_score")
        } else {
            (
                questions
                    .iter()
                    .filter_map(|q| q.get("score").and_then(|v| v.as_f64()))
                    .sum(),
                "question_score_sum",
            )
        };
        if section_has_exam_score {
            exam_score_sum += section_score_from_exams;
        } else if let Some(score) = type_score {
            exam_score_sum += score;
        }

        sections.push(json!({
            "index": index,
            "exam_type_order": type_order,
            "exam_type_name": name,
            "exam_type_collect": collect,
            "exam_type_score": type_score,
            "exam_score": section_score,
            "score_source": section_score_source,
            "question_count": questions.len(),
            "question_score_total": questions.iter()
                .filter_map(|q| q.get("score").and_then(|v| v.as_f64()))
                .sum::<f64>(),
            "questions": questions,
        }));
    }

    // 大题顺序以 exam_type_order 显式排序为准（真实文件里有该字段）
    sections.sort_by_key(|s| {
        s.get("exam_type_order")
            .and_then(|v| v.as_i64())
            .unwrap_or(i64::MAX)
    });

    // 卷面总分优先级：set_score（官方显式）→ exam_score 合计 → 逐题和
    let set_score = get_f64(res, "set_score", "setScore");
    let (total_score, total_score_source) = match set_score {
        Some(v) if v > 0.0 => (v, "set_score"),
        _ if exam_score_sum > 0.0 => (exam_score_sum, "exam_score_sum"),
        _ if question_score_sum > 0.0 => (question_score_sum, "question_score_sum"),
        _ => (0.0, "none"),
    };
    if total_score == 0.0 && question_total > 0 {
        warnings.push("res.json 未提供任何分值，无法校正卷面总分".into());
    }
    // 官方 set_score 与逐题和/大题和不一致时要能看见（不静默采信任一方）
    if let Some(v) = set_score {
        if v > 0.0 && question_score_sum > 0.0 && (v - question_score_sum).abs() > 0.01 {
            warnings.push(format!(
                "set_score({v}) 与逐题满分之和({question_score_sum})不一致，请核对模板"
            ));
        }
    }
    warnings.dedup();

    json!({
        "sections": sections,
        "section_order": section_order,
        "section_names": section_names,
        "set_score": set_score,
        "total_score": total_score,
        "total_score_source": total_score_source,
        "question_count": question_total,
        "warnings": warnings,
    })
}

/// 从模板里取某个 `structure_type` 对应的分节（多包合并时按此排序）。
pub fn section_for_collect(template: &Value, structure_type: &str) -> Option<Value> {
    template
        .get("sections")?
        .as_array()?
        .iter()
        .find(|s| {
            s.get("exam_type_collect")
                .and_then(|v| v.as_str())
                .map(|c| c.eq_ignore_ascii_case(structure_type))
                .unwrap_or(false)
        })
        .cloned()
}

/// 把模板的满分与题号盖到已解析题目上。
///
/// 规则（与官方一致）：
///   * `score` 取 `question_list[].question_score`（权威）；
///   * `no` 仅在解析结果原本没有题号时补，用 `question_order` 的累计序号；
///   * 不改变题目的内容、顺序与答案。
pub fn apply_section(template: &Value, section_index: usize, questions: &mut [Value]) -> Value {
    let section = template
        .get("sections")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.get(section_index))
        .cloned();
    let Some(section) = section else {
        return json!({"applied": false, "reason": "section_index 越界"});
    };
    let template_questions = as_array(section.get("questions"));

    let mut filled_no = 0usize;
    let mut filled_score = 0usize;
    let mut next_no = 0i64;
    for (i, q) in questions.iter_mut().enumerate() {
        let Some(tq) = template_questions.get(i) else {
            break;
        };
        if let (Some(obj), Some(entry)) = (q.as_object_mut(), tq.as_object()) {
            let order = entry.get("order").and_then(|v| v.as_i64()).unwrap_or(i as i64);
            next_no = next_no.max(order);
            if obj.get("no").map_or(true, |v| v.is_null()) {
                next_no += 1;
                obj.insert("no".into(), json!(next_no));
                filled_no += 1;
            } else {
                next_no = obj
                    .get("no")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(next_no);
            }
            if let Some(score) = entry.get("score").and_then(|v| v.as_f64()) {
                if obj.get("score").is_none() {
                    filled_score += 1;
                }
                obj.insert("score".into(), json!(score));
            }
            let mut tpl = Map::new();
            tpl.insert("evaluation_type".into(), entry.get("evaluation_type").cloned().unwrap_or(Value::Null));
            tpl.insert("engine_type".into(), entry.get("engine_type").cloned().unwrap_or(Value::Null));
            tpl.insert("exam_id".into(), entry.get("exam_id").cloned().unwrap_or(Value::Null));
            obj.insert("template".into(), Value::Object(tpl));
        }
    }

    json!({
        "applied": true,
        "section_index": section_index,
        "exam_type_name": section.get("exam_type_name").cloned().unwrap_or(Value::Null),
        "exam_type_collect": section.get("exam_type_collect").cloned().unwrap_or(Value::Null),
        "filled_no": filled_no,
        "filled_score": filled_score,
        "question_count": questions.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn guangdong_res() -> Value {
        json!({
            "exam_type_list": [
                {
                    "exam_type_name": "模仿朗读",
                    "exam_type_collect": "collector.read",
                    "exam_list": [{
                        "exam_id": "exam_reading_01",
                        "exam_score": "6.0",
                        "exam_info": [{"id": "prep_time", "value": "60"}],
                        "question_list": [{
                            "question_order": "0", "question_count": "1", "question_score": "6.0",
                            "evaluation_type": "read_passage", "engine_type": "eduspeak"
                        }]
                    }]
                },
                {
                    "exam_type_name": "听选信息",
                    "exam_type_collect": "listen_choice_standard",
                    "exam_list": [{
                        "exam_id": "exam_choice_01",
                        "exam_score": "3.0",
                        "question_list": [
                            {"question_order": "0", "question_count": "1", "question_score": "1.5", "evaluation_type": "choice"},
                            {"question_order": "1", "question_count": "1", "question_score": "1.5", "evaluation_type": "choice"}
                        ]
                    }]
                }
            ]
        })
    }

    #[test]
    fn parse_keeps_section_order_and_scores() {
        let t = parse(&guangdong_res());
        assert_eq!(t["section_order"][0], "collector.read");
        assert_eq!(t["section_order"][1], "listen_choice_standard");
        assert_eq!(t["section_names"][0], "模仿朗读");
        assert_eq!(t["total_score"], 9.0);
        assert_eq!(t["total_score_source"], "exam_score_sum");
        assert_eq!(t["question_count"], 3);
        let s1 = &t["sections"][1];
        assert_eq!(s1["exam_score"], 3.0);
        assert_eq!(s1["questions"][0]["score"], 1.5);
        assert_eq!(s1["questions"][1]["evaluation_type"], "choice");
    }

    #[test]
    fn accepts_camel_case_keys() {
        let camel = json!({
            "examTypeList": [{
                "examTypeName": "听选信息",
                "examTypeCollect": "listen_choice_standard",
                "examList": [{
                    "examId": "e1",
                    "examScore": "3.0",
                    "questionList": [
                        {"questionOrder": "0", "questionCount": "1", "questionScore": "1.5"}
                    ]
                }]
            }]
        });
        let t = parse(&camel);
        assert_eq!(t["section_order"][0], "listen_choice_standard");
        assert_eq!(t["total_score"], 3.0);
        assert_eq!(t["sections"][0]["questions"][0]["score"], 1.5);
    }

    #[test]
    fn falls_back_to_question_score_when_exam_score_missing() {
        let res = json!({
            "exam_type_list": [{
                "exam_type_name": "听选信息",
                "exam_type_collect": "listen_choice_standard",
                "exam_list": [{
                    "question_list": [
                        {"question_order": "0", "question_score": "1.5"},
                        {"question_order": "1", "question_score": "2.5"}
                    ]
                }]
            }]
        });
        let t = parse(&res);
        assert_eq!(t["total_score"], 4.0);
        assert_eq!(t["total_score_source"], "question_score_sum");
        assert_eq!(t["sections"][0]["exam_score"], 4.0);
    }

    #[test]
    fn warns_when_collect_missing_or_scores_absent() {
        let res = json!({"exam_type_list": [{"exam_type_name": "X", "exam_list": []}]});
        let t = parse(&res);
        let warnings = t["warnings"].as_array().unwrap();
        assert!(warnings.iter().any(|w| w.as_str().unwrap().contains("exam_type_collect")));
    }

    #[test]
    fn section_for_collect_matches_regardless_of_case() {
        let t = parse(&guangdong_res());
        let s = section_for_collect(&t, "LISTEN_CHOICE_STANDARD").unwrap();
        assert_eq!(s["exam_type_name"], "听选信息");
    }

    #[test]
    fn apply_section_fills_scores_and_numbering_without_touching_content() {
        let t = parse(&guangdong_res());
        let mut questions = vec![
            json!({"no": Value::Null, "type": "collector.choose", "question": "Q1", "answers": ["A"], "options": ["A. a"]}),
            json!({"no": 7, "type": "collector.choose", "question": "Q2", "answers": ["B"], "options": ["B. b"]}),
        ];
        let before = questions[1]["no"].clone();
        let report = apply_section(&t, 1, &mut questions);
        assert_eq!(report["applied"], true);
        assert_eq!(questions[0]["score"], 1.5);
        assert_eq!(questions[1]["score"], 1.5);
        assert_eq!(questions[1]["no"], before, "已有题号不得被覆盖");
        assert!(questions[0]["no"].is_i64(), "缺失题号应被补齐");
        assert_eq!(questions[0]["question"], "Q1");
        assert_eq!(questions[1]["answers"][0], "B");
        assert_eq!(questions[0]["template"]["evaluation_type"], "choice");
    }

    #[test]
    fn apply_section_out_of_range_is_reported_not_panicking() {
        let t = parse(&guangdong_res());
        let mut questions = vec![json!({"question": "Q"})];
        let report = apply_section(&t, 99, &mut questions);
        assert_eq!(report["applied"], false);
    }
}
