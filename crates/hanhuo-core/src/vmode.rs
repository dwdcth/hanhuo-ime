//! V 模式（快捷工具栏）——表达式计算 + 数字格式转换，1:1 逆向自 libwxhld.dll。
//!
//! 依据（2026-10-29 役N tick26-29 战档 agentCC_fbscore/）:
//! - `session::VModeV2::Impl`（ProcessInput sub_18044DFF0 / DropLast
//!   sub_18044DF20 / SearchCandidate sub_18044E1C0; scene 门 VModeActive）
//! - 输入模型 = 10 型 PendingInput variant（SelectedText/ExactlyMatched/
//!   Deletion/Insertion/Replacement/Exchange/Separator/Cursor/PreEmit/
//!   DoublePinReplacement）
//! - .rdata 格式套件表 0xf65880..0xf65c60（vmode_spec_tables.txt）:
//!   函数表 21 项 / 星期 7 / 拾佰仟 / 天干 10 + 地支 12 + 生肖 12 /
//!   罗马运行时构造 I..XX（sub_180177810: .rdata 仅存 7 硬型
//!   III VII XII XIV XVI XVIII XIX, 余由 I/V/X 单字构造）/ 元整角分 /
//!   正则分派 8 型
//! - 役N tick49 M-V2: 多参函数族 max/avg/var/stdev/mod（参数列表语法）
//!   + panel_candidates() 面板候选（序 = native 面板序）
//! - 役N tick50 M-V3: 10 型 PendingInput 节点链输入模型（wetypex
//!   service.hpp 实证: sigPendingInputUpdated 事件载荷 = 单链表, 节点
//!   布局 type u32@+0 / next@+8 / 数据@+24(+32); run17-22 面板事件）
//!   + pending_display() 重建 raw/display 双视图与光标位
//! - 役N tick50 M-V4: 选取语义——V 模式空格 = 选取当前面板候选（不是
//!   字符键, wetypex service_select → select_candidate(engine,
//!   engineText.ptr, len, id, len, null×8)）; 候选提交即结束 V 模式
//!   （铁律原文: "Calculator candidates commit a result without
//!   consuming the expression like a pinyin candidate. End VModeV2
//!   before accepting the next key."）; 无候选时 raw 提交 =
//!   reset_session + pending 原文整段上屏
//! - 数据归腾讯，仅本地个人使用，禁止分发。

/// V 模式会话状态机（M-V1: v 进出 + 组合串追加/回退；键级语义与
/// ProcessInput 的 pending_inputs_/cursor_reverse_pos_ 对齐）
#[derive(Default)]
pub struct VMode {
    pub pending: String,
    pub active: bool,
    /// M-V3 节点链（sigPendingInputUpdated 载荷镜像；空表 = 纯字符模式）
    pub pending_nodes: Vec<PendingInput>,
    /// 已进入节点模式（首个 VKey::Pending 后置位；DropLast 改弹末节点）
    pub node_mode: bool,
}

/// PendingInput 节点——10 型（役N tick50 M-V3；wetypex service.hpp
/// sigPendingInputUpdated 事件载荷 = 单链表, 节点布局: type u32@+0 /
/// next 指针@+8 / 数据@+24(+32), run17-22 面板事件实证）。
///
/// - 0 SelectedText   选中替换（数据=串, 每字一个停点）
/// - 1 ExactlyMatched 精确匹配段（数据=串）
/// - 2 Deletion       删除（保留用户原字节@+24 供显示）
/// - 3 Insertion      插入（数据=串）
/// - 4 Replacement    替换（原字节@+24 + 合成纠错串@+32）
/// - 5 Exchange       交换（保留原字节@+24）
/// - 6 Separator      分隔符（存储字节; 显示为 '\''）
/// - 7 Cursor         光标（displayCursor 定位）
/// - 8 PreEmit        预发射（跳过）
/// - 9 DoublePinReplacement 双拼替换（原字节 + 合成串）
#[derive(Debug, Clone, PartialEq)]
pub enum PendingInput {
    /// type 0: 选中替换文本（每字符产生一个停点, 见 pending_stops）
    SelectedText(String),
    /// type 1: 精确匹配段
    ExactlyMatched(String),
    /// type 2: 删除——保留用户原字节（u8）供显示
    Deletion(u8),
    /// type 3: 插入文本
    Insertion(String),
    /// type 4: 替换——(用户原字节, 合成纠错串); 显示只取原字节
    Replacement(u8, String),
    /// type 5: 交换——保留原字节
    Exchange(u8),
    /// type 6: 分隔符——存储字节; display 恒出 '\'', 仅 == b'\'' 时进 raw
    Separator(u8),
    /// type 7: 光标节点（displayCursor 定位）
    Cursor,
    /// type 8: 预发射标记（双视图均跳过）
    PreEmit,
    /// type 9: 双拼替换——(用户原字节, 合成串); 显示只取原字节
    DoublePinReplacement(u8, String),
}

pub enum VKey {
    Char(char),
    Drop,    // 退格（DropLast）
    Exit,    // 再次 v / Esc
    Commit,  // 回车（M-V1 兼容路径; M-V4 起 V 模式空格走 space() = select(0)）
    /// M-V3: 追加一个 PendingInput 节点（进入/维持节点模式）
    Pending(PendingInput),
}

/// 选取提交结果（役N tick50 M-V4; 对应 native select_candidate 路径）。
/// - [`VCommit::Text`]: 命中面板候选, 提交候选串——铁律: 提交即结束 V 模式
/// - [`VCommit::Raw`]: 越界且 pending 非空的 raw 提交（reset_session +
///   pending 原文整段上屏, 同样结束 V 模式）
/// - [`VCommit::None`]: 无候选且 pending 空（空会话, 无动作）
#[derive(Debug, Clone, PartialEq)]
pub enum VCommit {
    Text(String),
    Raw(String),
    None,
}

// ---------------- M-V3: 节点链双视图重建（役N tick50） ----------------

/// 双视图重建内核：一次走完节点链, 同时积累 raw/display/cursor/stops。
struct PendingViews {
    raw: String,
    display: String,
    cursor: Option<usize>,
    stops: Vec<usize>,
}

/// 按 wetypex 语义走节点链（逆向依据: 节点布局 type u32@+0 / next@+8 /
/// 数据@+24(+32); run17-22 面板事件）:
/// - SelectedText/ExactlyMatched/Insertion → 文本进双视图
///   （SelectedText 额外每字记一个停点 = 当时 display 长度）
/// - Deletion/Replacement/Exchange/DoublePinReplacement → 合成纠错串
///   **不显示**, 双视图均只出保留的用户原字节（u8 as char）
/// - Separator → display 恒加 '\''; 仅存储字节 == b'\'' 时 raw 才加
/// - Cursor → 记录 displayCursor = 当时 display.len()（多节点取最后）
/// - PreEmit → 跳过（native 语义 = 预发射标记, 不入视图）
fn build_pending_views(nodes: &[PendingInput]) -> PendingViews {
    let mut v = PendingViews {
        raw: String::new(),
        display: String::new(),
        cursor: None,
        stops: Vec::new(),
    };
    for n in nodes {
        match n {
            PendingInput::SelectedText(s) => {
                // 选中替换: 逐字进双视图, 每字一个停点
                for ch in s.chars() {
                    v.raw.push(ch);
                    v.display.push(ch);
                    v.stops.push(v.display.len());
                }
            }
            PendingInput::ExactlyMatched(s) | PendingInput::Insertion(s) => {
                v.raw.push_str(s);
                v.display.push_str(s);
            }
            // 纠错四型: 合成纠错不显示, 只出保留原字节（u8 as char）
            PendingInput::Deletion(orig)
            | PendingInput::Exchange(orig)
            | PendingInput::Replacement(orig, _)
            | PendingInput::DoublePinReplacement(orig, _) => {
                v.raw.push(*orig as char);
                v.display.push(*orig as char);
            }
            PendingInput::Separator(b) => {
                v.display.push('\'');
                if *b == b'\'' {
                    v.raw.push('\''); // 仅半角 ' 存储字节属于 raw
                }
            }
            PendingInput::Cursor => v.cursor = Some(v.display.len()),
            PendingInput::PreEmit => {}
        }
    }
    v
}

/// 节点链 → (raw, display, cursor) 双视图 + 光标位（M-V3 主出口）。
/// raw = 送评估管的原始输入串（VMode.pending 即由它派生）;
/// display = 面板显示串; cursor = display 视图字节偏移。
/// SelectedText 停点表单独出口见 [`pending_stops`]。
pub fn pending_display(nodes: &[PendingInput]) -> (String, String, Option<usize>) {
    let v = build_pending_views(nodes);
    (v.raw, v.display, v.cursor)
}

/// SelectedText 停点表（native: 选中替换每字一个停点）——display 视图
/// 字节偏移, 逐字记录「该字落定后的 display 长度」。
pub fn pending_stops(nodes: &[PendingInput]) -> Vec<usize> {
    build_pending_views(nodes).stops
}

impl VMode {
    pub fn new() -> Self {
        Self::default()
    }

    /// 单键处理。返回 Some(输出文本) 当该键产生提交（commit）。
    /// 役N tick50 M-V3: VKey::Pending 把节点追加进 pending_nodes, pending
    /// 串改由 pending_display 的 raw 视图派生（首个 Pending 键时既有
    /// pending 串转为 ExactlyMatched 引导节点, 字符不丢）; 纯字符模式下
    /// Char/Drop/Commit 的 M-V1 行为逐位不变。
    pub fn process(&mut self, k: VKey) -> Option<String> {
        if !self.active {
            if let VKey::Char('v') = k {
                if self.pending.is_empty() {
                    self.active = true;
                    self.pending_nodes.clear();
                    self.node_mode = false;
                    return None;
                }
            }
            // 非 V 模式：常规透传（拼音管线域，不在此处理）。
            // 节点链存在时 DropLast 仍作用于链（native sub_18044DF20 直接
            // 操作 pending_inputs_, 不看激活位）; 纯字符非激活态维持 M-V1
            // 的忽略语义。
            match k {
                VKey::Char(c) => self.push_char(c),
                VKey::Pending(node) => self.push_node(node),
                VKey::Drop if self.node_mode => {
                    self.pending_nodes.pop();
                    self.sync_pending();
                }
                _ => {}
            }
            return None;
        }
        match k {
            VKey::Char('v') | VKey::Exit => {
                // 再次按 v / Esc → 退出（官方文档: 按 v 进入后再按 v 或 Esc 退出）
                self.end_session();
                None
            }
            VKey::Drop => {
                // DropLast sub_18044DF20: 节点模式弹末节点重建; 字符模式退一格
                if self.node_mode {
                    self.pending_nodes.pop();
                    self.sync_pending();
                } else {
                    self.pending.pop();
                }
                None
            }
            VKey::Commit => {
                let out = self.panel_text();
                self.end_session();
                Some(out)
            }
            VKey::Char(c) => {
                self.push_char(c);
                None
            }
            VKey::Pending(node) => {
                self.push_node(node);
                None
            }
        }
    }

    // ---------------- M-V4: 选取语义（役N tick50） ----------------

    /// 选取面板候选（M-V4 主出口）。native 逆向（wetypex service.hpp /
    /// service_select + PC run19-22）: V 模式空格 = 选取当前面板候选
    /// （不是字符键）, 选取调用
    /// select_candidate(engine, engineText.ptr, len, id, len, null×8)。
    ///
    /// 铁律（native 源码注释原文）: "Calculator candidates commit a
    /// result without consuming the expression like a pinyin candidate.
    /// End VModeV2 before accepting the next key."——候选提交后必须结束
    /// V 模式（set_bool_option(0x11, false)）才能接受下一键, 故命中即
    /// `VCommit::Text(候选串)` + [`end_session`]（active=false、pending/
    /// 节点链全清）。
    ///
    /// index 越界（无该候选）时走 raw 提交语义: pending 非空 →
    /// `VCommit::Raw(pending)`（reset_session + 整段原样上屏, 同样结束
    /// V 模式）; pending 空 → `VCommit::None`（空会话无动作）。
    pub fn select(&mut self, index: usize) -> VCommit {
        let cands = self.panel_candidates();
        match cands.get(index) {
            Some(text) => {
                let out = text.clone();
                // 铁律: End VModeV2 before accepting the next key
                self.end_session();
                VCommit::Text(out)
            }
            None if self.pending.is_empty() => VCommit::None,
            None => {
                // 无候选 raw 提交: reset_session + pending 原文整段上屏
                let raw = self.pending.clone();
                self.end_session();
                VCommit::Raw(raw)
            }
        }
    }

    /// V 模式空格 = 选取面板首项（官方文档「空格选取结果」; native 空格
    /// 非字符键, 走 select_candidate 路径而非 ProcessInput 字符追加）
    pub fn space(&mut self) -> VCommit {
        self.select(0)
    }

    /// 结束 V 模式会话: active 置假 + pending/节点链全清
    /// （= native set_bool_option(0x11, false) / reset_session 语义）
    fn end_session(&mut self) {
        self.active = false;
        self.reset_pending();
    }

    /// 纯字符路径的追加。节点模式下转单字 ExactlyMatched 节点, 维持
    /// 「pending ≡ 节点链 raw 视图」的派生不变量; 字符模式直推 pending。
    fn push_char(&mut self, c: char) {
        if self.node_mode {
            self.pending_nodes
                .push(PendingInput::ExactlyMatched(c.to_string()));
            self.sync_pending();
        } else {
            self.pending.push(c);
        }
    }

    /// 追加一个 PendingInput 节点并重建 pending（M-V3 主路径）。
    /// 首个 Pending 键触发的模式切换中, 既有 pending 串整体转
    /// ExactlyMatched 引导节点（精确匹配段语义, 用户已键入字符）。
    fn push_node(&mut self, node: PendingInput) {
        if !self.node_mode {
            self.node_mode = true;
            if !self.pending.is_empty() {
                let lead = std::mem::take(&mut self.pending);
                self.pending_nodes.push(PendingInput::ExactlyMatched(lead));
            }
        }
        self.pending_nodes.push(node);
        self.sync_pending();
    }

    /// 派生不变量: pending := 节点链 raw 视图
    fn sync_pending(&mut self) {
        self.pending = pending_display(&self.pending_nodes).0;
    }

    /// 会话终止复位（退出/提交后）
    fn reset_pending(&mut self) {
        self.pending.clear();
        self.pending_nodes.clear();
        self.node_mode = false;
    }

    /// 面板输出（space/enter 提交值 = 面板首项: 表达式结果或格式转换）
    pub fn panel_text(&self) -> String {
        let s = self.pending.trim_start_matches('v');
        if let Some(r) = eval_expr(s) {
            return format_num(r);
        }
        if let Some(item) = classify_and_convert(s).first() {
            return item.clone(); // 首项 = 主转换
        }
        s.to_string()
    }

    /// native pending 显示形（run18/22 实证: 逐键实时计算, 显示串 =
    /// "表达式=结果"——如 "168+35.69=203.69"; 无空格触发, 键入即算）
    pub fn display_str(&self) -> String {
        let s = self.pending.trim_start_matches('v');
        if let Some(r) = eval_expr(s) {
            return format!("{s}={}", format_num(r));
        }
        s.to_string()
    }

    /// V 模式面板候选列表（役N tick49 M-V2; 序 = native 面板序）:
    /// - 表达式 → [计算结果]（面板首项 = 结果, 官方文档「空格选取结果」）
    /// - 纯整数 → [大写金额, 罗马(1..20 域内), 干支年]（复用
    ///   classify_and_convert, 保序去重）
    /// - 小数 ≤2 位 → [大写金额]
    /// - 其它 → 空
    /// 注: 纯数字（如 "2024"）既是合法表达式又是转换输入——native 面板
    /// 对数字型走正则分派 8 型先于计算器, 故此处转换面板优先（首项 =
    /// 大写金额而非数字本身; panel_text 的 M-V1 行为不变）。
    pub fn panel_candidates(&self) -> Vec<String> {
        let s = self.pending.trim_start_matches('v');
        // 数字型转换面板优先
        let conv = classify_and_convert(s);
        if !conv.is_empty() {
            // 保序去重（面板不重项; 现表 三/二 项互不重叠, 防御性保证）
            let mut out: Vec<String> = Vec::with_capacity(conv.len());
            for item in conv {
                if !out.contains(&item) {
                    out.push(item);
                }
            }
            return out;
        }
        // 表达式 → 首项 = 结果
        if let Some(r) = eval_expr(s) {
            return vec![format_num(r)];
        }
        Vec::new()
    }
}

// ---------------- 计算器（函数表 21 项 1:1） ----------------

/// 表达式求值（递归下降; 支持 + - × ÷ % ( ) 与函数表）。
/// 函数域: sin cos arcsin arccos arctan sqrt exp log ln × ÷ % ✖️ ➕ ➖ ➗ ✖
///         max var stdev avg mod（.rdata 0xf6588c..0xf658f8; 役N tick49
///         M-V2: 多参函数逗号分隔参数列表, 每参为完整表达式可嵌套）
pub fn eval_expr(s: &str) -> Option<f64> {
    let t: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return None;
    }
    let mut p = Parser { t: &t, i: 0 };
    let v = p.expr()?;
    if p.i != t.len() {
        return None;
    }
    v.is_finite().then_some(v)
}

struct Parser<'a> {
    t: &'a [char],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.t.get(self.i).copied()
    }

    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some('+') | Some('➕') => {
                    self.i += 1;
                    v += self.term()?;
                }
                Some('-') | Some('➖') => {
                    self.i += 1;
                    v -= self.term()?;
                }
                _ => return Some(v),
            }
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        loop {
            match self.peek() {
                Some('×') | Some('✖') | Some('*') => {
                    self.i += 1;
                    v *= self.factor()?;
                }
                Some('÷') | Some('➗') | Some('/') => {
                    self.i += 1;
                    let d = self.factor()?;
                    if d == 0.0 {
                        return None;
                    }
                    v /= d;
                }
                Some('%') => {
                    self.i += 1;
                    let d = self.factor()?;
                    if d == 0.0 {
                        return None;
                    }
                    v %= d;
                }
                _ => return Some(v),
            }
        }
    }

    fn factor(&mut self) -> Option<f64> {
        match self.peek()? {
            '(' => {
                self.i += 1;
                let v = self.expr()?;
                if self.peek() != Some(')') {
                    return None;
                }
                self.i += 1;
                Some(v)
            }
            '-' => {
                self.i += 1;
                Some(-self.factor()?)
            }
            _ => {
                // 函数名或数字
                let start = self.i;
                while matches!(self.peek(),
                    Some(c) if c.is_ascii_alphanumeric() || c == '.')
                {
                    self.i += 1;
                }
                let name: String = self.t[start..self.i].iter().collect();
                if self.i == start {
                    return None;
                }
                if let Ok(v) = name.parse::<f64>() {
                    return Some(v);
                }
                // 役N tick49 M-V2: 函数参数列表——括号内逗号分隔（全/半角
                // 均可, native 中文键盘常出全角「，」）, 每参是一个完整
                // 表达式（可嵌套: max(1+1, avg(2,4))）; 无括号仍为单参
                // 简写（sqrt4 = sqrt(4)）
                let args = if self.peek() == Some('(') {
                    self.i += 1;
                    let mut list = vec![self.expr()?];
                    while matches!(self.peek(), Some(',') | Some('，')) {
                        self.i += 1;
                        list.push(self.expr()?);
                    }
                    if self.peek() != Some(')') {
                        return None;
                    }
                    self.i += 1;
                    list
                } else {
                    vec![self.factor()?]
                };
                apply_fn(&name, &args)
            }
        }
    }
}

/// 函数应用（役N tick49 M-V2: 参数列表化——统计函数族 max/avg/var/stdev
/// 多参 + mod 二元, 覆盖 .rdata 0xf6588c..0xf658f8 函数表 21 项全集）。
///
/// 空参/参数数不符 → None。单参统计语义取「总体统计惯例」（同 Excel
/// VAR.P/STDEV.P 与 MAX/AVERAGE）: max(x)=avg(x)=x; 总体方差
/// Σ(x−μ)²/n 在 n=1 时良定义 → var(x)=stdev(x)=0。
fn apply_fn(name: &str, args: &[f64]) -> Option<f64> {
    match (name, args) {
        // —— 单参函数族（16 项——.rdata 单参区 1:1） ——
        ("sin", [x]) => Some(x.sin()),
        ("cos", [x]) => Some(x.cos()),
        ("arcsin", [x]) => Some(x.asin()),
        ("arccos", [x]) => Some(x.acos()),
        ("arctan", [x]) => Some(x.atan()),
        ("sqrt", [x]) => Some(x.sqrt()),
        ("exp", [x]) => Some(x.exp()),
        ("log", [x]) => Some(x.log10()),
        ("ln", [x]) => Some(x.ln()),
        // —— 统计函数族（多参; 逗号分隔参数列表） ——
        ("max", xs) if !xs.is_empty() => {
            Some(xs.iter().copied().fold(f64::NEG_INFINITY, f64::max))
        }
        ("avg", xs) if !xs.is_empty() => {
            Some(xs.iter().sum::<f64>() / xs.len() as f64)
        }
        ("var", xs) if !xs.is_empty() => Some(population_var(xs)),
        ("stdev", xs) if !xs.is_empty() => Some(population_var(xs).sqrt()),
        // —— 二元: a mod b（b=0 → None, 同 ÷0 域规则） ——
        ("mod", [a, b]) if *b != 0.0 => Some(a % b),
        _ => None,
    }
}

/// 总体方差 Σ(x−μ)²/n（役N tick49 M-V2; n≥1 由调用方保证）
fn population_var(xs: &[f64]) -> f64 {
    let n = xs.len() as f64;
    let mu = xs.iter().sum::<f64>() / n;
    xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n
}

// ---------------- 数字格式转换（.rdata 套件表 1:1） ----------------

/// 罗马数字: **域 1..20**（sub_180177810 运行时构造完整表 I..XX;
/// .rdata 仅存 7 硬型 III/VII/XII/XIV/XVI/XVIII/XIX）
pub fn to_roman(n: u32) -> Option<String> {
    if !(1..=20).contains(&n) {
        return None;
    }
    let tens = ["", "X", "XX"][((n / 10) % 10) as usize];
    let units = [
        "", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX",
    ][(n % 10) as usize];
    Some(format!("{tens}{units}"))
}

const DIGITS_UPPER: [&str; 10] =
    ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"];

/// 中文大写金额（元整/角/分——.rdata 0xf65b40..0xf65b54 单位表）
pub fn to_cny_upper(v: f64) -> String {
    let cents = (v * 100.0).round() as i64;
    if cents == 0 {
        return "零元整".to_string();
    }
    let neg = cents < 0;
    let cents = cents.abs();
    let yuan = cents / 100;
    let jiao = (cents / 10) % 10;
    let fen = cents % 10;
    let mut out = String::new();
    if yuan > 0 {
        out.push_str(&to_cjk_upper_units(yuan as u64));
        out.push_str("元");
    }
    if jiao > 0 {
        out.push_str(DIGITS_UPPER[jiao as usize]);
        out.push_str("角");
    } else if yuan > 0 && fen > 0 {
        out.push('零'); // 标准规则: 角位为零且元>0 → 单 零（不发 零角）
    }
    if fen > 0 {
        out.push_str(DIGITS_UPPER[fen as usize]);
        out.push_str("分");
    }
    if jiao == 0 && fen == 0 {
        out.push_str("整");
    }
    if neg {
        format!("负{out}")
    } else {
        out
    }
}

fn to_cjk_upper_units(n: u64) -> String {
    // 拾佰仟（.rdata 0xf65a8c）+ 万亿扩展
    const U16: [&str; 4] = ["", "拾", "佰", "仟"];
    const U64: [&str; 3] = ["", "万", "亿"];
    if n == 0 {
        return "零".to_string();
    }
    let mut groups: [u32; 3] = [0; 3];
    groups[0] = (n % 10000) as u32;
    groups[1] = ((n / 10000) % 10000) as u32;
    groups[2] = ((n / 100000000) % 10000) as u32;
    let mut out = String::new();
    for gi in (0..3).rev() {
        let g = groups[gi];
        if g == 0 {
            continue;
        }
        let mut started = false;
        for pos in (0..4).rev() {
            let d = (g / 10u32.pow(pos)) % 10;
            if d == 0 {
                if started && !out.ends_with('零') {
                    out.push('零');
                }
                continue;
            }
            out.push_str(DIGITS_UPPER[d as usize]);
            if pos > 0 {
                out.push_str(U16[pos as usize]);
            }
            started = true;
        }
        out.push_str(U64[gi]);
    }
    out.trim_end_matches('零').to_string()
}

const HEAVENLY: [&str; 10] =
    ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
const EARTHLY: [&str; 12] =
    ["子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥"];
const ZODIAC: [&str; 12] =
    ["鼠", "牛", "虎", "兔", "龙", "蛇", "马", "羊", "猴", "鸡", "狗", "猪"];

/// 干支+生肖年（天干 10 / 地支 12 / 生肖 12——.rdata 0xf65a98..0xf65b1c）
pub fn ganzhi_year(y: i32) -> String {
    // 公元 4 年 = 甲子（鼠）；负年向前推
    let idx = (y - 4).rem_euclid(60) as usize;
    let stem = HEAVENLY[idx % 10];
    let branch = EARTHLY[idx % 12];
    let animal = ZODIAC[idx % 12];
    format!("{stem}{branch}{animal}年")
}

/// 数字正则分派（8 型——.rdata 0xf65b60..；v 后裸数字的转换面板）
pub fn classify_and_convert(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    // ^0*([0-9]+)$ —— 纯整数: 大写/罗马/干支
    if let Ok(n) = s.trim_start_matches('0').parse::<u32>() {
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
            out.push(to_cny_upper(n as f64));
            if let Some(r) = to_roman(n) {
                out.push(r);
            }
            out.push(ganzhi_year(n as i32));
            return out;
        }
    }
    // ^0*([0-9]*)\.([0-9]{1,2})$ —— 小数 ≤2 位: 人民币大写金额
    if let Some((int_part, frac)) = s.split_once('.') {
        let frac_ok = (1..=2).contains(&frac.len())
            && frac.chars().all(|c| c.is_ascii_digit());
        let int_ok = int_part.chars().all(|c| c.is_ascii_digit());
        if int_ok && frac_ok {
            if let Ok(v) = s.parse::<f64>() {
                out.push(to_cny_upper(v));
            }
        }
    }
    out
}

/// 结果格式化（native 面板形: 整数不带尾零; 小数自然位）
fn format_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 官方文档锚: v168+35.69 → 203.69
    #[test]
    fn calc_official_anchor() {
        let mut vm = VMode::new();
        for c in "168+35.69".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.panel_text(), "203.69");
        // 役N tick33: native pending 显示形（run18/22 逐键实时算实证）
        assert_eq!(vm.display_str(), "168+35.69=203.69");
    }

    #[test]
    fn calc_functions() {
        assert!((eval_expr("sqrt(4)").unwrap() - 2.0).abs() < 1e-9);
        assert!((eval_expr("sin(0)").unwrap()).abs() < 1e-9);
        assert_eq!(eval_expr("1÷0"), None);
        assert_eq!(eval_expr("2×3+1").unwrap(), 7.0);
    }

    /// 罗马 1..20 域（sub_180177810 表 1:1）
    #[test]
    fn roman_table() {
        assert_eq!(to_roman(1).unwrap(), "I");
        assert_eq!(to_roman(3).unwrap(), "III");
        assert_eq!(to_roman(7).unwrap(), "VII");
        assert_eq!(to_roman(12).unwrap(), "XII");
        assert_eq!(to_roman(14).unwrap(), "XIV");
        assert_eq!(to_roman(16).unwrap(), "XVI");
        assert_eq!(to_roman(18).unwrap(), "XVIII");
        assert_eq!(to_roman(19).unwrap(), "XIX");
        assert_eq!(to_roman(20).unwrap(), "XX");
        assert_eq!(to_roman(21), None);
    }

    #[test]
    fn cny_upper() {
        assert_eq!(to_cny_upper(0.0), "零元整");
        assert_eq!(to_cny_upper(1.05), "壹元零伍分");
        assert_eq!(to_cny_upper(168.69), "壹佰陆拾捌元陆角玖分");
        assert_eq!(to_cny_upper(12.5), "壹拾贰元伍角");
    }

    /// 干支锚: 2024 = 甲辰龙年
    #[test]
    fn ganzhi() {
        assert_eq!(ganzhi_year(2024), "甲辰龙年");
        assert_eq!(ganzhi_year(1984), "甲子鼠年");
    }

    #[test]
    fn dispatch() {
        let r = classify_and_convert("168");
        assert!(r.len() >= 2);
        let r2 = classify_and_convert("1.5");
        assert_eq!(r2, vec!["壹元伍角".to_string()]);
    }

    /// 状态机: v 进/出 + DropLast
    #[test]
    fn state_machine() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('v')); // 进
        vm.process(VKey::Char('1'));
        vm.process(VKey::Char('2'));
        vm.process(VKey::Drop);
        assert_eq!(vm.pending, "1");
        assert_eq!(vm.process(VKey::Commit), Some("1".to_string()));
        assert!(!vm.active);
    }

    /// 役N tick49 M-V2: 多参统计函数族（max/avg/var/stdev/mod）
    #[test]
    fn calc_multi_arg_functions() {
        assert_eq!(eval_expr("max(1,5,3)"), Some(5.0));
        assert_eq!(eval_expr("avg(2,4,6)"), Some(4.0));
        // var = 总体方差 Σ(x−μ)²/n = 8/3 ≈ 2.6667
        let var = eval_expr("var(2,4,6)").unwrap();
        assert!((var - 8.0 / 3.0).abs() < 1e-9);
        assert!((var - 2.6667).abs() < 1e-4);
        // stdev = sqrt(var)
        let sd = eval_expr("stdev(2,4,6)").unwrap();
        assert!((sd - (8.0_f64 / 3.0).sqrt()).abs() < 1e-9);
        // mod 二元; b=0 → None（同 ÷0 域规则）
        assert_eq!(eval_expr("mod(7,3)"), Some(1.0));
        assert_eq!(eval_expr("mod(1,0)"), None);
        // 空参 → None; 单参总体统计惯例 max(x)=x
        assert_eq!(eval_expr("max()"), None);
        assert_eq!(eval_expr("avg()"), None);
        assert_eq!(eval_expr("max(5)"), Some(5.0));
        assert_eq!(eval_expr("stdev(5)"), Some(0.0));
        // 参数数不符（mod 二元 / sin 单参）→ None
        assert_eq!(eval_expr("mod(7)"), None);
        assert_eq!(eval_expr("sin(1,2)"), None);
        // 嵌套: 每参是完整表达式; 全角逗号/空格亦可
        assert_eq!(eval_expr("max(1+1, avg(2,4))"), Some(3.0));
        assert_eq!(eval_expr("max(1，5)"), Some(5.0));
        assert_eq!(eval_expr("max(1, 5, 3)"), Some(5.0));
    }

    /// 役N tick49 M-V2: 面板候选序（native 面板序 1:1）
    #[test]
    fn panel_candidates_order() {
        // 表达式 → [结果]（首项 = 结果, 「空格选取结果」）
        let mut vm = VMode::new();
        for c in "168+35.69".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.panel_candidates(), vec!["203.69".to_string()]);
        // 函数表达式同走计算器域
        let mut vm = VMode::new();
        for c in "max(1,5,3)".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.panel_candidates(), vec!["5".to_string()]);

        // 纯整数 → [大写金额, 罗马(1..20 域内), 干支年]（数字型分派
        // 优先于计算器——"2024" 首项 = 大写金额而非 "2024"）
        let mut vm = VMode::new();
        for c in "2024".chars() {
            vm.process(VKey::Char(c));
        }
        let cands = vm.panel_candidates();
        assert_eq!(cands[0], "贰仟零贰拾肆元整");
        assert!(cands.contains(&"甲辰龙年".to_string()));
        assert_eq!(cands.len(), 2); // 2024 > 20 → 罗马不出
        // 罗马在域内: 14 → [大写, XIV, 干支]
        let mut vm = VMode::new();
        for c in "14".chars() {
            vm.process(VKey::Char(c));
        }
        let cands = vm.panel_candidates();
        assert_eq!(cands.len(), 3);
        assert_eq!(cands[1], "XIV");

        // 小数 ≤2 位 → [大写金额]
        let mut vm = VMode::new();
        for c in "1.5".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.panel_candidates(), vec!["壹元伍角".to_string()]);

        // 其它（非表达式非数字）→ 空
        let mut vm = VMode::new();
        for c in "abc".chars() {
            vm.process(VKey::Char(c));
        }
        assert!(vm.panel_candidates().is_empty());
    }

    /// 役N tick50 M-V3: Char/Pending 混合链 + 节点模式 DropLast
    #[test]
    fn mv3_mixed_char_pending_chain() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('1'));
        assert_eq!(vm.pending, "1");
        // 首个 Pending 键 → 既有 "1" 转 ExactlyMatched 引导节点, 拼接其后
        vm.process(VKey::Pending(PendingInput::Insertion("68".to_string())));
        assert_eq!(vm.pending, "168");
        // 节点模式下 Char 继续拼入（pending ≡ raw 视图不变量）
        vm.process(VKey::Char('+'));
        assert_eq!(vm.pending, "168+");
        // Drop = 弹出末节点重建（末节点 = 单字 '+'）
        vm.process(VKey::Drop);
        assert_eq!(vm.pending, "168");
        vm.process(VKey::Drop); // 弹出 Insertion("68") 整节点
        assert_eq!(vm.pending, "1");
    }

    /// 役N tick50 M-V3: 纠错四型/分隔符/预发射的双视图语义
    #[test]
    fn mv3_pending_display_semantics() {
        // Replacement(b'3',"5"): raw 保留原字节 '3'（合成纠错 "5" 不显示）,
        // display 同; Deletion/Exchange/DoublePinReplacement 同规则;
        // PreEmit 跳过
        let (raw, display, cursor) = pending_display(&[
            PendingInput::Replacement(b'3', "5".to_string()),
            PendingInput::Deletion(b'7'),
            PendingInput::Exchange(b'9'),
            PendingInput::DoublePinReplacement(b'2', "6".to_string()),
            PendingInput::PreEmit,
        ]);
        assert_eq!(raw, "3792");
        assert_eq!(display, "3792");
        assert_eq!(cursor, None);
        // Separator: display 恒加 '\''; 仅存储字节 == b'\'' 时 raw 才加
        let (raw, display, _) = pending_display(&[
            PendingInput::Insertion("ab".to_string()),
            PendingInput::Separator(b'\''), // 半角 ' → 双视图
            PendingInput::Insertion("cd".to_string()),
            PendingInput::Separator(b'|'), // 其它存储字节 → 仅 display
        ]);
        assert_eq!(raw, "ab'cd");
        assert_eq!(display, "ab'cd'");
        // SelectedText/ExactlyMatched 贡献文本到双视图
        let (raw, display, _) = pending_display(&[
            PendingInput::SelectedText("选中文本".to_string()),
            PendingInput::ExactlyMatched("X".to_string()),
        ]);
        assert_eq!(raw, "选中文本X");
        assert_eq!(display, "选中文本X");
    }

    /// 役N tick50 M-V3: Cursor 节点 → displayCursor 光标位
    #[test]
    fn mv3_cursor_node() {
        let (raw, display, cursor) = pending_display(&[
            PendingInput::Insertion("168".to_string()),
            PendingInput::Cursor,
            PendingInput::Insertion("35".to_string()),
        ]);
        assert_eq!(raw, "16835");
        assert_eq!(display, "16835");
        assert_eq!(cursor, Some(3)); // 节点处 display.len()
        // 多 Cursor 节点 → 取最后节点位（displayCursor 单值）
        let (_, _, cursor) = pending_display(&[
            PendingInput::Insertion("ab".to_string()),
            PendingInput::Cursor,
            PendingInput::Insertion("c".to_string()),
            PendingInput::Cursor,
        ]);
        assert_eq!(cursor, Some(3));
    }

    /// 役N tick50 M-V3: Pending 拼出官方锚表达式 → 面板/显示形与 M-V1 一致
    #[test]
    fn mv3_pending_expression_e2e() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('v')); // 激活
        for seg in ["168", "+35", ".69"] {
            vm.process(VKey::Pending(PendingInput::Insertion(seg.to_string())));
        }
        // 混入 Char 键（节点模式下转单字节点, 混用无碍）
        vm.process(VKey::Char('*'));
        vm.process(VKey::Pending(PendingInput::Insertion("2".to_string())));
        assert_eq!(vm.pending, "168+35.69*2");
        // 标准优先级: 35.69*2 先乘 = 71.38, 168+71.38 = 239.38
        assert_eq!(vm.display_str(), "168+35.69*2=239.38");
        assert_eq!(vm.panel_text(), "239.38");
        // Commit 提交面板值 + 状态复位（节点链同清）
        assert_eq!(vm.process(VKey::Commit), Some("239.38".to_string()));
        assert!(vm.pending.is_empty());
        assert!(vm.pending_nodes.is_empty() && !vm.node_mode);
        // 纯 Pending 链逐字拼官方锚 "168+35.69"
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "168+35.69".chars() {
            vm.process(VKey::Pending(PendingInput::ExactlyMatched(
                c.to_string(),
            )));
        }
        assert_eq!(vm.display_str(), "168+35.69=203.69");
        assert_eq!(vm.panel_candidates(), vec!["203.69".to_string()]);
    }

    /// 役N tick50 M-V3: SelectedText 每字一个停点（选中替换停点表）
    #[test]
    fn mv3_selected_text_stops() {
        let stops = pending_stops(&[
            PendingInput::SelectedText("365".to_string()),
            PendingInput::Separator(b'\''),
            PendingInput::SelectedText("2".to_string()),
        ]);
        // display 视图字节偏移: "365" → 1,2,3; Separator 贡献 '\'（len 4,
        // 不停点）; '2' 落定后 len 5
        assert_eq!(stops, vec![1, 2, 3, 5]);
    }

    /// 役N tick50 M-V4: select(0) 表达式 → Text(结果) + 铁律自动结束;
    /// 结束后 '1' 不激活、'v' 重新进入（既有 M-V1 arm 规则不变）
    #[test]
    fn mv4_select_expression_end_and_rearm() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('v')); // 进 V 模式
        for c in "168+35.69".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.select(0), VCommit::Text("203.69".to_string()));
        // 铁律: "…End VModeV2 before accepting the next key."
        assert!(!vm.active);
        assert!(vm.pending.is_empty());
        // commit 后再按 '1' 不应进 V 模式（非 arm 键, 落透传域 pending）
        vm.process(VKey::Char('1'));
        assert!(!vm.active);
        assert_eq!(vm.pending, "1");
        // 按 'v' 重新进入——M-V1 arm 规则要求 pending 为空（'1' 属外层
        // 拼音管线域, 消费上屏后清账, 此处模拟该清账）
        vm.pending.clear();
        vm.process(VKey::Char('v'));
        assert!(vm.active);
        assert!(vm.pending.is_empty());
    }

    /// 役N tick50 M-V4: 数字面板选取 + 越界分支（2024 > 20 → 罗马域外,
    /// 面板实值 = [大写金额, 干支年] 两项; select(2) 越界 + pending 空 → None）
    #[test]
    fn mv4_select_number_panel_and_out_of_range() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "2024".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(
            vm.panel_candidates(),
            vec!["贰仟零贰拾肆元整".to_string(), "甲辰龙年".to_string()]
        );
        assert_eq!(vm.select(1), VCommit::Text("甲辰龙年".to_string()));
        assert!(!vm.active && vm.pending.is_empty());
        // 会话已结束（pending 空）→ 再越界选取 = 空会话 → None
        assert_eq!(vm.select(2), VCommit::None);
        assert!(!vm.active);
    }

    /// 役N tick50 M-V4: 越界 + 非空 pending → Raw 整段上屏（reset_session
    /// 语义, 同样结束 V 模式）; 空 pending / 空会话 → None 不改状态
    #[test]
    fn mv4_select_out_of_range_raw_commit() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "abc".chars() {
            vm.process(VKey::Char(c));
        }
        assert!(vm.panel_candidates().is_empty()); // 非表达式非数字 → 无候选
        assert_eq!(vm.select(0), VCommit::Raw("abc".to_string()));
        // raw 提交同样结束: active/pending/节点链全清
        assert!(!vm.active && vm.pending.is_empty() && !vm.node_mode);
        // 空会话（pending 空）→ None
        let mut vm2 = VMode::new();
        assert_eq!(vm2.select(0), VCommit::None);
        assert!(!vm2.active);
    }

    /// 役N tick50 M-V4: space() == select(0)（官方文档「空格选取结果」）
    #[test]
    fn mv4_space_is_select_first() {
        // 表达式域: space = 面板首项 = 计算结果
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "168+35.69".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.space(), VCommit::Text("203.69".to_string()));
        assert!(!vm.active);
        // 数字域: space = 面板首项 = 大写金额（M-V2 面板序: 转换先于计算器）
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "14".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.space(), VCommit::Text("壹拾肆元整".to_string()));
        // 无候选: space 同走 raw 整段上屏
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "xyz".chars() {
            vm.process(VKey::Char(c));
        }
        assert_eq!(vm.space(), VCommit::Raw("xyz".to_string()));
        assert!(!vm.active);
    }

    /// 役N tick50 M-V4: 节点链会话同受铁律约束——Text 提交清节点链 +
    /// node_mode, 结束后 '1' 不激活、'v' 重新进入
    #[test]
    fn mv4_select_node_session_end() {
        let mut vm = VMode::new();
        vm.process(VKey::Char('v'));
        for c in "168+35.69".chars() {
            vm.process(VKey::Pending(PendingInput::ExactlyMatched(
                c.to_string(),
            )));
        }
        assert!(vm.node_mode && !vm.pending_nodes.is_empty());
        assert_eq!(vm.select(0), VCommit::Text("203.69".to_string()));
        assert!(!vm.active && !vm.node_mode && vm.pending_nodes.is_empty());
        assert!(vm.pending.is_empty());
        // 结束后再按键: '1' 不激活（透传落账）; 'v' 重新 arm（空 pending）
        vm.process(VKey::Char('1'));
        assert!(!vm.active);
        assert_eq!(vm.pending, "1");
        vm.pending.clear();
        vm.process(VKey::Char('v'));
        assert!(vm.active);
    }
}
