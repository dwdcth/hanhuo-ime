//! 汉火输入法公共类型与常量族（crate 依赖图最底层，零依赖）.
//!
//! Cand/Kind（候选与来源类型）+ 引擎旋钮常量（B 档逆向成果：
//! 编辑罚十一旋钮 / FB_* 兜底通道常量 / KBN_MASK / 混合召回权重）与
//! 纯函数（face_override / lp_override_for / mix_recall_forward / kbn_neighbors）。
pub mod cand;
pub mod constants;
pub mod kind;

pub use cand::Cand;
pub use kind::Kind;
