package com.shuaiqiu.fuckets100;

/**
 * 与 Android 端 ResolverNative 声明完全一致的 JNI 桥（宿主侧冒烟测试用）。
 *
 * 注意：这份声明必须与 src/jni_bridge.rs 的 JNI_OnLoad 注册表一一对应，
 * 否则 JNI_OnLoad 里的 find_class / register_native_methods 会失败（UnsatisfiedLinkError）。
 */
public final class ResolverNative {
    public static native int setAuthConfig(String baseUrl, String softwareKey, String featureKey,
                                          String publicKeyB64, long appVersionCode);
    public static native void setAccessToken(String token);
    public static native void clearAccessToken();
    public static native void setAuthEnabled(boolean enabled);
    public static native boolean isAuthEnabled();
    public static native String authorizeFeature();
    public static native String parseTaskAuthorized(String content, String info, String content2);
    public static native String parseBatchAuthorized(String tasksJson);
    public static native String resolveRequest(String requestJson);
    public static native String capabilities();
    public static native String diagnose(String content, String info, String content2);
    public static native String classifyHomework(String homeworkJson);
    public static native String classifyHomeworkList(String listJson);
    public static native String selfIntegrityCheck();
}
