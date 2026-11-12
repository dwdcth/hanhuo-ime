// engine-adapter.js — 引擎适配器（UI 与引擎之间的唯一边界）
//
// 引擎契约（与引擎侧 glue 的约定，不可改）：
//   await Hanhuo.init(dictBytes: ArrayBuffer|Uint8Array, opts?: {nnlmBytes?, mdltrBytes?}) -> {ok, error?}
//   Hanhuo.query(input: string, topk?: number), ctx?: string) -> Array<{word, score, kind, reading}>
//   Hanhuo.wasmBytes: Uint8Array
//
// 职责：
//   1. 探测/加载引擎 glue 脚本（注入 <script>，等待 window.Hanhuo）
//   2. 按契约取词典字节（fetch 带进度 + IndexedDB cache-first），可选 nnlm/mdltr
//   3. 调 Hanhuo.init；全部状态经 onStatus 回调上报 UI
//   4. 引擎不可用（文件缺失/初始化失败）→ 自动降级 MockEngine，演示链路始终可用
//
// URL 参数（部署/联调可覆盖默认路径）：
//   ?engine=<glue.js 相对路径>   只探测该 glue，跳过候选探测
//   ?dict=<wxq_dict.bin 路径>    词典路径（默认 ./wxq_dict.bin）
//   ?nnlm=  ?mdltr=              可选模型路径
//   ?forceRefresh=1              跳过 IDB 缓存重新下载并覆盖缓存
//   ?noMock=1                    引擎失败时不降级 mock（保持错误状态，便于联调）

import { MockEngine } from './mock-engine.js';
import { loadAsset, urlExists, fmtBytes, assetCache } from './assets.js';

export const GLUE_CANDIDATES = ['./engine-glue.js', './engine.js', './glue.js', './hanhuo.js'];

const DEFAULT_DICT_URL = './wxq_dict.bin';
const DEFAULT_NNLM_URL = './nnlm.safetensors';
const DEFAULT_MDLTR_URL = './mdltr.safetensors';
const DEFAULT_EN_URL = './en_open.tsv';
const DEFAULT_ENLM_URL = './enlm.safetensors';

function queryParams() {
  try {
    if (typeof location !== 'undefined' && location.search) return new URLSearchParams(location.search);
  } catch { /* node */ }
  return new URLSearchParams('');
}

function isBrowser() {
  return typeof document !== 'undefined' && typeof window !== 'undefined';
}

/** 注入 classic script 并等待 onload/onerror */
function injectScript(src) {
  return new Promise((resolve, reject) => {
    if (!isBrowser()) return reject(new Error('非浏览器环境，无法注入 glue 脚本'));
    const s = document.createElement('script');
    s.src = src;
    s.async = true;
    s.onload = () => resolve(src);
    s.onerror = () => reject(new Error(`glue 脚本加载失败: ${src}`));
    document.head.appendChild(s);
  });
}

/** 轮询等待 window.Hanhuo 出现（glue 可能异步初始化） */
function waitForHanhuo(timeoutMs = 8000) {
  return new Promise((resolve, reject) => {
    if (typeof globalThis.Hanhuo !== 'undefined') return resolve(globalThis.Hanhuo);
    const t0 = Date.now();
    const timer = setInterval(() => {
      if (typeof globalThis.Hanhuo !== 'undefined') {
        clearInterval(timer);
        resolve(globalThis.Hanhuo);
      } else if (Date.now() - t0 > timeoutMs) {
        clearInterval(timer);
        reject(new Error('glue 已加载但未提供 window.Hanhuo（超时 8s）'));
      }
    }, 50);
  });
}

export class EngineAdapter {
  /**
   * @param {{engineObject?: object}} ctorOpts 主要供测试/嵌入：直接注入一个满足契约的引擎对象
   */
  constructor({ engineObject } = {}) {
    this.engine = null;        // 当前实际引擎（真引擎或 MockEngine）
    this.mode = 'idle';        // idle | engine | mock
    this.loadError = null;     // 引擎失败原因（降级 mock 时保留展示）
    this.info = {};            // {dictSize, nnlmSize, mdltrSize, wasmBytes, initMs, dictFromCache, dictUrl}
    this._loading = false;
    this._engineObject = engineObject || null;
    this.onStatus = () => {};  // UI 注入：(status) => void
  }

  _emit(phase, message, extra = {}) {
    this.onStatus({ phase, message, mode: this.mode, loadError: this.loadError, ...extra });
  }

  get ready() {
    return this.engine != null;
  }

  /**
   * 完整加载流程。幂等保护：进行中时重复调用返回同一个 Promise。
   * @param {{forceRefresh?: boolean, noMock?: boolean}} opts noMock: 失败不降级 mock（联调用）
   * @returns {Promise<{mode:'engine'|'mock'}>}
   */
  async load(opts = {}) {
    if (this._loading) return this._loadingPromise;
    this._loading = true;
    this._loadingPromise = this._loadInner(opts).finally(() => { this._loading = false; });
    return this._loadingPromise;
  }

  async _loadInner({ forceRefresh, noMock } = {}) {
    const params = queryParams();
    forceRefresh = forceRefresh || params.get('forceRefresh') === '1';
    noMock = noMock || params.get('noMock') === '1';
    const dictUrl = params.get('dict') || DEFAULT_DICT_URL;
    const t0 = Date.now();

    try {
      // ---- 1. 拿到引擎对象 ----
      let engine = null;
      if (this._engineObject) {
        engine = this._engineObject;
      } else if (typeof globalThis.Hanhuo !== 'undefined') {
        engine = globalThis.Hanhuo; // 页面自带（引擎侧联调页）
      } else {
        const glueUrl = params.get('engine') || (await this._probeGlue(params));
        if (!glueUrl) throw new Error(`未找到引擎 glue 脚本（探测了 ${GLUE_CANDIDATES.join(' / ')}）`);
        this._emit('probe', `加载引擎 glue：${glueUrl}`);
        await injectScript(glueUrl);
        engine = await waitForHanhuo();
      }
      if (!engine || typeof engine.init !== 'function' || typeof engine.query !== 'function') {
        throw new Error('引擎对象不满足契约（缺 init/query）');
      }

      // ---- 2. 词典字节（进度 + 缓存） ----
      this._emit('fetch', '获取词典…', { progress: { loaded: 0, total: 0 } });
      const dict = await loadAsset(dictUrl, {
        forceRefresh,
        onProgress: (loaded, total) =>
          this._emit('fetch', `下载词典 ${total ? Math.round((loaded / total) * 100) + '%' : fmtBytes(loaded)}`,
            { progress: { loaded, total } }),
      });
      if (!dict.bytes || dict.bytes.byteLength === 0) throw new Error(`词典为空：${dictUrl}`);

      // ---- 3. 可选模型（存在才带上） ----
      const initOpts = {};
      for (const [key, url, label] of [
        ['nnlmBytes', params.get('nnlm') || DEFAULT_NNLM_URL, 'NNLM'],
        ['mdltrBytes', params.get('mdltr') || DEFAULT_MDLTR_URL, 'mdltr'],
        ['enBytes', params.get('en') || DEFAULT_EN_URL, 'en词表'],
        ['enlmBytes', params.get('enlm') || DEFAULT_ENLM_URL, 'enlm'],
      ]) {
        if (params.get({ nnlmBytes: 'nnlm', mdltrBytes: 'mdltr', enBytes: 'en', enlmBytes: 'enlm' }[key]) === '0') continue; // 显式关闭
        if (!(await urlExists(url))) continue;
        const m = await loadAsset(url, {
          forceRefresh,
          onProgress: (loaded, total) =>
            this._emit('fetch', `下载${label} ${total ? Math.round((loaded / total) * 100) + '%' : fmtBytes(loaded)}`,
              { progress: { loaded, total } }),
        });
        initOpts[key] = m.bytes.buffer;
        const sizeKey = { nnlmBytes: 'nnlmSize', mdltrBytes: 'mdltrSize', enBytes: 'enSize', enlmBytes: 'enlmSize' }[key];
        this.info[sizeKey] = m.bytes.byteLength;
      }

      // ---- 4. 初始化 ----
      this._emit('init', '引擎初始化…');
      const ret = await engine.init(dict.bytes.buffer, initOpts);
      if (!ret || ret.ok !== true) throw new Error(`init 失败: ${(ret && ret.error) || '未知错误'}`);

      this.engine = engine;
      this.mode = 'engine';
      this.loadError = null;
      this.info = {
        ...this.info,
        dictUrl,
        dictSize: dict.bytes.byteLength,
        dictFromCache: dict.fromCache,
        wasmBytes: engine.wasmBytes ? engine.wasmBytes.byteLength : 0,
        initMs: Date.now() - t0,
        loadMs: Date.now() - t0,
      };
      this._emit('ready', `引擎就绪 · 词典 ${fmtBytes(this.info.dictSize)}${dict.fromCache ? '（缓存命中）' : ''} · 初始化 ${this.info.initMs}ms`);
      return { mode: 'engine' };
    } catch (err) {
      this.loadError = err && err.message ? err.message : String(err);
      if (noMock) {
        this.engine = null;
        this.mode = 'error';
        this._emit('error', `引擎加载失败：${this.loadError}`);
        throw err;
      }
      // ---- 降级 mock，保证演示链路可用 ----
      const mock = new MockEngine();
      await mock.init(null);
      this.engine = mock;
      this.mode = 'mock';
      this._emit('mock', `引擎不可用（${this.loadError}）→ Mock 演示模式（假数据）`);
      return { mode: 'mock' };
    }
  }

  /** 探测 glue 候选（fetch 200 即命中），?engine= 指定时只探测它 */
  async _probeGlue(params) {
    const custom = params.get('engine');
    const candidates = custom ? [custom] : GLUE_CANDIDATES;
    this._emit('probe', custom ? `探测指定 glue：${custom}` : '探测引擎 glue 脚本…');
    for (const url of candidates) {
      try {
        const res = await fetch(url, { method: 'GET', cache: 'no-store' });
        // 即使 404 也可能返回错误页 HTML，只有 ok 才算
        if (res.ok) {
          const text = await res.text();
          if (text && text.length > 0) return url;
        }
      } catch { /* fetch 失败（file:// 等）继续 */ }
    }
    return null;
  }

  /**
   * 查询候选（契约透传）。引擎未就绪时返回 []。
   * @returns {Promise<Array<{word,score,kind,reading}>>}
   */
  /** 英文候选（纯字母输入; 引擎未注入 en 表时返回 []）。 */
  async enCandidates(input, topk = 8) {
    if (!this.engine) return [];
    try {
      const r = this.engine.enCandidates
        ? await this.engine.enCandidates(input, topk) : [];
      return Array.isArray(r) ? r : [];
    } catch { return []; }
  }

  async query(input, topk = 10, ctx = '') {
    if (!this.engine) return [];
    try {
      const r = await this.engine.query(input, topk, ctx);
      return Array.isArray(r) ? r : [];
    } catch (err) {
      this._emit('error', `查询失败：${err && err.message ? err.message : err}`);
      return [];
    }
  }

  /** 清空资产缓存（设置保留） */
  async clearAssetCache() {
    return assetCache.clear();
  }
}
