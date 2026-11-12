# SafeTensors 开放格式工具链（feature = "safetensors"）

模型资产统一走 `hanhuo-nnlm` / `hanhuo-lmcn` 开放格式（safetensors v1）：
开放标准、单文件自包含（词表内嵌为张量）、fp32/fp16 双精度，
脱离 wxq.bin 原生容器即可分发与优化。原生 wxq.bin 为腾讯专有格式不可
分发，本格式是开源链（open/）的模型侧对应物。

## NNLM 双变体（用户可选轻量版 / 高级版）

| 变体 | 来源 | 结构 | 文件 |
|---|---|---|---|
| mobile（轻量） | wxq.bin `NNLM` 段（nlrgptand） | 2 层 pre-LN GPT-2, d192, 4H×48, ctx 128, vocab 5594 | `nnlm_mobile_{fp32,fp16}.safetensors` |
| advanced（高级） | BuildIn `nlrgpt2_nnlm.bin`（nlrgpt2） | 3 层 GPT-2, d256, 4H×64, ctx 512, vocab 5594（33 保留/空洞位） | `nnlm_advanced_{fp32,fp16}.safetensors` |

- **mobile 导出**：`hanhuo export-nnlm`（CLI，从原生 Nnlm 直接序列化，
  构造即正确）→ `crates/hanhuo-lm/src/nnlm.rs` `open_export`
- **advanced 导出**：`python3 export_nnlm_advanced.py`（xnet npz 权威槽位表，
  无转置——npz 槽位已 out×in；旧提取脚本槽位错位已废弃）
- **Rust 装载与推理**：mobile=`Nnlm::from_safetensors`；
  advanced=`nnlm_adv::NnlmAdv`（独立模块——past-concat KV / gelu_c 精确值
  / scale 0.125 / z 签名头，与 mobile 差异过大不混写）；fp32/fp16 皆可
- **验证**：mobile `test_open_format_roundtrip`（fp32 位精确）；
  advanced `test_numpy_golden`（numpy f64 基准 4 case 含 11 步深 cache，
  top-32 序精确 + 探针 |Δlp|≤5e-4，基准由 `gen_nnlm_adv_golden.py` 生成）
- **精度事实（四资产直审定档 2026-11-07，防"原厂 int8"再问）**：全部非
  int8，且均为**原始文件直读**（不经转码链）——
  nlrgpt2_nnlm.bin/nnlm_model.tflite 全 f32（flatbuffer 张量表 269 F32/
  0 INT8/带 scale 张量=0，interpreter 对拍 cos=1.0）；wxq.bin NNLM 段
  f16 LE（段格式本身，golden 对齐）；Android nnlm1145.bin 全 fp16
  （blob 普查 44/44 权重 2B/元素=6.13MB=文件全部权重字节，dtype 码表
  无 int8、无 scale/zp；唯一 uint8 blob=128×128 下三角掩码）。
  轻量版 fp16、高级版 f32 即原厂部署形态；家族中唯一 int8+hybrid 的是
  wltm（Android 词图，另一条线）。未来 int8 只会是**自主量化优化**
  （safetensors 第三精度档），非原生还原。CLI 探针：`hanhuo nnlm-adv 我们一起`

### 词表内嵌 schema（两变体一致）

- `vocab.bytes` U8：按 id 序拼接的 token UTF-8 字节
- `vocab.offsets` I64：**wte 行数 + 1** 个偏移；token i =
  `bytes[offsets[i]..offsets[i+1]]`（advanced 稀疏 id 的空洞/保留位
  为零长 token，行号=索引不变式保持）

## mdltr LightGBM（hanhuo-mdltr v1）

树模型非张量，但整数/F64 张量可**位精确**表达（无 fp16 话题）：
- 张量：`feat/dt I8[131,62]`、`left/right I32[131,62]`、`thr F64[131,62]`、
  `leaf F64[131,63]`；每树变长 cb/ct 拼接 + `*.offsets I64[132]`
  （与 NNLM 词表内嵌同构）
- **导出**：`hanhuo export-mdltr`（wxq.bin MDLT 区 → `data/mdltr.safetensors`）
- **装载/推理**：`mdltr_safetensors::MdltrOpen`——走 `mdltr::TreeTable`
  共享游走（原生字节版与开放向量版同一实现，杜绝双逻辑漂移）
- **验证**：合成森林语义测试（数值/类别/负叶编码）+ 真模型 REFS
  6 官方向量往返 **f64 位精确**（`assert_eq!`）

## lmcn n-gram

- `extract_lmcn_safetensors.py` —— fp32/fp16（**已验证**：loader 6/6，
  fp16 损失 <1e-5，内存 -33%）
- `verify_lmcn_safetensors.py` —— 双格式一致性校验
- Rust 侧：`crates/hanhuo-lm/src/lmcn_safetensors.rs`

产物落位 `../../data/`（LFS `*.safetensors`）。
数据归腾讯，仅本地个人研究使用，禁止分发。
