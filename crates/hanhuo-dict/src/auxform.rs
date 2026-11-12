//! AuxForm —— 辅助码码表（融合-辅助码-万象 §3.2, ckvd.rs 同构范式）
//!
//! 双源装载:
//! - wubi86_swbd.dict.yaml（swbd 桶哈希配对反演原生码）:
//!   字级行（单字 × 多码）+ 词级行。多码字取**最长码**为唯一源——
//!   wubi2 = 最长码前 2 位（是 jgh→jg, 满足 §4 验收 `shi\`jg`→是;
//!   文档 §3.2 "最短码" 措辞按此验收例修正: 最短码是一级简码,
//!   你=q/wqiy 前2=wq 两种取法同, 是=j/jgh 前2 只剩 jg 一种可验收）,
//!   head_tail = 最长码首+末位（镇 qfhw→qw）。
//!   一级简码单键（我=q）仍可经子序列匹配命中（q ⊄ tr, 但 shi\`j
//!   的 j ⊆ jg 命中）——码表语义与万象 "五笔前2" 对齐。
//! - rsd_char_strokes.tsv: 字→笔顺数字串（1横 2竖 3撇 4捺/点 5折）
//!   → 字母映射 hspdz（你 3235234→pspzspd, 万象实证逐位吻合）。
//!
//! 词级 word_codes = 逐字 wubi2 串接（一万→[gg,dn]; yaml 词码 ggdn
//! 恰为串接, 逐字切分即得——§2 词组跨字行判定）。缺码字返回 None
//! 不猜（§3.2 锚点律）。
//!
//! 数据归腾讯（wubi86_swbd=swbd 反演 / rsd=rsd.bin 运行时提取）,
//! 仅本地个人研究使用, 禁止分发; 测试资产走本机路径 + #[ignore],
//! 免数据测试用临时目录合成小表。

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// 笔顺数字 → 字母（1横 2竖 3撇 4捺/点 5折）
const STROKE_MAP: [(char, char); 5] =
    [('1', 'h'), ('2', 's'), ('3', 'p'), ('4', 'd'), ('5', 'z')];

fn stroke_letter(d: char) -> Option<char> {
    STROKE_MAP.iter().find(|(k, _)| *k == d).map(|(_, v)| *v)
}

/// 辅助码码表: 字→五笔最长码 + 字→笔顺(hspdz)。
/// 只读容器, 查询 O(1); 装载一次（文本直读 ~1MB, §3.2 PoC 档）。
pub struct AuxForm {
    /// 单字 → 最长五笔码（多码行取最长; 唯一码即本身）
    wubi: HashMap<char, String>,
    /// 单字 → 首末码（最长码首+末; 镇 qfhw→qw; 预计算免查询期分配）
    head_tail: HashMap<char, String>,
    /// 单字 → 笔顺字母串（你→pspzspd）
    strokes: HashMap<char, String>,
    /// 词 → 原生词码（一万→ggdn; word_codes 组合律的对照源）
    words: HashMap<String, String>,
}

impl AuxForm {
    /// 从文本资产装载。
    /// wubi_yaml: wubi86_swbd.dict.yaml（rime dict 格式, tab 分隔 词\t码）
    /// strokes_tsv: rsd_char_strokes.tsv（char\t0xcp\t数字笔顺, 带注释头）
    pub fn from_file(wubi_yaml: &Path, strokes_tsv: &Path) -> Result<AuxForm> {
        let mut wubi: HashMap<char, String> = HashMap::new();
        let mut words: HashMap<String, String> = HashMap::new();

        let data = std::fs::read_to_string(wubi_yaml).with_context(|| {
            format!("无法打开 {}", wubi_yaml.display())
        })?;
        for line in data.lines() {
            // rime dict 头（# 注释 / --- / ... / name: 等元数据）与空行跳过
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("---")
                || line.starts_with("...")
                || line.starts_with("name:")
                || line.starts_with("version:")
                || line.starts_with("sort:")
            {
                continue;
            }
            let Some((word, code)) = line.split_once('\t') else {
                continue;
            };
            let mut cs = word.chars();
            let (first, mut rest) = (cs.next(), cs);
            let Some(ch) = first else { continue };
            if rest.next().is_none() {
                // 字级行: 多码取最长（简码是全码前缀截断, 最长=全码）
                match wubi.entry(ch) {
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(code.to_string());
                    }
                    std::collections::hash_map::Entry::Occupied(mut o) => {
                        if o.get().len() < code.len() {
                            o.insert(code.to_string());
                        }
                    }
                }
            } else {
                // 词级行: 保留原生词码（word_codes 对照/诊断用）
                words.insert(word.to_string(), code.to_string());
            }
        }

        let mut strokes: HashMap<char, String> = HashMap::new();
        let sdata = std::fs::read_to_string(strokes_tsv).with_context(|| {
            format!("无法打开 {}", strokes_tsv.display())
        })?;
        for (ln, line) in sdata.lines().enumerate() {
            if ln == 0 && line.starts_with('#') {
                continue; // 注释头
            }
            if line.starts_with('#') || line.starts_with("char\t") {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                continue;
            }
            let mut cs = parts[0].chars();
            let (ch, mut rest) = (cs.next(), cs);
            let (Some(ch), None) = (ch, rest.next()) else {
                continue;
            };
            // 数字笔顺 → hspdz; 非法位跳过（防御, 资产实测全 1-5）
            let s: String = parts[2]
                .chars()
                .filter_map(stroke_letter)
                .collect();
            if !s.is_empty() {
                strokes.insert(ch, s);
            }
        }

        // head_tail 预计算（最长码首+末; 1 位码首=末）
        let head_tail: HashMap<char, String> = wubi
            .iter()
            .map(|(&ch, code)| {
                let mut it = code.chars();
                let h = it.next().unwrap_or_default();
                let t = it.last().unwrap_or(h);
                (ch, format!("{h}{t}"))
            })
            .collect();

        eprintln!(
            "[auxform] 装载完成: {} 字五笔, {} 字笔顺, {} 词码",
            wubi.len(),
            strokes.len(),
            words.len()
        );
        Ok(Self { wubi, head_tail, strokes, words })
    }

    /// 字 → 五笔前 2 码（呵→ks、你→wq、镇→qf; 最长码前缀位）
    pub fn wubi2(&self, ch: char) -> Option<&str> {
        let code = self.wubi.get(&ch)?;
        Some(&code[..code.len().min(2)])
    }

    /// 字 → 全码首末（镇 qfhw→qw; 虎码首末风格）
    pub fn wubi_head_tail(&self, ch: char) -> Option<&str> {
        self.head_tail.get(&ch).map(|s| s.as_str())
    }

    /// 字 → 笔顺 hspdz 串（你→pspzspd）; 缺笔顺字返回 None
    pub fn strokes(&self, ch: char) -> Option<&str> {
        self.strokes.get(&ch).map(|s| s.as_str())
    }

    /// 词 → 逐字前 2 码（一万→[gg,dn]; 缺码字整体 None 不猜）
    pub fn word_codes(&self, word: &str) -> Option<Vec<String>> {
        word.chars()
            .map(|ch| self.wubi2(ch).map(|s| s.to_string()))
            .collect()
    }

    /// 词 → 原生词码（yaml 词级行直查; 诊断/对照用）
    pub fn word_native_code(&self, word: &str) -> Option<&str> {
        self.words.get(word).map(|s| s.as_str())
    }

    /// 字级码表规模（装载诊断/锚点测试）
    pub fn n_chars(&self) -> usize {
        self.wubi.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 免数据合成 fixture: 临时目录写小表（ckvd 惯例的数据层测试法）
    fn synth(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let wubi = dir.join("mini.dict.yaml");
        std::fs::write(
            &wubi,
            "# 合成小表（测试 fixture）\n---\nname: mini\n...\n\n\
             你\twq\n你\twqi\n你\twqiy\n镇\tqfhw\n益\tuwl\n益\tuwlf\n\
             是\tj\n是\tjgh\n万\tdnv\n一\tg\n一\tggl\n一\tggll\n\
             一万\tggdn\n你好\twqvf\n坏行无tab\nX\tZZ\n",
        )
        .unwrap();
        let rsd = dir.join("mini_strokes.tsv");
        std::fs::write(
            &rsd,
            "# 合成笔顺表\nchar\tcp\tstrokes\n你\t0x4f60\t3235234\n镇\t0x9547\t311151225111134\n益\t0x76ca\t4313425221\n坏\t0x1\t9999\n",
        )
        .unwrap();
        (wubi, rsd)
    }

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir()
            .join(format!("hanhuo_auxform_{}_{}", tag, std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // ---- 装载解析 ----

    #[test]
    fn synth_load_parses_and_skips_headers() {
        let d = tmpdir("load");
        let (w, s) = synth(&d);
        let f = AuxForm::from_file(&w, &s).unwrap();
        // rime 头/注释/坏行跳过; 字级 7 个字, 词级 2 个词
        assert_eq!(f.n_chars(), 7);
        assert_eq!(f.word_native_code("一万"), Some("ggdn"));
        assert_eq!(f.word_native_code("你好"), Some("wqvf"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn multi_code_char_takes_longest() {
        // 多码字律: 最长码为源——你(wq/wqi/wqiy)→wubi2=wq;
        // 是(j/jgh)→jg（§4 验收 shi`jg→是 的码表前提）;
        // 一(g/ggl/ggll)→gg
        let d = tmpdir("longest");
        let (w, s) = synth(&d);
        let f = AuxForm::from_file(&w, &s).unwrap();
        assert_eq!(f.wubi2('你'), Some("wq"));
        assert_eq!(f.wubi2('是'), Some("jg"));
        assert_eq!(f.wubi2('一'), Some("gg"));
        assert_eq!(f.wubi_head_tail('你'), Some("wy"));
        assert_eq!(f.wubi_head_tail('是'), Some("jh"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn strokes_digit_to_letter_map() {
        // 笔顺映射律: 3235234→pspzspd（万象实证锚）; 非法数字位过滤
        let d = tmpdir("strokes");
        let (w, s) = synth(&d);
        let f = AuxForm::from_file(&w, &s).unwrap();
        assert_eq!(f.strokes('你'), Some("pspzspd"));
        assert_eq!(f.strokes('镇'), Some("phhhzhsszhhhhpd"));
        // "坏" 行笔顺 9999 全非法位 → 空串不收
        assert_eq!(f.strokes('坏'), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn word_codes_compose_per_char() {
        // 词级逐字前 2 码: 一万→[gg,dn]（=yaml 词码 ggdn 串接律）
        let d = tmpdir("word");
        let (w, s) = synth(&d);
        let f = AuxForm::from_file(&w, &s).unwrap();
        assert_eq!(f.word_codes("一万"), Some(vec!["gg".into(), "dn".into()]));
        // 缺码字整体 None（不猜律; X 在表内, 火不在）
        assert_eq!(f.word_codes("一火"), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn missing_char_returns_none() {
        let d = tmpdir("miss");
        let (w, s) = synth(&d);
        let f = AuxForm::from_file(&w, &s).unwrap();
        assert_eq!(f.wubi2('火'), None);
        assert_eq!(f.wubi_head_tail('火'), None);
        assert_eq!(f.strokes('火'), None);
        assert_eq!(f.word_codes("火"), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn missing_file_is_error() {
        let d = tmpdir("nofile");
        let e = AuxForm::from_file(
            &d.join("none.yaml"),
            &d.join("none.tsv"),
        );
        assert!(e.is_err(), "缺文件应报错而非静默空表");
        std::fs::remove_dir_all(&d).ok();
    }

    // ---- 真机锚点（本地腾讯资产, 勿分发; 照 ckvd 惯例 #[ignore]）----

    fn real_paths() -> (std::path::PathBuf, std::path::PathBuf) {
        let home = std::env::var("HOME").unwrap();
        let wubi = std::env::var("WUBI_YAML").map_or_else(
            |_| {
                std::path::PathBuf::from(
                    format!("{home}/tmp/hanhuo/wubi86_swbd.dict.yaml"),
                )
            },
            std::path::PathBuf::from,
        );
        let rsd = std::env::var("RSD_TSV").map_or_else(
            |_| {
                std::path::PathBuf::from(
                    format!("{home}/tmp/hanhuo/runtime_maps/rsd_char_strokes.tsv"),
                )
            },
            std::path::PathBuf::from,
        );
        (wubi, rsd)
    }

    #[test]
    #[ignore = "真机: 需 wubi86_swbd + rsd_char_strokes（本地资产）"]
    fn real_load_scale() {
        let (w, s) = real_paths();
        let f = AuxForm::from_file(&w, &s).unwrap();
        // §2 资产对照: 单字 14,253 种; 笔顺 29,513 字
        assert_eq!(f.n_chars(), 14_253);
        assert_eq!(f.strokes('我').is_some(), true);
    }

    #[test]
    #[ignore = "真机: 需 wubi86_swbd + rsd_char_strokes（本地资产）"]
    fn real_anchor_codes() {
        // §3.2 锚点五连: 呵→ks / 你→wq+pspzspd / 镇→qf+qw / 益→uw
        // + 是→jg（§4 验收前提）
        let (w, s) = real_paths();
        let f = AuxForm::from_file(&w, &s).unwrap();
        assert_eq!(f.wubi2('呵'), Some("ks"));
        assert_eq!(f.wubi2('你'), Some("wq"));
        assert_eq!(f.wubi2('镇'), Some("qf"));
        assert_eq!(f.wubi_head_tail('镇'), Some("qw"));
        assert_eq!(f.wubi2('益'), Some("uw"));
        assert_eq!(f.wubi2('是'), Some("jg"));
        assert_eq!(f.strokes('你'), Some("pspzspd"));
    }

    #[test]
    #[ignore = "真机: 需 wubi86_swbd + rsd_char_strokes（本地资产）"]
    fn real_word_codes_match_native() {
        // 组合律抽样: 逐字 wubi2 串接 ≡ 原生词码（双字词全量对照）
        let (w, s) = real_paths();
        let f = AuxForm::from_file(&w, &s).unwrap();
        let mut checked = 0;
        let mut same = 0;
        for (word, code) in f.words.iter() {
            if word.chars().count() != 2 {
                continue;
            }
            let Some(cs) = f.word_codes(word) else { continue };
            let joined: String = cs.concat();
            checked += 1;
            if &joined == code {
                same += 1;
            }
            if checked >= 5000 {
                break;
            }
        }
        assert!(checked >= 5000, "抽样不足 {checked}");
        // 实测一致性（简码词/二简词除外, 如 你好=wqvf≠wq+sk 全码形态）
        eprintln!("[auxform] 双字词组合律: {same}/{checked}");
        assert!(
            same * 100 >= checked * 95,
            "组合律一致率过低: {same}/{checked}"
        );
    }
}
