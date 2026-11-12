//! 四小件开源功能表（2026-11-18 数据层配套消费点）:
//! - EnTable: 英文候选（纯字母输入的词表前缀匹配, 频序）
//! - Awb: 敏感词过滤（候选词含敏感词→滤除）
//! - EmojiTable: 候选词命中→emoji 变体注入
//! - TcdTable: 候选词命中错写→正写变体注入
//!
//! 数据件（data/, 全开源; 缺席自动禁用）:
//!   en_open.tsv（49,130 词）/ awb_open.txt（8,857）/
//!   emoji_open.tsv（5,216）/ tcd_open.tsv（6,957 对）
//! 环境钮: HANHUO_EN / HANHUO_AWB / HANHUO_EMOJI / HANHUO_TCD 覆盖路径。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

fn data_path(env: &str, default: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var(env) {
        let pb = PathBuf::from(p);
        return if pb.exists() { Some(pb) } else { None };
    }
    for cand in ["data", "../../data"] {
        let p = PathBuf::from(cand).join(default);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// 英文词表（频序 rank 升序 = 频次降序）。
pub struct EnTable {
    /// 前缀 → 词序（rank 升序, 截 8）; 精确词也入（含 rank）
    prefix: HashMap<String, Vec<(String, u32)>>,
    exact: HashMap<String, u32>,
}

/// 内存注入表（wasm 等无文件 IO 环境; 与 get() 文件表独立——
/// injected() 优先消费点自行决定）。
static EN_INJECTED: OnceLock<Option<EnTable>> = OnceLock::new();

impl EnTable {
    /// tsv 解析（文件/内存双入口共用; 列 = 词\t频序rank）。
    fn parse(data: &str) -> Option<EnTable> {
        let mut prefix: HashMap<String, Vec<(String, u32)>> =
            HashMap::new();
        let mut exact = HashMap::new();
        for ln in data.lines() {
            if ln.starts_with('#') || ln.is_empty() {
                continue;
            }
            let mut f = ln.splitn(2, '\t');
            let (Some(w), Some(r)) = (f.next(), f.next()) else {
                continue;
            };
            let Ok(rank) = r.trim().parse::<u32>() else { continue };
            exact.insert(w.to_string(), rank);
            for l in 2..=w.len() {
                prefix.entry(w[..l].to_string())
                    .or_default()
                    .push((w.to_string(), rank));
            }
        }
        for v in prefix.values_mut() {
            v.truncate(8); // 每 prefix 截 8（rank 升序已保）
        }
        eprintln!("[en] 英文词表: {} 词", exact.len());
        Some(EnTable { prefix, exact })
    }

    pub fn get() -> Option<&'static EnTable> {
        static T: OnceLock<Option<EnTable>> = OnceLock::new();
        T.get_or_init(|| {
            let p = data_path("HANHUO_EN", "en_open.tsv")?;
            let data = std::fs::read_to_string(p).ok()?;
            Self::parse(&data)
        })
        .as_ref()
    }

    /// 内存注入（wasm 等无文件 IO 环境; CLI/原生路径仍走 get() 惰性
    /// 文件装载, 已注入则优先）。
    pub fn init_bytes(bytes: &[u8]) -> bool {
        let data = String::from_utf8_lossy(bytes);
        let _ = EN_INJECTED.set(Self::parse(&data));
        EN_INJECTED.get().and_then(|o| o.as_ref()).is_some()
    }

    /// 注入表优先（wasm）; 未注入回落文件表（原生）。
    pub fn injected() -> Option<&'static EnTable> {
        EN_INJECTED
            .get()
            .and_then(|o| o.as_ref())
            .or_else(Self::get)
    }

    /// 候选（精确词优先, 再前缀补全; 频序）
    pub fn candidates(&self, inp: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(r) = self.exact.get(inp) {
            out.push(format!("{inp}\t{r}"));
        }
        if let Some(vs) = self.prefix.get(inp) {
            for (w, r) in vs {
                if w != inp {
                    out.push(format!("{w}\t{r}"));
                }
            }
        }
        out
    }
}

/// 敏感词表（HashSet; 词级「包含即滤」）。
pub struct Awb {
    words: std::collections::HashSet<String>,
}

impl Awb {
    pub fn get() -> Option<&'static Awb> {
        static T: OnceLock<Option<Awb>> = OnceLock::new();
        T.get_or_init(|| {
            let p = data_path("HANHUO_AWB", "awb_open.txt")?;
            let words: Vec<String> = std::fs::read_to_string(p)
                .ok()?
                .lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .map(|l| l.trim().to_string())
                .collect();
            eprintln!("[awb] 敏感词表: {} 条", words.len());
            Some(Awb {
                words: std::collections::HashSet::from_iter(words),
            })
        })
        .as_ref()
    }

    pub fn is_sensitive(&self, text: &str) -> bool {
        self.words.iter().any(|w| text.contains(w.as_str()))
    }
}

/// emoji 映射（词 → emoji 列表）。
pub struct EmojiTable {
    map: HashMap<String, Vec<String>>,
}

impl EmojiTable {
    pub fn get() -> Option<&'static EmojiTable> {
        static T: OnceLock<Option<EmojiTable>> = OnceLock::new();
        T.get_or_init(|| {
            let p = data_path("HANHUO_EMOJI", "emoji_open.tsv")?;
            let mut map = HashMap::new();
            for ln in std::fs::read_to_string(p).ok()?.lines() {
                if ln.starts_with('#') || ln.is_empty() {
                    continue;
                }
                let mut f = ln.splitn(2, '\t');
                let (Some(w), Some(es)) = (f.next(), f.next()) else {
                    continue;
                };
                let vs: Vec<String> = es
                    .split_whitespace()
                    .map(std::borrow::ToOwned::to_owned)
                    .collect();
                if !vs.is_empty() {
                    map.insert(w.to_string(), vs);
                }
            }
            eprintln!("[emoji] 映射表: {} 词", map.len());
            Some(EmojiTable { map })
        })
        .as_ref()
    }

    pub fn lookup(&self, word: &str) -> Option<&[String]> {
        self.map.get(word).map(|v| v.as_slice())
    }
}

/// 错别字对（错写 → 正写）。
pub struct TcdTable {
    map: HashMap<String, String>,
}

impl TcdTable {
    pub fn get() -> Option<&'static TcdTable> {
        static T: OnceLock<Option<TcdTable>> = OnceLock::new();
        T.get_or_init(|| {
            let p = data_path("HANHUO_TCD", "tcd_open.tsv")?;
            let mut map = HashMap::new();
            for ln in std::fs::read_to_string(p).ok()?.lines() {
                if ln.starts_with('#') || ln.is_empty() {
                    continue;
                }
                let mut f = ln.splitn(3, '\t');
                let (Some(w), Some(t), _) =
                    (f.next(), f.next(), f.next())
                else {
                    continue;
                };
                map.insert(w.to_string(), t.to_string());
            }
            eprintln!("[tcd] 错别字对: {} 条", map.len());
            Some(TcdTable { map })
        })
        .as_ref()
    }

    pub fn correct(&self, word: &str) -> Option<&str> {
        self.map.get(word).map(|s| s.as_str())
    }
}
