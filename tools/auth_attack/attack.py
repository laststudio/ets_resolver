#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""授权门禁的对抗性验证（回归用例）。

背景：2a0b514 曾在"grant_key_id 命中生产 key_id"时**跳过 Ed25519 验签**，
而 grant_key_id 本身不参与签名 —— 于是任何人返回 allowed=true + 该 key_id + 垃圾签名
即可解锁功能。本脚本用假授权服务器复现该攻击，并验证修复后的行为。

用法（需要 cryptography；用任一含该依赖的 venv 即可）：
    python3 tools/auth_attack/attack.py

期望输出（修复后）：
    ① 伪造：生产 key_id + 垃圾签名        -> 拒绝 signature_invalid   ← 修复前是放行
    ② 伪造：真实 key_id + 垃圾签名        -> 拒绝 signature_invalid
    ③ 合法：动态下发公钥签发的 grant      -> 放行（生产换钥场景，无需绕过）
    ④ 合法签名但来自不相干私钥            -> 拒绝 signature_invalid
    ⑤ 合法：后端私钥签发 + 不传动态公钥   -> 放行（走内置默认钥）
    ⑥ 伪造：内置 key_id + 垃圾签名        -> 拒绝 signature_invalid

前置：SO 路径默认取本仓库 target/debug/libresolver.so（cargo build 即可）。
每个用例使用独立 feature_key，避免复用同 feature 的合法 grant 缓存（~120s TTL）。
"""
import base64, ctypes, hashlib, json, os, threading, time
from http.server import BaseHTTPRequestHandler, HTTPServer
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

SO = os.environ.get("RESOLVER_SO", os.path.join(os.path.dirname(__file__), "..", "..", "target", "debug", "libresolver.so"))
PROD_KEY_ID = "51b812c67199c51b"          # 生产签名钥 id（修复前"命中即放行"的那把）
B64 = lambda b: base64.urlsafe_b64encode(b).decode().rstrip("=")

# ---- 两把钥匙：生产钥（动态下发给客户端）与不相干钥 ----
prod_priv = Ed25519PrivateKey.generate()
prod_pub_b64 = B64(prod_priv.public_key().public_bytes_raw())
prod_kid = hashlib.sha256(prod_priv.public_key().public_bytes_raw()).hexdigest()[:16]
other_priv = Ed25519PrivateKey.generate()
other_pub_b64 = B64(other_priv.public_key().public_bytes_raw())

MODE = {"mode": "forged_prod_kid"}
FEATURE = {"value": "answer_parser"}
NONCE = {"value": ""}

def canonical(payload: dict) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)

def make_body(sign_with=None, kid=PROD_KEY_ID, sig_override=None):
    payload = {
        "allowed": True, "decision_id": "d-1", "software_key": "fe", "feature_key": FEATURE["value"],
        "operation_nonce": NONCE["value"], "policy_revision": 1, "user_id": 1, "session_id": 1,
        "device_id": 1, "client_key": "c-1",
        "issued_at": int(time.time()), "expires_at": int(time.time()) + 120,
    }
    sig = sig_override if sig_override is not None else sign_with.sign(canonical(payload).encode())
    return {**payload, "grant": B64(sig), "grant_key_id": kid}

class H(BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        req = json.loads(self.rfile.read(n) or b"{}")
        NONCE["value"] = (req.get("operation_nonce") or req.get("params", {}).get("operation_nonce")
                          or json.loads(json.dumps(req)).get("operation_nonce") or "")
        if not NONCE["value"]:
            # 后端请求体可能把 nonce 嵌在别处，做一次暴力搜索
            s = json.dumps(req)
            import re
            m = re.search(r'"operation_nonce"\s*:\s*"([^"]+)"', s)
            NONCE["value"] = m.group(1) if m else ""
        mode = MODE["mode"]
        if mode == "forged_prod_kid":
            body = make_body(sig_override=b"\x00" * 64, kid=PROD_KEY_ID)      # 垃圾签名 + 生产 key_id
        elif mode == "forged_configured_kid":
            body = make_body(sig_override=b"\x00" * 64, kid=prod_kid)          # 垃圾签名 + 真实 key_id
        elif mode == "valid_dynamic":
            body = make_body(sign_with=prod_priv, kid=prod_kid)                # 动态钥合法签名
        elif mode == "valid_builtin":
            body = make_body(sign_with=builtin_priv, kid=backend_init.SOFTWARE_AUTH_SIGNING_KEY_ID)
        elif mode == "forged_builtin_kid":
            body = make_body(sig_override=b"\x00" * 64, kid=backend_init.SOFTWARE_AUTH_SIGNING_KEY_ID)
        elif mode == "valid_unknown_key":
            body = make_body(sign_with=other_priv, kid="deadbeefdeadbeef")     # 不相干钥
        else:
            body = {"allowed": False, "reason": "denied"}
        if MODE.get("debug"):
            print(f"        [server] 收到 software_key={req.get('software_key')!r} feature_key={req.get('feature_key')!r}"
                  f" nonce={req.get('operation_nonce')!r} -> 签发 feature_key={body.get('feature_key')!r}")
        raw = json.dumps(body).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)
    def log_message(self, *a): pass

srv = HTTPServer(("127.0.0.1", 0), H)
threading.Thread(target=srv.serve_forever, daemon=True).start()
base = f"http://127.0.0.1:{srv.server_port}"

lib = ctypes.CDLL(SO)
lib.set_auth_enabled.argtypes = [ctypes.c_bool]
lib.set_auth_config.argtypes = [ctypes.c_char_p] * 4 + [ctypes.c_int64]
lib.set_access_token.argtypes = [ctypes.c_char_p]
lib.authorize_feature.restype = ctypes.c_void_p
lib.authorize_feature.argtypes = []
lib.free_string.argtypes = [ctypes.c_void_p]

def authorize():
    p = lib.authorize_feature()
    if not p:
        return {"ok": False, "raw": None}
    try:
        s = ctypes.string_at(p).decode()
    finally:
        lib.free_string(p)
    return json.loads(s)

lib.set_auth_enabled(True)
# 客户端把后端下发的动态公钥传进来（生产换钥的正确姿势）
rc = lib.set_auth_config(base.encode(), b"fe", b"answer_parser", prod_pub_b64.encode(), 1)
lib.set_access_token(b"fake-token")
print(f"set_auth_config rc={rc} | 动态公钥 key_id={prod_kid}（与生产内置 key_id {PROD_KEY_ID} 不同）\n")

# 内置默认钥（解析器 obfstr 里那把）对应的私钥：本机后端由 SECRET_KEY 派生，
# 用它签名可验证"不传动态公钥也能走通"（离线兜底场景）
import sys
sys.path.insert(0, os.environ.get("LASTUDIO_BACKEND", "."))
import init as backend_init
builtin_priv = Ed25519PrivateKey.from_private_bytes(backend_init.SOFTWARE_AUTH_SIGNING_SEED)
builtin_pub_b64 = B64(builtin_priv.public_key().public_bytes_raw())
print(f"后端派生的内置钥 key_id={backend_init.SOFTWARE_AUTH_SIGNING_KEY_ID} "
      f"| 与解析器内置默认钥一致: {builtin_pub_b64 == 'iIsgUPrBJidorQGIs-BJCt5EiLrRHVbAWStnNaPn3cI'}\n")

cases = [
    ("① 伪造：生产 key_id + 垃圾签名（修复前会被放行）", "forged_prod_kid", False, prod_pub_b64),
    ("② 伪造：真实 key_id + 垃圾签名", "forged_configured_kid", False, prod_pub_b64),
    ("③ 合法：动态下发公钥签发的 grant（生产换钥场景）", "valid_dynamic", True, prod_pub_b64),
    ("④ 合法签名但来自不相干私钥", "valid_unknown_key", False, prod_pub_b64),
    ("⑤ 合法：后端私钥签发 + 不传动态公钥（走内置默认钥）", "valid_builtin", True, ""),
    ("⑥ 伪造：内置 key_id + 垃圾签名", "forged_builtin_kid", False, ""),
    ("⑦ 伪造：不传动态公钥 + 生产 key_id + 垃圾签名", "forged_prod_kid", False, ""),
    ("⑧ 伪造：配置一把错钥 + 生产 key_id + 垃圾签名", "forged_prod_kid", False, other_pub_b64),
]
ok_all = True
for idx, (label, mode, should_pass, pk) in enumerate(cases):
    MODE["mode"] = mode
    MODE["debug"] = True
    # 用独立 feature_key 隔离各用例（同一 feature 的合法 grant 会被缓存复用 ~120s，属正常行为）
    FEATURE["value"] = f"answer_parser_case{idx}"
    lib.set_auth_config(base.encode(), b"fe", FEATURE["value"].encode(), pk.encode(), 1)
    r = authorize()
    passed = not (isinstance(r, dict) and r.get("ok") is False)
    verdict = "放行(grant 已签发)" if passed else f"拒绝({(r.get('error') or {}).get('code')})"
    good = (passed == should_pass)
    ok_all &= good
    print(f"  {'✅' if good else '❌'} {label}\n       -> {verdict}（期望{'放行' if should_pass else '拒绝'}）  raw={json.dumps(r, ensure_ascii=False)[:150]}")
print("\n结论:", "防护有效 ✅" if ok_all else "仍有问题 ❌")
