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

use std::collections::BTreeSet;

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
fn real_guangdong_paper_matches_official_res_json() {
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

    // 官方大题划分（来自真实 res.json）：听选信息被拆成 1/2/3 三个大题，序号必须保留
    let official_captions = vec![
        "模仿朗读", "听选信息1", "听选信息2", "听选信息3", "回答问题", "信息转述", "询问信息",
    ];

    // ---- 情形 A：无模板（只能靠资源包自己的 group_name 分节）----
    let merged = resolver::parse_homework_set_value(&packages, None, Some(&homework));
    let paper = &merged["paper"];
    let sections = paper["sections"].as_array().unwrap();
    let captions: Vec<&str> = sections.iter().map(|s| s["caption"].as_str().unwrap()).collect();
    assert_eq!(captions, official_captions, "无模板时也必须与官方 7 大题划分一致");
    assert_eq!(merged["coverage"]["ordered_by"], "package_order");
    let nos: Vec<i64> = sections
        .iter()
        .flat_map(|s| s["questions"].as_array().unwrap())
        .map(|q| q["no"].as_i64().unwrap())
        .collect();
    assert_eq!(nos, (1..=14).collect::<Vec<i64>>());
    assert_eq!(paper["question_count"], 14);

    // ---- 情形 B：带真实 res.json（官方模板）----
    let res_json = fixture("res_json_guangdong_mock1.json");
    let merged = resolver::parse_homework_set_value(&packages, Some(&res_json), Some(&homework));
    let paper = &merged["paper"];
    let sections = paper["sections"].as_array().unwrap();
    assert_eq!(merged["coverage"]["ordered_by"], "res_template");
    assert_eq!(merged["coverage"]["unmatched_packages"].as_array().unwrap().len(), 0,
               "7 个包都应挂上模板");
    let captions: Vec<&str> = sections.iter().map(|s| s["caption"].as_str().unwrap()).collect();
    assert_eq!(captions, official_captions);

    // 卷面总分与来源都取官方模板
    assert_eq!(paper["totalScore"], 30.0);
    assert_eq!(paper["totalScore_source"], "res_template_set_score");

    // 逐题满分与官方 question_list[].question_score 完全一致
    let scores: Vec<f64> = sections
        .iter()
        .flat_map(|s| s["questions"].as_array().unwrap())
        .map(|q| q["score"].as_f64().unwrap())
        .collect();
    assert_eq!(
        scores,
        vec![6.0, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 6.0, 1.5, 1.5]
    );
    assert!((scores.iter().sum::<f64>() - 30.0).abs() < 1e-9);

    // 分类随作业一起带出（真实作业是模拟套卷，7 包）
    assert_eq!(paper["category"], "mock_suite");
    assert_eq!(paper["is_exercise_set"], true);

    // res_template 自身也要给出官方口径
    let parsed = resolver::res_template_value(&res_json);
    assert_eq!(parsed["total_score"], 30.0);
    assert_eq!(parsed["total_score_source"], "set_score");
    assert_eq!(parsed["question_count"], 14);
    assert_eq!(parsed["sections"].as_array().unwrap().len(), 7);
}

#[test]
fn real_res_json_template_fields_are_parsed() {
    let res_json = fixture("res_json_guangdong_mock1.json");
    let parsed = resolver::res_template_value(&res_json);
    let sections = parsed["sections"].as_array().unwrap();

    // 大题顺序按官方 exam_type_order
    let orders: Vec<i64> = sections.iter().map(|s| s["exam_type_order"].as_i64().unwrap()).collect();
    assert_eq!(orders, (1..=7).collect::<Vec<i64>>());

    let first = &sections[0];
    assert_eq!(first["exam_type_name"], "模仿朗读");
    assert_eq!(first["exam_type_collect"], "collector.read");
    assert_eq!(first["exam_type_score"], 6.0);
    assert_eq!(first["exam_score"], 6.0);
    assert_eq!(first["questions"][0]["evaluation_type"], "read_chapter");
    assert_eq!(first["questions"][0]["engine_type"], "study");
    assert_eq!(first["questions"][0]["score"], 6.0);

    // 三个听选信息大题各 3 分 2 题
    for idx in [1, 2, 3] {
        let s = &sections[idx];
        assert!(s["exam_type_name"].as_str().unwrap().starts_with("听选信息"));
        assert_eq!(s["exam_score"], 3.0);
        assert_eq!(s["questions"].as_array().unwrap().len(), 2);
    }
    assert_eq!(parsed["warnings"].as_array().unwrap().len(), 0);
}

#[test]
fn real_guangdong_high_school_3q5a_paper_matches_official_res_json() {
    // 广东高中（高考听说 60 分卷）：模仿朗读 20 + 角色扮演 16(8×2) + 故事复述 24
    let res_json = fixture("res_json_guangdong_hs.json");
    let parsed = resolver::res_template_value(&res_json);
    assert_eq!(parsed["total_score"], 60.0);
    assert_eq!(parsed["total_score_source"], "set_score");
    assert_eq!(parsed["question_count"], 10); // 1 + 8 + 1
    let collects: Vec<&str> = parsed["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["exam_type_collect"].as_str().unwrap())
        .collect();
    assert_eq!(collects, vec!["collector.read", "collector.3q5a", "collector.picture"]);
    assert_eq!(parsed["sections"][1]["exam_score"], 16.0);
    assert_eq!(parsed["sections"][1]["questions"].as_array().unwrap().len(), 8);
    assert_eq!(parsed["sections"][1]["questions"][0]["score"], 2.0);
    assert_eq!(parsed["sections"][1]["questions"][0]["evaluation_type"], "simple_expression");

    // 真实 3q5a 资源包（角色扮演）：三问(中文题干) + 五答(英文问题) = 8 题，每题多参考答案
    let dir = [env!("CARGO_MANIFEST_DIR"), "tests", "corpus", "guangdong_hs_mock1", "01_角色扮演"];
    let path: std::path::PathBuf = dir.iter().collect();
    let content: Value =
        serde_json::from_str(&fs::read_to_string(path.join("content.json")).unwrap()).unwrap();
    let info: Value =
        serde_json::from_str(&fs::read_to_string(path.join("info.json")).unwrap()).unwrap();
    let content2: Value =
        serde_json::from_str(&fs::read_to_string(path.join("content2.json")).unwrap()).unwrap();
    assert_eq!(content["structure_type"], "collector.3q5a");

    let questions = resolver::parse_task_value(&content, Some(&content2), Some(&info));
    assert_eq!(questions.len(), 8, "三问 + 五答 = 8 题");
    assert!(questions.iter().all(|q| q["type"] == "collector.3q5a"));
    // 三问：题干是中文提示，答案是英文问句（多条）
    assert!(questions[0]["question"].as_str().unwrap().contains("父母"));
    let first_answers = questions[0]["answers"].as_array().unwrap();
    assert!(first_answers.len() >= 2, "三问每题应有多条参考问句");
    // 注意：官方参考答案里用的是弯引号（U+2019，如 "don’t"），**不得**做归一化 ——
    // E2/E3 要求逐字节与官方一致。这里只断言答案是英文（不含中日韩字符）。
    let has_cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    assert!(first_answers.iter().all(|a| !has_cjk(a.as_str().unwrap())));
    assert!(first_answers.iter().any(|a| a.as_str().unwrap().contains('\u{2019}') || a.as_str().unwrap().contains('\'')));
    // 五答：题干是英文问题，多条参考答案 + 采分点
    assert!(questions[3]["question"].as_str().unwrap().starts_with("How much"));
    assert!(!questions[3]["answers"].as_array().unwrap().is_empty());
    assert!(!questions[3]["important"].as_array().unwrap().is_empty(), "五答应带采分点");
    // 题号连续 1..8
    let nos: Vec<i64> = questions.iter().map(|q| q["no"].as_i64().unwrap()).collect();
    assert_eq!(nos, (1..=8).collect::<Vec<i64>>());

    // 合并成卷（真实 3 包 + 官方模板）
    let packages = json!([
        {"structure_type": "collector.read", "group_name": "模仿朗读", "code_id": "st1", "order": 0,
         "questions": [{"no": 1}]},
        {"structure_type": "collector.3q5a", "group_name": "角色扮演", "code_id": "st2", "order": 1,
         "questions": questions},
        {"structure_type": "collector.picture", "group_name": "故事复述", "code_id": "st3", "order": 2,
         "questions": [{"no": 1}]}
    ]);
    let homework = json!({"id": "102024948", "name": "模拟试题1", "set_type": 1, "resource_id": "1783",
        "struct": {"contents": [{"group_name": "模仿朗读"}, {"group_name": "角色扮演"}, {"group_name": "故事复述"}]}});
    let merged = resolver::parse_homework_set_value(&packages, Some(&res_json), Some(&homework));
    let paper = &merged["paper"];
    assert_eq!(paper["sections"].as_array().unwrap().len(), 3);
    assert_eq!(paper["question_count"], 10);
    assert_eq!(paper["totalScore"], 60.0);
    assert_eq!(paper["totalScore_source"], "res_template_set_score");
    assert_eq!(merged["coverage"]["unmatched_packages"].as_array().unwrap().len(), 0);
    let scores: Vec<f64> = paper["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["questions"].as_array().unwrap())
        .map(|q| q["score"].as_f64().unwrap())
        .collect();
    assert_eq!(scores, vec![20.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 24.0]);
    assert!((scores.iter().sum::<f64>() - 60.0).abs() < 1e-9);
}

#[test]
fn real_collector_repeat_paper_no_longer_loses_questions() {
    // 广东高中"听说能力监测 20 分卷"：单词朗读 4 + 课内语篇朗读 4 + 课内语篇听读 4
    // + 课外语篇听读 4 + 故事复述 4 = 20 分（5 大题各 1 题）
    // 修复前：collector.repeat（课内/课外语篇听读）解析出 0 题 -> 整卷少 8 分。
    let res_json = fixture("res_json_guangdong_hs_monitor.json");
    let parsed = resolver::res_template_value(&res_json);
    assert_eq!(parsed["total_score"], 20.0);
    assert_eq!(parsed["question_count"], 5);
    let collects: Vec<&str> = parsed["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["exam_type_collect"].as_str().unwrap())
        .collect();
    assert_eq!(
        collects,
        vec!["collector.read", "collector.read", "collector.repeat", "collector.repeat", "collector.picture"]
    );

    // 逐包解析：每个包都必须出 1 题（修复的核心断言）
    let base: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "corpus", "guangdong_hs_monitor_a"]
        .iter()
        .collect();
    let cases = ["0_单词朗读", "1_课内语篇朗读", "2_课内语篇听读", "3_课外语篇听读", "4_故事复述"];
    let mut packages = Vec::new();
    let mut total_questions = 0;
    for (idx, case) in cases.iter().enumerate() {
        let dir = base.join(case);
        let content: Value = serde_json::from_str(&fs::read_to_string(dir.join("content.json")).unwrap()).unwrap();
        let info: Value = serde_json::from_str(&fs::read_to_string(dir.join("info.json")).unwrap()).unwrap();
        let content2: Value = serde_json::from_str(&fs::read_to_string(dir.join("content2.json")).unwrap()).unwrap();
        let questions = resolver::parse_task_value(&content, Some(&content2), Some(&info));
        assert_eq!(
            questions.len(),
            1,
            "{case}（structure_type={}）应出 1 题，实际 {} —— 文本跟读类不能为空",
            content["structure_type"],
            questions.len()
        );
        // 听读/朗读类：answers 直接给原文（与 collector.read 语义一致）
        let answers = questions[0]["answers"].as_array().unwrap();
        assert!(!answers.is_empty(), "{case} 应把原文作为答案输出");
        assert!(answers[0].as_str().unwrap().len() > 10);
        total_questions += questions.len();
        packages.push(json!({
            "structure_type": content["structure_type"],
            "group_name": case.split('_').nth(1).unwrap(),
            "code_id": format!("st{}", idx + 1),
            "order": idx,
            "questions": questions,
        }));
    }
    assert_eq!(total_questions, 5);

    let homework = json!({"id": "104456286", "name": "9月4日周末听说练习", "set_type": 1, "resource_id": "1784",
        "struct": {"contents": [{"group_name": "单词朗读"}, {"group_name": "课内语篇朗读"}, {"group_name": "课内语篇听读"},
                                {"group_name": "课外语篇听读"}, {"group_name": "故事复述"}]}});
    let merged = resolver::parse_homework_set_value(&json!(packages), Some(&res_json), Some(&homework));
    let paper = &merged["paper"];
    assert_eq!(paper["totalScore"], 20.0, "修复后整卷应是 20 分（修复前 12 分）");
    assert_eq!(paper["totalScore_source"], "res_template_set_score");
    assert_eq!(paper["question_count"], 5);
    assert_eq!(merged["coverage"]["unmatched_packages"].as_array().unwrap().len(), 0);
    let scores: Vec<f64> = paper["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["questions"].as_array().unwrap())
        .map(|q| q["score"].as_f64().unwrap())
        .collect();
    assert_eq!(scores, vec![4.0, 4.0, 4.0, 4.0, 4.0]);
}

#[test]
fn explicit_route_without_questions_is_reported() {
    // 显式命中却出不了题，必须留下 explicit_route_empty 告警（否则会被 layer=explicit 掩盖）
    let content = json!({"structure_type": "collector.repeat", "info": {"translate": "", "value": ""}});
    let report = resolver::diagnose_value(&content, None, None);
    let warnings = report["warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|w| w == "explicit_route_empty"));
    assert!(warnings.iter().any(|w| w == "no_questions"));
}

#[test]
fn real_vocabulary_payload_uses_official_question_type_name() {
    // 真实 `/g/word/struct` 响应（广东高中 单词PK）：每题自带官方 `question_type_name`，
    // 它比数字映射更权威，也能消除"同一数字跨通道语义不同"的歧义。
    let fx = fixture("word_struct_guangdong_hs_pk.json");
    let body = &fx["body"];

    // 官方名 → 我们的 type（全量实测：6/16/17/18/19/20 六个题型）
    let pairs: Vec<(i64, &str, &str)> = vec![
        (6, "听音选义", "word.choice"),
        (16, "默写词汇", "word.write"),
        (17, "朗读词汇", "word.read"),
        (18, "听写词汇", "word.dictation"),
        (19, "英译中", "word.translate_en_cn"),
        (20, "中译英", "word.translate_cn_en"),
    ];
    for (qtype, name, _expect_type) in &pairs {
        let q = body["data"][0]["question"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["question_type"].as_i64() == Some(*qtype))
            .unwrap_or_else(|| panic!("fixture 缺少 qtype={qtype}"));
        assert_eq!(q["question_type_name"], *name, "官方名应与实测一致");
    }

    let questions = resolver::parse_word_pk_value(body);
    assert_eq!(questions.len(), 3 * 6, "3 词 × 6 题型");

    // 每个 type 都出现，且官方名/官方答案被原样带出
    let kinds: BTreeSet<&str> = questions.iter().map(|q| q["type"].as_str().unwrap()).collect();
    for (_, _, expect_type) in &pairs {
        assert!(kinds.contains(expect_type), "缺少题型 {expect_type}");
    }
    let first = &questions[0];
    assert_eq!(first["channel"], "sync_pk");
    assert!(!first["question_type_name"].as_str().unwrap().is_empty());
    assert!(
        first["official_answer"].as_str().is_some(),
        "选择题应带官方原始答案（选项字母）"
    );
    // 选择题：归一化答案 = 官方字母；选项存在
    let choice = questions
        .iter()
        .find(|q| q["type"] == "word.choice")
        .unwrap();
    assert_eq!(choice["answers"][0], choice["official_answer"]);
    assert!(choice["options"].as_array().unwrap().len() >= 3);
    // 朗读类：官方 answer 为空，但我们的 answers 给原文（与 read 家族语义一致）
    let read = questions.iter().find(|q| q["type"] == "word.read").unwrap();
    assert!(!read["answers"].as_array().unwrap().is_empty());
    // 英译中/中译英：题干与答案互为一对（英文 / 中文释义）
    let en_cn = questions.iter().find(|q| q["type"] == "word.translate_en_cn").unwrap();
    assert_eq!(en_cn["question"], "trait");
    assert!(!en_cn["answers"][0].as_str().unwrap().contains("trait"));
}
