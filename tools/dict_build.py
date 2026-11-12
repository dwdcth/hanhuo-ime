#!/usr/bin/env python3
"""dict_build.py — rime .dict.yaml → HBWC v2 领域词库转换器

rime 社区词库生态（医学/计算机/法律…）→ 汉火多词库格式（--dict 挂载）。
yaml 正文行 word\\t音节(空格分隔)\\tfreq → HBWC(word, 撇号拼音, weight)。
布局与 hanhuo-dict/src/bwcn.rs to_hbwc 逐字节镜像。
用法（Rust 主链, 部署无 python 依赖, 产出逐字节相同）:
  hanhuo dict-build <in.dict.yaml> --out <domain.hbwc>
python 参考实现保留（跨语言字节镜像对拍用）:
  python3 tools/dict_build.py <in.dict.yaml> --out <domain.hbwc>
词库许可随源（rime 生态多 GPL——分发需随附来源与许可声明）。

布局锁契约: 改本文件布局代码后必须再生 fixture 并跑锁定测试——
  python3 tools/dict_build.py /tmp/fixture.dict.yaml --out \
    crates/hanhuo-dict/tests/fixtures/domain_fixture.hbwc
  cargo test -p hanhuo-dict --test hbwc_fixture_test
（2026-11-08 漂移事故: key_head 曾写 14B 致 from_hbwc panic——fixture 把
 python writer 与 rust reader 钉死在同一字节镜像上）
"""
import os, struct, sys

src = sys.argv[1]
out = 'domain.hbwc'
if '--out' in sys.argv:
    out = sys.argv[sys.argv.index('--out') + 1]

km = {}  # key(无撇号拼音) -> [(word, pinyin, freq)]
n = 0
for ln in open(src, encoding='utf-8'):
    ln = ln.rstrip('\n')
    if ln.startswith(('#', '...', '---')) or '\t' not in ln:
        continue
    f = ln.split('\t')
    if len(f) < 2:
        continue
    word, pys = f[0], f[1].split()
    if not word or not pys:
        continue
    if not all(p.isascii() and p.isalpha() and p.islower() for p in pys):
        continue
    freq = int(f[2]) if len(f) > 2 and f[2].isdigit() else 1
    km.setdefault(''.join(pys), []).append((word, "'".join(pys), freq))
    n += 1

keys = sorted(km)
# HBWC v2（2026-11-10 瘦身, 与 bwcn.rs to_hbwc v2 逐字节镜像）:
# 去盘上 key_blob/key_idx（键 = 拼音去撇号装载端推导）; key_head
# 8B（first 前缀和装载端重建）; entries 8B（weight u16 饱和, 频序
# 保序——饱和单调）。
key_head, entries, str_blob = [], [], bytearray()
n_entries = 0
for k in keys:
    phs = sorted(km[k], key=lambda r: -r[2])
    py = phs[0][1]
    py_off = len(str_blob)
    str_blob += py.encode()
    key_head.append((len(phs), py_off, len(py)))
    for word, _, freq in phs:
        off = len(str_blob)
        str_blob += word.encode()
        entries.append((off, len(word.encode()), min(freq, 0xFFFF)))
        n_entries += 1

outb = bytearray()
outb += b'HBWC' + bytes([2, 0, 0, 0])
outb += struct.pack('<II', n_entries, len(keys))
for c, po, pl in key_head:
    outb += struct.pack('<HIH', c, po, pl)
for o, l, w in entries:
    outb += struct.pack('<IHH', o, l, w)
outb += str_blob
open(out, 'wb').write(outb)
print(f"{src}: {n} 词 {len(keys)} 键 → {out} ({os.path.getsize(out)/1e3:.1f}KB, HBWC v2)")
