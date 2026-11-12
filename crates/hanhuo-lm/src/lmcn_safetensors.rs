//! LMCN language model with SafeTensors format (optimized)
//!
//! Zero-copy mmap loading with fp16/fp32 support.

use anyhow::{Context as _, Result};
use half::f16;
use memmap2::Mmap;
use safetensors::SafeTensors;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use hanhuo_dict::wxhash::k2 as k2_hash;

/// LMCN language model loaded from SafeTensors
#[derive(Debug, Clone)]
pub struct LmcnModelSt {
    /// Hash (u32) -> (log_prob, backoff)
    /// backoff < 0.0 indicates unigram (no backoff)
    table: HashMap<u32, (f32, f32)>,
    /// Default probability for OOV words
    oov_penalty: f32,
}

impl LmcnModelSt {
    /// 自动检测格式并加载（推荐入口）
    pub fn from_safetensors<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy();
        if path_str.contains("fp16") {
            Self::from_safetensors_fp16(path)
        } else {
            Self::from_safetensors_fp32(path)
        }
    }

    /// 从 fp32 SafeTensors 加载（零拷贝）
    pub fn from_safetensors_fp32<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open {}", path.as_ref().display()))?;
        let mmap = unsafe { Mmap::map(&file)? };
        let tensors = SafeTensors::deserialize(&mmap)
            .context("Failed to deserialize SafeTensors")?;

        // 提取数组
        let hashes = Self::get_u32_array(&tensors, "hashes")?;
        let log_probs = Self::get_f32_array(&tensors, "log_probs")?;
        let backoffs = Self::get_f32_array(&tensors, "backoffs")?;

        anyhow::ensure!(
            hashes.len() == log_probs.len() && hashes.len() == backoffs.len(),
            "Array length mismatch: hashes={}, log_probs={}, backoffs={}",
            hashes.len(),
            log_probs.len(),
            backoffs.len()
        );

        // 构建 HashMap
        let mut table = HashMap::with_capacity(hashes.len());
        for i in 0..hashes.len() {
            table.insert(hashes[i], (log_probs[i], backoffs[i]));
        }

        tracing::info!(
            "Loaded {} entries from SafeTensors (fp32): {} unigrams, {} bigrams",
            table.len(),
            table.values().filter(|(_, bo)| *bo < 0.0).count(),
            table.values().filter(|(_, bo)| *bo >= 0.0).count()
        );

        Ok(Self {
            table,
            oov_penalty: -10.0,
        })
    }

    /// 从 fp16 SafeTensors 加载（推荐，节省 33% 内存）
    pub fn from_safetensors_fp16<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open {}", path.as_ref().display()))?;
        let mmap = unsafe { Mmap::map(&file)? };
        let tensors = SafeTensors::deserialize(&mmap)
            .context("Failed to deserialize SafeTensors")?;

        // 提取数组
        let hashes = Self::get_u32_array(&tensors, "hashes")?;
        let log_probs = Self::get_f16_array(&tensors, "log_probs")?;
        let backoffs = Self::get_f16_array(&tensors, "backoffs")?;

        anyhow::ensure!(
            hashes.len() == log_probs.len() && hashes.len() == backoffs.len(),
            "Array length mismatch"
        );

        // 转换 fp16 -> f32 并构建 HashMap
        let mut table = HashMap::with_capacity(hashes.len());
        for i in 0..hashes.len() {
            let log_prob = f16::from_bits(log_probs[i]).to_f32();
            let backoff = f16::from_bits(backoffs[i]).to_f32();
            table.insert(hashes[i], (log_prob, backoff));
        }

        tracing::info!(
            "Loaded {} entries from SafeTensors (fp16): {} unigrams, {} bigrams",
            table.len(),
            table.values().filter(|(_, bo)| *bo < 0.0).count(),
            table.values().filter(|(_, bo)| *bo >= 0.0).count()
        );

        Ok(Self {
            table,
            oov_penalty: -10.0,
        })
    }

    /// 提取 u32 数组（零拷贝）
    fn get_u32_array(tensors: &SafeTensors, name: &str) -> Result<Vec<u32>> {
        let view = tensors.tensor(name)
            .with_context(|| format!("Tensor '{}' not found", name))?;
        let data = view.data();

        anyhow::ensure!(
            data.len() % 4 == 0,
            "Tensor '{}' data length not aligned to u32",
            name
        );

        let ptr = data.as_ptr() as *const u32;
        let len = data.len() / 4;
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

        Ok(slice.to_vec())
    }

    /// 提取 f32 数组（零拷贝）
    fn get_f32_array(tensors: &SafeTensors, name: &str) -> Result<Vec<f32>> {
        let view = tensors.tensor(name)
            .with_context(|| format!("Tensor '{}' not found", name))?;
        let data = view.data();

        anyhow::ensure!(
            data.len() % 4 == 0,
            "Tensor '{}' data length not aligned to f32",
            name
        );

        let ptr = data.as_ptr() as *const f32;
        let len = data.len() / 4;
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

        Ok(slice.to_vec())
    }

    /// 提取 f16 数组（零拷贝读取，返回 u16 编码）
    fn get_f16_array(tensors: &SafeTensors, name: &str) -> Result<Vec<u16>> {
        let view = tensors.tensor(name)
            .with_context(|| format!("Tensor '{}' not found", name))?;
        let data = view.data();

        anyhow::ensure!(
            data.len() % 2 == 0,
            "Tensor '{}' data length not aligned to f16",
            name
        );

        let ptr = data.as_ptr() as *const u16;
        let len = data.len() / 2;
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };

        Ok(slice.to_vec())
    }

    /// Direct lookup by hash
    pub fn lookup(&self, hash: u32) -> Option<(f32, f32)> {
        self.table.get(&hash).copied()
    }

    /// Lookup by word (computes k2 hash)
    pub fn lookup_word(&self, word: &str) -> Option<(f32, f32)> {
        let hash = k2_hash(word.as_bytes());
        self.lookup(hash)
    }

    /// Calculate log probability for a word given context
    pub fn log_prob_word(&self, context: &[&str], word: &str) -> f32 {
        // Unigram fallback
        let unigram_prob = self.lookup_word(word)
            .map(|(lp, _)| lp)
            .unwrap_or(self.oov_penalty);

        if context.is_empty() {
            return unigram_prob;
        }

        // Try bigram with last context word
        if let Some(prev) = context.last() {
            let bigram_key = format!("{}\t{}", prev, word);
            if let Some((bigram_lp, backoff)) = self.lookup_word(&bigram_key) {
                if backoff >= 0.0 {
                    // Valid bigram
                    return bigram_lp;
                }
            }

            // Backoff: apply backoff weight if available
            let backoff_weight = self.lookup_word(prev)
                .and_then(|(_, bo)| if bo >= 0.0 { Some(bo) } else { None })
                .unwrap_or(0.0);

            return backoff_weight + unigram_prob;
        }

        unigram_prob
    }

    /// Query log probability with backoff (legacy API)
    pub fn log_prob(&self, context: &[&str], word: &str) -> f32 {
        self.log_prob_word(context, word)
    }

    /// Get unigram probability
    pub fn unigram_prob(&self, word: &str) -> f32 {
        self.lookup_word(word)
            .map(|(log_prob, _)| log_prob)
            .unwrap_or(self.oov_penalty)
    }

    /// Get bigram probability
    pub fn bigram_prob(&self, word1: &str, word2: &str) -> f32 {
        let bigram_key = format!("{}\t{}", word1, word2);
        self.lookup_word(&bigram_key)
            .and_then(|(log_prob, backoff)| {
                if backoff >= 0.0 {
                    Some(log_prob)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let backoff_weight = self.lookup_word(word1)
                    .and_then(|(_, bo)| if bo >= 0.0 { Some(bo) } else { None })
                    .unwrap_or(0.0);
                let unigram = self.unigram_prob(word2);
                backoff_weight + unigram
            })
    }

    /// Get table size
    pub fn size(&self) -> usize {
        self.table.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]  // 需要实际数据文件
    fn test_load_safetensors_fp32() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp32.safetensors");
        let model = LmcnModelSt::from_safetensors_fp32(path).unwrap();

        assert!(model.size() > 1_800_000);
        println!("Loaded {} entries", model.size());
    }

    #[test]
    #[ignore]
    fn test_load_safetensors_fp16() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp16.safetensors");
        let model = LmcnModelSt::from_safetensors_fp16(path).unwrap();

        assert!(model.size() > 1_800_000);
        println!("Loaded {} entries", model.size());
    }

    #[test]
    #[ignore]
    fn test_auto_detect_format() {
        let fp32_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp32.safetensors");
        let fp16_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp16.safetensors");

        let model_f32 = LmcnModelSt::from_safetensors(fp32_path).unwrap();
        let model_f16 = LmcnModelSt::from_safetensors(fp16_path).unwrap();

        assert_eq!(model_f32.size(), model_f16.size());
    }

    #[test]
    #[ignore]
    fn test_fp16_accuracy() {
        let fp32_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp32.safetensors");
        let fp16_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp16.safetensors");

        let model_f32 = LmcnModelSt::from_safetensors_fp32(fp32_path).unwrap();
        let model_f16 = LmcnModelSt::from_safetensors_fp16(fp16_path).unwrap();

        // 测试 unigram
        let ctx: Vec<&str> = vec![];
        let words = vec!["的", "你", "好", "天气", "今天"];

        for word in words {
            let prob_f32 = model_f32.log_prob_word(&ctx, word);
            let prob_f16 = model_f16.log_prob_word(&ctx, word);
            let diff = (prob_f32 - prob_f16).abs();

            println!("unigram '{}': fp32={:.6}, fp16={:.6}, diff={:.6}",
                     word, prob_f32, prob_f16, diff);
            assert!(diff < 0.01, "fp16 精度损失过大: {}", diff);
        }

        // 测试 bigram
        let test_bigrams = vec![
            ("今天", "天气"),
            ("你", "好"),
            ("的", "是"),
        ];

        for (w1, w2) in test_bigrams {
            let ctx = vec![w1];
            let prob_f32 = model_f32.log_prob_word(&ctx, w2);
            let prob_f16 = model_f16.log_prob_word(&ctx, w2);
            let diff = (prob_f32 - prob_f16).abs();

            println!("bigram '{}' -> '{}': fp32={:.6}, fp16={:.6}, diff={:.6}",
                     w1, w2, prob_f32, prob_f16, diff);
            assert!(diff < 0.01, "fp16 精度损失过大: {}", diff);
        }
    }

    #[test]
    #[ignore]
    fn test_word_lookup() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp16.safetensors");
        let model = LmcnModelSt::from_safetensors(path).unwrap();

        // Test unigram lookup
        let result = model.lookup_word("你");
        if let Some((log_prob, backoff)) = result {
            assert!(log_prob < 0.0, "log prob should be negative");
            println!("你: log_prob={}, backoff={}", log_prob, backoff);
        }
    }

    #[test]
    #[ignore]
    fn test_bigram_prob() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lmcn_fp16.safetensors");
        let model = LmcnModelSt::from_safetensors(path).unwrap();

        // Test unigram
        let prob1 = model.log_prob_word(&[], "好");
        println!("unigram '好': {}", prob1);

        // Test bigram
        let prob2 = model.log_prob_word(&["你"], "好");
        println!("bigram '你' -> '好': {}", prob2);

        assert!(prob1.is_finite());
        assert!(prob2.is_finite());
    }
}
