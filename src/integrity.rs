// -*- coding: utf-8 -*-
// 自校验与运行环境检测（Android / 宿主共用，wasm 下空实现）
//
// 两块能力：
//   1. 可执行段自校验（self text digest）
//      —— 在进程内定位自身映射，按 ELF program header 取 PF_X 段，对**文件字节**
//         [p_offset, p_offset+p_filesz) 求 sha256，与构建期注入的期望值比对。
//         与 tools/integrity/segment_digest.py 是同一算法（CI 用它算期望值）。
//         patch so 的代码段 → 摘要变化 → 可被拒服务。
//   2. 运行环境可疑项扫描
//      —— /proc/self/maps 里的注入/Hook/内存 dump 特征、rwxp 段、匿名可执行段。
//
// **灰度开关**：自校验的"不匹配即拒绝"默认关闭（见 `enforce_build_info`）。
// 摘要算错/环境差异会把所有用户拦死，因此构建期不显式开启时只记录、不拦截；
// 观察期数据来自 JNI `selfIntegrityCheck` 的 `text_digest_match` 字段。
#![allow(dead_code)]

use sha2::{Digest, Sha256};

// 由 build.rs 生成的固定长度常量（置于 .data 段，不影响可执行段布局）：
//   RESOLVER_EXPECTED_TEXT_DIGEST_BYTES / RESOLVER_INTEGRITY_ENFORCE
include!(concat!(env!("OUT_DIR"), "/selfcheck_digest.rs"));

/// 读取注入的期望摘要（未注入时为空串 —— 表示"不做段自校验"）。
///
/// 必须用 `read_volatile`：普通读取会被优化器把静态常量折进机器码，
/// 那样"改注入值"就会连带改可执行段，自校验会把正常产物判成被篡改。
#[inline(never)]
pub fn expected_text_digest() -> String {
    let bytes = unsafe {
        std::ptr::read_volatile(std::ptr::addr_of!(RESOLVER_EXPECTED_TEXT_DIGEST_BYTES))
    };
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    match std::str::from_utf8(&bytes[..end]) {
        Ok(s) => s.trim().to_string(),
        Err(_) => String::new(),
    }
}

/// 构建期注入的强制开关（env RESOLVER_INTEGRITY_ENFORCE=1 时开启段自校验拦截）。
/// 同样走 volatile 读取，保证开关不影响可执行段。
#[inline(never)]
pub fn enforce_build_info() -> bool {
    let flag = unsafe { std::ptr::read_volatile(std::ptr::addr_of!(RESOLVER_INTEGRITY_ENFORCE_BYTE)) };
    std::hint::black_box(flag) == 1
}

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
fn parse_maps() -> Vec<MapEntry> {
    let mut out = Vec::new();
    if let Ok(text) = std::fs::read_to_string("/proc/self/maps") {
        for line in text.lines() {
            if let Some(entry) = MapEntry::parse(line) {
                out.push(entry);
            }
        }
    }
    out
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
fn parse_maps() -> Vec<MapEntry> {
    Vec::new()
}

#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[derive(Debug, Clone)]
pub struct MapEntry {
    pub start: usize,
    pub end: usize,
    pub perms: String,
    pub offset: usize,
    pub path: String,
    pub raw: String,
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
#[derive(Debug, Clone)]
pub struct MapEntry {
    pub start: usize,
    pub end: usize,
    pub perms: String,
    pub offset: usize,
    pub path: String,
    pub raw: String,
}

impl MapEntry {
    fn parse(line: &str) -> Option<MapEntry> {
        // 7f8e4a200000-7f8e4a201000 r-xp 00002000 fd:01 1234 /apex/.../libc.so
        let mut parts = line.split_whitespace();
        let range = parts.next()?;
        let perms = parts.next()?.to_string();
        let offset = parts.next().and_then(|v| usize::from_str_radix(v, 16).ok()).unwrap_or(0);
        let _dev = parts.next();
        let _inode = parts.next();
        let path = parts.next().unwrap_or("").to_string();
        let (start_s, end_s) = range.split_once('-')?;
        let start = usize::from_str_radix(start_s, 16).ok()?;
        let end = usize::from_str_radix(end_s, 16).ok()?;
        Some(MapEntry { start, end, perms, offset, path, raw: line.to_string() })
    }

    pub fn is_exec(&self) -> bool {
        self.perms.as_bytes().get(2) == Some(&b'x')
    }

    pub fn is_write(&self) -> bool {
        self.perms.as_bytes().get(1) == Some(&b'w')
    }

    pub fn is_anonymous(&self) -> bool {
        self.path.is_empty()
    }
}

fn find_mapping_containing(addr: usize) -> Option<MapEntry> {
    parse_maps().into_iter().find(|m| addr >= m.start && addr < m.end)
}

/// 自身可执行映射的信息（运行时可哈希的内存区间 + 对应的文件区间）。
#[derive(Debug, Clone)]
pub struct CodeMapping {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub file_offset: usize,
    pub len: usize,
}

/// 定位包含自身代码的映射（r-xp）。
///
/// 说明：早期实现试图在内存里解析 ELF program header 再定位 PF_X 段，实测
/// `mapping.start - mapping.offset` 推出来的"载入基址"并不落在 ELF 头上
/// （读到的是别的映射），因此改为**直接对整段可执行映射求摘要**：
/// 运行时的内存区间 [start, end) 与文件区间 [file_offset, file_offset+len) 逐字节相同，
/// 静态工具（tools/integrity/segment_digest.py）按同一规则算即可对齐，
/// 不需要任何 ELF 解析，也避免页对齐/加载偏置的坑。
pub fn code_mapping() -> Option<CodeMapping> {
    let probe = code_mapping as *const () as usize;
    let mapping = find_mapping_containing(probe)?;
    if !mapping.is_exec() {
        return None;
    }
    let len = mapping.end.checked_sub(mapping.start)?;
    if len == 0 {
        return None;
    }
    Some(CodeMapping {
        path: mapping.path.clone(),
        start: mapping.start,
        end: mapping.end,
        file_offset: mapping.offset,
        len,
    })
}

/// 计算自身可执行段摘要（失败返回 None：环境不支持）。
///
/// 与 `tools/integrity/segment_digest.py` 严格同算法：
/// sha256(自身可执行映射覆盖的那段**文件字节**)。
pub fn text_segment_digest() -> Option<String> {
    #[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
    {
        return None;
    }
    #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
    {
        let mapping = code_mapping()?;
        let bytes = unsafe { std::slice::from_raw_parts(mapping.start as *const u8, mapping.len) };
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Some(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
    }
}

/// 注入代码/内存 dump 工具的强特征（命中即视为环境不可信）。
const STRONG_INDICATORS: [&str; 8] = [
    "frida-agent",
    "frida-gadget",
    "libgadget",
    "libxposed",
    "liblspd",
    "substrate",
    "libsandhook",
    "libmemprofiler",
];

/// 补充的弱特征（观察用：命中只上报，不拦截）。
const WEAK_INDICATORS: [&str; 10] = [
    "frida-server",
    "dumpmemory",
    "memdump",
    "libdump",
    "dumpso",
    "gdbserver",
    "lldb-server",
    "/gdb",
    "magisk",
    "riru",
];

#[derive(Debug, Clone, Default)]
pub struct EnvScan {
    /// 强特征命中（rwxp/注入/Hook）—— 调用方可据此拒绝服务
    pub strong: Vec<String>,
    /// 弱特征/结构异常 —— 只上报
    pub weak: Vec<String>,
    /// 我们自身模块映射是否可写可执行（正常绝不出现）
    pub self_rwxp: bool,
    /// 匿名可执行映射数量（Android 上 ART/JIT 常见，仅作参考）
    pub anon_exec_count: usize,
}

/// 扫描本进程内存映射里的可疑项。
pub fn scan_environment() -> EnvScan {
    let mut scan = EnvScan::default();
    let maps = parse_maps();
    let self_probe = text_segment_digest as *const () as usize;
    for entry in &maps {
        let lower = entry.raw.to_ascii_lowercase();
        if entry.is_exec() && entry.is_write() {
            // rwxp：JIT 之外几乎不合法；若正是我们自身模块则更严重
            let line = format!("rwxp {}", entry.path);
            if self_probe >= entry.start && self_probe < entry.end {
                scan.self_rwxp = true;
                scan.strong.push(format!("{line} (自身模块可写可执行)"));
            } else {
                scan.weak.push(line);
            }
        }
        if entry.is_exec() && entry.is_anonymous() {
            // Android 上 ART/JIT 会产生大量匿名可执行映射，逐条上报纯噪声：
            // 只计数；"可写+可执行"的组合已在上面的 rwxp 分支里单独判定。
            scan.anon_exec_count += 1;
        }
        for indicator in STRONG_INDICATORS {
            if lower.contains(indicator) && !scan.strong.iter().any(|s| s.contains(indicator)) {
                scan.strong.push(indicator.to_string());
            }
        }
        for indicator in WEAK_INDICATORS {
            if lower.contains(indicator) && !scan.weak.iter().any(|s| s.contains(indicator)) {
                scan.weak.push(indicator.to_string());
            }
        }
    }
    scan
}

#[derive(Debug, Clone)]
pub struct SelfCheck {
    pub digest: Option<String>,
    pub mapping: Option<CodeMapping>,
    pub expected: String,
    pub matched: Option<bool>,
    pub enforced: bool,
    pub scan: EnvScan,
}

impl SelfCheck {
    /// 是否应当拒绝服务。
    ///
    /// 规则：强特征命中 -> 拒绝（与既有反调试/反注入行为一致）；
    ///       摘要不匹配 -> **仅在构建期显式开启强制时**拒绝（灰度保护）。
    pub fn should_deny(&self) -> bool {
        if !self.scan.strong.is_empty() {
            return true;
        }
        self.enforced && self.matched == Some(false)
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "text_digest": self.digest,
            "text_mapping": self.mapping.as_ref().map(|m| serde_json::json!({
                "path": m.path, "start": m.start, "end": m.end, "file_offset": m.file_offset, "len": m.len
            })),
            "expected_digest": if self.expected.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(self.expected.clone()) },
            "text_digest_match": self.matched,
            "digest_enforced": self.enforced,
            "injection_strong": self.scan.strong,
            "injection_weak": self.scan.weak,
            "self_rwxp": self.scan.self_rwxp,
            "anon_exec_count": self.scan.anon_exec_count,
            "deny": self.should_deny(),
        })
    }
}

/// 完整自校验（段摘要 + 环境扫描）。纯计算，不产生副作用。
pub fn verify_self() -> SelfCheck {
    let digest = text_segment_digest();
    let expected = expected_text_digest();
    let matched = if expected.is_empty() {
        None
    } else {
        Some(digest.as_deref() == Some(expected.as_str()))
    };
    SelfCheck {
        digest,
        mapping: code_mapping(),
        expected,
        matched,
        enforced: enforce_build_info(),
        scan: scan_environment(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable_and_matches_static_tool_algorithm() {
        let a = text_segment_digest();
        let b = text_segment_digest();
        assert_eq!(a, b, "同一次运行内摘要必须稳定");
        let digest = a.expect("宿主/Android 上应能算出自身可执行段摘要");
        assert_eq!(digest.len(), 64, "sha256 十六进制长度");
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        // 与 tools/integrity/segment_digest.py 的一致性对拍必须在"进程就是那个 .so"时才有意义
        // （cargo test 跑的是测试可执行文件）。真正的门禁在宿主 JNI 探针：
        // tools/jni_probe/run.sh 会用静态工具算好摘要传进去核对。
        if let Ok(expected) = std::env::var("RESOLVER_EXPECT_TEXT_DIGEST") {
            let is_shared_object = code_mapping()
                .map(|m| m.path.ends_with(".so"))
                .unwrap_or(false);
            if !expected.trim().is_empty() && is_shared_object {
                assert_eq!(digest, expected.trim(), "运行时摘要必须等于静态工具算出的摘要");
            }
        }
    }

    #[test]
    fn expected_digest_defaults_to_empty_and_not_enforced() {
        // 未注入时不校验、不强制 —— 避免"算不出/算错就拦死用户"
        let injected = std::env::var("RESOLVER_TEXT_DIGEST").unwrap_or_default();
        let injected = injected.trim().to_ascii_lowercase();
        assert_eq!(expected_text_digest(), injected);
        let check = verify_self();
        assert_eq!(check.matched, None);
        assert!(!check.should_deny() || !check.scan.strong.is_empty());
    }

    #[test]
    fn environment_scan_reports_lists() {
        let scan = scan_environment();
        // 测试机上不该有注入特征（若有说明真被注入了）
        assert!(scan.strong.is_empty(), "测试环境出现强注入特征: {:?}", scan.strong);
        assert!(!scan.self_rwxp);
    }

    #[test]
    fn json_shape_is_stable() {
        let json = verify_self().to_json();
        for key in [
            "text_digest",
            "text_mapping",
            "expected_digest",
            "text_digest_match",
            "digest_enforced",
            "injection_strong",
            "injection_weak",
            "self_rwxp",
            "anon_exec_count",
            "deny",
        ] {
            assert!(json.get(key).is_some(), "缺少字段 {key}");
        }
    }
}
