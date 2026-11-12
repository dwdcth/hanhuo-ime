#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""bgrm_build.py — c.6 P4b: 开源重建 bgrm 词 bigram 三元组（用户指令试训）

格式（bgrm_safetensors.rs）: w1 U32 / w2 U32 / val U16, 按 (w1, w2)
有序——引擎 bigram_after 对 w1 带 searchsorted。词 id = bin 词层序号
（hanhuo words --bin open/hanhuo_open.bin 导出映射）。
val = min(bigram 频次, 65535)（u16 饱和, 保序）。

用法: python3 tools/bgrm_build.py [--bin open/hanhuo_open.bin] [--workers 12]
产出: data/bgrm_open.safetensors
验收: --bgrm 挂载 + fuzz 臂 A/B（bigram_after 消费点在 fuzz 通道）
"""
import json
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lmcn_build import segment_mp  # noqa: E402  (jieba×N 分词复用)

LCCC = os.path.expanduser("~/tmp/corpus/lccc/lccc_base_train.jsonl.gz")


def write_safetensors(path, w1, w2, val):
    """手工容器（trainer 同律）: [u64 len][JSON][blob]"""
    t = {"w1": ([len(w1)], struct.pack(f"<{len(w1)}I", *w1), "U32"),
         "w2": ([len(w2)], struct.pack(f"<{len(w2)}I", *w2), "U32"),
         "val": ([len(val)], struct.pack(f"<{len(val)}H", *val), "U16")}
    names = sorted(t)
    blob = bytearray()
    hdr = {"__metadata__": {"format": "hanhuo-bgrm", "version": "1",
                            "license_note": "开源重建（c.6 P4b）: "
                                            "LCCC jieba bigram, Apache-2.0"}}
    for n in names:
        shape, data, dtyp = t[n]
        s = len(blob)
        blob.extend(data)
        hdr[n] = {"dtype": dtyp, "shape": shape,
                  "data_offsets": [s, len(blob)]}
    hb = json.dumps(hdr, ensure_ascii=False,
                    separators=(",", ":")).encode()
    with open(path, "wb") as f:
        f.write(struct.pack("<Q", len(hb)))
        f.write(hb)
        f.write(blob)
    print(f"✓ {path}: {len(w1):,} 三元组 "
          f"({os.path.getsize(path)/1e6:.1f} MB)")


def main():
    import argparse
    from collections import Counter
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="open/hanhuo_open.bin")
    ap.add_argument("--words", default="/tmp/open_words.txt")
    ap.add_argument("--out", default="data/bgrm_open.safetensors")
    ap.add_argument("--workers", type=int, default=12)
    a = ap.parse_args()

    wid = {}
    for ln in open(a.words, encoding="utf-8"):
        i, w = ln.rstrip("\n").split("\t", 1)
        wid[w] = int(i)
    print(f"词 id 映射 {len(wid):,}")

    print("jieba 分词计数中…", flush=True)
    bg = Counter()
    uni = Counter()  # w1 unigram（条件概率分母）
    for words in segment_mp([LCCC], a.workers):
        for i, w in enumerate(words):
            if w in wid:
                uni[wid[w]] += 1
            if i + 1 < len(words):
                w1, w2 = w, words[i + 1]
                if w1 in wid and w2 in wid:  # 双词入 bin 词层才有效
                    bg[(wid[w1], wid[w2])] += 1
    print(f"bin 内 bigram {len(bg):,} 种")

    # val 语义修正（2026-11-20 联想质检: 原存频次计数, 引擎
    # val_to_log10p 按量化 logP 解码——lm=+46 荒谬值暴露错位）:
    # val = 四段量化(−log10 P(w2|w1) × 1024)（n_to_val 逆, 同
    # open_build c4g 的 log10p_to_val 段公式）
    import math
    _BASE, _B1, _B2, _B3, _VMAX = 47104, 49152, 50176, 51200, 51742
    def lp_to_val(lp):
        n = max(0, int(round(-lp * 1024)))
        if n < 2048:
            v = _BASE + n
        elif n < 4096:
            v = _B1 + (n - 2048) // 2
        elif n < 8192:
            v = _B2 + (n - 4096) // 4
        else:
            v = _B3 + (n - 8192) // 8
        return max(0, min(v, _VMAX))

    triples = sorted(
        (k[0], k[1], lp_to_val(math.log10(c / uni.get(k[0], c))))
        # min count 10（2026-11-20 联想质检二: c≥2 时生僻后继 lift 爆表
        # ——你好枙 共现 2 次但独立频 3, P≈0.67 压过 你好啊; 腾讯 766k
        # 对的过滤度对标, 宁缺毋滥）
        for k, c in bg.items() if c >= 10
    )
    write_safetensors(a.out,
                      [t[0] for t in triples],
                      [t[1] for t in triples],
                      [t[2] for t in triples])


if __name__ == "__main__":
    main()
