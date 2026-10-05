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

fn corpus_path(case: &str) -> std::path::PathBuf {
    let mut p: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "corpus"].iter().collect();
    p.push(case);
    p
}

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

#[test]
fn l3_salvage_recovers_answers_from_unknown_shape() {
    // 全新的 structure_type + 全新形状：L1/L2 都认不出，L3 兜底必须尽量抢救
    let content = json!({
        "structure_type": "collector.brand_new_2030",
        "info": {
            "stid": "999",
            "blocks": [
                { "kind": "qa", "ask": "What is your name?", "answer": "My name is Li Hua." },
                { "kind": "choice", "stem": "Pick one", "answer": "B",
                  "options": [{"name": "A", "value": "a"}, {"name": "B", "value": "b"}] },
                { "kind": "list", "std": [{"xth": "1", "value": "first"}, {"xth": "2", "value": "second"}] }
            ]
        }
    });
    let report = resolver::diagnose_value(&content, None, None);
    assert_eq!(report["layer"], "salvage", "L3 兜底应被命中");
    assert_eq!(report["family"], "salvage:structural_scan");

    let qs = resolver::parse_task_value(&content, None, None);
    assert_eq!(qs.len(), 4, "1 个问答 + 1 个选择 + 2 个 std 条目");
    for q in &qs {
        assert_eq!(q["confidence"], "low", "兜底结果必须标低置信");
        assert_eq!(q["type"], "collector.salvaged");
        assert!(!q["salvage_path"].as_str().unwrap().is_empty());
        assert!(!q["answers"].as_array().unwrap().is_empty());
    }
    // 题号被补齐，且能看出答案来自哪里
    let nos: Vec<i64> = qs.iter().map(|q| q["no"].as_i64().unwrap()).collect();
    assert_eq!(nos, vec![1, 2, 3, 4]);
    let picks: Vec<&str> = qs.iter().map(|q| q["salvage_path"].as_str().unwrap()).collect();
    assert!(picks.iter().any(|p| p.contains("blocks")));
    // 选项被抢救出来
    assert!(qs.iter().any(|q| !q["options"].as_array().unwrap().is_empty()));
}

#[test]
fn l3_salvage_does_not_touch_known_shapes() {
    // 已知形状必须仍走 L1（显式）/L2（嗅探），不得降级到低置信兜底
    let known = json!({"structure_type": "collector.role",
        "info": {"stid": "1", "question": [{"ask": "Q", "std": [{"value": "A"}]}]}});
    let qs = resolver::parse_task_value(&known, None, None);
    assert_eq!(qs.len(), 1);
    assert!(qs[0].get("confidence").is_none(), "正常路径不应带 confidence");

    let sniffable = json!({"structure_type": "collector.unknown_2027",
        "info": {"stid": "9", "question": [{"ask": "Q1", "std": [{"value": "A1"}]}]}});
    let report = resolver::diagnose_value(&sniffable, None, None);
    assert_eq!(report["layer"], "generic", "形状熟悉时应走 L2 而不是 L3");
}

#[test]
fn completeness_guard_detects_missing_questions() {
    // 漏题必须能被机器发现：卷面分来自官方 set_score，少几题也照样是 30 分，
    // 光看总分/成功标志看不出问题 —— 所以逐大题对账 + 明确告警。
    let res_json = fixture("res_json_guangdong_mock1.json");
    // 注意：官方模板把 7 个大题编号为 st1..st7，其中 回答问题 = st5
    let dirs = [
        ("00_模仿朗读", "st1"),
        ("01_听选信息1", "st2"),
        ("02_听选信息2", "st3"),
        ("04_回答问题", "st5"),
    ];
    let mut packages = Vec::new();
    for (i, (case, code_id)) in dirs.iter().enumerate() {
        let dir = corpus_path("guangdong_mock1").join(case);
        let content: Value = serde_json::from_str(&fs::read_to_string(dir.join("content.json")).unwrap()).unwrap();
        let info: Value = serde_json::from_str(&fs::read_to_string(dir.join("info.json")).unwrap()).unwrap();
        let content2: Value = serde_json::from_str(&fs::read_to_string(dir.join("content2.json")).unwrap()).unwrap();
        let mut questions = resolver::parse_task_value(&content, Some(&content2), Some(&info));
        // 故意把「回答问题」的题目砍掉一半（模拟解析/下载漏题）
        if case.contains("回答问题") {
            questions.truncate(questions.len() / 2);
        }
        packages.push(json!({
            "structure_type": content["structure_type"],
            "group_name": case.splitn(2, '_').nth(1).unwrap(),
            "code_id": code_id,
            "order": i,
            "questions": questions,
        }));
    }
    let merged = resolver::parse_homework_set_value(&json!(packages), Some(&res_json), None);
    let completeness = &merged["completeness"];
    assert_eq!(completeness["is_complete"], false, "少题必须判为不完整");
    assert!(
        completeness["missing_vs_template"].as_u64().unwrap() > 0,
        "应报告比官方模板少了几题"
    );
    let below = completeness["sections_below_template"].as_array().unwrap();
    assert!(
        below.iter().any(|s| s.as_str().unwrap_or("").contains("回答问题")),
        "应点出是哪个大题少题: {below:?}"
    );
    let warnings = merged["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w.as_str().unwrap_or("").starts_with("question_count_below_template")),
        "必须给出可发现的告警: {warnings:?}"
    );
    // 只挂上 4/7 个大题时，卷面分不取官方 set_score（30），而是已挂载大题之和（6+3+3+6=18）
    // —— 半张卷不能报满分。真实整卷（7/7）才会用官方 set_score。
    // 这也说明：**看总分不足以发现漏题**，必须靠 completeness 逐节对账。
    assert_eq!(merged["paper"]["totalScore"], 18.0);
    assert_eq!(merged["paper"]["totalScore_source"], "res_template_partial");
}

#[test]
fn completeness_guard_reports_full_alignment_for_real_paper() {
    // 正常情况：与官方模板逐节对齐、无告警
    let res_json = fixture("res_json_guangdong_hs_monitor.json");
    let mut packages = Vec::new();
    for (i, case) in ["0_单词朗读", "1_课内语篇朗读", "2_课内语篇听读", "3_课外语篇听读", "4_故事复述"]
        .iter()
        .enumerate()
    {
        let dir = corpus_path("guangdong_hs_monitor_a").join(case);
        let content: Value = serde_json::from_str(&fs::read_to_string(dir.join("content.json")).unwrap()).unwrap();
        let info: Value = serde_json::from_str(&fs::read_to_string(dir.join("info.json")).unwrap()).unwrap();
        let content2: Value = serde_json::from_str(&fs::read_to_string(dir.join("content2.json")).unwrap()).unwrap();
        let questions = resolver::parse_task_value(&content, Some(&content2), Some(&info));
        packages.push(json!({
            "structure_type": content["structure_type"],
            "group_name": case.splitn(2, '_').nth(1).unwrap(),
            "code_id": format!("st{}", i + 1),
            "order": i,
            "questions": questions,
        }));
    }
    let merged = resolver::parse_homework_set_value(&json!(packages), Some(&res_json), None);
    let completeness = &merged["completeness"];
    assert_eq!(completeness["is_complete"], true);
    assert_eq!(completeness["missing_vs_template"], 0);
    assert_eq!(completeness["template_questions"], 5);
    assert_eq!(completeness["parsed_questions"], 5);
    // 每个大题都用官方挂载键挂上（无家族兜底）
    for section in merged["paper"]["sections"].as_array().unwrap() {
        assert_eq!(section["mount_confidence"], "exact", "{section}");
        assert_eq!(section["count_status"], "match", "{section}");
    }
    let warnings = merged["warnings"].as_array().unwrap();
    assert!(
        !warnings.iter().any(|w| w.as_str().unwrap_or("").contains("below_template")),
        "对齐时不应有少题告警: {warnings:?}"
    );
}

#[test]
fn mount_reason_exposes_family_fallback() {
    // 大题名与挂载键都对不上时，题目会被"同结构家族"兜底挂到相邻大题：
    // 题不丢，但必须让调用方看见这次挂载不可靠。
    let res_json = fixture("res_json_guangdong_mock1.json");
    let dir = corpus_path("guangdong_mock1").join("01_听选信息1");
    let content: Value = serde_json::from_str(&fs::read_to_string(dir.join("content.json")).unwrap()).unwrap();
    let info: Value = serde_json::from_str(&fs::read_to_string(dir.join("info.json")).unwrap()).unwrap();
    let content2: Value = serde_json::from_str(&fs::read_to_string(dir.join("content2.json")).unwrap()).unwrap();
    let questions = resolver::parse_task_value(&content, Some(&content2), Some(&info));
    // structure_type 用官方 code 值（listen_choice_standard，handler=collector.role），
    // 与模板的 exam_type_collect（collector.role）不相等 -> 只能走"同结构家族"兜底
    let packages = json!([
        {"structure_type": "listen_choice_standard", "group_name": "老师新加的大题", "code_id": "",
         "order": 0, "questions": questions},
        {"structure_type": "collector.read", "group_name": "模仿朗读", "code_id": "st1",
         "order": 1, "questions": resolver::parse_task_value(
            &serde_json::from_str::<Value>(&fs::read_to_string(
                corpus_path("guangdong_mock1").join("00_模仿朗读").join("content.json")).unwrap()).unwrap(),
            None, None)},
    ]);
    let merged = resolver::parse_homework_set_value(&packages, Some(&res_json), None);
    let warnings = merged["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w.as_str().unwrap_or("").starts_with("package_mounted_by_family_fallback")),
        "家族兜底挂载必须告警: {warnings:?}"
    );
    let completeness = &merged["completeness"];
    assert!(
        !completeness["sections_weak_mount"].as_array().unwrap().is_empty(),
        "应点出靠兜底挂载的大题"
    );
    // 题目不丢
    let total: u64 = merged["paper"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["questions"].as_array().unwrap().len() as u64)
        .sum();
    assert!(total >= 3, "兜底也绝不能丢题，实际 {total}");
}

#[test]
fn completeness_guard_detects_package_lost_before_parsing() {
    // 更早阶段的漏题：资源包在下载/提取阶段就丢了，根本到不了解析器。
    // 靠"作业声明的包数 vs 实际解析包数"对账抓出来。
    let res_json = fixture("res_json_guangdong_hs_monitor.json");
    let dir = corpus_path("guangdong_hs_monitor_a").join("0_单词朗读");
    let content: Value = serde_json::from_str(&fs::read_to_string(dir.join("content.json")).unwrap()).unwrap();
    let questions = resolver::parse_task_value(&content, None, None);
    // 作业声明 5 个包，只传进来 2 个
    let homework = json!({"id": "104456286", "name": "听说练习",
        "contents": [{"group_name": "单词朗读"}, {"group_name": "课内语篇朗读"},
                     {"group_name": "课内语篇听读"}, {"group_name": "课外语篇听读"},
                     {"group_name": "故事复述"}]});
    let packages = json!([
        {"structure_type": "collector.read", "group_name": "单词朗读", "code_id": "st1",
         "order": 0, "questions": questions},
        {"structure_type": "collector.read", "group_name": "课内语篇朗读", "code_id": "st2",
         "order": 1, "questions": questions},
    ]);
    let merged = resolver::parse_homework_set_value(&packages, Some(&res_json), Some(&homework));
    assert_eq!(merged["coverage"]["declared_packages"], 5);
    assert_eq!(merged["coverage"]["packages"], 2);
    assert_eq!(merged["coverage"]["missing_packages"], 3);
    let warnings = merged["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w.as_str().unwrap_or("").starts_with("packages_missing_vs_homework")),
        "包数对不上必须告警: {warnings:?}"
    );
    assert_eq!(merged["completeness"]["is_complete"], false);
}

#[test]
fn unified_question_schema_across_all_official_families() {
    // 「输出统一格式」的机器化断言：10 个官方解析家族 + 词汇，题目对象字段集必须一致。
    // 各家族载荷形状差异极大（xtlist / std / question+std / info.value / 双通道词汇），
    // 但输出必须收敛成同一套字段，客户端才能只写一份渲染逻辑。
    let corpus = corpus_path("");
    let mut checked: Vec<String> = Vec::new();
    let cases: Vec<(&str, &str)> = vec![
        ("guangdong_mock1", "00_模仿朗读"),
        ("guangdong_mock1", "01_听选信息1"),
        ("guangdong_mock1", "05_信息转述"),
        ("guangdong_hs_mock1", "01_角色扮演"),
        ("guangdong_hs_monitor_a", "2_课内语篇听读"),
        ("guangdong_hs_monitor_a", "4_故事复述"),
        ("_examples", "choose_xtlist/mock-001"),
        ("_examples", "std_fill/mock-001"),
    ];
    for (group, case) in cases {
        let dir = corpus.join(group).join(case);
        if !dir.join("content.json").is_file() {
            continue;
        }
        let content: Value = serde_json::from_str(&fs::read_to_string(dir.join("content.json")).unwrap()).unwrap();
        let info = fs::read_to_string(dir.join("info.json")).ok().map(|s| serde_json::from_str::<Value>(&s).unwrap());
        let content2 = fs::read_to_string(dir.join("content2.json")).ok().map(|s| serde_json::from_str::<Value>(&s).unwrap());
        let questions = resolver::parse_task_value(&content, content2.as_ref(), info.as_ref());
        assert!(!questions.is_empty(), "{case} 应有题目");
        for q in &questions {
            let obj = q.as_object().unwrap();
            // 统一契约的必需字段（缺一不可）
            for key in ["no", "type", "question", "answers"] {
                assert!(
                    obj.contains_key(key),
                    "{case}（{}）缺少统一字段 `{key}`: {q}",
                    content["structure_type"]
                );
            }
            assert!(obj["answers"].is_array(), "{case} answers 必须是数组");
            assert!(obj["no"].is_number() || obj["no"].is_null(), "{case} no 必须是数字");
            // 可选但约定存在的字段
            assert!(obj.contains_key("options"), "{case} 应始终有 options（可为空数组）");
            assert!(obj.contains_key("stid"), "{case} 应始终有 stid（可为 null）");
        }
        checked.push(format!(
            "{}（{}）{} 题",
            case,
            content["structure_type"].as_str().unwrap_or(""),
            questions.len()
        ));
    }
    assert!(checked.len() >= 6, "至少应覆盖 6 个真实家族: {checked:?}");

    // 词汇走同一套字段
    let vocab = fixture("word_struct_guangdong_hs_pk.json");
    let word_questions = resolver::parse_word_pk_value(&vocab["body"]);
    assert!(!word_questions.is_empty());
    for q in &word_questions {
        let obj = q.as_object().unwrap();
        for key in ["no", "type", "question", "answers"] {
            assert!(obj.contains_key(key), "词汇缺统一字段 `{key}`: {q}");
        }
    }
}
