#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""probe_colloc.py — adv 优化咨询档 §一 两个零成本机理探针

1. Logit Lens 逐层剥离: ln_f(h_i) @ wte.T 逐层看「好」vs 泛用字边际
   → 验证机理1（顶层 MLP 平滑器捷径覆盖搭配回路）
2. 句首偏移探针: 「今天天气很」前拼 30 字日常对话再测
   → 验证机理2（BOS sink / 短前缀饥饿: 句首二元先验污染）

用法: python3 tools/probe_colloc.py [model1 model2 ...]
      模型路径自动识别 .ckpt / .safetensors（adv 档）
"""
import sys
import struct
import torch
import torch.nn.functional as F

sys.path.insert(0, "tools")
import train_nnlm as T  # noqa: E402

ADV = (3, 256, 4, 64, 1024, 0.7978845608028654, 0.044715)
FOILS = "我开心的"


def load(path):
    T.N_LAYER, T.D, T.N_HEAD, T.HEAD_DIM, T.FFN = ADV[:5]
    T.GELU_C, T.GELU_A = ADV[5:]
    T.VARIANT = "advanced"
    if path.endswith(".ckpt"):
        ck = torch.load(path, map_location="cpu", weights_only=False)
        vocab, sd = ck["vocab"], ck["sd"]
        T.CTX = max(int(sd["wpe.weight"].shape[0]), 128)
    else:
        hdr, raw2, vocab = T.read_export(path)
        T.CTX = hdr["wpe"]["shape"][0]
        blob = 8 + struct.unpack("<Q", raw2[:8])[0]

        def rd(n):
            e = hdr[n]
            b = raw2[blob + e["data_offsets"][0]:
                     blob + e["data_offsets"][1]]
            return torch.frombuffer(
                bytearray(b), dtype=torch.float32).clone()
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
    m = T.build_torch_model(len(vocab), "cpu")
    m.load_state_dict(sd, strict=False)
    m.eval()
    return m, vocab


@torch.no_grad()
def states(m, ids):
    """embed → 每块 attn 后/mlp 后 的残差流状态序列"""
    x = m.wte(ids) + m.wpe(torch.arange(ids.shape[1]))
    out = [("Embed", x.clone())]
    for i, b in enumerate(m.blocks):
        x = x + b.attn(b.ln1(x))
        out.append((f"L{i}_postAttn", x.clone()))
        h = b.fc(b.ln2(x))
        h = 0.5 * h * (1 + torch.tanh(
            T.GELU_C * (h + T.GELU_A * h ** 3)))
        x = x + b.pr2(h)
        out.append((f"L{i}_postMLP", x.clone()))
    return out


@torch.no_grad()
def lens(m, vocab, text, tgt="好", tag=""):
    stoi = {c: i for i, c in enumerate(vocab)}
    ids = torch.tensor([[stoi[c] for c in text]])
    t = stoi[tgt]
    foils = [stoi[c] for c in FOILS if c in stoi]
    print(f"  [{tag}] Logit Lens 「{text}」→ {tgt}")
    for name, h in states(m, ids):
        lp = F.log_softmax(m.ln_f(h[:, -1]) @ m.wte.weight.t(), -1)[0]
        f_max, f_c = max((lp[f].item(), vocab[f]) for f in foils)
        print(f"    {name:12s} {tgt}={lp[t]:+.2f}  "
              f"maxFoil {f_c}={f_max:+.2f}  margin={lp[t]-f_max:+.2f}")


@torch.no_grad()
def pad_probe(m, vocab, pre, stem="今天天气很", tgt="好", foil="我"):
    stoi = {c: i for i, c in enumerate(vocab)}
    out = []
    for txt in (stem, pre + stem):
        ids = torch.tensor([[stoi[c] for c in txt]])
        lp = F.log_softmax(m(ids)[0, -1], -1)
        out.append((lp[stoi[tgt]].item(), lp[stoi[foil]].item()))
    (a_h, a_w), (b_h, b_w) = out
    print(f"  偏移探针: {tgt} {a_h:+.2f}→{b_h:+.2f}  "
          f"{foil} {a_w:+.2f}→{b_w:+.2f}  "
          f"（前拼 {len(pre)} 字后 {tgt}{'↑回升' if b_h > a_h else '↓'}"
          f"/{foil}{'↓消失' if b_w < a_w else '↑'}）")


def main():
    models = sys.argv[1:] or [
        "data/nnlm_open_adv_v1.safetensors",
        "/mnt/s790/tmp/adv_v2/v2a.ckpt",
        "/mnt/s790/tmp/adv_v2/v2b.ckpt",
    ]
    for path in models:
        m, vocab = load(path)
        name = path.split("/")[-1]
        print(f"\n== {name}")
        lens(m, vocab, "今天天气很", tag="短前缀")
        pad_probe(m, vocab, "早上吃什么？随便吃点吧。我们聊聊昨天的事情，")


if __name__ == "__main__":
    main()
