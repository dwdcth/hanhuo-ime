//! 桶定位哈希（CityHash32 家族，微信 HashMapReader 表族共用）
//!
//! 双通道构成（2026-11-07 ckvd 战役实证）：
//! - ≤24B：MSVC 强度折减 fold（Android libwxhld sub_1F7C400 逐字转录，
//!   ckvdmac 12,382/12,382 = 100.000% 桶一致 + wsd 4/4 实证；
//!   PC sub_180B560A0 同语义）
//! - \>24B：Google CityHash32 long path（city.cc 参考，ckvd 长键 13/13 实证）
//!
//! 桶内比对用 k2（wxhash，种子 0x1DBBA5）——本表族律：桶=bhash / 比对=k2。
//! 数据归腾讯，仅本地个人使用，禁止分发。


const C1: u32 = 0xCC9E_2D51;
const C2: u32 = 0x1B87_3593;
const CC: u32 = 0xE654_6B64;
/// (C1 rotl 15) 折叠常数 —— MSVC 把 rot(C1*a,17) 强度削减成 (KROT*a)|((C1*a)>>17)
const KROT: u32 = 380_141_568;

#[inline]
const fn ror(x: u32, r: u32) -> u32 {
    x.rotate_right(r)
}

#[inline]
const fn mix(a: u32) -> u32 {
    (KROT.wrapping_mul(a)) | ((C1.wrapping_mul(a)) >> 17)
}

#[inline]
const fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

#[inline]
const fn rd32(p: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([p[o], p[o + 1], p[o + 2], p[o + 3]])
}

/// ≤24B 路径：MSVC 折减 fold（bucket_hash.py 逐字移植；公共尾部 fold+fmix）
fn bhash_short(p: &[u8]) -> u32 {
    let n = p.len();
    if n <= 4 {
        // 有符号字节链: b = b*C1 + int8(v), c ^= b (c 初值 9)
        let mut b: u32 = 0;
        let mut c: u32 = 9;
        for &ch in p {
            let v = ch as i8 as i32 as u32;
            b = v.wrapping_add(C1.wrapping_mul(b));
            c ^= b;
        }
        let t = c ^ C2.wrapping_mul(mix(n as u32));
        let h = C2.wrapping_mul(mix(b)) ^ (5u32.wrapping_mul(ror(t, 19)).wrapping_add(CC));
        fmix32(5u32.wrapping_mul(ror(h, 19)).wrapping_add(CC))
    } else if n <= 12 {
        let a = rd32(p, 0).wrapping_add(n as u32);
        let b = rd32(p, n - 4).wrapping_add(5u32.wrapping_mul(n as u32));
        let c = rd32(p, (n >> 1) & 4).wrapping_add(9);
        let t = C2.wrapping_mul(mix(a)) ^ (5u32.wrapping_mul(n as u32));
        let h1 = (5u32.wrapping_mul(ror(t, 19)).wrapping_add(CC))
            ^ C2.wrapping_mul(mix(b));
        let h2 = (5u32.wrapping_mul(ror(h1, 19)).wrapping_add(CC))
            ^ C2.wrapping_mul(mix(c));
        fmix32(5u32.wrapping_mul(ror(h2, 19)).wrapping_add(CC))
    } else {
        // 13..24: 6 抽样窗 [mid-4],[4],[n-8],[mid],[0],[n-4]
        let mid = n >> 1;
        let q = [
            rd32(p, mid - 4),
            rd32(p, 4),
            rd32(p, n - 8),
            rd32(p, mid),
            rd32(p, 0),
            rd32(p, n - 4),
        ];
        let mut t = C2.wrapping_mul(mix(q[0])) ^ (n as u32);
        for &x in &q[1..5] {
            t = (5u32.wrapping_mul(ror(t, 19)).wrapping_add(CC)) ^ C2.wrapping_mul(mix(x));
        }
        let h = (5u32.wrapping_mul(ror(t, 19)).wrapping_add(CC)) ^ C2.wrapping_mul(mix(q[5]));
        fmix32(5u32.wrapping_mul(ror(h, 19)).wrapping_add(CC))
    }
}

/// >24B 路径：Google CityHash32 long（city.cc 参考，PERMUTE3(f,h,g)=SWAP(f,h);SWAP(f,g)）
fn bhash_long(s: &[u8]) -> u32 {
    let len = s.len() as u32;
    let n = s.len();
    let mut h = len;
    let mut g = C1.wrapping_mul(len);
    let mut f = g;
    let a0 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, n - 4)).rotate_right(17));
    let a1 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, n - 8)).rotate_right(17));
    let a2 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, n - 16)).rotate_right(17));
    let a3 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, n - 12)).rotate_right(17));
    let a4 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, n - 20)).rotate_right(17));
    h ^= a0;
    h = h.rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    h ^= a2;
    h = h.rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    g ^= a1;
    g = g.rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    g ^= a3;
    g = g.rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    f = f.wrapping_add(a4).rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    let iters = (n - 1) / 20;
    let mut o = 0usize;
    for _ in 0..iters {
        let a0 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, o)).rotate_right(17));
        let a1 = rd32(s, o + 4);
        let a2 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, o + 8)).rotate_right(17));
        let a3 = C2.wrapping_mul(C1.wrapping_mul(rd32(s, o + 12)).rotate_right(17));
        let a4 = rd32(s, o + 16);
        h ^= a0;
        h = h.rotate_right(18).wrapping_mul(5).wrapping_add(CC);
        f = f.wrapping_add(a1).rotate_right(19).wrapping_mul(C1);
        g = g.wrapping_add(a2).rotate_right(18).wrapping_mul(5).wrapping_add(CC);
        h ^= a3.wrapping_add(a1);
        h = h.rotate_right(19).wrapping_mul(5).wrapping_add(CC);
        g ^= a4;
        g = g.swap_bytes().wrapping_mul(5);
        h = h.wrapping_add(a4.wrapping_mul(5)).swap_bytes();
        f = f.wrapping_add(a0);
        // PERMUTE3(f, h, g)
        let (nf, nh, ng) = (g, f, h);
        f = nf;
        h = nh;
        g = ng;
        o += 20;
    }
    g = g.rotate_right(11).wrapping_mul(C1).rotate_right(17).wrapping_mul(C1);
    f = f.rotate_right(11).wrapping_mul(C1).rotate_right(17).wrapping_mul(C1);
    h = h.wrapping_add(g).rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    h = h.rotate_right(17).wrapping_mul(C1);
    h = h.wrapping_add(f).rotate_right(19).wrapping_mul(5).wrapping_add(CC);
    h.rotate_right(17).wrapping_mul(C1)
}

/// 桶定位哈希（全长度）
pub fn bhash(p: &[u8]) -> u32 {
    if p.len() <= 24 {
        bhash_short(p)
    } else {
        bhash_long(p)
    }
}

/// 桶号
pub fn bucket_of(p: &[u8], n_buckets: u32) -> u32 {
    bhash(p) % n_buckets
}

/// T9 九宫格映射（sub_1807DE680 逐字移植；+48 成 ASCII 数字）
/// 返回 None 表示映射到哨兵 1（原生对非字母非数字返回 1）
pub fn t9_digit(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'r' => (c - b'a') / 3 + 2,
        b's' => 7,
        b't'..=b'v' => 8,
        b'w'..=b'z' => 9,
        _ => 1,
    }
}

/// 把字母串映射成 T9 数字串（跳过逗号，逗号在原生比对中被剔除）
pub fn to_t9(s: &str) -> String {
    s.bytes()
        .filter(|&b| b != b',')
        .map(|b| t9_digit(b) + b'0')
        .map(|b| b as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_smoke() {
        // 各长度路径冒烟（不崩 + 确定性）
        for len in [0usize, 1, 4, 5, 12, 13, 24, 25, 30, 45, 65] {
            let key: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
            let h1 = bhash(&key);
            let h2 = bhash(&key);
            assert_eq!(h1, h2, "len={len}");
        }
    }

    #[test]
    fn test_t9_map() {
        assert_eq!(t9_digit(b'a'), 2);
        assert_eq!(t9_digit(b'c'), 2);
        assert_eq!(t9_digit(b's'), 7);
        assert_eq!(t9_digit(b'z'), 9);
        assert_eq!(to_t9("nihao"), "64426");
        assert_eq!(to_t9("ni,hao"), "64426");
    }

    #[test]
    fn test_known_values() {
        // 与 Python 参考对拍锚点（ckvd 战役实测值）
        // bucket_hash.py: bhash("nihao") % 624348 == 569802
        assert_eq!(bucket_of(b"nihao", 624_348), 569_802);
        // wsd 4/4 锚: bhash("阿a") % 5144 == 2132
        assert_eq!(bucket_of("阿a".as_bytes(), 5144), 2132);
        assert_eq!(bucket_of("那na".as_bytes(), 5144), 288);
    }
}
