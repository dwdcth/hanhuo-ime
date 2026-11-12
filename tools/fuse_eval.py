"""fuse_eval.py — 与一遍解码分数融合的评测：NNLM 到底在一遍系统之上**多提供了多少信息**。

纯 NNLM 降序重排（B7 口径）会把解码器已有的 n-gram/词典信息全部扔掉；线上真正用法是
    final = first_pass_score + lambda * NNLM_score
所以模型好坏应该看"融合后相对一遍的增益"。本脚本:
  1) recall@K       : gold 是否在解码器给的候选里（不在 = 任何重排都不可能对，top1 的天花板）
  2) 一遍 top1      : lambda=0（解码器自己的排序）
  3) 纯 NNLM top1   : lambda 很大
  4) 融合 top1      : lambda 用 5 折交叉验证选（避免在评测集上调参的乐观偏差）
  5) 增益 CI        : 融合 - 一遍 的成对 bootstrap；以及多模型间的 McNemar

数据: 评测 JSONL 里需要 `cand_fp`（与 `cands` 对齐的一遍分数，越大越好）。没有的话可用
      --fp-from-order，按 cands 顺序视为一遍排名，fp = -log(名次)（粗略近似，仅应急）。
模型分数: 直接复用 eval_rerank.py 的输出（需要 --dump-pairs）:
      runs/<name>.queries.jsonl + runs/<name>.pairs.jsonl

用法:
  python fuse_eval.py --data engine500.jsonl --runs runs/mobile_v2 runs/adv_v1
"""
from __future__ import annotations

import argparse
from collections import defaultdict

import numpy as np

from common import mcnemar_exact, print_table, read_jsonl

LAMBDAS = [0, 0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1, 1.5, 2, 3, 5, 10, 1e3]


def load_run(prefix: str):
    qs = {r["id"]: r for r in read_jsonl(prefix + ".queries.jsonl")}
    sc = defaultdict(dict)
    for p in read_jsonl(prefix + ".pairs.jsonl"):
        sc[p["id"]][p["cand"]] = p["margin"]          # margin = S(gold) - S(cand)（sum 口径）
    return qs, sc


def cand_scores(qs, sc, qid, gold):
    """还原每个候选的 NNLM 分数: S(cand) = S(gold) - margin。"""
    Sg = -qs[qid]["gold_nll_sum"]
    out = {gold: Sg}
    for c, m in sc[qid].items():
        out[c] = Sg - m
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data", required=True)
    ap.add_argument("--runs", nargs="+", required=True, help="eval_rerank 输出前缀，如 runs/mobile_v2")
    ap.add_argument("--fp-from-order", action="store_true")
    ap.add_argument("--folds", type=int, default=5)
    ap.add_argument("--boot", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()

    data = read_jsonl(args.data)
    runs = [(p, *load_run(p)) for p in args.runs]
    rng = np.random.default_rng(args.seed)

    # 1) 组装每个 query: 候选集 = 在所有 run 里都有分数的候选
    Q = []
    n_missing_fp = n_dropped = 0
    for row in data:
        qid, gold = row["id"], row["gold"]
        if any(qid not in qs for _, qs, _ in runs):
            continue
        cands = list(dict.fromkeys(row["cands"]))
        if "cand_fp" in row:
            fp_map = {}
            for c, s in zip(row["cands"], row["cand_fp"]):
                fp_map.setdefault(c, float(s))
        elif args.fp_from_order:
            fp_map = {c: -np.log(i + 1.0) for i, c in enumerate(cands)}
        else:
            n_missing_fp += 1
            continue
        per_run = [cand_scores(qs, sc, qid, gold) for _, qs, sc in runs]
        keep = [c for c in cands if c in fp_map and all(c in d for d in per_run)]
        n_dropped += len(cands) - len(keep)
        winnable = gold in keep
        Q.append({"id": qid, "winnable": winnable, "keep": keep,
                  "fp": np.array([fp_map[c] for c in keep]),
                  "nn": [np.array([d[c] for c in keep]) for d in per_run],
                  "gi": keep.index(gold) if winnable else -1})
    if n_missing_fp:
        raise SystemExit(f"{n_missing_fp} 条 query 缺 cand_fp；请补一遍分数或加 --fp-from-order")
    N = len(Q)
    W = np.array([q["winnable"] for q in Q])
    print(f"queries {N} | gold 在候选内 (recall) {W.mean():.1%} = {int(W.sum())} 条 | 因缺分数被剔除的候选 {n_dropped}")
    print("  top1 的天花板就是 recall；下面的 top1 同时给出『占全体』与『占可赢 query』两种分母。\n")

    L = len(LAMBDAS)
    results = []
    for r, (prefix, _, _) in enumerate(runs):
        corr = np.zeros((N, L), dtype=bool)
        for i, q in enumerate(Q):
            if not q["winnable"]:
                continue
            for k, lam in enumerate(LAMBDAS):
                corr[i, k] = int(np.argmax(q["fp"] + lam * q["nn"][r])) == q["gi"]
        wi = np.where(W)[0]
        # 5 折 CV 选 lambda
        fold = rng.permutation(len(wi)) % args.folds
        cv = np.zeros(N, dtype=bool)
        chosen = []
        for f in range(args.folds):
            tr, te = wi[fold != f], wi[fold == f]
            kbest = int(np.argmax(corr[tr].sum(0)))
            chosen.append(LAMBDAS[kbest])
            cv[te] = corr[te, kbest]
        curve = corr[wi].mean(0)
        results.append({"name": prefix, "corr": corr, "cv": cv, "curve": curve, "chosen": chosen})

    rows = []
    for res in results:
        base = res["corr"][:, 0]
        pure = res["corr"][:, -1]
        d = (res["cv"].astype(float) - base.astype(float))[W]
        idx = rng.integers(0, len(d), (args.boot, len(d)))
        lo, hi = np.percentile(d[idx].mean(1), [2.5, 97.5])
        rows.append([res["name"].split("/")[-1],
                     f"{base.sum() / N:.1%}/{base[W].mean():.1%}",
                     f"{pure.sum() / N:.1%}/{pure[W].mean():.1%}",
                     f"{res['cv'].sum() / N:.1%}/{res['cv'][W].mean():.1%}",
                     f"{100 * d.mean():+.2f}pp [{100 * lo:+.2f},{100 * hi:+.2f}]",
                     str(res["chosen"])])
    print_table(["model", "一遍top1(全体/可赢)", "纯NNLM top1", "融合top1(CV)", "融合-一遍 增益 (可赢 query)", "各折选中λ"], rows)

    print("\nlambda 曲线（可赢 query 上的 top1，乐观：在评测集上直接看）:")
    print_table(["lambda"] + [r["name"].split("/")[-1] for r in results],
                [[f"{lam:g}"] + [f"{r['curve'][k]:.1%}" for r in results] for k, lam in enumerate(LAMBDAS)])

    if len(results) >= 2:
        print("\n融合(CV)后的成对 McNemar（相对第 1 个模型；只看可赢 query）:")
        a = results[0]["cv"][W]
        out = []
        for res in results[1:]:
            b = res["cv"][W]
            n10, n01 = int((a & ~b).sum()), int((~a & b).sum())
            out.append([res["name"].split("/")[-1], f"{n10}:{n01}", f"{mcnemar_exact(n10, n01):.4f}"])
        print_table(["vs 第1个", "第1对第2错 : 第1错第2对", "p"], out)
    print("\n判读: 若 adv 融合增益与 mobile 无差异 → 额外容量对线上排序没有互补信息；"
          "若 adv 增益显著更大 → 纯 NNLM 口径低估了它，应以融合口径做裁定。")


if __name__ == "__main__":
    main()
