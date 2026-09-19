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
RESOLVER_EXPORT char* parse_word_pk(const char* pk_data);
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

/* 释放由上述函数返回的字符串 */
RESOLVER_EXPORT void resolver_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* RESOLVER_H */
