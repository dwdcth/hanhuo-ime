#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""mdltr_train.py — c.6 P5b: 开源训练 mdltr（LightGBM lambdarank, 用户指令）

数据: LTR_DUMP 收割件（inp \t word \t 179 floats/候选）× eval 标签
训练: LGBMRanker num_leaves=63（引擎定长 62 内点+63 叶）× N 树
导出: hanhuo-mdltr v1 safetensors——DFS 定槽转换（内点 0..61,
  叶 ~id 负引）; 不足 62 内点的树补哑（thr=+inf 恒左, 永不触达）。

用法: python3 tools/mdltr_train.py [--trees 131]
验收: hanhuo eval ... --ltr --mdltr <out> vs --ltr 空档 vs 无
"""
import argparse
import json
import os
import struct

N_NODE, N_LEAF, N_FEAT = 62, 63, 179


def convert_tree(nested):
    """lightgbm 嵌套树 → 定长 (feat, dt, left, right, thr, leaf)"""
    feat = [0] * N_NODE
    dt = [0] * N_NODE
    left = [0] * N_NODE
    right = [0] * N_NODE
    thr = [0.0] * N_NODE
    leaf = []
    st = {"ni": 0}

    def walk(nd):
        if "split_index" in nd and st["ni"] < N_NODE:
            i = st["ni"]
            st["ni"] += 1
            feat[i] = min(nd["split_feature"], N_FEAT - 1)
            thr[i] = float(nd["threshold"])
            dt[i] = 0
            left[i] = walk(nd["left_child"])
            right[i] = walk(nd["right_child"])
            return i
        leaf.append(float(nd["leaf_value"]))
        return ~(len(leaf) - 1)

    walk(nested)
    while st["ni"] < N_NODE:  # 哑内点（真实路径不触达）
        i = st["ni"]
        st["ni"] += 1
        thr[i] = float("inf")
        left[i] = right[i] = ~0
    while len(leaf) < N_LEAF:
        leaf.append(0.0)
    return feat, dt, left, right, thr, leaf


def write_container(path, tensors, meta):
    blob = bytearray()
    hdr = {"__metadata__": meta}
    for name, shape, data, dtyp in sorted(tensors):
        s = len(blob)
        blob.extend(data)
        hdr[name] = {"dtype": dtyp, "shape": shape,
                     "data_offsets": [s, len(blob)]}
    hb = json.dumps(hdr, ensure_ascii=False,
                    separators=(",", ":")).encode()
    with open(path, "wb") as f:
        f.write(struct.pack("<Q", len(hb)))
        f.write(hb)
        f.write(blob)


def main():
    import lightgbm as lgb
    import numpy as np

    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", default="/mnt/s790/tmp/ltr_train.tsv")
    ap.add_argument("--eval", default="eval/eval_v2.jsonl")
    ap.add_argument("--trees", type=int, default=131)
    ap.add_argument("--out", default="data/mdltr_open.safetensors")
    a = ap.parse_args()

    tgt = {}
    for ln in open(a.eval, encoding="utf-8"):
        r = json.loads(ln)
        tgt[r["py"]] = r["text"]

    X, y, qs = [], [], []
    rows = []
    cur_q = None

    def flush():
        target = tgt.get(cur_q)
        if target:
            labels = [1 if w == target else 0 for _, w, _ in rows]
            if 1 in labels:
                for (i, w, f), lb in zip(rows, labels):
                    X.append(f)
                    y.append(lb)
                    qs.append(i)

    for ln in open(a.dump, encoding="utf-8"):
        inp, word, feats = ln.rstrip("\n").split("\t", 2)
        if inp != cur_q:
            flush()
            rows = []
            cur_q = inp
        rows.append((inp, word, [float(v) for v in feats.split(",")]))
    flush()
    print(f"样本 {len(X):,} / 正例 {sum(y):,} / 查询组 {len(set(qs)):,}")

    from collections import OrderedDict
    group_sizes = OrderedDict()
    for i in qs:
        group_sizes[i] = group_sizes.get(i, 0) + 1

    model = lgb.LGBMRanker(
        objective="lambdarank", num_leaves=63, n_estimators=a.trees,
        learning_rate=0.08, min_child_samples=50, label_gain=[0, 1],
        verbose=-1)
    model.fit(np.array(X, dtype=np.float32), np.array(y),
              group=np.array(list(group_sizes.values())))
    print(f"训练完（{a.trees} 树）")

    trees = model.booster_.dump_model()["tree_info"]
    F, DT, L, R, TH, LV = [], [], [], [], [], []
    for t in trees:
        fe, d, le, ri, th, lf = convert_tree(t["tree_structure"])
        F.append(fe); DT.append(d); L.append(le)
        R.append(ri); TH.append(th); LV.append(lf)

    def pack(rows, fmt):
        return b"".join(struct.pack(f"<{len(r)}{fmt}", *r) for r in rows)

    n = len(trees)
    write_container(a.out, [
        ("feat", [n, N_NODE], pack(F, "B"), "I8"),  # u8 存储（特征号≤178 超 i8; Rust 侧 Vec<u8>）
        ("dt", [n, N_NODE], pack(DT, "B"), "I8"),
        ("left", [n, N_NODE], pack(L, "i"), "I32"),
        ("right", [n, N_NODE], pack(R, "i"), "I32"),
        ("thr", [n, N_NODE], pack(TH, "d"), "F64"),
        ("leaf", [n, N_LEAF], pack(LV, "d"), "F64"),
        ("cat_boundaries", [0], b"", "U32"),
        ("cat_boundaries.offsets", [n + 1],
         struct.pack(f"<{n+1}q", *([0] * (n + 1))), "I64"),
        ("cat_bitmaps", [0], b"", "U32"),
        ("cat_bitmaps.offsets", [n + 1],
         struct.pack(f"<{n+1}q", *([0] * (n + 1))), "I64"),
    ], {"format": "hanhuo-mdltr", "version": "1",
        "license_note": "开源自训（c.6 P5b）: LTR_DUMP × LightGBM "
                        "lambdarank, Apache-2.0"})
    print(f"✓ {a.out} ({os.path.getsize(a.out)/1e6:.1f} MB)")


if __name__ == "__main__":
    main()
