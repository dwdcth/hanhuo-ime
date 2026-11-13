#!/usr/bin/env python3
"""P2C 长句电池 A/B——基线 vs --p2c 生成召回（52+2 长句 ctx 电池）。

用法: P2C_STEM=data/p2c_open python3 tools/p2c_eval.py
"""
import json
import os
import subprocess
import sys

env = dict(os.environ)
env.setdefault('WXQ_BIN', '/home/banshan/tmp/wetype-win/wxq_dict.bin')
env.setdefault('HANHUO_BIN', env['WXQ_BIN'])
P2C = os.environ.get('P2C_STEM', 'data/p2c_open')
BIN = 'target/release/hanhuo'
NN = 'data/nnlm_open_mobile_v2.safetensors'

pairs = json.load(open('/mnt/s790/tmp/zhiyi-orig/ctx_sents.json'))
for mode, extra in [('base', []), ('p2c', ['--p2c', P2C])]:
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
