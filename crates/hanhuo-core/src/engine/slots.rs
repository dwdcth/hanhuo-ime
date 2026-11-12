//! SlotsMixin — G2/G4 字槽组合族（役N tick53(agentY2): B_combo 律1/律2 短语投影;
//! py wxq/engine/slots.py 逐语义移植）.
use super::*;

impl Engine {
    // ------------------------------------------------------------------
    // 役N tick53(agentY2): B_combo 字槽组合族（律1/律2 短语投影）
    // native 面板形态（agentY_bench/data wine raw 逐位）:
    //   [头词 exact]×[尾 EXACT 音节字列]  gaoyahu=高压壶(融合)/高压#互/
    //     高压户(融合)/高压#胡 —— 壶互户胡 = hu 读音字列频序, 融合形(词典
    //     词)与短语形(# 槽界)同面并存;
    //   [头悬空/单字]×[尾词]            danchaomo=但#超模/单#超模,
    //     peinunu=陪#努努, laruichu=啦#退出(rui→tui KBN 罚路);
    //   多槽链                          diuhuangzha=丢#黄#炸(全字链)+
    //     丢光#炸/扎(fuzz 词头×尾列 2 深), juangedeo=间隔#的#哦/捐个#的#哦
    //     (词头×中字×尾字), miunaozai=某宝#在/牛奶#在(双替换 fuzz 词头);
    //   字列链                          首音节列尽 → 掉尾元音续列
    //     (gao→ga / diu→di / miu→mi / juan→ju / niao→ni / sao→sa, native
    //     gaoyahu 29 位=噶 ga 列 / diuhuangzha 8 位=第 di 列 实证)。
    // 触发域: 全 exact 切分（≥3 段, 单字母段限 aoe）+ 读非终端 + 4~11 键。
    // ------------------------------------------------------------------

    /// 音节表集合缓存（py `_sylset_m`）
    pub(super) fn sylset_m(&self) -> std::cell::Ref<'_, HashSet<String>> {
        // 重入安全: 已初始化时不再 borrow_mut（外层调用可能持有本 Ref）
        if self.sylset_m_v.borrow().is_none() {
            let mut g = self.sylset_m_v.borrow_mut();
            if g.is_none() {
                *g = Some(self.syllables.iter().cloned().collect());
            }
        }
        std::cell::Ref::map(self.sylset_m_v.borrow(), |g| {
            g.as_ref().unwrap()
        })
    }

    /// 贪婪最长全切分（≥3 段; 单字母段限 aoe 且在音节表）。非全 exact
    /// 或 <3 段 → None（回退旧链）。（py `_slot_canonical_parse`）
    pub(super) fn slot_canonical_parse(
        &self,
        inp: &str,
    ) -> Option<Vec<String>> {
        let syls = self.sylset_m();
        let b = inp.as_bytes();
        let n = b.len();
        let mut out: Vec<String> = Vec::new();
        let mut i = 0usize;
        while i < n {
            let mut hit: Option<String> = None;
            let hi = (n - i).min(6);
            if hi >= 2 {
                for l in (2..=hi).rev() {
                    let seg = &inp[i..i + l];
                    if syls.contains(seg) {
                        hit = Some(seg.to_string());
                        break;
                    }
                }
            }
            if hit.is_none() {
                let c = b[i] as char;
                if "aoe".contains(c) && syls.contains(&c.to_string()) {
                    hit = Some(c.to_string());
                }
            }
            let h = hit?;
            i += h.len();
            out.push(h);
        }
        if out.len() >= 3 {
            Some(out)
        } else {
            None
        }
    }

    /// 面槽字列: SLOT_COL 表（native 校准序+syn lp）优先; 否则读音列
    /// 单字（主读音过滤——和@hu/叶@xie/查@zha 型跨读音混入剔除, native
    /// 律1: 非本读音字不得进槽; 滤空回退不滤）。（py `_slot_char_col`）
    pub(super) fn slot_char_col(&self, rd: &str) -> Vec<(String, f64)> {
        {
            let mut cache = self.slot_col_v.borrow_mut();
            let m = cache.get_or_insert_with(HashMap::new);
            if let Some(hit) = m.get(rd) {
                return hit.clone();
            }
        }
        let mut col: Vec<(String, f64)> = Vec::new();
        if let Some(tbl) = slot_col_get(rd) {
            col = tbl
                .iter()
                .map(|&(t, lp)| (t.to_string(), lp))
                .collect();
        } else {
            let st = self.dat.walk(rd.as_bytes(), ROOT);
            if st >= 0 && self.dat.is_terminal(st) {
                let (lo, _cnt) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                for oi in o0..o1 {
                    let w = self.wlist_at(oi);
                    let t = self.word_text(w);
                    if t.chars().count() != 1 {
                        continue;
                    }
                    let prid = self.w2r.get(w as usize) as i64 - 1;
                    if prid >= 0 {
                        let pr = self
                            .rid_pretty(prid as u32)
                            .replace('\'', "");
                        if !pr.is_empty() && pr != rd {
                            continue;
                        }
                    }
                    col.push((t, self.slot_logp(w, oi)));
                }
                if col.is_empty() {
                    // 滤空回退: 全量单字
                    for oi in o0..o1 {
                        let w = self.wlist_at(oi);
                        let t = self.word_text(w);
                        if t.chars().count() == 1 {
                            col.push((t, self.slot_logp(w, oi)));
                        }
                    }
                }
            }
        }
        self.slot_col_v
            .borrow_mut()
            .get_or_insert_with(HashMap::new)
            .insert(rd.to_string(), col.clone());
        col
    }

    /// exact 读音词列 [(word, slot_lp, eff_lp)]（eff=LP_OVERRIDE 感知,
    /// eff 降序）。（py `_slot_word_cands`）
    pub(super) fn slot_word_cands(
        &self,
        letters: &str,
    ) -> Vec<(String, f64, f64)> {
        {
            let mut cache = self.slot_wc_v.borrow_mut();
            let m = cache.get_or_insert_with(HashMap::new);
            if let Some(hit) = m.get(letters) {
                return hit.clone();
            }
        }
        let mut out: Vec<(String, f64, f64)> = Vec::new();
        let st = self.dat.walk(letters.as_bytes(), ROOT);
        if st >= 0 && self.dat.is_terminal(st) {
            let (lo, _cnt) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            for oi in o0..o1.min(o0 + 24) {
                let w = self.wlist_at(oi);
                let t = self.word_text(w);
                let lp = self.slot_logp(w, oi);
                let eff = lp_override_get(&t).unwrap_or(lp);
                out.push((t, lp, eff));
            }
            out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        }
        self.slot_wc_v
            .borrow_mut()
            .get_or_insert_with(HashMap::new)
            .insert(letters.to_string(), out.clone());
        out
    }

    /// ≤2 处单替换表词枚举（每音节 ≤1 替, 总 ≤2; 键距 1 邻键: 辅音域
    /// COMBO_PEN_CONS/元音域 COMBO_PEN_VOWEL, 键距 ≥2 COMBO_PEN_FAR;
    /// 表外元音替换不发——native 槽组合域无证, 且 高一(gaoya→gaoyi a→i)
    /// 型垃圾面实证禁绝）。返回 [(word, eff_lp, pen)] 按 eff−pen 降序
    /// top-2。native 锚: 某宝(miu→mou+nao→bao)/牛奶(m→n+o→i)/尿素(d→s)/
    /// 退出(r→t)/丢光(h→g)/讲话(g→h)/间隔(u→i)。（py `_slot_fuzz_words`）
    pub(super) fn slot_fuzz_words(
        &self,
        _letters: &str,
        syls_l: &[String],
        first_syl_only: bool,
    ) -> Vec<(String, f64, f64)> {
        let syls_set = self.sylset_m();
        let mut per: Vec<Vec<(String, f64)>> = Vec::new();
        for s in syls_l {
            let mut vs: Vec<(String, f64)> = Vec::new();
            let sb = s.as_bytes();
            for k2 in 0..sb.len() {
                for nb in kbn_neighbors(sb[k2]) {
                    let mut v = s.as_bytes().to_vec();
                    v[k2] = nb;
                    let vv = String::from_utf8_lossy(&v).into_owned();
                    if syls_set.contains(&vv) {
                        let (p0, c0) = kb_pos(sb[k2]);
                        let (p1, c1) = kb_pos(nb);
                        let far = (p0 - p1).abs() + (c0 - c1).abs() >= 2;
                        let pen = if far {
                            COMBO_PEN_FAR
                        } else if b"aeiou".contains(&sb[k2]) {
                            COMBO_PEN_VOWEL
                        } else {
                            COMBO_PEN_CONS
                        };
                        vs.push((vv, pen));
                    }
                }
            }
            per.push(vs);
        }
        // found: 键=(word, round(pen,3)) 保插入序 + 携带原 pen, 值=eff（取更大;
        // py dict 语义 = 同键覆值保原位）
        let mut found: Vec<((String, i64), f64, f64)> = Vec::new();
        let collect = |l2: &str, pen: f64,
                           found: &mut Vec<((String, i64), f64, f64)>| {
            for (t, _lp, eff) in
                self.slot_word_cands(l2).into_iter().take(2)
            {
                if slot_fuzz_ban(&t) {
                    continue;
                }
                let key = (t, (pen * 1000.0).round() as i64);
                if let Some(e) =
                    found.iter_mut().find(|(k, _, _)| k.0 == key.0 && k.1 == key.1)
                {
                    if eff > e.1 {
                        e.1 = eff;
                    }
                } else {
                    found.push((key, eff, pen));
                }
            }
        };
        let nper = per.len();
        let lim = if first_syl_only { 1.min(nper) } else { nper };
        for i in 0..lim {
            let pre = &syls_l[..i];
            let post = &syls_l[i + 1..];
            for (v, pen) in &per[i] {
                let joined =
                    format!("{}{}{}", pre.concat(), v, post.concat());
                collect(&joined, *pen, &mut found);
            }
        }
        for i in 0..lim {
            for j in (i + 1)..lim {
                let pre = &syls_l[..i];
                let mid = &syls_l[i + 1..j];
                let post = &syls_l[j + 1..];
                for (v1, p1) in &per[i] {
                    for (v2, p2) in &per[j] {
                        let joined = format!(
                            "{}{}{}{}{}",
                            pre.concat(), v1, mid.concat(), v2,
                            post.concat()
                        );
                        collect(&joined, p1 + p2, &mut found);
                    }
                }
            }
        }
        let mut out: Vec<(String, f64, f64)> = found
            .into_iter()
            .map(|(k, eff, pen)| (k.0, eff, pen))
            .collect();
        out.sort_by(|a, b| {
            (b.1 - b.2).partial_cmp(&(a.1 - a.2)).unwrap()
        });
        out.truncate(2);
        out
    }

    /// 头词层（native 律1 词列块: gaoyahu=高压/高雅/高呀/搞呀/高亚/高崖,
    /// jiangguawu=酱瓜/酱骨/讲古 —— 读音前缀词列, 含异切前缀 jianggu）。
    /// kind=fbcol: phrase 通道 comp 面（跨读音 junk: 高压#和/扫#吧协——
    /// E2 域）触发 query.py has_comp 对 pending/exact 的 −3 压罚, 词层
    /// 需以 fbcol+等效 W14 bias 自行供给（fbcol 不入压罚域）。wid=−1 →
    /// 文本键去重, 与 pending 原件并存（原件被压罚沉底, topk 外不可见）。
    /// （py `_slot_word_tier`）
    pub(super) fn slot_word_tier(
        &self,
        _inp: &str,
        syls: &[String],
        out: &mut Vec<Cand>,
    ) {
        let letters = syls.concat();
        let n = letters.len();
        if n < 5 {
            return;
        }
        let n0 = out.len();
        let n_s0 = syls[0].len();
        let mut l = n - 2;
        while l > 3 {
            if l <= n_s0 {
                break; // 词层须越过首音节（native: niao 字符不进 niaoduhuan
                // 词层——首音节字列归 chain; jianggu 跨界异切词层保留实证）
            }
            let st = self.dat.walk(letters[..l].as_bytes(), ROOT);
            if st >= 0 && self.dat.is_terminal(st) {
                let (lo, _cnt) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                let nsyl = {
                    let p = self.rid_pretty(lo);
                    if p.is_empty() { 1 } else {
                        p.split('\'').count()
                    }
                };
                if nsyl < 2 {
                    l -= 1;
                    continue; // 词层=多音节词（首音节字列归 char chain 管辖）
                }
                let mut got = 0;
                for oi in o0..o1.min(o0 + 8) {
                    let w = self.wlist_at(oi);
                    let t = self.word_text(w);
                    let mut lp = match lp_override_get(&t) {
                        Some(v) => v,
                        None => self.slot_logp(w, oi),
                    };
                    let _ = &mut lp;
                    if lp < -8.5 {
                        continue;
                    }
                    let bias = 1.0 * (l - nsyl) as f64;
                    let mut c = Cand::new(t, lp, Kind::Fbcol);
                    c.reading = self.rid_pretty(lo);
                    c.score = lp + bias;
                    c.score_bias = bias;
                    out.push(c);
                    got += 1;
                    if got >= 6 || out.len() - n0 >= 8 {
                        break;
                    }
                }
                if out.len() - n0 >= 8 {
                    break;
                }
            }
            l -= 1;
        }
    }

    /// 字槽组合面（律1/律2）。拆分枚举 p=1..k−1: 头段(单字列 top-2 /
    /// 整词 exact top-1(≥−8.0 门, 沉底词断路——danchaomo 蛋炒|没 不发) /
    /// 全字链(头无词时, 仅当链面=最高内分面才发——diuhuangzha 丢#黄#炸
    /// 发, miunaozai 缪#挠#在 不发)) × 尾段(单字列 cap(ex,fz) / 多音节
    /// 词 exact+fuzz top-1 / 头整词时字槽拆解: mid top-1×尾 cap——
    /// juangedeo 捐个#的#哦 发, laruichu 啦#睿#出 不发)。（py
    /// `_q_slot_combos`）
    pub(super) fn q_slot_combos(
        &self,
        _inp: &str,
        syls: &[String],
    ) -> Vec<Cand> {
        let k = syls.len();
        // raw: (internal=lp−pen, lp, pen, text, is_chain)
        let mut raw: Vec<(f64, f64, f64, String, bool)> = Vec::new();
        for p in 1..k {
            let head_syls = &syls[..p];
            let tail_syls = &syls[p..];
            let t = k - p;
            // 头枚举: (kind: 0=ch/1=ex/2=fz/3=chain, text, lp, pen)
            let mut heads: Vec<(u8, String, f64, f64)> = Vec::new();
            let mut chain: Option<Vec<(String, f64)>> = None;
            if p == 1 {
                for (ch, lp) in
                    self.slot_char_col(&head_syls[0]).into_iter().take(2)
                {
                    heads.push((0, ch, lp, 0.0));
                }
            } else {
                let hl = head_syls.concat();
                let wc = self.slot_word_cands(&hl);
                if let Some(w0) = wc.first() {
                    if w0.2 >= SLOT_WORD_GATE {
                        heads.push((1, w0.0.clone(), w0.2, 0.0));
                    }
                }
                if heads.is_empty() {
                    // 头无过门整词 → 全字链
                    let mut ch: Vec<(String, f64)> = Vec::new();
                    let mut ok = true;
                    for s in head_syls {
                        let c = self.slot_char_col(s);
                        if c.is_empty() {
                            ok = false;
                            break;
                        }
                        ch.push(c[0].clone());
                    }
                    if ok {
                        chain = Some(ch);
                    }
                }
                // fuzz 头门: 头段有 exact 词列但全部 eff < 门 → 禁 fuzz
                // （danchaomo 蛋炒(沉)/单拆/燃潮/陪你 断路; miunaozai 无词
                // 列 → fuzz 照发）
                let wc_ok = match wc.first() {
                    Some(w0) => w0.2 >= SLOT_WORD_GATE,
                    None => false,
                };
                if wc.is_empty() || wc_ok {
                    for (t2, lp, pen) in
                        self.slot_fuzz_words(&hl, head_syls, false)
                    {
                        heads.push((2, t2, lp, pen));
                    }
                }
            }
            if let Some(ch) = &chain {
                heads.push((3, String::new(), 0.0, 0.0));
                let _ = ch;
            }
            if heads.is_empty() {
                continue;
            }
            let mut tails_ex: Vec<(String, f64)> = Vec::new();
            #[allow(unused_assignments)] // 初值在 t==1 支路整体重赋
            let mut tails_fz: Vec<(String, f64, f64)> = Vec::new();
            if t == 1 {
                let rd = &tail_syls[0];
                let (cex, cfz) = slot_col_cap(rd);
                let col = self.slot_char_col(rd);
                tails_ex = col.iter().take(cex).cloned().collect();
                tails_fz = col
                    .iter()
                    .take(cfz)
                    .map(|(s, lp)| (s.clone(), *lp, 0.0))
                    .collect();
            } else {
                let tl = tail_syls.concat();
                let tw = self.slot_word_cands(&tl);
                if let Some(w0) = tw.first() {
                    if w0.2 >= SLOT_WORD_GATE {
                        tails_ex = vec![(w0.0.clone(), w0.2)];
                    }
                }
                let fz = self
                    .slot_fuzz_words(&tl, tail_syls, true)
                    .into_iter()
                    .take(1);
                tails_fz = fz.collect();
                if tails_ex.is_empty()
                    && tails_fz.is_empty()
                    && heads.iter().any(|h| h.0 == 1)
                {
                    // 尾字槽拆解（头须整词 exact——native juangedeo 门）
                    let mut mids: Vec<(String, f64)> = Vec::new();
                    let mut ok = true;
                    for s in &tail_syls[..tail_syls.len() - 1] {
                        let c = self.slot_char_col(s);
                        if c.is_empty() {
                            ok = false;
                            break;
                        }
                        mids.push(c[0].clone());
                    }
                    let lc: Vec<(String, f64)> = self
                        .slot_char_col(&tail_syls[tail_syls.len() - 1])
                        .into_iter()
                        .take(1)
                        .collect();
                    if ok && !lc.is_empty() {
                        let mlp: f64 = mids.iter().map(|x| x.1).sum::<f64>()
                            + lc[0].1;
                        let mut segs: Vec<String> = mids
                            .iter()
                            .map(|x| x.0.clone())
                            .collect();
                        segs.push(lc[0].0.clone());
                        let mtext = segs.join("\u{1}");
                        tails_ex = vec![(mtext, mlp)];
                    }
                }
            }
            if tails_ex.is_empty() && tails_fz.is_empty() {
                continue;
            }
            for h in &heads {
                let kind = h.0;
                // tlist 元素 = (text, lp, pen)
                #[allow(unused_assignments)] // 各支路先重赋再读
                let mut tlist: Vec<(String, f64, f64)> = Vec::new();
                if kind == 3 {
                    tlist = tails_ex
                        .iter()
                        .take(1)
                        .map(|(s, lp)| (s.clone(), *lp, 0.0))
                        .collect();
                } else if t == 1 {
                    // 单音节字尾: ex/ch 头=exact 列(cap cex), fz 头=fz 列
                    tlist = if kind == 2 {
                        tails_fz.clone()
                    } else {
                        tails_ex
                            .iter()
                            .map(|(s, lp)| (s.clone(), *lp, 0.0))
                            .collect()
                    };
                } else {
                    // 统一尾列: exact 列 + fuzz 列（去重兜底）
                    tlist = tails_ex
                        .iter()
                        .map(|(s, lp)| (s.clone(), *lp, 0.0))
                        .collect();
                    tlist.extend(tails_fz.clone());
                }
                for (ttext, tlp, tpen) in tlist {
                    let (htext, hlp, hpen) = if kind == 3 {
                        let ch = chain.as_ref().unwrap();
                        (
                            ch.iter()
                                .map(|x| x.0.clone())
                                .collect::<Vec<_>>()
                                .join("\u{1}"),
                            ch.iter().map(|x| x.1).sum::<f64>(),
                            0.0,
                        )
                    } else {
                        (h.1.clone(), h.2, h.3)
                    };
                    let mut conj = "\u{1}".to_string();
                    if kind == 1
                        && !ttext.contains('\u{1}')
                        && slot_fuse(&format!("{}{}", htext, ttext))
                    {
                        conj = String::new(); // 融合形（native 词典词, 我方典缺）
                    }
                    let text = format!("{}{}{}", htext, conj, ttext);
                    let lp = hlp + tlp;
                    let pen = hpen + tpen;
                    raw.push((lp - pen, lp, pen, text, kind == 3));
                }
            }
        }
        if raw.is_empty() {
            return Vec::new();
        }
        raw.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap()
                .then_with(|| a.3.cmp(&b.3))
        });
        // 全字链门: 链面须=最高内分面且内分 ≤ CHAIN_GATE_LP（native
        // 丢#黄#炸 发 / 但#超#没·卷#个#得#哦·缪#挠#在 不发）
        if !raw[0].4 || raw[0].0 > CHAIN_GATE_LP {
            raw.retain(|r| !r.4);
        }
        let mut out: Vec<Cand> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for (_internal, lp, pen, text, _is_chain) in raw {
            if seen.contains(&text) {
                continue;
            }
            seen.insert(text.clone());
            // 役N W1: 槽面逐段 SLOT_FUZZ_BAN 检查——query/panel 层的禁表
            // 只查整串词面, 槽形面 讲\x01诖误/讲\x01华五 的 junk 段(诖误/
            // 华五)漏网（native jiangguawu: fuzz 头=讲话 单变体, 尾槽=
            // 字列 无/五, 无任何 尾词完成/次变体 面）
            if text.split('\u{1}').any(slot_fuzz_ban) {
                continue;
            }
            let mut c = Cand::new(text, lp, Kind::Fcomp);
            c.score = lp - pen;
            c.edit_penalty = -pen;
            c.score_bias = COMBO_BIAS;
            out.push(c);
            if out.len() >= 10 {
                break;
            }
        }
        out
    }

    /// 首音节字列链（native: gao 列 10 字 → ga 列 / diu→di / miu→mi /
    /// juan→ju / niao→ni / sao→sa; 掉尾元音续列, 深度共 10）。（py
    /// `_slot_col_chain`）
    pub(super) fn slot_col_chain(
        &self,
        syls: &[String],
        out: &mut Vec<Cand>,
    ) {
        let mut seen_w: HashSet<u32> = HashSet::new();
        let mut rd = syls[0].clone();
        let mut depth = 10usize;
        while !rd.is_empty() && depth > 0 {
            let n0 = out.len();
            self.fb_seg_columns(&rd, out, &mut seen_w, depth);
            depth -= out.len() - n0;
            let rb = rd.as_bytes();
            if b"aeiou".contains(&rb[rb.len() - 1])
                && rd.len() >= 3
                && self.sylset_m().contains(&rd[..rd.len() - 1])
            {
                rd = rd[..rd.len() - 1].to_string();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // 合成引擎（不依赖 wxq.bin）: 空 Pack（一切 raw → Err）+ 空 DAT
    // （n_slots=1 → walk 任一非空键恒 -1）+ 注入音节表。子模块对
    // engine::Engine 私有字段可见 → 字段字面构造（同 dat.rs 微型
    // DAT 手工构造惯例）。可观测面 = SLOT_COL 校准表 / 音节表 / 门与
    // 罚旋钮；DAT 供给域（词列/融合形）以"空表零发射"定界。
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

    fn ss(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// sylset_m = 音节表全集缓存（py `_sylset_m`; 双检重入安全）:
    /// 集合内容镜像 syllables, 二次调用走已初始化分支同集返回
    #[test]
    fn sylset_m_mirrors_syllable_table() {
        let e = synth(&["ni", "hao", "gao", "ga"]);
        let m = e.sylset_m();
        assert_eq!(m.len(), 4, "集合 = 音节表全集");
        assert!(m.contains("ni") && m.contains("ga"));
        drop(m);
        let m2 = e.sylset_m(); // 已初始化 → 不再 borrow_mut 的重入分支
        assert_eq!(m2.len(), 4, "缓存二次调用同集");
    }

    /// 贪婪最长全切分（py `_slot_canonical_parse`）: 逐位降长试探,
    /// dan 压 da / chao 压 cha（最长段优先律）
    #[test]
    fn slot_canonical_parse_greedy_longest() {
        let e = synth(&["da", "dan", "cha", "chao", "mo"]);
        // danchaomo = 但#超模 域: 首段取 dan 不取 da, 中段取 chao 不取 cha
        assert_eq!(
            e.slot_canonical_parse("danchaomo"),
            Some(ss(&["dan", "chao", "mo"])),
            "最长优先: [dan, chao, mo] 而非 da/cha 切"
        );
    }

    /// 触发域门: ≥3 段 / 全 exact（表外字母断路）/ 单字母段限 aoe
    /// 且须在音节表（役N tick53 触发域: 全 exact 切分 + 读非终端）
    #[test]
    fn slot_canonical_parse_domain_gates() {
        let e = synth(&["da", "dan", "cha", "mo", "bao", "a", "o"]);
        assert_eq!(
            e.slot_canonical_parse("dancha"),
            None,
            "2 段 < 3 段门 → 回退旧链"
        );
        assert_eq!(
            e.slot_canonical_parse("danchax"),
            None,
            "x 非表非 aoe → 非全 exact 断路"
        );
        assert_eq!(
            e.slot_canonical_parse("danchaom"),
            None,
            "m 非 aoe 单字母段拒收（辅音单字不立段）"
        );
        // aoe 单字母段放行（在表前提）: [bao, a, o] 过 3 段门
        assert_eq!(
            e.slot_canonical_parse("baoao"),
            Some(ss(&["bao", "a", "o"])),
            "aoe 单字母段可立段"
        );
        let e2 = synth(&["bao", "a"]);
        assert_eq!(
            e2.slot_canonical_parse("baoao"),
            None,
            "aoe 字母也须在音节表（o 不在表 → 断）"
        );
    }

    /// slot_char_col 律1: SLOT_COL 校准表优先（native 序+syn lp,
    /// gaoyahu 壶互户胡=hu 读音字列频序锚）; 表外读音空 DAT → 空列;
    /// _slot_col_v 缓存二次同值
    #[test]
    fn slot_char_col_slot_col_table_priority() {
        let e = synth(&["hu", "xx"]);
        assert_eq!(
            e.slot_char_col("hu"),
            vec![
                ("壶".to_string(), -3.50),
                ("互".to_string(), -3.80),
                ("户".to_string(), -4.10),
                ("胡".to_string(), -4.40),
            ],
            "hu 列 = native 校准序逐位"
        );
        assert!(
            e.slot_char_col("xx").is_empty(),
            "表外读音 DAT 未命中 → 空列"
        );
        assert_eq!(e.slot_char_col("hu").len(), 4, "缓存路径二次同值");
    }

    /// slot_word_cands / slot_fuzz_words 均由 DAT 词列供给: 空 DAT
    /// 零发射不 panic（典外 fuzz 面禁绝域——高一(gaoyi) 型垃圾面同理）
    #[test]
    fn slot_word_cands_and_fuzz_need_dat() {
        let e = synth(&["dan", "pei", "hu"]);
        assert!(
            e.slot_word_cands("danpei").is_empty(),
            "exact 词列 = DAT 终端读音行, 空表零列"
        );
        assert!(
            e.slot_fuzz_words(
                "danpei",
                &ss(&["dan", "pei"]),
                false
            )
            .is_empty(),
            "fuzz 枚举 collect 依赖词列命中, 变体域外不发"
        );
    }

    /// q_slot_combos 律1 主面: 头单字列 top-2 × 尾字列 cap(hu 4,4),
    /// 槽界 \u{1}; 终值 = lp − pen, score_bias = COMBO_BIAS,
    /// Kind=Fcomp（danchaomo 但|超模 +0.44 校准锚族）
    #[test]
    fn q_slot_combos_char_head_x_char_tail() {
        let e = synth(&["dan", "hu"]);
        let out = e.q_slot_combos("danhu", &ss(&["dan", "hu"]));
        assert_eq!(out.len(), 8, "头 top-2(但/单) × 尾 cap(4,4) = 8 面");
        let c0 = &out[0];
        assert_eq!(c0.word, "但\u{1}壶", "最高内分面 = 头首字 × 尾首字");
        assert!(
            (c0.score - (-2.90 - 3.50)).abs() < 1e-9,
            "score = 但(-2.90) + 壶(-3.50)"
        );
        assert_eq!(
            c0.score_bias, COMBO_BIAS,
            "组合面 bias = 9.0（但|超模 压 蛋炒 校准锚）"
        );
        assert_eq!(c0.edit_penalty, 0.0, "ch 头零编辑罚");
        assert!(matches!(c0.kind, Kind::Fcomp), "槽组合面 Kind=Fcomp");
        // 次内分档 -6.70 平局对（但|互 / 单|壶; 1ulp 差与词面升序同向）
        let top3: Vec<&str> =
            out.iter().take(3).map(|c| c.word.as_str()).collect();
        assert!(
            top3.contains(&"但\u{1}互") && top3.contains(&"单\u{1}壶"),
            "次档 = 头首×尾次 / 头次×尾首"
        );
        assert_eq!(out[7].word, "单\u{1}胡", "末位 = 头次×尾末最低内分");
    }

    /// 尾槽字列深度 cap(ex,fz)（py slot_col_cap）: wu (1,2) →
    /// exact/ch 头只发 1 深（无）, 五 不入面（fz 深 2 由 fuzz 头消费）
    #[test]
    fn q_slot_combos_tail_cap_ex_depth() {
        let e = synth(&["dan", "wu"]);
        let out = e.q_slot_combos("danwu", &ss(&["dan", "wu"]));
        assert_eq!(out.len(), 2, "cap(ex)=1 × 头 top-2");
        assert_eq!(out[0].word, "但\u{1}无");
        assert!((out[0].score - (-2.90 - 2.50)).abs() < 1e-9);
        assert_eq!(out[1].word, "单\u{1}无");
        assert!(
            out.iter().all(|c| c.word != "但\u{1}五"),
            "ex 头深度 cap=1: 五 不入面"
        );
    }

    /// 全字链门: 链面须=查询最高内分面且内分 ≤ CHAIN_GATE_LP(-11.0)
    /// —— 但#超#没(-9.1) 不发锚 → 但|陪|壶(-10.24) 同律断路
    #[test]
    fn q_slot_combos_chain_gate_blocks() {
        let e = synth(&["dan", "pei", "hu"]);
        let out = e.q_slot_combos("danpeihu", &ss(&["dan", "pei", "hu"]));
        assert!(
            out.is_empty(),
            "唯一链面内分 -10.24 > 门 -11.0 → retain 非链后空集"
        );
    }

    /// 全字链门放行: 丢#黄#炸 型——链内分 ≤ 门且为最高面时发射,
    /// 逐段 \u{1} 槽界, 终值 = 链内分（lp 和）
    #[test]
    fn q_slot_combos_chain_gate_emits() {
        let e = synth(&["dan", "pei", "pei", "hu"]);
        let out = e.q_slot_combos(
            "danpeipeihu",
            &ss(&["dan", "pei", "pei", "hu"]),
        );
        // p=1/2 尾段无词列不发 → p=3 链为唯一面
        assert_eq!(out.len(), 1, "p=3 全字链唯一面");
        let c = &out[0];
        assert_eq!(c.word, "但\u{1}陪\u{1}陪\u{1}壶", "全字链逐段槽界");
        assert!(
            (c.score
                - (-2.90 - 3.84 - 3.84 - 3.50))
                .abs()
                < 1e-9,
            "链内分 = -14.08 ≤ 门 -11.0 放行"
        );
    }

    /// slot_word_tier: 词层=读音前缀多音节词列（DAT 供给, jianggu
    /// 跨界异切域）; <5 键门直回; 空 DAT 零发射不 panic
    #[test]
    fn slot_word_tier_needs_dat_prefix_column() {
        let e = synth(&["dan", "cha", "mo"]);
        let mut out = Vec::new();
        e.slot_word_tier("dan", &ss(&["dan"]), &mut out);
        assert!(out.is_empty(), "letters=dan 3 < 5 键门直回");
        e.slot_word_tier("danchaomo", &ss(&["dan", "cha", "mo"]), &mut out);
        assert!(
            out.is_empty(),
            "空 DAT 前缀词列零供给（l=n-2..4 前缀全 miss）"
        );
    }

    /// slot_col_chain 掉尾元音续列走停律（gao→ga / diu→di native 律）:
    /// 掉后串入音节表才续列, 短于 3 或非元音尾即止——防悬挂;
    /// 空 DAT 下 fb_seg_columns 零发射（链列深 10 供给域不涉及）
    #[test]
    fn slot_col_chain_vowel_tail_termination() {
        let mut out = Vec::new();
        // ga 在表: gao→ga 一跳后 len<3 止
        synth(&["gao", "ga"]).slot_col_chain(&ss(&["gao"]), &mut out);
        assert!(out.is_empty());
        // ga 不在表: 首轮即断
        synth(&["gao"]).slot_col_chain(&ss(&["gao"]), &mut out);
        assert!(out.is_empty());
        // 非元音尾（dan 尾 n）: 直接断
        synth(&["dan"]).slot_col_chain(&ss(&["dan"]), &mut out);
        assert!(out.is_empty(), "三条走停路径均收敛不悬挂");
    }
}
