// -*- coding: utf-8 -*-
// HTML 清洗与题号提取工具 (与 resolver_py/cleaner.py 行为一致)

/// 剥离 HTML 标签: 跳过 <script>/<style> 内容与注释, 解码实体
pub fn strip_html(input: &str) -> String {
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

/// 按 </br>/<br> 拆分, 每段剥 HTML 后 trim, 去空
pub fn split_br(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    split_on_br(text)
        .iter()
        .map(|s| strip_html(s).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn split_on_br(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut start = 0;
    let mut i = 0;
    while i < n {
        if bytes[i] == b'<' {
            let mut j = i + 1;
            if j < n && bytes[j] == b'/' {
                j += 1;
            }
            if j < n && (bytes[j] == b'b' || bytes[j] == b'B')
                && j + 1 < n && (bytes[j + 1] == b'r' || bytes[j + 1] == b'R')
            {
                j += 2;
                if j < n && bytes[j] == b'/' {
                    j += 1;
                }
                if j < n && bytes[j] == b'>' {
                    result.push(&text[start..i]);
                    start = j + 1;
                    i = start;
                    continue;
                }
            }
        }
        i += 1;
    }
    result.push(&text[start..n]);
    result
}

/// 剥 HTML 后 trim, 去零宽空格
pub fn clean(text: &str) -> String {
    strip_html(text).replace('\u{200b}', "").trim().to_string()
}

/// 剥 HTML 后按行 trim 去空, 换行连接 (与 Python 一致: 先 strip_html)
pub fn clean_lines(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let cleaned = strip_html(text);
    cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 提取 ets_thN 编号
pub fn extract_ets_th(text: &str) -> Option<i64> {
    let t = text.trim_start();
    if let Some(rest) = t.to_ascii_lowercase().strip_prefix("ets_th") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    None
}

/// 去掉 ets_thN 前缀
pub fn strip_ets_th(text: &str) -> String {
    if extract_ets_th(text).is_some() {
        let t = text.trim_start();
        let lower = t.to_ascii_lowercase();
        let prefix_len = lower.find("ets_th").unwrap_or(0) + 6;
        let mut rest: String = t.chars().skip(prefix_len).collect();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        rest = rest.chars().skip(digits.len()).collect();
        return rest.trim().to_string();
    }
    text.trim().to_string()
}

/// 提取全局题号 "N." / "N、" / "N)" 等
pub fn extract_global_no(text: &str) -> Option<i64> {
    let t = text.trim_start();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let after: String = t.chars().skip(digits.len()).collect();
    let sep = after.chars().next()?;
    if ".、,，)）".contains(sep) {
        return digits.parse().ok();
    }
    None
}

/// 去掉全局题号前缀
pub fn strip_global_no(text: &str) -> String {
    if extract_global_no(text).is_some() {
        let t = text.trim_start();
        let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
        return t.chars().skip(digits.len() + 1).collect::<String>().trim().to_string();
    }
    text.trim().to_string()
}

/// 按分隔符拆分关键词, 每段 clean, 去空
pub fn split_keywords(text: &str, sep: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split(sep)
        .map(|k| clean(k))
        .filter(|k| !k.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
        assert_eq!(strip_html("a &amp; b &lt;c&gt;"), "a & b <c>");
        assert_eq!(strip_html("<script>var x=1;</script>text"), "text");
        assert_eq!(strip_html("&#x4F60;&#22909;"), "你好");
        assert_eq!(strip_html("<!-- comment -->x"), "x");
    }

    #[test]
    fn test_split_br() {
        assert_eq!(split_br("a</br>b<br/>c"), vec!["a", "b", "c"]);
        assert_eq!(split_br("<p>x</p>y"), vec!["xy"]);
    }

    #[test]
    fn test_extract_numbers() {
        assert_eq!(extract_ets_th("ets_th5 What is it?"), Some(5));
        assert_eq!(extract_ets_th("no number"), None);
        assert_eq!(strip_ets_th("ets_th3 ask"), "ask");
        assert_eq!(extract_global_no("7. What?"), Some(7));
        assert_eq!(extract_global_no("12、问"), Some(12));
        assert_eq!(extract_global_no("no"), None);
        assert_eq!(strip_global_no("3. q"), "q");
    }

    #[test]
    fn test_split_keywords() {
        assert_eq!(split_keywords("a | b | ", "|"), vec!["a", "b"]);
        assert_eq!(split_keywords("", "|"), Vec::<String>::new());
    }

    #[test]
    fn test_clean_lines() {
        assert_eq!(clean_lines("<p>a</p><p>b</p>"), "ab");
        assert_eq!(clean_lines("x</br>y"), "xy");
    }
}
