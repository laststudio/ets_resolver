#ifndef ETSRESOLVER_H
#define ETSRESOLVER_H

#include <stdbool.h>

#ifdef _WIN32
  #define ETSRESOLVER_EXPORT __declspec(dllimport)
#else
  #define ETSRESOLVER_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* 设置调试模式（true 输出调试信息） */
ETSRESOLVER_EXPORT void debug_mode(bool mode);

/* 解析单个文件夹，返回 JSON 字符串（失败返回 "null"），需调用 resolver_free_string 释放 */
ETSRESOLVER_EXPORT char* resolve(const char* path);

/* 解析单个作业，返回 JSON 数组字符串 */
ETSRESOLVER_EXPORT char* resolve_by_homework(const char* path);

/* 解析所有作业，返回 JSON 对象字符串 */
ETSRESOLVER_EXPORT char* resolve_by_all_homework(const char* path);

/* 释放由上述函数返回的字符串 */
ETSRESOLVER_EXPORT void resolver_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif /* DEMO_H */
