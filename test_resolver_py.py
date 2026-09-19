# -*- coding: utf-8 -*-
"""resolver_py 测试: 使用真实下载数据 (20套试卷) 验证解析正确性"""
import json
import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from resolver_py import (
    parse_task, parse_batch, extract_meta, group_by_time, parse_word_pk,
    build_pk_answer, build_pk_wrong_choice, build_pk_answer_text,
    TYPE_PK_CHOOSE, TYPE_PK_WRITE, TYPE_PK_READ, TYPE_PK_DICTATION,
    TYPE_PK_EN_CN, TYPE_PK_CN_EN,
)
from resolver_py.cleaner import strip_html, clean, split_br, split_keywords, \
    extract_ets_th, strip_ets_th, extract_global_no, strip_global_no

DATASET = "/tmp/opencode/dataset"

HAS_DATASET = os.path.isdir(DATASET)


def read_task(path):
    out = {}
    for fname, key in [("content.json", "content"), ("info.json", "info"),
                       ("content2.json", "content2")]:
        fp = os.path.join(path, fname)
        if os.path.isfile(fp):
            with open(fp, encoding="utf-8") as f:
                out[key] = f.read()
    return out


def paper_tasks(paper_dir):
    return sorted(
        os.path.join(paper_dir, d) + "/"
        for d in os.listdir(paper_dir)
        if os.path.isdir(os.path.join(paper_dir, d))
    )


# ============ cleaner ============

def test_strip_html():
    assert strip_html("<p>Hello <b>world</b></p>") == "Hello world"
    assert strip_html("a &amp; b &lt;c&gt;") == "a & b <c>"
    assert strip_html("<script>var x=1;</script>text") == "text"
    assert strip_html("&#x4F60;&#22909;") == "你好"


def test_split_br():
    assert split_br("a</br>b<br/>c") == ["a", "b", "c"]
    assert split_br("<p>x</p>y") == ["xy"]


def test_extract_numbers():
    assert extract_ets_th("ets_th5 What is it?") == 5
    assert extract_ets_th("no number") is None
    assert strip_ets_th("ets_th3 ask") == "ask"
    assert extract_global_no("7. What?") == 7
    assert extract_global_no("12、问") == 12
    assert strip_global_no("3. q") == "q"


def test_split_keywords():
    assert split_keywords("a | b | ") == ["a", "b"]
    assert split_keywords("") == []


# ============ 单任务解析 ============

@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
def test_parse_choose_task():
    d = os.path.join(DATASET, "extra/149795-%E6%A8%A1%E6%8B%9F%E8%AF%95%E9%A2%9842/f3bd3ee84a157e5061287a23cdc5263d")
    task = read_task(d)
    qs = parse_task(task["content"], task["info"], task["content2"])
    assert len(qs) == 2
    assert qs[0]["type"] == "collector.choose"
    assert qs[0]["no"] == 5          # content2 全局题号
    assert qs[0]["question"].startswith("Why does the boy love Sundays")
    assert qs[0]["answers"] == ["C"]
    assert len(qs[0]["options"]) == 3
    assert qs[1]["no"] == 6


@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
def test_parse_fill_task():
    d = os.path.join(DATASET, "extra/149795-%E6%A8%A1%E6%8B%9F%E8%AF%95%E9%A2%9842/3c1e75b1f31d06e5169d0cda75898a99")
    task = read_task(d)
    qs = parse_task(task["content"], task["info"], task["content2"])
    assert len(qs) == 4
    assert [q["answers"][0] for q in qs] == \
        ["Prepare", "conditions", "fertilizers/fertilisers", "regularly"]
    assert [q["no"] for q in qs] == [1, 2, 3, 4]


@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
def test_parse_3q5a_task():
    d = os.path.join(DATASET, "gd_high/286409-23-24%E5%AD%A6%E5%B9%B4%E9%AB%98%E4%BA%8C%E8%8B%B1%E8%AF%AD%E5%90%AC%E8%AF%B4%E8%AF%95%E9%A2%981/d305439f9c024b099dedb62edb5243d0")
    task = read_task(d)
    qs = parse_task(task["content"], task["info"], task["content2"])
    assert len(qs) == 8
    assert [q["role"] for q in qs] == ["a"] * 3 + ["b"] * 5
    assert qs[0]["answers"] and qs[0]["answers"][0]  # 三问有参考答案
    assert qs[4]["answers"] and qs[4]["answers"][0]  # 五答有参考答案


# ============ 批量解析与排序 ============

@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
@pytest.mark.parametrize("paper_rel", [
    "bj_high/142433-2020%E8%80%83%E5%89%8D%E5%86%B2%E5%88%BA1/",
    "bj_high/149668-2020%E8%80%83%E5%89%8D%E5%86%B2%E5%88%BA16/",
    "gd_junior/220364-%E6%B7%B1%E5%9C%B3%E5%B8%82%E4%B8%83%E5%B9%B4%E7%BA%A7%E6%9C%9F%E6%9C%AB%E5%90%AC%E8%AF%B4%E6%B5%8B%E8%AF%952/",
    "gd_high/284305-23-24%E5%AD%A6%E5%B9%B4%E9%AB%98%E4%B8%80%E8%8B%B1%E8%AF%AD%E5%90%AC%E8%AF%B4%E8%AF%95%E9%A2%98A/",
    "zk_mock/274643-%E4%B8%AD%E8%80%83%E6%9D%83%E5%A8%81%E6%95%99%E7%A8%8B%E6%A8%A1%E6%8B%9F31/",
    "special/282541-%28%E6%96%B0%E9%A2%98%E5%9E%8B%29%E6%A8%A1%E6%8B%9F%E8%AF%95%E9%A2%982/",
])
def test_parse_batch_ordering(paper_rel):
    paper = os.path.join(DATASET, paper_rel)
    tasks = [read_task(t) for t in paper_tasks(paper)]
    result = parse_batch(tasks)
    for section in result["sections"]:
        nos = [q["no"] for q in section["questions"]]
        for i in range(1, len(nos)):
            assert nos[i] >= nos[i - 1], f"{section['type']} 题号乱序: {nos}"


@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
def test_parse_batch_choose_section_full_coverage():
    """北京高中卷 choose 节应覆盖题号 1-14 且无重复"""
    paper = os.path.join(DATASET, "bj_high/142433-2020%E8%80%83%E5%89%8D%E5%86%B2%E5%88%BA1/")
    tasks = [read_task(t) for t in paper_tasks(paper)]
    result = parse_batch(tasks)
    choose = next(s for s in result["sections"] if s["type"] == "collector.choose")
    nos = [q["no"] for q in choose["questions"]]
    assert nos == list(range(1, 15))


@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
def test_parse_batch_gd_role_split():
    """广东初中: role 信息获取(1-10) 与 提问(1-2) 分两节, 且提问节在前者之后"""
    paper = os.path.join(DATASET, "gd_junior/220364-%E6%B7%B1%E5%9C%B3%E5%B8%82%E4%B8%83%E5%B9%B4%E7%BA%A7%E6%9C%9F%E6%9C%AB%E5%90%AC%E8%AF%B4%E6%B5%8B%E8%AF%952/")
    tasks = [read_task(t) for t in paper_tasks(paper)]
    result = parse_batch(tasks)
    role_secs = [s for s in result["sections"] if s["type"] == "collector.role"]
    assert len(role_secs) == 2
    assert [q["no"] for q in role_secs[0]["questions"]] == list(range(1, 11))
    assert [q["no"] for q in role_secs[1]["questions"]] == [1, 2]
    # 提问节的问题为中文
    assert any("\u4f60" in q["question"] for q in role_secs[1]["questions"])


# ============ 时间聚类 ============

def test_group_by_time_basic():
    items = [("a", 1000), ("b", 1030), ("c", 1060), ("d", 5000), ("e", 5030)]
    groups = group_by_time(items, threshold_ms=2000)
    assert groups == [["a", "b", "c"], ["d", "e"]]


def test_group_by_time_single():
    assert group_by_time([("a", 1)]) == [["a"]]
    assert group_by_time([]) == []


def test_group_by_time_threshold_boundary():
    items = [("a", 0), ("b", 2000), ("c", 4001)]
    assert group_by_time(items, threshold_ms=2000) == [["a", "b"], ["c"]]


# ============ 元信息 ============

@pytest.mark.skipif(not HAS_DATASET, reason="无真实数据集")
def test_extract_meta():
    d = os.path.join(DATASET, "extra/149795-%E6%A8%A1%E6%8B%9F%E8%AF%95%E9%A2%9842/f3bd3ee84a157e5061287a23cdc5263d")
    content = read_task(d)["content"]
    meta = extract_meta(content)
    assert meta["stid"] == 172909
    assert meta["structure_type"] == "collector.choose"
    assert meta["question_count"] == 2


# ============ 兜底: 未知类型 ============

def test_parse_generic_question_based():
    content = {
        "structure_type": "collector.unknown",
        "info": {"stid": "999", "question": [
            {"xh": "1", "ask": "Q1", "std": [{"value": "A1"}]}
        ]},
    }
    qs = parse_task(json.dumps(content))
    assert len(qs) == 1
    assert qs[0]["answers"] == ["A1"]
    assert qs[0]["type"] == "collector.unknown"


def test_parse_generic_std_based():
    content = {
        "structure_type": "collector.weird",
        "info": {"stid": "998", "std": [
            {"xth": "1", "value": "v1"}, {"xth": "2", "value": "v2"}
        ]},
    }
    qs = parse_task(json.dumps(content))
    assert len(qs) == 2
    assert qs[0]["answers"] == ["v1"]


# ============ 单词 PK 与词汇作业 ============

def test_pk_word_struct_nested():
    """测试 word/struct 嵌套形态解析"""
    data = {
        "data": [
            {
                "id": 101,
                "content": "apple",
                "translate": "苹果",
                "question": [
                    {
                        "id": 1001,
                        "question_type": 19,
                        "answer": "苹果",
                        "translate": "苹果",
                    },
                    {
                        "id": 1002,
                        "question_type": 20,
                        "answer": "apple",
                        "word": "apple",
                    }
                ]
            },
            {
                "id": 102,
                "content": "banana",
                "translate": "香蕉",
                "question": [
                    {
                        "id": 1003,
                        "question_type": 6,
                        "answer": "B",
                        "option": '[{"option_name":"A","option_value":"苹果"},{"option_name":"B","option_value":"香蕉"}]'
                    }
                ]
            }
        ]
    }
    qs = parse_word_pk(data)
    assert len(qs) == 3
    # 题 1: 英译中
    assert qs[0]["type"] == TYPE_PK_EN_CN
    assert qs[0]["question"] == "apple"
    assert qs[0]["answers"] == ["苹果"]
    assert qs[0]["word_id"] == 101
    # 题 2: 中译英
    assert qs[1]["type"] == TYPE_PK_CN_EN
    assert qs[1]["question"] == "苹果"
    assert qs[1]["answers"] == ["apple"]
    # 题 3: 选择题
    assert qs[2]["type"] == TYPE_PK_CHOOSE
    assert qs[2]["answers"] == ["B"]
    assert qs[2]["options"] == ["A. 苹果", "B. 香蕉"]


def test_pk_types_all_coverage():
    """测试各 PK 题型（6, 16, 17, 18, 19, 20）答案与题干提取"""
    questions = [
        {"id": 1, "question_type": 6, "word": "dog", "translate": "狗", "answer": "A", "option": [{"name": "A", "value": "狗"}]},
        {"id": 2, "question_type": 16, "word": "cat", "translate": "猫", "answer": ""},
        {"id": 3, "question_type": 17, "word": "bird", "translate": "鸟", "answer": ""},
        {"id": 4, "question_type": 18, "word": "fish", "translate": "鱼", "answer": ""},
        {"id": 5, "question_type": 19, "word": "book", "translate": "书", "answer": ""},
        {"id": 6, "question_type": 20, "word": "pen", "translate": "钢笔", "answer": ""},
    ]
    qs = parse_word_pk({"word_questions": questions})
    assert len(qs) == 6
    assert qs[0]["type"] == TYPE_PK_CHOOSE and qs[0]["answers"] == ["A"]
    assert qs[1]["type"] == TYPE_PK_WRITE and qs[1]["answers"] == ["cat"] and qs[1]["question"] == "猫"
    assert qs[2]["type"] == TYPE_PK_READ and qs[2]["answers"] == ["bird"] and qs[2]["question"] == "bird"
    assert qs[3]["type"] == TYPE_PK_DICTATION and qs[3]["answers"] == ["fish"] and qs[3]["question"] == "鱼"
    assert qs[4]["type"] == TYPE_PK_EN_CN and qs[4]["answers"] == ["书"] and qs[4]["question"] == "book"
    assert qs[5]["type"] == TYPE_PK_CN_EN and qs[5]["answers"] == ["pen"] and qs[5]["question"] == "钢笔"


def test_pk_helpers():
    """测试 build_pk_answer 与 build_pk_wrong_choice"""
    q_info = {
        "id": 888,
        "word_id": 999,
        "word": "orange",
        "translate": "橙子",
        "answer": "B",
        "option": '[{"option_name":"A","option_value":"苹果"},{"option_name":"B","option_value":"橙子"}]'
    }
    ans = build_pk_answer(6, q_info)
    assert ans == {"point": 1, "answer": "B", "word_id": 999, "question_id": 888}

    wrong = build_pk_wrong_choice(q_info)
    assert wrong == "A"

    # 测试 option_value 匹配的错题生成
    q_val_match = {
        "answer": "橙子",
        "option": [{"name": "A", "value": "苹果"}, {"name": "B", "value": "橙子"}]
    }
    assert build_pk_wrong_choice(q_val_match) == "苹果"


def test_repeat_essay_sublist():
    """测试 collector.repeat_essay 对 sublist 的逐句解析"""
    content = {
        "structure_type": "collector.repeat_essay",
        "info": {
            "stid": 1234,
            "sublist": [
                {"text": "Hello world."},
                {"text": "Good morning."}
            ]
        }
    }
    qs = parse_task(json.dumps(content))
    assert len(qs) == 2
    assert qs[0]["question"] == "Hello world."
    assert qs[1]["question"] == "Good morning."
    assert qs[0]["type"] == "collector.repeat_essay"
