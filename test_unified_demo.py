import json
import tempfile
import unittest
from pathlib import Path

import unified_demo as resolver


class UnifiedDemoTests(unittest.TestCase):
    def tearDown(self):
        resolver.legacy_mode(False)
        resolver.nofile_mode(False)
        resolver.diagnostic_mode(True)
        resolver.debug_mode(False)

    def test_beijing_choose_extracts_name_options_answer_and_order(self):
        content = {
            "structure_type": "collector.choose",
            "info": {"st_nr": "<p>Listening text</p>", "xtlist": [
                {"xt_xh": "2", "xt_nr": "ets_th2 Second?", "answer": "B", "xxlist": [{"xx_mc": "B", "xx_nr": "Two"}]},
                {"xt_xh": "1", "xt_nr": "ets_th1 First?", "answer": "A", "xxlist": [{"xx_mc": "A", "xx_nr": "One"}]},
            ]},
        }
        result = resolver._parse_content(content, region="北京初中")
        self.assertEqual("听后选择", result["type_name"])
        self.assertEqual([1, 2], [item["order"] for item in result["questions"]])
        self.assertEqual("First?", result["questions"][0]["name"])
        self.assertEqual(["A. One"], result["questions"][0]["options"])
        self.assertEqual(["A"], result["questions"][0]["answers"])
        self.assertEqual("Listening text", result["questions"][0]["original_text"])

    def test_role_extracts_multiple_answers_and_text_order(self):
        content = {
            "structure_type": "collector.3q5a",
            "info": {"question": [{
                "ask": "第 3 题 What is his job?",
                "std": [{"value": "A teacher."}, {"value": "He teaches."}],
            }]},
        }
        result = resolver._parse_content(content, region="广东高中")
        question = result["questions"][0]
        self.assertEqual(3, question["order"])
        self.assertEqual(["A teacher.", "He teaches."], question["answers"])

    def test_read_preserves_old_and_new_semantics(self):
        new_result = resolver._parse_content({
            "structure_type": "collector.read",
            "info": {"stid": "1", "value": "<p>Read me</p>", "analyze": ""},
        })
        self.assertEqual([], new_result["questions"][0]["answers"])
        self.assertEqual("Read me", new_result["questions"][0]["original_text"])

        resolver.legacy_mode(True)
        old_result = resolver._parse_content({
            "structure_type": "collector.read",
            "info": {"stid": "1", "value": "Answer", "analyze": "Tip"},
        })
        self.assertEqual(["Answer"], old_result["questions"][0]["answers"])

    def test_legacy_choose_info_is_supported(self):
        info = [{
            "code_id": "answer",
            "code_type": "json_array",
            "code_json_array": json.dumps([{"stid": "4", "xth": "2", "answer": "C"}]),
        }]
        result = resolver._parse_content({"structure_type": "collector.choose", "info": {}}, info)
        self.assertEqual("4_2", result["questions"][0]["id"])
        self.assertEqual(["C"], result["questions"][0]["answers"])

    def test_legacy_mode_switches_choose_and_read_mappings(self):
        resolver.legacy_mode(True)
        resolver.nofile_mode(True)
        choose = resolver.resolve([json.dumps({
            "structure_type": "collector.choose",
            "info": {"xtlist": [{"xt_xh": "1", "answer": "A"}]},
        }), "[]"])
        self.assertEqual(-1, choose)
        self.assertEqual("QUESTION_NOT_FOUND", resolver.resolver_last_error()["code"])

        read = resolver._parse_content({
            "structure_type": "collector.read",
            "info": {"stid": "1", "value": "Read me"},
        })
        self.assertEqual(["Read me"], read["questions"][0]["answers"])
        self.assertIsNone(read["questions"][0]["original_text"])

    def test_group_infers_region(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            structures = ["collector.read", "collector.role", "collector.picture", "collector.role", "collector.role", "collector.role", "collector.role"]
            for index, structure_type in enumerate(structures):
                task = root / str(index)
                task.mkdir()
                if structure_type == "collector.read":
                    info = {"stid": str(index + 1), "value": "Read me"}
                elif structure_type == "collector.picture":
                    info = {"stid": str(index + 1), "topic": "Topic", "std": [{"value": "Answer"}]}
                else:
                    info = {"stid": str(index + 1), "question": [{"xh": "1", "ask": "Question", "std": [{"value": "Answer"}]}]}
                (task / "content.json").write_text(json.dumps({"structure_type": structure_type, "info": info}), encoding="utf-8")
            result = resolver._parse_resource_group(root)
            self.assertEqual("广东初中", result["region"])

    def test_template_has_priority_for_region_resource_and_question_order(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            template_dir = root / "template"
            template_dir.mkdir()
            (template_dir / "res.json").write_text(json.dumps({
                "exam_type_list": [{
                    "exam_type_order": "1",
                    "exam_type_name": "听后选择",
                    "exam_type_collect": "collector.choose",
                    "exam_list": [{
                        "exam_order": "1",
                        "exam_id": "{{st1}}",
                        "exam_info": [{"id": "ets_th1", "value": "3."}],
                        "question_list": [{"engine_area": "beijing"}],
                    }],
                }],
            }), encoding="utf-8")
            task = root / "task"
            task.mkdir()
            (task / "content.json").write_text(json.dumps({
                "structure_type": "collector.choose",
                "info": {"stid": "100", "xtlist": [{"xt_xh": "1", "xt_nr": "Question", "answer": "A"}]},
            }), encoding="utf-8")

            result = resolver._parse_resource_group(root)
            self.assertEqual("res.engine_area", result["region_detection"]["method"])
            self.assertEqual("res.exam_id+content.stid", result["resource_order"]["method"])
            self.assertEqual(3, result["sections"][0]["questions"][0]["order"])
            self.assertEqual("res.exam_info", result["sections"][0]["question_order_method"])

    def test_safe_task_reports_invalid_content_json(self):
        with tempfile.TemporaryDirectory() as directory:
            task = Path(directory)
            (task / "content.json").write_text("{broken", encoding="utf-8")
            resolver.nofile_mode(False)
            result = resolver.resolve(task)
            self.assertEqual(-1, result)
            self.assertEqual("CONTENT_JSON_INVALID", resolver.resolver_last_error()["code"])
            self.assertEqual("load_content", resolver.resolver_last_error()["stage"])

    def test_safe_choose_reports_legacy_fallback(self):
        info = [{
            "code_id": "answer",
            "code_type": "json_array",
            "code_json_array": json.dumps([{"stid": "4", "xth": "2", "answer": "C"}]),
        }]
        resolver.nofile_mode(True)
        result = resolver.resolve([json.dumps({"structure_type": "collector.choose", "info": {}}), json.dumps(info)])
        self.assertNotEqual(-1, result)
        diagnostics = resolver.resolver_last_diagnostics()
        self.assertEqual("legacy_ets_resolver", diagnostics["channel"])
        self.assertEqual("NEW_FORMAT_UNAVAILABLE", diagnostics["warnings"][0]["code"])

    def test_safe_unknown_shape_reports_missing_structure_type(self):
        resolver.nofile_mode(True)
        result = resolver.resolve([json.dumps({"info": {}}), "[]"])
        self.assertEqual(-1, result)
        self.assertEqual("STRUCTURE_TYPE_MISSING", resolver.resolver_last_error()["code"])

    def test_diagnostic_mode_can_hide_success_warnings(self):
        resolver.diagnostic_mode(False)
        resolver.nofile_mode(False)
        result = resolver.resolve_resource("missing-path")
        self.assertEqual(-1, result)
        self.assertNotIn("warnings", resolver.resolver_last_diagnostics())
        self.assertEqual("TARGET_NOT_FOUND", resolver.resolver_last_error()["code"])


if __name__ == "__main__":
    unittest.main()
