#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""hanhuo 长句输入评测：eval/eval_sentence.jsonl → 双组(正确/错拼) × 长度桶 基线报告。

每行: {"kind":"sent","text":目标句,"py":正确全拼,"py_typo":错拼版(可选),"stratum":"user|lyric|chat|news"}
指标:
  top1      精确命中首位（候选 == 目标句）
  frag@3/10 关键片段命中（top3/top10 任一候选含目标句任意连续 ≥4 字片段）
  present@30 在场率（目标句出现在 top30 内）
  桶        按 py 字节长 ≤30 / 31-45 / 46+
用法: python3 tools/sentence_eval.py [--jsonl PATH] [--bin PATH] [--topk 30] [--jobs 4]
B2.8 接线(engine/typo_retry.rs)后复跑同一命令即可对照提升。
"""
import argparse
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HANHUO = ROOT / "target" / "release" / "hanhuo"
FRAG_LEN = 4  # 关键片段最短字数


def bucket_of(py: str) -> str:
    n = len(py.encode())  # 全拼 ASCII：字节长 == 字符长
    if n <= 30:
        return "≤30"
    if n <= 45:
        return "31-45"
    return "46+"


def run_query(q: str, bin_path: Path, topk: int, timeout: int,
               ctx: str = ""):
    """返回 (候选词列表(截 topk), 耗时秒, 错误信息)。"""
    cmd = [str(HANHUO), "query", q, "--bin", str(bin_path),
           "--topk", str(topk), "--dp", "--json"]
    if ctx:
        cmd += ["--ctx", ctx]
    # EVAL_DICTS: 空格分隔的 --dict 词典列表（域词库臂）; EVAL_INJ_TOP=1
    # 同开 B2w-τ 提权
    for d in os.environ.get("EVAL_DICTS", "").split():
        cmd += ["--dict", d]
    if os.environ.get("EVAL_INJ_TOP"):
        cmd += ["--boost-inj-top"]
    if os.environ.get("EVAL_TYPO_RETRY"):
        cmd += ["--typo-retry"]  # B2.8 变体重试臂
    if os.environ.get("EVAL_NNLM"):
        cmd += ["--nnlm", os.environ.get("EVAL_NNLM_MODEL",
                "data/nnlm_mobile_fp32.safetensors")]  # 模型在场臂(B3 域门自动触发)
    if os.environ.get("EVAL_B3"):
        # B3 语境排序臂（需 EVAL_NNLM 供模型; ctx 由 EVAL_CTX 供）。
        # --boost-tail 同开: 显式分支会关 tail/ckvd 默认基座（附录13
        # 陷阱——单旗标对照无意义, 同基座才可比）
        cmd += ["--boost-nn", "--boost-tail"]
    if os.environ.get("EVAL_CTX_JP"):
        # c.3 语境组词臂（需 EVAL_NNLM + EVAL_CTX）
        cmd += ["--boost-ctx-jp"]
    if os.environ.get("EVAL_TONE"):
        # 声调臂: 调号输入 + --tone-filter（句首调对整句 top1 的增量探针）
        cmd += ["--tone-filter"]
    if os.environ.get("EVAL_BWCN_SENT"):
        # B2w-sent 臂: 域词边进 B8 段格（需 EVAL_DICTS 供包 + EVAL_SENT_NN
        # 供 --boost-seg-beam 基座——单开显式分支陷阱同附录13）
        cmd += ["--boost-bwcn-sent"]
    if os.environ.get("EVAL_SENT_NN"):
        # B7 臂 = 基线档（--boost-tail: B0+B2.5 尾槽+ckvd）+ B8 全段变体束
        # + B7 句序列 NNLM 重排（需 --nnlm 模型在场）。注: 伴生旗标不可省
        # ——wire_boost 显式档会关默认旋钮, 单 --boost-sent-nn 会把尾槽/ckvd
        # 关掉且句组仅 1 候选（B7 需 ≥2 才出手）→ 自灭臂。
        cmd += ["--nnlm", os.environ.get("EVAL_NNLM_MODEL",
                "data/nnlm_mobile_fp32.safetensors"),
                "--boost-tail", "--boost-sent-nn",
                # 2026-11-20 宽档升默认: 引擎 CLI --boost-sent-nn 单开已
                # 注入 beam150+cap160/8000, 脚本默认跟随（历史 13.2/18.7
                # 口径 = EVAL_SEG_BEAM=20 显式复现）
                "--boost-seg-beam=" + os.environ.get("EVAL_SEG_BEAM", "150")]
    t0 = time.time()
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=str(ROOT))
    except subprocess.TimeoutExpired:
        return [], time.time() - t0, f"timeout>{timeout}s"
    dt = time.time() - t0
    if r.returncode != 0:
        return [], dt, f"exit={r.returncode}"
    line = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else ""
    try:
        cands = json.loads(line)["cands"]
    except Exception as e:
        return [], dt, f"parse:{e}"
    return [c["word"] for c in cands[:topk]], dt, None


def frag_windows(text: str):
    return {text[i:i + FRAG_LEN] for i in range(len(text) - FRAG_LEN + 1)}


def score_row(text: str, words):
    """→ (rank 0=miss, frag3, frag10)。present@30 即 rank>=1。"""
    rank = next((i + 1 for i, w in enumerate(words) if w == text), 0)
    wins = frag_windows(text)
    hit = lambda k: any(any(wg in w for wg in wins) for w in words[:k])
    return rank, hit(3), hit(10)


def pct(a, n):
    return f"{a / n * 100:.1f}%" if n else "-"


def agg_rows(rows):
    """rows: list of dict(rank,frag3,frag10,ms) → 聚合字符串行。"""
    n = len(rows)
    if not n:
        return {"n": 0}
    return {
        "n": n,
        "top1": pct(sum(r["rank"] == 1 for r in rows), n),
        "frag3": pct(sum(r["frag3"] for r in rows), n),
        "frag10": pct(sum(r["frag10"] for r in rows), n),
        "present30": pct(sum(r["rank"] >= 1 for r in rows), n),
        "avg_ms": f"{sum(r['ms'] for r in rows) / n:.0f}",
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jsonl", default=str(ROOT / "eval" / "eval_sentence.jsonl"))
    ap.add_argument("--bin", default=str(Path.home() / "tmp" / "hanhuo-assets" / "wxq_dict.bin"))
    ap.add_argument("--topk", type=int, default=30)
    ap.add_argument("--timeout", type=int, default=60)
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--out", default=str(ROOT / "eval" / "sentence_eval_result.json"))
    args = ap.parse_args()

    if not HANHUO.exists():
        sys.exit(f"未找到 {HANHUO}，先 cargo build --release")
    bin_path = Path(args.bin).expanduser()
    rows = [json.loads(l) for l in Path(args.jsonl).read_text().splitlines() if l.strip()]
    rows = [r for r in rows if r.get("kind") == "sent"]

    # 任务 = (行号, 组, 查询串); EVAL_CTX=1 时带前句作上文
    # （模拟真实 IME 的上文流——组内第 1 句无上文, 从第 2 句起
    # ctx = 同层前一句的 text; 上下文组词/联想/B3 收益的系统量化）
    use_ctx = bool(os.environ.get("EVAL_CTX"))
    tasks = []
    for i, r in enumerate(rows):
        ctx = ""
        if use_ctx and i > 0 and rows[i - 1].get("stratum") == r.get("stratum"):
            ctx = rows[i - 1]["text"]
        tasks.append((i, "py", r["py"], ctx))
        if r.get("py_typo"):
            tasks.append((i, "typo", r["py_typo"], ctx))

    def work(t):
        i, grp, q, ctx = t
        words, dt, err = run_query(
            q, bin_path, args.topk, args.timeout, ctx)
        return {"i": i, "grp": grp, "q": q, "words": words, "ms": dt, "err": err}

    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        results = list(ex.map(work, tasks))

    detail = []
    for res in results:
        r = rows[res["i"]]
        if res["err"]:
            print(f"[WARN] {res['grp']} {r['text']}: {res['err']}", file=sys.stderr)
            rank, f3, f10 = 0, False, False
        else:
            rank, f3, f10 = score_row(r["text"], res["words"])
        detail.append({
            "stratum": r["stratum"], "grp": res["grp"], "text": r["text"],
            "q": res["q"], "bucket": bucket_of(r["py"]), "rank": rank,
            "frag3": f3, "frag10": f10, "ms": round(res["ms"] * 1000, 1),
            "err": res["err"], "top1_word": res["words"][0] if res["words"] else "",
        })

    ok = [d for d in detail if not d["err"]]
    print(f"\n# hanhuo 长句评测：{len(rows)} 句 × 正确组{len(rows)} / 错拼组"
          f"{sum(1 for d in detail if d['grp'] == 'typo')}（bin={bin_path.name}, topk={args.topk}, dp）\n")

    hdr = f"| {'分组':<14} | {'n':>3} | {'top1':>6} | {'frag@3':>6} | {'frag@10':>7} | {'present@30':>10} | {'avg ms':>6} |"
    sep = "|-" + "-" * 14 + "-|-" + "-" * 3 + "-|-" + "-" * 6 + "-|-" + "-" * 6 + "-|-" + "-" * 7 + "-|-" + "-" * 10 + "-|-" + "-" * 6 + "-|"

    def emit(title, items):
        print(f"\n## {title}")
        print(hdr); print(sep)
        for label, rs in items:
            a = agg_rows(rs)
            if a["n"] == 0:
                continue
            print(f"| {label:<14} | {a['n']:>3} | {a['top1']:>6} | {a['frag3']:>6} "
                  f"| {a['frag10']:>7} | {a['present30']:>10} | {a['avg_ms']:>6} |")

    emit("总览", [
        ("正确组", [d for d in ok if d["grp"] == "py"]),
        ("错拼组", [d for d in ok if d["grp"] == "typo"]),
    ])
    emit("正确组 × 长度桶", [
        (f"py {b}", [d for d in ok if d["grp"] == "py" and d["bucket"] == b])
        for b in ("≤30", "31-45", "46+")])
    emit("错拼组 × 长度桶", [
        (f"typo {b}", [d for d in ok if d["grp"] == "typo" and d["bucket"] == b])
        for b in ("≤30", "31-45", "46+")])
    emit("正确组 × 领域", [
        (f"py {s}", [d for d in ok if d["grp"] == "py" and d["stratum"] == s])
        for s in ("user", "lyric", "chat", "news")])
    emit("错拼组 × 领域", [
        (f"typo {s}", [d for d in ok if d["grp"] == "typo" and d["stratum"] == s])
        for s in ("user", "lyric", "chat", "news")])

    print("\n## 正确组 miss 明细（present@30 未命中 / top1 ≠ 目标）")
    for d in ok:
        if d["grp"] == "py" and (d["rank"] == 0 or d["rank"] > 1):
            print(f"- [{d['stratum']}|{d['bucket']}] rank={d['rank']} {d['text']} | top1={d['top1_word']}")
    print("\n## 错拼组片段命中明细（frag@10 命中者，即『几乎对了』）")
    hit_any = [d for d in ok if d["grp"] == "typo" and (d["frag10"] or d["rank"])]
    if hit_any:
        for d in hit_any:
            print(f"- [{d['stratum']}|{d['bucket']}] rank={d['rank']} frag10={d['frag10']} {d['text']}")
    else:
        print("- (无)")

    if args.out:
        Path(args.out).write_text(json.dumps(detail, ensure_ascii=False, indent=1))
        print(f"\n明细已写 {args.out}")


if __name__ == "__main__":
    main()
