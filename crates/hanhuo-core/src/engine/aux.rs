//! 辅助码入口（融合-辅助码-万象 §3.1 P0 引导符模式, ADR-002 只增不改）
//!
//! 定位: query() **前**剥辅码语法、**后**过滤/提权的包装层（typo_retry 同款
//! 独立入口模式, 不动 query() 本体——原生数字/标点双阻断律（query.rs
//! L716-735）是设计前提: `` ` ``/7890//` 不可能在原生管线内表达, 故
//! 必须在调 query() 前剥离, 拼音段走原路径（boost/userdict 照常生效）。
//!
//! 语法（§3.1 P0）: `拼音段` + `` ` `` + `辅码段`
//! - 辅码段字母 = 形码查询串（a-z; 大写=锁定标记, §3.1 裁定不做 → 拒收）
//! - 数字 7890 = 声调 1/2/3/4（乱序自由, 连续只保留最后一位——万象律;
//!   **声调过滤 TODO**, 语法位已占住: 解析进 AuxSpec.tone）
//! - 尾缀 `/` = 聚拢（丢词组只留单字; P0 以纯 retain 收敛实现）
//! - 其他字符（含 1-6、第二个 `` ` ``）→ 语法无效 → 空返回
//!   （与原生对该输入的阻断行为逐位一致, 零扰动）
//!
//! 匹配（§3.3 收敛版）: 候选**首字/末字**辅码与查询串做有序子序列匹配
//! （顺序不反、可跨位跳跃; 单键简码 j ⊆ jg 亦命中）。三态:
//! - Filter: 仅保留命中, **空结果回退原列**（防打错辅码颗粒无收）
//! - Promote: 命中钉序提权（score = top+0.5−0.01×i, B2.7 jianpin_inject
//!   同式）, 未命中原序沉后——稳定分区, 不与原生序合并重排（B2w-σ 律）
//! - Mixed: L1 单字命中 > L2 词组命中 > 未命中原序（万象四级排序收敛）
//!
//! golden 零扰动（§3.5）: 本入口独立, golden/CLI 原路径不引用; 只作用于
//! aux_query 调用者的输出列, eng.query() 返回值原样。

use crate::Engine;
use crate::engine::Cand;
use hanhuo_dict::AuxForm;

/// 辅码方案（§3.2 旋钮: 同一引擎挂不同码表=万象九方案切换语义）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuxScheme {
    /// 五笔前 2 码（呵→ks; 默认, 万象"五笔前2"）
    Wubi2,
    /// 全码首末（镇→qw, 虎码首末风格）
    WubiHeadTail,
    /// 笔顺 hspdz（你→pspzspd; 万象律: 词组不走笔顺源——本层统一按字查,
    /// 词级性能由调用方 topk 兜底）
    Strokes,
}

/// 候选交互三态（§3.3）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuxMode {
    /// 仅保留命中; 空结果回退原列
    Filter,
    /// 命中钉序提权, 未命中原序沉后
    Promote,
    /// L1 单字命中 > L2 词组命中 > 未命中原序
    Mixed,
}

/// 辅助码配置
#[derive(Clone, Debug)]
pub struct AuxCfg {
    pub scheme: AuxScheme,
    pub mode: AuxMode,
    /// 引导符（默认 `` ` ``; recognizer: `^\`[a-z7890/]*$`）
    pub leader: char,
}

impl Default for AuxCfg {
    fn default() -> Self {
        AuxCfg {
            scheme: AuxScheme::Wubi2,
            mode: AuxMode::Filter,
            leader: '`',
        }
    }
}

/// 辅码段解析产物（声调/聚拢语法位占住, 应用层 P0/P2 渐进）
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuxSpec {
    /// 拼音段（已验证 a-z/'/空格, 直喂 query()）
    pub pinyin: String,
    /// 形码查询串（辅码段字母, 原序）
    pub code_q: String,
    /// 声调（7890→1/2/3/4, 连续只保留最后一位; **过滤 TODO**）
    pub tone: Option<u8>,
    /// 聚拢 `/`（丢词组只留单字）
    pub gather: bool,
}

/// 解析三态: 无引导符（直通）/ 语法有效 / 语法无效（空返回=原生阻断同像）
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuxParse {
    None,
    Ok(AuxSpec),
    Invalid,
}

/// `` ` `` 引导切分（§3.1 P0）。首个引导符为硬边界（`ni\`de` 无歧义:
/// 剥离后 de 亦音节也只作辅码段, 不回拼）。
pub fn parse_aux(inp: &str, leader: char) -> AuxParse {
    let Some((py, aux)) = inp.split_once(leader) else {
        return AuxParse::None;
    };
    // 拼音段: 原生白名单子集（字母+撇号+空格; 大写走 q_upper 原生域,
    // 辅码语法拒收——§3.1 大写辅码形态不做）
    if py.is_empty()
        || !py.chars().all(|c| matches!(c, 'a'..='z' | '\'' | ' '))
    {
        return AuxParse::Invalid;
    }
    let mut spec = AuxSpec {
        pinyin: py.to_string(),
        code_q: String::new(),
        tone: None,
        gather: false,
    };
    let bytes: Vec<char> = aux.chars().collect();
    for (i, c) in bytes.iter().enumerate() {
        match c {
            'a'..='z' => spec.code_q.push(*c),
            // 声调 7890→1/2/3/4（轻声并入 4）; 乱序自由, 只保留最后一位
            '7' | '8' | '9' | '0' => {
                spec.tone = Some(match c {
                    '7' => 1,
                    '8' => 2,
                    '9' => 3,
                    _ => 4,
                });
            }
            // 聚拢: 仅允许尾缀位
            '/' if i == bytes.len() - 1 => spec.gather = true,
            _ => return AuxParse::Invalid,
        }
    }
    AuxParse::Ok(spec)
}

/// 有序子序列匹配（万象律: 顺序不反、可跨位跳跃; 空查询恒真）
pub fn is_subseq(q: &[u8], code: &[u8]) -> bool {
    let mut i = 0;
    for &c in code {
        if i < q.len() && q[i] == c {
            i += 1;
        }
    }
    i == q.len()
}

/// 方案化取字辅码
fn code_of(form: &AuxForm, scheme: AuxScheme, ch: char) -> Option<&str> {
    match scheme {
        AuxScheme::Wubi2 => form.wubi2(ch),
        AuxScheme::WubiHeadTail => form.wubi_head_tail(ch),
        AuxScheme::Strokes => form.strokes(ch),
    }
}

/// P0 命中判据: 首字或末字辅码 ⊇ 查询串（有序子序列; 单字候选首=末）。
/// 聚拢时收敛为仅单字。缺码字该侧不命中（不猜律）。
pub fn cand_hit(
    form: &AuxForm, scheme: AuxScheme, q: &[u8], word: &str, gather: bool,
) -> bool {
    if gather && word.chars().count() != 1 {
        return false;
    }
    if q.is_empty() {
        return true; // 纯声调/纯聚拢语法: 无形码约束
    }
    let mut it = word.chars();
    let Some(first) = it.next() else { return false };
    let last = it.next_back().unwrap_or(first);
    [first, last].into_iter().any(|ch| {
        code_of(form, scheme, ch).is_some_and(|code| is_subseq(q, code.as_bytes()))
    })
}

/// 过滤宽列: 面板码深 96 档（syl_deep 表深）, 过滤后仍有料
const WIDE: usize = 96;

impl Engine {
    /// 辅助码查询入口（§3.1）。无引导符输入零开销直通 query()
    /// （nn/ltr/dp 与 typo_retry 入口同参: true/true/true, jianpin=false）。
    ///
    /// 例: `zhen\`qf` → query("zhen") → 镇系候选置顶/过滤;
    /// `ni\`wq9` → 声调语法占位（P0 不生效, 同 `ni\`wq`）;
    /// 无命中 → 回退拼音段原列（重跑一次 topk 原参, 逐位一致）。
    pub fn aux_query(
        &self, inp: &str, ctx: Option<&str>, form: &AuxForm,
        cfg: &AuxCfg, topk: usize,
    ) -> Vec<Cand> {
        match parse_aux(inp, cfg.leader) {
            AuxParse::None => {
                self.query(inp, ctx, false, topk, true, true, true)
            }
            AuxParse::Invalid => Vec::new(),
            AuxParse::Ok(spec) => {
                let q = spec.code_q.as_bytes();
                if cfg.mode == AuxMode::Filter {
                    // 过滤态: 宽列 retain 不动分; 空结果回退原参原列
                    // （重跑 topk 原参, 与裸 query 逐位一致）
                    let mut hits = self.query(
                        &spec.pinyin, ctx, false, WIDE.max(topk), true, true,
                        true,
                    );
                    hits.retain(|c| {
                        cand_hit(form, cfg.scheme, q, &c.word, spec.gather)
                    });
                    if hits.is_empty() {
                        return self.query(
                            &spec.pinyin, ctx, false, topk, true, true, true,
                        );
                    }
                    hits.truncate(topk);
                    return hits;
                }
                // 提权态: 纯声调无可提权 → 原列
                if q.is_empty() && !spec.gather {
                    return self.query(
                        &spec.pinyin, ctx, false, topk, true, true, true,
                    );
                }
                let wide = self.query(
                    &spec.pinyin, ctx, false, WIDE.max(topk), true, true, true,
                );
                if wide.is_empty() {
                    return wide;
                }
                let top = wide.first().map(|c| c.score).unwrap_or(0.0);
                // 三分稳定分区（保原序）: L1 单字命中 / L2 词组命中 / 未命中
                // （Promote 不分 L1/L2, 命中同段; Mixed L1 先于 L2——§3.3）
                let (mut l1, mut l2) = (Vec::new(), Vec::new());
                let mut miss: Vec<Cand> = Vec::new();
                for c in wide {
                    if !cand_hit(form, cfg.scheme, q, &c.word, spec.gather) {
                        miss.push(c);
                    } else if cfg.mode == AuxMode::Mixed
                        && c.word.chars().count() == 1
                    {
                        l1.push(c);
                    } else {
                        l2.push(c);
                    }
                }
                // 钉序提权: score = top+0.5−0.01×i（命中序内连续递减;
                // 未命中不动分不重排——B2w-σ 律: 不与原生序合并排序）
                let mut out: Vec<Cand> =
                    Vec::with_capacity(l1.len() + l2.len() + miss.len());
                for (i, mut c) in l1.drain(..).chain(l2).enumerate() {
                    c.score = top + 0.5 - 0.01 * i as f64;
                    out.push(c);
                }
                out.extend(miss);
                out.truncate(topk);
                out
            }
        }
    }
}

// 测试: crates/hanhuo-core/tests/aux_code.rs（集成测试文件——
// 纯语法律 + 真机编排律双层; 独立 target 编译, 不受 lib test 目标
// 其他模块的测试态影响, 参照 tests/input_domains.rs 惯例）。
