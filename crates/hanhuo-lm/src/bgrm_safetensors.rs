//! BGRM 词 bigram 统计语言模型——SafeTensors 开放格式（hanhuo-bgrm v1）.
//!
//! 非神经模型: 整型三元组表 (w1, w2, val), w1/w2 = 主包词层词 id
//! （强耦合——注入文件必须与词库包同源 wxq）。查询语义在引擎侧
//! `Engine::bigram_after`（w1 有序带 searchsorted）, 本模块只负责
//! 开放格式装载 + 段体位精确还原（into_native → Engine::set_bgrm）。
//!
//! 整型表天然位精确——无 fp16 变体（对照 nnlm/mdltr 双精度惯例）。
use anyhow::{Context as _, Result};
use safetensors::SafeTensors;

/// 开放格式装载态（保留张量引用, into_native 时拼段体）
pub struct BgrmOpen {
    n: usize,
    w1: Vec<u32>,
    w2: Vec<u32>,
    val: Vec<u16>,
}

impl BgrmOpen {
    /// 从 hanhuo-bgrm v1 safetensors 装载（w1/w2 U32 + val U16 三张量;
    /// dtype 即格式契约——整型表无精度变体）
    pub fn from_safetensors(bytes: &[u8]) -> Result<Self> {
        let st = SafeTensors::deserialize(bytes)
            .map_err(|e| anyhow::anyhow!("bgrm safetensors 反序列化: {e}"))?;
        let get = |name: &str, want: safetensors::Dtype| -> Result<&[u8]> {
            let t = st
                .tensor(name)
                .map_err(|e| anyhow::anyhow!("缺张量 {name}: {e}"))?;
            anyhow::ensure!(
                t.dtype() == want,
                "{name} dtype {:?} ≠ {want:?}",
                t.dtype()
            );
            Ok(t.data())
        };
        let w1 = get("w1", safetensors::Dtype::U32)?;
        let w2 = get("w2", safetensors::Dtype::U32)?;
        let val = get("val", safetensors::Dtype::U16)?;
        anyhow::ensure!(
            w1.len() == w2.len() && w1.len() == val.len() * 2,
            "张量长度不一致: w1={} w2={} val×2={}",
            w1.len(),
            w2.len(),
            val.len() * 2
        );
        let n = w1.len() / 4;
        let le_u32 = |b: &[u8], i: usize| {
            u32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]])
        };
        let le_u16 = |b: &[u8], i: usize| {
            u16::from_le_bytes([b[2 * i], b[2 * i + 1]])
        };
        Ok(BgrmOpen {
            n,
            w1: (0..n).map(|i| le_u32(w1, i)).collect(),
            w2: (0..n).map(|i| le_u32(w2, i)).collect(),
            val: (0..n).map(|i| le_u16(val, i)).collect(),
        })
    }

    /// 还原为 BGRM 段体布局 `[n u32][w1×n][w2×n][val u16×n]`
    /// （与 wxq.bin 原段逐字节等价——Engine::set_bgrm 的输入）
    pub fn into_native(self) -> Vec<u8> {
        let n = self.n;
        let mut out = Vec::with_capacity(4 + 10 * n);
        out.extend_from_slice(&(n as u32).to_le_bytes());
        for v in &self.w1 {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.w2 {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.val {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    pub fn len(&self) -> usize {
        self.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgrm_synth_roundtrip_layout() {
        // 合成 safetensors（safetensors crate 序列化）→ 装载 → 段体
        // 布局锁: [n][w1][w2][val] 逐字节
        use safetensors::tensor::{serialize, TensorView};
        use std::collections::HashMap;
        let w1: Vec<u32> = vec![5, 5, 7];
        let w2: Vec<u32> = vec![9, 12, 3];
        let val: Vec<u16> = vec![100, 200, 300];
        let le = |xs: &Vec<u32>| xs.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>();
        let b1 = le(&w1);
        let b2 = le(&w2);
        let b3: Vec<u8> = val.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut m: HashMap<&str, TensorView> = HashMap::new();
        m.insert(
            "w1",
            TensorView::new(safetensors::Dtype::U32, vec![3], &b1).unwrap(),
        );
        m.insert(
            "w2",
            TensorView::new(safetensors::Dtype::U32, vec![3], &b2).unwrap(),
        );
        m.insert(
            "val",
            TensorView::new(safetensors::Dtype::U16, vec![3], &b3).unwrap(),
        );
        let meta = HashMap::from([(
            "format".to_string(),
            "hanhuo-bgrm".to_string(),
        )]);
        let bytes = serialize(m, &Some(meta)).unwrap();
        let m = BgrmOpen::from_safetensors(&bytes).unwrap();
        assert_eq!(m.len(), 3);
        let nat = m.into_native();
        let expect: Vec<u8> = {
            let mut v = vec![3u8, 0, 0, 0];
            v.extend(b1);
            v.extend(b2);
            v.extend(b3);
            v
        };
        assert_eq!(nat, expect, "段体布局逐字节");
    }

    /// 真机往返锁: data/bgrm.safetensors → into_native == wxq.bin 原段体
    #[test]
    #[ignore = "真机 wxq.bin + data/bgrm.safetensors"]
    fn bgrm_real_roundtrip_bit_exact() {
        let st = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/bgrm.safetensors"),
        )
        .unwrap();
        let native = BgrmOpen::from_safetensors(&st).unwrap().into_native();
        let home = std::env::var("HOME").unwrap();
        let pack = std::fs::read(format!("{home}/tmp/hanhuo/wxq.bin")).unwrap();
        // 段目录定位 BGRM
        let n = u32::from_le_bytes(pack[8..12].try_into().unwrap()) as usize;
        let mut off = None;
        for i in 0..n {
            let s = 16 + i * 12;
            if &pack[s..s + 4] == b"BGRM" {
                let o = u32::from_le_bytes(pack[s + 4..s + 8].try_into().unwrap()) as usize;
                let l = u32::from_le_bytes(pack[s + 8..s + 12].try_into().unwrap()) as usize;
                off = Some((o, l));
                break;
            }
        }
        let (o, l) = off.expect("wxq.bin 有 BGRM 段");
        assert_eq!(native, &pack[o..o + l], "safetensors 往返段体位精确");
    }
}
