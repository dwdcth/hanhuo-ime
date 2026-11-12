#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""aux_build.py — c.6 审计后续二: 辅助码双表开源重建（替换 swbd/rsd 反演件）

上次审计漏网: CLI `aux` 辅码语法（ni`wq→你）消费两件腾讯反演资产——
  wubi86_swbd.dict.yaml（swbd 桶哈希配对反演, 91k 行）
  rsd_char_strokes.tsv（rsd.bin 运行时提取, 29,515 字）
开源重建同格式:
  五笔: rime/rime-wubi wubi86.dict.yaml（极点五笔 6 表, LGPL）→
        剥频列成 词\t码 两列; auxform 取每字最长码=全码同律
  笔顺: 双源分层——
        T1 cnchar stroke-order-jian（MIT, 大陆笔顺口径, 6,939 字;
           字母→数字映射 EM 逐位投票定标: j/i→1横 f/g→2竖 s→3撇
           k/l/d→4捺点 其余→5折; 与 rsd 交集 99.77% 逐字一致）
        T2 rime-stroke stroke.dict.yaml（LGPL, CNS11643+北大论坛,
           110k 字宽覆盖; 台湾口径与大陆系统性差异 ~16%——月旁点/提
           等, 仅补 T1 未覆盖字）
验收: WUBI_YAML/RSD_TSV 指开源件 vs 反演件——装载计数 + aux 查询 A/B
"""
import argparse
import glob
import json
import os

OUT_WUBI = "data/wubi86_open.dict.yaml"
OUT_STROKES = "data/rsd_strokes_open.tsv"

# cnchar 26 字母 → rsd 数字口径（EM 逐位投票定标, 见模块 docstring）
CNC_MAP = {"j": "1", "i": "1", "f": "2", "g": "2",
           "s": "3", "k": "4", "l": "4", "d": "4"}
# rime-stroke 5 字母 → 数字
RST_MAP = {"h": "1", "s": "2", "p": "3", "n": "4", "z": "5"}

SKIP = ("#", "---", "...", "name:", "version:", "sort:",
        "use_preset_vocabulary:", "max_phrase_length:")


def parse_rime_rows(path):
    """rime dict 行 → (word, code, 频列) 生成器"""
    for ln in open(path, encoding="utf-8"):
        if ln.startswith(SKIP) or "\t" not in ln:
            continue
        p = ln.rstrip("\n").split("\t")
        if len(p) >= 2:
            yield p[0], p[1], (p[2] if len(p) > 2 else "")


def build_wubi(rime_path, out):
    n_char = n_word = 0
    with open(out, "w", encoding="utf-8") as f:
        f.write(
            "# wubi86 开源码表（c.6 审计后续二重建）: rime/rime-wubi\n"
            "# wubi86.dict.yaml（极点五笔 6 表, LGPL-3.0）剥频列;\n"
            "# 替换 wubi86_swbd（swbd 桶哈希配对反演, 仅本地研究）。\n"
            "---\nname: wubi86-open\nversion: \"1.0\"\nsort: original\n...\n\n")
        for w, code, _freq in parse_rime_rows(rime_path):
            if not code.strip():
                continue
            f.write(f"{w}\t{code}\n")
            if len(w) == 1:
                n_char += 1
            else:
                n_word += 1
    print(f"✓ {out}: 字级 {n_char:,} 行 / 词级 {n_word:,} 行")
    return n_char, n_word


def build_strokes(cnchar_json, rime_path, out):
    so = json.load(open(cnchar_json, encoding="utf-8"))
    t1 = {ch: "".join(CNC_MAP.get(x, "5") for x in s)
          for ch, s in so.items()}

    t2 = {}
    for w, code, _ in parse_rime_rows(rime_path):
        if len(w) == 1 and w not in t1:
            t2[w] = "".join(RST_MAP.get(c, "5") for c in code)

    def rows():
        for ch in sorted(t1):
            yield ch, t1[ch], 1
        for ch in sorted(t2):
            yield ch, t2[ch], 2

    with open(out, "w", encoding="utf-8") as f:
        f.write(
            "# rsd_strokes 开源笔顺表（c.6 审计后续二重建）: char\tcp\t数字笔顺\n"
            "# 口径 1横 2竖 3撇 4捺/点 5折。双源分层:\n"
            "#   T1 cnchar stroke-order-jian（MIT, 大陆口径; 与反演 rsd 交集\n"
            "#      逐字一致 99.77%）  T2 rime-stroke（LGPL, CNS11643+北大\n"
            "#      论坛宽覆盖; 台湾口径差异仅补 T1 未覆盖字）。\n"
            "# 替换 rsd_char_strokes.tsv（rsd.bin 提取, 仅本地研究）。\n")
        for ch, s, tier in rows():
            f.write(f"{ch}\t0x{ord(ch):X}\t{s}\n")
    print(f"✓ {out}: T1 {len(t1):,} + T2 {len(t2):,} = {len(t1)+len(t2):,} 字")
    return t1, t2


def report(t1, t2):
    """开源件 vs 反演件一致率（在场时）+ 词库字集覆盖"""
    vocab = set()
    for y in glob.glob("data/domain/*.dict.yaml"):
        for ln in open(y, encoding="utf-8"):
            if not ln.startswith(("#", "---", "...")) and "\t" in ln:
                vocab |= set(ln.split("\t")[0])
    home = os.path.expanduser("~")
    rsd = {}
    try:
        for i, ln in enumerate(open(f"{home}/tmp/hanhuo/runtime_maps/"
                                    "rsd_char_strokes.tsv", encoding="utf-8")):
            if i == 0 or ln.startswith(("#", "char\t")):
                continue
            p = ln.rstrip("\n").split("\t")
            if len(p) >= 3 and p[2]:
                rsd[p[0]] = p[2]
    except FileNotFoundError:
        pass
    if rsd:
        inter = set(t1) & set(rsd)
        same = sum(1 for c in inter if t1[c] == rsd[c])
        print(f"  T1 vs rsd: 交集 {len(inter):,} 一致 "
              f"{same} ({same/len(inter)*100:.2f}%)")
    cov = vocab & (set(t1) | set(t2))
    print(f"  词库字集 {len(vocab):,} 覆盖 {len(cov):,} "
          f"({len(cov)/len(vocab)*100:.1f}%), 其中 T1 口径 "
          f"{len(vocab & set(t1)):,}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rime-wubi",
                    default="/mnt/s790/tmp/rime-wubi/wubi86.dict.yaml")
    ap.add_argument("--cnchar", default="/mnt/s790/tmp/so_jian.json")
    ap.add_argument("--rime-stroke",
                    default="/mnt/s790/tmp/rime-stroke/stroke.dict.yaml")
    ap.add_argument("--out-wubi", default=OUT_WUBI)
    ap.add_argument("--out-strokes", default=OUT_STROKES)
    a = ap.parse_args()

    build_wubi(a.rime_wubi, a.out_wubi)
    t1, t2 = build_strokes(a.cnchar, a.rime_stroke, a.out_strokes)
    report(t1, t2)


if __name__ == "__main__":
    main()
