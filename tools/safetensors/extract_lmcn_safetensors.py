#!/usr/bin/env python3
"""提取 LMCN 语言模型为 SafeTensors 格式

从 lmcn_full.npz 提取 hash 表数据，生成 fp32 和 fp16 两个版本。

输入格式:
  h: [N] uint64 - hash keys (lower 32 bits used)
  v1: [N] uint16 - encoded log_prob
  v2: [N] int32 - encoded backoff

输出格式:
  hashes: [N] uint32 - hash keys
  log_probs: [N] float32/float16 - decoded log probabilities
  backoffs: [N] float32/float16 - decoded backoff weights

用法:
    python extract_lmcn_safetensors.py <input.npz> <output_dir>
"""

import sys
import numpy as np
from pathlib import Path
from safetensors.numpy import save_file


def decode_lmcn(h, v1, v2):
    """解码 LMCN 数据格式

    Returns:
        hashes: uint32 array
        log_probs: float32 array
        backoffs: float32 array
    """
    # Hash: 使用低 32 位
    hashes = (h & 0xFFFFFFFF).astype(np.uint32)

    # Log probability: -(v1 - 50000) / 1000
    log_probs = -((v1.astype(np.float32) - 50000.0) / 1000.0)

    # Backoff: -1.0 for unigram (v2 < 0), else -(v2 - 50000) / 1000
    backoffs = np.where(
        v2 < 0,
        -1.0,
        np.maximum(-((v2.astype(np.float32) - 50000.0) / 1000.0), -20.0)
    )

    return hashes, log_probs, backoffs


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <input.npz> <output_dir>")
        sys.exit(1)

    input_path = Path(sys.argv[1])
    output_dir = Path(sys.argv[2])
    output_dir.mkdir(parents=True, exist_ok=True)

    if not input_path.exists():
        print(f"Error: {input_path} not found")
        sys.exit(1)

    print(f"Loading {input_path}...")
    data = np.load(input_path)

    # 提取原始数组
    h = data['h']
    v1 = data['v1']
    v2 = data['v2']

    print(f"Loaded {len(h)} entries")
    print(f"  h: {h.shape} {h.dtype}")
    print(f"  v1: {v1.shape} {v1.dtype}")
    print(f"  v2: {v2.shape} {v2.dtype}")

    # 解码
    print("\nDecoding...")
    hashes, log_probs, backoffs = decode_lmcn(h, v1, v2)

    # 统计
    n_unigrams = np.sum(backoffs < 0.0)
    n_bigrams = len(hashes) - n_unigrams
    print(f"  Unigrams: {n_unigrams:,}")
    print(f"  Bigrams: {n_bigrams:,}")
    print(f"  Total: {len(hashes):,}")

    # 保存 fp32 版本
    print("\nSaving fp32...")
    fp32_path = output_dir / "lmcn_fp32.safetensors"
    save_file({
        'hashes': hashes,
        'log_probs': log_probs,
        'backoffs': backoffs,
    }, str(fp32_path))
    size_fp32 = fp32_path.stat().st_size / (1024 * 1024)
    print(f"  {fp32_path.name}: {size_fp32:.2f} MB")

    # 保存 fp16 版本
    print("\nSaving fp16...")
    fp16_path = output_dir / "lmcn_fp16.safetensors"
    save_file({
        'hashes': hashes,  # u32 保持
        'log_probs': log_probs.astype(np.float16),
        'backoffs': backoffs.astype(np.float16),
    }, str(fp16_path))
    size_fp16 = fp16_path.stat().st_size / (1024 * 1024)
    print(f"  {fp16_path.name}: {size_fp16:.2f} MB")

    # 总结
    savings = (1 - size_fp16 / size_fp32) * 100
    print(f"\nSummary:")
    print(f"  Entries: {len(hashes):,}")
    print(f"  Unigrams: {n_unigrams:,} ({n_unigrams/len(hashes)*100:.1f}%)")
    print(f"  Bigrams: {n_bigrams:,} ({n_bigrams/len(hashes)*100:.1f}%)")
    print(f"  fp32 size: {size_fp32:.2f} MB")
    print(f"  fp16 size: {size_fp16:.2f} MB")
    print(f"  Savings: {savings:.1f}%")

    print(f"\nOutput files:")
    print(f"  {fp32_path}")
    print(f"  {fp16_path}")


if __name__ == '__main__':
    main()
