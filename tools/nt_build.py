#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""nt_build.py — noun_tag 开源重建（jieba 词层 → k2 u32 表）

引擎 ntgen 深带叉积按 4 字节滑窗 k2 低 32 位成员检测——开源表 =
jieba dict.txt（MIT, 34.9 万词）逐词 k2 排序去重 u32 LE 流。
HANHUO_NT=<out> 挂载（缺省路径=腾讯词性资产）。
用法: python3 tools/nt_build.py [--out /mnt/s790/tmp/noun_tag_open.bin]
"""
import os
import struct
import sys

sys.path.insert(0, "/mnt/s790/project/vtype-linux-archive/win/tools/wxassets")
from wxhash import k2, k2_long  # noqa: E402  （算法复现; Rust 侧 wxhash.rs 同源）
import jieba  # noqa: E402


def main():
    out = sys.argv[sys.argv.index("--out") + 1] \
        if "--out" in sys.argv else "/mnt/s790/tmp/noun_tag_open.bin"
    d = os.path.join(os.path.dirname(jieba.__file__), "dict.txt")
    words = [ln.split()[0] for ln in open(d, encoding="utf-8") if ln.split()]
    ks = set()
    for w in words:
        b = w.encode()
        try:
            ks.add((k2(b) if len(b) <= 32 else k2_long(b)) & 0xFFFFFFFF)
        except NotImplementedError:
            pass
    with open(out, "wb") as f:
        for k in sorted(ks):
            f.write(struct.pack("<I", k))
    print(f"✓ {out}: {len(ks):,} k2 u32（jieba {len(words):,} 词）")


if __name__ == "__main__":
    main()
