#!/usr/bin/env python3
"""eval_parallel.py — hanhuo eval 分片并行驱动（backlog 性能线）

领域 eval 155k 探针单进程 ~63min（引擎 RefCell 非 Sync, 进程内无法
rayon）→ 按 IO 分片 N 进程并行（每进程独立 Engine::open, round-robin
分片保各桶均衡）, --json 输出按 n×率 可逆合并。

用法:
  python3 tools/eval_parallel.py eval/eval_domain.jsonl [--workers 12] \
      [-- hanhuo eval 附加旗标（--typo-retry/--no-fuzz/--nnlm F 等）]
"""
import argparse
import json
import math
import os
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HANHUO = os.path.join(ROOT, "target/release/hanhuo")
METRICS = ["top1", "top3", "top5", "top10", "mrr"]


def run_shard(args):
    idx, shard_path, extra = args
    cmd = [HANHUO, "eval", shard_path, "--json"] + extra
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
    os.unlink(shard_path)
    if r.returncode != 0:
        print(f"[shard {idx}] 失败: {r.stderr[-300:]}", file=sys.stderr)
        return None
    for line in r.stdout.splitlines():
        if line.startswith("{"):
            return json.loads(line)
    return None


def merge(results):
    buckets = {}
    total_n = 0
    for res in results:
        if not res:
            continue
        total_n += res.get("n", 0)
        for name, v in res.items():
            if not isinstance(v, dict) or "n" not in v:
                continue
            b = buckets.setdefault(name, {"n": 0, **{m: 0.0 for m in METRICS}})
            b["n"] += v["n"]
            for m in METRICS:
                b[m] += v[m] * v["n"]  # 率×n 还原计数
    return total_n, buckets


def main():
    global HANHUO
    ap = argparse.ArgumentParser()
    ap.add_argument("eval_file")
    ap.add_argument("--workers", type=int,
                    default=min(16, os.cpu_count() or 8))
    ap.add_argument("--bin", default=HANHUO)
    argv, extra = (lambda a: (a, []))(
        sys.argv[1:]) if "--" not in sys.argv else (
        sys.argv[1:sys.argv.index("--")], sys.argv[sys.argv.index("--") + 1:])
    args = ap.parse_args(argv)
    HANHUO = args.bin
    lines = [l for l in open(args.eval_file) if l.strip()]
    w = args.workers
    print(f"# {args.eval_file}: {len(lines)} 条 × {w} 进程"
          f"（额外旗标: {extra or '无'}）")
    shards = [[] for _ in range(w)]
    for i, l in enumerate(lines):
        shards[i % w].append(l)  # round-robin: 各桶按比例均衡
    tmps = []
    for i, shard in enumerate(shards):
        f = tempfile.NamedTemporaryFile(
            "w", suffix=f".{i}.jsonl", delete=False, dir="/tmp")
        f.writelines(shard)
        f.close()
        tmps.append((i, f.name))
    t0 = time.time()
    with ThreadPoolExecutor(max_workers=w) as ex:
        results = list(
            ex.map(run_shard, [(i, p, extra) for i, p in tmps]))
    dt = time.time() - t0
    ok = [r for r in results if r]
    total_n, buckets = merge(ok)
    print(f"# 完成: {total_n} 条 / {dt:.0f}s"
          f"（单进程估算 {dt * w / 60:.0f}min → 并行 {dt / 60:.1f}min,"
          f" 加速 ~{w}×[IO 均摊]）")
    print(f"| {'桶':<16} | {'n':>7} | {'top1':>6} | {'top3':>6} |"
          f" {'top5':>6} | {'top10':>6} | {'MRR':>6} |")
    print("|" + "---|" * 7)
    for name in sorted(buckets):
        b = buckets[name]
        n = b["n"]
        if n == 0:
            continue
        cells = " | ".join(f"{b[m] / n * 100:5.1f}%" for m in METRICS[:4])
        print(f"| {name:<16} | {n:>7} | {cells} |"
              f" {b['mrr'] / n:6.3f} |")
    if len(ok) < w:
        print(f"[警告] {w - len(ok)} 分片失败", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
