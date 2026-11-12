//! M4 神经层：nlrgptand（2 层 pre-LN GPT-2, d192, 4H×48, FFN768, vocab 5594）
//! 批量打分器——wxq/nnlm.py 的 Rust 移植（提速版）.
//!
//! 打分语义（§十二返修轮定版）:
//!   N(w) = Σ_t ln P(w_t | <s>, ctx, w_0..w_{t-1})   —— 喂字【前】读分布
//!   ΔN = N_best − N_w ≥ 0 → final += clamp(0.23·λ·ΔN, ±1.5), λ=−0.40
//!
//! 提速三招（结果与逐候选直算在浮点容差内等价）:
//! 1. **f32 全程**（原厂即 TFLite f16/f32；权重 f16→f32 无损）
//! 2. **前缀组去重**: 同前缀候选共享一次前向（输入相同 → 结果相同），
//!    每步按下一 token 分桶裂变；只需读分的尾步不再前向
//! 3. **ctx 记忆化**: 相同上下文的 KV 前向缓存（逐键场景重复率极高）
//!
//! 权重布局（NNLM 区）: [n_arr u32] × {idx u32|dtype u8|ndim u16|nbytes u32|
//! dims u32×ndim|f16 LE}，词表长度前缀 UTF-8，start_cache f32×768
//! （[2层][2kv][4头][48]），bos_logp f32×5594。
use std::cell::RefCell;
use std::collections::HashMap;

/// 词表容量（历史常量, 已动态化——见 Nnlm.v; c.6 8105 扩表）
pub const VOCAB: usize = 5594;
pub const N_CTX: usize = 128;
pub const N_LAYER: usize = 2;
pub const N_HEADS: usize = 4;
pub const HEAD_DIM: usize = 48;
pub const D_MODEL: usize = 192;
const EPS: f32 = 1e-5;
const GELU_A: f32 = 0.044_708_252; // fp16(0.044715) 原厂常量（f32）
const GELU_C: f32 = 0.797_851_562_5; // fp16(sqrt(2/pi)) 原厂常量
const ATTN_SCALE: f32 = 6.928_203; // sqrt(48)

const W_WTE: u32 = 65;
const W_WPE: u32 = 67;
const W_LNF_G: u32 = 102;
const W_LNF_B: u32 = 103;
const LAYER_IDX: [[u32; 12]; 2] = [
    [5, 6, 18, 19, 68, 69, 82, 83, 22, 23, 31, 32],
    [35, 36, 48, 49, 86, 87, 98, 99, 52, 53, 61, 62],
];

pub struct Layer {
    attn_w: Vec<f32>,  // [576][192] 转置（行连续: out×in）
    attn_b: Vec<f32>,  // 576
    proj_w: Vec<f32>,  // [192][192]
    proj_b: Vec<f32>,
    ln1g: Vec<f32>,
    ln1b: Vec<f32>,
    ln2g: Vec<f32>,
    ln2b: Vec<f32>,
    fc_w: Vec<f32>,  // [768][192]
    fc_b: Vec<f32>,  // 768
    pr2_w: Vec<f32>, // [192][768]
    pr2_b: Vec<f32>, // 192
}

/// 增量 KV 缓存（pos-major: 位置 s 的第 h 头在 s*192 + h*48）
#[derive(Clone)]
pub struct Past {
    pub n_past: usize,
    k: Vec<Vec<f32>>, // [li][S*192]
    v: Vec<Vec<f32>>,
}

pub struct Nnlm {
    v: usize, // 词表容量 = wte 行数（c.6 扩表动态化——曾常量 5594）
    wte: Vec<f32>, // [v][192]
    wpe: Vec<f32>, // [128][192]
    lnf_g: Vec<f32>,
    lnf_b: Vec<f32>,
    layers: Vec<Layer>,
    pub vocab: HashMap<String, u32>,
    bos_logp: Vec<f32>, // 5594
    start_k: Vec<f32>,  // [li][h*48+d] → li*192 + h*48
    start_v: Vec<f32>,
    // ctx KV 前向记忆化（逐键场景同一 ctx 反复打分）
    ctx_memo: RefCell<HashMap<Vec<u32>, (Vec<f32>, Past)>>,
}

/// f16 LE（两字节）→ f32
fn f16(b0: u8, b1: u8) -> f32 {
    let h = u16::from_le_bytes([b0, b1]);
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as i32;
    let frac = (h & 0x3ff) as u32;
    let bits: u32 = if exp == 0 {
        if frac == 0 {
            sign << 31
        } else {
            let mut e: i32 = -1;
            let mut f = frac as i32;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            f &= 0x3ff;
            (sign << 31) | (((113 + e) as u32) << 23) | ((f as u32) << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | 0x7f80_0000 | (frac << 13)
    } else {
        (sign << 31) | (((exp + 112) as u32) << 23) | (frac << 13)
    };
    f32::from_bits(bits)
}

fn le32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

fn le16(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}

// ----------------------------------------------------------------------
// 基础算子（f32）
/// 批量线性: 同权重矩阵服务 G 行——o 外层使权重行跨行复用
/// （权重流量 G× 摊销; 单行 dot 形制与 linear 相同→舍入一致）
pub(crate) fn linear_batch(
    xs: &[Vec<f32>],
    w: &[f32],
    b: &[f32],
    outs: &mut [Vec<f32>],
) {
    let n_in = xs[0].len();
    let n_out = outs[0].len();
    for o in 0..n_out {
        let row = &w[o * n_in..(o + 1) * n_in];
        for (r, x) in xs.iter().enumerate() {
            outs[r][o] = b[o] + dot8(row, x);
        }
    }
}

#[inline]
pub(crate) fn ln(x: &[f32], g: &[f32], b: &[f32], out: &mut [f32]) {
    let n = x.len();
    let mut m = 0f32;
    for &v in x {
        m += v;
    }
    m /= n as f32;
    let mut var = 0f32;
    for &v in x {
        let d = v - m;
        var += d * d;
    }
    var /= n as f32;
    let inv = 1.0 / (var + EPS).sqrt();
    for i in 0..n {
        out[i] = (x[i] - m) * inv * g[i] + b[i];
    }
}

#[inline]
fn gelu_into(x: &[f32], out: &mut [f32]) {
    for (o, &v) in out.iter_mut().zip(x.iter()) {
        *o = 0.5 * v * (1.0 + (GELU_C * (v + GELU_A * v * v * v)).tanh());
    }
}

/// y = x @ W^T + b（W 以 [out][in] 行存储）
#[inline]
pub(crate) fn linear(x: &[f32], w: &[f32], b: &[f32], out: &mut [f32]) {
    // 8 路分道累加: 保序 sum 阻止向量化（190 MFLOP/查询标量执行实测）;
    // 分道确定可重放（固定道数）, 与 python 逐序和差 ~1ulp——引擎级
    // golden（面板序）为守护网
    let n_in = x.len();
    for (o, yo) in out.iter_mut().enumerate() {
        let row = &w[o * n_in..(o + 1) * n_in];
        *yo = b[o] + dot8(row, x);
    }
}

/// 8 路分道点积（可向量化形制; 确定性: 道数固定）
#[cfg(target_arch = "x86_64")]
fn avx2_fma_ok() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        std::arch::is_x86_feature_detected!("avx2")
            && std::arch::is_x86_feature_detected!("fma")
    })
}

/// AVX2+FMA 点积（8 宽 vfmadd 链×4 道）。舍入与标量版差 ulp 级
/// （fma 融合）——NNLM 分仅内部一致即可（golden/v2 不经 NNLM）。
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
unsafe fn dot8_avx2(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    let n8 = a.len() & !7;
    let mut acc = _mm256_setzero_ps();
    let mut i = 0usize;
    while i < n8 {
        let va = _mm256_loadu_ps(&a[i]);
        let vb = _mm256_loadu_ps(&b[i]);
        acc = _mm256_fmadd_ps(va, vb, acc);
        i += 8;
    }
    let mut t = [0f32; 8];
    _mm256_storeu_ps(t.as_mut_ptr(), acc);
    let mut s = ((t[0] + t[1]) + (t[2] + t[3]))
        + ((t[4] + t[5]) + (t[6] + t[7]));
    while i < a.len() {
        s += a[i] * b[i];
        i += 1;
    }
    s
}

pub(crate) fn dot8(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if avx2_fma_ok() {
            // 安全性: 只读两切片, intrinsics 无越界（n8 对齐步进）
            return unsafe { dot8_avx2(a, b) };
        }
    }
    dot8_scalar(a, b)
}

fn dot8_scalar(a: &[f32], b: &[f32]) -> f32 {
    // 8 独立累加链 × 固定索引步进: 与旧 acc[i&7] 映射**逐位一致**
    // （同元素入同累加器, 求和树不变）, 但索引编译期固定——LLVM
    // 可自动向量化（AVX2 vfmadd）; 旧 `i & 7` 数据依赖索引阻断之
    // （NNLM 前向实测 13×23 字批 167ms → 向量化后见 nnlm_bench）
    let (mut a0, mut a1, mut a2, mut a3) =
        (0f32, 0f32, 0f32, 0f32);
    let (mut a4, mut a5, mut a6, mut a7) =
        (0f32, 0f32, 0f32, 0f32);
    let n8 = a.len() & !7;
    for i in (0..n8).step_by(8) {
        a0 += a[i] * b[i];
        a1 += a[i + 1] * b[i + 1];
        a2 += a[i + 2] * b[i + 2];
        a3 += a[i + 3] * b[i + 3];
        a4 += a[i + 4] * b[i + 4];
        a5 += a[i + 5] * b[i + 5];
        a6 += a[i + 6] * b[i + 6];
        a7 += a[i + 7] * b[i + 7];
    }
    let mut s = ((a0 + a1) + (a2 + a3)) + ((a4 + a5) + (a6 + a7));
    for i in n8..a.len() {
        s += a[i] * b[i];
    }
    s
}

#[inline]
fn log_softmax_row(logits: &mut [f32]) {
    let m = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut lse = 0f32;
    for v in logits.iter_mut() {
        *v -= m;
        lse += v.exp();
    }
    lse = lse.ln();
    for v in logits.iter_mut() {
        *v -= lse;
    }
}

/// 注意力: q(192) 对 [shared K/V ++ own K/V]（pos-major）→ out(192)
fn attn_heads(
    q: &[f32],
    ks: &[f32],
    vs: &[f32],
    ko: &[f32],
    vo: &[f32],
    out: &mut [f32],
) {
    let s_shared = ks.len() / 192;
    let s_own = ko.len() / 192;
    let total = s_shared + s_own;
    let mut sc = vec![0f32; total];
    for h in 0..N_HEADS {
        let qh = &q[h * HEAD_DIM..(h + 1) * HEAD_DIM];
        for s2 in 0..total {
            let (off, src) = if s2 < s_shared {
                (s2, ks)
            } else {
                (s2 - s_shared, ko)
            };
            let kk =
                &src[off * 192 + h * HEAD_DIM..off * 192 + (h + 1) * HEAD_DIM];
            let mut d = 0f32;
            for i in 0..HEAD_DIM {
                d += qh[i] * kk[i];
            }
            sc[s2] = d / ATTN_SCALE;
        }
        let m = sc.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0f32;
        for x in sc.iter_mut() {
            *x = (*x - m).exp();
            sum += *x;
        }
        let inv_sum = 1.0 / sum;
        let mut oh = [0f32; HEAD_DIM];
        for s2 in 0..total {
            let p = sc[s2] * inv_sum;
            if p < 1e-12 {
                continue;
            }
            let (off, src) = if s2 < s_shared {
                (s2, vs)
            } else {
                (s2 - s_shared, vo)
            };
            let vv =
                &src[off * 192 + h * HEAD_DIM..off * 192 + (h + 1) * HEAD_DIM];
            for i in 0..HEAD_DIM {
                oh[i] += p * vv[i];
            }
        }
        out[h * HEAD_DIM..(h + 1) * HEAD_DIM].copy_from_slice(&oh);
    }
}

impl Nnlm {
    /// 解析 NNLM section（f16 权重 → f32；线性层转置为 [out][in]）
    pub fn parse(section: &[u8]) -> Result<Nnlm, String> {
        if section.len() < 4 {
            return Err("NNLM 区过短".into());
        }
        let n_arr = le32(section, 0) as usize;
        let mut o = 4usize;
        let mut raw: HashMap<u32, Vec<f32>> = HashMap::new();
        for _ in 0..n_arr {
            if o + 11 > section.len() {
                return Err("NNLM 数组头截断".into());
            }
            let idx = le32(section, o);
            let ndim = u16::from_le_bytes([section[o + 5], section[o + 6]])
                as usize;
            let nbytes = le32(section, o + 7) as usize;
            o += 11;
            if o + 4 * ndim + nbytes > section.len() {
                return Err("NNLM 数组体截断".into());
            }
            o += 4 * ndim;
            let vals: Vec<f32> = section[o..o + nbytes]
                .chunks_exact(2)
                .map(|c| f16(c[0], c[1]))
                .collect();
            o += nbytes;
            raw.insert(idx, vals);
        }
        let mut get = |idx: u32| -> Result<Vec<f32>, String> {
            raw.remove(&idx)
                .ok_or_else(|| format!("NNLM 缺权重 {idx}"))
        };
        let wte = get(W_WTE)?;
        let wpe = get(W_WPE)?;
        let lnf_g = get(W_LNF_G)?;
        let lnf_b = get(W_LNF_B)?;
        let mut layers = Vec::with_capacity(N_LAYER);
        for idxs in LAYER_IDX {
            let attn_w = transpose(&get(idxs[0])?, 192, 576);
            let proj_w = transpose(&get(idxs[2])?, 192, 192);
            let fc_w = transpose(&get(idxs[8])?, 192, 768);
            let pr2_w = transpose(&get(idxs[10])?, 768, 192);
            layers.push(Layer {
                attn_w,
                attn_b: get(idxs[1])?,
                proj_w,
                proj_b: get(idxs[3])?,
                ln1g: get(idxs[4])?,
                ln1b: get(idxs[5])?,
                ln2g: get(idxs[6])?,
                ln2b: get(idxs[7])?,
                fc_w,
                fc_b: get(idxs[9])?,
                pr2_w,
                pr2_b: get(idxs[11])?,
            });
        }
        // 词表
        let nv = le32(section, o) as usize;
        o += 4;
        let mut vocab = HashMap::with_capacity(nv);
        for i in 0..nv {
            let l = le16(section, o) as usize;
            o += 2;
            if o + l > section.len() {
                return Err("NNLM 词表截断".into());
            }
            let tok =
                String::from_utf8_lossy(&section[o..o + l]).into_owned();
            o += l;
            vocab.insert(tok, i as u32);
        }
        // start_cache [li][kv][h][48]
        let ns = le32(section, o) as usize;
        o += 4;
        if ns != 768 {
            return Err(format!("start_cache 尺寸 {ns}≠768"));
        }
        let mut sc = vec![0f32; 768];
        for i in 0..768 {
            let b = &section[o + 4 * i..o + 4 * i + 4];
            sc[i] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        o += 4 * ns;
        let mut start_k = vec![0f32; N_LAYER * 192];
        let mut start_v = vec![0f32; N_LAYER * 192];
        for li in 0..N_LAYER {
            for h in 0..N_HEADS {
                for d in 0..HEAD_DIM {
                    let src = li * 384 + h * HEAD_DIM + d;
                    start_k[li * 192 + h * HEAD_DIM + d] = sc[src];
                    start_v[li * 192 + h * HEAD_DIM + d] =
                        sc[li * 384 + 192 + h * HEAD_DIM + d];
                }
            }
        }
        let nb = le32(section, o) as usize;
        o += 4;
        let v = wte.len() / 192;
        if nb != v {
            return Err(format!("bos_logp 尺寸 {nb}≠wte 行数 {v}"));
        }
        if vocab.len() != v {
            return Err(format!(
                "词表条数 {}≠wte 行数 {v}（词表/wte 失配）",
                vocab.len()
            ));
        }
        let mut bos = vec![0f32; v];
        for i in 0..v {
            let b = &section[o + 4 * i..o + 4 * i + 4];
            bos[i] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        Ok(Nnlm {
            v,
            wte,
            wpe,
            lnf_g,
            lnf_b,
            layers,
            vocab,
            bos_logp: bos,
            start_k,
            start_v,
            ctx_memo: RefCell::new(HashMap::new()),
        })
    }

    /// 词编码：任一字符词表外 → None（整词弃）
    pub fn encode(&self, text: &str) -> Option<Vec<u32>> {
        let mut ids = Vec::with_capacity(text.chars().count());
        for c in text.chars() {
            match self.vocab.get(&c.to_string()) {
                Some(&i) => ids.push(i),
                None => return None,
            }
        }
        Some(ids)
    }

    fn start_past(&self) -> Past {
        let mut k = Vec::with_capacity(N_LAYER);
        let mut v = Vec::with_capacity(N_LAYER);
        for li in 0..N_LAYER {
            k.push(self.start_k[li * 192..(li + 1) * 192].to_vec());
            v.push(self.start_v[li * 192..(li + 1) * 192].to_vec());
        }
        Past { n_past: 1, k, v }
    }

    /// 组路径单 token 前向: 注意力 = [shared 上下文 KV ++ own 前缀 KV ++ 新位]。
    /// 返回 (新 own K, 新 own V, 新分布)——精确对应 Python batch_step 的单行。
    fn forward_one(
        &self,
        token: u32,
        shared: &Past,
        own_k: &[Vec<f32>],
        own_v: &[Vec<f32>],
        t: usize,
    ) -> (Vec<Vec<f32>>, Vec<Vec<f32>>, Vec<f32>) {
        let pos = shared.n_past + t; // wpe 位（Python wpe[S+t]）
        let mut h = vec![0f32; D_MODEL];
        for i in 0..D_MODEL {
            h[i] = self.wte[token as usize * D_MODEL + i]
                + self.wpe[pos * D_MODEL + i];
        }
        let mut nk: Vec<Vec<f32>> = Vec::with_capacity(N_LAYER);
        let mut nv: Vec<Vec<f32>> = Vec::with_capacity(N_LAYER);
        let mut x = vec![0f32; D_MODEL];
        let mut qkv = vec![0f32; 3 * D_MODEL];
        let mut o = vec![0f32; D_MODEL];
        let mut r = vec![0f32; D_MODEL];
        let mut f768 = vec![0f32; 768];
        let mut f = vec![0f32; 768];
        for (li, l) in self.layers.iter().enumerate() {
            ln(&h, &l.ln1g, &l.ln1b, &mut x);
            linear(&x, &l.attn_w, &l.attn_b, &mut qkv);
            let (q, k_new, v_new) =
                (&qkv[..192], &qkv[192..384], &qkv[384..]);
            let mut kk = own_k[li].clone();
            let mut vv = own_v[li].clone();
            kk.extend_from_slice(k_new);
            vv.extend_from_slice(v_new);
            attn_heads(q, &shared.k[li], &shared.v[li], &kk, &vv, &mut o);
            linear(&o, &l.proj_w, &l.proj_b, &mut r);
            for i in 0..D_MODEL {
                h[i] += r[i];
            }
            ln(&h, &l.ln2g, &l.ln2b, &mut x);
            linear(&x, &l.fc_w, &l.fc_b, &mut f768);
            gelu_into(&f768, &mut f);
            linear(&f, &l.pr2_w, &l.pr2_b, &mut r);
            for i in 0..D_MODEL {
                h[i] += r[i];
            }
            nk.push(kk);
            nv.push(vv);
        }
        ln(&h, &self.lnf_g, &self.lnf_b, &mut x);
        let mut logits = vec![0f32; self.v];
        for v in 0..self.v {
            let row = &self.wte[v * D_MODEL..(v + 1) * D_MODEL];
            let mut s = 0f32;
            for i in 0..D_MODEL {
                s += x[i] * row[i];
            }
            logits[v] = s;
        }
        log_softmax_row(&mut logits);
        (nk, nv, logits)
    }

    /// 批量单步前向: score_candidates 逐步裂变出的单例组同层权重一次
    /// 装载服务 G 行（权重流量 G× 摊销——ctx 打分 492ms→批量后主成本
    /// 消除）。行为与 forward_one 逐行等价（dot 形制相同→舍入一致）。
    fn forward_batch(
        &self,
        tokens: &[u32],
        shared: &Past,
        own_ks: &[&[Vec<f32>]],
        own_vs: &[&[Vec<f32>]],
        t: usize,
    ) -> Vec<(Vec<Vec<f32>>, Vec<Vec<f32>>, Vec<f32>)> {
        let g = tokens.len();
        if g == 0 {
            return Vec::new();
        }
        let pos = shared.n_past + t;
        let mut hs: Vec<Vec<f32>> = tokens
            .iter()
            .map(|&tk| {
                (0..D_MODEL)
                    .map(|i| {
                        self.wte[tk as usize * D_MODEL + i]
                            + self.wpe[pos * D_MODEL + i]
                    })
                    .collect()
            })
            .collect();
        let mut xs: Vec<Vec<f32>> = vec![vec![0f32; D_MODEL]; g];
        let mut qkvs: Vec<Vec<f32>> = vec![vec![0f32; 3 * D_MODEL]; g];
        let mut os: Vec<Vec<f32>> = vec![vec![0f32; D_MODEL]; g];
        let mut rs: Vec<Vec<f32>> = vec![vec![0f32; D_MODEL]; g];
        let mut f768s: Vec<Vec<f32>> = vec![vec![0f32; 768]; g];
        let mut fs: Vec<Vec<f32>> = vec![vec![0f32; 768]; g];
        // [r][li] 累积 KV（层外层 × 行内层）
        let mut nks: Vec<Vec<Vec<f32>>> = vec![vec![Vec::new(); N_LAYER]; g];
        let mut nvs: Vec<Vec<Vec<f32>>> = vec![vec![Vec::new(); N_LAYER]; g];
        for (li, l) in self.layers.iter().enumerate() {
            for r in 0..g {
                ln(&hs[r], &l.ln1g, &l.ln1b, &mut xs[r]);
            }
            linear_batch(&xs, &l.attn_w, &l.attn_b, &mut qkvs);
            for r in 0..g {
                let (q, k_new, v_new) = (
                    &qkvs[r][..D_MODEL],
                    &qkvs[r][D_MODEL..2 * D_MODEL],
                    &qkvs[r][2 * D_MODEL..],
                );
                let mut kk = own_ks[r][li].clone();
                let mut vv = own_vs[r][li].clone();
                kk.extend_from_slice(k_new);
                vv.extend_from_slice(v_new);
                attn_heads(
                    q,
                    &shared.k[li],
                    &shared.v[li],
                    &kk,
                    &vv,
                    &mut os[r],
                );
                nks[r][li] = kk;
                nvs[r][li] = vv;
            }
            linear_batch(&os, &l.proj_w, &l.proj_b, &mut rs);
            for r in 0..g {
                for i in 0..D_MODEL {
                    hs[r][i] += rs[r][i];
                }
            }
            for r in 0..g {
                ln(&hs[r], &l.ln2g, &l.ln2b, &mut xs[r]);
            }
            linear_batch(&xs, &l.fc_w, &l.fc_b, &mut f768s);
            for r in 0..g {
                gelu_into(&f768s[r], &mut fs[r]);
            }
            linear_batch(&fs, &l.pr2_w, &l.pr2_b, &mut rs);
            for r in 0..g {
                for i in 0..D_MODEL {
                    hs[r][i] += rs[r][i];
                }
            }
        }
        // logits 投影批量: wte（4.3MB）一次装载服务 G 行——逐行投影时
        // 权重流 G×/步是 boost 路径残余主成本
        for r in 0..g {
            ln(&hs[r], &self.lnf_g, &self.lnf_b, &mut xs[r]);
        }
        let zero_b = vec![0f32; self.v];
        let mut logits_rows: Vec<Vec<f32>> =
            vec![vec![0f32; self.v]; g];
        linear_batch(&xs, &self.wte, &zero_b, &mut logits_rows);
        (0..g)
            .map(|r| {
                let mut logits = std::mem::take(&mut logits_rows[r]);
                log_softmax_row(&mut logits);
                (std::mem::take(&mut nks[r]), std::mem::take(&mut nvs[r]), logits)
            })
            .collect()
    }

    /// 单步前向（B=1，上下文路径）；单段 KV。返回 (log_probs, new_past)
    pub fn predict(
        &self,
        token: u32,
        past: Option<&Past>,
    ) -> (Vec<f32>, Past) {
        let start = self.start_past();
        let base = past.unwrap_or(&start);
        let pos = base.n_past;
        let mut h = vec![0f32; D_MODEL];
        for i in 0..D_MODEL {
            h[i] = self.wte[token as usize * D_MODEL + i]
                + self.wpe[pos * D_MODEL + i];
        }
        let mut nk: Vec<Vec<f32>> = Vec::with_capacity(N_LAYER);
        let mut nv: Vec<Vec<f32>> = Vec::with_capacity(N_LAYER);
        let mut x = vec![0f32; D_MODEL];
        let mut qkv = vec![0f32; 3 * D_MODEL];
        let mut o = vec![0f32; D_MODEL];
        let mut r = vec![0f32; D_MODEL];
        let mut f768 = vec![0f32; 768];
        let mut f = vec![0f32; 768];
        for (li, l) in self.layers.iter().enumerate() {
            ln(&h, &l.ln1g, &l.ln1b, &mut x);
            linear(&x, &l.attn_w, &l.attn_b, &mut qkv);
            let (q, k_new, v_new) =
                (&qkv[..192], &qkv[192..384], &qkv[384..]);
            let mut kk = base.k[li].clone();
            let mut vv = base.v[li].clone();
            kk.extend_from_slice(k_new);
            vv.extend_from_slice(v_new);
            attn_heads(q, &kk, &vv, &[], &[], &mut o);
            linear(&o, &l.proj_w, &l.proj_b, &mut r);
            for i in 0..D_MODEL {
                h[i] += r[i];
            }
            ln(&h, &l.ln2g, &l.ln2b, &mut x);
            linear(&x, &l.fc_w, &l.fc_b, &mut f768);
            gelu_into(&f768, &mut f);
            linear(&f, &l.pr2_w, &l.pr2_b, &mut r);
            for i in 0..D_MODEL {
                h[i] += r[i];
            }
            nk.push(kk);
            nv.push(vv);
        }
        ln(&h, &self.lnf_g, &self.lnf_b, &mut x);
        let mut logits = vec![0f32; self.v];
        for v in 0..self.v {
            let row = &self.wte[v * D_MODEL..(v + 1) * D_MODEL];
            let mut s = 0f32;
            for i in 0..D_MODEL {
                s += x[i] * row[i];
            }
            logits[v] = s;
        }
        log_softmax_row(&mut logits);
        (logits, Past { n_past: pos + 1, k: nk, v: nv })
    }

    /// 上下文前向 → (末步 log_probs, past)；空上下文 → (bos, start)
    pub fn ctx_forward(&self, ctx_ids: &[u32]) -> (Vec<f32>, Past) {
        let mut lp = self.bos_logp.clone();
        let mut past: Option<Past> = None;
        for &tid in ctx_ids {
            let (l, np) = self.predict(tid, past.as_ref());
            lp = l;
            past = Some(np);
        }
        match past {
            Some(p) => (lp, p),
            None => (self.bos_logp.clone(), self.start_past()),
        }
    }

    fn ctx_forward_memo(&self, ctx_ids: &[u32]) -> (Vec<f32>, Past) {
        if ctx_ids.is_empty() {
            return (self.bos_logp.clone(), self.start_past());
        }
        if let Some(v) = self.ctx_memo.borrow().get(ctx_ids) {
            return (v.0.clone(), v.1.clone());
        }
        let r = self.ctx_forward(ctx_ids);
        let mut m = self.ctx_memo.borrow_mut();
        if m.len() >= 64 {
            m.clear();
        }
        m.insert(ctx_ids.to_vec(), r.clone());
        r
    }

    /// 批量候选打分: N(w) = Σ_t ln P(w_t|<s>,ctx,w_<t)；空 seq → 0。
    /// 前缀组算法: 同前缀候选共享一次前向，每步按下一 token 分桶裂变。
    pub fn score_candidates(
        &self,
        ctx_ids: &[u32],
        cand_seqs: &[Vec<u32>],
    ) -> Vec<f64> {
        FW_TIME.with(|c| *c.borrow_mut() = 0.0);
        FW_CALLS.with(|c| *c.borrow_mut() = 0);
        let b_n = cand_seqs.len();
        if b_n == 0 {
            return Vec::new();
        }
        let lens: Vec<usize> = cand_seqs.iter().map(|s| s.len()).collect();
        let max_len = *lens.iter().max().unwrap();
        let (lp_ctx, shared) = self.ctx_forward_memo(ctx_ids);
        struct Group {
            members: Vec<usize>,
            k: Vec<Vec<f32>>,
            v: Vec<Vec<f32>>,
            dist: Vec<f32>,
        }
        let mut scores = vec![0f32; b_n];
        let mut groups = vec![Group {
            members: (0..b_n).collect(),
            k: vec![Vec::new(); N_LAYER],
            v: vec![Vec::new(); N_LAYER],
            dist: lp_ctx,
        }];
        for t in 0..max_len {
            // 读分（喂字前读分布）
            for g in &groups {
                for &b in &g.members {
                    if lens[b] > t {
                        scores[b] += g.dist[cand_seqs[b][t] as usize];
                    }
                }
            }
            if t == max_len - 1 {
                break;
            }
            // 分桶裂变: 需要后续分布的成员按下一 token 分组。
            // **跨组合批**: 全部组的桶汇总为一次 forward_batch——
            // 裂变后常态是 13+ 单例组, 原各组独立调用 = 每步 13 次
            // g=1 前向, 每调固定开销（张量预置/克隆）独占
            // （实测 13×23字 批 171ms/call → 合批后 ~8×）
            let old_groups = std::mem::take(&mut groups);
            // (组下标, 桶 token, 成员)——组内桶互斥, 跨组可并行
            let mut plan: Vec<(usize, u32, Vec<usize>)> = Vec::new();
            for (gi, g) in old_groups.iter().enumerate() {
                let mut buckets: Vec<(u32, Vec<usize>)> = Vec::new();
                for &b in &g.members {
                    if lens[b] > t + 1 {
                        let tok = cand_seqs[b][t];
                        match buckets
                            .iter_mut()
                            .find(|(x, _)| *x == tok)
                        {
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
            let results = if std::env::var("NNLM_DBG").is_ok() {
                let t0 = std::time::Instant::now();
                let r = self.forward_batch(&toks, &shared, &oks, &ovs, t);
                FW_TIME.with(|c| {
                    *c.borrow_mut() += t0.elapsed().as_secs_f64()
                });
                FW_CALLS.with(|c| *c.borrow_mut() += 1);
                r
            } else {
                self.forward_batch(&toks, &shared, &oks, &ovs, t)
            };
            let mut next: Vec<Group> = Vec::with_capacity(n_bk);
            for ((_, _, members), (k2, v2, dist)) in
                plan.into_iter().zip(results)
            {
                next.push(Group { members, k: k2, v: v2, dist });
            }
            if next.is_empty() {
                break;
            }
            groups = next;
        }
        if std::env::var("NNLM_DBG").is_ok() {
            eprintln!(
                "[nnlm] forward_batch 累计 {:.1}ms / {} 次 = 均 {:.2}ms",
                FW_TIME.with(|c| *c.borrow()) * 1000.0,
                FW_CALLS.with(|c| *c.borrow()),
                FW_TIME.with(|c| *c.borrow()) * 1000.0
                    / FW_CALLS.with(|c| *c.borrow()).max(1) as f64
            );
        }
        scores.into_iter().map(|s| s as f64).collect()
    }
}

thread_local! {
    static FW_TIME: std::cell::RefCell<f64> =
        const { std::cell::RefCell::new(0.0) };
    static FW_CALLS: std::cell::RefCell<u64> =
        const { std::cell::RefCell::new(0) };
}

fn transpose(src: &[f32], n_in: usize, n_out: usize) -> Vec<f32> {
    let mut dst = vec![0f32; n_in * n_out];
    for i in 0..n_in {
        for o in 0..n_out {
            dst[o * n_in + i] = src[i * n_out + o];
        }
    }
    dst
}

// ----------------------------------------------------------------------
// 开放格式导出（feature = "safetensors"）—— hanhuo-nnlm schema v1
// 从黄金验证的原生模型直接序列化（构造即正确）; 轻量版 variant = "mobile"
#[cfg(feature = "safetensors")]
pub mod open_export {
    //! hanhuo-nnlm v1 开放格式导出（variant = "mobile"）
    use super::Nnlm;

    /// (shape, LE 字节, safetensors dtype 名)
    type T = (Vec<usize>, Vec<u8>, &'static str);

    impl Nnlm {
        /// 序列化为开放格式。fp16=true 权重转半精度
        /// （bos_logp/start_* 是内嵌 oracle 标定值，保持 f32）。
        pub fn to_safetensors(&self, fp16: bool) -> Vec<u8> {
            let mut t: std::collections::HashMap<String, T> =
                std::collections::HashMap::new();
            let put = |t: &mut std::collections::HashMap<String, T>,
                           name: &str, v: &[f32], shape: &[usize]| {
                t.insert(
                    name.to_string(),
                    (
                        shape.to_vec(),
                        v.iter().flat_map(|f| f.to_le_bytes()).collect(),
                        "F32",
                    ),
                );
            };
            put(&mut t, "wte", &self.wte, &[5594, 192]);
            put(&mut t, "wpe", &self.wpe, &[128, 192]);
            put(&mut t, "ln_f_g", &self.lnf_g, &[192]);
            put(&mut t, "ln_f_b", &self.lnf_b, &[192]);
            put(&mut t, "bos_logp", &self.bos_logp, &[5594]);
            put(&mut t, "start_k", &self.start_k, &[384]);
            put(&mut t, "start_v", &self.start_v, &[384]);
            for (li, l) in self.layers.iter().enumerate() {
                put(&mut t, &format!("blocks.{li}.attn_w"), &l.attn_w, &[576, 192]);
                put(&mut t, &format!("blocks.{li}.attn_b"), &l.attn_b, &[576]);
                put(&mut t, &format!("blocks.{li}.proj_w"), &l.proj_w, &[192, 192]);
                put(&mut t, &format!("blocks.{li}.proj_b"), &l.proj_b, &[192]);
                put(&mut t, &format!("blocks.{li}.ln1_g"), &l.ln1g, &[192]);
                put(&mut t, &format!("blocks.{li}.ln1_b"), &l.ln1b, &[192]);
                put(&mut t, &format!("blocks.{li}.ln2_g"), &l.ln2g, &[192]);
                put(&mut t, &format!("blocks.{li}.ln2_b"), &l.ln2b, &[192]);
                put(&mut t, &format!("blocks.{li}.fc_w"), &l.fc_w, &[768, 192]);
                put(&mut t, &format!("blocks.{li}.fc_b"), &l.fc_b, &[768]);
                put(&mut t, &format!("blocks.{li}.pr2_w"), &l.pr2_w, &[192, 768]);
                put(&mut t, &format!("blocks.{li}.pr2_b"), &l.pr2_b, &[192]);
            }
            if fp16 {
                t = half_map(t);
            }
            // 词表内嵌（单文件自足）: u8 字节流 + i64 偏移（id = 序号）
            {
                let mut items: Vec<(&str, u32)> = self
                    .vocab
                    .iter()
                    .map(|(k, v)| (k.as_str(), *v))
                    .collect();
                items.sort_by_key(|(_, id)| *id);
                let mut vbytes: Vec<u8> = Vec::new();
                let mut voff: Vec<i64> = vec![0];
                for (t2, _) in &items {
                    vbytes.extend_from_slice(t2.as_bytes());
                    voff.push(vbytes.len() as i64);
                }
                put_u8(&mut t, "vocab.bytes", &vbytes);
                put_i64(&mut t, "vocab.offsets", &voff);
            }
            let dtype = if fp16 { "F16" } else { "F32" };
            // 手工写 safetensors 容器: [u64 header_len][JSON header][data]
            // header: {name: {"dtype","shape","data_offsets"}, "__metadata__": {...}}
            let mut names: Vec<&String> = t.keys().collect();
            names.sort();
            let mut entries = Vec::new();
            let mut blob: Vec<u8> = Vec::new();
            let mut offsets = std::collections::HashMap::new();
            for n in &names {
                let (shape, data, dtyp) = &t[*n];
                let s = blob.len();
                blob.extend_from_slice(data);
                offsets.insert(
                    n.to_string(),
                    (s, blob.len(), shape.clone(), *dtyp),
                );
                entries.push((n.to_string(), shape.clone(), *dtyp));
            }
            let _ = dtype;
            let mut hdr = String::from("{");
            let meta = "\"__metadata__\":{\"format\":\"hanhuo-nnlm\",\"version\":\"1\",\"variant\":\"mobile\",\"n_layer\":\"2\",\"d_model\":\"192\",\"n_heads\":\"4\",\"head_dim\":\"48\",\"vocab_size\":\"5594\",\"n_ctx\":\"128\",\"eps\":\"1e-5\",\"gelu_c\":\"0.7978515625\",\"gelu_const_fp16\":\"true\",\"attn_scale\":\"6.928203\",\"gelu\":\"tanh\",\"license_note\":\"数据归腾讯，仅本地研究，禁止分发\"}";
            hdr.push_str(&meta);
            for (name, shape, dtyp) in &entries {
                let (s, e, _, _) = offsets[name];
                hdr.push_str(&format!(
                    ",\"{name}\":{{\"dtype\":\"{dtyp}\",\"shape\":{:?},\"data_offsets\":[{s},{e}]}}",
                    shape
                ));
            }
            hdr.push('}');
            let hb = hdr.as_bytes();
            let mut out = (hb.len() as u64).to_le_bytes().to_vec();
            out.extend_from_slice(hb);
            out.extend_from_slice(&blob);
            out
        }

    }

    fn put_u8(
        t: &mut std::collections::HashMap<String, T>,
        name: &str,
        v: &[u8],
    ) {
        t.insert(name.to_string(), (vec![v.len()], v.to_vec(), "U8"));
    }

    fn put_i64(
        t: &mut std::collections::HashMap<String, T>,
        name: &str,
        v: &[i64],
    ) {
        let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        t.insert(name.to_string(), (vec![v.len()], b, "I64"));
    }

    fn half_map(
        mut m: std::collections::HashMap<String, T>,
    ) -> std::collections::HashMap<String, T> {
        use half::f16;
        let keep = ["bos_logp", "start_k", "start_v"];
        let keys: Vec<String> = m
            .iter()
            .filter(|(k, _)| !keep.contains(&k.as_str()))
            .map(|(k, _)| k.clone())
            .collect();
        for k in keys {
            let (shape, data, _) = m.remove(&k).unwrap();
            let src: Vec<f32> = data
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let hb: Vec<u8> = src
                .iter()
                .flat_map(|f| f16::from_f32(*f).to_le_bytes())
                .collect();
            m.insert(k, (shape, hb, "F16"));
        }
        m
    }
}

#[cfg(feature = "safetensors")]
pub mod open_import {
    //! hanhuo-nnlm v1 开放格式装载（官方 safetensors crate 读取——
    //! 兼作自研 writer 的规范符合性验证）
    use super::{Layer, Nnlm, Past};
    use std::collections::HashMap;

    pub(crate) fn f32_of(st: &safetensors::SafeTensors, name: &str)
        -> Result<Vec<f32>, String> {
        let t = st.tensor(name).map_err(|e| format!("{name}: {e}"))?;
        if t.dtype() != safetensors::Dtype::F32 {
            return Err(format!("{name}: 期望 F32 实得 {:?}", t.dtype()));
        }
        Ok(t.data()
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }

    pub(crate) fn f16_of(st: &safetensors::SafeTensors, name: &str)
        -> Result<Vec<f32>, String> {
        use half::f16;
        let t = st.tensor(name).map_err(|e| format!("{name}: {e}"))?;
        match t.dtype() {
            safetensors::Dtype::F16 => Ok(t
                .data()
                .chunks_exact(2)
                .map(|c| f16::from_le_bytes([c[0], c[1]]).to_f32())
                .collect()),
            safetensors::Dtype::F32 => f32_of(st, name),
            d => Err(format!("{name}: 期望 F16/F32 实得 {d:?}")),
        }
    }

    /// 词表内嵌张量读取: vocab.bytes U8 + vocab.offsets I64（wte 行数+1）
    pub(crate) fn read_vocab(
        st: &safetensors::SafeTensors,
    ) -> Result<HashMap<String, u32>, String> {
        let vb = st.tensor("vocab.bytes")
            .map_err(|e| format!("vocab.bytes: {e}"))?;
        let voff = st.tensor("vocab.offsets")
            .map_err(|e| format!("vocab.offsets: {e}"))?;
        let bytes = vb.data();
        let offs: Vec<i64> = voff
            .data()
            .chunks_exact(8)
            .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let mut vocab = HashMap::with_capacity(offs.len() - 1);
        for i in 0..offs.len() - 1 {
            let tok = std::str::from_utf8(
                &bytes[offs[i] as usize..offs[i + 1] as usize],
            )
            .map_err(|e| format!("词表[{i}] UTF-8: {e}"))?;
            vocab.insert(tok.to_string(), i as u32);
        }
        Ok(vocab)
    }

    impl Nnlm {
        /// 从 hanhuo-nnlm v1 开放格式装载（mobile 变体——引擎原生维度）
        pub fn from_safetensors(bytes: &[u8]) -> Result<Nnlm, String> {
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
            if variant != "mobile" {
                return Err(format!(
                    "变体 {variant:?} 非 mobile——advanced 请用 \
                     nnlm_adv::NnlmAdv::from_safetensors（专属前向: \
                     past-concat KV / gelu_c 精确值 / scale 0.125）"
                ));
            }
            // 先探测文件精度（wte 权重 F16 = fp16 文件; bos/start 恒 F32）
            let f16_file = st.tensor("wte").unwrap().dtype()
                == safetensors::Dtype::F16;
            // P1b 量化文件: meta quant=int8 → Linear 权重/wte 为 I8 +
            // `<name>.scale` F32（per-out-channel 对称, QAT/PTQ 导出）。
            // 装载时反量化到 f32——下游推理零改动（盘上 −73%, 2.8MB 档）
            let quant_i8 = meta
                .metadata()
                .as_ref()
                .and_then(|m| m.get("quant"))
                .map(String::as_str)
                == Some("int8");
            if quant_i8 && f16_file {
                return Err("quant=int8 与 fp16 互斥".into());
            }
            let qd = |n: &str| -> Result<Vec<f32>, String> {
                let t = st.tensor(n).map_err(|e| format!("{n}: {e}"))?;
                let sh = t.shape();
                if sh.len() != 2 {
                    return Err(format!("{n}: 量化张量应 2 维 {sh:?}"));
                }
                let (rows, cols) = (sh[0], sh[1]);
                let data = t.data();
                let sc = st
                    .tensor(&format!("{n}.scale"))
                    .map_err(|e| format!("{n}.scale: {e}"))?;
                let sd_ = sc.data();
                let mut out = Vec::with_capacity(rows * cols);
                for r in 0..rows {
                    let s = f32::from_le_bytes([
                        sd_[4 * r],
                        sd_[4 * r + 1],
                        sd_[4 * r + 2],
                        sd_[4 * r + 3],
                    ]);
                    let row = &data[r * cols..(r + 1) * cols];
                    out.extend(row.iter().map(|&b| s * b as i8 as f32));
                }
                Ok(out)
            };
            // 量化名: wte + 各层四权重; 其余（偏置/ norm / bos / KV 种子）恒 f32
            let sel = |n: &str| -> Result<Vec<f32>, String> {
                let is_q = quant_i8
                    && (n == "wte"
                        || (n.starts_with("blocks.")
                            && ["attn_w", "proj_w", "fc_w", "pr2_w"]
                                .iter()
                                .any(|w| n.ends_with(w))));
                if is_q {
                    qd(n)
                } else if f16_file {
                    f16_of(&st, n)
                } else {
                    f32_of(&st, n)
                }
            };
            let ld = &sel;
            let wte = ld("wte")?;
            let wpe = ld("wpe")?;
            let lnf_g = ld("ln_f_g")?;
            let lnf_b = ld("ln_f_b")?;
            let bos = ld("bos_logp")?;
            let start_k = ld("start_k")?;
            let start_v = ld("start_v")?;
            let mut layers = Vec::new();
            for li in 0..2 {
                let p = |n: &str| format!("blocks.{li}.{n}");
                layers.push(Layer {
                    attn_w: ld(&p("attn_w"))?,
                    attn_b: ld(&p("attn_b"))?,
                    proj_w: ld(&p("proj_w"))?,
                    proj_b: ld(&p("proj_b"))?,
                    ln1g: ld(&p("ln1_g"))?,
                    ln1b: ld(&p("ln1_b"))?,
                    ln2g: ld(&p("ln2_g"))?,
                    ln2b: ld(&p("ln2_b"))?,
                    fc_w: ld(&p("fc_w"))?,
                    fc_b: ld(&p("fc_b"))?,
                    pr2_w: ld(&p("pr2_w"))?,
                    pr2_b: ld(&p("pr2_b"))?,
                });
            }

            // 词表张量（共享读取: u8 字节 + i64 偏移, 空洞=零长 token）
            let vocab = read_vocab(&st)?;

            let v = wte.len() / 192;
            if bos.len() != v {
                return Err(format!(
                    "bos_logp {}≠wte 行数 {v}",
                    bos.len()
                ));
            }
            Ok(Nnlm {
                v,
                wte,
                wpe,
                lnf_g,
                lnf_b,
                layers,
                vocab,
                bos_logp: bos,
                start_k,
                start_v,
                ctx_memo: std::cell::RefCell::new(HashMap::new()),
            })
        }
    }

    /// 开放格式往返对拍（fp32 位精确 / fp16 容差）——构造即正确的证明
    #[cfg(test)]
    mod tests {
        use super::*;

        fn native() -> Option<Nnlm> {
            let p = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
                format!(
                    "{}/tmp/hanhuo/wxq.bin",
                    std::env::var("HOME").unwrap_or_default()
                )
            });
            let pack = crate::pack::Pack::open(std::path::Path::new(&p)).ok()?;
            let sec = pack.raw(b"NNLM").ok()?;
            Nnlm::parse(sec).ok()
        }

        fn max_lp_diff(a: &Nnlm, b: &Nnlm, steps: &[u32]) -> f32 {
            let (pa0, mut pa) = a.predict(steps[0], None);
            let (pb0, mut pb) = b.predict(steps[0], None);
            let mut m = pa0
                .iter()
                .zip(&pb0)
                .fold(0f32, |x, (x1, x2)| x.max((x1 - x2).abs()));
            for &t in &steps[1..] {
                let (a1, na) = a.predict(t, Some(&pa));
                let (b1, nb) = b.predict(t, Some(&pb));
                pa = na;
                pb = nb;
                m = m.max(
                    a1.iter().zip(&b1).fold(0f32, |x, (x1, x2)| {
                        x.max((x1 - x2).abs())
                    }),
                );
            }
            m
        }

        #[test]
        #[ignore = "需要 wxq.bin"]
        fn test_open_format_roundtrip() {
            let m = native().expect("原生 NNLM 解析");
            // fp32: 位精确（同一 predict 代码路径 + 同一 f32 权重）
            let b32 = m.to_safetensors(false);
            let m32 = Nnlm::from_safetensors(&b32)
                .unwrap_or_else(|e| panic!("fp32 装载: {e}"));
            assert_eq!(m32.vocab.len(), 5594);
            assert_eq!(m.vocab, m32.vocab, "词表往返应一致");
            let steps: Vec<u32> = [100u32, 500, 2000, 4000].to_vec();
            let d = max_lp_diff(&m, &m32, &steps);
            assert_eq!(d, 0.0, "fp32 往返应位精确, 实测 {d}");
            // fp16: 容差（半精度权重 + f32 推理）
            let b16 = m.to_safetensors(true);
            let m16 = Nnlm::from_safetensors(&b16).expect("fp16 装载");
            let d16 = max_lp_diff(&m, &m16, &steps);
            assert!(d16 < 0.05, "fp16 往返容差, 实测 {d16}");
        }
    }

    // Past 仅测试用途引用（保持编译器满意）
    #[allow(dead_code)]
    fn _past_type_assert(_: &Past) {}
}
