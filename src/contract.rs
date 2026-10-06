// -*- coding: utf-8 -*-
// 跨平台固定接口契约 (contract v1)
//
// 目标：任何平台（Android/iOS/Windows/Web/后端）只实现"一次 JSON 进、一次 JSON 出"的桥，
// 后续所有新能力都通过新增 `op` 交付，不再改动 ABI，也不需要各端重新适配。
//
// 请求信封：
//   {"version": 1, "op": "parse_task", "payload": {"content": {...}, "info": {...}}}
//   * version 可省略（等价于 1）；
//   * payload 可省略，字段直接放在请求根上（宽容解析）；
//   * 任何 JSON 字段都接受"对象"或"JSON 文本字符串"两种形态（跨语言最省事）。
//
// 响应信封（成功）：
//   {"ok": true, "version": 1, "op": "parse_task", "result": ..., "meta": {...}}
// 响应信封（失败）：
//   {"ok": false, "version": 1, "op": "parse_task",
//    "error": {"code": "invalid_input", "message": "...", "hint": "..."}}
//
// 能力协商：先调 `capabilities`，拿到 ops / 错误码 / 枚举，再决定调用哪些 op。
// 客户端不做版本嗅探，只判断 `ok` 与 `error.code`。

use serde_json::{json, Map, Value};

use crate::collectors::parse_content_routed;
use crate::{api, diagnose, homework, homework_set, spec, template};

/// 契约版本。新增 op 不涨版本；**破坏性字段变更**才涨，且旧版本请求必须继续可用。
pub const CONTRACT_VERSION: u64 = 1;

fn error(code: &str, message: &str, hint: &str) -> Value {
    json!({"code": code, "message": message, "hint": hint})
}

fn respond_ok(op: &str, result: Value, meta: Option<Value>) -> Value {
    let mut out = Map::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("version".into(), json!(CONTRACT_VERSION));
    out.insert("op".into(), Value::String(op.to_string()));
    out.insert("result".into(), result);
    if let Some(m) = meta {
        out.insert("meta".into(), m);
    }
    Value::Object(out)
}

fn respond_err(op: &str, err: Value) -> Value {
    json!({
        "ok": false,
        "version": CONTRACT_VERSION,
        "op": op,
        "error": err,
    })
}

/// 字段取值：对象/数组直接用；字符串先尝试按 JSON 解析，
/// **解析失败则保留为普通字符串**（否则 `"structure_type": "listen_choice_standard"`
/// 这种非 JSON 的纯字符串会被静默丢掉）。
fn as_json_value(v: Option<&Value>) -> Option<Value> {
    match v {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                serde_json::from_str::<Value>(trimmed)
                    .ok()
                    .or_else(|| Some(Value::String(s.clone())))
            }
        }
        Some(other) => Some(other.clone()),
    }
}

/// 取整数下标：接受数字、数字字符串、JSON 字符串形态的数字。
fn as_index(v: &Value) -> Option<usize> {
    v.as_u64()
        .map(|n| n as usize)
        .or_else(|| v.as_str().and_then(|s| s.trim().parse::<usize>().ok()))
}

/// 从 payload（或请求根）里取字段，并做"对象 / JSON 字符串"归一。
fn field(request: &Value, payload: &Value, key: &str) -> Option<Value> {
    let raw = payload
        .get(key)
        .filter(|v| !v.is_null())
        .or_else(|| request.get(key).filter(|v| !v.is_null()));
    as_json_value(raw)
}

fn required(request: &Value, payload: &Value, key: &str) -> Result<Value, Value> {
    field(request, payload, key).ok_or_else(|| {
        error(
            "invalid_input",
            &format!("缺少必填字段 `{key}`"),
            "字段可以是 JSON 对象，也可以是 JSON 文本字符串",
        )
    })
}

/// 能力清单：客户端启动时读一次，用来决定 UI 与调用面。
pub fn capabilities() -> Value {
    json!({
        "contract_version": CONTRACT_VERSION,
        "library": {
            "name": "resolver",
            "version": env!("CARGO_PKG_VERSION"),
            "kind": "answer_resolver",
            "targets": ["android_arm64", "linux_x86_64", "windows", "wasm"],
        },
        "ops": [
            {"op": "capabilities", "status": "stable", "summary": "能力清单（本接口）", "payload": {}},
            {"op": "parse_task", "status": "stable",
             "summary": "解析单份资源包（content/info/content2）",
             "payload": {"content": "object|string", "info": "object|string?", "content2": "object|string?"},
             "result": "question[]", "meta": ["layer", "family", "warnings"]},
            {"op": "parse_batch", "status": "stable",
             "summary": "批量解析并合并分节",
             "payload": {"tasks": "[{content, info?, content2?, order?}]"},
             "result": "{sections: [...]}"},
            {"op": "parse_word_pk", "status": "stable",
             "summary": "词汇 / 单词 PK 作业解析",
             "payload": {"data": "object|string"}, "result": "question[]"},
            {"op": "extract_meta", "status": "stable",
             "summary": "轻量元信息（stid / structure_type / 题数）",
             "payload": {"content": "object|string"}, "result": "{stid, structure_type, question_count}"},
            {"op": "diagnose", "status": "stable",
             "summary": "解析诊断：命中层、家族、形状指纹、告警（不含题目文本）",
             "payload": {"content": "object|string", "info": "?", "content2": "?"},
             "result": "{structure_type, layer, family, question_count, shape, warnings, preview}"},
            {"op": "shape_fingerprint", "status": "stable",
             "summary": "结构指纹（用于发现官方新增题型）",
             "payload": {"content": "object|string", "info": "?", "content2": "?"},
             "result": "{hash, path_count, paths[]}"},
            {"op": "classify_homework", "status": "stable",
             "summary": "作业分类（词汇PK/模拟套卷/习题集/作文/在线/常规）+ 卷面满分口径",
             "payload": {"homework": "object|string"}, "result": "classification"},
            {"op": "classify_homework_list", "status": "stable",
             "summary": "作业列表批量分类",
             "payload": {"homeworks": "array|object|string"}, "result": "{category_counts, exercise_set_ids, homeworks[]}"},
            {"op": "spec", "status": "stable",
             "summary": "官方权威规格：题型全集、词汇双通道、展示要素、res.json 权威路径、边界规则、覆盖率",
             "payload": {}, "result": "taxonomy"},
            {"op": "res_template", "status": "stable",
             "summary": "规范化官方 res.json：大题顺序、大题分、逐题满分（E5/E6 的权威来源）",
             "payload": {"res": "object|string"}, "result": "{sections[], section_order[], total_score, ...}"},
            {"op": "apply_res_template", "status": "stable",
             "summary": "把 res.json 的满分与题号盖到已解析题目上（只补不改内容）",
             "payload": {"res": "object|string", "section_index": "int", "questions": "question[]"},
             "result": "{applied, filled_no, filled_score, ...}"},
            {"op": "parse_homework_set", "status": "stable",
             "summary": "老师布置的一整套多卷（习题集）：把多包解析结果合并成一张卷（官方 res.json 优先定顺序与满分）",
             "payload": {"packages": "[{structure_type, group_name, real_id?, order?, questions[]}]",
                         "res": "object|string?（官方 res.json，可选但强烈建议）",
                         "homework": "object|string?（作业 item，用于分类与标题）"},
             "result": "{paper: {sections[], totalScore, totalScore_source, category, is_exercise_set}, coverage, warnings}"},
        ],
        "layers": {
            "explicit": "structure_type 显式命中",
            "generic": "结构嗅探命中（形状熟悉）",
            "salvage": "L3 兜底扫描：名字与形状都陌生时抢救出的结果，每题带 confidence=low 与 \"salvage_path\"，不得当作与官方一致",
            "none": "未识别且抢救不到 —— 需要新增适配（应上报形状指纹）",
        },
        "categories": [
            homework::CATEGORY_VOCAB_PK, homework::CATEGORY_MOCK_SUITE, homework::CATEGORY_PAPER_SET,
            homework::CATEGORY_COMPOSITION, homework::CATEGORY_ONLINE, homework::CATEGORY_ORAL,
        ],
        "question_types": {
            "oral": ["collector.choose", "collector.read", "collector.role", "collector.picture",
                     "collector.fill", "collector.3q5a", "collector.dialogue", "collector.repeat_essay",
                     "collector.word", "collector.repeat", "collector.repeat_dialogue", "collector.essay"],
            "standards": ["listen_choice_standard", "listen_fill_standard", "read_passage_standard",
                          "read_sentence_standard", "read_word_standard", "read_and_choose_standard",
                          "templet_common", "collector.sentence"],
            "vocabulary": ["word.choice", "word.write", "word.read", "word.dictation",
                           "word.translate_en_cn", "word.translate_cn_en"],
            "vocabulary_symbol_channel": "word.symbol.<code>（新版超能单词 QuestionSymbol 0~13）",
            "unknown_fallback": "collector.unknown",
            "authoritative_source": "spec::STRUCTURE_TYPES（客户端反编译）",
            "how_to_get_full_spec": "{\"op\":\"spec\"}",
        },
        "coverage": spec::coverage(),
        "error_codes": [
            {"code": "invalid_input", "meaning": "字段缺失或形态不对"},
            {"code": "unsupported_version", "meaning": "请求的 contract version 高于当前库"},
            {"code": "unknown_op", "meaning": "未知 op（先调 capabilities 取清单）"},
            {"code": "not_implemented", "meaning": "op 已规划但当前版本未实现"},
            {"code": "internal_error", "meaning": "内部异常（不应出现，出现即 bug）"},
        ],
        "notes": [
            "所有 JSON 字段都接受对象或 JSON 文本字符串。",
            "parse_task 的 meta.layer 为 'none' 时应上报 meta.shape.hash，用于发现官方新题型。",
            "答案文本已完成 HTML / 零宽字符清洗，客户端直接展示即可，不要二次清洗。",
        ],
    })
}

/// 统一入口：一次 JSON 进、一次 JSON 出。
// ============================================================================
// 契约入口的授权门禁
//
// 背景：libcloud 的所有数据接口都走 `cloud_authorized_response`（强制 cloud_mode
// 授权），旧接口 `parseTaskAuthorized/parseBatchAuthorized` 也强制 answer_parser
// 授权；**但契约入口（客户端现在走的 resolve_request）此前完全没有授权检查** ——
// 只要调用 `{"op":"parse_task",...}` 就能拿到答案，等于把"库内部强制联网校验、
// 补丁掉 Java 层也没用"这条设计目标架空了。
//
// 因此对**产出题目/答案**的 op 补上授权门禁：
//   parse_task / parse_batch / parse_word_pk / parse_homework_set
// 不门禁的（保持免费，避免影响遥测与服务端消费者）：
//   capabilities / spec / diagnose / shape_fingerprint / extract_meta /
//   classify_homework(_list) / res_template / apply_res_template
//
// **只在 Android 目标默认开启**：
//   宿主 Linux 产物由后端（Python `call_op("parse_task")`）、Fe PC、CI 对拍使用，
//   它们没有客户端授权上下文，强开会把后端自己拦死。
//   （安全边界如实说明：公开发布里同时含宿主 .so 与 wasm，二者不带门禁，
//     所以该门禁只能约束 Android 客户端，不能保护算法本身。）
thread_local! {
    static ENFORCE_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// 契约解析类 op 是否强制 answer_parser 授权。Android 默认 true，其余目标 false。
pub fn enforce_answer_parser_auth() -> bool {
    ENFORCE_OVERRIDE
        .with(|c| c.get())
        .unwrap_or(cfg!(target_os = "android"))
}

/// 测试/宿主场景临时覆盖门禁开关（仅影响当前线程，避免并行测试互相干扰）。
pub fn override_enforce_answer_parser_auth(value: Option<bool>) {
    ENFORCE_OVERRIDE.with(|c| c.set(value));
}

/// 是否为需要授权的 op。
fn op_requires_auth(op: &str) -> bool {
    matches!(
        op,
        "parse_task" | "parse_batch" | "parse_word_pk" | "parse_homework_set"
    )
}

/// 执行门禁：通过返回 None，被拒返回已构造好的错误响应。
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
fn auth_gate(op: &str) -> Option<Value> {
    if !enforce_answer_parser_auth() || !op_requires_auth(op) {
        return None;
    }
    match crate::auth::authorize_for_feature(crate::auth::ANSWER_PARSER_FEATURE) {
        Ok(_grant) => None,
        Err(e) => Some(respond_err(
            op,
            error(
                &e.code,
                &e.message,
                "该 op 需要有效的 answer_parser 授权：请先配置授权地址/令牌并完成授权",
            ),
        )),
    }
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
fn auth_gate(_op: &str) -> Option<Value> {
    None
}

pub fn handle(request: &Value) -> Value {
    let op = request
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let op_ref = op.as_str();

    if op.is_empty() {
        return respond_err(
            "",
            error(
                "invalid_input",
                "缺少 `op`",
                "示例: {\"version\":1,\"op\":\"capabilities\"}",
            ),
        );
    }

    match request.get("version").and_then(|v| v.as_u64()) {
        Some(v) if v > CONTRACT_VERSION => {
            return respond_err(
                op_ref,
                error(
                    "unsupported_version",
                    &format!("请求契约版本 {v} 高于库支持的 {CONTRACT_VERSION}"),
                    "升级解析器，或降级 version 后重试",
                ),
            );
        }
        _ => {}
    }

    let payload = request
        .get("payload")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));

    if let Some(denied) = auth_gate(op_ref) {
        return denied;
    }

    match op_ref {
        "capabilities" => respond_ok(op_ref, capabilities(), None),

        "parse_task" | "diagnose" => {
            let content = match required(request, &payload, "content") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let info = field(request, &payload, "info");
            let content2 = field(request, &payload, "content2");
            let routed = parse_content_routed(&content, content2.as_ref(), info.as_ref());
            let structure_type = content
                .get("structure_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let spec_known = spec::find_structure_type(&structure_type).is_some();
            let meta = json!({
                "layer": routed.layer,
                "family": routed.family,
                "question_count": routed.questions.len(),
                "shape": diagnose::shape_report(&content, content2.as_ref(), info.as_ref())["hash"],
                // 是否在官方权威题型表内（false = 可能是官方新题型，请上报 shape）
                "spec_known": spec_known,
                "structure_type": structure_type,
            });
            if op_ref == "diagnose" {
                let report = diagnose::diagnose(&content, content2.as_ref(), info.as_ref());
                return respond_ok(op_ref, report, Some(meta));
            }
            respond_ok(op_ref, Value::Array(routed.questions), Some(meta))
        }

        "shape_fingerprint" => {
            let content = match required(request, &payload, "content") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let info = field(request, &payload, "info");
            let content2 = field(request, &payload, "content2");
            respond_ok(
                op_ref,
                diagnose::shape_report(&content, content2.as_ref(), info.as_ref()),
                None,
            )
        }

        "parse_word_pk" => {
            let data = match required(request, &payload, "data") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let questions = crate::collectors::collect_word_pk(&data);
            respond_ok(
                op_ref,
                Value::Array(questions.clone()),
                Some(json!({"family": "word_pk", "question_count": questions.len()})),
            )
        }

        "extract_meta" => {
            let content = match required(request, &payload, "content") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            respond_ok(op_ref, api::extract_meta(&content), None)
        }

        "parse_batch" => {
            let tasks = match required(request, &payload, "tasks") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let tasks_arr = match tasks.as_array() {
                Some(arr) => arr.clone(),
                None => {
                    return respond_err(
                        op_ref,
                        error("invalid_input", "`tasks` 必须是数组", "格式: [{content, info?, content2?}]"),
                    )
                }
            };
            respond_ok(
                op_ref,
                api::parse_batch(&tasks_arr, true),
                Some(json!({"task_count": tasks_arr.len()})),
            )
        }

        "classify_homework" => {
            let hw = match required(request, &payload, "homework") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            respond_ok(op_ref, homework::classify_homework(&hw), None)
        }

        "classify_homework_list" => {
            let list = match required(request, &payload, "homeworks") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            respond_ok(op_ref, homework::classify_homework_list(&list), None)
        }

        "parse_homework_set" => {
            let packages = match required(request, &payload, "packages") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let res = field(request, &payload, "res");
            let homework = field(request, &payload, "homework");
            let merged = homework_set::merge(&packages, res.as_ref(), homework.as_ref());
            let meta = json!({
                "packages": merged["coverage"]["packages"],
                "merged": merged["coverage"]["merged"],
                "ordered_by": merged["coverage"]["ordered_by"],
                "warning_count": merged["warnings"].as_array().map(|a| a.len()).unwrap_or(0),
            });
            respond_ok(op_ref, merged, Some(meta))
        }

        "spec" => respond_ok(op_ref, spec::taxonomy(), None),

        "res_template" => {
            let res = match required(request, &payload, "res") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let parsed = template::parse(&res);
            let meta = json!({
                "section_count": parsed["sections"].as_array().map(|a| a.len()).unwrap_or(0),
                "total_score": parsed["total_score"],
                "total_score_source": parsed["total_score_source"],
            });
            respond_ok(op_ref, parsed, Some(meta))
        }

        "apply_res_template" => {
            let res = match required(request, &payload, "res") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            let parsed = template::parse(&res);
            let mut questions = match required(request, &payload, "questions") {
                Ok(v) => v,
                Err(e) => return respond_err(op_ref, e),
            };
            // 分节定位：显式 section_index 优先；否则按 structure_type 匹配 exam_type_collect；
            // 都没有则用第 0 节（并在 meta 里如实回报用了哪一节）。
            let section_index = match field(request, &payload, "section_index").as_ref().and_then(as_index) {
                Some(v) => v,
                None => match field(request, &payload, "structure_type")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .and_then(|st| template::section_for_collect(&parsed, &st))
                    .and_then(|s| s.get("index").and_then(as_index))
                {
                    Some(v) => v,
                    None => 0,
                },
            };
            let report = match questions.as_array_mut() {
                Some(arr) => template::apply_section(&parsed, section_index, arr),
                None => {
                    return respond_err(
                        op_ref,
                        error(
                            "invalid_input",
                            "`questions` 必须是数组",
                            "直接传 parse_task 的 result",
                        ),
                    )
                }
            };
            let applied = report["applied"].clone();
            let meta = json!({
                "applied": applied,
                "section_index": report["section_index"],
                "exam_type_name": report["exam_type_name"],
                "filled_no": report["filled_no"],
                "filled_score": report["filled_score"],
            });
            respond_ok(op_ref, questions, Some(meta))
        }

        other => respond_err(
            other,
            error(
                "unknown_op",
                &format!("未知 op: {other}"),
                "先调用 {\"op\":\"capabilities\"} 获取支持的 op 清单",
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_lists_contract_and_ops() {
        let caps = capabilities();
        assert_eq!(caps["contract_version"], CONTRACT_VERSION);
        let ops: Vec<&str> = caps["ops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["op"].as_str().unwrap())
            .collect();
        assert!(ops.contains(&"parse_task"));
        assert!(ops.contains(&"diagnose"));
        assert!(ops.contains(&"classify_homework"));
        assert!(ops.contains(&"parse_homework_set"));
    }

    #[test]
    fn handle_dispatches_without_payload_wrapper() {
        // 字段直接放在请求根上也能用（宽容解析）
        let req = json!({
            "op": "parse_task",
            "content": {"structure_type": "collector.role",
                        "info": {"stid": "1", "question": [{"ask": "Q", "std": [{"value": "A"}]}]}}
        });
        let out = handle(&req);
        assert_eq!(out["ok"], true);
        assert_eq!(out["result"][0]["answers"][0], "A");
        assert_eq!(out["meta"]["layer"], "explicit");
    }

    #[test]
    fn handle_accepts_json_string_content() {
        let req = json!({
            "version": 1,
            "op": "parse_task",
            "payload": {
                "content": "{\"structure_type\":\"collector.fill\",\"info\":{\"stid\":\"7\",\"std\":[{\"xth\":\"15\",\"value\":\"v1\"}]}}"
            }
        });
        let out = handle(&req);
        assert_eq!(out["ok"], true);
        assert_eq!(out["result"][0]["answers"][0], "v1");
    }

    #[test]
    fn handle_unknown_op_guides_to_capabilities() {
        let out = handle(&json!({"op": "no_such_op"}));
        assert_eq!(out["ok"], false);
        assert_eq!(out["error"]["code"], "unknown_op");
        assert!(out["error"]["hint"].as_str().unwrap().contains("capabilities"));
    }

    #[test]
    fn handle_rejects_future_version() {
        let out = handle(&json!({"version": 99, "op": "capabilities"}));
        assert_eq!(out["ok"], false);
        assert_eq!(out["error"]["code"], "unsupported_version");
    }

    #[test]
    fn handle_reports_missing_field_precisely() {        let out = handle(&json!({"op": "parse_task"}));
        assert_eq!(out["ok"], false);
        assert_eq!(out["error"]["code"], "invalid_input");
        assert!(out["error"]["message"].as_str().unwrap().contains("content"));
    }

    #[test]
    fn handle_merges_homework_set_through_contract() {
        let res = json!({
            "exam_type_list": [
                {"exam_type_name": "模仿朗读", "exam_type_collect": "collector.read",
                 "exam_list": [{"exam_score": "6.0",
                                "question_list": [{"question_order": "0", "question_score": "6.0"}]}]},
                {"exam_type_name": "听选信息", "exam_type_collect": "listen_choice_standard",
                 "exam_list": [{"exam_score": "3.0", "question_list": [
                    {"question_order": "0", "question_score": "1.5"},
                    {"question_order": "1", "question_score": "1.5"}]}]}
            ]
        });
        let out = handle(&json!({
            "op": "parse_homework_set",
            "payload": {
                "homework": {"id": "5", "name": "Unit 3 综合练习", "set_id": "s-1",
                             "contents": [{"group_name": "模仿朗读"}, {"group_name": "听选信息"}]},
                "res": res,
                "packages": [
                    {"structure_type": "listen_choice_standard", "group_name": "听选信息", "order": 1,
                     "questions": [{"no": 1, "question": "Q1"}, {"no": 2, "question": "Q2"}]},
                    {"structure_type": "collector.read", "group_name": "模仿朗读", "order": 0,
                     "questions": [{"no": 1, "question": "Read"}]}
                ]
            }
        }));
        assert_eq!(out["ok"], true);
        let paper = &out["result"]["paper"];
        assert_eq!(paper["section_count"], 2);
        assert_eq!(paper["sections"][0]["caption"], "模仿朗读");
        assert_eq!(paper["question_count"], 3);
        assert_eq!(paper["totalScore"], 9.0);
        assert_eq!(paper["totalScore_source"], "res_template");
        assert_eq!(paper["is_exercise_set"], true);
        assert_eq!(out["meta"]["ordered_by"], "res_template");
        assert_eq!(out["result"]["coverage"]["merged"], 2);
    }

    #[test]
    fn handle_classifies_homework_through_contract() {
        let req = json!({
            "op": "classify_homework",
            "payload": {"homework": {
                "id": "5", "name": "Unit 3 综合练习", "set_id": "s-1",
                "contents": [{"group_name": "模仿朗读"}, {"group_name": "信息转述"}]
            }}
        });
        let out = handle(&req);
        assert_eq!(out["ok"], true);
        assert_eq!(out["result"]["is_exercise_set"], true);
        assert_eq!(out["result"]["category"], homework::CATEGORY_PAPER_SET);
    }

    #[test]
    fn handle_diagnose_does_not_leak_text() {
        let req = json!({
            "op": "diagnose",
            "payload": {"content": {
                "structure_type": "collector.role",
                "info": {"stid": "1", "question": [{"ask": "SECRET_Q", "std": [{"value": "SECRET_A"}]}]}
            }}
        });
        let text = handle(&req).to_string();
        assert!(!text.contains("SECRET_Q"));
        assert!(!text.contains("SECRET_A"));
    }

    #[test]
    fn handle_batch_and_meta_shape() {
        let req = json!({
            "op": "parse_batch",
            "payload": {"tasks": [
                {"content": {"structure_type": "collector.role",
                             "info": {"stid": "1", "question": [{"ask": "Q", "std": [{"value": "A"}]}]}}}
            ]}
        });
        let out = handle(&req);
        assert_eq!(out["ok"], true);
        assert!(out["result"]["sections"].is_array());
    }

    #[test]
    fn meta_reports_spec_known_for_official_and_unknown_types() {
        let known = json!({
            "op": "parse_task",
            "payload": {"content": {"structure_type": "read_word_standard", "info": {"stid": "1", "value": "hello"}}}
        });
        let out = handle(&known);
        assert_eq!(out["meta"]["spec_known"], true);
        assert_eq!(out["meta"]["family"], "collector.read");

        let unknown = json!({
            "op": "parse_task",
            "payload": {"content": {"structure_type": "collector.brand_new_2027",
                                    "info": {"stid": "1", "value": "hello"}}}
        });
        let out = handle(&unknown);
        assert_eq!(out["meta"]["spec_known"], false);
    }

    #[test]
    fn official_standard_identifiers_route_to_expected_families() {
        let cases = [
            ("listen_choice_standard", "collector.choose"),
            ("read_and_choose_standard", "collector.choose"),
            ("listen_fill_standard", "collector.fill"),
            ("read_passage_standard", "collector.read"),
            ("read_sentence_standard", "collector.read"),
            ("collector.sentence", "collector.read"),
        ];
        for (st, family) in cases {
            let req = json!({
                "op": "diagnose",
                "payload": {"content": {"structure_type": st, "info": {"stid": "1"}}}
            });
            let out = handle(&req);
            assert_eq!(out["ok"], true, "{st} 应被受理");
            assert_eq!(out["result"]["family"], family, "{st} 应走 {family}");
            assert_eq!(out["result"]["layer"], "explicit", "{st} 不应落到嗅探层");
        }
    }

    #[test]
    fn spec_op_returns_authoritative_taxonomy_and_coverage() {
        let out = handle(&json!({"op": "spec"}));
        assert_eq!(out["ok"], true);
        let types = out["result"]["structure_types"].as_array().unwrap();
        assert!(types.iter().any(|t| t["id"] == "listen_choice_standard"));
        let symbols = out["result"]["word_symbols"].as_array().unwrap();
        assert!(symbols.iter().any(|s| s["question_type"] == 13));
        let coverage = &out["result"]["coverage"];
        assert_eq!(coverage["total"], types.len());
        assert!(coverage["explicitly_adapted"].as_u64().unwrap() >= types.len() as u64 - 1);
    }

    #[test]
    fn res_template_op_normalizes_and_applies_scores() {
        let res = json!({
            "exam_type_list": [{
                "exam_type_name": "听选信息",
                "exam_type_collect": "listen_choice_standard",
                "exam_list": [{
                    "exam_score": "3.0",
                    "question_list": [
                        {"question_order": "0", "question_score": "1.5"},
                        {"question_order": "1", "question_score": "1.5"}
                    ]
                }]
            }]
        });
        let parsed = handle(&json!({"op": "res_template", "payload": {"res": res}}));
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["result"]["total_score"], 3.0);
        assert_eq!(parsed["meta"]["section_count"], 1);

        // 不传 section_index，靠 structure_type 自动定位分节
        let applied = handle(&json!({
            "op": "apply_res_template",
            "payload": {
                "res": res,
                "structure_type": "listen_choice_standard",
                "questions": [{"question": "Q1"}, {"question": "Q2"}]
            }
        }));
        assert_eq!(applied["ok"], true);
        assert_eq!(applied["meta"]["applied"], true);
        assert_eq!(applied["result"][0]["score"], 1.5);
        assert_eq!(applied["result"][1]["score"], 1.5);
        assert!(applied["result"][0]["no"].is_i64());
    }

    #[test]
    fn apply_res_template_locates_section_by_structure_type_string() {
        // 两节模板 + 纯字符串 structure_type：必须命中第 1 节，而不是退回第 0 节
        let res = json!({
            "exam_type_list": [
                {"exam_type_name": "模仿朗读", "exam_type_collect": "collector.read",
                 "exam_list": [{"exam_score": "6.0",
                                "question_list": [{"question_order": "0", "question_score": "6.0"}]}]},
                {"exam_type_name": "听选信息", "exam_type_collect": "listen_choice_standard",
                 "exam_list": [{"exam_score": "3.0",
                                "question_list": [{"question_order": "0", "question_score": "1.5"},
                                                  {"question_order": "1", "question_score": "1.5"}]}]}
            ]
        });
        let out = handle(&json!({
            "op": "apply_res_template",
            "payload": {
                "res": res,
                "structure_type": "listen_choice_standard",
                "questions": [{"question": "Q1"}, {"question": "Q2"}]
            }
        }));
        assert_eq!(out["meta"]["applied"], true);
        assert_eq!(out["meta"]["section_index"], 1);
        assert_eq!(out["meta"]["exam_type_name"], "听选信息");
        assert_eq!(out["result"][0]["score"], 1.5);
        assert_eq!(out["result"][1]["score"], 1.5);
        assert_eq!(out["meta"]["filled_no"], 2);
    }
}

#[cfg(test)]
mod auth_gate_tests {
    use super::*;
    use serde_json::json;

    fn parse_req() -> Value {
        json!({"version": 1, "op": "parse_task",
               "payload": {"content": {"structure_type": "collector.fill",
                                       "info": {"std": [{"xth": "1", "value": "A"}]}}}})
    }

    #[test]
    fn contract_parse_ops_are_gated_when_enforced() {
        override_enforce_answer_parser_auth(Some(true));
        // 未配置授权（无 base_url/token）→ 必须被拒，而不是照常出答案
        let resp = handle(&parse_req());
        assert_eq!(resp["ok"], json!(false), "开启门禁后解析必须失败: {resp}");
        let code = resp["error"]["code"].as_str().unwrap_or("");
        assert!(
            code.starts_with("auth_") || code == "auth_disabled" || code == "network_error",
            "应返回授权类错误码，实际 {code}: {resp}"
        );

        // 四个"出答案"的 op 都要门禁（用各自最小入参）
        for op in ["parse_task", "parse_batch", "parse_word_pk", "parse_homework_set"] {
            let req = json!({"version": 1, "op": op, "payload": {}});
            let r = handle(&req);
            assert_eq!(r["ok"], json!(false), "{op} 应被门禁拦下");
            let c = r["error"]["code"].as_str().unwrap_or("");
            assert!(!c.is_empty() && c != "invalid_input", "{op} 应是授权错误而非参数错误: {r}");
        }
        override_enforce_answer_parser_auth(None);
    }

    #[test]
    fn diagnostic_and_metadata_ops_stay_free() {
        override_enforce_answer_parser_auth(Some(true));
        // 遥测/元数据类 op 不门禁：授权被拒时也要能上报结构指纹，否则永远发现不了新题型
        let diag = handle(&json!({"version": 1, "op": "diagnose",
            "payload": {"content": {"structure_type": "collector.fill",
                                    "info": {"std": [{"xth": "1", "value": "A"}]}}}}));
        assert_eq!(diag["ok"], json!(true), "diagnose 不应被门禁拦: {diag}");

        let caps = handle(&json!({"version": 1, "op": "capabilities"}));
        assert_eq!(caps["ok"], json!(true), "capabilities 不应被门禁拦");

        let cls = handle(&json!({"version": 1, "op": "classify_homework_list",
            "payload": {"homeworks": []}}));
        assert_eq!(cls["ok"], json!(true), "classify_homework_list 不应被门禁拦: {cls}");
        override_enforce_answer_parser_auth(None);
    }

    #[test]
    fn gate_is_off_by_default_on_host_targets() {
        // 宿主（后端 Python / Fe PC / CI 对拍）默认不门禁，否则会把服务端消费者拦死
        assert_eq!(
            enforce_answer_parser_auth(),
            cfg!(target_os = "android"),
            "宿主目标应默认关闭契约授权门禁"
        );
        override_enforce_answer_parser_auth(Some(false));
        let resp = handle(&parse_req());
        assert_eq!(resp["ok"], json!(true), "关闭门禁时解析应正常: {resp}");
        override_enforce_answer_parser_auth(None);
    }
}
