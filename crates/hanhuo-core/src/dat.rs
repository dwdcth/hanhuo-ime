//! L2 读音索引：Double-Array Trie 运行期游走（读取侧；构建在 Python build 侧）.
//!
//! 渐进前缀：每状态子树叶子区间 [lo, lo+cnt)（叶子=读音 id，DFS 序）。
//! 终端位图：key==前缀 精确命中。cnt u16 + 溢出表（cnt≥65535 的浅状态）。

use crate::pack::{BitArr, U16Arr};
use std::collections::HashMap;

pub const ROOT: i32 = 1;

pub struct Dat {
    /// v4: 精确位宽位打包（base/check 值域 < n_slots, lo < n_keys）
    pub base: BitArr,
    pub check: BitArr,
    pub lo: BitArr,
    pub cnt16: U16Arr,
    pub term: &'static [u8],
    pub n_slots: usize,
    pub n_keys: usize,
    pub cnt_ovf: HashMap<u32, u32>,
}

impl Dat {
    #[inline]
    pub fn cnt_of(&self, state: i32) -> u32 {
        if let Some(&c) = self.cnt_ovf.get(&(state as u32)) {
            c
        } else {
            self.cnt16.get(state as usize) as u32
        }
    }

    /// 沿键字串走查；不可达 → -1（键仅 a-z）
    #[inline]
    pub fn walk(&self, key: &[u8], state: i32) -> i32 {
        let mut st = state as u32;
        for &ch in key {
            // 键域先判后减（py ord(c)-96 对非字母返回越界值 → 不可达;
            // 先减后判在 u32 debug 构建下会对 <'a' 字节下溢 panic——
            // tick62 镜像哨兵 '__adv_b_skip__' 实测触发, 行为不变）
            if !(b'a'..=b'z').contains(&ch) {
                return -1;
            }
            let c = ch as u32 - 96;
            let ns = self.base.get(st as usize) + c;
            if ns as usize >= self.n_slots
                || self.check.get(ns as usize) != st
            {
                return -1;
            }
            st = ns;
        }
        st as i32
    }

    #[inline]
    pub fn is_terminal(&self, state: i32) -> bool {
        if state <= 0 || state as usize >= self.n_slots {
            return false;
        }
        let s = state as usize;
        self.term[s >> 3] & (1 << (s & 7)) != 0
    }

    /// 子树叶子区间 (lo, cnt)
    #[inline]
    pub fn range(&self, state: i32) -> (u32, u32) {
        (self.lo.get(state as usize), self.cnt_of(state))
    }

    /// 沿 s 走查产出 (depth, state)：s[:depth] 为完整键的终端态（升序，DP 用）
    pub fn iter_terminal(&self, s: &[u8]) -> Vec<(usize, i32)> {
        let mut out = Vec::new();
        let mut st = ROOT as u32;
        if self.is_terminal(ROOT) {
            out.push((0usize, ROOT));
        }
        for (i, &ch) in s.iter().enumerate() {
            // 键域先判后减（与 walk 同律: 非 a-z 在 u32 debug 下减 96 下溢 panic,
            // 哨兵路径实测可触发——tick62 教训的 iter_terminal 镜像修复）
            if !(b'a'..=b'z').contains(&ch) {
                return out;
            }
            let c = ch as u32 - 96;
            let ns = self.base.get(st as usize) + c;
            if ns as usize >= self.n_slots
                || self.check.get(ns as usize) != st
            {
                return out;
            }
            st = ns;
            if self.is_terminal(st as i32) {
                out.push((i + 1, st as i32));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// w 位定宽小端位场编码 → BitArr（+8B u64 读窗余量）
    fn bitarr(vals: &[u32], w: u32) -> BitArr {
        let nbits = vals.len() * w as usize;
        let mut b = vec![0u8; nbits / 8 + 9];
        for (i, &v) in vals.iter().enumerate() {
            let bit = i * w as usize;
            let x = (v as u64) << (bit & 7);
            let byte = bit >> 3;
            for k in 0..8 {
                b[byte + k] |= (x >> (8 * k)) as u8;
            }
        }
        BitArr::new(Box::leak(b.into_boxed_slice()), w)
    }

    fn u16arr(vals: &[u16]) -> U16Arr {
        let mut b = Vec::new();
        for v in vals {
            b.extend_from_slice(&v.to_le_bytes());
        }
        U16Arr::new(Box::leak(b.into_boxed_slice()))
    }

    /// 手工微型 DAT: 键 a/ab/b（DFS 叶序 a=0, ab=1, b=2）
    /// slot: 1=ROOT, 3="a"(check 1), 4="b"(check 1), 7="ab"(check 3)
    fn tiny() -> Dat {
        Dat {
            base: bitarr(&[0, 2, 0, 5, 0, 0, 0, 0], 4),
            check: bitarr(&[0, 0, 0, 1, 1, 0, 0, 3], 4),
            lo: bitarr(&[0, 0, 0, 0, 2, 0, 0, 1], 4),
            cnt16: u16arr(&[0, 3, 0, 2, 1, 0, 0, 1]),
            term: Box::leak(vec![0b1001_1000u8].into_boxed_slice()), // 槽 3/4/7
            n_slots: 8,
            n_keys: 3,
            cnt_ovf: HashMap::new(),
        }
    }

    #[test]
    fn dat_walk_and_terminal() {
        let d = tiny();
        assert_eq!(d.walk(b"a", ROOT), 3);
        assert_eq!(d.walk(b"ab", ROOT), 7);
        assert_eq!(d.walk(b"b", ROOT), 4);
        // 不可达: 'c' 无槽、延续断裂、非 a-z 哨兵（tick62 镜像防下溢路径）
        assert_eq!(d.walk(b"c", ROOT), -1);
        assert_eq!(d.walk(b"ba", ROOT), -1);
        assert_eq!(d.walk(b"A", ROOT), -1);
        assert_eq!(d.walk(b"a!", ROOT), -1);
        assert!(d.is_terminal(3) && d.is_terminal(4) && d.is_terminal(7));
        assert!(!d.is_terminal(ROOT) && !d.is_terminal(0));
        assert!(!d.is_terminal(-1) && !d.is_terminal(8)); // 越界假安全
    }

    #[test]
    fn dat_range_and_overflow() {
        let mut d = tiny();
        assert_eq!(d.range(ROOT), (0, 3)); // 全子树
        assert_eq!(d.range(3), (0, 2)); // a 的子树: a, ab
        assert_eq!(d.range(7), (1, 1));
        assert_eq!(d.range(4), (2, 1));
        // 溢出表覆盖 cnt16（cnt≥65535 的浅状态）
        d.cnt_ovf.insert(7, 70000);
        assert_eq!(d.cnt_of(7), 70000);
        assert_eq!(d.cnt_of(3), 2); // 未覆盖者走 cnt16
    }

    #[test]
    fn dat_iter_terminal() {
        let d = tiny();
        // 前缀途中的终端态升序产出（DP 消费契约）
        assert_eq!(d.iter_terminal(b"ab"), vec![(1usize, 3i32), (2, 7)]);
        assert_eq!(d.iter_terminal(b"abc"), vec![(1, 3), (2, 7)]); // 'c' 断
        assert_eq!(d.iter_terminal(b"ba"), vec![(1, 4)]); // 'a' 自 b 不可达
        assert_eq!(d.iter_terminal(b"aX"), vec![(1, 3)]); // 非字母截断
        assert!(d.iter_terminal(b"c").is_empty());
    }
}
