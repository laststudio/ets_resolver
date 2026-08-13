// -*- coding: utf-8 -*-
// 旧接口兼容层: 保留原 demo.py 时代 FFI (resolve / resolve_by_homework /
// resolve_by_all_homework / nofile_mode / debug_mode), 行为与旧版一致,
// 内部复用新版 cleaner 模块。

use std::ffi::CString;
use std::fs;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Map, Value};

use crate::cleaner::strip_html;

static DEBUG: AtomicBool = AtomicBool::new(false);
static NOFILE_READ: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn nofile_mode(mode: bool) {
    NOFILE_READ.store(mode, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn debug_mode(mode: bool) {
    DEBUG.store(mode, Ordering::SeqCst);
}

fn debug_print(s: &str) {
    if DEBUG.load(Ordering::SeqCst) {
        println!("{s}");
    }
}

fn get_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

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

enum Target {
    Path(PathBuf),
    NoFile { content: String, info: String },
}

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
            let code_json_array_str = item
                .get("code_json_array")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let code_json_array: Value = serde_json::from_str::<Value>(code_json_array_str)
                .map_err(|e| e.to_string())?;
            if let Some(answers) = code_json_array.as_array() {
                for answer in answers {
                    let stid = get_str(answer.get("stid").unwrap_or(&Value::Null));
                    let xth = get_str(answer.get("xth").unwrap_or(&Value::Null));
                    let key = format!("{stid}_{xth}");
                    let answer_text = get_str(answer.get("answer").unwrap_or(&Value::Null));
                    let value = json!({
                        "answer": [strip_html(&answer_text)],
                        "important": []
                    });
                    result.insert(key, value);
                }
            }
        }
    }
    Ok(sort_dict_by_key(Value::Object(result)))
}

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
    let important: Vec<String> = analyze
        .split("</br>")
        .map(|s| strip_html(s).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut result = Map::new();
    result.insert(
        key,
        json!({
            "answer": [strip_html(&value_text)],
            "important": important
        }),
    );
    Ok(sort_dict_by_key(Value::Object(result)))
}

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
    let questions = info
        .get("question")
        .and_then(|v| v.as_array())
        .ok_or("question field missing or not array")?;

    let mut result = Map::new();
    for question in questions {
        let xh = get_str(question.get("xh").unwrap_or(&Value::Null));
        let key = format!("{stid}_{xh}");

        let mut part_answer = Vec::new();
        if let Some(stds) = question.get("std").and_then(|v| v.as_array()) {
            for std_item in stds {
                let value = get_str(std_item.get("value").unwrap_or(&Value::Null));
                part_answer.push(strip_html(&value));
            }
        }

        let keywords = get_str(question.get("keywords").unwrap_or(&Value::Null));
        let important: Vec<String> = keywords
            .split(" | ")
            .map(|s| strip_html(s).trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

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

fn collector_others(target: &Target) -> Result<Value, String> {
    if let Ok(r) = collector_repeat_essay_dialogue(target) {
        return Ok(r);
    }
    debug_print("Others can't be resolved by repeat, try others");

    if let Ok(r) = collector_choose(target) {
        return Ok(r);
    }
    debug_print("Others can't be resolved by choose, try others");

    if let Ok(r) = collector_read_word(target) {
        return Ok(r);
    }
    debug_print("Others can't be resolved by read_word, try others");

    Err("All collectors failed".into())
}

fn resolve_inner(target: &Target) -> Value {
    let content_val = match target {
        Target::Path(p) => {
            let content_path = p.join("content.json");
            if !content_path.exists() {
                return Value::Null;
            }
            let content_str = match fs::read_to_string(&content_path) {
                Ok(s) => s,
                Err(_) => return Value::Null,
            };
            match serde_json::from_str::<Value>(&content_str) {
                Ok(v) => v,
                Err(_) => return Value::Null,
            }
        }
        Target::NoFile { content, .. } => match serde_json::from_str::<Value>(content) {
            Ok(v) => v,
            Err(_) => return Value::Null,
        },
    };

    let structure_type = content_val
        .get("structure_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let result = if structure_type == "collector.choose" || structure_type.contains("choose") {
        collector_choose(target)
    } else if structure_type == "collector.read" || structure_type.contains("read") {
        collector_read_word(target)
    } else if structure_type == "collector.word" || structure_type.contains("word") {
        collector_read_word(target)
    } else if structure_type == "collector.repeat" || structure_type.contains("repeat") {
        collector_repeat_essay_dialogue(target)
    } else if structure_type == "collector.repeat_essay" || structure_type.contains("repeat_essay") {
        collector_repeat_essay_dialogue(target)
    } else if structure_type == "collector.repeat_dialogue" || structure_type.contains("repeat_dialogue") {
        collector_repeat_essay_dialogue(target)
    } else if structure_type == "collector.essay" || structure_type.contains("essay") {
        collector_repeat_essay_dialogue(target)
    } else if structure_type == "collector.dialogue" || structure_type.contains("dialogue") {
        collector_repeat_essay_dialogue(target)
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
        Err(_) => Value::Null,
    }
}

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

#[no_mangle]
pub extern "C" fn resolve(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let s = match unsafe { std::ffi::CStr::from_ptr(path) }.to_str() {
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

    crate::json_to_ptr(result)
}

#[no_mangle]
pub extern "C" fn resolve_by_homework(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let s = match unsafe { std::ffi::CStr::from_ptr(path) }.to_str() {
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

    crate::json_to_ptr(result)
}

#[no_mangle]
pub extern "C" fn resolve_by_all_homework(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let s = match unsafe { std::ffi::CStr::from_ptr(path) }.to_str() {
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

    crate::json_to_ptr(result)
}

/// 兼容旧名称: 释放字符串 (转发到新实现)
#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
