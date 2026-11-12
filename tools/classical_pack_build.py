#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""classical_pack_build.py — 古籍词包（第 27 分类, 用户裁定「低频用字典」）

classical.txt（NiuTrans/chtxt/dao 三源, t2s+GujiCC 归一, 89M 字）
→ 标点断句分段（4..12 字语段=古语词单位）→ 计频 ≥min-count
→ pypinyin 注音 → dict yaml → HBWC v2 + eval 探针 + catalog。

设计对齐 poetry 包（逗号相邻对同律）: 古文无现代分词, 语段即词——
含低频古字（龏/瞷 类, NN 词表线外）的语段经词典层上屏 + τ 直取,
不占 NN 词表（每字位保训练信号的取舍, 2026-11-11 用户裁定）。

用法: python3 tools/classical_pack_build.py [--min-count 3] [--max-words 200000]
"""
import json
import os
import re
import subprocess
import sys
from collections import Counter

SRC = os.path.expanduser("~/tmp/corpus/round2/classical.txt")
OUT = "data/domain"
NAME = "classical"
SEG_RE = re.compile(r"[，。；：、？！\s]+")

def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--min-count", type=int, default=3)
    ap.add_argument("--max-words", type=int, default=200_000)
    ap.add_argument("--min-len", type=int, default=4)
    ap.add_argument("--max-len", type=int, default=12)
    a = ap.parse_args()

    from pypinyin import pinyin, Style

    cnt = Counter()
    for ln in open(SRC, encoding="utf-8"):
        for seg in SEG_RE.split(ln):
            if a.min_len <= len(seg) <= a.max_len:
                cnt[seg] += 1
    cand = [(w, c) for w, c in cnt.most_common()
            if c >= a.min_count][:a.max_words]
    print(f"语段候选 {len(cnt):,} 种, ≥{a.min_count} 次 {len(cand):,} 条")

    rows = []
    for w, c in cand:
        py = pinyin(w, style=Style.NORMAL, errors="ignore")
        syls = [p[0] for p in py]
        if len(syls) != len(w):
            continue  # 含不可注音字符（□ 残缺符类）
        if not all(s.isascii() and s.isalpha() for s in syls):
            continue
        rows.append((w, " ".join(syls), c))
    print(f"注音可行 {len(rows):,}")

    os.makedirs(OUT, exist_ok=True)
    yml = f"{OUT}/{NAME}.dict.yaml"
    with open(yml, "w", encoding="utf-8") as f:
        f.write("# 古籍语段（NiuTrans/chtxt/wenyuange 三源, t2s+GujiCC 归一,"
                " tools/classical_pack_build.py）\n---\n")
        for w, p, c in rows:
            f.write(f"{w}\t{p}\t{c}\n")
    r = subprocess.run(
        ["python3", "tools/dict_build.py", yml, "--out", f"{OUT}/{NAME}.hbwc"],
        capture_output=True, text=True)
    sys.stdout.write(r.stdout[-160:] if r.returncode == 0 else r.stderr)
    if r.returncode != 0:
        sys.exit(1)
    with open(f"eval/eval_{NAME}.jsonl", "w", encoding="utf-8") as f:
        for w, p, c in rows:
            f.write(json.dumps({"kind": "word", "stratum": NAME,
                                "text": w, "py": p.replace(" ", ""),
                                "freq": c}, ensure_ascii=False) + "\n")
    print(f"古籍({NAME}): {len(rows):,} 语段")


if __name__ == "__main__":
    main()
