#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""mixed_eval.py — 混合输入（前缀全拼+尾声母）探针集评测

eval/eval_mixed.jsonl（kind=sent, py_mixed=混合键, text=对应句前缀）→
基线 vs --boost-mixed 两臂: top1 精确 / top3·top10 含。

用法: python3 tools/mixed_eval.py [--dict data/domain/lyric.hbwc]
"""
import json, os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HANHUO = os.path.join(ROOT, "target/release/hanhuo")
BIN = os.environ.get("HANHUO_BIN",
                     os.path.expanduser("~/tmp/hanhuo/wxq.bin"))


def run_arm(rows, extra):
    hit1 = hit3 = hit10 = 0
    for r in rows:
        cmd = [HANHUO, "query", r["py_mixed"], "--bin", BIN,
               "--topk", "10", "--json"] + extra
        p = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
        cands = []
        try:
            d = json.loads(p.stdout)
            cands = d if isinstance(d, list) else d.get("cands", [])
        except json.JSONDecodeError:
            pass
        words = [c["word"] for c in cands]
        hit1 += words[:1] and words[0] == r["text"]
        hit3 += any(w == r["text"] for w in words[:3])
        hit10 += any(w == r["text"] for w in words[:10])
    n = len(rows)
    return hit1 / n * 100, hit3 / n * 100, hit10 / n * 100


def main():
    extra = [a for a in sys.argv[1:] if a.startswith("--")]
    rows = [json.loads(l) for l in open("eval/eval_mixed.jsonl") if l.strip()]
    for name, arm in (("基线", []), ("--boost-mixed", ["--boost-mixed"])):
        t1, t3, t10 = run_arm(rows, arm + extra)
        print(f"{name:16} top1={t1:5.1f} top3={t3:5.1f} top10={t10:5.1f}"
              f"  (n={len(rows)}{'; ' + ' '.join(extra) if extra else ''})")


if __name__ == "__main__":
    main()
