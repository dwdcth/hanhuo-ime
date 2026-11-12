//! CkvdPhrases —— ckvdmac.bin 长尾短语词典加载器（642 万短语）
//!
//! 2026-11-07 ckvd 战役终解（对 archive Python 参考有两处关键修正）：
//! 1. **桶哈希 = bhash(无分隔符全拼) % 624348**（CityHash32 家族双通道，
//!    全量 12,382/12,382 = 100.000% 桶一致实证；archive 旧代码用
//!    DefaultHasher 是错的）
//! 2. **块基址 = base + 8**（不是 base + 12 —— archive 旧解码器 off-by-4，
//!    导致每个块边界产生幻影条目；旧 ckvd_all.tsv 有边界污染）
//! 3. 存储键 = 逗号 join 音节（"ni,hao"），原生查询比对时跳过逗号
//!    （sub_1806F2760: entry.key strip(',') == query）
//! 4. T9 探针律：字母查询若与自身 T9 数字桶碰撞则 bucket+1（原生
//!    字母/数字双键空间消歧）
//! 5. 定性：**长尾补充短语库** —— 核心短词（你好/早上好/对不起）在
//!    wxq.bin 主词典，ckvd 存语料长尾扩展（≥3 音节为主）
//!
//! 原生调用链：sub_1806F2760(查询) → sub_180B560A0(桶哈希) →
//! sub_18078E800(条目解码) → sub_180809120(Huffman 位流)。
//! 数据归腾讯，仅本地个人使用，禁止分发。

use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

use crate::bhash::{bucket_of, to_t9};

/// 块内容相对文件头的基址（原生 v9 = base+8；archive 旧码误用 12）
const CHUNK_BASE: usize = 8;
const EXPECTED_KIND: u32 = 0x26; // kind 38
const EXPECTED_VERSION: u32 = 1;

/// 解码后的短语条目
#[derive(Debug, Clone, PartialEq)]
pub struct Phrase {
    /// 短语文本
    pub text: String,
    /// 拼音（撇号分隔的显示形式，如 "ni'hao"）
    pub pinyin: String,
    /// 权重（语料频次档，多为 1，偶见 2/3）
    pub weight: u32,
}

impl Phrase {
    /// 无分隔符全拼（= 桶哈希键）
    pub fn pinyin_flat(&self) -> String {
        self.pinyin.replace('\'', "")
    }
}

/// Huffman 树解码器（0x180809120 逐语义移植）
struct HuffmanDecoder {
    /// 节点对: nodes[i*2]=左子, nodes[i*2+1]=右子; 叶子判定 nodes[node*2]==0
    nodes: Vec<u32>,
    /// 符号字符串池
    string_pool: Vec<u8>,
    /// 每符号长度
    sym_lens: Vec<u8>,
    /// 每符号在池内偏移
    sym_offsets: Vec<u32>,
}

impl HuffmanDecoder {
    /// 从码本区（4 段结构，各带 u64 长度前缀 + 8 字节对齐）装载
    fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut pos = 0usize;

        let read_section = |pos: &mut usize| -> Result<&[u8]> {
            let len = u64::from_le_bytes(
                data[*pos..*pos + 8].try_into().context("码本长度前缀越界")?,
            ) as usize;
            *pos += 8;
            let start = *pos;
            *pos += len;
            let section = data
                .get(start..*pos)
                .with_context(|| format!("码本段越界: start={start} len={len}"))?;
            // 8 字节对齐
            *pos = (*pos + 7) & !7;
            Ok(section)
        };

        let nodes_data = read_section(&mut pos)?;
        let string_pool = read_section(&mut pos)?.to_vec();
        let sym_lens = read_section(&mut pos)?.to_vec();
        let offsets_data = read_section(&mut pos)?;

        let nodes: Vec<u32> = nodes_data
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let sym_offsets: Vec<u32> = offsets_data
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        eprintln!(
            "[ckvd] Huffman: {} 节点对, {} 符号, 池 {}B",
            nodes.len() / 2,
            sym_lens.len(),
            string_pool.len()
        );

        Ok(Self {
            nodes,
            string_pool,
            sym_lens,
            sym_offsets,
        })
    }

    /// 解码一个块到位串符号序列（0x180809120: 首字节高 3 位=补位数，
    /// 数据从首字节 bit4 起，逐位走树）
    fn decode_chunk(&self, chunk: &[u8]) -> Vec<Vec<u8>> {
        if chunk.is_empty() {
            return Vec::new();
        }
        let mut output = Vec::new();
        let mut node = 0usize;
        let total = chunk.len();
        let pad_bits = (chunk[0] >> 5) as usize;

        for (byte_idx, &b) in chunk.iter().enumerate() {
            let start_bit = if byte_idx == 0 { 4 } else { 7 };
            let stop_bit = if byte_idx == total - 1 { pad_bits } else { 0 };
            // 从 start_bit 降到 stop_bit（含）
            for bit_pos in (stop_bit..=start_bit).rev() {
                let bit = (b >> bit_pos) & 1;
                node = if bit == 0 {
                    self.nodes[node * 2] as usize
                } else {
                    self.nodes[node * 2 + 1] as usize
                };
                // 叶子判定
                if node * 2 < self.nodes.len() && self.nodes[node * 2] == 0 {
                    let sym_idx = node;
                    if sym_idx > 0 && sym_idx <= self.sym_offsets.len() {
                        let offset = self.sym_offsets[sym_idx - 1] as usize;
                        let length = self.sym_lens[sym_idx] as usize;
                        if offset + length <= self.string_pool.len() {
                            output.push(self.string_pool[offset..offset + length].to_vec());
                        }
                    }
                    node = 0;
                }
            }
        }
        output
    }
}

/// 把符号流切成 (text, pinyin, weight) 条目
/// 符号形如 "字\x01拼音"；纯数字符号 = 权重并终结当前条目
fn parse_entries(symbols: &[Vec<u8>]) -> Vec<Phrase> {
    let mut phrases = Vec::new();
    let mut chars: Vec<String> = Vec::new();
    let mut pinyins: Vec<String> = Vec::new();

    let flush = |chars: &mut Vec<String>, pinyins: &mut Vec<String>, weight: u32, out: &mut Vec<Phrase>| {
        if !chars.is_empty() {
            out.push(Phrase {
                text: chars.join(""),
                pinyin: pinyins.join("'"),
                weight,
            });
            chars.clear();
            pinyins.clear();
        }
    };

    for sym in symbols {
        let Ok(s) = std::str::from_utf8(sym) else {
            continue;
        };
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            // 权重符号 → 终结条目
            let weight = s.parse().unwrap_or(1);
            flush(&mut chars, &mut pinyins, weight, &mut phrases);
        } else if let Some((ch, py)) = s.split_once('\x01') {
            chars.push(ch.to_string());
            pinyins.push(py.to_string());
        } else if !chars.is_empty() {
            // 无分隔符符号：并入词文本（拼音记空，罕见形态）
            chars.push(s.to_string());
            pinyins.push(String::new());
        }
    }
    // 块尾无权重收尾的残段（跨块残影，原生由基址+8 保证不会出现）
    flush(&mut chars, &mut pinyins, 1, &mut phrases);
    phrases
}

/// ckvdmac.bin 词典（mmap + 惰性桶解码）
pub struct CkvdPhrases {
    _mmap: Mmap,
    bucket_count: u32,
    /// 桶偏移表（相对 CHUNK_BASE；count+1 项）
    offsets: Vec<u32>,
    huffman: HuffmanDecoder,
}

impl CkvdPhrases {
    /// 从文件装载（校验尾部 kind=0x26/version=1 + 头部版本）
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("无法打开 {}", path.as_ref().display()))?;
        let mmap = unsafe { Mmap::map(&file)? };
        let len = mmap.len();
        if len < 0x20 {
            anyhow::bail!("文件过小: {len}B");
        }

        // 尾部 footer (倒数 0x20 处): kind + version
        let fp = len - 0x20;
        let kind = u32::from_le_bytes(mmap[fp..fp + 4].try_into()?);
        let version = u32::from_le_bytes(mmap[fp + 4..fp + 8].try_into()?);
        if kind != EXPECTED_KIND {
            anyhow::bail!("footer kind 不符: 期望 {EXPECTED_KIND}, 实得 {kind}");
        }
        if version != EXPECTED_VERSION {
            anyhow::bail!("footer version 不符: 期望 {EXPECTED_VERSION}, 实得 {version}");
        }

        // 头部: u64 版本 + u32 桶数；偏移表 @12
        let file_version = u64::from_le_bytes(mmap[0..8].try_into()?);
        let bucket_count = u32::from_le_bytes(mmap[8..12].try_into()?);
        let table_bytes = (bucket_count as usize + 1) * 4;
        if 12 + table_bytes > len {
            anyhow::bail!("偏移表越界");
        }
        let offsets: Vec<u32> = mmap[12..12 + table_bytes]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        // 码本区 = 表尾到首块起点（首块起点 = CHUNK_BASE + offsets[0]）
        let codebook_start = 12 + table_bytes;
        let codebook_end = CHUNK_BASE + offsets[0] as usize;
        if codebook_end <= codebook_start || codebook_end > len {
            anyhow::bail!(
                "码本区异常: [{codebook_start}, {codebook_end}), file={len}"
            );
        }
        let huffman = HuffmanDecoder::from_bytes(&mmap[codebook_start..codebook_end])?;

        eprintln!(
            "[ckvd] 装载完成: 版本 {file_version}, {} 桶, {} 符号",
            bucket_count,
            huffman.sym_lens.len()
        );

        Ok(Self {
            _mmap: mmap,
            bucket_count,
            offsets,
            huffman,
        })
    }

    /// 查询键的桶号（含 T9 探针律：字母键与自身 T9 桶碰撞时 +1）
    pub fn bucket_for(&self, pinyin: &str) -> u32 {
        let key = pinyin.replace(',', "");
        let n = self.bucket_count;
        let mut b = bucket_of(key.as_bytes(), n);
        // 原生 sub_1806F2760: 非数字起始的查询，若 T9 形态落同桶则 +1
        let first = key.as_bytes().first().copied().unwrap_or(b'a');
        let is_digit_query = (b'2'..=b'9').contains(&first);
        if !is_digit_query {
            let t9 = to_t9(&key);
            if bucket_of(t9.as_bytes(), n) == b {
                b = if b >= n - 1 { 0 } else { b + 1 };
            }
        }
        b
    }

    /// 解码指定桶的全部条目
    pub fn bucket_items(&self, bucket: u32) -> Vec<Phrase> {
        let b = bucket as usize;
        if b + 1 >= self.offsets.len() {
            return Vec::new();
        }
        let o0 = CHUNK_BASE + self.offsets[b] as usize;
        let o1 = CHUNK_BASE + self.offsets[b + 1] as usize;
        if o1 <= o0 {
            return Vec::new();
        }
        let symbols = self.huffman.decode_chunk(&self._mmap[o0..o1]);
        parse_entries(&symbols)
    }

    /// 按全拼查询（无分隔符或撇号形式皆可）
    /// 原生语义：块内条目键去逗号后与查询串精确相等
    pub fn lookup(&self, pinyin: &str) -> Vec<Phrase> {
        let target = pinyin.replace(['\'', ','], "");
        let b = self.bucket_for(&target);
        self.bucket_items(b)
            .into_iter()
            .filter(|p| p.pinyin_flat() == target)
            .collect()
    }

    /// 桶数
    pub fn bucket_count(&self) -> u32 {
        self.bucket_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bhash::bucket_of;

    fn ckvd_path() -> std::path::PathBuf {
        std::env::var("CKVD_BIN").map_or_else(
            |_| {
                let p = std::path::PathBuf::from(
                    "/home/banshan/tmp/hanhuo/extracted/3.0.1.1/BuildIn/ckvdmac.bin",
                );
                if p.exists() {
                    p
                } else {
                    std::path::PathBuf::from("/home/banshan/tmp/wetype-trace/engine/BuildIn/ckvdmac.bin")
                }
            },
            std::path::PathBuf::from,
        )
    }

    #[test]
    #[ignore = "需要 ckvdmac.bin"]
    fn test_load_structure() {
        let ckvd = CkvdPhrases::from_file(ckvd_path()).unwrap();
        assert_eq!(ckvd.bucket_count(), 624_348);
        assert_eq!(ckvd.offsets.len(), 624_349);
    }

    #[test]
    #[ignore = "需要 ckvdmac.bin"]
    fn test_lookup_roundtrip() {
        // 真实长尾短语往返：桶内条目的拼音必须哈希回本桶（100% 律）
        let ckvd = CkvdPhrases::from_file(ckvd_path()).unwrap();
        let n = ckvd.bucket_count();
        let mut checked = 0;
        let mut consistent = 0;
        for b in (0..n).step_by(503) {
            for p in ckvd.bucket_items(b) {
                let flat = p.pinyin_flat();
                if flat.is_empty() {
                    continue;
                }
                checked += 1;
                if bucket_of(flat.as_bytes(), n) == b {
                    consistent += 1;
                } else {
                    eprintln!(
                        "[mismatch] bucket={b} text={} py={flat} len={}",
                        p.text,
                        flat.len()
                    );
                }
            }
            if checked >= 2000 {
                break;
            }
        }
        assert!(checked >= 2000, "采样不足: {checked}");
        // 2026-11-07 实证 12,382/12,382 = 100.000%
        assert_eq!(consistent, checked, "桶一致率 {consistent}/{checked}");
    }

    #[test]
    #[ignore = "需要 ckvdmac.bin"]
    fn test_lookup_yuange_zaoshanghao() {
        // 战役锚点: 苑哥早上好 (yuan'ge'zao'shang'hao) 在 bucket 1682
        let ckvd = CkvdPhrases::from_file(ckvd_path()).unwrap();
        let rs = ckvd.lookup("yuangezaoshanghao");
        assert!(
            rs.iter().any(|p| p.text == "苑哥早上好"),
            "应含 苑哥早上好, 实得 {:?}",
            rs.iter().map(|p| p.text.as_str()).take(5).collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "需要 ckvdmac.bin"]
    fn test_core_words_absent() {
        // 定性锚: 核心短词不在 ckvd（长尾补充库）
        let ckvd = CkvdPhrases::from_file(ckvd_path()).unwrap();
        assert!(ckvd.lookup("nihao").is_empty());
        assert!(ckvd.lookup("duibuqi").is_empty());
    }

    #[test]
    fn test_parse_entries_weight() {
        let syms: Vec<Vec<u8>> = vec![
            b"\xe4\xbd\xa0\x01ni".to_vec(),
            b"\xe5\xa5\xbd\x01hao".to_vec(),
            b"2".to_vec(),
        ];
        let ps = parse_entries(&syms);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].text, "你好");
        assert_eq!(ps[0].pinyin, "ni'hao");
        assert_eq!(ps[0].weight, 2);
    }
}
