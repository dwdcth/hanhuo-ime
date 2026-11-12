#!/usr/bin/env python3
"""export_nnlm_advanced.py —— nlrgpt2（PC 3层 d256 高级版）→ SafeTensors 开放格式

来源 = xnet/models/nlrgpt2/ 已验证链（numpy 复现 cos>0.99999999，
nlrgpt2_numpy_REPORT.md）。槽位表 = nlrgpt2_numpy.py 权威常量
（旧 hanhuo-engine 提取脚本的槽位错位即本表纠正：63=c_attnW 非 ln_1_g）。

开放格式 schema v1（hanhuo-nnlm，metadata.variant = "advanced"）：
  wte [5594,256]  wpe [512,256]  ln_f_g/b [256]  wz [256,64] bz [64]
  blocks.{i}.attn_w [768,256]（预转置 out×in） attn_b [768]
  blocks.{i}.proj_w [256,256] proj_b [256]
  blocks.{i}.ln1_g/b ln2_g/b [256]
  blocks.{i}.fc_w [1024,256] fc_b [1024]  pr2_w [256,1024] pr2_b [256]
  词表内嵌（与 mobile 同 schema，单文件自包含）：
  vocab.bytes U8 = 按 id 序拼接的 token UTF-8
  vocab.offsets I64 = wte 行数+1 个偏移；稀疏 id（4..5593，33 空洞，
  0..3 为保留位）以零长 token 占位，保持「行号 i ↔ offsets[i]..offsets[i+1]」

用法: python3 export_nnlm_advanced.py <输出目录=../../data>
产物: nnlm_advanced_fp32.safetensors / nnlm_advanced_fp16.safetensors（词表内嵌）
数据归腾讯，仅本地个人研究使用，禁止分发。
"""
import os
import pickle
import sys

import numpy as np

XNET = "."
WEIGHTS_NPZ = os.path.join(XNET, "nlrgpt2_weights_fp32.npz")
VOCAB_PKL = "/home/banshan/tmp/hanhuo/cache/nlrgpt2_vocab.pkl"

VOCAB, N_CTX, N_LAYER, N_HEADS, HEAD_DIM, D_MODEL = 5594, 512, 3, 4, 64, 256
EPS, GELU_C = 1e-5, 0.7978845608028654

_WTE, _WPE, _LNF_G, _LNF_B, _WZ, _BZ = 8, 56, 9, 10, 60, 59
_LAYERS = [
    (63, 47, 64, 44, 33, 35, 31, 32, 65, 29, 66, 27),
    (67, 26, 68, 25, 23, 24, 21, 22, 69, 20, 70, 19),
    (71, 18, 72, 17, 15, 16, 13, 14, 73, 12, 74, 11),
]


def build_tensors() -> dict:
    z = np.load(WEIGHTS_NPZ)
    t = {(g, int(k.split(":", 1)[1])): v for g, k in
         ((k.split(":", 1)[0], k) for k in z.files) for v in [z[k]]}
    f32 = lambda k: t[("g0", k)].astype(np.float32)  # noqa: E731

    out = {
        "wte": f32(_WTE),
        "wpe": f32(_WPE),
        "ln_f_g": f32(_LNF_G).reshape(-1),
        "ln_f_b": f32(_LNF_B).reshape(-1),
        "wz": f32(_WZ),
        "bz": f32(_BZ).reshape(-1),
    }
    for li, idxs in enumerate(_LAYERS):
        aw, ab, pw, pb, l1g, l1b, l2g, l2b, fw, fb, p2w, p2b = idxs
        # npz 槽位已是 out×in（x @ W^T 语义）——参考实现的 .T 是它自身约定，此处不转
        out[f"blocks.{li}.attn_w"] = np.ascontiguousarray(f32(aw))  # [768,256]
        out[f"blocks.{li}.attn_b"] = f32(ab).reshape(-1)
        out[f"blocks.{li}.proj_w"] = np.ascontiguousarray(f32(pw))
        out[f"blocks.{li}.proj_b"] = f32(pb).reshape(-1)
        out[f"blocks.{li}.ln1_g"] = f32(l1g).reshape(-1)
        out[f"blocks.{li}.ln1_b"] = f32(l1b).reshape(-1)
        out[f"blocks.{li}.ln2_g"] = f32(l2g).reshape(-1)
        out[f"blocks.{li}.ln2_b"] = f32(l2b).reshape(-1)
        out[f"blocks.{li}.fc_w"] = np.ascontiguousarray(f32(fw))  # [1024,256]
        out[f"blocks.{li}.fc_b"] = f32(fb).reshape(-1)
        out[f"blocks.{li}.pr2_w"] = np.ascontiguousarray(f32(p2w))  # [256,1024]
        out[f"blocks.{li}.pr2_b"] = f32(p2b).reshape(-1)
    return out


def build_vocab_tensors() -> dict:
    """词表内嵌：稀疏 id 用零长 token 占位，行号=offset 索引。"""
    vocab = pickle.load(open(VOCAB_PKL, "rb"))
    by_id = sorted(vocab.items(), key=lambda kv: kv[1])
    vbytes = bytearray()
    offs = np.zeros(VOCAB + 1, dtype=np.int64)
    cur = 0  # 已写出的最大 id+1（未到的 id 一律零长）
    for tok, tid in by_id:
        assert cur <= tid, f"词表 id 乱序: {tid} after {cur}"
        vbytes += tok.encode("utf-8")
        offs[tid + 1] = len(vbytes)
        cur = tid + 1
    for i in range(VOCAB + 1):  # 空洞/保留位前向填充
        offs[i] = max(offs[i], offs[i - 1] if i else 0)
    return {
        "vocab.bytes": np.frombuffer(bytes(vbytes), dtype=np.uint8),
        "vocab.offsets": offs,
    }


def main():
    out_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(__file__), "..", "..", "data")
    os.makedirs(out_dir, exist_ok=True)

    tensors = build_tensors()
    vocab_t = build_vocab_tensors()  # U8/I64 不参与 fp16 转换
    meta = {
        "format": "hanhuo-nnlm", "version": "1", "variant": "advanced",
        "n_layer": str(N_LAYER), "d_model": str(D_MODEL),
        "n_heads": str(N_HEADS), "head_dim": str(HEAD_DIM),
        "vocab_size": str(VOCAB), "n_ctx": str(N_CTX),
        "vocab_tokens": str(int(vocab_t["vocab.offsets"].shape[0] - 1
                                - np.count_nonzero(np.diff(vocab_t["vocab.offsets"]) == 0))),
        "eps": repr(EPS), "gelu_c": repr(GELU_C), "attn_scale": "0.125",
        "gelu": "tanh", "license_note": "数据归腾讯，仅本地研究，禁止分发",
    }

    try:
        from safetensors.numpy import save_file
    except ImportError:
        sys.exit("pip install safetensors numpy")

    for tag, t in (("fp32", tensors), ("fp16", {k: v.astype(np.float16)
                                                for k, v in tensors.items()})):
        path = os.path.join(out_dir, f"nnlm_advanced_{tag}.safetensors")
        save_file(t | vocab_t, path, metadata=meta)
        print(f"{tag} → {path} ({os.path.getsize(path)/1e6:.1f}MB)")

    # 自检：关键形状 + 词表不变式
    assert tensors["wte"].shape == (VOCAB, D_MODEL)
    assert tensors["blocks.0.attn_w"].shape == (3 * D_MODEL, D_MODEL)
    assert tensors["blocks.2.fc_w"].shape == (4 * D_MODEL, D_MODEL)
    assert vocab_t["vocab.offsets"].shape == (VOCAB + 1,)
    assert vocab_t["vocab.offsets"][0] == 0
    assert vocab_t["vocab.offsets"][-1] == len(vocab_t["vocab.bytes"])
    print("形状自检通过")


if __name__ == "__main__":
    main()
