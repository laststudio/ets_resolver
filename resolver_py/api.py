# -*- coding: utf-8 -*-
"""统一接口: 单任务解析 / 批量解析 / 时间聚类 / 元信息

固定输出格式 (每题):
{
    "no": 5,                    # 全局题号 (content2.json 提取为主, 顺序兜底)
    "type": "collector.choose", # 题型 structure_type
    "stid": 172909,             # 任务 stid (排序键)
    "question": "...",          # 题干 (清洗后)
    "answers": [...],           # 答案 (多候选)
    "options": [...],           # 选项 (仅 choose)
    "important": [...],         # 关键词/解析
}
"""
import json

from .collectors import (
    parse_content, _stid_of, collect_word_pk,
    build_pk_answer, build_pk_wrong_choice, build_pk_answer_text,
)

DEFAULT_TIME_THRESHOLD_MS = 2000


def parse_task(content_str, info_str=None, content2_str=None):
    """单任务解析: 输入 JSON 字符串或对象/数组, 返回题目列表 (list[dict])"""
    content = json.loads(content_str) if isinstance(content_str, str) else content_str
    content2 = json.loads(content2_str) if content2_str else None
    info = json.loads(info_str) if info_str else None
    return parse_content(content, content2, info)


def parse_word_pk(content_data):
    """词汇作业与单词 PK 解析: 输入 JSON 字符串或数据对象/列表, 返回题目列表"""
    data = json.loads(content_data) if isinstance(content_data, str) else content_data
    return collect_word_pk(data)


def extract_meta(content_str):
    """元信息: {stid, structure_type, question_count}, 供前端排序/聚类参考"""
    content = json.loads(content_str) if isinstance(content_str, str) else content_str
    questions = parse_content(content, None, None)
    if isinstance(content, list):
        stid = content[0].get("word_id") or content[0].get("id") if (content and isinstance(content[0], dict)) else None
        stype = "word_pk"
    elif isinstance(content, dict):
        stid = _stid_of(content) or content.get("id") or content.get("word_id")
        stype = content.get("structure_type", "word_pk" if ("word_questions" in content or "data" in content) else "")
    else:
        stid = None
        stype = ""
    return {
        "stid": stid,
        "structure_type": stype,
        "question_count": len(questions),
    }


def _assign_fallback_numbers(questions):
    """小节内补号: 有全局题号的保留, 无题号的按前一题号+1 兜底 (order 兜底)"""
    next_no = 1
    for q in questions:
        no_source = q.pop("_no_source", None)
        if no_source == "global" and q["no"] is not None:
            next_no = max(next_no, q["no"] + 1)
        elif q["no"] is not None:
            next_no = max(next_no, q["no"] + 1)
        else:
            q["no"] = next_no
            next_no += 1
    return questions


def _order_questions_in_section(questions):
    """小节内排序: 按题号升序; 无题号的保持出现顺序 (稳定排序)"""
    def key(iq):
        i, q = iq
        no = q.get("no")
        if no is None:
            return (1, 0, i)
        return (0, no, i)
    ordered = sorted(enumerate(questions), key=key)
    return [q for _, q in ordered]


def parse_batch(tasks, sort_by_stid=True):
    """批量解析: tasks = [{content, info, content2, order?}, ...]
    返回: {
        "sections": [
            {"type": "collector.choose", "questions": [
                {"no": 1, "question": ..., "answers": [...], "options": [...], "important": [...]},
                ...
            ]},
            ...
        ]
    }
    - 任务按 stid 升序 (套内真实顺序, 20 套卷调研结论)
    - 相同题型合并为一个小节, 小节顺序 = 其第一个任务的 stid 序
    - 小节内题目按题号排序 (content2 全局题号 > 任务内题号 > 顺序兜底)
    """
    parsed = []
    for idx, task in enumerate(tasks):
        if isinstance(task, str):
            task = {"content": task}
        questions = parse_task(
            task.get("content"),
            task.get("info"),
            task.get("content2"),
        )
        order = task.get("order")
        if order is None:
            order = idx
        for q in questions:
            q["_order"] = order
        parsed.append((questions, order))
    if sort_by_stid:
        parsed.sort(key=lambda pair: (
            pair[0][0]["stid"] if pair[0] else float("inf"),
            pair[1],
        ))
    sections = []
    for questions, _ in parsed:
        for q in questions:
            qtype = q["type"]
            if sections and sections[-1]["type"] == qtype:
                sections[-1]["questions"].append(q)
            else:
                sections.append({
                    "type": qtype,
                    "questions": [q],
                })
    for section in sections:
        section["questions"] = _assign_fallback_numbers(section["questions"])
        section["questions"] = _order_questions_in_section(section["questions"])
        for q in section["questions"]:
            q.pop("_order", None)
    return {"sections": sections}


def group_by_time(items, threshold_ms=DEFAULT_TIME_THRESHOLD_MS):
    """时间聚类: 题目平铺目录按修改时间分组为试卷
    items: [(id, mtime_ms), ...]
    返回: [[id, ...], ...] 按时间升序
    """
    if not items:
        return []
    ordered = sorted(items, key=lambda x: x[1])
    groups = []
    current = [ordered[0][0]]
    prev_time = ordered[0][1]
    for ident, mtime in ordered[1:]:
        if abs(mtime - prev_time) <= threshold_ms:
            current.append(ident)
        else:
            groups.append(current)
            current = [ident]
        prev_time = mtime
    groups.append(current)
    return groups
