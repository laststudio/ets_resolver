# Android 接入与题目解析说明

本文面向 Kotlin Android 接入，重点说明解析器如何判断题目类型、题目顺序和题目名称。

## 1. Android 简单接入

本项目会编译为 `libresolver.so`。Android 不能直接调用当前 C 接口，需要增加一层 JNI，将 Kotlin 的 `String` 转给 Rust，再把 Rust 输出的 JSON 字符串返回 Kotlin。

推荐只提供两个简单接口：

```kotlin
internal object NativeResolver {
    init {
        System.loadLibrary("resolver")
    }

    external fun resolvePath(taskPath: String): String?

    external fun resolveJson(
        contentJson: String,
        infoJson: String
    ): String?
}
```

- `resolvePath()`：解析包含 `content.json` 和 `info.json` 的任务目录。
- `resolveJson()`：直接解析两个文件的文本，适合 Android 的 `content://` URI。
- JNI 内部直接返回 Java `String`，不要把 Rust 指针暴露给 Kotlin。
- 解析和文件读取应放在后台线程，不要阻塞 Android 主线程。

动态库按 ABI 放置：

```text
app/src/main/jniLibs/
├── arm64-v8a/libresolver.so
└── x86_64/libresolver.so
```

`arm64-v8a` 用于主流真机，`x86_64` 主要用于模拟器。正式发布 AAB 时，用户通常只会下载与设备匹配的一份动态库。

## 2. 解析入口

每个任务至少需要 `content.json`。部分题型还需要 `info.json`：

```text
task/
├── content.json
└── info.json
```

解析器首先读取 `content.json` 的 `structure_type`，再选择对应的答案解析方法。

最终结果目前类似：

```json
{
  "1_1": {
    "answer": ["答案内容"],
    "important": ["重点内容"]
  },
  "type": "collector.choose"
}
```

其中：

- `1_1` 是题目标识和子题序号组成的 key。
- `answer` 是答案列表。
- `important` 是解析出的重点或关键词列表。
- `type` 是 `content.json` 中原始的 `structure_type`。

## 3. 题目类型判断

题目类型来自 `content.json`：

```json
{
  "structure_type": "collector.choose"
}
```

当前判断规则如下：

| `structure_type` 关键字 | 解析方式 | 大致用途 |
|---|---|---|
| `choose` | `collector_choose` | 选择类题目 |
| `read` | `collector_read` | 阅读或单项文本题 |
| `word` | `collector_word` | 单词类题目 |
| `repeat` | `collector_repeat` | 跟读或重复类题目 |
| `repeat_essay` / `essay` | `collector_repeat_essay` | 作文类题目 |
| `repeat_dialogue` / `dialogue` | `collector_repeat_dialogue` | 对话类题目 |
| 未知类型 | `collector_others` | 依次尝试多个解析器 |

Android 可以直接使用返回结果中的 `type` 判断展示方式：

```kotlin
when {
    type.contains("choose") -> showChooseAnswers()
    type.contains("read") -> showReadAnswer()
    type.contains("word") -> showWordAnswer()
    type.contains("essay") -> showEssayAnswer()
    type.contains("dialogue") -> showDialogueAnswer()
    type.contains("repeat") -> showRepeatAnswer()
    else -> showGenericAnswer()
}
```

注意：当前 Rust 代码先判断普通 `repeat`，再判断 `repeat_essay` 和 `repeat_dialogue`。因为这几个解析器目前复用同一套底层逻辑，所以暂时不影响答案；如果以后分别实现，应先判断更具体的类型：

```text
repeat_essay -> repeat_dialogue -> repeat
```

## 4. 题目顺序判断

### 4.1 选择题

选择题从 `info.json` 中读取：

```json
{
  "stid": "1",
  "xth": "2",
  "answer": "答案"
}
```

解析器使用下面的规则生成 key：

```text
stid + "_" + xth
```

上述数据会生成：

```text
1_2
```

其中 `stid` 表示题目或题组标识，`xth` 用于区分该题组中的第几道题。

### 4.2 阅读题和单词题

这类题目从 `content.json.info.stid` 取得题目标识，并固定使用子序号 `0`：

```text
stid + "_0"
```

例如 `stid = 3` 时生成：

```text
3_0
```

### 4.3 作文、对话和重复类题目

这类题目遍历 `content.json.info.question` 数组，从每一项读取 `xh`：

```json
{
  "info": {
    "stid": "4",
    "question": [
      { "xh": "1" },
      { "xh": "2" }
    ]
  }
}
```

生成的 key 为：

```text
4_1
4_2
```

### 4.4 当前排序方法

解析器会把 key 按下划线拆开，并尽量按数字排序。例如：

```text
1_1
1_2
1_10
2_1
```

这可以避免普通字符串排序把 `1_10` 放到 `1_2` 前面。

但 Android 端仍不应依赖 JSON 对象字段的原始顺序，原因如下：

- JSON object 在语义上不是有序列表。
- Kotlin JSON 库转换后可能改变字段遍历顺序。
- 如果 `stid`、`xth` 或 `xh` 不是纯数字，当前数字排序可能失效。
- 批量解析任务目录时使用文件系统遍历顺序，没有明确排序。

Android 端建议重新解析 key 并排序：

```kotlin
data class QuestionOrder(
    val stid: Int,
    val subIndex: Int
)

fun parseOrder(key: String): QuestionOrder? {
    val parts = key.split('_')
    if (parts.size != 2) return null

    return QuestionOrder(
        stid = parts[0].toIntOrNull() ?: return null,
        subIndex = parts[1].toIntOrNull() ?: return null
    )
}
```

排序时先比较 `stid`，再比较 `subIndex`。如果实际数据中的 ID 不是数字，应保留 JSON 数组中的原始位置，或由 Rust 明确返回 `order` 字段。

## 5. 题目名字判断

当前解析器没有解析正式的题目名称或题干，只解析以下内容：

- 题目标识：`stid`。
- 子题序号：`xth` 或 `xh`。
- 答案：`answer` 或 `std.value`。
- 重点：`analyze` 或 `keywords`。
- 题目类型：`structure_type`。

因此返回结果中的 `1_2` 是题目标识，不是“选择正确答案”之类的人类可读名称。

批量解析全部作业时，最外层的作业名来自文件夹名称：

```text
作业根目录/英语第一单元/
```

会得到：

```json
{
  "英语第一单元": []
}
```

这仍然只是目录名，不一定是 JSON 数据中的正式作业标题。

如果 Android 页面需要显示题目名称，需要先检查真实 `content.json` 中存放题干或名称的字段。常见字段可能是：

```text
title
name
question_name
question_text
prompt
```

确认实际字段后，再由 Rust 提取并返回。建议的新结果结构为：

```json
{
  "type": "collector.choose",
  "answers": [
    {
      "id": "1_2",
      "name": "选择正确的答案",
      "stid": "1",
      "order": 2,
      "answer": ["A"],
      "important": []
    }
  ]
}
```

使用数组和显式 `order` 后，Android 不需要依赖 JSON object 的字段顺序。

## 6. 当前能力总结

| 内容 | 当前处理方法 | 是否可直接使用 |
|---|---|---|
| 题目类型 | 读取 `content.json.structure_type` | 可以 |
| 题目标识 | 读取 `stid` | 可以 |
| 子题顺序 | 读取 `xth` 或 `xh` | 可以，但 Android 最好重新排序 |
| 阅读/单词题子序号 | 固定为 `0` | 可以 |
| 作业内任务顺序 | 文件系统遍历顺序 | 不可靠 |
| 正式题目名称 | 当前未解析 | 不可以，需要确认真实字段后补充 |
| 作业名称 | 使用一级文件夹名 | 可以，但不一定是正式标题 |

接入初期可以保持当前返回结构；如果 Android 页面需要稳定展示题目名称和顺序，建议下一步把答案改成数组，并显式返回 `name` 和 `order`。

## 7. 统一 Python 原型

仓库中的 `unified_demo.py` 是合并两种解析方案的 Python 原型。它暂时不替换旧 `demo.py`，用于先验证统一的数据模型和真实样本兼容性。

解析单个任务目录：

```bash
python unified_demo.py path/to/task
```

解析一个 `resource` 分组，并根据题型组合推断地区：

```bash
python unified_demo.py path/to/resource-group --group
```

也可以直接作为 Python 模块调用：

```python
from unified_demo import parse_content, parse_resource_group, parse_task

result = parse_task("path/to/task", region="北京初中")
```

切换题型字段对应规则：

```python
from unified_demo import legacy_mode

legacy_mode(False)  # 新地区规则，默认值
legacy_mode(True)   # 原 ets_resolver 旧规则
```

该开关只改变答案从哪些字段读取，不改变统一输出结构。它是进程级全局状态，与旧版 `nofile_mode()` 一样，应在解析前设置；并发任务不要在执行过程中切换。

统一结果结构：

```json
{
  "region": "北京初中",
  "type": "collector.choose",
  "type_name": "听后选择",
  "questions": [
    {
      "id": "1",
      "order": 1,
      "section_order": 1,
      "name": "What did the man forget to do?",
      "answers": ["C"],
      "options": [
        "A. Take the bag.",
        "B. Lock the door.",
        "C. Leave food for the cat."
      ],
      "original_text": null
    }
  ]
}
```

当前原型结合了以下能力：

- 支持 `role`、`picture`、`read`、`3q5a`、`choose`、`fill`、`dialogue` 等新版结构。
- 保留旧版 `repeat`、`essay`、`word` 和 `info.json.code_json_array` 选择题兼容。
- 根据官方题号字段、`ets_thN`、`第 N 题` 或数组位置确定顺序。
- 提取题目文本、多个参考答案、选择题选项和朗读原文。
- 根据资源数量和特征题型推断广东高中、广东初中、北京初中、北京高中。
- 对新版 `collector.read` 将 `info.value` 作为朗读原文；旧数据存在 `analyze` 时保留原答案语义。

地区推断只是无模板时的兜底。正式实现仍应优先读取 `res.json`、试卷索引或上层明确提供的地区信息，避免仅凭文件夹数量误判。

## 8. 地区与排序可靠性

统一原型不会固定依赖一种推断方式，而是按可靠性使用第一种能够完整成立的方法。结果会返回实际采用的方法，便于 Android 调试和降级提示。

地区判断优先级：

```text
res.json question_list.engine_area                 可靠度 100
res.json 中的特征 structure_type 组合              可靠度 90
content.json 的 3q5a/fill/dialogue 特征            可靠度 85
content.json 的 choose 特征                        可靠度 80
content.json 的完整题型组合                        可靠度 75
resource 文件夹数量                                可靠度 50
无法判断                                            可靠度 0
```

资源包排序优先级：

```text
res.json exam_id + content.json info.stid          可靠度 100
唯一且完整的 content.json info.stid                可靠度 80
地区预定义题型顺序                                  可靠度 60
修改时间 + resource UUID                           可靠度 30
```

题目排序优先级：

```text
res.json exam_info[].value                         最高优先
题目文本中的 ets_thN / 第N题 / N.
xt_xh / xth / xh / th / question_no 等官方字段
JSON 数组位置                                      最终兜底
```

结果中会增加以下元数据：

```json
{
  "region_detection": {
    "value": "北京初中",
    "method": "res.engine_area",
    "reliability": 100
  },
  "resource_order": {
    "method": "res.exam_id+content.stid",
    "reliability": 100
  },
  "sections": [
    {
      "question_order_method": "res.exam_info"
    }
  ]
}
```

只有当前方法缺少数据、无法完整匹配或存在冲突时才回退到下一种方法，不会把低可靠证据覆盖高可靠结果。

## 9. Python 排错接口

公开接口保持与原始 Demo 接近：

```python
from unified_demo import (
    diagnostic_mode,
    nofile_mode,
    resolve,
    resolve_by_homework,
    resolve_by_all_homework,
    resolve_resource,
    resolver_last_error,
    resolver_last_diagnostics,
)

diagnostic_mode(True)
nofile_mode(False)
result = resolve_resource("path/to/resource")
```

成功时直接返回解析数据，不增加 envelope：

```json
{"groups": []}
```

失败时保持原 Python Demo 风格返回 `-1`，通过独立函数读取结构化错误：

```python
result = resolve("path/to/task")
if result == -1:
    print(resolver_last_error())
```

`resolver_last_error()` 示例：

```json
{
  "operation": "resolve",
  "code": "CONTENT_JSON_INVALID",
  "message": "content.json is not valid UTF-8 JSON",
  "stage": "load_content",
  "file": "path/to/content.json",
  "recoverable": false
}
```

默认优先使用 `Fuck_ets100` 的 `content.json + res.json` 新渠道。只有新版选择题不存在 `info.xtlist` 且旧 `info.json.code_json_array` 能成功解析时，才回退旧渠道，并返回：

```json
{
  "channel": "legacy_ets_resolver",
  "warnings": [
    {
      "code": "NEW_FORMAT_UNAVAILABLE",
      "stage": "parse_questions"
    }
  ]
}
```

常用错误码包括：

```text
TARGET_NOT_FOUND
TARGET_NOT_DIRECTORY
CONTENT_NOT_FOUND
CONTENT_READ_FAILED
CONTENT_JSON_INVALID
INFO_JSON_INVALID
RES_JSON_INVALID
STRUCTURE_TYPE_MISSING
QUESTION_NOT_FOUND
LEGACY_FORMAT_INVALID
INTERNAL_ERROR
```

命令行成功时直接输出结果；失败时 stdout 输出 `-1`，stderr 输出结构化错误：

```bash
python unified_demo.py PATH --resource-root --diagnostic
```

```text
退出码 0：成功
退出码 1：解析失败
退出码 2：命令参数错误
```

`--debug` 只在意外内部错误中附带 traceback。正常业务错误始终输出稳定错误码，不依赖 traceback。
