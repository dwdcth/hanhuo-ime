#!/usr/bin/env bash
# ci_local.sh — CI 本地跑（2026-11-14 用户指令: ci 本地跑——GitHub
# Actions 免费额度有限, 验证链全量本地化; ci.yml 改 workflow_dispatch
# 仅手动）。口径 = .github/workflows/ci.yml 原步骤。
set -e
cd "$(dirname "$0")/.."
echo "=== [1/4] Build ==="
cargo build --release -p hanhuo-cli
echo "=== [2/4] Test ==="
cargo test --release -p hanhuo-core -p hanhuo-lm -p hanhuo-dict -p hanhuo-cli 2>&1 | grep -E "test result" | tee /tmp/ci_local_tests.txt
awk -F'[ ;]' '{p+=$4; f+=$6} END {print "汇总: passed="p" failed="f; exit (f>0)}' /tmp/ci_local_tests.txt
echo "=== [3/4] 词典构建（仓内钉版源） ==="
python3 tools/open_build.py 2>&1 | tail -2
md5sum open/hanhuo_open.bin
echo "=== [4/4] Golden（自洽回归锁） ==="
WXQ_BIN="$PWD/open/hanhuo_open.bin" target/release/hanhuo golden golden/open_golden.jsonl
echo "=== 本地 CI 全绿 ==="
