#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""calib_temp.py — 导出侧 ln_f 吸收温度缩放（咨询档 Gemini §一 建议）

tied embedding 下缩放 wte 会连坐输入表示; 但 ln_f 的 γ/β 只作用于
输出投影: γ←γ/T, β←β/T ⇒ logits' = logits/T（逐位等价, Rust 零改动）。
T<1 锐化（拉开搭配边际, 救求和打分下的长句）; T>1 平滑。

用法: python3 tools/calib_temp.py <in.safetensors> <T> <out.safetensors>
"""
import json
import struct
import sys


def main():
    src, t, dst = sys.argv[1], float(sys.argv[2]), sys.argv[3]
    raw = open(src, "rb").read()
    (hlen,) = struct.unpack("<Q", raw[0:8])
    hdr = json.loads(raw[8:8 + hlen])
    blob_off = 8 + hlen

    scaled = {}
    for name in ("ln_f_g", "ln_f_b"):
        e = hdr[name]
        s, en = e["data_offsets"]
        if e["dtype"] != "F32":
            sys.exit(f"{name} 非 F32: {e['dtype']}")
        n = (en - s) // 4
        vals = struct.unpack(f"<{n}f", raw[blob_off + s:blob_off + en])
        scaled[name] = struct.pack(f"<{n}f", *(v / t for v in vals))

    out = bytearray(raw)
    for name, data in scaled.items():
        s = blob_off + hdr[name]["data_offsets"][0]
        out[s:s + len(data)] = data
    # 头不动（改元数据会使 blob 偏移失效）; 温度记入文件名即可
    with open(dst, "wb") as f:
        f.write(out)
    print(f"✓ {dst}: T={t}（ln_f γ/β ÷T; logits≡/T）")


if __name__ == "__main__":
    main()
