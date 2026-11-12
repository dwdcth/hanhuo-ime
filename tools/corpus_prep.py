#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""corpus_prep.py — c.6 二轮语料抽取（raw 源 → 清洗纯文本行）

源（~/tmp/corpus/）:
  wiki/train-*.parquet   wikimedia/wikipedia 20231101.zh（已抽取正文列）
  thucnews/txt/THUCNews  74 万单行新闻文件（目录按类别分层）
  poetry_ds/             chinese-poetry（json 数组文件, str 字段为正文行）
产出: round2/{wiki,news,poetry}.txt 逐行纯文本（GujiCC 归一 + 空白剥离,
与 trainer clean() 同律——trainer 直接 --texts 挂载）
用法: python3 tools/corpus_prep.py [wiki|news|poetry|all]
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from train_nnlm import _load_g2s, clean  # noqa: E402

C = os.path.expanduser("~/tmp/corpus")
OUT = f"{C}/round2"
MIN_LEN, MAX_LEN = 10, 400


def write_stream(name, gen):
    os.makedirs(OUT, exist_ok=True)
    n = ln = 0
    with open(f"{OUT}/{name}.txt", "w", encoding="utf-8") as f:
        for s in gen:
            if MIN_LEN <= len(s) <= MAX_LEN:
                f.write(s + "\n")
                n += len(s)
                ln += 1
    print(f"{name}: {n / 1e6:.1f}M 字 / {ln:,} 行")


def wiki():
    import pyarrow.parquet as pq
    import glob

    def gen():
        g2s = _load_g2s()
        for p in sorted(glob.glob(f"{C}/wiki/train-*.parquet")):
            t = pq.read_table(p, columns=["text"])
            for s in t.column("text").to_pylist():
                for ln in (s or "").split("\n"):
                    ln = clean(ln, g2s)
                    if ln:
                        yield ln

    write_stream("wiki", gen())


def news():
    def gen():
        g2s = _load_g2s()
        for root, _, fs in os.walk(f"{C}/thucnews/txt"):
            for fn in fs:
                if not fn.endswith(".txt"):
                    continue
                try:
                    with open(os.path.join(root, fn), encoding="utf-8",
                              errors="ignore") as f:
                        for ln in f:
                            ln = clean(ln, g2s)
                            if ln:
                                yield ln
                except OSError:
                    continue

    write_stream("news", gen())


def poetry():
    def gen():
        g2s = _load_g2s()  # 古籍异体→规范（GujiCC 主场）
        for root, _, fs in os.walk(os.path.expanduser("~/tmp/poetry_ds")):
            for fn in fs:
                if not fn.endswith(".json"):
                    continue
                try:
                    with open(os.path.join(root, fn), encoding="utf-8") as f:
                        d = json.load(f)
                except Exception:
                    continue
                for item in d if isinstance(d, list) else []:
                    for key in ("paragraphs", "sentences"):
                        for ln in item.get(key) or []:
                            ln = clean(ln, g2s)
                            if ln:
                                yield ln

    write_stream("poetry", gen())


def classical():
    r"""古籍三源（用户指令 2026-11-11「打古籍打不出」）:
    NiuTrans/Classical-Modern 双语数据（10,636 txt, 十三经/史子覆盖）+
    JasonWade001/chtxt（精校 儒/道/法/兵, 繁体）+ wenyuange/dao（道藏）。
    繁→简 opencc + GujiCC 异体归一（c6env: pip install opencc）"""
    try:
        import opencc
        t2s = opencc.OpenCC("t2s").convert
    except Exception:
        t2s = lambda s: s  # noqa: E731

    def gen():
        g2s = _load_g2s()
        roots = [
            os.path.expanduser("~/tmp/classical_modern/双语数据"),
            os.path.expanduser("~/tmp/chtxt"),
            os.path.expanduser("~/tmp/dao_ds"),
        ]
        for root in roots:
            if not os.path.isdir(root):
                print(f"跳过缺失源: {root}")
                continue
            for r, _, fs in os.walk(root):
                for fn in fs:
                    if not fn.endswith(".txt"):
                        continue
                    try:
                        with open(os.path.join(r, fn), encoding="utf-8",
                                  errors="ignore") as f:
                            for ln in f:
                                ln = clean(t2s(ln), g2s)
                                if len(ln) >= 6:
                                    yield ln
                    except OSError:
                        continue

    write_stream("classical", gen())


if __name__ == "__main__":
    what = sys.argv[1] if len(sys.argv) > 1 else "all"
    for w, fn in (("wiki", wiki), ("news", news), ("poetry", poetry),
                  ("classical", classical)):
        if what in (w, "all"):
            fn()
