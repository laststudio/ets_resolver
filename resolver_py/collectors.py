# -*- coding: utf-8 -*-
"""各题型解析器

调研结论 (20 套真实试卷验证):
- 任务内答题顺序: choose 用 xtlist[].xt_xh, fill 用 std[].xth,
  role/dialogue/3q5a 用 question[].xh, 均从 1 连续
- 全局题号: content2.json 的 xt_nr/ask 带 "N." 前缀; content.json 只有 ets_thN 任务内编号
- 答案位置: choose=xtlist[].answer, fill=std[].value, picture=std[].value,
  role/dialogue/3q5a=question[].std[].value
- read 无标准答案, 原文在 info.value
"""
from .cleaner import (
    clean, clean_lines, split_br, split_keywords,
    extract_ets_th, strip_ets_th, extract_global_no, strip_global_no,
)

TYPE_CHOOSE = "collector.choose"
TYPE_READ = "collector.read"
TYPE_WORD = "collector.word"
TYPE_REPEAT = "collector.repeat"
TYPE_REPEAT_ESSAY = "collector.repeat_essay"
TYPE_REPEAT_DIALOGUE = "collector.repeat_dialogue"
TYPE_ESSAY = "collector.essay"
TYPE_DIALOGUE = "collector.dialogue"
TYPE_ROLE = "collector.role"
TYPE_PICTURE = "collector.picture"
TYPE_FILL = "collector.fill"
TYPE_3Q5A = "collector.3q5a"

QUESTION_TYPES = {TYPE_ROLE, TYPE_DIALOGUE, TYPE_3Q5A,
                  TYPE_REPEAT, TYPE_REPEAT_ESSAY, TYPE_REPEAT_DIALOGUE, TYPE_ESSAY}


def _make_question(no, no_source, qtype, stid, question, answers,
                   options=None, important=None, extra=None):
    q = {
        "no": no,
        "type": qtype,
        "stid": stid,
        "question": question,
        "answers": answers,
        "options": options or [],
        "important": important or [],
    }
    if extra:
        q.update(extra)
    q["_no_source"] = no_source
    return q


def _stid_of(content):
    info = content.get("info") if isinstance(content, dict) else {}
    if not isinstance(info, dict):
        return None
    try:
        return int(info.get("stid"))
    except (TypeError, ValueError):
        return None


def _pick_question_text(primary, secondary):
    """优先 content2 文本(带全局题号), 去掉题号前缀; 否则 content 文本去掉 ets_thN"""
    if primary:
        text = strip_global_no(primary)
        text = strip_ets_th(text)
        if text:
            return text
    if secondary:
        return strip_ets_th(secondary)
    return ""


def collect_choose(content, content2=None, info=None):
    """北京: 听后选择, info.xtlist[] 每题 {xt_xh, xt_nr, answer, xxlist[]}"""
    stid = _stid_of(content)
    info_obj = content.get("info") or {}
    xtlist = info_obj.get("xtlist") or []
    info2 = (content2 or {}).get("info") or {}
    xtlist2 = info2.get("xtlist") or []
    questions = []
    for i, xt in enumerate(xtlist):
        xt2 = xtlist2[i] if i < len(xtlist2) else {}
        text_src = xt2.get("xt_nr") or xt.get("xt_nr", "")
        no = extract_global_no(text_src)
        no_source = "global" if no is not None else "local"
        if no is None:
            no = extract_ets_th(text_src)
            if no is None:
                try:
                    no = int(xt.get("xt_xh", i + 1))
                except (TypeError, ValueError):
                    no = i + 1
        question_text = _pick_question_text(xt2.get("xt_nr"), xt.get("xt_nr", ""))
        options = [
            f"{o.get('xx_mc', '')}. {clean(o.get('xx_nr', ''))}"
            for o in (xt.get("xxlist") or [])
            if clean(o.get("xx_nr", "")) or o.get("xx_mc")
        ]
        questions.append(_make_question(
            no=no, no_source=no_source, qtype=TYPE_CHOOSE, stid=stid,
            question=question_text,
            answers=[clean(xt.get("answer", ""))],
            options=options,
        ))
    return questions


def collect_question_based(content, content2=None, info=None, qtype=None):
    """role/dialogue/3q5a/repeat: info.question[] 每题 {xh, ask, std[], keywords}"""
    stid = _stid_of(content)
    info_obj = content.get("info") or {}
    questions_arr = info_obj.get("question") or info_obj.get("questions") or []
    info2 = (content2 or {}).get("info") or {}
    questions2 = info2.get("question") or info2.get("questions") or []
    result = []
    for i, q in enumerate(questions_arr):
        q2 = questions2[i] if i < len(questions2) else {}
        ask_src = q2.get("ask") or q.get("ask", "")
        no = extract_global_no(ask_src)
        no_source = "global" if no is not None else "local"
        if no is None:
            no = extract_ets_th(ask_src)
        if no is None:
            try:
                no = int(q.get("xh", i + 1))
            except (TypeError, ValueError):
                no = i + 1
        question_text = _pick_question_text(q2.get("ask"), q.get("ask", ""))
        answers = [clean(s.get("value", "")) for s in (q.get("std") or [])]
        answers = [a for a in answers if a]
        important = split_keywords(q.get("keywords", ""))
        extra = {}
        if q.get("role"):
            extra["role"] = q.get("role")
        if q.get("answer"):
            extra["ref_answer"] = clean(q.get("answer"))
        result.append(_make_question(
            no=no, no_source=no_source,
            qtype=qtype or TYPE_ROLE, stid=stid,
            question=question_text,
            answers=answers,
            important=important,
            extra=extra or None,
        ))
    return result


def collect_fill(content, content2=None, info=None):
    """北京高中: 听后记录, info.std[] 每题 {xth, value}"""
    stid = _stid_of(content)
    info_obj = content.get("info") or {}
    std = info_obj.get("std") or []
    result = []
    for i, s in enumerate(std):
        try:
            no = int(s.get("xth", i + 1))
        except (TypeError, ValueError):
            no = i + 1
        result.append(_make_question(
            no=no, no_source="local", qtype=TYPE_FILL, stid=stid,
            question="",
            answers=[clean(s.get("value", ""))],
        ))
    return result


def collect_picture(content, content2=None, info=None):
    """信息转述/看图: info.std[] 多候选答案, topic 为主题, keypoint/analyze 为要点"""
    stid = _stid_of(content)
    info_obj = content.get("info") or {}
    std = info_obj.get("std") or []
    answers = [clean(s.get("value", "")) for s in std]
    answers = [a for a in answers if a]
    important = split_keywords(info_obj.get("keypoint", ""), sep="</br>")
    important += split_br(info_obj.get("analyze", ""))
    return [_make_question(
        no=None, no_source=None, qtype=TYPE_PICTURE, stid=stid,
        question=clean(info_obj.get("topic", "")) or clean(info_obj.get("value", ""))[:200],
        answers=answers,
        important=important,
    )]


def collect_read(content, content2=None, info=None):
    """模仿朗读/短文朗读: 无标准答案, 原文在 info.value, analyze 为问题列表"""
    stid = _stid_of(content)
    info_obj = content.get("info") or {}
    important = split_br(info_obj.get("analyze", ""))
    return [_make_question(
        no=None, no_source=None, qtype=TYPE_READ, stid=stid,
        question=clean_lines(info_obj.get("value", "")),
        answers=[],
        important=important,
    )]


def collect_generic(content, content2=None, info=None, qtype=None):
    """未知类型兜底: 按结构探测"""
    info_obj = content.get("info") if isinstance(content, dict) else {}
    if not isinstance(info_obj, dict):
        return []
    if info_obj.get("question") or info_obj.get("questions"):
        return collect_question_based(content, content2, info, qtype=qtype)
    if info_obj.get("std"):
        if isinstance(info_obj["std"], list) and info_obj["std"] and \
                isinstance(info_obj["std"][0], dict) and "xth" in info_obj["std"][0]:
            return collect_fill(content, content2, info)
        return collect_picture(content, content2, info)
    if info_obj.get("xtlist"):
        return collect_choose(content, content2, info)
    if info_obj.get("value"):
        return collect_read(content, content2, info)
    return []


def dispatch(structure_type):
    st = structure_type or ""
    if "choose" in st:
        return collect_choose
    if st in (TYPE_READ, TYPE_WORD):
        return collect_read
    if "read" in st or "word" in st:
        return collect_read
    if "fill" in st:
        return collect_fill
    if "picture" in st:
        return collect_picture
    if st in QUESTION_TYPES:
        return lambda c, c2=None, i=None: collect_question_based(c, c2, i, qtype=st)
    if "repeat" in st or "essay" in st or "dialogue" in st or "role" in st or "3q5a" in st:
        return lambda c, c2=None, i=None: collect_question_based(c, c2, i, qtype=st)
    return lambda c, c2=None, i=None: collect_generic(c, c2, i, qtype=st)


def parse_content(content, content2=None, info=None):
    """解析单个 content.json, 返回题目列表"""
    structure_type = (content or {}).get("structure_type", "")
    collector = dispatch(structure_type)
    return collector(content, content2, info)
