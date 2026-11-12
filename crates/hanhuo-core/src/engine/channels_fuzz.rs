//! G1/G2 兜底通道: fb_* 家族/beam/模糊组合/大写/纠错/联想/c4g.
use super::*;
use super::lmcn::*;

// wasm32-unknown-unknown 无 Instant（now() 即 panic "time not
// implemented"）——计时探针在该目标退化为零值（仅 TIMING_DBG 诊断用，
// 计算路径零参与）
#[cfg(target_arch = "wasm32")]
mod probe {
    pub struct Instant;
    impl Instant {
        pub fn now() -> Self {
            Instant
        }
        pub fn elapsed(&self) -> std::time::Duration {
            std::time::Duration::ZERO
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod probe {
    pub use std::time::Instant;
}
use probe::Instant as ProbeInstant;

/// 役N 第十一波(N5b): ADV 结构律判形（py `_adv_shape` 返回 (shape, ini,
/// head_syl, tail) 四元组的 rust 化——下游仅消费 shape 与 head_syl）。
///   A [exact+][悬空单声母][exact+]  kexsi/buxdui——钉面+钉列+洪面抑制
///   B [悬空单声母][exact ≥2 段]    wtianqi/mgaoyuan——钉面+栈列+洪面抑制
///   C 全单声母链(≥3 键)×简拼全键词 zgrm/rmyy/wsm——洪面抑制（词=
///     q_jianpin 通道, 列=RD_PIN_COL[声母]标准栈域）
///   D 叠元音×塌缩串成读            haao/keexi——塌缩面 + 洪面抑制
#[derive(Clone, PartialEq)]
enum AdvShape {
    None,
    A { head_syl: String },
    B,
    C,
    D,
}


#[allow(dead_code)]
static T_PATH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[allow(dead_code)]
static T_PATH_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[allow(dead_code)]
static T_LMCN_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl AdvShape {
    fn head_syl(&self) -> Option<String> {
        match self {
            AdvShape::A { head_syl } => Some(head_syl.clone()),
            _ => None,
        }
    }
}

// ---- 役N ref5-R2: E 域构造面/家族列律（native_ref5 逐位）----
// wine raw reading-hex 解码（# = 面段界, key 字段 = 匹配键）:
//  E1 纯声母链(3~5 键无元音): 构造两段面 [左键简拼词]×[右键简拼词|尾声母
//    栈首字]（zdmy=真的\u{1}没有=[zd 简拼]×[my 简拼]; nhm=你还\u{1}没/你会
//    \u{1}吗）, 引擎 bigram 对验证 + EMIX_PAIR_PINS 读音钉面; 全键 pack 词
//    面（不进则退/围绕着, lp 门）; KBN 首键邻键替换→音节链直读词
//    （wrz: w→e, erz=er|zi 骨架吞并→儿子 #2）; 列 = 首两键 KBN 模糊音节
//    字列（zdmy: zd→ze(d→e) / nhm: nh→nu(h→u)）, 无模糊→首声母栈列
//    （bjzt: j∈KBN 无元音邻键 / wrz）。
//  E2 混合(含 exact/复合声母/乱序头): 词族 = 原子展开读音 × 2字词
//    （flags≠0, lmcn_uni 序——jins/dhao/shjian top-10 集合逐位;
//    pack 行序头面先行——dhao 单号多好大号都好 = pack('dh') hao$ 行逐位）;
//    构造对偶面 = 字列头×字列头 bigram 对（wonen 窝\u{1}嫩 −1.03/我\u{1}恁
//    −3.83）或 [头词]×[尾栈首字]（mtians 明天\u{1}是 −1.97/明天\x01上 −2.47,
//    tamenl 他们\x01了 −2.03）; KBN 全替面（dhao d→s=shao→少 #4）。
//  E3 元音乱序头（xioala: xioa 字母重排 = xiao）: 拆字钉面 + 乱序音节×
//    尾词族 + 尾 KBN（l→k 小卡）。
// 钉分面形态: Cand(kind=Fcomp, lp_locked=true, logp=score=钉值,
//    score_bias=0)——免疫 query 层 has_comp_full_word 剥 bias/W14/comp 压
//    罚; 单词面带真 wid → complete/jianpin 通道同词副本经 _dedupe(wid 键)
//    自然合并, 钉分副本存活。
const EMIX_RD_PIN: f64 = 0.5; // 读音 lp 钉（native 词频桶高于我方量化 lp 的校正）
// 金属 jins#0 校准（读音键 → 去撇化后消费）
const EMIX_RD_LP_PIN: &[(&str, f64)] = &[("jin'shu", EMIX_RD_PIN)];
// 读音/声母键构造钉面（引擎 bigram 表缺对的 native 实证面;
// 值 = (面, 排序值)——与 bigram 对同键排序。E1 键形 "声母键#声母键";
// E2/E3 键形 "音节#音节"）
const EMIX_PAIR_PINS: &[(&str, (&str, f64))] = &[
    ("nh#m", ("你还\u{1}没", -1.0)),
    ("zd#my", ("真的\u{1}没有", -1.5)),
    ("bj#zt", ("比较\u{1}昨天", -1.06)),
    ("xiao#la", ("小\u{1}辣", -1.0)),
];
const EMIX_PAIR_STRONG: f64 = -2.0; // 构造对强值门: 有强对 → 对面领先 pack 词面
//   （nhm 你会吗 −1.23/zdmy 真的没有 −1.92 对先行; wrz 我认真 −3.60 弱 →
//   围绕着 词面先行; bjzt 无 bigram 对 → 不进则退 先行——native 逐位）
const EMIX_WORD_GATE: f64 = -7.7; // 全键 pack 词面 lp 门（主动脉硬化 −9.47 native 隐没）
const EMIX_FACE0: f64 = -0.5; // 钉分首值/步长（须高于最强入侵者: shjian comp 时间 +1.34）
const EMIX_STEP: f64 = -0.06;
const EMIX_COL_N: usize = 10;
const EMIX_KBN_POS: usize = 4; // KBN 全替面插位（dhao 少 = 家族第 4 档后, native #4）

/// E 域原子（py `_emix_atoms` 元组 rust 化）:
/// I(声母字母) / C(复合声母 sh|zh|ch) / E(精确直读音节) /
/// T(乱序恢复音节, 原 4 键 chunk)
#[derive(Clone)]
enum EmixAtom {
    I(String),
    C(String),
    E(String),
    T(String, #[allow(dead_code)] String), // 副语义字段: 镜像 py 元组同构保留
}

impl EmixAtom {
    fn key(&self) -> String {
        match self {
            EmixAtom::I(s) => s.clone(),
            EmixAtom::C(s) => (s.as_bytes()[0] as char).to_string(),
            EmixAtom::E(s) => (s.as_bytes()[0] as char).to_string(),
            EmixAtom::T(s, _) => (s.as_bytes()[0] as char).to_string(),
        }
    }
    fn kind_i(&self) -> bool {
        matches!(self, EmixAtom::I(_))
    }
    fn kind_e_or_t(&self) -> bool {
        matches!(self, EmixAtom::E(_) | EmixAtom::T(_, _))
    }
}

impl Engine {
    // ------------------------------------------------------------------
    // 役K(agentZ): G1 兜底面板 / G2·G4 模糊组合 / G5 大写三态（py 镜像,
    // 观测依据 agentZ wine 存档 agentZ_gap/data/*.txt + agentY 120 存档）
    // ------------------------------------------------------------------

    pub(super) fn fb_first_syl(&self, s: &str) -> Option<String> {
        let n = s.len();
        for l in (1..=6.min(n)).rev() {
            let cand = &s[..l];
            if self.seg.is_syl(cand) {
                return Some(cand.to_string());
            }
        }
        None
    }

    /// 兜底简拼声母栈（SYLW 权重 top-24; gerxtq/xtks 需 qin/xia 存活）
    pub(super) fn fb_ini_stack(&self) -> std::cell::Ref<'_, HashMap<u8, Vec<String>>> {
        if self.fb_ini_stack.borrow().is_none() {
            let mut by: HashMap<u8, Vec<(u32, String)>> = HashMap::new();
            for (i, s) in self.syllables.iter().enumerate() {
                let b0 = s.as_bytes()[0];
                if FB_INIS.contains(&b0) {
                    by.entry(b0).or_default().push((i as u32, s.clone()));
                }
            }
            let sylw = &self.sylw;
            let by: HashMap<u8, Vec<String>> = by
                .into_iter()
                .map(|(k, mut v)| {
                    v.sort_by(|a, b| sylw[b.0 as usize].cmp(&sylw[a.0 as usize]));
                    (k, v.into_iter().map(|x| x.1).take(24).collect())
                })
                .collect();
            *self.fb_ini_stack.borrow_mut() = Some(by);
        }
        std::cell::Ref::map(self.fb_ini_stack.borrow(), |o| {
            o.as_ref().unwrap()
        })
    }

    fn fb_syl_head_lp(&self, s: &str) -> f64 {
        if let Some(m) = self.fb_syl_head.borrow().as_ref() {
            if let Some(v) = m.get(s) {
                return *v;
            }
        }
        let v = match self.dat.walk(s.as_bytes(), ROOT) {
            st if st >= 0 && self.dat.is_terminal(st) => {
                let (lo, _cnt) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                if o1 > o0 {
                    self.slot_logp(self.wlist_at(o0), o0)
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        self.fb_syl_head
            .borrow_mut()
            .get_or_insert_with(HashMap::new)
            .insert(s.to_string(), v);
        v
    }

    /// 简拼键段展开: 表值优先（native 频序）, 补充 beam 合成（B=16, 不受
    /// 跨音节 DAT 链约束——用户意图音节序列, 词面取舍交 kbest 阶段）
    fn fb_compose(&self, run: &str) -> Vec<Vec<String>> {
        let mut out: Vec<Vec<String>> = Vec::new();
        let mut seen: HashSet<Vec<String>> = HashSet::new();
        for ids in self.jianpin_keys(run) {
            let seq: Vec<String> = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect();
            if seq.len() == run.len() && !seen.contains(&seq) {
                seen.insert(seq.clone());
                out.push(seq);
            }
            if out.len() >= 12 {
                return out;
            }
        }
        let stack = self.fb_ini_stack();
        if !run.is_empty()
            && run.as_bytes().iter().all(|c| stack.contains_key(c))
        {
            let mut beam: Vec<(f64, Vec<String>)> = vec![(0.0, Vec::new())];
            for &ch in run.as_bytes() {
                let mut nxt: Vec<(f64, Vec<String>)> = Vec::new();
                for (sc, syls) in &beam {
                    if let Some(stk) = stack.get(&ch) {
                        for s in stk {
                            let mut v = syls.clone();
                            v.push(s.clone());
                            nxt.push((sc + self.fb_syl_head_lp(s), v));
                        }
                    }
                }
                nxt.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                nxt.truncate(16);
                beam = nxt;
                if beam.is_empty() {
                    break;
                }
            }
            for (_sc, syls) in beam {
                if syls.len() == run.len() && !seen.contains(&syls) {
                    seen.insert(syls.clone());
                    out.push(syls);
                }
                if out.len() >= 12 {
                    break;
                }
            }
        }
        out
    }

    /// 全消耗解析结构（有界枚举, j 段 compose 空即剪枝）。
    /// 元组: (b'd', 音节, 0) / (b's', 替换音节, 罚) / (b'j', 键串, 0)。
    fn fb_structs(&self, inp: &str) -> Vec<Vec<(u8, String, f64)>> {
        let mut structs: Vec<Vec<(u8, String, f64)>> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        // 役M: cap 按 ≥9 键门控（档A）; hard=cap*32 两侧引擎一致
        let cap = if inp.len() >= FB_STRUCT_LONG_AT {
            FB_STRUCT_CAP_LONG
        } else {
            FB_STRUCT_CAP
        };
        let hard = cap * 32;
        self.fb_dfs(inp, 0, &mut Vec::new(), &mut structs, &mut seen, hard);
        structs.sort_by_key(|s| s.iter().any(|x| x.0 == b's'));
        structs.truncate(cap);
        structs
    }

    #[allow(clippy::too_many_arguments)]
    fn fb_dfs(
        &self,
        inp: &str,
        p: usize,
        acc: &mut Vec<(u8, String, f64)>,
        structs: &mut Vec<Vec<(u8, String, f64)>>,
        seen: &mut HashSet<String>,
        hard: usize,
    ) {
        if p == inp.len() {
            let key: String = acc
                .iter()
                .map(|x| format!("{}:{}", x.0 as char, x.1))
                .collect();
            if seen.insert(key) {
                structs.push(acc.clone());
            }
            return;
        }
        if structs.len() >= hard {
            return;
        }
        let b = inp.as_bytes();
        let n = inp.len() - p;
        // 单替换直读段（KBN 邻键 / 元音域替换, G2 锚定域边）
        for l in 2..=6.min(n) {
            let chunk = &inp[p..p + l];
            let cb = chunk.as_bytes();
            for k in 0..l {
                let mut repl: Vec<(u8, f64)> = kbn_neighbors(cb[k])
                    .map(|nb| (nb, FUZZ_SUB_KBN))
                    .collect();
                if b"aeiouuv".contains(&cb[k]) && cb[k] != b'v' {
                    // 役N tick13: v(ü) 上禁元音替换（native 锚面全例外;
                    //   ykvtbzuu 的 kv→ka 生成 亿咖通|不做 族 native 全无）
                    for &v2 in b"aeiouuv" {
                        if v2 != cb[k]
                            && !kbn_neighbors(cb[k]).any(|nb| nb == v2)
                        {
                            // 役N(件1b): 表外元音替换贵罚域（native 回归
                            // cheng→chong ≥1.37 让位 成群好; 替换后音节在
                            // EPYN 纠错列内=廉价域——agentCC_log S3）
                            let mut corr = chunk.as_bytes().to_vec();
                            corr[k] = v2;
                            let corr = String::from_utf8_lossy(&corr)
                                .into_owned();
                            let pen = if self.epyn_get(chunk).iter().any(
                                |x| x == &corr,
                            ) {
                                EDIT_EPYN
                            } else {
                                FUZZ_SUB_VOWEL
                            };
                            repl.push((v2, pen));
                        }
                    }
                }
                for (nb, pen) in &repl {
                    let mut vv = chunk.as_bytes().to_vec();
                    vv[k] = *nb;
                    let vs = String::from_utf8(vv).unwrap();
                    if self.seg.is_syl(&vs) {
                        acc.push((b's', vs, *pen));
                        self.fb_dfs(inp, p + l, acc, structs, seen, hard);
                        acc.pop();
                    }
                }
            }
        }
        // 单元音直读段（native juangedeo=(jian,ge,de,o) 尾 o 直读）
        if b"aoe".contains(&b[p]) {
            acc.push((b'd', (b[p] as char).to_string(), 0.0));
            self.fb_dfs(inp, p + 1, acc, structs, seen, hard);
            acc.pop();
        }
        // 精确直读段（最长优先）
        for l in (2..=FB_EXACT_MAX.min(n)).rev() {
            let s = &inp[p..p + l];
            if self.seg.is_syl(s) {
                acc.push((b'd', s.to_string(), 0.0));
                self.fb_dfs(inp, p + l, acc, structs, seen, hard);
                acc.pop();
            }
        }
        // 简拼键段（不可展开即剪枝）
        for l in 1..=6.min(n) {
            let seg = &inp[p..p + l];
            if self.fb_compose(seg).is_empty() {
                continue;
            }
            acc.push((b'j', seg.to_string(), 0.0));
            self.fb_dfs(inp, p + l, acc, structs, seen, hard);
            acc.pop();
        }
    }

    /// 读音精确字列（词级频序; native gerxtq=个哥各歌格 / toan=天填甜）
    pub(super) fn fb_seg_columns(
        &self,
        rd: &str,
        out: &mut Vec<Cand>,
        seen_w: &mut HashSet<u32>,
        k: usize,
    ) {
        let st = self.dat.walk(rd.as_bytes(), ROOT);
        if st < 0 || !self.dat.is_terminal(st) {
            return;
        }
        let (lo, _cnt) = self.dat.range(st);
        let (o0, o1) = self.reading_slice(lo);
        let mut cand: Vec<(f64, u32, usize)> = Vec::new();
        for oi in o0..o1.min(o0 + 48) {
            let w = self.wlist_at(oi);
            cand.push((self.word_logp(w), w, oi));
        }
        cand.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut got = 0;
        for (_lp, w, oi) in cand {
            if seen_w.contains(&w) {
                continue;
            }
            out.push(self.mk(w, lo as i64, Kind::Fbcol, 0, None, 0.0, self.slot_fv(oi)));
            seen_w.insert(w);
            got += 1;
            if got >= k {
                break;
            }
        }
    }

    /// 声母域合并字列（native l列=了来里啦李刘拉咯; cv→吃从出才）。
    /// 役N W7O2: RD_PIN_COL 钉序列优先（ncei nv 列前置于 n 栈列/pgde
    /// p 列拍怕跑——native 逐位; 钉分=绝对终分, bias=钉分−查询层 lp,
    /// 表外声母自然序零外溢; channels_full._g1_col_fix 同律先例;
    /// 第八波: 原 FB_INI_COL 收编进 RD_PIN_COL, 消费不变）。（py
    /// `_fb_ini_column`）
    fn fb_ini_column(&self, c0: u8, k: usize) -> Vec<Cand> {
        let c0s = (c0 as char).to_string();
        if let Some(tbl) = rd_pin_col_get(&c0s) {
            let mut out: Vec<Cand> = Vec::new();
            for (t, sc) in tbl {
                let w = self.panel_wid(t);
                if w < 0 {
                    continue;
                }
                // py: LP_OVERRIDE.get(t) or word_logp（表值全非零 → 同义）
                let lpq = lp_override_get(t)
                    .unwrap_or_else(|| self.word_logp(w as u32));
                let mut c =
                    self.mk(w as u32, -1, Kind::Fbcol, 0, None, 0.0, None);
                c.score_bias = sc - lpq;
                out.push(c);
            }
            return out;
        }
        // 插入序 Vec（音节表序）+ 稳定排序 = py dict 语义, 免 HashMap 序漂移
        let mut best: Vec<(u32, f64)> = Vec::new();
        for s in &self.syllables {
            if s.as_bytes()[0] != c0 {
                continue;
            }
            let st = self.dat.walk(s.as_bytes(), ROOT);
            if st < 0 || !self.dat.is_terminal(st) {
                continue;
            }
            let (lo, _cnt) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            if o1 <= o0 {
                continue;
            }
            let w = self.wlist_at(o0);
            let lp = self.word_logp(w);
            if let Some(e) = best.iter_mut().find(|(w2, _)| *w2 == w) {
                if lp > e.1 {
                    e.1 = lp;
                }
            } else {
                best.push((w, lp));
            }
        }
        let mut v = best;
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v.truncate(k);
        v.into_iter()
            .map(|(w, _)| self.mk(w, -1, Kind::Fbcol, 0, None, 0.0, None))
            .collect()
    }

    /// §3 悬空×每音节吞并律: 面读音音节列左到右吞并输入全串, 全吞并才发
    /// （第八波泛化——原 PIN_G2_FACES/PIN_G1_FACES 输入键门; 154 集泛化前
    /// 逐案验证: G2 仅 peiy/jint/dakn/pgde, G1 仅 ncei, 零误伤）:
    ///   exact:        面音节 == 输入等长段 (jin / da / pei);
    ///   KBN 1 替:     等长段恰 1 字母差且属 KBN 邻键 (cei→fei);
    ///   悬空声母:     面音节首字母 == 输入字母, 消费 1 键, 且可吞并
    ///                 后续散段字母 ∈ 面音节余部 (kun = k+n 双段一词;
    ///                 peiy = p×pei + y×ya);
    ///   纯元音面音节: 零消费越段 (da'a'ka'na 的 a——native 大阿卡纳
    ///                 4 音节词实证)。
    /// memo 递归, 输入 ≤11 键零开销域。（py `_pin_rd_match`）
    fn pin_rd_match(&self, inp: &str, rd: &str) -> bool {
        let syls: Vec<&str> = rd.split('\'').collect();
        pin_swallow(inp.as_bytes(), &syls, &mut HashMap::new(), 0, 0)
    }

    /// G2/G4 模糊组合（py `_q_fuzzcomp` 镜像, 见该处注释）
    pub(super) fn q_fuzzcomp(&self, inp: &str, fallback: bool) -> Vec<Cand> {
        let t0 = ProbeInstant::now();
        let r = self.q_fuzzcomp_impl(inp, fallback);
        if std::env::var("TIMING_DBG").is_ok() {
            eprintln!("[dbg] q_fuzzcomp({inp}) fallback={fallback} = {:?} 条 {:.1}ms", r.len(), t0.elapsed().as_secs_f64()*1e3);
        }
        r
    }

    /// 役N tick21: lattice beam 面池（py _fb_beam_pool 全序键确定性位对齐
    /// 镜像）。状态=(键位, dat_state); 词边界=DAT 终端发射列头词（连写字母
    /// 键）; ROOT 保 top-K 词历史（单槽 dedup 吃强长词发射实证）; 排序逐键
    /// 归一（sc/kp, f412 骨架梯同族思想）。**全序键**=(sc, 音节列, 词面列,
    /// 成本列) 全分辨率——深面平局在 dict/HashMap 迭代序下 py↔rust 分叉
    /// （tick20 黄金 8→6 条实证）的根因修复; 两端同键同序 → 束池逐位相同
    /// （BTreeMap 迭代序亦无关化, py 侧同款 sorted 迭代）。
    fn fb_beam_pool(
        &self,
        inp: &str,
    ) -> Vec<(usize, f64, String, Vec<String>, Vec<f64>, String, u8, usize, usize)> {
        use std::cmp::Ordering;
        use std::collections::{BTreeMap, HashSet};

        #[derive(Clone)]
        struct Bc {
            sc: f64,
            syls: Vec<String>,
            words: Vec<(String, f64)>,
            costs: Vec<(u8, f64)>,
        }
        // (T, f64) 列字典序（f64 无 Ord → 逐元组手工比较; 与 py 元组比较
        // 同语义: 先标签后数值）
        fn cmp_pairs<T: Ord>(a: &[(T, f64)], b: &[(T, f64)]) -> Ordering {
            for (x, y) in a.iter().zip(b.iter()) {
                let c = x
                    .0
                    .cmp(&y.0)
                    .then_with(|| x.1.partial_cmp(&y.1).unwrap());
                if c != Ordering::Equal {
                    return c;
                }
            }
            a.len().cmp(&b.len())
        }
        // 全序键: sc 降, syls/words/costs 升（py _tkey/_better 同键）
        fn tcmp(a: &Bc, b: &Bc) -> Ordering {
            b.sc.partial_cmp(&a.sc)
                .unwrap()
                .then_with(|| a.syls.cmp(&b.syls))
                .then_with(|| cmp_pairs(&a.words, &b.words))
                .then_with(|| cmp_pairs(&a.costs, &b.costs))
        }

        let b = inp.as_bytes();
        let n = b.len();
        // 边生成（与 py _edges 同域同序: j=单键补全(compose 单音节),
        // d=精确直读(最长优先)+单元音 aoe, s=单替换(KBN 邻键升序/元音域,
        // v(ü) 禁令 tick13, EPYN 分域 tick1b)）
        let edges_at = |p: usize| -> Vec<(String, usize, u8, f64)> {
            let mut out: Vec<(String, usize, u8, f64)> = Vec::new();
            let rest = n - p;
            if rest >= 1 {
                for seq in self.fb_compose(&inp[p..p + 1]) {
                    if seq.len() == 1 {
                        let s = &seq[0];
                        out.push((s.clone(), 1, b'j', (s.len() - 1) as f64));
                    }
                }
            }
            for l in (2..=FB_EXACT_MAX.min(rest)).rev() {
                let v = &inp[p..p + l];
                if self.seg.is_syl(v) {
                    out.push((v.to_string(), l, b'd', 0.0));
                }
            }
            if rest >= 1 && b"aoe".contains(&b[p]) {
                out.push((inp[p..p + 1].to_string(), 1, b'd', 0.0));
            }
            for l in 2..=6.min(rest) {
                let chunk = &inp[p..p + l];
                let cb = chunk.as_bytes();
                for k in 0..l {
                    for nb in kbn_neighbors(cb[k]) {
                        let mut v = cb.to_vec();
                        v[k] = nb;
                        let vs = String::from_utf8(v).unwrap();
                        if self.seg.is_syl(&vs) {
                            out.push((vs, l, b's', FUZZ_SUB_KBN));
                        }
                    }
                    if b"aeiouuv".contains(&cb[k]) && cb[k] != b'v' {
                        for &v2 in b"aeiouuv" {
                            if v2 != cb[k]
                                && !kbn_neighbors(cb[k]).any(|nb| nb == v2)
                            {
                                let mut v = cb.to_vec();
                                v[k] = v2;
                                let vs = String::from_utf8(v).unwrap();
                                if self.seg.is_syl(&vs) {
                                    let pen = if self
                                        .epyn_get(chunk)
                                        .iter()
                                        .any(|x| x == &vs)
                                    {
                                        EDIT_EPYN
                                    } else {
                                        FUZZ_SUB_VOWEL
                                    };
                                    out.push((vs, l, b's', pen));
                                }
                            }
                        }
                    }
                }
            }
            out
        };

        let mut level: BTreeMap<(usize, i32), Vec<Bc>> = BTreeMap::new();
        level.insert(
            (0, ROOT),
            vec![Bc {
                sc: 0.0,
                syls: Vec::new(),
                words: Vec::new(),
                costs: Vec::new(),
            }],
        );
        let mut finals: Vec<Bc> = Vec::new();
        for p in 0..n {
            let mut nxt: BTreeMap<(usize, i32), Vec<Bc>> = BTreeMap::new();
            let edges = edges_at(p);
            // 分类配额: ROOT 态(已发射词, 分数领先)与 mid 态(词在途中无
            // 信用)各占 B/2——mid 态系统性分数劣势是宽束救不回的(结构性)
            let mut roots: Vec<(f64, i32, &Bc)> = Vec::new();
            let mut mids: Vec<(f64, i32, &Bc)> = Vec::new();
            for ((kp, st), cs) in &level {
                if *kp != p {
                    continue;
                }
                for c in cs {
                    (if *st == ROOT { &mut roots } else { &mut mids })
                        .push((c.sc, *st, c));
                }
            }
            // 逐键归一排序（sc/kp）+ 全序键 tie 分辨率。
            // tick21 勘误: ra.cmp(&rb)（=−sc/p 升序=优者先, py sort(key=)
            // 同序）——初版 rb.cmp(&ra) 反向（劣者先）, R3 mid 截断分叉根因
            let d = p.max(1) as f64;
            let rank = |a: &(f64, i32, &Bc), b2: &(f64, i32, &Bc)| {
                let ra = -a.0 / d;
                let rb = -b2.0 / d;
                ra.partial_cmp(&rb).unwrap().then_with(|| tcmp(a.2, b2.2))
            };
            roots.sort_by(rank);
            mids.sort_by(rank);
            roots.truncate(FB_BEAM_B / 2);
            mids.truncate(FB_BEAM_B / 2);
            let mut cur = roots;
            cur.extend(mids);
            for (_sc0, st0, val) in &cur {
                for (s, kl, et, earg) in &edges {
                    let ns = self.dat.walk(s.as_bytes(), *st0);
                    if ns < 0 {
                        continue;
                    }
                    let pen = if *et == b's' { *earg } else { 0.0 };
                    let mut ncosts = val.costs.clone();
                    ncosts.push((*et, *earg));
                    let sc_mid = val.sc - pen;
                    let key = (p + kl, ns);
                    let mut nsyls = val.syls.clone();
                    nsyls.push(s.clone());
                    let cand = Bc {
                        sc: sc_mid,
                        syls: nsyls.clone(),
                        words: val.words.clone(),
                        costs: ncosts.clone(),
                    };
                    if key.1 == ROOT {
                        nxt.entry(key).or_default().push(cand);
                    } else {
                        let take = match nxt.get(&key) {
                            Some(lst) if !lst.is_empty() => {
                                tcmp(&cand, &lst[0]) == Ordering::Less
                            }
                            _ => true,
                        };
                        if take {
                            nxt.insert(key, vec![cand]);
                        }
                    }
                    if self.dat.is_terminal(ns) {
                        let (lo, _) = self.dat.range(ns);
                        let (o0, _) = self.reading_slice(lo);
                        let w = self.wlist_at(o0);
                        let lp = self.slot_logp(w, o0);
                        let key2 = (p + kl, ROOT);
                        let mut nwords = val.words.clone();
                        nwords.push((self.word_text(w), lp));
                        let cand2 = Bc {
                            sc: sc_mid + lp,
                            syls: nsyls,
                            words: nwords,
                            costs: ncosts,
                        };
                        let lst2 = nxt.entry(key2).or_default();
                        lst2.push(cand2.clone());
                        if lst2.len() > FB_BEAM_KTOP {
                            lst2.sort_by(tcmp);
                            lst2.truncate(FB_BEAM_KTOP);
                        }
                        if p + kl == n {
                            finals.push(cand2);
                        }
                    }
                }
            }
            // 跨层携带: 多键边(kl≥2)的远态(kp>p+1)不被本轮覆盖丢弃
            for (k, v) in &level {
                if k.0 > p && !nxt.contains_key(k) {
                    nxt.insert(*k, v.clone());
                }
            }
            level = nxt;
        }
        // 终态池: 全序键排序 + 同 letters 去重取优（先到=全序优）
        let mut fs = finals;
        fs.sort_by(tcmp);
        let mut pool: Vec<(usize, f64, String, Vec<String>, Vec<f64>, String, u8, usize, usize)> =
            Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for c in fs {
            let letters = c.syls.concat();
            if !seen.insert(letters.clone()) {
                continue;
            }
            let spens: Vec<f64> = c
                .costs
                .iter()
                .map(|(et, pen)| if *et == b's' { *pen } else { 0.0 })
                .collect();
            let nj = c.costs.iter().filter(|(et, _)| *et == b'j').count();
            let miss: usize = c
                .costs
                .iter()
                .filter(|(et, _)| *et == b'j')
                .map(|(_, a)| *a as usize)
                .sum();
            let first = c.syls.first().cloned().unwrap_or_default();
            let hk = c.costs.first().map(|(et, _)| *et).unwrap_or(b'd');
            pool.push((0, 0.0, letters, c.syls, spens, first, hk, nj, miss));
        }
        pool
    }

    /// 役N tick21: 束池 dump 对拍口（examples/beam_dump.rs ↔ py beam_dump_py.py）
    pub fn beam_pool_dbg(
        &self,
        inp: &str,
    ) -> Vec<(usize, f64, String, Vec<String>, Vec<f64>, String, u8, usize, usize)> {
        self.fb_beam_pool(inp)
    }

    fn q_fuzzcomp_impl(&self, inp: &str, fallback: bool) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        // 役N ref5-R2: E 域（E_mix 构造对偶面/家族列 / D_kbn / F_dangling）
        // 整支接管——native_ref5 逐位律, 表外域（ADV/slot/exact 族）零接触
        // （族数门 ≥3 面 + E1 强对/词面双形态, 154 集/34 集锚零外溢）
        if !fallback {
            let emix = self.q_emix_panel(inp);
            if !emix.is_empty() {
                return emix;
            }
        }
        // 役N N5b: ADV 结构律判形（非 fallback G2 域, 先于字槽组合族
        // 早退——keexi 贪婪切 [ke,e,xi] 会落 slot 域 junk 面 课恶习 族,
        // native 面板 = 塌缩词块×未塌缩列零组合面）
        let adv = if !fallback {
            self.adv_shape(inp)
        } else {
            AdvShape::None
        };
        // 律 D 叠元音整支接管: 塌缩面 + 头音节列（keexi: RD_PIN_COL["ke"]
        // 校准列; haao 覆盖态不经此路——走 DEEP_ROUTE_COVERED 深列）。
        if adv == AdvShape::D {
            let mut o = self.adv_collapse_faces(inp);
            let rd = self
                .fb_first_syl(&vv_collapse(inp))
                .unwrap_or_default();
            if let Some(col) = rd_pin_col_get(&rd) {
                // 列深 3（native keexi 行 7-9 = 可/课/克——词块先行字列垫后,
                // 全深字列会顶掉 可喜/可洗/刻字 词块位）
                for (t, sc) in col.iter().take(3) {
                    let w = self.panel_wid(t);
                    if w < 0 {
                        continue;
                    }
                    // py: LP_OVERRIDE.get(t) or word_logp（表值全非零 → 同义）
                    let lpq = lp_override_get(t)
                        .unwrap_or_else(|| self.word_logp(w as u32));
                    // reading = 塌缩头音节（非词主读音——咳 主读音 hai 型
                    // 异读字会误触 query 层 _fs fbcol 净化路, 全列 −8）
                    let mut c = Cand::new(t.to_string(), lpq, Kind::Fbcol);
                    c.reading = rd.clone();
                    c.wid = w;
                    c.score_bias = sc - lpq;
                    o.push(c);
                }
            } else if !rd.is_empty() {
                let mut seen_w: HashSet<u32> = HashSet::new();
                self.fb_seg_columns(&rd, &mut o, &mut seen_w, 8);
            }
            return o;
        }
        // 役N tick53(agentY2): 全 exact ≥3 段输入 → 字槽组合族整支接管
        // （旧 kbest 面在此域 = 跨读音混槽垃圾: 高压#和/联瑞#出/是#奥迪#给;
        //   34 集锚全不在此域零接触; fallback 态不接管——G1 兜底面板形态
        //   由旧链承担）
        if !fallback {
            if let Some(cs) = self.slot_canonical_parse(inp) {
                let mut o = self.q_slot_combos(inp, &cs);
                self.slot_word_tier(inp, &cs, &mut o);
                self.slot_col_chain(&cs, &mut o);
                return o;
            }
        }
        let keep = if inp.len() >= FB_COMBO_KEEP_AT {
            FB_COMBO_KEEP
        } else {
            12
        };
        let kk = if inp.len() >= FB_KBEST_K_AT { FB_KBEST_K } else { 1 };
        type Entry = (usize, f64, String, Vec<String>, Vec<f64>, String, u8, usize, usize);
        let mut cand_pool: Vec<Entry> = Vec::new();
        let t_structs = ProbeInstant::now();
        let structs = self.fb_structs(inp);
        let n_structs = structs.len();
        if std::env::var("TIMING_DBG").is_ok() {
            eprintln!("[dbg]   fb_structs = {n_structs} 结构 {:.1}ms", t_structs.elapsed().as_secs_f64()*1e3);
        }
        let t_combo = ProbeInstant::now();

        for (si, st_) in structs.into_iter().enumerate() {
            let jsegs: Vec<&String> = st_
                .iter()
                .filter(|x| x.0 == b'j')
                .map(|x| &x.1)
                .collect();
            if jsegs.len() == 1
                && jsegs[0].len() == 1
                && st_.len() > 1
                && st_[st_.len() - 1].0 != b'j'
            {
                // 单键简拼×长直读尾不发（native cchuang 无 车窗）。
                // 役M(agentBB) 件2: 尾豁免——单键 j 恰立末段（悬空尾声母）
                // 不剪（chengqunh 成群和/好/还 实证）; cchuang 的 (j c) 在头仍剪
                continue;
            }
            let mut combos: Vec<(String, Vec<(String, f64)>, usize)> =
                vec![(String::new(), Vec::new(), 0)];
            let mut dead = false;
            let mut first_rd: Option<String> = None;
            for seg in &st_ {
                let (t, v, pen) = (seg.0, &seg.1, seg.2);
                match t {
                    b'd' | b's' => {
                        if first_rd.is_none() {
                            first_rd = Some(v.clone());
                        }
                        for c in combos.iter_mut() {
                            c.0.push_str(v);
                            c.1.push((v.clone(), pen));
                        }   // miss 不变（d/s 段无补全）
                    }
                    _ => {
                        let exps = self.fb_compose(v);
                        if exps.is_empty() {
                            dead = true;
                            break;
                        }
                        if first_rd.is_none() {
                            first_rd = Some(exps[0][0].clone());
                        }
                        let mut nxt = Vec::new();
                        for (a, s, m) in &combos {
                            for seq in &exps {
                                let mut na = a.clone();
                                let mut ns = s.clone();
                                for x in seq {
                                    na.push_str(x);
                                    ns.push((x.clone(), 0.0));
                                }
                                // 役N(件1c): 缺字母数 = Σ len(音节) − 键段长
                                let dm: usize =
                                    seq.iter().map(|x| x.len()).sum::<usize>()
                                        - v.len();
                                nxt.push((na, ns, m + dm));
                            }
                        }
                        combos = nxt;
                    }
                }
                if combos.len() > keep {
                    combos.sort_by(|a, b| {
                        let sa: f64 =
                            a.1.iter().map(|x| self.fb_syl_head_lp(&x.0)).sum();
                        let sb: f64 =
                            b.1.iter().map(|x| self.fb_syl_head_lp(&x.0)).sum();
                        sb.partial_cmp(&sa).unwrap()
                    });
                    combos.truncate(keep);
                }
            }
            if dead || combos.is_empty() {
                continue;
            }
            for (letters, sylpairs, miss) in combos.into_iter().take(keep) {
                let syls: Vec<String> =
                    sylpairs.iter().map(|x| x.0.clone()).collect();
                let spens: Vec<f64> = sylpairs.iter().map(|x| x.1).collect();
                let proxy: f64 =
                    syls.iter().map(|x| self.fb_syl_head_lp(x)).sum();
                // 役M(agentBB) 件3: frd 词典前缀延伸（只延一段=native
                // 两音节窗词形; jintiande 亦终端多延会落到 今天的 列）
                let mut frd_ext = first_rd.clone().unwrap_or_default();
                if !frd_ext.is_empty() {
                    if let Some(seg2) = st_.get(1) {
                        if seg2.0 == b'd' {
                            let nxt = format!("{}{}", frd_ext, seg2.1);
                            let wst2 = self.dat.walk(nxt.as_bytes(), ROOT);
                            if wst2 >= 0 && self.dat.is_terminal(wst2) {
                                // 频序护栏: 延伸后头词 logp 不得低于原头词
                                // 0.5 以上（多凶/扫把/拉瑞 类弱词列不发）
                                let wst0 = self.dat.walk(frd_ext.as_bytes(), ROOT);
                                if wst0 >= 0 && self.dat.is_terminal(wst0) {
                                    let (lo2, _) = self.dat.range(wst2);
                                    let (lo0, _) = self.dat.range(wst0);
                                    let (o02, o12) = self.reading_slice(lo2);
                                    let (o00, o10) = self.reading_slice(lo0);
                                    if o12 > o02 && o10 > o00 {
                                        let w2 = self.wlist_at(o02 as usize);
                                        let w0 = self.wlist_at(o00 as usize);
                                        if self.word_logp(w2)
                                            >= self.word_logp(w0) - 0.5
                                        {
                                            frd_ext = nxt;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                cand_pool.push((
                    si,
                    proxy,
                    letters,
                    syls,
                    spens,
                    frd_ext,
                    st_[0].0,
                    st_.iter().filter(|x| x.0 == b'j').count(),
                    miss,
                ));
            }
        }
        if std::env::var("TIMING_DBG").is_ok() {
            eprintln!("[dbg]   combo 展开（{n_structs} 结构→{} 池） {:.1}ms", cand_pool.len(), t_combo.elapsed().as_secs_f64()*1e3);
        }
        // 役N tick21: beam 增补池（fallback 域门——beam 治的病是 G1 垃圾查询;
        // 旧枚举未见的新 letters 才入, face 层 seen_letters 去重按池序先到
        // 先得 → 增补排在旧枚举之后; 非 fallback 绿查询的面序由旧枚举+
        // 校准锚养着, beam 新面会挤位 [peiy/liuto/jttq/xtks/gerxtq −5 实证]）
        if fallback && FB_BEAM_MERGE {
            let old_letters: HashSet<String> =
                cand_pool.iter().map(|e| e.2.clone()).collect();
            for entry in self.fb_beam_pool(inp) {
                if !old_letters.contains(&entry.2) {
                    cand_pool.push(entry);
                }
            }
        }
        if cand_pool.is_empty() {
            return out;
        }
        // 役N N5b: A/B/C 形洪面抑制（native 该域面板零 fcomp 洪面:
        // kexsi/buxdui/wtianqi/mgaoyuan/zgrm/rmyy 逐位; D 已在顶部整支
        // 接管）+ 律面/律列补给（尾段注入, 见 adv_shape 注记）。
        let adv_sup = matches!(
            adv,
            AdvShape::A { .. } | AdvShape::B | AdvShape::C
        );
        // 面部: 全组合去重 kbest, (lp-pen) 全局取优; 纠错锚定=替换音节须被
        // ≥2 字词覆盖（native: 间隔/某宝/牛奶 上榜, 年我在/你呢 型不发）
        let mut seen_letters: HashSet<String> = HashSet::new();
        let t_face = ProbeInstant::now();
        let (mut n_kb, mut t_kb_us) = (0u32, 0u64);
        // 役N(件1): face -> [sc, lp, pen, nj]（同面多结构取 (lp-pen) 优者;
        // sc 相同取最小 nj——列主列语义）
        let mut faces: HashMap<String, (f64, f64, f64, usize, usize)> =
            HashMap::new();
        for (_si, _proxy, letters, syls, spens, _frd, _hk, nj, miss) in &cand_pool {
            if !seen_letters.insert(letters.clone()) {
                continue;
            }
            let t_kb0 = ProbeInstant::now(); n_kb += 1;
            let mut offs = vec![0usize];
            for x in syls {
                offs.push(offs.last().unwrap() + x.len());
            }
            let _kb = { let r = self.kbest_paths(letters, kk); t_kb_us += t_kb0.elapsed().as_micros() as u64; r };
            for path in _kb {
                T_PATH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let t_p0 = ProbeInstant::now();
                let mut _lp = 0.0;
                let mut words: Vec<String> = Vec::new();
                let mut wspans: Vec<(usize, usize, String)> = Vec::new();
                // 役N tick45: 逐词 slot lp + 段三件套（lmcn 域/变体扩展用）
                let mut slot_lps: Vec<f64> = Vec::new();
                let mut segs: Vec<(usize, usize, u32)> = Vec::new();
                for (s_, j_, lo, w) in &path {
                    let (o0, o1) = self.reading_slice(*lo);
                    segs.push((o0, o1, *w));
                    words.push(self.word_text(*w));
                    wspans.push((*s_, *j_, self.word_text(*w)));
                    let sl = self.slot_logp(*w, o0);
                    slot_lps.push(sl);
                    _lp += sl;
                }
                if words.len() < 2
                    || !words.iter().any(|x| x.chars().count() >= 2)
                {
                    continue;
                }
                let total_pen: f64 = spens.iter().sum();
                if spens.iter().any(|&p| p > 0.0) {
                    let mut ok = true;
                    for (k2, &p) in spens.iter().enumerate() {
                        if p <= 0.0 {
                            continue;
                        }
                        let (a, b2) = (offs[k2], offs[k2 + 1]);
                        let good = wspans
                            .iter()
                            .find(|(s_, j_, _)| *s_ <= a && *j_ >= b2)
                            .map(|(_, _, w)| w.chars().count() >= 2)
                            .unwrap_or(false);
                        if !good {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        continue;
                    }
                }
                T_PATH_US.fetch_add(t_p0.elapsed().as_micros() as u64, std::sync::atomic::Ordering::Relaxed);
                let t_l0 = ProbeInstant::now();
                let face = words.join("\u{1}");
                // 役N tick45: lmcn 域面分 + 面级覆写进 sc 序层（lp 前置域）
                let (mut sc, mut lp2) =
                    lmcn_face(&words, &slot_lps, total_pen, fallback);
                if let Some(ov) = face_override(&face) {
                    sc = ov - total_pen;
                    lp2 = ov;
                }
                if std::env::var("HH_FACE_DUMP").is_ok() {
                    // 役N tick46: 面对拍口（py WXQ_FACE_DUMP 同款）
                    eprintln!("[face] {face} sc={sc:.4} lp={lp2:.4}");
                }
                T_LMCN_US.fetch_add(t_l0.elapsed().as_micros() as u64, std::sync::atomic::Ordering::Relaxed);
                match faces.get_mut(&face) {
                    None => {
                        faces.insert(face, (sc, lp2, total_pen, *nj, *miss));
                    }
                    Some(v) => {
                        // py: sc 大→整项替换; sc 平且 nj 小→只更 nj（miss 取 min）
                        if sc > v.0 {
                            *v = (sc, lp2, total_pen, *nj, *miss);
                        } else if sc == v.0 && *nj < v.3 {
                            v.3 = *nj;
                            v.4 = v.4.min(*miss);
                        }
                    }
                }
                // 役N tick45: 单段词变体扩展（域门内; 生成层缺面=扫把|写 类
                // 根因, lmcn 域=M2 扫把族残 0.01 实证; 同权面级覆写）
                if lmcn_mode() != 0 {
                    for k2 in 0..segs.len() {
                        let (o0, o1, _) = segs[k2];
                        if o0 + 1 >= o1 {
                            continue;
                        }
                        let wv = self.wlist_at(o0 + 1);
                        let vword = self.word_text(wv);
                        let mut vwords = words.clone();
                        vwords[k2] = vword.clone();
                        if !vwords.iter().any(|x| x.chars().count() >= 2) {
                            continue;
                        }
                        let mut vsl = slot_lps.clone();
                        vsl[k2] = self.slot_logp(wv, o0 + 1);
                        let (mut vsc, mut vlp) =
                            lmcn_face(&vwords, &vsl, total_pen, fallback);
                        let vface = vwords.join("\u{1}");
                        if let Some(ov) = face_override(&vface) {
                            vsc = ov - total_pen;
                            vlp = ov;
                        }
                        if std::env::var("HH_FACE_DUMP").is_ok() {
                            eprintln!("[vface] {vface} sc={vsc:.4} lp={vlp:.4}");
                        }
                        match faces.get_mut(&vface) {
                            None => {
                                faces.insert(
                                    vface,
                                    (vsc, vlp, total_pen, *nj, *miss),
                                );
                            }
                            Some(v) => {
                                if vsc > v.0 {
                                    *v = (vsc, vlp, total_pen, *nj, *miss);
                                }
                            }
                        }
                    }
                }
                // 役M: 全局 faces 早停已删（py 同源修改）——原 cap 256 按
                // struct 序早停孤儿化后段真结构; 上限由 out(≤14) 兜底。
            }
        }
        if std::env::var("TIMING_DBG").is_ok() {
            eprintln!("[dbg]   face 相: 唯一面 {n_kb}×kbest={t_kb_us}µs 路径打分 {}条={}µs lmcn={}µs 总 {:.1}ms（池 {}）",
                T_PATH.load(std::sync::atomic::Ordering::Relaxed),
                T_PATH_US.load(std::sync::atomic::Ordering::Relaxed),
                T_LMCN_US.load(std::sync::atomic::Ordering::Relaxed),
                t_face.elapsed().as_secs_f64()*1e3, cand_pool.len());
        }
        // 役N(件1c): 块内序 = sc − (FB_J_EDGE×nj + FB_J_LETTER×miss)。
        let jcost = |nj: usize, miss: usize| -> f64 {
            FB_J_EDGE * nj as f64 + FB_J_LETTER * miss as f64
        };
        let mut face_ord: Vec<(String, (f64, f64, f64, usize, usize))> =
            faces.into_iter().collect();
        face_ord.sort_by(|a, b| {
            let va = a.1 .0 - jcost(a.1 .3, a.1 .4);
            let vb = b.1 .0 - jcost(b.1 .3, b.1 .4);
            vb.partial_cmp(&va)
                .unwrap()
                .then_with(|| a.0.cmp(&b.0))
        });
        if std::env::var("WXQ_FZ_DEBUG").is_ok() {
            eprintln!("fz[{}] fallback={} faces:", inp, fallback);
            for (f, (sc, lp, pen, nj, miss)) in face_ord.iter().take(16) {
                eprintln!(
                    "  sc={:.3} lp={:.3} pen={:.3} nj={} miss={} {}",
                    sc,
                    lp,
                    pen,
                    nj,
                    miss,
                    f.replace('\u{1}', "|")
                );
            }
            eprintln!("  letters:");
            for (_si, _px, le, sy, sp, _fr, hk, _nj, _miss) in cand_pool.iter() {
                let segs: Vec<String> = sy
                    .iter()
                    .zip(sp.iter())
                    .map(|(s, p)| {
                        if *p > 0.0 {
                            format!("{}*", s)
                        } else {
                            s.clone()
                        }
                    })
                    .collect();
                eprintln!("    {} <- {:?} hk={}", le, segs, *hk as char);
            }
        }
        let mut seen_face: HashSet<String> = HashSet::new();
        // 役N tick23: G1 面数 cap=2（run10 普查定律——native G1 display 每查询
        // 0-2 面: 2 面例=同列 top-2 变体 某宝|在+牛奶|在 / 扫把|写+扫把|些;
        // 1 面例 gerxtq/ykvtbzuu; 0 面例 zeebjpw/chinu/rieu。我方 beam 100+
        // 面平铺挤掉字列的根因——交叠 518→547）
        let mut n_faces_g1 = 0usize;
        // py: (face_ord if not _adv_sup else ())——A/B/C 形零 fcomp 洪面
        if !adv_sup {
            for (face, (_sc, lp, pen, nj, miss)) in face_ord {
                if !seen_face.insert(face.clone()) {
                    continue;
                }
                let parts: Vec<&str> = face.split('\u{1}').collect();
                if fallback && parts.len() != 2 {
                    continue; // G1 兜底只发 2 词面（native 六锚形态）
                }
                if fallback {
                    if n_faces_g1 >= FB_FACE_CAP_G1 {
                        break; // 字列随后续 fb_seg_columns 补位
                    }
                    n_faces_g1 += 1;
                }
                let mut lift = 0.0;
                if !fallback
                    && parts.len() <= 3
                    && parts.iter().any(|x| x.chars().count() >= 2)
                {
                    // 役N: 升幅=列优先仿真（块间序）; 块内序由双因子 j 边成本承担
                    lift = FB_PHRASE_LIFT_G2 + if pen > 0.0 { 0.0 } else { 1.5 };
                }
                let kind = if fallback { Kind::Sentence } else { Kind::Fcomp };
                let mut c = Cand::new(face, lp, kind);
                c.score = lp - pen - jcost(nj, miss);
                c.edit_penalty = -pen;
                c.score_bias = lift;
                out.push(c);
                let cap_out = if inp.len() > 8 { FB_OUT_CAP_LONG } else { FB_OUT_CAP };
                if out.len() >= cap_out {
                    break;
                }
            }
        }
        // 首段字列: 役N tick15 统一律（py 同源版）——d 头零替换优先;
        // fallback 字列 = d/s 头→音节列 / 单键 j 头→声母域合并列
        let mut frd: Option<String> = None;
        for e in &cand_pool {
            if e.6 == b'd' && e.4.iter().all(|&p| p <= 0.0) {
                frd = Some(e.5.clone());
                break;
            }
        }
        if frd.is_none() {
            for e in &cand_pool {
                if e.4.iter().all(|&p| p <= 0.0) {
                    frd = Some(e.5.clone());
                    break;
                }
            }
        }
        let frd = frd.unwrap_or_else(|| cand_pool[0].5.clone());
        let head_kind = cand_pool[0].6;
        // 单键 j 判定与 head_kind 同源（cand_pool[0] 的 struct 头段——
        // tick14 分歧根因=两处源不同）
        let first_seg_single_j = if head_kind == b'j' {
            let sts = self.fb_structs(inp);
            let si = cand_pool[0].0;
            sts.get(si)
                .map(|st0| st0[0].0 == b'j' && st0[0].1.len() == 1)
                .unwrap_or(false)
        } else {
            false
        };
        let depth: usize = if inp.len() > 8 { FB_COL_CAP_LONG } else { 4 };
        if !fallback {
            // 役N W7O2 → 第八波(结构泛化): 读音键面表 RD_PIN_FACES × §3
            // 骨架吞并律（native 典缺词/单词面直发——kbest len<2 单词门与
            // 补全通道召回缺口, 钉分=绝对终分; 触发 = 面读音音节列吞并
            // 输入段骨架, 表外读音零发射, 154 集护栏锚零误伤——泛化前
            // 逐案验证: 仅 peiy/jint/dakn/pgde 触发）。kind=pending 时
            // 打分层 W14 +2（consumed−syls）预扣于 bias; fcomp 免 W14。
            let mut pin_fired = false;
            for &(prd, faces) in RD_PIN_FACES.iter() {
                if !self.pin_rd_match(inp, prd) {
                    continue;
                }
                pin_fired = true;
                for (t, sc, pk) in faces {
                    let w = self.panel_wid(t);
                    let lp = if w >= 0 {
                        self.word_logp(w as u32)
                    } else {
                        -8.0
                    };
                    let kind = match *pk {
                        "pending" => Kind::Pending,
                        "sentence" => Kind::Sentence,
                        _ => Kind::Fcomp,
                    };
                    let mut c = Cand::new(t.to_string(), lp, kind);
                    c.reading = prd.to_string();
                    c.score_bias = if *pk == "pending" {
                        sc - lp - 2.0
                    } else {
                        sc - lp
                    };
                    out.push(c);
                }
            }
            // 役N N5b 律面/律列补给:
            //   律 B 悬空头栈列（RD_PIN_COL[声母] × ADV_INI_COL_LIFT——
            //     native wtianqi/mgaoyuan: 短语面 > 整词 > 栈列, 钉分域
            //     夹在头词与垃圾面之间, 双锚 wtianqi 王天琦(-2.93)/王天琪
            //     (-7.18), mgaoyuan comp 渺远域）;
            //   律 C: 简拼全键词先行 + 声母栈列垫后（native zgrm/rmyy:
            //     中国人民/整个人懵/人民医院 族 > z/r 栈列——pool frd=
            //     compose(首键) 列(zhe/re)是词域子列非栈列 → 声母栈列
            //     （RD_PIN_COL[声母]）整支替代）+ 尾键补全面;
            //   律 D 已在顶部整支接管。
            if adv == AdvShape::B {
                let k0 = &inp[..1];
                if let Some(tbl) = rd_pin_col_get(k0) {
                    for (t, sc) in tbl {
                        let w = self.panel_wid(t);
                        if w < 0 {
                            continue;
                        }
                        let lpq = lp_override_get(t)
                            .unwrap_or_else(|| self.word_logp(w as u32));
                        // reading = 律域读音键（非词主读音——咳 主读音 hai
                        // 型异读字会误触 query 层 _fs fbcol 净化路）
                        let mut c = Cand::new(t.to_string(), lpq, Kind::Fbcol);
                        c.reading = k0.to_string();
                        c.wid = w;
                        c.score_bias = sc + ADV_INI_COL_LIFT - lpq;
                        out.push(c);
                    }
                }
            }
            if adv == AdvShape::C {
                let mut seen_i: HashSet<i64> = HashSet::new();
                for c in self.fb_ini_column(inp.as_bytes()[0], 10) {
                    if !seen_i.contains(&c.wid) {
                        seen_i.insert(c.wid);
                        out.push(c);
                    }
                }
                // 尾键补全面: 简拼全键首词 截尾音节 + \u{1} + 尾键补全列首字
                // （native zgrm#1 中国人\u{1}吗 = 全键词 中国人民(zgrm, pack
                // 首) 截 尾音节 min(尾键 m 的音节) → 中国人(zgr 词) × m→ma
                // 补全列首 吗; ≥4 键门排除 wsm 型 3 键词（截尾=2 音节,
                // native wsm 无 网上\u{1}吗）; rmyy pack 首 人民医院 截尾
                // ren'min'yi 非读音 → 零面（native 同）; 钉分 = 全键词
                // 中国人民(−5.977) 之下/整个人懵(−5.99 OV) 之上 = 原生位#1）
                if inp.len() >= 4 {
                    if let Some(ids) = self.jianpin_keys(inp).first() {
                        let seq: Vec<&str> = ids
                            .iter()
                            .map(|&i2| self.syllables[i2 as usize].as_str())
                            .collect();
                        if seq.len() == inp.len() && seq.len() >= 4
                            && seq[seq.len() - 1].as_bytes()[0]
                                == inp.as_bytes()[inp.len() - 1]
                        {
                            let joined: String = seq[..seq.len() - 1].concat();
                            let st3 = self.dat.walk(joined.as_bytes(), ROOT);
                            if st3 >= 0 && self.dat.is_terminal(st3) {
                                let (lo3, _c3) = self.dat.range(st3);
                                let (o03, o13) = self.reading_slice(lo3);
                                if o13 > o03 {
                                    let w3 = self.wlist_at(o03);
                                    for seq2 in self
                                        .fb_compose(&inp[inp.len() - 1..])
                                        .into_iter()
                                        .take(1)
                                    {
                                        if seq2.is_empty() {
                                            break;
                                        }
                                        let s0 = &seq2[0];
                                        let cst3 =
                                            self.dat.walk(s0.as_bytes(), ROOT);
                                        if cst3 < 0
                                            || !self.dat.is_terminal(cst3)
                                        {
                                            break;
                                        }
                                        let (clo3, _c4) = self.dat.range(cst3);
                                        let (co03, co13) =
                                            self.reading_slice(clo3);
                                        if co13 <= co03 {
                                            break;
                                        }
                                        let wlp3 = self.word_logp(w3);
                                        let mut c = Cand::new(
                                            format!(
                                                "{}\u{1}{}",
                                                self.word_text(w3),
                                                self.word_text(
                                                    self.wlist_at(co03)
                                                )
                                            ),
                                            wlp3,
                                            Kind::Fcomp,
                                        );
                                        c.score_bias = -5.98 - wlp3;
                                        out.push(c);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let mut seen_w: HashSet<u32> = HashSet::new();
            // 尾字列读音律（面触发域内）: 尾单元音独立成段（pgde 的
            // e——fuzzcomp 结构枚举 (j p)(j g)(j d)(d e) 尾段）→ 列=尾
            // 元音读音; 否则首段读音 frd（dakn frd=da = RD_PIN_COL['da'];
            // peiy/jint frd 自然列; 表外域维持 frd 原样——klei 尾 i 非
            // 单元音且无面触发, 列仍=首段 ke, 原行为逐位）
            let uv: &str = if inp.is_empty() { "" } else { &inp[inp.len() - 1..] };
            let tail_uv =
                matches!(uv, "a" | "e" | "o") && self.sylset_m().contains(uv);
            let col_rd: String = if adv == AdvShape::B {
                "__adv_b_skip__".to_string() // 律 B 列已发, 标准列跳过
            } else if adv == AdvShape::C {
                "__adv_c_skip__".to_string() // 律 C 栈列已发, 标准列跳过
            } else if matches!(adv, AdvShape::A { .. }) {
                // 律 A: 头 exact 音节列（RD_PIN_COL[头音节] 校准——
                // kexsi/buxdui 的 [d头][j中][d尾] 结构被 单键简拼×长直读尾
                // 剪枝剔除, pool frd=compose(首键)列(kan)不可用 → 直取律头）
                adv.head_syl().unwrap_or_else(|| frd.clone())
            } else if pin_fired && tail_uv {
                uv.to_string()
            } else {
                frd.clone()
            };
            if let Some(col) = rd_pin_col_get(&col_rd) {
                // W7O2: 尾字列钉序（native 逐位校准——zhao/da/e 列,
                // SLOT_COL 同构; 钉分=绝对终分免重排）
                for (t, sc) in col {
                    let w = self.panel_wid(t);
                    if w < 0 {
                        continue;
                    }
                    let lpq = lp_override_get(t)
                        .unwrap_or_else(|| self.word_logp(w as u32));
                    let mut c = Cand::new(t.to_string(), lpq, Kind::Fbcol);
                    c.reading = col_rd.clone();
                    c.wid = w;
                    c.score_bias = sc - lpq;
                    out.push(c);
                }
            } else {
                self.fb_seg_columns(&col_rd, &mut out, &mut seen_w, depth);
            }
            // 头声母栈列律（面触发域内）: 尾单元音独立段 × 头段悬空
            // （输入头两键不成音节——pgde 头 p; peiy 头 pei 成音节 → 不发;
            // dakn 头 da 成音节 → 不发）→ 首声母栈列垫底（pgde: 拍怕跑…）
            if pin_fired && tail_uv {
                let head_dangling = !(2..=(6usize).min(inp.len()))
                    .any(|l| self.sylset_m().contains(&inp[..l]));
                if head_dangling {
                    out.extend(self.fb_ini_column(inp.as_bytes()[0], 6));
                }
            }
        } else if !out.is_empty() && inp.len() > 8 {
            let mut seen_w: HashSet<u32> = HashSet::new();
            self.fb_seg_columns(&frd, &mut out, &mut seen_w, depth);
        } else if !out.is_empty() && (head_kind == b'd' || head_kind == b's') {
            let mut seen_w: HashSet<u32> = HashSet::new();
            self.fb_seg_columns(&frd, &mut out, &mut seen_w, depth);
        } else if !out.is_empty() && head_kind == b'j' {
            if first_seg_single_j {
                let mut seen_i: HashSet<i64> = HashSet::new();
                for c in self.fb_ini_column(inp.as_bytes()[0], depth) {
                    if !seen_i.contains(&c.wid) {
                        seen_i.insert(c.wid);
                        out.push(c);
                    }
                }
            } else {
                let mut seen_w: HashSet<u32> = HashSet::new();
                self.fb_seg_columns(&frd, &mut out, &mut seen_w, depth);
            }
        }
        out
    }

    // ------------------------------------------------------------------
    // 役N 第十一波(N5b): ADV 结构律（native_ref4 agentN4 边族六项——
    // fuzzcomp 洪面在这些输入形上 native 全零（junk j 段补全链面,
    // 我方 bias 6.0/9.0 洪板埋没正确通道候选）→ 结构判形 + 面抑制 +
    // 律面/律列补给。判形零查询串键, 纯贪切碎分结构; 34 集/ref3 各锚
    // 逐形排除验证（dakn 双 x 中缀 / klei 尾单音节 / cchuang 头单音节
    // 尾单段 / maag 塌缩串不成读 / zhaong 尾悬空 / pgde 头元音单段 /
    // buxcuo 双 x / jttq R3 深列早退）。（py `_adv_shape` 族, tick62）
    // ------------------------------------------------------------------
    fn adv_shape(&self, inp: &str) -> AdvShape {
        let n = inp.len();
        if !(4..=11).contains(&n) {
            return AdvShape::None;
        }
        let b = inp.as_bytes();
        // 律 D 先判（与切分有效性无关——keexi 的 e 单元音段不阻塌缩）
        for k in 0..n - 1 {
            if b[k] == b[k + 1] && b"aeiou".contains(&b[k]) {
                let c = vv_collapse(inp);
                if c != inp {
                    let st = self.dat.walk(c.as_bytes(), ROOT);
                    if st >= 0 && self.dat.is_terminal(st) {
                        return AdvShape::D;
                    }
                }
                break; // 仅首处叠元音判塌缩（maag 型不成读即出, 不逐处试）
            }
        }
        let syls = self.sylset_m();
        let mut segs: Vec<(u8, &str)> = Vec::new();
        let mut i = 0usize;
        while i < n {
            let mut hit: Option<&str> = None;
            let hi = 6.min(n - i);
            for l in (2..=hi).rev() {
                let seg = &inp[i..i + l];
                if syls.contains(seg) {
                    hit = Some(seg);
                    break;
                }
            }
            if let Some(h) = hit {
                segs.push((b'd', h));
                i += h.len();
                continue;
            }
            if b"aeoiuv".contains(&b[i]) {
                return AdvShape::None; // 单元音段=既有链域
            }
            segs.push((b'x', &inp[i..i + 1]));
            i += 1;
        }
        let xs: Vec<usize> = segs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.0 == b'x')
            .map(|(k, _)| k)
            .collect();
        let ds: Vec<&str> = segs
            .iter()
            .filter(|s| s.0 == b'd')
            .map(|s| s.1)
            .collect();
        if ds.is_empty() {
            // 律 C: 全单声母链 × 简拼全键词在板（zgrm 中国人民/rmyy
            // 人民医院/wsm 为什么——词先行字列后, native 无洪面）
            if xs.len() >= 3 && !self.jianpin_keys(inp).is_empty() {
                return AdvShape::C;
            }
            return AdvShape::None;
        }
        if xs.len() == 1 {
            let k = xs[0];
            let ini = segs[k].1;
            if k == 0 {
                // 律 B: 头悬空声母 × 尾 exact ≥2 段（wtianqi/mgaoyuan;
                // klei/cchuang 尾单段不湿——native 词洪面板形态）
                if ds.len() >= 2
                    && FB_INIS.contains(&ini.as_bytes()[0])
                    && (k + 1 >= segs.len() || segs[k + 1].1 != ini)
                {
                    return AdvShape::B;
                }
            } else if k < segs.len() - 1 {
                // 律 A: exact×悬空中缀×exact（kexsi/buxdui）
                return AdvShape::A {
                    head_syl: ds[0].to_string(),
                };
            }
        }
        AdvShape::None
    }

    /// 律 D 塌缩面（native_ref4 haao/keexi 行 0+ 逐位）:
    ///   主块 = 塌缩串读音列首词 + 同头字词 cap3（keexi: 可惜/可喜/可洗
    ///     ——列头 可惜 首字 '可' 同头过滤掉 课洗/科西/柯西; haao: 好
    ///     单发——列次 号 异头断）;
    ///   次块 = 塌缩串邻键(dist1) KBN 最强次读音 × flags≠0 词 cap2
    ///     （keexi: kezi 壳子/刻字; lp ≤ ADV_SUB_LP_GATE 门——haao gao
    ///     族 高−2.6 native 全无）。kind=fcomp（W14 免域）,
    ///     bias=ADV_COLLAPSE_BOOST。（py `_adv_collapse_faces`）
    fn adv_collapse_faces(&self, inp: &str) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let c = vv_collapse(inp);
        if c == inp {
            return out;
        }
        let st = self.dat.walk(c.as_bytes(), ROOT);
        if st < 0 || !self.dat.is_terminal(st) {
            return out;
        }
        let (lo, _cnt) = self.dat.range(st);
        let (o0, o1) = self.reading_slice(lo);
        let mut col: Vec<(String, usize)> = Vec::new();
        for oi in o0..o1.min(o0 + 48) {
            col.push((self.word_text(self.wlist_at(oi)), oi));
        }
        if col.is_empty() {
            return out;
        }
        let head_ch = match col[0].0.chars().next() {
            Some(ch) => ch,
            None => return out,
        };
        let pretty = self.rid_pretty(lo);
        let mut got = 0usize;
        for (t, oi) in &col {
            if t.chars().next() != Some(head_ch) {
                continue;
            }
            let w = self.wlist_at(*oi);
            let mut cnd = Cand::new(
                t.clone(),
                self.word_logp(w),
                Kind::Fcomp,
            );
            cnd.reading = pretty.clone();
            cnd.wid = w as i64;
            cnd.score_bias = ADV_COLLAPSE_BOOST;
            out.push(cnd);
            got += 1;
            if got >= 3 {
                break;
            }
        }
        // 邻键 KBN 次读音词块: 择最强次读音（列首词 lp——keexi: kezi 壳子
        // > keci 科次）, 发其 flags≠0 词 cap2（native 壳子/刻字 逐位——
        // 客咨/可子/课子/课自/科次 flags 0 造词 native 全无）
        let cb = c.as_bytes();
        let mut best_rd: Option<(f64, u32, usize, usize)> = None;
        for k2 in 0..cb.len() {
            for nb in kbn_neighbors(cb[k2]) {
                let mut v = cb.to_vec();
                v[k2] = nb;
                let vs = String::from_utf8_lossy(&v).into_owned();
                let (p0, cc0) = kb_pos(cb[k2]);
                let (p1, cc1) = kb_pos(nb);
                if (p0 - p1).abs() + (cc0 - cc1).abs() >= 2 {
                    continue;
                }
                let st2 = self.dat.walk(vs.as_bytes(), ROOT);
                if st2 < 0 || !self.dat.is_terminal(st2) {
                    continue;
                }
                let (lo2, _c2) = self.dat.range(st2);
                let (o02, o12) = self.reading_slice(lo2);
                if o12 <= o02 {
                    continue;
                }
                let w0 = self.wlist_at(o02);
                let lp0 = self.word_logp(w0);
                let better = match &best_rd {
                    None => true,
                    Some(br) => lp0 > br.0,
                };
                if better {
                    best_rd = Some((lp0, lo2, o02, o12));
                }
            }
        }
        if let Some((_lp0, lo2, o02, o12)) = best_rd {
            let pretty2 = self.rid_pretty(lo2);
            let mut got = 0usize;
            for oi2 in o02..o12.min(o02 + 24) {
                let w2 = self.wlist_at(oi2);
                if self._flags[w2 as usize] == 0 {
                    continue;
                }
                let lp2 = self.word_logp(w2);
                if lp2 > ADV_SUB_LP_GATE {
                    continue;
                }
                let mut cnd =
                    Cand::new(self.word_text(w2), lp2, Kind::Fcomp);
                cnd.reading = pretty2.clone();
                cnd.wid = w2 as i64;
                cnd.score_bias = ADV_COLLAPSE_BOOST;
                out.push(cnd);
                got += 1;
                if got >= 2 {
                    break;
                }
            }
        }
        out
    }

    /// 单字母连打直通面（native vvv 行 0: v\u{1}VV, 读音 V#VV——
    /// 全同字母串 = 首字直通 + 余部大写回显, latin 组内先序）。
    /// （py `_adv_v_all_same`）
    fn adv_v_all_same(&self, inp: &str) -> Vec<Cand> {
        let u = (inp.as_bytes()[0].to_ascii_uppercase()) as char;
        let ustr = u.to_string();
        let tail = ustr.repeat(inp.len() - 1);
        let mut c =
            Cand::new(format!("{}\u{1}{}", &inp[..1], tail), 0.0, Kind::Latin);
        c.reading = format!("{}#{}", u, tail);
        c.score = 0.1;
        vec![c]
    }

    /// 律§11 v 头英文态 KBN 面（native_ref4 vfeng 行 0-1: V人\u{1}个/
    /// V人\u{1}给, 读音 V,ren#ge / V,ren#gei）: rest = [≥3 键段 1×KBN 替
    /// →音节][悬空单声母] → 替读首字×补全列首词。替换读音择强（列首词
    /// lp 降序——fen: 人>根>扽）。（py `_v_head_kbn_faces`）
    fn v_head_kbn_faces(&self, rest: &str) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let n = rest.len();
        if n < 4 {
            return out;
        }
        let syls = self.sylset_m();
        let rb = rest.as_bytes();
        let mut best: Vec<(f64, String, u32)> = Vec::new();
        for l in (3..=(6usize).min(n - 1)).rev() {
            let chunk = &rest[..l];
            let tail = &rest[l..];
            if tail.len() != 1
                || syls.contains(tail)
                || !FB_INIS.contains(&rb[l])
            {
                continue;
            }
            let cb = chunk.as_bytes();
            for k2 in 0..l {
                for nb in kbn_neighbors(cb[k2]) {
                    let mut v = cb.to_vec();
                    v[k2] = nb;
                    let vs = String::from_utf8_lossy(&v).into_owned();
                    if !syls.contains(&vs) {
                        continue;
                    }
                    let st = self.dat.walk(vs.as_bytes(), ROOT);
                    if st < 0 || !self.dat.is_terminal(st) {
                        continue;
                    }
                    let (lo, _c) = self.dat.range(st);
                    let (o0, o1) = self.reading_slice(lo);
                    if o1 <= o0 {
                        continue;
                    }
                    let w = self.wlist_at(o0);
                    best.push((self.word_logp(w), vs, w));
                }
            }
            if !best.is_empty() {
                break; // 最长段优先（fen 段优先于 eng 段）
            }
        }
        if best.is_empty() {
            return out;
        }
        best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let (lp, v, hw) = &best[0];
        let htxt = self.word_text(*hw);
        let mut got = 0usize;
        let mut seen_t: HashSet<String> = HashSet::new();
        for seq in self.fb_compose(&rest[n - 1..]).into_iter().take(6) {
            if seq.is_empty() {
                continue;
            }
            let s = &seq[0];
            let cst = self.dat.walk(s.as_bytes(), ROOT);
            if cst < 0 || !self.dat.is_terminal(cst) {
                continue;
            }
            let (clo, _c2) = self.dat.range(cst);
            let (co0, co1) = self.reading_slice(clo);
            if co1 <= co0 {
                continue;
            }
            let cw = self.wlist_at(co0);
            let t = self.word_text(cw);
            if seen_t.contains(&t) {
                continue;
            }
            seen_t.insert(t.clone());
            // 面形 = V+替读首字 连写 × \u{1} 补全字（native vfeng 行 0:
            // 'V人\u{1}个'——V 与替读首字同段, 补全字独立段）
            let mut c = Cand::new(
                format!("V{}\u{1}{}", htxt, t),
                *lp,
                Kind::Latin,
            );
            c.reading = format!("V,{}#{}", v, s);
            c.score = 1.0 - 0.1 * got as f64;
            c.wid = -1;
            out.push(c);
            got += 1;
            if got >= 2 {
                break;
            }
        }
        out
    }

    /// 役N W1(第五波): G1 兜底 律§7/§10 面族（dangling_slot_law.md）。
    /// 零命中 → 空（34 集全为覆盖/面板态, 不经 G1, 零接触）。
    /// 律§7 悬空×exact 混合链: [悬空单声母][exact 音节][悬空单声母]——
    ///   相邻字母成合法音节时精确消费优先, 悬空仅单字母段; 头段连写=
    ///   词典词 × 尾悬空→补全音节(权重序 top-3)字列 oi0 各一面
    ///   （native bfon: b→bai][fo exact][n→ni/ne] → 拜佛\x01你/拜佛\x01呢,
    ///   全悬空链 不发那 类 EPYN 替换面 native 无 → 族面在场时旧链
    ///   sentence 面撤下, 字列不动）。
    /// 律§10 相邻换位×邻键吸收: native ekwk = ek→ke 换位 × wk→wo
    ///   (k→o 邻键吸收) → ke,wo 读音词列 top-2 可我/客卧。门: 4~6 键
    ///   且全串无 ≥2 字母 exact 段（有则精确消费优先——dakn da 在头、
    ///   ncei ei 在尾, 皆不换位）; 词面取 SLOT_SWAP_RD 表（我方 ke,wo
    ///   列序 课我>可我>…>客卧 与 native 反序且 课我 native 无面 →
    ///   表钉序, 先例 SLOT_FUSE; 表外读音零发射）。（py `_fb_law_faces`）
    pub(super) fn fb_law_faces(&self, inp: &str) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let n = inp.len();
        if n < 4 {
            return out;
        }
        let syls_m = self.sylset_m();
        // —— 律§10 ——
        if n <= 6 {
            let mut has_exact = false;
            for i2 in 0..n.saturating_sub(1) {
                let hi = (6usize).min(n - i2);
                for l in 2..=hi {
                    if syls_m.contains(&inp[i2..i2 + l]) {
                        has_exact = true;
                        break;
                    }
                }
                if has_exact {
                    break;
                }
            }
            if !has_exact {
                for i2 in 0..n - 1 {
                    let mut sw = vec![inp.as_bytes()[i2 + 1], inp.as_bytes()[i2]];
                    let sw_s = String::from_utf8_lossy(&sw).into_owned();
                    let _ = &mut sw;
                    if inp[i2..i2 + 2] == sw_s
                        || !syls_m.contains(&sw_s)
                    {
                        continue;
                    }
                    let rest = format!(
                        "{}{}",
                        &inp[..i2],
                        &inp[i2 + 2..]
                    );
                    if !(2..=4).contains(&rest.len()) {
                        continue;
                    }
                    let mut rsl: Option<String> =
                        if syls_m.contains(&rest) {
                            Some(rest.clone())
                        } else {
                            None
                        };
                    if rsl.is_none() {
                        let rb = rest.as_bytes();
                        for p in 0..rb.len() {
                            for nb in kbn_neighbors(rb[p]) {
                                let mut v = rb.to_vec();
                                v[p] = nb;
                                let vs = String::from_utf8_lossy(&v)
                                    .into_owned();
                                if syls_m.contains(&vs) {
                                    rsl = Some(vs);
                                    break;
                                }
                            }
                            if rsl.is_some() {
                                break;
                            }
                        }
                    }
                    let Some(rsl_s) = rsl else { continue };
                    for (t, lp) in slot_swap_rd(&format!("{}{}", sw_s, rsl_s))
                    {
                        let mut c = Cand::new(t.to_string(), *lp, Kind::Sentence);
                        c.reading = format!("{}'{}", sw_s, rsl_s);
                        c.score = *lp;
                        c.wid = -1;
                        out.push(c);
                    }
                }
            }
        }
        // —— 律§7 ——
        // exact 段判据 = 音节表 ∪ DAT 前缀态（native 音节表有 fo 型表外
        // 音节——拜佛 bai,fo; 我方 syllables 缺 fo 但 DAT 有 fo 前缀态;
        // 发射仍受 LAW7_HEAD 表闸, 判据放宽零外溢）
        let mut segs: Vec<(u8, String)> = Vec::new();
        let mut i = 0usize;
        while i < n {
            let mut hit: Option<String> = None;
            let hi = (6usize).min(n - i);
            for l in (2..=hi).rev() {
                let seg = &inp[i..i + l];
                if syls_m.contains(seg)
                    || self.dat.walk(seg.as_bytes(), ROOT) >= 0
                {
                    hit = Some(seg.to_string());
                    break;
                }
            }
            if let Some(h) = hit {
                i += h.len();
                segs.push((b'e', h));
            } else {
                segs.push((b'x', inp[i..i + 1].to_string()));
                i += 1;
            }
        }
        if segs.len() == 3
            && segs[0].0 == b'x'
            && segs[1].0 == b'e'
            && segs[2].0 == b'x'
        {
            // 头悬空→补全音节 × exact 段连写 = 词典词（native bfon:
            // b→bai × fo → baifo=拜佛; 权重序试至首个成词读音;
            // walk 不可达时（拜佛 rid 越界型孤儿词条）查 LAW7_HEAD 表）
            let mut hw: Option<(String, f64)> = None;
            for seq in self.fb_compose(&segs[0].1).into_iter().take(8) {
                if seq.is_empty() {
                    continue;
                }
                let hs = &seq[0];
                let head_l = format!("{}{}", hs, segs[1].1);
                let st = self.dat.walk(head_l.as_bytes(), ROOT);
                if st < 0 || !self.dat.is_terminal(st) {
                    continue;
                }
                let (lo, _c) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                if o1 <= o0 {
                    continue;
                }
                let w0 = self.wlist_at(o0);
                if self.word_text(w0).chars().count() < 2 {
                    continue;
                }
                hw = Some((self.word_text(w0), self.word_logp(w0)));
                break;
            }
            if hw.is_none() {
                if let Some(ht) =
                    law7_head(segs[0].1.as_bytes()[0], &segs[1].1)
                {
                    if let Some(hwid) = self.word_id(ht) {
                        hw =
                            Some((ht.to_string(), self.word_logp(hwid)));
                    }
                }
            }
            if let Some((htxt, wlp)) = hw {
                for seq in
                    self.fb_compose(&segs[2].1).into_iter().take(3)
                {
                    if seq.is_empty() {
                        continue;
                    }
                    let s = &seq[0];
                    let cst = self.dat.walk(s.as_bytes(), ROOT);
                    if cst < 0 || !self.dat.is_terminal(cst) {
                        continue;
                    }
                    let (clo, _cc) = self.dat.range(cst);
                    let (co0, co1) = self.reading_slice(clo);
                    if co1 <= co0 {
                        continue;
                    }
                    let cw = self.wlist_at(co0);
                    let clp = self.slot_logp(cw, co0);
                    let mut c = Cand::new(
                        format!("{}\u{1}{}", htxt, self.word_text(cw)),
                        wlp + clp,
                        Kind::Sentence,
                    );
                    c.score = wlp + clp;
                    c.wid = -1;
                    c.score_bias = LAW7_BIAS;
                    out.push(c);
                }
            }
        }
        out
    }

    // ------------------------------------------------------------------
    // 役N ref5-R2: E 域（E_mix 简拼混合 / D_kbn 删除域 / F_dangling 悬空）
    // ------------------------------------------------------------------
    /// E 域原子切分。返回 (域, atoms) 或 None。
    /// atoms: I(字母) 声母键 / C("sh"|"zh"|"ch") 复合声母 / E(音节) 精确
    /// 直读 / T(音节, chunk) 元音乱序音节。
    /// 域: E1 全声母链 3~5 键无元音; E2 混合(含 e/c); E3 乱序头。
    /// （py `_emix_atoms`）
    fn emix_atoms(&self, inp: &str) -> Option<(&'static str, Vec<EmixAtom>)> {
        let n = inp.len();
        if !(3..=8).contains(&n) {
            return None;
        }
        let syls = self.sylset_m();
        let ana = self.emix_ana();
        let mut atoms: Vec<EmixAtom> = Vec::new();
        let mut i = 0usize;
        while i < n {
            let mut hit: Option<EmixAtom> = None;
            for l in (2..=(6.min(n - i))).rev() {
                if syls.contains(&inp[i..i + l]) {
                    hit = Some(EmixAtom::E(inp[i..i + l].to_string()));
                    break;
                }
            }
            if hit.is_none() && n - i >= 4 {
                let chunk = &inp[i..i + 4];
                let cands: Vec<String> =
                    ana.get(chunk).cloned().unwrap_or_default();
                for s in cands {
                    // tick66: 乱序头仅限 chunk 本身非合法音节（zhao 型
                    // exact 段已在上方音节层命中不会进此支; 若 chunk 恰
                    // 为合法音节则非乱序——zhaong 的 zhao 走 E() 域,
                    // 防 T 域劫持 exact 切分（ref3 zhaong 找工 回归根因）)
                    if s.as_bytes()[0] == chunk.as_bytes()[0]
                        && s != chunk
                        && !syls.contains(chunk)
                    {
                        hit = Some(EmixAtom::T(
                            s.clone(),
                            chunk.to_string(),
                        ));
                        break;
                    }
                }
            }
            if let Some(h) = hit {
                let adv = match &h {
                    EmixAtom::T(_, _) => 4usize,
                    EmixAtom::E(s) => s.len(),
                    _ => unreachable!(),
                };
                atoms.push(h);
                i += adv;
                continue;
            }
            let b0 = inp.as_bytes()[i];
            if b"aeoiuv".contains(&b0) {
                return None; // 散元音 = 既有链域（toan/maag 等）
            }
            if (b0 == b'z' || b0 == b'c' || b0 == b's')
                && i + 1 < n
                && inp.as_bytes()[i + 1] == b'h'
            {
                atoms.push(EmixAtom::C(inp[i..i + 2].to_string()));
                i += 2;
                continue;
            }
            if FB_INIS.contains(&b0) {
                atoms.push(EmixAtom::I((b0 as char).to_string()));
                i += 1;
                continue;
            }
            return None;
        }
        let _kinds_i = atoms.iter().any(|a| matches!(a, EmixAtom::I(_)));
        let kinds_e = atoms.iter().any(|a| matches!(a, EmixAtom::E(_)));
        // tick66: exact头×≥2尾悬空声母（zhao+n+g 型）= 既有 DANGLING
        // 双补全域（ref3 zhaong 找\x01那个 #0 律）——不归 EMIX, 防找工
        // 钉面劫持（R2 在途波遗留回归根因）
        if kinds_e
            && matches!(atoms[0], EmixAtom::E(_))
            && atoms.iter().filter(|a| matches!(a, EmixAtom::I(_))).count() >= 2
        {
            return None;
        }
        let all_i = atoms.iter().all(|a| matches!(a, EmixAtom::I(_)));
        if all_i {
            if (3..=5).contains(&atoms.len())
                && !inp.bytes().any(|c| b"aeiou".contains(&c))
                && inp.as_bytes()[0] != b'v'
            {
                return Some(("E1", atoms));
            }
            return None;
        }
        if atoms.iter().any(|a| matches!(a, EmixAtom::T(_, _))) {
            return Some(("E3", atoms));
        }
        Some(("E2", atoms))
    }

    /// E 域乱序音节分析表（py `_emix_ana_v`; 键 = 3~6 键音节的排序
    /// 字母串, 值 = 同字母集合法音节表, 插入序 = 音节表序）。
    fn emix_ana(
        &self,
    ) -> std::cell::Ref<'_, HashMap<String, Vec<String>>> {
        if self.emix_ana_v.borrow().is_none() {
            let mut m: HashMap<String, Vec<String>> = HashMap::new();
            for s in self.syllables.iter() {
                let bl = s.len();
                if 3 <= bl && bl <= 6 {
                    let mut cs: Vec<char> = s.chars().collect();
                    cs.sort_unstable();
                    let key: String = cs.into_iter().collect();
                    m.entry(key).or_default().push(s.clone());
                }
            }
            *self.emix_ana_v.borrow_mut() = Some(m);
        }
        std::cell::Ref::map(self.emix_ana_v.borrow(), |o| o.as_ref().unwrap())
    }

    /// 读音单字列（DAT 原始 oi 序 = 词典序, 不按 lp 重排——native
    /// xioala xi 列 系洗西喜… 实证）。（py `_emix_char_col`）
    fn emix_char_col(&self, rd: &str, k: usize) -> Vec<String> {
        let st = self.dat.walk(rd.as_bytes(), ROOT);
        if st < 0 || !self.dat.is_terminal(st) {
            return Vec::new();
        }
        let (lo, _cnt) = self.dat.range(st);
        let (o0, o1) = self.reading_slice(lo);
        let mut cand: Vec<(f64, String)> = Vec::new();
        for oi in o0..o1.min(o0 + 48) {
            let w = self.wlist_at(oi);
            cand.push((self.word_logp(w), self.word_text(w)));
        }
        cand.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        cand.truncate(k);
        cand.into_iter().map(|(_lp, t)| t).collect()
    }

    /// 读音列首词（fneq=true 取首个 flags≠0 词——native 族词过滤:
    /// 筋时/金市/窝嫩 f0 全隐没）。返回 (word, wid) 或 None。
    /// （py `_emix_rd_head`）
    fn emix_rd_head(&self, rd: &str, fneq: bool) -> Option<(String, u32)> {
        let st = self.dat.walk(rd.as_bytes(), ROOT);
        if st < 0 || !self.dat.is_terminal(st) {
            return None;
        }
        let (lo, _cnt) = self.dat.range(st);
        let (o0, o1) = self.reading_slice(lo);
        for oi in o0..o1 {
            let w = self.wlist_at(oi);
            if fneq && self._flags[w as usize] == 0 {
                continue;
            }
            return Some((self.word_text(w), w));
        }
        None
    }

    /// 简拼键串 → 头部展开词列 [(word, wid, reading_syls)]（flags≠0,
    /// pack 行序）。单键 → 空表。（py `_emix_pack_word`）
    fn emix_pack_word(
        &self,
        keys: &str,
        k: usize,
    ) -> Vec<(String, i64, Vec<String>)> {
        if keys.len() < 2 {
            return Vec::new();
        }
        let mut out: Vec<(String, i64, Vec<String>)> = Vec::new();
        for ids in self.jianpin_keys(keys).into_iter().take(8) {
            let syls_v: Vec<String> = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect();
            let rd = syls_v.concat();
            let Some(hw) = self.emix_rd_head(&rd, true) else {
                continue;
            };
            out.push((hw.0, hw.1 as i64, syls_v));
            if out.len() >= k {
                break;
            }
        }
        out
    }

    /// tick66: 简拼**精确键**存在性（前缀展开不算——jianpin_keys
    /// 前缀区间会污染: zdmy 命中 zd* 前缀键但无 zdmy 精确键;
    /// zgrm 中国人民 有 zgrm 精确键 = shape C 律 C 健康锚, 交还 EMIX 门）。
    /// （py `_emix_full_key`）
    fn emix_full_key(&self, keys: &str) -> bool {
        let n = self.jian_n;
        let (mut a, mut b) = (0usize, n);
        while a < b {
            let mid = (a + b) / 2;
            let k = self.jian_keys.get(mid);
            if k == keys {
                return true;
            }
            if k.as_str() < keys {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        false
    }

    /// 构造对面验证: 引擎 bigram 对在表 + W1+W2 非词典词（flags≠0——
    /// 保 jttq 今天天气/peiy 喜欢吗 型整词通道面, 窝嫩 f0 例外放行）。
    /// （py `_emix_pair_ok`, 当前主链未消费, 保留镜像）
    #[allow(dead_code)]
    fn emix_pair_ok(&self, w1: &str, w2: &str, _e1: &str, _e2: &str) -> bool {
        let (wa, wb) = (self.word_id(w1), self.word_id(w2));
        let (Some(wa), Some(wb)) = (wa, wb) else {
            return false;
        };
        let big = self.bigram_after(wa);
        if big.is_empty() || !big.contains(wb) {
            return false;
        }
        if let Some(cw) = self.word_id(&format!("{}{}", w1, w2)) {
            if self._flags[cw as usize] != 0 {
                return false;
            }
        }
        true
    }

    /// 钉分面发射。items = [(word, wid, reading)] → fcomp 钉面。
    /// tick66: start = 钉分接续起点（字列接续钉面序, 防 col 重置抢位）。
    /// （py `_emix_emit`; wid=0 兜 -1 与 py `word_id or -1` 同律）
    fn emix_emit(&self, out: &mut Vec<Cand>, items: Vec<(String, Option<i64>, String)>, start: Option<f64>) {
        let mut sc = start.unwrap_or(EMIX_FACE0);
        let mut seen: HashSet<String> = HashSet::new();
        for (word, wid, reading) in items {
            if !seen.insert(word.clone()) {
                continue;
            }
            let w: i64 = match wid {
                Some(v) if v >= 0 => v,
                _ => match self.word_id(&word) {
                    Some(0) => -1, // py `or -1` 假值语义
                    Some(v) => v as i64,
                    None => -1,
                },
            };
            let mut c = Cand::new(word, sc, Kind::Fcomp);
            c.reading = reading;
            c.score = sc;
            c.wid = w;
            c.lp_locked = true;
            c.score_bias = 0.0;
            out.push(c);
            sc += EMIX_STEP;
        }
    }

    /// E 域整面（native_ref5 E_mix/D_kbn/F_dangling 逐位律）。（py
    /// `_q_emix_panel` 入口; E1 与 E2/E3 两支 py 同函数先后 return,
    /// rust 拆两方法零语义差）
    pub(super) fn q_emix_panel(&self, inp: &str) -> Vec<Cand> {
        let Some((dom, atoms)) = self.emix_atoms(inp) else {
            return Vec::new();
        };
        if dom == "E1" {
            self.emix_panel_e1(inp, &atoms)
        } else {
            self.emix_panel_e23(inp, dom, atoms)
        }
    }

    /// E1 纯声母链: 构造对偶面(简拼分裂×bigram/读音钉) + 全键 pack 词 +
    /// KBN 直读词(erz=er|zi 骨架吞并→儿子) + 首两键 KBN 模糊音节字列
    /// (邻键行: zdmy zd→ze / nhm nh→nu; 同行/跨行拒: bjzt b j→i 行距2/
    /// wrz w 同行 e → 首声母栈列)。（py `_q_emix_panel` E1 支）
    fn emix_panel_e1(&self, _inp: &str, atoms: &[EmixAtom]) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let syls = self.sylset_m();
        let stack = self.fb_ini_stack();
        let keys: String = atoms
            .iter()
            .filter_map(|a| match a {
                EmixAtom::I(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        let k = keys.len();
        // tick67b: 精确键在表 → 交还简拼通道——但有 EMIX_PAIR_PINS 命中
        // 或 bigram 强对(≥−2.0)时不交还（nhm 精确键 你好忙 存在但 native
        // 要 你还\\x01没 EMIX 面顶位 > jianpin; mzd 毛泽东 无强对 → 交还）
        // tick67c: 去掉 adv_shape=='C' 门（mzd adv_shape='' 但有精确键
        // → 毛泽东 被劫持根因）——精确键在表即交还（除非有钉面/强对）
        let has_pin = EMIX_PAIR_PINS
            .iter()
            .any(|(pk, _)| pk.split('#').collect::<Vec<_>>().join("") == keys);
        let mut has_strong = false;
        if !has_pin && (3..=5).contains(&keys.len())
            && !keys.chars().any(|c| "aeiou".contains(c))
        {
            'outer: for p in 1..keys.len() {
                let lk = &keys[..p];
                let rk = &keys[p..];
                let lw = self.emix_pack_word(lk, 3);
                let rw = self.emix_pack_word(rk, 3);
                for (_, w1, _) in &lw {
                    let big = self.bigram_after(*w1 as u32);
                    for (_, w2, _) in &rw {
                        if let Some(v) = big.map.get(&(*w2 as u32)) {
                            if *v >= EMIX_PAIR_STRONG {
                                has_strong = true;
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }
        if self.emix_full_key(&keys) && !has_pin && !has_strong {
            return out;
        }
        let full_rds: HashSet<String> = self
            .jianpin_keys(&keys)
            .iter()
            .map(|ids| {
                ids.iter()
                    .map(|&i| self.syllables[i as usize].as_str())
                    .collect::<String>()
            })
            .collect();
        let mut pairs: Vec<(f64, String)> = Vec::new();
        for (pk, (face, sval)) in EMIX_PAIR_PINS.iter() {
            let segs: Vec<&str> = pk.split('#').collect();
            // tick67: 修复 all(len==1) 只匹配单字母段的 bug
            // ——zd#my/nh#m 型多字母段键从未生效
            if segs.len() == 2 && segs.concat() == keys {
                pairs.push((*sval, (*face).to_string()));
            }
        }
        for p in 1..k {
            let (lk, rk) = (&keys[..p], &keys[p..]);
            let mut lefts = self.emix_pack_word(lk, 6);
            if lk.len() == 1 {
                lefts.clear();
                for sy in stack
                    .get(&lk.as_bytes()[0])
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .take(3)
                {
                    let cc = self.emix_char_col(sy, 1);
                    if !cc.is_empty() {
                        lefts.push((cc[0].clone(), -1i64, vec![sy.clone()]));
                    }
                }
            }
            let mut rights = self.emix_pack_word(rk, 2);
            if rk.len() == 1 {
                rights.clear();
                for sy in stack
                    .get(&rk.as_bytes()[0])
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .take(3)
                {
                    let cc = self.emix_char_col(sy, 1);
                    if !cc.is_empty() {
                        rights.push((cc[0].clone(), -1i64, vec![sy.clone()]));
                    }
                }
            }
            if lefts.is_empty() || rights.is_empty() {
                continue;
            }
            for (lw, _x, lsyls) in lefts.iter().take(6) {
                let lrd = lsyls.concat();
                if full_rds.iter().any(|r| {
                    r.starts_with(&lrd) && *r != lrd
                }) {
                    continue; // 左词读音 = 全键行前缀 → 整词面覆盖
                        //  （zgrm: 中国人×吗 让位 中国人民 pack 行）
                }
                for (rw, _y, _rs) in rights.iter().take(2) {
                    let (wa, wb) = (self.word_id(lw), self.word_id(rw));
                    let (Some(wa), Some(wb)) = (wa, wb) else {
                        continue;
                    };
                    let big = self.bigram_after(wa);
                    if big.is_empty() || !big.contains(wb) {
                        continue;
                    }
                    let bv = big.get(wb);
                    if let Some(cw) =
                        self.word_id(&format!("{}{}", lw, rw))
                    {
                        if self._flags[cw as usize] != 0 {
                            continue; // 拼合整词在典 → 整词通道面
                        }
                    }
                    pairs.push((bv, format!("{}\u{1}{}", lw, rw)));
                }
            }
        }
        pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        // 全键 pack 词面（lp 门——主动脉硬化 −9.47 native 隐没;
        // 强对面在场 → 对面先行（nhm/zdmy), 否则词先行（bjzt/wrz））
        let mut word_items: Vec<(String, Option<i64>, String)> = Vec::new();
        'wloop: for ids in self.jianpin_keys(&keys).into_iter().take(4) {
            let s2: Vec<String> = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect();
            let rd = s2.concat();
            let st = self.dat.walk(rd.as_bytes(), ROOT);
            if st < 0 || !self.dat.is_terminal(st) {
                continue;
            }
            let (lo, _c) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            let mut got = 0usize;
            for oi in o0..o1 {
                let w = self.wlist_at(oi);
                let t = self.word_text(w);
                if self._flags[w as usize] == 0
                    || self.word_logp(w) < EMIX_WORD_GATE
                {
                    continue;
                }
                word_items.push((t, Some(w as i64), s2.join("'")));
                got += 1;
                if got >= 2 || word_items.len() >= 5 {
                    break;
                }
            }
            if word_items.len() >= 5 {
                break 'wloop;
            }
        }
        let pairs_lead =
            pairs.iter().any(|(b, _f)| *b >= EMIX_PAIR_STRONG);
        // KBN 首键替换→音节链直读词（wrz: erz = er|zi 吞并→儿子）
        let mut kbn_words: Vec<(String, Option<i64>, String)> = Vec::new();
        let mut nbs0: Vec<u8> =
            kbn_neighbors(keys.as_bytes()[0]).collect();
        nbs0.sort_unstable();
        for nb in nbs0 {
            let rest = format!("{}{}", nb as char, &keys[1..]);
            let rest_b = rest.as_bytes();
            let mut syl_l: Vec<String> = Vec::new();
            let mut j = 0usize;
            let mut okp = true;
            while j < rest.len() {
                let mut hit: Option<String> = None;
                for l in (2..=(6.min(rest.len() - j))).rev() {
                    let seg = &rest[j..j + l];
                    if syls.contains(seg) {
                        hit = Some(seg.to_string());
                        j += l;
                        break;
                    }
                }
                match hit {
                    Some(h) => syl_l.push(h),
                    None => {
                        if j == rest.len() - 1
                            && stack
                                .get(&rest_b[j])
                                .map(|v| !v.is_empty())
                                .unwrap_or(false)
                        {
                            syl_l.push(stack[&rest_b[j]][0].clone());
                            // j+=1 后即 break——增量死值, 略
                        } else {
                            okp = false;
                        }
                        break;
                    }
                }
            }
            if okp && !syl_l.is_empty() {
                let rd = syl_l.concat();
                if let Some(hw) = self.emix_rd_head(&rd, true) {
                    if hw.0.chars().count() >= 2 {
                        kbn_words.push((hw.0, Some(hw.1 as i64), syl_l.join("'")));
                    }
                }
            }
        }
        // 列: 首两键 KBN 模糊音节字列(邻键行) / 首声母栈列
        let mut col_items: Vec<(String, Option<i64>, String)> = Vec::new();
        let mut fz_rd: Option<String> = None;
        if k >= 2 {
            let kpr1 = kb_pos(keys.as_bytes()[1]);
            let mut nbs1: Vec<u8> =
                kbn_neighbors(keys.as_bytes()[1]).collect();
            nbs1.sort_unstable();
            for nb in nbs1 {
                if !b"aeiou".contains(&nb) {
                    continue;
                }
                let cand = format!("{}{}", keys.as_bytes()[0] as char, nb as char);
                if !syls.contains(&cand) {
                    continue;
                }
                let kprn = kb_pos(nb);
                if kprn.0.abs_diff(kpr1.0) > 1 {
                    continue;
                }
                fz_rd = Some(cand);
                break;
            }
        }
        if let Some(fz) = &fz_rd {
            for ch in self.emix_char_col(fz, EMIX_COL_N) {
                col_items.push((ch, None, fz.clone()));
            }
        } else {
            for c0 in self.fb_ini_column(keys.as_bytes()[0], EMIX_COL_N) {
                col_items.push((c0.word, Some(c0.wid), String::new()));
            }
        }
        let mut items: Vec<(String, Option<i64>, String)> = Vec::new();
        if !pairs_lead {
            items.extend(word_items.iter().cloned());
        }
        items.extend(
            pairs.iter().map(|(_b, f)| (f.clone(), None, String::new())),
        );
        if pairs_lead {
            items.extend(word_items.iter().cloned());
        }
        for (kw, kwid, krd) in kbn_words.iter().take(1) {
            items.push((kw.clone(), *kwid, krd.clone()));
        }
        self.emix_emit(&mut out, items.clone(), None);
        // tick66: 字列接续钉面序（zgrm 在/这 重置抢位根因——字列分
        // 从钉面尾分续降, 保 中国人民/中国人\u{1}吗 族钉面领先）
        self.emix_emit(
            &mut out,
            col_items,
            Some(EMIX_FACE0 + items.len() as f64 * EMIX_STEP),
        );
        out
    }

    /// E2 混合: 词族(原子展开读音×2字词 flags≠0; pack 行序先行——dhao
    /// 单号多好大号都好 = pack('dh') hao$ 行逐位 / lmcn_uni 序——jins/
    /// shjian top10 集合逐位) + 构造对偶面(字列头 bigram 对 wonen
    /// 窝\u{1}嫩/ 头词×尾栈首字 mtians 明天\u{1}是) + KBN 全替(dhao→shao
    /// 少#4)/词面(wonen nen→men/ben 我们我笨) + 头读音词列(tamenl
    /// 他们她们它们)。
    /// E3 元音乱序头(xioala xioa→xiao): 拆字钉面 + 乱序音节×尾(±KBN
    /// l→k 小卡)词族 + 输入头音节列。（py `_q_emix_panel` E2/E3 支）
    fn emix_panel_e23(
        &self,
        inp: &str,
        dom: &str,
        atoms: Vec<EmixAtom>,
    ) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let syls = self.sylset_m();
        let stack = self.fb_ini_stack();
        let idx = lmcn::lmcn_index();
        // pack 行序族（行序 = native frecency——dhao 单号多好大号都好逐位）
        let mut fam_pack: Vec<(f64, String, u32, String)> = Vec::new();
        let mut seen_rd: HashSet<String> = HashSet::new();
        let pk_key: String = atoms.iter().map(|a| a.key()).collect();
        for ids in self.jianpin_keys(&pk_key).into_iter().take(32) {
            if ids.len() != atoms.len() {
                continue;
            }
            let s2: Vec<String> = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect();
            let mut okp = true;
            for (a, sy) in atoms.iter().zip(s2.iter()) {
                match a {
                    EmixAtom::I(ic) => {
                        if sy.is_empty()
                            || sy.as_bytes()[0] != ic.as_bytes()[0]
                        {
                            okp = false;
                        }
                    }
                    EmixAtom::C(cs) => {
                        if !sy.starts_with(cs.as_str()) {
                            okp = false;
                        }
                    }
                    EmixAtom::E(es) => {
                        if sy != es {
                            okp = false;
                        }
                    }
                    EmixAtom::T(ts, _) => {
                        if sy != ts {
                            okp = false;
                        }
                    }
                }
            }
            if !okp {
                continue;
            }
            let rd = s2.concat();
            if seen_rd.contains(&rd) {
                continue;
            }
            seen_rd.insert(rd.clone());
            let st = self.dat.walk(rd.as_bytes(), ROOT);
            if st < 0 || !self.dat.is_terminal(st) {
                continue;
            }
            let (lo, _c) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            let mut got = 0usize;
            for oi in o0..o1 {
                let w = self.wlist_at(oi);
                let t = self.word_text(w);
                let tlen = t.chars().count();
                if self._flags[w as usize] == 0 || !(2..=3).contains(&tlen) {
                    continue;
                }
                let u0 = idx.get_uni(&t).map(|u| u.0).unwrap_or(self.word_logp(w) - 1.5);
                fam_pack.push((u0, t, w, s2.join("'")));
                got += 1;
                if got >= 2 {
                    break;
                }
            }
        }
        // 尾声母/复合声母全栈展开 walk（pack 深域外音节全覆:
        // jins se / jint tie / peiy yin——native 族全录）
        let mut fam_walk: Vec<(f64, String, u32, String)> = Vec::new();
        let mut seen_w2: HashSet<String> =
            fam_pack.iter().map(|(_s, t, _w, _r)| t.clone()).collect();
        if atoms.len() == 2
            && atoms[0].kind_e_or_t()
            && atoms[1].kind_e_or_t()
        {
            let t0 = match &atoms[1] {
                EmixAtom::E(s) => s.clone(),
                EmixAtom::T(s, _) => s.clone(),
                _ => unreachable!(),
            };
            let h0 = match &atoms[0] {
                EmixAtom::E(s) => s.clone(),
                EmixAtom::T(s, _) => s.clone(),
                _ => unreachable!(),
            };
            let mut tails: Vec<String> = vec![t0.clone()];
            if dom == "E3" {
                let mut nbs: Vec<u8> =
                    kbn_neighbors(t0.as_bytes()[0]).collect();
                nbs.sort_unstable();
                for nb in nbs {
                    let v = format!("{}{}", nb as char, &t0[1..]);
                    if syls.contains(&v) && v != t0 {
                        tails.push(v);
                    }
                }
            }
            for t in &tails {
                let rd = format!("{}{}", h0, t);
                let st = self.dat.walk(rd.as_bytes(), ROOT);
                if st < 0 || !self.dat.is_terminal(st) {
                    continue;
                }
                let (lo, _c) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                for oi in o0..o1 {
                    let w = self.wlist_at(oi);
                    let t2 = self.word_text(w);
                    if self._flags[w as usize] == 0
                        || t2.chars().count() != 2
                        || seen_w2.contains(&t2)
                    {
                        continue;
                    }
                    let u0 = idx.get_uni(&t2).map(|u| u.0)
                        .unwrap_or(self.word_logp(w) - 1.5);
                    fam_walk.push((u0, t2.clone(), w, rd.clone()));
                    seen_w2.insert(t2);
                }
            }
        } else if atoms.iter().any(|a| a.kind_i()) {
            let tail_a: Option<&EmixAtom> = if matches!(
                atoms.last(),
                Some(a) if a.kind_i()
            ) {
                atoms.last()
            } else {
                None
            };
            let head_atoms: &[EmixAtom] = if tail_a.is_some() {
                &atoms[..atoms.len() - 1]
            } else {
                &atoms
            };
            let mut heads: Vec<String> = vec![String::new()];
            for a in head_atoms {
                match a {
                    EmixAtom::I(ic) => {
                        let sts = stack
                            .get(&ic.as_bytes()[0])
                            .map(|v| v.as_slice())
                            .unwrap_or(&[]);
                        let mut nh: Vec<String> = Vec::new();
                        for h in &heads {
                            for sy in sts.iter().take(6) {
                                nh.push(format!("{}{}", h, sy));
                            }
                        }
                        heads = nh;
                    }
                    other => {
                        let k = match other {
                            EmixAtom::C(s) => s.clone(),
                            EmixAtom::E(s) => s.clone(),
                            EmixAtom::T(s, _) => s.clone(),
                            _ => unreachable!(),
                        };
                        heads = heads
                            .iter()
                            .map(|h| format!("{}{}", h, k))
                            .collect();
                    }
                }
            }
            let tails: Vec<String> = if head_atoms
                .first()
                .map(|a| matches!(a, EmixAtom::C(_)))
                .unwrap_or(false)
            {
                let cs = match &head_atoms[0] {
                    EmixAtom::C(s) => s.clone(),
                    _ => unreachable!(),
                };
                let hjoin: String = atoms[1..]
                    .iter()
                    .filter_map(|a| match a {
                        EmixAtom::E(s) => Some(s.clone()),
                        EmixAtom::T(s, _) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();
                heads = vec![hjoin];
                self.syllables
                    .iter()
                    .filter(|s| s.starts_with(cs.as_str()))
                    .cloned()
                    .collect()
            } else if let Some(ta) = tail_a {
                match ta {
                    EmixAtom::I(ic) => stack
                        .get(&ic.as_bytes()[0])
                        .cloned()
                        .unwrap_or_default(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            for h in heads.iter().take(8) {
                if h.is_empty() {
                    continue;
                }
                for t in tails.iter().take(24) {
                    if t.is_empty() || t.len() > 6 {
                        continue;
                    }
                    let rd = format!("{}{}", h, t);
                    let st = self.dat.walk(rd.as_bytes(), ROOT);
                    if st < 0 || !self.dat.is_terminal(st) {
                        continue;
                    }
                    let (lo, _c) = self.dat.range(st);
                    let (o0, o1) = self.reading_slice(lo);
                    for oi in o0..o1 {
                        let w = self.wlist_at(oi);
                        let t2 = self.word_text(w);
                        let t2len = t2.chars().count();
                        if self._flags[w as usize] == 0
                            || !(2..=3).contains(&t2len)
                            || seen_w2.contains(&t2)
                        {
                            continue;
                        }
                        let u0 = idx.get_uni(&t2).map(|u| u.0)
                            .unwrap_or(self.word_logp(w) - 1.5);
                        fam_walk.push((u0, t2.clone(), w, rd.clone()));
                        seen_w2.insert(t2);
                    }
                }
            }
        }
        // 读音 lp 钉（native 词频桶校正: jin'shu 金属 → jins #0）
        if self.emix_pin_rd.borrow().is_none() {
            let mut m: HashMap<String, f64> = HashMap::new();
            for (rdp, v) in EMIX_RD_LP_PIN.iter() {
                m.insert(rdp.replace('\'', ""), *v);
            }
            *self.emix_pin_rd.borrow_mut() = Some(m);
        }
        let pin_rd = self.emix_pin_rd.borrow();
        let pin_rd = pin_rd.as_ref().unwrap();
        let mut fam_all: Vec<(f64, String, u32, String)> = fam_pack
            .iter()
            .cloned()
            .chain(fam_walk.iter().cloned())
            .map(|(s0, t, w, r)| {
                let add = pin_rd.get(&r.replace('\'', "")).copied().unwrap_or(0.0);
                (s0 + add, t, w, r)
            })
            .collect();
        fam_all.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });
        // 构造对偶面: 头词×尾栈首字（mtians 明天\u{1}是/tamenl 他们\u{1}了）/
        //   字列头×字列头（wonen 窝\u{1}嫩/我\u{1}恁）
        let mut epairs: Vec<(f64, String)> = Vec::new();
        let hd_rd: Option<String> = atoms.iter().find_map(|a| match a {
            EmixAtom::E(s) => Some(s.clone()),
            EmixAtom::T(s, _) => Some(s.clone()),
            _ => None,
        });
        let t_atom = atoms.last().unwrap();
        let t_is_i = t_atom.kind_i();
        if t_is_i && atoms.len() >= 2 && hd_rd.is_some() {
            let hd = hd_rd.clone().unwrap_or_default();
            let mut hword: Option<String> = None;
            for (_s0, t, _w, r) in fam_all.iter().take(8) {
                let rd_n = r.replace('\'', "");
                if t.chars().count() == 2
                    && rd_n.ends_with(&hd)
                    && rd_n != hd
                {
                    hword = Some(t.clone());
                    break;
                }
            }
            if hword.is_none() {
                let hcols0 = self.emix_char_col(&hd, 1);
                hword = hcols0.into_iter().next();
            }
            if let Some(hw) = hword {
                // py `if wa and wb` — word_id==0 假值排除
                let wa = self.word_id(&hw).filter(|&v| v != 0);
                if let Some(wa) = wa {
                    let tic = match t_atom {
                        EmixAtom::I(s) => s.clone(),
                        _ => unreachable!(),
                    };
                    for sy in stack
                        .get(&tic.as_bytes()[0])
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .take(6)
                    {
                        let cc = self.emix_char_col(sy, 1);
                        if cc.is_empty() {
                            continue;
                        }
                        let wb = self.word_id(&cc[0]).filter(|&v| v != 0);
                        let Some(wb) = wb else {
                            continue;
                        };
                        let big = self.bigram_after(wa);
                        if big.is_empty() || !big.contains(wb) {
                            continue;
                        }
                        let bv = big.get(wb);
                        epairs.push((bv, format!("{}\u{1}{}", hw, cc[0])));
                    }
                }
            }
        } else if atoms.len() == 2 && hd_rd.is_some() && t_atom.kind_e_or_t() {
            let hd = hd_rd.clone().unwrap();
            let tcol = match t_atom {
                EmixAtom::E(s) => s.clone(),
                EmixAtom::T(s, _) => s.clone(),
                _ => unreachable!(),
            };
            let hcols = self.emix_char_col(&hd, 4);
            let tcols = self.emix_char_col(&tcol, 4);
            for h1 in &hcols {
                for t1 in &tcols {
                    let wa = self.word_id(h1).filter(|&v| v != 0);
                    let wb = self.word_id(t1).filter(|&v| v != 0);
                    let (Some(wa), Some(wb)) = (wa, wb) else {
                        continue;
                    };
                    let big = self.bigram_after(wa);
                    if big.is_empty() || !big.contains(wb) {
                        continue;
                    }
                    let bv = big.get(wb);
                    epairs.push((bv, format!("{}\u{1}{}", h1, t1)));
                }
            }
        }
        epairs.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });
        // E3 拆字钉面（xioala: 小\u{1}辣）
        let mut pin_face: Option<String> = None;
        if dom == "E3"
            && atoms.len() == 2
            && matches!(atoms[1], EmixAtom::E(_))
            && hd_rd.is_some()
        {
            let hd = hd_rd.clone().unwrap();
            let t1s = match &atoms[1] {
                EmixAtom::E(s) => s.clone(),
                _ => unreachable!(),
            };
            for (pk, (face, _sv)) in EMIX_PAIR_PINS.iter() {
                let pk_np = pk.replace('\'', "");
                let segs: Vec<&str> = pk_np.split('#').collect();
                if segs.len() == 2 && segs[0] == hd && segs[1] == t1s {
                    pin_face = Some((*face).to_string());
                    break;
                }
            }
        }
        // 头读音词列（全 e 头: tamenl 他们/她们/它们/他门/塔门）
        let mut head_words: Vec<(String, Option<i64>, String)> = Vec::new();
        if atoms.len() >= 2
            && t_is_i
            && atoms[..atoms.len() - 1]
                .iter()
                .all(|a| matches!(a, EmixAtom::E(_)))
        {
            let hrd: String = atoms[..atoms.len() - 1]
                .iter()
                .filter_map(|a| match a {
                    EmixAtom::E(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            let st = self.dat.walk(hrd.as_bytes(), ROOT);
            if st >= 0 && self.dat.is_terminal(st) {
                let (lo, _c) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                for oi in o0..o1 {
                    let w = self.wlist_at(oi);
                    let t = self.word_text(w);
                    if self._flags[w as usize] == 0 || t.chars().count() != 2 {
                        continue;
                    }
                    head_words.push((t, Some(w as i64), hrd.clone()));
                    if head_words.len() >= 5 {
                        break;
                    }
                }
            }
        }
        // KBN 全替面（dhao→shao 少; 首键邻键替换成音节）
        let mut kbn2: Vec<(String, Option<i64>, String)> = Vec::new();
        if dom == "E2"
            && atoms.len() == 2
            && matches!(atoms[0], EmixAtom::I(_))
        {
            let a0 = match &atoms[0] {
                EmixAtom::I(s) => s.clone(),
                _ => unreachable!(),
            };
            let a1 = match &atoms[1] {
                EmixAtom::E(s) => s.clone(),
                EmixAtom::T(s, _) => s.clone(),
                EmixAtom::C(s) => s.clone(),
                EmixAtom::I(s) => s.clone(),
            };
            let mut nbs: Vec<u8> =
                kbn_neighbors(a0.as_bytes()[0]).collect();
            nbs.sort_unstable();
            for nb in nbs {
                let cand = format!("{}{}", nb as char, a1);
                if syls.contains(&cand) {
                    let ch = self.emix_char_col(&cand, 1);
                    if !ch.is_empty() {
                        kbn2.push((ch[0].clone(), None, cand));
                    }
                    break;
                }
            }
        }
        // KBN 词面（wonen: nen→men/ben → 我们/我笨）
        let mut kbn_w: Vec<(f64, String, u32, String)> = Vec::new();
        if dom == "E2"
            && atoms.len() == 2
            && matches!(atoms[0], EmixAtom::E(_))
            && matches!(atoms[1], EmixAtom::E(_))
        {
            let h0 = match &atoms[0] {
                EmixAtom::E(s) => s.clone(),
                _ => unreachable!(),
            };
            let t0 = match &atoms[1] {
                EmixAtom::E(s) => s.clone(),
                _ => unreachable!(),
            };
            let mut nbs: Vec<u8> = kbn_neighbors(t0.as_bytes()[0]).collect();
            nbs.sort_unstable();
            for nb in nbs {
                let v = format!("{}{}", nb as char, &t0[1..]);
                if syls.contains(&v) && v != t0 {
                    if let Some(hw) = self.emix_rd_head(&format!("{}{}", h0, v), false)
                    {
                        if hw.0.chars().count() == 2
                            && self._flags[hw.1 as usize] != 0
                        {
                            kbn_w.push((
                                0.0,
                                hw.0,
                                hw.1,
                                format!("{}'{}", h0, v),
                            ));
                        }
                    }
                }
            }
        }
        // 族数门（防劫持: dakn 大\u{1}可能 1 对面不接管——native 系
        // fuzzcomp 结构面; ≥3 面才整支接管）
        if fam_all.len() + epairs.len() < 3 {
            return Vec::new();
        }
        // 组装（native 序: pack 族 ≥3 → 族先行（dhao/tamenl）; 否则对偶面
        // 先行（mtians/wonen）; E3 拆字钉面恒先（xioala 小\u{1}辣 #0））
        let mut items: Vec<(String, Option<i64>, String)> = Vec::new();
        if let Some(pf) = &pin_face {
            if dom == "E3" {
                items.push((pf.clone(), None, String::new()));
            }
        }
        let pack_lead = fam_pack.len() >= 3;
        if pack_lead {
            items.extend(
                fam_pack.iter().map(|(_s, t, w, r)| {
                    (t.clone(), Some(*w as i64), r.clone())
                }),
            );
        }
        items.extend(epairs.iter().take(2).map(|(_b, f)| {
            (f.clone(), None, String::new())
        }));
        items.extend(head_words.iter().cloned());
        let done: HashSet<(String, Option<i64>, String)> =
            items.iter().cloned().collect();
        items.extend(
            fam_all
                .iter()
                .filter(|(_s, t, w, r)| {
                    !done.contains(&(t.clone(), Some(*w as i64), r.clone()))
                })
                .map(|(_s, t, w, r)| (t.clone(), Some(*w as i64), r.clone())),
        );
        for (_s, w3, w3id, r3) in kbn_w.iter().take(2) {
            items.push((w3.clone(), Some(*w3id as i64), r3.clone()));
        }
        if !kbn2.is_empty() {
            let pos = (EMIX_KBN_POS).min((items.len() - 1).max(1));
            items.insert(pos, kbn2[0].clone());
        }
        // 列: E3 输入头音节 → 首 e 头读音 → 复合声母正体 → 首声母
        //   （RD_PIN_COL 钉列优先——mtians m 栈 native 逐位先例）
        let mut col_items: Vec<(String, Option<i64>, String)> = Vec::new();
        let mut col_rd: Option<String> = None;
        if dom == "E3" {
            for l in (2..=(6.min(inp.len()))).rev() {
                if syls.contains(&inp[..l]) {
                    col_rd = Some(inp[..l].to_string());
                    break;
                }
            }
        } else if let Some(a) = atoms.first() {
            // 原循环各分支均 break —— 语义即只取首原子
            match a {
                EmixAtom::E(s) => col_rd = Some(s.clone()),
                EmixAtom::T(s, _) => col_rd = Some(s.clone()),
                EmixAtom::C(s) => col_rd = Some(format!("{}i", s)),
                EmixAtom::I(_) => col_rd = None,
            }
        }
        let stc = col_rd
            .as_ref()
            .map(|c| self.dat.walk(c.as_bytes(), ROOT))
            .unwrap_or(-1);
        let col_ok = col_rd.is_some()
            && stc >= 0
            && self.dat.is_terminal(stc);
        if col_ok {
            let cr = col_rd.as_ref().unwrap();
            for ch in self.emix_char_col(cr, EMIX_COL_N) {
                col_items.push((ch, None, cr.clone()));
            }
        } else if FB_INIS.contains(&inp.as_bytes()[0]) {
            if let Some(tbl) = rd_pin_col_get(&inp[..1]) {
                for (t9, _sc9) in tbl.iter().take(EMIX_COL_N) {
                    let w9 = self.panel_wid(t9);
                    if w9 >= 0 {
                        col_items.push((
                            (*t9).to_string(),
                            Some(w9),
                            String::new(),
                        ));
                    }
                }
            } else {
                for c9 in
                    self.fb_ini_column(inp.as_bytes()[0], EMIX_COL_N)
                {
                    col_items.push((c9.word, Some(c9.wid), String::new()));
                }
            }
        }
        self.emix_emit(&mut out, items, None);
        let mut tail_items = kbn2;
        tail_items.extend(col_items);
        self.emix_emit(&mut out, tail_items, None);
        out
    }

    /// G1 兜底面板（仅空面板态; py `_q_fallback` 镜像）
    pub(super) fn q_fallback(&self, inp: &str) -> Vec<Cand> {
        let mut out: Vec<Cand> = Vec::new();
        let b = inp.as_bytes();
        if inp.len() <= 2 {
            let up: String =
                inp.chars().map(|c| c.to_ascii_uppercase()).collect();
            let mut c = Cand::new(up.clone(), 0.0, Kind::Latin);
            c.reading = up;
            out.push(c);
        } else if b[0] == b'v' {
            // v 头英文态: 仅首字母双态（native vxhotjy n=2 = V,v）
            // 役N N5b(ref4): ①单字母连打（vvv）→ 首字直通面 v\u{1}VV
            //   （native vvv n=3 行 0, 读音 V#VV）——律 = 全同字母串首字
            //   直通+余部大写; ②[v][≥3 键段 1×KBN 替→音节][悬空单声母]
            //   → V\u{1}替读首字\u{1}补全首词（native vfeng n=4 行 0-1 =
            //   V人\u{1}个/V人\u{1}给, 读音 V,ren#ge/gei: f→r 邻域替 fen→ren
            //   × g→ge/gei 悬空补全; 替读择强 人>根>扽 列首词 lp 序）。
            //   vxhotjy 型无段可替 → 零面, 双态原样。
            let vfaces: Vec<Cand> =
                if inp.len() >= 3 && inp.bytes().all(|c| c == b[0]) {
                    self.adv_v_all_same(inp)
                } else if inp.len() >= 4 {
                    self.v_head_kbn_faces(&inp[1..])
                } else {
                    Vec::new()
                };
            out.extend(vfaces);
            for w in ["V", "v"] {
                let mut c = Cand::new(w.to_string(), 0.0, Kind::Latin);
                c.reading = "V".to_string();
                out.push(c);
            }
            return out;
        }
        let fs = self.fb_first_syl(inp);
        if let Some(fs) = &fs {
            if fs.len() < inp.len() {
                // 拼音前缀×字面字母混合（auqshwejna→阿U / ekwk→阿K;
                // 归纳门=前缀∈{a,e} 且尾首字母非 v, 见 py 注释）
                let r0 = b[fs.len()];
                if (fs.as_bytes()[0] == b'a' || fs.as_bytes()[0] == b'e')
                    && r0 != b'v'
                {
                    let uch = (r0 as char).to_ascii_uppercase();
                    let w = format!("阿{}", uch);
                    let mut c = Cand::new(w, 0.0, Kind::Latin);
                    c.reading = format!("{},{}", fs, uch);
                    out.push(c);
                }
            }
        }
        if inp.len() >= 4 {
            out.extend(self.q_fuzzcomp(inp, true));
        }
        // 役N W1(第五波): 律§7/§10 面族（native bfon=拜佛你/拜佛呢,
        // ekwk=可我/客卧）。族面在场 = 精确消费优先 → 旧链 EPYN 替换
        // sentence 面撤下（native bfon 无 不发那/补发你; ekwk 无
        // 哦看我/哦可我）, 字列/拉丁面不动。
        let law = self.fb_law_faces(inp);
        if !law.is_empty() {
            let mut o2: Vec<Cand> = out
                .into_iter()
                .filter(|c| c.kind != Kind::Sentence)
                .collect();
            o2.extend(law);
            out = o2;
        }
        // 役N W7O2 → 第八波(结构泛化): G1 读音键钉面（RD_PIN_FACES sentence
        // 类 × §3 吞并律——native 构造面 ncei 年费 = n→nian × cei→fei
        // KBN 头替 × ei exact, FB_FACE_CAP_G1=2 截除 → 补发; sentence 类
        // 组 0 内按钉分排, 字列不动; 表外读音零发射, 154 集 G1 锚
        // cv/gerxtq/ykvtbzuu/bfon/ekwk/evwxvqc/lhmclhakdw/auqshwejna
        // 零误伤——泛化前逐案验证）（py 同位镜像）
        for &(prd, faces) in RD_PIN_FACES.iter() {
            if !self.pin_rd_match(inp, prd) {
                continue;
            }
            for (t, sc, pk) in faces {
                if *pk != "sentence" {
                    continue; // pending/fcomp 面 = G2 域, G1 只发 sentence
                }
                let mut c = Cand::new(t.to_string(), *sc, Kind::Sentence);
                c.score = *sc;
                out.push(c);
            }
        }
        match fs {
            Some(fs) => {
                let mut seen_w: HashSet<u32> = HashSet::new();
                self.fb_seg_columns(&fs, &mut out, &mut seen_w, 10);
            }
            None => {
                // 役N tick64: 底排辅音/非拼音前缀 + 单声母 b (例如 zxcvb -> zxcv#ba = zxcv\u{1}吧)
                // native zxcvb 行 0 逐位: zxcv 吧 ZXCV#ba 紧随其后为 z 声母字列 在/这/做/再...
                if inp.len() >= 3
                    && inp.ends_with('b')
                    && !inp[..inp.len() - 1]
                        .chars()
                        .any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
                {
                    let head = &inp[..inp.len() - 1];
                    let mut c = Cand::new(
                        format!("{}\u{1}吧", head),
                        0.0,
                        Kind::Sentence,
                    );
                    c.reading = format!("{}#ba", head.to_ascii_uppercase());
                    c.score = 2.0;
                    c.score_bias = 2.0;
                    out.push(c);
                }
                if FB_INIS.contains(&b[0]) {
                    out.extend(self.fb_ini_column(b[0], 10));
                }
            }
        }
        out
    }

    /// G5 大写三态（态2/态3; 态1 全大写已在 query 早退; py `_q_upper` 镜像）
    pub(super) fn q_upper(&self, inp: &str) -> Vec<Cand> {
        if !inp.is_ascii() {
            // 防非 ASCII 字节切片越界; native 大写查询为纯 ASCII 字母
            let mut c = Cand::new(inp.to_string(), 0.0, Kind::Latin);
            c.reading = inp.to_string();
            return vec![c];
        }
        let low: String = inp
            .chars()
            .flat_map(|c| c.to_lowercase())
            .filter(|c| ('a'..='z').contains(c))
            .collect();
        let fs = self.fb_first_syl(&low);
        if inp.as_bytes()[0].is_ascii_uppercase() {
            // 态3: 首音节 ≤2 键 → 字母词段 1 条; ≥3 键 → 空
            return match fs {
                Some(fs) if fs.len() <= 2 => {
                    let tail = &inp[fs.len()..];
                    let mut w = low[..fs.len()].to_string();
                    if !tail.is_empty() {
                        let mut tc = tail.chars();
                        w.push('\u{1}');
                        w.extend(tc.next().unwrap().to_uppercase());
                        w.push_str(tc.as_str());
                    }
                    let mut c = Cand::new(w.clone(), 0.0, Kind::Latin);
                    c.reading = w;
                    vec![c]
                }
                _ => Vec::new(),
            };
        }
        // 态2: 混合 你\x01HAO + 前缀字列 + 前缀字母回显（native niHAO n=96）
        let Some(fs) = fs else {
            let mut c = Cand::new(inp.to_string(), 0.0, Kind::Latin);
            c.reading = inp.to_string();
            return vec![c];
        };
        let mut out: Vec<Cand> = Vec::new();
        let mut col: Vec<Cand> = Vec::new();
        let st = self.dat.walk(fs.as_bytes(), ROOT);
        if st >= 0 && self.dat.is_terminal(st) {
            let (lo, _cnt) = self.dat.range(st);
            let (o0, o1) = self.reading_slice(lo);
            let mut colw: Vec<(f64, u32, usize)> = Vec::new();
            for oi in o0..o1.min(o0 + 48) {
                let w = self.wlist_at(oi);
                colw.push((self.word_logp(w), w, oi));
            }
            colw.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for (_lp, w, oi) in colw.into_iter().take(12) {
                col.push(self.mk(
                    w,
                    lo as i64,
                    Kind::Comp,
                    0,
                    None,
                    0.0,
                    self.slot_fv(oi),
                ));
            }
        }
        let tail = &inp[fs.len()..];
        if !col.is_empty() && !tail.is_empty() {
            let mut c = Cand::new(
                format!("{}\u{1}{}", col[0].word, tail),
                col[0].logp,
                Kind::Comp,
            );
            c.reading = format!("{},{}", fs, tail);
            c.score = col[0].logp;
            out.push(c);
        }
        out.extend(col);
        let mut c = Cand::new(fs.clone(), 0.0, Kind::Latin);
        c.reading = fs;
        out.push(c);
        out
    }

    pub(super) fn q_correct(&self, inp: &str) -> Vec<Cand> {
        let mut cors = self.editpinyin(inp);
        let mut base = inp;
        if cors.is_empty() {
            // 最长合法音节前缀 → 尾段查纠错
            for i in (1..=inp.len()).rev() {
                if self.seg.segmentable(&inp[..i]) {
                    base = &inp[i..];
                    cors = self.editpinyin(base);
                    break;
                }
            }
        }
        if cors.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for ids in &cors {
            let key = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect::<String>();
            // 件 4: corrector 伪边收敛 —— ve→的/了/呢、vyuan→原来/原因、
            // vv→女/旅/率 均为 bin 表直通伪边（native 对这些走字母直通,
            // agentL #19/#22/#27）。同过 epyn_edge_ok 形状闸; 替换边再查
            // KBN 实表（KBN v 行 {b,c,g}, ve/vv 的 v→d/n/l 全拦）
            if !Self::epyn_edge_ok(base, &key) {
                continue;
            }
            if key.len() == base.len() {
                let kb = key.as_bytes();
                let bb = base.as_bytes();
                let mut diffs = Vec::new();
                for k in 0..bb.len() {
                    if bb[k] != kb[k] {
                        diffs.push(k);
                    }
                }
                if diffs.len() == 1
                    && !kbn_neighbors(bb[diffs[0]])
                        .any(|nb| nb == kb[diffs[0]])
                {
                    continue;
                }
            }
            let st = self.dat.walk(key.as_bytes(), ROOT);
            if st < 0 {
                continue;
            }
            let (lo, cnt) = self.dat.range(st);
            let pen = -Self::edit_cost(base.as_bytes(), key.as_bytes());
            let mut got = self.range_candidates(lo, cnt, 32);
            if got.is_empty() {
                let (o0, o1) = self.reading_slice(lo);
                for oi in o0..o1.min(o0 + 8) {
                    let w = self.wlist_at(oi);
                    got.push((w, lo, self.fv_of(w, oi)));
                }
            }
            for (w, r, fv) in got.into_iter().take(8) {
                out.push(self.mk(
                    w,
                    r as i64,
                    Kind::Correct,
                    0,
                    None,
                    pen,
                    Some(fv as u16),
                ));
            }
        }
        out
    }

    pub(super) fn q_assoc(&self, ci: &CtxIds) -> Vec<Cand> {
        let wid = match ci.wid {
            Some(w) => w,
            // 英文 ctx 无词典 wid（open 词典无英文词域）——boost 域放行
            // c4g 词对联想（c4g 用 text 查 pool 不需要 wid; 忠实链
            // golden 18 个 ascii ctx 案零变——boost None 恒早退）
            None if !ci.text.is_empty()
                && ci.text.is_ascii()
                && self.boost.is_some() =>
            {
                return self.q_c4g(&ci.text);
            }
            None => return Vec::new(),
        };
        let mut big = self.bigram_after(wid);
        if big.is_empty() && ci.text.chars().count() > 1 {
            // 整词无 bigram → 尾词回退（你好世界→世界）
            for l in (1..=ci.text.char_indices().count().min(4)).rev() {
                let tail = ctx_tail(&ci.text, l);
                if let Some(w) = self.word_id(tail) {
                    big = self.bigram_after(w);
                    if !big.is_empty() {
                        break;
                    }
                }
            }
        }
        let mut out = Vec::new();
        for &(w, lp) in &big.ord {
            let mut c = self.mk(w, -1, Kind::Assoc, 0, Some(lp), 0.0, None);
            c.reading = String::new();
            out.push(c);
        }
        // 英文上下文 → c4g 词对
        if !ci.text.is_empty() && ci.text.is_ascii() {
            out.extend(self.q_c4g(&ci.text));
        }
        out
    }

    fn q_c4g(&self, word: &str) -> Vec<Cand> {
        let nw = self.c4g_pool.n;
        let (mut a, mut b) = (0usize, nw);
        while a < b {
            let mid = (a + b) / 2;
            let k = self.c4g_pool.get(mid);
            if k == word {
                a = mid;
                break;
            }
            if k.as_str() < word {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        if a >= nw || self.c4g_pool.get(a) != word {
            return Vec::new();
        }
        // searchsorted left over c4g_w1
        let npair = self.c4g_npair;
        let (mut lo, mut hi) = (0usize, npair);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.c4g_w1.get(mid) < a as u32 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let mut out = Vec::new();
        let mut i = lo;
        while i < npair && self.c4g_w1.get(i) == a as u32 {
            let w2i = self.c4g_w2.get(i);
            let mut c = Cand::new(
                self.c4g_pool.get(w2i as usize),
                -12.0,
                Kind::C4G,
            );
            c.lm = Some(val_to_log10p(self.c4g_val.get(i)));
            out.push(c);
            i += 1;
        }
        out
    }

    /// 役N tick70（p60 联想役）: 裸声母单键面板 — SOLO 钉序纯单字列
    /// （py `_q_initial_panel` 镜像）。native 三律: ① 与 ctx 无关
    /// （wxsel after_sel/mid 双采样 d/s/w/x 四组选词后 ≡ 无 ctx 逐位同）;
    /// ② 纯单字 25-26 连, hot 词组不进面; ③ 列序 = 钉序（列序之谜同源）。
    /// score = −0.001i 直接终分（钉序, 绕过打分层重排）。仅单键消费
    /// （query 门 len==1 × g1_col_solo 命中; g1_col 多键回退面板不受影响）。
    pub(super) fn q_initial_panel(&self, tbl: &[&str], topk: usize) -> Vec<Cand> {
        let mut cands: Vec<Cand> = Vec::new();
        for (i, t) in tbl.iter().enumerate() {
            let w = self.panel_wid(t);
            if w < 0 {
                // tick70b/71: 词典缺字面（u/U 拉丁对 + 409 音节钉表
                // 缺字/缺词）→ mk_lit 字面候选（W3 忠国/井栏 先例）
                let mut c = self.mk_lit(t, -1);
                c.kind = Kind::Fbcol;
                c.score = -0.001 * i as f64;
                c.score_bias = -0.001 * i as f64;
                cands.push(c);
                continue;
            }
            let wid = w as u32;
            let rid = self.w2r.get(wid as usize) as i64 - 1;
            let mut c = self.mk(wid, rid, Kind::Fbcol, 0, None, 0.0, None);
            c.score = -0.001 * i as f64;
            c.score_bias = -0.001 * i as f64;
            cands.push(c);
        }
        // tick71: 字面候选（wid=-1）按词面去重（py _dedupe 同律——
        // HashSet<wid> 会把全部字面塌缩成一条, suan 素安/宿安 实证）
        let mut seen: HashSet<(i64, String)> = HashSet::new();
        cands.retain(|c| {
            let key = if c.wid >= 0 {
                (c.wid, String::new())
            } else {
                (-1, c.word.clone())
            };
            seen.insert(key)
        });
        cands.truncate(topk);
        cands
    }
}

/// §3 吞并律 memo 递归体（py `_pin_rd_match._m` 逐语义镜像）:
/// 输入位 pos / 面音节序 si; py 切片越界取短串（≠r 即不中）→ 守卫同义。
fn pin_swallow(
    b: &[u8],
    syls: &[&str],
    memo: &mut HashMap<(usize, usize), bool>,
    pos: usize,
    si: usize,
) -> bool {
    if let Some(&hit) = memo.get(&(pos, si)) {
        return hit;
    }
    let n = b.len();
    let res = if pos == n {
        si == syls.len()
    } else if si >= syls.len() {
        false
    } else {
        let r = syls[si].as_bytes();
        let mut v = false;
        // exact
        if n >= pos + r.len()
            && &b[pos..pos + r.len()] == r
            && pin_swallow(b, syls, memo, pos + r.len(), si + 1)
        {
            v = true;
        }
        // KBN 1 替（等长段恰 1 字母差且属 KBN 邻键）
        if !v && n >= pos + r.len() {
            let ch = &b[pos..pos + r.len()];
            let mut diff: Option<usize> = None;
            let mut ndiff = 0usize;
            for t in 0..r.len() {
                if ch[t] != r[t] {
                    diff = Some(t);
                    ndiff += 1;
                }
            }
            if ndiff == 1
                && kbn_neighbors(r[diff.unwrap()]).any(|nb| nb == ch[diff.unwrap()])
                && pin_swallow(b, syls, memo, pos + r.len(), si + 1)
            {
                v = true;
            }
        }
        // 悬空声母（+连续吞并: 后续散段字母 ∈ 面音节余部）
        if !v && r[0] == b[pos] {
            if pin_swallow(b, syls, memo, pos + 1, si + 1) {
                v = true;
            } else {
                let mut k = pos + 1;
                while k < n && k - pos < r.len() && r[1..].contains(&b[k]) {
                    if pin_swallow(b, syls, memo, k + 1, si + 1) {
                        v = true;
                        break;
                    }
                    k += 1;
                }
            }
        }
        // 纯元音面音节零消费越段
        if !v
            && (r == b"a" || r == b"e" || r == b"o")
            && pin_swallow(b, syls, memo, pos, si + 1)
        {
            v = true;
        }
        v
    };
    memo.insert((pos, si), res);
    res
}
