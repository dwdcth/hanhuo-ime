#!/usr/bin/env python3
"""L2 读音索引：Double-Array Trie（读音纯字母串键）.

- 渐进前缀查询：每状态存子树叶子区间 [lo, lo+cnt)（叶子 = 读音 id，按字典序 =
  DFS 序编号）→ 前缀一次走查即得候选读音区间，无需子树枚举。
- 终端位图：key==前缀 的精确命中（DP/精确匹配用）。
- 构建：排序键 + LCP 栈两遍法（先 trie CSR + lo/cnt/term，再自由链表放置）。

字符集: 'a'..'z' → code 1..26（读音键仅小写字母，含 lv/nv 的 v）。
"""
from __future__ import annotations

from array import array

ROOT = 1


def _code(ch: str) -> int:
    c = ord(ch) - 96
    if not (1 <= c <= 26):
        raise ValueError(f'DAT 键只允许 a-z: {ch!r}')
    return c


class DATBuilder:
    """从排序唯一键构建 DAT（base/check/lo/cnt/term 五数组）。"""

    def __init__(self):
        self.base = array('i', [0, 0])       # slot0 占位, ROOT=1
        self.check = array('i', [0, 0])
        self.lo = array('I', [0, 0])
        self.cnt = array('I', [0, 0])
        self._occ = bytearray(2)
        self._occ[1] = 1                     # ROOT 占用但从不入自由链
        self._fprev = array('i', [-1, -1])
        self._fnext = array('i', [-1, -1])
        self._free_head = -1
        self._free_tail = -1
        self.n_slots = 2
        self._term_bytes = bytearray(8)
        self.n_keys = 0

    # ---------------- 自由链表 ----------------
    def _grow(self, extra: int = 1 << 16):
        start = self.n_slots
        newn = start + extra
        for arr, init in ((self.base, 0), (self.check, 0),
                          (self.lo, 0), (self.cnt, 0)):
            arr.extend(array(arr.typecode, [init]) * extra)
        self._occ.extend(b'\x00' * extra)
        self._fprev.extend(array('i', [-1]) * extra)
        self._fnext.extend(array('i', [-1]) * extra)
        for s in range(start, newn):
            self._free_append(s)
        self.n_slots = newn

    def _free_append(self, s: int):
        self._fprev[s] = self._free_tail
        self._fnext[s] = -1
        if self._free_tail >= 0:
            self._fnext[self._free_tail] = s
        else:
            self._free_head = s
        self._free_tail = s

    def _free_detach(self, s: int):
        p, n = self._fprev[s], self._fnext[s]
        if p >= 0:
            self._fnext[p] = n
        else:
            self._free_head = n
        if n >= 0:
            self._fprev[n] = p
        else:
            self._free_tail = p
        self._fprev[s] = self._fnext[s] = -2   # 已摘除标记

    def _occupy(self, s: int):
        if self._occ[s]:
            raise RuntimeError(f'slot {s} double occupy')
        if self._fprev[s] != -2:               # 仍在自由链上 → 摘除
            self._free_detach(s)
        self._occ[s] = 1

    # ---------------- 构建 ----------------
    def build(self, sorted_keys: list[str]) -> None:
        """sorted_keys: 排序且唯一的键。叶子(读音)id = 键序号。"""
        n_keys = len(sorted_keys)
        if n_keys == 0:
            return
        # ---- pass 1: LCP 栈 → trie CSR + lo/cnt/term ----
        max_nodes = sum(len(k) for k in sorted_keys) + 1  # 上界
        UNSET = 0xFFFFFFFF
        t_lo = array('I', [UNSET]) * max_nodes   # 子树首叶 id（min 折叠，序无关）
        t_cnt = array('I', bytes(4 * max_nodes))
        t_term = bytearray(max_nodes)
        e_par = array('i')
        e_code = array('i')
        e_child = array('i')
        node_nchild: dict[int, int] = {}

        stack: list[int] = []
        stack_len: list[int] = []
        next_node = 1                          # node 0 = root
        prev_key = ''
        for rid, key in enumerate(sorted_keys):
            lcp = 0
            for a, b in zip(prev_key, key):
                if a != b:
                    break
                lcp += 1
            while stack_len and stack_len[-1] > lcp:
                child = stack.pop()
                stack_len.pop()
                parent = stack[-1] if stack else 0
                t_cnt[parent] += t_cnt[child]
                if t_lo[child] < t_lo[parent]:
                    t_lo[parent] = t_lo[child]
            depth = lcp
            par = stack[-1] if stack else 0
            for ch in key[lcp:]:
                depth += 1
                node = next_node
                next_node += 1
                e_par.append(par)
                e_code.append(_code(ch))
                e_child.append(node)
                node_nchild[par] = node_nchild.get(par, 0) + 1
                stack.append(node)
                stack_len.append(depth)
                par = node
            term_node = stack[-1] if stack else 0
            t_term[term_node] = 1
            t_cnt[term_node] += 1
            if rid < t_lo[term_node]:
                t_lo[term_node] = rid
            prev_key = key
        while stack:
            child = stack.pop()
            stack_len.pop()
            parent = stack[-1] if stack else 0
            t_cnt[parent] += t_cnt[child]
            if t_lo[child] < t_lo[parent]:
                t_lo[parent] = t_lo[child]
        n_nodes = next_node
        t_lo = t_lo[:n_nodes]
        t_cnt = t_cnt[:n_nodes]
        t_term = t_term[:n_nodes]

        # CSR 化边表（按父节点稳定分组 → 保持 code 序）
        child_off = array('i', bytes(4 * (n_nodes + 1)))
        for p in e_par:
            child_off[p + 1] += 1
        for i in range(1, n_nodes + 1):
            child_off[i] += child_off[i - 1]
        csr_child = array('i', bytes(4 * len(e_par)))
        csr_code = array('i', bytes(4 * len(e_par)))
        fill = array('i', child_off[:-1])
        for p, c, ch in zip(e_par, e_code, e_child):
            k = fill[p]
            csr_child[k] = ch
            csr_code[k] = c
            fill[p] = k + 1
        del e_par, e_code, e_child, fill

        # ---- pass 2: DAT 放置（node 创建序 = 父先于子）----
        self._grow(max(1 << 16, n_nodes * 2))
        state_of = array('i', bytes(4 * n_nodes))
        state_of[0] = ROOT
        self.lo[ROOT] = t_lo[0]
        self.cnt[ROOT] = t_cnt[0]
        if t_term[0]:
            self._set_term(ROOT)
        for u in range(0, n_nodes):      # 从 root 起（含 root 的孩子放置）
            s0, e0 = child_off[u], child_off[u + 1]
            if s0 == e0:
                continue                        # 叶节点无出边
            codes = csr_code[s0:e0]
            b = self._find_base(codes)
            su = state_of[u]
            self.base[su] = b
            for k in range(s0, e0):
                v = csr_child[k]
                slot = b + csr_code[k]
                if slot >= self.n_slots:
                    self._grow(slot - self.n_slots + 1)
                self._occupy(slot)
                self.check[slot] = su
                self.lo[slot] = t_lo[v]
                self.cnt[slot] = t_cnt[v]
                if t_term[v]:
                    self._set_term(slot)
                state_of[v] = slot
        self.n_keys = n_keys
        self._trim()

    # ---------------- term 位图 ----------------
    def _set_term(self, slot: int):
        while len(self._term_bytes) * 8 <= slot:
            self._term_bytes.extend(b'\x00' * 8192)
        self._term_bytes[slot >> 3] |= 1 << (slot & 7)

    def _find_base(self, codes) -> int:
        c0 = codes[0]
        x = self._free_head
        while x != -1:
            b = x - c0
            if b >= 1:
                ok = True
                for c in codes:
                    s = b + c
                    if s >= self.n_slots:
                        self._grow(s - self.n_slots + 1 + 4096)
                    if self._occ[s]:
                        ok = False
                        break
                if ok:
                    return b
            x = self._fnext[x]
        self._grow(max(4096, self.n_slots))
        return self._find_base(codes)

    def _trim(self):
        occ = self._occ
        last = self.n_slots - 1
        while last >= 0 and not occ[last]:
            last -= 1
        keep = last + 1
        del self.base[keep:]
        del self.check[keep:]
        del self.lo[keep:]
        del self.cnt[keep:]
        self.n_slots = keep
        need = (keep + 7) // 8
        del self._term_bytes[need:]

    def cnt_split(self):
        """cnt → (u16 数组, 溢出表 {slot: cnt})——cnt≥65535 走溢出"""
        c16 = array('H')
        ovf: dict[int, int] = {}
        cnt = self.cnt
        for s in range(self.n_slots):
            c = cnt[s]
            if c >= 0xFFFF:
                c16.append(0xFFFF)
                ovf[s] = c
            else:
                c16.append(c)
        return c16, ovf


class DAT:
    """运行期 DAT（mmap 零拷贝视图或内存数组）。

    base/check: i32; lo: u32; cnt: u16 + 溢出表（cnt≥65536 的浅状态，仅数十个）。
    """

    def __init__(self, base, check, lo, cnt16, term, n_keys, cnt_ovf=None):
        self.base, self.check = base, check
        self.lo, self.cnt16 = lo, cnt16
        self.cnt_ovf = cnt_ovf or {}
        self.term = term
        self.n_keys = n_keys
        self.n_slots = len(base)

    def cnt_of(self, state: int) -> int:
        c = self.cnt_ovf.get(state)
        return c if c is not None else int(self.cnt16[state])

    def walk(self, s: str, state: int = ROOT) -> int:
        """沿键字符串走查；不可达 → -1"""
        b, ck = self.base, self.check
        st = state
        for ch in s:
            c = ord(ch) - 96
            if not (1 <= c <= 26):
                return -1
            ns = b[st] + c
            if ns < 0 or ns >= self.n_slots or ck[ns] != st:
                return -1
            st = ns
        return st

    def is_terminal(self, state: int) -> bool:
        if state <= 0 or state >= self.n_slots:
            return False
        return bool(self.term[state >> 3] & (1 << (state & 7)))

    def range(self, state: int) -> tuple[int, int]:
        """子树叶子区间 [lo, lo+cnt)（读音 id 集合）"""
        return self.lo[state], self.cnt_of(state)

    def iter_terminal(self, s: str, state: int = ROOT):
        """沿 s 走查产出 (depth, state)：s[:depth] 为完整键的终端态（DP 用）"""
        b, ck = self.base, self.check
        st = state
        if self.is_terminal(st):
            yield 0, st
        for i, ch in enumerate(s, 1):
            c = ord(ch) - 96
            if not (1 <= c <= 26):
                return
            ns = b[st] + c
            if ns < 0 or ns >= self.n_slots or ck[ns] != st:
                return
            st = ns
            if self.is_terminal(st):
                yield i, st

    def iter_states_upto(self, depth: int):
        """BFS 枚举 ≤depth 的全部状态（HOT 表构建用）"""
        yield ROOT
        frontier = [ROOT]
        for _ in range(depth):
            nxt = []
            for st in frontier:
                b = self.base[st]
                for c in range(1, 27):
                    ns = b + c
                    if 0 <= ns < self.n_slots and self.check[ns] == st:
                        nxt.append(ns)
                        yield ns
            frontier = nxt
            if not frontier:
                return
