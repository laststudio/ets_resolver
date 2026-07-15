# -*- coding: utf-8 -*-
# fuck microsoft
import os, json
from html.parser import HTMLParser
import logging
import time

# TARGET = "./answer"
DEBUG = False
NOFILE_READ = False
LEGACY_ANALYSIS = False

# 选择题答案从info.json提取，非选择题答案从content.json提取

# NOFILE_READ == True时，调用resolve、resolve_by_homework、resolve_by_all_homework，传入str内部应为list
# resolve_by_homework："[[content1.json, info1.json], [content2.json, info2.json], ..., res.json]"
# resolve："[content.json, info.json]"
# resolve_by_all_homework："[[[content1.json, info1.json], [content2.json, info2.json], ..., res.json], ...]"

# export legacy_mode(bool), debug_mode(bool), nofile_mode(bool), resolve(str), resolve_by_homework(str), resolve_by_all_homework(str) function

# 注意：legacy_mode、nofile_mode、debug_mode需要在其他导出函数调用前调用，否则无效

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
    if DEBUG == True:
        logging.basicConfig(level = logging.DEBUG,
            format = '%(asctime)s [%(levelname)s] at %(filename)s,%(lineno)d: %(message)s',
            datefmt = '%Y-%m-%d(%a)%H:%M:%S',
            filename = f'debug_out_{int(time.time())}.txt',
            filemode = 'w')
        print("DEBUG MODE")
    else:
        logging.basicConfig(level = logging.INFO,
            format = '%(asctime)s [%(levelname)s] at %(filename)s,%(lineno)d: %(message)s',
            datefmt = '%Y-%m-%d(%a)%H:%M:%S',
            filename = f'info_out_{int(time.time())}.txt',
            filemode = 'w')
    return

def debug_print(s):
    if DEBUG == True:
        print(s)
    return

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
                value = {"answer": [remove_html_tags(answer["answer"])], "important": []} # important为关键词或解析信息，选择题没有关键词
                choose_answer_dict[key] = value
    
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
    
    key = txt["info"]["stid"] + "_0"
    value = {"answer": [remove_html_tags(txt["info"]["value"])], "important": [remove_html_tags(s) for s in txt["info"]["analyze"].split("</br>")]}
    read_word_answer_dict[key] = value
    
    return sort_dict_by_key(read_word_answer_dict)

def collector_read(TARGET):
    return collector_read_word(TARGET)

def collector_word(TARGET):
    return collector_read_word(TARGET)

def collector_repeat(TARGET):
    # using collector_repeat_essay_dialogue
    return collector_repeat_essay_dialogue(TARGET)

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
        value = {"answer": part_answer, "important": [remove_html_tags(s) for s in question["keywords"].split(" | ")]}
        essay_dialogue_answer_dict[key] = value
    
    return sort_dict_by_key(essay_dialogue_answer_dict)

def collector_repeat_essay(TARGET):
    return collector_repeat_essay_dialogue(TARGET)

def collector_repeat_dialogue(TARGET):
    return collector_repeat_essay_dialogue(TARGET)

def collector_others(TARGET):
    # 全都试一遍，成功为止
    try:
        return collector_repeat(TARGET)
    except:
        debug_print(f"Others can't be resolved by repeat, path:{TARGET}, try others")
    try:
        return collector_choose(TARGET)
    except:
        debug_print(f"Others can't be resolved by choose, path:{TARGET}, try others")
    try:
        return collector_read_word(TARGET)
    except:
        debug_print(f"Others can't be resolved by read_word, path:{TARGET}, try others")
    try:
        return collector_repeat_essay_dialogue(TARGET)
    except:
        debug_print(f"Others can't be resolved by essay_dialogue, path:{TARGET}, try others")

# int __thiscall sub_FCF2F0(void *this)
# {
#     if ( sub_DBEB0((int)this + 8, "txt") )        return 1;
#     if ( sub_DBEB0((int)this + 8, "image") )      return 2;
#     if ( sub_DBEB0((int)this + 8, "json_array") ) return 4;
#     if ( sub_DBEB0((int)this + 8, "json_obj") )   return 3;
#     return 0;
# }

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
            TARGET = list(TARGET)
            content = json.loads(TARGET[0])
        elif os.path.exists(os.path.join(TARGET, "content.json")):
            with open(os.path.join(TARGET, "content.json"), "r", encoding='utf-8') as f:
                content = json.load(f)
        else:
            raise Exception("content.json not exists")
        
        structure_type = content["structure_type"]
        match structure_type:
            case "collector.choose" | _ if "choose" in structure_type:
                _return = collector_choose(TARGET)
            case "collector.read" | _ if "read" in structure_type:
                _return = collector_read(TARGET)
            case "collector.word" | _ if "word" in structure_type:
                _return = collector_word(TARGET)
            case "collector.repeat" | _ if "repeat" in structure_type:
                _return = collector_repeat(TARGET)
            case "collector.repeat_essay" | _ if "repeat_essay" in structure_type:
                _return = collector_repeat_essay(TARGET)
            case "collector.repeat_dialogue" | _ if "repeat_dialogue" in structure_type:
                _return = collector_repeat_dialogue(TARGET)
            case "collector.essay" | _ if "essay" in structure_type:
                _return = collector_repeat_essay(TARGET)
            case "collector.dialogue" | _ if "dialogue" in structure_type:
                _return = collector_repeat_dialogue(TARGET)
            case x:
                debug_print(f"Unknown structure_type:{x}, but try to resolve it as others")
                _return = collector_others(TARGET)

        _return["type"] = structure_type
        return _return

    except Exception as e:
        print(f"Error:{e}, path:{TARGET}")
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
            with open(res_path, "r", encoding='utf-8') as f:
                res = json.load(f)

    total_score = {"total_score": res["set_score"]}
    answer_new = []
    for exam_list in res["exam_type_list"]:
        for item in ANSWER:
            answer = {}
            if item["type"] == exam_list["exam_type_collect"]:
                answer = item
                answer["name"] = exam_list["exam_type_name"]
                answer["score"] = exam_list["exam_type_score"]
                answer["count"] = exam_list["exam_type_count"]
                for exam in exam_list["exam_list"]:
                    for key in answer.keys():
                        # answer[key]有序，不用匹配了
                        answer[key]["score"] = exam["exam_score"]
                        answer[key]["count"] = exam["exam_count"]
                        answer[key]["eval_type"] = exam["evaluation_type"]
                answer_new.append(answer)
                break
            else:
                continue
    answer_new.append(total_score)

    return answer_new

def legacy_analysis(ANSWER, TARGET):
    pass # TODO

def resolve_by_homework(TARGET):
    """
    单作业解析函数
    Args:
        TARGET (str): 当NOFILE_READ == True时, 为"[[content1.json, info1.json], [content2.json, info2.json], ..., [res1.json]]"; 当NOFILE_READ == False时, 为包含info.json的TARGET目录地址
    Returns:
        answer_dict (dict): 解析后的答案字典
        *example (dict): [new_analysis_return1, new_analysis_return2, ...]
    """

    answers = []
    if NOFILE_READ == True:
        TARGET = list(TARGET)
        for item in TARGET:
            answers.append(resolve(str(item[:-1])))
    else:
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
    if LEGACY_ANALYSIS == False:
        answers = new_analysis(answers, TARGET)
    else:
        answers = legacy_analysis(answers, TARGET)
    return answers

def resolve_by_all_homework(TARGET):
    """
    全部作业解析函数
    Args:
        TARGET (str): 当NOFILE_READ == True时, 为"[[[content1.json, info1.json], [content2.json, info2.json], ..., [res1.json]], ...]"; 当NOFILE_READ == False时, 为包含info.json的TARGET目录地址
    Returns:
        answer_dict (dict): 解析后的答案字典
        *example (dict): {item1: resolve_by_homework_return1, item2:resolve_by_homework_return2, ...}
    """

    answers = {}
    if NOFILE_READ == True:
        TARGET = list(TARGET)
        for item in TARGET:
            answers[str(TARGET.index(item))] = resolve_by_homework(str(item))
    else:
        for item in os.listdir(TARGET):
            path = os.path.join(TARGET, item)
            if os.path.isdir(path):
                answers[item] = resolve_by_homework(path)
            else:
                continue
    return answers

# 妈的，快累死我了
if __name__ == "__main__":
    # debug_log()
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