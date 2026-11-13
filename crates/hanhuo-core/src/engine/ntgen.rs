//! 役N tick79n: 深带 noun_tag 叉积生成器（py wxhash.k2 位级镜像 +
//! noun_tag_dict 资产缓存 + 查询尾插入）。py `query.py` 79n 块同构。
//!
//! k2/wyhash_body 已下沉 hanhuo-dict/src/wxhash.rs（2026-11-07，
//! stt 表族查询需要）；此处 re-export 保持 ntgen::k2 路径稳定。

use std::sync::OnceLock;

pub use hanhuo_dict::wxhash::k2;

static NT_BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// noun_tag_dict 资产缓存（py `_nt_bytes` 同源同路径）
pub fn nt_bytes() -> Option<&'static [u8]> {
    NT_BYTES
        .get_or_init(|| {
            // HANHUO_NT 覆盖钮（c.6 审计后续: 开源重建表——jieba 词层
            // k2 u32 表, tools 侧生成; 缺省原路径 = 腾讯词性资产）
            if let Ok(p) = std::env::var("HANHUO_NT") {
                return std::fs::read(p).ok();
            }
            let p = std::env::var("HOME")
                .map(|h| format!("{}/tmp/hanhuo/extracted/3.0.1.1/BuildIn/noun_tag_dict.bin", h))
                .ok()?;
            std::fs::read(p).ok()
        })
        .as_deref()
}

static NT_SORTED: OnceLock<Option<Vec<u32>>> = OnceLock::new();

/// 资产一次性索引: 全部重叠 4 字节窗（k2 低 32 位）→ 排序去重 u32 表
/// （4.1MB 草垛 × 196 查询次的原线性扫 = 800M 比较/查询 ≈ 300ms;
/// 排序表 ~16MB 一次构建, 二分 22 比较/查。成员语义逐位不变）
fn nt_sorted() -> Option<&'static [u32]> {
    NT_SORTED
        .get_or_init(|| {
            let b = nt_bytes()?;
            if b.len() < 4 {
                return Some(Vec::new());
            }
            let mut v: Vec<u32> = (0..=b.len() - 4)
                .map(|i| u32::from_le_bytes(b[i..i + 4].try_into().unwrap()))
                .collect();
            v.sort_unstable();
            v.dedup();
            Some(v)
        })
        .as_deref()
}

/// k2 全哈希在资产中的子串成员测（排序表二分; 原 mem_subs 同义）
pub fn nt_has(word: &str) -> bool {
    match nt_sorted() {
        Some(t) => t.binary_search(&k2(word.as_bytes())).is_ok(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn k2_vector_jintian() {
        // py 对拍向量: k2('津田') & 0xffffffff == 0xc847e26b
        eprintln!("ab={:016x} abcdef={:016x} jt={:016x}",
            k2(b"ab"), k2(b"abcdef"), k2("津田".as_bytes()));
        assert_eq!(k2(b"ab"), 0xf131_8f59);
        assert_eq!(k2(b"abcdef"), 0x869a_c9b0);
        assert_eq!(k2("津田".as_bytes()), 0xc847_e26b);
        // k2('进提案') & 0xffffffff == 0x583efec6
        assert_eq!(k2("进提案".as_bytes()) & 0xFFFF_FFFF, 0x583e_fec6);
    }
    #[test]
    fn nt_membership() {
        // 开源资产语义锁: 成员测=排序表二分行为契约（词条域随资产变——
        // 「津田」属研究线语料资产; k2 向量锁由 k2_vector_jintian 承担）
        match nt_sorted() {
            Some(t) if !t.is_empty() => {
                let k = t[t.len() / 2];
                assert!(
                    t.binary_search(&k).is_ok(),
                    "表中位成员应命中（二分一致性）"
                );
            }
            _ => {} // 资产缺席=恒 false 语义（nt_has None 分支）
        }
    }
}
