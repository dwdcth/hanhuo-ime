#!/usr/bin/env bash
# 双门脚本（看守周期抽查; 2026-11-20 固化——防 bin 律坑重蹈:
# golden 必须腾讯原版 wxq.bin, HANHUO_BIN/open bin 会 27/611 假红灯,
# 详见 memory/golden-gate-bin-law）
set -euo pipefail
cd "$(dirname "$0")/.."
TC=/home/banshan/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin
export PATH="$TC:$PATH"
echo "== golden 门（腾讯原版 bin）=="
WXQ_BIN=/home/banshan/tmp/wetype-win/wxq.bin target/release/hanhuo golden golden/golden.jsonl | tail -1
echo "== lib 门（4 crate）=="
cargo test --release -p hanhuo-core -p hanhuo-lm -p hanhuo-dict -p hanhuo-cli 2>&1 \
  | grep -E '^test result' | awk '{p+=$4;f+=$6} END {print "passed="p" failed="f}'
