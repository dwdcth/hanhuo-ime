//! 双拼前置键位解映射（backlog 输入方案扩展; L1 前双拼→全拼, 引擎零改动）
//!
//! 小鹤双拼（flypy）——每音节恒两键, 无切分歧义:
//! - 声母: zh→v ch→i sh→u, 其余原键; 零声母见 ZERO 表
//! - 韵母（声母后第二键）: iu=q ei=w e=e uan/van=r ue/ve=t un/vn=y
//!   u=u i=i uo/o=o（o 声组 w/b/p/m/f 后为单 o） ie=p a=a
//!   ong/iong=s ai=d en=f eng=g ang=h an=j uai/ing=k uang/iang=l
//!   ua/ia=x ao=c ui=v in=b iao=n ian=m ou=z
//! - 零声母: 单字母韵母双击（aa/ee/oo）; 双字母韵母全拼照打
//!   （ai/ei/ao/ou/an/en/er）; 三字母取首字母+韵母键（ah=ang eg=eng）
//!
//! 表源: shuangpin.app 官方小鹤布局 + ipkk 双拼练习器零声母口径
//! （华宇改编 gist 的 ui=W 不采）。悬空尾键（在途输入）按声母原样
//! 透传（引擎前缀匹配兜底）; ü 族（nv/lv/nve/lve）按引擎 v 形归一。
//!
//! 用法: hanhuo query "nihc" --sp xiaohe  →  引擎收到 "nihao"

/// 韵母键位表（小鹤; 第二键 → 韵母, 多值并列）
/// 表源: shuangpin.app 官方小鹤布局（ui=v in=b ian=m iao=n——华宇改编
/// gist 的 ui=W 系笔误不采）+ ipkk 双拼练习器零声母口径
/// 全韵母键位（每键多韵母——共键族: ing/uai=k iang/uang=l ong/iong=s
/// ua/ia=x uan/van=r ue/ve=t un/vn=y uo/o=o）。demux 方向消歧靠音节
/// 验证（ging 非法 → k 取 uai）; 无验证时取首值。
const XIAOHE_KEYS: &[(&str, &[&str])] = &[
    ("q", &["iu"]), ("w", &["ei"]), ("e", &["e"]),
    ("r", &["uan", "van"]), ("t", &["ue", "ve", "v"]),
    ("y", &["un", "vn"]), ("u", &["u"]), ("i", &["i"]),
    ("o", &["uo", "o"]), ("p", &["ie"]), ("a", &["a"]),
    ("s", &["ong", "iong"]), ("d", &["ai"]), ("f", &["en"]),
    ("g", &["eng"]), ("h", &["ang"]), ("j", &["an"]),
    ("k", &["ing", "uai"]), ("l", &["uang", "iang"]),
    ("x", &["ua", "ia"]), ("c", &["ao"]), ("v", &["ui"]),
    ("b", &["in"]), ("n", &["iao"]), ("z", &["ou"]),
    ("m", &["ian"]),
];

/// 自然码韵母键位（ulpb.app 自然码键位图 + rime ziranma algebra 交叉;
/// 与小鹤同: zh=v ch=i sh=u, 零声母表, o 声组特则。差异: ing/uai=y
/// iang/uang=d ei=z ie=x iao=c in=n ian=m ou=b ua/ia=w un=p uan=r
/// üe=t iu=q ao=k ai=l en=f eng=g ang=h an=j ong/iong=s ü 在 V）
const ZIRANMA_KEYS: &[(&str, &[&str])] = &[
    ("q", &["iu"]), ("w", &["ia", "ua"]), ("e", &["e"]),
    ("r", &["uan", "van"]), ("t", &["ue", "ve", "v"]),
    ("y", &["ing", "uai"]), ("u", &["u"]), ("i", &["i"]),
    ("o", &["uo", "o"]), ("p", &["un", "vn"]), ("a", &["a"]),
    ("s", &["ong", "iong"]), ("d", &["iang", "uang"]),
    ("f", &["en"]), ("g", &["eng"]), ("h", &["ang"]), ("j", &["an"]),
    ("k", &["ao"]), ("l", &["ai"]), ("z", &["ei"]), ("x", &["ie"]),
    ("c", &["iao"]), ("v", &["ui", "v"]), ("b", &["ou"]),
    ("n", &["in"]), ("m", &["ian"]),
];

/// 零声母双键表（小鹤官方规则三式）
const ZERO: &[(&str, &str)] = &[
    ("aa", "a"), ("ai", "ai"), ("an", "an"), ("ao", "ao"), ("ah", "ang"),
    ("ee", "e"), ("ei", "ei"), ("en", "en"), ("eg", "eng"), ("er", "er"),
    ("oo", "o"), ("ou", "ou"),
];

/// ü 归一: 引擎音节表用 v 形（nv/lv/nve/lve）, 双拼韵母 ve/ue 出 e 形
fn vn_form(syl: &str) -> String {
    match syl {
        // 引擎 ü 形为单 v（lv/nv）
        "nve" | "nue" | "nv" => "nv".into(),
        "lve" | "lue" | "lv" => "lv".into(),
        "jvan" | "juan" => "jvan".into(),
        "qvan" | "quan" => "qvan".into(),
        "xvan" | "xuan" => "xvan".into(),
        "yun" | "yvn" => "yvn".into(),
        "jun" | "jvn" => "jvn".into(),
        "qun" | "qvn" => "qvn".into(),
        "xun" | "xvn" => "xvn".into(),
        _ => syl.to_string(),
    }
}

/// 双拼方案
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpScheme {
    /// 小鹤双拼
    Xiaohe,
    /// 自然码（经典母本方案; iOS16+ 内置「常用双拼」即此）
    Ziranma,
}

impl SpScheme {
    /// 韵母键位表（声母映射与零声母表两方案全同: zh=v ch=i sh=u）
    fn keys(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            SpScheme::Xiaohe => XIAOHE_KEYS,
            SpScheme::Ziranma => ZIRANMA_KEYS,
        }
    }
}

/// 小鹤双拼 → 全拼（无验证版: 共键族取首值——ging/guai 型歧义见
/// [`demux_validated`]）。非法键序原样透传两键; 悬空尾键按声母透传。
pub fn demux(inp: &str, scheme: SpScheme) -> String {
    demux_inner(inp, scheme, &|_| false)
}

/// 带音节验证的解映射: 共键族（ing/uai=k 等）候选逐个试 `is_syl`,
/// 首个合法者胜（ging 非法 → k 取 uai; 无一合法退回首值）。CLI 传
/// 引擎音节表集合——消歧即查表, 零启发。
pub fn demux_validated(
    inp: &str,
    scheme: SpScheme,
    is_syl: &dyn Fn(&str) -> bool,
) -> String {
    demux_inner(inp, scheme, is_syl)
}

fn demux_inner(
    inp: &str,
    _scheme: SpScheme,
    is_syl: &dyn Fn(&str) -> bool,
) -> String {
    let b: Vec<u8> = inp.bytes().collect();
    let mut out = String::new();
    let mut i = 0usize;
    let initial_of = |c: u8| -> String {
        match c {
            b'v' => "zh".into(),
            b'i' => "ch".into(),
            b'u' => "sh".into(),
            _ => (c as char).to_string(),
        }
    };
    let keys = _scheme.keys();
    let finals_of = |c: u8| -> Option<&'static [&'static str]> {
        keys.iter()
            .find(|(key, _)| key.as_bytes()[0] == c)
            .map(|(_, fs)| *fs)
    };
    while i < b.len() {
        let c1 = b[i];
        // 非字母透传（空格=软分隔/撇号, 与引擎输入白名单同律）
        if !c1.is_ascii_lowercase() {
            out.push(c1 as char);
            i += 1;
            continue;
        }
        // 零声母: a/e/o 起头
        if matches!(c1, b'a' | b'e' | b'o') {
            if i + 1 < b.len() {
                let two: &[u8] = &b[i..i + 2];
                let two_s = String::from_utf8_lossy(two).to_string();
                if let Some((_, full)) =
                    ZERO.iter().find(|(k, _)| *k == two_s)
                {
                    out.push_str(full);
                    i += 2;
                    continue;
                }
            } else {
                // 悬空元音: 透传
                out.push(c1 as char);
                i += 1;
                continue;
            }
        }
        if i + 1 >= b.len() {
            // 悬空尾键: 按声母透传（引擎前缀匹配兜底）
            out.push_str(&initial_of(c1));
            break;
        }
        let c2 = b[i + 1];
        let ini = initial_of(c1);
        let Some(fins) = finals_of(c2) else {
            // 非法第二键: 原样透传两键（保底）
            out.push(c1 as char);
            out.push(c2 as char);
            i += 2;
            continue;
        };
        // 候选序列: o 声组把单 o 提前; 其余按键表序（首值优先）
        let mut cands: Vec<String> = fins
            .iter()
            .map(|f| format!("{ini}{f}"))
            .collect();
        if c2 == b'o' && matches!(c1, b'w' | b'b' | b'p' | b'm' | b'f' | b'y') {
            cands.insert(0, format!("{ini}o"));
        }
        let pick = cands
            .iter()
            .find(|c| is_syl(c))
            .unwrap_or(&cands[0]);
        out.push_str(&vn_form(pick));
        i += 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// basics 用微型验证器（共键族消歧需要「合法音节」判定; 真机全量
    /// 往返测试用引擎 409 表, 此处列测试涉及的音节即可）
    const TEST_SYLS: &[&str] = &[
        "ni", "hao", "zhao", "zhuang", "shui", "a", "ai", "ang", "e",
        "er", "o", "jin", "tian", "qi", "bu", "cuo", "miao", "mian",
        "wo", "duo", "qing", "shuang", "pin", "xiao", "shi", "jing",
        "hei", "nv", "lv", "guai", "niao",
    ];
    fn v(s: &str) -> bool {
        TEST_SYLS.contains(&s)
    }

    #[test]
    fn xiaohe_basics() {
        assert_eq!(demux_validated("nihc", SpScheme::Xiaohe, &v), "nihao");
        assert_eq!(demux_validated("vc", SpScheme::Xiaohe, &v), "zhao");
        assert_eq!(demux_validated("vl", SpScheme::Xiaohe, &v), "zhuang");
        assert_eq!(demux_validated("uv", SpScheme::Xiaohe, &v), "shui"); // sh=u + ui=v
        assert_eq!(demux_validated("ni", SpScheme::Xiaohe, &v), "ni");
        // 零声母三式
        assert_eq!(demux_validated("aa", SpScheme::Xiaohe, &v), "a");
        assert_eq!(demux_validated("ai", SpScheme::Xiaohe, &v), "ai");
        assert_eq!(demux_validated("ah", SpScheme::Xiaohe, &v), "ang");
        assert_eq!(demux_validated("ee", SpScheme::Xiaohe, &v), "e");
        assert_eq!(demux_validated("er", SpScheme::Xiaohe, &v), "er");
        assert_eq!(demux_validated("oo", SpScheme::Xiaohe, &v), "o");
        // 复合句: 今天天气不错 = jbtmtmqibuco
        assert_eq!(
            demux_validated("jbtmtmqibuco", SpScheme::Xiaohe, &v),
            "jintiantianqibucuo"
        );
        // in/ian/ui/iao 键位: 进=jb 天=tm 水=uv 描=mn 面=mm
        assert_eq!(demux_validated("jb", SpScheme::Xiaohe, &v), "jin");
        assert_eq!(demux_validated("tm", SpScheme::Xiaohe, &v), "tian");
        assert_eq!(demux_validated("uv", SpScheme::Xiaohe, &v), "shui"); // sh=u + ui=v
        assert_eq!(demux_validated("mn", SpScheme::Xiaohe, &v), "miao"); // iao=n
        assert_eq!(demux_validated("mm", SpScheme::Xiaohe, &v), "mian"); // ian=m
        // o 声组: 我=wo（非 wuo）多=duo
        assert_eq!(demux_validated("wo", SpScheme::Xiaohe, &v), "wo");
        assert_eq!(demux_validated("do", SpScheme::Xiaohe, &v), "duo");
        // 悬空尾键
        assert_eq!(demux_validated("ni h", SpScheme::Xiaohe, &v), "ni h");
    }

    #[test]
    fn ziranma_basics() {
        // 表源交叉: v2ex 实打用例 udpn=双拼, xcui=小事, qy=请
        assert_eq!(demux_validated("udpn", SpScheme::Ziranma, &v), "shuangpin");
        assert_eq!(demux_validated("xcui", SpScheme::Ziranma, &v), "xiaoshi");
        assert_eq!(demux_validated("qy", SpScheme::Ziranma, &v), "qing");
        assert_eq!(demux_validated("vd", SpScheme::Ziranma, &v), "zhuang");
        assert_eq!(demux_validated("nihk", SpScheme::Ziranma, &v), "nihao");
        // ing=y / in=n / ei=z / ian=m: 请=jy 进=jn 黑=hz 面=mm
        assert_eq!(demux_validated("jy", SpScheme::Ziranma, &v), "jing");
        assert_eq!(demux_validated("jn", SpScheme::Ziranma, &v), "jin");
        assert_eq!(demux_validated("hz", SpScheme::Ziranma, &v), "hei");
        assert_eq!(demux_validated("mm", SpScheme::Ziranma, &v), "mian");
        // ü 在 V: 女=nv 绿=lv
        assert_eq!(demux_validated("nv", SpScheme::Ziranma, &v), "nv");
        assert_eq!(demux_validated("lv", SpScheme::Ziranma, &v), "lv");
        // 零声母（与小鹤同表）
        assert_eq!(demux_validated("aa", SpScheme::Ziranma, &v), "a");
        assert_eq!(demux_validated("ah", SpScheme::Ziranma, &v), "ang");
    }

    /// 引擎音节表全量往返（真机）: 每音节的 2 键编码 demux 回自身
    #[test]
    #[ignore = "真机 wxq.bin（HANHUO_BIN / 默认路径）"]
    fn roundtrip_all_syllables() {
        let bin = std::env::var("HANHUO_BIN").unwrap_or_else(|_| {
            format!(
                "{}/tmp/hanhuo/wxq.bin",
                std::env::var("HOME").unwrap_or_default()
            )
        });
        let Ok(e) =
            hanhuo_core::Engine::open(std::path::Path::new(&bin))
        else {
            return;
        };
        let syls = e.syllables_public();
        let sylset: std::collections::HashSet<String> =
            syls.iter().cloned().collect();
        let is_syl = move |s: &str| sylset.contains(s);
        assert!(!syls.is_empty());
        let ini_of = |s: &str| -> String {
            if s.starts_with("zh") {
                "v".into()
            } else if s.starts_with("ch") {
                "i".into()
            } else if s.starts_with("sh") {
                "u".into()
            } else {
                s.chars().next().unwrap().to_string()
            }
        };
        let key_of_final =
            |s: &str, scheme: SpScheme| -> Option<String> {
            // 韵母 = 音节去掉声母; ü 归一后查表（一音节一韵母键,
            // ong/iong、uai/ing 等共键组按全拼形式命中任一即可）
            let ini_len = if s.starts_with("zh")
                || s.starts_with("ch")
                || s.starts_with("sh")
            {
                2
            } else {
                1
            };
            let fin = &s[ini_len..];
            // 全韵母反查（方案键位表——每键多韵母）
            for (k, fs) in scheme.keys() {
                if fs.contains(&fin) {
                    return Some(k.to_string());
                }
            }
            None
        };
        for scheme in [SpScheme::Xiaohe, SpScheme::Ziranma] {
        let mut fails: Vec<String> = Vec::new();
        for s in syls {
            // 零声母域走 ZERO 表（demux 已覆盖; 编码方向在此跳过）
            if matches!(
                s.as_str(),
                "a" | "ai" | "an" | "ang" | "ao" | "e" | "ei" | "en"
                    | "er" | "o" | "ou"
            ) {
                let enc = match s.as_str() {
                    "a" => "aa", "e" => "ee", "o" => "oo",
                    "ang" => "ah", "eng" => "eg",
                    other => other, // 双字母韵母全拼照打
                };
                let dec = demux_validated(enc, scheme, &is_syl);
                if dec != *s {
                    fails.push(format!("{s} → {enc} → {dec}"));
                }
                continue;
            }
            // lo: 小鹤 2 键域不可达（l+o=luo; 咯 需全拼混打）——排除注记
            if s == "lo" {
                continue;
            }
            let ini = ini_of(s);
            let Some(k) = key_of_final(s, scheme) else {
                fails.push(format!("{s}: 韵母无键"));
                continue;
            };
            let enc = format!("{ini}{k}");
            let dec = demux_validated(&enc, scheme, &is_syl);
            let dec_vn = vn_form(&dec);
            let s_vn = vn_form(s);
            if dec_vn != s_vn {
                fails.push(format!("{s} → {enc} → {dec}"));
            }
        }
        assert!(
            fails.is_empty(),
            "{scheme:?} 往返失败 {} 例: {fails:?}",
            fails.len()
        );
        }
    }
}
