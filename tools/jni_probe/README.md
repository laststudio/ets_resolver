# JNI 可用性测试

**结论：不需要 Android 设备也能真实验证 JNI。** 本目录提供宿主侧探针，走的是与
Android 完全相同的 Rust JNI 桥（`src/jni_bridge.rs`），包括 `JNI_OnLoad` 动态注册那一段。

## 一键运行

```bash
./tools/jni_probe/run.sh          # 只需本机有 JDK 21+ 与 cargo
```

实测输出（当前仓库状态，11/11）：

```
✅ System.load 成功（JNI_OnLoad 已执行，方法已动态注册）
✅ selfIntegrityCheck          cmdline_valid=true（argv[0] 伪装生效）
✅ capabilities                contract=1 ops=13
✅ resolveRequest(parse_task)  answers=true
✅ diagnose                    layer=explicit family=collector.picture
✅ classifyHomework            category=paper_set
✅ classifyHomeworkList        exercise_set_ids=["5"]
✅ resolveRequest(parse_word_pk) type=word.choice channel=sync_pk
✅ authorizeFeature(auth disabled)  结构化错误信封
✅ isAuthEnabled / parseTaskAuthorized / parseBatchAuthorized
结果: 11 项通过, 0 项失败
JNI 可用 ✅
```

## 它是怎么做到的（两个关键点）

1. **`--features _jni-host-test`**：让 Android JNI 桥在 x86_64 宿主上也参与编译
   （该 feature 不参与任何发布构建，Android/后端产物完全不受影响）。
2. **argv[0] 伪装**：完整性校验里有一关是读 `/proc/self/cmdline` 并校验进程名以
   `com.shuaiqiu.fuckets100` 开头。`run.sh` 把 `java` 软链成这个名字并用它启动，
   于是 `cmdline_valid=true`。**校验逻辑一行都没改**。
   （`verify_package_and_signature` 在找不到 `android/app/ActivityThread` 时按设计放行 ——
   它在真实 Android 上才严格校验签名证书。）

## 这一层能证明什么 / 不能证明什么

| ✅ 能证明 | ❌ 不能证明 |
|---|---|
| `JNI_OnLoad` 成功执行、`find_class` 找到 `com/shuaiqiu/fuckets100/ResolverNative` | bionic 与 glibc 的差异 |
| `register_native_methods` 的**方法名与签名**全部匹配（有一个不匹配就 `UnsatisfiedLinkError`） | APK 打包是否把 so 放进了正确的 `jniLibs/<abi>` |
| 每个 JNI 方法的**函数体**（含完整性校验、诊断、分类、契约分发）行为正确 | 应用内 `System.loadLibrary("resolver")` 的库名解析 |
| 响应信封契约（`ok` / `error.code` / `meta`）在 JNI 边界上不变形 | Kotlin 侧 `external fun` 声明与 so 是否漂移（见第三层） |

## 三层测试策略

| 层 | 需要什么 | 覆盖 | 状态 |
|---|---|---|---|
| **① 宿主探针**（本目录） | JDK 21+ + cargo | JNI 注册 + 全部方法体 + 契约 | ✅ 已接入 CI，每次 push 自动跑 |
| **② 设备上 `app_process`** | 一台 arm64 设备 | 真实的 bionic / Android 运行时 | ⬜ 命令见下 |
| **③ 应用内 instrumented test** | 设备 + Fe-pro 工程 | Kotlin 声明 + `System.loadLibrary` + 真实业务路径 | ⬜ 建议在 Fe-pro 加 |

### ② 设备上跑（无需装 APK）

```bash
# 1) 取官方 release 的 arm64 so（或本地 cargo ndk 编译）
gh release download --repo laststudio/ets_resolver --pattern 'libresolver.so' --dir /tmp/probe
adb push /tmp/probe/libresolver.so /data/local/tmp/
# 2) 编译探针为 dex
javac -d /tmp/probe/classes tools/jni_probe/*.java
"$ANDROID_HOME"/build-tools/*/d8 --output /tmp/probe /tmp/probe/classes/com/shuaiqiu/fuckets100/*.class
adb push /tmp/probe/classes.dex /data/local/tmp/
# 3) 关键：--nice-name 会把 /proc/self/cmdline 改写成包名，从而通过 cmdline 校验
adb shell 'cd /data/local/tmp && CLASSPATH=classes.dex app_process --nice-name=com.shuaiqiu.fuckets100 \
  /data/local/tmp com.shuaiqiu.fuckets100.JniProbe'
```
> 需要探针里改成 `System.load("/data/local/tmp/libresolver.so")`（可用 `-Dresolver.lib=` 传入）。
> 这一层**尚未实测**（本机没有设备），命令按 Android `app_process --nice-name` 的行为编写。

### ③ Fe-pro 内 instrumented test

在 `core/platform` 加 `androidTest`，直接 `ResolverNative.resolveRequest(...)` —— 这层才能
同时覆盖"Kotlin 声明 ↔ so 注册表"是否漂移。建议给 `resolver-wasm`/so 的每次升级都跑一次。

## CI

`build-release.yml` 在构建 Android so 之前会跑本探针（JDK 21 + cargo，无需设备）。
一旦 JNI 注册表或方法签名被改坏，CI 立刻失败 —— 而不是等发版到手机上才发现。
