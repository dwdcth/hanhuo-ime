#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""open_dict_import.py — 开源词典重建域包（用户指令 2026-11-11:
「用开源的词库重新做我们的词库, 比如成语词库, 古诗词库等」）

双源（均 MIT）:
  1. chinese-xinhua idiom.json（30,895 成语, 源注音权威——NFD 剥声调
     符号得纯音节, 避免 pypinyin 多音字歧义; 自怨自艾 yi4 类源对）
     → data/domain/idiom.dict.yaml + idiom.hbwc + eval/eval_idiom.jsonl
  2. chinese-poetry（全唐诗 poet.tang + 全宋诗 poet.song + 宋词
     ci.song, ~34 万首）× n-gram 跨文件搭配挖掘（lyric_import 同律:
     2..4 字, 跨 ≥2 文件, 边缘功能字过滤）
     → data/domain/poetry.dict.yaml + poetry.hbwc + eval/eval_poetry.jsonl

输出与 domain_import/lyric_import 同出口（rime yaml → dict_build HBWC
→ eval 集）, 引擎侧零改动。

用法: python3 tools/open_dict_import.py [--max-words 150000]
词库许可: chinese-xinhua MIT / chinese-poetry MIT——纯开放数据可分发。
"""
import glob, json, os, re, subprocess, sys, unicodedata, collections

HOME = os.path.expanduser("~")
XINHUA = f"{HOME}/tmp/xinhua_dict/data/idiom.json"
POETRY = f"{HOME}/tmp/poetry_ds"
OUT_DIR = "data/domain"

EDGE_STOP = set("的了是在我有你他和她就也都吧吗呢啊呗呀哦嘛么之乎者")
CJK = re.compile(r"[一-鿿]")


def plain_pinyin(tone_marked):
    """声调注音（ā bí dì yù）→ 纯音节（a bi di yu）: NFD 剥组合符"""
    s = unicodedata.normalize("NFD", tone_marked)
    return "".join(c for c in s if not unicodedata.combining(c))


def write_pack(name, cov, comment):
    os.makedirs(OUT_DIR, exist_ok=True)
    yml = f"{OUT_DIR}/{name}.dict.yaml"
    with open(yml, "w", encoding="utf-8") as f:
        f.write(f"# {comment}（开源词典重建, 见 tools/open_dict_import.py）\n---\n")
        for w, p, c in cov:
            f.write(f"{w}\t{p}\t{c}\n")
    hb = f"{OUT_DIR}/{name}.hbwc"
    r = subprocess.run(["python3", "tools/dict_build.py", yml, "--out", hb],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stderr)
        sys.exit(1)
    with open(f"eval/eval_{name}.jsonl", "w", encoding="utf-8") as f:
        for w, p, c in cov:
            f.write(json.dumps({"kind": "word", "stratum": name,
                                "text": w, "py": p.replace(" ", ""),
                                "freq": c}, ensure_ascii=False) + "\n")
    dist = collections.Counter(len(w) for w, _, _ in cov)
    print(f"{name}: {len(cov):,} 词 "
          f"({' '.join(f'{k}字:{v}' for k, v in sorted(dist.items()))})")


def build_idiom():
    data = json.load(open(XINHUA, encoding="utf-8"))
    cov, drop = [], 0
    for e in data:
        w = e.get("word", "")
        py = plain_pinyin(e.get("pinyin", ""))
        syls = py.split()
        # 四字成语为主（含少量 3/5+）; 音节数须等字数（注音残缺行丢弃）
        if not w or len(syls) != len(w):
            drop += 1
            continue
        if not all(s.isascii() and s.isalpha() and s.islower() for s in syls):
            drop += 1
            continue
        cov.append((w, " ".join(syls), 1))
    write_pack("idiom", cov,
               "成语词典 chinese-xinhua(idiom.json 30,895) 源注音 NFD 剥调")
    print(f"  （注音残缺丢弃 {drop}）")
    return cov


def _poem_paras(poems):
    """schema 容错: 诗集 json 元素可能是 dict（paragraphs/content/
    contents 字段）或 str——统一产出段文本"""
    for pm in poems:
        if isinstance(pm, str):
            yield pm
            continue
        if not isinstance(pm, dict):
            continue
        for key in ("paragraphs", "contents", "content", "paras"):
            v = pm.get(key)
            if isinstance(v, list):
                yield from (x for x in v if isinstance(x, str))
            elif isinstance(v, str):
                yield v


def iter_poem_lines():
    """唐宋诗/宋词 + 诗经/楚辞/蒙学/论语/四书/元曲/五代 → 逐行产出"""
    pats = [f"{POETRY}/全唐诗/poet.tang.*.json",
            f"{POETRY}/全唐诗/poet.song.*.json",
            f"{POETRY}/宋词/ci.song.*.json",
            f"{POETRY}/诗经/**/*.json",
            f"{POETRY}/楚辞/**/*.json",
            f"{POETRY}/蒙学/**/*.json",
            f"{POETRY}/论语/**/*.json",
            f"{POETRY}/四书五经/**/*.json",
            f"{POETRY}/元曲/**/*.json",
            f"{POETRY}/五代诗词/**/*.json"]
    n_files = 0
    for pat in pats:
        for fp in sorted(glob.glob(pat)):
            try:
                poems = json.load(open(fp, encoding="utf-8"))
            except Exception:
                continue
            n_files += 1
            for para in _poem_paras(poems):
                for ln in re.split(r"[，。！？；：、]", para):
                        ln = ln.strip()
                        if ln and CJK.search(ln):
                            yield ln
    print(f"诗词文件 {n_files} 个", file=sys.stderr)


def build_poetry(max_words, min_count=3):
    cnt = collections.Counter()
    ng = collections.Counter()
    for ln in iter_poem_lines():
        cln = "".join(c for c in ln if "一" <= c <= "鿿")
        if len(cln) >= 4:
            for L in (2, 3, 4):
                for i in range(len(cln) - L + 1):
                    g = cln[i:i + L]
                    if g[0] in EDGE_STOP or g[-1] in EDGE_STOP:
                        continue
                    ng[g] += 1
    for g, c in ng.items():
        if c >= min_count:
            cnt[g] = c
    # 行级单元改 **逗号相邻对**（piece-pair）: 段 → 句 → 逗号片 →
    # 相邻对（天苍苍，野茫茫 → 天苍苍野茫茫, ≤8 字）——诗句续打的
    # 自然单元; 整句常超 8 字被长帽切掉（敕勒歌 12 字案根因）。
    # 门: 对 ≤5 字跨 ≥2 文件; ≥6 字单文件即收（长对天然精确）。
    # 配额分离: 词类（n-gram）≤ 140k 保底, 对类补足至 max——
    # 防止单文件长对洪泛挤掉词类（200k 全对化实测病理）。
    pair_files = collections.defaultdict(set)
    for pat in [f"{POETRY}/蒙学/**/*.json", f"{POETRY}/诗经/**/*.json",
                f"{POETRY}/楚辞/**/*.json", f"{POETRY}/全唐诗/poet.*.json",
                f"{POETRY}/宋词/ci.song.*.json"]:
        for fp in sorted(glob.glob(pat, recursive=True)):
            try:
                poems = json.load(open(fp, encoding="utf-8"))
            except Exception:
                continue
            for para in _poem_paras(poems):
                for sent in re.split(r"[。！？；]", para):
                    pieces = ["".join(c for c in pc if "一" <= c <= "鿿")
                              for pc in re.split(r"[，、]", sent)]
                    pieces = [pc for pc in pieces if pc]
                    for a, b in zip(pieces, pieces[1:]):
                        pair = a + b
                        if 4 <= len(pair) <= 8:
                            pair_files[pair].add(fp)
    pairs = {p: len(fs) for p, fs in pair_files.items()
             if len(fs) >= 2 or len(p) >= 6}
    # 拼音: pypinyin（无开源注音源——诗词词 pypinyin 首读可信度同
    # lyric 线; 宁缺毋滥丢非 ascii）
    from pypinyin import pinyin, Style
    from opencc import OpenCC
    t2s = OpenCC("t2s")  # 全宋诗/楚辞等集为繁体——统一转简（用户简体
    # 输入面）; 转换后去重合并计数
    merged = {}
    for w, c in cnt.items():
        merged[t2s.convert(w)] = max(merged.get(t2s.convert(w), 0), c)
    cnt = merged
    pair_s = {}
    for pr, f in pairs.items():
        ps = t2s.convert(pr)
        pair_s[ps] = max(pair_s.get(ps, 0), f)
    pairs = pair_s
    word_quota = max_words * 7 // 10
    words_c = sorted(cnt.items(), key=lambda x: -x[1])[:word_quota]
    pair_c = sorted(((p, 100 + f) for p, f in pairs.items()),
                    key=lambda x: -x[1])[: max_words - len(words_c)]
    cov = []
    for w, c in words_c + pair_c:
        flats = [x[0] for x in pinyin(w, style=Style.NORMAL)]
        if any((not s.isascii()) or not s.isalpha() for s in flats):
            continue
        cov.append((w, " ".join(flats), c))
    write_pack("poetry", cov,
               "古诗词 chinese-poetry(唐宋诗宋词) n-gram 跨文件搭配")
    return cov


def main():
    max_words = 150000
    for a in sys.argv[1:]:
        if a.startswith("--max-words="):
            max_words = int(a.split("=", 1)[1])
    build_idiom()
    build_poetry(max_words)


if __name__ == "__main__":
    main()
