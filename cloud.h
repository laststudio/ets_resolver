#ifndef CLOUD_H
#define CLOUD_H

#include <stdbool.h>
#include <stdint.h>

#ifdef _WIN32
#define CLOUD_EXPORT __declspec(dllimport)
#else
#define CLOUD_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

CLOUD_EXPORT int cloud_set_auth_config(const char*, const char*, const char*, const char*, int64_t);
CLOUD_EXPORT void cloud_set_access_token(const char* token);
CLOUD_EXPORT void cloud_clear_access_token(void);
CLOUD_EXPORT void cloud_set_auth_enabled(bool enabled);
CLOUD_EXPORT char* cloud_authorize_feature(void);
CLOUD_EXPORT int set_cloud_config(const char*, const char*, const char*, const char*, uint64_t);
CLOUD_EXPORT char* cloud_generate_device_code(void);
CLOUD_EXPORT char* cloud_login(const char*, const char*, const char*);
CLOUD_EXPORT char* cloud_login_changyan(const char*, const char*, const char*, const char*, const char*);
CLOUD_EXPORT char* cloud_send_device_binding_code(const char*);
CLOUD_EXPORT char* cloud_bind_device(const char*, const char*, const char*, const char*);
CLOUD_EXPORT char* cloud_get_ecard_selection(const char*, const char*);
CLOUD_EXPORT char* cloud_get_homework_list(const char*, const char*, const char*);
CLOUD_EXPORT char* cloud_download_file(const char*, const char*);
CLOUD_EXPORT char* cloud_zip_password(const char*);
CLOUD_EXPORT void cloud_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif
