// -*- coding: utf-8 -*-
// 官方题库盘点回归（数据源：laststudio/ets100_resources 的 139,314 套 / 441,931 条小题明细
// + 221 套真实模板包里的 res.json）
//
// 这两个 fixture 是"适配全部题目"的**定义域**与**门禁**：
//   * official_type_inventory.json —— 官方世界里到底有多少种大题名/题型名；
//   * official_exam_type_map.json  —— 官方 exam_type_collect 家族全集，
//     以及"官方大题名 → 解析家族"的实证映射。
// 一旦官方引入新的 collect 家族，这里的断言会失败 —— 那就是需要新增适配的信号。

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", name]
        .iter()
        .collect();
    serde_json::from_str(&fs::read_to_string(&path).expect("fixture 不存在")).expect("fixture JSON 非法")
}

/// spec 里登记的全部解析家族（含 id 与 handler）。
fn spec_families() -> BTreeSet<String> {
    let spec = resolver::spec_value();
    let mut out = BTreeSet::new();
    for item in spec["structure_types"].as_array().unwrap() {
        if let Some(id) = item["id"].as_str() {
            out.insert(id.to_string());
        }
        if let Some(h) = item["handler"].as_str() {
            out.insert(h.to_string());
        }
    }
    out
}

#[test]
fn every_official_collect_family_is_known_to_spec() {
    let map = fixture("official_exam_type_map.json");
    let families = spec_families();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in map["pairs"].as_array().unwrap() {
        let collect = entry["collect"].as_str().unwrap_or("");
        assert!(
            !collect.is_empty(),
            "官方模板出现空 exam_type_collect：{entry}"
        );
        assert!(
            families.contains(collect),
            "官方新增了解析家族 `{collect}`，但 spec 未收录 —— 需要新增适配"
        );
        seen.insert(collect.to_string());
    }
    // 实测这批模板覆盖到 9 个家族；数量变化说明官方题库结构变了，值得看一眼
    assert_eq!(
        seen.len(),
        9,
        "官方 collect 家族数量变化（现在 {seen:?}），请核对 fixture 与 spec"
    );
    assert!(seen.contains("collector.choose"));
    assert!(seen.contains("collector.picture"));
    assert!(seen.contains("collector.repeat_essay"));
}

#[test]
fn evaluation_and_engine_type_domains_match_spec() {
    let map = fixture("official_exam_type_map.json");
    let spec = resolver::spec_value();
    let spec_eval: BTreeSet<String> = spec["evaluation_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let spec_engine: BTreeSet<String> = spec["engine_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    for (key, domain) in [("evaluation_types", &spec_eval), ("engine_types", &spec_engine)] {
        for name in map[key].as_object().unwrap().keys() {
            if name.is_empty() {
                continue; // 官方确实存在空值
            }
            assert!(
                domain.contains(name),
                "{key} 出现未登记取值 `{name}`（spec 需要补充）"
            );
        }
    }
}

#[test]
fn official_name_space_is_huge_which_is_why_we_key_on_collect() {
    let inv = fixture("official_type_inventory.json");
    let distinct = inv["distinct_group_names"].as_u64().unwrap();
    assert!(distinct > 1000, "官方大题名应有上千种，实测 {distinct}");

    // 名字空间巨大且长尾严重：这解释了为什么"按名字适配"不可行，
    // 必须依赖 9 个 collect 家族 + 结构嗅探。
    let once = inv["group_names"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["count"].as_u64() == Some(1))
        .count();
    assert!(once > 500, "长尾（只出现一次的名字）应有数百个，实测 {once}");
}

#[test]
fn same_official_name_maps_to_multiple_families_so_name_is_not_authoritative() {
    let map = fixture("official_exam_type_map.json");
    let pairs = map["pairs"].as_array().unwrap();
    let mut per_name: std::collections::BTreeMap<&str, BTreeSet<&str>> = Default::default();
    for p in pairs {
        per_name
            .entry(p["group_name"].as_str().unwrap())
            .or_default()
            .insert(p["collect"].as_str().unwrap());
    }
    let ambiguous: Vec<&&str> = per_name
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| k)
        .collect();
    assert!(
        !ambiguous.is_empty(),
        "实测存在同一大题名归属不同家族的情况，spec 的 collect_authority 规则依赖这一点"
    );
    assert!(
        ambiguous.iter().any(|n| **n == "回答问题"),
        "「回答问题」应出现在跨家族名单里（dialogue / role）"
    );
    let spec = resolver::spec_value();
    let rule = spec["boundary_rules"]["collect_authority"].as_str().unwrap();
    assert!(rule.contains("不能靠中文大题名推断"));
}
