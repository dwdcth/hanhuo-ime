#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""train_p2c.py — 自训 P2C 拼音→汉字生成模型（长句召回战役主件）

配方（qwertium 调研档 §三, 青简含章·通变机制参考、代码 clean-room）:
  样本: <eos> 带噪拼音字母 <sep> 汉字 <eos> （样本流相接, 开头 eos 律）
  噪声: 干净全拼（pypinyin 无调）+ 键盘邻键替换/漏打/简拼截断/混合
  模型: GPT-2 式 decoder-only（pre-LN/fused QKV/MLP 4x gelu/可学习
        位置嵌入/tied embedding）; 冒烟 4×d384, 全量 12×d384≈23M
  推理: 前缀 <eos>+拼音+<sep> 自回归束生成（beam 5, max 32 字）
  纪律: 分只排名次不写回引擎分域（青简 P2C 分不可跨按键串比较）

语料: 复用 train_nnlm 管道（LCCC jsonl.gz + 文本文件逐行）。
用法:
  冒烟: python3 tools/train_p2c.py --smoke --steps 200 --out /tmp/p2c_smoke.safetensors
  训练: python3 tools/train_p2c.py --epochs 1 --out data/p2c_open.safetensors
许可: Apache-2.0; 产出权重随训语料许可（开源语料→可开源）。
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

try:
    import torch
    import torch.nn as nn
    import torch.nn.functional as F
except ImportError:
    print("需要 PyTorch: pip install torch --index-url https://download.pytorch.org/whl/cpu")
    sys.exit(1)

# ── 特殊位与字表 ──────────────────────────────────────────────
PAD, UNK, EOS, SEP = 0, 1, 2, 3
LETTERS = "abcdefghijklmnopqrstuvwxyz"

# QWERTY 邻键表（噪声注入用; typo_variants 同族）
NEIGH = {
    'q': 'wa', 'w': 'qes', 'e': 'wrd', 'r': 'etf', 't': 'ryg',
    'y': 'tuh', 'u': 'yij', 'i': 'uok', 'o': 'ipl', 'p': 'ol',
    'a': 'qsz', 's': 'awdxz', 'd': 'sefcx', 'f': 'drgv', 'g': 'fthb',
    'h': 'gyjn', 'j': 'hukm', 'k': 'jilm', 'l': 'kop',
    'z': 'asx', 'x': 'zsdc', 'c': 'xdfv', 'v': 'cfgb', 'b': 'vghn',
    'n': 'bhjm', 'm': 'njk',
}

def add_noise(py: str, rng: random.Random) -> str:
    """青简噪声配方: 邻键替换/漏打/简拼截断/混合（真实键入形态）。"""
    r = rng.random()
    if r < 0.30:
        return py  # 干净
    if r < 0.50:
        # 简拼截断: 按音节首字母（音节边界近似——元音起段切分）
        syls = split_syls(py)
        if len(syls) >= 2:
            return ''.join(s[0] for s in syls)
        return py
    if r < 0.70:
        # 漏打: 随机删 1-2 字母
        chars = list(py)
        for _ in range(min(2, max(1, len(chars) // 6))):
            if len(chars) > 2:
                chars.pop(rng.randrange(len(chars)))
        return ''.join(chars)
    if r < 0.90:
        # 邻键替换: 1-2 处
        chars = list(py)
        for _ in range(rng.choice([1, 1, 2])):
            i = rng.randrange(len(chars))
            nb = NEIGH.get(chars[i])
            if nb:
                chars[i] = rng.choice(nb)
        return ''.join(chars)
    # 混合: 前半全拼后半简拼
    syls = split_syls(py)
    if len(syls) >= 3:
        k = len(syls) // 2
        return ''.join(syls[:k]) + ''.join(s[0] for s in syls[k:])
    return py

_VOW = set('aeiouv')
def split_syls(py: str):
    """近似音节切分（噪声用——训练不需精确, 推理侧引擎供精确切分）。"""
    out, cur = [], ''
    prev_cons = False
    for ch in py:
        is_v = ch in _VOW
        if cur and prev_cons and not is_v:
            out.append(cur); cur = ch
        else:
            cur += ch
        prev_cons = not is_v
    if cur:
        out.append(cur)
    # 贪心修正: 过长段按常见音节库再切（简化: 保持近似）
    return out

def to_py(text: str) -> str:
    """汉字→无调全拼（非汉字/失败→空, 调用方跳过该样本）。"""
    try:
        from pypinyin import lazy_pinyin, Style
        parts = lazy_pinyin(text, style=Style.NORMAL,
                            errors=lambda x: [None] * len(x))
        if any(p is None for p in parts):
            return ''
        return ''.join(parts)
    except Exception:
        return ""

# ── 语料管道（复用 train_nnlm 清洗口径） ─────────────────────
def iter_corpus(paths, max_chars):
    total = 0
    for path in paths:
        if path.endswith('.jsonl.gz'):
            with gzip.open(path, 'rt', encoding='utf-8') as f:
                for line in f:
                    try:
                        turns = json.loads(line)
                    except Exception:
                        continue
                    for t in turns if isinstance(turns, list) else [turns]:
                        if isinstance(t, str):
                            yield t
                            total += len(t)
        else:
            with open(path, encoding='utf-8', errors='ignore') as f:
                for line in f:
                    yield line.rstrip('\n')
                    total += len(line)
        if total >= max_chars:
            return

def clean_line(s: str) -> str:
    s = re.sub(r'https?://\S+', '', s)
    s = re.sub(r'[\x00-\x1f]', '', s)
    # 只留汉字段（字母数字混合句留给后续英文域）
    s = re.sub(r'[^一-鿿]', '', s)
    return s if 4 <= len(s) <= 64 else ''

# ── 模型（GPT-2 式; 张量名对齐 zhiyi 风格方便 candle 侧复用经验） ──
class Block(nn.Module):
    def __init__(self, d, nh):
        super().__init__()
        self.ln1 = nn.LayerNorm(d)
        self.qkv = nn.Linear(d, 3 * d, bias=False)
        self.proj = nn.Linear(d, d, bias=False)
        self.ln2 = nn.LayerNorm(d)
        self.fc = nn.Linear(d, 4 * d, bias=False)
        self.proj2 = nn.Linear(4 * d, d, bias=False)
        self.nh = nh

    def forward(self, x):
        B, T, C = x.shape
        h = self.ln1(x)
        q, k, v = self.qkv(h).split(C, dim=2)
        q = q.view(B, T, self.nh, C // self.nh).transpose(1, 2)
        k = k.view(B, T, self.nh, C // self.nh).transpose(1, 2)
        v = v.view(B, T, self.nh, C // self.nh).transpose(1, 2)
        att = (q @ k.transpose(-2, -1)) / math.sqrt(C // self.nh)
        mask = torch.tril(torch.ones(T, T, dtype=torch.bool, device=x.device))
        att = att.masked_fill(~mask, float('-inf'))
        y = (F.softmax(att, dim=-1) @ v)
        y = y.transpose(1, 2).contiguous().view(B, T, C)
        x = x + self.proj(y)
        h = self.ln2(x)
        x = x + self.proj2(F.gelu(self.fc(h)))
        return x

class P2C(nn.Module):
    def __init__(self, vocab, d=384, n_layer=4, nh=6, ctx=192):
        super().__init__()
        self.tok = nn.Embedding(vocab, d)
        self.pos = nn.Embedding(ctx, d)
        self.blocks = nn.ModuleList([Block(d, nh) for _ in range(n_layer)])
        self.ln_f = nn.LayerNorm(d)
        self.head = nn.Linear(d, vocab, bias=False)
        self.head.weight = self.tok.weight  # tied
        self.ctx = ctx

    def forward(self, idx):
        B, T = idx.shape
        pos = torch.arange(T, device=idx.device)
        x = self.tok(idx) + self.pos(pos)
        for b in self.blocks:
            x = b(x)
        return self.head(self.ln_f(x))

# ── 样本构造 ────────────────────────────────────────────────
def build_samples(lines, rng, pypinyin_ok, max_samples):
    """每行 → (带噪拼音 token 序列 + <sep> + 汉字 token 序列 + <eos>)。"""
    out = []
    for text in lines:
        if len(out) >= max_samples:
            break
        py = to_py(text) if pypinyin_ok else ''
        if not py or not (4 <= len(text) <= 32):
            continue
        noisy = add_noise(py, rng)
        if not noisy or len(noisy) > 96:
            continue
        out.append((noisy, text))
    return out

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--corpus', nargs='*', default=[])
    ap.add_argument('--epochs', type=float, default=1.0)
    ap.add_argument('--steps', type=int, default=0)
    ap.add_argument('--batch', type=int, default=32)
    ap.add_argument('--ctx', type=int, default=192)
    ap.add_argument('--d', type=int, default=384)
    ap.add_argument('--n-layer', type=int, default=4)
    ap.add_argument('--lr', type=float, default=3e-4)
    ap.add_argument('--max-chars', type=int, default=50_000_000)
    ap.add_argument('--max-samples', type=int, default=2_000_000)
    ap.add_argument('--smoke', action='store_true')
    ap.add_argument('--out', default='data/p2c_open.safetensors')
    args = ap.parse_args()
    if args.smoke:
        args.max_chars, args.max_samples = 2_000_000, 20_000
        args.steps, args.epochs = max(args.steps, 200), 0
    rng = random.Random(42)
    torch.manual_seed(42)

    try:
        import pypinyin  # noqa
        pypinyin_ok = True
    except ImportError:
        pypinyin_ok = False
        print("[warn] pypinyin 缺席——干净样本不可造, 仅噪声域退化训练")

    # 语料
    print(f"[corpus] 装载 {args.corpus} (max {args.max_chars} 字符)...")
    lines = []
    for raw in iter_corpus(args.corpus, args.max_chars):
        c = clean_line(raw)
        if c:
            lines.append(c)
    print(f"[corpus] {len(lines)} 行清洗后")
    samples = build_samples(lines, rng, pypinyin_ok, args.max_samples)
    print(f"[data] {len(samples)} 样本（带噪拼音↔汉字）")

    # 字表: 汉字高频 + 26 字母 + 4 特殊
    from collections import Counter
    cnt = Counter()
    for _, text in samples:
        cnt.update(text)
    chars = [c for c, _ in cnt.most_common(8105)]
    vocab_list = ['<pad>', '<unk>', '<eos>', '<sep>'] + list(LETTERS) + chars
    stoi = {c: i for i, c in enumerate(vocab_list)}
    V = len(vocab_list)
    print(f"[vocab] {V}（{len(chars)} 汉字 + 26 字母 + 4 特殊）")

    # 批流: 样本拼接成 ctx 长块（<eos> 样本相接——青简流式律）
    def encode_pair(noisy, text):
        ids = [EOS] + [stoi.get(c, UNK) for c in noisy] + [SEP] \
            + [stoi.get(c, UNK) for c in text] + [EOS]
        return ids

    stream = []
    for noisy, text in samples:
        stream.extend(encode_pair(noisy, text))
    print(f"[stream] {len(stream)} token")

    dev = 'cuda' if torch.cuda.is_available() else 'cpu'
    model = P2C(V, d=args.d, n_layer=args.n_layer, ctx=args.ctx).to(dev)
    n_param = sum(p.numel() for p in model.parameters())
    print(f"[model] {args.n_layer}×d{args.d} ≈{n_param/1e6:.1f}M @ {dev}")

    opt = torch.optim.AdamW(model.parameters(), lr=args.lr)
    step_target = args.steps or int(len(stream) // (args.batch * args.ctx) * max(args.epochs, 1))
    losses = []
    t0 = time.time()
    for step in range(step_target):
        xs, ys = [], []
        for _ in range(args.batch):
            i = rng.randrange(0, max(1, len(stream) - args.ctx - 1))
            chunk = stream[i:i + args.ctx + 1]
            xs.append(chunk[:args.ctx])
            ys.append(chunk[1:args.ctx + 1])
        x = torch.tensor(xs, dtype=torch.long, device=dev)
        y = torch.tensor(ys, dtype=torch.long, device=dev)
        logits = model(x)
        loss = F.cross_entropy(logits.view(-1, V), y.view(-1), ignore_index=PAD)
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        opt.step()
        losses.append(loss.item())
        if step % 50 == 0 or step == step_target - 1:
            print(f"[{step}/{step_target}] loss {sum(losses[-50:])/min(50,len(losses)):.4f} "
                  f"({time.time()-t0:.0f}s)")

    # 导出 safetensors（张量名 p2c.* 契约 + config）
    try:
        from safetensors.torch import save_file
    except ImportError:
        print("pip install safetensors"); sys.exit(1)
    sd = {f"tok_emb.weight": model.tok.weight.detach().cpu().contiguous(),
          f"pos_emb.weight": model.pos.weight.detach().cpu().contiguous(),
          f"ln_f.weight": model.ln_f.weight.detach().cpu(),
          f"ln_f.bias": model.ln_f.bias.detach().cpu()}
    for i, b in enumerate(model.blocks):
        sd[f"blocks.{i}.ln1.weight"] = b.ln1.weight.detach().cpu()
        sd[f"blocks.{i}.ln1.bias"] = b.ln1.bias.detach().cpu()
        sd[f"blocks.{i}.qkv.weight"] = b.qkv.weight.detach().cpu().contiguous()
        sd[f"blocks.{i}.proj.weight"] = b.proj.weight.detach().cpu()
        sd[f"blocks.{i}.ln2.weight"] = b.ln2.weight.detach().cpu()
        sd[f"blocks.{i}.ln2.bias"] = b.ln2.bias.detach().cpu()
        sd[f"blocks.{i}.fc.weight"] = b.fc.weight.detach().cpu()
        sd[f"blocks.{i}.proj2.weight"] = b.proj2.weight.detach().cpu()
    cfg = {"vocab_size": V, "d": args.d, "n_layer": args.n_layer,
           "n_head": 6, "ctx": args.ctx, "variant": "p2c-open",
           "license_note": "自训; 语料开源(LCCC等), 权重可开源",
           "vocab": vocab_list}
    tmp = args.out + ".tmp"
    sd["__config__"] = torch.tensor([0])  # 占位（cfg 走 json 伴文件）
    save_file(sd, tmp)
    with open(args.out + ".json", 'w', encoding='utf-8') as f:
        json.dump(cfg, f, ensure_ascii=False)
    os.replace(tmp, args.out)
    print(f"[out] {args.out} (+.json config, {os.path.getsize(args.out)/1e6:.1f}MB)")

if __name__ == '__main__':
    main()
