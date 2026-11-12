#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""rime_pack_import.py — rime 生态分域词库 → 汉火域包（缺源分类补全）

用户指源（2026-11-11）: gaboolic/rime-frost（GPL-3.0, cn_dicts_cell
细胞词库——游戏/体育/网络热词等）+ amzxyz/rime-wanxiang-data
（CC BY 4.0, dicts/——艺人/药品/化学/物种/方言/缩写等）。
两源均为 rime dict.yaml——dict_build.py 直接格式, 本工具只做
选择/规范化（表头剥离、拼音列校验、频次列缺省 1）。

用法: python3 tools/rime_pack_import.py            # 全部缺源分类
      python3 tools/rime_pack_import.py game sport # 指定子集
许可: frost GPL-3.0 / wanxiang CC BY 4.0（词库数据）——分发需随附
来源与许可声明（词库目录.md 已注）。
"""
import os, subprocess, sys

FROST = os.path.expanduser("~/tmp/frost_ds/cn_dicts_cell")
WANX = os.path.expanduser("~/tmp/wanxiang_ds/dicts")
OUT = "data/domain"

# (包名, 源路径, 分类中文名, 源与许可)
SOURCES = [
    ("game",  f"{FROST}/game.dict.yaml",   "游戏",     "rime-frost GPL-3.0"),
    ("sport", f"{FROST}/sport.dict.yaml",  "体育",     "rime-frost GPL-3.0"),
    ("hot",   f"{FROST}/exthot.dict.yaml", "网络热词", "rime-frost GPL-3.0"),
    ("music", f"{FROST}/music.dict.yaml",  "音乐",     "rime-frost GPL-3.0"),
    ("chess", f"{FROST}/chess2.dict.yaml", "棋牌",     "rime-frost GPL-3.0"),
    ("media", f"{FROST}/media.dict.yaml",  "影视",     "rime-frost GPL-3.0"),
    ("yiren", f"{WANX}/yiren.dict.yaml",   "艺人",     "wanxiang CC BY 4.0"),
    ("yaopin", f"{WANX}/yaopin.dict.yaml", "药品",     "wanxiang CC BY 4.0"),
    ("huaxue", f"{WANX}/huaxue.dict.yaml", "化学",     "wanxiang CC BY 4.0"),
    ("wuzhong", f"{WANX}/wuzhong.dict.yaml", "物种",   "wanxiang CC BY 4.0"),
    ("fangyan", f"{WANX}/fangyan.dict.yaml", "方言",   "wanxiang CC BY 4.0"),
    ("acg", os.path.expanduser(
        "~/tmp/moegirl.dict.yaml"), "二次元", "mw2fcitx 萌娘百科 CC BY-NC-SA"),
    # abbrev 不入: 万象缩写表是 词→简拼首字母 反查形（感觉→gj）,
    # 非全拼词典——B2.7 简拼表域资产, 不走域包管线
]


import re, unicodedata

_U_ESC = re.compile(r"\\u\{([0-9a-fA-F]+)\}")


def _decode_u(s):
    r"""万象 \u{XXXX} 转义 → 明文"""
    return _U_ESC.sub(lambda m: chr(int(m.group(1), 16)), s)


def _strip_tones(s):
    """声调注音（ā·bǎo）→ 纯音节; · 为音节分隔"""
    nfd = unicodedata.normalize("NFD", s)
    plain = "".join(c for c in nfd if not unicodedata.combining(c))
    return plain.replace("·", " ").split()


def normalize(src):
    """rime yaml 正文行 → (word, 拼音空格形, 频次); 拼音列残缺/非小写
    ascii 丢弃（宁缺毋滥律）; 频次列缺省 1。万象变体: U 转义 +
    声调注音 + 制表符分隔"""
    out = []
    for raw in open(src, encoding="utf-8"):
        ln = _decode_u(raw.rstrip("\n"))
        if "├" in ln:  # 万象形: word├┤pinyin├─┤weight
            seg = re.split(r"├[─┤]*┤?", ln)
            seg = [x for x in seg if x]
            if len(seg) >= 2:
                w = seg[0].strip()
                syls = _strip_tones(seg[1])
                freq = seg[2].strip() if len(seg) > 2 and seg[2].strip().isdigit() else 1
                if w and len(syls) == len(w) and all(
                        x.isascii() and x.isalpha() and x.islower() for x in syls):
                    out.append((w, " ".join(syls), int(freq)))
            continue
        ln = ln
        if ln.startswith(("#", "...", "---", "\t")) or not ln:
            continue
        parts = ln.split("\t")
        if len(parts) < 2:
            continue
        w, py = parts[0].strip(), parts[1].strip()
        if not w or not py:
            continue
        # 声调注音形（万象: ā bǎo）剥调（NFD）; 无调形直过
        syls = (_strip_tones(py) if not py.isascii() else py.split())
        if not all(s.isascii() and s.isalpha() and s.islower() for s in syls):
            continue
        if len(syls) != len(w):
            continue
        freq = parts[2].strip() if len(parts) > 2 and parts[2].strip().isdigit() else 1
        out.append((w, " ".join(syls), int(freq)))
    return out


def main():
    want = [a for a in sys.argv[1:] if not a.startswith("-")]
    os.makedirs(OUT, exist_ok=True)
    import json
    for name, src, cname, lic in SOURCES:
        if want and name not in want:
            continue
        if not os.path.exists(src):
            print(f"跳过 {name}: {src} 不存在")
            continue
        cov = normalize(src)
        if not cov:
            print(f"跳过 {name}: 规范化后为空")
            continue
        yml = f"{OUT}/{name}.dict.yaml"
        with open(yml, "w", encoding="utf-8") as f:
            f.write(f"# {cname}（{lic}, tools/rime_pack_import.py 导入）\n---\n")
            for w, p, c in cov:
                f.write(f"{w}\t{p}\t{c}\n")
        r = subprocess.run(
            ["python3", "tools/dict_build.py", yml, "--out", f"{OUT}/{name}.hbwc"],
            capture_output=True, text=True)
        if r.returncode != 0:
            print(r.stderr)
            sys.exit(1)
        with open(f"eval/eval_{name}.jsonl", "w", encoding="utf-8") as f:
            for w, p, c in cov:
                f.write(json.dumps({"kind": "word", "stratum": name,
                                    "text": w, "py": p.replace(" ", ""),
                                    "freq": c}, ensure_ascii=False) + "\n")
        print(f"{cname}({name}): {len(cov):,} 词")


if __name__ == "__main__":
    main()
