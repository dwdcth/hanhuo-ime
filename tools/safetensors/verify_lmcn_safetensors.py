#!/usr/bin/env python3
"""验证 LMCN SafeTensors 实施成果"""

import numpy as np
from pathlib import Path
from safetensors import safe_open

def check_file_sizes():
    """检查文件大小"""
    print("=" * 60)
    print("文件大小验证")
    print("=" * 60)

    base = Path("hanhuo-engine/data")
    files = [
        ("lmcn_fp32.safetensors", "fp32 SafeTensors"),
        ("lmcn_fp16.safetensors", "fp16 SafeTensors"),
    ]

    sizes = {}
    for fname, desc in files:
        path = base / fname
        if path.exists():
            size_mb = path.stat().st_size / (1024 * 1024)
            sizes[fname] = size_mb
            print(f"  {desc:20s}: {size_mb:6.2f} MB - {path}")
        else:
            print(f"  {desc:20s}: 未找到")

    if "lmcn_fp32.safetensors" in sizes and "lmcn_fp16.safetensors" in sizes:
        savings = (1 - sizes["lmcn_fp16.safetensors"] / sizes["lmcn_fp32.safetensors"]) * 100
        print(f"\n  节省空间: {savings:.1f}%")
    print()

def check_data_integrity():
    """检查数据完整性"""
    print("=" * 60)
    print("数据完整性验证")
    print("=" * 60)

    base = Path("hanhuo-engine/data")

    # 检查 fp32
    fp32_path = base / "lmcn_fp32.safetensors"
    if fp32_path.exists():
        with safe_open(fp32_path, framework="numpy") as f:
            hashes = f.get_tensor("hashes")
            log_probs = f.get_tensor("log_probs")
            backoffs = f.get_tensor("backoffs")

            n_entries = len(hashes)
            n_unigrams = np.sum(backoffs < 0)
            n_bigrams = n_entries - n_unigrams

            print(f"fp32 格式:")
            print(f"  总条目: {n_entries:,}")
            print(f"  Unigrams: {n_unigrams:,} ({n_unigrams/n_entries*100:.1f}%)")
            print(f"  Bigrams: {n_bigrams:,} ({n_bigrams/n_entries*100:.1f}%)")
            print(f"  Hashes dtype: {hashes.dtype}")
            print(f"  Log_probs dtype: {log_probs.dtype}")
            print(f"  Backoffs dtype: {backoffs.dtype}")
            print(f"  Log_probs range: [{log_probs.min():.2f}, {log_probs.max():.2f}]")
            print(f"  Backoffs range: [{backoffs.min():.2f}, {backoffs.max():.2f}]")

    print()

    # 检查 fp16
    fp16_path = base / "lmcn_fp16.safetensors"
    if fp16_path.exists():
        with safe_open(fp16_path, framework="numpy") as f:
            hashes = f.get_tensor("hashes")
            log_probs = f.get_tensor("log_probs")
            backoffs = f.get_tensor("backoffs")

            # 转换 fp16 -> fp32 用于统计
            log_probs_f32 = log_probs.astype(np.float32)
            backoffs_f32 = backoffs.astype(np.float32)

            n_entries = len(hashes)
            n_unigrams = np.sum(backoffs_f32 < 0)
            n_bigrams = n_entries - n_unigrams

            print(f"fp16 格式:")
            print(f"  总条目: {n_entries:,}")
            print(f"  Unigrams: {n_unigrams:,} ({n_unigrams/n_entries*100:.1f}%)")
            print(f"  Bigrams: {n_bigrams:,} ({n_bigrams/n_entries*100:.1f}%)")
            print(f"  Hashes dtype: {hashes.dtype}")
            print(f"  Log_probs dtype: {log_probs.dtype} (存储), float32 (推理)")
            print(f"  Backoffs dtype: {backoffs.dtype} (存储), float32 (推理)")
            print(f"  Log_probs range: [{log_probs_f32.min():.2f}, {log_probs_f32.max():.2f}]")
            print(f"  Backoffs range: [{backoffs_f32.min():.2f}, {backoffs_f32.max():.2f}]")

    print()

def check_accuracy():
    """检查 fp16 精度"""
    print("=" * 60)
    print("精度对比验证")
    print("=" * 60)

    base = Path("hanhuo-engine/data")
    fp32_path = base / "lmcn_fp32.safetensors"
    fp16_path = base / "lmcn_fp16.safetensors"

    if not (fp32_path.exists() and fp16_path.exists()):
        print("  文件不存在，跳过精度验证")
        return

    with safe_open(fp32_path, framework="numpy") as f32:
        log_probs_f32 = f32.get_tensor("log_probs")
        backoffs_f32 = f32.get_tensor("backoffs")

    with safe_open(fp16_path, framework="numpy") as f16:
        log_probs_f16 = f16.get_tensor("log_probs").astype(np.float32)
        backoffs_f16 = f16.get_tensor("backoffs").astype(np.float32)

    # 计算差异
    log_prob_diff = np.abs(log_probs_f32 - log_probs_f16)
    backoff_diff = np.abs(backoffs_f32 - backoffs_f16)

    print("Log Probabilities:")
    print(f"  最大差异: {log_prob_diff.max():.6f}")
    print(f"  平均差异: {log_prob_diff.mean():.6f}")
    print(f"  中位差异: {np.median(log_prob_diff):.6f}")
    print(f"  95% 分位: {np.percentile(log_prob_diff, 95):.6f}")
    print(f"  差异 < 0.001: {np.sum(log_prob_diff < 0.001) / len(log_prob_diff) * 100:.2f}%")
    print(f"  差异 < 0.01: {np.sum(log_prob_diff < 0.01) / len(log_prob_diff) * 100:.2f}%")

    print("\nBackoffs:")
    print(f"  最大差异: {backoff_diff.max():.6f}")
    print(f"  平均差异: {backoff_diff.mean():.6f}")
    print(f"  中位差异: {np.median(backoff_diff):.6f}")
    print(f"  95% 分位: {np.percentile(backoff_diff, 95):.6f}")
    print(f"  差异 < 0.001: {np.sum(backoff_diff < 0.001) / len(backoff_diff) * 100:.2f}%")
    print(f"  差异 < 0.01: {np.sum(backoff_diff < 0.01) / len(backoff_diff) * 100:.2f}%")

    print()

def main():
    print("\n" + "=" * 60)
    print("LMCN SafeTensors 优化成果验收")
    print("=" * 60)
    print()

    check_file_sizes()
    check_data_integrity()
    check_accuracy()

    print("=" * 60)
    print("验收完成 ✓")
    print("=" * 60)

if __name__ == "__main__":
    main()
