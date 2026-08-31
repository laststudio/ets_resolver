# -*- coding: utf-8 -*-
"""内部后端授权 E2E 测试

前置: lastudio-backend 本地已在 127.0.0.1:8000 运行, 且已:
  - 应用 e5f6a7b8c9d0 迁移 (seed answer_parser 功能)
  - 应用 c3d4e5f6a7b8 迁移 (seed cloud_mode 功能)
  - 创建用户 resolver_e2e / TestPass!23456
  - fe-android 客户端已 provisioned (迁移 b9f2c1e4a7d3)

流程:
  1. 通过 /client-auth/login 获取 access_token (模拟 Fe 外部登录)
  2. 配置 libresolver.so 内部授权 (base_url/software_key/feature_key/公钥/token)
  3. parse_task_authorized: 内部先授权 (带 Ed25519 验签) 再解析
   4. cloud_authorize_feature: 内部授权并验签 cloud_mode grant
   5. 后端关闭对应功能后, 下一次未命中有效 grant 的调用应被拒绝
"""
import ctypes
import json
import os
import sys
import urllib.error
import urllib.request

BASE_URL = os.environ.get("RESOLVER_E2E_BASE", "http://127.0.0.1:8000")
CLIENT_KEY = "fe-android"
SOFTWARE_KEY = "fe"
FEATURE_KEY = "answer_parser"
VERSION_CODE = 17
SIGNING_CERT = "79fe6f92aa7f46084832cd62292c223101586b46e3cfaf39740fef1c6253aaca"
PUBLIC_KEY_B64 = "iIsgUPrBJidorQGIs-BJCt5EiLrRHVbAWStnNaPn3cI"

# 本地开发后端测试账号, 可通过环境变量覆盖。
E2E_USER = os.environ.get("RESOLVER_E2E_USER", "resolver_e2e")
E2E_PASS = os.environ.get("RESOLVER_E2E_PASS", "TestPass!23456")

LIB_PATH = os.environ.get(
    "RESOLVER_LIB_PATH",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "target", "release", "libresolver.so"),
)
CLOUD_LIB_PATH = os.environ.get(
    "CLOUD_LIB_PATH",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "target", "release", "libcloud.so"),
)

SAMPLE_CONTENT = json.dumps({
    "structure_type": "collector.fill",
    "info": {
        "stid": "1",
        "std": [
            {"xth": "1", "value": "answer_one"},
            {"xth": "2", "value": "answer_two"},
        ],
    },
}, ensure_ascii=False)


def http(method: str, path: str, body=None, headers=None):
    req = urllib.request.Request(BASE_URL + path, method=method, headers=headers or {})
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, data=data) as resp:
            raw = resp.read().decode()
            return resp.status, json.loads(raw) if raw else None
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        try:
            return e.code, json.loads(raw)
        except json.JSONDecodeError:
            return e.code, raw


def login() -> str:
    status, body = http("POST", "/client-auth/login", {
        "client_key": CLIENT_KEY,
        "username": E2E_USER,
        "password": E2E_PASS,
        "device": {
            "installation_id": "e2e-install-002",
            "platform_device_id": "e2e-device-002",
            "application_id": "com.shuaiqiu.fuckets100",
            "signing_certificate_sha256": SIGNING_CERT,
            "version_code": VERSION_CODE,
            "device_name": "e2e-dev",
            "platform_version": "Android 15",
        },
    })
    assert status == 200, f"登录失败: {status} {body}"
    return body["access_token"]


def toggle_feature(status_: str) -> None:
    """后台管理场景: 通过后台接口或数据库开关 answer_parser 功能。

    本 E2E 脚本只负责解析器侧验证; 后台开关由管理端操作。
    验证方式: 将 software_feature.status 改为 disabled 后, 解析器下一次
    parse_task_authorized 立即返回 {"ok": false, "error": {"code":
    "feature_disabled", ...}}。
    """


def call_ffi(lib, fn_name: str, *args, free_name: str = "resolver_free_string") -> object:
    fn = getattr(lib, fn_name)
    fn.restype = ctypes.c_void_p
    if args:
        fn.argtypes = [ctypes.c_char_p] * len(args)
    ptr = fn(*[a.encode() if isinstance(a, str) else a for a in args])
    if not ptr:
        return None
    try:
        s = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
        return json.loads(s)
    finally:
        getattr(lib, free_name)(ptr)


def main() -> int:
    lib = ctypes.CDLL(LIB_PATH)
    cloud_lib = ctypes.CDLL(CLOUD_LIB_PATH)
    lib.resolver_free_string.argtypes = [ctypes.c_void_p]
    lib.set_auth_config.argtypes = [
        ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int64,
    ]
    lib.set_auth_config.restype = ctypes.c_int32
    lib.set_access_token.argtypes = [ctypes.c_char_p]
    lib.set_auth_enabled.argtypes = [ctypes.c_bool]
    lib.is_auth_enabled.restype = ctypes.c_bool

    access_token = login()
    print(f"[1] 外部登录成功, access_token={access_token[:24]}...")

    rc = lib.set_auth_config(
        BASE_URL.encode(),
        SOFTWARE_KEY.encode(),
        FEATURE_KEY.encode(),
        PUBLIC_KEY_B64.encode(),
        VERSION_CODE,
    )
    assert rc == 0, f"set_auth_config 失败 rc={rc}"
    lib.set_access_token(access_token.encode())
    lib.set_auth_enabled(True)
    print("[2] 内部授权配置完成")

    ok = call_ffi(lib, "parse_task_authorized", SAMPLE_CONTENT, "[]", "[]")
    assert ok is not None and ok.get("ok") is True, f"授权解析失败: {ok}"
    result = ok["result"]
    assert isinstance(result, list) and len(result) == 2, f"解析结果异常: {result}"
    assert result[0]["answers"] == ["answer_one"], f"答案异常: {result}"
    grant = ok["grant"]
    assert grant.get("allowed") is True, f"grant 异常: {grant}"
    assert grant.get("feature_key") == FEATURE_KEY
    assert grant.get("grant_key_id"), "缺少 grant_key_id"
    print(f"[3] parse_task_authorized 成功: {len(result)} 题, "
          f"decision={grant.get('decision_id')}")

    batch = call_ffi(lib, "parse_batch_authorized", json.dumps([
        {"content": SAMPLE_CONTENT, "order": 1},
        {"content": SAMPLE_CONTENT, "order": 2},
    ]))
    assert batch is not None and batch.get("ok") is True, f"批量授权解析失败: {batch}"
    sections = batch["result"]["sections"]
    assert sections and sections[0]["questions"], f"批量结果异常: {sections}"
    print(f"[4] parse_batch_authorized 成功: {len(sections)} 节")

    cloud_lib.cloud_free_string.argtypes = [ctypes.c_void_p]
    cloud_lib.cloud_set_auth_config.argtypes = [
        ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int64,
    ]
    cloud_lib.cloud_set_auth_config.restype = ctypes.c_int32
    cloud_lib.cloud_set_access_token.argtypes = [ctypes.c_char_p]
    cloud_lib.cloud_set_auth_enabled.argtypes = [ctypes.c_bool]
    cloud_rc = cloud_lib.cloud_set_auth_config(
        BASE_URL.encode(), SOFTWARE_KEY.encode(), b"cloud_mode", PUBLIC_KEY_B64.encode(), VERSION_CODE
    )
    assert cloud_rc == 0, f"cloud_set_auth_config 失败 rc={cloud_rc}"
    cloud_lib.cloud_set_access_token(access_token.encode())
    cloud_lib.cloud_set_auth_enabled(True)
    cloud_grant = call_ffi(
        cloud_lib, "cloud_authorize_feature", free_name="cloud_free_string"
    )
    assert cloud_grant is not None and cloud_grant.get("ok") is True, (
        f"cloud_mode 授权失败: {cloud_grant}"
    )
    cloud_grant_result = cloud_grant["result"]
    assert cloud_grant_result.get("allowed") is True, cloud_grant
    assert cloud_grant_result.get("feature_key") == "cloud_mode", cloud_grant
    assert cloud_grant_result.get("grant_key_id"), "cloud_mode grant 缺少 grant_key_id"
    print(f"[5] cloud_mode 授权成功: decision={cloud_grant_result.get('decision_id')}")

    # 关闭授权后必须拒绝 (即便内容合法)
    lib.set_auth_enabled(False)
    denied = call_ffi(lib, "parse_task_authorized", SAMPLE_CONTENT, "[]", "[]")
    assert denied is not None and denied.get("ok") is False, f"关闭授权后未拒绝: {denied}"
    assert denied["error"]["code"] == "auth_disabled", f"错误码异常: {denied}"
    print(f"[6] 关闭内部授权后正确拒绝: {denied['error']}")

    # 使用无效 token 应被后端拒绝
    lib.set_auth_enabled(True)
    lib.set_access_token(b"lsa_bad.invalid-token")
    invalid = call_ffi(lib, "parse_task_authorized", SAMPLE_CONTENT, "[]", "[]")
    assert invalid is not None and invalid.get("ok") is False, f"无效 token 未拒绝: {invalid}"
    print(f"[7] 无效 token 被拒绝: {invalid['error']}")

    print("\nE2E 全部通过 ✓")
    return 0


if __name__ == "__main__":
    sys.exit(main())
