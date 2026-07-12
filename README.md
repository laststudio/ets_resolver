# Ets Resolver (Python & Rust)

本项目提供了作业答案解析的两种实现：原生 Python 脚本 (`demo.py`) 与高性能 Rust 动态链接库 (`libresolver.so` / `resolver.dll` 等)。通过 Rust 编译的动态库可以被 Python、C/C++、C#、Java 等其他程序以 FFI 的方式调用。

## 依赖与环境

### Python 版本
- Python 3.10+ (支持 `match-case` 语法)
- 无需额外第三方依赖（仅使用标准库 `os, json, html, logging, time`）

### Rust 版本
- Rust 1.60+
- 依赖：`serde`, `serde_json`

## 在本机编译 Rust 动态库

在项目根目录下执行：
```bash
cargo build --release
```
编译产物位于 `target/release/`：
- Windows: `resolver.dll`
- Android: `libresolver.so`
- Wasm: `resolver.wasm`

## 交叉编译 Rust 动态库

在项目根目录下执行：
```bash
rustup target add aarch64-linux-android wasm32-unknown-unknown x86_64-pc-windows-msvc
cargo install cargo-ndk
cargo build --target x86_64-pc-windows-msvc --release
cargo build --target wasm32-unknown-unknown --release
cargo ndk --target arm64-v8a build --release
```
编译产物位于 `target/release/`：
- Windows: `resolver.dll`
- Android: `libresolver.so`
- Wasm: `resolver.wasm`

---

## 使用方法

### 1. Python 程序 (`demo.py`) 使用

直接运行脚本，根据提示输入：
```bash
python demo.py
```

### 2. Rust 动态链接库使用 (以 Python 为例)

通过 Python 的 `ctypes` 调用 Rust 动态链接库：

```python
import ctypes
import json

lib = ctypes.CDLL("./target/release/libresolver.so")

lib.debug_mode.argtypes = [ctypes.c_bool]
lib.debug_mode.restype = None

lib.nofile_mode.argtypes = [ctypes.c_bool]
lib.nofile_mode.restype = None

lib.resolve.argtypes = [ctypes.c_char_p]
lib.resolve.restype = ctypes.c_void_p

lib.resolver_free_string.argtypes = [ctypes.c_void_p]
lib.resolver_free_string.restype = None

def call(fn, path: str):
    ptr = fn(path.encode("utf-8"))
    if not ptr:
        return None
    try:
        s = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
        return json.loads(s)
    finally:
        lib.resolver_free_string(ptr)

# 1. 路径模式
lib.nofile_mode(False)
print(call(lib.resolve, "./answer"))

# 2. NoFile 模式 (直接传入 JSON 字符串)
lib.nofile_mode(True)
content_json = '{"structure_type": "collector.read", "info": {"stid": "1", "value": "hello", "analyze": "world</br>!"}}'
info_json = "[]"
input_str = json.dumps([content_json, info_json])
print(call(lib.resolve, input_str))
```

注意: cpp引用头文件, 使用`resolver.h`, 动态链接库和静态链接库在`Releases`内

---

## 函数输入输出结构

### 1. `debug_mode(bool mode)` / `nofile_mode(bool mode)`
- **输入**：布尔值，`True` 开启模式，`False` 关闭。
- **输出**：无。需在其他函数调用前先调用此函数配置全局状态。

### 2. `resolve(str target)` -> `dict | null`
- **输入**：
  - `NOFILE_MODE = False`：包含 `content.json` 和 `info.json` 的文件夹路径。
  - `NOFILE_MODE = True`：JSON 字符串，格式为 `"[content.json_str, info.json_str]"`。
- **输出**：JSON 字符串，解析失败时返回 `"null"`。解析成功的结构为：
```json
{
  "stid1_xth1": {
    "answer": ["ans1", "ans2"],
    "important": ["keyword1", "keyword2"]
  },
  "type": "collector.xxx"
}
```

### 3. `resolve_by_homework(str target)` -> `list[dict]`
- **输入**：
  - `NOFILE_MODE = False`：包含多个作业子文件夹的目录路径。
  - `NOFILE_MODE = True`：JSON 字符串，格式为 `"[[content1, info1], [content2, info2], ...]"`。
- **输出**：JSON 字符串，解析失败时返回 `"[]"`。结构为包含多个 `resolve` 返回对象的数组。

### 4. `resolve_by_all_homework(str target)` -> `dict[str, list[dict]]`
- **输入**：
  - `NOFILE_MODE = False`：包含多个大作业文件夹的总目录路径。
  - `NOFILE_MODE = True`：JSON 字符串，格式为 `"[[[c1,i1],[c2,i2]], [[c3,i3], ...], ...]"`。
- **输出**：JSON 字符串，解析失败时返回 `"{}"`。结构为以作业名为键、对应作业解析结果（数组）为值的字典。

---

# 测试

在项目根目录下执行：
```bash
python3 test_demo.py ..\downloaded
python3 test_demo.py ..\downloaded_res
python3 test_lib.py ..\downloaded
python3 test_lib.py ..\downloaded_res
```

## 系统和环境信息

Python: 3.14.5  
System: Windows 11(25H2,26200.8037)  
CPU: AMD(R) Ryzen(TM) 3700X @ 4.3GHz  
RAM: G.SKILL Trident Z DDR4 8G*2 3600MHz (Samsung B-DIE)  
Disk: WD SN550 1TB  

## 数据

### 纯python实现

最终汇总:  
逐个调用:  14402 成功 / 0 失败  |  23348 答案  |  9.6807s  |  2411.81 answers/s  
批量调用:  14402 成功 / 0 失败  |  23348 答案  |  5.9476s  |  3925.60 answers/s  
加速比:    1.63x

### rust动态链接库

最终汇总:  
逐个调用:  14402 成功 / 0 失败  |  23348 答案  |  4.3968s  |  5310.23 answers/s  
批量调用:  14402 成功 / 0 失败  |  23348 答案  |  4.5328s  |  5150.93 answers/s  
加速比:    0.97x

# 关于与致谢

作者: leitianshuo1337  
感谢以下项目的支持:  
[Rust Programming Language](https://rust-lang.org/)  
[Python](https://python.org/)  
[ETS100](https://ets100.com/)  
本项目的产生离不开各个开源项目和开源开发者的支持, 向其他开源开发者致敬!  