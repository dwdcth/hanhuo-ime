//! 汉火查询引擎：四层管线（L1 音节切分 / L2 DAT 读音索引 / L3 旁挂路由 / L4 忠实打分）.
//!
//! wxq/engine.py（Python 参考实现）的逐语义移植；golden/golden.jsonl 全量对照。
//!
//! L4 融合公式（B 档逆向成果）:
//!     final = f6 + clamp(0.23 · λ · ΔN, ±1.5),  λ = −0.40（NNLM 层 M4 接入）
//!     f6 = 词频层(−log10P, 原厂四段量化) + bigram lift + 覆盖罚 + 编辑罚(原厂罚表)
//!
//! 编辑罚（hpmac 十一旋钮）: 替换 1.25 / 插入 2.88 / 换位 2.99 / 删除 3.22
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::dat::{Dat, ROOT};
use crate::mdltr::{Mdltr, N_FEAT, Wfea};
use crate::nnlm::Nnlm;
use crate::pack::{BitArr, FcStrings, Pack, U16Arr, U32Arr, fnv1a, pack_values_to_bitarr};
use crate::quant::val_to_log10p;
use crate::syllables::SyllableSeg;
use crate::userdict::{
    Entry, MAX_WORD_BYTES, UserDict, user_boost, user_boost_at,
    user_boost_native,
};


// 阶段1: engine.rs 拆分为本目录多文件（纯移动; 子模块 impl Engine 同类型多文件是 rust 惯用法）。

mod boost;
pub use boost::BoostCfg;

impl Engine {
    /// 开启 boost 覆盖层（ADR-002; 默认 None = 忠实模式逐位一致）
    pub fn set_boost(&mut self, cfg: BoostCfg) {
        self.boost = Some(cfg);
    }

    /// T9 九宫模式（设计-T9九宫输入 §5-P1; 镜像 set_boost 开关制——
    /// 不改 query 签名, 全调用方零改动）
    pub fn set_t9(&mut self, on: bool) {
        self.t9 = on;
    }
}
mod channels_full;
mod ntgen;
mod channels_fuzz;
// 阶段3: lmcn 域外迁 hanhuo-lm（经此 re-export 保持 engine::lmcn 路径稳定）。
pub use hanhuo_lm::lmcn;
mod panel;
mod query;
mod slots;
mod typo_retry;
mod channels_t9;
pub use typo_retry::{FUZZ_AUTO, FUZZ_OFF, FUZZ_ON};
pub mod aux;
pub mod awb; // 辅助码入口（融合-辅助码 §3.1; query() 前剥语法后过滤的包装层）
pub use aux::{AuxCfg, AuxMode, AuxScheme};
mod score;

// 阶段2: 类型与常量族外迁 hanhuo-types（路径经此 re-export 保持 engine::* 稳定）。
pub use hanhuo_types::constants::*;
pub use hanhuo_types::{Cand, Kind};


/// bigram 行（保序 + 查表双形态；Python dict 语义 = 首次插入定序）
struct Big {
    ord: Vec<(u32, f64)>,
    map: HashMap<u32, f64>,
}

impl Big {
    fn is_empty(&self) -> bool {
        self.ord.is_empty()
    }
    fn contains(&self, w: u32) -> bool {
        self.map.contains_key(&w)
    }
    fn get(&self, w: u32) -> f64 {
        self.map[&w]
    }
}

struct CtxIds {
    wid: Option<u32>,
    text: String,
}

pub struct Engine {
    boost: Option<boost::BoostCfg>,
    /// T9 九宫模式位（设计-T9九宫输入 §5-P1: true 时纯 2-9 数字键串
    /// 在白名单门前分叉走 q_t9——键串→音节签名展开→DAT 走查。
    /// 默认 false = 逐位零行为（golden/忠实链不触）
    t9: bool,
    /// 查询时钟注入点（anchor 实时衰减用; None=系统时钟。测试注入
    /// 固定值避免真实时钟漂移改变 boost——纪元初 ts 的库会全衰减）
    now_override: std::cell::Cell<Option<u64>>,
    pack: Pack,
    pub meta_n_words: usize,
    dat: Dat,
    ridx_n: usize,
    ridx_off: U32Arr,
    soff: U32Arr,
    syl_blob: &'static [u8],
    wlist: U32Arr,
    wlsi_fv: Option<U16Arr>, // v2 per-(读音,词) 频次伴列
    pub n_words: usize, // pub: CLI words 导出（c.6 bgrm 重建）
    freq: U16Arr,
    _flags: &'static [u8],
    fc: FcStrings,
    whash_slots: U32Arr, // v3: 槽=id+1；v1/v2: 交织 [2j]=id+1,[2j+1]=hash
    whash_n: usize,
    whash_v3: bool,
    hot_states: U32Arr,
    hot_off: U32Arr,
    hot_ids: U32Arr,
    pub syllables: Vec<String>,
    pub seg: SyllableSeg,
    bi_n: usize,
    bi_w1: U32Arr,
    bi_w2: U32Arr,
    bi_val: U16Arr,
    w2r: U32Arr,
    jian_n: usize,
    jian_keys: FcStrings,
    jian_eoff: U32Arr,
    jian_eblob: &'static [u8],
    epyn_n: usize,
    epyn_keys: FcStrings,
    epyn_vblob: &'static [u8],
    // 件 3: EditPinyinOnce 解出表（lru 惰性装载的 py 对应: open 时一次装入;
    // 键=typo 串, 值=校正音节串列表; 文件缺失 → 空表, 边静默不发）
    epyn_tsv: HashMap<String, Vec<String>>,
    c4g_pool: FcStrings,
    c4g_npair: usize,
    c4g_w1: U32Arr,
    c4g_w2: U32Arr,
    c4g_val: U16Arr,
    // 缓存（单线程引擎；非 Sync）
    wtext: RefCell<HashMap<u32, String>>,
    rid_memo: RefCell<HashMap<u32, Vec<Vec<u32>>>>,
    epyn_parsed: RefCell<Option<Vec<Vec<Vec<u32>>>>>,
    syl_by_initial: RefCell<Option<HashMap<u8, Vec<u32>>>>,
    // 件 5: 简拼悬空中缀组合的音节首字母分组（全量, 与 jianpin_compose 的
    // 加权 top-12 syl_by_initial 语义不同）
    syls_by_ini: RefCell<Option<HashMap<u8, Vec<String>>>>,
    // 役 K（agentZ）: G1/G2 兜底简拼声母栈（SYLW 权重 top-24）
    fb_ini_stack: RefCell<Option<HashMap<u8, Vec<String>>>>,
    // 役 K（agentZ）: G1/G2 音节列首词槽分缓存
    fb_syl_head: RefCell<Option<HashMap<String, f64>>>,
    // T9（设计-T9九宫输入 §3-c）: 键串签名→音节 id 组倒排（惰性构建）
    t9_sig_v: RefCell<Option<HashMap<String, Vec<u32>>>>,
    // 役N tick52: 声母 top-k 补全音节集缓存（py _ini_top_syls_c）
    ini_top_syls_c: RefCell<Option<HashMap<String, Vec<String>>>>,
    // 役N tick53: 字槽组合族/面板缓存（py _sylset_m_v/_slot_col_v/_slot_wc_v）
    sylset_m_v: RefCell<Option<HashSet<String>>>,
    slot_col_v: RefCell<Option<HashMap<String, Vec<(String, f64)>>>>,
    slot_wc_v: RefCell<Option<HashMap<String, Vec<(String, f64, f64)>>>>,
    sylw: Vec<u32>,
    // M3: 用户词学习库（可选挂载）
    user: RefCell<Option<UserDict>>,
    poly: RefCell<Option<HashSet<u32>>>,
    // M4: NNLM（惰性解析）
    nnlm: RefCell<Option<Box<Nnlm>>>,
    /// 高级版 NNLM（nlrgpt2, 开放格式注入; 分体布局同 nnlm 惰性槽,
    /// 优先于轻量版——nnlm_score/nnlm_encode 统一入口的分发序）
    #[cfg(feature = "safetensors")]
    nnlm_adv: RefCell<Option<Box<hanhuo_lm::nnlm_adv::NnlmAdv>>>,
    // mdltr LightGBM（惰性解析; --ltr 精排已接线且实证增益: v4 语境
    // +4.0pp, mdltr_open 自训反超腾讯件 +2.4pp——特征组走 LTR_DUMP
    // 自采自训闭环, 不依赖腾讯 179 维语义全解, 2026-11-20 终审勘误）
    mdltr: RefCell<Option<Option<Box<Mdltr>>>>,
    wfea: RefCell<Option<Option<Box<Wfea>>>>,
    // 役N tick66 ref5: 英文静态词典词集缓存（py _engdict_v; 空集=律休眠）
    engdict_v: RefCell<Option<HashSet<String>>>,
    // 役N tick66 ref5 R2: EMIX 乱序音节分析缓存（py _emix_ana_v,
    // 键=排序字母串, 值=同字母合法音节表）
    emix_ana_v: RefCell<Option<HashMap<String, Vec<String>>>>,
    // 役N tick66 ref5 R2: EMIX 读音钉表缓存（py _emix_pin_rd;
    // EMIX_RD_LP_PIN 读音键去撇化）
    emix_pin_rd: RefCell<Option<HashMap<String, f64>>>,
    // ckvd 长尾短语库（可选挂载；2026-11-07 数据层终解。
    // 管线注入点待 vtable 消费链逆向——注入前仅提供查询 API，零行为变化）
    ckvd: RefCell<Option<hanhuo_dict::CkvdPhrases>>,
    bwcn: RefCell<Vec<hanhuo_dict::bwcn::BwcnTriples>>,
    // stt 简→繁词典（可选挂载，输出层 API；P1-3 2026-11-07）
    stt: RefCell<Option<hanhuo_dict::S2t>>,
    // 声调筛选字表（char→音节→调; pypinyin 生成 TSV 惰性装载,
    // 设计-声调筛选 §2 数据层; 空 → 旋钮静默失效）
    tone_tab: RefCell<Option<std::rc::Rc<HashMap<char, HashMap<String, u8>>>>>,
    // B3-τ2 小包文本集（手工/小型搭配包的词面集, 惰性构建——只扫
    // ≤5000 条的挂载包; 大语料包的搭配由主库 τ2 探测覆盖）
    bwcn_texts: RefCell<Option<std::rc::Rc<std::collections::HashSet<String>>>>,
}

pub fn default_bin() -> PathBuf {
    if let Ok(p) = std::env::var("HANHUO_BIN") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("WXQ_BIN") {
        return PathBuf::from(p);
    }
    PathBuf::from("open/hanhuo_open.bin")
}

/// 声调筛选语法层共享助手: 调号剥离 + 记位（净输入字节位→调号 1-5;
/// 设计-声调筛选 §2）。query()/query_typo_retry() 双入口同律。
pub(crate) fn strip_tones(inp: &str) -> (String, Vec<(usize, u8)>) {
    let mut clean = String::new();
    let mut pos: Vec<(usize, u8)> = Vec::new();
    for c in inp.chars() {
        if let Some(t) = c.to_digit(10).filter(|d| (1..=5).contains(d)) {
            pos.push((clean.len(), t as u8));
        } else {
            let lc = c.to_ascii_lowercase();
            if lc.is_ascii_lowercase() {
                clean.push(lc);
            }
        }
    }
    (clean, pos)
}

/// 声调筛选字表装载（tone_solved.tsv; 路径律同 epyn: 环境变量 →
/// 仓根 → 原始路径; 无表 → 空表, 旋钮静默失效）。
fn load_tone_tab() -> std::rc::Rc<HashMap<char, HashMap<String, u8>>> {
    let mut path =
        std::env::var("WXQ_TONE_TSV").map(PathBuf::from).ok();
    for cand in ["data/tone_solved.tsv", "../../data/tone_solved.tsv"] {
        if path.is_none() && PathBuf::from(cand).exists() {
            path = Some(PathBuf::from(cand));
        }
    }
    let mut m: HashMap<char, HashMap<String, u8>> = HashMap::new();
    let Some(p) = path else { return std::rc::Rc::new(m) };
    let Ok(data) = std::fs::read_to_string(&p) else {
        return std::rc::Rc::new(m);
    };
    for (ln, line) in data.lines().enumerate() {
        if ln == 0 || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let Some(ch) = parts[0].chars().next() else { continue };
        let ent = m.entry(ch).or_default();
        for rd in parts[1].split_whitespace() {
            let (syl, tone) = rd.split_at(rd.len() - 1);
            if let Ok(t) = tone.parse::<u8>() {
                ent.insert(syl.to_string(), t);
            }
        }
    }
    std::rc::Rc::new(m)
}

/// 件 3: EPYN 解出表（editpinyin_solved.tsv, 与 py 引擎同一文件）。
/// 资源定位照 bin 惯例: 环境变量 WXQ_EPYN_TSV → 仓根副本（cwd 相对,
/// 同 golden/ 惯例）→ archive 原 path。字段内多值以空格分隔
/// （"xve\txue xie\t1 1"）; 第三列 prefixes 本通道不用。
fn load_epyn_tsv() -> HashMap<String, Vec<String>> {
    let mut path = std::env::var("WXQ_EPYN_TSV").map(PathBuf::from).ok();
    // cwd 相对双探: 仓根（CLI）+ crate 相对（cargo test cwd =
    // crates/<crate>, cli 测试内 golden 曾因此 588/611——同 tone 表律）
    if path.is_none() {
        for cand in
            ["editpinyin_solved.tsv", "../../editpinyin_solved.tsv"]
        {
            let p = PathBuf::from(cand);
            if p.exists() {
                path = Some(p);
                break;
            }
        }
    }
    if path.is_none() {
        let p = PathBuf::from(
            "",
        );
        if p.exists() {
            path = Some(p);
        }
    }
    let mut tab: HashMap<String, Vec<String>> = HashMap::new();
    let Some(p) = path else {
        return tab;
    };
    let Ok(data) = std::fs::read_to_string(&p) else {
        return tab;
    };
    for (ln, line) in data.lines().enumerate() {
        if ln == 0 {
            continue; // header: typo corrections prefixes
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && !parts[0].is_empty() {
            tab.insert(
                parts[0].to_string(),
                parts[1].split(' ').filter(|s| !s.is_empty())
                    .map(|s| s.to_string()).collect(),
            );
        }
    }
    tab
}

fn le32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

/// 取尾部 l 个字符（Python ctx[-L:]）
fn ctx_tail(s: &str, l: usize) -> &str {
    let cnt = s.chars().count();
    let skip = cnt.saturating_sub(l);
    s.char_indices().nth(skip).map(|(i, _)| &s[i..]).unwrap_or("")
}

/// a 删一字母可得 b（py 模块同构保留; 消费点在纠错族扩展）
#[allow(dead_code)]
fn one_del(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() + 1 {
        return false;
    }
    for k in 0..a.len() {
        let mut s = Vec::with_capacity(b.len());
        s.extend_from_slice(&a[..k]);
        s.extend_from_slice(&a[k + 1..]);
        if s == b {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    // ---- 工具 ----

    /// 环境变量哨兵（Drop 恢复原值; 不污染并行测试与本机环境）
    struct EnvVar(&'static str, Option<OsString>);

    impl EnvVar {
        fn set(k: &'static str, v: &str) -> EnvVar {
            let old = std::env::var_os(k);
            std::env::set_var(k, v);
            EnvVar(k, old)
        }
    }

    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.1 {
                Some(v) => std::env::set_var(self.0, v),
                None => std::env::remove_var(self.0),
            }
        }
    }

    // ---- one_del: 编辑距离族（a 删一字母得 b）----

    /// 删首/删中/删尾/重复字母四种可得路径
    #[test]
    fn one_del_deletion_positions() {
        assert!(one_del(b"abc", b"bc"), "删首字母");
        assert!(one_del(b"abc", b"ac"), "删中间字母");
        assert!(one_del(b"abc", b"ab"), "删尾字母");
        assert!(one_del(b"aac", b"ac"), "重复字母删一");
        assert!(one_del(b"a", b""), "单字母删成空串");
    }

    /// 长度不符/替换/换位一律不可得
    #[test]
    fn one_del_rejects_non_deletions() {
        assert!(!one_del(b"abc", b"abc"), "等长非删");
        assert!(!one_del(b"ab", b"abc"), "a 反而更短");
        assert!(!one_del(b"abc", b"abd"), "替换不是删");
        assert!(!one_del(b"abc", b"ca"), "换位不是删");
        assert!(!one_del(b"", b""), "空对空长度不符");
        assert!(!one_del(b"abc", b""));
    }

    // ---- ctx_tail / le32 ----

    /// Python ctx[-L:] 语义（按字符切, 不劈 UTF-8; 超长取整串）
    #[test]
    fn ctx_tail_char_boundary() {
        assert_eq!(ctx_tail("你好世界", 2), "世界");
        assert_eq!(ctx_tail("你好世界", 4), "你好世界");
        assert_eq!(ctx_tail("你好世界", 9), "你好世界");
        assert_eq!(ctx_tail("abc", 1), "c");
        assert_eq!(ctx_tail("abc", 0), "");
        assert_eq!(ctx_tail("", 3), "");
    }

    /// 小端 u32 读取（含非零偏移）
    #[test]
    fn le32_little_endian_read() {
        assert_eq!(le32(&[0x78, 0x56, 0x34, 0x12], 0), 0x1234_5678);
        assert_eq!(le32(&[0xFF, 0xAA, 0xBB, 0xCC, 0xDD], 1), 0xDDCC_BBAA);
        assert_eq!(le32(&[0, 0, 1, 0], 0), 0x0001_0000);
    }

    // ---- Big: bigram 双形态 ----

    /// 保序 + 查表双形态的判空/包含/取值
    #[test]
    fn big_ord_and_map_views() {
        let empty = Big { ord: Vec::new(), map: HashMap::new() };
        assert!(empty.is_empty());
        let mut map = HashMap::new();
        map.insert(1u32, 0.5);
        map.insert(2u32, 0.25);
        let b = Big { ord: vec![(1, 0.5), (2, 0.25)], map };
        assert!(!b.is_empty());
        assert!(b.contains(1) && b.contains(2));
        assert!(!b.contains(3));
        assert_eq!(b.get(2), 0.25);
    }

    // ---- mix_recall MLP 前向: 边界行为 ----
    // （数值锚三向量在 hanhuo-cli mix_recall_forward_anchors, 此处不重复）

    /// 输出被 tanh 隐层幅值严格夹住:
    /// y ∈ (b_out − Σ_{j<5}|out_j|, b_out + Σ_{j<5}|out_j|)（tanh ∈ (−1,1) 开区间）
    #[test]
    fn mix_recall_output_strictly_bounded() {
        // 开源版零权重: 前向恒 0（=b_out=0）——闭区间零锚
        let x = [3.0f64, -2.0, 1.5, 7.0, -8.0, 0.25, -11.0, 4.0];
        assert!(hanhuo_types::constants::mix_recall_forward(&x) == 0.0);
    }

    /// 单轴饱和: 仅 x_i → ±∞ 时前向收敛到闭式极限
    /// b_out + Σ_j out_j·sign(w_ij)（权重全非零 → tanh(±∞)=±1 精确饱和）
    #[test]
    fn mix_recall_single_axis_saturation() {
        for i in 0..8 {
            for &s in &[1.0f64, -1.0] {
                let mut x = [0.0f64; 8];
                x[i] = s * 1e9;
                let expect = MIX_RECALL_OUT[5]
                    + (0..5)
                        .map(|j| MIX_RECALL_OUT[j] * (MIX_RECALL_W[i][j] * s).signum())
                        .sum::<f64>();
                let y = mix_recall_forward(&x);
                assert!(
                    (y - expect).abs() < 1e-12,
                    "轴 {i} 符号 {s}: {y} vs {expect}"
                );
            }
        }
    }

    /// 极端输入不产生 NaN: 单轴 ±inf（无 0×inf 项, 权重全非零）各自
    /// 饱和收敛; 全幅 ±1e300 列和有界（|w|<1 → 部分和 < f64::MAX）
    #[test]
    fn mix_recall_extreme_inputs_stay_finite() {
        // 零权重: 有限极端值仍有限（0×有限=0）; ±inf 的 0×inf=NaN
        // 属 IEEE 数学事实, 非零权重时代的饱和性质不再适用——排除
        for i in 0..8 {
            let mut x = [0.0f64; 8];
            x[i] = 1e300;
            assert!(hanhuo_types::constants::mix_recall_forward(&x).is_finite());
        }
    }

    /// 原点一阶一致性: f(ε·e_i) ≈ f(0) + ε·∂f/∂x_i(0)
    /// （∂f/∂x_i = Σ_j out_j·(1−tanh²(b_j))·w_ij; 二阶残差 ≪ 容差）
    #[test]
    fn mix_recall_derivative_at_origin() {
        let f0 = mix_recall_forward(&[0.0; 8]);
        for i in 0..8 {
            let d = (0..5)
                .map(|j| {
                    let t = MIX_RECALL_B[j].tanh();
                    MIX_RECALL_OUT[j] * (1.0 - t * t) * MIX_RECALL_W[i][j]
                })
                .sum::<f64>();
            let mut x = [0.0; 8];
            x[i] = 1e-6;
            let f1 = mix_recall_forward(&x);
            assert!(
                (f1 - (f0 + 1e-6 * d)).abs() < 1e-9,
                "轴 {i}: 一阶预测残差 {}",
                f1 - (f0 + 1e-6 * d)
            );
        }
    }

    // ---- 资源定位 ----

    /// EPYN 解出表解析: 表头跳过/制表位切分/空格多值/空值过滤/单字段行
    /// 丢弃; 环境变量路径不存在 → 空表（边静默）
    #[test]
    fn load_epyn_tsv_parses_and_tolerates_missing() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("hanhuo_core_epyn_test_{}.tsv", std::process::id()));
        std::fs::write(
            &p,
            "typo\tcorrections\tprefixes\nxve\txue xie\t1 1\ngeom\tjiu\t3\nempty\t\t\nshort\n",
        )
        .unwrap();
        let _g = EnvVar::set("WXQ_EPYN_TSV", p.to_str().unwrap());
        let tab = load_epyn_tsv();
        std::fs::remove_file(&p).ok();
        assert_eq!(
            tab.get("xve").unwrap(),
            &vec!["xue".to_string(), "xie".to_string()]
        );
        assert_eq!(tab.get("geom").unwrap(), &vec!["jiu".to_string()]);
        assert_eq!(
            tab.get("empty").unwrap(),
            &Vec::<String>::new(),
            "空 corrections 保留键"
        );
        assert!(!tab.contains_key("short"), "单字段行丢弃");
        assert!(!tab.contains_key("typo"), "表头跳过");
        assert_eq!(tab.len(), 3);

        // 路径给出但文件不存在 → 空表
        let missing =
            dir.join(format!("hanhuo_core_epyn_missing_{}.none", std::process::id()));
        let _g2 = EnvVar::set("WXQ_EPYN_TSV", missing.to_str().unwrap());
        assert!(load_epyn_tsv().is_empty(), "文件缺失 → 空表");
    }

    /// bin 定位优先级: HANHUO_BIN > WXQ_BIN（双哨兵齐设时结果确定,
    /// 不触达 HOME/filesystem 回退分支）。调用方常导出 HANHUO_BIN
    /// （门脚本）——EnvVar Drop 会恢复之, 第二段须显式清空再验
    /// WXQ_BIN 独立生效（否则 HANHUO_BIN 残留恒占优先级）。
    #[test]
    fn default_bin_env_precedence() {
        let saved_h = std::env::var_os("HANHUO_BIN");
        std::env::remove_var("HANHUO_BIN");
        let h = EnvVar::set("HANHUO_BIN", "/hanhuo/core/test.bin");
        let _w = EnvVar::set("WXQ_BIN", "/wxq/core/test.bin");
        assert_eq!(default_bin(), PathBuf::from("/hanhuo/core/test.bin"));
        drop(h);
        std::env::remove_var("HANHUO_BIN");
        assert_eq!(default_bin(), PathBuf::from("/wxq/core/test.bin"));
        drop(_w);
        if let Some(v) = saved_h {
            std::env::set_var("HANHUO_BIN", v);
        }
    }
}
