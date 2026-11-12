# hanhuo-ime

开源中文输入法引擎（Rust）——拼音 / 简拼 / T9 九宫 / 中英混输 / 语境联想，
可编译到 WASM 在浏览器运行。全部数据与模型基于开源语料构建，不含任何
 proprietary 资产。

## 能力

| 能力 | 说明 |
|---|---|
| 全拼 / 简拼 | DAT 读音索引，四层管线（切分→索引→路由→打分） |
| T9 九宫 | 键串→音节签名展开→DAT 制导 DFS；邻键纠错（错击恢复） |
| 语境 NNLM | 自训 2 层 GPT（LCCC 语料），候选重排 / 句序列重排（宽档 boost） |
| 英文 | 词表候选（频序 / enlm 词级 LM 语境序） |
| 联想 | 中文 bigram（LCCC 统计）+ 英文词对（wikitext-103） |
| LTR 精排 | 自训 LightGBM lambdarank（v4 语境 +4.0pp） |
| t9_lr | 九宫候选重排 LR（自训，开源 t9_coef 等价物） |
| 敏感词 / emoji / 错别字 / 简繁 | 全开源数据表 |
| WASM | `hanhuo-wasm` crate → 浏览器端到端（web/app 演示应用） |

## 快速开始

```bash
# 构建
cargo build --release -p hanhuo-cli

# 词典构建（首次）: 下载 rime-ice cn_dicts 到 ~/tmp/rimeice_cn/
#   for f in 8105 41448 base ext others; \
#     curl -LO https://raw.githubusercontent.com/iDvel/rime-ice/main/cn_dicts/$f.dict.yaml
python3 tools/open_build.py          # → open/hanhuo_open.bin

# 查询
target/release/hanhuo query nihao --bin open/hanhuo_open.bin
target/release/hanhuo query 94664486 --bin open/hanhuo_open.bin --t9        # T9
target/release/hanhuo query "" --ctx the --bin open/hanhuo_open.bin \
  --boost-tail --nnlm data/nnlm_open_mobile_v2.safetensors --topk 5          # 英文联想

# 模型注入（可选, 增强档）
target/release/hanhuo query jintianwanqtianqbucuo --bin open/hanhuo_open.bin \
  --nnlm data/nnlm_open_mobile_v2.safetensors --boost-tail --boost-sent-nn

# 回归
cargo test --release
WXQ_BIN=open/hanhuo_open.bin target/release/hanhuo golden golden/open_golden.jsonl
```

## 模型与数据（git-lfs）

`data/` 下全部模型自训、数据表开源构建：

| 件 | 训练/构建 | 语料/来源 | 许可 |
|---|---|---|---|
| nnlm_open_mobile_v2（fp32/fp16/int8） | 2 层 GPT d192 | LCCC | Apache-2.0（代码）|
| enlm_open_v2（三档） | 4 层 GPT d256 词级 | wikitext-103 | 同上 |
| mdltr_open.safetensors | LightGBM lambdarank | 引擎特征自采 | 同上 |
| bgrm_open.safetensors | bigram 380 万对（条件概率量化） | LCCC + jieba | 同上 |
| t9_lr_open.json | pairwise LR | 词表自采特征 | 同上 |
| c4g_open.tsv / en_open.tsv 等 | 统计/衍生 | wikitext-103 / 开源词频 | 见各文件头 |
| wubi86_open / rsd_strokes_open | 五笔 / 笔画 | rime-wubi / rime-stroke | GPL-3.0 / 开源 |
| 27 分类域包（HBWC） | 词库打包 | THUOCL/chinese-xinhua/chinese-poetry/rime-frost/rime-wanxiang/mw2fcitx 等 | 混源, 见 docs/词库目录.md |

语料原始下载见 [HuggingFace 数据集](#数据与语料) 一节。

## 本项目参考与致谢

- **[rime / rime-ice](https://github.com/iDvel/rime-ice)**（GPL-3.0）— 主词典数据源（8105/41448/base/ext/others）与生态惯例
- **[rime-wubi](https://github.com/rime/rime-wubi)** / **rime-stroke** — 五笔与笔画辅码数据
- **[LCCC](https://github.com/thu-coai/LCCC)** — 中文 NNLM / bigram 训练语料
- **[wikitext-103](https://blog.einstein.ai/the-wikitext-long-term-dependency-language-modeling-dataset/)** — 英文 LM / 词对统计语料
- **[THUOCL](http://thuocl.thunlp.org/)**（LGPL）/ chinese-xinhua / chinese-poetry（MIT）— 领域词库
- **[rime-frost](https://github.com/gaboolic/rime-frost)**（GPL-3.0）/ **[rime-wanxiang-data](https://github.com/amzxyz/rime-wanxiang)**（CC BY 4.0）— 域词源
- **[mw2fcitx 萌娘百科](https://github.com/outloudvi/mw2fcitx)**（CC BY-NC-SA）— 二次元词库
- **[pypinyin](https://github.com/mozillazg/python-pinyin)** — 音节/声调派生
- **[jieba](https://github.com/fxsjy/jieba)** — bigram 构建分词
- 引擎架构（四层管线 / DAT 读音索引 / 面板语义）为本项目从零设计实现；
  早期开发中曾以一套私有 Python 参考实现作行为对照（不随本仓分发），
  现全部行为由 `golden/open_golden.jsonl` 自洽回归锁冻结。

## 数据与语料

训练用原始语料与历史模型配方归档：**HuggingFace [dwdcth/hanhuo-ime-corpus](https://huggingface.co/datasets/dwdcth/hanhuo-ime-corpus)**（含 LCCC / wikitext-103 / THUCNews 采样、27 域包源词表、历史配方模型，及训练/词库工具脚本 `tools/`）
（含 LCCC / wikitext-103 采样、27 域包源词表、历史配方模型）——每个目录附
`SOURCES.md` 记录上游来源与许可。

## 许可

- 代码: Apache-2.0
- 数据/模型: 随上游语料许可（见上表与各数据文件头）; 域包分发需随附来源声明
- 本项目不包含、不分发任何 proprietary 输入法厂商的词库、模型或数据资产
