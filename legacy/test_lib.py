#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
性能测试: 测量 Rust 动态链接库处理实际作业数据的速度。

用法:
    python benchmark_resolver.py <作业根目录路径>

示例:
    python benchmark_resolver.py /path/to/all_homework

测试内容:
    1. 逐个调用 resolve() 对每个任务计时（精确追踪失败项）
    2. 一次性调用 resolve_by_all_homework() 计时（整体吞吐量，3次取最优）
    3. 统计: 成功数、失败数、失败详情、总答案数、处理速度(答案/秒)
"""

import ctypes
import json
import os
import sys
import time
import collections

# ============================================================
# 1. 加载动态链接库
# ============================================================

def find_library():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    candidates = []
    for base in [script_dir, os.path.join(script_dir, "target", "release"),
                 os.path.join(script_dir, "..", "target", "release")]:
        base = os.path.abspath(base)
        candidates.extend([
            os.path.join(base, "libresolver.so"),
            os.path.join(base, "resolver.dll"),
            os.path.join(base, "libresolver.dylib"),
        ])
    for p in candidates:
        if os.path.isfile(p):
            return p
    import ctypes.util
    return ctypes.util.find_library("demo")

lib_path = find_library()
if lib_path is None:
    print("[FATAL] 找不到动态链接库，请先 cargo build --release")
    sys.exit(1)

print(f"[INFO] 加载动态库: {lib_path}")
lib = ctypes.CDLL(lib_path)

# 函数签名
lib.debug_mode.argtypes = [ctypes.c_bool]
lib.debug_mode.restype  = None

lib.nofile_mode.argtypes = [ctypes.c_bool]
lib.nofile_mode.restype  = None

lib.resolve.argtypes = [ctypes.c_char_p]
lib.resolve.restype  = ctypes.c_void_p

lib.resolve_by_homework.argtypes = [ctypes.c_char_p]
lib.resolve_by_homework.restype  = ctypes.c_void_p

lib.resolve_by_all_homework.argtypes = [ctypes.c_char_p]
lib.resolve_by_all_homework.restype  = ctypes.c_void_p

lib.resolver_free_string.argtypes = [ctypes.c_void_p]
lib.resolver_free_string.restype  = None

lib.debug_mode(False)
lib.nofile_mode(False)

# ============================================================
# 2. 辅助函数
# ============================================================

def call_ffi(fn, arg: str):
    """调用 FFI 函数，自动释放返回的字符串"""
    ptr = fn(arg.encode("utf-8"))
    if not ptr:
        return None
    try:
        raw = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
        return json.loads(raw)
    finally:
        lib.resolver_free_string(ptr)


def count_answers(result):
    """统计一个 resolve 结果中的答案条目数（排除 'type' 键）"""
    if result is None:
        return 0
    if isinstance(result, dict):
        return sum(1 for k in result.keys() if k != "type")
    return 0


def get_type(result):
    """获取 structure_type"""
    if result and isinstance(result, dict):
        return result.get("type", "unknown")
    return "failed"


def walk_find_tasks(root_path):
    """
    遍历目录，找到所有包含 content.json 的任务文件夹。
    返回: [(homework_name, task_path), ...]
    """
    tasks = []
    try:
        entries = sorted(os.listdir(root_path))
    except OSError as e:
        print(f"[ERROR] 无法读取目录 {root_path}: {e}")
        return tasks

    for hw_name in entries:
        hw_path = os.path.join(root_path, hw_name)
        if not os.path.isdir(hw_path):
            continue
        # 在 homework 目录下找任务子目录
        try:
            hw_entries = sorted(os.listdir(hw_path))
        except OSError:
            continue
        for task_name in hw_entries:
            task_path = os.path.join(hw_path, task_name)
            if os.path.isdir(task_path):
                if os.path.isfile(os.path.join(task_path, "content.json")):
                    tasks.append((hw_name, task_path))
        # 某些结构可能是 homework 目录本身就含 content.json
        if os.path.isfile(os.path.join(hw_path, "content.json")):
            tasks.append((hw_name, hw_path))
    return tasks


# ============================================================
# 3. 主测试逻辑
# ============================================================

def main():
    if len(sys.argv) < 2:
        print("用法: python benchmark_resolver.py <作业根目录路径>")
        print("示例: python benchmark_resolver.py /path/to/all_homework")
        sys.exit(1)

    root_path = os.path.abspath(sys.argv[1])
    if not os.path.isdir(root_path):
        print(f"[FATAL] 路径不存在或不是目录: {root_path}")
        sys.exit(1)

    print(f"[INFO] 作业根目录: {root_path}")

    # ---- 枚举所有任务 ----
    print("\n[1/3] 枚举任务目录...")
    tasks = walk_find_tasks(root_path)
    print(f"      共找到 {len(tasks)} 个任务")

    if not tasks:
        print("[WARN] 未找到任何包含 content.json 的任务目录")
        sys.exit(0)

    # 按作业分组统计
    hw_groups = collections.Counter()
    for hw_name, _ in tasks:
        hw_groups[hw_name] += 1
    print(f"      分布: {dict(hw_groups)}")

    # ============================================================
    # 逐个 resolve 计时
    # ============================================================
    print("\n[2/3] 逐个调用 resolve() 测试 (精确追踪每项)...")

    success_count = 0
    fail_count = 0
    total_answers = 0
    failures = []          # [(hw_name, task_name, type, detail, full_path)]
    type_stats = collections.Counter()
    type_answer_stats = collections.Counter()
    type_time_stats = collections.defaultdict(list)
    all_timings = []       # [(hw_name, task_path, elapsed, ans_cnt, stype)]

    progress_interval = max(1, len(tasks) // 20)
    start_all = time.perf_counter()

    for idx, (hw_name, task_path) in enumerate(tasks):
        t0 = time.perf_counter()
        result = call_ffi(lib.resolve, task_path)
        elapsed = time.perf_counter() - t0

        ans_cnt = count_answers(result)
        stype = get_type(result)
        all_timings.append((hw_name, os.path.basename(task_path), elapsed, ans_cnt, stype))

        if result is not None:
            success_count += 1
            total_answers += ans_cnt
            type_stats[stype] += 1
            type_answer_stats[stype] += ans_cnt
            type_time_stats[stype].append(elapsed)
        else:
            fail_count += 1
            err_detail = "返回 null"
            content_path = os.path.join(task_path, "content.json")
            if os.path.isfile(content_path):
                try:
                    with open(content_path, "r", encoding="utf-8") as f:
                        content = json.load(f)
                    st = content.get("structure_type", "unknown")
                    err_detail = f"structure_type={st}, 返回 null"
                except Exception as e:
                    err_detail = f"读取content.json失败: {e}"
            failures.append((hw_name, os.path.basename(task_path), stype, err_detail, task_path))

        # 进度
        if (idx + 1) % progress_interval == 0 or idx == 0 or idx == len(tasks) - 1:
            pct = (idx + 1) / len(tasks) * 100
            elapsed_so_far = time.perf_counter() - start_all
            speed = (idx + 1) / elapsed_so_far if elapsed_so_far > 0 else 0
            print(f"      [{idx+1:>5}/{len(tasks)}] {pct:5.1f}%  "
                  f"已用 {elapsed_so_far:.2f}s  速度 {speed:.1f} tasks/s")

    total_time_individual = time.perf_counter() - start_all

    # ============================================================
    # 批量 resolve_by_all_homework 计时
    # ============================================================
    print("\n[3/3] 一次性调用 resolve_by_all_homework() 测试 (整体吞吐量)...")

    best_time = float("inf")
    best_result = None
    for run_idx in range(3):
        t0 = time.perf_counter()
        all_result = call_ffi(lib.resolve_by_all_homework, root_path)
        elapsed = time.perf_counter() - t0
        if elapsed < best_time:
            best_time = elapsed
            best_result = all_result
        print(f"      Run {run_idx+1}/3: {elapsed:.4f}s")

    # 统计批量结果
    bulk_total_answers = 0
    bulk_success = 0
    bulk_fail = 0
    if best_result and isinstance(best_result, dict):
        for hw_name, task_list in best_result.items():
            if isinstance(task_list, list):
                for task_result in task_list:
                    if task_result is not None:
                        bulk_success += 1
                        bulk_total_answers += count_answers(task_result)
                    else:
                        bulk_fail += 1

    # ============================================================
    # 汇总报告
    # ============================================================
    print()
    print("=" * 70)
    print("                        性能测试报告")
    print("=" * 70)

    print(f"\n■ 目录: {root_path}")
    print(f"■ 总任务数: {len(tasks)}")

    # --- 逐个调用结果 ---
    print(f"\n{'─'*40}")
    print(f"  逐个 resolve() 调用结果:")
    print(f"{'─'*40}")
    print(f"  成功:     {success_count}")
    print(f"  失败:     {fail_count}")
    print(f"  总答案数: {total_answers}")
    print(f"  总耗时:   {total_time_individual:.4f}s")
    if total_time_individual > 0:
        print(f"  速度:     {len(tasks) / total_time_individual:.2f} tasks/s")
        print(f"  速度:     {total_answers / total_time_individual:.2f} answers/s")

    # --- 批量调用结果 ---
    print(f"\n{'─'*40}")
    print(f"  resolve_by_all_homework() 批量调用结果:")
    print(f"{'─'*40}")
    print(f"  成功:     {bulk_success}")
    print(f"  失败:     {bulk_fail}")
    print(f"  总答案数: {bulk_total_answers}")
    print(f"  最佳耗时: {best_time:.4f}s (3次取最优)")
    if best_time > 0:
        print(f"  速度:     {len(tasks) / best_time:.2f} tasks/s")
        print(f"  速度:     {bulk_total_answers / best_time:.2f} answers/s")

    # --- 加速比 ---
    if total_time_individual > 0 and best_time > 0:
        speedup = total_time_individual / best_time
        print(f"\n  批量 vs 逐个 加速比: {speedup:.2f}x")

    # --- 按类型统计 ---
    print(f"\n{'─'*70}")
    print(f"  按题目类型统计:")
    print(f"{'─'*70}")
    print(f"  {'类型':<35} {'数量':>6} {'答案数':>8} {'平均耗时':>12} {'总耗时':>10}")
    print(f"  {'-'*35} {'-'*6} {'-'*8} {'-'*12} {'-'*10}")
    for stype in sorted(type_stats.keys()):
        cnt = type_stats[stype]
        ans = type_answer_stats[stype]
        times = type_time_stats[stype]
        avg_t = sum(times) / len(times) if times else 0
        total_t = sum(times)
        print(f"  {stype:<35} {cnt:>6} {ans:>8} {avg_t*1000:>10.3f}ms {total_t*1000:>8.1f}ms")

    # --- 失败详情 ---
    if failures:
        print(f"\n{'─'*70}")
        print(f"  失败详情 ({len(failures)} 项):")
        print(f"{'─'*70}")
        for i, (hw, task, stype, detail, full_path) in enumerate(failures):
            print(f"  [{i+1}] {hw}/{task}")
            print(f"      类型: {stype}")
            print(f"      原因: {detail}")
            print(f"      路径: {full_path}")
    else:
        print(f"\n  ✅ 无失败项")

    # --- 最慢 / 最快 ---
    print(f"\n{'─'*70}")
    print(f"  最慢的 5 个任务:")
    print(f"{'─'*70}")
    sorted_slow = sorted(all_timings, key=lambda x: x[2], reverse=True)
    for i, (hw, task, elapsed, ans_cnt, stype) in enumerate(sorted_slow[:5]):
        print(f"  [{i+1}] {hw}/{task}  {elapsed*1000:.2f}ms  type={stype}  answers={ans_cnt}")

    print(f"\n{'─'*70}")
    print(f"  最快的 5 个任务:")
    print(f"{'─'*70}")
    sorted_fast = sorted(all_timings, key=lambda x: x[2])
    for i, (hw, task, elapsed, ans_cnt, stype) in enumerate(sorted_fast[:5]):
        print(f"  [{i+1}] {hw}/{task}  {elapsed*1000:.3f}ms  type={stype}  answers={ans_cnt}")

    # --- 最终汇总 ---
    print(f"\n{'='*70}")
    print("  最终汇总:")
    print(f"{'='*70}")
    if total_time_individual > 0:
        print(f"    逐个调用:  {success_count} 成功 / {fail_count} 失败  |  "
              f"{total_answers} 答案  |  {total_time_individual:.4f}s  |  "
              f"{total_answers / total_time_individual:.2f} answers/s")
    if best_time > 0:
        print(f"    批量调用:  {bulk_success} 成功 / {bulk_fail} 失败  |  "
              f"{bulk_total_answers} 答案  |  {best_time:.4f}s  |  "
              f"{bulk_total_answers / best_time:.2f} answers/s")
    if total_time_individual > 0 and best_time > 0:
        print(f"    加速比:    {total_time_individual / best_time:.2f}x")
    print(f"{'='*70}")


if __name__ == "__main__":
    main()
