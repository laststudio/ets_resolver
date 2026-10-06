#!/usr/bin/env bash
# 设备侧 JNI 探针（**不需要安装 App**）：把 arm64 so + 一个小 dex 推到手机，
# 用 app_process 在应用进程名下运行，打印自校验/契约的完整结果。
#
#   ./tools/jni_probe/run_device.sh
#
# 关键点：
#   --nice-name=com.shuaiqiu.fuckets100  让 /proc/self/cmdline 通过包名校验
#   -Dexpected.digest=<静态工具算出的值> 断言"运行期摘要 == 静态摘要"
# 需要：adb 可见的设备 + JDK + Android SDK build-tools(d8)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SO="${RESOLVER_SO:-$ROOT/../Fe-pro/app/src/main/jniLibs/arm64-v8a/libresolver.so}"
[ -f "$SO" ] || { echo "找不到 arm64 产物: $SO（可用 RESOLVER_SO=... 指定）"; exit 1; }

D8="$(ls "${ANDROID_HOME:-$HOME/Android/Sdk}"/build-tools/*/d8 2>/dev/null | tail -1)"
ANDROID_JAR="$(ls "${ANDROID_HOME:-$HOME/Android/Sdk}"/platforms/*/android.jar 2>/dev/null | tail -1)"
ADB="${ADB:-adb}"
[ -n "$(adb devices | sed -n '2,$p' | grep -w device || true)" ] || { echo "没有可用的 adb 设备（adb devices 为空）"; exit 1; }

OUT="${TMPDIR:-/tmp}/jni_probe_device"; rm -rf "$OUT"; mkdir -p "$OUT/classes"

echo "① 编译探针"
javac --release 17 -d "$OUT/classes" "$ROOT/tools/jni_probe/ResolverNative.java" "$ROOT/tools/jni_probe/JniProbe.java"
echo "② 生成 dex（d8）"
if [ -n "$D8" ] && [ -n "$ANDROID_JAR" ]; then
  "$D8" --lib "$ANDROID_JAR" --min-api 26 --output "$OUT" $(find "$OUT/classes" -name '*.dex' -o -name '*.class' | tr '\n' ' ')
else
  echo "  缺少 d8/android.jar，改用 zip 打包 class（部分设备可直接跑）"; (cd "$OUT/classes" && zip -qr "$OUT/classes.jar" .)
fi
DEX="$OUT/classes.dex"; [ -f "$DEX" ] || DEX="$OUT/classes.jar"

echo "③ 静态摘要（与运行期对拍）"
DIGEST="$(python3 "$ROOT/tools/integrity/segment_digest.py" "$SO" --json | python3 -c 'import json,sys;print(json.load(sys.stdin)["sha256"])')"
echo "   ${DIGEST:0:24}…"

echo "④ 推送到设备并运行"
adb push "$SO" /data/local/tmp/libresolver.so >/dev/null
adb push "$DEX" "/data/local/tmp/$(basename "$DEX")" >/dev/null
adb shell "cd /data/local/tmp && CLASSPATH=$(basename "$DEX") app_process --nice-name=com.shuaiqiu.fuckets100 \
  -Dresolver.lib=/data/local/tmp/libresolver.so -Dexpected.digest=$DIGEST /data/local/tmp com.shuaiqiu.fuckets100.JniProbe"
