//! 全量/整词通道: 简拼/纠错音节/DAT 子树词/短语/悬空补全.
use super::*;

impl Engine {
    // ------------------------------------------------------------------
    // L3: 简拼 / 纠错 / 联想
    // ------------------------------------------------------------------
    /// 简拼键 → 展开音节 id 序列表（精确键优先，否则前缀键区间）
    pub(super) fn jianpin_keys(&self, inp: &str) -> Vec<Vec<u32>> {
        let n = self.jian_n;
        // 二分精确（含 Python break 语义）
        let (mut a, mut b) = (0usize, n);
        while a < b {
            let mid = (a + b) / 2;
            let k = self.jian_keys.get(mid);
            if k == inp {
                a = mid;
                break;
            }
            if k.as_str() < inp {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        let rng: Vec<usize> = if a < n && self.jian_keys.get(a) == inp {
            vec![a]
        } else {
            // 前缀界二分（'{' > 'z'）
            let mut needle = Vec::with_capacity(inp.len() + 1);
            needle.extend_from_slice(inp.as_bytes());
            needle.push(b'{');
            let needle = String::from_utf8(needle).unwrap();
            let (mut lo, mut hi) = (a, n);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if self.jian_keys.get(mid) < needle {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            (a..lo.min(a + 64)).collect()
        };
        let mut out = Vec::new();
        let v3 = self.pack.ver >= 3;
        for &i in &rng {
            let o0 = self.jian_eoff.get(i) as usize;
            let _o1 = self.jian_eoff.get(i + 1) as usize;
            let eb = self.jian_eblob;
            let mut p = o0;
            let nexp = eb[p] as usize | ((eb[p + 1] as usize) << 8);
            p += 2;
            for _ in 0..nexp {
                if v3 {
                    let rid = eb[p] as u32
                        | ((eb[p + 1] as u32) << 8)
                        | ((eb[p + 2] as u32) << 16)
                        | ((eb[p + 3] as u32) << 24);
                    p += 4;
                    let vs = self.rid_variants(rid);
                    if let Some(first) = vs.first() {
                        out.push(first.clone());
                    }
                } else {
                    let ln = eb[p] as usize | ((eb[p + 1] as usize) << 8);
                    p += 2;
                    let mut ids = Vec::with_capacity(ln);
                    for _ in 0..ln {
                        ids.push(eb[p] as u32 | ((eb[p + 1] as u32) << 8));
                        p += 2;
                    }
                    out.push(ids);
                }
            }
        }
        out
    }

    /// B2.6b 简拼格 DP（boost 旋钮）: v3 诊断的两层门——① q_jianpin 只做
    /// 单词 range 查询, 多词句（英国|皇家|学会）从不进简拼面板; ② 音节
    /// 权重束结构性无多样性（yige 系霸占全部槽位, 金标读音出局）。
    /// 解法: 状态 = (已消费字母, 已闭词列, 当前词前缀 DAT 态)。闭词
    /// （terminal）即累积词 logp——词层信号在搜索内。产出 = 完整消耗
    /// 输入且全部闭词的路径, 按路径分降序。
    pub(super) fn jianpin_sent_beam(&self, inp: &str, cap: usize) -> Vec<Cand> {
        use crate::dat::ROOT;
        let letters: Vec<u8> = inp.bytes().collect();
        let n = letters.len();
        if !(3..=12).contains(&n) {
            return Vec::new();
        }
        let mut cache = self.syls_by_ini.borrow_mut();
        let inimap = cache.get_or_insert_with(|| {
            let mut m: HashMap<u8, Vec<String>> = HashMap::new();
            for s in &self.syllables {
                m.entry(s.as_bytes()[0]).or_default().push(s.clone());
            }
            m
        });
        let id_of: HashMap<&str, u32> = self
            .syllables
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i as u32))
            .collect();
        // 每首字母音节组（id 形式）
        let mut by_ini: HashMap<u8, Vec<u32>> = HashMap::new();
        for (c, ss) in inimap.iter() {
            by_ini.insert(
                *c,
                ss.iter()
                    .filter_map(|s| id_of.get(s.as_str()).copied())
                    .collect(),
            );
        }
        drop(cache);
        // 精确闭词: range() 返回前缀族（pi 族含 pipixia）——必须取
        // key==cur 的 rid, 否则 3 音节词被 1 字母误闭（皮皮虾 案）
        let exact_rid = |lo: u32, cnt: u32, cur: &str| -> Option<u32> {
            (lo..lo + cnt).find(|&r| {
                self.rid_variants(r).first().map(|v| {
                    v.iter()
                        .map(|&i| self.syllables[i as usize].clone())
                        .collect::<String>()
                }) == Some(cur.to_string())
            })
        };
        struct St {
            sc: f64,
            words: Vec<u32>, // 词 id 序列
            cur: String,     // 当前未闭词的音节前缀
            cur_n: usize,    // cur 已累计音节数（期望闭词成本用）
            key: String,     // 全路径音节串（闭词不清——键级枚举用）
            first: Option<u32>, // 首音节 id（束多样性配额维度）
            st: i32,         // cur 的 DAT 态（ROOT=已闭）
        }
        // 期望闭词成本: cur 收尾至少付 cur_n 个音节的词分（单音节词最优
        // lp 估计 -2.5）。不收则构造态免费搭车, 廉价首词家族霸桶
        // （v3 yghjxh: [英国,-4.7] 被 20 个 -2.0 单字首词×24 配额逐出）
        const SYL_LP_EST: f64 = -2.5;
        fn eff(s: &St) -> f64 {
            s.sc + s.cur_n as f64 * SYL_LP_EST
        }
        const B: usize = 2048; // 层束宽（分桶后总宽; 桶内高频单字垃圾链挤金标→宽束保覆盖）
        let mut layer: Vec<St> = vec![St { sc: 0.0, words: Vec::new(), cur: String::new(), cur_n: 0, key: String::new(), first: None, st: ROOT }];
        for &lc in &letters {
            let Some(cands) = by_ini.get(&lc) else {
                return Vec::new();
            };
            let mut next: Vec<St> = Vec::with_capacity(layer.len() * (cands.len() + 1));
            let mut dedup: HashMap<(Vec<u32>, String), usize> = HashMap::new();
            for stt in &layer {
                // 变体: 原状态 + （terminal 时）闭词变体（消费零字母）
                let mut variants: Vec<St> = vec![St {
                    sc: stt.sc,
                    words: stt.words.clone(),
                    cur: stt.cur.clone(),
                    cur_n: stt.cur_n,
                    key: stt.key.clone(),
                    first: stt.first,
                    st: stt.st,
                }];
                if stt.st != ROOT && self.dat.is_terminal(stt.st) {
                    // 闭词 = 精确 key==cur 的 rid 列首词（频序首）;
                    // 前缀非词（如 "pipi" 无独立词）→ 只扩展不闭
                    let (lo, cnt) = self.dat.range(stt.st);
                    if let Some(rid) = exact_rid(lo, cnt, &stt.cur) {
                        let (o0, _o1) = self.reading_slice(rid);
                        let w = self.wlist_at(o0);
                        let mut words2 = stt.words.clone();
                        words2.push(w);
                        variants.push(St {
                            sc: stt.sc + self.word_logp(w),
                            words: words2,
                            cur: String::new(),
                            cur_n: 0,
                            key: stt.key.clone(),
                            first: stt.first,
                            st: ROOT,
                        });
                    }
                }
                for v in variants {
                    for &sid in cands {
                        let syl = &self.syllables[sid as usize];
                        let mut cur2 = v.cur.clone();
                        cur2.push_str(syl);
                        // walk 从 v.st 走【增量音节】——传累积串会把前缀走两遍
                        let st2 = self.dat.walk(syl.as_bytes(), v.st);
                        if st2 < 0 {
                            continue;
                        }
                        let mut key2 = v.key.clone();
                        key2.push_str(syl);
                        let first = if v.key.is_empty() { Some(sid) } else { v.first };
                        let cand = St { sc: v.sc, words: v.words.clone(), cur: cur2, cur_n: v.cur_n + 1, key: key2, first, st: st2 };
                        let key = (cand.words.clone(), cand.cur.clone());
                        match dedup.get(&key) {
                            Some(&ix) if next[ix].sc >= cand.sc => {}
                            Some(&ix) => next[ix] = cand,
                            None => {
                                dedup.insert(key, next.len());
                                next.push(cand);
                            }
                        }
                    }
                }
            }
            // 分桶截断: 未闭前缀态 sc=0 的乐观霸权会把已付词分的句形
            // 路径整层挤出（lazy 剪枝病理）; 且不同词数的路径分不可同序
            // 比较——按已闭词数 0..=6 精确分桶各留 B/7（min(3) 合桶曾把
            // 4 词金标阶段挤出桶 3）。
            // 层内排序用 eff（期望成本压平构造态乐观）; 终选仍用裸 sc
            next.sort_by(|a, b| eff(b).partial_cmp(&eff(a)).unwrap_or(std::cmp::Ordering::Equal));
            let mut kept: Vec<St> = Vec::with_capacity(B);
            'outer: for bkt in 0..=6 {
                let mut got = 0;
                // 二级配额: 同首音节 ≤12 态——廉价首音节（yi/yao/you 系）
                // 的子树霸满桶（键层 1226 条零 ying 前缀实证; 首词维度
                // 配额被单字词闭词绕过, 首音节是上游一刀）
                let mut per_first: HashMap<(usize, u32), usize> = HashMap::new();
                for stt in next.iter() {
                    if stt.words.len() == bkt {
                        let fk = (
                            bkt,
                            stt.first.unwrap_or(u32::MAX),
                        );
                        let c = per_first.entry(fk).or_insert(0);
                        if *c >= 12 {
                            continue;
                        }
                        *c += 1;
                        kept.push(St { sc: stt.sc, words: stt.words.clone(), cur: stt.cur.clone(), cur_n: stt.cur_n, key: stt.key.clone(), first: stt.first, st: stt.st });
                        got += 1;
                        if got >= B / 7 {
                            continue 'outer;
                        }
                    }
                }
            }
            layer = kept;
        }
        // 终局: 键级枚举——格 DP 只产出 diverse 全拼键（列首闭词仅作
        // 剪枝引导与排序先验）; 词选择与打分交 dp_path（校准 slot logp
        // + 全词搜索——非列首词 脚/鞋 由此可达, v3 列首+裸sc 版仅 +0.1pp）
        let mut keys: Vec<(f64, String)> = Vec::new();
        for stt in layer {
            let ok = stt.st == ROOT
                || (self.dat.is_terminal(stt.st) && {
                    let (lo, cnt) = self.dat.range(stt.st);
                    exact_rid(lo, cnt, &stt.cur).is_some()
                });
            if ok && !stt.key.is_empty() {
                keys.push((stt.sc, stt.key.clone()));
            }
        }
        keys.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut seen_key: HashSet<String> = HashSet::new();
        let mut out: Vec<(f64, Cand)> = Vec::new();
        'kloop: for (sc0, key) in keys {
            if !seen_key.insert(key.clone()) {
                continue;
            }
            let Some((words, sc, _)) = self.dp_path(&key) else {
                continue;
            };
            if words.is_empty() {
                continue;
            }
            let texts: Vec<String> =
                words.iter().map(|&(w, _)| self.word_text(w)).collect();
            let text = texts.concat();
            if text.is_empty() || out.iter().any(|(_, c)| c.word == text) {
                continue;
            }
            let mut c = Cand::new(text, sc, crate::Kind::Sentence);
            c.wid = words[words.len() - 1].0 as i64;
            c.path_words = Some(texts);
            c.logp = sc;
            out.push((sc0.max(sc), c));
            if out.len() >= cap.min(24) {
                break 'kloop;
            }
        }
        out.sort_by(|a, b| b.1.logp.partial_cmp(&a.1.logp).unwrap_or(std::cmp::Ordering::Equal));
        out.into_iter().map(|(_, c)| c).collect()
    }


    /// c.3 词块感知组装（设计-上下文动态组词 §c.3）: 非合法全拼切分输入
    /// 的词块优先**部件**枚举——转移即「词」: 每位置先枚举词典多音节词块
    /// （连声母位整块, 如 爆米花 b-m-h; 每音节长度分桶保席位——纯 lp 序
    /// 会被双音节高频块挤掉三音节中频块）, 再枚举单字词 top-K。本层神经
    /// 无关, 只出部件（每位置 k-best 前缀/后缀 + 词块 + 终集）; NNLM(ctx)
    /// 预筛与组面在 boost 侧——「预筛打分在 beam 内做不入面板」, 部件分
    /// 永不入面板, 统一面分才写回。B2.6b 可达性病理（单字垃圾链中途
    /// 挤掉词块支路）由首词分桶配额 + 每块保底部件双重治愈。
    pub(super) fn jianpin_block_parts(
        &self,
        inp: &str,
    ) -> Option<JpParts> {
        self.jianpin_block_parts_constrained(inp, &HashMap::new())
    }

    /// 混合输入泛化（设计-混合输入 §2.2）: `constrain` = 位置 → 精确音节
    /// （全拼段位收窄: singles 只取该音节 reading 列, 块 DFS 每步过约束）。
    /// 空 constrain = 纯声母域（c.3 原行为）。
    pub(super) fn jianpin_block_parts_constrained(
        &self,
        inp: &str,
        constrain: &HashMap<usize, String>,
    ) -> Option<JpParts> {
        use crate::dat::ROOT;
        let letters: Vec<u8> = inp.bytes().collect();
        let n = letters.len();
        if !(3..=12).contains(&n) {
            return None;
        }
        let mut cache = self.syls_by_ini.borrow_mut();
        let inimap = cache.get_or_insert_with(|| {
            let mut m: HashMap<u8, Vec<String>> = HashMap::new();
            for s in &self.syllables {
                m.entry(s.as_bytes()[0]).or_default().push(s.clone());
            }
            m
        });
        let id_of: HashMap<&str, u32> = self
            .syllables
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i as u32))
            .collect();
        let mut by_ini: HashMap<u8, Vec<u32>> = HashMap::new();
        for (c, ss) in inimap.iter() {
            by_ini.insert(
                *c,
                ss.iter()
                    .filter_map(|s| id_of.get(s.as_str()).copied())
                    .collect(),
            );
        }
        drop(cache);
        // 与 jianpin_sent_beam 同律: 精确闭词取 key==cur 的 rid（皮皮虾案）
        let exact_rid = |lo: u32, cnt: u32, cur: &str| -> Option<u32> {
            (lo..lo + cnt).find(|&r| {
                self.rid_variants(r).first().map(|v| {
                    v.iter()
                        .map(|&i| self.syllables[i as usize].clone())
                        .collect::<String>()
                }) == Some(cur.to_string())
            })
        };
        // 单音节词列（每位置: 各候选音节 reading 列频序 top-K 单字, 合并
        // 后 lp 降序截宽）——单字词是全覆盖的兜底转移
        const SINGLE_K: usize = 3;
        const SINGLE_W: usize = 24;
        let mut singles: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
        for (i, &lc) in letters.iter().enumerate() {
            // 全拼段约束（混合输入）: 该位只允许精确音节
            if let Some(exact) = constrain.get(&i) {
                let syl = exact.as_str();
                if syl.as_bytes()[0] != lc {
                    return None; // 约束与键位不符（调用方切分错误）
                }
                let st = self.dat.walk(syl.as_bytes(), ROOT);
                if st < 0 || !self.dat.is_terminal(st) {
                    return None;
                }
                let (lo, cnt) = self.dat.range(st);
                let Some(rid) = exact_rid(lo, cnt, syl) else {
                    return None;
                };
                let (o0, o1) = self.reading_slice(rid);
                let mut cands: Vec<(f64, u32)> = Vec::new();
                for oi in o0..o1.min(o0 + SINGLE_W) {
                    let w = self.wlist_at(oi);
                    cands.push((self.word_logp(w), w));
                }
                cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                singles[i] =
                    cands.into_iter().map(|(lp, w)| (w, lp)).collect();
                if singles[i].is_empty() {
                    return None;
                }
                continue;
            }
            let Some(ss) = by_ini.get(&lc) else {
                return None;
            };
            let mut cands: Vec<(f64, u32)> = Vec::new();
            for &sid in ss {
                let syl = &self.syllables[sid as usize];
                let st = self.dat.walk(syl.as_bytes(), ROOT);
                if st < 0 || !self.dat.is_terminal(st) {
                    continue;
                }
                let (lo, cnt) = self.dat.range(st);
                let Some(rid) = exact_rid(lo, cnt, syl) else {
                    continue;
                };
                let (o0, o1) = self.reading_slice(rid);
                for oi in o0..o1.min(o0 + SINGLE_K) {
                    let w = self.wlist_at(oi);
                    cands.push((self.word_logp(w), w));
                }
            }
            cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            cands.truncate(SINGLE_W);
            singles[i] =
                cands.into_iter().map(|(lp, w)| (w, lp)).collect();
            if singles[i].is_empty() {
                return None; // 某位无任何单字词 → 全覆盖不达
            }
        }
        // 多音节词块 DFS: 每位置起, 沿词层 trie 走 ≥2 音节的 terminal
        const BLOCK_BUDGET: usize = 40_000;
        let mut blocks_at: Vec<Vec<(usize, u32, f64)>> = vec![Vec::new(); n];
        let mut budget = BLOCK_BUDGET;
        for i in 0..n {
            // 栈: (字母位, DAT 态, 累积音节串, 已走音节数)
            let mut stack: Vec<(usize, i32, String, usize)> =
                vec![(i, ROOT, String::new(), 0)];
            while let Some((j, st, cur, cnt)) = stack.pop() {
                if budget == 0 {
                    break;
                }
                budget -= 1;
                let Some(ss) = by_ini.get(&letters[j]) else {
                    continue;
                };
                for &sid in ss {
                    let syl = &self.syllables[sid as usize];
                    // 全拼段约束: 该步必须精确命中（块吞并全拼段合法——
                    // 块的 lp 自然带频次偏好, 不另加权）
                    if let Some(exact) = constrain.get(&j) {
                        if syl != exact {
                            continue;
                        }
                    }
                    let st2 = self.dat.walk(syl.as_bytes(), st);
                    if st2 < 0 {
                        continue;
                    }
                    let mut cur2 = cur.clone();
                    cur2.push_str(syl);
                    let cnt2 = cnt + 1;
                    if cnt2 >= 2 && self.dat.is_terminal(st2) {
                        let (lo, c) = self.dat.range(st2);
                        if let Some(rid) = exact_rid(lo, c, &cur2) {
                            let (o0, o1) = self.reading_slice(rid);
                            for oi in o0..o1.min(o0 + 2) {
                                let w = self.wlist_at(oi);
                                blocks_at[i]
                                    .push((j + 1, w, self.word_logp(w)));
                            }
                        }
                    }
                    if j + 1 < n {
                        stack.push((j + 1, st2, cur2, cnt2));
                    }
                }
            }
            // 长度分桶截断: 每音节长度保 top-8 席位（爆米花 正解位）。
            // 混合串（constrain 非空）放宽到 12: 全拼段约束位候选本就少,
            // 无约束位与纯声母域同级竞争——中频块（天青色 -6.36 类,
            // t 开头 3 音节块 9+ 个更高频）在 8 席下不可达, ctx-NNLM
            // 臂失去裁决机会（tqsdyu 实证）; 纯声母域 8 席零变化（c.3
            // 已标定行为）。
            let per_len_cap =
                if constrain.is_empty() { 8 } else { 12 };
            {
                let raw = std::mem::take(&mut blocks_at[i]);
                let mut dedup: HashSet<(usize, u32)> = HashSet::new();
                let mut by_len: std::collections::BTreeMap<
                    usize,
                    Vec<(usize, u32, f64)>,
                > = std::collections::BTreeMap::new();
                for t @ (end, w, _) in raw {
                    if dedup.insert((end, w)) {
                        by_len.entry(end - i).or_default().push(t);
                    }
                }
                let mut kept: Vec<(usize, u32, f64)> = Vec::new();
                for (_l, mut v) in by_len {
                    v.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
                    v.truncate(per_len_cap);
                    kept.extend(v);
                }
                kept.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
                blocks_at[i] = kept;
            }
        }
        if std::env::var("CTX_DBG").is_ok() {
            for (i, bs) in blocks_at.iter().enumerate() {
                eprintln!(
                    "[c3blk] pos={} 块数={} top={}",
                    i,
                    bs.len(),
                    bs.iter()
                        .take(4)
                        .map(|&(e, w, lp)| format!(
                            "{}(→{}, {:.2})",
                            self.word_text(w),
                            e,
                            lp
                        ))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            eprintln!("[c3blk] walk 预算余 {}", budget);
        }
        // k-best 前缀 DP: kb[p] = 覆盖 [0..p) 的 top-K 路径（lp 序）。
        // 转移严格前向（singles→p+1, blocks→end>p+1）, 按位递增处理时
        // 目标层在轮到它之前已收齐全部来源。
        // **按词数分桶**: lp 序会被垃圾词块（小命都 −4.0 一词盖三字母）
        // 挤掉逐字路径（想|买|点 −9.4）——词块路/逐字路各保席位,
        // NNLM 预筛才能在多样结构里选出语境契合者。
        const K_BUCKET: usize = 6;
        const K_TOTAL: usize = 24;
        let bucketize = |v: &mut Vec<(f64, Vec<u32>)>| {
            v.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            v.dedup_by(|a, b| a.1 == b.1);
            if v.len() > K_TOTAL {
                let mut per_wc: std::collections::HashMap<usize, usize> =
                    std::collections::HashMap::new();
                let mut kept: Vec<(f64, Vec<u32>)> =
                    Vec::with_capacity(K_TOTAL);
                for t in v.drain(..) {
                    if kept.len() >= K_TOTAL {
                        break;
                    }
                    let e = per_wc.entry(t.1.len()).or_insert(0);
                    if *e >= K_BUCKET {
                        continue;
                    }
                    *e += 1;
                    kept.push(t);
                }
                *v = kept;
            }
        };
        let mut kb_to: Vec<Vec<(f64, Vec<u32>)>> =
            (0..=n).map(|_| Vec::new()).collect::<Vec<_>>();
        kb_to[0].push((0.0, Vec::new()));
        for p in 0..n {
            let src = std::mem::take(&mut kb_to[p]);
            for (bl, bw) in &src {
                for &(w, lp) in &singles[p] {
                    let mut ws = bw.clone();
                    ws.push(w);
                    kb_to[p + 1].push((bl + lp, ws));
                }
                for &(end, w, lp) in &blocks_at[p] {
                    let mut ws = bw.clone();
                    ws.push(w);
                    kb_to[end].push((bl + lp, ws));
                }
            }
            // p+1 与各块跳迁落点分别聚合（落点未轮到前收齐来源）
            bucketize(&mut kb_to[p + 1]);
            for &(end, _, _) in &blocks_at[p] {
                if end != p + 1 {
                    bucketize(&mut kb_to[end]);
                }
            }
        }
        // k-best 后缀 DP: kb[p] = 覆盖 [p..n) 的 top-K 路径（倒序同理,
        // 同按词数分桶）
        let mut kb_from: Vec<Vec<(f64, Vec<u32>)>> =
            (0..=n).map(|_| Vec::new()).collect::<Vec<_>>();
        kb_from[n].push((0.0, Vec::new()));
        for p in (0..n).rev() {
            let mut best: Vec<(f64, Vec<u32>)> = Vec::new();
            for &(w, lp) in &singles[p] {
                for (sl, sw) in &kb_from[p + 1] {
                    let mut ws = vec![w];
                    ws.extend(sw.iter().copied());
                    best.push((lp + *sl, ws));
                }
            }
            for &(end, w, lp) in &blocks_at[p] {
                for (sl, sw) in &kb_from[end] {
                    let mut ws = vec![w];
                    ws.extend(sw.iter().copied());
                    best.push((lp + *sl, ws));
                }
            }
            bucketize(&mut best);
            kb_from[p] = best;
        }
        // 纯逐字最优路径保底入池: 全 singles 的贪心最优（想|买|点 类）
        // ——lp 桶内仍可能被同构变体（想妈的 -8.8 > 想买点 -9.4）挤掉,
        // 但逐字结构是无 ctx 时的正确基线（哦我知道你 律）, 必须在场
        // 与块路同台由 NNLM 裁决。
        {
            let mut sb_to: Vec<(f64, Vec<u32>)> =
                (0..=n).map(|_| (f64::NEG_INFINITY, Vec::new()))
                    .collect::<Vec<_>>();
            sb_to[0] = (0.0, Vec::new());
            for p in 0..n {
                if sb_to[p].0 == f64::NEG_INFINITY {
                    continue;
                }
                let (bl, bw) = sb_to[p].clone();
                let (w, lp) = singles[p][0];
                let mut ws = bw.clone();
                ws.push(w);
                sb_to[p + 1] = (bl + lp, ws);
            }
            for p in 0..=n {
                if sb_to[p].0 != f64::NEG_INFINITY
                    && !kb_to[p]
                        .iter()
                        .any(|(_, ws)| *ws == sb_to[p].1)
                {
                    kb_to[p].push(sb_to[p].clone());
                }
            }
            let mut sb_from: Vec<(f64, Vec<u32>)> =
                (0..=n).map(|_| (f64::NEG_INFINITY, Vec::new()))
                    .collect::<Vec<_>>();
            sb_from[n] = (0.0, Vec::new());
            for p in (0..n).rev() {
                let mut best: (f64, Vec<u32>) =
                    (f64::NEG_INFINITY, Vec::new());
                for &(w, lp) in singles[p].iter().take(1) {
                    if sb_from[p + 1].0 == f64::NEG_INFINITY {
                        continue;
                    }
                    let (sl, sw) = &sb_from[p + 1];
                    let mut ws = vec![w];
                    ws.extend(sw.iter().copied());
                    best = (lp + *sl, ws);
                }
                sb_from[p] = best;
            }
            for p in 0..n {
                if sb_from[p].0 != f64::NEG_INFINITY
                    && !kb_from[p]
                        .iter()
                        .any(|(_, ws)| *ws == sb_from[p].1)
                {
                    kb_from[p].push(sb_from[p].clone());
                }
            }
        }
        // 位置 DAG beam（终集供给——首词分桶配额的完整路径多样性）
        const W: usize = 3072;
        const PER_FIRST: usize = 64;
        struct P {
            lp: f64,
            words: Vec<u32>,
        }
        let mut states: Vec<Vec<P>> =
            (0..=n).map(|_| Vec::new()).collect::<Vec<_>>();
        states[0].push(P { lp: 0.0, words: Vec::new() });
        for p in 0..n {
            {
                let mut layer = std::mem::take(&mut states[p]);
                let mut dedup: HashMap<Vec<u32>, usize> = HashMap::new();
                let mut uniq: Vec<P> = Vec::with_capacity(layer.len());
                for c in layer {
                    match dedup.get(&c.words) {
                        Some(&ix) if uniq[ix].lp >= c.lp => {}
                        Some(&ix) => uniq[ix] = c,
                        None => {
                            dedup.insert(c.words.clone(), uniq.len());
                            uniq.push(c);
                        }
                    }
                }
                uniq.sort_by(|a, b| b.lp.partial_cmp(&a.lp).unwrap());
                let mut per_first: HashMap<u32, usize> = HashMap::new();
                let mut kept: Vec<P> = Vec::with_capacity(W);
                for c in uniq {
                    if kept.len() >= W {
                        break;
                    }
                    let fk =
                        c.words.first().copied().unwrap_or(u32::MAX);
                    let e = per_first.entry(fk).or_insert(0);
                    if *e >= PER_FIRST {
                        continue;
                    }
                    *e += 1;
                    kept.push(c);
                }
                states[p] = kept;
            }
            let layer = std::mem::take(&mut states[p]);
            for c in &layer {
                for &(w, lp) in &singles[p] {
                    let mut ws = c.words.clone();
                    ws.push(w);
                    states[p + 1].push(P { lp: c.lp + lp, words: ws });
                }
                for &(end, w, lp) in &blocks_at[p] {
                    let mut ws = c.words.clone();
                    ws.push(w);
                    states[end].push(P { lp: c.lp + lp, words: ws });
                }
            }
        }
        let mut finals: Vec<(f64, Vec<u32>)> = states
            .into_iter()
            .nth(n)
            .map(|v| v.into_iter().map(|p| (p.lp, p.words)).collect())
            .unwrap_or_default();
        finals.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        finals.dedup_by(|a, b| a.1 == b.1);
        finals.truncate(48);
        // 扁平化词块
        let blocks: Vec<(usize, usize, u32, f64)> = blocks_at
            .iter()
            .enumerate()
            .flat_map(|(i, bs)| {
                bs.iter().map(move |&(end, w, lp)| (i, end, w, lp))
            })
            .collect();
        Some(JpParts { n, singles, blocks, kb_to, kb_from, finals })
    }
    /// B2.6 简拼 DAT 制导展开（boost 旋钮, 关=零调用）: 盲合成（jianpin_compose）
    /// 的首字母笛卡尔积在 ≥4 键时超容量——v3 诊断 sent 70% 表缺键→兜底几乎
    /// 全灭。改为沿词层 trie 游走: 每步取以当前输入字母开头、且前缀在 DAT
    /// 活着的音节（sylw 权重贪心序, 稳定）, 完整消耗输入才产出——产出键
    /// 天然有词层 range 可查, 检索质量与表键同级。
    pub(super) fn jianpin_dat_compose(&self, inp: &str, cap: usize) -> Vec<Vec<u32>> {
        let letters: Vec<u8> = inp.bytes().collect();
        if letters.is_empty() || letters.len() > 12 {
            return Vec::new();
        }
        let mut cache = self.syls_by_ini.borrow_mut();
        let inimap = cache.get_or_insert_with(|| {
            let mut m: HashMap<u8, Vec<String>> = HashMap::new();
            for s in &self.syllables {
                m.entry(s.as_bytes()[0]).or_default().push(s.clone());
            }
            m
        });
        // 音节名→id, 每首字母组按 sylw 权重降序（稳定: 同权保表序）
        let id_of: HashMap<&str, u32> = self
            .syllables
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i as u32))
            .collect();
        let mut by_ini: HashMap<u8, Vec<u32>> = HashMap::new();
        for (c, ss) in inimap.iter() {
            let mut ids: Vec<u32> = ss
                .iter()
                .filter_map(|s| id_of.get(s.as_str()).copied())
                .collect();
            ids.sort_by(|&a, &b| self.sylw[b as usize].cmp(&self.sylw[a as usize]));
            by_ini.insert(*c, ids);
        }
        drop(cache);
        let mut out: Vec<Vec<u32>> = Vec::new();
        let mut budget: usize = 50_000; // walk 步预算（最坏界: 每步一次 DAT 走查）
        let mut maxd = 0usize;
        jianpin_dfs(
            self, &by_ini, &letters, 0, &mut String::new(),
            &mut Vec::new(), &mut out, &mut budget, cap, &mut maxd,
        );
        out
    }

    /// 裸键合成兜底：按首字母组合音节 × DAT 前缀剪枝（表键缺口免疫，如 jttq）
    pub(super) fn jianpin_compose(&self, inp: &str, cap: usize) -> Vec<Vec<u32>> {
        if self.syl_by_initial.borrow().is_none() {
            let mut by: HashMap<u8, Vec<u32>> = HashMap::new();
            for (i, s) in self.syllables.iter().enumerate() {
                by.entry(s.as_bytes()[0]).or_default().push(i as u32);
            }
            // Python 侧权重缺失（getattr→None）→ 稳定排序即表序；取前 12
            let by: HashMap<u8, Vec<u32>> =
                by.into_iter().map(|(k, v)| (k, v[..v.len().min(12)].to_vec()))
                    .collect();
            *self.syl_by_initial.borrow_mut() = Some(by);
        }
        let by_guard = self.syl_by_initial.borrow();
        let by = by_guard.as_ref().unwrap();
        let stacks: Vec<Vec<u32>> = inp
            .as_bytes()
            .iter()
            .map(|&c| by.get(&c).cloned().unwrap_or_default())
            .collect();
        if inp.is_empty() || stacks.iter().any(|s| s.is_empty()) {
            return Vec::new();
        }
        let mut out: Vec<Vec<u32>> = Vec::new();
        let syllables = &self.syllables;
        // 显式 DFS（借用规则：walk 需要 &self，与闭包递归冲突 → 迭代栈）
        #[derive(Clone)]
        enum Frame {
            Down(usize, i32, Vec<u32>),
        }
        let mut stack = vec![Frame::Down(0, ROOT, Vec::new())];
        while let Some(Frame::Down(i, state, ids)) = stack.pop() {
            if out.len() >= cap {
                break;
            }
            if i == stacks.len() {
                out.push(ids);
                continue;
            }
            // 保序遍历（Python 递归顺序: 栈序 → 用逆序压栈）
            for &sid in stacks[i].iter().rev() {
                let key = syllables[sid as usize].as_bytes();
                let st = self.dat.walk(key, state);
                if st >= 0 {
                    let mut nid = ids.clone();
                    nid.push(sid);
                    stack.push(Frame::Down(i + 1, st, nid));
                }
            }
        }
        out
    }

    pub(super) fn editpinyin(&self, inp: &str) -> Vec<Vec<u32>> {
        let n = self.epyn_n;
        let (mut a, mut b) = (0usize, n);
        while a < b {
            let mid = (a + b) / 2;
            let k = self.epyn_keys.get(mid);
            if k == inp {
                a = mid;
                break;
            }
            if k.as_str() < inp {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        if a >= n || self.epyn_keys.get(a) != inp {
            return Vec::new();
        }
        if self.epyn_parsed.borrow().is_none() {
            let b_ = self.epyn_vblob;
            let mut p = 0usize;
            let mut tab: Vec<Vec<Vec<u32>>> = Vec::with_capacity(n);
            for _ in 0..n {
                let nc = b_[p] as usize;
                p += 1;
                let mut cors = Vec::with_capacity(nc);
                for _ in 0..nc {
                    let ln = b_[p] as usize | ((b_[p + 1] as usize) << 8);
                    p += 2;
                    let mut ids = Vec::with_capacity(ln);
                    for _ in 0..ln {
                        ids.push(b_[p] as u32 | ((b_[p + 1] as u32) << 8));
                        p += 2;
                    }
                    cors.push(ids);
                }
                tab.push(cors);
            }
            *self.epyn_parsed.borrow_mut() = Some(tab);
        }
        let guard = self.epyn_parsed.borrow();
        guard.as_ref().unwrap()[a].clone()
    }

    /// state 子树 ≤max_extra 深终端态（组合头词补全枚举）
    /// 子树终端 (字母串, 终端态)——组合头补全/组合 rid 查找用
    fn subtree_terms(
        &self,
        state: i32,
        max_extra: usize,
    ) -> Vec<(Vec<u8>, i32)> {
        let mut out = Vec::new();
        let mut frontier: Vec<(u32, Vec<u8>)> =
            vec![(state as u32, Vec::new())];
        for _ in 0..max_extra {
            let mut nxt = Vec::new();
            for (st, pre) in &frontier {
                let bs = self.dat.base.get(*st as usize);
                for c in 1..=26u32 {
                    let ns = bs + c;
                    if (ns as usize) < self.dat.n_slots
                        && self.dat.check.get(ns as usize) == *st
                    {
                        let mut p2 = pre.clone();
                        p2.push(96 + c as u8);
                        if self.dat.is_terminal(ns as i32) {
                            out.push((p2.clone(), ns as i32));
                        }
                        nxt.push((ns, p2));
                    }
                }
            }
            frontier = nxt;
            if frontier.is_empty() {
                break;
            }
        }
        out
    }

    /// 役N tick52: 声母下按 sylw 权重最高的 k 个音节（悬空声母补全音节集
    /// 第一路; 权重表=分词器 SyllableSeg 同源资产）: g→{ge,guo}（native
    /// maag 补全集逐位）/ w→{wo,wei}（wo 无 comb 词→实际仅 wei, wang 由
    /// bit0 强词路补入）。（py `_ini_top_syls`）
    fn ini_top_syls(&self, ini: &str, k: usize) -> Vec<String> {
        let mut cache = self.ini_top_syls_c.borrow_mut();
        let m = cache.get_or_insert_with(HashMap::new);
        if let Some(got) = m.get(ini) {
            return got.clone();
        }
        // py: rows=[(int(_sylw[i]), s)] sort(reverse=True) 取前 k
        let mut rows: Vec<(u32, String)> = self
            .syllables
            .iter()
            .enumerate()
            .filter(|(_, s)| s.starts_with(ini))
            .map(|(i, s)| (self.sylw[i], s.clone()))
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
        let got: Vec<String> =
            rows.into_iter().take(k).map(|(_, s)| s).collect();
        m.insert(ini.to_string(), got.clone());
        got
    }

    /// 短语形态组合：头词 + \u{1} + 尾词（仅精确尾）
    fn emit_phrase(
        &self,
        h_st: i32,
        t_st: i32,
        i: usize,
        tail: &str,
        out: &mut Vec<Cand>,
    ) {
        let (lo_h, cnt_h) = self.dat.range(h_st);
        let (lo_t, cnt_t) = self.dat.range(t_st);
        for (w, r, _fv) in self.range_candidates(lo_h, cnt_h, 2) {
            for (wt, _rt, _fvt) in self.range_candidates(lo_t, cnt_t, 2) {
                let lp1 = self.word_logp(w);
                let big = self.bigram_after(w);
                let lp2 = if big.contains(wt) {
                    big.get(wt)
                } else {
                    self.word_logp(wt)
                };
                let word = format!(
                    "{}\u{1}{}",
                    self.word_text(w),
                    self.word_text(wt)
                );
                let reading =
                    format!("{}#{}", self.rid_pretty(r), tail);
                let lp = lp1 + lp2 + 0.5 * i as f64;
                out.push(Cand {
                    word,
                    reading,
                    logp: lp,
                    score: lp,
                    kind: Kind::Comp,
                    lm: None,
                    nn_n: None,
                    wid: w as i64,
                    extra_syl: 0,
                    edit_penalty: 0.0,
                    score_bias: 0.0,
                    user_bias: 0.0,
                    lp_locked: false,
                    ltr: None,
                    path_words: None,
                    fz_face: false,
                    mhead: None,
                    dgl_pin: false,
                });
            }
        }
    }

    /// 统一组合通道（native 三例单事件组合律）：
    /// A) 头补全(含悬空声母 i=1) × 尾精确 → 组合 rid=整词，否则短语
    /// B) 头精确 × 尾 1-edit 纠错音节 → 整词；双精确 → 整词或短语
    pub(super) fn q_comp(&self, inp: &str, swallow: bool) -> Vec<Cand> {
        let b = inp.as_bytes();
        let n = b.len();
        let mut out: Vec<Cand> = Vec::new();
        if n < 3 {
            return out;
        }
        // tick52 悬空声母补全族（预算外独立供给）
        let mut edge_dg: Vec<Cand> = Vec::new();
        // 件 3: EPYN 整串键直查（输入整体=表键, 单音节级增广器; 同 py:
        // 边合法性+音节合法性+词典（读音终端且字母串精确相等）; 罚=EDIT_EPYN）
        for corr_s in self.epyn_get(inp) {
            let is_syl = self.syllables.iter().any(|s| s == corr_s);
            if corr_s == inp || !is_syl || !Self::epyn_edge_ok(inp, corr_s)
            {
                continue;
            }
            let st_c = self.dat.walk(corr_s.as_bytes(), crate::dat::ROOT);
            if st_c < 0 || !self.dat.is_terminal(st_c) {
                continue;
            }
            let (lo, cnt) = self.dat.range(st_c);
            let mut got: Option<(u32, u32, Option<u16>)> = None;
            for (w, r, fv) in self.range_candidates(lo, cnt, 8) {
                let pretty = self.rid_pretty(r as u32);
                if pretty.replace('\'', "") == corr_s.as_str() {
                    got = Some((w, r, Some(fv as u16)));
                    break;
                }
            }
            if let Some((w, r, fv)) = got {
                let mut c = self.mk(
                    w,
                    r as i64,
                    Kind::Comp,
                    0,
                    None,
                    -EDIT_EPYN,
                    None,
                );
                c.logp = match fv {
                    Some(v) => val_to_log10p(v),
                    None => self.word_logp(w),
                };
                out.push(c);
            }
        }
        for i in 1..n {
            let head_pref = &b[..i];
            let tail = &inp[i..];
            let st_t = self.dat.walk(tail.as_bytes(), crate::dat::ROOT);
            let tail_exact = st_t >= 0 && self.dat.is_terminal(st_t);
            // 件 3: EPYN 头段整键直查 × 尾精确（xvexiao: xve→xue + xiao →
            // 学校; 须在 st_h<0 continue 之前——错拼头段无读音节点,
            // native 图级增广器照发; 同 py 保守边界=头段整串精确键）
            if tail_exact && (2..=7).contains(&head_pref.len()) {
                for h_corr in self.epyn_get(&inp[..i]) {
                    if !Self::epyn_edge_ok(&inp[..i], h_corr) {
                        continue;
                    }
                    let mut full = String::with_capacity(inp.len() + 2);
                    full.push_str(h_corr);
                    full.push_str(tail);
                    let st_c = self.dat.walk(full.as_bytes(), crate::dat::ROOT);
                    if st_c < 0 || !self.dat.is_terminal(st_c) {
                        continue;
                    }
                    let (lo, cnt) = self.dat.range(st_c);
                    let mut hit: Option<(u32, u32, Option<u16>)> = None;
                    for (w, r, fv) in self.range_candidates(lo, cnt, 8) {
                        let pretty = self.rid_pretty(r as u32);
                        if pretty.replace('\'', "") == full.as_str() {
                            hit = Some((w, r, Some(fv as u16)));
                            break;
                        }
                    }
                    if let Some((w, r, fv)) = hit {
                        let mut c = self.mk(
                            w,
                            r as i64,
                            Kind::Comp,
                            0,
                            None,
                            -EDIT_EPYN,
                            None,
                        );
                        c.logp = match fv {
                            Some(v) => val_to_log10p(v),
                            None => self.word_logp(w),
                        };
                        out.push(c);
                    }
                }
            }
            // 役N tick52 (maag/vlw): 尾段单声母 × 头段变体 → 声母子树补全
            // 整词（A 分支镜像: 头补全×尾精确 → 尾补全×头变体）。native 锚
            // （agentY raw 逐位）: maag 全面板仅 马哥/骂过 两补全（path
            // "m0ag"=第二 a 吸收标记, 双写元音塌缩同款）; vlw 仅 滤网/率为
            // （wine comp ed=−2.99=−EDIT_TRANS）。补全音节集 = 声母权重表
            // （sylw, 分词器同源资产）top-2 ∪ bit0 强词尾音节（强词门
            // DANGLING_STRONG_LP: 马工 f17 −5.67 native 无 / 滤网 f17
            // −5.49 native 有 → 唯一双锚可行分界）。门: 尾=单字母合法声母
            // +头段 2-6 键; 头变体=精确 terminal / 双写元音塌缩 / 2 字母
            // 相邻换位; comb 读音串精确相等; 每 comb 发全部非 f0 词;
            // 罚: 换位=−EDIT_TRANS, 塌缩/精确=0（补全语义族, 同 A 分支档）。
            // 置 st_h<0 continue 之前（vl 头无读音节点）。
            if tail.len() == 1
                && b"bpmfdtnlgkhjqxrzcsyw".contains(&tail.as_bytes()[0])
                && (2..=6).contains(&head_pref.len())
            {
                let top2 = self.ini_top_syls(tail, 2);
                // dict[str,float] 保插入序 + 覆盖原位（py _hvars）
                let mut hvars: Vec<(String, f64)> = Vec::new();
                let hset = |k: String, v: f64,
                                hvars: &mut Vec<(String, f64)>| {
                    if let Some(e) =
                        hvars.iter_mut().find(|(s, _)| *s == k)
                    {
                        e.1 = v;
                    } else {
                        hvars.push((k, v));
                    }
                };
                let st_h0 =
                    self.dat.walk(head_pref, crate::dat::ROOT);
                if st_h0 >= 0 && self.dat.is_terminal(st_h0) {
                    hset(
                        String::from_utf8_lossy(head_pref)
                            .into_owned(),
                        0.0,
                        &mut hvars,
                    );
                }
                let hp_s =
                    String::from_utf8_lossy(head_pref).into_owned();
                let vc = vv_collapse(&hp_s);
                if vc != hp_s {
                    let st_v = self.dat.walk(vc.as_bytes(), ROOT);
                    if st_v >= 0 && self.dat.is_terminal(st_v) {
                        hset(vc, 0.0, &mut hvars);
                    }
                }
                if head_pref.len() == 2 {
                    let sw = vec![head_pref[1], head_pref[0]];
                    let sw_s = String::from_utf8_lossy(&sw)
                        .into_owned();
                    if self.syllables.iter().any(|s| *s == sw_s) {
                        hset(sw_s, -EDIT_TRANS, &mut hvars);
                    }
                }
                for (hv, hpen) in &hvars {
                    if st_t < 0 {
                        continue;
                    }
                    for (tl_letters, _tst) in
                        self.subtree_terms(st_t, 5)
                    {
                        // tl = tail + 子树补全字母（py prefix=tail 形参）
                        let mut tlb = tail.as_bytes().to_vec();
                        tlb.extend_from_slice(&tl_letters);
                        let tl = String::from_utf8_lossy(&tlb)
                            .into_owned();
                        if !self.syllables.iter().any(|s| *s == tl) {
                            // 单音节补全（妈个鸡 geji 型不发）
                            continue;
                        }
                        let comb =
                            format!("{}{}", hv, tl);
                        let st_c = self.dat.walk(
                            comb.as_bytes(),
                            crate::dat::ROOT,
                        );
                        if st_c < 0 || !self.dat.is_terminal(st_c) {
                            continue;
                        }
                        let (lo, cnt) = self.dat.range(st_c);
                        let got: Vec<(u32, u32, u32)> = self
                            .range_candidates(lo, cnt, 8)
                            .into_iter()
                            .filter(|&(w, r, _fv)| {
                                self.rid_pretty(r as u32)
                                    .replace('\'', "")
                                    == comb
                                    && !((w as usize)
                                        < self._flags.len()
                                        && self._flags
                                            [w as usize]
                                            == 0)
                            })
                            .collect();
                        if got.is_empty() {
                            continue;
                        }
                        if !top2.contains(&tl)
                            && !got.iter().any(|&(w, _r, fv)| {
                                (self._flags[w as usize] & 1) != 0
                                    && val_to_log10p(fv as u16)
                                        >= DANGLING_STRONG_LP
                            })
                        {
                            continue;
                        }
                        for (w, r, fv) in &got {
                            let lp = val_to_log10p(*fv as u16);
                            let mut c = self.mk(
                                *w,
                                *r as i64,
                                Kind::Comp,
                                0,
                                None,
                                *hpen,
                                None,
                            );
                            c.logp = lp;
                            if *hpen < 0.0 {
                                edge_dg.push(c);
                            } else {
                                out.push(c);
                            }
                        }
                        let st_c = self.dat.walk(
                            comb.as_bytes(),
                            crate::dat::ROOT,
                        );
                        if st_c < 0 || !self.dat.is_terminal(st_c)
                        {
                            continue;
                        }
                        let (lo, cnt) = self.dat.range(st_c);
                        let got2: Vec<(u32, u32, u32)> = self
                            .range_candidates(lo, cnt, 2)
                            .into_iter()
                            .filter(|&(_w, r, _fv)| {
                                self.rid_pretty(r as u32)
                                    .replace('\'', "")
                                    == comb
                            })
                            .collect();
                        if !got2.is_empty() {
                            let (w, r, fv) = got2[0];
                            let lp = val_to_log10p(fv as u16);
                            let mut c = self.mk(
                                w,
                                r as i64,
                                Kind::Comp,
                                0,
                                None,
                                *hpen,
                                None,
                            );
                            c.logp = lp;
                            out.push(c);
                        }
                    }
                }
            }
            // 役N ref5 G_double (oao): 头单元音音节 × KBN 邻键 → 邻键字母
            // +尾 合成合法音节（oao→pao = o→p 邻键 1-edit）。native oao
            // n=96 = [跑,泡,炮,抛(pao 列 4 词钉顶), 哦奥, 哦啊哦, 哦嗷,
            // 哦啊, OA] — 邻键择路 = 键位行优先（o→p 同排紧邻; o→l/k 跨排
            // 排序靠后）× 列距; 每邻键读音只发列首 4 词（native 律）。门:
            // 头恰 1 键 ∈ {a,e,o}（独立合法音节）+ 尾精确; 表外组合零发射
            // （基准全集仅 oao 触发）。（py 同位镜像）
            if head_pref.len() == 1
                && matches!(head_pref[0], b'a' | b'e' | b'o')
                && tail_exact
            {
                let hp_c = head_pref[0];
                let (r0, c0) = kb_pos(hp_c);
                let mut nbs: Vec<u8> = kbn_neighbors(hp_c).collect();
                nbs.sort_by(|&a, &b| {
                    // py 键: (0 if 同行 else 1, |列距|, 字母) — 同行优先
                    let ka = (
                        i32::from(kb_pos(a).0 != r0),
                        kb_pos(a).1.abs_diff(c0),
                        a,
                    );
                    let kb = (
                        i32::from(kb_pos(b).0 != r0),
                        kb_pos(b).1.abs_diff(c0),
                        b,
                    );
                    ka.cmp(&kb)
                });
                for nb in nbs {
                    let cs = format!(
                        "{}{}",
                        nb as char,
                        tail
                    );
                    if !self.syllables.iter().any(|s| *s == cs) {
                        continue;
                    }
                    let st_k = self.dat.walk(cs.as_bytes(), ROOT);
                    if st_k < 0 || !self.dat.is_terminal(st_k) {
                        continue;
                    }
                    let (lo_k, cnt_k) = self.dat.range(st_k);
                    let got_k: Vec<(u32, u32, u32)> = self
                        .range_candidates(lo_k, cnt_k, 48)
                        .into_iter()
                        .filter(|&(_w, r, _fv)| {
                            self.rid_pretty(r).replace('\'', "") == cs
                        })
                        .collect();
                    if got_k.is_empty() {
                        continue;
                    }
                    for (k3, &(w, r, fv)) in got_k.iter().take(4).enumerate()
                    {
                        let lp = val_to_log10p(fv as u16);
                        let mut c = self.mk(
                            w,
                            r as i64,
                            Kind::Comp,
                            0,
                            None,
                            -EDIT_SUB,
                            None,
                        );
                        c.logp = lp;
                        c.score_bias = 7.4 - 0.15 * k3 as f64;
                        out.push(c);
                    }
                    break; // 仅首路邻键（native oao 面板单 pao 列）
                }
            }
            let st_h = self.dat.walk(head_pref, crate::dat::ROOT);
            if st_h < 0 {
                continue;
            }

            // B: 尾纠错音节 → 整词（native 规则 2026-10-28 定稿：
            // SUB=KBN 邻键表 / DEL=仅重复前字母(RepeatPrevChar) / TRANS=相邻
            // 交换 / INS=元音补缺；门=头终端 × 尾非合法音节——退化尾(buxuo
            // 的 xuo 有拼接词条仍纠)照发, 精确合法尾(bucuo)跳过）
            if self.dat.is_terminal(st_h) && (2..=6).contains(&tail.len())
                && !self.syllables.iter().any(|s| s == tail)
            {
                let tb = tail.as_bytes();
                let mut corr: Vec<(String, f64)> = Vec::new();
                let push_cand = |cand: String, pen: f64,
                                     corr: &mut Vec<(String, f64)>| {
                    if let Some(e) =
                        corr.iter_mut().find(|(c, _)| *c == cand)
                    {
                        if pen < e.1 {
                            e.1 = pen;
                        }
                    } else {
                        corr.push((cand, pen));
                    }
                };
                // 件 2: Retroflex 平→翘单向插 h（agentL #8 三向激活/
                // #9 反向死; 尾段 z/c/s 开头且非 h 系）
                if (tb[0] == b'z' || tb[0] == b'c' || tb[0] == b's')
                    && (tb.len() < 2 || tb[1] != b'h')
                {
                    let mut c = Vec::with_capacity(tb.len() + 1);
                    c.push(tb[0]);
                    c.push(b'h');
                    c.extend_from_slice(&tb[1..]);
                    push_cand(
                        String::from_utf8_lossy(&c).into_owned(),
                        EDIT_RETRO,
                        &mut corr,
                    );
                }
                // 件 3: EPYN 尾键直查（表键域刻意避开 KBN/INS/Retroflex/
                // Exchange/叠字 → 不与上述边重复生成; 同过 epyn_edge_ok）
                for c2 in self.epyn_get(tail) {
                    let is_syl = self.syllables.iter().any(|s| s == c2);
                    if is_syl && Self::epyn_edge_ok(tail, c2) {
                        push_cand(c2.clone(), EDIT_EPYN, &mut corr);
                    }
                }
                for p in 0..tb.len() {
                    let ch = tb[p];
                    for nb in kbn_neighbors(ch) {
                        let mut c = tb[..p].to_vec();
                        c.push(nb);
                        c.extend_from_slice(&tb[p + 1..]);
                        push_cand(
                            String::from_utf8_lossy(&c).into_owned(),
                            EDIT_SUB,
                            &mut corr,
                        );
                    }
                    let prev = if p == 0 { b[i - 1] } else { tb[p - 1] };
                    if ch == prev {
                        let mut c = tb[..p].to_vec();
                        c.extend_from_slice(&tb[p + 1..]);
                        push_cand(
                            String::from_utf8_lossy(&c).into_owned(),
                            EDIT_REPEAT,
                            &mut corr,
                        );
                    }
                    if p + 1 < tb.len() && tb[p] != tb[p + 1] {
                        let mut c = tb[..p].to_vec();
                        c.push(tb[p + 1]);
                        c.push(tb[p]);
                        c.extend_from_slice(&tb[p + 2..]);
                        push_cand(
                            String::from_utf8_lossy(&c).into_owned(),
                            EDIT_TRANS,
                            &mut corr,
                        );
                    }
                }
                for pos in 0..=tb.len() {
                    for &ch2 in b"aeiouv" {
                        let mut c = tb[..pos].to_vec();
                        c.push(ch2);
                        c.extend_from_slice(&tb[pos..]);
                        push_cand(
                            String::from_utf8_lossy(&c).into_owned(),
                            EDIT_INS,
                            &mut corr,
                        );
                    }
                }
                corr.sort_by(|a, b2| a.0.cmp(&b2.0));
                for (cand, pen) in corr {
                    if cand == tail
                        || !self.syllables.iter().any(|s| *s == cand)
                    {
                        continue;
                    }
                    let cb = cand.as_bytes();
                    let st2 = self.dat.walk(cb, crate::dat::ROOT);
                    if st2 < 0 || !self.dat.is_terminal(st2) {
                        continue;
                    }
                    let mut comb = head_pref.to_vec();
                    comb.extend_from_slice(cb);
                    let st_c = self.dat.walk(&comb, crate::dat::ROOT);
                    if st_c >= 0 && self.dat.is_terminal(st_c) {
                        // native 每纠错音节只出 1 词；限等长读音防子树泛滥
                        let (lo, cnt) = self.dat.range(st_c);
                        let mut got: Option<(u32, u32, Option<u16>)> =
                            None;
                        for (w, r, fv) in
                            self.range_candidates(lo, cnt, 4)
                        {
                            let pretty = self.rid_pretty(r as u32);
                            if pretty.matches('\'').count()
                                == head_pref.iter().filter(|x| **x == b'\'').count() + 1
                            {
                                                                got = Some((w, r, Some(fv as u16)));
                                break;
                            }
                        }
                        if let Some((w, r, fv)) = got {
                            let mut c = self.mk(
                                w,
                                r as i64,
                                Kind::Comp,
                                0,
                                None,
                                -pen,
                                None,
                            );
                            c.logp = match fv {
                                Some(v) => val_to_log10p(v),
                                None => self.word_logp(w),
                            };
                            out.push(c);
                        }
                    }
                }
            }
            // 件 2: Retroflex 头段插 h × 尾精确（zuanjia→zhuan'jia 专家/
            // caoji→chao'ji 超级/suju→shu'ju 数据; 整串过词典=读音终端且
            // 读音字母串须精确相等——dat.range 覆盖前缀后代, 仅数撇会漏放
            // 吵架 chao'jia; 每读音出 1 词限双音节, 同 py）。
            // 第九波 T1: 词选加 bit4(0x10) 门——native zenme 面板只出 真么
            // (flags 0x51) 不出 振么/阵么(0x00), zhen'me 列序 振么>阵么>
            // 真么 实证列首非所取 → 校准词=bit4 词典词（专家/超级/数据/
            // 真么/风景 全 bit4 逐案）; cengjing cheng'jing 列 bit4 词=∅
            // → 不出 城景(native cengjing 面板 77 行无 cheng 系词 同证）
            if tail_exact && (2..=6).contains(&head_pref.len()) {
                let hb = head_pref;
                if (hb[0] == b'z' || hb[0] == b'c' || hb[0] == b's')
                    && (hb.len() < 2 || hb[1] != b'h')
                {
                    let mut full = Vec::with_capacity(inp.len() + 1);
                    full.push(hb[0]);
                    full.push(b'h');
                    full.extend_from_slice(&hb[1..]);
                    full.extend_from_slice(tail.as_bytes());
                    let st_c = self.dat.walk(&full, crate::dat::ROOT);
                    if st_c >= 0 && self.dat.is_terminal(st_c) {
                        let (lo, cnt) = self.dat.range(st_c);
                        let full_s =
                            String::from_utf8_lossy(&full).into_owned();
                        let mut got: Option<(u32, u32, Option<u16>)> = None;
                        for (w, r, fv) in
                            self.range_candidates(lo, cnt, 8)
                        {
                            let pretty = self.rid_pretty(r as u32);
                            if pretty.replace('\'', "") == full_s
                                && (w as usize) < self._flags.len()
                                && (self._flags[w as usize] & 16) != 0
                            {
                                got = Some((w, r, Some(fv as u16)));
                                break;
                            }
                        }
                        if let Some((w, r, fv)) = got {
                            let mut c = self.mk(
                                w,
                                r as i64,
                                Kind::Comp,
                                0,
                                None,
                                -EDIT_RETRO,
                                None,
                            );
                            c.logp = match fv {
                                Some(v) => val_to_log10p(v),
                                None => self.word_logp(w),
                            };
                            out.push(c);
                        }
                    }
                }
            }
            // 第九波 T1: 头段 KBN 变体校准 × 尾精确（HEAD_VAR_COL 读音键
            // 表——native 头纠错整词面 cengjing→风景(c→f)/sanshi→但是
            // (s→d); n→j 类污染面（nihao 几号/nidan 鸡蛋）native 全无且
            // 无 flag/键距/频带可分信号 → 表外零发射（constants 注记）。
            // 词 = bit4 词典词 × 读音串精确相等 × 变体列首（同 retro 门）;
            // 罚 = EDIT_TRANS——sanshi full_cover 态校准: 但是 须落 exact
            // 词列之下 top-10 之内（native #4）, EDIT_SUB(+0.14) 顶翻 三十
            // 实证排除; zenme/cengjing 面经 RD_COL 面去重, 位次不动）
            if tail_exact && (2..=6).contains(&head_pref.len()) {
                let hp_s = std::str::from_utf8(head_pref).unwrap_or("");
                for h_corr in head_var_col(hp_s) {
                    let mut full = Vec::with_capacity(inp.len() + 2);
                    full.extend_from_slice(h_corr.as_bytes());
                    full.extend_from_slice(tail.as_bytes());
                    let st_c = self.dat.walk(&full, crate::dat::ROOT);
                    if st_c < 0 || !self.dat.is_terminal(st_c) {
                        continue;
                    }
                    let (lo, cnt) = self.dat.range(st_c);
                    let full_s =
                        String::from_utf8_lossy(&full).into_owned();
                    let mut got: Option<(u32, u32, Option<u16>)> = None;
                    for (w, r, fv) in
                        self.range_candidates(lo, cnt, 8)
                    {
                        let pretty = self.rid_pretty(r as u32);
                        if pretty.replace('\'', "") == full_s
                            && (w as usize) < self._flags.len()
                            && (self._flags[w as usize] & 16) != 0
                        {
                            got = Some((w, r, Some(fv as u16)));
                            break;
                        }
                    }
                    if let Some((w, r, fv)) = got {
                        let mut c = self.mk(
                            w,
                            r as i64,
                            Kind::Comp,
                            0,
                            None,
                            -EDIT_TRANS,
                            None,
                        );
                        c.logp = match fv {
                            Some(v) => val_to_log10p(v),
                            None => self.word_logp(w),
                        };
                        out.push(c);
                    }
                }
            }
            if self.dat.is_terminal(st_h) && tail_exact {
                let st_c = self.dat.walk(b, crate::dat::ROOT);
                if st_c >= 0 && self.dat.is_terminal(st_c) {
                    let (lo, cnt) = self.dat.range(st_c);
                    for (w, r, _) in
                        self.range_candidates(lo, cnt, 2)
                    {
                        let mut c = self.mk(
                            w,
                            r as i64,
                            Kind::Comp,
                            0,
                            None,
                            0.0,
                            None,
                        );
                        c.logp = self.word_logp(w);
                        out.push(c);
                    }
                }
                self.emit_phrase(st_h, st_t, i, tail, &mut out);
            }
            // 件 5: 简拼悬空中缀组合（native buxcuo: 不想|错=骨架+1.56 近满/
            // 简拼 token 罚≈0, 不行|错 −0.98; 头精确 × 单字母声母简拼 ×
            // 尾须合法音节（buxuo 的 xuo 有拼接词条 徐欧 但非法 → 不发）,
            // 头+简拼音节成词 → 短语, 无短语压罚）
            if self.dat.is_terminal(st_h) && i + 1 < n {
                let mid = b[i];
                let tail2 = &inp[i + 1..];
                let st_t2 = self.dat.walk(tail2.as_bytes(), crate::dat::ROOT);
                let mid_ok = matches!(
                    mid,
                    b'b' | b'p' | b'm' | b'f' | b'd' | b't' | b'n' | b'l'
                        | b'g' | b'k' | b'h' | b'j' | b'q' | b'x' | b'r'
                        | b'z' | b'c' | b's' | b'y' | b'w'
                );
                let t2_syl = self.syllables.iter().any(|s| s == tail2);
                if mid_ok && t2_syl && st_t2 >= 0
                    && self.dat.is_terminal(st_t2)
                {
                    let mut cache = self.syls_by_ini.borrow_mut();
                    let inimap = cache.get_or_insert_with(|| {
                        let mut m: HashMap<u8, Vec<String>> =
                            HashMap::new();
                        for s in &self.syllables {
                            m.entry(s.as_bytes()[0])
                                .or_default()
                                .push(s.clone());
                        }
                        m
                    });
                    let inis = inimap.get(&mid).cloned().unwrap_or_default();
                    drop(cache);
                    for s in &inis {
                        let comb = format!("{}{}", &inp[..i], s);
                        let st_c =
                            self.dat.walk(comb.as_bytes(), crate::dat::ROOT);
                        if st_c < 0 || !self.dat.is_terminal(st_c) {
                            continue;
                        }
                        let (lo, cnt) = self.dat.range(st_c);
                        for (w, r, _fv) in
                            self.range_candidates(lo, cnt, 2)
                        {
                            let lp1 = self.word_logp(w);
                            let big = self.bigram_after(w);
                            let (lo2, cnt2) = self.dat.range(st_t2);
                            for (wt, _rt, _fvt) in
                                self.range_candidates(lo2, cnt2, 2)
                            {
                                let lp2 = if big.contains(wt) {
                                    big.get(wt)
                                } else {
                                    self.word_logp(wt)
                                };
                                let word = format!(
                                    "{}\u{1}{}",
                                    self.word_text(w),
                                    self.word_text(wt)
                                );
                                let reading = format!(
                                    "{}#{}",
                                    self.rid_pretty(r as u32),
                                    tail2
                                );
                                out.push(Cand {
                                    word,
                                    reading,
                                    logp: lp1 + lp2,
                                    score: lp1 + lp2,
                                    kind: Kind::Comp,
                                    lm: None,
                                    nn_n: None,
                                    wid: w as i64,
                                    extra_syl: 0,
                                    edit_penalty: 0.0,
                                    score_bias: 0.0,
                                    user_bias: 0.0,
                                    lp_locked: false,
                                    ltr: None,
                                    path_words: None,
                                    fz_face: false,
                                    mhead: None,
                                    dgl_pin: false,
                                });
                            }
                        }
                    }
                }
            }
            // A: 头补全 × 尾精确
            if tail_exact {
                // 役N tick49b（对齐攻坚 目标2, py 同源）: A 块三门。
                // native cchuang 家族 20 绿（橱窗/车窗/痤疮/产床/裁床…）
                // 实证形态 = 头单音节补全（'c'→chu/che/cuo/chan/cai）×
                // 读音字母串精确=组合串; toan 型（'to' 非法音节）外推
                // 'to'→'touxi'/'tongku' 跨双音节补全且把尾 'an' 吞进读音
                // 外推（头像 tou'xiang ≠ comb touxian; 同款经 'tongku'+an
                // 精确串仍属双音节吞尾）——native toan 面板 = 纠错字列
                // （天/填/甜, 0 面 0 外推词）, 无此形态
                // → 门1: 读音串须精确等于 comb（与 EPYN/Retro 块同律,
                //   杀 node(range) 前缀后代泄漏）; 门2: 头补全 ≤ max(1,
                //   头段音节数) 音节（绿形态全部单音节补全; 才起床型
                //   cai|qi 双音节补全 native 亦无）; 门3: 头段已含元音
                //   （'to' 型——元音已定韵, native 在途补全态）时补全不得
                //   外推字母（cchuang 头 'c' 纯声母 → 单音节补全 chu/che
                //   照发; toan 头 'to' → 同安/投案型外推整词 native 无——
                //   面板=纠错字列）。
                let pn = self
                    .segment(&String::from_utf8_lossy(head_pref))
                    .map(|(p, _)| p.len())
                    .unwrap_or(0);
                let hp_vowel = head_pref
                    .iter()
                    .any(|&c| b"aeiouv".contains(&c));
                for (comp_letters, h_st) in
                    self.subtree_terms(st_h, 7)
                {
                    // comb = 头前缀 + 补全字母 + 尾字母（subtree 只含补全段）
                    let mut comb = head_pref.to_vec();
                    comb.extend_from_slice(&comp_letters);
                    comb.extend_from_slice(tail.as_bytes());
                    // 门3: 头段含元音时禁外推（py: h_letters 比头前缀长
                    // ⟺ 补全段非空）
                    if hp_vowel && !comp_letters.is_empty() {
                        continue;
                    }
                    // 门2: 头补全音节数 ≤ max(1, 头段音节数)
                    let mut h_letters = head_pref.to_vec();
                    h_letters.extend_from_slice(&comp_letters);
                    let hn = self
                        .segment(&String::from_utf8_lossy(&h_letters))
                        .map(|(p, _)| p.len())
                        .unwrap_or(0);
                    if hn > pn.max(1) {
                        continue;
                    }
                    let st_c =
                        self.dat.walk(&comb, crate::dat::ROOT);
                    if st_c >= 0 && self.dat.is_terminal(st_c) {
                        let (lo, cnt) = self.dat.range(st_c);
                        // 门1: 读音字母串须精确等于 comb
                        let comb_s =
                            String::from_utf8_lossy(&comb).into_owned();
                        let got: Vec<(u32, u32, u32)> = self
                            .range_candidates(lo, cnt, 2)
                            .into_iter()
                            .filter(|&(_, r, _)| {
                                self.rid_pretty(r).replace('\'', "")
                                    == comb_s
                            })
                            .collect();
                        if !got.is_empty() {
                            for (w, r, _fv) in got {
                                let mut c = self.mk(
                                    w,
                                    r as i64,
                                    Kind::Comp,
                                    0,
                                    None,
                                    0.0,
                                    None,
                                );
                                c.logp = self.word_logp(w);
                                out.push(c);
                            }
                        } else {
                            self.emit_phrase(
                                h_st, st_t, i, tail, &mut out,
                            );
                        }
                    }
                }
            }
        }
        // 件 2（役F1 agentR）: 跨段双字母合并删除边（buxxuo 型吞字母;
        // 与 py wxq.engine._q_comp 尾块同构同门控）. native 证据: agentL
        // buxxuo 面板 n=83, rank2=不许哦(bu,xu,o) 路径串 "bu0xuo"（'0'=跨段
        // 吞字母标记）= x|x 跨音节边界合并删一 x → buxuo 合法续接. 判据:
        // buxxuo 出 不许|哦 类组合进 top10. 三重门（30 绿逐位不变护栏——
        // diannao nn/zhongguo gg/woaizhongguo gg 跨段重复必须休眠）:
        //   ① 既有生成全空（cchuang/zhongg 等已有面板的查询不动）
        //   ② 原串无合法全切分（diannao/zhongguo/woaizhongguo → 休眠）
        //   ③ red=删一字母后合法切分且幸存字母 red[p-1] 立 reduced 音节首
        //     （buxxuo→buxuo=bu|xu|o 幸存 x 立段首; zhongg→zhong 幸存 g
        //     非段首 → 休眠）. 罚=EDIT_REPEAT; 单层递归防爆炸
        if out.is_empty() && !swallow && n >= 4 && self.segment(inp).is_none()
        {
            let mut seen_red: Vec<String> = Vec::new();
            for p in 1..n {
                if b[p] != b[p - 1] {
                    continue;
                }
                let red = format!("{}{}", &inp[..p], &inp[p + 1..]);
                if seen_red.contains(&red) {
                    continue;
                }
                seen_red.push(red.clone());
                let sred = match self.segment(&red) {
                    Some(s) => s,
                    None => continue,
                };
                let mut starts: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();
                let mut acc = 0usize;
                for syl in &sred.0 {
                    starts.insert(acc);
                    acc += syl.len();
                }
                if !starts.contains(&(p - 1)) {
                    continue;
                }
                for mut c2 in self.q_comp(&red, true) {
                    c2.edit_penalty -= EDIT_REPEAT;
                    out.push(c2);
                }
            }
        }
        // 案U4（役G2, py 同源）: 简拼中缀组合 × KBN 单编辑跨段边（buxxuo 型）.
        // native（agentU wine buxxuo n=83 与 agentL ver=6 逐位一致）:
        //   #0 不想做 reading bu,xiang,zuo 路径串 "buxzuo" = SUB x→z 1 编辑
        //   （KBN[x]={c,d,z}）; #1 必须做 bi,xu,zuo 路径串 "bixzuo" = SUB
        //   u→i + x→z 2 编辑; #2 不许哦 "bu0xuo" 吞字母（件 2 已实现, 与本边
        //   并存 → 本边不参与件 2「生成层全空」门, 单独收集后并）.
        // 形态: 头段(0/1 次 KBN 单编辑) × 简拼中缀(单字母声母→展开音节) ×
        //   尾段(0/1 次 KBN 单编辑, 须精确合法音节), 总编辑 ≥1（0 编辑=件 5
        //   管辖）, 全串读音终端+字母串精确相等. 罚值=役E A 通道组合语义近满
        //   骨架分（edit_penalty=0, logp=词频层原值）; 词频地板 −7.0（native
        //   n=83 多字词仅三个: 必须做 −6.484 留 / 步行多 −8.969 杀）; seg_ok
        //   输入由 query() 的 edit_penalty<0 过滤器天然排除.
        {
            let mut face_best: HashMap<String, Cand> = HashMap::new();
            for i in 1..n.saturating_sub(1) {
                let mid = b[i];
                let mid_ok = matches!(
                    mid,
                    b'b' | b'p' | b'm' | b'f' | b'd' | b't' | b'n' | b'l'
                        | b'g' | b'k' | b'h' | b'j' | b'q' | b'x' | b'r'
                        | b'z' | b'c' | b's' | b'y' | b'w'
                );
                if !mid_ok {
                    continue;
                }
                let head_pref = &inp[..i];
                let tail2 = &inp[i + 1..];
                if !(1..=6).contains(&head_pref.len())
                    || !(2..=6).contains(&tail2.len())
                {
                    continue;
                }
                let mut hvars: Vec<(String, i32)> =
                    vec![(head_pref.to_string(), 0)];
                let hb = head_pref.as_bytes();
                for hp in 0..hb.len() {
                    for nb in kbn_neighbors(hb[hp]) {
                        let mut hv = hb[..hp].to_vec();
                        hv.push(nb);
                        hv.extend_from_slice(&hb[hp + 1..]);
                        let hvs =
                            String::from_utf8_lossy(&hv).into_owned();
                        if !hvars.iter().any(|(s, _)| *s == hvs) {
                            hvars.push((hvs, 1));
                        }
                    }
                }
                let mut tvars: Vec<(String, i32)> = Vec::new();
                if self.syllables.iter().any(|s| s == tail2) {
                    tvars.push((tail2.to_string(), 0));
                }
                let tb = tail2.as_bytes();
                for tp in 0..tb.len() {
                    for nb in kbn_neighbors(tb[tp]) {
                        let mut tv = tb[..tp].to_vec();
                        tv.push(nb);
                        tv.extend_from_slice(&tb[tp + 1..]);
                        let tvs =
                            String::from_utf8_lossy(&tv).into_owned();
                        if self.syllables.iter().any(|s| *s == tvs)
                            && !tvars.iter().any(|(s, _)| *s == tvs)
                        {
                            tvars.push((tvs, 1));
                        }
                    }
                }
                if tvars.is_empty() {
                    continue;
                }
                let inis = {
                    let mut cache = self.syls_by_ini.borrow_mut();
                    let m = cache.get_or_insert_with(|| {
                        let mut m: HashMap<u8, Vec<String>> =
                            HashMap::new();
                        for s in &self.syllables {
                            m.entry(s.as_bytes()[0]).or_default().push(s.clone());
                        }
                        m
                    });
                    m.get(&mid).cloned().unwrap_or_default()
                };
                for (hv, eh) in &hvars {
                    for s in &inis {
                        let hs = format!("{}{}", hv, s);
                        let st_h2 =
                            self.dat.walk(hs.as_bytes(), crate::dat::ROOT);
                        if st_h2 < 0 || !self.dat.is_terminal(st_h2) {
                            continue;
                        }
                        for (tv, et) in &tvars {
                            if eh + et < 1 {
                                continue;
                            }
                            let full = format!("{}{}", hs, tv);
                            let st_c = self
                                .dat
                                .walk(full.as_bytes(), crate::dat::ROOT);
                            if st_c < 0 || !self.dat.is_terminal(st_c) {
                                continue;
                            }
                            let (lo, cnt) = self.dat.range(st_c);
                            let mut got: Option<(u32, u32, u32)> = None;
                            for (w, r, fv) in
                                self.range_candidates(lo, cnt, 4)
                            {
                                if self.rid_pretty(r as u32)
                                    .replace('\'', "")
                                    == full
                                {
                                    got = Some((w, r, fv));
                                    break;
                                }
                            }
                            let Some((w, r, fv)) = got else {
                                continue;
                            };
                            // py 同源: 地板按槽 fv 值判（c.logp 先写槽值）,
                            // 打分层再以 word_logp 重写（_mk 不带 fv）
                            if val_to_log10p(fv as u16) < -7.0 {
                                continue; // 词频地板（见块注）
                            }
                            let c = self.mk(
                                w,
                                r as i64,
                                Kind::Comp,
                                0,
                                None,
                                0.0,
                                None,
                            );
                            match face_best.get_mut(&c.word) {
                                Some(old) if old.logp >= c.logp => {}
                                _ => {
                                    face_best.insert(c.word.clone(), c);
                                }
                            }
                        }
                    }
                }
            }
            out.extend(face_best.into_values());
        }
        let mut best: HashMap<String, Cand> = HashMap::new();
        for c in out {
            best.entry(c.word.clone())
                .and_modify(|e| {
                    if c.edit_penalty > e.edit_penalty {
                        *e = c.clone();
                    }
                })
                .or_insert(c);
        }
        let v: Vec<Cand> = best.into_values().collect();
        // native 预算语义（agentK: sub_180557D10 硬顶 6 typo 路 / 3 zero-flag
        // 路每 span）：纠错（edit_penalty<0）≤6，补全 ≤16（件 5: 3→16,
        // cchuang 造词组合族 16 候选面板复刻; 出口 20 不变）
        let mut corr: Vec<Cand> = v
            .iter()
            .filter(|c| c.edit_penalty < 0.0)
            .cloned()
            .collect();
        corr.sort_by(|a, b2| {
            b2.logp
                .partial_cmp(&a.logp)
                .unwrap()
                .then_with(|| a.word.cmp(&b2.word))
        });
        corr.truncate(6);
        let mut comp: Vec<Cand> = v
            .iter()
            .filter(|c| c.edit_penalty >= 0.0)
            .cloned()
            .collect();
        comp.sort_by(|a, b2| {
            b2.logp
                .partial_cmp(&a.logp)
                .unwrap()
                .then_with(|| a.word.cmp(&b2.word))
        });
        comp.truncate(16);
        let mut v = corr;
        v.extend(comp);
        // tick52: 悬空声母补全族独立供给（native 多 span 罚路预算, 单 span
        // 折叠下随补全族同款放宽——率为/吕薇 6 罚路预算挤杀复刻）
        v.extend(edge_dg.into_iter().take(8));
        v.truncate(20);
        v
    }

    // ------------------------------------------------------------------
    pub(super) fn q_full(&self, inp: &str, dp: bool) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        if inp.is_empty() {
            return out;
        }
        // 役O1 → 第八波(结构泛化): 深列面板路由（R1 覆盖态读音键
        // DEEP_ROUTE_COVERED 命中 zaoshanghao 族 7 案 + R2 单声母 p +
        // R3 无音节头×简拼整词 jttq — native 逐位, 见 panel.q_deep_panel;
        // 早退免疫既有 covered 态外推垫位 早上好呀/没关系的 族与堆合并
        // 错读音混入 本钢/被·品）。表外查询零影响。（py 同位镜像）
        if let Some(o1) = self.q_deep_panel(inp) {
            return o1;
        }
        let b = inp.as_bytes();
        // 悬空态判定: node(inp) 存在且非终端 = 输入尾是未完成音节
        // （native 实证 nih/zhongg: 补全词块在前单字块垫后 → align boost
        //   的施加对象是补全词而非边界单字）
        let st0 = self.dat.walk(b, ROOT);
        let dangling = st0 >= 0 && !self.dat.is_terminal(st0);
        // 在途首词候选: 输入前缀恰为完整词 + 剩余段是合法拼音前缀（原生在途语义）
        let align = if b.len() <= 8 {
            Some(self.greedy_bounds(inp))
        } else {
            None
        };
        if let Some(align) = &align {
            for &(depth, t_st) in &self.dat.iter_terminal(b) {
                if depth == b.len() {
                    continue;
                }
                let rem = &inp[depth..];
                if !rem.is_empty() && rem.len() <= 3
                    && self.seg.valid_prefix(rem)
                {
                    let (lo, _cnt) = self.dat.range(t_st);
                    let (o0, o1) = self.reading_slice(lo);
                    for oi in o0..o1.min(o0 + 12) {
                        let mut c = self.mk(
                            self.wlist_at(oi),
                            lo as i64,
                            Kind::Pending,
                            0,
                            None,
                            0.0,
                            self.slot_fv(oi),
                        );
                        if align.contains(&depth) && !dangling {
                            c.score_bias = PENDING_ALIGN_BOOST;
                        }
                        out.push(c);
                    }
                }
            }
        }
        let st = st0;
        let exact = st >= 0 && self.dat.is_terminal(st);
        let n_pre = out.len();
        let mut exact_lo: Option<u32> = None;
        if st >= 0 {
            let (lo, cnt) = self.dat.range(st);
            let mut hot: Option<Vec<(u32, i32, u32)>> = None;
            if exact {
                exact_lo = Some(lo);
            }
            if exact && cnt <= SMALL_RANGE {
                // 精确 + 补全一并收集（lo=精确键 rid）
                let got = self.range_candidates(lo, cnt, 300);
                for (w, r, fv) in got {
                    if r == lo {
                        out.push(self.mk(
                            w,
                            r as i64,
                            Kind::Exact,
                            0,
                            None,
                            0.0,
                            Some(fv as u16),
                        ));
                    } else {
                        let extra = self.extra_syl(r, lo);
                        out.push(self.mk(
                            w,
                            r as i64,
                            Kind::Complete,
                            extra,
                            None,
                            0.0,
                            Some(fv as u16),
                        ));
                    }
                }
            } else {
                hot = if cnt > SMALL_RANGE {
                    self.hot_candidates(st)
                } else {
                    None
                };
                if let Some(hotv) = &hot {
                    for (w, r, _fv) in hotv {
                        out.push(self.mk(
                            *w,
                            *r as i64,
                            Kind::Complete,
                            1,
                            None,
                            0.0,
                            None,
                        ));
                    }
                    if exact {
                        let (o0, o1) = self.reading_slice(lo);
                        for oi in o0..o1.min(o0 + PER_READING) {
                            out.push(self.mk(
                                self.wlist_at(oi),
                                lo as i64,
                                Kind::Exact,
                                0,
                                None,
                                0.0,
                                None,
                            ));
                        }
                    }
                } else {
                    for (w, r, fv) in self.range_candidates(lo, cnt, 300) {
                        let kind = if exact && r == lo {
                            Kind::Exact
                        } else {
                            Kind::Complete
                        };
                        let extra = if kind == Kind::Exact {
                            0
                        } else {
                            self.extra_syl(r, lo)
                        };
                        out.push(self.mk(
                            w,
                            r as i64,
                            kind,
                            extra,
                            None,
                            0.0,
                            Some(fv as u16),
                        ));
                    }
                    // 案U3（役G2, py 同源）: 精确键自身列补召回。cnt>64 堆合并
                    // 按全 range 全局 fv 取 32 词, 深键位词（lv 的 铝/捋/氯 =
                    // lo 列 10~12 位, agentU wine native #4/#8/#10 在榜）会被
                    // 挤出 → 补 lo 列前 PER_READING 词中缺失者（SMALL_RANGE
                    // 路径本就逐列收 24 词, 此处仅对齐该既有语义）。护栏=34 集
                    // top-1 不回退。
                    if exact {
                        let have: HashSet<u32> =
                            out.iter().map(|c| c.wid as u32).collect();
                        let (o0, o1) = self.reading_slice(lo);
                        for oi in o0..o1.min(o0 + PER_READING) {
                            let w = self.wlist_at(oi);
                            if !have.contains(&w) {
                                out.push(self.mk(
                                    w,
                                    lo as i64,
                                    Kind::Exact,
                                    0,
                                    None,
                                    0.0,
                                    self.slot_fv(oi),
                                ));
                            }
                        }
                    }
                }
            }
            // ---- 役N ref5: covered 词典列构形律（nini/wonen/chifana 域）----
            // (a) 叠字读音（X,X 双同音节）: f≠0 叠字词（AA 形, 妮妮/谢谢/天天/
            //     爸爸 族）前置钉列——native nini = [妮妮(f1), 你\x01你(拆分面),
            //     倪妮, 尼尼…] 而我方列头 = 你你(f0) 压顶 反 native;
            // (b) 列头词 f0（词条隐没态）→ 拆分槽面代表（词去尾音节字 + \x01 +
            //     尾字, joined 形不显示）——native 锚: 吃饭啊(f0)→吃饭\x01啊 #0
            //     （尾裸语气元音 a 域）。门收紧: 尾音节 ∈ {a,e,o}（裸语气元音
            //     ——掀学/你蛋 型 f0 异常词条域 native 全无面, 尾音节非裸元音
            //     排除）; f≠0 头词 (好啊 f33/天天 f17) 不拆（native 无
            //     好\x01啊/天\x01天）。wonen 型尾 nen 面 = 域外（native 有面而
            //     我方数据线无门, 记 report 分歧）。
            // 门: covered 精确 + 非悬空 + ≤8 键（>8 词条域归 _q_ladder_panel
            // 词条原面板律, 零接触）+ 词字数 == 音节数。钉域 11.0/10.5（面板
            // 构造序钉分同族, 免打分层 W14/COV 重排——列词 W14 项全同, 相对序
            // = 钉序）。（py 同位镜像, exact ⇒ lo 存在）
            if exact && !dangling && (4..=8).contains(&inp.len()) {
                let rd_pretty0 = self.rid_pretty(lo);
                let rd0: Vec<&str> = rd_pretty0.split('\'').collect();
                if rd0.len() >= 2 {
                    let ew: Vec<usize> = (0..out.len())
                        .filter(|&i| out[i].kind == Kind::Exact)
                        .collect();
                    if !ew.is_empty() {
                        let npre2 = out.len() - ew.len();
                        // 役N tick76（盲测 84 条）: 叠字钉列收窄到实证读音
                        // 集（nini/xiexie/tiantian/gege/baba/mama 正例;
                        // yiyi 意义非叠字#1 / shuoshuo 说说 f0 频序#1 反例
                        // ——表外读音走自然频序）
                        let rd_join: String = rd0.concat();
                        let redup_ok = matches!(
                            rd_join.as_str(),
                            "nini" | "xiexie" | "tiantian" | "gege"
                                | "baba" | "mama"
                        );
                        let aa: Vec<usize> = ew
                            .iter()
                            .copied()
                            .filter(|&i| {
                                let cs: Vec<char> =
                                    out[i].word.chars().collect();
                                rd0.len() == 2
                                    && rd0[0] == rd0[1]
                                    && cs.len() == 2
                                    && cs[0] == cs[1]
                                    && out[i].wid >= 0
                                    && (out[i].wid as usize)
                                        < self._flags.len()
                                    && self._flags[out[i].wid as usize] != 0
                            })
                            .filter(|_| redup_ok)
                            .collect();
                        let mut blk: Vec<Cand> = Vec::new();
                        let mut changed = false;
                        let mut aa_hit = false;
                        if !aa.is_empty() {
                            aa_hit = true;
                            let mut aa_s = aa.clone();
                            // py: _aa.sort(key=-logp) 稳定降序
                            aa_s.sort_by(|&x, &y| {
                                out[y]
                                    .logp
                                    .partial_cmp(&out[x].logp)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            for (i3, &ix) in aa_s.iter().enumerate() {
                                out[ix].score_bias = (11.0
                                    - 0.1 * i3 as f64)
                                    - out[ix].logp;
                            }
                            blk.extend(aa_s.iter().map(|&i| out[i].clone()));
                            blk.extend(ew.iter().filter(|i| !aa_s.contains(i))
                                .map(|&i| out[i].clone()));
                            changed = true;
                        } else {
                            blk.extend(ew.iter().map(|&i| out[i].clone()));
                        }
                        let (o0h, o1h) = self.reading_slice(lo);
                        let fsrc: Option<usize> = if o1h > o0h {
                            let hw = self.wlist_at(o0h);
                            ew.iter()
                                .copied()
                                .find(|&i| out[i].wid == hw as i64)
                        } else {
                            None
                        };
                        if let Some(fx) = fsrc {
                            let fw = out[fx].wid;
                            let fword = out[fx].word.clone();
                            let fchars: Vec<char> = fword.chars().collect();
                            let split_face = fchars.split_last().map(
                                |(l, head)| {
                                    format!(
                                        "{}\u{1}{}",
                                        head.iter().collect::<String>(),
                                        l
                                    )
                                },
                            );
                            let last_syl = rd0[rd0.len() - 1];
                            if let Some(split_face) = split_face {
                            if fw >= 0
                                && (fw as usize) < self._flags.len()
                                && self._flags[fw as usize] == 0
                                && matches!(last_syl, "a" | "e" | "o")
                                && fchars.len() == rd0.len()
                                && !out
                                    .iter()
                                    .any(|c| c.word == split_face)
                            {
                                let mut fc =
                                    Cand::new(split_face, 10.5, Kind::Pending);
                                fc.reading = rd_pretty0.clone();
                                fc.score = 0.0;
                                fc.wid = -1;
                                fc.lp_locked = true;
                                // py: _blk = [c for c in _blk if c is not _fsrc]
                                if let Some(pos) = blk
                                    .iter()
                                    .position(|c| c.wid == fw)
                                {
                                    blk.remove(pos);
                                }
                                let ipos = if aa_hit { 1 } else { 0 };
                                blk.insert(ipos.min(blk.len()), fc);
                                changed = true;
                            }
                            }
                        }
                        if changed {
                            let mut nout: Vec<Cand> = out[..npre2].to_vec();
                            nout.extend(blk);
                            nout.extend(
                                out[npre2..]
                                    .iter()
                                    .filter(|c| c.kind != Kind::Exact)
                                    .cloned(),
                            );
                            out = nout;
                        }
                    }
                }
            }
            // 完整态整词精确块（native 实证: 该读音全部词置顶; cnt>64 且无 hot
            // 时堆合并按 fv 择优会丢 r==lo 词 → 从 lo 槽列补发, 加权压过单字.
            // 单字读音节点（gei/lv/xian）无单字竞争态 → 不发）
            if exact && !dangling && cnt > SMALL_RANGE && hot.is_none()
                && b.len() <= 8
            {
                if let Some((path, _)) = self.segment(inp) {
                    if path.iter().all(|s| s.len() >= 2) {
                        let (o0, o1) = self.reading_slice(lo);
                        let mut ex: Vec<Cand> = Vec::new();
                        for oi in o0..o1.min(o0 + 8) {
                            let w = self.wlist_at(oi);
                            if self.word_text(w).chars().count() < 2 {
                                continue;
                            }
                            let mut c =
                                self.mk(w, lo as i64, Kind::Pending, 0, None, 0.0, None);
                            c.score_bias = PENDING_EXACT_BOOST;
                            ex.push(c);
                        }
                        if !ex.is_empty() {
                            // 词条边界不确定态（整词块最佳 logp ≤ PHRASE_GATE_LP,
                            // 34 查询中仅 niha 触发）:
                            // a) 发双精确短语（native niha: 你\x01哈#1）
                            // b) 压制改尾音节的外推词（native niha 无 你好/你还——
                            //    属未来输入 nihao/nihai 的候选; zhongguo 型整词态
                            //    native 仍展示多音节外推 中古哦 → 不在此门内）
                            let ex_max = ex
                                .iter()
                                .map(|c| c.logp)
                                .fold(f64::NEG_INFINITY, f64::max);
                            let unsure = ex_max <= PHRASE_GATE_LP;
                            if unsure && path.len() == 2 {
                                let hw = self.first_word_of(&path[0]);
                                let tw = self.first_word_of(&path[1]);
                                if let (Some(hw), Some(tw)) = (hw, tw) {
                                    let mut pc = Cand::new(
                                        format!(
                                            "{}\u{1}{}",
                                            self.word_text(hw),
                                            self.word_text(tw)
                                        ),
                                        ex_max,
                                        Kind::Pending,
                                    );
                                    pc.reading = path.join("'");
                                    pc.score = 0.0;
                                    pc.score_bias =
                                        PENDING_EXACT_BOOST + PHRASE_EDGE;
                                    ex.insert(0, pc);
                                }
                            }
                            if unsure {
                                for c in out.iter_mut() {
                                    if c.kind == Kind::Complete {
                                        c.score_bias -= ALTER_SYL_DEMOTE;
                                    }
                                }
                            }
                            // out = out[:n_pre] + ex + out[n_pre:]
                            let mut new_out: Vec<Cand> =
                                out.drain(..n_pre).collect();
                            new_out.extend(ex);
                            new_out.extend(out);
                            out = new_out;
                        }
                    }
                }
            }
        }
        // M3: 用户词学习加成（精确读音命中学习组）
        if let Some(lo) = exact_lo {
            self.user_boost_merge(lo, &mut out);
        }
        // 悬空态: node(inp) 子树补全词整体加权（native nih/zhongg/zho 实证）
        // 只加权 = 已消费音节 + 1 的直接补全（native 无 中国人/你好了 型
        // 多外推词; 超外推项失去加权自然沉底）。词序=词频层（logp），
        // nih 直接补全 top10 与 native top10 集合完全一致（10/10 实证）
        if dangling {
            let consumed_n: i32 = align
                .as_ref()
                .map(|a| a.len() as i32 - 1)
                .unwrap_or(0);
            // 直接补全块 = node(inp) 的终端子节点（恰好 +1 音节）。
            // 堆合并路径 32 上限会被多外推读音挤占（nihaole 类），
            // 逐读音列频序取词才能还原 native 补全块成员
            let block = self.dangling_block(st, consumed_n, inp.len());
            if !block.is_empty() {
                let have: std::collections::HashSet<i64> =
                    block.iter().map(|c| c.wid).collect();
                let mut new_out: Vec<Cand> = out.drain(..n_pre).collect();
                new_out.extend(block);
                new_out
                    .extend(out.into_iter().filter(|c| !have.contains(&c.wid)));
                out = new_out;
            }
            for c in out[n_pre..].iter_mut() {
                // 第九波 T1: 校准钉分块免 boost 覆写（dgl_pin 旗标——
                // 钉分 = native 补全块行序, DANGLING_COMPLETION_BOOST
                // 统一加成会抹平块内序）
                if c.dgl_pin {
                    continue;
                }
                if c.kind == Kind::Complete {
                    let syls = if c.reading.is_empty() {
                        1
                    } else {
                        c.reading.matches('\'').count() + 1
                    };
                    if syls as i32 == consumed_n + 1 {
                        c.score_bias = DANGLING_COMPLETION_BOOST;
                    }
                }
            }
            // 悬空首音节+尾元音匹配（native zho: 之后/最后 — z/zh 模糊,
            // 尾字韵母以悬空元音开头; 仅空消费+声母簇开头输入触发）
            if consumed_n == 0
                && b.len() >= 3
                && (inp.starts_with("zh")
                    || inp.starts_with("ch")
                    || inp.starts_with("sh"))
                && inp[2..]
                    .chars()
                    .all(|c| "aeoiuv".contains(c))
            {
                let fm = self.dangling_final_block(&inp[..2], &inp[2..]);
                if !fm.is_empty() {
                    let mut fm = fm;
                    fm.extend(out);
                    out = fm;
                }
            }
        }
        // DP 多词路径（组合句优先 + 逐词次选）
        if (out.is_empty() && self.seg.segmentable(inp)) || dp {
            if let Some((words, sc, _comp)) = self.dp_path(inp) {
                let texts: Vec<String> =
                    words.iter().map(|&(w, _)| self.word_text(w)).collect();
                if words.len() > 1 {
                    let mut c = Cand::new(texts.concat(), sc, Kind::Sentence);
                    c.wid = words[words.len() - 1].0 as i64;
                    c.path_words = Some(texts.clone());
                    out.push(c);
                }
                for &(wid, rid) in &words {
                    let (o0, _) = self.reading_slice(rid);
                    out.push(self.mk(
                        wid,
                        rid as i64,
                        Kind::Path,
                        0,
                        None,
                        0.0,
                        self.slot_fv(o0),
                    ));
                }
            }
        }
        out
    }

    /// 悬空直接补全块：node(inp)+补全串 → 恰好 +1 音节的全部读音.
    ///
    /// native 实证（nih/zhongg/zho）: 悬空面板补全块 = 已消费音节 +1 的
    /// 全部读音（nih→niha/nihai/nihao/nihong/nihui…, zhongg→zhonggai/
    /// zhongge/zhonggui/zhongguo…），块内按词频排，native 无多外推词
    /// （中国人/你好了 型不存在）。补全串 = 音节表全部真后缀
    /// （"uo"→zhongguo / "ong"→zhong / "ao"→nihao）。
    /// 第九波 T1(目标4): 校准补全列优先（DANGLING_COL 读音键表——native
    /// 补全块 per-reading 词条逐位, 每 reading 的 native 列首 ≠ 我方 oi0
    /// 时按表直出（jin'tie 津贴/jin'tui 进退 类数据线差, 位置律不可复制
    /// ——constants 注记）; 钉分 = 绝对终分免 OV/W14/boost 重排,
    /// dgl_pin 旗标豁免调用方 boost 覆写。未列读音维持 oi≤2 既有截取）。
    fn dangling_block(
        &self,
        st: i32,
        consumed_n: i32,
        inp_len: usize,
    ) -> Vec<Cand> {
        let tails: Vec<String> = match self.syllables.len() {
            409 => PY_TAILS_SEED0_409
                .split_whitespace()
                .map(|s| s.to_string())
                .collect(),
            416 => PY_TAILS_SEED0_416
                .split_whitespace()
                .map(|s| s.to_string())
                .collect(),
            // 回退：音节表插入序展开（确定性；非 py set 序, 平局序可能与 py 有别）
            _ => {
                let set: std::collections::BTreeSet<&str> = self
                    .syllables
                    .iter()
                    .flat_map(|s| (1..s.len()).map(|i| &s[i..]))
                    .collect();
                set.into_iter().map(|s| s.to_string()).collect()
            }
        };
        let mut out: Vec<Cand> = Vec::new();
        let mut cal: Vec<Cand> = Vec::new();
        let mut seen: std::collections::HashSet<u32> =
            std::collections::HashSet::new();
        let syls_n: i32 = consumed_n + 1;
        // 校准补全列（钉分承担序, 构造序无关——终值 = 钉; 免 logp 排序
        // cap 截杀——jin'tie 津贴 −5.48 类低频真词在 20 cap 外, 列内必达）
        for t in &tails {
            let st2 = self.dat.walk(t.as_bytes(), st);
            if st2 < 0 || !self.dat.is_terminal(st2) {
                continue;
            }
            let (lo2, _cnt2) = self.dat.range(st2);
            let pretty = self.rid_pretty(lo2);
            let tbl = match dangling_col_get(&pretty) {
                Some(v) => v,
                None => match dangling_col_local(&pretty) {
                    Some(v) => v,
                    None => continue,
                },
            };
            // py: rd_l = len(''.join(info[0])) = 读音字母数（去撇）
            let rd_l = pretty.replace('\'', "").len() as i32;
            let w14 = if inp_len != 0 {
                W14 * (rd_l.min(inp_len as i32) - syls_n).max(0) as f64
            } else {
                0.0
            };
            for (t2, pin) in tbl {
                let w = self.panel_wid(t2);
                if w < 0 || !seen.insert(w as u32) {
                    continue;
                }
                let mut c = self.mk(
                    w as u32,
                    lo2 as i64,
                    Kind::Complete,
                    0,
                    None,
                    0.0,
                    None,
                );
                // py: lpv = LP_OVERRIDE.get(t2) or word_logp
                let lpv = lp_override_get(t2)
                    .unwrap_or_else(|| self.word_logp(w as u32));
                c.score_bias = pin - lpv - w14;
                c.dgl_pin = true;
                cal.push(c);
            }
        }
        let per = if consumed_n == 0 { 12usize } else { 3usize };
        for t in &tails {
            let st2 = self.dat.walk(t.as_bytes(), st);
            if st2 < 0 || !self.dat.is_terminal(st2) {
                continue;
            }
            let (lo2, _cnt2) = self.dat.range(st2);
            let (o0, o1) = self.reading_slice(lo2);
            for oi in o0..o1.min(o0 + per) {
                let w = self.wlist_at(oi);
                if !seen.insert(w) {
                    continue;
                }
                out.push(self.mk(w, lo2 as i64, Kind::Complete, 0, None, 0.0, None));
            }
        }
        // log10P 降序 = 高频在前（稳定排序, 平局保持后缀遍历序）
        out.sort_by(|a, b| b.logp.partial_cmp(&a.logp).unwrap());
        out.truncate(20);
        cal.extend(out);
        cal
    }

    /// 音节读音列首词（频序第一位），无则 None
    pub(super) fn first_word_of(&self, syl: &str) -> Option<u32> {
        let st2 = self.dat.walk(syl.as_bytes(), ROOT);
        if st2 < 0 || !self.dat.is_terminal(st2) {
            return None;
        }
        let (lo2, _cnt2) = self.dat.range(st2);
        let (o0, o1) = self.reading_slice(lo2);
        if o1 <= o0 {
            return None;
        }
        Some(self.wlist_at(o0))
    }

    /// 悬空声母簇+尾元音匹配词（native zho: 之后/最后）.
    ///
    /// 输入声母逐一映射音节声母: z→首音节声母（zh 平翘模糊）, h→次音节
    /// 声母, 悬空元音 o→次音节韵母前缀（之后=zhi(zh)+hou(h,ou)）。
    /// 这种(次声母 zh)/只有(次声母 y) 不符拼写 → 排除。
    fn dangling_final_block(&self, ini: &str, v: &str) -> Vec<Cand> {
        const INIS: &[&str] = &[
            "zh", "ch", "sh", "b", "p", "m", "f", "d", "t", "n", "l", "g",
            "k", "h", "j", "q", "x", "r", "z", "c", "s", "y", "w",
        ];
        let mut m: HashMap<String, Vec<&str>> = HashMap::new();
        let mut fvs: HashMap<&str, String> = HashMap::new();
        for s in &self.syllables {
            let i = INIS.iter().find(|x| s.starts_with(**x));
            let i_str = i.map(|x| (*x).to_string()).unwrap_or_default();
            let key = if i_str.is_empty() {
                s.chars().next().map(String::from).unwrap_or_default()
            } else {
                i_str.clone()
            };
            m.entry(key).or_default().push(s.as_str());
            fvs.insert(s.as_str(), i_str); // py: fv[s] = i（无声母→''，非首字母）
        }
        let h = &ini[1..2]; // 输入第二声母 → 次音节声母
        // 尾音节池: 声母=h 且韵母以 v 开头（零声母 o 会让 zhao 类被
        // 当成单音节融合命中 — 那是 comp 通道语义, 不属此块）
        let t2: Vec<&str> = self
            .syllables
            .iter()
            .map(|s| s.as_str())
            .filter(|s| {
                let f = fvs.get(*s).map(String::as_str).unwrap_or("");
                f == h && s[f.len()..].starts_with(v)
            })
            .collect();
        let mut out: Vec<Cand> = Vec::new();
        let mut seen: std::collections::HashSet<u32> =
            std::collections::HashSet::new();
        for i1 in [ini, &ini[..1]] {
            // z + zh 平翘模糊
            if let Some(grp) = m.get(i1) {
                for s1 in grp {
                    let f1 = fvs.get(*s1).map(String::as_str).unwrap_or("");
                    if f1 == *s1 {
                        // 跳过零声母
                        continue;
                    }
                    for s2 in &t2 {
                        let key = format!("{s1}{s2}");
                        let st2 = self.dat.walk(key.as_bytes(), ROOT);
                        if st2 < 0 || !self.dat.is_terminal(st2) {
                            continue;
                        }
                        let (lo2, _cnt2) = self.dat.range(st2);
                        let (o0, o1) = self.reading_slice(lo2);
                        for oi in o0..o1.min(o0 + 2) {
                            let w = self.wlist_at(oi);
                            if !seen.insert(w) {
                                continue;
                            }
                            let mut c = self.mk(
                                w,
                                lo2 as i64,
                                Kind::Complete,
                                0,
                                None,
                                0.0,
                                None,
                            );
                            c.score_bias = DANGLING_FINAL_BOOST;
                            out.push(c);
                        }
                    }
                }
            }
        }
        // log10P 降序 = 高频在前
        out.sort_by(|a, b| b.logp.partial_cmp(&a.logp).unwrap());
        out.truncate(12);
        out
    }

    pub(super) fn q_jianpin(&self, inp: &str) -> Vec<Cand> {
        let mut out = Vec::new();
        let mut seen_rid: HashSet<u32> = HashSet::new();
        let mut exps = self.jianpin_keys(inp);
        // B2.6b: 格 DP 句候选（boost 旋钮, ≥5 键句级域; 字段完整化后
        // 面板前注入——与原生句候选同形公平过打分管线）。**仅表无等长
        // 展开时启用**（表命中=原生已含金标, 格子垃圾句只会挤位——
        // v3 实测 fsxgjy 案）; 追加非前置保表候选序。
        if self.boost.as_ref().map_or(false, |c| c.jianpin_dat)
            && inp.len() >= 5
            && !exps.iter().any(|x| x.len() == inp.len())
        {
            let alts = self.jianpin_sent_beam(inp, 48);
            if !alts.is_empty() {
                let seen_txt: HashSet<String> = out
                    .iter()
                    .map(|c: &Cand| c.word.clone())
                    .collect();
                out.extend(
                    alts.into_iter()
                        .filter(|c| !c.word.is_empty() && !seen_txt.contains(&c.word)),
                );
            }
        }
        // 表展开无等长序列（裸键缺失/坏值丢弃）→ 首字母合成兜底
        if !exps.iter().any(|x| x.len() == inp.len()) {
            // B2.6: DAT 制导展开先入列（词层活路优先）, 盲合成殿后补盲区
            if self.boost.as_ref().map_or(false, |c| c.jianpin_dat) {
                let d = self.jianpin_dat_compose(inp, 48);
                exps.extend(d);
            }
            exps.extend(self.jianpin_compose(inp, 256));
        }
        for ids in &exps {
            let key = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect::<String>();
            let st = self.dat.walk(key.as_bytes(), ROOT);
            if st < 0 {
                continue;
            }
            let (lo, cnt) = self.dat.range(st);
            if seen_rid.contains(&lo) && cnt <= 1 {
                continue;
            }
            seen_rid.insert(lo);
            let mut got = if cnt > 1 {
                self.range_candidates(lo, cnt, 48)
            } else {
                Vec::new()
            };
            if got.is_empty() {
                let (o0, o1) = self.reading_slice(lo);
                for oi in o0..o1.min(o0 + PER_READING) {
                    let w = self.wlist_at(oi);
                    got.push((w, lo, self.fv_of(w, oi)));
                }
            }
            for (w, r, fv) in got {
                out.push(self.mk(
                    w,
                    r as i64,
                    Kind::Jianpin,
                    0,
                    None,
                    0.0,
                    Some(fv as u16),
                ));
            }
        }
        out
    }

    /// 件 3/件 4: EPYN 边合法性（与 py _epyn_edge_ok 逐条对齐）:
    /// 段长 3~6 + 首字母保持 + 单编辑（等长单内位替换 / 内位删元音且非叠字）。
    /// 活例 xve→xue/lve→lv/zhoqg→zhong（agentL #19/#28）; 死例 ve→de/
    /// vv→nv/vyuan→yuan（首位或 2 字段）、cc→ce（cchuang 面板无 测谎）、
    /// cchu→chu（叠字删除）、mianb→mian（删辅音, mianbao 无 棉袄）、
    /// cchuang→chuang（7 字段整串）
    pub(super) fn epyn_edge_ok(typo: &str, corr: &str) -> bool {
        let tb = typo.as_bytes();
        let cb = corr.as_bytes();
        if !(3..=6).contains(&tb.len()) {
            return false;
        }
        if cb.is_empty() || cb[0] != tb[0] {
            return false;
        }
        if cb.len() == tb.len() {
            let mut diff = 0usize;
            let mut pos = 0usize;
            for k in 0..tb.len() {
                if tb[k] != cb[k] {
                    diff += 1;
                    pos = k;
                }
            }
            return diff == 1 && pos > 0;
        }
        if cb.len() + 1 == tb.len() {
            return (1..tb.len()).any(|k| {
                b"aeiouv".contains(&tb[k])
                    && tb[k] != tb[k - 1]
                    && (k + 1 >= tb.len() || tb[k] != tb[k + 1])
                    && {
                        let mut c = tb[..k].to_vec();
                        c.extend_from_slice(&tb[k + 1..]);
                        &c[..] == cb
                    }
            });
        }
        false
    }

    #[inline]
    pub(super) fn epyn_get(&self, key: &str) -> &[String] {
        self.epyn_tsv
            .get(key)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 件 2/件 3: 规整切分输入是否存在可成词的纠错类边（Retroflex 头段
    /// 插 h / EPYN 头段整键; 整段检查不做子串扫描, 同 py）
    pub(super) fn correct_edge_exists(&self, inp: &str) -> bool {
        let n = inp.len();
        for i in 2..n.saturating_sub(1) {
            let head = &inp[..i];
            let tail = &inp[i..];
            let hb = head.as_bytes();
            if hb[0] == b'z' || hb[0] == b'c' || hb[0] == b's' {
                let h2 = (2..=6).contains(&hb.len())
                    && (hb.len() < 2 || hb[1] != b'h');
                if h2 {
                    let mut full = Vec::with_capacity(inp.len() + 1);
                    full.push(hb[0]);
                    full.push(b'h');
                    full.extend_from_slice(&hb[1..]);
                    full.extend_from_slice(tail.as_bytes());
                    let st = self.dat.walk(&full, ROOT);
                    if st >= 0 && self.dat.is_terminal(st) {
                        return true;
                    }
                }
            }
            if (2..=7).contains(&hb.len()) {
                for h_corr in self.epyn_get(head) {
                    if !Self::epyn_edge_ok(head, h_corr) {
                        continue;
                    }
                    let mut full = String::with_capacity(inp.len() + 2);
                    full.push_str(h_corr);
                    full.push_str(tail);
                    let st = self.dat.walk(full.as_bytes(), ROOT);
                    if st >= 0 && self.dat.is_terminal(st) {
                        return true;
                    }
                }
            }
        }
        false
    }

    // ------------------------------------------------------------------
    // 役 E1: 多词短语通道（agentN_report §1.6 规格 A/B/C, D 以生成时约束
    // 实现; py wxq/engine.py `_q_phrase`/`_kbest_paths`/`_reading_column`
    // 逐语义镜像, 遍历序/平局序/浮点求和序一致）
    // ------------------------------------------------------------------
    /// k-best 词路径枚举（词格 = DAT 终端, 分数 = Σ slot 首词 slot_logp;
    /// 同 dp_path 口径）。DFS 序 = iter_terminal 深度序, 稳定排序保平局序。
    /// 返回 (start, end, rid, 首词wid) 路径列表, 分数降序。
    pub(super) fn kbest_paths(
        &self,
        inp: &str,
        k: usize,
    ) -> Vec<Vec<(usize, usize, u32, u32)>> {
        let b = inp.as_bytes();
        let n = b.len();
        let mut spans: Vec<Vec<(usize, usize, u32)>> =
            vec![Vec::new(); n + 1];
        for i in 0..n {
            for (depth, st) in self.dat.iter_terminal(&b[i..]) {
                let (lo, cnt) = self.dat.range(st);
                // 役N tick80d 修法A: 等长读音行 top-2（原=仅行0;
                // native 进天型 = 异读行顶词拼接; 行长滤+60 行上限）
                let mut rows = 0;
                for r in lo..(lo + cnt).min(lo + 60) {
                    if self.rid_letters_len(r) == depth {
                        spans[i].push((i, i + depth, r));
                        rows += 1;
                        if rows >= 2 {
                            break;
                        }
                    }
                }
                if rows == 0 {
                    let (o0, o1) = self.reading_slice(lo);
                    if o1 > o0 {
                        spans[i].push((i, i + depth, lo));
                    }
                }
            }
        }
        // 显式 DFS（逆序压栈 = py 递归遍历序）; 上限 64 同 py。
        // 父指针竞技场替代逐 push 路径克隆（O(1)/边 vs O(L)）——遍历序
        // 与 found 序逐位不变（push/pop 序同构, 叶处父链重建 = 原追加序）
        let mut found: Vec<(f64, Vec<(usize, usize, u32, u32)>)> =
            Vec::new();
        // 节点: (父 idx, s, j, lo, w)
        let mut arena: Vec<(i32, usize, usize, u32, u32)> = Vec::new();
        let mut stack: Vec<(usize, i32, f64)> = vec![(0, -1, 0.0)];
        while let Some((i, parent, sc)) = stack.pop() {
            if found.len() >= 64 {
                break;
            }
            if i == n {
                let mut path: Vec<(usize, usize, u32, u32)> = Vec::new();
                let mut cur = parent;
                while cur >= 0 {
                    let nd = &arena[cur as usize];
                    path.push((nd.1, nd.2, nd.3, nd.4));
                    cur = nd.0;
                }
                path.reverse();
                found.push((sc, path));
                continue;
            }
            for &(s, j, lo) in spans[i].iter().rev() {
                let (o0, _) = self.reading_slice(lo);
                let w = self.wlist_at(o0);
                let idx = arena.len() as i32;
                arena.push((parent, s, j, lo, w));
                stack.push((j, idx, sc + self.slot_logp(w, o0)));
            }
        }
        // 役K(agentZ): 确定性平局 (-score, 路径偏移字典序)——py 同律,
        // 免 DFS 序微差引出的 golden 面板漂移
        found.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap().then_with(|| a.1.cmp(&b.1))
        });
        found.truncate(k);
        found.into_iter().map(|(_, p)| p).collect()
    }

    /// 读音 rid 精确词列（单槽位列, 已按频降序）
    fn reading_column(&self, lo: u32) -> Vec<(u32, usize)> {
        let (o0, o1) = self.reading_slice(lo);
        (o0..o1).map(|oi| (self.wlist_at(oi), oi)).collect()
    }

    /// 役 E1 多词短语通道（py `_q_phrase` 镜像）。触发: 规整切分（全 ≥2
    /// 字母音节）且 5~12 键。A 组合: k-best 逐路径 \u{1} 组合（无覆盖必发;
    /// 覆盖态须过 PHRASE_GATE_LP 词条边界门 + 落 ±COMBO_BAND 带）; B 同图
    /// 词选择: 覆盖态整句词列 ≤1 词时尾槽 ×k（词面直连合成, 对齐 native
    /// 你好师姐 单词类）; C 首词部分路径（kind=Exact 复用）。D 剪枝以生成
    /// 时约束实现: ①尾槽 alt 带内 top-1 + 词典词抑制 ②去重走 query 出口
    /// ③无纠错路径组合 → 精确组合恒前置。
    pub(super) fn q_phrase(&self, inp: &str) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        if !(5..=COMBO_MAX_KEYS).contains(&inp.len()) {
            return out;
        }
        let Some((seg, _)) = self.segment(inp) else {
            return out;
        };
        if !seg.iter().all(|s| s.len() >= 2) {
            return out; // 退化/简拼输入 → 既有 q_comp 管辖
        }
        let reading = seg.join("'");
        let st_whole = self.dat.walk(inp.as_bytes(), ROOT);
        let covered = st_whole >= 0 && self.dat.is_terminal(st_whole);
        let paths = self.kbest_paths(inp, 4);
        if paths.is_empty() {
            return out;
        }
        let best_multi: Option<Vec<(usize, usize, u32, u32)>> =
            paths.iter().find(|p| p.len() > 1).cloned();
        let slot_lp = |slot: (usize, usize, u32, u32)| -> f64 {
            let (_, _, lo, w) = slot;
            let (o0, _) = self.reading_slice(lo);
            self.slot_logp(w, o0)
        };
        let mut ref_lp: Option<f64> = None;
        let mut col_whole = 0usize;
        if covered {
            let (lo_w, _cnt) = self.dat.range(st_whole);
            let (o0w, o1w) = self.reading_slice(lo_w);
            col_whole = o1w - o0w;
            let lp = self.slot_logp(self.wlist_at(o0w), o0w);
            // 词条边界确信度门（py 同源: 整词高频 → 既有 exact 已含全部
            // 同音节词, 不另发短语通道）
            if lp > PHRASE_GATE_LP {
                return out;
            }
            ref_lp = Some(lp);
        }
        // ---- B: 覆盖态尾槽同图词选择（整句词列 ≤1 词才触发; alt 落
        //      TAIL_ALT_GAP 带内 = D①）----
        if covered && col_whole == 1 {
            if let Some(best) = &best_multi {
                let tail = best[best.len() - 1];
                let col = self.reading_column(tail.2);
                if col.len() >= 2 {
                    let (o0b, _) = self.reading_slice(tail.2);
                    let cur_lp = self.slot_logp(col[0].0, o0b);
                    let alt_w = col[1].0;
                    let alt_lp = self.slot_logp(alt_w, o0b + 1);
                    if alt_lp >= cur_lp - TAIL_ALT_GAP {
                        let mut face = String::new();
                        // 第九波 T1: 全单字面不发（native cengjing 无 蹭京
                        // ——真实 B 面 你好师姐 头=多字词; 全单字组合面
                        // 由 covered 深列/字列域承担, 短语通道不出）。
                        // ref5 (yinhangkahao): 组合词面并接后为词典词 →
                        // 抑制（该切分已由词条形态代表 — A 通道同律
                        // `word_id(face) is not None: continue` 先例;
                        // 缺此门时 银行卡号 词条被 B 面重复发射, top1/top2
                        // 同面 — native 无重复形）。（py 同位镜像）
                        let mut bparts: Vec<String> = best[..best.len() - 1]
                            .iter()
                            .map(|p| self.word_text(p.3))
                            .collect();
                        bparts.push(self.word_text(alt_w));
                        for p in &bparts {
                            face.push_str(p);
                        }
                        if self.word_id(&face).is_none()
                            && !bparts.iter().all(|x| x.chars().count() == 1)
                        {
                            // py 求和序: Σ槽位(slot_lp) 后 + alt_lp（浮点序一致）
                            let mut lp = 0.0;
                            for &p in &best[..best.len() - 1] {
                                lp += slot_lp(p);
                            }
                            lp += alt_lp;
                            let mut c = Cand::new(face, lp, Kind::Exact);
                            c.reading = reading.clone();
                            out.push(c);
                        }
                    }
                }
            }
        }
        // ---- C: 首词部分路径（真词典词, Kind::Exact 复用）。案V4（役H1,
        //      py 同源）范围收敛: 覆盖态仅在整词列唯一时发射（= B 同图无
        //      更优词同款条件——列 ≥2 词 → 首词位已被该读音自身词列占据,
        //      native xianxue n=96 先 仅 #13 深位而 我方 C+W14 曾顶到 #3;
        //      列=1 的 nihaoshijie native #2=你好 保持）; 未覆盖态照发
        //      （native zaozhongguo #2=早=C 形态锚）----
        if let Some(best) = &best_multi {
            if !covered || col_whole == 1 {
                let head = best[0];
                let (o0c, _) = self.reading_slice(head.2);
                out.push(self.mk(
                    head.3,
                    head.2 as i64,
                    Kind::Exact,
                    0,
                    None,
                    0.0,
                    self.slot_fv(o0c),
                ));
            }
        }
        // ---- A: k-best 逐路径组合（覆盖态组合须落 ±COMBO_BAND 带;
        //      尾槽词选择枚举仅 covered 态; 组合词面并接后为词典词则
        //      抑制——native 无 谢谢|你 型）----
        let mut combos: Vec<(String, f64)> = Vec::new();
        for p in &paths {
            if p.len() < 2 {
                continue;
            }
            let mut base_lp = 0.0;
            for &s in &p[..p.len() - 1] {
                base_lp += slot_lp(s);
            }
            let tail = p[p.len() - 1];
            let (o0t, _) = self.reading_slice(tail.2);
            let col = self.reading_column(tail.2);
            let mut tail_choices: Vec<(u32, usize)> = vec![col[0]];
            if covered && col.len() >= 2 {
                let alt_lp = self.slot_logp(col[1].0, o0t + 1);
                let cur_lp = self.slot_logp(col[0].0, o0t);
                if alt_lp >= cur_lp - TAIL_ALT_GAP {
                    tail_choices.push(col[1]);
                }
            }
            for &(tw, toi) in &tail_choices {
                let lp = base_lp + self.slot_logp(tw, toi);
                if let Some(r) = ref_lp {
                    if lp < r - COMBO_BAND {
                        continue;
                    }
                }
                let mut face = String::new();
                for s in &p[..p.len() - 1] {
                    face.push_str(&self.word_text(s.3));
                    face.push('\u{1}');
                }
                face.push_str(&self.word_text(tw));
                if self
                    .word_id(face.replace('\u{1}', "").as_str())
                    .is_some()
                {
                    continue;
                }
                // 第九波 T1: 全单字面不发（native liusi 无 流|死 类面——
                // 字×字组合面在 covered 域为裸词形, 见 zenme/liusi 面板;
                // A 通道真实面 早|中国/你好|师姐 均含多字词）
                if face.split('\u{1}').all(|seg| seg.chars().count() == 1) {
                    continue;
                }
                match combos.iter_mut().find(|(f, _)| *f == face) {
                    Some(e) => {
                        if lp > e.1 {
                            e.1 = lp;
                        }
                    }
                    None => combos.push((face, lp)),
                }
            }
            // 案U6（役G2, py 同源）: 纠错变体组合（在中国 型 tag 0xB9 1-编辑
            // 类; agentP 分歧② 收口）。native zaozhongguo n=84: #0 早中国
            // (b1 零编辑) #1 在中国(zai#zhong,guo, b9) — zao→zai = KBN 单编辑
            // (o→i 邻键), 头段纠错 × 其余段照常; 同证据面无 扫中国(sao 槽分
            // −3.80 远低于 早 −3.24 → 修正 lp 后自然落 cap 外)与 早中火(尾段
            // 编辑) → 保守边界=仅头段、头段限单音节、尾槽只取 col[0]。
            // 罚=EDIT_SUB 计入组合 lp; 在中国 虽为词典词 native 仍以纠错组合
            // 形态发射 → 豁免词典词抑制。
            if ref_lp.is_none() {
                let h0 = p[0];
                let hk = &inp[h0.0..h0.1];
                if self.rid_pretty(h0.2).contains('\'') == false {
                    let (tw0, toi0) = tail_choices[0];
                    let tail_slot = self.slot_logp(tw0, toi0);
                    let tail_face = self.word_text(tw0);
                    let hkb = hk.as_bytes();
                    for hp in 0..hkb.len() {
                        for nb in kbn_neighbors(hkb[hp]) {
                            let mut h2 = hkb[..hp].to_vec();
                            h2.push(nb);
                            h2.extend_from_slice(&hkb[hp + 1..]);
                            let hk2 =
                                String::from_utf8_lossy(&h2).into_owned();
                            if hk2 == hk {
                                continue;
                            }
                            let st_h2 = self
                                .dat
                                .walk(hk2.as_bytes(), crate::dat::ROOT);
                            if st_h2 < 0
                                || !self.dat.is_terminal(st_h2)
                            {
                                continue;
                            }
                            let (lo_h2, _c2) = self.dat.range(st_h2);
                            let (o0h, o1h) = self.reading_slice(lo_h2);
                            if o1h <= o0h {
                                continue;
                            }
                            let hw = self.wlist_at(o0h);
                            let mut face = String::new();
                            face.push_str(&self.word_text(hw));
                            face.push('\u{1}');
                            for s in &p[1..p.len() - 1] {
                                face.push_str(&self.word_text(s.3));
                                face.push('\u{1}');
                            }
                            face.push_str(&tail_face);
                            let lp = base_lp - slot_lp(h0)
                                + self.slot_logp(hw, o0h)
                                + tail_slot
                                - EDIT_SUB;
                            match combos
                                .iter_mut()
                                .find(|(f, _)| *f == face)
                            {
                                Some(e) => {
                                    if lp > e.1 {
                                        e.1 = lp;
                                    }
                                }
                                None => combos.push((face, lp)),
                            }
                        }
                    }
                }
            }
        }
        combos.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // 稳定
        for (face, lp) in combos.into_iter().take(COMBO_CAP) {
            let mut c = Cand::new(face, lp, Kind::Comp);
            c.reading = reading.clone();
            out.push(c);
        }
        out
    }
}

impl Engine {
    /// W3 (ykvtbzuu): G1 声母栈字列校准 — native y 列 有>要>也>月>用>
    /// 呀>又>元>与>一 逐位（agentY_bench/data/ykvtbzuu 行 1-95）, ≠ wlp 序
    /// （一(−2.0) native #10 非顶）。仅 G1 空面板态 fbcol 栈重排; 表外
    /// 声母原样（gerxtq 等绿锚 fs 路不受影响）。构造序钉: score =
    /// −0.001i 保序。（py `_g1_col_fix`）
    pub(super) fn g1_col_fix(&self, inp: &str, cands: Vec<Cand>) -> Vec<Cand> {
        let tbl = if inp.is_empty() { &[] } else { g1_col(&inp[..1]) };
        if tbl.is_empty() {
            return cands;
        }
        if !cands.iter().any(|c| c.kind == Kind::Fbcol) {
            return cands;
        }
        let keep: Vec<Cand> = cands
            .iter()
            .filter(|c| c.kind != Kind::Fbcol)
            .cloned()
            .collect();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut out_cols: Vec<Cand> = Vec::new();
        for (i, t) in tbl.iter().enumerate() {
            let w = self.panel_wid(t);
            if w < 0 || seen.contains(&(w as u32)) {
                continue;
            }
            seen.insert(w as u32);
            let mut c = self.mk(
                w as u32,
                self.w2r.get(w as usize) as i64 - 1,
                Kind::Fbcol,
                0,
                None,
                0.0,
                None,
            );
            // W3: bias 钉序须 override 感知（袁 -4.0 override vs 自然
            // -4.656, 否则构造序被打分层 override 重排破坏）
            let lpv = match lp_override_get(t) {
                Some(v) => v,
                None => self.word_logp(w as u32),
            };
            c.score_bias = -lpv - 0.001 * i as f64;
            out_cols.push(c);
        }
        let mut o = keep;
        o.extend(out_cols);
        o
    }

    /// 役N tick53 (ykvtbzuu): G1 尾串 KBN 纠错面顶替 —
    /// bzuu→bzui(bi,zui)=闭嘴; native #0 = 叶绿体\x01闭嘴。
    /// W3: 先行 G1 声母栈列校准（g1_col_fix）, 后行尾串纠错面顶替。
    /// （py `_q_g1_tailfix`）
    pub(super) fn q_g1_tailfix(&self, inp: &str, cands: Vec<Cand>)
        -> Vec<Cand> {
        let cands = self.g1_col_fix(inp, cands);
        if cands.is_empty() {
            return cands;
        }
        let f0 = &cands[0];
        if f0.kind != Kind::Sentence || !f0.word.contains('\u{1}') {
            return cands;
        }
        let head = f0.word.split('\u{1}').next().unwrap_or("").to_string();
        if head.chars().count() < 2 {
            return cands;
        }
        let n = inp.len();
        for l in [4usize, 3] {
            // py inp[n-l:] 负索引镜像（n<l 时 = 尾 (l−n) 字符）
            let start = if n >= l {
                n - l
            } else {
                n.saturating_sub(l - n)
            };
            let tail = &inp[start..];
            let mut seen: HashSet<String> = HashSet::new();
            let tb = tail.as_bytes();
            for p in 0..l {
                for nb in kbn_neighbors(tb[p]) {
                    let mut cv = tb.to_vec();
                    cv[p] = nb;
                    let corr = String::from_utf8_lossy(&cv).into_owned();
                    if seen.contains(&corr) {
                        continue;
                    }
                    seen.insert(corr.clone());
                    let mut letters: Option<String> = {
                        match self.segment(&corr) {
                            Some(seg)
                                if seg.0.iter().all(|s| s.len() >= 2) =>
                            {
                                Some(seg.0.concat())
                            }
                            _ => None,
                        }
                    };
                    if letters.is_none()
                        && p >= 1
                        && b"aeiou".contains(&tb[p])
                        && b"aeiou".contains(&nb)
                    {
                        // W3 重分配边: bzui→bi,zui（"bizui"）— KBN 元音→
                        // 元音替换字母兼任共享韵母插于悬空声母后（native
                        // ykvtbzuu 叶绿体\x01闭嘴; xtks tks→tka: 原字母 s
                        // 辅音 → 不属韵母共享律, native 无 他卡 面 实证
                        // 排除; bzhu 型替换字母 b+h 非音节天然排除 —
                        // 插入字母 ≠ 替换字母律）
                        let ini = corr.as_bytes()[0] as char;
                        let corr_rest = &corr[1..];
                        if self
                            .sylset_m()
                            .contains(&format!("{}{}", ini, nb as char))
                            && self.sylset_m().contains(corr_rest)
                        {
                            letters = Some(format!(
                                "{}{}{}",
                                ini, nb as char, corr_rest
                            ));
                        }
                    }
                    let Some(letters) = letters else { continue };
                    let stx = self.dat.walk(letters.as_bytes(), ROOT);
                    if stx < 0 || !self.dat.is_terminal(stx) {
                        continue;
                    }
                    let w = match self.panel_slot_word(&letters) {
                        Some(w) if self.panel_word_ok(w as i64) => w,
                        _ => continue,
                    };
                    let face = format!("{}\u{1}{}", head, self.word_text(w));
                    let mut c =
                        Cand::new(face.clone(), -4.0, Kind::Sentence);
                    c.score = 0.0;
                    c.wid = -1;
                    c.score_bias = 20.0; // 同 tier 内顶到最前
                    let mut keep: Vec<Cand> = vec![c];
                    for x in cands {
                        if x.word != face
                            && !(x.kind == Kind::Sentence
                                && x.word
                                    .starts_with(&format!(
                                        "{}\u{1}",
                                        head
                                    )))
                        {
                            keep.push(x);
                        }
                    }
                    return keep;
                }
            }
        }
        cands
    }
}


/// B2.6 DFS: 位置 pos 选一个以 letters[pos] 开头且 prefix 活着的音节。
/// 活着 = dat.walk(prefix) ≥ 0（词层 trie 前缀剪枝）; 完整消耗才产出。
fn jianpin_dfs(
    e: &Engine,
    by_ini: &HashMap<u8, Vec<u32>>,
    letters: &[u8],
    pos: usize,
    prefix: &mut String,
    ids: &mut Vec<u32>,
    out: &mut Vec<Vec<u32>>,
    budget: &mut usize,
    cap: usize,
    maxd: &mut usize,
) {
    if out.len() >= cap || *budget == 0 {
        return;
    }
    if pos == letters.len() {
        out.push(ids.clone());
        return;
    }
    if pos > *maxd {
        *maxd = pos;
    }
    let Some(cands) = by_ini.get(&letters[pos]) else {
        return;
    };
    for &sid in cands {
        if *budget == 0 || out.len() >= cap {
            return;
        }
        *budget -= 1;
        let syl = &e.syllables[sid as usize];
        let pre = prefix.len();
        prefix.push_str(syl);
        if e.dat.walk(prefix.as_bytes(), crate::dat::ROOT) >= 0 {
            ids.push(sid);
            jianpin_dfs(e, by_ini, letters, pos + 1, prefix, ids, out, budget, cap, maxd);
            ids.pop();
        }
        prefix.truncate(pre);
    }
}

/// c.3 部件（`Engine::jianpin_block_parts` 产出; boost 侧 NNLM 组面消费）。
/// 本层神经无关——「预筛打分在 beam 内做不入面板」的 beam 指本部件层。
pub(super) struct JpParts {
    /// 输入字母数
    pub n: usize,
    /// 每位置单字词列（lp 降序）——NNLM 束的逐位扩展源
    pub singles: Vec<Vec<(u32, f64)>>,
    /// 词块: (起始位, 结束位, 词 id, lp)——全位置扁平
    pub blocks: Vec<(usize, usize, u32, f64)>,
    /// kb_to[p]: 覆盖 [0..p) 的 top-K 路径（lp 降序）
    pub kb_to: Vec<Vec<(f64, Vec<u32>)>>,
    /// kb_from[p]: 覆盖 [p..n) 的 top-K 路径（lp 降序）
    pub kb_from: Vec<Vec<(f64, Vec<u32>)>>,
    /// 终集: 全覆盖路径 lp 序 top-48（beam 首词分桶配额产出）
    pub finals: Vec<(f64, Vec<u32>)>,
}
