//! lmcn 原厂四段量化解码（cascade/lmcn_val_formula.md 终破公式的忠实实现）.
//!
//! val1 ↔ log10P: N = −log10P × 1024，四段指数步长（1/2/4/8，边界 49152/50176/51200，
//! 基座 47104）。26,103 单字条目逐点验证（1e-5 容差）——公式封闭。

pub const BASE: u32 = 47104;
pub const B1: u32 = 49152;
pub const B2: u32 = 50176;
pub const B3: u32 = 51200;
pub const VMAX: u32 = 51742;
pub const QUANT: f64 = 1024.0;

#[inline]
pub fn val_to_n(v: u16) -> i64 {
    let v = v as u32;
    if v < B1 {
        v as i64 - BASE as i64 // 先转 i64 再减（u32 域下溢防护, v<BASE 时为负）
    } else if v < B2 {
        2048 + 2 * (v - B1) as i64
    } else if v < B3 {
        4096 + 4 * (v - B2) as i64
    } else {
        8192 + 8 * (v - B3) as i64
    }
}

#[inline]
pub fn val_to_log10p(v: u16) -> f64 {
    -(val_to_n(v) as f64) / QUANT
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 四段量化公式边界与单调性（26,103 单字逐点验证的抽样固化）
    #[test]
    fn quant_boundaries() {
        // 基座: v=BASE → N=0 → log10P=0
        assert_eq!(val_to_n(BASE as u16), 0);
        assert_eq!(val_to_log10p(BASE as u16), 0.0);
        // v<BASE → 负 N（u32 域下溢防护路径）
        assert_eq!(val_to_n((BASE - 1) as u16), -1);
        assert!((val_to_log10p((BASE - 1) as u16) - 1.0 / 1024.0).abs() < 1e-12);
        // 段边界连续性: B1 段尾 +1 步进 == 段2 头
        assert_eq!(val_to_n((B1 - 1) as u16), 2047);
        assert_eq!(val_to_n(B1 as u16), 2048);
        assert_eq!(val_to_n((B2 - 1) as u16), 2048 + 2 * 1023);
        assert_eq!(val_to_n(B2 as u16), 4096);
        assert_eq!(val_to_n((B3 - 1) as u16), 4096 + 4 * 1023);
        assert_eq!(val_to_n(B3 as u16), 8192);
        // VMAX 锚点
        assert_eq!(val_to_n(VMAX as u16), 8192 + 8 * (VMAX - B3) as i64);
        assert!((val_to_log10p(VMAX as u16) + 12528.0 / 1024.0).abs() < 1e-9);
    }

    #[test]
    fn quant_monotone_and_steps() {
        let mut prev = i64::MIN;
        for v in (BASE..=VMAX).step_by(7) {
            let n = val_to_n(v as u16);
            assert!(n >= prev, "v={v} N 递减");
            prev = n;
        }
        // 各段步长 1/2/4/8
        for (lo, hi, step) in [
            (BASE, B1, 1u32),
            (B1, B2, 2),
            (B2, B3, 4),
            (B3, B3 + 64, 8),
        ] {
            let d = val_to_n((lo + 1) as u16) - val_to_n(lo as u16);
            assert_eq!(d, step as i64, "段 [{lo},{hi}) 步长");
        }
        // log10P 恒 ≤ 0（概率域）
        for v in [BASE, B1, B2, B3, VMAX] {
            assert!(val_to_log10p(v as u16) <= 0.0);
        }
    }
}
