//! HBWC 跨语言布局锁（v2 默认 + v1 遗留双路径）: fixture 由
//! tools/dict_build.py 生成, 此处用 Rust from_file 解析——锁 python
//! writer 与 rust reader 的字节镜像（2026-11-08 漂移事故: key_head
//! 14B vs 12B panic; 2026-11-10 v2 瘦身后 fixture 再生, v1 件留档
//! domain_fixture_v1.hbwc 供 hbwc_v1_legacy_read）。
//!
//! fixture 内容（4 键 5 词）: 同键多词频序（ceshi）、前缀共享键
//! （ceshi/ceshikaifa）、四音节长键（zhizhenyuejie）、非 ASCII 词面（β）。
use hanhuo_dict::bwcn::BwcnTriples;

fn fixture() -> BwcnTriples {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/domain_fixture.hbwc");
    BwcnTriples::from_file(&p).expect("fixture 解析（布局漂移则此处炸）")
}

#[test]
fn hbwc_fixture_keys_and_counts() {
    let b = fixture();
    assert_eq!(b.len(), 4, "键数: ceshi/ceshikaifa/shoutizuzhiji/zhizhenyuejie");
    assert_eq!(b.map.values().map(|v| v.len()).sum::<usize>(), 5, "词条数");
}

#[test]
fn hbwc_fixture_same_key_freq_order() {
    let b = fixture();
    let v = b.lookup("ceshi", 4);
    let texts: Vec<&str> = v.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, vec!["测试", "侧试"], "同键多词频序降序（writer 按频排序写入, reader 保 entries 序）");
    assert_eq!(v[0].weight, 900);
    assert_eq!(v[1].weight, 400);
    assert_eq!(v[0].pinyin, "ce'shi", "键级拼音（撇号形）");
}

#[test]
fn hbwc_fixture_prefix_key_isolation() {
    // 前缀共享键: key_idx 偏移不串（漂移事故里错位偏移读出 65536）
    let b = fixture();
    let v = b.lookup("ceshikaifa", 4);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].text, "测试开发");
    assert_eq!(v[0].pinyin, "ce'shi'kai'fa");
}

#[test]
fn hbwc_fixture_long_key_and_nonascii_word() {
    let b = fixture();
    let v = b.lookup("zhizhenyuejie", 4);
    assert_eq!((v[0].text.as_str(), v[0].weight), ("指针越界", 800), "四音节长键（pl/entries 偏移覆盖文件尾）");
    let v = b.lookup("shoutizuzhiji", 4);
    assert_eq!(v[0].text, "β受体阻滞剂", "非 ASCII 词面 UTF-8 往返（len 字节形）");
}

#[test]
fn hbwc_fixture_miss_and_cap() {
    let b = fixture();
    assert!(b.lookup("ceshix", 4).is_empty(), "未命中键空返");
    assert_eq!(b.lookup("ceshi", 1).len(), 1, "cap 截断");
}

/// v1 遗留读路径（旧包兼容——bwcnmac.hbwc 等存量 v1 包; v2 为新默认）
#[test]
fn hbwc_v1_legacy_read() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/domain_fixture_v1.hbwc");
    let b = BwcnTriples::from_file(&p).expect("v1 fixture 解析");
    assert_eq!(b.len(), 4);
    let v = b.lookup("ceshi", 4);
    let texts: Vec<&str> = v.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, vec!["测试", "侧试"], "v1 同键频序");
    assert_eq!(v[0].weight, 900, "v1 weight u32 原值");
    assert_eq!(b.lookup("shoutizuzhiji", 4)[0].text, "β受体阻滞剂");
}
