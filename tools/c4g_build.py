#!/usr/bin/env python3
"""c4g_build.py — c4g 词对预测表开源重建（终审矩阵 13.5 缺口②）

微信 c4g.bin（S 级 99.995%）: 英文上下文 → 下一词预测分布（联想精准版,
引擎 q_c4g 消费, Kind::C4G）。开源等价: wikitext-103 相邻词对统计
（c4g 语义 = 英文词对, 中文联想已由 bgrm/q_assoc 覆盖）。

产出: data/c4g_open.tsv（w1 \t w2 \t log10P）——open_build 集成为
C4G section（词表+对+量化 val）。
用法: python3 tools/c4g_build.py [--min-cnt 3] [--per-w1 6] [--vocab 20000]
"""
import argparse
import glob
import math
import os
from collections import Counter

import pyarrow.parquet as pq

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WIKI = os.path.expanduser("~/tmp/corpus/wiki")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--min-cnt", type=int, default=3)
    ap.add_argument("--per-w1", type=int, default=6)
    ap.add_argument("--vocab", type=int, default=20000)
    ap.add_argument("--out", default="data/c4g_open.tsv")
    a = ap.parse_args()

    uni = Counter()
    pairs = Counter()
    files = sorted(glob.glob(os.path.join(WIKI, "*.parquet")))
    assert files, f"无 parquet: {WIKI}"
    for f in files:
        for batch in pq.ParquetFile(f).iter_batches(columns=["text"]):
            for rec in batch.column("text"):
                t = rec.as_py() or ""
                for line in t.split("\n"):
                    ws = [
                        w for w in line.lower().split()
                        if w.isalpha()
                    ]
                    uni.update(ws)
                    for i in range(len(ws) - 1):
                        pairs[(ws[i], ws[i + 1])] += 1
    vocab = {w for w, c in uni.most_common(a.vocab)}
    # 每 w1 top-K w2（词表域内, 计数门）
    by_w1 = {}
    for (w1, w2), c in pairs.items():
        if c >= a.min_cnt and w1 in vocab and w2 in vocab:
            by_w1.setdefault(w1, []).append((w2, c))
    n = 0
    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    with open(a.out, "w", encoding="utf-8") as f:
        f.write("# c4g_open — wikitext-103 英文词对（c4g.bin 开源重建, Apache 归语料源）\tw2\tlog10P\n")
        for w1 in sorted(by_w1):
            c1 = uni[w1]
            top = sorted(by_w1[w1], key=lambda x: -x[1])[: a.per_w1]
            for w2, c12 in top:
                f.write(f"{w1}\t{w2}\t{math.log10(c12 / c1):.4f}\n")
                n += 1
    print(f"[c4g] 词表 {len(vocab)} / w1 {len(by_w1)} / 对 {n} → {a.out}")


if __name__ == "__main__":
    main()
