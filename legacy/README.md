# legacy/ —— 历史实现归档（下游不使用）

这里的文件是 **Python demo 时代的旧实现**，仅作历史参考与行为比对用。
**Fe-pro、Fe_web、lastudio-platform 后端都不依赖这里的任何文件**，CI 也不构建它们。

## 为什么归档

仓库曾经同时存在三份解析实现，导致"明明做过某个处理却找不到"：

| 实现 | 位置 | 状态 |
|---|---|---|
| Rust 核心（唯一事实源） | `src/*.rs` + `cloud/src/*.rs` | ✅ 下游在用 |
| 纯 Python 解析器 | `legacy/resolver_py/` | 🟡 仅 `legacy/test_resolver_py.py` 用；后端的 Python 兜底实现维护在后端仓库内 |
| Python demo 脚本 | `legacy/demo.py`、`legacy/unified_demo.py` | 💀 仅历史参考 |

`src/*.rs` 的注释里写着"与 `resolver_py/*.py` 行为一致"，指的是这里的 `legacy/resolver_py/`。

## 文件清单

| 文件 | 说明 | 是否还能跑 |
|---|---|---|
| `resolver_py/` | 纯 Python 通用解析器（`api` / `collectors` / `cleaner`） | 可跑，见下 |
| `test_resolver_py.py` | `resolver_py` 的测试，需要 `/tmp/opencode/dataset` 数据集，缺失时自动跳过 | 可跑（无数据集则跳过） |
| `demo.py` | 最早的 CLI 解析 demo（1132 行） | 可跑，需本地作业目录 |
| `unified_demo.py` | demo 的统一版（784 行，2026-07-15 后未再改动） | 可跑，需本地作业目录 |
| `test_demo.py` / `test_lib.py` | demo / 旧 FFI 的性能测试（需要 `..\downloaded` 数据） | 需数据 |
| `test_new_demo.py` | demo 的早期测试 | 需数据 |
| `build.ps1` | 旧 Windows 构建脚本（CI 已改为 `build-release.yml`） | — |
| `todo` | 2026-07 时代的待办备忘 | — |

## 使用方式

```bash
# 纯 Python 解析器测试（无数据集时自动跳过真实用例）
python3 legacy/test_resolver_py.py

# 旧 demo（需要本地作业目录）
python3 legacy/demo.py
```

依赖关系说明：`test_demo.py` 用 `SCRIPT_DIR` 导入同目录的 `demo.py`；`test_resolver_py.py`
把脚本自身所在目录加进 `sys.path` 后导入同目录的 `resolver_py/`，因此**必须在 `legacy/` 内运行**，
不要把这些文件单独拷出去。

## 如果要继续瘦身

`demo.py`、`unified_demo.py`、`test_new_demo.py`、`build.ps1`、`todo` 已经没有任何引用，
确认不再需要行为比对后可直接删除；`resolver_py/` 建议保留到后端 Python 兜底实现完全退场为止。
