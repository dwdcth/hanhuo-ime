#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""epyn_build.py — c.6 审计后续: 拼音纠错对开源重建（替换 editpinyin 导出）

腾讯表模式解析（42,976 行）: 错拼=合法音节经 五型单点扰动——
前缀字母/位内插入/替换/删除/邻接换位; corrections 多值聚合
（同一错拼可源于多个音节, 12,691 行多值）。开源重建同律:
开源音节表（27 分类词库拼音并集, 全开源）× 扰动枚举 → 仅保留
非法结果（合法音节无需纠错）→ 聚合去重。

用法: python3 tools/epyn_build.py [--out data/editpinyin_open.tsv]
验收: WXQ_EPYN_TSV=<out> typo 臂 A/B（vs 腾讯表/无表）
"""
import glob
import os
import re
import sys
from collections import defaultdict

OUT = "data/editpinyin_open.tsv"
DOM = "data/domain/*.dict.yaml"


def load_syllables():
    """27 分类词库拼音列并集（开源; 贴产品音节分布）"""
    syls = set()
    for y in glob.glob(DOM):
        for ln in open(y, encoding="utf-8"):
            if ln.startswith(("#", "---", "...")) or "\t" not in ln:
                continue
            py = ln.split("\t")[1].strip()
            for s in py.split():
                syls.add(s)
    # 单字母音节（a/ai/e 等短音节扰动空间小, 仍保留）
    return {s for s in syls if s and s.isascii() and s.isalpha()}


def variants(s):
    """五型单点扰动 → (typo, 源音节)"""
    n = len(s)
    az = "abcdefghijklmnopqrstuvwxyz"
    out = []
    for c in az:                          # ① 前缀字母
        out.append(c + s)
    for i in range(1, n):                 # ② 位内插入
        for c in az:
            out.append(s[:i] + c + s[i:])
    for i in range(n):                    # ③ 替换
        for c in az:
            if c != s[i]:
                out.append(s[:i] + c + s[i + 1:])
    for i in range(n):                    # ④ 删除（保 ≥2 长度）
        if n >= 2:
            out.append(s[:i] + s[i + 1:])
    for i in range(n - 1):                # ⑤ 邻接换位
        if s[i] != s[i + 1]:
            out.append(s[:i] + s[i + 1] + s[i] + s[i + 2:])
    return out


def main():
    syls = load_syllables()
    print(f"开源音节表 {len(syls):,}（27 分类词库拼音并集）")
    table = defaultdict(set)
    for s in syls:
        for typo in variants(s):
            if typo not in syls:  # 合法音节不进纠错表
                table[typo].add(s)
    rows = sorted(table.items(), key=lambda x: x[0])
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("typo\tcorrections\tprefixes\n")
        for typo, corrs in rows:
            f.write(f"{typo}\t{' '.join(sorted(corrs))}\t0\n")
    print(f"✓ {OUT}: {len(rows):,} 错拼键（多值 "
          f"{sum(1 for _, c in rows if len(c) > 1):,}）")


if __name__ == "__main__":
    main()
