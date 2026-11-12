//! L1 音节切分：音节 DAG + Viterbi（unigram 先验 = SYLW 音节权重）.
//!
//! 409 音节表（slwd id 空间，0=a … 408=zuo）。切分歧义（xi'an/xian）走 DAG 全路径。
use std::collections::HashMap;

pub struct SyllableSeg {
    pub syls: Vec<String>,
    pub max_len: usize,
    logw: Vec<f64>,
    sylidx: HashMap<String, u32>,
}

impl SyllableSeg {
    pub fn new(syls: Vec<String>, weights: &[u32]) -> SyllableSeg {
        let n = syls.len();
        let tot: u64 = weights.iter().map(|&w| w as u64).sum();
        let tot = if tot > 0 {
            tot as f64
        } else {
            n as f64
        };
        let logw: Vec<f64> = (0..n)
            .map(|i| {
                let w = weights.get(i).copied().unwrap_or(0);
                let w = if w > 0 { w as f64 } else { 1.0 };
                (w / (tot + n as f64)).ln()
            })
            .collect();
        let mut sylidx = HashMap::with_capacity(n);
        for (i, s) in syls.iter().enumerate() {
            sylidx.insert(s.clone(), i as u32);
        }
        let max_len = syls.iter().map(|s| s.len()).max().unwrap_or(1);
        SyllableSeg {
            syls,
            max_len,
            logw,
            sylidx,
        }
    }

    #[inline]
    pub fn is_syl(&self, s: &str) -> bool {
        self.sylidx.contains_key(s)
    }

    /// 最优音节路径（多条并列取首个）；score = Σ log w(音节)
    pub fn viterbi(&self, s: &str) -> Option<(Vec<String>, f64)> {
        if s.is_empty() {
            return Some((Vec::new(), 0.0));
        }
        let b = s.as_bytes();
        let n = b.len();
        let neg = f64::NEG_INFINITY;
        let mut best = vec![neg; n + 1];
        let mut prev: Vec<Option<(usize, String)>> = vec![None; n + 1];
        best[0] = 0.0;
        for i in 0..n {
            if best[i] == neg {
                continue;
            }
            for l in 1..=self.max_len.min(n - i) {
                let sub = &s[i..i + l];
                if let Some(idx) = self.sylidx.get(sub) {
                    let j = i + l;
                    let sc = best[i] + self.logw[*idx as usize];
                    if sc > best[j] {
                        best[j] = sc;
                        prev[j] = Some((i, sub.to_string()));
                    }
                }
            }
        }
        if best[n] == neg {
            return None;
        }
        let mut path = Vec::new();
        let mut i = n;
        while i > 0 {
            let (p, sub) = prev[i].clone().unwrap();
            path.push(sub);
            i = p;
        }
        path.reverse();
        Some((path, best[n]))
    }

    /// 是否存在完整音节切分（BFS 可达）
    pub fn segmentable(&self, s: &str) -> bool {
        if s.is_empty() {
            return true;
        }
        let n = s.len();
        let mut reach = vec![false; n + 1];
        reach[0] = true;
        for i in 0..n {
            if !reach[i] {
                continue;
            }
            for l in 1..=self.max_len.min(n - i) {
                if self.sylidx.contains_key(&s[i..i + l]) {
                    reach[i + l] = true;
                }
            }
        }
        reach[n]
    }

    /// 是否为某音节（序列）的合法前缀——渐进输入合法性
    pub fn valid_prefix(&self, s: &str) -> bool {
        if s.is_empty() {
            return true;
        }
        let n = s.len();
        let mut reach = vec![false; n + 1];
        reach[0] = true;
        for i in 0..n {
            if !reach[i] {
                continue;
            }
            for l in 1..=self.max_len.min(n - i) {
                if self.sylidx.contains_key(&s[i..i + l]) {
                    reach[i + l] = true;
                }
            }
        }
        if reach[n] {
            return true;
        }
        for i in 0..n {
            if reach[i] && self.syls.iter().any(|x| x.starts_with(&s[i..])) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(weights: &[u32]) -> SyllableSeg {
        SyllableSeg::new(
            vec!["a".into(), "ai".into(), "i".into(), "shi".into()],
            weights,
        )
    }

    #[test]
    fn syl_viterbi_length_prior_and_weight() {
        // 归一化 logw = ln(w/(Σw+n)) 恒负 → 音节数先验支配:
        // 2 音节 ai|shi 恒胜 3 音节 a|i|shi（无论 a 权重）
        let (p, sc) = seg(&[1, 1, 1, 1]).viterbi("aishi").unwrap();
        assert_eq!(p, vec!["ai".to_string(), "shi".to_string()]);
        assert!((sc - 2.0 * (1.0f64 / 8.0).ln()).abs() < 1e-12);
        let (p2, _) = seg(&[16, 1, 1, 1]).viterbi("aishi").unwrap();
        assert_eq!(p2, vec!["ai".to_string(), "shi".to_string()]);
    }

    #[test]
    fn syl_viterbi_equal_length_weight_tiebreak() {
        // 等长二义 xian: x|ian vs xi|an —— 权重决胜负
        let mk = |w: &[u32]| {
            SyllableSeg::new(
                vec!["x".into(), "xi".into(), "ian".into(), "an".into()],
                w,
            )
        };
        // 等权同分取先见（i=1 的 x|ian 先占 best[4], i=2 的 xi|an 同分不覆盖）
        let (p, _) = mk(&[1, 1, 1, 1]).viterbi("xian").unwrap();
        assert_eq!(p, vec!["x".to_string(), "ian".to_string()]);
        // xi 权重 ×16 → xi|an 反超
        let (p2, _) = mk(&[1, 16, 1, 1]).viterbi("xian").unwrap();
        assert_eq!(p2, vec!["xi".to_string(), "an".to_string()]);
    }

    #[test]
    fn syl_reachability() {
        let s = seg(&[1, 1, 1, 1]);
        // 空串: 空切分零分
        assert_eq!(s.viterbi("").unwrap(), (Vec::<String>::new(), 0.0));
        // 不可达 → None
        assert!(s.viterbi("shix").is_none());
        assert!(s.viterbi("x").is_none());
        assert!(s.segmentable("aishi"));
        assert!(!s.segmentable("shix"));
        // 前缀门: sh 是 shi 的活前缀; x 死
        assert!(s.valid_prefix("sh"));
        assert!(s.valid_prefix("aishi")); // 可整体切分也是合法前缀形态
        assert!(!s.valid_prefix("x"));
        assert!(s.is_syl("ai"));
        assert!(!s.is_syl("ax"));
    }
}
