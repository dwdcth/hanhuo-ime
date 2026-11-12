#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""lmcn_build.py — c.6 P2: 开源语料重建 lmcn KN 双表（替换腾讯回收表）

目标格式（cascade 目录, lmcn.rs TSV 装载器）:
  lmcn_unigram_full.tsv:  word \t log10P \t log10α(w)   （loader 滤 1.000000）
  lmcn_bigram_final.tsv:  w1 \t w2 \t log10P(w2|w1)
平滑: 绝对折扣 KN——P(w2|w1)=max(c−D,0)/c(w1) + α(w1)·P(w2),
  α(w1)=D·N1+(w1)/c(w1)（N1+ = w1 的不同后继数）
语料: LCCC 对话（聊天域=lmcn 本域）+ round2 增料可选。
规模对齐腾讯表: uni ≤350k / bg ≤400k（频次截断）。

用法: python3 tools/lmcn_build.py [--texts a.txt b.txt] [--workers 12]
产出: ~/tmp/corpus/lmcn_open/{lmcn_unigram_full.tsv,lmcn_bigram_final.tsv}
验收: HANHUO_LMCN_DIR=<该目录> 挂 lmcn_chain 臂 vs 腾讯表 A/B
"""
import gzip
import json
import math
import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from train_nnlm import clean, _load_g2s  # noqa: E402

OUT_DIR = os.path.expanduser("~/tmp/corpus/lmcn_open")
D_DISCOUNT = 0.7
MAX_UNI, MAX_BG = 350_000, 400_000
URL_RE = re.compile(r"https?://\S+|www\.\S+")


def iter_turns(paths):
    for p in paths:
        if p.endswith(".jsonl.gz"):
            with gzip.open(p, "rt", encoding="utf-8", errors="ignore") as f:
                for ln in f:
                    try:
                        yield from json.loads(ln)
                    except Exception:
                        continue
        else:
            with open(p, encoding="utf-8", errors="ignore") as f:
                for ln in f:
                    yield ln.rstrip("\n")


def _seg_chunk(turns):
    """worker（模块级——Pool pickle 不了闭包）"""
    import jieba
    jieba.setLogLevel(60)
    jieba.initialize()
    g2s = _load_g2s()
    out = []
    for t in turns:
        t = clean(t, g2s)
        if len(t) >= 2:
            out.append([w for w in jieba.cut(t) if w.strip()])
    return out


def segment_mp(paths, workers):
    """jieba 多进程分词 → 产出生成器逐轮词组（顺序稳定）"""
    from multiprocessing import Pool

    # 分块喂池（chunk 5000 轮）
    CHUNK = 5000
    buf = []
    with Pool(workers) as pool:
        for t in iter_turns(paths):
            buf.append(t)
            if len(buf) >= CHUNK:
                for words in pool.map(_seg_chunk, [buf[i:i + 500]
                                                  for i in
                                                  range(0, len(buf), 500)]):
                    yield from words
                buf = []
        if buf:
            for words in pool.map(_seg_chunk, [buf[i:i + 500]
                                              for i in
                                              range(0, len(buf), 500)]):
                yield from words


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--lccc", default=os.path.expanduser(
        "~/tmp/corpus/lccc/lccc_base_train.jsonl.gz"))
    ap.add_argument("--texts", nargs="*", default=[])
    ap.add_argument("--workers", type=int, default=12)
    a = ap.parse_args()
    paths = ([a.lccc] if os.path.exists(a.lccc) else []) + a.texts
    if not paths:
        sys.exit("无语料源")

    print("分词中（jieba ×%d）…" % a.workers, flush=True)
    uni, bg = Counter(), Counter()
    n_turns = n_words = 0
    for words in segment_mp(paths, a.workers):
        n_turns += 1
        if words:
            uni.update(words)
            n_words += len(words)
            for i in range(len(words) - 1):
                bg[(words[i], words[i + 1])] += 1
        if n_turns % 1_000_000 == 0:
            print(f"  {n_turns/1e6:.0f}M 轮 / {n_words/1e6:.0f}M 词 "
                  f"（uni {len(uni)/1e3:.0f}k bg {len(bg)/1e3:.0f}k）",
                  flush=True)
    print(f"计频完: {n_turns/1e6:.1f}M 轮 / {n_words/1e6:.1f}M 词, "
          f"uni {len(uni):,} bg {len(bg):,}")

    # KN 绝对折扣
    top_uni = dict(uni.most_common(MAX_UNI))
    bg_kept = {p: c for p, c in bg.items() if c >= 2}
    if len(bg_kept) > MAX_BG:
        bg_kept = dict(sorted(bg_kept.items(), key=lambda x: -x[1])[:MAX_BG])
    N = sum(top_uni.values()) or 1
    logP = {w: math.log10(c / N) for w, c in top_uni.items()}
    succ = Counter()
    for (w1, _), c in bg_kept.items():
        succ[w1] += 1
    cont = {w1: c for w1, c in top_uni.items()}
    alpha = {}
    for w1, c in top_uni.items():
        a_w1 = D_DISCOUNT * succ.get(w1, 0) / c
        alpha[w1] = math.log10(a_w1) if a_w1 > 0 else -3.0  # α→0 地板

    os.makedirs(OUT_DIR, exist_ok=True)
    with open(f"{OUT_DIR}/lmcn_unigram_full.tsv", "w", encoding="utf-8") as f:
        f.write("# lmcn unigram (open rebuild): word\tlog10P\tlog10alpha\n")
        for w in top_uni:
            f.write(f"{w}\t{logP[w]:.6f}\t{alpha[w]:.6f}\n")
    with open(f"{OUT_DIR}/lmcn_bigram_final.tsv", "w", encoding="utf-8") as f:
        f.write("# lmcn bigram (open rebuild): w1\tw2\tlog10P(w2|w1)\n")
        for (w1, w2), c in bg_kept.items():
            cw1 = cont.get(w1)
            if not cw1:
                continue
            p = max(c - D_DISCOUNT, 0) / cw1 + \
                (10 ** alpha.get(w1, -3.0)) * \
                (10 ** logP.get(w2, -6.76))
            f.write(f"{w1}\t{w2}\t{math.log10(p):.6f}\n")
    print(f"✓ {OUT_DIR}: uni {len(top_uni):,} / bg {len(bg_kept):,}")


if __name__ == "__main__":
    main()
