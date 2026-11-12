#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""ckpt_export.py — 周期 ckpt → 三格式 safetensors（跳过语料管线）

用法: python3 tools/ckpt_export.py <ckpt> <out前缀> [--int8] [--fp16]
（v2 提前收刀役: 40k 步 ckpt 直接导出, 不再全量跑 179k）
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import torch  # noqa: E402
import train_nnlm as T  # noqa: E402

ck_path, out = sys.argv[1], sys.argv[2]
do_int8 = "--int8" in sys.argv
do_fp16 = "--fp16" in sys.argv

ck = torch.load(ck_path, map_location="cpu", weights_only=False)
a = T.ARCH[ck["variant"]]
for k, v in (("N_LAYER", a["n_layer"]), ("D", a["d"]),
             ("N_HEAD", a["n_head"]), ("HEAD_DIM", a["head_dim"]),
             ("FFN", a["ffn"]), ("CTX", a["ctx"]),
             ("VARIANT", a["variant"]), ("GELU_C", a["gelu_c"]),
             ("GELU_A", a["gelu_a"])):
    setattr(T, k, v)

vocab = ck["vocab"]
m = T.build_torch_model(len(vocab), "cpu")
m.load_state_dict(ck["sd"])
m.eval()
T.export(m, vocab, out, "自训开源权重（c.6）: LCCC/zhwiki/THUCNews/classical/"
                        "poetry 混合 2.2B 字, Apache-2.0")
print(f"✓ {out} (vocab {len(vocab)})")
if do_fp16:
    T.export(m, vocab, out.replace(".safetensors", "_fp16.safetensors"),
             "同上（fp16 盘上档）", fp16=True)
    print("✓ fp16")
if do_int8:
    T.export(m, vocab, out.replace(".safetensors", "_int8.safetensors"),
             "同上（int8 量化档）", int8=True)
    print("✓ int8")
