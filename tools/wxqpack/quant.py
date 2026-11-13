#!/usr/bin/env python3
"""lmcn 原厂四段量化编解码（cascade/lmcn_val_formula.md 终破公式的忠实实现）.

val1 ↔ log10P:  N = −log10P × 1024，四段指数步长（1/2/4/8，边界 49152/50176/51200，
基座 47104）。值域 val1 ∈ [47104 (P=1), 51742 (log10P=−12.232)]。
26,103/26,103 单字条目逐点验证（1e-5 容差）——公式封闭。

wxq 打包格式用它存词频与 bigram 概率（用他们的紧凑编码存我们的数据）。
"""
from __future__ import annotations

BASE = 47104          # N = 0  → P = 1
B1, B2, B3 = 49152, 50176, 51200   # 段边界（等距 1024）
VMAX = 51742          # N = 12528 → log10P = −12.232
QUANT = 1024.0        # log10P 量化步长倒数

# 段表: (边界, 段内步长, 段基 N)
_SEGS = ((B1, 1, 0), (B2, 2, 2048), (B3, 4, 4096), (1 << 30, 8, 8192))


def val_to_n(v1: int) -> int:
    """val1 → N（= −log10P × 1024）"""
    if v1 < B1:
        return v1 - BASE
    if v1 < B2:
        return 2048 + 2 * (v1 - B1)
    if v1 < B3:
        return 4096 + 4 * (v1 - B2)
    return 8192 + 8 * (v1 - B3)


def n_to_val(n: int) -> int:
    """N → val1（正向分段量化）"""
    if n < 0:
        n = 0
    if n < 2048:
        return BASE + n
    if n < 4096:
        return B1 + (n - 2048) // 2
    if n < 8192:
        return B2 + (n - 4096) // 4
    if n <= 12528:
        return B3 + (n - 8192) // 8
    return VMAX


def val_to_log10p(v1: int) -> float:
    """val1 → log10P（lmcn_val_formula.md 的可复用函数）"""
    return -val_to_n(v1) / QUANT


def log10p_to_val(p: float) -> int:
    """log10P → val1（先量化到 1/1024 网格再正向映射；P=1 → 47104）"""
    n = int(round(-p * QUANT))
    return n_to_val(n)


# ---- val2 → alpha（部分结构：密集段 val2 = 46080 + Nα，量化 1/2048）----
def val2_to_alpha(v2: int) -> float:
    """val2 → log10α（密集段；低端亚整数步未定段，按密集段解释并注明）"""
    return -(v2 - 46080) / 2048.0


def freq_to_val(freq: float, total: float) -> int:
    """词频 → val1（P = freq/total 归一后走原厂量化）。

    跨环境确定性律（2026-11-13 CI golden 592/611 案定谳）: math.log10
    的 glibc 实现差末 ulp, 词频落在量化边界（-lg·1024 恰近半整数）时
    round 翻档 → CI 与本地 bin 字节差 → 面板分漂移。边界带（距半整数
    <1e-6）改 Decimal 定点重算（纯整数算法, 跨环境逐位确定）。
    """
    if freq <= 0 or total <= 0:
        return VMAX
    import math
    p = freq / total
    if p >= 1.0:
        return BASE
    lg = math.log10(p)
    if lg < -12.232:
        lg = -12.232
    x = -lg * QUANT
    frac = x - (x // 1.0)  # 小数部分
    edge = frac < 1e-6 or frac > 1.0 - 1e-6 or abs(frac - 0.5) < 1e-6
    if edge:
        from decimal import Decimal, getcontext, ROUND_HALF_EVEN
        getcontext().prec = 40
        lg_d = (Decimal(freq) / Decimal(int(total))).log10()
        lg_d = min(Decimal(0), lg_d)  # p>=1 → 0（同上分支语义）
        if lg_d < Decimal('-12.232'):
            lg_d = Decimal('-12.232')
        n = int((-lg_d * QUANT).to_integral_value(rounding=ROUND_HALF_EVEN))
        return n_to_val(n)
    return log10p_to_val(lg)


def prob_to_val(p: float) -> int:
    """概率 → val1"""
    if p >= 1.0:
        return BASE
    if p <= 10 ** -12.232:
        return VMAX
    return log10p_to_val(__import__('math').log10(p))


# 自检参考值（公式文档逐点验证样例）
_REF = {48842: -1.69727, 49191: -2.07617}   # 的 / 我


def _selfcheck() -> None:
    for v, expect in _REF.items():
        got = val_to_log10p(v)
        assert abs(got - expect) < 1e-4, (v, got, expect)
    # 网格封闭性: encode(decode(v)) == v 对全部合法码字成立
    for v in range(BASE, VMAX + 1):
        assert n_to_val(val_to_n(v)) == v, v
    # P=1 与 −10.0 钳位
    assert val_to_log10p(51456) == -10.0
    assert val_to_log10p(BASE) == 0.0


if __name__ == '__main__':
    _selfcheck()
    print('quant selfcheck OK: val↔N 网格封闭, 的/我 参考值命中, '
          f'值域 [{BASE},{VMAX}] → log10P [0, {val_to_log10p(VMAX):.3f}]')
