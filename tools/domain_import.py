#!/usr/bin/env python3
"""domain_import.py — THUOCL 领域词库 → rime yaml + HBWC + 领域 eval 集

批量导入管线（2026-11-08 领域词库轮）:
  1. THUOCL 领域表（word\\tfreq, 本地 ext_vocab）× slwd 拼音词典
     （词→原生兼容拼音, ~/tmp/hanhuo/slwd/slwd_pinyin.dict.yaml）
     → data/domain/<domain>.dict.yaml（rime 形, dict_build.py 直接吃）
  2. 领域 eval 集 eval/eval_domain.jsonl（kind=domain, 原生缺失词为增量探针）
  3. 报告: 拼音覆盖率/长度分布/增量面

用法:
  python3 tools/domain_import.py [--eval-only] [--domains it,medical,law,caijing]

拼音覆盖缺口（THUOCL 词不在 slwd）: 丢弃并计数——宁缺毋滥, 错误拼音的
注入词是负资产（用户打对拼音出不了词比缺词更伤）。
词库许可: THUOCL 开放中文词库（清华, 允许研究/免费使用）; slwd 拼音
来源资产仅本地个人使用, 禁止分发。
"""
import os, sys, json, subprocess

HOME = os.path.expanduser("~")
THUOCL_DIR = f"{HOME}/tmp/hanhuo/ext_vocab/pmim-data-thuocl/thuocl"
SLWD_PY = f"{HOME}/tmp/hanhuo/slwd/slwd_pinyin.dict.yaml"
OUT_DIR = "data/domain"
EVAL_OUT = "eval/eval_domain.jsonl"
# 全域清单（THUOCL 本地 12 域; 微信"细胞词库"式分域的开放词源对应物）
DOMAINS = ["it", "medical", "law", "caijing", "poem", "chengyu",
           "diming", "food", "animal", "car", "lishimingren"]


def load_slwd_pinyin():
    """词 → 拼音(空格分隔)。多读音取首个（slwd 频序即最高频形）。"""
    m = {}
    with open(SLWD_PY, encoding="utf-8") as f:
        for ln in f:
            if ln.startswith("#") or ln.startswith("---") or ln.startswith("..."):
                continue
            parts = ln.rstrip("\n").split("\t")
            if len(parts) < 2:
                continue
            w, py = parts[0], parts[1]
            if w and py and w not in m:
                m[w] = py
    return m


def thuocl(d):
    p = f"{THUOCL_DIR}/THUOCL_{d}.txt"
    out = []
    with open(p, encoding="utf-8") as f:
        for ln in f:
            parts = ln.rstrip("\n").split("\t")
            if len(parts) == 2:
                w = parts[0].strip()  # it 表词尾带空格
                if w and parts[1].strip().isdigit():
                    out.append((w, int(parts[1].strip())))
    return out


def pypinyin_of(w):
    """pypinyin 兜底（Style.NORMAL 已是引擎 v 约定）; 非 ascii 音节 → None"""
    from pypinyin import pinyin, Style
    syl = pinyin(w, style=Style.NORMAL)
    flats = [x[0] for x in syl]
    if any((not s.isascii()) or (not s.isalpha()) or (not s.islower()) for s in flats):
        return None
    return " ".join(flats)


def main():
    args = sys.argv[1:]
    domains = DOMAINS
    for a in args:
        if a.startswith("--domains="):
            domains = a.split("=", 1)[1].split(",")
    eval_only = "--eval-only" in args

    py = load_slwd_pinyin()
    print(f"slwd 拼音词典: {len(py):,} 词")

    os.makedirs(OUT_DIR, exist_ok=True)
    eval_rows = []
    for d in domains:
        words = thuocl(d)
        cov = []          # (word, py, freq, src)  src: slwd/pypinyin
        drop = 0
        for w, fr in words:
            if w in py:
                cov.append((w, py[w], fr, "slwd"))
                continue
            pp = pypinyin_of(w)
            if pp:
                cov.append((w, pp, fr, "pypinyin"))
            else:
                drop += 1
        # rime yaml（dict_build.py 输入形: word\tp y p\tfreq）
        yml = f"{OUT_DIR}/{d}.dict.yaml"
        if not eval_only:
            with open(yml, "w", encoding="utf-8") as f:
                f.write(f"# THUOCL {d} 领域词库（拼音源 slwd/pypinyin）\n---\n")
                for w, p, fr, _ in cov:
                    f.write(f"{w}\t{p}\t{fr}\n")
            # HBWC v1
            hb = f"{OUT_DIR}/{d}.hbwc"
            r = subprocess.run(
                ["python3", "tools/dict_build.py", yml, "--out", hb],
                capture_output=True, text=True)
            if r.returncode != 0:
                print(r.stderr); sys.exit(1)
        # eval 探针: kind=word（Rust 聚合只有 word/sent 两类）, 域标签走
        # stratum 自由桶 → 聚合打印 word:it / word:medical 独立行
        for w, p, fr, _ in cov:
            eval_rows.append({
                "kind": "word", "stratum": d, "text": w,
                "py": p.replace(" ", ""), "freq": fr})
        dist = {}
        for w, _, _, _ in cov:
            dist[len(w)] = dist.get(len(w), 0) + 1
        dist_s = " ".join(f"{k}字:{v}" for k, v in sorted(dist.items()))
        n_slwd = sum(1 for c in cov if c[3] == "slwd")
        print(f"{d}: {len(words):,} 词 → 覆盖 {len(cov):,}"
              f"（slwd {n_slwd}/pypinyin {len(cov)-n_slwd}, 弃 {drop}）"
              f" [{dist_s}]")

    with open(EVAL_OUT, "w", encoding="utf-8") as f:
        for r in eval_rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    print(f"eval 探针 {len(eval_rows):,} 条 → {EVAL_OUT}")
    print("下一步: hanhuo eval 基线（无 --dict）定增量面, 再挂 --dict 对比")


if __name__ == "__main__":
    main()
