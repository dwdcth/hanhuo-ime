//! 输入域全覆盖真机测试（用户需求 2026-11-09）：全拼/简拼/混合/长句/
//! 短句/渐进——候选在场性 × 位次边界 × 排序单调性。
//!
//! 运行: cargo test -p hanhuo-core --test input_domains -- --ignored
//! 环境: WXQ_BIN（默认 ~/tmp/hanhuo/wxq_dict.bin）
//!
//! 数据归腾讯，仅本地个人研究使用，禁止分发。
use hanhuo_core::Engine;

fn engine() -> Engine {
    let bin = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
        format!(
            "{}/tmp/hanhuo/wxq_dict.bin",
            std::env::var("HOME").unwrap()
        )
    });
    Engine::open(std::path::Path::new(&bin)).expect("wxq_dict.bin 装载")
}

struct Case {
    /// 域标签（报告用）
    tag: &'static str,
    inp: &'static str,
    /// 期望在场的词
    expect: &'static str,
    /// 位次上界（1-based, 含）
    rank_max: usize,
    /// 期望 top1（None = 只验在场与位次）
    top1: Option<&'static str>,
    dp: bool,
    /// GAP 档: 报告不过门（当前能力缺口记账, 修复后转 strict）
    gap: bool,
    /// 整句子串匹配档: expect 是句内词面子串（长句候选=全句文本）
    sent: bool,
    /// B2.9 模糊音档: query_typo_retry 入口 + 精确词面匹配（合法错拼
    /// qijinbamai 类; 门控/合并律见 engine/typo_retry.rs）
    fuzz: bool,
}

impl Case {
    fn strict(
        tag: &'static str, inp: &'static str, expect: &'static str,
        rank_max: usize, top1: Option<&'static str>, dp: bool,
    ) -> Case {
        Case { tag, inp, expect, rank_max, top1, dp, gap: false, sent: false, fuzz: false }
    }
    fn gap(
        tag: &'static str, inp: &'static str, expect: &'static str,
        rank_max: usize,
    ) -> Case {
        Case { tag, inp, expect, rank_max, top1: None, dp: false, gap: true, sent: false, fuzz: false }
    }
    /// 整句档: dp=true + expect 子串匹配（长句候选 word=全句文本）
    fn strict_sent(
        tag: &'static str, inp: &'static str, expect: &'static str, rank_max: usize,
    ) -> Case {
        Case { tag, inp, expect, rank_max, top1: None, dp: true, gap: false, sent: true, fuzz: false }
    }
    fn gap_sent(tag: &'static str, inp: &'static str, expect: &'static str) -> Case {
        Case { tag, inp, expect, rank_max: usize::MAX, top1: None, dp: true, gap: true, sent: true, fuzz: false }
    }
    /// B2.9 模糊音档: query_typo_retry 入口, expect 精确词面匹配
    fn strict_fuzz(
        tag: &'static str, inp: &'static str, expect: &'static str,
        rank_max: usize, top1: Option<&'static str>,
    ) -> Case {
        Case { tag, inp, expect, rank_max, top1, dp: true, gap: false, sent: false, fuzz: true }
    }
}

fn cases() -> Vec<Case> {
    vec![
        // ---- 全拼单词（exact 词典直查域）----
        Case::strict("全拼词", "nihao", "你好", 1, Some("你好"), false),
        Case::strict("全拼词", "shujuku", "数据库", 1, Some("数据库"), false),
        Case::strict("全拼词", "yuxi", "玉溪", 3, None, false),
        // ---- 短句（2 词短语, 全拼）----
        Case::strict("短句", "nihaoshijie", "你好世界", 3, None, false),
        Case::strict("短句", "jintiantianqi", "今天天气", 3, None, false),
        // ---- 长句（多词整句, dp 通道）----
        Case::strict("长句", "jintiantianqibucuo", "今天天气不错", 3, Some("今天天气不错"), true),
        Case::strict("长句", "mulujiegou", "目录结构", 3, None, true),
        Case::strict("长句", "womenshiyiqizuo", "我们是一起做", 10, None, true),
        // ---- 简拼·表外键（fuzzcomp 组合域）----
        Case::strict("简拼表外", "tqsdy", "天青色等烟雨", 3, None, false),
        // ---- 简拼·表内键（B2.7 表键注入+在场提权: 表序钉 top+0.5−0.01i）----
        Case::strict("简拼表键", "tqsdyy", "天青色等烟雨", 1, Some("天青色等烟雨"), false),
        Case::strict("简拼表键", "mzd", "毛泽东", 5, None, false),
        Case::strict("简拼表键", "zgrm", "中国人民", 10, None, false),
        // ---- 混合输入（当前能力缺口记账）----
        Case::gap("混合", "tianqsdy", "天青色等烟雨", 10),
        Case::gap("混合", "tqsdyu", "天青色等烟雨", 10),
        Case::gap("混合", "jintiantq", "今天天气", 10),
        // ---- 渐进/挂起（pending 前缀语义: 可达形=悬空声母补全）----
        Case::strict("渐进", "niha", "你哈", 3, None, false),
        Case::strict("渐进", "jintiant", "今天天气", 5, None, false),
        Case::strict("渐进", "shuju", "数据", 3, None, false),
        // ---- 长句·真机语料（2026-11-09 六句实测, 词面子串匹配）----
        // 无死音节 → dp 整句通道已通
        Case::strict_sent("长句真机", "santianliangyesihaibahuangzennengbiwuhusihaidenielema", "三天两夜", 3),
        Case::strict_sent("长句真机", "nishiyoushiyoushenmeshima", "你是有", 3),
        // 死音节(gi/ging) → B2.8 变体重试已接线: #2/#4 1-编辑修复句进场
        // strict; #1 双错(j+ging = 漏 i + 错拼 g) → v1.1 2-编辑(sub×ins
        // 邻近组合)修复, 充满激情 进场 strict（rank 9, 句分同音并列下
        // 编辑局部性序择优）; #3 句进场但同音字选错(速赔够了储氢≠素胚
        // 勾勒) → 同音质量 GAP
        Case::strict_sent("长句错拼", "chongmanjgingdebasheyuanbidaodamudidigengnenggeirendailailequ", "充满激情", 10),
        Case::strict_sent("长句错拼", "cangmangdetianyashiwodeaimianyandegingshanjiaoxiahuashengkai", "苍茫的天", 5),
        Case::gap_sent("长句错拼", "supeigoulechuginghuabifengnongzhuandanpingshenmiaohuodemudan", "素胚勾勒"),
        Case::strict_sent("长句错拼", "hegangjiaodewangyoudajibawangzhewanleyigiquhuashan", "的网友", 5),
        // ---- B2.9 模糊音/前后鼻音（合法错拼: 切了但切错, 2026-11-09）----
        // 验收探针 #1: jin→jing, 奇经八脉 top5（合并律: 变体 exact 覆盖
        // 类 > 原生缝合句企金吧买, 实测 top1）
        Case::strict_fuzz("错拼合法", "qijinbamai", "奇经八脉", 5, Some("奇经八脉")),
        // 验收探针 #2（零伤害对照）: jin 是真意图, 今年 rank1 不动
        Case::strict_fuzz("错拼零伤", "jinnian", "今年", 1, Some("今年")),
        // 自造鼻音案: 双向（ins-g: xinweng→新闻/xinwenglianbo→新闻联播;
        // del-g: rengmin→人民/wengrou→温柔）+ 平翘舌二档（zidao→知道）。
        // wanluo/yinghang/chenshi 类不救: 错拼串自身有 exact 覆盖
        //（完络/英航/陈世）, 覆盖门保护, 见设计文档 §7
        Case::strict_fuzz("错拼合法", "rengmin", "人民", 5, None),
        Case::strict_fuzz("错拼合法", "wengrou", "温柔", 5, None),
        Case::strict_fuzz("错拼合法", "xinweng", "新闻", 5, None),
        Case::strict_fuzz("错拼合法", "xinwenglianbo", "新闻联播", 5, None),
        Case::strict_fuzz("错拼合法", "zidao", "知道", 5, None),
        // 模糊双读: jingnian 原生经年在场, 变体今年 进场 top3（ fuzzy
        // 双呈现, 皆合法读）
        Case::strict_fuzz("错拼双读", "jingnian", "今年", 3, None),
    ]
}

fn run(c: &Case, e: &Engine) -> Result<usize, String> {
    // 产品口径: 超越默认档（B0+尾槽+ckvd）+ nn/ltr 重排——列拼接面板
    // 本非全局降序（G1 列结构）, 排序契约由重排器视图测试单独锁定
    // sent/fuzz 档走 B2.8/B2.9 容错入口（健康=直通或模糊音门控, 死段=
    // 变体重试）; 其余档走原生 query
    let out = if c.sent || c.fuzz {
        e.query_typo_retry(c.inp, None, 30, true, true)
    } else {
        e.query(c.inp, None, false, 30, true, true, c.dp)
    };
    if out.is_empty() {
        return Err(format!("面板空"));
    }
    let rank = out
        .iter()
        .position(|x| if c.sent { x.word.contains(c.expect) } else { x.word == c.expect })
        .map(|i| i + 1)
        .ok_or_else(|| format!("「{}」不在面板", c.expect))?;
    if rank > c.rank_max {
        return Err(format!("「{}」rank {} > 上界 {}", c.expect, rank, c.rank_max));
    }
    if let Some(t) = c.top1 {
        if out[0].word != t {
            return Err(format!("top1={:?} 期望 {:?}", out[0].word, t));
        }
    }
    Ok(rank)
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin"]
fn input_domain_coverage() {
    let mut e = engine();
    e.set_boost(hanhuo_core::BoostCfg::default());
    let (mut ok, mut bad, mut gap_ok, mut gap_bad) = (0, 0, 0, 0);
    for c in cases() {
        match run(&c, &e) {
            Ok(r) => {
                println!(
                    "[{}] {:<5} {:<20} {} → rank {r}",
                    if c.gap { "gap✓" } else { "ok  " }, c.tag, c.inp, c.expect
                );
                if c.gap { gap_ok += 1 } else { ok += 1 }
            }
            Err(m) => {
                println!(
                    "[{}] {:<5} {:<20} {} → {m}",
                    if c.gap { "GAP " } else { "FAIL" }, c.tag, c.inp, c.expect
                );
                if c.gap { gap_bad += 1 } else { bad += 1 }
            }
        }
    }
    println!("== 通过 {ok} / 失败 {bad} / 缺口记账 {gap_ok}✓{gap_bad}✗");
    assert_eq!(bad, 0, "{bad} 例 strict 未过——见上方 [FAIL] 行");
}

/// 排序契约: 忠实引擎 + nn/ltr 重排 → 终面板全局分数非严格降
/// （重排器以全局 sort 收尾——本测锁该契约不被回归破坏）
#[test]
#[ignore = "真机: 需 wxq_dict.bin"]
fn reranked_panel_score_monotonicity() {
    let e = engine();
    for inp in [
        "nihao", "shujuku", "tqsdyy", "jintiantianqibucuo", "nihaoshijie",
    ] {
        let out = e.query(inp, None, false, 30, true, true, false);
        for w in out.windows(2) {
            assert!(
                w[0].score >= w[1].score - 1e-9,
                "{inp}: {:?}({:.3}) < {:?}({:.3})",
                w[0].word, w[0].score, w[1].word, w[1].score
            );
        }
    }
}
