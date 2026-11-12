#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""ext_pack_build.py — 扩展词库包（wanxiang jichu+mingren ≥4 字 → ext.hbwc）

c.5 v1 包线（2026-11-11）: τ 长度门后 sent 残余包外损失定性为 open bin
词表缺口（俗语/外国专名/政史长词）。bin 合流受阻于 open_build
max_syl=4 剪枝（斯德哥尔摩 5 音节/俗语 6+ 全被剪）——包线 τ 钉序
无音节上限, 不动 bin 不碰 golden 冻结。

取 jichu.dict.yaml + mingren.dict.yaml 的 ≥4 字词（≤3 字 = 基础词汇,
rime-ice 基座已覆盖, 且 τ 长度门本就不钉 <3 字）; 万象格式双形
（\u{} 转义 + 声调注音; ├┤ 分隔变体）均解析; 跨源按 (词, 读音) 去重。

用法: python3 tools/ext_pack_build.py
产出: data/domain/ext.dict.yaml / ext.hbwc / eval/eval_ext.jsonl
许可: wanxiang 词库 CC BY 4.0——分发需随附来源与署名。
"""
import json
import os
import re
import subprocess
import sys
import unicodedata

WANX = os.path.expanduser("~/tmp/wanxiang_ds/dicts")
OUT = "data/domain"
MIN_CHARS = 4
SOURCES = [
    (f"{WANX}/jichu.dict.yaml", "jichu"),
    (f"{WANX}/mingren.dict.yaml", "mingren"),
]

_U_ESC = re.compile(r"\\u\{([0-9a-fA-F]+)\}")
_SEP = re.compile(r"├[─┤]*┤?")


def _strip_tones(s):
    """声调注音（ā·bǎo）→ 纯音节序列; · 为音节分隔"""
    nfd = unicodedata.normalize("NFD", s)
    plain = "".join(c for c in nfd if not unicodedata.combining(c))
    return plain.replace("·", " ").split()


def main():
    seen = set()
    rows = []  # (word, 拼音空格形, 频次)
    for path, tag in SOURCES:
        if not os.path.exists(path):
            print(f"跳过 {tag}: {path} 不存在")
            continue
        n_src = 0
        for raw in open(path, encoding="utf-8"):
            ln = _U_ESC.sub(lambda m: chr(int(m.group(1), 16)), raw.rstrip("\n"))
            if ln.startswith(("#", "---", "...")):
                continue
            parts = [x.strip() for x in _SEP.split("\t".join(ln.split("\t")))
                     if x] if "├" in ln else ln.split("\t")
            parts = [x for x in parts if x]
            if len(parts) < 2:
                continue
            w = parts[0]
            if len(w) < MIN_CHARS:
                continue
            syls = _strip_tones(parts[1]) if not parts[1].isascii() \
                else parts[1].split()
            if len(syls) != len(w):
                continue
            if not all(s.isascii() and s.isalpha() and s.islower() for s in syls):
                continue
            fq = parts[2] if len(parts) > 2 and parts[2].isdigit() else "1"
            k = (w, " ".join(syls))
            if k in seen:
                continue
            seen.add(k)
            rows.append((w, k[1], int(fq)))
            n_src += 1
        print(f"{tag}: {n_src:,} 词（≥{MIN_CHARS}字）")

    if not rows:
        sys.exit("无产出——检查源表是否已 clone 到 ~/tmp/wanxiang_ds")
    os.makedirs(OUT, exist_ok=True)
    yml = f"{OUT}/ext.dict.yaml"
    with open(yml, "w", encoding="utf-8") as f:
        f.write(f"# 扩展词库（wanxiang jichu+mingren ≥{MIN_CHARS}字, "
                "CC BY 4.0, tools/ext_pack_build.py 导入）\n---\n")
        for w, p, c in rows:
            f.write(f"{w}\t{p}\t{c}\n")
    r = subprocess.run(
        ["python3", "tools/dict_build.py", yml, "--out", f"{OUT}/ext.hbwc"],
        capture_output=True, text=True)
    sys.stdout.write(r.stdout[-200:] if r.returncode == 0 else r.stderr)
    if r.returncode != 0:
        sys.exit(1)
    with open(f"eval/eval_ext.jsonl", "w", encoding="utf-8") as f:
        for w, p, c in rows:
            f.write(json.dumps({"kind": "word", "stratum": "ext",
                                "text": w, "py": p.replace(" ", ""),
                                "freq": c}, ensure_ascii=False) + "\n")
    print(f"扩展(ext): {len(rows):,} 词")


if __name__ == "__main__":
    main()
