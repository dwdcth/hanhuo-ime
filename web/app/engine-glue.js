// engine-glue.js — 真引擎桥接（window.Hanhuo 契约适配器）
//
// 把 wasm-bindgen 的 ES 模块（../hanhuo_wasm.js: init/init_nnlm/
// enable_boost/query）适配成 app 契约:
//   await Hanhuo.init(dictBytes, opts?) -> {ok, error?}
//     opts.nnlmBytes — NNLM 模型字节（mobile/advanced 变体路由）;
//       注入成功即开宽档 boost（2026-11-20 升默认口径: 尾槽+B7 句
//       重排+beam150——注入即产品默认, 同 native --boost-sent-nn）
//     opts.mdltrBytes — 预留（mdltr 排序接线未破, 忽略）
//     opts.enBytes — 英文词表 tsv 字节（四小件英文通道; 纯字母输入
//       候选见 Hanhuo.enCandidates）
//     opts.enlmBytes — 英文 LM safetensors 字节（enlm_v2; 注入后
//       enCandidates 自动切 LM 语境序 P(w|BOS)——CLI --enlm 同律）
//   Hanhuo.query(input, topk?, ctx?) -> Array<{word, score, kind, reading}>
//     ctx — 已上屏文本（引擎自取尾段: B3-d 搭配钉序[时间+轴→轴#1]、
//     联想 bigram、B3 语境序全消费; 空串=无语境）
//   Hanhuo.wasmBytes -> Uint8Array（fetch 本 .wasm 供 UI 展示）
//
// 挂载后 window.Hanhuo 即 app 探测目标（engine-adapter GLUE 候选之一）。

(async () => {
  const mod = await import('../hanhuo_wasm.js');
  await mod.default(); // wasm 实例化

  let nWords = 0;

  window.Hanhuo = {
    async init(dictBytes, opts = {}) {
      try {
        const bytes = dictBytes instanceof Uint8Array
          ? dictBytes : new Uint8Array(dictBytes);
        nWords = mod.init(bytes);
        if (!nWords) return { ok: false, error: 'init 返回 0 词条' };
        if (opts.enBytes) {
          const eb = opts.enBytes instanceof Uint8Array
            ? opts.enBytes : new Uint8Array(opts.enBytes);
          mod.init_en(eb);
        }
        if (opts.enlmBytes) {
          const lb = opts.enlmBytes instanceof Uint8Array
            ? opts.enlmBytes : new Uint8Array(opts.enlmBytes);
          mod.init_enlm(lb);
        }
        if (opts.lmcnUni && opts.lmcnBg) {
          const u8 = opts.lmcnUni instanceof Uint8Array ? opts.lmcnUni : new Uint8Array(opts.lmcnUni);
          const g8 = opts.lmcnBg instanceof Uint8Array ? opts.lmcnBg : new Uint8Array(opts.lmcnBg);
          if (!mod.init_lmcn(u8, g8)) console.warn('[glue] lmcn 注入失败');
        }
        if (opts.nnlmBytes) {
          const nb = opts.nnlmBytes instanceof Uint8Array
            ? opts.nnlmBytes : new Uint8Array(opts.nnlmBytes);
          if (mod.init_nnlm(nb) && mod.enable_boost()) {
            console.log('[glue] NNLM 注入 + 宽档 boost 已开');
          } else {
            console.warn('[glue] NNLM 装载失败, 忠实模式运行');
          }
        }
        return { ok: true };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    query(input, topk = 10, ctx = '') {
      if (!nWords) return [];
      const r = JSON.parse(mod.query(input, String(ctx || ''), topk));
      return (r.cands || []).map(c => ({
        // reading 透传（拼音分段高亮用——消费键数 = 全拼字母数 或 简拼
        // 音节数, app.renderPreedit 双态探测）
        word: c.word, score: c.score, kind: c.kind,
        reading: c.reading || '',
      }));
    },

    // T9 九宫模式开关（纯 2-9 数字键串→九宫展开; CLI --t9 同律）
    enableT9(on = true) {
      try { return !!mod.enable_t9(!!on); } catch { return false; }
    },

    // 英文候选（纯字母输入; 精确词优先+前缀补全频序, CLI --en 同律）
    enCandidates(input, topk = 8) {
      try {
        const r = JSON.parse(mod.en_candidates(input, topk));
        return (r.cands || []).map(c => ({
          word: c.word, score: -c.rank / 1000, kind: 'en', reading: '',
        }));
      } catch { return []; }
    },

    get wasmBytes() {
      return undefined; // 惰性: UI 需要展示时再 fetch
    },
  };
})();
