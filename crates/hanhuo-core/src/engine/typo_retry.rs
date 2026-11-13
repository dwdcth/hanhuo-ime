//! B2.8 死音节变体枚举（长句容错, ADR-002 超越线）
//!
//! 现状: 整句 dp 通道要求全消耗切分; 输入含非法音节（ging/yigi 类）
//! 时 best[] 传播中途死亡 → 整句路径灭 → 退化 fbcol 单字。
//! 本模块在死段窗口内枚举变体, 由调用方对变体重跑 dp——**词路径分数选向**
//! （青花>汀花 由 LM 决定, 非规则表决定; kbn 键位邻接表不含 g→q,
//! 方向无法先验）。
//!
//! v1.1 变体族: 1-编辑 = 替换/插入/删除 三族同构; 2-编辑 = 替换×插入
//! 组合（两编辑位距离 ≤2, 邻近双错如「jging」= 漏 i + g 错拼 q）。
//! 组合只做 sub×ins——窗口²×字母² 全组合会爆; del×ins/sub×del 类
//! 双错语料暂缺, 不做。
//!
//! 成本实测（61 字节 #1 句, wxq_dict.bin）: dp_reachable 1.8µs/次;
//! 三族 1-编辑过滤 ~0.7ms + sub×ins 组合过滤 ~30ms（23,400 串）——
//! 远低于 200ms 剪枝线, 无需死段起点收窄。变体全查询热缓存 ~1.5ms/个
//! （v1 注释的 38ms 是冷缓存口径）。
//!
//! 排序律: 句通道分数对同音头结构性并列（实测 #1 修正句与 11 个同骨架
//! 变体同为 +4.400）, 稳定排序下列序=枚举序。枚举序取「编辑局部性」:
//! 1-编辑先于 2-编辑; 组合内 |sub位−ins位| 升序、插入字母升序、替换
//! 字母升序——邻近双错（一次手滑）先于远距组合, 无 LM 信号时的确定性
//! 先验。
//!
//! 分层: 纯字节层（dead_window/三族/组合生成器）免数据可测;
//! 编排层（Engine::typo_variants）需 DAT, 真机 #[ignore] 测试。

//! B2.9 模糊音/前后鼻音变体纠正（jin↔jing 类合法音节错拼, 2026-11-09）:
//! 与 B2.8 互补的触发域——输入**合法**（qi|jin|ba|mai 全消耗切分完美）,
//! 错在「切了但切错」。挂点在 query_typo_retry 健康分支: 原查询照常,
//! 门控通过后对鼻音/平翘舌变位串重查合并。设计/门控选型/校准数据见
//! docs/设计-模糊音变体.md。
//!
//! 分层: 纯字节层（nasal_edits/sib_edits/fuzz_gate_auto）免数据可测;
//! 编排层（Engine::fuzz_retry）需 DAT, 真机 #[ignore] 测试。

/// 最长音节字节数（zhuang/chuang = 6）
const MAX_SYL: usize = 6;

/// 2-编辑（替换×插入）两编辑位距离上界
const PAIR_DIST: usize = 2;

/// 死段替换窗口: reachable[i] = 「前 i 字节可全消耗切分」。
/// 返回 None = 全消耗（无死段）; Some((lo, hi)) = 变体候选窗口
/// （字节下标左闭右开, 覆盖最后可达音节 + 死段头部）。
pub(crate) fn dead_window(reachable: &[bool]) -> Option<(usize, usize)> {
    let n = reachable.len().saturating_sub(1); // 位图长 n+1
    if reachable[n] {
        return None;
    }
    // r = 最大可达前缀长度
    let r = (0..=n).rev().find(|&i| reachable[i])?;
    // 窗口: 最后可达音节可能整体错（吞错方向）→ 回退 MAX_SYL;
    // 死段头探 MAX_SYL+2——死段本尊可能是整音节错字母（wenzhon 的
    // 修法在 o→u/n 后插 g, 位于 r+2..r+3——旧 r+2 窗实测枚举恒空,
    // 长句 news 案 6/7 零救援即此）; 上界 = 最长音节 zhuang/chuang 6
    // + 2 余孽, 与回退同律对称
    let lo = r.saturating_sub(MAX_SYL);
    let hi = (r + MAX_SYL + 2).min(n);
    (lo < hi).then_some((lo, hi))
}

/// 窗口内 1-替换生成器: (变体串, 替换位, 新字节)。
/// 跳过恒等替换与非字母位; 输入约定纯 ASCII 拼音串。
pub(crate) fn substitutions(
    inp: &str, lo: usize, hi: usize,
) -> impl Iterator<Item = (String, usize, u8)> + '_ {
    let b = inp.as_bytes();
    (lo..hi.min(b.len()))
        .filter(|&i| b[i].is_ascii_lowercase())
        .flat_map(move |i| {
            (b'a'..=b'z').filter(move |&c| c != b[i]).map(move |c| {
                let mut v = b.to_vec();
                v[i] = c;
                (String::from_utf8(v).expect("纯 ASCII 替换"), i, c)
            })
        })
}

/// 窗口内 1-插入生成器: (变体串, 插入位, 新字节)。插入位 = 插在前
/// 该下标字节之前, 取 lo..=hi（窗口内的全部间隙, 含右缘）。
pub(crate) fn insertions(
    inp: &str, lo: usize, hi: usize,
) -> impl Iterator<Item = (String, usize, u8)> + '_ {
    let b = inp.as_bytes();
    (lo..=hi.min(b.len())).flat_map(move |i| {
        (b'a'..=b'z').map(move |c| {
            let mut v = b[..i].to_vec();
            v.push(c);
            v.extend_from_slice(&b[i..]);
            (String::from_utf8(v).expect("纯 ASCII 插入"), i, c)
        })
    })
}

/// 窗口内 1-删除生成器: (变体串, 删除位, 被删字节)。
/// 非小写字母位不删（撇号等结构字节, 防御性跳过）。
pub(crate) fn deletions(
    inp: &str, lo: usize, hi: usize,
) -> impl Iterator<Item = (String, usize, u8)> + '_ {
    let b = inp.as_bytes();
    (lo..hi.min(b.len()))
        .filter(|&i| b[i].is_ascii_lowercase())
        .map(move |i| {
            let mut v = b.to_vec();
            let old = v.remove(i);
            (String::from_utf8(v).expect("纯 ASCII 删除"), i, old)
        })
}

/// 窗口内 2-编辑组合生成器（替换×插入, |两位|≤PAIR_DIST）:
/// (变体串, 替换位, 替换字节, 插入位, 插入字节)。插入位为原串坐标
/// （替换保位, 先替换后插入坐标一致）。
/// 枚举序 = 编辑局部性: 替换位升序 → 距离升序 → 插入位升序 →
/// 插入字母升序 → 替换字母升序。
pub(crate) fn sub_insert_pairs(
    inp: &str, lo: usize, hi: usize,
) -> impl Iterator<Item = (String, usize, u8, usize, u8)> + '_ {
    let b = inp.as_bytes();
    let hi = hi.min(b.len());
    (lo..hi)
        .filter(|&i| b[i].is_ascii_lowercase())
        .flat_map(move |i| {
            (0..=PAIR_DIST).flat_map(move |dist| {
                // 同距离下插入位升序; dist=0 左右重合只出一回
                let js: Vec<usize> = if dist == 0 {
                    vec![i]
                } else {
                    [i as isize - dist as isize, i as isize + dist as isize]
                        .into_iter()
                        .filter(|&j| j >= lo as isize && j <= hi as isize)
                        .map(|j| j as usize)
                        .collect()
                };
                js.into_iter().flat_map(move |j| {
                    (b'a'..=b'z').flat_map(move |d| {
                        (b'a'..=b'z').filter(move |&c| c != b[i]).map(move |c| {
                            let mut v = b[..i].to_vec();
                            v.push(c);
                            v.extend_from_slice(&b[i + 1..]);
                            let mut w = v[..j].to_vec();
                            w.push(d);
                            w.extend_from_slice(&v[j..]);
                            (String::from_utf8(w).expect("纯 ASCII 组合"), i, c, j, d)
                        })
                    })
                })
            })
        })
}

use crate::engine::{Cand, Kind};
use super::boost;
use crate::Engine;
use crate::dat::ROOT;

// ===================== B2.9 模糊音变体（n↔ng / z⇄zh…） =====================

/// 门控模式（u8 原子; Engine::set_fuzz_mode 设置——真 IME 的「模糊音」
/// 设置位。进程级静态而非 Engine 字段: typo_retry.rs 单文件领地内自治,
/// 引擎单线程契约与既有 RefCell 缓存同律）。
pub const FUZZ_AUTO: u8 = 0; // 默认: 结构质量门
pub const FUZZ_ON: u8 = 1; // 强制开（跳过质量门, 长句也救）
pub const FUZZ_OFF: u8 = 2; // 强制关（纯直通）
static FUZZ_MODE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(FUZZ_AUTO);

/// Auto 门短输入上限（字节）。= 词/成语域上界: 验收探针 qijinbamai=10
/// 在内; 52 句正确组最短 20 字节（明天…/晚上…档）全组在门外——长句域
/// logp 尺度实证不可分（正确组 per-syl −0.68..−0.11 与错拼组合法段
/// −0.71..−0.14 完全重叠, 见设计文档校准表）。
const FUZZ_SHORT_MAX: usize = 16;

/// 变体数上限（防御位; ≤16 字节输入族位天然 ≤16, force-on 长句截断）
const FUZZ_MAX_VARIANTS: usize = 24;

/// 一档·鼻音变位原始串枚举（字节级; 合法性由调用方 dp_reachable 过滤,
/// 如 jinngian/ngan 类非法切分自然出局）。每个「n 不后随 g」位之后试插
/// g（jin→jing 方向）, 每个「ng」试删 g（jing→jin 方向）。枚举序 = 位序
/// （插入族先, 删除族后）。串级去重。
pub(crate) fn nasal_edits(inp: &str) -> Vec<String> {
    let b = inp.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut push = |v: String| {
        if !out.contains(&v) {
            out.push(v);
        }
    };
    for i in 0..b.len() {
        if !b[i].is_ascii_lowercase() {
            continue;
        }
        // 插入 g: n 位且下一字节非 g（ng 位插入必得 ngg 死串, 免枚举）
        if b[i] == b'n' && b.get(i + 1) != Some(&b'g') {
            let mut v = b[..=i].to_vec();
            v.push(b'g');
            v.extend_from_slice(&b[i + 1..]);
            push(String::from_utf8(v).expect("纯 ASCII 插入"));
        }
        // 删除 g: ng 之 g（n 结尾音节的 g 删后可能仍合法, 如 jing→jin）
        if b[i] == b'g' && i > 0 && b[i - 1] == b'n' {
            let mut v = b.to_vec();
            v.remove(i);
            push(String::from_utf8(v).expect("纯 ASCII 删除"));
        }
    }
    out
}

/// 二档·平翘舌变位原始串枚举（z⇄zh, c⇄ch, s⇄sh; 同构律: z/c/s 位且下一
/// 字节非 h 试插 h, zh/ch/sh 试删 h）。一档零产出才入场（分档控制成本
/// 与误伤面）。l/n 族弃做: 两字母皆全位置合法声母, 合法性过滤零鉴别力,
/// 枚举量翻倍且歧义高（林/邻 vs 您——无探针案支撑）。
pub(crate) fn sib_edits(inp: &str) -> Vec<String> {
    let b = inp.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut push = |v: String| {
        if !out.contains(&v) {
            out.push(v);
        }
    };
    for i in 0..b.len() {
        if !b[i].is_ascii_lowercase() {
            continue;
        }
        if matches!(b[i], b'z' | b'c' | b's') && b.get(i + 1) != Some(&b'h') {
            let mut v = b[..=i].to_vec();
            v.push(b'h');
            v.extend_from_slice(&b[i + 1..]);
            push(String::from_utf8(v).expect("纯 ASCII 插入"));
        }
        if b[i] == b'h' && i > 0 && matches!(b[i - 1], b'z' | b'c' | b's') {
            let mut v = b.to_vec();
            v.remove(i);
            push(String::from_utf8(v).expect("纯 ASCII 删除"));
        }
    }
    out
}

/// 词典词覆盖类: 引擎已给出「整词答案」的 kind 族（exact 直查/complete
/// 补全/user 用户词/correct 原生纠错）。合并律里覆盖类 > 缝合类
/// （sentence/path…）——变体输入的 exact 词命中（奇经八脉）天然压过
/// 原输入的缝合句（企金吧买）, 与两个面板各自的列首钉分 5.0 无关。
fn cover_class(k: Kind) -> u8 {
    matches!(k, Kind::Exact | Kind::Complete | Kind::User | Kind::Correct)
        .then_some(1)
        .unwrap_or(0)
}

/// 全消耗候选判定（B2.8/B2.9 同律: Sentence kind || reading 去撇号==键）
fn is_full(c: &Cand, key: &str) -> bool {
    c.kind == Kind::Sentence || c.reading.replace('\'', "") == key
}

/// B2.9 Auto 结构质量门: 短输入 + 全消耗最佳候选**无词典词覆盖**才入场。
/// 校准实证（52 句正确组 + 探针组, 设计文档表 2/3）: 词典词覆盖 ⇒ 健康
/// （jinnian→今年 exact / nihaoshijie→你好世界 exact, 全组免触发）;
/// 无覆盖（bf=None 或 Sentence 缝合）⇒ 错拼嫌疑域（qijinbamai 企金吧买
/// sentence / xinweng 无 full）。logp 阈值案被数据否定: 健康 exact 词
/// logp 地板 −6.41（琵琶行）与错拼侧经融 −6.74 分离带仅 0.33, 且正确
/// 长句组与错拼组 logp 完全互穿——结构信号（有无整词覆盖）全程可分。
fn fuzz_gate_auto(inp_len: usize, bf_kind: Option<Kind>) -> bool {
    inp_len <= FUZZ_SHORT_MAX && cover_class(bf_kind.unwrap_or(Kind::Sentence)) == 0
}

/// B2.8 变体上限。1-编辑 8（v1 行为保持: 可修句的变体集中在窗口头几
/// 个）; 2-编辑组合 12。全查询热缓存 ~1.5ms/个（61 字节句实测）,
/// 最坏 8+12=20 个 ≈ 30ms。
const MAX_VARIANTS_1: usize = 8;
const MAX_VARIANTS_2: usize = 12;
/// 变体罚（每编辑 0.3）: 修正句与原生句分数可比（句分 −2.x 罚后仍压
/// fbcol 单字 −3.x）; 2-编辑 0.6 < 1-编辑 0.3, 编辑数先验进分。
const TYPO_PENALTY: f64 = 0.3;

/// 通过全消耗过滤的变体（edits = 编辑数, 供逐编辑罚分）
pub(crate) struct TypoVariant {
    pub s: String,
    pub edits: u8,
}

impl Engine {
    /// B2.8 句查询容错入口（独立入口模式, 不动 query() 本体——与辅助码
    /// aux.rs 设计同哲学; boost/userdict 在 query() 内部照常生效）。
    ///
    /// 健康输入（全消耗可达）零开销直通; 死段输入枚举变体（1-编辑三族
    /// + 2-编辑 sub×ins 邻近组合）逐个 query(dp) 取全消耗句候选, 按
    /// 编辑数罚分合并——同音方向由 LM 选（qing 变体的 青花 词路径自然
    /// 压过 ting 变体）。变体零句产出时回退原查询（不劣化现状）。
    ///
    /// B2.9: 健康分支追加模糊音变体重试（fuzz_retry）——合法输入的
    /// 「切了但切错」（jin↔jing）在此域救, 门控/合并律见模块注释。
    pub fn query_typo_retry(
        &self, inp: &str, ctx: Option<&str>, topk: usize, nn: bool, ltr: bool,
    ) -> Vec<Cand> {
        let mut __o = self.query_typo_retry_awb(inp, ctx, topk, nn, ltr);
        crate::engine::awb::filter_out(&mut __o);
        self.ctx_colloc_pin(ctx, &mut __o);
        return __o;
    }

    fn query_typo_retry_awb(
        &self, inp: &str, ctx: Option<&str>, topk: usize, nn: bool, ltr: bool,
    ) -> Vec<Cand> {
        // T9 九宫门前分叉（设计-T9九宫输入 §5-P1②: 数字 1-5 被调号
        // 征用、6-9 被白名单挡——T9 域键串须在 strip_tones 之前接管;
        /// query_inner 白名单前已同律分叉, 此处罩住默认 typo_retry 通道）
        if self.t9
            && !inp.is_empty()
            && inp.bytes().all(|k| (b'2'..=b'9').contains(&k))
        {
            let mut out = self.q_t9_typo(inp);
            super::awb::filter_out(&mut out);
            return out;
        }

        // 空输入 × ctx = 联想模式（assoc/c4g 域）——fuzz 重试不适用,
        // 直查透传（此前空输入早退把 ctx 联想链整条闷死——c4g 触发
        // 链 2026-11-20 修: the→new/world… 需此透传）
        if inp.is_empty() {
            let mut out = self.query(inp, ctx, false, topk, nn, ltr, false);
            super::awb::filter_out(&mut out);
            return out;
        }

        // 声调筛选语法层（设计-声调筛选 §2）: 重试编排前剥离调号——
        // dp 死窗/变体域按净输入（带数字串上 dp 恒死, 全链误入变体域）
        let has_tone = inp.bytes().any(|b| b.is_ascii_digit());
        let mut out = if has_tone {
            let (clean, tone_pos) = crate::engine::strip_tones(inp);
            let mut out = self.query_typo_retry_inner(
                &clean, ctx, topk, nn, ltr,
            );
            if let Some(b) = &self.boost {
                if b.tone_filter && !tone_pos.is_empty() {
                    boost::tone_filter(
                        self, &mut out, &clean, &tone_pos,
                    );
                }
            }
            out
        } else {
            self.query_typo_retry_inner(inp, ctx, topk, nn, ltr)
        };
        out.truncate(topk);
        out
    }

    fn query_typo_retry_inner(
        &self, inp: &str, ctx: Option<&str>, topk: usize, nn: bool, ltr: bool,
    ) -> Vec<Cand> {
        let reach = self.dp_reachable(inp);
        if reach[reach.len() - 1] {
            let base = self.query(inp, ctx, false, topk, nn, ltr, true);
            return self.fuzz_retry(inp, ctx, topk, nn, ltr, base);
        }
        // 混合输入域（设计-混合输入 §2.4）: 纯全拼不可达但混合切分可达
        // → 原链跑原始输入（mixed_input 挂点在主链内注入混合面）。
        // 声母死段是有意输入, 不走 typo 变体改写（q→a 等会覆写声母意图,
        // 变体面挤掉原始面——实测 jintiantq 变体面「今天昂起」污染面板）。
        // dp=false: 混合输入按定义 dp 不可达, 免死输入 dp 慢通道病理。
        if self.boost.as_ref().is_some_and(|b| b.mixed_input)
            && boost::mixed_segs(self, inp).is_some()
        {
            return self.query(inp, ctx, false, topk, nn, ltr, false);
        }
        let mut pool: Vec<Cand> = Vec::new();
        for tv in self.typo_variants(inp, MAX_VARIANTS_1, MAX_VARIANTS_2) {
            for mut c in self.query(&tv.s, ctx, false, topk, nn, ltr, true) {
                // 全消耗判定（B0 同律: Sentence kind || reading 去撇号==输入）
                let full = c.kind == Kind::Sentence
                    || c.reading.replace('\'', "") == tv.s;
                if !full || pool.iter().any(|x| x.word == c.word) {
                    continue;
                }
                c.score -= TYPO_PENALTY * tv.edits as f64;
                pool.push(c);
            }
        }
        if pool.is_empty() {
            // 回退: 不可修（≥3 编辑/超窗）→ 原查询不进 dp 慢通道。
            // dp 剥离（D 代理定位, 2026-11-09）: 死输入上 dp=true 是秒级
            // 病理（61 字节死输入实测 2.2s, 最差观测 14.6s; dp=false 45ms,
            // 快 49×）。面板差异属垃圾类间取舍（dp=false 多真词 fuzz 候选
            // 充满/冲满, dp=true 多罕字列 沌/铳/蟲——前者不劣）; 回退只在
            // 变体池空=不可修死输入时触发, 修复面窄。
            return self.query(inp, ctx, false, topk, nn, ltr, false);
        }
        // B2.7c 变体域简拼接管（boost 域产品增强）: 不可切分输入恰为
        // 简拼表键（sm→什么）时, 变体面（邻键改写 s→a/e/o）覆写简拼
        // 意图——原始 query(inp) 本体在此分支永不返回, 主管线 B2.7
        // 注入段收不到。简拼整键命中=强意图信号, B2.7 律置顶注入到
        // 变体池（音节意图门: 全串 terminal 如 ma/ke 不接管——音节
        // 优先, ma→母爱 是误伤）。忠实门（无 boost）零行为。
        if self.boost.as_ref().is_some_and(|b| b.jianpin_inject) {
            let st = self.dat.walk(inp.as_bytes(), ROOT);
            if !(st >= 0 && self.dat.is_terminal(st)) {
                let hits = self.jianpin_key_hits(inp);
                if !hits.is_empty() {
                    let top = pool.first().map(|c| c.score).unwrap_or(0.0);
                    let mut injected = false;
                    for (i, h) in hits.into_iter().take(6).enumerate() {
                        let target = top + 0.5 - 0.01 * i as f64;
                        match pool.iter_mut().find(|x| x.word == h.word) {
                            Some(x) => {
                                if x.score < target {
                                    x.score = target;
                                    injected = true;
                                }
                            }
                            None => {
                                let mut h = h;
                                h.score = target;
                                pool.push(h);
                                injected = true;
                            }
                        }
                    }
                    if injected {
                        pool.sort_by(|a, b| {
                            b.score.partial_cmp(&a.score).unwrap_or(
                                std::cmp::Ordering::Equal,
                            )
                        });
                    }
                }
            }
        }
        pool.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        pool.truncate(topk);
        pool
    }
}

impl Engine {
    /// best[] 可达传播（dp_path 前向同构: 无打分/回溯, 只判可达）。
    /// reach[i] = 前 i 字节可全消耗切成合法音节序列。
    fn dp_reachable(&self, inp: &str) -> Vec<bool> {
        let b = inp.as_bytes();
        let n = b.len();
        let mut reach = vec![false; n + 1];
        reach[0] = true;
        for i in 0..n {
            if !reach[i] {
                continue;
            }
            for &(depth, st) in &self.dat.iter_terminal(&b[i..]) {
                // 与 dp_path 同律: 空读音槽的死终端不算可达
                let (lo, _cnt) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                if o1 > o0 {
                    reach[i + depth] = true;
                }
            }
        }
        reach
    }

    /// B2.8 v1.1 变体枚举: 死段窗口内 1-编辑三族（位序: 替换→插入→
    /// 删除）+ 2-编辑 sub×ins 组合（编辑局部性序）, 全消耗过滤 +
    /// 字符串去重（组合内 dist-1 与 dist-0 常产出同串, 如 jibing 的
    /// 两种编辑几何）, 分层截断 max1/max2。
    /// 健康输入（全消耗）返回空——调用方只在整句路径死亡时入场。
    pub(crate) fn typo_variants(
        &self, inp: &str, max1: usize, max2: usize,
    ) -> Vec<TypoVariant> {
        let reachable = self.dp_reachable(inp);
        let Some((lo, hi)) = dead_window(&reachable) else {
            return Vec::new();
        };
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        let full = |v: &str| {
            let r = self.dp_reachable(v);
            r[r.len() - 1]
        };
        // ---- 1-编辑: KBN 家族优先, 其后窗口位序 替换→插入→删除 ----
        // 词频层是原厂四段量化——wenzhan/wenzhong 类变体句常并列同分,
        // 合并池 stable sort 下枚举序即面板序; 鼻音 n↔ng（jin↔jing）与
        // 平翘舌 z/c/s↔zh/ch/sh 是错拼最高频族, 前置 = 正解变体先入池
        // （B2.9 同族设计在死段域的前置版; 家族窄, 不挤 max1 席位）。
        let mut one: Vec<(String, u8)> = Vec::new();
        {
            let b = inp.as_bytes();
            let mut ins_at = |one: &mut Vec<(String, u8)>, gap: u8, ch: u8| {
                let mut v = b[..gap as usize].to_vec();
                v.push(ch);
                v.extend_from_slice(&b[gap as usize..]);
                one.push((String::from_utf8(v).expect("纯 ASCII 插入"), 1));
            };
            let mut del_at = |one: &mut Vec<(String, u8)>, i: u8| {
                let mut v = b.to_vec();
                v.remove(i as usize);
                one.push((String::from_utf8(v).expect("纯 ASCII 删除"), 1));
            };
            for i in lo..hi.min(inp.len()) {
                let cur = b[i];
                let prev = if i > 0 { b[i - 1] } else { 0 };
                let next = b.get(i + 1).copied().unwrap_or(0);
                // 鼻音: n 不后随 g → 插 g; ng → 删 g
                if cur == b'n' && next != b'g' {
                    ins_at(&mut one, (i + 1) as u8, b'g');
                }
                if cur == b'g' && prev == b'n' {
                    del_at(&mut one, i as u8);
                }
                // 平翘舌: z/c/s 不后随 h → 插 h; zh/ch/sh → 删 h
                if matches!(cur, b'z' | b'c' | b's') && next != b'h' {
                    ins_at(&mut one, (i + 1) as u8, b'h');
                }
                if cur == b'h'
                    && matches!(prev, b'z' | b'c' | b's')
                {
                    del_at(&mut one, i as u8);
                }
            }
        }
        for i in lo..hi.min(inp.len()) {
            if !inp.as_bytes()[i].is_ascii_lowercase() {
                continue;
            }
            for (v, _, _) in substitutions(inp, i, i + 1) {
                one.push((v, 1));
            }
            for (v, _, _) in insertions(inp, i, i) {
                // insertions(inp, i, i) = 只在 i 前插（位序对齐）
                one.push((v, 1));
            }
            for (v, _, _) in deletions(inp, i, i + 1) {
                one.push((v, 1));
            }
        }
        // 窗口右缘的插入（i=hi 间隙, 上面的位循环未覆盖）
        for (v, _, _) in insertions(inp, hi, hi) {
            one.push((v, 1));
        }
        for (v, e) in one {
            if out.len() >= max1 {
                break;
            }
            if seen.insert(v.clone()) && full(&v) {
                out.push(TypoVariant { s: v, edits: e });
            }
        }
        // ---- 2-编辑: sub×ins 组合, 编辑局部性序 ----
        for (v, _, _, _, _) in sub_insert_pairs(inp, lo, hi) {
            if out.len() >= max1 + max2 {
                break;
            }
            if seen.insert(v.clone()) && full(&v) {
                out.push(TypoVariant { s: v, edits: 2 });
            }
        }
        out
    }

    // ---------------- B2.9 模糊音变体编排 ----------------

    /// 模糊音门控模式设置（真 IME 模糊音设置位; 常量 FUZZ_AUTO/ON/OFF）。
    /// 进程级: 同进程多 Engine 实例共享（与单线程引擎契约一致）。
    pub fn set_fuzz_mode(&self, mode: u8) {
        FUZZ_MODE.store(mode, std::sync::atomic::Ordering::Relaxed);
    }

    /// 当前模糊音门控模式
    pub fn fuzz_mode(&self) -> u8 {
        FUZZ_MODE.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 原始串 → 全消耗合法变体（dp_reachable 过滤 + 截断）
    fn fuzz_legal_variants(&self, inp: &str, raw: Vec<String>) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for v in raw {
            if out.len() >= FUZZ_MAX_VARIANTS {
                break;
            }
            let r = self.dp_reachable(&v);
            if r[r.len() - 1] {
                out.push(v);
            }
        }
        out
    }

    /// B2.9 合法输入模糊音变体重试（query_typo_retry 健康分支挂点）。
    ///
    /// 门控: Off 直通; Auto = 快通道字节位预检 + 短输入门 + 结构质量门
    /// （全消耗最佳无词典词覆盖）; On 全跳（长句也救, 罚分律自护）。
    /// 变体: 一档鼻音 n↔ng, 零产出试二档平翘舌; 全零产出回退原面板。
    /// 合并律: 原生 full 保序保分入池, 变体 full 罚 0.3/编辑（1 编辑
    /// 变体恒 −0.3）, 覆盖类>缝合类、类内分数降序、稳定排序原生先;
    /// 面板 = 合并 full 池 ++ 原生非 full 尾段, 截 topk（B0 全词前置同型）。
    fn fuzz_retry(
        &self, inp: &str, ctx: Option<&str>, topk: usize, nn: bool, ltr: bool,
        base: Vec<Cand>,
    ) -> Vec<Cand> {
        use std::sync::atomic::Ordering::Relaxed;
        let mode = FUZZ_MODE.load(Relaxed);
        if mode == FUZZ_OFF {
            return base;
        }
        if mode == FUZZ_AUTO {
            // 快通道: 无 n/z/c/s 字节 → 两族零变体, 免后续一切开销
            if !inp
                .as_bytes()
                .iter()
                .any(|&b| matches!(b, b'n' | b'z' | b'c' | b's'))
            {
                return base;
            }
            if inp.len() > FUZZ_SHORT_MAX {
                return base;
            }
            let bf_kind = base.iter().find(|c| is_full(c, inp)).map(|c| c.kind);
            if !fuzz_gate_auto(inp.len(), bf_kind) {
                return base;
            }
        }
        // 一档鼻音; 零合法变体才试二档平翘舌（分档控误伤面与成本）
        let mut variants = self.fuzz_legal_variants(inp, nasal_edits(inp));
        if variants.is_empty() {
            variants = self.fuzz_legal_variants(inp, sib_edits(inp));
        }
        if variants.is_empty() {
            return base;
        }
        let mut pool: Vec<Cand> =
            base.iter().filter(|c| is_full(c, inp)).cloned().collect();
        let mut words: std::collections::HashSet<String> =
            pool.iter().map(|c| c.word.clone()).collect();
        let mut added = false;
        for v in variants {
            for mut c in self.query(&v, ctx, false, topk, nn, ltr, true) {
                if !is_full(&c, &v) || !words.insert(c.word.clone()) {
                    continue;
                }
                c.score -= TYPO_PENALTY; // 1 编辑 × 0.3（B2.8 同律）
                pool.push(c);
                added = true;
            }
        }
        if !added {
            // 变体查询零新增 full → 原面板原样返回（零伤害回退）
            return base;
        }
        // 合并律: 覆盖类（exact/complete/user/correct）> 缝合类; 类内
        // 罚后分降序; 稳定排序 = 原生先于同分变体
        pool.sort_by(|a, b| {
            cover_class(b.kind)
                .cmp(&cover_class(a.kind))
                .then(b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
        });
        let mut out = pool;
        out.extend(base.into_iter().filter(|c| !is_full(c, inp)));
        out.truncate(topk);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 位图构造助手: 给可达前缀长度集
    fn bm(n: usize, reach: &[usize]) -> Vec<bool> {
        let mut v = vec![false; n + 1];
        v[0] = true;
        for &r in reach {
            v[r] = true;
        }
        v
    }

    #[test]
    fn window_full_consume_none() {
        // 全消耗 → 无死段
        assert_eq!(dead_window(&bm(4, &[4])), None);
    }

    #[test]
    fn window_simple_dead_tail() {
        // "yi|gi": yi 可达(2), gi 死 → r=2, 窗口 [0,4)
        assert_eq!(dead_window(&bm(4, &[2])), Some((0, 4)));
    }

    #[test]
    fn window_mid_dead_clamps() {
        // 长串中途死: r=10 → 窗口 [4,12), hi 夹 n
        let m = bm(11, &[5, 10]);
        assert_eq!(dead_window(&m), Some((4, 11)));
    }

    #[test]
    fn window_r_zero_full_head() {
        // 首字节即死（r=0 可达空前缀）: 窗口 [0, min(8, n))——死段头
        // 整音节探窗（n=3 夹界）
        assert_eq!(dead_window(&bm(3, &[])), Some((0, 3)));
        // n>8: 全开头部窗
        assert_eq!(dead_window(&bm(20, &[])), Some((0, 8)));
    }

    #[test]
    fn subs_enumerate_counts() {
        // 4 字节窗口 × 25 替换 = 100; 恒等排除
        let n = substitutions("abcd", 0, 4).count();
        assert_eq!(n, 100);
    }

    #[test]
    fn subs_yield_correct_strings() {
        // ging 位置 0 g→q = qing 在枚举内（方向无关性: 全字母都试）
        let all: Vec<_> = substitutions("ging", 0, 3).collect();
        assert!(all.iter().any(|(v, i, c)| v == "qing" && *i == 0 && *c == b'q'));
        assert!(all.iter().any(|(v, _, _)| v == "jing"));
        assert!(all.iter().any(|(v, _, _)| v == "ting"));
    }

    #[test]
    fn subs_skip_non_letters() {
        // 非小写字母位被跳过（防御: 输入约定纯 a-z, 违约不炸）
        assert_eq!(substitutions("a-b", 0, 3).count(), 50); // 2 字母位 × 25
    }

    #[test]
    fn ins_enumerate_positions_and_counts() {
        // "ab" 窗口 [0,2]: 间隙 0/1/2 各 26 = 78
        let all: Vec<_> = insertions("ab", 0, 2).collect();
        assert_eq!(all.len(), 78);
        // 在 0 前插 x → "xab"; 在 1 前插 → "axb"; 在 2 前插（右缘）→ "abx"
        assert!(all.iter().any(|(v, i, c)| v == "xab" && *i == 0 && *c == b'x'));
        assert!(all.iter().any(|(v, i, _)| v == "axb" && *i == 1));
        assert!(all.iter().any(|(v, i, _)| v == "abx" && *i == 2));
    }

    #[test]
    fn ins_yields_jging_fix_geometry() {
        // "jging" 在 9 位前插 i 的同构: 位置 1 前插 i → "jiging"
        let all: Vec<_> = insertions("jging", 0, 2).collect();
        assert!(all.iter().any(|(v, i, c)| v == "ijging" && *i == 0 && *c == b'i'));
        assert!(all.iter().any(|(v, i, c)| v == "jiging" && *i == 1 && *c == b'i'));
    }

    #[test]
    fn del_enumerate() {
        // "abcd" 窗口 [1,3): 删 b → "acd", 删 c → "abd"; 携带被删字节
        let all: Vec<_> = deletions("abcd", 1, 3).collect();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], ("acd".into(), 1, b'b'));
        assert_eq!(all[1], ("abd".into(), 2, b'c'));
    }

    #[test]
    fn del_skip_non_letters() {
        // 撇号位不删（防御）
        assert_eq!(deletions("a'b", 0, 3).count(), 2); // a 与 b
    }

    #[test]
    fn pairs_distance_bound_and_geometry() {
        // "jg" 窗口 [0,2): sub i=0(g? 例: i=1 位 'g' 替换 q) × 插入。
        // 验证: (变体, sub i=1 c='q', ins j=1 d='i') = "jiq"（j i q:
        // 先替换 g→q 得 "jq", 再 1 位前插 i → "jiq"）
        let all: Vec<_> = sub_insert_pairs("jg", 0, 2).collect();
        // 距离界: ins 位 ∈ {i-2..i+2} ∩ [lo,hi]
        assert!(all
            .iter()
            .any(|(v, i, c, j, d)| v == "jiq" && *i == 1 && *c == b'q' && *j == 1 && *d == b'i'));
        // 计数: i=0 'j': j∈{0,1,2}∩[0,2] × 26 × 25; i=1 'g': j∈{0,1,2}...
        // lo=0,hi=2: i=0: j∈{0,1,2}=3; i=1: j∈{0,1,2}=3 → (3+3)×26×25
        assert_eq!(all.len(), 6 * 26 * 25);
    }

    #[test]
    fn pairs_locality_order() {
        // 编辑局部性序: 同 sub 位先 dist=0 再 dist=1; 同距离插入位升序;
        // 同位插入字母升序、替换字母升序
        let all: Vec<_> = sub_insert_pairs("abc", 1, 3).collect();
        let idx = all
            .iter()
            .position(|(_, i, _, j, _)| *i == 2 && *j == 2)
            .unwrap();
        let first_at_i2 = all
            .iter()
            .position(|(_, i, _, _, _)| *i == 2)
            .unwrap();
        // i=2 的首个组合 = (i=2, j=2, d='a', c= first ≠'c')
        assert_eq!(first_at_i2, idx);
        let (v, _, c, _, d) = &all[first_at_i2];
        // "abc" 先 sub c→a 得 "aba", 再 2 位前插 a → "abaa"
        assert_eq!((v.as_str(), *c, *d), ("abaa", b'a', b'a'));
    }

    #[test]
    fn pairs_duplicate_geometries_yield_same_string() {
        // (sub c@1, ins i@1) 与 (sub i@1... ) 组合几何同串现象:
        // "jgng" 上 (i=1,'q',j=1,'i') = "jiqng"; (i=1,'i',j=2,'q') = j i q n g
        // 先替换 g→i: "jing"? j-i-n-g, 再 2 位前插 q → "jiqng" 同串
        let all: Vec<_> = sub_insert_pairs("jgng", 0, 4).collect();
        let n1 = all.iter().filter(|(v, _, _, _, _)| v == "jiqng").count();
        assert!(n1 >= 2, "双几何应产出同串 ≥2 回: {n1}");
    }

    /// ---- B2.9 纯层: 鼻音/平翘舌变位枚举 + 门控判定（免数据）----

    #[test]
    fn nasal_ins_del_geometry() {
        // qijinbamai: n@4（后随 b）插 g → qijingbamai; 无 ng → 无删除族
        assert_eq!(nasal_edits("qijinbamai"), vec!["qijingbamai".to_string()]);
        // wangba: n@2 后随 g（免插）+ ng 删 g → wanba
        assert_eq!(nasal_edits("wangba"), vec!["wanba".to_string()]);
        // renmin: n@2 插 → rengmin; 尾 n@5 插 → renming
        assert_eq!(
            nasal_edits("renmin"),
            vec!["rengmin".to_string(), "renming".to_string()]
        );
    }

    #[test]
    fn nasal_ins_positions_and_skip_ng() {
        // jinnian: n@2（后随 n）插 → jingnian; n@3（后随 i）插 → jinngian
        //（非法切分, dp 过滤层负责, 原始枚举照出）; 尾 n → jinniang
        assert_eq!(
            nasal_edits("jinnian"),
            vec![
                "jingnian".to_string(),
                "jinngian".to_string(),
                "jinniang".to_string(),
            ]
        );
        // 纯 ng 输入: qing 的 n 后随 g 免插; 无删除位（qing 无 ng? 有: n@g
        // 对——qing = q-i-n-g, ng 删 g → qin）
        assert_eq!(nasal_edits("qing"), vec!["qin".to_string()]);
    }

    #[test]
    fn nasal_initial_n_inserted_then_filtered_upstream() {
        // 声母 n 位也枚举（nian 的 n@0 插 g → ngian, 非法音节由
        // dp_reachable 过滤; 原始层方向无关性同 B2.8 哲学）
        let vs = nasal_edits("nian");
        assert!(vs.contains(&"ngian".to_string()));
        assert!(vs.contains(&"niang".to_string()));
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn sib_ins_del_geometry() {
        // sizi: s@0 插 h → shizi; z@2 插 h → sizhi（si|zhi 亦合法）
        assert_eq!(
            sib_edits("sizi"),
            vec!["shizi".to_string(), "sizhi".to_string()]
        );
        // shizi: sh 删 h → sizi; s@0 后随 h 免插; z@3（后随 i）插 h →
        // shizhi（shi|zhi 亦合法切分, 与 sizi 同为变体）
        assert_eq!(sib_edits("shizi"), vec!["sizi".to_string(), "shizhi".to_string()]);
        // zhongguo: zh 删 h → zongguo; 尾无 z/c/s 位
        assert_eq!(sib_edits("zhongguo"), vec!["zongguo".to_string()]);
        // canjia: c@0 插 h → chanjia; n 位不属本族
        assert_eq!(sib_edits("canjia"), vec!["chanjia".to_string()]);
        // cheng: ch 删 h → ceng
        assert_eq!(sib_edits("cheng"), vec!["ceng".to_string()]);
    }

    #[test]
    fn fuzz_gate_auto_structural() {
        // 无 full 覆盖（None）→ 入场
        assert!(fuzz_gate_auto(10, None));
        // 缝合句 → 入场（错拼嫌疑域: qijinbamai 企金吧买 sentence）
        assert!(fuzz_gate_auto(10, Some(Kind::Sentence)));
        assert!(fuzz_gate_auto(10, Some(Kind::Path)));
        // 词典词覆盖 → 免触发（jinnian 今年 exact 等）
        for k in [Kind::Exact, Kind::Complete, Kind::User, Kind::Correct] {
            assert!(!fuzz_gate_auto(7, Some(k)), "{k:?} 不应触发");
        }
        // 长输入 → 免触发（52 句正确组最短 20 字节, 全组门外）
        assert!(!fuzz_gate_auto(17, None));
        assert!(!fuzz_gate_auto(20, Some(Kind::Sentence)));
        // 边界: 16 字节内仍入场
        assert!(fuzz_gate_auto(16, None));
    }

    #[test]
    fn fuzz_cover_class_and_full() {
        // 覆盖类 vs 缝合类
        assert_eq!(cover_class(Kind::Exact), 1);
        assert_eq!(cover_class(Kind::User), 1);
        assert_eq!(cover_class(Kind::Sentence), 0);
        assert_eq!(cover_class(Kind::Path), 0);
        // full 判定: Sentence kind 或 reading 去撇号 == 键
        let mut c = Cand::new("x".into(), 0.0, Kind::Sentence);
        assert!(is_full(&c, "anything"));
        c = Cand::new("x".into(), 0.0, Kind::Exact);
        c.reading = "qi'jing'ba'mai".into();
        assert!(is_full(&c, "qijingbamai"));
        assert!(!is_full(&c, "qijinbamai"));
    }

    #[test]
    fn fuzz_mode_static_roundtrip() {
        // 模式静态读写（Engine 方法需数据; 直测原子层, 用毕复位 Auto）
        let old = FUZZ_MODE.load(std::sync::atomic::Ordering::Relaxed);
        for m in [FUZZ_AUTO, FUZZ_ON, FUZZ_OFF] {
            FUZZ_MODE.store(m, std::sync::atomic::Ordering::Relaxed);
            assert_eq!(
                FUZZ_MODE.load(std::sync::atomic::Ordering::Relaxed),
                m
            );
        }
        FUZZ_MODE.store(old, std::sync::atomic::Ordering::Relaxed);
    }

    /// ---- 编排层真机测试（需 wxq_dict.bin）----

    fn engine() -> Engine {
        let bin = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
            format!(
                "{}/tmp/wetype-win/wxq_dict.bin",
                std::env::var("HOME").unwrap()
            )
        });
        Engine::open(std::path::Path::new(&bin)).expect("wxq_dict.bin 装载")
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn healthy_input_no_variants() {
        // 全消耗健康输入 → 零变体（零开销路径）
        let e = engine();
        assert!(e.typo_variants("nihao", 8, 12).is_empty());
        assert!(e.typo_variants("jintiantianqibucuo", 8, 12).is_empty());
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn gingshan_yields_qing_jing() {
        // ging 死段: qing/jing 方向变体在场（方向枚举性, 非规则纠错）
        let e = engine();
        // cap 16: 1-编辑位序首位 g→X 家族在前（v1 行为保持）
        let vs = e.typo_variants("gingshan", 16, 0);
        let strs: Vec<&str> = vs.iter().map(|t| t.s.as_str()).collect();
        assert!(strs.contains(&"qingshan"), "vs = {strs:?}");
        assert!(strs.contains(&"jingshan"), "vs = {strs:?}");
        assert!(strs.contains(&"tingshan"), "vs = {strs:?}");
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn long_sentence_yigi_fix() {
        // #4 长句: yigi 死段 → yiqi 变体在场（真机 46 字节窗枚举）
        let e = engine();
        let inp = "hegangjiaodewangyoudajibawangzhewanleyigiquhuashan";
        let vs = e.typo_variants(inp, 32, 0);
        assert!(
            vs.iter().any(|t| t.s.contains("yiqi")),
            "变体缺 yiqi: {:?}",
            vs.iter().map(|t| &t.s[36..]).collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn twoedit_pair_enumerated_for_jging() {
        // #1 双错: sub g→q × ins i@9（jging→jiqing）在 2-编辑组合内
        let e = engine();
        let inp = "chongmanjgingdebasheyuanbidaodamudidigengnenggeirendailailequ";
        let vs = e.typo_variants(inp, 8, 12);
        assert!(
            vs.iter().any(|t| t.s.contains("jiqingde") && t.edits == 2),
            "缺 jiqing 2-编辑变体: {:?}",
            vs.iter().map(|t| t.s.as_str()).collect::<Vec<_>>()
        );
        // 1-编辑侧: 删 g → jing 变体在场（编辑数先验分层）
        assert!(vs.iter().any(|t| t.s.contains("jingde") && t.edits == 1));
    }


    /// ---- 混合输入域（设计-混合输入 §2.4）真机测试 ----

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn mixed_segs_gate_and_segmentation() {
        // 门: 纯全拼/纯声母 → None（各归原生 DP / c.3 域）
        let e = engine();
        assert!(boost::mixed_segs(&e, "nihao").is_none(), "纯全拼归原生 DP");
        assert!(boost::mixed_segs(&e, "tqsdy").is_none(), "纯声母归 c.3");
        assert!(boost::mixed_segs(&e, "ab").is_none(), "长度下界门");
        // 混合切分: jin|tian|t|q → 虚拟 jttq + 全拼段约束
        let (vbuf, constrain) =
            boost::mixed_segs(&e, "jintiantq").expect("混合键可达");
        assert_eq!(vbuf, "jttq");
        assert_eq!(constrain.get(&0).map(String::as_str), Some("jin"));
        assert_eq!(constrain.get(&1).map(String::as_str), Some("tian"));
        // zh 双键声母整体: jin|t|zh → 虚拟 jtz（zh 不拆）; 2 槽不足门
        // （部件层 3..=12 同律, jinzh 由变体/原生路径接手）
        assert!(boost::mixed_segs(&e, "jinzh").is_none(), "2 槽不足门");
        let (vbuf2, _) =
            boost::mixed_segs(&e, "jintzh").expect("混合键可达");
        assert_eq!(vbuf2, "jtz");
        // 槽数门非字节门: chongmanjiqdbsh 14 字节 7 槽可达
        let (vbuf3, _) = boost::mixed_segs(&e, "chongmanjiqdbsh")
            .expect("长混合键（槽数门）可达");
        assert_eq!(vbuf3.len(), 7);
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn mixed_input_probe_and_knob_gate() {
        // 旋钮开: 不可达分支走原链（原始输入的混合面登顶, 不被 typo
        // 变体面污染——q→a 变体「今天昂起」案）; top3 含 今天天气
        let e = engine();
        let mut e2 = engine();
        e2.set_boost(crate::engine::BoostCfg {
            mixed_input: true,
            ..Default::default()
        });
        let out = e2.query_typo_retry("jintiantq", None, 8, true, true);
        assert!(
            out.iter().take(3).any(|c| c.word == "今天天气"),
            "混合面应在 top3: {:?}",
            out.iter().map(|c| c.word.as_str()).take(5)
                .collect::<Vec<_>>()
        );
        // 旋钮关: 原变体池行为（B2.8 恒等——面板不含注入面/或仍是
        // 变体面主导, 关键是与旋钮开不同源）
        let out0 = e.query_typo_retry("jintiantq", None, 8, true, true);
        assert!(!out0.is_empty(), "旋钮关仍有变体池面板");
        // 简拼域收口（≥8 位纯声母）: 真句面替换字面透传垃圾
        // （cmjqdbsy吧）; 目标面受频序墙压制归 ctx 臂（设计同律）
        let jp = e2.query_typo_retry("cmjqdbsyb", None, 8, true, true);
        assert!(
            !jp.is_empty()
                && jp[0].kind == crate::engine::Kind::Sentence
                && !jp[0].word.contains('c')
                || jp[0].word.chars().all(|c| !c.is_ascii()),
            "纯声母长键 top1 应为真句面: {:?}",
            jp.first().map(|c| c.word.clone())
        );
    }

/// 死窗右缘覆盖整音节错字母（wenzhon 案——修法在 r+2 之外, 旧窗
/// 枚举恒空）; 修正变体在场且为 1-编辑。
#[test]
#[ignore = "真机: 需 wxq_dict.bin"]
fn typo_variants_long_news_wenzhong_in_pool() {
    let e = engine();
    let inp = "huiyiqiangdiaoyaojianchiwenzhonqiujintuidonggaozhiliangfazhan";
    let vs = e.typo_variants(inp, 8, 12);
    assert!(
        vs.iter().any(|t| t.s.contains("wenzhong") && t.edits == 1),
        "wenzhong 1-编辑变体不在池: {:?}",
        vs.iter().map(|t| t.s.as_str()).collect::<Vec<_>>()
    );
}

    /// ---- B2.8 入口层真机测试 ----

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn retry_entry_passthrough_healthy() {
        // 健康输入 = query(dp) 直通（词面逐位一致）
        let e = engine();
        let a = e.query_typo_retry("nihao", None, 30, true, true);
        let b = e.query("nihao", None, false, 30, true, true, true);
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x.word == y.word));
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn retry_fixes_typo_sentences() {
        let e = engine();
        // #4: yigi 死段 → 全消耗句进场（top5 含 的网友——LM 选向实证）
        let out = e.query_typo_retry(
            "hegangjiaodewangyoudajibawangzhewanleyigiquhuashan",
            None, 30, true, true,
        );
        assert!(
            out.iter().take(5).any(|c| c.word.contains("的网友")),
            "top3 = {:?}",
            out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
        // #2: ging 死段 → 苍茫的天 进场（1-编辑优先不被组合稀释）
        let out2 = e.query_typo_retry(
            "cangmangdetianyashiwodeaimianyandegingshanjiaoxiahuashengkai",
            None, 30, true, true,
        );
        assert!(
            out2.iter().take(5).any(|c| c.word.contains("苍茫的天")),
            "top3 = {:?}",
            out2.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn retry_fixes_2edit_sentence() {
        // #1 双错（j+ging = 漏 i + g 错拼 q）: v1.1 组合变体修复——
        // 充满激情 全消耗句进场, rank ≤ 10（句分对同音头结构性并列,
        // 编辑局部性序把 jiqing 排进前列; 实测 rank 9）
        let e = engine();
        let t0 = std::time::Instant::now();
        let out = e.query_typo_retry(
            "chongmanjgingdebasheyuanbidaodamudidigengnenggeirendailailequ",
            None, 30, true, true,
        );
        let dt = t0.elapsed();
        let rank = out.iter().position(|c| c.word.contains("充满激情"));
        assert!(
            rank.is_some_and(|r| r < 10),
            "充满激情 rank = {:?}, top10 = {:?}",
            rank,
            out.iter().take(10).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
        println!("#1 修复 rank={} 耗时 {:?}", rank.map(|r| r + 1).unwrap_or(0), dt);
        // 热路径回归: 二次调用同句不劣化（缓存热）
        let t1 = std::time::Instant::now();
        let _ = e.query_typo_retry(
            "chongmanjgingdebasheyuanbidaodamudidigengnenggeirendailailequ",
            None, 30, true, true,
        );
        println!("#1 热路径 {:?}", t1.elapsed());
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn retry_fallback_unfixable() {
        // ≥3 编辑/超窗 → 回退原查询不劣化（空面板防御, 非空即过）
        let e = engine();
        let out = e.query_typo_retry("hhhhhhhhhh", None, 30, true, true);
        assert!(!out.is_empty());
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn retry_fallback_dp_stripped_latency() {
        // B2.8 回退路径 dp 剥离: 合法头 + 三连死尾（qqq = 窗内 ≤2 编辑
        // 必不可修 → 变体池空走回退）。release 实测: dp=true 病理 5.2s,
        // dp=false 62ms（含进程启动; 快 84×）。debug 档给 30s 预算
        //（dp=true 病理在 debug 数分钟级, 仍能区分）。面板非空不劣化。
        // 另注: 首部即死的长串（hhh… 61B）慢在悬尾 fuzzcomp 枚举
        //（plain query 同慢, 与 dp 无关）——旧裁定封存域, 非本修对象。
        let e = engine();
        let _ = e.query("warmup", None, false, 5, true, true, true);
        let inp = "chongmanjgingdebasheyuanbidaodamudidigengnenggeirendailaileqqq";
        let t0 = std::time::Instant::now();
        let out = e.query_typo_retry(inp, None, 30, true, true);
        let dt = t0.elapsed();
        assert!(!out.is_empty(), "回退面板空");
        println!(
            "61B 死尾回退 {dt:?}, top3={:?}",
            out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
        let budget_ms = if cfg!(debug_assertions) { 30_000 } else { 100 };
        assert!(dt.as_millis() < budget_ms as u128, "回退耗时 {dt:?} ≥ {budget_ms}ms");
    }

    /// ---- B2.9 入口层真机测试（模式静态串行锁: 模式可变测试互斥）----
    /// 中毒容忍: 某测试失败不级联（互斥语义不变, 锁卫兵照常持有到期）
    static FUZZ_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn fuzz_lock() -> std::sync::MutexGuard<'static, ()> {
        FUZZ_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_rescue_qijinbamai() {
        // 验收探针 #1: jin→jing（前后鼻音最高频错类）, 奇经八脉 top5
        //（合并律: 变体 exact 覆盖类 > 原生缝合句, 期望 top1）
        let _g = fuzz_lock();
        let e = engine();
        // 预热: debug 测试档首查含 mmap 缺页 + 冷缓存（实测冷首查 1.9s,
        // release CLI 热档全程 85ms/含进程启动——延迟口径取热路径）
        let _ = e.query("warmup", None, false, 5, true, true, true);
        let t0 = std::time::Instant::now();
        let out = e.query_typo_retry("qijinbamai", None, 30, true, true);
        let dt = t0.elapsed();
        let rank = out.iter().position(|c| c.word == "奇经八脉");
        assert!(
            rank.is_some_and(|r| r < 5),
            "奇经八脉 rank={rank:?}, top5={:?}",
            out.iter().take(5).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
        println!("qijinbamai rank={} 耗时 {dt:?}", rank.map(|r| r + 1).unwrap_or(0));
        assert!(dt.as_millis() < 300, "触发延迟 {dt:?} > 300ms");
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_zero_harm_jinnian_and_covers() {
        // 验收探针 #2（零伤害对照）: jinnian→今年 rank1 面板逐位不动;
        // 词典词覆盖类输入同律（Auto 门全免触发, 输出与原生 query 全等）
        let _g = fuzz_lock();
        let e = engine();
        for inp in ["jinnian", "jinrong", "nihaoshijie", "jintiantianqi", "nihao"] {
            let a = e.query_typo_retry(inp, None, 30, true, true);
            let b = e.query(inp, None, false, 30, true, true, true);
            assert_eq!(a.len(), b.len(), "{inp} 面板长度漂移");
            assert!(
                a.iter().zip(&b).all(|(x, y)| x.word == y.word),
                "{inp} 面板词面漂移"
            );
        }
        let out = e.query_typo_retry("jinnian", None, 30, true, true);
        assert_eq!(out[0].word, "今年", "jinnian top1={:?}", out[0].word);
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_rescue_legal_typos() {
        // 自造鼻音案（双向: del-g rengmin/wengrou, ins-g xinweng/
        // xinwenglianbo 短词+短语两档）+ 模糊双读案 jingnian。
        // 注: wanluo/yinghang/chenshi 类不救——错拼串本身有 exact 词覆盖
        //（完络/英航/陈世）, 覆盖门保护, 见设计文档 §7
        let _g = fuzz_lock();
        let e = engine();
        for (inp, want, rank_max) in [
            ("rengmin", "人民", 5),
            ("wengrou", "温柔", 5),
            ("xinweng", "新闻", 5),
            ("xinwenglianbo", "新闻联播", 5),
            ("jingnian", "今年", 3),
        ] {
            let out = e.query_typo_retry(inp, None, 30, true, true);
            let rank = out
                .iter()
                .position(|c| c.word == want)
                .map(|r| r + 1)
                .unwrap_or(usize::MAX);
            assert!(
                rank <= rank_max,
                "{inp}→{want} rank={rank}, top5={:?}",
                out.iter().take(5).map(|c| c.word.clone()).collect::<Vec<_>>()
            );
            println!("{inp} → {want} rank {rank}");
        }
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_tier2_sib_rescue() {
        // 二档平翘舌: zidao（字到 缝合, 知道 不可达）→ zhidao 变体救
        let _g = fuzz_lock();
        let e = engine();
        let base = e.query("zidao", None, false, 30, true, true, true);
        assert!(
            !base.iter().any(|c| c.word == "知道"),
            "前置失效: 知道 已在原生面板（改用他例）"
        );
        let out = e.query_typo_retry("zidao", None, 30, true, true);
        let rank = out.iter().position(|c| c.word == "知道").map(|r| r + 1);
        assert!(
            rank.is_some_and(|r| r <= 5),
            "zidao→知道 rank={rank:?}, top5={:?}",
            out.iter().take(5).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_gate_neutral_fire_harmless() {
        // 中性触发案（无明确错拼意图, 触发后面板行为合法）:
        // - tiancaidongwu: 原生 top1 天才动物 原位不动, 变体缝合句
        //   （ti|ang 切分变体）罚 0.3 沉原生之下——罚分律自护实证
        // - qianquan/yinyun: 覆盖类排序副作用 = 面板内已有 exact 词
        //   （缱绻/氤氲, 原生被缝合句压在 rank2）登顶, 原生 top1 仍在 top5
        let _g = fuzz_lock();
        let e = engine();
        let out = e.query_typo_retry("tiancaidongwu", None, 30, true, true);
        assert_eq!(out[0].word, "天才动物", "top1={:?}", out[0].word);
        // 变体缝合句（ti|ang 切分变体）罚 0.3 沉于原生 top1 之下
        let junk = out.iter().position(|c| c.word == "体昂才动物");
        assert!(
            junk.is_some_and(|r| r > 0),
            "变体缝合句应在 原生 top1 之后: {:?}",
            out.iter().take(4).map(|c| c.word.clone()).collect::<Vec<_>>()
        );
        for (inp, new_top1, old_top1) in
            [("qianquan", "缱绻", "钱全"), ("yinyun", "氤氲", "音云")]
        {
            let out = e.query_typo_retry(inp, None, 30, true, true);
            assert_eq!(out[0].word, new_top1, "{inp} top1={:?}", out[0].word);
            let rank = out
                .iter()
                .position(|c| c.word == old_top1)
                .map(|r| r + 1)
                .unwrap_or(usize::MAX);
            assert!(rank <= 10, "{inp} 原生 top1 {old_top1} rank={rank}");
        }
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_mode_off_and_force_on() {
        // Off: qijinbamai 与原生全等（救援关闭）; On: 跳门但罚分律自护
        //（jinnian 今年 rank1 不动; 长句健康输入同护）。用毕复位 Auto。
        let _g = fuzz_lock();
        let e = engine();
        e.set_fuzz_mode(FUZZ_OFF);
        let a = e.query_typo_retry("qijinbamai", None, 30, true, true);
        let b = e.query("qijinbamai", None, false, 30, true, true, true);
        assert!(a.iter().zip(&b).all(|(x, y)| x.word == y.word));
        assert!(!a.iter().take(5).any(|c| c.word == "奇经八脉"));
        e.set_fuzz_mode(FUZZ_ON);
        let out = e.query_typo_retry("jinnian", None, 30, true, true);
        assert_eq!(out[0].word, "今年", "force-on 劫持: {:?}", out[0].word);
        // 长句 force-on: 52 句正确组代表（top1 不被变体劫持）
        let out = e.query_typo_retry("jintiantianqibucuo", None, 30, true, true);
        assert_eq!(out[0].word, "今天天气不错", "长句 force-on 劫持");
        e.set_fuzz_mode(FUZZ_AUTO);
    }

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn fuzz_trigger_cost_on_eval_set() {
        // 成本实测: 52 句正确组全组免触发（长度门, 结构性零开销）;
        // 触发案延迟档（qijinbamai 首查冷缓存 + 热路径）
        let _g = fuzz_lock();
        let e = engine();
        let rows: Vec<String> = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../eval/eval_sentence.jsonl"),
        )
        .expect("eval_sentence.jsonl")
        .lines()
        .filter_map(|l| {
            // 轻量抽取 "py":"…"（jsonl 单行单对象, 免 serde 依赖）
            let k = l.split_once("\"py\":\"")?.1;
            Some(k.split_once('"')?.0.to_string())
        })
        .collect();
        assert!(rows.len() >= 50, "eval 句集加载异常: {}", rows.len());
        let mut fired = 0;
        for py in &rows {
            let a = e.query_typo_retry(py, None, 30, true, true);
            let bl = e.query(py, None, false, 30, true, true, true);
            if a.len() != bl.len()
                || !a.iter().zip(&bl).all(|(x, y)| x.word == y.word)
            {
                fired += 1;
            }
        }
        assert_eq!(fired, 0, "正确组 {fired} 句面板漂移（应全组免触发）");
        // 热路径延迟
        let t0 = std::time::Instant::now();
        let _ = e.query_typo_retry("qijinbamai", None, 30, true, true);
        println!("qijinbamai 热路径 {:?}", t0.elapsed());
    }
}
