#!/usr/bin/env bash
# 宿主侧 JNI 冒烟测试：不需要 Android 设备。
#
#   ./tools/jni_probe/run.sh
#
# 做三件事：① 用 _jni-host-test 编译带 JNI 桥的宿主 so
#          ② 编译探针 Java 类
#          ③ 以 argv[0]=com.shuaiqiu.fuckets100 启动 JVM（绕过 /proc/self/cmdline 的包名校验）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${TMPDIR:-/tmp}/jni_probe"
mkdir -p "$OUT/classes" "$OUT/bin"

echo "① 编译宿主 so（--features _jni-host-test）"
cargo build --manifest-path "$ROOT/Cargo.toml" --features _jni-host-test
SO="$ROOT/target/debug/libresolver.so"
[ -f "$SO" ] || { echo "未找到 $SO"; exit 1; }

echo "② 编译探针"
javac -d "$OUT/classes" "$ROOT/tools/jni_probe/ResolverNative.java" "$ROOT/tools/jni_probe/JniProbe.java"

echo "③ 运行（argv[0] 伪装为包名，使完整性校验的 cmdline 检查通过）"
ln -sf "$(readlink -f "$(command -v java)")" "$OUT/bin/com.shuaiqiu.fuckets100"
# JDK 24+ 对 JNI 有 "restricted method" 提示，该开关并非所有版本都接受 —— 实测探测
EXTRA=()
if java --enable-native-access=ALL-UNNAMED -version >/dev/null 2>&1; then
  EXTRA=(--enable-native-access=ALL-UNNAMED)
fi
PATH="$OUT/bin:$PATH" exec com.shuaiqiu.fuckets100 "${EXTRA[@]}" \
  -cp "$OUT/classes" -Dresolver.lib="$SO" com.shuaiqiu.fuckets100.JniProbe
