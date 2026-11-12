//! hanhuo 引擎 wasm 壳（2026-11-19 浏览器端到端）。
//! 面: init(dict_bytes) → query(inp, ctx, topk) → JSON 候选串。
//! 线下资产（bin/域包/NNLM）由 JS 侧 fetch 后以字节注入——wasm 侧
//! 零文件 IO（smallfeat/lmcn 全局表未 init 即零行为, golden 同律）。

use wasm_bindgen::prelude::*;

static mut ENGINE: Option<hanhuo_core::Engine> = None;

/// 英文 LM（enlm, NnlmAdv advanced 变体）——独立槽: Engine 的 nnlm_adv
/// 槽被中文 mobile 占用, enlm 与 CLI cmd_query 同律局部装载（enlm 不
/// 进 Engine, 只服务 en_candidates 重排）。
static mut ENLM: Option<hanhuo_lm::nnlm_adv::NnlmAdv> = None;

fn enlm() -> Option<&'static hanhuo_lm::nnlm_adv::NnlmAdv> {
    unsafe { ENLM.as_ref() }
}

fn engine() -> Option<&'static hanhuo_core::Engine> {
    unsafe { ENGINE.as_ref() }
}

fn engine_mut() -> Option<&'static mut hanhuo_core::Engine> {
    unsafe { ENGINE.as_mut() }
}

/// 装载词典 bin（open/hanhuo_open.bin 等字节流）。返回词条数, 0=失败。
#[wasm_bindgen]
pub fn init(dict: &[u8]) -> usize {
    match hanhuo_core::Engine::open_bytes(dict) {
        Ok(e) => {
            unsafe { ENGINE = Some(e) };
            engine().map(|e| e.n_words).unwrap_or(0)
        }
        Err(msg) => {
            console_log(&format!("[wasm] init 失败: {msg}"));
            0
        }
    }
}

/// 注入 NNLM 模型字节（mobile/advanced 变体自动路由——CLI
/// attach_model_files 同律; advanced 需 core safetensors feature, 本
/// crate 已开）。返回 true=装载成功。B7/c.3 等 NNLM 消费方即刻换脑。
#[wasm_bindgen]
pub fn init_nnlm(bytes: &[u8]) -> bool {
    let Some(e) = engine() else {
        console_log("[wasm] init_nnlm: 先 init 词典");
        return false;
    };
    match hanhuo_core::nnlm::Nnlm::from_safetensors(bytes) {
        Ok(m) => {
            e.set_nnlm(m);
            true
        }
        Err(_) => {
            match hanhuo_lm::nnlm_adv::NnlmAdv::from_safetensors(bytes) {
                Ok(m) => {
                    e.set_nnlm_adv(m);
                    true
                }
                Err(msg) => {
                    console_log(&format!("[wasm] nnlm 装载失败: {msg}"));
                    false
                }
            }
        }
    }
}

/// 开启宽档 boost 配方（2026-11-20 升默认口径: 尾槽 B2.5 + B7 句序列
/// 重排 + beam150 段变体束——CLI wire_boost 的 --boost-tail
/// --boost-sent-nn 同律; jianpin_inject 同 wire_boost 默认）。返回
/// true=成功。词典未 init → false。
#[wasm_bindgen]
pub fn enable_boost() -> bool {
    let Some(e) = engine_mut() else {
        console_log("[wasm] enable_boost: 先 init 词典");
        return false;
    };
    e.set_boost(hanhuo_core::engine::BoostCfg {
        sentence_first_min_len: 8,
        sentence_tail_sweep: 6,
        sentence_nnlm_rescore: true,
        sentence_seg_beam: 150,
        jianpin_inject: true,
        ..hanhuo_core::engine::BoostCfg::default()
    });
    true
}

/// 注入英文词表（data/en_open.tsv 字节, tsv; 四小件英文通道）。
/// 返回 true=装载成功。候选见 en_candidates。
#[wasm_bindgen]
pub fn init_en(bytes: &[u8]) -> bool {
    match hanhuo_dict::smallfeat::EnTable::init_bytes(bytes) {
        true => {
            console_log("[wasm] en 词表已注入");
            true
        }
        false => {
            console_log("[wasm] en 词表装载失败");
            false
        }
    }
}

/// 注入英文 LM（enlm safetensors 字节, NnlmAdv advanced 变体——CLI
/// --enlm 同律）。注入后 en_candidates 自动切 LM 语境序。
#[wasm_bindgen]
pub fn init_enlm(bytes: &[u8]) -> bool {
    match hanhuo_lm::nnlm_adv::NnlmAdv::from_safetensors(bytes) {
        Ok(m) => {
            unsafe { ENLM = Some(m) };
            console_log("[wasm] enlm 已注入");
            true
        }
        Err(msg) => {
            console_log(&format!("[wasm] enlm 装载失败: {msg}"));
            false
        }
    }
}

/// 英文候选（纯字母输入; 精确词优先 + 前缀补全频序——CLI --en 同律,
/// enlm 在场时切 LM 语境序: P(w|BOS) 降序, 词表外保序尾）。返回 JSON
/// {cands:[{word,rank}]}。
#[wasm_bindgen]
pub fn en_candidates(inp: &str, topk: usize) -> String {
    let Some(t) = hanhuo_dict::smallfeat::EnTable::injected() else {
        return r#"{"cands":[]}"#.into();
    };
    if inp.is_empty() || !inp.chars().all(|c| c.is_ascii_alphabetic()) {
        return r#"{"cands":[]}"#.into();
    }
    let mut cands = t.candidates(inp);
    // enlm 语境序（CLI cmd_query --enlm 同律: predict(0,None) 取 BOS
    // 分布, lp 降序; 词表外 -30.0 保序尾）
    if let Some(m) = enlm() {
        let o = m.predict(0, None);
        let lp_of = |w: &str| -> f32 {
            m.vocab
                .get(w)
                .and_then(|&i| o.lp.get(i as usize).copied())
                .unwrap_or(-30.0)
        };
        cands.sort_by(|a, b| {
            let (wa, _) = a.split_once('\t').unwrap_or((a, ""));
            let (wb, _) = b.split_once('\t').unwrap_or((b, ""));
            lp_of(wb)
                .partial_cmp(&lp_of(wa))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let items: Vec<String> = cands
        .iter()
        .take(topk)
        .map(|s| {
            let (w, r) = s.split_once('\t').unwrap_or((s, "0"));
            format!("{{\"word\":\"{w}\",\"rank\":{r}}}")
        })
        .collect();
    format!("{{\"cands\":[{}]}}", items.join(","))
}

/// T9 九宫模式开关（纯 2-9 数字键串走九宫展开——设计-T9九宫输入;
/// CLI --t9 同律, on=false 恢复默认关）。返回 true=成功。
#[wasm_bindgen]
pub fn enable_t9(on: bool) -> bool {
    let Some(e) = engine_mut() else {
        console_log("[wasm] enable_t9: 先 init 词典");
        return false;
    };
    e.set_t9(on);
    true
}

/// 查询 → JSON 候选串（word/kind/score/logp/reading 数组）。
#[wasm_bindgen]
pub fn query(inp: &str, ctx: &str, topk: usize) -> String {
    let Some(e) = engine() else {
        return r#"{"error":"未 init"}"#.into();
    };
    let ctx_opt = if ctx.is_empty() { None } else { Some(ctx) };
    // CLI 默认同律（cmd_query b28_default）: query_typo_retry 通道——
    // 直接 e.query(dp=false) 在长句上只出 faithful 单字列, boost 的
    // B0 句前置面走 typo_retry 内部的 dp 重建
    let cs = e.query_typo_retry(inp, ctx_opt, topk, false, false);
    let esc = |w: &str| -> String {
        w.chars()
            .map(|c| match c {
                '"' => "\\\"".into(),
                '\\' => "\\\\".into(),
                c if (c as u32) < 0x20 => String::new(),
                c => c.to_string(),
            })
            .collect()
    };
    let items: Vec<String> = cs
        .iter()
        .map(|c| {
            format!(
                "{{\"word\":\"{}\",\"kind\":\"{}\",\"score\":{:.4},\"logp\":{:.4}}}",
                esc(&c.word), c.kind.as_str(), c.score, c.logp
            )
        })
        .collect();
    format!("{{\"query\":\"{inp}\",\"n\":{},\"cands\":[{}]}}",
            cs.len(), items.join(","))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn console_log(s: &str) {
    log(s);
}
