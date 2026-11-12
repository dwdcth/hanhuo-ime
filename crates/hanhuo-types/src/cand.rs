//! 候选结构（native Cand 直译）.

use crate::kind::Kind;

#[derive(Clone, Debug)]
pub struct Cand {
    pub word: String,
    pub reading: String, // 展示形 "ni'hao"
    pub logp: f64,       // 词频层 −log10P(w)
    pub score: f64,      // final
    pub kind: Kind,
    pub lm: Option<f64>, // bigram log10P(w|ctx)
    pub nn_n: Option<f64>,
    pub wid: i64,
    pub extra_syl: i32,
    pub edit_penalty: f64,
    pub score_bias: f64,
    /// 用户词学习加成（B4）: 评分层并入（同 score_bias 路径）, 且
    /// nn/ltr 重排后重加——重排重写 score 会洗掉学习信号（原生走
    /// mdltr 槽 19/20 特征, 我们无该通道, 后置补加对齐行为）
    pub user_bias: f64,
    pub lp_locked: bool, // v2: per-(读音,词) 频次已定 lp
    pub ltr: Option<f64>, // mdltr LightGBM raw score
    pub path_words: Option<Vec<String>>,
    /// py Cand._fz_face: 面板域 fz 面（panel pin 时抵 edit_penalty）
    pub fz_face: bool,
    /// py Cand._mhead: 并槽面头（今天的\x01的; fz 同界面顶替判据）
    pub mhead: Option<String>,
    /// py Cand._dgl_pin: 悬空校准钉分块（免 DANGLING_COMPLETION_BOOST 覆写）
    pub dgl_pin: bool,
}

impl Cand {
    pub fn new(word: String, logp: f64, kind: Kind) -> Cand {
        Cand {
            word,
            reading: String::new(),
            logp,
            score: 0.0,
            kind,
            lm: None,
            nn_n: None,
            wid: -1,
            extra_syl: 0,
            edit_penalty: 0.0,
            score_bias: 0.0,
            user_bias: 0.0,
            lp_locked: false,
            ltr: None,
            path_words: None,
            fz_face: false,
            mhead: None,
            dgl_pin: false,
        }
    }
}
