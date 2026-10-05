# -*- coding: utf-8 -*-
"""Unified ETS100 parser prototype.

This keeps the old resolver's lightweight file/string input while adopting the
Android parser's explicit region, question title, type, and ordering model.
"""

import argparse
import contextvars
import html
import json
import re
import sys
import traceback
from pathlib import Path


TYPE_NAMES = {
    "collector.role": "问答",
    "collector.picture": "信息转述",
    "collector.read": "朗读",
    "collector.3q5a": "3问5答",
    "collector.choose": "选择题",
    "collector.fill": "填空",
    "collector.dialogue": "回答问题",
    "collector.word": "单词",
    "collector.repeat": "跟读",
    "collector.repeat_essay": "作文",
    "collector.repeat_dialogue": "对话",
    "collector.essay": "作文",
}

CATEGORY_TYPES = {
    "read_chapter": "collector.read",
    "simple_expression_ufi": "collector.role",
    "simple_expression_ufk": "collector.role",
    "topic": "collector.picture",
    "simple_expression_ufj": "collector.role",
}

# False uses the current regional ETS100 mappings. True uses the original
# ets_resolver mappings. Keep output shape unchanged across both modes.
LEGACY_MODE = False
NOFILE_READ = False
DIAGNOSTIC_MODE = True
DEBUG_MODE = False
_WARNINGS = contextvars.ContextVar("resolver_warnings", default=None)
_CHANNEL = contextvars.ContextVar("resolver_channel", default="fuck_ets100")
_LAST_ERROR = contextvars.ContextVar("resolver_last_error", default=None)
_LAST_DIAGNOSTICS = contextvars.ContextVar("resolver_last_diagnostics", default=None)


class ResolverError(Exception):
    def __init__(self, code, message, stage, **context):
        super().__init__(message)
        self.code = code
        self.message = message
        self.stage = stage
        self.context = {key: value for key, value in context.items() if value is not None}

    def to_dict(self):
        return {
            "code": self.code,
            "message": self.message,
            "stage": self.stage,
            **self.context,
        }


def legacy_mode(mode: bool):
    global LEGACY_MODE
    LEGACY_MODE = mode


def nofile_mode(mode: bool):
    global NOFILE_READ
    NOFILE_READ = mode


def diagnostic_mode(mode: bool):
    global DIAGNOSTIC_MODE
    DIAGNOSTIC_MODE = mode


def debug_mode(mode: bool):
    global DEBUG_MODE
    DEBUG_MODE = mode


def add_warning(code, message, stage, **context):
    warnings = _WARNINGS.get()
    if warnings is not None:
        warnings.append({
            "code": code,
            "message": message,
            "stage": stage,
            **{key: value for key, value in context.items() if value is not None},
        })

REGION_TYPE_NAMES = {
    "广东高中": {
        "collector.read": "模仿朗读",
        "collector.3q5a": "3问5答",
    },
    "广东初中": {
        "collector.read": "模仿朗读",
        "collector.role": "问答",
        "collector.picture": "信息转述",
    },
    "北京初中": {
        "collector.choose": "听后选择",
        "collector.role": "听后回答",
        "collector.picture": "听后转述",
        "collector.read": "短文朗读",
    },
    "北京高中": {
        "collector.choose": "听后选择",
        "collector.fill": "听后记录",
        "collector.picture": "听后转述",
        "collector.dialogue": "回答问题",
        "collector.read": "短文朗读",
    },
}

REGION_SECTION_ORDER = {
    "广东高中": ["模仿朗读", "3问5答"],
    "广东初中": ["模仿朗读", "听选信息", "回答问题", "信息转述", "提问"],
    "北京初中": ["听后选择", "听后回答", "听后转述", "短文朗读"],
    "北京高中": ["听后选择", "听后记录", "听后转述", "回答问题", "短文朗读"],
}

NUMBER_FIELDS = (
    "xt_xh", "xth", "xh", "th", "question_no", "questionNo",
    "order", "sort", "index",
)
TEXT_NUMBER_PATTERNS = (
    re.compile(r"ets_th\s*(\d+)", re.I),
    re.compile(r"第\s*(\d+)\s*题"),
    re.compile(r"^\s*(\d+)\s*[.、．)）]"),
)


def clean_text(value, split_pipes=False):
    text = "" if value is None else str(value)
    text = re.sub(r"ets_th\d+\s*", "", text, flags=re.I)
    text = re.sub(r"</p>\s*<p[^>]*>", "\n", text, flags=re.I)
    text = re.sub(r"<br\s*/?>|</br>|</p>|<p[^>]*>", "\n", text, flags=re.I)
    text = re.sub(r"<[^>]+>", "", text)
    text = html.unescape(text).replace("\u200b", "")
    if split_pipes:
        text = text.replace("|", "\n")
    return "\n".join(line.strip() for line in text.splitlines() if line.strip())


def positive_int(value):
    try:
        number = int(str(value).strip())
        return number if number > 0 else None
    except (TypeError, ValueError):
        return None


def question_number(item, *text_fields):
    texts = [str(item.get(key, "")) for key in text_fields]
    texts.extend(str(item.get(key, "")) for key in ("xt_nr", "ask", "question", "text", "topic", "value"))
    for pattern in TEXT_NUMBER_PATTERNS:
        for text in texts:
            match = pattern.search(text)
            if match:
                return positive_int(match.group(1))
    for field in NUMBER_FIELDS:
        number = positive_int(item.get(field))
        if number is not None:
            return number
    return None


def infer_region_decision(contents, template=None):
    """Return the first conclusive region decision, ordered by reliability."""
    structure_types = [item.get("structure_type", "") for item in contents]
    count = len(contents)
    if template:
        engine_areas = template.get("engine_areas", set())
        template_types = template.get("structure_types", set())
        if any(area.startswith("guangdong") for area in engine_areas):
            region = "广东高中" if "collector.3q5a" in template_types else "广东初中"
            return {"value": region, "method": "res.engine_area", "reliability": 100}
        if "beijing" in engine_areas:
            region = "北京高中" if template_types & {"collector.fill", "collector.dialogue"} else "北京初中"
            return {"value": region, "method": "res.engine_area", "reliability": 100}
        if template_types:
            decision = infer_region_decision([{"structure_type": item} for item in template_types])
            if decision["value"] != "未知":
                return {"value": decision["value"], "method": "res.structure_types", "reliability": 90}
    if "collector.3q5a" in structure_types:
        return {"value": "广东高中", "method": "content.structure_type.3q5a", "reliability": 85}
    if "collector.fill" in structure_types or "collector.dialogue" in structure_types:
        return {"value": "北京高中", "method": "content.structure_type.fill_dialogue", "reliability": 85}
    if "collector.choose" in structure_types:
        region = "北京高中" if count == 13 else "北京初中"
        return {"value": region, "method": "content.structure_type.choose", "reliability": 80}
    if count % 7 == 0 and "collector.role" in structure_types and "collector.picture" in structure_types:
        return {"value": "广东初中", "method": "content.type_profile", "reliability": 75}
    if count == 3:
        return {"value": "广东高中", "method": "resource.count", "reliability": 50}
    if count == 10:
        return {"value": "北京初中", "method": "resource.count", "reliability": 50}
    if count == 13:
        return {"value": "北京高中", "method": "resource.count", "reliability": 50}
    return {"value": "未知", "method": "none", "reliability": 0}


def infer_region(contents, template=None):
    return infer_region_decision(contents, template)["value"]


def infer_type_name(structure_type, region, content):
    if region == "广东初中" and structure_type == "collector.role":
        first = ((content.get("info") or {}).get("question") or [{}])[0]
        ask = str(first.get("ask", ""))
        audio = str(first.get("askaudio", ""))
        if "<br>" in ask.lower() and audio:
            return "听选信息"
        if audio:
            return "回答问题"
        if re.search(r"[\u4e00-\u9fff]", ask):
            return "提问"
        return "听选信息"
    return REGION_TYPE_NAMES.get(region, {}).get(structure_type, TYPE_NAMES.get(structure_type, structure_type or "未知题型"))


def make_question(order, title, answers=None, original_text=None, options=None, question_id=None):
    return {
        "id": question_id or str(order),
        "order": order,
        "name": clean_text(title) or f"第{order}题",
        "answers": [clean_text(answer, split_pipes=True) for answer in (answers or []) if clean_text(answer, split_pipes=True)],
        "options": [clean_text(option) for option in (options or []) if clean_text(option)],
        "original_text": clean_text(original_text) or None,
    }


def parse_legacy_choose(info):
    questions = []
    for entry in info if isinstance(info, list) else []:
        if entry.get("code_id") != "answer" or entry.get("code_type") != "json_array":
            continue
        try:
            answers = json.loads(entry.get("code_json_array", "[]"))
        except (TypeError, json.JSONDecodeError) as error:
            raise ResolverError(
                "LEGACY_FORMAT_INVALID",
                "Legacy choice answer JSON is invalid",
                "parse_questions",
                detail=str(error),
                field="info.code_json_array",
                structure_type="collector.choose",
                recoverable=False,
            ) from error
        for index, answer in enumerate(answers, 1):
            order = positive_int(answer.get("xth")) or index
            stid = str(answer.get("stid", ""))
            questions.append(make_question(
                order,
                f"第{order}题",
                [answer.get("answer", "")],
                question_id=f"{stid}_{answer.get('xth', order)}",
            ))
    return questions


def _parse_content(content, info=None, region="未知", category=""):
    structure_type = content.get("structure_type") or CATEGORY_TYPES.get(category, "")
    data = content.get("info") or {}
    original = data.get("value") or data.get("st_nr")
    questions = []

    if structure_type == "collector.choose":
        if LEGACY_MODE:
            questions = parse_legacy_choose(info)
        else:
            for index, item in enumerate(data.get("xtlist") or [], 1):
                order = question_number(item, "xt_nr", "xt_value", "xt_wj") or index
                options = [f"{option.get('xx_mc', '')}. {option.get('xx_nr', '')}" for option in item.get("xxlist") or []]
                questions.append(make_question(order, item.get("xt_nr"), [item.get("answer", "")], original, options))
            if not questions:
                questions = parse_legacy_choose(info)
                if questions:
                    _CHANNEL.set("legacy_ets_resolver")
                    add_warning(
                        "NEW_FORMAT_UNAVAILABLE",
                        "content.info.xtlist is missing; used legacy info.json answers",
                        "parse_questions",
                        structure_type=structure_type,
                        fallback={"from": "content.info.xtlist", "to": "info.code_json_array"},
                    )
    elif structure_type == "collector.fill":
        for index, item in enumerate(data.get("std") or [], 1):
            order = question_number(item, "xth", "value") or index
            questions.append(make_question(order, f"第{order}题", [item.get("value", "")]))
    elif structure_type == "collector.picture":
        answers = [item.get("value", "") for item in data.get("std") or []]
        order = question_number(data, "topic", "value") or 1
        questions.append(make_question(order, data.get("topic", "信息转述"), answers, original))
    elif structure_type in ("collector.role", "collector.3q5a", "collector.dialogue", "collector.repeat", "collector.repeat_essay", "collector.repeat_dialogue", "collector.essay"):
        items = data.get("question") or data.get("questions") or []
        for index, item in enumerate(items, 1):
            order = question_number(item, "ask", "question", "text") or index
            title = item.get("ask") or item.get("question") or item.get("text")
            answers = [answer.get("value", "") for answer in item.get("std") or []]
            questions.append(make_question(order, title, answers, original, question_id=f"{data.get('stid', '')}_{item.get('xh', order)}"))
    elif structure_type == "collector.read":
        # The original resolver treats value as an answer; current regional
        # ETS100 data treats it as reading material, even when analyze is empty.
        if LEGACY_MODE:
            questions.append(make_question(1, "朗读", [original], data.get("analyze"), question_id=f"{data.get('stid', '')}_0"))
        else:
            questions.append(make_question(1, "朗读原文", [], original, question_id=f"{data.get('stid', '')}_0"))
    elif structure_type == "collector.word":
        questions.append(make_question(1, "单词", [original], data.get("analyze"), question_id=f"{data.get('stid', '')}_0"))
    else:
        # Shape-based fallback for unknown or missing structure_type.
        if data.get("question") or data.get("questions"):
            fallback = dict(content, structure_type="collector.role")
            return _parse_content(fallback, info, region, category)
        if data.get("std"):
            fallback = dict(content, structure_type="collector.picture")
            return _parse_content(fallback, info, region, category)

    if not structure_type:
        raise ResolverError(
            "STRUCTURE_TYPE_MISSING",
            "structure_type is missing and content shape is not recognized",
            "detect_structure_type",
            field="structure_type",
            recoverable=False,
        )
    if not questions and structure_type != "collector.read":
        raise ResolverError(
            "QUESTION_NOT_FOUND",
            "No questions could be parsed for the selected structure type",
            "parse_questions",
            structure_type=structure_type,
            recoverable=False,
        )

    questions.sort(key=lambda question: question["order"])
    for index, question in enumerate(questions, 1):
        question["section_order"] = index

    return {
        "region": region,
        "type": structure_type or "unknown",
        "type_name": infer_type_name(structure_type, region, content),
        "questions": questions,
    }


def load_json(path, default=None, required=False, stage="load_json"):
    if not path.exists():
        if required:
            raise ResolverError(
                "CONTENT_NOT_FOUND" if path.name == "content.json" else "TARGET_NOT_FOUND",
                f"{path.name} not found",
                stage,
                file=str(path),
                target=str(path.parent),
                recoverable=False,
            )
        return default
    try:
        with path.open("r", encoding="utf-8") as file:
            return json.load(file)
    except OSError as error:
        raise ResolverError(
            "CONTENT_READ_FAILED" if path.name == "content.json" else "FILE_READ_FAILED",
            f"Failed to read {path.name}",
            stage,
            detail=str(error),
            file=str(path),
            recoverable=False,
        ) from error
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        code = "CONTENT_JSON_INVALID" if path.name == "content.json" else "RES_JSON_INVALID" if path.name == "res.json" else "INFO_JSON_INVALID"
        raise ResolverError(
            code,
            f"{path.name} is not valid UTF-8 JSON",
            stage,
            detail=str(error),
            file=str(path),
            recoverable=path.name != "content.json",
        ) from error


def read_resource_template(root_path):
    """Read the first usable res.json template below a resource root."""
    for path in sorted(Path(root_path).glob("*/res.json")):
        try:
            data = load_json(path, {}, stage="load_template")
        except ResolverError as error:
            add_warning(error.code, error.message, error.stage, **error.context)
            continue
        exam_types = data.get("exam_type_list") or []
        sections = []
        engine_areas = set()
        for type_index, exam_type in enumerate(exam_types, 1):
            structure_type = exam_type.get("exam_type_collect", "")
            type_name = exam_type.get("exam_type_name") or structure_type
            type_order = positive_int(exam_type.get("exam_type_order")) or type_index
            exams = exam_type.get("exam_list") or ([exam_type] if exam_type.get("exam_id") else [])
            for exam_index, exam in enumerate(exams, 1):
                match = re.search(r"st(\d+)", str(exam.get("exam_id", "")), re.I)
                template_order = positive_int(match.group(1)) if match else None
                question_numbers = []
                for item in exam.get("exam_info") or []:
                    number = positive_int(str(item.get("value", "")).strip().rstrip(".。"))
                    if number is not None:
                        question_numbers.append(number)
                for question in exam.get("question_list") or []:
                    area = str(question.get("engine_area", "")).lower()
                    if area:
                        engine_areas.add(area)
                sections.append({
                    "structure_type": structure_type,
                    "type_name": type_name,
                    "type_order": type_order,
                    "template_order": template_order or len(sections) + 1,
                    "question_numbers": question_numbers,
                    "exam_order": positive_int(exam.get("exam_order")) or exam_index,
                })
        if sections:
            return {
                "source": str(path),
                "sections": sections,
                "engine_areas": engine_areas,
                "structure_types": {section["structure_type"] for section in sections},
            }
    return None


def order_resource_tasks(tasks, contents, template=None):
    """Order resource packages using the first complete reliable strategy."""
    paired = list(zip(tasks, contents))
    stids = [positive_int((content.get("info") or {}).get("stid")) for content in contents]
    if template and stids and all(stid is not None for stid in stids):
        base_stid = min(stids)
        template_by_order = {item["template_order"]: item for item in template["sections"]}
        mapped = []
        for pair, stid in zip(paired, stids):
            item = template_by_order.get(stid - base_stid + 1)
            if item is None or item["structure_type"] != pair[1].get("structure_type", ""):
                break
            mapped.append((item, pair))
        else:
            mapped.sort(key=lambda value: (value[0]["type_order"], value[0]["exam_order"], value[0]["template_order"]))
            return [pair for _, pair in mapped], "res.exam_id+content.stid", 100, {str(pair[0]): item for item, pair in mapped}

    if stids and all(stid is not None for stid in stids) and len(set(stids)) == len(stids):
        paired.sort(key=lambda pair: positive_int((pair[1].get("info") or {}).get("stid")))
        return paired, "content.stid", 80, {}

    section_order = REGION_SECTION_ORDER.get(infer_region(contents), [])
    if section_order:
        order_map = {name: index for index, name in enumerate(section_order)}
        paired.sort(key=lambda pair: (order_map.get(infer_type_name(pair[1].get("structure_type", ""), infer_region(contents), pair[1]), 999), pair[0].name))
        return paired, "region.type_order", 60, {}

    paired.sort(key=lambda pair: (-pair[0].stat().st_mtime, pair[0].name))
    return paired, "modified_time+resource_id", 30, {}


def apply_template_question_numbers(result, template_section):
    numbers = template_section.get("question_numbers") or []
    if not numbers:
        return "content.question_number"
    for question, number in zip(result["questions"], numbers):
        question["order"] = number
    result["questions"].sort(key=lambda question: question["order"])
    for index, question in enumerate(result["questions"], 1):
        question["section_order"] = index
    result["type_name"] = template_section["type_name"]
    return "res.exam_info"


def _parse_task(task_path, region="未知", category=""):
    task_path = Path(task_path)
    if not task_path.exists():
        raise ResolverError("TARGET_NOT_FOUND", "Task path not found", "validate_argument", target=str(task_path), recoverable=False)
    if not task_path.is_dir():
        raise ResolverError("TARGET_NOT_DIRECTORY", "Task path is not a directory", "validate_argument", target=str(task_path), recoverable=False)
    content = load_json(task_path / "content.json", required=True, stage="load_content")
    try:
        info = load_json(task_path / "info.json", [], stage="load_info")
    except ResolverError as error:
        add_warning(error.code, error.message, error.stage, **error.context)
        info = []
    return _parse_content(content, info, region, category)


def _parse_resource_group(root_path):
    root_path = Path(root_path)
    if not root_path.exists():
        raise ResolverError("TARGET_NOT_FOUND", "Resource group path not found", "validate_argument", target=str(root_path), recoverable=False)
    if not root_path.is_dir():
        raise ResolverError("TARGET_NOT_DIRECTORY", "Resource group path is not a directory", "validate_argument", target=str(root_path), recoverable=False)
    task_data = []
    for child in sorted(root_path.iterdir(), key=lambda item: item.name):
        if child.is_dir() and (child / "content.json").is_file():
            task_data.append((child, load_json(child / "content.json")))
    contents = [content for _, content in task_data]
    template = read_resource_template(root_path)
    if template is None:
        add_warning("TEMPLATE_NOT_FOUND", "No usable res.json template was found", "load_template", target=str(root_path))
    region_decision = infer_region_decision(contents, template)
    ordered, order_method, order_reliability, template_matches = order_resource_tasks(
        [path for path, _ in task_data], contents, template
    )
    if order_reliability < 100:
        add_warning(
            "RESOURCE_ORDER_FALLBACK",
            "Official template order was unavailable; used a lower-reliability method",
            "sort_resources",
            method=order_method,
            reliability=order_reliability,
        )
    sections = []
    for path, _ in ordered:
        result = _parse_task(path, region_decision["value"])
        template_section = template_matches.get(str(path))
        question_order_method = apply_template_question_numbers(result, template_section) if template_section else "content.question_number"
        sections.append(dict(result, resource_id=path.name, question_order_method=question_order_method))
    return {
        "region": region_decision["value"],
        "region_detection": region_decision,
        "resource_order": {"method": order_method, "reliability": order_reliability},
        "template_source": template["source"] if template else None,
        "sections": sections,
    }


def _parse_resource_root(root_path, threshold_seconds=3):
    """Split a resource root into download-time groups and parse each group."""
    root_path = Path(root_path)
    if not root_path.exists():
        raise ResolverError("TARGET_NOT_FOUND", "Resource root path not found", "validate_argument", target=str(root_path), recoverable=False)
    if not root_path.is_dir():
        raise ResolverError("TARGET_NOT_DIRECTORY", "Resource root path is not a directory", "validate_argument", target=str(root_path), recoverable=False)
    tasks = [
        child for child in root_path.iterdir()
        if child.is_dir() and (child / "content.json").is_file()
    ]
    tasks.sort(key=lambda item: item.stat().st_mtime, reverse=True)
    groups = []
    for task in tasks:
        if not groups or abs(groups[-1][-1].stat().st_mtime - task.stat().st_mtime) > threshold_seconds:
            groups.append([task])
        else:
            groups[-1].append(task)

    template = read_resource_template(root_path)
    if template is None:
        add_warning("TEMPLATE_NOT_FOUND", "No usable res.json template was found", "load_template", target=str(root_path))
    results = []
    for index, group in enumerate(groups, 1):
        contents = [load_json(task / "content.json") for task in group]
        region_decision = infer_region_decision(contents, template)
        ordered, order_method, order_reliability, template_matches = order_resource_tasks(group, contents, template)
        if order_reliability < 100:
            add_warning(
                "RESOURCE_ORDER_FALLBACK",
                "Official template order was unavailable; used a lower-reliability method",
                "sort_resources",
                group=index,
                method=order_method,
                reliability=order_reliability,
            )
        sections = []
        for task, _ in ordered:
            result = _parse_task(task, region_decision["value"])
            template_section = template_matches.get(str(task))
            question_order_method = apply_template_question_numbers(result, template_section) if template_section else "content.question_number"
            sections.append(dict(result, resource_id=task.name, question_order_method=question_order_method))
        results.append({
            "group": index,
            "region": region_decision["value"],
            "region_detection": region_decision,
            "resource_order": {"method": order_method, "reliability": order_reliability},
            "template_source": template["source"] if template else None,
            "sections": sections,
        })
    return {"groups": results}


def _public_call(operation, function, *args, **kwargs):
    warning_token = _WARNINGS.set([])
    channel_token = _CHANNEL.set("legacy_ets_resolver" if LEGACY_MODE else "fuck_ets100")
    _LAST_ERROR.set(None)
    _LAST_DIAGNOSTICS.set(None)
    try:
        data = function(*args, **kwargs)
        warnings = _WARNINGS.get() or []
        diagnostics = {
            "operation": operation,
            "channel": _CHANNEL.get(),
            "legacy_mode": LEGACY_MODE,
        }
        if DIAGNOSTIC_MODE:
            diagnostics["warnings"] = warnings
        _LAST_DIAGNOSTICS.set(diagnostics)
        return data
    except ResolverError as error:
        diagnostics = {
            "operation": operation,
            "channel": _CHANNEL.get(),
            "legacy_mode": LEGACY_MODE,
        }
        if DIAGNOSTIC_MODE:
            diagnostics["warnings"] = _WARNINGS.get() or []
        _LAST_DIAGNOSTICS.set(diagnostics)
        _LAST_ERROR.set({"operation": operation, **error.to_dict()})
        return -1
    except Exception as error:
        error_data = {
            "operation": operation,
            "code": "INTERNAL_ERROR",
            "message": "Unexpected resolver error",
            "stage": "internal",
            "detail": str(error),
            "recoverable": False,
        }
        if DEBUG_MODE:
            error_data["traceback"] = traceback.format_exc()
        diagnostics = {
            "operation": operation,
            "channel": _CHANNEL.get(),
            "legacy_mode": LEGACY_MODE,
        }
        if DIAGNOSTIC_MODE:
            diagnostics["warnings"] = _WARNINGS.get() or []
        _LAST_DIAGNOSTICS.set(diagnostics)
        _LAST_ERROR.set(error_data)
        return -1
    finally:
        _WARNINGS.reset(warning_token)
        _CHANNEL.reset(channel_token)


def resolver_last_error():
    return _LAST_ERROR.get()


def resolver_last_diagnostics():
    return _LAST_DIAGNOSTICS.get()


def resolver_clear_diagnostics():
    _LAST_ERROR.set(None)
    _LAST_DIAGNOSTICS.set(None)


def _decode_json_input(value, name, default=None):
    if value is None:
        return default
    if isinstance(value, (dict, list)):
        return value
    if not isinstance(value, str):
        raise ResolverError("INVALID_ARGUMENT", f"{name} must be JSON text or a parsed JSON value", "validate_argument", field=name, recoverable=False)
    try:
        return json.loads(value)
    except json.JSONDecodeError as error:
        raise ResolverError("CONTENT_JSON_INVALID" if name == "content" else "INFO_JSON_INVALID", f"{name} is not valid JSON", "parse_json_input", detail=str(error), field=name, recoverable=False) from error


def _decode_container(target, expected_name):
    if isinstance(target, str):
        try:
            target = json.loads(target)
        except json.JSONDecodeError as error:
            raise ResolverError("INVALID_NOFILE_INPUT", f"{expected_name} input is not valid JSON", "parse_json_input", detail=str(error), recoverable=False) from error
    return target


def _resolve_input(target):
    if not NOFILE_READ:
        return _parse_task(target)
    pair = _decode_container(target, "resolve")
    if not isinstance(pair, (list, tuple)) or len(pair) < 2:
        raise ResolverError("INVALID_NOFILE_INPUT", "resolve expects [content, info] in nofile mode", "validate_argument", recoverable=False)
    return _parse_content(_decode_json_input(pair[0], "content"), _decode_json_input(pair[1], "info", []))


def resolve(TARGET):
    return _public_call("resolve", _resolve_input, TARGET)


def _resolve_by_homework_input(target):
    if NOFILE_READ:
        pairs = _decode_container(target, "resolve_by_homework")
        if not isinstance(pairs, list):
            raise ResolverError("INVALID_NOFILE_INPUT", "resolve_by_homework expects a list of [content, info] pairs", "validate_argument", recoverable=False)
        results = []
        for index, pair in enumerate(pairs):
            try:
                if not isinstance(pair, (list, tuple)) or len(pair) < 2:
                    raise ResolverError("INVALID_NOFILE_INPUT", "Homework item must be [content, info]", "validate_argument", item=index, recoverable=False)
                results.append(_parse_content(_decode_json_input(pair[0], "content"), _decode_json_input(pair[1], "info", [])))
            except ResolverError as error:
                results.append(-1)
                add_warning("ITEM_PARSE_FAILED", error.message, error.stage, item=index, error=error.to_dict())
        return results

    root = Path(target)
    if not root.exists() or not root.is_dir():
        raise ResolverError("TARGET_NOT_DIRECTORY", "Homework path is not a directory", "validate_argument", target=str(root), recoverable=False)
    results = []
    for child in sorted(root.iterdir(), key=lambda item: item.name):
        if not child.is_dir() or not (child / "content.json").is_file():
            continue
        try:
            results.append(_parse_task(child))
        except ResolverError as error:
            results.append(-1)
            add_warning("ITEM_PARSE_FAILED", error.message, error.stage, resource_id=child.name, error=error.to_dict())
    return results


def resolve_by_homework(TARGET):
    return _public_call("resolve_by_homework", _resolve_by_homework_input, TARGET)


def _resolve_by_all_homework_input(target):
    if NOFILE_READ:
        groups = _decode_container(target, "resolve_by_all_homework")
        if not isinstance(groups, list):
            raise ResolverError("INVALID_NOFILE_INPUT", "resolve_by_all_homework expects nested homework lists", "validate_argument", recoverable=False)
        return {str(index): _resolve_by_homework_input(group) for index, group in enumerate(groups)}

    root = Path(target)
    if not root.exists() or not root.is_dir():
        raise ResolverError("TARGET_NOT_DIRECTORY", "All-homework path is not a directory", "validate_argument", target=str(root), recoverable=False)
    return {
        child.name: _resolve_by_homework_input(child)
        for child in sorted(root.iterdir(), key=lambda item: item.name)
        if child.is_dir()
    }


def resolve_by_all_homework(TARGET):
    return _public_call("resolve_by_all_homework", _resolve_by_all_homework_input, TARGET)


def resolve_resource(TARGET, threshold_seconds=3):
    if NOFILE_READ:
        return _public_call(
            "resolve_resource",
            lambda: (_ for _ in ()).throw(ResolverError("NOFILE_NOT_SUPPORTED", "resolve_resource currently requires path mode", "validate_argument", recoverable=False)),
        )
    return _public_call("resolve_resource", _parse_resource_root, TARGET, threshold_seconds)


def main():
    parser = argparse.ArgumentParser(description="Parse ETS100 content.json answers")
    parser.add_argument("target", help="Task directory or resource group directory")
    parser.add_argument("--group", action="store_true", help="Parse all direct child task directories")
    parser.add_argument("--resource-root", action="store_true", help="Split tasks by modification time before parsing groups")
    parser.add_argument("--diagnostic", action="store_true", help="Include warnings and fallback details")
    parser.add_argument("--debug", action="store_true", help="Include traceback for unexpected internal errors")
    args = parser.parse_args()
    diagnostic_mode(args.diagnostic)
    debug_mode(args.debug)
    if args.resource_root:
        result = resolve_resource(args.target)
    elif args.group:
        result = _public_call("resolve_resource_group", _parse_resource_group, args.target)
    else:
        result = resolve(args.target)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    if result == -1 and resolver_last_error():
        print(json.dumps(resolver_last_error(), ensure_ascii=False), file=sys.stderr)
    return 1 if result == -1 else 0


if __name__ == "__main__":
    sys.exit(main())
