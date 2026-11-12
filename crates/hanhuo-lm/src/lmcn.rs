//! lmcn 域: 表装载与面打分（cascade 目录, 缺省全域激活）.

// ================= 役N tick45: lmcn KN 双表（第二转移源) =================
// S44-45 定谳: face_score = Σ lmcn_unigram_lp + trans(首词对, 表内直查/
// 缺对→alpha(w1)) − 成本; 表=外包回收产物（外部路径, 腾讯衍生仅自用）。
// WXQ_LMCN_FACE: 缺省/'1'=全域 / '2'=G1 门 / '0'=关（tick45 激活: 缺省开）
pub const LMCN_DIR: &str = "/home/banshan/tmp/hanhuo/cascade";

/// 目录解析（含 c.6 开源重建表覆盖钮——不动腾讯原表, A/B 切换用）
pub fn lmcn_dir() -> String {
    std::env::var("HANHUO_LMCN_DIR").unwrap_or_else(|_| LMCN_DIR.into())
}

pub const LMCN_WORD_FLOOR: f64 = -6.76; // 缺词地板=2 字词中位（singleton 档中性化）

pub fn lmcn_mode() -> u8 {
    // tick47 激活: 缺省/'1'=全域 / '2'=G1 门 / '0'=关（py 对称;
    // 双端 79/120+564, 面集 923/923 逐位咬合）
    match std::env::var("WXQ_LMCN_FACE").as_deref() {
        Ok("0") => 0,
        Ok("2") => 2,
        _ => 1,
    }
}

/// (uni, bg) 双表（tick66: pub —— channels_fuzz EMIX 域 uni 直读消费,
/// py `_lmcn_tables` 镜像）。
pub fn lmcn_tables(
) -> &'static (std::collections::HashMap<String, (f64, f64)>,
               std::collections::HashMap<(String, String), f64>) {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static T: OnceLock<(HashMap<String, (f64, f64)>,
                        HashMap<(String, String), f64>)> = OnceLock::new();
    T.get_or_init(|| {
        let mut uni = HashMap::new();
        let mut bg = HashMap::new();
        let load = |p: &str| -> Option<String> {
            std::fs::read_to_string(p).ok()
        };
        // 缺表告警（golden 跨机差 541/611 破案教训: 静默空表降级把
        // 「数据没同步」伪装成「跨机数值玄学」——两表任一缺席即 stderr
        // 一条, 进程内仅此一次[OnceLock 天然保证]）
        {
            let bg_ok =
                std::path::Path::new(&lmcn_dir()).join("lmcn_bigram_final.tsv");
            let uni_ok = std::path::Path::new(&lmcn_dir())
                .join("lmcn_unigram_full.tsv");
            if !bg_ok.exists() || !uni_ok.exists() {
                eprintln!(
                    "[lmcn] 警告: {LMCN_DIR} 双表不完整（uni={} bg={}）\
                     ——空表降级运行, fcomp/sentence 面分将偏离基线",
                    uni_ok.exists(),
                    bg_ok.exists()
                );
            }
        }
        if let Some(txt) =
            load(&format!("{}/lmcn_bigram_final.tsv", lmcn_dir()))
        {
            for ln in txt.lines() {
                if ln.starts_with('#') {
                    continue;
                }
                let f: Vec<&str> = ln.split('\t').take(3).collect();
                if f.len() == 3 {
                    if let Ok(v) = f[2].parse::<f64>() {
                        bg.insert((f[0].into(), f[1].into()), v);
                    }
                }
            }
        }
        if let Some(txt) =
            load(&format!("{}/lmcn_unigram_full.tsv", lmcn_dir()))
        {
            for ln in txt.lines() {
                if ln.starts_with('#') {
                    continue;
                }
                let f: Vec<&str> = ln.split('\t').take(3).collect();
                if f.len() == 3 && f[1] != "1.000000" {
                    if let (Ok(a), Ok(b)) =
                        (f[1].parse::<f64>(), f[2].parse::<f64>())
                    {
                        uni.insert(f[0].into(), (a, b));
                    }
                }
            }
        }
        (uni, bg)
    })
}

/// lmcn 排序数组索引（LMC1 缓存格式, mmap 零拷贝二分）——
/// 替代每进程 203ms 的 TSV→HashMap 解析（uni 353k + bg 365k, 64MB）。
/// 布局: [magic LMC1][ver u32][n_uni u32][n_bg u32]
///   uni 区 n×24B {off u32, len u16, pad u16, a f64, b f64}（词字典序）
///   bg 区 n×24B {off1 u32, len1 u16, pad, off2 u32, len2 u16, pad, v f64}
///        （(w1,w2) 字典序; 字段和=24B——曾误算 28B 步长致键区整体错位）
///   blob 区 词字节
/// 缓存缺席时回落 TSV 构建（与 lmcn_tables 同源 HashMap → 数据等价）
/// 并尽力回写缓存（自愈部署）。
pub struct LmcnIdx {
    bytes: &'static [u8],
    n_uni: usize,
    n_bg: usize,
    uni_off: usize,
    bg_off: usize,
    blob_off: usize,
}

fn le32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}
fn le16(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn lef64(b: &[u8], p: usize) -> f64 {
    f64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}

impl LmcnIdx {
    fn from_bytes(b: &'static [u8]) -> LmcnIdx {
        let n_uni = le32(b, 8) as usize;
        let n_bg = le32(b, 12) as usize;
        let uni_off = 16;
        let bg_off = uni_off + n_uni * 24;
        let blob_off = bg_off + n_bg * 24;
        LmcnIdx { bytes: b, n_uni, n_bg, uni_off, bg_off, blob_off }
    }
    fn uni_row(&self, i: usize) -> (&[u8], f64, f64) {
        let r = self.uni_off + i * 24;
        let off = le32(self.bytes, r) as usize;
        let len = le16(self.bytes, r + 4) as usize;
        (
            &self.bytes[self.blob_off + off..self.blob_off + off + len],
            lef64(self.bytes, r + 8),
            lef64(self.bytes, r + 16),
        )
    }
    fn bg_row(&self, i: usize) -> (&[u8], &[u8], f64) {
        let r = self.bg_off + i * 24;
        let o1 = le32(self.bytes, r) as usize;
        let l1 = le16(self.bytes, r + 4) as usize;
        let o2 = le32(self.bytes, r + 8) as usize;
        let l2 = le16(self.bytes, r + 12) as usize;
        (
            &self.bytes[self.blob_off + o1..self.blob_off + o1 + l1],
            &self.bytes[self.blob_off + o2..self.blob_off + o2 + l2],
            lef64(self.bytes, r + 16),
        )
    }
    /// uni 二分（词字节字典序）
    pub fn get_uni(&self, w: &str) -> Option<(f64, f64)> {
        let (mut a, mut b) = (0usize, self.n_uni);
        while a < b {
            let m = (a + b) / 2;
            let (k, x, y) = self.uni_row(m);
            match k.cmp(w.as_bytes()) {
                std::cmp::Ordering::Less => a = m + 1,
                std::cmp::Ordering::Greater => b = m,
                std::cmp::Ordering::Equal => return Some((x, y)),
            }
        }
        None
    }
    pub fn contains_uni(&self, w: &str) -> bool {
        self.get_uni(w).is_some()
    }
    /// bg 二分（(w1,w2) 字典序）
    pub fn get_bg(&self, w1: &str, w2: &str) -> Option<f64> {
        let (mut a, mut b) = (0usize, self.n_bg);
        while a < b {
            let m = (a + b) / 2;
            let (k1, k2, v) = self.bg_row(m);
            let ord = k1
                .cmp(w1.as_bytes())
                .then_with(|| k2.cmp(w2.as_bytes()));
            match ord {
                std::cmp::Ordering::Less => a = m + 1,
                std::cmp::Ordering::Greater => b = m,
                std::cmp::Ordering::Equal => return Some(v),
            }
        }
        None
    }
    pub fn uni_len(&self) -> usize {
        self.n_uni
    }
    /// 调试: 第 i 行词字节（行序完整性检查用）
    pub fn uni_key_at(&self, i: usize) -> &[u8] {
        let r = self.uni_off + i * 24;
        let off = le32(self.bytes, r) as usize;
        let len = le16(self.bytes, r + 4) as usize;
        &self.bytes[self.blob_off + off..self.blob_off + off + len]
    }
    pub fn is_empty(&self) -> bool {
        self.n_uni == 0
    }

    /// 从 lmcn_tables 的 HashMap 构建 LMC1 布局（先按词字节排序, 写行时
    /// 顺序赋 blob 偏移——闭包借用版有寿命纠纷, 直白两遍式）
    fn build_layout(
        uni: &std::collections::HashMap<String, (f64, f64)>,
        bg: &std::collections::HashMap<(String, String), f64>,
    ) -> Vec<u8> {
        let mut uni_items: Vec<(&str, f64, f64)> =
            uni.iter().map(|(w, &(a, b))| (w.as_str(), a, b)).collect();
        uni_items.sort_by(|x, y| x.0.as_bytes().cmp(y.0.as_bytes()));
        let mut bg_items: Vec<(&str, &str, f64)> = bg
            .iter()
            .map(|((w1, w2), &v)| (w1.as_str(), w2.as_str(), v))
            .collect();
        bg_items
            .sort_by(|x, y| (x.0.as_bytes(), x.1.as_bytes()).cmp(&(y.0.as_bytes(), y.1.as_bytes())));
        let mut blob: Vec<u8> = Vec::new();
        let mut uni_rows: Vec<(u32, u16, f64, f64)> =
            Vec::with_capacity(uni_items.len());
        for (w, a, b) in &uni_items {
            uni_rows.push((blob.len() as u32, w.len() as u16, *a, *b));
            blob.extend_from_slice(w.as_bytes());
        }
        let mut bg_rows: Vec<(u32, u16, u32, u16, f64)> =
            Vec::with_capacity(bg_items.len());
        for (w1, w2, v) in &bg_items {
            let o1 = blob.len() as u32;
            blob.extend_from_slice(w1.as_bytes());
            let o2 = blob.len() as u32;
            blob.extend_from_slice(w2.as_bytes());
            bg_rows.push((o1, w1.len() as u16, o2, w2.len() as u16, *v));
        }
        let mut out: Vec<u8> = Vec::with_capacity(
            16 + uni_rows.len() * 24 + bg_rows.len() * 24 + blob.len(),
        );
        out.extend_from_slice(b"LMC1");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&(uni_rows.len() as u32).to_le_bytes());
        out.extend_from_slice(&(bg_rows.len() as u32).to_le_bytes());
        for r in &uni_rows {
            out.extend_from_slice(&r.0.to_le_bytes());
            out.extend_from_slice(&r.1.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&r.2.to_le_bytes());
            out.extend_from_slice(&r.3.to_le_bytes());
        }
        for r in &bg_rows {
            out.extend_from_slice(&r.0.to_le_bytes());
            out.extend_from_slice(&r.1.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&r.2.to_le_bytes());
            out.extend_from_slice(&r.3.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&r.4.to_le_bytes());
        }
        out.extend_from_slice(&blob);
        out
    }
}

/// lmcn 索引单例: LMC1 缓存 mmap 优先, 缺席回落 TSV 解析（并尽力回写）
pub fn lmcn_index() -> &'static LmcnIdx {
    use std::sync::OnceLock;
    static IDX: OnceLock<LmcnIdx> = OnceLock::new();
    IDX.get_or_init(|| {
        let cache = format!("{}/lmcn_cache.bin", lmcn_dir());
        if let Ok(b) = std::fs::read(&cache) {
            if b.len() >= 16 && &b[..4] == b"LMC1" {
                let leaked: &'static [u8] = Box::leak(b.into_boxed_slice());
                return LmcnIdx::from_bytes(leaked);
            }
        }
        let (uni, bg) = lmcn_tables();
        let bytes = LmcnIdx::build_layout(&uni, &bg);
        let _ = std::fs::write(&cache, &bytes); // 自愈回写（失败容忍）
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        LmcnIdx::from_bytes(leaked)
    })
}

/// 真机等价锁: 缓存索引与 TSV HashMap 全键等值（抽样+全量长度）
#[test]
fn lmcn_build_layout_synth_roundtrip() {
    use std::collections::HashMap;
    let mut uni = HashMap::new();
    uni.insert("空吹".to_string(), (-5.5, -0.5));
    uni.insert("啊".to_string(), (-1.0, -1.0));
    uni.insert("zz".to_string(), (-9.0, 0.0));
    let mut bg: HashMap<(String, String), f64> = HashMap::new();
    bg.insert(("啊".into(), "空吹".into()), -2.5);
    let bytes = LmcnIdx::build_layout(&uni, &bg);
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let idx = LmcnIdx::from_bytes(leaked);
    assert_eq!(idx.uni_len(), 3);
    for i in 0..3usize {
        let r = 16 + i * 24;
        let off = u32::from_le_bytes(leaked[r..r+4].try_into().unwrap());
        let len = u16::from_le_bytes(leaked[r+4..r+6].try_into().unwrap());
        eprintln!("行{i}: off={off} len={len} bytes={:02x?}", &leaked[r..r+8]);
    }
    for (w, v) in &uni {
        assert_eq!(idx.get_uni(w), Some(*v), "uni {w}");
    }
    assert_eq!(idx.get_bg("啊", "空吹"), Some(-2.5));
    assert_eq!(idx.get_bg("x", "y"), None);
}

#[test]
#[ignore = "真机 cascade 目录 + 缓存"]
fn lmcn_idx_equals_tables() {
    let idx = lmcn_index();
    if idx.is_empty() {
        return;
    }
    let (uni, bg) = lmcn_tables();
    assert_eq!(idx.uni_len(), uni.len(), "uni 全量");
    let mut checked = 0;
    for (w, v) in uni.iter() {
        assert_eq!(idx.get_uni(w), Some(*v), "uni {w}");
        checked += 1;
        if checked >= 2000 {
            break; // 抽样 2000
        }
    }
    let mut checked = 0;
    for ((w1, w2), v) in bg.iter() {
        assert_eq!(idx.get_bg(w1, w2), Some(*v), "bg {w1}|{w2}");
        checked += 1;
        if checked >= 2000 {
            break;
        }
    }
}

/// 面 lmcn 域重打: Σ lmcn_lp(词, miss→slot; 缺 2+字词→地板) + trans
/// (首词对: 表内直查/缺→alpha(w1)) − Σ罚 → (sc, lp)
pub fn lmcn_face(
    words: &[String],
    slot_lps: &[f64],
    spens_sum: f64,
    fallback: bool,
) -> (f64, f64) {
    let mode = lmcn_mode();
    if mode == 0 || (mode == 2 && !fallback) {
        let lp: f64 = slot_lps.iter().sum();
        return (lp - spens_sum, lp);
    }
    let idx = lmcn_index();
    let mut lp = 0.0;
    for (w, &sl) in words.iter().zip(slot_lps.iter()) {
        match idx.get_uni(w) {
            Some(u) => {
                if mode != 2 || w.chars().count() >= 2 {
                    lp += u.0;
                } else {
                    lp += sl;
                }
            }
            None => lp += sl,
        }
    }
    if words.len() >= 2 {
        let t = match idx.get_bg(&words[0], &words[1]) {
            Some(v) => v,
            None => idx.get_uni(&words[0]).map(|u| u.1).unwrap_or(0.0),
        };
        lp += t;
    }
    // 缺词地板: 2+ 字词不在 uni → slot 换地板（扫吧 miss→slot 抬头修正）
    for (w, &sl) in words.iter().zip(slot_lps.iter()) {
        if w.chars().count() >= 2 && !idx.contains_uni(w) {
            lp = lp - sl + LMCN_WORD_FLOOR;
        }
    }
    (lp - spens_sum, lp)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// env 语义合并测试（并行竞态防护——env 型断言必须单测试串行）:
    /// 模式解析 / 门控数学（表无关）/ 地板语义（表无关, 生僻词双环境
    /// 均不在 uni）。
    #[test]
    fn lmcn_env_and_face_semantics() {
        let saved = std::env::var("WXQ_LMCN_FACE").ok();
        // ① 模式解析: "0"→0 / "2"→2 / 其余→1 / 缺省→1
        for (v, want) in [("0", 0u8), ("2", 2), ("1", 1), ("x", 1)] {
            std::env::set_var("WXQ_LMCN_FACE", v);
            assert_eq!(lmcn_mode(), want, "WXQ_LMCN_FACE={v}");
        }
        // ② 门控数学: mode 0 与 mode 2 非兜底 → 纯 slot 面
        let words: Vec<String> =
            ["上海", "市政", "府"].iter().map(|s| s.to_string()).collect();
        let slots = [-1.0, -2.0, -3.0];
        for m in ["0", "2"] {
            std::env::set_var("WXQ_LMCN_FACE", m);
            let (sc, lp) = lmcn_face(&words, &slots, 0.5, false);
            assert_eq!(lp, -6.0, "mode {m}: lp=Σslot");
            assert_eq!(sc, -6.5, "mode {m}: sc=lp−pen");
        }
        // ③ 地板语义: 双 2 字缺词 → 2×地板; 缺对转移 → alpha 缺 → 0
        std::env::set_var("WXQ_LMCN_FACE", "1");
        let words2: Vec<String> =
            ["痚瘛", "燚爩"].iter().map(|s| s.to_string()).collect();
        let slots2 = [-1.5, -2.5];
        let (sc, lp) = lmcn_face(&words2, &slots2, 0.0, true);
        let want = 2.0 * LMCN_WORD_FLOOR;
        assert_eq!(lp, want, "双 2 字缺词 → 2×地板");
        assert_eq!(sc, want, "pen=0 → sc=lp");
        // 恢复
        match &saved {
            Some(s) => std::env::set_var("WXQ_LMCN_FACE", s),
            None => std::env::remove_var("WXQ_LMCN_FACE"),
        }
    }

    /// 真机表装载 [ignore]: cascade 目录在场时 uni/bg 非空且含已知键
    /// （上海市/中国 bigram 样例源首行）。缺目录环境自动跳过。
    #[test]
    #[ignore = "真机 cascade 目录"]
    fn lmcn_tables_real_load() {
        let (uni, bg) = lmcn_tables();
        if uni.is_empty() && bg.is_empty() {
            return; // 无 cascade 环境
        }
        assert!(uni.len() > 100_000, "uni 全量表 35 万级, got {}", uni.len());
        assert!(uni.contains_key("上海市"), "首行键");
        assert!(bg.len() > 100_000, "bigram 全量表");
    }
}
