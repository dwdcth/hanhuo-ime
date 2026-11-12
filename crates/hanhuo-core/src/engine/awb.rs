//! 敏感词全管线封杀（用户语义裁定 2026-11-19: 命中词库的输入
//! **无法组词、联想、形成记忆、永远不出现在候选中**——结构性
//! 缺席, 非输出过滤）。lmcn 同款全局 lazy 表（core 无 dict 依赖）。
//!
//! 消费点: Engine::query / query_typo_retry 出口统一滤（罩住
//! 忠实面/B0-B8 boost/ckvd 注入/组词全通道）; 用户词典写入拒绝
//! （「不形成记忆」）。HANHUO_AWB 覆盖路径; 未装载零行为。

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

/// 词长扫描窗（敏感词典型 2-12 字; 超长词截此窗内必含其前缀段）
const MAX_LEN: usize = 12;

struct AwbSet {
    words: HashSet<String>,
}

static AWB: OnceLock<Option<AwbSet>> = OnceLock::new();

/// 装载（显式 init 或首次 is_sensitive 时按默认路径惰性装载）。
pub fn init(path: &Path) -> bool {
    let load = || -> Option<AwbSet> {
        let words: HashSet<String> = std::fs::read_to_string(path)
            .ok()?
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|l| l.trim().to_string())
            .filter(|l| l.len() >= 3) // UTF-8: ≥2 汉字
            .collect();
        eprintln!("[awb] 全管线敏感词表: {} 条", words.len());
        Some(AwbSet { words })
    };
    // OnceLock 已初始化则不重载（测试隔离由进程边界保证）
    let _ = AWB.set(load());
    AWB.get().and_then(|o| o.as_ref()).is_some()
}

fn table() -> Option<&'static AwbSet> {
    AWB.get_or_init(|| {
        let p = std::env::var("HANHUO_AWB").ok()?;
        let pb = std::path::PathBuf::from(p);
        if !pb.exists() {
            return None;
        }
        init(&pb);
        None // 显式 init 优先; 惰性路径不重复装载
    })
    .as_ref()
}

/// 候选词命中判定（候选子串枚举查表——O(len²) 哈希查, 免 AC 自动机）。
pub fn is_sensitive(word: &str) -> bool {
    let Some(t) = table() else { return false };
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 2 {
        return false;
    }
    for l in 2..=chars.len().min(MAX_LEN) {
        for i in 0..=chars.len() - l {
            let seg: String = chars[i..i + l].iter().collect();
            if t.words.contains(&seg) {
                return true;
            }
        }
    }
    false
}

/// 候选出口统一滤（query / query_typo_retry 尾部调用）。
pub fn filter_out(out: &mut Vec<crate::Cand>) {
    if table().is_none() {
        return;
    }
    let n = out.len();
    out.retain(|c| !is_sensitive(&c.word));
    if out.len() != n {
        eprintln!("[awb] 封杀 {} 条", n - out.len());
    }
}
