// -*- coding: utf-8 -*-
// Rust 重写版 demo.py，导出为动态链接库 (cdylib)。
// FFI 约定：
//   输入：以 '\0' 结尾的 UTF-8 C 字符串（路径或 JSON 字符串）
//   输出：以 '\0' 结尾的 UTF-8 C 字符串（JSON 文本），由调用方使用 `resolver_free_string` 释放
//   错误：返回 JSON 字符串 "null"（对应 Python 中的 -1 / 异常）

use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Map, Value};

// ============== 全局标志 ==============

static DEBUG: AtomicBool = AtomicBool::new(false);
static NOFILE_READ: AtomicBool = AtomicBool::new(false);

/// 设置无文件模式
#[no_mangle]
pub extern "C" fn nofile_mode(mode: bool) {
    NOFILE_READ.store(mode, Ordering::SeqCst);
}

/// 设置调试模式
#[no_mangle]
pub extern "C" fn debug_mode(mode: bool) {
    DEBUG.store(mode, Ordering::SeqCst);
}

fn debug_print(s: &str) {
    if DEBUG.load(Ordering::SeqCst) {
        println!("{s}");
    }
}

/// 工具：获取 JSON 字段并转为 String
fn get_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

// ============== HTML 标签剥离 (纯 Rust 标准库实现) ==============
// 行为：移除所有 HTML 标签；跳过 <script>/<style> 内部内容；
//       跳过 HTML 注释 <!-- ... -->；解码常见 HTML 实体。

fn decode_html_entity(entity: &str) -> Option<String> {
    if entity.starts_with("&#") && entity.len() > 3 {
        let inner = &entity[2..entity.len() - 1];
        if let Some(hex_str) = inner.strip_prefix('x').or_else(|| inner.strip_prefix('X')) {
            return u32::from_str_radix(hex_str, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string());
        } else if let Ok(code) = inner.parse::<u32>() {
            return char::from_u32(code).map(|c| c.to_string());
        }
        return None;
    } else if entity.starts_with('&') && entity.ends_with(';') && entity.len() > 2 {
        let name = &entity[1..entity.len() - 1];
        return match name {
            "amp" => Some("&".into()),
            "lt" => Some("<".into()),
            "gt" => Some(">".into()),
            "quot" => Some("\"".into()),
            "apos" => Some("'".into()),
            "nbsp" => Some("\u{00A0}".into()),
            "copy" => Some("\u{00A9}".into()),
            "reg" => Some("\u{00AE}".into()),
            "trade" => Some("\u{2122}".into()),
            "hellip" => Some("\u{2026}".into()),
            "mdash" => Some("\u{2014}".into()),
            "ndash" => Some("\u{2013}".into()),
            "ldquo" => Some("\u{201C}".into()),
            "rdquo" => Some("\u{201D}".into()),
            "lsquo" => Some("\u{2018}".into()),
            "rsquo" => Some("\u{2019}".into()),
            _ => None,
        };
    }
    None
}

fn remove_html_tags(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut result = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < n {
        let c = chars[i];

        if c == '<' {
            let mut j = i + 1;
            let mut is_end = false;
            if j < n && chars[j] == '/' {
                is_end = true;
                j += 1;
            }

            if j < n && chars[j] == '!' {
                if j + 2 < n && chars[j + 1] == '-' && chars[j + 2] == '-' {
                    let mut k = j + 3;
                    let mut end = n;
                    while k + 2 < n {
                        if chars[k] == '-' && chars[k + 1] == '-' && chars[k + 2] == '>' {
                            end = k + 3;
                            break;
                        }
                        k += 1;
                    }
                    i = end;
                    continue;
                } else {
                    while j < n && chars[j] != '>' {
                        j += 1;
                    }
                    if j < n {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }

            while j < n && chars[j].is_whitespace() {
                j += 1;
            }
            let tag_start = j;
            while j < n && chars[j].is_alphanumeric() {
                j += 1;
            }
            let tag_name: String = chars[tag_start..j].iter().collect::<String>().to_lowercase();

            while j < n && chars[j] != '>' {
                j += 1;
            }
            if j < n {
                j += 1;
            }

            if !is_end && (tag_name == "script" || tag_name == "style") {
                let close_tag_lower = format!("</{tag_name}");
                let mut k = j;
                let mut found = false;
                while k + close_tag_lower.len() <= n {
                    let slice: String = chars[k..k + close_tag_lower.len()].iter().collect();
                    if slice.to_lowercase() == close_tag_lower {
                        let mut m = k + close_tag_lower.len();
                        while m < n && chars[m] != '>' {
                            m += 1;
                        }
                        if m < n {
                            m += 1;
                        }
                        i = m;
                        found = true;
                        break;
                    }
                    k += 1;
                }
                if !found {
                    i = n;
                }
                continue;
            }

            i = j;
        } else if c == '&' {
            let mut j = i + 1;
            let mut found_semi = false;
            while j < n && j - i < 30 {
                if chars[j] == ';' {
                    found_semi = true;
                    break;
                }
                if !chars[j].is_alphanumeric() && chars[j] != '#' {
                    break;
                }
                j += 1;
            }
            if found_semi {
                let entity: String = chars[i..=j].iter().collect();
                if let Some(decoded) = decode_html_entity(&entity) {
                    result.push_str(&decoded);
                } else {
                    result.push_str(&entity);
                }
                i = j + 1;
            } else {
                result.push(c);
                i += 1;
            }
        } else {
            result.push(c);
            i += 1;
        }
    }
    result
}

// ============== 字典排序 ==============

fn sort_dict_by_key(d: Value) -> Value {
    if let Value::Object(map) = d {
        let mut items: Vec<(String, Value)> = map.into_iter().collect();
        items.sort_by(|a, b| {
            let a_parts: Vec<i64> = a.0.split('_').filter_map(|s| s.parse().ok()).collect();
            let b_parts: Vec<i64> = b.0.split('_').filter_map(|s| s.parse().ok()).collect();
            a_parts.cmp(&b_parts)
        });
        let mut sorted = Map::new();
        for (k, v) in items {
            sorted.insert(k, v);
        }
        Value::Object(sorted)
    } else {
        d
    }
}

// ============== Target 枚举 ==============

enum Target {
    Path(PathBuf),
    NoFile { content: String, info: String },
}

// ============== Collector: choose ==============

fn collector_choose(target: &Target) -> Result<Value, String> {
    let info_val = match target {
        Target::Path(p) => {
            let info_path = p.join("info.json");
            if !info_path.exists() {
                return Err("info.json not exists".into());
            }
            let content = fs::read_to_string(&info_path).map_err(|e| e.to_string())?;
            serde_json::from_str::<Value>(&content).map_err(|e| e.to_string())?
        }
        Target::NoFile { info, .. } => {
            serde_json::from_str::<Value>(info).map_err(|e| e.to_string())?
        }
    };

    let arr = info_val.as_array().ok_or("info.json is not an array")?;
    let mut result = Map::new();

    for item in arr {
        let code_id = item.get("code_id").and_then(|v| v.as_str()).unwrap_or("");
        let code_type = item.get("code_type").and_then(|v| v.as_str()).unwrap_or("");
        if code_id == "answer" && code_type == "json_array" {
            let code_json_array_str = item.get("code_json_array").and_then(|v| v.as_str()).unwrap_or("");
            let code_json_array: Value = serde_json::from_str::<Value>(code_json_array_str).map_err(|e| e.to_string())?;
            if let Some(answers) = code_json_array.as_array() {
                for answer in answers {
                    let stid = get_str(answer.get("stid").unwrap_or(&Value::Null));
                    let xth = get_str(answer.get("xth").unwrap_or(&Value::Null));
                    let key = format!("{stid}_{xth}");
                    let answer_text = get_str(answer.get("answer").unwrap_or(&Value::Null));
                    let value = json!({
                        "answer": [remove_html_tags(&answer_text)],
                        "important": []
                    });
                    result.insert(key, value);
                }
            }
        }
    }
    Ok(sort_dict_by_key(Value::Object(result)))
}

// ============== Collector: read_word ==============

fn collector_read_word(target: &Target) -> Result<Value, String> {
    let txt_val = match target {
        Target::Path(p) => {
            let content_path = p.join("content.json");
            if !content_path.exists() {
                return Err("content.json not exists".into());
            }
            let content = fs::read_to_string(&content_path).map_err(|e| e.to_string())?;
            serde_json::from_str::<Value>(&content).map_err(|e| e.to_string())?
        }
        Target::NoFile { content, .. } => {
            serde_json::from_str::<Value>(content).map_err(|e| e.to_string())?
        }
    };

    let info = txt_val.get("info").ok_or("info field missing")?;
    let stid = get_str(info.get("stid").unwrap_or(&Value::Null));
    let key = format!("{stid}_0");
    let value_text = get_str(info.get("value").unwrap_or(&Value::Null));
    let analyze = get_str(info.get("analyze").unwrap_or(&Value::Null));
    let important: Vec<String> = analyze.split("</br>").map(remove_html_tags).collect();

    let mut result = Map::new();
    result.insert(
        key,
        json!({
            "answer": [remove_html_tags(&value_text)],
            "important": important
        }),
    );
    Ok(sort_dict_by_key(Value::Object(result)))
}

fn collector_read(target: &Target) -> Result<Value, String> {
    collector_read_word(target)
}

fn collector_word(target: &Target) -> Result<Value, String> {
    collector_read_word(target)
}

// ============== Collector: repeat_essay_dialogue ==============

fn collector_repeat_essay_dialogue(target: &Target) -> Result<Value, String> {
    let txt_val = match target {
        Target::Path(p) => {
            let content_path = p.join("content.json");
            if !content_path.exists() {
                return Err("content.json not exists".into());
            }
            let content = fs::read_to_string(&content_path).map_err(|e| e.to_string())?;
            serde_json::from_str::<Value>(&content).map_err(|e| e.to_string())?
        }
        Target::NoFile { content, .. } => {
            serde_json::from_str::<Value>(content).map_err(|e| e.to_string())?
        }
    };

    let info = txt_val.get("info").ok_or("info field missing")?;
    let stid = get_str(info.get("stid").unwrap_or(&Value::Null));
    let questions = info.get("question").and_then(|v| v.as_array()).ok_or("question field missing or not array")?;

    let mut result = Map::new();
    for question in questions {
        let xh = get_str(question.get("xh").unwrap_or(&Value::Null));
        let key = format!("{stid}_{xh}");

        let mut part_answer = Vec::new();
        if let Some(stds) = question.get("std").and_then(|v| v.as_array()) {
            for std_item in stds {
                let value = get_str(std_item.get("value").unwrap_or(&Value::Null));
                part_answer.push(remove_html_tags(&value));
            }
        }

        let keywords = get_str(question.get("keywords").unwrap_or(&Value::Null));
        let important: Vec<String> = keywords.split(" | ").map(remove_html_tags).collect();

        result.insert(
            key,
            json!({
                "answer": part_answer,
                "important": important
            }),
        );
    }
    Ok(sort_dict_by_key(Value::Object(result)))
}

fn collector_repeat(target: &Target) -> Result<Value, String> {
    collector_repeat_essay_dialogue(target)
}

fn collector_repeat_essay(target: &Target) -> Result<Value, String> {
    collector_repeat_essay_dialogue(target)
}

fn collector_repeat_dialogue(target: &Target) -> Result<Value, String> {
    collector_repeat_essay_dialogue(target)
}

// ============== Collector: others ==============

fn collector_others(target: &Target) -> Result<Value, String> {
    if let Ok(r) = collector_repeat(target) { return Ok(r); }
    debug_print("Others can't be resolved by repeat, try others");

    if let Ok(r) = collector_choose(target) { return Ok(r); }
    debug_print("Others can't be resolved by choose, try others");

    if let Ok(r) = collector_read_word(target) { return Ok(r); }
    debug_print("Others can't be resolved by read_word, try others");

    if let Ok(r) = collector_repeat_essay_dialogue(target) { return Ok(r); }
    debug_print("Others can't be resolved by essay_dialogue, try others");

    Err("All collectors failed".into())
}

// ============== resolve (核心逻辑) ==============

fn resolve_inner(target: &Target) -> Value {
    let content_val = match target {
        Target::Path(p) => {
            let content_path = p.join("content.json");
            if !content_path.exists() {
                eprintln!("Error: content.json not exists, path: {}", p.display());
                return Value::Null;
            }
            let content_str = match fs::read_to_string(&content_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error:{e}, path:{}", p.display());
                    return Value::Null;
                }
            };
            match serde_json::from_str::<Value>(&content_str) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error:{e}, path:{}", p.display());
                    return Value::Null;
                }
            }
        }
        Target::NoFile { content, .. } => {
            match serde_json::from_str::<Value>(content) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error parsing content json string: {e}");
                    return Value::Null;
                }
            }
        }
    };

    let structure_type = content_val.get("structure_type").and_then(|v| v.as_str()).unwrap_or("");

    let result = if structure_type == "collector.choose" || structure_type.contains("choose") {
        collector_choose(target)
    } else if structure_type == "collector.read" || structure_type.contains("read") {
        collector_read(target)
    } else if structure_type == "collector.word" || structure_type.contains("word") {
        collector_word(target)
    } else if structure_type == "collector.repeat" || structure_type.contains("repeat") {
        collector_repeat(target)
    } else if structure_type == "collector.repeat_essay" || structure_type.contains("repeat_essay") {
        collector_repeat_essay(target)
    } else if structure_type == "collector.repeat_dialogue" || structure_type.contains("repeat_dialogue") {
        collector_repeat_dialogue(target)
    } else if structure_type == "collector.essay" || structure_type.contains("essay") {
        collector_repeat_essay(target)
    } else if structure_type == "collector.dialogue" || structure_type.contains("dialogue") {
        collector_repeat_dialogue(target)
    } else {
        debug_print(&format!("Unknown structure_type:{structure_type}, but try to resolve it as others"));
        collector_others(target)
    };

    match result {
        Ok(mut v) => {
            if let Value::Object(ref mut map) = v {
                map.insert("type".to_string(), Value::String(structure_type.to_string()));
            }
            v
        }
        Err(e) => {
            eprintln!("Error:{e}");
            Value::Null
        }
    }
}

// ============== resolve_by_homework (单作业) ==============

fn walkdir_has_content(path: &Path) -> bool {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if p.join("content.json").exists() {
                return true;
            }
            if walkdir_has_content(&p) {
                return true;
            }
        } else if p.file_name().and_then(|n| n.to_str()) == Some("content.json") {
            return true;
        }
    }
    false
}

fn resolve_by_homework_path(target_path: &Path) -> Value {
    let mut answers: Vec<Value> = Vec::new();
    let entries = match fs::read_dir(target_path) {
        Ok(e) => e,
        Err(_) => return Value::Array(answers),
    };

    for entry in entries.flatten() {
        let item_path = entry.path();
        if !item_path.is_dir() {
            continue;
        }
        let found = if item_path.join("content.json").exists() {
            true
        } else {
            walkdir_has_content(&item_path)
        };
        if !found {
            continue;
        }
        answers.push(resolve_inner(&Target::Path(item_path)));
    }
    Value::Array(answers)
}

// ============== resolve_by_all_homework (所有作业) ==============

fn resolve_by_all_homework_path(target_path: &Path) -> Value {
    let mut answers = Map::new();
    let entries = match fs::read_dir(target_path) {
        Ok(e) => e,
        Err(_) => return Value::Object(answers),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        answers.insert(name, resolve_by_homework_path(&path));
    }
    Value::Object(answers)
}

// ============== FFI 导出函数 ==============

#[no_mangle]
pub extern "C" fn resolve(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let s = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = if NOFILE_READ.load(Ordering::SeqCst) {
        match serde_json::from_str::<Vec<String>>(s) {
            Ok(pair) if pair.len() >= 2 => resolve_inner(&Target::NoFile {
                content: pair[0].clone(),
                info: pair[1].clone(),
            }),
            _ => Value::Null,
        }
    } else {
        resolve_inner(&Target::Path(PathBuf::from(s)))
    };

    json_to_ptr(result)
}

#[no_mangle]
pub extern "C" fn resolve_by_homework(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let s = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = if NOFILE_READ.load(Ordering::SeqCst) {
        match serde_json::from_str::<Vec<Vec<String>>>(s) {
            Ok(list_of_pairs) => {
                let mut arr = Vec::new();
                for pair in list_of_pairs {
                    if pair.len() >= 2 {
                        arr.push(resolve_inner(&Target::NoFile {
                            content: pair[0].clone(),
                            info: pair[1].clone(),
                        }));
                    }
                }
                Value::Array(arr)
            }
            _ => Value::Array(vec![]),
        }
    } else {
        resolve_by_homework_path(Path::new(s))
    };

    json_to_ptr(result)
}

#[no_mangle]
pub extern "C" fn resolve_by_all_homework(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let s = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = if NOFILE_READ.load(Ordering::SeqCst) {
        match serde_json::from_str::<Vec<Vec<Vec<String>>>>(s) {
            Ok(list_of_list_of_pairs) => {
                let mut map = Map::new();
                for (i, list_of_pairs) in list_of_list_of_pairs.iter().enumerate() {
                    let mut arr = Vec::new();
                    for pair in list_of_pairs {
                        if pair.len() >= 2 {
                            arr.push(resolve_inner(&Target::NoFile {
                                content: pair[0].clone(),
                                info: pair[1].clone(),
                            }));
                        }
                    }
                    map.insert(i.to_string(), Value::Array(arr));
                }
                Value::Object(map)
            }
            _ => Value::Object(Map::new()),
        }
    } else {
        resolve_by_all_homework_path(Path::new(s))
    };

    json_to_ptr(result)
}

fn json_to_ptr(result: Value) -> *mut c_char {
    let json_str = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string());
    match CString::new(json_str) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 释放由 resolve 等函数返回的字符串
#[no_mangle]
pub extern "C" fn resolver_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
