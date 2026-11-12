//! 汉火输入法引擎核心（脑）.
//!
//! wxq（Python 参考实现，archive win/tools/wxassets/wxq/）的忠实 Rust 移植。
//! 分层: hanhuo-lm（pack 容器 / nnlm / mdltr / lmcn）/ 本 crate:
//! dat（读音 DAT）/ syllables（L1 切分）/ quant（原厂四段量化）/
//! engine（四层查询管线，类型与常量族在 hanhuo-types）。
//!
//! 数据归腾讯，仅本地个人使用，禁止分发。
pub use hanhuo_lm::{mdltr, mdltr_refs, nnlm, pack};
pub use hanhuo_lm::mdltr::Mdltr;
pub use hanhuo_lm::nnlm::Nnlm;

pub mod dat;
pub mod engine;
pub mod quant;
pub mod syllables;
pub mod vmode;
pub mod userdict;

pub use engine::{BoostCfg, Cand, Engine, Kind, default_bin, mix_recall_forward};
pub use userdict::{Entry, UserDict, user_boost};
