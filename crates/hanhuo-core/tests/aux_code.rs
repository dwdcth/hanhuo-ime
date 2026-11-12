//! 辅助码 PoC 测试（融合-辅助码-万象 §4 验收; 覆盖 engine::aux + dict::auxform）
//!
//! 运行: cargo test -p hanhuo-core --test aux_code            （免数据纯语法律）
//!       cargo test -p hanhuo-core --test aux_code -- --ignored （真机编排律）
//! 环境: WXQ_BIN（默认 ~/tmp/hanhuo/wxq_dict.bin）
//!       WUBI_YAML / RSD_TSV（默认 ~/tmp/hanhuo/ 下, 见 real_form）
//!
//! 数据归腾讯（wxq/wubi86_swbd/rsd）, 仅本地个人研究使用, 禁止分发;
//! 码表资产走本机路径, 测试文件不含数据。
//!
//! 独立集成 target 说明: lib 内联测试目标当前被 boost.rs B7 测试态
//! 编译缺口（f8299ee 漏补 sentence_nnlm_rescore 字段, agent D 领地,
//! 本役未触碰）阻断, aux 测试故落本文件——只编译 lib 本体, 可独立运行。
use hanhuo_core::Engine;
use hanhuo_core::engine::aux::{
    AuxCfg, AuxMode, AuxParse, AuxScheme, cand_hit, is_subseq, parse_aux,
};
use hanhuo_dict::AuxForm;

// ------------------------------------------------------------------
// 有序子序列匹配器六律（§4.2）
// ------------------------------------------------------------------

#[test]
fn subseq_exact() {
    assert!(is_subseq(b"qf", b"qf"));
    assert!(is_subseq(b"pspzspd", b"pspzspd"), "等长全等");
}

#[test]
fn subseq_skip_jump() {
    // 跨位跳跃律: 查询串可跳过码中间位
    assert!(is_subseq(b"qf", b"qfhw"));
    assert!(is_subseq(b"pd", b"pspzspd"), "首末跳中");
}

#[test]
fn subseq_order_matters() {
    // 顺序不反律: fq 不是 qfhw 的子序列
    assert!(!is_subseq(b"fq", b"qfhw"));
    assert!(!is_subseq(b"qp", b"pq"));
}

#[test]
fn subseq_empty_and_overflow() {
    // 空查询恒真（纯声调/纯聚拢语法位）; 空码非空查询恒假;
    // 查询长于码恒假
    assert!(is_subseq(b"", b"qf"));
    assert!(is_subseq(b"", b""));
    assert!(!is_subseq(b"q", b""));
    assert!(!is_subseq(b"qfh", b"qf"));
    assert!(!is_subseq(b"pspzspde", b"pspzspd"));
}

#[test]
fn subseq_single_letter_jianma() {
    // 单键简码兼容律: j ⊆ jg（是）——一级简码作短查询仍命中
    assert!(is_subseq(b"j", b"jg"));
    assert!(is_subseq(b"w", b"wq"));
}

#[test]
fn subseq_stroke_anchor_ni() {
    // 万象实证锚: 你 = 3235234 → pspzspd, 逐位吻合
    assert!(is_subseq(b"pspzspd", b"pspzspd"));
    assert!(is_subseq(b"psp", b"pspzspd"));
    assert!(!is_subseq(b"dp", b"pspzspd"), "逆序不命中（d 在 p 后）");
}

// ------------------------------------------------------------------
// 解析语法律（§3.1 P0）
// ------------------------------------------------------------------

#[test]
fn parse_no_leader_is_none() {
    assert_eq!(parse_aux("nihao", '`'), AuxParse::None);
    assert_eq!(parse_aux("", '`'), AuxParse::None);
    assert_eq!(parse_aux("zhen/qf", '`'), AuxParse::None, "缺省引导符下 / 非语法");
}

#[test]
fn parse_basic_split() {
    let AuxParse::Ok(s) = parse_aux("zhen`qf", '`') else {
        panic!("zhen`qf 应有效")
    };
    assert_eq!(s.pinyin, "zhen");
    assert_eq!(s.code_q, "qf");
    assert_eq!(s.tone, None);
    assert!(!s.gather);
}

#[test]
fn parse_custom_leader() {
    let AuxParse::Ok(s) = parse_aux("zhen/qf", '/') else {
        panic!("自定义引导符 / 应有效")
    };
    assert_eq!(s.pinyin, "zhen");
    assert_eq!(s.code_q, "qf");
}

#[test]
fn parse_tone_free_order_keep_last() {
    // 乱序自由: 字母数字任意插位; 连续数字只保留最后一位（万象律）
    let AuxParse::Ok(s) = parse_aux("ni`w9q", '`') else {
        panic!("乱序声调应可解析"
        )
    };
    assert_eq!(s.code_q, "wq");
    assert_eq!(s.tone, Some(3));
    let AuxParse::Ok(s2) = parse_aux("ni`w7890", '`') else {
        panic!("连续声调应可解析")
    };
    assert_eq!(s2.code_q, "w");
    assert_eq!(s2.tone, Some(4), "0=声调4 且只留最后一位");
    let AuxParse::Ok(s3) = parse_aux("ni`wq9", '`') else {
        panic!("尾声调应可解析")
    };
    assert_eq!(s3.tone, Some(3));
}

#[test]
fn parse_gather_trailing_slash() {
    let AuxParse::Ok(s) = parse_aux("zhen`qf/", '`') else {
        panic!("聚拢尾缀应可解析")
    };
    assert!(s.gather);
    assert_eq!(s.code_q, "qf");
    assert_eq!(parse_aux("zhen`q/f", '`'), AuxParse::Invalid, "/ 非尾位");
    let AuxParse::Ok(s2) = parse_aux("zhen`/", '`') else {
        panic!("纯聚拢应可解析")
    };
    assert!(s2.gather);
    assert_eq!(s2.code_q, "");
}

#[test]
fn parse_invalid_forms() {
    // 空拼音 / 1-6 数字 / 大写锁定（§3.1 裁定不做）→ 无效 = 原生阻断同像
    assert_eq!(parse_aux("`qf", '`'), AuxParse::Invalid);
    assert_eq!(parse_aux("zhen`q5", '`'), AuxParse::Invalid);
    assert_eq!(parse_aux("zhen`QF", '`'), AuxParse::Invalid);
    assert_eq!(parse_aux("zhen`q-f", '`'), AuxParse::Invalid);
    assert_eq!(parse_aux("zhen`q`f", '`'), AuxParse::Invalid, "双引导符");
    assert_eq!(parse_aux("Zhen`qf", '`'), AuxParse::Invalid, "拼音段大写");
}

#[test]
fn parse_pinyin_apostrophe_and_space() {
    // 撇号/空格 = 原生白名单（音节分隔/软分隔, query 内剥离）
    let AuxParse::Ok(s) = parse_aux("xi'an`jt", '`') else {
        panic!("撇号拼音段应有效")
    };
    assert_eq!(s.pinyin, "xi'an");
    let AuxParse::Ok(s2) = parse_aux("zhen hua`jt", '`') else {
        panic!("空格拼音段应有效")
    };
    assert_eq!(s2.pinyin, "zhen hua");
}

// ------------------------------------------------------------------
// cand_hit 判据律（合成码表, 免数据）
// ------------------------------------------------------------------

fn synth_form(dir: &std::path::Path) -> AuxForm {
    let wubi = dir.join("mini.dict.yaml");
    std::fs::write(
        &wubi,
        "# t\n...\n镇\tqfhw\n真\tfhwv\n这\tp\n这\typi\n是\tj\n是\tjgh\n\
         我\ttrnt\n你\twq\n你\twqiy\n的\tr\n的\trqyy\n定\tpgh\n\
         镇定\tqfpg\n我们\ttrwu\n",
    )
    .unwrap();
    let rsd = dir.join("mini_strokes.tsv");
    std::fs::write(&rsd, "# t\nchar\tcp\tstrokes\n我\t0x6211\t3121534\n").unwrap();
    AuxForm::from_file(&wubi, &rsd).unwrap()
}

fn tmpdir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir()
        .join(format!("hanhuo_auxcode_{}_{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn hit_first_or_last_char() {
    let d = tmpdir("hit");
    let f = synth_form(&d);
    // 单字首=末: 镇(qfhw→qf)
    assert!(cand_hit(&f, AuxScheme::Wubi2, b"qf", "镇", false));
    // 词组首字路径: 镇定 的 镇
    assert!(cand_hit(&f, AuxScheme::Wubi2, b"qf", "镇定", false));
    // 词组末字路径: 镇真 的 真(fhwv→fh)
    assert!(cand_hit(&f, AuxScheme::Wubi2, b"fh", "镇真", false));
    // 我们: 我(trnt→tr) 首 + 们(缺码)
    assert!(cand_hit(&f, AuxScheme::Wubi2, b"tr", "我们", false));
    // 不命中
    assert!(!cand_hit(&f, AuxScheme::Wubi2, b"zz", "镇定", false));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn hit_scheme_variants() {
    let d = tmpdir("scheme");
    let f = synth_form(&d);
    // HeadTail: 镇 qfhw→qw
    assert!(cand_hit(&f, AuxScheme::WubiHeadTail, b"qw", "镇", false));
    assert!(!cand_hit(&f, AuxScheme::WubiHeadTail, b"qf", "镇", false));
    // Strokes: 我 3121534→phshzpd（3撇1横2竖1横5折3撇4捺）
    assert_eq!(f.strokes('我'), Some("phshzpd"));
    assert!(cand_hit(&f, AuxScheme::Strokes, b"phshzpd", "我", false));
    assert!(cand_hit(&f, AuxScheme::Strokes, b"ph", "我", false), "前缀子序列");
    // 缺笔顺字（镇 不在 mini 笔顺表）→ 该侧不命中
    assert!(!cand_hit(&f, AuxScheme::Strokes, b"ph", "镇", false));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn hit_gather_single_char_only() {
    let d = tmpdir("gather");
    let f = synth_form(&d);
    assert!(cand_hit(&f, AuxScheme::Wubi2, b"qf", "镇", true));
    assert!(
        !cand_hit(&f, AuxScheme::Wubi2, b"qf", "镇定", true),
        "聚拢丢词组"
    );
    // 纯聚拢（空查询）: 仅按单字 retain
    assert!(cand_hit(&f, AuxScheme::Wubi2, b"", "真", true));
    assert!(!cand_hit(&f, AuxScheme::Wubi2, b"", "镇定", true));
    std::fs::remove_dir_all(&d).ok();
}

// ------------------------------------------------------------------
// 真机编排律（需 wxq_dict.bin + 本地码表资产, #[ignore] 惯例）
// ------------------------------------------------------------------

fn engine() -> Engine {
    let bin = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
        format!(
            "{}/tmp/hanhuo/wxq_dict.bin",
            std::env::var("HOME").unwrap()
        )
    });
    Engine::open(std::path::Path::new(&bin)).expect("wxq_dict.bin 装载")
}

fn real_form() -> AuxForm {
    let home = std::env::var("HOME").unwrap();
    let wubi = std::env::var("WUBI_YAML")
        .unwrap_or_else(|_| format!("{home}/tmp/hanhuo/wubi86_swbd.dict.yaml"));
    let rsd = std::env::var("RSD_TSV").unwrap_or_else(|_| {
        format!("{home}/tmp/hanhuo/runtime_maps/rsd_char_strokes.tsv")
    });
    AuxForm::from_file(
        std::path::Path::new(&wubi),
        std::path::Path::new(&rsd),
    )
    .expect("码表装载（本地资产, 勿分发）")
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn healthy_passthrough_identical() {
    // 零扰动铁律: 无引导符 = query 直通, 词面逐位一致
    let e = engine();
    let f = real_form();
    for inp in ["nihao", "zhen", "ni", "jintiantianqibucuo", "xi'an"] {
        let a = e.aux_query(inp, None, &f, &AuxCfg::default(), 30);
        let b = e.query(inp, None, false, 30, true, true, true);
        assert_eq!(a.len(), b.len(), "{inp}");
        assert!(
            a.iter().zip(&b).all(|(x, y)| x.word == y.word),
            "{inp} 词面应逐位一致"
        );
    }
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn zhen_qf_pins_zhen() {
    // §4 验收: zhen`qf → 镇 系置顶/过滤
    let e = engine();
    let f = real_form();
    let out = e.aux_query("zhen`qf", None, &f, &AuxCfg::default(), 30);
    assert!(!out.is_empty(), "镇系候选应在场");
    assert!(
        out.iter().take(3).any(|c| c.word.contains('镇')),
        "top3 = {:?}",
        out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
    // 自洽律: 过滤后每个候选都满足首/末字命中
    for c in &out {
        assert!(
            cand_hit(&f, AuxScheme::Wubi2, b"qf", &c.word, false),
            "漏网候选: {}",
            c.word
        );
    }
    // 收敛律: 过滤列严格短于原列（真过滤发生）
    let raw = e.query("zhen", None, false, 30, true, true, true);
    assert!(out.len() < raw.len(), "{} vs {}", out.len(), raw.len());
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn ni_wq_pins_ni() {
    // §4 验收: ni`wq → 你 首位
    let e = engine();
    let f = real_form();
    let out = e.aux_query("ni`wq", None, &f, &AuxCfg::default(), 30);
    assert!(!out.is_empty());
    assert_eq!(
        out[0].word.chars().next(),
        Some('你'),
        "top1 = {:?}",
        out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn shi_jg_pins_shi() {
    // §4 验收: shi`jg → 是（日曰类同音群收敛）
    let e = engine();
    let f = real_form();
    let out = e.aux_query("shi`jg", None, &f, &AuxCfg::default(), 30);
    assert!(
        out.iter().take(3).any(|c| c.word.contains('是')),
        "top3 = {:?}",
        out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn no_match_falls_back_to_original() {
    // 空结果回退原列: 与裸 query(pinyin) 逐位一致（防打错辅码颗粒无收）
    let e = engine();
    let f = real_form();
    for inp in ["zhen`zz", "ni`qqq", "shi`vv"] {
        let a = e.aux_query(inp, None, &f, &AuxCfg::default(), 30);
        let b = e.query(
            inp.split('`').next().unwrap(),
            None, false, 30, true, true, true,
        );
        assert_eq!(a.len(), b.len(), "{inp}");
        assert!(
            a.iter().zip(&b).all(|(x, y)| x.word == y.word),
            "{inp} 回退列应逐位一致"
        );
    }
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn invalid_grammar_returns_empty() {
    // 语法无效 = 原生阻断同像（数字/大写/坏字符/空拼音）
    let e = engine();
    let f = real_form();
    for inp in ["zhen`q5", "zhen`QF", "zhen`q-f", "`qf", "zhen`q`f"] {
        let out = e.aux_query(inp, None, &f, &AuxCfg::default(), 30);
        assert!(out.is_empty(), "{inp} 应空返回");
    }
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn tone_digit_parses_but_inert() {
    // 声调语法位占住: ni`wq9 ≡ ni`wq（过滤 TODO, 行为不劣化）
    let e = engine();
    let f = real_form();
    let a = e.aux_query("ni`wq9", None, &f, &AuxCfg::default(), 30);
    let b = e.aux_query("ni`wq", None, &f, &AuxCfg::default(), 30);
    assert_eq!(a.len(), b.len());
    assert!(a.iter().zip(&b).all(|(x, y)| x.word == y.word));
    // 纯声调（无形码）: 原列直通
    let c = e.aux_query("ni`9", None, &f, &AuxCfg::default(), 30);
    let d = e.query("ni", None, false, 30, true, true, true);
    assert!(c.iter().zip(&d).all(|(x, y)| x.word == y.word));
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn gather_keeps_single_chars_only() {
    // 聚拢: zhen`qf/ → 只剩单字命中（词组丢弃）
    let e = engine();
    let f = real_form();
    let out = e.aux_query("zhen`qf/", None, &f, &AuxCfg::default(), 30);
    assert!(!out.is_empty());
    assert!(
        out.iter().all(|c| c.word.chars().count() == 1),
        "聚拢后应全单字: {:?}",
        out.iter().take(5).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn promote_hits_first_miss_sinks() {
    // 提权态: 命中钉序在前（分严格递减）, 未命中原序沉后
    let e = engine();
    let f = real_form();
    let cfg = AuxCfg { mode: AuxMode::Promote, ..AuxCfg::default() };
    let out = e.aux_query("zhen`qf", None, &f, &cfg, 30);
    let n_hit = out
        .iter()
        .filter(|c| cand_hit(&f, AuxScheme::Wubi2, b"qf", &c.word, false))
        .count();
    assert!(n_hit > 0 && n_hit < out.len(), "需同时有命中与未命中");
    for (i, c) in out.iter().enumerate() {
        let h = cand_hit(&f, AuxScheme::Wubi2, b"qf", &c.word, false);
        assert_eq!(h, i < n_hit, "稳定分区破坏 @ {i}: {}", c.word);
    }
    for w in out[..n_hit].windows(2) {
        assert!(w[0].score > w[1].score, "钉序分应递减");
    }
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn mixed_single_before_word_hits() {
    // mixed 态: L1 单字命中先于 L2 词组命中, 未命中殿后
    let e = engine();
    let f = real_form();
    let cfg = AuxCfg { mode: AuxMode::Mixed, ..AuxCfg::default() };
    let out = e.aux_query("zhen`qf", None, &f, &cfg, 30);
    let hit = |w: &str| cand_hit(&f, AuxScheme::Wubi2, b"qf", w, false);
    let first_word_hit = out.iter().position(|c| {
        c.word.chars().count() > 1 && hit(&c.word)
    });
    if let Some(wi) = first_word_hit {
        for c in out[..wi].iter() {
            assert!(
                c.word.chars().count() == 1 || !hit(&c.word),
                "L1/L2 乱序: {}",
                c.word
            );
        }
    }
    // 未命中全在命中之后
    if let (Some(last_hit), _) = (
        out.iter().rposition(|c| hit(&c.word)),
        (),
    ) {
        for c in out[..last_hit].iter() {
            assert!(hit(&c.word), "未命中渗入命中段: {}", c.word);
        }
    }
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn strokes_scheme_filters_ni() {
    // 笔顺方案旋钮: ni`pspzspd → 你（万象 你=3235234 实证锚）
    let e = engine();
    let f = real_form();
    let cfg = AuxCfg { scheme: AuxScheme::Strokes, ..AuxCfg::default() };
    let out = e.aux_query("ni`pspzspd", None, &f, &cfg, 30);
    assert!(
        out.iter().take(3).any(|c| c.word.contains('你')),
        "笔顺锚 你: {:?}",
        out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn head_tail_scheme_filters() {
    // 首末方案旋钮: zhen`qw → 镇（qfhw 首末= qw, §3.2 锚）
    let e = engine();
    let f = real_form();
    let cfg =
        AuxCfg { scheme: AuxScheme::WubiHeadTail, ..AuxCfg::default() };
    let out = e.aux_query("zhen`qw", None, &f, &cfg, 30);
    assert!(
        out.iter().take(3).any(|c| c.word.contains('镇')),
        "首末锚 镇: {:?}",
        out.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "真机: 需 wxq_dict.bin + wubi/rsd 本地资产"]
fn ctx_passthrough_in_filter() {
    // ctx 经 query() 原样生效（faithful: boost/bigram 照常）
    let e = engine();
    let f = real_form();
    let a = e.aux_query("zhen`qf", Some("小"), &f, &AuxCfg::default(), 30);
    assert!(
        a.iter().take(3).any(|c| c.word.contains('镇')),
        "ctx 下过滤照常: {:?}",
        a.iter().take(3).map(|c| c.word.clone()).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "真机: 样例输出报告（--nocapture 查看）"]
fn sample_output_report() {
    // 样例对比: 裸拼音面板 vs 辅码过滤列（CLI 接线前的行为基线）
    let e = engine();
    let f = real_form();
    for (inp, py) in [
        ("zhen`qf", "zhen"),
        ("ni`wq", "ni"),
        ("shi`jg", "shi"),
        ("zhen`qf/", "zhen"),
    ] {
        let aux = e.aux_query(inp, None, &f, &AuxCfg::default(), 10);
        let raw = e.query(py, None, false, 10, true, true, true);
        println!(
            "[aux] {inp:>10} → {:?}",
            aux.iter().map(|c| c.word.clone()).collect::<Vec<_>>()
        );
        println!(
            "[raw] {py:>10} → {:?}",
            raw.iter().map(|c| c.word.clone()).collect::<Vec<_>>()
        );
    }
}
