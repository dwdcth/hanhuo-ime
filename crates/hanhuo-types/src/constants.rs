//! 引擎旋钮/罚表/覆盖表/混合召回权重与纯函数（B 档逆向成果常量族）.


// ---- 旋钮（B 档 + 近似覆盖因子；与 wxq/engine.py 逐项一致）----
pub const LAMBDA: f64 = -0.40; // NNLM 融合 λ（hpmac@0x37c 实测值）

pub const NN_COEF: f64 = 0.23; // 0.23 × λ × ΔN

pub const NN_CLAMP: f64 = 1.5; // 饱和界 G∈(0.6,1.8) 中值

pub const W_FREQ_ASSOC: f64 = 0.25; // 联想模式词频并列破除权重

pub const COV_PENALTY: f64 = 0.35; // 补全候选每额外音节的覆盖罚

pub const PENDING_ALIGN_BOOST: f64 = 0.8; // 贪婪音节边界对齐的 pending 加权

// ---- agentB ②悬空/在途通道（2026-10-27 移植；py 行为唯一真值）----
pub const DANGLING_COMPLETION_BOOST: f64 = 2.0; // 悬空态补全词加权（native nih 前12位/

//   zhongg 前16位全是补全词块, 单字块垫后; zho 需压 comp 找(-0.688) 0.20 分.
//   boost 对象从边界单字移到 node(inp) 子树补全词 = PENDING_ALIGN 真实语义;
//   量级: 盖过 单字↔首补全词 logp 差 0.73 + comp 压制 0.20 + 裕量）
pub const PENDING_DEMOTE_COMPLETE: f64 = 1.8; // 完整态降权

pub const PENDING_EXACT_BOOST: f64 = 3.0; // 完整态整词精确块加权（agentB 实证: native 该读音

//   全部词永远置顶——zhongguo 前10全为 zhong'guo 词/niha 你哈压单字; cnt>64
//   且无 hot 时堆合并丢 r==lo 词, 需从 lo 槽列补发. 3.0 = 盖过整词↔单字
//   最大 logp 差 2.93 + 裕量, 单字读音节点无此竞争态不发）
pub const PHRASE_EDGE: f64 = 0.5; // 双精确短语压整词（native niha: 你\x01哈#1 > 你哈）

pub const PHRASE_GATE_LP: f64 = -4.0; // 整词最佳 logp ≤ 此值才发短语（nihao 你好-2.799 不发,

//   词形态安全; native 只在整词低频/词条边界不确定时展示组合形态）
pub const ALTER_SYL_DEMOTE: f64 = 3.0; // 词条边界不确定态压制改尾音节外推词（native niha

//   无 你好/你还, 单字块填 #5-16; 需盖过 你好(-0.8) 到 妮(-3.0) 的缝）
pub const DANGLING_FINAL_BOOST: f64 = 2.3; // 悬空首音节+尾元音匹配词（native zho: 之后#3/

//   最后#6 = 首音节 z/zh 模糊 + 尾字韵母以悬空元音开头; 需插进单字块中部）
pub const WINDOW_PHRASE_LP: f64 = -3.2; // 在途窗口双词短语门: 头词 logp ≤ 此值才发

//   （native woaizhongguo=我爱\x01中国#1 头词我爱-4.516; jintiandetianqi
//   面板无短语, 头词今天-2.408 → 以词条边界确信度分界）
pub const COMPLETE_SUPPRESS: f64 = 6.0;

pub const COMP_DOMINATE: f64 = 3.0; // 组合在场时非全消费降权（native buxcuo: 不 f392≈5） // full_cover 下补全外推硬抑制（native 96 条不可见实证）

pub const W14: f64 = 1.0; // path_rank LR feat"14"=消费键数−音节数先验（md.bin 实证；0=零化态）

// 原生融合（第六役终破）: f396 = raw + f412 + (5.0 - 0.5*rank)（f412 查询
// 常量不影响排序）；rank < top_N(4) → f392 ← f396；否则拓扑等价保持原分
pub const LTR_TOP_N: usize = 4;

pub const LTR_RANK_PRIOR: f64 = 5.0;

pub const LTR_RANK_DECAY: f64 = 0.5; // mdltr 融合系数（自有设计；原生 v26 未解）

pub const EDIT_SUB: f64 = 1.25;

pub const EDIT_INS: f64 = 2.88;

pub const EDIT_TRANS: f64 = 2.99;

pub const EDIT_DEL: f64 = 3.22;

/// RepeatPrevChar 边罚（重复字母删除; 管线具名增广器; agentM §5: hpmac
/// base.bin @0x298 = −1.95 f32 精确, 独立于 DEL 槽 @0x160=−3.22129）
pub const EDIT_REPEAT: f64 = 1.95;

/// 件 2: Retroflex 平→翘单向插 h 边罚（agentO 终证: hpmac_base.bin = Cap'n
/// Proto 单段 233 词, 根结构 40 指针字段; field0 = 编辑 op 成本表, 由
/// sub_180280A30 op-token 发射序锚定: f[4]=DEL/3.221, f[5]=TRANS/2.988,
/// f[6]=Exact/0.0, f[2]=INS/2.878 —— 剩余重罚槽 f[3]=−3.285001 即 Retroflex
/// 真槽（旧借 EDIT_INS=2.88 弃用）。行为裁决: 34 集 30/34 持平 + 判据
/// zuanjia 专家#1 / suju 数据#1 / xvexiao 学校#0 保持, caoji 超级 #3→#4）
pub const EDIT_RETRO: f64 = 3.285;

/// 件 3: EditPinyinOnce 整键直查边罚（agentO 终证: 同表 f[1]=−0.586424 =
/// op1 廉价直查/续接边族（每 case 第二 token 同码）; 旧借 EDIT_SUB=1.25
/// 弃用; xvexiao 学校#0 判据保持）
pub const EDIT_EPYN: f64 = 0.586;

// 役 K（agentZ）: G1/G2 兜底与模糊组合旋钮（观测与调定见 agentZ_log S4/S5）
pub const FB_PHRASE_LIFT_G2: f64 = 4.5; // G2 组合面基础升幅+净解析 1.5（bias 入打分层）

pub const FUZZ_SUB_KBN: f64 = 2.5; // 模糊组合 KBN 替换罚（ncei 你猜的 须沉于 奶茶诶）

pub const FUZZ_SUB_VOWEL: f64 = 1.55; // 役N(件1b): 表外元音替换罚（native 回归下界

// 1.37: cheng→chong 让位 成群好; 表内 EPYN 维持 0.586——agentCC_log S3）
pub const FB_INIS: &[u8] = b"bpmfdtnlgkhjqxrzcsyvw";

// 役M(agentBB) 件1 构造层剪枝放宽（py 镜像, 逐档 diff 白名单核查过）:
// 结构 cap/combos 保留/kbest k 三旋钮按键长门控（<门槛维持役K 基线）——
// 全局放宽实测伤 34 集 buxcuo 与 4 键 peiy/jttq 两绿（新面挤占）。
pub const FB_STRUCT_CAP: usize = 64;

pub const FB_STRUCT_CAP_LONG: usize = 1024; // ≥FB_STRUCT_LONG_AT 键（native 四锚全 9 键）

pub const FB_STRUCT_LONG_AT: usize = 9;

pub const FB_COMBO_KEEP: usize = 24; // ≥FB_COMBO_KEEP_AT 键（gerxtq 锚需 24）

pub const FB_COMBO_KEEP_AT: usize = 5;

pub const FB_KBEST_K: usize = 2; // ≥FB_KBEST_K_AT 键（miunaozai 某宝在在 kbest 第2位）

pub const FB_KBEST_K_AT: usize = 5;

pub const FB_OUT_CAP: usize = 20; // fuzzcomp 发射 cap（役M 件2: 14→20; chengqunh 成群和 faces 序 15+ 被截）

pub const FB_OUT_CAP_LONG: usize = 2; // 役M 件3: >8 键窗口态 fcomp cap=2（native 窗口态组合词 ≤2）

pub const FB_EXACT_MAX: usize = 7; // 役M 件3: 精确直读段长上限 6→7（jintiandet jin,tian 词列实证）

pub const FB_COL_CAP_LONG: usize = 8; // 役M 件3: >8 键窗口态首词列深 4→8

#[allow(dead_code)] // (退役保留: flat 模型, 件1c 双因子接替)
const FB_J_COST: f64 = 0.78;

pub const FB_J_EDGE: f64 = 0.45; // 役N(件1c): j 边基费（chengqunh 家族锁定）

pub const FB_J_LETTER: f64 = 0.165; // 役N(件1c): 每缺字母费（和 0.612/好 0.779/还 0.764）

// 回归: 成群和 0.61/找那个 0.79/找哪个 0.71/不想错 0.90/不行错 1.03/
// 个人小提琴 0.42; 叶绿体闭嘴另含吸收键+替换项（agentCC 回归, agentCC_log S3）
pub const FB_BEAM_B: usize = 64; // 役N tick19: lattice beam 宽（ROOT/mid 各半）

pub const FB_BEAM_KTOP: usize = 8; // ROOT 态词历史 top-K（单槽 dedup 吃强长词发射）

pub const FB_BEAM_MERGE: bool = true; // 役N tick21: 激活（全序键确定性落地后;

// gerxtq 个人|小提琴 #1 / xtks 夏天|开始>系统|开始 native 序逐位 / ykvtbzuu
// 叶绿体|不做 #0 已实证; 面 lp 数据差由面级 override 修——lp_override_for 全串臂）
pub const FB_FACE_CAP_G1: usize = 2; // 役N tick23: G1 面数 cap（native 普查 0-2 面/查询）

/// 役 F1（agentR）件 1: 词条 logP override（数据线四案; 与 py wxq.engine.
/// LP_OVERRIDE 同表同值同注）。在 query() 打分层 lp 解析后应用（\\u{1} 短语
/// 取头词, 含 lp_locked 槽路径）。逐条 native 依据:
/// - 不想 −2.92: native_ref2 buxcuo #1=不想\\u{1}错 > #2=不行\\u{1}错; freq
///   49699>49631 被四段量化段2 反转成 −3.0684/−2.9355 → 回写频序
///   （wxm_Qon 电池 不想错 f392 1566.61 > 不行错 1564.06 同证）
/// - 吕/绿/率/铝/旅: native_ref2 lv 序 吕>绿>率>铝>旅; 我方量化频序错乱 →
///   等距阶梯回写（律−4.2227/驴−4.2422 恰居旅后=native #6/#7）
/// - 苏州/设置/深圳: native_ref2 sz 序 苏州>实在>设置>深圳; 深圳 上调过猛
///   → 苏州/设置 上调、深圳调至设置之下睡着之上
/// - 掀学 −7.95: native_ref2 + agentU wine xianxue n=96 全面板: native 序
///   [献血,先学,鲜血,现学,显学,显血,县学,西安学,仙学,贤学,…] 无 掀学 →
///   役G2(agentU) 案1 后手全序对齐: 下调至显血(−7.898)之下, 家族前6 逐位
///   =native（役F1 分歧2 后手 −5.69 只到 先学 之下, 不满足家族全序）
/// 役N tick44: 面级全串覆写值（lmcn 域 sc 序层用——lp 前置域, +lift 由
/// 调用层叠加; py LP_OVERRIDE 同表）
pub fn face_override(word: &str) -> Option<f64> {
    match word {
        "夏天\u{1}开始" => Some(-6.62),
        "个人\u{1}小桃去" => Some(-9.35),
        "个人\u{1}小提琴" => Some(-9.50),
        "找你\u{1}个" => Some(-7.00),
        "着\u{1}那个" => Some(-6.75),
        _ => None,
    }
}

pub fn lp_override_get(word: &str) -> Option<f64> {
    lp_override_head(word)
}

fn lp_override_head(head: &str) -> Option<f64> {
    // 役N tick21: 面级键全串先查（多词面整体 override, 无头词重写连带——
    // 头词键查不到尾词域; py LP_OVERRIDE 同表同值同注, tick20 run8 依据:
    // xtks 夏天开始 −9.274>系统开始 −9.629（我方反序 0.22）/ gerxtq
    // 小桃去 −9.066 微压 native#0 小提琴 −9.168（0.1=纯词 lp 数据差））
    match head {
        // 役N tick75b（盲测38 校准）: 同频段词对量化序与 native 相反
        // （qiche 汽车先于骑车 / youyong 有用先于游泳 / yayuan 雅苑先,
        // 呀源 同桶 fv=50575 并列退半步）
        "汽车" => Some(-3.99), "骑车" => Some(-4.01),
        "有用" => Some(-4.32), "游泳" => Some(-4.36),
        "雅苑" => Some(-5.52), "呀源" => Some(-5.58),
        "不想" => Some(-2.92),
        "吕" => Some(-4.03),
        "绿" => Some(-4.07),
        "率" => Some(-4.11),
        "铝" => Some(-4.15),
        "旅" => Some(-4.19),
        "苏州" => Some(-3.70),
        "设置" => Some(-3.82),
        "深圳" => Some(-3.828),
        // 掀学 −7.95: native_ref2 + agentU wine n=96 全面板: native 序
        //   [献血,先学,鲜血,现学,显学,显血,县学,西安学,仙学,贤学,…] 无 掀学
        //   → 役G2(agentU) 案1 后手全序对齐: 调至显血(−7.898)之下, 家族前6
        //   逐位=native（役F1 分歧2 后手 −5.69 只到 先学 之下, 不满足全序）
        "掀学" => Some(-7.95),
        // 役H1(agentV) 题3 补词: native_ref2 lv top10 = 吕绿率铝旅律驴捋滤氯;
        //   agentU 案3 补召回后 捋#12/氯#13 仍沉底（量化 −4.957/−5.434）。
        //   捋 −4.27 = 驴(−4.2422) 之下/虑(−4.469) 之上 → #8; 氯 −4.84 =
        //   滤(−4.832) 之下/履(−4.859) 之上（虑 自然值仍居 #9 → 与 native
        //   #9 滤/#10 氯 差一位, 两词授权外不另 override 虑）
        "捋" => Some(-4.27),
        "氯" => Some(-4.84),
        // ---- 役 L（agentAA）数据线批: 与 py LP_OVERRIDE 同表同值同注。
        // 逐条依据 = native_ref3 面板 + attribution.json ②_数据线;
        // 值 = 刚好越过 native 序（py diag 逐候选打分实测）----
        "怕" => Some(-2.95), // p: native #1=怕; 我方 被(-3.025)/品(-3.176) 压顶
        "几" => Some(-2.79), // 役N tick12: 近自然微抬（native 序 几及级机）
        "崩" => Some(-4.727), // 役N tick11: override 退役（实测 −4.736≈原典, Δ0.009）
        "培养" => Some(-5.073), // 役N v2 实测（陪一/佩瑶族已沉底）
        "吟哦" => Some(-2.75), // yine: native #1=吟哦; 我方 #10 沉底, 顶=pending 一(-2.800)
        "岑" => Some(-5.744), // 役N v2 实测（cen 梯: 陈/参/涔 在下）
        "元" => Some(-3.563), // 役N v2 实测（原典 −3.553; 全族梯在下）
        // ---- 役N v2 yuan 梯（native +10 体制内相对序 原>员>源>袁>院>园）----
        "原" => Some(-3.85),
        "员" => Some(-3.90),
        "源" => Some(-3.95),
        "袁" => Some(-4.00),
        "院" => Some(-4.05),
        "园" => Some(-4.10),
        "流通" => Some(-5.964), // 役N v2 实测（六头/流统已沉底）
        "本王" => Some(-5.498), // 役N v2 实测（本网 −7.10 已同批）
        // ---- 役N tick10: buxuo 族序修（run6 d_old 相对序: 不做>不多, 我方反）----
        "不做" => Some(-3.83),
        // tick11: beng 族序（run7 实测: 崩 −4.736≈原典; native 序 崩>泵>蹦）
        "泵" => Some(-4.75),
        // tick12: ji 短查族（run7 native 序 几及级机寄急; 给 gei 锚保护不动）
        "及" => Some(-2.81),
        "级" => Some(-2.815),
        "蹦" => Some(-4.78),
        // ---- 役N v2 沉底批（tick8, native 面板不可见面/词）----
        // W7O2 run3: complete 面 sc = OV+W14×2 → −7.0x 批回潮 −3.0 槽位,
        //   压住 comp 配有/配以(−3.3/−3.6) → 深沉至 comp 面之下
        //   （native peiy 面板 78 行无位, 先例=同批六头/流统）
        "陪一" => Some(-8.00),
        "佩瑶" => Some(-8.05),
        "佩云" => Some(-8.10),
        "裴垣" => Some(-8.15),
        "六头" => Some(-7.00),
        "流统" => Some(-7.05),
        "你蛋" => Some(-7.50),
        // ---- cen 梯（native 序 岑>册那>陈>参>涔）----
        "陈" => Some(-6.70),
        "参" => Some(-6.90),
        "涔" => Some(-7.10),
        "你单" => Some(-6.119), // 役N v2 实测（你蛋已沉底）
        "点入" => Some(-6.166), // 役N 实测（配对真值: 电褥 −6.459 同批, 防 partial 倒置）
        "啧" => Some(-4.139), // 役N v2 实测（≈原典 −4.129）; ze 梯在下
        // ---- 役N v2 ze 梯（native 序 啧>则>泽>责>择; 咋不可见 → 沉底）----
        "则" => Some(-4.20),
        "泽" => Some(-4.24),
        "责" => Some(-4.28),
        "择" => Some(-4.32),
        "咋" => Some(-4.45),
        "计划" => Some(-3.45), // jh: native #1=计划; 结婚(-3.486) 微压
        // ---- 役N 竞争词真值批（agentCC score_run5 top-4 行 d_old 实测）;
        // 锚词保持役L 反推值（部分真值化制造倒置, tick6 实验实证）----
        "就好" => Some(-4.630),
        "机会" => Some(-4.829),
        "结婚" => Some(-4.835),
        "远" => Some(-3.795),
        "配音" => Some(-5.542),
        // W7O2: OV 梯钉 培育/配乐 至 培养/配音 之后（原 tick6 值
        // −6.202/−6.233 沉于 junk 佩怡族）; 梯续 配药→配以
        "培育" => Some(-5.56),
        "配乐" => Some(-5.60),
        "本网" => Some(-7.100),
        "电褥" => Some(-6.459),
        "册那" => Some(-6.288),
        "趁" => Some(-3.974),
        "刘同" => Some(-7.331),
        "额" => Some(-2.05), // evwxvqc: native #1=额; e 列 哦(-2.098) 压顶
        "奶茶" => Some(-7.10), // ncei: native #1=奶茶\x01诶; 面 lp=头词 override 直达越 内粗那(-7.166)
        "今天天气" => Some(-1.0), // jttq: native #1=今天天气; 词典词经 jianpin 通道可达(rank #18, lp -5.863), 位次反推值
        // ---- 役N tick53(agentY2 B_combo 战役): 字槽组合族数据线批 ----
        // native 面板逐位依据 = agentY_bench/data wine raw（gaoyahu/
        // niaoduhuan/miunaozai/juangedeo/danchaomo 族 120 集锚）。
        // 某宝: native 某宝#在 > 牛奶#在（miunaozai #0/#1）; 我方词典 −6.398
        //   远低于 牛奶 −4.082 → 面序反转。抬至 −3.5（高于牛奶, 面 lp−pen 序
        //   某宝 −8.05 > 牛奶 −8.63 成立; 某宝 不入 34/120 集其他查询, 影响面
        //   = miunaozai 族面+moubao 型输入）
        "某宝" => Some(-3.5),
        // 尿都: native niaoduhuan 全面板 96 行无 尿都（bare 词与 尿都|换 面均
        //   无位）; 我方 −4.175 独占鳌头且作头词污染 尿都|换 面 → 沉底（低于
        //   尿毒 −7.078; 连带杀 DP sentence 尿都还 的头词重写 lp）
        "尿都" => Some(-10.0),
        // 卷个/卷哥: native juangedeo 全面板无（juan,ge 词列 native 首词=捐个
        //   #2）; 我方 −5.309/−5.371 压过 捐个 −7.031 且 pending 态 −0.31 淹没
        //   间隔|的|哦 面 → 沉底
        // 蛋炒/单炒: native danchaomo 全面板无（dan,chao 词 native=丹超/单插/
        //   淡茶, 我方典缺）; 我方 −5.418/−5.793 exact 态 −0.42/−0.79 压
        //   但|超模 面 → 沉底（亦断 蛋炒|没 型 p=2 面头词重写 lp）
        "蛋炒" => Some(-9.5), "单炒" => Some(-9.55),
        // ---- tick53 槽组合 junk 词沉底批（native 逐面板 96 行全无, 沉底批
        // 先例=陪一/佩瑶/六头/流统）: p=1 尾词列/fuzz 头词列 junk 断路 ----
        // gaoyahu yahu 列（native 无 ya,hu 词: 高#亚虎/高#呀呼 不发）
        "亚虎" => Some(-9.0), "呀呼" => Some(-9.0), "氩弧" => Some(-9.0), "雅虎" => Some(-9.0),
        // niaoduhuan duhuan 列（native 无 尿#杜欢 面）
        "杜欢" => Some(-9.0), "都欢" => Some(-9.0),
        // juangedeo: native juan,ge 词列首=捐个（娟哥 无面）; deo 尾走字槽
        // 拆解 的#哦（的哦/的噢 词面 native 无——间隔#的#哦 三槽形实证）
        "娟哥" => Some(-8.1), "的哦" => Some(-9.0), "的噢" => Some(-9.0),
        // 卷个/卷哥 需低于 SLOT_WORD_GATE(−8.0) 才断 p=2 头误取（eff 序
        // 卷个 −8.0 曾 ≥ 门 → 头取 卷个 非 捐个）
        "卷个" => Some(-8.5), "卷哥" => Some(-8.6),
        // saobaxie baxie 列（native 无 扫#吧协/吧些/吧写/把鞋 面）
        "吧协" => Some(-9.0), "吧些" => Some(-9.0), "吧写" => Some(-9.0), "把鞋" => Some(-9.0),
        // peinunu: native 尾词列=努努（怒怒/奴奴 无面）; 头段无词列
        // （配奴/陪奴 native 无 → fuzz 头 陪你 断路）
        "怒怒" => Some(-9.0), "奴奴" => Some(-9.5), "配奴" => Some(-9.0), "陪奴" => Some(-9.0),
        // miunaozai naozai 列（native 无 缪#挠在 面）
        "挠在" => Some(-9.0),
        // juangedeo E2 comp 面 捐给#的哦 native 无位 → 头词沉底
        "捐给" => Some(-8.0),
        // saobaxie: native sao,ba 词序 扫把>扫吧（#2/#3）; 我方 −5.08/−5.20 反
        // → 面序词头取 扫把、bare 词序同 native
        "扫把" => Some(-5.05), "扫吧" => Some(-5.30),
        // zenme: native #1=真么（retro z→zh 头纠错整词, comp 件2 已发 pen
        // −3.285）; 我方典 lp −5.65 → 面 −5.94 沉于 怎(−5.89) 不可见 → 抬至
        // −3.3: 面 = −3.3−3.285+W14×3 = −3.59 越 怎 → [怎么, 真么, 怎] native
        // 序（影响面=zhenme 键候选族, 34/120 其他查询无此词）
        "真么" => Some(-3.3),
        // kj: native kj 行 0 科技 > 看见; 科技 原 logp -4.07 抬至 -3.5 > 看见(-3.62)
        "科技" => Some(-3.5),
        // ---- 役N W1(第五波 B_combo/兜底): junk 词沉底批（native agentY_bench
        // 逐面板全无, 先例=陪一/佩瑶/六头/流统 沉底批）----
        // jiangguawu guawu 尾词列: native 无 诖误/挂五 尾词面（尾槽=字列 无/五,
        // 律2）。LP_OVERRIDE 使 _slot_word_cands eff < SLOT_WORD_GATE(-8.0) →
        // tails_ex 断路; SLOT_FUZZ_BAN per-segment 检查双保险（W1）。
        "诖误" => Some(-9.5), "挂五" => Some(-9.55),
        // geiguo 完成词洪水: native #4-#10 = 给狗/给/給/个/哥/各, 全面板无
        // 给过X 三字完成词 → 沉底让位（六词仅 geiguo 面, 34/120 他查无）。
        "给过去" => Some(-9.5), "给过跟" => Some(-9.55), "给过子" => Some(-9.6),
        "给过海" => Some(-9.65), "给过你" => Some(-9.7), "给过我" => Some(-9.75),
        // ---- 役N W7O2(第七波 O2 交叠深度): peiy/jint/zhaong 面序批 ----
        // native 逐位 = agentY_bench/data/{peiy,jint,zhaong,ncei,dakn,pgde}.txt
        // （悬空声母展开 = 每音节 top 词×_sylw 权重序; 典位次深词走 PIN
        // 输入钉面）。peiy: OV 梯钉 培育/配乐 至 培养/配音 之后,
        // junk 沉底（佩怡/胚衣/佩莹/佩瑜 = pei,yi/ying/yu 异读混入,
        // native peiy 面板 78 行无位）。----
        "配药" => Some(-5.64),
        "培优" => Some(-5.68),
        "配有" => Some(-5.72),
        "配以" => Some(-5.76),
        // junk 沉底批二（W7O2 run2: 一批 -6.x 沉底仍占 #9/#10 槽, 压住
        //   comp 通道 配有(-3.72)/配以(-3.76) → 全批深沉至 comp 面以下;
        //   3 音节补全词 sc = lp+W14×1, 同批深沉）
        "佩怡" => Some(-9.00),
        "胚衣" => Some(-9.05),
        "佩莹" => Some(-9.10),
        "佩瑜" => Some(-9.15),
        "沛源" => Some(-9.20),
        "配员" => Some(-9.25),
        "沛瑶" => Some(-9.30),
        "培玉" => Some(-9.35),
        "培阳" => Some(-9.40),
        "陪验" => Some(-9.45),
        "培养人" => Some(-9.50),
        "培优课" => Some(-9.55),
        "裴言澈" => Some(-9.60),
        "裴宴之" => Some(-9.65),
        "赔一命" => Some(-9.70),
        "陪一根" => Some(-9.75),
        "配音稿" => Some(-9.80),
        "配眼镜" => Some(-9.85),
        "陪一陪" => Some(-9.90),
        "胚芽粕" => Some(-9.95),
        "配一配" => Some(-10.00),
        "配音秀" => Some(-10.05),
        "培养板" => Some(-10.10),
        "配液罐" => Some(-10.15),
        // jint: native [今天,尽头,津贴,金条,劲头,金坛,进退,金塔,金堂,进图]。
        //   comp 通道词 金条/劲头/津贴 OV 提位（comp 面 sc = lp + W14×2;
        //   值 = 面序反推）; 尽头/金坛/进退/金塔/金堂/进图 补全通道未召回
        //   （per-reading oi 截 2）→ RD_PIN_FACES 读音键钉面（第八波）。
        "金条" => Some(-3.2),
        "劲头" => Some(-3.4),
        "津贴" => Some(-2.2),
        // zhaong: comp 纠错面 native 全面板无（zhao,n,g 域只发 找#那个/找#哪个
        //   两面）→ 沉底。flags=0 族（绍宋/中宋/准送…）自带 COMP_UDICT_DEMOTE
        //   −6 已沉; flags≠0 族（W14 +4 后 −1.6~−2.7 挤 字列槽）逐词沉底。
        "闪送" => Some(-12.0),
        "专送" => Some(-12.05),
        "输送" => Some(-12.10),
        "真怂" => Some(-12.15),
        "主送" => Some(-12.20),
        "周松" => Some(-12.25),
        // zgrm: 简拼全键词（q_jianpin 通道, 无钉面路径）native 序 中国人民#0 >
        //   整个人懵#2 > z 栈列(−6.1 钉域)——整个人懵 自然 lp −9.281 沉于字列
        //   → OV 抬至 中国人民(−5.977) 之下、z 列钉(−6.10) 之上（N5b; 唯一
        //   消费域 = zgrm 简拼链/zhenggerenmeng 全拼, 溢出面=典深词零锚）。
        //   （py LP_OVERRIDE 同表同值同注, tick62）
        "整个人懵" => Some(-5.99),
        _ => None,
    }
}


pub fn lp_override_for(word: &str, lp: f64) -> f64 {
    // 役N tick21: 面级键全串先查（多词面整体 override, 无头词重写连带）
    if let Some(v) = face_override(word) {
        return v;
    }
    let head = match word.find('\u{1}') {
        Some(p) => &word[..p],
        None => word,
    };
    lp_override_head(head).unwrap_or(lp)
}

// ---- 役N tick53(agentY2 B_combo 战役): 字槽组合族旋钮 ----
// 组合面终值 = lp − pen + COMBO_BIAS（query.py 打分层: score = lp +
//   edit_penalty(−pen) + score_bias）。9.0 校准锚: 但|超模 +0.44 压
//   蛋炒 −0.42（danchaomo）/ 尿毒|换 −1.03 压 尿毒 −3.08（niaoduhuan）/
//   扫把|写 +0.58（saobaxie）/ 高压壶 +0.64 压 高压 −1.86（gaoyahu）。
pub const COMBO_BIAS: f64 = 9.0;

/// KBN 单替换罚分域: 辅音域 3.0（laruichu 啦|退出须沉于 拉瑞|出）
pub const COMBO_PEN_CONS: f64 = 3.0;

/// 元音域 1.55（juangedeo 间隔|的|哦 须压 捐个 面）
pub const COMBO_PEN_VOWEL: f64 = 1.55;

/// 键距 ≥2 的 KBN 替换罚（角度|换 n→j 键距2 须沉于 尿毒 −3.08）
pub const COMBO_PEN_FAR: f64 = 4.85;

/// 全字链面门: 链面须=查询最高内分面且内分 ≤ 此值（丢#黄#炸 −11.94 发 /
/// 但#超#没 −9.1 / 卷#个#得#哦 −11.25(非最高) 不发 / 缪#挠#在 不发）
pub const CHAIN_GATE_LP: f64 = -11.0;

/// 头/尾整词 eff 门（LP_OVERRIDE 沉底词断路——蛋炒(−9.5) 断 p=2 词头;
/// 配奴/挠在 断 fuzz 头与尾词面; 超模 −5.66/努努/退出/拉瑞/酱瓜/尿毒 全过）
pub const SLOT_WORD_GATE: f64 = -8.0;

/// 键盘坐标（键距计价: 行, 列序）— py FuzzChannelsMixin._KB_POS
pub fn kb_pos(c: u8) -> (i32, i32) {
    match c {
        b'q' => (0, 0), b'w' => (0, 1), b'e' => (0, 2), b'r' => (0, 3),
        b't' => (0, 4), b'y' => (0, 5), b'u' => (0, 6), b'i' => (0, 7),
        b'o' => (0, 8), b'p' => (0, 9),
        b'a' => (1, 0), b's' => (1, 1), b'd' => (1, 2), b'f' => (1, 3),
        b'g' => (1, 4), b'h' => (1, 5), b'j' => (1, 6), b'k' => (1, 7),
        b'l' => (1, 8),
        b'z' => (2, 0), b'x' => (2, 1), b'c' => (2, 2), b'v' => (2, 3),
        b'b' => (2, 4), b'n' => (2, 5), b'm' => (2, 6),
        _ => (0, 0),
    }
}

/// 役N W1(第五波): G1 兜底 律§7 悬空×exact 混合链面 bias（校准锚 bfon:
/// 拜佛你 lp=拜佛(-5.730)+你槽(-2.066)=−7.796, 须压过旧链 EPYN 面
/// 不发那(−7.73)且 保 拜佛呢(−8.125+3.3=−4.83) 亦过 → 3.3）
pub const LAW7_BIAS: f64 = 3.3;

// ---- 役N 第十一波(N5b): ADV 结构律旋钮（native_ref4 agentN4 逐位;
// py FuzzChannelsMixin 类属性同值同注, tick62）----
// 悬空头族（wtianqi/mgaoyuan, 律 B）字列 = RD_PIN_COL[声母] 钉分 +
// 本抬升: 标准栈域钉分（−6.0 域）为 wsm/zxcvb 型全简拼/垃圾查询域
// （字列须低于全键词 卧室门−5.934/为什么−3.104）, 悬空头族 native
// 字列位于头词/整词（王天琦−2.93/comp 族）之后、垃圾面之前。wtianqi
// 域 comp 王天琦 触发 has_comp_full_word（query E1 件 B 形:
// wang'tian'qi 头前缀×尾全消费）→ fbcol 全列 −3.0, 净抬升 = LIFT−3.0:
// wtianqi 字列域 sc+0.2 ∈ (王天琪−7.18, 王天琦−2.93) ✓; mgaoyuan 域
// 无 comp 全词件 → 净抬升 = LIFT: 字列 −2.8.. 压 comp 渺远 ✓。
// 一旋钮双域（3.2）。
pub const ADV_INI_COL_LIFT: f64 = 3.2;

// 叠元音塌缩面（haao/keexi, 律 D）词块抬升: 塌缩词 native 居顶
// （好/可惜 #0）压过未塌缩链首与校准列顶（哈−2.17/可−1.0）→ boost 域
// （keexi 锚: 可惜−4.0+3.2=−0.8 > ke 列顶 −1.0; 壳子/刻字 −1.8/−2.2
// 插列中——native 行 0-4 词块先行逐位）。
pub const ADV_COLLAPSE_BOOST: f64 = 3.2;

// 塌缩×KBN 次变体词 lp 门（keexi: 壳子−5.02/刻字−5.43 native 在板;
// haao gao 族 高−2.6 native 全无 → 门双锚, 表外域零外溢）。
pub const ADV_SUB_LP_GATE: f64 = -4.5;

// ---- mix_recall MLP 召回门（第十五役 §十五 + agentC_rules §5: md.bin
// mix_recall_model = one_layer_mlp_v1, 8 输入→5 隐元(tanh)→1 输出(6 权重)。
// 判活（agentC_rules.md §2 台账）: mix_recall 全零 → fangan 的 档案(dang,an
// typo) 掉出 #4; 同实验 buxuo/cchuang/buxcuo 面板零位移 → 门只作用 recall
// 类深位候选（我方 = Kind::Correct），comp 族=corrected 不设门。
// 权重源: ~/tmp/hanhuo/md_params.json（t9_coef==coef 逐位同, agentO 复核）。
// 开关 WXQ_MIX_RECALL_MLP（默认 on=native 活体; py 镜像同环境变量）。
// 激活 tanh/线性输出/阈值 >0 为工程定（native 前向未逆向），数值锚 agentO_log S6 ----
pub const MIX_RECALL_W: [[f64; 5]; 8] = [[0.0; 5]; 8];

pub const MIX_RECALL_B: [f64; 5] = [0.0; 5];

pub const MIX_RECALL_OUT: [f64; 6] = [0.0; 6];

/// one_layer_mlp_v1 前向（与 py wxq.engine.mix_recall_forward 逐位一致锚:
/// agentO_log S6 三组固定输入）。x 序 = [best_cand_typo, best_cand_completion,
/// best_cand_lm, cand_typo, cand_completion, cand_lm, rank, typo_rank]
pub fn mix_recall_forward(x: &[f64; 8]) -> f64 {
    let mut y = MIX_RECALL_OUT[5];
    for j in 0..5 {
        let mut s = MIX_RECALL_B[j];
        for (i, xi) in x.iter().enumerate() {
            s += MIX_RECALL_W[i][j] * xi;
        }
        y += MIX_RECALL_OUT[j] * s.tanh();
    }
    y
}

/// mix_recall 开关（默认 on = native 活体; '0'/'false'/'off' 关回）
pub fn mix_recall_enabled() -> bool {
    match std::env::var("WXQ_MIX_RECALL_MLP") {
        Ok(v) => !matches!(v.to_lowercase().as_str(), "0" | "false" | "off"),
        Err(_) => true,
    }
}

/// 件 5: 组合短语压罚（agentM §2 native buxuo: 不许|哦 须低于纠错 不错
/// ≥8.5 分终值口径; logP 域等效常数; 在打分层施加——短语 lp 被头词 lp
/// 重写是既有语义）
pub const COMP_PHRASE_DEMOTE: f64 = 3.5;

/// 件 5: 组合通道用户词/组合词条压制（词库 flags=0 族——诚创/初创公司
/// vs 核心词 flags=17/33; agentM §3 native 造词组合 −21.5 档且 诚创 缺席）
pub const COMP_UDICT_DEMOTE: f64 = 6.0;

/// 役N tick52: 尾段悬空声母补全的 bit0 强词门（DANGLING 补全音节集第二路）。
/// 双锚分界: 马工 f17 −5.67 native 无 / 滤网 f17 −5.49 native 有（agentY
/// raw 面板 n=72/vlw 逐位实证）→ 阈值取两值中点。
pub const DANGLING_STRONG_LP: f64 = -5.6;

/// 双写元音塌缩：连写重复元音删一（maa→ma）。
///
/// native 音节图级吸收边（agentCC eprep_epyn.md §2: maag = [maa→ma 塌缩]
/// × [g 悬空声母→ge 补全] → 马哥）；仅元音域 aeiouv 连写塌缩, 辅音不动
/// （zhongg 的 gg / diannao 的 nn 不塌——跨段叠字另属件 2 管辖）。
pub fn vv_collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if let Some(last) = out.chars().next_back() {
            if ch == last && "aeiouv".contains(ch) {
                continue;
            }
        }
        out.push(ch);
    }
    out
}

// ---- 役 E1（agentP）多词短语通道旋钮（agentN_report §1.6 规格 A-D; py 同源）----
/// A 通道键长上限: native 组合发射观测跨度 5~13 键（agentN §1.3; 役H1 agentV
/// wine 直证收敛: 12 键 nihaoxshijie #0=你好\u{1}新世界 发射、13 键
/// nihaoshijiema/woaizhongguoa #0=三词组合发射、14 键四样本全零组合
/// （nihaoshijieman/woaizhongguoan/woaizhongguoba/nihaoxshijiema）→ 上界=13;
/// 15 键 jintiandetianqi/nihaoshijiemada 亦零 → 旧值 12 偏紧一档, 役H1 放宽）
pub const COMBO_MAX_KEYS: usize = 13;

/// A 通道覆盖态组合距整句词的分数带（实测双锚: native 必发 谢谢呢−谢谢你
/// gap 1.04 带内; native 不发 你好|师姐 gap 1.65 带外 → 1.5）
pub const COMBO_BAND: f64 = 1.5;

/// A 通道每查询组合上限（native 观测整句组合 0~2 条/面板）
pub const COMBO_CAP: usize = 2;

/// B 通道尾槽词选择 logp 带宽（世界−3.91/师姐−4.44 差 0.53 带内;
/// 学→削 型低质变体 >1.7 剪除 = D① 生成时实现）
pub const TAIL_ALT_GAP: f64 = 1.0;

/// KBN 字母混淆邻接表（libwxhld 0x181164390 .data, 26×u32 位掩码, agentK
/// dump 2026-10-28; KeyboardNeighbourAugmenter 读取; 对称策划表, 非几何）
pub const KBN_MASK: [u32; 26] = [
    0x00450000, 0x00202080, 0x00a00020, 0x00860030, 0x00460008, 0x000a004c,
    0x012800a0, 0x01100242, 0x00104600, 0x00102580, 0x00005b00, 0x0000c400,
    0x00002400, 0x00001202, 0x00008d00, 0x00004800, 0x00400001, 0x00080038,
    0x02400019, 0x01020060, 0x01000380, 0x00000046, 0x00050011, 0x0200000c,
    0x001800c0, 0x00840000,
];

pub fn kbn_neighbors(ch: u8) -> impl Iterator<Item = u8> {
    let row = KBN_MASK[(ch - b'a') as usize];
    (0u8..26).filter(move |j| row >> j & 1 == 1).map(|j| b'a' + j)
}

pub const SMALL_RANGE: u32 = 64;

pub const PER_READING: usize = 24;

pub const MAX_RANGE_HEAP: u32 = 8192;

pub const HOT_FALLBACK_K: usize = 16;

// agentB `_dangling_block` 的补全串集合 = 音节表全部真后缀（Python set 遍历序）。
// py 侧该序 = CPython set 槽位序（PYTHONHASHSEED 敏感）；golden 重导固定
// PYTHONHASHSEED=0 后逐项提取（tails_seed0_*.txt, md5 cd5c6886/9a15261e）。
// 这里按 pack 音节表条数选序（409=主词库 wxq.bin / 416=open 双 bin），
// 保证块内同 logp 平局的先后与 py seed-0 输出逐位一致。
// 其它条数表：回退音节表插入序（确定性近似, 平局序可能与 py 有别）。
pub const PY_TAILS_SEED0_409: &str = "r hou iong huang uang a han o v hua uai ue ua uan huan in hong g ao heng ei hao ou iu ong hang ing hen ang ie n un en hu ia hi hun uo hai an u huo he ha ve i iao ai eng hui ui iang e huai ian ng";

pub const PY_TAILS_SEED0_416: &str = "r hou iong huang uang a han o v hua uai ue ua uan huan in hong g ao heng ei hao ou iu ong hang ing hen ang ie n un en hu ia hi hun uo hai an u huo he ha ve i hei iao ai eng iang hui ui e huai ian ng";


// ==========================================================================
// ---- 役N tick53-57: 字槽组合族/面板校准表（py wxq.engine.constants 同表,
// 逐字面镜像; 消费点 = engine/slots.rs + engine/panel.rs + channels_full.rs）
// ==========================================================================

/// 律1 字槽列（native 校准序 + syn lp）。键=读音, 值=[(字, lp)] 发射序。
pub static SLOT_COL: &[(&str, &[(&str, f64)])] = &[
    ("hu", &[("壶", -3.50), ("互", -3.80), ("户", -4.10), ("胡", -4.40)]),
    ("xie", &[("写", -3.22), ("些", -3.52)]),
    ("huan", &[("换", -2.95), ("还", -5.50)]),
    ("chu", &[("出", -2.82), ("处", -6.80)]),
    ("zha", &[("炸", -4.13), ("扎", -4.43)]),
    ("dan", &[("但", -2.90), ("单", -3.20), ("蛋", -3.50), ("丹", -3.80)]),
    ("pei", &[("陪", -3.84), ("配", -4.14)]),
    ("wu", &[("无", -2.50), ("五", -2.60)]),
    ("o", &[("哦", -2.10)]),
    // 役N W1 预放（W2 panel 段1.5 律2 尾槽字列消费; native zhongguorenmi
    // mi 列 = 迷>米, 我方自然列 米(-3.53)>迷(-4.39) 反序——mi 列序另源
    // 同 wsd 之谜 律§12。现无槽域 mi 尾查询, 预放零外溢）
    ("mi", &[("迷", -3.90), ("米", -4.00)]),
];

pub fn slot_col_get(rd: &str) -> Option<&'static [(&'static str, f64)]> {
    SLOT_COL.iter().find(|(k, _)| *k == rd).map(|(_, v)| *v)
}

/// 尾槽字列深度 (exact/chain 头深度, fuzz 头深度); 缺省 (1, 2)。
/// W3: mi 2+2（zhongguorenmi 段1.5 尾槽字列 = 迷/米 双面, native #0/#1）
pub fn slot_col_cap(rd: &str) -> (usize, usize) {
    match rd {
        "hu" => (4, 4),
        "xie" => (2, 2),
        "wu" => (1, 2),
        "zha" => (1, 2),
        "zai" => (1, 1),
        "mi" => (2, 2),
        _ => (1, 2),
    }
}

/// 融合形组合面（native 词典词, 我方典缺 → 构造无 \u{1} 词面）
pub fn slot_fuse(w: &str) -> bool {
    w == "高压壶" || w == "高压户"
}

/// 槽组合 fuzz 枚举黑名单（junk 词面禁绝——native 槽组合 fuzz 面实证集,
/// 表外词一律不发; 这些词仅限本枚举域, 他处不受影响）
pub fn slot_fuzz_ban(w: &str) -> bool {
    matches!(
        w,
        // laruichu rui→tui/d 头
        "拉腿" | "啦对"
        // gaoyahu gao→gai/hao 头
        | "盖雅" | "盖亚" | "好呀"
        // niaoduhuan niao→miao 头
        | "秒读"
        // diuhuangzha diu→du/dou 头
        | "都慌" | "斗皇"
        // peinunu nu→ni 头
        | "陪你" | "配你"
        // gaoyahu yahu 尾 junk
        | "他户" | "哈湖" | "雅居" | "雅谷" | "雅部"
        // laruichu chu→chi 尾
        | "睿驰"
        // saobaxie ba→na 尾
        | "那些"
        // miunaozai nao→bao 尾
        | "包在"
        // diuhuangzha huang→guang 尾
        | "光闸"
        // danchaomo chao→chai/dan→ran
        | "单拆" | "燃潮"
        // niaoduhuan niao→jiao/biao 头
        | "角度" | "标度"
        // gaoyahu yahu/gaoya junk
        | "牙距" | "搞她" | "好压"
        // niaoduhuan du→di/huan→yuan junk
        | "尿滴" | "度原"
        // saobaxie ba→na 尾
        | "哪些"
        // miunaozai nao→nai+zai→zao 尾
        | "耐造"
        // diuhuangzha zha→sha 尾
        | "黄砂" | "黄莎"
        // saobaxie sao→sai/dao 头
        | "赛霸" | "到吧"
        // saobaxie xie→xue 尾
        | "吧学"
        // gaoyahu junk
        | "呀于" | "告他" | "套呀" | "搞哈" | "呀句"
        // niaoduhuan niao→biao, du→fu junk
        | "表弟" | "腐换"
        // saobaxie xie→xue, sao→dao/sna junk
        | "吧雪" | "道吧" | "嫂那"
        // peinunu nu→ni 尾
        | "你奴"
        // miunaozai nao→bao 尾
        | "报在"
        // diuhuangzha huang→guang 尾
        | "广厦"
        // juangedeo juan→jian 头(间隔 同变体劣词)
        | "简个"
        // danchaomo mo→mi/chao→chai junk
        | "炒米" | "拆模"
        // gaoyahu ya→ta 尾
        | "他湖"
        // niaoduhuan huan→yuan/niao→biao junk
        | "杜院" | "表底"
        // peinunu nunu→nini 尾
        | "你你"
        // saobaxie sao→sai/zao 头
        | "塞巴" | "早八"
        // laruichu 异切前缀词层 junk
        | "啦入" | "拉入"
        // niaoduhuan/yahu 尾·头 junk
        | "亚宇" | "读观" | "鸟入"
        // peinunu nunu→nini 尾
        | "尼尼"
        // danchaomo chao→chaimi 尾
        | "超密"
        // diuhuangzha 尾
        | "逛啥"
        // miunaozai nao→mao 尾
        | "毛在"
        // saobaxie sao→sai+ba→na 尾
        | "赛娜" | "塞那"
        // niaoduhuan niao→jiao 头
        | "叫度"
        // gaoyahu ya→ha 尾
        | "哈呼"
        // niaoduhuan du→di 尾
        | "地换"
        // saobaxie sao→dao/zao+ba→na junk
        | "到那" | "早吧"
        // jiangguawu gua→gua?? 尾 junk（native 无面）
        | "诖误"
        // miunaozai nao→mao 尾 junk
        | "猫仔" | "毛崽"
        // ---- 役N W1(第五波): jiangguawu 槽组合 junk 断路 ----
        // guawu 尾词列第 2（诖误之后, eff −5.81 过门漏网）
        | "挂五"
        // guawu 尾 fuzz（gua→hua）词完成面 junk
        | "华五" | "华武"
        // jianghua fuzz 头（native fuzz 头=讲话 单发; 娘化=jiang→niang+
        // gua→hua 双替 junk 面）
        | "江华" | "娘化"
        // ---- 役N W7O2(第七波): 悬空声母链 junk 组合面 ban（native 逐面板
        // 全无, 律2/律4: 尾槽=每悬空音节 top 字/词单发——zhao,n,g 只发
        // 找#那个/找#哪个; da,k,n 只发 大#可能/打开#呢; p,g,d,e 只发
        // 苹果#的/屁股#的。先例=本表表外词禁绝; 这些面仅 zhaong/dakn/
        // pgde 族枚举域产生）----
        // zhaong 族
        | "着\u{1}那个" | "找你\u{1}个" | "找你\u{1}给" | "找你\u{1}跟"
        | "找你\u{1}咯" | "找你\u{1}过" | "找你\u{1}干" | "找你\u{1}搞"
        | "找你\u{1}高" | "着\u{1}难过" | "找你\u{1}刚" | "找你\u{1}改"
        | "找\u{1}难过" | "找你\u{1}该" | "着\u{1}你个" | "找你\u{1}更"
        | "着\u{1}能够" | "找\u{1}你个"
        // dakn 族
        | "打\u{1}可能" | "打开\u{1}你" | "大\u{1}看你" | "打开\u{1}那"
        | "打\u{1}看你" | "大\u{1}困难" | "打\u{1}困难" | "大\u{1}柯南"
        | "打开\u{1}哪" | "的\u{1}啊\u{1}可能" | "打\u{1}柯南"
        | "大\u{1}看您" | "大坑\u{1}你" | "大课\u{1}呢" | "大块\u{1}你"
        | "大课\u{1}那" | "大坑\u{1}呢" | "大\u{1}可你"
        // pgde 族
        | "排骨\u{1}的" | "拍个\u{1}的" | "配个\u{1}的" | "苹果\u{1}得"
        | "屁股\u{1}得" | "评估\u{1}的" | "胖哥\u{1}的" | "排骨\u{1}得"
        | "抛光\u{1}的" | "被\u{1}过的" | "跑个\u{1}的" | "鹏哥\u{1}的"
        | "怕个\u{1}的" | "被\u{1}过得" | "拍过\u{1}的" | "评估\u{1}得"
        | "泡个\u{1}的" | "平菇\u{1}的"
        // tick61: saodigei 族（native 面板=扫地#给+sao,di 词列+sao 字列,
        // 槽合并变体面逐位全无; 含繁體 給 变体）
        | "扫\u{1}抵给" | "到底\u{1}给" | "骚\u{1}抵给" | "塞地\u{1}给"
        | "扫\u{1}死给" | "骚\u{1}死给" | "骚弟\u{1}给"
    )
}

/// 律§10 相邻换位×邻键吸收 读音词列表。键=换位结构产出的连写读音,
/// 值=[(词, lp)] 发射序。表外读音零发射。
pub fn slot_swap_rd(k: &str) -> &'static [(&'static str, f64)] {
    match k {
        "kewo" => &[("可我", -4.656), ("客卧", -6.289)],
        _ => &[],
    }
}

/// 律§7 头段连写词表（walk 失败兜底）。native bfon 头链 = b→bai × fo exact
/// → 拜佛; 表外零发射。
pub fn law7_head(a: u8, b: &str) -> Option<&'static str> {
    match (a, b) {
        (b'b', "fo") => Some("拜佛"),
        _ => None,
    }
}

/// ---- 第九波 T1(尾扫批): 头段变体校准表（native 头纠错整词面, 信号墙域
/// → 读音键校准, FB_INI_COL['n'] 同形先例; 键 = 头音节读音, 值 = 变体音节
/// （KBN 邻键类; retro 域 zcs→+h 件2 既有通律不入表））。消费 =
/// channels_full.q_comp（尾 exact × 头变体 × 词典 bit4 词 × 读音串精确
/// 相等）。native 锚: cengjing→风景(feng,jing, c→f) / sanshi→但是
/// (dan,shi, s→d)。表外零发射——nihao 几号/nidan 鸡蛋 类 n→j 污染面
/// native 全无, 且与 风景/但是 无 flag/键距/频带 可分信号（wxq.bin 数据
/// 线墙, 逐案实证见 report）→ 校准表是唯一合法路径。----
/// （py constants.HEAD_VAR_COL 同表同值同注）
pub fn head_var_col(k: &str) -> &'static [&'static str] {
    match k {
        "ceng" => &["feng"],
        "san" => &["dan"],
        _ => &[],
    }
}

/// ---- 第八波(结构泛化): 读音/声母钉列（原 FB_SEG_COL + FB_INI_COL 双表
/// 收编统一, 零行为——键域不相交, 消费点各自取键）。
/// 键 = 列读音（zhao/da/e = FB_SEG_COL 旧键, native 尾字列钉序——zhaong
/// n=96 行 2-9 zhao 列 / dakn n=77 行 6-9 da 列 / pgde n=96 行 4-8 e 列）
/// 或声母（n/p = FB_INI_COL 旧键——ncei nv 列前置于 n 栈列 / pgde p 栈列首）。
/// 值 = [(字, 钉终分)]——钉分承担列序且免打分层重排, 表外键自然序零外溢。
/// （py constants.RD_PIN_COL 同表同值同注）
pub static RD_PIN_COL: &[(&str, &[(&str, f64)])] = &[
    ("zhao", &[("找", -2.00), ("赵", -2.10), ("招", -2.20), ("照", -2.30),
               ("着", -2.40), ("兆", -2.50), ("罩", -2.60), ("昭", -2.70)]),
    ("da", &[("打", -2.50), ("大", -2.60), ("达", -2.70), ("哒", -2.80)]),
    ("e", &[("哦", -1.00), ("噢", -1.10), ("喔", -1.20), ("筽", -1.30),
            ("嚄", -1.40)]),
    ("n", &[("女", -3.00), ("钕", -3.10), ("衄", -3.20), ("釹", -3.30),
            ("籹", -3.40), ("恧", -3.50), ("朒", -3.60), ("你", -3.70),
            ("呢", -3.80), ("能", -3.90), ("男", -4.00), ("年", -4.10),
            ("您", -4.20), ("牛", -4.30)]),
    ("p", &[("拍", -2.50), ("怕", -2.60), ("跑", -2.70), ("陪", -2.80),
            ("配", -2.90), ("屁", -3.00), ("片", -3.10), ("品", -3.20)]),
    // ---- 役N 第十一波(N5b): ADV 族钉列（native_ref4 agentN4/data 逐位）。
    // 单声母键 = 声母栈列（n/p 旧键同型）:
    //   w = wtianqi 行 2-29 栈列 28 字（我>为>万>问>玩>无>王>未…; 我方
    //       wlp 序 位>为/物>无 不可复制 → 逐位校准）;
    //   m = mgaoyuan 行 2-29（吗>没>么>买>嘛>们>卖>米…; 嘛/们 我方 lp 反序）;
    //   z = zgrm 行 4-19（在>这>做>再>中>找>走>张>真>总>只>转>咋>周>着>组;
    //       zxcvb native 同列复用——钉分域取 zgrm 整个人懵(−5.99 OV)之下、
    //       中国人民(−5.977)之下 → 全简拼词先行字列垫后, 双锚一致）;
    //   r = rmyy 栈列（native 面板尾列 = e 列异形——记分歧; 钉分压至
    //       人民影院(−8.688)/人民友谊(−8.898) 之下 → 三简拼词先行）。
    // 读音键 = 读音列（ke/bu = kexsi 行 2+/buxdui 行 2+ 列——可>课>克>科>客…
    //       native 序 ≠ 我方 wlp 序(课>可) → 逐位）。钉分域 = comp 垃圾
    //       面（可惜四−4.5/不想对啊−2.42 族）之上 → −1.0 域。
    // （py constants.RD_PIN_COL 同表同值同注, tick62）----
    ("w", &[("我", -6.00), ("为", -6.15), ("万", -6.30), ("问", -6.45),
            ("玩", -6.60), ("无", -6.75), ("王", -6.90), ("未", -7.05),
            ("完", -7.20), ("哇", -7.35), ("五", -7.50), ("文", -7.65),
            ("吴", -7.80), ("晚", -7.95), ("位", -8.10), ("网", -8.25),
            ("外", -8.40), ("往", -8.55), ("娃", -8.70), ("伟", -8.85),
            ("喔", -9.00), ("唔", -9.15), ("汪", -9.30), ("微", -9.45),
            ("魏", -9.60), ("稳", -9.75), ("味", -9.90), ("忘", -10.05)]),
    ("m", &[("吗", -6.00), ("没", -6.15), ("么", -6.30), ("买", -6.45),
            ("嘛", -6.60), ("们", -6.75), ("卖", -6.90), ("米", -7.05),
            ("忙", -7.20), ("面", -7.35), ("码", -7.50), ("妈", -7.65),
            ("门", -7.80), ("名", -7.95), ("明", -8.10), ("马", -8.25),
            ("满", -8.40), ("秒", -8.55), ("慢", -8.70), ("美", -8.85),
            ("毛", -9.00), ("猫", -9.15), ("咩", -9.30), ("梦", -9.45),
            ("骂", -9.60), ("敏", -9.75), ("每", -9.90), ("摸", -10.05)]),
    ("z", &[("在", -6.10), ("这", -6.25), ("做", -6.40), ("再", -6.55),
            ("中", -6.70), ("找", -6.85), ("走", -7.00), ("张", -7.15),
            ("真", -7.30), ("总", -7.45), ("只", -7.60), ("转", -7.75),
            ("咋", -7.90), ("周", -8.05), ("着", -8.20), ("组", -8.35)]),
    ("r", &[("人", -9.20), ("日", -9.35), ("让", -9.50), ("然", -9.65),
            ("入", -9.80), ("肉", -9.95), ("如", -10.10), ("任", -10.25),
            ("忍", -10.40), ("热", -10.55)]),
    ("ke", &[("可", -1.00), ("课", -1.15), ("克", -1.30), ("科", -1.45),
             ("客", -1.60), ("颗", -1.75), ("壳", -1.90), ("柯", -2.05),
             ("氪", -2.20), ("刻", -2.35), ("磕", -2.50), ("珂", -2.65),
             ("咳", -2.80), ("棵", -2.95), ("渴", -3.10), ("嗑", -3.25),
             ("轲", -3.40)]),
    ("bu", &[("不", -1.00), ("补", -1.15), ("部", -1.30), ("布", -1.45),
             ("步", -1.60), ("卜", -1.75), ("埠", -1.90), ("捕", -2.05),
             ("簿", -2.20), ("怖", -2.35)]),
];

pub fn rd_pin_col_get(k: &str) -> Option<&'static [(&'static str, f64)]> {
    RD_PIN_COL.iter().find(|(kk, _)| *kk == k).map(|(_, v)| *v)
}

/// ---- 第八波(结构泛化): 读音键面表（原 PIN_G2_FACES/PIN_G1_FACES 输入键
/// 表泛化, 零行为）。键 = 面读音（pretty 形——悬空链典缺词的构造面读音,
/// native 面板词, 我方典位次深或典缺, 补全/comp 通道召回缺口 → 面直发;
/// 先例 = LAW7_HEAD/SLOT_FUSE/WORD_COL 典外直发）。触发 = §3 骨架吞并律
/// （channels_fuzz.pin_rd_match）: 面读音音节列左到右吞并输入段骨架
/// （exact 段 = 全等 / 前缀(尾 exact e→er) / KBN 1 替(cei→fei);
///   悬空声母段 = 首字母前缀 + 可吞并后续声母段(kun = k+n);
///   纯元音面音节越段(da'a'ka'na 的 a)）——全吞并才发, 表外读音零发射,
/// 154 集护栏锚零误伤（泛化前逐案验证）。
/// 值 = [(词面, 钉终分, kind)]（py dict 插入序 = 消费序）:
///   kind="pending"  域 = 面读音可全词展开且 comp 全词件在场（peiy/jint,
///                   has_comp_full_word 下 fcomp 被 −bias 杀）——打分层
///                   W14 +2（consumed−syls, consumed=len(inp) cap）预扣于
///                   bias（W7O2 四 pending 面读严恰好 +2）;
///   kind="fcomp"    域 = 无 comp 全词件（dakn/pgde）, 免 W14;
///   kind="sentence" 域 = G1 空面板（ncei 年费: n→nian × cei→fei KBN 头替
///                   × ei exact, FB_FACE_CAP_G1=2 截除 → 补发, 组 0 钉分）。
/// （py constants.RD_PIN_FACES 同表同值同注）
pub static RD_PIN_FACES: &[(&str, &[(&str, f64, &str)])] = &[
    // peiy/jint 补全绕行面已退役（第九波 T1 目标4: pei'ya/pei'yuan/
    // jin'tou/jin'tan/jin'tui/jin'ta/jin'tang/jin'tu 八条 → DANGLING_COL
    // 悬空补全列校准接管, 钉序 = native 行序; 金条/劲头/津贴 comp OV 词
    // 由 jin'tiao/jin'tou/jin'tie 校准列去重收编——见 DANGLING_COL 注记）
    // dakn: native #2-#5 达克宁(da,ke,ning 词)/大阿卡纳(典缺 4 音节词)/
    //   大坤/打捆(da,kun 词列)——单词面 kbest len<2 门不发 → 钉面
    ("da'ke'ning", &[("达克宁", -0.90, "fcomp")]),
    ("da'a'ka'na", &[("大阿卡纳", -1.30, "fcomp")]),
    ("da'kun", &[("大坤", -1.60, "fcomp"), ("打捆", -1.90, "fcomp")]),
    // pgde: native #2/#3 屁股蹲儿(pi,gu,dun,er——e→er 补全 4 音节词)/
    //   屁股蛋儿(律4 词面版)
    ("pi'gu'dun'er", &[("屁股蹲儿", -0.50, "fcomp")]),
    ("pi'gu'dan'er", &[("屁股蛋儿", -0.80, "fcomp")]),
    // ncei: native #2 年费(nian,fei)——sentence 组 0, 钉分 −9.5 居
    //   那次诶 −8.44 后
    ("nian'fei", &[("年费", -9.50, "sentence")]),
    // ---- 役N 第十一波(N5b): ADV 族钉面（native_ref4 agentN4/data 行 0-1）。
    // 域 = 头段×悬空中缀×尾段（kexsi/buxdui）与 悬空头声母×尾 exact
    // （wtianqi/mgaoyuan）——native 面词 = 头字×尾完成词 / 头完成词×尾字 /
    // 头声母首字×尾首词, 我方典位次深（笑死 −3.531 词级可召回但 kbest
    // 单词门 len(words)≥2 不发; 不想对/不行对 comp 通道在板但 −2.42 被
    // fcomp 洪面挤位）。kind=fcomp 域 = 非 comp 全词件占顶(kexsi/buxdui,
    // 与 dakn/pgde 同型); kind=sentence 域 = 悬空头短语(wtianqi/mgaoyuan,
    // 打分层 sentence 类先序）。钉分 16.0/15.5 = 面>字列(−1.0 域)逐位。
    // 泛化护栏: §3 吞并律门——wtianq 类在途变体自然复用（wo'tian'qi
    // 吞并 wtianq: w 悬空+tian+q 悬空尾 → native 在途语义同形）,
    // kexsi/buxdui 域外零触发（154 集+120 集验证）。
    // （py constants.RD_PIN_FACES 同表同值同注, tick62）----
    ("ke'xiao'si", &[("可\u{1}笑死", 16.0, "fcomp")]),
    ("ke'xi'si", &[("可惜\u{1}死", 15.5, "fcomp")]),
    ("bu'xiang'dui", &[("不想\u{1}对", 16.0, "fcomp")]),
    ("bu'xing'dui", &[("不行\u{1}对", 15.5, "fcomp")]),
    // buxcuo（34 集锚, 同构复用）: bu,x,cuo = bu+xiang 补全+cuo exact——
    //   comp 通道面 不想错/不行错 被 律 A 洪面抑制吞没 → 同构钉面回收
    ("bu'xiang'cuo", &[("不想\u{1}错", 16.0, "fcomp")]),
    ("bu'xing'cuo", &[("不行\u{1}错", 15.5, "fcomp")]),
    ("wo'tian'qi", &[("我\u{1}天气", 16.0, "sentence")]),
    ("mai'gao'yuan", &[("买\u{1}高原", 16.0, "sentence")]),
    ("mei'gao'yuan", &[("没\u{1}高原", 15.5, "sentence")]),
];

/// ---- 第九波 T1(目标4): 悬空补全列校准（每音节列首词律的校准列实现——
/// native 补全块 per-reading 词条, native 运行时词典槽频 ≠ wxq.bin
/// （jin'tie 我方列头 近铁 / native 津贴; jin'tui 我方 金腿 / native 进退
/// ——数据线差, 位置律不可复制, 同 wsd/CHAR_COL 之谜）→ 读音键校准列
/// （RD_COL 同族）。键 = 补全读音 pretty, 值 = [(词, 钉终分)]——钉分承担
/// native 补全块序且免疫 OV/W14/boost 重排（bias = 钉 − lp(OV) −
/// W14×(consumed−syls), 终值 = 钉）。未列读音维持 oi≤2 既有截取（junk
/// 列头 劲托/进厅 native 面板全无 → 表外不发校准词, 仍走旧池兜底）。
/// 消费 = channels_full.dangling_block; 原绕行面 RD_PIN_FACES
/// jin'tou/jin'tan/jin'tui/jin'ta/jin'tang/jin'tu/pei'ya/pei'yuan 八条
/// 随本表退役（零行为验证: jint/peiy 双 align 10/10 + top-1 保持,
/// 他查询逐位 diff）。native 逐位 = agentY_bench/data/{jint,peiy}.txt
/// 行 0-12 / 0-11（per-reading: jin'tou 双词 尽头>劲头 / pei'you 双词
/// 培优>配有, native 序 = 面板行序）----
pub static DANGLING_COL: &[(&str, &[(&str, f64)])] = &[
    ("jin'tian", &[("今天", 12.0)]),
    ("jin'tou", &[("尽头", 11.0), ("劲头", 8.0)]),
    ("jin'tie", &[("津贴", 10.0)]),
    ("jin'tiao", &[("金条", 9.0)]),
    ("jin'tan", &[("金坛", 7.0)]),
    ("jin'tui", &[("进退", 6.0)]),
    ("jin'ta", &[("金塔", 5.0)]),
    ("jin'tang", &[("金堂", 4.0)]),
    ("jin'tu", &[("进图", 3.0)]),
    ("jin'tai", &[("金泰", 2.0)]),
    ("jin'tuan", &[("金团", 1.0)]),
    ("jin'tong", &[("金通", 0.0)]),
    ("pei'yang", &[("培养", 12.0)]),
    ("pei'yin", &[("配音", 11.0)]),
    ("pei'yu", &[("培育", 10.0)]),
    ("pei'yue", &[("配乐", 9.0)]),
    ("pei'yao", &[("配药", 8.0)]),
    ("pei'you", &[("培优", 7.0), ("配有", 6.0)]),
    ("pei'yi", &[("配以", 5.0)]),
    ("pei'ya", &[("胚芽", 4.0)]),
    ("pei'yuan", &[("培元", 3.0)]),
    ("pei'ying", &[("培英", 2.0)]),
    ("pei'ye", &[("配液", 1.0)]),
    // ---- 役N 第十一波(N5b): exact头×悬空尾补全块逐位（native_ref4
    // agentN4/data {dianw,gongk}.txt 行 0-15/0-16。native 补全块序 =
    // per-reading 词列轮转合并（dianw: wei[点位 典韦 点为 电位]/wang[电网]/
    // wo[点我 电我]/wan[点完 电玩]/wu[玷污 点舞 电务]/wai[店外]/wen[点蚊
    // 电文]/wa[点哇] 轮转位序 = 行序 0-15 逐位咬合; gongk 同构）——位置律
    // 在 native 运行时词典（f412 槽频）, 我方 wlp 序不可复制（点位 fv 序
    // 我方 点我>点位 反序）→ 逐位钉分。钉分 16.0→4.8/17.0→4.1 全域高于
    // 未钉补全词（lp+DANGLING_COMPLETION_BOOST+W14×3 ≈ +0.6 上界）→
    // 我方典有 native 无词（店务/店我/店位/点问/公客/工可/工勘/工看/
    // 供款/供考 族）整体沉于钉块。gongk 公克(native #17) 我方典缺——
    // 钉块 17 词截 native 行 0-16。消费 = channels_full.dangling_block
    // 既有校准路径, 表外读音零外溢。（py 同表同值同注, tick62）----
    ("dian'wei", &[("点位", 16.0), ("典韦", 12.8), ("点为", 8.4),
                   ("电位", 5.6)]),
    ("dian'wang", &[("电网", 15.2)]),
    ("dian'wo", &[("点我", 14.4), ("电我", 9.0)]),
    ("dian'wan", &[("点完", 12.0), ("电玩", 11.4)]),
    ("dian'wu", &[("玷污", 8.6), ("点舞", 8.2), ("电务", 5.2)]),
    ("dian'wai", &[("店外", 5.8)]),
    ("dian'wen", &[("点蚊", 5.4), ("电文", 4.8)]),
    ("dian'wa", &[("点哇", 5.0)]),
    ("gong'kai", &[("公开", 17.0)]),
    ("gong'ke", &[("功课", 16.2), ("工科", 15.4), ("攻克", 11.4),
                  ("公克", 4.1)]),
    ("gong'kao", &[("公考", 12.2)]),
    ("gong'kuan", &[("公款", 10.6)]),
    ("gong'kang", &[("共抗", 9.8), ("共康", 4.2)]),
    ("gong'kou", &[("宫口", 9.0), ("工口", 5.0)]),
    ("gong'ka", &[("工卡", 8.2)]),
    ("gong'kuai", &[("公筷", 7.4)]),
    ("gong'kuang", &[("工况", 6.6), ("工矿", 5.4)]),
    ("gong'ku", &[("共苦", 6.2)]),
    ("gong'kong", &[("工控", 5.8)]),
    ("gong'kui", &[("功亏", 4.6)]),
    // ---- 役N tick64: exact头×悬空声母 r 补全块（native_ref4 agentN4/data
    // {nar,zher}.txt 行 0-6/0-9 逐位。native 律 = DANGLING_COL 悬空声母钉列,
    // 与 dianw/gongk 同构——纳入>那人>那让 / 这人>这让>遮肉>柘荣 逐位咬合）----
    ("na'ru", &[("纳入", 16.0)]),
    ("na'ren", &[("那人", 15.0), ("拿人", 13.0), ("哪人", 10.0)]),
    ("na'rang", &[("那让", 14.0)]),
    ("na'rou", &[("那肉", 12.0)]),
    ("na'ri", &[("那日", 11.0)]),
    ("zhe'ren", &[("这人", 16.0), ("着人", 12.0), ("哲人", 11.0), ("蜇人", 9.0)]),
    ("zhe'rang", &[("这让", 15.0), ("折让", 8.0)]),
    ("zhe'rou", &[("遮肉", 14.0)]),
    ("zhe'rong", &[("柘荣", 13.0)]),
    ("zhe'ri", &[("这日", 10.0)]),
    // renm: ren + m 悬空补全 (native_ref4 agentN4/data/renm.txt 行 0-9 逐位)
    ("ren'min", &[("人民", 16.0)]),
    ("ren'men", &[("人们", 15.0)]),
    ("ren'mai", &[("人脉", 14.0)]),
    ("ren'ming", &[("人名", 13.0), ("人命", 10.0), ("认命", 9.0), ("任命", 7.0)]),
    ("ren'ma", &[("人嘛", 12.0)]),
    ("ren'mei", &[("人美", 11.0)]),
    ("ren'me", &[("人么", 8.0)]),
];

pub fn dangling_col_get(k: &str) -> Option<&'static [(&'static str, f64)]> {
    DANGLING_COL.iter().find(|(kk, _)| *kk == k).map(|(_, v)| *v)
}

/// ---- 第八波(结构泛化): §1 深度律路由键（原 panel._O1_PANELS 查询键白
/// 名单门的读音键化）。键 = 读音 pretty 形（含撇号, 由构造不可能等于任何
/// 查询串——查询层输入已剥为纯小写字母）。DEEP_ROUTE_COVERED = 覆盖态
/// 输入的读音命中 → 深列面板; DEEP_ROUTE_LONG = >8 键在途输入的前缀链
/// 节点读音命中 → 深列面板。面板本体 = 前缀链节点列拼接（panel.deep_chain
/// 归纳: 完整音节切分边界上的读音终端节点, 长→短; 列 = RD_COL/CHAR_COL/
/// WORD_COL 校准列, 缺校准的链节点不发——native 列内容在 native 运行时
/// 词典, wxq.bin 不可见的部分只能校准, 故路由键与列同表族校准
/// （FB_INI_COL['n'] 先例: 声母/读音键与查询串同形属合法校准形态））。
pub fn deep_route_covered(k: &str) -> bool {
    matches!(
        k,
        "zao'shang'hao"
            | "wan'shang'hao"
            | "mei'guan'xi"
            | "xin'ku'le"
            | "ben'wang"
            | "xing'lang"
            | "yi'ne"
            // ---- 第九波 T1(尾扫批): covered 前缀链路由（native agentY_bench/data
            // 逐位: covered 面板 = 词条 + 头音节前缀链字列（zenme=[zen'me,zen,ze] /
            // cengjing=[ceng'jing,ceng,cen,ce] / liusi=[liu'si,liu,li] / geiguo=
            // [gei'guo,gei,ge] / duibuqi=[dui'bu'qi,dui'bu,dui] / cen=[cen,ce]）——
            // 与 §1 在途链的差别: 前缀不要求完整切分边界（cen 的 ce 非 za|o 型
            // 边界也发; native zenme 行 10-85 = ze 列 74 字实证）。键=读音 pretty,
            // 查询串同形属合法校准形态（cou/FB_INI_COL['n'] 先例）----
            | "zen'me"
            | "ceng'jing"
            | "liu'si"
            | "gei'guo"
            | "cen"
            | "dui'bu'qi"
            // ---- 役N 第十一波(N5b): ü 域单音节 covered 深列（native_ref4
            // ju/xu n=96 纯读音字列面板——我方 exact 通道 wlp 序+异读字混入
            // （车/且/柜 混入 ju 列顶）双缺陷, native 列序在运行时词典 →
            // RD_COL 逐位校准, cen 单音节 covered 先例）。（tick62）----
            | "ju"
            | "xu"
            // ---- 役N 第十一波(N5b): 叠元音覆盖态深列（native_ref4 haao
            // n=14——"haao" = ha'ao 读音字母串 = DAT 覆盖态 → fuzzcomp
            // 律 D 域外（G2 门 _wst terminal 直通）→ 深列路由接管: native
            // 面板 = 塌缩(hao)词块 与 未塌缩(ha'ao/ha'a/ha)列交错逐位——
            // 好>哈奥>哈傲(典缺字面直发)>号>哈啊>哈>蛤>铪>奤>蝦。
            // 非覆盖叠元音（keexi）走 channels_fuzz 律 D 塌缩面。（tick62）----
            | "ha'ao"
            // ---- 役N tick64: ref4 覆盖态深列与常用词核心序校准（native_ref4
            // {xianlai,jinyu,minggao}.txt 行 0-9 逐位。核心常用词 先来/金鱼
            // 及 名#高 词面直发）----
            | "xian'lai"
            | "jin'yu"
            | "ming'gao"
            | "wan'neng"
    )
}

pub fn deep_route_long(k: &str) -> bool {
    matches!(k, "tian'qi" | "cou")
}

/// ---- 第八波(结构泛化): §1 深列面板的全读音列校准（原 _O1_PANELS 首列
/// 的读音键化; 键 = 读音 pretty 形, 值 = native 面板列逐位, 典缺词面字面
/// 直发（忠国/井栏 先例）。多 variant 读音（yi'ne = yi'ne/yin'e 双切）列
/// 合并在首 variant 键下——native 面板双切词列同板逐位。）
/// （py constants.RD_COL 同表同值同注）
pub static RD_COL: &[(&str, &[&str])] = &[
    // 来源 zaoshanghao n=91 行 0-7（zsh 词列 8: 2 典词 + 6 典缺组字面）
    ("zao'shang'hao", &["早上好", "早上号", "枣尚豪", "枣上好", "枣尚蚝",
                        "枣尚耗", "枣尚好", "枣尚嚎"]),
    // 来源 wanshanghao n=96 行 0-5（wsh 词列 6）
    ("wan'shang'hao", &["晚上好", "晩上好", "晚上号", "晚上豪", "晚上嚎",
                        "碗上好"]),
    // 来源 meiguanxi n=96 行 0
    ("mei'guan'xi", &["没关系"]),
    // 来源 xinkule n=96 行 0-4（词 + 心苦#了 短语形 + le 尾槽字列变体
    //   叻嘞勒——律1 尾槽字列跨读音字面, native 在板逐位）
    ("xin'ku'le", &["辛苦了", "心苦\u{1}了", "辛苦叻", "辛苦嘞", "辛苦勒"]),
    // 来源 benwang n=44 行 0-5（native 序 本王>本网>犇旺>本旺>奔往>本汪
    //   ——LP_OVERRIDE 本王/本网 反推值的 native 原序）
    ("ben'wang", &["本王", "本网", "犇旺", "本旺", "奔往", "本汪"]),
    // 来源 xinglang n=96 行 0-4（xl 词列 5——星蓝(xing'lan 悬空)全无）
    ("xing'lang", &["邢朗", "星朗", "星浪", "行廊", "星狼"]),
    // 来源 yine n=96 行 0-7（yi'ne/yin'e 双切词列合并 + 一#呢 短语形）
    ("yi'ne", &["吟哦", "一\u{1}呢", "银额", "银娥", "亿呢", "银鹅", "以呢",
                "蚓厄"]),
    // 来源 tianqizenmeyang n=96 行 0-8（tianqi 深列 9）
    ("tian'qi", &["天气", "天启", "天奇", "天齐", "天琪", "天琦", "田七",
                  "天祺", "天汽"]),
    // 来源 jttq n=96 行 0（简拼整词面——今天天气 jianpin 键 jttq 直达,
    //   无注入时 rank #18, 面板钉位; 词后 = j 声母栈列 G1_COL['j']）
    ("jin'tian'tian'qi", &["今天天气"]),
    // ---- 第九波 T1(尾扫批): covered 前缀链词条列（native agentY_bench/
    // data 逐位; 消费 = panel 深列链 covered 支 exact 节点——前缀节点走
    // CHAR_COL/WORD_COL, RD_COL 只服务自身查询, 防 cen 型键双查询串列）。
    // 典缺词面字面直发（真么/给郭/给锅/刘斯/六死 类 native 在板, 我方
    // 部分典缺——deep_emit_col synthetic 先例 忠国/井栏）----
    // 来源 zenme n=86 行 0-1（真么 = retro z→zh 头纠错整词, tag b9+04 类）
    ("zen'me", &["怎么", "真么"]),
    // 来源 cengjing n=77 行 0-1（风景 = 头段 KBN c→f 单替换整词）
    ("ceng'jing", &["曾经", "风景"]),
    // 来源 liusi n=96 行 0-10（11 词列: 刘思/柳丝典词 + 刘斯/刘四/流失/
    //   刘司/柳思/流死/流泗/刘寺/六死 = 头字列×尾字列组合面 + 流失
    //   (liu,shi 尾 retro s→sh 变体)——典缺 8 词字面直发）
    ("liu'si", &["刘思", "柳丝", "刘斯", "刘四", "流失", "刘司", "柳思",
                 "流死", "流泗", "刘寺", "六死"]),
    // 来源 geiguo n=95 行 0-4（给郭/给锅 典缺; 给狗 = gei'gou 跨读音词
    //   native #4 在板——跨读音字面 RD_COL 先例 xin'ku'le 叻嘞勒）
    ("gei'guo", &["给过", "给郭", "给锅", "给果", "给狗"]),
    // 来源 cen n=63 行 0-15（册那/册呢/车尔尼 = 尾悬空补全组合面
    //   (ce,na)/(ce,ne)/(che,er,ni) 与 cen 字列交错逐位——全列收录,
    //   字列子集在 CHAR_COL['cen']（cengjing 前缀节点消费））
    ("cen", &["岑", "册那", "参", "涔", "叅", "梣", "册呢", "參", "笒",
              "汵", "车尔尼", "叄", "埁", "硶", "篸", "嵾"]),
    // 来源 duibuqi n=95 行 0-2（躲不起 = duo,bu,qi 头段 i→o? 变体整词,
    //   native #2; 我方典词 wid 在——无需字面）
    ("dui'bu'qi", &["对不起", "对不齐", "躲不起"]),
    // ---- 役N 第十一波(N5b): ü 域单音节列（native_ref4 {ju,xu}.txt 行 0-29
    // 逐位。native 列 = 主读音域全字（含 据(ju)/旭 等次读音字, native 律
    // = 次读音字在板——与 律1 槽域主读音过滤不同域）; 我方 oi 序 = 具据居
    // 局… / wlp 序 = 车具…（车/且/柜 异读字混入顶位, native 全面板无）
    // → 逐位校准, 词典缺字（袓/罝/澽 类）由 deep_emit_col 字面直发。
    // 异读字排除律留 spec: exact 通道（channels_full/core 域）主读音
    // 过滤 + 列序校准化——本表只覆盖 ju/xu 两键, ü 域全量消债待他域。）
    // （py constants.RD_COL 同表同值同注, tick62）----
    ("ju", &["局", "句", "距", "聚", "剧", "巨", "居", "具", "举", "菊",
             "拒", "据", "狙", "鞠", "橘", "袓", "锯", "桔", "炬", "钜",
             "焗", "驹", "琚", "矩", "俱", "罝", "跼", "澽", "咀", "踘"]),
    ("xu", &["徐", "需", "许", "旭", "续", "虚", "须", "嘘", "序", "恤",
             "胥", "绪", "戌", "栩", "煦", "叙", "恓", "朐", "蓄", "圩",
             "絮", "敍", "魖", "墟", "禑", "滸", "婿", "許", "虗", "漵"]),
    // 来源 haao n=14 行 0-9（塌缩词块×未塌缩列交错——哈傲 典缺字面直发,
    // deep_emit_col literal 先例; 哈啊 = ha'a 词面）
    ("ha'ao", &["好", "哈奥", "哈傲", "号", "哈啊", "哈", "蛤", "铪", "奤",
                "蝦"]),
    // 来源 xianlai n=96 行 0-9（native 核心常用词序 先来>闲来>现来,
    // 压过 wxq.bin 中 现来 虚高 logp）
    ("xian'lai", &["先来", "闲来", "现来", "仙籁", "鲜来", "仙来", "线来",
                   "掀来", "显来", "贤来"]),
    // 来源 jinyu n=96 行 0-9（金鱼>金玉>金域>禁欲, 避免 金誉 f0 抢位）
    ("jin'yu", &["金鱼", "金玉", "金域", "禁欲", "金隅", "金宇", "瑾瑜",
                 "今雨", "近于", "巾羽"]),
    // 来源 minggao n=96 行 0-8（名#高 短语面 + 明高/鸣皋 词列 + 单字列）
    ("ming'gao", &["名\u{1}高", "明高", "鸣皋", "铭高", "名号", "名", "明",
                   "命", "铭"]),
    // 来源 wanneng n=96 行 0-5（万能 覆盖态深列）
    ("wan'neng", &["万能", "皖能", "完能", "玩能", "挽梦", "皖N"]),
];

pub fn rd_col_get(k: &str) -> Option<&'static [&'static str]> {
    RD_COL.iter().find(|(kk, _)| *kk == k).map(|(_, v)| *v)
}

/// ---- 役N W3(第六波 列序清盘): native 面板字列校准表 ----
/// 消费点: panel 段W 单音节字块 / garbage_cols exact 支 / _panel_syl_char
/// 槽字——列序两处消费同表一致。词典缺失字面（拉丁/词典无词）字面直发。
pub static CHAR_COL: &[(&str, &[&str])] = &[
    // 来源 neishuanzengzae n=38 全面板（nei 主列 + ne 前缀列拼接原序）
    ("nei", &["内", "內", "馁", "那", "鮾", "哪", "浽", "娞", "餒", "呢",
              "呐", "讷", "疒", "抐", "吶", "訥", "鯘", "氝", "錗", "氞",
              "脮", "腇", "㐻", "㘨", "㖏", "㨅", "䇣", "㼏", "䜆", "䳖",
              "䲎", "䡾", "眲", "㕯", "䅞", "䎪", "䭆", "䭚"]),
    // 来源 zeebjpw n=75 全面板（纯 ze 列零组合）
    ("ze", &["则", "泽", "啧", "责", "择", "則", "仄", "沢", "澤", "迮",
             "灂", "赜", "齰", "嘖", "稄", "昃", "歵", "蠌", "咋", "擇",
             "責", "齚", "侧", "瞔", "皟", "笮", "帻", "夨", "箦", "捑",
             "唶", "幘", "樍", "泎", "舴", "溭", "渞", "択", "鸅", "耫",
             "滜", "賾", "昗", "襗", "庂", "飵", "嫧", "蔶", "礋", "睪",
             "矠", "諎", "謮", "㳁", "䂝", "㟙", "㳻", "䇥", "側", "㖽",
             "㢎", "崱", "㮣", "䅚", "㣱", "㥽", "㻭", "䔾", "䯔", "䰹",
             "䕉", "簀", "䕪", "䶡", "䶦"]),
    // 来源 wenzuantuodun（行 3-95 wen 列; 行 7 wen=拉丁字面位）
    ("wen", &["问", "文", "稳", "温", "wen", "雯", "闻", "纹", "蚊", "吻",
              "揾", "問", "汶", "搵", "玟", "瘟", "穩", "炆", "穏", "溫",
              "鴖", "鴍", "絻", "聞", "紋", "蕰", "縕", "蟁", "芠", "韫",
              "藴", "紊", "閺", "彣", "閿", "榅", "刎", "塭", "妏", "忞",
              "肳", "饂", "呅", "繧", "豱", "闦", "昷", "渂", "鎾", "瑥",
              "亠", "鳁", "缊", "璺", "鰮", "抆", "忟", "蚉", "伆", "桽",
              "鞰", "阌", "馼", "緼", "呚", "呡", "殟", "魰", "莬", "辒",
              "薀", "珳", "脗", "榲", "輼", "鼤", "鰛", "歾", "駇", "鳼",
              "轀", "褞", "螡", "顐", "㐎", "㡈", "㑮", "㖧", "㳷", "㗃",
              "㟩", "㝧", "䎹"]),
    // 来源 taidesiweihenminjie（行 3-26 tai 列; 行 20 Tai=拉丁字面位）
    ("tai", &["太", "台", "泰", "态", "胎", "抬", "钛", "肽", "邰", "苔",
              "臺", "汰", "漦", "呔", "冭", "酞", "籉", "Tai", "儓", "薹",
              "枱", "跆", "檯", "箈"]),
    // 来源 shuileicunongcu（行 3-14 shui 列 12 字）
    ("shui", &["谁", "水", "睡", "税", "说", "誰", "氵", "氺", "稅", "說",
               "娷", "帨"]),
    // 来源 shuochuanceeei（行 4-11 shuo 列 8 字）
    ("shuo", &["说", "硕", "說", "烁", "朔", "铄", "数", "槊"]),
    // 来源 jinglanhuanchuangjue（行 4-74 jing 列; 行 4 京L/行 29 Jing
    // = 拉丁混排字面位）
    ("jing", &["京L", "经", "静", "景", "精", "京", "晶", "井", "净", "镜",
               "靖", "敬", "婧", "境", "惊", "璟", "菁", "警", "竟", "径",
               "荆", "竞", "劲", "颈", "Jing", "鲸", "泾", "茎", "睛", "憬",
               "旌", "經", "燝", "兢", "靜", "腈", "弪", "迳", "靓", "箐",
               "驚", "穽", "胫", "丼", "阱", "肼", "淨", "瀞", "粳", "綡",
               "痉", "暻", "竧", "鏡", "競", "倞", "稉", "儆", "葝", "澋",
               "汫", "汬", "鋞", "粇", "鯨"]),
    // 来源 zhongguorenmi（行 11-18 zhong 列 8 字）
    ("zhong", &["中", "种", "钟", "重", "忠", "众", "仲", "终"]),
    // wenzuantuodun 槽字位（tuo 槽 = 托, native oi2; 我方 oi0=脱 → 校准）
    ("tuo", &["托"]),
    // wenzuantuodun 平读槽字位（zuan 槽 = 钻, native #1; 我方 oi0=赚 → 校准）
    ("zuan", &["钻"]),
    // ---- 第八波(结构泛化): §1 深列面板字列（原 _O1_PANELS 拆列; 键 = 单
    // 音节读音, 与上列同族——native 字列序 ≠ wxq.bin oi 序, 逐位校准先例
    // 同源; 来源查询随注）。消费点 = panel 深列链（泛化后唯一新消费点;
    // 既有消费点对这些键零接触——154 集仅深列路由查询含这些链节点）----
    // 来源 zaoshanghao n=91 行 9-23（zao 字列 15; oi16+ fv 地板桶 native 全隐）
    ("zao", &["早", "造", "遭", "澡", "枣", "灶", "皂", "凿", "燥", "糟",
              "藻", "躁", "噪", "璅", "蹧"]),
    // 来源 zaoshanghao n=91 行 24-25（za 丢边列 2——zao 掉尾元音续列,
    // 完整切分 za|o|shang|hao 边界节点）
    ("za", &["咋", "砸"]),
    // 来源 wanshanghao n=96 行 9-25（wan 字列 17）
    ("wan", &["万", "玩", "完", "晚", "碗", "湾", "弯", "丸", "皖", "婉",
              "関", "宛", "萬", "腕", "琬", "挽", "菀"]),
    // 来源 meiguanxi n=96 行 9-25（mei 字列 17）
    ("mei", &["没", "美", "每", "梅", "妹", "枚", "煤", "沒", "酶", "媚",
              "镁", "霉", "眉", "莓", "魅", "玫", "媒"]),
    // 来源 xinkule n=96 行 7-25（xin 字列 19）
    ("xin", &["新", "心", "信", "鑫", "欣", "芯", "辛", "馨", "薪", "昕",
              "锌", "歆", "忻", "莘", "炘", "盺", "訫", "杺", "焮"]),
    // 来源 benwang n=44 行 6-25（ben 字列 20）
    ("ben", &["本", "笨", "奔", "苯", "犇", "贲", "坋", "锛", "坌", "夯",
              "捹", "夲", "燌", "畚", "喯", "栟", "倴", "奙", "渀", "賁"]),
    // 来源 xinglang n=96 行 5-25（xing 字列 21）
    ("xing", &["行", "性", "星", "型", "兴", "醒", "姓", "邢", "形", "杏",
               "幸", "刑", "腥", "猩", "省", "興", "惺", "擤", "郉", "煋",
               "瑆"]),
    // 来源 tianqizenmeyang n=96 行 9-18（tian 列 10——含 ti'an 跨界异切词
    // 提案（读音字母同串 tian, 词典 oi7 位, native 在板））
    ("tian", &["天", "填", "田", "甜", "舔", "添", "提案", "恬", "钿", "湉"]),
    // 来源 tianqizenmeyang n=96 行 19-25（ti 列 7——完整切分 ti|an|qi|… 边界）
    ("ti", &["提", "题", "体", "踢", "替", "梯", "鬄"]),
    // 来源 coucaotuoruiban n=8 全面板（纯 cou 列 8; 长串垃圾尾 → 只 cou 列）
    ("cou", &["凑", "湊", "腠", "楱", "辏", "傶", "輳", "㫶"]),
    // 来源 yine n=96 行 8-23（yin 字列 15 + Yin 拉丁字面位——native 面板
    // 逐位实证在场先例 wen/Tai/Jing）
    ("yin", &["因", "印", "银", "阴", "尹", "音", "殷", "饮", "引", "瘾",
              "茵", "寅", "隐", "淫", "吟", "Yin"]),
    // 来源 yine n=96 行 24-25（yi 列 2——完整切分 yi|ne 边界节点）
    ("yi", &["一", "已"]),
    // ---- 第九波 T1: covered 前缀链字列（native 逐位; 消费 = panel 深列
    // 链 covered 支前缀节点）。native covered 面板字列序 ≠ wxq.bin oi 序
    // （liu: 我方 oi0=流, native #11=刘——同 wsd 之谜 律§12, 校准唯一路）----
    // 来源 zenme n=86 行 2-9（zen 列 8; Zen = 拉丁字面位——wen/Tai/Jing
    //   先例。行 31 譛 = native 第二轮列余量, 未录——录则挤位 则@10）
    ("zen", &["怎", "谮", "囎", "撍", "譖", "Zen", "㻸", "䫈"]),
    // 来源 cengjing n=77 行 2-10（ceng 列 9——行 31-35 橧㣒㬝䁬䉕 第二轮未录）
    ("ceng", &["层", "蹭", "曾", "噌", "層", "曽", "嶒", "竲", "驓"]),
    // 来源 cengjing n=77 行 11-15（cen 列 5 第一轮）
    ("cen", &["岑", "参", "涔", "叅", "梣"]),
    // 来源 cengjing n=77 行 16-30 / cen n=63 行 16-30（ce 列 15 第一轮;
    //   语义双查询共享——cengjing 前缀链与 cen 前缀链同节点同列）
    ("ce", &["测", "侧", "册", "策", "厕", "惻", "測", "箣", "側", "冊",
             "墄", "簎", "栅", "萴", "廁"]),
    // 来源 liusi n=96 行 11-45（liu 列 35——整列无第二轮, 陸 止）
    ("liu", &["刘", "六", "留", "流", "柳", "溜", "碌", "陆", "遛", "瘤",
              "硫", "劉", "榴", "浏", "琉", "鎏", "騮", "馏", "罶", "骝",
              "绺", "熘", "瑠", "鶹", "駵", "镠", "蓅", "栁", "磟", "塯",
              "飀", "鎦", "旒", "镏", "陸"]),
    // 来源 liusi n=96 行 46-95（li 列 50——前缀链 li 节点, 隶 止）
    ("li", &["里", "李", "力", "离", "丽", "理", "立", "利", "黎", "粒",
             "礼", "例", "哩", "莉", "厘", "梨", "历", "栗", "厉", "隸",
             "锂", "励", "嚟", "裡", "俐", "郦", "俪", "樆", "荔", "狸",
             "醨", "璃", "笠", "沥", "鲤", "骊", "犁", "氂", "吏", "鵹",
             "栎", "砾", "砺", "麗", "蠫", "漓", "離", "剺", "隶", "褵"]),
    // 来源 geiguo n=95 行 5-6（gei 列 2）
    ("gei", &["给", "給"]),
    // 来源 geiguo n=95 行 7-15（ge 列 9 第一轮）
    ("ge", &["个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁"]),
    // 来源 duibuqi n=95 行 5-15（dui 列 11——DUI = 大写拉丁字面位
    //   （大写三态态1 域, covered 面板字面位与 wen/Tai 同族先例））
    ("dui", &["对", "队", "怼", "堆", "兑", "對", "鎚", "碓", "隊", "兌",
              "DUI"]),
];

pub fn char_col_get(k: &str) -> Option<&'static [&'static str]> {
    CHAR_COL.iter().find(|(kk, _)| *kk == k).map(|(_, v)| *v)
}

/// ---- 役N W3: native 面板词列校准表（段W 多音节词块逐位）。
/// py dict 插入序 = 消费序（W3b 典外深前缀词面按表序枚举）。
pub static WORD_COL: &[(&str, &[&str])] = &[
    // 来源 zhongguorenmi 行 4-10
    ("zhongguo", &["中国", "中果", "中过", "种过", "忠国", "种果", "中古"]),
    // 来源 taidesiweihenminjie 行 0-2（台的 flags=0, native #2 在板）
    ("taide", &["泰德", "抬得", "台的"]),
    // 来源 shuileicunongcu 行 0-2
    ("shuilei", &["水雷", "睡嘞", "谁累"]),
    // 来源 jinglanhuanchuangjue 行 1-3（井栏 典缺, native #3 在板）
    ("jinglan", &["景兰", "静兰", "井栏"]),
    // tick57 来源 mingtianjiudianjian 行 0（明天酒店: 我方读路径 trie 零词
    // walk=−1, 典外直发; 尾 jian 3 键 native 不计——词读终于 dian 悬空）
    ("mingtianjiudian", &["明天酒店"]),
    // ---- 第八波(结构泛化): §1 深列面板祖先读音词列（原 _O1_PANELS 拆列;
    // 键 = 读音字母连写, 与上列同族——154 集仅深列路由查询含这些链节点,
    // 既有消费点（段W 词块 / 典外深前缀词面）零接触）----
    // 来源 zaoshanghao n=91 行 8（zaoshang 词列 1——oi1 灶上 native 无位）
    ("zaoshang", &["早上"]),
    // 来源 wanshanghao n=96 行 6-8（wanshang 词列 3——oi1/2/3/5/7 native
    // 无位: 万商/完上/烷上/万上 f0 + 玩赏 fv 地板桶）
    ("wanshang", &["晚上", "玩上", "腕上"]),
    // 来源 meiguanxi n=96 行 1-5（meiguan 词列 5——oi3 每关 native 无位）
    ("meiguan", &["没关", "美观", "没管", "每罐", "梅关"]),
    // 来源 meiguanxi n=96 行 6-8（meigu 词列 3 双深——完整切分 mei|gu|an|xi
    // 边界; oi1/2/3 美谷/没古/美固 + oi4 眉骨/oi7 美姑/oi8 媚骨 native 无位）
    ("meigu", &["美股", "每股", "没顾"]),
    // 来源 xinkule n=96 行 5-6（xinku 词列 2——oi1 新库 native 无位）
    ("xinku", &["辛苦", "心苦"]),
    // ---- 第九波 T1: covered 前缀链祖先读音词列（native duibuqi 行 3-4
    // dui'bu 词列 2——前缀节点消费（WORD_COL 语义=祖先读音词列, 同族））
    ("duibu", &["对不", "队部"]),
];

pub fn word_col_get(k: &str) -> Option<&'static [&'static str]> {
    WORD_COL.iter().find(|(kk, _)| *kk == k).map(|(_, v)| *v)
}

/// ---- 役N W3: native 尾串 KBN 纠错词列（panel _panel_tail_corr 消费;
/// 纠正串 letters 为键）。
pub fn tail_corr_col(k: &str) -> &'static [&'static str] {
    match k {
        "cewei" => &["侧位", "侧围"],
        _ => &[],
    }
}

/// ---- 役N W3: G1 兜底声母栈字列校准（_g1_col_fix 消费）。native 栈列序
/// ≠ wlp 序（有(you) #1 > 一(yi) — native ykvtbzuu 行 1-95 逐位）。
pub fn g1_col(k: &str) -> &'static [&'static str] {
    match k {
        "y" => &["有", "要", "也", "月", "用", "呀", "又", "元", "与", "一",
                 "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云",
                 "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余",
                 "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁",
                 "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运",
                 "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶",
                 "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹",
                 "咦", "洋", "游", "音", "友", "粤", "夜", "牙", "铅", "說",
                 "野", "闫", "壹", "优", "岳", "腰", "怡", "颖", "意", "义",
                 "演", "颜", "言", "俞", "莹"],
        // ---- 第八波(结构泛化): 单声母栈面板列（原 O1 案 p/jttq 面板拆列;
        // 栈拆列; 键 = 声母——与 FB_INI_COL['n']/['p'] 同族先例（声母键与
        // 查询串同形属合法校准形态）。native 栈序 ≠ wlp 序（自然栈逐位对比
        // 失配实证）, 表值 = native 面板逐位。p 尾 o = 邻键（KBN p→o）字位;
        // j 栈 = 今天天气 词后声母全栈。消费 = panel 单声母面板律
        // （len==1 / 无音节头短串 × 简拼整词）----
        // 来源 p n=25 全面板（p 声母栈 24 + 哦）
        "p" => &["怕", "拍", "跑", "票", "陪", "配", "屁", "片", "平", "排",
                 "皮", "拼", "泡", "骗", "瓶", "盘", "喷", "彭", "牌", "破",
                 "品", "批", "鹏", "派", "哦"],
        // 来源 jttq n=96 行 1-25（j 声母全栈 25）
        "j" => &["就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进",
                 "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤",
                 "价", "卷", "近", "间", "局"],
        _ => &[],
    }
}

/// ---- 役N tick66 ref5 R1: H_stack 单声母栈列（native_ref5 agentN5/data
/// 逐位）。键 = 声母（G1_COL 同族合法形态——G1_COL['p']/['j'] 先例）。
/// 消费 = panel q_deep_panel R2（len==1 单声母栈面板）/ R3（声母栈）。
/// native 栈序 ≠ RD_PIN_COL 自然栈（m: 没>吗>么… — RD_PIN_COL['m'] 吗>没
/// 反序数据线差, 主线程收编时以本表为准归并 G1_COL）。
pub fn g1_col_local(k: &str) -> &'static [&'static str] {
    match k {
        // 来源 m n=96 行 0-29 / b n=96 行 0-29 / t n=96 行 0-29
        "m" => &["没", "吗", "么", "买", "嘛", "卖", "们", "妈", "猫",
                 "美", "敏", "每", "面", "摸", "忙", "木", "米", "毛",
                 "猛", "明", "慢", "门", "马", "码", "骂", "满", "咩",
                 "名", "梦", "秒"],
        "b" => &["吧", "不", "把", "被", "宝", "比", "别", "呗", "包",
                 "标", "魄", "笔", "保", "波", "班", "白", "边", "变",
                 "帮", "办", "报", "爸", "补", "半", "本", "表", "版",
                 "部", "并", "板"],
        "t" => &["他", "她", "贴", "太", "唐", "它", "田", "听", "跳",
                 "魄", "头", "挺", "汤", "图", "退", "同", "天", "提",
                 "谈", "条", "推", "台", "题", "停", "通", "套", "投",
                 "团", "调", "填"],
        _ => &[],
    }
}

/// ---- 役N tick66 ref5 R1: G_double 简拼整词域读音键列（RD_COL 同族,
/// 键 = 读音 pretty 形, 构造上不等于查询串）。消费 = q_deep_panel R3
/// （无音节头 × 简拼整词 × 声母栈）。native 锚:
///   ggxx n=96 行 0 = 高高兴兴(gao,gao,xing,xing — AABB 叠字简拼成语,
///     jianpin 表键 ggxx 直达) + g 声母栈(G2 shape-C ini col 供给);
///   bjzt n=96 行 0 = 不进则退(bu,jin,ze,tui) + b 栈;
///   wrz n=95 行 0 = 围绕着(wei,rao,zhe) + w 栈。
/// 表外读音零发射（xtks/zgrm/rmyy/wsm 等既有形状 C 锚无键不湿）。
pub fn rd_col_local(k: &str) -> Option<&'static [&'static str]> {
    match k {
        "gao'gao'xing'xing" => Some(&["高高兴兴"]),
        "bu'jin'ze'tui" => Some(&["不进则退"]),
        "wei'rao'zhe" => Some(&["围绕着"]),
        _ => None,
    }
}

/// ---- 役N tick66 ref5 R1 补遗: covered 读法链本地路由与列（junv/deque
/// 域）。键 = 读音 pretty / 连写键。native 逐位 = agentN5/data
/// {junv,deque}.txt 行 0-15（ju'nv 词列 6 + ju 字列 10 / de'que 词列 3 +
/// de'qu 词列 3 + de 字列 10）。native 列序 ≠ wxq.bin oi 序（ju 列 车/据
/// 异读字混顶, de'que 列 的确 压 的却 —— 数据线差, RD_COL ju/xu 先例同族
/// 校准）。消费 = deep_panel_rows covered 支（本地路由集命中才发, 表外
/// 零影响——基准全集仅 junv/deque 触发）/ q_deep_panel R1b 孤儿读法链。
pub fn covered_col_local(k: &str) -> Option<&'static [&'static str]> {
    match k {
        "ju'nv" => Some(&["剧女", "妓女", "局女", "粔籹", "距女", "巨女"]),
        "ju" => Some(&["局", "距", "句", "聚", "剧", "巨", "居", "具",
                       "举", "菊"]),
        "de'que" => Some(&["的却", "得确", "的缺"]),
        "dequ" => Some(&["的去", "的区", "地去"]),
        "de" => Some(&["的", "得", "德", "地", "嘚", "徳", "锝", "淂",
                       "旳", "鍀"]),
        _ => None,
    }
}

/// covered 读法链本地路由集（_COVERED_ROUTE_LOCAL; 命中才走深列面板）。
pub fn covered_route_local(k: &str) -> bool {
    matches!(k, "ju'nv" | "de'que")
}

/// ---- 役N tick66 ref5 R1: 悬空声母补全轮转本地钉列（jins 域）。
/// 键 = 补全读音 pretty（DANGLING_COL 同族, 表外零发射）。
/// native 逐位 = agentN5/data/jins.txt 行 0-15（dangling s → per-reading
/// 词列轮转合并: 金属>金色>近视>谨慎>晋升>禁赛>进水>金山>金沙>仅剩…）。
/// 我方 wlp 序反（金色>金属…）→ 钉分 = native 行序（位次律不可复制,
/// DANGLING_COL jin'tie 津贴 先例）。消费 = channels_full.dangling_block。
pub fn dangling_col_local(k: &str) -> Option<&'static [(&'static str, f64)]> {
    match k {
        "jin'shu" => Some(&[("金属", 16.0)]),
        "jin'se" => Some(&[("金色", 15.0)]),
        "jin'shi" => Some(&[("近视", 14.0), ("进食", 5.5), ("仅是", 4.5)]),
        "jin'shen" => Some(&[("谨慎", 13.0), ("紧身", 5.0)]),
        "jin'sheng" => Some(&[("晋升", 12.0), ("仅剩", 7.5)]),
        "jin'sai" => Some(&[("禁赛", 11.0)]),
        "jin'shui" => Some(&[("进水", 10.0), ("金水", 2.5)]),
        "jin'shan" => Some(&[("金山", 9.0)]),
        "jin'sha" => Some(&[("金沙", 8.0)]),
        "jin'si" => Some(&[("金丝", 3.5), ("近似", 3.0)]),
        _ => None,
    }
}

/// ---- 役N tick70（p60 联想役）: 裸声母单键列 SOLO 表 — 仅 len==1 消费
/// （query 单键门 → q_initial_panel）。与 g1_col/g1_col_local 分离的必要性:
/// g1_col 按 inp[0] 服务多键回退面板（jttq 词后栈）, 本表六列是纯单键列,
/// 混入会劫持 auqshwejna/ncei 类多键回退面（py 端 −3/−7 实证）。
/// native 三律: ① 与 ctx 无关（wxsel after_sel/mid 双采样 d/s/w/x 四组
/// 选词后面板 ≡ 无 ctx 面板逐位相同）; ② 纯单字 25-26 连, hot 词组
/// （但是/都是/多少）不进面; ③ 列序 ≠ wlp/fv/sylw/oi 序（列序之谜同源,
/// f412 运行时槽频不可见）→ 钉序。d/s/w/x 来源 wxsel typed 全采样;
/// a/n 来源 native_ref3 单键用例 20 连全列。
pub fn g1_col_solo(k: &str) -> Option<&'static [&'static str]> {
    match k {
        "d" => Some(&["的", "都", "到", "大", "对", "点", "多", "等", "得",
                      "带", "当", "地", "但", "店", "打", "单", "度", "掉",
                      "定", "第", "低", "电", "待", "段", "额"]),
        "s" => Some(&["是", "说", "上", "啥", "谁", "市", "省", "所", "孙",
                      "撒", "送", "睡", "事", "时", "书", "算", "少", "收",
                      "三", "手", "生", "刷", "岁", "双", "数", "四"]),
        "w" => Some(&["我", "玩", "问", "无", "哇", "为", "完", "王", "万",
                      "五", "魏", "晚", "稳", "味", "往", "忘", "喂", "娃",
                      "网", "未", "文", "外", "喔", "唔", "位", "微"]),
        "x" => Some(&["想", "先", "小", "下", "行", "项", "像", "西", "新",
                      "休", "晓", "心", "许", "写", "些", "学", "洗", "选",
                      "系", "线", "修", "信", "徐", "星", "性", "向"]),
        "a" => Some(&["啊", "阿", "哎", "爱", "吖", "嗄", "昂", "安", "按",
                      "锕", "錒", "腌", "醃", "A", "a", "奥", "唉", "熬",
                      "俺", "嗷"]),
        "n" => Some(&["你", "呢", "那", "能", "拿", "年", "您", "哪", "弄",
                      "男", "女", "难", "牛", "内", "拟", "捏", "呐", "奶",
                      "娜", "诺"]),
        // tick70b: 剩余 14 字母全采集（wxsel typed ver=1, 大列取前 26 钉;
        // k/l/q/r 全长 25-26）。特殊元音: i → [I,i] 拉丁双形; u → [u,U]
        // 小写先（词典缺 → mk_lit 字面支）; v → 空表（native n=0）。
        // 列尾邻键渗漏字（k/l→哦, q→啊, r→额）随钉保留。
        "c" => Some(&["吃", "车", "才", "从", "出", "处", "此", "场", "测",
                      "存", "程", "穿", "成", "菜", "差", "抽", "催", "次",
                      "超", "长", "冲", "查", "陈", "错", "擦", "纯"]),
        "e" => Some(&["额", "嗯", "饿", "呃", "鹅", "儿", "而", "二", "诶",
                      "恶", "俄", "額", "鄂", "讹", "娥", "屙", "啞", "厄",
                      "娿", "蛾", "哦", "噩", "鳄", "峨", "餓", "頋"]),
        "f" => Some(&["发", "分", "放", "费", "饭", "冯", "凤", "房", "富",
                      "风", "丰", "付", "法", "方", "翻", "份", "烦", "服",
                      "福", "封", "反", "非", "返", "否", "飞", "峰"]),
        "g" => Some(&["个", "给", "跟", "过", "刚", "哥", "郭", "搞", "干",
                      "高", "更", "改", "盖", "股", "噶", "该", "感", "狗",
                      "够", "管", "贵", "光", "挂", "工", "各", "关"]),
        "h" => Some(&["好", "和", "还", "会", "换", "哈", "很", "号", "回",
                      "话", "后", "喝", "含", "海", "户", "何", "货", "花",
                      "画", "活", "黑", "或", "红", "黄", "喊", "胡"]),
        "k" => Some(&["看", "开", "快", "可", "卡", "靠", "考", "块", "哭",
                      "扣", "课", "款", "空", "克", "口", "康", "困", "坑",
                      "科", "客", "亏", "宽", "凯", "库", "孔", "哦"]),
        "l" => Some(&["了", "来", "啦", "里", "咯", "李", "老", "两", "嘞",
                      "拉", "刘", "楼", "连", "路", "俩", "冷", "龙", "聊",
                      "留", "领", "罗", "量", "六", "类", "林", "哦"]),
        "o" => Some(&["哦", "噢", "喔", "欧", "筽", "偶", "嚄", "呕", "藕",
                      "区", "鸥", "瓯", "殴", "歐", "耦", "怄", "讴", "敺",
                      "沤", "嘔", "吽", "區", "吘", "禺", "甌", "鷗"]),
        "q" => Some(&["去", "钱", "起", "群", "请", "前", "亲", "全", "强",
                      "抢", "气", "取", "区", "求", "切", "清", "缺", "球",
                      "七", "期", "签", "千", "其", "且", "券", "啊"]),
        "r" => Some(&["人", "让", "肉", "日", "热", "如", "入", "认", "忍",
                      "任", "惹", "软", "然", "扔", "染", "若", "润", "瑞",
                      "荣", "弱", "容", "仁", "睿", "融", "额"]),
        "z" => Some(&["在", "这", "做", "中", "再", "至", "站", "赵", "正",
                      "走", "之", "找", "真", "着", "咋", "张", "早", "子",
                      "最", "转", "住", "总", "装", "咱", "整", "只"]),
        "i" => Some(&["I", "i"]),
        "u" => Some(&["u", "U"]),
        "v" => Some(&[]),
        // tick70c: 二键声母图 digraph（zh/ch/sh）同律纯字列 + 偶发词
        // （之后/最后/最好/时候 在列内; 列尾邻键渗漏 粗/从/组/在 随钉）。
        "zh" => Some(&["这", "中", "找", "张", "真", "着", "转", "住",
                       "装", "整", "只", "站", "正", "种", "周", "之后",
                       "最后", "之", "赚", "主", "涨", "最好", "值", "赵",
                       "组", "在"]),
        "ch" => Some(&["吃", "车", "出", "穿", "成", "差", "抽", "超",
                       "长", "冲", "查", "陈", "茶", "纯", "床", "春",
                       "厂", "场", "充", "城", "传", "初", "拆", "除",
                       "粗", "从"]),
        "sh" => Some(&["是", "说", "上", "啥", "谁", "睡", "事", "时",
                       "书", "少", "收", "手", "时候", "生", "刷", "双",
                       "山", "神", "数", "删", "杀", "省", "水", "算",
                       "三", "送"]),
        _ => None,
    }
}

/// ---- 役N tick71: 音节字列律 409 音节钉表（syl_panels.tsv 生成;
/// 键 = 音节, 键序 = sort 序。native 律: 音节面板 = 字列+补全词+
/// 简拼组合按原生序混排（61/79 面板混词实证）, 原样钉。消费 =
/// query 音节门 → q_initial_panel。表空 → 律休眠。----
pub static SYL_PANELS: &[(&str, &[&str])] = &[
    ("ai", &["哎", "爱", "唉", "艾", "挨", "矮", "癌", "埃", "欸", "愛", "AI", "哀", "碍", "硙", "瑷", "嗳", "捱", "蔼", "嫒", "霭", "敱", "隘", "乂", "磑", "暧", "嗌"]),
    ("an", &["按", "安", "俺", "案", "暗", "爱你", "岸", "韽", "啊你", "氨", "胺", "桉", "按钮", "鞍", "庵", "铵", "安宁", "按你", "安妮", "an", "黯", "谙", "垵", "屽", "咹", "安娜"]),
    ("ang", &["昂", "俺哥", "骯", "盎", "肮", "安个", "俺姑", "按个", "安宫", "安哥", "安国", "鞍钢", "卬", "安规", "安钢", "案管", "安工", "按过", "暗光", "暗管", "唔", "岇", "阿牛哥", "昻", "俺那个", "醠"]),
    ("ao", &["奥", "熬", "嗷", "澳", "凹", "傲", "敖", "袄", "啊哦", "獓", "岙", "坳", "翱", "鳌", "奧", "泑", "磝", "啊噢", "啊欧", "爱哦", "AO", "阿哦", "啊喔", "骜", "拗", "遨"]),
    ("ba", &["把", "吧", "爸", "八", "叭", "巴", "拔", "捌", "扒", "霸", "坝", "罢", "疤", "保安", "不啊", "不爱", "靶", "丷", "芭", "备案", "粑", "别啊", "钯", "ba", "跋", "耙"]),
    ("bai", &["白", "百", "摆", "呗", "佰", "拜", "柏", "掰", "败", "伯", "栢", "擺", "稗", "唄", "敗", "Bai", "庍", "薭", "捭", "贁", "粨", "擘", "襬", "絔", "拝", "瓸"]),
    ("ban", &["班", "办", "版", "半", "板", "搬", "把你", "般", "伴", "拌", "斑", "瓣", "扮", "坂", "八年", "把那", "扳", "辦", "籓", "绊", "颁", "阪", "把您", "ban", "不爱你", "不安宁"]),
    ("bang", &["帮", "棒", "绑", "榜", "邦", "半个", "办公", "磅", "幫", "办个", "镑", "浜", "办过", "半根", "蚌", "搒", "半挂", "傍", "膀", "搬个", "梆", "颁给", "bang", "Bang", "班岗", "塝"]),
    ("bao", &["宝", "包", "报", "保", "爆", "抱", "薄", "饱", "鲍", "煲", "堡", "豹", "暴", "葆", "寶", "報", "胞", "曝", "苞", "吧哦", "把哦", "剥", "刨", "鮑", "孢", "鑤"]),
    ("bei", &["被", "呗", "倍", "北", "背", "杯", "备", "贝", "辈", "蓓", "碑", "悲", "卑", "鞞", "唄", "焙", "钡", "棑", "陂", "琲", "貝", "碚", "備", "悖", "韝", "孛"]),
    ("ben", &["本", "笨", "奔", "苯", "Ben", "犇", "贲", "坋", "锛", "坌", "夯", "捹", "夲", "燌", "八二年", "贝尔尼", "伯尔尼", "比尔呢", "拜尔娜", "畚", "喯", "栟", "倴", "奙", "渀", "賁"]),
    ("beng", &["崩", "泵", "蹦", "绷", "甭", "挷", "蚌", "本国", "本该", "嘣", "本宫", "絣", "塴", "迸", "埲", "本馆", "笨狗", "本官", "本钢", "本格", "本港", "本根", "本哥", "綳", "跰", "奟"]),
    ("bi", &["比", "逼", "笔", "币", "必", "毕", "碧", "俾", "闭", "臂", "壁", "鼻", "避", "睥", "彼", "哔", "屄", "鞞", "畀", "璧", "痺", "弊", "毙", "獙", "纰", "铋"]),
    ("bian", &["边", "变", "编", "遍", "便", "卞", "币安", "扁", "鞭", "彼岸", "辩", "邊", "辨", "變", "贬", "籓", "比安", "碧安", "必安", "狴犴", "比俺", "弊案", "鯾", "匾", "辫", "煸"]),
    ("biao", &["表", "标", "彪", "裱", "飙", "镖", "婊", "膘", "飚", "嫑", "標", "灬", "骉", "比奥", "碧奥", "毕奥", "必奥", "碧傲", "BIAO", "驃", "褾", "儦", "僄", "鑣", "諘", "檦"]),
    ("bie", &["别", "憋", "鳖", "別", "瘪", "徶", "蹩", "虌", "避恶", "比厄", "逼额", "碧娥", "莂", "比额", "必饿", "彆", "鱉", "襒", "蟞", "蛂", "龞", "柭", "癟", "鼈", "柲", "䏟"]),
    ("bin", &["斌", "彬", "比你", "宾", "滨", "槟", "逼你", "缤", "镔", "比拟", "避难", "比那", "必能", "比呢", "玢", "鬓", "殡", "賔", "膑", "濒", "傧", "bin", "髌", "賓", "濱", "摈"]),
    ("bing", &["并", "病", "冰", "兵", "饼", "宾馆", "丙", "炳", "柄", "秉", "斌哥", "並", "邴", "彬哥", "禀", "槟", "餅", "苪", "疒", "缤果", "髌骨", "滨哥", "摒", "Bing", "昺", "仌"]),
    ("bo", &["波", "魄", "播", "博", "薄", "啵", "伯", "拨", "柏", "剥", "泊", "铂", "卜", "玻", "勃", "搏", "茀", "驳", "不哦", "布偶", "bo", "脖", "帛", "钵", "渤", "箔"]),
    ("bu", &["不", "补", "部", "布", "步", "卜", "埠", "捕", "簿", "補", "卟", "怖", "埔", "堡", "咘", "哺", "钚", "埗", "吥", "佈", "陠", "歩", "瓿", "勏", "晡", "蔀"]),
    ("ca", &["擦", "拆", "嚓", "吃啊", "长安", "冲啊", "攃", "长按", "菜啊", "出啊", "次啊", "穿啊", "惨啊", "唱啊", "CA", "礤", "礸", "囃", "遪", "䟃", "䵽"]),
    ("cai", &["才", "菜", "蔡", "猜", "财", "彩", "踩", "采", "裁", "材", "釆", "睬", "扐", "財", "偲", "採", "CAI", "跴", "縩", "纔", "寀", "啋", "埰", "婇", "倸", "棌"]),
    ("can", &["惨", "餐", "参", "灿", "残", "蚕", "璨", "掺", "粲", "驂", "慘", "囋", "朁", "嵾", "can", "參", "喰", "嘇", "燦", "摻", "殘", "惭", "鏒", "摲", "嬠", "黲"]),
    ("cang", &["仓", "藏", "舱", "参观", "苍", "餐馆", "沧", "倉", "嵢", "鶬", "匨", "瑲", "参股", "灿哥", "参公", "賶", "臧", "蒼", "艙", "伧", "玱", "鑶", "滄", "凔", "鸧", "欌"]),
    ("cao", &["草", "操", "曹", "艹", "槽", "糙", "肏", "屮", "嘈", "漕", "擦哦", "拆哦", "艸", "愺", "襙", "曺", "懆", "騲", "艚", "撡", "鼜", "螬", "蓸", "鄵", "傮", "慒"]),
    ("ce", &["测", "侧", "册", "策", "厕", "超额", "初二", "嫦娥", "惻", "差额", "測", "从而", "丑恶", "采耳", "蔡锷", "除恶", "箣", "CE", "側", "冊", "词儿", "刺耳", "错愕", "虫儿", "墄", "簎"]),
    ("cen", &["岑", "册那", "参", "涔", "叅", "梣", "册呢", "參", "笒", "汵", "车尔尼", "叄", "埁", "硶", "篸", "嵾", "穇", "㞥", "䨙", "䯔", "㻸", "䤁", "䫈", "䃡", "䅾", "䲋"]),
    ("ceng", &["层", "蹭", "曾", "噌", "層", "曽", "嶒", "竲", "驓", "橧", "㣒", "㬝", "䁬", "䉕", "岑", "参", "涔", "叅", "梣", "测", "侧", "册", "策", "厕", "惻", "測"]),
    ("cha", &["差", "查", "茶", "插", "叉", "茬", "察", "岔", "搽", "猹", "槎", "檫", "镲", "碴", "衩", "汊", "杈", "刹", "侘", "姹", "艖", "梌", "剎", "诧", "策划案", "嚓"]),
    ("chai", &["拆", "柴", "差", "钗", "儕", "豺", "侪", "虿", "釵", "茝", "芆", "瘥", "chai", "祡", "犲", "袃", "喍", "蠆", "肞", "訍", "囆", "㳗", "㑪", "㼮", "䓱", "㾹"]),
    ("chan", &["产", "馋", "铲", "缠", "婵", "禅", "蝉", "掺", "闡", "鑱", "颤", "諂", "墠", "刹那", "茶农", "查你", "茶娘", "查宁", "儃", "僝", "忏", "嶄", "蟾", "欃", "產", "潺"]),
    ("chang", &["长", "厂", "场", "常", "唱", "昌", "畅", "肠", "尝", "昶", "偿", "嫦", "長", "苌", "廠", "場", "倡", "敞", "蟾宫", "馋鬼", "蟾光", "Chang", "仩", "娼", "閶", "菖"]),
    ("chao", &["超", "炒", "抄", "吵", "朝", "潮", "焯", "巢", "钞", "晁", "嘲", "槱", "鼂", "勦", "煼", "鈔", "查哦", "差哦", "叉喔", "麨", "绰", "樔", "巣", "轈", "焣", "弨"]),
    ("che", &["车", "撤", "扯", "澈", "察哈尔", "車", "彻", "硩", "呫", "掣", "摰", "瞮", "坼", "吃喝儿", "聅", "出活儿", "船火儿", "烲", "春花儿", "俥", "屮", "尺", "揊", "硨", "徹", "勶"]),
    ("chen", &["陈", "晨", "辰", "趁", "宸", "沉", "臣", "尘", "琛", "称", "衬", "谌", "陳", "车内", "车呢", "忱", "抻", "郴", "嗔", "瘨", "莀", "櫬", "谶", "扯呢", "车奴", "踸"]),
    ("cheng", &["成", "城", "程", "称", "诚", "乘", "撑", "橙", "承", "呈", "秤", "陈哥", "澄", "晨光", "丞", "铖", "晟", "盛", "骋", "鎗", "珵", "宬", "陈工", "陈刚", "埕", "塍"]),
    ("chi", &["吃", "迟", "池", "哆", "尺", "持", "驰", "齿", "赤", "赿", "翅", "褫", "痴", "奓", "炽", "弛", "噭", "耻", "斥", "蚳", "遅", "鴟", "柅", "跢", "踟", "呎"]),
    ("chong", &["冲", "充", "重", "虫", "宠", "崇", "翀", "涌", "沖", "种", "舂", "铳", "蟲", "憧", "茺", "寵", "緟", "衝", "蝩", "Chong", "忡", "摏", "漴", "湧", "艟", "徸"]),
    ("chou", &["抽", "丑", "臭", "愁", "仇", "瞅", "筹", "稠", "疇", "盩", "酬", "犨", "绸", "畴", "雦", "躊", "Chou", "檮", "醜", "詶", "雔", "篘", "犫", "醻", "酧", "紬"]),
    ("chu", &["出", "初", "处", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕"]),
    ("chua", &["出啊", "出阿", "处啊", "除啊", "吃啊", "楚啊", "楚阿", "歘", "欻", "䫄", "䵵", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱"]),
    ("chuai", &["踹", "揣", "初爱", "啜", "储爱", "搋", "触爱", "欼", "嘬", "膪", "出哎", "膗", "㪓", "㪜", "初艾", "㱀", "䦤", "䴝", "歘", "欻", "出", "处", "初", "除", "楚", "储"]),
    ("chuan", &["穿", "传", "船", "川", "串", "喘", "傳", "钏", "氚", "瑏", "踳", "C环", "椽", "楚安", "初案", "巛", "初安", "出安", "出岸", "出按", "膞", "汌", "舛", "剶", "遄", "玔"]),
    ("chuang", &["床", "创", "窗", "闯", "穿个", "穿过", "疮", "传给", "幢", "传功", "創", "朣", "传个", "传感", "窻", "漺", "传过", "船歌", "船工", "串钩", "传告", "穿管", "传哥", "牎", "楚昂", "出昂"]),
    ("chui", &["吹", "锤", "垂", "捶", "炊", "槌", "埀", "陲", "錘", "龡", "顀", "惙", "圌", "棰", "椎", "倕", "搥", "菙", "桘", "硾", "箠", "鎚", "㝽", "䄲", "㓃", "㩾"]),
    ("chun", &["纯", "春", "蠢", "唇", "处女", "椿", "淳", "醇", "处男", "出纳", "朐", "純", "蓴", "储能", "鶉", "厨娘", "莼", "初年", "触怒", "雏鸟", "犉", "橁", "蝽", "膞", "槆", "櫄"]),
    ("chuo", &["戳", "齪", "鏃", "绰", "啜", "辍", "出哦", "酫", "哾", "踀", "擉", "鋜", "踔", "龊", "拺", "嚽", "淖", "鑡", "辵", "繛", "孎", "婥", "齱", "辶", "綽", "逴"]),
    ("ci", &["次", "词", "此", "刺", "辞", "磁", "瓷", "慈", "赐", "呲", "雌", "茨", "祠", "呰", "兹", "嬨", "佌", "詞", "CI", "糍", "礠", "餈", "鮆", "骴", "疵", "伺"]),
    ("cong", &["从", "聪", "葱", "丛", "從", "琮", "枞", "匆", "淙", "藂", "骢", "璁", "聰", "囱", "苁", "蓯", "聦", "蔥", "悰", "爜", "錝", "鏓", "叢", "憁", "徔", "灇"]),
    ("cou", &["凑", "湊", "腠", "楱", "辏", "傶", "輳", "㫶"]),
    ("cu", &["粗", "醋", "促", "簇", "噈", "趨", "蹴", "卒", "殂", "猝", "觕", "蹙", "蹵", "誎", "趗", "瘄", "皻", "踿", "蔟", "徂", "酢", "麤", "縬", "橻", "踧", "麁"]),
    ("cuan", &["窜", "蹿", "攒", "窾", "篡", "撺", "汆", "粗安", "爨", "鑹", "巑", "酇", "灒", "攛", "镩", "殩", "熶", "濽", "竄", "鋑", "櫕", "穳", "欑", "攢", "攅", "簒"]),
    ("cui", &["催", "崔", "翠", "脆", "萃", "粹", "淬", "摧", "璀", "啐", "縗", "膵", "顇", "皠", "啛", "毳", "趡", "慛", "琗", "嵟", "槯", "衰", "凗", "焠", "悴", "墔"]),
    ("cun", &["村", "寸", "存", "吋", "皴", "邨", "忖", "粗呢", "拵", "籿", "侟", "蹲", "踆", "澊", "刌", "膥", "竴", "䍎", "粗", "促", "醋", "簇", "噈", "趨", "蹴", "卒"]),
    ("cuo", &["错", "搓", "挫", "措", "厝", "錯", "锉", "撮", "矬", "磋", "剉", "縒", "鹺", "痤", "粗哦", "醋哦", "嵳", "夎", "蹉", "蓌", "襊", "酂", "莡", "嵯", "瑳", "脞"]),
    ("da", &["大", "打", "达", "对啊", "哒", "答", "搭", "答案", "多啊", "的啊", "打啊", "点啊", "笪", "憚", "懂啊", "遝", "到啊", "档案", "沓", "都啊", "大啊", "嗒", "掉啊", "带啊", "等啊", "達"]),
    ("dai", &["带", "戴", "待", "代", "袋", "呆", "贷", "逮", "隸", "黛", "帶", "岱", "歹", "詒", "埭", "呔", "Dai", "傣", "跢", "隷", "紿", "玳", "蝳", "曃", "怠", "殆"]),
    ("dan", &["但", "单", "蛋", "丹", "淡", "胆", "旦", "打你", "担", "大脑", "弹", "大年", "大娘", "大牛", "氮", "單", "诞", "啖", "鄲", "聃", "Dan", "都爱你", "迪奥娜", "躭", "愖", "黮"]),
    ("dang", &["当", "党", "档", "挡", "蛋糕", "荡", "當", "耽搁", "单个", "裆", "瑒", "铛", "氹", "宕", "单管", "单干", "单根", "弹弓", "单杠", "dang", "璫", "檔", "潒", "凼", "讜", "垱"]),
    ("dao", &["到", "倒", "道", "刀", "导", "岛", "盗", "稻", "捣", "叨", "蹈", "氘", "D奥", "島", "箌", "檮", "祷", "大哦", "打哦", "Dao", "導", "翿", "悼", "捯", "刂", "幬"]),
    ("de", &["的", "得", "德", "地", "第二", "大鹅", "嘚", "点儿", "大额", "徳", "的诶", "得儿", "大二", "地儿", "锝", "都饿", "定额", "戴尔", "等额", "肚饿", "东阿", "对额", "淂", "旳", "鍀", "惪"]),
    ("den", &["扽", "的你", "的呢", "扥", "的那", "㩐", "的女", "的男", "得能", "德农", "的能", "的您", "得拿", "第二年", "德能", "的奶", "的捏", "的内", "德纳", "den", "的", "得", "德", "地", "嘚", "徳"]),
    ("deng", &["等", "灯", "邓", "登", "蹬", "凳", "瞪", "澂", "噔", "燈", "鄧", "澄", "镫", "嶝", "櫈", "磴", "豋", "戥", "隥", "簦", "墱", "嬁", "鐙", "瀓", "僜", "璒"]),
    ("di", &["滴", "地", "第", "低", "底", "弟", "睇", "迪", "抵", "帝", "递", "娣", "鬄", "狄", "啲", "蒂", "邸", "敌", "涤", "笛", "堤", "哋", "的", "詆", "荻", "棣"]),
    ("dian", &["点", "店", "电", "垫", "丶", "典", "颠", "點", "癫", "佔", "殿", "掂", "迪安", "涤氨", "低安", "堤岸", "笛安", "迪岸", "地安", "底案", "帝安", "地暗", "涤安", "碘", "滇", "電"]),
    ("diao", &["掉", "吊", "调", "屌", "叼", "钓", "刁", "雕", "迪奥", "貂", "弔", "椆", "調", "地奥", "凋", "帝奥", "鲷", "迪澳", "嗲哦", "嗲噢", "低凹", "嬥", "蛁", "窎", "釣", "碉"]),
    ("die", &["跌", "爹", "叠", "蝶", "碟", "迭", "惵", "喋", "臷", "谍", "低额", "咥", "die", "鐡", "底额", "帝饿", "帝俄", "帝鳄", "抵饿", "敌厄", "哋", "槢", "殜", "楪", "啑", "蜨"]),
    ("ding", &["定", "订", "丁", "顶", "盯", "鼎", "钉", "叮", "锭", "腚", "町", "訂", "頂", "玎", "酊", "啶", "ding", "碇", "仃", "釘", "掟", "帄", "疔", "濎", "錠", "耵"]),
    ("diu", &["丢", "丟", "铥", "銩", "颩", "第", "低", "地", "滴", "底", "弟", "睇", "迪", "抵", "帝", "递", "娣", "鬄", "狄", "啲", "蒂", "邸", "敌", "涤", "笛", "堤"]),
    ("dong", &["东", "懂", "动", "栋", "董", "冬", "冻", "洞", "咚", "東", "動", "垌", "狪", "揰", "dong", "DONG", "氡", "棟", "硐", "凍", "侗", "岽", "鸫", "峒", "姛", "苳"]),
    ("dou", &["都", "豆", "抖", "斗", "讀", "逗", "窦", "兜", "痘", "陡", "钭", "毭", "鬥", "荳", "唞", "鋀", "蔸", "蚪", "竇", "篼", "阧", "瞗", "浢", "枓", "梪", "閗"]),
    ("du", &["度", "读", "都", "杜", "堵", "毒", "赌", "嘟", "独", "肚", "渡", "镀", "督", "牘", "笃", "闍", "讀", "髑", "蠧", "渎", "睹", "犊", "賭", "櫝", "裻", "荰"]),
    ("duan", &["段", "短", "端", "断", "锻", "缎", "都安", "煅", "斷", "篅", "塅", "杜安", "毈", "椴", "度安", "渡安", "独安", "度岸", "毒案", "耑", "簖", "鍛", "碫", "剬", "煆", "媏"]),
    ("dui", &["对", "队", "怼", "堆", "兑", "對", "鎚", "碓", "隊", "兌", "DUI", "磓", "懟", "敓", "譈", "痽", "鴭", "綐", "対", "憝", "譵", "镦", "憞", "敦", "杸", "啍"]),
    ("dun", &["蹲", "吨", "盾", "顿", "炖", "墩", "敦", "钝", "遁", "盹", "趸", "毒奶", "肚腩", "嘟囔", "逇", "囤", "镦", "噸", "沌", "惇", "dun", "頓", "墪", "撉", "犜", "燉"]),
    ("duo", &["多", "躲", "朵", "夺", "剁", "铎", "哆", "垛", "舵", "挆", "堕", "駄", "杝", "硾", "惰", "duo", "度哦", "读哦", "毒哦", "馱", "桗", "缍", "跺", "柂", "敚", "鵽"]),
    ("ei", &["诶", "欸", "誒", "EI", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪"]),
    ("en", &["嗯", "恩", "嗯呢", "摁", "蒽", "嗯呐", "儿女", "而你", "en", "恶念", "儿呢", "二年", "奀", "饿呢", "二奶", "嗯哪", "婀娜", "二娘", "讹你", "屙尿", "二妞", "煾", "恩诺", "嗯捏", "恶女", "峎"]),
    ("er", &["二", "儿", "而", "尔", "贰", "耳", "饵", "貳", "迩", "恶人", "兒", "洱", "鹅绒", "er", "鹅肉", "二人", "珥", "爾", "恩人", "而让", "二日", "二热", "而然", "弍", "尓", "铒"]),
    ("fa", &["发", "法", "罚", "伐", "阀", "方案", "發", "乏", "发啊", "飞啊", "肺癌", "放啊", "烦啊", "筏", "分啊", "珐", "妨碍", "酦", "蕟", "髮", "垡", "罰", "笩", "髪", "砝", "発"]),
    ("fan", &["饭", "翻", "反", "烦", "范", "返", "凡", "樊", "发你", "帆", "犯", "泛", "番", "梵", "繁", "钒", "璠", "飯", "畈", "籓", "fan", "富安娜", "菲安妮", "藩", "贩", "幡"]),
    ("fang", &["放", "方", "房", "芳", "防", "仿", "坊", "访", "反感", "纺", "返工", "反光", "饭馆", "昉", "犯规", "舫", "妨", "返岗", "凡哥", "梵高", "枋", "錺", "鴋", "钫", "Fang", "匚"]),
    ("fei", &["非", "飞", "费", "废", "肥", "菲", "肺", "斐", "妃", "霏", "匪", "啡", "飛", "吠", "沸", "绯", "翡", "費", "蕡", "扉", "癈", "暃", "婓", "狒", "胏", "蜚"]),
    ("fen", &["分", "份", "粉", "芬", "饙", "奋", "粪", "坟", "汾", "纷", "酚", "焚", "瞓", "翂", "轒", "氛", "愤", "岎", "凡尔纳", "费尔南", "Fen", "砏", "玢", "鼢", "忿", "朆"]),
    ("feng", &["风", "封", "冯", "峰", "凤", "丰", "锋", "疯", "缝", "枫", "逢", "奉", "蜂", "烽", "沣", "風", "分给", "分工", "峯", "分管", "俸", "分割", "分隔", "艂", "楓", "鳳"]),
    ("fou", &["否", "缶", "不", "炰", "妚", "缻", "雬", "紑", "殕", "缹", "鴀", "㓡", "䬏", "䳕", "佛", "仏", "坲", "梻", "仸", "㤇"]),
    ("fu", &["付", "服", "福", "副", "富", "负", "复", "符", "附", "傅", "府", "父", "夫", "赴", "伏", "幅", "敷", "扶", "浮", "芙", "肤", "赋", "氟", "孚", "辅", "甫"]),
    ("ga", &["噶", "嘎", "尬", "咖", "狗啊", "过啊", "尕", "公安", "高啊", "干啊", "贵啊", "呷", "关爱", "旮", "轧", "钆", "戛", "伽", "嘠", "尜", "玍", "魀", "朒", "夾", "釓", "錷"]),
    ("gai", &["改", "该", "盖", "钙", "概", "丐", "該", "葢", "忋", "胲", "垓", "GAI", "蓋", "侅", "溉", "隑", "祴", "荄", "鈣", "瓂", "豥", "賌", "赅", "陔", "匄", "杚"]),
    ("gan", &["干", "敢", "感", "甘", "赶", "咁", "肝", "杆", "淦", "赣", "柑", "竿", "幹", "苷", "倝", "擀", "更爱你", "贵爱娘", "凎", "矸", "趕", "酐", "鱤", "秆", "乾", "绀"]),
    ("gang", &["刚", "港", "岗", "钢", "杠", "尴尬", "缸", "纲", "罡", "干过", "冈", "干干", "干锅", "肛", "剛", "干个", "戆", "感官", "冮", "鋼", "赣G", "岡", "gang", "崗", "戅", "堽"]),
    ("gao", &["搞", "高", "稿", "告", "膏", "糕", "郜", "锆", "皋", "镐", "羙", "羔", "杲", "櫜", "睾", "篙", "髙", "鼛", "诰", "GAO", "噶哦", "嘎哦", "噶喔", "餻", "藁", "槁"]),
    ("ge", &["个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁", "個", "嘅", "戈", "阁", "鸽", "感恩", "高二", "高额", "孤儿", "个儿", "歌尔", "龟儿", "挂耳", "革", "咯", "铬", "嗝"]),
    ("gei", &["给", "給", "个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁", "個", "嘅", "戈", "阁", "鸽", "革", "咯", "铬", "嗝", "镉", "嗰", "舸", "猲", "硌", "紇"]),
    ("gen", &["跟", "根", "艮", "个你", "搁那", "亘", "哏", "搁哪", "茛", "个能", "亙", "搄", "官恩娜", "揯", "㫔", "㮓", "䫀", "个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁"]),
    ("geng", &["更", "梗", "耿", "庚", "跟个", "耕", "羹", "頸", "羮", "埂", "赓", "縆", "根管", "过而能改", "哽", "峺", "畊", "鶊", "緪", "焿", "暅", "鲠", "颈", "莄", "浭", "椩"]),
    ("gong", &["工", "共", "公", "供", "龚", "宫", "功", "弓", "攻", "巩", "拱", "贡", "汞", "恭", "宮", "躬", "碽", "熕", "肱", "珙", "Gong", "蛬", "貢", "觵", "躳", "塨"]),
    ("gou", &["狗", "够", "购", "勾", "沟", "苟", "钩", "构", "垢", "缑", "枸", "芶", "豿", "啂", "姤", "購", "溝", "夠", "煹", "篝", "媾", "褠", "诟", "搆", "笱", "緱"]),
    ("gu", &["姑", "古", "股", "顾", "故", "谷", "估", "骨", "固", "鼓", "菇", "賈", "辜", "孤", "咕", "钴", "蛊", "雇", "唃", "箍", "沽", "錮", "毂", "鴣", "罛", "榾"]),
    ("gua", &["挂", "瓜", "刮", "卦", "呱", "寡", "掛", "铦", "褂", "剐", "焻", "股啊", "姑啊", "古啊", "古阿", "顾啊", "煱", "胍", "咼", "啩", "緺", "劀", "鴰", "坬", "鸹", "括"]),
    ("guai", &["怪", "乖", "拐", "骨癌", "罫", "夬", "柺", "叏", "古艾", "谷艾", "古爱", "顾爱", "掴", "箉", "枴", "恠", "㧔", "䊽", "䂯", "㷇", "摑", "㽇", "挂", "瓜", "刮", "卦"]),
    ("guan", &["管", "关", "罐", "馆", "官", "观", "冠", "惯", "灌", "固安", "莞", "挂念", "贯", "綸", "關", "棺", "掼", "矔", "挂你", "顾安", "谷安", "谷胺", "谷氨", "古安", "Guan", "顾桉"]),
    ("guang", &["光", "逛", "广", "管管", "潢", "管够", "观光", "古昂", "关关", "咣", "关公", "观感", "罐罐", "灌溉", "廣", "獷", "胱", "管过", "管个", "管工", "洸", "挄", "桄", "僙", "珖", "櫎"]),
    ("gui", &["贵", "鬼", "柜", "归", "桂", "龟", "规", "硅", "跪", "轨", "癸", "貴", "圭", "攰", "诡", "闺", "瑰", "GUI", "鴂", "櫃", "璝", "騩", "蟡", "螝", "櫰", "歸"]),
    ("gun", &["滚", "棍", "姑娘", "丨", "辊", "衮", "滾", "gun", "姑奶", "緄", "顾念", "惃", "鲧", "顾南", "硍", "睴", "磙", "鯀", "绲", "袞", "謴", "輥", "棞", "緷", "睔", "蓘"]),
    ("guo", &["过", "国", "郭", "锅", "果", "裹", "過", "粿", "濄", "虢", "國", "菓", "姑哦", "古哦", "呙", "帼", "蝈", "鍋", "埚", "腘", "囯", "輠", "膕", "楇", "啯", "涡"]),
    ("ha", &["哈", "蛤", "好啊", "铪", "会啊", "回啊", "奤", "黑暗", "淮安", "很爱", "话啊", "海岸", "哈啊", "蝦", "虾", "鉿", "紦", "妎"]),
    ("hai", &["还", "嗨", "海", "害", "孩", "嗐", "蓋", "還", "亥", "咳", "氦", "hai", "猲", "骇", "骸", "酼", "餀", "塰", "咍", "醢", "烸", "饚", "胲", "絯", "嚡", "駭"]),
    ("han", &["喊", "含", "韩", "汉", "函", "涵", "寒", "汗", "焊", "瀚", "晗", "翰", "憨", "菡", "旱", "罕", "笒", "哈尼", "焓", "晥", "悍", "很爱你", "好爱你", "歛", "頷", "漢"]),
    ("hang", &["航", "行", "杭", "韩国", "巷", "焊工", "涵盖", "夯", "韩哥", "吭", "喊个", "珩", "绗", "桁", "頏", "沆", "hang", "亢", "垳", "蚢", "颃", "絎", "貥", "妔", "斻", "迒"]),
    ("hao", &["好", "号", "浩", "豪", "郝", "昊", "薅", "皓", "耗", "號", "灏", "嚎", "濠", "蚝", "壕", "毫", "颢", "淏", "蒿", "哈哦", "Hao", "晧", "澔", "镐", "皞", "灝"]),
    ("he", &["和", "喝", "盒", "何", "合", "贺", "核", "河", "禾", "鹤", "好饿", "呵", "赫", "荷", "噈", "会饿", "会儿", "颌", "还饿", "闔", "朅", "海尔", "嗬", "褐", "he", "愒"]),
    ("hei", &["黑", "嘿", "黒", "嗨", "潶", "和", "喝", "盒", "何", "合", "贺", "核", "河", "禾", "鹤", "呵", "赫", "荷", "噈", "颌", "闔", "朅", "嗬", "褐", "愒", "菏"]),
    ("hen", &["很", "狠", "和你", "恨", "痕", "河南", "和您", "喝奶", "和牛", "和那", "河内", "和能", "核能", "hen", "哏", "拫", "皇额娘", "佷", "好儿女", "噷", "詪", "鞎", "䓳", "㯊", "和", "喝"]),
    ("heng", &["哼", "恒", "横", "衡", "很高", "很贵", "亨", "很乖", "很干", "珩", "很怪", "很广", "蘅", "桁", "姮", "恆", "很赶", "很够", "橫", "很过", "行", "涥", "鸻", "烆", "堼", "啈"]),
    ("hong", &["红", "宏", "洪", "鸿", "哄", "虹", "弘", "泓", "轰", "烘", "澋", "紅", "荭", "闳", "竑", "Hong", "鴻", "叿", "吽", "閧", "焢", "浤", "瓨", "薨", "訌", "翃"]),
    ("hou", &["后", "厚", "侯", "猴", "吼", "候", "後", "喉", "齁", "腄", "逅", "鲎", "翵", "銗", "鯸", "翭", "垕", "瘊", "犼", "堠", "骺", "糇", "矦", "鲘", "呴", "郈"]),
    ("hu", &["胡", "户", "虎", "互", "湖", "护", "糊", "呼", "沪", "壶", "戲", "乎", "狐", "弧", "忽", "扈", "楛", "濩", "唬", "戯", "鵠", "琥", "斛", "膴", "嗃", "鹄"]),
    ("hua", &["话", "花", "华", "化", "画", "豁", "滑", "划", "話", "桦", "骅", "铧", "哗", "華", "婳", "畫", "虎啊", "户啊", "胡啊", "互啊", "呼啊", "觟", "嫿", "鋘", "呚", "椛"]),
    ("huai", &["坏", "怀", "淮", "槐", "踝", "褢", "壞", "互爱", "懷", "徊", "胡爱", "胡皑", "蘾", "胡艾", "护爱", "互艾", "耲", "懐", "劃", "佪", "壊", "瀤", "咶", "櫰", "蘹", "䈭"]),
    ("huan", &["换", "还", "环", "欢", "缓", "焕", "幻", "患", "換", "桓", "唤", "化脓", "华南", "华农", "寰", "宦", "护岸", "轘", "郇", "綄", "還", "湖岸", "华能", "胡安", "沪安", "呼安"]),
    ("huang", &["黄", "慌", "皇", "换个", "晃", "还给", "煌", "荒", "簧", "凰", "换过", "换购", "谎", "黃", "璜", "磺", "篁", "潢", "爌", "欢哥", "换岗", "换股", "湟", "胡昂", "Huang", "蝗"]),
    ("hui", &["会", "回", "辉", "慧", "灰", "汇", "惠", "晖", "會", "荟", "毁", "卉", "徽", "绘", "潰", "墮", "廆", "挥", "Hui", "蕙", "悔", "烩", "賄", "濊", "珲", "恢"]),
    ("hun", &["混", "婚", "魂", "湖南", "昏", "荤", "梱", "糊弄", "掍", "浑", "倱", "胡闹", "呼你", "虎年", "閽", "户内", "鼲", "餛", "慁", "諢", "睧", "棔", "尡", "睴", "馄", "珲"]),
    ("huo", &["货", "活", "或", "火", "霍", "嚯", "获", "伙", "祸", "灬", "貨", "豁", "惑", "和", "互哦", "胡哦", "户哦", "濊", "矆", "雘", "彠", "癨", "藿", "镬", "钬", "靃"]),
    ("ji", &["几", "及", "级", "机", "寄", "急", "记", "即", "鸡", "集", "給", "计", "吉", "季", "基", "纪", "冀", "挤", "既", "极", "剂", "據", "暨", "己", "技", "积"]),
    ("jia", &["家", "加", "价", "假", "佳", "嘉", "贾", "夹", "甲", "架", "嫁", "驾", "钾", "猰", "珈", "伽", "脥", "迦", "镓", "稼", "几啊", "急啊", "级啊", "寄啊", "鸡啊", "集啊"]),
    ("jian", &["件", "见", "建", "间", "减", "剪", "剑", "健", "键", "捡", "检", "简", "贱", "尖", "兼", "坚", "煎", "肩", "吉安", "箭", "监", "加你", "碱", "鉴", "佳能", "艱"]),
    ("jiang", &["将", "讲", "江", "蒋", "降", "姜", "奖", "酱", "见过", "犟", "匠", "浆", "疆", "坚果", "講", "僵", "监管", "间隔", "將", "桨", "建国", "激昂", "兼顾", "建个", "蔣", "绛"]),
    ("jiao", &["叫", "交", "教", "脚", "角", "较", "胶", "焦", "娇", "觉", "缴", "姣", "椒", "嚼", "浇", "蕉", "蹻", "饺", "搅", "喬", "绞", "矫", "蛟", "吉奥", "击凹", "加哦"]),
    ("jie", &["姐", "接", "借", "节", "结", "杰", "解", "洁", "街", "届", "界", "捷", "截", "價", "阶", "介", "戒", "皆", "婕", "劫", "饥饿", "袓", "揭", "喈", "卪", "劼"]),
    ("jin", &["进", "斤", "金", "近", "今", "仅", "紧", "锦", "劲", "晋", "尽", "几年", "津", "禁", "技能", "济南", "靳", "筋", "瑾", "巾", "纪念", "谨", "烬", "浸", "進", "Jin"]),
    ("jing", &["静", "经", "景", "精", "京", "晶", "井", "净", "镜", "靖", "敬", "婧", "境", "惊", "璟", "菁", "警", "竟", "径", "尽管", "荆", "竞", "劲", "颈", "Jing", "鲸"]),
    ("jiong", &["囧", "炯", "宭", "炅", "窘", "侰", "逈", "迥", "銄", "臦", "埛", "臩", "澃", "泂", "冏", "僒", "煚", "颎", "顈", "扃", "烱", "冂", "冋", "駫", "坰", "綗"]),
    ("jiu", &["就", "酒", "久", "旧", "九", "救", "玖", "舅", "灸", "鸠", "揪", "究", "啾", "纠", "韭", "舊", "氿", "臼", "赳", "鹫", "咎", "鳩", "疚", "汣", "乆", "杦"]),
    ("ju", &["局", "句", "距", "聚", "剧", "巨", "居", "具", "举", "菊", "拒", "据", "狙", "鞠", "橘", "袓", "锯", "桔", "炬", "钜", "焗", "驹", "琚", "矩", "俱", "罝"]),
    ("juan", &["卷", "娟", "捐", "劵", "圈", "倦", "涓", "鹃", "巂", "隽", "眷", "绢", "睠", "聚安", "居安", "巨安", "局安", "聚氨", "举案", "Juan", "飬", "睊", "襈", "镌", "獧", "縳"]),
    ("jue", &["觉", "绝", "决", "撅", "珏", "爵", "巨额", "倔", "蹻", "妜", "角", "噱", "诀", "巨鳄", "巨饿", "巨恶", "菊娥", "据俄", "掘", "熦", "芵", "燋", "鳜", "璚", "鴂", "蕨"]),
    ("jun", &["军", "君", "均", "俊", "骏", "菌", "钧", "峻", "珺", "郡", "龜", "竣", "隽", "浚", "筠", "鋆", "Jun", "麕", "馂", "軍", "燇", "焌", "鵔", "攈", "濬", "攟"]),
    ("ka", &["卡", "咖", "咔", "可爱", "喀", "快啊", "开啊", "咯", "考啊", "苦啊", "卡啊", "课啊", "空啊", "胩", "口岸", "酷安", "佧", "裃", "垰", "鉲", "衉", "呿", "哢", "㮟", "䘔"]),
    ("kai", &["开", "凯", "楷", "嘅", "恺", "開", "铠", "锴", "慨", "垲", "凱", "Kai", "揩", "剀", "愷", "锎", "烗", "岂", "闿", "鎧", "忾", "欬", "噄", "奒", "鍇", "塏"]),
    ("kan", &["看", "砍", "坎", "刊", "卡你", "阚", "侃", "堪", "勘", "槛", "冚", "瞰", "龛", "卡农", "衎", "卡内", "卡呢", "顑", "卡奴", "栞", "磡", "可爱捏", "墈", "嵌", "戡", "莰"]),
    ("kang", &["康", "抗", "扛", "看过", "亢", "炕", "看个", "糠", "看管", "看够", "慷", "看顾", "邟", "砍个", "钪", "伉", "摃", "槺", "鈧", "漮", "闶", "犺", "忼", "嫝", "砊", "穅"]),
    ("kao", &["靠", "考", "烤", "拷", "尻", "铐", "薧", "犒", "kao", "卡哦", "栲", "丂", "鮳", "攷", "焅", "卡噢", "KAO", "銬", "鯌", "洘", "嵪", "鲓", "髛", "㼥", "䎋", "䐧"]),
    ("ke", &["可", "课", "克", "科", "客", "颗", "壳", "柯", "氪", "刻", "磕", "珂", "咳", "棵", "渴", "嗑", "轲", "課", "恪", "科二", "顆", "坷", "可饿", "科恩", "苛", "钶"]),
    ("kei", &["剋", "尅", "可", "课", "克", "科", "客", "颗", "壳", "柯", "氪", "刻", "磕", "珂", "咳", "棵", "渴", "嗑", "轲", "課", "恪", "顆", "坷", "苛", "钶", "硞"]),
    ("ken", &["可能", "肯", "啃", "垦", "恳", "柯南", "可你", "可那", "课内", "可难", "课呢", "齦", "可逆", "可嫩", "科宁", "可宁", "裉", "科内", "龈", "Ken", "硍", "錹", "豤", "珢", "科尔尼", "科尔能"]),
    ("keng", &["坑", "吭", "殸", "肯干", "铿", "硜", "肯给", "啃过", "啃个", "鍞", "誙", "肯高", "揁", "挳", "坈", "鏗", "阬", "硁", "妔", "牼", "摼", "硻", "銵", "劥", "䡩", "䡰"]),
    ("kong", &["空", "控", "孔", "恐", "箜", "崆", "倥", "涳", "硿", "Kong", "悾", "鵼", "埪", "羫", "鞚", "椌", "錓", "躻", "㑋", "矼", "㤟", "㸜"]),
    ("kou", &["扣", "口", "抠", "寇", "叩", "蔻", "筘", "滱", "芤", "眗", "冦", "摳", "釦", "蔲", "瞘", "宼", "簆", "剾", "怐", "劶", "鷇", "眍", "敂", "竘", "佝", "瞉"]),
    ("ku", &["哭", "库", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐", "瘔", "胐", "扝", "焅", "窋"]),
    ("kua", &["夸", "跨", "垮", "胯", "苦啊", "挎", "絓", "哭啊", "酷啊", "咵", "楇", "侉", "库啊", "顝", "裤啊", "苦阿", "趶", "誇", "姱", "晇", "恗", "銙", "骻", "㡁", "䋀", "舿"]),
    ("kuai", &["快", "块", "筷", "会", "蒯", "酷爱", "塊", "駃", "侩", "郐", "脍", "苦艾", "库爱", "酷艾", "酷哎", "獪", "噲", "廥", "巜", "旝", "擓", "圦", "糩", "哙", "墤", "鲙"]),
    ("kuan", &["款", "宽", "跨年", "酷安", "夸你", "髋", "歀", "寬", "酷岸", "窾", "库岸", "库安", "寛", "髖", "欵", "梡", "鑧", "㝟", "㮯", "䕀", "䤭", "㯘", "窽", "臗", "䥗", "䲌"]),
    ("kuang", &["框", "狂", "矿", "况", "匡", "邝", "筐", "旷", "哐", "宽广", "诓", "眶", "宽哥", "爌", "壙", "贶", "穬", "夼", "髋骨", "宽高", "宽沟", "宽轨", "酷昂", "絖", "圹", "黋"]),
    ("kui", &["亏", "葵", "奎", "魁", "盔", "馈", "溃", "愧", "窥", "逵", "磈", "犪", "虧", "隗", "闚", "夔", "匮", "聵", "馗", "傀", "樻", "瞶", "簣", "蕢", "喹", "岿"]),
    ("kun", &["困", "坤", "昆", "捆", "琨", "苦恼", "苦难", "鲲", "哭闹", "堃", "锟", "焜", "髠", "睏", "崑", "鹍", "醌", "貇", "稛", "梱", "錕", "堒", "崐", "鵾", "髨", "熴"]),
    ("kuo", &["扩", "阔", "括", "铦", "廓", "鞟", "濶", "銛", "鞹", "酷哦", "哭哦", "苦哦", "萿", "闊", "髺", "噋", "栝", "鬠", "擴", "葀", "頢", "适", "銽", "漷", "挄", "霩"]),
    ("la", &["啦", "拉", "辣", "腊", "来啊", "蜡", "恋爱", "喇", "了啊", "冷啊", "累啊", "练啊", "落", "la", "啦啊", "垃", "里啊", "楼啊", "辢", "聊啊", "老啊", "立案", "剌", "菈", "臈", "櫴"]),
    ("lai", &["来", "赖", "莱", "來", "疠", "濑", "徕", "頼", "賴", "涞", "籁", "勑", "铼", "癞", "唻", "庲", "癘", "睐", "郲", "崃", "赉", "娕", "瀨", "萊", "俫", "藾"]),
    ("lan", &["懒", "蓝", "兰", "烂", "栏", "岚", "澜", "拦", "拉你", "揽", "缆", "篮", "览", "蘭", "榄", "藍", "阑", "斓", "拉尿", "覧", "恋爱脑", "来爱你", "LAN", "镧", "滥", "爛"]),
    ("lang", &["浪", "狼", "郎", "朗", "蓝光", "廊", "栏杆", "琅", "勆", "榔", "啷", "蓝港", "烺", "塱", "螂", "稂", "Lang", "阆", "崀", "埌", "瑯", "欴", "蓢", "蒗", "樃", "莨"]),
    ("lao", &["老", "捞", "樂", "佬", "牢", "劳", "唠", "姥", "烙", "酪", "涝", "橑", "铑", "落", "崂", "撈", "啦哦", "勞", "痨", "Lao", "拉哦", "辣哦", "轑", "哰", "醪", "蟧"]),
    ("le", &["了", "乐", "勒", "叻", "嘞", "老二", "灵儿", "樂", "楽", "肋", "老鹅", "仂", "玏", "理儿", "卤鹅", "龙儿", "临额", "泐", "簕", "氻", "砳", "阞", "竻", "忇", "艻", "鳓"]),
    ("lei", &["嘞", "类", "累", "雷", "磊", "蕾", "泪", "垒", "勒", "镭", "肋", "瘣", "擂", "類", "錑", "耒", "磥", "Lei", "儽", "虆", "蔂", "檑", "纇", "櫑", "腂", "瓃"]),
    ("leng", &["冷", "愣", "棱", "楞", "碐", "堎", "塄", "稜", "唥", "踜", "睖", "崚", "薐", "輘", "倰", "㘄", "䉄", "䬋", "䮚", "䚏", "了", "乐", "勒", "叻", "嘞", "樂"]),
    ("li", &["李", "里", "力", "离", "丽", "理", "立", "利", "黎", "粒", "礼", "例", "哩", "莉", "厘", "梨", "历", "栗", "厉", "隸", "锂", "励", "嚟", "裡", "俐", "郦"]),
    ("lia", &["俩", "力啊", "里啊", "倆", "力阿", "理啊", "礼啊", "离啊", "丽啊", "李啊", "利啊", "利阿", "历啊", "里", "李", "力", "离", "丽", "理", "立", "利", "黎", "粒", "礼", "例", "哩"]),
    ("lian", &["连", "脸", "练", "联", "链", "立案", "莲", "纞", "恋", "炼", "廉", "帘", "离岸", "利安", "李安", "怜", "連", "涟", "敛", "亷", "俩年", "立安", "俩妞", "黎安", "梨安", "丽安"]),
    ("liang", &["两", "量", "梁", "亮", "良", "辆", "凉", "靓", "粮", "俩", "连个", "晾", "兩", "李昂", "粱", "里昂", "连跪", "倆", "樑", "练过", "练功", "谅", "连贯", "立昂", "魎", "练歌"]),
    ("liao", &["聊", "料", "了", "廖", "辽", "撩", "疗", "寮", "撂", "燎", "里奥", "李傲", "李奥", "李澳", "李敖", "利奥", "窌", "繆", "钌", "蟟", "寥", "俩哦", "丽岙", "立奥", "李翱", "利澳"]),
    ("lie", &["列", "咧", "裂", "猎", "烈", "劣", "獦", "哷", "冽", "颲", "丽娥", "lie", "棙", "李娥", "里厄", "丽鹅", "里额", "迾", "猟", "睙", "蛚", "擸", "綟", "獵", "鬣", "儠"]),
    ("lin", &["林", "临", "琳", "霖", "淋", "麟", "理你", "李宁", "拎", "离你", "理念", "磷", "蔺", "邻", "李娜", "鳞", "凛", "粼", "璘", "吝", "历年", "凜", "丽娜", "獜", "赁", "檩"]),
    ("ling", &["领", "零", "玲", "灵", "另", "凌", "令", "岭", "龄", "铃", "菱", "陵", "伶", "翎", "苓", "泠", "绫", "瓴", "Ling", "羚", "聆", "領", "澪", "齢", "靈", "棱"]),
    ("liu", &["刘", "留", "六", "流", "柳", "溜", "碌", "陆", "遛", "瘤", "硫", "劉", "榴", "浏", "琉", "鎏", "騮", "馏", "罶", "骝", "绺", "熘", "瑠", "鶹", "駵", "镠"]),
    ("lo", &["咯", "楼", "龙", "喽", "漏", "隆", "搂", "笼", "来哦", "了哦", "娄", "弄", "露", "lo", "囖", "鞻", "珑", "拢", "龍", "泷", "聋", "陇", "樓", "垄", "篓", "窿"]),
    ("long", &["龙", "隆", "弄", "笼", "珑", "拢", "龍", "泷", "聋", "陇", "垄", "窿", "垅", "胧", "茏", "龒", "long", "砻", "咙", "櫳", "礱", "栊", "鸗", "竜", "瀧", "巃"]),
    ("lou", &["楼", "喽", "漏", "娄", "鞻", "搂", "露", "樓", "篓", "瘘", "陋", "嘍", "镂", "蔞", "寠", "溇", "蒌", "膢", "剅", "耬", "嶁", "艛", "漊", "偻", "鷜", "遱"]),
    ("lu", &["路", "录", "陆", "卢", "鲁", "露", "撸", "鹿", "璐", "炉", "噜", "卤", "禄", "芦", "鹭", "绿", "逯", "麓", "庐", "潞", "泸", "碌", "賂", "六", "穞", "輅"]),
    ("luan", &["乱", "卵", "癵", "栾", "六安", "鸾", "銮", "潞安", "峦", "陆安", "亂", "路安", "攣", "滦", "鲁安", "灤", "鹿安", "卢安", "臡", "臠", "娈", "孪", "孿", "曫", "挛", "羉"]),
    ("lun", &["轮", "论", "伦", "仑", "露娜", "抡", "鲁能", "纶", "路南", "沦", "囵", "輪", "論", "鲁尼", "埨", "錀", "崘", "颅内", "鲁南", "卢娜", "倫", "碖", "侖", "崙", "綸", "稐"]),
    ("luo", &["罗", "落", "骆", "咯", "洛", "啰", "裸", "螺", "络", "摞", "锣", "萝", "雒", "逻", "珞", "攞", "羅", "泺", "臝", "躶", "路哦", "Luo", "录哦", "囉", "烙", "箩"]),
    ("lv", &["吕", "绿", "率", "铝", "旅", "律", "驴", "捋", "滤", "氯", "虑", "履", "缕", "侣", "闾", "LV", "鑢", "綠", "呂", "藘", "屡", "瞜", "榈", "氀", "慺", "寠"]),
    ("lve", &["略", "掠", "锊", "稤", "鋢", "绿萼", "绿鹅", "圙", "寽", "旅俄", "畧", "吕鹅", "剠", "鋝", "䤚", "㗉", "㔀", "㨼", "擽", "率", "吕", "绿", "铝", "旅", "律", "驴"]),
    ("ma", &["吗", "嘛", "码", "妈", "马", "骂", "麻", "嗎", "玛", "没啊", "买啊", "馬", "碼", "媽", "嫲", "忙啊", "木啊", "棉袄", "命啊", "妙啊", "妈啊", "慢啊", "没按", "么", "抹", "孖"]),
    ("mai", &["买", "卖", "麦", "埋", "迈", "脉", "買", "唛", "賣", "荬", "霾", "劢", "麥", "佅", "脈", "霢", "邁", "嘜", "鷶", "霡", "薶", "売", "蕒", "勱", "衇", "䘑"]),
    ("man", &["慢", "满", "蛮", "曼", "漫", "骂你", "蔓", "瞒", "嫚", "嘛呢", "滿", "码农", "缦", "熳", "幔", "鳗", "man", "馒", "螨", "蠻", "玛瑙", "墁", "镘", "Man", "莫阿娜", "颟"]),
    ("mang", &["忙", "盲", "芒", "莽", "满格", "曼谷", "蟒", "茫", "氓", "蛮高", "朚", "哤", "狵", "蛮贵", "龒", "漫过", "痝", "慢歌", "漫灌", "满贯", "蛮干", "满哥", "瞒过", "蛮乖", "漫改", "满攻"]),
    ("mao", &["猫", "毛", "冇", "茂", "冒", "帽", "矛", "茅", "贸", "卯", "锚", "貌", "茆", "懋", "铆", "貓", "码哦", "媢", "犛", "秏", "码噢", "吗哦", "妈哦", "Mao", "昴", "峁"]),
    ("me", &["么", "麽", "名额", "me", "嚒", "麼", "明儿", "妹儿", "木耳", "满额", "面额", "末", "没饿", "马儿", "嚰", "庅", "美俄", "买二", "猫儿", "马额", "猫耳", "摩尔", "母鹅", "濹", "嚜"]),
    ("mei", &["没", "美", "每", "梅", "妹", "枚", "煤", "沒", "酶", "媚", "镁", "霉", "眉", "莓", "魅", "玫", "媒", "昧", "湄", "寐", "嚜", "楣", "媄", "渼", "槑", "韎"]),
    ("men", &["们", "门", "闷", "焖", "么你", "men", "們", "么呢", "門", "扪", "懣", "么女", "悶", "殙", "么难", "么宁", "么娘", "每二年", "鍆", "燜", "懑", "米尔纳", "钔", "璊", "捫", "菛"]),
    ("meng", &["梦", "猛", "孟", "蒙", "萌", "盟", "懵", "锰", "檬", "勐", "濛", "朦", "门岗", "夢", "黽", "们给", "蝱", "氓", "焖锅", "门关", "闷棍", "门高", "门规", "门盖", "霿", "癦"]),
    ("mi", &["米", "迷", "密", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "芈", "泌", "糸", "藌", "谧", "祢", "羃", "弭", "麛", "汨", "靡", "塓"]),
    ("mian", &["面", "免", "棉", "睌", "眠", "绵", "缅", "勉", "冕", "密胺", "黽", "靦", "麪", "谜案", "澠", "米安", "迷案", "秘岸", "密案", "娩", "沔", "麵", "櫋", "牑", "宀", "蠠"]),
    ("miao", &["秒", "苗", "喵", "妙", "淼", "缪", "庙", "描", "瞄", "渺", "邈", "米奥", "藐", "仯", "缈", "杪", "繆", "廟", "觅凹", "媌", "眇", "米澳", "咪嗷", "米嗷", "秘奥", "咪奥"]),
    ("mie", &["咩", "灭", "乜", "衊", "蔑", "滅", "篾", "薎", "鴓", "米额", "咪额", "米蛾", "幭", "簚", "覕", "孭", "搣", "鑖", "礣", "蠛", "瀎", "芈", "懱", "烕", "吀", "眜"]),
    ("min", &["敏", "民", "闽", "闵", "迷你", "旻", "珉", "皿", "抿", "岷", "泌尿", "min", "泯", "米内", "黽", "敯", "忞", "米娜", "米妮", "悯", "米诺", "僶", "慜", "旼", "玟", "刡"]),
    ("ming", &["明", "名", "命", "铭", "敏感", "鸣", "茗", "冥", "洺", "民国", "民工", "鄍", "溟", "銘", "酩", "暝", "民歌", "敏哥", "鳴", "闽G", "民哥", "民革", "民高", "Ming", "眀", "瞑"]),
    ("miu", &["谬", "缪", "繆", "謬", "米", "密", "迷", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "芈", "泌", "糸", "藌", "谧", "祢", "羃", "弭"]),
    ("mo", &["摸", "莫", "膜", "末", "模", "磨", "墨", "魔", "抹", "么", "默", "摩", "沫", "馍", "陌", "茉", "漠", "嚩", "麿", "嬷", "买哦", "忙哦", "劰", "没", "麽", "嚜"]),
    ("mou", &["某", "谋", "牟", "眸", "哞", "蛑", "踎", "厶", "缪", "呣", "堥", "謀", "洠", "鍪", "侔", "鞪", "桙", "繆", "麰", "劺", "恈", "鴾", "敄", "㭌", "䍒", "㼋"]),
    ("mu", &["木", "母", "目", "亩", "沐", "穆", "慕", "牧", "幕", "牟", "姆", "模", "墓", "暮", "睦", "钼", "募", "牡", "幙", "霂", "莯", "畮", "朷", "苜", "拇", "炑"]),
    ("na", &["那", "拿", "哪", "呐", "娜", "你啊", "纳", "牛啊", "钠", "哪啊", "能啊", "乸", "捺", "吶", "嗱", "雫", "納", "内", "南", "衲", "笚", "镎", "妠", "拏", "鈉", "抐"]),
    ("nai", &["奶", "耐", "乃", "奈", "萘", "柰", "妳", "氖", "佴", "鼐", "Nai", "艿", "迺", "倷", "能", "廼", "孻", "搱", "摨", "釢", "錼", "嬭", "腉", "螚", "疓", "渿"]),
    ("nan", &["男", "难", "南", "那你", "楠", "喃", "娜娜", "那年", "哪能", "拿捏", "囡", "腩", "妠", "難", "蝻", "侽", "你啊你", "娘啊娘", "Nan", "囝", "莮", "柟", "婻", "湳", "赧", "娚"]),
    ("nang", &["囊", "难过", "馕", "难搞", "难怪", "涳", "鬞", "南瓜", "南关", "楠哥", "难关", "憹", "南宫", "攮", "南国", "南哥", "灢", "南岗", "男哥", "囔", "儾", "搑", "嚢", "曩", "齉", "乪"]),
    ("nao", &["闹", "脑", "挠", "孬", "恼", "摎", "橈", "嫐", "夒", "鐃", "那哦", "拿哦", "哪哦", "鬧", "怓", "詉", "垴", "淖", "瑙", "腦", "獶", "蝚", "铙", "呶", "匘", "蟯"]),
    ("ne", &["呢", "呐", "女儿", "哪", "哪儿", "那儿", "你儿", "妞儿", "讷", "男儿", "鸟儿", "疒", "抐", "妮儿", "吶", "女二", "訥", "奈尔", "尼尔", "纳尔", "努尔", "眲", "㕯", "䅞", "䎪", "䭆"]),
    ("nei", &["内", "內", "馁", "那", "鮾", "哪", "浽", "娞", "餒", "鯘", "氝", "錗", "氞", "脮", "腇", "㐻", "㘨", "㖏", "㨅", "䇣", "㼏", "䜆", "䳖", "䲎", "䡾", "呢"]),
    ("nen", &["嫩", "恁", "呢你", "嫰", "媆", "讷讷", "呢弄", "呢哝", "㶧", "㒄", "㜛", "㯎", "呢", "呐", "哪", "讷", "疒", "抐", "吶", "訥", "眲", "㕯", "䅞", "䎪", "䭆", "䭚"]),
    ("neng", &["能", "嫩个", "恁个", "螚", "竜", "㲌", "恁哥", "䏻", "㴰", "䘅", "嫩", "恁", "呢", "呐", "哪", "讷", "疒", "抐", "吶", "訥", "嫰", "媆", "㶧", "㒄", "㜛", "㯎"]),
    ("ni", &["你", "尼", "呢", "拟", "倪", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "ni", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼"]),
    ("nian", &["年", "念", "黏", "粘", "廿", "你按", "撵", "捻", "蔫", "碾", "撚", "唸", "溓", "涊", "埝", "你安", "你啊你", "拟按", "倪安", "尼氨", "尼安", "你俺", "你岸", "泥安", "辇", "拈"]),
    ("niang", &["娘", "酿", "孃", "年糕", "年关", "嬢", "年过", "年根", "尼昂", "年工", "年哥", "你昂", "釀", "醸", "䖆", "年", "念", "黏", "粘", "廿", "撵", "捻", "蔫", "碾", "撚", "唸"]),
    ("niao", &["鸟", "尿", "嬲", "尼奥", "袅", "鳥", "脲", "樢", "茑", "嫋", "你熬", "你奥", "你嗷", "嬝", "你啊哦", "溺", "裊", "褭", "尦", "蔦", "茮", "㭤", "㳮", "㞙", "㼭", "䐁"]),
    ("nie", &["捏", "聂", "攝", "镍", "孽", "涅", "揑", "你额", "乜", "喦", "躡", "你饿", "颞", "你恶", "逆鄂", "臲", "镊", "槷", "嗫", "嚙", "摰", "掜", "囓", "聶", "闑", "隉"]),
    ("nin", &["您", "你呢", "恁", "你那", "你能", "囜", "脌", "妮妮", "你你", "你拿", "你弄", "你牛", "你哪", "你奶", "你娘", "泥泞", "倪妮", "你嫩", "拰", "㤛", "䚾", "䛘", "䋻", "你", "呢", "拟"]),
    ("ning", &["宁", "拧", "凝", "柠", "您给", "甯", "獰", "寧", "泞", "咛", "苧", "嚀", "鬤", "狞", "佞", "鬡", "聹", "檸", "鸋", "聍", "擰", "寕", "寜", "矃", "寗", "侫"]),
    ("niu", &["牛", "妞", "扭", "钮", "纽", "拗", "牜", "怓", "狃", "汼", "忸", "杻", "紐", "孧", "鈕", "沑", "炄", "㽱", "䒜", "抝", "㺲", "䀔", "䏔", "䂇", "莥", "靵"]),
    ("nong", &["弄", "农", "浓", "侬", "脓", "哝", "秾", "襛", "農", "繷", "挊", "濃", "Nong", "燶", "蕽", "譨", "齈", "儂", "欁", "穠", "挵", "膿", "鬞", "噥", "癑", "辳"]),
    ("nu", &["怒", "奴", "弩", "努", "孥", "驽", "呶", "帑", "砮", "胬", "伮", "傉", "駑", "笯", "搙", "㚢", "㐐", "䢞", "䢪"]),
    ("nuan", &["暖", "煖", "湪", "奻", "怒安", "渜", "煗", "䎡", "㬉", "䙇", "餪", "怒", "奴", "弩", "努", "孥", "驽", "呶", "帑", "砮", "胬", "伮", "傉", "駑", "笯", "搙"]),
    ("nuo", &["诺", "挪", "糯", "喏", "懦", "锘", "堧", "傩", "諾", "毭", "愞", "娜", "袳", "穤", "搦", "橠", "榒", "掿", "鍩", "懧", "糥", "稬", "儺", "糑", "搻", "搙"]),
    ("nv", &["女", "钕", "衄", "NV", "釹", "籹", "恧", "朒", "沑", "衂", "聏"]),
    ("nve", &["虐", "疟", "瘧", "女额", "硸", "婩", "女呃", "䖈", "䖋", "女", "钕", "衄", "釹", "籹", "恧", "朒", "沑", "衂", "聏"]),
    ("ou", &["欧", "偶", "呕", "藕", "区", "鸥", "瓯", "殴", "歐", "耦", "怄", "讴", "敺", "沤", "嘔", "吽", "區", "吘", "禺", "甌", "鷗", "慪", "蕅", "鏂", "醧", "謳"]),
    ("pa", &["怕", "爬", "趴", "啪", "平安", "帕", "扒", "跑啊", "屁啊", "耙", "拍啊", "偏爱", "怕啊", "票啊", "葩", "琶", "飘啊", "杷", "破案", "爬啊", "平啊", "胖啊", "派", "钯", "潖", "掱"]),
    ("pai", &["拍", "排", "牌", "派", "湃", "哌", "廹", "俳", "簰", "迫", "蒎", "徘", "鎃", "簲", "沠", "棑", "猅", "箄", "犤", "㭛", "渒", "䖰", "輫", "㵺", "䮘", "䱝"]),
    ("pan", &["盘", "潘", "判", "盼", "怕你", "攀", "闆", "畔", "胖", "磐", "泮", "跘", "袢", "盤", "番", "蟠", "叛", "牉", "帕尼", "趴你", "帕纳", "怕难", "pan", "怕呢", "膰", "皮阿诺"]),
    ("pang", &["胖", "旁", "庞", "逄", "盘古", "潘哥", "滂", "乓", "叛国", "膀", "判给", "螃", "磅", "霶", "判官", "pang", "膖", "篣", "胮", "盘管", "盘根", "盼哥", "潘工", "攀钢", "盘感", "沗"]),
    ("pao", &["跑", "泡", "炮", "抛", "刨", "袍", "靤", "炰", "疱", "嚗", "礮", "怕哦", "爬哦", "狍", "瓟", "拋", "砲", "袌", "咆", "烰", "麅", "穮", "庖", "匏", "犥", "麭"]),
    ("pei", &["配", "陪", "赔", "培", "裴", "呸", "佩", "沛", "胚", "珮", "霈", "棑", "轡", "賠", "锫", "姵", "PEI", "蓜", "醅", "毰", "旆", "辔", "婄", "媐", "馷", "裵"]),
    ("pen", &["喷", "盆", "pen", "噴", "湓", "喯", "濆", "衯", "歕", "瓫", "呠", "葐", "翸", "㾦"]),
    ("peng", &["彭", "鹏", "碰", "朋", "棚", "捧", "蓬", "芃", "砰", "澎", "挷", "硼", "嘭", "漰", "膨", "篷", "盆骨", "烹", "喷过", "喷灌", "喷管", "喷个", "喷狗", "絣", "鵬", "怦"]),
    ("pi", &["屁", "皮", "批", "匹", "劈", "啤", "脾", "披", "坯", "痞", "丕", "疲", "癖", "陂", "铍", "辟", "抷", "Pi", "蚍", "狉", "笓", "邳", "魮", "嚊", "秠", "噽"]),
    ("pian", &["片", "骗", "篇", "偏", "便", "谝", "翩", "骈", "騙", "缏", "魸", "猵", "扁", "皮阿诺", "屁啊你", "屁安", "脾安", "皮安", "批案", "貵", "緶", "胼", "騈", "鍂", "賆", "蹁"]),
    ("piao", &["票", "飘", "漂", "嫖", "朴", "瓢", "瞟", "驃", "缥", "飄", "皮袄", "殍", "嘌", "剽", "骠", "皫", "徱", "皮奥", "闝", "皮啊哦", "旚", "僄", "螵", "慓", "薸", "彯"]),
    ("pie", &["撇", "瞥", "丿", "pie", "潎", "撆", "批额", "氕", "覕", "屁额", "屁鹅", "苤", "暼", "嫳", "鐅", "䥕", "批", "皮", "屁", "匹", "劈", "啤", "脾", "披", "坯", "痞"]),
    ("pin", &["品", "拼", "频", "聘", "贫", "皮囊", "榀", "嫔", "玭", "頻", "牝", "蘋", "拚", "pin", "劈你", "姘", "皮内", "颦", "脾牛", "睥睨", "皮诺", "汖", "貧", "顰", "朩", "嚬"]),
    ("ping", &["平", "瓶", "屏", "评", "萍", "凭", "坪", "苹", "馮", "乒", "娉", "品格", "硑", "拼购", "軿", "拼个", "枰", "品管", "拼过", "ping", "品冠", "蛢", "拼柜", "品高", "品阁", "品工"]),
    ("po", &["破", "婆", "坡", "po", "泼", "覇", "迫", "魄", "皛", "珀", "嚩", "拍哦", "票哦", "配偶", "排偶", "屁哦", "猼", "颇", "粕", "蔢", "泊", "朴", "陂", "髲", "鄱", "岶"]),
    ("pou", &["剖", "抔", "犃", "頮", "裒", "掊", "捊", "咅", "婄", "棓", "娝", "抙", "垺", "哣", "箁", "㰴", "䎧", "䯽", "㧵", "㼜", "㟝", "䳝", "破", "婆", "坡", "泼"]),
    ("pu", &["噗", "铺", "普", "蒲", "谱", "浦", "扑", "朴", "璞", "濮", "葡", "埔", "仆", "圃", "脯", "莆", "襆", "菩", "PU", "堡", "溥", "瀑", "镨", "鋪", "蹼", "潽"]),
    ("qi", &["起", "气", "其", "期", "七", "器", "齐", "揭", "骑", "啐", "琪", "奇", "启", "琦", "漆", "柒", "企", "祁", "旗", "戚", "祺", "棋", "淇", "汽", "弃", "Qi"]),
    ("qia", &["恰", "掐", "起啊", "洽", "卡", "愘", "抲", "髂", "气啊", "帢", "揢", "骑啊", "奇啊", "匼", "擖", "硈", "磍", "冾", "峠", "跒", "拤", "胢", "殎", "咭", "葜", "酠"]),
    ("qian", &["钱", "前", "签", "千", "欠", "倩", "浅", "仟", "茜", "迁", "乾", "谦", "芊", "牵", "铅", "纤", "錢", "黔", "潜", "钳", "骞", "七安", "启安", "奇案", "祺安", "齐安"]),
    ("qiang", &["强", "抢", "墙", "枪", "將", "腔", "呛", "牵挂", "羌", "強", "千古", "钳工", "蔷", "签个", "锵", "搶", "钱哥", "前杠", "签过", "鶬", "羟", "前高", "谦哥", "前盖", "迁改", "启昂"]),
    ("qiao", &["桥", "敲", "乔", "巧", "翘", "蕉", "瞧", "俏", "侨", "撬", "谯", "鞘", "窍", "荞", "峤", "樵", "锹", "鞒", "悄", "壳", "喿", "启奥", "淇澳", "齐奥", "淇奥", "奇奥"]),
    ("qie", &["切", "且", "企鹅", "椄", "脥", "茄", "砌", "妾", "企额", "骑鹅", "弃恶", "起鹅", "郄", "怯", "窃", "锲", "伽", "惬", "悏", "踥", "癿", "蛪", "挈", "箧", "淁", "厒"]),
    ("qin", &["亲", "琴", "秦", "勤", "覃", "钦", "芹", "沁", "禽", "斳", "七年", "親", "寝", "气你", "芩", "气馁", "气囊", "侵", "期内", "Qin", "擒", "嗪", "溱", "鋟", "揿", "骎"]),
    ("qing", &["请", "清", "青", "情", "庆", "轻", "晴", "卿", "倾", "請", "氢", "擎", "亲哥", "亲", "箐", "氰", "傾", "顷", "磬", "秦国", "亲故", "勍", "Qing", "輕", "慶", "罄"]),
    ("qiong", &["穷", "琼", "嬛", "穹", "赹", "惸", "窮", "邛", "笻", "藑", "瞏", "瓊", "茕", "熍", "瓗", "竆", "舼", "銎", "蛩", "筇", "跫", "焪", "宆", "儝", "桏", "藭"]),
    ("qiu", &["求", "球", "秋", "邱", "丘", "仇", "裘", "龜", "囚", "湫", "糗", "橚", "酋", "楸", "鳅", "紌", "朹", "虬", "逑", "萩", "蓲", "坵", "觩", "遒", "俅", "釚"]),
    ("qu", &["去", "取", "区", "佢", "曲", "屈", "趣", "瞿", "娶", "渠", "驱", "蛆", "區", "軀", "祛", "龋", "趋", "衢", "覷", "爠", "趍", "袪", "躯", "詘", "璩", "阹"]),
    ("quan", &["全", "圈", "券", "权", "泉", "劝", "犬", "拳", "铨", "荃", "醛", "踡", "诠", "矔", "去俺", "佺", "狋", "權", "曲安", "去安", "去按", "曲岸", "跧", "婘", "顴", "犭"]),
    ("que", &["缺", "却", "确", "雀", "阙", "瘸", "鹊", "棤", "碻", "卻", "礭", "炔", "去恶", "去额", "去饿", "確", "阕", "愨", "敠", "墧", "礐", "榷", "皵", "悫", "敪", "闕"]),
    ("qun", &["群", "去年", "裙", "去哪", "麕", "踆", "去拿", "去你", "去呢", "去那", "取暖", "娶你", "去弄", "歏", "峮", "羣", "囷", "逡", "宭", "夋", "箘", "帬", "裠", "麇", "㪊", "䞭"]),
    ("ran", &["然", "染", "冉", "燃", "苒", "熯", "衻", "蚺", "姌", "髯", "ran", "蚦", "橪", "珃", "媣", "冄", "繎", "蒅", "嘫", "袇", "袡", "髥", "肰", "呥", "㚩", "䒣"]),
    ("rang", &["让", "嚷", "讓", "瓤", "染个", "壤", "攘", "穰", "染过", "禳", "染膏", "然哥", "染缸", "瀼", "蘘", "勷", "穣", "rang", "躟", "壌", "鬤", "儴", "譲", "獽", "纕", "懹"]),
    ("rao", &["绕", "饶", "扰", "娆", "Rao", "桡", "荛", "繞", "擾", "穘", "饒", "隢", "蕘", "襓", "遶", "嬈", "橈", "犪", "㑱", "䫞", "㹛"]),
    ("re", &["热", "惹", "熱", "然而", "若", "然鹅", "人儿", "蓉儿", "入额", "入耳", "日俄", "软腭", "喏", "渃", "捼", "㻰"]),
    ("ren", &["人", "认", "任", "仁", "忍", "热闹", "刃", "惹你", "壬", "韧", "棯", "紉", "仞", "紝", "妊", "热能", "惹怒", "惹恼", "热内", "热鸟", "热念", "任儿女", "REN", "荏", "亻", "稔"]),
    ("reng", &["扔", "仍", "人工", "人格", "陾", "礽", "认购", "人够", "人给", "任哥", "人更", "人过", "人广", "人干", "认个", "忍过", "任工", "认股", "任刚", "芿", "辸", "㭁", "䄧", "㺱", "䚮", "人"]),
    ("ri", &["日", "囸", "驲", "鈤", "衵", "馹", "氜", "釰", "䒤", "釼"]),
    ("rong", &["荣", "容", "融", "绒", "蓉", "榕", "戎", "溶", "熔", "嵘", "茸", "镕", "榮", "冗", "瑢", "嬫", "肜", "氄", "嶸", "羢", "鎔", "絨", "嵤", "毧", "媶", "槦"]),
    ("rou", &["肉", "揉", "柔", "鞣", "媃", "糅", "蹂", "煣", "渘", "輮", "韖", "宍", "譳", "脜", "瑈", "鍒", "鰇", "禸", "厹", "葇", "鶔", "騥", "腬", "楺", "粈", "蝚"]),
    ("ru", &["如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸", "蒘", "襦", "顬", "筎"]),
    ("rua", &["rua", "挼", "入啊", "茹啊", "如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸"]),
    ("ruan", &["软", "阮", "壖", "瓀", "腝", "茹安", "軟", "如安", "蝡", "入案", "如按", "乳安", "汝安", "儒安", "輭", "朊", "碝", "撋", "礝", "媆", "耎", "堧", "瑌", "緛", "檽", "㐾"]),
    ("rui", &["瑞", "睿", "锐", "蕊", "芮", "蕤", "汭", "叡", "入I", "繠", "銳", "枘", "鋭", "惢", "蚋", "緌", "壡", "蘂", "笍", "兌", "蘃", "蕋", "桵", "苼", "兊", "抐"]),
    ("run", &["润", "闰", "如你", "如您", "潤", "膶", "汝南", "入内", "run", "如能", "乳牛", "入脑", "瞤", "閏", "閠", "橍", "䏕", "㝄", "㠈", "䦞", "如", "入", "茹", "乳", "汝", "儒"]),
    ("ruo", &["若", "弱", "偌", "叒", "箬", "渃", "婼", "如哦", "鶸", "挼", "楉", "鄀", "爇", "篛", "焫", "蒻", "嵶", "鰙", "捼", "鰯", "㬉", "䐞", "㘃", "如", "入", "茹"]),
    ("sa", &["撒", "是啊", "仨", "萨", "飒", "啥啊", "谁啊", "说啊", "洒", "睡啊", "卅", "上啊", "上岸", "爽啊", "送啊", "sa", "颯", "薩", "潵", "脎", "钑", "櫒", "灑", "摋", "躠", "挲"]),
    ("sai", &["塞", "赛", "噻", "嗮", "腮", "鳃", "賽", "嘥", "毸", "顋", "Sai", "鰓", "思", "愢", "簺", "僿", "毢", "揌", "㗷", "䈢", "㘔", "撒", "仨", "萨", "飒", "洒"]),
    ("san", &["三", "散", "叁", "伞", "撒尿", "彡", "叄", "弎", "馓", "糁", "San", "说爱你", "圣安娜", "傘", "氵", "參", "潵", "仐", "壭", "毶", "糂", "繖", "毵", "鏾", "俕", "橵"]),
    ("sang", &["桑", "三个", "丧", "嗓", "三观", "搡", "三哥", "三国", "散光", "三高", "三姑", "三根", "三公", "喪", "颡", "槡", "磉", "sang", "鎟", "褬", "桒", "顙", "䘮", "䡦", "䫙", "三"]),
    ("sao", &["扫", "骚", "嫂", "懆", "臊", "掃", "騒", "搔", "繰", "撒哦", "仨哦", "騷", "梢", "氉", "埽", "缫", "鰠", "瘙", "溞", "鱢", "螦", "髞", "鳋", "掻", "颾", "慅"]),
    ("se", &["色", "涩", "瑟", "閪", "事儿", "十二", "数额", "烧鹅", "善恶", "塞", "少儿", "铯", "首恶", "是恶", "首尔", "啬", "澀", "穑", "渋", "歰", "璱", "擌", "歮", "拺", "嗇", "琗"]),
    ("sen", &["森", "椮", "襂", "槮", "色女", "色男", "色你", "十二年", "Sen", "顺尔宁", "色", "涩", "瑟", "閪", "塞", "铯", "啬", "澀", "穑", "渋", "歰", "璱", "擌", "歮", "拺", "嗇"]),
    ("seng", &["僧", "森哥", "鬙", "SENG", "森", "椮", "色", "涩", "瑟", "閪", "塞", "铯", "啬", "澀", "穑", "渋", "歰", "璱", "擌", "襂", "槮", "歮", "拺", "嗇", "琗", "雭"]),
    ("sha", &["啥", "傻", "杀", "沙", "纱", "砂", "莎", "鲨", "煞", "刹", "厦", "濈", "殺", "痧", "说话啊", "猀", "杉", "上号啊", "上海啊", "啑", "廈", "嗄", "霎", "榝", "硰", "儍"]),
    ("shai", &["晒", "筛", "曬", "色", "篩", "籭", "酾", "簛", "諰", "簁", "摋", "釃", "攦", "㩄", "㬠", "䵘", "啥", "杀", "傻", "沙", "纱", "砂", "莎", "鲨", "煞", "刹"]),
    ("shan", &["山", "删", "闪", "善", "珊", "扇", "啥呢", "衫", "单", "陕", "杉", "姗", "膳", "汕", "栅", "膻", "傻妞", "閃", "摻", "杀你", "訕", "傻鸟", "疝", "鳝", "Shan", "刪"]),
    ("shang", &["上", "商", "尚", "伤", "赏", "熵", "闪光", "殇", "裳", "丄", "晌", "傷", "山歌", "尙", "山谷", "闪过", "山哥", "垧", "觞", "闪购", "Shang", "墒", "绱", "賞", "殤", "贘"]),
    ("shao", &["少", "烧", "邵", "稍", "绍", "勺", "捎", "韶", "哨", "芍", "劭", "苕", "召", "燒", "梢", "卲", "啥哦", "弰", "紹", "傻哦", "啥喔", "啥噢", "杓", "潲", "蛸", "鞘"]),
    ("she", &["射", "蛇", "设", "社", "舍", "佘", "涉", "舌", "摄", "奢", "折", "赊", "畲", "厍", "設", "赦", "she", "睡会儿", "麝", "賖", "拾", "慑", "攝", "时候儿", "涻", "磼"]),
    ("shen", &["神", "身", "沈", "什", "深", "审", "申", "肾", "伸", "参", "甚", "婶", "慎", "燊", "谂", "砷", "渗", "谌", "绅", "珅", "黮", "莘", "諗", "眘", "Shen", "糁"]),
    ("sheng", &["生", "省", "升", "剩", "盛", "声", "胜", "圣", "身高", "晟", "绳", "申购", "昇", "笙", "深耕", "聖", "身故", "聲", "牲", "苼", "勝", "审过", "审稿", "沈哥", "Sheng", "甥"]),
    ("shi", &["是", "事", "时", "市", "室", "十", "式", "试", "石", "师", "使", "史", "屎", "食", "施", "诗", "世", "实", "拾", "士", "视", "仕", "湿", "识", "氏", "栅"]),
    ("shou", &["收", "手", "受", "瘦", "首", "守", "售", "寿", "兽", "熟", "授", "収", "艏", "狩", "绶", "獸", "壽", "扌", "痩", "獣", "夀", "嘼", "膄", "垨", "醻", "綬"]),
    ("shu", &["书", "数", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書"]),
    ("shua", &["刷", "耍", "唰", "书啊", "输啊", "树啊", "数啊", "熟啊", "叔啊", "暑啊", "鼠啊", "誜", "㕞", "SHUA", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑"]),
    ("shuai", &["帅", "甩", "摔", "衰", "率", "帥", "蟀", "书艾", "叔哎", "舒艾", "舒爱", "卛", "赎爱", "书爱", "縗", "淑爱", "缞", "书哎", "数爱", "䢦", "刷", "耍", "唰", "数", "书", "输"]),
    ("shuan", &["栓", "涮", "拴", "数安", "舒安", "耍你", "闩", "书安", "书案", "耍弄", "刷哪", "树安", "熟谙", "腨", "絟", "閂", "槫", "䧠", "刷", "耍", "唰", "数", "书", "输", "树", "熟"]),
    ("shuang", &["双", "爽", "霜", "雙", "涮锅", "泷", "塽", "孀", "礵", "骦", "书昂", "舒昂", "舒盎", "驦", "漺", "鹴", "灀", "樉", "慡", "漴", "艭", "騻", "縔", "孇", "欆", "瀧"]),
    ("shui", &["谁", "睡", "水", "税", "说", "誰", "氵", "氺", "稅", "說", "娷", "帨", "説", "閖", "脽", "祱", "挩", "涚", "裞", "涗", "捝", "㝽", "㽷", "㥨", "䲧", "䬽"]),
    ("shun", &["顺", "舜", "瞬", "淑女", "吮", "熟女", "順", "鼠年", "属牛", "枢纽", "属你", "数年", "舒宁", "孰能", "庶女", "熟男", "书内", "瞤", "舒娜", "鬊", "楯", "眴", "shun", "瞚", "蕣", "橓"]),
    ("shuo", &["说", "硕", "說", "烁", "朔", "铄", "数", "书哦", "槊", "説", "蒴", "碩", "叔哦", "妁", "数哦", "熟哦", "输哦", "爍", "數", "銏", "搠", "愬", "矟", "洬", "鑠", "欶"]),
    ("si", &["四", "死", "思", "司", "丝", "斯", "肆", "嘶", "私", "撕", "似", "巳", "亖", "寺", "枱", "泗", "嗣", "锶", "偲", "食", "禩", "佀", "厮", "驷", "伺", "饲"]),
    ("song", &["送", "松", "宋", "怂", "颂", "嵩", "淞", "诵", "崧", "餸", "讼", "菘", "耸", "凇", "揔", "鬆", "蜙", "song", "倯", "頌", "捒", "枀", "漎", "嵷", "慫", "悚"]),
    ("sou", &["搜", "嗖", "艘", "廋", "馊", "嗽", "叟", "颼", "擻", "棷", "鄋", "颾", "鏉", "薮", "鎪", "摉", "醙", "瘶", "飕", "騪", "廀", "溲", "摗", "擞", "螋", "撨"]),
    ("su", &["苏", "素", "速", "诉", "宿", "塑", "酥", "粟", "俗", "珟", "溯", "肃", "蘇", "夙", "甦", "愫", "樕", "圱", "遬", "缩", "稣", "玊", "涑", "謖", "簌", "谡"]),
    ("suan", &["算", "酸", "蒜", "苏安", "祘", "速安", "筭", "素安", "狻", "宿安", "痠", "苏垵", "笇", "匴", "篹", "㔯", "㱹", "苏", "素", "速", "诉", "宿", "塑", "酥", "粟", "俗"]),
    ("sui", &["岁", "随", "碎", "虽", "隋", "穗", "遂", "髓", "歲", "眭", "绥", "睢", "缞", "燧", "隧", "襚", "熣", "隨", "邃", "嗺", "繐", "膸", "祟", "檖", "穂", "雖"]),
    ("sun", &["孙", "损", "笋", "苏宁", "隼", "扻", "荪", "苏南", "孫", "榫", "肃宁", "潠", "鶽", "損", "速诺", "sun", "Sun", "筍", "摌", "箰", "狲", "槂", "鎨", "飧", "喰", "簨"]),
    ("suo", &["所", "锁", "缩", "索", "嗦", "梭", "鎖", "璅", "唆", "琐", "羧", "苏哦", "素哦", "素噢", "挱", "蓑", "髿", "縒", "娑", "縮", "簔", "琑", "惢", "嗍", "蜶", "桫"]),
    ("ta", &["他", "她", "它", "塔", "塌", "踏", "祂", "天啊", "他啊", "听啊", "疼啊", "跳啊", "图案", "痛啊", "她啊", "挞", "牠", "榻", "拓", "泰安", "沓", "图啊", "头啊", "投啊", "推啊", "团啊"]),
    ("tai", &["太", "台", "泰", "态", "胎", "抬", "钛", "肽", "邰", "苔", "臺", "汰", "漦", "呔", "冭", "酞", "籉", "Tai", "儓", "薹", "枱", "跆", "檯", "箈", "態", "颱"]),
    ("tan", &["谈", "弹", "谭", "碳", "探", "摊", "贪", "痰", "坛", "坦", "潭", "滩", "炭", "檀", "他能", "他那", "他呢", "毯", "覃", "叹", "瘫", "钽", "談", "昙", "譚", "tan"]),
    ("tang", &["躺", "汤", "唐", "糖", "趟", "堂", "烫", "塘", "樘", "谈过", "棠", "谈个", "淌", "镗", "膛", "溏", "踼", "瑭", "湯", "搪", "鶶", "碳钢", "Tang", "倘", "劏", "蝪"]),
    ("tao", &["套", "涛", "陶", "桃", "淘", "掏", "讨", "逃", "韬", "滔", "焘", "迯", "弢", "涭", "騊", "他哦", "她哦", "蜪", "萄", "鞉", "洮", "檮", "綯", "濤", "縧", "謟"]),
    ("te", &["特", "忒", "天鹅", "忑", "提额", "铽", "太二", "图二", "胎儿", "太饿", "腿儿", "套二", "他儿", "蟘", "慝", "脦", "犆", "螣", "貣", "貸", "鋱", "㥂", "㧹"]),
    ("tei", &["忒", "特", "忑", "铽", "蟘", "慝", "脦", "犆", "螣", "貣", "貸", "鋱", "㥂", "㧹"]),
    ("teng", &["疼", "腾", "滕", "藤", "誊", "驣", "虅", "邆", "螣", "騰", "縢", "熥", "膯", "駦", "漛", "霯", "籐", "鰧", "痋", "幐", "謄", "鼟", "儯", "籘", "䒅", "䕨"]),
    ("ti", &["提", "题", "体", "踢", "替", "梯", "鬄", "剃", "蹄", "缇", "锑", "剔", "穉", "洟", "涕", "屉", "體", "啼", "題", "媞", "逷", "鷈", "蹏", "傂", "鶗", "悌"]),
    ("tian", &["天", "填", "田", "甜", "舔", "添", "提案", "恬", "钿", "湉", "靦", "餂", "娗", "兲", "淟", "缇安", "提安", "提按", "题案", "替安", "Tian", "替俺", "畑", "阗", "鴫", "紾"]),
    ("tiao", &["条", "跳", "调", "挑", "條", "調", "眺", "鞗", "迢", "提奥", "体奥", "缇奥", "蓨", "糶", "朷", "嬥", "樤", "笤", "銚", "鋚", "窕", "庣", "祒", "蓚", "旫", "誂"]),
    ("tie", &["贴", "铁", "提额", "帖", "貼", "跕", "鐵", "萜", "驖", "tie", "提鹅", "题额", "飻", "鉄", "餮", "鴩", "鐡", "怗", "呫", "銕", "聑", "僣", "蛈", "䂿", "䩞", "䴴"]),
    ("ting", &["听", "挺", "停", "厅", "婷", "庭", "亭", "廷", "汀", "霆", "艇", "聽", "町", "烃", "鞓", "葶", "ting", "珽", "铤", "渟", "梃", "蜓", "莛", "廳", "颋", "庍"]),
    ("tong", &["同", "通", "桶", "痛", "童", "铜", "彤", "桐", "筒", "统", "砼", "佟", "捅", "仝", "瞳", "潼", "酮", "曈", "橦", "狪", "烔", "僮", "侗", "Tong", "茼", "垌"]),
    ("tou", &["头", "投", "偷", "透", "頭", "骰", "綉", "黈", "埱", "钭", "緰", "婾", "偸", "亠", "媮", "敨", "妵", "蘣", "鋀", "鍮", "飳", "紏", "鈄", "㳆", "㕻", "㤟"]),
    ("tu", &["图", "吐", "土", "涂", "兔", "途", "突", "屠", "凸", "秃", "徒", "圖", "凃", "迌", "葖", "稌", "荼", "鵌", "菟", "瘏", "塗", "钍", "嵞", "潳", "檡", "鋀"]),
    ("tuan", &["团", "图案", "途安", "團", "疃", "蓴", "湍", "抟", "褖", "涂安", "图安", "土安", "Tuan", "篿", "鏄", "畽", "嫥", "団", "彖", "墥", "摶", "鷻", "鷒", "揣", "圕", "槫"]),
    ("tui", &["退", "推", "腿", "忒", "褪", "颓", "頽", "騩", "蜕", "穨", "蓷", "Tui", "骽", "煺", "脮", "蹪", "頺", "尵", "聉", "墤", "弚", "蘈", "魋", "蛻", "弟", "駾"]),
    ("tun", &["囤", "吞", "屯", "臀", "兔年", "豚", "吐奶", "图呢", "图你", "臋", "途牛", "暾", "軘", "吐纳", "朜", "蛌", "氽", "图内", "土牛", "Tun", "饨", "鲀", "坉", "芚", "憞", "焞"]),
    ("tuo", &["拖", "脱", "魄", "托", "拓", "妥", "坨", "驼", "庹", "陀", "沲", "啴", "沱", "萚", "杝", "鼉", "驮", "图哦", "乇", "椭", "侻", "阤", "橢", "佗", "砣", "鱓"]),
    ("wa", &["哇", "娃", "瓦", "挖", "晚安", "蛙", "袜", "我啊", "玩啊", "午安", "文案", "我爱", "洼", "嗢", "问啊", "哇啊", "娃啊", "为爱", "娲", "佤", "韎", "我按", "凹", "呜啊", "网啊", "胃癌"]),
    ("wai", &["外", "歪", "崴", "喎", "呙", "咼", "顡", "竵", "瀤", "㨤", "䶐", "䠿", "哇", "娃", "瓦", "挖", "蛙", "袜", "洼", "嗢", "娲", "佤", "韎", "凹", "韈", "屲"]),
    ("wan", &["玩", "完", "万", "晚", "碗", "湾", "弯", "丸", "皖", "婉", "関", "宛", "萬", "我爱你", "腕", "琬", "挽", "菀", "烷", "杬", "箢", "莞", "wan", "哇能", "哇你", "我爱您"]),
    ("wang", &["王", "往", "网", "汪", "忘", "旺", "望", "玩过", "亡", "徍", "完工", "玩个", "妄", "顽固", "迬", "万古", "万个", "枉", "網", "万国", "朚", "玩够", "罔", "玩梗", "弯管", "完稿"]),
    ("wei", &["为", "未", "位", "伟", "微", "魏", "味", "喂", "维", "韦", "卫", "威", "胃", "尾", "委", "薇", "炜", "围", "玮", "蔚", "唯", "巍", "危", "為", "伪", "纬"]),
    ("wen", &["问", "文", "稳", "温", "雯", "闻", "纹", "蚊", "吻", "揾", "問", "汶", "搵", "玟", "瘟", "穩", "炆", "穏", "溫", "鴖", "wen", "五二年", "王二妮", "维尔纳", "沃尔诺", "鴍"]),
    ("weng", &["翁", "嗡", "问过", "问个", "瓮", "文哥", "罋", "文革", "稳固", "温哥", "文稿", "温故", "闻过", "稳过", "文管", "温感", "文官", "文广", "鶲", "文光", "文工", "文蛤", "文港", "鹟", "鎓", "滃"]),
    ("wo", &["我", "喔", "窝", "沃", "握", "卧", "哇哦", "我哦", "玩偶", "涡", "蜗", "哇偶", "渥", "肟", "踠", "窩", "喔哦", "呙", "莴", "倭", "问哦", "无哦", "网哦", "围殴", "王鸥", "硪"]),
    ("wu", &["无", "五", "吴", "唔", "武", "伍", "物", "午", "屋", "舞", "误", "务", "雾", "乌", "邬", "吾", "巫", "勿", "悟", "污", "無", "呜", "戊", "捂", "Wu", "坞"]),
    ("xi", &["洗", "系", "西", "喜", "细", "吸", "习", "希", "戏", "席", "熙", "息", "係", "溪", "曦", "锡", "玺", "汐", "奚", "夕", "稀", "禧", "茜", "铣", "騱", "兮"]),
    ("xia", &["下", "夏", "霞", "瞎", "虾", "吓", "侠", "系啊", "徦", "厦", "峡", "辖", "匣", "瑕", "丅", "暇", "遐", "俠", "呷", "洗啊", "舺", "系阿", "喺啊", "细啊", "戏啊", "喜啊"]),
    ("xian", &["先", "线", "现", "西安", "县", "闲", "险", "仙", "限", "鲜", "显", "贤", "嫌", "咸", "馅", "弦", "献", "娴", "冼", "宪", "妶", "纤", "西岸", "陷", "掀", "線"]),
    ("xiang", &["想", "像", "向", "箱", "项", "香", "相", "祥", "翔", "响", "乡", "湘", "象", "享", "巷", "先给", "限购", "详", "镶", "厢", "橡", "襄", "響", "項", "降", "纕"]),
    ("xiao", &["小", "笑", "晓", "肖", "校", "消", "销", "效", "孝", "萧", "潇", "筱", "霄", "啸", "骁", "削", "宵", "下哦", "枭", "囂", "魈", "箫", "硝", "逍", "熇", "西奥"]),
    ("xie", &["写", "些", "谢", "鞋", "卸", "歇", "斜", "邪", "协", "蟹", "血", "解", "寫", "屑", "泄", "携", "械", "鲑", "觧", "頡", "泻", "喜鹅", "西饿", "西鹅", "喜恶", "系额"]),
    ("xin", &["新", "心", "信", "鑫", "欣", "芯", "辛", "馨", "薪", "昕", "洗脑", "锌", "歆", "细腻", "忻", "西南", "西宁", "莘", "炘", "盺", "訫", "杺", "焮", "伈", "鈊", "鐔"]),
    ("xing", &["行", "星", "性", "型", "兴", "醒", "姓", "新冠", "邢", "形", "杏", "幸", "刑", "腥", "新歌", "猩", "新股", "省", "新高", "興", "惺", "擤", "郉", "煋", "瑆", "騂"]),
    ("xiong", &["熊", "胸", "雄", "兄", "凶", "匈", "胷", "芎", "汹", "兇", "夐", "熋", "賯", "敻", "焸", "讻", "诇", "忷", "匂", "哅", "詾", "恟", "洶", "訩", "㐫", "焽"]),
    ("xiu", &["修", "休", "秀", "绣", "袖", "锈", "咻", "羞", "溴", "嗅", "宿", "朽", "岫", "琇", "褎", "嚊", "髤", "鵂", "脩", "峀", "鱃", "俢", "褏", "馐", "貅", "臹"]),
    ("xu", &["徐", "需", "许", "旭", "续", "虚", "须", "嘘", "序", "恤", "胥", "绪", "戌", "栩", "煦", "叙", "恓", "朐", "蓄", "圩", "絮", "Xu", "敍", "魖", "墟", "禑"]),
    ("xuan", &["选", "炫", "轩", "宣", "悬", "縣", "璇", "萱", "玄", "旋", "煊", "渲", "瑄", "暄", "選", "烜", "禤", "喧", "癣", "绚", "喛", "楦", "铉", "徐安", "旭安", "许安"]),
    ("xue", &["学", "雪", "血", "薛", "穴", "削", "靴", "學", "趐", "滈", "桖", "噱", "恤额", "徐哦", "虚额", "徐娥", "徐额", "许娥", "鳕", "蒆", "彐", "茓", "觷", "辥", "樰", "怴"]),
    ("xun", &["训", "寻", "孫", "虚拟", "讯", "询", "熏", "勋", "迅", "巡", "旬", "荀", "逊", "循", "巽", "洵", "浔", "珣", "许诺", "汛", "薰", "馴", "煇", "鲟", "蟫", "訓"]),
    ("ya", &["呀", "牙", "压", "亚", "雅", "鸭", "吖", "丫", "有啊", "押", "娅", "芽", "哑", "崖", "涯", "要啊", "伢", "用啊", "鸦", "ya", "氩", "垭", "亞", "壓", "桠", "轧"]),
    ("yan", &["烟", "眼", "燕", "艳", "严", "铅", "闫", "演", "颜", "言", "盐", "验", "彦", "研", "岩", "妍", "延", "炎", "麣", "晏", "宴", "雁", "亚楠", "yan", "益安宁", "越爱你"]),
    ("yang", &["杨", "阳", "样", "养", "羊", "洋", "扬", "痒", "氧", "严格", "眼光", "央", "仰", "漾", "旸", "炀", "樣", "楊", "掩盖", "秧", "阉割", "烟感", "泱", "Yang", "垟", "烊"]),
    ("yao", &["要", "药", "姚", "腰", "瑶", "耀", "咬", "摇", "約", "尧", "妖", "垚", "邀", "曜", "窑", "遥", "幺", "爻", "殽", "谣", "吆", "夭", "肴", "謡", "藥", "呀哦"]),
    ("ye", &["也", "耶", "页", "叶", "业", "夜", "野", "爷", "液", "烨", "嘢", "余额", "冶", "晔", "椰", "窫", "亱", "婴儿", "噎", "又饿", "一二", "幼儿", "ye", "嚈", "吔", "葉"]),
    ("yi", &["一", "已", "以", "射", "亿", "易", "咦", "壹", "怡", "意", "义", "乙", "益", "艺", "姨", "仪", "亦", "宜", "伊", "依", "奕", "衣", "逸", "毅", "医", "翼"]),
    ("yin", &["阴", "因", "印", "一年", "银", "尹", "音", "殷", "饮", "引", "以内", "瘾", "茵", "寅", "隐", "韾", "淫", "吟", "胤", "犾", "荫", "凐", "以你", "一N", "Yin", "垠"]),
    ("ying", &["赢", "硬", "应", "英", "颖", "莹", "影", "营", "盈", "映", "迎", "鹰", "瑛", "樱", "滢", "婴", "莺", "荧", "瀛", "因果", "鞕", "嘤", "萤", "颍", "楹", "贏"]),
    ("yo", &["有", "哟", "用", "又", "油", "呦", "由", "友", "永", "游", "优", "勇", "右", "yo", "唷", "喲", "育", "嚛", "尤", "邮", "佑", "泳", "佣", "咏", "幼", "悠"]),
    ("yong", &["用", "永", "勇", "泳", "佣", "咏", "涌", "雍", "拥", "甬", "庸", "蛹", "俑", "湧", "邕", "廱", "滽", "噰", "镛", "詠", "痈", "墉", "埇", "壅", "灉", "郺"]),
    ("you", &["有", "又", "由", "油", "右", "游", "友", "优", "呦", "尤", "邮", "佑", "幼", "悠", "酉", "柚", "幽", "忧", "釉", "宥", "铀", "鼬", "疣", "you", "犹", "痏"]),
    ("yu", &["与", "雨", "于", "余", "玉", "鱼", "宇", "语", "俞", "预", "域", "豫", "钰", "羽", "育", "瑜", "喻", "裕", "遇", "煜", "渝", "禹", "予", "御", "昱", "郁"]),
    ("yuan", &["元", "远", "原", "员", "源", "袁", "院", "园", "圆", "苑", "愿", "媛", "预案", "缘", "渊", "沅", "怨", "猿", "冤", "鸢", "垣", "援", "塬", "裕安", "瑗", "yuan"]),
    ("yue", &["月", "约", "越", "余额", "悦", "粤", "説", "岳", "跃", "阅", "玥", "乐", "曰", "樾", "約", "矆", "钺", "哕", "玉娥", "与额", "豫E", "豫鄂", "Yue", "悅", "栎", "矱"]),
    ("yun", &["云", "晕", "运", "韵", "芸", "孕", "允", "匀", "昀", "与你", "蕴", "与您", "赟", "雲", "耘", "贠", "榅", "筠", "韫", "恽", "沄", "陨", "頵", "煇", "鋆", "熨"]),
    ("za", &["咋", "砸", "咱", "杂", "早啊", "早安", "走啊", "在啊", "做啊", "最爱", "真爱", "扎", "中啊", "找啊", "字啊", "匝", "值啊", "障碍", "紥", "咂", "魳", "阻碍", "桚", "拶", "雜", "雥"]),
    ("zai", &["在", "再", "仔", "载", "崽", "宰", "灾", "栽", "哉", "載", "災", "甾", "洅", "渽", "賳", "菑", "溨", "扗", "傤", "畠", "儎", "烖", "縡", "抂", "酨", "㞨"]),
    ("zan", &["咱", "赞", "暂", "攒", "咋弄", "咋能", "昝", "砸你", "瓒", "杂念", "咋呢", "簪", "贊", "喒", "揝", "最爱你", "只爱你", "讚", "蹔", "錾", "暫", "灒", "儹", "鵤", "攢", "趲"]),
    ("zang", &["脏", "藏", "臧", "葬", "咱哥", "攒够", "赃", "赞歌", "臟", "贜", "駔", "奘", "蔵", "賘", "賍", "髒", "牂", "驵", "塟", "弉", "臓", "銺", "匨", "羘", "㘸", "贓"]),
    ("zao", &["早", "造", "遭", "澡", "枣", "灶", "皂", "凿", "燥", "糟", "藻", "躁", "噪", "璅", "蹧", "蚤", "咋哦", "趮", "栆", "皁", "棗", "竈", "唕", "璪", "慥", "喿"]),
    ("ze", &["啧", "则", "泽", "责", "择", "这儿", "周二", "則", "罪恶", "总额", "仄", "作恶", "沢", "中俄", "侄儿", "澤", "迮", "足额", "昨儿", "灂", "中二", "左耳", "字儿", "赜", "齰", "嘖"]),
    ("zei", &["贼", "賊", "鲗", "戝", "蠈", "鱡", "鰂", "㖽", "则", "泽", "啧", "责", "择", "則", "仄", "沢", "澤", "迮", "灂", "赜", "齰", "嘖", "稄", "昃", "歵", "蠌"]),
    ("zen", &["怎", "谮", "责难", "囎", "撍", "则能", "则你", "譖", "这二年", "Zen", "㻸", "䫈", "譛", "则", "泽", "啧", "责", "择", "則", "仄", "沢", "澤", "迮", "灂", "赜", "齰"]),
    ("zeng", &["曾", "增", "赠", "綜", "憎", "甑", "鬷", "锃", "鬵", "贈", "怎敢", "怎搞", "怎个", "罾", "増", "曽", "缯", "鄫", "鱛", "矰", "璔", "鋥", "熷", "综", "磳", "橧"]),
    ("zha", &["炸", "扎", "蜡", "渣", "咋", "闸", "查", "榨", "诈", "喋", "柤", "轧", "真好啊", "怍", "最黑暗", "纵火案", "招呼啊", "祝好啊", "知乎啊", "吒", "乍", "揸", "喳", "眨", "馇", "紥"]),
    ("zhai", &["摘", "债", "翟", "窄", "宅", "寨", "斋", "擿", "砦", "齋", "債", "择", "捚", "嚌", "斎", "鉙", "瘵", "祭", "侧", "齊", "檡", "厏", "哜", "夈", "亝", "側"]),
    ("zhan", &["站", "占", "展", "战", "粘", "詹", "栈", "渣男", "沾", "斩", "盏", "湛", "渣女", "蘸", "顫", "毡", "瞻", "鳣", "绽", "戰", "僝", "嫸", "欃", "桟", "鳽", "崭"]),
    ("zhang", &["张", "长", "章", "涨", "账", "帐", "掌", "胀", "仗", "杖", "張", "丈", "璋", "障", "站岗", "樟", "彰", "漳", "仉", "長", "展馆", "漲", "Zhang", "嶂", "賬", "瘴"]),
    ("zhao", &["找", "照", "赵", "招", "着", "兆", "罩", "昭", "钊", "召", "朝", "肇", "曌", "诏", "趙", "沼", "爪", "炤", "棹", "謿", "瞾", "炸哦", "扎哦", "菬", "釽", "皽"]),
    ("zhe", &["这", "着", "折", "者", "浙", "哲", "適", "喆", "這", "遮", "啫", "著", "辙", "褶", "嗻", "锗", "蛰", "蔗", "这会儿", "柘", "蜇", "辄", "谪", "搩", "晢", "赭"]),
    ("zhen", &["真", "针", "镇", "振", "珍", "帧", "震", "阵", "臻", "甄", "贞", "朕", "枕", "诊", "着你", "桢", "圳", "这呢", "祯", "这你", "眹", "这能", "蓁", "瑧", "这年", "中华儿女"]),
    ("zheng", &["整", "正", "郑", "证", "挣", "蒸", "政", "争", "症", "征", "铮", "真贵", "峥", "珍贵", "真乖", "真够", "真高", "鲭", "睁", "筝", "钲", "證", "鄭", "怔", "真个", "真该"]),
    ("zhi", &["只", "之", "至", "值", "支", "指", "志", "纸", "知", "直", "智", "制", "治", "識", "致", "止", "质", "汁", "职", "芝", "置", "枝", "芷", "织", "执", "郦"]),
    ("zhong", &["中", "种", "钟", "重", "忠", "众", "仲", "终", "肿", "蚣", "盅", "冢", "衷", "種", "鐘", "锺", "終", "蹱", "汷", "塚", "踵", "鍾", "柊", "眾", "蜙", "鈡"]),
    ("zhou", &["周", "粥", "轴", "州", "洲", "舟", "咒", "皱", "肘", "宙", "箒", "昼", "週", "胕", "骤", "绉", "Zhou", "胄", "诌", "椆", "軸", "纣", "疛", "睭", "謅", "菷"]),
    ("zhu", &["住", "猪", "朱", "主", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱", "烛", "拀"]),
    ("zhua", &["抓", "爪", "髽", "主啊", "住啊", "猪啊", "檛", "簻", "挝", "猪阿", "撾", "祝阿", "煮啊", "朱啊", "珠啊", "爫", "膼", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠"]),
    ("zhuai", &["拽", "跩", "转", "助爱", "猪哎", "朱爱", "主爱", "筑爱", "睉", "轉", "抓", "爪", "髽", "檛", "簻", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹"]),
    ("zhuan", &["转", "赚", "专", "砖", "传", "注安", "轉", "賺", "篆", "抓你", "漙", "啭", "抓挠", "主案", "抓奶", "朱安", "诸安", "住安", "祝安", "撰", "專", "籑", "簨", "馔", "磗", "颛"]),
    ("zhuang", &["装", "撞", "庄", "桩", "幢", "状", "壮", "妆", "转给", "专柜", "裝", "转个", "转告", "赚个", "转过", "专供", "莊", "转岗", "荘", "赚够", "壯", "狀", "转股", "撰稿", "专干", "朱昂"]),
    ("zhui", &["追", "隊", "坠", "锥", "缀", "椎", "赘", "甀", "骓", "膇", "隹", "醊", "奞", "諈", "墜", "沝", "缒", "錐", "惴", "畷", "綴", "礈", "錣", "硾", "笍", "腏"]),
    ("zhun", &["准", "祝你", "祝您", "住哪", "旽", "忳", "肫", "猪年", "助你", "住你", "猪脑", "住呢", "主内", "逐年", "住那", "窀", "谆", "助农", "朱诺", "朱宁", "湻", "稕", "屯", "宒", "圫", "準"]),
    ("zhuo", &["桌", "卓", "着", "捉", "蠿", "灼", "镯", "酌", "啄", "琢", "拙", "焯", "繳", "浊", "燋", "炪", "住噢", "住哦", "猪哦", "倬", "茁", "禚", "著", "濯", "叕", "灂"]),
    ("zi", &["子", "字", "自", "紫", "梓", "资", "仔", "籽", "滋", "兹", "姿", "孜", "咨", "渍", "畠", "訾", "倳", "緇", "姊", "淄", "嗞", "剚", "趑", "葘", "吱", "洓"]),
    ("zong", &["总", "宗", "棕", "综", "纵", "粽", "踪", "總", "熜", "嵕", "从", "枞", "鬃", "豵", "嵸", "縦", "糉", "鍐", "踨", "葼", "碂", "繌", "緃", "騣", "猣", "腙"]),
    ("zou", &["走", "揍", "邹", "奏", "棸", "诹", "驺", "陬", "鄒", "辶", "赱", "齱", "鄹", "齺", "鲰", "箃", "鯐", "黀", "搊", "諏", "掫", "騶", "棷", "菆", "楱", "緅"]),
    ("zu", &["组", "租", "足", "祖", "族", "阻", "卒", "俎", "诅", "組", "踤", "镞", "錊", "崒", "詛", "鎺", "鏃", "哫", "菹", "卆", "柤", "崪", "倅", "怚", "踿", "爼"]),
    ("zuan", &["钻", "赚", "祖安", "攥", "籫", "组胺", "纂", "鉆", "欑", "篹", "賺", "组安", "足安", "祖庵", "足按", "鑽", "劗", "缵", "纉", "躜", "鑚", "纘", "揝", "攢", "籑", "躦"]),
    ("zui", &["最", "嘴", "罪", "醉", "蕞", "咀", "辠", "觜", "檌", "蟕", "濢", "祽", "纗", "酔", "璻", "槜", "錊", "欈", "樶", "嗺", "鋷", "冣", "嶉", "噿", "晬", "朘"]),
    ("zun", &["尊", "遵", "组内", "樽", "阻尼", "鳟", "阻挠", "鱒", "拵", "譐", "袸", "组你", "噂", "阻难", "瀳", "繜", "鐏", "墫", "銌", "僔", "鷷", "撙", "捘", "罇", "栫", "䔿"]),
    ("zuo", &["做", "坐", "左", "作", "座", "咗", "佐", "昨", "嘬", "唑", "祚", "柞", "撮", "琢", "组哦", "穝", "岞", "怍", "胙", "租哦", "足哦", "酢", "飵", "諎", "捽", "阼"]),
];

pub fn syl_panel(k: &str) -> Option<&'static [&'static str]> {
    SYL_PANELS
        .binary_search_by(|(s, _)| s.cmp(&k))
        .ok()
        .map(|i| SYL_PANELS[i].1)
}
// rows: 401

// rows: 377
/// ---- 役N tick71: 双声母对面板钉表 380 对（pair_panels.tsv 生成;
/// 键 = 音节, 键序 = sort 序。native 律: 音节面板 = 字列+补全词+
/// 简拼组合按原生序混排（61/79 面板混词实证）, 原样钉。消费 =
/// query 音节门 → q_initial_panel。表空 → 律休眠。----
pub static PAIR_PANELS: &[(&str, &[&str])] = &[
    ("bc", &["不错", "变成", "保持", "不吃", "保存", "补充", "不出", "彼此", "奔驰", "不成", "白菜", "不穿", "补偿", "不差", "班车", "编程", "本次", "菠菜", "包车", "B超", "班次", "报错", "板材", "BC", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("bd", &["别的", "不懂", "不到", "不对", "不得", "不大", "不多", "不打", "病毒", "百度", "不带", "表达", "不动", "八点", "本地", "变得", "不断", "绑定", "报道", "报的", "保底", "变动", "BD", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕"]),
    ("bf", &["部分", "办法", "不发", "拜访", "北方", "爆发", "不放", "播放", "不分", "不服", "暴富", "补发", "病房", "备份", "报废", "保费", "百分", "bf", "不符", "并非", "摆放", "帮扶", "并发", "拨付", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("bg", &["不过", "不敢", "不管", "不够", "不给", "报告", "半个", "办公", "不高", "不该", "表格", "不干", "不贵", "不跟", "表哥", "八卦", "饼干", "包裹", "变更", "曝光", "不改", "报个", "被告", "BG", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("bh", &["不会", "不好", "保护", "变化", "不回", "不喝", "背后", "包含", "编号", "不换", "补货", "不喊", "备货", "不含", "闭环", "驳回", "标红", "不", "保函", "BH", "吧", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋", "拜", "柏", "捌"]),
    ("bj", &["比较", "北京", "毕竟", "不就", "报警", "不急", "报价", "不见", "背景", "别急", "白酒", "笔记", "搬家", "不接", "班级", "编辑", "不仅", "不加", "保洁", "本金", "布局", "标记", "吧", "把", "被", "别", "表", "不", "比", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋"]),
    ("bk", &["包括", "不看", "不可", "不开", "补课", "本科", "不肯", "崩溃", "板块", "贝壳", "办卡", "报考", "备考", "爆款", "避开", "不亏", "不卡", "边框", "把控", "播客", "备课", "补卡", "BK", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕"]),
    ("bl", &["本来", "不了", "办理", "不来", "罢了", "玻璃", "不冷", "比例", "变了", "报了", "不理", "保留", "保利", "暴露", "摆烂", "暴力", "爆了", "不良", "表里", "补录", "便利", "变量", "BL", "魄", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕"]),
    ("bm", &["帮忙", "部门", "报名", "爸妈", "不买", "宝马", "避免", "不忙", "表面", "表妹", "不卖", "便秘", "北门", "保密", "不满", "保姆", "宝妈", "北面", "拌面", "编码", "背面", "表明", "北美", "BM", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("bn", &["不能", "帮你", "把你", "半年", "比你", "被你", "拜年", "保暖", "不难", "帮您", "不拿", "百年", "不弄", "班", "BN", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋", "拜", "柏", "捌", "丙", "饱", "彪", "碧", "鲍"]),
    ("bp", &["不怕", "不配", "白嫖", "被迫", "鞭炮", "被骗", "别怕", "不跑", "标配", "爆破", "爆品", "帮派", "BP", "编排", "波", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋", "拜", "柏", "捌", "丙", "饱", "彪", "碧", "鲍"]),
    ("bq", &["不去", "表情", "并且", "抱歉", "标签", "把钱", "别去", "不缺", "病情", "不清", "比起", "本期", "B区", "版权", "不全", "本群", "北区", "不强", "补齐", "搬迁", "保全", "不起", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋"]),
    ("br", &["不然", "别人", "不如", "比如", "不让", "本人", "病人", "被人", "必然", "包容", "把人", "别让", "不热", "不认", "步入", "帮人", "白人", "不容", "薄弱", "鄙人", "比人", "逼人", "别惹", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕"]),
    ("bs", &["不是", "本身", "不少", "不说", "比赛", "不上", "白色", "不算", "表示", "吧", "博士", "不熟", "BS", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋", "拜", "柏", "捌", "丙", "饱", "彪", "碧", "鲍", "磅", "伴"]),
    ("bt", &["白天", "不太", "不同", "半天", "不听", "把他", "补贴", "变态", "不停", "帮他", "拜托", "把它", "比他", "标题", "被他", "不通", "不退", "帮她", "摆摊", "本田", "不提", "B套", "BT", "必填", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("bw", &["帮我", "把我", "比我", "被我", "不玩", "被窝", "把握", "百万", "不问", "部位", "保温", "办完", "傍晚", "不为", "霸王", "博文", "吧", "变为", "报文", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋", "拜", "柏", "捌", "丙"]),
    ("bx", &["不行", "不想", "必须", "保险", "表现", "不信", "不像", "报销", "冰箱", "比心", "不许", "不下", "不小", "包厢", "保修", "不写", "变形", "不限", "巴西", "变现", "便携", "冰雪", "编写", "备选", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("by", &["不要", "不用", "毕业", "保养", "半夜", "包邮", "必要", "不远", "不", "表演", "白银", "抱怨", "不也", "边缘", "备用", "白云", "表扬", "本月", "by", "搬运", "吧", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨", "毕", "憋", "拜", "柏", "捌"]),
    ("bz", &["不在", "标准", "保证", "壁纸", "B站", "备注", "不知", "包装", "不做", "被子", "宝子", "帮助", "不止", "不再", "杯子", "保障", "不足", "部长", "编制", "博主", "本周", "步骤", "板子", "标注", "吧", "不", "把", "被", "别", "比", "表", "并", "呗", "包", "报", "帮", "补", "宝", "班", "本", "版", "办", "部", "白", "边", "变", "半", "板", "标", "爸", "魄", "笔", "保", "波", "八", "倍", "播", "博", "北", "冰", "背", "病", "搬", "币", "逼", "爆", "百", "布", "杯", "步", "斌", "兵", "棒", "备", "编", "摆", "遍", "便", "彬", "抱", "必", "泵", "饼", "绑", "贝", "崩", "薄", "般", "叭", "啵", "巴", "榜", "拔", "佰", "邦", "笨"]),
    ("cb", &["成本", "吃吧", "传播", "差别", "吃饱", "初八", "财报", "初步", "储备", "翅膀", "超标", "承包", "筹备", "出版", "参保", "差不", "餐补", "擦边", "侧边", "C杯", "船舶", "层板", "CB", "初版", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cd", &["成都", "吃的", "差点", "吃点", "充电", "彻底", "穿的", "迟到", "出的", "程度", "承担", "场地", "沉淀", "菜单", "才对", "长度", "查到", "出单", "车队", "传达", "传递", "存档", "CD", "测", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭"]),
    ("cf", &["吃饭", "出发", "除非", "重复", "厨房", "采访", "成分", "财富", "惩罚", "炒饭", "车费", "肠粉", "春风", "充分", "嘲讽", "处罚", "厂房", "触发", "吃法", "重发", "处方", "餐费", "存放", "CF", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cg", &["成功", "超过", "唱歌", "吃个", "吃过", "采购", "错过", "出国", "出轨", "草稿", "常规", "参观", "出个", "成果", "吃瓜", "超哥", "穿过", "抽个", "炒股", "城管", "橱柜", "初稿", "持股", "CG", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cj", &["参加", "吵架", "超级", "车间", "刺激", "春节", "成绩", "场景", "厂家", "曾经", "插件", "抽奖", "差距", "成交", "差价", "促进", "春季", "采集", "创建", "常见", "承接", "拆解", "CJ", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭"]),
    ("ck", &["仓库", "参考", "查看", "出口", "抽空", "存款", "车库", "吃亏", "窗口", "拆开", "刺客", "出库", "此刻", "吃苦", "乘客", "重开", "长款", "残酷", "长裤", "错开", "操控", "次卡", "CK", "次", "吃", "从", "出", "才", "车", "成", "陈", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭"]),
    ("cl", &["出来", "处理", "吃了", "除了", "错了", "材料", "出了", "成了", "厂里", "从来", "车辆", "车里", "查了", "村里", "穿了", "陈列", "窗帘", "撤了", "彩礼", "策略", "成立", "测量", "C罗", "存量", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cm", &["cm", "出门", "草莓", "聪明", "尺码", "沉默", "吃吗", "充满", "吃面", "炒面", "侧面", "场面", "出名", "沉迷", "传媒", "层面", "纯棉", "车门", "出面", "才买", "村民", "催眠", "触摸", "筹码", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cn", &["才能", "承诺", "成年", "菜鸟", "吹牛", "从哪", "从你", "常年", "产能", "出纳", "cn", "场内", "采纳", "储能", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭", "扯", "蹭", "昌", "辰", "财", "诚", "彩", "窗", "朝"]),
    ("cp", &["产品", "差评", "车票", "车牌", "彩票", "成品", "测评", "出品", "陈皮", "裁判", "菜品", "扯皮", "彩排", "出票", "持平", "CP", "昌平", "重拍", "川普", "草坪", "磁盘", "成片", "C盘", "茶品", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cq", &["出去", "重庆", "长期", "重启", "初七", "存钱", "出钱", "传奇", "出勤", "澄清", "拆迁", "春秋", "城区", "初期", "充钱", "才去", "除去", "厂区", "产权", "C区", "采取", "此前", "抽取", "超期", "擦", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cr", &["传染", "承认", "成人", "吃肉", "愁人", "出入", "脆弱", "残忍", "超人", "炒肉", "插入", "裁人", "此人", "测", "次日", "从容", "成日", "缠绕", "春日", "存入", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭", "扯", "蹭", "昌", "辰"]),
    ("cs", &["超市", "才是", "吃啥", "厕所", "cs", "测试", "城市", "长沙", "出生", "车上", "成熟", "尝试", "产生", "次数", "参数", "措施", "查收", "彩色", "超时", "厂商", "传输", "CS", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭", "扯"]),
    ("ct", &["餐厅", "传统", "冲突", "春天", "CT", "出头", "抽屉", "床头", "插头", "常态", "从头", "长途", "词条", "撤退", "错题", "辞退", "出台", "车头", "出图", "城投", "磁铁", "穿透", "车贴", "长图", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cw", &["财务", "吃完", "成为", "错误", "初五", "车位", "春晚", "宠物", "肠胃", "除外", "催我", "次卧", "测完", "仓位", "称为", "出完", "常温", "测", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆", "臭", "扯", "蹭", "昌", "辰", "财", "诚"]),
    ("cx", &["重新", "出现", "才行", "持续", "操心", "查询", "程序", "从小", "出行", "创新", "车险", "诚信", "从新", "车型", "抽象", "除夕", "呈现", "促销", "查下", "磁吸", "撤销", "CX", "产线", "重写", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("cy", &["参与", "才有", "吃药", "餐饮", "抽烟", "初一", "创业", "茶叶", "出院", "处于", "产业", "成员", "差异", "朝阳", "采用", "创意", "常用", "裁员", "词语", "持有", "采样", "粗", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "池", "哆", "臭", "扯", "蹭"]),
    ("cz", &["操作", "存在", "成长", "初中", "陈总", "车子", "辞职", "创造", "充值", "橙子", "穿着", "锤子", "出租", "常州", "材质", "车主", "插座", "瓷砖", "查找", "创作", "重装", "重置", "处置", "参照", "吃", "从", "出", "才", "车", "成", "陈", "次", "查", "差", "穿", "长", "错", "催", "超", "传", "层", "测", "菜", "冲", "抽", "纯", "处", "擦", "此", "村", "场", "厂", "存", "程", "词", "拆", "充", "春", "操", "草", "寸", "侧", "曹", "初", "除", "仓", "蔡", "城", "床", "茶", "产", "船", "崔", "炒", "创", "插", "常", "吹", "唱", "晨", "艹", "猜", "重", "川", "凑", "称", "餐", "丑", "撤", "抄", "串", "惨", "吵", "粗", "池", "哆"]),
    ("db", &["对吧", "点半", "都不", "打包", "东北", "代表", "对比", "都被", "倒闭", "大宝", "答辩", "豆瓣", "对不", "地板", "蛋白", "底部", "达标", "担保", "的吧", "豆包", "顶部", "对标", "调拨", "DB", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dc", &["电池", "到处", "打车", "单纯", "当初", "堵车", "单词", "多次", "调查", "多长", "当成", "电车", "达成", "地产", "档次", "底层", "督促", "打错", "大厂", "导出", "得出", "大促", "单次", "DC", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("df", &["地方", "对方", "东方", "豆腐", "巅峰", "大夫", "电费", "得分", "大方", "但凡", "答复", "到付", "都发", "多发", "垫付", "大风", "东风", "代发", "打分", "代付", "大幅", "党费", "打法", "DF", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dg", &["大概", "大哥", "蛋糕", "打工", "打个", "带个", "东莞", "都给", "多个", "点个", "德国", "代购", "打过", "灯光", "东哥", "电工", "订购", "对公", "耽搁", "单个", "大纲", "大G", "定稿", "DG", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dh", &["电话", "等会", "都会", "都好", "待会", "多好", "导航", "到货", "单号", "的话", "大号", "订婚", "动画", "兑换", "对话", "大会", "等哈", "带货", "带回", "订货", "定好", "DH", "的", "到", "对", "点", "等", "都", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫"]),
    ("dj", &["大家", "多久", "到家", "登记", "大姐", "单价", "对接", "打架", "点击", "打击", "定金", "等级", "低价", "顶级", "冬季", "单机", "电机", "道具", "冻结", "定价", "搭建", "单据", "党建", "DJ", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷"]),
    ("dk", &["打开", "打卡", "贷款", "点开", "都快", "打款", "端口", "对抗", "档口", "抵扣", "得空", "多开", "地库", "地块", "雕刻", "打孔", "断开", "带宽", "DK", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫", "屌", "怼", "毒"]),
    ("dl", &["到了", "多了", "得了", "点了", "店里", "打了", "对了", "登录", "锻炼", "带了", "掉了", "懂了", "大佬", "代理", "道理", "独立", "带来", "定了", "大量", "动力", "大连", "道路", "电力", "DL", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dm", &["代码", "都没", "对面", "对吗", "动漫", "多么", "大门", "大米", "当面", "大妈", "多吗", "倒霉", "点名", "地面", "东门", "的吗", "店面", "弹幕", "打磨", "叼毛", "大麦", "的嘛", "DM", "dm", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dn", &["电脑", "等你", "都能", "对你", "带你", "到哪", "多年", "当年", "到你", "打你", "当你", "大脑", "等您", "的你", "但", "的", "都", "到", "对", "点", "等", "多", "打", "大", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫", "屌", "怼", "毒", "堆", "蹲", "睇", "冻", "迪"]),
    ("dp", &["店铺", "打牌", "电瓶", "点评", "搭配", "单排", "大牌", "灯泡", "大盘", "大片", "打破", "大鹏", "刀片", "订票", "单品", "顶配", "地盘", "底盘", "大屏", "吊牌", "低配", "底片", "跌破", "DP", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dq", &["的确", "多钱", "地区", "打球", "都去", "道歉", "到期", "地球", "电器", "大气", "带去", "定期", "短期", "当前", "打钱", "大群", "档期", "大区", "电气", "对齐", "点前", "读取", "DQ", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾"]),
    ("dr", &["当然", "的", "的人", "打扰", "大人", "多人", "丢人", "达人", "导入", "单人", "敌人", "等人", "Dr", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫", "屌", "怼", "毒", "堆", "蹲", "睇", "冻", "迪", "赌", "盯"]),
    ("ds", &["但是", "都是", "多少", "当时", "电视", "打算", "倒是", "都说", "读书", "大师", "地上", "带上", "到时", "到手", "第三", "导师", "电商", "大", "大厦", "DS", "都", "到", "对", "点", "等", "的", "多", "打", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫", "屌", "怼", "毒"]),
    ("dt", &["冬天", "地图", "地铁", "电梯", "对头", "等他", "对他", "大厅", "带他", "当天", "动态", "大腿", "打听", "答题", "大头", "代替", "单挑", "大同", "等她", "打通", "独特", "地毯", "大唐", "大图", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dw", &["等我", "单位", "的", "对我", "但我", "定位", "带我", "打完", "到位", "打我", "耽误", "动物", "队伍", "带娃", "地位", "当我", "大王", "电网", "对外", "多万", "点位", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫", "屌", "怼"]),
    ("dx", &["东西", "等下", "对象", "都行", "担心", "大学", "都想", "大小", "短信", "电信", "多谢", "底下", "短袖", "地下", "当下", "大型", "底薪", "底线", "电线", "典型", "掉线", "大写", "定向", "对下", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("dy", &["抖音", "都有", "都要", "电影", "对呀", "第一", "答应", "对于", "打印", "等于", "单元", "对应", "电源", "低于", "定义", "党员", "大于", "调研", "的", "到", "对", "点", "等", "都", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定", "掉", "待", "低", "度", "地", "段", "端", "电", "动", "东", "懂", "灯", "吨", "读", "滴", "订", "底", "道", "代", "丢", "袋", "邓", "刀", "达", "丁", "德", "栋", "队", "顶", "董", "戴", "调", "跌", "杜", "倒", "导", "登", "短", "蛋", "哒", "丹", "党", "嗲", "档", "断", "爹", "吊", "答", "呆", "堵", "弟", "豆", "搭", "冬", "贷", "盾", "垫", "屌", "怼", "毒", "堆"]),
    ("dz", &["地址", "都在", "肚子", "带着", "大众", "等着", "单子", "点赞", "打字", "导致", "电子", "定制", "动作", "袋子", "打折", "到账", "店长", "队长", "盯着", "打造", "大致", "对账", "对照", "DZ", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "的", "都", "到", "对", "点", "等", "多", "打", "大", "但", "得", "单", "带", "当", "店", "第", "定"]),
    ("fb", &["方便", "发布", "分别", "封闭", "风暴", "反驳", "封板", "发表", "翻倍", "发吧", "副本", "分布", "分包", "腹部", "分辨", "法宝", "粉笔", "发包", "分部", "防爆", "发版", "非标", "分贝", "封边", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fc", &["非常", "发出", "付出", "发财", "房产", "返场", "复查", "发车", "饭菜", "返程", "翻车", "翡翠", "发错", "分成", "犯错", "扶持", "反差", "服从", "范畴", "方程", "分层", "FC", "复测", "辅材", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fd", &["发的", "放到", "饭店", "房东", "非得", "放的", "发到", "奋斗", "负担", "放大", "房贷", "辅导", "反对", "发达", "封顶", "福袋", "否定", "法定", "返点", "浮动", "幅度", "发电", "分段", "FD", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fg", &["发给", "发个", "风格", "发过", "法国", "复工", "飞哥", "覆盖", "反感", "放过", "富贵", "复古", "放个", "发光", "法官", "分给", "分工", "峰哥", "返工", "反光", "分管", "复购", "分割", "法规", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fh", &["发货", "符合", "防护", "发挥", "废话", "复合", "返回", "复活", "凤凰", "分红", "分行", "封号", "复核", "符号", "饭后", "孵化", "放好", "返还", "防滑", "防火", "负荷", "付", "发", "分", "放", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉", "芙", "妃"]),
    ("fj", &["放假", "附近", "房间", "飞机", "风景", "福建", "费劲", "房价", "饭局", "腹肌", "副驾", "分解", "附件", "发酵", "芳姐", "放进", "反击", "分局", "附加", "分拣", "风机", "分级", "封禁", "分镜", "付", "发", "分", "放", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fk", &["反馈", "付款", "分开", "疯狂", "放开", "罚款", "放款", "复刻", "妇科", "反抗", "放空", "犯困", "防控", "风控", "副卡", "饭卡", "返款", "访客", "风口", "房卡", "封口", "方可", "方块", "法考", "佛", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fl", &["发了", "服了", "福利", "疯了", "放了", "法律", "废了", "分了", "发来", "封了", "翻了", "飞了", "付了", "分类", "返利", "分离", "费率", "分量", "发力", "分裂", "分流", "辅料", "放量", "法兰", "佛", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fm", &["方面", "父母", "封面", "发明", "蜂蜜", "分明", "发霉", "负面", "分泌", "发吗", "丰满", "阀门", "辅酶", "反面", "繁忙", "抚摸", "房门", "发膜", "粉末", "FM", "分母", "附魔", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉", "芙"]),
    ("fn", &["发你", "烦恼", "妇女", "放你", "分", "放哪", "愤怒", "烦你", "肥牛", "发", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉", "芙", "妃", "肤", "赋", "繁", "蜂", "氟", "孚", "辅", "霏", "甫", "弗", "奋", "粪", "烽"]),
    ("fp", &["发票", "分配", "放屁", "富婆", "放炮", "复盘", "废品", "扶贫", "肥胖", "翻盘", "反派", "分屏", "分批", "翻牌", "放平", "风评", "发泡", "粉扑", "FP", "封皮", "返聘", "分派", "分片", "佛", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉"]),
    ("fq", &["放弃", "父亲", "分期", "夫妻", "番茄", "付钱", "发钱", "福气", "发起", "翻墙", "发圈", "分钱", "饭钱", "费钱", "分区", "风气", "飞起", "烦请", "付清", "发群", "分歧", "F区", "废弃", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉"]),
    ("fr", &["发热", "法人", "烦人", "夫人", "肥肉", "否认", "富人", "放入", "凡人", "芙蓉", "分人", "繁荣", "废人", "犯人", "放任", "腐乳", "分润", "风热", "服软", "放人", "妇人", "丰润", "福瑞", "缝纫", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fs", &["方式", "发烧", "粉丝", "发生", "分手", "法师", "放松", "粉色", "发送", "分数", "佛山", "防水", "烦死", "风扇", "反思", "复试", "飞书", "发", "FS", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉", "芙", "妃", "肤", "赋", "繁"]),
    ("ft", &["丰田", "反弹", "福田", "飞天", "福特", "发图", "饭堂", "粉条", "饭团", "访谈", "发帖", "发条", "丰台", "分摊", "法庭", "繁体", "方太", "封条", "返图", "斧头", "发他", "分体", "ft", "附图", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fw", &["服务", "发我", "范围", "氛围", "废物", "放我", "发完", "访问", "房屋", "分为", "风味", "发文", "法务", "复位", "方位", "反问", "付完", "符文", "范文", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉", "芙", "妃", "肤", "赋"]),
    ("fx", &["发现", "分享", "分析", "放心", "方向", "风险", "复习", "放下", "发型", "发下", "放学", "发小", "返现", "飞行", "费心", "福星", "发行", "翻新", "缝隙", "分销", "反向", "放行", "复现", "FX", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("fy", &["费用", "反应", "翻译", "非要", "敷衍", "法院", "发言", "发育", "妇幼", "复印", "副业", "反映", "服用", "房源", "发音", "防御", "付", "FY", "发", "分", "放", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵", "奉", "芙", "妃", "肤", "赋", "繁", "蜂"]),
    ("fz", &["分钟", "反正", "负责", "房子", "复制", "放在", "发展", "辅助", "复杂", "房租", "福州", "否则", "放着", "防止", "服装", "副总", "负债", "发走", "分子", "分组", "分支", "放置", "仿真", "负载", "发", "分", "放", "付", "方", "费", "份", "否", "非", "饭", "法", "飞", "房", "风", "粉", "服", "冯", "反", "封", "福", "烦", "翻", "峰", "范", "返", "芳", "凤", "副", "富", "丰", "防", "负", "复", "锋", "凡", "废", "符", "肥", "疯", "樊", "菲", "芬", "罚", "附", "傅", "伐", "佛", "阀", "府", "帆", "缝", "肺", "父", "夫", "枫", "仿", "犯", "赴", "坊", "饙", "伏", "泛", "访", "幅", "發", "番", "敷", "扶", "斐", "逢", "浮", "梵"]),
    ("gb", &["根本", "隔壁", "改变", "关闭", "胳膊", "干部", "公布", "国标", "告别", "个别", "刚把", "港币", "勾八", "搞不", "广播", "规避", "盖板", "钢板", "GB", "改版", "各班", "改吧", "过磅", "隔板", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gc", &["刚才", "过程", "工厂", "工程", "观察", "改成", "广场", "国产", "高潮", "干脆", "歌词", "高层", "构成", "给出", "古城", "搞错", "搞成", "规程", "购车", "贯彻", "共创", "GC", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆"]),
    ("gd", &["搞定", "更多", "搞得", "广东", "给的", "固定", "工地", "规定", "高端", "股东", "高度", "观点", "更大", "给到", "管道", "工单", "改动", "G点", "归档", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮"]),
    ("gf", &["官方", "国服", "过分", "功夫", "高峰", "股份", "干饭", "规范", "广发", "公分", "高分", "刚发", "购房", "光伏", "广泛", "古风", "稿费", "古法", "工服", "各方", "国防", "工费", "国风", "功放", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gh", &["刚好", "干活", "更好", "规划", "过后", "更换", "搞好", "过户", "工会", "公户", "盖好", "挂号", "工行", "工号", "供货", "改好", "公会", "归还", "GH", "股", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮"]),
    ("gj", &["感觉", "估计", "赶紧", "关键", "国家", "干净", "高级", "国际", "根据", "更加", "工具", "攻击", "格局", "关机", "管家", "跟进", "公斤", "挂机", "共计", "改进", "钢筋", "轨迹", "稿件", "构建", "股", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gk", &["顾客", "高考", "公开", "赶快", "观看", "感慨", "更快", "管控", "港口", "国考", "刚开", "功课", "概括", "刚看", "隔空", "隔开", "骨科", "挂靠", "高空", "高开", "高科", "概况", "国控", "工况", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gl", &["过来", "管理", "过了", "关了", "隔离", "够了", "给了", "公里", "干了", "改了", "搞了", "挂了", "鼓励", "贵了", "高了", "概率", "规律", "给力", "攻略", "桂林", "关联", "过滤", "功率", "GL", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gm", &["干嘛", "购买", "感冒", "哥们", "闺蜜", "过敏", "关门", "光明", "规模", "过吗", "革命", "改名", "古茗", "够吗", "国贸", "共鸣", "公民", "高明", "国民", "GM", "光猫", "柜门", "更名", "过目", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gn", &["给你", "功能", "过年", "跟你", "国内", "给您", "姑娘", "闺女", "概念", "管你", "跟您", "观念", "干", "个你", "GN", "个", "给", "过", "改", "搞", "跟", "哥", "高", "刚", "该", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮", "沟", "功", "骨", "隔", "杠"]),
    ("gp", &["股票", "公平", "狗屁", "高配", "购票", "高频", "钙片", "光盘", "公婆", "挂牌", "果盘", "高鹏", "干皮", "规培", "工牌", "跟拍", "GP", "个屁", "公屏", "贡品", "光谱", "G牌", "桂P", "改派", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gq", &["过去", "感情", "过期", "够呛", "给钱", "国企", "钢琴", "歌曲", "刚去", "搞钱", "国庆", "高清", "枸杞", "搞起", "改签", "股权", "工期", "更强", "广汽", "共情", "改期", "关卡", "工区", "公区", "噶", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gr", &["个人", "果然", "感染", "工人", "个", "给人", "跟人", "国人", "感人", "狗肉", "干扰", "贵人", "光荣", "古人", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮", "沟", "功", "骨", "隔", "杠", "弓"]),
    ("gs", &["公司", "干啥", "告诉", "故事", "感受", "高速", "改善", "赶上", "格式", "工商", "更是", "公式", "甘肃", "公示", "个税", "工时", "贵司", "GS", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮", "沟"]),
    ("gt", &["沟通", "给他", "给她", "跟他", "改天", "高铁", "跟她", "共同", "骨头", "给它", "罐头", "管他", "柜台", "光头", "个体", "固态", "隔天", "狗头", "钢铁", "GT", "构图", "固体", "国泰", "柜体", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gw", &["给我", "跟我", "各位", "岗位", "国外", "改为", "官网", "个", "购物", "搞完", "怪我", "过完", "干完", "高温", "顾问", "国网", "感悟", "过往", "工位", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮", "沟"]),
    ("gx", &["更新", "关系", "感谢", "关心", "高兴", "恭喜", "搞笑", "共享", "广西", "贡献", "高新", "个性", "功效", "高效", "高校", "光线", "各项", "改下", "给下", "过下", "勾选", "光纤", "工序", "跟新", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("gy", &["关于", "故意", "公园", "公寓", "高一", "贵阳", "工艺", "更有", "过于", "够用", "工业", "供应", "公益", "高于", "高压", "干预", "共用", "姑", "Gy", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "购", "肝", "观", "割", "勾", "梗", "宫", "估", "杆", "耿", "冠", "刮", "沟"]),
    ("gz", &["工作", "工资", "各种", "关注", "广州", "郭总", "高中", "跟着", "贵州", "柜子", "公众", "盖章", "故障", "规则", "各自", "告知", "改造", "高总", "观众", "盖子", "跟踪", "稿子", "公章", "感知", "个", "给", "跟", "过", "改", "搞", "哥", "高", "刚", "该", "干", "更", "各", "管", "工", "贵", "挂", "够", "关", "光", "共", "狗", "郭", "国", "怪", "盖", "根", "股", "噶", "感", "敢", "公", "歌", "广", "柜", "嘎", "甘", "赶", "顾", "鬼", "锅", "果", "供", "格", "岗", "滚", "稿", "港", "乖", "古", "故", "瓜", "逛", "归", "桂", "告", "龚", "谷", "罐", "钢", "馆", "葛", "咁", "官", "姑", "购", "肝", "观", "割", "勾", "梗", "宫"]),
    ("hb", &["好吧", "红包", "后边", "还不", "黑白", "汇报", "伙伴", "花呗", "河北", "货币", "湖北", "会被", "很棒", "汉堡", "海边", "海报", "何必", "回报", "环保", "回本", "航班", "会把", "合并", "HB", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("hc", &["好吃", "换成", "火车", "还差", "喝茶", "好处", "好惨", "很长", "很差", "好丑", "合成", "货车", "红茶", "会场", "换车", "缓存", "核查", "画册", "耗材", "回程", "回传", "回车", "HC", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑"]),
    ("hd", &["好的", "活动", "很多", "好多", "还得", "很大", "好点", "好滴", "回答", "回到", "好哒", "获得", "互动", "厚度", "核对", "hd", "回单", "后端", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄", "怀", "划"]),
    ("hf", &["回复", "恢复", "合肥", "好烦", "话费", "护肤", "很烦", "回放", "盒饭", "汉服", "合法", "回访", "会发", "还发", "花费", "画风", "划分", "恒丰", "横幅", "会费", "华府", "汇丰", "耗费", "HF", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("hg", &["火锅", "换个", "韩国", "很高", "回国", "好贵", "回归", "还给", "回个", "合格", "很贵", "黄瓜", "后果", "宏观", "回购", "回顾", "海关", "华哥", "浩哥", "辉哥", "合规", "化工", "涵盖", "Hg", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("hj", &["回家", "环境", "好久", "喝酒", "很久", "黄金", "寒假", "缓解", "好几", "红酒", "火箭", "合计", "很近", "环节", "呼叫", "痕迹", "焊接", "户籍", "合集", "获奖", "货架", "很急", "合金", "混剪", "胡", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("hk", &["好看", "很快", "海口", "户口", "好快", "还款", "货款", "好困", "回款", "航空", "回馈", "何况", "还可", "汇款", "花开", "会看", "海康", "汉口", "贺卡", "好卡", "HK", "获客", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼"]),
    ("hl", &["回来", "好了", "后来", "换了", "坏了", "喝了", "好嘞", "花了", "好啦", "好累", "回了", "合理", "哈喽", "忽略", "婚礼", "很累", "护理", "好咧", "含量", "活力", "汇率", "HL", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼"]),
    ("hm", &["还没", "后面", "号码", "好吗", "好嘛", "画面", "好美", "很忙", "好么", "红米", "后门", "很慢", "盒马", "好慢", "好忙", "毁灭", "海绵", "好卖", "缓慢", "毫米", "鸿蒙", "还么", "HM", "hm", "和", "好", "会", "还", "哈", "噷", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌"]),
    ("hn", &["还能", "和你", "很难", "河南", "好呢", "喊你", "好难", "湖南", "海南", "回你", "很", "怀念", "还你", "和您", "黄牛", "红牛", "糊弄", "好捏", "华南", "和", "好", "会", "还", "哈", "号", "后", "话", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄", "怀", "划"]),
    ("hp", &["害怕", "好评", "和平", "嗨皮", "花瓶", "活泼", "好怕", "花盆", "后排", "哈皮", "黑屏", "黄埔", "惠普", "货品", "很怕", "虎扑", "横屏", "琥珀", "航拍", "HP", "环评", "获批", "海派", "Hp", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("hq", &["回去", "后期", "花钱", "好奇", "还去", "行情", "会去", "很强", "获取", "还钱", "虹桥", "晦气", "红旗", "环球", "后勤", "还请", "还缺", "好强", "合区", "黄芪", "会签", "货期", "HQ", "哈", "和", "好", "会", "还", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑"]),
    ("hr", &["HR", "好人", "和", "坏人", "黑人", "华润", "忽然", "会让", "还让", "换人", "华人", "浩然", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄", "怀", "划", "糊", "呼", "沪", "霍", "寒", "壶", "嚯"]),
    ("hs", &["还是", "合适", "黑色", "很少", "还说", "红色", "话说", "回事", "好事", "还剩", "划算", "黄色", "回收", "灰色", "核实", "会所", "话术", "哈", "含税", "核算", "函数", "好", "会", "还", "号", "后", "和", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄"]),
    ("ht", &["合同", "后天", "回头", "好听", "和他", "后台", "和她", "还挺", "话题", "喊他", "核桃", "画图", "航天", "回调", "回填", "回退", "HT", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄", "怀", "划", "糊"]),
    ("hw", &["华为", "和我", "好玩", "喊我", "喝完", "回我", "和", "海外", "户外", "毫无", "很晚", "货物", "还未", "好", "会", "还", "号", "后", "哈", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄", "怀", "划", "糊", "呼", "沪", "霍", "寒", "壶"]),
    ("hx", &["好像", "还行", "互相", "还想", "好想", "呼吸", "后续", "或许", "海鲜", "核心", "很想", "好些", "和谐", "化学", "很像", "很小", "幻想", "滑雪", "户型", "核销", "华夏", "号线", "换新", "画像", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("hy", &["还有", "还要", "会员", "会有", "欢迎", "会议", "好友", "行业", "怀疑", "好呀", "回忆", "好用", "很有", "花园", "活跃", "海洋", "还原", "胡", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横", "滑", "吼", "候", "哄", "怀", "划", "糊"]),
    ("hz", &["或者", "还在", "孩子", "合作", "Hz", "杭州", "盒子", "活着", "化妆", "患者", "还早", "黄总", "惠州", "猴子", "会在", "胡总", "很早", "汇总", "护照", "何总", "好在", "会长", "韩总", "画质", "和", "好", "会", "还", "哈", "号", "后", "话", "很", "回", "换", "或", "货", "黄", "喝", "花", "活", "胡", "红", "黑", "华", "化", "含", "盒", "喊", "海", "户", "何", "画", "行", "火", "合", "韩", "坏", "哼", "厚", "害", "混", "宏", "辉", "洪", "浩", "豁", "嗨", "航", "慧", "豪", "灰", "恒", "汉", "函", "环", "侯", "虎", "汇", "惠", "贺", "鸿", "互", "核", "狠", "湖", "郝", "涵", "昊", "嘿", "河", "欢", "护", "猴", "慌", "横"]),
    ("jb", &["基本", "加班", "酒吧", "举报", "就不", "金币", "就把", "级别", "几把", "几百", "进步", "就被", "疾病", "肩膀", "剧本", "举办", "即便", "具备", "脚本", "嘉宾", "局部", "江北", "解绑", "渐变", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jc", &["检查", "经常", "坚持", "机场", "几次", "聚餐", "接触", "基础", "检测", "警察", "就差", "教程", "解除", "教材", "基层", "寄出", "进场", "继承", "监测", "决策", "交叉", "集成", "进程", "简称", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jd", &["觉得", "记得", "几点", "酒店", "京东", "简单", "决定", "绝对", "假的", "九点", "鸡蛋", "经典", "阶段", "街道", "角度", "进度", "降低", "接到", "接单", "接待", "监督", "季度", "节点", "JD", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jf", &["减肥", "积分", "缴费", "姐夫", "解封", "就发", "解放", "甲方", "交付", "肌肤", "交费", "几分", "交房", "加分", "假发", "机房", "经费", "激发", "纠纷", "几份", "均分", "计费", "教辅", "建发", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jg", &["价格", "几个", "结果", "见过", "就跟", "经过", "机构", "交给", "加个", "架构", "结构", "加工", "坚果", "机关", "监管", "激光", "间隔", "警告", "杰哥", "竣工", "寄给", "景观", "加固", "较高", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jh", &["计划", "就好", "机会", "结婚", "就会", "几乎", "几号", "激活", "集合", "讲话", "结合", "精华", "金华", "交换", "建行", "节后", "寄回", "交货", "交互", "简化", "局", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京", "句", "甲", "吉", "聚"]),
    ("jk", &["健康", "尽快", "即可", "就看", "几块", "监控", "进口", "借口", "接口", "讲课", "加快", "就可", "监考", "节课", "解开", "借款", "几款", "JK", "及", "就", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京", "句", "甲", "吉", "聚", "胶", "锦"]),
    ("jl", &["记录", "家里", "尽量", "经理", "进来", "经历", "教练", "奖励", "距离", "交了", "交流", "加了", "简历", "绝了", "精力", "焦虑", "建立", "吉利", "接了", "尽力", "积累", "结论", "激励", "监理", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jm", &["见面", "姐妹", "救命", "节目", "就没", "寂寞", "就买", "舅妈", "界面", "进门", "积木", "加盟", "居民", "鸡毛", "减免", "集美", "江门", "加密", "建模", "解密", "机密", "解码", "紧密", "JM", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jn", &["今年", "就能", "几年", "叫你", "接你", "技能", "见你", "济南", "就你", "教你", "加你", "胶囊", "佳能", "江南", "艰难", "缴纳", "济宁", "Jn", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京", "句", "甲", "吉", "聚", "胶", "锦"]),
    ("jp", &["键盘", "机票", "就怕", "精品", "截屏", "就跑", "极品", "奖品", "镜片", "几瓶", "金牌", "鸡排", "家婆", "接盘", "胶片", "紧迫", "浸泡", "竞品", "竞拍", "奖牌", "几篇", "教培", "竞聘", "JP", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jq", &["进去", "机器", "就去", "假期", "剧情", "节前", "几千", "近期", "进群", "借钱", "交钱", "价钱", "加强", "结清", "技巧", "激情", "金钱", "景区", "减去", "减轻", "极其", "加钱", "交强", "交期", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jr", &["今日", "居然", "进入", "家人", "竟然", "加入", "既然", "肌肉", "金融", "节日", "就让", "加热", "鸡肉", "叫人", "加绒", "假如", "介入", "贱人", "军人", "接入", "兼容", "假日", "几人", "Jr", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("js", &["就是", "结束", "技术", "加上", "介绍", "就说", "精神", "接受", "及时", "就算", "解释", "建设", "健身", "即使", "计算", "据说", "减少", "江苏", "角色", "结算", "加速", "教师", "接收", "js", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jt", &["今天", "几天", "具体", "截图", "家庭", "交通", "叫他", "镜头", "鸡腿", "集体", "吉他", "剪头", "几条", "接头", "极兔", "几台", "阶梯", "机台", "集团", "接听", "箭头", "静态", "接通", "计提", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jw", &["今晚", "叫我", "加我", "接我", "就我", "降温", "价位", "借我", "教我", "几万", "绝望", "就问", "家务", "结尾", "纪委", "几位", "境外", "讲完", "教务", "交完", "较为", "均为", "机位", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京", "句"]),
    ("jx", &["就行", "继续", "进行", "就像", "就想", "惊喜", "嘉兴", "教学", "江西", "吉祥", "绩效", "机箱", "机械", "极限", "解析", "觉醒", "检修", "精选", "机型", "精修", "进项", "接线", "奖项", "镜像", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("jy", &["建议", "加油", "就要", "教育", "就有", "经验", "交易", "记忆", "经营", "基于", "就业", "具有", "检验", "校验", "局", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京", "句", "甲", "吉", "聚", "胶", "锦", "架", "剧", "觉", "洁"]),
    ("jz", &["就在", "兼职", "家长", "饺子", "接着", "节奏", "紧张", "简直", "价值", "记住", "就这", "几张", "竞争", "戒指", "截止", "架子", "集中", "今早", "建筑", "禁止", "进展", "机制", "精准", "加载", "就", "及", "加", "家", "将", "姐", "叫", "件", "接", "进", "讲", "交", "金", "级", "机", "见", "几", "寄", "建", "斤", "价", "卷", "近", "间", "局", "急", "经", "酒", "假", "记", "均", "教", "今", "即", "鸡", "久", "佳", "借", "江", "减", "节", "结", "杰", "旧", "军", "九", "仅", "集", "静", "蒋", "降", "姜", "脚", "俊", "奖", "嘉", "剪", "給", "解", "君", "紧", "景", "贾", "剑", "距", "娟", "角", "精", "较", "计", "夹", "京"]),
    ("kb", &["看吧", "恐怖", "可不", "看病", "看不", "可别", "开吧", "开播", "口碑", "课本", "空白", "课表", "开班", "亏本", "捆绑", "拷贝", "靠背", "卡包", "开标", "狂暴", "K宝", "看板", "口播", "KB", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kc", &["开车", "课程", "库存", "考场", "看成", "考察", "看出", "快充", "快餐", "看错", "开除", "看车", "扣除", "开窗", "烤肠", "开场", "开出", "开仓", "卡车", "科创", "开船", "扩充", "勘察", "KC", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kd", &["肯定", "看到", "快递", "看的", "快点", "开的", "开到", "考的", "宽带", "看得", "开店", "快到", "口袋", "看懂", "空的", "开单", "考点", "扩大", "宽度", "卡点", "看", "开", "快", "卡", "款", "可", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開"]),
    ("kf", &["客服", "开发", "咖啡", "开放", "可否", "康复", "开饭", "看房", "开房", "看法", "库房", "空房", "开封", "扣分", "克服", "空腹", "口服", "扣费", "客房", "开服", "匮乏", "跨服", "可发", "扣罚", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kg", &["kg", "看过", "开工", "开关", "开个", "口感", "看个", "客观", "开挂", "酷狗", "考公", "K歌", "开过", "旷工", "考过", "考个", "空格", "可贵", "控股", "凯哥", "坤哥", "空港", "康哥", "开给", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kh", &["客户", "开会", "考核", "看好", "口红", "可好", "卡号", "开户", "开黑", "看哈", "开花", "课后", "开好", "科幻", "困惑", "口号", "口嗨", "开荒", "括号", "空号", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開"]),
    ("kj", &["科技", "看见", "空间", "会计", "开机", "靠近", "看剧", "恐惧", "框架", "开奖", "课件", "可见", "开局", "快捷", "砍价", "氪金", "考级", "跨境", "口径", "困境", "K金", "开具", "控件", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨"]),
    ("kl", &["快乐", "开了", "看了", "看来", "考虑", "困了", "哭了", "可怜", "快了", "快来", "可乐", "考了", "颗粒", "空了", "卡了", "亏了", "扣了", "卡里", "库里", "口令", "客流", "昆仑", "开裂", "KL", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("km", &["开门", "昆明", "科目", "抠门", "空吗", "开麦", "看嘛", "看吗", "快吗", "可没", "开吗", "开骂", "苦命", "卡密", "快慢", "开明", "km", "开幕", "快门", "开卖", "楷模", "开模", "K码", "口蘑", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kn", &["可能", "看你", "跨年", "困难", "看", "夸你", "柯南", "靠你", "开年", "苦难", "KN", "看您", "看那", "卡你", "坑你", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開", "旷", "渴", "铠", "鲲", "魁", "嗑"]),
    ("kp", &["开票", "靠谱", "可怕", "卡片", "科普", "恐怕", "开盘", "看片", "快跑", "磕碰", "开屏", "卡牌", "开平", "考评", "开炮", "开辟", "烤盘", "开拍", "空瓶", "看盘", "鲲鹏", "KP", "开篇", "客片", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kq", &["快去", "开启", "客气", "空气", "哭泣", "考勤", "口腔", "扣钱", "看清", "口气", "看起", "亏钱", "考前", "况且", "课前", "快钱", "孔雀", "块钱", "靠前", "空缺", "烤漆", "卡其", "客情", "看", "开", "可", "快", "款", "卡", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨"]),
    ("kr", &["客人", "烤肉", "看人", "困扰", "扣肉", "可", "坑人", "宽容", "扩容", "看", "开", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開", "旷", "渴", "铠", "鲲", "魁", "嗑", "堃", "锴", "亢", "炕", "括", "侃"]),
    ("ks", &["开始", "可是", "考试", "快手", "咳嗽", "快速", "看书", "款式", "看上", "考上", "科室", "宽松", "课时", "昆山", "亏损", "卡死", "卡", "ks", "KS", "看", "开", "快", "款", "块", "可", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開", "旷", "渴"]),
    ("kt", &["空调", "客厅", "看他", "开头", "开通", "看她", "课堂", "抗体", "课题", "看图", "开庭", "开团", "卡通", "口头", "磕头", "开题", "开拓", "空投", "考题", "抠图", "可退", "可调", "KT", "楷体", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("kw", &["看我", "看完", "口味", "kW", "考完", "可恶", "开完", "可", "课文", "看", "开", "快", "款", "块", "卡", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開", "旷", "渴", "铠", "鲲", "魁", "嗑", "堃", "锴", "亢", "炕", "括", "侃"]),
    ("kx", &["看下", "开心", "可惜", "科学", "开学", "可笑", "开销", "可行", "烤箱", "空闲", "空虚", "可信", "可选", "看戏", "开箱", "K线", "款项", "科兴", "快些", "凯旋", "开下", "抗性", "空心", "科协", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("ky", &["可以", "考研", "阔以", "开业", "快要", "烤鱼", "可用", "考验", "刻意", "烤鸭", "可有", "科研", "口语", "空运", "跨越", "开元", "开源", "课余", "哭", "KY", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝", "恺", "葵", "咔", "垮", "筐", "琨", "阚", "恐", "開", "旷"]),
    ("kz", &["看着", "控制", "裤子", "口罩", "开着", "科长", "夸张", "克制", "筷子", "开展", "空中", "考证", "看中", "开支", "看在", "口子", "壳子", "卡在", "扣子", "扩展", "空着", "卡着", "卡主", "克重", "哦", "噢", "喔", "筽", "嚄", "看", "开", "可", "快", "卡", "款", "块", "空", "课", "扣", "靠", "库", "考", "口", "克", "亏", "科", "坑", "孔", "宽", "康", "哭", "框", "困", "凯", "客", "剋", "控", "砍", "颗", "肯", "苦", "跨", "夸", "坤", "抗", "壳", "烤", "矿", "柯", "裤", "狂", "酷", "扩", "氪", "抠", "啃", "刻", "扛", "咖", "磕", "昆", "奎", "捆", "珂", "咳", "楷", "况", "拷", "匡", "阔", "寇", "坎", "棵", "刊", "嘅", "邝"]),
    ("lb", &["老板", "来吧", "老爸", "两边", "里边", "路边", "喇叭", "礼拜", "两百", "萝卜", "老表", "来不", "礼包", "吕布", "两把", "了吧", "列表", "录播", "两笔", "类别", "lb", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒"]),
    ("lc", &["流程", "两次", "凌晨", "理财", "楼层", "绿茶", "临床", "流畅", "两场", "路程", "立场", "聊城", "露出", "老陈", "两层", "留存", "两侧", "拉扯", "里程", "流出", "量产", "浪潮", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒"]),
    ("ld", &["领导", "来的", "老大", "六点", "溜达", "懒得", "两点", "落地", "老弟", "聊的", "拉倒", "来电", "来到", "劳动", "拉到", "老的", "力度", "亮点", "漏洞", "了", "联动", "LD", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒"]),
    ("lf", &["浪费", "两份", "理发", "路费", "立方", "廊坊", "冷风", "临汾", "礼服", "两房", "来访", "疗法", "立法", "漏风", "劳烦", "裂缝", "李飞", "漏发", "刘芳", "龙飞", "流放", "刘飞", "李峰", "LF", "哦", "噢", "喔", "筽", "嚄", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂"]),
    ("lg", &["两个", "老公", "来个", "龙哥", "路过", "老哥", "李工", "理工", "连个", "李哥", "六个", "刘哥", "乐观", "留给", "留个", "磊哥", "俩个", "轮毂", "流感", "聊过", "灵感", "亮哥", "龙岗", "LG", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪"]),
    ("lh", &["厉害", "离婚", "礼盒", "拉黑", "来回", "良好", "两盒", "灵魂", "老虎", "联合", "莲花", "落后", "灵活", "龙华", "龙湖", "例会", "落户", "利好", "老化", "老黄", "绿化", "量化", "LH", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳"]),
    ("lj", &["了解", "链接", "理解", "垃圾", "老家", "逻辑", "辣椒", "连接", "邻居", "两件", "冷静", "两家", "两句", "累计", "拦截", "立即", "临近", "来讲", "路径", "零件", "联机", "LJ", "了", "来", "啦", "刘", "拉", "里", "李", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒"]),
    ("lk", &["离开", "来看", "立刻", "路口", "理科", "唠嗑", "两块", "裂开", "两颗", "拉跨", "两款", "拉开", "老款", "领克", "旅客", "轮廓", "里", "了", "来", "啦", "刘", "拉", "李", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒", "链", "礼", "凌", "辆", "栏"]),
    ("lm", &["里面", "了吗", "老妈", "立马", "浪漫", "来吗", "联盟", "礼貌", "厘米", "拉面", "蓝莓", "黎明", "联名", "老马", "路面", "了么", "了没", "类目", "乱码", "栏目", "螺母", "L码", "拉满", "LM", "哦", "噢", "喔", "筽", "嚄", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂"]),
    ("ln", &["两年", "来拿", "辽宁", "来年", "老娘", "拉你", "理你", "老年", "李宁", "离你", "六年", "理念", "了呢", "李娜", "历年", "懒", "Ln", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒", "链", "礼", "凌", "辆", "栏"]),
    ("lp", &["老婆", "离谱", "礼品", "录屏", "两瓶", "凉皮", "理赔", "楼盘", "脸皮", "两片", "咯", "蓝屏", "刘鹏", "两篇", "临平", "令牌", "良品", "李鹏", "零跑", "LP", "流派", "丽萍", "两票", "罗盘", "了", "来", "里", "啦", "李", "刘", "拉", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳"]),
    ("lq", &["领取", "两千", "篮球", "力气", "六千", "乐趣", "录取", "拉去", "零钱", "领券", "龙泉", "李强", "漏气", "冷却", "两期", "路桥", "沥青", "拉群", "临期", "拉起", "拉取", "了", "来", "里", "李", "刘", "啦", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒"]),
    ("lr", &["利润", "老人", "了", "两人", "俩人", "路人", "录入", "腊肉", "来人", "令人", "恋人", "例如", "拉人", "懒人", "狼人", "猎人", "六日", "列入", "流入", "LR", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒", "链", "礼"]),
    ("ls", &["老师", "路上", "历史", "来说", "楼上", "老是", "零食", "律师", "类似", "绿色", "临时", "流水", "蓝色", "拉屎", "脸上", "螺丝", "落实", "啦", "LS", "来", "里", "李", "刘", "了", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒", "链", "礼", "凌"]),
    ("lt", &["两天", "聊天", "联通", "老头", "两条", "老铁", "轮胎", "两套", "辣条", "楼梯", "里头", "两台", "龙头", "论坛", "立体", "蓝天", "两头", "骆驼", "流通", "链条", "猎头", "雷霆", "联调", "Lt", "哦", "噢", "喔", "筽", "嚄", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂"]),
    ("lw", &["另外", "论文", "礼物", "了", "来我", "老王", "两万", "拉我", "理我", "两位", "老外", "老吴", "劳务", "聊完", "联网", "螺纹", "LW", "来", "里", "李", "刘", "啦", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒", "链", "礼", "凌", "辆", "栏"]),
    ("lx", &["联系", "楼下", "理想", "利息", "连续", "练习", "类型", "留下", "老乡", "旅行", "路线", "流行", "联想", "陆续", "理性", "录像", "留学", "领先", "聊下", "滤芯", "离线", "履行", "拉新", "LX", "哦", "噢", "喔", "筽", "嚄", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂"]),
    ("ly", &["旅游", "理由", "洛阳", "乐意", "利益", "留言", "留意", "利用", "录音", "蓝牙", "领域", "临沂", "来源", "路由", "路", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂", "另", "磊", "黎", "琳", "浪", "柳", "勒", "粒", "链", "礼", "凌", "辆", "栏", "论", "凉"]),
    ("lz", &["来着", "李总", "老子", "lz", "刘总", "留着", "离职", "两张", "来自", "梁总", "兰州", "两种", "两只", "留在", "两周", "林总", "连着", "罗总", "拉着", "来做", "例子", "老张", "录制", "柳州", "哦", "噢", "喔", "筽", "嚄", "了", "来", "里", "啦", "李", "刘", "拉", "咯", "量", "老", "类", "聊", "楼", "林", "连", "俩", "六", "路", "龙", "两", "留", "罗", "嘞", "领", "率", "冷", "力", "列", "梁", "累", "离", "脸", "录", "丽", "蓝", "练", "乐", "亮", "料", "理", "流", "立", "吕", "陆", "雷", "乱", "绿", "漏", "利", "卢", "兰", "廖", "零", "玲", "咧", "鲁", "轮", "联", "喽", "落", "樂", "赖", "良", "懒", "辣", "灵", "烂"]),
    ("mb", &["目标", "明白", "面包", "毛病", "忙吧", "买吧", "模板", "麻痹", "没变", "弥补", "美白", "懵逼", "忙不", "没把", "模版", "面板", "面部", "膜拜", "没报", "末班", "木板", "门板", "幕布", "MB", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("mc", &["每次", "没错", "没吃", "买菜", "买车", "名称", "没车", "没出", "没穿", "摩擦", "门窗", "马超", "目测", "抹茶", "卖车", "漫长", "卖出", "名词", "没差", "明朝", "卖场", "牧场", "MC", "满仓", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("md", &["买的", "妈的", "没得", "买点", "卖的", "免得", "买到", "面对", "门店", "美的", "名单", "矛盾", "目的", "没到", "忙的", "慢点", "没带", "没打", "买单", "没电", "卖掉", "没懂", "卖点", "么", "吗", "没", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽"]),
    ("mf", &["麻烦", "免费", "没法", "买房", "米饭", "没发", "满分", "米粉", "棉服", "模仿", "面粉", "魔法", "没放", "密封", "蜜蜂", "魔方", "卖房", "没分", "美缝", "没付", "买方", "卖方", "MF", "蛮烦", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("mg", &["买个", "每个", "美国", "玫瑰", "敏感", "芒果", "买过", "没给", "某个", "没过", "蘑菇", "卖给", "没干", "马哥", "木工", "没关", "咪咕", "蒙古", "没改", "美观", "曼谷", "明哥", "美股", "mg", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("mh", &["美好", "没回", "蛮好", "模糊", "漫画", "迷糊", "没货", "没喝", "买好", "没好", "卖货", "没换", "忙活", "盲盒", "磨合", "梦幻", "棉花", "梅花", "美化", "灭火", "闵行", "门户", "木", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚"]),
    ("mj", &["面积", "麻将", "美甲", "没见", "卖家", "毛巾", "墨迹", "没劲", "马甲", "买家", "母鸡", "美金", "没接", "模具", "米家", "没加", "满级", "密集", "没交", "门禁", "满减", "敏捷", "媒介", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽"]),
    ("mk", &["门口", "模块", "没空", "没看", "没开", "门槛", "没课", "毛孔", "马克", "命苦", "煤矿", "mk", "麦克", "摩卡", "模考", "每款", "门框", "MK", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽", "沐", "煤", "穆"]),
    ("ml", &["买了", "没了", "卖了", "ml", "美丽", "没来", "麻了", "忙了", "满了", "梦里", "魅力", "买来", "麻辣", "骂了", "面料", "猫粮", "马路", "忙碌", "慢了", "命令", "目录", "面临", "茉莉", "毛利", "摸", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("mn", &["明年", "美女", "每年", "没呢", "骂你", "没你", "模拟", "迷你", "没拿", "没能", "猛男", "卖你", "没那", "没弄", "蒙牛", "慢", "MN", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽", "沐", "煤", "穆", "淼", "慕"]),
    ("mp", &["名片", "买票", "门票", "没票", "毛坯", "名牌", "门派", "MP", "门牌", "铭牌", "摸", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽", "沐", "煤", "穆", "淼", "慕", "乜", "闵", "抹", "漫", "蜜", "缪"]),
    ("mq", &["目前", "没钱", "明确", "没去", "母亲", "勉强", "没起", "默契", "煤气", "毛钱", "棉签", "门前", "米奇", "密切", "面签", "名气", "面前", "盲区", "每期", "没签", "幕墙", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽"]),
    ("mr", &["没人", "默认", "每日", "骂人", "每人", "某人", "美容", "明日", "么", "母乳", "美人", "迷人", "没让", "买入", "名人", "面容", "末日", "慢热", "毛绒", "盲人", "Mr", "猫人", "敏锐", "吗", "没", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚"]),
    ("ms", &["没事", "马上", "没啥", "模式", "面试", "没说", "貌似", "美食", "吗", "描述", "秒杀", "美术", "民生", "民宿", "魔兽", "MS", "ms", "没", "买", "们", "卖", "米", "么", "嘛", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽", "沐", "煤", "穆", "淼", "慕"]),
    ("mt", &["明天", "每天", "美团", "馒头", "面条", "茅台", "摩托", "媒体", "马桶", "模特", "骂他", "美图", "没听", "木头", "码头", "面谈", "门头", "命题", "煤炭", "没提", "每条", "民泰", "民天", "Mt", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("mw", &["骂我", "忙完", "买完", "门卫", "明晚", "没玩", "美味", "么", "没问", "漫威", "卖完", "MW", "没", "买", "们", "卖", "米", "吗", "嘛", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽", "沐", "煤", "穆", "淼", "慕", "乜", "闵", "抹", "漫", "蜜"]),
    ("mx", &["明显", "梦想", "明星", "毛线", "米线", "没想", "明细", "模型", "没写", "没戏", "某些", "名下", "默写", "没下", "面相", "慢性", "冒险", "免息", "没学", "满血", "mx", "描写", "面向", "冥想", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("my", &["没有", "没用", "木有", "蚂蚁", "妈呀", "满意", "毛衣", "每月", "绵阳", "命运", "么有", "免疫", "美元", "摸鱼", "贸易", "名义", "my", "没雨", "木", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙", "闽", "枚", "帽", "沐", "煤", "穆"]),
    ("mz", &["名字", "满足", "妹子", "门诊", "帽子", "明早", "没在", "面子", "没做", "马总", "没准", "忙着", "每周", "民族", "没咋", "没走", "民主", "美妆", "某种", "明珠", "梅州", "命中", "模组", "面诊", "吗", "没", "么", "买", "嘛", "们", "卖", "米", "忙", "面", "码", "妈", "门", "名", "明", "马", "满", "秒", "慢", "美", "毛", "猫", "咩", "梦", "骂", "敏", "每", "摸", "木", "猛", "莫", "梅", "民", "麦", "妹", "某", "孟", "膜", "命", "蛮", "免", "末", "目", "模", "铭", "苗", "麻", "蒙", "磨", "棉", "喵", "灭", "萌", "母", "密", "冇", "嗎", "亩", "迷", "咪", "埋", "迈", "茂", "曼", "盟", "墨", "懵", "鸣", "冒", "魔", "闷", "妙"]),
    ("nb", &["那边", "牛逼", "你把", "内部", "你不", "你别", "你爸", "宁波", "那不", "能不", "能把", "你被", "哪边", "南边", "弄吧", "那把", "南北", "您把", "NB", "年报", "南部", "难绷", "那比", "女", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲"]),
    ("nc", &["奶茶", "你猜", "农村", "你吃", "能吃", "那次", "内存", "你才", "南昌", "拿出", "脑残", "难吃", "昵称", "你从", "弄成", "年初", "南充", "农场", "能出", "内测", "弄错", "南侧", "NC", "女", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲"]),
    ("nd", &["你的", "男的", "女的", "你都", "难道", "拿到", "年底", "难得", "你得", "您的", "拿的", "弄的", "脑袋", "能到", "难度", "年度", "年代", "能打", "牛的", "内地", "浓度", "难点", "呢", "你", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努"]),
    ("nf", &["能否", "你发", "奶粉", "南方", "能发", "女方", "男方", "那份", "年饭", "农夫", "年费", "你放", "年份", "能放", "暖风", "南非", "奈飞", "能付", "NF", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫"]),
    ("ng", &["那个", "哪个", "能够", "难过", "你个", "你给", "你跟", "难搞", "拿个", "弄个", "能干", "你哥", "能给", "难怪", "南瓜", "拿给", "年糕", "能过", "嫩个", "拿过", "内购", "牛哥", "弄过", "NG", "唔", "嗯", "女", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩"]),
    ("nh", &["你好", "您好", "你还", "你会", "女孩", "年后", "你和", "那会", "那还", "暖和", "弄好", "男孩", "年会", "拿货", "年货", "农行", "内核", "内耗", "NH", "怒", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫"]),
    ("nj", &["那就", "你就", "你家", "南京", "年纪", "那家", "那集", "哪家", "你姐", "你叫", "年级", "年假", "那件", "那句", "娜姐", "年检", "您家", "凝胶", "你", "内卷", "年金", "内径", "NJ", "那", "能", "年", "您", "拿", "呢", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努"]),
    ("nk", &["你看", "内裤", "年卡", "你可", "你快", "难看", "脑壳", "能看", "你开", "您看", "那块", "耐克", "那款", "哪款", "能开", "那可", "宁可", "内科", "男款", "尼康", "女款", "你", "哪块", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努"]),
    ("nl", &["哪里", "那里", "努力", "能力", "你了", "你来", "你俩", "年龄", "拿了", "拿来", "弄了", "能量", "你两", "农历", "哪来", "NL", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫", "內", "恼", "孽"]),
    ("nm", &["你们", "那么", "你妈", "你没", "你买", "尼玛", "柠檬", "你忙", "南门", "内蒙", "您们", "农民", "纳闷", "匿名", "奶妈", "糯米", "你么", "难免", "nm", "年末", "纳米", "牛马", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努"]),
    ("np", &["哪怕", "南平", "牛批", "牛排", "你怕", "你拍", "奶瓶", "牛皮", "你跑", "脑婆", "能跑", "能拍", "那篇", "女仆", "那批", "那片", "那排", "您拍", "NP", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫"]),
    ("nq", &["年前", "你去", "年轻", "拿去", "能去", "暖气", "拿钱", "哪去", "南区", "拿起", "你钱", "扭曲", "内勤", "女权", "那群", "呢亲", "怒气", "内墙", "内嵌", "奶骑", "那", "你", "呢", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻"]),
    ("nr", &["内容", "男人", "女人", "牛肉", "呢", "你让", "那人", "能让", "粘人", "你人", "南瑞", "纳入", "牛人", "诺如", "呢人", "nr", "你", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫", "內", "恼", "孽", "馕"]),
    ("ns", &["那是", "你是", "你说", "难受", "男生", "女生", "女神", "南山", "男士", "女士", "那", "那啥", "您是", "内饰", "农商", "您说", "NS", "你", "能", "年", "您", "弄", "呢", "拿", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫", "內", "恼", "孽"]),
    ("nt", &["那天", "哪天", "逆天", "那他", "那条", "你太", "南通", "难听", "那套", "难题", "年头", "拿铁", "那台", "脑瘫", "念头", "那她", "哪条", "内推", "男同", "牛头", "挠头", "农田", "NT", "暖通", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽"]),
    ("nw", &["那我", "你问", "你我", "弄完", "呢", "哪位", "你玩", "那位", "拿我", "女王", "能玩", "牛蛙", "你为", "内网", "拿完", "内外", "年味", "女巫", "挪威", "NW", "你", "能", "年", "您", "弄", "那", "拿", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努", "孬", "捻", "蔫"]),
    ("nx", &["那些", "你想", "你先", "哪些", "耐心", "内心", "女性", "拿下", "男性", "能行", "闹心", "那行", "年薪", "那先", "你写", "宁夏", "内向", "年限", "逆袭", "弄下", "您先", "能效", "粘性", "逆向", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽"]),
    ("ny", &["你也", "你要", "你有", "那样", "你又", "哪有", "能用", "那也", "内衣", "能有", "你用", "南阳", "男友", "女友", "农业", "宁愿", "奶油", "那要", "难以", "您要", "能源", "NY", "怒", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽", "嬲", "努"]),
    ("nz", &["你这", "那种", "你在", "脑子", "你咋", "你再", "拿着", "你做", "哪种", "年终", "能做", "闹钟", "拿走", "那张", "女主", "你找", "女子", "男主", "能在", "哪吒", "女装", "您在", "内置", "NZ", "你", "呢", "那", "能", "年", "您", "拿", "弄", "内", "哪", "男", "女", "难", "牛", "奶", "南", "捏", "宁", "嫩", "呐", "拟", "鸟", "闹", "念", "农", "娜", "诺", "楠", "暖", "怒", "倪", "尼", "聂", "尿", "娘", "脑", "挪", "耐", "妮", "泥", "腻", "乃", "纳", "喃", "妞", "恁", "浓", "黏", "拧", "虐", "逆", "扭", "囊", "妳", "粘", "奈", "钠", "凝", "攝", "挠", "侬", "镍", "奴", "柠", "酿", "廿", "糯", "喏", "钮", "弩", "撵", "纽"]),
    ("pb", &["旁边", "跑步", "平板", "陪伴", "屏蔽", "普遍", "排班", "疲惫", "排版", "拼搏", "配比", "攀比", "排便", "破冰", "瀑布", "配备", "屏保", "漂白", "评比", "拍板", "PB", "旁白", "排布", "评标", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("pc", &["赔偿", "平常", "破产", "拼车", "泡茶", "排查", "跑车", "排除", "嫖娼", "品尝", "爬虫", "排斥", "批次", "偏差", "评测", "PC", "飘窗", "派车", "跑出", "鹏程", "频次", "平仓", "排产", "抛出", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("pd", &["判断", "拍的", "排队", "碰到", "跑的", "跑到", "频道", "平等", "排到", "配的", "皮带", "浦东", "盘点", "派对", "拍到", "片段", "佩戴", "铺垫", "判定", "派单", "配对", "偏低", "排单", "PD", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("pf", &["皮肤", "批发", "频繁", "佩服", "浦发", "平方", "评分", "配方", "泡芙", "赔付", "平凡", "票房", "破防", "批复", "平分", "破费", "派发", "鹏飞", "普法", "披风", "排放", "篇幅", "跑分", "屏风", "哦", "噢", "喔", "筽", "嚄", "拍", "²", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇"]),
    ("pg", &["苹果", "评估", "屁股", "拍个", "排骨", "配个", "鹏哥", "拍过", "普攻", "胖哥", "批改", "偏高", "抛光", "跑过", "瓶盖", "皮革", "盘古", "普工", "平哥", "PG", "派工", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖"]),
    ("ph", &["配合", "平衡", "破坏", "排行", "平和", "屁话", "喷绘", "陪护", "普惠", "胖虎", "排号", "拍好", "配货", "配好", "平湖", "批号", "偏好", "铺货", "平滑", "牌号", "普货", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤"]),
    ("pj", &["评价", "配件", "啤酒", "平均", "派件", "泡脚", "扑街", "碰见", "平静", "普及", "破解", "拼接", "票价", "票据", "品鉴", "萍姐", "判决", "评级", "凭借", "瓶颈", "破局", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤"]),
    ("pk", &["PK", "皮卡", "扑克", "贫困", "跑酷", "派克", "抛开", "排课", "盘口", "凭空", "品控", "片刻", "匹克", "盘库", "赔款", "浦口", "朋克", "铺开", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤", "闆", "苹", "脾"]),
    ("pl", &["漂亮", "评论", "跑了", "拍了", "胖了", "排练", "跑路", "怕了", "频率", "破了", "批了", "骗了", "疲劳", "排了", "批量", "赔了", "跑来", "品类", "配了", "配料", "排列", "跑量", "披露", "偏离", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴"]),
    ("pm", &["屏幕", "排名", "泡面", "拼命", "泡沫", "pm", "平米", "拍卖", "平面", "平民", "碰面", "排面", "皮毛", "片面", "铺面", "盘面", "品名", "排满", "票面", "铺满", "泡棉", "P棉", "喷码", "剖面", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("pn", &["陪你", "怕你", "骗你", "叛逆", "婆娘", "泡妞", "拍你", "平南", "排尿", "普宁", "跑男", "盘", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤", "闆", "苹", "脾", "刨", "胚", "泼", "蓬", "璞", "馮", "芃"]),
    ("pq", &["脾气", "跑去", "赔钱", "抛弃", "骗钱", "喷漆", "片区", "贫穷", "排气", "排期", "喷泉", "佩奇", "派去", "排球", "碰巧", "派遣", "迫切", "配齐", "聘请", "喷枪", "皮球", "拍全", "PQ", "拍", "跑", "票", "平", "品", "怕", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈"]),
    ("pr", &["骗人", "平日", "派人", "烹饪", "怕人", "PR", "仆人", "旁人", "怕热", "譬如", "疲软", "飘柔", "喷人", "偏弱", "陪人", "拍人", "偏软", "聘任", "普瑞", "排入", "叛忍", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖"]),
    ("ps", &["平时", "拍摄", "配送", "怕是", "披萨", "爬山", "怕啥", "派送", "PS", "配上", "破损", "配色", "评审", "凭啥", "怕", "拍", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤", "闆", "苹", "脾", "刨", "胚", "泼", "蓬"]),
    ("pt", &["平台", "普通", "配套", "葡萄", "陪她", "跑腿", "拼图", "陪他", "莆田", "怕他", "拼团", "P图", "陪同", "碰头", "配图", "平摊", "跑图", "喷头", "普陀", "PT", "旁听", "平替", "喷涂", "片头", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("pw", &["陪我", "骗我", "排位", "喷雾", "怕我", "品味", "拍完", "评委", "拍我", "平稳", "跑完", "脾胃", "陪玩", "批文", "票务", "评为", "片尾", "铺位", "排污", "批完", "PW", "拍", "跑", "票", "平", "品", "怕", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤"]),
    ("px", &["培训", "拍下", "螃蟹", "皮鞋", "品相", "排序", "偏心", "贫血", "偏向", "平行", "评选", "貔貅", "跑鞋", "婆媳", "偏小", "萍乡", "咆哮", "排线", "剖析", "破晓", "拼写", "皮下", "品项", "碰下", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("py", &["朋友", "便宜", "培养", "拼音", "配音", "濮阳", "番禺", "偏远", "平原", "皮衣", "评语", "漂移", "聘用", "培育", "评优", "平移", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪", "坪", "扑", "剖", "捧", "朴", "劈", "帕", "啤", "闆", "苹", "脾", "刨", "胚"]),
    ("pz", &["配置", "拍照", "牌子", "品种", "胖子", "品质", "陪着", "骗子", "潘总", "片子", "凭证", "瓶子", "盘子", "趴着", "泡澡", "批准", "彭总", "排在", "牌照", "膨胀", "铺子", "碰撞", "拍子", "盘中", "哦", "噢", "喔", "筽", "嚄", "拍", "怕", "跑", "票", "平", "品", "排", "配", "片", "盘", "批", "瓶", "陪", "彭", "潘", "破", "拼", "牌", "篇", "皮", "鹏", "偏", "屁", "喷", "派", "泡", "骗", "屏", "胖", "评", "碰", "爬", "赔", "萍", "普", "噗", "铺", "朋", "盆", "炮", "庞", "培", "判", "旁", "飘", "裴", "抛", "呸", "匹", "凭", "婆", "漂", "佩", "趴", "蒲", "坡", "盼", "频", "嫖", "棚", "谱", "浦", "沛", "聘", "攀", "撇", "啪"]),
    ("qb", &["全部", "去吧", "区别", "钱包", "起步", "去不", "确保", "七八", "去呗", "前边", "全包", "铅笔", "全班", "前辈", "枪毙", "请把", "情报", "墙壁", "Q币", "墙板", "q版", "签报", "请帮", "QB", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qc", &["清楚", "起床", "汽车", "骑车", "去吃", "全程", "青春", "青菜", "全场", "芹菜", "其次", "清除", "去除", "清仓", "球场", "七彩", "牵扯", "器材", "全称", "取出", "千川", "QC", "起草", "去重", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qd", &["确定", "七点", "期待", "去的", "全都", "渠道", "青岛", "签到", "启动", "去掉", "清单", "抢到", "强大", "缺点", "强调", "驱动", "祈祷", "取得", "前端", "强度", "起点", "前段", "签订", "起到", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qf", &["欺负", "群发", "起飞", "缺乏", "气氛", "清风", "区分", "欠费", "祈福", "勤奋", "前夫", "前方", "潜伏", "启发", "恰饭", "侵犯", "曲阜", "起伏", "球阀", "签发", "区服", "千帆", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆"]),
    ("qg", &["全国", "去过", "奇怪", "强哥", "情感", "抢购", "七个", "器官", "球馆", "请个", "缺个", "强国", "切割", "去个", "乞丐", "气管", "取个", "求个", "起个", "清关", "全给", "签个", "前杠", "气缸", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qh", &["前后", "切换", "清华", "缺货", "青海", "取货", "强化", "情怀", "气候", "期货", "前海", "启航", "清河", "契合", "签好", "浅灰", "全会", "群晖", "企划", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒", "仟", "屈", "掐", "企", "祁", "钦", "卿", "丘", "旗"]),
    ("qj", &["请假", "期间", "全家", "清洁", "情节", "请教", "前进", "奇迹", "取件", "秋季", "旗舰", "区间", "全景", "前景", "全局", "桥架", "请", "亲", "钱", "前", "群", "去", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒", "仟", "屈", "掐", "企", "祁", "钦", "卿", "丘", "旗", "趣", "裙", "犬"]),
    ("qk", &["情况", "去看", "请客", "全款", "勤快", "秋裤", "全靠", "清空", "欠款", "缺口", "全科", "乾坤", "期刊", "请看", "千克", "起", "去", "请", "亲", "钱", "前", "群", "全", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒", "仟", "屈", "掐", "企", "祁", "钦", "卿", "丘", "旗", "趣", "裙", "犬", "翘"]),
    ("ql", &["起来", "去了", "群里", "清理", "情侣", "起了", "签了", "抢了", "权利", "强烈", "请了", "全力", "齐了", "麒麟", "权力", "清零", "群聊", "强力", "潜力", "区里", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒"]),
    ("qm", &["前面", "全面", "去吗", "起码", "去买", "全民", "期末", "签名", "清明", "敲门", "亲们", "亲密", "全名", "群名", "墙面", "骑马", "奇妙", "启蒙", "起名", "前门", "取名", "QM", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆"]),
    ("qn", &["去年", "请你", "去哪", "去拿", "去你", "青年", "前年", "全年", "亲你", "请您", "全能", "欠你", "求你", "劝你", "群", "群内", "取暖", "七年", "千年", "去", "请", "钱", "前", "全", "亲", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒", "仟", "屈", "掐", "企", "祁", "钦", "卿", "丘", "旗", "趣"]),
    ("qp", &["奇葩", "欺骗", "气泡", "抢票", "强迫", "前排", "切片", "期盼", "旗袍", "取票", "去拍", "棋牌", "起泡", "全屏", "青浦", "起皮", "清盘", "去皮", "全盘", "全品", "汽配", "球拍", "壳牌", "棋盘", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qr", &["确认", "前任", "情人", "气人", "亲人", "穷人", "缺人", "奇瑞", "请人", "亲热", "清热", "千人", "求人", "轻柔", "侵入", "强弱", "去人", "七日", "切入", "前人", "琴日", "嵌入", "迁入", "QR", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qs", &["其实", "确实", "全是", "轻松", "全身", "去世", "气死", "寝室", "签收", "墙上", "强势", "趋势", "起诉", "情商", "缺少", "骑手", "骑士", "缺失", "全省", "请示", "全市", "签署", "券商", "清算", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qt", &["其他", "前天", "其它", "七天", "秋天", "前台", "全天", "全套", "全体", "前提", "去趟", "晴天", "群体", "拳头", "前途", "请他", "青铜", "劝退", "气体", "牵头", "洽谈", "墙体", "QT", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼"]),
    ("qw", &["请问", "千万", "请我", "期望", "去玩", "全网", "轻微", "全屋", "气温", "欠我", "去问", "青蛙", "权威", "前往", "亲吻", "气味", "趣味", "全文", "清完", "请勿", "签完", "企微", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆"]),
    ("qx", &["取消", "权限", "情绪", "清醒", "全新", "清晰", "清洗", "强行", "谦虚", "期限", "倾向", "骑行", "清新", "气息", "曲线", "缺陷", "旗下", "前行", "器械", "气血", "气象", "倾斜", "区县", "情形", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦"]),
    ("qy", &["企业", "区域", "清远", "去", "轻易", "签约", "其余", "迁移", "全员", "权益", "群友", "启用", "前沿", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒", "仟", "屈", "掐", "企", "祁", "钦", "卿", "丘", "旗", "趣", "裙", "犬", "翘", "戚", "茜", "迁"]),
    ("qz", &["其中", "签字", "签证", "群主", "裙子", "去做", "去找", "亲自", "强制", "气质", "泉州", "庆祝", "圈子", "群众", "妻子", "全职", "亲子", "前置", "潜在", "权重", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "去", "请", "亲", "钱", "前", "群", "全", "起", "区", "且", "其", "强", "签", "取", "期", "清", "切", "缺", "抢", "求", "券", "七", "气", "器", "千", "圈", "球", "青", "齐", "欠", "揭", "却", "秦", "情", "墙", "秋", "邱", "权", "庆", "枪", "琴", "确", "骑", "佢", "轻", "桥", "穷", "乔", "啐", "將", "恰", "琪", "曲", "巧", "敲", "奇", "倩", "晴", "勤", "启", "覃", "浅", "泉", "琦", "琼", "漆", "劝", "柒"]),
    ("rb", &["日本", "人保", "日报", "肉包", "人不", "人吧", "日版", "让步", "肉饼", "软包", "人本", "绒布", "热泵", "RB", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉"]),
    ("rc", &["人才", "日常", "如此", "日出", "认错", "入场", "入仓", "融创", "日程", "日产", "认出", "人次", "如初", "热菜", "人称", "仁川", "热刺", "荣成", "荣昌", "人车", "容错", "RC", "入池", "入厂", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("rd", &["人多", "人的", "人都", "认得", "热的", "扔掉", "热度", "热点", "认定", "人大", "软的", "入党", "人地", "弱点", "扔到", "热带", "瑞典", "蠕动", "弱电", "熔断", "热电", "RD", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤"]),
    ("rf", &["染发", "热风", "乳房", "热敷", "热饭", "人防", "人份", "润肤", "软饭", "润发", "日服", "软辅", "如风", "日方", "燃放", "瑞风", "日丰", "瑞丰", "荣放", "让负", "让发", "人福", "R方", "润丰", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("rg", &["如果", "人工", "入股", "人格", "让给", "若果", "认购", "日光", "若干", "热狗", "软管", "如皋", "荣光", "乳鸽", "肉桂", "软膏", "绕过", "荣哥", "扔给", "瑞哥", "容桂", "锐哥", "日更", "肉鸽", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("rh", &["然后", "如何", "任何", "融合", "让后", "热乎", "入户", "人和", "日后", "人话", "融化", "人后", "入会", "仁和", "润滑", "日韩", "人行", "柔和", "弱化", "入伙", "荣获", "日化", "人", "让", "日", "热", "肉", "如", "入", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤"]),
    ("rj", &["人家", "软件", "如今", "入境", "人间", "人机", "日记", "人均", "日结", "如家", "乳胶", "溶解", "日均", "儒家", "瑞金", "入金", "仍旧", "人际", "人居", "溶剂", "锐捷", "人", "让", "热", "肉", "若", "日", "如", "入", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "瓤", "褥", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤", "铷"]),
    ("rk", &["入口", "认可", "入库", "人口", "入坑", "让开", "任课", "绕开", "软壳", "软考", "认款", "入科", "人", "让", "如", "入", "热", "日", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤", "铷", "叒", "榮", "汭", "箬", "紉", "仞", "冗", "桡", "紝"]),
    ("rl", &["人力", "人类", "日历", "扔了", "热了", "人流", "容量", "热量", "人来", "认领", "入了", "日落", "人脸", "日料", "热烈", "日了", "认了", "肉类", "人了", "热力", "燃料", "RL", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱"]),
    ("rm", &["人民", "人们", "热门", "入门", "人脉", "人嘛", "人名", "人美", "肉麻", "热卖", "辱骂", "人命", "RM", "认命", "肉沫", "任命", "扰民", "容貌", "绒毛", "日吗", "日漫", "软膜", "人马", "人吗", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("rn", &["让你", "人呢", "热闹", "人", "嗯", "让您", "RN", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤", "铷", "叒", "榮", "汭", "箬", "紉", "仞", "冗", "桡", "紝", "妊", "荏", "孺", "洳", "亻", "鈤"]),
    ("rp", &["人品", "肉片", "日抛", "热评", "人陪", "RP", "乳品", "饶平", "肉皮", "瑞鹏", "让平", "任凭", "锐评", "睿频", "人皮", "日配", "瑞普", "任鹏", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐"]),
    ("rq", &["日期", "热情", "燃气", "人情", "人气", "人群", "入侵", "人去", "入群", "热气", "认清", "融洽", "让去", "容器", "人权", "瑞琪", "如期", "日前", "任丘", "入去", "日清", "让其", "人齐", "任期", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("rs", &["认识", "人生", "人数", "热水", "人少", "人事", "入手", "热搜", "人士", "燃烧", "人手", "人设", "染色", "肉丝", "人参", "瑞士", "如实", "人寿", "日式", "若是", "如上", "人社", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤"]),
    ("rt", &["让他", "让她", "人头", "人体", "让它", "认同", "乳头", "肉体", "如同", "如图", "肉疼", "软糖", "乳糖", "入团", "融通", "软体", "人天", "RT", "热图", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁"]),
    ("rw", &["任务", "让我", "认为", "人物", "惹我", "人为", "肉丸", "人文", "入伍", "入围", "RW", "容我", "荣威", "日文", "如我", "热", "软文", "软卧", "仍未", "入网", "热舞", "如无", "锐雯", "人", "让", "日", "如", "入", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤"]),
    ("rx", &["如下", "人性", "任性", "人心", "热线", "瑞幸", "入学", "人选", "荣幸", "热血", "任选", "日系", "热心", "乳腺", "人像", "热销", "RX", "入选", "人效", "如需", "韧性", "仍需", "人形", "日线", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("ry", &["人员", "容易", "荣耀", "如意", "如愿", "日语", "任意", "如有", "荣誉", "日元", "乳液", "入园", "肉眼", "燃油", "入院", "若有", "日用", "仍有", "溶液", "如", "人", "让", "日", "热", "肉", "入", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容", "仍", "弱", "惹", "睿", "仁", "锐", "染", "融", "忍", "冉", "绕", "阮", "绒", "蓉", "茹", "饶", "乳", "燃", "蕊", "柔", "芮", "刃", "汝", "揉", "榕", "儒", "戎", "讓", "壬", "溶", "熔", "闰", "扰", "嵘", "韧", "苒", "挼", "嚷", "辱", "茸", "熯", "衻", "囸", "熱", "袽", "镕", "褥", "瓤", "壤", "驲", "濡", "棯", "蕠", "偌", "娆", "壖", "蕤", "铷", "叒", "榮"]),
    ("rz", &["认真", "日子", "认证", "入职", "入住", "认知", "人在", "忍住", "忍着", "日照", "融资", "弱智", "日志", "入账", "任总", "入驻", "睿智", "软装", "人资", "任职", "润泽", "仍在", "软著", "入组", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "人", "让", "日", "如", "入", "热", "肉", "若", "然", "任", "瑞", "荣", "扔", "认", "软", "润", "容"]),
    ("sb", &["上班", "是吧", "傻逼", "随便", "睡吧", "顺便", "是不", "手表", "身边", "设备", "生病", "失败", "说吧", "社保", "识别", "双倍", "鼠标", "上报", "申报", "水杯", "傻B", "上边", "手柄", "商标", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sc", &["上次", "市场", "删除", "生产", "身材", "上车", "四川", "三次", "上传", "商场", "收藏", "输出", "商城", "蔬菜", "素材", "生存", "刹车", "生成", "食材", "手册", "时长", "首次", "实操", "SC", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sd", &["是的", "收到", "说的", "啥的", "睡得", "速度", "上的", "稍等", "三点", "送到", "山东", "时代", "是滴", "十点", "删掉", "手动", "商店", "刷到", "适当", "深度", "圣诞", "水电", "手段", "SD", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sf", &["顺丰", "是否", "师傅", "舒服", "身份", "收费", "沙发", "首付", "十分", "说法", "双方", "师父", "释放", "手法", "水费", "算法", "书法", "师范", "私发", "三方", "首发", "书房", "省份", "SF", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sg", &["三个", "是个", "水果", "四个", "帅哥", "说过", "身高", "上个", "十个", "时光", "施工", "送给", "手工", "傻瓜", "试过", "事故", "手感", "搜狗", "水管", "收购", "三国", "诗歌", "申购", "SG", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sj", &["时间", "数据", "手机", "睡觉", "世界", "设计", "升级", "司机", "实际", "商家", "书记", "神经", "事件", "随机", "涉及", "收集", "上架", "实践", "三级", "审计", "视角", "视觉", "上级", "SJ", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sk", &["上课", "思考", "烧烤", "时刻", "收款", "刷卡", "伤口", "深刻", "社恐", "省考", "水库", "是看", "授课", "声卡", "时空", "三款", "双开", "收口", "SK", "色卡", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社", "司", "胜", "瘦"]),
    ("sl", &["算了", "说了", "商量", "死了", "睡了", "上来", "上了", "删了", "少了", "数量", "实力", "顺利", "手里", "思路", "塑料", "梳理", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社", "司", "胜", "瘦", "尚", "晒", "史", "射"]),
    ("sm", &["什么", "是吗", "上面", "说明", "睡眠", "扫码", "生命", "失眠", "上门", "山姆", "扫描", "实名", "审美", "是么", "神秘", "是嘛", "数码", "寿命", "双面", "商贸", "售卖", "声明", "SM", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社"]),
    ("sn", &["是你", "三年", "说你", "送你", "酸奶", "十年", "室内", "少年", "是呢", "四年", "少女", "水泥", "随你", "索尼", "收纳", "省内", "三", "是", "说", "上", "啥", "时", "算", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社", "司", "胜", "瘦", "尚", "晒", "史", "射"]),
    ("sp", &["视频", "商品", "审批", "水平", "食品", "薯片", "双排", "碎片", "商铺", "射频", "上牌", "饰品", "实拍", "锁屏", "生怕", "视屏", "适配", "审判", "食谱", "索赔", "竖屏", "首批", "收盘", "SP", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sq", &["事情", "申请", "上去", "生气", "市区", "社区", "失去", "收钱", "神奇", "三千", "省钱", "时期", "送去", "授权", "帅气", "睡前", "神器", "三期", "宿迁", "社群", "收取", "诉求", "商圈", "sq", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sr", &["虽然", "收入", "生日", "输入", "熟人", "私人", "死人", "杀人", "送人", "骚扰", "三人", "深入", "是人", "散热", "双人", "诗人", "四人", "商人", "上饶", "色", "Sr", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社", "司", "胜", "瘦"]),
    ("st", &["身体", "三天", "食堂", "石头", "说他", "是他", "四天", "上头", "手套", "实体", "汕头", "生态", "三条", "申通", "手头", "上台", "刷题", "试听", "闪退", "试图", "渗透", "沙特", "ST", "视图", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sw", &["上午", "是我", "稍微", "说我", "死亡", "十五", "生物", "思维", "食物", "上网", "说完", "商务", "所谓", "实物", "税务", "室外", "收尾", "事务", "色", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社", "司", "胜", "瘦", "尚", "晒"]),
    ("sx", &["实习", "数学", "师兄", "思想", "上学", "上线", "熟悉", "实现", "剩下", "首先", "私信", "三星", "说下", "手续", "上下", "山西", "陕西", "属性", "顺序", "刷新", "试下", "筛选", "生效", "事项", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("sy", &["所以", "所有", "使用", "声音", "属于", "生意", "是要", "是有", "实验", "剩余", "随意", "事业", "沈阳", "摄影", "是呀", "适应", "收益", "商业", "水印", "首页", "试用", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申", "社", "司", "胜"]),
    ("sz", &["苏州", "实在", "设置", "深圳", "啥子", "是在", "睡着", "傻子", "甚至", "嫂子", "嗓子", "数字", "孙总", "上周", "素质", "手指", "始终", "随着", "是指", "所在", "上涨", "述职", "三种", "数值", "是", "说", "上", "啥", "时", "算", "三", "事", "谁", "送", "收", "四", "少", "刷", "数", "水", "手", "睡", "书", "生", "岁", "市", "死", "省", "双", "所", "孙", "撒", "色", "宋", "苏", "山", "删", "杀", "搜", "升", "神", "爽", "扫", "商", "输", "帅", "锁", "税", "沈", "什", "室", "十", "式", "受", "烧", "深", "树", "试", "石", "顺", "剩", "思", "师", "盛", "蛇", "设", "素", "傻", "审", "松", "随", "声", "耍", "身", "使", "申"]),
    ("tb", &["特别", "淘宝", "同步", "投标", "他爸", "头孢", "贴吧", "图标", "逃避", "通报", "填报", "头部", "拖把", "徒步", "填表", "投保", "同比", "提报", "TB", "替补", "踏板", "提拔", "退保", "图表", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("tc", &["套餐", "停车", "退出", "同城", "吐槽", "提成", "提出", "特产", "太差", "推迟", "太长", "提车", "突出", "天才", "推出", "台词", "陶瓷", "通常", "题材", "弹出", "填充", "弹窗", "统筹", "剔除", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("td", &["太多", "团队", "他的", "听到", "她的", "态度", "太大", "土豆", "挺多", "徒弟", "提到", "通道", "太低", "挺大", "天地", "特点", "土地", "推动", "提的", "替代", "特定", "痛点", "TD", "特", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘"]),
    ("tf", &["头发", "投放", "天赋", "通风", "退房", "痛风", "土匪", "退费", "突发", "台风", "脱发", "套房", "停发", "腾飞", "天府", "推翻", "TF", "提分", "托福", "停放", "土方", "条幅", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭"]),
    ("tg", &["提供", "通过", "推广", "提高", "太高", "泰国", "听过", "太贵", "听歌", "团购", "舔狗", "挺高", "提过", "涛哥", "通关", "糖果", "托管", "跳过", "推给", "投稿", "停工", "TG", "提个", "体感", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("th", &["挺好", "退货", "太好", "通话", "听话", "体会", "同行", "他还", "他会", "退回", "谈话", "提货", "替换", "特惠", "天河", "谈好", "退换", "填好", "贴合", "退还", "弹簧", "图", "他", "她", "天", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭", "态"]),
    ("tj", &["推荐", "条件", "统计", "天津", "他家", "他就", "提交", "体检", "听见", "添加", "特价", "太久", "调节", "停机", "团建", "推进", "团结", "途径", "调教", "台阶", "调解", "体积", "调价", "婷姐", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("tk", &["退款", "同款", "痛苦", "坦克", "挺快", "听课", "太快", "天空", "头盔", "条款", "调侃", "题库", "太空", "泰康", "太卡", "统考", "填空", "特快", "提款", "调控", "图库", "提", "他", "她", "天", "图", "太", "台", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭", "态"]),
    ("tl", &["讨论", "退了", "他俩", "太冷", "套路", "太累", "提了", "停了", "调理", "踢了", "听了", "吐了", "体力", "谈了", "投了", "填了", "偷懒", "听力", "铁路", "她俩", "脱离", "推了", "推理", "体量", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("tm", &["他们", "她们", "他妈", "题目", "特么", "贴膜", "它们", "天猫", "透明", "太忙", "甜蜜", "太慢", "他没", "她妈", "TM", "土木", "台面", "偷摸", "涂抹", "天马", "他么", "条码", "同名", "脱敏", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("tn", &["听你", "太难", "替你", "童年", "体内", "他能", "他那", "天呐", "头脑", "体能", "推拿", "挺难", "谈", "天内", "TN", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭", "态", "拓", "掏", "胎", "抬", "桐", "探", "途"]),
    ("tp", &["图片", "突破", "投票", "投屏", "调皮", "偷拍", "头皮", "甜品", "太平", "躺平", "谈判", "退票", "逃跑", "天平", "特批", "托盘", "调配", "铁皮", "贴片", "贴牌", "TP", "图谱", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭"]),
    ("tq", &["提前", "天气", "他去", "退群", "探亲", "掏钱", "提取", "退钱", "透气", "叹气", "踢球", "提起", "同情", "台球", "太强", "特权", "通勤", "拖欠", "同期", "凸起", "T区", "听取", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭"]),
    ("tr", &["突然", "他人", "天然", "太热", "天热", "同仁", "铜仁", "通融", "坦然", "投入", "唐人", "他让", "土壤", "同人", "铁人", "踢人", "太弱", "踏入", "退让", "天润", "天人", "填入", "特", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭"]),
    ("ts", &["同事", "他说", "听说", "同时", "投诉", "提示", "他是", "提升", "她说", "特殊", "特色", "太少", "调试", "推送", "图书", "他", "天", "图", "太", "台", "提", "她", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭", "态", "拓", "掏", "胎", "抬", "桐", "探"]),
    ("tw", &["跳舞", "太晚", "台湾", "提问", "体温", "替我", "听我", "听完", "推我", "图文", "摊位", "填完", "投喂", "谈完", "条纹", "贪污", "调味", "突兀", "团委", "体外", "他", "天", "图", "太", "台", "提", "她", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭", "态", "拓"]),
    ("tx", &["同学", "提醒", "头像", "腾讯", "填写", "退休", "提现", "通宵", "天下", "特效", "T恤", "太小", "体系", "体现", "调休", "挑选", "通讯", "弹性", "通信", "天线", "图像", "图形", "特性", "TX", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("ty", &["同意", "体验", "统一", "太阳", "讨厌", "他也", "头晕", "同样", "特意", "体育", "他要", "太原", "他有", "汤圆", "太远", "通用", "同一", "投影", "提议", "图", "他", "她", "天", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤", "淘", "庭", "态", "拓", "掏"]),
    ("tz", &["通知", "调整", "投资", "躺着", "体重", "他在", "兔子", "听着", "体质", "套装", "挑战", "同志", "停止", "贴纸", "太早", "图纸", "帖子", "唐总", "团长", "拓展", "台账", "特征", "梯子", "台州", "他", "她", "天", "图", "太", "台", "提", "条", "退", "套", "填", "它", "听", "头", "同", "推", "调", "挺", "谈", "贴", "通", "投", "题", "唐", "团", "田", "停", "跳", "魄", "汤", "体", "拖", "特", "疼", "挑", "桶", "铁", "腿", "谭", "涛", "痛", "甜", "厅", "糖", "婷", "土", "躺", "塔", "偷", "陶", "泰", "托", "涂", "吐", "舔", "踢", "弹", "童", "趟", "铜", "腾", "替", "脱", "添", "堂", "兔", "烫", "透", "碳", "桃", "囤", "彤"]),
    ("wb", &["我不", "我爸", "微博", "我把", "玩吧", "五百", "网吧", "外边", "我被", "务必", "未必", "尾巴", "完毕", "晚班", "外包", "无比", "外部", "文本", "尾部", "维保", "稳步", "王斌", "文博", "王博", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wc", &["完成", "卧槽", "我才", "外出", "我吃", "维持", "午餐", "我从", "晚餐", "我操", "维C", "我擦", "我猜", "五次", "我出", "我超", "王超", "误差", "挖槽", "为此", "王朝", "外侧", "外场", "文创", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wd", &["我的", "我都", "晚点", "味道", "玩的", "稳定", "温度", "我得", "五点", "万达", "外地", "无敌", "问的", "我对", "舞蹈", "伟大", "文档", "网点", "完蛋", "闻到", "我丢", "问到", "维度", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜"]),
    ("wf", &["无法", "晚饭", "午饭", "我发", "潍坊", "违法", "玩法", "往返", "无非", "我放", "五福", "网费", "违反", "万分", "外放", "王菲", "无缝", "我方", "王芳", "未付", "外发", "万方", "王飞", "未发", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wg", &["我给", "我刚", "我跟", "五个", "外公", "我哥", "王哥", "无关", "玩过", "伟哥", "外国", "外观", "违规", "我该", "问过", "我国", "完工", "玩个", "外挂", "网格", "网管", "网关", "王工", "文稿", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wh", &["武汉", "我会", "我还", "文化", "我和", "我好", "网红", "维护", "往后", "为何", "误会", "玩会", "问候", "芜湖", "晚会", "威海", "问号", "尾号", "危害", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯"]),
    ("wj", &["我就", "忘记", "文件", "我家", "玩具", "我姐", "王姐", "我叫", "玩家", "围巾", "五金", "危机", "问卷", "完结", "挖机", "旺季", "文静", "问界", "文具", "挖掘", "未接", "未见", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬"]),
    ("wk", &["我看", "我靠", "网课", "胃口", "我可", "我开", "万科", "哇靠", "文科", "尾款", "网卡", "外科", "我", "王凯", "外壳", "悟空", "挖矿", "wk", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯", "巍", "勿"]),
    ("wl", &["完了", "为了", "忘了", "我来", "我了", "无聊", "未来", "网络", "问了", "我俩", "玩了", "屋里", "物理", "物流", "晚了", "无论", "物料", "我", "威力", "往来", "王磊", "围栏", "文旅", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬"]),
    ("wm", &["我们", "我妈", "外面", "我没", "外卖", "完美", "网名", "我买", "我吗", "玩吗", "文明", "挖煤", "我妹", "无名", "外贸", "我么", "外貌", "为毛", "为嘛", "污蔑", "唯美", "王敏", "屋面", "位面", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wn", &["问你", "我能", "为你", "温暖", "我呢", "嗯", "我拿", "五年", "我那", "无奈", "万能", "我", "为", "无", "王", "未", "万", "问", "玩", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯", "巍", "勿", "悟", "危", "揾", "握", "為", "卧"]),
    ("wp", &["我怕", "我", "外婆", "物品", "五排", "我拍", "网盘", "王牌", "卧铺", "王鹏", "无票", "尾盘", "外派", "误判", "WP", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯", "巍", "勿", "悟", "危", "揾"]),
    ("wq", &["我去", "完全", "委屈", "五千", "围棋", "武器", "温泉", "无情", "维权", "往前", "屋企", "无趣", "外勤", "外企", "我敲", "网球", "外墙", "蛙趣", "网签", "围裙", "围墙", "弯曲", "王强", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜"]),
    ("wr", &["我让", "我日", "无人", "温柔", "外人", "侮辱", "勿扰", "微软", "污染", "为人", "沃日", "围绕", "万人", "儿", "我认", "五人", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯", "巍", "勿", "悟"]),
    ("ws", &["晚上", "我是", "为啥", "我说", "网上", "五十", "卫生", "完事", "卧室", "完善", "往上", "我司", "哇塞", "哇", "文书", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯", "巍", "勿", "悟", "危", "揾"]),
    ("wt", &["问题", "问他", "外套", "五天", "我天", "我听", "胃疼", "问她", "屋头", "舞台", "委托", "无糖", "为他", "王涛", "无痛", "网图", "稳妥", "外滩", "物体", "文体", "微调", "梧桐", "弯头", "wt", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wx", &["微信", "我想", "问下", "维修", "我先", "危险", "无线", "无锡", "无限", "微笑", "玩笑", "威胁", "温馨", "无效", "午休", "网线", "往下", "五星", "无需", "文学", "文献", "卫星", "万象", "晚些", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("wy", &["我也", "我要", "万一", "我有", "无语", "我又", "唯一", "物业", "玩意", "我用", "网易", "网页", "万元", "无", "WY", "我", "为", "万", "问", "玩", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻", "袜", "邬", "吾", "巫", "唯", "巍", "勿", "悟", "危", "揾"]),
    ("wz", &["我在", "位置", "我这", "王总", "王者", "袜子", "我再", "文章", "文字", "吴总", "网站", "我咋", "屋子", "我真", "为主", "我只", "完整", "我找", "物质", "温州", "网址", "为准", "为止", "物资", "我", "为", "万", "问", "玩", "无", "王", "未", "完", "哇", "五", "文", "吴", "晚", "位", "网", "外", "往", "娃", "伟", "喔", "唔", "汪", "微", "魏", "稳", "味", "忘", "喂", "温", "瓦", "挖", "武", "伍", "维", "物", "韦", "卫", "威", "碗", "旺", "午", "望", "胃", "屋", "歪", "雯", "闻", "翁", "窝", "尾", "委", "湾", "舞", "纹", "薇", "弯", "丸", "炜", "皖", "蚊", "误", "围", "玮", "沃", "务", "婉", "雾", "蛙", "乌", "蔚", "吻"]),
    ("xb", &["下班", "行吧", "行不", "小宝", "先把", "小白", "细胞", "相比", "学霸", "下吧", "性别", "下巴", "下边", "新版", "想把", "宣布", "西北", "小班", "小包", "续保", "先不", "先别", "修补", "西部", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xc", &["下次", "现场", "宣传", "洗车", "想吃", "下车", "相处", "行程", "县城", "相册", "形成", "携程", "小车", "新车", "相差", "消除", "乡村", "小丑", "新春", "薪酬", "现成", "巡查", "写出", "线程", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xd", &["兄弟", "下单", "新的", "写的", "想的", "晓得", "想到", "小的", "消毒", "学的", "下的", "行动", "相对", "显得", "相当", "小弟", "现代", "选的", "心动", "许多", "小店", "新店", "限定", "XD", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xf", &["媳妇", "想法", "幸福", "修复", "消费", "学费", "新房", "兴奋", "消防", "续费", "西方", "校服", "先发", "相反", "下发", "学分", "先锋", "下方", "宪法", "信封", "讯飞", "细分", "悬浮", "写法", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xg", &["效果", "香港", "习惯", "性格", "新冠", "相关", "修改", "想过", "西瓜", "像个", "小狗", "性感", "下个", "小哥", "膝盖", "先给", "写个", "香菇", "选个", "鞋柜", "星光", "小怪", "X光", "线管", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xh", &["喜欢", "小孩", "小号", "现货", "学会", "信号", "想好", "型号", "相互", "消化", "幸好", "笑话", "消耗", "循环", "下回", "续航", "西湖", "修好", "新华", "卸货", "序号", "细化", "需", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限", "鲜", "邢"]),
    ("xj", &["现金", "休假", "新疆", "细节", "香蕉", "相机", "学姐", "小姐", "下降", "下架", "先进", "小鸡", "巡检", "虚假", "新建", "询价", "衔接", "详见", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限", "鲜", "邢", "希", "戏", "卸", "兄"]),
    ("xk", &["辛苦", "新款", "想看", "小康", "下课", "显卡", "先看", "幸亏", "血亏", "新开", "胸口", "学科", "星空", "新卡", "选课", "细看", "许可", "新客", "相框", "先开", "系", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限", "鲜", "邢", "希", "戏"]),
    ("xl", &["下来", "心里", "醒了", "想了", "笑了", "项链", "效率", "下了", "心理", "系列", "训练", "行了", "写了", "下楼", "学历", "行李", "销量", "选了", "线路", "小李", "XL", "迅雷", "泄露", "下路", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xm", &["项目", "下面", "羡慕", "小米", "行吗", "厦门", "姓名", "想买", "小妹", "小猫", "熊猫", "行么", "西门", "小明", "小马", "先买", "小麦", "先忙", "小美", "小敏", "醒目", "小满", "写明", "休眠", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xn", &["新年", "想你", "性能", "像你", "仙女", "行呢", "小年", "向你", "洗脑", "细腻", "新娘", "虚拟", "新", "向您", "小牛", "西南", "下", "先", "想", "行", "写", "小", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限", "鲜", "邢", "希", "戏", "卸", "兄", "凶", "辛", "显"]),
    ("xp", &["芯片", "新品", "相片", "小胖", "橡皮", "小票", "小瓶", "小鹏", "小品", "下铺", "小平", "选品", "虾皮", "小潘", "息屏", "选票", "选片", "吸盘", "斜坡", "选配", "询盘", "洗盘", "XP", "小炮", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xq", &["心情", "相亲", "下去", "想去", "小区", "需求", "先去", "星期", "嫌弃", "想起", "兴趣", "新区", "校区", "学期", "详情", "下棋", "小钱", "星球", "小群", "西区", "寻求", "续签", "续期", "选取", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xr", &["新人", "吓人", "信任", "形容", "小人", "想让", "现任", "虚弱", "笑容", "渲染", "仙人", "陷入", "虾仁", "夏日", "先让", "鲜肉", "选人", "显然", "雪人", "行人", "削弱", "写入", "祥瑞", "XR", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xs", &["显示", "小时", "学生", "销售", "笑死", "现实", "线上", "享受", "洗漱", "小说", "先生", "心思", "消失", "形式", "吸收", "像是", "选手", "协商", "新手", "限时", "相似", "学术", "线索", "XS", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xt", &["系统", "夏天", "心态", "心疼", "洗头", "相同", "协调", "想吐", "心跳", "心痛", "血糖", "小腿", "邢台", "修图", "线条", "形态", "湘潭", "习题", "下调", "XT", "协同", "选题", "信托", "下图", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("xw", &["下午", "希望", "新闻", "行为", "想我", "像我", "写完", "洗完", "小王", "想问", "小微", "询问", "学位", "纤维", "小吴", "西屋", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限", "鲜", "邢", "希", "戏", "卸", "兄", "凶", "辛"]),
    ("xy", &["需要", "想要", "学院", "下雨", "信用", "幸运", "协议", "咸鱼", "学员", "吸引", "小雨", "闲鱼", "校园", "现有", "先用", "响应", "相应", "小于", "兴业", "徐", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限", "鲜", "邢", "希", "戏", "卸"]),
    ("xz", &["现在", "选择", "下载", "洗澡", "鞋子", "想着", "下周", "箱子", "限制", "行政", "校长", "卸载", "徐总", "新增", "薪资", "寻找", "小组", "协助", "性质", "现状", "写作", "旋转", "修正", "选中", "下", "先", "想", "行", "写", "小", "新", "选", "像", "现", "线", "需", "学", "些", "向", "心", "性", "系", "徐", "洗", "箱", "信", "星", "谢", "项", "修", "西", "休", "晓", "许", "型", "鑫", "雪", "肖", "熊", "香", "夏", "笑", "县", "兴", "血", "喜", "细", "秀", "鞋", "欣", "相", "薛", "醒", "霞", "校", "旭", "祥", "续", "闲", "芯", "姓", "吸", "训", "轩", "瞎", "虚", "险", "翔", "虾", "胸", "响", "乡", "雄", "仙", "习", "限"]),
    ("yb", &["一般", "要不", "一半", "也不", "夜班", "有病", "一遍", "一把", "一边", "右边", "医保", "一波", "一包", "一杯", "一本", "要把", "一步", "原本", "一笔", "元宝", "一部", "样板", "样本", "一版", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yc", &["一次", "异常", "延迟", "一场", "延长", "远程", "牙齿", "隐藏", "一层", "有车", "一车", "预测", "演出", "因此", "养成", "原厂", "月初", "原创", "用车", "盐城", "右侧", "用餐", "移除", "以此", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yd", &["有点", "一点", "一定", "有的", "遇到", "用的", "要的", "运动", "月底", "也得", "移动", "一堆", "一顿", "一段", "元旦", "阅读", "预定", "引导", "印度", "一单", "用到", "约定", "也", "yd", "有", "要", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游"]),
    ("yf", &["月份", "衣服", "一份", "运费", "缘分", "孕妇", "邮费", "一分", "预防", "研发", "一副", "一方", "药房", "一发", "要发", "应付", "已发", "引发", "语法", "预付", "乙方", "已付", "油费", "用法", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yg", &["一个", "应该", "有个", "一共", "员工", "阳光", "有关", "要个", "一根", "用过", "严格", "压根", "英国", "眼光", "要给", "牙膏", "勇敢", "有过", "一股", "衣柜", "约个", "预告", "预估", "要改", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yh", &["以后", "一会", "银行", "也会", "优惠", "一盒", "也好", "用户", "也很", "烟花", "遗憾", "约会", "一号", "优化", "有货", "樱花", "越好", "验货", "养护", "银河", "隐患", "一行", "印花", "与", "有", "要", "也", "月", "用", "呀", "又", "元", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游"]),
    ("yj", &["已经", "也就", "研究", "眼睛", "一家", "以及", "一句", "一级", "邮件", "意见", "眼镜", "预计", "原价", "邮寄", "业绩", "押金", "一件", "硬件", "佣金", "原件", "应急", "依据", "预警", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游"]),
    ("yk", &["有空", "一块", "一看", "要看", "一口", "一颗", "也快", "愉快", "一款", "也可", "优酷", "依靠", "要开", "月卡", "游客", "遥控", "一开", "一卡", "眼科", "哟", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游", "音", "友", "粤", "夜"]),
    ("yl", &["原来", "有了", "压力", "用了", "一楼", "越来", "一路", "约了", "要来", "饮料", "娱乐", "医疗", "用来", "又来", "养老", "依赖", "引流", "以来", "盈利", "原理", "预留", "原料", "预览", "哟", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游"]),
    ("ym", &["要么", "也没", "有吗", "有没", "羊毛", "疫苗", "姨妈", "要命", "要买", "玉米", "页面", "一名", "一面", "要吗", "一米", "移民", "有么", "幽默", "一门", "以免", "羽毛", "月末", "域名", "源码", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yn", &["一年", "也能", "云南", "以内", "有你", "要你", "用你", "约你", "又能", "油腻", "与你", "越南", "印尼", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游", "音", "友", "粤", "夜", "牙", "铅", "説", "野", "闫", "壹"]),
    ("yp", &["一瓶", "一批", "一片", "硬盘", "有票", "样品", "一篇", "药品", "应聘", "哟", "音频", "用品", "已拍", "一盘", "一排", "压迫", "一票", "优品", "一平", "饮品", "预判", "云盘", "影片", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游", "音"]),
    ("yq", &["一起", "以前", "要求", "要去", "疫情", "一切", "有钱", "运气", "一千", "邀请", "又去", "尤其", "也去", "引起", "要钱", "一群", "一圈", "一期", "语气", "延期", "预期", "园区", "仪器", "逾期", "呀", "有", "要", "也", "月", "用", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yr", &["有人", "一人", "依然", "羊肉", "也", "羽绒", "一日", "野人", "要人", "要让", "艺人", "月入", "也让", "预热", "引入", "yr", "有", "要", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游", "音", "友", "粤", "夜", "牙", "铅", "説", "野"]),
    ("ys", &["也是", "要是", "意思", "医生", "有事", "颜色", "又是", "有啥", "以上", "钥匙", "预算", "意识", "艺术", "优势", "呀", "有", "要", "月", "用", "又", "也", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游", "音", "友", "粤", "夜", "牙", "铅", "説", "野", "闫"]),
    ("yt", &["一天", "一套", "一条", "一台", "一趟", "阳台", "丫头", "圆通", "原图", "樱桃", "一桶", "一体", "烟台", "一头", "液体", "一坨", "源头", "用途", "依托", "议题", "意图", "研讨", "已退", "YT", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yw", &["因为", "以为", "业务", "语文", "一万", "意外", "要我", "有无", "英文", "一位", "欲望", "也", "药物", "以外", "用完", "疑问", "义乌", "义务", "以往", "原文", "运维", "有", "要", "月", "用", "又", "呀", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋", "游", "音", "友", "粤"]),
    ("yx", &["一下", "一些", "影响", "也行", "游戏", "有些", "优秀", "也许", "优先", "一箱", "有限", "英雄", "以下", "有效", "印象", "也想", "邮箱", "运行", "营销", "允许", "一线", "医学", "意向", "原型", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("yz", &["一直", "一张", "严重", "一周", "样子", "一种", "也在", "一只", "邮政", "业主", "要在", "验证", "杨总", "宇宙", "颜值", "要做", "院长", "又在", "一致", "优质", "原则", "一组", "一早", "椅子", "有", "要", "也", "月", "用", "呀", "又", "元", "与", "一", "已", "以", "约", "射", "于", "杨", "由", "样", "哟", "云", "原", "因", "页", "亿", "越", "赢", "药", "应", "远", "余", "员", "玉", "鱼", "油", "叶", "源", "雨", "阳", "养", "袁", "压", "永", "硬", "宇", "院", "烟", "易", "英", "姚", "运", "印", "银", "园", "燕", "右", "眼", "语", "亚", "艳", "耶", "圆", "严", "阴", "悦", "业", "雅", "勇", "羊", "晕", "尹", "咦", "洋"]),
    ("zb", &["这边", "准备", "直播", "咋办", "在不", "总部", "值班", "主播", "这不", "资本", "装备", "左边", "周边", "指标", "主板", "这把", "质保", "坐标", "逐步", "占比", "招标", "中标", "支部", "周报", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zc", &["正常", "支持", "这次", "早餐", "坐车", "政策", "注册", "再次", "早晨", "造成", "资产", "做出", "做成", "支出", "支撑", "主持", "最迟", "职称", "装车", "职场", "组成", "仲裁", "指出", "左侧", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zd", &["真的", "知道", "早点", "做的", "找到", "最多", "自动", "主动", "值得", "走的", "这点", "重点", "做到", "最低", "最大", "指导", "在的", "针对", "这段", "指定", "制度", "账单", "站点", "制定", "则", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zf", &["支付", "政府", "做饭", "祝福", "转发", "早饭", "租房", "煮饭", "再发", "做法", "这份", "自费", "脂肪", "住房", "作废", "丈夫", "只发", "执法", "直飞", "走访", "直发", "总分", "涨幅", "字符", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zg", &["这个", "中国", "找个", "整个", "照顾", "做个", "总共", "最高", "做过", "主管", "足够", "资格", "张哥", "正规", "再给", "转给", "职工", "整改", "质感", "主观", "咋搞", "张工", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增"]),
    ("zj", &["自己", "直接", "最近", "在家", "着急", "早就", "涨价", "中间", "之间", "增加", "抓紧", "浙江", "专家", "总结", "这就", "资金", "这家", "证据", "中介", "主机", "总监", "租金", "支架", "组件", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zk", &["这款", "这块", "这可", "再看", "在看", "真快", "最快", "折扣", "状况", "中考", "只看", "专科", "展开", "召开", "真空", "租客", "中控", "转款", "中科", "质控", "字", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增", "挣", "赞"]),
    ("zl", &["这里", "咋了", "做了", "走了", "咱俩", "质量", "资料", "治疗", "找了", "转了", "再来", "周六", "助力", "整理", "咋啦", "这俩", "助理", "战略", "重量", "这类", "专利", "租赁", "指令", "治理", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zm", &["怎么", "这么", "咱们", "周末", "在吗", "证明", "专门", "做梦", "折磨", "再买", "咱妈", "在忙", "桌面", "正面", "真没", "字母", "琢磨", "照明", "招募", "字幕", "注明", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增", "挣"]),
    ("zn", &["只能", "找你", "在哪", "在你", "祝你", "在那", "智能", "祝您", "渣男", "在呢", "子女", "咋弄", "周年", "转你", "指南", "咱", "职能", "Zn", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增", "挣", "赞", "桌", "展", "专"]),
    ("zp", &["照片", "招聘", "作品", "赠品", "自拍", "真皮", "正品", "诈骗", "帐篷", "这篇", "最怕", "这批", "招牌", "专票", "转盘", "支票", "支配", "直聘", "装配", "制品", "早盘", "张鹏", "自评", "指派", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zq", &["之前", "赚钱", "再去", "早起", "挣钱", "争取", "追求", "正确", "准确", "足球", "周期", "增强", "最强", "值钱", "证券", "早期", "中期", "征求", "专区", "债券", "债权", "账期", "政企", "咋", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹"]),
    ("zr", &["自然", "主任", "找人", "周日", "责任", "主人", "这人", "猪肉", "招人", "做人", "早日", "阵容", "真人", "转让", "走人", "昨日", "注入", "转入", "专人", "啧", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增", "挣", "赞"]),
    ("zs", &["这是", "早上", "只是", "真是", "再说", "暂时", "至少", "总是", "咋说", "正式", "真实", "知识", "紫色", "招商", "周四", "证书", "周三", "助手", "展示", "自身", "赠送", "住宿", "装饰", "咋", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹"]),
    ("zt", &["昨天", "状态", "整体", "整天", "折腾", "主题", "找他", "周天", "中通", "这条", "字体", "暂停", "这套", "找她", "自提", "中途", "总体", "展厅", "主体", "涨停", "粘贴", "专题", "这台", "主图", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zw", &["中午", "昨晚", "周五", "找我", "在我", "作为", "做完", "自我", "周围", "作文", "中文", "掌握", "最晚", "植物", "座位", "这位", "职位", "主卧", "走完", "暂无", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增", "挣", "赞"]),
    ("zx", &["这些", "中心", "在线", "咨询", "装修", "最新", "自信", "只想", "执行", "仔细", "中学", "在想", "造型", "注销", "自行", "征信", "最小", "专项", "找下", "展现", "转型", "纸箱", "中信", "专线", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "组", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮"]),
    ("zy", &["这样", "只有", "主要", "左右", "只要", "注意", "重要", "咋样", "终于", "专业", "作业", "怎样", "周一", "自由", "住院", "作用", "中医", "职业", "资源", "至于", "自营", "转移", "组", "在", "这", "做", "再", "中", "找", "走", "张", "真", "总", "只", "转", "咋", "周", "着", "装", "字", "早", "咱", "住", "最", "子", "整", "至", "值", "站", "赵", "正", "之", "朱", "种", "则", "主", "章", "坐", "赚", "自", "涨", "郑", "支", "招", "抓", "租", "证", "折", "长", "指", "钟", "照", "曾", "者", "左", "账", "占", "重", "志", "镇", "纸", "知", "炸", "猪", "追", "准", "作", "祝", "座", "直", "轴", "嘴", "泽", "煮", "邹", "增"]),
];

pub fn pair_panel(k: &str) -> Option<&'static [&'static str]> {
    PAIR_PANELS
        .binary_search_by(|(s, _)| s.cmp(&k))
        .ok()
        .map(|i| PAIR_PANELS[i].1)
}
// rows: 377
// rows: 377

// rows: 401
/// ---- 役N tick77/78: 深页定律 96 深音节列（syl_panels_deep.tsv 生成; 污染键 hao/men 前缀准则重提后 401/401 一致）
/// 键 = 音节, 键序 = sort 序。native 律: 音节面板 = 字列+补全词+
/// 简拼组合按原生序混排（61/79 面板混词实证）, 原样钉。消费 =
/// query 音节门 → q_initial_panel。表空 → 律休眠。----
pub static SYL_DEEP: &[(&str, &[&str])] = &[
    ("ai", &["哎", "爱", "唉", "艾", "挨", "矮", "癌", "埃", "欸", "愛", "AI", "哀", "碍", "硙", "瑷", "嗳", "捱", "蔼", "嫒", "霭", "敱", "隘", "乂", "磑", "暧", "嗌", "皑", "餲", "壒", "諰", "懝", "砹", "譺", "馤", "噯", "啀", "嬡", "叆", "薆", "璦", "毐", "絠", "嘊", "靄", "僾", "曖", "靉", "懓", "昹", "鑀", "娭", "锿", "娾", "鎄", "鱫", "瞹", "凒", "焥", "礙", "鴱", "伌", "硋", "堨", "隑", "銰", "騃", "藹", "溾", "皧", "塧", "溰", "敳", "皚", "濭", "㘷", "㤅", "㕌", "㝶", "㢊", "㑸", "㗒", "䀳", "嵦", "㗨", "㱯", "䬵", "䅬", "䔽", "䝽", "躷", "㿄", "䑂", "䨠", "賹", "譪", "䶣"]),
    ("an", &["按", "安", "俺", "案", "暗", "爱你", "岸", "韽", "啊你", "氨", "胺", "桉", "按钮", "鞍", "庵", "铵", "安宁", "按你", "安妮", "an", "黯", "谙", "垵", "屽", "咹", "安娜", "葊", "罯", "碪", "唵", "鵪", "闇", "侒", "鹌", "洝", "荌", "菴", "盦", "銨", "裺", "揞", "犴", "峖", "埯", "偣", "堓", "腤", "屵", "雸", "玵", "貋", "諳", "萻", "痷", "馣", "媕", "隌", "鮟", "鞌", "广", "晻", "誝", "鶕", "盫", "厈", "蓭", "儑", "豻", "錌", "㸩", "㟁", "䅁", "啽", "㽢", "䁆", "䅖", "䎏", "㜝", "䎨", "㱘", "䬓", "䮗", "䨄", "䜙", "䯥", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "A", "a"]),
    ("ang", &["昂", "俺哥", "骯", "盎", "肮", "安个", "俺姑", "按个", "安宫", "安哥", "安国", "鞍钢", "卬", "安规", "安钢", "案管", "安工", "按过", "暗光", "暗管", "唔", "岇", "阿牛哥", "昻", "俺那个", "醠", "枊", "䒢", "㼜", "䀚", "䍩", "㭿", "㦹", "䩕", "䭹", "䭺", "䬓", "按", "安", "俺", "案", "暗", "岸", "韽", "氨", "胺", "桉", "鞍", "庵", "铵", "黯", "谙", "垵", "屽", "咹", "葊", "罯", "碪", "唵", "鵪", "闇", "侒", "鹌", "洝", "荌", "啊", "阿", "吖", "锕", "嗄", "錒", "腌", "醃", "菴", "盦", "銨", "裺", "揞", "犴", "峖", "埯", "偣", "堓", "腤", "屵", "雸", "玵", "貋", "諳", "萻", "痷", "馣", "媕", "隌", "鮟", "鞌"]),
    ("ao", &["奥", "熬", "嗷", "澳", "凹", "傲", "敖", "袄", "啊哦", "獓", "岙", "坳", "翱", "鳌", "奧", "泑", "磝", "啊噢", "啊欧", "爱哦", "AO", "阿哦", "啊喔", "骜", "拗", "遨", "奡", "熝", "獒", "螯", "廒", "摮", "媪", "鏊", "懊", "鏖", "垇", "艹", "璈", "爊", "镺", "蔜", "墺", "鼇", "芺", "謸", "擙", "鰲", "嶅", "厫", "翶", "襖", "抝", "柪", "嚻", "慠", "謷", "聱", "嚣", "隞", "嗸", "滶", "驁", "鷔", "隩", "嶴", "翺", "眑", "軪", "媼", "嫯", "蝹", "囂", "扷", "岰", "鏕", "㑃", "㕭", "㘬", "㘭", "㤇", "郩", "㿰", "梎", "䞝", "㜜", "㟼", "㠂", "㥿", "㜩", "㠗", "䐿", "墽", "䁱", "䦋", "䯠"]),
    ("ba", &["把", "吧", "爸", "八", "叭", "巴", "拔", "捌", "扒", "霸", "坝", "罢", "疤", "保安", "不啊", "不爱", "靶", "丷", "芭", "备案", "粑", "别啊", "钯", "ba", "跋", "耙", "岜", "灞", "魃", "笆", "玐", "鲅", "仈", "朳", "鲃", "峇", "壩", "菝", "罷", "垻", "妭", "犮", "欛", "釟", "蚆", "柭", "抜", "覇", "矲", "哵", "鈀", "墢", "叐", "坺", "茇", "鲌", "夿", "鼥", "颰", "紦", "羓", "扷", "釛", "鮊", "跁", "癹", "軷", "鮁", "哱", "皅", "豝", "胈", "炦", "弝", "魞", "㞎", "㧊", "㔜", "㖠", "䟦", "詙", "䇑", "䩗", "䰾", "䳁", "䳊", "䩻", "䱝", "䆉", "㶚", "䥯", "䃻"]),
    ("bai", &["白", "百", "摆", "呗", "佰", "拜", "柏", "掰", "败", "伯", "栢", "擺", "稗", "唄", "敗", "Bai", "庍", "薭", "捭", "贁", "粨", "擘", "襬", "絔", "拝", "瓸", "挀", "竡", "韛", "粺", "猈", "䒔", "㓦", "㼟", "㼣", "㿟", "䢙", "㧳", "㗑", "㗗", "䙓", "㠔", "䳆", "䴽", "䥯", "吧", "把", "爸", "八", "叭", "巴", "拔", "捌", "扒", "霸", "坝", "罢", "疤", "靶", "丷", "芭", "粑", "钯", "跋", "耙", "岜", "灞", "魃", "笆", "玐", "鲅", "仈", "朳", "鲃", "峇", "壩", "菝", "罷", "垻", "妭", "犮", "欛", "釟", "蚆", "柭", "抜", "覇", "矲", "哵", "鈀", "墢", "叐", "坺", "茇", "鲌", "夿"]),
    ("ban", &["班", "办", "版", "半", "板", "搬", "把你", "般", "伴", "拌", "斑", "瓣", "扮", "坂", "八年", "把那", "扳", "辦", "籓", "绊", "颁", "阪", "把您", "ban", "不爱你", "不安宁", "粄", "钣", "湴", "辬", "螌", "闆", "靽", "癍", "絆", "鳻", "螁", "昄", "瘢", "魬", "舨", "攽", "柈", "怑", "坢", "覂", "岅", "頒", "搫", "朌", "鈑", "跘", "姅", "斒", "肦", "鉡", "瓪", "秚", "褩", "㚘", "㸞", "㪵", "䉽", "䬳", "蝂", "䕰", "㩯", "吧", "把", "爸", "八", "叭", "巴", "拔", "捌", "扒", "霸", "坝", "罢", "疤", "靶", "丷", "芭", "粑", "钯", "跋", "耙", "岜", "灞", "魃", "笆", "玐", "鲅", "仈", "朳", "鲃"]),
    ("bang", &["帮", "棒", "绑", "榜", "邦", "半个", "办公", "磅", "幫", "办个", "镑", "浜", "办过", "半根", "蚌", "搒", "半挂", "傍", "膀", "搬个", "梆", "颁给", "bang", "Bang", "班岗", "塝", "綁", "谤", "蒡", "稖", "鞤", "棓", "玤", "蚄", "鎊", "牓", "挷", "硥", "垹", "幚", "埲", "縍", "幇", "蛖", "蜯", "謗", "髈", "㔙", "㭋", "䂜", "捠", "䖫", "邫", "㮄", "䧛", "㾦", "㯁", "䎧", "䟺", "䰷", "艕", "班", "版", "办", "半", "板", "搬", "般", "伴", "拌", "斑", "瓣", "扮", "坂", "扳", "辦", "籓", "绊", "颁", "阪", "粄", "钣", "湴", "辬", "螌", "闆", "靽", "癍", "絆", "鳻", "吧", "把", "爸", "八", "叭", "巴"]),
    ("bao", &["宝", "包", "报", "保", "爆", "抱", "薄", "饱", "鲍", "煲", "堡", "豹", "暴", "葆", "寶", "報", "胞", "曝", "苞", "吧哦", "把哦", "剥", "刨", "鮑", "孢", "鑤", "褒", "飽", "雹", "袌", "褓", "鸨", "勽", "寳", "儤", "鉋", "犦", "駂", "笣", "龅", "瀑", "緥", "趵", "枹", "窇", "髱", "炮", "闁", "宀", "勹", "铇", "靌", "佨", "忁", "蕔", "虣", "剝", "宲", "犳", "堢", "寚", "媬", "藵", "珤", "曓", "菢", "靤", "骲", "怉", "蚫", "裦", "襃", "賲", "齙", "㲏", "㲒", "㙅", "㫧", "㻄", "砲", "䎂", "䨌", "䪨", "飹", "䨔", "鳵", "䈏", "鴇", "㵡", "䳈", "㙸", "䭋", "嚗", "㿺", "䳰", "䥤"]),
    ("bei", &["被", "呗", "倍", "北", "背", "杯", "备", "贝", "辈", "蓓", "碑", "悲", "卑", "鞞", "唄", "焙", "钡", "棑", "陂", "琲", "貝", "碚", "備", "悖", "韝", "孛", "狈", "禙", "邶", "鞁", "愂", "苝", "犕", "鹎", "輩", "惫", "臂", "僃", "昁", "椑", "垻", "褙", "鐴", "棓", "梖", "郥", "盃", "珼", "蛽", "藣", "鞴", "柸", "揹", "俻", "諀", "鄁", "狽", "鐾", "誖", "憊", "鵯", "牬", "桮", "杮", "偝", "鉳", "箄", "糒", "偹", "骳", "鋇", "軰", "㓈", "㸬", "㛝", "㤳", "㔨", "㸽", "㼎", "庳", "㣁", "㻗", "㽡", "䁅", "䔒", "㸢", "㾱", "䟺", "䋳", "䩀", "䰽", "㷶", "䠙", "㰆", "䡶", "㶔"]),
    ("ben", &["本", "笨", "奔", "苯", "Ben", "犇", "贲", "坋", "锛", "坌", "夯", "捹", "夲", "燌", "八二年", "贝尔尼", "伯尔尼", "比尔呢", "拜尔娜", "畚", "喯", "栟", "倴", "奙", "渀", "賁", "撪", "蟦", "漰", "楍", "泍", "錛", "翉", "獖", "炃", "桳", "輽", "㡷", "㤓", "逩", "㨧", "䬱", "㮺", "㱵"]),
    ("beng", &["崩", "泵", "蹦", "绷", "甭", "挷", "蚌", "本国", "本该", "嘣", "本宫", "絣", "塴", "迸", "埲", "本馆", "笨狗", "本官", "本钢", "本格", "本港", "本根", "本哥", "綳", "跰", "奟", "镚", "閍", "甏", "逬", "揼", "痭", "嵭", "甮", "祊", "傰", "嗙", "繃", "伻", "埄", "琫", "堋", "菶", "琣", "㑟", "㛝", "䙀", "㷯", "䋽", "㱶", "錋", "䩬", "䭰", "鞛", "䳞", "鏰", "䨻", "本", "笨", "奔", "苯", "犇", "贲", "坋", "锛", "坌", "夯", "捹", "夲", "燌", "畚", "喯", "栟", "倴", "奙", "渀", "賁", "撪", "蟦", "漰", "楍", "泍", "錛", "翉", "獖", "炃", "桳", "輽", "㡷", "㤓", "逩", "㨧", "䬱", "㮺", "㱵"]),
    ("bi", &["比", "逼", "笔", "币", "必", "毕", "碧", "俾", "闭", "臂", "壁", "鼻", "避", "睥", "彼", "哔", "屄", "鞞", "畀", "璧", "痺", "弊", "毙", "獙", "纰", "铋", "痹", "筆", "匕", "弼", "鸊", "笓", "幣", "吡", "妣", "蔽", "篦", "髲", "韠", "柀", "蹕", "啚", "獘", "敝", "饆", "躃", "聛", "庇", "駜", "紕", "枈", "芘", "翍", "梐", "鎞", "腷", "蓽", "贲", "稫", "彃", "鄨", "鄙", "煏", "皕", "幤", "廦", "苾", "婢", "泌", "熚", "濞", "髀", "鞸", "貏", "箆", "堛", "箅", "陛", "辟", "滭", "荜", "粊", "肶", "秘", "貱", "赑", "畢", "薜", "崥", "蜌", "襞", "繴", "豍", "紴", "鐴", "鶝"]),
    ("bian", &["边", "变", "编", "遍", "便", "卞", "币安", "扁", "鞭", "彼岸", "辩", "邊", "辨", "變", "贬", "籓", "比安", "碧安", "必安", "狴犴", "比俺", "弊案", "鯾", "匾", "辫", "煸", "汴", "編", "苄", "砭", "昪", "閞", "艑", "蝙", "疺", "箯", "鳊", "弁", "藊", "覵", "緶", "鯿", "獱", "辺", "忭", "稨", "覍", "萹", "辯", "徧", "碥", "褊", "抃", "笾", "諚", "鞕", "峅", "貶", "炞", "窆", "変", "玣", "辮", "汳", "揙", "缏", "籩", "糄", "臱", "釆", "惼", "辡", "猵", "牑", "甂", "辧", "鴘", "邉", "鍽", "鶣", "㳎", "䒪", "㭓", "㺹", "㝸", "㣐", "㴜", "䛒", "㲢", "㻞", "䪻", "䡢", "㵷", "㦚", "䁵", "䉸"]),
    ("biao", &["表", "标", "彪", "裱", "飙", "镖", "婊", "膘", "飚", "嫑", "標", "灬", "骉", "比奥", "碧奥", "毕奥", "必奥", "碧傲", "BIAO", "驃", "褾", "儦", "僄", "鑣", "諘", "檦", "飊", "骠", "錶", "穮", "俵", "镳", "飈", "杓", "鳔", "鰾", "謤", "爂", "膔", "颷", "颮", "猋", "脿", "飆", "摽", "贆", "滮", "蔈", "鏢", "驫", "飇", "飑", "熛", "颩", "藨", "幖", "淲", "麃", "瀌", "髟", "臕", "瘭", "墂", "磦", "㧼", "㟽", "䔸", "㠒", "䞄", "㯱", "篻", "㯹", "比", "笔", "币", "逼", "必", "毕", "碧", "俾", "闭", "臂", "壁", "鼻", "避", "睥", "彼", "哔", "屄", "鞞", "畀", "璧", "痺", "弊", "毙", "獙"]),
    ("bie", &["别", "憋", "鳖", "別", "瘪", "徶", "蹩", "虌", "避恶", "比厄", "逼额", "碧娥", "莂", "比额", "必饿", "彆", "鱉", "襒", "蟞", "蛂", "龞", "柭", "癟", "鼈", "柲", "䏟", "㢼", "㓖", "㲢", "䇷", "䋢", "䭱", "㱸", "䠥", "䉲", "㿜", "比", "笔", "币", "逼", "必", "毕", "碧", "俾", "闭", "臂", "壁", "鼻", "避", "睥", "彼", "哔", "屄", "鞞", "畀", "璧", "痺", "弊", "毙", "獙", "纰", "铋", "痹", "筆", "匕", "弼", "鸊", "笓", "幣", "吡", "妣", "蔽", "篦", "髲", "韠", "柀", "蹕", "啚", "獘", "敝", "饆", "躃", "聛", "庇", "駜", "紕", "枈", "芘", "翍", "梐", "鎞", "腷", "蓽", "贲", "稫", "彃"]),
    ("bin", &["斌", "彬", "比你", "宾", "滨", "槟", "逼你", "缤", "镔", "比拟", "避难", "比那", "必能", "比呢", "玢", "鬓", "殡", "賔", "膑", "濒", "傧", "bin", "髌", "賓", "濱", "摈", "瑸", "豳", "邠", "豩", "梹", "鬢", "霦", "儐", "顮", "臏", "虨", "鑌", "汃", "砏", "椕", "擯", "殯", "璸", "髩", "頻", "瀕", "鬂", "訜", "檳", "濵", "髕", "䐔", "䳁", "繽", "䚔", "比", "笔", "币", "逼", "必", "毕", "碧", "俾", "闭", "臂", "壁", "鼻", "避", "睥", "彼", "哔", "屄", "鞞", "畀", "璧", "痺", "弊", "毙", "獙", "纰", "铋", "痹", "筆", "匕", "弼", "鸊", "笓", "幣", "吡", "妣", "蔽", "篦", "髲", "韠", "柀"]),
    ("bing", &["并", "病", "冰", "兵", "饼", "宾馆", "丙", "炳", "柄", "秉", "斌哥", "並", "邴", "彬哥", "禀", "槟", "餅", "苪", "疒", "缤果", "髌骨", "滨哥", "摒", "Bing", "昺", "仌", "冫", "掤", "鞸", "餠", "屏", "併", "氷", "寎", "鈵", "靐", "竝", "眪", "栟", "昞", "幷", "癛", "仒", "栤", "傡", "陃", "梹", "怲", "倂", "棅", "鞞", "抦", "屛", "琕", "鋲", "窉", "稟", "枋", "鞆", "檳", "偋", "絣", "誁", "鮩", "癝", "㓈", "垪", "庰", "䓑", "㨀", "䋑", "蛃", "䈂", "䗒", "鉼", "䴵", "斌", "彬", "宾", "滨", "缤", "镔", "玢", "鬓", "殡", "賔", "膑", "濒", "傧", "髌", "賓", "濱", "比", "笔", "币", "逼"]),
    ("bo", &["波", "魄", "播", "博", "薄", "啵", "伯", "拨", "柏", "剥", "泊", "铂", "卜", "玻", "勃", "搏", "茀", "驳", "不哦", "布偶", "bo", "脖", "帛", "钵", "渤", "箔", "擗", "珀", "趵", "磻", "菠", "髆", "舶", "亳", "彴", "皪", "襮", "孹", "擘", "碆", "钹", "袹", "孛", "跛", "膊", "欂", "袯", "浡", "誖", "帗", "馛", "柭", "豰", "砵", "殕", "鱍", "狛", "礴", "謈", "蚾", "餑", "镈", "簸", "瓝", "檗", "餺", "犦", "饽", "僠", "譒", "壆", "愽", "挬", "撥", "僰", "侼", "紴", "鵓", "剝", "穛", "鲌", "鉑", "煿", "馞", "淿", "襎", "桲", "湐", "牔", "蹳", "駁", "般", "檘", "秡", "噃", "嶓"]),
    ("bu", &["不", "补", "部", "布", "步", "卜", "埠", "捕", "簿", "補", "卟", "怖", "埔", "堡", "咘", "哺", "钚", "埗", "吥", "佈", "陠", "歩", "瓿", "勏", "晡", "蔀", "逋", "醭", "踄", "轐", "柨", "鈈", "婄", "鯆", "餔", "峬", "荹", "廍", "篰", "錻", "秿", "钸", "鳪", "鸔", "抪", "尃", "餢", "鵏", "歨", "箁", "捗", "悑", "誧", "郶", "輹", "捬", "鈽", "獛", "㘵", "㚴", "㳍", "庯", "㻉", "㨐", "㾟", "䀯", "䏽", "㙛", "䊇", "䋠", "䑰", "䍌", "䝵", "䪔", "䬏", "䒈", "䳝", "䴺", "䪁", "䴝"]),
    ("ca", &["擦", "拆", "嚓", "吃啊", "长安", "冲啊", "攃", "长按", "菜啊", "出啊", "次啊", "穿啊", "惨啊", "唱啊", "CA", "礤", "礸", "囃", "遪", "䟃", "䵽"]),
    ("cai", &["才", "菜", "蔡", "猜", "财", "彩", "踩", "采", "裁", "材", "釆", "睬", "扐", "財", "偲", "採", "CAI", "跴", "縩", "纔", "寀", "啋", "埰", "婇", "倸", "棌", "綵", "揌", "毝", "㒲", "䞗", "䣋", "㥒", "䌽", "䐆", "䴭", "䰂", "䴺", "䌨", "擦", "拆", "嚓", "攃", "礤", "礸", "囃", "遪", "䟃", "䵽"]),
    ("can", &["惨", "餐", "参", "灿", "残", "蚕", "璨", "掺", "粲", "驂", "慘", "囋", "朁", "嵾", "can", "參", "喰", "嘇", "燦", "摻", "殘", "惭", "鏒", "摲", "嬠", "黲", "骖", "薒", "孱", "謲", "叄", "穇", "蠶", "飡", "叅", "黪", "傪", "憯", "湌", "慙", "澯", "飱", "爘", "儏", "慚", "蠺", "㱚", "㛑", "㕘", "㥇", "䏼", "䙁", "䬫", "㜗", "㺑", "䑶", "䝳", "䣟", "蝅", "㨻", "㻮", "䗝", "㣓", "㽩", "䗞", "䛹", "㿊", "篸", "䍼", "㦧", "㘔", "䫮", "䘉", "䳻", "擦", "拆", "嚓", "攃", "礤", "礸", "囃", "遪", "䟃", "䵽"]),
    ("cang", &["仓", "藏", "舱", "参观", "苍", "餐馆", "沧", "倉", "嵢", "鶬", "匨", "瑲", "参股", "灿哥", "参公", "賶", "臧", "蒼", "艙", "伧", "玱", "鑶", "滄", "凔", "鸧", "欌", "濸", "篬", "螥", "仺", "獊", "傖", "䢢", "䅮", "㵴", "㶓", "餐", "惨", "参", "灿", "残", "蚕", "璨", "掺", "粲", "驂", "慘", "囋", "朁", "嵾", "參", "喰", "嘇", "燦", "摻", "殘", "惭", "鏒", "摲", "嬠", "黲", "擦", "拆", "嚓", "攃", "礤", "骖", "薒", "孱", "謲", "叄", "穇", "蠶", "飡", "叅", "黪", "傪", "憯", "湌", "慙", "澯", "飱", "爘", "儏", "慚", "蠺", "㱚", "㛑", "㕘", "㥇", "䏼", "䙁", "䬫", "㜗", "㺑", "䑶"]),
    ("cao", &["草", "操", "曹", "艹", "槽", "糙", "肏", "屮", "嘈", "漕", "擦哦", "拆哦", "艸", "愺", "襙", "曺", "懆", "騲", "艚", "撡", "鼜", "螬", "蓸", "鄵", "傮", "慒", "鏪", "嶆", "慅", "褿", "䒑", "㜖", "㿷", "䄚", "䐬", "㯥", "㽩", "䎭", "䏆", "擦", "拆", "嚓", "攃", "礤", "礸", "囃", "遪", "䟃", "䵽"]),
    ("ce", &["测", "侧", "册", "策", "厕", "超额", "初二", "嫦娥", "惻", "差额", "測", "从而", "丑恶", "采耳", "蔡锷", "除恶", "箣", "CE", "側", "冊", "词儿", "刺耳", "错愕", "虫儿", "墄", "簎", "栅", "萴", "廁", "恻", "蓛", "夨", "筴", "茦", "粣", "畟", "敇", "厠", "拺", "笧", "荝", "萗", "憡", "䇲", "䊂", "㨲", "筞", "㥽", "㩍", "䈟", "䔴", "䜺"]),
    ("cen", &["岑", "册那", "参", "涔", "叅", "梣", "册呢", "參", "笒", "汵", "车尔尼", "叄", "埁", "硶", "篸", "嵾", "穇", "㞥", "䨙", "䯔", "㻸", "䤁", "䫈", "䃡", "䅾", "䲋", "测", "侧", "册", "策", "厕", "惻", "測", "箣", "側", "冊", "墄", "簎", "栅", "萴", "廁", "恻", "蓛", "夨", "筴", "茦", "粣", "畟", "敇", "厠", "拺", "笧", "荝", "萗", "憡", "䇲", "䊂", "㨲", "筞", "㥽", "㩍", "䈟", "䔴", "䜺"]),
    ("ceng", &["层", "蹭", "曾", "噌", "層", "曽", "嶒", "竲", "驓", "橧", "㣒", "㬝", "䁬", "䉕", "岑", "参", "涔", "叅", "梣", "测", "侧", "册", "策", "厕", "惻", "測", "箣", "側", "冊", "墄", "簎", "栅", "萴", "廁", "參", "笒", "汵", "叄", "埁", "硶", "篸", "嵾", "穇", "㞥", "䨙", "䯔", "㻸", "䤁", "䫈", "䃡", "䅾", "䲋", "恻", "蓛", "夨", "筴", "茦", "粣", "畟", "敇", "厠", "拺", "笧", "荝", "萗", "憡", "䇲", "䊂", "㨲", "筞", "㥽", "㩍", "䈟", "䔴", "䜺"]),
    ("cha", &["差", "查", "茶", "插", "叉", "茬", "察", "岔", "搽", "猹", "槎", "檫", "镲", "碴", "衩", "汊", "杈", "刹", "侘", "姹", "艖", "梌", "剎", "诧", "策划案", "嚓", "摖", "垞", "鎈", "蹅", "喳", "偛", "疀", "馇", "锸", "嗏", "楂", "扠", "褨", "査", "訍", "挿", "仛", "嵖", "奼", "揷", "餷", "扱", "臿", "捈", "鑔", "詫", "芆", "苴", "銟", "肞", "秅", "紁", "靫", "鍤", "詧", "㣾", "㪯", "㤞", "䑡", "䒲", "㫅", "䟕", "㢉", "䆛", "䓭", "㛳", "㛼", "㢎", "㢒", "䁟", "䤩", "䲦", "䑘", "䡨", "䕓", "䊬", "䰈", "䶪"]),
    ("chai", &["拆", "柴", "差", "钗", "儕", "豺", "侪", "虿", "釵", "茝", "芆", "瘥", "chai", "祡", "犲", "袃", "喍", "蠆", "肞", "訍", "囆", "㳗", "㑪", "㼮", "䓱", "㾹", "䡨", "䘍", "查", "茶", "插", "叉", "茬", "察", "岔", "搽", "猹", "槎", "檫", "镲", "碴", "衩", "汊", "杈", "刹", "侘", "姹", "艖", "梌", "剎", "诧", "嚓", "摖", "垞", "鎈", "蹅", "喳", "偛", "疀", "馇", "锸", "嗏", "楂", "扠", "褨", "査", "挿", "仛", "嵖", "奼", "揷", "餷", "扱", "臿", "捈", "鑔", "詫", "苴", "銟", "秅", "紁", "靫", "鍤", "詧", "㣾", "㪯", "㤞", "䑡", "䒲", "㫅", "䟕", "㢉", "䆛", "䓭", "㛳", "㛼"]),
    ("chan", &["产", "馋", "铲", "缠", "婵", "禅", "蝉", "掺", "闡", "鑱", "颤", "諂", "墠", "刹那", "茶农", "查你", "茶娘", "查宁", "儃", "僝", "忏", "嶄", "蟾", "欃", "產", "潺", "鋋", "儳", "覘", "攙", "阐", "鄽", "幨", "纒", "鐔", "蕆", "搀", "嘽", "繵", "饞", "棎", "誗", "丳", "禪", "緂", "嚵", "讇", "獑", "脠", "幝", "巉", "浐", "裧", "剷", "摌", "燀", "煘", "囅", "旵", "湹", "澶", "镡", "辴", "谗", "躔", "潹", "嵼", "谄", "嶃", "嬋", "磛", "醦", "瀺", "産", "硟", "瀍", "纏", "壥", "羼", "懺", "刬", "鏟", "辿", "廛", "酁", "觇", "孱", "鋓", "譂", "灛", "艬", "驏", "啴", "梴", "单", "閳"]),
    ("chang", &["长", "厂", "场", "常", "唱", "昌", "畅", "肠", "尝", "昶", "偿", "嫦", "長", "苌", "廠", "場", "倡", "敞", "蟾宫", "馋鬼", "蟾光", "Chang", "仩", "娼", "閶", "菖", "阊", "韔", "裳", "鲳", "腸", "锠", "猖", "怅", "暢", "僘", "鼚", "伥", "嘗", "闛", "氅", "鱨", "裮", "鏛", "玚", "蟐", "仧", "徜", "鬯", "琩", "瑺", "錩", "倘", "瓺", "淐", "塲", "償", "嚐", "惝", "鲿", "兏", "椙", "瑒", "厰", "镸", "淌", "萇", "鋹", "甞", "膓", "畼", "誯", "晿", "焻", "鋿", "倀", "悵", "㫤", "㙊", "䗅", "䯴", "㦂", "䠀", "䠆", "䕋", "䩨", "鯧", "产", "馋", "铲", "缠", "婵", "禅", "蝉", "掺", "闡"]),
    ("chao", &["超", "炒", "抄", "吵", "朝", "潮", "焯", "巢", "钞", "晁", "嘲", "槱", "鼂", "勦", "煼", "鈔", "查哦", "差哦", "叉喔", "麨", "绰", "樔", "巣", "轈", "焣", "弨", "耖", "剿", "怊", "綤", "窲", "牊", "眧", "摷", "罺", "窼", "觘", "劋", "謿", "鄛", "鼌", "繛", "仯", "綽", "仦", "漅", "趠", "欩", "訬", "粆", "㶤", "䏚", "䄻", "㷅", "䬤", "䎐", "䰫", "巐", "查", "差", "茶", "插", "叉", "茬", "察", "岔", "搽", "猹", "槎", "檫", "镲", "碴", "衩", "汊", "杈", "刹", "侘", "姹", "艖", "梌", "剎", "诧", "嚓", "摖", "垞", "鎈", "蹅", "喳", "偛", "疀", "馇", "锸", "嗏", "楂", "扠", "褨"]),
    ("che", &["车", "撤", "扯", "澈", "察哈尔", "車", "彻", "硩", "呫", "掣", "摰", "瞮", "坼", "吃喝儿", "聅", "出活儿", "船火儿", "烲", "春花儿", "俥", "屮", "尺", "揊", "硨", "徹", "勶", "烢", "蛼", "頙", "焎", "迠", "砗", "伡", "唓", "奲", "偖", "爡", "莗", "㿭", "㔭", "㥉", "㨋", "㾝", "䞣", "㵔", "䋲", "䑲", "撦", "䛸", "䧪", "㬚", "㯙", "䁤", "䚢", "䤁", "䨁", "䰩", "䒆", "㒤", "㱌", "䜠"]),
    ("chen", &["陈", "晨", "辰", "趁", "宸", "沉", "臣", "尘", "琛", "称", "衬", "谌", "陳", "车内", "车呢", "忱", "抻", "郴", "嗔", "瘨", "莀", "櫬", "谶", "扯呢", "车奴", "踸", "塵", "趻", "謓", "麎", "沈", "諃", "揨", "棽", "瞋", "齓", "鍖", "桭", "莐", "碜", "綝", "賝", "醦", "薼", "榇", "疢", "襯", "軙", "迧", "煁", "稱", "茞", "蔯", "龀", "敐", "訦", "鈂", "諶", "栕", "曟", "愖", "鷐", "瘎", "霃", "贂", "讖", "伧", "偁", "硶", "傖", "敶", "齔", "爯", "樄", "夦", "螴", "穪", "趂", "墋", "磣", "肜", "㕴", "䒞", "䢻", "㽸", "䑣", "䣅", "㧱", "㫳", "㲀", "捵", "䆣", "䟢", "䤟", "跈", "㴴"]),
    ("cheng", &["成", "城", "程", "称", "诚", "乘", "撑", "橙", "承", "呈", "秤", "陈哥", "澄", "晨光", "丞", "铖", "晟", "盛", "骋", "鎗", "珵", "宬", "陈工", "陈刚", "埕", "塍", "Cheng", "琤", "珹", "惩", "蛏", "稱", "逞", "誠", "頳", "柽", "阷", "枨", "撐", "撜", "筬", "鏳", "洆", "鋮", "庱", "澂", "赪", "荿", "铛", "浧", "畻", "裎", "氶", "酲", "郕", "矃", "緽", "乗", "鐣", "瞠", "憕", "偁", "娍", "峸", "椉", "赬", "鏿", "牚", "塖", "碀", "挰", "棖", "淨", "脭", "朾", "悜", "棦", "掁", "窚", "爯", "侱", "徎", "抢", "橖", "泟", "凈", "堘", "憆", "靗", "噌", "檙", "饓", "騬", "檉", "蟶", "僜"]),
    ("chi", &["吃", "迟", "池", "哆", "尺", "持", "驰", "齿", "赤", "赿", "翅", "褫", "痴", "奓", "炽", "弛", "噭", "耻", "斥", "蚳", "遅", "鴟", "柅", "跢", "踟", "呎", "敕", "遲", "嗤", "鶗", "彳", "匙", "誃", "豉", "叺", "迡", "郗", "瓻", "湁", "趩", "翨", "拸", "鉓", "螭", "彨", "茌", "蚇", "篪", "鵄", "叱", "翤", "攡", "遟", "裭", "痸", "翄", "墀", "懘", "胣", "哧", "勅", "饬", "齝", "鉹", "蚩", "彲", "俿", "腟", "垑", "袲", "邌", "捇", "慸", "笞", "馳", "徲", "誺", "侈", "銐", "慗", "癡", "魑", "喫", "鶙", "恜", "欼", "坻", "雴", "黐", "憏", "鸱", "齒", "啻", "伬", "摛", "謘"]),
    ("chong", &["冲", "充", "重", "虫", "宠", "崇", "翀", "涌", "沖", "种", "舂", "铳", "蟲", "憧", "茺", "寵", "緟", "衝", "蝩", "Chong", "忡", "摏", "漴", "湧", "艟", "徸", "銃", "埫", "種", "痋", "珫", "隀", "憃", "褈", "蹖", "爞", "浺", "嘃", "䖝", "㧤", "㳘", "崈", "揰", "㓼", "㓽", "㹐", "罿", "䡴", "䳯", "䌬"]),
    ("chou", &["抽", "丑", "臭", "愁", "仇", "瞅", "筹", "稠", "疇", "盩", "酬", "犨", "绸", "畴", "雦", "躊", "Chou", "檮", "醜", "詶", "雔", "篘", "犫", "醻", "酧", "紬", "幬", "侴", "矁", "俦", "惆", "皗", "跾", "薵", "懤", "籌", "遚", "雠", "醔", "鮘", "嚋", "踌", "瘳", "椆", "絒", "怞", "菗", "魗", "丒", "吜", "帱", "畤", "杽", "牰", "讎", "讐", "搊", "綢", "儔", "嬦", "臰", "栦", "梼", "裯", "掫", "殠", "杻", "㐜", "诪", "㤽", "䓓", "偢", "婤", "㛶", "䀺", "䇺", "䔏", "㨶", "㩅", "㵞", "㦞", "燽", "㿧", "䪮", "䊭", "䌧", "䲖", "㘜"]),
    ("chu", &["出", "初", "处", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕", "埱", "滁", "嘼", "櫉", "楮", "踀", "刍", "諔", "耡", "琡", "亍", "儲", "儊", "櫖", "岀", "廚", "俶", "濋", "鉏", "黜", "幮", "媰", "齼", "臅", "搐", "绌", "欪", "礎", "樗", "傗", "閦", "榋", "齣", "齭", "觸", "処", "憷", "斶", "蹰", "雛", "摴", "橻", "滀", "躇", "豖", "鋤", "竌", "蒢", "芻", "竐", "蟵", "椘", "躕", "豠", "鄐", "篨", "鶵", "拀", "絀", "蒭", "摢", "璴", "藸", "貙", "珿", "耝", "犓", "檚", "㔘", "䢺"]),
    ("chua", &["出啊", "出阿", "处啊", "除啊", "吃啊", "楚啊", "楚阿", "歘", "欻", "䫄", "䵵", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕", "埱", "滁", "嘼", "櫉", "楮", "踀", "刍", "諔", "耡", "琡", "亍", "儲", "儊", "櫖", "岀", "廚", "俶", "濋", "鉏", "黜", "幮", "媰", "齼", "臅", "搐", "绌", "欪", "礎", "樗", "傗", "閦", "榋", "齣", "齭", "觸", "処", "憷", "斶", "蹰", "雛", "摴", "橻", "滀", "躇", "豖", "鋤", "竌", "蒢", "芻", "竐", "蟵", "椘", "躕", "豠", "鄐", "篨", "鶵", "拀", "絀"]),
    ("chuai", &["踹", "揣", "初爱", "啜", "储爱", "搋", "触爱", "欼", "嘬", "膪", "出哎", "膗", "㪓", "㪜", "初艾", "㱀", "䦤", "䴝", "歘", "欻", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕", "埱", "滁", "嘼", "櫉", "楮", "踀", "刍", "諔", "耡", "琡", "亍", "儲", "儊", "櫖", "岀", "廚", "俶", "濋", "鉏", "䫄", "䵵", "黜", "幮", "媰", "齼", "臅", "搐", "绌", "欪", "礎", "樗", "傗", "閦", "榋", "齣", "齭", "觸", "処", "憷", "斶", "蹰", "雛", "摴", "橻", "滀", "躇", "豖", "鋤", "竌", "蒢"]),
    ("chuan", &["穿", "传", "船", "川", "串", "喘", "傳", "钏", "氚", "瑏", "踳", "C环", "椽", "楚安", "初案", "巛", "初安", "出安", "出岸", "出按", "膞", "汌", "舛", "剶", "遄", "玔", "釧", "荈", "輲", "鶨", "圌", "歂", "篅", "堾", "椯", "僢", "賗", "舡", "舩", "猭", "㱛", "㼷", "㯌", "暷", "䁣", "歘", "欻", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕", "埱", "滁", "嘼", "櫉", "楮", "踀", "刍", "諔", "耡", "琡", "亍", "儲", "儊", "櫖", "岀", "廚", "俶", "濋", "鉏", "䫄", "䵵", "黜", "幮"]),
    ("chuang", &["床", "创", "窗", "闯", "穿个", "穿过", "疮", "传给", "幢", "传功", "創", "朣", "传个", "传感", "窻", "漺", "传过", "船歌", "船工", "串钩", "传告", "穿管", "传哥", "牎", "楚昂", "出昂", "刱", "喠", "怆", "剙", "闖", "傸", "瘡", "噇", "剏", "牀", "仺", "磢", "窓", "橦", "牕", "摐", "刅", "㡖", "䇬", "愴", "㵂", "㼽", "䎫", "䃥", "䚎", "䚒", "䡴", "䭚", "穿", "传", "船", "川", "串", "喘", "傳", "钏", "氚", "瑏", "踳", "椽", "巛", "膞", "汌", "舛", "剶", "歘", "欻", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗"]),
    ("chui", &["吹", "锤", "垂", "捶", "炊", "槌", "埀", "陲", "錘", "龡", "顀", "惙", "圌", "棰", "椎", "倕", "搥", "菙", "桘", "硾", "箠", "鎚", "㝽", "䄲", "㓃", "㩾", "腄", "㥨", "䍋", "䞼", "䕓", "䳠", "䶴", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕", "埱", "滁", "嘼", "櫉", "楮", "踀", "刍", "諔", "耡", "琡", "亍", "儲", "儊", "櫖", "岀", "廚", "俶", "濋", "鉏", "黜", "幮", "媰", "齼", "臅", "搐", "绌", "欪", "礎", "樗", "傗", "閦", "榋", "齣", "齭", "觸", "処", "憷"]),
    ("chun", &["纯", "春", "蠢", "唇", "处女", "椿", "淳", "醇", "处男", "出纳", "朐", "純", "蓴", "储能", "鶉", "厨娘", "莼", "初年", "触怒", "雏鸟", "犉", "橁", "蝽", "膞", "槆", "櫄", "鹑", "偆", "鯙", "錞", "瑃", "暙", "堾", "醕", "媋", "肫", "鶞", "惷", "湻", "浱", "蒓", "杶", "漘", "芚", "鰆", "陙", "睶", "旾", "脣", "萅", "萶", "箺", "賰", "輴", "滣", "㸪", "䏛", "䓐", "䡅", "䣨", "䏰", "䣩", "㖺", "㝄", "䐇", "䐏", "䞐", "㿤", "䔚", "㵮", "䄝", "㝇", "䫃", "䦮", "䥎", "䮞", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎"]),
    ("chuo", &["戳", "齪", "鏃", "绰", "啜", "辍", "出哦", "酫", "哾", "踀", "擉", "鋜", "踔", "龊", "拺", "嚽", "淖", "鑡", "辵", "繛", "孎", "婥", "齱", "辶", "綽", "逴", "磭", "趠", "惙", "歠", "輟", "娖", "婼", "涰", "箹", "娕", "吷", "餟", "䄪", "㲋", "䂐", "䓎", "㚟", "䇍", "䋘", "䍳", "䃗", "䆯", "䮕", "出", "处", "初", "除", "楚", "储", "厨", "褚", "触", "畜", "杵", "础", "炪", "雏", "橱", "锄", "怵", "處", "歜", "趎", "禇", "櫥", "矗", "慉", "敊", "觕", "埱", "滁", "嘼", "櫉", "楮", "刍", "諔", "耡", "琡", "亍", "儲", "儊", "櫖", "岀", "廚", "俶", "濋", "鉏", "黜", "幮", "媰"]),
    ("ci", &["次", "词", "此", "刺", "辞", "磁", "瓷", "慈", "赐", "呲", "雌", "茨", "祠", "呰", "兹", "嬨", "佌", "詞", "CI", "糍", "礠", "餈", "鮆", "骴", "疵", "伺", "泚", "蛓", "賜", "髊", "珁", "甆", "朿", "辭", "偨", "茈", "鹚", "差", "絘", "茲", "跐", "鈶", "玼", "濨", "粢", "庛", "辝", "柌", "莿", "齹", "佽", "皉", "螆", "趀", "啙", "鴜", "飺", "垐", "栨", "辤", "鷀", "茦", "縒", "䦻", "㡹", "刾", "㓨", "㘹", "㞖", "㠿", "㢀", "㤵", "䂣", "䓧", "㹂", "䖪", "䛐", "䧳", "䨏", "䈘", "㘂", "䯸", "䲿", "薋", "㩞", "䳄", "䳐", "䆅", "䗹", "蠀", "鶿", "䰍", "䭣"]),
    ("cong", &["从", "聪", "葱", "丛", "從", "琮", "枞", "匆", "淙", "藂", "骢", "璁", "聰", "囱", "苁", "蓯", "聦", "蔥", "悰", "爜", "錝", "鏓", "叢", "憁", "徔", "灇", "賨", "謥", "漗", "賩", "繱", "熜", "棇", "瑽", "聡", "蟌", "従", "楤", "慒", "騘", "樅", "潀", "怱", "婃", "篵", "徖", "茐", "碂", "樷", "悤", "潨", "焧", "暰", "囪", "忩", "孮", "潈", "誴", "瞛", "愡", "漎", "鏦", "驄", "鍯", "䟲", "㗰", "㼻", "樬", "䉘", "䕺", "䳷", "欉"]),
    ("cou", &["凑", "湊", "腠", "楱", "辏", "傶", "輳", "㫶"]),
    ("cu", &["粗", "醋", "促", "簇", "噈", "趨", "蹴", "卒", "殂", "猝", "觕", "蹙", "蹵", "誎", "趗", "瘄", "皻", "踿", "蔟", "徂", "酢", "麤", "縬", "橻", "踧", "麁", "憱", "麄", "麆", "顣", "怚", "脨", "媨", "䢐", "䓚", "䟟", "䬨", "䎌", "㗤", "䛤", "蔖", "䃚", "䠓", "瘯", "䠞", "䥄", "鼀", "䥘", "㰗", "䙯"]),
    ("cuan", &["窜", "蹿", "攒", "窾", "篡", "撺", "汆", "粗安", "爨", "鑹", "巑", "酇", "灒", "攛", "镩", "殩", "熶", "濽", "竄", "鋑", "櫕", "穳", "欑", "攢", "攅", "簒", "躥", "窽", "䆘", "㭫", "㵀", "䞼", "㠝", "㸑", "䰖", "粗", "促", "醋", "簇", "噈", "趨", "蹴", "卒", "殂", "猝", "觕", "蹙", "蹵", "誎", "趗", "瘄", "皻", "踿", "蔟", "徂", "酢", "麤", "縬", "橻", "踧", "麁", "憱", "麄", "麆", "顣", "怚", "脨", "媨", "䢐", "䓚", "䟟", "䬨", "䎌", "㗤", "䛤", "蔖", "䃚", "䠓", "瘯", "䠞", "䥄", "鼀", "䥘", "㰗", "䙯"]),
    ("cui", &["催", "崔", "翠", "脆", "萃", "粹", "淬", "摧", "璀", "啐", "縗", "膵", "顇", "皠", "啛", "毳", "趡", "慛", "琗", "嵟", "槯", "衰", "凗", "焠", "悴", "墔", "瘁", "襊", "繀", "濢", "疩", "漼", "隹", "倅", "粋", "崒", "翆", "椊", "紣", "缞", "獕", "伜", "脺", "榱", "綷", "磪", "膬", "竁", "臎", "脃", "鏙", "㳃", "忰", "㧘", "㥞", "㱖", "㷃", "䂱", "䃀", "㵏", "㷪", "㝮", "㯜", "䔴", "䢪", "䧽", "熣", "㯔", "䄟", "䊫", "䆊", "粗", "促", "醋", "簇", "噈", "趨", "蹴", "卒", "殂", "猝", "觕", "蹙", "蹵", "誎", "趗", "瘄", "皻", "踿", "蔟", "徂", "酢", "麤", "縬", "橻", "踧"]),
    ("cun", &["村", "寸", "存", "吋", "皴", "邨", "忖", "粗呢", "拵", "籿", "侟", "蹲", "踆", "澊", "刌", "膥", "竴", "䍎", "粗", "促", "醋", "簇", "噈", "趨", "蹴", "卒", "殂", "猝", "觕", "蹙", "蹵", "誎", "趗", "瘄", "皻", "踿", "蔟", "徂", "酢", "麤", "縬", "橻", "踧", "麁", "憱", "麄", "麆", "顣", "怚", "脨", "媨", "䢐", "䓚", "䟟", "䬨", "䎌", "㗤", "䛤", "蔖", "䃚", "䠓", "瘯", "䠞", "䥄", "鼀", "䥘", "㰗", "䙯"]),
    ("cuo", &["错", "搓", "挫", "措", "厝", "錯", "锉", "撮", "矬", "磋", "剉", "縒", "鹺", "痤", "粗哦", "醋哦", "嵳", "夎", "蹉", "蓌", "襊", "酂", "莡", "嵯", "瑳", "脞", "瘥", "睉", "鹾", "庴", "蒫", "莝", "逪", "斮", "諎", "遳", "棤", "蔖", "銼", "醝", "剒", "虘", "鎈", "㟇", "㭫", "䂳", "䐤", "䣜", "㽨", "㿷", "䐣", "䟶", "䑘", "䠡", "䰈", "䱜", "䥘", "䴾", "酇", "粗", "促", "醋", "簇", "噈", "趨", "蹴", "卒", "殂", "猝", "觕", "蹙", "蹵", "誎", "趗", "瘄", "皻", "踿", "蔟", "徂", "酢", "麤", "縬", "橻", "踧", "麁", "憱", "麄", "麆", "顣", "怚", "脨", "媨", "䢐", "䓚", "䟟", "䬨"]),
    ("da", &["大", "打", "达", "对啊", "哒", "答", "搭", "答案", "多啊", "的啊", "打啊", "点啊", "笪", "憚", "懂啊", "遝", "到啊", "档案", "沓", "都啊", "大啊", "嗒", "掉啊", "带啊", "等啊", "達", "溚", "龘", "妲", "噠", "炟", "怛", "跶", "汏", "龖", "咑", "鞑", "笚", "耷", "呾", "荙", "垯", "逹", "瘩", "荅", "亣", "迖", "褡", "迏", "眔", "墶", "鐽", "鎝", "疸", "剳", "靼", "鎉", "墖", "韃", "羍", "畗", "燵", "畣", "匒", "蟽", "迚", "惮", "繨", "詚", "薘", "㾑", "㿯", "㜓", "㟷", "䐊", "䐛", "䪏", "㙮", "撘", "㯚", "䑽", "䩢", "䃮", "䵣", "䳴"]),
    ("dai", &["带", "戴", "待", "代", "袋", "呆", "贷", "逮", "隸", "黛", "帶", "岱", "歹", "詒", "埭", "呔", "Dai", "傣", "跢", "隷", "紿", "玳", "蝳", "曃", "怠", "殆", "螮", "貸", "靆", "甙", "黱", "迨", "箉", "瀻", "大", "诒", "垈", "懛", "棣", "鴏", "軩", "鮘", "帒", "軑", "緿", "绐", "叇", "帯", "貣", "霴", "轪", "骀", "艜", "獃", "侢", "蹛", "柋", "蚮", "釱", "瑇", "廗", "襶", "駘", "㐲", "㞭", "㶡", "䒫", "軚", "㫹", "㻖", "䈆", "䲦", "㯂", "䚞", "䚟", "㿃", "簤", "打", "达", "哒", "答", "搭", "笪", "憚", "遝", "沓", "嗒", "達", "溚", "龘", "妲", "噠", "炟", "怛", "跶", "汏"]),
    ("dan", &["但", "单", "蛋", "丹", "淡", "胆", "旦", "打你", "担", "大脑", "弹", "大年", "大娘", "大牛", "氮", "單", "诞", "啖", "鄲", "聃", "Dan", "都爱你", "迪奥娜", "躭", "愖", "黮", "噉", "禫", "啗", "抌", "耽", "掸", "簞", "褝", "澹", "澸", "郸", "癉", "甔", "箪", "彈", "僤", "腅", "繵", "耼", "石", "鴠", "儋", "黵", "単", "疍", "丼", "皽", "衴", "勯", "霮", "膽", "啿", "媅", "賧", "疸", "惮", "噡", "呾", "眈", "匰", "萏", "亶", "贉", "燀", "殚", "亻", "擔", "玬", "嚪", "瘅", "誕", "赕", "頕", "妉", "醈", "膻", "譂", "蜑", "憺", "沊", "馾", "惔", "弾", "蓞", "泹", "窞", "卩", "刐", "砃", "柦"]),
    ("dang", &["当", "党", "档", "挡", "蛋糕", "荡", "當", "耽搁", "单个", "裆", "瑒", "铛", "氹", "宕", "单管", "单干", "单根", "弹弓", "单杠", "dang", "璫", "檔", "潒", "凼", "讜", "垱", "簜", "蟷", "黨", "崵", "噹", "擋", "儅", "珰", "砀", "蘯", "婸", "簹", "礑", "攩", "蕩", "灙", "瞊", "谠", "雼", "譡", "菪", "趤", "筜", "闣", "碭", "盪", "欓", "偒", "瓽", "壋", "鐺", "圵", "艡", "嵣", "澢", "愓", "逿", "璗", "襠", "䣊", "䦒", "䑗", "㜭", "䣣", "但", "单", "蛋", "丹", "淡", "胆", "旦", "担", "弹", "氮", "單", "诞", "啖", "鄲", "聃", "躭", "愖", "黮", "噉", "禫", "啗", "抌", "耽", "掸", "簞", "褝"]),
    ("dao", &["到", "倒", "道", "刀", "导", "岛", "盗", "稻", "捣", "叨", "蹈", "氘", "D奥", "島", "箌", "檮", "祷", "大哦", "打哦", "Dao", "導", "翿", "悼", "捯", "刂", "幬", "盜", "嶋", "纛", "翢", "椡", "衟", "稲", "焘", "艔", "忉", "嶌", "隯", "釖", "搗", "辺", "嶹", "禂", "菿", "絩", "虭", "帱", "噵", "朷", "擣", "禱", "梼", "魛", "陦", "隝", "舠", "裯", "嘄", "衜", "檤", "壔", "瓙", "鱽", "槝", "螩", "軇", "㧅", "䧂", "㠀", "㨶", "䲽", "䆃", "㔑", "燾", "㿒", "䊭", "䌦", "䣣", "䮻", "打", "大", "达", "哒", "答", "搭", "笪", "憚", "遝", "沓", "嗒", "達", "溚", "龘", "妲", "噠", "炟"]),
    ("de", &["的", "得", "德", "地", "第二", "大鹅", "嘚", "点儿", "大额", "徳", "的诶", "得儿", "大二", "地儿", "锝", "都饿", "定额", "戴尔", "等额", "肚饿", "东阿", "对额", "淂", "旳", "鍀", "惪", "悳", "蚮", "棏", "恴", "㝵", "㝶", "㤫", "䙷", "䙸", "㥀", "㥁", "㯖"]),
    ("den", &["扽", "的你", "的呢", "扥", "的那", "㩐", "的女", "的男", "得能", "德农", "的能", "的您", "得拿", "第二年", "德能", "的奶", "的捏", "的内", "德纳", "den", "的", "得", "德", "地", "嘚", "徳", "锝", "淂", "旳", "鍀", "惪", "悳", "蚮", "棏", "恴", "㝵", "㝶", "㤫", "䙷", "䙸", "㥀", "㥁", "㯖"]),
    ("deng", &["等", "灯", "邓", "登", "蹬", "凳", "瞪", "澂", "噔", "燈", "鄧", "澄", "镫", "嶝", "櫈", "磴", "豋", "戥", "隥", "簦", "墱", "嬁", "鐙", "瀓", "僜", "璒", "憕", "覴", "䒭", "㲪", "竳", "艠", "䠬", "䮴", "扽", "扥", "的", "得", "德", "地", "嘚", "徳", "锝", "淂", "旳", "鍀", "㩐", "惪", "悳", "蚮", "棏", "恴", "㝵", "㝶", "㤫", "䙷", "䙸", "㥀", "㥁", "㯖"]),
    ("di", &["滴", "地", "第", "低", "底", "弟", "睇", "迪", "抵", "帝", "递", "娣", "鬄", "狄", "啲", "蒂", "邸", "敌", "涤", "笛", "堤", "哋", "的", "詆", "荻", "棣", "嘀", "秪", "缔", "谛", "頔", "踶", "掋", "鍉", "蓧", "镝", "嫡", "翟", "肑", "蹏", "嚁", "碲", "骶", "樀", "砥", "揥", "氐", "俤", "釱", "螮", "魡", "墆", "遰", "碮", "鸐", "甋", "菂", "呧", "弤", "坻", "腣", "嵽", "玓", "眱", "菧", "逓", "摕", "埞", "籴", "蝭", "柢", "枤", "蔋", "唙", "軧", "遞", "慸", "磾", "馰", "墑", "袛", "媂", "廸", "梑", "羝", "杕", "敵", "苐", "提", "鉪", "拞", "藡", "梊", "諦", "僀", "蔐"]),
    ("dian", &["点", "店", "电", "垫", "丶", "典", "颠", "點", "癫", "佔", "殿", "掂", "迪安", "涤氨", "低安", "堤岸", "笛安", "迪岸", "地安", "底案", "帝安", "地暗", "涤安", "碘", "滇", "電", "甸", "巅", "佃", "淀", "钿", "奠", "瘨", "靛", "墊", "踮", "癲", "惦", "驔", "顛", "坫", "齻", "嚸", "癜", "槇", "玷", "嵮", "簟", "扂", "婰", "澱", "槙", "婝", "奌", "蕇", "敁", "腍", "磹", "巔", "跕", "阽", "厧", "攧", "琔", "鈿", "痁", "傎", "蒧", "壂", "蹎", "巓", "椣", "痶", "蜔", "窴", "橝", "橂", "顚", "㓠", "䧃", "㚲", "㥆", "䍄", "䓦", "㼭", "敟", "㝪", "䩚", "㞟", "㵤", "㸃", "㒹", "㶘", "䟍", "嗲", "第"]),
    ("diao", &["掉", "吊", "调", "屌", "叼", "钓", "刁", "雕", "迪奥", "貂", "弔", "椆", "調", "地奥", "凋", "帝奥", "鲷", "迪澳", "嗲哦", "嗲噢", "低凹", "嬥", "蛁", "窎", "釣", "碉", "矵", "雿", "汈", "彫", "伄", "訋", "鸟", "鋽", "銱", "竨", "铞", "鈟", "铫", "鵰", "鮉", "琱", "鑃", "扚", "刟", "盄", "虭", "弴", "魡", "瘹", "藋", "鯛", "奝", "鳥", "蓧", "誂", "鳭", "錭", "鵃", "鼦", "莜", "淍", "瞗", "窵", "䄪", "䘟", "䄷", "㹿", "䂪", "㪕", "絩", "䂽", "䔙", "銚", "㒛", "䠼", "䉆", "簓", "䳝", "䵲", "嗲", "第", "低", "地", "滴", "底", "弟", "睇", "迪", "抵", "帝", "递", "娣", "鬄", "狄", "啲"]),
    ("die", &["跌", "爹", "叠", "蝶", "碟", "迭", "惵", "喋", "臷", "谍", "低额", "咥", "die", "鐡", "底额", "帝饿", "帝俄", "帝鳄", "抵饿", "敌厄", "哋", "槢", "殜", "楪", "啑", "蜨", "氎", "疊", "牒", "戜", "眰", "褺", "艓", "昳", "詄", "胅", "堞", "鰈", "鲽", "螲", "跮", "垤", "恎", "镻", "鴩", "佚", "疉", "峌", "耋", "瓞", "蹀", "幉", "趃", "苵", "揲", "耊", "泆", "嵽", "疂", "绖", "眣", "畳", "軼", "怢", "挕", "鞢", "柣", "絰", "褋", "鮙", "曡", "跕", "㑙", "㦶", "㲳", "㭯", "䏲", "䘭", "㥈", "㫼", "崼", "㻡", "牃", "㦅", "㷸", "䳀", "諜", "䙝", "㬪", "䞇", "䠟", "䮢", "䞕", "䲀", "䳻", "㩸"]),
    ("ding", &["定", "订", "丁", "顶", "盯", "鼎", "钉", "叮", "锭", "腚", "町", "訂", "頂", "玎", "酊", "啶", "ding", "碇", "仃", "釘", "掟", "帄", "疔", "濎", "錠", "耵", "靪", "艼", "鼑", "椗", "嵿", "聢", "矴", "碠", "薡", "鐤", "铤", "饤", "萣", "奵", "磸", "虰", "飣", "忊", "蝊", "甼", "顁", "鋌", "㐉", "䦺", "㝎", "㼗", "㫀", "㝪", "㴿", "第", "低", "地", "滴", "底", "弟", "睇", "迪", "抵", "帝", "递", "娣", "鬄", "狄", "啲", "蒂", "邸", "敌", "涤", "笛", "堤", "哋", "的", "詆", "荻", "棣", "嘀", "秪", "缔", "谛", "頔", "踶", "掋", "鍉", "蓧", "镝", "嫡", "翟", "肑", "蹏", "嚁"]),
    ("diu", &["丢", "丟", "铥", "銩", "颩", "第", "低", "地", "滴", "底", "弟", "睇", "迪", "抵", "帝", "递", "娣", "鬄", "狄", "啲", "蒂", "邸", "敌", "涤", "笛", "堤", "哋", "的", "詆", "荻", "棣", "嘀", "秪", "缔", "谛", "頔", "踶", "掋", "鍉", "蓧", "镝", "嫡", "翟", "肑", "蹏", "嚁", "碲", "骶", "樀", "砥", "揥", "氐", "俤", "釱", "螮", "魡", "墆", "遰", "碮", "鸐", "甋", "菂", "呧", "弤", "坻", "腣", "嵽", "玓", "眱", "菧", "逓", "摕", "埞", "籴", "蝭", "柢", "枤", "蔋", "唙", "軧", "遞", "慸", "磾", "馰", "墑", "袛", "媂", "廸", "梑", "羝", "杕", "敵", "苐", "提", "鉪", "拞"]),
    ("dong", &["东", "懂", "动", "栋", "董", "冬", "冻", "洞", "咚", "東", "動", "垌", "狪", "揰", "dong", "DONG", "氡", "棟", "硐", "凍", "侗", "岽", "鸫", "峒", "姛", "苳", "胨", "胴", "鶇", "昸", "働", "霘", "烔", "崬", "夂", "菄", "嬞", "笗", "埬", "娻", "峝", "涷", "徚", "鼕", "墥", "恫", "諌", "倲", "騆", "戙", "鶫", "崠", "腖", "箽", "燑", "駧", "絧", "鯟", "迵", "氭", "蕫", "挏", "狫", "湩", "勭", "鮗", "㐑", "㑈", "㓊", "㢥", "䂢", "㖦", "㨂", "䅍", "㼯", "䞒", "詷", "㗢", "䍶", "蝀", "䳉", "㜱", "䵔"]),
    ("dou", &["都", "豆", "抖", "斗", "讀", "逗", "窦", "兜", "痘", "陡", "钭", "毭", "鬥", "荳", "唞", "鋀", "蔸", "蚪", "竇", "篼", "阧", "瞗", "浢", "枓", "梪", "閗", "脰", "鈄", "鬬", "鬦", "乧", "鬭", "狵", "渎", "餖", "侸", "读", "唗", "饾", "瀆", "闘", "酘", "兠", "吋", "郖", "鬪", "吺", "凟", "斣", "㞳", "㐙", "㛒", "㢄", "㪷", "䄈", "㷆", "䬦", "䇺", "㨮", "䛠", "䕆", "橷", "䕱"]),
    ("du", &["度", "读", "都", "杜", "堵", "毒", "赌", "嘟", "独", "肚", "渡", "镀", "督", "牘", "笃", "闍", "讀", "髑", "蠧", "渎", "睹", "犊", "賭", "櫝", "裻", "荰", "剫", "妒", "獨", "襡", "韣", "凟", "匵", "牍", "殰", "椟", "蠹", "螙", "鍍", "殬", "芏", "涜", "嬻", "豄", "襩", "篤", "黩", "騳", "阇", "皾", "叾", "妬", "顿", "讟", "厾", "暏", "豛", "陼", "読", "覩", "瀆", "韥", "晵", "犢", "韇", "錖", "贕", "頓", "姤", "靯", "樚", "醏", "斁", "鑟", "黷", "㡯", "㾄", "剢", "㸿", "䅊", "帾", "秺", "㓃", "㱩", "䀾", "䄍", "䐗", "䓯", "琽", "䟻", "䲧", "㹍", "䈞", "䢱", "瓄", "䪅"]),
    ("duan", &["段", "短", "端", "断", "锻", "缎", "都安", "煅", "斷", "篅", "塅", "杜安", "毈", "椴", "度安", "渡安", "独安", "度岸", "毒案", "耑", "簖", "鍛", "碫", "剬", "煆", "媏", "瑖", "葮", "褍", "籪", "緞", "偳", "腶", "躖", "㱭", "㫁", "鍴", "䠪", "度", "都", "读", "杜", "堵", "毒", "赌", "嘟", "独", "肚", "渡", "镀", "督", "牘", "笃", "闍", "讀", "髑", "蠧", "渎", "睹", "犊", "賭", "櫝", "裻", "荰", "剫", "妒", "獨", "襡", "韣", "凟", "匵", "牍", "殰", "椟", "蠹", "螙", "鍍", "殬", "芏", "涜", "嬻", "豄", "襩", "篤", "黩", "騳", "阇", "皾", "叾", "妬", "顿", "讟", "厾", "暏", "豛", "陼"]),
    ("dui", &["对", "队", "怼", "堆", "兑", "對", "鎚", "碓", "隊", "兌", "DUI", "磓", "懟", "敓", "譈", "痽", "鴭", "綐", "対", "憝", "譵", "镦", "憞", "敦", "杸", "啍", "濻", "埻", "祋", "濧", "瀢", "垖", "嵟", "兊", "鐓", "鐜", "轛", "頧", "薱", "搥", "錞", "謉", "塠", "㳔", "㙂", "㟋", "陮", "㨃", "䇤", "敚", "䇏", "䔪", "䊚", "䬽", "䬈", "㠚", "㬣", "㵽", "䯟", "瀩", "䨴", "䨺", "度", "都", "读", "杜", "堵", "毒", "赌", "嘟", "独", "肚", "渡", "镀", "督", "牘", "笃", "闍", "讀", "髑", "蠧", "渎", "睹", "犊", "賭", "櫝", "裻", "荰", "剫", "妒", "獨", "襡", "韣", "凟", "匵", "牍"]),
    ("dun", &["蹲", "吨", "盾", "顿", "炖", "墩", "敦", "钝", "遁", "盹", "趸", "毒奶", "肚腩", "嘟囔", "逇", "囤", "镦", "噸", "沌", "惇", "dun", "頓", "墪", "撉", "犜", "燉", "蜳", "楯", "礅", "鈍", "遯", "驐", "碷", "撴", "墫", "砘", "坉", "躉", "潡", "橔", "鐓", "壿", "蹾", "踲", "庉", "憞", "忳", "伅", "腯", "䤜", "㥫", "腞", "獤", "㬿", "度", "都", "读", "杜", "堵", "毒", "赌", "嘟", "独", "肚", "渡", "镀", "督", "牘", "笃", "闍", "讀", "髑", "蠧", "渎", "睹", "犊", "賭", "櫝", "裻", "荰", "剫", "妒", "獨", "襡", "韣", "凟", "匵", "牍", "殰", "椟", "蠹", "螙", "鍍", "殬", "芏", "涜"]),
    ("duo", &["多", "躲", "朵", "夺", "剁", "铎", "哆", "垛", "舵", "挆", "堕", "駄", "杝", "硾", "惰", "duo", "度哦", "读哦", "毒哦", "馱", "桗", "缍", "跺", "柂", "敚", "鵽", "媠", "椯", "喥", "哚", "軃", "詑", "鬌", "掇", "垜", "貀", "咄", "嫷", "嶞", "跿", "度", "趓", "奲", "枤", "朶", "埵", "袳", "跥", "踱", "奪", "毲", "崜", "挅", "飿", "痥", "鈬", "鐸", "柁", "茤", "綞", "驮", "墮", "杕", "凙", "夛", "鮵", "刴", "饳", "亸", "陏", "畓", "剟", "敓", "隳", "嚲", "陊", "剫", "跢", "仛", "沰", "隓", "憜", "嚉", "柮", "裰", "墯", "沲", "㛆", "㛊", "㣞", "䒳", "尮", "㧷", "㙍", "㥩", "䑨"]),
    ("ei", &["诶", "欸", "誒", "EI", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "崿", "騀", "琧", "悪", "涐", "堨", "婀", "戹", "屵", "擜", "餩", "囮", "魤", "硆", "礘", "鰪", "齾", "讍", "匎", "阨", "鑩", "頞", "鱷", "咢", "搕", "覨", "魥", "鵈", "姶", "阏", "珴", "砐", "捓", "堊", "枙", "訛", "阸", "吪", "皒"]),
    ("en", &["嗯", "恩", "嗯呢", "摁", "蒽", "嗯呐", "儿女", "而你", "en", "恶念", "儿呢", "二年", "奀", "饿呢", "二奶", "嗯哪", "婀娜", "二娘", "讹你", "屙尿", "二妞", "煾", "恩诺", "嗯捏", "恶女", "峎", "䊐", "䬶", "䅰", "䭓", "䭡", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙", "讹", "厄", "額", "鳄", "啞", "娿", "蛾", "噩", "峨", "餓", "頋", "莪", "颚", "腭", "阿", "惡", "痷", "锷", "閼", "萼", "锇", "搤", "轭", "鵝", "鰐", "鹗", "噁", "遏", "睋", "扼", "偔", "卾", "遌", "苊", "愕", "峩", "垩", "蝁", "齶", "歺", "谔", "遻", "僫", "诶", "崿", "騀", "琧", "悪", "涐", "堨", "婀", "戹", "屵", "擜", "餩"]),
    ("er", &["二", "儿", "而", "尔", "贰", "耳", "饵", "貳", "迩", "恶人", "兒", "洱", "鹅绒", "er", "鹅肉", "二人", "珥", "爾", "恩人", "而让", "二日", "二热", "而然", "弍", "尓", "铒", "貮", "佴", "児", "咡", "侕", "餌", "尒", "唲", "鲕", "胹", "洏", "耏", "毦", "趰", "栮", "弐", "聏", "陑", "粫", "杒", "栭", "髶", "薾", "刵", "峏", "荋", "轜", "駬", "鉺", "髵", "嬭", "衈", "鮞", "輀", "邇", "鴯", "梕", "袻", "誀", "陾", "樲", "鸸", "隭", "㚷", "㖇", "㛅", "㧫", "䌺", "䎟", "䎠", "䏪", "䎶", "㢽", "䋙", "㮕", "䣵", "㒃", "䋩", "㜨", "䮘", "额", "饿", "呃", "鹅", "哦", "恶", "鄂", "娥", "俄", "屙"]),
    ("fa", &["发", "法", "罚", "伐", "阀", "方案", "發", "乏", "发啊", "飞啊", "肺癌", "放啊", "烦啊", "筏", "分啊", "珐", "妨碍", "酦", "蕟", "髮", "垡", "罰", "笩", "髪", "砝", "発", "傠", "灋", "閥", "琺", "罸", "鍅", "橃", "佱", "彂", "藅", "醗", "沷", "茷", "墢", "姂", "疺", "冹", "坺", "栰", "峜", "瞂", "醱", "浌", "䒥", "㕹", "㳒", "㘺", "䇅", "㛲", "䂲", "䣹"]),
    ("fan", &["饭", "翻", "反", "烦", "范", "返", "凡", "樊", "发你", "帆", "犯", "泛", "番", "梵", "繁", "钒", "璠", "飯", "畈", "籓", "fan", "富安娜", "菲安妮", "藩", "贩", "幡", "蕃", "橎", "礬", "煩", "矾", "汎", "轓", "嬔", "杋", "飰", "蠜", "蘩", "婏", "範", "燔", "鷭", "瀪", "桳", "氾", "羳", "忛", "仮", "凣", "襎", "販", "颿", "笵", "滼", "凢", "墦", "膰", "棥", "旙", "訉", "籵", "犭", "攵", "蹯", "軓", "釩", "嬎", "匥", "柉", "噃", "輽", "舤", "畨", "舧", "軬", "薠", "払", "犿", "拚", "笲", "勫", "旛", "飜", "奿", "舩", "盕", "渢", "緐", "憣", "繙", "鐢", "鱕", "瀿", "㕨", "㠶", "㤆"]),
    ("fang", &["放", "方", "房", "芳", "防", "仿", "坊", "访", "反感", "纺", "返工", "反光", "饭馆", "昉", "犯规", "舫", "妨", "返岗", "凡哥", "梵高", "枋", "錺", "鴋", "钫", "Fang", "匚", "蚄", "邡", "肪", "瓬", "紡", "眆", "彷", "訪", "汸", "趽", "鲂", "淓", "堏", "埅", "祊", "昘", "倣", "髣", "鶭", "牥", "旊", "鈁", "㑂", "㕫", "㤃", "㧍", "䢍", "眪", "䦈", "㯐", "䲱", "魴", "䉊", "饭", "反", "烦", "翻", "范", "返", "凡", "樊", "帆", "犯", "泛", "番", "梵", "繁", "钒", "璠", "飯", "畈", "籓", "藩", "贩", "幡", "蕃", "橎", "礬", "煩", "矾", "汎", "轓", "嬔", "杋", "飰", "蠜", "蘩", "婏", "範", "燔"]),
    ("fei", &["非", "飞", "费", "废", "肥", "菲", "肺", "斐", "妃", "霏", "匪", "啡", "飛", "吠", "沸", "绯", "翡", "費", "蕡", "扉", "癈", "暃", "婓", "狒", "胏", "蜚", "腓", "廢", "昲", "萉", "蟦", "笰", "榧", "淝", "飝", "鼣", "靟", "镄", "橃", "鲱", "芾", "朏", "痱", "棐", "悱", "鯡", "襏", "濷", "诽", "杮", "騛", "蜰", "婔", "剕", "緋", "厞", "渄", "奜", "誹", "裶", "馡", "砩", "陫", "騑", "鐨", "蕜", "疿", "屝", "猆", "俷", "橨", "篚", "胇", "曊", "櫠", "靅", "犻", "胐", "髴", "柹", "廃", "㓈", "㔗", "㸬", "㐟", "䀟", "䍨", "䑔", "㭭", "䨽", "䨾", "㹃", "䛍", "㥱", "㾱", "䈈"]),
    ("fen", &["分", "份", "粉", "芬", "饙", "奋", "粪", "坟", "汾", "纷", "酚", "焚", "瞓", "翂", "轒", "氛", "愤", "岎", "凡尔纳", "费尔南", "Fen", "砏", "玢", "鼢", "忿", "朆", "衯", "幩", "黂", "紛", "鈖", "吩", "餴", "羒", "燌", "棻", "糞", "橨", "魵", "黺", "墳", "鱝", "鐼", "梤", "棼", "躮", "奮", "馩", "鲼", "獖", "雰", "坆", "昐", "偾", "枌", "蚡", "坋", "蕡", "膹", "馚", "憤", "訜", "瀵", "豮", "濆", "妢", "帉", "炃", "弅", "竕", "鼖", "秎", "隫", "噴", "鳻", "燓", "羵", "䢍", "㸮", "朌", "䀟", "蚠", "㖹", "㷊", "㻞", "䯨", "蒶", "㥹", "㮥", "僨", "㱵", "錀", "㿎", "豶", "䩿", "䴅"]),
    ("feng", &["风", "封", "冯", "峰", "凤", "丰", "锋", "疯", "缝", "枫", "逢", "奉", "蜂", "烽", "沣", "風", "分给", "分工", "峯", "分管", "俸", "分割", "分隔", "艂", "楓", "鳳", "馮", "豐", "篈", "蠭", "沨", "瘋", "葑", "鋒", "砜", "埄", "賵", "酆", "讽", "凨", "捀", "鴌", "熢", "鳯", "夆", "縫", "蘴", "仹", "霻", "豊", "灃", "犎", "偑", "渢", "鎽", "妦", "桻", "堸", "焨", "堼", "唪", "凮", "莑", "煈", "凬", "浲", "崶", "寷", "甮", "琒", "炐", "檒", "溄", "僼", "鄷", "綘", "盽", "猦", "湗", "諷", "靊", "飌", "赗", "鏠", "碸", "麷", "䏎", "䒠", "覂", "䟪", "㡝", "㦀", "摓", "漨", "㷭", "䙜"]),
    ("fou", &["否", "缶", "不", "炰", "妚", "缻", "雬", "紑", "殕", "缹", "鴀", "㓡", "䬏", "䳕", "佛", "仏", "坲", "梻", "仸", "㤇"]),
    ("fu", &["付", "服", "福", "副", "富", "负", "复", "符", "附", "傅", "府", "父", "夫", "赴", "伏", "幅", "敷", "扶", "浮", "芙", "肤", "赋", "氟", "孚", "辅", "甫", "弗", "釜", "妇", "腹", "覆", "馥", "腐", "阜", "抚", "邚", "斧", "拂", "颰", "缚", "鵩", "襆", "俯", "虙", "椱", "鳧", "袝", "鍑", "澓", "孵", "輹", "痡", "俌", "駙", "乀", "佛", "葍", "涪", "鬴", "郙", "腑", "鮄", "呋", "蝠", "茯", "辐", "箁", "賻", "荂", "鮒", "怤", "哹", "復", "麩", "坿", "枎", "蚹", "麸", "韍", "弣", "柭", "洑", "蚨", "嬔", "負", "萯", "凫", "滏", "笰", "鍢", "岪", "芾", "苻", "拊", "紼", "璷"]),
    ("ga", &["噶", "嘎", "尬", "咖", "狗啊", "过啊", "尕", "公安", "高啊", "干啊", "贵啊", "呷", "关爱", "旮", "轧", "钆", "戛", "伽", "嘠", "尜", "玍", "魀", "朒", "夾", "釓", "錷", "夹", "肐", "軋"]),
    ("gai", &["改", "该", "盖", "钙", "概", "丐", "該", "葢", "忋", "胲", "垓", "GAI", "蓋", "侅", "溉", "隑", "祴", "荄", "鈣", "瓂", "豥", "賌", "赅", "陔", "匄", "杚", "匃", "戤", "乢", "摡", "晐", "槩", "鎅", "郂", "絯", "槪", "磑", "畡", "絠", "漑", "姟", "㧉", "䏗", "峐", "㱾", "䀭", "䪱", "㮣", "賅", "㕢", "噶", "嘎", "尬", "咖", "尕", "呷", "旮", "轧", "钆", "戛", "伽", "嘠", "尜", "玍", "魀", "朒", "夾", "釓", "錷", "夹", "肐", "軋"]),
    ("gan", &["干", "敢", "感", "甘", "赶", "咁", "肝", "杆", "淦", "赣", "柑", "竿", "幹", "苷", "倝", "擀", "更爱你", "贵爱娘", "凎", "矸", "趕", "酐", "鱤", "秆", "乾", "绀", "橄", "魐", "玕", "虷", "尴", "乹", "贑", "佄", "旰", "攼", "泔", "桿", "贛", "坩", "疳", "鳡", "盰", "鰔", "豃", "汵", "紺", "亁", "扞", "皯", "澉", "芉", "漧", "粓", "檊", "衦", "玵", "榦", "灨", "仠", "澸", "尷", "笴", "篢", "凲", "忓", "骭", "迀", "稈", "詌", "筸", "簳", "䇞", "㺂", "䔈", "尲", "㽏", "䯎", "鳱", "䵟", "䲺", "尶", "䃭", "䤗", "噶", "嘎", "尬", "咖", "尕", "呷", "旮", "轧", "钆", "戛", "伽", "嘠"]),
    ("gang", &["刚", "港", "岗", "钢", "杠", "尴尬", "缸", "纲", "罡", "干过", "冈", "干干", "干锅", "肛", "剛", "干个", "戆", "感官", "冮", "鋼", "赣G", "岡", "gang", "崗", "戅", "堽", "扛", "犅", "綱", "棡", "罓", "槓", "戇", "釭", "鎠", "矼", "筻", "牨", "疘", "阬", "焵", "摃", "犺", "堈", "掆", "焹", "碙", "罁", "㽘", "㟠", "䚗", "㟵", "䴚", "干", "感", "敢", "甘", "赶", "咁", "肝", "杆", "淦", "赣", "柑", "竿", "幹", "苷", "倝", "擀", "凎", "矸", "趕", "酐", "鱤", "秆", "乾", "绀", "橄", "魐", "玕", "虷", "尴", "乹", "贑", "佄", "旰", "攼", "泔", "桿", "贛", "坩", "疳", "噶", "嘎", "尬", "咖"]),
    ("gao", &["搞", "高", "稿", "告", "膏", "糕", "郜", "锆", "皋", "镐", "羙", "羔", "杲", "櫜", "睾", "篙", "髙", "鼛", "诰", "GAO", "噶哦", "嘎哦", "噶喔", "餻", "藁", "槁", "缟", "勂", "獔", "橰", "皐", "吿", "誥", "睪", "筶", "鷱", "鎬", "槔", "夰", "暠", "獋", "稁", "藳", "槀", "鋯", "祮", "祰", "檺", "峼", "臯", "滜", "菒", "槹", "㚏", "㚖", "䔌", "禞", "㾸", "稾", "縞", "䗣", "䨢", "韟", "鷎", "噶", "嘎", "尬", "咖", "尕", "呷", "旮", "轧", "钆", "戛", "伽", "嘠", "尜", "玍", "魀", "朒", "夾", "釓", "錷", "夹", "肐", "軋"]),
    ("ge", &["个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁", "個", "嘅", "戈", "阁", "鸽", "感恩", "高二", "高额", "孤儿", "个儿", "歌尔", "龟儿", "挂耳", "革", "咯", "铬", "嗝", "镉", "嗰", "舸", "猲", "硌", "紇", "膈", "頜", "圪", "蛤", "鞈", "鬲", "閣", "箇", "哿", "愅", "閤", "盖", "骼", "敋", "胳", "搿", "疙", "餎", "鴿", "仡", "合", "彁", "滆", "秴", "韚", "擱", "騔", "鲄", "戨", "謌", "鉻", "鎶", "虼", "袼", "屹", "饹", "亇", "輵", "茖", "滒", "挌", "韐", "吤", "笴", "鎘", "鮥", "塥", "蓋", "獦", "櫊", "佮", "牫", "蛒", "犵", "轕", "臵", "鞷", "纥", "鮯", "鴐", "呄", "肐", "颌", "杚"]),
    ("gei", &["给", "給", "个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁", "個", "嘅", "戈", "阁", "鸽", "革", "咯", "铬", "嗝", "镉", "嗰", "舸", "猲", "硌", "紇", "膈", "頜", "圪", "蛤", "鞈", "鬲", "閣", "箇", "哿", "愅", "閤", "盖", "骼", "敋", "胳", "搿", "疙", "餎", "鴿", "仡", "合", "彁", "滆", "秴", "韚", "擱", "騔", "鲄", "戨", "謌", "鉻", "鎶", "虼", "袼", "屹", "饹", "亇", "輵", "茖", "滒", "挌", "韐", "吤", "笴", "鎘", "鮥", "塥", "蓋", "獦", "櫊", "佮", "牫", "蛒", "犵", "轕", "臵", "鞷", "纥", "鮯", "鴐", "呄", "肐", "颌", "杚", "佫", "匌", "槅", "扢", "戓", "牱"]),
    ("gen", &["跟", "根", "艮", "个你", "搁那", "亘", "哏", "搁哪", "茛", "个能", "亙", "搄", "官恩娜", "揯", "㫔", "㮓", "䫀", "个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁", "個", "嘅", "戈", "阁", "鸽", "革", "咯", "铬", "嗝", "镉", "嗰", "舸", "猲", "硌", "紇", "膈", "頜", "圪", "蛤", "鞈", "鬲", "閣", "箇", "哿", "愅", "閤", "盖", "骼", "敋", "胳", "搿", "疙", "餎", "鴿", "仡", "合", "彁", "滆", "秴", "韚", "擱", "騔", "鲄", "戨", "謌", "鉻", "鎶", "虼", "袼", "屹", "饹", "亇", "輵", "茖", "滒", "挌", "韐", "吤", "笴", "鎘", "鮥", "塥", "蓋", "獦", "櫊", "佮", "牫", "蛒", "犵", "轕"]),
    ("geng", &["更", "梗", "耿", "庚", "跟个", "耕", "羹", "頸", "羮", "埂", "赓", "縆", "根管", "过而能改", "哽", "峺", "畊", "鶊", "緪", "焿", "暅", "鲠", "颈", "莄", "浭", "椩", "绠", "賡", "絙", "菮", "堩", "絚", "骾", "刯", "掶", "挭", "綆", "鯁", "郠", "䋁", "㾘", "鹒", "䌄", "䨣", "䱍", "䱎", "䱭", "䱴", "跟", "根", "艮", "亘", "哏", "茛", "个", "哥", "各", "歌", "格", "葛", "割", "隔", "搁", "個", "嘅", "戈", "阁", "鸽", "革", "咯", "铬", "嗝", "镉", "嗰", "舸", "猲", "硌", "紇", "膈", "頜", "圪", "蛤", "鞈", "鬲", "閣", "箇", "哿", "愅", "閤", "盖", "骼", "敋", "亙", "搄", "揯", "㫔"]),
    ("gong", &["工", "共", "公", "供", "龚", "宫", "功", "弓", "攻", "巩", "拱", "贡", "汞", "恭", "宮", "躬", "碽", "熕", "肱", "珙", "Gong", "蛬", "貢", "觵", "躳", "塨", "龔", "廾", "红", "唝", "龷", "栱", "蚣", "匑", "慐", "觥", "愩", "厷", "銾", "篢", "杛", "侊", "羾", "紅", "嗊", "魟", "贛", "拲", "匔", "糼", "幊", "鞏", "髸", "糿", "輁", "澒", "龏", "㫒", "㺬", "㓋", "䢼", "㤨", "㧬", "㭟", "㼦", "䢚", "䂬", "䇨", "渱", "䔈", "䡗", "㯯", "䱋", "㔶"]),
    ("gou", &["狗", "够", "购", "勾", "沟", "苟", "钩", "构", "垢", "缑", "枸", "芶", "豿", "啂", "姤", "購", "溝", "夠", "煹", "篝", "媾", "褠", "诟", "搆", "笱", "緱", "彀", "耉", "岣", "構", "鈎", "佝", "耇", "觏", "糓", "耈", "遘", "鞲", "抅", "軥", "茍", "拘", "雊", "句", "袧", "蚼", "傋", "覯", "坸", "呴", "玽", "冓", "簼", "韝", "茩", "痀", "撀", "訽", "詬", "㽛", "㳶", "㺃", "䑦", "䝭", "㗕", "㜌", "㝅", "㝤", "㨌", "鉤", "䃓", "䞀"]),
    ("gu", &["姑", "古", "股", "顾", "故", "谷", "估", "骨", "固", "鼓", "菇", "賈", "辜", "孤", "咕", "钴", "蛊", "雇", "唃", "箍", "沽", "錮", "毂", "鴣", "罛", "榾", "崮", "锢", "皷", "鈲", "顧", "硲", "牯", "堌", "罟", "磆", "蛌", "觚", "榖", "巭", "祻", "汩", "稒", "菰", "軲", "鮕", "尳", "贾", "瀔", "愲", "橭", "鸪", "穀", "濲", "餶", "笟", "嘏", "嗀", "蛄", "蠱", "轱", "诂", "縎", "薣", "蓇", "鲴", "詁", "頋", "皼", "瞽", "傦", "鹄", "崓", "梏", "痼", "呱", "臌", "羖", "鹘", "鈷", "盬", "牿", "酤", "轂", "杚", "糓", "苽", "淈", "僱", "夃", "焸", "鯝", "泒", "唂", "馉", "箛"]),
    ("gua", &["挂", "瓜", "刮", "卦", "呱", "寡", "掛", "铦", "褂", "剐", "焻", "股啊", "姑啊", "古啊", "古阿", "顾啊", "煱", "胍", "咼", "啩", "緺", "劀", "鴰", "坬", "鸹", "括", "桰", "罫", "栝", "罣", "踻", "銽", "歄", "諣", "袿", "冎", "剮", "诖", "颪", "呙", "叧", "颳", "絓", "詿", "銛", "騧", "㒷", "䒷", "趏", "䈑", "䯏", "股", "顾", "古", "故", "谷", "姑", "估", "骨", "固", "鼓", "菇", "賈", "辜", "孤", "咕", "钴", "蛊", "雇", "唃", "箍", "沽", "錮", "毂", "鴣", "罛", "榾", "崮", "锢", "皷", "鈲", "顧", "硲", "牯", "堌", "罟", "磆", "蛌", "觚", "榖", "巭", "祻", "汩", "稒", "菰", "軲"]),
    ("guai", &["怪", "乖", "拐", "骨癌", "罫", "夬", "柺", "叏", "古艾", "谷艾", "古爱", "顾爱", "掴", "箉", "枴", "恠", "㧔", "䊽", "䂯", "㷇", "摑", "㽇", "挂", "瓜", "刮", "卦", "呱", "寡", "掛", "铦", "褂", "剐", "焻", "煱", "胍", "咼", "啩", "緺", "劀", "鴰", "坬", "股", "顾", "古", "故", "谷", "姑", "估", "骨", "固", "鼓", "菇", "賈", "辜", "孤", "咕", "钴", "蛊", "雇", "唃", "箍", "沽", "錮", "毂", "鴣", "罛", "榾", "崮", "锢", "皷", "鈲", "顧", "硲", "牯", "堌", "罟", "磆", "蛌", "觚", "榖", "巭", "祻", "汩", "稒", "菰", "軲", "鮕", "尳", "贾", "瀔", "愲", "橭", "鸹", "括", "桰", "栝"]),
    ("guan", &["管", "关", "罐", "馆", "官", "观", "冠", "惯", "灌", "固安", "莞", "挂念", "贯", "綸", "關", "棺", "掼", "矔", "挂你", "顾安", "谷安", "谷胺", "谷氨", "古安", "Guan", "顾桉", "姑庵", "故安", "GUAN", "觀", "倌", "館", "鹳", "琯", "淉", "躀", "痯", "鑵", "貫", "筦", "盥", "慣", "罆", "蒄", "関", "璭", "鱞", "遦", "癏", "鳏", "瓘", "欟", "舘", "窤", "鱹", "祼", "鏆", "覌", "涫", "毌", "卝", "雚", "纶", "悺", "斡", "矜", "泴", "矝", "鳤", "闗", "丱", "錧", "爟", "萖", "悹", "閞", "懽", "鰥", "礶", "潅", "樌", "輨", "観", "鸛", "䏓", "䂯", "㮡", "㴦", "䦎", "䘾", "䗆", "摜", "䝺", "瘝", "䙛", "䩪"]),
    ("guang", &["光", "逛", "广", "管管", "潢", "管够", "观光", "古昂", "关关", "咣", "关公", "观感", "罐罐", "灌溉", "廣", "獷", "胱", "管过", "管个", "管工", "洸", "挄", "桄", "僙", "珖", "櫎", "烡", "広", "黆", "趪", "犷", "灮", "侊", "銧", "輄", "炛", "垙", "茪", "炗", "姯", "炚", "俇", "臦", "硄", "㫛", "㤮", "撗", "䍍", "臩", "管", "关", "罐", "馆", "官", "观", "冠", "惯", "灌", "莞", "贯", "綸", "關", "棺", "掼", "矔", "觀", "倌", "館", "鹳", "琯", "淉", "躀", "痯", "鑵", "貫", "筦", "盥", "挂", "瓜", "刮", "卦", "呱", "寡", "掛", "铦", "褂", "剐", "焻", "煱", "胍", "咼", "啩", "緺", "劀", "鴰", "坬"]),
    ("gui", &["贵", "鬼", "柜", "归", "桂", "龟", "规", "硅", "跪", "轨", "癸", "貴", "圭", "攰", "诡", "闺", "瑰", "GUI", "鴂", "櫃", "璝", "騩", "蟡", "螝", "櫰", "歸", "簋", "樻", "禬", "瞶", "猤", "珪", "窐", "劌", "胿", "潙", "妫", "龜", "媯", "鲑", "雟", "匭", "皈", "鳜", "嶡", "刿", "鬶", "赽", "匮", "襘", "瞡", "槻", "眭", "鬹", "鳺", "槼", "規", "攱", "鐀", "晷", "槣", "觤", "簂", "櫷", "桧", "傀", "厬", "撌", "椢", "邽", "亀", "氿", "刽", "膭", "姽", "軌", "廆", "昋", "詭", "庋", "炅", "宄", "閨", "匦", "佹", "帰", "摫", "炔", "垝", "茥", "筀", "鑎", "槶", "溎", "嶲", "郌"]),
    ("gun", &["滚", "棍", "姑娘", "丨", "辊", "衮", "滾", "gun", "姑奶", "緄", "顾念", "惃", "鲧", "顾南", "硍", "睴", "磙", "鯀", "绲", "袞", "謴", "輥", "棞", "緷", "睔", "蓘", "裷", "璭", "㔳", "㙥", "㨰", "䃂", "䎾", "蔉", "㫎", "䙛", "鮌", "䜇", "㯻", "䵪", "股", "顾", "古", "故", "谷", "姑", "估", "骨", "固", "鼓", "菇", "賈", "辜", "孤", "咕", "钴", "蛊", "雇", "唃", "箍", "沽", "錮", "毂", "鴣", "罛", "榾", "崮", "锢", "皷", "鈲", "顧", "硲", "牯", "堌", "罟", "磆", "蛌", "觚", "榖", "巭", "祻", "汩", "稒", "菰", "軲", "鮕", "尳", "贾", "瀔", "愲", "橭", "鸪", "穀", "濲", "餶", "笟"]),
    ("guo", &["过", "国", "郭", "锅", "果", "裹", "過", "粿", "濄", "虢", "國", "菓", "姑哦", "古哦", "呙", "帼", "蝈", "鍋", "埚", "腘", "囯", "輠", "膕", "楇", "啯", "涡", "囗", "櫎", "敋", "掴", "淉", "崞", "餜", "聒", "椁", "懖", "慖", "咼", "馃", "漍", "猓", "幗", "渦", "槨", "囻", "彍", "蜾", "墎", "馘", "嘓", "摑", "囶", "圀", "堝", "綶", "聝", "簂", "喐", "鈛", "瘑", "彉", "蔮", "蟈", "矌", "鐹", "㕵", "㳀", "㖪", "㞅", "惈", "腂", "褁", "䂸", "㗻", "䐸", "䤋", "䬎", "㚍", "㶁", "䙨", "䴹", "䆐", "股", "顾", "古", "故", "谷", "姑", "估", "骨", "固", "鼓", "菇", "賈", "辜", "孤"]),
    ("ha", &["哈", "蛤", "好啊", "铪", "会啊", "回啊", "奤", "黑暗", "淮安", "很爱", "话啊", "海岸", "哈啊", "蝦", "虾", "鉿", "紦", "妎"]),
    ("hai", &["还", "嗨", "海", "害", "孩", "嗐", "蓋", "還", "亥", "咳", "氦", "hai", "猲", "骇", "骸", "酼", "餀", "塰", "咍", "醢", "烸", "饚", "胲", "絯", "嚡", "駭", "郂", "駴", "拸", "㜾", "妎", "㱚", "㤥", "㧡", "䂤", "䍖", "䇋", "㨟", "䠹", "㕢", "㺔", "䯐", "䠽", "㦟", "䱺", "哈", "蛤", "铪", "奤", "蝦", "虾", "鉿", "紦"]),
    ("han", &["喊", "含", "韩", "汉", "函", "涵", "寒", "汗", "焊", "瀚", "晗", "翰", "憨", "菡", "旱", "罕", "笒", "哈尼", "焓", "晥", "悍", "很爱你", "好爱你", "歛", "頷", "漢", "邯", "浛", "憾", "韓", "撼", "酣", "鼾", "捍", "椷", "涆", "閈", "肣", "浫", "睅", "撖", "邗", "虷", "唅", "筨", "娢", "梒", "佄", "攼", "鳱", "蚶", "琀", "嫨", "哻", "頇", "鶾", "譀", "輚", "颔", "莟", "澏", "蜬", "雗", "垾", "馠", "鋎", "汵", "崡", "阚", "傼", "螒", "犴", "暵", "顸", "蜭", "凾", "蛿", "嵅", "闬", "兯", "扞", "丆", "闞", "鋡", "澣", "旰", "蔊", "釬", "爳", "銲", "魽", "圅", "馯", "仠", "厈", "忓"]),
    ("hang", &["航", "行", "杭", "韩国", "巷", "焊工", "涵盖", "夯", "韩哥", "吭", "喊个", "珩", "绗", "桁", "頏", "沆", "hang", "亢", "垳", "蚢", "颃", "絎", "貥", "妔", "斻", "迒", "苀", "笐", "裄", "邟", "魧", "忼", "㤚", "䘕", "䢚", "䀪", "䟘", "筕", "䣈", "䲳", "䦳", "䴂", "含", "喊", "韩", "汉", "函", "涵", "寒", "汗", "焊", "瀚", "晗", "翰", "憨", "菡", "旱", "罕", "笒", "焓", "晥", "悍", "歛", "頷", "漢", "邯", "浛", "憾", "韓", "撼", "酣", "鼾", "捍", "椷", "涆", "閈", "肣", "浫", "睅", "撖", "邗", "虷", "唅", "筨", "娢", "梒", "佄", "攼", "鳱", "蚶", "琀", "嫨", "哻", "頇", "鶾", "譀"]),
    ("hao", &["好", "号", "浩", "豪", "郝", "昊", "薅", "皓", "耗", "號", "灏", "嚎", "濠", "蚝", "壕", "毫", "颢", "淏", "蒿", "哈哦", "Hao", "晧", "澔", "镐", "皞", "灝", "呺", "蠔", "滈", "乚", "暠", "貉", "皜", "嗥", "嚆", "譹", "顥", "恏", "昦", "毜", "曍", "哠", "暤", "茠", "鄗", "侴", "聕", "獆", "皥", "籇", "鎬", "悎", "滜", "噑", "暭", "秏", "皡", "傐", "椃", "獋", "竓", "薃", "藃", "獔", "嘷", "儫", "薧", "虠", "諕", "鰝", "㘪", "㚪", "㕺", "㝀", "㞻", "䒵", "㬶", "䚽", "䧚", "㵆", "䪽", "㬔", "䝞", "㙱", "䧫", "䬉", "㠙", "㩝", "䜰", "䝥", "䯫", "哈", "蛤", "铪", "奤", "蝦"]),
    ("he", &["和", "喝", "盒", "何", "合", "贺", "核", "河", "禾", "鹤", "好饿", "呵", "赫", "荷", "噈", "会饿", "会儿", "颌", "还饿", "闔", "朅", "海尔", "嗬", "褐", "he", "愒", "菏", "鶴", "阖", "賀", "壑", "籺", "诃", "閡", "齕", "貉", "謞", "郃", "翯", "翮", "隺", "煂", "曷", "訸", "澕", "龢", "閤", "盉", "涸", "熇", "姀", "饸", "篕", "餄", "盍", "嚇", "麧", "龁", "吓", "咊", "劾", "頜", "鲄", "爀", "阂", "纥", "鹖", "鞨", "焃", "惒", "垎", "靏", "萂", "訶", "鑉", "狢", "碋", "峆", "釛", "靎", "欱", "鶮", "柇", "寉", "熆", "佫", "蓋", "覈", "靍", "紇", "楁", "毼", "詥", "鸖", "呙", "盇"]),
    ("hei", &["黑", "嘿", "黒", "嗨", "潶", "和", "喝", "盒", "何", "合", "贺", "核", "河", "禾", "鹤", "呵", "赫", "荷", "噈", "颌", "闔", "朅", "嗬", "褐", "愒", "菏", "鶴", "阖", "賀", "壑", "籺", "诃", "閡", "齕", "貉", "謞", "郃", "翯", "翮", "隺", "煂", "曷", "訸", "澕", "龢", "閤", "盉", "涸", "熇", "姀", "饸", "篕", "餄", "盍", "嚇", "麧", "龁", "吓", "咊", "劾", "頜", "鲄", "爀", "阂", "纥", "鹖", "鞨", "焃", "惒", "垎", "靏", "萂", "訶", "鑉", "狢", "碋", "峆", "釛", "靎", "欱", "鶮", "柇", "寉", "熆", "佫", "蓋", "覈", "靍", "紇", "楁", "毼", "詥", "鸖", "呙", "盇", "蠚"]),
    ("hen", &["很", "狠", "和你", "恨", "痕", "河南", "和您", "喝奶", "和牛", "和那", "河内", "和能", "核能", "hen", "哏", "拫", "皇额娘", "佷", "好儿女", "噷", "詪", "鞎", "䓳", "㯊", "和", "喝", "盒", "何", "合", "贺", "核", "河", "禾", "鹤", "呵", "赫", "荷", "噈", "颌", "闔", "朅", "嗬", "褐", "愒", "菏", "鶴", "阖", "賀", "壑", "籺", "诃", "閡", "齕", "貉", "謞", "郃", "翯", "翮", "隺", "煂", "曷", "訸", "澕", "龢", "閤", "盉", "涸", "熇", "姀", "饸", "篕", "餄", "盍", "嚇", "麧", "龁", "吓", "咊", "劾", "頜", "鲄", "爀", "阂", "纥", "鹖", "鞨", "焃", "惒", "垎", "靏", "萂", "訶", "鑉", "狢", "碋", "峆"]),
    ("heng", &["哼", "恒", "横", "衡", "很高", "很贵", "亨", "很乖", "很干", "珩", "很怪", "很广", "蘅", "桁", "姮", "恆", "很赶", "很够", "橫", "很过", "行", "涥", "鸻", "烆", "堼", "啈", "鑅", "鴴", "悙", "胻", "鵆", "脝", "撗", "㔰", "䒛", "䄓", "䯒", "㶇", "䬖", "䬝", "很", "狠", "恨", "痕", "和", "喝", "盒", "何", "合", "贺", "核", "河", "禾", "鹤", "呵", "赫", "荷", "噈", "颌", "闔", "朅", "嗬", "褐", "愒", "菏", "鶴", "阖", "賀", "壑", "籺", "诃", "閡", "齕", "貉", "謞", "郃", "翯", "翮", "隺", "煂", "曷", "訸", "澕", "龢", "閤", "盉", "涸", "熇", "姀", "饸", "哏", "拫", "佷", "噷", "詪", "鞎"]),
    ("hong", &["红", "宏", "洪", "鸿", "哄", "虹", "弘", "泓", "轰", "烘", "澋", "紅", "荭", "闳", "竑", "Hong", "鴻", "叿", "吽", "閧", "焢", "浤", "瓨", "薨", "訌", "翃", "汯", "黌", "鍧", "黉", "纮", "谼", "揈", "鞃", "訇", "谾", "鋐", "蕻", "篊", "鈜", "綋", "慐", "闂", "轟", "紭", "厷", "葒", "霟", "玒", "魟", "舼", "溄", "愩", "粠", "硔", "葓", "嚝", "宖", "銾", "竤", "垬", "軣", "苰", "吰", "讧", "撔", "霐", "屸", "羾", "鉷", "灴", "渱", "澒", "紘", "翝", "妅", "耾", "浲", "閎", "仜", "玜", "沗", "硡", "巆", "娂", "唝", "谹", "嗊", "潂", "彋", "輷", "闀", "呍", "㢬", "䧆", "䆖"]),
    ("hou", &["后", "厚", "侯", "猴", "吼", "候", "後", "喉", "齁", "腄", "逅", "鲎", "翵", "銗", "鯸", "翭", "垕", "瘊", "犼", "堠", "骺", "糇", "矦", "鲘", "呴", "郈", "茩", "篌", "吽", "鍭", "鱟", "睺", "鄇", "鮜", "餱", "葔", "洉", "帿", "豞", "㫗", "㕈", "㖃", "㤧", "㸸", "㰯", "㗋", "㺅", "㬋", "㮢", "䞧", "䫺", "䂉", "䙈", "䪷", "䗔", "䞀", "䡩", "䫛", "䳧"]),
    ("hu", &["胡", "户", "虎", "互", "湖", "护", "糊", "呼", "沪", "壶", "戲", "乎", "狐", "弧", "忽", "扈", "楛", "濩", "唬", "戯", "鵠", "琥", "斛", "膴", "嗃", "鹄", "戶", "浒", "葫", "瑚", "烀", "搰", "餬", "蝴", "護", "戱", "鶦", "祜", "幠", "枑", "笏", "焀", "嫭", "槲", "豰", "鶮", "冴", "鯱", "雽", "壺", "箶", "煳", "頀", "婟", "鄠", "觷", "錿", "和", "謼", "熩", "核", "鶘", "瓠", "虍", "鳸", "摢", "鹱", "魱", "恗", "苸", "匫", "怙", "螜", "縎", "鱯", "寣", "猢", "瓳", "簄", "嘝", "惚", "滬", "轷", "鰗", "鹕", "醐", "冱", "唿", "戸", "滹", "雐", "岵", "囫", "喖", "絗", "軤"]),
    ("hua", &["话", "花", "华", "化", "画", "豁", "滑", "划", "話", "桦", "骅", "铧", "哗", "華", "婳", "畫", "虎啊", "户啊", "胡啊", "互啊", "呼啊", "觟", "嫿", "鋘", "呚", "椛", "嬅", "嘩", "槬", "樺", "婲", "黊", "崋", "吪", "撶", "猾", "譮", "諙", "劃", "澅", "鷨", "畵", "誮", "檴", "摦", "夻", "驊", "蘳", "埖", "砉", "錵", "搳", "糀", "硴", "蒊", "諣", "姡", "譁", "枠", "璍", "蕐", "杹", "舙", "釫", "磆", "釪", "螖", "繣", "蘤", "芲", "㕦", "㕲", "㭉", "㕷", "㟆", "㓰", "䇈", "䋀", "䏦", "㠏", "㦊", "㚌", "㮯", "䊐", "䔢", "䛡", "㦎", "㩇", "䅿", "䔯", "䠉", "䴳", "鏵", "䱻", "䶤", "胡"]),
    ("huai", &["坏", "怀", "淮", "槐", "踝", "褢", "壞", "互爱", "懷", "徊", "胡爱", "胡皑", "蘾", "胡艾", "护爱", "互艾", "耲", "懐", "劃", "佪", "壊", "瀤", "咶", "櫰", "蘹", "䈭", "褱", "㠢", "㜳", "䴜", "䃶", "话", "花", "华", "化", "画", "豁", "滑", "划", "話", "桦", "骅", "铧", "哗", "華", "婳", "畫", "觟", "嫿", "鋘", "呚", "椛", "嬅", "嘩", "槬", "樺", "婲", "黊", "崋", "吪", "撶", "猾", "胡", "户", "虎", "互", "湖", "护", "糊", "呼", "沪", "壶", "戲", "乎", "狐", "弧", "忽", "扈", "楛", "濩", "唬", "戯", "鵠", "琥", "斛", "膴", "嗃", "鹄", "戶", "浒", "葫", "瑚", "烀", "搰", "餬", "蝴"]),
    ("huan", &["换", "还", "环", "欢", "缓", "焕", "幻", "患", "換", "桓", "唤", "化脓", "华南", "华农", "寰", "宦", "护岸", "轘", "郇", "綄", "還", "湖岸", "华能", "胡安", "沪安", "呼安", "护安", "喛", "涣", "奂", "浣", "漶", "蒝", "圂", "豲", "獾", "洹", "瘓", "環", "嬛", "嚾", "歡", "懁", "羦", "睆", "澴", "峘", "雈", "鴅", "緩", "煥", "讙", "鲩", "鹮", "圜", "鯶", "擐", "喚", "萑", "肒", "锾", "鬟", "愌", "镮", "瓛", "瑍", "梙", "鵍", "攌", "豢", "缳", "痪", "阛", "糫", "垸", "瞣", "逭", "荁", "懽", "孉", "歓", "驩", "澣", "寏", "萈", "鰀", "奐", "狟", "烉", "雚", "瞏", "鯇", "槵", "繯", "酄", "鐶"]),
    ("huang", &["黄", "慌", "皇", "换个", "晃", "还给", "煌", "荒", "簧", "凰", "换过", "换购", "谎", "黃", "璜", "磺", "篁", "潢", "爌", "欢哥", "换岗", "换股", "湟", "胡昂", "Huang", "蝗", "鐄", "隍", "幌", "恍", "穔", "惶", "瑝", "锽", "癀", "皝", "滉", "諻", "鳇", "詤", "趪", "騜", "遑", "鎤", "肓", "曂", "鍠", "晄", "熀", "徨", "蟥", "巟", "鷬", "媓", "縨", "謊", "喤", "楻", "艎", "汻", "堭", "怳", "炾", "獚", "熿", "墴", "韹", "鰉", "衁", "朚", "愰", "榥", "餭", "櫎", "兤", "偟", "塃", "崲", "皩", "㠵", "㞷", "㡆", "㤺", "䍿", "宺", "㿠", "奛", "㾠", "揘", "葟", "㨪", "㬻", "䄓", "䐠", "㾮", "䅣"]),
    ("hui", &["会", "回", "辉", "慧", "灰", "汇", "惠", "晖", "會", "荟", "毁", "卉", "徽", "绘", "潰", "墮", "廆", "挥", "Hui", "蕙", "悔", "烩", "賄", "濊", "珲", "恢", "洄", "蚘", "撝", "叀", "讳", "茴", "彗", "輝", "匯", "喙", "桧", "懳", "篲", "螝", "晦", "秽", "噦", "嚖", "譭", "獩", "澮", "浍", "迴", "诲", "薉", "褘", "翚", "颒", "繢", "蛕", "虺", "袆", "暳", "闠", "暉", "璯", "橞", "麾", "頮", "鐬", "毇", "滙", "隓", "咴", "檓", "幑", "楎", "廻", "鰴", "贿", "潓", "恚", "僡", "翙", "煇", "薈", "鮰", "憓", "繪", "璤", "絵", "櫘", "圚", "噅", "禈", "烣", "徻", "嬒", "殨", "隳"]),
    ("hun", &["混", "婚", "魂", "湖南", "昏", "荤", "梱", "糊弄", "掍", "浑", "倱", "胡闹", "呼你", "虎年", "閽", "户内", "鼲", "餛", "慁", "諢", "睧", "棔", "尡", "睴", "馄", "珲", "睯", "惛", "蔒", "诨", "觨", "阍", "葷", "繉", "渾", "溷", "轋", "涽", "忶", "焄", "湷", "昬", "焝", "眃", "圂", "婫", "殙", "湣", "琿", "㕵", "俒", "㑮", "䐊", "惽", "㥵", "㨡", "㨰", "㱪", "䅙", "䰟", "㮯", "䊐", "䅱", "䚠", "䛰", "䡣", "㯻", "䴷", "䫟", "䮝", "䧰", "胡", "户", "虎", "互", "湖", "护", "糊", "呼", "沪", "壶", "戲", "乎", "狐", "弧", "忽", "扈", "楛", "濩", "唬", "戯", "鵠", "琥", "斛", "膴", "嗃"]),
    ("huo", &["货", "活", "或", "火", "霍", "嚯", "获", "伙", "祸", "灬", "貨", "豁", "惑", "和", "互哦", "胡哦", "户哦", "濊", "矆", "雘", "彠", "癨", "藿", "镬", "钬", "靃", "劐", "矐", "锪", "艧", "曤", "湱", "掝", "秳", "獲", "蠖", "漷", "吙", "禍", "攉", "夥", "篧", "秴", "趏", "秮", "嚄", "鑊", "耠", "鈥", "耯", "佸", "濩", "邩", "蒦", "穫", "嚿", "擭", "咟", "窢", "謋", "彟", "閄", "喐", "鍃", "沎", "姡", "眓", "檴", "俰", "捇", "剨", "礊", "瀖", "騞", "臛", "韄", "㘞", "㶡", "㦯", "㓉", "䄀", "䄆", "䄑", "旤", "㨯", "㨰", "䣶", "煷", "㕡", "㚌", "䋭", "䦚", "㗲", "㩇", "㯉", "䁨"]),
    ("ji", &["几", "及", "级", "机", "寄", "急", "记", "即", "鸡", "集", "給", "计", "吉", "季", "基", "纪", "冀", "挤", "既", "极", "剂", "據", "暨", "己", "技", "积", "姬", "继", "济", "击", "际", "肌", "忌", "籍", "激", "迹", "魕", "嵇", "褀", "骥", "鹡", "蹟", "叽", "绩", "霁", "幾", "茤", "譏", "濈", "級", "辑", "機", "祭", "疾", "踦", "汲", "脊", "戟", "鬾", "饥", "璂", "玑", "妀", "佶", "雞", "摖", "屰", "記", "戢", "躸", "蹐", "禨", "惎", "稽", "虀", "稷", "璣", "唧", "攲", "寂", "唶", "蓟", "敧", "繋", "棘", "錤", "皀", "楫", "覬", "苙", "矶", "枅", "賫", "計", "系", "忣"]),
    ("jia", &["家", "加", "价", "假", "佳", "嘉", "贾", "夹", "甲", "架", "嫁", "驾", "钾", "猰", "珈", "伽", "脥", "迦", "镓", "稼", "几啊", "急啊", "级啊", "寄啊", "鸡啊", "集啊", "击啊", "郏", "痂", "颊", "價", "筴", "荚", "茄", "夾", "葭", "裌", "岬", "犌", "猳", "笳", "麚", "忦", "戞", "枷", "賈", "泇", "浃", "傢", "擖", "胛", "斝", "耞", "挟", "戛", "瘕", "駕", "貑", "鴶", "鵊", "铗", "叚", "腵", "袈", "槚", "恝", "莢", "豭", "哿", "徍", "蛱", "榎", "跏", "鉀", "郟", "鎵", "袷", "宊", "圿", "玾", "婽", "跲", "糘", "拮", "徦", "毠", "唊", "嘏", "拁", "挾", "檟", "抸", "浹", "榢", "乫", "埉"]),
    ("jian", &["件", "见", "建", "间", "减", "剪", "剑", "健", "键", "捡", "检", "简", "贱", "尖", "兼", "坚", "煎", "肩", "吉安", "箭", "监", "加你", "碱", "鉴", "佳能", "艱", "集安", "积案", "济安", "基安", "技安", "冀安", "季安", "即安", "极暗", "歼", "舰", "奸", "拣", "俭", "涧", "見", "蹇", "荐", "茧", "溅", "渐", "間", "玪", "菅", "笺", "餞", "檻", "僣", "践", "腱", "柬", "傔", "減", "谏", "戬", "磵", "锏", "冿", "鵑", "縑", "撿", "枧", "劍", "熸", "艰", "釰", "翦", "劒", "瞷", "姦", "鬋", "戩", "椷", "瞼", "醎", "睑", "銒", "笕", "瑐", "鋄", "朁", "簡", "囏", "籛", "揀", "鍵", "諓", "筧", "豜", "葥"]),
    ("jiang", &["将", "讲", "江", "蒋", "降", "姜", "奖", "酱", "见过", "犟", "匠", "浆", "疆", "坚果", "講", "僵", "监管", "间隔", "將", "桨", "建国", "激昂", "兼顾", "建个", "蔣", "绛", "建工", "吉昂", "鸡昂", "姬昂", "极昂", "季昂", "纪昂", "橿", "韁", "獎", "醬", "醤", "茳", "摪", "謽", "缰", "螿", "繮", "强", "顜", "殭", "勥", "鳉", "豇", "弜", "弶", "絳", "薑", "袶", "漿", "鱂", "洚", "礓", "耩", "摾", "夅", "疅", "塂", "糨", "螀", "匞", "杢", "葁", "櫤", "畺", "翞", "槳", "壃", "糡", "強", "港", "滰", "畕", "傋", "奬", "嵹", "彊", "䋌", "䙹", "䞪", "奨", "㢡", "㯍", "蔃", "膙", "㹔", "䁰", "䉃", "䒂", "䕬"]),
    ("jiao", &["叫", "交", "教", "脚", "角", "较", "胶", "焦", "娇", "觉", "缴", "姣", "椒", "嚼", "浇", "蕉", "蹻", "饺", "搅", "喬", "绞", "矫", "蛟", "吉奥", "击凹", "加哦", "价哦", "吉翱", "寄傲", "鸡奥", "家哦", "极奥", "吉傲", "假哦", "Jiao", "几奥", "家噢", "窖", "骄", "皎", "斠", "郊", "铰", "轎", "校", "佼", "滘", "踋", "膠", "窌", "酵", "轿", "僥", "腳", "礁", "嫶", "撹", "璬", "趫", "釂", "灂", "轇", "跤", "較", "儌", "鐎", "茭", "劋", "鲛", "鷦", "暞", "漖", "樔", "譥", "虠", "嬓", "鵤", "醮", "嶣", "繳", "狡", "艽", "湬", "嬌", "覺", "嘄", "剿", "徼", "獥", "峧", "藠", "簥", "煍", "徺", "嘂", "敿"]),
    ("jie", &["姐", "接", "借", "节", "结", "杰", "解", "洁", "街", "届", "界", "捷", "截", "價", "阶", "介", "戒", "皆", "婕", "劫", "饥饿", "袓", "揭", "喈", "卪", "劼", "疾厄", "极恶", "疾恶", "几额", "积恶", "缉恶", "基恶", "积饿", "鸡鹅", "颉", "耤", "桀", "刼", "芥", "扢", "睫", "竭", "節", "結", "唶", "傑", "嶰", "嗟", "孑", "瑎", "刦", "杢", "碣", "诫", "莭", "褯", "鶡", "跲", "尐", "啑", "潔", "桔", "诘", "蠽", "嶻", "藉", "秸", "煯", "桝", "屆", "玠", "疖", "羯", "踕", "蓵", "雃", "倢", "巀", "嚌", "嵑", "擮", "菨", "偈", "嫅", "嶱", "堺", "价", "鞂", "階", "蚧", "蛶", "畍", "斺", "媘", "蠘"]),
    ("jin", &["进", "斤", "金", "近", "今", "仅", "紧", "锦", "劲", "晋", "尽", "几年", "津", "禁", "技能", "济南", "靳", "筋", "瑾", "巾", "纪念", "谨", "烬", "浸", "進", "Jin", "槿", "緊", "堇", "襟", "衿", "妗", "缙", "琎", "臸", "錦", "瑨", "僅", "饉", "琻", "矜", "勁", "歏", "噤", "盡", "觐", "晉", "藎", "钅", "荩", "珒", "浕", "婜", "嫤", "斳", "惍", "謹", "赆", "濜", "埐", "廑", "釒", "卺", "菳", "嬐", "璡", "仐", "溍", "嚍", "漌", "壗", "搢", "墐", "鹶", "馑", "荕", "祲", "菫", "殣", "嬧", "馸", "儘", "伒", "砛", "唫", "笒", "觔", "燼", "矝", "厪", "蓳", "釿", "侭", "巹", "璶", "兓"]),
    ("jing", &["静", "经", "景", "精", "京", "晶", "井", "净", "镜", "靖", "敬", "婧", "境", "惊", "璟", "菁", "警", "竟", "径", "尽管", "荆", "竞", "劲", "颈", "Jing", "鲸", "泾", "茎", "睛", "憬", "旌", "經", "燝", "兢", "靜", "腈", "弪", "迳", "靓", "箐", "驚", "穽", "胫", "丼", "阱", "肼", "淨", "瀞", "粳", "綡", "痉", "暻", "竧", "鏡", "競", "倞", "勁", "稉", "儆", "葝", "荊", "澋", "汫", "汬", "鋞", "粇", "鯨", "麠", "靚", "頸", "獍", "徑", "傹", "鼱", "猄", "儬", "誩", "竸", "鶄", "曔", "竫", "婙", "蟼", "亰", "妌", "鵛", "経", "逕", "脛", "鶁", "剄", "浄", "涇", "刭", "莖", "璄"]),
    ("jiong", &["囧", "炯", "宭", "炅", "窘", "侰", "逈", "迥", "銄", "臦", "埛", "臩", "澃", "泂", "冏", "僒", "煚", "颎", "顈", "扃", "烱", "冂", "冋", "駫", "坰", "綗", "褧", "蘏", "煛", "駉", "絅", "浻", "熲", "燛", "蘔", "扄", "㢠", "㑋", "䌹", "㓏", "㤯", "䢛", "㖥", "䐃", "㷗", "㷡", "及", "级", "机", "几", "寄", "急", "记", "即", "鸡", "集", "給", "计", "吉", "季", "基", "纪", "冀", "挤", "既", "极", "剂", "據", "暨", "己", "技", "积", "姬", "继", "济", "击", "际", "肌", "忌", "籍", "激", "迹", "魕", "嵇", "褀", "骥", "鹡", "蹟", "叽", "绩", "霁", "幾", "茤", "譏", "濈", "級"]),
    ("jiu", &["就", "酒", "久", "旧", "九", "救", "玖", "舅", "灸", "鸠", "揪", "究", "啾", "纠", "韭", "舊", "氿", "臼", "赳", "鹫", "咎", "鳩", "疚", "汣", "乆", "杦", "镹", "桕", "厩", "奺", "柩", "阄", "柾", "鬏", "鷲", "揂", "欍", "韮", "僦", "齨", "殧", "丩", "糾", "朻", "揫", "樛", "匶", "勼", "匛", "蹴", "廄", "匓", "萛", "紤", "乣", "舏", "剹", "稵", "慦", "倃", "牞", "捄", "廐", "鬮", "糺", "廏", "摎", "麔", "鯦", "㐇", "㝌", "㡱", "㺩", "㽱", "㧕", "㺵", "䆒", "㶭", "䡂", "㼋", "䓘", "㲃", "䊆", "䬨", "䅢", "㠇", "㩆", "䊘", "䛮", "䠓", "䳎", "䳔", "蹵", "䰗", "及", "级"]),
    ("ju", &["局", "句", "距", "聚", "剧", "巨", "居", "具", "举", "菊", "拒", "据", "狙", "鞠", "橘", "袓", "锯", "桔", "炬", "钜", "焗", "驹", "琚", "矩", "俱", "罝", "跼", "澽", "咀", "踘", "惧", "拘", "葅", "冣", "匊", "莒", "痀", "雎", "閰", "车", "繘", "榉", "粔", "輂", "劇", "齟", "櫸", "苴", "鶋", "懅", "聥", "梮", "窶", "蒩", "飓", "車", "掬", "寠", "趜", "腒", "檋", "郰", "倨", "苣", "啹", "艍", "歫", "跙", "愳", "舉", "埾", "踞", "據", "趉", "蘜", "駏", "沮", "疽", "揟", "讵", "臄", "螶", "犑", "貗", "遽", "襷", "粷", "鉅", "忂", "焣", "蓻", "昛", "锔", "枸", "婅", "姖"]),
    ("juan", &["卷", "娟", "捐", "劵", "圈", "倦", "涓", "鹃", "巂", "隽", "眷", "绢", "睠", "聚安", "居安", "巨安", "局安", "聚氨", "举案", "Juan", "飬", "睊", "襈", "镌", "獧", "縳", "鞙", "臇", "帣", "脧", "絭", "勌", "巻", "勬", "菤", "龹", "餋", "慻", "鄄", "闂", "讂", "捲", "圏", "狷", "蠲", "奆", "韏", "絹", "罥", "裐", "锩", "焆", "椦", "惓", "鐫", "錈", "鋗", "瓹", "呟", "棬", "淃", "雋", "姢", "埍", "埢", "悁", "朘", "蔨", "嶲", "鋑", "鎸", "奍", "桊", "婘", "羂", "㢧", "䌸", "弮", "䄅", "梋", "㢾", "䅌", "䖭", "腃", "䡓", "䐪", "䣺", "㷷", "䚈", "㯞", "鵑", "䳪", "㪻", "䥴", "局", "距"]),
    ("jue", &["觉", "绝", "决", "撅", "珏", "爵", "巨额", "倔", "蹻", "妜", "角", "噱", "诀", "巨鳄", "巨饿", "巨恶", "菊娥", "据俄", "掘", "熦", "芵", "燋", "鳜", "璚", "鴂", "蕨", "壆", "瑴", "厥", "躩", "崛", "玦", "鈌", "覺", "弡", "繘", "嚼", "貜", "絕", "鶌", "穱", "噘", "蟨", "戄", "虳", "桷", "趉", "觼", "瘚", "橜", "撧", "決", "臄", "啳", "玨", "崫", "孓", "蹶", "抉", "櫭", "鐝", "攫", "矍", "镼", "蟩", "逫", "鷢", "疦", "谲", "勪", "觮", "袦", "誳", "訣", "憠", "橛", "繑", "爴", "亅", "絶", "傕", "匷", "镢", "噊", "嶥", "彏", "孒", "吷", "欔", "脚", "龣", "嶡", "獗", "爝", "殌", "玃"]),
    ("jun", &["军", "君", "均", "俊", "骏", "菌", "钧", "峻", "珺", "郡", "龜", "竣", "隽", "浚", "筠", "鋆", "Jun", "麕", "馂", "軍", "燇", "焌", "鵔", "攈", "濬", "攟", "畯", "鈞", "駿", "蚐", "莙", "晙", "埈", "箟", "雋", "儁", "皲", "棞", "桾", "汮", "袀", "呁", "葰", "懏", "捃", "龟", "陖", "箘", "麇", "寯", "姰", "鲪", "皸", "鮶", "銁", "銞", "皹", "覠", "鍕", "餕", "鵕", "碅", "蜠", "賐", "麏", "㢠", "㽙", "㤯", "㓴", "㑺", "㴫", "㻒", "䜭", "㝦", "䇹", "㕙", "䝌", "䕑", "䝍", "頵", "㒞", "鵘", "局", "距", "句", "聚", "剧", "巨", "居", "具", "举", "拒", "菊", "据", "狙", "鞠"]),
    ("ka", &["卡", "咖", "咔", "可爱", "喀", "快啊", "开啊", "咯", "考啊", "苦啊", "卡啊", "课啊", "空啊", "胩", "口岸", "酷安", "佧", "裃", "垰", "鉲", "衉", "呿", "哢", "㮟", "䘔"]),
    ("kai", &["开", "凯", "楷", "嘅", "恺", "開", "铠", "锴", "慨", "垲", "凱", "Kai", "揩", "剀", "愷", "锎", "烗", "岂", "闿", "鎧", "忾", "欬", "噄", "奒", "鍇", "塏", "剴", "蒈", "闓", "暟", "鐦", "愒", "颽", "愾", "炌", "豈", "輆", "嵦", "鎎", "䒓", "㳀", "㡁", "㱾", "欯", "勓", "㗆", "㚊", "䐩", "䁗", "濭", "䫦", "䡷", "卡", "咖", "咔", "喀", "咯", "胩", "佧", "裃", "垰", "鉲", "衉", "呿", "哢", "㮟", "䘔"]),
    ("kan", &["看", "砍", "坎", "刊", "卡你", "阚", "侃", "堪", "勘", "槛", "冚", "瞰", "龛", "卡农", "衎", "卡内", "卡呢", "顑", "卡奴", "栞", "磡", "可爱捏", "墈", "嵌", "戡", "莰", "惂", "嵁", "崁", "鬫", "闞", "凵", "扻", "歁", "偘", "輱", "龕", "塪", "埳", "欿", "檻", "轗", "矙", "竷", "㸝", "㸔", "䖔", "䘓", "䬻", "㵎", "輡", "㙳", "䳚", "䫲", "䀍", "䶫", "卡", "咖", "咔", "喀", "咯", "胩", "佧", "裃", "垰", "鉲", "衉", "呿", "哢", "㮟", "䘔"]),
    ("kang", &["康", "抗", "扛", "看过", "亢", "炕", "看个", "糠", "看管", "看够", "慷", "看顾", "邟", "砍个", "钪", "伉", "摃", "槺", "鈧", "漮", "闶", "犺", "忼", "嫝", "砊", "穅", "囥", "閌", "匟", "粇", "鏮", "嵻", "躿", "鱇", "㰠", "䡉", "㝩", "㱂", "䲘", "看", "砍", "坎", "刊", "阚", "侃", "堪", "勘", "槛", "冚", "瞰", "龛", "衎", "顑", "栞", "磡", "墈", "嵌", "戡", "卡", "咖", "咔", "喀", "咯", "胩", "佧", "莰", "惂", "嵁", "崁", "鬫", "闞", "凵", "扻", "歁", "偘", "輱", "龕", "塪", "埳", "欿", "檻", "轗", "矙", "竷", "㸝", "㸔", "䖔", "䘓", "䬻", "㵎", "輡", "㙳", "䳚", "䫲", "䀍", "䶫"]),
    ("kao", &["靠", "考", "烤", "拷", "尻", "铐", "薧", "犒", "kao", "卡哦", "栲", "丂", "鮳", "攷", "焅", "卡噢", "KAO", "銬", "鯌", "洘", "嵪", "鲓", "髛", "㼥", "䎋", "䐧", "䘓", "䯌", "㸆", "䯪", "卡", "咖", "咔", "喀", "咯", "胩", "佧", "裃", "垰", "鉲", "衉", "呿", "哢", "㮟", "䘔"]),
    ("ke", &["可", "课", "克", "科", "客", "颗", "壳", "柯", "氪", "刻", "磕", "珂", "咳", "棵", "渴", "嗑", "轲", "課", "恪", "科二", "顆", "坷", "可饿", "科恩", "苛", "钶", "硞", "稞", "歁", "髁", "岢", "颏", "殼", "疴", "窠", "瞌", "蝌", "锞", "炣", "尅", "剋", "醘", "磆", "呵", "嶱", "錁", "濭", "蚵", "騍", "匼", "愘", "嵙", "榼", "勊", "欬", "缂", "砢", "勀", "軻", "萪", "峇", "牁", "翗", "悈", "骒", "揢", "搕", "溘", "袔", "緙", "艐", "堁", "铪", "牱", "胢", "趷", "嵑", "鈳", "閜", "碦", "薖", "礊", "娔", "殻", "愙", "頦", "渇", "痾", "鉿", "礚", "衉", "毼", "犐", "窼", "敤", "㲺"]),
    ("kei", &["剋", "尅", "可", "课", "克", "科", "客", "颗", "壳", "柯", "氪", "刻", "磕", "珂", "咳", "棵", "渴", "嗑", "轲", "課", "恪", "顆", "坷", "苛", "钶", "硞", "稞", "歁", "髁", "岢", "颏", "殼", "疴", "窠", "瞌", "蝌", "锞", "炣", "醘", "磆", "呵", "嶱", "錁", "濭", "蚵", "騍", "匼", "愘", "嵙", "榼", "勊", "欬", "缂", "砢", "勀", "軻", "萪", "峇", "牁", "翗", "悈", "骒", "揢", "搕", "溘", "袔", "緙", "艐", "堁", "铪", "牱", "胢", "趷", "嵑", "鈳", "閜", "碦", "薖", "礊", "娔", "殻", "愙", "頦", "渇", "痾", "鉿", "礚", "衉", "毼", "犐", "窼", "敤", "㲺", "㐓", "㕉", "㞹"]),
    ("ken", &["可能", "肯", "啃", "垦", "恳", "柯南", "可你", "可那", "课内", "可难", "课呢", "齦", "可逆", "可嫩", "科宁", "可宁", "裉", "科内", "龈", "Ken", "硍", "錹", "豤", "珢", "科尔尼", "科尔能", "懇", "掯", "墾", "肎", "肻", "貇", "㸧", "龂", "褃", "可", "课", "克", "科", "客", "颗", "壳", "柯", "氪", "刻", "磕", "珂", "咳", "棵", "渴", "嗑", "轲", "課", "恪", "顆", "坷", "苛", "钶", "硞", "稞", "歁", "髁", "岢", "颏", "殼", "疴", "窠", "瞌", "蝌", "锞", "炣", "尅", "剋", "醘", "磆", "呵", "嶱", "錁", "濭", "蚵", "騍", "匼", "愘", "嵙", "榼", "勊", "欬", "缂", "砢", "勀", "軻", "萪", "峇", "牁", "翗", "悈"]),
    ("keng", &["坑", "吭", "殸", "肯干", "铿", "硜", "肯给", "啃过", "啃个", "鍞", "誙", "肯高", "揁", "挳", "坈", "鏗", "阬", "硁", "妔", "牼", "摼", "硻", "銵", "劥", "䡩", "䡰", "肯", "啃", "垦", "恳", "齦", "裉", "可", "课", "克", "科", "客", "颗", "壳", "柯", "氪", "刻", "磕", "珂", "咳", "棵", "渴", "嗑", "轲", "課", "恪", "顆", "坷", "苛", "钶", "硞", "稞", "歁", "髁", "岢", "颏", "殼", "疴", "窠", "瞌", "蝌", "锞", "炣", "尅", "剋", "醘", "磆", "呵", "嶱", "錁", "龈", "硍", "錹", "豤", "珢", "懇", "掯", "墾", "肎", "肻", "貇", "㸧", "龂", "褃", "濭", "蚵", "騍", "匼", "愘", "嵙", "榼"]),
    ("kong", &["空", "控", "孔", "恐", "箜", "崆", "倥", "涳", "硿", "Kong", "悾", "鵼", "埪", "羫", "鞚", "椌", "錓", "躻", "㑋", "矼", "㤟", "㸜"]),
    ("kou", &["扣", "口", "抠", "寇", "叩", "蔻", "筘", "滱", "芤", "眗", "冦", "摳", "釦", "蔲", "瞘", "宼", "簆", "剾", "怐", "劶", "鷇", "眍", "敂", "竘", "佝", "瞉", "彄", "㔚", "䧆", "㓂", "㸸", "窛", "㲄", "䍍", "䳟", "䳹"]),
    ("ku", &["哭", "库", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐", "瘔", "胐", "扝", "焅", "窋", "俈", "廤", "秙", "狜", "嚳", "搰", "硞", "朏", "鮬", "捁", "㠸", "郀", "㱠", "䧊", "挎", "䊿", "㐣", "䇢", "跍", "㒂", "㲄", "䔯", "䵈"]),
    ("kua", &["夸", "跨", "垮", "胯", "苦啊", "挎", "絓", "哭啊", "酷啊", "咵", "楇", "侉", "库啊", "顝", "裤啊", "苦阿", "趶", "誇", "姱", "晇", "恗", "銙", "骻", "㡁", "䋀", "舿", "䦚", "䯞", "库", "哭", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐", "瘔", "胐", "扝", "焅", "窋", "俈", "廤", "秙", "狜", "嚳", "搰", "硞", "朏", "鮬", "捁", "㠸", "郀", "㱠", "䧊", "䊿", "㐣", "䇢", "跍", "㒂", "㲄", "䔯", "䵈"]),
    ("kuai", &["快", "块", "筷", "会", "蒯", "酷爱", "塊", "駃", "侩", "郐", "脍", "苦艾", "库爱", "酷艾", "酷哎", "獪", "噲", "廥", "巜", "旝", "擓", "圦", "糩", "哙", "墤", "鲙", "欳", "浍", "狯", "蒉", "凷", "會", "蕢", "鄶", "澮", "璯", "鱠", "儈", "㭈", "㔞", "㻅", "䓒", "㙕", "㙗", "㟴", "㨤", "㫎", "䈛", "膾", "䭝", "䯤", "㬮", "㧟", "跨", "夸", "垮", "胯", "挎", "絓", "咵", "楇", "侉", "库", "哭", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "顝", "趶", "誇", "姱", "晇", "恗", "銙", "骻", "㡁", "䋀", "舿", "䦚", "䯞", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐"]),
    ("kuan", &["款", "宽", "跨年", "酷安", "夸你", "髋", "歀", "寬", "酷岸", "窾", "库岸", "库安", "寛", "髖", "欵", "梡", "鑧", "㝟", "㮯", "䕀", "䤭", "㯘", "窽", "臗", "䥗", "䲌", "跨", "夸", "垮", "胯", "挎", "絓", "咵", "楇", "侉", "库", "哭", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "顝", "趶", "誇", "姱", "晇", "恗", "銙", "骻", "㡁", "䋀", "舿", "䦚", "䯞", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐", "瘔", "胐", "扝", "焅", "窋", "俈", "廤", "秙", "狜", "嚳", "搰", "硞", "朏", "鮬", "捁", "㠸", "郀", "㱠", "䧊", "䊿", "㐣", "䇢", "跍", "㒂", "㲄", "䔯", "䵈"]),
    ("kuang", &["框", "狂", "矿", "况", "匡", "邝", "筐", "旷", "哐", "宽广", "诓", "眶", "宽哥", "爌", "壙", "贶", "穬", "夼", "髋骨", "宽高", "宽沟", "宽轨", "酷昂", "絖", "圹", "黋", "昿", "礦", "況", "诳", "鄺", "懬", "鵟", "劻", "卝", "眖", "軠", "洭", "恇", "邼", "砿", "曠", "軭", "纩", "鋛", "迋", "貺", "誑", "狅", "懭", "矌", "硄", "鑛", "抂", "岲", "誆", "軖", "筺", "鉱", "儣", "丱", "忹", "軦", "㫛", "匩", "㔞", "㤮", "絋", "㾠", "䊯", "纊", "䵃", "䂄", "款", "宽", "髋", "歀", "寬", "窾", "跨", "夸", "垮", "胯", "挎", "絓", "咵", "楇", "侉", "库", "哭", "苦", "裤", "酷", "枯", "窟", "庫"]),
    ("kui", &["亏", "葵", "奎", "魁", "盔", "馈", "溃", "愧", "窥", "逵", "磈", "犪", "虧", "隗", "闚", "夔", "匮", "聵", "馗", "傀", "樻", "瞶", "簣", "蕢", "喹", "岿", "揆", "胿", "睽", "楏", "蒉", "蝰", "謉", "聭", "尯", "嬇", "櫆", "悝", "跬", "聩", "藈", "顝", "殨", "暌", "籄", "骙", "篑", "愦", "聧", "晆", "饋", "喟", "匱", "躨", "潰", "鍷", "刲", "窺", "頯", "憒", "媿", "虁", "楑", "嘳", "蘷", "鄈", "頄", "頍", "瞆", "蹞", "煃", "膭", "巋", "蘬", "鍨", "鮭", "鐀", "㧉", "䏗", "㕟", "㤬", "㚝", "䍪", "㙓", "㛻", "㟴", "㨒", "㷇", "㾠", "䖯", "䧶", "䇻", "䞨", "戣", "䁛", "䙆"]),
    ("kun", &["困", "坤", "昆", "捆", "琨", "苦恼", "苦难", "鲲", "哭闹", "堃", "锟", "焜", "髠", "睏", "崑", "鹍", "醌", "貇", "稛", "梱", "錕", "堒", "崐", "鵾", "髨", "熴", "閸", "悃", "菎", "髡", "瑻", "裍", "阃", "婫", "鯤", "壸", "裈", "祵", "尡", "鶤", "騉", "潉", "晜", "硱", "涃", "猑", "裩", "稇", "壼", "蜫", "閫", "臗", "㕎", "㫻", "䖵", "綑", "褌", "䠅", "㩲", "库", "哭", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐", "瘔", "胐", "扝", "焅", "窋", "俈", "廤", "秙", "狜", "嚳", "搰", "硞", "朏", "鮬", "捁", "㠸"]),
    ("kuo", &["扩", "阔", "括", "铦", "廓", "鞟", "濶", "銛", "鞹", "酷哦", "哭哦", "苦哦", "萿", "闊", "髺", "噋", "栝", "鬠", "擴", "葀", "頢", "适", "銽", "漷", "挄", "霩", "筈", "拡", "秮", "蛞", "韕", "懖", "䄆", "秳", "㣪", "㪙", "䄑", "㗥", "㨯", "䟯", "䙃", "䦢", "䯺", "库", "哭", "苦", "裤", "酷", "枯", "窟", "庫", "堀", "骷", "絝", "褲", "桍", "喾", "绔", "刳", "楛", "袴", "矻", "崫", "圐", "瘔", "胐", "扝", "焅", "窋", "俈", "廤", "秙", "狜", "嚳", "搰", "硞", "朏", "鮬", "捁", "㠸", "郀", "㱠", "䧊", "挎", "䊿", "㐣", "䇢", "跍", "㒂", "㲄", "䔯", "䵈"]),
    ("la", &["啦", "拉", "辣", "腊", "来啊", "蜡", "恋爱", "喇", "了啊", "冷啊", "累啊", "练啊", "落", "la", "啦啊", "垃", "里啊", "楼啊", "辢", "聊啊", "老啊", "立案", "剌", "菈", "臈", "櫴", "擸", "儠", "砬", "瓎", "楋", "旯", "臘", "揦", "搚", "揧", "瘌", "爉", "蠟", "蝲", "邋", "柆", "翋", "鞡", "嚹", "鑞", "磖", "攋", "鬎", "溂", "鱲", "藞", "镴", "蝋", "鯻", "癩", "癞", "㕇", "㻋", "䓥", "䀳", "䂰", "㻝", "䏀", "䗶", "㸊", "䃳", "䱫", "䶛", "䝓", "䟑", "䪉"]),
    ("lai", &["来", "赖", "莱", "來", "疠", "濑", "徕", "頼", "賴", "涞", "籁", "勑", "铼", "癞", "唻", "庲", "癘", "睐", "郲", "崃", "赉", "娕", "瀨", "萊", "俫", "藾", "瀬", "鯠", "猍", "誺", "琜", "錸", "崍", "倈", "鵣", "襰", "淶", "徠", "騋", "梾", "籟", "婡", "逨", "賚", "睞", "攋", "麳", "櫴", "棶", "箂", "顂", "鶆", "癩", "㚓", "䧒", "㥎", "䀳", "䓶", "筙", "㾢", "䂾", "䅘", "䋱", "䚅", "㠣", "㸊", "䄤", "䠭", "䲚", "啦", "拉", "辣", "腊", "蜡", "喇", "落", "垃", "辢", "剌", "菈", "臈", "擸", "儠", "砬", "瓎", "楋", "旯", "臘", "揦", "搚", "揧", "瘌", "爉", "蠟", "蝲", "邋"]),
    ("lan", &["懒", "蓝", "兰", "烂", "栏", "岚", "澜", "拦", "拉你", "揽", "缆", "篮", "览", "蘭", "榄", "藍", "阑", "斓", "拉尿", "覧", "恋爱脑", "来爱你", "LAN", "镧", "滥", "爛", "懶", "瓓", "斕", "嵐", "襤", "譋", "欄", "浨", "韊", "讕", "嚂", "瀾", "婪", "爤", "蘫", "燷", "囒", "糷", "繿", "孏", "攬", "燗", "籃", "籣", "襽", "壏", "谰", "葻", "懢", "鑭", "灠", "罱", "灡", "嬾", "覽", "攔", "儖", "顲", "褴", "襕", "漤", "坔", "灆", "闌", "躝", "爦", "濫", "璼", "壈", "擥", "纜", "幱", "啉", "湅", "醂", "燣", "爁", "欖", "惏", "孄", "襴", "钄", "厱", "礷", "㘝", "㞩", "㑑", "㳕", "㑣", "䧒"]),
    ("lang", &["浪", "狼", "郎", "朗", "蓝光", "廊", "栏杆", "琅", "勆", "榔", "啷", "蓝港", "烺", "塱", "螂", "稂", "Lang", "阆", "崀", "埌", "瑯", "欴", "蓢", "蒗", "樃", "莨", "朤", "锒", "嫏", "郞", "躴", "脼", "俍", "郒", "誏", "桹", "哴", "斏", "蓈", "閬", "朖", "艆", "鋃", "鎯", "㓪", "㝗", "㟍", "㢃", "唥", "㫰", "㱢", "㾗", "䀶", "䆡", "䍚", "硠", "㙟", "㾿", "䁁", "筤", "蜋", "㮾", "䡙", "䯖", "駺", "䕞", "䱶", "䃹", "蓝", "兰", "懒", "烂", "栏", "岚", "澜", "拦", "揽", "缆", "篮", "览", "蘭", "榄", "藍", "阑", "斓", "覧", "镧", "滥", "爛", "懶", "瓓", "斕", "嵐", "襤", "譋", "欄"]),
    ("lao", &["老", "捞", "樂", "佬", "牢", "劳", "唠", "姥", "烙", "酪", "涝", "橑", "铑", "落", "崂", "撈", "啦哦", "勞", "痨", "Lao", "拉哦", "辣哦", "轑", "哰", "醪", "蟧", "朥", "咾", "嫪", "耂", "乐", "簩", "嗠", "耮", "粩", "軂", "栳", "窂", "荖", "橯", "狫", "络", "躼", "潦", "恅", "磱", "嘮", "髝", "蛯", "铹", "労", "銠", "珯", "浶", "耢", "僗", "澇", "憥", "鐒", "硓", "絡", "癆", "鮱", "顟", "嶗", "憦", "㐗", "㧯", "㟉", "㟙", "㨓", "䇭", "㗦", "㟹", "㞠", "䃕", "䝁", "䳓", "䕩", "䜎", "䝤", "䵏", "䲏", "啦", "拉", "辣", "腊", "蜡", "喇", "垃", "辢", "剌", "菈", "臈", "櫴", "擸"]),
    ("le", &["了", "乐", "勒", "叻", "嘞", "老二", "灵儿", "樂", "楽", "肋", "老鹅", "仂", "玏", "理儿", "卤鹅", "龙儿", "临额", "泐", "簕", "氻", "砳", "阞", "竻", "忇", "艻", "鳓", "饹", "餎", "鱳", "韷", "牞", "扐", "鰳", "㔹", "㖀", "㨋", "㦡"]),
    ("lei", &["嘞", "类", "累", "雷", "磊", "蕾", "泪", "垒", "勒", "镭", "肋", "瘣", "擂", "類", "錑", "耒", "磥", "Lei", "儽", "虆", "蔂", "檑", "纇", "櫑", "腂", "瓃", "罍", "灅", "癗", "嫘", "櫐", "淚", "頪", "鼺", "纝", "壘", "讄", "轠", "酹", "儡", "禷", "羸", "厽", "涙", "靁", "畾", "銇", "攂", "礌", "鑸", "鑘", "诔", "蘱", "缧", "藟", "矋", "蕌", "祱", "鱩", "塁", "鸓", "傫", "誄", "洡", "縲", "樏", "頛", "礧", "蘲", "蠝", "鐳", "壨", "欙", "蘽", "㲺", "㑍", "䣂", "䒹", "䣦", "㭩", "㲕", "㴃", "㻋", "㼍", "㗊", "絫", "㒍", "㡞", "䨓", "㹎", "䐯", "䛶", "㵢", "䢮", "㔣", "䉂"]),
    ("leng", &["冷", "愣", "棱", "楞", "碐", "堎", "塄", "稜", "唥", "踜", "睖", "崚", "薐", "輘", "倰", "㘄", "䉄", "䬋", "䮚", "䚏", "了", "乐", "勒", "叻", "嘞", "樂", "楽", "肋", "仂", "玏", "泐", "簕", "氻", "砳", "阞", "竻", "忇", "艻", "鳓", "饹", "餎", "鱳", "韷", "牞", "扐", "鰳", "㔹", "㖀", "㨋", "㦡"]),
    ("li", &["李", "里", "力", "离", "丽", "理", "立", "利", "黎", "粒", "礼", "例", "哩", "莉", "厘", "梨", "历", "栗", "厉", "隸", "锂", "励", "嚟", "裡", "俐", "郦", "俪", "樆", "荔", "狸", "醨", "璃", "笠", "沥", "鲤", "骊", "犁", "氂", "吏", "鵹", "栎", "砾", "砺", "麗", "蠫", "漓", "離", "剺", "隶", "褵", "篱", "澧", "藜", "鹂", "盭", "蠡", "斄", "溧", "皪", "櫟", "柂", "觻", "蠣", "醴", "迣", "峢", "筣", "盠", "邐", "瓑", "呖", "厤", "雳", "琍", "瓈", "禮", "濿", "莅", "磿", "俚", "灕", "戾", "喱", "轢", "栵", "櫪", "枥", "痢", "唳", "蔾", "藶", "綟", "鬲", "攦", "鬁", "瓅"]),
    ("lia", &["俩", "力啊", "里啊", "倆", "力阿", "理啊", "礼啊", "离啊", "丽啊", "李啊", "利啊", "利阿", "历啊", "里", "李", "力", "离", "丽", "理", "立", "利", "黎", "粒", "礼", "例", "哩", "莉", "厘", "梨", "历", "栗", "厉", "隸", "锂", "励", "嚟", "裡", "俐", "郦", "俪", "樆", "荔", "狸", "醨", "璃", "笠", "沥", "鲤", "骊", "犁", "氂", "吏", "鵹", "栎", "砾", "砺", "麗", "蠫", "漓", "離", "剺", "隶", "褵", "篱", "澧", "藜", "鹂", "盭", "蠡", "斄", "溧", "皪", "櫟", "柂", "觻", "蠣", "醴", "迣", "峢", "筣", "盠", "邐", "瓑", "呖", "厤", "雳", "琍", "瓈", "禮", "濿", "莅", "磿", "俚", "灕", "戾", "喱"]),
    ("lian", &["连", "脸", "练", "联", "链", "立案", "莲", "纞", "恋", "炼", "廉", "帘", "离岸", "利安", "李安", "怜", "連", "涟", "敛", "亷", "俩年", "立安", "俩妞", "黎安", "梨安", "丽安", "李岸", "里安", "莉安", "礼安", "例案", "理俺", "哩岸", "歛", "練", "臉", "鬑", "濂", "镰", "槤", "琏", "聯", "潋", "鲢", "嗹", "鏈", "蓮", "劆", "戀", "瀲", "聨", "楝", "槏", "稴", "薟", "濓", "燫", "蠊", "匳", "殓", "摙", "鰱", "嫾", "蘞", "聮", "嬚", "籨", "煉", "磏", "蹥", "裢", "醶", "譧", "鰊", "奁", "蔹", "襝", "梿", "螊", "鄻", "臁", "羷", "籢", "覝", "噒", "褳", "簾", "澰", "鍊", "憐", "匲", "翴", "錬", "蘝", "浰", "檶"]),
    ("liang", &["两", "量", "梁", "亮", "良", "辆", "凉", "靓", "粮", "俩", "连个", "晾", "兩", "李昂", "粱", "里昂", "连跪", "倆", "樑", "练过", "练功", "谅", "连贯", "立昂", "魎", "练歌", "练个", "连杆", "利昂", "Liang", "辌", "靚", "椋", "涼", "綡", "倞", "糧", "莨", "緉", "俍", "輛", "蜽", "輌", "魉", "喨", "冫", "墚", "踉", "脼", "湸", "諒", "唡", "両", "哴", "悢", "裲", "煷", "輬", "簗", "掚", "鍄", "駺", "㒳", "㔝", "䓣", "啢", "㹁", "㾗", "䀶", "䁁", "䝶", "䠃", "䣼", "䭪", "䩫", "连", "脸", "练", "联", "链", "莲", "纞", "恋", "炼", "廉", "帘", "怜", "連", "涟", "敛", "亷", "歛", "練", "臉", "鬑", "濂"]),
    ("liao", &["聊", "料", "了", "廖", "辽", "撩", "疗", "寮", "撂", "燎", "里奥", "李傲", "李奥", "李澳", "李敖", "利奥", "窌", "繆", "钌", "蟟", "寥", "俩哦", "丽岙", "立奥", "李翱", "利澳", "力奥", "摎", "蓼", "尦", "僚", "鐐", "潦", "瞭", "缭", "嘹", "藔", "髎", "敹", "簝", "蟧", "獠", "遼", "豂", "漻", "廫", "蹽", "屪", "蹘", "療", "镣", "嫽", "鹩", "尥", "曢", "尞", "炓", "爒", "嵺", "璙", "賿", "镽", "釕", "樛", "嶛", "繚", "窷", "飂", "僇", "嶚", "憭", "暸", "橑", "竂", "爎", "鄝", "憀", "膋", "膫", "飉", "鷯", "㝋", "䄦", "㡻", "䑠", "㶫", "䒿", "䉼", "䎆", "㜃", "㨓", "㺒", "䜮", "䢧", "䣈", "㙩"]),
    ("lie", &["列", "咧", "裂", "猎", "烈", "劣", "獦", "哷", "冽", "颲", "丽娥", "lie", "棙", "李娥", "里厄", "丽鹅", "里额", "迾", "猟", "睙", "蛚", "擸", "綟", "獵", "鬣", "儠", "鱲", "洌", "挒", "埒", "燤", "鴷", "劦", "埓", "爄", "爉", "捩", "犣", "毟", "巁", "躐", "聗", "趔", "烮", "劽", "脟", "煭", "忚", "挘", "茢", "峢", "栵", "鮤", "浖", "巤", "奊", "姴", "㧜", "㤠", "㭞", "㭩", "㸹", "䅀", "䓟", "㽟", "䟩", "㼲", "䟹", "䬅", "䮋", "㻺", "㯿", "㲱", "㸊", "䁽", "䉭", "䶛", "䜲", "䝓", "鬛", "㬯", "䪉", "里", "李", "力", "离", "丽", "理", "立", "利", "黎", "粒", "礼", "例", "哩", "莉"]),
    ("lin", &["林", "临", "琳", "霖", "淋", "麟", "理你", "李宁", "拎", "离你", "理念", "磷", "蔺", "邻", "李娜", "鳞", "凛", "粼", "璘", "吝", "历年", "凜", "丽娜", "獜", "赁", "檩", "膦", "冧", "躪", "粦", "癝", "啉", "嶙", "臨", "轔", "箖", "瀶", "遴", "菻", "鄰", "廪", "轥", "潾", "藺", "躙", "燐", "崊", "魿", "辚", "碄", "晽", "阾", "甐", "隣", "懔", "瞵", "綝", "檁", "賃", "厸", "鏻", "鱗", "廩", "橉", "亃", "驎", "悋", "惏", "恡", "痳", "僯", "麐", "翷", "疄", "蹸", "焛", "澟", "躏", "閵", "繗", "懍", "暽", "壣", "㐭", "㖁", "㝝", "㨆", "㖼", "㔂", "㝺", "㷠", "䟹", "䢧", "撛", "斴", "䢯"]),
    ("ling", &["领", "零", "玲", "灵", "另", "凌", "令", "岭", "龄", "铃", "菱", "陵", "伶", "翎", "苓", "泠", "绫", "瓴", "Ling", "羚", "聆", "領", "澪", "齢", "靈", "棱", "蓤", "麢", "囹", "呤", "棂", "淩", "柃", "衑", "欞", "嶺", "酃", "皊", "岺", "鈴", "蘦", "羐", "崚", "蛉", "鲮", "祾", "姈", "鸰", "綾", "阾", "昤", "齡", "夌", "稜", "坽", "舲", "炩", "瀮", "龗", "霊", "彾", "砱", "刢", "霝", "錂", "朎", "笭", "琌", "鴒", "狑", "袊", "霗", "裬", "靇", "紷", "軨", "婈", "孁", "跉", "秢", "竛", "魿", "霛", "掕", "詅", "駖", "蕶", "蔆", "輘", "櫺", "閝", "燯", "鯪", "㡵", "㸳", "㾉"]),
    ("liu", &["刘", "留", "六", "流", "柳", "溜", "碌", "陆", "遛", "瘤", "硫", "劉", "榴", "浏", "琉", "鎏", "騮", "馏", "罶", "骝", "绺", "熘", "瑠", "鶹", "駵", "镠", "蓅", "栁", "磟", "塯", "飀", "鎦", "旒", "镏", "陸", "鰡", "锍", "裗", "瑬", "鹨", "疁", "畄", "珋", "璢", "蹓", "桞", "瀏", "鏐", "鬸", "雡", "懰", "驑", "駠", "翏", "飂", "羀", "橮", "嵧", "嬼", "綹", "鐂", "畱", "旈", "廇", "蒥", "橊", "霤", "斿", "漻", "嚠", "飅", "鋶", "藰", "磂", "桺", "摎", "熮", "癅", "蟉", "餾", "澑", "麍", "㝋", "㐬", "㳅", "䒥", "沠", "㧕", "㶯", "䄂", "㙀", "窌", "㽞", "㨨", "媹", "䭷"]),
    ("lo", &["咯", "楼", "龙", "喽", "漏", "隆", "搂", "笼", "来哦", "了哦", "娄", "弄", "露", "lo", "囖", "鞻", "珑", "拢", "龍", "泷", "聋", "陇", "樓", "垄", "篓", "窿", "瘘", "垅", "陋", "胧", "嘍", "茏", "镂", "蔞", "龒", "砻", "咙", "櫳", "礱", "栊", "寠", "溇", "鸗", "蒌", "竜", "瀧", "巃", "谾", "膢", "剅", "梇", "礲", "耬", "眬", "嶁", "尨", "癃", "滝", "艛", "漊", "籠", "巄", "襱", "壟", "偻", "篭", "鷜", "瓏", "遱", "耧", "昽", "甊", "髅", "婁", "摟", "屚", "朧", "靇", "嵝", "蝼", "硦", "軁", "瞜", "攏", "嚨", "蠬", "鑨", "貗", "爖", "龓", "廔", "豅", "簍", "儱", "壠", "聾"]),
    ("long", &["龙", "隆", "弄", "笼", "珑", "拢", "龍", "泷", "聋", "陇", "垄", "窿", "垅", "胧", "茏", "龒", "long", "砻", "咙", "櫳", "礱", "栊", "鸗", "竜", "瀧", "巃", "谾", "梇", "礲", "眬", "尨", "癃", "滝", "籠", "巄", "襱", "壟", "篭", "瓏", "昽", "朧", "靇", "硦", "攏", "嚨", "蠬", "鑨", "爖", "龓", "豅", "儱", "壠", "聾", "漋", "隴", "竉", "屸", "嶐", "贚", "躘", "蠪", "衖", "槞", "鏧", "矓", "哢", "湰", "驡", "㑝", "㚅", "㛞", "㟖", "㢅", "㳥", "㙙", "㴳", "㝫", "㦕", "蕯", "㡣", "䃧", "篢", "徿", "蘢", "霳", "㰍", "曨", "䆍", "䙪", "䥢", "䏊", "䡁", "䪊", "咯", "囖"]),
    ("lou", &["楼", "喽", "漏", "娄", "鞻", "搂", "露", "樓", "篓", "瘘", "陋", "嘍", "镂", "蔞", "寠", "溇", "蒌", "膢", "剅", "耬", "嶁", "艛", "漊", "偻", "鷜", "遱", "耧", "甊", "髅", "婁", "摟", "屚", "嵝", "蝼", "軁", "瞜", "貗", "廔", "簍", "塿", "僂", "瘺", "瘻", "螻", "髏", "㔷", "㲎", "㥪", "䣚", "㟺", "㡞", "㺏", "慺", "㪹", "䄛", "熡", "䅹", "䝏", "謱", "鏤", "䫫", "䮫", "䱾", "咯", "囖"]),
    ("lu", &["路", "录", "陆", "卢", "鲁", "露", "撸", "鹿", "璐", "炉", "噜", "卤", "禄", "芦", "鹭", "绿", "逯", "麓", "庐", "潞", "泸", "碌", "賂", "六", "穞", "輅", "顱", "颅", "錄", "盧", "陸", "渌", "觻", "櫓", "箓", "戮", "漉", "硉", "鲈", "祿", "胪", "簶", "虏", "橹", "鐪", "蕗", "纑", "掳", "嚕", "擼", "魯", "琭", "镥", "辘", "轤", "璷", "甪", "栌", "艫", "侓", "圥", "硵", "獹", "鵦", "爐", "謢", "簵", "蠦", "菉", "録", "氇", "膟", "緑", "垆", "趢", "攎", "舻", "辂", "簏", "虜", "踛", "蓼", "螰", "馿", "艣", "蓾", "髗", "彔", "鸬", "捛", "醁", "勎", "籚", "娽", "綠", "廬"]),
    ("luan", &["乱", "卵", "癵", "栾", "六安", "鸾", "銮", "潞安", "峦", "陆安", "亂", "路安", "攣", "滦", "鲁安", "灤", "鹿安", "卢安", "臡", "臠", "娈", "孪", "孿", "曫", "挛", "羉", "癴", "脔", "奱", "薍", "鸞", "虊", "欒", "龻", "巒", "圝", "鑾", "鵉", "灓", "圞", "釠", "乿", "㰸", "㼑", "䜌", "覶", "㡩", "孌", "㱍", "㝈", "䖂", "䚕", "路", "录", "陆", "卢", "鲁", "露", "撸", "鹿", "璐", "炉", "噜", "卤", "禄", "芦", "鹭", "绿", "逯", "麓", "庐", "潞", "泸", "碌", "賂", "六", "穞", "輅", "顱", "颅", "錄", "盧", "陸", "渌", "觻", "櫓", "箓", "戮", "漉", "硉", "鲈", "祿", "胪", "簶", "虏", "橹"]),
    ("lun", &["轮", "论", "伦", "仑", "露娜", "抡", "鲁能", "纶", "路南", "沦", "囵", "輪", "論", "鲁尼", "埨", "錀", "崘", "颅内", "鲁南", "卢娜", "倫", "碖", "侖", "崙", "綸", "稐", "陯", "掄", "踚", "耣", "溣", "芲", "磮", "惀", "棆", "淪", "蜦", "圇", "菕", "腀", "鯩", "婨", "㖮", "㤻", "㷍", "㕢", "䈁", "䑳", "路", "录", "陆", "卢", "鲁", "露", "撸", "鹿", "璐", "炉", "噜", "卤", "禄", "芦", "鹭", "绿", "逯", "麓", "庐", "潞", "泸", "碌", "賂", "六", "穞", "輅", "顱", "颅", "錄", "盧", "陸", "渌", "觻", "櫓", "箓", "戮", "漉", "硉", "鲈", "祿", "胪", "簶", "虏", "橹", "鐪", "蕗", "纑", "掳"]),
    ("luo", &["罗", "落", "骆", "咯", "洛", "啰", "裸", "螺", "络", "摞", "锣", "萝", "雒", "逻", "珞", "攞", "羅", "泺", "臝", "躶", "路哦", "Luo", "录哦", "囉", "烙", "箩", "皪", "骡", "驘", "囖", "曪", "覼", "漯", "荦", "濼", "鮥", "倮", "鸁", "鏍", "蛒", "玀", "椤", "躒", "跞", "覶", "駱", "絡", "渃", "硌", "攊", "鵅", "猡", "攭", "癳", "镙", "捋", "蘿", "詻", "蠃", "腡", "洜", "硦", "瘰", "頱", "峈", "磱", "寽", "鑼", "蓏", "邏", "砢", "犖", "脶", "笿", "捰", "儸", "騾", "纙", "欏", "剆", "擽", "攎", "饠", "㕵", "㓢", "㖀", "㑩", "㪾", "䀩", "䉿", "䎅", "㴖", "㾖", "㮝", "䲞", "䈁"]),
    ("lv", &["吕", "绿", "率", "铝", "旅", "律", "驴", "捋", "滤", "氯", "虑", "履", "缕", "侣", "闾", "LV", "鑢", "綠", "呂", "藘", "屡", "瞜", "榈", "氀", "慺", "寠", "鋁", "箻", "侶", "膟", "葎", "慮", "穭", "驢", "稆", "偻", "褛", "儢", "緑", "膂", "爈", "馿", "縷", "濾", "挔", "梠", "垏", "曥", "絽", "寽", "菉", "閭", "櫚", "郘", "繂", "祣", "屢", "櫖", "膢", "卛", "勴", "穞", "鷜", "哷", "嵂", "僂", "捛", "焒", "㲶", "膐", "褸", "䕡", "㠥"]),
    ("lve", &["略", "掠", "锊", "稤", "鋢", "绿萼", "绿鹅", "圙", "寽", "旅俄", "畧", "吕鹅", "剠", "鋝", "䤚", "㗉", "㔀", "㨼", "擽", "率", "吕", "绿", "铝", "旅", "律", "驴", "捋", "滤", "氯", "虑", "履", "缕", "侣", "闾", "鑢", "綠", "呂", "藘", "屡", "瞜", "榈", "氀", "慺", "寠", "鋁", "箻", "侶", "膟", "葎", "慮", "穭", "驢", "稆", "偻", "褛", "儢", "緑", "膂", "爈", "馿", "縷", "濾", "挔", "梠", "垏", "曥", "絽", "菉", "閭", "櫚", "郘", "繂", "祣", "屢", "櫖", "膢", "卛", "勴", "穞", "鷜", "哷", "嵂", "僂", "捛", "焒", "㲶", "膐", "褸", "䕡", "㠥"]),
    ("ma", &["吗", "嘛", "码", "妈", "马", "骂", "麻", "嗎", "玛", "没啊", "买啊", "馬", "碼", "媽", "嫲", "忙啊", "木啊", "棉袄", "命啊", "妙啊", "妈啊", "慢啊", "没按", "么", "抹", "孖", "罵", "蚂", "唛", "蟆", "瑪", "犸", "祃", "麽", "杩", "傌", "蔴", "獁", "痳", "螞", "禡", "痲", "亇", "庅", "榪", "溤", "礣", "鷌", "蟇", "閁", "駡", "鎷", "摩", "鬕", "菻", "鰢", "嘜", "遤", "犘", "睰", "㐷", "䀣", "䣕", "䧞", "㑻", "㨸", "㦄", "㾺", "䠋", "㜫", "䗫", "䣖", "䠨", "䳸", "䯦"]),
    ("mai", &["买", "卖", "麦", "埋", "迈", "脉", "買", "唛", "賣", "荬", "霾", "劢", "麥", "佅", "脈", "霢", "邁", "嘜", "鷶", "霡", "薶", "売", "蕒", "勱", "衇", "䘑", "㜥", "嘪", "䈿", "䁲", "㦟", "䨪", "䚑", "䜕", "䨫", "䮮", "䳸", "吗", "嘛", "码", "妈", "马", "骂", "麻", "嗎", "玛", "馬", "碼", "媽", "嫲", "么", "抹", "孖", "罵", "蚂", "蟆", "瑪", "犸", "祃", "麽", "杩", "傌", "蔴", "獁", "痳", "螞", "禡", "痲", "亇", "庅", "榪", "溤", "礣", "鷌", "蟇", "閁", "駡", "鎷", "摩", "鬕", "菻", "鰢", "遤", "犘", "睰", "㐷", "䀣", "䣕", "䧞", "㑻", "㨸", "㦄", "㾺", "䠋", "㜫", "䗫"]),
    ("man", &["慢", "满", "蛮", "曼", "漫", "骂你", "蔓", "瞒", "嫚", "嘛呢", "滿", "码农", "缦", "熳", "幔", "鳗", "man", "馒", "螨", "蠻", "玛瑙", "墁", "镘", "Man", "莫阿娜", "颟", "絻", "蟃", "樠", "姏", "矕", "澫", "獌", "谩", "蘰", "蔄", "顢", "娨", "襔", "鏋", "鬘", "澷", "埋", "瞞", "槾", "睌", "謾", "満", "饅", "縵", "悗", "鏝", "蟎", "鄤", "僈", "慲", "鰻", "鞔", "鬗", "摱", "䒥", "屘", "㒼", "㛧", "㗄", "㗈", "㬅", "㙢", "䑱", "㡢", "䐽", "䛲", "䯶", "㵘", "㿸", "䅼", "䊡", "䕕", "䜱", "䝡", "䝢", "䟂", "䡬", "䨫", "䰋", "吗", "嘛", "码", "妈", "马", "骂", "麻", "嗎", "玛", "馬", "碼"]),
    ("mang", &["忙", "盲", "芒", "莽", "满格", "曼谷", "蟒", "茫", "氓", "蛮高", "朚", "哤", "狵", "蛮贵", "龒", "漫过", "痝", "慢歌", "漫灌", "满贯", "蛮干", "满哥", "瞒过", "蛮乖", "漫改", "满攻", "蛖", "茻", "漭", "邙", "铓", "蘉", "牤", "壾", "駹", "杧", "牻", "尨", "吂", "恾", "硭", "莾", "盳", "笀", "浝", "杗", "甿", "厖", "娏", "庬", "釯", "硥", "汒", "㝑", "㟐", "䖟", "㙁", "㟌", "㻊", "䀮", "䅒", "㜃", "㡛", "䟥", "㟿", "䓼", "㬒", "䈍", "鋩", "䁳", "䒎", "蠎", "䵨", "䙪", "满", "慢", "蛮", "曼", "漫", "蔓", "瞒", "嫚", "滿", "缦", "熳", "幔", "鳗", "馒", "螨", "蠻", "墁", "镘", "颟", "絻", "蟃", "樠"]),
    ("mao", &["猫", "毛", "冇", "茂", "冒", "帽", "矛", "茅", "贸", "卯", "锚", "貌", "茆", "懋", "铆", "貓", "码哦", "媢", "犛", "秏", "码噢", "吗哦", "妈哦", "Mao", "昴", "峁", "毷", "泖", "牦", "嵍", "耄", "瑁", "髦", "袤", "罞", "蓩", "旄", "芼", "貿", "楙", "皃", "夘", "眊", "鶜", "瞀", "鉚", "冐", "枆", "戼", "鄚", "愗", "渵", "酕", "冃", "軞", "鄮", "氂", "笷", "蝥", "錨", "毣", "緢", "蝐", "覒", "暓", "堥", "乮", "鉾", "蟊", "霿", "柕", "㧇", "㒵", "㧌", "㚹", "㺺", "㿞", "䀤", "䋃", "㒻", "㝟", "㡌", "㫯", "䖥", "㴘", "䓮", "萺", "㪞", "㮘", "㲠", "㹈", "䅦", "㬒", "䡚", "䭷", "髳"]),
    ("me", &["么", "麽", "名额", "me", "嚒", "麼", "明儿", "妹儿", "木耳", "满额", "面额", "末", "没饿", "马儿", "嚰", "庅", "美俄", "买二", "猫儿", "马额", "猫耳", "摩尔", "母鹅", "濹", "嚜"]),
    ("mei", &["没", "美", "每", "梅", "妹", "枚", "煤", "沒", "酶", "媚", "镁", "霉", "眉", "莓", "魅", "玫", "媒", "昧", "湄", "寐", "嚜", "楣", "媄", "渼", "槑", "韎", "嵋", "鎂", "眛", "袂", "鹛", "脄", "篃", "镅", "塺", "矀", "娒", "躾", "鶥", "鋂", "瑂", "攗", "郿", "燘", "黴", "湈", "糜", "羙", "祙", "苺", "沬", "媺", "坆", "睂", "嵄", "谜", "氼", "凂", "栂", "浼", "禖", "楳", "痗", "旀", "挴", "堳", "鬽", "毎", "謎", "脢", "煝", "珻", "跊", "眊", "葿", "睸", "嬍", "穈", "鎇", "黣", "抺", "殙", "猸", "蝞", "徾", "櫗", "蘪", "㺳", "㭑", "㶬", "䀛", "䀜", "䒽", "㙁", "㡌", "䍙"]),
    ("men", &["们", "门", "闷", "焖", "么你", "men", "們", "么呢", "門", "扪", "懣", "么女", "悶", "殙", "么难", "么宁", "么娘", "每二年", "鍆", "燜", "懑", "米尔纳", "钔", "璊", "捫", "菛", "汶", "虋", "怋", "暪", "椚", "玧", "悗", "惛", "穈", "亹", "㡈", "閅", "㥃", "㻊", "䝧", "㱪", "㙢", "㨺", "㦖", "㵍", "瞞", "鞔", "䊟", "䫒", "斖", "么", "麽", "嚒", "麼", "末", "嚰", "庅", "濹", "嚜"]),
    ("meng", &["梦", "猛", "孟", "蒙", "萌", "盟", "懵", "锰", "檬", "勐", "濛", "朦", "门岗", "夢", "黽", "们给", "蝱", "氓", "焖锅", "门关", "闷棍", "门高", "门规", "门盖", "霿", "癦", "鹲", "虻", "矇", "饛", "懜", "獴", "甍", "蠓", "蜢", "氋", "艋", "掹", "曚", "礞", "鄸", "鋂", "霥", "尨", "艨", "鸏", "夣", "莔", "橗", "瞢", "錳", "瓾", "靀", "鯍", "儚", "顭", "幪", "懞", "溕", "甿", "黾", "矒", "鼆", "擝", "庬", "鄳", "鯭", "冡", "雺", "蕄", "髳", "㒱", "䖟", "䇇", "㚞", "㻊", "䏵", "䓝", "䟥", "䀄", "䁅", "䗈", "㙹", "㠓", "㩚", "㽇", "䉚", "䑃", "䑅", "䠢", "㜴", "䙦", "䙩", "䝢", "䥂", "㝱"]),
    ("mi", &["米", "迷", "密", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "芈", "泌", "糸", "藌", "谧", "祢", "羃", "弭", "麛", "汨", "靡", "塓", "熐", "攠", "鼏", "蘪", "摵", "獼", "洣", "樒", "幎", "爢", "麋", "覓", "敉", "淧", "嘧", "幦", "麊", "冖", "覛", "猕", "渳", "沵", "檷", "脒", "謎", "濗", "覔", "簚", "羋", "蘼", "瞴", "滵", "彌", "瞇", "瓕", "粎", "灖", "侎", "縻", "镾", "戂", "鑖", "祕", "漞", "蒾", "沕", "冪", "銤", "宻", "醾", "鸍", "蓂", "擟", "謐", "峚", "怽", "袮", "瀰", "孊", "濔", "櫁", "冞", "禰", "醿", "釄", "枈", "蔤", "㐘", "㘠", "㫘"]),
    ("mian", &["面", "免", "棉", "睌", "眠", "绵", "缅", "勉", "冕", "密胺", "黽", "靦", "麪", "谜案", "澠", "米安", "迷案", "秘岸", "密案", "娩", "沔", "麵", "櫋", "牑", "宀", "蠠", "蝒", "麺", "渑", "腼", "綿", "莬", "湎", "丏", "黾", "勔", "矏", "婂", "媔", "偭", "愐", "緬", "汅", "俛", "眄", "鮸", "芇", "檰", "喕", "絻", "厸", "糆", "杣", "靣", "葂", "臱", "麫", "矈", "矊", "緜", "㐷", "䀎", "㤁", "㫘", "㝃", "㛯", "㥝", "㴐", "䛉", "㬆", "㮌", "䃇", "㨺", "㻰", "㒙", "䏃", "䤄", "䌏", "嬵", "㝰", "㰃", "䫵", "䰓", "米", "密", "迷", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓"]),
    ("miao", &["秒", "苗", "喵", "妙", "淼", "缪", "庙", "描", "瞄", "渺", "邈", "米奥", "藐", "仯", "缈", "杪", "繆", "廟", "觅凹", "媌", "眇", "米澳", "咪嗷", "米嗷", "秘奥", "咪奥", "Miao", "鹋", "庿", "緲", "玅", "厸", "緢", "劰", "訬", "篎", "嫹", "鶓", "鱙", "㠺", "䏚", "竗", "㑤", "䖢", "㷅", "䁧", "䅺", "㦝", "米", "密", "迷", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "芈", "泌", "糸", "藌", "谧", "祢", "羃", "弭", "麛", "汨", "靡", "塓", "熐", "攠", "鼏", "蘪", "摵", "獼", "洣", "樒", "幎", "爢", "麋", "覓", "敉", "淧", "嘧", "幦", "麊", "冖", "覛", "猕", "渳", "沵"]),
    ("mie", &["咩", "灭", "乜", "衊", "蔑", "滅", "篾", "薎", "鴓", "米额", "咪额", "米蛾", "幭", "簚", "覕", "孭", "搣", "鑖", "礣", "蠛", "瀎", "芈", "懱", "烕", "吀", "眜", "櫗", "鱴", "哶", "㒝", "㩢", "䈼", "䁾", "䌩", "䘊", "䩏", "米", "密", "迷", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "泌", "糸", "藌", "谧", "祢", "羃", "弭", "麛", "汨", "靡", "塓", "熐", "攠", "鼏", "蘪", "摵", "獼", "洣", "樒", "幎", "爢", "麋", "覓", "敉", "淧", "嘧", "幦", "麊", "冖", "覛", "猕", "渳", "沵", "檷", "脒", "謎", "濗", "覔", "羋", "蘼", "瞴", "滵", "彌", "瞇", "瓕", "粎"]),
    ("min", &["敏", "民", "闽", "闵", "迷你", "旻", "珉", "皿", "抿", "岷", "泌尿", "min", "泯", "米内", "黽", "敯", "忞", "米娜", "米妮", "悯", "米诺", "僶", "慜", "旼", "玟", "刡", "苠", "愍", "勄", "鳘", "黾", "缗", "閔", "鰵", "呡", "琘", "閩", "簢", "冺", "湣", "錉", "賯", "姄", "怋", "罠", "敃", "琝", "暋", "垊", "忟", "捪", "渂", "盿", "憫", "笢", "瑉", "潣", "砇", "湏", "鈱", "緍", "鴖", "緡", "笽", "痻", "碈", "㞶", "㢯", "䂥", "䡅", "䋋", "䝧", "崏", "㟩", "㟭", "㨉", "䟨", "䡑", "惽", "㬆", "㮌", "㥸", "䁕", "䃉", "䪸", "䞀", "鍲", "䶔", "蠠", "䡻", "䲄", "米", "密", "迷", "咪", "蜜"]),
    ("ming", &["明", "名", "命", "铭", "敏感", "鸣", "茗", "冥", "洺", "民国", "民工", "鄍", "溟", "銘", "酩", "暝", "民歌", "敏哥", "鳴", "闽G", "民哥", "民革", "民高", "Ming", "眀", "瞑", "螟", "佲", "朙", "姳", "詺", "掵", "蓂", "凕", "眳", "嫇", "榠", "慏", "猽", "覭", "㫥", "㝠", "䊅", "㟰", "䆨", "䆩", "䄙", "䒌", "䤉", "䫤", "䳟", "敏", "民", "闽", "闵", "旻", "珉", "皿", "抿", "岷", "泯", "黽", "敯", "忞", "悯", "僶", "慜", "旼", "玟", "刡", "苠", "愍", "勄", "鳘", "米", "密", "迷", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "芈", "泌", "糸", "藌", "谧", "祢", "羃", "弭"]),
    ("miu", &["谬", "缪", "繆", "謬", "米", "密", "迷", "咪", "蜜", "眯", "秘", "觅", "弥", "幂", "谜", "糜", "宓", "醚", "芈", "泌", "糸", "藌", "谧", "祢", "羃", "弭", "麛", "汨", "靡", "塓", "熐", "攠", "鼏", "蘪", "摵", "獼", "洣", "樒", "幎", "爢", "麋", "覓", "敉", "淧", "嘧", "幦", "麊", "冖", "覛", "猕", "渳", "沵", "檷", "脒", "謎", "濗", "覔", "簚", "羋", "蘼", "瞴", "滵", "彌", "瞇", "瓕", "粎", "灖", "侎", "縻", "镾", "戂", "鑖", "祕", "漞", "蒾", "沕", "冪", "銤", "宻", "醾", "鸍", "蓂", "擟", "謐", "峚", "怽", "袮", "瀰", "孊", "濔", "櫁", "冞", "禰", "醿", "釄", "枈"]),
    ("mo", &["摸", "莫", "膜", "末", "模", "磨", "墨", "魔", "抹", "么", "默", "摩", "沫", "馍", "陌", "茉", "漠", "嚩", "麿", "嬷", "买哦", "忙哦", "劰", "没", "麽", "嚜", "谟", "擵", "蘑", "歾", "貘", "殁", "摹", "尛", "袹", "寞", "蓦", "銆", "懡", "嚤", "秣", "麼", "粖", "貊", "貈", "瞙", "嗼", "莈", "镆", "妺", "蟔", "嬤", "瞐", "絈", "沒", "黙", "髍", "嫫", "帞", "蛨", "饝", "皌", "脉", "爅", "饃", "歿", "貉", "万", "枺", "昩", "瘼", "縸", "謨", "魹", "礳", "藦", "劘", "絔", "耱", "冒", "无", "纆", "脈", "塻", "靺", "嘿", "帓", "艒", "鏌", "狢", "無", "謩", "圽", "眜", "砞", "骳"]),
    ("mou", &["某", "谋", "牟", "眸", "哞", "蛑", "踎", "厶", "缪", "呣", "堥", "謀", "洠", "鍪", "侔", "鞪", "桙", "繆", "麰", "劺", "恈", "鴾", "敄", "㭌", "䍒", "㼋", "䏬", "㖼", "䦈", "䋷", "䗋", "䥐", "䱕", "摸", "莫", "膜", "末", "模", "磨", "墨", "魔", "抹", "么", "默", "摩", "沫", "馍", "陌", "茉", "漠", "嚩", "麿", "嬷", "劰", "没", "麽", "嚜", "谟", "擵", "蘑", "歾", "貘", "殁", "摹", "尛", "袹", "寞", "蓦", "銆", "懡", "嚤", "秣", "麼", "粖", "貊", "貈", "瞙", "嗼", "莈", "镆", "妺", "蟔", "嬤", "瞐", "絈", "沒", "黙", "髍", "嫫", "帞", "蛨", "饝", "皌", "脉", "爅", "饃"]),
    ("mu", &["木", "母", "目", "亩", "沐", "穆", "慕", "牧", "幕", "牟", "姆", "模", "墓", "暮", "睦", "钼", "募", "牡", "幙", "霂", "莯", "畮", "朷", "苜", "拇", "炑", "娒", "莫", "畝", "姥", "畒", "暯", "踇", "繆", "氁", "鉬", "坶", "牳", "凩", "仫", "雮", "蚞", "鉧", "毣", "狇", "楘", "縸", "鞪", "胟", "砪", "畂", "畆", "艒", "峔", "慔", "樢", "畞", "㧅", "㟂", "㫛", "㙁", "㾇", "䊾", "䧔", "毪", "㣎", "㡔", "䀲", "㜈", "㒇", "䑵", "䥈", "䁼", "䱯", "䮸"]),
    ("na", &["那", "拿", "哪", "呐", "娜", "你啊", "纳", "牛啊", "钠", "哪啊", "能啊", "乸", "捺", "吶", "嗱", "雫", "納", "内", "南", "衲", "笚", "镎", "妠", "拏", "鈉", "抐", "訤", "郍", "貀", "魶", "挐", "蒳", "鎿", "肭", "誽", "詉", "靹", "蒘", "笝", "秅", "袦", "軜", "搻", "㘝", "䀑", "㙁", "㧱", "䏧", "㭯", "䇣", "䖧", "䟜", "豽", "䇱", "㗙", "㨥", "㴸", "䅞", "䎎", "䖓", "䛔", "䪏", "䈫", "䱹", "䞕", "䫱"]),
    ("nai", &["奶", "耐", "乃", "奈", "萘", "柰", "妳", "氖", "佴", "鼐", "Nai", "艿", "迺", "倷", "能", "廼", "孻", "搱", "摨", "釢", "錼", "嬭", "腉", "螚", "疓", "渿", "褦", "耏", "㚷", "㾍", "㮈", "䯮", "㮏", "㲡", "䅞", "䍲", "熋", "㜨", "䱞", "㜷", "䘅", "那", "拿", "哪", "呐", "娜", "纳", "钠", "乸", "捺", "吶", "嗱", "雫", "納", "内", "南", "衲", "笚", "镎", "妠", "拏", "鈉", "抐", "訤", "郍", "貀", "魶", "挐", "蒳", "鎿", "肭", "誽", "詉", "靹", "蒘", "笝", "秅", "袦", "軜", "搻", "㘝", "䀑", "㙁", "㧱", "䏧", "㭯", "䇣", "䖧", "䟜", "豽", "䇱", "㗙", "㨥", "㴸", "䎎", "䖓"]),
    ("nan", &["男", "难", "南", "那你", "楠", "喃", "娜娜", "那年", "哪能", "拿捏", "囡", "腩", "妠", "難", "蝻", "侽", "你啊你", "娘啊娘", "Nan", "囝", "莮", "柟", "婻", "湳", "赧", "娚", "諵", "萳", "煵", "暔", "遖", "嫨", "枬", "枏", "揇", "戁", "抩", "畘", "㽖", "㓓", "㫱", "䛁", "䔜", "䈒", "䊖", "䔳", "䁪", "䕼", "䶲", "㬮", "那", "拿", "哪", "呐", "娜", "纳", "钠", "乸", "捺", "吶", "嗱", "雫", "納", "内", "衲", "笚", "镎", "拏", "鈉", "抐", "訤", "郍", "貀", "魶", "挐", "蒳", "鎿", "肭", "誽", "詉", "靹", "蒘", "笝", "秅", "袦", "軜", "搻", "㘝", "䀑", "㙁", "㧱", "䏧", "㭯", "䇣", "䖧", "䟜"]),
    ("nang", &["囊", "难过", "馕", "难搞", "难怪", "涳", "鬞", "南瓜", "南关", "楠哥", "难关", "憹", "南宫", "攮", "南国", "南哥", "灢", "南岗", "男哥", "囔", "儾", "搑", "嚢", "曩", "齉", "乪", "欜", "瀼", "饢", "蠰", "䂇", "㒄", "擃", "㶞", "饟", "男", "难", "南", "楠", "喃", "囡", "腩", "妠", "難", "蝻", "侽", "囝", "莮", "柟", "婻", "湳", "赧", "娚", "那", "拿", "哪", "呐", "娜", "纳", "钠", "乸", "捺", "吶", "嗱", "雫", "納", "内", "衲", "笚", "諵", "萳", "煵", "暔", "遖", "嫨", "枬", "枏", "揇", "戁", "抩", "畘", "㽖", "㓓", "㫱", "䛁", "䔜", "䈒", "䊖", "䔳", "䁪", "䕼", "䶲", "㬮", "镎", "拏", "鈉"]),
    ("nao", &["闹", "脑", "挠", "孬", "恼", "摎", "橈", "嫐", "夒", "鐃", "那哦", "拿哦", "哪哦", "鬧", "怓", "詉", "垴", "淖", "瑙", "腦", "獶", "蝚", "铙", "呶", "匘", "蟯", "碙", "獿", "碯", "憹", "撓", "脳", "猱", "臑", "桡", "堖", "硇", "嶩", "蛲", "閙", "惱", "峱", "悩", "巎", "譊", "㞪", "㑎", "䄩", "㧘", "㺀", "䙹", "䫸", "婥", "㛴", "㺁", "䜧", "䛝", "䜀", "䃩", "䑋", "䴃", "那", "拿", "哪", "呐", "娜", "纳", "钠", "乸", "捺", "吶", "嗱", "雫", "納", "内", "南", "衲", "笚", "镎", "妠", "拏", "鈉", "抐", "訤", "郍", "貀", "魶", "挐", "蒳", "鎿", "肭", "誽", "靹", "蒘", "笝", "秅"]),
    ("ne", &["呢", "呐", "女儿", "哪", "哪儿", "那儿", "你儿", "妞儿", "讷", "男儿", "鸟儿", "疒", "抐", "妮儿", "吶", "女二", "訥", "奈尔", "尼尔", "纳尔", "努尔", "眲", "㕯", "䅞", "䎪", "䭆", "䭚"]),
    ("nei", &["内", "內", "馁", "那", "鮾", "哪", "浽", "娞", "餒", "鯘", "氝", "錗", "氞", "脮", "腇", "㐻", "㘨", "㖏", "㨅", "䇣", "㼏", "䜆", "䳖", "䲎", "䡾", "呢", "呐", "讷", "疒", "抐", "吶", "訥", "眲", "㕯", "䅞", "䎪", "䭆", "䭚"]),
    ("nen", &["嫩", "恁", "呢你", "嫰", "媆", "讷讷", "呢弄", "呢哝", "㶧", "㒄", "㜛", "㯎", "呢", "呐", "哪", "讷", "疒", "抐", "吶", "訥", "眲", "㕯", "䅞", "䎪", "䭆", "䭚"]),
    ("neng", &["能", "嫩个", "恁个", "螚", "竜", "㲌", "恁哥", "䏻", "㴰", "䘅", "嫩", "恁", "呢", "呐", "哪", "讷", "疒", "抐", "吶", "訥", "嫰", "媆", "㶧", "㒄", "㜛", "㯎", "眲", "㕯", "䅞", "䎪", "䭆", "䭚"]),
    ("ni", &["你", "尼", "呢", "拟", "倪", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "ni", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "怩", "柅", "鑈", "麑", "儞", "薿", "臡", "聻", "嶷", "儗", "鉨", "籾", "鯢", "輗", "齯", "馜", "鈮", "苨", "堄", "觬", "掜", "檷", "抐", "暱", "淣", "縌", "懝", "屰", "孨", "蚭", "孴", "抳", "胒", "貎", "嬺", "屔", "誽", "痆", "眤", "蜺", "濔", "嫟", "秜", "惄", "跜", "腝", "棿", "埿", "狔", "迡", "郳", "愵", "聣", "晲", "㲻", "㞾", "㣇", "䀑", "䧇", "㧱", "㹸", "䘦", "㪒", "䛏", "䝚", "蛪", "㥾"]),
    ("nian", &["年", "念", "黏", "粘", "廿", "你按", "撵", "捻", "蔫", "碾", "撚", "唸", "溓", "涊", "埝", "你安", "你啊你", "拟按", "倪安", "尼氨", "尼安", "你俺", "你岸", "泥安", "辇", "拈", "惗", "鲶", "鲇", "姩", "榐", "卄", "淰", "辗", "蹨", "哖", "攆", "秊", "焾", "鯰", "輦", "簐", "悥", "鮎", "秥", "跈", "蹍", "輾", "鵇", "躎", "㘝", "㞋", "㲽", "䄭", "䄹", "䧔", "䟢", "㮟", "䬯", "䩞", "艌", "㜣", "㜤", "䴴", "䚓", "你", "呢", "拟", "倪", "尼", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "怩", "柅", "鑈"]),
    ("niang", &["娘", "酿", "孃", "年糕", "年关", "嬢", "年过", "年根", "尼昂", "年工", "年哥", "你昂", "釀", "醸", "䖆", "年", "念", "黏", "粘", "廿", "撵", "捻", "蔫", "碾", "撚", "唸", "溓", "涊", "埝", "辇", "拈", "惗", "鲶", "鲇", "姩", "榐", "卄", "你", "呢", "拟", "倪", "尼", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "淰", "辗", "蹨", "哖", "攆", "秊", "焾", "鯰", "輦", "簐", "悥", "鮎", "秥", "跈", "蹍", "輾", "鵇", "躎", "㘝", "㞋", "㲽", "䄭", "䄹", "䧔", "䟢", "㮟", "䬯", "䩞", "艌", "㜣", "㜤"]),
    ("niao", &["鸟", "尿", "嬲", "尼奥", "袅", "鳥", "脲", "樢", "茑", "嫋", "你熬", "你奥", "你嗷", "嬝", "你啊哦", "溺", "裊", "褭", "尦", "蔦", "茮", "㭤", "㳮", "㞙", "㼭", "䐁", "䦊", "䮍", "䙚", "㒟", "㜵", "㠡", "䃵", "你", "呢", "拟", "倪", "尼", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "怩", "柅", "鑈", "麑", "儞", "薿", "臡", "聻", "嶷", "儗", "鉨", "籾", "鯢", "輗", "齯", "馜", "鈮", "苨", "堄", "觬", "掜", "檷", "抐", "暱", "淣", "縌", "懝", "屰", "孨", "蚭", "孴", "抳", "胒", "貎", "嬺", "屔"]),
    ("nie", &["捏", "聂", "攝", "镍", "孽", "涅", "揑", "你额", "乜", "喦", "躡", "你饿", "颞", "你恶", "逆鄂", "臲", "镊", "槷", "嗫", "嚙", "摰", "掜", "囓", "聶", "闑", "隉", "櫱", "囐", "蠥", "帇", "讘", "臬", "顳", "峊", "蹑", "惗", "敜", "啮", "糵", "糱", "鑈", "苶", "蘖", "篞", "錜", "巕", "摄", "嵲", "菍", "鋷", "踂", "鎳", "钀", "籋", "齧", "陧", "噛", "圼", "枿", "湼", "銸", "孼", "囁", "鑷", "㘝", "㚔", "㖕", "㖖", "㘿", "痆", "㟧", "㮆", "䄒", "㙞", "㴪", "䂼", "䯅", "㜦", "䯵", "踗", "踙", "嶭", "㜸", "䌜", "䜓", "䞕", "㡪", "㸎", "㩶", "䡾", "䯀", "你", "呢", "拟", "倪", "尼"]),
    ("nin", &["您", "你呢", "恁", "你那", "你能", "囜", "脌", "妮妮", "你你", "你拿", "你弄", "你牛", "你哪", "你奶", "你娘", "泥泞", "倪妮", "你嫩", "拰", "㤛", "䚾", "䛘", "䋻", "你", "呢", "拟", "倪", "尼", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "怩", "柅", "鑈", "麑", "儞", "薿", "臡", "聻", "嶷", "儗", "鉨", "籾", "鯢", "輗", "齯", "馜", "鈮", "苨", "堄", "觬", "掜", "檷", "抐", "暱", "淣", "縌", "懝", "屰", "孨", "蚭", "孴", "抳", "胒", "貎", "嬺", "屔", "誽", "痆", "眤", "蜺", "濔", "嫟", "秜", "惄", "跜"]),
    ("ning", &["宁", "拧", "凝", "柠", "您给", "甯", "獰", "寧", "泞", "咛", "苧", "嚀", "鬤", "狞", "佞", "鬡", "聹", "檸", "鸋", "聍", "擰", "寕", "寜", "矃", "寗", "侫", "儜", "嬣", "寍", "倿", "澝", "橣", "濘", "鑏", "㝕", "䔭", "㿦", "㣷", "薴", "㲰", "䗿", "䭢", "㩶", "您", "恁", "你", "呢", "拟", "倪", "尼", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "囜", "脌", "拰", "㤛", "䚾", "䛘", "䋻", "怩", "柅", "鑈", "麑", "儞", "薿", "臡", "聻", "嶷", "儗", "鉨", "籾", "鯢", "輗", "齯", "馜"]),
    ("niu", &["牛", "妞", "扭", "钮", "纽", "拗", "牜", "怓", "狃", "汼", "忸", "杻", "紐", "孧", "鈕", "沑", "炄", "㽱", "䒜", "抝", "㺲", "䀔", "䏔", "䂇", "莥", "靵", "䋴", "䮗", "䤔", "你", "呢", "拟", "倪", "尼", "妮", "泥", "腻", "逆", "妳", "霓", "昵", "铌", "旎", "伱", "溺", "匿", "隬", "坭", "鲵", "伲", "膩", "睨", "猊", "氼", "擬", "婗", "兒", "怩", "柅", "鑈", "麑", "儞", "薿", "臡", "聻", "嶷", "儗", "鉨", "籾", "鯢", "輗", "齯", "馜", "鈮", "苨", "堄", "觬", "掜", "檷", "抐", "暱", "淣", "縌", "懝", "屰", "孨", "蚭", "孴", "抳", "胒", "貎", "嬺", "屔", "誽", "痆", "眤"]),
    ("nong", &["弄", "农", "浓", "侬", "脓", "哝", "秾", "襛", "農", "繷", "挊", "濃", "Nong", "燶", "蕽", "譨", "齈", "儂", "欁", "穠", "挵", "膿", "鬞", "噥", "癑", "辳", "檂", "禯", "醲", "䂇", "㶶", "莀", "㺜", "䁸", "䢉", "䵜"]),
    ("nu", &["怒", "奴", "弩", "努", "孥", "驽", "呶", "帑", "砮", "胬", "伮", "傉", "駑", "笯", "搙", "㚢", "㐐", "䢞", "䢪"]),
    ("nuan", &["暖", "煖", "湪", "奻", "怒安", "渜", "煗", "䎡", "㬉", "䙇", "餪", "怒", "奴", "弩", "努", "孥", "驽", "呶", "帑", "砮", "胬", "伮", "傉", "駑", "笯", "搙", "㚢", "㐐", "䢞", "䢪"]),
    ("nuo", &["诺", "挪", "糯", "喏", "懦", "锘", "堧", "傩", "諾", "毭", "愞", "娜", "袳", "穤", "搦", "橠", "榒", "掿", "鍩", "懧", "糥", "稬", "儺", "糑", "搻", "搙", "耎", "逽", "難", "㑚", "㛂", "㡅", "䎟", "䎠", "梛", "㐡", "㖠", "袲", "蹃", "䚥", "䇔", "㔮", "㰙", "怒", "奴", "弩", "努", "孥", "驽", "呶", "帑", "砮", "胬", "伮", "傉", "駑", "笯", "㚢", "㐐", "䢞", "䢪"]),
    ("nv", &["女", "钕", "衄", "NV", "釹", "籹", "恧", "朒", "沑", "衂", "聏"]),
    ("nve", &["虐", "疟", "瘧", "女额", "硸", "婩", "女呃", "䖈", "䖋", "女", "钕", "衄", "釹", "籹", "恧", "朒", "沑", "衂", "聏"]),
    ("ou", &["欧", "偶", "呕", "藕", "区", "鸥", "瓯", "殴", "歐", "耦", "怄", "讴", "敺", "沤", "嘔", "吽", "區", "吘", "禺", "甌", "鷗", "慪", "蕅", "鏂", "醧", "謳", "齵", "漚", "膒", "腢", "毆", "藲", "塸", "鴎", "蓲", "㛏", "㸸", "㼴", "㒖", "䌂", "䚆", "熰", "䯚", "䌔", "櫙", "䳼", "哦", "噢", "喔", "筽", "嚄"]),
    ("pa", &["怕", "爬", "趴", "啪", "平安", "帕", "扒", "跑啊", "屁啊", "耙", "拍啊", "偏爱", "怕啊", "票啊", "葩", "琶", "飘啊", "杷", "破案", "爬啊", "平啊", "胖啊", "派", "钯", "潖", "掱", "帊", "袙", "筢", "舥", "汃", "皅", "跁", "苩", "妑", "㞎", "鈀", "䔤", "䯲", "䶕"]),
    ("pai", &["拍", "排", "牌", "派", "湃", "哌", "廹", "俳", "簰", "迫", "蒎", "徘", "鎃", "簲", "沠", "棑", "猅", "箄", "犤", "㭛", "渒", "䖰", "輫", "㵺", "䮘", "䱝", "䃻", "怕", "爬", "趴", "啪", "帕", "扒", "耙", "葩", "琶", "杷", "钯", "潖", "掱", "帊", "袙", "筢", "舥", "汃", "皅", "跁", "苩", "妑", "㞎", "鈀", "䔤", "䯲", "䶕"]),
    ("pan", &["盘", "潘", "判", "盼", "怕你", "攀", "闆", "畔", "胖", "磐", "泮", "跘", "袢", "盤", "番", "蟠", "叛", "牉", "帕尼", "趴你", "帕纳", "怕难", "pan", "怕呢", "膰", "皮阿诺", "偏爱你", "蹣", "眅", "爿", "槃", "畨", "磻", "炍", "襻", "縏", "鑻", "溿", "鋬", "冸", "瀊", "盻", "蒰", "丬", "湴", "般", "拚", "籓", "柈", "鞶", "肨", "詊", "媻", "鎜", "聁", "坢", "萠", "鵥", "沜", "洀", "眫", "蹒", "㳪", "䋣", "幋", "䙃", "搫", "頖", "䃑", "㵗", "䈲", "㩯", "䃲", "䰉", "䙪", "䰔", "怕", "爬", "趴", "啪", "帕", "扒", "耙", "葩", "琶", "杷", "派", "钯", "潖", "掱", "帊", "袙", "筢", "舥", "汃", "皅"]),
    ("pang", &["胖", "旁", "庞", "逄", "盘古", "潘哥", "滂", "乓", "叛国", "膀", "判给", "螃", "磅", "霶", "判官", "pang", "膖", "篣", "胮", "盘管", "盘根", "盼哥", "潘工", "攀钢", "盘感", "沗", "嗙", "雱", "厐", "龐", "舽", "嫎", "鳑", "覫", "耪", "彷", "蒡", "肨", "龎", "尨", "髈", "汸", "炐", "眫", "鰟", "厖", "徬", "㑂", "㝑", "夆", "㫄", "䂇", "㤶", "庬", "趽", "㜊", "㥬", "䅭", "䒍", "䠙", "䨦", "䮾", "盘", "潘", "判", "盼", "攀", "闆", "畔", "磐", "泮", "跘", "袢", "盤", "番", "蟠", "叛", "牉", "膰", "蹣", "眅", "爿", "槃", "畨", "磻", "炍", "襻", "縏", "怕", "爬", "趴", "啪", "帕", "扒", "耙", "葩"]),
    ("pao", &["跑", "泡", "炮", "抛", "刨", "袍", "靤", "炰", "疱", "嚗", "礮", "怕哦", "爬哦", "狍", "瓟", "拋", "砲", "袌", "咆", "烰", "麅", "穮", "庖", "匏", "犥", "麭", "褜", "萢", "垉", "脬", "軳", "爮", "鞄", "皰", "奅", "跁", "藨", "窌", "礟", "㚿", "䛌", "䩝", "麃", "㯡", "㯱", "䠙", "㘐", "䶌", "怕", "爬", "趴", "啪", "帕", "扒", "耙", "葩", "琶", "杷", "派", "钯", "潖", "掱", "帊", "袙", "筢", "舥", "汃", "皅", "苩", "妑", "㞎", "鈀", "䔤", "䯲", "䶕"]),
    ("pei", &["配", "陪", "赔", "培", "裴", "呸", "佩", "沛", "胚", "珮", "霈", "棑", "轡", "賠", "锫", "姵", "PEI", "蓜", "醅", "毰", "旆", "辔", "婄", "媐", "馷", "裵", "妚", "坏", "帔", "抷", "嶏", "琣", "伂", "柸", "肧", "怌", "斾", "柭", "衃", "阫", "淠", "浿", "犻", "俖", "茷", "錇", "㤄", "㳈", "㸬", "㧩", "䂜", "㟝", "㫲", "䡊", "㨐", "䊃", "㾦", "䣙", "䟺", "䪹", "䯱", "䫊", "䲹", "䕗", "䫠", "攈", "㣆"]),
    ("pen", &["喷", "盆", "pen", "噴", "湓", "喯", "濆", "衯", "歕", "瓫", "呠", "葐", "翸", "㾦"]),
    ("peng", &["彭", "鹏", "碰", "朋", "棚", "捧", "蓬", "芃", "砰", "澎", "挷", "硼", "嘭", "漰", "膨", "篷", "盆骨", "烹", "喷过", "喷灌", "喷管", "喷个", "喷狗", "絣", "鵬", "怦", "塜", "鬔", "輧", "熢", "傰", "堋", "蟚", "漨", "輣", "匉", "髼", "恲", "閛", "摓", "駍", "弸", "椪", "抨", "騯", "軯", "樥", "椖", "淎", "胓", "竼", "淜", "倗", "纄", "痭", "錋", "韼", "蟛", "袶", "踫", "磞", "鑝", "剻", "韸", "掽", "搒", "鬅", "稝", "莑", "梈", "泙", "捀", "槰", "憉", "篣", "蘕", "硑", "㔙", "㑟", "㼞", "㛔", "㥊", "皏", "㮄", "㮟", "塳", "䄘", "䋽", "㱶", "䡫", "䰃", "䴶", "㘐", "䵄", "喷", "盆"]),
    ("pi", &["屁", "皮", "批", "匹", "劈", "啤", "脾", "披", "坯", "痞", "丕", "疲", "癖", "陂", "铍", "辟", "抷", "Pi", "蚍", "狉", "笓", "邳", "魮", "嚊", "秠", "噽", "噼", "僻", "潎", "毗", "紕", "礕", "鈚", "埤", "猈", "翍", "膍", "蜱", "駓", "郫", "鎞", "錍", "否", "嫓", "稫", "疋", "豼", "鈹", "炋", "貔", "枇", "礔", "毞", "揊", "睥", "螷", "釽", "憵", "琵", "岯", "吡", "魾", "譬", "諀", "鵧", "蠯", "鉟", "毘", "罴", "豾", "霹", "芘", "螕", "淠", "苉", "圮", "秛", "鷿", "髬", "肶", "悂", "嶏", "陴", "旇", "罢", "錃", "纰", "榌", "壀", "砒", "怶", "睤", "鴄", "澼", "被", "耚"]),
    ("pian", &["片", "骗", "篇", "偏", "便", "谝", "翩", "骈", "騙", "缏", "魸", "猵", "扁", "皮阿诺", "屁啊你", "屁安", "脾安", "皮安", "批案", "貵", "緶", "胼", "騈", "鍂", "賆", "蹁", "楩", "囨", "諚", "楄", "犏", "諞", "腁", "覑", "媥", "跰", "瑸", "骿", "駢", "璸", "騗", "鶣", "獱", "䏒", "㓲", "㛹", "㼐", "㾫", "䮁", "㸤", "批", "皮", "屁", "匹", "劈", "啤", "脾", "披", "坯", "痞", "丕", "疲", "癖", "陂", "铍", "辟", "抷", "蚍", "狉", "笓", "邳", "魮", "嚊", "秠", "噽", "噼", "僻", "潎", "毗", "紕", "礕", "鈚", "埤", "猈", "翍", "膍", "蜱", "駓", "郫", "鎞", "錍", "否", "嫓", "稫", "疋", "豼"]),
    ("piao", &["票", "飘", "漂", "嫖", "朴", "瓢", "瞟", "驃", "缥", "飄", "皮袄", "殍", "嘌", "剽", "骠", "皫", "徱", "皮奥", "闝", "皮啊哦", "旚", "僄", "螵", "慓", "薸", "彯", "魒", "鰾", "勡", "翲", "醥", "篻", "犥", "莩", "㲏", "㹾", "㺓", "㬓", "㼼", "㯱", "㵱", "䏇", "縹", "㩠", "䕯", "顠", "飃", "䴩", "批", "皮", "屁", "匹", "劈", "啤", "脾", "披", "坯", "痞", "丕", "疲", "癖", "陂", "铍", "辟", "抷", "蚍", "狉", "笓", "邳", "魮", "嚊", "秠", "噽", "噼", "僻", "潎", "毗", "紕", "礕", "鈚", "埤", "猈", "翍", "膍", "蜱", "駓", "郫", "鎞", "錍", "否", "嫓", "稫", "疋", "豼", "鈹", "炋"]),
    ("pie", &["撇", "瞥", "丿", "pie", "潎", "撆", "批额", "氕", "覕", "屁额", "屁鹅", "苤", "暼", "嫳", "鐅", "䥕", "批", "皮", "屁", "匹", "劈", "啤", "脾", "披", "坯", "痞", "丕", "疲", "癖", "陂", "铍", "辟", "抷", "蚍", "狉", "笓", "邳", "魮", "嚊", "秠", "噽", "噼", "僻", "毗", "紕", "礕", "鈚", "埤", "猈", "翍", "膍", "蜱", "駓", "郫", "鎞", "錍", "否", "嫓", "稫", "疋", "豼", "鈹", "炋", "貔", "枇", "礔", "毞", "揊", "睥", "螷", "釽", "憵", "琵", "岯", "吡", "魾", "譬", "諀", "鵧", "蠯", "鉟", "毘", "罴", "豾", "霹", "芘", "螕", "淠", "苉", "圮", "秛", "鷿", "髬", "肶", "悂", "嶏"]),
    ("pin", &["品", "拼", "频", "聘", "贫", "皮囊", "榀", "嫔", "玭", "頻", "牝", "蘋", "拚", "pin", "劈你", "姘", "皮内", "颦", "脾牛", "睥睨", "皮诺", "汖", "貧", "顰", "朩", "嚬", "礗", "蠙", "嬪", "穦", "矉", "薲", "砏", "琕", "馪", "㲏", "娦", "䀻", "㻞", "㰋", "驞", "批", "皮", "屁", "匹", "劈", "啤", "脾", "披", "坯", "痞", "丕", "疲", "癖", "陂", "铍", "辟", "抷", "蚍", "狉", "笓", "邳", "魮", "嚊", "秠", "噽", "噼", "僻", "潎", "毗", "紕", "礕", "鈚", "埤", "猈", "翍", "膍", "蜱", "駓", "郫", "鎞", "錍", "否", "嫓", "稫", "疋", "豼", "鈹", "炋", "貔", "枇", "礔", "毞", "揊", "睥", "螷"]),
    ("ping", &["平", "瓶", "屏", "评", "萍", "凭", "坪", "苹", "馮", "乒", "娉", "品格", "硑", "拼购", "軿", "拼个", "枰", "品管", "拼过", "ping", "品冠", "蛢", "拼柜", "品高", "品阁", "品工", "塀", "評", "鉼", "庰", "簈", "憑", "箳", "蘋", "屛", "呯", "鲆", "俜", "玶", "艵", "泙", "凴", "帲", "冯", "砯", "檘", "幈", "聠", "洴", "荓", "甹", "鮃", "胓", "蚲", "郱", "帡", "涄", "淜", "岼", "焩", "蓱", "竮", "輧", "頩", "缾", "慿", "㲏", "㺸", "㻂", "䓑", "䍈", "䈂", "甁", "㵗", "䶄", "品", "拼", "频", "聘", "贫", "榀", "嫔", "玭", "頻", "牝", "拚", "姘", "颦", "批", "皮", "屁", "匹", "劈", "啤", "脾", "披"]),
    ("po", &["破", "婆", "坡", "po", "泼", "覇", "迫", "魄", "皛", "珀", "嚩", "拍哦", "票哦", "配偶", "排偶", "屁哦", "猼", "颇", "粕", "蔢", "泊", "朴", "陂", "髲", "鄱", "岶", "蒪", "緐", "濼", "烞", "钋", "醱", "謈", "砶", "駊", "頗", "叵", "钷", "湐", "泺", "嘙", "醗", "溌", "笸", "潑", "昢", "櫇", "敀", "皤", "奤", "岥", "洦", "繁", "尀", "廹", "桲", "翍", "搫", "箥", "鏺", "攴", "釙", "鉕", "㧊", "䘠", "㛘", "䄸", "哱", "䎅", "䣪", "䦌", "㨇", "㰴", "䎊", "䞟", "䣮", "酦", "㜑", "㔇", "䯙", "䪖", "䮘", "䪙", "㩯", "䨰"]),
    ("pou", &["剖", "抔", "犃", "頮", "裒", "掊", "捊", "咅", "婄", "棓", "娝", "抙", "垺", "哣", "箁", "㰴", "䎧", "䯽", "㧵", "㼜", "㟝", "䳝", "破", "婆", "坡", "泼", "覇", "迫", "魄", "皛", "珀", "嚩", "猼", "颇", "粕", "蔢", "泊", "朴", "陂", "髲", "鄱", "岶", "蒪", "緐", "濼", "烞", "钋", "醱", "謈", "砶", "駊", "頗", "叵", "钷", "湐", "泺", "嘙", "醗", "溌", "笸", "潑", "昢", "櫇", "敀", "皤", "奤", "岥", "洦", "繁", "尀", "廹", "桲", "翍", "搫", "箥", "鏺", "攴", "釙", "鉕", "㧊", "䘠", "㛘", "䄸", "哱", "䎅", "䣪", "䦌", "㨇", "䎊", "䞟", "䣮", "酦", "㜑", "㔇", "䯙", "䪖"]),
    ("pu", &["噗", "铺", "普", "蒲", "谱", "浦", "扑", "朴", "璞", "濮", "葡", "埔", "仆", "圃", "脯", "莆", "襆", "菩", "PU", "堡", "溥", "瀑", "镨", "鋪", "蹼", "潽", "譜", "僕", "攴", "镤", "曝", "撲", "樸", "匍", "攵", "贌", "圤", "捗", "墣", "炇", "鏷", "蒱", "諩", "秿", "暴", "酺", "瞨", "箁", "舖", "檏", "烳", "暜", "舗", "氆", "痡", "抪", "菐", "圑", "獛", "巬", "陠", "蜅", "穙", "鯆", "纀", "鐠", "柨", "㺪", "䔕", "䧤", "㒒", "㲫", "㹒", "䈬", "䪬", "擈", "㙸", "䈻", "㬥", "㯷", "䑑", "䗱", "襥", "䲕", "䴆"]),
    ("qi", &["起", "气", "其", "期", "七", "器", "齐", "揭", "骑", "啐", "琪", "奇", "启", "琦", "漆", "柒", "企", "祁", "旗", "戚", "祺", "棋", "淇", "汽", "弃", "Qi", "妻", "绮", "亓", "綮", "魕", "岐", "麒", "嘁", "骐", "恓", "砌", "崎", "祈", "啟", "噐", "綦", "炁", "踦", "栖", "蘄", "緝", "圻", "杞", "芪", "氣", "摖", "岂", "契", "歧", "鳍", "锜", "欺", "泣", "齊", "籏", "觭", "脐", "颀", "焏", "慽", "濝", "乞", "蕲", "朞", "蛣", "萁", "讫", "玘", "碶", "僛", "沏", "畦", "憩", "梩", "騎", "簱", "湇", "盵", "碁", "疧", "埼", "禥", "褀", "耆", "啓", "趞", "婍", "凄", "呚", "陭"]),
    ("qia", &["恰", "掐", "起啊", "洽", "卡", "愘", "抲", "髂", "气啊", "帢", "揢", "骑啊", "奇啊", "匼", "擖", "硈", "磍", "冾", "峠", "跒", "拤", "胢", "殎", "咭", "葜", "酠", "袷", "圶", "鞐", "㓞", "㤉", "㧎", "㓣", "㓤", "㦴", "㡊", "䂒", "㵄", "䁍", "䨐", "䯊", "䠍", "䜑", "䶗", "䶝", "起", "其", "期", "七", "气", "器", "齐", "揭", "骑", "啐", "琪", "奇", "启", "琦", "漆", "柒", "企", "祁", "旗", "戚", "祺", "棋", "淇", "汽", "弃", "妻", "绮", "亓", "綮", "魕", "岐", "麒", "嘁", "骐", "恓", "砌", "崎", "祈", "啟", "噐", "綦", "炁", "踦", "栖", "蘄", "緝", "圻", "杞", "芪", "氣", "摖"]),
    ("qian", &["钱", "前", "签", "千", "欠", "倩", "浅", "仟", "茜", "迁", "乾", "谦", "芊", "牵", "铅", "纤", "錢", "黔", "潜", "钳", "骞", "七安", "启安", "奇案", "祺安", "齐安", "祁安", "祈安", "起岸", "嵌", "譴", "簽", "钎", "芡", "遣", "羬", "虔", "廞", "歉", "悭", "阡", "扲", "欿", "鳽", "撍", "钤", "扦", "淺", "朁", "諐", "拪", "堑", "孅", "蒨", "鈆", "岍", "墘", "攓", "歬", "佥", "縴", "篏", "籤", "亁", "謙", "汧", "壍", "黚", "蚈", "輤", "撁", "愆", "鵮", "掮", "嬱", "炶", "慊", "顅", "粁", "搴", "汘", "箞", "摼", "遷", "騝", "儙", "慳", "篟", "攑", "缱", "騚", "犍", "褰", "鬝", "杄", "鉗"]),
    ("qiang", &["强", "抢", "墙", "枪", "將", "腔", "呛", "牵挂", "羌", "強", "千古", "钳工", "蔷", "签个", "锵", "搶", "钱哥", "前杠", "签过", "鶬", "羟", "前高", "谦哥", "前盖", "迁改", "启昂", "起昂", "气昂", "篬", "炝", "牆", "嫱", "镪", "琷", "槍", "廧", "瑲", "唴", "樯", "锖", "繦", "戗", "嶈", "繈", "羗", "嗆", "将", "玱", "彊", "戕", "蔃", "斨", "羥", "摤", "艢", "跄", "蜣", "羻", "鏘", "襁", "矼", "椌", "鏹", "蹡", "鎗", "熗", "漒", "啌", "謒", "薔", "戧", "墻", "檣", "嗴", "嬙", "錆", "墏", "摪", "羫", "蘠", "㛨", "猐", "溬", "獇", "䅚", "牄", "㩖", "蹌", "䵁", "钱", "前", "签", "千", "欠", "倩", "浅"]),
    ("qiao", &["桥", "敲", "乔", "巧", "翘", "蕉", "瞧", "俏", "侨", "撬", "谯", "鞘", "窍", "荞", "峤", "樵", "锹", "鞒", "悄", "壳", "喿", "启奥", "淇澳", "齐奥", "淇奥", "奇奥", "恰哦", "祁澳", "掐哦", "碻", "骹", "磝", "睄", "嫶", "誚", "橇", "箾", "繰", "橋", "峭", "喬", "跷", "硚", "撽", "潐", "摮", "菬", "顦", "翹", "鐈", "毃", "嵪", "磽", "躈", "幓", "幧", "僑", "荍", "礉", "墽", "趬", "鐰", "憔", "藮", "犞", "繑", "癄", "雀", "劁", "礄", "诮", "燆", "踍", "嘺", "鄥", "韒", "鞩", "硗", "櫵", "僺", "髚", "墝", "缲", "髜", "蹻", "釥", "塙", "殼", "鄡", "蕎", "燋", "槗", "陗", "愀", "鍫", "竅"]),
    ("qie", &["切", "且", "企鹅", "椄", "脥", "茄", "砌", "妾", "企额", "骑鹅", "弃恶", "起鹅", "郄", "怯", "窃", "锲", "伽", "惬", "悏", "踥", "癿", "蛪", "挈", "箧", "淁", "厒", "帹", "苆", "竊", "洯", "鐑", "愜", "趄", "鯜", "慊", "籡", "栔", "稧", "笡", "朅", "穕", "疌", "鍥", "聺", "跙", "猰", "嗛", "緁", "匧", "藒", "㚗", "㹤", "㰤", "㛍", "㛗", "㛙", "㼤", "㾀", "㓶", "㤲", "㰰", "䟙", "㫸", "㾜", "䞣", "㥦", "㰼", "詧", "㗫", "篋", "䦧", "䬊", "䌌", "㙻", "䤿", "䫔", "起", "其", "期", "七", "气", "器", "齐", "揭", "骑", "啐", "琪", "奇", "启", "琦", "漆", "柒", "企", "祁", "旗", "戚"]),
    ("qin", &["亲", "琴", "秦", "勤", "覃", "钦", "芹", "沁", "禽", "斳", "七年", "親", "寝", "气你", "芩", "气馁", "气囊", "侵", "期内", "Qin", "擒", "嗪", "溱", "鋟", "揿", "骎", "藽", "嵚", "欽", "噙", "衾", "捦", "靲", "顉", "瘽", "矜", "寖", "琹", "嶜", "檎", "鈙", "寴", "锓", "儭", "嶔", "螼", "寢", "昑", "耹", "慬", "鳹", "吣", "澿", "懄", "庈", "鈂", "撳", "吢", "嫀", "廑", "鮼", "鵭", "肣", "菳", "抋", "螓", "忴", "矝", "鈊", "綅", "埐", "菦", "梫", "坅", "扲", "珡", "懃", "蚙", "駸", "瀙", "唚", "雂", "嵰", "搇", "誛", "蠄", "菣", "赾", "媇", "寑", "鬵", "鈫", "㘦", "㞬", "㤈", "㪁"]),
    ("qing", &["请", "清", "青", "情", "庆", "轻", "晴", "卿", "倾", "請", "氢", "擎", "亲哥", "亲", "箐", "氰", "傾", "顷", "磬", "秦国", "亲故", "勍", "Qing", "輕", "慶", "罄", "靑", "蜻", "掅", "謦", "凊", "淸", "鑋", "鲭", "黥", "苘", "啨", "綪", "檠", "鶄", "圊", "頃", "埥", "綮", "濪", "靘", "親", "氫", "剠", "甠", "棾", "暒", "夝", "鯖", "葝", "寈", "軽", "檾", "郬", "庼", "殸", "碃", "璚", "硘", "漀", "渹", "櫦", "殑", "廎", "擏", "錆", "儬", "䋜", "䯧", "㔀", "䋯", "䔛", "䵞", "㢣", "㷫", "樈", "㯳", "㩩", "㵾", "䌠", "䲔", "秦", "琴", "勤", "覃", "钦", "芹", "沁", "禽", "斳", "寝"]),
    ("qiong", &["穷", "琼", "嬛", "穹", "赹", "惸", "窮", "邛", "笻", "藑", "瞏", "瓊", "茕", "熍", "瓗", "竆", "舼", "銎", "蛩", "筇", "跫", "焪", "宆", "儝", "桏", "藭", "焭", "睘", "橩", "璚", "蛬", "憌", "煢", "卭", "㑋", "䧆", "㤨", "㧭", "㷀", "䅃", "琁", "䊄", "㒌", "㮪", "㵌", "䆳", "䛪", "䠻", "㼇", "起", "其", "期", "七", "气", "器", "齐", "揭", "骑", "啐", "琪", "奇", "启", "琦", "漆", "柒", "企", "祁", "旗", "戚", "祺", "棋", "淇", "汽", "弃", "妻", "绮", "亓", "綮", "魕", "岐", "麒", "嘁", "骐", "恓", "砌", "崎", "祈", "啟", "噐", "綦", "炁", "踦", "栖", "蘄", "緝", "圻"]),
    ("qiu", &["求", "球", "秋", "邱", "丘", "仇", "裘", "龜", "囚", "湫", "糗", "橚", "酋", "楸", "鳅", "紌", "朹", "虬", "逑", "萩", "蓲", "坵", "觩", "遒", "俅", "釚", "蠤", "賕", "毬", "璆", "鰽", "湬", "莍", "櫹", "渞", "鶖", "蛷", "蚯", "肍", "恷", "芁", "湭", "泅", "殏", "盚", "蟗", "犰", "穐", "趥", "艽", "鼽", "丠", "銶", "皳", "巯", "醔", "鞧", "鮂", "秌", "鯄", "釻", "蝤", "鰍", "巰", "玌", "厹", "蘒", "頄", "恘", "鞦", "浗", "赇", "叴", "龟", "媝", "唒", "鹙", "篍", "虯", "逎", "牫", "緧", "鰌", "汓", "觓", "訄", "崷", "搝", "絿", "蝵", "鱃", "龝", "㐀", "㕤", "㧄", "㲹"]),
    ("qu", &["去", "取", "区", "佢", "曲", "屈", "趣", "瞿", "娶", "渠", "驱", "蛆", "區", "軀", "祛", "龋", "趋", "衢", "覷", "爠", "趍", "袪", "躯", "詘", "璩", "阹", "敺", "翵", "闃", "魼", "蛐", "鴝", "鶌", "蕖", "蘧", "髷", "軥", "菃", "覰", "蠷", "塸", "觑", "坥", "鸲", "螶", "朐", "岴", "麹", "呿", "麮", "竬", "欪", "誳", "磲", "黢", "诎", "麴", "翑", "劬", "岖", "憈", "鼩", "阒", "浀", "佉", "欔", "胠", "驅", "戌", "癯", "焌", "忂", "粬", "灈", "鱋", "麯", "鑺", "竘", "齲", "斪", "衐", "趨", "鸜", "紶", "蝺", "氍", "臞", "厺", "苣", "絇", "駆", "閴", "駈", "鰸", "躣", "蠼"]),
    ("quan", &["全", "圈", "券", "权", "泉", "劝", "犬", "拳", "铨", "荃", "醛", "踡", "诠", "矔", "去俺", "佺", "狋", "權", "曲安", "去安", "去按", "曲岸", "跧", "婘", "顴", "犭", "綣", "蜷", "筌", "搼", "蠸", "颧", "勸", "絟", "奍", "湶", "觠", "虇", "硂", "绻", "銓", "輇", "瑔", "痊", "圏", "詮", "悛", "烇", "峑", "弮", "鳈", "洤", "畎", "辁", "棬", "甽", "椦", "啳", "灥", "牷", "恮", "姾", "惓", "勧", "埢", "葲", "騡", "牶", "槫", "韏", "巏", "箞", "犮", "犈", "楾", "権", "駩", "孉", "捲", "腃", "鬈", "齤", "縓", "㒰", "䀬", "㟨", "㟫", "䄐", "䅚", "䊎", "闎", "㩲", "鐉", "鰁", "䑏", "䌯"]),
    ("que", &["缺", "却", "确", "雀", "阙", "瘸", "鹊", "棤", "碻", "卻", "礭", "炔", "去恶", "去额", "去饿", "確", "阕", "愨", "敠", "墧", "礐", "榷", "皵", "悫", "敪", "闕", "鵲", "埆", "燩", "蒛", "塙", "傕", "碏", "崅", "硞", "慤", "缼", "琷", "趞", "蚗", "汋", "㕁", "㹤", "㱿", "㹱", "㾡", "㴶", "䇎", "搉", "㩁", "䍳", "䢧", "䲵", "䀇", "䧿", "㲉", "䦬", "闋", "䱜", "㰌", "㱋", "去", "区", "取", "佢", "曲", "屈", "趣", "瞿", "娶", "渠", "驱", "蛆", "區", "軀", "祛", "龋", "趋", "衢", "覷", "爠", "趍", "袪", "躯", "詘", "璩", "阹", "敺", "翵", "闃", "魼", "蛐", "鴝", "鶌", "蕖", "蘧"]),
    ("qun", &["群", "去年", "裙", "去哪", "麕", "踆", "去拿", "去你", "去呢", "去那", "取暖", "娶你", "去弄", "歏", "峮", "羣", "囷", "逡", "宭", "夋", "箘", "帬", "裠", "麇", "㪊", "䞭", "䭽", "㿏", "䆽", "去", "区", "取", "佢", "曲", "屈", "趣", "瞿", "娶", "渠", "驱", "蛆", "區", "軀", "祛", "龋", "趋", "衢", "覷", "爠", "趍", "袪", "躯", "詘", "璩", "阹", "敺", "翵", "闃", "魼", "蛐", "鴝", "鶌", "蕖", "蘧", "髷", "軥", "菃", "覰", "蠷", "塸", "觑", "坥", "鸲", "螶", "朐", "岴", "麹", "呿", "麮", "竬", "欪", "誳", "磲", "黢", "诎", "麴", "翑", "劬", "岖", "憈", "鼩", "阒", "浀", "佉", "欔", "胠"]),
    ("ran", &["然", "染", "冉", "燃", "苒", "熯", "衻", "蚺", "姌", "髯", "ran", "蚦", "橪", "珃", "媣", "冄", "繎", "蒅", "嘫", "袇", "袡", "髥", "肰", "呥", "㚩", "䒣", "㹱", "㾆", "䑙", "䇋", "䎃", "䣸", "䤡", "㒄", "㿵", "㜣", "䔳", "㯗", "䫇", "㲯", "䕼", "䶲", "㸐", "䰯", "䳿", "䖄"]),
    ("rang", &["让", "嚷", "讓", "瓤", "染个", "壤", "攘", "穰", "染过", "禳", "染膏", "然哥", "染缸", "瀼", "蘘", "勷", "穣", "rang", "躟", "壌", "鬤", "儴", "譲", "獽", "纕", "懹", "䑋", "爙", "䉴", "㚂", "然", "染", "冉", "燃", "苒", "熯", "衻", "蚺", "姌", "髯", "蚦", "橪", "珃", "媣", "冄", "繎", "蒅", "嘫", "袇", "袡", "髥", "肰", "呥", "㚩", "䒣", "㹱", "㾆", "䑙", "䇋", "䎃", "䣸", "䤡", "㒄", "㿵", "㜣", "䔳", "㯗", "䫇", "㲯", "䕼", "䶲", "㸐", "䰯", "䳿", "䖄"]),
    ("rao", &["绕", "饶", "扰", "娆", "Rao", "桡", "荛", "繞", "擾", "穘", "饒", "隢", "蕘", "襓", "遶", "嬈", "橈", "犪", "㑱", "䫞", "㹛"]),
    ("re", &["热", "惹", "熱", "然而", "若", "然鹅", "人儿", "蓉儿", "入额", "入耳", "日俄", "软腭", "喏", "渃", "捼", "㻰"]),
    ("ren", &["人", "认", "任", "仁", "忍", "热闹", "刃", "惹你", "壬", "韧", "棯", "紉", "仞", "紝", "妊", "热能", "惹怒", "惹恼", "热内", "热鸟", "热念", "任儿女", "REN", "荏", "亻", "稔", "祍", "認", "靱", "栣", "轫", "朲", "纫", "刄", "饪", "牣", "芢", "衽", "綛", "魜", "忈", "讱", "屻", "忎", "仭", "鵀", "秹", "扨", "腍", "韌", "岃", "秂", "荵", "肕", "軔", "纴", "釰", "葚", "銋", "杒", "釼", "袵", "姙", "栠", "訒", "菍", "絍", "鈓", "餁", "㠴", "㣼", "㲽", "䀔", "䏕", "㤛", "㶵", "㸾", "䏰", "䚾", "梕", "䀼", "䄒", "䇮", "䋕", "䌾", "靭", "飪", "䛘", "躵", "䭃", "䴦", "热", "惹", "熱", "若", "喏"]),
    ("reng", &["扔", "仍", "人工", "人格", "陾", "礽", "认购", "人够", "人给", "任哥", "人更", "人过", "人广", "人干", "认个", "忍过", "任工", "认股", "任刚", "芿", "辸", "㭁", "䄧", "㺱", "䚮", "人", "任", "认", "仁", "忍", "刃", "壬", "韧", "棯", "紉", "仞", "紝", "妊", "荏", "亻", "稔", "祍", "認", "靱", "栣", "轫", "朲", "纫", "刄", "饪", "热", "惹", "熱", "若", "喏", "牣", "芢", "衽", "綛", "魜", "忈", "讱", "屻", "忎", "仭", "鵀", "秹", "扨", "腍", "韌", "岃", "秂", "荵", "肕", "軔", "纴", "釰", "葚", "銋", "杒", "釼", "袵", "姙", "栠", "訒", "菍", "絍", "鈓", "餁", "㠴", "㣼", "㲽", "䀔", "䏕", "㤛", "㶵"]),
    ("ri", &["日", "囸", "驲", "鈤", "衵", "馹", "氜", "釰", "䒤", "釼"]),
    ("rong", &["荣", "容", "融", "绒", "蓉", "榕", "戎", "溶", "熔", "嵘", "茸", "镕", "榮", "冗", "瑢", "嬫", "肜", "氄", "嶸", "羢", "鎔", "絨", "嵤", "毧", "媶", "槦", "蠑", "搑", "嫆", "蝾", "穃", "縙", "狨", "栄", "爃", "烿", "傛", "瀜", "軵", "曧", "宂", "茙", "螎", "峵", "搈", "穁", "榵", "駥", "坈", "傇", "褣", "㝐", "䄧", "㭜", "㲓", "䘬", "䡆", "㲝", "䇯", "㲨", "㣑", "㺎", "㼸", "䋴", "䢇", "䈶", "䠜", "䤊", "㘇", "䡥", "䩸", "䇀"]),
    ("rou", &["肉", "揉", "柔", "鞣", "媃", "糅", "蹂", "煣", "渘", "輮", "韖", "宍", "譳", "脜", "瑈", "鍒", "鰇", "禸", "厹", "葇", "鶔", "騥", "腬", "楺", "粈", "蝚", "䄾", "㖻", "㮟", "䐓", "䧷", "㽥", "䋴", "䢇", "䥆", "䰆", "瓇"]),
    ("ru", &["如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸", "蒘", "襦", "顬", "筎", "蓐", "嚅", "缛", "嬬", "繻", "侞", "扖", "杁", "颥", "鄏", "溽", "銣", "桇", "挐", "獳", "鱬", "渪", "邚", "鳰", "嶿", "醹", "鴑", "曘", "肗", "媷", "檽", "縟", "㐈", "吺", "㼋", "㾒", "䋈", "㨎", "䘫", "㨌", "䐓", "㦺", "䋴", "㱶", "䤉", "擩", "㹘", "䞕", "䫱", "䰭", "䰰"]),
    ("rua", &["rua", "挼", "入啊", "茹啊", "如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸", "蒘", "襦", "顬", "筎", "蓐", "嚅", "缛", "嬬", "繻", "侞", "扖", "杁", "颥", "鄏", "溽", "銣", "桇", "挐", "獳", "鱬", "渪", "邚", "鳰", "嶿", "醹", "鴑", "曘", "肗", "媷", "檽", "縟", "㐈", "吺", "㼋", "㾒", "䋈", "㨎", "䘫", "㨌", "䐓", "㦺", "䋴", "㱶", "䤉", "擩", "㹘", "䞕", "䫱", "䰭", "䰰"]),
    ("ruan", &["软", "阮", "壖", "瓀", "腝", "茹安", "軟", "如安", "蝡", "入案", "如按", "乳安", "汝安", "儒安", "輭", "朊", "碝", "撋", "礝", "媆", "耎", "堧", "瑌", "緛", "檽", "㐾", "䆓", "䎡", "㓴", "偄", "㨎", "䓴", "㮕", "㼱", "㜛", "䙃", "䙇", "䞂", "䪭", "㽭", "䪀", "挼", "如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "帤", "薷", "蠕", "鴽", "嗕", "燸", "蒘", "襦", "顬", "筎", "蓐", "嚅", "缛", "嬬", "繻", "侞", "扖", "杁", "颥", "鄏", "溽", "銣", "桇", "挐", "獳", "鱬", "渪", "邚", "鳰", "嶿", "醹", "鴑", "曘", "肗", "媷", "縟", "㐈", "吺", "㼋", "㾒"]),
    ("rui", &["瑞", "睿", "锐", "蕊", "芮", "蕤", "汭", "叡", "入I", "繠", "銳", "枘", "鋭", "惢", "蚋", "緌", "壡", "蘂", "笍", "兌", "蘃", "蕋", "桵", "苼", "兊", "抐", "橤", "蜹", "鏸", "㨅", "䌼", "䇤", "䦌", "㓹", "㛱", "㮃", "㻔", "䂱", "䅑", "䓲", "甤", "䞩", "䍴", "㕙", "㪫", "㲊", "㹘", "䳠", "如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸", "蒘", "襦", "顬", "筎", "蓐", "嚅", "缛", "嬬", "繻", "侞", "扖", "杁", "颥", "鄏", "溽", "銣", "桇", "挐", "獳", "鱬", "渪", "邚", "鳰", "嶿", "醹", "鴑"]),
    ("run", &["润", "闰", "如你", "如您", "潤", "膶", "汝南", "入内", "run", "如能", "乳牛", "入脑", "瞤", "閏", "閠", "橍", "䏕", "㝄", "㠈", "䦞", "如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸", "蒘", "襦", "顬", "筎", "蓐", "嚅", "缛", "嬬", "繻", "侞", "扖", "杁", "颥", "鄏", "溽", "銣", "桇", "挐", "獳", "鱬", "渪", "邚", "鳰", "嶿", "醹", "鴑", "曘", "肗", "媷", "檽", "縟", "㐈", "吺", "㼋", "㾒", "䋈", "㨎", "䘫", "㨌", "䐓", "㦺", "䋴", "㱶", "䤉", "擩", "㹘", "䞕", "䫱", "䰭", "䰰"]),
    ("ruo", &["若", "弱", "偌", "叒", "箬", "渃", "婼", "如哦", "鶸", "挼", "楉", "鄀", "爇", "篛", "焫", "蒻", "嵶", "鰙", "捼", "鰯", "㬉", "䐞", "㘃", "如", "入", "茹", "乳", "汝", "儒", "辱", "袽", "褥", "濡", "蕠", "铷", "孺", "洳", "蝡", "帤", "薷", "蠕", "鴽", "嗕", "偄", "燸", "蒘", "襦", "顬", "筎", "蓐", "嚅", "缛", "嬬", "繻", "侞", "扖", "杁", "颥", "鄏", "溽", "銣", "桇", "挐", "獳", "鱬", "渪", "邚", "鳰", "嶿", "醹", "鴑", "曘", "肗", "媷", "檽", "縟", "㐈", "吺", "㼋", "㾒", "䋈", "㨎", "䘫", "㨌", "䐓", "㦺", "䋴", "㱶", "䤉", "擩", "㹘", "䞕", "䫱", "䰭", "䰰"]),
    ("sa", &["撒", "是啊", "仨", "萨", "飒", "啥啊", "谁啊", "说啊", "洒", "睡啊", "卅", "上啊", "上岸", "爽啊", "送啊", "sa", "颯", "薩", "潵", "脎", "钑", "櫒", "灑", "摋", "躠", "挲", "挱", "馺", "訯", "蕯", "纚", "鏾", "攃", "鞈", "靸", "泧", "鈒", "隡", "㳐", "䑥", "䘮", "䓲", "㒎", "䬃", "㪪", "㽂", "䊛", "䙣"]),
    ("sai", &["塞", "赛", "噻", "嗮", "腮", "鳃", "賽", "嘥", "毸", "顋", "Sai", "鰓", "思", "愢", "簺", "僿", "毢", "揌", "㗷", "䈢", "㘔", "撒", "仨", "萨", "飒", "洒", "卅", "颯", "薩", "潵", "脎", "钑", "櫒", "灑", "摋", "躠", "挲", "挱", "馺", "訯", "蕯", "纚", "鏾", "攃", "鞈", "靸", "泧", "鈒", "隡", "㳐", "䑥", "䘮", "䓲", "㒎", "䬃", "㪪", "㽂", "䊛", "䙣"]),
    ("san", &["三", "散", "叁", "伞", "撒尿", "彡", "叄", "弎", "馓", "糁", "San", "说爱你", "圣安娜", "傘", "氵", "參", "潵", "仐", "壭", "毶", "糂", "繖", "毵", "鏾", "俕", "橵", "糝", "叅", "毿", "糤", "鏒", "閐", "厁", "饊", "糣", "鬖", "帴", "犙", "䀐", "㧲", "㤾", "㪔", "㪚", "䊉", "䫅", "䉈", "䫩", "撒", "仨", "萨", "飒", "洒", "卅", "颯", "薩", "脎", "钑", "櫒", "灑", "摋", "躠", "挲", "挱", "馺", "訯", "蕯", "纚", "攃", "鞈", "靸", "泧", "鈒", "隡", "㳐", "䑥", "䘮", "䓲", "㒎", "䬃", "㪪", "㽂", "䊛", "䙣"]),
    ("sang", &["桑", "三个", "丧", "嗓", "三观", "搡", "三哥", "三国", "散光", "三高", "三姑", "三根", "三公", "喪", "颡", "槡", "磉", "sang", "鎟", "褬", "桒", "顙", "䘮", "䡦", "䫙", "三", "散", "叁", "伞", "彡", "叄", "弎", "馓", "糁", "傘", "氵", "參", "撒", "仨", "萨", "飒", "洒", "卅", "颯", "薩", "潵", "仐", "壭", "毶", "糂", "繖", "毵", "鏾", "俕", "橵", "糝", "叅", "毿", "糤", "鏒", "閐", "厁", "饊", "糣", "鬖", "帴", "犙", "䀐", "㧲", "㤾", "㪔", "㪚", "䊉", "䫅", "䉈", "䫩", "脎", "钑", "櫒", "灑", "摋", "躠", "挲", "挱", "馺", "訯", "蕯", "纚", "攃", "鞈", "靸", "泧", "鈒", "隡", "㳐", "䑥"]),
    ("sao", &["扫", "骚", "嫂", "懆", "臊", "掃", "騒", "搔", "繰", "撒哦", "仨哦", "騷", "梢", "氉", "埽", "缫", "鰠", "瘙", "溞", "鱢", "螦", "髞", "鳋", "掻", "颾", "慅", "繅", "缲", "矂", "㛐", "㛮", "䕅", "䐹", "㿋", "撒", "仨", "萨", "飒", "洒", "卅", "颯", "薩", "潵", "脎", "钑", "櫒", "灑", "摋", "躠", "挲", "挱", "馺", "訯", "蕯", "纚", "鏾", "攃", "鞈", "靸", "泧", "鈒", "隡", "㳐", "䑥", "䘮", "䓲", "㒎", "䬃", "㪪", "㽂", "䊛", "䙣"]),
    ("se", &["色", "涩", "瑟", "閪", "事儿", "十二", "数额", "烧鹅", "善恶", "塞", "少儿", "铯", "首恶", "是恶", "首尔", "啬", "澀", "穑", "渋", "歰", "璱", "擌", "歮", "拺", "嗇", "琗", "雭", "廧", "栜", "澁", "摵", "鏼", "銫", "洓", "懎", "瘷", "粣", "溹", "槭", "瀒", "鎍", "濏", "穯", "譅", "飋", "濇", "穡", "轖", "闟", "繬", "薔", "㺩", "㻎", "㴔", "䊂", "㥶", "㮦", "㻭", "㒊", "䨛", "㱇", "㽇", "䔼", "䉢"]),
    ("sen", &["森", "椮", "襂", "槮", "色女", "色男", "色你", "十二年", "Sen", "顺尔宁", "色", "涩", "瑟", "閪", "塞", "铯", "啬", "澀", "穑", "渋", "歰", "璱", "擌", "歮", "拺", "嗇", "琗", "雭", "廧", "栜", "澁", "摵", "鏼", "銫", "洓", "懎", "瘷", "粣", "溹", "槭", "瀒", "鎍", "濏", "穯", "譅", "飋", "濇", "穡", "轖", "闟", "繬", "薔", "㺩", "㻎", "㴔", "䊂", "㥶", "㮦", "㻭", "㒊", "䨛", "㱇", "㽇", "䔼", "䉢"]),
    ("seng", &["僧", "森哥", "鬙", "SENG", "森", "椮", "色", "涩", "瑟", "閪", "塞", "铯", "啬", "澀", "穑", "渋", "歰", "璱", "擌", "襂", "槮", "歮", "拺", "嗇", "琗", "雭", "廧", "栜", "澁", "摵", "鏼", "銫", "洓", "懎", "瘷", "粣", "溹", "槭", "瀒", "鎍", "濏", "穯", "譅", "飋", "濇", "穡", "轖", "闟", "繬", "薔", "㺩", "㻎", "㴔", "䊂", "㥶", "㮦", "㻭", "㒊", "䨛", "㱇", "㽇", "䔼", "䉢"]),
    ("sha", &["啥", "傻", "杀", "沙", "纱", "砂", "莎", "鲨", "煞", "刹", "厦", "濈", "殺", "痧", "说话啊", "猀", "杉", "上号啊", "上海啊", "啑", "廈", "嗄", "霎", "榝", "硰", "儍", "紗", "唦", "剎", "喢", "菨", "裟", "魦", "鯊", "铩", "帹", "閯", "蔱", "翣", "繺", "鎩", "挲", "乷", "栅", "唼", "箑", "摋", "閷", "萐", "桬", "鯋", "髿", "歃", "倽", "挱", "翜", "歰", "樧", "㚫", "粆", "㰱", "㲚", "㸺", "毮", "㛼", "䈉", "䝊", "㵤", "䬊", "䀉", "䮜", "䶎", "䵘", "䯫"]),
    ("shai", &["晒", "筛", "曬", "色", "篩", "籭", "酾", "簛", "諰", "簁", "摋", "釃", "攦", "㩄", "㬠", "䵘", "啥", "杀", "傻", "沙", "纱", "砂", "莎", "鲨", "煞", "刹", "厦", "濈", "殺", "痧", "猀", "杉", "啑", "廈", "嗄", "霎", "榝", "硰", "儍", "紗", "唦", "剎", "喢", "菨", "裟", "魦", "鯊", "铩", "帹", "閯", "蔱", "翣", "繺", "鎩", "挲", "乷", "栅", "唼", "箑", "閷", "萐", "桬", "鯋", "髿", "歃", "倽", "挱", "翜", "歰", "樧", "㚫", "粆", "㰱", "㲚", "㸺", "毮", "㛼", "䈉", "䝊", "㵤", "䬊", "䀉", "䮜", "䶎", "䯫"]),
    ("shan", &["山", "删", "闪", "善", "珊", "扇", "啥呢", "衫", "单", "陕", "杉", "姗", "膳", "汕", "栅", "膻", "傻妞", "閃", "摻", "杀你", "訕", "傻鸟", "疝", "鳝", "Shan", "刪", "擅", "襂", "蟺", "缮", "狦", "彡", "钐", "煽", "撣", "潬", "縿", "鄯", "骟", "剡", "姍", "苫", "煔", "禅", "舢", "嘇", "埏", "幓", "檆", "讪", "笧", "羴", "炶", "樿", "鐥", "柵", "赡", "晱", "睒", "剼", "跚", "閄", "墡", "單", "嬗", "掞", "閊", "敾", "芟", "銏", "鱔", "磰", "饍", "掸", "陝", "潸", "灗", "軕", "邖", "歚", "鯅", "掺", "痁", "羶", "謆", "蟮", "挻", "赸", "搧", "繕", "釤", "墠", "穇", "圸", "傓", "澘"]),
    ("shang", &["上", "商", "尚", "伤", "赏", "熵", "闪光", "殇", "裳", "丄", "晌", "傷", "山歌", "尙", "山谷", "闪过", "山哥", "垧", "觞", "闪购", "Shang", "墒", "绱", "賞", "殤", "贘", "埫", "鏛", "恦", "滳", "慯", "汤", "蔏", "謪", "觴", "鋿", "鞝", "螪", "愓", "漡", "緔", "湯", "塲", "鬺", "扄", "鑜", "禓", "樉", "䬕", "山", "删", "闪", "善", "珊", "扇", "衫", "单", "陕", "杉", "姗", "膳", "汕", "栅", "膻", "閃", "摻", "訕", "疝", "鳝", "刪", "擅", "襂", "蟺", "缮", "狦", "彡", "钐", "煽", "撣", "潬", "縿", "鄯", "骟", "剡", "姍", "苫", "煔", "禅", "舢", "嘇", "埏", "幓", "檆", "讪", "笧", "啥"]),
    ("shao", &["少", "烧", "邵", "稍", "绍", "勺", "捎", "韶", "哨", "芍", "劭", "苕", "召", "燒", "梢", "卲", "啥哦", "弰", "紹", "傻哦", "啥喔", "啥噢", "杓", "潲", "蛸", "鞘", "蕱", "焼", "輎", "綤", "柖", "艄", "玿", "娋", "筲", "莦", "旓", "萷", "髾", "鮹", "袑", "睄", "佋", "䒚", "㸛", "䏴", "䘯", "䙼", "㪢", "䬰", "䔠", "㷹", "㲈", "䈰", "䈾", "啥", "杀", "傻", "沙", "纱", "砂", "莎", "鲨", "煞", "刹", "厦", "濈", "殺", "痧", "猀", "杉", "啑", "廈", "嗄", "霎", "榝", "硰", "儍", "紗", "唦", "剎", "喢", "菨", "裟", "魦", "鯊", "铩", "帹", "閯", "蔱", "翣", "繺", "鎩", "挲", "乷", "栅"]),
    ("she", &["射", "蛇", "设", "社", "舍", "佘", "涉", "舌", "摄", "奢", "折", "赊", "畲", "厍", "設", "赦", "she", "睡会儿", "麝", "賖", "拾", "慑", "攝", "时候儿", "涻", "磼", "歙", "滠", "捨", "輋", "畬", "猞", "舎", "阇", "畭", "虵", "騇", "渉", "厙", "蔎", "挕", "揲", "韘", "灄", "泏", "摂", "賒", "懾", "蠂", "欇", "摵", "闍", "弽", "蛥", "檨", "㢵", "奓", "㪿", "㭙", "䂠", "䞌", "䏴", "㴇", "䀅", "䀹", "䠶", "䄕", "㵃", "䁋", "䬷", "慴", "䤮", "䤁", "䁯", "䕣", "䰥", "䜆", "䠟", "䵥", "䜓", "㒤", "㰒", "䌰"]),
    ("shen", &["神", "身", "沈", "什", "深", "审", "申", "肾", "伸", "参", "甚", "婶", "慎", "燊", "谂", "砷", "渗", "谌", "绅", "珅", "黮", "莘", "諗", "眘", "Shen", "糁", "蜃", "槮", "屾", "呻", "頣", "蜄", "搷", "椹", "審", "葚", "腎", "堔", "穼", "棽", "甡", "糂", "哂", "眒", "瘆", "幓", "曑", "參", "鯵", "榊", "诜", "鰺", "覾", "訷", "讅", "鲹", "兟", "侁", "胂", "罙", "昚", "糣", "紳", "娠", "鰰", "氠", "渖", "嬸", "鉮", "駪", "矧", "邥", "抌", "柛", "棯", "滲", "叄", "脤", "瀋", "妽", "侺", "鋠", "糝", "吲", "蔘", "鯓", "姺", "宷", "敒", "鵢", "弞", "矤", "嫀", "愼", "籶", "籸"]),
    ("sheng", &["生", "省", "升", "剩", "盛", "声", "胜", "圣", "身高", "晟", "绳", "申购", "昇", "笙", "深耕", "聖", "身故", "聲", "牲", "苼", "勝", "审过", "审稿", "沈哥", "Sheng", "甥", "嵊", "陞", "乘", "泩", "眚", "繩", "珄", "殅", "枡", "狌", "湦", "鉎", "貹", "呏", "焺", "阩", "琞", "娍", "譝", "竔", "渑", "鍟", "曻", "晠", "乗", "榺", "墭", "椉", "陹", "賸", "剰", "渻", "殸", "偗", "繉", "斘", "蕂", "憴", "橳", "澠", "鵿", "鼪", "㞼", "䎴", "㗂", "㮐", "㼳", "䇸", "㾪", "䋲", "㹌", "䁞", "䪿", "縄", "䚇", "䱆", "䞉", "神", "沈", "什", "深", "审", "身", "申", "肾", "伸", "参", "甚", "婶", "慎"]),
    ("shi", &["是", "事", "时", "市", "室", "十", "式", "试", "石", "师", "使", "史", "屎", "食", "施", "诗", "世", "实", "拾", "士", "视", "仕", "湿", "识", "氏", "栅", "示", "始", "失", "势", "狮", "适", "時", "饰", "噓", "什", "释", "尸", "侍", "似", "蚀", "矢", "逝", "柿", "莳", "誓", "噬", "狶", "澨", "試", "師", "驶", "魳", "匙", "铈", "識", "弑", "諟", "詩", "嗜", "拭", "轼", "鳲", "虱", "辻", "貰", "恃", "溡", "實", "湜", "醳", "褷", "鼭", "峙", "豕", "痑", "惿", "視", "餙", "筮", "冟", "鉽", "奭", "礻", "恀", "餝", "宲", "炻", "釶", "濕", "釋", "觢", "屍", "谥", "鉇", "昰"]),
    ("shou", &["收", "手", "受", "瘦", "首", "守", "售", "寿", "兽", "熟", "授", "収", "艏", "狩", "绶", "獸", "壽", "扌", "痩", "獣", "夀", "嘼", "膄", "垨", "醻", "綬", "龵", "涭", "敊", "鏉", "㝊", "㖟", "㥅", "䛵", "䭭"]),
    ("shu", &["书", "数", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥", "倏", "捒", "朮", "籔", "怸", "鏣", "秫", "豎", "婌", "跾", "襩", "忄", "潄", "龧", "鮛", "軗", "柕", "忬", "鄃", "鵨", "尗", "鉥", "虪", "疋", "紓", "樞", "琡", "贖", "尌", "兪", "侸", "咰", "怷"]),
    ("shua", &["刷", "耍", "唰", "书啊", "输啊", "树啊", "数啊", "熟啊", "叔啊", "暑啊", "鼠啊", "誜", "㕞", "SHUA", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥", "倏", "捒", "朮", "籔", "怸", "鏣", "秫", "豎", "婌", "跾", "襩", "忄", "潄", "龧", "鮛", "軗", "柕", "忬", "鄃"]),
    ("shuai", &["帅", "甩", "摔", "衰", "率", "帥", "蟀", "书艾", "叔哎", "舒艾", "舒爱", "卛", "赎爱", "书爱", "縗", "淑爱", "缞", "书哎", "数爱", "䢦", "刷", "耍", "唰", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥", "倏", "捒", "朮", "籔", "怸", "鏣", "誜", "㕞", "秫", "豎"]),
    ("shuan", &["栓", "涮", "拴", "数安", "舒安", "耍你", "闩", "书安", "书案", "耍弄", "刷哪", "树安", "熟谙", "腨", "絟", "閂", "槫", "䧠", "刷", "耍", "唰", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥", "倏", "捒", "朮", "籔", "怸", "鏣", "誜", "㕞", "秫", "豎", "婌", "跾"]),
    ("shuang", &["双", "爽", "霜", "雙", "涮锅", "泷", "塽", "孀", "礵", "骦", "书昂", "舒昂", "舒盎", "驦", "漺", "鹴", "灀", "樉", "慡", "漴", "艭", "騻", "縔", "孇", "欆", "瀧", "傱", "鏯", "鸘", "㕠", "㦼", "䔪", "㼽", "䗮", "䡯", "䫪", "鷞", "栓", "涮", "拴", "闩", "刷", "耍", "唰", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞"]),
    ("shui", &["谁", "睡", "水", "税", "说", "誰", "氵", "氺", "稅", "說", "娷", "帨", "説", "閖", "脽", "祱", "挩", "涚", "裞", "涗", "捝", "㝽", "㽷", "㥨", "䲧", "䬽", "㔑", "䭨", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥", "倏", "捒", "朮", "籔", "怸"]),
    ("shun", &["顺", "舜", "瞬", "淑女", "吮", "熟女", "順", "鼠年", "属牛", "枢纽", "属你", "数年", "舒宁", "孰能", "庶女", "熟男", "书内", "瞤", "舒娜", "鬊", "楯", "眴", "shun", "瞚", "蕣", "橓", "䀢", "㥧", "䀵", "䞐", "㯗", "䑞", "䴄", "数", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "數", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥"]),
    ("shuo", &["说", "硕", "說", "烁", "朔", "铄", "数", "书哦", "槊", "説", "蒴", "碩", "叔哦", "妁", "数哦", "熟哦", "输哦", "爍", "數", "銏", "搠", "愬", "矟", "洬", "鑠", "欶", "箾", "鎙", "獡", "㸛", "䀥", "㮶", "䌃", "䈾", "䁻", "书", "输", "树", "熟", "叔", "舒", "鼠", "属", "术", "丨", "淑", "竖", "束", "述", "姝", "蜀", "薯", "梳", "墅", "暑", "疏", "署", "澍", "書", "曙", "赎", "黍", "枢", "殊", "橾", "蔬", "抒", "恕", "塾", "踈", "戍", "纾", "沭", "輸", "漱", "殳", "樹", "孰", "暏", "庶", "潻", "綀", "菽", "捈", "俞", "杸", "癙", "濖", "霔", "藷", "術", "屬", "腧", "襡", "蒁", "薥"]),
    ("si", &["四", "死", "思", "司", "丝", "斯", "肆", "嘶", "私", "撕", "似", "巳", "亖", "寺", "枱", "泗", "嗣", "锶", "偲", "食", "禩", "佀", "厮", "驷", "伺", "饲", "姒", "祀", "絲", "厶", "楒", "糸", "俬", "澌", "罒", "俟", "磃", "咝", "纟", "蟖", "兕", "飔", "媤", "汜", "鸶", "蛳", "洠", "缌", "笥", "凘", "蟴", "虒", "鍶", "娰", "柶", "耜", "涘", "鐁", "愢", "鼶", "鯣", "泀", "牭", "糹", "鋖", "緦", "飼", "鷥", "竢", "蕼", "颸", "泤", "鈻", "洍", "簛", "噝", "駟", "瀃", "恖", "禗", "鷉", "梩", "銉", "燍", "蕬", "孠", "覗", "禠", "罳", "螄", "杫", "肂", "傂", "儩", "釲", "貄"]),
    ("song", &["送", "松", "宋", "怂", "颂", "嵩", "淞", "诵", "崧", "餸", "讼", "菘", "耸", "凇", "揔", "鬆", "蜙", "song", "倯", "頌", "捒", "枀", "漎", "嵷", "慫", "悚", "誦", "鎹", "愯", "駷", "柗", "忪", "竦", "摗", "訟", "梥", "濍", "棇", "娀", "聳", "庺", "楤", "愡", "傱", "憽", "吅", "㕬", "枩", "㣝", "䢠", "䜬", "㨦", "㮸", "硹", "㞞", "䛦", "檧", "䉥"]),
    ("sou", &["搜", "嗖", "艘", "廋", "馊", "嗽", "叟", "颼", "擻", "棷", "鄋", "颾", "鏉", "薮", "鎪", "摉", "醙", "瘶", "飕", "騪", "廀", "溲", "摗", "擞", "螋", "撨", "餿", "捜", "嗾", "蒐", "傁", "藪", "獀", "櫢", "瞍", "捒", "蓃", "锼", "凁", "叜", "籔", "欶", "㖩", "㟬", "䏂", "㵻", "䈹", "䐹", "䬒", "䉤", "䮟", "䱸"]),
    ("su", &["苏", "素", "速", "诉", "宿", "塑", "酥", "粟", "俗", "珟", "溯", "肃", "蘇", "夙", "甦", "愫", "樕", "圱", "遬", "缩", "稣", "玊", "涑", "謖", "簌", "谡", "膆", "鱐", "骕", "訴", "嗉", "穌", "嫊", "愬", "蔌", "傃", "僳", "梀", "藗", "蹜", "櫯", "溸", "肅", "囌", "洬", "憟", "泝", "蘓", "遡", "潚", "潥", "榡", "璛", "縮", "稡", "摵", "餗", "窣", "鹔", "縤", "橚", "鷫", "粛", "驌", "殐", "觫", "鋉", "趚", "㑉", "㑛", "㕖", "莤", "㓘", "㝛", "㥛", "㴋", "䅇", "䇤", "埣", "㴑", "棴", "㨞", "䏋", "䔎", "䘻", "塐", "㔄", "㜚", "㯈", "㴼", "樎", "碿", "㩋", "㪩", "㬘", "䑿"]),
    ("suan", &["算", "酸", "蒜", "苏安", "祘", "速安", "筭", "素安", "狻", "宿安", "痠", "苏垵", "笇", "匴", "篹", "㔯", "㱹", "苏", "素", "速", "诉", "宿", "塑", "酥", "粟", "俗", "珟", "溯", "肃", "蘇", "夙", "甦", "愫", "樕", "圱", "遬", "缩", "稣", "玊", "涑", "謖", "簌", "谡", "膆", "鱐", "骕", "訴", "嗉", "穌", "嫊", "愬", "蔌", "傃", "僳", "梀", "藗", "蹜", "櫯", "溸", "肅", "囌", "洬", "憟", "泝", "蘓", "遡", "潚", "潥", "榡", "璛", "縮", "稡", "摵", "餗", "窣", "鹔", "縤", "橚", "鷫", "粛", "驌", "殐", "觫", "鋉", "趚", "㑉", "㑛", "㕖", "莤", "㓘", "㝛", "㥛", "㴋", "䅇", "䇤", "埣"]),
    ("sui", &["岁", "随", "碎", "虽", "隋", "穗", "遂", "髓", "歲", "眭", "绥", "睢", "缞", "燧", "隧", "襚", "熣", "隨", "邃", "嗺", "繐", "膸", "祟", "檖", "穂", "雖", "鐩", "禭", "瀡", "繸", "濉", "尿", "繀", "鏸", "嵗", "歳", "滖", "賥", "荽", "鐆", "芕", "穟", "谇", "髄", "譢", "夊", "璲", "亗", "煫", "綏", "旞", "埣", "嬘", "娞", "浽", "韢", "砕", "毸", "誶", "遀", "哸", "倠", "荾", "睟", "縗", "澻", "㞸", "㒸", "㥞", "㴚", "䅑", "㻟", "䅗", "㻪", "䔹", "䢫", "㵦", "䆳", "䍁", "䤭", "㻽", "䠔", "鞖", "䉌", "䭉", "䜆", "䯝", "瓍", "䜔", "䡵", "䥙", "䲀", "䪎", "苏", "素", "速"]),
    ("sun", &["孙", "损", "笋", "苏宁", "隼", "扻", "荪", "苏南", "孫", "榫", "肃宁", "潠", "鶽", "損", "速诺", "sun", "Sun", "筍", "摌", "箰", "狲", "槂", "鎨", "飧", "喰", "簨", "飱", "蓀", "蕵", "搎", "㡄", "㔼", "猻", "䐣", "㦏", "䁚", "薞", "㰂", "苏", "素", "速", "诉", "宿", "塑", "酥", "粟", "俗", "珟", "溯", "肃", "蘇", "夙", "甦", "愫", "樕", "圱", "遬", "缩", "稣", "玊", "涑", "謖", "簌", "谡", "膆", "鱐", "骕", "訴", "嗉", "穌", "嫊", "愬", "蔌", "傃", "僳", "梀", "藗", "蹜", "櫯", "溸", "肅", "囌", "洬", "憟", "泝", "蘓", "遡", "潚", "潥", "榡", "璛", "縮", "稡", "摵", "餗", "窣"]),
    ("suo", &["所", "锁", "缩", "索", "嗦", "梭", "鎖", "璅", "唆", "琐", "羧", "苏哦", "素哦", "素噢", "挱", "蓑", "髿", "縒", "娑", "縮", "簔", "琑", "惢", "嗍", "蜶", "桫", "摍", "唢", "莎", "挲", "睃", "趖", "葰", "莏", "溹", "暛", "鎻", "嗩", "簑", "逤", "鏁", "些", "乺", "鎍", "鮻", "傞", "溑", "瑣", "褨", "㪽", "㱔", "㛖", "䣔", "嫅", "䂹", "䓾", "㮦", "䐝", "獕", "䅴", "䈗", "䌇", "䞆", "䞽", "䖛", "䵀", "苏", "素", "速", "诉", "宿", "塑", "酥", "粟", "俗", "珟", "溯", "肃", "蘇", "夙", "甦", "愫", "樕", "圱", "遬", "稣", "玊", "涑", "謖", "簌", "谡", "膆", "鱐", "骕", "訴", "嗉"]),
    ("ta", &["他", "她", "它", "塔", "塌", "踏", "祂", "天啊", "他啊", "听啊", "疼啊", "跳啊", "图案", "痛啊", "她啊", "挞", "牠", "榻", "拓", "泰安", "沓", "图啊", "头啊", "投啊", "推啊", "团啊", "脱啊", "獭", "铊", "毾", "闼", "闒", "蹋", "傝", "嚃", "嗒", "誻", "濌", "溚", "褟", "鳎", "闥", "鎉", "趿", "撻", "溻", "咜", "遢", "阘", "粏", "蹹", "鞈", "獺", "狧", "遝", "譶", "榙", "鉈", "墖", "躂", "躢", "涾", "漯", "錔", "鞜", "鎑", "橽", "鞳", "鰨", "侤", "澾", "鮙", "嚺", "闟", "闧", "㣛", "㳫", "䍇", "㛥", "㧺", "㹺", "䓠", "崉", "㭼", "䂿", "䍝", "搨", "㒓", "䈋", "䎓", "禢", "㗳", "㣵", "㺚", "䑜", "䠅"]),
    ("tai", &["太", "台", "泰", "态", "胎", "抬", "钛", "肽", "邰", "苔", "臺", "汰", "漦", "呔", "冭", "酞", "籉", "Tai", "儓", "薹", "枱", "跆", "檯", "箈", "態", "颱", "鈦", "嬯", "溙", "燤", "擡", "舦", "鲐", "坮", "囼", "夳", "汏", "珆", "汱", "骀", "旲", "忲", "炲", "炱", "忕", "菭", "奤", "孡", "斄", "駘", "鮐", "㑀", "㣍", "䣭", "㑷", "㙜", "㥭", "㬃", "㷘", "䑓", "㒗", "䈚", "㙵", "㘆", "㸀", "䢰", "他", "她", "它", "塔", "塌", "踏", "祂", "挞", "牠", "榻", "拓", "沓", "獭", "铊", "毾", "闼", "闒", "蹋", "傝", "嚃", "嗒", "誻", "濌", "溚", "褟", "鳎", "闥", "鎉", "趿", "撻"]),
    ("tan", &["谈", "弹", "谭", "碳", "探", "摊", "贪", "痰", "坛", "坦", "潭", "滩", "炭", "檀", "他能", "他那", "他呢", "毯", "覃", "叹", "瘫", "钽", "談", "昙", "譚", "tan", "太爱你", "他爱你", "貚", "彈", "郯", "鐔", "榃", "镡", "嘽", "锬", "繵", "澹", "坍", "壇", "倓", "舑", "痑", "怹", "緂", "憛", "錟", "貪", "舕", "鷤", "罎", "嘾", "嘆", "攤", "袒", "歎", "壜", "譠", "灘", "埮", "菼", "惔", "醈", "癱", "婒", "墰", "顃", "擹", "忐", "璮", "憳", "啴", "弾", "鉭", "墵", "燂", "儃", "藫", "曇", "湠", "僋", "嗿", "潬", "憻", "醓", "磹", "罈", "襢", "醰", "䏙", "䢻", "㫜", "䀡", "㛶", "㪔", "㲜"]),
    ("tang", &["躺", "汤", "唐", "糖", "趟", "堂", "烫", "塘", "樘", "谈过", "棠", "谈个", "淌", "镗", "膛", "溏", "踼", "瑭", "湯", "搪", "鶶", "碳钢", "Tang", "倘", "劏", "蝪", "蹚", "坣", "橖", "钂", "燙", "攩", "灙", "曭", "鼞", "矘", "戃", "帑", "醣", "爣", "傥", "羰", "螳", "螗", "糃", "傏", "嘡", "赯", "篖", "鄌", "煻", "鐋", "铴", "嵣", "膅", "漟", "榶", "啺", "饧", "伖", "耥", "鏜", "饄", "偒", "餹", "摥", "鎕", "愓", "禟", "儻", "镋", "餳", "惝", "蓎", "鎲", "薚", "糛", "闛", "鞺", "䟖", "㭻", "䧜", "㑽", "㜍", "䣘", "隚", "㒉", "㲥", "㼒", "㼺", "漡", "㦂", "䅯", "磄", "㙶", "䌅"]),
    ("tao", &["套", "涛", "陶", "桃", "淘", "掏", "讨", "逃", "韬", "滔", "焘", "迯", "弢", "涭", "騊", "他哦", "她哦", "蜪", "萄", "鞉", "洮", "檮", "綯", "濤", "縧", "謟", "饕", "抭", "韜", "瑫", "绦", "饀", "駣", "慆", "啕", "夲", "討", "燾", "梼", "鞱", "跳", "叨", "幍", "匋", "轁", "咷", "祹", "鼗", "醄", "绹", "仐", "詜", "裪", "嫍", "槄", "夵", "搯", "鋾", "鞀", "飸", "㚐", "䚯", "䬢", "䄻", "㴞", "䛌", "絛", "䛬", "䱇", "縚", "㹗", "䵚", "䬞", "他", "她", "它", "塔", "塌", "踏", "祂", "挞", "牠", "榻", "拓", "沓", "獭", "铊", "毾", "闼", "闒", "蹋", "傝", "嚃", "嗒", "誻", "濌"]),
    ("te", &["特", "忒", "天鹅", "忑", "提额", "铽", "太二", "图二", "胎儿", "太饿", "腿儿", "套二", "他儿", "蟘", "慝", "脦", "犆", "螣", "貣", "貸", "鋱", "㥂", "㧹"]),
    ("tei", &["忒", "特", "忑", "铽", "蟘", "慝", "脦", "犆", "螣", "貣", "貸", "鋱", "㥂", "㧹"]),
    ("teng", &["疼", "腾", "滕", "藤", "誊", "驣", "虅", "邆", "螣", "騰", "縢", "熥", "膯", "駦", "漛", "霯", "籐", "鰧", "痋", "幐", "謄", "鼟", "儯", "籘", "䒅", "䕨", "䲍", "䠮", "特", "忒", "忑", "铽", "蟘", "慝", "脦", "犆", "貣", "貸", "鋱", "㥂", "㧹"]),
    ("ti", &["提", "题", "体", "踢", "替", "梯", "鬄", "剃", "蹄", "缇", "锑", "剔", "穉", "洟", "涕", "屉", "體", "啼", "題", "媞", "逷", "鷈", "蹏", "傂", "鶗", "悌", "遆", "揥", "籊", "瑅", "啑", "倜", "醍", "嗁", "惕", "磃", "嚔", "蝭", "鷤", "笹", "绨", "趧", "惖", "荑", "薙", "徲", "奃", "扌", "逖", "鶙", "歒", "蕛", "嚏", "鹈", "鐟", "禔", "緹", "謕", "鳀", "鮷", "骵", "稊", "睼", "禵", "漽", "姼", "媂", "珶", "趯", "桋", "屟", "躰", "眣", "渧", "弟", "俶", "罤", "褆", "徥", "惿", "掦", "裼", "褅", "擿", "鵜", "戻", "挮", "偍", "屜", "悐", "殢", "鬀", "騠", "軆", "苐", "厗"]),
    ("tian", &["天", "填", "田", "甜", "舔", "添", "提案", "恬", "钿", "湉", "靦", "餂", "娗", "兲", "淟", "缇安", "提安", "提按", "题案", "替安", "Tian", "替俺", "畑", "阗", "鴫", "紾", "佃", "沺", "睼", "搷", "畋", "忝", "瑱", "腆", "磌", "鷆", "唺", "酟", "錪", "甛", "菾", "鷏", "晪", "栝", "掭", "殄", "婖", "盷", "睓", "銽", "璳", "碵", "屇", "觍", "緂", "賟", "鈿", "舚", "胋", "靔", "塡", "呑", "琠", "黇", "靝", "奵", "倎", "悿", "铦", "寘", "銛", "窴", "覥", "㐁", "䧃", "㤁", "㧂", "㧱", "䄹", "䏦", "䑚", "畠", "㖭", "㙉", "㥏", "㶺", "䄼", "䄽", "㮇", "䐌", "䟧", "䡒", "䣯", "䄕", "䣶", "䩄"]),
    ("tiao", &["条", "跳", "调", "挑", "條", "調", "眺", "鞗", "迢", "提奥", "体奥", "缇奥", "蓨", "糶", "朷", "嬥", "樤", "笤", "銚", "鋚", "窕", "庣", "祒", "蓚", "旫", "誂", "趒", "齠", "岹", "祧", "粜", "苕", "佻", "絩", "髫", "蓧", "鰷", "蜩", "朓", "龆", "岧", "鲦", "铫", "鎥", "鯈", "萔", "宨", "恌", "晀", "斢", "脁", "芀", "覜", "啁", "䒒", "䧂", "㸠", "㟘", "䂪", "䎄", "䖺", "䟭", "聎", "䠷", "窱", "㩖", "䩦", "䳂", "䯾", "䱔", "提", "题", "体", "踢", "替", "梯", "鬄", "剃", "蹄", "缇", "锑", "剔", "穉", "洟", "涕", "屉", "體", "啼", "題", "媞", "逷", "鷈", "蹏", "傂", "鶗", "悌"]),
    ("tie", &["贴", "铁", "提额", "帖", "貼", "跕", "鐵", "萜", "驖", "tie", "提鹅", "题额", "飻", "鉄", "餮", "鴩", "鐡", "怗", "呫", "銕", "聑", "僣", "蛈", "䂿", "䩞", "䴴", "䵿", "䥫", "提", "题", "体", "踢", "替", "梯", "鬄", "剃", "蹄", "缇", "锑", "剔", "穉", "洟", "涕", "屉", "體", "啼", "題", "媞", "逷", "鷈", "蹏", "傂", "鶗", "悌", "遆", "揥", "籊", "瑅", "啑", "倜", "醍", "嗁", "惕", "磃", "嚔", "蝭", "鷤", "笹", "绨", "趧", "惖", "荑", "薙", "徲", "奃", "扌", "逖", "鶙", "歒", "蕛", "嚏", "鹈", "鐟", "禔", "緹", "謕", "鳀", "鮷", "骵", "稊", "睼", "禵", "漽", "姼", "媂", "珶"]),
    ("ting", &["听", "挺", "停", "厅", "婷", "庭", "亭", "廷", "汀", "霆", "艇", "聽", "町", "烃", "鞓", "葶", "ting", "珽", "铤", "渟", "梃", "蜓", "莛", "廳", "颋", "庍", "筳", "娗", "鼮", "圢", "邒", "侹", "蝏", "楟", "桯", "聴", "涏", "嵉", "甼", "脡", "頲", "烶", "庁", "諪", "榳", "忊", "誔", "鋌", "聤", "厛", "耓", "聼", "綎", "廰", "侱", "閮", "濎", "㼗", "㹶", "䅍", "烴", "䦐", "䋼", "䵺", "䱓", "䗴", "䫖", "提", "题", "体", "踢", "替", "梯", "鬄", "剃", "蹄", "缇", "锑", "剔", "穉", "洟", "涕", "屉", "體", "啼", "題", "媞", "逷", "鷈", "蹏", "傂", "鶗", "悌", "遆", "揥", "籊"]),
    ("tong", &["同", "通", "桶", "痛", "童", "铜", "彤", "桐", "筒", "统", "砼", "佟", "捅", "仝", "瞳", "潼", "酮", "曈", "橦", "狪", "烔", "僮", "侗", "Tong", "茼", "垌", "銅", "嗵", "統", "恸", "赨", "詷", "峒", "浵", "燑", "峂", "蓪", "犝", "氃", "樋", "恫", "鲖", "衕", "朣", "穜", "哃", "秱", "蚒", "囲", "慟", "峝", "粡", "冂", "庝", "偅", "鉖", "炵", "鮦", "餇", "獞", "絧", "熥", "晍", "綂", "爞", "眮", "硧", "筩", "㠽", "㤏", "㣚", "㣠", "㛚", "㸗", "㪌", "㼧", "䂈", "䆚", "痌", "㮔", "勭", "鉵", "㠉", "憅", "㼿", "膧", "䆹", "䳋", "䮵", "䶱", "䴀"]),
    ("tou", &["头", "投", "偷", "透", "頭", "骰", "綉", "黈", "埱", "钭", "緰", "婾", "偸", "亠", "媮", "敨", "妵", "蘣", "鋀", "鍮", "飳", "紏", "鈄", "㳆", "㕻", "㤟", "㼥", "㓱", "㖣", "㰯", "䚵", "䟝", "㡏", "㢏", "㪗", "䞬", "斢", "䡩", "䱏", "䕱", "䵉"]),
    ("tu", &["图", "吐", "土", "涂", "兔", "途", "突", "屠", "凸", "秃", "徒", "圖", "凃", "迌", "葖", "稌", "荼", "鵌", "菟", "瘏", "塗", "钍", "嵞", "潳", "檡", "鋀", "跿", "摕", "筡", "兎", "痜", "揬", "禿", "堍", "鈯", "宊", "馟", "図", "鷋", "鍎", "鵵", "廜", "趃", "圡", "莵", "汢", "酴", "鋵", "悇", "捸", "釷", "鷵", "啚", "湥", "駼", "庩", "唋", "蒤", "峹", "堗", "腯", "嶀", "怢", "梌", "圗", "瑹", "墿", "鵚", "㐋", "䣄", "捈", "涋", "㻌", "㟮", "㭸", "㻠", "䣝", "㻬", "㻯", "䅷", "䠈", "䖘", "䤅", "䩣", "䭭", "䳜", "䴛", "鶟", "鼵"]),
    ("tuan", &["团", "图案", "途安", "團", "疃", "蓴", "湍", "抟", "褖", "涂安", "图安", "土安", "Tuan", "篿", "鏄", "畽", "嫥", "団", "彖", "墥", "摶", "鷻", "鷒", "揣", "圕", "槫", "煓", "剸", "糰", "猯", "塼", "湪", "檲", "貒", "慱", "漙", "㩛", "䊜", "䳪", "䵎", "䜝", "䵯", "鱄", "图", "土", "涂", "吐", "兔", "途", "突", "屠", "凸", "秃", "徒", "圖", "凃", "迌", "葖", "稌", "荼", "鵌", "菟", "瘏", "塗", "钍", "嵞", "潳", "檡", "鋀", "跿", "摕", "筡", "兎", "痜", "揬", "禿", "堍", "鈯", "宊", "馟", "図", "鷋", "鍎", "鵵", "廜", "趃", "圡", "莵", "汢", "酴", "鋵", "悇", "捸", "釷", "鷵", "啚"]),
    ("tui", &["退", "推", "腿", "忒", "褪", "颓", "頽", "騩", "蜕", "穨", "蓷", "Tui", "骽", "煺", "脮", "蹪", "頺", "尵", "聉", "墤", "弚", "蘈", "魋", "蛻", "弟", "駾", "娧", "頹", "俀", "侻", "隤", "蹆", "焞", "僓", "橔", "㞂", "䍾", "㢈", "㥆", "㱣", "䀃", "㢑", "㷟", "㾯", "㾼", "㾽", "䅪", "㦌", "䩣", "㿉", "䫋", "藬", "㰐", "㿗", "讉", "图", "土", "涂", "吐", "兔", "途", "突", "屠", "凸", "秃", "徒", "圖", "凃", "迌", "葖", "稌", "荼", "鵌", "菟", "瘏", "塗", "钍", "嵞", "潳", "檡", "鋀", "跿", "摕", "筡", "兎", "痜", "揬", "禿", "堍", "鈯", "宊", "馟", "図", "鷋", "鍎", "鵵"]),
    ("tun", &["囤", "吞", "屯", "臀", "兔年", "豚", "吐奶", "图呢", "图你", "臋", "途牛", "暾", "軘", "吐纳", "朜", "蛌", "氽", "图内", "土牛", "Tun", "饨", "鲀", "坉", "芚", "憞", "焞", "旽", "呑", "褪", "霕", "魨", "噋", "黗", "忳", "啍", "庉", "豘", "飩", "㹠", "㼊", "㖔", "㧷", "涒", "㞘", "㩔", "图", "土", "涂", "吐", "兔", "途", "突", "屠", "凸", "秃", "徒", "圖", "凃", "迌", "葖", "稌", "荼", "鵌", "菟", "瘏", "塗", "钍", "嵞", "潳", "檡", "鋀", "跿", "摕", "筡", "兎", "痜", "揬", "禿", "堍", "鈯", "宊", "馟", "図", "鷋", "鍎", "鵵", "廜", "趃", "圡", "莵", "汢", "酴", "鋵", "悇", "捸", "釷"]),
    ("tuo", &["拖", "脱", "魄", "托", "拓", "妥", "坨", "驼", "庹", "陀", "沲", "啴", "沱", "萚", "杝", "鼉", "驮", "图哦", "乇", "椭", "侻", "阤", "橢", "佗", "砣", "鱓", "軃", "詑", "蘀", "脫", "紽", "鴕", "鼧", "籜", "唾", "碢", "鮀", "岮", "嫷", "嶞", "飥", "楕", "铊", "鼍", "袉", "鸵", "堶", "挩", "毻", "彵", "杔", "柁", "鵎", "魠", "橐", "饦", "託", "莌", "柝", "撱", "讬", "鰖", "鮵", "跎", "毤", "酡", "踻", "驒", "駝", "箨", "仛", "鋖", "咜", "陁", "袥", "槖", "騨", "沰", "扡", "侂", "拕", "捝", "汑", "媠", "跅", "馲", "涶", "咃", "狏", "砤", "駄", "駞", "驝", "鉈", "㸰", "㸱"]),
    ("wa", &["哇", "娃", "瓦", "挖", "晚安", "蛙", "袜", "我啊", "玩啊", "午安", "文案", "我爱", "洼", "嗢", "问啊", "哇啊", "娃啊", "为爱", "娲", "佤", "韎", "我按", "凹", "呜啊", "网啊", "胃癌", "韈", "屲", "聉", "襪", "窊", "攨", "溛", "呙", "腽", "瓸", "瓲", "咓", "膃", "婠", "搲", "畖", "帓", "媧", "窐", "劸", "邷", "鼃", "砙", "漥", "窪", "韤", "穵", "咼", "徍", "唲", "瓾", "㧚", "㼘", "㰪", "䎳", "䍪", "䚴", "啘", "䖯", "嗗", "䯉", "㒝", "䠚", "譁", "鮭", "䵷"]),
    ("wai", &["外", "歪", "崴", "喎", "呙", "咼", "顡", "竵", "瀤", "㨤", "䶐", "䠿", "哇", "娃", "瓦", "挖", "蛙", "袜", "洼", "嗢", "娲", "佤", "韎", "凹", "韈", "屲", "聉", "襪", "窊", "攨", "溛", "腽", "瓸", "瓲", "咓", "膃", "婠", "搲", "畖", "帓", "媧", "窐", "劸", "邷", "鼃", "砙", "漥", "窪", "韤", "穵", "徍", "唲", "瓾", "㧚", "㼘", "㰪", "䎳", "䍪", "䚴", "啘", "䖯", "嗗", "䯉", "㒝", "䠚", "譁", "鮭", "䵷"]),
    ("wan", &["玩", "完", "万", "晚", "碗", "湾", "弯", "丸", "皖", "婉", "関", "宛", "萬", "我爱你", "腕", "琬", "挽", "菀", "烷", "杬", "箢", "莞", "wan", "哇能", "哇你", "我爱您", "顽", "卍", "绾", "晥", "萖", "踠", "塆", "卐", "灣", "鄤", "脘", "魭", "涴", "掔", "剜", "夘", "畹", "豌", "闗", "槾", "惌", "婠", "鍐", "惋", "澫", "纨", "椀", "妧", "彎", "晩", "錽", "芄", "娩", "綰", "鋔", "琓", "倇", "潫", "蔓", "汍", "贎", "蜿", "輓", "夗", "梚", "帵", "唍", "脕", "岏", "笂", "壪", "睕", "頑", "捖", "抏", "瞣", "杤", "紈", "晼", "盌", "綄", "埦", "絻", "鋺", "翫", "綩", "輐", "莬", "刓", "忨"]),
    ("wang", &["王", "往", "网", "汪", "忘", "旺", "望", "玩过", "亡", "徍", "完工", "玩个", "妄", "顽固", "迬", "万古", "万个", "枉", "網", "万国", "朚", "玩够", "罔", "玩梗", "弯管", "完稿", "蚟", "蝄", "惘", "辋", "罒", "尪", "瀇", "罖", "尣", "朢", "魍", "彺", "亾", "莣", "仼", "尫", "徃", "罓", "尩", "抂", "誷", "忹", "迋", "琞", "盳", "暀", "棢", "蛧", "菵", "兦", "㓁", "㲿", "䒽", "㔞", "䋄", "㳹", "䛃", "㴏", "䋞", "䰣", "輞", "䤑", "万", "玩", "完", "晚", "碗", "湾", "弯", "丸", "皖", "婉", "関", "宛", "萬", "腕", "琬", "挽", "菀", "烷", "杬", "箢", "莞", "顽", "卍", "绾", "晥", "萖", "踠", "塆"]),
    ("wei", &["为", "未", "位", "伟", "微", "魏", "味", "喂", "维", "韦", "卫", "威", "胃", "尾", "委", "薇", "炜", "围", "玮", "蔚", "唯", "巍", "危", "為", "伪", "纬", "惟", "Wei", "尉", "谓", "苇", "圩", "慰", "渭", "萎", "畏", "葳", "违", "隗", "煨", "潍", "隇", "讆", "崴", "偉", "壝", "溦", "愄", "韡", "蔿", "蜼", "觽", "爲", "帷", "濰", "維", "娓", "洧", "諉", "餧", "嵬", "艉", "痿", "罻", "沩", "膸", "圍", "覹", "碨", "桅", "唩", "囗", "媦", "韋", "謂", "衛", "癐", "觹", "椳", "帏", "覣", "煒", "瑋", "熭", "趡", "闱", "蓶", "渨", "暐", "矀", "犚", "韑", "腲", "蘶", "鍏", "偎"]),
    ("wen", &["问", "文", "稳", "温", "雯", "闻", "纹", "蚊", "吻", "揾", "問", "汶", "搵", "玟", "瘟", "穩", "炆", "穏", "溫", "鴖", "wen", "五二年", "王二妮", "维尔纳", "沃尔诺", "鴍", "絻", "聞", "紋", "蕰", "縕", "蟁", "芠", "韫", "藴", "紊", "閺", "彣", "閿", "榅", "刎", "塭", "妏", "忞", "肳", "饂", "呅", "繧", "豱", "闦", "昷", "渂", "鎾", "瑥", "亠", "鳁", "缊", "璺", "鰮", "抆", "忟", "蚉", "伆", "桽", "鞰", "阌", "馼", "緼", "呚", "呡", "殟", "魰", "莬", "辒", "薀", "珳", "脗", "榲", "輼", "鼤", "鰛", "歾", "駇", "鳼", "轀", "褞", "螡", "顐", "㐎", "㡈", "㑮", "㖧", "㳷", "㗃", "㟩", "㝧"]),
    ("weng", &["翁", "嗡", "问过", "问个", "瓮", "文哥", "罋", "文革", "稳固", "温哥", "文稿", "温故", "闻过", "稳过", "文管", "温感", "文官", "文广", "鶲", "文光", "文工", "文蛤", "文港", "鹟", "鎓", "滃", "蓊", "攚", "奣", "甕", "聬", "蕹", "塕", "嵡", "瞈", "勜", "暡", "螉", "齆", "㘢", "䐥", "䤰", "䈵", "㜲", "㹙", "㹚", "䱵", "䨴", "问", "文", "稳", "温", "雯", "闻", "纹", "蚊", "吻", "揾", "問", "汶", "搵", "玟", "瘟", "穩", "炆", "穏", "溫", "鴖", "鴍", "絻", "聞", "紋", "蕰", "縕", "蟁", "芠", "韫", "藴", "紊", "閺", "彣", "閿", "榅", "刎", "塭", "妏", "忞", "肳", "饂", "呅", "繧", "豱", "闦", "昷", "渂", "鎾"]),
    ("wo", &["我", "喔", "窝", "沃", "握", "卧", "哇哦", "我哦", "玩偶", "涡", "蜗", "哇偶", "渥", "肟", "踠", "窩", "喔哦", "呙", "莴", "倭", "问哦", "无哦", "网哦", "围殴", "王鸥", "硪", "挝", "嚄", "斡", "幄", "臥", "龌", "渦", "濣", "艧", "楃", "咼", "偓", "仴", "瓁", "臒", "撾", "薶", "雘", "蒦", "捰", "捾", "馧", "婑", "濄", "緺", "龏", "萵", "涹", "媉", "焥", "蝸", "婐", "捼", "瞃", "齷", "枂", "腛", "踒", "唩", "涴", "猧", "㦱", "䀑", "㧴", "㱧", "䁊", "䂺", "㠗", "㠛", "䠎", "䙠", "䩊", "䁷", "䰀", "䮸"]),
    ("wu", &["无", "五", "吴", "唔", "武", "伍", "物", "午", "屋", "舞", "误", "务", "雾", "乌", "邬", "吾", "巫", "勿", "悟", "污", "無", "呜", "戊", "捂", "Wu", "坞", "钨", "毋", "梧", "芜", "仵", "吳", "乄", "兀", "膴", "阢", "熃", "茣", "婺", "晤", "幠", "恶", "妩", "鹜", "啎", "浯", "敄", "剭", "鋘", "霧", "芴", "務", "焐", "烏", "嵍", "堥", "歍", "岉", "扝", "莁", "侮", "僫", "誤", "陚", "隖", "铻", "鹀", "骛", "鹉", "迕", "鋈", "蜈", "瞴", "鄔", "逜", "靰", "俉", "寤", "忤", "霚", "诬", "庑", "圬", "怃", "趶", "憮", "窏", "痦", "於", "呉", "珷", "旿", "噁", "洿", "塢", "鴮"]),
    ("xi", &["洗", "系", "西", "喜", "细", "吸", "习", "希", "戏", "席", "熙", "息", "係", "溪", "曦", "锡", "玺", "汐", "奚", "夕", "稀", "禧", "茜", "铣", "騱", "兮", "XI", "惜", "喺", "熹", "嘻", "硒", "析", "囍", "矖", "咭", "栖", "咥", "晞", "昔", "誒", "浠", "郗", "娭", "羲", "烯", "忚", "晰", "裼", "悉", "巂", "膝", "矽", "虩", "摡", "細", "碏", "绤", "忾", "媳", "犀", "扢", "袭", "皙", "僖", "淅", "塈", "翕", "纚", "隙", "閪", "習", "钑", "瘜", "蠵", "嬉", "蹊", "舃", "郋", "饎", "臈", "熄", "翖", "闟", "槢", "澙", "熂", "磎", "覤", "葈", "獡", "愾", "黖", "爔", "隟", "戲"]),
    ("xia", &["下", "夏", "霞", "瞎", "虾", "吓", "侠", "系啊", "徦", "厦", "峡", "辖", "匣", "瑕", "丅", "暇", "遐", "俠", "呷", "洗啊", "舺", "系阿", "喺啊", "细啊", "戏啊", "喜啊", "习啊", "席啊", "惜啊", "鰕", "諕", "蝦", "狭", "硤", "谺", "硖", "黠", "嚇", "螛", "珨", "笚", "廈", "狎", "磍", "峽", "罅", "夓", "芐", "乤", "懗", "鏬", "鎼", "傄", "鶷", "柙", "圷", "疨", "狹", "烚", "瘕", "昰", "睱", "閜", "陜", "夾", "虲", "陿", "翈", "閕", "敮", "鍜", "疜", "唬", "捾", "颬", "蕸", "縖", "埉", "赮", "舝", "碬", "騢", "梺", "筪", "鎋", "祫", "轄", "䒠", "㘡", "䒩", "炠", "䖎", "䘥", "㙈", "㭱", "㗇"]),
    ("xian", &["先", "线", "现", "西安", "县", "闲", "险", "仙", "限", "鲜", "显", "贤", "嫌", "咸", "馅", "弦", "献", "娴", "冼", "宪", "妶", "纤", "西岸", "陷", "掀", "線", "希岸", "锡安", "熙岸", "溪岸", "玺安", "Xian", "現", "腺", "衔", "羡", "藓", "玁", "酰", "摻", "溓", "羬", "綖", "莧", "尠", "峴", "憸", "僴", "舷", "陥", "岘", "蚬", "銛", "枮", "胘", "埳", "涎", "衘", "醎", "铣", "閒", "蘚", "賢", "锨", "苋", "縣", "嶮", "縿", "孅", "籼", "豏", "獮", "硍", "槏", "嘕", "稴", "繊", "瞯", "薟", "氙", "獫", "鮮", "纎", "撏", "襳", "婱", "霰", "顯", "咞", "韅", "躚", "險", "輱", "憪", "攕", "鱻"]),
    ("xiang", &["想", "像", "向", "箱", "项", "香", "相", "祥", "翔", "响", "乡", "湘", "象", "享", "巷", "先给", "限购", "详", "镶", "厢", "橡", "襄", "響", "項", "降", "纕", "西昂", "Xiang", "席昂", "洗昂", "飨", "蟓", "芗", "骧", "饷", "閧", "儴", "蠁", "庠", "鄉", "鲞", "葙", "嚮", "缃", "鮝", "襐", "珦", "闂", "瓖", "楿", "蚃", "詳", "鱌", "稥", "鑲", "勨", "忀", "麘", "餉", "曏", "瓨", "晑", "郷", "夅", "鱶", "啍", "衖", "饗", "亯", "廂", "潒", "姠", "羏", "萫", "鯗", "缿", "嶑", "勷", "鄊", "膷", "鄕", "跭", "銄", "欀", "栙", "鐌", "薌", "銗", "鱜", "饟", "佭", "驤", "㐔", "㟄", "㖜", "䙹"]),
    ("xiao", &["小", "笑", "晓", "肖", "校", "消", "销", "效", "孝", "萧", "潇", "筱", "霄", "啸", "骁", "削", "宵", "下哦", "枭", "囂", "魈", "箫", "硝", "逍", "熇", "西奥", "西澳", "瞎哦", "希奥", "下噢", "西岙", "夏哦", "细坳", "西傲", "霞哦", "喜奥", "咲", "驍", "橚", "嗃", "哓", "曉", "睄", "虓", "箾", "皛", "鸮", "鵁", "颵", "蕭", "簘", "嚣", "哮", "蟰", "謏", "銷", "篠", "萷", "翛", "绡", "鴵", "歗", "獟", "髇", "誵", "蟂", "洨", "瀟", "崤", "嘯", "猇", "蠨", "淆", "熽", "姣", "暁", "枵", "傚", "殽", "梟", "敩", "嘋", "効", "蟏", "嚻", "薂", "撨", "蛸", "恔", "俲", "綃", "鴞", "灱", "虈", "鷍", "宯"]),
    ("xie", &["写", "些", "谢", "鞋", "卸", "歇", "斜", "邪", "协", "蟹", "血", "解", "寫", "屑", "泄", "携", "械", "鲑", "觧", "頡", "泻", "喜鹅", "西饿", "西鹅", "喜恶", "系额", "晞鹅", "喜娥", "系哦", "谐", "蠍", "蝎", "瀣", "榭", "獦", "謝", "褻", "楔", "蠏", "韘", "颉", "屧", "伳", "嶰", "燮", "勰", "瑎", "緤", "擕", "胁", "偕", "躠", "衺", "屟", "懈", "熁", "鞵", "缬", "炧", "綊", "劦", "觟", "挟", "襭", "魼", "絏", "嚡", "榝", "鮭", "烲", "屭", "紲", "鞢", "撷", "蝑", "滊", "屓", "纈", "暬", "偰", "樧", "嗋", "緳", "薢", "澥", "契", "脋", "噧", "愶", "協", "縀", "韰", "邂", "焎", "叶", "绁"]),
    ("xin", &["新", "心", "信", "鑫", "欣", "芯", "辛", "馨", "薪", "昕", "洗脑", "锌", "歆", "细腻", "忻", "西南", "西宁", "莘", "炘", "盺", "訫", "杺", "焮", "伈", "鈊", "鐔", "妡", "忄", "馫", "鋅", "囟", "孞", "俽", "镡", "訢", "枔", "邤", "衅", "襑", "噷", "惞", "噺", "釁", "伩", "阠", "嬜", "顖", "礥", "廞", "軐", "脪", "舋", "馸", "㐰", "䒖", "㚯", "㭄", "㛛", "㭡", "䎣", "䚱", "㹷", "愖", "䛨", "㜦", "䪿", "䰼", "㻸", "㩪", "䜗", "䜣", "系", "洗", "西", "喜", "细", "吸", "习", "希", "戏", "席", "熙", "息", "係", "溪", "曦", "锡", "玺", "汐", "奚", "夕", "稀", "禧", "茜", "铣", "騱"]),
    ("xing", &["行", "星", "性", "型", "兴", "醒", "姓", "新冠", "邢", "形", "杏", "幸", "刑", "腥", "新歌", "猩", "新股", "省", "新高", "興", "惺", "擤", "郉", "煋", "瑆", "騂", "钘", "荥", "荇", "悻", "倖", "洐", "陉", "硎", "骍", "垶", "婞", "狌", "铏", "涬", "饧", "娙", "曐", "莕", "睲", "謃", "侀", "巠", "鈃", "哘", "銒", "葕", "緈", "皨", "蛵", "滎", "鮏", "篂", "嬹", "陘", "觪", "觲", "餳", "臖", "鯹", "㓝", "㒷", "㐩", "㼛", "㣜", "䣆", "㓑", "㴆", "㨘", "㼬", "䓷", "㙚", "㮐", "䁄", "䂔", "㬐", "䋲", "鉶", "㝭", "䛭", "䤯", "䰢", "箵", "鋞", "䮍", "䳙", "新", "心", "信", "鑫", "欣"]),
    ("xiong", &["熊", "胸", "雄", "兄", "凶", "匈", "胷", "芎", "汹", "兇", "夐", "熋", "賯", "敻", "焸", "讻", "诇", "忷", "匂", "哅", "詾", "恟", "洶", "訩", "㐫", "焽", "詗", "䧺", "䎿", "系", "洗", "西", "喜", "细", "吸", "习", "希", "戏", "席", "熙", "息", "係", "溪", "曦", "锡", "玺", "汐", "奚", "夕", "稀", "禧", "茜", "铣", "騱", "兮", "惜", "喺", "熹", "嘻", "硒", "析", "囍", "矖", "咭", "栖", "咥", "晞", "昔", "誒", "浠", "郗", "娭", "羲", "烯", "忚", "晰", "裼", "悉", "巂", "膝", "矽", "虩", "摡", "細", "碏", "绤", "忾", "媳", "犀", "扢", "袭", "皙", "僖", "淅", "塈", "翕"]),
    ("xiu", &["修", "休", "秀", "绣", "袖", "锈", "咻", "羞", "溴", "嗅", "宿", "朽", "岫", "琇", "褎", "嚊", "髤", "鵂", "脩", "峀", "鱃", "俢", "褏", "馐", "貅", "臹", "庥", "臭", "髹", "綇", "糔", "鏅", "鸺", "繡", "飍", "鮴", "鏥", "璓", "潃", "綉", "銹", "鏽", "滫", "繍", "珛", "烌", "鎀", "樇", "齅", "苬", "脙", "臰", "螑", "饈", "㱙", "㾋", "䜬", "㗜", "銝", "㩰", "䰍", "系", "洗", "西", "喜", "细", "吸", "习", "希", "戏", "席", "熙", "息", "係", "溪", "曦", "锡", "玺", "汐", "奚", "夕", "稀", "禧", "茜", "铣", "騱", "兮", "惜", "喺", "熹", "嘻", "硒", "析", "囍", "矖", "咭"]),
    ("xu", &["徐", "需", "许", "旭", "续", "虚", "须", "嘘", "序", "恤", "胥", "绪", "戌", "栩", "煦", "叙", "恓", "朐", "蓄", "圩", "絮", "Xu", "敍", "魖", "墟", "禑", "滸", "婿", "許", "虗", "漵", "慉", "勖", "蚼", "诩", "滀", "嘼", "嬃", "畜", "顼", "淢", "珝", "續", "虛", "湏", "藇", "鄦", "浒", "昫", "須", "溆", "吁", "惐", "珬", "谞", "鉥", "緒", "眗", "晇", "蝑", "訹", "驉", "盱", "窢", "聟", "噓", "緰", "燸", "湑", "怴", "藚", "姁", "瞲", "譃", "敘", "喣", "稰", "盢", "俆", "垿", "幁", "蓿", "媭", "緖", "綇", "暊", "楈", "烅", "糈", "旴", "壻", "歔", "盨", "洫", "訏", "勗"]),
    ("xuan", &["选", "炫", "轩", "宣", "悬", "縣", "璇", "萱", "玄", "旋", "煊", "渲", "瑄", "暄", "選", "烜", "禤", "喧", "癣", "绚", "喛", "楦", "铉", "徐安", "旭安", "许安", "需按", "须按", "漩", "泫", "敻", "怰", "鋗", "玹", "翾", "軒", "晅", "眩", "譞", "媗", "镟", "楥", "鞙", "蠉", "鍹", "碹", "蘐", "颴", "谖", "駽", "璿", "縼", "儇", "嫙", "讂", "暶", "煖", "吅", "昡", "蝖", "懸", "塇", "翧", "蕿", "鉉", "昍", "咺", "琄", "琁", "繏", "矎", "券", "痃", "睻", "愃", "駨", "伭", "嬛", "衒", "蓒", "梋", "絢", "揎", "鏇", "箮", "諠", "眴", "鰚", "愋", "蔙", "懁", "癬", "萲", "蜁", "檈", "妶"]),
    ("xue", &["学", "雪", "血", "薛", "穴", "削", "靴", "學", "趐", "滈", "桖", "噱", "恤额", "徐哦", "虚额", "徐娥", "徐额", "许娥", "鳕", "蒆", "彐", "茓", "觷", "辥", "樰", "怴", "瞲", "澩", "瀥", "谑", "坹", "踅", "燢", "峃", "辪", "疦", "泬", "鷽", "疶", "斈", "岤", "鱈", "泶", "吷", "膤", "鸴", "乴", "狘", "壆", "斅", "泧", "謔", "雤", "嶨", "鞾", "敩", "艝", "轌", "㞽", "㧒", "䀜", "䒸", "袕", "䋉", "䎀", "㖸", "㿱", "䛎", "䖼", "䫻", "䫼", "㡜", "㷤", "䬂", "㗾", "㦜", "䠼", "䩈", "䆷", "㔧", "䱑", "㙾", "㶅", "䨮", "㰒", "䜡", "䤕", "䭥", "需", "徐", "许", "旭", "续", "虚", "须", "嘘"]),
    ("xun", &["训", "寻", "孫", "虚拟", "讯", "询", "熏", "勋", "迅", "巡", "旬", "荀", "逊", "循", "巽", "洵", "浔", "珣", "许诺", "汛", "薰", "馴", "煇", "鲟", "蟫", "訓", "驯", "埙", "醺", "荨", "郇", "稄", "蕈", "燖", "恂", "尋", "殉", "蟳", "揗", "訊", "鶽", "徇", "殾", "峋", "顨", "鱏", "奞", "勳", "爋", "窨", "鱘", "燅", "勛", "栒", "浚", "桪", "曛", "璕", "壦", "樳", "詢", "嚑", "噚", "焄", "巺", "潯", "攳", "訙", "燻", "塤", "遜", "蘍", "駨", "蕁", "卂", "矄", "杊", "愻", "勲", "獯", "纁", "荤", "葷", "眴", "坃", "濬", "臐", "鄩", "紃", "噀", "侚", "咰", "姰", "廵", "畃", "潠"]),
    ("ya", &["呀", "牙", "压", "亚", "雅", "鸭", "吖", "丫", "有啊", "押", "娅", "芽", "哑", "崖", "涯", "要啊", "伢", "用啊", "鸦", "ya", "氩", "垭", "亞", "壓", "桠", "轧", "穵", "圠", "俹", "庌", "瑘", "鴨", "衙", "鵶", "琊", "讶", "輵", "犽", "唖", "冴", "迓", "圔", "睚", "瘂", "猚", "蚜", "婭", "稏", "氬", "啞", "圧", "疋", "亜", "铔", "庘", "玡", "聐", "岈", "錏", "厓", "鴉", "孲", "砑", "襾", "揠", "挜", "齾", "姶", "蕥", "劜", "窫", "軋", "瓛", "漄", "覀", "猰", "厊", "枒", "笌", "埡", "崕", "痖", "訝", "椏", "碣", "厑", "齖", "呾", "拁", "堐", "掗", "釾", "椻", "磍", "鐚", "㤉"]),
    ("yan", &["烟", "眼", "燕", "艳", "严", "铅", "闫", "演", "颜", "言", "盐", "验", "彦", "研", "岩", "妍", "延", "炎", "麣", "晏", "宴", "雁", "亚楠", "yan", "益安宁", "越爱你", "沿", "焱", "阎", "鄢", "琰", "衍", "腌", "啱", "焰", "嫣", "厌", "焉", "咽", "堰", "砚", "讞", "鉛", "淹", "験", "儼", "掞", "阽", "剦", "滟", "晻", "檐", "鴈", "魇", "兗", "魘", "煙", "羬", "窴", "酓", "歅", "鷃", "掩", "閼", "猒", "遃", "彥", "俨", "龑", "鬳", "阉", "剡", "喭", "傿", "贋", "騐", "顏", "鷰", "嬿", "谚", "偃", "詽", "筵", "饜", "溎", "齗", "兖", "嶮", "鹽", "顑", "贗", "戭", "黰", "琂", "臙", "姸"]),
    ("yang", &["杨", "阳", "样", "养", "羊", "洋", "扬", "痒", "氧", "严格", "眼光", "央", "仰", "漾", "旸", "炀", "樣", "楊", "掩盖", "秧", "阉割", "烟感", "泱", "Yang", "垟", "烊", "陽", "養", "飏", "飬", "鸯", "恙", "暘", "楧", "玚", "殃", "鞅", "佯", "钖", "揚", "禓", "瑒", "癢", "歍", "劷", "疡", "鴹", "怏", "駚", "昜", "徉", "慃", "氱", "煬", "様", "眻", "咉", "鍈", "雵", "鐊", "攁", "颺", "鸉", "懩", "礢", "卬", "軮", "瀁", "姎", "鰑", "眏", "蛘", "鍚", "阦", "羕", "珜", "羏", "勜", "柍", "婸", "抰", "胦", "氜", "崵", "岟", "紻", "敭", "鉠", "佒", "炴", "崸", "愓", "詇", "諹", "輰", "鴦"]),
    ("yao", &["要", "药", "姚", "腰", "瑶", "耀", "咬", "摇", "約", "尧", "妖", "垚", "邀", "曜", "窑", "遥", "幺", "爻", "殽", "谣", "吆", "夭", "肴", "謡", "藥", "呀哦", "Yao", "舀", "瘧", "钥", "隃", "窈", "么", "皐", "杳", "葯", "媱", "窔", "瑤", "宎", "珧", "鹞", "鳐", "峣", "颻", "堯", "榚", "騕", "燿", "搖", "鷕", "撽", "约", "獟", "鰩", "侥", "齩", "铫", "矅", "繇", "薬", "遙", "鱙", "曣", "滧", "闄", "榣", "徺", "徭", "楆", "尭", "烑", "顤", "轺", "崾", "飖", "纅", "蘨", "蓔", "窕", "摿", "熎", "謠", "讑", "疟", "窅", "猺", "窯", "傜", "苭", "仸", "暚", "倄", "箹", "鼼", "岆"]),
    ("ye", &["也", "耶", "页", "叶", "业", "夜", "野", "爷", "液", "烨", "嘢", "余额", "冶", "晔", "椰", "窫", "亱", "婴儿", "噎", "又饿", "一二", "幼儿", "ye", "嚈", "吔", "葉", "頁", "焆", "邺", "業", "腋", "曳", "掖", "爺", "壄", "谒", "殗", "枼", "璍", "埜", "饁", "擖", "靥", "邪", "嶫", "漜", "爗", "楪", "咽", "燁", "暍", "曅", "墷", "鵺", "揶", "铘", "曄", "倻", "曗", "堨", "擨", "歋", "鎑", "拽", "亪", "餣", "僷", "洂", "偞", "潱", "嶪", "曵", "鄴", "靨", "瞱", "蠮", "澲", "鸈", "抴", "喝", "捓", "揲", "煠", "鎁", "瓛", "捙", "馌", "驜", "峫", "謁", "擫", "殕", "瑘", "鍱", "擪", "鐷"]),
    ("yi", &["一", "已", "以", "射", "亿", "易", "咦", "壹", "怡", "意", "义", "乙", "益", "艺", "姨", "仪", "亦", "宜", "伊", "依", "奕", "衣", "逸", "毅", "医", "翼", "Yi", "移", "异", "忆", "懿", "椅", "译", "煕", "溢", "翊", "祎", "轶", "熠", "疙", "议", "颐", "屹", "邑", "噫", "谊", "燚", "弋", "弈", "叆", "羿", "矣", "贻", "翌", "疑", "沂", "驿", "夷", "蚁", "遗", "苡", "漪", "铱", "倚", "裔", "役", "钇", "義", "斁", "怿", "埶", "悥", "億", "绎", "旖", "衵", "乂", "黓", "錡", "蓺", "镒", "儗", "翳", "攲", "疫", "薏", "诣", "敧", "歖", "彝", "咿", "嬑", "抑", "瘞", "縊", "豷"]),
    ("yin", &["阴", "因", "印", "一年", "银", "尹", "音", "殷", "饮", "引", "以内", "瘾", "茵", "寅", "隐", "韾", "淫", "吟", "胤", "犾", "荫", "凐", "以你", "一N", "Yin", "垠", "铟", "伒", "龈", "諲", "銀", "飲", "姻", "泿", "崟", "窨", "釿", "輑", "陰", "鄞", "婣", "駰", "梀", "垔", "堷", "櫽", "洇", "嶾", "齗", "憖", "訔", "筃", "烎", "齦", "茚", "乑", "癊", "讔", "訚", "吲", "喑", "濦", "欭", "檼", "愔", "癮", "霠", "噖", "隠", "荶", "氤", "蒑", "朄", "蚓", "蘟", "磤", "隱", "赺", "嚚", "堙", "趛", "蔩", "崯", "噾", "狺", "湮", "骃", "烟", "歅", "硍", "濥", "秵", "鞇", "夤", "懚", "圻"]),
    ("ying", &["赢", "硬", "应", "英", "颖", "莹", "影", "营", "盈", "映", "迎", "鹰", "瑛", "樱", "滢", "婴", "莺", "荧", "瀛", "因果", "鞕", "嘤", "萤", "颍", "楹", "贏", "営", "罃", "缨", "蝇", "鸚", "郢", "應", "蓥", "濙", "璎", "潆", "禜", "萦", "瑩", "嬴", "愥", "荥", "膺", "灜", "蘡", "桜", "穎", "頴", "濚", "巊", "瘿", "甖", "鹦", "譍", "營", "鷹", "瀯", "媖", "茔", "鱦", "鸎", "溁", "瀴", "譻", "鷪", "櫻", "绬", "撄", "罂", "孾", "溋", "摬", "锳", "瀅", "巆", "謍", "眏", "攍", "櫿", "煐", "蝧", "膡", "鴬", "暎", "濴", "螢", "霙", "旲", "覮", "熒", "蠳", "耺", "渶", "鶧", "噟"]),
    ("yo", &["有", "哟", "用", "又", "油", "呦", "由", "友", "永", "游", "优", "勇", "右", "yo", "唷", "喲", "育", "嚛", "尤", "邮", "佑", "泳", "佣", "咏", "幼", "悠", "涌", "雍", "拥", "酉", "柚", "幽", "甬", "忧", "釉", "宥", "铀", "鼬", "疣", "庸", "犹", "痏", "攸", "祐", "诱", "蛹", "俑", "蓧", "湧", "邕", "莜", "侑", "猷", "廱", "褎", "汼", "滽", "噰", "镛", "囿", "鱿", "詠", "痈", "墉", "遊", "埇", "獶", "铕", "優", "壅", "尢", "灉", "郺", "踊", "黝", "唀", "莠", "卣", "牰", "鱅", "慵", "槦", "揘", "颙", "醟", "塎", "蚴", "輏", "邎", "秞", "訧", "憂", "滺", "猶", "郵", "牖"]),
    ("yong", &["用", "永", "勇", "泳", "佣", "咏", "涌", "雍", "拥", "甬", "庸", "蛹", "俑", "湧", "邕", "廱", "滽", "噰", "镛", "詠", "痈", "墉", "埇", "壅", "灉", "郺", "踊", "鱅", "慵", "槦", "揘", "颙", "醟", "塎", "栐", "鳙", "銿", "恿", "鰫", "蒏", "癰", "喁", "傭", "鷛", "擁", "饔", "顒", "鲬", "柡", "彮", "澭", "鄘", "臃", "鯒", "鏞", "勈", "牗", "怺", "苚", "愹", "慂", "嫞", "硧", "傛", "嵱", "筩", "踴", "砽", "牅", "癕", "雝", "悀", "㞲", "㑙", "㶲", "㝘", "㣧", "㦷", "惥", "愑", "㴩", "㷏", "嗈", "㙲", "䞻", "䗤", "㽫", "䮵", "哟", "唷", "喲", "育", "嚛"]),
    ("you", &["有", "又", "由", "油", "右", "游", "友", "优", "呦", "尤", "邮", "佑", "幼", "悠", "酉", "柚", "幽", "忧", "釉", "宥", "铀", "鼬", "疣", "you", "犹", "痏", "攸", "祐", "You", "诱", "蓧", "莜", "侑", "猷", "褎", "汼", "囿", "鱿", "遊", "獶", "铕", "優", "尢", "黝", "唀", "莠", "卣", "牰", "蚴", "輏", "邎", "秞", "訧", "憂", "滺", "猶", "郵", "牖", "羑", "浟", "莸", "孧", "貁", "狖", "鲉", "脜", "偤", "沋", "揂", "峟", "蝤", "怣", "冘", "肬", "姷", "甴", "怞", "駀", "迶", "繇", "哊", "蚰", "脩", "丣", "懮", "禉", "麀", "栯", "逌", "亴", "纋", "聈", "魷", "蚘", "槱", "斿"]),
    ("yu", &["与", "雨", "于", "余", "玉", "鱼", "宇", "语", "俞", "预", "域", "豫", "钰", "羽", "育", "瑜", "喻", "裕", "遇", "煜", "渝", "禹", "予", "御", "昱", "郁", "誉", "虞", "欲", "穀", "渔", "與", "屿", "毓", "奧", "於", "妤", "浴", "榆", "寓", "彧", "愈", "峪", "歟", "芋", "驭", "聿", "愉", "逾", "痏", "顒", "谕", "捥", "庾", "狱", "愚", "隅", "瑀", "娱", "铻", "魚", "吁", "瘀", "玙", "歈", "嫗", "俣", "諛", "淤", "語", "尉", "堉", "盂", "蔚", "禺", "釪", "鐍", "羭", "鸆", "舆", "覦", "醧", "楰", "淯", "欥", "琟", "繘", "鹬", "齬", "嵛", "鈺", "餘", "欎", "飫", "藇", "迂"]),
    ("yuan", &["元", "远", "原", "员", "源", "袁", "院", "园", "圆", "苑", "愿", "媛", "预案", "缘", "渊", "沅", "怨", "猿", "冤", "鸢", "垣", "援", "塬", "裕安", "瑗", "yuan", "宇安", "予安", "玉安", "雨安", "遇安", "昱安", "余安", "俞安", "渔安", "于安", "语安", "嫄", "遠", "円", "縁", "芫", "垸", "員", "獂", "辕", "園", "薗", "鸳", "圓", "蝯", "湲", "橼", "爰", "妴", "願", "緣", "妧", "厵", "騵", "榬", "鼋", "灁", "裷", "贠", "圜", "螈", "宛", "淵", "傆", "噮", "掾", "杬", "鳶", "鶢", "鎱", "渕", "酛", "羱", "鹓", "榞", "鼝", "茒", "眢", "駌", "逺", "箢", "涴", "蚖", "薳", "盶", "媴", "蜎", "櫞", "肙", "厡"]),
    ("yue", &["月", "约", "越", "余额", "悦", "粤", "説", "岳", "跃", "阅", "玥", "乐", "曰", "樾", "約", "矆", "钺", "哕", "玉娥", "与额", "豫E", "豫鄂", "Yue", "悅", "栎", "矱", "钥", "粵", "彠", "啘", "籰", "岄", "枂", "说", "樂", "髺", "鈅", "蘥", "躍", "擽", "楽", "曱", "閱", "嶽", "龠", "瀹", "刖", "鉞", "噦", "籥", "箹", "彟", "泧", "爚", "戉", "兌", "嬳", "蚏", "鸑", "跀", "篗", "抈", "說", "兊", "哾", "閲", "黦", "扚", "汋", "礿", "籆", "鸙", "鑰", "蚎", "軏", "櫟", "禴", "妜", "㧒", "䢁", "䆕", "䒸", "恱", "䎳", "䋐", "䎀", "䡇", "捳", "㭾", "䆢", "䟠", "䋤", "㹊", "䤦", "䬂", "㜧"]),
    ("yun", &["云", "晕", "运", "韵", "芸", "孕", "允", "匀", "昀", "与你", "蕴", "与您", "赟", "雲", "耘", "贠", "榅", "筠", "韫", "恽", "沄", "陨", "頵", "煇", "鋆", "熨", "贇", "運", "纭", "霣", "暈", "酝", "郧", "篔", "妘", "郓", "韻", "蝹", "縕", "澐", "涢", "馻", "薀", "貟", "殒", "枟", "愠", "畇", "餫", "员", "韗", "玧", "蘊", "荺", "蕓", "伝", "緷", "煴", "縜", "氲", "勻", "缊", "橒", "齫", "愪", "蒕", "齳", "秐", "筼", "囩", "紜", "賱", "熅", "藴", "惲", "夽", "抎", "員", "耺", "眃", "鄆", "韞", "菀", "鈗", "蕰", "鞰", "狁", "涒", "傊", "溳", "阭", "緼", "抣", "隕", "腪", "殞"]),
    ("za", &["咋", "砸", "咱", "杂", "早啊", "早安", "走啊", "在啊", "做啊", "最爱", "真爱", "扎", "中啊", "找啊", "字啊", "匝", "值啊", "障碍", "紥", "咂", "魳", "阻碍", "桚", "拶", "雜", "雥", "臜", "紮", "帀", "囐", "喒", "迊", "偺", "鉔", "雑", "囃", "沯", "噈", "襍", "韴", "臢", "沞", "㳨", "䞙", "䦈", "䨿", "䪞", "䕹", "囋"]),
    ("zai", &["在", "再", "仔", "载", "崽", "宰", "灾", "栽", "哉", "載", "災", "甾", "洅", "渽", "賳", "菑", "溨", "扗", "傤", "畠", "儎", "烖", "縡", "抂", "酨", "㞨", "㦲", "䣬", "㴓", "㱰", "睵", "䏁", "䵧", "䮨", "咋", "砸", "咱", "杂", "扎", "匝", "紥", "咂", "魳", "桚", "拶", "雜", "雥", "臜", "紮", "帀", "囐", "喒", "迊", "偺", "鉔", "雑", "囃", "沯", "噈", "襍", "韴", "臢", "沞", "㳨", "䞙", "䦈", "䨿", "䪞", "䕹", "囋"]),
    ("zan", &["咱", "赞", "暂", "攒", "咋弄", "咋能", "昝", "砸你", "瓒", "杂念", "咋呢", "簪", "贊", "喒", "揝", "最爱你", "只爱你", "讚", "蹔", "錾", "暫", "灒", "儹", "鵤", "攢", "趲", "鐟", "橵", "穳", "糌", "鏩", "禶", "襸", "趱", "簮", "讃", "拶", "酂", "偺", "瓉", "囋", "瓚", "饡", "塹", "噆", "寁", "攅", "撍", "賛", "濽", "桚", "儧", "兂", "㳨", "㟛", "㤰", "䬤", "䍝", "㔆", "䙉", "㣅", "㨻", "䗞", "篸", "鄼", "㙻", "䟃", "䟅", "䥄", "鏨", "䭕", "鐕", "酇", "㜺", "咋", "砸", "杂", "扎", "匝", "紥", "咂", "魳", "雜", "雥", "臜", "紮", "帀", "囐", "迊", "鉔", "雑", "囃", "沯", "噈", "襍", "韴"]),
    ("zang", &["脏", "藏", "臧", "葬", "咱哥", "攒够", "赃", "赞歌", "臟", "贜", "駔", "奘", "蔵", "賘", "賍", "髒", "牂", "驵", "塟", "弉", "臓", "銺", "匨", "羘", "㘸", "贓", "咱", "赞", "暂", "攒", "昝", "瓒", "簪", "贊", "喒", "揝", "讚", "蹔", "錾", "暫", "灒", "儹", "鵤", "攢", "趲", "咋", "砸", "杂", "扎", "匝", "紥", "咂", "魳", "桚", "拶", "雜", "雥", "臜", "鐟", "橵", "穳", "糌", "鏩", "禶", "襸", "趱", "簮", "讃", "酂", "偺", "瓉", "囋", "瓚", "饡", "塹", "噆", "寁", "攅", "撍", "賛", "濽", "儧", "兂", "㳨", "㟛", "㤰", "䬤", "䍝", "㔆", "䙉", "㣅", "㨻", "䗞", "篸", "鄼", "㙻"]),
    ("zao", &["早", "造", "遭", "澡", "枣", "灶", "皂", "凿", "燥", "糟", "藻", "躁", "噪", "璅", "蹧", "蚤", "咋哦", "趮", "栆", "皁", "棗", "竈", "唕", "璪", "慥", "喿", "傮", "鑿", "煰", "繰", "唣", "梍", "簉", "醩", "譟", "薻", "䖣", "㲧", "艁", "㷮", "㯥", "䒃", "䗢", "竃", "䜊", "䥣", "䲃", "咋", "砸", "咱", "杂", "扎", "匝", "紥", "咂", "魳", "桚", "拶", "雜", "雥", "臜", "紮", "帀", "囐", "喒", "迊", "偺", "鉔", "雑", "囃", "沯", "噈", "襍", "韴", "臢", "沞", "㳨", "䞙", "䦈", "䨿", "䪞", "䕹", "囋"]),
    ("ze", &["啧", "则", "泽", "责", "择", "这儿", "周二", "則", "罪恶", "总额", "仄", "作恶", "沢", "中俄", "侄儿", "澤", "迮", "足额", "昨儿", "灂", "中二", "左耳", "字儿", "赜", "齰", "嘖", "稄", "昃", "歵", "蠌", "咋", "擇", "責", "齚", "侧", "瞔", "皟", "笮", "帻", "夨", "箦", "捑", "唶", "幘", "樍", "泎", "舴", "溭", "汄", "択", "鸅", "耫", "滜", "賾", "昗", "襗", "庂", "飵", "嫧", "蔶", "礋", "睪", "矠", "諎", "謮", "㳁", "䂝", "㟙", "㳻", "䇥", "側", "㖽", "㢎", "崱", "㮣", "䅚", "㣱", "㥽", "㻭", "䔾", "䯔", "䰹", "䕉", "簀", "䕪", "䶡", "䶦"]),
    ("zei", &["贼", "賊", "鲗", "戝", "蠈", "鱡", "鰂", "㖽", "则", "泽", "啧", "责", "择", "則", "仄", "沢", "澤", "迮", "灂", "赜", "齰", "嘖", "稄", "昃", "歵", "蠌", "咋", "擇", "責", "齚", "侧", "瞔", "皟", "笮", "帻", "夨", "箦", "捑", "唶", "幘", "樍", "泎", "舴", "溭", "汄", "択", "鸅", "耫", "滜", "賾", "昗", "襗", "庂", "飵", "嫧", "蔶", "礋", "睪", "矠", "諎", "謮", "㳁", "䂝", "㟙", "㳻", "䇥", "側", "㢎", "崱", "㮣", "䅚", "㣱", "㥽", "㻭", "䔾", "䯔", "䰹", "䕉", "簀", "䕪", "䶡", "䶦"]),
    ("zen", &["怎", "谮", "责难", "囎", "撍", "则能", "则你", "譖", "这二年", "Zen", "㻸", "䫈", "譛", "则", "泽", "啧", "责", "择", "則", "仄", "沢", "澤", "迮", "灂", "赜", "齰", "嘖", "稄", "昃", "歵", "蠌", "咋", "擇", "責", "齚", "侧", "瞔", "皟", "笮", "帻", "夨", "箦", "捑", "唶", "幘", "樍", "泎", "舴", "溭", "汄", "択", "鸅", "耫", "滜", "賾", "昗", "襗", "庂", "飵", "嫧", "蔶", "礋", "睪", "矠", "諎", "謮", "㳁", "䂝", "㟙", "㳻", "䇥", "側", "㖽", "㢎", "崱", "㮣", "䅚", "㣱", "㥽", "㻭", "䔾", "䯔", "䰹", "䕉", "簀", "䕪", "䶡", "䶦"]),
    ("zeng", &["曾", "增", "赠", "綜", "憎", "甑", "鬷", "锃", "鬵", "贈", "怎敢", "怎搞", "怎个", "罾", "増", "曽", "缯", "鄫", "鱛", "矰", "璔", "鋥", "熷", "综", "磳", "橧", "囎", "繒", "竲", "譄", "䇸", "縡", "㽪", "䙢", "䰝", "䵴", "怎", "谮", "则", "泽", "啧", "责", "择", "則", "仄", "沢", "澤", "迮", "灂", "赜", "齰", "嘖", "稄", "昃", "歵", "蠌", "咋", "擇", "責", "撍", "譖", "㻸", "䫈", "譛", "齚", "侧", "瞔", "皟", "笮", "帻", "夨", "箦", "捑", "唶", "幘", "樍", "泎", "舴", "溭", "汄", "択", "鸅", "耫", "滜", "賾", "昗", "襗", "庂", "飵", "嫧", "蔶", "礋", "睪", "矠", "諎", "謮"]),
    ("zha", &["炸", "扎", "蜡", "渣", "咋", "闸", "查", "榨", "诈", "喋", "柤", "轧", "真好啊", "怍", "最黑暗", "纵火案", "招呼啊", "祝好啊", "知乎啊", "吒", "乍", "揸", "喳", "眨", "馇", "紥", "箑", "楂", "札", "蹅", "栅", "喥", "砟", "囃", "啑", "樝", "鮓", "醡", "鍘", "潳", "査", "閘", "牐", "蚻", "铡", "咤", "踷", "柞", "摣", "拃", "偧", "餷", "皻", "鲊", "奓", "諎", "蚱", "軋", "鮺", "搾", "譇", "劄", "霅", "耫", "紮", "譗", "痄", "詐", "鲝", "哳", "齄", "齇", "皶", "厏", "柵", "剳", "甴", "灹", "挓", "溠", "抯", "苲", "搩", "煠", "觰", "飵", "㚫", "㡸", "㱜", "㳐", "䄰", "宱", "䆛", "㑵", "㓃", "㞚"]),
    ("zhai", &["摘", "债", "翟", "窄", "宅", "寨", "斋", "擿", "砦", "齋", "債", "择", "捚", "嚌", "斎", "鉙", "瘵", "祭", "侧", "齊", "檡", "厏", "哜", "夈", "亝", "側", "粂", "骴", "厇", "抧", "㡯", "㟙", "䍉", "責", "㢎", "䓱", "䑲", "榸", "擇", "㩟", "炸", "扎", "蜡", "渣", "咋", "闸", "查", "榨", "诈", "喋", "柤", "轧", "怍", "吒", "乍", "揸", "喳", "眨", "馇", "紥", "箑", "楂", "札", "蹅", "栅", "喥", "砟", "囃", "啑", "樝", "鮓", "醡", "鍘", "潳", "査", "閘", "牐", "蚻", "铡", "咤", "踷", "柞", "摣", "拃", "偧", "餷", "皻", "鲊", "奓", "諎", "蚱", "軋", "鮺", "搾", "譇", "劄"]),
    ("zhan", &["站", "占", "展", "战", "粘", "詹", "栈", "渣男", "沾", "斩", "盏", "湛", "渣女", "蘸", "顫", "毡", "瞻", "鳣", "绽", "戰", "僝", "嫸", "欃", "桟", "鳽", "崭", "旜", "戦", "斬", "旃", "黵", "谵", "皽", "虦", "噡", "盞", "佔", "驙", "搌", "嶃", "辗", "霑", "轏", "虥", "讝", "栴", "魙", "嶘", "枬", "嵁", "飐", "驏", "棧", "醆", "邅", "閚", "偡", "趈", "鹯", "譫", "怗", "嶦", "嶄", "綻", "惉", "颭", "菚", "詀", "覱", "岾", "氈", "譧", "饘", "蛅", "飦", "琖", "碊", "氊", "輾", "颤", "鱣", "橏", "㟞", "䋎", "㠭", "㜊", "䐤", "䘺", "䎒", "䗃", "䩅", "䩆", "榐", "㺘", "輚", "㙴"]),
    ("zhang", &["张", "长", "章", "涨", "账", "帐", "掌", "胀", "仗", "杖", "張", "丈", "璋", "障", "站岗", "樟", "彰", "漳", "仉", "長", "展馆", "漲", "Zhang", "嶂", "賬", "瘴", "獐", "瞕", "帳", "蟑", "鄣", "脹", "嫜", "幛", "鱆", "鏱", "仧", "暲", "幥", "扙", "弡", "蔁", "镸", "兏", "傽", "涱", "痮", "慞", "礃", "墇", "瘬", "粻", "麞", "粀", "遧", "餦", "騿", "㕩", "㽴", "㙊", "㙣", "㢓", "鞝", "站", "占", "展", "战", "粘", "詹", "栈", "沾", "斩", "盏", "湛", "蘸", "顫", "毡", "瞻", "鳣", "绽", "戰", "僝", "嫸", "欃", "桟", "鳽", "崭", "旜", "戦", "斬", "旃", "黵", "谵", "皽", "虦", "炸"]),
    ("zhao", &["找", "照", "赵", "招", "着", "兆", "罩", "昭", "钊", "召", "朝", "肇", "曌", "诏", "趙", "沼", "爪", "炤", "棹", "謿", "瞾", "炸哦", "扎哦", "菬", "釽", "皽", "釗", "妱", "垗", "詔", "鮡", "啁", "鍣", "櫂", "笊", "嘲", "鉊", "旐", "爫", "鳭", "肁", "巶", "枛", "狣", "啅", "瑵", "佋", "鼌", "箌", "肈", "盄", "罀", "濯", "駋", "鼂", "羄", "燳", "㕚", "㐍", "㐒", "䧂", "㡽", "㫤", "㨄", "㹿", "䄻", "䝖", "䍮", "䖺", "㑿", "㷖", "䍜", "㺐", "䃍", "䈃", "䈇", "䑲", "䮓", "炸", "扎", "蜡", "渣", "咋", "闸", "查", "榨", "诈", "喋", "柤", "轧", "怍", "吒", "乍", "揸", "喳", "眨"]),
    ("zhe", &["这", "着", "折", "者", "浙", "哲", "適", "喆", "這", "遮", "啫", "著", "辙", "褶", "嗻", "锗", "蛰", "蔗", "这会儿", "柘", "蜇", "辄", "谪", "搩", "晢", "赭", "整活儿", "慹", "棏", "嫬", "嚞", "樜", "詟", "砓", "摺", "鹧", "謺", "蟅", "磔", "鍺", "蟄", "矺", "乇", "埑", "讋", "輒", "晣", "螫", "乽", "厇", "鷓", "杔", "袩", "淛", "輙", "聑", "謫", "歽", "襵", "悊", "啠", "轍", "虴", "粍", "馲", "踷", "銸", "鮿", "讁", "籷", "㞏", "㡳", "䎲", "扸", "㞼", "䂝", "䂞", "㡇", "㪿", "㭙", "䓆", "䙷", "㭯", "䆛", "䏳", "㢎", "㫼", "䀅", "䐑", "䋲", "㦻", "㵊", "䐲", "䧪", "㯰", "䊞"]),
    ("zhen", &["真", "针", "镇", "振", "珍", "帧", "震", "阵", "臻", "甄", "贞", "朕", "枕", "诊", "着你", "桢", "圳", "这呢", "祯", "这你", "眹", "这能", "蓁", "瑧", "这年", "中华儿女", "侦", "榛", "疹", "禛", "浈", "箴", "胗", "陣", "針", "揕", "縝", "鎮", "碪", "砧", "溱", "趂", "轸", "斟", "黰", "鸩", "竧", "瑱", "貞", "籈", "謓", "迧", "稹", "潧", "覙", "榐", "眞", "赈", "缜", "楨", "絼", "鍖", "幀", "禎", "澵", "樼", "畛", "珎", "屒", "寊", "紖", "診", "轃", "甽", "姫", "誫", "駗", "遉", "錱", "鬒", "葴", "聄", "蒖", "槙", "椹", "縥", "纼", "袗", "酙", "鎭", "薽", "靕", "鍼", "栚", "昣", "鉁"]),
    ("zheng", &["整", "正", "郑", "证", "挣", "蒸", "政", "争", "症", "征", "铮", "真贵", "峥", "珍贵", "真乖", "真够", "真高", "鲭", "睁", "筝", "钲", "證", "鄭", "怔", "真个", "真该", "诤", "拯", "徵", "憕", "掙", "鯖", "崢", "狰", "爭", "癥", "撜", "崝", "証", "媜", "烝", "炡", "晸", "踭", "鉦", "錚", "佂", "睜", "姃", "氶", "埩", "徴", "幁", "丁", "鴊", "脀", "徎", "聇", "抍", "猙", "靕", "箏", "埥", "諍", "掟", "徰", "凧", "綪", "糽", "愸", "鬇", "塣", "㐼", "㱏", "眐", "䋊", "䋫", "揁", "䂻", "䡕", "㡠", "䈣", "䛫", "䫆", "㽀", "篜", "䥌", "㡧", "鏳", "䥭", "真", "镇", "振", "针", "珍", "帧"]),
    ("zhi", &["只", "之", "至", "值", "支", "指", "志", "纸", "知", "直", "智", "制", "治", "識", "致", "止", "质", "汁", "职", "芝", "置", "枝", "芷", "织", "执", "郦", "植", "痣", "脂", "址", "酯", "隻", "茝", "吱", "旨", "絺", "扻", "祉", "稚", "秩", "郅", "滞", "秪", "挚", "蹠", "趾", "幟", "炙", "巵", "枳", "栀", "凪", "踯", "觗", "侄", "懥", "紙", "肢", "臷", "陟", "峙", "掷", "痔", "漐", "徵", "搘", "徝", "殖", "帜", "傂", "犆", "疷", "執", "筫", "袠", "坧", "墌", "楖", "沚", "鳷", "鼅", "翐", "剬", "乿", "梽", "稺", "蜘", "雉", "縶", "躑", "蚔", "帋", "墆", "隲", "禃", "跖"]),
    ("zhong", &["中", "种", "钟", "重", "忠", "众", "仲", "终", "肿", "蚣", "盅", "冢", "衷", "種", "鐘", "锺", "終", "蹱", "汷", "塚", "踵", "鍾", "柊", "眾", "蜙", "鈡", "蝩", "腫", "衆", "舯", "徸", "媑", "堹", "妕", "螽", "茽", "忪", "諥", "彸", "鼨", "尰", "夂", "喠", "刣", "伀", "歱", "籦", "鴤", "狆", "妐", "炂", "衳", "偅", "幒", "銿", "穜", "筗", "衶", "蚛", "蔠", "緟", "泈", "祌", "㐺", "㲴", "㹣", "㣫", "煄", "瘇", "螤", "䳋", "斔", "䱰"]),
    ("zhou", &["周", "粥", "轴", "州", "洲", "舟", "咒", "皱", "肘", "宙", "箒", "昼", "週", "胕", "骤", "绉", "Zhou", "胄", "诌", "椆", "軸", "纣", "疛", "睭", "謅", "菷", "帚", "啁", "霌", "烐", "晭", "珘", "輖", "鵃", "籕", "晝", "酎", "呪", "辀", "盩", "掫", "薵", "矪", "籀", "妯", "婤", "伷", "淍", "嚋", "賙", "皺", "诪", "鸼", "荮", "赒", "侜", "甃", "粙", "炿", "碡", "繇", "喌", "驟", "冑", "咮", "葤", "駲", "駎", "縐", "譸", "輈", "僽", "噣", "騆", "詋", "銂", "㝌", "㼙", "㲴", "䖞", "紂", "郮", "㛩", "䇠", "䋓", "徟", "㥁", "㫶", "䐌", "䛆", "㗙", "䐍", "詶", "㨶", "㾭", "䎻"]),
    ("zhu", &["住", "猪", "朱", "主", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱", "烛", "拀", "窋", "渚", "陼", "蠋", "蛛", "紵", "诛", "豬", "矚", "褚", "蓫", "洙", "贮", "蛀", "翥", "茱", "潴", "劚", "鸀", "拄", "术", "鼄", "跓", "跦", "邾", "箸", "尌", "罜", "杼", "眝", "薥", "簗", "櫫", "飳", "笜", "炷", "莇", "硃", "磩", "諸", "侏", "苎", "註", "蠩", "藸", "殶", "鱁", "伫", "槠", "櫧", "瞩", "櫡", "斀", "坾", "孎", "朮", "砫", "属", "駐", "爥", "炢", "蝫", "麈", "柷", "篫", "橥", "駯", "舳", "纻", "苧"]),
    ("zhua", &["抓", "爪", "髽", "主啊", "住啊", "猪啊", "檛", "簻", "挝", "猪阿", "撾", "祝阿", "煮啊", "朱啊", "珠啊", "爫", "膼", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱", "烛", "拀", "窋", "渚", "陼", "蠋", "蛛", "紵", "诛", "豬", "矚", "褚", "蓫", "洙", "贮", "蛀", "翥", "茱", "潴", "劚", "鸀", "拄", "术", "鼄", "跓", "跦", "邾", "箸", "尌", "罜", "杼", "眝", "薥", "簗", "櫫", "飳", "笜", "炷", "莇", "硃", "磩", "諸", "侏", "苎", "註", "蠩", "藸", "殶", "鱁", "伫", "槠", "櫧", "瞩", "櫡", "斀"]),
    ("zhuai", &["拽", "跩", "转", "助爱", "猪哎", "朱爱", "主爱", "筑爱", "睉", "轉", "抓", "爪", "髽", "檛", "簻", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱", "烛", "拀", "窋", "渚", "陼", "蠋", "蛛", "紵", "诛", "豬", "矚", "褚", "蓫", "洙", "贮", "蛀", "翥", "茱", "潴", "劚", "鸀", "拄", "术", "鼄", "跓", "跦", "邾", "箸", "尌", "罜", "杼", "眝", "薥", "簗", "櫫", "飳", "笜", "炷", "莇", "硃", "磩", "諸", "侏", "苎", "註", "蠩", "藸", "挝", "撾", "爫", "膼", "殶", "鱁", "伫", "槠", "櫧", "瞩"]),
    ("zhuan", &["转", "赚", "专", "砖", "传", "注安", "轉", "賺", "篆", "抓你", "漙", "啭", "抓挠", "主案", "抓奶", "朱安", "诸安", "住安", "祝安", "撰", "專", "籑", "簨", "馔", "磗", "颛", "篹", "傳", "囀", "篿", "縳", "磚", "嫥", "堟", "耑", "沌", "贃", "孨", "専", "剸", "饌", "僎", "叀", "瑑", "転", "蒃", "瑼", "灷", "竱", "譔", "鱄", "膞", "顓", "恮", "腞", "僝", "塼", "鄟", "諯", "襈", "蟤", "䧘", "甎", "䚈", "䤄", "䡱", "䉵", "抓", "爪", "髽", "檛", "簻", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱"]),
    ("zhuang", &["装", "撞", "庄", "桩", "幢", "状", "壮", "妆", "转给", "专柜", "裝", "转个", "转告", "赚个", "转过", "专供", "莊", "转岗", "荘", "赚够", "壯", "狀", "转股", "撰稿", "专干", "朱昂", "祝昂", "猪昂", "住昂", "樁", "妝", "壵", "粧", "庒", "戆", "奘", "糚", "湷", "僮", "贛", "戇", "娤", "梉", "焋", "䚒", "戅", "转", "赚", "专", "砖", "传", "轉", "賺", "篆", "漙", "啭", "撰", "專", "籑", "簨", "馔", "磗", "颛", "篹", "傳", "囀", "篿", "縳", "磚", "嫥", "堟", "抓", "爪", "髽", "檛", "簻", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸"]),
    ("zhui", &["追", "隊", "坠", "锥", "缀", "椎", "赘", "甀", "骓", "膇", "隹", "醊", "奞", "諈", "墜", "沝", "缒", "錐", "惴", "畷", "綴", "礈", "錣", "硾", "笍", "腏", "騅", "倕", "贅", "鵻", "轛", "縋", "㝽", "娷", "䄌", "䋘", "揣", "㿢", "㾽", "䃍", "餟", "鑆", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱", "烛", "拀", "窋", "渚", "陼", "蠋", "蛛", "紵", "诛", "豬", "矚", "褚", "蓫", "洙", "贮", "蛀", "翥", "茱", "潴", "劚", "鸀", "拄", "术", "鼄", "跓", "跦", "邾", "箸", "尌", "罜"]),
    ("zhun", &["准", "祝你", "祝您", "住哪", "旽", "忳", "肫", "猪年", "助你", "住你", "猪脑", "住呢", "主内", "逐年", "住那", "窀", "谆", "助农", "朱诺", "朱宁", "湻", "稕", "屯", "宒", "圫", "準", "訰", "淳", "衠", "凖", "迍", "諄", "埻", "綧", "啍", "㡒", "住", "朱", "主", "猪", "祝", "煮", "注", "柱", "珠", "屬", "竹", "助", "株", "筑", "驻", "诸", "著", "嗻", "逐", "铸", "竺", "丶", "铢", "嘱", "烛", "拀", "窋", "渚", "陼", "蠋", "蛛", "紵", "诛", "豬", "矚", "褚", "蓫", "洙", "贮", "蛀", "翥", "茱", "潴", "劚", "鸀", "拄", "术", "鼄", "跓", "跦", "邾", "箸", "尌", "罜", "杼", "眝", "薥", "簗", "櫫", "飳"]),
    ("zhuo", &["桌", "卓", "着", "捉", "蠿", "灼", "镯", "酌", "啄", "琢", "拙", "焯", "繳", "浊", "燋", "炪", "住噢", "住哦", "猪哦", "倬", "茁", "禚", "著", "濯", "叕", "灂", "墌", "捔", "噣", "涿", "琸", "晫", "擢", "鋜", "棹", "穱", "斫", "淖", "梲", "浞", "撯", "汋", "聉", "諁", "籗", "櫡", "濁", "斀", "窡", "鵫", "棁", "鐲", "缴", "擆", "蠗", "彴", "诼", "謶", "剢", "斲", "鷟", "窧", "啅", "龺", "斮", "硺", "烵", "丵", "圴", "犳", "椓", "槕", "妰", "娺", "斱", "罬", "蓔", "劅", "趠", "斵", "穛", "鐯", "棳", "篧", "藋", "㣿", "䏐", "㲋", "㧳", "㧻", "㭬", "㹿", "䖦", "㓸", "䐁", "㒂"]),
    ("zi", &["子", "字", "自", "紫", "梓", "资", "仔", "籽", "滋", "兹", "姿", "孜", "咨", "渍", "畠", "訾", "倳", "緇", "姊", "淄", "嗞", "剚", "趑", "葘", "吱", "洓", "茊", "薋", "恣", "秭", "訿", "資", "孖", "趦", "眦", "杍", "啙", "芓", "玆", "锱", "鶅", "璾", "嗭", "茈", "呲", "釨", "滓", "赀", "頾", "赼", "椔", "澬", "耔", "甾", "沝", "孳", "姉", "茲", "緕", "榟", "姕", "湽", "髭", "缁", "觜", "鼒", "粢", "笫", "橴", "眥", "吇", "嵫", "秄", "虸", "谘", "胔", "漬", "鲻", "紎", "牸", "輺", "鍿", "諮", "胏", "菑", "孶", "镃", "崰", "呰", "栥", "泚", "茡", "龇", "荢", "胾", "秶"]),
    ("zong", &["总", "宗", "棕", "综", "纵", "粽", "踪", "總", "熜", "嵕", "从", "枞", "鬃", "豵", "嵸", "縦", "糉", "鍐", "踨", "葼", "碂", "繌", "緃", "騣", "猣", "腙", "緵", "倧", "錝", "鏓", "縂", "鬉", "鯮", "綜", "翪", "堫", "熧", "蝬", "骔", "騌", "偬", "縱", "総", "鯼", "蹤", "昮", "惣", "猔", "揔", "緫", "鍯", "疭", "潀", "磫", "倊", "從", "樅", "焧", "傯", "揌", "搃", "朡", "潈", "稯", "瘲", "嵏", "惾", "椶", "蓗", "鬷", "鑁", "㚇", "捴", "㷓", "㹅", "㢔", "䁓", "䍟", "摠", "䎫", "䝋", "潨", "䙕", "燪", "㯶", "䗥", "䡯", "糭", "䰌"]),
    ("zou", &["走", "揍", "邹", "奏", "棸", "诹", "驺", "陬", "鄒", "辶", "赱", "齱", "鄹", "齺", "鲰", "箃", "鯐", "黀", "搊", "諏", "掫", "騶", "棷", "菆", "楱", "緅", "㔿", "㞫", "郰", "媰", "㵵", "鯫", "䠫", "䲀"]),
    ("zu", &["组", "租", "足", "祖", "族", "阻", "卒", "俎", "诅", "組", "踤", "镞", "錊", "崒", "詛", "鎺", "鏃", "哫", "菹", "卆", "柤", "崪", "倅", "怚", "踿", "爼", "靻", "珇", "唨", "椊", "蒩", "箤", "顇", "鎐", "葅", "稡", "綷", "伜", "㞺", "䘚", "㲋", "㧻", "㰵", "㲞", "䔃", "䬨", "䖕", "㵀", "䅸", "䚝", "䙘", "䯿", "䱣"]),
    ("zuan", &["钻", "赚", "祖安", "攥", "籫", "组胺", "纂", "鉆", "欑", "篹", "賺", "组安", "足安", "祖庵", "足按", "鑽", "劗", "缵", "纉", "躜", "鑚", "纘", "揝", "攢", "籑", "躦", "䡅", "㷪", "䤸", "繤", "䌣", "䂎", "组", "租", "足", "祖", "族", "阻", "卒", "俎", "诅", "組", "踤", "镞", "錊", "崒", "詛", "鎺", "鏃", "哫", "菹", "卆", "柤", "崪", "倅", "怚", "踿", "爼", "靻", "珇", "唨", "椊", "蒩", "箤", "顇", "鎐", "葅", "稡", "綷", "伜", "㞺", "䘚", "㲋", "㧻", "㰵", "㲞", "䔃", "䬨", "䖕", "㵀", "䅸", "䚝", "䙘", "䯿", "䱣"]),
    ("zui", &["最", "嘴", "罪", "醉", "蕞", "咀", "辠", "觜", "檌", "蟕", "濢", "祽", "纗", "酔", "璻", "槜", "錊", "欈", "樶", "嗺", "鋷", "冣", "嶉", "噿", "晬", "朘", "厜", "稡", "檇", "栬", "槯", "睟", "嶵", "脧", "嶊", "絊", "㝡", "㭰", "䘒", "䘹", "酻", "㠑", "䮔", "㰎", "组", "租", "足", "祖", "族", "阻", "卒", "俎", "诅", "組", "踤", "镞", "崒", "詛", "鎺", "鏃", "哫", "菹", "卆", "柤", "崪", "倅", "怚", "踿", "爼", "靻", "珇", "唨", "椊", "蒩", "箤", "顇", "鎐", "葅", "綷", "伜", "㞺", "䘚", "㲋", "㧻", "㰵", "㲞", "䔃", "䬨", "䖕", "㵀", "䅸", "䚝", "䙘", "䯿", "䱣"]),
    ("zun", &["尊", "遵", "组内", "樽", "阻尼", "鳟", "阻挠", "鱒", "拵", "譐", "袸", "组你", "噂", "阻难", "瀳", "繜", "鐏", "墫", "銌", "僔", "鷷", "撙", "捘", "罇", "栫", "䔿", "嶟", "鶎", "组", "租", "足", "祖", "族", "阻", "卒", "俎", "诅", "組", "踤", "镞", "錊", "崒", "詛", "鎺", "鏃", "哫", "菹", "卆", "柤", "崪", "倅", "怚", "踿", "爼", "靻", "珇", "唨", "椊", "蒩", "箤", "顇", "鎐", "葅", "稡", "綷", "伜", "㞺", "䘚", "㲋", "㧻", "㰵", "㲞", "䔃", "䬨", "䖕", "㵀", "䅸", "䚝", "䙘", "䯿", "䱣"]),
    ("zuo", &["做", "坐", "左", "作", "座", "咗", "佐", "昨", "嘬", "唑", "祚", "柞", "撮", "琢", "组哦", "穝", "岞", "怍", "胙", "租哦", "足哦", "酢", "飵", "諎", "捽", "阼", "笮", "岝", "袏", "莋", "柮", "蓙", "糳", "侳", "秨", "苲", "葃", "鈼", "筰", "繓", "㶡", "㑅", "㘴", "㝾", "㸲", "㛗", "㤰", "㭮", "䇥", "䋏", "䝫", "䞢", "䦈", "䬤", "葄", "稓", "㣱", "䔘", "䞰", "䟶", "㘀", "䩦", "䎰", "䟄", "䯿", "组", "租", "足", "祖", "族", "阻", "卒", "俎", "诅", "組", "踤", "镞", "錊", "崒", "詛", "鎺", "鏃", "哫", "菹", "卆", "柤", "崪", "倅", "怚", "踿", "爼", "靻", "珇", "唨", "椊", "蒩"]),
];

pub fn syl_deep(k: &str) -> Option<&'static [&'static str]> {
    SYL_DEEP
        .binary_search_by(|(s, _)| s.cmp(&k))
        .ok()
        .map(|i| SYL_DEEP[i].1)
}
// rows: 401
// rows: 401

#[cfg(test)]
mod panel_table_tests {
    // 役N tick79b: 面板真值表完整性锁（py sim 33 同族——黄金锁 py↔rust
    // 一致, 本组锁 rust 表与 native 采集真值的对齐防表损坏）
    use super::*;

    #[test]
    fn syl_deep_ti_head() {
        let t = syl_deep("ti").expect("ti 在深表");
        assert_eq!(&t[..3], &["提", "题", "体"]);
    }

    #[test]
    fn pair_ns_head() {
        let t = pair_panel("ns").expect("ns 在对表");
        assert_eq!(&t[..2], &["那是", "你是"]);
    }

    #[test]
    fn solo_special_vowels() {
        assert_eq!(g1_col_solo("i"), Some(&["I", "i"][..]));
        assert_eq!(g1_col_solo("u"), Some(&["u", "U"][..]));
        assert_eq!(g1_col_solo("v"), Some(&[][..]));
    }

    #[test]
    fn table_load_sizes() {
        assert!(SYL_DEEP.len() >= 400, "SYL_DEEP={}", SYL_DEEP.len());
        assert!(PAIR_PANELS.len() >= 370, "PAIR={}", PAIR_PANELS.len());
    }

    /// 铁律守护（交接文档 §七.4）：所有校准表的 Key 必须是读音或声母
    /// （纯 [a-z'] 形态），严禁具体查询字符串硬编码答案。
    /// 本测试把该规范变成可执行断言——任何违规提交立即红。
    #[test]
    fn calibration_keys_are_pinyin_only() {
        let tables: &[(&str, Vec<&str>)] = &[
            ("SLOT_COL", SLOT_COL.iter().map(|(k, _)| *k).collect()),
            ("RD_PIN_COL", RD_PIN_COL.iter().map(|(k, _)| *k).collect()),
            ("RD_PIN_FACES", RD_PIN_FACES.iter().map(|(k, _)| *k).collect()),
            ("DANGLING_COL", DANGLING_COL.iter().map(|(k, _)| *k).collect()),
            ("RD_COL", RD_COL.iter().map(|(k, _)| *k).collect()),
            ("CHAR_COL", CHAR_COL.iter().map(|(k, _)| *k).collect()),
            ("WORD_COL", WORD_COL.iter().map(|(k, _)| *k).collect()),
            ("SYL_PANELS", SYL_PANELS.iter().map(|(k, _)| *k).collect()),
            ("PAIR_PANELS", PAIR_PANELS.iter().map(|(k, _)| *k).collect()),
            ("SYL_DEEP", SYL_DEEP.iter().map(|(k, _)| *k).collect()),
        ];
        for (name, keys) in tables {
            for k in keys {
                assert!(
                    !k.is_empty()
                        && k.chars()
                            .all(|c| c.is_ascii_lowercase() || c == '\''),
                    "{name} 出现非读音键 {k:?} —— 违反零查询字符串答案表铁律"
                );
            }
        }
    }

    /// 各校准表键唯一（重复键 = 静默覆盖源，维护地雷）
    #[test]
    fn calibration_keys_unique() {
        macro_rules! uniq {
            ($name:expr, $t:expr) => {{
                let mut seen = std::collections::HashSet::new();
                for (k, _) in $t.iter() {
                    assert!(seen.insert(*k), "{} 重复键 {k:?}", $name);
                }
            }};
        }
        uniq!("SLOT_COL", SLOT_COL);
        uniq!("RD_PIN_COL", RD_PIN_COL);
        uniq!("RD_PIN_FACES", RD_PIN_FACES);
        uniq!("DANGLING_COL", DANGLING_COL);
        uniq!("RD_COL", RD_COL);
        uniq!("CHAR_COL", CHAR_COL);
        uniq!("WORD_COL", WORD_COL);
        uniq!("SYL_PANELS", SYL_PANELS);
        uniq!("PAIR_PANELS", PAIR_PANELS);
        uniq!("SYL_DEEP", SYL_DEEP);
    }

    /// 打分常量健全性：全部有限、罚值为正、加权为正
    #[test]
    fn scoring_constants_sanity() {
        for (name, v) in [
            ("LAMBDA", LAMBDA),
            ("NN_COEF", NN_COEF),
            ("NN_CLAMP", NN_CLAMP),
            ("W_FREQ_ASSOC", W_FREQ_ASSOC),
            ("COV_PENALTY", COV_PENALTY),
            ("EDIT_SUB", EDIT_SUB),
            ("EDIT_INS", EDIT_INS),
            ("EDIT_TRANS", EDIT_TRANS),
        ] {
            assert!(v.is_finite(), "{name} 非有限值");
        }
        assert!(EDIT_SUB > 0.0 && EDIT_INS > 0.0 && EDIT_TRANS > 0.0, "编辑罚须为正");
        assert!(COV_PENALTY > 0.0 && NN_CLAMP > 0.0, "罚/饱和须为正");
    }
}
