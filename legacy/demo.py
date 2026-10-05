# -*- coding: utf-8 -*-
# fuck microsoft
import os, json
from html.parser import HTMLParser
import logging
import time
import io
import sys
from functools import wraps
from typing import Callable, Optional
import traceback
import copy
import this

# TARGET = "./answer"
DEBUG = False
NOFILE_READ = False
LEGACY_ANALYSIS = False
FALLBACK = True # 当new_analysis或legacy_analysis失效时，自动回退到其他版本，如果还是失败，则回退到不调用*_analysis函数的结果
SAVE_FILE = False

# 选择题答案从info.json提取，非选择题答案从content.json提取

# NOFILE_READ == True时，调用resolve、resolve_by_homework、resolve_by_all_homework，传入str内部应为list
# resolve_by_homework："[[content1.json, info1.json], [content2.json, info2.json], ..., res.json]"
# resolve："[content.json, info.json]"
# resolve_by_all_homework："[[[content1.json, info1.json], [content2.json, info2.json], ..., res.json], ...]"

# export fallback_mode(bool), legacy_mode(bool), debug_mode(bool), nofile_mode(bool), resolve(str), resolve_by_homework(str), resolve_by_all_homework(str) function

# 注意：fallback_mode、legacy_mode、nofile_mode、debug_mode需要在其他导出函数调用前调用，否则无效

def save_mode(mode: bool):
    global SAVE_FILE
    SAVE_FILE = mode
    return

def fallback_mode(mode: bool):
    global FALLBACK
    FALLBACK = mode
    return

def legacy_mode(mode: bool):
    global LEGACY_ANALYSIS
    LEGACY_ANALYSIS = mode
    return

def nofile_mode(mode: bool):
    global NOFILE_READ
    NOFILE_READ = mode
    return

def debug_mode(mode: bool):
    global DEBUG
    DEBUG = mode
    debug_log()
    return

def debug_log():
    try:
        os.mkdir('./log')
    except:
        pass
    if DEBUG == True:
        logging.basicConfig(level = logging.DEBUG,
            format = '%(asctime)s [%(levelname)s] at %(filename)s,%(lineno)d: %(message)s',
            datefmt = '%Y-%m-%d(%a)%H:%M:%S',
            filename = f'./log/debug_out_{int(time.time())}.txt',
            filemode = 'w')
        print("DEBUG MODE")
    else:
        logging.basicConfig(level = logging.INFO,
            format = '%(asctime)s [%(levelname)s] at %(filename)s,%(lineno)d: %(message)s',
            datefmt = '%Y-%m-%d(%a)%H:%M:%S',
            filename = f'./log/info_out_{int(time.time())}.txt',
            filemode = 'w')
    return

def debug_print(s):
    if DEBUG == True:
        print("debug_print: ", s)
    return

def return_hook(func):
    @wraps(func)
    def wrapper(*args, **kwargs):
        result = func(*args, **kwargs)
        if SAVE_FILE == True:
            try:
                os.mkdir("./answer")
            except:
                pass
            try:
                with open(f"./answer/answer_{int(time.time())}.txt", "w", encoding="utf-8") as f:
                    f.write(result)
            except:
                pass
            return result
        else:
            return result
    return wrapper

def log_print(func):
    @wraps(func)
    def wrapper(*args, **kwargs):
        try:
            os.mkdir("./log")
        except:
            pass
        return 
    return wrapper
def log(
    logger: Optional[logging.Logger] = None,
    log_level: int = logging.INFO,
    log_args: bool = True,
    log_return: bool = True,
    log_time: bool = True,
    log_exceptions: bool = True,
    log_print_level: Optional[int] = logging.INFO,  # 新增：捕获print输出的日志级别，None表示不捕获
) -> Callable:
    """
    一个可配置的日志装饰器，支持捕获函数内的 print 输出。

    参数:
        logger: 指定的 logger 实例，如果为 None 则使用 root logger。
        log_level: 日志级别，默认为 INFO。
        log_args: 是否记录函数参数。
        log_return: 是否记录返回值。
        log_time: 是否记录执行耗时。
        log_exceptions: 是否记录异常（并重新抛出）。
        log_print_level: 若指定（例如 logging.INFO），则捕获函数内所有 print 输出，
                         以该级别记录到日志，每条输出前缀为 "Print from {func_name}: "。
                         默认 None 表示不捕获。
    """
    if logger is None:
        logger = logging.getLogger()  # root logger

    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            func_name = func.__name__
            # 构造参数表示（如果启用）
            args_repr = ""
            if log_args:
                args_str = ", ".join(repr(a) for a in args)
                kwargs_str = ", ".join(f"{k}={repr(v)}" for k, v in kwargs.items())
                args_repr = f"({', '.join(filter(None, [args_str, kwargs_str]))})"

            start_time = time.perf_counter() if log_time else None

            # 如果启用捕获 print，则重定向 stdout
            old_stdout = None
            stdout_capture = None
            if log_print_level is not None:
                old_stdout = sys.stdout
                stdout_capture = io.StringIO()
                sys.stdout = stdout_capture

            try:
                result = func(*args, **kwargs)

                # 记录捕获的 print 输出（成功时）
                if stdout_capture is not None:
                    output = stdout_capture.getvalue()
                    if output:
                        for line in output.splitlines():
                            if line:  # 保留空行？这里忽略空行，可根据需要调整
                                logger.log(log_print_level, f"Print from {func_name}: {line}")

                # 记录成功调用
                msg = f"Call {func_name}{args_repr}"
                if log_return:
                    msg += f" -> {repr(result)}"
                if log_time and start_time is not None:
                    elapsed = time.perf_counter() - start_time
                    msg += f" [elapsed: {elapsed:.6f}s]"
                logger.log(log_level, msg)
                return result

            except Exception as e:
                # 记录捕获的 print 输出（异常时）
                if stdout_capture is not None:
                    output = stdout_capture.getvalue()
                    if output:
                        for line in output.splitlines():
                            if line:
                                logger.log(log_print_level, f"Print from {func_name}: {line}")

                # 记录异常（如果启用）
                if log_exceptions:
                    elapsed = time.perf_counter() - start_time if start_time is not None else None
                    msg = f"Exception in {func_name}{args_repr}: {repr(e)}"
                    if elapsed is not None:
                        msg += f" [elapsed: {elapsed:.6f}s]"
                    logger.log(logging.ERROR, msg)  # 异常总是用 ERROR 级别
                # 重新抛出异常
                raise

            finally:
                # 恢复 stdout
                if old_stdout is not None:
                    sys.stdout = old_stdout

        return wrapper
    return decorator

def sort_dict_by_key(d: dict) -> dict:
    """
    将字典按键 'xxxx_x' 排序，先按 xxxx 升序，再按 x 升序。
    键必须是 '数字_数字' 格式。
    """
    # 使用 sorted 对 items 排序，key 为 (int(xxxx), int(x))
    sorted_items = sorted(
        d.items(),
        key=lambda item: tuple(map(int, item[0].split('_')))
    )
    # 返回有序字典（Python 3.7+ 直接 dict 即可）
    return dict(sorted_items)

class HTMLTagStripper(HTMLParser):
    """用于移除 HTML 标签的解析器，仅提取文本内容"""
    def __init__(self):
        super().__init__()
        self.reset()
        self.text = []
        self.ignore_data = False  # 是否忽略数据（用于 script/style）

    def handle_starttag(self, tag, attrs):
        # 遇到 <script> 或 <style> 时开始忽略内部内容
        if tag in ('script', 'style'):
            self.ignore_data = True

    def handle_endtag(self, tag):
        # 遇到 </script> 或 </style> 时恢复提取
        if tag in ('script', 'style'):
            self.ignore_data = False

    def handle_data(self, data):
        # 只保留非忽略状态下的文本
        if not self.ignore_data:
            self.text.append(data)

    def handle_entityref(self, name):
        # 实体引用转换为对应字符（例如 &amp; -> &）
        if not self.ignore_data:
            self.text.append(self.unescape(f'&{name};'))

    def handle_charref(self, name):
        # 字符引用（数字形式，如 &#38;）
        if not self.ignore_data:
            self.text.append(self.unescape(f'&#{name};'))

    def get_text(self):
        return ''.join(self.text)
def remove_html_tags(html_string: str) -> str:
    """删除字符串中的所有 HTML 标记，仅返回纯文本内容"""
    stripper = HTMLTagStripper()
    stripper.feed(html_string)
    return stripper.get_text()

# 废弃函数
# 选择题不要调用这个函数
# 选择题，code_type = json_array(4)，答案从info.json解析
# 其他所有，code_type != json_array(4)，答案从content.json解析
# def resolve_by_type(code_type):
#     if (code_type == "txt") | (code_type == "image"):

#     return answerkey

def collector_choose(TARGET):
    choose_answer_dict = {}

    if NOFILE_READ == True:
        # TARGET此时应为info.json的具体内容，需要json.loads加载解析
        info = json.loads(TARGET)
    elif os.path.exists(os.path.join(TARGET, "info.json")):
        with open(os.path.join(TARGET, "info.json"), "r", encoding='utf-8') as k:
            info = json.load(k)
    else:
        raise Exception("info.json not exists")
    
    for item in info:
        if (item["code_id"] == "answer") & (item["code_type"] == "json_array"):
            # 4
            # 解析code_json_array的answer项
            code_json_array = json.loads(item["code_json_array"])
            for answer in code_json_array:
                # xth用于区分第几道题
                key = answer["stid"] + "_" + answer["xth"]
                value = {"answer": [remove_html_tags(answer["answer"])], "important": {"analyze": [], "key": []}} # important为关键词或解析信息，选择题没有关键词
                choose_answer_dict[key] = value
    for item in info:
        if "code_json_obj" in item.keys():
            quest = item["code_json_obj"]
            stid = quest["stid"]
            for questlist in quest["xtlist"]:
                questindex = questlist["xt_xh"]
                key = stid + "_" + questindex
                q = questlist["xt_nr"]
    
    return sort_dict_by_key(choose_answer_dict)

def collector_read_word(TARGET):
    read_word_answer_dict = {}

    if NOFILE_READ == True:
        # TARGET此时应为content.json的具体内容，需要json.loads加载解析
        txt = json.loads(TARGET)
    elif os.path.exists(os.path.join(TARGET, "content.json")):
        # with open(os.path.join(TARGET, "info.json"), "r") as k:
        with open(os.path.join(TARGET, "content.json"), "r", encoding='utf-8') as m:
            # info = json.load(k)
            # for item in info:
            #     if (item["code_id"] == "value_all") & (item["code_type"] == "txt"):
            #         # 1
            #         # 解析txt的answer项
            txt = json.load(m)
    else:
        raise Exception("content.json not exists") # 懒
    
    debug_print(txt)
    key = txt["info"]["stid"] + "_0"
    value = {"answer": [remove_html_tags(txt["info"]["value"])], 
             "important": {"analyze": [remove_html_tags(s) for s in txt["info"]["analyze"].split("</br>")], 
                           "key": []}}
    read_word_answer_dict[key] = value
    
    return sort_dict_by_key(read_word_answer_dict)

def collector_read(TARGET):
    return collector_read_word(TARGET)

def collector_word(TARGET):
    return collector_read_word(TARGET)

# maybe
def collector_repeat(TARGET):
    repeat_answer_dict = {}

    if NOFILE_READ == True:
        # TARGET此时应为content.json的具体内容，需要json.loads加载解析
        txt = json.loads(TARGET)
    elif os.path.exists(os.path.join(TARGET, "content.json")):
        with open(os.path.join(TARGET, "content.json"), "r", encoding='utf-8') as m:
            txt = json.load(m)
    else:
        raise Exception("content.json not exists") # 懒
    
    debug_print(txt)
    key = txt["info"]["stid"] + "_0"
    answer_list = []
    for answer in txt["info"]["std"]:
        answer_list.append(remove_html_tags(answer["value"]))
    value = {"answer": answer_list, 
             "important": {"analyze": [remove_html_tags(s) for s in txt["info"]["analyze"].split("</br>")], 
                           "key": [remove_html_tags(s) for s in txt["info"]["keypoint"].split("</br>")]}}
    repeat_answer_dict[key] = value
    
    return sort_dict_by_key(repeat_answer_dict)

def collector_repeat_essay_dialogue(TARGET):
    essay_dialogue_answer_dict = {}

    if NOFILE_READ == True:
        # TARGET此时应为content.json的具体内容，需要json.loads加载解析
        txt = json.loads(TARGET)
    elif os.path.exists(os.path.join(TARGET, "content.json")):
        with open(os.path.join(TARGET, "content.json"), "r", encoding='utf-8') as m:
            txt = json.load(m)
    else:
        raise Exception("content.json not exists")
    
    for question in txt["info"]["question"]:
        part_answer = []
        for std in question["std"]:
            part_answer.append(remove_html_tags(std["value"]))
        key = txt["info"]["stid"] + "_" + question["xh"]
        value = {"answer": part_answer, "important": {"analyze": [remove_html_tags(s) for s in question["analyze"].split("</br>")], 
                                                      "key": [remove_html_tags(s) for s in question["keywords"].split(" | ")]}}
        essay_dialogue_answer_dict[key] = value
    
    return sort_dict_by_key(essay_dialogue_answer_dict)

def collector_repeat_essay(TARGET):
    return collector_repeat_essay_dialogue(TARGET)

def collector_repeat_dialogue(TARGET):
    return collector_repeat_essay_dialogue(TARGET)

def collector_others(TARGET):
    # 全都试一遍，成功为止
    try:
        return collector_repeat(TARGET[0])
        # debug_print("Others return repeat")
    except:
        debug_print(f"Others can't be resolved by repeat, path:{TARGET}, try others")
    try:
        return collector_read_word(TARGET[0])
        # debug_print("Others return read_word")
    except:
        debug_print(f"Others can't be resolved by read_word, path:{TARGET}, try others")
    try:
        return collector_repeat_essay_dialogue(TARGET[0])
        # debug_print("Others return essay_dialogue")
    except:
        debug_print(f"Others can't be resolved by essay_dialogue, path:{TARGET}, try others")
    try:
        return collector_choose(TARGET[1])
        # debug_print("Others return choose")
    except:
        debug_print(f"Others can't be resolved by choose, path:{TARGET}, try others")

    raise Exception("Others can't be resolved")

# int __thiscall sub_FCF2F0(void *this)
# {
#     if ( sub_DBEB0((int)this + 8, "txt") )        return 1;
#     if ( sub_DBEB0((int)this + 8, "image") )      return 2;
#     if ( sub_DBEB0((int)this + 8, "json_array") ) return 4;
#     if ( sub_DBEB0((int)this + 8, "json_obj") )   return 3;
#     return 0;
# }

@log()
@return_hook
def resolve(TARGET):
    """
    单答案解析函数
    Args:
        TARGET (str): 当NOFILE_READ == True时, 为"[content.json, info.json]"; 当NOFILE_READ == False时, 为包含info.json的TARGET目录地址
    Returns:
        answer_dict (dict): 解析后的答案字典
        *example (dict): {"stid1_xth1":{"answer":["answer1", "answer2", ...], "important":["important1", "important2", ...]}, "stid2_xth2":{...}, ..., "type": structure_type}
    """

    # 从TARGET获取content.json并解析structure_type值
    try:
        # 这里io其实可以优化下，都是小文件，多一次打开还是比较吃4k读取的
        if NOFILE_READ == True:
            print("NOFILE_READ mode")
            TARGET = list(TARGET)
            content = json.loads(TARGET[0])
        elif os.path.exists(os.path.join(TARGET, "content.json")):
            print("FILE_READ mode")
            with open(os.path.join(TARGET, "content.json"), "r", encoding='utf-8') as f:
                content = json.load(f)
        else:
            raise Exception("content.json not exists")
        
        structure_type = content["structure_type"]
        match structure_type:
            case "collector.choose" | _ if "choose" in structure_type:
                _return = collector_choose(TARGET[1])
            case "collector.read" | _ if "read" in structure_type:
                _return = collector_read(TARGET[0])
            case "collector.word" | _ if "word" in structure_type:
                _return = collector_word(TARGET[0])
            case "collector.repeat" | _ if "repeat" in structure_type:
                _return = collector_repeat(TARGET[0])
            case "collector.repeat_essay" | _ if "repeat_essay" in structure_type:
                _return = collector_repeat_essay(TARGET[0])
            case "collector.repeat_dialogue" | _ if "repeat_dialogue" in structure_type:
                _return = collector_repeat_dialogue(TARGET[0])
            case "collector.essay" | _ if "essay" in structure_type:
                _return = collector_repeat_essay(TARGET[0])
            case "collector.dialogue" | _ if "dialogue" in structure_type:
                _return = collector_repeat_dialogue(TARGET[0])
            case x:
                debug_print(f"Unknown structure_type:{x}, but try to resolve it as others")
                _return = collector_others(TARGET)

        _return["type"] = structure_type
        return _return

    except Exception as e:
        print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}")
        return -1;

def listdir_sorted(path):
    """
    完全兼容 os.listdir(path) 的用法，返回按修改时间升序（从旧到新）排列的名称列表。
    """
    # 获取所有条目（包括文件和目录）
    entries = os.listdir(path)
    # 按完整路径的修改时间排序（os.path.getmtime 对目录同样有效）
    entries.sort(key=lambda name: os.path.getmtime(os.path.join(path, name)))
    return entries

def new_analysis(ANSWER, TARGET):
    """
    元信息解析和插入函数
    Args:
        TARGET (str): 当NOFILE_READ == True时, 为"[[content1.json, info1.json], [content2.json, info2.json], ..., [res1.json]]"; 当NOFILE_READ == False时, 为作业根目录
    Returns:
        answer_dict (dict): 插入后的答案字典
        *example (dict): [{"stid1_xth1":{"answer":["answer1", "answer2", ...], "important":["important1", "important2", ...], 
                        "score":"score", "count":"count", "eval_type":"eval_type"}, "stid2_xth2":{...}, ..., "type": structure_type, 
                        "name": "name", "score": "score", "count": "count"}, ..., {"total_score":"total_score"}]
    """
    if NOFILE_READ == True:
        TARGET = list(TARGET)
        res = json.loads(TARGET[-1][0])
    else:
        for item in os.listdir(TARGET):
            found = False
            item_path = os.path.join(TARGET, item)
            for root, dirs, files in os.walk(item_path):
                if "res.json" in files:
                    found = True
                    break
            if not found:
                continue
            res_path = os.path.join(item_path, "res.json")
            if not os.path.exists(res_path):
                raise Exception("res.json not exists")
            with open(res_path, "r", encoding='utf-8') as f:
                res = json.load(f)

    total_score = {"total_score": res["set_score"]}
    answer_new = []
    for exam_list in res["exam_type_list"]:
        # debug_print(f"ANSWER: {ANSWER}")
        for item in ANSWER:
            answer = {}
            if item["type"] == exam_list["exam_type_collect"]:
                answer = copy.deepcopy(item)
                answer["name"] = exam_list["exam_type_name"]
                answer["score"] = exam_list["exam_type_score"]
                answer["count"] = exam_list["exam_type_count"]
                for exam in exam_list["exam_list"]:
                    # debug_print(f"keys: {list(item.keys())}")
                    for key in list(item.keys())[0:-1]:
                        # debug_print(f"key:{key}")
                        # debug_print(f"item:{item}")
                        # answer[key]有序，不用匹配了
                        answer[key]["score"] = exam["exam_score"]
                        answer[key]["count"] = exam["exam_count"]
                        eval_types = []
                        for questlist in exam["question_list"]:
                            eval_types.append(questlist["evaluation_type"])
                        answer[key]["eval_type"] = eval_types
                    # answer["score"] = exam["exam_score"]
                    # answer["count"] = exam["exam_count"]
                    # eval_types = []
                    # for questlist in exam["question_list"]:
                    #     eval_types.append(questlist["evaluation_type"])
                    # answer["eval_type"] = eval_types
                answer_new.append(answer)
                # break
            else:
                continue
    answer_new.append(total_score)

    return answer_new

def no_analysis(ANSWER, TARGET):
    """
    无元信息解析和插入函数。
    Args:
        ANSWER (list): resolve生成的答案字典列表
        TARGET (str): 当NOFILE_READ == True时，为包含res.json内容的列表；否则为作业根目录
    Returns:
        answer_new (list): 与new_analysis相同结构的元信息增强答案列表
    """
    total_score = {"total_score": ""}
    answer_new = []
    for item in ANSWER:
        answer = {}
        answer = item
        answer["name"] = ""
        answer["score"] = ""
        answer["count"] = ""
        for key in answer.keys():
            # answer[key]有序，不用匹配了
            answer[key]["score"] = ""
            answer[key]["count"] = ""
            answer[key]["eval_type"] = ""
        answer_new.append(answer)
    answer_new.append(total_score)

    return answer_new


# gpt5.6-sol xhigh
# legacy_analysis demo by @rzsgsfm
def legacy_analysis(ANSWER, TARGET):
    """
    旧版元信息解析和插入函数。
    Args:
        ANSWER (list): resolve生成的答案字典列表
        TARGET (str): 当NOFILE_READ == True时，为包含res.json内容的列表；否则为作业根目录
    Returns:
        answer_new (list): 与new_analysis相同结构的元信息增强答案列表
    """
    import re

    type_names = {
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
    region_type_names = {
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
    region_section_order = {
        "广东高中": ["模仿朗读", "3问5答"],
        "广东初中": ["模仿朗读", "听选信息", "回答问题", "信息转述", "提问"],
        "北京初中": ["听后选择", "听后回答", "听后转述", "短文朗读"],
        "北京高中": ["听后选择", "听后记录", "听后转述", "回答问题", "短文朗读"],
    }
    number_fields = (
        "xt_xh", "xth", "xh", "th", "question_no", "questionNo",
        "order", "sort", "index",
    )
    text_number_patterns = (
        re.compile(r"ets_th\s*(\d+)", re.I),
        re.compile(r"第\s*(\d+)\s*题"),
        re.compile(r"^\s*(\d+)\s*[.、．)）]"),
    )

    if NOFILE_READ == True:
        target = list(TARGET)
        if len(target[-1]) == 1:
            res = json.loads(target[-1][0])
            task_inputs = target[:-1]
        else:
            res = None
            task_inputs = target
        task_paths = [str(index) for index in range(len(task_inputs))]
        task_mtimes = [0 for _ in task_inputs]
        contents = [json.loads(item[0]) for item in task_inputs]
        infos = [json.loads(item[1]) for item in task_inputs]
    else:
        task_paths = []
        task_mtimes = []
        contents = []
        infos = []
        for item in listdir_sorted(TARGET):
            item_path = os.path.join(TARGET, item)
            content_path = os.path.join(item_path, "content.json")
            if not os.path.isfile(content_path):
                continue
            with open(content_path, "r", encoding="utf-8") as f:
                contents.append(json.load(f))
            info_path = os.path.join(item_path, "info.json")
            if os.path.isfile(info_path):
                with open(info_path, "r", encoding="utf-8") as f:
                    infos.append(json.load(f))
            else:
                infos.append([])
            task_paths.append(item_path)
            task_mtimes.append(os.path.getmtime(item_path))

        res_paths = [
            os.path.join(TARGET, item, "res.json")
            for item in sorted(os.listdir(TARGET))
            if os.path.isfile(os.path.join(TARGET, item, "res.json"))
        ]
        if res_paths:
            with open(res_paths[0], "r", encoding="utf-8") as f:
                res = json.load(f)
        else:
            res = None

    if len(ANSWER) != len(contents):
        raise ValueError("ANSWER and content.json count mismatch")

    sections = []
    engine_areas = set()
    if res is not None:
        for type_index, exam_type in enumerate(res.get("exam_type_list") or [], 1):
            structure_type = exam_type.get("exam_type_collect", "")
            exams = exam_type.get("exam_list") or (
                [exam_type] if exam_type.get("exam_id") else []
            )
            for exam_index, exam in enumerate(exams, 1):
                match = re.search(r"st(\d+)", str(exam.get("exam_id", "")), re.I)
                template_order = int(match.group(1)) if match else len(sections) + 1
                question_numbers = []
                for exam_info in exam.get("exam_info") or []:
                    value = str(exam_info.get("value", "")).strip().rstrip(".。")
                    if re.fullmatch(r"[1-9]\d*", value):
                        question_numbers.append(int(value))
                for question in exam.get("question_list") or []:
                    area = str(question.get("engine_area", "")).lower()
                    if area:
                        engine_areas.add(area)
                sections.append({
                    "structure_type": structure_type,
                    "type_name": exam_type.get("exam_type_name") or structure_type,
                    "score": exam_type.get("exam_type_score"),
                    "count": exam_type.get("exam_type_count"),
                    "type_order": int(exam_type.get("exam_type_order") or type_index),
                    "exam_order": int(exam.get("exam_order") or exam_index),
                    "template_order": template_order,
                    "question_numbers": question_numbers,
                    "exam": exam,
                })

    structure_types = [content.get("structure_type", "") for content in contents]
    template_types = {section["structure_type"] for section in sections}
    region = "未知"

    if any(area.startswith("guangdong") for area in engine_areas):
        region = "广东高中" if "collector.3q5a" in template_types else "广东初中"
    elif "beijing" in engine_areas:
        region = "北京高中" if template_types & {"collector.fill", "collector.dialogue"} else "北京初中"
    elif template_types:
        if "collector.3q5a" in template_types:
            region = "广东高中"
        elif template_types & {"collector.fill", "collector.dialogue"}:
            region = "北京高中"
        elif "collector.choose" in template_types:
            region = "北京高中" if len(template_types) == 13 else "北京初中"
        elif len(template_types) % 7 == 0 and {"collector.role", "collector.picture"} <= template_types:
            region = "广东初中"
        elif len(template_types) == 3:
            region = "广东高中"
        elif len(template_types) == 10:
            region = "北京初中"
        elif len(template_types) == 13:
            region = "北京高中"

    if region == "未知":
        if "collector.3q5a" in structure_types:
            region = "广东高中"
        elif "collector.fill" in structure_types or "collector.dialogue" in structure_types:
            region = "北京高中"
        elif "collector.choose" in structure_types:
            region = "北京高中" if len(contents) == 13 else "北京初中"
        elif len(contents) % 7 == 0 and "collector.role" in structure_types and "collector.picture" in structure_types:
            region = "广东初中"
        elif len(contents) == 3:
            region = "广东高中"
        elif len(contents) == 10:
            region = "北京初中"
        elif len(contents) == 13:
            region = "北京高中"

    records = []
    for index, answer in enumerate(ANSWER):
        content = contents[index]
        content_info = content.get("info") or {}
        stid_value = str(content_info.get("stid", "")).strip()
        stid = int(stid_value) if re.fullmatch(r"[1-9]\d*", stid_value) else None
        structure_type = content.get("structure_type", "")

        if region == "广东初中" and structure_type == "collector.role":
            first = (content_info.get("question") or [{}])[0]
            ask = str(first.get("ask", ""))
            audio = str(first.get("askaudio", ""))
            if "<br>" in ask.lower() and audio:
                type_name = "听选信息"
            elif audio:
                type_name = "回答问题"
            elif re.search(r"[\u4e00-\u9fff]", ask):
                type_name = "提问"
            else:
                type_name = "听选信息"
        else:
            type_name = region_type_names.get(region, {}).get(
                structure_type,
                type_names.get(structure_type, structure_type or "未知题型"),
            )

        records.append({
            "answer": answer,
            "content": content,
            "info_file": infos[index],
            "resource_id": os.path.basename(task_paths[index]) if NOFILE_READ == False else f"{index:08d}",
            "mtime": task_mtimes[index],
            "stid": stid,
            "structure_type": structure_type,
            "type_name": type_name,
        })

    ordered_records = None
    exact_template_matches = {}
    stids = [record["stid"] for record in records]
    if sections and stids and all(stid is not None for stid in stids):
        base_stid = min(stids)
        template_by_order = {
            section["template_order"]: section for section in sections
        }
        mapped = []
        for record in records:
            section = template_by_order.get(record["stid"] - base_stid + 1)
            if section is None or section["structure_type"] != record["structure_type"]:
                mapped = []
                break
            mapped.append((section, record))
        if len(mapped) == len(records):
            mapped.sort(key=lambda item: (
                item[0]["type_order"],
                item[0]["exam_order"],
                item[0]["template_order"],
            ))
            ordered_records = [record for _, record in mapped]
            exact_template_matches = {
                record["resource_id"]: section for section, record in mapped
            }

    if ordered_records is None and stids and all(stid is not None for stid in stids) and len(set(stids)) == len(stids):
        ordered_records = sorted(records, key=lambda record: record["stid"])

    section_order = region_section_order.get(region, [])
    if ordered_records is None and section_order:
        order_map = {name: index for index, name in enumerate(section_order)}
        ordered_records = sorted(
            records,
            key=lambda record: (
                order_map.get(record["type_name"], 999),
                record["resource_id"],
            ),
        )

    if ordered_records is None:
        ordered_records = sorted(
            records,
            key=lambda record: (-record["mtime"], record["resource_id"]),
        )

    ordered_sections = sorted(
        sections,
        key=lambda section: (
            section["type_order"],
            section["exam_order"],
            section["template_order"],
        ),
    )
    remaining_sections = list(ordered_sections)
    for record in ordered_records:
        matched_section = exact_template_matches.get(record["resource_id"])
        record["template_exact"] = matched_section is not None
        if matched_section is not None:
            remaining_sections.remove(matched_section)
        else:
            for section_index, candidate in enumerate(remaining_sections):
                if candidate["structure_type"] == record["structure_type"]:
                    matched_section = candidate
                    remaining_sections.pop(section_index)
                    break
        record["section"] = matched_section

    answer_new = []
    for record in ordered_records:
        answer = record["answer"]
        content = record["content"]
        data = content.get("info") or {}
        structure_type = record["structure_type"]
        section = record["section"]
        keys = sorted(
            [
                key for key, value in answer.items()
                if isinstance(value, dict) and "answer" in value
            ],
            key=lambda key: tuple(map(int, key.split("_"))),
        )

        if structure_type == "collector.choose":
            raw_questions = list(data.get("xtlist") or [])
            if not raw_questions and isinstance(record["info_file"], list):
                for info_item in record["info_file"]:
                    if info_item.get("code_id") == "answer" and info_item.get("code_type") == "json_array":
                        raw_questions.extend(json.loads(info_item.get("code_json_array", "[]")))
        elif structure_type == "collector.fill":
            raw_questions = list(data.get("std") or [])
        elif structure_type in (
            "collector.role", "collector.3q5a", "collector.dialogue",
            "collector.repeat", "collector.repeat_essay",
            "collector.repeat_dialogue", "collector.essay",
        ):
            raw_questions = list(data.get("question") or data.get("questions") or [])
        elif structure_type in ("collector.picture", "collector.read", "collector.word"):
            raw_questions = [data]
        elif data.get("question") or data.get("questions"):
            raw_questions = list(data.get("question") or data.get("questions") or [])
        elif data.get("std"):
            raw_questions = list(data.get("std") or [])
        else:
            raw_questions = [data]

        question_orders = []
        for key_index, key in enumerate(keys):
            key_subindex = int(key.split("_", 1)[1])
            raw_question = raw_questions[key_index] if key_index < len(raw_questions) else {}
            for candidate in raw_questions:
                candidate_number = None
                for field in ("xth", "xh", "xt_xh", "th"):
                    value = str(candidate.get(field, "")).strip()
                    if re.fullmatch(r"[1-9]\d*", value):
                        candidate_number = int(value)
                        break
                if candidate_number == key_subindex:
                    raw_question = candidate
                    break

            texts = [
                str(raw_question.get(field, ""))
                for field in (
                    "xt_nr", "xt_value", "xt_wj", "ask", "question",
                    "text", "topic", "value",
                )
            ]
            question_order = None
            for pattern in text_number_patterns:
                for text in texts:
                    match = pattern.search(text)
                    if match:
                        question_order = int(match.group(1))
                        break
                if question_order is not None:
                    break
            if question_order is None:
                for field in number_fields:
                    value = str(raw_question.get(field, "")).strip()
                    if re.fullmatch(r"[1-9]\d*", value):
                        question_order = int(value)
                        break
            question_orders.append(question_order or key_index + 1)

        if record["template_exact"] and section["question_numbers"]:
            for index, number in enumerate(section["question_numbers"][:len(question_orders)]):
                question_orders[index] = number

        ordered_keys = [
            key for _, _, key in sorted(
                zip(question_orders, range(len(keys)), keys),
                key=lambda item: (item[0], item[1]),
            )
        ]

        exam = section["exam"] if section is not None else {}
        question_metadata = []
        for metadata_index, question in enumerate(exam.get("question_list") or []):
            value = str(question.get("question_order", "")).strip()
            metadata_order = int(value) if re.fullmatch(r"[1-9]\d*", value) else metadata_index + 1
            question_metadata.append((metadata_order, metadata_index, question))
        question_metadata.sort(key=lambda item: (item[0], item[1]))

        for index, key in enumerate(keys):
            question = question_metadata[index][2] if index < len(question_metadata) else {}
            answer[key]["score"] = question.get(
                "question_score", exam.get("exam_score")
            )
            answer[key]["count"] = question.get(
                "question_count", exam.get("exam_count")
            )
            answer[key]["eval_type"] = question.get(
                "evaluation_type", exam.get("evaluation_type")
            )

        question_values = {key: answer[key] for key in ordered_keys}
        other_values = [
            (key, value) for key, value in answer.items()
            if key not in keys and key not in ("name", "score", "count")
        ]
        answer.clear()
        for key in ordered_keys:
            answer[key] = question_values[key]
        for key, value in other_values:
            answer[key] = value
        answer["name"] = section["type_name"] if record["template_exact"] else record["type_name"]
        answer["score"] = section["score"] if section is not None else None
        answer["count"] = section["count"] if section is not None else None
        answer_new.append(answer)

    answer_new.append({"total_score": res.get("set_score") if res is not None else None})
    return answer_new

@log()
@return_hook
def resolve_by_homework(TARGET):
    """
    单作业解析函数
    Args:
        TARGET (str): 当NOFILE_READ == True时, 为"[[content1.json, info1.json], [content2.json, info2.json], ..., [res1.json]]"; 当NOFILE_READ == False时, 为包含info.json的TARGET目录地址
    Returns:
        answer_dict (dict): 解析后的答案字典
        *example (dict): [new_analysis_return1, new_analysis_return2, ...]
    """

    try:
        answers = []
        if NOFILE_READ == True:
            print("NOFILE_READ mode")
            TARGET = list(TARGET)
            for item in TARGET:
                answers.append(resolve(str(item[:-1])))
        else:
            print("FILE_READ mode")
            for item in listdir_sorted(TARGET):
                found = False
                item_path = os.path.join(TARGET, item)
                for root, dirs, files in os.walk(item_path):
                    if "content.json" in files:
                        found = True
                        break
                if not found:
                    continue
                answers.append(resolve(item_path))
        debug_print(answers)
        # 其实这里也可以用装饰器的，应该就不用这么多if和回退了
        if LEGACY_ANALYSIS == False:
            print("NEW_ANALYSIS mode")
            if FALLBACK == True:
                print("FALLBACK mode")
                try:
                    answers = new_analysis(answers, TARGET)
                except Exception as e:
                    print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}, Fallback to legacy analysis")
                    try:
                        answers = legacy_analysis(answers, TARGET)
                    except Exception as e:
                        print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}, Fallback to no analysis")
                        answers = no_analysis(answers, TARGET)
            else:
                print("NOFALLBACK mode")
                try:
                    answers = new_analysis(answers, TARGET)
                except Exception as e:
                    print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}, Fallback to no analysis")
                    answers = no_analysis(answers, TARGET)
        else:
            print("LEGACY_ANALYSIS mode")
            if FALLBACK == True:
                print("FALLBACK mode")
                try:
                    answers = legacy_analysis(answers, TARGET)
                except Exception as e:
                    print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}, Fallback to new analysis")
                    try:
                        answers = new_analysis(answers, TARGET)
                    except Exception as e:
                        print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}, Fallback to no analysis")
                        answers = no_analysis(answers, TARGET)
            else:
                print("NOFALLBACK mode")
                try:
                    answers = legacy_analysis(answers, TARGET)
                except Exception as e:
                    print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}, Fallback to no analysis")
                    answers = no_analysis(answers, TARGET)
        return answers
    except Exception as e:
        print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}")
        return -1;

@log()
@return_hook
def resolve_by_all_homework(TARGET):
    """
    全部作业解析函数
    Args:
        TARGET (str): 当NOFILE_READ == True时, 为"[[[content1.json, info1.json], [content2.json, info2.json], ..., [res1.json]], ...]"; 当NOFILE_READ == False时, 为包含info.json的TARGET目录地址
    Returns:
        answer_dict (dict): 解析后的答案字典
        *example (dict): {item1: resolve_by_homework_return1, item2:resolve_by_homework_return2, ...}
    """

    try:
        answers = {}
        if NOFILE_READ == True:
            print("NOFILE_READ mode")
            TARGET = list(TARGET)
            for item in TARGET:
                answers[str(TARGET.index(item))] = resolve_by_homework(str(item))
        else:
            print("FILE_READ mode")
            for item in os.listdir(TARGET):
                path = os.path.join(TARGET, item)
                if os.path.isdir(path):
                    answers[item] = resolve_by_homework(path)
                else:
                    continue
        return answers
    except Exception as e:
        print(f"Error:{e}, path:{TARGET}, line:{traceback.format_exc()}")
        return -1;

# 妈的，快累死我了
if __name__ == "__main__":
    # debug_log()
    debug_mode(True)
    save_mode(True)
    resolve_type: int = int(input("请输入需要解析的类型数字\n1.单文件夹\n2.单作业\n3.所有作业: "))
    path = input("请输入路径: ")
    match resolve_type:
        case 1:
            # test pass
            print(resolve(TARGET = path))
        case 2:
            # test pass
            print(resolve_by_homework(TARGET = path))
        case 3:
            # test pass
            print(resolve_by_all_homework(TARGET = path))
        case _:
            print("请输入正确的数字")
    # print(resolve(TARGET = "./answer"))


debug_mode(False)