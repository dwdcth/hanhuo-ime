//! M3 用户词学习模块——微信用户词学习算法（轮 XIX 实证）的忠实移植.
//!
//! 三规则（userdict_sim.py / cascade/userdict_learning.md）:
//! 1. 晋升: 词在词典内 + 读音唯一（多音字拒收）+ 词长 ≤24B；key = 拼音逗号全串
//! 2. 调频: count += 1/事件（无 burst 去重、无饱和）
//! 3. 衰减: anchor = anchor×exp(−Δt/86400) + 1  [f32 逐运算舍入]；τ=1 天，无 clamp
//!    count 不衰减；ts = 最近事件时间
//!
//! 存储（自有格式 HHD1，学其"命令队列+快照"思想，不用 LevelDB）:
//!   [magic|ver|rsv][checkpoint: n_groups × {key, entries}][journal: events → EOF]
//!   commit = 内存应用 + 追加 event；journal 超 4096 条 → 原子重写快照
//!   尾部截断容忍（崩溃安全：读到无效处止）
//!
//! 排名注（sim 实证）: 同 key 候选按 anchor 排；anchor≈3 无可见提升、≥5 超基座高频词。
//! user_boost 为自有设计（对齐该观测）: 1.9·ln(anchor)，封顶 6.0。
//! 系数标定（2026-11-08 会话实证重标）: 面板钉序下 rank4 词基座差 ≈3.0
//! （pending +8.0 vs exact +5.0）, 「anchor≥5 超基座」要求 k·ln5≥3.0
//! → k≥1.86; 旧 1.2/帽 2.5 使 anchor=10（burst 十连）仍差 0.24 拉不动。
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAGIC: u32 = 0x3144_4848; // 'HHD1' LE
/// v2: + phrases 段 + tail 段 + journal 打标签事件（读兼容 v1）
pub const VERSION: u32 = 2;
pub const DAY: f64 = 86400.0;
pub const MAX_WORD_BYTES: usize = 24;
const FLUSH_THRESHOLD: usize = 4096;
/// B5 会话窗: 相邻 commit 间隔 ≤ 此值（秒）视为连续输入
pub const SESSION_GAP: u64 = 30;
/// B5 n-gram 帽: 组词 2-3 词 / 组句 4-6 词
pub const MAX_PHRASE_WORDS: usize = 6;
/// B5 音节帽: 超长整句不入库（句 DP 已覆盖, 只学可复用块）
pub const MAX_PHRASE_SYL: usize = 24;
/// B5-3 短语表容量帽默认值（动态可配: HHD1 头 rsv 槽持久化, 0=默认;
/// set_max_phrases 配置, 超帽按 anchor 降序保留——anchor 恒 ≥1 无自然
/// 死亡, 与词表三规则不同源——防长期无界增长）
pub const MAX_PHRASE_ENTRIES: usize = 5000;
/// 容量帽配置界（下界防误配零值, 上界防 rsv 溢出语义混乱）
pub const MAX_PHRASES_MIN: usize = 1;
pub const MAX_PHRASES_MAX: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub count: u32,
    pub anchor: f32,
    pub ts: u64,
}

impl Entry {
    fn new(ts: u64) -> Entry {
        Entry {
            count: 1,
            anchor: 1.0,
            ts,
        }
    }

    /// 衰减+调频（f32 逐运算，与引擎逐位一致）
    pub fn on_event(&mut self, ts: u64) {
        let dt = ts.saturating_sub(self.ts) as f64;
        // 指数衰减（wine 时钟注入直证: 12h→anchor 1.606517、25h→1.352854，
        // 与 exp(−Δt/86400) 差 ≤1.5e-5；线性 1−Δt/86400 仅小 Δt 一阶近似）
        let f = (-dt / DAY).exp() as f32;
        self.anchor = self.anchor * f + 1.0f32;
        self.count += 1;
        self.ts = ts;
    }
}

/// 用户词学习评分加成（自有设计，标定自 sim 实证: anchor=5 超基座高频词）
/// 1.9·ln(anchor) 封顶 6.0（≈anchor 23; anchor 由日衰减自然有界——
/// burst 上界=连击次数, daily 收敛 1.58 → 加成 0.885 温和）
pub fn user_boost(anchor: f32) -> f64 {
    let a = if anchor > 1.0 { anchor as f64 } else { 1.0 };
    (1.9 * a.ln()).min(6.0)
}

/// anchor 实时衰减（backlog 学习子系统; 附录 15 行为差异对齐原生）:
/// 查询时有效锚 = anchor × exp(−Δt/1d)（M3 衰减同律前移到查询点——
/// 原实现惰性冻结, 久不用词 boost 保持虚高; 原生 user_boost_native
/// 即按 now−ts 折算）。Δt=0 → anchor 原值; 无限久 → 0（boost 归零,
/// 词仍在库可复活——下次事件 anchor×exp(−Δt)+1 重爬）。
pub fn effective_anchor(anchor: f32, ts: u64, now: u64) -> f32 {
    let dt = now.saturating_sub(ts) as f64;
    (anchor as f64 * (-dt / DAY).exp()) as f32
}

/// user_boost 的查询时变体（实时衰减后打分）
pub fn user_boost_at(anchor: f32, ts: u64, now: u64) -> f64 {
    user_boost(effective_anchor(anchor, ts, now))
}

/// 读音逗号串 → 首字母投影（B5-c）。wxq 音节表系统性拆存复合韵母族
/// （ian 族实测 2026-11-09: 天=ti,an 连=li,an 面=mi,an 前/钱=qi,an
/// 点=di,an——逐字段投影会把这些高频字放大成双声母）; 按连续性合并:
/// 元音开头字段（a/e/o 起）并入前一音节当且仅当并后成合法音节
/// （di+an→dian ✓ 并一声母 d; 毛阿 mao,a 并后 maoa 非法 → a 自立 ✓）。
/// 残余歧义（嗯衣 vs 你 类三字段）归并优先——词内拆分存储远多于
/// 跨词元音字连排。zh/ch/sh 天然投影 z/c/s（首字母律）。
pub fn project_initials(reading: &str, is_syl: &dyn Fn(&str) -> bool) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    for f in reading.split(',').filter(|s| !s.is_empty()) {
        let vowel_lead =
            matches!(f.as_bytes().first(), Some(b'a') | Some(b'e') | Some(b'o'));
        if !cur.is_empty() && vowel_lead {
            let merged = format!("{cur}{f}");
            if is_syl(&merged) {
                cur = merged;
                continue;
            }
        }
        out.push(f.chars().next().unwrap());
        cur = f.to_string();
    }
    out
}

/// 原生用户词加成曲线（libwxhld user_dict_lookup.h sub_18035EC80 1:1 复刻，
/// 2026-10-26 深夜战役）。本地引擎该路径惰性（云合并器门控——活体实证
/// anchor=4.9999 排名零变化）；服务器路径经 cand+844→registry"16"→mdltr
/// 槽19/20 生效（obs_006/007 anchor≥5 超基座）。
/// 公式：Δ=clamp(now_ms−ts,1,2097152)；e=exp(−Δ/86400ms)（τ=86.4s）；
/// v=e×anchor；base=curve(v) 分段线性（X=0.9/2.8/3.0 上界 ln 压缩×0.05，
/// Y=−1.1439/−3.1439/−7.1439）；lr=ln(min(count,500))/ln(500)；
/// v≥0.9 → +0.01/Δ；Δ≥604800ms → −0.8×(base−curve(86400))×lr；
/// with_penalty → −exp(−0.7×(count−1))（查表路 a4=1 专用）。
pub fn user_boost_native(count: u32, dt_ms: u64, anchor: f32, with_penalty: bool) -> f32 {
    const X1: f32 = 0.9;
    const X2: f32 = 2.8;
    const X11: f32 = 3.0;
    const Y1: f32 = -1.1439;
    const Y2: f32 = -3.1439;
    const Y3: f32 = -7.1439;
    fn curve(v: f32) -> f32 {
        if v <= X1 {
            return Y3;
        }
        let x = if v > X2 { v.ln() * 0.05 + X2 } else { v };
        (x - X1) / (X11 - X1) * (Y1 - Y2) + Y2
    }
    let d = (dt_ms as f32).clamp(1.0, 2_097_152.0);
    let e = (-d / 86400.0f32).exp();
    let v = e * anchor;
    let base = curve(v);
    let lr = (count.min(500) as f32).ln() / 500f32.ln();
    let term = if v >= X1 {
        0.01 / d
    } else if d >= 604_800.0 {
        -0.8 * (base - curve(86400.0)) * lr
    } else {
        0.0
    };
    let pen = if with_penalty {
        -(-0.7 * (count as f32 - 1.0)).exp()
    } else {
        0.0
    };
    base + term + pen
}

pub struct UserDict {
    path: PathBuf,
    /// py key → word → entry（同 key 多词各占一条，如 ni,hao → {你好, 泥嚎}）
    groups: HashMap<String, HashMap<String, Entry>>,
    /// B5 组词/组句: 无分隔全拼键 → 拼接文本 → 短语条目（三规则同词表）
    phrases: HashMap<String, HashMap<String, PhraseEntry>>,
    /// 会话滚动尾: 相邻间隔 ≤ SESSION_GAP 的连续 commit 序列
    /// （跨进程持久——CLI 每命令一进程, 运行态内存活不过下一次 commit）
    tail: Vec<TailWord>,
    journal_len: usize,
    /// 文件版本（1 旧库; 写路径一律 v2）。v1 追加 tagged 事件会错位——
    /// commit 前强制 flush 升版
    ver: u32,
    /// B5-4 短语容量帽（动态配置, HHD1 头 rsv 槽持久化; 0=默认 5000）
    max_phrases: usize,
}

/// 短语条目: 全读逗号形随行（journal/checkpoint/展示共用）
#[derive(Clone, Debug)]
pub struct PhraseEntry {
    pub reading: String, // 逗号形全读, 如 "ni,hao,shi,jie"
    pub e: Entry,
}

#[derive(Clone, Debug)]
struct TailWord {
    word: String,
    key: String, // 逗号形读音键
    ts: u64,
}

fn le32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

fn le16(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}

impl UserDict {
    /// 加载 checkpoint + 重放 journal（尾截断容忍）
    pub fn load(path: &Path) -> UserDict {
        let ud = Self::empty(path.to_path_buf());
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => return ud, // 不存在 → 空库
        };
        Self::ingest(ud, &data)
    }

    /// 从内存字节装载（wasm/JS 链路；坏头/截断容忍语义同 load，无 fs
    /// ——落盘改走 save_bytes 由宿主持久化，如 localStorage）
    pub fn from_bytes(bytes: &[u8]) -> UserDict {
        Self::ingest(Self::empty(PathBuf::new()), bytes)
    }

    fn empty(path: PathBuf) -> UserDict {
        UserDict {
            path,
            groups: HashMap::new(),
            phrases: HashMap::new(),
            tail: Vec::new(),
            journal_len: 0,
            ver: 2,
            max_phrases: MAX_PHRASE_ENTRIES,
        }
    }

    /// checkpoint + journal 解析（坏头/截断 → 已解析部分返回，宁丢勿炸）
    fn ingest(mut ud: UserDict, data: &[u8]) -> UserDict {
        if data.len() < 12 || le32(&data, 0) != MAGIC {
            return ud; // 坏头 → 空库（宁丢勿炸）
        }
        let ver = le32(&data, 4);
        if ver != 1 && ver != 2 {
            return ud;
        }
        ud.ver = ver;
        // rsv 槽 = 容量帽（0=默认; v1 库 rsv 未写恒 0 → 默认）
        let cap = le32(&data, 8) as usize;
        if cap > 0 {
            ud.max_phrases = cap.clamp(MAX_PHRASES_MIN, MAX_PHRASES_MAX);
        }
        let mut p = 12usize;
        let n_groups = le32(&data, p) as usize;
        p += 4;
        for _ in 0..n_groups {
            let Some(key) = read_str(&data, &mut p) else { return ud };
            let nent = *data.get(p).unwrap_or(&0) as usize;
            p += 1;
            let g = ud.groups.entry(key).or_default();
            for _ in 0..nent {
                let Some(word) = read_str(&data, &mut p) else { return ud };
                let (count, anchor, ts) = match read_entry(&data, &mut p) {
                    Some(v) => v,
                    None => return ud,
                };
                g.insert(word, Entry { count, anchor, ts });
            }
        }
        if ver == 2 {
            // B5 短语段
            let n_ph = le32(&data, p) as usize;
            p += 4;
            for _ in 0..n_ph {
                let Some(pk) = read_str(&data, &mut p) else { return ud };
                let nent = *data.get(p).unwrap_or(&0) as usize;
                p += 1;
                let g = ud.phrases.entry(pk).or_default();
                for _ in 0..nent {
                    let Some(text) = read_str(&data, &mut p) else { return ud };
                    let Some(reading) = read_str(&data, &mut p) else { return ud };
                    let (count, anchor, ts) = match read_entry(&data, &mut p) {
                        Some(v) => v,
                        None => return ud,
                    };
                    g.insert(
                        text,
                        PhraseEntry {
                            reading,
                            e: Entry { count, anchor, ts },
                        },
                    );
                }
            }
            // B5 会话尾段
            let n_tail = le32(&data, p) as usize;
            p += 4;
            for _ in 0..n_tail {
                let Some(word) = read_str(&data, &mut p) else { return ud };
                let Some(key) = read_str(&data, &mut p) else { return ud };
                if p + 8 > data.len() {
                    return ud;
                }
                let ts = u64::from_le_bytes([
                    data[p], data[p + 1], data[p + 2], data[p + 3],
                    data[p + 4], data[p + 5], data[p + 6], data[p + 7],
                ]);
                p += 8;
                ud.tail.push(TailWord { word, key, ts });
            }
        }
        // journal 到 EOF（v1: 无标签词事件; v2: 打标签）
        while p + 4 <= data.len() {
            if ver == 2 {
                match *data.get(p).unwrap_or(&9) {
                    0 => {
                        let Some((ts, key, word)) =
                            read_event(&data, &mut p, true)
                        else {
                            break;
                        };
                        ud.apply(&key, &word, ts);
                        ud.tail_push(&key, &word, ts, false);
                    }
                    1 => {
                        let Some((ts, pk, text, reading)) =
                            read_phrase_event(&data, &mut p)
                        else {
                            break;
                        };
                        ud.apply_phrase(&pk, &text, &reading, ts);
                    }
                    _ => break, // 未知标签 → 尾部当截断
                }
            } else {
                let Some((ts, key, word)) = read_event(&data, &mut p, false)
                else {
                    break; // 截断尾——事件不完整, p 停在 ev_start 前的语义已无读者
                };
                ud.apply(&key, &word, ts);
                ud.tail_push(&key, &word, ts, false);
            }
            ud.journal_len += 1;
        }
        ud
    }

    fn apply(&mut self, key: &str, word: &str, ts: u64) {
        let g = self.groups.entry(key.to_string()).or_default();
        match g.get_mut(word) {
            Some(e) => e.on_event(ts),
            None => {
                g.insert(word.to_string(), Entry::new(ts));
            }
        }
    }

    /// 学习事件：内存应用 + 追加 journal；超阈值原子重写。
    /// B5: 连续 commit 自动造短语（n-gram 2..=6）随事件落盘。
    pub fn commit(&mut self, key: &str, word: &str, ts: u64) {
        // 敏感词全管线封杀之「不形成记忆」（用户语义裁定 2026-11-19）:
        // 学习事件直接拒绝——组词/联想/候选已由 query 出口统一滤。
        if crate::engine::awb::is_sensitive(word) {
            return;
        }
        self.apply(key, word, ts);
        self.journal_len += 1;
        let formed = self.tail_push(key, word, ts, true);
        let mut buf = Vec::with_capacity(16 + key.len() + word.len());
        put_event(&mut buf, ts, key, word, 0);
        for (pk, text, reading) in formed {
            put_phrase_event(&mut buf, ts, &pk, &text, &reading);
            self.journal_len += 1;
        }
        if self.journal_len >= FLUSH_THRESHOLD || self.ver < 2 {
            // v1 旧库: 追加 tagged 事件会与无标签旧事件错位——flush 整体重写为 v2
            self.flush();
            return;
        }
        // G 移交发现 2（根因修正）: 空文件起步直接 append 会写出**无
        // HHD1 头的裸 journal**——load 判坏头回空库, 首个学习事件必丢。
        // 文件不存在 → 先 flush 落全量快照（头+checkpoint）, 后续事件
        // 再走 append。
        if !self.path.as_os_str().is_empty() && !self.path.exists() {
            self.flush();
            return;
        }
        if let Ok(mut f) = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)
        {
            let _ = f.write_all(&buf); // 失败容忍：内存态仍正确，下次 flush 补
        }
    }

    /// 会话尾滚动: 裁掉与最新 commit 链式间隔断开的前缀, 推入新词;
    /// learn=true 时对每个后缀 n-gram (2..=MAX_PHRASE_WORDS, 音节≤帽)
    /// 造/调频短语并返回本次形成清单（journal 落盘用）。
    fn tail_push(
        &mut self,
        key: &str,
        word: &str,
        ts: u64,
        learn: bool,
    ) -> Vec<(String, String, String)> {
        // 链式连续判定: 从尾往前, 相邻间隔 ≤ GAP 的最长后缀
        let mut start = self.tail.len();
        while start > 0 {
            let prev_ts = self.tail[start - 1].ts;
            let cur_ts =
                if start == self.tail.len() { ts } else { self.tail[start].ts };
            if cur_ts.saturating_sub(prev_ts) <= SESSION_GAP {
                start -= 1;
            } else {
                break;
            }
        }
        self.tail.drain(..start);
        self.tail.push(TailWord {
            word: word.to_string(),
            key: key.to_string(),
            ts,
        });
        let mut formed = Vec::new();
        if !learn {
            if self.tail.len() > MAX_PHRASE_WORDS + 1 {
                self.tail.remove(0);
            }
            return formed;
        }
        let n_max = self.tail.len().min(MAX_PHRASE_WORDS);
        for n in 2..=n_max {
            let seg = &self.tail[self.tail.len() - n..];
            // G 移交发现 3: 同词快连击自 n-gram（火锅火锅 rank0 压死本尊
            // ——§4.2 实录）→ 相邻同词的 n-gram 一律不造（60s 断会话已挡
            // 慢重复, 此处挡快连击）
            if seg.windows(2).any(|p| p[0].word == p[1].word) {
                continue;
            }
            let reading: String =
                seg.iter().map(|s| s.key.as_str()).collect::<Vec<_>>().join(",");
            if reading.split(',').count() > MAX_PHRASE_SYL {
                continue;
            }
            let text: String =
                seg.iter().map(|s| s.word.as_str()).collect();
            let pk = reading.replace(',', "");
            self.apply_phrase(&pk, &text, &reading, ts);
            formed.push((pk, text, reading));
        }
        if self.tail.len() > MAX_PHRASE_WORDS + 1 {
            self.tail.remove(0);
        }
        formed
    }

    fn apply_phrase(
        &mut self,
        pk: &str,
        text: &str,
        reading: &str,
        ts: u64,
    ) {
        let g = self.phrases.entry(pk.to_string()).or_default();
        match g.get_mut(text) {
            Some(pe) => pe.e.on_event(ts),
            None => {
                g.insert(
                    text.to_string(),
                    PhraseEntry {
                        reading: reading.to_string(),
                        e: Entry::new(ts),
                    },
                );
            }
        }
    }

    /// B5 短语精确直查（无分隔全拼键）; anchor 降序
    pub fn phrase_lookup(&self, pk: &str) -> Vec<(String, PhraseEntry)> {
        match self.phrases.get(pk) {
            Some(g) => {
                let mut v: Vec<(String, PhraseEntry)> =
                    g.iter().map(|(t, pe)| (t.clone(), pe.clone())).collect();
                v.sort_by(|a, b| {
                    b.1.e
                        .anchor
                        .partial_cmp(&a.1.e.anchor)
                        .unwrap()
                        .then(b.1.e.count.cmp(&a.1.e.count))
                });
                v
            }
            None => Vec::new(),
        }
    }

    /// B5-c 短语简拼投影（docs/设计-上下文动态组词.md §c.2）:
    /// 条目读音（逗号形）的首字母投影恰等于输入首字母串时命中。
    /// anchor 降序, cap 条（同 phrase_lookup 律）。
    /// 动机（验收探针 #1/#2 实证）: 短语键域=无分隔全拼, 简拼输入
    /// （ewzdn/xmdbmh）永不命中 B5 注入——同一预学短语全拼 rank1 /
    /// 简拼缺席, 缺口在「简拼×短语桥」。个人库量级（千级）全键线性
    /// 扫可接受（同 phrase_prefix_lookup）。
    /// is_syl: 引擎音节表判真（L1 权威域）——投影的拆分合并需要它
    /// （见 project_initials; 引擎接线传 `|s| seg.is_syl(s)`）。
    pub fn phrase_initial_lookup(
        &self,
        inits: &str,
        cap: usize,
        is_syl: &dyn Fn(&str) -> bool,
    ) -> Vec<(String, PhraseEntry)> {
        if inits.is_empty() {
            return Vec::new();
        }
        let mut v: Vec<(String, PhraseEntry)> = Vec::new();
        for g in self.phrases.values() {
            for (t, pe) in g {
                if project_initials(&pe.reading, is_syl) == inits {
                    v.push((t.clone(), pe.clone()));
                }
            }
        }
        v.sort_by(|a, b| {
            b.1.e
                .anchor
                .partial_cmp(&a.1.e.anchor)
                .unwrap()
                .then(b.1.e.count.cmp(&a.1.e.count))
        });
        v.truncate(cap);
        v
    }

    /// B5-2 前缀补全: 键前缀命中（键 != 前缀——精确命中走 phrase_lookup）;
    /// anchor 降序, cap 条。个人库量级（千级）线性扫可接受。
    pub fn phrase_prefix_lookup(
        &self,
        prefix: &str,
        cap: usize,
    ) -> Vec<(String, String, PhraseEntry)> {
        let mut v: Vec<(String, String, PhraseEntry)> = Vec::new();
        for (k, g) in &self.phrases {
            if k.len() <= prefix.len() || !k.starts_with(prefix) {
                continue;
            }
            for (t, pe) in g {
                v.push((k.clone(), t.clone(), pe.clone()));
            }
        }
        v.sort_by(|a, b| {
            b.2.e
                .anchor
                .partial_cmp(&a.2.e.anchor)
                .unwrap()
                .then(b.2.e.count.cmp(&a.2.e.count))
        });
        v.truncate(cap);
        v
    }

    /// B5-4 删词: 全键扫除该词（一词可跨键存在——不同 rid hint 提交）。
    /// 删除即 flush（checkpoint 原子重写吸收 journal——重放无删事件,
    /// 天然一致; 删除低频, 全量重写代价可接受）。返回是否有删动。
    pub fn remove_word(&mut self, word: &str) -> bool {
        let mut removed = false;
        for g in self.groups.values_mut() {
            removed |= g.remove(word).is_some();
        }
        self.groups.retain(|_, g| !g.is_empty());
        if removed {
            self.flush();
        }
        removed
    }

    /// 删词级联（backlog 学习子系统项）: 删词 + 连带删**文本包含该词**
    /// 的自造短语（组句 n-gram 遗骸——词都删了, 含词短语不再有独立
    /// 语义）。单字词不级联（单字几乎含于一切短语, 连删即清库——
    /// 误伤面不可接受）。返回 (词删动, 级联删短语数)。
    pub fn remove_word_cascade(&mut self, word: &str) -> (bool, usize) {
        let removed = self.remove_word(word);
        if !removed || word.chars().count() < 2 {
            return (removed, 0);
        }
        let mut n = 0usize;
        for g in self.phrases.values_mut() {
            let hit: Vec<String> = g
                .keys()
                .filter(|t| t.contains(word))
                .cloned()
                .collect();
            for t in hit {
                g.remove(&t);
                n += 1;
            }
        }
        self.phrases.retain(|_, g| !g.is_empty());
        if n > 0 {
            self.flush();
        }
        (removed, n)
    }

    /// B5-4 删短语: 文本精确匹配（全键扫）。
    pub fn remove_phrase(&mut self, text: &str) -> bool {
        let mut removed = false;
        for g in self.phrases.values_mut() {
            removed |= g.remove(text).is_some();
        }
        self.phrases.retain(|_, g| !g.is_empty());
        if removed {
            self.flush();
        }
        removed
    }

    pub fn n_phrases(&self) -> usize {
        self.phrases.values().map(|g| g.len()).sum()
    }

    /// 短语全库转储（CLI/调试）
    pub fn dump_phrases(&self) -> Vec<(String, String, PhraseEntry)> {
        let mut v = Vec::new();
        for (k, g) in &self.phrases {
            for (t, pe) in g {
                v.push((k.clone(), t.clone(), pe.clone()));
            }
        }
        v.sort_by(|a, b| {
            b.2.e.anchor.partial_cmp(&a.2.e.anchor).unwrap()
        });
        v
    }

    /// B5-4 容量帽配置（clamp 界内; 即时生效——下次 flush 落 rsv 持久化,
    /// 超帽回收即时执行一次）
    pub fn set_max_phrases(&mut self, n: usize) {
        self.max_phrases =
            n.clamp(MAX_PHRASES_MIN, MAX_PHRASES_MAX);
        self.evict_phrases(self.max_phrases);
    }

    pub fn max_phrases(&self) -> usize {
        self.max_phrases
    }

    /// B5-3 短语回收: 超帽按 (anchor, count) 降序保留 cap 条。
    /// 正常个人库量级（千级）远低于帽——仅长期防线。
    /// 短语 TTL（backlog 学习子系统）: >180 天且 count==1、anchor==1.0
    /// 的死短语清理——容量帽外的年龄维度（帽保高 anchor 活短语, TTL 清
    /// 从未复用的陈货; anchor 恒 >1 除从未二次事件外——事件重算
    /// anchor×exp(−Δt)+1 > 1 恒成立, 故 ==1.0 精确等价「单次事件」）。
    /// 挂 checkpoint（flush 路径, 与 evict_phrases 同点）。返回清理数。
    pub fn ttl_sweep_phrases(&mut self, now: u64) -> usize {
        const PHRASE_TTL: u64 = 180 * 86_400;
        let mut n = 0usize;
        for g in self.phrases.values_mut() {
            let dead: Vec<String> = g
                .iter()
                .filter(|(_, pe)| {
                    pe.e.count == 1
                        && pe.e.anchor == 1.0
                        && now.saturating_sub(pe.e.ts) > PHRASE_TTL
                })
                .map(|(t, _)| t.clone())
                .collect();
            for t in dead {
                g.remove(&t);
                n += 1;
            }
        }
        self.phrases.retain(|_, g| !g.is_empty());
        n
    }

    pub fn evict_phrases(&mut self, cap: usize) {
        let n = self.n_phrases();
        if n <= cap {
            return;
        }
        let mut all: Vec<(String, String, f32, u32)> = self
            .phrases
            .iter()
            .flat_map(|(k, g)| {
                g.iter().map(move |(t, pe)| {
                    (k.clone(), t.clone(), pe.e.anchor, pe.e.count)
                })
            })
            .collect();
        all.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap()
                .then(b.3.cmp(&a.3))
        });
        let keep: std::collections::HashSet<(String, String)> =
            all.into_iter()
                .take(cap)
                .map(|(k, t, _, _)| (k, t))
                .collect();
        self.phrases.retain(|k, g| {
            g.retain(|t, _| keep.contains(&(k.clone(), t.clone())));
            !g.is_empty()
        });
    }

    /// 原子重写（tmp + rename）：checkpoint = 当前全量状态，journal 清零
    pub fn flush(&mut self) {
        let buf = self.checkpoint_bytes();
        let tmp = self.path.with_extension("tmp");
        if fs::write(&tmp, &buf).is_ok() {
            let _ = fs::rename(&tmp, &self.path);
            self.journal_len = 0;
            self.ver = 2;
        }
    }

    /// 全量 checkpoint 序列化（HHD1 v2；wasm 宿主持久化路径——
    /// 与 flush 同形态，仅免 fs。journal 归零语义同 flush 成功分支）
    pub fn save_bytes(&mut self) -> Vec<u8> {
        let buf = self.checkpoint_bytes();
        self.journal_len = 0;
        self.ver = 2;
        buf
    }

    fn checkpoint_bytes(&mut self) -> Vec<u8> {
        // TTL 清扫（真实时钟; 陈年死短语不进快照——journal 重放同样
        // 不会复生: 重放事件重新 apply 的 ts 同样陈旧, 下次清扫再清）
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.ttl_sweep_phrases(now);
        self.evict_phrases(self.max_phrases);
        let mut buf = Vec::with_capacity(4096);
        buf.extend_from_slice(&MAGIC.to_le_bytes());
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.extend_from_slice(&(self.max_phrases as u32).to_le_bytes());
        buf.extend_from_slice(&(self.groups.len() as u32).to_le_bytes());
        let mut keys: Vec<&String> = self.groups.keys().collect();
        keys.sort();
        for k in keys {
            let g = &self.groups[k];
            put_str(&mut buf, k);
            buf.push(g.len() as u8);
            let mut words: Vec<&String> = g.keys().collect();
            words.sort();
            for w in words {
                let e = &g[w];
                put_str(&mut buf, w);
                buf.extend_from_slice(&e.count.to_le_bytes());
                buf.extend_from_slice(&e.anchor.to_le_bytes());
                buf.extend_from_slice(&e.ts.to_le_bytes());
            }
        }
        // B5 短语段
        buf.extend_from_slice(&(self.phrases.len() as u32).to_le_bytes());
        let mut pks: Vec<&String> = self.phrases.keys().collect();
        pks.sort();
        for pk in pks {
            let g = &self.phrases[pk];
            put_str(&mut buf, pk);
            buf.push(g.len() as u8);
            let mut texts: Vec<&String> = g.keys().collect();
            texts.sort();
            for t in texts {
                let pe = &g[t];
                put_str(&mut buf, t);
                put_str(&mut buf, &pe.reading);
                buf.extend_from_slice(&pe.e.count.to_le_bytes());
                buf.extend_from_slice(&pe.e.anchor.to_le_bytes());
                buf.extend_from_slice(&pe.e.ts.to_le_bytes());
            }
        }
        // B5 会话尾段
        buf.extend_from_slice(&(self.tail.len() as u32).to_le_bytes());
        for tw in &self.tail {
            put_str(&mut buf, &tw.word);
            put_str(&mut buf, &tw.key);
            buf.extend_from_slice(&tw.ts.to_le_bytes());
        }
        buf
    }

    /// 同 key 候选按 anchor 降序（并列按 count/ts）
    pub fn lookup(&self, key: &str) -> Vec<(String, Entry)> {
        match self.groups.get(key) {
            Some(g) => {
                let mut v: Vec<(String, Entry)> =
                    g.iter().map(|(w, e)| (w.clone(), *e)).collect();
                v.sort_by(|a, b| {
                    b.1.anchor
                        .partial_cmp(&a.1.anchor)
                        .unwrap()
                        .then(b.1.count.cmp(&a.1.count))
                        .then(b.1.ts.cmp(&a.1.ts))
                });
                v
            }
            None => Vec::new(),
        }
    }

    pub fn get(&self, key: &str, word: &str) -> Option<Entry> {
        self.groups.get(key)?.get(word).copied()
    }

    pub fn n_groups(&self) -> usize {
        self.groups.len()
    }

    pub fn n_entries(&self) -> usize {
        self.groups.values().map(|g| g.len()).sum()
    }

    pub fn journal_len(&self) -> usize {
        self.journal_len
    }

    /// 全库转储（CLI/调试）
    pub fn dump(&self) -> Vec<(String, String, Entry)> {
        let mut v = Vec::new();
        for (k, g) in &self.groups {
            for (w, e) in g {
                v.push((k.clone(), w.clone(), *e));
            }
        }
        v
    }
}

// ----------------------------------------------------------------------
// 编解码助手
fn read_str(b: &[u8], p: &mut usize) -> Option<String> {
    if *p + 2 > b.len() {
        return None;
    }
    let l = le16(b, *p) as usize;
    *p += 2;
    if *p + l > b.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&b[*p..*p + l]).into_owned();
    *p += l;
    Some(s)
}

fn read_entry(b: &[u8], p: &mut usize) -> Option<(u32, f32, u64)> {
    if *p + 16 > b.len() {
        return None;
    }
    let count = le32(b, *p);
    let anchor = f32::from_le_bytes([
        b[*p + 4],
        b[*p + 5],
        b[*p + 6],
        b[*p + 7],
    ]);
    let ts = u64::from_le_bytes([
        b[*p + 8],
        b[*p + 9],
        b[*p + 10],
        b[*p + 11],
        b[*p + 12],
        b[*p + 13],
        b[*p + 14],
        b[*p + 15],
    ]);
    *p += 16;
    Some((count, anchor, ts))
}

fn read_event(
    b: &[u8],
    p: &mut usize,
    tagged: bool,
) -> Option<(u64, String, String)> {
    if tagged {
        if *p >= b.len() {
            return None;
        }
        *p += 1; // 标签字节（调用方已判 =0）
    }
    if *p + 8 > b.len() {
        return None;
    }
    let mut ts = 0u64;
    for i in 0..8 {
        ts |= (b[*p + i] as u64) << (8 * i);
    }
    *p += 8;
    let key = read_str(b, p)?;
    let word = read_str(b, p)?;
    Some((ts, key, word))
}

/// v2 短语事件: [tag=1][ts][nosep 键][拼接文本][逗号全读]
fn read_phrase_event(
    b: &[u8],
    p: &mut usize,
) -> Option<(u64, String, String, String)> {
    if *p >= b.len() {
        return None;
    }
    *p += 1; // 标签字节（调用方已判 =1）
    if *p + 8 > b.len() {
        return None;
    }
    let mut ts = 0u64;
    for i in 0..8 {
        ts |= (b[*p + i] as u64) << (8 * i);
    }
    *p += 8;
    let pk = read_str(b, p)?;
    let text = read_str(b, p)?;
    let reading = read_str(b, p)?;
    Some((ts, pk, text, reading))
}

fn put_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

fn put_event(buf: &mut Vec<u8>, ts: u64, key: &str, word: &str, tag: u8) {
    buf.push(tag);
    buf.extend_from_slice(&ts.to_le_bytes());
    put_str(buf, key);
    put_str(buf, word);
}

fn put_phrase_event(
    buf: &mut Vec<u8>,
    ts: u64,
    pk: &str,
    text: &str,
    reading: &str,
) {
    buf.push(1);
    buf.extend_from_slice(&ts.to_le_bytes());
    put_str(buf, pk);
    put_str(buf, text);
    put_str(buf, reading);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试时间基: 当前时钟（TTL 清扫用真实时钟——固定旧值会被当死短语清）
    const NOW_FOR_TESTS: u64 = 1_790_000_000; // 2026-10 中; 与真实钟差 <180d

    /// G 移交发现 3: 同词快连击不造自 n-gram（火锅火锅 压死本尊案）
    #[test]
    fn fast_double_tap_no_self_ngram() {        let mut ud = UserDict::empty(PathBuf::from("/tmp/hanhuo-test-none"));
        ud.commit("huo,guo", "火锅", 1000);
        ud.commit("huo,guo", "火锅", 2000); // 快连击（< SESSION_GAP）
        ud.commit("huo,guo", "火锅", 3000);
        // 词级学习照常（3 次）
        assert_eq!(ud.get("huo,guo", "火锅").unwrap().count, 3);
        // (w,w) 自短语不得成形
        assert!(
            !ud.phrase_lookup("huoguo").iter().any(|(t, _)| t == "火锅火锅"),
            "快连击不应造 火锅火锅"
        );
    }

    /// G 移交发现 2: 空文件起步 commit → journal 落盘（append+create）
    #[test]
    fn fresh_file_journal_persists() {
        let p = std::env::temp_dir().join(format!(
            "hanhuo-ud-journal-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&p);
        {
            let mut ud = UserDict::empty(p.clone());
            ud.commit("ni,hao", "你好", 1000);
        }
        assert!(p.exists(), "commit 后 journal 文件应在场");
        assert!(fs::metadata(&p).unwrap().len() > 0, "journal 非空");
        // 重开续学（append 链路完整）
        let ud2 = UserDict::load(&p);
        assert_eq!(ud2.get("ni,hao", "你好").unwrap().count, 1);
        let _ = fs::remove_file(&p);
    }

    /// 删除级联（--cascade）: 删词连带删含词短语; 单字词不级联
    #[test]
    fn remove_word_cascade_phrases() {
        let mut ud = UserDict::empty(PathBuf::from("/tmp/hanhuo-test-none"));
        // 造词 + 含词短语（模拟 pick 链）
        ud.commit("huo,guo", "火锅", NOW_FOR_TESTS + 0);
        ud.commit("chi", "吃", NOW_FOR_TESTS + 1000);
        ud.commit("qu", "去", NOW_FOR_TESTS + 2000);
        // 直接埋两条含 火锅 的短语 + 一条无关短语
        ud.apply_phrase("chihuoguo", "吃火锅", "chi,huo,guo", NOW_FOR_TESTS + 2000);
        ud.apply_phrase("quchihuoguo", "去吃火锅", "qu,chi,huo,guo", NOW_FOR_TESTS + 2000);
        ud.apply_phrase("chiwanle", "吃完了", "chi,wan,le", NOW_FOR_TESTS + 2000);
        assert_eq!(ud.n_phrases(), 3);
        // 级联删 火锅: 词 + 2 条含词短语, 无关短语保留
        let (ok, n) = ud.remove_word_cascade("火锅");
        assert!(ok);
        assert_eq!(n, 2, "含 火锅 的两条短语应级联删除");
        assert!(ud.get("huo,guo", "火锅").is_none());
        assert_eq!(ud.n_phrases(), 1);
        assert!(
            ud.phrase_lookup("chiwanle").iter().any(|(t, _)| t == "吃完了"),
            "无关短语保留"
        );
        // 单字词不级联: 吃 在场 → 删除仅词本身, 吃完了 保留
        let (ok2, n2) = ud.remove_word_cascade("吃");
        assert!(ok2);
        assert_eq!(n2, 0, "单字词不级联");
        // 未学词 → 无删动
        let (ok3, _) = ud.remove_word_cascade("不存在");
        assert!(!ok3);
    }

    /// anchor 实时衰减: eff = anchor×exp(−Δt/1d); 同刻原值, 久置归零
    #[test]
    fn effective_anchor_decay() {
        // 同刻: Δt=0 → 原值
        assert!((effective_anchor(2.0, 1000, 1000) - 2.0).abs() < 1e-6);
        // 1 天前: 2.0 → 2/e ≈ 0.736（<1 → boost 归零）
        let e1 = effective_anchor(2.0, 1000, 1000 + 86_400);
        assert!((e1 - 2.0 / std::f64::consts::E as f32).abs() < 1e-3);
        assert_eq!(user_boost_at(2.0, 1000, 1000 + 86_400), 0.0, "衰减后 <1 → boost 0");
        // 半天前: 2.0 → 2×e^−0.5 ≈ 1.213（>1 → boost >0, 但低于同刻）
        let e2 = effective_anchor(2.0, 1000, 1000 + 43_200);
        assert!(e2 > 1.0);
        assert!(
            user_boost_at(2.0, 1000, 1000 + 43_200)
                < user_boost_at(2.0, 1000, 1000)
        );
        // 无限久 → 0
        assert!(effective_anchor(99.0, 1000, 1000 + 86_400 * 365) < 1e-6);
    }

    /// 短语 TTL: >180 天 count==1 anchor==1 清; 活跃/新/复用过的都留
    #[test]
    fn phrase_ttl_sweep() {
        let now = 40_000_000u64; // 任意基准
        let ttl = 180 * 86_400u64;
        let mut ud = UserDict::empty(PathBuf::from("/tmp/hanhuo-test-none"));
        // 死短语: 181 天前单次
        ud.apply_phrase("siwangle", "死完了", "si,wan,le", now - ttl - 1);
        // 同龄但二次事件过（anchor>1）→ 留
        ud.apply_phrase("fuole", "复读了", "fu,le", now - ttl - 1);
        ud.apply_phrase("fuole", "复读了", "fu,le", now - ttl); // 二次 → anchor>1
        // 新短语（昨天）→ 留
        ud.apply_phrase("xinde", "新的", "xin,de", now - 86_400);
        // 边界: 恰 180 天不清（> 严格）
        ud.apply_phrase("bianjie", "边界", "bian,jie", now - ttl);
        let n = ud.ttl_sweep_phrases(now);
        assert_eq!(n, 1, "只清 181 天单次的死短语");
        assert!(
            !ud.phrase_lookup("siwangle").iter().any(|(t, _)| t == "死完了"),
            "死短语应被清"
        );
        assert!(
            ud.phrase_lookup("fuole").iter().any(|(t, _)| t == "复读了"),
            "复用过的留（anchor>1）"
        );
        assert!(
            ud.phrase_lookup("xinde").iter().any(|(t, _)| t == "新的"),
            "新短语留"
        );
        assert!(
            ud.phrase_lookup("bianjie").iter().any(|(t, _)| t == "边界"),
            "恰 180 天不清（严格大于）"
        );
    }

    /// from_bytes ← save_bytes 往返 + checkpoint 幂等（wasm 链路锁）
    #[test]
    fn from_bytes_save_bytes_roundtrip() {
        let mut ud = UserDict::empty(PathBuf::new());
        ud.commit("ni,hao", "你好", 1000);
        ud.commit("ni,hao", "你好", 2000);
        ud.commit("ni,hao", "泥嚎", 3000);
        let bytes = ud.save_bytes();
        let mut ud2 = UserDict::from_bytes(&bytes);
        assert_eq!(ud2.n_groups(), 1);
        assert_eq!(ud2.get("ni,hao", "你好").unwrap().count, 2);
        assert_eq!(ud2.get("ni,hao", "泥嚎").unwrap().count, 1);
        // checkpoint 后 journal 归零 → 再存字节不变（幂等）
        assert_eq!(ud2.save_bytes(), bytes);
        // 坏头 → 空库（宁丢勿炸）
        assert_eq!(UserDict::from_bytes(b"garbage!!").n_entries(), 0);
    }

    /// obs_002 学习曲线复刻（你好×5@3s + 泥嚎×2@4s + 3 天闲置重爬）
    #[test]
    fn obs002_replay() {
        let mut ud = UserDict {
            path: PathBuf::from("/tmp/hanhuo-test-none"),
            groups: HashMap::new(),
            phrases: HashMap::new(),
            tail: Vec::new(),
            journal_len: 0,
            ver: 2,
            max_phrases: MAX_PHRASE_ENTRIES,
        };
        let t0: u64 = 1792969845;
        for i in 0..5u64 {
            ud.apply("ni,hao", "你好", t0 + i * 3);
        }
        let e = ud.get("ni,hao", "你好").unwrap();
        assert_eq!(e.count, 5);
        assert!((e.anchor - 4.9997).abs() < 1e-3, "anchor={}", e.anchor);
        // 泥嚎 ×2 @4s
        ud.apply("ni,hao", "泥嚎", t0 + 15);
        ud.apply("ni,hao", "泥嚎", t0 + 19);
        let e = ud.get("ni,hao", "泥嚎").unwrap();
        assert_eq!(e.count, 2);
        assert!((e.anchor - 2.0).abs() < 1e-3);
        // 3 天闲置 → exp 因子 e^-3 → anchor = 4.9997×0.049787+1 ≈ 1.2489
        // （wine 时钟注入 25h 直证指数衰减；旧"归零重爬"系线性外推误判）
        ud.apply("ni,hao", "你好", t0 + 3 * 86400);
        let e = ud.get("ni,hao", "你好").unwrap();
        assert_eq!(e.count, 6);
        assert!((e.anchor - 1.2489).abs() < 2e-3, "anchor={}", e.anchor);
    }

    /// 持久化往返: commit → flush → load 状态一致
    #[test]
    fn persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "hanhuo-ud-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user.bin");
        let _ = fs::remove_file(&path);
        {
            let mut ud = UserDict::load(&path);
            assert_eq!(ud.n_entries(), 0);
            ud.commit("ni,hao", "你好", 1000);
            ud.commit("ni,hao", "你好", 1003);
            ud.commit("xiu,zheng", "修正", 2000);
            ud.flush();
        }
        let ud2 = UserDict::load(&path);
        let e = ud2.get("ni,hao", "你好").unwrap();
        assert_eq!(e.count, 2);
        assert!((e.anchor - 1.99997).abs() < 1e-3, "anchor={}", e.anchor);
        assert_eq!(ud2.get("xiu,zheng", "修正").unwrap().count, 1);
        // journal 路径: commit 不 flush → load 重放一致
        let mut ud3 = UserDict::load(&path);
        ud3.commit("ni,hao", "你好", 2000); // Δt=997s
        drop(ud3);
        let ud4 = UserDict::load(&path);
        let e = ud4.get("ni,hao", "你好").unwrap();
        assert_eq!(e.count, 3);
        // anchor = f32(2.0 × (1−997/86400)) + 1 ≈ 2.97691
        assert!((e.anchor - 2.97691).abs() < 1e-3, "anchor={}", e.anchor);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn boost_shape() {
        assert_eq!(user_boost(1.0), 0.0); // 首次学习不加成
        // ≥5 超基座高频（实证标定; 面板钉序基座差≈3.0 → 1.9·ln5≈3.06）
        assert!(user_boost(5.0) >= 3.0);
        assert_eq!(user_boost(100.0), 6.0); // 封顶
        // 会话实证回归锚: anchor=10（burst 十连）4.375 > 3.0 基座差
        assert!((user_boost(10.0) - 1.9 * 10f64.ln()).abs() < 1e-12);
    }

    /// 原生曲线复刻自检（libwxhld user_dict_lookup 2026-10-26 战役）
    #[test]
    fn boost_native_curve() {
        use super::user_boost_native as nb;
        // v=anchor（dt=0 → e≈1）：anchor=1 → 线性区 y≈−3.048；anchor=5 → 压缩区
        let a1 = nb(1, 0, 1.0, false);
        let a5 = nb(5, 0, 5.0, false);
        assert!((a1 - (-3.0387)).abs() < 1e-3, "a1={a1}");   // 含 0.01/Δ 项
        assert!((a5 - (-1.2477)).abs() < 1e-3, "a5={a5}");
        // 陈旧弱词：v→y3 平底
        let old = nb(1, 2_097_152, 1.0, false);
        assert!((old - (-7.1439)).abs() < 1e-2, "old={old}");
        // 强词显著优于弱词（服务器"≥5 超基座"的 log 域差 ≈1.79）
        assert!(a5 - a1 > 1.5);
        // 惩罚项（查表路）：count=1 → −1.0
        let pen = nb(1, 0, 5.0, true);
        assert!((pen - (a5 - 1.0)).abs() < 1e-5, "pen={pen}");
    }

    // ---- B5 组词/组句（HHD1 v2）----

    fn tmp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hanhuo_ud_b5_{tag}_{}.hhd1",
            std::process::id()
        ))
    }

    #[test]
    fn b5_phrase_ngram_formation_and_repeat() {
        let p = tmp_path("ngram");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        ud.commit("ni,hao", "你好", 1000);
        ud.commit("shi,jie", "世界", 1010); // +10s 连续 → bigram
        ud.commit("a,ab", "阿坝", 1020); // +10s → trigram
        // bigram ×2（你好世界 出现两轮）
        let v = ud.phrase_lookup("nihaoshijie");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].0, "你好世界");
        assert_eq!(v[0].1.reading, "ni,hao,shi,jie");
        assert_eq!(v[0].1.e.count, 1, "首学 count=1");
        // trigram 首学
        let v3 = ud.phrase_lookup("nihaoshijieaab");
        assert_eq!(v3.len(), 1, "3-gram 后缀");
        assert_eq!(v3[0].0, "你好世界阿坝");
        // 复用（隔 GAP 内第二轮）→ count=2, anchor↑
        ud.commit("ni,hao", "你好", 2000);
        ud.commit("shi,jie", "世界", 2010);
        let v2 = ud.phrase_lookup("nihaoshijie");
        assert_eq!(v2[0].1.e.count, 2);
        assert!(v2[0].1.e.anchor > v[0].1.e.anchor);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b5_session_gap_breaks_formation() {
        let p = tmp_path("gap");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        ud.commit("ni,hao", "你好", 1000);
        ud.commit("shi,jie", "世界", 1000 + SESSION_GAP + 1); // 超窗
        assert!(ud.phrase_lookup("nihaoshijie").is_empty(), "隔断不成词");
        // 新会话重启: 世界 之后连续 → 新 bigram
        ud.commit("a,ab", "阿坝", 1000 + SESSION_GAP + 11);
        let v = ud.phrase_lookup("shijieaab");
        assert_eq!(v.len(), 1, "新会话 bigram");
        assert!(ud.phrase_lookup("nihaoshijieaab").is_empty());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b5_flush_roundtrip_phrases_and_tail() {
        // 现代时间基（TTL 清扫以真实时钟跑——纪元初/距今>180 天的单次短语必被清）
        const T0: u64 = NOW_FOR_TESTS;
        let p = tmp_path("flush");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        ud.commit("ni,hao", "你好", T0 + 0);
        ud.commit("shi,jie", "世界", T0 + 10);
        ud.commit("a,ab", "阿坝", T0 + 20);
        ud.flush();
        // 重载: 短语与 tail 都在（tail 链式窗内 3 词）
        let ud2 = UserDict::load(&p);
        assert_eq!(ud2.n_phrases(), 3, "bigram×2 + trigram");
        assert_eq!(ud2.phrase_lookup("nihaoshijieaab")[0].0, "你好世界阿坝");
        assert_eq!(ud2.tail.len(), 3, "会话尾持久");
        // tail 存活 → 下一 commit 在同会话续造 4-gram
        let mut ud2 = ud2;
        ud2.commit("b", "吧", T0 + 30);
        assert_eq!(
            ud2.phrase_lookup("nihaoshijieaab")[0].0,
            "你好世界阿坝"
        );
        let v4 = ud2.phrase_lookup("nihaoshijieaabb");
        assert_eq!(v4.len(), 1, "跨进程续会话 4-gram");
        assert_eq!(v4[0].0, "你好世界阿坝吧");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b5_v1_file_upgrade_loads_and_flushes_v2() {
        let p = tmp_path("v1up");
        let _ = fs::remove_file(&p);
        // 手写 v1 最小库: header + 1 组 1 条 + 1 个无标签事件
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes()); // v1
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes()); // n_groups
        put_str(&mut b, "ni,hao");
        b.push(1); // nent
        put_str(&mut b, "你好");
        b.extend_from_slice(&3u32.to_le_bytes());
        b.extend_from_slice(&2.5f32.to_le_bytes());
        b.extend_from_slice(&900u64.to_le_bytes());
        // journal: 无标签词事件
        b.extend_from_slice(&(NOW_FOR_TESTS as u64).to_le_bytes());
        put_str(&mut b, "ni,hao");
        put_str(&mut b, "你好");
        fs::write(&p, &b).unwrap();
        let mut ud = UserDict::load(&p);
        assert_eq!(ud.n_groups(), 1, "v1 checkpoint 组");
        let e = ud.get("ni,hao", "你好").unwrap();
        assert_eq!(e.count, 4, "checkpoint 3 + journal 1 事件");
        assert_eq!(ud.tail.len(), 1, "v1 journal 重放重建 tail");
        // commit 到 v1 库 → 强制 flush 升 v2（不与旧事件错位）
        ud.commit("shi,jie", "世界", NOW_FOR_TESTS + 10);
        let ud2 = UserDict::load(&p);
        assert_eq!(ud2.ver, 2, "升版 v2");
        assert_eq!(ud2.get("ni,hao", "你好").unwrap().count, 4, "词表无损");
        assert_eq!(ud2.n_phrases(), 1, "升级后短语在");
        assert_eq!(ud2.phrase_lookup("nihaoshijie")[0].0, "你好世界");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b5_prefix_lookup_semantics() {
        let p = tmp_path("prefix");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        ud.commit("ni,hao", "你好", 1000);
        ud.commit("shi,jie", "世界", 1010);
        ud.commit("a,ab", "阿坝", 1020);
        // 键: nihaoshijie / shijieaab / nihaoshijieaab
        // 前缀 nihaoshi → 两键（nihaoshijie, nihaoshijieaab）, 精确键排除
        let v = ud.phrase_prefix_lookup("nihaoshi", 8);
        let keys: Vec<String> = v.iter().map(|(k, _, _)| k.clone()).collect();
        assert!(keys.contains(&"nihaoshijie".to_string()));
        assert!(keys.contains(&"nihaoshijieaab".to_string()));
        // 全前缀（等于某完整键）→ 该键排除（len <= prefix.len()）
        let v2 = ud.phrase_prefix_lookup("nihaoshijie", 8);
        assert!(v2.iter().all(|(k, _, _)| k.as_str() != "nihaoshijie"),
            "精确键走 phrase_lookup, 前缀查询不含");
        assert!(v2.iter().any(|(k, _, _)| k == "nihaoshijieaab"));
        // cap
        assert!(ud.phrase_prefix_lookup("nihaoshi", 1).len() <= 1);
        // 无命中
        assert!(ud.phrase_prefix_lookup("zzz", 8).is_empty());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b53_evict_phrases_by_anchor() {
        let p = tmp_path("evict");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        // 造 5 个短语: 后两个复用一轮（anchor 更高）
        let pairs = [
            ("a", "啊", "b", "吧"),
            ("ni", "你", "hao", "好"),
            ("shi", "是", "ab", "阿"),
            ("ba", "把", "ma", "吗"),
            ("lv", "驴", "kou", "口"),
        ];
        let t0 = 1000u64;
        for (i, (k1, w1, k2, w2)) in pairs.iter().enumerate() {
            let t = t0 + i as u64 * 100;
            ud.commit(k1, w1, t);
            ud.commit(k2, w2, t + 1);
        }
        // 复用第 4 对（把吗）与第 5 对（驴口）
        ud.commit("ba", "把", 2000);
        ud.commit("ma", "吗", 2001);
        ud.commit("lv", "驴", 2002);
        ud.commit("kou", "口", 2003);
        assert!(ud.n_phrases() >= 5);
        ud.evict_phrases(2); // 只留 anchor 最高的两条
        let kept: Vec<(String, String, PhraseEntry)> = ud.dump_phrases();
        assert_eq!(kept.len(), 2, "容量帽");
        let texts: Vec<&str> =
            kept.iter().map(|(_, t, _)| t.as_str()).collect();
        assert!(texts.contains(&"把吗") && texts.contains(&"驴口"),
            "复用轮（anchor↑）保留: {texts:?}");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b54_remove_word_and_phrase() {
        // 现代时间基（TTL 清扫以真实时钟跑——纪元初/距今>180 天的单次短语必被清）
        const T0: u64 = NOW_FOR_TESTS;
        let p = tmp_path("remove");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        ud.commit("ni,hao", "你好", T0 + 0);
        ud.commit("shi,jie", "世界", T0 + 10); // 短语 你好世界
        assert!(ud.remove_word("你好"));
        assert!(ud.get("ni,hao", "你好").is_none());
        assert!(ud.n_phrases() == 1, "删词不级联删短语（显式删）");
        assert!(ud.remove_phrase("你好世界"));
        assert_eq!(ud.n_phrases(), 0);
        // flush 吸收 journal → 跨进程一致
        let ud2 = UserDict::load(&p);
        assert!(ud2.get("ni,hao", "你好").is_none(), "重载后仍无");
        assert!(ud2.get("shi,jie", "世界").is_some(), "其余词无损");
        assert_eq!(ud2.n_phrases(), 0, "短语删除持久");
        // 删不存在 → false
        assert!(!ud2.groups.is_empty());
        let mut ud2 = ud2;
        assert!(!ud2.remove_word("不存在"));
        assert!(!ud2.remove_phrase("不存在短语"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b54_max_phrases_dynamic_config() {
        // 现代时间基（TTL 清扫以真实时钟跑——纪元初/距今>180 天的单次短语必被清）
        const T0: u64 = NOW_FOR_TESTS;
        let p = tmp_path("cap");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        assert_eq!(ud.max_phrases(), MAX_PHRASE_ENTRIES, "新库默认");
        // 配置 + flush → rsv 持久化
        ud.set_max_phrases(3);
        assert_eq!(ud.max_phrases(), 3);
        ud.flush();
        let ud2 = UserDict::load(&p);
        assert_eq!(ud2.max_phrases(), 3, "rsv 槽跨进程读回");
        // 界外 clamp
        let mut ud2 = ud2;
        ud2.set_max_phrases(usize::MAX);
        assert_eq!(ud2.max_phrases(), MAX_PHRASES_MAX);
        ud2.set_max_phrases(0);
        assert_eq!(ud2.max_phrases(), MAX_PHRASES_MIN);
        // 动态帽驱动回收: 帽=2, 造 3 短语, flush 后剩 2
        ud2.set_max_phrases(2);
        ud2.commit("a", "啊", T0 + 0);
        ud2.commit("b", "吧", NOW_FOR_TESTS + 1);
        ud2.commit("ni", "你", NOW_FOR_TESTS + 2);
        ud2.commit("hao", "好", NOW_FOR_TESTS + 3);
        ud2.commit("shi", "是", NOW_FOR_TESTS + 4);
        ud2.commit("ab", "阿", NOW_FOR_TESTS + 5);
        ud2.flush();
        assert_eq!(UserDict::load(&p).n_phrases(), 2, "动态帽回收");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn b5_phrase_caps() {
        let p = tmp_path("caps");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        // 8 个不同词连续 → 6-gram 是最长入库, 无 7/8-gram
        let words = [
            ("a", "啊"),
            ("b", "吧"),
            ("ni", "你"),
            ("hao", "好"),
            ("shi", "是"),
            ("ab", "阿"),
            ("ba", "把"),
            ("ma", "吗"),
        ];
        for (i, (k, w)) in words.iter().enumerate() {
            ud.commit(k, w, 1000 + i as u64);
        }
        let all = ud.dump_phrases();
        assert!(all.iter().all(|(_, t, _)| t.chars().count() <= MAX_PHRASE_WORDS),
            "词数帽: 最长 {}",
            all.iter().map(|(_, t, _)| t.chars().count()).max().unwrap_or(0));
        assert!(all.iter().any(|(_, t, _)| t.chars().count() == MAX_PHRASE_WORDS),
            "6-gram 存在");
        assert_eq!(all.iter().filter(|(_, t, _)| t.chars().count() == 6).count(),
            3, "6-gram 后缀数（词 6/7/8 提交时各成一个新 6-gram 文本）");
        let _ = fs::remove_file(&p);
    }

    /// B5-c 短语简拼投影: 首字母串恰等命中; zh/ch/sh → z/c/s 投影;
    /// anchor 降序 + cap; n-gram 伴生短语（4-gram 投影 ewzd）同表并存
    /// ——输入 4 首字母命中 4-词短语（渐进上屏友好, 恰等语义的自然
    /// 推论）; 无对应 n-gram 的串不命中
    #[test]
    fn b5c_phrase_initial_lookup_projection() {
        // 合并律直测（wxq 拆存族: di+an→dian 当且仅当表内合法）
        let has_dian = |s: &str| s == "dian";
        assert_eq!(
            project_initials("di,an,bao,mi,hua", &has_dian),
            "dbmh",
            "di+an 并成 dian（点）→ 仍只出一个声母 d"
        );
        assert_eq!(
            project_initials("mao,a", &has_dian),
            "ma",
            "mao+a 并后非法 → a 自立"
        );
        assert_eq!(
            project_initials("er,wo,zai,deng,ni", &has_dian),
            "ewzdn",
            "首字段元音开头无前驱 → 自立（而）"
        );
        assert_eq!(
            project_initials("zhong,guo", &has_dian),
            "zg",
            "zh/ch/sh → z/c/s（首字母律）"
        );
        let p = tmp_path("inits");
        let _ = fs::remove_file(&p);
        let mut ud = UserDict::load(&p);
        // 会话 A: 而 我 在 等 你（5 词连续）→ 短语 而我在等你
        let t0 = 1000u64;
        for (i, (k, w)) in [("er", "而"), ("wo", "我"), ("zai", "在"),
            ("deng", "等"), ("ni", "你")].iter().enumerate() {
            ud.commit(k, w, t0 + i as u64 * 5);
        }
        // 会话 B: 中国（zhong+guo, zh 投影 z）
        for (i, (k, w)) in [("zhong", "中"), ("guo", "国")]
            .iter().enumerate() {
            ud.commit(k, w, t0 + 10_000 + i as u64 * 5);
        }
        let no_merge = |_: &str| false;
        // 恰等命中
        let v = ud.phrase_initial_lookup("ewzdn", 8, &no_merge);
        assert!(v.iter().any(|(t, _)| t == "而我在等你"), "ewzdn 命中");
        let v2 = ud.phrase_initial_lookup("zg", 8, &no_merge);
        assert!(v2.iter().any(|(t, _)| t == "中国"), "zhong'guo → zg 投影");
        // n-gram 伴生: 4-gram 而我在等（投影 ewzd）同表并存——
        // 输入 4 首字母命中 4-词短语; 无对应 n-gram 的串不命中
        let v4 = ud.phrase_initial_lookup("ewzd", 8, &no_merge);
        assert!(v4.iter().any(|(t, _)| t == "而我在等"), "ewzd 命中 4-gram");
        assert!(
            ud.phrase_initial_lookup("ewzdni", 8, &no_merge).is_empty(),
            "超长串不命中（无 6-gram）"
        );
        // 空/无命中
        assert!(ud.phrase_initial_lookup("", 8, &no_merge).is_empty());
        assert!(ud.phrase_initial_lookup("xyz", 8, &no_merge).is_empty());
        // anchor 降序 + cap: 复用 中国 一轮（anchor↑）后两短语并存,
        // cap=1 时 anchor 高者留
        ud.commit("zhong", "中", t0 + 20_000);
        ud.commit("guo", "国", t0 + 20_005);
        let one = ud.phrase_initial_lookup("zg", 1, &no_merge);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].0, "中国");
        // 复用轮 anchor = 1×exp(−10000/86400)+1 ≈ 1.891（日衰减在途）
        assert!(
            (one[0].1.e.anchor - 1.8913).abs() < 1e-2,
            "复用轮 anchor: {}",
            one[0].1.e.anchor
        );
        let _ = fs::remove_file(&p);
    }
}
