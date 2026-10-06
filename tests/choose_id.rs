//! `choose` 族的 `choose_id` 提取（官方 payload 字段）。
//!
//! 依据（官方前端明文源码 `useExamStepper-8ad3fbd7.js::G()`）：
//! ```js
//! const e = r.find(e => e.xt_xh === o);              // 按 order 找题
//! const s = e.xxlist.find(e => e.xx_xh == n);        // 按 xx_xh 找选项
//! return s ? (e.answer === s.xx_mc && (l = m(a.question_score / a.question_count)),
//!   { entity_id:t, order:o, score_detail:{ total_score:l, choose:s.xx_mc,
//!     choose_id:n, category:a.evaluation_type } }) : null
//! ```
//! 即：判分 = `xtlist[].answer === xxlist[].xx_mc`（两边都是选项字母），
//!     提交的 `choose` = `xx_mc`、`choose_id` = `xx_xh`。
//! 真实模板佐证：`{"xx_xh":"1","xx_mc":"A","xx_nr":" The history of TV."}`、`"answer":"A"`。

use serde_json::Value;

fn parse_fixture() -> Vec<Value> {
    let raw = std::fs::read_to_string("tests/corpus/_examples/choose_xtlist/mock-001/content.json").unwrap();
    let content: Value = serde_json::from_str(&raw).unwrap();
    resolver::collectors::collect_choose(&content, None, None)
}

#[test]
fn choose_id_is_the_correct_option_serial() {
    let qs = parse_fixture();
    assert_eq!(qs.len(), 1, "一题");
    let q = &qs[0];
    // 答案字母仍是 C（submit_answer 用字母，已用真实语料定案）
    assert_eq!(q["answers"][0], Value::String("C".into()));
    // 正确答案 C 对应的选项序号是 "3"
    assert_eq!(q["choose_id"], Value::String("3".into()), "choose_id 应为正确选项的 xx_xh");
    // 结构化选项（含序号/字母/文本）
    let od = q["options_detail"].as_array().expect("options_detail 必须是数组");
    assert_eq!(od.len(), 3);
    assert_eq!(od[0]["xx_xh"], Value::String("1".into()));
    assert_eq!(od[0]["xx_mc"], Value::String("A".into()));
    assert_eq!(od[0]["xx_nr"], Value::String("Take the bag.".into()));
    assert_eq!(od[2]["xx_xh"], Value::String("3".into()));
    assert_eq!(od[2]["xx_mc"], Value::String("C".into()));
}

#[test]
fn choose_id_empty_when_answer_has_no_matching_option() {
    // answer 与任何 xx_mc 都不匹配时，choose_id 必须是空串（而不是乱填）
    let content: Value = serde_json::from_str(
        r#"{"structure_type":"collector.choose","info":{"stid":"9","xtlist":[
             {"xt_xh":"1","xt_nr":"Q?","answer":"Z",
              "xxlist":[{"xx_xh":"1","xx_mc":"A","xx_nr":"a"},{"xx_xh":"2","xx_mc":"B","xx_nr":"b"}]}]}}"#,
    ).unwrap();
    let qs = resolver::collectors::collect_choose(&content, None, None);
    assert_eq!(qs[0]["choose_id"], Value::String("".into()));
}

#[test]
fn choose_id_matches_case_insensitively() {
    let content: Value = serde_json::from_str(
        r#"{"structure_type":"collector.choose","info":{"stid":"9","xtlist":[
             {"xt_xh":"1","xt_nr":"Q?","answer":"b",
              "xxlist":[{"xx_xh":"1","xx_mc":"A","xx_nr":"a"},{"xx_xh":"2","xx_mc":"B","xx_nr":"b"}]}]}}"#,
    ).unwrap();
    let qs = resolver::collectors::collect_choose(&content, None, None);
    assert_eq!(qs[0]["choose_id"], Value::String("2".into()));
}
