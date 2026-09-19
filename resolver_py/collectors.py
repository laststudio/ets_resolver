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
import json
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

# 单词 PK 与词汇作业题型 (逆向自 SyncWordQuestionBean)
TYPE_PK_CHOOSE = "word.choice"            # 6: 听选/词义选择
TYPE_PK_WRITE = "word.write"              # 16: 看释义写单词 (拼写)
TYPE_PK_READ = "word.read"                # 17: 朗读单词
TYPE_PK_DICTATION = "word.dictation"      # 18: 听写单词
TYPE_PK_EN_CN = "word.translate_en_cn"    # 19: 英译中
TYPE_PK_CN_EN = "word.translate_cn_en"    # 20: 中译英

PK_TYPE_MAP = {
    6: TYPE_PK_CHOOSE,
    16: TYPE_PK_WRITE,
    17: TYPE_PK_READ,
    18: TYPE_PK_DICTATION,
    19: TYPE_PK_EN_CN,
    20: TYPE_PK_CN_EN,
}

QUESTION_TYPES = {TYPE_ROLE, TYPE_DIALOGUE, TYPE_3Q5A,
                  TYPE_REPEAT, TYPE_REPEAT_DIALOGUE, TYPE_ESSAY}


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


def parse_pk_options(option_data):
    """解析 PK 选择题选项，支持 JSON 字符串或 list[dict]
    返回格式化列表: ['A. 苹果', 'B. 香蕉', ...]
    """
    if not option_data:
        return []
    if isinstance(option_data, str):
        try:
            option_data = json.loads(option_data)
        except Exception:
            return []
    if not isinstance(option_data, list):
        return []
    options = []
    for opt in option_data:
        if not isinstance(opt, dict):
            continue
        name = clean(opt.get("option_name") or opt.get("name") or "")
        val = clean(opt.get("option_value") or opt.get("value") or "")
        if name and val:
            options.append(f"{name}. {val}")
        elif val:
            options.append(val)
        elif name:
            options.append(name)
    return options


def _parse_pk_raw_options(option_data):
    """返回原始选项结构 [{'name': 'A', 'value': 'apple'}, ...]"""
    if not option_data:
        return []
    if isinstance(option_data, str):
        try:
            option_data = json.loads(option_data)
        except Exception:
            return []
    if not isinstance(option_data, list):
        return []
    raw = []
    for opt in option_data:
        if isinstance(opt, dict):
            name = clean(opt.get("option_name") or opt.get("name") or "")
            val = clean(opt.get("option_value") or opt.get("value") or "")
            if name or val:
                raw.append({"name": name, "value": val})
    return raw


def build_pk_answer_text(question, word_map=None):
    """根据题型与词典生成 PK/词汇题目标准答案文本 (逆向自 SyncWordQuestionBean.java)
    - 6  (听选/选择): question.answer
    - 16 (写单词/拼写): word.content
    - 17 (朗读单词): word.content
    - 18 (听写单词): word.content
    - 19 (英译中): word.translate
    - 20 (中译英): word.content
    """
    qtype = int(question.get("question_type") or question.get("type") or question.get("q_type") or 0)
    word_id = int(question.get("word_id") or 0)
    ans = clean(question.get("answer") or "")

    word_info = (word_map or {}).get(word_id, {})
    content = clean(word_info.get("content") or question.get("word") or question.get("content") or "")
    trans = clean(word_info.get("translate") or question.get("translate") or "")

    if qtype == 6:
        return ans
    elif qtype in (16, 17, 18, 20):
        return content or ans
    elif qtype == 19:
        return trans or ans
    else:
        return ans or content


def build_pk_answer(q_type, q_info, word_map=None):
    """构建用于提交 PK 的单题答案结构: {'point': 1, 'answer': '...', 'word_id': ..., 'question_id': ...}"""
    q = dict(q_info)
    q["question_type"] = q_type
    ans_text = build_pk_answer_text(q, word_map)
    word_id = int(q.get("word_id") or 0)
    q_id = int(q.get("q_id") or q.get("question_id") or q.get("id") or 0)
    return {
        "point": 1,
        "answer": ans_text,
        "word_id": word_id,
        "question_id": q_id,
    }


def build_pk_wrong_choice(question):
    """为选择题智能构建错误答案（保持与 answer 是选项名或选项值格式一致，用于控分场景）"""
    ans = clean(question.get("answer") or "")
    raw_opts = _parse_pk_raw_options(question.get("option") or question.get("options"))
    if not raw_opts:
        return "X"

    is_opt_name = any(o["name"] == ans for o in raw_opts)
    is_opt_val = any(o["value"] == ans for o in raw_opts)

    wrong_candidates = []
    for o in raw_opts:
        if is_opt_name:
            if o["name"] and o["name"] != ans:
                wrong_candidates.append(o["name"])
        elif is_opt_val:
            if o["value"] and o["value"] != ans:
                wrong_candidates.append(o["value"])
        else:
            if o["value"] and o["value"] != ans:
                wrong_candidates.append(o["value"])
            elif o["name"] and o["name"] != ans:
                wrong_candidates.append(o["name"])

    return wrong_candidates[0] if wrong_candidates else "X"


def collect_repeat_essay(content, content2=None, info=None):
    """短文逐句跟读: 优先遍历 info.sublist, 每句作为一道题目"""
    stid = _stid_of(content)
    info_obj = content.get("info") if isinstance(content, dict) else {}
    if not isinstance(info_obj, dict):
        info_obj = {}
    sublist = info_obj.get("sublist") or []
    if isinstance(sublist, list) and sublist:
        questions = []
        for i, sub in enumerate(sublist):
            if isinstance(sub, dict):
                text = clean_lines(sub.get("text") or sub.get("value") or sub.get("content") or "")
                if text:
                    questions.append(_make_question(
                        no=i + 1,
                        no_source="local",
                        qtype=TYPE_REPEAT_ESSAY,
                        stid=stid,
                        question=text,
                        answers=[],
                        important=[],
                    ))
        if questions:
            return questions
    # 回退到问答型或整篇朗读
    if info_obj.get("question") or info_obj.get("questions"):
        return collect_question_based(content, content2, info, qtype=TYPE_REPEAT_ESSAY)
    return collect_read(content, content2, info)


def collect_word_pk(content, content2=None, info=None):
    """单词 PK 对战与词汇作业解析器
    兼容多种输入形态:
    1. word/struct 形式: {"data": [{"id": 1, "content": "apple", "translate": "苹果", "question": [...]}]}
    2. pk_info 形式: {"word_questions": [...], "words": [...]}
    3. 纯题目列表: [{"question_type": 19, "word": "apple", "translate": "苹果", "answer": "..."}, ...]
    4. 封装任务形式: {"structure_type": "word_pk", "info": ...}
    """
    root = content
    if isinstance(root, dict):
        if "info" in root and isinstance(root["info"], (dict, list)):
            root = root["info"]

    word_map = {}
    raw_questions = []

    # 提取单词字典 words
    words = []
    if isinstance(root, dict):
        words = root.get("words") or root.get("word") or []
    elif isinstance(content, dict):
        words = content.get("words") or content.get("word") or []

    if isinstance(words, list):
        for w in words:
            if isinstance(w, dict):
                wid = w.get("id") or (w.get("struct") or {}).get("id")
                if wid is not None:
                    word_map[int(wid)] = {
                        "content": clean(w.get("content") or (w.get("struct") or {}).get("content") or ""),
                        "translate": clean(w.get("translate") or (w.get("struct") or {}).get("translate") or ""),
                    }

    # 提取 word/struct 嵌套形态
    data = None
    if isinstance(root, dict):
        data = root.get("data")
    elif isinstance(root, list):
        data = root

    if isinstance(data, list) and data and isinstance(data[0], dict) and "question" in data[0]:
        for item in data:
            if not isinstance(item, dict):
                continue
            wid = item.get("id")
            w_content = clean(item.get("content") or "")
            w_trans = clean(item.get("translate") or "")
            if wid is not None:
                word_map[int(wid)] = {"content": w_content, "translate": w_trans}
            for q in (item.get("question") or []):
                if isinstance(q, dict):
                    q_copy = dict(q)
                    if wid is not None and not q_copy.get("word_id"):
                        q_copy["word_id"] = wid
                    if w_content and not q_copy.get("word"):
                        q_copy["word"] = w_content
                    if w_trans and not q_copy.get("translate"):
                        q_copy["translate"] = w_trans
                    raw_questions.append(q_copy)

    # 提取扁平题目形态
    if not raw_questions:
        qs = None
        if isinstance(root, dict):
            qs = root.get("word_question") or root.get("word_questions") or root.get("questions") or root.get("list")
        elif isinstance(root, list):
            qs = root
        if qs is None and isinstance(content, dict):
            qs = content.get("word_question") or content.get("word_questions") or content.get("questions")
        if isinstance(qs, list):
            for q in qs:
                if isinstance(q, dict):
                    raw_questions.append(q)

    questions = []
    for i, q in enumerate(raw_questions):
        qtype_num = int(q.get("question_type") or q.get("type") or q.get("q_type") or 0)
        qtype_str = PK_TYPE_MAP.get(qtype_num, f"word.pk_{qtype_num}" if qtype_num else "word.unknown")
        word_id = int(q.get("word_id") or 0)
        q_id = int(q.get("id") or q.get("q_id") or q.get("question_id") or 0)

        word_info = word_map.get(word_id, {})
        w_content = clean(word_info.get("content") or q.get("word") or q.get("content") or "")
        w_trans = clean(word_info.get("translate") or q.get("translate") or "")

        ans_text = build_pk_answer_text(q, word_map)

        # 题干生成
        if qtype_num == 19:
            question_text = w_content or "英译中"
        elif qtype_num == 20:
            question_text = w_trans or "中译英"
        elif qtype_num == 16:
            question_text = w_trans or "拼写单词"
        elif qtype_num == 17:
            question_text = w_content or "朗读单词"
        elif qtype_num == 18:
            question_text = w_trans or "听写单词"
        elif qtype_num == 6:
            q_custom = clean(q.get("xt_nr") or q.get("question") or "")
            if q_custom:
                question_text = q_custom
            elif w_content and w_trans:
                question_text = f"{w_content} ({w_trans})"
            else:
                question_text = w_content or w_trans or "选择题"
        else:
            question_text = clean(q.get("question") or q.get("xt_nr") or w_content or w_trans or "")

        # 选项生成
        raw_opts = q.get("option") or q.get("options")
        options = parse_pk_options(raw_opts) if (qtype_num == 6 or raw_opts) else []

        # 重点要点 (释义/单词)
        important = []
        if w_trans and qtype_num != 19:
            important.append(w_trans)
        elif w_content and qtype_num != 20:
            important.append(w_content)

        extra = {}
        if word_id:
            extra["word_id"] = word_id
        if q_id:
            extra["question_id"] = q_id
        if w_content:
            extra["word"] = w_content
        if w_trans:
            extra["translate"] = w_trans

        questions.append(_make_question(
            no=i + 1,
            no_source="local",
            qtype=qtype_str,
            stid=word_id or None,
            question=question_text,
            answers=[ans_text] if ans_text else [],
            options=options,
            important=important,
            extra=extra or None,
        ))

    return questions


def collect_generic(content, content2=None, info=None, qtype=None):
    """未知类型兜底: 按结构探测"""
    if isinstance(content, list):
        return collect_word_pk(content, content2, info)
    info_obj = content.get("info") if isinstance(content, dict) else {}
    if not isinstance(info_obj, dict):
        if isinstance(content, dict) and (
            "word_question" in content
            or "word_questions" in content
            or "data" in content
            or "word_id" in content
            or "words" in content
            or "word" in content
        ):
            return collect_word_pk(content, content2, info)
        return []
    if (
        info_obj.get("word_question")
        or info_obj.get("word_questions")
        or info_obj.get("words")
        or info_obj.get("word")
    ):
        return collect_word_pk(content, content2, info)
    if info_obj.get("question") or info_obj.get("questions"):
        return collect_question_based(content, content2, info, qtype=qtype)
    if info_obj.get("sublist"):
        return collect_repeat_essay(content, content2, info)
    if info_obj.get("std"):
        if isinstance(info_obj["std"], list) and info_obj["std"] and \
                isinstance(info_obj["std"][0], dict) and "xth" in info_obj["std"][0]:
            return collect_fill(content, content2, info)
        return collect_picture(content, content2, info)
    if info_obj.get("xtlist"):
        return collect_choose(content, content2, info)
    if info_obj.get("value"):
        return collect_read(content, content2, info)
    if isinstance(content, dict) and (
        "word_question" in content
        or "word_questions" in content
        or "data" in content
        or "words" in content
        or "word" in content
    ):
        return collect_word_pk(content, content2, info)
    return []


def dispatch(structure_type):
    st = structure_type or ""
    if "choose" in st:
        return collect_choose
    if st == TYPE_REPEAT_ESSAY:
        return collect_repeat_essay
    if "pk" in st or "word_pk" in st or "vocabulary" in st or "word_homework" in st:
        return collect_word_pk
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
    """解析单个 content.json 或词汇作业结构, 返回题目列表"""
    if isinstance(content, list):
        return collect_word_pk(content, content2, info)
    if isinstance(content, dict) and ("word_question" in content or "word_questions" in content):
        return collect_word_pk(content, content2, info)
    structure_type = (content or {}).get("structure_type", "")
    collector = dispatch(structure_type)
    return collector(content, content2, info)

