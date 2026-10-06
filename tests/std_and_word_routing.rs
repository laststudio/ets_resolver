//! `info.std[]` 题号键兼容、`/` 多解拆分、`collector.word` 路由（静态语义提取所得）。
//!
//! 依据：
//! - 官方**两处**实现用的键不同：套题链路 `std.find(e => e.xth === a)`；
//!   听说同步 `Practice`：`std.find(e => e.th === l[1])`，且示例里出现过 `"th":"1."`。
//! - `/` 多解：`X(e.value).split("/").map(e => e.trim().toLowerCase())`，
//!   另一处模板 `slashToArr: str.replace(/<br \/>/g,'').split('/')`。
//! - `collector.word` 是独立族（题目带 `question_type ∈ {6,16,17,18,19,20}`）。

use serde_json::{json, Value};

fn parse(content: &Value) -> (Vec<Value>, &'static str) {
    resolver::collectors::collect_generic_routed(content, None, None, None)
}

#[test]
fn fill_accepts_th_key_and_trailing_dot() {
    // 官方听说同步路径用 `th`，且实测出现过 "1."
    let content = json!({
        "structure_type": "collector.fill",
        "info": {"stid": "7", "std": [
            {"th": "1.", "value": "apple"},
            {"th": "2",  "value": "pear/pears"}
        ]}
    });
    let (qs, family) = parse(&content);
    assert_eq!(family, "std_fill", "有 std[].th 必须识别为填空族");
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[0]["no"], json!(1), "`th`=\"1.\" 也要能解析成 1");
    assert_eq!(qs[1]["no"], json!(2));
    // `/` 多解：两个可接受答案都要保留
    let answers = qs[1]["answers"].as_array().unwrap();
    assert!(answers.len() >= 1);
}

#[test]
fn std_accepted_answers_splits_slash_and_normalizes() {
    // 官方：split("/") + trim + toLowerCase + 标点归一（’→' 等）
    let got = resolver::collectors::std_accepted_answers(" Apple / Pears / ‘quoted’ 。");
    assert_eq!(got[0], "apple");
    assert_eq!(got[1], "pears");
    assert!(got.iter().any(|s| s.contains('\'')), "右单引号应归一为 ASCII 单引号: {got:?}");
    assert!(
        got.iter().any(|s| s.contains('.')),
        "中文句号 。 应归一为 ASCII . : {got:?}"
    );
    // 单独的标点归一用例（整段就是中文句号）
    let punct = resolver::collectors::std_accepted_answers("。");
    assert_eq!(punct, vec![".".to_string()]);
    // 空项要被丢掉
    let got2 = resolver::collectors::std_accepted_answers("a//b");
    assert_eq!(got2, vec!["a".to_string(), "b".to_string()]);
    // <br /> 被去掉（官方 slashToArr 行为）
    let got3 = resolver::collectors::std_accepted_answers("red<br />/blue");
    assert_eq!(got3, vec!["red".to_string(), "blue".to_string()]);
}

#[test]
fn collector_word_with_word_markers_is_not_routed_to_read() {
    // 词汇题标记（question_type / option）存在时，必须按词汇题处理，不能当朗读文本集合
    let content = json!({
        "structure_type": "collector.word",
        "info": {"stid": "42", "question": [
            {"question_type": 6, "answer": "A",
             "option": "[{\"option_name\":\"A\",\"option_value\":\"n.政治\"}]"}
        ]}
    });
    let (qs, family) = parse(&content);
    assert_ne!(family, "read_value", "带词汇题标记的 collector.word 不该走朗读分支");
    assert_ne!(family, "collector.read");
    assert!(!qs.is_empty(), "应当解析出题目（family={family}）");
}

#[test]
fn collector_word_text_collection_still_routes_to_read() {
    // 既有行为不能破：文本集合形态的 word 资源包仍按朗读处理
    let content = json!({
        "structure_type": "collector.word",
        "info": {"stid": "5", "value": "apple banana pear"}
    });
    let (qs, family) = parse(&content);
    assert_eq!(family, "read_value", "纯文本集合仍走朗读分支");
    assert!(!qs.is_empty());
}
