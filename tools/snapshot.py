#!/usr/bin/env python3
"""snapshot.py — open bin 行为快照生成器（开源仓自洽回归锁）。

逐案例跑 `hanhuo query --json` 落盘 golden/open_golden.jsonl 同构格式;
`hanhuo golden <snapshot>` 对拍 = 当前引擎 vs 冻结行为的回归锁
（不锁跨引擎 parity——那是私有研究线的职责）。
用法: WXQ_BIN=<bin> python3 tools/snapshot.py [--ltr] [out.jsonl]
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
H = os.path.join(ROOT, "target/release/hanhuo")
BIN = os.environ.get("WXQ_BIN", os.path.join(ROOT, "open/hanhuo_open.bin"))


def main():
    src = os.environ.get("SNAP_SRC", os.path.join(ROOT, "golden", "open_golden.jsonl"))
    out = sys.argv[-1] if sys.argv[-1].endswith(".jsonl") else \
        os.path.join(ROOT, "golden", "open_golden.jsonl")
    cases = [json.loads(l) for l in open(src, encoding="utf-8")]
    ltr = "--ltr" in sys.argv
    with open(out, "w", encoding="utf-8") as f:
        for c in cases:
            # 镜像 cmd_golden 口径: e.query(inp, ctx, jianpin, topk, nn, ltr, dp)
            cmd = [H, "query", c["inp"], "--bin", BIN,
                   "--topk", str(c.get("topk", 20)), "--json"]
            if c.get("ctx"):
                cmd += ["--ctx", c["ctx"]]
            if c.get("jianpin"):
                cmd += ["-p"]
            if c.get("nn"):
                cmd += ["--nn"]
            if c.get("ltr"):
                cmd += ["--ltr"]
            if c.get("dp"):
                cmd += ["--dp"]
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
            d = json.loads(r.stdout)
            cands = [{
                "word": x["word"], "kind": x["kind"], "reading": x["reading"],
                "logp": x["logp"], "score": x["score"], "lm": x.get("lm"),
            } for x in d.get("cands", [])]
            rec = {k: c[k] for k in ("inp", "ctx", "jianpin", "nn", "ltr", "dp", "topk") if k in c}
            rec["cands"] = cands
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"快照 {len(cases)} 案 → {out}")


if __name__ == "__main__":
    main()
