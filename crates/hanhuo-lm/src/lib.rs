//! 汉火输入法语言模型与容器层.
//!
//! pack（wxq.bin v1 mmap 容器与列式数组）/ nnlm（TFLite NNLM 打分）/
//! mdltr（LightGBM lambdarank 重排）/ mdltr_refs（回归特征参考表）/
//! lmcn（cascade 目录 lmcn 域表与面打分）。
// clippy: 浮点常量为 Python 位精确镜像（golden 逐位对齐）——精度超长是有意为之
#![allow(clippy::excessive_precision)]
pub mod lmcn;
pub mod mdltr;
pub mod mdltr_refs;
pub mod nnlm;
pub mod pack;

/// SafeTensors 可选格式（feature = "safetensors"）：
/// NNLM/lmcn 的 fp16/fp32 独立装载器（部署优化线，产物在 data/，
/// 提取工具 tools/safetensors/）。主引擎默认走 pack.rs 原生格式。
#[cfg(feature = "safetensors")]
pub mod lmcn_safetensors;
#[cfg(feature = "safetensors")]
pub mod nnlm_adv;
#[cfg(feature = "safetensors")]
pub mod mdltr_safetensors;
#[cfg(feature = "safetensors")]
pub mod bgrm_safetensors;
/// 知意双条件选词模型（Apache-2.0 开源资产, clean-room 自写推理,
/// 规格源=官方 README+test_vectors）——candle 纯 Rust 前向。
#[cfg(feature = "zhiyi")]
pub mod zhiyi;

