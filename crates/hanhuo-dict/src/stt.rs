//! S2t —— stt.bin 简→繁词典（SimplifiedToTraditionalDict，55,313 词对）
//!
//! 格式（B 档轮 XIV 全解，stt_extract.py 镜像）：
//! - 文件：[4B 前缀][magic 0x80000001][count u32][offsets u32×count 相对+4]
//!   [桶区][尾 32B]；类 wxime::data::dict::SimplifiedToTraditionalDict
//!   （kind 36，查询 0x1806A2A00 → 0x1806A2F40）
//! - 桶律（表族通律）：bucket = bhash(简体词) % count，比对 = k2(简体词)
//! - 桶 = [n u32][n×u32 k2][n×u16 累计槽尾][槽区]
//! - 槽 = [nrec u8] + nrec × [x u8][len u8][len B 载荷]，
//!   载荷 = UTF-8 繁体词**每字节 +1 存储**（原生 SIMD 减 0x01 还原）
//! - 一对多保留（里→裡/里），首记录为主变体
//!
//! 数据归腾讯，仅本地个人使用，禁止分发。

use anyhow::{Context, Result};

use crate::bhash::bucket_of;
use crate::wxhash::k2;

/// 简→繁词典（mmap 直读，惰性桶查询）
pub struct S2t {
    data: Vec<u8>,
    count: usize,
}

impl S2t {
    /// 从 stt.bin 装载（全量读入 1.2MB；词表静态无 mmap 必要性）
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path.as_ref())
            .with_context(|| format!("无法打开 {}", path.as_ref().display()))?;
        if data.len() < 16 {
            anyhow::bail!("文件过小: {}B", data.len());
        }
        let count = u32::from_le_bytes(data[8..12].try_into()?) as usize;
        let table_end = 12 + count * 4 + 4;
        if table_end > data.len() {
            anyhow::bail!("偏移表越界");
        }
        eprintln!("[stt] 装载完成: {count} 桶, {}B", data.len());
        Ok(Self { data, count })
    }

    #[inline]
    fn offset(&self, i: usize) -> u32 {
        u32::from_le_bytes(
            self.data[12 + i * 4..12 + i * 4 + 4].try_into().unwrap(),
        )
    }

    /// 词级查询：返回全部繁体变体（首项为主变体）
    pub fn s2t_word(&self, word: &str) -> Option<Vec<String>> {
        let key = word.as_bytes();
        if key.is_empty() || key.len() > 32 {
            return None;
        }
        let h = k2(key);
        let b = bucket_of(key, self.count as u32) as usize;
        let s = 4 + self.offset(b) as usize;
        let e = if b + 1 < self.count {
            4 + self.offset(b + 1) as usize
        } else {
            4 + self.data.len() - 37
        };
        if e <= s {
            return None;
        }
        let n = u32::from_le_bytes(self.data[s..s + 4].try_into().ok()?) as usize;
        if n == 0 || 4 + 6 * n > e - s {
            return None;
        }
        // 哈希区扫描
        let mut slot_end = None;
        for j in 0..n {
            let hj = u32::from_le_bytes(
                self.data[s + 4 + j * 4..s + 4 + j * 4 + 4].try_into().ok()?,
            );
            if hj == h {
                slot_end = Some(j);
                break;
            }
        }
        let j = slot_end?;
        // 累计槽尾表（u16）定位槽字节区
        let cum = |idx: usize| -> u16 {
            u16::from_le_bytes(
                self.data[s + 4 + 4 * n + idx * 2..s + 4 + 4 * n + idx * 2 + 2]
                    .try_into()
                    .unwrap(),
            )
        };
        let slot_start = s + 4 + 6 * n;
        let p0 = if j == 0 { 0 } else { cum(j - 1) as usize };
        let p1 = cum(j) as usize;
        let slot = self.data.get(slot_start + p0..slot_start + p1)?;
        Self::decode_slot(slot)
    }

    /// 槽解码：[nrec u8] + nrec × [x u8][len u8][len B (每字节+1)] → 繁体词表
    fn decode_slot(s: &[u8]) -> Option<Vec<String>> {
        if s.is_empty() {
            return None;
        }
        let nrec = s[0] as usize;
        let mut recs = Vec::with_capacity(nrec);
        let mut p = 1usize;
        for _ in 0..nrec {
            if p + 2 > s.len() {
                return if recs.is_empty() { None } else { Some(recs) };
            }
            let len = s[p + 1] as usize;
            if p + 2 + len > s.len() {
                return None;
            }
            let raw: Vec<u8> = s[p + 2..p + 2 + len].iter().map(|&b| b.wrapping_sub(1)).collect();
            recs.push(String::from_utf8(raw).ok()?);
            p += 2 + len;
        }
        if p == s.len() {
            Some(recs)
        } else if recs.is_empty() {
            None
        } else {
            Some(recs)
        }
    }

    /// 文本转繁（输出层 API）：贪心最长词匹配（≤8 字），词级 miss 回退单字，
    /// 均无条目则原样保留。多变体取首项（主变体）。
    pub fn s2t_text(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len() + text.len() / 4);
        let mut i = 0usize;
        while i < chars.len() {
            let mut matched = false;
            // 最长优先（词级，≤8 字）
            let max_len = (chars.len() - i).min(8);
            for l in (2..=max_len).rev() {
                let w: String = chars[i..i + l].iter().collect();
                if let Some(vs) = self.s2t_word(&w) {
                    if let Some(first) = vs.first() {
                        out.push_str(first);
                        i += l;
                        matched = true;
                        break;
                    }
                }
            }
            if matched {
                continue;
            }
            // 单字回退（单字也在 stt 内：里→裡/里）
            let c: String = chars[i].to_string();
            match self.s2t_word(&c) {
                Some(vs) if !vs.is_empty() => out.push_str(&vs[0]),
                _ => out.push(chars[i]),
            }
            i += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stt_path() -> std::path::PathBuf {
        std::env::var("STT_BIN").map_or_else(
            |_| {
                std::path::PathBuf::from(
                    "/home/banshan/tmp/hanhuo/extracted/3.0.1.1/BuildIn/stt.bin",
                )
            },
            std::path::PathBuf::from,
        )
    }

    #[test]
    #[ignore = "需要 stt.bin"]
    fn test_word_anchors() {
        // 提取器锚点（stt_extract 实证）:
        // 里→[裡,里](一对多首项=裡), 头发→[頭髮], 干杯→[乾杯], 皇后→[皇后]
        let s2t = S2t::from_file(stt_path()).unwrap();
        assert_eq!(s2t.s2t_word("里").unwrap()[0], "裡");
        assert_eq!(s2t.s2t_word("头发").unwrap()[0], "頭髮");
        assert_eq!(s2t.s2t_word("干杯").unwrap()[0], "乾杯");
        assert_eq!(s2t.s2t_word("皇后").unwrap()[0], "皇后");
        assert!(s2t.s2t_word("中国").is_none(), "中国无词级条目(走单字回退)");
    }

    #[test]
    #[ignore = "需要 stt.bin"]
    fn test_text_conversion() {
        let s2t = S2t::from_file(stt_path()).unwrap();
        // 词级: 头发→頭髮
        assert_eq!(s2t.s2t_text("头发"), "頭髮");
        // 词级 miss + 单字回退: 中国→中(无条目保留)+國
        assert_eq!(s2t.s2t_text("中国"), "中國");
        // 混合: 整词 "干了什么" 在典(→乾了什麼)，最长匹配优先于 "干了"(→幹了)
        assert_eq!(s2t.s2t_text("干了什么"), "乾了什麼");
        // 而孤立 "干了" 单独成词时: 干了→幹了
        assert_eq!(s2t.s2t_text("干了"), "幹了");
    }

    #[test]
    #[ignore = "需要 stt.bin"]
    fn test_bucket_law() {
        // 表族桶律回归: 抽样词的 k2 必在其 bhash 桶内
        let s2t = S2t::from_file(stt_path()).unwrap();
        let words = ["里", "头发", "干杯", "皇后", "发", "了", "面", "干"];
        let mut hit = 0;
        for w in words {
            if s2t.s2t_word(w).is_some() {
                hit += 1;
            }
        }
        // 8 词全在 stt（Python 7/7 实证 + 干）
        assert!(hit >= 7, "桶律命中 {hit}/8");
    }

    #[test]
    fn test_decode_slot_unit() {
        // 手工构造: nrec=1, [x=0][len=6] 载荷="頭髮"每字节+1
        let payload: Vec<u8> = "頭髮".bytes().map(|b| b + 1).collect();
        let mut slot = vec![1u8, 0, payload.len() as u8];
        slot.extend_from_slice(&payload);
        assert_eq!(S2t::decode_slot(&slot).unwrap(), vec!["頭髮".to_string()]);
        // 空槽
        assert!(S2t::decode_slot(&[]).is_none());
    }
}


/// 开源 TSV 变体（c.6 审计后续: opencc 字典导出, `简 \t 繁1 繁2` 行）
/// ——同 s2t_word 语义; s2t_text 经 S2tAny 分发。
pub struct S2tTsv {
    map: std::collections::HashMap<String, Vec<String>>,
}

impl S2tTsv {
    pub fn from_tsv_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self> {
        let mut map = std::collections::HashMap::new();
        for ln in std::fs::read_to_string(path.as_ref())?.lines() {
            let mut f = ln.splitn(2, '\t');
            let (Some(k), Some(v)) = (f.next(), f.next()) else {
                continue;
            };
            let vs: Vec<String> = v.split_whitespace()
                .map(std::borrow::ToOwned::to_owned)
                .collect();
            if !k.is_empty() && !vs.is_empty() {
                map.entry(k.to_string())
                    .or_insert_with(|| vs.clone());
            }
        }
        eprintln!("[stt] 开源 TSV 装载: {} 条", map.len());
        Ok(Self { map })
    }

    pub fn s2t_word(&self, word: &str) -> Option<Vec<String>> {
        self.map.get(word).cloned()
    }

    /// 反向表（t2s: 值→键聚合, 多值保序）——tts 等价开源重建
    pub fn reversed(&self) -> S2tTsv {
        let mut rev: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (k, vs) in &self.map {
            for v in vs {
                rev.entry(v.clone()).or_default().push(k.clone());
            }
        }
        eprintln!("[t2s] 反向表构建: {} 条", rev.len());
        Self { map: rev }
    }
}

/// 二进制 / 开源 TSV 双路分发（cmd_stt 按扩展名择一）
pub enum S2tAny {
    Bin(S2t),
    Tsv(S2tTsv),
}

impl S2tAny {
    pub fn s2t_word(&self, w: &str) -> Option<Vec<String>> {
        match self {
            S2tAny::Bin(s) => s.s2t_word(w),
            S2tAny::Tsv(s) => s.s2t_word(w),
        }
    }

    /// 最长优先词级（≤8 字）+ 单字回退（S2t::s2t_text 同律）
    pub fn s2t_text(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len() + text.len() / 4);
        let mut i = 0usize;
        while i < chars.len() {
            let mut matched = false;
            let max_len = (chars.len() - i).min(8);
            for l in (2..=max_len).rev() {
                let w: String = chars[i..i + l].iter().collect();
                if let Some(vs) = self.s2t_word(&w) {
                    if let Some(first) = vs.first() {
                        out.push_str(first);
                        i += l;
                        matched = true;
                        break;
                    }
                }
            }
            if matched {
                continue;
            }
            let c: String = chars[i].to_string();
            match self.s2t_word(&c) {
                Some(vs) if !vs.is_empty() => out.push_str(&vs[0]),
                _ => out.push(chars[i]),
            }
            i += 1;
        }
        out
    }
}
