//! k2 哈希（wxhash.py 逐位镜像；libwxhld 0x180b56490/0x180b569e0）
//!
//! HashMapReader 表族（0x80000001）桶内比对哈希：桶=bhash（bhash.rs）、
//! 比对=k2（本文件）。2026-11-07 自 hanhuo-core/engine/ntgen.rs 下沉至
//! 词库层（stt 等表族资产查询需要；core 经依赖反向引用）。
//! 数据归腾讯，仅本地个人使用，禁止分发。

const M64: u64 = u64::MAX;
const P0M: u64 = 0x9ae1_6a3b_2f90_404f;
const P1: u64 = (-0x6220_15f7_14c7_d297i64) as u64;
const P2M: u64 = (-0x4b6d_4990_4167_0d8di64) as u64;
const P3M: u64 = (-0x1e95_c4d0_6fbf_b100i64) as u64;
const P4M: u64 = (-0x3c5a_37a3_6834_ced9i64) as u64;

#[inline]
fn rotl(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

#[inline]
fn rd64(p: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(p[o..o + 8].try_into().unwrap())
}

#[inline]
fn rd32(p: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(p[o..o + 4].try_into().unwrap())
}

/// wyhash body（len ≤ 32 路径；>32 截断到前 32B——资产键 ≤32B 防御）
pub fn wyhash_body(p: &[u8]) -> u64 {
    let ln = p.len();
    if ln > 0x20 {
        return wyhash_body(&p[..0x20]);
    }
    if 16 < ln {
        let see = (ln as u64)
            .wrapping_mul(2)
            .wrapping_sub(0x651e_95c4_d06f_bfb1);
        let u1 = see.wrapping_mul(rd64(p, ln - 8));
        let u2a = rd64(p, 8).wrapping_add(P0M) & M64;
        let u7 = (rotl(u2a, 46)
            .wrapping_add(u1)
            .wrapping_add(rd64(p, 0).wrapping_mul(P2M)))
            & M64;
        let mut u2 = rd64(p, 8)
            .wrapping_add(rd64(p, 0).wrapping_mul(P2M))
            & M64;
        u2 = ((rotl(u2, 21)
            .wrapping_add(rotl(u1, 34))
            .wrapping_add(rd64(p, ln - 16).wrapping_mul(P0M) ^ u7))
        .wrapping_mul(see))
            & M64;
        u2 = (((u2 >> 47) ^ u2 ^ u7).wrapping_mul(see)) & M64;
        (((u2 >> 47) ^ u2).wrapping_mul(see)) & M64
    } else if 8 <= ln {
        let u1 = rd64(p, 0).wrapping_add(P0M) & M64;
        let see = (ln as u64)
            .wrapping_mul(2)
            .wrapping_sub(0x651e_95c4_d06f_bfb1);
        let mut u2 = rd64(p, ln - 8);
        let u7 = rotl(u1, 39).wrapping_add(u2).wrapping_mul(see) & M64;
        u2 = (rotl(u2, 27).wrapping_mul(see).wrapping_add(u1) ^ u7)
            .wrapping_mul(see)
            & M64;
        u2 = (((u2 >> 47) ^ u2 ^ u7).wrapping_mul(see)) & M64;
        (((u2 >> 47) ^ u2).wrapping_mul(see)) & M64
    } else if 4 <= ln {
        let see = (ln as u64)
            .wrapping_mul(2)
            .wrapping_sub(0x651e_95c4_d06f_bfb1);
        let u1 = rd32(p, ln - 4) as u64;
        let mut u2 = ((ln as u64).wrapping_add((rd32(p, 0) as u64).wrapping_mul(8)) ^ u1) & M64;
        u2 = u2.wrapping_mul(see) & M64;
        u2 = (((u2 >> 47) ^ u2 ^ u1).wrapping_mul(see)) & M64;
        (((u2 >> 47) ^ u2).wrapping_mul(see)) & M64
    } else if ln != 0 {
        let b_mid = p[ln >> 1] as u64;
        let b_last = p[ln - 1] as u64;
        let mut u2 = (b_mid.wrapping_mul(P3M))
            .wrapping_add((p[0] as u64).wrapping_mul(P0M))
            & M64;
        u2 ^= ((ln as u64).wrapping_add(b_last.wrapping_mul(4)).wrapping_mul(P4M)) & M64;
        ((u2 >> 47) ^ u2).wrapping_mul(P0M) & M64
    } else {
        P0M
    }
}

/// k2 finalizer（0x1dbba5 mixer；键 ≤32B）
pub fn k2(p: &[u8]) -> u32 {
    let h = wyhash_body(p);
    let seed: u64 = 0x1DBBA5;
    let mut v = ((h.wrapping_add(0x651e_95c4_d06f_bfb1) & M64) ^ seed)
        .wrapping_mul(P1)
        & M64;
    v = (((v >> 47) ^ v) ^ seed).wrapping_mul(P1) & M64;
    v = (((v >> 47) ^ v).wrapping_mul(P1)) & M64;
    (v & 0xFFFF_FFFF) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k2_vectors() {
        // py 对拍向量（与 ntgen 原测试同源）:
        // k2('ab') == 0xf1318f59, k2('abcdef') == 0x869ac9b0,
        // k2('津田') == 0xc847e26b, k2('进提案') == 0x583efec6
        assert_eq!(k2(b"ab"), 0xf131_8f59);
        assert_eq!(k2(b"abcdef"), 0x869a_c9b0);
        assert_eq!(k2("津田".as_bytes()), 0xc847_e26b);
        assert_eq!(k2("进提案".as_bytes()) & 0xFFFF_FFFF, 0x583e_fec6);
        // py 直拍: k2('中国') == 0xc0d111d3
        assert_eq!(k2("中国".as_bytes()), 0xc0d1_11d3);
    }

    #[test]
    fn test_paths() {
        // 各长度路径冒烟
        for len in [0usize, 1, 3, 4, 7, 8, 15, 16, 17, 31, 32, 40] {
            let key: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
            let _ = k2(&key);
        }
    }
}
