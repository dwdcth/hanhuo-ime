#!/usr/bin/env python3
"""open_build.py —— 汉火开源构建：rime-ice 开源词典 → wxq.bin v2.

数据源（全部开源，/usr/share/rime-data/cn_dicts/，rime-ice 雾凇拼音）:
  8105.dict.yaml    通用规范汉字 + 25亿字语料字频
  41448.dict.yaml   扩展汉字（Unihan+汉典注音）
  base.dict.yaml    词组 55 万（华宇野风/THUOCL/现代汉语常用词表）
  ext.dict.yaml     三字以上词 34 万（社区注音）
  others.dict.yaml  叠字
  ✗ tencent.dict.yaml 有意排除（腾讯词源，红线）

产出（与 wxq v2 逐 section 兼容，Rust/Python 双引擎即读即用）:
  SYL/SYLW/RDAT/RIDX/WLSI(v2 per-pair 频次)/WORD/WHSH/HOT/JIAN(自动生成)/
  EPYN(空)/BGRM(空)/C4G(空)/META —— 无 NNLM（引擎 nn 路径自动 no-op）

用法: python3 tools/open_build.py [--out PATH]
词库数据许可: rime-ice GPL-3.0（分发时需随附许可与来源声明）
"""
from __future__ import annotations

import json
import math
import os
import struct
import sys
from array import array
from collections import defaultdict

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# 开源构建工具链: tools/wxqpack（自研 PackWriter/DAT/量化——无外部依赖）
sys.path.insert(0, os.path.join(ROOT_DIR, 'tools', 'wxqpack'))

from pack import (PackWriter, fc_encode, fnv1a,  # noqa: E402
                      T_BGRM, T_C4G, T_EPYN, T_HOT, T_JIANP, T_META, T_RIDX,
                      T_RDAT, T_SYL, T_SYLW, T_WHASH, T_W2R, T_WORDS, T_WLIST)
from dat import DATBuilder  # noqa: E402
from quant import freq_to_val  # noqa: E402

# cn_dicts 源: 仓内钉版副本优先（data/rimeice_cn/, GPL-3.0, LFS——
# 上游 main 会漂移致行为锁失效）; 外部目录为研究线回退
for CN in [os.path.join(ROOT_DIR, 'data', 'rimeice_cn'),
           os.path.expanduser('~/tmp/rimeice_cn'),
           os.path.expanduser('~/.local/share/rime-data/cn_dicts')]:
    if os.path.isdir(CN):
        break
else:
    raise SystemExit('cn_dicts 未找到（rime-ice 源）')
SOURCES = ['8105', '41448', 'base', 'ext', 'others']   # 有意排除 tencent
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..',
                   'open', 'hanhuo_open.bin')
HOT_DEPTH = 3
HOT_K = 16
JIAN_MAX_SYL = 6
JIAN_MAX_EXP = 64


def log(msg: str):
    print(f'[{os.times().elapsed:7.1f}s] {msg}', flush=True)


def parse_yaml_dict(path: str):
    """rime dict.yaml → [(word, 拼音串, freq)]；解析正文三列/两列行"""
    out = []
    with open(path, encoding='utf-8') as f:
        in_body = False
        for line in f:
            if not in_body:
                if line.rstrip('\n') == '...':
                    in_body = True
                continue
            line = line.split('#', 1)[0].rstrip()
            if not line or line.startswith('\t'):
                continue
            parts = line.split('\t')
            if len(parts) >= 3:
                w, py, fq = parts[0], parts[1], parts[2]
            elif len(parts) == 2:
                w, py, fq = parts[0], parts[1], '1'
            else:
                continue
            w = w.strip()
            py = py.strip().replace('ü', 'v').replace('V', 'v')
            try:
                freq = int(fq)
            except ValueError:
                continue
            if not w or freq < 0:
                continue
            out.append((w, py, freq))
    return out


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', default=os.path.expanduser(OUT))
    ap.add_argument('--max-syl', type=int, default=4,
                    help='A 长尾剪枝：音节数超此值的词不进索引（DP 可组合），'
                         '0=不剪')
    a = ap.parse_args()

    # ---------- 1. 词料装载 ----------
    per_source = {}
    for name in SOURCES:
        path = os.path.join(CN, f'{name}.dict.yaml')
        rows = parse_yaml_dict(path)
        total = sum(f for _, _, f in rows) or 1
        per_source[name] = (rows, total)
        log(f'{name}: {len(rows)} 条 / 频次总量 {total}')
    # 跨源归一化律（2026-11-13 sm→什么 案定谳）: fv 必须用全局 total
    # ——各源 total 差 6 个量级（base 5.6e9 vs others 1.9e3）, 按源归一
    # 时缺频次源（缺省 freq=1）的词 P 虚大 → fv 系统性偏小（假高频）,
    # 污染简拼键 rid 排序（others 杉木 P=1/1941 压过 base 什么
    # 502468/5.6e9）与一切跨词 fv 比较。
    GLOBAL_TOTAL = sum(t for _, t in per_source.values())

    # ---------- 2. 主表 entries[(word, syl_tuple)] = fv（跨源取最高频） ----------
    entries: dict[tuple[str, tuple[str, ...]], int] = {}
    raw_entries: dict[tuple[str, tuple[str, ...]], int] = {}
    bad = 0
    for name in SOURCES:
        rows, total = per_source[name]
        for w, py, freq in rows:
            syls = tuple(py.split())
            if not syls or any(
                    not (s.isascii() and s.isalpha() and s.islower())
                    for s in syls):
                bad += 1
                continue
            if a.max_syl and len(syls) > a.max_syl:
                continue
            fv = freq_to_val(freq, GLOBAL_TOTAL)
            k = (w, syls)
            cur = entries.get(k)
            if cur is None or fv < cur:
                entries[k] = fv
            # 原始频次平行表（简拼键排序 tie-break 用——量化 val 同档的
            # 词（什么 502468/上面 500494 → 同 50221）按原始频次分序）
            cur_r = raw_entries.get(k)
            if cur_r is None or freq > cur_r:
                raw_entries[k] = freq
    log(f'主表条目: {len(entries)}（坏注音 {bad}）')
    # ---------- 2.5 占位词估频（WowType 律吸收, 2026-11-14; MIT/Apache
    # 参考 Gloridust/WowType lexicon.rs estimate_placeholder_freqs）:
    # rime-ice 占位频次（ext 源 33.9 万词同值 100）按拼音切成已有词
    # DP 最小代价、部分概率相乘 ×成词加成 e^3≈20, 上限=最冷部分
    # 频次, 只升不降——中低频 top1 95.5→99.0 同款。raw 频次域做
    # （entries 是量化 val 域——方向相反, 不可混用）。
    PLACEHOLDER_BONUS = 3.0   # 成词加成（ln 域 ×20）
    PLACEHOLDER_MAX_PART = 4  # 段长上限
    from collections import Counter as _C
    multi_keys = [k for k in raw_entries if len(k[1]) >= 2]
    hist = _C(raw_entries[k] for k in multi_keys)
    (placeholder, pcount), = hist.most_common(1)
    if pcount >= 1000 and pcount * 4 >= len(multi_keys):
        total0 = sum(raw_entries.values()) or 1.0
        # 音节列→(词, 原始频次) 索引（估频切词查频; 同键取最高频）
        syl_freq = {}
        for (w, t), fq in raw_entries.items():
            cur = syl_freq.get(t)
            if cur is None or cur[1] < fq:
                syl_freq[t] = (w, fq)
        n_up = 0
        for (w, t), fq in list(raw_entries.items()):
            if fq != placeholder or len(t) < 2:
                continue
            chars = list(w)
            n = len(chars)
            if n != len(t):
                continue  # 字音节不对齐（儿化等）
            INF = float('inf')
            best = [INF] * (n + 1); best[0] = 0.0
            rarest = [INF] * (n + 1)
            for i in range(n):
                if best[i] == INF:
                    continue
                for j in range(i + 1, min(n, i + PLACEHOLDER_MAX_PART) + 1):
                    if j - i == n:
                        continue  # 不能用自己
                    hit = syl_freq.get(t[i:j])
                    if not hit:
                        continue
                    pf = hit[1]
                    if pf <= 0:
                        continue
                    cost = best[i] - math.log(pf / total0)
                    if cost < best[j]:
                        best[j] = cost
                        rarest[j] = min(rarest[i], pf)
            if best[n] == INF:
                continue
            est = min(total0 * math.exp(PLACEHOLDER_BONUS - best[n]),
                      rarest[n])
            if est > fq:
                raw_entries[(w, t)] = est
                n_up += 1
        log(f'[估频] 占位值 {placeholder}（{pcount} 词）→ {n_up} 词升频')
    else:
        log(f'[估频] 众数 {placeholder}（{pcount}/{len(multi_keys)}）未达门槛, 跳过')
    words_set = {w for (w, _) in entries}
    words = sorted(words_set)
    wid = {w: i for i, w in enumerate(words)}
    n_words = len(words)
    log(f'词表: {n_words}')

    freq_arr = array('H', [51742]) * n_words   # VMAX（词级兜底=最低频）
    for (w, _t), fv in entries.items():
        i = wid[w]
        if fv < freq_arr[i]:
            freq_arr[i] = fv

    # ---------- 3. 读音表（WLSI v2: per-pair 频次） ----------
    rmap: dict[str, dict[tuple, dict[int, int]]] = defaultdict(dict)
    for (w, t), fv in entries.items():
        key = ''.join(t)
        rmap[key].setdefault(t, {})[wid[w]] = fv
    keys = sorted(rmap)
    n_readings = len(keys)
    log(f'读音键: {n_readings}')

    syl_set = sorted({s for t in rmap_variant_iter(rmap) for s in t})
    syl_id = {s: i for i, s in enumerate(syl_set)}
    log(f'音节表: {len(syl_set)}')

    ridx_n = array('H')
    ridx_off = array('I', [0])
    soff = array('I', [0])
    syl_blob = bytearray()
    wlist = array('I')
    wlsi_fv = array('H')
    best_fv_by_reading: dict[str, int] = {}
    for key in keys:
        pairs: list[tuple[int, int]] = []
        for t, wd in rmap[key].items():
            pairs.extend((fv, i) for i, fv in wd.items())
        pairs.sort()
        ridx_n.append(len(pairs))
        for fv, i in pairs:
            wlist.append(i)
            wlsi_fv.append(fv)
        ridx_off.append(len(wlist))
        best_fv_by_reading[key] = pairs[0][0]
        variants = sorted(rmap[key].keys())
        syl_blob.append(len(variants))
        for t in variants:
            syl_blob.append(len(t))
            for sy in t:
                syl_blob.extend(array('H', [syl_id[sy]]).tobytes())
        soff.append(len(syl_blob))
    log(f'WLSI: {len(wlist)} 词引用')

    # SYLW: 音节 unigram 权重（单字频次聚合；>>10 缩放保总量 < 2^32）
    sylw = array('I', bytes(4 * len(syl_set)))
    for (w, t), fv in entries.items():
        if len(w) == 1:
            n = val_to_freq_units(fv) >> 10
            for s in t:
                sylw[syl_id[s]] += n
    for i in range(len(syl_set)):
        if sylw[i] == 0:
            sylw[i] = 1

    # ---------- 4. DAT ----------
    db = DATBuilder()
    db.build(keys)
    log(f'DAT: {db.n_slots} slots / {n_readings} 键')

    # ---------- 5. HOT ----------
    import heapq
    from dat import DAT
    _c16, _ovf = db.cnt_split()
    dat = DAT(db.base, db.check, db.lo, _c16, db._term_bytes, n_readings,
              _ovf)
    hot_states = array('I')
    hot_off = array('I', [0])
    hot_ids = array('I')
    for st in dat.iter_states_upto(HOT_DEPTH):
        lo, cnt = dat.range(st)
        if cnt == 0:
            continue
        heap = []
        for r in range(lo, lo + cnt):
            o0, o1 = int(ridx_off[r]), int(ridx_off[r + 1])
            if o1 > o0:
                heap.append((int(wlsi_fv[o0]), r, o0, o1))
        heapq.heapify(heap)
        top = []
        while heap and len(top) < HOT_K:
            fv, r, oi, o1 = heapq.heappop(heap)
            top.append(int(wlist[oi]))
            if oi + 1 < o1:
                heapq.heappush(heap, (int(wlsi_fv[oi + 1]), r, oi + 1, o1))
        if top:
            hot_states.append(st)
            hot_ids.extend(top)
            hot_off.append(len(hot_ids))
    log(f'HOT: {len(hot_states)} 状态')

    # ---------- 6. WHASH ----------
    n_slots = 1
    while n_slots < int(n_words * 1.3):
        n_slots <<= 1
    slots = array('I', bytes(4 * n_slots))   # v3: 槽=id+1
    mask = n_slots - 1
    for i, w in enumerate(words):
        h = fnv1a(w.encode('utf-8'))
        j = h & mask
        while slots[j] != 0:
            j = (j + 1) & mask
        slots[j] = i + 1
    log(f'WHASH: {n_slots} 槽')

    # ---------- 7. 简拼（从读音自动生成, v3: rid 引用） ----------
    # 键序律: (量化 fv, −原始频次)——fv 同档 tie 时原始频次降序
    # （什么 502468 > 上面 500494 → sm 键 shen'me rid 居首）。
    best_raw_by_reading: dict[str, int] = {}
    for (_w, t), fr in raw_entries.items():
        key = ''.join(t)
        cur = best_raw_by_reading.get(key)
        if cur is None or fr > cur:
            best_raw_by_reading[key] = fr
    jmap: dict[str, dict[int, int]] = defaultdict(dict)  # jk → rid → fv
    for rid, key in enumerate(keys):
        for t in rmap[key].keys():
            if 2 <= len(t) <= JIAN_MAX_SYL:
                jk = ''.join(s[0] for s in t)
                fv = best_fv_by_reading[key]
                d = jmap[jk]
                if rid not in d or fv < d[rid]:
                    d[rid] = fv
    jkeys = sorted(jmap)
    jkoff = array('I')
    jblob = bytearray()
    jeoff = array('I')
    je = bytearray()
    for jk in jkeys:
        exps = sorted(
            jmap[jk].items(),
            key=lambda x: (x[1], -best_raw_by_reading.get(
                keys[x[0]], 0)),
        )[:JIAN_MAX_EXP]
        jkoff.append(len(jblob))
        b = jk.encode()
        jblob.append(0)
        jblob.append(len(b))
        jblob.extend(b)
        jeoff.append(len(je))
        je.extend(array('H', [len(exps)]).tobytes())
        for rid, _fv in exps:
            je.extend(array('I', [rid]).tobytes())
    jkoff.append(len(jblob))
    jeoff.append(len(je))
    log(f'JIANPIN: {len(jkeys)} 键（自动生成, rid 引用）')

    # ---------- 8. 组包 ----------
    w = PackWriter()
    w.add(T_SYL, b'\x00'.join(s.encode() for s in syl_set) + b'\x00')
    w.add(T_SYLW, sylw.tobytes())

    cnt16, cnt_ovf = db.cnt_split()
    from pack import encode_rdat_v4
    w.add(T_RDAT, encode_rdat_v4(db, n_readings, cnt16, cnt_ovf))

    # W2R: 词 → 首读音（rid+1）
    w2r = array('I', bytes(4 * n_words))
    for rid, key in enumerate(keys):
        for t, wd in rmap[key].items():
            for wi in wd:
                if w2r[wi] == 0:
                    w2r[wi] = rid + 1
    ridx = bytearray()
    ridx.extend(array('I', [n_readings]).tobytes())
    ridx.extend(ridx_n.tobytes())
    ridx.extend(ridx_off.tobytes())
    ridx.extend(soff.tobytes())
    ridx.extend(bytes(syl_blob))
    w.add(T_RIDX, bytes(ridx))
    wlsi_sec = bytearray()
    wlsi_sec.extend(array('I', [len(wlist)]).tobytes())
    wlsi_sec.extend(wlist.tobytes())
    wlsi_sec.extend(wlsi_fv.tobytes())
    w.add(T_WLIST, bytes(wlsi_sec))
    w.add(T_W2R, w2r.tobytes())

    words_sec = bytearray()
    words_sec.extend(array('I', [n_words]).tobytes())
    words_sec.extend(freq_arr.tobytes())
    words_sec.extend(bytes(n_words))            # flags 全 0
    fc_blob, fc_off = fc_encode(words)
    words_sec.extend(fc_off.tobytes())
    words_sec.extend(fc_blob)
    w.add(T_WORDS, bytes(words_sec))

    wh = bytearray()
    wh.extend(array('I', [n_slots]).tobytes())
    wh.extend(slots.tobytes())
    w.add(T_WHASH, bytes(wh))

    hot = bytearray()
    hot.extend(array('I', [len(hot_states)]).tobytes())
    hot.extend(hot_states.tobytes())
    hot.extend(hot_off.tobytes())
    hot.extend(hot_ids.tobytes())
    w.add(T_HOT, bytes(hot))

    jp = bytearray()
    jp.extend(array('I', [len(jkeys)]).tobytes())
    jp.extend(jkoff.tobytes())
    jp.extend(bytes(jblob))
    jp.extend(jeoff.tobytes())
    jp.extend(bytes(je))
    w.add(T_JIANP, bytes(jp))

    ep = bytearray()
    ep.extend(array('I', [0]).tobytes())
    ep.extend(array('I', [0]).tobytes())
    w.add(T_EPYN, bytes(ep))

    # BGRM bigram（data/bgrm_open.safetensors 在场即真表——中文联想
    # bigram_after 消费; 缺席空占位。380 万对 LCCC jieba, Apache-2.0）
    bg = bytearray()
    bgrm_st = os.path.join(ROOT_DIR, 'data', 'bgrm_open.safetensors')
    if os.path.exists(bgrm_st):
        import struct as _st
        raw_st = open(bgrm_st, 'rb').read()
        (hl,) = _st.unpack('<Q', raw_st[:8])
        hdr = json.loads(raw_st[8:8+hl])
        blob = 8 + hl
        def _ts(name, fmt, wid):
            m = hdr[name]
            o0, o1 = blob + m['data_offsets'][0], blob + m['data_offsets'][1]
            a = array(fmt); a.frombytes(raw_st[o0:o1])
            if sys.byteorder == 'big':
                a.byteswap()
            return a
        bw1 = _ts('w1', 'I', 4)
        bw2 = _ts('w2', 'I', 4)
        bval = _ts('val', 'H', 2)
        n = min(len(bw1), len(bw2), len(bval))
        bg.extend(array('I', [n]).tobytes())
        bg.extend(bw1[:n].tobytes())
        bg.extend(bw2[:n].tobytes())
        bg.extend(bval[:n].tobytes())
        log(f'[bgrm] 真表: {n} 对')
    else:
        bg.extend(array('I', [0]).tobytes())
    w.add(T_BGRM, bytes(bg))

    # c4g 词对预测（终审 13.5 缺口②: data/c4g_open.tsv 在场即真表,
    # 缺席=空占位——wxq v2 逐位兼容）
    c4 = bytearray()
    c4g_tsv = os.path.join(ROOT_DIR, 'data', 'c4g_open.tsv')
    if os.path.exists(c4g_tsv):
        _BASE, _B1, _B2, _B3, _VMAX = 47104, 49152, 50176, 51200, 51742
        def log10p_to_val(lp):
            n = max(0, int(round(-lp * 1024)))
            if n < 2048:
                v = _BASE + n
            elif n < 4096:
                v = _B1 + (n - 2048) // 2
            elif n < 8192:
                v = _B2 + (n - 4096) // 4
            else:
                v = _B3 + (n - 8192) // 8
            return max(0, min(int(v), _VMAX))
        pool = sorted({l.split('\t')[0] for l in open(c4g_tsv, encoding='utf-8')
                       if l and not l.startswith('#')}
                      | {l.split('\t')[1] for l in open(c4g_tsv, encoding='utf-8')
                         if l and not l.startswith('#')})
        pidx = {w: i for i, w in enumerate(pool)}
        plist = []
        for l in open(c4g_tsv, encoding='utf-8'):
            if l.startswith('#') or not l.strip():
                continue
            w1, w2, lp = l.split('\t')
            plist.append((pidx[w1], pidx[w2], log10p_to_val(float(lp))))
        plist.sort()  # w1 主序（引擎 searchsorted 消费律）
        c4.extend(array('I', [len(pool)]).tobytes())
        fb, fo = fc_encode(pool)
        c4.extend(fo.tobytes())
        c4.extend(fb)
        c4.extend(array('I', [len(plist)]).tobytes())
        c4.extend(array('I', [p[0] for p in plist]).tobytes())
        c4.extend(array('I', [p[1] for p in plist]).tobytes())
        c4.extend(array('H', [p[2] for p in plist]).tobytes())
        log(f'[c4g] 真表: 词 {len(pool)} / 对 {len(plist)}')
    else:
        c4.extend(array('I', [0]).tobytes())
        c4.extend(array('I', [0]).tobytes())
        c4.extend(array('I', [0]).tobytes())
    w.add(T_C4G, bytes(c4))

    meta = {
        'source': 'rime-ice (GPL-3.0): 8105/41448/base/ext/others',
        'excluded': ['tencent.dict.yaml'],
        'n_words': n_words, 'n_readings': n_readings,
        'n_wlsi': len(wlist), 'n_syl': len(syl_set),
        'n_jianpin': len(jkeys), 'n_hot': len(hot_states),
        'open_build': True,
    }
    w.add(T_META, json.dumps(meta, ensure_ascii=False).encode())
    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    size = w.write(a.out)
    log(f'写入 {a.out}: {size / 1e6:.1f} MB')
    print(json.dumps(meta, ensure_ascii=False, indent=1))


def rmap_variant_iter(rmap):
    for variants in rmap.values():
        yield from variants.keys()


def val_to_freq_units(fv: int) -> int:
    """fv → 粗略频次量级（SYLW 聚合用；只需相对大小，封顶防 u32 溢出）"""
    from quant import val_to_n
    n = val_to_n(fv)
    if n <= 0:
        return 1000000
    return max(1, min(int(10 ** (12 - n / 1024)), 1000000))


if __name__ == '__main__':
    main()
