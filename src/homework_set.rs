// -*- coding: utf-8 -*-
// 习题集合并：老师布置的一整套多卷 (parse_homework_set)
//
// 背景：一个作业常常由多个资源包组成（每包一份 content.json），客户端要把它合并成"一张卷"。
// 这段逻辑原先分散在两处且口径不同：
//   * 后端 lastudio-platform: homework.py::format_parsed_paper
//   * Fe-pro Android:         ReadCloudPaperLoader::normalizeCloudParsedSections
// 现在收口到这里，并优先使用官方 res.json 的权威顺序与满分（见 src/template.rs）。
//
// 合并规则（依据客户端反编译结论）：
//   1. 大题顺序：有 res.json 时以 `exam_type_list` 下标为准；同一 `exam_type_collect`
//      对应的多个资源包**合并成同一大题**（听后选择分包合并）；无模板时退回包的 `order`。
//   2. 逐题满分：模板小题数与合并后题数一致时 1:1 取 `question_score`；
//      不一致时把该大题 `exam_score` 均分并告警（绝不编造一个"看起来对"的数）。
//   3. 全局题号：按大题顺序累计；题目自带 `th`（卷面大题号）时以 `th` 为准并接续计数。
//   4. 分类与总分：作业分类复用 `homework::classify_homework`；
//      卷面总分优先取 res.json（`exam_score` 合计），否则用分类推算值并标明来源。

use serde_json::{json, Map, Value};

use crate::{homework, template};

fn get_str(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn get_i64(value: &Value, key: &str) -> Option<i64> {
    match value.get(key) {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// 一个待合并的资源包。
#[derive(Clone)]
struct Package {
    index: usize,
    structure_type: String,
    group_name: String,
    real_id: String,
    order: i64,
    questions: Vec<Value>,
}

fn read_packages(value: &Value) -> Vec<Package> {
    value
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, pkg)| {
            let questions = pkg
                .get("questions")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Package {
                index,
                structure_type: get_str(pkg, "structure_type"),
                group_name: get_str(pkg, "group_name"),
                real_id: get_str(pkg, "real_id"),
                order: get_i64(pkg, "order").unwrap_or(index as i64),
                questions,
            }
        })
        .collect()
}

/// 把某个包挂到模板的哪一节：先按 `exam_type_collect` 精确匹配，
/// 再按官方题型表的 handler 家族兜底（例如 `read_passage_standard` 与 `collector.read`
/// 同属朗读家族，可挂到同一大题）。
fn section_index_for_package(template: &Value, pkg: &Package) -> Option<usize> {
    let sections = template.get("sections")?.as_array()?;
    if !pkg.structure_type.is_empty() {
        if let Some(idx) = sections.iter().position(|s| {
            s.get("exam_type_collect")
                .and_then(|v| v.as_str())
                .map(|c| c.eq_ignore_ascii_case(&pkg.structure_type))
                .unwrap_or(false)
        }) {
            return Some(idx);
        }
    }
    // 家族兜底：拿两边的 handler
    let pkg_family = crate::spec::find_structure_type(&pkg.structure_type)
        .and_then(|s| s.handler)
        .map(|h| h.to_string());
    if let Some(family) = pkg_family {
        return sections.iter().position(|s| {
            let collect = s
                .get("exam_type_collect")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            crate::spec::find_structure_type(collect)
                .and_then(|spec| spec.handler)
                .map(|h| h == family)
                .unwrap_or(false)
        });
    }
    None
}

/// 合并多包为一张卷。
///
/// 输入见 `docs/INTERFACE.md` 的 `parse_homework_set`；返回 `{paper, coverage, warnings}`。
pub fn merge(packages_value: &Value, res: Option<&Value>, homework_value: Option<&Value>) -> Value {
    let packages = read_packages(packages_value);
    let mut warnings: Vec<String> = Vec::new();

    if packages.is_empty() {
        return json!({
            "paper": {"sections": [], "question_count": 0},
            "coverage": {"packages": 0, "merged": 0, "ordered_by": "none",
                         "unmatched_packages": []},
            "warnings": ["no_packages"],
        });
    }

    let parsed_template = res.map(template::parse);
    let mut ordered_by = "package_order";

    // 组装大题：key = 归一化后的挂载键（同一 key 的包合并）
    struct Section {
        key: String,
        caption: String,
        structure_type: String,
        template_index: Option<usize>,
        template_section: Option<Value>,
        packages: Vec<Package>,
    }

    let mut sections: Vec<Section> = Vec::new();
    let mut unmatched: Vec<Value> = Vec::new();
    let mut sorted: Vec<Package> = packages.clone();

    match parsed_template.as_ref() {
        Some(tpl) => {
            ordered_by = "res_template";
            // 挂不上的包单独记账（不是错误，但要能看见）
            for pkg in packages.iter() {
                if section_index_for_package(tpl, pkg).is_none() {
                    unmatched.push(json!({
                        "index": pkg.index,
                        "structure_type": pkg.structure_type,
                        "group_name": pkg.group_name,
                    }));
                }
            }
            // 先按模板顺序建节，再按包 order 挂题，保证大题顺序权威
            let tpl_sections = tpl
                .get("sections")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for (tpl_index, tpl_section) in tpl_sections.iter().enumerate() {
                let collect = tpl_section
                    .get("exam_type_collect")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut members: Vec<Package> = packages
                    .iter()
                    .filter(|p| section_index_for_package(tpl, p) == Some(tpl_index))
                    .cloned()
                    .collect();
                if members.is_empty() {
                    continue;
                }
                // 同一大题内的多包按官方 order 排（听后选择分包合并的关键）
                members.sort_by_key(|p| (p.order, p.index));
                sections.push(Section {
                    key: collect.clone(),
                    caption: tpl_section
                        .get("exam_type_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&collect)
                        .to_string(),
                    structure_type: collect,
                    template_index: Some(tpl_index),
                    template_section: Some(tpl_section.clone()),
                    packages: members,
                });
            }
            // 模板没覆盖到的包，接在后面（保持可见，不丢）
            let mut leftovers: Vec<Package> = sorted
                .iter()
                .filter(|p| section_index_for_package(tpl, p).is_none())
                .cloned()
                .collect();
            if !leftovers.is_empty() {
                leftovers.sort_by_key(|p| (p.order, p.index));
                warnings.push("packages_not_in_template_appended".into());
                sections.push(Section {
                    key: "__unmatched__".into(),
                    caption: "未在模板中登记的大题".into(),
                    structure_type: String::new(),
                    template_index: None,
                    template_section: None,
                    packages: leftovers,
                });
            }
            sorted.clear();
        }
        None => {
            sorted.sort_by_key(|p| (p.order, p.index));
            for pkg in sorted.iter() {
                let key = if pkg.structure_type.is_empty() {
                    pkg.group_name.clone()
                } else {
                    pkg.structure_type.clone()
                };
                if let Some(existing) = sections.iter_mut().find(|s| s.key == key) {
                    existing.packages.push(pkg.clone());
                } else {
                    sections.push(Section {
                        key: key.clone(),
                        caption: if pkg.group_name.is_empty() {
                            key.clone()
                        } else {
                            pkg.group_name.clone()
                        },
                        structure_type: pkg.structure_type.clone(),
                        template_index: None,
                        template_section: None,
                        packages: vec![pkg.clone()],
                    });
                }
            }
        }
    }

    // 逐节展开题目、定题号、定满分
    let mut global_no: i64 = 0;
    let mut out_sections: Vec<Value> = Vec::new();
    let mut total_from_template = 0.0f64;
    let mut question_count = 0usize;

    for (section_pos, section) in sections.iter().enumerate() {
        let mut questions: Vec<Value> = Vec::new();
        for pkg in section.packages.iter() {
            for q in pkg.questions.iter() {
                let mut obj = q.as_object().cloned().unwrap_or_else(Map::new);
                // 保留包内原题号，卷面题号另算
                if let Some(pkg_no) = obj.get("no").cloned() {
                    if !pkg_no.is_null() {
                        obj.insert("package_no".into(), pkg_no);
                    }
                }
                if !pkg.group_name.is_empty() {
                    obj.insert("group_name".into(), Value::String(pkg.group_name.clone()));
                }
                obj.insert(
                    "package_index".into(),
                    json!(pkg.index),
                );
                if !obj.contains_key("structure_type") && !pkg.structure_type.is_empty() {
                    obj.insert(
                        "structure_type".into(),
                        Value::String(pkg.structure_type.clone()),
                    );
                }
                questions.push(Value::Object(obj));
            }
        }

        // 题号：题目自带 th（卷面大题号）时以 th 为准并接续
        for q in questions.iter_mut() {
            let th = q.get("th").and_then(|v| match v {
                Value::Number(n) => n.as_i64(),
                Value::String(s) => s.trim().parse().ok(),
                _ => None,
            });
            global_no = match th {
                Some(v) if v > 0 => v,
                _ => global_no + 1,
            };
            if let Some(obj) = q.as_object_mut() {
                obj.insert("no".into(), json!(global_no));
            }
        }

        // 满分：优先模板 1:1，其次均分大题分，最后留给分类推算
        let mut score_note = "none";
        if let Some(tpl_section) = section.template_section.as_ref() {
            let tpl_questions = tpl_section
                .get("questions")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let exam_score = tpl_section
                .get("exam_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if tpl_questions.len() == questions.len() && !tpl_questions.is_empty() {
                for (i, q) in questions.iter_mut().enumerate() {
                    let score = tpl_questions[i]
                        .get("score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    if let Some(obj) = q.as_object_mut() {
                        obj.insert("score".into(), json!(score));
                    }
                }
                total_from_template += tpl_section
                    .get("question_score_total")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(exam_score);
                score_note = "template_per_question";
            } else if exam_score > 0.0 && !questions.is_empty() {
                let each = round2(exam_score / questions.len() as f64);
                for q in questions.iter_mut() {
                    if let Some(obj) = q.as_object_mut() {
                        obj.insert("score".into(), json!(each));
                    }
                }
                total_from_template += round2(each * questions.len() as f64);
                score_note = "template_section_even_split";
                warnings.push(format!(
                    "section[{section_pos}] 模板小题数({}) 与合并后题数({}) 不一致，已按大题分均分",
                    tpl_questions.len(),
                    questions.len()
                ));
            } else {
                warnings.push(format!(
                    "section[{section_pos}] 模板未提供可用分值，逐题满分留空"
                ));
            }
        }

        question_count += questions.len();
        out_sections.push(json!({
            "index": section_pos,
            "caption": section.caption,
            "structure_type": section.structure_type,
            "template_index": section.template_index,
            "package_count": section.packages.len(),
            "package_indexes": section.packages.iter().map(|p| p.index).collect::<Vec<_>>(),
            // 官方挂载键：real_id（部分接口叫 entity_id），客户端/后端做分节挂载时用它
            "real_ids": section
                .packages
                .iter()
                .map(|p| p.real_id.clone())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            "score_source": score_note,
            "questions": questions,
        }));
    }

    // 分类与总分
    let classification = homework_value.map(homework::classify_homework);
    let (total_score, total_score_source) = if total_from_template > 0.0 {
        (round2(total_from_template), "res_template")
    } else if let Some(tpl) = parsed_template.as_ref() {
        let t = tpl.get("total_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if t > 0.0 {
            (t, "res_template_exam_score_sum")
        } else if let Some(c) = classification.as_ref() {
            (
                c.get("expected_total_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                "classification_estimate",
            )
        } else {
            (0.0, "none")
        }
    } else if let Some(c) = classification.as_ref() {
        (
            c.get("expected_total_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            "classification_estimate",
        )
    } else {
        (0.0, "none")
    };

    if sorted.is_empty() && sections.len() > 1 && unmatched.is_empty() {
        // 全部包都挂上了模板
    }

    json!({
        "paper": {
            "homework_id": homework_value.map(|h| get_str(h, "id")).unwrap_or_default(),
            "title": homework_value.map(|h| get_str(h, "name")).unwrap_or_default(),
            "regionLabel": "云端",
            "sections": out_sections,
            "section_count": sections.len(),
            "question_count": question_count,
            "totalScore": total_score,
            "totalScore_source": total_score_source,
            "category": classification.as_ref().and_then(|c| c.get("category").cloned()).unwrap_or(Value::Null),
            "is_exercise_set": classification.as_ref().and_then(|c| c.get("is_exercise_set").cloned()).unwrap_or(Value::Null),
            "classification": classification.unwrap_or(Value::Null),
        },
        "coverage": {
            "packages": packages.len(),
            "merged": packages.len() - unmatched.len(),
            "ordered_by": ordered_by,
            "unmatched_packages": unmatched,
        },
        "warnings": warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pkg(st: &str, group: &str, order: i64, questions: Value) -> Value {
        json!({
            "structure_type": st,
            "group_name": group,
            "order": order,
            "questions": questions
        })
    }

    fn q(no: i64, text: &str) -> Value {
        json!({"no": no, "type": "collector.choose", "question": text, "answers": ["A"], "options": ["A. a"]})
    }

    fn res_two_sections() -> Value {
        json!({
            "exam_type_list": [
                {"exam_type_name": "模仿朗读", "exam_type_collect": "collector.read",
                 "exam_list": [{"exam_score": "6.0",
                                "question_list": [{"question_order": "0", "question_score": "6.0"}]}]},
                {"exam_type_name": "听选信息", "exam_type_collect": "listen_choice_standard",
                 "exam_list": [{"exam_score": "3.0",
                                "question_list": [{"question_order": "0", "question_score": "1.5"},
                                                  {"question_order": "1", "question_score": "1.5"}]}]}
            ]
        })
    }

    #[test]
    fn merges_multi_package_into_single_paper_in_template_order() {
        // 故意乱序传入：模板顺序应把朗读放前面
        let packages = json!([
            pkg("listen_choice_standard", "听选信息", 1, json!([q(1, "Q1"), q(2, "Q2")])),
            pkg("collector.read", "模仿朗读", 0, json!([q(1, "Read aloud")]))
        ]);
        let out = merge(&packages, Some(&res_two_sections()), None);
        let sections = out["paper"]["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0]["caption"], "模仿朗读");
        assert_eq!(sections[1]["caption"], "听选信息");
        // 全局题号连续
        let nos: Vec<i64> = sections
            .iter()
            .flat_map(|s| s["questions"].as_array().unwrap().iter())
            .map(|q| q["no"].as_i64().unwrap())
            .collect();
        assert_eq!(nos, vec![1, 2, 3]);
        // 逐题满分来自模板
        assert_eq!(sections[0]["questions"][0]["score"], 6.0);
        assert_eq!(sections[1]["questions"][0]["score"], 1.5);
        assert_eq!(out["paper"]["totalScore"], 9.0);
        assert_eq!(out["paper"]["totalScore_source"], "res_template");
        assert_eq!(out["coverage"]["ordered_by"], "res_template");
        // 包内原题号被保留
        assert_eq!(sections[1]["questions"][0]["package_no"], 1);
    }

    #[test]
    fn merges_same_collect_packages_into_one_section() {
        // 听后选择分包：两个包同属 listen_choice_standard -> 合并为一个大题
        let res = json!({
            "exam_type_list": [{
                "exam_type_name": "听选信息", "exam_type_collect": "listen_choice_standard",
                "exam_list": [{"exam_score": "6.0", "question_list": [
                    {"question_order": "0", "question_score": "1.5"},
                    {"question_order": "1", "question_score": "1.5"},
                    {"question_order": "2", "question_score": "1.5"},
                    {"question_order": "3", "question_score": "1.5"}
                ]}]
            }]
        });
        let packages = json!([
            pkg("listen_choice_standard", "听选信息A", 0, json!([q(1, "Q1"), q(2, "Q2")])),
            pkg("listen_choice_standard", "听选信息B", 1, json!([q(1, "Q3"), q(2, "Q4")]))
        ]);
        let out = merge(&packages, Some(&res), None);
        let sections = out["paper"]["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 1, "分包必须合并成一个大题");
        assert_eq!(sections[0]["package_count"], 2);
        assert_eq!(sections[0]["questions"].as_array().unwrap().len(), 4);
        let scores: Vec<f64> = sections[0]["questions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["score"].as_f64().unwrap())
            .collect();
        assert_eq!(scores, vec![1.5, 1.5, 1.5, 1.5]);
    }

    #[test]
    fn warns_and_splits_evenly_when_template_count_mismatch() {
        let res = json!({
            "exam_type_list": [{
                "exam_type_name": "听选信息", "exam_type_collect": "listen_choice_standard",
                "exam_list": [{"exam_score": "6.0", "question_list": [
                    {"question_order": "0", "question_score": "3.0"}
                ]}]
            }]
        });
        let packages = json!([
            pkg("listen_choice_standard", "听选信息", 0, json!([q(1, "Q1"), q(2, "Q2")]))
        ]);
        let out = merge(&packages, Some(&res), None);
        let qs = out["paper"]["sections"][0]["questions"].as_array().unwrap();
        assert_eq!(qs[0]["score"], 3.0);
        assert_eq!(qs[1]["score"], 3.0);
        assert_eq!(out["paper"]["sections"][0]["score_source"], "template_section_even_split");
        assert!(out["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("不一致")));
    }

    #[test]
    fn falls_back_to_package_order_without_template() {
        let packages = json!([
            pkg("collector.read", "模仿朗读", 0, json!([q(1, "R")])),
            pkg("collector.role", "回答问题", 1, json!([q(1, "A1"), q(2, "A2")]))
        ]);
        let out = merge(&packages, None, None);
        assert_eq!(out["coverage"]["ordered_by"], "package_order");
        let sections = out["paper"]["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0]["caption"], "模仿朗读");
        assert_eq!(out["paper"]["question_count"], 3);
        // 无模板时不编造满分
        assert!(sections[0]["questions"][0].get("score").is_none());
    }

    #[test]
    fn appends_packages_missing_from_template_with_warning() {
        let packages = json!([
            pkg("collector.read", "模仿朗读", 0, json!([q(1, "R")])),
            pkg("collector.dialogue", "回答问题", 1, json!([q(1, "A")]))
        ]);
        let res = json!({
            "exam_type_list": [{
                "exam_type_name": "模仿朗读", "exam_type_collect": "collector.read",
                "exam_list": [{"exam_score": "6.0", "question_list": [
                    {"question_order": "0", "question_score": "6.0"}
                ]}]
            }]
        });
        let out = merge(&packages, Some(&res), None);
        assert_eq!(out["coverage"]["unmatched_packages"].as_array().unwrap().len(), 1);
        assert!(out["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w == "packages_not_in_template_appended"));
        let sections = out["paper"]["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0]["caption"], "模仿朗读");
    }

    #[test]
    fn carries_classification_and_uses_it_for_score_without_template() {
        let hw = json!({
            "id": "5", "name": "Unit 3 综合练习", "set_id": "s-1",
            "contents": [{"group_name": "模仿朗读"}, {"group_name": "听选信息"}]
        });
        let packages = json!([
            pkg("collector.read", "模仿朗读", 0, json!([q(1, "R")])),
            pkg("listen_choice_standard", "听选信息", 1, json!([q(1, "C")]))
        ]);
        let out = merge(&packages, None, Some(&hw));
        assert_eq!(out["paper"]["is_exercise_set"], true);
        assert_eq!(out["paper"]["category"], "paper_set");
        assert_eq!(out["paper"]["totalScore_source"], "classification_estimate");
        assert!(out["paper"]["totalScore"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn empty_packages_is_reported_not_panicking() {
        let out = merge(&json!([]), None, None);
        assert_eq!(out["paper"]["question_count"], 0);
        assert!(out["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w == "no_packages"));
    }
}
