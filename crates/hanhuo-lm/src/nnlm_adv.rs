//! nlrgpt2 高级版 NNLM（PC 3 层 d256）——开放格式装载与推理
//!
//! 与 mobile（`nnlm::Nnlm`）同骨架的 pre-LN GPT-2，但差异足够大，
//! 独立模块而非泛化混写（mobile 前向已黄金验证，不动）：
//! - KV 纯 past-concat（无 mobile 的 shared/start_cache 机制）
//! - attn scale = 0.125（乘法；mobile 为除以 sqrt(48)）
//! - gelu 常量精确值（mobile 为原厂 fp16 圆整值）
//! - 输入恒单步 [1,1] → 因果掩码退化（全可见），保留 numpy 的 +1e-9
//! - 附加 z 签名头（64 维 ±1 上下文签名，z_flag 门控，用途未明）
//!
//! 参考: xnet/models/nlrgpt2/nlrgpt2_numpy.py（f64 复现，对拍 TFLite
//! cos>0.99999999）。本实现 f32 推理，与参考的偏差由
//! `test_numpy_golden`（f64 基准向量）约束。
//!
//! 数据归腾讯，仅本地个人研究使用，禁止分发。
#![cfg(feature = "safetensors")]

use crate::nnlm::open_import::{f16_of, f32_of, read_vocab};
use crate::nnlm::{ln, linear};
use std::collections::HashMap;

/// 高级版常量（nlrgpt2_numpy.py 权威值，非 fp16 圆整）
const GELU_C: f32 = 0.797_884_560_802_865_4; // sqrt(2/π)
const GELU_A: f32 = 0.044_715;
const ATTN_SCALE: f32 = 0.125; // 1/sqrt(64)
const MASK_ADD: f32 = 1e-9; // 掩码后加和（T50）

pub struct AdvLayer {
    attn_w: Vec<f32>, // [3d][d] 行连续 out×in
    attn_b: Vec<f32>,
    proj_w: Vec<f32>, // [d][d]
    proj_b: Vec<f32>,
    ln1g: Vec<f32>,
    ln1b: Vec<f32>,
    ln2g: Vec<f32>,
    ln2b: Vec<f32>,
    fc_w: Vec<f32>, // [4d][d]
    fc_b: Vec<f32>,
    pr2_w: Vec<f32>, // [d][4d]
    pr2_b: Vec<f32>,
}

/// 增量 KV 缓存（pos-major: 位置 s 第 h 头在 s*d + h*hd）
#[derive(Clone)]
pub struct Past {
    pub n_past: usize,
    k: Vec<Vec<f32>>, // [li][S*d]
    v: Vec<Vec<f32>>,
}

pub struct NnlmAdv {
    wte: Vec<f32>, // [V][d] 词嵌入 = 输出投影（weight-tying）
    wpe: Vec<f32>, // [n_ctx][d]
    lnf_g: Vec<f32>,
    lnf_b: Vec<f32>,
    wz: Vec<f32>, // [z][d] z 签名头（可选输出）
    bz: Vec<f32>,
    layers: Vec<AdvLayer>,
    pub vocab: HashMap<String, u32>,
    n_layer: usize,
    d_model: usize,
    n_heads: usize,
    head_dim: usize,
    n_ctx: usize,
    vocab_size: usize,
}

/// 单步推理产物（hf 供 z 签名头复用）
pub struct AdvOut {
    pub lp: Vec<f32>, // log_softmax，len = vocab_size
    pub past: Past,
    pub hf: Vec<f32>, // ln_f 后终态隐向量
}

#[inline]
fn gelu_adv(x: &mut [f32]) {
    for v in x.iter_mut() {
        *v = 0.5
            * *v
            * (1.0 + (GELU_C * (*v + GELU_A * *v * *v * *v)).tanh());
    }
}

impl NnlmAdv {
    /// 从 hanhuo-nnlm v1 开放格式装载（variant = "advanced"，fp32/fp16 皆可）
    pub fn from_safetensors(bytes: &[u8]) -> Result<NnlmAdv, String> {
        let st = safetensors::SafeTensors::deserialize(bytes)
            .map_err(|e| format!("容器解析: {e}"))?;
        let (_, meta) = safetensors::SafeTensors::read_metadata(bytes)
            .map_err(|e| format!("元数据: {e}"))?;
        let variant = meta
            .metadata()
            .as_ref()
            .and_then(|m| m.get("variant"))
            .map(String::as_str)
            .unwrap_or("");
        if variant != "advanced" {
            return Err(format!(
                "变体 {variant:?} 非 advanced——mobile 请用 \
                 nnlm::Nnlm::from_safetensors"
            ));
        }
        // 维度从张量形状推导（不信元数据数值）；n_heads 形状推不出，读元数据
        let wte_shape = st
            .tensor("wte")
            .map_err(|e| format!("wte: {e}"))?
            .shape()
            .to_vec();
        if wte_shape.len() != 2 {
            return Err(format!("wte 应为 2 维, 实得 {wte_shape:?}"));
        }
        let vocab_size = wte_shape[0];
        let d_model = wte_shape[1];
        let n_ctx = st
            .tensor("wpe")
            .map_err(|e| format!("wpe: {e}"))?
            .shape()[0];
        let n_heads: usize = meta
            .metadata()
            .as_ref()
            .and_then(|m| m.get("n_heads"))
            .and_then(|s| s.parse().ok())
            .ok_or("元数据缺 n_heads")?;
        if n_heads == 0 || d_model % n_heads != 0 {
            return Err(format!("d_model {d_model} 不可整除 n_heads {n_heads}"));
        }
        let head_dim = d_model / n_heads;

        let f16_file =
            st.tensor("wte").unwrap().dtype() == safetensors::Dtype::F16;
        let ld = |n: &str| {
            if f16_file {
                f16_of(&st, n)
            } else {
                f32_of(&st, n)
            }
        };
        let wte = ld("wte")?;
        let wpe = ld("wpe")?;
        let lnf_g = ld("ln_f_g")?;
        let lnf_b = ld("ln_f_b")?;
        let wz = ld("wz")?;
        let bz = ld("bz")?;

        let mut layers = Vec::new();
        let p = |li: usize, f: &str| format!("blocks.{li}.{f}");
        let mut li = 0usize;
        while st.tensor(&p(li, "attn_w")).is_ok() {
            let row = |name: &str, want: usize| -> Result<Vec<f32>, String> {
                let v = ld(name)?;
                if v.len() != want {
                    return Err(format!(
                        "{name}: 期望 {want} 实得 {}（维度不一致）",
                        v.len()
                    ));
                }
                Ok(v)
            };
            layers.push(AdvLayer {
                attn_w: row(&p(li, "attn_w"), 3 * d_model * d_model)?,
                attn_b: row(&p(li, "attn_b"), 3 * d_model)?,
                proj_w: row(&p(li, "proj_w"), d_model * d_model)?,
                proj_b: row(&p(li, "proj_b"), d_model)?,
                ln1g: row(&p(li, "ln1_g"), d_model)?,
                ln1b: row(&p(li, "ln1_b"), d_model)?,
                ln2g: row(&p(li, "ln2_g"), d_model)?,
                ln2b: row(&p(li, "ln2_b"), d_model)?,
                fc_w: row(&p(li, "fc_w"), 4 * d_model * d_model)?,
                fc_b: row(&p(li, "fc_b"), 4 * d_model)?,
                pr2_w: row(&p(li, "pr2_w"), d_model * 4 * d_model)?,
                pr2_b: row(&p(li, "pr2_b"), d_model)?,
            });
            li += 1;
        }
        if layers.is_empty() {
            return Err("无 blocks.* 层".into());
        }

        let vocab = read_vocab(&st)?;
        if vocab_size < 1 || vocab.values().max().map_or(true, |&m| m as usize >= vocab_size) {
            return Err("词表 id 越界（offsets 与 wte 行数不一致）".into());
        }
        let n_layer = layers.len();
        Ok(NnlmAdv {
            wte,
            wpe,
            lnf_g,
            lnf_b,
            wz,
            bz,
            layers,
            vocab,
            n_layer,
            d_model,
            n_heads,
            head_dim,
            n_ctx,
            vocab_size,
        })
    }

    pub fn dims(&self) -> (usize, usize, usize, usize, usize, usize) {
        (self.n_layer, self.d_model, self.n_heads, self.head_dim,
         self.n_ctx, self.vocab_size)
    }

    /// 文本 → token id 序列（逐字查表；缺字报错并列出）
    pub fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        let mut ids = Vec::new();
        let mut missing = Vec::new();
        for c in text.chars() {
            match self.vocab.get(&c.to_string()) {
                Some(&id) => ids.push(id),
                None => missing.push(c),
            }
        }
        if !missing.is_empty() {
            return Err(format!("词表缺字: {}", missing.iter().collect::<String>()));
        }
        Ok(ids)
    }

    /// 批量候选打分——与 `nnlm::Nnlm::score_candidates` 同语义：返回各
    /// 候选 ΣlnP(候选 token | ctx)。**分桶裂变 + 跨组合批**（轻量版
    /// 同构移植, 2026-11-20）: 候选按 token 前缀分桶共享 KV 步进, 每
    /// 步全部桶汇总一次 forward_batch——同音簇变体只付尾段增量成本,
    /// 逐候选独立前向的 past.clone()×N + 单步 predict×N 是电池评测
    /// 残余主成本。ctx+候选 超 n_ctx 时截 ctx 尾（保最大候选完整——
    /// 可见窗语义）。逐条参考实现见 [`Self::score_candidates_seq`]
    /// （parity 对照）。
    pub fn score_candidates(
        &self,
        ctx_ids: &[u32],
        cand_seqs: &[Vec<u32>],
    ) -> Vec<f64> {
        if cand_seqs.is_empty() {
            return Vec::new();
        }
        let max_len =
            cand_seqs.iter().map(|s| s.len()).max().unwrap_or(0);
        let room = self.n_ctx.saturating_sub(max_len + 1);
        // BOS 热启动（咨询二 Gemini §四: 裸 BOS 后是流中段分布——探针
        // 实测 top1=「，」; HANHUO_ADV_WARM=好的。 空上文时先喂中性
        // 前文再起评, 链式法则不变, 仅移动候选起始位置）。
        let warm: Vec<u32> = std::env::var("HANHUO_ADV_WARM")
            .ok()
            .and_then(|s| {
                let s = if s.is_empty() { "好的。".into() } else { s };
                self.encode(&s).ok()
            })
            .unwrap_or_default();
        let eff_ctx: Vec<u32> = if ctx_ids.is_empty() && !warm.is_empty()
        {
            warm
        } else {
            ctx_ids.to_vec()
        };
        let ctx: &[u32] = if eff_ctx.len() > room {
            &eff_ctx[eff_ctx.len() - room..]
        } else {
            &eff_ctx
        };
        // 初始分布: 喂 ctx（末步 lp 留作候选首字读分）; 空 ctx 喂 BOS(0)
        // 取初始分布——训练流 [BOS]+字符 同律（mobile bos oracle 等价物）。
        // **读分必须先于喂字**（链式法则 Σ ln P(t_i|ctx,t_<i); mobile
        // score_candidates 同律）——旧实现喂字后读 lp[t] = P(t|…,t)
        // 自包含条件化, 打分退化为噪声 → B7 全崩（曾误判「架构固有」,
        // 2026-11-13 adv 复核三定位: 引擎 bug 而非模型缺陷）。
        let mut past: Option<Past> = None;
        let mut dist: Vec<f32> = {
            let mut d = None;
            for &t in ctx {
                let o = self.predict(t, past.as_ref());
                d = Some(o.lp);
                past = Some(o.past);
            }
            match d {
                Some(d) => d,
                None => self.predict(0, None).lp, // BOS 初始分布
            }
        };
        // shared KV = ctx（或 BOS 单步）后的 Past——全部候选共同前缀。
        // 空 ctx 时 dist 已由 BOS 前进一步取得, 这里再前进一步取其
        // Past（predict 纯函数无状态, 两次 BOS 数值一致）
        let (shared_k, shared_v) = match past.as_ref() {
            Some(p) => (p.k.clone(), p.v.clone()),
            None => {
                let o = self.predict(0, None);
                (o.past.k, o.past.v)
            }
        };
        // 有效长（旧 take 同律: 候选最多喂 n_ctx−1）
        let lens: Vec<usize> = cand_seqs
            .iter()
            .map(|s| s.len().min(self.n_ctx.saturating_sub(1)))
            .collect();
        let max_len = lens.iter().copied().max().unwrap_or(0);
        let b_n = cand_seqs.len();
        struct Group {
            members: Vec<usize>,
            k: Vec<Vec<f32>>,
            v: Vec<Vec<f32>>,
            dist: Vec<f32>,
        }
        let mut scores = vec![0f32; b_n];
        let mut groups = vec![Group {
            members: (0..b_n).collect(),
            k: vec![Vec::new(); self.n_layer],
            v: vec![Vec::new(); self.n_layer],
            dist,
        }];
        for t in 0..max_len {
            // 读分（喂字前——链式法则）
            for g in &groups {
                for &b in &g.members {
                    if lens[b] > t {
                        scores[b] +=
                            g.dist[cand_seqs[b][t] as usize % self.vocab_size];
                    }
                }
            }
            if t == max_len - 1 {
                break;
            }
            // 分桶裂变: 需要后续分布的成员按下一 token 分组（同前缀
            // 共享 KV）; 跨组合批——裂变后常态多组, 每步一次批前向
            let old_groups = std::mem::take(&mut groups);
            let mut plan: Vec<(usize, u32, Vec<usize>)> = Vec::new();
            for (gi, g) in old_groups.iter().enumerate() {
                let mut buckets: Vec<(u32, Vec<usize>)> = Vec::new();
                for &b in &g.members {
                    if lens[b] > t + 1 {
                        let tok = cand_seqs[b][t];
                        match buckets.iter_mut().find(|(x, _)| *x == tok) {
                            Some((_, v)) => v.push(b),
                            None => buckets.push((tok, vec![b])),
                        }
                    }
                }
                for (tok, members) in buckets {
                    plan.push((gi, tok, members));
                }
            }
            let n_bk = plan.len();
            let mut toks = Vec::with_capacity(n_bk);
            let mut oks: Vec<&[Vec<f32>]> = Vec::with_capacity(n_bk);
            let mut ovs: Vec<&[Vec<f32>]> = Vec::with_capacity(n_bk);
            for (gi, tok, _) in plan.iter() {
                toks.push(*tok);
                oks.push(&old_groups[*gi].k);
                ovs.push(&old_groups[*gi].v);
            }
            let results =
                self.forward_batch(&toks, &shared_k, &shared_v, &oks, &ovs, t);
            groups = plan
                .into_iter()
                .zip(results)
                .map(|((_, _, members), (k, v, dist))| Group {
                    members,
                    k,
                    v,
                    dist,
                })
                .collect();
            if groups.is_empty() {
                break;
            }
        }
        scores.into_iter().map(|s| s as f64).collect()
    }

    /// 逐候选独立前向（旧 score_candidates 主体——past.clone() +
    /// 单步 predict; 保留供批化版 parity 对照与诊断）。
    pub fn score_candidates_seq(
        &self,
        ctx_ids: &[u32],
        cand_seqs: &[Vec<u32>],
    ) -> Vec<f64> {
        if cand_seqs.is_empty() {
            return Vec::new();
        }
        // 与批化版同律的 ctx 前置（warm/裁剪/初始分布）——复用同一
        // 段逻辑: 直接调用 predict 步进
        let max_len =
            cand_seqs.iter().map(|s| s.len()).max().unwrap_or(0);
        let room = self.n_ctx.saturating_sub(max_len + 1);
        let warm: Vec<u32> = std::env::var("HANHUO_ADV_WARM")
            .ok()
            .and_then(|s| {
                let s = if s.is_empty() { "好的。".into() } else { s };
                self.encode(&s).ok()
            })
            .unwrap_or_default();
        let eff_ctx: Vec<u32> = if ctx_ids.is_empty() && !warm.is_empty() {
            warm
        } else {
            ctx_ids.to_vec()
        };
        let ctx: &[u32] = if eff_ctx.len() > room {
            &eff_ctx[eff_ctx.len() - room..]
        } else {
            &eff_ctx
        };
        let mut past: Option<Past> = None;
        let mut dist: Vec<f32> = {
            let mut d = None;
            for &t in ctx {
                let o = self.predict(t, past.as_ref());
                d = Some(o.lp);
                past = Some(o.past);
            }
            match d {
                Some(d) => d,
                None => self.predict(0, None).lp,
            }
        };
        // 空 ctx: BOS 步入 KV（mobile ctx_forward_memo→start_past 同律,
        // 训练流 [BOS]+字符——候选从位置 1 起。原版 BOS 只取分布不占槽
        // → 候选首步 wpe[0], 系存量偏差; 批化版修正, 此对照实现同步）
        if past.is_none() {
            past = Some(self.predict(0, None).past);
        }
        let mut out = Vec::with_capacity(cand_seqs.len());
        for seq in cand_seqs {
            let mut p = past.clone();
            let mut cur = dist.clone();
            let mut s = 0f64;
            let take = seq.len().min(self.n_ctx.saturating_sub(1));
            for &t in &seq[..take] {
                s += cur[t as usize] as f64; // 读分在喂字前
                let o = self.predict(t, p.as_ref());
                cur = o.lp;
                p = Some(o.past);
            }
            out.push(s);
        }
        out
    }

    /// z 签名头: sign(hf @ wz^T + bz)（numpy z_flag>0 分支；hf 为终态隐向量）
    pub fn z_sign(&self, hf: &[f32]) -> Vec<i8> {
        let z = self.bz.len();
        let mut out = vec![0i8; z];
        for (o, yo) in out.iter_mut().enumerate() {
            let mut s = self.bz[o];
            let row = &self.wz[o * self.d_model..(o + 1) * self.d_model];
            for i in 0..self.d_model {
                s += hf[i] * row[i];
            }
            *yo = if s > 0.0 {
                1
            } else if s < 0.0 {
                -1
            } else {
                0
            };
        }
        out
    }

    /// 单步推理（1:1 nlrgpt2_numpy.predict，f32）
    pub fn predict(&self, token: u32, past: Option<&Past>) -> AdvOut {
        let n_past = past.map_or(0, |p| p.n_past);
        assert!(n_past < self.n_ctx, "past 长 {n_past} ≥ n_ctx {}", self.n_ctx);
        let d = self.d_model;
        let id = (token as usize) % self.vocab_size;
        // h = wte[id] + wpe[n_past]
        let mut h: Vec<f32> = self.wte[id * d..id * d + d]
            .iter()
            .zip(&self.wpe[n_past * d..n_past * d + d])
            .map(|(a, &b)| a + b)
            .collect();

        let mut newk: Vec<Vec<f32>> = Vec::with_capacity(self.n_layer);
        let mut newv: Vec<Vec<f32>> = Vec::with_capacity(self.n_layer);
        let mut x = vec![0f32; d];
        let mut qkv = vec![0f32; 3 * d];
        let mut att_out = vec![0f32; d];
        let mut ffn = vec![0f32; 4 * d];
        let mut sc = vec![0f32; n_past + 1];
        for (li, l) in self.layers.iter().enumerate() {
            ln(&h, &l.ln1g, &l.ln1b, &mut x);
            linear(&x, &l.attn_w, &l.attn_b, &mut qkv);
            let mut k = past.map_or_else(Vec::new, |p| p.k[li].clone());
            let mut v = past.map_or_else(Vec::new, |p| p.v[li].clone());
            k.extend_from_slice(&qkv[d..2 * d]);
            v.extend_from_slice(&qkv[2 * d..]);
            // 单查询 → 因果掩码全可见; 保留 numpy 的 +1e-9
            let ns = n_past + 1;
            for hh in 0..self.n_heads {
                let qh = &qkv[hh * self.head_dim..(hh + 1) * self.head_dim];
                for (s2, sc2) in sc.iter_mut().enumerate() {
                    let kb = &k[s2 * d + hh * self.head_dim
                        ..s2 * d + (hh + 1) * self.head_dim];
                    let mut dot = 0f32;
                    for i in 0..self.head_dim {
                        dot += qh[i] * kb[i];
                    }
                    *sc2 = dot * ATTN_SCALE + MASK_ADD;
                }
                let m = sc.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let mut sum = 0f32;
                for e in sc.iter_mut() {
                    *e = (*e - m).exp();
                    sum += *e;
                }
                let inv = 1.0 / sum;
                let oh = &mut att_out[hh * self.head_dim
                    ..(hh + 1) * self.head_dim];
                oh.fill(0.0);
                for s2 in 0..ns {
                    let p = sc[s2] * inv;
                    let vb = &v[s2 * d + hh * self.head_dim
                        ..s2 * d + (hh + 1) * self.head_dim];
                    for (o, &vv) in oh.iter_mut().zip(vb) {
                        *o += p * vv;
                    }
                }
            }
            linear(&att_out, &l.proj_w, &l.proj_b, &mut x);
            for (hi, xi) in h.iter_mut().zip(&x) {
                *hi += xi;
            }
            ln(&h, &l.ln2g, &l.ln2b, &mut x);
            linear(&x, &l.fc_w, &l.fc_b, &mut ffn);
            gelu_adv(&mut ffn);
            linear(&ffn, &l.pr2_w, &l.pr2_b, &mut x);
            for (hi, xi) in h.iter_mut().zip(&x) {
                *hi += xi;
            }
            newk.push(k);
            newv.push(v);
        }
        ln(&h, &self.lnf_g, &self.lnf_b, &mut x);
        // logits = hf @ wte^T（weight-tying）→ log_softmax
        let mut lp = vec![0f32; self.vocab_size];
        for (vid, lo) in lp.iter_mut().enumerate() {
            let row = &self.wte[vid * d..vid * d + d];
            let mut s = 0f32;
            for i in 0..d {
                s += x[i] * row[i];
            }
            *lo = s;
        }
        let m = lp.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0f32;
        for e in lp.iter_mut() {
            *e -= m;
            sum += e.exp();
        }
        let lse = sum.ln();
        for e in lp.iter_mut() {
            *e -= lse;
        }
        AdvOut {
            lp,
            past: Past { n_past: n_past + 1, k: newk, v: newv },
            hf: x,
        }
    }

    /// 批化单步前向（G 行 trie 同深度 t——轻量版 forward_batch 同构）。
    /// shared_k/v = ctx 共同前缀 KV（[层][S*d] 扁平）; own_k/v[r] = 行内
    /// 步进累积 KV（各层长恰 t*d——trie 性质: 同深度步进贡献同数 KV）。
    /// 行位置 = shared_len + t（wpe 同律, 全行一致）。数值路径与 predict
    /// 逐位同构（ln/linear_batch/gelu_adv/头循环+1e-9/weight-tying 投影）。
    #[allow(clippy::too_many_arguments)]
    fn forward_batch(
        &self,
        toks: &[u32],
        shared_k: &[Vec<f32>],
        shared_v: &[Vec<f32>],
        own_k: &[&[Vec<f32>]],
        own_v: &[&[Vec<f32>]],
        t: usize,
    ) -> Vec<(Vec<Vec<f32>>, Vec<Vec<f32>>, Vec<f32>)> {
        use crate::nnlm::linear_batch;
        let g = toks.len();
        let d = self.d_model;
        let hd = self.head_dim;
        let shared_len = shared_k.first().map_or(0, |k| k.len() / d);
        let pos = shared_len + t;
        let ns = pos + 1;
        let mut hs: Vec<Vec<f32>> = toks
            .iter()
            .map(|&tk| {
                let id = (tk as usize) % self.vocab_size;
                self.wte[id * d..id * d + d]
                    .iter()
                    .zip(&self.wpe[pos * d..pos * d + d])
                    .map(|(a, &b)| a + b)
                    .collect()
            })
            .collect();
        let mut xs = vec![vec![0f32; d]; g];
        let mut qkvs = vec![vec![0f32; 3 * d]; g];
        let mut att_outs = vec![vec![0f32; d]; g];
        let mut f768s = vec![vec![0f32; 4 * d]; g];
        let mut newk: Vec<Vec<Vec<f32>>> =
            vec![vec![Vec::new(); self.n_layer]; g];
        let mut newv: Vec<Vec<Vec<f32>>> =
            vec![vec![Vec::new(); self.n_layer]; g];
        let mut sc = vec![0f32; ns];
        for (li, l) in self.layers.iter().enumerate() {
            for r in 0..g {
                ln(&hs[r], &l.ln1g, &l.ln1b, &mut xs[r]);
            }
            linear_batch(&xs, &l.attn_w, &l.attn_b, &mut qkvs);
            for r in 0..g {
                // 行内 KV = own + new（shared 段由 attention 分段读取,
                // 不拼进行内——否则下一步再拼 shared 会双拼）
                let mut k = own_k[r][li].clone();
                k.extend_from_slice(&qkvs[r][d..2 * d]);
                let mut v = own_v[r][li].clone();
                v.extend_from_slice(&qkvs[r][2 * d..]);
                for hh in 0..self.n_heads {
                    let qh = &qkvs[r][hh * hd..(hh + 1) * hd];
                    for (s2, sc2) in sc.iter_mut().enumerate() {
                        // s2 段位: [0..shared_len)=ctx 前缀, 之后=行内 own+new
                        let (kb, vb) = if s2 < shared_len {
                            (
                                &shared_k[li]
                                    [s2 * d + hh * hd..s2 * d + (hh + 1) * hd],
                                &shared_v[li]
                                    [s2 * d + hh * hd..s2 * d + (hh + 1) * hd],
                            )
                        } else {
                            let o = (s2 - shared_len) * d;
                            (
                                &k[o + hh * hd..o + (hh + 1) * hd],
                                &v[o + hh * hd..o + (hh + 1) * hd],
                            )
                        };
                        let mut dot = 0f32;
                        for i in 0..hd {
                            dot += qh[i] * kb[i];
                        }
                        *sc2 = dot * ATTN_SCALE + MASK_ADD;
                    }
                    let m =
                        sc.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    let mut sum = 0f32;
                    for e in sc.iter_mut() {
                        *e = (*e - m).exp();
                        sum += *e;
                    }
                    let inv = 1.0 / sum;
                    let oh = &mut att_outs[r][hh * hd..(hh + 1) * hd];
                    oh.fill(0.0);
                    for s2 in 0..ns {
                        let p = sc[s2] * inv;
                        let (kb, vb) = if s2 < shared_len {
                            (
                                &shared_k[li]
                                    [s2 * d + hh * hd..s2 * d + (hh + 1) * hd],
                                &shared_v[li]
                                    [s2 * d + hh * hd..s2 * d + (hh + 1) * hd],
                            )
                        } else {
                            let o = (s2 - shared_len) * d;
                            (&k[o + hh * hd..o + (hh + 1) * hd],
                             &v[o + hh * hd..o + (hh + 1) * hd])
                        };
                        for (o, &vv) in oh.iter_mut().zip(vb) {
                            *o += p * vv;
                        }
                    }
                }
                newk[r][li] = k;
                newv[r][li] = v;
            }
            linear_batch(&att_outs, &l.proj_w, &l.proj_b, &mut xs);
            for r in 0..g {
                for (hi, xi) in hs[r].iter_mut().zip(&xs[r]) {
                    *hi += *xi;
                }
            }
            for r in 0..g {
                ln(&hs[r], &l.ln2g, &l.ln2b, &mut xs[r]);
            }
            linear_batch(&xs, &l.fc_w, &l.fc_b, &mut f768s);
            for r in 0..g {
                gelu_adv(&mut f768s[r]);
            }
            linear_batch(&f768s, &l.pr2_w, &l.pr2_b, &mut xs);
            for r in 0..g {
                for (hi, xi) in hs[r].iter_mut().zip(&xs[r]) {
                    *hi += *xi;
                }
            }
        }
        for r in 0..g {
            ln(&hs[r], &self.lnf_g, &self.lnf_b, &mut xs[r]);
        }
        // logits 批量投影（weight-tying; wte 行装载一次服务 G 行）
        let mut lps = vec![vec![0f32; self.vocab_size]; g];
        for vid in 0..self.vocab_size {
            let row = &self.wte[vid * d..vid * d + d];
            for r in 0..g {
                let mut s = 0f32;
                for i in 0..d {
                    s += xs[r][i] * row[i];
                }
                lps[r][vid] = s;
            }
        }
        for r in 0..g {
            let m = lps[r].iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let mut sum = 0f32;
            for e in lps[r].iter_mut() {
                *e -= m;
                sum += e.exp();
            }
            let lse = sum.ln();
            for e in lps[r].iter_mut() {
                *e -= lse;
            }
        }
        (0..g)
            .map(|r| {
                (
                    std::mem::take(&mut newk[r]),
                    std::mem::take(&mut newv[r]),
                    std::mem::take(&mut lps[r]),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 批化 score_candidates vs 逐条 score_candidates_seq parity
    /// （分桶裂变+跨组合批与 past.clone 逐前向的数值一致性）。
    #[test]
    #[ignore = "真机: 需 data/nnlm_advanced_fp32"]
    fn adv_batch_parity() {
        let bytes = std::fs::read("data/nnlm_advanced_fp32.safetensors")
            .or_else(|_| {
                std::fs::read(
                    "../../data/nnlm_advanced_fp32.safetensors",
                )
            })
            .unwrap();
        let a = NnlmAdv::from_safetensors(&bytes).unwrap();
        let enc = |t: &str| a.encode(t).unwrap_or_default();
        // 象限: 全同序列 / 同前缀分叉簇（trie 深分桶）/ 无共享异簇 /
        // 单序列（G=1 回退形态）
        let cases: Vec<Vec<Vec<u32>>> = vec![
            vec![enc("跋涉远比到达目的地"), enc("跋涉远比到达目的地")],
            vec![
                enc("跋涉远比到达目的地"),
                enc("跋涉远比到达墓的地"),
                enc("吧社圆鼻刀大目的"),
            ],
            vec![enc("天青色等烟雨"), enc("袅袅升起"), enc("你好世界")],
            vec![enc("现在是")],
        ];
        for (i, seqs) in cases.iter().enumerate() {
            for (tag, ctx) in [("空ctx", Vec::new()), ("带ctx", enc("今天天气不错我们"))] {
                let s1 = a.score_candidates(&ctx, seqs);
                let s2 = a.score_candidates_seq(&ctx, seqs);
                assert_eq!(s1.len(), s2.len());
                for (j, (x, y)) in s1.iter().zip(s2.iter()).enumerate() {
                    assert!(
                        (x - y).abs() < 1e-3,
                        "case{i} {tag} seq{j}: 批 {x:.5} vs 逐 {y:.5}"
                    );
                }
            }
        }
    }

    /// B7 排序残局·adv 短窗判别探针（整句 top1 线 §瓶颈③——
    /// mobile 全句/短窗已证伪, adv 短窗为最后候选信号）
    #[test]
    #[ignore = "真机: 需 data/nnlm_advanced_fp32"]
    fn adv_window_endgame_probe() {
        let bytes = std::fs::read("data/nnlm_advanced_fp32.safetensors")
            .or_else(|_| {
                std::fs::read(
                    "../../data/nnlm_advanced_fp32.safetensors",
                )
            })
            .unwrap();
        let a = NnlmAdv::from_safetensors(&bytes).unwrap();
        let aseq = |t: &str| a.encode(t).unwrap_or_default();
        for (n, x, y) in [
            ("窗2", "跋涉远比到达目的地", "吧社圆鼻刀大目的"),
            ("歌词窗", "天青色等烟雨而我在等你", "天青色等烟雨儿我在等你"),
            ("窗2b", "跋涉远比到大墓地", "跋涉圆鼻刀大墓地"),
            ("窗3", "袅袅升起隔江千万里", "袅袅生气个讲千万里"),
        ] {
            let sc = a.score_candidates(&[], &[aseq(x), aseq(y)]);
            eprintln!("[advwin] {n}: 正确={:.3} 垃圾={:.3} Δ={:.3}",
                sc[0], sc[1], sc[0] - sc[1]);
        }
    }

    use super::*;

    /// 手写 safetensors 容器（与 open_export 同格式: [u64 len][json][blob]）
    fn write_st(
        tensors: &[(&str, Vec<usize>, Vec<u8>, &str)],
        variant: &str,
        n_heads: usize,
    ) -> Vec<u8> {
        let mut names: Vec<&str> = tensors.iter().map(|t| t.0).collect();
        names.sort();
        let mut blob = Vec::new();
        let mut entries = String::from("{");
        for (i, n) in names.iter().enumerate() {
            let (_, shape, data, dtype) =
                tensors.iter().find(|t| t.0 == *n).unwrap();
            let s = blob.len();
            blob.extend_from_slice(data);
            if i > 0 {
                entries.push(',');
            }
            entries.push_str(&format!(
                "\"{n}\":{{\"dtype\":\"{dtype}\",\"shape\":[{}],\
                 \"data_offsets\":[{s},{}]}}",
                shape.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(","),
                blob.len()
            ));
        }
        entries.push_str(&format!(
            ",\"__metadata__\":{{\"format\":\"hanhuo-nnlm\",\
             \"version\":\"1\",\"variant\":\"{variant}\",\
             \"n_heads\":\"{n_heads}\"}}}}"
        ));
        let jb = entries.into_bytes();
        let mut out = (jb.len() as u64).to_le_bytes().to_vec();
        out.extend_from_slice(&jb);
        out.extend_from_slice(&blob);
        out
    }

    /// LCG 伪随机（确定性权重）
    struct Lcg(u64);
    impl Lcg {
        fn f32(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 33) as f32 / (u32::MAX >> 1) as f32) - 1.0
        }
        fn vec(&mut self, n: usize) -> Vec<f32> {
            (0..n).map(|_| self.f32()).collect()
        }
        fn f16vec(&mut self, n: usize) -> Vec<u8> {
            self.vec(n)
                .iter()
                .flat_map(|v| half::f16::from_f32(*v).to_le_bytes())
                .collect()
        }
    }

    // 合成微型模型: V=5(1 空洞), D=8, 2H×4, ctx=4, 1 层, z=3
    const V: usize = 5;
    const D: usize = 8;
    fn syn_fp32() -> Vec<u8> {
        let mut r = Lcg(42);
        let f = |n: usize, r: &mut Lcg| {
            r.vec(n).iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>()
        };
        let toks: [&str; 5] = ["你", "好", "", "们", "的"]; // id2 = 空洞
        let mut vb = Vec::new();
        let mut voff: Vec<i64> = vec![0];
        for t in &toks {
            vb.extend_from_slice(t.as_bytes());
            voff.push(vb.len() as i64);
        }
        let mut ts: Vec<(&str, Vec<usize>, Vec<u8>, &str)> = vec![
            ("wte", vec![V, D], f(V * D, &mut r), "F32"),
            ("wpe", vec![4, D], f(4 * D, &mut r), "F32"),
            ("ln_f_g", vec![D], f(D, &mut r), "F32"),
            ("ln_f_b", vec![D], f(D, &mut r), "F32"),
            ("wz", vec![3, D], f(3 * D, &mut r), "F32"),
            ("bz", vec![3], f(3, &mut r), "F32"),
            ("blocks.0.attn_w", vec![3 * D, D], f(3 * D * D, &mut r), "F32"),
            ("blocks.0.attn_b", vec![3 * D], f(3 * D, &mut r), "F32"),
            ("blocks.0.proj_w", vec![D, D], f(D * D, &mut r), "F32"),
            ("blocks.0.proj_b", vec![D], f(D, &mut r), "F32"),
            ("blocks.0.ln1_g", vec![D], f(D, &mut r), "F32"),
            ("blocks.0.ln1_b", vec![D], f(D, &mut r), "F32"),
            ("blocks.0.ln2_g", vec![D], f(D, &mut r), "F32"),
            ("blocks.0.ln2_b", vec![D], f(D, &mut r), "F32"),
            ("blocks.0.fc_w", vec![4 * D, D], f(4 * D * D, &mut r), "F32"),
            ("blocks.0.fc_b", vec![4 * D], f(4 * D, &mut r), "F32"),
            ("blocks.0.pr2_w", vec![D, 4 * D], f(D * 4 * D, &mut r), "F32"),
            ("blocks.0.pr2_b", vec![D], f(D, &mut r), "F32"),
            ("vocab.bytes", vec![vb.len()], vb.clone(), "U8"),
            (
                "vocab.offsets",
                vec![V + 1],
                voff.iter().flat_map(|o| o.to_le_bytes()).collect(),
                "I64",
            ),
        ];
        ts.sort_by(|a, b| a.0.cmp(b.0));
        write_st(&ts, "advanced", 2)
    }

    #[test]
    fn test_synthetic_load_and_predict() {
        let m = NnlmAdv::from_safetensors(&syn_fp32()).expect("装载");
        assert_eq!(m.dims(), (1, D, 2, 4, 4, V));
        // 词表: 4 个实词 + 空洞（零长合并）
        assert_eq!(m.vocab.get("你"), Some(&0));
        assert_eq!(m.vocab.get("们"), Some(&3));
        assert_eq!(m.encode("你好").unwrap(), vec![0, 1]);
        assert!(m.encode("好X").is_err(), "缺字应报错");

        let o1 = m.predict(0, None);
        assert_eq!(o1.lp.len(), V);
        // log_softmax: Σexp(lp) == 1
        let s: f32 = o1.lp.iter().map(|e| e.exp()).sum();
        assert!((s - 1.0).abs() < 1e-5, "Σexp(lp)={s}");
        assert_eq!(o1.past.n_past, 1);
        assert_eq!(o1.past.k.len(), 1);
        assert_eq!(o1.past.k[0].len(), D);
        // z 签名: ±1/0 且长度 3
        let z = m.z_sign(&o1.hf);
        assert_eq!(z.len(), 3);
        assert!(z.iter().all(|&v| v == 1 || v == -1 || v == 0));

        // 位置敏感: 同 token 不同位置 → 不同分布
        let o2 = m.predict(0, Some(&o1.past));
        let diff: f32 = o1.lp.iter().zip(&o2.lp).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
        assert!(diff > 1e-6, "同 token 异位置应变化, max|Δ|={diff}");
        assert_eq!(o2.past.n_past, 2);
    }

    #[test]
    fn test_synthetic_fp16_and_errors() {
        // fp16: 权重半精度, 词表 U8/I64 不变
        let mut r = Lcg(7);
        let f16 = |n: usize, r: &mut Lcg| r.f16vec(n);
        let toks: [&str; 5] = ["a", "b", "", "c", "d"];
        let mut vb = Vec::new();
        let mut voff: Vec<i64> = vec![0];
        for t in &toks {
            vb.extend_from_slice(t.as_bytes());
            voff.push(vb.len() as i64);
        }
        let mut ts: Vec<(&str, Vec<usize>, Vec<u8>, &str)> = vec![
            ("wte", vec![V, D], f16(V * D, &mut r), "F16"),
            ("wpe", vec![4, D], f16(4 * D, &mut r), "F16"),
            ("ln_f_g", vec![D], f16(D, &mut r), "F16"),
            ("ln_f_b", vec![D], f16(D, &mut r), "F16"),
            ("wz", vec![3, D], f16(3 * D, &mut r), "F16"),
            ("bz", vec![3], f16(3, &mut r), "F16"),
            ("blocks.0.attn_w", vec![3 * D, D], f16(3 * D * D, &mut r), "F16"),
            ("blocks.0.attn_b", vec![3 * D], f16(3 * D, &mut r), "F16"),
            ("blocks.0.proj_w", vec![D, D], f16(D * D, &mut r), "F16"),
            ("blocks.0.proj_b", vec![D], f16(D, &mut r), "F16"),
            ("blocks.0.ln1_g", vec![D], f16(D, &mut r), "F16"),
            ("blocks.0.ln1_b", vec![D], f16(D, &mut r), "F16"),
            ("blocks.0.ln2_g", vec![D], f16(D, &mut r), "F16"),
            ("blocks.0.ln2_b", vec![D], f16(D, &mut r), "F16"),
            ("blocks.0.fc_w", vec![4 * D, D], f16(4 * D * D, &mut r), "F16"),
            ("blocks.0.fc_b", vec![4 * D], f16(4 * D, &mut r), "F16"),
            ("blocks.0.pr2_w", vec![D, 4 * D], f16(D * 4 * D, &mut r), "F16"),
            ("blocks.0.pr2_b", vec![D], f16(D, &mut r), "F16"),
            ("vocab.bytes", vec![vb.len()], vb, "U8"),
            (
                "vocab.offsets",
                vec![V + 1],
                voff.iter().flat_map(|o| o.to_le_bytes()).collect(),
                "I64",
            ),
        ];
        ts.sort_by(|a, b| a.0.cmp(b.0));
        let m = NnlmAdv::from_safetensors(&write_st(&ts, "advanced", 2))
            .expect("fp16 装载");
        let o = m.predict(1, None);
        let s: f32 = o.lp.iter().map(|e| e.exp()).sum();
        assert!((s - 1.0).abs() < 1e-4);

        // 错误路径: variant 不符（同张量集标成 mobile）
        let mut ts2 = ts.clone();
        ts2.sort_by(|a, b| a.0.cmp(b.0));
        let err = NnlmAdv::from_safetensors(&write_st(&ts2, "mobile", 2))
            .err()
            .expect("variant 不符应报错");
        assert!(err.contains("mobile"), "{err}");
    }

    /// numpy f64 基准对拍（tools/safetensors/gen_nnlm_adv_golden.py 产物）
    #[test]
    #[ignore = "需要 data/nnlm_advanced_fp32.safetensors + golden 基准"]
    fn test_numpy_golden() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/nnlm_advanced_fp32.safetensors"
        ))
        .expect("data/nnlm_advanced_fp32.safetensors");
        let m = NnlmAdv::from_safetensors(&bytes).expect("装载");
        assert_eq!(m.dims(), (3, 256, 4, 64, 512, 5594));

        let gj: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/nnlm_adv_golden.json"),
        )
        .expect("golden/nnlm_adv_golden.json"))
        .expect("json");

        let pair = |v: &serde_json::Value| -> (u32, f64) {
            let a = v.as_array().unwrap();
            (a[0].as_u64().unwrap() as u32, a[1].as_f64().unwrap())
        };
        for case in gj["cases"].as_array().expect("cases") {
            let ids: Vec<u32> = case["ids"].as_array().unwrap()
                .iter().map(|v| v.as_u64().unwrap() as u32).collect();
            let tol = case["tol"].as_f64().unwrap();
            let mut past: Option<Past> = None;
            let mut hf_last = Vec::new();
            let mut kv_last: Vec<(Vec<f32>, Vec<f32>)> = Vec::new();
            for (si, &tid) in ids.iter().enumerate() {
                let o = m.predict(tid, past.as_ref());
                let step = &case["steps"][si];
                // top-32: id 序列与 lp 双约束（序精确, |Δlp| ≤ tol）
                let mut mine: Vec<u32> = (0..m.vocab_size as u32).collect();
                mine.sort_by(|&a, &b| o.lp[b as usize].total_cmp(&o.lp[a as usize]));
                for (k2, tv) in step["top"].as_array().unwrap().iter().enumerate() {
                    let (rid, rlp) = pair(tv);
                    assert_eq!(mine[k2], rid, "case {ids:?} step {si} top[{k2}]");
                    let d = (rlp - o.lp[rid as usize] as f64).abs();
                    assert!(d <= tol, "top[{k2}] |Δlp|={d} > {tol}");
                }
                // 探针: 固定 64 id 的 |Δlp|
                for pv in step["probe"].as_array().unwrap() {
                    let (pid, plp) = pair(pv);
                    let d = (plp - o.lp[pid as usize] as f64).abs();
                    assert!(d <= tol, "probe {pid} |Δlp|={d} > {tol}");
                }
                past = Some(o.past);
                hf_last = o.hf;
                kv_last = past.as_ref().unwrap().k.iter()
                    .zip(&past.as_ref().unwrap().v)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
            }
            // 末步 hf
            let hf_ref: Vec<f64> = case["final"]["hf"].as_array().unwrap()
                .iter().map(|v| v.as_f64().unwrap()).collect();
            let md = hf_ref.iter().zip(&hf_last)
                .fold(0f64, |a, (&r, &x)| a.max((r - x as f64).abs()));
            assert!(md <= 5e-4, "hf max|Δ|={md}");
            // 末层 KV 尾位置（每层 k/v 256 值）
            let d = m.dims().1;
            for (li, kv) in case["final"]["kv_tail"].as_array().unwrap().iter().enumerate() {
                let last_off = (ids.len() - 1) * d;
                for (side, data) in [("k", &kv_last[li].0), ("v", &kv_last[li].1)] {
                    let r: Vec<f64> = kv[side].as_array().unwrap()
                        .iter().map(|v| v.as_f64().unwrap()).collect();
                    let mine = &data[last_off..last_off + d];
                    let md = r.iter().zip(mine)
                        .fold(0f64, |a, (&x, &y)| a.max((x - y as f64).abs()));
                    assert!(md <= 1e-3, "layer{li} {side} tail max|Δ|={md}");
                }
            }
        }
    }
}

