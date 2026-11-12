#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""train_nnlm.py — 自训开源 NNLM（c.6 P1: nlrgptand mobile 档镜像）

架构 1:1 复刻 crates/hanhuo-lm/src/nnlm.rs（to_safetensors 契约）:
  2 层 pre-LN GPT-2, d192, 4H×48, FFN 768, gelu-tanh(0.7978515625),
  attn scale √48=6.928203, wpe ctx 128。
  - attn_w [576,192] = Q/K/V 三投影行拼接
  - 双 oracle 训练后一次前向导出: bos_logp[V] / start_k,start_v[384]
    （BOS 位逐层 KV; 引擎 start_past n_past=1 起步, 推理不喂 BOS）
  - 词表 vocab.bytes(U8)+vocab.offsets(I64); 词表外字符引擎侧整词弃
  - safetensors 元数据 variant=mobile; 自训版 license_note 注开源

语料: LCCC jsonl.gz（每行 ["句1","句2",...] 对话轮）+ 文本文件
（THUCNews 解包后 .txt / poetry / lyric 逐行）。行级清洗: 繁→简
（可用 opencc, 缺省跳过）、去 URL/控制符、长度带 8..128。
词表: 语料高频字 top（默认 8105+ascii 封顶）+ [BOS]（id 0）。

用法:
  训练: python3 tools/train_nnlm.py --epochs 2 --out data/nnlm_open_mobile.safetensors
  冒烟: python3 tools/train_nnlm.py --smoke --max-chars 2_000_000
         --steps 200 --out /tmp/smoke.safetensors
验收: hanhuo query <py> --nnlm <out> --boost-nn ... 热插对比
许可: 本脚本 Apache-2.0; 产出权重随训语料许可（开源混合语料→可开源）。
"""
import argparse
import gzip
import json
import math
import os
import random
import re
import struct
import sys
import time

# 残差 dropout 率（默认 0=恒等; --dropout 覆盖——训练态专用, 导出无参）
RESID_DROPOUT = 0.0
# v3.5 句界打包的块对角因果掩码（[T,T] bool, True=可注意; None=纯因果）
CUR_MASK = None

# ── 架构常量（两档契约, nnlm.rs / nnlm_adv.rs 定形） ──────────
# mobile: 2×d192/4H×48/FFN768/ctx128, gelu_c=fp16 圆整值, bos 双 oracle
# adv:    3×d256/4H×64/FFN1024/ctx512, gelu 精确常量, 纯 past-concat
#         （无 bos_logp/start_kv）, 必带 wz/bz 签名头（自训填零）
ARCH = {
    "mobile": dict(n_layer=2, d=192, n_head=4, head_dim=48, ffn=768,
                   ctx=128, variant="mobile",
                   gelu_c=0.7978515625, gelu_a=0.044708252,
                   bos_oracle=True),
    "enlm": dict(n_layer=4, d=256, n_head=4, head_dim=64, ffn=1024,
                 ctx=256, variant="advanced",  # 容器兼容 nnlm_adv 装载器
                 gelu_c=0.7978845608028654, gelu_a=0.044715,
                 bos_oracle=False),
    "adv": dict(n_layer=3, d=256, n_head=4, head_dim=64, ffn=1024,
                ctx=512, variant="advanced",
                gelu_c=0.7978845608028654, gelu_a=0.044715,
                bos_oracle=False),
}
N_LAYER = D = N_HEAD = HEAD_DIM = FFN = CTX = None  # 由 ARCH 注入
GELU_C = None
EPS = 1e-5
VARIANT = "mobile"

URL_RE = re.compile(r"https?://\S+|www\.\S+")
CTRL_RE = re.compile(r"[\x00-\x08\x0b-\x1f\x7f]")

# GujiCC（forFudan, GB/Z 40637-2021 古籍规范字形）字符映射——古籍/诗词
# 语料异体→规范简体（现代文本命中极低, 无害）; 表缺则跳过。
# GSCharacters.txt: 源字\t目标（g2s.json 是 OpenCC 配置壳非映射本体）
GUJI_CC = os.path.expanduser("~/tmp/gujicc/opencc/GSCharacters.txt")
VOCAB_TABLE = os.path.expanduser(
    "~/tmp/frost_ds/cn_dicts/8105.dict.yaml")  # 通用规范汉字表 8105+字频


def _load_g2s():
    """GSCharacters.txt（源\t目标[, 多候选空格分, 取首]）→ 单字映射"""
    m = {}
    try:
        with open(GUJI_CC, encoding="utf-8") as f:
            for ln in f:
                if ln.startswith(("#", "---")) or "\t" not in ln:
                    continue
                src, dst = ln.rstrip("\n").split("\t", 1)
                d = dst.split(" ")[0]
                if len(src) == 1 and len(d) == 1 and src != d:
                    m[src] = d
    except Exception:
        pass
    return m


def _load_table_chars():
    r"""通用规范汉字表 8105——首选权威解析件（Standard_glyph.htm
    编号形, 一级3500/二级3000/三级1605 序号连续性自校验, 级序=频次
    先验; 用户指源 2026-11-11）; 缺则退 frost 8105.dict.yaml。
    另有古籍 14250 表（guji_14250.txt, GB/Z 40637-2021）在库备查——
    超表字按语料频次补位, 不整表并入"""
    # 仓内权威件优先（data/ 随 git）; 本机 ~/tmp 解析件次之
    p = os.path.join(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__))), "data", "guifan_8105.txt")
    if not os.path.exists(p):
        p = os.path.expanduser("~/tmp/corpus/guifan_8105.txt")
    out = []
    if os.path.exists(p):
        for ln in open(p, encoding="utf-8"):
            if ln.startswith("# 一级 ") or ln.startswith("# 二级 ") \
                    or ln.startswith("# 三级 "):
                out.extend(list(ln.rstrip("\n").split(" ", 2)[2]))
        if len(out) == 8105:
            return out
    try:
        with open(VOCAB_TABLE, encoding="utf-8") as f:
            for ln in f:
                if ln.startswith(("#", "---", "...")) or "\t" not in ln:
                    continue
                w = ln.split("\t")[0].strip()
                if len(w) == 1:
                    out.append(w)
    except Exception:
        pass
    return out


def log(msg):
    print(f"[{time.time() % 1e5:9.1f}] {msg}", flush=True)


# ── 语料管线 ───────────────────────────────────────────────
def iter_lines(paths):
    for p in paths:
        if p.endswith(".jsonl.gz"):
            with gzip.open(p, "rt", encoding="utf-8", errors="ignore") as f:
                for ln in f:
                    try:
                        for turn in json.loads(ln):
                            yield turn
                    except Exception:
                        continue
        else:
            with open(p, encoding="utf-8", errors="ignore") as f:
                for ln in f:
                    yield ln.rstrip("\n")


def clean(s, g2s=None):
    """LCCC 系分词语料（"狗 年 行大运 。"）——剥离全部空白;
    中文 LM 无自然空格, 分词空格是语料伪影（不剥则空格占 40% 字符流,
    模型学会平凡交替律, loss 假崩 0.07——2026-11-11 验尸教训）。
    g2s: GujiCC 古籍规范字形→简体逐字映射（诗词/古籍语料归一）"""
    s = URL_RE.sub(" ", s)
    s = CTRL_RE.sub(" ", s)
    s = "".join(s.split())
    if g2s and len(s) >= 3:
        s = "".join(g2s.get(c, c) for c in s)
    return s


def round_robin(paths, per_take=2000):
    r"""多源轮转行流（词表扫描用）——单源顺序扫会让 LCCC×4 前缀
    吃掉全部扫描配额, 古籍/wiki 表外字频被漏（2026-11-11 古字缺口
    验尸: 恆/遯/燄 不在 v2 词表）"""
    import itertools
    gens = [iter_lines([p]) for p in paths]
    while gens:
        for g in list(gens):
            for _ in range(per_take):
                try:
                    yield next(g)
                except StopIteration:
                    gens.remove(g)
                    break


def build_vocab(args, texts):
    r"""词表 = [BOS] + 通用规范汉字表 8105（基表全收, 频次序）+
    语料超表字补位（--vocab-extra 上限）+ ASCII 可打印。
    引擎 v 已动态化（Nnlm.v = wte 行数）, 容量自由; 词表外字符
    引擎侧整词弃（encode→None 优雅降级）
    --word-level N: 英文词级模式——词表 = 语料词频 top N（wltm
    开源等价, 词表 25,046 对齐腾讯规模量级）"""
    if args.word_level:
        from collections import Counter as _C
        cnt = _C()
        _tot = 0
        for t in texts:
            ws = t.split()
            cnt.update(ws)
            _tot += len(ws)
            if _tot > 50_000_000:
                break
        vocab = ["\x00"] + [w for w, _ in
                            sorted(cnt.items(), key=lambda x: -x[1])
                            [:args.word_level]]
        log(f"词表(词级) {len(vocab)}")
        return vocab
    from collections import Counter
    cnt = Counter()
    n = 0
    for t in texts:
        cnt.update(t)
        n += 1
        if n >= args.vocab_scan_lines:
            break
    vocab = ["\x00"]  # id 0 = BOS（嵌入用, 不参与解码）
    seen = {"\x00"}
    for ch in _load_table_chars():
        if ch not in seen:
            vocab.append(ch)
            seen.add(ch)
    n_extra = 0
    for ch, _ in sorted(cnt.items(), key=lambda x: -x[1]):
        if ch in seen:
            continue
        if n_extra >= args.vocab_extra:
            break
        vocab.append(ch)
        seen.add(ch)
        n_extra += 1
    # ASCII 可打印（英文候选可进 NN 打分——腾讯版结构性无此能力）
    for c in (chr(i) for i in range(33, 127)):
        if c not in seen:
            vocab.append(c)
            seen.add(c)
    log(f"词表 {len(vocab)}（扫 {n} 行; 8105 基表 + 超表字 + ASCII）")
    return vocab


def encode_stream(texts, stoi, word_level=False):
    """逐文本 → np.uint16 数组流（词表外字符丢弃; numpy 原生——
    python int 列表 28B/字, 亿级字会 RAM 爆）; 词级模式按空格切"""
    if word_level:
        for t in texts:
            ids = np_fromiter(
                (stoi[w] for w in t.split() if w in stoi),
                dtype="uint16")
            if len(ids) >= 4:
                yield ids
        return
    for t in texts:
        ids = np_fromiter((stoi[c] for c in t if c in stoi),
                          dtype="uint16")
        if len(ids) >= 4:
            yield ids


def np_fromiter(gen, dtype):
    import numpy as np
    return np.fromiter(gen, dtype=dtype)


# ── 模型（torch 镜像） ─────────────────────────────────────
def build_torch_model(V, device):
    import torch
    import torch.nn as nn

    # 残差 dropout（训练态; DROP=恒等时导出/推理语义不变）。
    # v2d 实验位: 治 MLP 平滑器捷径随训练固化（咨询档机理1）
    global DROP
    DROP = nn.Dropout(RESID_DROPOUT) if RESID_DROPOUT > 0 \
        else (lambda x: x)

    class Attn(nn.Module):
        def __init__(self):
            super().__init__()
            # 构造期冻结维度（KD 双模型并训时全局 D 会被教师档改写）
            self._d, self._nh, self._hd = D, N_HEAD, HEAD_DIM
            self.qkv = nn.Linear(D, 3 * D, bias=True)   # →attn_w[576,192]
            self.proj = nn.Linear(D, D, bias=True)

        def forward(self, x):
            B, T, _ = x.shape
            d, nh, hd = self._d, self._nh, self._hd
            q, k, v = self.qkv(x).split(d, dim=2)
            q = q.view(B, T, nh, hd).transpose(1, 2)
            k = k.view(B, T, nh, hd).transpose(1, 2)
            v = v.view(B, T, nh, hd).transpose(1, 2)
            # v3.5 句界打包: 句间块对角掩码（CUR_MASK 全局注入, 缺省 None）
            # is_causal 必须显式——漏掩码则位置可见未来 token, 模型学会
            # 「抄下一位置」把 loss 假崩至 0.04（2026-11-11 二次验尸）
            y = torch.nn.functional.scaled_dot_product_attention(
                q, k, v, is_causal=CUR_MASK is None, attn_mask=CUR_MASK)
            y = y.transpose(1, 2).reshape(B, T, d)
            return self.proj(y)

    class Block(nn.Module):
        def __init__(self):
            super().__init__()
            self._d, self._ffn = D, FFN
            self._gc, self._ga = GELU_C, GELU_A
            self.ln1 = nn.LayerNorm(D, eps=EPS)
            self.attn = Attn()
            self.ln2 = nn.LayerNorm(D, eps=EPS)
            self.fc = nn.Linear(D, FFN, bias=True)
            self.pr2 = nn.Linear(FFN, D, bias=True)

        def forward(self, x):
            x = x + DROP(self.attn(self.ln1(x)))
            h = self.fc(self.ln2(x))
            h = 0.5 * h * (1 + torch.tanh(self._gc * (h + self._ga * h**3)))
            return x + DROP(self.pr2(h))

    class GPT(nn.Module):
        def __init__(self):
            super().__init__()
            self.wte = nn.Embedding(V, D)
            self.wpe = nn.Embedding(CTX, D)
            self.blocks = nn.ModuleList([Block() for _ in range(N_LAYER)])
            self.ln_f = nn.LayerNorm(D, eps=EPS)

        def forward(self, idx, pos=None):
            B, T = idx.shape
            if pos is None:
                pos = torch.arange(T, device=idx.device)
            x = self.wte(idx) + self.wpe(pos)
            for b in self.blocks:
                x = b(x)
            return self.ln_f(x) @ self.wte.weight.t()

    torch.manual_seed(20261111)
    m = GPT().to(device)
    # GPT-2 式初始化: N(0,0.02) + 残差投影 1/√(2L) 缩放
    # （缺省 torch init 方差过大, 初始 loss 126 起步浪费早期步）
    with torch.no_grad():
        for name, p in m.named_parameters():
            if "weight" in name and p.dim() >= 2:
                p.normal_(0.0, 0.02)
            elif "bias" in name:
                p.zero_()
        for li, b in enumerate(m.blocks):
            b.attn.proj.weight.mul_(1 / math.sqrt(2 * N_LAYER))
            b.pr2.weight.mul_(1 / math.sqrt(2 * N_LAYER))
    return m



def read_export(path):
    """自训导出件 → (hdr, raw, vocab)——词表早读（编码前替换 stoi）+
    权重后载共用; 仅自训件（腾讯件禁入衍生线）"""
    import struct as _st
    raw = open(path, "rb").read()
    hn = _st.unpack("<Q", raw[:8])[0]
    hdr = json.loads(raw[8:8 + hn])
    blob = 8 + hn
    vb = raw[blob + hdr["vocab.bytes"]["data_offsets"][0]:
             blob + hdr["vocab.bytes"]["data_offsets"][1]]
    voff = _st.unpack(
        f"<{hdr['vocab.offsets']['shape'][0]}q",
        raw[blob + hdr["vocab.offsets"]["data_offsets"][0]:
            blob + hdr["vocab.offsets"]["data_offsets"][1]])
    vocab = [vb[voff[i]:voff[i + 1]].decode("utf-8", "replace")
             for i in range(len(voff) - 1)]
    return hdr, raw, vocab


def gelu_tanh(x):
    import torch
    return 0.5 * x * (1 + torch.tanh(GELU_C * (x + GELU_A * x**3)))


def make_qat(bits: int):
    r"""STE fake-quant monkey-patch——nn.Linear 权重 per-out-channel
    对称量化前向, 梯度直通（P1b: 手机 int8/int4 档; wte 保 f32, v1）"""
    import torch
    import torch.nn as nn
    import torch.nn.functional as F

    class _FQ(torch.autograd.Function):
        @staticmethod
        def forward(ctx, w):
            qmax = 127 if bits == 8 else 7
            s = w.abs().amax(dim=-1, keepdim=True).clamp(min=1e-8) / qmax
            return torch.clamp(torch.round(w / s), -qmax, qmax) * s

        @staticmethod
        def backward(ctx, g):
            return g  # STE

    orig = nn.Linear.forward

    def patched(self, x):
        return F.linear(x, _FQ.apply(self.weight), self.bias)

    nn.Linear.forward = patched
    return lambda: setattr(nn.Linear, "forward", orig)


# ── safetensors 导出（手写容器: [u64 len][JSON][blob]） ────
def export(m, vocab, out_path, license_note, fp16=False, int8=False):
    import torch
    t = {}

    def put(name, tensor, shape):
        x = tensor.detach().cpu().to(torch.float32)
        keep_f32 = name in ("bos_logp", "start_k", "start_v",
                            "wpe", "ln_f_g", "ln_f_b") or name.endswith(
            ("_g", "_b", ".scale"))
        if int8 and (name == "wte" or (name.startswith("blocks.") and
                any(name.endswith(w) for w in
                    ("attn_w", "proj_w", "fc_w", "pr2_w")))):
            # per-out-channel 对称 I8 + scale（QAT 训练权重已可整数量化）
            qmax = 127.0
            sc = x.abs().amax(dim=-1, keepdim=True).clamp(min=1e-8) / qmax
            q = torch.clamp(torch.round(x / sc), -127, 127).to(torch.int8)
            t[name] = (shape, q.contiguous().view(-1).numpy().tobytes(),
                       "I8")
            t[name + ".scale"] = ([shape[0]],
                                  sc.view(-1).numpy().astype(
                                      "float32").tobytes(), "F32")
            return
        if fp16 and not keep_f32:
            x = x.to(torch.float16)
        dtyp = "F16" if fp16 and not keep_f32 else "F32"
        t[name] = (shape, x.contiguous().view(-1).numpy().tobytes(), dtyp)

    sd = m.state_dict()
    put("wte", sd["wte.weight"], [len(vocab), D])
    put("wpe", sd["wpe.weight"], [CTX, D])
    put("ln_f_g", sd["ln_f.weight"], [D])
    put("ln_f_b", sd["ln_f.bias"], [D])
    for li in range(N_LAYER):
        p = f"blocks.{li}."
        # torch Linear weight [out,in] → 引擎 [out,in] 行主序直存
        put(p + "attn_w", sd[f"blocks.{li}.attn.qkv.weight"], [3 * D, D])
        put(p + "attn_b", sd[f"blocks.{li}.attn.qkv.bias"], [3 * D])
        put(p + "proj_w", sd[f"blocks.{li}.attn.proj.weight"], [D, D])
        put(p + "proj_b", sd[f"blocks.{li}.attn.proj.bias"], [D])
        put(p + "ln1_g", sd[f"blocks.{li}.ln1.weight"], [D])
        put(p + "ln1_b", sd[f"blocks.{li}.ln1.bias"], [D])
        put(p + "ln2_g", sd[f"blocks.{li}.ln2.weight"], [D])
        put(p + "ln2_b", sd[f"blocks.{li}.ln2.bias"], [D])
        put(p + "fc_w", sd[f"blocks.{li}.fc.weight"], [FFN, D])
        put(p + "fc_b", sd[f"blocks.{li}.fc.bias"], [FFN])
        put(p + "pr2_w", sd[f"blocks.{li}.pr2.weight"], [D, FFN])
        put(p + "pr2_b", sd[f"blocks.{li}.pr2.bias"], [D])

    # 双 oracle: BOS 前向 → 首字分布 + 逐层 K/V
    # （mobile 契约专属——adv 纯 past-concat 无 start_cache 机制）
    m.eval()
    if VARIANT == "mobile":
      with torch.no_grad():
        bos = torch.zeros(1, 1, dtype=torch.long, device=next(m.parameters()).device)
        logits = m(bos)[0, -1]              # p(首字 | BOS)
        bos_logp = torch.log_softmax(logits, dim=-1)
        x = m.wte(bos) + m.wpe(torch.zeros(1, 1, dtype=torch.long,
                                           device=bos.device))
        ks, vs = [], []
        for b in m.blocks:
            h = b.ln1(x)
            B_, T_, _ = h.shape
            _, k, v = b.attn.qkv(h).split(D, dim=2)
            k = k.view(B_, T_, N_HEAD, HEAD_DIM).transpose(1, 2)
            v = v.view(B_, T_, N_HEAD, HEAD_DIM).transpose(1, 2)
            # 引擎 start_k[384] = layer0_K[192] ++ layer1_K[192]
            # （BOS 位逐层 K, 头主序展平——start_past 逐层切片 192）
            ks.append(k[0, :, 0, :].reshape(-1))   # [D]
            vs.append(v[0, :, 0, :].reshape(-1))
            x = x + b.attn(h)                       # 复用 Block 前半
            x = x + b.pr2(gelu_tanh(b.fc(b.ln2(x))))
    if VARIANT == "mobile":
        put("bos_logp", bos_logp, [len(vocab)])
        put("start_k", torch.cat(ks), [N_LAYER * D])
        put("start_v", torch.cat(vs), [N_LAYER * D])

    vb = bytearray()
    voff = [0]
    for tok in vocab:
        vb.extend(tok.encode("utf-8"))
        voff.append(len(vb))

    is_adv = VARIANT == "advanced"
    if is_adv:
        # adv 契约（nnlm_adv.rs）: z 签名头必带（自训填零, 用途未明）
        zt = torch.zeros(64, D)
        put("wz", zt, [64, D])
        put("bz", torch.zeros(64), [64])
    meta = {"format": "hanhuo-nnlm", "version": "1", "variant": VARIANT,
            "n_layer": str(N_LAYER), "d_model": str(D),
            "n_heads": str(N_HEAD), "head_dim": str(HEAD_DIM),
            "vocab_size": str(len(vocab)), "n_ctx": str(CTX),
            "eps": "1e-5",
            "gelu_c": f"{GELU_C!r}",
            "gelu_const_fp16": "false" if is_adv else "true",
            "attn_scale": "0.125" if is_adv else "6.928203",
            "gelu": "tanh", "license_note": license_note}
    if int8:
        meta["quant"] = "int8"

    t["vocab.bytes"] = ([len(vb)], bytes(vb), "U8")
    t["vocab.offsets"] = ([len(voff)],
                          struct.pack(f"<{len(voff)}q", *voff), "I64")
    names = sorted(t)
    entries, blob, hdr = [], bytearray(), {
        "__metadata__": meta}
    for n in names:
        shape, data, dtyp = t[n]
        s = len(blob)
        blob.extend(data)
        hdr[n] = {"dtype": dtyp, "shape": shape,
                  "data_offsets": [s, len(blob)]}
    hb = json.dumps(hdr, ensure_ascii=False,
                    separators=(",", ":")).encode("utf-8")
    with open(out_path, "wb") as f:
        f.write(struct.pack("<Q", len(hb)))
        f.write(hb)
        f.write(blob)
    log(f"导出 {out_path} ({(len(hb) + len(blob)) / 1e6:.1f} MB, "
        f"vocab {len(vocab)})")


# ── 训练 ───────────────────────────────────────────────────
def main():
    import numpy as np
    import torch

    ap = argparse.ArgumentParser()
    ap.add_argument("--variant", choices=list(ARCH), default="mobile",
                    help="mobile=nlrgptand 2×192 / adv=nlrgpt2 3×256")
    ap.add_argument("--lccc", default=os.path.expanduser(
        "~/tmp/corpus/lccc/lccc_base_train.jsonl.gz"))
    ap.add_argument("--texts", nargs="*", default=[],
                    help="附加纯文本语料（THUCNews 解包 .txt / poetry 等）")
    ap.add_argument("--out", default="data/nnlm_open_mobile.safetensors")
    ap.add_argument("--epochs", type=float, default=2.0)
    ap.add_argument("--steps", type=int, default=0, help=">0 则覆盖 epochs")
    ap.add_argument("--batch", type=int, default=64)
    ap.add_argument("--seq", type=int, default=128,                    help="训练窗长（≤ctx; default=CTX 在注入前是 None——坑）")
    ap.add_argument("--ls", type=float, default=0.0,
                    help="label smoothing ε（adv-v2: 治 ΔN 过锐, 文献 "
                         "Calibrated-LMs 首选正则）")
    ap.add_argument("--ctx-override", type=int, default=0,
                    help=">0 压缩 ctx（adv-v2: 512→192, 引擎 wpe 动态）")
    ap.add_argument("--dropout", type=float, default=0.0,
                    help="残差 dropout 率（v2d: 治 MLP 捷径固化; 导出无参）")
    ap.add_argument("--kd-teacher", default="",
                    help="离线教师 safetensors（咨询三善后: 同音簇局部")
    ap.add_argument("--kd-alpha", type=float, default=0.3,
                    help="KD 项权重（同音簇内 KL, 隔离教师词内过自信）")
    ap.add_argument("--word-level", type=int, default=0,
                    help=">0=词级英文模式（词表 top N; wltm 开源等价）")
    ap.add_argument("--sent-pack", action="store_true",
                    help="v3.5 句界对齐打包: 每句真 BOS+位置重置+块对角掩码")
    ap.add_argument("--homo-ce", type=float, default=0.0,
                    help="v3.5 同音簇约束 CE 权重（PinyinGPT 式 restricted softmax）")
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--vocab-extra", type=int, default=800,
                    help="8105 基表外的语料超表字补位上限")
    ap.add_argument("--vocab-scan-lines", type=int, default=2_000_000)
    ap.add_argument("--max-chars", type=int, default=400_000_000)
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--license-note",
                    default="自训开源权重（c.6）: 语料 LCCC/zhwiki/THUCNews/"
                            "poetry/lyric 混合, Apache-2.0")
    ap.add_argument("--save-ckpt", default="",
                    help="训练态 checkpoint 落盘（state_dict+vocab+arch, "
                         "QAT 续训入口）")
    ap.add_argument("--load-safetensors", default="",
                    help="从导出件载入（权重+词表自足——导出件即 "
                         "checkpoint; 仅自训件, 腾讯件禁入衍生线）")
    ap.add_argument("--load-ckpt", default="",
                    help="从 checkpoint 续训（QAT: fp32 训完 → 末段量化微调）")
    ap.add_argument("--qat", type=int, default=0, choices=(0, 8, 4),
                    help="0=off; 8/4=末段 QAT（STE fake-quant, "
                         "per-out-channel 对称; Linear 层, wte 保 f32）")
    ap.add_argument("--ckpt-every", type=int, default=0,
                    help=">0: 每 N 步落 ckpt（断点续训——v2 于 82% 被"
                         "内存清扫击杀的教训 2026-11-11）")
    ap.add_argument("--qat-steps", type=int, default=0,
                    help="QAT 末段步数（默认总步数 15%%）")
    ap.add_argument("--int8-out", default="",
                    help="量化版导出（Linear 权重 I8+per-out-channel scale, "
                         "盘上 -75%%; 装载侧反量化——P1b）")
    ap.add_argument("--fp16-out", default="",
                    help="同时导出 fp16 盘上版（x86 CPU 正解: f16 文件"
                         "+装载转 f32 计算; bos_logp/start_kv 保 F32）")
    args = ap.parse_args()
    if args.smoke:
        args.max_chars, args.steps = 2_000_000, 200
        args.vocab_scan_lines = 200_000
        args.out = "/tmp/smoke_nnlm.safetensors"

    # 选档注入: 模块级常量按 ARCH 落值（后续 build/export 直接用）
    a = ARCH[args.variant]
    globals().update(N_LAYER=a["n_layer"], D=a["d"], N_HEAD=a["n_head"],
                     HEAD_DIM=a["head_dim"], FFN=a["ffn"], CTX=a["ctx"],
                     VARIANT=a["variant"], GELU_C=a["gelu_c"],
                     GELU_A=a["gelu_a"])
    if args.ctx_override:  # adv-v2 优化: 压 ctx 抗过拟合（引擎 wpe 动态）
        globals()["CTX"] = args.ctx_override
    globals()["RESID_DROPOUT"] = args.dropout
    args.seq = min(args.seq, CTX)
    log(f"档位 {args.variant}: {N_LAYER}×d{D}/4H×{HEAD_DIM}/"
        f"FFN{FFN}/ctx{CTX}")
    device = "cuda" if torch.cuda.is_available() else "cpu"
    log(f"设备 {device}" + (f"（{torch.cuda.get_device_name(0)}）"
                            if device == "cuda" else "（CPU 冒烟档降参）"))

    log("编码语料为 id 流…")
    stream = []
    total = 0
    # --texts 支持 path×N 复制语法（源配比: 聊天主场 ×N 提权重）
    texts_src = ([args.lccc] if os.path.exists(args.lccc) and
                 not args.texts else [])
    for spec in args.texts:
        if "×" in spec:
            p, n = spec.rsplit("×", 1)
            texts_src.extend([p] * int(n))
        else:
            texts_src.append(spec)
    pre = read_export(args.load_safetensors) \
        if args.load_safetensors and os.path.exists(
            args.load_safetensors) else None
    if texts_src:
        g2s = _load_g2s()
        if pre:
            vocab = pre[2]
            log(f"词表从导出件载入 {len(vocab)}（编码前置换）")
        else:
            # 词表扫描走轮转多源（补位公平覆盖古字/百科表外字）;
            # 训练流仍按源序（配比=源体量）
            # 词级模式: 扫描管线与训练流同律（clean() 剥空格会把
            # == History == 粘成 ==History== 假词——enlm_v1 验尸教训）
            if args.word_level:
                import re as _re2

                def _wc(t):
                    t = t.strip()
                    if not t or t.startswith('=') or '<' in t:
                        return ''
                    return _re2.sub(
                        r"[^a-z0-9' ]+", ' ', t.lower()).strip()

                vocab = build_vocab(
                    args,
                    (t for t in (
                        _wc(x) for x in round_robin(texts_src)) if t))
            else:
                vocab = build_vocab(
                    args, (clean(t, g2s) for t in round_robin(texts_src)))
        stoi = {c: i for i, c in enumerate(vocab)}
        raw = iter_lines(texts_src)
        if args.word_level:
            # 词级轻清洗: 小写化 + 特殊记号/标题行剥离（保空格词界!）
            import re as _re
            def wclean(t):
                t = t.strip()
                if not t or t.startswith('=') or '<' in t:
                    return ''
                return _re.sub(r'[^a-z0-9\' ]+', ' ',
                               t.lower()).strip()
            texts = (wclean(t) for t in raw)
        else:
            texts = (clean(t, g2s) for t in raw)
        if args.sent_pack:
            # v3.5 句界对齐: 逐句切分（。！？!?；;）, 每句独立样本
            import numpy as _np
            sent_pool = []
            for t in texts:
                for sent in re.split(r"[。！？!?；;]", t):
                    if 6 <= len(sent) <= args.seq - 1:
                        ids = _np.fromiter(
                            (stoi[c] for c in sent if c in stoi),
                            dtype="uint16")
                        if len(ids) >= 6:
                            sent_pool.append(ids)
                            total += len(ids)
                if total >= args.max_chars:
                    break
            stream.append(_np.zeros(1, dtype="uint16"))  # flat 占位
        else:
            for ids in encode_stream(texts, stoi, args.word_level > 0):
                stream.append(ids)
                total += len(ids)
                if total >= args.max_chars:
                    break
    else:
        vocab = ["\x00"]
    if stream:
        flat = np.concatenate(stream) if len(stream) > 1 else stream[0]
    else:
        flat = np.zeros(0, dtype=np.uint16)
    del stream
    log(f"语料 {len(flat) / 1e6:.1f}M 字")

    if pre:
        import struct as _st
        import torch as _t
        hdr, raw2, vocab = pre
        blob = 8 + _st.unpack("<Q", raw2[:8])[0]

        def rd_f32(n):
            e = hdr[n]
            b = raw2[blob + e["data_offsets"][0]:blob +
                    e["data_offsets"][1]]
            return _t.frombuffer(bytearray(b), dtype=_t.float32).clone()

        sd = {"wte.weight": rd_f32("wte").view(hdr["wte"]["shape"]),
              "wpe.weight": rd_f32("wpe").view(hdr["wpe"]["shape"]),
              "ln_f.weight": rd_f32("ln_f_g"),
              "ln_f.bias": rd_f32("ln_f_b")}
        for li in range(N_LAYER):
            for src, dst in (
                    ("attn_w", "attn.qkv.weight"),
                    ("attn_b", "attn.qkv.bias"),
                    ("proj_w", "attn.proj.weight"),
                    ("proj_b", "attn.proj.bias"),
                    ("ln1_g", "ln1.weight"), ("ln1_b", "ln1.bias"),
                    ("ln2_g", "ln2.weight"), ("ln2_b", "ln2.bias"),
                    ("fc_w", "fc.weight"), ("fc_b", "fc.bias"),
                    ("pr2_w", "pr2.weight"), ("pr2_b", "pr2.bias")):
                k = f"blocks.{li}.{src}"
                sd[f"blocks.{li}.{dst}"] = rd_f32(k).view(hdr[k]["shape"])
        m = build_torch_model(len(vocab), device)
        m.load_state_dict(sd, strict=False)  # oracle 张量导出时重算
        log(f"导出件载入 {args.load_safetensors}（vocab {len(vocab)}）")
    elif args.load_ckpt and os.path.exists(args.load_ckpt):
        import torch as _t
        ck = _t.load(args.load_ckpt, map_location="cpu", weights_only=False)
        assert ck["variant"] == args.variant, "ckpt 档位不符"
        assert ck["vocab"] == vocab, "ckpt 词表与语料重建不一致（同语料同参即对齐）"
        m = build_torch_model(len(vocab), device)
        m.load_state_dict(ck["sd"])
        log(f"checkpoint 载入 {args.load_ckpt}")
    else:
        m = build_torch_model(len(vocab), device)
    n_param = sum(p.numel() for p in m.parameters())
    log(f"参数 {n_param / 1e6:.2f}M")

    # v3.5 同音簇表（student-only, --homo-ce 时; KD 块的表是师生对,
    # 此处只按学生词表）
    homo_s = homo_mask = None
    if args.homo_ce > 0:
        import torch as _t
        from pypinyin import lazy_pinyin, Style
        _groups = {}
        for ch in vocab[1:]:
            if len(ch) == 1 and '一' <= ch <= '鿿':
                _groups.setdefault(
                    lazy_pinyin(ch, style=Style.NORMAL)[0], []).append(ch)
        _K = 64
        homo_s = _t.full((len(vocab), _K), 0, dtype=_t.long)
        homo_mask = _t.zeros((len(vocab), _K), dtype=_t.bool)
        _n = 0
        for _py, _chs in _groups.items():
            _ids = [stoi[c] for c in _chs]
            if len(_ids) < 2:
                continue
            for si in _ids:
                for j, s2 in enumerate(_ids[:_K]):
                    homo_s[si, j] = s2
                    homo_mask[si, j] = True
                _n += 1
        homo_s = homo_s.to(device)
        homo_mask = homo_mask.to(device)
        log(f"同音簇表(student): {_n} 锚字")

    # ── 同音簇局部蒸馏（咨询三 Q3: adv 离线教师, 只在同音字集合内 KL,
    #    隔离其词内过自信, 白嫖 +10pp 局部判别锐度）──
    teacher = None
    homo_t = None  # homo_s/homo_mask: v3.5 student 表可能已建, KD 激活
    # 时由下方覆盖为师生对表（--homo-ce 与 --kd-teacher 并用时受限 CE
    # 目标位列非自居——v3.5 不并用, 留坑注记）
    if args.kd_teacher and os.path.exists(args.kd_teacher):
        import struct as _st
        hdr_t, raw_t, vocab_t = read_export(args.kd_teacher)
        meta_t = hdr_t["__metadata__"]
        blob_t = 8 + _st.unpack("<Q", raw_t[:8])[0]

        def rd_t(n):
            e = hdr_t[n]
            b = raw_t[blob_t + e["data_offsets"][0]:
                      blob_t + e["data_offsets"][1]]
            return torch.frombuffer(bytearray(b), dtype=torch.float32
                                    ).clone()

        # 教师 GPT（按教师容器元数据建, 冻结）
        save = (N_LAYER, D, N_HEAD, HEAD_DIM, FFN, CTX,
                VARIANT, GELU_C, GELU_A)
        globals().update(
            N_LAYER=int(meta_t["n_layer"]), D=int(meta_t["d_model"]),
            N_HEAD=int(meta_t["n_heads"]),
            HEAD_DIM=int(meta_t["head_dim"]),
            CTX=int(meta_t["n_ctx"]), VARIANT=meta_t["variant"],
            GELU_C=float(meta_t["gelu_c"]),
            GELU_A=0.044715 if meta_t["variant"] == "advanced"
            else 0.044708252)
        globals()["FFN"] = 1024 if meta_t["variant"] == "advanced" else 768
        teacher = build_torch_model(len(vocab_t), device)
        sd_t = {"wte.weight": rd_t("wte").view(hdr_t["wte"]["shape"]),
                "wpe.weight": rd_t("wpe").view(hdr_t["wpe"]["shape"]),
                "ln_f.weight": rd_t("ln_f_g"), "ln_f.bias": rd_t("ln_f_b")}
        for li in range(N_LAYER):
            for src, dst in (("attn_w", "attn.qkv.weight"),
                             ("attn_b", "attn.qkv.bias"),
                             ("proj_w", "attn.proj.weight"),
                             ("proj_b", "attn.proj.bias"),
                             ("ln1_g", "ln1.weight"), ("ln1_b", "ln1.bias"),
                             ("ln2_g", "ln2.weight"), ("ln2_b", "ln2.bias"),
                             ("fc_w", "fc.weight"), ("fc_b", "fc.bias"),
                             ("pr2_w", "pr2.weight"), ("pr2_b", "pr2.bias")):
                k = f"blocks.{li}.{src}"
                sd_t[f"blocks.{li}.{dst}"] = rd_t(k).view(hdr_t[k]["shape"])
        teacher.load_state_dict(sd_t, strict=False)
        teacher.eval().to(device)
        for p in teacher.parameters():
            p.requires_grad_(False)
        globals().update(N_LAYER=save[0], D=save[1], N_HEAD=save[2],
                         HEAD_DIM=save[3], FFN=save[4], CTX=save[5],
                         VARIANT=save[6], GELU_C=save[7], GELU_A=save[8])
        log(f"KD 教师 {args.kd_teacher}（vocab {len(vocab_t)}）已冻结")

        # 同音簇表（学生 id → (学生 id, 教师 id) 对, 按字对齐跨词表）
        from pypinyin import lazy_pinyin, Style
        groups = {}
        for ch in vocab[1:]:
            if len(ch) == 1 and '一' <= ch <= '鿿':
                groups.setdefault(
                    lazy_pinyin(ch, style=Style.NORMAL)[0], []).append(ch)
        stoi_t = {c: i for i, c in enumerate(vocab_t)}
        K = 64
        hs = torch.full((len(vocab), K), 0, dtype=torch.long)
        ht = torch.full((len(vocab), K), 0, dtype=torch.long)
        hm = torch.zeros((len(vocab), K), dtype=torch.bool)
        n_cl = 0
        for py, chs in groups.items():
            if len(chs) < 2:
                continue
            pair = [(stoi[ch], stoi_t[ch]) for ch in chs
                    if ch in stoi_t]
            if len(pair) < 2:
                continue
            # 逐锚字填充: 该字所在簇的前 K 个 (学生, 教师) 对
            for si, _ in pair:
                for j, (s2, t2) in enumerate(pair[:K]):
                    hs[si, j] = s2
                    ht[si, j] = t2
                    hm[si, j] = True
                n_cl += 1
        homo_s, homo_t, homo_mask = (hs.to(device), ht.to(device),
                                     hm.to(device))
        log(f"同音簇表: {n_cl} 锚字（pypinyin 无调, K={K}）")


    opt = torch.optim.AdamW(m.parameters(), lr=args.lr,
                            weight_decay=0.01, betas=(0.9, 0.95))
    steps = args.steps or int(args.epochs * len(flat) /
                              (args.batch * args.seq))
    sched = torch.optim.lr_scheduler.OneCycleLR(
        opt, max_lr=args.lr, total_steps=max(steps, 1),
        pct_start=0.02)
    m.train()
    qat_start = steps - (args.qat_steps or max(steps * 15 // 100, 1)) \
        if args.qat else steps + 1
    undo_qat = None
    if qat_start <= 0:
        undo_qat = make_qat(args.qat)
        log(f"QAT int{args.qat} 全程启用")
    t0 = time.time()
    for step in range(steps):
        if step == qat_start:
            undo_qat = make_qat(args.qat)
            log(f"QAT int{args.qat} 末段启用 @step {step}")
        ix = np.random.randint(0, max(len(flat) - args.seq, 1),
                               size=args.batch)
        batch = np.stack([flat[i:i + args.seq] for i in ix])  # c0..c127
        # x = [BOS, c0..c126], y = c0..c127——位 t 前缀终于 c_{t-1},
        # 标签 c_t（对齐引擎 start_past n_past=1 语义; 曾错位 c_{t+1}）
        if args.sent_pack:
            # v3.5 打包批: [BOS|句1|BOS|句2|…] 位置重置 + 块对角因果
            xb = np.zeros((args.batch, args.seq), dtype=np.int64)
            yb = np.full((args.batch, args.seq), -1, dtype=np.int64)
            posb = np.zeros((args.batch, args.seq), dtype=np.int64)
            blkb = np.full((args.batch, args.seq), -1, dtype=np.int64)
            for r in range(args.batch):
                tl, bid = 0, 0
                while tl < args.seq:
                    sn = sent_pool[np.random.randint(len(sent_pool))]
                    L = len(sn) + 1
                    if tl + L > args.seq:
                        for _ in range(8):  # 试装短句填缝
                            s2 = sent_pool[np.random.randint(len(sent_pool))]
                            if tl + len(s2) + 1 <= args.seq:
                                sn, L = s2, len(s2) + 1
                                break
                        else:
                            break
                    xb[r, tl] = 0
                    xb[r, tl + 1:tl + L] = sn
                    posb[r, tl:tl + L] = np.arange(L)
                    blkb[r, tl:tl + L] = bid
                    yb[r, tl:tl + L - 1] = xb[r, tl + 1:tl + L]
                    tl += L
                    bid += 1
            x = torch.from_numpy(xb).to(device)
            y = torch.from_numpy(yb).to(device)
            pos = torch.from_numpy(posb).to(device)
            bi = torch.arange(args.batch, device=device)
            iu = torch.arange(args.seq, device=device)
            blkt = torch.from_numpy(blkb).to(device)
            mk = (blkt[:, :, None] == blkt[:, None, :]) & \
                 (blkt[:, :, None] >= 0) & \
                 (iu[None, None, :] <= iu[None, :, None])
            globals()["CUR_MASK"] = mk.unsqueeze(1)  # [B,1,T,T] 广播到头维
            logits = m(x, pos)
            globals()["CUR_MASK"] = None
            valid = y.reshape(-1) >= 0
            flat_lg = logits.reshape(-1, len(vocab))[valid]
            flat_y = y.reshape(-1)[valid]
            loss = torch.nn.functional.cross_entropy(
                flat_lg, flat_y, label_smoothing=args.ls)
        else:
            x = torch.from_numpy(
                np.concatenate([np.zeros((args.batch, 1), dtype=np.uint16),
                                batch[:, :-1]], axis=1).astype(np.int64)
            ).to(device)
            y = torch.from_numpy(batch.astype(np.int64)).to(device)
            logits = m(x)
            loss = torch.nn.functional.cross_entropy(
                logits.reshape(-1, len(vocab)), y.reshape(-1),
                label_smoothing=args.ls)
        if args.homo_ce > 0:
            # v3.5 同音簇约束 CE（PinyinGPT 式）: 目标字同音集合内
            # 局部 softmax, 强迫同音竞争位拉开边际
            fy = flat_y if args.sent_pack else y.reshape(-1)
            fl = (logits.reshape(-1, len(vocab))
                  if not args.sent_pack else flat_lg)
            hs_idx = homo_s[fy]
            hs_mk = homo_mask[fy]
            sub = fl.gather(1, hs_idx).masked_fill(~hs_mk, -1e9)
            has_cl = hs_mk.sum(dim=1) > 1
            if has_cl.any():
                lsr = torch.nn.functional.cross_entropy(
                    sub[has_cl],
                    torch.zeros(int(has_cl.sum()), dtype=torch.long,
                                device=fl.device))
                loss = loss + args.homo_ce * lsr
        if teacher is not None:
            with torch.no_grad():
                t_logits = teacher(x)
            fy = y.reshape(-1)
            s_sub = logits.reshape(-1, len(vocab)).gather(
                1, homo_s[fy]).masked_fill(~homo_mask[fy], -1e9)
            t_sub = t_logits.reshape(-1, len(vocab_t)).gather(
                1, homo_t[fy]).masked_fill(~homo_mask[fy], -1e9)
            has_cl = homo_mask[fy].any(dim=1) & \
                (homo_mask[fy].sum(dim=1) > 1)
            t_p = torch.softmax(t_sub, dim=1)
            s_lp = torch.log_softmax(s_sub, dim=1)
            kl = (t_p * (torch.log(t_p.clamp(min=1e-9)) - s_lp)
                  ).sum(dim=1)
            kd = (kl * has_cl).sum() / has_cl.sum().clamp(min=1)
            loss = loss + args.kd_alpha * kd
        opt.zero_grad(set_to_none=True)
        loss.backward()
        torch.nn.utils.clip_grad_norm_(m.parameters(), 1.0)
        opt.step()
        sched.step()
        if args.ckpt_every and args.save_ckpt and \
                step % args.ckpt_every == 0 and step > 0:
            import torch as _t
            _t.save({"sd": m.state_dict(), "vocab": vocab,
                     "variant": args.variant}, args.save_ckpt)
        if step % 100 == 0 or step == steps - 1:
            log(f"step {step}/{steps} loss {loss.item():.3f} "
                f"({(time.time() - t0) / max(step + 1, 1) * 1000:.0f}ms/step)")

    if undo_qat:
        undo_qat()
    if args.save_ckpt:
        import torch as _t
        _t.save({"sd": m.state_dict(), "vocab": vocab,
                 "variant": args.variant}, args.save_ckpt)
        log(f"checkpoint 落盘 {args.save_ckpt}")
    export(m, vocab, args.out, args.license_note)
    if args.fp16_out:
        export(m, vocab, args.fp16_out, args.license_note, fp16=True)
    if args.int8_out:
        export(m, vocab, args.int8_out, args.license_note, int8=True)
    log("完成——验收: hanhuo query <py> --nnlm " + args.out)


if __name__ == "__main__":
    main()
