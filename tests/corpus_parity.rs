// -*- coding: utf-8 -*-
// 语料库一致性测试 (parity gate)
//
// 目标：把"和官方客户端一样"从主观判断变成 CI 可验证的指标。
//
// 语料库结构（默认 tests/corpus/，可用 PARITY_CORPUS 指定）：
//   tests/corpus/<地区>/<题型>/<case-id>/
//       content.json      必需（官方原始 content.json，未做任何脱敏）
//       info.json         可选
//       content2.json     可选
//       golden.json       必需：期望输出（见下）
//       meta.json         可选：{"region","type","source","captured_at","homework_name"}
//       source.zip        可选：官方原始加密包（存档用，本测试不解析）
//
// golden.json 两种写法都支持：
//   [ {...题目...} ]                                   ← 裸数组
//   {"captured_from":"official_client","questions":[...]}  ← 带来源标注（推荐）
//
// 运行方式：
//   cargo test --test corpus_parity                     # 比对模式（默认）
//   PARITY_UPDATE=1 cargo test --test corpus_parity     # 录制/更新 golden.json
//   PARITY_REQUIRE_CORPUS=1 cargo test --test corpus_parity   # 语料库为空即失败（CI 门禁）
//   PARITY_REPORT=/tmp/parity.json cargo test --test corpus_parity  # 额外输出机器可读报告
//
// 比对字段：no / type / question / answers / options / important
// 不比对：stid 的取值形态（后端与 Fe-pro 各自有历史口径），但会统计差异条数到报告里。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

const COMPARE_FIELDS: [&str; 6] = ["no", "type", "question", "answers", "options", "important"];

/// 只有这些来源的 golden 才算"证明了与官方一致"。
const OFFICIAL_SOURCES: [&str; 4] = [
    "official_client",
    "official_api",
    "official_res_json",
    "manual_reviewed",
];

/// 与官方无关、但**有独立参照物**的 golden：目前是"后端 Python 兜底实现"的产出。
/// 它锁的是"两个实现是否一致"，不等于"与官方 UI 一致"，所以单独统计。
const REFERENCE_SOURCES: [&str; 1] = ["python_reference"];

fn corpus_root() -> PathBuf {
    std::env::var("PARITY_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus"))
}

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

/// 收集所有 case 目录（含 content.json 的目录即视为一个 case）。
fn collect_cases(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let mut has_content = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().map(|n| n == "content.json").unwrap_or(false) {
                has_content = true;
            }
        }
        if has_content {
            out.push(dir);
        }
    }
    out.sort();
    out
}

fn case_id(root: &Path, dir: &Path) -> String {
    dir.strip_prefix(root)
        .unwrap_or(dir)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

/// 取出 golden 里的题目数组，兼容裸数组与带来源标注两种写法。
fn golden_questions(golden: &Value) -> Option<Vec<Value>> {
    if let Some(arr) = golden.as_array() {
        return Some(arr.clone());
    }
    golden.get("questions")?.as_array().cloned()
}

fn golden_source(golden: &Value) -> String {
    golden
        .get("captured_from")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn norm(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "null".to_string(),
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "?".to_string()),
    }
}

/// 逐题比对，返回人类可读的差异列表。
fn diff_questions(id: &str, expected: &[Value], actual: &[Value], limit: usize) -> Vec<String> {
    let mut diffs = Vec::new();
    if expected.len() != actual.len() {
        diffs.push(format!(
            "[{id}] 题数不一致: golden={} resolver={}",
            expected.len(),
            actual.len()
        ));
    }
    for i in 0..expected.len().max(actual.len()) {
        let (Some(e), Some(a)) = (expected.get(i), actual.get(i)) else {
            diffs.push(format!("[{id}] 第 {} 题只有一侧存在", i + 1));
            continue;
        };
        for field in COMPARE_FIELDS {
            let (ev, av) = (norm(e.get(field)), norm(a.get(field)));
            if ev != av {
                diffs.push(format!(
                    "[{id}] 第 {} 题 {field} 不一致\n      golden : {ev}\n      resolver: {av}",
                    i + 1
                ));
            }
        }
        if diffs.len() >= limit {
            diffs.push(format!("[{id}] 差异过多，已截断"));
            break;
        }
    }
    diffs
}

struct CaseOutcome {
    id: String,
    structure_type: String,
    layer: String,
    family: String,
    shape: String,
    questions: usize,
    golden_source: Option<String>,
    status: &'static str, // verified | updated | missing_golden | failed
}

#[test]
fn corpus_parity() {
    let root = corpus_root();
    let update = std::env::var("PARITY_UPDATE").map(|v| v == "1").unwrap_or(false);
    let require_corpus = std::env::var("PARITY_REQUIRE_CORPUS")
        .map(|v| v == "1")
        .unwrap_or(false);
    let strict_official = std::env::var("PARITY_STRICT_OFFICIAL")
        .map(|v| v == "1")
        .unwrap_or(false);

    let cases = collect_cases(&root);
    if cases.is_empty() {
        assert!(
            !require_corpus,
            "语料库为空：{}（PARITY_REQUIRE_CORPUS=1 时视为门禁失败）",
            root.display()
        );
        println!(
            "⚠️  语料库为空，跳过一致性测试：{}\n    放一个 case 进去即可开始（结构见 tests/corpus/README.md）",
            root.display()
        );
        return;
    }

    let mut outcomes: Vec<CaseOutcome> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let mut shape_index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut family_index: BTreeMap<String, usize> = BTreeMap::new();

    for dir in &cases {
        let id = case_id(&root, dir);
        let content = read_json(&dir.join("content.json")).expect("content.json 无法解析");
        let info = read_json(&dir.join("info.json"));
        let content2 = read_json(&dir.join("content2.json"));

        let report = resolver::diagnose_value(&content, content2.as_ref(), info.as_ref());
        let actual = resolver::parse_task_value(&content, content2.as_ref(), info.as_ref());

        let structure_type = report["structure_type"].as_str().unwrap_or("").to_string();
        let layer = report["layer"].as_str().unwrap_or("").to_string();
        let family = report["family"].as_str().unwrap_or("").to_string();
        let shape = report["shape"]["hash"].as_str().unwrap_or("").to_string();

        shape_index
            .entry(shape.clone())
            .or_default()
            .insert(structure_type.clone());
        *family_index.entry(family.clone()).or_insert(0) += 1;

        let golden_path = dir.join("golden.json");
        let golden = read_json(&golden_path);

        let (status, golden_source) = if update {
            // 录制模式：用当前 resolver 输出落盘。注意这只是"自拍"，
            // 真正的门禁 golden 必须来自官方客户端/官方接口（见 docs/PARITY.md）。
            let captured_from = golden
                .as_ref()
                .map(golden_source)
                .filter(|s| s != "resolver_self_recorded")
                .unwrap_or_else(|| "resolver_self_recorded".to_string());
            let payload = json!({
                "captured_from": captured_from,
                "note": "PARITY_UPDATE=1 录制；作为门禁前请用官方结果替换 questions",
                "structure_type": structure_type,
                "questions": actual,
            });
            fs::write(&golden_path, serde_json::to_string_pretty(&payload).unwrap())
                .expect("写入 golden.json 失败");
            ("updated", Some(captured_from))
        } else {
            match golden.as_ref().and_then(golden_questions) {
                Some(expected) => {
                    let diffs = diff_questions(&id, &expected, &actual, 24);
                    let source = golden.as_ref().map(golden_source).unwrap_or_default();
                    if !diffs.is_empty() {
                        failures.extend(diffs);
                        ("failed", Some(source))
                    } else if OFFICIAL_SOURCES.contains(&source.as_str()) {
                        ("verified", Some(source))
                    } else if REFERENCE_SOURCES.contains(&source.as_str()) {
                        ("reference_diff", Some(source))
                    } else {
                        // 自录的 golden 与 resolver 输出当然一致，不能当一致性证据。
                        ("self_recorded", Some(source))
                    }
                }
                None => {
                    failures.push(format!(
                        "[{id}] 缺少可用的 golden.json —— 无法证明与官方一致\
                         （先 PARITY_UPDATE=1 录制，再用官方结果替换 questions）"
                    ));
                    ("missing_golden", None)
                }
            }
        };

        outcomes.push(CaseOutcome {
            id,
            structure_type,
            layer,
            family,
            shape,
            questions: actual.len(),
            golden_source,
            status,
        });
    }

    // ---- 报告 ----
    println!("\n=== 语料库一致性报告 ({} 个 case) ===", outcomes.len());
    println!(
        "{:<44} {:<26} {:<9} {:<22} {:<6} {}",
        "case", "structure_type", "layer", "family", "题数", "状态"
    );
    for o in &outcomes {
        println!(
            "{:<44} {:<26} {:<9} {:<22} {:<6} {}",
            truncate(&o.id, 43),
            truncate(&o.structure_type, 25),
            o.layer,
            truncate(&o.family, 21),
            o.questions,
            match o.status {
                "verified" => format!("verified({})", o.golden_source.clone().unwrap_or_default()),
                other => other.to_string(),
            }
        );
    }

    let verified = outcomes.iter().filter(|o| o.status == "verified").count();
    let self_recorded = outcomes
        .iter()
        .filter(|o| o.status == "self_recorded")
        .count();
    let reference_diff = outcomes
        .iter()
        .filter(|o| o.status == "reference_diff")
        .count();
    let updated = outcomes.iter().filter(|o| o.status == "updated").count();
    let missing = outcomes.iter().filter(|o| o.status == "missing_golden").count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    println!(
        "\n合计: verified={verified} reference_diff={reference_diff} self_recorded={self_recorded} \
         updated={updated} missing_golden={missing} failed={failed}"
    );
    let official_ratio = if outcomes.is_empty() {
        0.0
    } else {
        verified as f64 * 100.0 / outcomes.len() as f64
    };
    println!(
        "官方已证明的比例: {official_ratio:.1}% （self_recorded 的自拍 golden 不计入）"
    );

    println!("\n-- 路由家族分布（越低说明越多包靠结构嗅探兜住，越需要显式适配）--");
    for (family, count) in &family_index {
        println!("{:<28} {count}", family);
    }

    println!("\n-- 形状指纹聚类（同一指纹=结构相同；出现新指纹就是要跟的新结构）--");
    for (shape, types) in &shape_index {
        let types: Vec<&String> = types.iter().collect();
        println!("{shape}  x{}  {}", types.len(), types.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    if let Ok(report_path) = std::env::var("PARITY_REPORT") {
        let report = json!({
            "corpus": root.to_string_lossy(),
            "total": outcomes.len(),
            "verified": verified,
            "reference_diff": reference_diff,
            "self_recorded": self_recorded,
            "official_ratio_percent": (official_ratio * 10.0).round() / 10.0,
            "updated": updated,
            "missing_golden": missing,
            "failed": failed,
            "families": family_index,
            "shapes": shape_index.iter().map(|(k, v)| json!({
                "hash": k,
                "structure_types": v.iter().collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "cases": outcomes.iter().map(|o| json!({
                "id": o.id,
                "structure_type": o.structure_type,
                "layer": o.layer,
                "family": o.family,
                "shape": o.shape,
                "questions": o.questions,
                "status": o.status,
                "golden_source": o.golden_source,
            })).collect::<Vec<_>>(),
            "generated_at_unix": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
        fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap())
            .expect("写入 PARITY_REPORT 失败");
        println!("\n机器可读报告已写入: {report_path}");
    }

    if !failures.is_empty() {
        panic!(
            "\n语料库一致性失败（{} 条差异）:\n{}\n",
            failures.len(),
            failures.join("\n")
        );
    }
    if strict_official && self_recorded > 0 {
        panic!(
            "\nPARITY_STRICT_OFFICIAL=1：有 {self_recorded} 个 case 的 golden 是自录的（非官方来源），\
             不构成一致性证据。请用官方客户端/官方接口结果替换，或改写成 official_client/official_api。\n"
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// 让 map 的字段顺序稳定，便于 golden 比对（serde_json 开了 preserve_order）。
#[allow(dead_code)]
fn sorted_object(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                sorted.insert(k.clone(), sorted_object(&map[k]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted_object).collect()),
        other => other.clone(),
    }
}
