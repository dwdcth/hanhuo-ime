//! wxq.bin 打包格式读取器（v1）——与 archive wxq/pack.py 逐字节对齐。
//!
//! 布局（全定宽小端、8 对齐、目录一次解析）:
//!   [0] magic 'WXQ1' | ver=1 | n_sections | reserved
//!   [16] 目录 n × {tag 4B, offset u32, length u32}
//!   sections: SYL/SYLW/RDAT/RIDX/WLSI/WORD/WHSH/HOT/JIAN/EPYN/BGRM/C4G/NNLM/META/W2R
//!
//! mmap 以 &'static [u8] 泄漏持有（IME 进程生命周期单次加载，零拷贝）。
//! 对齐不保证的数组走 U32Arr/U16Arr/I32Arr（对齐则零拷贝引用，否则一次性拷贝）。
//!
//! 数据归腾讯，仅本地个人使用，禁止分发。
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use bytemuck::cast_slice;
use memmap2::Mmap;

pub const MAGIC: u32 = 0x3151_5857; // 'WXQ1' LE
pub const CKPT: usize = 32; // front-code 检查点间隔（与 pack.py 一致）

pub fn fnv1a(data: &[u8]) -> u32 {
    let mut h = 0x811C_9DC5u32;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

// ----------------------------------------------------------------------
/// 对齐自适应定宽数组：对齐则骑 mmap 零拷贝，否则开拷一次
pub enum U32Arr {
    Ref(&'static [u32]),
    Own(Vec<u32>),
}

impl U32Arr {
    pub fn new(b: &'static [u8]) -> U32Arr {
        debug_assert_eq!(b.len() % 4, 0);
        if b.as_ptr() as usize % 4 == 0 {
            U32Arr::Ref(cast_slice(b))
        } else {
            U32Arr::Own(
                b.chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect(),
            )
        }
    }
    #[inline]
    pub fn get(&self, i: usize) -> u32 {
        match self {
            U32Arr::Ref(s) => s[i],
            U32Arr::Own(v) => v[i],
        }
    }
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            U32Arr::Ref(s) => s.len(),
            U32Arr::Own(v) => v.len(),
        }
    }
}

pub enum U16Arr {
    Ref(&'static [u16]),
    Own(Vec<u16>),
}

impl U16Arr {
    pub fn new(b: &'static [u8]) -> U16Arr {
        debug_assert_eq!(b.len() % 2, 0);
        if b.as_ptr() as usize % 2 == 0 {
            U16Arr::Ref(cast_slice(b))
        } else {
            U16Arr::Own(
                b.chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect(),
            )
        }
    }
    #[inline]
    pub fn get(&self, i: usize) -> u16 {
        match self {
            U16Arr::Ref(s) => s[i],
            U16Arr::Own(v) => v[i],
        }
    }
}

pub enum I32Arr {
    Ref(&'static [i32]),
    Own(Vec<i32>),
}

impl I32Arr {
    pub fn new(b: &'static [u8]) -> I32Arr {
        debug_assert_eq!(b.len() % 4, 0);
        if b.as_ptr() as usize % 4 == 0 {
            I32Arr::Ref(cast_slice(b))
        } else {
            I32Arr::Own(
                b.chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect(),
            )
        }
    }
    #[inline]
    pub fn get(&self, i: usize) -> i32 {
        match self {
            I32Arr::Ref(s) => s[i],
            I32Arr::Own(v) => v[i],
        }
    }
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            I32Arr::Ref(s) => s.len(),
            I32Arr::Own(v) => v.len(),
        }
    }
}

// ----------------------------------------------------------------------
/// v4 精确位宽数组（bit p = byte p>>3 的 bit p&7；单 8B 载荷移位掩码，
/// w ≤ 24 保证 shift+w ≤ 31 不跨第二字）
pub struct BitArr {
    b: &'static [u8],
    w: u32,
}

impl BitArr {
    pub fn new(b: &'static [u8], w: u32) -> BitArr {
        BitArr { b, w }
    }

    #[inline]
    pub fn get(&self, i: usize) -> u32 {
        let bit = i * self.w as usize;
        let byte = bit >> 3;
        let shift = (bit & 7) as u32;
        let v = u64::from_le_bytes(
            self.b[byte..byte + 8].try_into().unwrap(),
        );
        ((v >> shift) & ((1u64 << self.w) - 1)) as u32
    }
}

/// v1-v3 兼容：定宽 u32 数组 → 一次性位打包为 BitArr（旧 bin 加载时转换，
/// v4 直接零拷贝）。值域 = 实测最大值所需位宽。
pub fn pack_values_to_bitarr(bytes: &'static [u8], n: usize) -> BitArr {
    let mut vals = Vec::with_capacity(n);
    for i in 0..n {
        let b = &bytes[i * 4..i * 4 + 4];
        vals.push(u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    }
    let mx = vals.iter().copied().max().unwrap_or(1).max(1);
    let w = 32 - mx.leading_zeros();
    let w = w.max(1);
    let total_bits = n * w as usize;
    let mut buf = vec![0u8; (total_bits + 7) / 8 + 8].into_boxed_slice();
    for (i, &v) in vals.iter().enumerate() {
        let bit = i * w as usize;
        for k in 0..w as usize {
            if v >> k & 1 == 1 {
                let p = bit + k;
                buf[p >> 3] |= 1 << (p & 7);
            }
        }
    }
    BitArr::new(Box::leak(buf), w)
}

// ----------------------------------------------------------------------
/// front-coded 字符串区随机访问（检查点独立记录 + 有界缓存）
pub struct FcStrings {
    pub blob: &'static [u8],
    pub off: U32Arr,
    pub n: usize,
    cache: RefCell<HashMap<usize, String>>,
}

impl FcStrings {
    pub fn new(blob: &'static [u8], off: U32Arr) -> FcStrings {
        let n = off.len() - 1;
        FcStrings {
            blob,
            off,
            n,
            cache: RefCell::new(HashMap::new()),
        }
    }

    fn ckpt_word(&self, cp: usize) -> String {
        let p = self.off.get(cp) as usize;
        let ln = self.blob[p + 1] as usize;
        let bytes = &self.blob[p + 2..p + 2 + ln];
        String::from_utf8_lossy(bytes).into_owned()
    }

    pub fn get(&self, i: usize) -> String {
        if let Some(v) = self.cache.borrow().get(&i) {
            return v.clone();
        }
        let cp = i - i % CKPT;
        let mut w = self.ckpt_word(cp);
        for j in cp + 1..=i {
            let p = self.off.get(j) as usize;
            let m = self.blob[p] as usize; // 共享前缀长（字符数, 非字节）
            let ln = self.blob[p + 1] as usize;
            let suf = String::from_utf8_lossy(&self.blob[p + 2..p + 2 + ln])
                .into_owned();
            // Python w[:m] 按字符截断（CJK 词 m≠字节数）
            w = w.chars().take(m).collect();
            w.push_str(&suf);
        }
        let mut c = self.cache.borrow_mut();
        if c.len() > 65536 {
            c.clear();
        }
        c.insert(i, w.clone());
        w
    }
}

// ----------------------------------------------------------------------
/// 打包容器：header + 目录一次解析，raw 按标签切
pub struct Pack {
    pub bytes: &'static [u8],
    pub ver: u32, // 1 | 2（v2: WLSI 含 per-(读音,词) 频次伴列）
    pub dir: HashMap<u32, (u32, u32)>, // tag → (offset, length)
}

fn le32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

impl Pack {
    /// mmap 打开；映射以 &'static 持有（进程生命周期）
    pub fn open(path: &Path) -> Result<Pack, String> {
        let f = File::open(path)
            .map_err(|e| format!("open {}: {e}", path.display()))?;
        let mm = unsafe { Mmap::map(&f) }
            .map_err(|e| format!("mmap {}: {e}", path.display()))?;
        let mm: &'static Mmap = Box::leak(Box::new(mm));
        Self::from_bytes(&mm[..])
    }

    /// 从内存字节装载（wasm/JS 链路；无 fs）。字节以 &'static 持有
    /// ——由调用方 Box::leak（Engine 生命周期 = 页面生命周期，同 mmap 语义）
    pub fn from_bytes(blob: &'static [u8]) -> Result<Pack, String> {
        let bytes = blob;
        if bytes.len() < 16 {
            return Err("文件过短".into());
        }
        let magic = le32(bytes, 0);
        if magic != MAGIC {
            return Err(format!("bad magic {magic:#x}"));
        }
        let ver = le32(bytes, 4);
        if !(1..=4).contains(&ver) {
            return Err(format!("不支持的版本 {ver}"));
        }
        let n = le32(bytes, 8) as usize;
        let mut dir = HashMap::with_capacity(n);
        for i in 0..n {
            let p = 16 + i * 12;
            let tag = le32(bytes, p);
            dir.insert(tag, (le32(bytes, p + 4), le32(bytes, p + 8)));
        }
        Ok(Pack { bytes, ver, dir })
    }

    pub fn has(&self, tag: &[u8; 4]) -> bool {
        self.dir.contains_key(&u32::from_le_bytes(*tag))
    }

    pub fn raw(&self, tag: &[u8; 4]) -> Result<&'static [u8], String> {
        let (off, ln) = *self
            .dir
            .get(&u32::from_le_bytes(*tag))
            .ok_or_else(|| format!("缺 section {tag:?}"))?;
        Ok(&self.bytes[off as usize..(off + ln) as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_standard_vectors() {
        // FNV-1a 32-bit 官方测试向量
        assert_eq!(fnv1a(b""), 0x811C_9DC5);
        assert_eq!(fnv1a(b"a"), 0xE40C_292C);
        assert_eq!(fnv1a(b"foobar"), 0xBF9C_F968);
    }

    #[test]
    fn bitarr_manual_bitstream() {
        // 手工位流：w=3 写 [1,2,3,4,5,6,7]，get 逐位读回应相等
        let vals = [1u32, 2, 3, 4, 5, 6, 7];
        let w = 3usize;
        let mut bytes = vec![0u8; 16].into_boxed_slice();
        for (i, &v) in vals.iter().enumerate() {
            for bit in 0..w {
                if (v >> bit) & 1 == 1 {
                    let p = i * w + bit;
                    bytes[p >> 3] |= 1 << (p & 7);
                }
            }
        }
        let arr = BitArr::new(Box::leak(bytes), w as u32);
        for (i, &v) in vals.iter().enumerate() {
            assert_eq!(arr.get(i), v, "i={i}");
        }
    }

    #[test]
    fn pack_values_roundtrip() {
        // u32 LE 数组 → 位打包 → 逐值读回（含 0 与最大值定宽）
        let vals = [0u32, 1, 3, 7, 255, 256, 65535, 0x00FF_FFFF];
        let n = vals.len();
        let mut raw = Vec::new();
        for v in vals {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        let arr = pack_values_to_bitarr(Box::leak(raw.into_boxed_slice()), n);
        for (i, &v) in vals.iter().enumerate() {
            assert_eq!(arr.get(i), v, "i={i}");
        }
        // 位宽 = 最大值 0xFFFFFF → 24
        assert_eq!(arr.w, 24);
    }

    #[test]
    fn fcstrings_decode_cjk_prefix() {
        // 合成 front-code 区：ckpt0="我们好"（shared=0 全量）
        // + 条目1 shared=2（字符数截断）suffix="好呀" → "我们好呀"
        // + 条目2 shared=1 suffix="才" → "我才"
        let blob: Vec<u8> = {
            let mut b = Vec::new();
            // ckpt 0: [shared=0][len][bytes]
            let w0 = "我们好";
            b.push(0u8);
            b.push(w0.len() as u8);
            b.extend_from_slice(w0.as_bytes());
            // entry 1
            b.push(2u8);
            let suf1 = "好呀";
            b.push(suf1.len() as u8);
            b.extend_from_slice(suf1.as_bytes());
            // entry 2
            b.push(1u8);
            let suf2 = "才";
            b.push(suf2.len() as u8);
            b.extend_from_slice(suf2.as_bytes());
            b
        };
        let offs: Vec<u8> = [0u32, 11, 19]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let fc = FcStrings::new(
            Box::leak(blob.into_boxed_slice()),
            U32Arr::new(Box::leak(offs.into_boxed_slice())),
        );
        assert_eq!(fc.n, 2);
        assert_eq!(fc.get(0), "我们好");
        assert_eq!(fc.get(1), "我们好呀"); // chars().take(2)="我们" + "好呀"
        assert_eq!(fc.get(2), "我才"); // chars().take(1)="我" + "才"
    }

    #[test]
    fn u32arr_get() {
        let raw: Vec<u8> = [7u32, 0xDEAD_BEEF, 42]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let a = U32Arr::new(Box::leak(raw.into_boxed_slice()));
        assert_eq!(a.get(0), 7);
        assert_eq!(a.get(1), 0xDEAD_BEEF);
        assert_eq!(a.get(2), 42);
        assert_eq!(a.len(), 3);
    }
}
