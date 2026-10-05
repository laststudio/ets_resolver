// -*- coding: utf-8 -*-
// 作业分类 (homework classification): 把"这个作业是什么"的判断收口到解析器。
//
// 背景：同一套判断原先散在三处，且三处口径不一致 ——
//   * 后端 lastudio-platform: ets100.py 的 is_pk_or_vocab / is_mock_suite + 卷面满分推算
//   * Fe-pro Android:        ETS100Api.kt / ReadCloudDerivedState 各自判 set_type==6 / pk==1
//   * Fe_web:                App.tsx / autoComplete.ts 再判一遍 kind==='pk'|'vocab'|'composition'
// 于是"老师布置的习题集"这种新形态要在三个仓库各改一次。这里给出唯一实现。
//
// 本模块是纯函数：吃一份作业 JSON（官方 /g/homework/list 的 item，字段可平铺或嵌在
// `struct` / `raw` 里），吐出分类结论 + 命中的规则 + 卷面满分推算。
// 刻意让结论带 `reasons`：误判时必须能看出是哪条规则命中的，否则无法排错。

use serde_json::{json, Value};

/// 词汇 / 单词 PK 类：走词汇通道（word/struct、pk-info、score-detail），不是听说卷。
const VOCAB_NAME_KEYS: [&str; 5] = ["词汇", "单词", "默写", "pk", "word"];

/// 模拟卷 / 机考套卷关键词（后端 ets100.py 的既有口径，保持一致）。
const MOCK_NAME_KEYS: [&str; 11] = [
    "套题",
    "模考",
    "模拟考",
    "机房模考",
    "仿真",
    "中考英语听说",
    "听说模拟",
    "听说测试",
    "期末听说",
    "期中听说",
    "模拟考试",
];

/// 多卷套题的次级关键词（需要 contents >= 3 才成立）。
const MULTI_PAPER_KEYS: [&str; 3] = ["机考", "综合", "套题"];

/// 作文类（Fe_web 的 kind === 'composition'）。
const COMPOSITION_NAME_KEYS: [&str; 2] = ["作文", "composition"];

/// 依次在 item 本体、`struct`、`raw`、`data` 里找字段（官方不同接口把字段放在不同层）。
fn lookup(item: &Value, key: &str) -> Option<Value> {
    if let Some(v) = item.get(key) {
        if !v.is_null() {
            return Some(v.clone());
        }
    }
    for holder in ["struct", "raw", "data"] {
        if let Some(v) = item.get(holder).and_then(|h| h.get(key)) {
            if !v.is_null() {
                return Some(v.clone());
            }
        }
    }
    None
}

fn lookup_str(item: &Value, key: &str) -> String {
    lookup(item, key)
        .map(|v| match v {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn lookup_i64(item: &Value, key: &str) -> Option<i64> {
    lookup(item, key).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    })
}

fn lookup_f64(item: &Value, key: &str) -> Option<f64> {
    lookup(item, key).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    })
}

/// 资源包（contents）列表：优先顶层 `contents`，其次 `struct.contents`。
fn contents(item: &Value) -> Vec<Value> {
    let direct = item.get("contents").and_then(|v| v.as_array());
    let nested = item
        .get("struct")
        .and_then(|s| s.get("contents"))
        .and_then(|v| v.as_array());
    direct
        .or(nested)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn contains_any(haystack: &str, needles: &[&str]) -> Option<String> {
    let lower = haystack.to_lowercase();
    needles
        .iter()
        .find(|k| lower.contains(&k.to_lowercase()))
        .map(|k| k.to_string())
}

/// 分类结论的稳定标识。
pub const CATEGORY_VOCAB_PK: &str = "vocab_pk"; // 词汇 / 单词 PK / 默写
pub const CATEGORY_MOCK_SUITE: &str = "mock_suite"; // 模拟考 / 机考套卷（听说，官方显式标记）
pub const CATEGORY_PAPER_SET: &str = "paper_set"; // 老师布置的一整套多卷（习题集）
pub const CATEGORY_COMPOSITION: &str = "composition"; // 作文
pub const CATEGORY_ONLINE: &str = "online"; // 无资源包的在线作业（走在线接口取题）
pub const CATEGORY_ORAL: &str = "oral"; // 常规听说作业（单卷）

/// 判断一份作业属于哪一类，并推算卷面满分。
///
/// 返回：
/// ```json
/// {
///   "category": "paper_set",
///   "is_exercise_set": true,        // 老师布置的一整套多卷（习题集）
///   "is_vocab_pk": false,
///   "is_mock_suite": false,
///   "is_composition": false,
///   "package_count": 6,             // contents 里的资源包数量
///   "expected_total_score": 30.0,   // 卷面总分（口径来源见 score_source）
///   "score_source": "mock_suite_default",
///   "reasons": ["name_keyword:套题", "packages:6"],
///   "warnings": []
/// }
/// ```
pub fn classify_homework(item: &Value) -> Value {
    let name = lookup_str(item, "name");
    let kind = lookup_str(item, "kind");
    let set_type = lookup_i64(item, "set_type").unwrap_or(0);
    let pk = lookup_i64(item, "pk").unwrap_or(0);
    let mock = lookup_i64(item, "mock").unwrap_or(0);
    let exam = lookup_i64(item, "exam").unwrap_or(0);
    let set_id = lookup_str(item, "set_id");
    let resource_id = lookup_str(item, "resource_id");
    let packages = contents(item);
    let package_count = packages.len() as i64;

    let group_names: Vec<String> = packages
        .iter()
        .map(|c| {
            c.get("group_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();

    let mut reasons: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // ---- 1. 词汇 / PK：优先级最高，因为它的资源获取通道与听说卷完全不同 ----
    let mut is_vocab_pk = false;
    if set_type == 6 {
        is_vocab_pk = true;
        reasons.push("set_type:6".into());
    }
    if pk == 1 {
        is_vocab_pk = true;
        reasons.push("pk:1".into());
    }
    if matches!(kind.as_str(), "pk" | "vocab") {
        is_vocab_pk = true;
        reasons.push(format!("kind:{kind}"));
    }
    if let Some(hit) = contains_any(&name, &VOCAB_NAME_KEYS) {
        is_vocab_pk = true;
        reasons.push(format!("name_keyword:{hit}"));
    }

    // ---- 2. 作文 ----
    let mut is_composition = matches!(kind.as_str(), "composition");
    if is_composition {
        reasons.push(format!("kind:{kind}"));
    } else if let Some(hit) = contains_any(&name, &COMPOSITION_NAME_KEYS) {
        is_composition = true;
        reasons.push(format!("name_keyword:{hit}"));
    }

    // ---- 3. 模拟考 / 机考套卷 ----
    // 只信官方显式字段（kind/mock/exam/set_type）。名字关键词与"包数>=3"这类启发式
    // 历史上把普通作业误判成模拟考（后端 418cf99 的真实故障），所以降级为
    // `mock_suspected_by_name` 告警 + `mock_suspected` 字段，由调用方决定是否采信。
    let mut is_mock_suite = false;
    let mut mock_suspected = false;
    if !is_vocab_pk && !is_composition {
        if matches!(kind.as_str(), "mock") {
            is_mock_suite = true;
            reasons.push("kind:mock".into());
        }
        if mock == 1 {
            is_mock_suite = true;
            reasons.push("mock:1".into());
        }
        if exam == 1 {
            is_mock_suite = true;
            reasons.push("exam:1".into());
        }
        if set_type == 1 {
            is_mock_suite = true;
            reasons.push("set_type:1".into());
        }
        if !is_mock_suite {
            if let Some(hit) = contains_any(&name, &MOCK_NAME_KEYS) {
                mock_suspected = true;
                reasons.push(format!("mock_suspected_name_keyword:{hit}"));
                warnings.push("mock_suspected_by_name".into());
            } else if package_count >= 3 {
                if let Some(hit) = contains_any(&name, &MULTI_PAPER_KEYS) {
                    mock_suspected = true;
                    reasons.push(format!("mock_suspected_multi_paper_keyword:{hit}"));
                    warnings.push("mock_suspected_by_name".into());
                }
            }
        }
    }

    // ---- 4. 在线作业（无资源包，走在线接口取题；词汇/PK 也属于这一类通道）----
    let is_online = !is_vocab_pk
        && !is_composition
        && !is_mock_suite
        && package_count == 0;
    if is_online {
        reasons.push("no_packages".into());
    }

    // ---- 5. 老师布置的一整套多卷（习题集）----
    // 判据：非词汇/非作文，且需要把多个资源包合并成一张卷。
    // 模拟套卷同样为 true —— 下游要的是"要不要合并多包"，不是"是不是考试"。
    let is_exercise_set = !is_vocab_pk
        && !is_composition
        && !is_online
        && (package_count >= 2 || (!set_id.is_empty() && package_count >= 1));
    if is_exercise_set {
        reasons.push(format!("packages:{package_count}"));
        if !set_id.is_empty() {
            reasons.push(format!("set_id:{set_id}"));
        }
    }

    // ---- 6. 归类 ----
    let category = if is_vocab_pk {
        CATEGORY_VOCAB_PK
    } else if is_composition {
        CATEGORY_COMPOSITION
    } else if is_mock_suite {
        CATEGORY_MOCK_SUITE
    } else if is_online {
        CATEGORY_ONLINE
    } else if is_exercise_set {
        CATEGORY_PAPER_SET
    } else {
        CATEGORY_ORAL
    };

    // ---- 7. 卷面满分推算（口径与后端 ets100.py 一致，统一收到这里）----
    let raw_total = lookup_f64(item, "total_point")
        .or_else(|| lookup_f64(item, "total_score"))
        .unwrap_or(0.0);
    let is_beijing = name.contains("北京");
    let (expected_total_score, score_source) = if is_vocab_pk || is_online {
        (
            if raw_total > 0.0 { raw_total } else { 100.0 },
            "online_default_100",
        )
    } else if is_mock_suite {
        if raw_total <= 0.0 || (raw_total - 100.0).abs() < 0.01 {
            (if is_beijing { 40.0 } else { 30.0 }, "mock_suite_default")
        } else {
            (raw_total, "official_total")
        }
    } else if raw_total > 0.0
        && (raw_total - 100.0).abs() >= 0.01
        && (raw_total - 30.0).abs() >= 0.01
    {
        (raw_total, "official_total")
    } else if package_count > 0 {
        let sum = group_name_score_sum(&group_names, package_count as usize, is_beijing);
        (sum, "group_name_weights")
    } else if contains_any(&name, &["朗读"]).is_some() {
        (if is_beijing { 6.0 } else { 5.0 }, "name_read_aloud")
    } else if contains_any(&name, &["转述", "看图"]).is_some() {
        (if is_beijing { 10.0 } else { 6.0 }, "name_retell")
    } else if raw_total > 0.0 && (raw_total - 100.0).abs() >= 0.01 {
        (raw_total, "official_total")
    } else {
        (10.0, "fallback_10")
    };

    // ---- 8. 一致性告警 ----
    if is_vocab_pk && package_count > 0 {
        warnings.push("vocab_with_packages".into());
    }
    if is_exercise_set && package_count <= 1 {
        warnings.push("exercise_set_with_single_package".into());
    }
    if resource_id.is_empty() && is_vocab_pk {
        warnings.push("vocab_without_resource_id".into());
    }
    warnings.dedup();

    json!({
        "category": category,
        "is_exercise_set": is_exercise_set,
        "is_vocab_pk": is_vocab_pk,
        "is_mock_suite": is_mock_suite,
        "is_mock_suspected": mock_suspected,
        "is_composition": is_composition,
        "is_online": is_online,
        "package_count": package_count,
        "expected_total_score": expected_total_score,
        "score_source": score_source,
        "set_id": set_id,
        "resource_id": resource_id,
        "reasons": reasons,
        "warnings": warnings,
    })
}

/// 按 group_name 语义权重累加卷面分（与后端既有口径逐字一致）：
/// 朗读类按卷内大组数递减（1 卷 5/6 分、2 卷 2.5 分、更多 2 分）。
fn group_name_score_sum(group_names: &[String], package_count: usize, is_beijing: bool) -> f64 {
    let mut sum = 0.0f64;
    for name in group_names {
        if name.contains("朗读") {
            sum += if package_count <= 1 {
                if is_beijing {
                    6.0
                } else {
                    5.0
                }
            } else if package_count == 2 {
                2.5
            } else {
                2.0
            };
        } else if name.contains("转述") || name.contains("看图") || name.contains("表达") {
            sum += if is_beijing { 10.0 } else { 6.0 };
        } else if name.contains("选择") || name.contains("听选") {
            sum += 1.5;
        } else if name.contains("填空") || name.contains("记录") {
            sum += 1.5;
        } else if name.contains("回答") || name.contains("3问5答") {
            sum += 2.0;
        } else {
            sum += 2.0;
        }
    }
    (sum * 100.0).round() / 100.0
}

/// 批量分类：吃 /g/homework/list 的 `{homeworks:[...]}`（也接受裸数组），
/// 顺带给出"哪些是习题集"的清单，供列表页直接打标。
pub fn classify_homework_list(list: &Value) -> Value {
    let items: Vec<Value> = list
        .get("homeworks")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| list.as_array().cloned())
        .unwrap_or_default();

    let mut out = Vec::with_capacity(items.len());
    let mut exercise_sets: Vec<String> = Vec::new();
    let mut category_counts: std::collections::BTreeMap<String, usize> = Default::default();

    for item in &items {
        let classification = classify_homework(item);
        let category = classification["category"].as_str().unwrap_or("").to_string();
        *category_counts.entry(category.clone()).or_insert(0) += 1;
        let id = lookup_str(item, "id");
        if classification["is_exercise_set"] == Value::Bool(true) {
            exercise_sets.push(id.clone());
        }
        out.push(json!({
            "id": id,
            "name": lookup_str(item, "name"),
            "classification": classification,
        }));
    }

    json!({
        "count": out.len(),
        "category_counts": category_counts,
        "exercise_set_ids": exercise_sets,
        "homeworks": out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pkg(group: &str) -> Value {
        json!({"group_name": group, "url": format!("{group}.zip")})
    }

    #[test]
    fn classifies_vocab_pk_by_set_type() {
        let hw = json!({"id": "1", "name": "Unit 2 单词默写", "set_type": 6, "pk": 0, "contents": []});
        let c = classify_homework(&hw);
        assert_eq!(c["category"], CATEGORY_VOCAB_PK);
        assert_eq!(c["is_vocab_pk"], true);
        assert_eq!(c["expected_total_score"], 100.0);
        assert_eq!(c["score_source"], "online_default_100");
    }

    #[test]
    fn classifies_vocab_pk_by_name_when_fields_absent() {
        let hw = json!({"id": "2", "name": "第5单元 词汇专练", "contents": []});
        let c = classify_homework(&hw);
        assert_eq!(c["category"], CATEGORY_VOCAB_PK);
        assert!(c["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r.as_str().unwrap().starts_with("name_keyword:")));
    }

    #[test]
    fn classifies_mock_suite_only_from_official_flags() {
        // 官方显式标记（mock=1）才判模拟考，并用 30 / 北京 40 的默认卷面分
        let hw = json!({
            "id": "3", "name": "中考英语听说模拟考", "mock": 1,
            "contents": [pkg("模仿朗读"), pkg("听选信息"), pkg("信息转述")]
        });
        let c = classify_homework(&hw);
        assert_eq!(c["category"], CATEGORY_MOCK_SUITE);
        assert_eq!(c["is_mock_suite"], true);
        assert_eq!(c["expected_total_score"], 30.0);
        assert_eq!(c["score_source"], "mock_suite_default");

        let beijing = json!({
            "id": "4", "name": "北京中考英语听说模拟考试", "set_type": 1,
            "contents": [pkg("听后选择"), pkg("听后回答")]
        });
        assert_eq!(classify_homework(&beijing)["expected_total_score"], 40.0);
    }

    #[test]
    fn mock_name_keyword_alone_is_only_a_suspicion() {
        // 名字像模考但没有官方标记：只给 suspected 告警，不擅自判成模拟考
        // （后端曾因这类启发式把普通作业误判成"模拟考封存"，见 418cf99）
        let hw = json!({
            "id": "3b", "name": "第八周机房套题训练",
            "contents": [pkg("模仿朗读"), pkg("听选信息"), pkg("回答问题")]
        });
        let c = classify_homework(&hw);
        assert_eq!(c["is_mock_suite"], false);
        assert_eq!(c["is_mock_suspected"], true);
        assert_eq!(c["category"], CATEGORY_PAPER_SET);
        assert!(c["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w == "mock_suspected_by_name"));
    }

    #[test]
    fn classifies_teacher_assigned_multi_paper_set() {
        // 老师布置的一整套多卷：多个资源包需要合并成一张卷，且名字里没有模考关键词
        let hw = json!({
            "id": "5", "name": "Unit 3 综合练习",
            "set_id": "set-9981",
            "contents": [pkg("模仿朗读"), pkg("听选信息"), pkg("回答问题"), pkg("信息转述")]
        });
        let c = classify_homework(&hw);
        assert_eq!(c["is_exercise_set"], true);
        assert_eq!(c["package_count"], 4);
        assert!(c["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "packages:4"));
        // 卷面分按大组权重累加：朗读2 + 听选1.5 + 回答2 + 转述6 = 11.5
        assert_eq!(c["score_source"], "group_name_weights");
        assert_eq!(c["expected_total_score"], 11.5);
    }

    #[test]
    fn single_package_oral_homework_is_not_exercise_set() {
        let hw = json!({"id": "6", "name": "短文朗读", "contents": [pkg("模仿朗读")]});
        let c = classify_homework(&hw);
        assert_eq!(c["category"], CATEGORY_ORAL);
        assert_eq!(c["is_exercise_set"], false);
        // 单卷朗读：group_name 权重口径 => 5 分
        assert_eq!(c["expected_total_score"], 5.0);
        assert_eq!(c["score_source"], "group_name_weights");
    }

    #[test]
    fn reads_fields_from_nested_struct_and_raw() {
        let hw = json!({
            "id": "7",
            "name": "作业",
            "raw": {"set_type": 6, "pk": 1, "resource_id": "r-1"},
            "struct": {"contents": []}
        });
        let c = classify_homework(&hw);
        assert_eq!(c["category"], CATEGORY_VOCAB_PK);
        assert_eq!(c["is_vocab_pk"], true);
    }

    #[test]
    fn list_summary_reports_exercise_sets() {
        let list = json!({"homeworks": [
            {"id": "a", "name": "听力模拟考", "mock": 1,
             "contents": [pkg("听后选择"), pkg("听后回答"), pkg("听后转述")]},
            {"id": "b", "name": "单词默写", "set_type": 6, "contents": []},
            {"id": "c", "name": "Unit 4 综合练习", "set_id": "s-1",
             "contents": [pkg("模仿朗读"), pkg("听选信息")]}
        ]});
        let out = classify_homework_list(&list);
        assert_eq!(out["count"], 3);
        assert_eq!(out["category_counts"][CATEGORY_MOCK_SUITE], 1);
        assert_eq!(out["category_counts"][CATEGORY_VOCAB_PK], 1);
        let ids = out["exercise_set_ids"].as_array().unwrap();
        assert!(ids.iter().any(|v| v == "a"));
        assert!(ids.iter().any(|v| v == "c"));
        assert!(!ids.iter().any(|v| v == "b"));
    }

    #[test]
    fn composition_is_not_treated_as_exercise_set() {
        let hw = json!({"id": "8", "name": "My Weekend 作文", "kind": "composition"});
        let c = classify_homework(&hw);
        assert_eq!(c["category"], CATEGORY_COMPOSITION);
        assert_eq!(c["is_exercise_set"], false);
    }
}
