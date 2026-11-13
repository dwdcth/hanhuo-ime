//! 知意 Zhiyi 1.4V2-Long-L12 双条件选词模型——clean-room 自写推理。
//!
//! 规格来源: 官方发布包 README.md + test_vectors.safetensors
//! （Apache-2.0, dd1000001000/zhiyi_models; 按文档规格实现,
//! 不参考任何 GPL 参考实现——clean-room 纪律）。
//!
//! 用途: 拼音+前文双条件给句候选重排（读光标前文与所敲拼音挑候选）
//! ——长句召回/排序缺口的神经补件。
//!
//! 阶段: ①载入+BPE+序列构造（本文件当前）②前向 encoder+双头
//! ③test_vectors 对拍 ④Q8_0 性能化 ⑤CLI 接入评测。

use std::collections::HashMap;
use std::path::Path;

/// tokenizer（mmBERT Gemma BPE, 256k 词表 + 58 万合并）。
/// 规则（test_vectors case2 对拍验证）: normalize ' '→'▁';
/// Metaspace prepend + split——▁ 开新词, 每词前加 ▁; BPE 贪心合并。
pub struct ZhiyiTokenizer {
    vocab: HashMap<String, u32>,
    ranks: HashMap<(String, String), u32>,
    pub unk: u32,
}

impl ZhiyiTokenizer {
    pub fn from_json(txt: &str) -> Option<Self> {
        // 轻量手解（tokenizer.json 34MB——serde_json 全量解析内存可行但慢;
        // 仅取 model.vocab 与 model.merges 两段, 走原始字符串切片扫描）
        let mut vocab = HashMap::new();
        let mut ranks = HashMap::new();
        let v = serde_json::from_str::<serde_json::Value>(txt).ok()?;
        let m = v.get("model")?;
        for (k, id) in m.get("vocab")?.as_object()? {
            vocab.insert(k.clone(), id.as_u64()? as u32);
        }
        for (i, pair) in m.get("merges")?.as_array()?.iter().enumerate() {
            // 数组形 ["a","b"] 或字符串形 "a b"
            let (a, b) = match pair.as_array() {
                Some(arr) => (
                    arr.first()?.as_str()?.to_string(),
                    arr.get(1)?.as_str()?.to_string(),
                ),
                None => {
                    let s = pair.as_str()?;
                    let mut it = s.splitn(2, ' ');
                    (
                        it.next()?.to_string(),
                        it.next().unwrap_or("").to_string(),
                    )
                }
            };
            ranks.insert((a, b), i as u32);
        }
        let unk = vocab.get("<unk>").copied().unwrap_or(3);
        Some(Self { vocab, ranks, unk })
    }

    /// BPE 单词合并（贪心最低 rank）。
    fn bpe(&self, word: &str) -> Vec<String> {
        let mut syms: Vec<String> =
            word.chars().map(|c| c.to_string()).collect();
        loop {
            let mut best: Option<(u32, usize)> = None;
            for i in 0..syms.len().saturating_sub(1) {
                if let Some(&r) = self.ranks.get(&(syms[i].clone(), syms[i + 1].clone())) {
                    if best.is_none() || r < best.unwrap().0 {
                        best = Some((r, i));
                    }
                }
            }
            let Some((_r, i)) = best else { break };
            let merged = format!("{}{}", syms[i], syms[i + 1]);
            syms.splice(i..i + 2, [merged]);
        }
        syms
    }

    /// encode（无特殊 token）。带前导空格的候选写法: encode(" 你好")
    /// ——空格即 Metaspace ▁（与 prepend 等效单 ▁）。
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let s = text.replace(' ', "\u{2581}");
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut first = true;
        for ch in s.chars() {
            if ch == '\u{2581}' {
                if !cur.is_empty() {
                    self.push_word(&mut out, &cur);
                    cur.clear();
                }
                first = false;
            } else {
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            self.push_word(&mut out, &cur);
        } else if first && !s.is_empty() {
            // 无 ▁ 的输入（不常见）——整串一词
        }
        out
    }

    fn push_word(&self, out: &mut Vec<u32>, word: &str) {
        for sym in self.bpe(&format!("\u{2581}{word}")) {
            out.push(self.vocab.get(&sym).copied().unwrap_or(self.unk));
        }
    }
}

/// 拼音单元表（unified_vocab.json: units 932 + letters 28 + chars 6163
/// + readings 字→读音列）。
pub struct ZhiyiVocab {
    pub units: Vec<String>,
    pub letters: Vec<String>,
    pub chars: Vec<String>,
    /// 字 → 读音串列（小写全拼, 如 一→[yi]）
    pub readings: HashMap<String, Vec<String>>,
    pub unit_id: HashMap<String, u32>,
}

impl ZhiyiVocab {
    pub fn from_json(txt: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(txt).ok()?;
        let units: Vec<String> = v
            .get("units")?
            .as_array()?
            .iter()
            .map(|x| x.as_str().unwrap_or_default().to_string())
            .collect();
        let letters: Vec<String> = match v.get("letters")? {
            // 官方包为字符串 "abcdefghijklmnopqrstuvwxyz'-"（28 字符）
            serde_json::Value::String(s) => {
                s.chars().map(|c| c.to_string()).collect()
            }
            serde_json::Value::Array(arr) => arr
                .iter()
                .map(|x| x.as_str().unwrap_or_default().to_string())
                .collect(),
            _ => return None,
        };
        let chars: Vec<String> = v
            .get("chars")?
            .as_array()?
            .iter()
            .map(|x| x.as_str().unwrap_or_default().to_string())
            .collect();
        let mut readings = HashMap::new();
        for (ch, rs) in v.get("readings")?.as_object()? {
            readings.insert(
                ch.clone(),
                rs.as_array()?
                    .iter()
                    .map(|x| x.as_str().unwrap_or_default().to_string())
                    .collect(),
            );
        }
        let unit_id: HashMap<String, u32> = units
            .iter()
            .enumerate()
            .map(|(i, u)| (u.clone(), i as u32))
            .collect();
        Some(Self { units, letters, chars, readings, unit_id })
    }

    /// py 贪心音节切分（f: 单元全集按长度降序; 切不尽→空——调用方
    /// 置零单元, 模型退化无拼音条件仍可比）。
    pub fn greedy_sylls(&self, py: &str) -> Vec<String> {
        let mut syls: Vec<&str> = self
            .units
            .iter()
            .filter(|u| u.starts_with("f:"))
            .map(|u| &u[2..])
            .collect();
        syls.sort_by_key(|s| std::cmp::Reverse(s.len()));
        let mut out = Vec::new();
        let mut i = 0usize;
        let b = py.as_bytes();
        while i < b.len() {
            let mut hit = false;
            for s in &syls {
                let sb = s.as_bytes();
                if i + sb.len() <= b.len() && &b[i..i + sb.len()] == sb {
                    out.push((*s).to_string());
                    i += sb.len();
                    hit = true;
                    break;
                }
            }
            if !hit {
                return Vec::new();
            }
        }
        out
    }

    /// 敲键对齐: typed（字母串）× 读音音节列 → 单元 id 列
    /// （f:全拼 / p:前缀; 读不进键 → None——调用方置零单元）。
    /// 单元语义: 覆盖 typed 的对齐（README: 中国 typed zhongg →
    /// f:zhong p:g）。
    pub fn align_units(&self, typed: &str, syllables: &[String]) -> Option<Vec<u32>> {
        let mut out = Vec::new();
        let mut pos = 0usize;
        let tb = typed.as_bytes();
        for syl in syllables {
            if pos >= tb.len() {
                return None;
            }
            let sb = syl.as_bytes();
            let mut full = pos + sb.len() <= tb.len();
            if full {
                for k in 0..sb.len() {
                    if tb[pos + k] != sb[k] {
                        full = false;
                        break;
                    }
                }
            }
            if full {
                let id = *self.unit_id.get(&format!("f:{syl}"))?;
                out.push(id);
                pos += sb.len();
            } else {
                // 前缀: typed 剩余是 syl 的前缀
                let rem = &tb[pos..];
                if rem.len() < sb.len()
                    && !rem.is_empty()
                    && sb.starts_with(rem)
                {
                    let prefix =
                        std::str::from_utf8(rem).ok()?;
                    let id = *self.unit_id.get(&format!("p:{prefix}"))?;
                    out.push(id);
                    pos = tb.len();
                } else {
                    return None;
                }
            }
        }
        if pos != tb.len() {
            return None; // 键未用尽
        }
        Some(out)
    }
}

/// 模型载入态（阶段①: 配置+词表; 阶段②: 张量）。
pub struct ZhiyiModel {
    pub tok: ZhiyiTokenizer,
    pub vocab: ZhiyiVocab,
    pub layer_types: Vec<u8>, // 0=full 1=sliding
    pub rope_theta: f32,
    pub sliding_window: usize,
    pub max_slots: usize,
    pub choice_context_tokens: usize,
    pub choice_candidate_tokens: usize,
}

pub const TOK_BOS: u32 = 2;
pub const TOK_EOS: u32 = 1;
pub const TOK_MASK: u32 = 4;

impl ZhiyiModel {
    /// 从发布包目录载入（config.json + tokenizer.json +
    /// unified_vocab.json; 张量阶段②再挂）。
    pub fn open(dir: &Path) -> Option<Self> {
        let dbg = std::env::var("ZHIYI_DBG").is_ok();
        macro_rules! step {
            ($e:expr, $m:expr) => {
                match $e {
                    Some(v) => v,
                    None => {
                        if dbg {
                            eprintln!("[zhiyi] 载入失败于: {}", $m);
                        }
                        return None;
                    }
                }
            };
        }
        let cfg_txt = std::fs::read_to_string(dir.join("config.json")).ok()?;
        let cfg: serde_json::Value =
            step!(serde_json::from_str(&cfg_txt).ok(), "config 解析");
        let enc = step!(cfg.get("encoder"), "encoder 段");
        let layer_types: Vec<u8> = step!(
            enc.get("layer_types")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|t| {
                            if t.as_str() == Some("full_attention") { 0 } else { 1 }
                        })
                        .collect()
                }),
            "layer_types"
        );
        let tok_txt = std::fs::read_to_string(dir.join("tokenizer.json")).ok()?;
        let vocab_txt =
            std::fs::read_to_string(dir.join("unified_vocab.json")).ok()?;
        let tok = step!(
            ZhiyiTokenizer::from_json(&tok_txt),
            "tokenizer（34MB 解析）"
        );
        let vocab = step!(ZhiyiVocab::from_json(&vocab_txt), "unified_vocab");
        Some(Self {
            tok,
            vocab,
            layer_types,
            rope_theta: step!(enc.get("rope_theta").and_then(|v| v.as_f64()), "rope_theta") as f32,
            sliding_window: step!(
                enc.get("sliding_window").and_then(|v| v.as_u64()),
                "sliding_window"
            ) as usize,
            max_slots: 24,
            choice_context_tokens: 128,
            choice_candidate_tokens: 16,
        })
    }

    /// 选词序列构造（choice_format "ctx-first"）:
    /// `<bos> ctx[末128] <eos> <mask>×单元(≤24) <eos> (<mask> 候选≤16tok)×N <eos>`
    /// 返回 (input_ids, unit_ids 对齐, marker_pos)。
    pub fn choice_sequence(
        &self,
        ctx: &str,
        typed: &str,
        syllables: &[String],
        candidates: &[String],
    ) -> (Vec<u32>, Vec<u32>, Vec<usize>) {
        let mut ids = vec![TOK_BOS];
        let mut units: Vec<u32> = vec![0];
        // ctx 段（末 128 token）
        let mut ctx_ids = if ctx.is_empty() {
            Vec::new()
        } else {
            self.tok.encode(ctx)
        };
        if ctx_ids.len() > self.choice_context_tokens {
            ctx_ids = ctx_ids[ctx_ids.len() - self.choice_context_tokens..].to_vec();
        }
        for id in ctx_ids {
            ids.push(id);
            units.push(0);
        }
        ids.push(TOK_EOS);
        units.push(0);
        // 单元段（≤24）
        let unit_ids = self
            .vocab
            .align_units(typed, syllables)
            .unwrap_or_default();
        for &u in unit_ids.iter().take(self.max_slots) {
            ids.push(TOK_MASK);
            units.push(u);
        }
        ids.push(TOK_EOS);
        units.push(0);
        // 候选段
        let mut markers = Vec::new();
        for cand in candidates {
            let mut cids = self.tok.encode(&format!(" {cand}"));
            cids.truncate(self.choice_candidate_tokens);
            if cids.is_empty() {
                continue;
            }
            markers.push(ids.len());
            ids.push(TOK_MASK);
            units.push(0);
            for id in cids {
                ids.push(id);
                units.push(0);
            }
        }
        ids.push(TOK_EOS);
        units.push(0);
        (ids, units, markers)
    }
}

// ================= 阶段②: candle 纯 Rust 前向（效果验证线） =================
// 规格: 官方 README「Forward pass」节。f32 运算域（Q8_0 性能化=阶段④）。

use candle_core::{DType, Device, Tensor};

struct EncL {
    attn_norm: Tensor, // layer0 无（占位空）
    wqkv: Tensor,      // [1152, 384]
    wo: Tensor,        // [384, 384]
    mlp_norm: Tensor,
    wi: Tensor,        // [2304, 384]
    mlp_wo: Tensor,    // [384, 1152]
}

struct HeadL {
    norm1_w: Tensor, norm1_b: Tensor,
    in_proj_w: Tensor, in_proj_b: Tensor,
    out_proj_w: Tensor, out_proj_b: Tensor,
    norm2_w: Tensor, norm2_b: Tensor,
    linear1_w: Tensor, linear1_b: Tensor,
    linear2_w: Tensor, linear2_b: Tensor,
}

/// 前向网络（candle 张量; CPU）。
pub struct ZhiyiNet {
    vocab_remap: Tensor, // [256000] u32
    tok_emb: Tensor,     // [112393, 384] f32
    emb_norm_w: Tensor,
    layers: Vec<EncL>,
    final_norm_w: Tensor,
    unit_emb: Tensor, // [960, 384]
    type_emb: Tensor, // [3, 384]
    head: Vec<HeadL>,
    sc_ln_w: Tensor, sc_ln_b: Tensor,
    sc_w1: Tensor, sc_b1: Tensor,
    sc_w2: Tensor, sc_b2: Tensor,
    // config
    n_heads: usize,
    rope_cache: Vec<f32>, // 32 freq（base 160000）
    sliding: Vec<bool>,
    window: usize,
}

const ENC_EPS: f64 = 1e-5;

impl ZhiyiNet {
    /// 载 model.safetensors（f16→f32）。
    pub fn load(dir: &Path, cfg: &ZhiyiModel) -> candle_core::Result<Self> {
        use safetensors::SafeTensors;
        let raw = std::fs::read(dir.join("model.safetensors"))?;
        let st = SafeTensors::deserialize(&raw).map_err(candle_core::Error::wrap)?;
        let dev = Device::Cpu;
        let f32t = |name: &str| -> candle_core::Result<Tensor> {
            let v = st.tensor(name).map_err(candle_core::Error::wrap)?;
            let dt = match v.dtype() {
                safetensors::Dtype::F16 => DType::F16,
                safetensors::Dtype::F32 => DType::F32,
                safetensors::Dtype::I32 => DType::U32,
                d => panic!("未知 dtype {d:?} {name}"),
            };
            let t = Tensor::from_raw_buffer(v.data(), dt, &v.shape(), &dev)?;
            match dt {
                DType::U32 => Ok(t),
                _ => t.to_dtype(DType::F32),
            }
        };
        let n_heads = 6usize;
        let mut rope_cache = Vec::with_capacity(32);
        for i in 0..32 {
            rope_cache.push(
                (cfg.rope_theta as f64).powf(-(2.0 * i as f64) / 64.0)
                    as f32,
            );
        }
        let mut layers = Vec::new();
        for n in 0..12 {
            layers.push(EncL {
                attn_norm: if n == 0 {
                    Tensor::zeros(0, DType::F32, &dev)?
                } else {
                    f32t(&format!("encoder.layers.{n}.attn_norm.weight"))?
                },
                wqkv: f32t(&format!("encoder.layers.{n}.attn.Wqkv.weight"))?,
                wo: f32t(&format!("encoder.layers.{n}.attn.Wo.weight"))?,
                mlp_norm: f32t(&format!("encoder.layers.{n}.mlp_norm.weight"))?,
                wi: f32t(&format!("encoder.layers.{n}.mlp.Wi.weight"))?,
                mlp_wo: f32t(&format!("encoder.layers.{n}.mlp.Wo.weight"))?,
            });
        }
        let head = (0..2)
            .map(|n| candle_core::Result::Ok(HeadL {
                norm1_w: f32t(&format!("head.layers.{n}.norm1.weight"))?,
                norm1_b: f32t(&format!("head.layers.{n}.norm1.bias"))?,
                in_proj_w: f32t(&format!(
                    "head.layers.{n}.self_attn.in_proj_weight"
                ))?,
                in_proj_b: f32t(&format!(
                    "head.layers.{n}.self_attn.in_proj_bias"
                ))?,
                out_proj_w: f32t(&format!(
                    "head.layers.{n}.self_attn.out_proj.weight"
                ))?,
                out_proj_b: f32t(&format!(
                    "head.layers.{n}.self_attn.out_proj.bias"
                ))?,
                norm2_w: f32t(&format!("head.layers.{n}.norm2.weight"))?,
                norm2_b: f32t(&format!("head.layers.{n}.norm2.bias"))?,
                linear1_w: f32t(&format!("head.layers.{n}.linear1.weight"))?,
                linear1_b: f32t(&format!("head.layers.{n}.linear1.bias"))?,
                linear2_w: f32t(&format!("head.layers.{n}.linear2.weight"))?,
                linear2_b: f32t(&format!("head.layers.{n}.linear2.bias"))?,
            }))
            .collect::<candle_core::Result<_>>()?;
        Ok(Self {
            vocab_remap: f32t("vocab_remap")?,
            tok_emb: f32t("encoder.embeddings.tok_embeddings.weight")?,
            emb_norm_w: f32t("encoder.embeddings.norm.weight")?,
            layers,
            final_norm_w: f32t("encoder.final_norm.weight")?,
            unit_emb: f32t("unit_emb.weight")?,
            type_emb: f32t("type_emb.weight")?,
            head,
            sc_ln_w: f32t("scorer.0.weight")?,
            sc_ln_b: f32t("scorer.0.bias")?,
            sc_w1: f32t("scorer.1.weight")?,
            sc_b1: f32t("scorer.1.bias")?,
            sc_w2: f32t("scorer.3.weight")?,
            sc_b2: f32t("scorer.3.bias")?,
            n_heads,
            rope_cache,
            sliding: cfg
                .layer_types
                .iter()
                .map(|&t| t == 1)
                .collect(),
            window: cfg.sliding_window,
        })
    }

    fn ln(&self, x: &Tensor, w: &Tensor, b: Option<&Tensor>, eps: f64) -> candle_core::Result<Tensor> {
        match b {
            Some(b) => candle_nn::ops::layer_norm(x, w, b, eps as f32),
            None => {
                let zeros = Tensor::zeros(x.dim(1)?, x.dtype(), &x.device())?;
                candle_nn::ops::layer_norm(x, w, &zeros, eps as f32)
            }
        }
    }

    /// RoPE（rotate-half, 单头 [seq, hd]: 配对 (j, j+hd/2), 位置 0 恒等）。
    fn rope(&self, x: &Tensor, _seq: usize) -> candle_core::Result<Tensor> {
        let seq = x.dim(0)?;
        let d = x.dim(1)?; // 单头维（64）
        let half = d / 2;
        let xf = x.flatten_all()?.to_vec1::<f32>()?;
        let mut out = vec![0f32; xf.len()];
        for p in 0..seq {
            let base = p * d;
            for j in 0..half {
                let f = self.rope_cache[j];
                let (a, b) = (xf[base + j], xf[base + j + half]);
                let (c, s) = ((p as f32 * f).cos(), (p as f32 * f).sin());
                out[base + j] = a * c - b * s;
                out[base + j + half] = a * s + b * c;
            }
        }
        Tensor::from_vec(out, x.shape().dims().to_vec(), &x.device())
    }

    /// attention（scale 1/8; full 或滑窗 |i-j|≤window）。
    fn attn(
        &self,
        x: &Tensor,       // [seq, d]
        wqkv: &Tensor,    // [3d, d] 无偏置
        wo: &Tensor,      // [d, d]
        sliding: bool,
    ) -> candle_core::Result<Tensor> {
        let seq = x.dim(0)?;
        let d = x.dim(1)?;
        let qkv = x.matmul(&wqkv.t()?)?; // [seq, 3d]
        let (hd, nh) = (d / self.n_heads, self.n_heads);
        let split: Vec<Tensor> = qkv.chunk(3, 1)?;
        let mut heads_out = Vec::new();
        for h in 0..nh {
            let q = self.rope(
                &split[0].narrow(1, h * hd, hd)?,
                seq,
            )?;
            let k = self.rope(
                &split[1].narrow(1, h * hd, hd)?,
                seq,
            )?;
            let v = split[2].narrow(1, h * hd, hd)?;
            let scores = q.matmul(&k.t()?)?.affine(1.0 / 8.0, 0.0)?;
            let scores = if sliding {
                // 掩码: |i-j|>window → -inf
                let mut m = vec![];
                for i in 0..seq {
                    for j in 0..seq {
                        m.push(if i.abs_diff(j) > self.window {
                            f32::NEG_INFINITY
                        } else {
                            0.0
                        });
                    }
                }
                let mask =
                    Tensor::from_vec(m, (seq, seq), &x.device())?;
                (scores + mask)?
            } else {
                scores
            };
            let w = candle_nn::ops::softmax_last_dim(&scores)?;
            heads_out.push(w.matmul(&v)?);
        }
        let cat = Tensor::cat(&heads_out, 1)?;
        cat.matmul(&wo.t()?)
    }

    /// encoder 12 层（pre-norm 无偏置; GeGLU-erf）。
    fn encoder(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let mut x = x.clone();
        let seq = x.dim(0)?;
        for (n, l) in self.layers.iter().enumerate() {
            let h = if n == 0 {
                x.clone()
            } else {
                self.ln(&x, &l.attn_norm, None, ENC_EPS)?
            };
            let a = self.attn(&h, &l.wqkv, &l.wo, self.sliding[n])?;
            if std::env::var("ZHIYI_MID").is_ok() && n == 0 {
                let r = a.narrow(0, 0, 1)?.flatten_all()?.to_vec1::<f32>()?;
                eprintln!(
                    "[zhiyi-mid] 层0 attn row0 和: {:.6} 前4: {:?}",
                    r.iter().sum::<f32>(),
                    &r[..4]
                );
            }
            x = (x + a)?;
            let h2 = self.ln(&x, &l.mlp_norm, None, ENC_EPS)?;
            let ag = h2.matmul(&l.wi.t()?)?; // [seq, 2304]
            let seq_len = ag.dim(0)?;
            let a = ag.narrow(1, 0, 1152)?;
            let g = ag.narrow(1, 1152, 1152)?;
            let gel = a.gelu_erf()?;
            let prod = (gel * g)?;
            let _ = seq_len;
            let back = prod.matmul(&l.mlp_wo.t()?)?;
            x = (x + back)?;
            let _ = seq;
        }
        let fx = self.ln(&x, &self.final_norm_w, None, ENC_EPS)?;
        if std::env::var("ZHIYI_MID").is_ok() {
            let r = fx.narrow(0, 0, 1)?.flatten_all()?.to_vec1::<f32>()?;
            eprintln!("[zhiyi-mid] final row0 和: {:.6}", r.iter().sum::<f32>());
        }
        Ok(fx)
    }

    /// PyTorch nn.TransformerEncoderLayer（norm_first, ReLU FFN 1536）。
    fn head_layer(&self, x: &Tensor, l: &HeadL) -> candle_core::Result<Tensor> {
        let seq = x.dim(0)?;
        let h = self.ln(x, &l.norm1_w, Some(&l.norm1_b), ENC_EPS)?;
        let qkv = h.matmul(&l.in_proj_w.t()?)?.broadcast_add(&l.in_proj_b.reshape((1, 1152))?)?;
        let (nh, hd) = (self.n_heads, 64usize);
        let parts = qkv.chunk(3, 1)?;
        let mut outs = Vec::new();
        for hi in 0..nh {
            let q = parts[0].narrow(1, hi * hd, hd)?;
            let k = parts[1].narrow(1, hi * hd, hd)?;
            let v = parts[2].narrow(1, hi * hd, hd)?;
            let s = q.matmul(&k.t()?)?.affine(1.0 / 8.0, 0.0)?;
            let w = candle_nn::ops::softmax_last_dim(&s)?;
            outs.push(w.matmul(&v)?);
        }
        let cat = Tensor::cat(&outs, 1)?;
        let attn = cat.matmul(&l.out_proj_w.t()?)?.broadcast_add(&l.out_proj_b.reshape((1, 384))?)?;
        let x = (x + attn)?;
        let h2 = self.ln(&x, &l.norm2_w, Some(&l.norm2_b), ENC_EPS)?;
        let ff = h2.matmul(&l.linear1_w.t()?)?.broadcast_add(&l.linear1_b.reshape((1, 1536))?)?;
        let ff = ff.relu()?;
        let ff = ff.matmul(&l.linear2_w.t()?)?.broadcast_add(&l.linear2_b.reshape((1, 384))?)?;
        let _ = seq;
        x + ff
    }

    /// 选词前向: (ids, unit_ids, markers) → 每候选一分（越大越好）。
    pub fn forward_choice(
        &self,
        ids: &[u32],
        unit_ids: &[u32],
        markers: &[usize],
    ) -> candle_core::Result<Vec<f32>> {
        let dev = Device::Cpu;
        let seq = ids.len();
        let ids_t = Tensor::from_vec(ids.to_vec(), seq, &dev)?;
        let remapped = self.vocab_remap.gather(&ids_t, 0)?; // [seq] u32
        let tok = self.tok_emb.index_select(&remapped, 0)?; // [seq, 384]
        let units_t = Tensor::from_vec(unit_ids.to_vec(), seq, &dev)?;
        let unit = self.unit_emb.index_select(&units_t.to_dtype(DType::U32)?, 0)?;
        let mut x = (tok + unit)?;
        x = self.ln(&x, &self.emb_norm_w, None, ENC_EPS)?;
        let enc = self.encoder(&x)?;
        let type0 = self.type_emb.get(0)?;
        let h0 = enc.broadcast_add(&type0.reshape((1, 384))?)?;
        let mut h = h0;
        for l in &self.head {
            h = self.head_layer(&h, l)?;
        }
        // scorer: ln → 384→384 → gelu → 384→1
        let mut scores = Vec::with_capacity(markers.len());
        for &m in markers {
            let row = h.narrow(0, m, 1)?; // [1, 384]
            let r = self.ln(&row, &self.sc_ln_w, Some(&self.sc_ln_b), ENC_EPS)?;
            let r = r.matmul(&self.sc_w1.t()?)?.broadcast_add(&self.sc_b2_broadcast1()?)?;
            let r = r.gelu_erf()?;
            let r = r.matmul(&self.sc_w2.t()?)?.broadcast_add(&self.sc_b2.reshape((1, 1))?)?;
            scores.push(r.flatten_all()?.to_vec1::<f32>()?[0]);
        }
        Ok(scores)
    }

    fn sc_b2_broadcast1(&self) -> candle_core::Result<Tensor> {
        self.sc_b1.reshape((1, 384))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// test_vectors case2 对拍（真机包在场才跑; 包路径 env 钮）。
    /// case2: 无 ctx + ni hao, 候选 你好/拟好/泥好 → ids
    /// [2,1,4,4,1, 4,30485,235607, 4,235248,238399,235607, 4,169524,235607, 1]
    #[test]
    #[ignore = "真机 zhiyi 包（ZHIYI_DIR 指发布包目录）"]
    fn zhiyi_case2_sequence_roundtrip() {
        let dir = std::env::var("ZHIYI_DIR").unwrap_or_else(|_| {
            "/mnt/s790/tmp/zhiyi-orig".into()
        });
        let m = ZhiyiModel::open(std::path::Path::new(&dir))
            .expect("载入失败");
        let (ids, units, markers) = m.choice_sequence(
            "",
            "nihao",
            &["ni".into(), "hao".into()],
            &["你好".into(), "拟好".into(), "泥好".into()],
        );
        let want = vec![
            2u32, 1, 4, 4, 1, 4, 30485, 235607, 4, 235248, 238399, 235607,
            4, 169524, 235607, 1,
        ];
        assert_eq!(ids, want, "case2 序列");
        assert_eq!(markers, vec![5usize, 8, 12], "markers");
        assert_eq!(units[2], m.vocab.unit_id["f:ni"], "unit f:ni");
        assert_eq!(units[3], m.vocab.unit_id["f:hao"], "unit f:hao");
        // 阶段②前向对拍: logits 期望 [2.2265, -7.0351, -6.4352]
        // （fp32 ONNX 输出; 自写 f32 前向容差 0.21 内——README Q8_0 同档）
        let net = ZhiyiNet::load(std::path::Path::new(&dir), &m)
            .expect("网络载入失败");
        let scores = net
            .forward_choice(&ids, &units, &markers)
            .expect("前向失败");
        let want_s = [2.2265f32, -7.0351, -6.4352];
        if std::env::var("ZHIYI_DBG").is_ok() {
            eprintln!(
                "[zhiyi] got={scores:?} want={want_s:?} 差={:?}",
                scores.iter().zip(want_s.iter()).map(|(a, b)| (a - b).abs()).collect::<Vec<_>>()
            );
        }
        // rope 单头 bug 修复后逐位对拍（1e-3 容差, f32 精度位）
        for (got, w) in scores.iter().zip(want_s.iter()) {
            assert!(
                (got - w).abs() < 1e-3,
                "logit {got:.6} vs 期望 {w:.6}（差 {:.6}）",
                (got - w).abs()
            );
        }
    }

    /// 全 5 choice case 序对拍（官方 ids 直喂——排除序列构造差异,
    /// 纯前向口径; cases.json 由 test_vectors 导出）。
    #[test]
    #[ignore = "真机 zhiyi 包"]
    fn zhiyi_choice_cases_rank() {
        let dir = std::env::var("ZHIYI_DIR")
            .unwrap_or_else(|_| "/mnt/s790/tmp/zhiyi-orig".into());
        let m = ZhiyiModel::open(std::path::Path::new(&dir))
            .expect("载入失败");
        let net = ZhiyiNet::load(std::path::Path::new(&dir), &m)
            .expect("网络载入失败");
        let cases: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                std::path::Path::new(&dir).join("cases.json"),
            )
            .expect("cases.json（由 test_vectors 导出）"),
        )
        .expect("cases.json 解析");
        let mut all_top1 = 0usize;
        let mut n = 0usize;
        for c in 0..5 {
            let cs = cases.get(c.to_string()).expect("case");
            let ids: Vec<u32> = cs["ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect();
            let units: Vec<u32> = cs["units"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect();
            let markers: Vec<usize> = cs["markers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as usize)
                .collect();
            let want: Vec<f32> = cs["logits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap() as f32)
                .collect();
            let got = net
                .forward_choice(&ids, &units, &markers)
                .expect("前向失败");
            let am = |v: &[f32]| {
                v.iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .map(|(i, _)| i)
                    .unwrap()
            };
            let (gm, wm) = (am(&got), am(&want));
            n += 1;
            if gm == wm {
                all_top1 += 1;
            }
            if std::env::var("ZHIYI_DBG").is_ok() {
                eprintln!("[zhiyi] case{c}: got={got:?} want={want:?} top {gm}=={wm}");
            }
        }
        // rope 修复后全 case 首选对拍（曾 4/5——case3 英文案翻车的
        // root cause=rope 单头张量按多头布局切, 已修）
        assert_eq!(all_top1, n, "{n} case 首选全对拍");
    }
}

    /// 阶段⑤ 端到端效果验证: 406 句电池 × 引擎宽档 top30 ×
    /// 知意重排（纯序口径, 不写回引擎分）vs 基线。
    /// battery.jsonl = {py, gold, ctx, cands:[{w,r,k}]}。
    #[test]
    #[ignore = "真机 zhiyi 包 + 电池候选文件"]
    fn zhiyi_battery_eval() {
        let dir = std::env::var("ZHIYI_DIR")
            .unwrap_or_else(|_| "/mnt/s790/tmp/zhiyi-orig".into());
        let m = ZhiyiModel::open(std::path::Path::new(&dir))
            .expect("载入失败");
        let net = ZhiyiNet::load(std::path::Path::new(&dir), &m)
            .expect("网络载入失败");
        let bat_txt = std::fs::read_to_string(
            std::path::Path::new(&dir).join("battery.jsonl"),
        )
        .expect("battery.jsonl");
        let mut n = 0usize;
        let mut base_top1 = 0usize;
        let mut zy_top1 = 0usize;
        let mut gold_in = 0usize;
        let mut fwd_err = 0usize;
        for line in bat_txt.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
            else {
                continue;
            };
            let py = v["py"].as_str().unwrap_or_default().to_string();
            let gold = v["gold"].as_str().unwrap_or_default().to_string();
            let ctx = v["ctx"].as_str().unwrap_or_default().to_string();
            // 只留整句候选（青简接入律: 整句转换候选才进模型比较
            // ——词级/单字杂类混入破坏比较分布; 首次 1.0% 翻车根因）。
            // 口径: 句级集只留 sentence 候选; 词级集（ZHIYI_KALL=1）
            // 全候选 top16（青简律: 词级选词比 exact/complete 全谱）
            let kall = std::env::var("ZHIYI_KALL").is_ok();
            let cands: Vec<String> = v["cands"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter(|c| {
                            kall || c["k"].as_str() == Some("sentence")
                        })
                        .filter_map(|c| c["w"].as_str().map(|x| x.to_string()))
                        .take(if kall { 16 } else { 30 })
                        .collect()
                })
                .unwrap_or_default();
            let sylls: Vec<String> = v["sylls"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if sylls.is_empty() {
                continue; // 贪心切分失败（14 句）跳过
            }
            if cands.is_empty() {
                continue;
            }
            n += 1;
            if cands.first().map(|w| w == &gold).unwrap_or(false) {
                base_top1 += 1;
            }
            if cands.iter().any(|w| *w == gold) {
                gold_in += 1;
            }
            // 候选音节列（reading ' 分隔）→ align_units
            let words: Vec<String> = cands.clone();
            // 单元对齐: py 贪心切分（battery 预计算, 中立无 gold 泄漏）
            let units = m.vocab.align_units(&py, &sylls).unwrap_or_default();
            let best_sylls = sylls;
            let _ = &best_sylls;
            let (ids, unit_ids, markers) = m.choice_sequence(
                &ctx,
                &py,
                &best_sylls,
                &words,
            );
            let _ = &units;
            if markers.is_empty() {
                continue;
            }
            match net.forward_choice(&ids, &unit_ids, &markers) {
                Ok(sc) => {
                    let am = sc
                        .iter()
                        .enumerate()
                        .max_by(|a, b| {
                            a.1.partial_cmp(b.1).unwrap_or(
                                std::cmp::Ordering::Equal,
                            )
                        })
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    if am < words.len() && words[am] == gold {
                        zy_top1 += 1;
                    }
                }
                Err(_) => fwd_err += 1,
            }
        }
        eprintln!(
            "[zhiyi-battery] n={n} 基线top1={base_top1}({:.1}%) \
             知意top1={zy_top1}({:.1}%) gold在池={gold_in}({:.1}%) \
             前向异常={fwd_err}",
            base_top1 as f64 / n as f64 * 100.0,
            zy_top1 as f64 / n as f64 * 100.0,
            gold_in as f64 / n as f64 * 100.0,
        );
    }
