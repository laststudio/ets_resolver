#ifndef RESOLVER_H
#define RESOLVER_H

#include <stdbool.h>
#include <stdint.h>

#ifdef _WIN32
  #define RESOLVER_EXPORT __declspec(dllimport)
#else
  #define RESOLVER_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* 设置调试模式（true 输出调试信息） */
RESOLVER_EXPORT void debug_mode(bool mode);

/* 设置无文件模式（true 时，传入参数为 JSON 字符串而不是路径） */
RESOLVER_EXPORT void nofile_mode(bool mode);

/* 纯 JSON 解析接口，返回值均由 resolver_free_string 释放。 */
RESOLVER_EXPORT char* parse_task(const char* content, const char* info, const char* content2);
RESOLVER_EXPORT char* parse_batch(const char* tasks_json);
RESOLVER_EXPORT char* group_by_time(const char* items_json, int64_t threshold_ms);
RESOLVER_EXPORT char* extract_meta(const char* content);

/* 解析单个文件夹或JSON字符串，返回 JSON 字符串 */
RESOLVER_EXPORT char* resolve(const char* path);

/* 解析单个作业，返回 JSON 数组字符串 */
RESOLVER_EXPORT char* resolve_by_homework(const char* path);

/* 解析所有作业，返回 JSON 对象字符串 */
RESOLVER_EXPORT char* resolve_by_all_homework(const char* path);

/* LA Studio 内部授权；authorized 解析入口固定校验 answer_parser。 */
RESOLVER_EXPORT int set_auth_config(
    const char* base_url,
    const char* software_key,
    const char* feature_key,
    const char* public_key_b64,
    int64_t app_version_code
);
RESOLVER_EXPORT void set_access_token(const char* token);
RESOLVER_EXPORT void clear_access_token(void);
RESOLVER_EXPORT void set_auth_enabled(bool enabled);
RESOLVER_EXPORT bool is_auth_enabled(void);
RESOLVER_EXPORT char* authorize_feature(void);
RESOLVER_EXPORT char* parse_task_authorized(const char* content, const char* info, const char* content2);
RESOLVER_EXPORT char* parse_batch_authorized(const char* tasks_json);

/* ETS100 云端模式。空配置字符串保留默认值；讯飞登录不在此接口中。 */
RESOLVER_EXPORT int set_cloud_config(
    const char* api_base_url,
    const char* cdn_base_url,
    const char* pid,
    const char* secret_key,
    uint64_t timeout_ms
);
RESOLVER_EXPORT char* cloud_authorize_feature(void);
RESOLVER_EXPORT char* cloud_generate_device_code(void);
RESOLVER_EXPORT char* cloud_login(const char* phone, const char* password, const char* device_code);
RESOLVER_EXPORT char* cloud_login_changyan(
    const char* login,
    const char* user_id,
    const char* captcha_result,
    const char* device_code,
    const char* preferred_account_id
);
RESOLVER_EXPORT char* cloud_send_device_binding_code(const char* phone);
RESOLVER_EXPORT char* cloud_bind_device(
    const char* phone,
    const char* password,
    const char* device_code,
    const char* verification_code
);
RESOLVER_EXPORT char* cloud_get_ecard_selection(const char* token, const char* preferred_account_id);
RESOLVER_EXPORT char* cloud_get_homework_list(
    const char* token,
    const char* parent_account_id,
    const char* status
);
RESOLVER_EXPORT char* cloud_download_file(const char* url, const char* destination);
RESOLVER_EXPORT char* cloud_zip_password(const char* path);
RESOLVER_EXPORT char* cloud_download_and_parse_homework(
    const char* base_url,
    const char* contents_json,
    const char* cache_directory
);

/* 释放由上述函数返回的字符串 */
RESOLVER_EXPORT void resolver_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* RESOLVER_H */
