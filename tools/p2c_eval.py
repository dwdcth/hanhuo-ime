#!/usr/bin/env python3
"""P2C 长句电池 A/B——基线 vs --p2c 生成召回（52+2 长句 ctx 电池）。

用法: P2C_STEM=data/p2c_open python3 tools/p2c_eval.py [--skip-p2c]

口径注记（2026-11-14 清理役）:
- WXQ_BIN 透传（缺省仓内 open bin——开源自洽; 腾讯 wxq_dict 锚
  需显式传, 与 real_tests.sh 口径互补）。
- --skip-p2c: 只跑 base 模式（看守回归用, 省半程）。旧版不认识此
  旗标会静默跑双模。
- p2c 模式 top1=0 是已知态（P2C v1 注入「句面补位不抢头」保守律
  + 覆盖零, 见 docs/  P2C 战役终记）——非回归信号; gold 在池两模
  同值时 p2c 注入零增益。
- 电池 eval/ctx_sents_long52.json 入仓（原 /mnt/s790/tmp/zhiyi-orig
  易失依赖消除）。
"""
import json
import os
import subprocess
import sys

env = dict(os.environ)
REPO = os.path.normpath(os.path.join(os.path.dirname(__file__) or '.', '..'))
env.setdefault('WXQ_BIN', os.path.join(REPO, 'open', 'hanhuo_open.bin'))
env.setdefault('HANHUO_BIN', env['WXQ_BIN'])
P2C = os.environ.get('P2C_STEM', 'data/p2c_open')
BIN = 'target/release/hanhuo'
NN = 'data/nnlm_open_mobile_v2.safetensors'

SKIP_P2C = '--skip-p2c' in sys.argv
pairs = json.load(open(os.path.join(os.path.dirname(__file__) or '.',
                                    '..', 'eval', 'ctx_sents_long52.json')))
MODES = [('base', [])] if SKIP_P2C else [('base', []), ('p2c', ['--p2c', P2C])]
for mode, extra in MODES:
    top1 = inpool = n = 0
    for p in pairs:
        cmd = [BIN, 'query', p['py'], '--json',
               '--boost-sent-nn', '--boost-nn', '--nnlm', NN,
               '--topk', '30', '--ctx', p['ctx']] + extra
        r = subprocess.run(cmd, capture_output=True, text=True,
                           env=env, timeout=300)
        i = r.stdout.find('{')
        try:
            cs = [c['word'] for c in json.loads(r.stdout[i:])['cands']]
        except Exception:
            cs = []
        n += 1
        top1 += 1 if cs and cs[0] == p['text'] else 0
        inpool += 1 if p['text'] in cs else 0
    print(f"{mode}: n={n} top1={top1}({top1/n*100:.1f}%) "
          f"gold在池={inpool}({inpool/n*100:.1f}%)")
