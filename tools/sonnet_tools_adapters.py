"""adapters.py — 把你们的模型接到评测脚本上。只需要改 load_user。

脚本通过 --loader adapters:load_user --ckpt path/to/ckpt.pt 调用，
函数签名固定为 fn(ckpt, device) -> Scorer。

三种接法，任选其一:
  A) PyTorch 因果 LM  → 用 TorchScorer 包一下 forward（最常用，见下方模板）
  B) Rust 引擎         → 自己写一个 Scorer 子类，在 logprobs(seqs) 里调引擎
                         （subprocess / FFI / HTTP 都行），返回逐 token lnP。
                         这样评测口径与线上完全一致，推荐最终验收用它。
  C) 冒烟测试          → load_bigram，纯 numpy，不需要任何模型。
"""
from __future__ import annotations

import json  # noqa: F401
import os

from common import BigramScorer, Scorer, TorchScorer, load_vocab, read_sentences  # noqa: F401


def load_user(ckpt: str, device: str = "cpu") -> Scorer:
    """TODO: 按你们训练代码改写。下面是模板（注释状态）。"""
    raise NotImplementedError(
        "请编辑 adapters.py 的 load_user：构造模型、载入权重，返回 TorchScorer。\n"
        "关键点: (1) forward(ids[B,T]) 必须返回 logits[B,T,V]；"
        "(2) bos_id/stoi 必须与训练一致；(3) max_len 用训练窗口（mobile=128, adv v2=192）。"
    )
    # ---- 模板 ----------------------------------------------------------
    # import torch
    # from my_train import GPT, GPTConfig            # 你们的训练代码
    # sd = torch.load(ckpt, map_location="cpu")
    # model = GPT(GPTConfig(**sd["config"]))
    # model.load_state_dict(sd["model"]); model.eval().to(device)
    # stoi = json.load(open("vocab.json", encoding="utf-8"))   # char -> id，含 "[BOS]"
    # return TorchScorer(forward=lambda x: model(x),
    #                    stoi=stoi, bos_id=stoi["[BOS]"],
    #                    max_len=192, device=device, batch_size=64)


def load_bigram(ckpt: str, device: str = "cpu") -> Scorer:
    """冒烟测试用。ckpt = 语料路径，可写成 'corpus.txt@0.5' 表示只用前 50% 的行（造一个弱模型）。"""
    frac = 1.0
    if "@" in ckpt:
        ckpt, f = ckpt.rsplit("@", 1)
        frac = float(f)
    lines = [t for _, t in read_sentences(ckpt)]
    sub = lines[: max(1, int(len(lines) * frac))]
    extra = []
    vp = os.environ.get("BIGRAM_VOCAB")            # 可选: 额外词表(vocab.json/txt)，补全语料里没出现的字
    if vp:
        extra = [c for c in load_vocab(vp) if len(c) == 1]
    return BigramScorer(sub, vocab_from=lines + ["".join(extra)])   # 词表用全量，弱模型只是少看了数据


# ── hanhuo 接线（2026-11-14; ckpt = data/nnlm_open_*.safetensors 导出件） ──
def load_hanhuo(ckpt: str, device: str = "cpu") -> Scorer:
    import json as _json
    import struct as _struct
    import sys as _sys
    _sys.path.insert(0, "/mnt/s790/project/vtype-linux-archive/hanhuo/tools")
    import train_nnlm as T  # noqa: E402

    raw = open(ckpt, "rb").read()
    (hlen,) = _struct.unpack("<Q", raw[:8])
    hdr = _json.loads(raw[8:8 + hlen])
    meta = hdr["__metadata__"]
    blob = 8 + hlen

    def rd(n):
        e = hdr[n]
        b = raw[blob + e["data_offsets"][0]:blob + e["data_offsets"][1]]
        import torch as _t
        return _t.frombuffer(bytearray(b), dtype=_t.float32).clone()

    vb, vo = hdr["vocab.bytes"], hdr["vocab.offsets"]
    vbytes = raw[blob + vb["data_offsets"][0]:blob + vb["data_offsets"][1]]
    offs = _struct.unpack(f"<{vo['shape'][0]}q",
                          raw[blob + vo["data_offsets"][0]:
                              blob + vo["data_offsets"][1]])
    stoi = {}
    for i, (a, b) in enumerate(zip(offs, offs[1:])):
        ch = vbytes[a:b].decode("utf-8")
        stoi["[BOS]" if i == 0 else ch] = i
    # 腾讯容器 offsets 解码可能缺尾段: 对齐 wte 行数（缺补占位/多余截断）
    n_rows = int(hdr["wte"]["shape"][0])
    for i in range(len(stoi), n_rows):
        stoi[f"[PAD{i}]"] = i
    if len(stoi) > n_rows:
        stoi = {c: i for c, i in stoi.items() if i < n_rows}

    T.N_LAYER = int(meta["n_layer"]); T.D = int(meta["d_model"])
    T.N_HEAD = int(meta["n_heads"]); T.HEAD_DIM = int(meta["head_dim"])
    T.CTX = int(meta["n_ctx"]); T.VARIANT = meta["variant"]
    T.GELU_C = float(meta["gelu_c"]); T.GELU_A = 0.044715 \
        if meta["variant"] == "advanced" else 0.044708252
    T.FFN = {("advanced", 3): 1024, ("mobile", 2): 768,
             ("advanced", 2): 1024}[meta["variant"], T.N_LAYER]
    T.RESID_DROPOUT = 0.0
    m = T.build_torch_model(len(stoi), device)
    sd = {"wte.weight": rd("wte").view(hdr["wte"]["shape"]),
          "wpe.weight": rd("wpe").view(hdr["wpe"]["shape"]),
          "ln_f.weight": rd("ln_f_g"), "ln_f.bias": rd("ln_f_b")}
    for li in range(T.N_LAYER):
        for src, dst in (("attn_w", "attn.qkv.weight"),
                         ("attn_b", "attn.qkv.bias"),
                         ("proj_w", "attn.proj.weight"),
                         ("proj_b", "attn.proj.bias"),
                         ("ln1_g", "ln1.weight"), ("ln1_b", "ln1.bias"),
                         ("ln2_g", "ln2.weight"), ("ln2_b", "ln2.bias"),
                         ("fc_w", "fc.weight"), ("fc_b", "fc.bias"),
                         ("pr2_w", "pr2.weight"), ("pr2_b", "pr2.bias")):
            k = f"blocks.{li}.{src}"
            sd[f"blocks.{li}.{dst}"] = rd(k).view(hdr[k]["shape"])
    missing, unexpected = m.load_state_dict(sd, strict=False)
    assert not missing, f"装载缺键: {missing}"
    m.eval().to(device)
    return TorchScorer(forward=lambda x: m(x), stoi=stoi,
                       bos_id=stoi["[BOS]"], max_len=T.CTX,
                       device=device, batch_size=48)
