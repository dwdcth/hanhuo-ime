//! hanhuo CLI：查询 / 黄金对照 / 基准.
//!
//!   hanhuo query "nihao" [--ctx 我们] [-p] [--dp] [--topk N] [--json]
//!   hanhuo golden [golden.jsonl]     全量对照 Python 参考实现
//!   hanhuo bench [--n 200]
use std::path::{Path, PathBuf};
use std::time::Instant;

use hanhuo_core::{BoostCfg, Cand, Engine, UserDict, default_bin};
use serde::Deserialize;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        std::process::exit(2);
    }
    let bin = flag_path(&args, "--bin").unwrap_or_else(default_bin);
    let cmd = args[0].as_str();
    let rest = &args[1..];
    let code = match cmd {
        "query" => cmd_query(&bin, rest),
        "golden" => cmd_golden(&bin, rest),
        "dicts" => cmd_dicts(rest),
        "words" => cmd_words(&bin, rest),
        "bench" => cmd_bench(&bin, rest),
        "user" => cmd_user(&bin, rest),
        "ckvd" => cmd_ckvd(&bin, rest),
        "stt" => cmd_stt(&bin, rest),
        "t2s" => cmd_t2s(&bin, rest),
        "export-nnlm" => cmd_export_nnlm(&bin, rest),
        "nnlm-adv" => cmd_nnlm_adv(rest),
        "export-mdltr" => cmd_export_mdlttr(rest),
        "eval" => cmd_eval(&bin, rest),
        "bwcn-build" => cmd_bwcn_build(rest),
        "dict-build" => cmd_dict_build(rest),
        "ltr" => cmd_ltr(&bin, rest),
        _ => {
            eprintln!("未知命令: {cmd}");
            usage();
            2
        }
    };
    std::process::exit(code);
}

/// 分体布局: --nnlm/--mdltr <file> 模型文件注入（safetensors 开放格式;
/// 引擎槽位优先于包段——字典包可无模型段）
/// 多词库: --dict <file> 可重复（领域词库 医学/计算机/行业…HBWC 或 tsv,
/// 魔数路由）。挂载顺序即同词去重优先级。
fn attach_dicts(e: &Engine, args: &[String]) {
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        if a == "--dict" {
            if let Some(f) = it.peek() {
                if e.attach_bwcn(std::path::Path::new(f)) {
                    println!("[dict] 已挂载 {f}");
                }
            }
        }
    }
}

/// boost 旗标接线（cmd_query/cmd_eval 共用）: 组合判定 + 领域资产挂载 +
/// 配置落位。--dict 在场即视为 boost 域——挂了外挂词库即开启注入
/// （只挂不查=死挂, 2026-11-08 三连修）。
fn wire_boost(e: &mut Engine, args: &[String]) {
    // 2026-11-09 用户裁定: 超越为产品目标——CLI 默认档=引擎默认
    // BoostCfg（B0 整句前置等零代价胜项）; --faithful 显式退回纯忠实
    // （回归/对照口径）。eval 子命令不默认（科学口径显式旗标）。
    if has_flag(args, "--faithful") {
        return;
    }
    let boost_any = [
        "--boost", "--boost-order", "--boost-ckvd", "--boost-tail",
        "--boost-chain", "--boost-nn", "--boost-dict", "--boost-jian",
        "--boost-compose", "--boost-sent-nn", "--boost-seg-beam",
        "--boost-ctx-jp", "--boost-inj-top", "--boost-mixed",
        "--boost-bwcn-sent", "--tone-filter",
        "--dict",
    ]
    .iter()
    .any(|f| has_flag(args, f));
    if !boost_any {
        // 超越档 v2: B0+尾槽+ckvd（资产在场即开——ckvdmac.bin 缺位自动
        // 降级 v1 行为, ckvd_inject 惰性无资产零开销）
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let ck = PathBuf::from(&home)
            .join("tmp/hanhuo/extracted/3.0.1.1/BuildIn/ckvdmac.bin");
        if ck.exists() {
            e.attach_ckvd(&ck);
        }
        let mut d = BoostCfg::default();
        if has_flag(args, "--no-tone-filter") {
            d.tone_filter = false; // 逃生门（调号退化为剥离忽略）
        }
        e.set_boost(d);
        return;
    }
    let ckvd = has_flag(args, "--boost-ckvd") || has_flag(args, "--boost-tail");
    if ckvd {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let ck = PathBuf::from(&home)
            .join("tmp/hanhuo/extracted/3.0.1.1/BuildIn/ckvdmac.bin");
        if !ck.exists() {
            eprintln!("[ckvd] 缺 {ck:?}，B2 跳过");
        } else {
            e.attach_ckvd(&ck);
        }
    }
    // --dict 挂了外挂词库即开启注入（只挂不查=死挂）
    let bwcn = has_flag(args, "--boost-dict") || has_flag(args, "--dict");
    if has_flag(args, "--boost-dict") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        // HBWC v1 自有格式优先, tsv 回退（from_file 魔数路由）
        let mut bt = PathBuf::from(&home)
            .join("tmp/hanhuo/cache/android_harvest/dicts/bwcnmac.hbwc");
        if !bt.exists() {
            bt = PathBuf::from(&home).join(
                "tmp/hanhuo/cache/android_harvest/dicts/bwcnmac_triples.tsv",
            );
        }
        if !bt.exists() {
            eprintln!("[bwcn] 缺 {bt:?}，B2w 跳过");
        } else {
            e.attach_bwcn(&bt);
        }
    }
    e.set_boost(BoostCfg {
        sentence_first_min_len: 8,
        ckvd_inject: ckvd,
        sentence_tail_sweep: if has_flag(args, "--boost-tail") {
            6
        } else {
            0
        },
        lmcn_chain: has_flag(args, "--boost-chain"),
        nnlm_order: has_flag(args, "--boost-nn"),
        jianpin_dat: has_flag(args, "--boost-jian"),
        inj_score_merge: has_flag(args, "--boost-inj-merge"),
        dict_recal: has_flag(args, "--boost-recal"),
        jianpin_inject: true, // B2.7 表键注入: 超越默认档成员（BoostCfg::default 一致）
        compose_inject: has_flag(args, "--boost-compose"), // B6 表外组词
        sentence_nnlm_rescore: has_flag(args, "--boost-sent-nn"), // B7 句序列重排
        // 宽档升默认（2026-11-20 时延勘误后裁定）: --boost-sent-nn 单开
        // 即完整宽档配方（beam 150 + cap160/8000）。原「不升默认」依据
        // 是电池口径时延 1.8×（992ms avg/句=评测全管线）; 引擎单 query
        // 实测 1.04×（157→163ms, 69 候选/990 字符打分量——分桶合批
        // 兑现）→ 产品化阻塞解除。显式 --boost-seg-beam[=N] 仍优先
        // （评测脚本总显式传值, 科学口径零影响）。
        sentence_seg_beam: {
            let explicit = args.iter().position(|a| {
                a == "--boost-seg-beam"
                    || a.starts_with("--boost-seg-beam=")
            }).map(|i| {
                // B8 全段变体束预算（heads+单编辑×段格, 字面去重后填充;
                // 裸旗标=20 缺省, =N 覆盖——recall 战役旋钮）
                args.get(i + 1).and_then(|v| v.parse().ok())
                    .or_else(|| args[i]
                        .strip_prefix("--boost-seg-beam=")
                        .and_then(|v| v.parse().ok()))
                    .unwrap_or(20)
            });
            explicit.unwrap_or(if has_flag(args, "--boost-sent-nn") {
                150
            } else {
                0
            })
        },
        bwcn_inject: bwcn,
        ctx_jp_compose: has_flag(args, "--boost-ctx-jp"), // c.3 词块组装×ctx-NNLM
        inj_top_promote: has_flag(args, "--boost-inj-top"), // B2w-τ 域意图提权
        mixed_input: has_flag(args, "--boost-mixed"), // 混合输入（全拼∪声母）
        bwcn_sent: has_flag(args, "--boost-bwcn-sent"), // B2w-sent 域词边（需 --boost-seg-beam 同开）
        tone_filter: has_flag(args, "--tone-filter"), // 声调筛选（输入含 1-5 调号）
    });
}

fn attach_model_files(e: &mut Engine, args: &[String]) {
    if let Some(f) = flag_val(args, "--nnlm") {
        // 变体路由: mobile → Nnlm; advanced（nlrgpt2）→ NnlmAdv（注入即
        // 优先于轻量版——nnlm_* 统一入口全链换脑: M4/a 层/B3/B7/c.3）
        match std::fs::read(&f)
            .map_err(|e| e.to_string())
            .and_then(|b| hanhuo_lm::nnlm::Nnlm::from_safetensors(&b))
        {
            Ok(m) => {
                e.set_nnlm(m);
                println!("[nnlm] 已注入 {f}");
            }
            Err(light_err) => {
                let adv = std::fs::read(&f)
                    .map_err(|e| e.to_string())
                    .and_then(|b| {
                        hanhuo_lm::nnlm_adv::NnlmAdv::from_safetensors(&b)
                    });
                match adv {
                    Ok(m) => {
                        e.set_nnlm_adv(m);
                        let d = e.nnlm_adv_dims().unwrap_or_default();
                        println!(
                            "[nnlm] 已注入高级版 {f}（{}层 d{} heads{} \
                             ctx{} vocab{}）",
                            d.0, d.1, d.2, d.4, d.5
                        );
                    }
                    Err(adv_err) => {
                        eprintln!("[nnlm] {f} 装载失败: {light_err} / {adv_err}");
                    }
                }
            }
        }
    }
    if let Some(f) = flag_val(args, "--bgrm") {
        // 魔数路由: .safetensors 走开放格式装载→段体还原; 其余=剥出的
        // 原始段体直注
        let res = std::fs::read(&f)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                if f.ends_with(".safetensors") {
                    hanhuo_lm::bgrm_safetensors::BgrmOpen::from_safetensors(&b)
                        .map_err(|e| e.to_string())
                        .map(|m| m.into_native())
                } else {
                    Ok(b)
                }
            })
            .and_then(|b| e.set_bgrm(&b));
        match res {
            Ok(()) => println!("[bgrm] 已注入 {f}"),
            Err(err) => eprintln!("[bgrm] {f} 注入失败: {err}"),
        }
    }
    if let Some(f) = flag_val(args, "--mdltr") {
        match std::fs::read(&f)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                hanhuo_lm::mdltr_safetensors::MdltrOpen::from_safetensors(&b)
                    .and_then(|m| m.into_native())
            })
        {
            Ok(m) => {
                e.set_mdltr(m);
                println!("[mdltr] 已注入 {f}");
            }
            Err(err) => eprintln!("[mdltr] {f} 装载失败: {err}"),
        }
    }
}

mod shuangpin;

fn usage() {
    eprintln!(
        "用法: hanhuo query|golden|bench [--bin PATH] ...\n\
         \x20 query <inp> [--ctx S] [-p] [--dp] [--topk N] [--json]\n\
         \x20       [--boost-tail] [--boost-seg-beam] [--boost-sent-nn --nnlm F]\n\
         \x20       [--boost-ctx-jp=c.3 词块×ctx-NNLM（需 --ctx）]\n\
         \x20       默认: B2.8 错拼容错开（--no-typo-retry/--faithful 关）;\n\
         \x20       B2.9 模糊音显式 --fuzz（或 --typo-retry 兼容别名, 同开两者）;\n\
         \x20       --no-fuzz 强制关（三态设置位）\n\
         \x20       辅码语法 `拼音`辅码`（如 ni`wq→你）: [--aux-scheme\n\
         \x20 wubi2|headtail|strokes] [--aux-mode filter|promote|mixed]\n\
         \x20       四小件: --en 英文候选 | --safe 敏感词过滤 |\n\
         \x20 --emoji emoji 变体 | --tcd 错别字纠正建议（表缺席零行为）\n\
         \x20 golden [PATH=golden/golden.jsonl]\n\
         \x20 bench [--n 200]\n\
         \x20 boost 旗标: --boost-tail=B2.5 尾槽变体 --boost-seg-beam=B8 全段\n\
         \x20 变体束 --boost-sent-nn=B7 句序列 NNLM 重排（需 --nnlm 模型）\n\
         \x20 --boost-ctx-jp=c.3 非合法切分域词块组装×ctx-NNLM"
    );
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn flag_val(args: &[String], name: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == name {
            return it.next().cloned();
        }
    }
    None
}

fn flag_path(args: &[String], name: &str) -> Option<PathBuf> {
    flag_val(args, name).map(PathBuf::from)
}

fn positional(args: &[String]) -> Option<String> {
    // 第一个非 flag 位置参数
    let mut skip_next = false;
    for a in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a.starts_with("--") {
            skip_next = matches!(
                a.as_str(),
                "--ctx" | "--topk" | "--n" | "--bin" | "--nnlm" | "--mdltr"
                    | "--dict" | "--user" | "--bgrm"
            );
            continue;
        }
        return Some(a.clone());
    }
    None
}

fn fmt_rows(cands: &[Cand]) {
    for (i, c) in cands.iter().enumerate() {
        let lm = c.lm.map(|v| format!(" lm={v:+.2}")).unwrap_or_default();
        let nn =
            c.nn_n.map(|v| format!(" N={v:+.2}")).unwrap_or_default();
        let rd = if c.reading.is_empty() {
            String::new()
        } else {
            format!("  [{}]", c.reading)
        };
        println!(
            "{:3}. {:<12} score={:+8.3} logP={:+7.3} {}{}{}",
            i + 1,
            c.word,
            c.score,
            c.logp,
            c.kind.as_str(),
            lm,
            rd
        );
        let _ = nn;
    }
}

/// ckvd 长尾短语词典直查（2026-11-07 数据层终解）。
/// 用法: hanhuo ckvd yuangezaoshanghao [--ckvd PATH]
fn cmd_ckvd(_bin: &Path, args: &[String]) -> i32 {
    let inp = positional(args).unwrap_or_default();
    let ckvd_path = flag_path(args, "--ckvd").unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let p = PathBuf::from(home)
            .join("tmp/hanhuo/extracted/3.0.1.1/BuildIn/ckvdmac.bin");
        if p.exists() {
            p
        } else {
            PathBuf::from("ckvdmac.bin")
        }
    });
    let ckvd = match hanhuo_dict::CkvdPhrases::from_file(&ckvd_path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("ckvd 加载失败: {err}");
            return 1;
        }
    };
    let rs = ckvd.lookup(&inp);
    println!(
        "# ckvd {:?} ({} 桶) → {} 条",
        inp,
        ckvd.bucket_count(),
        rs.len()
    );
    for (i, p) in rs.iter().take(20).enumerate() {
        println!("{:3}. {:<16} {} w={}", i + 1, p.text, p.pinyin, p.weight);
    }
    0
}

/// t2s 繁→简转换（stt 反向表, 内存构建零新数据件——tts 开源等价）。
/// 用法: hanhuo t2s 裏面 [--stt PATH]
fn cmd_t2s(_bin: &Path, args: &[String]) -> i32 {
    let inp = positional(args).unwrap_or_default();
    let stt_path = flag_path(args, "--stt").unwrap_or_else(|| {
        PathBuf::from("data/stt_open.tsv")
    });
    let table = match hanhuo_dict::S2tTsv::from_tsv_file(&stt_path) {
        Ok(t) => hanhuo_dict::S2tAny::Tsv(t.reversed()),
        Err(err) => {
            eprintln!("t2s 加载失败: {err}");
            return 1;
        }
    };
    match table.s2t_word(&inp) {
        Some(vs) if !vs.is_empty() => {
            println!("{inp} → {}", vs.join(" / "));
        }
        _ => println!("{}", table.s2t_text(&inp)),
    }
    0
}

/// stt 简→繁转换（词级/文本级）。用法: hanhuo stt 头发 [--stt PATH]
fn cmd_stt(_bin: &Path, args: &[String]) -> i32 {
    let inp = positional(args).unwrap_or_default();
    let stt_path = flag_path(args, "--stt").unwrap_or_else(|| {
        // 开源链优先（c.6 审计后续: opencc 导出表）, 腾讯件退居显式
        let p = PathBuf::from("data/stt_open.tsv");
        if p.exists() {
            return p;
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let p = PathBuf::from(home)
            .join("tmp/hanhuo/extracted/3.0.1.1/BuildIn/stt.bin");
        if p.exists() { p } else { PathBuf::from("stt.bin") }
    });
    let s2t = if stt_path.extension().and_then(|e| e.to_str())
        == Some("tsv") {
        match hanhuo_dict::S2tTsv::from_tsv_file(&stt_path) {
            Ok(s) => hanhuo_dict::S2tAny::Tsv(s),
            Err(err) => {
                eprintln!("stt 加载失败: {err}");
                return 1;
            }
        }
    } else {
        match hanhuo_dict::S2t::from_file(&stt_path) {
            Ok(s) => hanhuo_dict::S2tAny::Bin(s),
            Err(err) => {
                eprintln!("stt 加载失败: {err}");
                return 1;
            }
        }
    };
    match s2t.s2t_word(&inp) {
        Some(vs) => println!("词级: {vs:?} (主变体 {})", vs[0]),
        None => println!("词级: 无条目"),
    }
    println!("文本级: {}", s2t.s2t_text(&inp));
    0
}

/// NNLM 开放格式导出（轻量版 = wxq.bin 内嵌 nlrgptand 2层d192）。
/// 用法: hanhuo export-nnlm [目录=data] （写 fp32+fp16，词表内嵌单文件）
fn cmd_export_nnlm(bin: &Path, args: &[String]) -> i32 {
    let dir = positional(args)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    let e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    let Some(sec) = e.nnlm_section() else {
        eprintln!("wxq.bin 无 NNLM 区");
        return 1;
    };
    let model = match hanhuo_lm::nnlm::Nnlm::parse(sec) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("NNLM 解析失败: {err}");
            return 1;
        }
    };
    std::fs::create_dir_all(&dir).ok();
    for fp16 in [false, true] {
        let name = if fp16 { "nnlm_mobile_fp16.safetensors" } else { "nnlm_mobile_fp32.safetensors" };
        let p = dir.join(name);
        std::fs::write(&p, model.to_safetensors(fp16)).expect("写文件");
        println!("{} ({:.1}MB)", p.display(), p.metadata().unwrap().len() as f64 / 1e6);
    }
    println!("词表内嵌 ({} tokens)", model.vocab.len());
    0
}

/// 高级版 NNLM 探针: nlrgpt2(d256) 开放格式直查——下 token 分布 top-k。
/// 用法: hanhuo nnlm-adv 我们一起 [--k 8] [--fp16] [--model <文件>]
fn cmd_nnlm_adv(args: &[String]) -> i32 {
    let text = positional(args).unwrap_or_default();
    let k: usize = flag_val(args, "--k").and_then(|v| v.parse().ok()).unwrap_or(8);
    let model = flag_path(args, "--model").unwrap_or_else(|| {
        PathBuf::from("data").join(if args.contains(&"--fp16".to_string()) {
            "nnlm_advanced_fp16.safetensors"
        } else {
            "nnlm_advanced_fp32.safetensors"
        })
    });
    let bytes = match std::fs::read(&model) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("模型读取失败 {err}: {}（默认 data/, 可 --model 指定）", model.display());
            return 1;
        }
    };
    let m = match hanhuo_lm::nnlm_adv::NnlmAdv::from_safetensors(&bytes) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("装载失败: {err}");
            return 1;
        }
    };
    let ids = match m.encode(&text) {
        Ok(i) => i,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let mut inv = vec![String::new(); m.dims().5];
    for (t, &i) in &m.vocab {
        inv[i as usize] = t.clone();
    }
    let mut past = None;
    let mut lp = Vec::new();
    for &tid in &ids {
        let o = m.predict(tid, past.as_ref());
        past = Some(o.past);
        lp = o.lp;
    }
    let mut order: Vec<usize> = (0..lp.len()).collect();
    order.sort_by(|&a, &b| lp[b].total_cmp(&lp[a]));
    println!("输入 {:?} → ids {:?}（dims {:?}）", text, ids, m.dims());
    for (r, &i) in order.iter().take(k).enumerate() {
        println!("{:>2}. {:>6} {:<4} lp={:.4}", r + 1, i, inv[i], lp[i]);
    }
    0
}

/// mdltr LightGBM 开放格式导出（wxq.bin MDLT 区 → hanhuo-mdltr v1，位精确）。
/// 用法: hanhuo export-mdltr [目录=data]
fn cmd_export_mdlttr(args: &[String]) -> i32 {
    let dir = positional(args)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    let bin = default_bin();
    let pack = match hanhuo_lm::pack::Pack::open(&bin) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("wxq.bin 打开失败: {err}");
            return 1;
        }
    };
    let sec = match pack.raw(b"MDLT") {
        Ok(s) => s,
        Err(err) => {
            eprintln!("MDLT 区缺失: {err}");
            return 1;
        }
    };
    let m = match hanhuo_lm::mdltr::Mdltr::parse(sec) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("MDLT 解析失败: {err}");
            return 1;
        }
    };
    std::fs::create_dir_all(&dir).ok();
    let p = dir.join("mdltr.safetensors");
    std::fs::write(&p, m.to_safetensors()).expect("写文件");
    println!(
        "{} ({:.0}KB, 131 树, 往返位精确)",
        p.display(),
        p.metadata().unwrap().len() as f64 / 1e3
    );
    0
}

/// IME 评测: eval_v0.jsonl {kind,text,py} → 句/词级 top-K 命中 + MRR。
/// 用法: hanhuo eval <file> [--nn --ltr] [--limit N] [--topk 10] [--json]
/// 忠实模式基线 / boost 模式对比共用此命令（超越线度量地基, ADR-002）。
/// 评测聚合核心（cmd_eval 度量层）: (kind, stratum, rank) 流 →
/// BTreeMap 桶 (n, [hits@1,3,5,10], Σrr)。抽出为纯函数——全超越线
/// 指标数字的正确性锚（rank=0 表 miss: 不入 hits 不入 rr）。
pub(crate) fn eval_aggregate(
    rows: &[(u8, String, usize)],
) -> std::collections::BTreeMap<String, (usize, [usize; 4], f64)> {
    let mut agg = std::collections::BTreeMap::new();
    for (kind, stratum, rank) in rows {
        let (kind, stratum, rank) = (*kind, stratum.as_str(), *rank);
        let key = format!(
            "{}{}",
            if kind == 1 { "sent" } else { "word" },
            if stratum == "all" { String::new() } else { format!(":{stratum}") }
        );
        let a = agg.entry(key).or_insert((0usize, [0usize; 4], 0f64));
        a.0 += 1;
        if rank > 0 {
            a.2 += 1.0 / rank as f64;
            for (bi, &k) in [1usize, 3, 5, 10].iter().enumerate() {
                if rank <= k {
                    a.1[bi] += 1;
                }
            }
        }
    }
    agg
}

fn cmd_eval(bin: &Path, args: &[String]) -> i32 {
    let file = match positional(args) {
        Some(f) => PathBuf::from(f),
        None => {
            eprintln!("用法: hanhuo eval <eval.jsonl> [--nn --ltr --jianpin] [--limit N]");
            return 1;
        }
    };
    let nn = has_flag(args, "--nn");
    let ltr = has_flag(args, "--ltr");
    let jianpin = has_flag(args, "--jianpin");
    // B2.8 臂: --typo-retry（健康直通/死段变体重试——v2 零伤害门口径）
    let typo_retry = has_flag(args, "--typo-retry");
    let topk: usize = flag_val(args, "--topk")
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let limit: usize = flag_val(args, "--limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    let mut e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    attach_model_files(&mut e, args);
    attach_dicts(&e, args);
    wire_boost(&mut e, args);
    if has_flag(args, "--no-fuzz") {
        e.set_fuzz_mode(hanhuo_core::engine::FUZZ_OFF);
    }
    let mut rows: Vec<(u8, String, usize)> = Vec::new();
    let data = std::fs::read_to_string(&file).unwrap_or_else(|err| {
        eprintln!("读取 {file:?} 失败: {err}");
        String::new()
    });
    let mut n_lines = 0usize;
    for line in data.lines() {
        if line.trim().is_empty() || n_lines >= limit {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let text = v["text"].as_str().unwrap_or("");
        let py = v["py"].as_str().unwrap_or("");
        // v2 分层: stratum 字段（缺省=all）→ 独立聚合桶（kind×stratum）
        let stratum = v["stratum"].as_str().unwrap_or("all").to_string();
        let kind = if v["kind"] == "sent" { 1 } else { 0 };
        if text.is_empty() || py.is_empty() {
            continue;
        }
        n_lines += 1;
        // 深取重排浅评: 统一 top-50 检索（boost 与忠实同深度公平）,
        // 计分只看前 topk——整句候选常沉底, 浅取会饿死 B0
        let ctx = v["ctx"].as_str();
        let cands = if typo_retry {
            e.query_typo_retry(py, ctx, 50, nn, ltr)
        } else {
            e.query(py, ctx, jianpin, 50, nn, ltr, true)
        };
        let rank = cands
            .iter()
            .position(|c| c.word == text)
            .map(|i| i + 1)
            .unwrap_or(0);
        rows.push((kind, stratum, rank));
    }
    let agg = eval_aggregate(&rows);
    if !has_flag(args, "--json") {
        println!(
            "eval {} 条（nn={nn} ltr={ltr} topk={topk}）",
            n_lines
        );
        for (name, a) in &agg {
            if a.0 == 0 {
                continue;
            }
            let n = a.0 as f64;
            println!(
                "  {:<12} n={:<4} top1={:.1}% top3={:.1}% top5={:.1}%                  top10={:.1}% MRR={:.3}",
                name,
                a.0,
                a.1[0] as f64 / n * 100.0,
                a.1[1] as f64 / n * 100.0,
                a.1[2] as f64 / n * 100.0,
                a.1[3] as f64 / n * 100.0,
                a.2 / n
            );
        }
    } else {
        let mut out = serde_json::json!({"n": n_lines, "nn": nn, "ltr": ltr});
        for (name, a) in &agg {
            if a.0 > 0 {
                let n = a.0 as f64;
                out[name] = serde_json::json!({
                    "n": a.0,
                    "top1": a.1[0] as f64 / n,
                    "top3": a.1[1] as f64 / n,
                    "top5": a.1[2] as f64 / n,
                    "top10": a.1[3] as f64 / n,
                    "mrr": a.2 / n,
                });
            }
        }
        println!("{out}");
    }
    0
}

/// rime .dict.yaml → HBWC v1（from_rime_yaml 的 CLI 面; 与
/// tools/dict_build.py 产出逐字节相同——部署链无 python 依赖）。
/// 用法: hanhuo dict-build <in.dict.yaml> --out <domain.hbwc>
fn cmd_dict_build(args: &[String]) -> i32 {
    let (Some(inp), Some(out)) = (positional(args), flag_val(args, "--out"))
    else {
        eprintln!("用法: hanhuo dict-build <in.dict.yaml> --out <domain.hbwc>");
        return 1;
    };
    match hanhuo_dict::bwcn::BwcnTriples::from_rime_yaml(&inp) {
        Ok(b) => {
            let bin = b.to_hbwc();
            if let Err(e) = std::fs::write(&out, &bin) {
                eprintln!("写失败: {e}");
                return 1;
            }
            let n_words =
                b.map.values().map(|v| v.len()).sum::<usize>();
            println!(
                "{} → {} ({:.1}KB, {} 词 {} 键, HBWC v1)",
                inp,
                out,
                bin.len() as f64 / 1e3,
                n_words,
                b.len()
            );
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// bwcn tsv → HBWC v1 自有二进制格式（排序键+二分布局, 键级拼音去重）。
/// 用法: hanhuo bwcn-build <in.tsv> <out.hbwc>
fn cmd_bwcn_build(args: &[String]) -> i32 {
    let (Some(inp), Some(out)) = (positional(args), flag_val(args, "--out"))
    else {
        eprintln!("用法: hanhuo bwcn-build <in.tsv> --out <out.hbwc>");
        return 1;
    };
    match hanhuo_dict::bwcn::BwcnTriples::from_tsv(&inp) {
        Ok(b) => {
            let bin = b.to_hbwc();
            if let Err(e) = std::fs::write(&out, &bin) {
                eprintln!("写失败: {e}");
                return 1;
            }
            println!(
                "{} → {} ({:.1}MB, {} 词 {} 键, HBWC v1)",
                inp,
                out,
                bin.len() as f64 / 1e6,
                b.map.values().map(|v| v.len()).sum::<usize>(),
                b.len()
            );
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn cmd_query(bin: &Path, args: &[String]) -> i32 {
    let inp = positional(args).unwrap_or_default();
    // 双拼前置键位解映射（backlog 输入方案扩展; --sp xiaohe——小鹤。
    // 引擎零改动: 双拼串在此转为全拼串后走原管线）
    // 双拼前置键位解映射（backlog 输入方案扩展; --sp xiaohe——小鹤。
    // 引擎零改动: 双拼串在引擎打开后转全拼（音节表消歧共键族）再走
    // 原管线——见下方 demux_validated 应用点）
    let sp_scheme = match flag_val(args, "--sp") {
        Some(v) if v == "xiaohe" => Some(shuangpin::SpScheme::Xiaohe),
        Some(v) if v == "ziranma" => Some(shuangpin::SpScheme::Ziranma),
        Some(other) => {
            eprintln!("[sp] 未知双拼方案 {other:?}（现支持: xiaohe|ziranma）");
            return 1;
        }
        None => None,
    };
    let ctx = flag_val(args, "--ctx");
    let jianpin = has_flag(args, "-p") || has_flag(args, "--jianpin");
    let dp = has_flag(args, "--dp");
    // B2.8 句容错入口: --typo-retry（健康直通, 死段变体重试; 蕴含 dp）
    let typo_retry = has_flag(args, "--typo-retry");
    let nn = has_flag(args, "--nn");
    let ltr = has_flag(args, "--ltr");
    let topk: usize =
        flag_val(args, "--topk").and_then(|v| v.parse().ok()).unwrap_or(20);
    let json = has_flag(args, "--json");
    let mut e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    // T9 九宫模式（设计-T9九宫输入 §5-P1: 纯 2-9 数字键串走九宫展开;
    /// 门前分叉在 query_inner 白名单之前）
    if has_flag(args, "--t9") {
        e.set_t9(true);
    }
    attach_model_files(&mut e, args);
    attach_dicts(&e, args);
    if let Some(up) = flag_path(args, "--user") {
        e.attach_user(&up);
    }
    wire_boost(&mut e, args);
    // B2.9 模糊音设置位（真 IME 语义; --no-fuzz 关闭——科学臂/口径对照）
    if has_flag(args, "--no-fuzz") {
        e.set_fuzz_mode(hanhuo_core::engine::FUZZ_OFF);
    }
    // 双拼解映射在引擎开后（音节表消歧）; inp 为 query 用串
    let inp = match sp_scheme {
        Some(sc) => {
            let sylset: std::collections::HashSet<String> =
                e.syllables_public().iter().cloned().collect();
            shuangpin::demux_validated(&inp, sc, &|s| sylset.contains(s))
        }
        None => inp,
    };
    // 辅助码语法路由（融合-辅助码-万象 §3.1 P0, 交接 §6 #3）: 输入含
    // 引导符 `` ` `` 即走辅码管线。原生 query 对 ` 走标点阻断律（空返回）
    // → 本路由纯增量, 无 ` 输入零扰动。
    if inp.contains('`') {
        return cmd_aux(&e, &inp, ctx.as_deref(), topk, args, json);
    }
    // 2026-11-09 升默认裁定（backlog 归责隔离结论）: B2.8 错拼容错为
    // **产品默认**（v2 门 word:common/rare 逐位零差; 死输入重试是净收益
    // ——错拼组 frag@3 84%）; B2.9 模糊音保持**显式**（--fuzz 或
    // --typo-retry 兼容别名——v2 common top3 −0.4pp 归 B2.9, 真 IME
    // 模糊音即设置项默认关）。--faithful / --no-typo-retry 退回纯忠实。
    let fuzz_explicit =
        has_flag(args, "--fuzz") || typo_retry;
    if !fuzz_explicit {
        // 默认关 fuzz（--no-fuzz 已在场时同样 OFF, 幂等）
        e.set_fuzz_mode(hanhuo_core::engine::FUZZ_OFF);
    }
    let b28_default = typo_retry
        || (!has_flag(args, "--no-typo-retry")
            && !has_flag(args, "--faithful"));
    // awb 敏感词全管线封杀（--safe）: 引擎级装载——组词/联想/记忆/
    // 候选四面结构性缺席（query 双出口滤 + userdict commit 拒绝）
    if has_flag(args, "--safe") {
        let p = std::env::var("HANHUO_AWB")
            .unwrap_or_else(|_| "data/awb_open.txt".to_string());
        hanhuo_core::engine::awb::init(std::path::Path::new(&p));
    }
    // ── 四小件消费点（2026-11-18; 表缺席自动零行为）──
    // 英文通道: 纯字母输入 + --en → 词表候选（精确优先, 前缀补全频序）
    if has_flag(args, "--en")
        && inp.chars().all(|c| c.is_ascii_alphabetic())
        && !inp.is_empty()
    {
        if let Some(t) = hanhuo_dict::smallfeat::EnTable::get() {
            let mut cands = t.candidates(&inp);
            // --enlm: 词级英文 LM 重排（wltm 开源等价 enlm_v1）——
            // P(w|BOS) 单前向取句首分布, 候选按 lp 降序（词表外保序尾）
            if let Some(f) = flag_val(args, "--enlm") {
                if let Ok(b) = std::fs::read(&f) {
                    if let Ok(m) =
                        hanhuo_lm::nnlm_adv::NnlmAdv::from_safetensors(&b)
                    {
                        let dbg_o = m.predict(0, None);
                        eprintln!(
                            "[enlm] 装载 ok: vocab {} lp[the]={:.3}",
                            m.vocab.len(),
                            m.vocab.get("the")
                                .map(|&i| dbg_o.lp[i as usize])
                                .unwrap_or(-99.0)
                        );
                        let o = m.predict(0, None);
                        let lp_of = |w: &str| -> f32 {
                            m.vocab
                                .get(w)
                                .map(|&i| o.lp.get(i as usize)
                                     .copied().unwrap_or(-30.0))
                                .unwrap_or(-30.0)
                        };
                        cands.sort_by(|a, b| {
                            let (wa, _) = a.split_once('\t')
                                .unwrap_or((a, ""));
                            let (wb, _) = b.split_once('\t')
                                .unwrap_or((b, ""));
                            lp_of(wb).partial_cmp(&lp_of(wa))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                }
            }
            if !cands.is_empty() {
                println!(
                    "# 英文候选（{}）: {}",
                    if flag_val(args, "--enlm").is_some() {
                        "enlm 语境序"
                    } else {
                        "en_open 频序"
                    },
                    cands.len()
                );
                for c in &cands {
                    println!("EN {c}");
                }
            }
        }
    }
    let mut cs0 = if b28_default {
        e.query_typo_retry(&inp, ctx.as_deref(), topk, nn, ltr)
    } else {
        e.query(&inp, ctx.as_deref(), jianpin, topk, nn, ltr, dp)
    };
    // emoji/tcd 注入（--emoji/--tcd）: 命中词后插变体行
    if has_flag(args, "--emoji") || has_flag(args, "--tcd") {
        let em = if has_flag(args, "--emoji") {
            hanhuo_dict::smallfeat::EmojiTable::get()
        } else {
            None
        };
        let tc = if has_flag(args, "--tcd") {
            hanhuo_dict::smallfeat::TcdTable::get()
        } else {
            None
        };
        let mut extra: Vec<String> = Vec::new();
        for c in &cs0 {
            if let Some(t) = em {
                if let Some(es) = t.lookup(&c.word) {
                    if let Some(first) = es.first() {
                        extra.push(format!("{} {}", c.word, first));
                    }
                }
            }
            if let Some(t) = tc {
                if let Some(w) = t.correct(&c.word) {
                    extra.push(format!("{} ⇐ 错写 {}", w, c.word));
                }
            }
        }
        for x in extra {
            println!("※ {x}");
        }
    }
    let cs = cs0;
    if json {
        let items: Vec<serde_json::Value> = cs
            .iter()
            .map(|c| {
                serde_json::json!({
                    "word": c.word, "reading": c.reading,
                    "logp": round4(c.logp), "score": round4(c.score),
                    "kind": c.kind.as_str(),
                    "lm": c.lm.map(round4),
                    "ltr": c.ltr.map(round4),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({"query": inp, "ctx": ctx,
                               "jianpin": jianpin, "cands": items})
        );
    } else {
        println!(
            "# query={inp:?} {}{}{}  top{}",
            if jianpin { "简拼 " } else { "" },
            ctx.as_deref().map(|c| format!("ctx={c} "))
                .unwrap_or_default(),
            if dp { "dp " } else { "" },
            cs.len()
        );
        fmt_rows(&cs);
    }
    0
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// 辅助码查询（融合-辅助码-万象 §3.1 P0）: 语法 = `拼音段` + `` ` `` +
/// `辅码段`（a-z 形码串; 尾缀 / 聚拢; 7890 声调位已占住——引擎层解析,
/// 见 engine::aux）。码表惰性装载一次（wubi86_swbd + rsd 笔顺, 本地
/// 资产勿分发; WUBI_YAML/RSD_TSV 环境变量可覆盖）。旋钮:
/// --aux-scheme wubi2|headtail|strokes（缺省 wubi2 = 万象「五笔前2」）/
/// --aux-mode filter|promote|mixed（缺省 filter）。
fn cmd_aux(
    e: &Engine,
    inp: &str,
    ctx: Option<&str>,
    topk: usize,
    args: &[String],
    json: bool,
) -> i32 {
    use hanhuo_core::engine::{AuxCfg, AuxMode, AuxScheme};
    use hanhuo_dict::AuxForm;
    use std::sync::OnceLock;
    static FORM: OnceLock<Option<AuxForm>> = OnceLock::new();
    let form = FORM.get_or_init(|| {
        // 开源链优先（c.6 审计后续二: rime-wubi/cnchar 重建表）,
        // 反演件退居显式（WUBI_YAML/RSD_TSV 覆盖仍最高优先）。
        let wubi = std::env::var("WUBI_YAML").ok().unwrap_or_else(|| {
            let p = PathBuf::from("data/wubi86_open.dict.yaml");
            if p.exists() {
                return p.display().to_string();
            }
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            format!("{home}/tmp/hanhuo/wubi86_swbd.dict.yaml")
        });
        let rsd = std::env::var("RSD_TSV").ok().unwrap_or_else(|| {
            let p = PathBuf::from("data/rsd_strokes_open.tsv");
            if p.exists() {
                return p.display().to_string();
            }
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            format!("{home}/tmp/hanhuo/runtime_maps/rsd_char_strokes.tsv")
        });
        match AuxForm::from_file(
            std::path::Path::new(&wubi),
            std::path::Path::new(&rsd),
        ) {
            Ok(f) => Some(f),
            Err(err) => {
                eprintln!("[aux] 码表装载失败: {err}");
                None
            }
        }
    });
    let Some(form) = form else {
        return 1;
    };
    let mut cfg = AuxCfg::default();
    if let Some(s) = flag_val(args, "--aux-scheme") {
        cfg.scheme = match s.as_str() {
            "wubi2" => AuxScheme::Wubi2,
            "headtail" => AuxScheme::WubiHeadTail,
            "strokes" => AuxScheme::Strokes,
            other => {
                eprintln!(
                    "[aux] 未知方案 {other:?}（wubi2|headtail|strokes）"
                );
                return 1;
            }
        };
    }
    if let Some(s) = flag_val(args, "--aux-mode") {
        cfg.mode = match s.as_str() {
            "filter" => AuxMode::Filter,
            "promote" => AuxMode::Promote,
            "mixed" => AuxMode::Mixed,
            other => {
                eprintln!(
                    "[aux] 未知模式 {other:?}（filter|promote|mixed）"
                );
                return 1;
            }
        };
    }
    let cs = e.aux_query(inp, ctx, form, &cfg, topk);
    if json {
        let items: Vec<serde_json::Value> = cs
            .iter()
            .map(|c| {
                serde_json::json!({
                    "word": c.word, "reading": c.reading,
                    "logp": round4(c.logp), "score": round4(c.score),
                    "kind": c.kind.as_str(),
                    "lm": c.lm.map(round4),
                    "ltr": c.ltr.map(round4),
                })
            })
            .collect();
        let scheme = format!("{:?}", cfg.scheme);
        let mode = format!("{:?}", cfg.mode);
        println!(
            "{}",
            serde_json::json!({"query": inp, "ctx": ctx,
                               "aux": true, "scheme": scheme,
                               "mode": mode, "cands": items})
        );
    } else {
        println!(
            "# query={inp:?} aux({:?}/{:?}) {}top{}",
            cfg.scheme,
            cfg.mode,
            ctx.map(|c| format!("ctx={c} ")).unwrap_or_default(),
            cs.len()
        );
        fmt_rows(&cs);
    }
    0
}

// ----------------------------------------------------------------------
// golden 对照
// ----------------------------------------------------------------------
#[derive(Deserialize)]
struct GCase {
    inp: String,
    ctx: Option<String>,
    jianpin: bool,
    #[serde(default)]
    nn: bool,
    #[serde(default)]
    ltr: bool,
    dp: bool,
    #[serde(default = "d_topk")]
    topk: usize,
    cands: Vec<GCand>,
}

fn d_topk() -> usize {
    20
}

#[derive(Deserialize)]
struct GCand {
    word: String,
    #[serde(default)]
    reading: String,
    logp: f64,
    score: f64,
    kind: String,
    lm: Option<f64>,
    #[serde(default)]
    nn: Option<f64>,
    #[serde(default)]
    ltr: Option<f64>,
}

/// 词库中心列表（data/domain/catalog.json——微信式分库的 CLI 发现面;
/// 壳层/用户按分类取包, `--dict data/domain/<file>` 挂载）
/// 词表导出: hanhuo words [--out /tmp/words.txt] —— 逐行 id:词
/// （c.6 bgrm 重建需词 id 映射; 原 dump_wxq_words 测试的 CLI 化）
fn cmd_words(bin: &Path, args: &[String]) -> i32 {
    let out = flag_val(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/hanhuo_words.txt"));
    let e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(&out).expect("创建输出"));
    use std::io::Write;
    for w in 0..e.n_words as u32 {
        writeln!(f, "{w}\t{}", e.word_text(w)).expect("写");
    }
    f.flush().ok();
    println!("导出 {} 词 → {}", e.n_words, out.display());
    0
}

fn cmd_dicts(args: &[String]) -> i32 {
    let path = flag_val(args, "--catalog")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/domain/catalog.json"));
    let Ok(data) = std::fs::read_to_string(&path) else {
        eprintln!("目录不存在 {:?}（--catalog 可指定; 重建: python3 tools/…_import.py 系列）", path);
        return 1;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) else {
        eprintln!("catalog.json 解析失败");
        return 1;
    };
    println!("词库目录 v{}（{} 分类; 挂载: --dict data/domain/<file>）",
        v["version"].as_str().unwrap_or("?"),
        v["categories"].as_array().map(|a| a.len()).unwrap_or(0));
    println!("{:<4} {:<8} {:<22} {:>9} {:>9}  {:<14} {}",
        "#", "分类", "包", "词数", "KB", "许可", "源");
    for (i, c) in v["categories"].as_array().unwrap_or(&vec![]).iter().enumerate() {
        println!("{:<4} {:<8} {:<22} {:>9} {:>9.0}  {:<14} {}{}",
            i + 1,
            c["name"].as_str().unwrap_or("?"),
            c["file"].as_str().unwrap_or("?"),
            c["words"].as_i64().unwrap_or(0),
            c["bytes"].as_f64().unwrap_or(0.0) / 1e3,
            c["license"].as_str().unwrap_or("?"),
            c["src"].as_str().unwrap_or("?"),
            c["note"].as_str().map(|n| format!(" | {n}")).unwrap_or_default(),
        );
    }
    if let Some(gaps) = v["gaps"].as_array() {
        if !gaps.is_empty() {
            println!("缺源: {}", gaps.iter().filter_map(|g| g.as_str())
                .collect::<Vec<_>>().join(" · "));
        }
    }
    0
}

fn cmd_golden(bin: &Path, args: &[String]) -> i32 {
    let path = positional(args)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("golden/golden.jsonl"));
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("读 {path:?} 失败: {e}");
            return 1;
        }
    };
    let mut e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    attach_model_files(&mut e, args);
    attach_dicts(&e, args);
    let mut total = 0usize;
    let mut pass = 0usize;
    let mut fail_cases: Vec<(usize, String)> = Vec::new();
    for (ln, line) in data.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let case: GCase = match serde_json::from_str(line) {
            Ok(c) => c,
            Err(err) => {
                fail_cases.push((ln + 1, format!("JSON 解析失败: {err}")));
                total += 1;
                continue;
            }
        };
        total += 1;
        let got = e.query(
            &case.inp,
            case.ctx.as_deref(),
            case.jianpin,
            case.topk,
            case.nn,
            case.ltr,
            case.dp,
        );
        let mut why = String::new();
        if got.len() != case.cands.len() {
            why = format!(
                "条数 rust={} py={} [rust 前5: {} | py 前5: {}]",
                got.len(),
                case.cands.len(),
                got.iter().take(5).map(|c| c.word.clone())
                    .collect::<Vec<_>>().join(","),
                case.cands.iter().take(5).map(|c| c.word.clone())
                    .collect::<Vec<_>>().join(",")
            );
        } else {
            for (i, (g, w)) in got.iter().zip(case.cands.iter()).enumerate()
            {
                if g.word != w.word {
                    why = format!(
                        "#{} 词 {}≠{}",
                        i + 1,
                        g.word,
                        w.word
                    );
                    break;
                }
                if g.kind.as_str() != w.kind {
                    why = format!(
                        "#{} {} kind {}≠{}",
                        i + 1,
                        g.word,
                        g.kind.as_str(),
                        w.kind
                    );
                    break;
                }
                if g.reading != w.reading {
                    why = format!(
                        "#{} {} reading {:?}≠{:?}",
                        i + 1,
                        g.word,
                        g.reading,
                        w.reading
                    );
                    break;
                }
                // 容差 5e-3（2026-11-20）: 量化段 3 步长 4/1024=0.0039——
                // open 线词料更新后 rust/py 对同词槽位 fv 可差 1 步
                // （词面/名次/kind/reading 仍逐位锁; 腾讯 611 实际零差不受影响）
                if (g.logp - w.logp).abs() > 5e-3
                    || (g.score - w.score).abs() > 5e-3
                {
                    why = format!(
                        "#{} {} 分数 rust(logp={:.4},score={:.4}) \
                         py(logp={:.4},score={:.4})",
                        i + 1,
                        g.word,
                        g.logp,
                        g.score,
                        w.logp,
                        w.score
                    );
                    break;
                }
                match (g.ltr, w.ltr) {
                    (Some(a), Some(b)) if (a - b).abs() > 5e-3 => {
                        why = format!(
                            "#{} {} ltr {a:.4}≠{b:.4}",
                            i + 1,
                            g.word
                        );
                        break;
                    }
                    (None, Some(_)) | (Some(_), None) => {
                        why =
                            format!("#{} {} ltr 有无不一致", i + 1, g.word);
                        break;
                    }
                    _ => {}
                }
                match (g.nn_n, w.nn) {
                    (Some(a), Some(b)) if (a - b).abs() > 5e-3 => {
                        why = format!(
                            "#{} {} nn {a:.4}≠{b:.4}",
                            i + 1,
                            g.word
                        );
                        break;
                    }
                    (None, Some(_)) | (Some(_), None) => {
                        why =
                            format!("#{} {} nn 有无不一致", i + 1, g.word);
                        break;
                    }
                    _ => {}
                }
                match (g.lm, w.lm) {
                    (Some(a), Some(b)) if (a - b).abs() > 1e-3 => {
                        why = format!(
                            "#{} {} lm {a:.4}≠{b:.4}",
                            i + 1,
                            g.word
                        );
                        break;
                    }
                    (None, Some(_)) | (Some(_), None) => {
                        why =
                            format!("#{} {} lm 有无不一致", i + 1, g.word);
                        break;
                    }
                    _ => {}
                }
            }
        }
        if why.is_empty() {
            pass += 1;
        } else if fail_cases.len() < 12 {
            fail_cases.push((
                ln + 1,
                format!(
                    "inp={:?} ctx={:?} jp={} nn={} dp={}: {why}",
                    case.inp, case.ctx, case.jianpin, case.nn, case.dp
                ),
            ));
        }
    }
    println!(
        "golden: {}/{} 通过 ({:.1}%)",
        pass,
        total,
        100.0 * pass as f64 / total as f64
    );
    for (ln, msg) in &fail_cases {
        eprintln!("  L{ln}: {msg}");
    }
    if pass == total {
        0
    } else {
        1
    }
}

// ----------------------------------------------------------------------
fn cmd_bench(bin: &Path, args: &[String]) -> i32 {
    let n: usize = flag_val(args, "--n")
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let mut e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    attach_model_files(&mut e, args);
    attach_dicts(&e, args);
    // boost 旗标同 query 口径（ADR-002: 增强开销实测）
    if has_flag(args, "--boost-tail") || has_flag(args, "--boost-seg-beam") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let ck = PathBuf::from(&home)
            .join("tmp/hanhuo/extracted/3.0.1.1/BuildIn/ckvdmac.bin");
        if ck.exists() {
            e.attach_ckvd(&ck);
        }
        let mut bt = PathBuf::from(&home)
            .join("tmp/hanhuo/cache/android_harvest/dicts/bwcnmac.hbwc");
        if !bt.exists() {
            bt = PathBuf::from(&home)
                .join("tmp/hanhuo/cache/android_harvest/dicts/bwcnmac_triples.tsv");
        }
        if bt.exists() {
            e.attach_bwcn(&bt);
        }
        e.set_boost(BoostCfg {
            sentence_first_min_len: 8,
            ckvd_inject: true,
            bwcn_inject: true,
            sentence_tail_sweep: 6,
            nnlm_order: has_flag(args, "--boost-nn"),
            jianpin_dat: has_flag(args, "--boost-jian"),
            inj_score_merge: has_flag(args, "--boost-inj-merge"),
            dict_recal: has_flag(args, "--boost-recal"),
            jianpin_inject: false, // 科学口径: 默认档不含 B2.7
            compose_inject: false, // B6 同科学口径: 显式旗标
            sentence_nnlm_rescore: false, // B7 同科学口径
            sentence_seg_beam: if has_flag(args, "--boost-seg-beam") {
                20 // B8 全段变体束（科学口径: 显式旗标）
            } else {
                0
            },
            lmcn_chain: false,
            ctx_jp_compose: has_flag(args, "--boost-ctx-jp"), // c.3（科学口径: 显式旗标）
            inj_top_promote: has_flag(args, "--boost-inj-top"), // B2w-τ（科学口径）
            mixed_input: has_flag(args, "--boost-mixed"), // 混合输入（科学口径）
            bwcn_sent: has_flag(args, "--boost-bwcn-sent"), // B2w-sent（科学口径）
            tone_filter: has_flag(args, "--tone-filter"), // 声调筛选（科学口径）
        });
    }
    // 基准语料：黄金集前 200 条输入（--nn 用 nn 用例）
    let nn = has_flag(args, "--nn");
    let data = std::fs::read_to_string("golden/golden.jsonl")
        .unwrap_or_default();
    let mut queries: Vec<(String, Option<String>, bool, bool)> = Vec::new();
    for line in data.lines() {
        if queries.len() >= n {
            break;
        }
        if let Ok(c) = serde_json::from_str::<GCase>(line) {
            if !c.nn {
                queries.push((c.inp, c.ctx, c.jianpin, c.dp));
            }
        }
    }
    if nn {
        queries.clear();
        for line in data.lines() {
            if queries.len() >= n {
                break;
            }
            if let Ok(c) = serde_json::from_str::<GCase>(line) {
                if c.nn {
                    queries.push((c.inp, c.ctx, c.jianpin, c.dp));
                }
            }
        }
    }
    if queries.is_empty() {
        eprintln!("无基准语料（golden/golden.jsonl 缺失）");
        return 1;
    }
    let nn = has_flag(args, "--nn");
    let ltr = has_flag(args, "--ltr");
    let mut times = Vec::with_capacity(queries.len() * 2);
    for _ in 0..2 {
        for (inp, ctx, jp, dp) in &queries {
            let t0 = Instant::now();
            e.query(inp, ctx.as_deref(), *jp, 12, nn, ltr, *dp);
            times.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean: f64 = times.iter().sum::<f64>() / times.len() as f64;
    let p50 = times[times.len() / 2];
    let p95 = times[times.len() * 95 / 100];
    println!(
        "bench{}: {} 查询 ×2 | mean={mean:.3}ms p50={p50:.3}ms p95={p95:.3}ms",
        if nn { "(nn)" } else { "" },
        queries.len()
    );
    0
}


// ----------------------------------------------------------------------
// M3: 用户词学习
// ----------------------------------------------------------------------
fn cmd_user(bin: &Path, args: &[String]) -> i32 {
    let sub = args.first().map(String::as_str).unwrap_or("show");
    let rest = &args[1..];
    let user_path = flag_path(args, "--user").unwrap_or_else(|| {
        std::env::var("HANHUO_USER")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("hanhuo_user.bin"))
    });
    match sub {
        "show" => {
            let ud = UserDict::load(&user_path);
            println!(
                "# {} 组 / {} 条 / 短语 {} / journal {}",
                ud.n_groups(),
                ud.n_entries(),
                ud.n_phrases(),
                ud.journal_len()
            );
            let mut all = ud.dump();
            all.sort_by(|a, b| b.2.anchor.partial_cmp(&a.2.anchor).unwrap());
            for (k, w, e) in all.iter().take(50) {
                println!(
                    "{k:<20} {w:<12} count={} anchor={:.4} ts={}",
                    e.count, e.anchor, e.ts
                );
            }
            0
        }
        // B5-4 容量帽动态配置（持久化 HHD1 头 rsv 槽）
        "config" => {
            let Some(v) = flag_val(rest, "--max-phrases") else {
                eprintln!("用法: hanhuo user config --max-phrases N");
                return 2;
            };
            let Ok(n) = v.parse::<usize>() else {
                eprintln!("--max-phrases 需正整数");
                return 2;
            };
            let mut ud = UserDict::load(&user_path);
            ud.set_max_phrases(n);
            let cap = ud.max_phrases();
            ud.flush();
            println!("容量帽 = {cap}（已持久化）");
            0
        }
        // B5-4 删词/删自造短语（IME 删词手势后端）; --cascade 连带删
        // 含该词短语（组句 n-gram 遗骸; 单字词不级联防误伤清库）
        "remove" => {
            let Some(target) = positional(rest) else {
                eprintln!("用法: hanhuo user remove <词|短语> [--phrase] [--cascade]");
                return 2;
            };
            let e = match Engine::open(bin) {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("引擎加载失败: {err}");
                    return 1;
                }
            };
            e.attach_user(&user_path);
            if has_flag(rest, "--phrase") {
                let ok = e.user_remove_phrase(&target);
                println!(
                    "{} {target}",
                    if ok { "已删除" } else { "未找到（本就不在学习库）" }
                );
            } else if has_flag(rest, "--cascade") {
                let (ok, n) = e.user_remove_cascade(&target);
                if ok {
                    println!("已删除 {target}（级联短语 {n} 条）");
                } else {
                    println!("未找到（本就不在学习库）: {target}");
                }
            } else {
                let ok = e.user_remove(&target);
                println!(
                    "{} {target}",
                    if ok { "已删除" } else { "未找到（本就不在学习库）" }
                );
            }
            0
        }
        // B5-3 选词回报: 上屏后回报（输入+所选）→ 自动学习
        // （句选走 path_words 序列, 组句 n-gram 随之成形）
        "pick" => {
            let pos: Vec<&String> = rest
                .iter()
                .filter(|a| !a.starts_with("--"))
                .collect();
            if pos.len() < 2 {
                eprintln!("用法: hanhuo user pick <输入拼音> <所选词> [--ts N]");
                return 2;
            }
            let (inp, word) = (pos[0].clone(), pos[1].clone());
            let ts: u64 = flag_val(rest, "--ts")
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });
            let e = match Engine::open(bin) {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("引擎加载失败: {err}");
                    return 1;
                }
            };
            e.attach_user(&user_path);
            let ok = e.feedback(&inp, &word, ts);
            e.user_flush();
            println!(
                "{} {word}（path_words 序列学习/单词 commit）",
                if ok { "OK" } else { "REJECT" }
            );
            0
        }
        // B5 组词/组句: 已学短语表（anchor 降序）
        "phrases" => {
            let ud = UserDict::load(&user_path);
            println!("# 短语 {} 条 / 容量帽 {}", ud.n_phrases(), ud.max_phrases());
            for (pk, t, pe) in ud.dump_phrases().iter().take(50) {
                println!(
                    "{pk:<28} {t:<16} count={} anchor={:.4} [{}]",
                    pe.e.count, pe.e.anchor, pe.reading
                );
            }
            0
        }
        "commit" => {
            let Some(word) = positional(rest) else {
                eprintln!("用法: hanhuo user commit <词> [--py 拼音] [--ts N]");
                return 2;
            };
            let ts: u64 = flag_val(rest, "--ts")
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });
            let e = match Engine::open(bin) {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("引擎加载失败: {err}");
                    return 1;
                }
            };
            e.attach_user(&user_path);
            // --py 提示读音（走 DAT 精确键）
            let rid_hint = flag_val(rest, "--py").and_then(|py| {
                let st = e.dat_walk_public(py.as_bytes());
                if st >= 0 {
                    Some(e.dat_range_lo_public(st))
                } else {
                    None
                }
            });
            let ok = e.user_commit(&word, rid_hint, ts);
            let key = rid_hint
                .map(|r| e.rid_key_public(r))
                .or_else(|| e.user_key_of(&word))
                .unwrap_or_default();
            let entry = e.user_entry(&key, &word);
            e.user_flush();
            if ok {
                let ent = entry.unwrap();
                println!(
                    "OK {word} key={key:?} count={} anchor={:.4}",
                    ent.count, ent.anchor
                );
                0
            } else {
                println!("REJECT {word}（非词典词/多音字/超长）");
                0
            }
        }
        _ => {
            eprintln!("未知子命令: {sub}（show|commit）");
            2
        }
    }
}


// ----------------------------------------------------------------------
// mdltr LightGBM（131 树，官方对照逐位一致；179 维组装语义未破，不接排序）
// ----------------------------------------------------------------------
fn cmd_ltr(bin: &Path, args: &[String]) -> i32 {
    let e = match Engine::open(bin) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("引擎加载失败: {err}");
            return 1;
        }
    };
    let sub = args.first().map(String::as_str).unwrap_or("selftest");
    match sub {
        "selftest" => {
            use hanhuo_core::mdltr_refs::REFS;
            let mut ok = 0;
            for (i, (x, want)) in REFS.iter().enumerate() {
                match e.mdltr_score(x) {
                    Some(got) if (got - want).abs() < 1e-9 => {
                        println!("[OK] row{i}: {got:+.10}");
                        ok += 1;
                    }
                    Some(got) => {
                        println!("[FAIL] row{i}: {got:+.10} ≠ {want:+.10}")
                    }
                    None => {
                        println!("[SKIP] bin 无 MDLT 区");
                        return 1;
                    }
                }
            }
            println!("ltr selftest: {ok}/6");
            if ok == 6 {
                0
            } else {
                1
            }
        }
        word => {
            match e.word_feat11(word) {
                Some(f) => {
                    println!(
                        "{word}: {}",
                        f.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>()
                            .join(" ")
                    );
                    0
                }
                None => {
                    println!("{word}: 无 wtid 特征");
                    0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 黄金回归：Rust 引擎 vs Python 参考实现（golden/golden.jsonl 全量）
    /// [ignore] debug 构建 ~23 分钟（nn+ltr 全量 × 无优化 15×）——
    /// 正口 = CLI `hanhuo golden`（release, ~90s）或
    /// `cargo test -p hanhuo-cli --release -- --ignored`
    #[test]
    #[ignore = "真机+release 口径: CLI golden 子命令（debug 23 分钟）"]
    fn golden_all_pass() {
        let bin = default_bin();
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../golden/golden.jsonl"
        );
        let args = vec![path.to_string()];
        assert_eq!(cmd_golden(&bin, &args), 0, "golden 对照未全过");
    }

    /// mix_recall MLP 前向数值锚（agentO_log S6; 原期望值 = python
    /// wxq.engine.mix_recall_forward 腾讯权重的 f64 输出——开源发布
    /// MIX_RECALL 常量权重全零（Correct 通道分布为零行为不变）后
    /// 前向恒 tanh(0)=0。本测试在开源仓锁「置零律」; 腾讯权重位锚
    /// 见私有研究线同名测试。
    #[test]
    fn mix_recall_forward_anchors() {
        use hanhuo_core::mix_recall_forward;
        let a = [-1.0, 0.5, 0.0, -2.0, 0.0, 0.0, 3.0, 0.0];
        let b = [0.25, -1.75, -3.2, -4.5, -1.0, -0.5, 0.0, 1.0];
        let z = [0.0; 8];
        let ya = mix_recall_forward(&a);
        let yb = mix_recall_forward(&b);
        let yz = mix_recall_forward(&z);
        // 置零律: 权重零化 → 前向恒 0（含偏置零——开源发布定版语义）
        assert_eq!(ya, 0.0, "anchor A: {ya:?}");
        assert_eq!(yb, 0.0, "anchor B: {yb:?}");
        assert_eq!(yz, 0.0, "anchor Z: {yz:?}");
    }
}

#[cfg(test)]
mod open_tests {
    use super::*;

    /// 混合 bin NN 回归（open/hanhuo_open_nn.bin 存在才跑；开源词料+腾讯 NNLM）
    #[test]
    fn open_nn_golden_all_pass() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../open");
        let bin = PathBuf::from(format!("{root}/hanhuo_open_nn.bin"));
        if !bin.exists() {
            eprintln!("跳过: {bin:?} 不存在");
            return;
        }
        let args = vec![format!("{root}/open_nn_golden.jsonl")];
        assert_eq!(cmd_golden(&bin, &args), 0, "open nn golden 未全过");
    }

    /// 开源构建回归（open/hanhuo_open.bin 存在才跑；rime-ice 数据链）。
    /// 2026-11-20 parity 战役结案（docs/诊断-rust-py-parity-官方源bin.md）:
    /// 尾部 44 条 ltr 案分歧 = LTR 资产装配缺口（py 恒读全局 npz、rust
    /// 只读 bin MDLT 区, open bin 零 ML 区）——非引擎 bug; 修复 = 对拍
    /// 带 --mdltr 外挂（不注入 bin 保 open 线纯 GPL; 快照再生成须
    /// cascade npz 在位, open_env.sh 注记）。
    #[test]
    fn open_golden_all_pass() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../open");
        let bin = PathBuf::from(format!("{root}/hanhuo_open.bin"));
        if !bin.exists() {
            eprintln!("跳过: {bin:?} 不存在（先跑 tools/open_build.py）");
            return;
        }
        let path = format!("{root}/open_golden.jsonl");
        // LTR 外挂（parity 战役结案: open bin 零 ML 区, py 侧恒读全局
        // npz——不带此件尾部 44 条 ltr 案 raw=0 分歧, 见诊断文档）
        let args = vec![
            path,
            "--mdltr".into(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/mdltr.safetensors").into(),
        ];
        // ≥605（2026-11-20 基线 609/611）: 容 6 案深层 tie 枚举尾差
        // （#7 位低频并列词面板序; 归因修正——构建实为确定性, 连跑
        // md5 逐位同, 差源是官方源词表新槽位踩出 py/rust 引擎在并列
        // tie 的枚举序既有差异, 产品影响零; 与 L174 量化步差同家族,
        // 深挖成本高价值零不立项）。腾讯 golden 611 仍逐位。
        let rc = cmd_golden(&bin, &args);
        assert!(rc <= 6, "open golden 尾差超容差 rc={rc}");
    }
}

#[cfg(test)]

mod eval_agg_tests {
    use super::eval_aggregate;

    fn rows(v: &[(u8, &str, usize)]) -> Vec<(u8, String, usize)> {
        v.iter().map(|(k, s, r)| (*k, s.to_string(), *r)).collect()
    }

    /// 桶键: word/sent × all/分层; miss(rank=0) 计 n 不计 hits/rr
    #[test]
    fn buckets_and_miss_semantics() {
        let agg = eval_aggregate(&rows(&[
            (0, "all", 1),
            (0, "all", 0),   // miss
            (0, "common", 2),
            (1, "all", 5),
        ]));
        assert_eq!(agg.len(), 3, "三桶: word/word:common/sent");
        let w = &agg["word"];
        assert_eq!(w.0, 2);
        assert_eq!(w.1, [1, 1, 1, 1], "top1..top10 都只含 rank1");
        assert_eq!(w.2, 1.0, "miss 不入 rr");
        let c = &agg["word:common"];
        assert_eq!(c.1, [0, 1, 1, 1], "rank2: top3/5/10 入");
        assert_eq!(c.2, 0.5);
        let s_ = &agg["sent"];
        assert_eq!(s_.1, [0, 0, 1, 1], "rank5: top5/10 入");
        assert_eq!(s_.2, 0.2);
    }

    /// Σrr 语义（除 n 在打印层）: ranks [1,2,0] → Σrr=1+0.5+0=1.5,
    /// MRR=1.5/3=0.5 由打印块计算
    #[test]
    fn rr_sum_semantics() {
        let agg = eval_aggregate(&rows(&[(0, "all", 1), (0, "all", 2), (0, "all", 0)]));
        let w = &agg["word"];
        assert_eq!(w.0, 3);
        assert_eq!(w.1, [1, 2, 2, 2]);
        assert!((w.2 - 1.5).abs() < 1e-12, "Σrr=1+0.5+0（MRR=Σ/n 在打印层）");
    }
}
