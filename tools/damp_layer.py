#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""damp_layer.py — 指定块的残差写回投影按 α 缩放（咨询二 Gemini §二.1B）

Pre-LN 纯残差结构: 把 Block_i 的 pr2(MLP 写回)与 proj(attn 写回)
乘 α 可在推理期直接衰减该块贡献——零重训验证「第 3 层副作用」。
α=0 即整块旁路。导出权重补丁, Schema/Rust 零改动。

用法: python3 tools/damp_layer.py <in.safetensors> <layer> <alpha> <out>
      layer=-1 表示最后一块; 仅缩放 MLP(pr2)加 --mlp-only
"""
import json
import struct
import sys


def main():
    src, layer, alpha, dst = (
        sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), sys.argv[4])
    mlp_only = "--mlp-only" in sys.argv
    raw = open(src, "rb").read()
    (hlen,) = struct.unpack("<Q", raw[0:8])
    hdr = json.loads(raw[8:8 + hlen])
    blob = 8 + hlen

    n_layer = int(hdr["__metadata__"]["n_layer"])
    li = layer if layer >= 0 else n_layer + layer
    targets = [f"blocks.{li}.pr2_w", f"blocks.{li}.pr2_b"]
    if not mlp_only:
        targets += [f"blocks.{li}.proj_w", f"blocks.{li}.proj_b"]

    out = bytearray(raw)
    for name in targets:
        e = hdr[name]
        s, en = e["data_offsets"]
        n = (en - s) // 4
        vals = struct.unpack(f"<{n}f", raw[blob + s:blob + en])
        out[blob + s:blob + en] = struct.pack(
            f"<{n}f", *(v * alpha for v in vals))
    with open(dst, "wb") as f:
        f.write(out)
    kind = "MLP(pr2)" if mlp_only else "attn+MLP(proj+pr2)"
    print(f"✓ {dst}: Block{li} {kind} ×{alpha}")


if __name__ == "__main__":
    main()
