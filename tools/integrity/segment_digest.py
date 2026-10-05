#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""计算 ELF 可执行段（PT_LOAD with PF_X）的 sha256 —— 与运行时自校验同一算法。

运行时实现（src/integrity.rs）在进程内定位自身映射，按 ELF program header 找到
PF_X 段，对 [p_offset, p_offset+p_filesz) 这段**文件字节**求 sha256。
本工具对文件做同样的事，用于：
  1) CI 在构建 Android 产物后算出摘要，作为 RESOLVER_TEXT_DIGEST 注入二次构建；
  2) 本地校验"运行时算出来的"与"静态算出来的"是否一致（算法对齐，避免误拦）。

用法: python3 tools/integrity/segment_digest.py <elf文件> [--json]
"""
import hashlib
import json
import struct
import sys

PT_LOAD = 1
PF_X = 0x1


PAGE = 0x1000


def executable_mapping_range(path: str) -> tuple[int, int]:
    """返回可执行映射对应的**文件区间** (offset, length)。

    运行时的做法（src/integrity.rs::code_mapping）是取包含自身代码的那条 r-xp 映射，
    对内存 [start, end) 求 sha256。加载器把 PF_X 段按页对齐映射，所以文件侧同一区间是：
        offset = floor(p_offset / PAGE) * PAGE
        length = align_up((p_offset - offset) + p_filesz, PAGE)
    两者逐字节相同（该段只读、file-backed，无运行期改写）。
    """
    with open(path, "rb") as fh:
        data = fh.read()
    if data[:4] != b"\x7fELF":
        raise SystemExit(f"不是 ELF 文件: {path}")
    is64 = data[4] == 2
    little = data[5] == 1
    endian = "<" if little else ">"
    if is64:
        e_phoff = struct.unpack_from(endian + "Q", data, 0x20)[0]
        e_phentsize = struct.unpack_from(endian + "H", data, 0x36)[0]
        e_phnum = struct.unpack_from(endian + "H", data, 0x38)[0]
    else:
        e_phoff = struct.unpack_from(endian + "I", data, 0x1C)[0]
        e_phentsize = struct.unpack_from(endian + "H", data, 0x2A)[0]
        e_phnum = struct.unpack_from(endian + "H", data, 0x2C)[0]
    best = None
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack_from(endian + "I", data, off)[0]
        if p_type != PT_LOAD:
            continue
        if is64:
            p_flags = struct.unpack_from(endian + "I", data, off + 4)[0]
            p_offset = struct.unpack_from(endian + "Q", data, off + 8)[0]
            p_filesz = struct.unpack_from(endian + "Q", data, off + 32)[0]
        else:
            p_offset = struct.unpack_from(endian + "I", data, off + 4)[0]
            p_filesz = struct.unpack_from(endian + "I", data, off + 16)[0]
            p_flags = struct.unpack_from(endian + "I", data, off + 24)[0]
        if p_flags & PF_X:
            aligned = (p_offset // PAGE) * PAGE
            length = ((p_offset - aligned) + p_filesz + PAGE - 1) // PAGE * PAGE
            if best is None or length > best[1]:
                best = (aligned, length)
    if best is None:
        raise SystemExit(f"未找到可执行段: {path}")
    return best


def digest(path: str) -> tuple[str, int, int, int]:
    offset, size = executable_mapping_range(path)
    with open(path, "rb") as fh:
        fh.seek(offset)
        blob = fh.read(size)
    return hashlib.sha256(blob).hexdigest(), offset, size, len(blob)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    target = sys.argv[1]
    h, off, size, _ = digest(target)
    if "--json" in sys.argv:
        print(json.dumps({"file": target, "sha256": h, "offset": off, "size": size}))
    else:
        print(f"{target}\n  可执行映射(文件区间): offset={off} size={size}\n  sha256: {h}")
