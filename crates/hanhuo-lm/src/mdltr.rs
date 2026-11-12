//! mdltr LightGBM (lambdarank, PC 131 树) 纯 Rust 推理器——
//! mdltr_inference.py 的忠实移植（官方推理 0x1808A0920 对照 35/35 逐位一致）。
//!
//! 语义:
//!   - 每树 62 内点(0=根)/63 叶；left/right 负值 = ~leaf_id
//!   - decision_type bit0 = categorical（thr 存 cat_boundaries slot）；
//!     其余 = 数值节点 v <= thr → 左
//!   - 类别: cat=int(v); slot=thr; 位图 [cb[slot], cb[slot+1]) u32 数组;
//!     0<=cat<n*32 且 bit 命中 → 左，否则右（cat<0/超范围 → 右）
//!   - 输出 = Σ_trees leaf_value（raw score，官方 f32 喂入路径）
//!   - 特征 169..178：**已接**（score.rs ltr_rerank 按 FEATURES.md 第二/四役
//!     语义填充：169/170=单字/多字粗类 2/2 与 13/4，171..178=score/rank/
//!     delta/lm 族；官方推理 35/35 + 黄金端到端验证。旧"wtid 组装未破"
//!     系被七役特征语义战取代的陈账——wtid(Wfea) 保留为研究资产
//!
//! MDLTR 区布局（pack.py encode_mdltr）:
//!   [n_trees u32][n_feat u32] × tree { feat u8×62 | dt i8×62+pad | left/right
//!   i32×62 | thr f64×62 | leaf f64×63 | n_cb+cb u32 | n_ct+ct u32 }
use crate::pack::FcStrings;
use crate::pack::U32Arr;

pub const N_TREES: usize = 131;
pub const N_FEAT: usize = 179;

pub(crate) struct TreeSec {
    pub(crate) feat: &'static [u8],  // 62
    pub(crate) dt: &'static [u8],    // 62
    pub(crate) left: &'static [u8],  // 62×4
    pub(crate) right: &'static [u8], // 62×4
    pub(crate) thr: &'static [u8],   // 62×8
    pub(crate) leaf: &'static [u8],  // 63×8
    pub(crate) cb: &'static [u8],    // n×4
    pub(crate) ct: &'static [u8],    // n×4
}

#[inline]
pub(crate) fn i32at(b: &[u8], i: usize) -> i32 {
    let p = i * 4;
    i32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

#[inline]
pub(crate) fn u32at(b: &[u8], i: usize) -> u32 {
    let p = i * 4;
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

#[inline]
pub(crate) fn f64at(b: &[u8], i: usize) -> f64 {
    let p = i * 8;
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[p..p + 8]);
    f64::from_le_bytes(a)
}

pub struct Mdltr {
    pub(crate) trees: Vec<TreeSec>,
}

impl Mdltr {
    pub fn parse(sec: &'static [u8]) -> Result<Mdltr, String> {
        if sec.len() < 8 {
            return Err("MDLT 过短".into());
        }
        let le = |p: usize| -> u32 {
            u32::from_le_bytes([sec[p], sec[p + 1], sec[p + 2], sec[p + 3]])
        };
        let n_trees = le(0) as usize;
        let n_feat = le(4) as usize;
        if n_trees != N_TREES || n_feat != N_FEAT {
            return Err(format!("MDLT 形状 {n_trees}×{n_feat} ≠ 131×179"));
        }
        let mut trees = Vec::with_capacity(n_trees);
        let mut o = 8usize;
        for _ in 0..n_trees {
            let take = |o: &mut usize, n: usize| -> Option<&'static [u8]> {
                if *o + n > sec.len() {
                    return None;
                }
                let s = &sec[*o..*o + n];
                *o += n;
                Some(s)
            };
            let feat = take(&mut o, 64).ok_or("MDLT feat 截断")?;
            let dt = take(&mut o, 68).ok_or("MDLT dt 截断")?;
            let left = take(&mut o, 248).ok_or("MDLT left 截断")?;
            let right = take(&mut o, 248).ok_or("MDLT right 截断")?;
            let thr = take(&mut o, 496).ok_or("MDLT thr 截断")?;
            let leaf = take(&mut o, 504).ok_or("MDLT leaf 截断")?;
            if o + 4 > sec.len() {
                return Err("MDLT cb 头截断".into());
            }
            let n_cb = le(o) as usize;
            o += 4;
            let cb =
                take(&mut o, n_cb * 4).ok_or("MDLT cb 截断")?;
            if o + 4 > sec.len() {
                return Err("MDLT ct 头截断".into());
            }
            let n_ct = le(o) as usize;
            o += 4;
            let ct =
                take(&mut o, n_ct * 4).ok_or("MDLT ct 截断")?;
            trees.push(TreeSec {
                feat: &feat[..62],
                dt: &dt[..62],
                left,
                right,
                thr,
                leaf,
                cb,
                ct,
            });
        }
        Ok(Mdltr { trees })
    }

    /// 官方 raw score = Σ 131 树叶值（树序累加，与 Python 逐位一致）
    pub fn predict(&self, x: &[f32; N_FEAT]) -> f64 {
        forest_score(self, x)
    }
}

// ----------------------------------------------------------------------
// 共享树游走——原生字节版（Mdltr）与开放格式平坦向量版（MdltrOpen，
// mdltr_safetensors.rs）走同一实现，杜绝两套逻辑漂移。
pub trait TreeTable {
    fn n_trees(&self) -> usize;
    fn feat_at(&self, t: usize, n: usize) -> u8;
    fn dt_at(&self, t: usize, n: usize) -> u8;
    fn left_at(&self, t: usize, n: usize) -> i32;
    fn right_at(&self, t: usize, n: usize) -> i32;
    fn thr_at(&self, t: usize, n: usize) -> f64;
    fn leaf_at(&self, t: usize, n: usize) -> f64;
    fn cb_at(&self, t: usize, i: usize) -> u32;
    fn ct_at(&self, t: usize, i: usize) -> u32;
}

/// 单树游走（0=根；left/right 负值 = ~leaf_id；语义同 LightGBM lambdarank）
pub fn tree_score<T: TreeTable + ?Sized>(tt: &T, t: usize, x: &[f32; N_FEAT]) -> f64 {
    let mut node: i32 = 0;
    while node >= 0 {
        let n = node as usize;
        let d = tt.dt_at(t, n);
        let v = x[tt.feat_at(t, n) as usize] as f64; // f32→f64（官方喂入路径）
        let go = if d & 1 != 0 {
            // categorical: cat=int(v), slot=thr, 位图命中 → 左
            let cat = v as i64; // 截断向零（Python int()）
            let slot = tt.thr_at(t, n) as i32 as usize;
            let base = tt.cb_at(t, slot) as usize;
            let nend = tt.cb_at(t, slot + 1) as usize;
            let hit = cat >= 0
                && (cat as usize) < (nend - base) * 32
                && (tt.ct_at(t, base + ((cat as usize) >> 5))
                    >> ((cat as u32) & 31))
                    & 1
                    == 1;
            if hit {
                tt.left_at(t, n)
            } else {
                tt.right_at(t, n)
            }
        } else if v <= tt.thr_at(t, n) {
            tt.left_at(t, n)
        } else {
            tt.right_at(t, n)
        };
        if go < 0 {
            return tt.leaf_at(t, !go as usize);
        }
        node = go;
    }
    0.0
}

/// 森林得分 = Σ 树叶值（树序 f64 累加，位序与官方一致）
pub fn forest_score<T: TreeTable + ?Sized>(tt: &T, x: &[f32; N_FEAT]) -> f64 {
    let mut s = 0.0;
    for t in 0..tt.n_trees() {
        s += tree_score(tt, t, x);
    }
    s
}

impl TreeTable for Mdltr {
    fn n_trees(&self) -> usize {
        self.trees.len()
    }
    fn feat_at(&self, t: usize, n: usize) -> u8 {
        self.trees[t].feat[n]
    }
    fn dt_at(&self, t: usize, n: usize) -> u8 {
        self.trees[t].dt[n]
    }
    fn left_at(&self, t: usize, n: usize) -> i32 {
        i32at(self.trees[t].left, n)
    }
    fn right_at(&self, t: usize, n: usize) -> i32 {
        i32at(self.trees[t].right, n)
    }
    fn thr_at(&self, t: usize, n: usize) -> f64 {
        f64at(self.trees[t].thr, n)
    }
    fn leaf_at(&self, t: usize, n: usize) -> f64 {
        f64at(self.trees[t].leaf, n)
    }
    fn cb_at(&self, t: usize, i: usize) -> u32 {
        u32at(self.trees[t].cb, i)
    }
    fn ct_at(&self, t: usize, i: usize) -> u32 {
        u32at(self.trees[t].ct, i)
    }
}

/// WFEA: wtid 11B 词特征表（[n][off n+1 → front-code 词区][feat u8×11×n]）
pub struct Wfea {
    pub fc: FcStrings,
    feats: &'static [u8],
}

impl Wfea {
    pub fn parse(sec: &'static [u8]) -> Result<Wfea, String> {
        let le = |p: usize| -> u32 {
            u32::from_le_bytes([sec[p], sec[p + 1], sec[p + 2], sec[p + 3]])
        };
        if sec.len() < 8 {
            return Err("WFEA 过短".into());
        }
        let n = le(0) as usize;
        let off_len = 4 * (n + 1);
        if 4 + off_len > sec.len() {
            return Err("WFEA off 截断".into());
        }
        let off = U32Arr::new(&sec[4..4 + off_len]);
        let fc_len = off.get(n) as usize;
        let fc_blob = &sec[4 + off_len..4 + off_len + fc_len];
        let feats = &sec[4 + off_len + fc_len..];
        if feats.len() < 11 * n {
            return Err("WFEA feat 截断".into());
        }
        Ok(Wfea {
            fc: FcStrings::new(fc_blob, off),
            feats,
        })
    }

    pub fn len(&self) -> usize {
        self.fc.n
    }

    /// 词 → 11B 特征（二分，词表排序）
    pub fn get(&self, word: &str) -> Option<[u8; 11]> {
        let (mut a, mut b) = (0usize, self.fc.n);
        while a < b {
            let mid = (a + b) / 2;
            let k = self.fc.get(mid);
            if k.as_str() < word {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        if a < self.fc.n && self.fc.get(a) == word {
            let p = a * 11;
            Some([
                self.feats[p],
                self.feats[p + 1],
                self.feats[p + 2],
                self.feats[p + 3],
                self.feats[p + 4],
                self.feats[p + 5],
                self.feats[p + 6],
                self.feats[p + 7],
                self.feats[p + 8],
                self.feats[p + 9],
                self.feats[p + 10],
            ])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdltr_refs::REFS;

    /// 官方推理对照基准（Unicorn 0x1808A0920 验证 35/35 的种子向量）
    #[test]
    fn official_ref_vectors() {
        let bin = std::path::PathBuf::from(
            std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap();
                format!("{home}/tmp/hanhuo/wxq.bin")
            }),
        );
        let pack = crate::pack::Pack::open(&bin).expect("bin");
        let sec = pack.raw(b"MDLT").expect("MDLT 区");
        let m = Mdltr::parse(sec).expect("parse");
        for (x, want) in REFS {
            let got = m.predict(x);
            assert!(
                (got - want).abs() < 1e-9,
                "predict {got} ≠ 官方 {want}"
            );
        }
    }
}
