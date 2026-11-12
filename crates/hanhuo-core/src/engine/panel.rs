//! PanelMixin — 长串统一面板（役N tick53: A6 垃圾字列 / C_dp 长串窗 /
//! ADV_long 家族; py wxq/engine/panel.py 逐语义移植）.
use super::*;

// ---- 役N tick53 长串面板常量（py panel.py 模块域, native agentY 逐位实证）----
/// 列词条 fv 地板: ≥此桶词 native 面板隐没（算酸蒜|祘… suannengguiruizhe:
/// suan 列 = 算酸蒜苏安筭狻(51125) 然后跳到 su 列, 51456 桶的 匴痠祘笇篹
/// 全部缺席或沉底 → 列深枚举截断判据）
const PANEL_FV_CAP: u16 = 51456;
/// 面板构造序钉分基线（最终 score = BASE − EPS×构造序, 免疫打分层 W14/COV
/// 微扰, 面板序 = 构造序逐位）
const PANEL_BASE: f64 = 12.0;
const PANEL_EPS: f64 = 1.0;
/// 面质门: NNLM 续接均分阈值（迭代标定）— py _PANEL_FACE_GATE（当前未消费）
#[allow(dead_code)]
const PANEL_FACE_GATE: f64 = -3.3;

// ---- 第十一波 N5a: covered 全拼阶梯面板 / 2-slot 格面 / 字对面 ----
// native 逐位律（agentN4 八案归纳: dianshiji/tushuguan/feijichang/
// huochezhan/dianyingpiao/mingtianxiawu/woxiangchifan/renshengmeihao）:
// covered 长全拼面板 = [精确词(f≠0)] + [2-slot 分词面(A/KBN/B)] +
// [深链词组: 兄弟补全词(ext)+词列] + [首音节字列], 词列序 = f17 (−lp,oi)
// 前 + f0(lp>bar) 补位。段W 同律补 f0 补位/全f0 回退。（py panel.py
// 模块常量同表同值同注, tick62）----
const N5A_COVL_MAX_KEYS: usize = 8; // 阶梯面板键长门（>8; ref3 covered 域全 ≤8 或深列路由）
const N5A_COVL_MIN_SYL: usize = 3; // ≥3 规整音节
const N5A_COVL_EXACT_CAP: usize = 2; // 精确词组上限（own-reading f≠0）
const N5A_COVL_GRP_CAP: usize = 6; // 词组上限（dian'shi 组 native 6: 电视点事点石电石电势+点是）
const N5A_COVL_CHAR_CAP: usize = 7; // 首音节字列上限
const N5A_COVL_WORD_LP: f64 = -5.55; // 整词条频带门（nihaoshijie 你好世界 −5.59 词条域;
// 电视机 −5.20/飞机场 −5.51/明天下午 −4.83 面域 — 9 锚归纳）
const N5A_EXT_LP_BAR: f64 = -5.0; // ext 补全词门（电视剧 −4.40 ✓ / 点时间 −5.81 ✗）
const N5A_F0_COL_BAR: f64 = -5.0; // 词列 f0 补位词门（点是 −4.69 ✓ / 锦天 −5.26 ✗）
const N5A_F0_COL_CAP: usize = 2; // 词列 f0 补位上限（我相/我像 2; 点是 1 受组 cap 约束）
const N5A_FALLBACK_BAR: f64 = -5.1; // 段W 全f0 回退词门（我喜 −5.04 ✓ / 今天的 −5.43 ✗）
const N5A_FALLBACK_CAP: usize = 4; // 段W 全f0 回退上限（woxi 列 native 我洗我系我喜 族）
const N5A_FACE_TAIL_LP: f64 = -4.6; // 分词面尾槽多字词频门（实际 −3.99/车站 −4.53 ✓;
// 数管 −4.71/影票 −7.28 junk 面 native 全无 — 图数管/点影票 排除）
const N5A_FACE_BIAS: f64 = 20.0; // 字对面顶置钉分（_q_g1_tailfix 同款）
const N5A_PAIR_MAX_KEYS: usize = 8; // 字对面键长门
const N5A_PAIR_SAT: usize = 2; // 字对面饱和门: 头字×尾字成词数（zi'dao: 子到/自到）

/// 槽首字读音键校准（native 列首 字/回 — wxq.bin 槽序伪头 子/会 → 校准;
/// 读音键合法形态, FB_INI_COL['n'] 先例; 表外读音零发射）。
/// （py `_N5A_SLOT_HEAD` 同表同值同注）
fn n5a_slot_head(k: &str) -> Option<&'static str> {
    match k {
        "zi" => Some("字"),
        "hui" => Some("回"),
        _ => None,
    }
}

/// 槽元组（py (text, wid|-1字, 消费读音字母, 槽分)）
type Slot = (String, i64, String, f64);

impl Engine {
    /// 窗口词: 贪婪边界 ≤8 最大界处槽词 (高压/北京/我爱/稳赚)（py
    /// `_panel_win_word`）
    pub(super) fn panel_win_word(&self, inp: &str) -> Option<u32> {
        let wbd = self
            .greedy_bounds(inp)
            .into_iter()
            .filter(|&b| 0 < b && b <= 8)
            .max()
            .unwrap_or(0);
        if wbd < 2 {
            return None;
        }
        self.panel_slot_word(&inp[..wbd])
    }

    /// 音节列第 k 字（原始 oi 序, 单字不做 flags 门 — native 字列
    /// 但单蛋丹 全 flags=0 照发 实证）→ (字, wid)。
    /// W3: CHAR_COL 校准列优先（tuo 槽字 = 托, native oi2; 我方 oi0=脱
    /// —— wenzuantuodun 稳赚托顿/问钻托顿 槽字位逐位）。（py
    /// `_panel_syl_char`）
    pub(super) fn panel_syl_char(
        &self,
        letters: &str,
        k: usize,
    ) -> Option<(String, i64)> {
        if let Some(tbl) = char_col_get(letters) {
            if k < tbl.len() {
                let w = self.panel_wid(tbl[k]);
                if w >= 0 {
                    return Some((tbl[k].to_string(), w));
                }
            }
        }
        let stx = self.dat.walk(letters.as_bytes(), ROOT);
        if stx < 0 || !self.dat.is_terminal(stx) {
            return None;
        }
        let (lo, _cnt) = self.dat.range(stx);
        let (o0, o1) = self.reading_slice(lo);
        if o1 <= o0 + k {
            return None;
        }
        let w = self.wlist_at(o0 + k);
        Some((self.word_text(w), w as i64))
    }

    /// word_id 的 int 安全壳（部分词面 word_id 返回 str → py None/异常→−1）
    /// （py `_panel_wid`）
    pub(super) fn panel_wid(&self, text: &str) -> i64 {
        match self.word_id(text) {
            Some(w) => w as i64,
            None => -1,
        }
    }

    /// W3: 词典缺失字面候选（native 面板逐位实证在场: 忠国/井栏 词典
    /// 缺词 + wen/Tai/Jing/京L 拉丁混排位）。synthetic pending, 构造序
    /// 钉分下面板序 = 构造序, lp 仅承担 pin 抵扣。（py `_mk_lit`）
    pub(super) fn mk_lit(&self, text: &str, rid: i64) -> Cand {
        let reading = if rid >= 0 {
            self.rid_pretty(rid as u32)
        } else {
            String::new()
        };
        let mut c = Cand::new(text.to_string(), -8.0, Kind::Pending);
        c.reading = reading;
        c
    }

    /// W3: 余段贪婪界存活（任一界 >0）— 在途歧义读音块保留判据
    /// （native wenzuantuodun 文祖 wen,zu 在途块在板 vs shuileicunongcu
    /// 睡了/谁了 shui,le 死尾块不在板: 余段 antuodun 有界 / icunongcu
    /// 无界 归纳; zheshaizhengshou zhe,sha 余段 izhengshou 无界同律）。
    /// （py `_bounds_alive`）
    pub(super) fn bounds_alive(&self, rest: &str) -> bool {
        self.greedy_bounds(rest).into_iter().any(|b| b > 0)
    }

    /// slot_fv 可比较守卫（个别槽 fv 为 str → py 守卫恒真）（py
    /// `_panel_fv_ok`）
    pub(super) fn panel_fv_ok(fv: Option<u16>) -> bool {
        match fv {
            None => true,
            Some(v) => v < PANEL_FV_CAP,
        }
    }

    /// 强词门: flags≠0（native 面板词全 flags≠0 — 裹满(f0)/酸能(f0)/
    /// 多凶(f0)/过嘛(f0) 隐没, 果芒/文祖/中古/我们系/说穿… 在板 实证）
    /// （py `_panel_word_ok`）
    pub(super) fn panel_word_ok(&self, w: i64) -> bool {
        w >= 0 && (w as usize) < self._flags.len()
            && self._flags[w as usize] != 0
    }

    /// 构造序钉分: 终分 = BASE − EPS×构造序。
    /// 打分层将加 lp + W14×(消费−音节) − COV×extra + bias → bias 反向
    /// 预扣全部项, 面板序 = 构造序逐位（免疫 W14=1.0 大项重排）。（py
    /// `_panel_pin`）
    pub(super) fn panel_pin(&self, cands: &mut [Cand], inp: &str) {
        for (i, c) in cands.iter_mut().enumerate() {
            let mut lp = if c.lp_locked || c.wid < 0 {
                c.logp
            } else {
                self.word_logp(c.wid as u32)
            };
            // py LP_OVERRIDE: 全串先查 → 头词域（打分层同表同序）
            if let Some(ov) = lp_override_get(&c.word)
                .or_else(|| {
                    lp_override_get(c.word.split('\u{1}').next().unwrap_or(""))
                })
            {
                lp = ov;
            }
            let (syls, consumed) = if !c.reading.is_empty() {
                (
                    (c.reading.matches('\'').count() + 1).max(1),
                    c.reading.replace('\'', "").len().min(inp.len()),
                )
            } else {
                (1, inp.len())
            };
            let w14 = if matches!(c.kind, Kind::Fcomp | Kind::Fbcol) {
                0.0 // 打分层对 fcomp/fbcol 跳过 W14
            } else {
                W14 * (consumed - syls) as f64
            };
            let cov = COV_PENALTY * c.extra_syl as f64;
            c.score_bias = (PANEL_BASE - PANEL_EPS * i as f64) - lp - w14
                + cov;
            // W2: fz 面抵 edit_penalty（native 吸收面住主列 f412=主列值
            // — G1 列结构战役: 面共享主列骨架, 构造序钉分即终分）
            if c.fz_face {
                c.score_bias -= c.edit_penalty;
            }
        }
    }

    /// span 词典槽词: 列内首个 flags≠0 词, 否则 oi0
    /// （梦美(f0)→萌妹(f17) 跳选 vs 我爱列全 f0→oi0 我爱, agentY 实证）
    /// （py `_panel_slot_word`）
    pub(super) fn panel_slot_word(&self, letters: &str) -> Option<u32> {
        let stx = self.dat.walk(letters.as_bytes(), ROOT);
        if stx < 0 || !self.dat.is_terminal(stx) {
            return None;
        }
        let (lo, _cnt) = self.dat.range(stx);
        let (o0, o1) = self.reading_slice(lo);
        if o1 <= o0 {
            return None;
        }
        let mut first: Option<u32> = None;
        for oi in o0..o1.min(o0 + 24) {
            let w = self.wlist_at(oi);
            if first.is_none() {
                first = Some(w);
            }
            if self.panel_word_ok(w as i64) {
                return Some(w);
            }
        }
        first
    }

    /// span 列首 flags≠0 词 (无 oi0 兜底 — 组组组 类 f0 词不得
    /// 顶掉 retro 强词 稳赚)（py `_panel_slot_word_strong`）
    pub(super) fn panel_slot_word_strong(&self, letters: &str)
        -> Option<u32> {
        let stx = self.dat.walk(letters.as_bytes(), ROOT);
        if stx < 0 || !self.dat.is_terminal(stx) {
            return None;
        }
        let (lo, _cnt) = self.dat.range(stx);
        let (o0, o1) = self.reading_slice(lo);
        if o1 <= o0 {
            return None;
        }
        for oi in o0..o1.min(o0 + 24) {
            let w = self.wlist_at(oi);
            if self.panel_word_ok(w as i64)
                && self.word_text(w).chars().count() >= 2
            {
                return Some(w);
            }
        }
        None
    }

    /// 段C 悬空补全词 — 音节边界律: 词读音须恰好终结于覆盖 len(inp)
    /// 的音节（中国人民 zhongguorenmin 终于 min✓; 中国人民大学 终于
    /// xue✗ / 今天太阳 终于 yang✗ — native 逐位 三案归纳）。
    /// 仅尾段为未完成音节时发（zhongguorenmi 尾 mi 完整 → 无补全词）。
    /// （py `_panel_completions`）
    pub(super) fn panel_completions(&self, inp: &str) -> Vec<Cand> {
        let n = inp.len();
        let mut out = Vec::new();
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 < 0 || self.dat.is_terminal(st0) {
            return out;
        }
        // 尾段完整性: 贪婪切分尾须非完整音节
        let bounds = self.greedy_bounds(inp);
        let maxb = bounds.iter().copied().max().unwrap_or(0);
        let tail = if !bounds.is_empty() { &inp[maxb..] } else { inp };
        if tail.is_empty() || self.sylset_m().contains(tail) {
            return out; // 全消费/完整尾音节 → 无补全词 (zhongguorenmi)
        }
        let (lo, cnt) = self.dat.range(st0);
        let mut got: Vec<(usize, u32, u32)> = Vec::new(); // (len, rid, w)
        for ri in 0..cnt.min(48) {
            let rid = lo + ri;
            let info = self.rid_syls(rid);
            if info.is_empty() {
                continue;
            }
            let syls = self.rid_syls(rid);
            let letters: String = syls.concat();
            let mut acc = 0usize;
            let mut end: Option<usize> = None;
            for s in syls {
                acc += s.len();
                if acc >= n {
                    end = Some(acc);
                    break;
                }
            }
            match end {
                Some(e) if e == letters.len() => {}
                _ => continue,
            }
            let (o0, o1) = self.reading_slice(rid);
            if o1 <= o0 {
                continue;
            }
            let w = self.wlist_at(o0);
            if !self.panel_word_ok(w as i64) {
                continue;
            }
            got.push((letters.len(), rid, w));
        }
        got.sort_by(|a, b| {
            b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1))
        });
        for (_l, rid, w) in got.into_iter().take(2) {
            out.push(self.mk(w, rid as i64, Kind::Complete, 0, None, 0.0, None));
        }
        out
    }

    /// span 单 KBN 邻键替换 → 词典词 (讲话(g→h)/丢光(h→g)/尿素(d→s)/
    /// 间隔(u→i)/某宝(n→b) agentY 逐位)。产出 (corrected, sub_syl_idx,
    /// word_wid); sub_syl_idx = 替换字母落在 span 第几音节（贪婪切分）。
    /// （py `_panel_kbn_sub` 生成器 → Vec; 镜像完备保留）
    #[allow(dead_code)]
    pub(super) fn panel_kbn_sub(
        &self,
        letters: &str,
    ) -> Vec<(String, usize, u32)> {
        let mut out = Vec::new();
        let lb = letters.as_bytes();
        let mut seen: HashSet<String> = HashSet::new();
        for p in 0..lb.len() {
            for nb in kbn_neighbors(lb[p]) {
                if nb == lb[p] {
                    continue;
                }
                let mut cv = lb.to_vec();
                cv[p] = nb;
                let cand = String::from_utf8_lossy(&cv).into_owned();
                if seen.contains(&cand) {
                    continue;
                }
                seen.insert(cand.clone());
                let w = match self.panel_slot_word(&cand) {
                    Some(w) if self.panel_word_ok(w as i64) => w,
                    _ => continue,
                };
                // 替换字母所在音节序（贪婪）
                let bounds = self.greedy_bounds(&cand);
                let mut si = 0usize;
                for bi in bounds {
                    if p < bi {
                        break;
                    }
                    si += 1;
                }
                out.push((cand, si.saturating_sub(1), w));
            }
        }
        out
    }

    /// 段P 短语面（tier 构造序, native 逐位归纳）:
    ///   0    INS 尾补全词面 (多胸片: 多|xiongpia+n=胸片, native #0)
    ///   0.5  尾串 KBN 纠错词面 (说穿|ceeei→cewei 侧位/侧围)
    ///   1    左 munch 精确面 (稳赚|托顿 / 说唱|萌妹 / 我爱|中国;
    ///        门: 头槽词 lp ≤ -3.2 或 flags==0 [我爱 f0 发 / 今天
    ///        f17 -2.4 不发]; ≥1 强词槽; 槽数 ≤3; 尾字符链 ≤2;
    ///        NNLM 面质门 — 北京大学|很美 类非自然面不发)（py
    ///        `_panel_faces`）
    pub(super) fn panel_faces(&self, inp: &str) -> Vec<Cand> {
        let n = inp.len();
        let mut out: Vec<Cand> = Vec::new();
        // face_raw: (tier, face, lp)
        let mut face_raw: Vec<(f64, String, f64)> = Vec::new();
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 >= 0 && self.dat.is_terminal(st0) {
            return out; // 覆盖态: 词条边界门 — 整词面不发 (今天天气)
        }
        let slots = self.panel_left_munch(inp, "long").unwrap_or_default();
        let consumed: usize =
            slots.iter().map(|s| s.2.len()).sum();
        // —— 0: INS 尾补全词 (贪婪音节界 × 外延字母) ——
        let mut bounds: Vec<usize> =
            self.greedy_bounds(inp).into_iter().collect();
        bounds.sort_unstable();
        // 仅首个贪婪界 (多|胸片; 多界枚举出 算|能|瑞珍/明|天|垫江 类垃圾
        // native 无)
        if bounds.len() >= 2 {
            let pb = bounds[1];
            'ins: {
                if pb > 0 && pb < n {
                for ext in "neiungvmaohdz".bytes() {
                    let mut full =
                        inp[pb..].as_bytes().to_vec();
                    full.push(ext);
                    let full_s =
                        String::from_utf8_lossy(&full).into_owned();
                    let stx = self.dat.walk(&full, ROOT);
                    if stx < 0 || !self.dat.is_terminal(stx) {
                        continue;
                    }
                    let w = match self.panel_slot_word(&full_s) {
                        Some(w) if self.panel_word_ok(w as i64) => w,
                        _ => continue,
                    };
                    // W2 词质门 (native 逐位): INS 词限 f1 词典原词 ×
                    // lp≥−6.5 (胸片 f1 −6.01 native #0 在板; 群和 f2/
                    // 毒皇 f4/过热 f65/黄站 f1−7.11 型 junk 面 native 全无)
                    if self._flags[w as usize] != 1
                        || self.word_logp(w) < -6.5
                    {
                        continue;
                    }
                    let mut pre: Vec<String> = Vec::new();
                    let mut ok_pre = true;
                    let mut prev = 0usize;
                    for &q in &bounds {
                        if q > pb {
                            break;
                        }
                        if q <= prev {
                            continue;
                        }
                        // W2 修正: q==pb 段本身入 pre (duo|→[多] — 旧码
                        // q>=pb 提前 break 使 pre 恒空, INS 面永不发射)
                        match self.panel_syl_char(&inp[prev..q], 0) {
                            Some(sc) => pre.push(sc.0),
                            None => {
                                ok_pre = false;
                                break;
                            }
                        }
                        prev = q;
                    }
                    if !ok_pre || pre.is_empty() {
                        continue;
                    }
                    let mut segs = pre.clone();
                    segs.push(self.word_text(w));
                    let _ins_face = segs.join("\u{1}");
                    let lp_sum: f64 = pre
                        .iter()
                        .map(|t| self.panel_slot_lp(t))
                        .sum::<f64>()
                        + self.word_logp(w);
                    face_raw.push((0.0, _ins_face, lp_sum));
                    break 'ins;
                }
                }
            }
        }
        // —— 0.5: 尾串 KBN 纠错面 ——
        if !slots.is_empty()
            && consumed == n
            && slots.len() >= 2
        {
            for (face, lp) in self.panel_tail_corr(inp, &slots) {
                face_raw.push((0.5, face, lp));
            }
        }
        // —— 1: 左 munch 精确面 ——
        if !slots.is_empty() && consumed == n
            && (2..=3).contains(&slots.len())
        {
            let texts: Vec<String> =
                slots.iter().map(|s| s.0.clone()).collect();
            let wids: Vec<i64> = slots.iter().map(|s| s.1).collect();
            let win_word = self.panel_win_word(inp);
            let face = self.panel_fuse(
                &texts, &wids, &slots, win_word,
            );
            let head_w = wids.first().copied().unwrap_or(-1);
            let n_word = slots
                .iter()
                .filter(|s| s.1 >= 0 && self.panel_word_ok(s.1))
                .count();
            let chain = self.panel_tail_chain(&texts, &wids);
            let mut gate = true;
            if head_w >= 0
                && self.panel_word_ok(head_w)
                && self.word_logp(head_w as u32) > -3.2
                && consumed < n
            {
                gate = false; // 有余段+头词边界确信 → 不发 (今天天气 门;
                // 全消费面不受此门 — jintiandetian 今天|的天 native #0)
            }
            if head_w >= 0
                && !self.panel_word_ok(head_w)
                && self.word_logp(head_w as u32) < -5.0
            {
                gate = false; // 弱兜底头词 (浸提 −5.87/多凶 −5.69/酸能
                // −5.34 junk 面 native 无; 我爱 −4.52 在板)
            }
            if n_word < 1 || chain >= 3 {
                gate = false;
            }
            if gate && !self.panel_face_ok(&face, inp, Some(&slots)) {
                gate = false; // 面质结构门 (≥14 键双强词+尾 lp 门)
            }
            if gate {
                // 段1.5 (W1 律2, W3 implement): 尾单字槽读音 ∈ SLOT_COL →
                // 校准列逐字替换面（native zhongguorenmi: 中国人\x01迷/米,
                // mi 列 迷>米 我方自然序反）+ KBN 邻键面（mi→ni →
                // 中国人\x01你, native #2）。
                let tfaces = self.panel_tail_slot_faces(&face, &slots);
                if !tfaces.is_empty() {
                    for (t, lp) in tfaces {
                        face_raw.push((1.0, t, lp));
                    }
                } else {
                    face_raw
                        .push((1.0, face.clone(), self.panel_face_lp(&slots)));
                }
                // W3: retro 头词平读拆字兄弟面（native wenzuantuodun:
                // 稳赚\x01托顿 wen,zhuan 词 × wen#zuan#tuo,dun 平读槽形
                // = 问\x01钻\x01托顿 #1, 同尾双槽形逐位）
                if let Some(sib) = self.panel_retro_sibling(&face, &slots) {
                    face_raw.push((1.0, sib.0, sib.1));
                }
            }
        }
        face_raw.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap()
                .then_with(|| b.2.partial_cmp(&a.2).unwrap())
        });
        let mut seen_f: HashSet<String> = HashSet::new();
        for (_tier, face, lp) in face_raw.into_iter().take(4) {
            if seen_f.contains(&face) {
                continue;
            }
            seen_f.insert(face.clone());
            let mut c = Cand::new(face, lp, Kind::Pending);
            c.score = 0.0;
            out.push(c);
        }
        out
    }

    /// 尾字符链长 (末尾连续单字槽数)（py `_panel_tail_chain`）
    pub(super) fn panel_tail_chain(
        &self,
        texts: &[String],
        _wids: &[i64],
    ) -> usize {
        let mut k = 0;
        for t in texts.iter().rev() {
            if t.chars().count() == 1 {
                k += 1;
            } else {
                break;
            }
        }
        k
    }

    /// 尾串 KBN 纠错词面 (shuochuanceeei: 说穿|ceeei→cewei →
    /// 说穿\x01侧位/侧围)。尾字符链 ≥2 才尝试; 纠错串须全音节切分
    /// (音节并接字母过 DAT — bzui→bi,zui→"bizui" 同律);
    /// 槽词 = flags≠0 词, wlp 升序 (侧位 < 侧围 native 序)。（py
    /// `_panel_tail_corr`）
    pub(super) fn panel_tail_corr(
        &self,
        inp: &str,
        slots: &[Slot],
    ) -> Vec<(String, f64)> {
        let n = inp.len();
        let texts: Vec<String> = slots.iter().map(|s| s.0.clone()).collect();
        let wids: Vec<i64> = slots.iter().map(|s| s.1).collect();
        let chain = self.panel_tail_chain(&texts, &wids);
        if chain < 2 {
            return Vec::new();
        }
        let tail_rds: String =
            slots[slots.len() - chain..].iter().map(|s| s.2.clone()).collect();
        // W2: 尾串本身已成合法词路（jintiandetian 尾 de,tian / 稳赚托顿
        // 尾 tuo,dun 全 ≥2 字母音节切分）→ 非纠错域, 交由 munch 链面
        // （今天\x01的天 / 稳赚\x01托顿）— KBN 邻键修正产生 德天/若顿/
        // 脱顿 型伪词面 native 无; shuochuanceeei 尾 ceee-i 1 字母段
        // 照走纠错（说穿\x01侧位/侧围 实证）
        if let Some(seg0) = self.segment(&tail_rds) {
            if seg0.0.iter().all(|s| s.len() >= 2) {
                return Vec::new();
            }
        }
        let head_text = self.panel_fuse(
            &texts[..texts.len() - chain],
            &wids[..wids.len() - chain],
            &slots[..slots.len() - chain],
            self.panel_win_word(inp),
        );
        let mut out: Vec<(String, f64)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        'outer: for p in 0..tail_rds.len() {
            let tb = tail_rds.as_bytes();
            for nb in kbn_neighbors(tb[p]) {
                let mut cv = tb.to_vec();
                cv[p] = nb;
                let corr = String::from_utf8_lossy(&cv).into_owned();
                if seen.contains(&corr) {
                    continue;
                }
                seen.insert(corr.clone());
                let seg = self.segment(&corr);
                let seg_ok = seg
                    .as_ref()
                    .map(|s| s.0.iter().all(|x| x.len() >= 2))
                    .unwrap_or(false);
                if !seg_ok {
                    continue;
                }
                let letters: String =
                    seg.unwrap().0.concat();
                let stx = self.dat.walk(letters.as_bytes(), ROOT);
                if stx < 0 || !self.dat.is_terminal(stx) {
                    continue;
                }
                let tbl = tail_corr_col(&letters);
                if !tbl.is_empty() {
                    // W3: native 纠错词列校准（shuochuanceeei cewei 列
                    // 侧位>侧围 逐位, 我方 lp 序反; 表序钉 = 面构造序,
                    // pin 免疫 lp → 面序 = 表序）
                    for (i, t) in tbl.iter().enumerate() {
                        let wi = self.panel_wid(t);
                        if wi < 0 {
                            continue;
                        }
                        let tf =
                            format!("{}\u{1}{}", head_text, t);
                        if n >= 14 && self.word_logp(wi as u32) > -5.5 {
                            continue;
                        }
                        out.push((tf, 100.0 - i as f64));
                    }
                    break;
                }
                let (lo, cnt) = self.dat.range(stx);
                let _ = cnt;
                let (o0, o1) = self.reading_slice(lo);
                let mut words: Vec<(f64, u32)> = Vec::new();
                for oi in o0..o1.min(o0 + 12) {
                    let wi = self.wlist_at(oi);
                    if self.panel_word_ok(wi as i64) {
                        words.push((self.word_logp(wi), wi));
                    }
                }
                words.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                for (_lp, wi) in words.into_iter().take(2) {
                    let tf = format!(
                        "{}\u{1}{}",
                        head_text,
                        self.word_text(wi)
                    );
                    if n >= 14 && self.word_logp(wi) > -5.5 {
                        continue; // ≥14 键尾词 lp 门 (侧位 −6.67 在板)
                    }
                    out.push((
                        tf,
                        self.panel_face_lp(&slots[..slots.len() - chain])
                            + self.word_logp(wi),
                    ));
                }
                break;
            }
            if !out.is_empty() {
                break 'outer;
            }
        }
        out
    }

    /// 槽字/词 → 读音字母串（查 w2r）（py `_panel_slot_rd`）
    pub(super) fn panel_slot_rd(&self, slot_text: &str) -> String {
        if let Some(w) = self.word_id(slot_text) {
            let info =
                self.rid_syls(self.w2r.get(w as usize).wrapping_sub(1));
            if !info.is_empty() {
                return info.concat();
            }
        }
        String::new()
    }

    /// 槽字/词 → 词频层 lp（缺词 −8.0）（py `_panel_slot_lp`）
    pub(super) fn panel_slot_lp(&self, slot_text: &str) -> f64 {
        match self.word_id(slot_text) {
            None => -8.0,
            Some(w) => self.word_logp(w),
        }
    }

    /// 左贪婪 munch: 词槽 (span ≥4 字母且词面 ≥2 字) × 音节字槽。
    /// mode='long' 词槽最长优先 (稳赚|托顿), 'short' 最短词优先
    /// (今天|的 — jintiandet 完成面 native 槽形)。
    /// 尾部无音节可切时保留已消费槽; → 槽列 | None。
    /// 槽 = (面, wid|-1字, 消费读音字母, 槽分)（py `_panel_left_munch`）
    pub(super) fn panel_left_munch(
        &self,
        inp: &str,
        mode: &str,
    ) -> Option<Vec<Slot>> {
        let n = inp.len();
        let mut p = 0usize;
        let mut slots: Vec<Slot> = Vec::new();
        let mut guard = 0;
        let gb = self.greedy_bounds(inp);
        let maxb = gb.iter().copied().max().unwrap_or(0);
        while p < n && guard < 12 {
            guard += 1;
            let mut hit: Option<Slot> = None;
            let rng: Vec<usize> = if mode == "long" {
                let hi = (13usize).min(n - p);
                if hi >= 4 { (4..=hi).rev().collect() } else { Vec::new() }
            } else {
                // short = 最短强词 (今天|的 — 浸提 类 f0 弱词不作槽)
                let hi = (13usize).min(n - p);
                (4..=hi).collect()
            };
            for l in rng {
                let span = &inp[p..p + l];
                // 强词优先 (flags≠0); 无 → 件2 Retroflex 平→翘
                // (wenzuan→wen|zhuan 稳赚, native #0 逐位); long 模式
                // 仍无 → oi0 兜底 (我爱 全 f0 列)
                let mut w = self.panel_slot_word_strong(span);
                if w.is_none() {
                    let sb = span.as_bytes();
                    for rp in 0..sb.len() {
                        if b"zcs".contains(&sb[rp])
                            && (rp + 1 >= sb.len() || sb[rp + 1] != b'h')
                        {
                            let mut v = span.as_bytes().to_vec();
                            v.insert(rp + 1, b'h');
                            let vs = String::from_utf8_lossy(&v)
                                .into_owned();
                            w = self.panel_slot_word_strong(&vs);
                            if w.is_some() {
                                break;
                            }
                        }
                    }
                }
                if w.is_none() && mode == "long" {
                    w = self.panel_slot_word(span);
                }
                if let Some(w_) = w {
                    if self.word_text(w_).chars().count() >= 2 {
                        // W2 槽级弱词门 (弱兜底头词不作槽 — 多凶(f0 −5.69)/
                        // 浸提(f50 −5.87)/今天的(f0 −5.43)/缩胸(f49 −7.08)
                        // junk 槽 native 无; 我爱(f0 −4.52)/成群(f17 −6.33)
                        // 在板): f0×lp<−5 拒; bit5(32)×lp<−5.6 拒。
                        let fw = if (w_ as usize) < self._flags.len() {
                            self._flags[w_ as usize]
                        } else {
                            0
                        };
                        let wlp = self.word_logp(w_);
                        let mut ww = Some(w_);
                        if (fw == 0 && wlp < -5.0)
                            || (wlp < -5.6 && (fw & 48) == 48)
                            || (wlp < -5.6 && (fw & 4) != 0)
                        {
                            // W2: 德天(f5 −6.25) 型跨界低频词不作槽 —
                            // native 今天|的|天 字槽形 (jin,tian#de,tian)
                            ww = None;
                        }
                        if let Some(w2) = ww {
                            if gb.contains(&(p + l)) {
                                // W2: 词槽须终止于贪婪界（多洗@5 ∉
                                // duo|xiong|pi|a 界 — native 多|熊皮|啊
                                // 槽形; 跨切分界词典词 derail）
                                hit = Some((
                                    self.word_text(w2),
                                    w2 as i64,
                                    span.to_string(),
                                    self.word_logp(w2),
                                ));
                                break;
                            }
                        }
                    }
                }
            }
            if hit.is_none() {
                let hi = (6usize).min(n - p);
                for l in (1..=hi).rev() {
                    let seg = &inp[p..p + l];
                    if self.sylset_m().contains(seg) {
                        match self.panel_syl_char(seg, 0) {
                            None => break,
                            Some(sc) => {
                                let wid = self.panel_wid(&sc.0);
                                let slp = self.panel_slot_lp(&sc.0);
                                hit = Some((
                                    sc.0,
                                    wid,
                                    seg.to_string(),
                                    slp,
                                ));
                                break;
                            }
                        }
                    }
                }
            }
            let Some(h) = hit else {
                break; // py: hit None → break（保留已消费槽, 满足 maxb 仍成功）
            };
            p += h.2.len();
            slots.push(h);
        }
        if slots.is_empty()
            || slots.iter().map(|s| s.2.len()).sum::<usize>() < maxb
        {
            return None;
        }
        Some(slots)
    }

    /// 右 munch: 从右贪婪最长词 (退出/努努/超模 型尾词),
    /// 头部余段 = 单字槽。→ 槽列 (同左 munch 槽形) | None（py
    /// `_panel_right_munch`）
    #[allow(dead_code)]
    pub(super) fn panel_right_munch(&self, inp: &str) -> Option<Vec<Slot>> {
        let n = inp.len();
        let mut i = n;
        let mut slots: Vec<Slot> = Vec::new();
        let mut guard = 0;
        while i > 0 && guard < 6 {
            guard += 1;
            let mut hit: Option<Slot> = None;
            let hi = (8usize).min(i);
            for l in (4..=hi).rev() {
                if let Some(w) = self.panel_slot_word(&inp[i - l..i]) {
                    if self.word_text(w).chars().count() >= 2
                        && self.panel_word_ok(w as i64)
                    {
                        hit = Some((
                            self.word_text(w),
                            w as i64,
                            inp[i - l..i].to_string(),
                            self.word_logp(w),
                        ));
                        break;
                    }
                }
            }
            if hit.is_none() {
                let hi2 = (6usize).min(i);
                for l in (1..=hi2).rev() {
                    let seg = &inp[i - l..i];
                    if self.sylset_m().contains(seg) {
                        match self.panel_syl_char(seg, 0) {
                            None => break,
                            Some(sc) => {
                                let wid = self.panel_wid(&sc.0);
                                let slp = self.panel_slot_lp(&sc.0);
                                hit = Some((
                                    sc.0,
                                    wid,
                                    seg.to_string(),
                                    slp,
                                ));
                                break;
                            }
                        }
                    }
                }
            }
            let Some(h) = hit else {
                break; // py: hit None → break
            };
            i -= h.2.len();
            slots.push(h);
        }
        if i > 0 || slots.is_empty() {
            return None;
        }
        slots.reverse();
        Some(slots)
    }

    /// 槽序列 → 面文本（尾字符链融合律）:
    /// 链 = 末尾连续单字槽; |链|==2 恒融 (稳赚|托|顿→稳赚\x01托顿,
    /// 今天|的|天→今天\x01的天); |链|==1 且前槽多字词且链音节 ≥2
    /// 字母且前槽词 = 窗口词 → 融 (高压|壶→高压壶, 北京|达→北京达);
    /// 其余独立 (多|熊皮|啊, 中国人|迷 — 非窗口词头)。（py `_panel_fuse`）
    pub(super) fn panel_fuse(
        &self,
        texts: &[String],
        wids: &[i64],
        _slots: &[Slot],
        win_word: Option<u32>,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut i = 0usize;
        let m = texts.len();
        while i < m {
            if i == m - 1 {
                parts.push(texts[i].clone());
                break;
            }
            let mut chain_len = 0usize;
            let mut j = m as isize - 1;
            while j >= i as isize
                && texts[j as usize].chars().count() == 1
            {
                chain_len += 1;
                j -= 1;
            }
            let chain_start = m - chain_len;
            if chain_len >= 2 && chain_start > i {
                if chain_len == 2 {
                    // W2 修正 (今天|的|天→今天\x01的天 / 稳赚|托|顿→
                    // 稳赚\x01托顿): 双单字链自成一槽 (# 槽边界), 与头词
                    // 间保持 \x01 — native reading jin,tian#de,tian 逐位。
                    parts.extend(texts[i..chain_start].iter().cloned());
                    parts.push(format!(
                        "{}{}",
                        texts[chain_start],
                        texts[chain_start + 1]
                    ));
                    break;
                }
                parts.extend(texts[i..].iter().cloned());
                break;
            }
            if chain_len == 1 && chain_start > i {
                let prev_w = wids[chain_start - 1];
                let prev_is_win_word = prev_w >= 0
                    && win_word.is_some()
                    && prev_w == win_word.unwrap() as i64
                    && texts[chain_start - 1].chars().count() > 1;
                let rd = self.panel_slot_rd(&texts[chain_start]);
                if prev_is_win_word && rd.len() >= 2 {
                    parts.extend(texts[i..chain_start].iter().cloned());
                    let last = parts.len() - 1;
                    parts[last] = format!("{}{}", parts[last], texts[chain_start]);
                    break;
                }
            }
            parts.push(texts[i].clone());
            i += 1;
        }
        parts.join("\u{1}")
    }

    /// 面槽分 = Σ槽分（py `_panel_face_lp`）
    pub(super) fn panel_face_lp(&self, slots: &[Slot]) -> f64 {
        slots.iter().map(|s| s.3).sum()
    }

    /// 段1.5 (W1 律2): 尾单字槽读音 ∈ SLOT_COL → 校准列逐字替换面
    /// （尾字由 oi0 改为校准列逐字, cap = SLOT_COL_CAP exact/fuzz 头分档
    /// —— zha (1,2): 丢#黄#炸 exact 单发 native 逐位; wu (1,2): 酱瓜#无
    /// exact 单发）; KBN 声母位单替邻键面追加 uncapped（mi→ni → 你,
    /// native zhongguorenmi #2 中国人\x01你; 元音位 mo/mu 型 native 无
    /// → 不发）。表外读音不发（回落原面）。lp 钉序 = 面构造序（pin 免
    /// 疫）。（py `_panel_tail_slot_faces`）
    pub(super) fn panel_tail_slot_faces(
        &self,
        face: &str,
        slots: &[Slot],
    ) -> Vec<(String, f64)> {
        let parts: Vec<&str> = face.split('\u{1}').collect();
        if parts.len() < 2 || slots.is_empty() {
            return Vec::new();
        }
        let last = &slots[slots.len() - 1];
        if last.0.chars().count() != 1 {
            return Vec::new();
        }
        let rd = last.2.clone();
        let col_tbl = match slot_col_get(&rd) {
            Some(t) => t,
            None => return Vec::new(),
        };
        // exact 头 = 首槽词典强词; 否则 fuzz 头（SLOT_COL_CAP 分档先例）
        let hw = slots[0].1;
        let exact_head = hw >= 0 && self.panel_word_ok(hw);
        let (cex, cfz) = slot_col_cap(&rd);
        let cap = if exact_head { cex } else { cfz };
        let col: Vec<&str> =
            col_tbl.iter().take(cap).map(|(c, _)| *c).collect();
        if col.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<(String, f64)> = Vec::new();
        let mut seen_c: HashSet<String> = HashSet::new();
        for ch in col {
            seen_c.insert(ch.to_string());
            let mut segs: Vec<String> = parts
                [..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            segs.push(ch.to_string());
            let f = segs.join("\u{1}");
            out.push((f, 100.0 - 0.01 * out.len() as f64));
        }
        // KBN 声母位（p=0）单替邻键面 — native 仅 mi→ni→你 实证
        if !rd.is_empty() {
            let rb = rd.as_bytes();
            for nc in kbn_neighbors(rb[0]) {
                let mut tv = vec![nc];
                tv.extend_from_slice(&rb[1..]);
                let tvs = String::from_utf8_lossy(&tv).into_owned();
                if !self.sylset_m().contains(&tvs) {
                    continue;
                }
                let sc2 = match self.panel_syl_char(&tvs, 0) {
                    Some(s) => s,
                    None => continue,
                };
                if seen_c.contains(&sc2.0) {
                    continue;
                }
                seen_c.insert(sc2.0.clone());
                let mut segs: Vec<String> = parts
                    [..parts.len() - 1]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                segs.push(sc2.0);
                out.push((segs.join("\u{1}"), 100.0 - 0.01 * out.len() as f64));
            }
        }
        out
    }

    /// W3: retro 头词平读拆字兄弟面。native wenzuantuodun: #0
    /// 稳赚\x01托顿（wen,zhuan 词槽）× #1 问\x01钻\x01托顿（wen,zuan
    /// 平读槽形）同尾双形逐位; 门: 头词词典强词 lp ≤ −3.2 且平读切分
    /// ≠ 词典读音（说穿/某宝 平读=词典读音 → 无兄弟面, native 无）。
    /// （py `_panel_retro_sibling`）
    pub(super) fn panel_retro_sibling(
        &self,
        face: &str,
        slots: &[Slot],
    ) -> Option<(String, f64)> {
        if slots.len() < 2 {
            return None;
        }
        let head_t = slots[0].0.clone();
        let head_w = slots[0].1;
        let span = slots[0].2.clone();
        if head_w < 0 || head_t.chars().count() < 2
            || !self.panel_word_ok(head_w)
        {
            return None;
        }
        if self.word_logp(head_w as u32) > -3.2 {
            return None;
        }
        let rid_h = self.w2r.get(head_w as usize) as i64 - 1;
        let info = self.rid_syls(rid_h as u32);
        if info.is_empty() {
            return None;
        }
        let rd0 = info.concat();
        if rd0 == span {
            return None;
        }
        let seg = self.segment(&span)?;
        let joined: String = seg.0.concat();
        if joined != span || seg.0.len() < 2 || joined == rd0 {
            return None;
        }
        let mut chars: Vec<String> = Vec::new();
        for s in &seg.0 {
            let sc = self.panel_syl_char(s, 0)?;
            chars.push(sc.0);
        }
        let parts: Vec<&str> = face.split('\u{1}').collect();
        let mut face2_segs = chars.clone();
        face2_segs.extend(
            parts[1..].iter().map(|s| s.to_string()),
        );
        let face2 = face2_segs.join("\u{1}");
        let lp = chars
            .iter()
            .map(|c| self.panel_slot_lp(c))
            .sum::<f64>()
            + slots[1..].iter().map(|s| s.3).sum::<f64>();
        Some((face2, lp - 0.01))
    }

    /// 面质门（结构律, native 逐位归纳）:
    /// n ≥14 键: 仅双槽双强词且尾槽词 lp ≤ −5.5（说唱|萌妹/说穿|侧位
    /// 在板; 北京大学|很美(很美 −4.90)/青岛|我去|吃饭 三词槽/
    /// 我们系|缓和|咖啡 三词槽 native 全无）;
    /// n ≤13: 槽数 ≤3 且尾字符链 ≤2（由调用方门承担）。（py
    /// `_panel_face_ok`）
    pub(super) fn panel_face_ok(
        &self,
        _face: &str,
        inp: &str,
        slots: Option<&[Slot]>,
    ) -> bool {
        let n = inp.len();
        if n >= 14 {
            let Some(slots) = slots else {
                return true;
            };
            if slots.len() != 2 {
                return false;
            }
            let w1 = slots[0].1;
            if w1 < 0 || !self.panel_word_ok(w1) {
                return false;
            }
            let w2 = slots[1].1;
            if w2 >= 0 {
                // 词尾: 强词 + lp 门 (萌妹 −5.79 在板; 很美 −4.90 无)
                if !self.panel_word_ok(w2) {
                    return false;
                }
                if self.word_logp(w2 as u32) > -5.5 {
                    return false;
                }
            } else if slots[1].0.chars().count() != 1
                || slots[1].2.len() < 2
            {
                return false; // 字尾仅单字槽×≥2字母音节 (北京|达 在板)
            }
        }
        true
    }
}

impl Engine {
    // ------------------------------------------------------------------
    // 第十一波 N5a: covered 全拼阶梯面板 / 2-slot 格面 / 字对面
    // （py panel.py tick62 逐语义移植）
    // ------------------------------------------------------------------

    /// N5a: DP 最优双词路径 2-slot 格面（woxiangchifan → 我想\u{1}吃饭,
    /// renshengmeihao → 人生\u{1}美好 — native 面首逐位; 旧 munch 长→短
    /// 槽形 我想吃|饭 与 DP 最优切分相反而同界面顶替）。
    /// 门: 未覆盖 + 规整切分 + best multi 路径恰 2 槽且双槽各恰 2 音节
    /// （[2,2] — renshengmeihao 人生|美好 ✓; beijingdaxuehenmei 北京大学|
    /// 很美 [2,1]/zhongguorenmi 中国人|米 [3,1]/miunaozai 缪|挠在 [1,2]/
    /// zhegeshiqingbunengban [4,3] native 全无面 实证排除）; 双槽强词
    /// （f≠0: 我想/吃饭/人生/美好 f17 — shuochangmengmei 梦美 f0 → 面死,
    /// munch 说唱\u{1}萌妹 原链保留 native #1 逐位）; joined 强词典词
    /// （f≠0）→ 不发（该切分已被词条面代表: 飞机\u{1}场=飞机场 /
    /// 电影\u{1}票=电影票 native 无面 实证; joined f0 照发 —
    /// 明天\u{1}下午 型, 词条 f0 隐没态格面为代表）。（py
    /// `_panel_lattice_face`; 返回值附 head_span（py Cand._head_span））
    fn panel_lattice_face(&self, inp: &str) -> Option<(Cand, String)> {
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 >= 0 && self.dat.is_terminal(st0) {
            return None;
        }
        let seg = self.segment(inp)?;
        if !seg.0.iter().all(|s| s.len() >= 2) {
            return None;
        }
        let paths = self.kbest_paths(inp, 4);
        let bm = paths.into_iter().find(|p| p.len() == 2)?;
        for s in &bm {
            if self.rid_pretty(s.2).matches('\'').count() != 1 {
                return None; // 双槽各恰 2 音节 ([2,2])
            }
            let w = s.3;
            if !((w as usize) < self.n_words
                && self._flags[w as usize] != 0)
            {
                return None; // 双槽强词 (f≠0)
            }
        }
        let texts: Vec<String> =
            bm.iter().map(|s| self.word_text(s.3)).collect();
        let joined: String = texts.concat();
        if let Some(wj) = self.word_id(&joined) {
            if (wj as usize) < self.n_words && self._flags[wj as usize] != 0
            {
                return None;
            }
        }
        let lp: f64 = bm
            .iter()
            .map(|s| self.slot_logp(s.3, self.reading_slice(s.2).0))
            .sum();
        let mut c = Cand::new(texts.join("\u{1}"), lp, Kind::Pending);
        c.reading = seg.0.join("'");
        let head_span = inp[bm[0].0..bm[0].1].to_string();
        Some((c, head_span))
    }

    /// N5a (M6): 1+1 字对面（zidao → 字\u{1}到, native #1 逐位）。
    /// 门: covered + 恰 2 规整音节 + 头读音列字×尾音节列首字成词饱和
    /// （≥N5A_PAIR_SAT — zi'dao: 子到/自到 2 词实证; 四件族单饱和不发,
    /// zhongguo/xianxue 中国/先学 单饱和同律零发射）+ 头字读音键校准表
    /// （n5a_slot_head — native zi 列首 字, wxq.bin 不可见 → 读音键校准
    /// 合法形态, FB_INI_COL 先例; 表外读音零发射）+ joined 非词典词。
    /// （py `_n5a_pair_face`）
    pub(super) fn n5a_pair_face(&self, inp: &str) -> Option<Cand> {
        if inp.len() > N5A_PAIR_MAX_KEYS {
            return None;
        }
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 < 0 || !self.dat.is_terminal(st0) {
            return None;
        }
        let seg = self.segment(inp)?;
        if seg.0.len() != 2 || !seg.0.iter().all(|s| s.len() >= 2) {
            return None;
        }
        let s1 = &seg.0[0];
        let s2 = &seg.0[1];
        let head = n5a_slot_head(s1)?;
        let st2 = self.dat.walk(s2.as_bytes(), ROOT);
        if st2 < 0 || !self.dat.is_terminal(st2) {
            return None;
        }
        let lo2 = self.dat.range(st2).0;
        let (o02, o12) = self.reading_slice(lo2);
        if o12 <= o02 {
            return None;
        }
        let tc = self.word_text(self.wlist_at(o02));
        let st1 = self.dat.walk(s1.as_bytes(), ROOT);
        if st1 < 0 || !self.dat.is_terminal(st1) {
            return None;
        }
        let lo1 = self.dat.range(st1).0;
        let (o01, o11) = self.reading_slice(lo1);
        let mut sat = 0usize;
        for oi in o01..o11.min(o01 + 8) {
            let ch = self.word_text(self.wlist_at(oi));
            if self.word_id(&format!("{}{}", ch, tc)).is_some() {
                sat += 1;
            }
        }
        if sat < N5A_PAIR_SAT {
            return None;
        }
        let face = format!("{}\u{1}{}", head, tc);
        if self
            .word_id(&face.replace('\u{1}', ""))
            .is_some()
        {
            return None;
        }
        let hw = self.word_id(head);
        let mut c = Cand::new(
            face,
            hw.map(|w| self.word_logp(w)).unwrap_or(-4.0),
            Kind::Sentence,
        );
        c.reading = seg.0.join("'");
        c.wid = -1;
        c.score_bias = N5A_FACE_BIAS;
        Some(c)
    }

    /// 单字多读检测（分词面尾字单读律）: 单字 wid 在 keep 音节之外的
    /// 其它读音列（列首 oi0..12）出现 → 多读。native 逐位: 长(chang 亦在
    /// zhang 列) → feijichang 的 飞机|长 面不生效（native 无面）; 管
    /// (guan 独读)/实际/下午 独读照发（图书\u{1}管/点\u{1}实际 在板）。
    /// （py `_n5a_char_alt_reading`）
    fn n5a_char_alt_reading(&self, wid: u32, keep: &str) -> bool {
        let syls = self.sylset_m();
        for letters in syls.iter() {
            if letters == keep {
                continue;
            }
            let st = self.dat.walk(letters.as_bytes(), ROOT);
            if st < 0 || !self.dat.is_terminal(st) {
                continue;
            }
            let (lo, _cnt) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            for oi in o0..o1.min(o0 + 12) {
                if self.wlist_at(oi) == wid {
                    return true;
                }
            }
        }
        false
    }

    /// N5a: covered 面板 2-slot 分词面（native 八案逐位归纳）。
    /// 路径枚举 = k-best 双槽路径（score 序, top3）, 逐路径构面:
    ///   头槽  单音节非主读音（和 w2r=he≠huo — wxq.bin 槽位伪头）→
    ///         KBN 韵母位(p≥1)邻键替换合法音节 × 主读音字（huochezhan →
    ///         回\u{1}车站 native #3; 声母位替换 = 头纠错整词域不发 —
    ///         h→g 过\u{1}车站 native 无 实证排除）;
    ///   尾槽  单字多读（长 chang/zhang）→ 面不生效（feijichang native
    ///         无面; 独读 管/到/机 照发 — 图书\u{1}管 在板 实证）;
    ///   joined 强词典词（f≠0: 飞机\u{1}场=飞机场 / 电影\u{1}票=电影票 /
    ///         电视\u{1}机=电视机）→ 该路径死（native 无面 实证;
    ///         joined f0 照发 = 词条隐没态代表 — 明天\u{1}下午 native #1）。
    /// 面选择: 双字头优先, 再按路径分（tushuguan: 图书\u{1}管(双字头) >
    /// 图\u{1}数管 — native #2 逐位; dianshiji 仅 点\u{1}实际 存活）。
    /// B 面 = 选中路径单字头 × 多音节尾 × 尾列次词(TAIL_ALT_GAP 带内)
    /// join 面（dianshiji → 点十几 native #3; joined f17 抑制）。（py
    /// `_n5a_covl_faces`）
    fn n5a_covl_faces(
        &self,
        inp: &str,
        seg: &(Vec<String>, f64),
    ) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let paths: Vec<Vec<(usize, usize, u32, u32)>> = self
            .kbest_paths(inp, 4)
            .into_iter()
            .filter(|p| p.len() == 2)
            .take(3)
            .collect();
        if paths.is_empty() {
            return out;
        }
        let reading = seg.0.join("'");
        let syls = self.sylset_m();
        // faces: (head_len, -lp, face, lp, path, kbn_head)
        let mut faces: Vec<(
            usize,
            f64,
            String,
            f64,
            Vec<(usize, usize, u32, u32)>,
            bool,
        )> = Vec::new();
        for p in &paths {
            let h = &p[0];
            let t = &p[1];
            let h_letters = &inp[h.0..h.1];
            let h_single = !self.rid_pretty(h.2).contains('\'');
            let h_prim = h_single
                && self.w2r.get(h.3 as usize) as i64 - 1 == h.2 as i64;
            let mut lp_pen = 0.0f64;
            let mut kbn_head = false;
            // py: head_text 初值 = word_text(h[3]), KBN 头替域整支作废
            let mut head_opt: Option<String> = if h_single && !h_prim {
                None
            } else {
                Some(self.word_text(h.3))
            };
            if h_single && !h_prim {
                'kbn: for p_ in 1..h_letters.len() {
                    for nb in kbn_neighbors(h_letters.as_bytes()[p_]) {
                        let mut h2b = h_letters.as_bytes().to_vec();
                        h2b[p_] = nb;
                        let h2 = String::from_utf8_lossy(&h2b).into_owned();
                        if !syls.contains(&h2) {
                            continue;
                        }
                        let st2 = self.dat.walk(h2.as_bytes(), ROOT);
                        if st2 < 0 || !self.dat.is_terminal(st2) {
                            continue;
                        }
                        let lo2 = self.dat.range(st2).0;
                        let (o02, o12) = self.reading_slice(lo2);
                        if o12 <= o02 {
                            continue;
                        }
                        let hw = self.wlist_at(o02);
                        if self.w2r.get(hw as usize) as i64 - 1
                            != lo2 as i64
                        {
                            continue;
                        }
                        match n5a_slot_head(&h2) {
                            Some(cal) => {
                                let ok = match self.word_id(cal) {
                                    Some(cw) => {
                                        self.w2r.get(cw as usize) as i64
                                            - 1
                                            == lo2 as i64
                                    }
                                    None => false,
                                };
                                if !ok {
                                    continue;
                                }
                                head_opt = Some(cal.to_string());
                            }
                            None => head_opt = Some(self.word_text(hw)),
                        }
                        lp_pen = -EDIT_SUB;
                        kbn_head = true;
                        break 'kbn;
                    }
                }
                if head_opt.is_none() {
                    continue;
                }
            }
            let head = head_opt.unwrap();
            let t_text = self.word_text(t.3);
            if t_text.chars().count() == 1
                && self.n5a_char_alt_reading(t.3, &inp[t.0..t.1])
            {
                continue;
            }
            if t_text.chars().count() > 1
                && self.word_logp(t.3) < N5A_FACE_TAIL_LP
            {
                continue; // 尾槽 junk 词面（图数管/点影票 native 全无）
            }
            let joined = format!("{}{}", head, t_text);
            if let Some(wj) = self.word_id(&joined) {
                if (wj as usize) < self.n_words
                    && self._flags[wj as usize] != 0
                {
                    continue;
                }
            }
            let lp: f64 = p
                .iter()
                .map(|s| self.slot_logp(s.3, self.reading_slice(s.2).0))
                .sum::<f64>()
                + lp_pen;
            faces.push((
                head.chars().count(),
                -lp,
                format!("{}\u{1}{}", head, t_text),
                lp,
                p.clone(),
                kbn_head,
            ));
        }
        if faces.is_empty() {
            return out;
        }
        // py sort key=(0 if head_len>=2 else 1, -lp)——双字头优先再按路径分
        faces.sort_by(|a, b| {
            let ka = if a.0 >= 2 { 0 } else { 1 };
            let kb = if b.0 >= 2 { 0 } else { 1 };
            ka.cmp(&kb).then_with(|| a.1.partial_cmp(&b.1).unwrap())
        });
        let (_hl, _neg, face, lp, p, kbn_head) = faces.swap_remove(0);
        let mut c = Cand::new(face, lp, Kind::Pending);
        c.reading = reading.clone();
        out.push(c);
        // —— B 面: 选中路径 单字头(非 KBN) × 多音节尾 × 尾列次词 ——
        if p.len() == 2 && !kbn_head {
            let ht = self.word_text(p[0].3);
            if ht.chars().count() == 1
                && self.rid_pretty(p[1].2).contains('\'')
            {
                let lo_t = p[1].2;
                let (o0t, o1t) = self.reading_slice(lo_t);
                if o1t - o0t >= 2 {
                    let cur = self.slot_logp(p[1].3, o0t);
                    let alt_w = self.wlist_at(o0t + 1);
                    let alt_lp = self.slot_logp(alt_w, o0t + 1);
                    if alt_lp >= cur - TAIL_ALT_GAP {
                        let bface =
                            format!("{}{}", ht, self.word_text(alt_w));
                        let ok = match self.word_id(&bface) {
                            None => true,
                            Some(wbj) => {
                                self._flags[wbj as usize] == 0
                            }
                        };
                        if ok {
                            let mut bc = Cand::new(
                                bface,
                                lp - cur + alt_lp,
                                Kind::Pending,
                            );
                            bc.reading = reading;
                            out.push(bc);
                        }
                    }
                }
            }
        }
        out
    }

    /// N5a: 首音节字列（贪婪首音节读音列, 主读音字优先 — 和(w2r=he)
    /// 伪槽位排除, native huo 列 或货活火霍 逐位; oi 序, fv 地板门）。
    /// （py `_n5a_char_col`）
    fn n5a_char_col(
        &self,
        letters: &str,
        out: &mut Vec<Cand>,
        seen_w: &mut HashSet<u32>,
    ) {
        let stx = self.dat.walk(letters.as_bytes(), ROOT);
        if stx < 0 || !self.dat.is_terminal(stx) {
            return;
        }
        let lo3 = self.dat.range(stx).0;
        let (o0c, o1c) = self.reading_slice(lo3);
        let mut em = 0usize;
        for oi in o0c..o1c.min(o0c + 24) {
            let w = self.wlist_at(oi);
            if seen_w.contains(&w) {
                continue;
            }
            if self.w2r.get(w as usize) as i64 - 1 != lo3 as i64 {
                continue;
            }
            if !Self::panel_fv_ok(self.slot_fv(oi)) {
                continue;
            }
            seen_w.insert(w);
            out.push(self.mk(
                w,
                lo3 as i64,
                Kind::Pending,
                0,
                None,
                0.0,
                self.slot_fv(oi),
            ));
            em += 1;
            if em >= N5A_COVL_CHAR_CAP {
                break;
            }
        }
    }

    /// N5a: 深链词组 = ext 兄弟补全词(1) + 词列。
    /// ext: 子树内非链节点读音的最强 f≠0 词（lp > bar; dianshi 子树
    /// 电视剧 −4.40 native #4; 电视家/点时间 f0/弱词排除）。音节对齐门:
    /// ext 读音须以贪婪前段(all-but-last 音节)起头且自切音节数 == 输入
    /// 音节数（dianshi|ju ✓; fei|jin(费劲) 2≠3 音节 native 无 实证排除;
    /// 明天吧 3≠4 同律）。
    /// 词列: f≠0 (−lp,oi) 前 + f0(lp > bar) 补位（dian'shi: 电视|点事|
    /// 点石|电石|电势 + 点是; ming'tian: 明天|茗天, 鸣天/名天/铭天 f0 深
    /// 词不入 — native 逐位）。（py `_n5a_word_group`; bset 形参未消费,
    /// rust 侧省略）
    fn n5a_word_group(
        &self,
        letters: &str,
        _rid: u32,
        chain_letters: &HashSet<String>,
        seen_w: &mut HashSet<u32>,
        gpre: &str,
        ngre: usize,
    ) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let stx = self.dat.walk(letters.as_bytes(), ROOT);
        if stx < 0 || !self.dat.is_terminal(stx) {
            return out;
        }
        let lo = self.dat.range(stx).0;
        let (o0, o1) = self.reading_slice(lo);
        // —— ext 兄弟补全词 ——
        let (lo_e, cnt_e) = self.dat.range(stx);
        let mut best: Option<(f64, u32, u32, usize)> = None;
        for r in lo_e..(lo_e + cnt_e).min(lo_e + 96) {
            if r == lo {
                continue;
            }
            let info = self.rid_syls(r);
            if info.is_empty() {
                continue;
            }
            let rl: String = info.concat();
            if !rl.starts_with(letters) || chain_letters.contains(&rl) {
                continue;
            }
            if ngre != 0 {
                if !rl.starts_with(gpre) {
                    continue;
                }
                let s2 = self.segment(&rl);
                match s2 {
                    Some(s2) if s2.0.len() == ngre => {}
                    _ => continue,
                }
            }
            let (r0, r1) = self.reading_slice(r);
            for oi in r0..r1.min(r0 + 4) {
                let w = self.wlist_at(oi);
                if !self.panel_word_ok(w as i64) {
                    continue;
                }
                let lp = self.word_logp(w);
                if lp > N5A_EXT_LP_BAR {
                    let better = match &best {
                        None => true,
                        Some(b) => lp > b.0,
                    };
                    if better {
                        best = Some((lp, w, r, oi));
                    }
                }
                break;
            }
        }
        if let Some(b) = best {
            if !seen_w.contains(&b.1) {
                seen_w.insert(b.1);
                out.push(self.mk(
                    b.1,
                    b.2 as i64,
                    Kind::Pending,
                    0,
                    None,
                    0.0,
                    self.slot_fv(b.3),
                ));
            }
        }
        // —— 词列 ——
        let mut f17: Vec<(f64, u32, usize)> = Vec::new();
        let mut f0: Vec<(usize, u32, f64)> = Vec::new();
        for oi in o0..o1.min(o0 + 24) {
            let w = self.wlist_at(oi);
            if seen_w.contains(&w) {
                continue;
            }
            if self.panel_word_ok(w as i64) {
                f17.push((self.word_logp(w), w, oi));
            } else {
                f0.push((oi, w, self.word_logp(w)));
            }
        }
        if f17.is_empty() {
            return out; // N5a: 全 f0 词组不补位（woai/我爱 型 — 段W 同律）
        }
        let mut entries: Vec<(f64, u32, usize)> = f17.clone();
        entries.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap().then_with(|| a.2.cmp(&b.2))
        });
        // N5a: f0 补位限 oi≥1（oi0 = 列首 — zhegeshiqingbunengban 的
        // 这个事情(oi0 f0 −4.93) native 不入面板 逐位; 观测补位词全 oi1+
        // 点是/任生/我相/我像 逐位）; f17 为 f0 补位让位（dianshi 列
        // native 6 = 电视|点事|点石|电石|电势 + 点是(f0) — 殿试 让位）
        let mut f0s = f0.clone();
        f0s.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let fill: Vec<(f64, u32, usize)> = f0s
            .iter()
            .filter(|(oi, _w, lp)| *oi - o0 >= 1 && *lp > N5A_F0_COL_BAR)
            .take(N5A_F0_COL_CAP)
            .map(|(oi, w, lp)| (*lp, *w, *oi))
            .collect();
        entries.truncate(N5A_COVL_GRP_CAP.saturating_sub(fill.len()));
        entries.extend(fill);
        for (_lp, w, oi) in entries {
            if seen_w.contains(&w) {
                continue;
            }
            seen_w.insert(w);
            out.push(self.mk(
                w,
                lo as i64,
                Kind::Pending,
                0,
                None,
                0.0,
                self.slot_fv(oi),
            ));
        }
        out
    }

    /// N5a: covered 全拼（>8 键）阶梯面板 — native 八案逐位律:
    /// [精确词(f≠0)] → [2-slot 分词面 A/KBN/B] → [深链词组: ext+词列,
    /// 完整切分边界前缀深→浅(deep_chain)] → [首音节字列(贪婪首音节,
    /// 主读音字优先)]。构造序钉分(panel_pin) = 面板序逐位。
    /// 路由门: covered × >8 键 × ≥3 规整音节 × 非 DEEP_ROUTE_COVERED
    /// （ref3 covered 域全在深列路由/≤8 键 — 路由审计 0 交叉实证）。
    /// 整词条频带门: 整串词典词 lp ≤ −5.55 → native 词条原面板(词典频序
    /// — nihaoshijie 你好世界 −5.59 词条域 逐位) → 不走阶梯; > −5.55 →
    /// 切分阶梯面板（电视机 −5.20/图书馆 −4.40/飞机场 −5.51/火车站
    /// −4.46/电影票 −5.42/明天下午 −4.83 全 > −5.55 逐位）。（py
    /// `_q_ladder_panel`）
    pub(super) fn q_ladder_panel(&self, inp: &str) -> Option<Vec<Cand>> {
        let n = inp.len();
        if n <= N5A_COVL_MAX_KEYS {
            return None;
        }
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 < 0 || !self.dat.is_terminal(st0) {
            return None;
        }
        let lo0 = self.dat.range(st0).0;
        if deep_route_covered(&self.rid_pretty(lo0)) {
            return None;
        }
        let (o0w, _o1w) = self.reading_slice(lo0);
        if self.word_logp(self.wlist_at(o0w)) <= N5A_COVL_WORD_LP {
            return None; // 整词条频带门（nihaoshijie 词条域）
        }
        let seg = self.segment(inp)?;
        if !seg.0.iter().all(|s| s.len() >= 2)
            || seg.0.len() < N5A_COVL_MIN_SYL
        {
            return None;
        }
        let mut out: Vec<Cand> = Vec::new();
        let mut seen_w: HashSet<u32> = HashSet::new();
        // —— 1 精确词 (own-reading, f≠0) ——
        let (o0, o1) = self.reading_slice(lo0);
        for oi in o0..o1.min(o0 + N5A_COVL_EXACT_CAP) {
            let w = self.wlist_at(oi);
            if seen_w.contains(&w) || !self.panel_word_ok(w as i64) {
                continue;
            }
            seen_w.insert(w);
            out.push(self.mk(
                w,
                lo0 as i64,
                Kind::Pending,
                0,
                None,
                0.0,
                self.slot_fv(oi),
            ));
        }
        // —— 2 分词面 ——
        out.extend(self.n5a_covl_faces(inp, &seg));
        // —— 3 深链阶梯 ——
        let first_syl: Option<String> = {
            let syls = self.sylset_m();
            let mut got = None;
            for l in (2..=(6usize).min(n)).rev() {
                if syls.contains(&inp[..l]) {
                    got = Some(inp[..l].to_string());
                    break;
                }
            }
            got
        };
        let mut bounds: Vec<usize> =
            self.greedy_bounds(inp).into_iter().collect();
        bounds.sort_unstable();
        let gpre: String = if bounds.len() >= 2 {
            let gsyls: Vec<&str> = bounds
                .windows(2)
                .map(|w| &inp[w[0]..w[1]])
                .collect();
            gsyls[..gsyls.len() - 1].concat()
        } else {
            String::new()
        };
        let ngre = bounds.len().saturating_sub(1);
        let chain = self.deep_chain(inp);
        let mut chain_letters: HashSet<String> = HashSet::new();
        chain_letters.insert(inp.to_string());
        for (l, _r) in &chain {
            chain_letters.insert(l.clone());
        }
        for (letters, rid) in chain {
            if letters == inp {
                continue; // 整串节点 = 精确词组
            }
            if let Some(fs) = &first_syl {
                if letters.len() <= fs.len() {
                    if letters == *fs {
                        self.n5a_char_col(&letters, &mut out, &mut seen_w);
                    }
                    continue; // 更短链节点（hu 型）native 无 → 不发
                }
            }
            out.extend(self.n5a_word_group(
                &letters,
                rid,
                &chain_letters,
                &mut seen_w,
                &gpre,
                ngre,
            ));
        }
        if out.is_empty() {
            return None;
        }
        self.panel_pin(&mut out, inp);
        Some(out)
    }

    /// 段P② 尾悬空声母/残串补全面（律 2）: 头 munch 槽 × 补全字。
    /// 补全音节集 = 尾串精确音节(迷,mi ×2 字) ∪ KBN 邻键(你,ni) ∪
    /// 首字母栈(he/hao/hai); 头槽 munch = 长/短(最短强词)两种槽形。
    /// 排序 = (缺字母数 asc, P(字|头上下文) NNLM desc) — chengqunh
    /// he(1)<hao(2) native 和>好>还 骨架 + NNLM 同级定序。门: 头槽
    /// 须含强词 (成群/今天/中国人 — zhongguor 中|过 无 → 无面)。
    /// 融合: 补全字融进尾单字槽 (今天\x01的+天 → 今天\x01的天)。（py
    /// `_panel_completion_faces`）
    pub(super) fn panel_completion_faces(&self, inp: &str) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 >= 0 && self.dat.is_terminal(st0) {
            return out; // 覆盖态不发
        }
        let maxb = self
            .greedy_bounds(inp)
            .into_iter()
            .max()
            .unwrap_or(0);
        let tail = &inp[maxb..];
        if tail.is_empty() || tail.len() > 3 {
            return out;
        }
        let syls_m = self.sylset_m();
        let mut cands_syl: Vec<String> = Vec::new();
        let exact_tail = syls_m.contains(tail);
        if exact_tail {
            cands_syl.push(tail.to_string());
            let tb = tail.as_bytes();
            for p in 0..tb.len() {
                for nb in kbn_neighbors(tb[p]) {
                    let mut tv = tb.to_vec();
                    tv[p] = nb;
                    let tvs = String::from_utf8_lossy(&tv).into_owned();
                    if syls_m.contains(&tvs) && tvs != tail
                        && !cands_syl.contains(&tvs)
                    {
                        cands_syl.push(tvs);
                    }
                }
            }
        } else {
            let ini = tail.as_bytes()[0];
            // py: 首字母栈（_fb_ini_stack 过滤 ini 键）头段前缀补全
            let stack = self.fb_ini_stack();
            let v: Vec<String> = stack
                .get(&ini)
                .cloned()
                .unwrap_or_default();
            drop(stack);
            cands_syl = v
                .into_iter()
                .filter(|s| s.starts_with(tail))
                .take(8)
                .collect();
            if cands_syl.is_empty() {
                cands_syl = self
                    .syllables
                    .iter()
                    .filter(|s| s.starts_with(tail))
                    .take(8)
                    .cloned()
                    .collect();
            }
        }
        let mut pool: Vec<(String, i64, String)> = Vec::new();
        for s in cands_syl.iter().take(6) {
            let ks: &[usize] =
                if exact_tail && tail.len() > 2 { &[0] } else { &[0, 1] };
            for &k in ks {
                if let Some(sc) = self.panel_syl_char(s, k) {
                    pool.push((sc.0, sc.1, s.clone()));
                }
            }
        }
        let wbd = self
            .greedy_bounds(inp)
            .into_iter()
            .filter(|&b| 0 < b && b <= 8)
            .max()
            .unwrap_or(0);
        let win_word =
            if wbd >= 2 { self.panel_slot_word(&inp[..wbd]) } else { None };
        // face_cands: (face, lp, miss, head_best)
        let mut face_cands: Vec<(String, f64, usize, f64)> = Vec::new();
        let mut merged_set: HashSet<String> = HashSet::new();
        let mut merged_head: HashMap<String, String> = HashMap::new();
        for mode in ["long", "short"] {
            let slots = match self.panel_left_munch(inp, mode) {
                Some(s)
                    if s.iter().map(|x| x.2.len()).sum::<usize>() == maxb =>
                {
                    s
                }
                _ => continue,
            };
            let n_word = slots
                .iter()
                .filter(|s| s.1 >= 0 && self.panel_word_ok(s.1))
                .count();
            if n_word < 1 {
                continue; // 头槽无强词 → 无完成面 (zhongguor 中|过)
            }
            let texts: Vec<String> =
                slots.iter().map(|s| s.0.clone()).collect();
            let wids: Vec<i64> = slots.iter().map(|s| s.1).collect();
            let head_text =
                self.panel_fuse(&texts, &wids, &slots, win_word);
            let mut head_best = -99.0f64;
            for s in &slots {
                if s.1 >= 0 && self.panel_word_ok(s.1) {
                    head_best = head_best.max(self.word_logp(s.1 as u32));
                }
            }
            for (ch, _w, syl) in &pool {
                let face;
                if slots[slots.len() - 1].0.chars().count() == 1 {
                    // W2 律 (native 逐位): 尾单字槽读音 == 补全音节 →
                    // 字替换槽 (中国人|米 × mi 列 → 中国人\x01迷);
                    // 否则字并槽 (今天|的 × t→tian → 今天\x01的天 —
                    // native jin,tian#de,tian 逐位)。
                    if slots.len() >= 2 {
                        let texts2: Vec<String> =
                            slots[..slots.len() - 1]
                                .iter()
                                .map(|s| s.0.clone())
                                .collect();
                        let wids2: Vec<i64> =
                            slots[..slots.len() - 1]
                                .iter()
                                .map(|s| s.1)
                                .collect();
                        let h2 = self.panel_fuse(
                            &texts2, &wids2, &slots[..slots.len() - 1],
                            win_word,
                        );
                        let last = &slots[slots.len() - 1];
                        if last.2 == *syl {
                            face = if h2.is_empty() {
                                ch.clone()
                            } else {
                                format!("{}\u{1}{}", h2, ch)
                            };
                        } else {
                            face = if h2.is_empty() {
                                format!("{}{}", last.0, ch)
                            } else {
                                format!("{}\u{1}{}{}", h2, last.0, ch)
                            };
                            merged_set.insert(face.clone());
                            merged_head.insert(
                                face.clone(),
                                if h2.is_empty() {
                                    last.0.clone()
                                } else {
                                    format!("{}\u{1}{}", h2, last.0)
                                },
                            );
                        }
                    } else {
                        face = format!("{}{}", head_text, ch);
                        merged_set.insert(face.clone());
                    }
                } else {
                    face = format!("{}\u{1}{}", head_text, ch);
                }
                let miss = syl.len() - tail.len();
                let lp = self.panel_face_lp(&slots)
                    + self.panel_slot_lp(ch);
                face_cands.push((face, lp, miss, head_best));
            }
        }
        if face_cands.is_empty() {
            return out;
        }
        // W2: 双槽形 (long/short) 常产出同面 — 去重后再入 NNLM 批
        // （批内重复项会位移 batch 分 — 今天的天 −5.84 被重复项挤到
        // 他 −4.83 之后的实证）
        {
            let mut seen_fc: HashSet<String> = HashSet::new();
            let mut uniq_fc: Vec<(String, f64, usize, f64)> =
                Vec::new();
            for fc in face_cands.into_iter() {
                if seen_fc.contains(&fc.0) {
                    continue;
                }
                seen_fc.insert(fc.0.clone());
                uniq_fc.push(fc);
            }
            face_cands = uniq_fc;
        }
        let ranked: Vec<(String, f64)>;
        if exact_tail {
            ranked = face_cands
                .iter()
                .map(|(f, lp, _m, _hb)| (f.clone(), *lp))
                .collect();
        } else {
            // scored: (nn, face, lp, miss, head_best)
            let mut scored: Vec<(f64, String, f64, usize, f64)> =
                Vec::new();
            let _ = self.nnlm_ok();
            if let Some(m) = self.nnlm_model() {
                // W2: 续接上下文 = 面串去尾字（今天的天 → ctx 今天的 +
                // ch 天; 成群和 → ctx 成群 + ch 和 — 旧 by_head 取
                // 首 \x01 段使并槽面 ctx=今天, 天/他 翻序实证）
                let mut by_head: Vec<(String, Vec<usize>)> = Vec::new();
                for (idx, fc) in face_cands.iter().enumerate() {
                    // py face[:-1] = 去尾【字符】（字节切片会切进多字节字）
                    let h: String = fc.0
                        .chars()
                        .take(fc.0.chars().count() - 1)
                        .collect::<String>()
                        .replace('\u{1}', "");
                    if let Some(e) =
                        by_head.iter_mut().find(|(hh, _)| *hh == h)
                    {
                        e.1.push(idx);
                    } else {
                        by_head.push((h, vec![idx]));
                    }
                }
                for (h, items) in &by_head {
                    let ctx = m.encode(h);
                    let mut seqs: Vec<Vec<u32>> = Vec::new();
                    let mut keep: Vec<usize> = Vec::new();
                    for &idx in items {
                        let f = &face_cands[idx].0;
                        let ch: String =
                            f.chars().last().map(|c| c.to_string())
                                .unwrap_or_default();
                        if let Some(s) = m.encode(&ch) {
                            seqs.push(s);
                            keep.push(idx);
                        }
                    }
                    if !seqs.is_empty() {
                        let empty: Vec<u32> = Vec::new();
                        let sc = m.score_candidates(
                            ctx.as_deref().unwrap_or(&empty),
                            &seqs,
                        );
                        for (i, &idx) in keep.iter().enumerate() {
                            let (f, lp, miss, hbl) = &face_cands[idx];
                            scored.push((
                                sc[i], f.clone(), *lp, *miss, *hbl,
                            ));
                        }
                    }
                }
            }
            if !scored.is_empty() {
                // W2: 并槽面（的X 型）= NNLM 续接定序取顶1（今天的天 −5.84
                // 压 他 −6.67/太 −6.48 — miss 序给不出 native 天-only）;
                // 并槽头弱词门（中古 −5.47 ✗ / 今天 −2.41 ✓ — native
                // zhongguor 无 中古|哦人 面）;
                // 追加面（成群X 型）= 既有 (miss, −NN, −lp) 序 cap3。
                let head_ok = |f: &String| -> bool {
                    match merged_head.get(f) {
                        None => true,
                        Some(hd) => {
                            for pt in hd.split('\u{1}') {
                                if let Some(pw) = self.word_id(pt) {
                                    if self.word_logp(pw) < -5.0 {
                                        return false;
                                    }
                                }
                            }
                            true
                        }
                    }
                };
                let mut merged_items: Vec<
                    (f64, String, f64, usize, f64),
                > = scored
                    .iter()
                    .filter(|t| merged_set.contains(&t.1)
                        && head_ok(&t.1))
                    .cloned()
                    .collect();
                let mut append_items: Vec<
                    (f64, String, f64, usize, f64),
                > = scored
                    .iter()
                    .filter(|t| !merged_set.contains(&t.1))
                    .cloned()
                    .collect();
                let mut ranked_v: Vec<(String, f64)> = Vec::new();
                if !merged_items.is_empty() {
                    // py key=(-nn, -head_best)
                    merged_items.sort_by(|a, b| {
                        b.0.partial_cmp(&a.0)
                            .unwrap()
                            .then_with(|| b.4.partial_cmp(&a.4).unwrap())
                    });
                    for (_s, f, lp, _m2, _hb) in
                        merged_items.into_iter().take(1)
                    {
                        ranked_v.push((f, lp));
                    }
                    // py key=(miss, -nn, -head_best)
                    append_items.sort_by(|a, b| {
                        a.3.cmp(&b.3)
                            .then_with(|| {
                                b.0.partial_cmp(&a.0).unwrap()
                            })
                            .then_with(|| {
                                b.4.partial_cmp(&a.4).unwrap()
                            })
                    });
                    for (_s, f, lp, _m2, _hb) in
                        append_items.into_iter().take(2)
                    {
                        ranked_v.push((f, lp));
                    }
                } else {
                    append_items.sort_by(|a, b| {
                        a.3.cmp(&b.3)
                            .then_with(|| {
                                b.0.partial_cmp(&a.0).unwrap()
                            })
                            .then_with(|| {
                                b.4.partial_cmp(&a.4).unwrap()
                            })
                    });
                    for (_s, f, lp, _m2, _hb) in
                        append_items.into_iter().take(3)
                    {
                        ranked_v.push((f, lp));
                    }
                }
                ranked = ranked_v;
            } else {
                let mut v: Vec<(String, f64)> = face_cands
                    .iter()
                    .map(|(f, lp, _m, _hb)| (f.clone(), *lp))
                    .collect();
                v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                ranked = v.into_iter().take(3).collect();
            }
        }
        let mut seen: HashSet<String> = HashSet::new();
        for (face, lp) in ranked.into_iter().take(3) {
            if seen.contains(&face) {
                continue;
            }
            seen.insert(face.clone());
            let mut c = Cand::new(face.clone(), lp, Kind::Pending);
            if let Some(hd) = merged_head.get(&face) {
                c.mhead = Some(hd.clone());
            }
            out.push(c);
        }
        out
    }

    /// NNLM 续接分排序（native 成群和>好>还 / 今天的天>他 — 与
    /// 显示面板同源模型）。NNLM 不可用 → 栈序原样。（py
    /// `_panel_nnlm_rank`）
    #[allow(dead_code)]
    pub(super) fn panel_nnlm_rank(
        &self,
        ctx_text: &str,
        pool: &[(String, i64)],
    ) -> Vec<(String, i64)> {
        let mut ranked = pool.to_vec();
        let _ = self.nnlm_ok();
        if let Some(m) = self.nnlm_model() {
            let ctx = if ctx_text.is_empty() {
                None
            } else {
                m.encode(ctx_text)
            };
            let mut seqs: Vec<Vec<u32>> = Vec::new();
            let mut keep: Vec<(String, i64)> = Vec::new();
            for (ch, w) in pool {
                if let Some(s) = m.encode(ch) {
                    seqs.push(s);
                    keep.push((ch.clone(), *w));
                }
            }
            if !seqs.is_empty() {
                let empty: Vec<u32> = Vec::new();
                let scores = m.score_candidates(
                    ctx.as_deref().unwrap_or(&empty),
                    &seqs,
                );
                let mut ord: Vec<usize> = (0..keep.len()).collect();
                ord.sort_by(|&a, &b| {
                    scores[b]
                        .partial_cmp(&scores[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                ranked = ord.into_iter().map(|i| keep[i].clone()).collect();
            }
        }
        ranked
    }

    /// 全面文本 NNLM 续接分排序（完成面槽形/字池定序）（py
    /// `_panel_nnlm_rank_faces`）
    #[allow(dead_code)]
    pub(super) fn panel_nnlm_rank_faces(
        &self,
        face_cands: &[(String, f64)],
    ) -> Vec<(String, f64)> {
        let _ = self.nnlm_ok();
        if let Some(m) = self.nnlm_model() {
            let mut seqs: Vec<Vec<u32>> = Vec::new();
            let mut keep: Vec<(String, f64)> = Vec::new();
            for (face, lp) in face_cands {
                let flat = face.replace('\u{1}', "");
                if let Some(s) = m.encode(&flat) {
                    seqs.push(s);
                    keep.push((face.clone(), *lp));
                }
            }
            if !seqs.is_empty() {
                let empty: Vec<u32> = Vec::new();
                let scores = m.score_candidates(&empty, &seqs);
                let mut ord: Vec<usize> = (0..keep.len()).collect();
                ord.sort_by(|&a, &b| {
                    scores[b]
                        .partial_cmp(&scores[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                return ord.into_iter().map(|i| keep[i].clone()).collect();
            }
        }
        face_cands.to_vec()
    }

    /// W2: 面板域 fz 面质门 v2（native 逐位归纳, 校准锚 15+）:
    /// (i)  单字头吞窗不发（我\x01爱中国 — 窗词被头吞并型, native 无;
    ///      但|超模 单字头 但=exact 首音节字 在板 → 限「窗词在余部」）;
    /// (ii) 弱兜底头不发（多凶 f0−5.69/我四 f0−4.78/今天的 f0−5.43/
    ///      酸能 f0−5.34 kill; 我爱 f0−4.52/但 f0−3.40 在板）;
    /// (iii) 编辑面 NNLM < −27 不发（缩胸批啊 −35.4 kill; 尿素换
    ///      −23.4/丢光炸 −26.0 在板）;
    /// (iv) 弱头编辑面 NNLM < −23.5 不发（我气中国 −24.0 kill;
    ///      但超模 −23.0 edit0 不入此门 在板）;
    /// 并槽头同界面顶替（今天的他 vs 并槽面 今天的天 同头 '今天\x01的'
    /// — native 唯 天 面）。出口 cap=3（丢黄炸/丢光炸/丢光扎 三面）。
    /// （py `_panel_fz_filter`）
    pub(super) fn panel_fz_filter(
        &self,
        fz: Vec<Cand>,
        inp: &str,
        have_faces: &[Cand],
    ) -> Vec<Cand> {
        let mut win_text: Option<String> = None;
        if let Some(wwid) = self.panel_win_word(inp) {
            if wwid > 0 {
                win_text = Some(self.word_text(wwid));
            }
        }
        let mut mheads: HashSet<String> = HashSet::new();
        for c0 in have_faces {
            if let Some(mh) = &c0.mhead {
                mheads.insert(mh.clone());
            }
        }
        let mut scores: HashMap<String, f64> = HashMap::new();
        let _ = self.nnlm_ok();
        if let Some(m) = self.nnlm_model() {
            let mut seqs: Vec<Vec<u32>> = Vec::new();
            let mut keep: Vec<String> = Vec::new();
            for c in &fz {
                if c.word.contains('\u{1}') {
                    if let Some(s) =
                        m.encode(&c.word.replace('\u{1}', ""))
                    {
                        seqs.push(s);
                        keep.push(c.word.clone());
                    }
                }
            }
            if !seqs.is_empty() {
                let empty: Vec<u32> = Vec::new();
                let sc = m.score_candidates(&empty, &seqs);
                for (k, v) in keep.iter().zip(sc.iter()) {
                    scores.insert(k.clone(), *v);
                }
            }
        }
        let mut out: Vec<Cand> = Vec::new();
        for c in fz {
            let w = c.word.clone();
            if !w.contains('\u{1}') {
                out.push(c);
                continue;
            }
            let parts: Vec<&str> = w.split('\u{1}').collect();
            let head = parts[0];
            let hw = self.word_id(head);
            let hfl = match hw {
                Some(h) if (h as usize) < self._flags.len() => {
                    self._flags[h as usize]
                }
                _ => 0,
            };
            let hlp = match hw {
                Some(h) => self.word_logp(h),
                None => -9.0,
            };
            if head.chars().count() == 1 {
                if let Some(wt) = &win_text {
                    // py w[len(head)+1:] = 字符域切片（head 1 字 + \x01 之后）
                    let hc = head.chars().count() + 1;
                    let rest: String =
                        w.chars().skip(hc).collect();
                    if rest.contains(wt.as_str()) {
                        continue;
                    }
                }
            }
            if hfl == 0 && hlp < -5.0 {
                continue;
            }
            if let Some(nn) = scores.get(&w) {
                if c.edit_penalty != 0.0 && *nn < -27.0 {
                    continue;
                }
                if hfl == 0 && c.edit_penalty != 0.0 && *nn < -23.5 {
                    continue;
                }
            }
            if !mheads.is_empty() {
                // py w.rsplit('\x01', 1)[0] = 末段之前的全部（同头判定）
                let head_key = match w.rfind('\u{1}') {
                    Some(ix) => w[..ix].to_string(),
                    None => w.clone(),
                };
                if mheads.contains(&head_key) {
                    continue;
                }
            }
            out.push(c);
        }
        out.truncate(3);
        out
    }

    // ------------------------------------------------------------------
    // 役O1（第七波 O1: 读音列深度）→ 第八波(结构泛化)
    // 原 _O1_PANELS 查询键整面板 + _q_long_panel/_q_full 白名单门, 泛化为
    // §1 读音列深度律（native 对 exact/近 exact 输入的面板 = 前缀链读音
    // 节点列深枚举, 无外推垫位——dangling_slot_law.md §1/§5/§6）:
    //   链律  deep_chain: 完整音节切分边界上的读音终端节点（存在完整切分
    //         使该前缀为边界）, 长→短。zaoshanghao → [zaoshanghao,
    //         zaoshang, zao, za]（zaoshan/zaosha 非任何完整切分边界 →
    //         不在链; za = za|o|shang|hao 边界; tianqizenmeyang 的 ti =
    //         ti|an|qi|… 边界, tia 非边界 → 不在链——154 集逐案归纳）。
    //   路由  覆盖态: 读音 ∈ DEEP_ROUTE_COVERED; >8 键在途: 链节点读音 ∈
    //         DEEP_ROUTE_LONG; 单声母: len==1 × G1_COL; 无音节头短串:
    //         简拼整词 × G1_COL[声母]（jttq 律）。路由键与列同表族校准
    //         （constants RD_COL/DEEP_ROUTE_*; native 触发数据在 native
    //         运行时词典, wxq.bin 不可见 → 读音键校准为合法形态,
    //         FB_INI_COL['n'] 先例）。
    //   列    RD_COL（全读音列, pretty 键）/ WORD_COL（祖先读音词列）/
    //         CHAR_COL（单音节字列）/ G1_COL（声母栈列）——校准缺列的链
    //         节点不发（native 列内容在 native 运行时词典, wxq.bin 不可见
    //         的部分只能校准; 154 集深列路由查询的链节点全有校准列,
    //         自然列零接触）。
    //   面板  列拼接（无外推垫位: 早上好呀/没关系的 族, 错读音混入
    //         本钢/被·品, 短语垫位 今天他去 native 全无）+ 构造序钉分
    //         （panel_pin, 序 = native 面板逐位）。11 案基线 top-1 与
    //         native 全一致 → 零翻锚。
    // ------------------------------------------------------------------

    /// §1 深列链: 完整音节切分边界上的读音终端节点（存在自首至该位、
    /// 自该位至尾的合法音节切分）, 长→短。
    /// zaoshanghao → [zaoshanghao, zaoshang, zao, za]: za = za|o|shang|hao
    /// 边界; zaoshan/zaosha 非任何完整切分边界 → 不在链（zao|shan 后余
    /// ghao 无切分）。tianqizenmeyang → [tianqi, tian, ti]: ti =
    /// ti|an|qi|zen|me|yang 边界; tia 非音节边界 → 不在链。
    /// 154 集 11 深列案逐位归纳; 护栏锚（nihao/xian/zhuomian）链形不变
    /// （列消费仅限路由键命中, 见 q_deep_panel/q_deep_long_panel）。
    /// （py `_deep_chain`）
    pub(super) fn deep_chain(&self, inp: &str) -> Vec<(String, u32)> {
        let n = inp.len();
        let syls = self.sylset_m();
        let mut reach = vec![false; n + 1];
        reach[0] = true;
        for i in 0..n {
            if !reach[i] {
                continue;
            }
            for l in 1..=(6usize).min(n - i) {
                if syls.contains(&inp[i..i + l]) {
                    reach[i + l] = true;
                }
            }
        }
        let mut fin = vec![false; n + 1];
        fin[n] = true;
        for i in (0..n).rev() {
            for l in 1..=(6usize).min(n - i) {
                if fin[i + l] && syls.contains(&inp[i..i + l]) {
                    fin[i] = true;
                    break;
                }
            }
        }
        let mut out: Vec<(String, u32)> = Vec::new();
        for l in (1..=n).rev() {
            if !(reach[l] && fin[l]) {
                continue;
            }
            let st = self.dat.walk(inp[..l].as_bytes(), ROOT);
            if st >= 0 && self.dat.is_terminal(st) {
                let (lo, _cnt) = self.dat.range(st);
                out.push((inp[..l].to_string(), lo));
            }
        }
        out
    }

    /// covered 前缀链: 整串读音节点 + 全部终端读音前缀, 长→短
    /// （第九波 T1: native covered 面板 = 词条 + 头音节前缀链字列——
    /// zenme=[zen'me,zen,ze] / cengjing=[ceng'jing,ceng,cen,ce] /
    /// liusi=[liu'si,liu,li] / geiguo=[gei'guo,gei,ge] / duibuqi=
    /// [dui'bu'qi,dui'bu,dui] / cen=[cen,ce] 逐位归纳）。与 §1 在途链
    /// （deep_chain）的差别: 前缀不要求完整切分边界（cen 的 ce 无
    /// za|o 型边界也发; native zenme 行 10-85 = ze 列 74 字实证）。
    /// （py `_deep_chain_covered`）
    fn deep_chain_covered(&self, inp: &str) -> Vec<(String, u32)> {
        let mut out: Vec<(String, u32)> = Vec::new();
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 >= 0 && self.dat.is_terminal(st0) {
            let (lo, _) = self.dat.range(st0);
            out.push((inp.to_string(), lo));
        }
        // py: range(len(inp)-1, 1, -1) = 前缀长 len-1 → 2
        for l in (2..inp.len()).rev() {
            let st = self.dat.walk(inp[..l].as_bytes(), ROOT);
            if st >= 0 && self.dat.is_terminal(st) {
                let (lo, _) = self.dat.range(st);
                out.push((inp[..l].to_string(), lo));
            }
        }
        out
    }

    /// 校准列 → fbcol 行（词典缺失字面直发——native 在板逐位先例:
    /// 忠国/井栏/wen/Tai/Yin; 文本去重, 构造序保持）。（py `_deep_emit_col`）
    fn deep_emit_col(
        &self,
        col: &[&str],
        out: &mut Vec<Cand>,
        seen: &mut HashSet<String>,
    ) {
        for t in col {
            if seen.contains(*t) {
                continue;
            }
            seen.insert((*t).to_string());
            let w = self.panel_wid(t);
            if w >= 0 {
                out.push(self.mk(w as u32, -1, Kind::Fbcol, 0, None, 0.0, None));
            } else {
                // py: Cand(word=t, reading='', logp=-8.0, kind='fbcol', wid=-1)
                out.push(Cand::new((*t).to_string(), -8.0, Kind::Fbcol));
            }
        }
    }

    /// §1 深列面板: 链节点校准列拼接 + 构造序钉分（终值 = BASE −
    /// EPS×构造序, 序 = native 面板逐位; 校准缺列的链节点不发——native
    /// 列内容在 native 运行时词典, wxq.bin 不可见的部分只能校准）。
    /// covered 支: exact 节点走 RD_COL（自身查询词条列）, 前缀节点仅
    /// CHAR_COL/WORD_COL（防 cen 型单音节键在双查询域串列——cen 的
    /// RD_COL 词条列不得混入 cengjing 面板, native cengjing 行 11-15 =
    /// 纯 cen 字列 逐位实证）。
    /// （py `_deep_panel_rows`）
    fn deep_panel_rows(&self, inp: &str, covered: bool) -> Option<Vec<Cand>> {
        let mut out: Vec<Cand> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let chain = if covered {
            self.deep_chain_covered(inp)
        } else {
            self.deep_chain(inp)
        };
        for (ci, (letters, rid)) in chain.into_iter().enumerate() {
            let col = if covered && ci > 0 {
                char_col_get(&letters)
                    .or_else(|| word_col_get(&letters))
                    .or_else(|| covered_col_local(&letters))
            } else {
                rd_col_get(&self.rid_pretty(rid))
                    .or_else(|| rd_col_local(&self.rid_pretty(rid)))
                    .or_else(|| covered_col_local(&self.rid_pretty(rid)))
                    .or_else(|| char_col_get(&letters))
                    .or_else(|| word_col_get(&letters))
                    .or_else(|| covered_col_local(&letters))
            };
            if let Some(col) = col {
                self.deep_emit_col(col, &mut out, &mut seen);
            }
        }
        if out.is_empty() {
            return None;
        }
        self.panel_pin(&mut out, inp);
        Some(out)
    }

    /// 役O1 → 第八波(结构泛化): 深列面板路由（q_full 顶部, 原
    /// _q_o1_deep_panel 查询键门）:
    ///   (R1) 覆盖态: 读音 ∈ DEEP_ROUTE_COVERED → 链列拼接（zaoshanghao
    ///        族 7 案; native 覆盖态面板 = 前缀链读音列深枚举, 无外推
    ///        垫位 早上好呀/没关系的 族）;
    ///   (R2) 单声母: len==1 × G1_COL → 声母栈列（p）;
    ///   (R3) 无音节头短串 × 简拼整词: 4≤len≤8, 头字母非 v（英文态）,
    ///        无合法音节头, 无 ≥2 键 DAT 前缀段（bfon fo 段 = 律7 域,
    ///        排除）, 简拼展开含首字母串==输入的整词读音 → RD_COL 整词
    ///        列 + G1_COL[声母] 栈列（jttq）。
    /// 表外查询零影响（154 集验证: R1 仅 7 案, R2 仅 p, R3 仅 jttq）。
    /// （py `_q_deep_panel`）
    pub(super) fn q_deep_panel(&self, inp: &str) -> Option<Vec<Cand>> {
        // (R2) 单声母栈
        if inp.len() == 1 {
            let tbl: Vec<&str> = {
                let mut v = g1_col(inp).to_vec();
                if v.is_empty() {
                    v = g1_col_local(inp).to_vec();
                }
                v
            };
            if !tbl.is_empty() {
                let mut out: Vec<Cand> = Vec::new();
                let mut seen: HashSet<String> = HashSet::new();
                self.deep_emit_col(&tbl, &mut out, &mut seen);
                self.panel_pin(&mut out, inp);
                return Some(out);
            }
        }
        // (R3) 无音节头 × 简拼整词 + 声母栈（ref5: 键长门 4→3 — wrz 型
        // 3 键全辅音简拼整词同律; 仅 RD_COL/本地键命中才发, 表外零影响）
        if (3..=8).contains(&inp.len())
            && inp.as_bytes()[0] != b'v'
            && self.fb_first_syl(inp).is_none()
            && !(0..inp.len()).any(|i| {
                (2..=(6usize).min(inp.len() - i))
                    .any(|l| self.dat.walk(inp[i..i + l].as_bytes(), ROOT) >= 0)
            })
        {
            for seq in self.jianpin_keys(inp) {
                let syls: Vec<&str> = seq
                    .iter()
                    .map(|&i| self.syllables[i as usize].as_str())
                    .collect();
                let initials: String = syls
                    .iter()
                    .map(|s| s.as_bytes()[0] as char)
                    .collect();
                if initials != inp {
                    continue;
                }
                let joined = syls.join("'");
                let col = match rd_col_get(&joined) {
                    Some(c) => c,
                    None => match rd_col_local(&joined) {
                        Some(c) => c,
                        None => break,
                    },
                };
                let mut out: Vec<Cand> = Vec::new();
                let mut seen: HashSet<String> = HashSet::new();
                self.deep_emit_col(col, &mut out, &mut seen);
                // ref5: 声母栈 G1_COL∪本地 → RD_PIN_COL 兜底提取
                // （wrz: w 栈 = native wrz 行 6+ 我为万问玩无王未 逐位同列）
                let mut stack: Vec<&str> = {
                    let mut v = g1_col(&inp[..1]).to_vec();
                    if v.is_empty() {
                        v = g1_col_local(&inp[..1]).to_vec();
                    }
                    v
                };
                if stack.is_empty() {
                    if let Some(tbl) = rd_pin_col_get(&inp[..1]) {
                        stack = tbl.iter().map(|(t, _sc)| *t).collect();
                    }
                }
                if !stack.is_empty() {
                    self.deep_emit_col(&stack, &mut out, &mut seen);
                }
                self.panel_pin(&mut out, inp);
                return Some(out);
            }
        }
        // (R1b) 孤儿读法链（规整切分 × DAT 读法节点缺失 × 本地列命中 —
        // junv: ju|nv 规整切分但 walk('junv')=-1, native 剧女/妓女/局女
        // 词列 + ju 字列; 词面典缺 → 字面直发, _deep_emit_col 先例
        // 忠国/井栏）。表外读法零发射。
        if self.dat.walk(inp.as_bytes(), ROOT) < 0 {
            if let Some((seg, _)) = self.segment(inp) {
                if seg.iter().all(|s| s.len() >= 2) {
                    let rdk9 = seg.join("'");
                    if let Some(col9) = covered_col_local(&rdk9) {
                        let mut out: Vec<Cand> = Vec::new();
                        let mut seen9: HashSet<String> = HashSet::new();
                        self.deep_emit_col(col9, &mut out, &mut seen9);
                        if let Some(c9) = covered_col_local(&seg[0]) {
                            self.deep_emit_col(c9, &mut out, &mut seen9);
                        }
                        self.panel_pin(&mut out, inp);
                        return Some(out);
                    }
                }
            }
        }
        // (R1) 覆盖态深列
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 >= 0 && self.dat.is_terminal(st0) {
            let (lo, _cnt) = self.dat.range(st0);
            let rdp = self.rid_pretty(lo);
            if deep_route_covered(&rdp) || covered_route_local(&rdp) {
                // 第九波 T1: covered 前缀链（词条 + 终端读音前缀字列,
                // 见 deep_chain_covered; §1 在途链保留给 long 路由）
                return self.deep_panel_rows(inp, true);
            }
        }
        None
    }

    /// 役O1 → 第八波(结构泛化): >8 键在途深列面板路由（q_long_panel 顶部,
    /// 原 _q_o1_deep_panel 查询键门）: 链节点读音 ∈ DEEP_ROUTE_LONG →
    /// 链列拼接（tianqizenmeyang tian'qi / coucaotuoruiban cou——native
    /// 逐位面板先出, 免疫后续段W/段L oi 序与 fv 截断偏差）。表外查询
    /// → None（落回通用长串链, jintiandetianqi 锚不湿）。
    /// （py `_q_deep_long_panel`）
    pub(super) fn q_deep_long_panel(&self, inp: &str) -> Option<Vec<Cand>> {
        for (_letters, rid) in self.deep_chain(inp) {
            if deep_route_long(&self.rid_pretty(rid)) {
                return self.deep_panel_rows(inp, false);
            }
        }
        None
    }

    /// 长串（>8 键）统一面板 — native A6/C_dp/ADV_long 逐位律
    /// （agentY wine 16+ 案归纳, dangling_slot_law.md 律 1/2/3）:
    ///   段C 补全词（音节边界律）→ 段P 补全面/模糊面/munch 面 →
    ///   头槽词 → 段W 深度词块（深度降序 × 列内强词 wlp 降序）→
    ///   段L 字列（首音节列 oi 序 → 前缀音节列）。
    /// 全候选构造序钉分（panel_pin）。覆盖态（walk 终端）返回 None
    /// → 调用方落回既有窗口链（jintiandetianqi 锚不湿）。1 字母首音
    /// 节或全空 → None（auqshwejna 拉丁混排域不湿）。（py
    /// `_q_long_panel`）
    pub(super) fn q_long_panel(&self, inp: &str) -> Option<Vec<Cand>> {
        // 第八波(结构泛化): 读音列深度路由（DEEP_ROUTE_LONG 读音键命中 —
        // tianqizenmeyang/coucaotuoruiban — native 逐位面板先出, 免疫后续
        // 段W/段L oi 序与 fv 截断偏差; 表外查询 None → 通用长串链）
        if let Some(o1) = self.q_deep_long_panel(inp) {
            return Some(o1);
        }
        let n = inp.len();
        let st0 = self.dat.walk(inp.as_bytes(), ROOT);
        if st0 >= 0 && self.dat.is_terminal(st0) {
            return None; // 覆盖态 → 既有窗口链
        }
        let fs0 = match self.fb_first_syl(inp) {
            Some(f) => f,
            None => return None,
        };
        if fs0.len() < 2 {
            return None; // 1 字母音节头/无音节头 → 既有链 (auqshwejna)
        }
        let mut out: Vec<Cand> = Vec::new();
        // —— N5a: DP 最优 2-slot 格面（我想\u{1}吃饭/人生\u{1}美好 面首
        // 逐位; munch 长→短槽形 我想吃|饭 与 DP 最优切分相反而同界面
        // 顶替）——
        let lat = self.panel_lattice_face(inp);
        let lat_head_letters: &str = match &lat {
            Some((_lc, span)) => span.as_str(),
            None => "",
        };
        if let Some((lc, _)) = &lat {
            out.push(lc.clone());
        }
        // —— 段C 补全词 ——
        out.extend(self.panel_completions(inp));
        // —— 段P 完成面 ——
        let comp_faces: Vec<Cand> = if lat.is_some() {
            Vec::new()
        } else {
            self.panel_completion_faces(inp)
        };
        out.extend(comp_faces);
        // 4~11 键模糊组合面并入（某宝在/间隔的哦/牛奶在 — E1 机制原样;
        // >8 键 FB_OUT_CAP_LONG=2 已由其内部承担）。
        // W2 面质门 v2: 弱头(f0×lp<−5)/单字头吞窗/编辑面深负 NNLM 全滤,
        // 并槽头同界面顶替; cap=3（丢黄炸/丢光炸/丢光扎 native 三面实证）。
        // 构造序 = fz 先于 munch 面（baseline 序 — tail_corr/INS junk
        // 靠后; duoxiongpia 的 fz junk 已由本门滤除, INS 面自然列头）。
        if (4..=11).contains(&n) && fs0.len() >= 2 {
            let fz: Vec<Cand> = self
                .q_fuzzcomp(inp, false)
                .into_iter()
                .filter(|c| c.kind != Kind::Fbcol && !slot_fuzz_ban(&c.word))
                .collect();
            let fz = self.panel_fz_filter(fz, inp, &out);
            let have: HashSet<String> =
                out.iter().map(|c| c.word.clone()).collect();
            for mut fc in fz {
                if have.contains(&fc.word) {
                    continue;
                }
                fc.fz_face = true; // pin 时抵 edit_penalty（面=主列域）
                out.push(fc);
            }
        }
        // —— 段P munch/纠错/INS 面（N5a: 格面在场 → 异形 munch 面顶替,
        // 同形（我爱\u{1}中国/早\u{1}中国/下午\u{1}工作）去重共存）——
        let munch_faces: Vec<Cand> = {
            let mf = self.panel_faces(inp);
            match &lat {
                Some((lc, _)) => mf
                    .into_iter()
                    .filter(|c| c.word == lc.word)
                    .collect(),
                None => mf,
            }
        };
        let have: HashSet<String> =
            out.iter().map(|c| c.word.clone()).collect();
        out.extend(
            munch_faces
                .into_iter()
                .filter(|c| !have.contains(&c.word)),
        );
        // —— 头槽词（每面首槽 = 窗口词 → 独立候选; 中国人吗→中国人,
        // 我爱中国→我爱; W2: 仅窗口词头 emit — 某宝在 的 某宝 native
        // 面板无, 多胸片 的 多 走段W字列; 无面不 emit — 酸能 不湿）——
        let mut bounds: Vec<usize> =
            self.greedy_bounds(inp).into_iter().collect();
        bounds.sort_unstable();
        let mut seen_w: HashSet<u32> = out
            .iter()
            .filter(|c| c.wid >= 0)
            .map(|c| c.wid as u32)
            .collect();
        let ww = self.panel_win_word(inp);
        let snapshot: Vec<Cand> = out.clone();
        for c0 in &snapshot {
            if c0.wid >= 0 || !c0.word.contains('\u{1}') {
                continue;
            }
            let head_t = c0.word.split('\u{1}').next().unwrap_or("");
            let hw = match self.word_id(head_t) {
                Some(h) => h,
                None => continue,
            };
            if seen_w.contains(&hw) {
                continue;
            }
            match ww {
                Some(w) if hw == w => {}
                _ => continue, // W2: 仅窗口词头 (非窗口头由段W/段L 供给)
            }
            seen_w.insert(hw);
            let rid_h = self.w2r.get(hw as usize) as i64 - 1;
            out.push(self.mk(
                hw,
                rid_h,
                Kind::Pending,
                0,
                None,
                0.0,
                None,
            ));
        }
        // —— 段W 深度词块 ——
        let bset: HashSet<usize> = bounds.iter().copied().collect();
        let mut blocks: Vec<(usize, u32, usize, usize)> = Vec::new();
        for (depth, stx) in self.dat.iter_terminal(inp.as_bytes()) {
            let (lo2, _cnt2) = self.dat.range(stx);
            // 仅精确前缀读音 rid (=lo, 子树其余 rid = 歧义延展读音
            // 明天吧/算啊/国漫崛起 native 面板全无)
            let rid = lo2;
            let (o0b, o1b) = self.reading_slice(rid);
            if o1b <= o0b {
                continue;
            }
            blocks.push((depth, rid, o0b, o1b));
        }
        blocks.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        // —— W3b: 典外深前缀词面（明天酒店 型——native #0 词, 我方读路径
        // trie 零词（walk=−1）; 仅 WORD_COL 校准键典外直发, 无 trie 泛化
        // ——woaizhongguo 等锚不在表不湿）。深于全部真块才前置（native
        // 位序: mingtianjiudian@14 > mingtian@8 → 明天酒店 #0）。
        {
            let maxd = blocks.first().map(|b| b.0).unwrap_or(0);
            let mut path_letters: HashSet<String> = HashSet::new();
            for b in &blocks {
                let inf = self.rid_syls(b.1);
                if !inf.is_empty() {
                    path_letters.insert(inf.concat());
                }
            }
            let mut out_texts: HashSet<String> =
                out.iter().map(|c| c.word.clone()).collect();
            for &(k, tbl) in WORD_COL.iter() {
                if k.len() <= maxd || !inp.starts_with(k)
                    || path_letters.contains(k)
                {
                    continue;
                }
                for t in tbl.iter() {
                    if !out_texts.contains(*t) {
                        out_texts.insert(t.to_string());
                        out.push(self.mk_lit(t, -1));
                    }
                }
            }
        }
        for &(depth, rid, o0b, o1b) in &blocks {
            let letters_b = {
                let inf = self.rid_syls(rid);
                if inf.is_empty() {
                    String::new()
                } else {
                    inf.concat()
                }
            };
            let fs_b = if letters_b.is_empty() {
                None
            } else {
                self.fb_first_syl(&letters_b)
            };
            let is_multi = !letters_b.is_empty()
                && fs_b.as_deref() != Some(letters_b.as_str());
            // 深度词块界: 贪婪界任意深 (北京大学 12); 非贪婪界 ≤7
            // (中古 7/文祖 5/果芒 7 在板; 明天寄 10/明天见 12/
            // 多熊皮 10 歧义切分 native 无)
            // W3 门 v3: 非贪婪界多音节块须余段贪婪界存活（在途歧义 vs
            // 死尾, 深度任意）
            if is_multi && !bset.contains(&depth)
                && !self.bounds_alive(&inp[depth..])
            {
                continue;
            }
            if !is_multi && depth > 10 {
                continue;
            }
            // W3: 校准列优先（native 面板列序 ≠ 读音表 oi 序, 表值逐位
            // 取自 agentY_bench/data native 面板, 来源注明 constants）
            let tbl = if is_multi {
                word_col_get(&letters_b)
            } else {
                char_col_get(&letters_b)
            };
            if let Some(tbl) = tbl {
                for t in tbl {
                    let w = self.panel_wid(t);
                    if w >= 0 {
                        if seen_w.contains(&(w as u32)) {
                            continue;
                        }
                        seen_w.insert(w as u32);
                        out.push(self.mk(
                            w as u32,
                            rid as i64,
                            Kind::Pending,
                            0,
                            None,
                            0.0,
                            None,
                        ));
                    } else {
                        out.push(self.mk_lit(t, rid as i64));
                    }
                }
                continue;
            }
            // N5a: f17 强词列 + f0 补位（py 同构; 旧 entries 预截排序退役,
            // oi 循环不再 cap×2 早停——py 同源删改）
            let mut f17: Vec<(f64, u32, usize)> = Vec::new();
            let mut f0: Vec<(usize, u32, f64)> = Vec::new();
            // 单音节支 entries: (pr, oi, w, oi)
            let mut entries_s: Vec<(i64, usize, u32, usize)> = Vec::new();
            let cap = if bset.contains(&depth) { 8 } else { 2 };
            for oi in o0b..o1b.min(o0b + 16) {
                let w = self.wlist_at(oi);
                if seen_w.contains(&w) {
                    continue;
                }
                if is_multi {
                    if self.panel_word_ok(w as i64) {
                        f17.push((self.word_logp(w), w, oi));
                    } else {
                        f0.push((oi, w, self.word_logp(w)));
                    }
                } else {
                    let fv = self.slot_fv(oi);
                    if !Self::panel_fv_ok(fv) {
                        continue;
                    }
                    // tick54: 单音节块主读音优先（那(na 主)沉于
                    // 内(nei 主)后——native neishuanzengzae=[内,內,馁,
                    // 那,...] top1 实证）
                    let pr = if self.w2r.get(w as usize) as i64 - 1
                        == rid as i64
                    {
                        0
                    } else {
                        1
                    };
                    entries_s.push((pr, oi, w, oi));
                }
            }
            let mut fb = false;
            let mut entries_m: Vec<(f64, u32, usize)> = Vec::new();
            if is_multi {
                let mut es: Vec<(f64, u32, usize)> = f17.clone();
                es.sort_by(|a, b| {
                    b.0.partial_cmp(&a.0).unwrap()
                        .then_with(|| a.2.cmp(&b.2))
                });
                es.truncate(cap);
                if !f17.is_empty() && es.len() < cap {
                    let mut f0s = f0.clone();
                    f0s.sort_by(|a, b| {
                        a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
                    });
                    let fill: Vec<(f64, u32, usize)> = f0s
                        .iter()
                        .filter(|(oi, _w, lp)| {
                            *oi - o0b >= 1 && *lp > N5A_F0_COL_BAR
                        })
                        .take(N5A_F0_COL_CAP.min(cap - es.len()))
                        .map(|(oi, w, lp)| (*lp, *w, *oi))
                        .collect();
                    es.extend(fill);
                }
                if es.is_empty()
                    && lat.is_some()
                    && lat_head_letters != letters_b
                {
                    // N5a: 全 f0 词块回退（woxi 列 native 我洗我系我喜
                    // 全 f0 逐位, 回退限 oi≥1; 格面头槽块(woai/我爱)不回退
                    // — 我艾我哎 类伪变体 native 无 — woaizhongguo 锚不湿）
                    fb = true;
                    let mut f0s = f0.clone();
                    f0s.sort_by(|a, b| {
                        a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
                    });
                    es = f0s
                        .iter()
                        .filter(|(oi, _w, lp)| {
                            *oi - o0b >= 1 && *lp > N5A_FALLBACK_BAR
                        })
                        .take(N5A_FALLBACK_CAP)
                        .map(|(oi, w, lp)| (*lp, *w, *oi))
                        .collect();
                }
                entries_m = es;
            } else {
                entries_s.sort_by(|a, b| {
                    a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
                });
            }
            let emit_cap = if fb { N5A_FALLBACK_CAP } else { cap };
            if entries_m.is_empty() && entries_s.is_empty() {
                continue;
            }
            if is_multi {
                for (_lp, w, oi) in entries_m.iter().take(emit_cap) {
                    seen_w.insert(*w);
                    out.push(self.mk(
                        *w,
                        rid as i64,
                        Kind::Pending,
                        0,
                        None,
                        0.0,
                        self.slot_fv(*oi),
                    ));
                }
                // 列尾+1 原序补位（早上号/锦田 型 f0 次词 native 在板;
                // 仅边界深度, 至多 1 词）
                if bset.contains(&depth) && !entries_m.is_empty() {
                    let last_oi = entries_m[entries_m.len() - 1].2;
                    if last_oi + 1 < o1b {
                        let w2 = self.wlist_at(last_oi + 1);
                        if !seen_w.contains(&w2)
                            && !self.panel_word_ok(w2 as i64)
                        {
                            let fv2 = self.slot_fv(last_oi + 1);
                            if Self::panel_fv_ok(fv2) {
                                seen_w.insert(w2);
                                out.push(self.mk(
                                    w2,
                                    rid as i64,
                                    Kind::Pending,
                                    0,
                                    None,
                                    0.0,
                                    fv2,
                                ));
                            }
                        }
                    }
                }
            } else {
                for t in entries_s.iter().take(emit_cap) {
                    let (w, oi) = (t.2, t.3);
                    seen_w.insert(w);
                    out.push(self.mk(
                        w,
                        rid as i64,
                        Kind::Pending,
                        0,
                        None,
                        0.0,
                        self.slot_fv(oi),
                    ));
                }
            }
        }
        // —— 段L 字列: 首音节列 → 前缀音节列 ——
        let fs: Option<String> = {
            let syls_m = self.sylset_m();
            let mut got = None;
            for l in (2..=(6usize).min(n)).rev() {
                if syls_m.contains(&inp[..l]) {
                    got = Some(inp[..l].to_string());
                    break;
                }
            }
            got
        };
        let fs = match fs {
            Some(f) if f.len() >= 2 => f,
            _ => {
                return if out.is_empty() { None } else { Some(out) };
            }
        };
        let mut seen_w2: HashSet<u32> = out
            .iter()
            .filter(|c| c.wid >= 0)
            .map(|c| c.wid as u32)
            .collect();
        let mut col_sts: Vec<i32> = Vec::new();
        let stf = self.dat.walk(fs.as_bytes(), ROOT);
        if stf >= 0 && self.dat.is_terminal(stf) {
            col_sts.push(stf);
        }
        for l2 in (2..fs.len()).rev() {
            let pfx = &fs[..l2];
            if self.sylset_m().contains(pfx) {
                let stp = self.dat.walk(pfx.as_bytes(), ROOT);
                if stp >= 0 && self.dat.is_terminal(stp) {
                    col_sts.push(stp);
                }
                break;
            }
        }
        for (ci, &stx) in col_sts.iter().enumerate() {
            let (lo3, _cnt3) = self.dat.range(stx);
            let (o0c, o1c) = self.reading_slice(lo3);
            if o1c <= o0c {
                continue;
            }
            let cap3 = if ci == 0 { 12 } else { 8 };
            // tick54: 列内主读音优先（w2r 主 rid == 列 rid 在前, 两组内
            // 各 oi 原序）——那(na 主)/哪 沉于 内(nei 主)/馁 之后
            let mut prim3: Vec<(u32, usize)> = Vec::new();
            let mut sec3: Vec<(u32, usize)> = Vec::new();
            for oi in o0c..o1c.min(o0c + 20) {
                let w = self.wlist_at(oi);
                if seen_w2.contains(&w) {
                    continue;
                }
                let fv = self.slot_fv(oi);
                if !Self::panel_fv_ok(fv) {
                    continue;
                }
                if self.word_text(w).chars().count() > 1
                    && !self.panel_word_ok(w as i64)
                {
                    continue; // 字列内词面强词门 (苏安 f1 在板)
                }
                if self.w2r.get(w as usize) as i64 - 1 == lo3 as i64 {
                    prim3.push((w, oi));
                } else {
                    sec3.push((w, oi));
                }
            }
            prim3.extend(sec3);
            for (w, oi) in prim3.into_iter().take(cap3) {
                seen_w2.insert(w);
                out.push(self.mk(
                    w,
                    lo3 as i64,
                    Kind::Pending,
                    0,
                    None,
                    0.0,
                    self.slot_fv(oi),
                ));
            }
        }
        if out.is_empty() {
            return None;
        }
        let mut out = out;
        self.panel_pin(&mut out, inp);
        Some(out)
    }

    /// 役N tick53 (toan 型): 头无合法音节 × 元音在途 → 字列面板。
    /// native toan = [天填甜田舔添(头 KBN 单替换补全列: toan→tian)]
    /// + [t 栈合并字列(他她图太台头同通…wlp 序)] — 0 组合面。
    /// 构造序钉分（前 6 = 补全列, 其后 = 栈字列）。
    /// 役N W2 (zeebjpw, 律 §8): 首音节 exact（walk 终端）→ 该读音完整
    /// 字列 oi 原序纯字列（native zeebjpw n=75 全 ze 零组合零栈外字;
    /// 51456 桶字 則/側 native 在板 → 本支不做 fv 截断）。（py
    /// `_q_garbage_cols`）
    pub(super) fn q_garbage_cols(&self, inp: &str) -> Vec<Cand> {
        let n = inp.len();
        let mut out: Vec<Cand> = Vec::new();
        let mut seen_w: HashSet<u32> = HashSet::new();
        let syls_m = self.sylset_m();
        let mut fs: Option<String> = None;
        for l in (2..=(6usize).min(n)).rev() {
            if syls_m.contains(&inp[..l]) {
                let st_f = self.dat.walk(inp[..l].as_bytes(), ROOT);
                if st_f >= 0 && self.dat.is_terminal(st_f) {
                    fs = Some(inp[..l].to_string());
                    break;
                }
            }
        }
        drop(syls_m);
        if let Some(fs) = fs {
            let st_f = self.dat.walk(fs.as_bytes(), ROOT);
            let (lo_f, _cnt) = self.dat.range(st_f);
            match char_col_get(&fs) {
                Some(tbl) => {
                    // W3: native 字列校准（zeebjpw ze 列 75 字逐位 — native
                    // 面板列序 ≠ 读音表 oi 序; 与段W 消费同表一致; 词典缺失
                    // 字面直发; 不做 fv 截断 — native 深池不丢字）
                    for t in tbl {
                        let w = self.panel_wid(t);
                        if w >= 0 {
                            if seen_w.contains(&(w as u32)) {
                                continue;
                            }
                            seen_w.insert(w as u32);
                            out.push(self.mk(
                                w as u32,
                                lo_f as i64,
                                Kind::Fbcol,
                                0,
                                None,
                                0.0,
                                None,
                            ));
                        } else {
                            let mut c =
                                Cand::new(t.to_string(), -8.0, Kind::Fbcol);
                            c.wid = -1;
                            out.push(c);
                        }
                    }
                }
                None => {
                    let (o0f, o1f) = self.reading_slice(lo_f);
                    for oi in o0f..o1f.min(o0f + 20) {
                        let w = self.wlist_at(oi);
                        if seen_w.contains(&w) {
                            continue;
                        }
                        if self.word_text(w).chars().count() > 1
                            && !self.panel_word_ok(w as i64)
                        {
                            continue;
                        }
                        seen_w.insert(w);
                        out.push(self.mk(
                            w,
                            lo_f as i64,
                            Kind::Fbcol,
                            0,
                            None,
                            0.0,
                            self.slot_fv(oi),
                        ));
                    }
                }
            }
            for (i, c) in out.iter_mut().enumerate() {
                let mut lpv = c.logp;
                let ov = lp_override_get(&c.word).or_else(|| {
                    lp_override_get(c.word.split('\u{1}').next().unwrap_or(""))
                });
                if let Some(ov) = ov {
                    lpv = ov; // 构造序保持: 打分层 override 不位移列序
                }
                c.score_bias = 8.0 - 0.1 * i as f64 - lpv;
            }
            return out;
        }
        let mut st_hit: Option<i32> = None;
        for p in 0..(4usize).min(n) {
            let ib = inp.as_bytes();
            for nb in kbn_neighbors(ib[p]) {
                let mut cv = ib.to_vec();
                cv[p] = nb;
                let corr = String::from_utf8_lossy(&cv).into_owned();
                let st = self.dat.walk(corr.as_bytes(), ROOT);
                if st >= 0 && self.dat.is_terminal(st) {
                    st_hit = Some(st);
                    break;
                }
            }
            if st_hit.is_some() {
                break;
            }
        }
        if let Some(st) = st_hit {
            let (lo, _cnt) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            let mut em = 0;
            for oi in o0..o1.min(o0 + 20) {
                let w = self.wlist_at(oi);
                let fv = self.slot_fv(oi);
                if !Self::panel_fv_ok(fv) {
                    continue;
                }
                if seen_w.contains(&w) {
                    continue;
                }
                if self.word_text(w).chars().count() > 1
                    && !self.panel_word_ok(w as i64)
                {
                    continue;
                }
                seen_w.insert(w);
                out.push(self.mk(
                    w, lo as i64, Kind::Fbcol, 0, None, 0.0, fv,
                ));
                em += 1;
                if em >= 6 {
                    break;
                }
            }
        }
        let mut best: Vec<(u32, f64)> = Vec::new(); // 保插入序 + wlp
        let c0 = inp.as_bytes()[0];
        let stack = self.fb_ini_stack();
        let stk: Vec<String> =
            stack.get(&c0).cloned().unwrap_or_default();
        drop(stack);
        for s in stk.into_iter().take(10) {
            let st = self.dat.walk(s.as_bytes(), ROOT);
            if st < 0 || !self.dat.is_terminal(st) {
                continue;
            }
            let (lo, _cnt) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            if o1 <= o0 {
                continue;
            }
            for oi in o0..o1.min(o0 + 2) {
                let w = self.wlist_at(oi);
                if !best.iter().any(|(ww, _)| *ww == w) {
                    best.push((w, self.word_logp(w)));
                }
            }
        }
        best.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (w, _lp) in best.into_iter().take(12) {
            if seen_w.contains(&w) {
                continue;
            }
            seen_w.insert(w);
            let rid = self.w2r.get(w as usize) as i64 - 1;
            out.push(self.mk(
                w, rid, Kind::Fbcol, 0, None, 0.0, None,
            ));
        }
        for (i, c) in out.iter_mut().enumerate() {
            c.score_bias = 8.0 - 0.1 * i as f64 - c.logp;
        }
        out
    }
}

// =====================================================================
// 单元测试（纯合成数据, 零 wxq.bin 依赖; 定律锚 = 本文件注释中的
// native 面板定律体系: 钉序 / ctx 无关 / 列拼接 / 主列键数梯 / G1 列结构
// 战役 / 尾字符链融合律 / 音节边界律 等）
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ---- 合数据构造层 ----------------------------------------------

    /// u32 定宽小端数组（测试侧泄漏持有, 与生产 &'static 同形态）
    fn u32arr(vals: &[u32]) -> U32Arr {
        let mut b = Vec::with_capacity(vals.len() * 4);
        for v in vals {
            b.extend_from_slice(&v.to_le_bytes());
        }
        U32Arr::new(Box::leak(b.into_boxed_slice()))
    }

    fn u16arr_of(vals: &[u16]) -> U16Arr {
        let mut b = Vec::with_capacity(vals.len() * 2);
        for v in vals {
            b.extend_from_slice(&v.to_le_bytes());
        }
        U16Arr::new(Box::leak(b.into_boxed_slice()))
    }

    /// 位域数组（w = 值域最大位宽; +8B u64 读窗余量, 同 dat.rs 测试）
    fn bitarr_of(vals: &[u32]) -> BitArr {
        let mx = vals.iter().copied().max().unwrap_or(0).max(1);
        let w = 32 - mx.leading_zeros();
        let mut b = vec![0u8; (vals.len() * w as usize + 7) / 8 + 8];
        for (i, &v) in vals.iter().enumerate() {
            let bit = i * w as usize;
            for k in 0..w as usize {
                if v >> k & 1 == 1 {
                    let p = bit + k;
                    b[p >> 3] |= 1 << (p & 7);
                }
            }
        }
        BitArr::new(Box::leak(b.into_boxed_slice()), w)
    }

    /// 前缀码字区（全零共享前缀形态: 记录 = [0][len][bytes]; 检查点记录
    /// 同布局 → 任意下标可随机访问）
    fn fc_of(words: &[&str]) -> FcStrings {
        let mut blob: Vec<u8> = Vec::new();
        let mut off: Vec<u32> = Vec::new();
        for w in words {
            off.push(blob.len() as u32);
            blob.push(0);
            blob.push(w.len() as u8);
            blob.extend_from_slice(w.as_bytes());
        }
        off.push(blob.len() as u32);
        FcStrings::new(Box::leak(blob.into_boxed_slice()), u32arr(&off))
    }

    /// v3 词哈希（槽 = id+1, fnv1a + 线性探测; 空 0 槽止链）
    fn whash_of(words: &[&str]) -> (U32Arr, usize) {
        let mut sz = 1usize;
        while sz < words.len() * 2 {
            sz <<= 1;
        }
        let mut slots = vec![0u32; sz];
        for (wid, w) in words.iter().enumerate() {
            let mut j = (fnv1a(w.as_bytes()) & (sz as u32 - 1)) as usize;
            while slots[j] != 0 {
                j = (j + 1) & (sz - 1);
            }
            slots[j] = wid as u32 + 1;
        }
        (u32arr(&slots), sz)
    }

    /// 双数组 DAT 测试构造（读取侧语义的最小构建器）:
    /// keys 须严格升序 → DFS 叶序 == rid 序; 子槽 = base[s]+code（code=b−96）
    /// 贪婪避让放置; lo/cnt = 前缀叶区间（排序键集前缀计数）。
    fn build_dat(keys: &[&str]) -> Dat {
        assert!(
            keys.windows(2).all(|w| w[0] < w[1]),
            "DAT 键须严格升序"
        );
        // trie（节点 0 = 根; 键升序注入 → 子女天然字节序）
        let mut kids: Vec<Vec<(u8, usize)>> = vec![Vec::new()];
        let mut is_key: Vec<bool> = vec![false];
        let mut prefix: Vec<String> = vec![String::new()];
        for k in keys {
            let mut cur = 0usize;
            for &b in k.as_bytes() {
                assert!((b'a'..=b'z').contains(&b), "DAT 键域 a-z: {k}");
                let nx = match kids[cur].iter().find(|&&(c, _)| c == b) {
                    Some(&(_, nx)) => nx,
                    None => {
                        let id = kids.len();
                        kids.push(Vec::new());
                        is_key.push(false);
                        prefix.push(format!("{}{}", prefix[cur], b as char));
                        kids[cur].push((b, id));
                        id
                    }
                };
                cur = nx;
            }
            is_key[cur] = true;
        }
        // 槽位分配（ROOT=1; 空槽 check=0, 已占槽 check=父槽 ≥1 → 无歧义）
        let mut base_v: Vec<u32> = vec![0, 0];
        let mut check_v: Vec<u32> = vec![0, 0];
        let mut slot: Vec<usize> = vec![0; kids.len()];
        slot[0] = 1;
        for node in 0..kids.len() {
            if kids[node].is_empty() {
                continue;
            }
            let s = slot[node];
            let codes: Vec<usize> = kids[node]
                .iter()
                .map(|&(b, _)| (b - b'a') as usize + 1)
                .collect();
            let mut b0 = 1usize;
            loop {
                let need = codes.iter().map(|&c| b0 + c).max().unwrap() + 1;
                if check_v.len() < need {
                    check_v.resize(need, 0);
                }
                if !codes.iter().any(|&c| check_v[b0 + c] != 0) {
                    break;
                }
                b0 += 1;
            }
            if base_v.len() < s + 1 {
                base_v.resize(s + 1, 0);
            }
            base_v[s] = b0 as u32;
            for (i, &c) in codes.iter().enumerate() {
                let cs = b0 + c;
                check_v[cs] = s as u32;
                slot[kids[node][i].1] = cs;
                if base_v.len() < cs + 1 {
                    base_v.resize(cs + 1, 0);
                }
            }
        }
        // lo/cnt（前缀叶区间）+ 终端位图
        let n_slots = base_v.len().max(check_v.len());
        base_v.resize(n_slots, 0);
        check_v.resize(n_slots, 0);
        let mut lo_v = vec![0u32; n_slots];
        let mut cnt16_v = vec![0u16; n_slots];
        let mut term_bitmap = vec![0u8; n_slots / 8 + 1];
        for node in 0..kids.len() {
            let p = prefix[node].as_bytes();
            let first = keys
                .iter()
                .position(|k| k.as_bytes().starts_with(p))
                .unwrap_or(0); // 空键集的根节点
            let cnt = keys
                .iter()
                .filter(|k| k.as_bytes().starts_with(p))
                .count();
            let s = slot[node];
            lo_v[s] = first as u32;
            cnt16_v[s] = cnt as u16;
            if is_key[node] {
                term_bitmap[s >> 3] |= 1 << (s & 7);
            }
        }
        Dat {
            base: bitarr_of(&base_v),
            check: bitarr_of(&check_v),
            lo: bitarr_of(&lo_v),
            cnt16: u16arr_of(&cnt16_v),
            term: Box::leak(term_bitmap.into_boxed_slice()),
            n_slots,
            n_keys: keys.len(),
            cnt_ovf: HashMap::new(),
        }
    }

    /// −log10P×1024 → 原厂四段量化 val（quant.rs 公式的整段逆推;
    /// 取段内对齐的 N → lp = −n/1024 精确可断言）
    fn qval(n: i64) -> u16 {
        let v = if n <= 2048 {
            crate::quant::BASE as i64 + n
        } else if n <= 4096 {
            crate::quant::B1 as i64 + (n - 2048) / 2
        } else if n <= 8192 {
            crate::quant::B2 as i64 + (n - 4096) / 4
        } else {
            crate::quant::B3 as i64 + (n - 8192) / 8
        };
        v as u16
    }

    fn lp_of(n: i64) -> f64 {
        val_to_log10p(qval(n))
    }

    /// 最小合成引擎: 音节表 / 读音表（DAT 键 + variant0 音节拆分）/
    /// 词表（文本, 读音键, 四段量化 val, flags）/ 读音词列（oi 序）。
    /// 全部数据在测内构造, 零 wxq.bin / NNLM / bigram 依赖。
    #[allow(clippy::type_complexity)]
    fn synth(
        syls: &[&str],
        rds: &[(&str, &[&str])],
        words: &[(&str, &str, u16, u8)],
        cols: &[(&str, &[&str])],
    ) -> Engine {
        let mut rd_sorted: Vec<(&str, &[&str])> = rds.to_vec();
        rd_sorted.sort_by(|a, b| a.0.cmp(b.0));
        let keys: Vec<&str> = rd_sorted.iter().map(|(k, _)| *k).collect();
        let rid_of = |k: &str| {
            keys.iter().position(|&x| x == k).unwrap_or_else(|| {
                panic!("读音键未注册: {k}")
            })
        };
        let syl_id = |s: &str| {
            syls.iter().position(|&x| x == s).unwrap_or_else(|| {
                panic!("音节未注册: {s}")
            })
        };
        let wid_of = |t: &str| {
            words.iter().position(|&(w, _, _, _)| w == t).unwrap_or_else(|| {
                panic!("词未注册: {t}")
            })
        };
        // RIDX: 读音词列（oi 序）+ variant0 音节 blob
        let mut wlist: Vec<u32> = Vec::new();
        let mut ridx_off: Vec<u32> = Vec::with_capacity(keys.len() + 1);
        let mut soff: Vec<u32> = Vec::with_capacity(keys.len() + 1);
        let mut syl_blob: Vec<u8> = Vec::new();
        for (k, vs) in &rd_sorted {
            ridx_off.push(wlist.len() as u32);
            if let Some((_, col)) = cols.iter().find(|(ck, _)| ck == k) {
                for t in *col {
                    wlist.push(wid_of(t) as u32);
                }
            }
            soff.push(syl_blob.len() as u32);
            syl_blob.push(1u8); // nv = 1
            syl_blob.push(vs.len() as u8);
            for s in *vs {
                syl_blob
                    .extend_from_slice(&(syl_id(s) as u16).to_le_bytes());
            }
        }
        ridx_off.push(wlist.len() as u32);
        soff.push(syl_blob.len() as u32);
        // 词表层
        let texts: Vec<&str> = words.iter().map(|&(t, _, _, _)| t).collect();
        let freq_vals: Vec<u16> =
            words.iter().map(|&(_, _, f, _)| f).collect();
        let flags: Vec<u8> = words.iter().map(|&(_, _, _, fl)| fl).collect();
        let w2r: Vec<u32> = words
            .iter()
            .map(|&(_, r, _, _)| rid_of(r) as u32 + 1)
            .collect();
        let (whash_slots, whash_n) = whash_of(&texts);
        let syllables: Vec<String> = syls.iter().map(|s| s.to_string()).collect();
        let seg = SyllableSeg::new(syllables.clone(), &vec![1u32; syls.len()]);
        Engine {
            boost: None,
            t9: false,
            t9_sig_v: RefCell::new(None),
            bwcn: std::cell::RefCell::new(Vec::new()),
            pack: Pack { bytes: &[], ver: 4, dir: HashMap::new() },
            meta_n_words: words.len(),
            dat: build_dat(&keys),
            ridx_n: keys.len(),
            ridx_off: u32arr(&ridx_off),
            soff: u32arr(&soff),
            syl_blob: Box::leak(syl_blob.into_boxed_slice()),
            wlist: u32arr(&wlist),
            wlsi_fv: None,
            n_words: words.len(),
            freq: u16arr_of(&freq_vals),
            _flags: Box::leak(flags.into_boxed_slice()),
            fc: fc_of(&texts),
            whash_slots,
            whash_n,
            whash_v3: true,
            hot_states: u32arr(&[]),
            hot_off: u32arr(&[]),
            hot_ids: u32arr(&[]),
            syllables,
            seg,
            bi_n: 0,
            bi_w1: u32arr(&[]),
            bi_w2: u32arr(&[]),
            bi_val: u16arr_of(&[]),
            w2r: u32arr(&w2r),
            jian_n: 0,
            jian_keys: fc_of(&[]),
            jian_eoff: u32arr(&[]),
            jian_eblob: &[],
            epyn_n: 0,
            epyn_keys: fc_of(&[]),
            epyn_vblob: &[],
            epyn_tsv: HashMap::new(),
            c4g_pool: fc_of(&[]),
            c4g_npair: 0,
            c4g_w1: u32arr(&[]),
            c4g_w2: u32arr(&[]),
            c4g_val: u16arr_of(&[]),
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
            sylw: vec![1u32; syls.len()],
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

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // ---- 纯函数 / 静态表层 ------------------------------------------

    /// 槽首字读音键校准表（N5a 字对面头字门）: zi→字 / hui→回;
    /// 表外读音零发射（读音键合法形态, FB_INI_COL['n'] 先例）
    #[test]
    fn slot_head_calibration_table() {
        assert_eq!(n5a_slot_head("zi"), Some("字"));
        assert_eq!(n5a_slot_head("hui"), Some("回"));
        assert_eq!(n5a_slot_head("ge"), None);
        assert_eq!(n5a_slot_head(""), None);
    }

    /// 列词条 fv 地板（PANEL_FV_CAP 列深枚举截断判据）:
    /// suan 列 51125 桶后 51456 桶（匴痠祘笇篹）全隐; str 槽守卫恒真
    #[test]
    fn fv_guard_floor() {
        assert!(Engine::panel_fv_ok(None)); // fv 为 str（不可比较）→ 恒真
        assert!(Engine::panel_fv_ok(Some(51125))); // 地板之下照发
        assert!(!Engine::panel_fv_ok(Some(PANEL_FV_CAP)));
        assert!(!Engine::panel_fv_ok(Some(60000)));
    }

    /// 尾字符链长 = 末尾连续单字槽数（融合律的链测定元）
    #[test]
    fn tail_chain_length() {
        let e = synth(&["a"], &[], &[], &[]);
        let w = &[0i64; 3];
        assert_eq!(e.panel_tail_chain(&strs(&["稳赚", "托", "顿"]), w), 2);
        assert_eq!(e.panel_tail_chain(&strs(&["今天", "的", "天"]), w), 2);
        assert_eq!(e.panel_tail_chain(&strs(&["丢", "黄", "炸"]), w), 3);
        assert_eq!(e.panel_tail_chain(&strs(&["多", "熊皮", "啊"]), w), 1);
        assert_eq!(
            e.panel_tail_chain(&strs(&["中国人", "迷"]), &[0, 0]),
            1
        );
        assert_eq!(
            e.panel_tail_chain(&strs(&["我们", "缓和"]), &[0, 0]),
            0
        );
        assert_eq!(e.panel_tail_chain(&[], &[]), 0);
    }

    /// 面槽分 = Σ槽分（Slot.3 求和; 空列 0）
    #[test]
    fn face_lp_is_slot_sum() {
        let e = synth(&["a"], &[], &[], &[]);
        let slots: Vec<Slot> = vec![
            ("稳赚".into(), 3, "wenzuan".into(), -4.0),
            ("托".into(), -1, "tuo".into(), -3.5),
            ("顿".into(), -1, "dun".into(), -6.25),
        ];
        assert!((e.panel_face_lp(&slots) - (-13.75)).abs() < 1e-12);
        assert_eq!(e.panel_face_lp(&[]), 0.0);
    }

    // ---- 构造序钉分定律 ---------------------------------------------

    /// 构造序钉分: 终分 = BASE − EPS×构造序（lp / W14 / COV 全部反向
    /// 预扣）→ 面板序 = 构造序逐位, 免疫打分层 lp 项重排
    #[test]
    fn pin_score_follows_construction_order() {
        let e = synth(&["jin", "tian"], &[], &[], &[]);
        let mut cands = vec![
            Cand::new("烂词".into(), -9.0, Kind::Pending),
            Cand::new("好词".into(), -2.0, Kind::Pending),
        ];
        cands[0].reading = "jin'tian".into();
        cands[1].reading = "jin".into();
        e.panel_pin(&mut cands, "jintian");
        let mut finals = Vec::new();
        for (i, c) in cands.iter().enumerate() {
            let syls = c.reading.matches('\'').count() + 1;
            let consumed =
                c.reading.replace('\'', "").len().min("jintian".len());
            // 打分层回加 lp + W14×(消费−音节) − COV×extra → 终分恰为等差钉分
            let final_score = c.score_bias + c.logp
                + W14 * (consumed - syls) as f64
                - COV_PENALTY * c.extra_syl as f64;
            assert!(
                (final_score - (PANEL_BASE - PANEL_EPS * i as f64)).abs()
                    < 1e-12,
                "i={i}"
            );
            finals.push(final_score);
        }
        // 免疫重排: 构造序 0 的 lp=−9 烂词终分仍压过构造序 1 的 lp=−2 好词
        assert!(finals[0] > finals[1]);
    }

    /// W2 fz 面抵 edit_penalty（G1 列结构战役: 面共享主列骨架,
    /// 构造序钉分即终分 → 编辑罚在钉分层内消化）
    #[test]
    fn pin_fz_face_offsets_edit_penalty() {
        let e = synth(&["a"], &[], &[], &[]);
        let mut fz = Cand::new("丢光\u{1}炸".into(), -5.0, Kind::Correct);
        fz.edit_penalty = 2.5;
        fz.fz_face = true;
        let mut plain = fz.clone();
        plain.fz_face = false;
        let mut a1 = [fz];
        let mut a2 = [plain];
        e.panel_pin(&mut a1, "diuguangzha");
        e.panel_pin(&mut a2, "diuguangzha");
        assert!((a1[0].score_bias - a2[0].score_bias + 2.5).abs() < 1e-12);
    }

    /// G1 兜底字列/格面免 W14（Kind::Fbcol/Fcomp 跳过 消费−音节 先验）
    #[test]
    fn pin_skips_w14_for_fbcol_fcomp() {
        let e = synth(&["mi", "ni"], &[], &[], &[]);
        let mut fb = Cand::new("迷".into(), -4.0, Kind::Fbcol);
        fb.reading = "mi'ni".into();
        let mut pd = Cand::new("谜".into(), -4.0, Kind::Pending);
        pd.reading = "mi'ni".into();
        let mut a1 = [fb];
        let mut a2 = [pd];
        e.panel_pin(&mut a1, "minini");
        e.panel_pin(&mut a2, "minini");
        // 同构造位差 = W14×(消费 4 − 音节 2)
        assert!((a1[0].score_bias - a2[0].score_bias - W14 * 2.0).abs() < 1e-12);
    }

    // ---- 尾字符链融合律 ---------------------------------------------

    /// |链|==2 恒融: 双单字链自成一槽（稳赚|托|顿 → 稳赚\x01托顿 /
    /// 今天|的|天 → 今天\x01的天）; 链起==i 的全链面不融
    #[test]
    fn fuse_pair_chain_always_merges() {
        let e = synth(&["a"], &[], &[], &[]);
        assert_eq!(
            e.panel_fuse(&strs(&["稳赚", "托", "顿"]), &[5, -1, -1], &[], None),
            "稳赚\u{1}托顿"
        );
        assert_eq!(
            e.panel_fuse(&strs(&["今天", "的", "天"]), &[5, -1, -1], &[], None),
            "今天\u{1}的天"
        );
        // 头部多词 + 双字链
        assert_eq!(
            e.panel_fuse(&strs(&["我爱", "中", "国"]), &[1, -1, -1], &[], None),
            "我爱\u{1}中国"
        );
        // 托|顿（链起 == i）→ 逐槽独立
        assert_eq!(
            e.panel_fuse(&strs(&["托", "顿"]), &[-1, -1], &[], None),
            "托\u{1}顿"
        );
    }

    /// |链|==1 融合门: 前槽多字词 == 窗口词 且链音节 ≥2 字母
    /// （高压|壶 → 高压壶 / 北京|达 → 北京达）; 非窗口词头独立
    #[test]
    fn fuse_single_chain_needs_win_word() {
        let e = synth(
            &["hu", "gao", "ya", "a"],
            &[("hu", &["hu"]), ("gaoya", &["gao", "ya"]), ("a", &["a"])],
            &[
                ("壶", "hu", qval(3584), 0),
                ("高压", "gaoya", qval(4096), 0x11),
                ("啊", "a", qval(3584), 0),
            ],
            &[],
        );
        let texts = strs(&["高压", "壶"]);
        let wids = vec![1i64, -1];
        // 窗口词 = 高压（wid 1）且 rd(壶)="hu" ≥2 字母 → 融
        assert_eq!(e.panel_fuse(&texts, &wids, &[], Some(1)), "高压壶");
        // 无窗口词 → 独立
        assert_eq!(e.panel_fuse(&texts, &wids, &[], None), "高压\u{1}壶");
        // 前槽 wid ≠ 窗口词 → 独立（中国人|迷 非窗口词头同律）
        assert_eq!(e.panel_fuse(&texts, &wids, &[], Some(0)), "高压\u{1}壶");
        // 链音节 1 字母（a）→ 即便窗口词也不融
        assert_eq!(
            e.panel_fuse(&strs(&["高压", "啊"]), &wids, &[], Some(1)),
            "高压\u{1}啊"
        );
    }

    /// |链|≥3 全独立（丢|黄|炸 全字链三槽; 多|熊皮|啊 非窗口词头）;
    /// 纯词槽列保持 \x01 边界
    #[test]
    fn fuse_long_chain_stays_independent() {
        let e = synth(&["a"], &[], &[], &[]);
        assert_eq!(
            e.panel_fuse(&strs(&["丢", "黄", "炸"]), &[-1, -1, -1], &[], None),
            "丢\u{1}黄\u{1}炸"
        );
        assert_eq!(
            e.panel_fuse(&strs(&["多", "熊皮", "啊"]), &[0, -1, -1], &[], None),
            "多\u{1}熊皮\u{1}啊"
        );
        assert_eq!(
            e.panel_fuse(&strs(&["我们", "缓和"]), &[0, 1], &[], None),
            "我们\u{1}缓和"
        );
        assert_eq!(e.panel_fuse(&strs(&["稳赚"]), &[0], &[], None), "稳赚");
    }

    // ---- 强词门 / 槽词跳选 ------------------------------------------

    /// 强词门 flags≠0（裹满/梦美 f0 隐没; 萌妹 f17 在板）; 负/越界 wid 恒假
    #[test]
    fn word_ok_strong_gate() {
        let e = synth(
            &["meng"],
            &[("meng", &["meng"])],
            &[
                ("梦美", "meng", qval(4096), 0),
                ("萌妹", "meng", qval(5928), 0x11),
            ],
            &[],
        );
        assert!(!e.panel_word_ok(0)); // 梦美 f0 → 隐没
        assert!(e.panel_word_ok(1)); // 萌妹 f17 → 在板
        assert!(!e.panel_word_ok(-1));
        assert!(!e.panel_word_ok(2)); // 越界假安全
    }

    /// span 槽词跳选律: 列内首个 flags≠0 词（梦美 f0 → 萌妹 f17 跳选）;
    /// 全 f0 列回落 oi0（我爱）; 非终端/无读音 → None
    #[test]
    fn slot_word_first_strong_else_oi0() {
        let e = synth(
            &["meng", "wo"],
            &[("meng", &["meng"]), ("wo", &["wo"])],
            &[
                ("梦美", "meng", qval(4096), 0),
                ("萌妹", "meng", qval(5928), 0x11),
                ("我爱", "wo", qval(4608), 0),
            ],
            &[("meng", &["梦美", "萌妹"]), ("wo", &["我爱"])],
        );
        assert_eq!(e.panel_slot_word("meng"), Some(1)); // 跳过 f0 梦美
        assert_eq!(e.panel_slot_word("wo"), Some(2)); // 全 f0 → oi0 兜底
        assert_eq!(e.panel_slot_word("bu"), None); // 无读音
        assert_eq!(e.panel_slot_word("me"), None); // 途中态
    }

    /// strong 版无 oi0 兜底（组组组 类 f0 词不得顶掉 retro 强词 稳赚）;
    /// 且槽词面须 ≥2 字（单字强词不作 strong 槽词）
    #[test]
    fn slot_word_strong_no_fallback() {
        let e = synth(
            &["meng", "wo", "ba"],
            &[("meng", &["meng"]), ("wo", &["wo"]), ("ba", &["ba"])],
            &[
                ("梦美", "meng", qval(4096), 0),
                ("萌妹", "meng", qval(5928), 0x11),
                ("我爱", "wo", qval(4608), 0),
                ("把", "ba", qval(3584), 0x11),
            ],
            &[("meng", &["梦美", "萌妹"]), ("wo", &["我爱"]), ("ba", &["把"])],
        );
        assert_eq!(e.panel_slot_word_strong("meng"), Some(1));
        assert_eq!(e.panel_slot_word_strong("wo"), None); // 全 f0 不兜底
        assert_eq!(e.panel_slot_word_strong("ba"), None); // 单字强词不作槽
        assert_eq!(e.panel_slot_word("ba"), Some(3)); // 非强版无 ≥2 字门
    }

    // ---- 槽字 / 读音 / 词频层 ---------------------------------------

    /// CHAR_COL 校准列优先（tuo 槽字 = 托 native oi2 vs 我方 oi0 脱 ——
    /// wenzuantuodun 槽字位逐位律）; 校准列外 k 回落 DAT oi 序
    /// （单字不做 flags 门: 脱/拖 f0 照发）
    #[test]
    fn syl_char_prefers_calibration_column() {
        let e = synth(
            &["tuo"],
            &[("tuo", &["tuo"])],
            &[
                ("脱", "tuo", qval(3584), 0),
                ("拖", "tuo", qval(3680), 0),
                ("托", "tuo", qval(3784), 0x11),
            ],
            &[("tuo", &["脱", "拖", "托"])],
        );
        assert_eq!(e.panel_syl_char("tuo", 0), Some(("托".into(), 2)));
        assert_eq!(e.panel_syl_char("tuo", 1), Some(("拖".into(), 1)));
        assert_eq!(e.panel_syl_char("tuo", 2), Some(("托".into(), 2)));
        assert_eq!(e.panel_syl_char("tuo", 3), None); // 列尽
        assert_eq!(e.panel_syl_char("tuz", 0), None); // 无读音
    }

    /// 槽字/词 → 读音字母串（w2r × variant0 拼接, 多音节直拼）;
    /// 词频层 lp（缺词 −8.0）; word_id int 安全壳（缺词 −1）
    #[test]
    fn slot_rd_lp_and_wid() {
        let e = synth(
            &["hu", "gao", "ya"],
            &[("hu", &["hu"]), ("gaoya", &["gao", "ya"])],
            &[
                ("壶", "hu", qval(3584), 0),
                ("高压", "gaoya", qval(4096), 0x11),
            ],
            &[],
        );
        assert_eq!(e.panel_slot_rd("壶"), "hu");
        assert_eq!(e.panel_slot_rd("高压"), "gaoya"); // gao+ya 拼接
        assert_eq!(e.panel_slot_rd("词典无词"), "");
        assert!((e.panel_slot_lp("壶") - lp_of(3584)).abs() < 1e-12);
        assert_eq!(e.panel_slot_lp("词典无词"), -8.0);
        assert_eq!(e.panel_wid("高压"), 1);
        assert_eq!(e.panel_wid("词典无词"), -1);
    }

    // ---- 窗口词 / 余段存活 ------------------------------------------

    /// 窗口词: 贪婪边界 ≤8 最大界处槽词; 界处无读音词 / 无 >0 界 → None
    #[test]
    fn win_word_at_max_greedy_bound() {
        let e = synth(
            &["gao", "ya"],
            &[("gaoya", &["gao", "ya"])],
            &[("高压", "gaoya", qval(4096), 0x11)],
            &[("gaoya", &["高压"])],
        );
        assert_eq!(e.panel_win_word("gaoya"), Some(0)); // 界 {0,3,5} → 5 处槽词
        assert_eq!(e.panel_win_word("gao"), None); // 界 3 处无读音词
        assert_eq!(e.panel_win_word("h"), None); // 无 >0 界 → wbd<2
    }

    /// 余段贪婪界存活（任一界 >0）: 在途歧义读音块保留判据
    /// （wenzuan|tuodun 余段有界在板 vs 死尾块无界不在板 同律）
    #[test]
    fn bounds_alive_keeps_ambiguous_block() {
        let e = synth(&["gao", "ya"], &[("gaoya", &["gao", "ya"])], &[], &[]);
        assert!(e.bounds_alive("gaox")); // gao 界存活
        assert!(e.bounds_alive("gaoyax"));
        assert!(!e.bounds_alive("xgao")); // 首字母坏 → 全程无界（死尾）
        assert!(!e.bounds_alive("x"));
    }

    // ---- 段C 悬空补全（音节边界律）-----------------------------------

    /// 词读音须恰好终结于覆盖 len(inp) 的音节（中国人民 终于 min ✓ 型）;
    /// 更长读音不恰好终结不发; 已覆盖（终端）输入无补全
    #[test]
    fn completions_end_exactly_at_cover() {
        let e = synth(
            &["zhong", "guo", "ren", "min", "z"],
            &[
                ("zhongguorenmin", &["zhong", "guo", "ren", "min"]),
                ("zhongguorenminz", &["zhong", "guo", "ren", "min", "z"]),
            ],
            &[("中国人们", "zhongguorenmin", qval(4096), 0x11)],
            &[("zhongguorenmin", &["中国人们"])],
        );
        let out = e.panel_completions("zhongguorenmi");
        assert_eq!(out.len(), 1); // len 15 的 zhongguorenminz 不恰好终结 → 不发
        assert_eq!(out[0].word, "中国人们");
        assert_eq!(out[0].reading, "zhong'guo'ren'min");
        assert_eq!(out[0].kind, Kind::Complete);
        assert!((out[0].logp - lp_of(4096)).abs() < 1e-12);
        // 尾段为完整音节（终端态）→ 无补全词
        assert!(e.panel_completions("zhongguorenmin").is_empty());
    }

    // ---- 面质门 ------------------------------------------------------

    /// 面质门 n≥14 键: 仅双槽双强词且尾槽词 lp ≤ −5.5（说唱|萌妹 在板;
    /// 很美 −4.90 / 三词槽 / 弱头 native 全无）; n≤13 结构门由调用方承担
    #[test]
    fn face_gate_long_input() {
        let e = synth(
            &["a"],
            &[("a", &["a"])],
            &[
                ("说唱", "a", qval(4096), 0x11), // −4.00
                ("萌妹", "a", qval(5928), 0x11), // −5.79 ≤ −5.5
                ("很美", "a", qval(5020), 0x11), // −4.90 > −5.5
                ("吃饭", "a", qval(4096), 0x11),
            ],
            &[],
        );
        let s = |t: &str, w: i64, rd: &str| {
            (t.to_string(), w, rd.to_string(), -4.0)
        };
        let long = "shuochangmengmei"; // 16 键
        let ok = [s("说唱", 0, "shuo"), s("萌妹", 1, "chang")];
        assert!(e.panel_face_ok("", long, Some(&ok)));
        // 尾槽词 lp −4.90 → 不发（北京大学|很美 native 无）
        let bad_lp = [s("北京", 0, "bei"), s("很美", 2, "jing")];
        assert!(!e.panel_face_ok("", long, Some(&bad_lp)));
        // 三词槽 → 不发（青岛|我去|吃饭 / 我们系|缓和|咖啡 同律）
        let three = [
            s("说唱", 0, "shuo"),
            s("萌妹", 1, "chang"),
            s("吃饭", 3, "le"),
        ];
        assert!(!e.panel_face_ok("", long, Some(&three)));
        // 弱头（f0 / 字面 wid −1）→ 不发
        let weak = [s("梦美", -1, "meng"), s("萌妹", 1, "chang")];
        assert!(!e.panel_face_ok("", long, Some(&weak)));
        // 字尾槽: 单字 × ≥2 字母音节 在板（北京|达）; 1 字母死
        let char_tail = [s("北京", 0, "bei"), s("达", -1, "da")];
        assert!(e.panel_face_ok("", long, Some(&char_tail)));
        let short_rd = [s("北京", 0, "bei"), s("达", -1, "a")];
        assert!(!e.panel_face_ok("", long, Some(&short_rd)));
        // n ≤13: 结构门由调用方承担 → 恒真
        assert!(e.panel_face_ok("", "shuochang", Some(&three)));
        assert!(e.panel_face_ok("", long, None));
    }

    // ---- §1 深列链 / covered 前缀链 ----------------------------------

    /// §1 深列链 — 完整音节切分边界律: zaoshanghao →
    /// [zaoshanghao, zaoshang, zao, za]（za = za|o|shang|hao 边界;
    /// zaoshan/zaosha 非任何完整切分边界 → 不在链）; 死尾 → 空链
    #[test]
    fn deep_chain_full_segmentation_boundaries() {
        let e = synth(
            &["za", "zao", "o", "shang", "hao"],
            &[
                ("za", &["za"]),
                ("zao", &["zao"]),
                ("zaoshang", &["zao", "shang"]),
                ("zaoshanghao", &["zao", "shang", "hao"]),
            ],
            &[],
            &[],
        );
        let chain = e.deep_chain("zaoshanghao");
        let letters: Vec<&str> =
            chain.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(letters, vec!["zaoshanghao", "zaoshang", "zao", "za"]);
        // 链节点附读音叶区间首（= rid; DFS 叶序 = 键升序）
        let rids: Vec<u32> = chain.iter().map(|(_, r)| *r).collect();
        assert_eq!(rids, vec![3, 2, 1, 0]);
        // 死尾（尾串不可完整切分）→ 链空
        assert!(e.deep_chain("zaoshangx").is_empty());
    }

    /// covered 前缀链 vs §1 在途链: 前缀不要求完整切分边界
    /// （cen 的 ce 无 za|o 型边界也发 — native zenme 行 10-85 = ze 列
    /// 实证; cengjing = [cengjing,ceng,cen,ce]); §1 仅完整边界
    /// （cengjing → [cengjing, ceng]）
    #[test]
    fn covered_prefix_chain_vs_deep_chain() {
        let e = synth(
            &["ce", "cen", "ceng", "jing"],
            &[
                ("ce", &["ce"]),
                ("cen", &["cen"]),
                ("ceng", &["ceng"]),
                ("cengjing", &["ceng", "jing"]),
            ],
            &[],
            &[],
        );
        let cov = e.deep_chain_covered("cengjing");
        let letters: Vec<&str> =
            cov.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(letters, vec!["cengjing", "ceng", "cen", "ce"]);
        let deep = e.deep_chain("cengjing");
        let letters2: Vec<&str> =
            deep.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(letters2, vec!["cengjing", "ceng"]);
    }

    /// §1 深列面板: 链节点校准列拼接（RD_COL zao'shang'hao 词列 →
    /// WORD_COL zaoshang 词列 → CHAR_COL zao 字列 → CHAR_COL za 字列）
    /// + 构造序钉分（终分 = BASE − EPS×i, LP_OVERRIDE 感知）
    #[test]
    fn deep_panel_rows_splice_columns_with_pin() {
        let e = synth(
            &["za", "zao", "o", "shang", "hao"],
            &[
                ("za", &["za"]),
                ("zao", &["zao"]),
                ("zaoshang", &["zao", "shang"]),
                ("zaoshanghao", &["zao", "shang", "hao"]),
            ],
            &[],
            &[],
        );
        let rows = e.deep_panel_rows("zaoshanghao", false).unwrap();
        // 列拼接序 = 链节点序 × 列原序（zaoshang 节点走 WORD_COL 词列;
        // 典缺字面直发 — native 在板先例）
        let mut expect: Vec<&str> =
            rd_col_get("zao'shang'hao").unwrap().to_vec();
        expect.extend(word_col_get("zaoshang").unwrap());
        expect.extend(char_col_get("zao").unwrap());
        expect.extend(char_col_get("za").unwrap());
        assert_eq!(
            rows.iter().map(|c| c.word.as_str()).collect::<Vec<_>>(),
            expect
        );
        assert!(rows.iter().all(|c| c.kind == Kind::Fbcol));
        for (i, c) in rows.iter().enumerate() {
            // py LP_OVERRIDE: 全串先查 → 表内词回写频序（咋 在 CHAR_COL za 列）
            let lp = lp_override_get(&c.word).unwrap_or(c.logp);
            // fbcol 行免 W14（G1 兜底字列免消费−音节先验）→ 终分 = bias + lp
            let final_score = c.score_bias + lp;
            assert!(
                (final_score - (PANEL_BASE - PANEL_EPS * i as f64)).abs()
                    < 1e-12,
                "i={i}"
            );
        }
    }

    // ---- W3 retro 兄弟面 / 段1.5 尾槽字列 ----------------------------

    /// retro 头词平读拆字兄弟面: wenzuantuodun #0 稳赚|托顿（词槽形）×
    /// #1 问|钻|托顿（平读槽形）同尾双形; 平读 == 词典读音 → 无兄弟面
    /// （说穿/某宝 律）
    #[test]
    fn retro_sibling_flat_reading_split() {
        let e = synth(
            &["wen", "zhuan", "zuan", "tuo", "dun"],
            &[
                ("wen", &["wen"]),
                ("zuan", &["zuan"]),
                ("wenzhuan", &["wen", "zhuan"]),
                ("wenzuan", &["wen", "zuan"]),
                ("tuodun", &["tuo", "dun"]),
            ],
            &[
                ("问", "wen", qval(3584), 0x11),
                ("钻", "zuan", qval(3680), 0x11),
                ("稳赚", "wenzhuan", qval(4096), 0x11),
                ("文钻", "wenzuan", qval(4096), 0x11),
                ("托顿", "tuodun", qval(4096), 0x11),
            ],
            &[("wen", &["问"]), ("zuan", &["钻"])],
        );
        let slots = vec![
            ("稳赚".to_string(), 2, "wenzuan".to_string(), -4.0),
            ("托顿".to_string(), 4, "tuodun".to_string(), -6.0),
        ];
        let (face2, lp) = e.panel_retro_sibling("稳赚\u{1}托顿", &slots).unwrap();
        // 平读 wen,zuan → CHAR_COL 槽字 问/钻（native #1 逐位）
        assert_eq!(face2, "问\u{1}钻\u{1}托顿");
        let want = e.panel_slot_lp("问") + e.panel_slot_lp("钻") + (-6.0)
            - 0.01;
        assert!((lp - want).abs() < 1e-12);
        // 文钻 词典读音 = wen'zuan == 平读 → 无兄弟面
        let slots2 = vec![
            ("文钻".to_string(), 3, "wenzuan".to_string(), -4.0),
            ("托顿".to_string(), 4, "tuodun".to_string(), -6.0),
        ];
        assert!(e.panel_retro_sibling("文钻\u{1}托顿", &slots2).is_none());
    }

    /// 段1.5 尾单字槽校准列替换（W1 律2）: mi 尾槽 → SLOT_COL mi 列
    /// （迷/米, cap 2+2 — native zhongguorenmi #0/#1 序）+ KBN 声母位
    /// 单替邻键面（mi→ni → 你, native #2 中国人\x01你）; 表外读音不发
    #[test]
    fn tail_slot_faces_replace_tail_char() {
        let e = synth(
            &["mi", "ni"],
            &[("mi", &["mi"]), ("ni", &["ni"])],
            &[
                ("中国人", "mi", qval(4096), 0x11), // 仅消费 wid 强词门
                ("迷", "mi", qval(3996), 0),
                ("米", "mi", qval(4100), 0),
                ("你", "ni", qval(3584), 0x11),
            ],
            &[("mi", &["迷", "米"]), ("ni", &["你"])],
        );
        let slots = vec![
            ("中国人".to_string(), 0, "zhongguoren".to_string(), -4.0),
            ("迷".to_string(), -1, "mi".to_string(), -3.9),
        ];
        let out = e.panel_tail_slot_faces("中国人\u{1}迷", &slots);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].0, "中国人\u{1}迷"); // 列首 = 原尾字
        assert_eq!(out[1].0, "中国人\u{1}米");
        assert_eq!(out[2].0, "中国人\u{1}你"); // KBN m→n → ni 列首 你
        // 面分 = 100 − 0.01×构造序（钉序占位分）
        assert!((out[0].1 - 100.0).abs() < 1e-12);
        assert!((out[1].1 - 99.99).abs() < 1e-9);
        assert!((out[2].1 - 99.98).abs() < 1e-9);
        // 表外读音（SLOT_COL 无列）→ 不发（回落原面）
        let slots2 = vec![
            ("中国人".to_string(), 0, "zhongguoren".to_string(), -4.0),
            ("哦".to_string(), -1, "xx".to_string(), -3.0),
        ];
        assert!(e.panel_tail_slot_faces("中国人\u{1}哦", &slots2).is_empty());
        // 尾槽非单字 / 面无槽界 → 不发
        let slots3 = vec![
            ("中国人".to_string(), 0, "zhongguoren".to_string(), -4.0),
            ("托顿".to_string(), -1, "tuodun".to_string(), -3.0),
        ];
        assert!(e.panel_tail_slot_faces("中国人\u{1}托顿", &slots3).is_empty());
        assert!(e.panel_tail_slot_faces("中国人", &slots).is_empty());
    }

    // ---- KBN 邻键替换 / 字面候选 / NNLM 回退 --------------------------

    /// span 单 KBN 邻键替换 → 词典词（尿素 d→s native 锚）;
    /// 无命中的替换零发射（纠错串须落在词典词上）
    #[test]
    fn kbn_sub_neighbor_word() {
        let e = synth(
            &["ni", "ao", "su"],
            &[("niaosu", &["ni", "ao", "su"])],
            &[("尿素", "niaosu", qval(4096), 0x11)],
            &[("niaosu", &["尿素"])],
        );
        let out = e.panel_kbn_sub("niaodu");
        assert_eq!(out.len(), 1); // 唯一命中 d→s（其余邻键串无词典词）
        assert_eq!(out[0].0, "niaosu");
        assert_eq!(out[0].2, 0); // 尿素 wid
        assert!(e.panel_kbn_sub("zzzzz").is_empty());
    }

    /// W3 词典缺失字面候选（忠国/井栏/wen native 在板先例）:
    /// pending 坯, lp=−8, rid<0 空读音; rid≥0 → variant0 撇点形
    #[test]
    fn mk_lit_pending_candidate() {
        let e = synth(
            &["zao", "shang", "hao"],
            &[("zaoshanghao", &["zao", "shang", "hao"])],
            &[],
            &[],
        );
        let c = e.mk_lit("忠国", -1);
        assert_eq!(c.word, "忠国");
        assert_eq!(c.logp, -8.0);
        assert_eq!(c.kind, Kind::Pending);
        assert_eq!(c.reading, "");
        assert_eq!(c.wid, -1);
        assert_eq!(e.mk_lit("井栏", 0).reading, "zao'shang'hao");
    }

    /// NNLM 不可用 → 栈序原样（合成引擎无 NNLM 段; py 同律）
    #[test]
    fn nnlm_rank_falls_back_to_pool_order() {
        let e = synth(&["a"], &[], &[], &[]);
        let pool = vec![
            ("和".to_string(), 1),
            ("好".to_string(), 2),
            ("还".to_string(), 3),
        ];
        assert_eq!(e.panel_nnlm_rank("成群", &pool), pool);
        assert_eq!(e.panel_nnlm_rank("", &pool), pool);
        let faces = vec![("今天\x01的天".to_string(), -5.0)];
        assert_eq!(e.panel_nnlm_rank_faces(&faces), faces);
    }
}
