//! `collector.repeat`：`info.fj[]` **一句一题**（官方语义）。
//!
//! 证据：
//! - PC 前端 `useExamStepper`：`case"collector.repeat": for(const n of r.info.fj) e = `${D}_${r.info.stid}_${n.xh}``
//!   —— 逐句生成同步键 = 一句一题，题号取 `fj[].xh`。
//! - 答案页模板 `v-for='i in one.info.fj'`。
//! - 真实内容包：`info.fj` 10/12 项，元素 `{value, xh, fs, ai, time, audio}`；
//!   同目录 `info_extra.json` 行数与之相同（决定小题数与分值）。
//!
//! 旧行为把整段 `info.value` 出成 **1 题** → 实测丢掉 9~11 题。

use serde_json::{json, Value};

fn load(name: &str) -> Value {
    let path = format!("tests/corpus/{name}/content.json");
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不到 {path}: {e}"));
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn repeat_splits_one_question_per_sentence() {
    let content = load("guangdong_hs_monitor_a/2_课内语篇听读");
    let fj_len = content["info"]["fj"].as_array().unwrap().len();
    assert_eq!(fj_len, 10, "真实包应有 10 句（前置事实）");

    let (qs, family) = resolver::collectors::collect_generic_routed(&content, None, None, None);
    assert_eq!(family, "repeat_by_sentence");
    assert_eq!(qs.len(), fj_len, "一句一题：10 句必须出 10 题（旧行为只有 1 题）");

    // 题号 = fj[].xh，且逐句答案 = fj[].value
    let nos: Vec<i64> = qs.iter().map(|q| q["no"].as_i64().unwrap()).collect();
    assert_eq!(nos, (1..=10).collect::<Vec<_>>());
    assert_eq!(qs[0]["answers"][0], json!("It boils down to this:"));
    assert_eq!(qs[0]["type"], json!("collector.repeat"));
    // 逐句音频与录音时长（官方 fj[].audio / info_extra 的 record_time）
    assert_eq!(qs[0]["audio"], json!("fj1.mp3"));
    assert_eq!(qs[0]["record_time"], json!("15"));
    assert_eq!(qs[9]["audio"], json!("fj10.mp3"));
}

#[test]
fn repeat_second_real_package_has_twelve() {
    let content = load("guangdong_hs_monitor_a/3_课外语篇听读");
    let (qs, family) = resolver::collectors::collect_generic_routed(&content, None, None, None);
    assert_eq!(family, "repeat_by_sentence");
    assert_eq!(qs.len(), 12);
    // HTML 标签必须被清洗（真实包里有 <i>The Kite Runner</i>）
    let second = qs[1]["answers"][0].as_str().unwrap();
    assert!(!second.contains("<i>"), "答案不应残留 HTML 标签: {second}");
}

#[test]
fn repeat_without_fj_keeps_old_text_collection_behaviour() {
    // 回归保护：没有 info.fj 的 repeat 资源包仍按"文本集合"处理，不能变成 0 题
    let content = json!({
        "structure_type": "collector.repeat",
        "info": {"stid": "1", "value": "Some passage without per-sentence rows."}
    });
    let (qs, _family) = resolver::collectors::collect_generic_routed(&content, None, None, None);
    assert!(!qs.is_empty(), "没有 fj 时不能一题都不出");
}

#[test]
fn read_family_with_fj_is_not_expanded() {
    // 「模仿朗读」是 collector.read：官方整段 1 题，**不按句拆**。
    // 即使某个 read 包恰好带 info.fj，也不能被展开（语料实测 fj 只在 repeat，但要有守卫）。
    let content = json!({
        "structure_type": "collector.read",
        "info": {
            "stid": "77",
            "value": "Della searched the stores. She found a watch chain.",
            "fj": [
                {"xh": "1", "value": "Della searched the stores."},
                {"xh": "2", "value": "She found a watch chain."}
            ]
        }
    });
    let (qs, family) = resolver::collectors::collect_generic_routed(&content, None, None, Some("collector.read"));
    assert_ne!(family, "repeat_by_sentence", "read 族不能被逐句展开");
    assert_eq!(qs.len(), 1, "read 族应保持 1 题，实际 {}", qs.len());
}
