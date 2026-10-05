# 防护清单（当前状态 · 实测）

> 结论先说：**当前不是 OLLVM**。OLLVM 在 CI 里只是一个未实现的桩（见文末"OLLVM 现状"）。
> 实际生效的是"标准 Rust 强加固 + 代码级手写防护 + 运行期自校验"。

## 一、分层清单

| 层 | 内容 | 证据/位置 |
|---|---|---|
| 编译期 | `opt-level=3`、`lto="fat"`、`codegen-units=1`、`panic="abort"`、`strip=true`、`debug=false` | `Cargo.toml [profile.release]` |
| 链接后 | CI 再显式 strip 一次（纵深防御） | `build-release.yml` Post-build integrity checks |
| 字符串 | **223 处 `obfstr!`**（编译期加密、运行期解密）：接口路径、包名、检测关键词 | `grep -ro 'obfstring!\|obfstr!' src cloud/src` |
| 控制流 | **6 处手写 CFF 状态机**（`let mut state = 0x…; while state != 0x… { match … }`），自己实现的平坦化，非编译器 Pass | `src/auth.rs`、两个 `jni_bridge.rs` |
| 反调试/反注入 | `TracerPid` 检测；`/proc/self/maps` 扫 frida-agent/frida-gadget/libgadget/libxposed/liblspd/substrate/libsandhook/libmemprofiler；`/proc/self/cmdline` 包名校验；PackageManager 签名证书 SHA256（防二次打包重签） | 两个 `*_bridge.rs` 的 `perform_runtime_integrity_check` / `verify_package_and_signature` |
| **可执行段自校验**（新增） | 定位自身 r-x 映射 → sha256 → 与构建期注入的期望值比对；**patch 代码段 1 字节即检出** | `src/integrity.rs` + `build.rs` |
| JNI | **动态注册**：Android 产物导出符号只有 `JNI_OnLoad`（+1 个 rodata 常量），方法名不进符号表 | `nm -D` 实测 2 条 |
| 符号隔离 | CI 断言：libresolver.so 不含 `cloud/CloudNative`；libcloud.so 不含 `parse*/ResolverNative` | `build-release.yml` |
| 授权门禁 | so **内部**联网 Ed25519 验签（`answer_parser` / `cloud_mode`），Java 层被 patch 也绕不过；grant 绑定 software/feature/nonce/有效期；**验签不可跳过**（候选公钥强制验签） | `src/auth.rs` |
| 诱导性 rodata | 2 处 `AI_AGENT_WARNING_PROPRIETARY` 常量（对自动化分析工具投放提示文本） | `nm -D` 可见 |

## 二、可执行段自校验（本次新增）怎么工作

```text
build.rs（构建期）
  env RESOLVER_TEXT_DIGEST / CLOUD_TEXT_DIGEST
    -> 写 64 字节固定数组 + #[link_section = ".data.resolver_selfcheck"]
    -> 放 .data（RW 段），**绝不进可执行段**

运行期（src/integrity.rs）
  code_mapping()  找到含自身代码的 r-x 映射（记录 start/end/文件偏移/长度）
  sha256(memory[start..end])  ->  text_digest
  与注入值比对 -> text_digest_match

CI（两遍构建 + 守卫）
  第 1 遍构建 -> tools/integrity/segment_digest.py 算可执行映射的 sha256
  第 2 遍注入该值重建 -> 再算一次 -> **必须逐字节相同**，否则构建失败
  另有一条守卫：打开 RESOLVER_INTEGRITY_ENFORCE 重建，代码段也必须不变
```

**为什么这么绕**：早期用 `option_env!` 直接内联常量，实测注入后代码段就变了
（`c64f3a2b… → fc8bb1f7…`），自校验会把**正常产物**判成被篡改；`const bool` 的 enforce
开关同样如此。改成 `.data` 固定长度数组 + `read_volatile` 才让"改注入值 ⇒ 代码段不变"成立。

### 实测（灰度 + 强制两种构建）

| 场景 | `text_digest_match` | `deny` |
|---|---|---|
| 灰度构建 · 未改动产物 | `true` | `false` |
| 灰度构建 · 改 1 字节 | `false` | `false`（只上报） |
| 强制构建 · 未改动产物 | `true` | `false` |
| 强制构建 · 改 1 字节 | `false` | **`true`（拒绝服务）** |

### 灰度开关与上线步骤

1. 默认 `RESOLVER_INTEGRITY_ENFORCE` **未设置 = 0（灰度）**：只上报 `text_digest_match`，
   不影响任何用户；端侧数据来自 JNI `selfIntegrityCheck`。
2. 观察一段时间（真实机型上 `text_digest_match` 应恒为 `true`；出现 `false` 说明有
   我们没预料到的映射形态/改机环境，先别开强制）。
3. 确认无误后，在仓库 Variables 里设 `RESOLVER_INTEGRITY_ENFORCE=1` → 下一次 CI 构建
   即为强制模式（CI 会用宿主产物先证明"开强制不影响代码段"）。

对拍工具：`python3 tools/integrity/segment_digest.py <so> [--json]`
宿主探针：`./tools/jni_probe/run.sh`（会断言"运行时摘要 == 静态摘要"，13 项检查）

## 三、已知薄弱点（诚实清单）

| 项 | 现状 | 影响 |
|---|---|---|
| OLLVM（FLA/SUB/BCF/字符串 Pass） | **未实现**（CI 里是桩，见文末） | 反编译出来仍是可读性较高的 Rust；只有 6 处手写 CFF 保护关键路径 |
| 段加密 / 段自校验之外的完整性 | 无 | 代码段可修复式 patch（自校验能发现，但灰度期不拦） |
| Debug 构建的签名校验 | `EXPECTED_SIGNATURE_SHA256` 为空时按设计**跳过** | 仅 Release 包严格校验 |
| 宿主 Linux .so / wasm | 无 Android 专有校验；wasm 基本等于源码级可读 | Fe_web 按设计不走 wasm，只请求后端 |
| CFF 覆盖度 | 关键路径（授权验签、完整性校验、包名校验）已覆盖；解析分发/ZIP 密码推导未覆盖 | 可继续扩展 |

## 四、OLLVM 现状（核实过，不是猜的）

| 检查 | 结果 |
|---|---|
| `ollvm-obfuscate` job | 只有 `echo` + `true`（**未实现的桩**），且 `needs: release` → 跑在发布之后，改不了产物 |
| 另一个 worktree（`.kilo/…/rectangular-glow`） | 同一份桩，没有真实脚本 |
| 本机工具链 / 镜像 | 无 OLLVM 目录、无自定义 rustc、无相关镜像 |
| 近 40 次 CI 用时 | **没有一次超过 20 分钟**（OLLVM 自述 ~1h）→ 从未构建过 |

要真正上 OLLVM 需要：预构建 LLVM 17 + 自定义 Pass 的容器镜像 + 在 `cargo ndk` 前切换
`RUSTC`/`RUSTFLAGS`，并把 `ollvm-obfuscate` 挪到 release 之前、由它产出可发布产物。
在接入之前，建议先按第二节把自校验开成强制模式（收益/成本比最高）。
