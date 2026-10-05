// -*- coding: utf-8 -*-
// 真实数据回归（2026-10-05，广东 2026 测试账号）
//
// 这两个 fixture 都来自真实线上作业，专门用来锁住"真实数据里踩出来的坑"：
//   1. classify_homework：`set_type=1` **不能**当模拟卷标记（否则 76 个作业里 64 个误判）；
//      单包作业即使带 set_id 也不是习题集。
//   2. parse_homework_set：无模板时必须按"归一化大题名"分组 —— 广东 30 分卷的
//      `听选信息1/2/3` 要合并成一个大题（6 题），而 `听选信息/回答问题/询问信息`
//      虽然同为 `collector.role` 却必须分开。

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", name]
        .iter()
        .collect();
    serde_json::from_str(&fs::read_to_string(&path).expect("fixture 不存在")).expect("fixture JSON 非法")
}

#[test]
fn real_homework_list_classification_matches_calibrated_expectations() {
    let fixture = fixture("homework_list_guangdong.json");
    let homeworks = fixture["homeworks"].as_array().unwrap();
    assert_eq!(homeworks.len(), 76, "真实作业列表应有 76 项");

    let report = resolver::classify_homework_list_value(&json!({ "homeworks": homeworks }));
    let counts = &report["category_counts"];

    // 校准后的分布（修复 set_type=1 误判之前是 mock_suite=64）
    assert_eq!(counts["mock_suite"], 8, "只有真正的模拟/期中期末测试才算模拟卷: {counts}");
    assert_eq!(counts["vocab_pk"], 10, "词汇训练 + 单词PK: {counts}");
    assert_eq!(counts["composition"], 2, "作文: {counts}");
    assert_eq!(
        counts["oral"].as_u64().unwrap() + counts["paper_set"].as_u64().unwrap(),
        54,
        "其余为常规听说作业与多卷习题集: {counts}"
    );

    let by_name = |name: &str| -> &Value {
        report["homeworks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|h| h["name"].as_str().map(|n| n.trim()) == Some(name))
            .unwrap_or_else(|| panic!("找不到作业 {name}"))
    };

    // 真实模拟卷：7 包，形态与广东 30 分卷一致
    let mock1 = &by_name("模拟试题1")["classification"];
    assert_eq!(mock1["category"], "mock_suite");
    assert_eq!(mock1["is_exercise_set"], true);
    assert_eq!(mock1["package_count"], 7);
    assert_eq!(mock1["expected_total_score"], 30.0, "广东卷默认 30 分");

    // 单包朗读作业：set_type=1 但它不是模拟卷，也不是习题集
    let read = &by_name("Unit 1 Reading_短文 朗读")["classification"];
    assert_eq!(read["is_mock_suite"], false);
    assert_eq!(read["is_exercise_set"], false, "单包作业即使带 set_id 也不是习题集");
    assert_eq!(read["category"], "oral");

    // 作文与词汇
    assert_eq!(by_name("U1作文")["classification"]["category"], "composition");
    let vocab = report["homeworks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|h| h["classification"]["category"] == "vocab_pk")
        .count();
    assert_eq!(vocab, 10);
}

#[test]
fn real_guangdong_paper_merges_into_official_five_sections() {
    // 真实作业 ' 模拟试题1' 的 7 个包（group_name / structure_type / real_id / 题数）
    let packages = json!([
        {"structure_type": "collector.read",    "group_name": "模仿朗读",  "real_id": "49062", "order": 0, "questions": [{"no": 1}]},
        {"structure_type": "collector.role",    "group_name": "听选信息1", "real_id": "49063", "order": 1, "questions": [{"no": 1}, {"no": 2}]},
        {"structure_type": "collector.role",    "group_name": "听选信息2", "real_id": "49067", "order": 2, "questions": [{"no": 1}, {"no": 2}]},
        {"structure_type": "collector.role",    "group_name": "听选信息3", "real_id": "49068", "order": 3, "questions": [{"no": 1}, {"no": 2}]},
        {"structure_type": "collector.role",    "group_name": "回答问题",  "real_id": "49064", "order": 4, "questions": [{"no": 7}, {"no": 8}, {"no": 9}, {"no": 10}]},
        {"structure_type": "collector.picture", "group_name": "信息转述",  "real_id": "49065", "order": 5, "questions": [{"no": null}]},
        {"structure_type": "collector.role",    "group_name": "询问信息",  "real_id": "49066", "order": 6, "questions": [{"no": 1}, {"no": 2}]}
    ]);
    let homework = json!({
        "id": "98186857", "name": " 模拟试题1", "set_type": 1, "set_id": "4030",
        "resource_id": "58", "mock": 0.0, "exam": 0, "pk": 0,
        "struct": {"contents": [
            {"group_name": "模仿朗读"}, {"group_name": "听选信息1"}, {"group_name": "听选信息2"},
            {"group_name": "听选信息3"}, {"group_name": "回答问题"}, {"group_name": "信息转述"},
            {"group_name": "询问信息"}
        ]}
    });

    let merged = resolver::parse_homework_set_value(&packages, None, Some(&homework));
    let paper = &merged["paper"];
    let sections = paper["sections"].as_array().unwrap();

    let captions: Vec<&str> = sections.iter().map(|s| s["caption"].as_str().unwrap()).collect();
    assert_eq!(
        captions,
        vec!["模仿朗读", "听选信息", "回答问题", "信息转述", "询问信息"],
        "无模板时必须按归一化大题名分节（不能按 structure_type 把 role 三类并成一节）"
    );

    // 听选信息1/2/3 合并为一个大题（6 题），包内题号接续
    let listening = &sections[1];
    assert_eq!(listening["package_count"], 3);
    assert_eq!(listening["questions"].as_array().unwrap().len(), 6);
    assert_eq!(listening["real_ids"], json!(["49063", "49067", "49068"]));

    // 全局卷面题号连续 1..14（与广东 30 分卷结构 1+6+4+1+2 一致）
    let nos: Vec<i64> = sections
        .iter()
        .flat_map(|s| s["questions"].as_array().unwrap())
        .map(|q| q["no"].as_i64().unwrap())
        .collect();
    assert_eq!(nos, (1..=14).collect::<Vec<i64>>());
    assert_eq!(paper["question_count"], 14);

    // 分类随作业一起带出
    assert_eq!(paper["category"], "mock_suite");
    assert_eq!(paper["is_exercise_set"], true);
    assert_eq!(paper["totalScore"], 30.0);
}
