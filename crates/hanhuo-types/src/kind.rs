//! 候选来源类型（native Kind 直译）.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Exact,
    Complete,
    Comp,
    Sentence,
    Path,
    Assoc,
    Correct,
    C4G,
    Pending,
    Jianpin,
    User,
    /// 役 F1（agentR 件 3）: 字母直通候选（Buxuo 型; native agentL #22 n=1）
    Latin,
    /// 役 K（agentZ）: G1 兜底字列（首音节列/首声母合并列, 免 W14）
    Fbcol,
    /// 役 K（agentZ）: G2/G4 模糊组合词面（绕开 comp 压罚/配额, 免 W14）
    Fcomp,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Exact => "exact",
            Kind::Complete => "complete",
            Kind::Comp => "comp",
            Kind::Sentence => "sentence",
            Kind::Path => "path",
            Kind::Assoc => "assoc",
            Kind::Correct => "correct",
            Kind::C4G => "c4g",
            Kind::Pending => "pending",
            Kind::Jianpin => "jianpin",
            Kind::User => "user",
            Kind::Latin => "latin",
            Kind::Fbcol => "fbcol",
            Kind::Fcomp => "fcomp",
        }
    }
}
