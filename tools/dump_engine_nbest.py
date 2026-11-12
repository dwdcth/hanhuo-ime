#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""dump_engine_nbest.py — 引擎真 n-best 数据集导出（fuse_eval 前置）

对电池每句跑 hanhuo query --json --topk 30, 取候选词与 score
（=一遍分数 cand_fp, 融合口径评测用）。候选保持引擎原序。
用法: python3 tools/dump_engine_nbest.py [--jsonl eval/eval_sentence_large.jsonl]
      [--out eval/engine_nbest.jsonl] [--jobs 11]
"""
import argparse
import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HANHUO = ROOT / "target" / "release" / "hanhuo"


def one(args):
    i, py = args
    try:
        import shlex
        extra = shlex.split(os.environ.get("DUMP_EXTRA_FLAGS", ""))
        out = subprocess.run(
            [str(HANHUO), "query", py, "--bin", "open/hanhuo_open.bin",
             "--json", "--topk", os.environ.get("DUMP_TOPK", "30")]
            + extra,
            capture_output=True, text=True, timeout=60,
            cwd=str(ROOT))
        d = out.stdout
        if '{' in d:  # 剥离 [nnlm]/[dict] 等 stdout 前缀行
            d = d[d.index('{'):]
        d = json.loads(d)
        # 只留 Sentence 同类——引擎 B7 只重排同类候选, 混长候选在
        # ΣlnP 求和下短者恒胜（单字 −3 vs 30 字句 −100）, 会假性全 0
        cands = [(c["word"], c["score"]) for c in d.get("cands", [])
                 if c.get("word") and c.get("kind") == "sentence"]
        return i, cands, None
    except Exception as e:  # noqa: BLE001
        return i, [], str(e)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jsonl", default="eval/eval_sentence_large.jsonl")
    ap.add_argument("--out", default="eval/engine_nbest.jsonl")
    ap.add_argument("--jobs", type=int, default=11)
    a = ap.parse_args()
    rows = [json.loads(l) for l in open(a.jsonl, encoding="utf-8")
            if l.strip()]
    with ThreadPoolExecutor(a.jobs) as ex:
        res = dict((i, c) for i, c, _ in
                   ex.map(one, [(i, r["py"]) for i, r in enumerate(rows)]))
    n_gold_in = 0
    with open(a.out, "w", encoding="utf-8") as f:
        for i, r in enumerate(rows):
            cands = res.get(i, [])
            words = [w for w, _ in cands]
            fps = [s for _, s in cands]
            if r["text"] in words:
                n_gold_in += 1
            f.write(json.dumps({
                "id": f"e{i:05d}", "gold": r["text"], "cands": words,
                "cand_fp": fps, "cand_src": ["engine"] * len(words),
                "ctx": "", "domain": r.get("stratum", "")},
                ensure_ascii=False) + "\n")
    print(f"✓ {a.out}: {len(rows)} query, gold 在候选内 {n_gold_in} "
          f"({n_gold_in / len(rows) * 100:.1f}%)")


if __name__ == "__main__":
    main()
