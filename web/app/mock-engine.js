// mock-engine.js — 内置 Mock 引擎（假数据）
//
// 用途：引擎侧 glue（window.Hanhuo）尚未部署时，演示完整交互链路。
// 严格实现引擎契约的三个成员（init / query / wasmBytes），可直接喂给 engine-adapter。
//
// kind 约定（演示用）：0=单字  1=词  2=短语  3=拼句(逐音节拼合)
// 数据纯属手写假数据，仅覆盖演示样例与常见音节，不代表真实词库质量。

// ---- 精确词条（整串拼音 -> 候选 [word, score, kind, reading]）----
const WORDS = {
  nihao: [
    ['你好', 0.99, 1, 'ni hao'],
    ['您好', 0.95, 1, 'nin hao'],
    ['你好吗', 0.90, 2, 'ni hao ma'],
  ],
  jintian: [
    ['今天', 0.98, 1, 'jin tian'],
    ['金田', 0.78, 1, 'jin tian'],
  ],
  tianqi: [
    ['天气', 0.98, 1, 'tian qi'],
    ['天启', 0.84, 1, 'tian qi'],
    ['田七', 0.79, 1, 'tian qi'],
  ],
  bucuo: [['不错', 0.97, 1, 'bu cuo']],
  mulu: [
    ['目录', 0.98, 1, 'mu lu'],
    ['木路', 0.72, 1, 'mu lu'],
  ],
  jiegou: [
    ['结构', 0.98, 1, 'jie gou'],
    ['结果', 0.96, 1, 'jie guo'],
  ],
  mulujiegou: [['目录结构', 0.97, 2, 'mu lu jie gou']],
  hanhuo: [['汉火', 0.99, 1, 'han huo']],
  shurufa: [['输入法', 0.99, 1, 'shu ru fa']],
  wangzhan: [['网站', 0.98, 1, 'wang zhan']],
  wangye: [['网页', 0.97, 1, 'wang ye']],
  yanshi: [['演示', 0.97, 1, 'yan shi']],
  cidian: [['词典', 0.97, 1, 'ci dian']],
  huancun: [['缓存', 0.96, 1, 'huan cun']],
  yinqing: [['引擎', 0.97, 1, 'yin qing']],
  ceshi: [['测试', 0.97, 1, 'ce shi']],
  xiexie: [['谢谢', 0.98, 1, 'xie xie']],
  zhongguo: [['中国', 0.99, 1, 'zhong guo']],
  xuexi: [['学习', 0.97, 1, 'xue xi']],
};

// ---- 音节 -> 候选单字表（拼句回退用，第 k 个字用于第 k 个变体）----
const SYL_CHARS = {
  a: '啊阿呀', ai: '爱哎矮', an: '安按暗', ang: '昂', ao: '奥熬袄',
  ba: '把八吧', bai: '白百摆', ban: '办半班', bang: '帮绑棒', bao: '报包保',
  bei: '被北背', ben: '本奔', beng: '蹦崩', bi: '比笔必', bian: '变边便',
  biao: '表标', bie: '别', bin: '宾', bing: '并冰病', bo: '波播伯',
  bu: '不布部', ca: '擦', cai: '才菜采', can: '参残', cang: '藏苍',
  cao: '草操', ce: '测策', cen: '岑', cha: '查茶差', chai: '拆柴',
  chan: '产缠', chang: '长常厂', chao: '超朝吵', che: '车扯', chen: '陈沉晨',
  cheng: '成城程', chi: '吃迟持', chong: '冲充虫', chou: '抽愁丑', chu: '出处初',
  chuan: '传船穿', chuang: '窗床创', chui: '吹垂', chun: '春纯', chuo: '戳',
  ci: '次此词', cong: '从匆', cou: '凑', cu: '粗促', cuan: '窜',
  cui: '翠催脆', cun: '村存', cuo: '错措挫', da: '大打达', dai: '带代待',
  dan: '但单蛋', dang: '当党挡', dao: '到道路', de: '的得', dei: '得',
  deng: '等灯登', di: '的地底', dian: '点电店', diao: '掉调吊', die: '跌叠',
  ding: '定顶订', diu: '丢', dong: '动东懂', dou: '都斗豆', du: '读度独',
  duan: '断段短', dui: '对队', dun: '顿盾', duo: '多朵躲', e: '额恶俄',
  ei: '诶', en: '恩', er: '而二儿', fa: '发法', fan: '反烦返',
  fang: '放方房', fei: '非飞费', fen: '分份粉', feng: '风丰封', fo: '佛',
  fou: '否', fu: '复付服', ga: '嘎', gai: '改盖该', gan: '感甘干',
  gang: '刚钢港', gao: '高搞告', ge: '个各格', gei: '给', gen: '跟根',
  geng: '更耕', gong: '公共工', gou: '够构购', gu: '古故顾', gua: '刮瓜挂',
  guai: '怪乖', guan: '关管观', guang: '光广逛', gui: '贵规鬼', gun: '滚棍',
  guo: '国过果', ha: '哈', hai: '还海害', han: '汉含喊', hang: '行航',
  hao: '好号毫', he: '和合喝', hei: '黑嘿', hen: '很恨', heng: '横衡',
  hong: '红洪宏', hou: '后候厚', hu: '互湖呼', hua: '话化花', huai: '坏怀',
  huan: '换还环', huang: '黄晃荒', hui: '会回汇', hun: '混婚魂', huo: '火活或',
  ji: '级机几', jia: '家加价', jian: '见间建', jiang: '将讲降', jiao: '叫教交',
  jie: '结接街', jin: '今金进', jing: '京精经', jiong: '窘', jiu: '就久旧',
  ju: '具据句', juan: '卷倦', jue: '觉绝', jun: '军均', ka: '卡',
  kai: '开凯', kan: '看砍刊', kang: '抗康', kao: '考靠', ke: '可课客',
  ken: '肯啃', keng: '坑', kong: '空控', kou: '口扣', ku: '苦库哭',
  kua: '夸垮跨', kuai: '快块', kuan: '宽款', kuang: '况矿框', kui: '愧溃葵',
  kun: '困昆', kuo: '扩括', la: '拉啦蜡', lai: '来赖', lan: '蓝兰栏',
  lang: '朗狼', lao: '老劳牢', le: '了乐', lei: '类累雷', leng: '冷愣',
  li: '里理力', lia: '俩', lian: '连脸练', liang: '两亮量', liao: '料聊了',
  lie: '列烈裂', lin: '林临邻', ling: '领令另', liu: '六留刘', long: '龙笼聋',
  lou: '楼漏', lu: '路录露', luan: '乱', lun: '论轮', luo: '罗落洛',
  ma: '吗妈马', mai: '买卖迈', man: '慢满漫', mang: '忙芒盲', mao: '猫毛冒',
  me: '么', mei: '没美每', men: '们门闷', meng: '梦猛蒙', mi: '米密蜜',
  mian: '面免棉', miao: '妙描秒', mie: '灭', min: '民敏', ming: '名明命',
  mo: '莫模磨', mou: '某谋', mu: '木目母', na: '那拿哪', nai: '奶耐乃',
  nan: '南难男', nao: '闹脑恼', ne: '呢讷', nei: '内', nen: '嫩',
  neng: '能', ni: '你尼泥', nian: '年念', niang: '娘', niao: '鸟尿',
  nie: '捏聂', nin: '您', ning: '宁凝', niu: '牛纽扭', nong: '农浓弄',
  nu: '怒奴', nuan: '暖', nuo: '诺挪', o: '哦噢', ou: '欧偶呕',
  pa: '怕爬帕', pai: '排拍牌', pan: '判盘盼', pang: '旁胖庞', pao: '跑炮泡',
  pei: '配陪佩', pen: '盆喷', peng: '朋碰捧', pi: '批皮疲', pian: '片篇骗',
  piao: '票漂飘', pie: '撇瞥', pin: '品贫拼', ping: '平评瓶', po: '破泼婆',
  pou: '剖', pu: '普铺扑', qi: '气起七', qia: '恰卡', qian: '前千钱',
  qiang: '强枪墙', qiao: '桥悄巧', qie: '切且窃', qin: '亲勤琴', qing: '清请轻',
  qiong: '穷', qiu: '求球秋', qu: '去取趣', quan: '全权劝', que: '确缺却',
  qun: '群裙', ran: '然染', rang: '让嚷', rao: '绕扰', re: '热惹',
  ren: '人认任', reng: '扔仍', ri: '日', rong: '容荣融', rou: '肉柔',
  ru: '如入乳', ruan: '软', rui: '瑞锐', run: '润闰', ruo: '若弱',
  sa: '撒洒', sai: '赛塞', san: '三散伞', sang: '桑嗓', sao: '扫嫂',
  se: '色涩', sen: '森', seng: '僧', sha: '杀沙傻', shai: '筛晒',
  shan: '山善闪', shang: '上商伤', shao: '少烧绍', she: '设社射', shei: '谁',
  shen: '什深神身', sheng: '生声升', shi: '是时十事', shou: '手首受',
  shu: '书数树输', shua: '刷耍', shuai: '帅甩率', shuan: '栓拴', shuang: '双爽霜',
  shui: '水谁睡', shun: '顺瞬', shuo: '说朔硕', si: '四思死司', song: '送松宋',
  sou: '搜艘', su: '速素苏', suan: '算蒜酸', sui: '随虽岁', sun: '损孙笋',
  suo: '所锁索', ta: '他她它', tai: '太台抬', tan: '谈探弹', tang: '堂糖汤',
  tao: '逃桃讨', te: '特', teng: '疼腾藤', ti: '提题体', tian: '天田添',
  tiao: '条跳调', tie: '铁贴帖', ting: '听停厅', tong: '同通统', tou: '头投透',
  tu: '图土突', tuan: '团', tui: '退推腿', tun: '吞屯', tuo: '脱拖托',
  wa: '挖哇蛙', wai: '外歪', wan: '完万晚', wang: '王望往网', wei: '为位围维',
  wen: '问文闻温', weng: '翁嗡', wo: '我握窝', wu: '五无物误', xi: '系西细希',
  xia: '下夏吓', xian: '先现线显', xiang: '想向象相', xiao: '小笑消校',
  xie: '写些谢解', xin: '新心信', xing: '行星性形', xiong: '兄雄熊',
  xiu: '修休秀', xu: '需许续', xuan: '选旋宣', xue: '学雪血', xun: '寻训巡',
  ya: '呀压牙雅', yan: '眼言严研演', yang: '样央洋阳', yao: '要药摇遥',
  ye: '也夜叶业', yi: '一以已意', yin: '因音引银', ying: '应影英迎',
  yo: '哟', yong: '用永拥勇', you: '有又由友', yu: '于与育域语',
  yuan: '元员远院', yue: '月越约跃', yun: '云运允韵', za: '杂砸咱',
  zai: '在再载', zan: '咱暂赞', zang: '脏葬赃', zao: '早造遭', ze: '则责择泽',
  zei: '贼', zen: '怎', zeng: '增赠', zha: '炸渣扎', zhai: '窄宅摘',
  zhan: '站战占展', zhang: '张章涨帐', zhao: '找着照招', zhe: '这者着浙',
  zhen: '真镇阵震', zheng: '正整争政', zhi: '只知之制直', zhong: '中种重众终',
  zhou: '周州洲舟', zhu: '主住注助', zhua: '抓爪', zhuai: '拽', zhuan: '转专砖赚',
  zhuang: '装壮状庄', zhui: '追坠缀', zhun: '准', zhuo: '桌捉拙浊', zi: '子自字资',
  zong: '总纵综', zou: '走奏', zu: '组足族阻', zuan: '钻', zui: '最嘴醉罪',
  zun: '尊遵', zuo: '做作坐座',
};

// 合法音节集合 = 有字表的音节 + 少量无字表但有码的音节（回退为原样输出）
const NULL_SYLLABLES = new Set(['den', 'dia', 'eng', 'lo', 'nia', 'nue', 'rua', 'tei', 'lve', 'nve']);
const SYLLABLES = new Set([...Object.keys(SYL_CHARS), ...NULL_SYLLABLES]);
const MAX_SYL_LEN = 6;

/** 贪心最长匹配拼音分词（演示级） */
export function segmentPinyin(input) {
  const out = [];
  let i = 0;
  while (i < input.length) {
    let matched = '';
    for (let len = Math.min(MAX_SYL_LEN, input.length - i); len >= 1; len--) {
      const s = input.slice(i, i + len);
      if (SYLLABLES.has(s)) { matched = s; break; }
    }
    if (matched) { out.push(matched); i += matched.length; }
    else { out.push(input[i]); i += 1; } // 非法字符原样占位
  }
  return out;
}

/** 单音节候选字；未收录音节回退为音节原样输出（mock 透明性） */
function sylCandidates(syl) {
  const entry = SYL_CHARS[syl];
  if (!entry) return [syl];
  return Array.from(entry);
}

/**
 * Mock 引擎，实现 Hanhuo 契约：
 *   await init(dictBytes, opts?) -> {ok:true}
 *   query(input, topk?) -> [{word, score, kind, reading}]
 *   wasmBytes: Uint8Array
 */
export class MockEngine {
  constructor() {
    this.wasmBytes = new Uint8Array(8); // 占位：mock 无真实 wasm
    this.initMs = 0;
    this.dictBytes = 0;
  }

  async init(dictBytes) {
    const t0 = Date.now();
    if (dictBytes) {
      this.dictBytes = dictBytes.byteLength != null ? dictBytes.byteLength : dictBytes.length || 0;
    }
    await new Promise((r) => setTimeout(r, 30)); // 模拟初始化耗时
    this.initMs = Date.now() - t0;
    return { ok: true };
  }

  query(input, topk = 10) {
    const raw = String(input == null ? '' : input).toLowerCase().replace(/[^a-z]/g, '');
    if (!raw) return [];
    const out = [];
    const seen = new Set();
    const push = (word, score, kind, reading) => {
      if (seen.has(word)) return;
      seen.add(word);
      out.push({ word, score, kind, reading });
    };

    // 1) 精确词条
    const exact = WORDS[raw];
    if (exact) for (const [w, s, k, r] of exact) push(w, s, k, r);

    // 2) 逐音节拼合（贪心分词 + 每音节取第 k 字做变体）
    const syls = segmentPinyin(raw);
    const variants = Math.max(1, Math.min(4, topk - out.length));
    for (let v = 0; v < variants; v++) {
      let word = '';
      let ok = true;
      for (const s of syls) {
        const cs = sylCandidates(s);
        const c = cs[Math.min(v, cs.length - 1)];
        if (!c) { ok = false; break; }
        word += c;
      }
      if (ok && word) {
        const kind = syls.length > 2 ? 3 : syls.length === 2 ? 1 : 0;
        const score = Math.max(0.05, 0.92 - v * 0.06 - (exact ? 0.05 : 0));
        push(word, Number(score.toFixed(3)), kind, syls.join(' '));
      }
    }

    // 3) 词条前缀联想（输入是某词条键的前缀）
    if (out.length < topk) {
      for (const key of Object.keys(WORDS)) {
        if (key !== raw && key.startsWith(raw)) {
          const [w, s, k, r] = WORDS[key][0];
          push(w, Math.min(0.9, s - 0.1), k, r);
          if (out.length >= topk) break;
        }
      }
    }

    out.sort((a, b) => b.score - a.score);
    return out.slice(0, topk);
  }
}
