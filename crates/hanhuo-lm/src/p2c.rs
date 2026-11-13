//! 自训 P2C 拼音→汉字生成模型推理（candle 纯 Rust）。
//!
//! 权重契约 = tools/train_p2c.py 导出（safetensors + json config）:
//!   tok_emb/pos_emb/blocks.N.{ln1,qkv,proj,ln2,fc,proj2}/ln_f.*
//!   （head=tied → logits = ln_f(x)·tok_embᵀ）
//! 生成: 前缀 <eos>+拼音字母+<sep> 自回归束（beam 5, max 32 字）;
//! 纪律（青简律）: 分只用于候选排名, 不写回引擎分域。

use candle_core::{DType, Device, Tensor};

pub struct P2cModel {
    tok_emb: Tensor,  // [V, d]
    pos_emb: Tensor,  // [ctx, d]
    blocks: Vec<P2cBlock>,
    ln_f_w: Tensor,
    ln_f_b: Tensor,
    pub vocab: Vec<String>, // token 串（特殊位/字母/汉字）
    stoi: std::collections::HashMap<String, u32>,
    pub d: usize,
    n_head: usize,
    ctx: usize,
}

struct P2cBlock {
    ln1_w: Tensor, ln1_b: Tensor,
    qkv: Tensor,     // [3d, d]
    proj: Tensor,    // [d, d]
    ln2_w: Tensor, ln2_b: Tensor,
    fc: Tensor,      // [4d, d]
    proj2: Tensor,   // [d, 4d]
}

pub const EOS: u32 = 2;
pub const SEP: u32 = 3;

impl P2cModel {
    pub fn open(stem: &std::path::Path) -> candle_core::Result<Self> {
        use safetensors::SafeTensors;
        let raw = std::fs::read(stem.with_extension("safetensors"))?;
        let st = SafeTensors::deserialize(&raw)
            .map_err(candle_core::Error::wrap)?;
        // 训练导出 config = <out>.safetensors.json（双扩展）; 兼容
        // <stem>.json 直拼两形态
        let cfg_path = stem.with_extension("safetensors")
            .with_extension("safetensors.json");
        let cfg_txt = std::fs::read_to_string(
            if cfg_path.exists() {
                cfg_path
            } else {
                stem.with_extension("json")
            },
        )
        .map_err(|e| candle_core::Error::Msg(format!("config 读失败: {e}")))?;
        let cfg: serde_json::Value = serde_json::from_str(&cfg_txt)
            .map_err(|e| candle_core::Error::Msg(format!("config 解析: {e}")))?;
        let d = cfg["d"].as_u64().unwrap_or(384) as usize;
        let n_layer =
            cfg["n_layer"].as_u64().unwrap_or(8) as usize;
        let n_head = cfg["n_head"].as_u64().unwrap_or(6) as usize;
        let ctx = cfg["ctx"].as_u64().unwrap_or(192) as usize;
        let vocab: Vec<String> = cfg["vocab"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|x| x.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let dev = Device::Cpu;
        let f32t = |name: &str| -> candle_core::Result<Tensor> {
            let v = st.tensor(name).map_err(candle_core::Error::wrap)?;
            let t = Tensor::from_raw_buffer(
                v.data(),
                DType::F32,
                &v.shape(),
                &dev,
            )?;
            t.to_dtype(DType::F32)
        };
        let mut blocks = Vec::new();
        for n in 0..n_layer {
            blocks.push(P2cBlock {
                ln1_w: f32t(&format!("blocks.{n}.ln1.weight"))?,
                ln1_b: f32t(&format!("blocks.{n}.ln1.bias"))?,
                qkv: f32t(&format!("blocks.{n}.qkv.weight"))?,
                proj: f32t(&format!("blocks.{n}.proj.weight"))?,
                ln2_w: f32t(&format!("blocks.{n}.ln2.weight"))?,
                ln2_b: f32t(&format!("blocks.{n}.ln2.bias"))?,
                fc: f32t(&format!("blocks.{n}.fc.weight"))?,
                proj2: f32t(&format!("blocks.{n}.proj2.weight"))?,
            });
        }
        let mut stoi = std::collections::HashMap::new();
        for (i, t) in vocab.iter().enumerate() {
            stoi.insert(t.clone(), i as u32);
        }
        Ok(Self {
            tok_emb: f32t("tok_emb.weight")?,
            pos_emb: f32t("pos_emb.weight")?,
            blocks,
            ln_f_w: f32t("ln_f.weight")?,
            ln_f_b: f32t("ln_f.bias")?,
            vocab,
            stoi,
            d,
            n_head,
            ctx,
        })
    }

    fn ln(
        &self,
        x: &Tensor,
        w: &Tensor,
        b: &Tensor,
    ) -> candle_core::Result<Tensor> {
        candle_nn::ops::layer_norm(x, w, b, 1e-5)
    }

    /// 单序列前向（[seq]→logits [seq, V]; 朴素全算——ctx≤192 评测评度）。
    pub fn forward(&self, ids: &[u32]) -> candle_core::Result<Tensor> {
        let dev = Device::Cpu;
        let seq = ids.len().min(self.ctx);
        let ids = &ids[..seq];
        let ids_t =
            Tensor::from_vec(ids.to_vec(), seq, &dev)?.to_dtype(DType::U32)?;
        let mut x = self.tok_emb.index_select(&ids_t, 0)?; // [seq, d]
        let pos_ids = Tensor::arange(0u32, seq as u32, &dev)?;
        let pos = self.pos_emb.index_select(&pos_ids, 0)?;
        x = (x + pos)?;
        let (d, nh) = (self.d, self.n_head);
        let hd = d / nh;
        for b in &self.blocks {
            let h = self.ln(&x, &b.ln1_w, &b.ln1_b)?;
            let qkv = h.matmul(&b.qkv.t()?)?; // [seq, 3d]
            let parts = qkv.chunk(3, 1)?;
            let mut heads = Vec::new();
            for hi in 0..nh {
                let q = parts[0].narrow(1, hi * hd, hd)?;
                let k = parts[1].narrow(1, hi * hd, hd)?;
                let v = parts[2].narrow(1, hi * hd, hd)?;
                let mut sc = q.matmul(&k.t()?)?;
                sc = sc.affine(1.0 / (hd as f64).sqrt(), 0.0)?;
                // 因果掩码: 上三角 -inf
                let mut m = Vec::with_capacity(seq * seq);
                for i in 0..seq {
                    for j in 0..seq {
                        m.push(if j > i {
                            f32::NEG_INFINITY
                        } else {
                            0.0
                        });
                    }
                }
                let mask =
                    Tensor::from_vec(m, (seq, seq), &dev)?;
                let sc = (sc + mask)?;
                let w = candle_nn::ops::softmax_last_dim(&sc)?;
                heads.push(w.matmul(&v)?);
            }
            let cat = Tensor::cat(&heads, 1)?;
            let att = cat.matmul(&b.proj.t()?)?;
            x = (x + att)?;
            let h2 = self.ln(&x, &b.ln2_w, &b.ln2_b)?;
            let ff = h2.matmul(&b.fc.t()?)?;
            let ff = ff.gelu_erf()?;
            let ff = ff.matmul(&b.proj2.t()?)?;
            x = (x + ff)?;
        }
        let xf = self.ln(&x, &self.ln_f_w, &self.ln_f_b)?;
        // tied head: logits = x·tok_embᵀ
        xf.matmul(&self.tok_emb.t()?)
    }

    /// 束生成: 前缀 <eos>+py+<sep> → 候选列（beam 5, ≤max_chars 字）。
    /// 返回 (文本, 累计 logprob) 降序——只用于排名, 分不跨键串比较。
    pub fn generate(
        &self,
        py: &str,
        beam: usize,
        max_chars: usize,
    ) -> candle_core::Result<Vec<(String, f32)>> {
        let mut prefix: Vec<u32> = vec![EOS];
        for c in py.chars() {
            if let Some(&id) = self.stoi.get(&c.to_string()) {
                prefix.push(id);
            }
        }
        prefix.push(SEP);
        // 束状态: (ids, score)
        let mut beams: Vec<(Vec<u32>, f32)> =
            vec![(prefix.clone(), 0.0)];
        let mut done: Vec<(String, f32)> = Vec::new();
        for _step in 0..max_chars + 2 {
            let mut next: Vec<(Vec<u32>, f32)> = Vec::new();
            for (ids, score) in &beams {
                let logits = self.forward(ids)?;
                let last = logits
                    .narrow(0, ids.len() - 1, 1)?
                    .flatten_all()?
                    .to_vec1::<f32>()?;
                // 每束取 top-beam 词（朴素版; eos 入 done）
                let mut idx: Vec<usize> = (0..last.len()).collect();
                idx.sort_by(|&a, &b| {
                    last[b].partial_cmp(&last[a]).unwrap_or(
                        std::cmp::Ordering::Equal,
                    )
                });
                for &j in idx.iter().take(beam) {
                    let lgp = last[j];
                    if j as u32 == EOS {
                        let text: String = ids[prefix.len()..]
                            .iter()
                            .filter_map(|&t| {
                                self.vocab
                                    .get(t as usize)
                                    .map(|s| s.as_str())
                            })
                            .collect();
                        if !text.is_empty() {
                            done.push((text, score + lgp));
                        }
                    } else {
                        let mut nids = ids.clone();
                        nids.push(j as u32);
                        next.push((nids, score + lgp));
                    }
                }
            }
            next.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            next.truncate(beam);
            if next.is_empty() {
                break;
            }
            beams = next;
            if done.len() >= beam * 2 {
                break;
            }
        }
        // 未完成束也收（青简律: 未耗尽束入池）
        for (ids, score) in beams {
            let text: String = ids[prefix.len()..]
                .iter()
                .filter_map(|&t| {
                    self.vocab.get(t as usize).map(|s| s.as_str())
                })
                .collect();
            if !text.is_empty() && !done.iter().any(|(t, _)| *t == text) {
                done.push((text, score));
            }
        }
        done.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 冒烟权重生成链验证（200 步弱模型——只验「能生成合理 token
    /// 串」, 语义质量待全量权重）。
    #[test]
    #[ignore = "真机冒烟权重（P2C_SMOKE 指路径）"]
    fn p2c_smoke_generate() {
        let p = std::env::var("P2C_SMOKE")
            .unwrap_or_else(|_| "/mnt/s790/tmp/p2c_smoke".into());
        let m = P2cModel::open(std::path::Path::new(&p))
            .expect("载入失败");
        let t0 = std::time::Instant::now();
        let cands = m.generate("nihaoshijie", 5, 12).expect("生成失败");
        eprintln!(
            "[p2c-smoke] {:?} ({:?})",
            cands.iter().take(5).collect::<Vec<_>>(),
            t0.elapsed()
        );
        assert!(!cands.is_empty(), "应有候选");
        for (t, _) in &cands {
            assert!(!t.is_empty(), "空候选");
        }
    }
}
