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
    /// 官方挂载键：/m/homework/struct 的 contents[].code_id（`st1`…），
    /// 与模板 exam_list[].exam_id 的 `{{st1}}` 一一对应。有它就用它，最可靠。
    code_id: String,
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
                code_id: get_str(pkg, "code_id"),
                order: get_i64(pkg, "order").unwrap_or(index as i64),
                questions,
            }
        })
        .collect()
}

/// 归一化大题名：去空白/全角空格。**保留尾部序号** ——
/// 真实官方 res.json 会把"听选信息"拆成「听选信息1/2/3」三个大题（各 3 分、2 题），
/// 资源包的 group_name 也带同样的序号，所以序号是**官方的分节依据**，不能剥掉。
pub fn normalize_section_name(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c: char| c == '\u{3000}' || c.is_whitespace())
        .to_string()
}

/// 匹配用的名字键：两端去空白 + **把所有空白（含 `\r\n\t` 与全角空格）折叠成单个空格**。
///
/// 真实数据里 group_name 会带换行，例如 `必修一U4A\r\n模仿朗读`；
/// 只 trim 两端会与模板的 `exam_type_name` 匹配不上。
pub fn name_match_key(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.chars() {
        if ch.is_whitespace() || ch == '\u{3000}' {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// 兜底用的"去序号"形式：仅当精确名两边都对不上时才用
/// （例如资源包叫「听选信息」而模板叫「听选信息1/2/3」）。
fn strip_trailing_index(raw: &str) -> String {
    let s: String = raw
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_digit() || ('０'..='９').contains(&c))
        .trim_end_matches(['_', '-', ' ', '（', '('])
        .trim()
        .to_string();
    if s.is_empty() {
        raw.trim().to_string()
    } else {
        s
    }
}

fn package_key(pkg: &Package) -> String {
    let normalized = name_match_key(&pkg.group_name);
    if !normalized.is_empty() {
        return normalized;
    }
    if !pkg.structure_type.is_empty() {
        return pkg.structure_type.clone();
    }
    format!("#{}", pkg.index)
}

/// 模板里的 `exam_id` 是占位符（真实值形如 `{{st1}}`），与 contents 的 `code_id`（`st1`）对应。
fn normalize_exam_id(raw: &str) -> String {
    raw.trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim()
        .to_string()
}

/// 把某个包挂到模板的哪一节。匹配优先级（真实 res.json + /m/homework/struct 标定）：
///   0. `code_id` == 模板 `exam_list[].exam_id`（官方挂载键，最可靠）
///   1. 精确大题名 == `exam_type_name`
///   2. `structure_type` == `exam_type_collect`
///   3. 去序号后的名字相等（应对两边一处带序号、一处不带）
///   4. handler 家族相同
/// 匹配原因：官方挂载键 > 精确大题名 > 尾部序号剔除后的大题名 > 同结构家族。
/// **原因必须带出来**：靠"同家族"兜底挂上去的题虽然不丢，但可能落到相邻大题里，
/// 调用方需要知道这次挂载有多可靠。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountReason {
    CodeId,
    ExactName,
    StrippedName,
    CollectFamily,
}

impl MountReason {
    pub fn as_str(self) -> &'static str {
        match self {
            MountReason::CodeId => "code_id",
            MountReason::ExactName => "exact_name",
            MountReason::StrippedName => "stripped_name",
            MountReason::CollectFamily => "collect_family",
        }
    }

    /// 是否属于"确定性挂载"（官方键或名字命中）；家族兜底不算。
    pub fn is_exact(self) -> bool {
        !matches!(self, MountReason::CollectFamily)
    }
}

fn section_index_for_package_with_reason(
    template: &Value,
    pkg: &Package,
) -> Option<(usize, MountReason)> {
    section_index_for_package_inner(template, pkg)
}

fn section_index_for_package(template: &Value, pkg: &Package) -> Option<usize> {
    section_index_for_package_inner(template, pkg).map(|(idx, _)| idx)
}

fn section_index_for_package_inner(
    template: &Value,
    pkg: &Package,
) -> Option<(usize, MountReason)> {
    let sections = template.get("sections")?.as_array()?;

    if !pkg.code_id.is_empty() {
        let want = pkg.code_id.trim();
        if let Some(idx) = sections.iter().position(|s| {
            s.get("questions")
                .and_then(|v| v.as_array())
                .map(|qs| {
                    qs.iter().any(|q| {
                        q.get("exam_id")
                            .and_then(|v| v.as_str())
                            .map(|raw| normalize_exam_id(raw) == want)
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        }) {
            return Some((idx, MountReason::CodeId));
        }
    }

    let pkg_name = name_match_key(&pkg.group_name);
    if !pkg_name.is_empty() {
        if let Some(idx) = sections.iter().position(|s| {
            s.get("exam_type_name")
                .and_then(|v| v.as_str())
                .map(|n| name_match_key(n) == pkg_name)
                .unwrap_or(false)
        }) {
            return Some((idx, MountReason::ExactName));
        }
    }

    if !pkg.structure_type.is_empty() {
        if let Some(idx) = sections.iter().position(|s| {
            s.get("exam_type_collect")
                .and_then(|v| v.as_str())
                .map(|c| c.eq_ignore_ascii_case(&pkg.structure_type))
                .unwrap_or(false)
        }) {
            // exam_type_collect 本身也是官方字段，但同一家族可能对应多个大题，
            // 因此只算"确定"里最弱的一档（仍不是家族兜底）。
            return Some((idx, MountReason::ExactName));
        }
    }

    if !pkg_name.is_empty() {
        let bare = strip_trailing_index(&pkg_name);
        if let Some(idx) = sections.iter().position(|s| {
            s.get("exam_type_name")
                .and_then(|v| v.as_str())
                .map(|n| strip_trailing_index(&name_match_key(n)) == bare)
                .unwrap_or(false)
        }) {
            return Some((idx, MountReason::StrippedName));
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
        })
        .map(|idx| (idx, MountReason::CollectFamily));
    }
    None
}

/// 合并多包为一张卷。
///
/// 输入见 `docs/INTERFACE.md` 的 `parse_homework_set`；返回 `{paper, coverage, warnings}`。
pub fn merge(packages_value: &Value, res: Option<&Value>, homework_value: Option<&Value>) -> Value {
    let packages = read_packages(packages_value);
    let mut warnings: Vec<String> = Vec::new();

    // 作业自己声明的资源包数量：与传进来的 packages 数对账。
    // 这一步能抓到**下载/提取阶段就丢掉的包**（那些包根本到不了解析器），
    // 是"漏题"最早的一道防线 —— 少了包但没人说，是最难发现的一种漏题。
    let declared_packages = homework_value
        .and_then(|h| h.get("contents"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let packages_missing = declared_packages.saturating_sub(packages.len());
    if packages_missing > 0 {
        warnings.push(format!(
            "packages_missing_vs_homework: 作业声明 {} 个资源包，实际只解析了 {} 个（少 {} 个）—— 检查下载/提取是否失败",
            declared_packages,
            packages.len(),
            packages_missing
        ));
    }

    if packages.is_empty() {
        return json!({
            "paper": {"sections": [], "question_count": 0},
            "completeness": {
                "template_questions": 0,
                "parsed_questions": 0,
                "missing_vs_template": 0,
                "extra_vs_template": 0,
                "sections_below_template": [],
                "sections_weak_mount": [],
                "sections_not_covered": [],
                "is_complete": true,
            },
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
        /// (资源包 index, 挂载原因)：靠家族兜底挂上来的必须可见
        mounts: Vec<(usize, MountReason)>,
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
                let mounts: Vec<(usize, MountReason)> = packages
                    .iter()
                    .filter_map(|p| {
                        section_index_for_package_with_reason(tpl, p)
                            .filter(|(idx, _)| *idx == tpl_index)
                            .map(|(_, reason)| (p.index, reason))
                    })
                    .collect();
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
                    mounts,
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
                    packages: leftovers.clone(),
                    mounts: leftovers.iter().map(|p| (p.index, MountReason::CollectFamily)).collect(),
                });
            }
            sorted.clear();
        }
        None => {
            sorted.sort_by_key(|p| (p.order, p.index));
            for pkg in sorted.iter() {
                // 无模板时按"精确大题名"分组（保留序号）：真实官方 res.json 会把
                // 听选信息拆成「听选信息1/2/3」三个大题，资源包也一一对应，因此
                // 序号必须保留；而不同大题即使同为 collector.role 也绝不能并成一节。
                let key = package_key(pkg);
                if let Some(existing) = sections.iter_mut().find(|s| s.key == key) {
                    existing.packages.push(pkg.clone());
                    existing.mounts.push((pkg.index, MountReason::ExactName));
                } else {
                    sections.push(Section {
                        key: key.clone(),
                        caption: if pkg.group_name.is_empty() {
                            key.clone()
                        } else {
                            normalize_section_name(&pkg.group_name)
                        },
                        structure_type: pkg.structure_type.clone(),
                        template_index: None,
                        template_section: None,
                        packages: vec![pkg.clone()],
                        mounts: vec![(pkg.index, MountReason::ExactName)],
                    });
                }
            }
        }
    }

    // 逐节展开题目、定题号、定满分
    let mut global_no: i64 = 0;
    // 完整性对账计数（漏题可观测性的核心）
    let mut missing_vs_template: usize = 0;
    let mut extra_vs_template: usize = 0;
    let mut template_question_total: usize = 0;
    let mut below_template: Vec<String> = Vec::new();
    let mut weak_mounts: Vec<String> = Vec::new();
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

        // ── 完整性对账：合并后的题数 vs 官方模板要求的题数 ────────────────────
        // 漏题（少题）必须能被机器发现：卷面分来自官方 set_score，即使少几题也照样是
        // 30/20 分，光看总分看不出问题，所以这里逐大题对账并给出明确状态。
        let template_question_count = section
            .template_section
            .as_ref()
            .and_then(|s| s.get("questions"))
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let count_status = if section.template_section.is_none() {
            "no_template"
        } else if template_question_count == questions.len() {
            "match"
        } else if questions.len() < template_question_count {
            "below_template"
        } else {
            "above_template"
        };
        if count_status == "below_template" {
            missing_vs_template += template_question_count - questions.len();
            below_template.push(format!(
                "{}: {} 题（官方模板 {} 题）",
                section.caption,
                questions.len(),
                template_question_count
            ));
        } else if count_status == "above_template" {
            extra_vs_template += questions.len() - template_question_count;
        }
        template_question_total += template_question_count;

        let mount_entries: Vec<Value> = section
            .mounts
            .iter()
            .map(|(idx, reason)| json!({"package_index": idx, "by": reason.as_str()}))
            .collect();
        let family_fallback = section
            .mounts
            .iter()
            .any(|(_, reason)| !reason.is_exact());
        if family_fallback {
            weak_mounts.push(section.caption.clone());
        }

        out_sections.push(json!({
            "index": section_pos,
            "caption": section.caption,
            "structure_type": section.structure_type,
            "template_index": section.template_index,
            "package_count": section.packages.len(),
            // 每个资源包是怎么挂到这一节的：code_id / exact_name / stripped_name /
            // collect_family。collect_family 属兜底（题不丢，但可能落到相邻大题）。
            "mounts": mount_entries,
            "mount_confidence": if section.template_section.is_none() {
                "unmatched"
            } else if family_fallback { "family_fallback" } else { "exact" },
            // 题数对账：match / below_template / above_template / no_template
            "question_count": questions.len(),
            "template_question_count": template_question_count,
            "count_status": count_status,
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
    let tpl_section_total = parsed_template
        .as_ref()
        .and_then(|t| t.get("sections"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let covered_from_template = sections
        .iter()
        .filter(|s| s.template_index.is_some())
        .count();
    // 只有把模板里的每个大题都挂上包，官方的 set_score 才是这张卷的卷面分；
    // 只合并了一部分包时，用已合并部分的分值之和才诚实。
    let full_template_coverage = tpl_section_total > 0 && covered_from_template == tpl_section_total;

    let (total_score, total_score_source) = match parsed_template.as_ref() {
        Some(tpl) if full_template_coverage => {
            let set_score = tpl.get("set_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if set_score > 0.0 {
                (set_score, "res_template_set_score")
            } else if total_from_template > 0.0 {
                (round2(total_from_template), "res_template")
            } else {
                (
                    tpl.get("total_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    "res_template_exam_score_sum",
                )
            }
        }
        Some(_) if total_from_template > 0.0 => {
            warnings.push(format!(
                "partial_template_coverage:{covered_from_template}/{tpl_section_total}"
            ));
            (round2(total_from_template), "res_template_partial")
        }
        _ => match classification.as_ref() {
            Some(c) => (
                c.get("expected_total_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                "classification_estimate",
            ),
            None => (0.0, "none"),
        },
    };

    // 模板里"完全没挂上包"的大题：题不是漏了，而是整节缺失（同样属于不完整）
    let mut sections_not_covered: Vec<String> = Vec::new();
    if let Some(tpl) = parsed_template.as_ref() {
        if let Some(tpl_sections) = tpl.get("sections").and_then(|v| v.as_array()) {
            for (idx, s) in tpl_sections.iter().enumerate() {
                let covered = sections
                    .iter()
                    .any(|sec| sec.template_index == Some(idx) && !sec.packages.is_empty());
                if !covered {
                    sections_not_covered.push(
                        s.get("exam_type_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(未命名大题)")
                            .to_string(),
                    );
                }
            }
        }
    }
    if !sections_not_covered.is_empty() {
        warnings.push(format!(
            "template_sections_not_covered: 官方模板有 {} 个大题没挂上任何资源包 —— {}",
            sections_not_covered.len(),
            sections_not_covered.join("；")
        ));
    }

    // 漏题/多题/弱挂载都必须进 warnings，让调用方能直接判"这张卷可不可信"
    if missing_vs_template > 0 {
        warnings.push(format!(
            "question_count_below_template: 合并后 {} 题，官方模板要求 {} 题，少 {} 题 —— {}",
            question_count,
            template_question_total,
            missing_vs_template,
            below_template.join("；")
        ));
    }
    if extra_vs_template > 0 {
        warnings.push(format!(
            "question_count_above_template: 合并后 {} 题，官方模板要求 {} 题，多 {} 题",
            question_count, template_question_total, extra_vs_template
        ));
    }
    if !weak_mounts.is_empty() {
        warnings.push(format!(
            "package_mounted_by_family_fallback: 这些大题靠同结构家族兜底挂载（题目未丢，但落点可能与官方模板不同）—— {}",
            weak_mounts.join("；")
        ));
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
        "completeness": {
            // 官方模板要求的总题数 / 实际合并出的总题数
            "template_questions": template_question_total,
            "parsed_questions": question_count,
            "missing_vs_template": missing_vs_template,
            "extra_vs_template": extra_vs_template,
            // 少题的大题（空数组 = 与官方模板逐节对齐）
            "sections_below_template": below_template,
            // 靠家族兜底挂载的大题（题不丢，但落点需人工确认）
            "sections_weak_mount": weak_mounts,
            "sections_not_covered": sections_not_covered,
            "is_complete": missing_vs_template == 0
                && extra_vs_template == 0
                && packages_missing == 0
                && sections_not_covered.is_empty(),
        },
        "coverage": {
            "packages": packages.len(),
            "declared_packages": declared_packages,
            "missing_packages": packages_missing,
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
    fn group_name_with_crlf_still_matches_template() {
        // 真实数据：广东高中作业的 group_name 形如 "必修一U4A\r\n模仿朗读"，
        // 模板侧 exam_type_name 则是 "必修一U4A 模仿朗读"（空白形态不同）
        assert_eq!(super::name_match_key("必修一U4A\r\n模仿朗读"), "必修一U4A 模仿朗读");
        assert_eq!(super::name_match_key("  模仿朗读\u{3000}"), "模仿朗读");

        let res = json!({
            "set_score": "20",
            "exam_type_list": [{
                "exam_type_order": "1", "exam_type_name": "必修一U4A 模仿朗读",
                "exam_type_collect": "collector.read", "exam_type_score": "20",
                "exam_list": [{"exam_id": "{{st1}}", "exam_score": "20",
                               "question_list": [{"question_order": "1", "question_score": "20"}]}]
            }]
        });
        let packages = json!([
            {"structure_type": "collector.read", "group_name": "必修一U4A\r\n模仿朗读",
             "order": 0, "questions": [{"no": 1}]}
        ]);
        let out = merge(&packages, Some(&res), None);
        assert_eq!(out["coverage"]["unmatched_packages"].as_array().unwrap().len(), 0,
                   "含换行的 group_name 必须能挂上模板");
        assert_eq!(out["paper"]["sections"][0]["questions"][0]["score"], 20.0);
    }

    #[test]
    fn code_id_is_the_official_mount_key() {
        // 官方挂载键：contents[].code_id = "st1" <-> 模板 exam_list[].exam_id = "{{st1}}"
        // 即使大题名两边不一致，也应按 code_id 挂载。
        let res = json!({
            "set_score": "30",
            "exam_type_list": [
                {"exam_type_order": "1", "exam_type_name": "模仿朗读",
                 "exam_type_collect": "collector.read", "exam_type_score": "6",
                 "exam_list": [{"exam_id": "{{st1}}", "exam_score": "6",
                                "question_list": [{"question_order": "1", "question_score": "6"}]}]},
                {"exam_type_order": "2", "exam_type_name": "听选信息1",
                 "exam_type_collect": "collector.role", "exam_type_score": "3",
                 "exam_list": [{"exam_id": "{{st2}}", "exam_score": "3",
                                "question_list": [{"question_order": "1", "question_score": "1.5"},
                                                  {"question_order": "2", "question_score": "1.5"}]}]}
            ]
        });
        // 包名故意与模板名不同，且传入顺序颠倒
        let packages = json!([
            {"structure_type": "collector.role", "group_name": "题目组B", "code_id": "st2", "order": 9,
             "questions": [{"no": 1}, {"no": 2}]},
            {"structure_type": "collector.read", "group_name": "题目组A", "code_id": "st1", "order": 8,
             "questions": [{"no": 1}]}
        ]);
        let out = merge(&packages, Some(&res), None);
        let sections = out["paper"]["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0]["caption"], "模仿朗读");
        assert_eq!(sections[1]["caption"], "听选信息1");
        assert_eq!(sections[0]["questions"][0]["score"], 6.0);
        assert_eq!(sections[1]["questions"][0]["score"], 1.5);
        assert_eq!(out["paper"]["totalScore"], 30.0);
        assert_eq!(out["paper"]["totalScore_source"], "res_template_set_score");
        assert_eq!(out["coverage"]["unmatched_packages"].as_array().unwrap().len(), 0);
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
