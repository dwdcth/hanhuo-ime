#!/usr/bin/env python3
"""export_bgrm.py —— wxq.bin BGRM 段 → SafeTensors 开放格式（hanhuo-bgrm v1）

BGRM = 词 bigram 统计语言模型（[n][w1×n u32][w2×n u32][val×n u16] 段体，
w1/w2 = 本包词层词 id——与词层强耦合, 注入文件必须同源 wxq）。
非神经模型, 整型表天然位精确——无 fp16 双精度变体（对照 nnlm/mdltr）。

schema v1（metadata.format = "hanhuo-bgrm"）:
  w1  U32 [n]  前词 id（有序——searchsorted 左界查询）
  w2  U32 [n]  后词 id
  val U16 [n]  bigram lift 量化值

用法: python3 export_bgrm.py [in=wxq.bin] [out=../../data/bgrm.safetensors]
产物位精确可验: into_native() == 原段体逐字节。
数据归腾讯，仅本地个人研究使用，禁止分发。
"""
import os, struct, sys

import numpy as np
from safetensors.numpy import save_file

src = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser('~/tmp/hanhuo/wxq.bin')
dst = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), '../../data/bgrm.safetensors')

data = open(src, 'rb').read()
magic, ver, n_sec, _ = struct.unpack_from('<IIII', data, 0)
bg_off = bg_len = None
for i in range(n_sec):
    tag, off, ln = struct.unpack_from('<4sII', data, 16 + i * 12)
    if tag == b'BGRM':
        bg_off, bg_len = off, ln
        break
if bg_off is None:
    sys.exit(f"{src} 无 BGRM 段")

bg = data[bg_off:bg_off + bg_len]
n = struct.unpack_from('<I', bg, 0)[0]
w1 = np.frombuffer(bg, dtype='<u4', count=n, offset=4)
w2 = np.frombuffer(bg, dtype='<u4', count=n, offset=4 + 4 * n)
val = np.frombuffer(bg, dtype='<u2', count=n, offset=4 + 8 * n)

save_file(
    {'w1': w1.copy(), 'w2': w2.copy(), 'val': val.copy()},
    dst,
    metadata={'format': 'hanhuo-bgrm', 'version': '1', 'n': str(n),
              'source': 'wxq.bin BGRM section'},
)
print(f"BGRM {n:,} 对 ({bg_len/1e6:.1f}MB 段体) → {dst} ({os.path.getsize(dst)/1e6:.1f}MB)")
