// -*- coding: utf-8 -*-
// 自校验参数的构建期注入（期望摘要 + 强制开关）。
//
// 关键设计：两者都写成**固定长度、放在 `.data` 段的静态数据**，运行期用 volatile 读取：
//   1. 若用 option_env! / const 直接内联，注入值会被折进机器码 -> 可执行段摘要变化，
//      自校验会把正常产物判成"被篡改"（本地预演实测：c64f3a2b… -> fc8bb1f7…）；
//   2. public const RESOLVER_INTEGRITY_ENFORCE 同样如此（enforce 一开，代码段就变）。
// 固定长度保证段布局恒定，于是"只改注入值、可执行段摘要不变"可被 CI 守卫证明。
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=RESOLVER_TEXT_DIGEST");
    println!("cargo:rerun-if-env-changed=RESOLVER_INTEGRITY_ENFORCE");

    let raw = env::var("RESOLVER_TEXT_DIGEST").unwrap_or_default();
    let digest = raw.trim().to_ascii_lowercase();
    let enforce = if env::var("RESOLVER_INTEGRITY_ENFORCE").ok().as_deref() == Some("1") { 1u8 } else { 0u8 };

    let mut bytes = [0u8; 64];
    for (i, b) in digest.as_bytes().iter().take(64).enumerate() {
        bytes[i] = *b;
    }
    let values = bytes
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let generated = format!(
        "#[used]\n#[no_mangle]\n#[link_section = \".data.resolver_selfcheck\"]\n\
         pub static RESOLVER_EXPECTED_TEXT_DIGEST_BYTES: [u8; 64] = [{values}];\n\
         \n\
         #[used]\n#[no_mangle]\n#[link_section = \".data.resolver_selfcheck\"]\n\
         pub static RESOLVER_INTEGRITY_ENFORCE_BYTE: u8 = {enforce};\n"
    );

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("selfcheck_digest.rs"), generated).expect("write selfcheck_digest.rs");
}
