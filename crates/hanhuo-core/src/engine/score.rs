//! 打分层: 候选构造 mk / bigram / 编辑罚 / ltr 重排 / NNLM 重排.
use super::*;

impl Engine {
    pub(super) fn bigram_after(&self, wid: u32) -> Big {
        // searchsorted left/right over bi_w1
        let n = self.bi_n;
        let lower = |v: u32| -> usize {
            let (mut a, mut b) = (0usize, n);
            while a < b {
                let mid = (a + b) / 2;
                if self.bi_w1.get(mid) < v {
                    a = mid + 1;
                } else {
                    b = mid;
                }
            }
            a
        };
        let i0 = lower(wid);
        let i1 = lower(wid + 1); // 值域 u32 → right(wid) == lower(wid+1)
        let mut ord = Vec::new();
        let mut map = HashMap::new();
        if i0 < i1 {
            for i in i0..i1 {
                let w = self.bi_w2.get(i);
                let lp = val_to_log10p(self.bi_val.get(i));
                ord.push((w, lp));
                map.insert(w, lp); // Python dict 覆写值、保首现位（序在 ord）
            }
        }
        Big { ord, map }
    }

    /// 编辑距离（原厂罚表, Damerau）
    pub(super) fn edit_cost(a: &[u8], b: &[u8]) -> f64 {
        let (la, lb) = (a.len(), b.len());
        let mut d = vec![vec![0.0f64; lb + 1]; la + 1];
        for i in 1..=la {
            d[i][0] = d[i - 1][0] + EDIT_DEL;
        }
        for j in 1..=lb {
            d[0][j] = d[0][j - 1] + EDIT_INS;
        }
        for i in 1..=la {
            for j in 1..=lb {
                let cost = if a[i - 1] == b[j - 1] { 0.0 } else { EDIT_SUB };
                let mut v = d[i - 1][j] + EDIT_DEL;
                v = v.min(d[i][j - 1] + EDIT_INS);
                v = v.min(d[i - 1][j - 1] + cost);
                if i > 1 && j > 1 && a[i - 1] == b[j - 2]
                    && a[i - 2] == b[j - 1]
                {
                    v = v.min(d[i - 2][j - 2] + EDIT_TRANS);
                }
                d[i][j] = v;
            }
        }
        d[la][lb]
    }

    /// MixRank::GetRankFeat 复刻（mdltr_feat/FEATURES.md 第二役）：
    /// 活体实证核心组（f0/f1 键数、f2/f3/f12 派生、f8、f171 主分、
    /// f173 名次、f174 Δbest、f175..178 哨兵对）；余 native-only 0 填充。
    /// 融合 adj = LTR_COEF × raw。
    /// 输入悬空尾（最大吞噬切分残留）：(键数, 起位, 声母串)
    fn ltr_dangling(&self, inp: &str) -> (usize, usize, String) {
        let n = inp.len();
        let mut i = 0usize;
        while i < n {
            let mut hit = 0usize;
            for l in (1..=self.seg.max_len.min(n - i).min(6)).rev() {
                if self.seg.is_syl(&inp[i..i + l]) {
                    hit = l;
                    break;
                }
            }
            if hit == 0 {
                break;
            }
            i += hit;
        }
        let tail = &inp[i..];
        if tail.is_empty() {
            return (0, 0, String::new());
        }
        let ini = if tail.starts_with("zh") || tail.starts_with("ch")
            || tail.starts_with("sh")
        {
            tail[..2].to_string()
        } else {
            tail[..1].to_string()
        };
        (tail.len(), i, ini)
    }

    /// MixRank::GetRankFeat 复刻 v2（第四役全规格，见 mdltr_feat/FEATURES.md）。
    /// 槽语义=活体实证+ctx 映射表；native-only 槽 0 填充。
    /// 候选在途音节（读音音节横跨输入边界者）的 (声母, 起键位)
    fn ltr_boundary(&self, c: &Cand, n_keys: usize) -> (String, usize) {
        if c.kind == Kind::Sentence || c.reading.is_empty() {
            return (String::new(), 0);
        }
        let mut pos = 0usize;
        for s_ in c.reading.split('\'') {
            if pos + s_.len() > n_keys {
                let ini = if s_.starts_with("zh")
                    || s_.starts_with("ch")
                    || s_.starts_with("sh")
                {
                    s_[..2].to_string()
                } else {
                    s_[..1].to_string()
                };
                return (ini, pos);
            }
            pos += s_.len();
        }
        (String::new(), 0)
    }

    pub(super) fn ltr_rerank(
        &self,
        cands: &mut [Cand],
        n_keys: usize,
        inp: &str,
        has_ctx: bool,
    ) {
        const C_X: &[(&str, usize)] = &[
            ("w", 69), ("z", 72), ("d", 73), ("n", 74), ("t", 76),
            ("y", 77), ("g", 78), ("j", 79), ("x", 80), ("b", 81),
            ("h", 82), ("q", 85), ("c", 86), ("k", 89), ("f", 90),
            ("r", 91), ("p", 101), ("a", 95), ("e", 114), ("o", 122),
            ("l", 66), ("s", 67), ("m", 68), ("zh", 138), ("sh", 140),
        ];
        const P_C: &[(usize, usize)] =
            &[(0, 60), (1, 62), (2, 64), (3, 63), (4, 70), (6, 105)];
        const R_C_0: usize = 59;

        let mut raws: Vec<f64> = Vec::with_capacity(cands.len());
        let (dang_n, dang_pos, dang_ini) = self.ltr_dangling(inp);
        let best = cands
            .iter()
            .map(|c| c.score)
            .fold(f64::NEG_INFINITY, f64::max);
        let best_lm = cands
            .iter()
            .filter_map(|c| c.lm)
            .map(|v| v.abs())
            .fold(0f64, f64::max);
        // 主分名次（score 降序，0 基）
        let mut order: Vec<usize> = (0..cands.len()).collect();
        order.sort_by(|&a, &b| {
            cands[b]
                .score
                .partial_cmp(&cands[a].score)
                .unwrap()
        });
        let mut rank_of = vec![0usize; cands.len()];
        for (rk, &i) in order.iter().enumerate() {
            rank_of[i] = rk;
        }
        let lm_vals: Vec<f64> =
            cands.iter().map(|c| c.lm.unwrap_or(0.0)).collect();
        let mut lm_order: Vec<usize> = (0..lm_vals.len()).collect();
        lm_order.sort_by(|&a, &b| {
            lm_vals[b].partial_cmp(&lm_vals[a]).unwrap()
        });
        let mut lm_rank = vec![0usize; lm_vals.len()];
        for (rk, &i) in lm_order.iter().enumerate() {
            lm_rank[i] = rk;
        }
        let lm_best = lm_vals
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        for i in 0..cands.len() {
            let consumed = if cands[i].kind == Kind::Sentence {
                n_keys
            } else {
                cands[i].reading.replace('\'', "").len()
            };
            let lm = cands[i].lm.unwrap_or(0.0);
            let mut f = [0f32; 179];
            f[0] = consumed as f32;
            f[1] = n_keys as f32;
            if n_keys != 0 {
                f[2] = (consumed as f64 / n_keys as f64) as f32;
                f[3] = (consumed as f64 - n_keys as f64) as f32;
            }
            f[8] = 1.0;
            f[17] = dang_n as f32;
            f[10] = (dang_n + consumed) as f32;
            f[11] = (dang_n + n_keys) as f32;
            if f[11] != 0.0 {
                f[12] = f[10] / f[11];
            }
            f[5] = lm as f32;
            f[4] = lm as f32;
            if n_keys != 0 {
                f[6] = f[2] * f[4];
                f[7] = f[2] * f[5];
            }
            // sum1-5_lm = 组内五行各自的 lm（活体实证分布语义）
            for (k, &slot) in [23usize, 24, 27, 28, 33].iter().enumerate() {
                f[slot] = if k < lm_vals.len() {
                    lm_vals[k] as f32
                } else {
                    0.0
                };
            }
            // 微槽族（第七役: 音节阶梯实验解码, 单音节门控）
            let n_syl = self
                .segment(inp)
                .map(|(p, _)| p.len())
                .unwrap_or(0);
            if n_syl >= 2 {
                f[163] = (n_syl - 1) as f32; // name28 前置音节
                f[164] = n_syl as f32;       // name3 音节数
                f[165] = 1.0;                // name27 多音节旗标
                f[168] = 1.0;                // name6 备族旗标
                let b = inp.as_bytes();
                if b.len() > 1 && b"aeiou".contains(&b[1]) {
                    f[102] = 1.0;            // i
                    f[126] = 1.0;            // P_i_1
                    f[133] = 1.0;            // i__h 第二键元音
                }
            }
            f[171] = cands[i].score as f32;
            f[173] = rank_of[i] as f32;
            f[174] = if best != f64::NEG_INFINITY {
                (cands[i].score - best) as f32
            } else {
                -100.0
            };
            // typo 族（纠错命中）+ 35/36 类别（单字 2/2 多字 13/4 粗类）
            if cands[i].kind == Kind::Correct {
                f[39] = 1.0;              // exact_with_typo
                f[47] = cands[i].logp as f32; // exact_after_typo
            }
            let multi = cands[i].word.chars().count() > 1;
            f[169] = if multi { 13.0 } else { 2.0 };
            f[170] = if multi { 4.0 } else { 2.0 };
            f[166] = 0.0;
            f[175] = 100.0;
            f[176] = -100.0;
            if best_lm >= 1e-6 {
                f[177] = lm_rank[i] as f32;
                f[178] = (lm - lm_best) as f32;
            } else {
                f[177] = 100.0;
                f[178] = -100.0;
            }
            f[172] = if has_ctx { 1.0 } else { 0.0 };
            // 在途旗标 v3: 候选边界音节优先（我@z→c_w_, nih→c_h_）,
            // 回退输入悬空尾（zhongg→c_g_）
            let (bini, bpos) =
                self.ltr_boundary(&cands[i], n_keys);
            let (ini, pos) = if !bini.is_empty() {
                (bini, bpos)
            } else {
                (dang_ini.clone(), dang_pos)
            };
            if !ini.is_empty() {
                f[58] = 1.0;
                f[R_C_0] = 1.0;
                if let Some(&(_, slot)) =
                    P_C.iter().find(|&&(p, _)| p == pos)
                {
                    f[slot] = 1.0;
                }
                if let Some(&(_, slot)) =
                    C_X.iter().find(|&&(ref x, _)| *x == ini)
                {
                    f[slot] = 1.0;
                }
            }
            // c.6 P5b: 特征导出（LTR_DUMP=<path>——开源 mdltr 训练数据
            // 收割口; 每候选一行: inp \t word \t f0..f178）
            if let Ok(path) = std::env::var("LTR_DUMP") {
                use std::io::Write;
                let mut f_ = std::fs::OpenOptions::new()
                    .create(true).append(true)
                    .open(&path).unwrap();
                let _ = writeln!(
                    f_, "{}\t{}\t{}",
                    inp,
                    cands[i].word,
                    f.iter().map(|v| format!("{v:.4}"))
                        .collect::<Vec<_>>().join(","));
            }
            if let Some(raw) = self.mdltr_score(&f) {
                raws.push(raw);
            } else {
                raws.push(0.0);
            }
        }
        // 列分离（第七役: 原生三 searcher 向量; hanhuo 按 Kind 分列各自 rank/top-4）
        let col_a: Vec<usize> = cands
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                matches!(c.kind, Kind::Sentence | Kind::Path)
            })
            .map(|(i, _)| i)
            .collect();
        let set_a: std::collections::HashSet<usize> =
            col_a.iter().copied().collect();
        let col_b: Vec<usize> = (0..cands.len())
            .filter(|i| !set_a.contains(i))
            .collect();
        for col in [&col_a, &col_b] {
            for (rk, &i) in col.iter().enumerate() {
                cands[i].ltr = Some(raws[i]);
                if rk < LTR_TOP_N {
                    cands[i].score = raws[i]
                        + (LTR_RANK_PRIOR
                            - LTR_RANK_DECAY * rk as f64);
                }
            }
        }
    }

    pub(super) fn mk(
        &self,
        wid: u32,
        rid: i64,
        kind: Kind,
        extra_syl: i32,
        lm: Option<f64>,
        edit_penalty: f64,
        fv: Option<u16>,
    ) -> Cand {
        let rid = if rid < 0 {
            self.w2r.get(wid as usize) as i64 - 1
        } else {
            rid
        };
        let reading = if rid >= 0 {
            self.rid_pretty(rid as u32)
        } else {
            String::new()
        };
        let logp = match fv {
            Some(v) => val_to_log10p(v),
            None => self.word_logp(wid),
        };
        Cand {
            word: self.word_text(wid),
            reading,
            logp,
            score: 0.0,
            kind,
            lm,
            nn_n: None,
            wid: wid as i64,
            extra_syl,
            edit_penalty,
            score_bias: 0.0,
            user_bias: 0.0,
            lp_locked: fv.is_some(),
            ltr: None,
            path_words: None,
            fz_face: false,
            mhead: None,
            dgl_pin: false,
        }
    }

    // ------------------------------------------------------------------
    // M4: NNLM 重排（nlrgptand 批量打分 + 有界融合）
    // ------------------------------------------------------------------
    /// 役K(agentZ): py `_nnlm_ok` 镜像——NNLM 可解析为真（含惰性装载;
    /// v2: 高级版注入亦为真——nnlm_* 统一入口的分发序同源）
    pub(super) fn nnlm_ok(&self) -> bool {
        #[cfg(feature = "safetensors")]
        {
            if self.nnlm_adv.borrow().is_some() {
                return true;
            }
        }
        if self.nnlm.borrow().is_none() {
            let parsed = self
                .pack
                .raw(b"NNLM")
                .ok()
                .and_then(|s| Nnlm::parse(s).ok());
            *self.nnlm.borrow_mut() = parsed.map(Box::new);
        }
        self.nnlm.borrow().is_some()
    }

    /// NNLM 原始区（开放格式导出用——CLI 侧重解析后走 safetensors 序列化）
    pub fn nnlm_section(&self) -> Option<&[u8]> {
        self.pack.raw(b"NNLM").ok()
    }

    /// 役N tick53: NNLM 模型句柄（惰性装载; py `self.nnlm` 属性镜像）
    pub(super) fn nnlm_model(
        &self,
    ) -> Option<std::cell::Ref<'_, Box<Nnlm>>> {
        if self.nnlm.borrow().is_none() {
            let parsed = self
                .pack
                .raw(b"NNLM")
                .ok()
                .and_then(|s| Nnlm::parse(s).ok());
            *self.nnlm.borrow_mut() = parsed.map(Box::new);
        }
        let g = self.nnlm.borrow();
        if g.is_some() {
            Some(std::cell::Ref::map(g, |x| x.as_ref().unwrap()))
        } else {
            None
        }
    }

    pub(super) fn nnlm_rerank(&self, cands: &mut [Cand], ctx_ids: Option<&CtxIds>) -> bool {
        // 统一入口前置探测（高级版优先/轻量惰性——nnlm_score 内部同源;
        // 此处先探测避免空跑编码）
        if !self.nnlm_ok() {
            return false;   // 无 NNLM：py 侧整块跳过保序（open bin 语义）
        }
        let ctx_text = ctx_ids.map(|c| c.text.as_str()).unwrap_or("");
        let ctx_seq: Option<Vec<u32>> = if ctx_text.is_empty() {
            None
        } else {
            self.nnlm_encode(ctx_text)
        };
        let ctx_seq = match ctx_seq {
            Some(s) => Some(s),
            None if !ctx_text.is_empty() => Some(
                ctx_text
                    .chars()
                    .filter_map(|c| self.nnlm_vocab_id(&c.to_string()))
                    .collect(),
            ),
            None => None,
        };
        let mut seqs: Vec<Vec<u32>> = Vec::new();
        let mut keep_idx: Vec<usize> = Vec::new();
        for (i, c) in cands.iter().enumerate() {
            if let Some(s) = self.nnlm_encode(&c.word) {
                seqs.push(s);
                keep_idx.push(i);
            }
        }
        if seqs.is_empty() {
            return false;
        }
        let Some(scores) =
            self.nnlm_score(ctx_seq.as_deref().unwrap_or(&[]), &seqs)
        else {
            return false;
        };
        let best = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        for (&i, &s) in keep_idx.iter().zip(scores.iter()) {
            let c = &mut cands[i];
            c.nn_n = Some(s);
            let dn = best - s; // ΔN = N_best − N_w ≥ 0
            let mut adj = NN_COEF * LAMBDA * dn;
            if adj < -NN_CLAMP {
                adj = -NN_CLAMP;
            } else if adj > NN_CLAMP {
                adj = NN_CLAMP;
            }
            c.score += adj;
        }
        true
    }

    // ------------------------------------------------------------------
    // 语境×用户信号（用户想法 2026-11-09; docs/设计-上下文动态组词.md §a）
    // ------------------------------------------------------------------
    /// 终位用户候选的语境微调: NNLM(ctx, 词面) 分差并入终分.
    ///
    /// 缺口: B4 user_boost / B5 短语注入只看 anchor（个人频率单维）;
    /// M4 nnlm_rerank 的 ctx 融合发生在 user_boost_final **之前**
    /// （query.rs NNLM 神经层重排块）——注入的短语/前缀补全候选从未
    /// 见过 ctx。本函数在终位（与 user_boost_final 同层——B4 教训:
    /// 信号必须落终面板实例, 中段信号会被面板列重建与 nn/ltr 重排
    /// 洗掉）为携带用户信号的候选补上 ctx 维。
    ///
    /// 公式（L4 融合律同源, 域 = 用户信号子集）:
    ///   adj = clamp(κ_u · (N_w − N_ref), ±NN_CLAMP)
    ///   N_w = NNLM ΣlnP(w | ctx)（原始和, 同 score_candidates）
    ///   N_ref = 终位 top1 词面的 N（top1 自身是用户候选时 adj≡0,
    ///           退化为组内纯重排——挑战者须在 NNLM 分上胜过在位者）
    ///
    /// κ_u 标定锚: 一次复用的 anchor 权重 ≈ 0.6~0.77
    /// （1.9·ln(a+1) − 1.9·ln(a), a∈[1,4]）; 令 3 nat 语境优势 ≈ 一次
    /// 复用 → κ_u ≈ 0.25。与 M4 的 0.23·|λ|=0.092 不同域: M4 是同拼
    /// 同长 homophone 微调（ΔN ~1-3 nat）, 此处跨词面竞争（同长类内,
    /// ΔN 常 3+ nat）, 系数取强一档。帽 = NN_CLAMP 1.5: 语境是修饰
    /// 信号, 不覆盖 anchor 帽 6.0 的个人偏好主序——层级 anchor > ctx
    /// > 词频（6.0 > 1.5, 基座差 ~3.0 的成交结构见 B4 标定注）。
    ///
    /// 同长参照律（v2, PoC W 组实证反推）: NNLM 原子和跨长度不可比
    /// ——短词天然少累加每字 −1~−2 nat（W 组实测: 2 字学习词对 4 字
    /// n-gram 参照恒定吃满 +帽, ctx 方向性被长度差完全淹没; Q 组
    /// 反向: 6 字前缀补全对 2 字参照恒吃 −帽）。参照 = 同字长类内
    /// 打分最高的**非用户信号**候选; 类内无原生 → 类内用户候选最大
    /// N（组内重排语义, P 组同键短语即此路）; 单候选类 → adj 0
    /// （无公平参照不发力——保守律）。跨长度提权（前缀补全越位
    /// 原生 top1）因此不在本层域内, 留作 α 长度惩罚归一的后续旋钮。
    ///
    /// 门（零伤害纪律）: 无 ctx / 无 NNLM / 无用户信号 → 恒等 false;
    /// golden 忠实路径（无用户库、无 ctx）双保险零行为。不可编码候选
    /// 不动（保守保序; B3 "不可编码 → 整体放弃" 律的用户子集弱化——
    /// 用户候选量小, 逐条放行不引噪）。
    pub fn user_ctx_adjust(
        &self,
        ctx: Option<&str>,
        out: &mut [Cand],
    ) -> bool {
        let dbg = std::env::var("CTX_DBG").is_ok();
        // 门 1: 无 ctx（py 空 ctx falsy 语义）
        let Some(ctx) = ctx.filter(|c| !c.is_empty()) else {
            return false;
        };
        // 门 2: NNLM 缺席 → 保序（open bin 语义, 同 nnlm_rerank;
        // 统一入口——高级版注入即换脑）
        if !self.nnlm_ok() {
            return false;
        }
        // 门 3: 无用户信号 → 恒等（B4 词级加成 / B5 注入 / 前缀补全
        // 都在 user_bias 或 Kind::User 上留痕）
        let is_target =
            |c: &Cand| c.user_bias != 0.0 || c.kind == Kind::User;
        let targets: Vec<usize> = out
            .iter()
            .enumerate()
            .filter(|(_, c)| is_target(c))
            .map(|(i, _)| i)
            .collect();
        if targets.is_empty() {
            return false;
        }
        // ctx 编码（nnlm_rerank 同律: encode 失败 → 逐字词表过滤回退）
        let ctx_seq: Vec<u32> = match self.nnlm_encode(ctx) {
            Some(s) => s,
            None => ctx
                .chars()
                .filter_map(|c| self.nnlm_vocab_id(&c.to_string()))
                .collect(),
        };
        if ctx_seq.is_empty() {
            return false;
        }
        // 打分集: 用户信号候选（面板序前 K, 注入块天然 anchor 序——
        // 同 B3 SCORING_CAP 量级控 NNLM 前向）+ 各字长类的原生参照
        const SCORE_CAP: usize = 8;
        // 语境×用户信号系数（标定见函数文档: 3 nat ≈ 一次复用 0.6~0.77）
        const KAPPA_U: f64 = 0.25;
        let n_t = targets.len().min(SCORE_CAP);
        let targets = &targets[..n_t];
        let len_of =
            |c: &Cand| c.word.chars().filter(|&ch| ch != '\u{1}').count();
        let lens: Vec<usize> =
            targets.iter().map(|&i| len_of(&out[i])).collect();
        // v2 同长参照: 每个出现过的字长类取面板序首个非用户信号候选
        // （面板按分降序 → 即该类最强原生）
        let mut ref_of_len: Vec<(usize, usize)> = Vec::new(); // (len, idx)
        for (i, c) in out.iter().enumerate() {
            let l = len_of(c);
            if !is_target(c)
                && lens.contains(&l)
                && !ref_of_len.iter().any(|&(rl, _)| rl == l)
            {
                ref_of_len.push((l, i));
            }
        }
        // 打分项: (词面, 归属) —— Target(k) / RefLen(l)
        enum Slot {
            Target(usize),
            Ref(usize),
        }
        let mut entries: Vec<(Slot, Vec<u32>)> = Vec::new();
        for (k, &ti) in targets.iter().enumerate() {
            if let Some(s) = self.nnlm_encode(&out[ti].word) {
                if !s.is_empty() {
                    entries.push((Slot::Target(k), s));
                }
            }
        }
        for &(l, i) in &ref_of_len {
            if let Some(s) = self.nnlm_encode(&out[i].word) {
                if !s.is_empty() {
                    entries.push((Slot::Ref(l), s));
                }
            }
        }
        if entries.is_empty() {
            return false;
        }
        let seqs: Vec<Vec<u32>> =
            entries.iter().map(|(_, s)| s.clone()).collect();
        let Some(scores) = self.nnlm_score(&ctx_seq, &seqs) else {
            return false;
        };
        // 各字长类参照分: 原生参照可编码 → 其分; 否则该类用户候选
        // 最大 N（组内重排）; 该类无任何可编码项 → 类内全体不动。
        let score_of = |k: usize| -> Option<f64> {
            entries
                .iter()
                .position(|(s, _)| matches!(s, Slot::Target(t) if *t == k))
                .map(|p| scores[p])
        };
        let ref_score_of_len = |l: usize| -> Option<f64> {
            if let Some(p) = entries.iter().position(|(s, _)| {
                matches!(s, Slot::Ref(rl) if *rl == l)
            }) {
                return Some(scores[p]);
            }
            // 回退: 类内用户候选最大 N
            let mut best = f64::NEG_INFINITY;
            let mut any = false;
            for (k, &li) in lens.iter().enumerate() {
                if li == l {
                    if let Some(s) = score_of(k) {
                        best = best.max(s);
                        any = true;
                    }
                }
            }
            any.then_some(best)
        };
        let mut touched = false;
        for (k, &ti) in targets.iter().enumerate() {
            let Some(s) = score_of(k) else { continue };
            let Some(r) = ref_score_of_len(lens[k]) else { continue };
            let adj = (KAPPA_U * (s - r)).clamp(-NN_CLAMP, NN_CLAMP);
            if dbg {
                eprintln!(
                    "[ctxdbg] {}（len={}） N={:.3} ref={:.3} adj={:+.3}\
                     （ctx={}）",
                    out[ti].word.replace('\u{1}', "·"),
                    lens[k],
                    s,
                    r,
                    adj,
                    ctx
                );
            }
            if adj != 0.0 {
                let c = &mut out[ti];
                c.score += adj;
                if c.nn_n.is_none() {
                    c.nn_n = Some(s); // 可观测性: M4 已写则不覆写
                }
                touched = true;
            }
        }
        if touched {
            out.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        touched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // 合成引擎（不依赖 wxq.bin）: 空 Pack（一切 raw → Err, MDLT/NNLM
    // 惰性解析恒失败 → mdltr_score=None / nnlm 缺席=open bin 语义）+
    // 空 DAT + 注入音节表与 BGRM 数组。子模块对 engine::Engine 私有
    // 字段可见 → 字段字面构造（同 dat.rs 微型手工构造惯例）。
    // =====================================================================

    fn leak(b: Vec<u8>) -> &'static [u8] {
        Box::leak(b.into_boxed_slice())
    }

    fn u32s(v: &[u32]) -> U32Arr {
        let mut b = Vec::new();
        for &x in v {
            b.extend_from_slice(&x.to_le_bytes());
        }
        U32Arr::new(leak(b))
    }

    fn u16s(v: &[u16]) -> U16Arr {
        let mut b = Vec::new();
        for &x in v {
            b.extend_from_slice(&x.to_le_bytes());
        }
        U16Arr::new(leak(b))
    }

    fn empty_dat() -> Dat {
        Dat {
            base: BitArr::new(leak(vec![0u8; 40]), 4),
            check: BitArr::new(leak(vec![0u8; 40]), 4),
            lo: BitArr::new(leak(vec![0u8; 40]), 4),
            cnt16: u16s(&[0; 24]),
            term: leak(vec![0u8; 8]),
            // walk 任一非空键: ns = base.get(...)+c ≥ 1 ≥ n_slots → -1
            n_slots: 1,
            n_keys: 0,
            cnt_ovf: HashMap::new(),
        }
    }

    fn empty_fc() -> FcStrings {
        FcStrings::new(leak(Vec::new()), u32s(&[0]))
    }

    fn synth(syls: &[&str]) -> Engine {
        let syllables: Vec<String> =
            syls.iter().map(|s| s.to_string()).collect();
        let seg = SyllableSeg::new(syllables.clone(), &[]);
        Engine {
            boost: None,
            t9: false,
            t9_sig_v: RefCell::new(None),
            bwcn: std::cell::RefCell::new(Vec::new()),
            pack: Pack {
                bytes: leak(Vec::new()),
                ver: 4,
                dir: HashMap::new(),
            },
            meta_n_words: 0,
            dat: empty_dat(),
            ridx_n: 0,
            ridx_off: u32s(&[0, 0]),
            soff: u32s(&[0, 0]),
            syl_blob: leak(vec![0u8]),
            wlist: u32s(&[0, 0]),
            wlsi_fv: None,
            n_words: 0,
            freq: u16s(&[0, 0]),
            _flags: leak(Vec::new()),
            fc: empty_fc(),
            whash_slots: u32s(&[0, 0]),
            whash_n: 1,
            whash_v3: true,
            hot_states: u32s(&[0, 0]),
            hot_off: u32s(&[0, 0]),
            hot_ids: u32s(&[0, 0]),
            syllables,
            seg,
            bi_n: 0,
            bi_w1: u32s(&[0, 0]),
            bi_w2: u32s(&[0, 0]),
            bi_val: u16s(&[0, 0]),
            w2r: u32s(&[0, 0]),
            jian_n: 0,
            jian_keys: empty_fc(),
            jian_eoff: u32s(&[0, 0]),
            jian_eblob: leak(Vec::new()),
            epyn_n: 0,
            epyn_keys: empty_fc(),
            epyn_vblob: leak(Vec::new()),
            epyn_tsv: HashMap::new(),
            c4g_pool: empty_fc(),
            c4g_npair: 0,
            c4g_w1: u32s(&[0, 0]),
            c4g_w2: u32s(&[0, 0]),
            c4g_val: u16s(&[0, 0]),
            wtext: RefCell::new(HashMap::new()),
            rid_memo: RefCell::new(HashMap::new()),
            epyn_parsed: RefCell::new(None),
            syl_by_initial: RefCell::new(None),
            syls_by_ini: RefCell::new(None),
            fb_ini_stack: RefCell::new(None),
            fb_syl_head: RefCell::new(None),
            ini_top_syls_c: RefCell::new(None),
            sylset_m_v: RefCell::new(None),
            slot_col_v: RefCell::new(None),
            slot_wc_v: RefCell::new(None),
            sylw: Vec::new(),
            user: RefCell::new(None),
            poly: RefCell::new(None),
            nnlm: RefCell::new(None),
            #[cfg(feature = "safetensors")]
            nnlm_adv: RefCell::new(None),
            now_override: std::cell::Cell::new(None),
            mdltr: RefCell::new(None),
            wfea: RefCell::new(None),
            engdict_v: RefCell::new(None),
            emix_ana_v: RefCell::new(None),
            emix_pin_rd: RefCell::new(None),
            ckvd: RefCell::new(None),
            stt: RefCell::new(None),
            tone_tab: RefCell::new(None),
            bwcn_texts: RefCell::new(None),
        }
    }

    /// 原厂编辑罚表（hpmac 十一旋钮: 替换 1.25 / 插入 2.88 / 删除 3.22）
    /// —— 插入便宜于删除的不对称律（悬空/吞噬域方向性）
    #[test]
    fn edit_cost_hpmac_penalty_table() {
        assert_eq!(Engine::edit_cost(b"", b""), 0.0);
        assert_eq!(Engine::edit_cost(b"ni", b"ni"), 0.0, "全等零罚");
        assert_eq!(Engine::edit_cost(b"n", b"t"), EDIT_SUB, "单替换 = 1.25");
        assert_eq!(Engine::edit_cost(b"n", b"nt"), EDIT_INS, "单插入 = 2.88");
        assert_eq!(Engine::edit_cost(b"nt", b"n"), EDIT_DEL, "单删除 = 3.22");
        assert_eq!(
            Engine::edit_cost(b"ab", b"cd"),
            2.0 * EDIT_SUB,
            "双替换 = 2×1.25"
        );
        // 同一对字母反向: 插入侧恒便宜于删除侧（不对称罚表律）
        assert!(
            Engine::edit_cost(b"n", b"nt") < Engine::edit_cost(b"nt", b"n"),
            "ins(2.88) < del(3.22)"
        );
    }

    /// Damerau 换位分支在原厂罚表下恒被双替换支配
    /// （EDIT_TRANS 2.99 > 2×EDIT_SUB 2.50）: 分支保留 = py 镜像
    /// 完备性, 最优路径永不取换位
    #[test]
    fn edit_cost_transposition_dominated_by_double_sub() {
        assert_eq!(
            Engine::edit_cost(b"ab", b"ba"),
            2.0 * EDIT_SUB,
            "换位面最优 = 双替换 2.50（2.99 分支落选）"
        );
        assert_eq!(
            Engine::edit_cost(b"abc", b"bac"),
            2.0 * EDIT_SUB,
            "首两位换位同律: 双替换价"
        );
    }

    /// bigram_after: bi_w1 searchsorted 左界带 [lower(wid), lower(wid+1))
    /// （值域 u32 → right(wid) == lower(wid+1) 律）
    #[test]
    fn bigram_after_searchsorted_band() {
        let mut e = synth(&["ni"]);
        e.bi_n = 3;
        e.bi_w1 = u32s(&[5, 5, 7]);
        e.bi_w2 = u32s(&[100, 101, 200]);
        // N = 1024/512/1536 → log10P = -1.0/-0.5/-1.5（四段量化段 1）
        e.bi_val = u16s(&[48128, 47616, 48640]);
        let b5 = e.bigram_after(5);
        assert_eq!(
            b5.ord,
            vec![(100u32, -1.0f64), (101, -0.5)],
            "wid=5 两行按 bi 序"
        );
        let b7 = e.bigram_after(7);
        assert_eq!(b7.ord, vec![(200u32, -1.5f64)], "wid=7 单行");
        assert!(
            e.bigram_after(6).is_empty(),
            "值域无 6 行（右界 = lower(wid+1) 不含）"
        );
        assert!(
            e.bigram_after(4).is_empty(),
            "左界外（lower(4)=lower(5) 前无行）"
        );
    }

    /// py dict 覆写语义: 同 w2 重复行 → map 取后值、ord 保插入序双记
    /// （首现位定序, 值域后者覆写）
    #[test]
    fn bigram_after_py_dict_overwrite_semantics() {
        let mut e = synth(&["ni"]);
        e.bi_n = 2;
        e.bi_w1 = u32s(&[5, 5]);
        e.bi_w2 = u32s(&[100, 100]);
        e.bi_val = u16s(&[48128, 47616]); // -1.0 被 -0.5 覆写
        let b = e.bigram_after(5);
        assert_eq!(b.ord.len(), 2, "ord 保首现位双记（行序）");
        assert_eq!(b.ord[0].1, -1.0, "ord 首行保原值");
        assert!(b.contains(100));
        assert_eq!(b.get(100), -0.5, "map 覆写值 = 后值（Python dict）");
    }

    /// ltr_dangling: 输入悬空尾（最大吞噬切分残留）(键数, 起位, 声母串)
    /// —— zh/ch/sh 双字母声母整取（zhongg→c_g_ 锚形）
    #[test]
    fn ltr_dangling_tail_and_zh_ch_sh_initial() {
        let e = synth(&["ni", "hao", "wo"]);
        assert_eq!(
            e.ltr_dangling("nihao"),
            (0, 0, String::new()),
            "全切分无悬空"
        );
        assert_eq!(e.ltr_dangling(""), (0, 0, String::new()));
        assert_eq!(
            e.ltr_dangling("wozh"),
            (2, 2, "zh".to_string()),
            "wo 消费后 zh 悬空 → 双字母声母"
        );
        assert_eq!(
            e.ltr_dangling("nihaoz"),
            (1, 5, "z".to_string()),
            "单字母悬空（z 尾声母, 起位 5）"
        );
    }

    /// ltr_boundary: 候选在途音节（读音音节横跨输入边界者）的
    /// (声母, 起键位)——nih→c_h_ 锚; Sentence/空读音/全消费 → 空槽
    #[test]
    fn ltr_boundary_inflight_syllable_slot() {
        let e = synth(&["ni"]);
        let mut c = Cand::new("你好".into(), -3.0, Kind::Exact);
        c.reading = "ni'hao".into();
        assert_eq!(
            e.ltr_boundary(&c, 4),
            ("h".to_string(), 2),
            "hao 跨界 → 在途声母 h 起位 2"
        );
        assert_eq!(
            e.ltr_boundary(&c, 5),
            (String::new(), 0),
            "全消费无在途"
        );
        let mut zh = Cand::new("中国".into(), -3.0, Kind::Exact);
        zh.reading = "zhong'guo".into();
        assert_eq!(
            e.ltr_boundary(&zh, 3),
            ("zh".to_string(), 0),
            "zhong 跨界 → zh 双字母声母"
        );
        let mut s = Cand::new("句".into(), -3.0, Kind::Sentence);
        s.reading = "ni'hao".into();
        assert_eq!(
            e.ltr_boundary(&s, 2),
            (String::new(), 0),
            "Sentence 免边界槽"
        );
        let blank = Cand::new("词".into(), -3.0, Kind::Exact);
        assert_eq!(
            e.ltr_boundary(&blank, 2),
            (String::new(), 0),
            "空读音 → 空槽"
        );
    }

    /// ltr_rerank 列分离律（第七役: 原生三 searcher 向量; hanhuo 按
    /// Kind 分列, 列内位次 = 数组位——入参约定已按 score 降序）:
    /// 无 mdltr（open bin）→ raw 全 0, 各列独立前 LTR_TOP_N 名发
    /// LTR_RANK_PRIOR−DECAY·rk, 列内 rk ≥ top-N 保原分
    #[test]
    fn ltr_rerank_column_split_and_rank_prior() {
        let e = synth(&["ni", "hao"]);
        let cand = |w: &str, k: Kind, sc: f64| {
            let mut c = Cand::new(w.into(), -5.0, k);
            c.score = sc;
            c.reading = "ni'hao".into();
            c
        };
        // col_a（Sentence|Path）3 条 + col_b（其余）6 条, 各列降序预排
        let mut cands = vec![
            cand("甲", Kind::Path, 3.0),
            cand("丙", Kind::Path, 2.0),
            cand("乙", Kind::Path, 1.0),
            cand("丁", Kind::Exact, 19.0),
            cand("戊", Kind::Exact, 18.0),
            cand("己", Kind::Exact, 17.0),
            cand("庚", Kind::Exact, 16.0),
            cand("辛", Kind::Exact, 15.0),
            cand("壬", Kind::Exact, 14.0),
        ];
        e.ltr_rerank(&mut cands, 5, "nihao", false);
        let want =
            |rk: f64| LTR_RANK_PRIOR - LTR_RANK_DECAY * rk;
        // col_a 列内位次 0/1/2 → 5.0/4.5/4.0（与 col_b 互不挤位）
        assert!((cands[0].score - want(0.0)).abs() < 1e-9, "Path 首名 = 先验 5.0");
        assert!((cands[1].score - want(1.0)).abs() < 1e-9, "Path 次名 = 4.5");
        assert!((cands[2].score - want(2.0)).abs() < 1e-9, "Path 第三 = 4.0");
        // col_b 各自位次: 19..16 → top-4 重写; 15/14 rk≥LTR_TOP_N 保原分
        assert!((cands[3].score - want(0.0)).abs() < 1e-9, "col_b 独立首名 = 5.0");
        assert!((cands[6].score - want(3.0)).abs() < 1e-9, "第 4 名 = 5.0−0.5×3");
        assert_eq!(cands[7].score, 15.0, "rk≥4 保原分（top-4 门）");
        assert_eq!(cands[8].score, 14.0);
        assert!(
            cands.iter().all(|c| c.ltr == Some(0.0)),
            "无 mdltr → raw 全 0 但 ltr 槽必写"
        );
    }

    /// mk: rid<0 → w2r 反查读音（空表回退 reading=""）;
    /// fv 槽路径锁 lp（v2 per-(读音,词) 频次伴列律 lp_locked）
    #[test]
    fn mk_w2r_reverse_lookup_and_fv_lp_lock() {
        let mut e = synth(&["ni", "hao"]);
        // 单词 fc: 词 0 = 但（checkpoint 独立记录: [共享前缀, 字节长, UTF-8]）
        e.fc = FcStrings::new(
            leak(vec![0u8, 3, 0xE4, 0xBD, 0x86]),
            u32s(&[0]),
        );
        e.freq = u16s(&[48128]); // N=1024 → logP = −1.0
        e.w2r = u32s(&[1]); // wid0 → rid0
        e.syl_blob = leak(vec![1, 1, 0, 0]); // 变体0 = [音节0] → "ni"
        // rid=-1 → w2r 反查 rid=0 → reading = 音节表[0]
        let c = e.mk(0, -1, Kind::Exact, 0, None, -1.25, None);
        assert_eq!(c.word, "但", "词面 = fc.get(wid)");
        assert_eq!(c.reading, "ni", "rid<0 → w2r 反查读音");
        assert_eq!(c.logp, -1.0, "fv=None → 词频层 word_logp");
        assert!(!c.lp_locked, "词频层路径 lp 不锁");
        assert_eq!(c.edit_penalty, -1.25);
        assert_eq!(c.wid, 0);
        // fv=Some → per-(读音,词) 伴列锁 lp
        let c2 = e.mk(0, -1, Kind::Exact, 0, None, 0.0, Some(47616)); // −0.5
        assert_eq!(c2.logp, -0.5, "fv 槽 lp 覆词频层");
        assert!(c2.lp_locked, "v2 伴列路径 lp 锁定");
        // w2r 空（槽 0 = 无读音哨兵）→ rid −1 → 空 reading
        e.w2r = u32s(&[0]);
        let c3 = e.mk(0, -1, Kind::Exact, 0, None, 0.0, None);
        assert_eq!(c3.reading, "", "空 w2r → 无读音");
    }

    /// M4 open bin 语义: 无 NNLM → nnlm_section None / nnlm_ok false /
    /// nnlm_model None / nnlm_rerank 整块跳过保序（役K agentZ
    /// `_nnlm_ok` 镜像; 空包惰性解析失败路径）
    #[test]
    fn nnlm_open_bin_skip_semantics() {
        let e = synth(&["ni"]);
        assert!(e.nnlm_section().is_none(), "空 Pack 无 NNLM 区");
        assert!(!e.nnlm_ok(), "惰性解析失败 → 不可用");
        assert!(e.nnlm_model().is_none());
        let mut cands = vec![Cand::new("你好".into(), -3.0, Kind::Exact)];
        cands[0].score = -2.0;
        let touched = e.nnlm_rerank(&mut cands, None);
        assert!(!touched, "无模型 → false（整块跳过）");
        assert_eq!(cands[0].score, -2.0, "保序: 无 clamp 融合位移");
        assert!(cands[0].nn_n.is_none(), "NN 分数不写");
    }

    /// user_ctx_adjust 门律（免数据半边）: 无 ctx / 空 ctx / 无 NNLM →
    /// 恒等 false 且分数不动。有 NNLM 的语境方向性门由
    /// tests/ctx_dyn_rank.rs 真机探针覆盖。
    #[test]
    fn user_ctx_adjust_gates_no_ctx_no_nnlm() {
        let e = synth(&["ni"]);
        let mut cands = vec![Cand::new("你好".into(), -3.0, Kind::User)];
        cands[0].score = -2.0;
        cands[0].user_bias = 1.0;
        // 门 1: 无 ctx / 空 ctx（py falsy 语义）
        assert!(!e.user_ctx_adjust(None, &mut cands));
        assert!(!e.user_ctx_adjust(Some(""), &mut cands));
        // 门 2: 无 NNLM（synth 空 Pack 惰性解析恒失败 → 保序）
        assert!(!e.user_ctx_adjust(Some("我们"), &mut cands));
        assert_eq!(cands[0].score, -2.0, "全门恒等: 分数不动");
        assert!(cands[0].nn_n.is_none(), "NN 分不写");
        // 门 3: 无用户信号 → 恒等（NNLM 缺席时不可达, 此处锁语义文档）
    }
}
