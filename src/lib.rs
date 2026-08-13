// -*- coding: utf-8 -*-
// ETS 作业答案通用解析器 (Rust cdylib)
//
// 架构 (与 resolver_py 对齐, 纯 JSON 输入输出, 核心零 IO):
//   cleaner.rs   - HTML 清洗 + 题号提取
//   collectors.rs - 7 种题型解析器 + 未知类型兜底
//   api.rs       - parse_task / parse_batch / group_by_time / extract_meta
//
// FFI 约定:
//   输入: 以 '\0' 结尾的 UTF-8 C 字符串 (JSON 文本)
//   输出: 以 '\0' 结尾的 UTF-8 C 字符串 (JSON 文本), 调用方用 resolver_free_string 释放
//   错误: 返回 "null"

mod api;
mod cleaner;
mod collectors;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use serde_json::Value;

// ============== 新接口: 纯 JSON 字符串 ==============

/// 单任务解析: resolve(content_json, info_json, content2_json) -> 题目列表 JSON
/// content2/info 可为 null/空串
#[no_mangle]
pub extern "C" fn parse_task(
    content: *const c_char,
    info: *const c_char,
    content2: *const c_char,
) -> *mut c_char {
    let content = read_str(content);
    let info = read_str(info);
    let content2 = read_str(content2);
    let content_val = match parse_json(&content) {
        Some(v) => v,
        None => return json_to_ptr(Value::Null),
    };
    let info_val = parse_json(&info);
    let content2_val = parse_json(&content2);
    let questions = api::parse_task(
        &content_val,
        content2_val.as_ref(),
        info_val.as_ref(),
    );
    json_to_ptr(Value::Array(questions))
}

/// 批量解析: parse_batch("[{content, info?, content2?, order?}, ...]") -> {sections: [...]}
#[no_mangle]
pub extern "C" fn parse_batch(tasks_json: *const c_char) -> *mut c_char {
    let tasks_json = read_str(tasks_json);
    let tasks = match parse_json(&tasks_json).and_then(|v| v.as_array().cloned()) {
        Some(arr) => arr,
        None => return json_to_ptr(Value::Null),
    };
    json_to_ptr(api::parse_batch(&tasks, true))
}

/// 时间聚类: group_by_time([[id, mtime_ms], ...], threshold_ms) -> [[id, ...], ...]
#[no_mangle]
pub extern "C" fn group_by_time(items_json: *const c_char, threshold_ms: i64) -> *mut c_char {
    let items_json = read_str(items_json);
    let items = match parse_json(&items_json).and_then(|v| v.as_array().cloned()) {
        Some(arr) => arr,
        None => return json_to_ptr(Value::Null),
    };
    json_to_ptr(api::group_by_time(&items, threshold_ms))
}

/// 元信息: extract_meta(content_json) -> {stid, structure_type, question_count}
#[no_mangle]
pub extern "C" fn extract_meta(content: *const c_char) -> *mut c_char {
    let content = read_str(content);
    let content_val = match parse_json(&content) {
        Some(v) => v,
        None => return json_to_ptr(Value::Null),
    };
    json_to_ptr(api::extract_meta(&content_val))
}

// ============== 旧接口: 兼容 demo.py 调用方 ==============

mod legacy;

// ============== 工具 ==============

fn read_str(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => String::new(),
    }
}

fn parse_json(s: &str) -> Option<Value> {
    if s.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(s).ok()
}

fn json_to_ptr(result: Value) -> *mut c_char {
    let json_str = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
    match CString::new(json_str) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 释放由本库返回的字符串
#[no_mangle]
pub extern "C" fn resolver_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
