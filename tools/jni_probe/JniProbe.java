package com.shuaiqiu.fuckets100;

import java.util.ArrayList;
import java.util.List;

/** 宿主侧 JNI 冒烟测试：加载 libresolver.so -> 逐个调用 JNI 方法 -> 校验响应信封。 */
public final class JniProbe {

    private static final List<String> FAILURES = new ArrayList<>();
    private static int passed = 0;

    /** 结果落盘（Android 上 app_process 的 stdout 常被吞，落盘最稳）。 */
    private static final StringBuilder REPORT = new StringBuilder();
    private static java.io.File outFile;

    private static void emit(String line) {
        System.out.println(line);
        REPORT.append(line).append('\n');
        if (outFile != null) {
            try (java.io.FileWriter w = new java.io.FileWriter(outFile, false)) {
                w.write(REPORT.toString());
            } catch (Exception ignored) {
            }
        }
    }

    private static void check(String name, boolean ok, String detail) {
        if (ok) {
            passed++;
            emit(String.format("  ✅ %-26s %s", name, detail));
        } else {
            FAILURES.add(name + " -> " + detail);
            emit(String.format("  ❌ %-26s %s", name, detail));
        }
    }

    /** 极简 JSON 取值（避免引入依赖）：只支持顶层字符串/布尔/数字字段。 */
    private static String topField(String json, String key) {
        String needle = "\"" + key + "\":";
        int i = json.indexOf(needle);
        if (i < 0) return null;
        int j = i + needle.length();
        while (j < json.length() && Character.isWhitespace(json.charAt(j))) j++;
        if (j >= json.length()) return null;
        char c = json.charAt(j);
        if (c == '"') {
            int end = json.indexOf('"', j + 1);
            return end < 0 ? null : json.substring(j + 1, end);
        }
        int end = j;
        while (end < json.length() && ",}]\n".indexOf(json.charAt(end)) < 0) end++;
        return json.substring(j, end).trim();
    }

    public static void main(String[] args) {
        String lib = System.getProperty("resolver.lib", "target/debug/libresolver.so");
        String outPath = System.getProperty("out.file", "");
        if (!outPath.isEmpty()) {
            outFile = new java.io.File(outPath);
            emit("输出文件: " + outPath);
        }
        emit("libresolver: " + lib);
        try {
            System.load(lib);
        } catch (Throwable t) {
            emit("  ❌ System.load 失败: " + t);
            System.exit(2);
        }
        emit("  ✅ System.load 成功（JNI_OnLoad 已执行，方法已动态注册）\n");

        // 1) 完整性自校验 + 包名校验（宿主上 ActivityThread 不存在 -> 按设计放行）
        String integrity = ResolverNative.selfIntegrityCheck();
        emit("SELFCHECK_RAW " + integrity);
        check("selfIntegrityCheck",
                integrity != null && integrity.contains("\"cmdline_valid\":true"),
                "cmdline_valid=true（argv[0] 伪装生效）");

        // 1.5) 自校验：可执行段摘要必须与静态工具（tools/integrity/segment_digest.py）算出的一致。
        //      这是"自校验不会误拦用户"的前提 —— 两侧算法必须逐字节对齐。
        String digest = topField(integrity, "text_digest");
        String expectedDigest = System.getProperty("expected.digest", "").trim();
        if (expectedDigest.isEmpty()) {
            check("text digest", digest != null && digest.length() == 64,
                    "digest=" + trim(digest) + "（未提供 --expected 跳过对拍）");
        } else {
            check("text digest == 静态工具", expectedDigest.equals(digest),
                    "运行时=" + trim(digest) + " 静态=" + expectedDigest.substring(0, Math.min(16, expectedDigest.length())) + "…");
        }
        check("digest 未强制（灰度观察期）", "false".equals(topField(integrity, "digest_enforced")),
                "digest_enforced=" + topField(integrity, "digest_enforced"));

        // 2) 能力清单：契约版本 / op 数 / 覆盖率
        String caps = ResolverNative.capabilities();
        boolean capsOk = caps != null && "\"contract_version\"".length() > 0 && caps.contains("contract_version");
        String opCount = caps == null ? "?" : String.valueOf(countOccurrences(caps, "\"op\":"));
        check("capabilities", capsOk, "contract=" + topField(caps, "contract_version") + " ops=" + opCount);

        // 3) 单个解析（走契约入口，与 Android 业务层同一条路）
        String req = "{\"version\":1,\"op\":\"parse_task\",\"payload\":{\"content\":"
                + "{\"structure_type\":\"collector.role\",\"info\":{\"stid\":\"1\",\"question\":"
                + "[{\"ask\":\"Q\",\"std\":[{\"value\":\"A\"}]}]}}}}";
        String resp = ResolverNative.resolveRequest(req);
        check("resolveRequest(parse_task)", resp != null && resp.contains("\"ok\":true") && resp.contains("\"A\""),
                "answers=" + (resp != null && resp.contains("\"A\"")));

        // 4) 诊断（layer / family / spec_known）
        String content = "{\"structure_type\":\"collector.picture\",\"info\":{\"stid\":\"2\",\"std\":[{\"value\":\"text\"}]}}";
        String diag = ResolverNative.diagnose(content, "", "");
        check("diagnose", diag != null && diag.contains("\"layer\":\"explicit\""),
                "layer=" + topField(diag, "layer") + " family=" + topField(diag, "family"));

        // 5) 作业分类（习题集识别）
        String hw = "{\"id\":\"5\",\"name\":\"Unit 3 综合练习\",\"set_id\":\"s-1\",\"contents\":"
                + "[{\"group_name\":\"模仿朗读\"},{\"group_name\":\"信息转述\"}]}";
        String cls = ResolverNative.classifyHomework(hw);
        check("classifyHomework", cls != null && cls.contains("\"is_exercise_set\":true"),
                "category=" + topField(cls, "category"));

        // 6) 列表分类
        String clsList = ResolverNative.classifyHomeworkList("{\"homeworks\":[" + hw + "]}");
        check("classifyHomeworkList", clsList != null && clsList.contains("exercise_set_ids"), trim(clsList));

        // 7) 词汇解析
        // payload.data 就是词汇载荷本身（SyncWordQuestionBean 形状）
        String pk = "{\"words\":[{\"id\":1,\"content\":\"apple\",\"translate\":\"苹果\"}],"
                + "\"word_questions\":[{\"question_type\":6,\"word_id\":1,\"question_type_name\":\"听音选义\","
                + "\"answer\":\"A\",\"option\":[{\"option_name\":\"A\",\"option_value\":\"苹果\"}]}]}";
        String pkResp = ResolverNative.resolveRequest(
                "{\"op\":\"parse_word_pk\",\"payload\":{\"data\":" + pk + "}}");
        check("resolveRequest(parse_word_pk)",
                pkResp != null && pkResp.contains("word.choice") && pkResp.contains("\"channel\":\"sync_pk\""),
                "type=word.choice channel=sync_pk");

        // 8) 授权接口：无 token 时应返回结构化错误（证明 JNI 调用链通、不崩）
        ResolverNative.setAuthEnabled(false);
        String auth = ResolverNative.authorizeFeature();
        boolean authOk = auth != null && auth.contains("\"ok\"");
        check("authorizeFeature(auth disabled)", authOk, trim(auth));
        check("isAuthEnabled", !ResolverNative.isAuthEnabled(), "false after setAuthEnabled(false)");

        // 9) 旧接口（授权门控）：未配置授权时应返回结构化错误信封，而不是崩溃/空指针
        String legacy = ResolverNative.parseTaskAuthorized(
                "{\"structure_type\":\"collector.fill\",\"info\":{\"std\":[{\"xth\":\"1\",\"value\":\"v1\"}]}}", "", "");
        check("parseTaskAuthorized(旧接口,未授权)",
                legacy != null && legacy.contains("\"ok\":false") && legacy.contains("\"code\""),
                trim(legacy));

        // 10) 批量解析（授权门控）：同上
        String batch = ResolverNative.parseBatchAuthorized("[{\"content\":{\"structure_type\":\"collector.fill\"}}]");
        check("parseBatchAuthorized(旧接口,未授权)",
                batch != null && batch.contains("\"ok\":false"), trim(batch));

        emit(String.format("%n结果: %d 项通过, %d 项失败", passed, FAILURES.size()));
        if (!FAILURES.isEmpty()) {
            FAILURES.forEach(f -> emit("  失败: " + f));
            System.exit(1);
        }
        emit("JNI 可用 ✅");
    }

    private static String trim(String s) {
        if (s == null) return "null";
        return s.length() > 90 ? s.substring(0, 90) + "…" : s;
    }

    private static int countOccurrences(String haystack, String needle) {
        int count = 0, i = 0;
        while ((i = haystack.indexOf(needle, i)) >= 0) { count++; i += needle.length(); }
        return count;
    }
}
