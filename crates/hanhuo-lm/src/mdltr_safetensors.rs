//! mdltr LightGBM 开放格式（hanhuo-mdltr v1）——导出/装载/推理
//!
//! 树模型非张量，但 safetensors 的整数/浮点张量足以无损表达：
//!   feat I8 [n_trees, 62]   分裂特征号
//!   dt   I8 [n_trees, 62]   decision_type（bit0 = categorical）
//!   left/right I32 [n_trees, 62]  子节点（负值 = ~leaf_id）
//!   thr  F64 [n_trees, 62]  阈值（categorical 节点 = cat_boundaries 槽号）
//!   leaf F64 [n_trees, 63]  叶值
//!   cat_boundaries U32 [Σ] + cat_boundaries.offsets I64 [n_trees+1]
//!   cat_bitmaps    U32 [Σ] + cat_bitmaps.offsets    I64 [n_trees+1]
//!   （每树变长 cb/ct 拼接 + 偏移，与 NNLM 词表内嵌同构）
//!
//! 全整数/F64 存储 → 往返**位精确**（无 fp16 精度话题）。
//! 推理走 mdltr::TreeTable 共享游走，语义与原生官方对拍一致。
//!
//! 数据归腾讯，仅本地个人研究使用，禁止分发。
#![cfg(feature = "safetensors")]

use crate::mdltr::{f64at, forest_score, i32at, Mdltr, TreeTable, N_FEAT};

const N_NODE: usize = 62; // 内点数（0=根）
const N_LEAF: usize = 63;

/// 开放格式 mdltr（owned 平坦向量版）
pub struct MdltrOpen {
    n_trees: usize,
    feat: Vec<u8>, // [t][n]
    dt: Vec<u8>,
    left: Vec<i32>,
    right: Vec<i32>,
    thr: Vec<f64>,
    leaf: Vec<f64>,
    cb: Vec<u32>,
    cb_off: Vec<usize>, // n_trees+1
    ct: Vec<u32>,
    ct_off: Vec<usize>,
}

/// 手写 safetensors 容器: [u64 header_len][JSON][blob]
/// meta_pair 形如 `"__metadata__":{...}`（不含外层花括号）
fn write_container(
    tensors: &[(&str, Vec<usize>, &[u8], &str)],
    meta_pair: &str,
) -> Vec<u8> {
    let mut names: Vec<&str> = tensors.iter().map(|t| t.0).collect();
    names.sort();
    let mut entries = format!("{{{meta_pair}");
    let mut blob = Vec::new();
    for n in &names {
        let (_, shape, data, dtype) = tensors.iter().find(|t| t.0 == *n).unwrap();
        let s = blob.len();
        blob.extend_from_slice(data);
        entries.push_str(&format!(
            ",\"{n}\":{{\"dtype\":\"{dtype}\",\"shape\":[{}],\
             \"data_offsets\":[{s},{}]}}",
            shape.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","),
            blob.len()
        ));
    }
    entries.push('}');
    let jb = entries.into_bytes();
    let mut out = (jb.len() as u64).to_le_bytes().to_vec();
    out.extend_from_slice(&jb);
    out.extend_from_slice(&blob);
    out
}

impl Mdltr {
    /// 原生 MDLTR 区 → hanhuo-mdltr v1 safetensors
    pub fn to_safetensors(&self) -> Vec<u8> {
        let nt = self.trees.len();
        let mut feat = Vec::with_capacity(nt * N_NODE);
        let mut dt = Vec::with_capacity(nt * N_NODE);
        let mut left = Vec::with_capacity(nt * N_NODE * 4);
        let mut right = Vec::with_capacity(nt * N_NODE * 4);
        let mut thr = Vec::with_capacity(nt * N_NODE * 8);
        let mut leaf = Vec::with_capacity(nt * N_LEAF * 8);
        let mut cb: Vec<u8> = Vec::new();
        let mut ct: Vec<u8> = Vec::new();
        let mut cb_off: Vec<i64> = vec![0];
        let mut ct_off: Vec<i64> = vec![0];
        for t in &self.trees {
            feat.extend_from_slice(&t.feat[..N_NODE]);
            dt.extend_from_slice(&t.dt[..N_NODE]);
            for n in 0..N_NODE {
                left.extend_from_slice(&i32at(t.left, n).to_le_bytes());
                right.extend_from_slice(&i32at(t.right, n).to_le_bytes());
                thr.extend_from_slice(&f64at(t.thr, n).to_le_bytes());
            }
            for n in 0..N_LEAF {
                leaf.extend_from_slice(&f64at(t.leaf, n).to_le_bytes());
            }
            cb.extend_from_slice(t.cb);
            ct.extend_from_slice(t.ct);
            cb_off.push((cb.len() / 4) as i64);
            ct_off.push((ct.len() / 4) as i64);
        }
        let meta_pair = format!(
            "\"__metadata__\":{{\"format\":\"hanhuo-mdltr\",\"version\":\"1\",\
             \"n_trees\":\"{nt}\",\"n_feat\":\"{N_FEAT}\",\
             \"n_node\":\"{N_NODE}\",\"n_leaf\":\"{N_LEAF}\",\
             \"semantics\":\"lambdarank sum-of-leaves; left<0=~leaf_id; \
             v<=thr→left; categorical: cat位图命中→left\",\
             \"license_note\":\"数据归腾讯，仅本地研究，禁止分发\"}}"
        );
        write_container(
            &[
                ("feat", vec![nt, N_NODE], &feat, "I8"),
                ("dt", vec![nt, N_NODE], &dt, "I8"),
                ("left", vec![nt, N_NODE], &left, "I32"),
                ("right", vec![nt, N_NODE], &right, "I32"),
                ("thr", vec![nt, N_NODE], &thr, "F64"),
                ("leaf", vec![nt, N_LEAF], &leaf, "F64"),
                ("cat_boundaries", vec![cb.len() / 4], &cb, "U32"),
                ("cat_bitmaps", vec![ct.len() / 4], &ct, "U32"),
                (
                    "cat_boundaries.offsets",
                    vec![nt + 1],
                    &cb_off.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
                    "I64",
                ),
                (
                    "cat_bitmaps.offsets",
                    vec![nt + 1],
                    &ct_off.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
                    "I64",
                ),
            ],
            &meta_pair,
        )
    }
}

impl MdltrOpen {
    /// 还原为原生 Mdltr（重排 MDLT 段字节 → Mdltr::parse; 开放格式
    /// 本为原生段位精确导出 → 往返无损。分体布局（字典/模型分离）的
    /// 引擎注入路径。
    pub fn into_native(self) -> Result<crate::mdltr::Mdltr, String> {
        let mut sec: Vec<u8> = Vec::with_capacity(8 + self.n_trees * 1600);
        sec.extend_from_slice(&(self.n_trees as u32).to_le_bytes());
        sec.extend_from_slice(&(crate::mdltr::N_FEAT as u32).to_le_bytes());
        for t in 0..self.n_trees {
            // feat: [t][62 节点] 特征号 u8 → 64B 段（62 用 + 2 填充）
            sec.extend_from_slice(&self.feat[t * 62..(t + 1) * 62]);
            sec.extend_from_slice(&[0u8; 2]);
            // dt: 62B + 6 填充 = 68B 段
            sec.extend_from_slice(&self.dt[t * 62..(t + 1) * 62]);
            sec.extend_from_slice(&[0u8; 6]);
            for v in &self.left[t * 62..(t + 1) * 62] {
                sec.extend_from_slice(&v.to_le_bytes());
            }
            for v in &self.right[t * 62..(t + 1) * 62] {
                sec.extend_from_slice(&v.to_le_bytes());
            }
            for v in &self.thr[t * 62..(t + 1) * 62] {
                sec.extend_from_slice(&v.to_le_bytes());
            }
            for v in &self.leaf[t * 63..(t + 1) * 63] {
                sec.extend_from_slice(&v.to_le_bytes());
            }
            let (cb0, cb1) = (self.cb_off[t], self.cb_off[t + 1]);
            let n_cb = (cb1 - cb0) as u32;
            sec.extend_from_slice(&n_cb.to_le_bytes());
            sec.extend_from_slice(
                &self.cb[cb0..cb1].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
            );
            let (ct0, ct1) = (self.ct_off[t], self.ct_off[t + 1]);
            let n_ct = (ct1 - ct0) as u32;
            sec.extend_from_slice(&n_ct.to_le_bytes());
            sec.extend_from_slice(
                &self.ct[ct0..ct1].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
            );
        }
        let sec: &'static [u8] = Box::leak(sec.into_boxed_slice());
        crate::mdltr::Mdltr::parse(sec)
    }

    /// 从 hanhuo-mdltr v1 开放格式装载（位精确还原）
    pub fn from_safetensors(bytes: &[u8]) -> Result<MdltrOpen, String> {
        use safetensors::Dtype;
        let st = safetensors::SafeTensors::deserialize(bytes)
            .map_err(|e| format!("容器解析: {e}"))?;
        let (_, meta) = safetensors::SafeTensors::read_metadata(bytes)
            .map_err(|e| format!("元数据: {e}"))?;
        let fmt = meta
            .metadata()
            .as_ref()
            .and_then(|m| m.get("format"))
            .map(String::as_str)
            .unwrap_or("");
        if fmt != "hanhuo-mdltr" {
            return Err(format!("格式 {fmt:?} 非 hanhuo-mdltr"));
        }
        let read = |name: &str, want: Dtype| -> Result<Vec<u8>, String> {
            let t = st.tensor(name).map_err(|e| format!("{name}: {e}"))?;
            if t.dtype() != want {
                return Err(format!(
                    "{name}: dtype {:?} 与预期 {want:?} 不符",
                    t.dtype()
                ));
            }
            Ok(t.data().to_vec())
        };
        let feat = read("feat", Dtype::I8)?;
        let dt = read("dt", Dtype::I8)?;
        let left: Vec<i32> = read("left", Dtype::I32)?
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let right: Vec<i32> = read("right", Dtype::I32)?
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let thr: Vec<f64> = read("thr", Dtype::F64)?
            .chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let leaf: Vec<f64> = read("leaf", Dtype::F64)?
            .chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let cb: Vec<u32> = read("cat_boundaries", Dtype::U32)?
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let ct: Vec<u32> = read("cat_bitmaps", Dtype::U32)?
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let offs = |name: &str, total: usize| -> Result<Vec<usize>, String> {
            let raw = read(name, Dtype::I64)?;
            let v: Vec<i64> = raw
                .chunks_exact(8)
                .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            let v = if v.len() == total + 1 {
                v
            } else {
                return Err(format!(
                    "{name}: 长度 {} ≠ n_trees+1 {}",
                    v.len(),
                    total + 1
                ));
            };
            for w in v.windows(2) {
                if w[0] > w[1] || w[0] < 0 {
                    return Err(format!("{name}: 非单调或负值"));
                }
            }
            Ok(v.iter().map(|&x| x as usize).collect())
        };
        let nt = left.len() / N_NODE;
        if nt == 0
            || feat.len() != nt * N_NODE
            || dt.len() != nt * N_NODE
            || right.len() != nt * N_NODE
            || thr.len() != nt * N_NODE
            || leaf.len() != nt * N_LEAF
        {
            return Err(format!("张量长度不一致（n_trees 推导 {nt}）"));
        }
        let cb_off = offs("cat_boundaries.offsets", nt)?;
        let ct_off = offs("cat_bitmaps.offsets", nt)?;
        if cb_off[nt] != cb.len() || ct_off[nt] != ct.len() {
            return Err("cat offsets 末端与拼接长度不符".into());
        }
        Ok(MdltrOpen {
            n_trees: nt,
            feat,
            dt,
            left,
            right,
            thr,
            leaf,
            cb,
            cb_off,
            ct,
            ct_off,
        })
    }

    pub fn n_trees(&self) -> usize {
        self.n_trees
    }

    /// 与原生 Mdltr::predict 同一游走（TreeTable 共享实现）
    pub fn predict(&self, x: &[f32; N_FEAT]) -> f64 {
        forest_score(self, x)
    }
}

impl TreeTable for MdltrOpen {
    fn n_trees(&self) -> usize {
        self.n_trees
    }
    fn feat_at(&self, t: usize, n: usize) -> u8 {
        self.feat[t * N_NODE + n]
    }
    fn dt_at(&self, t: usize, n: usize) -> u8 {
        self.dt[t * N_NODE + n]
    }
    fn left_at(&self, t: usize, n: usize) -> i32 {
        self.left[t * N_NODE + n]
    }
    fn right_at(&self, t: usize, n: usize) -> i32 {
        self.right[t * N_NODE + n]
    }
    fn thr_at(&self, t: usize, n: usize) -> f64 {
        self.thr[t * N_NODE + n]
    }
    fn leaf_at(&self, t: usize, n: usize) -> f64 {
        self.leaf[t * N_LEAF + n]
    }
    fn cb_at(&self, t: usize, i: usize) -> u32 {
        self.cb[self.cb_off[t] + i]
    }
    fn ct_at(&self, t: usize, i: usize) -> u32 {
        self.ct[self.ct_off[t] + i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造合成 MDLTR 区（131 树 × 179 维满足 parse 形状门）：
    /// tree0 数值分裂、tree1 categorical、其余 129 棵平凡（恒左→leaf0）
    fn synthetic_section() -> &'static [u8] {
        let mut sec = Vec::new();
        sec.extend_from_slice(&131u32.to_le_bytes());
        sec.extend_from_slice(&(N_FEAT as u32).to_le_bytes());
        for ti in 0..131 {
            let mut feat = vec![0u8; 64];
            let mut dt = vec![0u8; 68];
            let mut left = vec![0u8; 248];
            let mut right = vec![0u8; 248];
            let mut thr = vec![0u8; 496];
            let mut leaf = vec![0u8; 504];
            let put_i32 = |buf: &mut Vec<u8>, n: usize, v: i32| {
                buf[n * 4..n * 4 + 4].copy_from_slice(&v.to_le_bytes());
            };
            let put_f64 = |buf: &mut Vec<u8>, n: usize, v: f64| {
                buf[n * 8..n * 8 + 8].copy_from_slice(&v.to_le_bytes());
            };
            match ti {
                0 => {
                    // x[1] <= 0.5 → leaf0=1.5, 否则 leaf1=-2.25
                    feat[0] = 1;
                    put_f64(&mut thr, 0, 0.5);
                    put_i32(&mut left, 0, -1);
                    put_i32(&mut right, 0, -2);
                    put_f64(&mut leaf, 0, 1.5);
                    put_f64(&mut leaf, 1, -2.25);
                }
                1 => {
                    // categorical: x[2]∈{0,2} → 左(0.75), 其余 → 右(-0.5)
                    feat[0] = 2;
                    dt[0] = 1;
                    put_f64(&mut thr, 0, 0.0); // slot 0
                    put_i32(&mut left, 0, -1);
                    put_i32(&mut right, 0, -2);
                    put_f64(&mut leaf, 0, 0.75);
                    put_f64(&mut leaf, 1, -0.5);
                }
                _ => {
                    // 平凡: thr=+inf 恒左 → leaf0=0.1
                    put_f64(&mut thr, 0, f64::INFINITY);
                    put_i32(&mut left, 0, -1);
                    put_f64(&mut leaf, 0, 0.1);
                }
            }
            let (cb, ct) = if ti == 1 {
                (vec![0u32, 1], vec![0b101u32]) // cat {0,2} 位图
            } else {
                (vec![0u32, 0], vec![])
            };
            sec.extend_from_slice(&feat);
            sec.extend_from_slice(&dt);
            sec.extend_from_slice(&left);
            sec.extend_from_slice(&right);
            sec.extend_from_slice(&thr);
            sec.extend_from_slice(&leaf);
            sec.extend_from_slice(&(cb.len() as u32).to_le_bytes());
            for v in cb {
                sec.extend_from_slice(&v.to_le_bytes());
            }
            sec.extend_from_slice(&(ct.len() as u32).to_le_bytes());
            for v in ct {
                sec.extend_from_slice(&v.to_le_bytes());
            }
        }
        Box::leak(sec.into_boxed_slice())
    }

    #[test]
    fn synthetic_native_and_roundtrip() {
        let m = Mdltr::parse(synthetic_section()).expect("原生解析");
        // 平凡树基数: 129 × 0.1
        let base = 129.0 * 0.1;
        let mk = |f1: f32, f2: f32| {
            let mut x = [0f32; N_FEAT];
            x[1] = f1;
            x[2] = f2;
            x
        };
        // 语义手算（f64 累加序不同 → 1e-9 容差; 位精确留给 native↔open 对比）
        let want = |a: f64, b: f64| base + a + b;
        for (x, w) in [
            (mk(0.3, 0.0), want(1.5, 0.75)),  // 数值左 + cat0 命中
            (mk(0.7, 1.0), want(-2.25, -0.5)), // 数值右 + cat1 未命中
            (mk(0.3, 2.0), want(1.5, 0.75)),  // cat2 命中
            (mk(0.3, -1.0), want(1.5, -0.5)), // cat<0 走右
        ] {
            let got = m.predict(&x);
            assert!((got - w).abs() < 1e-9, "{got} ≠ 手算 {w}");
        }

        // 开放格式往返: 探针 f64 位精确
        let bytes = m.to_safetensors();
        let mo = MdltrOpen::from_safetensors(&bytes).expect("开放装载");
        assert_eq!(mo.n_trees(), 131);
        for x in [mk(0.3, 0.0), mk(0.7, 1.0), mk(0.3, 2.0), mk(0.3, -1.0)] {
            assert_eq!(mo.predict(&x), m.predict(&x), "开放格式应位精确 (f64)");
        }
    }

    /// 真模型往返: wxq.bin MDLT 区导出→装载→REFS 6 向量位精确
    #[test]
    #[ignore = "需要 wxq.bin"]
    fn open_format_roundtrip_real() {
        use crate::mdltr_refs::REFS;
        let p = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
            format!(
                "{}/tmp/hanhuo/wxq.bin",
                std::env::var("HOME").unwrap_or_default()
            )
        });
        let pack = crate::pack::Pack::open(std::path::Path::new(&p)).expect("bin");
        let sec = pack.raw(b"MDLT").expect("MDLT 区");
        let m = Mdltr::parse(sec).expect("parse");
        let bytes = m.to_safetensors();
        let mo = MdltrOpen::from_safetensors(&bytes).expect("装载");
        assert_eq!(mo.n_trees(), 131);
        for (x, want) in REFS {
            let native = m.predict(x);
            let open = mo.predict(x);
            assert_eq!(native, open, "开放格式往返应位精确");
            assert!((open - want).abs() < 1e-9, "{open} ≠ 官方 {want}");
        }
    }
}
