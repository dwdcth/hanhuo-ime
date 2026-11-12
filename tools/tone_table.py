#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tone_table.py — pypinyin → 字×音节→调号 表生成器（声调筛选 §2 数据层）

输出 data/tone_solved.tsv: 字\\t音节:调 音节:调…（同字多音节各行消歧;
同字同音节多调取首调——pypinyin 频序即最常用形）。pypinyin 为 MIT
开放数据, 生成表可入库分发。
用法: python3 tools/tone_table.py [--out data/tone_solved.tsv]
"""
import os, sys
from pypinyin import pinyin, Style

OUT = "data/tone_solved.tsv"
if "--out" in sys.argv:
    OUT = sys.argv[sys.argv.index("--out") + 1]


def main():
    from pypinyin.core import PINYIN_DICT
    from pypinyin import pinyin as _py, Style
    rows = []
    n_amb = 0
    for cp in sorted(PINYIN_DICT):
        ch = chr(cp)
        m = {}
        # heteronym 全读数 × TONE3 数字调形（首读数=pypinyin 频序）
        for rd in _py(ch, style=Style.TONE3, heteronym=True,
                      errors=lambda x: []):
            for s in rd:
                if not s or not s.isascii() or not s.islower():
                    continue
                # TONE3 轻声渲染为无数字尾形（的→"de"）→ 调 5;
                # 带尾数字形去尾取调（0 亦轻声律归 5）
                tone = 5
                syl = s
                if s[-1] in "012345":
                    tone = 5 if s[-1] in "05" else int(s[-1])
                    syl = s[:-1]
                if syl and syl.isalpha():
                    m.setdefault(syl, tone)  # 首调律（pypinyin 频序）
        if len(m) > 1:
            n_amb += 1
        if m:
            rows.append(ch + "\t" + " ".join(
                f"{k}{v}" for k, v in sorted(m.items())))
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("# tone_solved v1: char\tsyl+tone…（pypinyin MIT 生成; 5=轻声）\n")
        for r in rows:
            f.write(r + "\n")
    print(f"{OUT}: {len(rows)} 字（多音节消歧 {n_amb}）")


if __name__ == "__main__":
    main()
