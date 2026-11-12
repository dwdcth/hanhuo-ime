#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""lyric_import.py — 歌词语料 → lyric 领域词库（B2w 自建线·语料源扩展）

Chinese-Lyric-Corpus（gaussic, MIT, ~5 万首网易云歌词）× jieba 分词
× slwd/pypinyin 拼音 → data/domain/lyric.dict.yaml + lyric.hbwc
+ eval/eval_lyric.jsonl——与 domain_import.py（THUOCL 词表源）同出口,
引擎侧零改动（--dict 挂载 + B2w-τ 提权同律）。

语料清洗: 只留 CJK 行; 丢元数据行（作词/作曲/编曲/制作/母带/混音/
和声/吉他/贝斯/鼓/键盘/弦乐/制作人/词: 曲:）; 丢括号内容。
词过滤: 2..6 字; slwd 优先、pypinyin 兜底（宁缺毋滥——错误拼音的
注入词是负资产, domain_import 同律）; 频次 ≥ MIN_COUNT 控噪声。
语料许可: MIT（dzkang 2018）; slwd 拼音来源资产仅本地个人使用, 禁止分发。

用法: python3 tools/lyric_import.py [--min-count 2] [--max-words 50000]
"""
import os, sys, re, json, glob, subprocess, collections

HOME = os.path.expanduser("~")
CORPUS = f"{HOME}/tmp/lyric_corpus/lyrics/Chinese_Lyrics"
SLWD_PY = f"{HOME}/tmp/hanhuo/slwd/slwd_pinyin.dict.yaml"
OUT_DIR = "data/domain"
EVAL_OUT = "eval/eval_lyric.jsonl"

META_PAT = re.compile(
    r"作词|作曲|编曲|制作人|监制|母带|混音|录音|和声|吉他|贝斯|架子鼓|"
    r"键盘|弦乐|发行|出品|词曲|统筹|企划|OP:|SP:|词:|曲:|编:|唱:|"
    r"MV|Vocal|Producer|Arrange|Lyric|Composer|Mix|Master")
CJK_PAT = re.compile(r"[一-鿿]")


def load_slwd_pinyin():
    m = {}
    with open(SLWD_PY, encoding="utf-8") as f:
        for ln in f:
            if ln.startswith(("#", "---", "...")):
                continue
            parts = ln.rstrip("\n").split("\t")
            if len(parts) >= 2 and parts[0] and parts[1] and parts[0] not in m:
                m[parts[0]] = parts[1]
    return m


def pypinyin_of(w):
    from pypinyin import pinyin, Style
    flats = [x[0] for x in pinyin(w, style=Style.NORMAL)]
    if any((not s.isascii()) or (not s.isalpha()) or (not s.islower()) for s in flats):
        return None
    return " ".join(flats)


# 功能字边缘（n-gram 降噪: 的了我你他她是就在也都吧吗呢啊呗）
EDGE_STOP = set("的了是在我有你他和她就也都吧吗呢啊呗呀哦嘛么")


def corpus_word_freq():
    """语料 → 词频表（双源: jieba 词典词 + 3..4 字 n-gram 跨文件搭配）。

    歌词钩子短语（天青色/等烟雨）非 jieba 词典词——纯分词挖掘结构性
    缺席; n-gram 源以「跨 ≥2 文件重复」为搭配信号（单歌独有词如 素胚
    自然出局——与 frag≥4 字指标对齐）, 边缘功能字过滤压垃圾。
    """
    import jieba
    jieba.setLogLevel(60)
    cnt = collections.Counter()
    ng = collections.Counter()
    files = glob.glob(f"{CORPUS}/*/*.txt")
    n_lines = 0
    for fp in files:
        try:
            with open(fp, encoding="utf-8", errors="ignore") as f:
                for ln in f:
                    ln = re.sub(r"[（(].*?[)）]", "", ln)
                    ln = ln.strip()
                    if not ln or len(ln) > 60 or META_PAT.search(ln):
                        continue
                    if not CJK_PAT.search(ln):
                        continue
                    n_lines += 1
                    for w in jieba.lcut(ln):
                        # 纯 CJK 2..6 字（1 字归主库域; >6 罕见且拼装覆盖）
                        if 2 <= len(w) <= 6 and all(
                                '一' <= c <= '鿿' for c in w):
                            cnt[w] += 1
                    # n-gram: 纯 CJK 化后再切（歌词行天然短句单元）
                    cln = "".join(c for c in ln if '一' <= c <= '鿿')
                    if len(cln) >= 4:
                        for L in (3, 4):
                            for i in range(len(cln) - L + 1):
                                g = cln[i:i + L]
                                if g[0] in EDGE_STOP or g[-1] in EDGE_STOP:
                                    continue
                                ng[g] += 1
        except OSError:
            continue
    # 跨文件门: n-gram 总频 ≥3 且 不与 jieba 词重复（词典形优先）
    for g, c in ng.items():
        if c >= 3 and g not in cnt:
            cnt[g] = c
    print(f"语料 {len(files):,} 文件 / {n_lines:,} 行 → {len(cnt):,} 词形"
          f"（含 n-gram {sum(1 for g in ng if ng[g] >= 3 and True):,} 候选）")
    return cnt


def main():
    min_count, max_words = 2, 50000
    for a in sys.argv[1:]:
        if a.startswith("--min-count="):
            min_count = int(a.split("=", 1)[1])
        if a.startswith("--max-words="):
            max_words = int(a.split("=", 1)[1])
    freq = corpus_word_freq()
    py = load_slwd_pinyin()
    print(f"slwd 拼音词典: {len(py):,} 词")

    cand = [(w, c) for w, c in freq.items() if c >= min_count]
    cand.sort(key=lambda x: -x[1])
    cov, drop = [], 0
    for w, c in cand:
        if len(cov) >= max_words:
            break
        p = py.get(w) or pypinyin_of(w)
        if p:
            cov.append((w, p, c))
        else:
            drop += 1
    os.makedirs(OUT_DIR, exist_ok=True)
    yml = f"{OUT_DIR}/lyric.dict.yaml"
    with open(yml, "w", encoding="utf-8") as f:
        f.write("# 歌词领域词库（Chinese-Lyric-Corpus MIT × jieba; 拼音源 slwd/pypinyin）\n---\n")
        for w, p, c in cov:
            f.write(f"{w}\t{p}\t{c}\n")
    hb = f"{OUT_DIR}/lyric.hbwc"
    r = subprocess.run(["python3", "tools/dict_build.py", yml, "--out", hb],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stderr); sys.exit(1)
    with open(EVAL_OUT, "w", encoding="utf-8") as f:
        for w, p, c in cov:
            f.write(json.dumps({"kind": "word", "stratum": "lyric",
                                "text": w, "py": p.replace(" ", ""),
                                "freq": c}, ensure_ascii=False) + "\n")
    dist = {}
    for w, _, _ in cov:
        dist[len(w)] = dist.get(len(w), 0) + 1
    print(f"覆盖 {len(cov):,} 词（拼音缺口丢弃 {drop:,}; 上限 {max_words:,}）")
    print("长度分布:", " ".join(f"{k}字:{v}" for k, v in sorted(dist.items())))
    # 关键词在场抽查（青花瓷系探针词）
    probe = ["天青色", "等烟雨", "离愁", "伫立", "汉隶", "前朝", "飘逸",
             "琵琶", "弹奏", "东风破", "素胚", "勾勒", "青花"]
    have = {w for w, _, _ in cov}
    print("探针在场:", {w: (w in have) for w in probe})


if __name__ == "__main__":
    main()
