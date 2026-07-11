#ifndef RESOLVER_H
#define RESOLVER_H

#include <stdbool.h>

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

/* 解析单个文件夹或JSON字符串，返回 JSON 字符串 */
RESOLVER_EXPORT char* resolve(const char* path);

/* 解析单个作业，返回 JSON 数组字符串 */
RESOLVER_EXPORT char* resolve_by_homework(const char* path);

/* 解析所有作业，返回 JSON 对象字符串 */
RESOLVER_EXPORT char* resolve_by_all_homework(const char* path);

/* 释放由上述函数返回的字符串 */
RESOLVER_EXPORT void resolver_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* RESOLVER_H */
