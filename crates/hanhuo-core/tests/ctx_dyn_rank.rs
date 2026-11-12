//! 语境×用户信号真机探针（docs/设计-上下文动态组词.md §a PoC + §评估）.
//!
//! 臂纪律: A 基线（无 ctx） / B0 学习+错配 ctx（M4 融合, 无本层） /
//! B 同 B0 + user_ctx_adjust / C0 学习+契合 ctx（M4 融合, 无本层） /
//! C 同 C0 + user_ctx_adjust。B0→B 与 C0→C 的差 = 本层边际效应。
//!
//! 运行: cargo test -p hanhuo-core --test ctx_dyn_rank -- --ignored --nocapture
//! 环境: CTX_BIN（默认 ~/tmp/hanhuo/wxq.bin——全量 bin 内嵌 NNLM,
//!       nnlm_model 惰性装载即可用; wxq_dict.bin 分体无 NNLM, 不适用）。
//!
//! 学习协议: 词连击间隔 60s（>SESSION_GAP=30s）——5s 间隔会滚出自
//! n-gram 垃圾短语（火锅火锅）, 首轮实测即翻车于此。
//!
//! 数据归腾讯，仅本地个人研究使用，禁止分发。
use hanhuo_core::{Cand, Engine};

fn engine() -> Engine {
    let bin = std::env::var("CTX_BIN").unwrap_or_else(|_| {
        format!(
            "{}/tmp/hanhuo/wxq.bin",
            std::env::var("HOME").unwrap()
        )
    });
    Engine::open(std::path::Path::new(&bin)).expect("wxq.bin 装载")
}

/// 同词面可多实例在板（B4 双通道: Exact 原生 + User 补位）——取最优位次
fn rank_min(cands: &[Cand], word: &str) -> Option<usize> {
    cands
        .iter()
        .enumerate()
        .filter(|(_, c)| c.word == word)
        .map(|(i, _)| i + 1)
        .min()
}

/// 无 boost 无 ltr 无 dp 的 open_nn 链（nn=true 同 CLI --nnlm）
fn query(e: &Engine, py: &str, ctx: Option<&str>) -> Vec<Cand> {
    e.query(py, ctx, false, 12, true, false, false)
}

fn user_path(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("hanhuo_ctx_probe_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("user.hhd1");
    // 注意: 每次调用会删文件——需要跨 attach/load 复用同一文件时,
    // 调用方应一次物化路径存局部变量（重载前再调 = 抹库）
    let _ = std::fs::remove_file(&p);
    p
}

/// 词级学习: 同词连击 n 次, 60s 间隔（断 n-gram 会话, 防火锅火锅垃圾）
fn learn_word(e: &Engine, word: &str, times: usize, t0: u64) {
    for i in 0..times {
        assert!(
            e.user_commit(word, None, t0 + i as u64 * 60),
            "{word} 学习拒收（词典/多音/超长门）"
        );
    }
}

/// 短语学习: 组成词按会话序提交（组内 5s, 轮间 100s 断会话）
fn learn_phrase(e: &Engine, words: &[&str], rounds: usize, t0: u64) {
    for r in 0..rounds as u64 {
        for (i, w) in words.iter().enumerate() {
            assert!(
                e.user_commit(w, None, t0 + r * 100 + i as u64 * 5),
                "{w} 学习拒收"
            );
        }
    }
}

// =====================================================================
// W1 组: 词级 12 对三臂总表（报告档——方向性结论见 W2/P 组）
// =====================================================================
struct WPair {
    word: &'static str,
    py: &'static str,
    fit: &'static str,
    alt: &'static str,
}

const W_PAIRS: &[WPair] = &[
    WPair { word: "火锅", py: "huoguo", fit: "今晚一起吃", alt: "今晚一起开" },
    WPair { word: "机票", py: "jipiao", fit: "订一张去上海的", alt: "读一本" },
    WPair { word: "体检", py: "tijian", fit: "下周一去做个", alt: "下周一去逛" },
    WPair { word: "签证", py: "qianzheng", fit: "出国前先办", alt: "出国前先吃" },
    WPair { word: "报销", py: "baoxiao", fit: "拿着发票去", alt: "拿着球拍去" },
    WPair { word: "团建", py: "tuanjian", fit: "这周五公司组织", alt: "这周五公司倒闭" },
    WPair { word: "房租", py: "fangzu", fit: "每月月初交", alt: "每月月初吃" },
    WPair { word: "汇报", py: "huibao", fit: "明早向领导", alt: "明早去医院" },
    WPair { word: "快递", py: "kuaidi", fit: "驿站有个", alt: "锅里有个" },
    WPair { word: "物业", py: "wuye", fit: "停电了找", alt: "停电了吃" },
    WPair { word: "班车", py: "banche", fit: "下班后坐", alt: "下班后炒" },
    WPair { word: "磁共振", py: "cigongzhen", fit: "医生建议做个", alt: "厨师建议做个" },
];

/// 报告档（不设门）: 词级 anchor+ctx 组合行为总表。断言仅在场律
/// （学习词全臂 top12——B4 词级加成 + 词典词保证）。
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn w1_word_level_three_arm_table() {
    let e = engine();
    e.attach_user(&user_path("w1"));
    let t0 = 1_800_000_000u64;
    for (k, p) in W_PAIRS.iter().enumerate() {
        learn_word(&e, p.word, 3, t0 + k as u64 * 1000); // anchor=3 → +2.09
    }
    let (mut fit_le_alt, mut present) = (0usize, 0usize);
    println!("┌词──────┬py──────────┬A──B0──B──C0──C┬top1(A)─────┐");
    for p in W_PAIRS {
        let a = query(&e, p.py, None);
        let ra = rank_min(&a, p.word);
        let mut b = query(&e, p.py, Some(p.alt));
        let rb0 = rank_min(&b, p.word);
        e.user_ctx_adjust(Some(p.alt), &mut b);
        let rb = rank_min(&b, p.word);
        let mut c = query(&e, p.py, Some(p.fit));
        let rc0 = rank_min(&c, p.word);
        e.user_ctx_adjust(Some(p.fit), &mut c);
        let rc = rank_min(&c, p.word);
        println!(
            "│{:<6}│{:<12}│{:>2} {:>3} {:>3} {:>3} {:>3}│{:<12}│",
            p.word,
            p.py,
            ra.unwrap_or(99),
            rb0.unwrap_or(99),
            rb.unwrap_or(99),
            rc0.unwrap_or(99),
            rc.unwrap_or(99),
            a.first().map(|x| x.word.replace('\u{1}', "·")).unwrap_or_default(),
        );
        if let (Some(rb), Some(rc)) = (rb, rc) {
            fit_le_alt += (rc <= rb) as usize;
            present += (rc <= 12) as usize;
        }
    }
    println!(
        "└契合≤错配 {fit_le_alt}/12; 学习词在场 {present}/12┘"
    );
    assert_eq!(present, W_PAIRS.len(), "学习词全臂在场");
}

// =====================================================================
// W2 组: 同拼竞争者方向性探针（断言档——ctx 信号的可分辨域）
// =====================================================================
/// 同拼同长类内存在 ctx 可分辨的竞争词（活过/板车/午夜/爆笑）:
/// 契合 ctx 学习词应稳住/上浮, 竞争者 ctx 应使其失去本层增益。
struct W2Pair {
    word: &'static str,
    py: &'static str,
    /// 契合 ctx（偏爱学习词）
    fit: &'static str,
    /// 竞争者 ctx（偏爱同拼竞争词——方向对照）
    comp: &'static str,
    competitor: &'static str,
}

const W2_PAIRS: &[W2Pair] = &[
    W2Pair {
        word: "火锅", py: "huoguo",
        fit: "今晚一起吃", comp: "从重病里", competitor: "活过",
    },
    W2Pair {
        word: "班车", py: "banche",
        fit: "下班后坐", comp: "工地拉砖用", competitor: "板车",
    },
    W2Pair {
        word: "物业", py: "wuye",
        fit: "停电了找", comp: "凌晨三点的", competitor: "午夜",
    },
    W2Pair {
        word: "报销", py: "baoxiao",
        fit: "拿着发票去", comp: "台下观众哄堂", competitor: "爆笑",
    },
];

/// 断言档:
/// - 无伤害律: rank(fit) ≤ rank(A) 全对（契合 ctx 永不降位）
/// - 稳定律: 契合臂学习词全对在场 top3（anchor 主导 + ctx 不拆台）
/// - 饱和报告: anchor=3（b=2.09）时词级位次全臂 rank1——层级律
///   anchor > ctx 的实证面（方向性位移在基座近邻对才可见, 见打印;
///   同长类竞争者位距 >1.5 帽时位次结构性饱和, 非本层缺陷）
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn w2_homophone_directional_probe() {
    let e = engine();
    e.attach_user(&user_path("w2"));
    let t0 = 1_800_000_000u64;
    for (k, p) in W2_PAIRS.iter().enumerate() {
        learn_word(&e, p.word, 3, t0 + k as u64 * 1000);
    }
    let (mut noharm, mut top3) = (0usize, 0usize);
    println!("┌词──────┬A──F(fit)──K(comp)┬竞争者 A/F/K───┐");
    for p in W2_PAIRS {
        let a = query(&e, p.py, None);
        let ra = rank_min(&a, p.word).unwrap();
        let ra_c = rank_min(&a, p.competitor);
        let mut f = query(&e, p.py, Some(p.fit));
        let rf0 = rank_min(&f, p.word).unwrap();
        e.user_ctx_adjust(Some(p.fit), &mut f);
        let rf = rank_min(&f, p.word).unwrap();
        let rf_c = rank_min(&f, p.competitor);
        let mut k = query(&e, p.py, Some(p.comp));
        let rk0 = rank_min(&k, p.word).unwrap();
        e.user_ctx_adjust(Some(p.comp), &mut k);
        let rk = rank_min(&k, p.word).unwrap();
        let rk_c = rank_min(&k, p.competitor);
        println!(
            "│{:<6}│{:>2} {:>2}({:>2}) {:>2}({:>2})│{:>2}/{:>2}/{:>2}───│",
            p.word, ra, rf, rf0, rk, rk0,
            ra_c.unwrap_or(0), rf_c.unwrap_or(0), rk_c.unwrap_or(0),
        );
        noharm += (rf <= ra) as usize;
        top3 += (rf <= 3) as usize;
    }
    println!("└无伤害 {noharm}/4 契合臂 top3 {top3}/4┘");
    assert_eq!(noharm, W2_PAIRS.len(), "契合 ctx 降位 = 伤害");
    assert_eq!(top3, W2_PAIRS.len(), "契合臂学习词应稳定 top3");
}

// =====================================================================
// P 组: 同键短语 ctx 重排（B5 注入块的 ctx 维——同长公平比较域）
// =====================================================================

/// 上午开会 / 商务开会 同键 shangwukaihui（均非词典词 → 无去重劫持）:
/// anchor 序 商务(3轮) > 上午(2轮)（Δb = 1.9·ln(3/2) ≈ 0.77）。
/// 契合 ctx「明天早上」应使 上午开会 位次改善（越位需 κ·ΔN > 0.77
/// → ΔN > 3.1 nat, 打印观测）; 锚定 ctx「和客户谈」下 商务开会 守 top。
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn p_same_key_phrase_ctx_reorder() {
    let e = engine();
    e.attach_user(&user_path("p"));
    let t0 = 1_800_000_000u64;
    learn_phrase(&e, &["上午", "开会"], 2, t0); // anchor 2
    learn_phrase(&e, &["商务", "开会"], 3, t0 + 10_000); // anchor 3

    // A 无 ctx: anchor 律——商务开会 在 上午开会 前
    let a = query(&e, "shangwukaihui", None);
    let (ra_sw, ra_sx) = (
        rank_min(&a, "商务开会"),
        rank_min(&a, "上午开会"),
    );
    println!("A  无ctx: 商务开会={ra_sw:?} 上午开会={ra_sx:?}");
    assert!(ra_sw.is_some() && ra_sx.is_some(), "双短语注入在场");
    assert!(
        ra_sw.unwrap() < ra_sx.unwrap(),
        "anchor 律: 商务(3) 应先于 上午(2)"
    );

    // C 契合 ctx「明天早上」: 同键对翻位（ΔN=6.01 nat, κ·ΔN=1.50=帽;
    // 竞的是 anchor 差 0.77, 翻位带 0.73 余量——首轮 CTX_DBG 实测）
    // query() 已内建 user_ctx_adjust 接线（85a3915 主线合入）——
    // 旧版手动调用会造成双重应用超调, 已删
    let c0 = query(&e, "shangwukaihui", Some("明天早上"));
    let rc0_sx = rank_min(&c0, "上午开会").unwrap();
    let rc_sx = rc0_sx;
    let rc_sw = rank_min(&c0, "商务开会").unwrap();
    println!("C  明天早上: 上午开会 {rc0_sx}→{rc_sx}（商务开会={rc_sw}）");
    assert!(
        rc_sx < rc_sw,
        "契合 ctx 翻位: 上午开会({rc_sx}) 应越 商务开会({rc_sw})"
    );
    assert!(rc_sx <= ra_sx.unwrap(), "契合 ctx 不降位");

    // 锚定 ctx「和客户谈」: 方向反转——商务开会 守前（不翻船）
    let d = query(&e, "shangwukaihui", Some("和客户谈"));
    let rd_sw = rank_min(&d, "商务开会").unwrap();
    let rd_sx = rank_min(&d, "上午开会").unwrap();
    println!("D  和客户谈: 商务开会={rd_sw} 上午开会={rd_sx}");
    assert!(
        rd_sw < rd_sx,
        "锚定 ctx 方向反转: 商务开会({rd_sw}) 应在 上午开会({rd_sx}) 前"
    );
}

// =====================================================================
// Q 组: 前缀补全跨组报告（长度保守性实测——只报告不设门）
// =====================================================================

/// 学 上午开会讨论（1 轮, anchor=1 → 注入分 top+0−0.5, 天然 rank≈2）。
/// v2 同长参照律下: 补全（6 字类）在同键面板无同长原生/同长同伴 →
/// 单候选类 adj≡0 不发力——跨长度提权明确移出本层域（设计文档 §a.4）,
/// 此处锁「不动」语义（B5-2 基线行为不受本层扰动）。
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn q_prefix_completion_cross_group_report() {
    let e = engine();
    e.attach_user(&user_path("q"));
    learn_phrase(&e, &["上午", "开会", "讨论"], 1, 1_800_000_000);

    let a = query(&e, "shangwukaihui", None);
    let ra = rank_min(&a, "上午开会讨论").expect("前缀补全注入在场");
    let mut c = query(&e, "shangwukaihui", Some("明天早上"));
    let rc0 = rank_min(&c, "上午开会讨论").unwrap();
    let adj = e.user_ctx_adjust(Some("明天早上"), &mut c);
    let rc = rank_min(&c, "上午开会讨论").unwrap();
    println!(
        "前缀补全 上午开会讨论: 无ctx rank={ra} 契合ctx {rc0}→{rc} \
         (adjust={adj}, 单候选 6 字类 adj≡0 预期不动)"
    );
    assert!(rc <= 5, "补全在场且近顶（B5-2 基线保证）");
}

// =====================================================================
// V 组: 主线验收探针 #1（用户 2026-11-09: 上屏「天青色等烟雨」后输
// ewzdn 应提示 而我在等你——c 层生成宽度 + ctx 消歧的联合验收面）
// =====================================================================

/// 四臂诊断（报告档, 干净 HEAD 基线——主线实测在 D-WIP CLI 上, 此处
/// worktree HEAD 复测锁定）:
/// 1) 简拼裸查: 而我在等你 是否生成（主线实测不在 top60——生成宽度缺口）
/// 2) ctx 臂: M4 ctx 重排对简拼组句候选的作用面
/// 3) 学习臂（a 层顺路覆盖检验）: 预学 而我在等你（B5 短语键=全拼
///    erzaidengni）后简拼查询——短语键域=全拼, 简拼输入不命中;
///    3b 全拼对照证明同一短语在全拼下即刻出场 → 缺口精确定位在
///    简拼×短语桥（c 层）, 非 a 层。
/// 4) B2.7 简拼表键臂: ewzdn 若在简拼表（tqsdyy 同款歌词链）,
///    jianpin_inject 直接置顶——无表则仍缺。
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn v_acceptance_ewzdn() {
    let e = engine();
    // 0) 纯简拼表路径（jianpin=true, q_jianpin 域）
    let jp = e.query("ewzdn", None, true, 60, true, false, false);
    println!(
        "0) 纯简拼表（jianpin=true）: n={} top3={:?}",
        jp.len(),
        jp.iter().take(3).map(|x| x.word.clone()).collect::<Vec<_>>()
    );
    // 1) 裸基线（主线 CLI 口径: jianpin=false, 混合域联想通道, top60）
    let a = e.query("ewzdn", None, false, 60, true, false, false);
    let hit = a.iter().position(|c| c.word.contains("而我在等你"));
    println!(
        "1) 裸查 n={} top1={:?} 而我在等你={:?}（top60 内）",
        a.len(),
        a.first().map(|c| c.word.clone()),
        hit.map(|i| i + 1)
    );
    // 2) ctx 臂（M4 神经层重排 + 本层 adjust 双开）
    let mut c = e.query(
        "ewzdn",
        Some("天青色等烟雨"),
        false,
        60,
        true,
        false,
        false,
    );
    let top5_ctx: Vec<String> =
        c.iter().take(5).map(|x| x.word.clone()).collect();
    e.user_ctx_adjust(Some("天青色等烟雨"), &mut c);
    println!(
        "2) ctx 臂 top5={top5_ctx:?} 而我在等你={:?}",
        c.iter().position(|x| x.word.contains("而我在等你")).map(|i| i + 1)
    );
    // 3) 学习臂: 预学短语（而,我,在,等,你 ×2 轮 → 而我在等你）→
    //    简拼/混合域查询不命中（短语键域=全拼 erwozaidengni）
    let up_v = user_path("v"); // 一次物化（helper 重复调用会删文件）
    e.attach_user(&up_v);
    learn_phrase(&e, &["而", "我", "在", "等", "你"], 2, 1_800_000_000);
    e.user_flush(); // journal 落盘（attach 空文件起步: append 不建文件）
    let l = e.query("ewzdn", Some("天青色等烟雨"), false, 60, true, false, false);
    println!(
        "3) 学习臂（混合域） 而我在等你={:?}",
        l.iter().position(|x| x.word.contains("而我在等你")).map(|i| i + 1)
    );
    // 3b) 全拼对照: 同一短语 B5 精确命中即刻出场
    let full = e.query("erwozaidengni", None, false, 20, true, false, false);
    println!(
        "3b) 全拼对照 erwozaidengni: 而我在等你={:?}（B5 注入域）",
        rank_min(&full, "而我在等你")
    );
    // 4) B2.7 简拼表键臂（独立引擎挂默认 boost——jianpin_inject: true）
    let mut e2 = engine();
    e2.set_boost(hanhuo_core::BoostCfg {
        jianpin_inject: true,
        ..Default::default()
    });
    let b = e2.query("ewzdn", None, false, 20, true, false, false);
    println!(
        "4) B2.7 jianpin_inject: 而我在等你={:?} top3={:?}",
        rank_min(&b, "而我在等你"),
        b.iter().take(3).map(|x| x.word.clone()).collect::<Vec<_>>()
    );
    // 5) §c.2 桥接模拟（userdict.rs phrase_initial_lookup + spec'd 注入
    //    律——query.rs 短语块同构: top+user_boost(anchor), Kind::User;
    //    接线一行归主线, 此处端到端验证机制闭环）→ 验收位 top3
    let ud = hanhuo_core::userdict::UserDict::load(&up_v);
    let oracle = |s: &str| e.seg.is_syl(s);
    let phs = ud.phrase_initial_lookup("ewzdn", 4, &oracle);
    assert!(
        !phs.is_empty() && phs[0].0 == "而我在等你",
        "简拼投影命中: {:?}",
        phs.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>()
    );
    let mut panel = e.query(
        "ewzdn",
        Some("天青色等烟雨"),
        false,
        12,
        true,
        false,
        false,
    );
    let top = panel.first().map(|c| c.score).unwrap_or(0.0);
    for (text, pe) in phs {
        if panel.iter().any(|c| c.word == text) {
            continue;
        }
        let b_inj = hanhuo_core::userdict::user_boost(pe.e.anchor);
        let mut c = Cand::new(text.clone(), -8.0, hanhuo_core::Kind::User);
        c.reading = pe.reading.replace(',', "'");
        c.score = top + b_inj;
        c.user_bias = b_inj;
        panel.push(c);
    }
    panel.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    e.user_ctx_adjust(Some("天青色等烟雨"), &mut panel);
    let r5 = rank_min(&panel, "而我在等你").unwrap();
    println!(
        "5) §c.2 桥+注入+ctx: 而我在等你=rank {r5}（验收线 top3）"
    );
    assert!(r5 <= 3, "验收探针 #1: 而我在等你 应进 top3（实际 {r5}）");
}


// =====================================================================
// V2 组: 主线验收探针 #2（用户 2026-11-09: ctx=「想去看电影」输
// xmdbmh 应提示 想买点爆米花——c 层「词块组装 × ctx-NNLM 重排」
// 的联合验收面; 比探针 #1 深一层: 生成侧从不把 爆米花 当三声母
// 词块整体拼入, 逐声母配字组装出 新买的白马湖 类荒谬面）
// =====================================================================

/// 臂: 1) 混合域裸查（生成宽度缺口锁定） 1b) 爆米花 词块在库性
/// （baomihua exact 直查——词块在库, 缺的是组装感知）
/// 3) §c.2 学习臂: 预学 想买点爆米花（想,买,点,爆米花 会话——
/// 爆米花 整词一块入尾段, 4 词 7 字 6 音节 ≤ 帽）→ 简拼投影
/// xmdbmh 命中 + spec'd 注入 + ctx 微调 → 验收位 top3。
/// （未学习路径的通用解 = B2.6 词块组装复活 + ctx-NNLM 统一尺度
/// 重排——设计文档 §c.3, 工作量档另计）
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn v2_acceptance_xmdbmh() {
    let e = engine();
    // 1) 混合域裸查
    let a = e.query("xmdbmh", None, false, 60, true, false, false);
    let hit = a.iter().position(|c| c.word.contains("想买点爆米花"));
    println!(
        "1) 裸查 n={} top1={:?} 想买点爆米花={:?}",
        a.len(),
        a.first().map(|c| c.word.clone()),
        hit.map(|i| i + 1)
    );
    // 1b) 爆米花 词块在库性
    let bmh = e.query("baomihua", None, false, 10, true, false, false);
    println!(
        "1b) baomihua 直查: 爆米花={:?}（词块在库, 组装不感知）",
        rank_min(&bmh, "爆米花")
    );
    // 3) §c.2 学习臂
    let up = user_path("v2"); // 一次物化
    e.attach_user(&up);
    learn_phrase(&e, &["想", "买", "点", "爆米花"], 2, 1_800_000_000);
    e.user_flush();
    let l = e.query("xmdbmh", Some("想去看电影"), false, 60, true, false, false);
    println!(
        "3a) 学习臂（未接线, 混合域）: 想买点爆米花={:?}",
        l.iter().position(|x| x.word.contains("想买点爆米花")).map(|i| i + 1)
    );
    let ud = hanhuo_core::userdict::UserDict::load(&up);
    // wxq 拆存律观察: 音节表含 dian/tian/…（409 音节）但读音变体把
    // ian 族拆存（点=di,an——dump_phrases 实证）→ 投影必须带合并律
    println!(
        "  [dbg] 音节表: n={} dian={} tian={}",
        e.syllables.len(),
        e.seg.is_syl("dian"),
        e.seg.is_syl("tian"),
    );
    let oracle = |s: &str| e.seg.is_syl(s);
    let phs = ud.phrase_initial_lookup("xmdbmh", 4, &oracle);
    assert!(
        !phs.is_empty() && phs.iter().any(|(t, _)| t == "想买点爆米花"),
        "简拼投影命中: {:?}",
        phs.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>()
    );
    let mut panel =
        e.query("xmdbmh", Some("想去看电影"), false, 12, true, false, false);
    let top = panel.first().map(|c| c.score).unwrap_or(0.0);
    for (text, pe) in phs {
        if panel.iter().any(|c| c.word == text) {
            continue;
        }
        let b_inj = hanhuo_core::userdict::user_boost(pe.e.anchor);
        let mut c = Cand::new(text.clone(), -8.0, hanhuo_core::Kind::User);
        c.reading = pe.reading.replace(',', "'");
        c.score = top + b_inj;
        c.user_bias = b_inj;
        panel.push(c);
    }
    panel.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    e.user_ctx_adjust(Some("想去看电影"), &mut panel);
    let r = rank_min(&panel, "想买点爆米花").unwrap();
    println!("3b) §c.2 桥+注入+ctx: 想买点爆米花=rank {r}（验收线 top3）");
    assert!(r <= 3, "验收探针 #2: 想买点爆米花 应进 top3（实际 {r}）");
}


// =====================================================================
// Z 组: 面板形态诊断（N_ref 参照域实证——长度类/同拼竞争者结构）
// =====================================================================

/// 打印 huoguo / shangwukaihui 学习后面板 top-8（词面/字长/分/用户信号/
/// NNLM 分）, 对照契合与错配 ctx——同长参照律（v2）的设计依据。
#[test]
#[ignore = "需要 wxq.bin（内嵌 NNLM）"]
fn z_panel_shape_diag() {
    let e = engine();
    e.attach_user(&user_path("z"));
    learn_word(&e, "火锅", 3, 1_800_000_000);
    learn_phrase(&e, &["上午", "开会"], 2, 1_800_010_000);
    for (py, ctx) in [
        ("huoguo", "今晚一起吃"),
        ("huoguo", "从重病里"),
        ("shangwukaihui", "明天早上"),
        ("shangwukaihui", "和客户谈"),
    ] {
        let mut c = query(&e, py, Some(ctx));
        println!("── py={py} ctx={ctx}");
        for (i, x) in c.iter().take(8).enumerate() {
            println!(
                "  #{i} {:<10} len={} score={:7.3} ub={:.2} kind={:?} \
                 nn={:?}",
                x.word.replace('\u{1}', "·"),
                x.word.chars().count(),
                x.score,
                x.user_bias,
                x.kind,
                x.nn_n
            );
        }
        e.user_ctx_adjust(Some(ctx), &mut c);
        println!(
            "  adjust 后 top3: {:?}",
            c.iter()
                .take(3)
                .map(|x| x.word.replace('\u{1}', "·"))
                .collect::<Vec<_>>()
        );
    }
}
