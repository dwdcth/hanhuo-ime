"""diff_cases.py — 逐条读"A 对 B 错"的 query，并看两个模型在同一候选对上的边际分解差。

当 McNemar 是 11:0 这种一边倒的结构（B 的正确集合几乎是 A 的子集），读这 11 条
比任何聚合指标都快: 你会直接看到 B 把哪类候选排在了 gold 前面、为什么。

输入: eval_rerank.py 的输出（需要 --dump-pairs），以及可选的评测集(用来打印 gold 原文)。

  python diff_cases.py --a runs/mobile_v2 --b runs/adv_v1 --data engine500.jsonl --src engine

输出:
  1) 每条 case: gold / B 排第一的候选 / 该候选的来源 / 首个分歧位 / B 与 A 在 local-near-far 上的差
  2) 聚合: 这些 case 上 (B - A) 的 local/near/far 均值；以及全部可对齐候选对上的同口径均值
     —— (B-A) 的 far 明显为负，说明 B 对"错字之后的共享字"惩罚更少（对错字宽容）
"""
from __future__ import annotations

import argparse
from collections import defaultdict

import numpy as np

from common import print_table, read_jsonl


def load(prefix):
    qs = {r["id"]: r for r in read_jsonl(prefix + ".queries.jsonl")}
    pr = defaultdict(dict)
    for p in read_jsonl(prefix + ".pairs.jsonl"):
        pr[p["id"]][p["cand"]] = p
    return qs, pr


def delta_rows(pa, pb, ids, src):
    """对齐的 (id,cand) 上 B-A 的分解差。"""
    out = []
    for i in ids:
        for c, b in pb.get(i, {}).items():
            a = pa.get(i, {}).get(c)
            if a is None or not (a["aligned"] and b["aligned"]):
                continue
            if src and b["src"] != src:
                continue
            out.append({"d_local": b["local"] - a["local"], "d_near": b["near"] - a["near"],
                        "d_far": b["far"] - a["far"], "d_margin": b["margin"] - a["margin"],
                        "win_a": a["win"], "win_b": b["win"]})
    return out


def agg(rows, title):
    if not rows:
        print(f"  [{title}] 无可对齐的候选对")
        return
    f = lambda k: np.mean([r[k] for r in rows])  # noqa: E731
    print(f"  [{title}] n={len(rows)}  pair_acc A={f('win_a'):.3f} B={f('win_b'):.3f}   "
          f"(B-A) margin {f('d_margin'):+.2f} = local {f('d_local'):+.2f} + near {f('d_near'):+.2f} + far {f('d_far'):+.2f}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--a", required=True, help="eval_rerank 输出前缀（'好'的一方，如 runs/mobile_v2）")
    ap.add_argument("--b", required=True, help="另一方（如 runs/adv_v1）")
    ap.add_argument("--data", default=None)
    ap.add_argument("--src", default=None, help="只看某类候选来源，如 engine")
    ap.add_argument("--top", type=int, default=30)
    args = ap.parse_args()

    qa, pa = load(args.a)
    qb, pb = load(args.b)
    gold = {r["id"]: r["gold"] for r in read_jsonl(args.data)} if args.data else {}
    common = [i for i in qa if i in qb]
    cases = [i for i in common if qa[i]["top1"] and not qb[i]["top1"]]
    rev = [i for i in common if qb[i]["top1"] and not qa[i]["top1"]]
    print(f"A 对 B 错 {len(cases)} 条；A 错 B 对 {len(rev)} 条（共同 {len(common)}）\n")

    shown = []
    for i in cases:
        cands = pb.get(i, {})
        if args.src:
            cands = {c: p for c, p in cands.items() if p["src"] == args.src} or cands
        if not cands:
            continue
        c, b = min(cands.items(), key=lambda kv: kv[1]["margin"])   # B 的第一名：margin 最小（最该排在 gold 前面）
        a = pa.get(i, {}).get(c)
        shown.append((b["margin"], i, c, a, b))
    shown.sort(key=lambda x: x[0])
    for _, i, c, a, b in shown[: args.top]:
        print(f"[{i}] rank_A={qa[i]['rank']} rank_B={qb[i]['rank']}  src={b['src']}")
        if gold:
            print(f"   gold : {gold.get(i, '?')}")
        print(f"   B第一: {c}")
        if b["aligned"] and a and a["aligned"]:
            print(f"   首个分歧位={b['p1']} 分歧字数={b['ndiff']} | margin A={a['margin']:+.2f} B={b['margin']:+.2f} | "
                  f"(B-A) local {b['local'] - a['local']:+.2f} near {b['near'] - a['near']:+.2f} far {b['far'] - a['far']:+.2f}")
        else:
            print(f"   (候选与 gold 不等长，不可分解) margin B={b['margin']:+.2f}")
        print()

    print("聚合:")
    case_rows = []
    for _, i, c, a, b in shown:
        if a and a["aligned"] and b["aligned"]:
            case_rows.append({"d_local": b["local"] - a["local"], "d_near": b["near"] - a["near"],
                              "d_far": b["far"] - a["far"], "d_margin": b["margin"] - a["margin"],
                              "win_a": a["win"], "win_b": b["win"]})
    agg(case_rows, "这些 case 里 B 的第一名候选")
    agg(delta_rows(pa, pb, common, args.src), "全体可对齐候选对" + (f"(src={args.src})" if args.src else ""))
    print("\n怎么读: (B-A) 的 margin 为负 = B 比 A 更偏向候选。看负值主要落在 local(分歧字本身) 还是 far(分歧之后)。")


if __name__ == "__main__":
    main()
