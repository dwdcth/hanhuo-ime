//! boost 覆盖层（ADR-002）——忠实管线产出后的只增不改增强。
//!
//! B0 整句前置: 长输入时全消耗候选（reading 去撇号 == 输入）稳定前置。
//! 原生面板为渐进上屏设计（前缀词优先），整句候选沉底——整句意图下
//! 这是纯排序增量。B1.5 lmcn 链: 全消耗候选内部按 lmcn 双表整句分重排
//! （Viterbi 分词 → Σ uni_lp + Σ trans(wi,wi+1, 缺对→alpha(wi))），
//! 表源 = 外包回收 TSV（uni 58.1% / bigram 74.5% 覆盖，ADR-001 勘误注）。
//!
//! 关闭 boost = 逐位回到忠实模式（黄金 611 永远跑忠实模式作回归门）。

use crate::Cand;

/// boost 旋钮（默认值 = CLI --boost 全开）
#[derive(Clone, Debug)]
pub struct BoostCfg {
    /// B0: 输入字母数 ≥ 此值时整句前置（0 = 关）
    pub sentence_first_min_len: usize,
    /// B1.5: 全消耗候选按 lmcn bigram 链整句分重排
    pub lmcn_chain: bool,
    /// B2: ckvd 长尾短语注入（642 万——原生查询但面板不可见, boost 解冻;
    /// 注入项排在引擎原生 full 候选之后, weight 降序）
    pub ckvd_inject: bool,
    /// B2w: bwcn PC 三元组词注入（70.7 万词, +13 万覆盖; 频序降序,
    /// 位于变体之后、ckvd 之前——词典级优先于长尾）
    pub bwcn_inject: bool,
    /// B2.5: 尾槽词扫描——原生最优 DP 路径尾段读音列前 N 词生成
    /// 同拼异词整句变体（参考市政→参考实证类; 0=关）
    pub sentence_tail_sweep: usize,
    /// B3 前哨: promoted 合并集按引擎 NNLM 语境分重排
    /// （lmcn 负结果后的语境排序信号; 与 lmcn_chain 互斥, NNLM 优先）
    pub nnlm_order: bool,
    /// B2.6: 简拼表缺键时 DAT 制导展开（首字母约束沿词层 trie 游走,
    /// 替代盲笛卡尔积的前 N 条; v3 诊断 sent 70% 表缺键→兜底全灭）
    pub jianpin_dat: bool,
    /// B2w-σ: 注入词分尺度合并——按 score 插入 full（原生序保持, 平分
    /// 原生先）而非保守垫后。默认 false: 注入分=ln(词库频) 与引擎校准分
    /// 跨尺度, 合并有 v2 common 回归风险——开启前必须过零伤害门
    /// （ADR-002 附录12 top1 天花板 → 精化方向）
    pub inj_score_merge: bool,
    /// B2w-ρ: 外挂词典频次重标——full 候选被外挂词典（词+全拼双匹配）
    /// 覆盖时, 以 log10(词库频) 替换频次分量（Δ 并入 score 后重排）。
    /// 动机: wxq 语料怪癖（属组 lp−5.668 > 数组 −5.973, lmcn/THUOCL 均
    /// 无 属组）——THUOCL 数组=357,924 → −5.55 翻案。默认关: 词库频是
    /// 网域列表频非语料概率, 尺度漂移风险（lmcn 前科 −1.7pp）。
    /// **已过门（2026-11-08）: v2 common −1.3pp/rare −0.2pp 负结果**——
    /// 尺度漂移伤害>属组类翻案收益; 病理样本 Δ0.419<钉梯 0.5 间距也翻不动。
    /// 默认关, 负结果旋钮保留（同 lmcn_chain 惯例）
    pub dict_recal: bool,
    /// B2.7 简拼表键注入（超越档）: 输入恰为简拼表已有键时, 表内整键
    /// 候选置顶（tqsdyy→天青色等烟雨 在册 5382 却沉 rank11——表数据
    /// 优先于 fbcol 兜底列）。忠实门（golden）恒 false。
    pub jianpin_inject: bool,
    /// B6 表外组词: 双音节全拼的字级组词注入。输入恰切两规整音节时,
    /// 每音节 DAT 读音列 top-K 单字 × 字频积打分, 缺席对注入面板尾部
    /// （≤CAP 条, 钉分低于面板 min——只增不改, 原生序零扰动）。
    /// 动机: 词典无词条的双常见字组合（峰澈/澈雪 类生僻搭配、人名、
    /// 新造词）在忠实管线任何通道都不出场（exact 只枚举 wlist 词条;
    /// tick79n 叉积受 noun_tag 资产成员门; DP 句面仅在空面板发射）。
    /// 默认 false: 待 v2 common 零伤害门 + 探针集在场率评估后定档。
    pub compose_inject: bool,
    /// B7 句序列 NNLM 重排（需 --nnlm 模型在场; 分体布局后 wxq_dict 无内嵌）
    pub sentence_nnlm_rescore: bool,
    /// B8 全段变体束: 输入的 n-best 切分逐段建格（B2.5 只在最优 DP 路径
    /// 的段格上变词——bashe 列的 跋涉 不在 ba|she 单字段的任何列里,
    /// 双词路径 跋涉+远比 永不可达; B8 段覆盖多样化后词变体全域可达）。
    /// 值 = 变体产出预算（0=关）。评分/列深/beam 宽与 B2.5 同尺。
    pub sentence_seg_beam: usize,
    /// c.3 词块感知组装 × ctx-NNLM 重排（设计-上下文动态组词 §c.3,
    /// 验收探针 #2 未学习路径正解——B2.6 复活）: 非合法全拼切分输入
    /// （ewzdn/xmdbmh 类纯声母串）走词块优先组装（连声母位先匹配词典
    /// 多音节词块, 爆米花 b-m-h 整块拼入; 剩余声母位逐字 top-K）, 组装
    /// 为 **NNLM 引导扩展束**（逐位扩展, 每层按 NNLM(ctx, 序列) 截束——
    /// 「预筛打分在 beam 内做不入面板」; lp 只做层内分桶预截宽）, 终面
    /// 与面板 sentence 组并统一 NNLM 序写回（B7 律, 槽外追加 B2.7 律）。
    /// 门: ctx 非空 + NNLM 在场 + 旋钮开——三者缺一恒等（无 ctx 时逐字
    /// 频序仍对, golden/忠实链零行为）。实测: 块级信号成立（爆米花面
    /// 碾压 白马湖面）; 前缀级精确序（想买点 > 下面的）轻量模型不及,
    /// 待高级版 NNLM 臂（§6 #2, nnlm_adv predict 对接）。
    pub ctx_jp_compose: bool,
    /// B2w-τ 域意图全消耗提权: 外挂词典命中（全拼恰等输入+原生缺席）
    /// 的注入词钉序置顶（top+0.4−0.01i, B2.7 律, cap 3）——用户打全某
    /// 领域词拼音即强意图, 原生面板只有同拼异词。仅 --dict 域激活
    /// （v2 无 dict 恒等零伤害）; 保序垫后纪律不变, 提权是显式旋钮。
    pub inj_top_promote: bool,
    /// 混合输入（设计-混合输入 §2）: 全拼段∪声母段——见 boost::mixed_input
    pub mixed_input: bool,
    /// B2w-sent: 域词边进 B8 段变体束边格（设计-B2w句级域词格; 只进
    /// alts 路, 不碰 ckvd 段/面板主序——τ 提权域纪律）
    pub bwcn_sent: bool,
    /// 声调筛选（设计-声调筛选）: 调位验证全消耗候选——净输入位=
    /// 音节末 且 字×音节调号一致; 有过→未过全消耗候选移除,
    /// 零过→不过滤（优雅回退）。表缺/调位空恒等。
    pub tone_filter: bool,
}

impl Default for BoostCfg {
    fn default() -> Self {
        // 超越默认档 v2（附录20）: B0 + B2.5 尾槽 + B2 ckvd——四臂实证
        // common 全臂 70.9 完美零伤害, rare top10 +3.6pp, sent top10
        // +15.9pp。ckvd 生效需资产在场（CLI 资产在场即开）。
        // lmcn_chain 默认关: rare −1.0pp trade 明码（附录18）
        BoostCfg {
            sentence_first_min_len: 8,
            lmcn_chain: false,
            bwcn_inject: false,
            ckvd_inject: true,
            sentence_tail_sweep: 6,
            nnlm_order: false,
            jianpin_dat: false,
            inj_score_merge: false,
            dict_recal: false,
            jianpin_inject: true,
            compose_inject: false, sentence_nnlm_rescore: false,
            sentence_seg_beam: 0,
            ctx_jp_compose: false, inj_top_promote: false,
            mixed_input: false,
            bwcn_sent: false,
            // 声调筛选升默认（2026-11-10）: 只作用于数字输入（原整链空
            // → 纯增量）; 无数字输入逐位恒等（golden/v2 实证）。
            // --faithful 无 boost → 调号剥离不滤。
            tone_filter: true,
        }
    }
}

/// lmcn 链整句分: Viterbi 分词(uni 键, ≤6字词) → Σ lp + Σ trans。
/// 表缺失 → None（调用方跳过重排，保忠实序）。
pub fn lmcn_chain_score(text: &str) -> Option<f64> {
    use super::lmcn::{lmcn_index, LMCN_WORD_FLOOR};
    let idx = lmcn_index();
    if idx.is_empty() {
        return None;
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let neg = f64::NEG_INFINITY;
    let mut best = vec![neg; n + 1];
    let mut prev = vec![0usize; n + 1];
    best[0] = 0.0;
    for i in 0..n {
        if best[i] == neg {
            continue;
        }
        for l in 1..=6.min(n - i) {
            let w: String = chars[i..i + l].iter().collect();
            let lp = match idx.get_uni(&w) {
                Some(u) => u.0,
                // 缺词地板（lmcn_face 同源约定; 单字缺省略抬——无 slot lp 可用）
                None => LMCN_WORD_FLOOR,
            };
            let sc = best[i] + lp;
            if sc > best[i + l] {
                best[i + l] = sc;
                prev[i + l] = i;
            }
        }
    }
    if best[n] == neg {
        return None;
    }
    let mut words: Vec<String> = Vec::new();
    let mut i = n;
    while i > 0 {
        let p = prev[i];
        words.push(chars[p..i].iter().collect());
        i = p;
    }
    words.reverse();
    let mut s = best[n];
    for pair in words.windows(2) {
        let t = match idx.get_bg(&pair[0], &pair[1]) {
            Some(v) => v,
            // 缺对 → alpha(wi)（lmcn_face 首词对同律, 链式推广）
            None => idx.get_uni(&pair[0]).map(|u| u.1).unwrap_or(0.0),
        };
        s += t;
    }
    Some(s)
}

/// B2.5: 尾槽词扫描——最优 DP 路径尾段读音列前 n 词 → 同拼异词整句
/// 变体。kbest 只给切分多样性（每段取列首词）, 列深位词（市政列的
/// 实证）由此通道出场。分数沿用路径分（列序即频序, 稳定）。
pub(super) fn sentence_tail_sweep(
    e: &super::Engine,
    inp: &str,
    n: usize,
) -> Vec<Cand> {
    // v2 段格变体束: 每段读音列前 M 词组成格, Σ slot_logp beam 剪枝
    // 取前 n 变体（诊断实证: 剩余 miss 为任意段位的同音字变体——
    // 格式/各十、终局/中聚、归/规——尾-2 不可达, 全段格可）。
    // beam[0] = 原生最优（列首 = 频序首 = logp 最大）→ 过滤。
    const M: usize = 4; // 每段备选词数
    let Some((words, _sc, _)) = e.dp_path(inp) else {
        return Vec::new();
    };
    if words.len() < 2 {
        return Vec::new();
    }
    let cols: Vec<Vec<(String, f64)>> = words
        .iter()
        .map(|&(_w, rid)| {
            let (o0, o1) = e.reading_slice(rid);
            (o0..o1)
                .take(M)
                .map(|oi| {
                    let w = e.wlist_at(oi);
                    (e.word_text(w), e.slot_logp(w, oi))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if cols.iter().any(|c| c.is_empty()) {
        return Vec::new();
    }
    let k = n.max(12); // beam 宽（变体产出上限 = n）
    let mut beam: Vec<(f64, Vec<String>)> = vec![(0.0, Vec::new())];
    for col in &cols {
        let mut next: Vec<(f64, Vec<String>)> =
            Vec::with_capacity(beam.len() * col.len());
        for (s, ts) in &beam {
            for (t, lp) in col {
                let mut ts2 = ts.clone();
                ts2.push(t.clone());
                next.push((s + lp, ts2));
            }
        }
        next.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        next.truncate(k);
        beam = next;
    }
    let native: String =
        cols.iter().map(|c| c[0].0.as_str()).collect::<String>();
    let mut out = Vec::new();
    for (s, ts) in beam {
        if out.len() >= n {
            break;
        }
        let text: String = ts.concat();
        if text == native {
            continue; // 原生最优已在忠实输出在场
        }
        let mut c = Cand::new(text, s, crate::Kind::Sentence);
        c.score = s; // Cand::new 第二参落 logp 位——score 补设（显示/调试）
        c.logp = s;
        out.push(c);
    }
    out
}

/// B8 全段变体束: 段覆盖多样化。B2.5 的格建在 dp_path 唯一切分上——
/// 中段双词路径（bashe 列 跋涉 × yuanbi 列 远比）不在任何单字段的
/// 读音列里, beam/列深再宽也不可达（截断层=段覆盖, 非剪枝）。B8 对
/// 输入的 n-best 全消耗切分逐段建格（列深/beam/评分与 B2.5 同尺）,
/// 变体池按 Σ slot_logp 全局降序截 n。段格 #1 = dp_path 主支（全消耗
/// 最优）, 其列首拼接过滤（原生最优已在忠实输出在场——B2.5 同律;
/// 补全支输入由 B2.5 段格覆盖, B8 只管全消耗域）。
/// 跨度 [i,j) 局部 n-best 切分（n-best Viterbi, 回溯竞技场免克隆;
/// 段序列唯一决定路径——同 (前驱链, 末边) 不重复）。边表由调用方预建。
fn local_nbest(
    edges: &[Vec<(usize, Piece, f64)>],
    i: usize,
    j: usize,
    k: usize,
) -> Vec<(f64, Vec<Piece>)> {
    let mut arena: Vec<(usize, Piece, i32)> = Vec::new();
    let mut best: Vec<Vec<(f64, i32)>> =
        (i..=j).map(|_| Vec::new()).collect();
    best[0].push((0.0, -1));
    for p in i..j {
        if best[p - i].is_empty() {
            continue;
        }
        // 消费侧剪枝: 展开前降序截 k
        best[p - i].sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        best[p - i].truncate(k);
        let entries: Vec<(f64, i32)> = best[p - i].clone();
        for (q, pc, esc) in &edges[p] {
            if *q > j {
                continue;
            }
            for &(ps, pidx) in &entries {
                let idx = arena.len() as i32;
                arena.push((p, pc.clone(), pidx));
                best[q - i].push((ps + esc, idx));
            }
        }
    }
    let mut finals = std::mem::take(&mut best[j - i]);
    finals.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    finals.truncate(k);
    let mut out: Vec<(f64, Vec<Piece>)> = Vec::with_capacity(finals.len());
    for (sc, idx) in finals {
        let mut rids: Vec<Piece> = Vec::new();
        let mut cur = idx;
        while cur >= 0 {
            let (_p, ref rid, par) = arena[cur as usize];
            rids.push(rid.clone());
            cur = par;
        }
        rids.reverse();
        out.push((sc, rids));
    }
    out
}

/// B2w-sent 边格载荷: 主库 rid 或域词表下标（设计-B2w句级域词格 §2——
/// 域词边可与原生边同台造新切分, 汉隶/素胚类「正确切分需域词才存在」
/// 的缺口由 Dom 边直治; 列注入换词面是同台竞争的副产品）
#[derive(Clone, PartialEq, Debug)]
enum Piece {
    Rid(u32),
    Dom(usize),
}

/// 域词 lp 归一锚: log10 语料频次上限（lyric 59700 ≈ 4.78; THUOCL 同量级）
const DOM_LOG_W_MAX: f64 = 4.78;
/// 域词带压: 归一后 [0, -3.4] 平移到原生稀有带下方（域词只该在
/// 原生无好词的段赢——τ 提权域纪律, 实测可调 DOMAIN_TAU）
const DOM_LP_SHIFT: f64 = -3.2;
/// 每 (i,j) 位域词边上限（n-gram 包垃圾边防护）
const DOM_EDGE_PER_SPAN: usize = 2;

pub(super) fn sentence_seg_variants(
    e: &super::Engine,
    inp: &str,
    n: usize,
) -> Vec<Cand> {
    // recall 战役旋钮（§十二 后条件重测: 旧负结果是 B7cap16+污染
    // 电池下测的; 现 cap30+干净电池, M/变体裁量重开实验位）
    static MK: std::sync::OnceLock<(usize, usize, usize)> =
        std::sync::OnceLock::new();
    let (m_col, var_cap_big, seg_total) = *MK.get_or_init(|| {
        let g = |k: &str, d: usize| {
            std::env::var(k).ok()
                .and_then(|v| v.parse().ok()).unwrap_or(d)
        };
        (g("HANHUO_SEG_M", 4), g("HANHUO_SEG_VAR_BIG", 2),
         g("HANHUO_SEG_TOTAL", 32))
    });
    let M: usize = m_col; // 每段备选词数（B2.5 同尺; 旋钮可覆）
    let SPAN_LOC_K: usize = std::env::var("HANHUO_SEG_LOCK")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(6); // 跨度局部 n-best 宽（recall 战役旋钮）
    const SPAN_MAX: usize = 28; // 跨度字母上限（61 字句目标域 basheyuanbidaodamudi=20; 性能护栏）
    let SEG_TOTAL: usize = seg_total; // 段格总数上限（拼接去重后, 分降序截）
    const SEG_VAR_CAP: usize = 5; // 小段格（≤3 槽）变体上限——全单编辑覆盖（跋涉远比=第4位, 5 罩住）
    let SEG_VAR_CAP_BIG: usize = var_cap_big; // 大段格（>3 槽）变体上限——heads+首编辑（61 字句族编辑面配额爆炸, 代表性优先）。B2.8b 调参负结果（2026-11-10 实测: cap4+结构差异槽序 B7 臂 frag@10 94.2→90.4, 轮转选拔 →92.3——20 变体预算零和, 单段格扩容必饿死他段格; 远比 类第 2+ 便宜编辑的浮出受 B7 打分集 16 席与轻量 NNLM 判别力双卡, 排序残局非预算问题, 勿重试）
    let b = inp.as_bytes();
    let nn = b.len();
    if nn == 0 {
        return Vec::new();
    }
    // 词格边表（一次构建, 全程复用）: 边 = DAT 终端 ∪ 域词边,
    // 边分 = 读音列首 slot_logp（dp_path 同口径）/ 域词归一 lp
    let mut edges: Vec<Vec<(usize, Piece, f64)>> = vec![Vec::new(); nn];
    for i in 0..nn {
        for &(depth, st) in &e.dat.iter_terminal(&b[i..]) {
            let (lo, _cnt) = e.dat.range(st);
            let (o0, o1) = e.reading_slice(lo);
            if o1 <= o0 {
                continue;
            }
            let w = e.wlist_at(o0);
            // 神经 emission 加分（召回侧武器位: 知意 composition 头等
            // 逐槽字分布件经 set_emission_hook 注入——词面×字节起×
            // 音节数 → 加分叠加到段格边分; 无钩子 0.0 零行为）
            let bonus = if e.emission_hook_active() {
                let word = e.word_text(w);
                let nsyl = e
                    .rid_variants(lo)
                    .first()
                    .map(|v| v.len())
                    .unwrap_or(1);
                e.emission_bonus(&word, i, nsyl)
            } else {
                0.0
            };
            edges[i].push((
                i + depth,
                Piece::Rid(lo),
                e.slot_logp(w, o0) + bonus,
            ));
        }
    }
    // B2w-sent 域词边（§2）: 位段子串精确查 bwcn（HashMap 零成本）;
    // 旋钮关恒等。lp = log10(w) − DOM_LOG_W_MAX + DOM_LP_SHIFT——
    // 常见域词 ≈ −4.7、稀有 ≈ −8.1, 整带压在原生常见词下方
    let mut dom_tab: Vec<(String, f64)> = Vec::new();
    if e.boost.as_ref().is_some_and(|bb| bb.bwcn_sent)
        && !e.bwcn.borrow().is_empty()
    {
        for i in 0..nn {
            let jmax = (i + SPAN_MAX).min(nn);
            for j in i + 2..=jmax {
                if dom_tab.len() >= 512 {
                    break;
                }
                // 每 (i,j) 位 ≤2 边（垃圾边防护; 前置配额免事后重映射）
                for ph in e
                    .bwcn_lookup(&inp[i..j])
                    .into_iter()
                    .take(DOM_EDGE_PER_SPAN)
                {
                    let lp = (ph.weight as f64).log10()
                        - DOM_LOG_W_MAX
                        + DOM_LP_SHIFT;
                    let ix = dom_tab.len();
                    dom_tab.push((ph.text.clone(), lp));
                    edges[i].push((j, Piece::Dom(ix), lp));
                }
            }
        }
    }
    // 全局 1-best 前向/后向（拼接基座: 跨度外槽位取全局最优; 载 rid 链
    // ——段格 = 前缀链 ⊕ 跨度局部切分 ⊕ 后缀链）
    let neg = f64::NEG_INFINITY;
    let mut fwd: Vec<(f64, Vec<Piece>)> =
        (0..=nn).map(|_| (neg, Vec::new())).collect();
    let mut bwd: Vec<(f64, Vec<Piece>)> =
        (0..=nn).map(|_| (neg, Vec::new())).collect();
    fwd[0] = (0.0, Vec::new());
    bwd[nn] = (0.0, Vec::new());
    for i in 0..nn {
        if fwd[i].0 == neg {
            continue;
        }
        for &(j, ref pc, esc) in &edges[i] {
            let s = fwd[i].0 + esc;
            if s > fwd[j].0 {
                let mut chain = fwd[i].1.clone();
                chain.push(pc.clone());
                fwd[j] = (s, chain);
            }
        }
    }
    if fwd[nn].0 == neg {
        return Vec::new(); // 无全消耗切分（补全支域归 B2.5 段格）
    }
    for i in (0..nn).rev() {
        for &(j, ref pc, esc) in &edges[i] {
            if bwd[j].0 == neg {
                continue;
            }
            let s = esc + bwd[j].0;
            if s > bwd[i].0 {
                let mut chain = vec![pc.clone()];
                chain.extend_from_slice(&bwd[j].1);
                bwd[i] = (s, chain);
            }
        }
    }
    // 跨度拼接枚举: 跨度 (i,j) 的局部 n-best 切分 × 全局最优外套 =
    // 多局部变异路径的可达域。纯全局 n-best 够不着多变异切分（61 字
    // 句实证: bashe 合并 + 原币读音行 交换的双变异切分排名 48 外,
    // 单变异组合占据前排名次）; 跨度 (0,n) 退化即全局 n-best。
    let mut seg_pool: Vec<(f64, Vec<Piece>)> = Vec::new();
    for i in 0..nn {
        if fwd[i].0 == neg || edges[i].is_empty() {
            continue;
        }
        let jmax = (i + SPAN_MAX).min(nn);
        for j in (i + 2..=jmax).rev() {
            if bwd[j].0 == neg {
                continue;
            }
            let locals = local_nbest(&edges, i, j, SPAN_LOC_K);
            for (lscore, rids) in locals {
                let mut full = fwd[i].1.clone();
                full.extend(rids);
                full.extend_from_slice(&bwd[j].1);
                seg_pool.push((fwd[i].0 + lscore + bwd[j].0, full));
            }
        }
    }
    seg_pool.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // 段格去重 → 段格列表（分降序, #1 = 全局最优切分）。键 = 列首拼接
    // **词面**（rid 序列去不动 merely-partition 段格——的吧 合并与
    // 的|吧 切分 rid 不同而词面相同, 会以高频假多样性灌满前排名次;
    // 词面去重后预算自动流向真多样段格）
    let mut segs: Vec<(f64, Vec<Piece>)> = Vec::new();
    let mut seg_seen: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let head_text = |pcs: &[Piece]| -> String {
        pcs.iter()
            .filter_map(|pc| match pc {
                Piece::Rid(rid) => {
                    let (o0, o1) = e.reading_slice(*rid);
                    if o0 >= o1 {
                        return None;
                    }
                    Some(e.word_text(e.wlist_at(o0)))
                }
                Piece::Dom(ix) => Some(dom_tab[*ix].0.clone()),
            })
            .collect::<String>()
    };
    let has_dom = |pcs: &[Piece]| {
        pcs.iter().any(|pc| matches!(pc, Piece::Dom(_)))
    };
    let mut dom_segs = 0usize;
    for (sc, rids) in seg_pool {
        if segs.len() >= SEG_TOTAL + 8 {
            break; // 保位扫描护栏
        }
        let dom = has_dom(&rids);
        if segs.len() >= SEG_TOTAL && (!dom || dom_segs >= 8) {
            continue; // 已满: 只再收域词保位段格
        }
        if seg_seen.insert(head_text(&rids)) {
            dom_segs += dom as usize;
            segs.push((sc, rids));
        }
    }
    if segs.is_empty() {
        return Vec::new();
    }
    // 每段格逐列组 beam → 变体池（段格 #1 列首拼接 = 原生最优, 过滤）
    // 列构建 Piece 化: Rid → 读音列 top-M; Dom → 单词列（设计 §2）
    let cols_of = |pcs: &[Piece]| -> Vec<Vec<(String, f64)>> {
        pcs.iter()
            .map(|pc| match pc {
                Piece::Rid(rid) => {
                    let (o0, o1) = e.reading_slice(*rid);
                    (o0..o1)
                        .take(M)
                        .map(|oi| {
                            let w = e.wlist_at(oi);
                            (e.word_text(w), e.slot_logp(w, oi))
                        })
                        .collect::<Vec<_>>()
                }
                Piece::Dom(ix) => vec![dom_tab[*ix].clone()],
            })
            .collect()
    };
    let dbg = std::env::var("SEG_DBG").is_ok();
    if dbg {
        for (r, (sc, rids)) in segs.iter().enumerate() {
            eprintln!(
                "[seg] #{r} score={sc:.3} rids={:?} cols={:?}",
                rids,
                cols_of(rids)
            );
        }
    }
    let native: String = head_text(&segs[0].1);
    // 段格变体域 = 列首拼接（段格自身最优）+ **单编辑**变体（恰一槽
    // 取非列首词）——多编辑组合（八舍远比 族）罕见流畅且组合爆炸,
    // 不入池。
    //
    // 词面全局面重: merely-partition 段格（的吧 合并 ≡ 的|吧 切分,
    // 同字面不同切分）折叠为零增量。原生段格（#1）的 heads+全量单
    // 编辑面先入 seen 池（去重基座——其变体供给归 B2.5 尾槽, B8 不
    // 重复占预算）。
    //
    // 段内编辑序 = **结构差异槽优先**（rid ∉ 原生 rid 集的槽, 其列词
    // 替换是该段格存在的意义——跋涉段格的 远比 换原币）, 后公共槽
    // 编辑, 各自按分降序。段格序 = 差异槽数降序 × 分降序——纯分序
    // 下廉价单变异段格（的吧族）永远压过多变异段格（bashe+yuanbi
    // 双变异）, 结构多样性优先（最终序下游 B7 NNLM 定）。
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    seen.insert(native);
    // 单段格变体构建（edits_limit=None 全量——原生基座去重用）
    let seg_vars_of = |pcs: &[Piece],
                       seen: &mut std::collections::HashSet<String>,
                       cap: Option<usize>|
     -> Vec<(f64, String)> {
        let cols: Vec<Vec<(String, f64)>> = cols_of(pcs);
        if cols.iter().any(|c| c.is_empty()) {
            return Vec::new();
        }
        let heads: Vec<&(String, f64)> =
            cols.iter().map(|c| &c[0]).collect();
        let mut vars: Vec<(f64, String)> = Vec::new();
        let hscore: f64 = heads.iter().map(|(_, lp)| *lp).sum();
        let htext: String =
            heads.iter().map(|(t, _)| t.as_str()).collect();
        if seen.insert(htext.clone()) {
            vars.push((hscore, htext));
        }
        for (si, col) in cols.iter().enumerate() {
            for (t, lp) in col.iter().skip(1) {
                let s = hscore - heads[si].1 + lp;
                let text: String = heads
                    .iter()
                    .enumerate()
                    .map(|(sj, (ht, _))| {
                        if sj == si {
                            t.as_str()
                        } else {
                            ht.as_str()
                        }
                    })
                    .collect();
                if seen.insert(text.clone()) {
                    vars.push((s, text));
                }
            }
        }
        if let Some(c) = cap {
            vars.truncate(c);
        }
        vars
    };
    // 原生基座: heads+全量单编辑入 seen（B2.5 域, 不占 B8 预算）
    let _ = seg_vars_of(&segs[0].1, &mut seen, None);
    // 其余段格: 深变异优先 × 分降序。浅变异（对原生分差 ≤0.6——单
    // 廉价编辑等价面, 如 给人大来了 族重切分）与原生单编辑域高度重叠
    // 且 B7 序必沉底, 殿后; 深变异（跋涉远比到达墓地 族, 分差大但语
    // 境真多样性）先填。差异槽数加权（sc+λd）实测反效——廉价双区域
    // 垃圾切分（原币+到了+来了 族 d≈6）借 λ 冒顶压过真目标（d=4）,
    // 纯分序下深域内目标居 rank2-5（61 字句实测）。
    let sc0 = segs[0].0;
    let mut order: Vec<(usize, bool, f64)> = segs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(r, (sc, _rids))| (r, sc0 - sc > 0.6, *sc))
        .collect();
    order.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    let mut seg_vars: Vec<Vec<(f64, String)>> = Vec::new();
    for &(r, _deep, _key) in &order {
        let rids = &segs[r].1;
        let cols: Vec<Vec<(String, f64)>> = cols_of(rids);
        if cols.iter().any(|c| c.is_empty()) {
            continue;
        }
        let heads: Vec<&(String, f64)> =
            cols.iter().map(|c| &c[0]).collect();
        let hscore: f64 = heads.iter().map(|(_, lp)| *lp).sum();
        let htext: String =
            heads.iter().map(|(t, _)| t.as_str()).collect();
        let mut heads_var: Vec<(f64, String)> = Vec::new();
        if seen.insert(htext.clone()) {
            heads_var.push((hscore, htext));
        }
        // 单编辑: 分升序（编辑代价小者先——远比(1.53) 对 八舍(0.37):
        // 频序在此容忍, 配额 5 罩住两槽位型）
        let mut edits: Vec<(f64, usize, usize)> = Vec::new();
        for (si, col) in cols.iter().enumerate() {
            for (wi, (_t, lp)) in col.iter().enumerate().skip(1) {
                edits.push((hscore - heads[si].1 + lp, si, wi));
            }
        }
        edits.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut vars = heads_var;
        for (s, si, wi) in edits {
            if vars.len()
                >= if cols.len() <= 3 {
                    SEG_VAR_CAP
                } else {
                    SEG_VAR_CAP_BIG
                }
            {
                break;
            }
            let text: String = heads
                .iter()
                .enumerate()
                .map(|(sj, (ht, _))| {
                    if sj == si {
                        cols[si][wi].0.as_str()
                    } else {
                        ht.as_str()
                    }
                })
                .collect();
            if seen.insert(text.clone()) {
                vars.push((s, text));
            }
        }
        if !vars.is_empty() {
            if dbg {
                eprintln!(
                    "[segv] key={:.3} vars={:?}",
                    order
                        .iter()
                        .find(|(rr, _, _)| *rr == r)
                        .map(|(_, _, k)| *k)
                        .unwrap_or(0.0),
                    vars.iter().map(|(s, t)| (format!("{s:.2}"), t)).collect::<Vec<_>>()
                );
            }
            seg_vars.push(vars);
        }
    }
    // 选拔: 段格秩序填充预算（段格 #1 最优切分优先, merely-partition
    // 段格已被字面去重折叠）——纯全局降序会被常见单字段变体灌满
    // （吧设原币族 系统性压过 跋涉远比, 词频和偏爱常见字, 正是 B8
    // 要治的病）; 段内序 = 单编辑分降序。选拔序保留为输出序——B7
    // 打分集截断按面板序取前位, 多样性优先（最终序下游 NNLM 定）。
    let dom_texts: Vec<&str> =
        dom_tab.iter().map(|(t, _)| t.as_str()).collect();
    let mut picked: Vec<(f64, String)> = Vec::with_capacity(n);
    'outer: for vars in &seg_vars {
        for v in vars {
            if picked.len() >= n {
                break 'outer;
            }
            picked.push(v.clone());
        }
    }
    // B2w-sent 保位: 选拔序（分序）下域词变体恒沉底——若零在场, 从
    // seg_vars 直捞域词含面补至 n（frag 在场是硬指标; 排序交 B7）
    if dom_segs > 0
        && !picked
            .iter()
            .any(|(_, t)| dom_texts.iter().any(|d| t.contains(*d)))
    {
        'fill: for vars in &seg_vars {
            for (s, t) in vars {
                if picked.len() >= n {
                    break 'fill;
                }
                if dom_texts.iter().any(|d| t.contains(*d))
                    && !picked.iter().any(|(_, pt)| pt == t)
                {
                    picked.push((*s, t.clone()));
                }
            }
        }
    }
    picked
        .into_iter()
        .map(|(s, t)| {
            // 神经 emission 面分合成（召回侧闭环: 段格边选择之外,
            // 变体终分同吃槽字分布——「参考实证」类真词路径的 logP 域
            // 劣势由 emission 补偿; 无钩子零行为）。
            let s = if e.emission_hook_active() {
                s + e.emission_bonus(
                    &t,
                    0,
                    t.chars().count(),
                )
            } else {
                s
            };
            let mut c = Cand::new(t, s, crate::Kind::Sentence);
            c.score = s; // 显示/调试位（B2.5 同律; B7 重排会重锚）
            c.logp = s;
            c
        })
        .collect()
}

///
/// 触发域: 输入经 L1 Viterbi 恰切两规整音节（各 ≥2 字母）。每音节取
/// DAT 读音列（wlist 频序）前 [`COMPOSE_COL_K`] 个单字, 笛卡尔积全量
/// [`COMPOSE_INJECT_CAP`] = K² 条（面板在场词去重后）按字频积
/// （Σ slot_logp, 与 B2.5 尾槽同尺）降序。词条对由注入去重律天然
/// 排除（在场即跳过; tick79n noun_tag 叉积面同理）。词级生成设施全
/// 复用: DAT 列游走 = sentence_tail_sweep 同款 reading_slice/wlist_at;
/// 打分 = slot_logp。
///
/// 刻意的保守面（PoC v0 边界, 见 docs/设计-表外组词.md）:
/// - 仅双音节（探针集全域; 3+ 音节组合爆炸留待 beam 版）
/// - 不做 NNLM 重排（组合域 256 对 × ~1ms/步 前向不可用; 频积预筛
///   后 top-32 重排是后续实验臂）
/// - 注入钉分低于面板 min（在场即可口径; 原生 top-10 零位移）
pub(super) fn compose_pair_hits(
    e: &super::Engine,
    inp: &str,
    out: &[Cand],
) -> Vec<Cand> {
    // L1 切分门: 恰两规整音节
    let Some((syls, _)) = e.segment(inp) else {
        return Vec::new();
    };
    if syls.len() != 2 || syls.iter().any(|s| s.len() < 2) {
        return Vec::new();
    }
    // 每音节 DAT 读音列 top-K 单字（列序 = 频序; 扫描窗内滤单字）
    let col = |s: &str| -> Vec<(String, f64)> {
        let st = e.dat.walk(s.as_bytes(), crate::dat::ROOT);
        if st < 0 || !e.dat.is_terminal(st) {
            return Vec::new();
        }
        let (lo, _) = e.dat.range(st);
        let (o0, o1) = e.reading_slice(lo);
        (o0..o1.min(o0 + COMPOSE_COL_SCAN))
            .filter_map(|oi| {
                let w = e.wlist_at(oi);
                let t = e.word_text(w);
                if t.chars().count() != 1 {
                    return None;
                }
                Some((t, e.slot_logp(w, oi)))
            })
            .take(COMPOSE_COL_K)
            .collect()
    };
    let c1 = col(&syls[0]);
    let c2 = col(&syls[1]);
    if c1.is_empty() || c2.is_empty() {
        return Vec::new();
    }
    // 笛卡尔积 × 频积打分; 面板在场词去重（词条/变体/tick79n 面全covered）
    let have: std::collections::HashSet<&str> =
        out.iter().map(|c| c.word.as_str()).collect();
    let mut pairs: Vec<(f64, String)> =
        Vec::with_capacity(COMPOSE_COL_K * COMPOSE_COL_K);
    for (a, la) in &c1 {
        for (b, lb) in &c2 {
            let w = format!("{}{}", a, b);
            if have.contains(w.as_str()) {
                continue;
            }
            pairs.push((la + lb, w));
        }
    }
    pairs.sort_by(|x, y| {
        y.0.partial_cmp(&x.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let reading = format!("{}'{}", syls[0], syls[1]);
    let mut hits: Vec<Cand> = Vec::new();
    for (sc, w) in pairs {
        if hits.len() >= COMPOSE_INJECT_CAP {
            break;
        }
        let mut c = Cand::new(w, sc, crate::Kind::Fbcol);
        c.reading = reading.clone();
        c.score = sc;
        hits.push(c);
    }
    hits
}

/// 每音节单字列深。K 的语义 = 字级可达域: 目标对的两个字只要都落在
/// 各自读音列前 K 位（lp 序）即可被组出。探针集实测最深 汐/焰/砚 在
/// 14~40 位（面板序; lp 列序略有出入）——K=16 覆盖大多数, 深位字由
/// K 调参权衡（列深 ↑ = 组合域 ↑ = 深页垃圾量 ↑）。
const COMPOSE_COL_K: usize = 48;
/// 读音列单字扫描窗（列内多字词让位后仍能取满 K 个单字）。
const COMPOSE_COL_SCAN: usize = 64;
/// 注入上限。v1 = 全量 K×K（在场优先: 频积排序对「双中频字目标对」
/// 系统性不利——峰澈 输给 风扯 族单常见字×任意字垃圾, 截 top-N 会
/// 恰好滤掉目标; 全量注入皆钉分低于面板 min, 只增不改零位移, 代价
/// 仅深页长度）。排序质量升级路径（NNLM 预筛重排 / 位次和序）见
/// 设计文档——本常量届时收缩为重排后截断位。
const COMPOSE_INJECT_CAP: usize = COMPOSE_COL_K * COMPOSE_COL_K;

/// 在忠实管线输出上应用 boost（query() 尾部调用; boost=None 时零开销）。
/// tail_alts: B2.5 尾槽变体（引擎侧生成, 参数化）; ckvd_hits: B2 注入源。
/// 注入项落位（B2w-σ 两路径同律）:
/// - 保守（默认）: 尾部追加——原生校准序完全不动, 只增不改
/// - 分尺度合并: 按 score 插入（首个严格低分原生之前）——原生相互序
///   保持, 平分原生先于注入; 注入分=ln(词库频) 跨尺度竞争有回归风险,
///   开启前必须过 v2 common 零伤害门（ADR-002 附录12）
fn place_injected(
    base: &mut Vec<Cand>,
    injected: Vec<Cand>,
    score_merge: bool,
) {
    if !score_merge {
        base.extend(injected);
        return;
    }
    for c in injected {
        let pos = base
            .iter()
            .position(|f| f.score < c.score)
            .unwrap_or(base.len());
        base.insert(pos, c);
    }
}

pub(super) fn apply(
    inp: &str,
    cfg: &BoostCfg,
    out: &mut Vec<Cand>,
    tail_alts: &[Cand],
    bwcn_hits: &[hanhuo_dict::Phrase],
    ckvd_hits: &[hanhuo_dict::Phrase],
    nnlm_scorer: Option<&dyn Fn(&[String]) -> Option<Vec<f64>>>,
) {
    let b0 = cfg.sentence_first_min_len > 0 && inp.len() >= cfg.sentence_first_min_len;
    // B2: ckvd 注入不依赖 B0 门槛（短查询长尾也有价值）
    let mut injected: Vec<Cand> = Vec::new();
    // 注入去重域: 原生输出已有的词不再注入（wxq 已含 bwcn 70.7 万中 57.5 万）
    let native_words: std::collections::HashSet<String> =
        out.iter().map(|c| c.word.clone()).collect();
    // B2w bwcn 词典级词注入（变体后、ckvd 前）
    if cfg.bwcn_inject && !bwcn_hits.is_empty() {
        for p in bwcn_hits {
            if native_words.contains(&p.text) {
                continue;
            }
            if p.pinyin.split('\'').collect::<String>() != inp {
                continue;
            }
            let mut c = Cand::new(p.text.clone(), -1.0, crate::Kind::Sentence);
            c.reading = p.pinyin.clone();
            c.logp = -(p.weight as f64).ln().max(1.0); // 频次→logp 占位
            c.score = -c.logp;
            injected.push(c);
        }
    }
    // B2w-τ 域意图全消耗提权（bwcn 段专属——ckvd 长尾提权会伤 sent 面板,
    /// 实测 v2 sent top3 47.6→39.5; 钉序 cap3, B2.7 律）
    // 长度门（2026-11-11 sent 差分役）: 仅钉 ≥3 字候选——2 字包词同音
    // 面是劫持主体（v2 word:common top1 73.3→34.3 实测）; 精化二轮
    // 证伪（字母长门）: 2 字但全拼 ≥8 字母仍钉 → common 72.5→66.5
    // （−6.0）——长拼 2 字同音面同样致伤, 字母门无效, 严格字数门为准
    if cfg.inj_top_promote && !injected.is_empty() {
        let top_score = out.first().map(|c| c.score).unwrap_or(0.0);
        let mut v = injected;
        v.sort_by(|a, b| {
            b.logp.partial_cmp(&a.logp)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut head: Vec<&Cand> = Vec::new();
        let mut tail: Vec<Cand> = Vec::new();
        for c in &v {
            if head.len() < 3 && c.word.chars().count() >= 3 {
                head.push(c);
            } else {
                tail.push(c.clone());
            }
        }
        let mut promoted: Vec<Cand> = Vec::new();
        for (i, c) in head.iter().enumerate() {
            let mut c = (*c).clone();
            c.score = top_score + 0.4 - 0.01 * i as f64;
            promoted.push(c);
        }
        injected = tail;
        if !promoted.is_empty() {
            out.splice(0..0, promoted);
        }
    }
    if cfg.ckvd_inject && !ckvd_hits.is_empty() {
        let mut phs: Vec<&hanhuo_dict::Phrase> = ckvd_hits
            .iter()
            .filter(|p| {
                p.pinyin.split('\'').collect::<String>() == inp
                    && !native_words.contains(&p.text)
            })
            .collect();
        phs.sort_by(|a, b| b.weight.cmp(&a.weight));
        for p in phs {
            let mut c = Cand::new(p.text.clone(), -1.0, crate::Kind::Sentence);
            c.reading = p.pinyin.clone();
            c.logp = -(p.weight as f64); // 语料频次档占位（无原生校准 lp）
            injected.push(c);
        }
    }
    // B2w-ρ 外挂词典频次重标（词+全拼双匹配）: 频次分量换 log10(词库频),
    // Δ 并入 score 后重排。置于 b0 分支前——病理集中在短查询（属组>
    // 数组案 5 字节 < 8）, full 内版本够不着; 双匹配（词+全拼）条件天然
    // 只选中全消耗候选, 长输入部分词不受扰
    if cfg.dict_recal && !bwcn_hits.is_empty() {
        for c in out.iter_mut() {
            if let Some(p) = bwcn_hits.iter().find(|p| {
                p.text == c.word
                    && p.pinyin.split('\'').collect::<String>() == inp
            }) {
                let new_lp = -((p.weight as f64).log10());
                let delta = new_lp - c.logp;
                c.logp = new_lp;
                c.score += delta;
            }
        }
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    if !b0 {
        if injected.is_empty() {
            return;
        }
        // 无 B0 门槛时: 保守追加尾部或分尺度合并（B2w-σ 两路径同律）
        place_injected(out, injected, cfg.inj_score_merge);
        return;
    }
    let mut full: Vec<Cand> = Vec::new();
    let mut rest: Vec<Cand> = Vec::new();
    for c in out.drain(..) {
        // 全消耗判定: kind=Sentence（DP 全路径, 天然整句）或
        // reading 去撇号 == 输入; \u{1} 分隔展示形一律不算（有净形并存）
        let sep = c.word.contains('\u{1}');
        let is_full = (!sep && c.kind == crate::Kind::Sentence)
            || (!sep
                && !c.reading.is_empty()
                && c.reading.split('\'').collect::<String>() == inp);
        if is_full {
            full.push(c);
        } else {
            rest.push(c);
        }
    }
    if std::env::var("BOOST_DBG").is_ok() {
        eprintln!("[boost] inp={inp} full={} rest={}", full.len(), rest.len());
        for c in full.iter().take(2) {
            eprintln!("  FULL: {:?} rd={:?}", c.word, c.reading);
        }
        for c in rest.iter().take(2) {
            eprintln!("  rest: {:?} rd={:?} kind={:?} pw={:?}", c.word, c.reading, c.kind, c.path_words.as_ref().map(|v| v.len()));
        }
    }
    if full.is_empty() {
        // 空整句域: 注入项是唯一全消耗候选 → 置首（不能丢——
        // 无原生整句时（如四音节领域词）外挂词是该输入的唯一整词答案）
        injected.extend(rest);
        *out = injected;
        return;
    }
    // B2.5 尾槽变体: 原生 full 之后、ckvd 之前（同路径派生 > 外部长尾）
    let seen: std::collections::HashSet<&str> =
        full.iter().map(|c| c.word.as_str()).collect();
    let mut alts: Vec<Cand> = tail_alts
        .iter()
        .filter(|c| !seen.contains(c.word.as_str()))
        .cloned()
        .collect();
    alts.dedup_by(|a, b| a.word == b.word);
    full.append(&mut alts);
    // B3 前哨: NNLM 语境重排——仅组合域（尾槽变体在场=多词路径派生）。
    // 单字/单词域禁用: NNLM 无 ctx 的 unigram 信念劣于引擎校准词频
    // （eval 实证 word top1 -1.4pp; 同 lmcn 病理）。
    // 有不可编码项 → 整体放弃保序。
    // 域门已上移 query 供给侧（scorer 只在有变体或有 ctx 时传入）
    if cfg.nnlm_order && full.len() > 1 {
        if let Some(f) = nnlm_scorer {
            // 打分集截 top-12: RRF/纯 NNLM 超过 12 名对 top-10 度量无影响,
            // 而 NNLM 前向 ~1ms/步×候选数（v4 实测满量 492ms/条不可用）
            const SCORING_CAP: usize = 12;
            let n_score = full.len().min(SCORING_CAP);
            let words: Vec<String> =
                full.iter().take(n_score).map(|c| c.word.clone()).collect();
            if let Some(scores) = f(&words) {
                // 域分叉融合律:
                // - 变体域（tail_alts 非空, 候选=原生+束变体混源）: 纯
                //   NNLM 序——变体无引擎校准分, NNLM 是唯一可比信号
                //   （v2 sent 实证 44.0% vs RRF 40.8%）
                // - 词/语境域（同源原生候选）: RRF 倒数排名融合 k=60——
                //   引擎校准分与 NNLM 分尺度不可比, RRF 双榜一致优先
                //   （v4 实证 61.1% vs 纯 NNLM 59.0%）
                if !tail_alts.is_empty() {
                    let head: Vec<Cand> =
                        full.drain(..n_score).collect();
                    let mut idx: Vec<usize> = (0..head.len()).collect();
                    idx.sort_by(|&a, &b| {
                        scores[b]
                            .partial_cmp(&scores[a])
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let rest = full;
                    full = idx
                        .into_iter()
                        .map(|i| head[i].clone())
                        .chain(rest)
                        .collect();
                } else {
                const K: f64 = 60.0;
                let mut nn_idx: Vec<usize> = (0..n_score).collect();
                nn_idx.sort_by(|&a, &b| {
                    scores[b]
                        .partial_cmp(&scores[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let mut nn_rank = vec![0usize; n_score];
                for (r, &i) in nn_idx.iter().enumerate() {
                    nn_rank[i] = r;
                }
                let head: Vec<Cand> = full.drain(..n_score).collect();
                let mut fused: Vec<f64> = (0..n_score)
                    .map(|i| 1.0 / (K + i as f64) + 1.0 / (K + nn_rank[i] as f64))
                    .collect();
                let mut idx: Vec<usize> = (0..n_score).collect();
                idx.sort_by(|&a, &b| {
                    fused[b]
                        .partial_cmp(&fused[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                fused.clear();
                let rest = full;
                full = idx
                    .into_iter()
                    .map(|i| head[i].clone())
                    .chain(rest)
                    .collect();
                }
            }
        }
    }
    // B1.5 lmcn 链重排: NNLM 未启用时备用（互斥）
    else if cfg.lmcn_chain && full.len() > 1 {
        // 全局 lmcn 序（表外走地板沉底）。域门变体（表外锚定原位）已试并
        // 归档: 伤害全消（rare/sent 归零）但收益同步蒸发（common +2.0→
        // +0.2）——收益与伤害同源（表内金标×表外垃圾的跨层换位）, 单向
        // 门不可得, 维持全局序 + 旗标可组合（附录18）。
        full.sort_by(|a, b| {
            let sa = lmcn_chain_score(&a.word).unwrap_or(f64::NEG_INFINITY);
            let sb = lmcn_chain_score(&b.word).unwrap_or(f64::NEG_INFINITY);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    // B2 注入项落位: 变体之后（保守——原生校准优先, 长尾补检索）
    // 或分尺度合并（inj_score_merge: 按 score 插入, 原生序保持, 平分原生先）
    place_injected(&mut full, injected, cfg.inj_score_merge);
    full.extend(rest);
    // B3-fbcol（用户例证 2026-11-19: ctx=时间 × zhou → 轴应提权）:
    // 单字回退列的 ctx-NNLM 纯序重排——fbcol 分是伪频序（−0.00i
    // 等差）, NNLM 语境分是唯一真信号（RRF K=60 拉不开 2 位差,
    // 纯序直用）; top-12 截断同 B3 预算律。无 ctx 时 scorer 不传,
    // 恒等零行为。
    if cfg.nnlm_order {
        if let Some(f) = nnlm_scorer {
            let idx: Vec<usize> = out
                .iter()
                .enumerate()
                .filter(|(_, c)| c.kind == crate::Kind::Fbcol)
                .map(|(i, _)| i)
                .take(12)
                .collect();
            if idx.len() >= 2 {
                let words: Vec<String> = idx
                    .iter()
                    .map(|&i| out[i].word.clone())
                    .collect();
                if let Some(scores) = f(&words) {
                    let mut ord: Vec<usize> =
                        (0..idx.len()).collect();
                    ord.sort_by(|&a, &b| {
                        scores[b].partial_cmp(&scores[a])
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let ordered: Vec<Cand> = ord
                        .into_iter()
                        .map(|k| out[idx[k]].clone())
                        .collect();
                    for (&slot, c) in idx.iter().zip(ordered) {
                        out[slot] = c;
                    }
                }
            }
        }
    }
    *out = full;
}


/// B3-τ 语境意图提权（旋钮 nnlm_order, ctx 门）: 全消耗候选的
/// NNLM(ctx→词) 分显著领先面板头（margin ≥ TAU_MARGIN）时钉序置顶
/// ——用户选完上文即续打 = 搭配强意图信号（天下/世外桃源/邯郸学步
/// 类, B2w-τ「打全即意图」同哲学; RRF 融合对此太保守——v4 复测
/// +1.6pp 的补强件）。单提权位、只增不改: 无 ctx/无 margin/不可编码
/// 恒等。
pub(super) fn ctx_intent_promote(
    e: &crate::Engine,
    ctx: &str,
    out: &mut Vec<Cand>,
    clean: &str,
) {
    use crate::engine::Kind;
    if ctx.is_empty() || out.len() < 2 {
        return;
    }
    const N_SCORE: usize = 8; // NNLM 诊断打分集: 面板前 8 全消耗候选
    const TAU_MARGIN: f64 = 1.5; // nat; 低于此不视为强意图
    let idxs: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.kind == Kind::Exact
                || c.kind == Kind::Complete
                || c.kind == Kind::Sentence
        })
        .map(|(i, _)| i)
        .take(N_SCORE)
        .collect();
    // τ2 扫描窗 = 全部全消耗候选（注入词被保序垫后排在 top-8 外——
    // 卧薪+尝胆 案: 原生 场单/长单 族满窗, 尝胆 rank9+ 查表不到）;
    // 查表零 NNLM 成本, 窗即面板全长
    let full_idxs: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            !c.reading.is_empty()
                && c.reading.replace('\'', "") == clean
        })
        .map(|(i, _)| i)
        .collect();
    if idxs.len() < 2 {
        return;
    }
    // τ2 词典搭配直提（确定性, 先于 NNLM）: ctx 尾 2·3 字 × 候选词拼接
    // ∈ 主库词（邯郸+学步=邯郸学步 / 世外+桃源=世外桃源——网络错形
    // 先验下 NNLM 双双判反, 硬搭配查表即中; 最长尾优先, 面板序打破平局）
    {
        let ccs: Vec<char> = ctx.chars().collect();
        // 小包文本集（惰性: 只扫 ≤40000 条挂载包——手工/成语搭配包正解
        // 主库全缺的硬搭配, 天下+共主=天下共主 案）
        let pack_texts = {
            let mut slot = e.bwcn_texts.borrow_mut();
            slot.get_or_insert_with(|| {
                let mut m = std::collections::HashSet::new();
                for b in e.bwcn.borrow().iter() {
                    if b.len() <= 200_000 {
                        for v in b.map.values() {
                            for ph in v {
                                m.insert(ph.text.clone());
                            }
                        }
                    }
                }
                std::rc::Rc::new(m)
            })
            .clone()
        };
        for &i in &full_idxs {
            if i == 0 {
                continue;
            }
            let mut hit = None;
            for k in [3usize, 2] {
                if ccs.len() >= k {
                    let cat: String = ccs[ccs.len() - k..]
                        .iter()
                        .collect::<String>()
                        + &out[i].word;
                    if e.word_id(&cat).is_some()
                        || (!pack_texts.is_empty()
                            && pack_texts.contains(&cat))
                    {
                        hit = Some(k);
                        break;
                    }
                }
            }
            if let Some(k) = hit {
                if std::env::var("B3_DBG").is_ok() {
                    eprintln!("[b3τ] 搭配直提 k={k}: {}+{} ∈ 词典（主库/小包）",
                        ccs[ccs.len() - k..].iter().collect::<String>(),
                        out[i].word);
                }
                let top = out[0].score;
                let mut c = out[i].clone();
                c.score = top + 0.5; // B2.7 钉序律
                out.remove(i);
                out.insert(0, c);
                return;
            }
        }
    }
    let ctx_seq: Vec<u32> = match e.nnlm_encode(ctx) {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let seqs: Vec<Vec<u32>> = idxs
        .iter()
        .map(|&i| e.nnlm_encode(&out[i].word).unwrap_or_default())
        .collect();
    if seqs.iter().any(|s| s.is_empty()) {
        return; // 有不可编码项 → 放弃（B3 同律）
    }
    let Some(scores) = e.nnlm_score(&ctx_seq, &seqs) else {
        return;
    };
    // 最强语境候选 vs 面板头的 margin
    let (bi, bs) = scores
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, s)| (i, *s))
        .unwrap_or((0, 0.0));
    let head = scores[0];
    if std::env::var("B3_DBG").is_ok() {
        for (k, &i) in idxs.iter().enumerate() {
            eprintln!(
                "[b3τ] N={:.3} {}",
                scores[k], out[i].word
            );
        }
        eprintln!("[b3τ] 最强#{bi} {bs:.3} vs 头 {head:.3} margin={:.3}",
            bs - head);
    }
    // NNLM margin 提权裁撤（2026-11-11 实测: 成语集仅补 0 案,
    // v4 词域 -10.6pp——网络文本先验对搭配词判别不可靠, 双模型
    // 同栽 世外桃园 错形即证; 确定性 τ2 词典搭配直提是正解,
    // NNLM 分留 B3_DBG 诊断位）
    let _ = (bi, bs, head, TAU_MARGIN);
}

/// 声调筛选（设计-声调筛选 §2 过滤层, 旋钮 tone_filter）: 调位图
/// （净输入字节位→调号）验证全消耗候选——位=音节末 & 字×音节调号
/// 一致（轻声宽容: 任一侧 5 即过; 字/音节不在表 fail-open）。
/// ≥1 过 → 未过的全消耗候选移除; 零过/异构 → 不过滤（优雅回退）。
pub(super) fn tone_filter(
    e: &crate::Engine,
    out: &mut Vec<Cand>,
    clean: &str,
    tone_pos: &[(usize, u8)],
) {
    if tone_pos.is_empty() || out.len() < 2 {
        return;
    }
    let tab = {
        let mut slot = e.tone_tab.borrow_mut();
        slot.get_or_insert_with(super::load_tone_tab).clone()
    };
    if tab.is_empty() {
        if std::env::var("TONE_DBG").is_ok() {
            eprintln!("[tone] 表空——旋钮失效");
        }
        return;
    }
    // 全消耗判定+音节边界导出: reading 匹配净输入 → 用其权威切分;
    // 否则（Sentence 面常缺 reading）调表贪心游走——逐字在表中找
    // 净输入前缀匹配的音节（最长优先）, 恰好整消耗即全消耗。
    // 段元组 = (音节末位, 字, 音节串)。
    let seg_of = |c: &Cand| -> Option<Vec<(usize, char, String)>> {
        // 复合词面（窗口短语 \u{1} 连接形）按显示形（拼接）验证
        let chars: Vec<char> = c
            .word
            .chars()
            .filter(|ch| *ch != '\u{1}')
            .collect();
        if !c.reading.is_empty()
            && c.reading.replace('\'', "") == clean
        {
            let syls: Vec<&str> = c.reading.split('\'').collect();
            if syls.len() != chars.len() {
                return None;
            }
            let mut out = Vec::with_capacity(chars.len());
            let mut cum = 0usize;
            for (k, sy) in syls.iter().enumerate() {
                cum += sy.len();
                out.push((cum, chars[k], (*sy).to_string()));
            }
            return Some(out);
        }
        let mut out = Vec::with_capacity(chars.len());
        let mut pos = 0usize;
        for &ch in &chars {
            let m = tab.get(&ch)?;
            let mut sl: Vec<&String> = m.keys().collect();
            sl.sort_by_key(|k| std::cmp::Reverse(k.len()));
            let mut hit: Option<&String> = None;
            for syl in sl {
                if clean[pos..].starts_with(syl.as_str()) {
                    hit = Some(syl);
                    break;
                }
            }
            let syl = hit?;
            pos += syl.len();
            out.push((pos, ch, syl.clone()));
        }
        (pos == clean.len()).then_some(out)
    };
    let full: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, c)| seg_of(c).is_some())
        .map(|(i, _)| i)
        .collect();
    if std::env::var("TONE_DBG").is_ok() {
        eprintln!(
            "[tone] clean={clean:?} 调位={tone_pos:?} full={} / {}",
            full.len(),
            out.len()
        );
    }
    if full.len() < 2 {
        return;
    }
    let passes = |c: &Cand| -> bool {
        let Some(seg) = seg_of(c) else {
            return true; // 非全消耗不动
        };
        for &(p, t) in tone_pos {
            let Some(k) = seg.iter().position(|(b, _, _)| *b == p) else {
                return false; // 调位不落音节末
            };
            let (_, ch, syl) = &seg[k];
            let Some(m) = tab.get(ch) else {
                return true; // 字不在表 fail-open
            };
            let Some(ct) = m.get(syl) else {
                return true; // 字×音节不在表 fail-open
            };
            // 非对称宽容: 用户打 5（轻声意图）→ 任意候选调过;
            // 候选 ct=5（吧 类轻声读）不豁免——打 ba2 即非 吧 读
            if !(t == *ct || t == 5) {
                return false;
            }
        }
        true
    };
    let n_pass = full.iter().filter(|&&i| passes(&out[i])).count();
    if n_pass == 0 || n_pass == full.len() {
        return; // 零过回退 / 全过无操作
    }
    let keep: std::collections::HashSet<usize> = full
        .iter()
        .filter(|&&i| passes(&out[i]))
        .copied()
        .collect();
    let mut i = 0;
    out.retain(|_| {
        let keep_i = !full.contains(&i) || keep.contains(&i);
        i += 1;
        keep_i
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
#[test]
#[ignore = "真机: 需 nnlm_open_mobile_v2"]
fn nnlm_bench_b7_batch() {
    let bytes =
        std::fs::read("../../data/nnlm_open_mobile_v2.safetensors").unwrap();
    let m = hanhuo_lm::nnlm::Nnlm::from_safetensors(&bytes).unwrap();
    let base = "充满激情的跋涉远比到达大墓地的更能给人带来了去";
    // 单字变体 13 面（B7 形态: 同骨架一字之差）
    let mut seqs = Vec::new();
    for i in 0..13 {
        let mut c: Vec<char> = base.chars().collect();
        let n = c.len();
        c[(i * 4) % n] = ['吧','圆','鼻','刀','冲','及','情','德','八',
                          '舍','原','币','到'][i % 13];
        let s: String = c.into_iter().collect();
        seqs.push(m.encode(&s).unwrap_or_default());
    }
    // 预热
    let _ = m.score_candidates(&[], &seqs);
    std::env::set_var("NNLM_DBG", "1");
    let _ = m.score_candidates(&[], &seqs);
    std::env::remove_var("NNLM_DBG");
    let t0 = std::time::Instant::now();
    let iters = 20;
    for _ in 0..iters {
        let _ = m.score_candidates(&[], &seqs);
    }
    let dt = t0.elapsed().as_secs_f64() / iters as f64 * 1000.0;
    eprintln!("[bench] 13×{}字 批: {:.2} ms/call", base.chars().count(), dt);
    // 对照: 单序列
    let one = vec![seqs[0].clone()];
    let t1 = std::time::Instant::now();
    for _ in 0..iters {
        let _ = m.score_candidates(&[], &one);
    }
    eprintln!("[bench] 1×{}字: {:.2} ms/call", base.chars().count(),
        t1.elapsed().as_secs_f64() / iters as f64 * 1000.0);
}

    #[test]
    #[ignore = "真机"]
    fn tanh_share_bench() {
        // gelu tanh 份额: B7 批形态 13 行×22 步×3 层×768 宽 ≈ 690k 次
        let v: Vec<f32> = (0..690_000).map(|i| {
            ((i as f32 * 0.618) % 8.0) - 4.0
        }).collect();
        let t0 = std::time::Instant::now();
        let mut acc = 0f32;
        for &x in &v { acc += x.tanh(); }
        eprintln!("[tanh] 690k 次: {:.1}ms (acc={})", 
            t0.elapsed().as_secs_f64()*1000.0, acc);
    }

    // 局部差分证伪探针（整句排序残局 §瓶颈③）: 短窗 mobile NNLM 分辨力
    #[test]
    #[ignore = "真机: 需 nnlm_open_mobile_v2"]
    fn win_probe_endgame() {
        let bytes =
            std::fs::read("../../data/nnlm_open_mobile_v2.safetensors")
            .unwrap();
        let m = hanhuo_lm::nnlm::Nnlm::from_safetensors(&bytes).unwrap();
        let seq = |t: &str| m.encode(t).unwrap_or_default();
        let cases: &[(&str, &str, &str)] = &[
            ("全句",
             "充满激情的跋涉远比到达大墓地的更能给人带来了去",
             "充满激情的吧社圆鼻刀大墓地的更能给人带来了去"),
            ("窗1",
             "激情的跋涉远比到达大墓地",
             "激情的吧社圆鼻刀大墓地"),
            ("窗2",
             "跋涉远比到达目的地",
             "吧社圆鼻刀大目的"),
            ("歌词窗",
             "天青色等烟雨而我在等你",
             "天青色等烟雨儿我在等你"),
        ];
        for (n, a, b) in cases {
            let sc = m.score_candidates(&[], &[seq(a), seq(b)]);
            eprintln!("[win] {n}: 正确={:.3} 垃圾={:.3} Δ={:.3}",
                sc[0], sc[1], sc[0] - sc[1]);
        }
    }

    fn cand(word: &str, reading: &str) -> Cand {
        let mut c = Cand::new(word.into(), -1.0, crate::Kind::Fbcol);
        c.reading = reading.into();
        c
    }

    #[test]
    fn boost_b0_sentence_first_stable() {
        // 长输入: 全消耗候选前置, 其余保序在后
        let mut out = vec![
            cand("今天", "jin'tian"),
            cand("今天天气不错", "jin'tian'tian'qi'bu'cuo"),
            cand("进", "jin"),
            cand("今天天", "jin'tian'tian"),
        ];
        apply(
            "jintiantianqibucuo",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: false, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: false, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out,
            &[],
            &[],
            &[],
            None,
        );
        assert_eq!(out[0].word, "今天天气不错");
        assert_eq!(out[1].word, "今天"); // 其余稳定保序
        assert_eq!(out[2].word, "进");
        // 短输入不触发
        let mut out2 = vec![cand("你", "ni"), cand("你好", "ni'hao")];
        apply(
            "nihao",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: false, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: false, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out2,
            &[],
            &[],
            &[],
            None,
        );
        assert_eq!(out2[0].word, "你");
        // 无全消耗候选: 原序
        let mut out3 = vec![cand("今天", "jin'tian"), cand("进", "jin")];
        apply(
            "jintiantianqibucuo",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: true, bwcn_inject: false, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: false, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out3,
            &[],
            &[],
            &[],
            None,
        );
        assert_eq!(out3[0].word, "今天");
    }

    #[test]
    fn boost_predicate_sentence_kind_and_display_form() {
        // kind=Sentence + 空 reading（DP 全路径通道）→ 全消耗前置
        let mut c = Cand::new("目录结构".into(), -1.0, crate::Kind::Sentence);
        c.reading = String::new();
        let mut out = vec![
            Cand::new("目录".into(), -1.0, crate::Kind::Path),
            c,
            Cand::new("木".into(), -1.0, crate::Kind::Fbcol),
        ];
        out[0].reading = "mu'lu".into();
        out[2].reading = "mu".into();
        apply(
            "mulujiegou",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: false, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: false, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out,
            &[],
            &[],
            &[],
            None,
        );
        assert_eq!(out[0].word, "目录结构", "Sentence 空_reading 也应前置");

        // \u{1} 分隔展示形: 即使 reading 全串命中也不前置（净形并存语义）
        let mut disp = Cand::new(
            "目录\u{1}接\u{1}狗".into(),
            -1.0,
            crate::Kind::Fcomp,
        );
        disp.reading = "mu'lu'jie'gou".into();
        let mut clean = Cand::new("目录结构".into(), -1.0, crate::Kind::Sentence);
        clean.reading = String::new();
        let mut out2 = vec![disp, clean];
        apply(
            "mulujiegou",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: false, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: false, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out2,
            &[],
            &[],
            &[],
            None,
        );
        assert_eq!(out2[0].word, "目录结构", "净形 Sentence 前置");
        assert_eq!(out2[1].word, "目录\u{1}接\u{1}狗", "展示形不参与前置");
    }


    #[test]
    fn boost_b2_ckvd_inject_ordering() {
        use hanhuo_dict::Phrase;
        let mut native_full = Cand::new("目录结构".into(), -1.0, crate::Kind::Sentence);
        native_full.reading = String::new();
        let mut out = vec![native_full, Cand::new("目录".into(), -1.0, crate::Kind::Path)];
        out[1].reading = "mu'lu".into();
        let hits = vec![
            Phrase { text: "木路结构".into(), pinyin: "mu'lu'jie'gou".into(), weight: 1 },
            Phrase { text: "目录结构".into(), pinyin: "mu'lu'jie'gou".into(), weight: 3 },
            Phrase { text: "异键".into(), pinyin: "mu'lu'jie'gou'x".into(), weight: 9 }, // 非全消耗, 过滤
        ];
        apply(
            "mulujiegou",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: false, ckvd_inject: true, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: false, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out,
            &[],
            &[],
            &hits,
            None,
        );
        assert_eq!(out[0].word, "目录结构", "原生 full 优先");
        // B2w 去重律: 注入项与原生词面重复时过滤（目录结构 weight=3 复本不注入）
        assert!(!out[1..].iter().any(|c| c.word == "目录结构"), "注入复本去重");
        assert_eq!(out[1].word, "木路结构", "weight 降序");
        assert!(!out.iter().any(|c| c.word == "异键"), "非全消耗过滤");
        assert_eq!(out.last().unwrap().word, "目录", "原生 rest 殿后");
    }


    #[test]
    fn boost_nnlm_order_stable_and_bailout() {
        // stub scorer: 按词序给 [0.1, 0.9, 0.5] → 期望 idx1,2,0 序
        // 三个同拼异词（全消耗于同一输入）
        let mut out = vec![
            cand("甲乙丙", "jia'yi'bing"),
            cand("甲乙丁", "jia'yi'bing"),
            cand("甲乙戊", "jia'yi'bing"),
        ];
        let scorer = |w: &[String]| -> Option<Vec<f64>> {
            let _ = w;
            Some(vec![0.1, 0.9, 0.5, 0.7])
        };
        let alt = cand("甲乙己", "jia'yi'bing");
        apply(
            "jiayibing",
            &BoostCfg {
                sentence_first_min_len: 8,
                lmcn_chain: false,
                bwcn_inject: false,
                ckvd_inject: false,
                sentence_tail_sweep: 6,
                nnlm_order: true,
                jianpin_dat: false,
                inj_score_merge: false,
                dict_recal: false,
                jianpin_inject: false,
                compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0,
                ctx_jp_compose: false, inj_top_promote: false,
                mixed_input: false,
                bwcn_sent: false,
                tone_filter: false,
            },
            &mut out,
            &[alt],
            &[],
            &[],
            Some(&scorer),
        );
        // 变体域 → 纯 NNLM 序（变体无引擎校准分; RRF 会丢语境信号）
        assert_eq!(
            out.iter().map(|c| c.word.as_str()).collect::<Vec<_>>(),
            vec!["甲乙丁", "甲乙己", "甲乙戊", "甲乙丙"],
            "变体域纯 NNLM 分降序稳定重排"
        );
        // scorer 返回 None（不可编码）→ 整体保序
        let mut out2 = vec![
            cand("甲乙丙", "jia'yi'bing"),
            cand("甲乙丁", "jia'yi'bing"),
        ];
        let bail = |_: &[String]| -> Option<Vec<f64>> { None };
        let alt2 = cand("甲乙庚", "jia'yi'bing");
        apply(
            "jiayibing",
            &BoostCfg {
                sentence_first_min_len: 8,
                lmcn_chain: false,
                bwcn_inject: false,
                ckvd_inject: false,
                sentence_tail_sweep: 6,
                nnlm_order: true,
                jianpin_dat: false,
                inj_score_merge: false,
                dict_recal: false,
                jianpin_inject: false,
                compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0,
                ctx_jp_compose: false, inj_top_promote: false,
                mixed_input: false,
                bwcn_sent: false,
                tone_filter: false,
            },
            &mut out2,
            &[alt2],
            &[],
            &[],
            Some(&bail),
        );
        assert_eq!(out2[0].word, "甲乙丙", "放弃路径保序");
    }
    #[test]
    fn boost_nnlm_ctx_word_domain_no_alts() {
        // B3 语境门（v4）: 无变体的词域也允许 NNLM 重排——域门上移到
        // query 供给侧（有 ctx 或有变体才传 scorer）, apply 只看在场
        let mut out = vec![
            cand("甲乙丙", "jia'yi'bing"),
            cand("甲乙丁", "jia'yi'bing"),
        ];
        let scorer = |w: &[String]| -> Option<Vec<f64>> {
            let _ = w;
            Some(vec![0.2, 0.8])
        };
        apply(
            "jiayibing",
            &BoostCfg {
                sentence_first_min_len: 8,
                lmcn_chain: false,
                bwcn_inject: false,
                ckvd_inject: false,
                sentence_tail_sweep: 0,
                nnlm_order: true,
                jianpin_dat: false,
                inj_score_merge: false,
                dict_recal: false,
                jianpin_inject: false,
                compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0,
                ctx_jp_compose: false, inj_top_promote: false,
                mixed_input: false,
                bwcn_sent: false,
                tone_filter: false,
            },
            &mut out,
            &[],
            &[],
            &[],
            Some(&scorer),
        );
        // 2 项交叉（引擎[丙0丁1]×NNLM[丁0丙1]）→ RRF 并列, 稳定排序
        // 保引擎序——融合语义: NNLM 需压倒性优势才换位
        assert_eq!(out[0].word, "甲乙丙", "RRF 并列保引擎序");
    }

    #[test]
    fn boost_nnlm_cap12_tail_preserved() {
        // cap-12 边界: 打分集只取前 12——满量候选时 13+ 名保序殿后,
        // 且 RRF 只在被打分的头部内重排（性能轮 e287218 语义锁定）
        let mut out: Vec<Cand> = (0..15)
            .map(|i| {
                let mut c = cand(&format!("词{i}"), "jia'yi'bing");
                c.reading = "jia'yi'bing".into();
                c
            })
            .collect();
        let scorer = |w: &[String]| -> Option<Vec<f64>> {
            Some((0..w.len()).map(|i| -(i as f64)).collect()) // NNLM 序=引擎序
        };
        apply(
            "jiayibing",
            &BoostCfg {
                sentence_first_min_len: 8,
                lmcn_chain: false,
                bwcn_inject: false,
                ckvd_inject: false,
                sentence_tail_sweep: 0,
                nnlm_order: true,
                jianpin_dat: false,
                inj_score_merge: false,
                dict_recal: false,
                jianpin_inject: false,
                compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0,
                ctx_jp_compose: false, inj_top_promote: false,
                mixed_input: false,
                bwcn_sent: false,
                tone_filter: false,
            },
            &mut out,
            &[],
            &[],
            &[],
            Some(&scorer),
        );
        // NNLM 序与引擎序一致 → RRF 同序; 尾部 3 条不参与打分保序
        let words: Vec<&str> = out.iter().map(|c| c.word.as_str()).collect();
        assert_eq!(&words[..12], &(0..12).map(|i| format!("词{i}")).collect::<Vec<_>>().iter().map(|s| s.as_str()).collect::<Vec<_>>(), "头部保序（双榜一致）");
        assert_eq!(&words[12..], &["词12", "词13", "词14"], "cap 外保序殿后");
    }

    #[test]
    fn boost_lmcn_chain_score_semantics() {
        // 无表环境（CI/无 cascade 目录）: None → 调用方保忠实序
        // 有表环境由 eval 集成验证; 此处锁 Viterbi 边界: 空串/不可达
        match lmcn_chain_score("") {
            Some(s) => assert_eq!(s, 0.0),
            None => {} // 表缺失时统一 None, 合法
        }
    }

    #[test]
    fn boost_bwcn_inject_score_merge_semantics() {
        // B2w-σ 分尺度合并: 注入词按 score 插入 full——原生序保持,
        // 平分原生先, 高于注入分的原生不被压
        use hanhuo_dict::Phrase;
        let mk = |w: &str, sc: f64| {
            let mut c = Cand::new(w.into(), -1.0, crate::Kind::Path);
            c.score = sc;
            c
        };
        // 原生 full: 分 9.0 / 7.0 / 5.0（引擎序）
        let mut out = vec![mk("原生甲", 9.0), mk("原生乙", 7.0), mk("原生丙", 5.0)];
        out[0].reading = "ni'hao".into(); // 全消耗标记
        let hits = vec![
            Phrase { text: "注入高".into(), pinyin: "ni'hao".into(), weight: 8_000 },
            Phrase { text: "注入平".into(), pinyin: "ni'hao".into(), weight: 1_096 },
            Phrase { text: "注入低".into(), pinyin: "ni'hao".into(), weight: 20 },
        ];
        apply(
            "nihao",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: true, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: true, dict_recal: true, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out,
            &[],
            &hits,
            &[],
            None,
        );
        // 注入分=ln(weight): 注入高=8.99 → 甲(9.0)后乙(7.0)前;
        // 注入平=ln(1096)≈6.9996 < 乙(7.0) → 乙后丙前（严格高分才越位）;
        // 注入低=3.0 → 丙后
        let words: Vec<&str> = out.iter().map(|c| c.word.as_str()).collect();
        assert_eq!(
            words,
            vec!["原生甲", "注入高", "原生乙", "注入平", "原生丙", "注入低"],
            "分尺度合并: 原生序保持, 严格高分才越位, 平分原生先"
        );
    }

    /// B2w-ρ 外挂词典频次重标语义锁: Δ=log10(词库频)−引擎 lp 并入 score;
    /// 双匹配（词+全拼）才覆盖; Δ<钉梯间距不翻序, Δ 足够则翻——
    /// 以及 v2 负结果（common −1.3pp）的机制面在此可复算
    #[test]
    fn boost_dict_recal_semantics() {
        use hanhuo_dict::Phrase;
        let mk = |w: &str, lp: f64, sc: f64| {
            let mut c = Cand::new(w.into(), lp, crate::Kind::Exact);
            c.score = sc;
            c.reading = "shu'zu".into();
            c
        };
        let hits = vec![Phrase {
            text: "数组".into(),
            pinyin: "shu'zu".into(),
            weight: 357_924, // log10=5.554; 引擎 lp −5.973 → Δ=+0.419
        }];
        // 案1: Δ0.419 < 钉梯 0.5 → 不翻序（属组案实测形态）
        let mut out = vec![mk("属组", -5.668, 5.0), mk("数组", -5.973, 4.5)];
        apply(
            "shuzu",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: true, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: true, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out,
            &[],
            &hits,
            &[],
            None,
        );
        assert_eq!(out[0].word, "属组", "Δ<钉梯不翻序");
        let sz = out.iter().find(|c| c.word == "数组").unwrap();
        assert!((sz.logp + 5.5538).abs() < 1e-3, "logp 换词库 log10 频");
        assert!((sz.score - 4.919).abs() < 1e-3, "Δ 并入 score");
        // 案2: Δ>0.5 翻序——注意方向: lp=−log10(频), 词库频 1e3 → lp −3
        // → Δ=+2.97（低词库频反而大 boost——正是 v2 负结果的病根:
        // 词库低频词以小 |lp| 覆盖引擎校准, 尺度漂移放大噪声）
        let mut out2 = vec![mk("属组", -5.668, 5.0), mk("数组", -5.973, 4.5)];
        let hits2 = vec![Phrase {
            text: "数组".into(),
            pinyin: "shu'zu".into(),
            weight: 1_000,
        }];
        apply(
            "shuzu",
            &BoostCfg { sentence_first_min_len: 8, lmcn_chain: false, bwcn_inject: true, ckvd_inject: false, sentence_tail_sweep: 0, nnlm_order: false, jianpin_dat: false, inj_score_merge: false, dict_recal: true, jianpin_inject: false, compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0, ctx_jp_compose: false, inj_top_promote: false, mixed_input: false, bwcn_sent: false, tone_filter: false },
            &mut out2,
            &[],
            &hits2,
            &[],
            None,
        );
        assert_eq!(out2[0].word, "数组", "Δ>钉梯翻序");
    }

    #[test]
    fn boost_bwcn_inject_empty_full_domain_word_first() {
        // 回归锁: 无原生整句（四音节领域词, 面板全是 pending 部分候选）时,
        // full 恒空 → 早退路径不得丢注入项; 注入词是唯一全消耗候选, 置首
        use hanhuo_dict::Phrase;
        let mut partial = Cand::new("至臻".into(), -1.0, crate::Kind::Pending);
        partial.reading = "zhi'zhen".into();
        let mut out = vec![partial];
        let hits = vec![Phrase {
            text: "指针越界".into(),
            pinyin: "zhi'zhen'yue'jie".into(),
            weight: 800,
        }];
        apply(
            "zhizhenyuejie",
            &BoostCfg {
                sentence_first_min_len: 8,
                lmcn_chain: false,
                bwcn_inject: true,
                ckvd_inject: false,
                sentence_tail_sweep: 0,
                nnlm_order: false,
                jianpin_dat: false,
                inj_score_merge: false,
                dict_recal: false,
                jianpin_inject: false,
                compose_inject: false, sentence_nnlm_rescore: false, sentence_seg_beam: 0,
                ctx_jp_compose: false, inj_top_promote: false,
                mixed_input: false,
                bwcn_sent: false,
                tone_filter: false,
            },
            &mut out,
            &[],
            &hits,
            &[],
            None,
        );
        assert_eq!(out[0].word, "指针越界", "空整句域: 注入词置首");
        assert_eq!(out.len(), 2, "pending 部分候选保留殿后");
        assert_eq!(out[1].word, "至臻");
    }
}

/// B7 句序列 NNLM 重排: dp 句候选按全句字符序列 logprob 纯 NNLM 排序
/// （B3 变体域律——同长候选原分是词频和, 对流畅性失聪: 吧社圆鼻 vs
/// 跋涉远比 词频和同量级, LM 序列打分一眼定胜负）。
/// v2 语义（B8 配套修正）: **组内原位换位**——句组成员只在自身占据的
/// 面板槽位内重排, 不做全局分数排序（v1 全局 sort 实证把句组锚定分
/// -4.7 压到 rank18+, 单字候选 -1.3 浮顶——B0 整句前置结构被冲垮,
/// EVAL_SENT_NN 臂 top1 0% 的直接病根）。打分集截 [`RESCORE_CAP`]
/// （NNLM 前向 ~1ms/步 × 63 字候选, 满量 30+ 条不可用; B3 SCORING_CAP
/// 同尺）; 不可编码者按原序缀于可编码者后。分数=打分集原分最大值锚定
/// 递减 0.001（组内显示单调）。
pub(super) fn sentence_nnlm_rescore(e: &crate::Engine, out: &mut Vec<Cand>) {
    use crate::engine::Kind;
    // 统一入口（高级版优先/轻量惰性——注入即换脑, B7 语境序自动升级）
    if !e.nnlm_ok() {
        return;
    }
    let idxs: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == Kind::Sentence)
        .map(|(i, _)| i)
        .collect();
    if idxs.len() < 2 {
        return;
    }
    // 打分集自适应截断: 成员数 ≤ RESCORE_CAP 且 Σ字符 ≤ RESCORE_CHAR
    // （NNLM 前向 ~0.12ms/字, 61 字句满量 36 条 = 270ms 爆预算——字符
    // 预算使长句自动收窄打分面, 短句全量; 恒含前 2 名保重排语义）
    let (rescore_cap, rescore_char) = rescore_limits();
    let mut n_score = 0usize;
    let mut chars = 0usize;
    for &i in &idxs {
        let l = out[i].word.chars().count();
        if n_score >= 2 && (n_score >= rescore_cap || chars + l > rescore_char) {
            break;
        }
        chars += l;
        n_score += 1;
    }
    let mut valid: Vec<(usize, Vec<u32>)> = Vec::new();
    for &i in &idxs[..n_score] {
        if let Some(ids) = e.nnlm_encode(&out[i].word) {
            if !ids.is_empty() {
                valid.push((i, ids));
            }
        }
    }
    if valid.len() < 2 {
        return;
    }
    let seqs: Vec<Vec<u32>> = valid.iter().map(|(_, s)| s.clone()).collect();
    let Some(scores) = e.nnlm_score_noctx(&seqs) else {
        return;
    };
    if std::env::var("SENTNN_DBG").is_ok() {
        eprintln!(
            "[snn] 打分集 {}/{} (cap {} char {}/{}):",
            n_score, idxs.len(), rescore_cap, chars, rescore_char
        );
        for ((i, _), sc) in valid.iter().zip(scores.iter()) {
            eprintln!("[snn] N={:.3} 原={:.3} {}", sc, out[*i].score, out[*i].word);
        }
    }
    let mut ranked: Vec<(usize, f64)> = valid
        .iter()
        .map(|(i, _)| *i)
        .zip(scores.into_iter())
        .collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| out[b.0].score.total_cmp(&out[a.0].score))
    });
    // 新序 = 可编码（NNLM 降序）⊕ 不可编码（原序）⊕ cap 外（不动）
    let encodable: std::collections::HashSet<usize> =
        valid.iter().map(|(i, _)| *i).collect();
    let ordered: Vec<usize> = ranked
        .iter()
        .map(|(i, _)| *i)
        .chain(
            idxs[..n_score]
                .iter()
                .copied()
                .filter(|i| !encodable.contains(i)),
        )
        .collect();
    let anchor = ordered
        .iter()
        .map(|&i| out[i].score)
        .fold(f64::NEG_INFINITY, f64::max);
    // 原位写回: ordered[r] 的候选落入句组第 r 个槽位（idxs 升序）
    let moved: Vec<Cand> = ordered.iter().map(|&i| out[i].clone()).collect();
    for (r, &slot) in idxs[..n_score].iter().enumerate() {
        let mut c = moved[r].clone();
        c.score = anchor - 0.001 * r as f64;
        out[slot] = c;
    }
}

/// B7 打分集上限（组内前位优先——B0 前置态下 = 引擎序最优变体在前;
/// B8 臂下面板序 native+B8+tail, cap 需罩住 B8 多样性位。前缀共享使
/// 变体打分增量成本低）
const RESCORE_CAP: usize = 160;
/// B7 打分集字符预算（长句自适应收窄——61 字满量实测 270ms 不可用;
/// 2026-11-16 16/800→30/2400: +43% 相对; 2026-11-20 30/2400→160/8000
/// 宽档升默认: 单 query 实测 1.04×（157→163ms, 69 候选/990 字符——
/// score_candidates 分桶裂变+跨组合批兑现「前缀共享再榨」, 曾记录的
/// 电池口径 1.8× 是评测全管线开销非引擎时延）; top1 14.9/recall 21.7
/// （+13% 相对, 池150 配套——CLI --boost-sent-nn 默认注入））
const RESCORE_CHAR: usize = 8000;

/// cap 放宽钮（咨询三 §十: torch 全量打分 34 命中 vs 引擎 23——差值
/// 在 cap 截断; HANHUO_RESCORE_CAP/HANHUO_RESCORE_CHAR 覆盖默认。
/// 代价=NNLM 前向时延线性涨（~0.12ms/字, 组批落地前不进产品默认）。
fn rescore_limits() -> (usize, usize) {
    use std::sync::OnceLock;
    static L: OnceLock<(usize, usize)> = OnceLock::new();
    *L.get_or_init(|| {
        let cap = std::env::var("HANHUO_RESCORE_CAP")
            .ok().and_then(|v| v.parse().ok())
            .unwrap_or(RESCORE_CAP);
        let ch = std::env::var("HANHUO_RESCORE_CHAR")
            .ok().and_then(|v| v.parse().ok())
            .unwrap_or(RESCORE_CHAR);
        (cap, ch)
    })
}

/// c.3 词块感知组装 × ctx-NNLM 统一重排（设计-上下文动态组词 §c.3;
/// 验收探针 #2 未学习路径正解——B2.6 复活）。
///
/// 域门（全过才激活, 任一缺失恒等返回）:
/// 1. 旋钮 ctx_jp_compose 开（boost 层纪律——golden/忠实链零行为）
/// 2. ctx 非空（无 ctx 时逐字频序仍对——哦我知道你 赢是正确的, 不动）
/// 3. NNLM 在场
/// 4. 输入非合法全拼切分（与 c.2 桥同判定: 贪心音节 reach 不可达全长）
///    且 3..=12 位——纯声母串/死切分域
///
/// 机制: 词块枚举（[`Engine::jianpin_block_paths`]——连声母位先匹配词典
/// 多音节词块, 爆米花 b-m-h 整块拼入; 剩余位单字 top-K）产出面集, 与
/// 面板已有 sentence 组并, **不比面板分**（面板分与路径分不可比——
/// B2.6 当年金标沉底根因）, 直接 NNLM(ctx, 词面) 纯序统一重排
/// （B7 变体域先例）。写回 = sentence 槽原位（槽位数不变, 面板长度与
/// 其他候选位次零扰动; B7 律）; 无 sentence 槽时按 B2.7 钉分律注入顶部。
pub(super) fn ctx_jp_compose(
    e: &crate::Engine,
    inp: &str,
    ctx: Option<&str>,
    out: &mut Vec<Cand>,
) {
    use crate::engine::Kind;
    let dbg = std::env::var("CTX_DBG").is_ok();
    let Some(ctx) = ctx.filter(|c| !c.is_empty()) else {
        return;
    };
    if !e.nnlm_ok() {
        return;
    }
    // 域门 4: 非合法全拼切分（c.2 桥同律——贪心 reach 判定）
    let b = inp.as_bytes();
    if b.is_empty() || !(3..=12).contains(&b.len()) {
        return;
    }
    let mut reach = vec![false; b.len() + 1];
    reach[0] = true;
    for i in 0..b.len() {
        if !reach[i] {
            continue;
        }
        for j in i + 1..=(i + 7).min(b.len()) {
            if e.seg.is_syl(&inp[i..j]) {
                reach[j] = true;
            }
        }
    }
    if reach[b.len()] {
        return; // 纯全拼切分 → 非本域（走原生 DP 链; 混合键含声母死段
        // 天然不可达 → 混合域自然进入本通道——设计-混合输入 §2.3）
    }
    // 词块部件 → **NNLM 引导扩展束**（「预筛打分在 beam 内做不入面板」
    // 的字面实现）: 逐字母位扩展, 每位转移 = 单字词 top-N ∪ 多音节词块;
    // 每层收齐后按 NNLM(ctx, 序列) 截束（lp 无从保住 想|买|点 这类中频
    // 逐字组合——lp 全线平庸, 但 ctx=想去看电影 下逐步 NNLM 必然挑
    // 想→买→点→爆米花）。lp 只做层内预截宽（控打分成本）; 束分只用于
    // 剪枝, 入面板的仍是终面统一重排分。
    // 混合输入适配（设计-混合输入 §2.3 有 ctx 直接进 c.3）: 混合键部件层
    // 跑虚拟槽位（全拼段折叠 1 槽 + 精确音节约束——单字列收窄与块 DFS
    // 过约束都在 constrained 版内）; 纯声母串 mixed_segs=None → 原部件层。
    // 旋钮 mixed_input 关 → 不扩判（原行为恒等）。
    let parts = if e.boost.as_ref().is_some_and(|b| b.mixed_input) {
        match mixed_segs(e, inp) {
            Some((vbuf, constrain)) => {
                e.jianpin_block_parts_constrained(&vbuf, &constrain)
            }
            None => e.jianpin_block_parts(inp),
        }
    } else {
        e.jianpin_block_parts(inp)
    };
    let Some(parts) = parts else {
        return;
    };
    // ctx 编码（nnlm_rerank 同律: encode 失败 → 逐字词表过滤回退）
    if !e.nnlm_ok() {
        return;
    }
    let ctx_seq: Vec<u32> = match e.nnlm_encode(ctx) {
        Some(s) => s,
        None => ctx
            .chars()
            .filter_map(|c| e.nnlm_vocab_id(&c.to_string()))
            .collect(),
    };
    if ctx_seq.is_empty() {
        return;
    }
    let panel_words: std::collections::HashSet<&str> =
        out.iter().map(|c| c.word.as_str()).collect();
    // 词 id → NNLM token 列（memo）
    let mut tok_of: std::collections::HashMap<u32, Vec<u32>> =
        std::collections::HashMap::new();
    {
        let mut uniq_w: Vec<u32> = Vec::new();
        for (_, _, w, _) in &parts.blocks {
            uniq_w.push(*w);
        }
        uniq_w.sort();
        uniq_w.dedup();
        for w in uniq_w {
            let t = e.word_text(w);
            if let Some(s) = e.nnlm_encode(&t) {
                if !s.is_empty() {
                    tok_of.insert(w, s);
                }
            }
        }
    }
    // 束状态: (词列, Σlp, token 列, 面文本)
    struct Bm {
        words: Vec<u32>,
        lp: f64,
        toks: Vec<u32>,
        text: String,
    }
    const BEAM: usize = 4; // NNLM 截束宽
    const PRE: usize = 48; // lp 预截宽（分桶后总量; 控每层打分序列数）
    const PRE_BUCKET: usize = 12; // 每词数桶 lp 席位
    const SINGLE_N: usize = 10; // 每位单字扩展数
    let mut layers: Vec<Vec<Bm>> =
        (0..=parts.n).map(|_| Vec::new()).collect::<Vec<_>>();
    layers[0].push(Bm {
        words: Vec::new(),
        lp: 0.0,
        toks: Vec::new(),
        text: String::new(),
    });
    let mut fresh: Vec<(String, Vec<String>, f64)> = Vec::new();
    let mut seen_face: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for pos in 0..parts.n {
        // 收层: 去重 → 按词数分桶 lp 预截宽（逐字路 3 词族与词块路 1 词
        // 族各保席位——lp 纯序会被 下面的 类垃圾块占满 24 席, 想买点
        // 永远到不了 NNLM 裁决眼前）→ NNLM(ctx) 批量打分截束
        {
            let mut layer = std::mem::take(&mut layers[pos]);
            let mut dedup: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            layer.retain(|s| dedup.insert(s.text.clone()));
            if layer.len() > PRE {
                layer.sort_by(|a, b| b.lp.partial_cmp(&a.lp).unwrap());
                let mut per_wc: std::collections::HashMap<usize, usize> =
                    std::collections::HashMap::new();
                let mut kept: Vec<Bm> = Vec::with_capacity(PRE);
                for s in layer.into_iter() {
                    if kept.len() >= PRE {
                        break;
                    }
                    let e = per_wc.entry(s.words.len()).or_insert(0);
                    if *e >= PRE_BUCKET {
                        continue;
                    }
                    *e += 1;
                    kept.push(s);
                }
                layer = kept;
            }
            // 束内 NNLM 剪枝 = 轻量版专属: 部分前缀的累计分对比在高级版
            // 上实证跑偏（而为之 类块串中途霸束, 终面尽数垃圾——adv 对
            // 部分前缀与终面的偏好不一致）; 高级版改保宽 lp 分桶束
            // （PRE=48 结构多样）, 语境裁决全部后置到等长终面评分
            // （实测: 想买点爆米花 −28.0 vs 下面的爆米花 −38.7, 决定性）。
            #[cfg(feature = "safetensors")]
            let adv_mode = e.nnlm_has_adv();
            #[cfg(not(feature = "safetensors"))]
            let adv_mode = false;
            if !adv_mode && layer.len() > BEAM {
                let seqs: Vec<Vec<u32>> =
                    layer.iter().map(|s| s.toks.clone()).collect();
                let scores = e.nnlm_score(&ctx_seq, &seqs).unwrap_or_default();
                let mut ranked: Vec<(usize, f64)> = layer
                    .iter()
                    .enumerate()
                    .zip(scores.iter())
                    .map(|((i, _), sc)| (i, *sc))
                    .collect();
                ranked.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1).unwrap()
                });
                if dbg && pos >= 1 && pos <= 4 {
                    for (i, sc) in ranked.iter().take(6) {
                        eprintln!(
                            "[c3bm] pos={} N={:.3} {} ({}词, lp={:.2})",
                            pos,
                            sc,
                            layer[*i].text,
                            layer[*i].words.len(),
                            layer[*i].lp
                        );
                    }
                }
                layer = ranked
                    .iter()
                    .take(BEAM)
                    .map(|&(i, _)| std::mem::replace(
                        &mut layer[i],
                        Bm { words: Vec::new(), lp: 0.0, toks: Vec::new(), text: String::new() },
                    ))
                    .collect();
            }
            layers[pos] = layer;
        }
        if layers[pos].is_empty() {
            continue;
        }
        // 扩展: 单字 top-N ∪ 本位词块（跳迁到 end）
        let mut exts: Vec<(usize, u32, f64)> = Vec::new();
        {
            let mut seen_x: std::collections::HashSet<u32> =
                std::collections::HashSet::new();
            for &(w, lp) in parts.singles[pos].iter() {
                if seen_x.insert(w)
                    && exts.len() < SINGLE_N
                    && tok_of.contains_key(&w)
                {
                    exts.push((pos + 1, w, lp));
                }
            }
            for &(i, end, w, lp) in &parts.blocks {
                if i == pos && tok_of.contains_key(&w) {
                    exts.push((end, w, lp));
                }
            }
        }
        if exts.is_empty() {
            layers[pos] = Vec::new();
            continue;
        }
        let src_states = std::mem::take(&mut layers[pos]);
        for st in src_states.iter() {
            for &(end, w, lp) in &exts {
                let mut words = st.words.clone();
                words.push(w);
                let mut toks = st.toks.clone();
                toks.extend(tok_of[&w].iter().copied());
                let mut text = st.text.clone();
                text.push_str(&e.word_text(w));
                layers[end].push(Bm { words, lp: st.lp + lp, toks, text });
            }
        }
        // 已消费（src_states 就地丢弃）
    }
    // 终层: NNLM 定序取 top-K 面
    {
        let mut layer = std::mem::take(&mut layers[parts.n]);
        let mut dedup: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        layer.retain(|s| {
            !s.text.is_empty() && dedup.insert(s.text.clone())
        });
        if !layer.is_empty() {
            let seqs: Vec<Vec<u32>> =
                layer.iter().map(|s| s.toks.clone()).collect();
            let scores = e.nnlm_score(&ctx_seq, &seqs).unwrap_or_default();
            let mut ranked: Vec<(usize, f64)> = layer
                .iter()
                .enumerate()
                .zip(scores.iter())
                .map(|((i, _), sc)| (i, *sc))
                .collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            for (i, _sc) in ranked {
                let st = &layer[i];
                if st.text.is_empty()
                    || !seen_face.insert(st.text.clone())
                    || panel_words.contains(st.text.as_str())
                {
                    continue;
                }
                let texts: Vec<String> = st
                    .words
                    .iter()
                    .map(|&w| e.word_text(w))
                    .collect();
                fresh.push((st.text.clone(), texts, st.lp));
                if fresh.len() >= 12 {
                    break;
                }
            }
        }
    }
    // 结构保底: 终集 lp 序补面（NNLM 束意外窄时; 亦供逐字频序对照）
    if fresh.len() < 4 {
        for (lp, ws) in parts.finals.iter() {
            let texts: Vec<String> =
                ws.iter().map(|&w| e.word_text(w)).collect();
            let face = texts.concat();
            if face.is_empty()
                || !seen_face.insert(face.clone())
                || panel_words.contains(face.as_str())
            {
                continue;
            }
            fresh.push((face, texts, *lp));
            if fresh.len() >= 4 {
                break;
            }
        }
    }
    if fresh.is_empty() {
        return;
    }
    if dbg {
        for (f, ws, lp) in fresh.iter().take(12) {
            eprintln!("[c3dbg] 束面 {} lp={:.3} 词={}", f, lp, ws.join("|"));
        }
        let probes = ["想买点爆米花","下面的爆米花","新买的白马湖","而我在等你","哦我知道你","而为之都能","小米对吧美好","行吗对吧美好","羡慕对吧美好","想买点爸妈好"];
        let seqs: Vec<Vec<u32>> = probes.iter().map(|t| e.nnlm_encode(t).unwrap_or_default()).collect();
        if let Some(scores) = e.nnlm_score(&ctx_seq, &seqs) {
            for (t, s) in probes.iter().zip(scores.iter()) {
                eprintln!("[c3probe] N={:.3} {t}", s);
            }
        }
    }
    // 打分集 = 面板 sentence 面（对照组——哦我知道你 类逐字面参战）
    //             ⊕ 新块面（cap）; NNLM(ctx) 批量纯序
    let slots: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == Kind::Sentence)
        .map(|(i, _)| i)
        .collect();
    let mut words: Vec<String> = Vec::new();
    let mut src: Vec<Option<usize>> = Vec::new(); // Some=面板槽位 / None=新面
    for &i in &slots {
        words.push(out[i].word.clone());
        src.push(Some(i));
    }
    for (face, _, _) in fresh.iter() {
        words.push(face.clone());
        src.push(None);
    }
    if words.len() < 2 {
        return;
    }
    let seqs: Vec<Vec<u32>> = words
        .iter()
        .map(|w| e.nnlm_encode(w).unwrap_or_default())
        .collect();
    let ok: Vec<usize> = (0..words.len()).filter(|&i| !seqs[i].is_empty()).collect();
    if ok.len() < 2 {
        return;
    }
    let batch: Vec<Vec<u32>> = ok.iter().map(|&i| seqs[i].clone()).collect();
    let scores = e.nnlm_score(&ctx_seq, &batch).unwrap_or_default();
    let mut ranked: Vec<(usize, f64)> = ok
        .iter()
        .copied()
        .zip(scores.into_iter())
        .collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    if dbg {
        for (i, sc) in ranked.iter().take(8) {
            eprintln!(
                "[c3dbg] N={:.3} {} {}",
                sc,
                words[*i].replace('\u{1}', "·"),
                if src[*i].is_some() { "面板" } else { "块面" }
            );
        }
    }
    // 写回（B7 原位律）: 排序重放 = 面板 sentence 候选 or 新面 Cand,
    // 依序落入 sentence 槽（槽位不变）; 分锚定槽内原最大分递减保单调。
    if !slots.is_empty() {
        let mut seq_cands: Vec<Cand> = Vec::with_capacity(ranked.len());
        for &(ix, _) in &ranked {
            match src[ix] {
                Some(i) => seq_cands.push(out[i].clone()),
                None => {
                    let f = &words[ix];
                    if let Some((face, ws, lp)) =
                        fresh.iter().find(|(face, _, _)| face == f)
                    {
                        let mut c =
                            Cand::new(face.clone(), *lp, Kind::Sentence);
                        c.path_words = Some(ws.clone());
                        c.score = *lp;
                        seq_cands.push(c);
                    }
                }
            }
        }
        let anchor = slots
            .iter()
            .map(|&i| out[i].score)
            .fold(f64::NEG_INFINITY, f64::max);
        for (r, &slot) in slots.iter().enumerate() {
            if r < seq_cands.len() {
                let mut c = seq_cands[r].clone();
                c.score = anchor - 0.001 * r as f64;
                out[slot] = c;
            }
        }
        // 槽外追加: 束面多于 sentence 槽时, 余者按 NNLM 序钉分缀于槽后
        // （B2.7 律——只增不改: fbcol 等原生位次不因注入而抬升, 新面
        // 以递减钉分入场; 面板长度 +≤4）
        if seq_cands.len() > slots.len() {
            let last_slot = *slots.last().unwrap();
            let mut ins: Vec<Cand> = Vec::new();
            for (r, c) in seq_cands
                .iter()
                .enumerate()
                .skip(slots.len())
                .take(4)
            {
                let mut c = c.clone();
                c.score = anchor - 0.001 * r as f64;
                ins.push(c);
            }
            if !ins.is_empty() {
                out.splice(
                    last_slot + 1..last_slot + 1,
                    ins,
                );
            }
        }
    } else {
        // 无 sentence 槽: B2.7 钉分律注入顶部（top + 0.5 递减）
        let top = out.first().map(|c| c.score).unwrap_or(0.0);
        let mut ins: Vec<Cand> = Vec::new();
        for (i, &(ix, _)) in ranked.iter().take(3).enumerate() {
            if src[ix].is_some() {
                continue; // slots 空 → 打分集全为新面, 此行防御性保留
            }
            let f = &words[ix];
            let Some((face, ws, lp)) =
                fresh.iter().find(|(face, _, _)| face == f)
            else {
                continue;
            };
            let mut c = Cand::new(face.clone(), *lp, Kind::Sentence);
            c.path_words = Some(ws.clone());
            c.score = top + 0.5 - 0.01 * i as f64;
            ins.push(c);
        }
        if !ins.is_empty() {
            let n = ins.len();
            out.splice(0..0, ins);
            if dbg {
                eprintln!("[c3dbg] 无槽注入 {n} 面（ctx={ctx}）");
            }
        }
    }
}


/// 混合切分走位（mixed_input 的域判与切分核心, typo_retry 不可达分支
/// 复用）: 贪心全拼切分（最长优先）; 死位回退声母段（zh/ch/sh 双键整体）。
/// 全拼段折叠为 1 个**虚拟槽位**（首字母代表）, 声母段天然 1 槽——
/// 现有部件机制跑虚拟串, 约束按虚拟位传「精确音节」（段内字母
/// 不再逐位暴露——设计-混合输入 §2.2 的收窄语义）。
/// 门: 长度 [3,12]; 纯声母（归 c.3）/纯全拼（归原生 DP）→ None。
pub(super) fn mixed_segs(
    e: &crate::Engine,
    inp: &str,
) -> Option<(String, std::collections::HashMap<usize, String>)> {
    let b = inp.as_bytes();
    if b.len() < 3 {
        return None;
    }
    let mut vbuf = String::new();
    let mut constrain: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();
    let mut i = 0usize;
    let mut n_full = 0usize;
    let mut n_ini = 0usize;
    while i < b.len() {
        let mut hit = 0usize;
        for l in (1..=7.min(b.len() - i)).rev() {
            if e.seg.is_syl(&inp[i..i + l]) {
                hit = l;
                break;
            }
        }
        if hit >= 2 {
            let seg = &inp[i..i + hit];
            if std::env::var("CTX_DBG").is_ok() {
                eprintln!("[mixseg] i={i} 全拼段 {seg:?}");
            }
            constrain.insert(vbuf.len(), seg.to_string());
            vbuf.push(seg.as_bytes()[0] as char);
            n_full += 1;
            i += hit;
        } else {
            let two = &inp[i..(i + 2).min(inp.len())];
            if std::env::var("CTX_DBG").is_ok() {
                eprintln!("[mixseg] i={i} 声母段 {:?}", &inp[i..]);
            }
            if two.starts_with("zh") || two.starts_with("ch")
                || two.starts_with("sh")
            {
                // 双键声母: 虚拟位用首字母, 约束空（z/c/s 域语义——与
                // B2.7 简拼表 zh→z 投影同律）
                vbuf.push(two.as_bytes()[0] as char);
                i += 2;
            } else {
                vbuf.push(b[i] as char);
                i += 1;
            }
            n_ini += 1;
        }
    }
    if n_full == 0 || n_ini == 0 {
        return None;
    }
    // 槽数门（非原始字节长——全拼段多字母折叠 1 槽, chongmanjiqdbsh
    // 14 字节 7 槽曾被字节门误杀; 部件层 3..=12 同律）
    if vbuf.len() < 3 || vbuf.len() > 12 {
        return None;
    }
    if std::env::var("CTX_DBG").is_ok() {
        eprintln!(
            "[mixdbg] 门过 虚拟={vbuf:?} 约束={constrain:?} (全拼段 {n_full}/声母段 {n_ini})"
        );
    }
    Some((vbuf, constrain))
}

/// 混合输入（设计-混合输入 §2, 旋钮 mixed_input）: 全拼段∪声母段混合键。
/// 域门: 纯全拼 greedy 不可达（可达则原生 DP 链已覆盖）且贪心全拼切分
/// 后确有全拼段（纯声母串归 c.3 域）且确有声母段。产出 = 约束版部件
/// （jianpin_block_parts_constrained）终集 Σlp 序 top-N, B2.7 钉序注入。
pub(super) fn mixed_input(e: &crate::Engine, inp: &str, out: &mut Vec<Cand>) {
    use crate::engine::Kind;
    // 简拼域 boost 空白收口（backlog 📐「B0 len≥8 不可达」）: 纯声母
    // 长键（≥8 位, 无全拼段）——简拼表键长不及, fuzzcomp 长键垃圾化
    // （cmjqdbsy吧 字面透传脸登顶实测）。c.3 部件层（无约束版）直接
    // 组装 + 频序钉序注入 = 混合域无 ctx 臂的最后一块拼图;
    // 短键（<8）仍归简拼表/G1G2 原链, 有 ctx 归 c.3（hook 已让位）。
    let parts = match mixed_segs(e, inp) {
        Some((vbuf, constrain)) => {
            e.jianpin_block_parts_constrained(&vbuf, &constrain)
        }
        None if inp.len() >= 8 => e.jianpin_block_parts(inp),
        None => return,
    };
    let Some(parts) = parts else {
        if std::env::var("CTX_DBG").is_ok() {
            eprintln!("[mixdbg] 部件 None");
        }
        return;
    };
    if parts.finals.is_empty() {
        return;
    }
    let dbg = std::env::var("CTX_DBG").is_ok();
    if dbg {
        eprintln!("[mixdbg] finals {}", parts.finals.len());
        for (lp, ws) in parts.finals.iter().take(8) {
            let texts: Vec<String> =
                ws.iter().map(|&w| e.word_text(w)).collect();
            eprintln!("[mixdbg] 面 {} lp={:.3} {}", texts.concat(), lp, texts.join("|"));
        }
    }
    // B2.7 钉序注入 cap4（只增不改; 与原生面去重）
    let top = out.first().map(|c| c.score).unwrap_or(0.0);
    let seen: std::collections::HashSet<&str> =
        out.iter().map(|c| c.word.as_str()).collect();
    let mut ins: Vec<Cand> = Vec::new();
    for (i, (lp, ws)) in parts.finals.iter().take(4).enumerate() {
        let texts: Vec<String> =
            ws.iter().map(|&w| e.word_text(w)).collect();
        let face = texts.concat();
        if face.is_empty() || seen.contains(face.as_str()) {
            continue;
        }
        let mut c = Cand::new(face.clone(), *lp, Kind::Sentence);
        c.path_words = Some(texts);
        c.score = top + 0.5 - 0.01 * i as f64;
        ins.push(c);
    }
    if !ins.is_empty() {
        let n = ins.len();
        out.splice(0..0, ins);
        if dbg {
            eprintln!("[mixdbg] 注入 {n} 面");
        }
    }
}
