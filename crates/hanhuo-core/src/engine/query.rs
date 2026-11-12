//! Engine 打开/词表访问/切分/主查询编排/用户词与公共探针.
use super::*;

// ---- 第十一波 N5a: retro 组合降权 ----
// native retro 律（agentN4 sijian/zidao 双锚对照归纳）: 翘舌恢复候选
// （si→shi 时间 / zi→zhi 知道, edit_penalty = −EDIT_RETRO）须落直读音节
// 词典列腹地 — sijian native = 四件|四建|四季安|四键|时间(#5)|四间…
// （时间 落 四键(−2.531) 与 四间(−3.457) 之间 → 总罚 ∈ (3.68, 4.53)）;
// zidao native = 字\u{1}到|知道(#2)|自刀…（知道留 top-10 → 总罚 ≤ 4.52）。
// EDIT_RETRO 3.285 + 本降权 = 总罚 4.005 双锚同落。（py query.py
// `_N5A_RETRO_DEMOTE` 同值同注, tick62）
pub(super) const N5A_RETRO_DEMOTE: f64 = 0.72;

impl Engine {
    pub fn open(path: &Path) -> Result<Engine, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        Self::open_bytes(&data)
    }

    /// 从内存字节装载（wasm/JS 链路；无 fs）。字节泄漏为 'static 供
    /// Pack/U32Arr 切片借用——与 mmap 版同生命周期语义（单次 init）
    pub fn open_bytes(dict: &[u8]) -> Result<Engine, String> {
        let blob: &'static [u8] =
            Box::leak(dict.to_vec().into_boxed_slice());
        let pack = Pack::from_bytes(blob)?;
        let raw = |tag: &[u8; 4]| pack.raw(tag);

        // SYL / SYLW
        let syllables: Vec<String> = raw(b"SYL ")?
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        let sylw = U32Arr::new(raw(b"SYLW")?);
        let mut weights = Vec::with_capacity(syllables.len());
        for i in 0..syllables.len() {
            weights.push(sylw.get(i));
        }
        let seg = SyllableSeg::new(syllables.clone(), &weights);
        let sylw = weights.clone();

        // RDAT（v4: 位打包 base/check/lo；v1-v3: 定宽数组）
        let d = raw(b"RDAT")?;
        let (n_slots, n_keys, mut o) = if pack.ver >= 4 {
            (le32(d, 0) as usize, le32(d, 4) as usize, 16usize)
        } else {
            (le32(d, 0) as usize, le32(d, 4) as usize, 8usize)
        };
        let (base, check, lo) = if pack.ver >= 4 {
            let w_base = d[8] as u32;
            let w_lo = d[10] as u32;
            let packlen =
                |w: u32| -> usize {
                    (((n_slots * w as usize + 7) / 8 + 7) & !7) + 8
                };
            let b0 = &d[o..o + packlen(w_base)];
            let b1 = &d[o + packlen(w_base)..o + 2 * packlen(w_base)];
            let b2 = &d[o + 2 * packlen(w_base)
                ..o + 2 * packlen(w_base) + packlen(w_lo)];
            o += 2 * packlen(w_base) + packlen(w_lo);
            (
                BitArr::new(b0, w_base),
                BitArr::new(b1, w_base),
                BitArr::new(b2, w_lo),
            )
        } else {
            // v1-v3: 定宽数组一次性转位打包（兼容路径）
            let base = pack_values_to_bitarr(&d[o..o + 4 * n_slots], n_slots);
            o += 4 * n_slots;
            let check =
                pack_values_to_bitarr(&d[o..o + 4 * n_slots], n_slots);
            o += 4 * n_slots;
            let lo = pack_values_to_bitarr(&d[o..o + 4 * n_slots], n_slots);
            o += 4 * n_slots;
            (base, check, lo)
        };
        let cnt16 = U16Arr::new(&d[o..o + 2 * n_slots]);
        o += 2 * n_slots;
        let n_ovf = le32(d, o) as usize;
        o += 4;
        let mut cnt_ovf = HashMap::with_capacity(n_ovf);
        for _ in 0..n_ovf {
            cnt_ovf.insert(le32(d, o), le32(d, o + 4));
            o += 8;
        }
        let term = &d[o..o + (n_slots + 7) / 8];
        let dat = Dat {
            base,
            check,
            lo,
            cnt16,
            term,
            n_slots,
            n_keys,
            cnt_ovf,
        };

        // RIDX
        let r = raw(b"RIDX")?;
        let ridx_n = le32(r, 0) as usize;
        let mut o = 4;
        let _nw = U16Arr::new(&r[o..o + 2 * ridx_n]);
        o += 2 * ridx_n;
        let ridx_off = U32Arr::new(&r[o..o + 4 * (ridx_n + 1)]);
        o += 4 * (ridx_n + 1);
        let soff = U32Arr::new(&r[o..o + 4 * (ridx_n + 1)]);
        o += 4 * (ridx_n + 1);
        let syl_blob = &r[o..];

        // WLSI / WORD / WHSH（v2+: [n u32][wid×n][fv u16×n]）
        let (wlist, wlsi_fv) = {
            let wl = raw(b"WLSI")?;
            if pack.ver >= 2 {
                let n = le32(wl, 0) as usize;
                (
                    U32Arr::new(&wl[4..4 + 4 * n]),
                    Some(U16Arr::new(&wl[4 + 4 * n..4 + 6 * n])),
                )
            } else {
                (U32Arr::new(wl), None)
            }
        };
        let w = raw(b"WORD")?;
        let n_words = le32(w, 0) as usize;
        let mut o = 4;
        let freq = U16Arr::new(&w[o..o + 2 * n_words]);
        o += 2 * n_words;
        let flags = &w[o..o + n_words];
        o += n_words;
        let woff = U32Arr::new(&w[o..o + 4 * (n_words + 1)]);
        o += 4 * (n_words + 1);
        let fc = FcStrings::new(&w[o..], woff);
        // WHSH（v3: u32 槽=id+1；v1/v2: u64 槽交织为 2×u32）
        let wh = raw(b"WHSH")?;
        let whash_n = le32(wh, 0) as usize;
        let (whash_slots, whash_v3) = if pack.ver >= 3 {
            (U32Arr::new(&wh[4..4 + 4 * whash_n]), true)
        } else {
            (U32Arr::new(&wh[4..4 + 8 * whash_n]), false)
        };

        // HOT
        let h = raw(b"HOT ")?;
        let hn = le32(h, 0) as usize;
        let hot_states = U32Arr::new(&h[4..4 + 4 * hn]);
        let hot_off = U32Arr::new(&h[4 + 4 * hn..4 + 4 * hn + 4 * (hn + 1)]);
        let hot_ids = U32Arr::new(&h[4 + 4 * hn + 4 * (hn + 1)..]);

        // JIAN
        let j = raw(b"JIAN")?;
        let jian_n = le32(j, 0) as usize;
        let mut o = 4;
        let koff = U32Arr::new(&j[o..o + 4 * (jian_n + 1)]);
        o += 4 * (jian_n + 1);
        let kend = o + koff.get(jian_n) as usize;
        let jian_keys = FcStrings::new(&j[o..kend], koff);
        let eoff = U32Arr::new(&j[kend..kend + 4 * (jian_n + 1)]);
        let jian_eblob = &j[kend + 4 * (jian_n + 1)..];
        let jian_eoff = eoff;

        // EPYN
        let ep = raw(b"EPYN")?;
        let epyn_n = le32(ep, 0) as usize;
        let mut o = 4;        let ekoff = U32Arr::new(&ep[o..o + 4 * (epyn_n + 1)]);
        o += 4 * (epyn_n + 1);
        let ekend = o + ekoff.get(epyn_n) as usize;
        let epyn_keys = FcStrings::new(&ep[o..ekend], ekoff);
        let epyn_vblob = &ep[ekend..];
        // 件 3: EPYN 解出表装载（与 py 同源同文件; 资源机制=bin 同款
        // 环境变量/默认路径惯例, 仓内放副本）
        let epyn_tsv = load_epyn_tsv();


        // BGRM（可选段: 词 id 耦合的 bigram 表, 默认在包内; 分体布局
        // 可剥出经 set_bgrm 注入——aarch64 精简核心部署弹性, 缺位=
        // 语境 lift 关闭不致命）
        let (bi_n, bi_w1, bi_w2, bi_val) = match raw(b"BGRM") {
            Ok(bg) => {
                let n = le32(bg, 0) as usize;
                (
                    n,
                    U32Arr::new(&bg[4..4 + 4 * n]),
                    U32Arr::new(&bg[4 + 4 * n..4 + 8 * n]),
                    U16Arr::new(&bg[4 + 8 * n..4 + 10 * n]),
                )
            }
            Err(_) => (0, U32Arr::new(&[]), U32Arr::new(&[]), U16Arr::new(&[])),
        };

        // C4G
        let c = raw(b"C4G ")?;
        let nw = le32(c, 0) as usize;
        let mut o = 4;
        let coff = U32Arr::new(&c[o..o + 4 * (nw + 1)]);
        o += 4 * (nw + 1);
        let cpend = o + coff.get(nw) as usize;
        let c4g_pool = FcStrings::new(&c[o..cpend], coff);
        let c4g_npair = le32(c, cpend) as usize;
        let c4g_w1 =
            U32Arr::new(&c[cpend + 4..cpend + 4 + 4 * c4g_npair]);
        let c4g_w2 = U32Arr::new(
            &c[cpend + 4 + 4 * c4g_npair..cpend + 4 + 8 * c4g_npair],
        );
        let c4g_val = U16Arr::new(
            &c[cpend + 4 + 8 * c4g_npair..cpend + 4 + 10 * c4g_npair],
        );

        // W2R
        let w2r = U32Arr::new(raw(b"W2R ")?);

        let meta_n_words = n_words;
        Ok(Engine {
            boost: None,
            t9: false,
            bwcn: std::cell::RefCell::new(Vec::new()),
            pack,
            meta_n_words,
            dat,
            ridx_n,
            ridx_off,
            soff,
            syl_blob,
            wlist,
            wlsi_fv,
            n_words,
            freq,
            _flags: flags,
            fc,
            whash_slots,
            whash_n,
            whash_v3,
            hot_states,
            hot_off,
            hot_ids,
            syllables,
            seg,
            fb_ini_stack: RefCell::new(None),
            fb_syl_head: RefCell::new(None),
            t9_sig_v: RefCell::new(None),
            ini_top_syls_c: RefCell::new(None),
            sylset_m_v: RefCell::new(None),
            slot_col_v: RefCell::new(None),
            slot_wc_v: RefCell::new(None),
            engdict_v: RefCell::new(None),
            emix_ana_v: RefCell::new(None),
            emix_pin_rd: RefCell::new(None),
            ckvd: RefCell::new(None),
            stt: RefCell::new(None),
            tone_tab: RefCell::new(None),
            bwcn_texts: RefCell::new(None),
            sylw,
            bi_n,
            bi_w1,
            bi_w2,
            bi_val,
            w2r,
            jian_n,
            jian_keys,
            jian_eoff,
            jian_eblob,
            epyn_n,
            epyn_keys,
            epyn_vblob,
            epyn_tsv,
            c4g_pool,
            c4g_npair,
            c4g_w1,
            c4g_w2,
            c4g_val,
            wtext: RefCell::new(HashMap::new()),
            rid_memo: RefCell::new(HashMap::new()),
            epyn_parsed: RefCell::new(None),
            syl_by_initial: RefCell::new(None),
            syls_by_ini: RefCell::new(None),
            user: RefCell::new(None),
            poly: RefCell::new(None),
            nnlm: RefCell::new(None),
            #[cfg(feature = "safetensors")]
            nnlm_adv: RefCell::new(None),
            now_override: std::cell::Cell::new(None),
            mdltr: RefCell::new(None),
            wfea: RefCell::new(None),
        })
    }

    // ------------------------------------------------------------------
    // 基础查表
    // ------------------------------------------------------------------
    pub fn word_text(&self, wid: u32) -> String {
        if let Some(t) = self.wtext.borrow().get(&wid) {
            return t.clone();
        }
        let t = self.fc.get(wid as usize);
        let mut c = self.wtext.borrow_mut();
        if c.len() > 100000 {
            c.clear();
        }
        c.insert(wid, t.clone());
        t
    }

    #[inline]
    pub fn word_logp(&self, wid: u32) -> f64 {
        val_to_log10p(self.freq.get(wid as usize))
    }

    /// WLSI 槽位频次（per-(读音,词)）；v1 → None（回退词级）
    #[inline]
    pub(super) fn slot_fv(&self, oi: usize) -> Option<u16> {
        self.wlsi_fv.as_ref().map(|a| a.get(oi))
    }

    #[inline]
    pub(super) fn slot_logp(&self, w: u32, oi: usize) -> f64 {
        match self.slot_fv(oi) {
            Some(fv) => val_to_log10p(fv),
            None => self.word_logp(w),
        }
    }

    #[inline]
    pub(super) fn fv_of(&self, w: u32, oi: usize) -> u32 {
        match self.slot_fv(oi) {
            Some(fv) => fv as u32,
            None => self.freq_of(w),
        }
    }

    /// 音节表只读导出（CLI 双拼往返测试/工具用）
    pub fn syllables_public(&self) -> &[String] {
        &self.syllables
    }

    pub fn word_id(&self, word: &str) -> Option<u32> {
        let h = fnv1a(word.as_bytes());
        let mask = (self.whash_n - 1) as u32;
        let mut j = (h & mask) as usize;
        if self.whash_v3 {
            // v3: 槽=id+1，fnv 定链 + 词面校验（空槽止链）
            loop {
                let s = self.whash_slots.get(j);
                if s == 0 {
                    return None;
                }
                let wid = s - 1;
                if self.word_text(wid) == word {
                    return Some(wid);
                }
                j = ((j as u32 + 1) & mask) as usize;
            }
        }
        loop {
            let lo = self.whash_slots.get(2 * j);
            if lo == 0 {
                return None;
            }
            let hi = self.whash_slots.get(2 * j + 1);
            if hi == h {
                let wid = lo - 1;
                if self.word_text(wid) == word {
                    return Some(wid);
                }
            }
            j = ((j as u32 + 1) & mask) as usize;
        }
    }

    /// 读音的音节 id 变体表（RIDX syl blob 随机访问 + 缓存）
    pub(super) fn rid_variants(&self, rid: u32) -> Vec<Vec<u32>> {
        if let Some(v) = self.rid_memo.borrow().get(&rid) {
            return v.clone();
        }
        let b = self.syl_blob;
        let mut p = self.soff.get(rid as usize) as usize;
        let nv = b[p] as usize;
        p += 1;
        let mut variants = Vec::with_capacity(nv);
        for _ in 0..nv {
            let ns = b[p] as usize;
            p += 1;
            let mut ids = Vec::with_capacity(ns);
            for _ in 0..ns {
                ids.push(b[p] as u32 | ((b[p + 1] as u32) << 8));
                p += 2;
            }
            variants.push(ids);
        }
        let mut m = self.rid_memo.borrow_mut();
        if m.len() > 100000 {
            m.clear();
        }
        m.insert(rid, variants.clone());
        variants
    }

    /// rid 变体0 音节字母数和（= rid_pretty 去撇号长度; 免分配——
    /// kbest spans 深度判等热路径, 16k 面 × ≤60 行调用）
    pub(super) fn rid_letters_len(&self, rid: u32) -> usize {
        let b = self.syl_blob;
        let mut p = self.soff.get(rid as usize) as usize;
        let nv = b[p] as usize;
        if nv == 0 {
            return 0;
        }
        p += 1;
        let ns = b[p] as usize;
        p += 1;
        let mut sum = 0usize;
        for k in 0..ns {
            let id = (b[p + 2 * k] as u32) | ((b[p + 2 * k + 1] as u32) << 8);
            sum += self.syllables[id as usize].len();
        }
        sum
    }

    pub(super) fn rid_pretty(&self, rid: u32) -> String {
        let vs = self.rid_variants(rid);
        if vs.is_empty() {
            return String::new();
        }
        vs[0]
            .iter()
            .map(|&i| self.syllables[i as usize].clone())
            .collect::<Vec<_>>()
            .join("'")
    }

    /// 役N tick53: 读音变体 0 的音节串列表（py rid_info.get(rid)[0]）
    pub(super) fn rid_syls(&self, rid: u32) -> Vec<String> {
        let vs = self.rid_variants(rid);
        if vs.is_empty() {
            return Vec::new();
        }
        vs[0]
            .iter()
            .map(|&i| self.syllables[i as usize].clone())
            .collect()
    }

    #[inline]
    pub(super) fn reading_slice(&self, rid: u32) -> (usize, usize) {
        (
            self.ridx_off.get(rid as usize) as usize,
            self.ridx_off.get(rid as usize + 1) as usize,
        )
    }

    #[inline]
    pub(super) fn wlist_at(&self, i: usize) -> u32 {
        self.wlist.get(i)
    }

    #[inline]
    pub(super) fn freq_of(&self, wid: u32) -> u32 {
        self.freq.get(wid as usize) as u32
    }

    // ------------------------------------------------------------------
    // L2: 读音区间取词
    // ------------------------------------------------------------------
    /// 区间内按频合并（保插入序；每读音列已按频降序）
    pub(super) fn range_candidates(
        &self,
        lo: u32,
        cnt: u32,
        cap: usize,
    ) -> Vec<(u32, u32, u32)> {
        let mut order: Vec<u32> = Vec::new();
        let mut map: HashMap<u32, usize> = HashMap::new();
        let mut vals: Vec<(u32, u32)> = Vec::new(); // (fv, r)
        let put = |w: u32, fv: u32, r: u32,
                       order: &mut Vec<u32>,
                       map: &mut HashMap<u32, usize>,
                       vals: &mut Vec<(u32, u32)>| {
            if let Some(&ix) = map.get(&w) {
                if fv < vals[ix].0 {
                    vals[ix] = (fv, r);
                }
            } else {
                map.insert(w, order.len());
                order.push(w);
                vals.push((fv, r));
            }
        };
        if cnt <= SMALL_RANGE {
            for r in lo..lo + cnt {
                let (o0, o1) = self.reading_slice(r);
                for oi in o0..o1.min(o0 + PER_READING) {
                    let w = self.wlist_at(oi);
                    let fv = self.fv_of(w, oi);
                    put(w, fv, r, &mut order, &mut map, &mut vals);
                }
            }
        } else {
            let mut heap: BinaryHeap<Reverse<(u32, u32, u32, u32)>> =
                BinaryHeap::new();
            for r in lo..(lo + cnt).min(lo + MAX_RANGE_HEAP) {
                let (o0, o1) = self.reading_slice(r);
                if o1 > o0 {
                    let w0 = self.wlist_at(o0);
                    heap.push(Reverse((
                        self.fv_of(w0, o0),
                        r,
                        o0 as u32,
                        o1 as u32,
                    )));
                }
            }
            let want = cap.min(HOT_FALLBACK_K + 16);
            let mut got = 0usize;
            while let Some(Reverse((fv, r, oi, o1))) = heap.pop() {
                if got >= want {
                    break;
                }
                let w = self.wlist_at(oi as usize);
                if !map.contains_key(&w) {
                    map.insert(w, order.len());
                    order.push(w);
                    vals.push((fv, r));
                    got += 1;
                }
                if oi + 1 < o1 {
                    let w1 = self.wlist_at(oi as usize + 1);
                    heap.push(Reverse((
                        self.fv_of(w1, oi as usize + 1),
                        r,
                        oi + 1,
                        o1,
                    )));
                }
            }
        }
        order
            .into_iter()
            .zip(vals.into_iter())
            .map(|(w, (fv, r))| (w, r, fv))
            .collect()
    }

    pub(super) fn hot_candidates(&self, state: i32) -> Option<Vec<(u32, i32, u32)>> {
        // bisect_left over hot_states
        let n = self.hot_states.len();
        let st = state as u32;
        let (mut a, mut b) = (0usize, n);
        while a < b {
            let mid = (a + b) / 2;
            if self.hot_states.get(mid) < st {
                a = mid + 1;
            } else {
                b = mid;
            }
        }
        if a < n && self.hot_states.get(a) == st {
            let o0 = self.hot_off.get(a) as usize;
            let o1 = self.hot_off.get(a + 1) as usize;
            return Some(
                (o0..o1)
                    .map(|i| {
                        let w = self.hot_ids.get(i);
                        (w, -1, self.freq_of(w))
                    })
                    .collect(),
            );
        }
        None
    }

    // ------------------------------------------------------------------
    // L1 / DP
    // ------------------------------------------------------------------
    pub fn segment(&self, inp: &str) -> Option<(Vec<String>, f64)> {
        self.seg.viterbi(inp)
    }

    /// 多词 DP（词格 = DAT 终端; unigram = 词频层）。末段允许前缀补全。
    pub(super) fn dp_path(&self, inp: &str) -> Option<(Vec<(u32, u32)>, f64, bool)> {
        let b = inp.as_bytes();
        let n = b.len();
        let neg = f64::NEG_INFINITY;
        let mut best = vec![neg; n + 1];
        let mut back: Vec<Option<(usize, u32)>> = vec![None; n + 1];
        best[0] = 0.0;
        for i in 0..n {
            if best[i] == neg {
                continue;
            }
            for &(depth, st) in &self.dat.iter_terminal(&b[i..]) {
                let j = i + depth;
                let (lo, _cnt) = self.dat.range(st);
                let (o0, o1) = self.reading_slice(lo);
                if o1 <= o0 {
                    continue;
                }
                let w = self.wlist_at(o0);
                let sc = best[i] + self.slot_logp(w, o0);
                if sc > best[j] {
                    best[j] = sc;
                    back[j] = Some((i, lo));
                }
            }
        }
        // 末段前缀补全: best[i] + 完成词 − 覆盖罚
        let mut comp_best = neg;
        let mut comp_state: Option<u32> = None;
        let mut comp_i = 0usize;
        for i in 0..n {
            if best[i] == neg {
                continue;
            }
            let st = self.dat.walk(&b[i..], ROOT);
            if st < 0 {
                continue;
            }
            let (lo, cnt) = self.dat.range(st);
            if cnt == 0 {
                continue;
            }
            let (o0, o1) = self.reading_slice(lo);
            if o1 <= o0 {
                continue;
            }
            let w = self.wlist_at(o0);
            let sc = best[i] + self.slot_logp(w, o0) - COV_PENALTY;
            if sc > comp_best {
                comp_best = sc;
                comp_state = Some(lo);
                comp_i = i;
            }
        }
        if best[n] == neg && comp_state.is_none() {
            return None;
        }
        let mut words: Vec<(u32, u32)> = Vec::new();
        if best[n] >= comp_best {
            let mut j = n;
            while j > 0 {
                let (i, lo) = back[j].unwrap();
                let (o0, _) = self.reading_slice(lo);
                words.push((self.wlist_at(o0), lo));
                j = i;
            }
            words.reverse();
            Some((words, best[n], false))
        } else {
            let mut j = comp_i;
            while j > 0 {
                let (i, lo) = back[j].unwrap();
                let (o0, _) = self.reading_slice(lo);
                words.push((self.wlist_at(o0), lo));
                j = i;
            }
            words.reverse();
            let (o0, _) = self.reading_slice(comp_state.unwrap());
            words.push((self.wlist_at(o0), comp_state.unwrap()));
            Some((words, comp_best, true))
        }
    }

    /// 贪婪最长音节切分的边界位置集（字母下标, 含 0）
    pub(super) fn greedy_bounds(&self, inp: &str) -> HashSet<usize> {
        let n = inp.len();
        let mut bounds = HashSet::new();
        bounds.insert(0usize);
        let mut i = 0usize;
        while i < n {
            let mut hit = 0usize;
            let lmax = self.seg.max_len.min(n - i);
            for l in (1..=lmax).rev() {
                if self.seg.is_syl(&inp[i..i + l]) {
                    hit = l;
                    break;
                }
            }
            if hit == 0 {
                break;
            }
            i += hit;
            bounds.insert(i);
        }
        bounds
    }

    pub(super) fn extra_syl(&self, rid: u32, base_rid: u32) -> i32 {
        let a = self.rid_variants(rid);
        let b = self.rid_variants(base_rid);
        if a.is_empty() || b.is_empty() {
            return 1;
        }
        a[0].len() as i32 - b[0].len() as i32
    }

    // ------------------------------------------------------------------
    // 主查询
    // ------------------------------------------------------------------
    pub fn query(
        &self,
        inp: &str,
        ctx: Option<&str>,
        jianpin: bool,
        topk: usize,
        nn: bool,
        ltr: bool,
        dp: bool,
    ) -> Vec<Cand> {
        let mut out =
            self.query_inner(inp, ctx, jianpin, topk, nn, ltr, dp);
        super::awb::filter_out(&mut out);
        Self::ctx_colloc_pin(self, ctx, &mut out);
        out
    }

    /// B3-d ctx 搭配钉序（用户设计 2026-11-19「时间zhou→查词库
    /// 时间轴→轴排前」）: ctx 尾段 1..=4 字 × 候选词面拼串查词库
    /// （word_id O(1) 纯存在性）——命中 = 字典级强搭配, 钉序置顶
    /// （B2.7 律 top+0.5−0.01i, cap 3）。无 ctx 恒等零行为;
    /// 零模型依赖零推理开销。wrapper 位=罩住全部内路径早退。
    pub(super) fn ctx_colloc_pin(&self, ctx: Option<&str>, out: &mut Vec<Cand>) {
        // 忠实门同律: golden/对照跑 --faithful 无 boost → 恒等零行为
        if self.boost.is_none() {
            return;
        }
        let Some(ct) = ctx.filter(|c| !c.is_empty()) else {
            return;
        };
        // 纯单字列场景门: 面板头即 fbcol（整列单字续打）才开钉——
        // 词级面板（头=exact/pending 词）的 fbcol 尾巴不钉（v4 病理
        // 二段: 混合面板钉尾字劫持词级 top1, 61.9 仍 −8.3pp）
        if !out.first().is_some_and(|c| c.kind == crate::Kind::Fbcol) {
            return;
        }
        let cc: Vec<char> = ct.chars().collect();
        let top = out.first().map(|c| c.score).unwrap_or(0.0);
        let mut pinned = 0usize;
        for ci in 0..out.len().min(12) {
            // 场景限定: 只钉 fbcol 单字列（用户例=单字续打搭配）;
            // 词级面板（exact/pending）由 B3 RRF 负责——钉分会劫持
            // 词级精排（v4 实测 −13pp 的病理位）
            if out[ci].kind != crate::Kind::Fbcol {
                continue;
            }
            let w = out[ci].word.clone();
            let mut hit = false;
            for l in (1..=4usize).rev() {
                if cc.len() < l {
                    continue;
                }
                let tail: String = cc[cc.len() - l..].iter().collect();
                if self.word_id(&format!("{tail}{w}")).is_some() {
                    hit = true;
                    break;
                }
            }
            if hit {
                let target = top + 0.5 - 0.01 * pinned as f64;
                if out[ci].score < target {
                    out[ci].score = target;
                }
                pinned += 1;
                if pinned >= 3 {
                    break;
                }
            }
        }
        if pinned > 0 {
            out.sort_by(|a, b| {
                b.score.partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    fn query_inner(
        &self,
        inp: &str,
        ctx: Option<&str>,
        jianpin: bool,
        topk: usize,
        nn: bool,
        ltr: bool,
        dp: bool,
    ) -> Vec<Cand> {
        // Python: 空 ctx 为 falsy（联想门/assoc_mode/早退三处同语义）→ 归一为 None
        let ctx = ctx.filter(|c| !c.is_empty());
        // 件 4/件 7（役F1 agentR 件 3; 役K agentZ 三态细化, py 镜像）: 大写输入
        //  - 态1 全大写: 空（native agentL #23 + agentZ 复核 BUXUO/NIHAO/
        //    DIANHUA n=0）
        //  - 态2 内部大写（首字母小写）: 拼音前缀字列 + 字面尾混合
        //    （native niHAO n=96 = 你\x01HAO/你/呢/拟/…/ni/…; zhongGuo=
        //    中种钟重列; buXuo=不补部布列, agentZ wine agentZ_gap/data/）
        //  - 态3 首字母大写: 首音节 ≤2 键 → 字母词段通道（Nihao→ni\x01Hao,
        //    Buxuo→英文词段 n=1, Ni→ni）; 首音节 ≥3 键 → 空（native
        //    Zhongguo/Zhonggu/Hao n=0, agentZ wine 15 条族观测归纳）
        //  - 34 集与 golden 全小写, 不受影响
        let has_upper = inp.chars().any(|c| c.is_ascii_uppercase());
        if has_upper {
            let letters: Vec<char> =
                inp.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            if !letters.is_empty() && letters.iter().all(|c| c.is_uppercase())
            {
                return Vec::new();
            }
            return self.q_upper(inp);
        }
        // T9 九宫模式门前分叉（设计-T9九宫输入 §5-P1②: T9 域内 2-9
        // 是字母键——不过白名单（6-9 被挡）也不过 strip_tones（1-5
        // 调号征用冲突）。纯 2-9 键串才接管; 默认关 = 逐位零行为）
        if self.t9
            && !inp.is_empty()
            && inp.bytes().all(|k| (b'2'..=b'9').contains(&k))
        {
            let mut out = self.q_t9_typo(inp);
            super::awb::filter_out(&mut out);
            return out;
        }
        // 役N tick63（N5b spec①）: 数字阻断拼音管线——native niha0
        // ver=5 n=0 逐位（数字入串即整链空返回）
        // 役N tick68（p59 mix_rank_post 战役）: 标点阻断律——白名单外
        // 任何字符整链空。native 实证 nihao, / ni,hao / nihao. / ni-hao /
        // ni?hao 全部 0 候选; 白名单 = 字母 + 撇号（音节分隔）+ 空格
        // （软分隔）, 二者剥离后续查（面板=nihao）
        if inp
            .chars()
            .any(|c| !matches!(c, 'a'..='z' | 'A'..='Z' | '\'' | ' ' | '1'..='5'))
        {
            return Vec::new();
        }
        // 声调筛选（设计-声调筛选 §2 语法层）: 数字 1-5 = 前字母串
        // 末音节的调约束——剥离记位（净输入位→调号; 5=轻声）。数字
        // 输入原为整链空, 此扩展纯增量; 过滤在 boost 旋钮 tone_filter。
        let (inp, tone_pos) = super::strip_tones(&inp);
        let inp = inp;
        if inp.is_empty() && ctx.is_none() {
            return Vec::new();
        }
        let ctx_ids = self.ctx_ids(ctx);
        let big = match &ctx_ids {
            Some(ci) if ci.wid.is_some() => self.bigram_after(ci.wid.unwrap()),
            _ => Big {
                ord: Vec::new(),
                map: HashMap::new(),
            },
        };

        // 役N tick70（p60 联想役）: 裸声母字列律 — 单键面板 = g1_col_solo
        // 钉序纯单字列, 与 ctx 无关（py 镜像: native 实证 d/s/w/x 选词后
        // ≡ 无 ctx 逐位同; 词组不进面）。表外声母与多键串保持原管线
        // （g1_col 多键回退面板不受影响）。
        if !jianpin {
            // 声调筛选（设计-声调筛选）: 早退面板（钉表面板不经 boost
            // 尾段）在此过滤——单音节打调（shi2）是最高频用例
            let tf = |mut c: Vec<Cand>| -> Vec<Cand> {
                if let Some(b) = &self.boost {
                    if b.tone_filter && !tone_pos.is_empty() {
                        boost::tone_filter(self, &mut c, &inp, &tone_pos);
                    }
                }
                c
            };
            if inp.chars().count() <= 2 {
                if let Some(tbl) = g1_col_solo(&inp) {
                    return tf(self.q_initial_panel(tbl, topk));
                }
            }
            // 役N tick71: 音节字列律 — 409 音节钉表面板（与 ctx 无关;
            // 字列+补全词+简拼组合原生序混排, 原样钉）
            // 役N tick78: 音节门切深表源（96 深; 前 26 与浅表逐位同）
            if let Some(tbl) = syl_deep(&inp) {
                return tf(self.q_initial_panel(tbl, topk));
            }
            // 役N tick74: 双声母对面板钉序（ns/bj/... 377 对; 表空休眠）
            if let Some(tbl) = pair_panel(&inp) {
                return tf(self.q_initial_panel(tbl, topk));
            }
        }

        let mut cands: Vec<Cand> = Vec::new();
        // 打分基准键长（COMP_DOMINATE/W14 消费键数）：在途窗口态 = 窗口长度
        // （py: sinp = win；否则 = inp）
        let mut sinp = inp.clone();
        // 役N tick53: 面板路由旗（garbage/long panel 域撤 comp/phrase/
        // fuzzcomp 通道）
        let mut panel_route = false;
        if jianpin {
            cands = self.q_jianpin(&inp);
        } else if !inp.is_empty() {
            let st0p = self.dat.walk(inp.as_bytes(), ROOT);
            let covered = st0p >= 0 && self.dat.is_terminal(st0p);
            let bounds_p = self.greedy_bounds(&inp);
            // 役N tick53 (toan): 头无合法音节 × 元音在途 → 字列面板。
            // native toan = [天填甜田舔添(KBN 补全列)] + [t 栈合并字列],
            // 0 组合面（套爱你/掏爱你 类 fcomp 垃圾 native 全无 — G2 面
            // 抑制于本门）。cchuang 型 (头辅音重复) 不经此门 → 既有链。
            // 役N W2 (zeebjpw, 律 §8 扩展): 首音节 exact + 余段元音开头
            // 死尾（贪婪后残段 ≥3 键且无元音可组音节）→ 同律纯字列。
            // native zeebjpw = [则泽啧责择…] 纯 ze 列零组合（我方 贼不就怕
            // junk 面 native 无）。maag 不湿（残段 g 单键 = 悬空声母补全
            // 域）; gerxtq 不湿（余段 rxtq 辅音开头 = 简拼链域）。
            let mut garbage_hit = false;
            if !dp
                && (3..=11).contains(&inp.len())
                && !covered
                && inp.len() > 1
                && b"aeoiuv".contains(&inp.as_bytes()[1])
            {
                if bounds_p.len() == 1 && bounds_p.contains(&0) {
                    garbage_hit = true;
                } else {
                    let mx = bounds_p.iter().copied().max().unwrap_or(0);
                    if mx >= 2 {
                        let sb = bounds_p
                            .iter()
                            .copied()
                            .filter(|&b| b > 0)
                            .min()
                            .unwrap_or(0);
                        let stf =
                            self.dat.walk(inp[..sb].as_bytes(), ROOT);
                        if stf >= 0 && self.dat.is_terminal(stf) {
                            let r = &inp[sb..];
                            if !r.is_empty()
                                && b"aeoiuv".contains(&r.as_bytes()[0])
                            {
                                let rb = self.greedy_bounds(r);
                                let rb_max = rb.iter().copied().max().unwrap_or(0);
                                let rem = &r[rb_max..];
                                if !rem.is_empty()
                                    && rem.len() >= 3
                                    && !rem.bytes().any(|c| {
                                        b"aeoiuv".contains(&c)
                                    })
                                {
                                    garbage_hit = true;
                                }
                            }
                        }
                    }
                }
            }
            if garbage_hit {
                sinp = inp.clone();
                cands = self.q_garbage_cols(&inp);
                panel_route = true;
            } else if !dp && inp.len() > 8 && !covered {
                // 役N tick53: 长串统一面板（A6 垃圾字列 / C_dp 长串窗 /
                // ADV_long; native 逐位律 = 补全词+短语面+深度词+字列,
                // 构造序钉分）。覆盖态 None → 落回既有链
                // （jintiandetianqi == jintian 窗口面板, 锚不湿）。
                match self.q_long_panel(&inp) {
                    Some(pan) => {
                        sinp = inp.clone();
                        cands = pan;
                        panel_route = true;
                    }
                    None if st0p < 0 => {
                // 在途窗口（native 实证 jintiandetianqi == jintian 面板:
                // >8 键整串无读音节点时, 面板 = 贪婪完整音节前缀(≤8键)的
                // 候选, 无整句外推。窗口空则落回下方 DP 兜底链）
                let wlen = bounds_p
                    .iter()
                    .copied()
                    .filter(|&b| 0 < b && b <= 8)
                    .max()
                    .unwrap_or(0);
                if wlen >= 2 {
                    let win = inp[..wlen].to_string();
                    sinp = win.clone();
                    cands = self.q_full(&win, false);
                    // 役N tick49b（对齐攻坚 目标1, py 同源）: 窗口
                    // q_full 仅 DP 派生（sentence/path, 无词典面）= 窗口
                    // 无词典内容。native zheshaizhengshou(16 键,
                    // win=zheshai) 面板 = 首音节 zhe 字列（这/着/折/者/…）,
                    // 0 面——native G1 display 无单词 sentence 面（面=多词,
                    // 单词走字列）; DP join(这|色)= 这色 单词面是本引擎
                    // 独有发射, native 无。与 luntao 型 pending 首音节字列
                    // 同律（win 后缀 rem>3 键时 pending 块不发, 120 集内
                    // DP-only 窗口仅 zheshai/diuhuang 型）。word-phrase 绿锚
                    // （woaizhongguo 我爱\x01中国）不属 DP-only（woai 有
                    // pending 列）→ 不受影响; 置换后仍走下方短语块/
                    // fuzzcomp/G2 原链路。
                    if !cands.is_empty()
                        && cands.iter().all(|c| {
                            matches!(
                                c.kind,
                                Kind::Sentence | Kind::Path
                            )
                        })
                    {
                        let mut fb: Vec<usize> = self
                            .greedy_bounds(&win)
                            .into_iter()
                            .collect();
                        fb.sort_unstable();
                        let mut col: Vec<Cand> = Vec::new();
                        if fb.len() > 1 {
                            let fs = &win[..fb[1]];
                            let st = self
                                .dat
                                .walk(fs.as_bytes(), ROOT);
                            if st >= 0 && self.dat.is_terminal(st) {
                                let (lo, _cnt) = self.dat.range(st);
                                let (o0, o1) = self.reading_slice(lo);
                                for oi in o0..(o1.min(o0 + 12)) {
                                    let w = self.wlist_at(oi);
                                    col.push(self.mk(
                                        w,
                                        lo as i64,
                                        Kind::Pending,
                                        0,
                                        None,
                                        0.0,
                                        self.slot_fv(oi),
                                    ));
                                }
                            }
                        }
                        if !col.is_empty() {
                            cands = col;
                        }
                    }
                    // 长句双词短语（native woaizhongguo: 我爱\x01中国#1）:
                    // 窗口头词 + 余段（贪婪尾音节）首词, 头词低频
                    // （词条边界不确定）才发
                    let hw = self.first_word_of(&win);
                    let last_bound = bounds_p.iter().copied().max();
                    if let Some(hw) = hw {
                        if let Some(last) = last_bound {
                            if last > wlen {
                                let tail = inp[wlen..last].to_string();
                                let tw = self.first_word_of(&tail);
                                if let Some(tw) = tw {
                                    let hlp = self.word_logp(hw);
                                    // 案V6（役H1, py 同源）: 尾槽词同过词条
                                    // 边界不确定门。native zaozhongguo n=84
                                    // 全面板无 遭重/遭重|过（面板=早中国/在
                                    // 中国组合+zao/za 单字列）: 遭重|过 尾词
                                    // 过(−2.678) 高频 = 伪组合; woaizhongguo
                                    // 我爱|中国 尾词 中国(−3.549) 低频保留
                                    // （native_ref2 #0）→ 头门外加尾门。
                                    let tlp = self.word_logp(tw);
                                    // 役K(agentZ) G3: 尾段键长门——native
                                    // 在途窗口态 "窗口词+长尾" 落单字列
                                    // (tianqizenmeyang 无 天气\x01怎么样,
                                    // 尾段 zenmeyang 9 键); 尾 ≤8 键
                                    // (woaizhongguo 尾 zhong) 照发
                                    if tail.len() <= 8
                                        && hlp <= WINDOW_PHRASE_LP
                                        && tlp <= WINDOW_PHRASE_LP
                                    {
                                        let reading = self
                                            .segment(&inp)
                                            .map(|(p, _)| p.join("'"))
                                            .unwrap_or_default();
                                        let mut pc = Cand::new(
                                            format!(
                                                "{}\u{1}{}",
                                                self.word_text(hw),
                                                self.word_text(tw)
                                            ),
                                            hlp,
                                            Kind::Sentence,
                                        );
                                        pc.reading = reading;
                                        pc.score = hlp;
                                        pc.wid = tw as i64;
                                        cands.insert(0, pc);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    cands = Vec::new();
                }
                    }
                    None => {}
                }
            } else {
                // N5a: covered 全拼（>8 键 × ≥3 规整音节 × 非深列路由）
                // → 阶梯面板（native 八案逐位律, panel.q_ladder_panel;
                // 表外零交叉 — ref3 covered 域路由审计 0 命中）
                let covl = if dp {
                    None
                } else {
                    self.q_ladder_panel(&inp)
                };
                match covl {
                    Some(cvl) => {
                        sinp = inp.clone();
                        cands = cvl;
                        panel_route = true;
                    }
                    None => {
                        cands = self.q_full(&inp, dp);
                    }
                }
            }
            // 役N ref5 G_double (ee): 叠元音双键 × jianpin 键在板 → 简拼
            // 通道整面板接管（叠语气字面直通, e 单字列撤下）。native ee
            // n=97 = [嗯嗯(en,en), 额额(e,e), 呃呃(e,e), 嗯\u{1}额(en#e),
            // 饿饿, 鹅鹅, 谔谔, 峨峨, 恩恩…] — jianpin 表键 ee 直达（展开
            // en,en/e,e/er,er/ei,ei）, 面板无 e 单字裸列。词频归并序。
            // 门: len2 × 双同元音 × 键在板 — 基准全集仅 ee 触发（零外溢）。
            if !panel_route
                && !dp
                && inp.len() == 2
                && inp.as_bytes()[0] == inp.as_bytes()[1]
                && b"aeiou".contains(&inp.as_bytes()[0])
                && !self.jianpin_keys(&inp).is_empty()
            {
                let jp2 = self.q_jianpin(&inp);
                if !jp2.is_empty() {
                    cands = jp2;
                }
            }
            // N5a (M6): 1+1 字对面（zidao → 字\u{1}到 — native #1; 读音键
            // 校准表 + 成词饱和双门, 表外读音零发射）
            if !cands.is_empty() && !panel_route {
                if let Some(pfc) = self.n5a_pair_face(&inp) {
                    cands.insert(0, pfc);
                }
            }
            if cands.is_empty() {
                // 全拼无命中 → 简拼自动回退（原生行为: 非法全拼按首字母解释）
                cands = self.q_jianpin(&inp);
            }
            // 组合候选：无全切分（≥2 字母音节全覆盖）时触发
            // （buxuo 的 bu'xu'o 退化切分 → 组合仍发；liaotengqi 不发）
            let seg_ok = self
                .segment(&inp)
                .map(|sg| sg.0.iter().all(|s| s.len() >= 2))
                .unwrap_or(false);
            // 件 2/件 3: 规整切分输入存在可成词纠错边（Retroflex/EPYN）
            // 时照发 q_comp, 但只并纠错类候选（comp/短语类仍限退化输入,
            // 防规整查询面板被短语污染; 同 py 触发门）
            if !panel_route
                && (!seg_ok || self.correct_edge_exists(&inp))
            {
                let comps = self.q_comp(&inp, false);
                if seg_ok {
                    cands.extend(
                        comps.into_iter().filter(|c| c.edit_penalty < 0.0),
                    );
                } else {
                    cands.extend(comps);
                }
            }
            if cands.is_empty() {
                cands = self.q_correct(&inp); // 纠错表兜底（最后手段）
            }
            if cands.is_empty() && inp.len() > 8 {
                // 长输入渐进兜底：pending 窗口(≤8)外由句子 DP 接管
                cands = self.q_full(&inp, true);
            }
            // 役 E1: 多词短语通道（agentN §1.6 A/B/C + D 生成时约束;
            // 追加式接入, 不改既有回退链语义; py 同位镜像）
            if !panel_route {
                let ph = self.q_phrase(&inp);
                if !ph.is_empty() {
                    cands.extend(ph);
                }
            }
            // 役K(agentZ) G2/G4: 模糊组合通道（简拼×直读×KBN/元音替换边;
            // native 锚: juangedeo=间隔\x01的\x01哦 miunaozai=某宝\x01在
            // zhaong=找\x01那个 toan=天 dakn=大\x01可能 pgde=苹果\x01的）。
            // 门: 面板非空（空面板走 G1 兜底, 其内置同源组合器）+ 全串非
            // 词典读音覆盖 + ≤11 键（>11 = 长窗态 native 无组合, 34 集
            // 护栏）。
            // 役N tick48: G1 触发判据——≥12 键时 complete 外推不算面板
            // 填充者（≥15 键垃圾族 native=字列: 内/內/馁; 短查询的
            // completion-only 绿面板[−5 实证]不适用）。
            // tick49: 长度门精化为覆盖度判据——complete 读音字母数
            // ≥ len(inp)−3（外推悬空 ≤3 键）才算填充者（zhongguorenm
            // 12 键: 中国人民 读音 13 字母 ≥9 → completion 保留,
            // native #0 回收; neishuanzengzae 15 键: 内双 9 字母
            // <12 → 不算填充 → G1 字列接管; py 同源镜像）
            let pre_empty = if inp.len() < 12 {
                cands.is_empty()
            } else {
                !cands.iter().any(|c| {
                    c.kind != Kind::Complete
                        || c.reading.replace('\'', "").len() >= inp.len() - 3
                })
            };
            // tick67: 3 键门放行——仅 EMIX 域命中时（nhm 你还没 型构造
            // 面需要; mzd 毛泽东 型纯简拼不受影响——emix_full_key 交还
            // 门返回空 → 不放行常规 fuzzcomp 面）
            let min_len = if inp.len() == 3 {
                let probe = self.q_emix_panel(&inp);
                if probe.is_empty() { 4 } else { 3 }
            } else {
                4
            };
            if !panel_route && !pre_empty
                && (min_len..=11).contains(&inp.len())
            {
                let wst = self.dat.walk(inp.as_bytes(), ROOT);
                if !(wst >= 0 && self.dat.is_terminal(wst)) {
                    let mut fz = self.q_fuzzcomp(&inp, false);
                    // 役N tick53: 槽组合 junk 禁表（整串词面域;
                    // W1 起逐段检查在 q_slot_combos 出口）
                    fz.retain(|c| !slot_fuzz_ban(&c.word));
                    if !fz.is_empty() {
                        // 役N E1 (rieu): 存在精确首音节 _fs 时，抑制从 's' 替换结构
                        // 派生的 fbcol（如 rieu 的 di 列 的/滴/地、buxcuo 的 hu 列），
                        // 置换为精确首音节字列并后置垫底（native rieu=日入/日五/日度
                        // + 日/囸/驲/鈤；toan 等无精确首音节查询 _fs=None 不受影响）
                        if let Some(fs) = self.fb_first_syl(&inp) {
                            if fz.iter().any(|c| {
                                c.kind == Kind::Fbcol
                                    && !c.reading.replace('\'', "").starts_with(&fs)
                            }) {
                                fz.retain(|c| c.kind != Kind::Fbcol);
                                let mut fcol = Vec::new();
                                let mut seen = HashSet::new();
                                self.fb_seg_columns(&fs, &mut fcol, &mut seen, 4);
                                for fc in fcol.iter_mut() {
                                    fc.score_bias -= 8.0;
                                }
                                fz.extend(fcol);
                            }
                        }
                        cands.extend(fz);
                    }
                }
            }
            // 役K(agentZ) G1: 乱码/超长串兜底面板（仅空面板态触发）
            if pre_empty {
                cands = self.q_fallback(&inp);
                // 役N tick53 (ykvtbzuu): G1 尾串 KBN 纠错面顶替 —
                // bzuu→bzui(bi,zui)=闭嘴; native #0 = 叶绿体\x01闭嘴
                cands = self.q_g1_tailfix(&inp, cands);
            }
            // 役N ref5 G_double (llama): 纯拉丁英文词直通（native llama
            // 行 0 = llama 拉丁本体 #0, 读音 llama; 后接 拉拉\x01吗 面 +
            // l 声母栈）。律: 输入无任何拼音音节前缀（首字母串不成音节）×
            // 英文静态词典（bwcnenmac EnglishStaticDictV5 提取词表, 107,514
            // 词）命中 → 拉丁本体直通钉顶。词表缺失时律休眠（零外溢）。
            // 命中态整面板 G1 化（native = 拉丁本体 + G1 面 + 声母栈;
            // EPYN 纠错 俩吗 族 junk 撤下 — native 面板无）。
            if let Some(lat) = self.latin_direct(&inp) {
                let mut nc: Vec<Cand> = Vec::new();
                nc.push(lat);
                nc.extend(self.q_fallback(&inp));
                cands = self.q_g1_tailfix(&inp, nc);
            }
            // 役N tick52 (vlw): 3 键 v 头相邻换位（vl→lv / vn→nv）形态:
            // comp 伴换位音节字列（rieu 同律 comp+字列并存; native_ref3
            // vlw=滤网/率为 + lv 列 18 字, 列头紧随 comps）。−8.0 垫底 =
            // rieu 预案（全局分序架构下的通道序模拟）。仅在面板非空
            // （comp 分支命中）时伴发——真垃圾 v 头仍走 G1 V/v 态。
            if !cands.is_empty() && inp.len() == 3
                && inp.as_bytes()[0] == b'v'
                && (inp.as_bytes()[1] == b'l' || inp.as_bytes()[1] == b'n')
            {
                let sw: String = format!(
                    "{}{}",
                    inp.as_bytes()[1] as char, 'v'
                );
                if self.syllables.iter().any(|s| *s == sw) {
                    let mut fcol3: Vec<Cand> = Vec::new();
                    let mut seen3 = HashSet::new();
                    self.fb_seg_columns(&sw, &mut fcol3, &mut seen3, 12);
                    for fc in fcol3.iter_mut() {
                        fc.score_bias -= 8.0;
                    }
                    cands.extend(fcol3);
                }
            }
        }
        if ctx.is_some() && cands.is_empty() && !jianpin {
            let ci = ctx_ids.as_ref().unwrap();
            if !ci.text.is_empty() {
                cands = self.q_assoc(ci);
            }
        }

        // ---- 打分 ----
        let assoc_mode = ctx.is_some() && inp.is_empty();
        let full_cover = cands.iter().any(|c| {
            matches!(c.kind, Kind::Exact | Kind::Sentence)
                && c.extra_syl == 0
                && c.edit_penalty == 0.0
        });
        let has_comp = cands.iter().any(|c| c.kind == Kind::Comp);
        // 役N E1 (klei): 存在零编辑全消费 comp 整词（头前缀补全×尾音节精确消费）
        // 时，fcomp 的 lift 失效 + fbcol 后置（native klei=傀儡/可累/快累 整词面在前、
        // 字列垫底，无 开了|诶 类 fcomp 面；ykvtbzuu/gerxtq 等 G1 锚 comp 全空不受影响）
        let is_comp_full_word = |c: &Cand| -> bool {
            if c.kind != Kind::Comp || c.word.contains('\u{1}') || c.edit_penalty != 0.0 {
                return false;
            }
            let rs: Vec<&str> = if c.reading.is_empty() {
                vec![]
            } else {
                c.reading.split('\'').collect()
            };
            if rs.len() < 2 {
                return false;
            }
            let tail_r: String = rs[1..].concat();
            if tail_r.is_empty() {
                return false;
            }
            // (A) klei 形: 尾音节精确消费输入尾字母（头可前缀补全）
            if sinp.ends_with(&tail_r) && sinp.len() >= tail_r.len() {
                let head_in = &sinp[..sinp.len() - tail_r.len()];
                if rs[0].starts_with(head_in) {
                    return true;
                }
            }
            // (B) maag 形: 输入=头段+悬空声母, 头段塌缩消费 × 尾音节以
            // 该声母开头补全（native_ref3 maag: 马哥/骂过 面板顶、ma 字列
            // 垫底 → klei 同律压字列; 换位头变体 pen<0 天然不入此门）
            if rs.len() == 2 && sinp.len() >= 3 {
                let last = sinp.as_bytes()[sinp.len() - 1];
                if b"bpmfdtnlgkhjqxrzcsyw".contains(&last)
                    && rs[1].as_bytes()[0] == last
                {
                    let head_in = &sinp[..sinp.len() - 1];
                    let vc = vv_collapse(head_in);
                    if rs[0] == head_in || rs[0] == vc {
                        return true;
                    }
                }
            }
            false
        };
        let has_comp_full_word = cands.iter().any(is_comp_full_word);
        for c in cands.iter_mut() {
            let lp = if c.wid < 0 || c.lp_locked {
                c.logp
            } else {
                self.word_logp(c.wid as u32)
            };
            // 件 1（役F1）: 词条 logP override（数据线四案; \\u{1} 短语取头词,
            // 与既有「短语 lp 被头词 lp 重写」语义对齐; 含 lp_locked 槽路径）
            let lp = lp_override_for(&c.word, lp);
            c.logp = lp;
            if assoc_mode {
                c.score = c.lm.unwrap_or(-12.0) + W_FREQ_ASSOC * lp;
            } else {
                let mut lm_term = 0.0;
                if !big.is_empty() && c.wid >= 0 && big.contains(c.wid as u32)
                {
                    lm_term = big.get(c.wid as u32) - lp;
                }
                c.score = lp + lm_term - COV_PENALTY * c.extra_syl as f64
                    + c.edit_penalty
                    + c.score_bias
                    + c.user_bias;
                // N5a: retro 翘舌恢复组合降权（native 条件律 — sijian 时间
                // 落直读列腹地 四键/四间 之间, zidao 知道 留 top-10;
                // wenzuantuodun 稳赚 长串面板面不在此打分层, 锚不湿）
                if c.kind == Kind::Comp
                    && c.edit_penalty <= -EDIT_RETRO
                {
                    c.score -= N5A_RETRO_DEMOTE;
                }
                // 件 5: 组合短语压罚（agentM §2 native buxuo: 不许|哦 低于
                // 纠错 不错 ≥8.5 终值; 短语 lp 被头词 lp 重写（既有语义）→
                // 打分层施加）+ flags=0 用户词/组合词条压制（诚创/初创公司;
                // agentM §3 native 造词组合 −21.5 档且 诚创 缺席）
                if c.kind == Kind::Comp {
                    if c.word.contains('\u{1}') {
                        c.score -= COMP_PHRASE_DEMOTE;
                    }
                    if c.wid >= 0
                        && (c.wid as usize) < self.n_words
                        && self._flags[c.wid as usize] == 0
                    {
                        c.score -= COMP_UDICT_DEMOTE;
                    }
                }
                // 案U3 连带（役G2, py 同源）: exact 补召回使 unsure 短语与词形
                // 并存. native 分界（34 集锚）: niha(4 键) 短语 你|哈 在 #1;
                // wuxian(6)/jiefang(7) 的 物|先/接|方 native 面板均无 → 短语
                // 豁免完整态降权仅限 ≤4 键短输入。
                if full_cover
                    && c.kind == Kind::Pending
                    && !(c.word.contains('\u{1}') && sinp.len() <= 4)
                {
                    c.score -= PENDING_DEMOTE_COMPLETE + c.score_bias;
                }
                // 役N tick79d 全消费支配律: 全读 exact 词提权 +1.5
                // （raner 根因: 全读词然而被前缀字然压过; 四基准 +8 零回退;
                //   py WXQ_EXACT_FULL_BOOST 同值, rust 侧常量直用）
                if c.kind == Kind::Exact && sinp.len() >= 5
                    && c.reading.replace('\'', "").len() == sinp.len()
                {
                    c.score += 1.5;
                }
                if has_comp
                    && matches!(c.kind, Kind::Pending | Kind::Exact)
                    && c.extra_syl == 0
                    && c.reading.replace('\'', "").len() < sinp.len()
                {
                    c.score -= COMP_DOMINATE;
                }
                if has_comp_full_word {
                    if c.kind == Kind::Fcomp {
                        c.score -= c.score_bias;
                    } else if c.kind == Kind::Fbcol {
                        c.score -= COMP_DOMINATE;
                    } else if is_comp_full_word(c) {
                        let rl = c.reading.replace('\'', "").len();
                        if rl > sinp.len() {
                            c.score += 0.19 * (rl as f64 - sinp.len() as f64);
                        }
                    }
                }
                if full_cover && c.kind == Kind::Complete {
                    // native 面板实证（MDLR 电池）：exact 全消费时补全外推
                    // 整体不可见（xian 96 条无 想/现在）→ 硬抑制
                    c.score -= COMPLETE_SUPPRESS;
                }
                // path_rank LR feat"14" = 消费键数−音节数
                // （native md.bin 实证; 在途窗口态按窗口长度计）
                // 役K(agentZ): fbcol/fcomp/兜底 sentence 面（reading 空）
                // 免 W14——消费键数已由 lp+bias 定价, 叠加会把垃圾面顶到
                // 词面之上（assoc/c4g 空 reading 保持旧语义不动）
                if W14 != 0.0
                    && c.kind != Kind::Fbcol
                    && c.kind != Kind::Fcomp
                    && (c.kind != Kind::Sentence || !c.reading.is_empty())
                {
                    const Q: char = '\''; // 读音分隔符
                    let syls = c.reading.split(Q).count().max(1);
                    let rlen = c.reading.replace(Q, "").len();
                    let consumed = match c.kind {
                        Kind::Complete | Kind::Sentence => sinp.len(),
                        // 空 reading（联想等通道）按已全消费计（对齐 python None 语义）
                        _ if rlen == 0 => sinp.len(),
                        _ => rlen.min(sinp.len()),
                    };
                    c.score += W14 * (consumed as f64 - syls as f64);
                }
            }
        }
        // ---- mix_recall MLP 召回门（第十五役; 只作用 Kind::Correct recall
        // 通道, 依据=agentC z_recall 台账: 档案 掉/饭饭 进而 buxuo/cchuang/
        // buxcuo 纠错主族零位移。开关默认 on; 特征构造与 py 逐项同源）----
        if mix_recall_enabled() && !cands.is_empty() {
            let mut order: Vec<usize> = (0..cands.len()).collect();
            order.sort_by(|&a, &b| {
                cands[b].score.partial_cmp(&cands[a].score).unwrap()
            });
            let mut rank_of = vec![0usize; cands.len()];
            for (r, &i) in order.iter().enumerate() {
                rank_of[i] = r;
            }
            let corr: Vec<usize> =
                order.iter().copied().filter(|&i| cands[i].kind == Kind::Correct).collect();
            let best_typo = corr.iter().map(|&i| cands[i].score).fold(f64::NEG_INFINITY, f64::max);
            let best_typo = if corr.is_empty() { 0.0 } else { best_typo };
            let best_comp = cands
                .iter()
                .filter(|c| c.kind == Kind::Complete)
                .map(|c| c.score)
                .fold(0.0_f64, f64::max);
            let best_lm = cands
                .iter()
                .filter_map(|c| c.lm)
                .fold(0.0_f64, f64::max);
            let mut drop: HashSet<usize> = HashSet::new();
            for (tr, &i) in corr.iter().enumerate() {
                let c = &cands[i];
                let x = [
                    best_typo,                                  // best_cand_typo: 池内 correct 通道最高分
                    best_comp,                                  // best_cand_completion: complete 通道最高分
                    best_lm,                                    // best_cand_lm: 全池最大 bigram lm（无 ctx 全 0）
                    c.score,                                    // cand_typo: 本候选分（门作用类=Correct）
                    0.0,                                        // cand_completion: 本候选非 complete → 0
                    c.lm.unwrap_or(0.0),                        // cand_lm: 本候选 bigram
                    rank_of[i] as f64,                          // rank: 全池分数降序名次(0 基)
                    tr as f64,                                  // typo_rank: Correct 通道内名次(0 基)
                ];
                if let Ok(path) = std::env::var("MIXRECALL_DUMP") {
                    // 开源重建采集: inp 	 word 	 f0..f7 	 is_top1_face
                    // （gold 标签由离线对齐——此处记面板位形; 追加式）
                    use std::io::Write;
                    if let Ok(mut f) =
                        std::fs::OpenOptions::new()
                            .create(true).append(true)
                            .open(path)
                    {
                        let _ = writeln!(
                            f,
                            "{}\t{}\t{:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {} {}\t{}",
                            inp, c.word, x[0], x[1], x[2], x[3], x[4],
                            x[5], x[6] as i64, x[7] as i64,
                            if tr == 0 { "face" } else { "deep" });
                    }
                }
                if mix_recall_forward(&x) <= 0.0 {
                    drop.insert(i);
                }
            }
            if !drop.is_empty() {
                let mut kept: Vec<Cand> = Vec::with_capacity(cands.len());
                for (i, c) in cands.drain(..).enumerate() {
                    if !drop.contains(&i) {
                        kept.push(c);
                    }
                }
                cands = kept;
            }
        }
        cands.sort_by(|a, b| {
            let ra = if a.kind == Kind::Sentence { 0 } else { 1 };
            let rb = if b.kind == Kind::Sentence { 0 } else { 1 };
            ra.cmp(&rb).then(b.score.partial_cmp(&a.score).unwrap())
        });
        // 去重（首现保留）+ 截断
        // 件 5: 组合短语以词面为身份（\x01 形态）——短语 wid=头词 wid,
        // 按 wid 去重会让 不许|哦 被 bare 不许 吞掉（native buxuo:
        // 不许|哦 rank5 在面板, agentM §2）
        let mut seen: HashSet<u64> = HashSet::new();
        // 案U2（役G2, py 同源）: 组合短语同(头词,尾音节)择一——native buxcuo
        // 只有 不想|错/不行|错 各一条（wine n=82, 无 了-尾）。错/错了同分
        // （头词 lp 重写语义）纯 override 不可分 → 同键留生成序首位。
        // 仅件 5 形（reading 含 #）参与; A 通道全句组合不适用。
        let mut phrase_seen: HashSet<u64> = HashSet::new();
        let mut out: Vec<Cand> = Vec::new();
        for c in cands.into_iter() {
            let key = if c.wid >= 0 && !c.word.contains('\u{1}') {
                c.wid as u64
            } else {
                let mut h = DefaultHasher::new();
                b's'.hash(&mut h);
                c.word.hash(&mut h);
                h.finish()
            };
            if seen.insert(key) {
                if c.word.contains('\u{1}') && c.reading.contains('#') {
                    let head = c.word.split('\u{1}').next().unwrap_or("");
                    let tsyl = c.reading.split('#').nth(1).unwrap_or("");
                    let mut h = DefaultHasher::new();
                    b'p'.hash(&mut h);
                    head.hash(&mut h);
                    tsyl.hash(&mut h);
                    if phrase_seen.insert(h.finish()) {
                        out.push(c);
                    }
                } else {
                    out.push(c);
                }
            }
            // 案V5（役H1, py 同源）: 不再 early-break——配额在去重后统一
            // 裁剪, 与 py 先 dedupe 全量再配额再截断逐位同序
        }
        // ---- 案V5（役H1, py 同源）: comp 通道全局短语配额。native 短语头
        // 分布（役H1 wine 复核 = agentU 存档逐位同）: buxcuo n=82 恰 2 条
        // （不想|错/不行|错, 0xffff）; buxuo n=90 仅 1 条; bucuo n=86 零
        // \x01 短语 ——每面板 \x01 组合 ≤2 与 A 通道 COMBO_CAP 同律。
        // 终打分序裁剪至 COMBO_CAP（判据: buxcuo 短语头 ≤3 且 top2 保持）----
        {
            let mut ph: Vec<usize> = out
                .iter()
                .enumerate()
                .filter(|(_, c)| c.kind == Kind::Comp && c.word.contains('\u{1}'))
                .map(|(i, _)| i)
                .collect();
            if ph.len() > COMBO_CAP {
                ph.sort_by(|&a, &b| {
                    out[b].score.partial_cmp(&out[a].score).unwrap()
                });
                let kill: std::collections::HashSet<usize> =
                    ph[COMBO_CAP..].iter().copied().collect();
                let mut kept: Vec<Cand> = Vec::with_capacity(out.len());
                for (i, c) in out.drain(..).enumerate() {
                    if !kill.contains(&i) {
                        kept.push(c);
                    }
                }
                out = kept;
            }
        }
        if out.len() > topk {
            out.truncate(topk);
        }
        // ---- 役N tick77b: 中带列化 — 短词表读音（全读音词块 <10）的
        // [10:topk) 用首音节列延续替换（native nihao/qiche [10:20) = 列
        // 10/10 集合实证; 词表长读音/叠字读音门排除）。top-10 校准区不动。
        // ----
        if inp.len() >= 4 && out.len() >= 10 {
            let st2 = self.dat.walk(inp.as_bytes(), ROOT);
            if st2 >= 0 && self.dat.is_terminal(st2) {
                let (lo2, _) = self.dat.range(st2);
                let rd2 = self.rid_pretty(lo2 as u32);
                let cs2: Vec<&str> = rd2.split('\'').collect();
                if cs2.len() >= 2 && cs2[0] != cs2[cs2.len() - 1] {
                    let nw2b = out[..20.min(out.len())]
                        .iter()
                        .filter(|c| c.reading == rd2)
                        .count();
                    if nw2b < 10 {
                        let mut s02 = "";
                        for s in &self.syllables {
                            if s.len() > s02.len()
                                && inp.starts_with(s.as_str())
                            {
                                s02 = s;
                            }
                        }
                        if let Some(col2) = syl_deep(s02) {
                            let mut keep2: Vec<Cand> =
                                out[..10].to_vec();
                            let mut have2: HashSet<String> =
                                keep2.iter().map(|c| c.word.clone()).collect();
                            let base2 = keep2
                                .iter()
                                .map(|c| c.score)
                                .fold(f64::INFINITY, f64::min);
                            let base2 =
                                if base2.is_finite() { base2 } else { 0.0 };
                            for (i5, t5) in col2.iter().enumerate() {
                                if keep2.len() >= topk {
                                    break;
                                }
                                if have2.contains(*t5) {
                                    continue;
                                }
                                let w5 = self.panel_wid(t5);
                                let mut c5 = if w5 < 0 {
                                    let mut lc = self.mk_lit(t5, -1);
                                    lc.kind = Kind::Fbcol;
                                    lc
                                } else {
                                    let wid = w5 as u32;
                                    self.mk(wid,
                                            self.w2r.get(wid as usize) as i64
                                                - 1,
                                            Kind::Fbcol, 0, None, 0.0, None)
                                };
                                c5.score = base2 - 0.001 * (i5 as f64 + 1.0);
                                c5.score_bias = c5.score;
                                keep2.push(c5);
                                have2.insert(t5.to_string());
                            }
                            out = keep2;
                        }
                    }
                }
            }
        }
        // ---- 役N tick77: 深页定律填充 — topk > 候选池时以首音节全字列
        // 补尾（native nihao[6:96] ≡ ni 列 76/90 逐位; py 镜像。
        // 默认 topk=20 池恒足 → 零影响）----
        if out.len() < topk && !inp.is_empty() {
            let mut s0 = "";
            for s in &self.syllables {
                if s.len() > s0.len() && inp.starts_with(s.as_str()) {
                    s0 = s;
                }
            }
            if let Some(col) = syl_deep(s0) {
                let mut have: HashSet<String> =
                    out.iter().map(|c| c.word.clone()).collect();
                let base = out
                    .iter()
                    .map(|c| c.score)
                    .fold(f64::INFINITY, f64::min);
                let base = if base.is_finite() { base } else { 0.0 };
                for (i, t) in col.iter().enumerate() {
                    if out.len() >= topk {
                        break;
                    }
                    if have.contains(*t) {
                        continue;
                    }
                    let w = self.panel_wid(t);
                    let mut c = if w < 0 {
                        let mut lc = self.mk_lit(t, -1);
                        lc.kind = Kind::Fbcol;
                        lc
                    } else {
                        let wid = w as u32;
                        self.mk(wid, self.w2r.get(wid as usize) as i64 - 1,
                                Kind::Fbcol, 0, None, 0.0, None)
                    };
                    c.score = base - 0.001 * (i as f64 + 1.0);
                    c.score_bias = c.score;
                    out.push(c);
                    have.insert(t.to_string());
                }
            }
        }
        // ---- 役N tick79n: 深带 noun_tag 叉积生成器（py 镜像）----
        if !inp.is_empty() && inp.len() >= 4 {
            let st3 = self.dat.walk(inp.as_bytes(), ROOT);
            if st3 >= 0 && self.dat.is_terminal(st3) {
                let mut s1 = "";
                let mut s2 = "";
                for s in &self.syllables {
                    if s.len() > s1.len() && inp.starts_with(s.as_str()) {
                        s1 = s;
                    }
                }
                if !s1.is_empty() && s1.len() < inp.len() {
                    let rest = &inp[s1.len()..];
                    for s in &self.syllables {
                        if s.len() > s2.len() && rest.starts_with(s.as_str()) {
                            s2 = s;
                        }
                    }
                }
                if !s1.is_empty() && !s2.is_empty() {
                    let c1: Vec<&str> = syl_deep(s1)
                        .map(|v| v.iter().take(14).copied().collect())
                        .unwrap_or_default();
                    let c2: Vec<&str> = syl_deep(s2)
                        .map(|v| v.iter().take(14).copied().collect())
                        .unwrap_or_default();
                    let mut adds: Vec<String> = Vec::new();
                    let have3: HashSet<String> =
                        out.iter().map(|c| c.word.clone()).collect();
                    for a in &c1 {
                        if a.chars().count() != 1 { continue; }
                        for b in &c2 {
                            if b.chars().count() != 1 { continue; }
                            let w = format!("{}{}", a, b);
                            if have3.contains(&w) { continue; }
                            if super::ntgen::nt_has(&w) {
                                adds.push(w);
                            }
                        }
                    }
                    // tick80e: 2-词 kbest 路径面 top-1（进天型; py 同律）
                    {
                        let mut n2w = 0;
                        'outer: for pth in self.kbest_paths(&inp, 4) {
                            if pth.len() != 2 {
                                continue;
                            }
                            let w2: String = pth
                                .iter()
                                .map(|p3| self.word_text(p3.3))
                                .collect();
                            let cs: Vec<char> = w2.chars().collect();
                            if (2..=4).contains(&cs.len())
                                && !have3.contains(&w2)
                            {
                                adds.push(w2);
                                n2w += 1;
                                if n2w >= 1 {
                                    break 'outer;
                                }
                            }
                        }
                    }
                    // tick80f 修法B: 尾行多字词面（进提案型 = 头行顶字
                    // × 尾行首个多字词; 行序实证同 py）
                    {
                        let hst = self.dat.walk(s1.as_bytes(), ROOT);
                        let mut hw: Option<String> = None;
                        if hst >= 0 && self.dat.is_terminal(hst) {
                            let (hlo, _) = self.dat.range(hst);
                            let (ho0, ho1) = self.reading_slice(hlo);
                            for oi in ho0..ho1.min(ho0 + 30) {
                                let t7 = self
                                    .word_text(self.wlist_at(oi));
                                if t7.chars().count() == 1 {
                                    hw = Some(t7);
                                    break;
                                }
                            }
                        }
                        let tst2 = self.dat.walk(s2.as_bytes(), ROOT);
                        let mut tw2: Option<String> = None;
                        if tst2 >= 0 && self.dat.is_terminal(tst2) {
                            let (tlo2, _) = self.dat.range(tst2);
                            let (to02, to12) = self.reading_slice(tlo2);
                            for oi in to02..to12.min(to02 + 40) {
                                let t8 = self
                                    .word_text(self.wlist_at(oi));
                                if t8.chars().count() >= 2 {
                                    tw2 = Some(t8);
                                    break;
                                }
                            }
                        }
                        if let (Some(hw), Some(tw2)) = (&hw, &tw2) {
                            let w9 = format!("{}{}", hw, tw2);
                            if !have3.contains(&w9) {
                                adds.push(w9);
                            }
                        }
                    }
                    if !adds.is_empty() {
                        // tick79o: 插位=词块边界（连续 exact 末端;
                        // native 干净态 [8词][4叉积] 对齐）
                        let mut pos = 0usize;
                        for (j, c) in out.iter().take(20).enumerate() {
                            if c.kind == Kind::Exact {
                                pos = j + 1;
                            } else if j > 0 {
                                break;
                            }
                        }
                        if pos < 4 {
                            pos = out.len();
                            for (j, c) in out.iter().enumerate() {
                                if j >= 10 && c.kind == Kind::Fbcol {
                                    pos = j;
                                    break;
                                }
                            }
                        }                        let base3 = out
                            .iter()
                            .map(|c| c.score)
                            .fold(f64::INFINITY, f64::min);
                        let base3 =
                            if base3.is_finite() { base3 } else { 0.0 };
                        let ins: Vec<Cand> = adds
                            .iter()
                            .take(6)
                            .enumerate()
                            .map(|(i6, w6)| {
                                let mut c6 = self.mk_lit(w6, -1);
                                c6.kind = Kind::Fbcol;
                                c6.score =
                                    base3 - 0.0005 * (i6 as f64 + 1.0);
                                c6.score_bias = c6.score;
                                c6
                            })
                            .collect();
                        let n_ins = ins.len();
                        out.splice(pos..pos, ins);
                        if out.len() > topk {
                            out.truncate(topk);
                        }
                        let _ = n_ins;
                    }
                }
            }
        }

        // ---- NNLM 神经层重排（有界微调器, M4）----
        // 役K(agentZ): py 语义 = rerank 后无条件按 score 重排（去 sentence
        // 组优先级）——rerank 空转（全池 encode 失败）也要排, 镜像对齐
        if nn && !out.is_empty() && self.nnlm_ok() {
            self.nnlm_rerank(&mut out, ctx_ids.as_ref());
            out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        }
        // ---- mdltr LightGBM 精排（活体实证特征组复刻）----
        if ltr && !out.is_empty() {
            let has_ctx = ctx.is_some();
            self.ltr_rerank(&mut out, inp.len(), &inp, has_ctx);
            out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        }
        // ADR-002 boost 覆盖层: 忠实管线之后只增不改（None 零开销）
        if let Some(b) = &self.boost {
            let hits = if b.ckvd_inject {
                self.ckvd_lookup(&inp)
            } else {
                Vec::new()
            };
            let bwcn_hits = if b.bwcn_inject {
                self.bwcn_lookup(&inp)
            } else {
                Vec::new()
            };
            let mut alts = if b.sentence_tail_sweep > 0 {
                boost::sentence_tail_sweep(self, &inp, b.sentence_tail_sweep)
            } else {
                Vec::new()
            };
            // B8 全段变体束: 段覆盖多样化（n-best 切分逐段建格; B2.5 只
            // 在最优 DP 路径段格上变词——双词路径 不可达）。B8 变体置
            // tail 之前（B7 打分集按面板序取前位×字符预算——结构多样
            // 性先于频序变体入场）; 全量 HashSet 去重（apply 的相邻
            // dedup 不足跨块）。
            if b.sentence_seg_beam > 0 {
                let seg =
                    boost::sentence_seg_variants(self, &inp, b.sentence_seg_beam);
                let mut seen: std::collections::HashSet<String> =
                    seg.iter().map(|c| c.word.clone()).collect();
                let mut merged = seg;
                merged.extend(
                    alts.into_iter().filter(|c| seen.insert(c.word.clone())),
                );
                alts = merged;
            }
            // B3 前哨: NNLM 语境打分器（惰性装载; ctx 可为空）
            let ctx_text: String =
                ctx.map(|c| c.to_string()).unwrap_or_default();
            let scorer = |words: &[String]| -> Option<Vec<f64>> {
                if std::env::var("B3_DBG").is_ok() {
                    eprintln!(
                        "[b3] scorer 入: {} 词, ctx_len={}",
                        words.len(),
                        ctx_text.len()
                    );
                }
                let seqs: Vec<Vec<u32>> = words
                    .iter()
                    .map(|w| self.nnlm_encode(w).unwrap_or_default())
                    .collect();
                if seqs.iter().any(|s| s.is_empty()) {
                    return None; // 不可编码项 → 整体放弃保序
                }
                let cseq: Vec<u32> = if ctx_text.is_empty() {
                    Vec::new()
                } else {
                    self.nnlm_encode(&ctx_text).unwrap_or_default()
                };
                self.nnlm_score(&cseq, &seqs)
            };
            // B3 域门在闭包供给侧: 组合域（变体在场）或有语境——
            // 无 ctx 的单词域不传 scorer（v2 实证 -1.4pp 病理防护）
            let scorer_avail = b.nnlm_order && (!alts.is_empty() || ctx.is_some());
            boost::apply(
                &inp,
                b,
                &mut out,
                &alts,
                &bwcn_hits,
                &hits,
                if scorer_avail { Some(&scorer) } else { None },
            );
        }
        // B2.7 简拼表键注入（超越档）: 输入恰为简拼表键时, 表内整键候选
        // 置顶（首字母序列 == 输入 的整键命中; tqsdyy→天青色等烟雨）。
        // 只增不改——面板已有词跳过; 无 boost / 忠实门零行为。
        if let Some(b) = &self.boost {
            if b.jianpin_inject && !inp.is_empty() {
                let hits = self.jianpin_key_hits(&inp);
                if !hits.is_empty() {
                    let top = out.first().map(|c| c.score).unwrap_or(0.0);
                    let mut injected = false;
                    for (i, c) in hits.into_iter().take(6).enumerate() {
                        let target = top + 0.5 - 0.01 * i as f64;
                        match out.iter_mut().find(|x| x.word == c.word) {
                            // 在场: 提权到钉序位（表键命中=强意图信号）
                            Some(x) => {
                                if x.score < target {
                                    x.score = target;
                                    injected = true;
                                }
                            }
                            // 缺席: 注入
                            None => {
                                let mut c = c;
                                c.score = target;
                                out.push(c);
                                injected = true;
                            }
                        }
                    }
                    if injected {
                        out.sort_by(|a, b| {
                            b.score
                                .partial_cmp(&a.score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                }
            }
        }
        // 声调筛选（设计-声调筛选 §2 过滤层, 旋钮 tone_filter）:
        // 面板终态过滤——全消耗候选调位验证, 未过者移除（≥1 过时）。
        // 语法层已在入口剥离数字, 此处 tone_pos 非空 = 用户打了调。
        if let Some(b) = &self.boost {
            if std::env::var("TONE_DBG").is_ok() {
                eprintln!(
                    "[tonehook] 旋钮={} 调位数={} out={}",
                    b.tone_filter,
                    tone_pos.len(),
                    out.len()
                );
            }
            if b.tone_filter && !tone_pos.is_empty() {
                boost::tone_filter(self, &mut out, &inp, &tone_pos);
            }
        }
        // B3-τ 语境意图提权（nnlm_order × ctx 门; B2w-τ 同哲学——
        // 上文→候选 NNLM 分显著领先即钉序置顶, RRF 融合的补强件）
        if let Some(b) = &self.boost {
            if b.nnlm_order {
                if let Some(cx) = ctx.filter(|c| !c.is_empty()) {
                    boost::ctx_intent_promote(self, cx, &mut out, &inp);
                }
            }
        }

        // B7 句序列 NNLM 重排（超越档旋钮, 需 --nnlm 模型在场）:
        // 句候选组内纯 NNLM 序——词频和对流畅性失聪（吧社圆鼻 vs
        // 跋涉远比 同量级）, 序列 logprob 定胜负。v2: 组内原位换位
        // （boost.rs 注记）——不做全局 sort, B0 整句前置结构保持。
        if let Some(b) = &self.boost {
            if b.sentence_nnlm_rescore && out.len() > 1 {
                boost::sentence_nnlm_rescore(self, &mut out);
            }
        }
        // 混合输入（设计-混合输入 §2）: 全拼段∪声母段——无 ctx 频序钉分
        // 注入; 有 ctx 且 c.3 质变链在场（旋钮+NNLM 双备）时让位——
        // NNLM 统一重排接管（boost::ctx_jp_compose 混合适配）
        if let Some(bb) = &self.boost {
            if bb.mixed_input && !jianpin && !out.is_empty()
                && !(ctx.is_some()
                    && bb.ctx_jp_compose
                    && self.nnlm_ok())
            {
                boost::mixed_input(self, &inp, &mut out);
            }
        }
        // c.3 词块感知组装 × ctx-NNLM（设计-上下文动态组词 §c.3）:
        // 非合法全拼切分域（ewzdn/xmdbmh 类）的未学习路径正解——词块
        // 优先组装 + NNLM(ctx) 统一尺度重排（详见 boost::ctx_jp_compose
        // 域门注记: 旋钮+ctx+NNLM 三门缺一恒等, 忠实链零行为）。
        if let Some(b) = &self.boost {
            if b.ctx_jp_compose && !jianpin && !out.is_empty() {
                boost::ctx_jp_compose(self, &inp, ctx, &mut out);
            }
        }
        // B6 表外组词注入（compose_inject）: 双音节全拼的字级组词——
        // 词典无词条的双常见字组合（峰澈/澈雪 类）忠实管线任何通道都
        // 不出场, 此处补缺席对。只增不改: 钉分低于面板 min（原生序/
        // top-10 零位移）; 无 boost / 忠实门（golden）零行为。
        if let Some(b) = &self.boost {
            if b.compose_inject && !jianpin {
                let hits = boost::compose_pair_hits(self, &inp, &out);
                if !hits.is_empty() {
                    let base = out
                        .iter()
                        .map(|c| c.score)
                        .fold(f64::INFINITY, f64::min);
                    let base =
                        if base.is_finite() { base } else { 0.0 };
                    for (i, mut c) in hits.into_iter().enumerate() {
                        c.score = base - 0.01 * (i as f64 + 1.0);
                        c.score_bias = c.score;
                        out.push(c);
                    }
                }
            }
        }
        // B4 终位学习加成: 真终位=boost 层之后——面板列重建、nn/ltr 重排、
        // boost 内重排（lmcn 链只看 lmcn 分/B3 RRF 只看双榜）都会洗掉
        // 学习信号; 用户记忆是最高优先个人信号, 必须最后落子。
        // 无用户词典零开销。
        if self.user.borrow().is_some() {
            self.user_boost_final(&inp, &mut out);
            // 上下文动态组词 §a: 用户信号终位 ctx 调整（自门控: 无 ctx/
            // 无 NNLM/无用户信号恒等, golden 双保险零行为）
            self.user_ctx_adjust(ctx, &mut out);
        }
        out
    }

    pub(super) fn ctx_ids(&self, ctx: Option<&str>) -> Option<CtxIds> {
        let ctx = ctx?;
        if ctx.is_empty() {
            return None;
        }
        if let Some(wid) = self.word_id(ctx) {
            return Some(CtxIds {
                wid: Some(wid),
                text: ctx.to_string(),
            });
        }
        // 长上下文回退：取尾部成词片段（你好世界→世界）做联想锚点
        for l in (1..=ctx.char_indices().count().min(4)).rev() {
            let tail = ctx_tail(ctx, l);
            if let Some(wid) = self.word_id(tail) {
                return Some(CtxIds {
                    wid: Some(wid),
                    text: tail.to_string(),
                });
            }
        }
        Some(CtxIds {
            wid: None,
            text: ctx.to_string(),
        })
    }

    // ------------------------------------------------------------------
    // M3: 用户词学习（挂载/事件/注入）
    // ------------------------------------------------------------------
    /// 挂载用户词库（不存在则空库起步；commit 时落盘）
    pub fn attach_user(&self, path: &std::path::Path) {
        *self.user.borrow_mut() = Some(UserDict::load(path));
    }

    /// 从内存字节挂载用户词库（wasm/JS 链路；HHD1 checkpoint，无 fs。
    /// 落盘改走 user_save_bytes 由宿主持久化）
    pub fn attach_user_bytes(&self, bytes: &[u8]) {
        *self.user.borrow_mut() = Some(UserDict::from_bytes(bytes));
    }

    /// 用户词库全量序列化（HHD1 v2 checkpoint；wasm 宿主持久化路径）
    pub fn user_save_bytes(&self) -> Option<Vec<u8>> {
        self.user.borrow_mut().as_mut().map(|ud| ud.save_bytes())
    }

    /// 挂载 ckvd 长尾短语库（ckvdmac.bin，642 万短语）。
    /// 2026-11-07 数据层终解（hanhuo-dict）；管线注入点待 vtable 消费链
    /// 逆向——注入前仅提供查询 API，对现有查询零行为变化。
    /// 分体布局注入: 模型文件（safetensors 开放格式, CLI 侧解析）装入
    /// 惰性槽——优先于包段（nnlm_model/mdltr_score 仅在槽空时回落包）
    /// 查询时钟注入（anchor 实时衰减; 测试口径。None 恢复系统时钟）
    pub fn set_now(&self, now: Option<u64>) {
        self.now_override.set(now);
    }

    /// 查询时钟（注入优先, 否则系统）
    fn now_unix(&self) -> u64 {
        self.now_override.get().unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
    }

    pub fn set_nnlm(&self, m: crate::Nnlm) {
        *self.nnlm.borrow_mut() = Some(Box::new(m));
    }

    /// 分体布局注入: 高级版 NNLM（nlrgpt2 开放格式）——注入即优先于
    /// 轻量版（nnlm_score/nnlm_encode 统一入口的分发序）
    #[cfg(feature = "safetensors")]
    pub fn set_nnlm_adv(&self, m: hanhuo_lm::nnlm_adv::NnlmAdv) {
        *self.nnlm_adv.borrow_mut() = Some(Box::new(m));
    }

    /// 高级版维度（诊断输出用; 未注入 → None）
    #[cfg(feature = "safetensors")]
    pub fn nnlm_adv_dims(
        &self,
    ) -> Option<(usize, usize, usize, usize, usize, usize)> {
        self.nnlm_adv.borrow().as_ref().map(|m| m.dims())
    }

    /// NNLM 统一入口（高级版优先, 轻量惰性回退）。消费方一律走
    /// nnlm_encode/nnlm_vocab_id/nnlm_score——不再各自持模型引用,
    /// 附加高级版时全链（M4/a 层/B3/B7/c.3）自动换脑。
    #[cfg(feature = "safetensors")]
    pub fn nnlm_has_adv(&self) -> bool {
        self.nnlm_adv.borrow().is_some()
    }

    /// 统一文本编码: 高级版优先; 双双缺席/不可编码 → None（含轻量
    /// 惰性解析——无包段且无注入时）
    pub fn nnlm_encode(&self, s: &str) -> Option<Vec<u32>> {
        #[cfg(feature = "safetensors")]
        {
            let g = self.nnlm_adv.borrow();
            if let Some(m) = g.as_ref() {
                return m.encode(s).ok();
            }
        }
        self.nnlm_model().and_then(|m| m.encode(s))
    }

    /// 统一单字词表查询（ctx 逐字回退路径用）
    pub fn nnlm_vocab_id(&self, s: &str) -> Option<u32> {
        #[cfg(feature = "safetensors")]
        {
            let g = self.nnlm_adv.borrow();
            if let Some(m) = g.as_ref() {
                return m.vocab.get(s).copied();
            }
        }
        self.nnlm_model().and_then(|m| m.vocab.get(s).copied())
    }

    /// 统一批量打分: 高级版优先; 轻量惰性解析回退; 双缺 → None
    pub fn nnlm_score(
        &self,
        ctx: &[u32],
        seqs: &[Vec<u32>],
    ) -> Option<Vec<f64>> {
        #[cfg(feature = "safetensors")]
        {
            let g = self.nnlm_adv.borrow();
            if let Some(m) = g.as_ref() {
                return Some(m.score_candidates(ctx, seqs));
            }
        }
        self.nnlm_model()
            .map(|m| m.score_candidates(ctx, seqs))
    }

    /// 无 ctx 域批量打分（B7 句序列重排专用）: **轻量优先**——实测
    /// （2026-11-09）adv 绝对序列分在无语境域判别力崩塌（句评测 top1
    /// 32.7%→5.8%）而 ctx 域（c.3 前缀判别）质变（10.7 nat）→ 双模型
    /// 分工: 语境域 adv 优先（nnlm_score）, 无语境域轻量优先（本入口）。
    pub fn nnlm_score_noctx(
        &self,
        seqs: &[Vec<u32>],
    ) -> Option<Vec<f64>> {
        if self.nnlm_model().is_some() {
            return self.nnlm_model()
                .map(|m| m.score_candidates(&[], seqs));
        }
        #[cfg(feature = "safetensors")]
        {
            let g = self.nnlm_adv.borrow();
            if let Some(m) = g.as_ref() {
                return Some(m.score_candidates(&[], seqs));
            }
        }
        None
    }

    /// 分体布局注入: mdltr（开放格式经 into_native 还原）
    pub fn set_mdltr(&self, m: crate::Mdltr) {
        *self.mdltr.borrow_mut() = Some(Some(Box::new(m)));
    }

    /// BGRM bigram 表注入（分体布局: 段剥出为独立文件, 布局同段体
    /// [n][w1×n][w2×n][val u16×n]; 词 id 与主包词层耦合——注入文件
    /// 必须来自同一 wxq 源）。字节泄漏为 'static 供 U32Arr 借用。
    pub fn set_bgrm(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() < 4 {
            return Err("BGRM 载荷 <4B".into());
        }
        let head: [u8; 4] = bytes[0..4]
            .try_into()
            .map_err(|e: std::array::TryFromSliceError| e.to_string())?;
        let n = u32::from_le_bytes(head) as usize;
        let need = 4 + 10 * n;
        if bytes.len() < need {
            return Err(format!(
                "BGRM 载荷需 {need}B（实 {}B）",
                bytes.len()
            ));
        }
        let bg: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
        self.bi_n = n;
        self.bi_w1 = U32Arr::new(&bg[4..4 + 4 * n]);
        self.bi_w2 = U32Arr::new(&bg[4 + 4 * n..4 + 8 * n]);
        self.bi_val = U16Arr::new(&bg[4 + 8 * n..4 + 10 * n]);
        Ok(())
    }

    pub fn attach_ckvd(&self, path: &std::path::Path) -> bool {
        match hanhuo_dict::CkvdPhrases::from_file(path) {
            Ok(c) => {
                *self.ckvd.borrow_mut() = Some(c);
                true
            }
            Err(err) => {
                eprintln!("[ckvd] 挂载失败 {err}");
                false
            }
        }
    }

    /// ckvd 长尾短语查询（全拼，无分隔符或撇号形式皆可）
    pub fn ckvd_lookup(&self, pinyin: &str) -> Vec<hanhuo_dict::Phrase> {
        self.ckvd
            .borrow()
            .as_ref()
            .map(|c| c.lookup(pinyin))
            .unwrap_or_default()
    }

    /// 挂载 bwcn PC 三元组（boost 词注入源, B2w）。
    pub fn attach_bwcn(&self, path: &std::path::Path) -> bool {
        match hanhuo_dict::bwcn::BwcnTriples::from_file(path) { // 魔数路由: HBWC 优先/tsv 回退
            Ok(b) => {
                // 多词库（领域词库: 医学/计算机/行业…）——累加挂载,
                // lookup 跨库聚合去重
                self.bwcn.borrow_mut().push(b);
                true
            }
            Err(err) => {
                eprintln!("[bwcn] 挂载失败 {err}");
                false
            }
        }
    }

    /// bwcn 全拼直查（无分隔形; 频序降序 cap 4——注入序用）
    pub fn bwcn_lookup(&self, py: &str) -> Vec<hanhuo_dict::Phrase> {
        let dbg = std::env::var("HBWC_DBG").is_ok();
        if dbg { eprintln!("[dbg] bwcn_lookup({py}) dicts={}", self.bwcn.borrow().len()); }
        // 跨库聚合 + 词面去重（同词首库优先）+ weight 降序
        let mut out: Vec<hanhuo_dict::Phrase> = Vec::new();
        let mut seen: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for b in self.bwcn.borrow().iter() {
            for p in b.lookup(py, 4) {
                if seen.insert(p.text.clone()) {
                    out.push(p);
                }
            }
        }
        out.sort_by(|a, b| b.weight.cmp(&a.weight));
        if dbg {
            eprintln!(
                "[dbg] bwcn_lookup({py}) -> {} 条: {:?}",
                out.len(),
                out.iter().map(|p| p.text.clone()).collect::<Vec<_>>()
            );
        }
        out
    }

    /// 挂载 stt 简→繁词典（stt.bin）。输出层转换，不改动候选管线。
    pub fn attach_stt(&self, path: &std::path::Path) -> bool {
        match hanhuo_dict::S2t::from_file(path) {
            Ok(s) => {
                *self.stt.borrow_mut() = Some(s);
                true
            }
            Err(err) => {
                eprintln!("[stt] 挂载失败 {err}");
                false
            }
        }
    }

    /// 文本转繁体（未挂载 stt 时原样返回）
    pub fn to_traditional(&self, text: &str) -> String {
        self.stt
            .borrow()
            .as_ref()
            .map(|s| s.s2t_text(text))
            .unwrap_or_else(|| text.to_string())
    }

    pub fn user_flush(&self) {
        if let Some(u) = self.user.borrow_mut().as_mut() {
            u.flush();
        }
    }

    pub fn user_entry(&self, key: &str, word: &str) -> Option<Entry> {
        self.user.borrow().as_ref()?.get(key, word)
    }

    /// 学习事件（晋升门: 词典内 + 读音唯一 + ≤24B；key=读音全串）。
    /// rid_hint = 打字读音（如 walk(nihao) 的精确键）；无则用词主读音。
    pub fn user_commit(
        &self,
        word: &str,
        rid_hint: Option<u32>,
        ts: u64,
    ) -> bool {
        if word.len() > MAX_WORD_BYTES {
            return false; // 超长拒收
        }
        let Some(wid) = self.word_id(word) else {
            return false; // 非词典词拒收（英文等）
        };
        if self.is_poly(wid) {
            return false; // 多音字拒收（的/行/重…）
        }
        let rid = match rid_hint {
            Some(r) => r,
            None => {
                let base = self.w2r.get(wid as usize);
                if base == 0 {
                    return false; // 无中文读音（英文词等）
                }
                base - 1
            }
        };
        let key = self.rid_key(rid);
        if key.is_empty() {
            return false;
        }
        match self.user.borrow_mut().as_mut() {
            Some(ud) => {
                ud.commit(&key, word, ts);
                true
            }
            None => false,
        }
    }

    /// 多音字集合（同词 ≥2 读音；惰性一次构建: 扫全读音词列 ~70 万条）
    fn is_poly(&self, wid: u32) -> bool {
        if self.poly.borrow().is_none() {
            let mut cnt: HashMap<u32, u32> = HashMap::new();
            for r in 0..self.ridx_n as u32 {
                let (o0, o1) = self.reading_slice(r);
                for oi in o0..o1 {
                    *cnt.entry(self.wlist_at(oi)).or_insert(0) += 1;
                }
            }
            let set: HashSet<u32> = cnt
                .into_iter()
                .filter(|(_, c)| *c >= 2)
                .map(|(w, _)| w)
                .collect();
            *self.poly.borrow_mut() = Some(set);
        }
        self.poly.borrow().as_ref().unwrap().contains(&wid)
    }

    /// 读音 → 学习 key（音节逗号全串, 如 "ni,hao"）
    pub(super) fn rid_key(&self, rid: u32) -> String {
        let vs = self.rid_variants(rid);
        if vs.is_empty() {
            return String::new();
        }
        vs[0]
            .iter()
            .map(|&i| self.syllables[i as usize].clone())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// B4 终位学习加成: 终面板上按 (reading→key, word) 重匹配用户词典,
    /// user_boost 并入 score/user_bias 后重排。中段 user_boost_merge 的
    /// 信号会被面板列重建与 nn/ltr 重排洗掉——此处是唯一可靠落点。
    pub(super) fn user_boost_final(&self, inp: &str, out: &mut Vec<Cand>) {
        let dbg = std::env::var("USER_DBG").is_ok();
        let now = self.now_unix(); // anchor 实时衰减基准
        let ud_guard = self.user.borrow();
        let Some(ud) = ud_guard.as_ref() else {
            return;
        };
        // reading → 该组词加成缓存（一次查询内少量不同读音）
        let mut cache: HashMap<String, HashMap<String, f64>> =
            HashMap::new();
        let mut key_of = |rd: &str| -> String {
            rd.replace('\u{27}', "\u{2c}")
        };
        let mut touched = false;
        for c in out.iter_mut() {
            if c.reading.is_empty() || c.word.contains('\u{1}') {
                continue;
            }
            let k = key_of(&c.reading);
            let boosts = cache.entry(k.clone()).or_insert_with(|| {
                ud.lookup(&k)
                    .into_iter()
                    .map(|(w, e)| (w, user_boost_at(e.anchor, e.ts, now)))
                    .collect()
            });
            if let Some(b) = boosts.get(&c.word) {
                if dbg {
                    eprintln!("[udbg] 终位 {} +{b:.3}（{}）", c.word, c.reading);
                }
                c.user_bias += *b;
                c.score += *b;
                touched = true;
            }
        }
        // DAT 终态学习组双通道: 中段 Kind::User 补位候选能进面板但
        // ①其 bias 被面板钉分覆写 ②其 reading 由 mk 主读音派生（可与
        // 学习 key 不同读音）→ reading 匹配不上。此处按词面匹配补加
        // （user_bias==0 守卫防双计）; 真缺席词以中频 exact 钉分基座
        // (≈5.0)+加成入板, anchor≥2 才上板（单次误选不劫持面板）。
        let st = self.dat_walk_public(inp.as_bytes());
        if st >= 0 && self.dat.is_terminal(st) {
            let rid = self.dat_range_lo_public(st);
            let key = self.rid_key_public(rid);
            if !key.is_empty() {
                const ABSENT_BASE: f64 = 5.0; // 中频 exact 钉分常数域
                let floor = user_boost(2.0);
                for (w, e) in ud.lookup(&key).into_iter().take(2) {
                    let b = user_boost_at(e.anchor, e.ts, now);
                    if b < floor {
                        continue;
                    }
                    // B4 双面孔双计修复（G 移交发现 1）: 同词面已有携带
                    // 用户信号的面（reading 通道已加成的 Exact）→ 信号
                    // 单面承载, 不再给第二面孔（中段 User 补位, 其 bias
                    // 被面板钉分覆写为 0 而绕过守卫）补加成; 残留同词
                    // User 面一并清出（防 火锅 Exact+ub 与 User 5.0+b
                    // 两面同板）。
                    if out.iter().any(|c| c.word == w && c.user_bias != 0.0)
                    {
                        out.retain(|c| {
                            !(c.kind == Kind::User
                                && c.word == w
                                && c.user_bias == 0.0)
                        });
                        continue;
                    }
                    // 通道 1: 已在板（含中段补位, 无论其 reading 形）
                    if let Some(c) =
                        out.iter_mut().find(|c| c.word == w && c.user_bias == 0.0)
                    {
                        if dbg {
                            eprintln!("[udbg] 终位词面补加 {w} +{b:.3}");
                        }
                        c.user_bias = b;
                        c.score += b;
                        // 中段补位的 reading 由 mk 主读音派生, 可与本组
                        // 学习 key 异读（一招 [long'li] 案）→ 以组 key 矫正;
                        // 其携带分是通道低分（附录14 洗后残形）——中频基座
                        // 保底, 学习缺席词不入深水区
                        if c.kind == Kind::User {
                            c.reading = key.replace(',', "'");
                            c.score = c.score.max(ABSENT_BASE + b);
                        }
                        touched = true;
                        continue;
                    }
                    // 通道 2: 真缺席 → 入板
                    let Some(wid) = self.word_id(&w) else {
                        continue;
                    };
                    let mut c =
                        self.mk(wid, -1, Kind::User, 0, None, 0.0, None);
                    c.reading = key.replace(',', "'");
                    c.user_bias = b;
                    c.score = ABSENT_BASE + b;
                    if dbg {
                        eprintln!("[udbg] 终位补位 {w} = {ABSENT_BASE}+{b:.3}");
                    }
                    out.push(c);
                    touched = true;
                }
            }
        }
        if touched {
            out.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        // B5 组词/组句: 用户短语表精确命中（无分隔键 == 输入）→ 注入
        // Kind::User。首学 anchor=1 → boost 0 → 与 top1 平分殿后（稳定
        // 排序原生先）; 复用 anchor≥2 → 越位 top1。同词面去重（面板已有
        // 则只走上面的词级加成）。
        let phs = ud.phrase_lookup(inp);
        if !phs.is_empty() {
            let top = out.first().map(|c| c.score).unwrap_or(0.0);
            let mut injected = false;
            for (text, pe) in phs {
                if out.iter().any(|c| c.word == text) {
                    continue;
                }
                let b = user_boost_at(pe.e.anchor, pe.e.ts, now);
                let mut c =
                    Cand::new(text.clone(), -8.0, crate::Kind::User);
                c.reading =
                    pe.reading.replace('\u{2c}', "\u{27}"); // 逗号→撇号展示形
                c.score = top + b;
                c.user_bias = b;
                if dbg {
                    eprintln!(
                        "[udbg] 短语注入 {} anchor={:.2} +{b:.3}",
                        c.word, pe.e.anchor
                    );
                }
                out.push(c);
                injected = true;
            }
            if injected {
                out.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        // c.2 简拼桥（设计-上下文动态组词 §c.2）: 输入非合法全拼切分
        // （L1 死/纯声母串, 如 ewzdn）时, 学得短语的简拼投影整键命中
        // ——钉分律同精确块（top+user_boost(anchor), Kind::User, 去重）。
        // 门控内联（is_syl 贪心可达, 不依赖 typo_retry——其在途）。
        {
            let b = inp.as_bytes();
            let mut reach = vec![false; b.len() + 1];
            reach[0] = true;
            for i in 0..b.len() {
                if !reach[i] {
                    continue;
                }
                for j in i + 1..=(i + 7).min(b.len()) {
                    if self.seg.is_syl(&inp[i..j]) {
                        reach[j] = true;
                    }
                }
            }
            if !reach[b.len()] {
                let phs =
                    ud.phrase_initial_lookup(inp, 4, &|s| self.seg.is_syl(s));
                if !phs.is_empty() {
                    let top = out.first().map(|c| c.score).unwrap_or(0.0);
                    let mut injected = false;
                    for (text, pe) in phs {
                        if out.iter().any(|c| c.word == text) {
                            continue;
                        }
                        let b = user_boost_at(pe.e.anchor, pe.e.ts, now);
                        let mut c = Cand::new(text, -8.0, crate::Kind::User);
                        c.reading = pe
                            .reading
                            .replace('\u{2c}', "\u{27}");
                        c.score = top + b;
                        c.user_bias = b;
                        out.push(c);
                        injected = true;
                    }
                    if injected {
                        out.sort_by(|a, b| {
                            b.score.partial_cmp(&a.score).unwrap_or(
                                std::cmp::Ordering::Equal,
                            )
                        });
                    }
                }
            }
        }
        // B5-2 前缀补全: 学得短语随打随出（键前缀命中; 精确块已处理）。
        // 分数 = top + boost(anchor) − 0.5: anchor=1 首学沉半步（不打扰
        // 原生序）, 复用 anchor≥2 越位（用户明确偏好）。≥4B 门槛防单字母
        // 倾倒。同文本去重。
        if inp.len() >= 4 {
            let comps = ud.phrase_prefix_lookup(inp, 4);
            if !comps.is_empty() {
                let top = out.first().map(|c| c.score).unwrap_or(0.0);
                let mut injected = false;
                for (_key, text, pe) in comps {
                    if out.iter().any(|c| c.word == text) {
                        continue;
                    }
                    let b = user_boost_at(pe.e.anchor, pe.e.ts, now);
                    let mut c =
                        Cand::new(text.clone(), -8.0, crate::Kind::User);
                    c.reading =
                        pe.reading.replace('\u{2c}', "\u{27}");
                    c.score = top + b - 0.5;
                    c.user_bias = b;
                    if dbg {
                        eprintln!(
                            "[udbg] 前缀补全 {} anchor={:.2}",
                            c.word, pe.e.anchor
                        );
                    }
                    out.push(c);
                    injected = true;
                }
                if injected {
                    out.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
    }

    /// B5-3 选词回报: IME 前端上屏后回报（输入拼音 + 所选词面）→
    /// 自动学习。句选候选携 path_words（DP 词路径）→ 按序逐词 commit,
    /// 会话 n-gram 随之成形（组句学习的主驱动）; 无路径/单词 → 直接
    /// commit。面板外词走 user_commit 拒收门（词典/多音/超长）。
    /// 反馈不在热路径——一次内部查询可接受。
    pub fn feedback(&self, inp: &str, chosen: &str, ts: u64) -> bool {
        let cands =
            self.query(inp, None, false, 50, false, false, true);
        if let Some(c) = cands.iter().find(|c| c.word == chosen) {
            match &c.path_words {
                Some(ws) if ws.len() >= 2 => {
                    let mut any = false;
                    for w in ws {
                        any |= self.user_commit(w, None, ts);
                    }
                    any
                }
                _ => self.user_commit(chosen, None, ts),
            }
        } else {
            self.user_commit(chosen, None, ts)
        }
    }

    /// B2.7 简拼整键命中: q_jianpin 面板中首字母序列恰等于输入的候选
    /// （表内键的真语义命中; 部分前缀/列候选排除）。
    fn jianpin_key_hits(&self, inp: &str) -> Vec<Cand> {
        let initials: String = inp
            .chars()
            .map(|c| c.to_ascii_lowercase())
            .collect();
        self.q_jianpin(inp)
            .into_iter()
            .filter(|c| {
                !c.reading.is_empty()
                    && c.reading
                        .split('\u{27}')
                        .map(|syl| {
                            syl.chars().next().unwrap_or(' ').to_ascii_lowercase()
                        })
                        .collect::<String>()
                        == initials
            })
            .take(6)
            .collect()
    }

    /// B5-4 删词（IME 删词手势后端）: 学习词全键清除, 即时落盘。
    pub fn user_remove(&self, word: &str) -> bool {
        match self.user.borrow_mut().as_mut() {
            Some(ud) => ud.remove_word(word),
            None => false,
        }
    }

    /// 删词级联（--cascade）: 删词 + 连带删文本包含该词的短语
    /// （单字词不级联——见 userdict::remove_word_cascade 注记）
    pub fn user_remove_cascade(&self, word: &str) -> (bool, usize) {
        match self.user.borrow_mut().as_mut() {
            Some(ud) => ud.remove_word_cascade(word),
            None => (false, 0),
        }
    }

    /// B5-4 短语容量帽配置（clamp 界内; 下次 flush 持久化到 HHD1 头）。
    pub fn user_set_max_phrases(&self, n: usize) -> usize {
        match self.user.borrow_mut().as_mut() {
            Some(ud) => {
                ud.set_max_phrases(n);
                ud.max_phrases()
            }
            None => 0,
        }
    }

    /// B5-4 删自造短语（文本精确匹配）。
    pub fn user_remove_phrase(&self, text: &str) -> bool {
        match self.user.borrow_mut().as_mut() {
            Some(ud) => ud.remove_phrase(text),
            None => false,
        }
    }

    /// 词 → 学习键（w2r 主读反查; 非词典/多音/无读 → None）
    /// ——CLI 无 --py 提交时的展示/对账用（与 user_commit 同判）
    pub fn user_key_of(&self, word: &str) -> Option<String> {
        let wid = self.word_id(word)?;
        if self.is_poly(wid) {
            return None;
        }
        let base = self.w2r.get(wid as usize);
        if base == 0 {
            return None;
        }
        let key = self.rid_key(base - 1);
        (!key.is_empty()).then_some(key)
    }

    /// 学习组加成: 命中词 boost 并入 score_bias；缺席高热词补位（Kind::User）
    pub(super) fn user_boost_merge(&self, rid: u32, out: &mut Vec<Cand>) {
        let dbg = std::env::var("USER_DBG").is_ok();
        if dbg { eprintln!("[udbg] merge rid={rid} key={:?}", self.rid_key(rid)); }
        let ud_guard = self.user.borrow();
        let Some(ud) = ud_guard.as_ref() else {
            return;
        };
        let entries = ud.lookup(&self.rid_key(rid));
        if entries.is_empty() {
            return;
        }
        // NATIVE_USER_BOOST=true 切原生 user_dict_lookup 曲线（服务器行为仿真）；
        // false=本地 1:1（本地引擎该路径惰性，活体实证排名零变化）
        const NATIVE_USER_BOOST: bool = false;
        let mut boosts: HashMap<u32, f64> = HashMap::new();
        for (w, e) in &entries {
            if let Some(wid) = self.word_id(w) {
                let b = if NATIVE_USER_BOOST {
                    user_boost_native(e.count, 0, e.anchor, false) as f64
                } else {
                    user_boost(e.anchor)
                };
                boosts.insert(wid, b);
            }
        }
        if boosts.is_empty() {
            if dbg { eprintln!("[udbg] boosts 空（entries={:?})", entries.len()); }
            return;
        }
        let mut present: HashSet<u32> = HashSet::new();
        for c in out.iter_mut() {
            if c.wid >= 0 {
                if let Some(b) = boosts.get(&(c.wid as u32)) {
                    if dbg { eprintln!("[udbg] 命中 wid={} word={} +{b:.3}", c.wid, c.word); }
                    c.user_bias += *b;
                    present.insert(c.wid as u32);
                }
            }
        }
        for (w, e) in entries.iter().take(2) {
            if let Some(wid) = self.word_id(w) {
                if !present.contains(&wid) {
                    // user_bias 不预置: 面板钉分覆写 score 使其永不生效,
                    // 且非零值会挡住终位通道 1 的词面匹配守卫
                    let c = self.mk(wid, -1, Kind::User, 0, None, 0.0, None);
                    out.push(c);
                }
            }
        }
    }

    /// mdltr LightGBM raw score（131 树 Σ 叶值；官方 f32 喂入路径）
    pub fn mdltr_score(&self, x: &[f32; N_FEAT]) -> Option<f64> {
        if self.mdltr.borrow().is_none() {
            let parsed = self
                .pack
                .raw(b"MDLT")
                .ok()
                .and_then(|s| Mdltr::parse(s).ok())
                .map(Box::new);
            *self.mdltr.borrow_mut() = Some(parsed);
        }
        self.mdltr.borrow().as_ref().unwrap().as_ref().map(|m| m.predict(x))
    }

    /// wtid 词特征（11B；研究资产——179 维实装走 score.rs 候选派生语义,
    /// 见 FEATURES.md 七役；本表不参与排序）
    pub fn word_feat11(&self, word: &str) -> Option<[u8; 11]> {
        if self.wfea.borrow().is_none() {
            let parsed = self
                .pack
                .raw(b"WFEA")
                .ok()
                .and_then(|s| Wfea::parse(s).ok())
                .map(Box::new);
            *self.wfea.borrow_mut() = Some(parsed);
        }
        self.wfea
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .and_then(|w| w.get(word))
    }

    // ------------------------------------------------------------------
    // CLI 只读探针（公开）
    // ------------------------------------------------------------------
    pub fn dat_walk_public(&self, key: &[u8]) -> i32 {
        self.dat.walk(key, ROOT)
    }

    pub fn dat_range_lo_public(&self, state: i32) -> u32 {
        self.dat.range(state).0
    }

    pub fn rid_key_public(&self, rid: u32) -> String {
        self.rid_key(rid)
    }

    // ------------------------------------------------------------------
    // 役N ref5: 英文静态词典直通域（bwcnenmac 提取词表, 本地资产）
    // ------------------------------------------------------------------
    /// 英文词典词集（纯小写拉丁词）。文件缺失/为空 → None（律休眠）。
    /// （py `_engdict_words`; bwcnenmac_words.tsv 首行为 header）
    fn engdict_words(&self) -> Option<std::cell::Ref<'_, HashSet<String>>> {
        if self.engdict_v.borrow().is_none() {
            // py _ENGDICT_TSV = os.path.expanduser('~/tmp/...')
            let raw = "~/tmp/hanhuo/extracted/3.0.1.1/BuildIn/bwcnenmac_words.tsv";
            let tsv = if let Some(rest) = raw.strip_prefix("~/") {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join(rest)
            } else {
                std::path::PathBuf::from(raw)
            };
            let mut ws: HashSet<String> = HashSet::new();
            if let Ok(data) = std::fs::read_to_string(&tsv) {
                for (ln, line) in data.lines().enumerate() {
                    if ln == 0 {
                        continue; // header: word slot_hex
                    }
                    if line.is_empty() {
                        continue;
                    }
                    let Some((w, _rest)) = line.split_once('\t') else {
                        continue;
                    };
                    if w.is_empty() {
                        continue;
                    }
                    // py: w.isascii() and w.isalpha() and w.islower()
                    let ascii_lower_alpha = !w.is_empty()
                        && w.bytes().all(|c| c.is_ascii_lowercase());
                    if ascii_lower_alpha {
                        ws.insert(w.to_string());
                    }
                }
            }
            *self.engdict_v.borrow_mut() = Some(ws);
        }
        if self.engdict_v.borrow().as_ref().is_some_and(|s| s.is_empty()) {
            return None; // 空 → 律休眠（py `return ws or None`）
        }
        Some(std::cell::Ref::map(self.engdict_v.borrow(), |o| {
            o.as_ref().unwrap()
        }))
    }

    /// 纯拉丁英文词直通（llama 律）。门: ≥3 键纯字母 × 无任何拼音音节
    /// 前缀（ll/llama 串不成音节; hello 的 he 前缀排除）× 词表命中。
    /// kind=sentence 钉顶（native llama #0 在 拉拉\x01吗 面之前）。
    /// （py `_latin_direct`）
    pub(super) fn latin_direct(&self, inp: &str) -> Option<Cand> {
        if inp.len() < 3 || !inp.is_ascii() || !inp.bytes().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        // py: any(inp[:L] in self._sylset_m for L in range(1, min(6, len(inp)) + 1))
        let syls = self.sylset_m();
        for l in 1..=(6.min(inp.len())) {
            if syls.contains(&inp[..l]) {
                return None;
            }
        }
        let ws = self.engdict_words()?;
        if !ws.contains(inp) {
            return None;
        }
        let mut c = Cand::new(inp.to_string(), 2.0, Kind::Sentence);
        c.reading = inp.to_string();
        c.score = 2.0;
        c.wid = -1;
        Some(c)
    }
}

// ---------------------------------------------------------------------------
// 单元测试: 合成微型引擎（全内存, 不依赖 wxq.bin）。
// 覆盖纯逻辑接缝——词表 front-code 解码/词哈希双代/读音变体解码/区间
// 合并两路/hot bisect/L1 切分/DP 词格/上下文回退/用户词门控/可选资源
// 惰性。主查询 query() 需完整管线+DAT 全量, 归 golden 回归（hanhuo-cli）。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn tone_engine() -> Engine {
        let bin = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
            format!(
                "{}/tmp/hanhuo/wxq_dict.bin",
                std::env::var("HOME").unwrap()
            )
        });
        Engine::open(std::path::Path::new(&bin)).expect("wxq_dict.bin 装载")
    }

    /// B3-τ2 词典搭配直提（语境意图, 确定性查表）
    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn ctx_intent_collocation_promote() {
        let mut e = tone_engine();
        e.set_boost(crate::engine::BoostCfg {
            nnlm_order: true,
            sentence_tail_sweep: 6,
            ..Default::default()
        });
        // 世外+taoyuan: 桃源 压 桃园（网络错形先验双 NNLM 判反的案）
        let out = e.query("taoyuan", Some("世外"), false, 5, true, true, true);
        assert_eq!(
            out.first().map(|c| c.word.as_str()),
            Some("桃源"),
            "τ2 搭配直提: {:?}",
            out.iter().map(|c| c.word.clone()).take(3)
                .collect::<Vec<_>>()
        );
        // 邯郸+xuebu: 学步 登顶
        let out2 = e.query("xuebu", Some("邯郸"), false, 5, true, true, true);
        assert_eq!(out2.first().map(|c| c.word.as_str()), Some("学步"));
        // 无 ctx 恒等: taoyuan 无上文仍 桃园 首位
        let out3 = e.query("taoyuan", None, false, 5, true, true, true);
        assert_eq!(out3.first().map(|c| c.word.as_str()), Some("桃园"));
    }

    // 审计用: 导出 wxq 主库全词表（word_text, 频序）
    #[test]
    #[ignore = "真机: 需 wxq_dict.bin"]
    fn dump_wxq_words() {
        let bin = std::env::var("WXQ_BIN").unwrap_or_else(|_| {
            format!("{}/tmp/hanhuo/wxq_dict.bin",
                std::env::var("HOME").unwrap())
        });
        let e = Engine::open(std::path::Path::new(&bin)).unwrap();
        use std::io::Write;
        let mut f = std::fs::File::create("/tmp/wxq_words.txt").unwrap();
        for w in 0..e.n_words as u32 {
            writeln!(f, "{}", e.word_text(w)).unwrap();
        }
        eprintln!("导出 {} 词 → /tmp/wxq_words.txt", e.n_words);
    }

    /// ---- 声调筛选（设计-声调筛选）真机测试 ----

    #[test]
    #[ignore = "真机: 需 wxq_dict.bin + data/tone_solved.tsv"]
    fn tone_filter_single_syllable_split() {
        // shi2/shi4 分调: 单音节走 syl_deep 钉表面板早退——过滤须挂
        // 早退路径; 时(shi2) 系与 是/事/市/室(shi4) 系互斥
        let mut e = tone_engine();
        e.set_boost(crate::engine::BoostCfg {
            tone_filter: true,
            ..Default::default()
        });
        let w2: Vec<String> = e
            .query("shi2", None, false, 5, true, true, true)
            .into_iter()
            .map(|c| c.word)
            .collect();
        assert_eq!(w2.first().map(String::as_str), Some("时"), "shi2 top1");
        assert!(!w2.iter().take(3).any(|w| w == "是" || w == "事"),
            "shi4 系应滤除: {w2:?}");
        let w4: Vec<String> = e
            .query("shi4", None, false, 5, true, true, true)
            .into_iter()
            .map(|c| c.word)
            .collect();
        assert_eq!(w4.first().map(String::as_str), Some("是"), "shi4 top1");
        // 语法层: 旋钮关时数字剥离→面板 == 无调输入面板（digits ignored）
        let e2 = tone_engine();
        let a = e2.query("shi2", None, false, 5, true, true, true);
        let b = e2.query("shi", None, false, 5, true, true, true);
        assert_eq!(
            a.iter().map(|c| c.word.clone()).collect::<Vec<_>>(),
            b.iter().map(|c| c.word.clone()).collect::<Vec<_>>(),
            "旋钮关: 调号剥离恒等"
        );
    }

    use super::*;

    /// 与 userdict 测试同源的时钟基（anchor 实时衰减测试口径）
    const NOW_FOR_TESTS: u64 = 1_790_000_000;
    use crate::pack::CKPT;

    /// 词面表（wid 0..4）; freq 取量化锚点值: 47104+1024k → logp = −k
    /// （47104=quant::BASE → 0.0; 48128 → −1.0; 49152 → −2.0）
    const WORDS: [&str; 5] = ["啊", "阿坝", "吧", "你好", "世界"];

    // ---- 位场/数组合成（同 dat.rs 测试惯例）----

    /// w 位定宽小端位场编码 → BitArr（+8B u64 读窗余量）
    fn bitarr(vals: &[u32], w: u32) -> BitArr {
        let nbits = vals.len() * w as usize;
        let mut b = vec![0u8; nbits / 8 + 9];
        for (i, &v) in vals.iter().enumerate() {
            let bit = i * w as usize;
            let x = (v as u64) << (bit & 7);
            let byte = bit >> 3;
            for k in 0..8 {
                b[byte + k] |= (x >> (8 * k)) as u8;
            }
        }
        BitArr::new(Box::leak(b.into_boxed_slice()), w)
    }

    fn u16arr(vals: &[u16]) -> U16Arr {
        let mut b = Vec::new();
        for v in vals {
            b.extend_from_slice(&v.to_le_bytes());
        }
        U16Arr::new(Box::leak(b.into_boxed_slice()))
    }

    fn u32arr(vals: &[u32]) -> U32Arr {
        let mut b = Vec::new();
        for v in vals {
            b.extend_from_slice(&v.to_le_bytes());
        }
        U32Arr::new(Box::leak(b.into_boxed_slice()))
    }

    /// front-coded 字符串区合成（检查点=全词, 其余=共享前缀字符数+后缀;
    /// 与 pack 构建侧同构; 词数 < CKPT 时仅 0 号为检查点）
    fn fcstrs(words: &[&str]) -> FcStrings {
        let mut blob: Vec<u8> = Vec::new();
        let mut off: Vec<u32> = Vec::new();
        for (i, w) in words.iter().enumerate() {
            off.push(blob.len() as u32);
            if i % CKPT == 0 {
                blob.push(0); // 检查点: blob[p] 未用
                blob.push(w.len() as u8);
                blob.extend_from_slice(w.as_bytes());
            } else {
                let m = words[i - 1]
                    .chars()
                    .zip(w.chars())
                    .take_while(|(a, b)| a == b)
                    .count() as u8;
                let suf: String = w.chars().skip(m as usize).collect();
                blob.push(m);
                blob.push(suf.len() as u8);
                blob.extend_from_slice(suf.as_bytes());
            }
        }
        off.push(blob.len() as u32);
        FcStrings::new(Box::leak(blob.into_boxed_slice()), u32arr(&off))
    }

    /// v3 词哈希槽表合成（槽=id+1/0=空; fnv 定位+线性探测, 同构建侧）
    fn whash_v3_slots(words: &[&str], n: usize) -> U32Arr {
        let mut slots = vec![0u32; n];
        for (id, w) in words.iter().enumerate() {
            let mut j = (fnv1a(w.as_bytes()) & (n as u32 - 1)) as usize;
            while slots[j] != 0 {
                j = (j + 1) & (n - 1);
            }
            slots[j] = id as u32 + 1;
        }
        u32arr(&slots)
    }

    /// v1/v2 交织槽表合成（[2j]=id+1, [2j+1]=fnv 哈希）
    fn whash_v1_slots(words: &[&str], n: usize) -> U32Arr {
        let mut slots = vec![0u32; 2 * n];
        for (id, w) in words.iter().enumerate() {
            let h = fnv1a(w.as_bytes());
            let mut j = (h & (n as u32 - 1)) as usize;
            while slots[2 * j] != 0 {
                j = (j + 1) & (n - 1);
            }
            slots[2 * j] = id as u32 + 1;
            slots[2 * j + 1] = h;
        }
        u32arr(&slots)
    }

    /// 微型前缀 DAT: 键 "abcd"/"b"（DFS 叶序 abcd=0, b=1; slot: 1=ROOT,
    /// 2="a", 3="b"(终端), 4="ab", 5="abc", 6="abcd"(终端)）——a/ab/abc
    /// 为可走非终端前缀, 供 dp_path 补全分支消费
    fn prefix_dat() -> Dat {
        Dat {
            base: bitarr(&[0, 1, 2, 0, 2, 2, 0], 4),
            check: bitarr(&[0, 0, 1, 1, 2, 4, 5], 4),
            lo: bitarr(&[0, 0, 0, 1, 0, 0, 0], 4),
            cnt16: u16arr(&[0, 2, 1, 1, 1, 1, 1]),
            term: Box::leak(vec![0x48u8].into_boxed_slice()), // 槽 3/6
            n_slots: 7,
            n_keys: 2,
            cnt_ovf: HashMap::new(),
        }
    }

    /// 合成微型引擎: 音节表 a/b/ab/ni/hao/shi;
    /// 读音 r0=[啊,你好] r1=[你好,吧] r2=[吧]（你好/吧 跨读音 → 多音字;
    /// r2 无变体 → 空 rid 边界）; w2r: 阿坝→r0, 吧→r1, 你好→r0
    fn micro() -> Engine {
        let syllables: Vec<String> =
            ["a", "b", "ab", "ni", "hao", "shi"].iter().map(|s| s.to_string()).collect();
        let weights = vec![1u32; syllables.len()];
        // RIDX syl blob: r0 = [[a,ab],[ni]], r1 = [[b]], r2 = []（nv=0）
        let syl_blob: &[u8] = Box::leak(
            vec![2, 2, 0, 0, 2, 0, 1, 3, 0, 1, 1, 1, 0, 0]
                .into_boxed_slice(),
        );
        Engine {
            now_override: std::cell::Cell::new(None),
            boost: None,
            t9: false,
            t9_sig_v: RefCell::new(None),
            bwcn: std::cell::RefCell::new(Vec::new()),
            pack: Pack { bytes: &[], ver: 3, dir: HashMap::new() },
            meta_n_words: WORDS.len(),
            dat: prefix_dat(),
            ridx_n: 3,
            ridx_off: u32arr(&[0, 2, 4, 5]),
            soff: u32arr(&[0, 9, 13, 14]),
            syl_blob,
            wlist: u32arr(&[0, 3, 3, 2, 2]),
            wlsi_fv: None,
            n_words: WORDS.len(),
            freq: u16arr(&[48128, 49152, 47104, 48128, 47104]),
            _flags: &[],
            fc: fcstrs(&WORDS),
            whash_slots: whash_v3_slots(&WORDS, 8),
            whash_n: 8,
            whash_v3: true,
            hot_states: u32arr(&[10, 20, 30]),
            hot_off: u32arr(&[0, 2, 3, 3]),
            hot_ids: u32arr(&[0, 3, 4]),
            syllables: syllables.clone(),
            seg: SyllableSeg::new(syllables, &weights),
            sylw: weights,
            bi_n: 0,
            bi_w1: u32arr(&[]),
            bi_w2: u32arr(&[]),
            bi_val: u16arr(&[]),
            w2r: u32arr(&[0, 1, 2, 1, 0]),
            jian_n: 0,
            jian_keys: fcstrs(&[]),
            jian_eoff: u32arr(&[0]),
            jian_eblob: &[],
            epyn_n: 0,
            epyn_keys: fcstrs(&[]),
            epyn_vblob: &[],
            epyn_tsv: HashMap::new(),
            c4g_pool: fcstrs(&[]),
            c4g_npair: 0,
            c4g_w1: u32arr(&[]),
            c4g_w2: u32arr(&[]),
            c4g_val: u16arr(&[]),
            wtext: RefCell::new(HashMap::new()),
            rid_memo: RefCell::new(HashMap::new()),
            epyn_parsed: RefCell::new(None),
            syl_by_initial: RefCell::new(None),
            syls_by_ini: RefCell::new(None),
            fb_ini_stack: RefCell::new(None),
            fb_syl_head: RefCell::new(None),
            ini_top_syls_c: RefCell::new(None),
            sylset_m_v: RefCell::new(None),
            slot_col_v: RefCell::new(None),
            slot_wc_v: RefCell::new(None),
            user: RefCell::new(None),
            poly: RefCell::new(None),
            nnlm: RefCell::new(None),
            #[cfg(feature = "safetensors")]
            nnlm_adv: RefCell::new(None),
            mdltr: RefCell::new(None),
            wfea: RefCell::new(None),
            engdict_v: RefCell::new(None),
            emix_ana_v: RefCell::new(None),
            emix_pin_rd: RefCell::new(None),
            ckvd: RefCell::new(None),
            stt: RefCell::new(None),
            tone_tab: RefCell::new(None),
            bwcn_texts: RefCell::new(None),
        }
    }

    // ---- 基础查表 ----

    /// front-code 解码: 共享前缀增量项（阿坝 = 啊[:1]+坝）与全词项均还原;
    /// 结果进 wtext 缓存
    #[test]
    fn word_text_front_coded_and_cached() {
        let e = micro();
        assert_eq!(e.word_text(0), "啊");
        assert_eq!(e.word_text(1), "阿坝", "共享前缀增量项");
        assert_eq!(e.word_text(2), "吧");
        assert_eq!(e.word_text(3), "你好");
        assert_eq!(e.word_text(4), "世界");
        assert!(
            (0..5).all(|w| e.wtext.borrow().contains_key(&w)),
            "查过的词进 wtext 缓存"
        );
    }

    /// 词频层 logp: freq u16 走 quant 四段量化（47104+1024k → −k）
    #[test]
    fn word_logp_quantized_freq() {
        let e = micro();
        assert_eq!(e.word_logp(0), -1.0);
        assert_eq!(e.word_logp(1), -2.0);
        assert_eq!(e.word_logp(2), 0.0);
        assert_eq!(e.word_logp(3), -1.0);
        assert_eq!(e.word_logp(4), 0.0);
    }

    /// WLSI 槽位频次: v1（无伴列）回退词级; v2 伴列逐槽覆写 fv/logp
    #[test]
    fn slot_fv_override_and_fallback() {
        let mut e = micro();
        assert_eq!(e.slot_fv(0), None, "v1 无伴列");
        assert_eq!(e.slot_logp(0, 0), e.word_logp(0), "v1 槽级回退词级");
        assert_eq!(e.fv_of(0, 0), 48128, "v1 fv_of = 词频原值");
        // v2: 槽 0 的伴列频次覆写词级（47104 ≠ 词频 48128）
        e.wlsi_fv = Some(u16arr(&[47104, 48128, 49152, 48128, 47104]));
        assert_eq!(e.slot_fv(0), Some(47104));
        assert_eq!(e.slot_logp(0, 0), 0.0, "槽级 logp 用伴列值");
        assert_eq!(e.fv_of(0, 0), 47104);
    }

    /// 词哈希 v3（槽=id+1）: 全词面往返命中; 词典外词探到空槽止链 → None
    #[test]
    fn word_id_v3_roundtrip_and_miss() {
        let e = micro();
        for (wid, w) in WORDS.iter().enumerate() {
            assert_eq!(e.word_id(w), Some(wid as u32), "词 {w}");
        }
        assert_eq!(e.word_id("不在词典"), None);
        assert_eq!(e.word_id(""), None);
    }

    /// 词哈希 v1/v2（交织 [2j]=id+1/[2j+1]=fnv）: 同一构建律下往返命中
    #[test]
    fn word_id_v1_interleaved_roundtrip() {
        let mut e = micro();
        e.whash_v3 = false;
        e.whash_slots = whash_v1_slots(&WORDS, 8);
        for (wid, w) in WORDS.iter().enumerate() {
            assert_eq!(e.word_id(w), Some(wid as u32), "词 {w}");
        }
        assert_eq!(e.word_id("不在词典"), None);
    }

    // ---- 读音解码 ----

    /// RIDX syl blob 解码: 变体表结构逐 u16 LE 还原 + rid_memo 缓存落账
    #[test]
    fn rid_variants_decode_and_memo() {
        let e = micro();
        assert_eq!(e.rid_variants(0), vec![vec![0, 2], vec![3]]);
        assert_eq!(e.rid_variants(1), vec![vec![1]]);
        assert_eq!(e.rid_variants(2), Vec::<Vec<u32>>::new(), "nv=0 空");
        assert!(
            e.rid_memo.borrow().contains_key(&0),
            "解码结果进 rid_memo"
        );
    }

    /// 读音三视图: pretty 撇号连/音节串/学习键逗号连; 空 rid → 空
    #[test]
    fn rid_views_pretty_syls_key() {
        let e = micro();
        assert_eq!(e.rid_pretty(0), "a'ab");
        assert_eq!(e.rid_syls(0), vec!["a".to_string(), "ab".to_string()]);
        assert_eq!(e.rid_key_public(0), "a,ab");
        assert_eq!(e.rid_pretty(1), "b");
        assert_eq!(e.rid_pretty(2), "");
        assert_eq!(e.rid_syls(2), Vec::<String>::new());
        assert_eq!(e.rid_key_public(2), "");
    }

    /// extra_syl: 变体 0 的音节数差; 任一侧空变体 → 恒 1（不可比哨兵）
    #[test]
    fn extra_syl_variant_len_diff() {
        let e = micro();
        assert_eq!(e.extra_syl(0, 1), 1, "r0 两音节 − r1 一音节");
        assert_eq!(e.extra_syl(1, 0), -1);
        assert_eq!(e.extra_syl(2, 0), 1, "rid 无变体 → 1");
        assert_eq!(e.extra_syl(1, 2), 1, "base_rid 无变体 → 1");
    }

    // ---- L2 区间取词 ----

    /// 小区间路径（cnt ≤ 64）: 按频合并去重保插入序, 同词取最小 fv;
    /// fv 并列保留首读音, 伴列更小 fv 时归属迁移到次读音
    #[test]
    fn range_candidates_small_dedup_min_fv() {
        let mut e = micro();
        // 词频口径: r0/r1 都含 你好(3), fv 并列 → 保留首读音 r0
        let out = e.range_candidates(0, 2, 10);
        assert_eq!(
            out,
            vec![(0, 0, 48128), (3, 0, 48128), (2, 1, 47104)],
            "并列保留首读音"
        );
        // v2 伴列: 你好 在 r0 槽 fv=48128, 在 r1 槽 fv=47104 → 取 r1
        e.wlsi_fv = Some(u16arr(&[49152, 48128, 47104, 46080, 47104]));
        let out = e.range_candidates(0, 2, 10);
        assert_eq!(
            out,
            vec![(0, 0, 49152), (3, 1, 47104), (2, 1, 46080)],
            "最小 fv 归属迁移"
        );
    }

    /// 大区间路径（cnt > 64）: k 路归并按 fv 升序出 top-k;
    /// 上限 want = min(cap, HOT_FALLBACK_K+16)
    #[test]
    fn range_candidates_heap_topk() {
        let mut e = micro();
        let n = 100u32;
        e.ridx_n = n as usize;
        e.ridx_off = u32arr(&(0..=n).collect::<Vec<_>>());
        e.wlist = u32arr(&(0..n).collect::<Vec<_>>());
        e.soff = u32arr(&vec![0u32; (n + 1) as usize]);
        // 槽 r 伴列 fv = 900+r → 归并弹序即读音序
        e.wlsi_fv =
            Some(u16arr(&(0..n).map(|r| (900 + r) as u16).collect::<Vec<_>>()));
        let out = e.range_candidates(0, n, 5);
        assert_eq!(
            out,
            vec![(0, 0, 900), (1, 1, 901), (2, 2, 902), (3, 3, 903), (4, 4, 904)]
        );
        // want 上限: cap=50 → 16+16=32 条
        let out = e.range_candidates(0, n, 50);
        assert_eq!(out.len(), 32);
        assert_eq!(*out.first().unwrap(), (0, 0, 900));
        assert_eq!(*out.last().unwrap(), (31, 31, 931));
    }

    /// hot 表 bisect: 命中出 (词, −1, 词频), 段空出空表, 未命中 None;
    /// 边界含小于全程/大于全程
    #[test]
    fn hot_candidates_bisect_lookup() {
        let e = micro();
        assert_eq!(
            e.hot_candidates(10),
            Some(vec![(0, -1, 48128), (3, -1, 48128)])
        );
        assert_eq!(e.hot_candidates(20), Some(vec![(4, -1, 47104)]));
        assert_eq!(e.hot_candidates(30), Some(Vec::new()), "空段");
        assert_eq!(e.hot_candidates(5), None, "小于首状态");
        assert_eq!(e.hot_candidates(15), None);
        assert_eq!(e.hot_candidates(99), None, "大于末状态");
    }

    // ---- L1 切分 ----

    /// segment（Viterbi 包装）: 全切分取最优路径, 不可达 → None;
    /// 单音节路径（ab）胜过两段路径（a|b, 等权下 Σlog 更负）
    #[test]
    fn segment_viterbi_paths() {
        let e = micro();
        let (p, s) = e.segment("nihao").unwrap();
        assert_eq!(p, vec!["ni".to_string(), "hao".to_string()]);
        assert!((s - 2.0 * (1.0f64 / 12.0).ln()).abs() < 1e-12);
        let (p, _) = e.segment("abab").unwrap();
        assert_eq!(p, vec!["ab".to_string(), "ab".to_string()]);
        assert_eq!(e.segment("az"), None, "z 不可达");
    }

    /// greedy_bounds: 贪婪最长切分的边界位集（含 0）; 非音节字符断链
    #[test]
    fn greedy_bounds_break_on_non_syllable() {
        let e = micro();
        let b = e.greedy_bounds("nihao");
        assert_eq!(b, HashSet::from([0, 2, 5]));
        assert_eq!(e.greedy_bounds("abx"), HashSet::from([0, 2]), "x 断链");
        assert_eq!(e.greedy_bounds("abab"), HashSet::from([0, 2, 4]));
        assert_eq!(e.greedy_bounds(""), HashSet::from([0]));
    }

    // ---- 多词 DP ----

    /// dp_path 四分支: 整键精确 / 尾段前缀补全（−COV_PENALTY）/ 多词
    /// 路径回溯 / 无可达 → None; 空输入 → 空路径
    #[test]
    fn dp_path_exact_completion_and_none() {
        let e = micro();
        // "abcd" 整键: 词 啊(w0) logp −1.0
        assert_eq!(e.dp_path("abcd"), Some((vec![(0, 0)], -1.0, false)));
        // "abc" 非终端前缀: 补全 = 词频 − 覆盖罚 0.35
        assert_eq!(
            e.dp_path("abc"),
            Some((vec![(0, 0)], -1.0 - COV_PENALTY, true))
        );
        // "babcd" = b(你好,r1) + abcd(啊,r0) 两词 DP 路径
        assert_eq!(
            e.dp_path("babcd"),
            Some((vec![(3, 1), (0, 0)], -2.0, false))
        );
        assert_eq!(e.dp_path("z"), None, "键域外不可达");
        assert_eq!(e.dp_path(""), Some((Vec::new(), 0.0, false)));
        // CLI 探针在同一前缀 DAT 上的走查
        assert_eq!(e.dat_walk_public(b"abc"), 5);
        assert_eq!(e.dat_walk_public(b"abcd"), 6);
        assert_eq!(e.dat_walk_public(b"abce"), -1);
    }

    /// B2.6b 格 DP 真机锁定: rbjjxw(表缺键) → 日本经济新闻 是格 DP
    /// 实证 top1 产出（面板后注入 + 字段镜像原生句候选的端到端语义）。
    /// micro 央具音节覆盖不足以构造完整格径（DAT 键域手术会级联）——
    /// 真机路径是格径语义的唯一可行锁定点。
    #[test]
    #[ignore = "真机 wxq.bin（HANHUO_BIN / 默认路径）"]
    fn jianpin_sent_beam_real_rbjjxw() {
        let bin = std::env::var("HANHUO_BIN")
            .unwrap_or_else(|_| {
                format!("{}/tmp/hanhuo/wxq.bin",
                        std::env::var("HOME").unwrap_or_default())
            });
        let Ok(e) = Engine::open(std::path::Path::new(&bin)) else {
            return; // 环境无真机则跳过（语义见集成 eval v3）
        };
        let out = e.jianpin_sent_beam("rbjjxw", 8);
        assert!(
            out.iter().any(|c| c.word.contains("日本经济")),
            "格 DP 应产出 日本经济* 路径, got {:?}",
            out.iter().map(|c| c.word.clone()).take(3).collect::<Vec<_>>()
        );
        // 字段镜像律: Sentence kind + path_words 在场
        let c = out.iter().find(|c| c.word.contains("日本经济")).unwrap();
        assert_eq!(c.kind, crate::Kind::Sentence);
        assert!(c.path_words.is_some());
    }

    /// B2.6b 简拼格 DP 输入守卫（micro 央具音节覆盖不足以构造完整
    /// 路径——c/d 无音节; 完整格径测试待专用央具, 守卫语义先锁）
    #[test]
    fn jianpin_sent_beam_input_guards() {
        let e = micro();
        assert!(e.jianpin_sent_beam("ab", 8).is_empty(), "<3 键空手");
        assert!(e.jianpin_sent_beam(&"a".repeat(13), 8).is_empty(), ">12 键空手");
        assert!(e.jianpin_sent_beam("zzz", 8).is_empty(), "无音节字母空手");
    }

    // ---- c.3 词块感知组装 × ctx-NNLM（设计-上下文动态组词 §c.3）----

    /// 门律（免数据半边）: 无 ctx / 无 NNLM → 恒等不动（micro 无 NNLM,
    /// 双门各自可验）。忠实链零行为 = 旋钮默认 false 的结构性保证。
    #[test]
    fn ctx_jp_compose_gates_inert() {
        use super::super::boost::ctx_jp_compose;
        let e = micro();
        let mut out = e.query("abcd", None, false, 8, false, false, false);
        let cmp = |v: &[crate::Cand]| {
            v.iter()
                .map(|c| (c.word.clone(), c.score))
                .collect::<Vec<_>>()
        };
        let before = cmp(&out);
        // 门 2: 无 ctx
        ctx_jp_compose(&e, "abcd", None, &mut out);
        assert_eq!(cmp(&out), before, "无 ctx 恒等");
        // 门 3: 有 ctx 无 NNLM（micro 无模型）
        ctx_jp_compose(&e, "abcd", Some("上文"), &mut out);
        assert_eq!(cmp(&out), before, "无 NNLM 恒等");
    }

    /// c.3 真机探针 #2（词块级验收, 轻量 NNLM 档）: xmdbmh+ctx=想去看电影
    /// → 爆米花 词块面进 sentence 槽顶位（块级碾压 白马湖 族——统一
    /// NNLM 序的设计断言）。前缀级精确验收（想买点爆米花 rank1）需
    /// 高级版 NNLM 臂（§6 #2）——轻量模型 N(下面的)>N(想买点) 实测。
    #[test]
    #[ignore = "真机 wxq.bin（HANHUO_BIN / 默认路径）"]
    fn ctx_jp_compose_real_popcorn_block() {
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!("{}/tmp/hanhuo/wxq.bin",
                    std::env::var("HOME").unwrap_or_default())
        });
        let Ok(mut e) = Engine::open(std::path::Path::new(&bin)) else {
            return;
        };
        e.set_boost(crate::BoostCfg {
            ctx_jp_compose: true,
            ..crate::BoostCfg::default()
        });
        let out = e.query("xmdbmh", Some("想去看电影"), false, 12, false, false, false);
        let top_sent: Vec<&str> = out
            .iter()
            .filter(|c| c.kind == crate::Kind::Sentence)
            .map(|c| c.word.as_str())
            .collect();
        assert!(
            top_sent.first().is_some_and(|w| w.ends_with("爆米花")),
            "词块面应占 sentence 槽顶位, got {top_sent:?}"
        );
    }

    /// c.3 真机探针 #1（可达性验收）: ewzdn+ctx=天青色等烟雨 →
    /// 而我在等你 在库面（B2.6b 病理下 top60 永缺席——词块 而我|在等你
    /// 拼装可达性 + 无 ctx 零行为门）。
    #[test]
    #[ignore = "真机 wxq.bin（HANHUO_BIN / 默认路径）"]
    fn ctx_jp_compose_real_erwo_reachable() {
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!("{}/tmp/hanhuo/wxq.bin",
                    std::env::var("HOME").unwrap_or_default())
        });
        let Ok(mut e) = Engine::open(std::path::Path::new(&bin)) else {
            return;
        };
        // 无 ctx 零行为门: 旋钮开但无 ctx → 与旋钮关逐位一致
        let bare = e.query("ewzdn", None, false, 12, false, false, false);
        e.set_boost(crate::BoostCfg {
            ctx_jp_compose: true,
            ..crate::BoostCfg::default()
        });
        let noctx =
            e.query("ewzdn", None, false, 12, false, false, false);
        let cmp = |v: &[crate::Cand]| {
            v.iter()
                .map(|c| (c.word.clone(), c.score))
                .collect::<Vec<_>>()
        };
        assert_eq!(cmp(&bare), cmp(&noctx), "无 ctx 逐位一致（门律）");
        // 可达性: 而我在等你 进场（槽内或槽外追加）
        let out = e.query(
            "ewzdn",
            Some("天青色等烟雨"),
            false,
            12,
            false,
            false,
            false,
        );
        assert!(
            out.iter()
                .take(12)
                .any(|c| c.word.contains("而我在等你")),
            "而我在等你 应可达进场, got {:?}",
            out.iter().map(|c| c.word.clone()).take(8).collect::<Vec<_>>()
        );
    }

    // ---- B2.5v2 段格变体束 ----

    /// 段格束: 每段读音列 top-M 词组格 beam; 原生最优（列首拼接）过滤,
    /// 变体按 Σ slot_logp 降序; 单段输入无变体空间 → 空。
    /// micro 下 dp_path("babcd") = [你好(r1), 啊(r0)]:
    /// r1 列 = [你好, 吧], r0 列 = [啊, 你好]（lp: 吧 > 你好=啊）
    #[test]
    fn boost_tail_sweep_beam_variants() {
        use super::super::boost::sentence_tail_sweep;
        let e = micro();
        let out = sentence_tail_sweep(&e, "babcd", 3);
        let words: Vec<&str> = out.iter().map(|c| c.word.as_str()).collect();
        assert_eq!(words, vec!["吧啊", "吧你好", "你好你好"], "beam 降序+原生过滤");
        assert!(out.iter().all(|c| c.kind == crate::Kind::Sentence));
        // 分数单调不增
        for w in out.windows(2) {
            assert!(w[0].logp >= w[1].logp);
        }
        // n=1 截断: 仅最优变体（同分稳定序 → 列首组合次序）
        let one = sentence_tail_sweep(&e, "babcd", 1);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].word, "吧啊");
        // 单段输入（dp_path 单词）→ 无变体
        assert!(sentence_tail_sweep(&e, "abcd", 3).is_empty());
        // 键域外 → dp_path None → 空
        assert!(sentence_tail_sweep(&e, "z", 3).is_empty());
    }

    // ---- B8 全段变体束 ----

    /// B8 专用微型机: DAT 键 ab/abcd/b/cd（DFS 叶序 ab=0, abcd=1, b=2,
    /// cd=3）, 读音列 r0=[吧(0.0),你好(-1.0)] r1=[啊(-1.0)] r2=[世界
    /// (0.0)] r3=[阿坝(-2.0)]——"abcd" 双段格: [abcd](啊, Σ−1.0) vs
    /// [ab|cd](吧阿坝, Σ−2.0)（micro 只有 abcd/b 两键, 造不出段多样性）
    fn seg_dat() -> Dat {
        Dat {
            // slot: 1=ROOT(base2: a@3,b@4,c@5) 3="a"(base4: b@6)
            // 6="ab"终端(base4: c@7) 7="abc"(base4: d@8) 8="abcd"终端
            // 5="c"(base5: d@9) 9="cd"终端
            base: bitarr(&[0, 2, 0, 4, 0, 5, 4, 4, 0, 0], 4),
            check: bitarr(&[0, 0, 0, 1, 1, 1, 3, 6, 7, 5], 4),
            // 叶区间: ab 子树(0,2) abcd(1,1) b(2,1) cd(3,1)
            lo: bitarr(&[0, 0, 0, 0, 2, 3, 0, 1, 1, 3], 4),
            cnt16: u16arr(&[0, 4, 0, 2, 1, 1, 2, 1, 1, 1]),
            term: Box::leak(vec![0b0101_0000u8, 0b0000_0011u8].into_boxed_slice()), // 槽 4/6/8/9
            n_slots: 10,
            n_keys: 4,
            cnt_ovf: HashMap::new(),
        }
    }

    fn micro_seg() -> Engine {
        let mut e = micro();
        e.dat = seg_dat();
        e.ridx_n = 4;
        e.ridx_off = u32arr(&[0, 2, 3, 4, 5]);
        e.wlist = u32arr(&[2, 3, 0, 4, 1]);
        e
    }

    /// B8 段覆盖多样化: 双词路径（ab|cd 的 吧阿坝/你好阿坝）在 B2.5
    /// （格建在 [abcd] 单段上）不可达, B8 段格 #2 出场; 原生最优
    /// （段格 #1 列首 啊）过滤; Σ slot_logp 降序。
    #[test]
    fn b8_seg_variants_diversify_segmentation() {
        use super::super::boost::sentence_seg_variants;
        let e = micro_seg();
        let out = sentence_seg_variants(&e, "abcd", 6);
        let words: Vec<&str> = out.iter().map(|c| c.word.as_str()).collect();
        assert_eq!(
            words,
            vec!["吧阿坝", "你好阿坝"],
            "段格#2 变体降序; 段格#1 单槽无变体空间且列首=原生已过滤"
        );
        assert!(out.iter().all(|c| c.kind == crate::Kind::Sentence));
        assert!((out[0].logp - (-2.0)).abs() < 1e-9, "吧(0)+阿坝(-2)");
        assert!((out[1].logp - (-3.0)).abs() < 1e-9, "你好(-1)+阿坝(-2)");
        // n 截断: 预算 1 → 仅最优变体
        let one = sentence_seg_variants(&e, "abcd", 1);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].word, "吧阿坝");
        // 键域外 / 无全消耗切分（"abc" 仅补全支）→ 空（该域归 B2.5 段格）
        assert!(sentence_seg_variants(&e, "z", 6).is_empty());
        assert!(sentence_seg_variants(&e, "abc", 6).is_empty());
        // 单切分输入（micro: b|abcd 唯一切分）→ 段格#1=原生整格让位
        // B2.5（同格同 beam; B8 增量 = 非原生切分）→ 空
        let e2 = micro();
        assert!(
            sentence_seg_variants(&e2, "babcd", 6).is_empty(),
            "单切分域 B8 零产出（原生段格归 B2.5）"
        );
    }

    /// B8 轮转选拔: 段间结构多样性优先于频序——多段格时全局降序截断
    /// 会被单字段高频变体灌满（basheyuanbi 实证: 吧设原币族 压过
    /// 跋涉远比）, 轮转保证每段格出场。micro_seg 单一非原生段格退化为
    /// 段内降序。
    #[test]
    fn b8_seg_variants_round_robin_diversity() {
        use super::super::boost::sentence_seg_variants;
        let e = micro_seg();
        // micro_seg 段格#2 [ab|cd] 仅有 2 变体; n=1 只取段内最优
        let one = sentence_seg_variants(&e, "abcd", 1);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].word, "吧阿坝");
        assert!((one[0].logp - (-2.0)).abs() < 1e-9);
    }

    /// B8 真机锁定: basheyuanbi 的双词路径（跋涉 × 远比）在场——B2.5
    /// 格建在 ba|she|yuan|bi 单字段上, 任何列深/beam 不可达（截断层=
    /// 段覆盖）; B8 n-best 切分把 bashe|yuanbi 段格带入变体池。
    #[test]
    #[ignore = "真机 wxq.bin（HANHUO_BIN / 默认路径）"]
    fn b8_seg_variants_real_basheyuanbi() {
        use super::super::boost::sentence_seg_variants;
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!("{}/tmp/hanhuo/wxq.bin",
                    std::env::var("HOME").unwrap_or_default())
        });
        let Ok(e) = Engine::open(std::path::Path::new(&bin)) else {
            return;
        };
        let out = sentence_seg_variants(&e, "basheyuanbi", 6);
        assert!(
            out.iter().any(|c| c.word == "跋涉远比"),
            "双词路径应在变体池, got {:?}",
            out.iter().map(|c| c.word.clone()).collect::<Vec<_>>()
        );
    }

    /// B7 v2 组内原位重排语义: 句组成员只换位不越组（非 Sentence 候选
    /// 原位原分不动——v1 全局 sort 实证把句组压 rank18+ 的回归锁）,
    /// NNLM 降序 + 锚定递减; <2 句成员早退零扰动; cap=12 外保序。
    #[test]
    fn b7_sent_nnlm_rescore_inplace_group_semantics() {
        use super::super::boost::sentence_nnlm_rescore;
        let e = micro();
        let mbytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/nnlm_mobile_fp32.safetensors"
        ));
        let Ok(mb) = mbytes else {
            return; // 模型文件缺位（浅克隆）则跳过
        };
        let m = crate::Nnlm::from_safetensors(&mb).unwrap();
        e.set_nnlm(m);
        let mk = |w: &str, sc: f64| {
            let mut c = Cand::new(w.to_string(), sc, Kind::Sentence);
            c.score = sc;
            c.logp = sc;
            c
        };
        let mk_path = || {
            let mut c = Cand::new("吧".to_string(), -1.3, Kind::Path);
            c.score = -1.3;
            c
        };
        // 面板: 句组 3 成员（分序 吧社原币>吧设原币>吧射原币）+ 高分
        // path 候选殿后夹击——v1 全局 sort 会让 吧(-1.3) 浮顶
        let mut out = vec![
            mk("吧社原币", -4.6),
            mk("吧设原币", -11.0),
            mk("吧射原币", -11.1),
            mk_path(),
        ];
        sentence_nnlm_rescore(&e, &mut out);
        // 非句成员: 原位原分不动
        assert_eq!(out[3].word, "吧");
        assert!((out[3].score - (-1.3)).abs() < 1e-12, "path 原分不动");
        // 句组仍占位 0..3（原位换位, 无全局重排）
        let mut group: Vec<&str> =
            out[..3].iter().map(|c| c.word.as_str()).collect();
        group.sort_unstable();
        assert_eq!(
            group,
            vec!["吧射原币", "吧社原币", "吧设原币"],
            "组内成员集合不变"
        );
        // NNLM 期望序自算（与实现同源独立调用, 锁换位方向）
        let nn = e.nnlm.borrow();
        let nn = nn.as_ref().unwrap();
        let words = ["吧社原币", "吧设原币", "吧射原币"];
        let seqs: Vec<Vec<u32>> = words
            .iter()
            .map(|w| nn.encode(w).unwrap())
            .collect();
        let sc: Vec<f64> = nn
            .score_candidates(&[], &seqs)
            .into_iter()
            .map(|s| s as f64)
            .collect();
        let best = (0..3).max_by(|a, b| sc[*a].total_cmp(&sc[*b])).unwrap();
        assert_eq!(
            out[0].word, words[best],
            "NNLM 最优者换位到组首（纯 NNLM 序）"
        );
        // 锚定递减: 组首分 = 组内原分最大值（-4.6）, 组内单调不增
        assert!((out[0].score - (-4.6)).abs() < 1e-9);
        assert!(out[0].score >= out[1].score);
        assert!(out[1].score >= out[2].score);
        // <2 句成员 → 早退零扰动（B0 结构保持; v1 全局 sort 回归锁）
        let mut one = vec![mk("吧社原币", -4.6), mk_path()];
        sentence_nnlm_rescore(&e, &mut one);
        assert_eq!(one[0].word, "吧社原币");
        assert!((one[0].score - (-4.6)).abs() < 1e-12);
        assert_eq!(one[1].word, "吧");
        // cap=12: 第 13+ 句成员不参与打分且原位原分
        let mut big: Vec<Cand> = (0..15)
            .map(|i| mk(&format!("吧射原币{i}"), -11.0 - i as f64))
            .collect();
        big.push(mk_path());
        sentence_nnlm_rescore(&e, &mut big);
        assert_eq!(big[15].word, "吧", "cap 外非句成员不动");
        assert!(
            (big[14].score - (-11.0 - 14.0)).abs() < 1e-9,
            "第 15 名（cap 外）原分不动"
        );
    }

    // ---- 上下文锚点 ----

    /// ctx_ids: None/空串 → None; 整词命中直取; 长上下文取尾部成词
    /// 片段（你好世界→世界）; 全不成词 → wid None 原文透传
    #[test]
    fn ctx_ids_direct_hit_tail_fallback() {
        let e = micro();
        assert!(e.ctx_ids(None).is_none());
        assert!(e.ctx_ids(Some("")).is_none());
        let c = e.ctx_ids(Some("你好")).unwrap();
        assert_eq!(c.wid, Some(3));
        assert_eq!(c.text, "你好");
        let c = e.ctx_ids(Some("你好世界")).unwrap();
        assert_eq!(c.wid, Some(4), "尾部 世界 成词");
        assert_eq!(c.text, "世界");
        let c = e.ctx_ids(Some("不成词上下文")).unwrap();
        assert_eq!(c.wid, None);
        assert_eq!(c.text, "不成词上下文");
    }

    // ---- M3 用户词 ----

    /// user_commit 六门: 超长拒收 / 非词典词拒收 / 多音字拒收（你好 跨
    /// r0/r1）/ 无中文读音拒收（w2r=0）/ 空学习键拒收（rid 无变体）/
    /// 通过（学习键 = rid_key 逗号全串）
    /// B4 终位学习加成: 面板/重排之后的可靠落点——按 (reading→key, word)
    /// 重匹配; \u{1} 展示形豁免; 无用户词典零操作
    #[test]
    fn user_boost_final_rematches_terminal_panel() {
        // 时钟注入: anchor 实时衰减下, 测试词 ts 须与查询同刻
        let NOW = NOW_FOR_TESTS + 10_000; // user_commit 基准之后
        let e = micro();
        e.attach_user(&std::env::temp_dir()
            .join(format!("hanhuo_core_query_udf_{}.bin", std::process::id())));
        e.set_now(Some(NOW));
        assert!(e.user_commit("阿坝", Some(0), NOW + 0));
        assert!(e.user_commit("阿坝", Some(0), NOW + 0)); // 同刻: anchor=2.0 恰过补位门
        let boost = user_boost(e.user_entry("a,ab", "阿坝").unwrap().anchor);
        assert!((e.user_entry("a,ab", "阿坝").unwrap().anchor - 2.0).abs() < 1e-6);
        let mut lead = Cand::new("你好".to_string(), -1.0, Kind::Exact);
        lead.score = 1.5;
        lead.reading = "ni'hao".to_string();
        let mut lag = Cand::new("阿坝".to_string(), -2.0, Kind::Exact);
        lag.score = 1.0;
        lag.reading = "a'ab".to_string();
        let mut face = Cand::new("阿坝\u{1}的".to_string(), -2.0, Kind::Fbcol);
        face.score = 1.0;
        face.reading = "a'ab".to_string();
        // 中段补位形: Kind::User + 异读 reading + 通道低分（附录14 洗后残形）
        let mut u15 = Cand::new("阿坝".to_string(), -2.0, Kind::User);
        u15.score = -1.6;
        u15.reading = "long'li".to_string();
        let mut out = vec![lead, lag.clone(), face, u15];
        // micro DAT 终态键 = "abcd"（→rid 0, 组 key "a,ab"）
        e.user_boost_final("abcd", &mut out);
        // B4 双面孔双计修复（G 移交发现 1）: lag（reading 命中通道 0 加成,
        // 1.0+boost > 1.5 → 越位置顶）为**唯一** 阿坝 面承载信号; 中段
        // 补位形（异读 User 面）不再二次加成, 同词残留 User 面清出面板。
        assert_eq!(out[0].word, "阿坝");
        assert_eq!(out[0].kind, Kind::Exact, "reading 命中面单面胜出");
        assert_eq!(out[0].reading, "a'ab");
        assert!((out[0].score - 1.0 - boost).abs() < 1e-12);
        assert!((out[0].user_bias - boost).abs() < 1e-12);
        assert_eq!(
            out.iter().filter(|c| c.word == "阿坝").count(),
            1,
            "同词单面孔（User 残面已清出）"
        );
        let face_c = out.iter().find(|c| c.word.contains('\u{1}')).unwrap();
        assert_eq!(face_c.score, 1.0, "展示形豁免");
        // 无词典零操作
        let e2 = micro();
        let mut out2 = vec![lag];
        e2.user_boost_final("abcd", &mut out2);
        assert_eq!(out2[0].score, 1.0, "无用户词典零操作");
    }

    /// B5 组词/组句注入: 连续 commit 自动造短语 → 查询精确命中注入
    /// Kind::User; 首学 anchor=1 平分殿后（稳定排序原生先）, 复用越位 top1
    #[test]
    fn user_phrase_inject_via_terminal_panel() {
        // 时钟注入: anchor 实时衰减下, 测试词 ts 须与查询同刻
        let NOW = NOW_FOR_TESTS + 10_000; // user_commit 基准之后
        let e = micro();
        e.attach_user(&std::env::temp_dir()
            .join(format!("hanhuo_core_query_b5_{}.bin", std::process::id())));
        e.set_now(Some(NOW));
        // micro 词表可用集: 阿坝/世界 非多音（吧=设计内多音夹具被拒收）
        assert!(e.user_commit("阿坝", Some(0), NOW + 0));
        assert!(e.user_commit("世界", Some(0), NOW + 10)); // +10s 连续 → 短语
        let out = e.query("aabaab", None, false, 50, false, false, false);
        let ph = out.iter().find(|c| c.word == "阿坝世界");
        let ph = ph.expect("短语注入面板");
        assert_eq!(ph.kind, crate::Kind::User);
        assert_eq!(ph.reading, "a'ab'a'ab", "逗号形→撇号展示形");
        assert!(out[0].word != "阿坝世界", "首学 anchor=1 平分殿后");
        // 复用（第二轮）→ anchor≈2 → top1
        assert!(e.user_commit("阿坝", Some(0), NOW + 20));
        assert!(e.user_commit("世界", Some(0), NOW + 30));
        let out2 = e.query("aabaab", None, false, 50, false, false, false);
        assert_eq!(out2[0].word, "阿坝世界", "复用越位 top1");
    }

    /// B5-2 短语前缀补全: 半截输入命中已学长短语——首学沉半步（不越原生
    /// top1）, 复用越位; 单字母门槛拦倾倒
    #[test]
    fn user_phrase_prefix_completion() {
        // 时钟注入: anchor 实时衰减下, 测试词 ts 须与查询同刻
        let NOW = NOW_FOR_TESTS + 10_000; // user_commit 基准之后
        let e = micro();
        e.attach_user(&std::env::temp_dir()
            .join(format!("hanhuo_core_query_b5p_{}.bin", std::process::id())));
        e.set_now(Some(NOW));
        assert!(e.user_commit("阿坝", Some(0), NOW + 0));
        assert!(e.user_commit("世界", Some(0), NOW + 10)); // 短语 阿坝世界 key aabaab
        // 4B 前缀 aaba → 补全注入; 首学 anchor=1 → top−0.5 沉半步
        let out = e.query("aaba", None, false, 50, false, false, false);
        let ph = out.iter().find(|c| c.word == "阿坝世界");
        let ph = ph.expect("前缀补全注入");
        assert_eq!(ph.kind, crate::Kind::User);
        assert!(out[0].word != "阿坝世界", "首学沉半步不越原生 top1");
        // 复用 → anchor≈2 → top+boost−0.5 > top → 越位
        assert!(e.user_commit("阿坝", Some(0), NOW + 20));
        assert!(e.user_commit("世界", Some(0), NOW + 30));
        let out2 = e.query("aaba", None, false, 50, false, false, false);
        assert_eq!(out2[0].word, "阿坝世界", "复用前缀补全越位 top1");
        // 精确全拼仍走精确块（rank1 级）
        let out3 = e.query("aabaab", None, false, 50, false, false, false);
        assert!(out3.iter().any(|c| c.word == "阿坝世界"));
    }

    /// B5-3 选词回报: 面板词 → 自动 commit（fallback 路径——micro 无
    /// path_words 句选; 句选 path 序列由真机 pick 实证）
    #[test]
    fn user_feedback_commits_panel_word() {
        let e = micro();
        e.attach_user(&std::env::temp_dir()
            .join(format!("hanhuo_core_query_b5f_{}.bin", std::process::id())));
        // micro 词表: 阿坝 非多音（你好=跨读音多音夹具被拒收）
        assert!(e.feedback("aab", "阿坝", 1000));
        assert!(e.user_entry("a,ab", "阿坝").is_some(), "回报→词级 commit");
        // 面板外非词典词 → 拒收门透传
        assert!(!e.feedback("aab", "不存在词", 1000));
    }

    #[test]
    fn user_commit_gates() {
        let e = micro();
        e.attach_user(&std::env::temp_dir()
            .join(format!("hanhuo_core_query_ud_{}.bin", std::process::id())));
        assert!(!e.user_commit(&"超".repeat(25), None, 1), "超 24B 拒收");
        assert!(!e.user_commit("不存在", None, 1), "非词典词拒收");
        assert!(!e.user_commit("你好", None, 1), "多音字拒收");
        assert!(!e.user_commit("世界", None, 1), "无读音（w2r=0）拒收");
        assert!(
            !e.user_commit("啊", Some(2), 1),
            "rid_hint 指空变体读音 → 空键拒收"
        );
        assert!(e.user_commit("阿坝", Some(0), 1000), "通过");
        let ent = e.user_entry("a,ab", "阿坝").unwrap();
        assert_eq!(ent.count, 1);
    }

    /// user_boost_merge: 在板命中词并入 score_bias; 缺席词以 Kind::User
    /// 补位（mk 走 w2r 主读音 → reading/词面/词频齐备）
    #[test]
    fn user_boost_merge_bias_and_appends() {
        let NOW = NOW_FOR_TESTS + 10_000;
        let e = micro();
        e.attach_user(&std::env::temp_dir()
            .join(format!("hanhuo_core_query_ud2_{}.bin", std::process::id())));
        e.set_now(Some(NOW));
        // 两次 commit 抬 anchor > 1（首次 anchor=1.0 → boost 恒 0）
        assert!(e.user_commit("阿坝", Some(0), NOW + 0));
        assert!(e.user_commit("阿坝", Some(0), NOW + 5));
        let ent = e.user_entry("a,ab", "阿坝").unwrap();
        assert_eq!(ent.count, 2);
        let boost = user_boost(ent.anchor);
        assert!(boost > 0.0, "anchor {} → boost {boost}", ent.anchor);
        // 在板: 命中词加成, 不补位
        let mut c = Cand::new("阿坝".to_string(), -2.0, Kind::Exact);
        c.wid = 1;
        let mut out = vec![c];
        e.user_boost_merge(0, &mut out);
        assert_eq!(out.len(), 1);
        // B4 信号走专用 user_bias 槽（score_bias 是面板位次合成分领域,
        // 面板层会整体覆写——学习信号混入即被洗）
        assert!((out[0].user_bias - boost).abs() < 1e-12);
        assert_eq!(out[0].score_bias, 0.0);
        // 缺席: 补 Kind::User 候选
        let mut c2 = Cand::new("吧".to_string(), 0.0, Kind::Exact);
        c2.wid = 2;
        let mut out2 = vec![c2];
        e.user_boost_merge(0, &mut out2);
        assert_eq!(out2.len(), 2);
        assert_eq!(out2[1].kind, Kind::User);
        assert_eq!(out2[1].word, "阿坝");
        assert_eq!(out2[1].wid, 1);
        assert_eq!(out2[1].reading, "a'ab", "mk 主读音");
        // 中段补位不预置 user_bias（面板钉分覆写下永不生效）——终位补加
        assert_eq!(out2[1].user_bias, 0.0);
    }

    // ---- 可选资源与探针 ----

    /// 未挂载的可选资源全部惰性: stt/ckvd 缺位透传, 挂载坏路径 → false,
    /// MDLT/WFEA 段缺位 → 探针 None
    #[test]
    fn unmounted_optionals_are_inert() {
        let e = micro();
        assert_eq!(e.to_traditional("你好世界"), "你好世界");
        assert!(e.ckvd_lookup("nihao").is_empty());
        assert!(!e.attach_ckvd(Path::new("/nonexistent-hanhuo/ckvd.bin")));
        assert!(!e.attach_stt(Path::new("/nonexistent-hanhuo/stt.bin")));
        assert_eq!(e.mdltr_score(&[0f32; N_FEAT]), None);
        assert_eq!(e.word_feat11("你好"), None);
    }

    /// 多词库聚合语义: 跨库词面去重（首库优先, 非高分覆盖）+ weight 降序;
    /// 未挂键空返（bwcn_lookup 层, 2026-11-08 多词库轮）
    #[test]
    fn bwcn_multi_dict_aggregation_semantics() {
        let e = micro();
        let dir = std::env::temp_dir().join("hanhuo_engine_bwcn_multi");
        std::fs::create_dir_all(&dir).ok();
        let d1 = dir.join("d1.tsv");
        let d2 = dir.join("d2.tsv");
        // d1 先挂: 妮豪(500) 与 d2 同词面(900) → 首库副本胜
        std::fs::write(
            &d1,
            "word\treading\tfreq\n妮豪\tni,hao\t500\n泥耗\tni,hao\t300\n",
        )
        .unwrap();
        std::fs::write(
            &d2,
            "word\treading\tfreq\n妮豪\tni,hao\t900\n拟好\tni,hao\t700\n",
        )
        .unwrap();
        assert!(e.attach_bwcn(&d1));
        assert!(e.attach_bwcn(&d2));
        let out = e.bwcn_lookup("nihao");
        let texts: Vec<&str> = out.iter().map(|p| p.text.as_str()).collect();
        // 聚合后 weight 降序: 拟好700 > 妮豪500(首库副本) > 泥耗300
        assert_eq!(texts, vec!["拟好", "妮豪", "泥耗"]);
        assert_eq!(
            out.iter().find(|p| p.text == "妮豪").unwrap().weight,
            500,
            "同词面首库优先（d2 的 900 分副本被去重）"
        );
        assert!(e.bwcn_lookup("ab").is_empty(), "未挂键空返");
    }

    /// 引擎级注入接线: 只挂不开旋钮=死挂（注入词不可见）; 开 bwcn_inject 后
    /// 外挂词进面板且原生序零扰动（2026-11-08 --dict 死挂事故的接缝锁）
    #[test]
    fn query_bwcn_inject_pipeline_wiring() {
        let mut e = micro();
        let dir = std::env::temp_dir().join("hanhuo_engine_bwcn_pipe");
        std::fs::create_dir_all(&dir).ok();
        let d = dir.join("d.tsv");
        std::fs::write(&d, "word\treading\tfreq\n妮豪\tni,hao\t500\n").unwrap();
        assert!(e.attach_bwcn(&d));
        let out0 = e.query("nihao", None, false, 50, false, false, false);
        assert!(
            out0.iter().all(|c| c.word != "妮豪"),
            "bwcn_inject 关: 挂载不生效（死挂语义）"
        );
        e.set_boost(crate::BoostCfg {
            sentence_first_min_len: 8,
            lmcn_chain: false,
            bwcn_inject: true,
            ckvd_inject: false,
            sentence_tail_sweep: 0,
            nnlm_order: false,
            jianpin_dat: false,
            inj_score_merge: false,
            dict_recal: false,
            jianpin_inject: false,
            compose_inject: false, sentence_nnlm_rescore: false,
            sentence_seg_beam: 0,
            ctx_jp_compose: false, inj_top_promote: false,
            mixed_input: false,
                bwcn_sent: false,
                tone_filter: false,
        });
        let out1 = e.query("nihao", None, false, 50, false, false, false);
        assert!(
            out1.iter().any(|c| c.word == "妮豪"),
            "开旋钮: 外挂词注入面板"
        );
        // 只增不改精确锁: 短输入（<8B）走尾部追加 → 原生序为 out1 前缀,
        // 注入词殿后（micro 引擎 nihao 原生面板首项是 你, 非整词 你好）
        let n = out0.len();
        assert_eq!(out1.len(), n + 1, "恰增一条");
        let w0: Vec<&str> = out0.iter().map(|c| c.word.as_str()).collect();
        let w1: Vec<&str> = out1.iter().map(|c| c.word.as_str()).collect();
        assert_eq!(&w1[..n], &w0[..], "原生序零扰动（只增不改）");
        assert_eq!(w1[n], "妮豪", "注入词殿后");
    }

    // ---- B6 表外组词 ----

    /// B6 门语义（micro 数据无关）: 非「恰两规整音节」输入一律空手。
    /// micro DAT 只有 ab/abcd 两键（ni/hao 音节无读音列）——恰好锁住
    /// 「两音节但无字列」的空列门。
    #[test]
    fn b6_compose_pair_gates() {
        use super::super::boost::compose_pair_hits;
        let e = micro();
        assert!(compose_pair_hits(&e, "a", &[]).is_empty(), "单音节");
        assert!(compose_pair_hits(&e, "ab", &[]).is_empty(), "单音节(双字母)");
        assert!(
            compose_pair_hits(&e, "nihao", &[]).is_empty(),
            "两音节但读音列缺席（micro DAT 无 ni/hao 键）"
        );
        assert!(
            compose_pair_hits(&e, "nihaoshi", &[]).is_empty(),
            "三音节"
        );
    }

    /// B6 真机集成: 词典无词条对（峰澈）注入在场 + 原生序零扰动 +
    /// 注入块钉分低于原生 min（只增不改语义锁）。
    #[test]
    #[ignore = "真机 wxq.bin（HANHUO_BIN / 默认路径）"]
    fn b6_compose_inject_presence_and_zero_perturbation() {
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!("{}/tmp/hanhuo/wxq.bin",
                    std::env::var("HOME").unwrap_or_default())
        });
        let Ok(mut e) = Engine::open(std::path::Path::new(&bin)) else {
            return; // 环境无真机则跳过
        };
        // 忠实: 峰澈 缺席（表外对, top-60 任何通道不发射）
        let faithful = e.query("fengche", None, false, 60, false, false, false);
        assert!(!faithful.iter().any(|c| c.word == "峰澈"));
        // 开旋钮: 在场 + kind/reading 语义
        e.set_boost(crate::BoostCfg {
            compose_inject: true,
            ..crate::BoostCfg::default()
        });
        let out = e.query("fengche", None, false, 60, false, false, false);
        let hit = out
            .iter()
            .find(|c| c.word == "峰澈")
            .expect("B6 应组出 峰澈");
        assert_eq!(hit.kind, crate::Kind::Fbcol);
        assert_eq!(hit.reading, "feng'che");
        // 只增不改: 原生前缀逐位一致, 注入块整体钉分低于原生 min
        let n = faithful.len();
        assert!(out.len() > n, "有注入");
        for i in 0..n {
            assert_eq!(out[i].word, faithful[i].word, "原生序零扰动 @{i}");
        }
        let native_min =
            faithful.iter().map(|c| c.score).fold(f64::INFINITY, f64::min);
        assert!(
            out[n..].iter().all(|c| c.score < native_min),
            "注入块钉分低于面板 min"
        );
        // 注入块内部频积降序（稳定深页序）
        for w in out[n..].windows(2) {
            assert!(w[0].score >= w[1].score);
        }
        // 多音节输入不受扰: 长查询两配置逐位一致
        e.set_boost(crate::BoostCfg::default());
        let long_a = e.query("jintiantianqibucuo", None, false, 30, false, false, false);
        e.set_boost(crate::BoostCfg {
            compose_inject: true,
            ..crate::BoostCfg::default()
        });
        let long_b = e.query("jintiantianqibucuo", None, false, 30, false, false, false);
        assert_eq!(long_a.len(), long_b.len(), "非双音节域零注入");
        for (a, b) in long_a.iter().zip(long_b.iter()) {
            assert_eq!(a.word, b.word);
        }
    }

    /// 英文直通门: ≥3 键纯字母 × 无拼音音节前缀（shi 排斥 shill）×
    /// 词表命中; 词表空集 → 律休眠
    #[test]
    fn latin_direct_engdict_gate() {
        let e = micro();
        let dict: HashSet<String> =
            ["llama", "hello", "shill"].iter().map(|s| s.to_string()).collect();
        *e.engdict_v.borrow_mut() = Some(dict);
        assert!(e.latin_direct("ll").is_none(), "<3 键");
        let c = e.latin_direct("llama").unwrap();
        assert_eq!(c.word, "llama");
        assert_eq!(c.kind, Kind::Sentence);
        assert_eq!(c.wid, -1);
        assert_eq!(c.reading, "llama");
        assert!(e.latin_direct("hello").is_some());
        assert!(
            e.latin_direct("shill").is_none(),
            "shi 前缀成音节 → 拼音域排斥"
        );
        assert!(e.latin_direct("zzz").is_none(), "词表未收");
        // 空词表 = 律休眠（py `return ws or None`）
        *e.engdict_v.borrow_mut() = Some(HashSet::new());
        assert!(e.latin_direct("llama").is_none());
    }
}

#[cfg(test)]
/// 模糊硬度: 任意字节串进 query() 永不 panic。历史前科: iter_terminal
/// 非 a-z 下溢、NNLM 空 batch——输入域全量护栏（真机口径）。
mod fuzz_probe_tests {
    use super::*;

    #[test]
    #[ignore = "真机 wxq.bin"]
    fn query_never_panics_arbitrary_inputs() {
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!(
                "{}/tmp/hanhuo/wxq.bin",
                std::env::var("HOME").unwrap_or_default()
            )
        });
        let Ok(e) = Engine::open(std::path::Path::new(&bin)) else {
            return;
        };
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        let alphabet: &[u8] =
            b"abcdefghijklmnopqrstuvwxyz'{}[]\x00\xff";
        for i in 0..3000 {
            let len = (next() % 24) as usize;
            let s: String = (0..len)
                .map(|_| {
                    if next() % 8 == 0 {
                        char::from_u32((next() % 0x10000) as u32).unwrap_or('?')
                    } else {
                        alphabet[(next() as usize) % alphabet.len()] as char
                    }
                })
                .collect();
            let jianpin = next() % 3 == 0;
            let _ = e.query(
                &s,
                None,
                jianpin,
                12,
                i % 2 == 0,
                i % 3 == 0,
                i % 5 == 0,
            );
        }
    }

    /// 决定论门: 同引擎两次查询与双引擎同查询均逐位一致——抓
    /// HashMap 迭代序泄漏/RefCell 跨查询状态污染/mmap 地址依赖
    #[test]
    #[ignore = "真机 wxq.bin"]
    fn query_determinism_same_and_cross_engine() {
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!(
                "{}/tmp/hanhuo/wxq_dict.bin",
                std::env::var("HOME").unwrap_or_default()
            )
        });
        let queries = [
            "nihao", "shuzu", "zhonghuarenmingongheguo", "xinjigengsi",
            "suopeiguanxi", "ceshikaifa", "yingyongchengxu", "wenzuantuodun",
            "rbjjxw", "ceshi",
        ];
        let snap = |e: &Engine, q: &str| -> Vec<(String, f64, String)> {
            e.query(q, None, false, 50, true, true, true)
                .into_iter()
                .map(|c| (c.word.clone(), c.score, c.reading.clone()))
                .collect()
        };
        let Ok(e1) = Engine::open(std::path::Path::new(&bin)) else {
            eprintln!("跳过: 无 {bin}");
            return;
        };
        for q in queries {
            let a = snap(&e1, q);
            let b = snap(&e1, q);
            assert_eq!(a, b, "同引擎两次查询不一致: {q}");
        }
        let Ok(e2) = Engine::open(std::path::Path::new(&bin)) else {
            return;
        };
        for q in queries {
            let a = snap(&e1, q);
            let c = snap(&e2, q);
            assert_eq!(a, c, "双引擎同查询不一致: {q}");
        }
    }
}