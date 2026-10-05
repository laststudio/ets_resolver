#!/usr/bin/env python3
"""从官方题库数据挖掘"官方大题名 → 解析家族"的实证映射（可复现）。

数据源（两者配合）：
  1. laststudio/ets100_resources 的 SQLite 分库（sets / valid_sets / contents）
     —— 提供每套题的 template_url 与 contents[].code_id / group_name / real_id；
  2. 模板包内的 res.json（https://cdn.subject.ets100.com/made/<template_url>.zip，未加密）
     —— 提供 exam_type_list[].exam_type_collect（解析家族）与 exam_list[].exam_id（{{st1}}…）。

关联方式：模板 exam_list[].exam_id 的 `{{st1}}` ↔ contents[].code_id 的 `st1`，
于是得到 (group_name → exam_type_collect) 的官方实证映射。

用法：
    python3 tools/mine_official_map.py <ets100_resources 目录> <输出 json> [每类抽样数]

产出 fixture：tests/fixtures/official_exam_type_map.json（含 collect 取值域、
evaluation_type / engine_type 取值域），供 tests/official_inventory.rs 做门禁。

注意：CDN 返回 default.chinanetcenter.com 证书，需对该精确主机做主机名例外
（脚本内已处理，证书链/有效期仍校验）。
"""
import collections
import concurrent.futures as cf
import http.client
import io
import json
import os
import random
import sqlite3
import ssl
import sys
import zipfile

CDN_HOST = "cdn.subject.ets100.com"


def _ctx() -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.check_hostname = False  # 仅该主机；证书链/有效期仍由默认校验完成
    return ctx


def _norm_exam_id(raw: str) -> str:
    return str(raw or "").strip().lstrip("{").rstrip("}").strip()


def _fetch_template(name: str, cache_dir: str, timeout: int = 120):
    """下载并解出 res.json；返回 (res_json | None, error | None)。"""
    os.makedirs(cache_dir, exist_ok=True)
    cached = os.path.join(cache_dir, f"{name}.json")
    if os.path.exists(cached):
        try:
            return json.load(open(cached, encoding="utf-8")), None
        except Exception:
            pass
    try:
        conn = http.client.HTTPSConnection(CDN_HOST, 443, timeout=timeout, context=_ctx())
        conn.request("GET", f"/made/{name}.zip",
                     headers={"Host": CDN_HOST, "User-Agent": "libcurl-agent/1.0", "Accept": "*/*"})
        resp = conn.getresponse()
        blob = resp.read()
        conn.close()
        if resp.status != 200:
            return None, f"HTTP {resp.status}"
        archive = zipfile.ZipFile(io.BytesIO(blob))
        if "res.json" not in archive.namelist():
            return None, "模板包内无 res.json"
        res = json.loads(archive.read("res.json").decode("utf-8", "replace"))
        json.dump(res, open(cached, "w", encoding="utf-8"), ensure_ascii=False)
        return res, None
    except Exception as exc:  # noqa: BLE001
        return None, f"{type(exc).__name__}: {exc}"


def load_samples(db_dir: str, per_category: int = 3, scan: int = 400, seed: int = 1337):
    random.seed(seed)
    chosen: dict[str, list[dict]] = {}
    contents: dict[str, list[dict]] = {}
    for db in sorted(f for f in os.listdir(db_dir) if f.endswith(".db")):
        con = sqlite3.connect(os.path.join(db_dir, db))
        for sid, name, fcn, turl in con.execute(
            "SELECT id, name, first_column_name, template_url FROM valid_sets "
            "WHERE template_url IS NOT NULL AND template_url != '' ORDER BY RANDOM() LIMIT ?",
            (scan,),
        ):
            bucket = chosen.setdefault((fcn or "").strip(), [])
            if len(bucket) < per_category:
                bucket.append({"set_id": sid, "name": name, "category": (fcn or "").strip(), "template": turl})
        for sid, code, gname, tname, real_id in con.execute(
            "SELECT set_id, code_id, group_name, type_name, real_id FROM contents"
        ):
            contents.setdefault(sid, []).append(
                {"code_id": code, "group_name": gname, "type_name": tname, "real_id": real_id}
            )
        con.close()
    samples = [x for bucket in chosen.values() for x in bucket]
    for s in samples:
        s["contents"] = sorted(contents.get(s["set_id"], []), key=lambda x: str(x["code_id"]))
    return samples


def mine(db_dir: str, out_path: str, per_category: int = 3, cache_dir: str = "tpl_json") -> dict:
    samples = load_samples(db_dir, per_category=per_category)
    pairs = collections.Counter()
    name2collect = collections.Counter()
    collects = collections.Counter()
    evals = collections.Counter()
    engines = collections.Counter()
    errors = collections.Counter()
    ok = 0

    def work(sample):
        res, err = _fetch_template(sample["template"], cache_dir)
        return sample, res, err

    with cf.ThreadPoolExecutor(max_workers=8) as pool:
        for sample, res, err in pool.map(work, samples):
            if err:
                errors[err.split(":")[0]] += 1
                continue
            ok += 1
            cid2group = {c["code_id"]: c["group_name"] for c in sample["contents"] if c.get("code_id")}
            for et in res.get("exam_type_list") or []:
                collect = str(et.get("exam_type_collect") or "")
                name2collect[(str(et.get("exam_type_name") or ""), collect)] += 1
                collects[collect] += 1
                for ex in et.get("exam_list") or []:
                    for q in ex.get("question_list") or []:
                        evals[str(q.get("evaluation_type") or "")] += 1
                        engines[str(q.get("engine_type") or "")] += 1
                    group = cid2group.get(_norm_exam_id(ex.get("exam_id")))
                    if group:
                        pairs[(group, collect)] += 1

    payload = {
        "source": "laststudio/ets100_resources + 模板包 res.json 实证关联",
        "method": "exam_list[].exam_id({{st1}}) ↔ contents[].code_id(st1)",
        "templates_parsed": ok,
        "templates_failed": dict(errors),
        "collect_counts": dict(collects),
        "evaluation_types": dict(evals),
        "engine_types": dict(engines),
        "pairs": [{"group_name": g, "collect": c, "count": n} for (g, c), n in pairs.most_common()],
        "name_to_collect": [{"exam_type_name": n, "collect": c, "count": k} for (n, c), k in name2collect.most_common()],
    }
    json.dump(payload, open(out_path, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(f"模板解析 {ok} 套（失败 {dict(errors)}）")
    print(f"collect 家族 {len(collects)} 个: {dict(collects)}")
    print(f"(group_name, collect) 组合 {len(pairs)} 组；写出 {out_path}")
    return payload


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    mine(sys.argv[1], sys.argv[2], per_category=int(sys.argv[3]) if len(sys.argv) > 3 else 3)
