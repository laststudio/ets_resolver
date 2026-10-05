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

/* 跨平台固定接口（唯一入口，推荐所有新接入方使用）
 *
 * resolve_request: {"version":1,"op":"parse_task","payload":{...}} -> 响应信封
 *                  成功 {"ok":true,"version":1,"op":"...","result":...,"meta":{...}}
 *                  失败 {"ok":false,"version":1,"op":"...","error":{"code","message","hint"}}
 * capabilities:    能力清单（ops / categories / error_codes / notes）
 *
 * 平台只需实现这一次 JSON 进出；新能力通过新增 op 交付，不改 ABI。
 * 详细契约见 docs/INTERFACE.md。 */
RESOLVER_EXPORT char* resolve_request(const char* request_json);
RESOLVER_EXPORT char* capabilities(void);

/* 诊断与一致性工具（parity 基础设施），返回值均由 resolver_free_string 释放。
 *
 * diagnose:          这份资源包为什么出题/没出题：命中的路由层、形状指纹、告警。
 *                    输出不含题目文本，可安全进日志。
 * shape_fingerprint: 结构指纹。官方新增题型时用它判定"只是改名"还是"结构变了"。
 * classify_homework: 作业分类（含"老师布置的一整套多卷/习题集"识别）与卷面满分推算。
 * classify_homework_list: 作业列表批量分类，给出 exercise_set_ids 清单。 */
RESOLVER_EXPORT char* diagnose(const char* content, const char* info, const char* content2);
RESOLVER_EXPORT char* shape_fingerprint(const char* content, const char* info, const char* content2);
RESOLVER_EXPORT char* classify_homework(const char* homework_json);
RESOLVER_EXPORT char* classify_homework_list(const char* list_json);

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
