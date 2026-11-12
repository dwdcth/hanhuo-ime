//! T9 九宫通道（设计-T9九宫输入 §3 方案 c: 键串→音节 trie 在线走查,
//! DAT 制导 DFS——2026-11-20 P1）。
//!
//! 键位映射 2=abc 3=def 4=ghi 5=jkl 6=mno 7=pqrs 8=tuv 9=wxyz（与微信
//! 移动端硬编码映射同构, wetyp-微信输入法词库设计.md:220-222）。
//! 展开产物 `Vec<Vec<u32>>`（音节 id 序列）与 `q_jianpin` 消费口逐字节
//! 同形——打分/L4/用户词/awb 全链复用; 面板语义 Kind::Jianpin（设计稿
//! §5-P1③「面板语义同 Jianpin」, P1 不动 Kind 枚举）。
//!
//! P1 口径: 2-6 键单/双音节域; P2 泛化窗口律。纯辅音域（全 9 键等）
//! 分解天然归零（§2.2-2 实测）→ 空面板即正确行为。

use super::Engine;
use crate::dat::ROOT;
use crate::{Cand, Kind};
use std::collections::HashMap;

/// 字母→九宫键（2-9）。a-z 全覆盖; 声母/韵母无例外（九宫无 1 键/q 键域 7）。
fn key_of_letter(c: u8) -> Option<u8> {
    Some(match c {
        b'a' | b'b' | b'c' => b'2',
        b'd' | b'e' | b'f' => b'3',
        b'g' | b'h' | b'i' => b'4',
        b'j' | b'k' | b'l' => b'5',
        b'm' | b'n' | b'o' => b'6',
        b'p' | b'q' | b'r' | b's' => b'7',
        b't' | b'u' | b'v' => b'8',
        b'w' | b'x' | b'y' | b'z' => b'9',
        _ => return None,
    })
}

/// 音节 → 数字键串（逐字母映射; zhuang→948264, ni→64, qing→7464）。
fn t9_sig(syl: &str) -> Option<String> {
    syl.bytes().map(|c| key_of_letter(c).map(|k| k as char)).collect()
}

/// 九宫几何邻接（3×3 字母域 2-9, 含对角——误触域; 设计-T9九宫输入
/// §5-P3①, KBN 邻键的数字版）:
/// ```text
///  1 2 3        T9 字母域 2-9; 1/0/# 非字母域不入
///  4 5 6
///  7 8 9
/// ```
fn t9_neighbors(k: u8) -> &'static [u8] {
    match k {
        b'2' => &[b'3', b'4', b'5', b'6'], // 几何邻含 1, 但 1 非字母域
                                             // （错击到 1 的串不进 T9 域）→ 剔除
        b'3' => &[b'2', b'5', b'6'],
        b'4' => &[b'2', b'5', b'7', b'8'],
        b'5' => &[b'2', b'3', b'4', b'6', b'7', b'8', b'9'], // 中心键=全 8 方
        b'6' => &[b'2', b'3', b'5', b'8', b'9'],
        b'7' => &[b'4', b'5', b'8'],
        b'8' => &[b'4', b'5', b'6', b'7', b'9'],
        b'9' => &[b'5', b'6', b'8'],
        _ => &[],
    }
}

impl Engine {
    /// T9 键串 → 音节 id 序列列表（DAT 制导 DFS, 预算帽 50k walk 步
    /// + cap——jianpin_dat_compose 同尺）。P1: ≤6 键整消耗域。
    pub(super) fn t9_dat_compose(
        &self,
        inp: &str,
        cap: usize,
    ) -> Vec<Vec<u32>> {
        let mut keys: Vec<u8> = inp.bytes().collect();
        if keys.is_empty()
            || !keys.iter().all(|&k| (b'2'..=b'9').contains(&k))
        {
            return Vec::new();
        }
        // P2 窗口律（>12 键: 12→11→…→6 最长有解窗口——固定截 12 会在
        // 无解键位切出空面板, 如 jintiandetianqi(15 键)的 12 键截断
        // 恰无整消耗分解而 9 键「今天的」/尾窗「…tianqi」有解;
        /// 递减首个有产物窗口止, 打字在途语义连续）
        let mut win = keys.len().min(12);
        loop {
            let t: Vec<u8> = keys[..win].to_vec();
            if !t.is_empty() && !Self::t9_compose_once(self, &t, cap).is_empty() {
                keys = t;
                break;
            }
            if win <= 6 {
                return Vec::new();
            }
            win -= 1;
        }
        self.t9_compose_once(&keys, cap)
    }

    /// 单窗口 DFS（窗口律回退的单次尝试）。
    fn t9_compose_once(
        &self,
        keys: &[u8],
        cap: usize,
    ) -> Vec<Vec<u32>> {
        self.t9_sig_build();
        let sig_map = self.t9_sig_v.borrow();
        let Some(sig_map) = sig_map.as_ref() else { return Vec::new() };
        let keys = keys.to_vec();
        let mut out = Vec::new();
        // 预算帽: 50k(6 键内富余)对 12 键深层路径不足——枚举树中途节点
        // 远多于终点, 浅层短段组合先烧预算。500k walk 步（内存查, 实测
        // 毫秒级）罩住 12 键最坏 2,131 分解的探索树
        let mut budget = 500_000usize;
        t9_dfs(
            self, sig_map, &keys, 0, &mut String::new(), &mut Vec::new(),
            &mut out, &mut budget, cap,
        );
        drop(sig_map);
        out
    }

    /// 签名倒排构建（首查构建, sylw 权重降序稳定同表序——
    /// jianpin_dat_compose 的 by_ini 同律）
    fn t9_sig_build(&self) {
        let mut cache = self.t9_sig_v.borrow_mut();
        if cache.is_some() {
            return;
        }
        let mut m: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, s) in self.syllables.iter().enumerate() {
            if let Some(sig) = t9_sig(s) {
                m.entry(sig).or_default().push(i as u32);
            }
        }
        for v in m.values_mut() {
            v.sort_by(|&a, &b| self.sylw[b as usize].cmp(&self.sylw[a as usize]));
        }
        *cache = Some(m);
    }

    /// T9 查询入口（P3: 主查 + 零产出时九宫邻键纠错）。
    /// 门控（设计稿 §5-P3③ fuzz_gate_auto 同哲学）: 健康输入（主查有
    /// 产出）直通零回归; 仅空面板时枚举单替换变体（每位×几何邻键,
    /// 1 编辑 × TYPO_PENALTY=0.3 罚分——typo_retry.rs:589 同律）。
    pub(super) fn q_t9_typo(&self, inp: &str) -> Vec<Cand> {
        let direct = self.q_t9(inp);
        if !direct.is_empty() {
            return direct;
        }
        let keys: Vec<u8> = inp.bytes().collect();
        // 变体去重（同串可经多位替换重合）
        let mut seen: std::collections::HashSet<Vec<u8>> =
            std::collections::HashSet::new();
        let mut pool: Vec<(Cand, u16)> = Vec::new();
        for i in 0..keys.len() {
            for &nb in t9_neighbors(keys[i]) {
                if !(b'2'..=b'9').contains(&nb) {
                    continue; // 邻键 1（非字母域）不入变体
                }
                let mut v = keys.clone();
                v[i] = nb;
                if !seen.insert(v.clone()) {
                    continue;
                }
                for (mut c, fv) in
                    self.q_t9_scored(&String::from_utf8_lossy(&v))
                {
                    c.score -= 0.3; // 1 编辑罚分
                    pool.push((c, fv));
                }
            }
        }
        // 跨变体全局 fv 序（词频序）; 罚分统一平移不改序——去重词面保首见
        let mut seen_w: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        pool.retain(|(c, _)| seen_w.insert(c.word.clone()));
        pool.sort_by(|a, b| a.1.cmp(&b.1));
        pool.into_iter().map(|(c, _)| c).collect()
    }

    /// T9 查询（P1: 单/双音节域）——q_jianpin 消费口尾段同构。
    pub(super) fn q_t9(&self, inp: &str) -> Vec<Cand> {
        self.q_t9_scored(inp)
            .into_iter()
            .map(|(c, _)| c)
            .collect()
    }

    /// q_t9 带 fv 伴行（P3 纠错合并需跨变体全局频序——Cand 无 fv
    /// 字段, 伴行元组返回）。
    fn q_t9_scored(&self, inp: &str) -> Vec<(Cand, u16)> {
        let exps = self.t9_dat_compose(inp, 48);
        let mut out: Vec<(Cand, u16)> = Vec::new();
        let mut seen_rid: std::collections::HashSet<u32> =
            std::collections::HashSet::new();
        const PER_READING: usize = 8;
        // P2 修正: score 全 0 平票会让枚举序冒充排序（中国输给中火——
        // huo 桶先枚举）。收集后按 fv（频次**排名**, 小值=高频——
        // range_candidates 内部 Reverse 堆同语义）升序重排; 词级去重
        // （put 的去重仅在单 range 内, 跨展开路径会重）
        let mut pool: Vec<(u32, u32, u16)> = Vec::new(); // (w, rid, fv)
        let mut seen_w: std::collections::HashSet<u32> =
            std::collections::HashSet::new();
        for ids in &exps {
            let key = ids
                .iter()
                .map(|&i| self.syllables[i as usize].clone())
                .collect::<String>();
            let st = self.dat.walk(key.as_bytes(), ROOT);
            if st < 0 {
                continue;
            }
            let (lo, cnt) = self.dat.range(st);
            if seen_rid.contains(&lo) && cnt <= 1 {
                continue;
            }
            seen_rid.insert(lo);
            let mut got = if cnt > 1 {
                self.range_candidates(lo, cnt, 48)
            } else {
                Vec::new()
            };
            if got.is_empty() {
                let (o0, o1) = self.reading_slice(lo);
                for oi in o0..o1.min(o0 + PER_READING) {
                    let w = self.wlist_at(oi);
                    got.push((w, lo, self.fv_of(w, oi)));
                }
            }
            for (w, r, fv) in got {
                if seen_w.insert(w) {
                    pool.push((w, r, fv as u16));
                }
            }
        }
        pool.sort_by(|a, b| a.2.cmp(&b.2));
        // t9_lr 开源重排（t9_coef 自训等价物, 2026-11-20: 常用词单
        // 6000 组 pairwise LR, +4.1pp top1 over 纯 fv 序; 特征 =
        // [fv, fv序名次, 音节数, 键长, 词长, logp, 整消耗位]——rank
        // 特征以 fv 序为基（先 fv 排再取 index）, 与训练同律）。
        // 常量=data/t9_lr_open.json（z-score 标准化域）。
        pool = t9_lr_rerank(self, inp, pool);
        // T9_DUMP 采集旋钮（t9_coef 自训: inp word fv rank syl_n
        // key_len word_len logp is_exact_len——路径/词级特征族;
        // 标签由离线词单对齐）
        if let Ok(path) = std::env::var("T9_DUMP") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true).append(true).open(path)
            {
                for (rk, (w, r, fv)) in pool.iter().enumerate() {
                    let wt = self.word_text(*w);
                    let logp = self.freq_of(*w);
                    let rd = self.rid_key(*r);
                    let syl_n = rd.split('\'').count().max(1);
                    let _ = writeln!(
                        f,
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        inp, wt, fv, rk, syl_n, inp.len(),
                        wt.chars().count(), logp,
                        (rd.replace('\'', "").len() == inp.len()) as u8,
                    );
                }
            }
        }
        let mut out = Vec::new();
        for (w, r, fv) in pool {
            out.push((
                self.mk(w, r as i64, Kind::Jianpin, 0, None, 0.0, Some(fv)),
                fv,
            ));
        }
        out
    }
}

/// T9 DFS（jianpin_dfs 同构; 差异: 每步消费一个键段 l∈1..=6 而非单字母,
/// 段签名查倒排表得音节组）。DAT walk 剪枝: 拼一个音节续走一次,
/// 死态（walk<0）即剪。
fn t9_dfs(
    e: &Engine,
    sig_map: &HashMap<String, Vec<u32>>,
    keys: &[u8],
    pos: usize,
    prefix: &mut String,
    ids: &mut Vec<u32>,
    out: &mut Vec<Vec<u32>>,
    budget: &mut usize,
    cap: usize,
) {
    if out.len() >= cap || *budget == 0 {
        return;
    }
    if pos == keys.len() {
        out.push(ids.clone());
        return;
    }
    // 键段长度 1..=6（最长音节 zhuang/chuang 六键; 剩余不足截断）
    let max_l = (keys.len() - pos).min(6);
    for l in 1..=max_l {
        let sig = std::str::from_utf8(&keys[pos..pos + l])
            .unwrap_or("")
            .to_string();
        let Some(cands) = sig_map.get(&sig) else { continue };
        for &sid in cands {
            if *budget == 0 || out.len() >= cap {
                return;
            }
            *budget -= 1;
            let syl = &e.syllables[sid as usize];
            let pre = prefix.len();
            prefix.push_str(syl);
            if e.dat.walk(prefix.as_bytes(), ROOT) >= 0 {
                ids.push(sid);
                t9_dfs(e, sig_map, keys, pos + l, prefix, ids, out, budget, cap);
                ids.pop();
            }
            prefix.truncate(pre);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mapping_full_alphabet() {
        assert_eq!(key_of_letter(b'a'), Some(b'2'));
        assert_eq!(key_of_letter(b'c'), Some(b'2'));
        assert_eq!(key_of_letter(b'z'), Some(b'9'));
        assert_eq!(key_of_letter(b'q'), Some(b'7'));
        assert_eq!(key_of_letter(b'1'), None);
        assert_eq!(key_of_letter(b'0'), None);
        // 26 字母全覆盖
        for c in b'a'..=b'z' {
            assert!(key_of_letter(c).is_some(), "{}", c as char);
        }
    }

    #[test]
    fn neighbors_geometry() {
        // 对称性: k 在 n 的邻集 ⟺ n 在 k 的邻集（几何邻接律）
        for k in b'2'..=b'9' {
            for &n in t9_neighbors(k) {
                assert!(
                    (b'2'..=b'9').contains(&n),
                    "{} 的邻键 {} 应在字母域",
                    k as char, n as char
                );
                assert!(
                    t9_neighbors(n).contains(&k),
                    "{}↔{} 邻接不对称",
                    k as char, n as char
                );
            }
        }
        // 角键 3 邻集 = {2,5,6}（无上/右）
        assert_eq!(t9_neighbors(b'3'), &[b'2', b'5', b'6']);
        // 中心键 5 = 全 8 方（四正交 + 四对角, 字母域内）
        assert_eq!(
            t9_neighbors(b'5'),
            &[b'2', b'3', b'4', b'6', b'7', b'8', b'9']
        );
    }

    #[test]
    fn sig_examples() {
        assert_eq!(t9_sig("ni").as_deref(), Some("64"));
        assert_eq!(t9_sig("qing").as_deref(), Some("7464"));
        assert_eq!(t9_sig("zhuang").as_deref(), Some("948264"));
        assert_eq!(t9_sig("a").as_deref(), Some("2"));
    }
}

/// t9_lr 重排（开源 t9_coef: 7 特征标准化 × LR 线性分降序, fv 序
/// tie-break）。特征构造与 T9_DUMP 采集逐项同律。
fn t9_lr_rerank(
    e: &Engine,
    inp: &str,
    pool: Vec<(u32, u32, u16)>,
) -> Vec<(u32, u32, u16)> {
    const COEF: [f64; 7] = [
        -0.03705027800899863,
        -0.23987382191284673,
        0.0,
        0.0,
        -2.012413021321953,
        -0.0370358411314738,
        -0.6225046105195655,
    ];
    const MU: [f64; 7] = [
        50751.01154779814, 96.11163156455142, 1.0, 5.563089783916849,
        3.296917737964989, 50750.909540823304, 0.004051559080962801,
    ];
    const SD: [f64; 7] = [
        316.4114755995142, 84.9809841098636, 1.0, 1.662225978074511,
        0.8009294847587645, 316.53481541220344, 0.06352278392060588,
    ];
    let mut scored: Vec<(f64, u16, (u32, u32, u16))> = pool
        .into_iter()
        .enumerate()
        .map(|(rk, t)| {
            let (w, r, fv) = t;
            let wt = e.word_text(w);
            let rd = e.rid_key(r);
            let syl_n = rd.split('\'').count().max(1) as f64;
            let exact =
                (rd.replace('\'', "").len() == inp.len()) as u8 as f64;
            let f = [
                fv as f64,
                rk as f64,
                syl_n,
                inp.len() as f64,
                wt.chars().count() as f64,
                e.freq_of(w) as f64,
                exact,
            ];
            let mut s = 0.0;
            for i in 0..7 {
                s += COEF[i] * (f[i] - MU[i]) / SD[i];
            }
            (s, fv, (w, r, fv))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    scored.into_iter().map(|x| x.2).collect()
}
