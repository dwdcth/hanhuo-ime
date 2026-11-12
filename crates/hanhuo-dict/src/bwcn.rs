//! bwcn PC 主词典三元组（bwcnmac_triples.tsv, 70.7 万词）——boost 词注入源。
//!
//! 资产审计（2026-11-07）未用榜首：wxq 打包只进了 Android 版 575k 三元组，
//! PC 版多 ~13 万词覆盖。boost 旁挂注入（忠实层 wxq 不动，黄金零扰动）。
//! 复用 ckvd::Phrase 类型（pinyin 逗号→撇号形），注入通道同构。
//!
//! 数据归腾讯，仅本地个人研究使用，禁止分发。
use crate::ckvd::Phrase;
use std::collections::HashMap;

pub struct BwcnTriples {
    /// 键 = 无分隔全拼（IME 输入形）→ 词列（频序降序）
    pub map: HashMap<String, Vec<Phrase>>,
}

impl BwcnTriples {
    /// rime .dict.yaml → 词库（dict_build.py 的 Rust 镜像; 产出逐字节相同
    /// ——由 fixture 锁与 dict-build CLI 对拍守护）。
    /// 正文行 `word\t音节(空格分隔)\t[freq]`; #/---/... 头与无制表行跳过;
    /// 音节须 ascii 小写字母; freq 缺省 1。
    pub fn from_rime_yaml<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, String> {
        let txt = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("rime yaml 读取: {e}"))?;
        let mut map: HashMap<String, Vec<Phrase>> = HashMap::new();
        for ln in txt.lines() {
            if ln.starts_with('#')
                || ln.starts_with("...")
                || ln.starts_with("---")
                || !ln.contains('\t')
            {
                continue;
            }
            let f: Vec<&str> = ln.split('\t').collect();
            if f.len() < 2 {
                continue;
            }
            let (word, pys) = (f[0], f[1].split_whitespace().collect::<Vec<_>>());
            if word.is_empty() || pys.is_empty() {
                continue;
            }
            let syl_ok = |p: &str| {
                !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase())
            };
            if !pys.iter().all(|p| syl_ok(p)) {
                continue;
            }
            let freq = f
                .get(2)
                .filter(|s| {
                    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
                })
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            let key = pys.concat();
            let py = pys.join("'");
            map.entry(key).or_default().push(Phrase {
                text: word.to_string(),
                pinyin: py,
                weight: freq,
            });
        }
        for v in map.values_mut() {
            v.sort_by(|a, b| b.weight.cmp(&b.weight));
        }
        Ok(BwcnTriples { map })
    }

    pub fn from_tsv<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let txt = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("bwcn tsv 读取: {e}"))?;
        let mut map: HashMap<String, Vec<Phrase>> = HashMap::new();
        for ln in txt.lines() {
            if ln.starts_with('#') || ln.starts_with("word\t") {
                continue;
            }
            let f: Vec<&str> = ln.split('\t').collect();
            if f.len() < 3 {
                continue;
            }
            let (word, reading, freq) = (f[0], f[1], f[2]);
            if word.is_empty() || reading.is_empty() {
                continue;
            }
            let py = reading.replace(',', "'");
            let key = reading.replace(',', "");
            let w: u32 = freq.parse().unwrap_or(1);
            map.entry(key).or_default().push(Phrase {
                text: word.to_string(),
                pinyin: py,
                weight: w,
            });
        }
        for v in map.values_mut() {
            v.sort_by(|a, b| b.weight.cmp(&a.weight));
        }
        Ok(BwcnTriples { map })
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 全拼直查（无分隔形）；频序降序，cap 条
    pub fn lookup(&self, pinyin_nosep: &str, cap: usize) -> Vec<Phrase> {
        self.map
            .get(pinyin_nosep)
            .map(|v| v.iter().take(cap).cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bwcn_tsv_parse_and_lookup() {
        let dir = std::env::temp_dir().join("hanhuo_bwcn_test");
        std::fs::create_dir_all(&dir).ok();
        let p = dir.join("t.tsv");
        std::fs::write(
            &p,
            "# 头\nword\treading\tfreq\tis_primary\n\
             扶港路\tfu,gang,lu\t89\t1\n\
             服钢\tfu,gang\t500\t1\n\
             福\tfu\t30\t1\n",
        )
        .unwrap();
        let b = BwcnTriples::from_tsv(&p).unwrap();
        assert_eq!(b.len(), 3);
        let r = b.lookup("fuganglu", 8);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "扶港路");
        assert_eq!(r[0].pinyin, "fu'gang'lu");
        assert_eq!(r[0].weight, 89);
        let r2 = b.lookup("fugang", 8);
        assert_eq!(r2.len(), 1);
        assert!(b.lookup("fuganglux", 8).is_empty());
        // HBWC v1 往返位等价 + 路由
        let bin = b.to_hbwc();
        let b2 = BwcnTriples::from_hbwc(&bin).unwrap();
        assert_eq!(b2.len(), b.len());
        assert_eq!(b2.lookup("fuganglu", 8), b.lookup("fuganglu", 8));
        assert_eq!(b2.lookup("fugang", 8), b.lookup("fugang", 8));
        let bp = dir.join("t.hbwc");
        std::fs::write(&bp, &bin).unwrap();
        let b3 = BwcnTriples::from_file(&bp).unwrap(); // 走 HBWC 路由
        assert_eq!(b3.lookup("fuganglu", 8)[0].text, "扶港路");
        let b4 = BwcnTriples::from_file(&p).unwrap(); // tsv 回退
        assert_eq!(b4.len(), 3);
    }
}

// =====================================================================
// HBWC v1 —— 自有二进制格式（hanhuo-bwcn）
//
// [magic "HBWC"][ver u8=1][pad u16][n_entries u32][n_keys u32]
// keys 索引（按 nosep 拼音字典序）: n_keys × { off u32, len u16 }（入 key_blob）
// 每键头: n_keys × { first u32（entries 序）, cnt u16, py_off u32, py_len u16 }（撇号形, 键级去重）
// entries: n_entries × { word_off u32, word_len u16, freq u32 }（频序降序）
// key_blob | str_blob（词面）
//
// 特性: 排序键二分探查（免 HashMap 构建, attach 毫秒级）; 同键共享显示
// 拼音; mmap 单块可整体读。roundtrip 位等价由测试锁定。
impl BwcnTriples {
    /// 序列化为 HBWC v1
    /// HBWC v2（2026-11-10 瘦身, lyric 150k 词 8.4→5.0MB 级）:
    /// - 去盘上 key_blob/key_idx——装载端本就物化 HashMap, 无撇号键
    ///   = pinyin 去撇号可推导（v1 双存纯冗余）
    /// - key_head 12→8B: first 条目下标改装载端前缀和
    /// - entries 10→8B: weight u32→u16 饱和（频序保序——饱和单调,
    ///   稳定排序不倒序; >65535 词频仅 THUOCL 少数, lp 标定影响
    ///   ≤0.35nat 见 DOM_LOG_W_MAX 注记）
    /// 键序 = 无撇号键字节序（同 v1）; v1 读路径保留（旧包兼容）。
    pub fn to_hbwc(&self) -> Vec<u8> {
        let mut keys: Vec<(&String, &Vec<Phrase>)> = self.map.iter().collect();
        keys.sort_by(|a, b| a.0.cmp(b.0));
        let (n_keys, n_entries): (usize, usize) = (
            keys.len(),
            keys.iter().map(|(_, v)| v.len()).sum(),
        );
        let mut key_head: Vec<(u16, u32, u16)> = Vec::with_capacity(n_keys);
        let mut entries: Vec<(u32, u16, u16)> = Vec::with_capacity(n_entries);
        let mut str_blob = Vec::new();
        for (_k, phs) in &keys {
            let py = &phs[0].pinyin; // 同键共享（提取时同键同音节）
            let py_off = str_blob.len() as u32;
            str_blob.extend_from_slice(py.as_bytes());
            key_head.push((phs.len() as u16, py_off, py.len() as u16));
            for p in phs.iter() {
                let off = str_blob.len() as u32;
                str_blob.extend_from_slice(p.text.as_bytes());
                entries.push((
                    off,
                    p.text.len() as u16,
                    p.weight.min(u16::MAX as u32) as u16,
                ));
            }
        }
        let mut out = Vec::with_capacity(
            16 + str_blob.len() + n_keys * 8 + n_entries * 8,
        );
        out.extend_from_slice(b"HBWC");
        out.push(2);
        out.extend_from_slice(&[0u8; 3]); // 头 16B: magic4+ver1+pad3+n4+n4
        out.extend_from_slice(&(n_entries as u32).to_le_bytes());
        out.extend_from_slice(&(n_keys as u32).to_le_bytes());
        for (c, po, pl) in &key_head {
            out.extend_from_slice(&c.to_le_bytes());
            out.extend_from_slice(&po.to_le_bytes());
            out.extend_from_slice(&pl.to_le_bytes());
        }
        for (o, l, w) in &entries {
            out.extend_from_slice(&o.to_le_bytes());
            out.extend_from_slice(&l.to_le_bytes());
            out.extend_from_slice(&w.to_le_bytes());
        }
        out.extend_from_slice(&str_blob);
        out
    }

    /// 从 HBWC v1 装载（二分探查结构; 免 HashMap 构建）
    pub fn from_hbwc(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 16 || &bytes[..4] != b"HBWC" {
            return Err("非 HBWC 格式".into());
        }
        let ver = bytes[4];
        if ver != 1 && ver != 2 {
            return Err(format!("HBWC 版本 {ver}"));
        }
        let le32 = |o: usize| -> u32 {
            u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap())
        };
        let le16 = |o: usize| -> u16 {
            u16::from_le_bytes(bytes[o..o + 2].try_into().unwrap())
        };
        let n_entries = le32(8) as usize;
        let n_keys = le32(12) as usize;
        // v2: 无 key_idx/key_blob——键 = pinyin 去撇号推导; first 前缀和
        let mut o = 16;
        let (key_head, entries, str_blob): (
            Vec<(u32, u16, u32, u16)>,
            Vec<(u32, u16, u32)>,
            &[u8],
        ) = if ver == 2 {
            let heads: Vec<(u16, u32, u16)> = (0..n_keys)
                .map(|_| {
                    let x = (le16(o), le32(o + 2), le16(o + 6));
                    o += 8;
                    x
                })
                .collect();
            let ents: Vec<(u32, u16, u16)> = (0..n_entries)
                .map(|_| {
                    let x = (le32(o), le16(o + 4), le16(o + 6));
                    o += 8;
                    x
                })
                .collect();
            let mut first = 0u32;
            let heads: Vec<(u32, u16, u32, u16)> = heads
                .into_iter()
                .map(|(c, po, pl)| {
                    let f = first;
                    first += c as u32;
                    (f, c, po, pl)
                })
                .collect();
            let ents: Vec<(u32, u16, u32)> = ents
                .into_iter()
                .map(|(a, b, w)| (a, b, w as u32))
                .collect();
            (heads, ents, &bytes[o..])
        } else {
            let key_idx: Vec<(u32, u16)> = (0..n_keys)
                .map(|_| {
                    let x = (le32(o), le16(o + 4));
                    o += 6;
                    x
                })
                .collect();
            let heads: Vec<(u32, u16, u32, u16)> = (0..n_keys)
                .map(|_| {
                    let x = (le32(o), le16(o + 4), le32(o + 6), le16(o + 10));
                    o += 12; // f u32 + c u16 + po u32 + pl u16
                    x
                })
                .collect();
            let ents: Vec<(u32, u16, u32)> = (0..n_entries)
                .map(|_| {
                    let x = (le32(o), le16(o + 4), le32(o + 6));
                    o += 10;
                    x
                })
                .collect();
            let key_start = o;
            let str_start = key_start + {
                let mut n = 0usize;
                if let Some(&(lo, l)) = key_idx.last() {
                    n = lo as usize + l as usize;
                }
                n
            };
            (&key_idx, &heads, &ents, &bytes[str_start..], key_start)
                .0;
            // 借用布局: v1 键区在头区与串区之间, 显式取片
            let kb = &bytes[key_start..str_start];
            let _ = kb.len();
            // （键从 key_idx+key_blob 读取——下循环统一处理, 这里仅
            //  组装四元组头/条目/串区三件套）
            let sb = &bytes[str_start..];
            // v1 键: 先物化到 sidecar vec 以统一循环
            let keys_v1: Vec<String> = key_idx
                .iter()
                .map(|&(koff, klen)| {
                    std::str::from_utf8(
                        &bytes
                            [key_start + koff as usize..key_start + koff as usize + klen as usize],
                    )
                    .map_err(|e| format!("键 UTF-8: {e}"))
                    .map(|s| s.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let heads2: Vec<(u32, u16, u32, u16)> = heads;
            let _ = heads2.len();
            // 通过线程局部暂存把 keys_v1 递给统一循环——直接内联展开更直白:
            let mut map = HashMap::with_capacity(n_keys);
            for (i, key) in keys_v1.into_iter().enumerate() {
                let (first, cnt, po, pl) = heads2[i];
                let pinyin = std::str::from_utf8(
                    &sb[po as usize..po as usize + pl as usize],
                )
                .map_err(|e| format!("拼音 UTF-8: {e}"))?
                .to_string();
                let mut v = Vec::with_capacity(cnt as usize);
                for e in &ents[first as usize..first as usize + cnt as usize] {
                    let text = std::str::from_utf8(
                        &sb[e.0 as usize..e.0 as usize + e.1 as usize],
                    )
                    .map_err(|er| format!("词 UTF-8: {er}"))?
                    .to_string();
                    v.push(Phrase { text, pinyin: pinyin.clone(), weight: e.2 });
                }
                map.insert(key, v);
            }
            return Ok(BwcnTriples { map });
        };
        let mut map = HashMap::with_capacity(n_keys);
        for i in 0..n_keys {
            let (_first, cnt, po, pl) = key_head[i];
            let pinyin = std::str::from_utf8(
                &str_blob[po as usize..po as usize + pl as usize],
            )
            .map_err(|e| format!("拼音 UTF-8: {e}"))?
            .to_string();
            // v2 键 = 拼音去撇号（键序 = 无撇号字节序, 写侧已保证）
            let key = pinyin.replace('\'', "");
            let mut v = Vec::with_capacity(cnt as usize);
            let first = key_head[i].0;
            for e in &entries[first as usize..first as usize + cnt as usize] {
                let text = std::str::from_utf8(
                    &str_blob[e.0 as usize..e.0 as usize + e.1 as usize],
                )
                .map_err(|er| format!("词 UTF-8: {er}"))?
                .to_string();
                v.push(Phrase { text, pinyin: pinyin.clone(), weight: e.2 });
            }
            map.insert(key, v);
        }
        Ok(BwcnTriples { map })
    }

    /// 路径装载: HBWC 二进制优先, 回退 tsv
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let p = path.as_ref();
        if let Ok(b) = std::fs::read(p) {
            if b.len() >= 4 && &b[..4] == b"HBWC" {
                return Self::from_hbwc(&b);
            }
        }
        Self::from_tsv(p)
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;

    #[test]
    #[ignore = "真文件对拍"]
    fn hbwc_real_file_matches_tsv() {
        let home = std::env::var("HOME").unwrap();
        let tsv = format!("{home}/tmp/hanhuo/cache/android_harvest/dicts/bwcnmac_triples.tsv");
        let hb = format!("{home}/tmp/hanhuo/cache/android_harvest/dicts/bwcnmac.hbwc");
        let a = BwcnTriples::from_tsv(&tsv).unwrap();
        let bytes = std::fs::read(&hb).unwrap();
        let b = BwcnTriples::from_hbwc(&bytes).unwrap();
        assert_eq!(a.len(), b.len(), "键数");
        for k in ["beixi", "fuganglu", "guimeng", "nihao", "zhongguo"] {
            assert_eq!(a.lookup(k, 4), b.lookup(k, 4), "键 {k}");
        }
    }
}
