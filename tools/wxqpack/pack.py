#!/usr/bin/env python3
"""wxq.bin —— 自有 mmap 打包格式（v1）.

设计原则: 全定宽小端整数、绝对偏移、section 目录一次解析、加载零拷贝
（numpy.frombuffer 直接骑在 mmap 上，不搬字节）。

文件布局:
  [0]  magic 'WXQ1' u32 | ver u32=1 | n_sections u32 | reserved u32
  [16] 目录 n × {tag u32(FOURCC), offset u64, length u64}
  sections（8 对齐）:

  SYL   409 音节（NUL 分隔，排序）；SYLW 音节 unigram 权重 u32×409（L1 先验）
  RDAT  DAT: n_slots u32, n_keys u32,
        base i32[n], check i32[n], lo u32[n], cnt u32[n], term bitmap(ceil(n/8))
  RIDX  读音表（读音 id = DAT 叶子序）: n u32, n_words u16[n], off u32[n+1](→WLIST),
        syl blob（每读音 [u8 n_syl][u16 syl_id ×n_syl]，id∈[0,409)）
  WLSI  词 id 串联（每读音按词频降序）u32[]
  WORD  词表（按词文本排序）: n u32, freq_val u16[n]（原厂四段量化 −log10P）,
        flags u8[n], off u32[n+1](→词区), front-coded 词区
        （每词 [u8 共享前缀长][u8 后缀长][后缀 UTF-8]；每 32 词强制独立成检查点）
  WHSH  词→id 开放寻址（FNV-1a 32）：n_slots u32, slot u64[n]（hash<<32|id+1, 0=空）
  HOT   DAT 态（深度≤3）热词表: n u32, states u32[n], off u32[n+1], ids u32[]
  JIAN  简拼: n u32, koff u32[n+1](→键区 front-coded), eoff u32[n+1](→展开区),
        展开区每键 [u16 n][u32 读音id ×n]
  EPYN  纠错: n u32, koff u32[n+1](→键区 front-coded), 值区每键 [u8 n]
        [每纠错: u32 读音id+1, u8 prefix_flag]
  BGRM  bigram（按 w1,w2 排序）: n u32, w1 u32[n], w2 u32[n], val u16[n]
  C4G   英文词对: n_words u32, off u32[n+1], front-coded 池,
        n_pairs u32, w1 u32[n], w2 u32[n], val u16[n]
  NNLM  nlrgptand 权重（f16 原厂精度）+ 词表 + start_cache + BOS 先验
  META  JSON（构建统计/来源/量化自检数字）

数据归腾讯，仅本地个人使用，禁止分发。
"""
from __future__ import annotations

import json
import mmap
import os
import struct
from array import array

import numpy as np

MAGIC = 0x31515857          # 'WXQ1' LE
VERSION = 4                 # v4: RDAT 位打包（读兼容 v1/v2/v3）
CKPT = 32                    # front-code 检查点间隔（记录序 % CKPT == 0 → 独立记录）

T_SYL, T_SYLW, T_RDAT, T_RIDX, T_WLIST, T_WORDS, T_WHASH, T_HOT, T_JIANP, \
    T_EPYN, T_BGRM, T_C4G, T_NNLM, T_META, T_W2R, T_MDLT, T_WFEA = (
        b'SYL ', b'SYLW', b'RDAT', b'RIDX', b'WLSI', b'WORD', b'WHSH',
        b'HOT ', b'JIAN', b'EPYN', b'BGRM', b'C4G ', b'NNLM', b'META',
        b'W2R ', b'MDLT', b'WFEA')


def _pad8(n: int) -> int:
    return (-n) & 7


def fnv1a(data: bytes) -> int:
    h = 0x811C9DC5
    for b in data:
        h ^= b
        h = (h * 0x01000193) & 0xFFFFFFFF
    return h


# ----------------------------------------------------------------------
# front-coded 字符串区（写入 + 随机读）
# ----------------------------------------------------------------------
def fc_encode(strings: list[str]) -> tuple[bytes, array]:
    """返回 (blob, off u32[n+1])。off[i]=记录 i 起始，off[n]=总长。
    每 CKPT 条强制 shared=0（独立检查点，供随机解码）。"""
    out = bytearray()
    off = array('I')
    prev = ''
    for i, s in enumerate(strings):
        if i % CKPT == 0:
            m = 0
        else:
            m = 0
            for a, b in zip(prev, s):
                if a != b:
                    break
                m += 1
        prev = s
        suf = s[m:].encode('utf-8')
        if m > 255 or len(suf) > 255:
            raise ValueError(f'frontcode overflow ({m},{len(suf)}): {s[:40]}')
        off.append(len(out))
        out.append(m)
        out.append(len(suf))
        out += suf
    off.append(len(out))
    return bytes(out), off


class FCStrings:
    """front-coded 随机访问（检查点独立记录 + 有界缓存）"""

    def __init__(self, blob, off):
        self.blob = blob if isinstance(blob, (bytes, bytearray)) else bytes(blob)
        self.off = off
        self.n = len(off) - 1
        self._cache: dict[int, str] = {}
        self._ckpt: dict[int, str] = {}

    def _ckpt_word(self, cp: int) -> str:
        w = self._ckpt.get(cp)
        if w is None:
            p = int(self.off[cp])
            m = self.blob[p]                    # 检查点记录: m==0
            ln = self.blob[p + 1]
            assert m == 0, f'checkpoint {cp} not independent'
            w = self.blob[p + 2:p + 2 + ln].decode('utf-8')
            self._ckpt[cp] = w
        return w

    def get(self, i: int) -> str:
        if i < 0 or i >= self.n:
            raise IndexError(i)
        v = self._cache.get(i)
        if v is not None:
            return v
        cp = i - (i % CKPT)
        w = self._ckpt_word(cp)
        blob, off = self.blob, self.off
        for j in range(cp + 1, i + 1):
            p = int(off[j])
            m = blob[p]
            ln = blob[p + 1]
            w = w[:m] + blob[p + 2:p + 2 + ln].decode('utf-8')
        if len(self._cache) > 65536:
            self._cache.clear()
        self._cache[i] = w
        return w

    def iter_all(self):
        """顺序全解（自检/报告用）"""
        w = ''
        blob, off = self.blob, self.off
        for j in range(self.n):
            p = int(off[j])
            m = blob[p]
            ln = blob[p + 1]
            w = w[:m] + blob[p + 2:p + 2 + ln].decode('utf-8')
            yield w


# ----------------------------------------------------------------------
# v4 位打包（bit p = byte p>>3 的 bit p&7，读侧单 8B 载荷移位掩码即可）
def unpack_bits(buf: 'np.ndarray', w: int, n: int):
    import numpy as np
    bits = np.unpackbits(buf, bitorder='little')[: n * w].reshape(n, w)
    return (bits.astype('<u8') << np.arange(w, dtype='<u8')).sum(axis=1)
def pack_bits(vals, w: int) -> bytes:
    """vals(u32 域非负) → 精确 w bit 流，8 对齐且 ≥8B（读侧单 8B 载荷冗余）"""
    import numpy as np
    v = np.asarray(vals, dtype='<u8')
    assert v.max(initial=0) < (1 << w), 'pack_bits 值溢出位宽'
    bits = ((v[:, None] >> np.arange(w, dtype='<u8')) & 1).astype(np.uint8)
    packed = np.packbits(bits.ravel(), bitorder='little').tobytes()
    n8 = max(16, ((len(packed) + 7) & ~7) + 8)   # 对齐后再 +8（末元素 8B 读窗）
    return packed + b'\x00' * (n8 - len(packed))


def encode_rdat_v4(db, n_keys, cnt16, cnt_ovf) -> bytes:
    """v4 RDAT: [n][n_keys][w_base][w_check][w_lo][rsv] + 位包 base/check/lo
    + cnt16 + 溢出表 + term 位图。位宽=各自值域。"""
    n = db.n_slots
    w_base = (n - 1).bit_length() if n > 1 else 1
    w_lo = (n_keys - 1).bit_length() if n_keys > 1 else 1
    out = bytearray(struct.pack('<IIBBBBI', n, n_keys, w_base, w_base,
                                w_lo, 0, 0))   # 16B 头（8 对齐）
    for arr, w in ((db.base, w_base), (db.check, w_base), (db.lo, w_lo)):
        out += pack_bits(arr, w)
    out += cnt16.tobytes()
    out += struct.pack('<I', len(cnt_ovf))
    for s_, c_ in sorted(cnt_ovf.items()):
        out += struct.pack('<II', s_, c_)
    out += bytes(db._term_bytes)
    assert len(db._term_bytes) == (n + 7) // 8
    return bytes(out)


def encode_mdltr(npz_path: str) -> bytes:
    """cascade/mdltr_lgbm.npz → MDLTR 区。
    [n_trees][n_feat] × tree{feat u8×62, dt i8×62, left/right i32×62,
    thr/leaf f64, cb u32×16, ct u32×n}（mdltr_inference.py 语义忠实）"""
    import numpy as np
    z = np.load(npz_path)
    nt, nf = 131, 179
    out = bytearray(struct.pack('<II', nt, nf))
    for k in range(nt):
        p = f't{k:03d}_'
        feat = z[p + 'feat'].astype('<u1')          # <179 → u8
        dt = z[p + 'dt'].tobytes()
        assert len(feat) == 62 and len(dt) == 62
        out += feat.tobytes() + b'\x00' * 2
        out += dt + b'\x00' * 2 + b'\x00' * 4      # 8 对齐
        out += z[p + 'left'].astype('<i4').tobytes()
        out += z[p + 'right'].astype('<i4').tobytes()
        out += z[p + 'thr'].astype('<f8').tobytes()
        out += z[p + 'leaf'].astype('<f8').tobytes()
        cb = z[p + 'cb'].astype('<u4')
        ct = z[p + 'ct'].astype('<u4')
        out += struct.pack('<I', len(cb)) + cb.tobytes()
        out += struct.pack('<I', len(ct)) + ct.tobytes()
    return bytes(out)


def encode_wfea(tsv_path: str) -> bytes:
    """wtid.tsv (word	11B hex	hash) → WFEA 区。
    [n u32][off u32[n+1] → front-code 词区][feat u8×11×n]"""
    words, feats = [], []
    with open(tsv_path, encoding='utf-8') as f:
        for line in f:
            if line.startswith('#') or 'feature_bytes' in line:
                continue
            p = line.rstrip('\n').split('\t')
            if len(p) < 2 or not p[0]:
                continue
            try:
                b = bytes(int(x, 16) for x in p[1].split())
            except ValueError:
                continue
            if len(b) != 11:
                continue
            words.append(p[0])
            feats.append(b)
    order = sorted(range(len(words)), key=lambda i: words[i])
    words = [words[i] for i in order]
    feats = [feats[i] for i in order]
    fc_blob, fc_off = fc_encode(words)
    out = bytearray(struct.pack('<I', len(words)))
    out += fc_off.tobytes()
    out += fc_blob
    for b in feats:
        out += b
    return bytes(out)


# ----------------------------------------------------------------------
class PackWriter:
    def __init__(self):
        self.sections: list[tuple[bytes, bytes]] = []
        self._tags: set[bytes] = set()

    def add(self, tag: bytes, blob: bytes):
        assert len(tag) == 4 and tag not in self._tags
        self._tags.add(tag)
        self.sections.append((tag, blob))

    def write(self, path: str) -> int:
        """目录: 16B 头 + n × 12B {tag u32, offset u32, length u32}"""
        n = len(self.sections)
        body_start = (16 + n * 12 + 7) & ~7
        offsets = []
        pos = body_start
        for tag, blob in self.sections:
            offsets.append((tag, pos, len(blob)))
            pos += len(blob) + _pad8(len(blob))
        with open(path, 'wb') as f:
            f.write(struct.pack('<IIII', MAGIC, VERSION, n, 0))
            for tag, off, ln in offsets:
                f.write(struct.pack('<4sII', tag, off, ln))
            assert f.tell() <= body_start
            f.write(b'\x00' * (body_start - f.tell()))
            for (tag, off, ln), (_, blob) in zip(offsets, self.sections):
                assert f.tell() == off, (f.tell(), off, tag)
                f.write(blob)
                pad = _pad8(ln)
                if pad:
                    f.write(b'\x00' * pad)
        return os.path.getsize(path)


# ----------------------------------------------------------------------
class Pack:
    """mmap 加载器（零拷贝）：header + 目录解析一次，数据区全部惰性视图。"""

    def __init__(self, path: str):
        self.path = path
        self.f = open(path, 'rb')
        self.mm = mmap.mmap(self.f.fileno(), 0, access=mmap.ACCESS_READ)
        magic, ver, n, _ = struct.unpack_from('<IIII', self.mm, 0)
        assert magic == MAGIC, f'bad magic {magic:#x}'
        assert ver in (1, 2, 3, 4), f'不支持的版本 {ver}'
        self.ver = ver
        self.dir = {}
        for i in range(n):
            tag, off, ln = struct.unpack_from('<4sII', self.mm, 16 + i * 12)
            self.dir[tag] = (off, ln)
        self._cache: dict[bytes, object] = {}

    def close(self):
        self.mm.close()
        self.f.close()

    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.close()

    def raw(self, tag: bytes) -> memoryview:
        off, ln = self.dir[tag]
        return self.mm[off:off + ln]

    def has(self, tag: bytes) -> bool:
        return tag in self.dir

    # ---------------- 各 section 视图 ----------------
    def syllables(self) -> list[str]:
        if T_SYL not in self._cache:
            self._cache[T_SYL] = [s.decode() for s in
                                  bytes(self.raw(T_SYL)).split(b'\x00') if s]
        return self._cache[T_SYL]

    def syllable_weights(self) -> np.ndarray:
        if T_SYLW not in self._cache:
            self._cache[T_SYLW] = np.frombuffer(self.raw(T_SYLW), dtype='<u4')
        return self._cache[T_SYLW]

    def dat(self):
        if T_RDAT not in self._cache:
            mv = self.raw(T_RDAT)
            if self.ver >= 4:
                (n_slots, n_keys, w_base, _w_check, w_lo,
                 _r) = struct.unpack_from('<IIBBBB', mv, 0)
                o = 16

                def take(w):
                    nonlocal o
                    ln = max(16, ((n_slots * w + 7) // 8 + 7) & ~7) + 8
                    blob = np.frombuffer(
                        mv, dtype=np.uint8, count=ln, offset=o)
                    o += ln
                    return unpack_bits(blob, w, n_slots)

                base = take(w_base)
                check = take(w_base).astype(np.int32)
                lo = take(w_lo)
                cnt16 = np.frombuffer(mv, dtype='<u2', count=n_slots,
                                      offset=o)
                o += 2 * n_slots
                n_ovf = struct.unpack_from('<I', mv, o)[0]
                o += 4
                ovf = {}
                for _ in range(n_ovf):
                    s_, c_ = struct.unpack_from('<II', mv, o)
                    ovf[s_] = c_
                    o += 8
                term = np.frombuffer(mv, dtype=np.uint8,
                                     count=(n_slots + 7) // 8, offset=o)
                from .dat import DAT
                self._cache[T_RDAT] = DAT(base, check, lo, cnt16, term,
                                          n_keys, ovf)
                return self._cache[T_RDAT]
            n_slots, n_keys = struct.unpack_from('<II', mv, 0)
            o = 8
            base = np.frombuffer(mv, dtype='<i4', count=n_slots, offset=o)
            o += 4 * n_slots
            check = np.frombuffer(mv, dtype='<i4', count=n_slots, offset=o)
            o += 4 * n_slots
            lo = np.frombuffer(mv, dtype='<u4', count=n_slots, offset=o)
            o += 4 * n_slots
            cnt16 = np.frombuffer(mv, dtype='<u2', count=n_slots, offset=o)
            o += 2 * n_slots
            n_ovf = struct.unpack_from('<I', mv, o)[0]
            o += 4
            ovf = {}
            for _ in range(n_ovf):
                s, c = struct.unpack_from('<II', mv, o)
                ovf[s] = c
                o += 8
            term = np.frombuffer(mv, dtype=np.uint8,
                                 count=(n_slots + 7) // 8, offset=o)
            from .dat import DAT
            self._cache[T_RDAT] = DAT(base, check, lo, cnt16, term, n_keys,
                                      ovf)
        return self._cache[T_RDAT]

    def ridx(self):
        """(n, n_words u16[n], off u32[n+1] → WLSI, soff u32[n+1] → syl, syl blob)"""
        if T_RIDX not in self._cache:
            mv = self.raw(T_RIDX)
            n = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            nw = np.frombuffer(mv, dtype='<u2', count=n, offset=o)
            o += 2 * n
            off = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            soff = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            self._cache[T_RIDX] = (n, nw, off, soff, mv[o:])
        return self._cache[T_RIDX]

    def wlist(self) -> np.ndarray:
        if T_WLIST not in self._cache:
            mv = self.raw(T_WLIST)
            if self.ver == 1:
                self._cache[T_WLIST] = np.frombuffer(mv, dtype='<u4')
            else:
                n = struct.unpack_from('<I', mv, 0)[0]
                self._cache[T_WLIST] = np.frombuffer(
                    mv, dtype='<u4', count=n, offset=4)
        return self._cache[T_WLIST]

    def wlsi_freq(self) -> np.ndarray | None:
        """v2: per-(读音,词) 频次伴列 u16（与 wlist 等长）；v1 → None"""
        if T_WLIST not in self._cache and self.ver < 2:
            return None
        if 'wlsi_fv' not in self._cache:
            mv = self.raw(T_WLIST)
            if self.ver < 2:
                return None
            n = struct.unpack_from('<I', mv, 0)[0]
            self._cache['wlsi_fv'] = np.frombuffer(
                mv, dtype='<u2', count=n, offset=4 + 4 * n)
        return self._cache['wlsi_fv']

    def words(self):
        """(n, freq u16[n], flags u8[n], off u32[n+1], FCStrings)"""
        if T_WORDS not in self._cache:
            mv = self.raw(T_WORDS)
            n = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            freq = np.frombuffer(mv, dtype='<u2', count=n, offset=o)
            o += 2 * n
            flags = np.frombuffer(mv, dtype=np.uint8, count=n, offset=o)
            o += n
            off = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            self._cache[T_WORDS] = (n, freq, flags, off,
                                    FCStrings(mv[o:], off))
        return self._cache[T_WORDS]

    def whash(self) -> tuple[np.ndarray, int]:
        if T_WHASH not in self._cache:
            mv = self.raw(T_WHASH)
            n = struct.unpack_from('<I', mv, 0)[0]
            if self.ver >= 3:   # v3: u32 槽 = id+1（fnv 链 + 词面校验）
                self._cache[T_WHASH] = (
                    np.frombuffer(mv, dtype='<u4', count=n, offset=4), n)
            else:               # v1/v2: u64 槽 = hash<<32|id+1
                self._cache[T_WHASH] = (
                    np.frombuffer(mv, dtype='<u8', count=n, offset=4), n)
        return self._cache[T_WHASH]

    def hot(self):
        """(states u32[n] 升序, off u32[n+1], ids u32[])"""
        if T_HOT not in self._cache:
            mv = self.raw(T_HOT)
            n = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            states = np.frombuffer(mv, dtype='<u4', count=n, offset=o)
            o += 4 * n
            off = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            self._cache[T_HOT] = (states, off,
                                  np.frombuffer(mv, dtype='<u4', offset=o))
        return self._cache[T_HOT]

    def jianpin(self):
        """(n, koff u32[n+1], FCStrings 键, eoff u32[n+1], 展开区 bytes)"""
        if T_JIANP not in self._cache:
            mv = self.raw(T_JIANP)
            n = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            koff = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            kend = o + int(koff[n])
            keys = FCStrings(mv[o:kend], koff)
            o = kend
            eoff = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            self._cache[T_JIANP] = (n, koff, keys, eoff, mv[o:])
        return self._cache[T_JIANP]

    def editpinyin(self):
        """(n, koff, FCStrings 键, 值区 bytes)"""
        if T_EPYN not in self._cache:
            mv = self.raw(T_EPYN)
            n = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            koff = np.frombuffer(mv, dtype='<u4', count=n + 1, offset=o)
            o += 4 * (n + 1)
            kend = o + int(koff[n])
            keys = FCStrings(mv[o:kend], koff)
            self._cache[T_EPYN] = (n, koff, keys, mv[kend:])
        return self._cache[T_EPYN]

    def bigram(self):
        """(n, w1 u32[n] 升序, w2 u32[n], val u16[n])"""
        if T_BGRM not in self._cache:
            mv = self.raw(T_BGRM)
            n = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            w1 = np.frombuffer(mv, dtype='<u4', count=n, offset=o)
            o += 4 * n
            w2 = np.frombuffer(mv, dtype='<u4', count=n, offset=o)
            o += 4 * n
            val = np.frombuffer(mv, dtype='<u2', count=n, offset=o)
            self._cache[T_BGRM] = (n, w1, w2, val)
        return self._cache[T_BGRM]

    def c4g(self):
        """(n_words, off, FCStrings 池, n_pairs, w1, w2, val)"""
        if T_C4G not in self._cache:
            mv = self.raw(T_C4G)
            nw = struct.unpack_from('<I', mv, 0)[0]
            o = 4
            off = np.frombuffer(mv, dtype='<u4', count=nw + 1, offset=o)
            o += 4 * (nw + 1)
            pend = o + int(off[nw])
            pool = FCStrings(mv[o:pend], off)
            o = pend
            npair = struct.unpack_from('<I', mv, o)[0]
            o += 4
            w1 = np.frombuffer(mv, dtype='<u4', count=npair, offset=o)
            o += 4 * npair
            w2 = np.frombuffer(mv, dtype='<u4', count=npair, offset=o)
            o += 4 * npair
            val = np.frombuffer(mv, dtype='<u2', count=npair, offset=o)
            self._cache[T_C4G] = (nw, off, pool, npair, w1, w2, val)
        return self._cache[T_C4G]

    def meta(self) -> dict:
        if T_META not in self._cache:
            self._cache[T_META] = json.loads(bytes(self.raw(T_META)))
        return self._cache[T_META]
