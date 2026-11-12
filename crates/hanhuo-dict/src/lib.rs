//! hanhuo-dict —— 词库资产层
//!
//! 与 hanhuo-lm（模型层）平级的字典容器层：
//! - bhash: 桶定位哈希（CityHash32 家族，表族共用）
//! - ckvd: ckvdmac.bin 长尾短语词典（642 万短语）
//! - auxform: 辅助码码表（wubi86_swbd 字码 + rsd 笔顺, 融合-辅助码 §3.2）
//!
//! 后续词库资产（c4g / slwd / bwcn / bwcnen 等）按同模式落位本 crate。
//! 数据归腾讯，仅本地个人使用，禁止分发。

// array_chunks 未 stable；表长度恒为 4 倍数，chunks_exact 等价
#![allow(clippy::chunks_exact_to_as_chunks)]

pub mod bhash;
pub mod auxform;
pub mod bwcn;
pub mod ckvd;
pub mod smallfeat;
pub mod stt;
pub mod wxhash;

pub use auxform::AuxForm;
pub use ckvd::{CkvdPhrases, Phrase};
pub use stt::{S2t, S2tAny, S2tTsv};

