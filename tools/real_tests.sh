#!/usr/bin/env bash
# real_tests.sh — 真机测试套（#[ignore] 全量, 2026-11-14 双 bin 清理役固化）
#
# 与 ci_local.sh 的分工: ci_local = 开源自洽链（open bin, golden 锚）;
# 本脚本 = 真机行为锁（腾讯 bin 三件: wxq_dict 主词典 / wxq 全量 bin
# 内嵌 NNLM / CTX 动态学习域）。资产属腾讯, 仅本地跑, 勿入 CI。
#
# 跑法域:
#   lib 单测 --ignored: 35 个（typo/fuzz/erwo/nnlm_bench/win_probe 等）
#     WXQ_BIN = wxq_dict.bin（腾讯词典锚; 2026-11-14 双 bin 化改造后
#     open bin 亦 35/35 全绿——脚本保持腾讯锚与 ci_local 的 open 锚分工）
#   tests/ctx_dyn_rank: CTX_BIN = wxq.bin（全量 bin, 内嵌 NNLM 必需）
#   tests/aux_code: 需 WUBI_YAML/RSD_TSV 私有码表（~/tmp/hanhuo 资产
#     随易失盘迁移丢失, 缺位时 14 测 expect panic——资产恢复前跳过）
set -e
cd "$(dirname "$0")/.."
WXQ_DICT="${WXQ_DICT:-$HOME/tmp/wetype-win/wxq_dict.bin}"
WXQ_FULL="${WXQ_FULL:-$HOME/tmp/wetype-win/wxq.bin}"
[ -f "$WXQ_DICT" ] || { echo "缺 $WXQ_DICT（腾讯词典）"; exit 1; }
[ -f "$WXQ_FULL" ] || { echo "缺 $WXQ_FULL（全量 bin, 内嵌 NNLM）"; exit 1; }

echo "=== [1/3] lib 真机单测（wxq_dict 锚） ==="
WXQ_BIN="$WXQ_DICT" cargo test --release -p hanhuo-core --lib -- --ignored 2>&1 \
  | grep -E "^test .* FAILED|test result" | tail -3

echo "=== [2/3] ctx 动态学习域（wxq 全量 bin） ==="
CTX_BIN="$WXQ_FULL" WXQ_BIN="$WXQ_DICT" \
  cargo test --release -p hanhuo-core --test ctx_dyn_rank -- --ignored 2>&1 \
  | grep "test result" | tail -1

echo "=== [3/3] aux_code 码表域（私有资产, 缺位跳过） ==="
if [ -f "${WUBI_YAML:-$HOME/tmp/hanhuo/wubi86_swbd.dict.yaml}" ]; then
  WXQ_BIN="$WXQ_DICT" cargo test --release -p hanhuo-core --test aux_code -- --ignored 2>&1 \
    | grep "test result" | tail -1
else
  echo "跳过: wubi/rsd 资产缺位（~/tmp/hanhuo 易失盘迁移丢失, 恢复需 wine 提取流程）"
fi
echo "=== 真机套跑完 ==="
