# -*- coding: utf-8 -*-
"""ETS 作业答案解析器 (Python 版)

统一接口:
- parse_task(content_str, info_str, content2_str) -> list[dict]  单任务解析
- parse_batch(tasks) -> list[dict]                                批量解析(按stid排序)
- extract_meta(content_str) -> dict                               元信息
- group_by_time(items, threshold_ms) -> list[list]                时间聚类

固定输出格式见 api.py 文档字符串。
"""
from .api import (
    parse_task, parse_batch, extract_meta, group_by_time, parse_word_pk,
    build_pk_answer, build_pk_wrong_choice, build_pk_answer_text,
)
from .collectors import parse_content, TYPE_CHOOSE, TYPE_READ, TYPE_WORD, \
    TYPE_REPEAT, TYPE_REPEAT_ESSAY, TYPE_REPEAT_DIALOGUE, TYPE_ESSAY, \
    TYPE_DIALOGUE, TYPE_ROLE, TYPE_PICTURE, TYPE_FILL, TYPE_3Q5A, \
    TYPE_PK_CHOOSE, TYPE_PK_WRITE, TYPE_PK_READ, TYPE_PK_DICTATION, \
    TYPE_PK_EN_CN, TYPE_PK_CN_EN, PK_TYPE_MAP, parse_pk_options
from .cleaner import strip_html, clean, clean_lines, split_br

__all__ = [
    "parse_task", "parse_batch", "extract_meta", "group_by_time",
    "parse_word_pk", "build_pk_answer", "build_pk_wrong_choice", "build_pk_answer_text",
    "parse_content", "parse_pk_options",
    "TYPE_CHOOSE", "TYPE_READ", "TYPE_WORD",
    "TYPE_REPEAT", "TYPE_REPEAT_ESSAY", "TYPE_REPEAT_DIALOGUE", "TYPE_ESSAY",
    "TYPE_DIALOGUE", "TYPE_ROLE", "TYPE_PICTURE", "TYPE_FILL", "TYPE_3Q5A",
    "TYPE_PK_CHOOSE", "TYPE_PK_WRITE", "TYPE_PK_READ", "TYPE_PK_DICTATION",
    "TYPE_PK_EN_CN", "TYPE_PK_CN_EN", "PK_TYPE_MAP",
    "strip_html", "clean", "clean_lines", "split_br",
]

__version__ = "0.2.0"
