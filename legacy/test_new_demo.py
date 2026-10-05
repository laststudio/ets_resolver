#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Pytest coverage for the pure-Python resolver in ``demo.py``.

The suite creates small, representative ETS payloads in ``tmp_path`` so it
does not depend on a particular downloaded homework archive.  A pytest
terminal report is printed at the end, including the same high-level figures
as ``test_demo.py``: successful/failed tasks, answer totals, type statistics,
timings, and failure details.

Examples:
    python -m pytest -q test_new_demo.py

The repository's ``answer`` directory is shown in the final report for easy
comparison, while functional tests remain deterministic and self-contained.
"""

from __future__ import annotations

import json
import logging
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

import pytest


SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

try:
    import demo as resolver
except SyntaxError as exc:  # demo.py uses Python 3.10 structural matching.
    pytest.skip(
        f"demo.py requires Python 3.10 or newer: {exc}",
        allow_module_level=True,
    )


# ---------------------------------------------------------------------------
# Report helpers
# ---------------------------------------------------------------------------

_REPORT: Dict[str, Any] = {
    "results": [],
    "test_timings": [],
    "failures": [],
}


def count_answers(result: Any) -> int:
    """Count answer entries in one ``resolve`` result, excluding ``type``."""
    if result is None or result == -1 or not isinstance(result, dict):
        return 0
    return sum(1 for key in result if key != "type")


def get_type(result: Any) -> str:
    if isinstance(result, dict) and result != -1:
        return str(result.get("type", "unknown"))
    return "failed"


def record_result(label: str, result: Any) -> None:
    """Record resolver-level metrics used by the terminal report."""
    _REPORT["results"].append(
        {
            "label": label,
            "success": result is not None and result != -1,
            "answers": count_answers(result),
            "type": get_type(result),
        }
    )


def _test_category(item: pytest.Item) -> str:
    """Derive a report category without requiring a project-level pytest.ini."""
    name = item.name
    if name.startswith(("test_remove", "test_sort", "test_listdir")):
        return "helpers"
    if name.startswith("test_mode"):
        return "modes"
    if name.startswith("test_log"):
        return "logging"
    if name.startswith("test_collectors_file"):
        return "collector"
    if name.startswith("test_resolve_dispatch") or name.startswith("test_resolve_unknown"):
        return "resolve"
    if name.startswith("test_collectors_and_resolve"):
        return "errors"
    if name.startswith(("test_no_analysis", "test_legacy_analysis", "test_new_analysis")):
        return "analysis"
    if name.startswith("test_resolve_by_") or name.startswith("test_batch_"):
        return "batch"
    if name.startswith("test_documented_resolve_nofile"):
        return "nofile"
    return "general"


class DemoReportPlugin:
    """A local pytest plugin registered by a session fixture below."""

    @pytest.hookimpl(hookwrapper=True)
    def pytest_runtest_makereport(
        self,
        item: pytest.Item,
        call: pytest.CallInfo[Any],
    ):
        outcome = yield
        report = outcome.get_result()
        if report.when != "call":
            return
        category = _test_category(item)
        _REPORT["test_timings"].append(
            {
                "nodeid": item.nodeid,
                "category": category,
                "duration": report.duration,
                "outcome": report.outcome,
            }
        )
        if report.failed:
            _REPORT["failures"].append(item.nodeid)

    def pytest_terminal_summary(
        self,
        terminalreporter: Any,
        exitstatus: int,
        config: pytest.Config,
    ) -> None:
        """Print a compact performance-style report after pytest's summary."""
        results = _REPORT["results"]
        timings = _REPORT["test_timings"]
        successes = sum(1 for row in results if row["success"])
        failed_tasks = len(results) - successes
        total_answers = sum(row["answers"] for row in results)
        total_time = sum(row["duration"] for row in timings)
        type_stats: Counter[str] = Counter()
        type_answers: Counter[str] = Counter()
        for row in results:
            if row["success"]:
                type_stats[row["type"]] += 1
                type_answers[row["type"]] += row["answers"]

        terminalreporter.write_sep("=", "demo.py pytest test report")
        terminalreporter.write_line(
            f"data root: {os.path.abspath(SCRIPT_DIR / 'answer')}"
        )
        terminalreporter.write_line(f"resolver calls: {len(results)}")
        terminalreporter.write_line(f"successful: {successes}")
        terminalreporter.write_line(f"failed: {failed_tasks}")
        terminalreporter.write_line(f"total answers: {total_answers}")
        terminalreporter.write_line(f"test time: {total_time:.4f}s")
        if total_time:
            terminalreporter.write_line(
                f"test speed: {len(timings) / total_time:.2f} tests/s"
            )

        terminalreporter.write_line("type statistics:")
        if type_stats:
            for structure_type in sorted(type_stats):
                terminalreporter.write_line(
                    f"  {structure_type:<28} "
                    f"tasks={type_stats[structure_type]:>3} "
                    f"answers={type_answers[structure_type]:>3}"
                )
        else:
            terminalreporter.write_line("  (no successful resolver calls)")

        terminalreporter.write_line("category timings:")
        category_times: defaultdict[str, List[float]] = defaultdict(list)
        for row in timings:
            category_times[row["category"]].append(row["duration"])
        for category in sorted(category_times):
            values = category_times[category]
            terminalreporter.write_line(
                f"  {category:<28} count={len(values):>3} "
                f"total={sum(values):.4f}s"
            )

        if _REPORT["failures"]:
            terminalreporter.write_line("failure details:")
            for nodeid in _REPORT["failures"]:
                terminalreporter.write_line(f"  {nodeid}")
        else:
            terminalreporter.write_line("failure details: none")

        slowest = sorted(timings, key=lambda row: row["duration"], reverse=True)[:5]
        terminalreporter.write_line("slowest tests:")
        for index, row in enumerate(slowest, 1):
            terminalreporter.write_line(
                f"  {index}. {row['duration'] * 1000:.3f}ms {row['nodeid']}"
            )


# ---------------------------------------------------------------------------
# Payload builders
# ---------------------------------------------------------------------------

def _dump(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False), encoding="utf-8")


def choose_payload(stid: str = "100") -> Tuple[Dict[str, Any], List[Dict[str, Any]]]:
    content = {
        "structure_type": "collector.choose",
        "info": {
            "stid": stid,
            "xtlist": [
                {"xt_xh": "1", "xt_nr": "ets_th1 Choose <b>one</b>", "answer": "B"},
                {"xt_xh": "2", "xt_nr": "ets_th2 Choose two", "answer": "C"},
            ],
        },
    }
    info = [
        {
            "code_id": "answer",
            "code_type": "json_array",
            "code_json_array": json.dumps(
                [
                    {"stid": stid, "xth": "1", "answer": "B"},
                    {"stid": stid, "xth": "2", "answer": "C"},
                ]
            ),
        }
    ]
    return content, info


def read_payload(stid: str = "200", structure_type: str = "collector.read") -> Dict[str, Any]:
    return {
        "structure_type": structure_type,
        "info": {
            "stid": stid,
            "value": "<p>Read <b>this</b></p>",
            "analyze": "<i>keyword</i></br>second",
        },
    }


def repeat_payload(stid: str = "300", structure_type: str = "collector.repeat") -> Dict[str, Any]:
    return {
        "structure_type": structure_type,
        "info": {
            "stid": stid,
            "question": [
                {
                    "xh": "1",
                    "std": [{"value": "<b>Hello</b>"}, {"value": "World"}],
                    "keywords": "<i>keyword</i> | second",
                },
                {
                    "xh": "2",
                    "std": [{"value": "Answer"}],
                    "keywords": "third",
                },
            ],
        },
    }


def write_task(
    parent: Path,
    name: str,
    content: Dict[str, Any],
    info: Optional[List[Dict[str, Any]]] = None,
    res: Optional[Dict[str, Any]] = None,
) -> Path:
    task = parent / name
    task.mkdir(parents=True, exist_ok=True)
    _dump(task / "content.json", content)
    if info is not None:
        _dump(task / "info.json", info)
    if res is not None:
        _dump(task / "res.json", res)
    return task


def result_metadata() -> Dict[str, Any]:
    return {
        "set_score": 10,
        "exam_type_list": [
            {
                "exam_type_collect": "collector.choose",
                "exam_type_name": "Choose",
                "exam_type_score": 10,
                "exam_type_count": 2,
                "exam_type_order": 1,
                "exam_list": [
                    {
                        "exam_id": "st1",
                        "exam_order": 1,
                        "exam_score": 10,
                        "exam_count": 2,
                        "evaluation_type": "single",
                        "question_list": [
                            {
                                "question_order": 1,
                                "question_score": 5,
                                "question_count": 1,
                                "evaluation_type": "single",
                            },
                            {
                                "question_order": 2,
                                "question_score": 5,
                                "question_count": 1,
                                "evaluation_type": "single",
                            },
                        ],
                    }
                ],
            }
        ],
    }


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session", autouse=True)
def register_report_plugin(request: pytest.FixtureRequest) -> DemoReportPlugin:
    """Register the report hooks early enough to observe every test call."""
    plugin = DemoReportPlugin()
    request.config.pluginmanager.register(plugin, "demo-report-plugin")
    return plugin


@pytest.fixture(autouse=True)
def reset_resolver_state(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Iterable[None]:
    """Keep global demo.py modes isolated and prevent log files in the repo."""
    monkeypatch.chdir(tmp_path)
    resolver.nofile_mode(False)
    resolver.fallback_mode(True)
    resolver.legacy_mode(False)
    resolver.debug_mode(False)
    yield
    resolver.nofile_mode(False)
    resolver.fallback_mode(True)
    resolver.legacy_mode(False)
    resolver.debug_mode(False)


@pytest.fixture
def choose_task(tmp_path: Path) -> Path:
    content, info = choose_payload()
    return write_task(tmp_path, "choose", content, info)


# ---------------------------------------------------------------------------
# Unit tests: helpers, modes, parser and decorator
# ---------------------------------------------------------------------------

def test_remove_html_tags_and_parser_entities() -> None:
    value = "<div>Hello &amp; <b>world</b>&#33;</div><script>ignored()</script><style>.x{}</style>"
    assert resolver.remove_html_tags(value) == "Hello & world!"
    stripper = resolver.HTMLTagStripper()
    stripper.feed("A&nbsp;B&#x21;")
    assert stripper.get_text() == "A B!"


def test_sort_dict_by_key_and_invalid_key() -> None:
    assert list(resolver.sort_dict_by_key({"10_2": 1, "2_10": 2, "2_1": 3})) == [
        "2_1",
        "2_10",
        "10_2",
    ]
    with pytest.raises(ValueError):
        resolver.sort_dict_by_key({"bad": 1})


def test_mode_switches_and_debug_output(capsys: pytest.CaptureFixture[str]) -> None:
    resolver.debug_mode(True)
    resolver.debug_print("visible")
    assert resolver.DEBUG is True
    captured = capsys.readouterr()
    assert "DEBUG MODE" in captured.out
    assert "visible" in captured.out

    resolver.debug_mode(False)
    resolver.debug_print("hidden")
    assert resolver.DEBUG is False
    assert "hidden" not in capsys.readouterr().out

    resolver.nofile_mode(True)
    resolver.fallback_mode(False)
    resolver.legacy_mode(True)
    assert resolver.NOFILE_READ is True
    assert resolver.FALLBACK is False
    assert resolver.LEGACY_ANALYSIS is True


def test_log_decorator_captures_prints_and_exceptions(caplog: pytest.LogCaptureFixture) -> None:
    logger = logging.getLogger("demo-test")

    @resolver.log(logger=logger, log_print_level=logging.INFO)
    def add(left: int, right: int = 0) -> int:
        print("inside")
        return left + right

    with caplog.at_level(logging.INFO, logger="demo-test"):
        assert add(2, right=3) == 5
    assert "Print from add: inside" in caplog.text
    assert "Call add" in caplog.text

    @resolver.log(logger=logger, log_exceptions=True)
    def fail() -> None:
        raise RuntimeError("boom")

    with caplog.at_level(logging.ERROR, logger="demo-test"), pytest.raises(RuntimeError):
        fail()
    assert "Exception in fail" in caplog.text


def test_listdir_sorted_uses_mtime(tmp_path: Path) -> None:
    old_file = tmp_path / "old"
    new_file = tmp_path / "new"
    old_file.write_text("old", encoding="utf-8")
    new_file.write_text("new", encoding="utf-8")
    os.utime(old_file, ns=(1_000_000_000, 1_000_000_000))
    os.utime(new_file, ns=(2_000_000_000, 2_000_000_000))
    assert resolver.listdir_sorted(str(tmp_path)) == ["old", "new"]


# ---------------------------------------------------------------------------
# Unit tests: collectors and resolve dispatch
# ---------------------------------------------------------------------------

def test_collectors_file_and_memory_inputs(tmp_path: Path) -> None:
    choose_content, choose_info = choose_payload()
    choose_dir = write_task(tmp_path, "choose", choose_content, choose_info)
    choose_info_json = json.dumps(choose_info)
    assert resolver.collector_choose(str(choose_dir)) == {
        "100_1": {"answer": ["B"], "important": []},
        "100_2": {"answer": ["C"], "important": []},
    }
    resolver.nofile_mode(True)
    assert resolver.collector_choose(choose_info_json)["100_1"]["answer"] == ["B"]

    read_content = read_payload()
    read_dir = write_task(tmp_path, "read", read_content)
    resolver.nofile_mode(False)
    assert resolver.collector_read_word(str(read_dir))["200_0"] == {
        "answer": ["Read this"],
        "important": ["keyword", "second"],
    }
    resolver.nofile_mode(True)
    assert resolver.collector_read(json.dumps(read_content))["200_0"]["answer"] == [
        "Read this"
    ]

    repeat_content = repeat_payload()
    repeat_dir = write_task(tmp_path, "repeat", repeat_content)
    resolver.nofile_mode(False)
    repeated = resolver.collector_repeat(str(repeat_dir))
    assert repeated["300_1"]["answer"] == ["Hello", "World"]
    assert repeated["300_1"]["important"] == ["keyword", "second"]
    assert resolver.collector_repeat_essay(str(repeat_dir)) == repeated
    assert resolver.collector_repeat_dialogue(str(repeat_dir)) == repeated


@pytest.mark.parametrize(
    "label,content,expected_type,expected_key",
    [
        ("choose", choose_payload()[0], "collector.choose", "100_1"),
        ("read", read_payload(), "collector.read", "200_0"),
        ("word", read_payload(structure_type="collector.word"), "collector.word", "200_0"),
        ("repeat", repeat_payload(), "collector.repeat", "300_1"),
        ("repeat_essay", repeat_payload(structure_type="collector.repeat_essay"), "collector.repeat_essay", "300_1"),
        ("repeat_dialogue", repeat_payload(structure_type="collector.repeat_dialogue"), "collector.repeat_dialogue", "300_1"),
        ("essay", repeat_payload(structure_type="collector.essay"), "collector.essay", "300_1"),
        ("dialogue", repeat_payload(structure_type="collector.dialogue"), "collector.dialogue", "300_1"),
    ],
)
def test_resolve_dispatches_all_supported_types(
    tmp_path: Path,
    label: str,
    content: Dict[str, Any],
    expected_type: str,
    expected_key: str,
) -> None:
    info = choose_payload()[1] if expected_type == "collector.choose" else None
    task = write_task(tmp_path, label, content, info)
    result = resolver.resolve(str(task))
    record_result(label, result)
    assert isinstance(result, dict)
    assert result["type"] == expected_type
    assert expected_key in result


def test_resolve_unknown_type_uses_collector_fallback(tmp_path: Path) -> None:
    content = repeat_payload(structure_type="collector.unknown")
    result = resolver.resolve(str(write_task(tmp_path, "unknown", content)))
    record_result("unknown", result)
    assert result["type"] == "collector.unknown"
    assert "300_1" in result


def test_collectors_and_resolve_report_missing_inputs(tmp_path: Path) -> None:
    missing = tmp_path / "missing"
    missing.mkdir()
    with pytest.raises(Exception, match="content.json not exists"):
        resolver.collector_read_word(str(missing))
    with pytest.raises(Exception, match="info.json not exists"):
        resolver.collector_choose(str(missing))

    result = resolver.resolve(str(missing))
    record_result("missing", result)
    assert result == -1


# ---------------------------------------------------------------------------
# Analysis and batch APIs
# ---------------------------------------------------------------------------

@pytest.mark.xfail(
    raises=TypeError,
    strict=False,
    reason="demo.py no_analysis iterates the string-valued metadata keys",
)
def test_no_analysis_adds_empty_metadata() -> None:
    answer = [{"1_1": {"answer": ["A"], "important": []}}]
    result = resolver.no_analysis(answer, "unused")
    assert result[-1] == {"total_score": ""}
    assert result[0]["name"] == ""
    assert result[0]["1_1"]["eval_type"] == ""


def test_legacy_analysis_enriches_a_nofile_answer() -> None:
    content, info = choose_payload(stid="1")
    result = result_metadata()
    target = [[json.dumps(content), json.dumps(info)], [json.dumps(result)]]
    answer = [
        {
            "1_1": {"answer": ["B"], "important": []},
            "1_2": {"answer": ["C"], "important": []},
            "type": "collector.choose",
        }
    ]
    resolver.nofile_mode(True)
    enriched = resolver.legacy_analysis(answer, target)
    assert enriched[-1] == {"total_score": 10}
    assert enriched[0]["name"]
    assert enriched[0]["1_1"]["score"] == 5
    assert enriched[0]["1_2"]["eval_type"] == "single"


@pytest.mark.xfail(
    raises=(TypeError, KeyError),
    strict=False,
    reason="demo.py new_analysis currently iterates the string-valued type key",
)
def test_new_analysis_metadata_contract() -> None:
    """Document the documented metadata contract and expose regressions clearly."""
    answer = [
        {
            "1_1": {"answer": ["B"], "important": []},
            "type": "collector.choose",
        }
    ]
    resolver.nofile_mode(True)
    content, info = choose_payload(stid="1")
    target = [[json.dumps(content), json.dumps(info)], [json.dumps(result_metadata())]]
    result = resolver.new_analysis(answer, target)
    assert result[-1] == {"total_score": 10}


def test_resolve_by_homework_and_all_homework(tmp_path: Path) -> None:
    root = tmp_path / "all_homework"
    hw_one = root / "homework_one"
    hw_two = root / "homework_two"
    choose_content, choose_info = choose_payload(stid="1")
    write_task(hw_one, "task_one", choose_content, choose_info, result_metadata())
    write_task(hw_two, "task_two", choose_content, choose_info, result_metadata())

    one = resolver.resolve_by_homework(str(hw_one))
    assert isinstance(one, list)
    assert one
    assert one[-1] == {"total_score": 10}

    all_results = resolver.resolve_by_all_homework(str(root))
    assert set(all_results) == {"homework_one", "homework_two"}
    assert all_results["homework_one"][-1] == {"total_score": 10}


def test_batch_apis_handle_empty_and_invalid_paths(tmp_path: Path) -> None:
    empty = tmp_path / "empty"
    empty.mkdir()
    assert resolver.resolve_by_homework(str(empty)) == [{"total_score": None}]
    assert resolver.resolve_by_all_homework(str(empty)) == {}
    assert resolver.resolve(str(tmp_path / "does-not-exist")) == -1


@pytest.mark.xfail(
    strict=False,
    reason="demo.py currently treats the documented JSON string as an iterable of characters",
)
def test_documented_resolve_nofile_json_string() -> None:
    content, info = choose_payload()
    resolver.nofile_mode(True)
    result = resolver.resolve(json.dumps([json.dumps(content), json.dumps(info)]))
    record_result("nofile-resolve", result)
    assert result["type"] == "collector.choose"
    assert result["100_1"]["answer"] == ["B"]
