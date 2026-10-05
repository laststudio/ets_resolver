#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
性能测试: 测量原生 Python demo.py 处理实际作业数据的速度。

用法:
    python benchmark_demo_py.py <作业根目录路径>

示例:
    python benchmark_demo_py.py /path/to/all_homework

测试内容:
    1. 逐个调用 resolve() 对每个任务计时（精确追踪失败项）
    2. 一次性调用 resolve_by_all_homework() 计时（整体吞吐量，3次取最优）
    3. 按作业分组调用 resolve_by_homework() 计时
    4. 统计: 成功数、失败数、失败详情、总答案数、处理速度(答案/秒)
"""

import json
import os
import sys
import time
import collections

# ============================================================
# 1. 导入 demo.py 中的函数
# ============================================================

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPT_DIR)

try:
    import demo as resolver
except Exception as e:
    print(f"[FATAL] 无法导入 demo.py: {e}")
    print("        请确保 demo.py 与本脚本在同一目录，或在 PYTHONPATH 中")
    sys.exit(1)

# 确保处于文件模式、关闭调试输出
resolver.nofile_mode(False)
resolver.debug_mode(False)

# ============================================================
# 2. 辅助函数
# ============================================================

def count_answers(result):
    """
    统计一个 resolve 结果中的答案条目数（排除 'type' 键）。
    resolve 成功返回 dict，失败返回 -1。
    """
    if result is None or result == -1:
        return 0
    if isinstance(result, dict):
        return sum(1 for k in result.keys() if k != "type")
    return 0


def get_type(result):
    """获取 structure_type"""
    if result is not None and result != -1 and isinstance(result, dict):
        return result.get("type", "unknown")
    return "failed"


def walk_find_tasks(root_path):
    """
    遍历目录，找到所有包含 content.json 的任务文件夹。
    与 demo.py 中 resolve_by_homework 的查找逻辑保持一致：
      for item in os.listdir(TARGET):
          item_path = os.path.join(TARGET, item)
          for root, dirs, files in os.walk(item_path):
              if "content.json" in files:
                  found = True; break
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
        # 与 Python demo.py 逻辑一致：os.walk 递归查找 content.json
        for root, dirs, files in os.walk(hw_path):
            if "content.json" in files:
                tasks.append((hw_name, root))
    return tasks


def get_failure_detail(task_path):
    """读取 content.json 获取失败详情"""
    content_path = os.path.join(task_path, "content.json")
    if os.path.isfile(content_path):
        try:
            with open(content_path, "r", encoding="utf-8") as f:
                content = json.load(f)
            st = content.get("structure_type", "unknown")
            return f"structure_type={st}, resolve 返回 -1"
        except Exception as e:
            return f"读取content.json失败: {e}"
    return "content.json 不存在"


# ============================================================
# 3. 主测试逻辑
# ============================================================

def main():
    if len(sys.argv) < 2:
        print("用法: python benchmark_demo_py.py <作业根目录路径>")
        print("示例: python benchmark_demo_py.py /path/to/all_homework")
        sys.exit(1)

    root_path = os.path.abspath(sys.argv[1])
    if not os.path.isdir(root_path):
        print(f"[FATAL] 路径不存在或不是目录: {root_path}")
        sys.exit(1)

    print(f"[INFO] 作业根目录: {root_path}")
    print(f"[INFO] 使用原生 Python demo.py ({getattr(resolver, '__file__', 'demo.py')})")

    # ============================================================
    # 枚举所有任务
    # ============================================================
    print("\n[1/4] 枚举任务目录...")
    tasks = walk_find_tasks(root_path)
    print(f"      共找到 {len(tasks)} 个任务")

    if not tasks:
        print("[WARN] 未找到任何包含 content.json 的任务目录")
        sys.exit(0)

    # 按作业分组统计
    hw_groups = collections.Counter()
    for hw_name, _ in tasks:
        hw_groups[hw_name] += 1
    print(f"      作业分布: {dict(hw_groups)}")

    # ============================================================
    # 逐个 resolve 计时
    # ============================================================
    print("\n[2/4] 逐个调用 resolve() 测试 (精确追踪每项)...")

    success_count = 0
    fail_count = 0
    total_answers = 0
    failures = []
    type_stats = collections.Counter()
    type_answer_stats = collections.Counter()
    type_time_stats = collections.defaultdict(list)
    all_timings = []

    progress_interval = max(1, len(tasks) // 20)
    start_all = time.perf_counter()

    for idx, (hw_name, task_path) in enumerate(tasks):
        t0 = time.perf_counter()
        try:
            result = resolver.resolve(task_path)
        except Exception as e:
            result = -1
            fail_count += 1
            failures.append((
                hw_name, os.path.basename(task_path), "exception",
                f"异常: {e}", task_path
            ))
            all_timings.append((hw_name, os.path.basename(task_path),
                                time.perf_counter() - t0, 0, "exception"))
            continue
        elapsed = time.perf_counter() - t0

        ans_cnt = count_answers(result)
        stype = get_type(result)
        all_timings.append((hw_name, os.path.basename(task_path), elapsed, ans_cnt, stype))

        if result is not None and result != -1:
            success_count += 1
            total_answers += ans_cnt
            type_stats[stype] += 1
            type_answer_stats[stype] += ans_cnt
            type_time_stats[stype].append(elapsed)
        else:
            fail_count += 1
            detail = get_failure_detail(task_path)
            failures.append((hw_name, os.path.basename(task_path), stype, detail, task_path))

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
    print("\n[3/4] 一次性调用 resolve_by_all_homework() 测试 (整体吞吐量)...")
    best_time_all = float("inf")
    best_result_all = None
    for run_idx in range(3):
        t0 = time.perf_counter()
        try:
            all_result = resolver.resolve_by_all_homework(root_path)
        except Exception as e:
            print(f"      Run {run_idx+1}/3: 异常 - {e}")
            continue
        elapsed = time.perf_counter() - t0
        if elapsed < best_time_all:
            best_time_all = elapsed
            best_result_all = all_result
        print(f"      Run {run_idx+1}/3: {elapsed:.4f}s")

    # 统计批量结果
    bulk_total_answers = 0
    bulk_success = 0
    bulk_fail = 0
    if best_result_all and isinstance(best_result_all, dict):
        for hw_name, task_list in best_result_all.items():
            if isinstance(task_list, list):
                for task_result in task_list:
                    if task_result is not None and task_result != -1:
                        bulk_success += 1
                        bulk_total_answers += count_answers(task_result)
                    else:
                        bulk_fail += 1

    # ============================================================
    # 按作业分组 resolve_by_homework 计时
    # ============================================================
    print("\n[4/4] 按作业分组调用 resolve_by_homework() 测试...")
    hw_names = sorted(hw_groups.keys())
    hw_timings = []  # [(hw_name, elapsed, answer_count, task_count)]
    for hw_name in hw_names:
        hw_path = os.path.join(root_path, hw_name)
        t0 = time.perf_counter()
        try:
            hw_result = resolver.resolve_by_homework(hw_path)
        except Exception as e:
            print(f"      {hw_name}: 异常 - {e}")
            hw_timings.append((hw_name, 0, 0, 0))
            continue
        elapsed = time.perf_counter() - t0
        hw_ans = 0
        hw_tasks = 0
        if isinstance(hw_result, list):
            for item in hw_result:
                if item is not None and item != -1:
                    hw_tasks += 1
                    hw_ans += count_answers(item)
        hw_timings.append((hw_name, elapsed, hw_ans, hw_tasks))
        print(f"      {hw_name}: {elapsed:.4f}s  ({hw_tasks} 任务, {hw_ans} 答案)")

    # ============================================================
    # 汇总报告
    # ============================================================
    print()
    print("=" * 70)
    print("                    Python demo.py 性能测试报告")
    print("=" * 70)

    print(f"\n■ 目录: {root_path}")
    print(f"■ 总任务数: {len(tasks)}")

    # --- 逐个调用结果 ---
    print(f"\n{'─'*50}")
    print(f"  逐个 resolve() 调用结果:")
    print(f"{'─'*50}")
    print(f"  成功:     {success_count}")
    print(f"  失败:     {fail_count}")
    print(f"  总答案数: {total_answers}")
    print(f"  总耗时:   {total_time_individual:.4f}s")
    if total_time_individual > 0:
        print(f"  速度:     {len(tasks) / total_time_individual:.2f} tasks/s")
        print(f"  速度:     {total_answers / total_time_individual:.2f} answers/s")

    # --- 批量调用结果 ---
    print(f"\n{'─'*50}")
    print(f"  resolve_by_all_homework() 批量调用结果:")
    print(f"{'─'*50}")
    print(f"  成功:     {bulk_success}")
    print(f"  失败:     {bulk_fail}")
    print(f"  总答案数: {bulk_total_answers}")
    print(f"  最佳耗时: {best_time_all:.4f}s (3次取最优)")
    if best_time_all > 0 and best_time_all != float('inf'):
        print(f"  速度:     {len(tasks) / best_time_all:.2f} tasks/s")
        print(f"  速度:     {bulk_total_answers / best_time_all:.2f} answers/s")

    # --- 加速比 ---
    if total_time_individual > 0 and best_time_all > 0 and best_time_all != float('inf'):
        speedup = total_time_individual / best_time_all
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

    # --- 按作业统计 ---
    print(f"\n{'─'*70}")
    print(f"  按作业分组统计 (resolve_by_homework):")
    print(f"{'─'*70}")
    print(f"  {'作业名':<25} {'任务数':>6} {'答案数':>8} {'耗时':>10} {'速度':>14}")
    print(f"  {'-'*25} {'-'*6} {'-'*8} {'-'*10} {'-'*14}")
    for hw_name, elapsed, ans, task_cnt in hw_timings:
        speed_str = f"{ans/elapsed:.1f} ans/s" if elapsed > 0 else "N/A"
        print(f"  {hw_name:<25} {task_cnt:>6} {ans:>8} {elapsed*1000:>8.1f}ms {speed_str:>14}")

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
    if best_time_all > 0 and best_time_all != float('inf'):
        print(f"    批量调用:  {bulk_success} 成功 / {bulk_fail} 失败  |  "
              f"{bulk_total_answers} 答案  |  {best_time_all:.4f}s  |  "
              f"{bulk_total_answers / best_time_all:.2f} answers/s")
    if total_time_individual > 0 and best_time_all > 0 and best_time_all != float('inf'):
        print(f"    加速比:    {total_time_individual / best_time_all:.2f}x")
    print(f"{'='*70}")


if __name__ == "__main__":
    main()
