// app.js — 页面装配：输入 → 候选 → 翻页 → 上屏 交互链路 + 设置面 + 引擎状态栏 + 性能显示
import { EngineAdapter } from './engine-adapter.js';
import { Composer } from './composer.js';
import { loadSettings, saveSettings, storageEstimate, requestPersistence } from './storage.js';
import { fmtBytes, assetCache } from './assets.js';

const assetCacheEntries = () => assetCache.entries();

const $ = (sel) => document.querySelector(sel);

const els = {
  statusPill: $('#status-pill'),
  statusMsg: $('#status-msg'),
  progressBar: $('#progress-bar'),
  progressFill: $('#progress-fill'),
  perf: $('#perf'),
  committed: $('#committed'),
  committedCount: $('#committed-count'),
  preedit: $('#preedit'),
  pinyin: $('#pinyin'),
  candidates: $('#candidates'),
  candPager: $('#cand-pager'),
  pagePrev: $('#page-prev'),
  pageNext: $('#page-next'),
  pageInd: $('#page-ind'),
  latency: $('#latency'),
  engineInfo: $('#engine-info'),
  clearCache: $('#clear-cache'),
  retry: $('#retry-engine'),
  settingsBtn: $('#settings-btn'),
  settingsPanel: $('#settings-panel'),
  topkRange: $('#topk-range'),
  topkVal: $('#topk-val'),
  t9Toggle: $('#t9-toggle'),
  themeSeg: $('#theme-seg'),
  examples: $('#examples'),
  cacheInfo: $('#cache-info'),
};

// ---------- 设置（topk 每页候选 / 主题） ----------

const POOL_PAGES = 3; // 翻页候选池 = 每页 × 3 页（一次查询取回，本地翻页）

function clampTopk(v) {
  const n = Math.round(Number(v));
  if (!Number.isFinite(n)) return 10;
  return Math.min(20, Math.max(5, n));
}

const settings = loadSettings({ topk: 10, theme: 'auto', t9: false });
let pageSize = clampTopk(settings.topk);

function applyThemeSetting() {
  const theme = settings.theme === 'light' || settings.theme === 'dark' ? settings.theme : 'auto';
  if (theme === 'auto') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
  for (const b of els.themeSeg.querySelectorAll('button')) {
    const on = b.dataset.themeOpt === theme;
    b.classList.toggle('active', on);
    b.setAttribute('aria-checked', String(on));
  }
}

const adapter = new EngineAdapter();
const composer = new Composer();
let currentCandidates = [];
let pageIdx = 0;         // 候选当前页（0 起，每页 pageSize 条）
let committedCaret = 0;  // 上屏区插入点（Unicode 码点位置，默认句尾）
let querySeq = 0;      // 竞态防护：慢查询回来时若输入已变则丢弃
let queryTimer = null;
let lastQueryMs = null;
const latencyRing = [];  // 最近 10 次查询延迟（ms）

// ---------- 状态栏 ----------

const PILL_TEXT = {
  loading: '引擎加载中',
  ready: '引擎就绪',
  mock: 'MOCK 演示',
  error: '引擎不可用',
};

function renderStatus(st) {
  const map = { probe: 'loading', fetch: 'loading', init: 'loading', ready: 'ready', mock: 'mock', error: 'error' };
  const cls = map[st.phase] || 'loading';
  els.statusPill.className = `pill ${cls}`;
  els.statusPill.textContent = PILL_TEXT[cls];
  els.statusMsg.textContent = st.message || '';
  const p = st.progress;
  if (p && p.total > 0) {
    els.progressBar.style.display = 'block';
    els.progressFill.style.width = `${Math.min(100, (p.loaded / p.total) * 100).toFixed(1)}%`;
  } else if (cls === 'loading') {
    els.progressBar.style.display = p ? 'block' : 'none';
    els.progressFill.style.width = p ? '100%' : '0%';
  } else {
    els.progressBar.style.display = 'none';
  }
  els.retry.hidden = !(cls === 'mock' || cls === 'error');
  if (st.phase === 'ready' || st.phase === 'mock') {
    // 引擎就绪重放 T9 开关（toggle 早于装载拨动时生效）
    if (settings.t9) applyT9(true);
    renderEngineInfo();
    updateCacheInfo();
  }
}

function renderEngineInfo() {
  const i = adapter.info || {};
  const parts = [];
  if (i.dictSize) parts.push(`词典 ${fmtBytes(i.dictSize)}${i.dictFromCache ? ' · 缓存命中' : ' · 网络'}`);
  if (i.wasmBytes) parts.push(`wasm ${fmtBytes(i.wasmBytes)}`);
  if (i.nnlmSize) parts.push(`nnlm ${fmtBytes(i.nnlmSize)}`);
  if (i.mdltrSize) parts.push(`mdltr ${fmtBytes(i.mdltrSize)}`);
  if (i.initMs != null) parts.push(`初始化 ${i.initMs}ms`);
  els.engineInfo.textContent = parts.join(' · ') || (adapter.mode === 'mock' ? 'mock 引擎（无真实资产）' : '');
}

async function updateCacheInfo() {
  const est = await storageEstimate();
  const list = await assetCacheEntries();
  const cachedTotal = list.reduce((s, e) => s + e.size, 0);
  let text = `本地缓存 ${list.length} 项 / ${fmtBytes(cachedTotal)}`;
  if (est.supported && est.quota > 0) {
    text += ` · 站点存储 ${fmtBytes(est.usage)}/${fmtBytes(est.quota)}`;
  }
  els.cacheInfo.textContent = text;
}

// ---------- 性能显示（最近 10 次查询延迟 mean/p95） ----------

function recordLatency(ms) {
  latencyRing.push(ms);
  if (latencyRing.length > 10) latencyRing.shift();
  const n = latencyRing.length;
  const mean = latencyRing.reduce((a, b) => a + b, 0) / n;
  const sorted = [...latencyRing].sort((a, b) => a - b);
  const p95 = sorted[Math.max(0, Math.ceil(0.95 * n) - 1)];
  els.perf.textContent = `查询延迟 · n${n} mean ${mean.toFixed(1)}ms / p95 ${p95.toFixed(1)}ms`;
}

// ---------- 上屏文本区（插入点定位 + 字数） ----------

function renderCommitted() {
  const cps = Array.from(composer.committed);
  if (committedCaret < 0 || committedCaret > cps.length) committedCaret = cps.length;
  els.committed.textContent = cps.slice(0, committedCaret).join('');
  if (cps.length) {
    const mark = document.createElement('span');
    mark.className = 'caret-marker';
    els.committed.appendChild(mark);
    els.committed.appendChild(document.createTextNode(cps.slice(committedCaret).join('')));
  }
  els.committedCount.textContent = cps.length ? `${cps.length} 字` : '';
}

/** 点击上屏区 → 把插入点换算成码点偏移（节点内偏移是 UTF-16 单元） */
function clickCaretOffset(e) {
  let node = null;
  let off = 0;
  if (document.caretPositionFromPoint) {
    const pos = document.caretPositionFromPoint(e.clientX, e.clientY);
    if (pos && els.committed.contains(pos.offsetNode)) { node = pos.offsetNode; off = pos.offset; }
  } else if (document.caretRangeFromPoint) {
    const r = document.caretRangeFromPoint(e.clientX, e.clientY);
    if (r && els.committed.contains(r.startContainer)) { node = r.startContainer; off = r.startOffset; }
  }
  if (!node) return Array.from(composer.committed).length;
  let acc = '';
  for (const n of els.committed.childNodes) {
    if (n === node) return Array.from(acc + n.textContent.slice(0, off)).length;
    acc += n.textContent; // 文本节点 + 空的光标 span
  }
  return Array.from(composer.committed).length;
}

els.committed.addEventListener('click', (e) => {
  committedCaret = clickCaretOffset(e);
  renderCommitted();
  els.pinyin.focus({ preventScroll: true }); // 键盘事件仍挂在输入框上
});

// ---------- 候选与上屏 ----------

function totalPages() {
  return Math.max(1, Math.ceil(currentCandidates.length / pageSize));
}

/** 翻页（dir: +1/-1）。翻动成功返回 true；越界返回 false。 */
function flipPage(dir) {
  const next = pageIdx + dir;
  if (next < 0 || next >= totalPages()) return false;
  pageIdx = next;
  renderCandidates();
  els.candidates.scrollLeft = 0; // 移动端横滑容器回到页首
  return true;
}

function renderCandidates() {
  els.candidates.innerHTML = '';
  if (!currentCandidates.length) {
    els.candidates.innerHTML = '<span class="empty">（无候选）</span>';
    els.candPager.hidden = true;
    return;
  }
  const pages = totalPages();
  if (pageIdx >= pages) pageIdx = 0;
  const start = pageIdx * pageSize;
  currentCandidates.slice(start, start + pageSize).forEach((c, i) => {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'cand' + (i === 0 ? ' top1' : '');
    btn.title = c.reading ? `reading: ${c.reading}` : '';
    const word = document.createElement('span');
    word.className = 'cand-word';
    word.textContent = c.word;
    const meta = document.createElement('span');
    meta.className = 'cand-meta';
    const score = typeof c.score === 'number' ? c.score.toFixed(3) : '—';
    meta.textContent = `${i + 1}. ${score} k${c.kind ?? '?'}`;
    btn.appendChild(word);
    btn.appendChild(meta);
    btn.addEventListener('click', () => pick(start + i));
    els.candidates.appendChild(btn);
  });
  els.candPager.hidden = pages <= 1;
  els.pageInd.textContent = `${pageIdx + 1}/${pages}`;
  els.pagePrev.disabled = pageIdx === 0;
  els.pageNext.disabled = pageIdx >= pages - 1;
}

function pick(idx) {
  const c = currentCandidates[idx];
  if (!c) return;
  const cps = Array.from(composer.committed);
  const at = Math.min(Math.max(committedCaret, 0), cps.length);
  const word = composer.pick(c, at);
  if (word == null) return;
  committedCaret = at + Array.from(word).length;
  renderCommitted();
  els.preedit.textContent = '';
  els.pinyin.value = '';
  currentCandidates = [];
  pageIdx = 0;
  renderCandidates();
  els.pinyin.focus();
  scheduleQuery(); // 空态触发联想条（上屏后续想）
}

function scheduleQuery() {
  clearTimeout(queryTimer);
  queryTimer = setTimeout(runQuery, 60);
}

async function runQuery() {
  const pending = composer.pending;
  if (!pending || !adapter.ready) {
    if (!pending) {
      // 联想条（微信产品形态: 上屏后空态续想）——空输入×ctx 走引擎
      // assoc 面（中文 bigram 联想 + 英文 c4g 词对, 2026-11-20）
      if (composer.committed && adapter.ready) {
        const seq = ++querySeq;
        const t0 = performance.now();
        try {
          const cands = await adapter.query('', settings.topk, composer.committed);
          if (seq !== querySeq) return;
          lastQueryMs = performance.now() - t0;
          currentCandidates = cands;
          pageIdx = 0;
          renderCandidates();
          els.latency.textContent = `联想 ${lastQueryMs.toFixed(1)}ms`;
          return;
        } catch { /* 联想失败静默 */ }
      }
      currentCandidates = [];
      pageIdx = 0;
      renderCandidates();
      els.preedit.textContent = '';
    }
    return;
  }
  els.preedit.textContent = pending;
  const seq = ++querySeq;
  const t0 = performance.now();
  // ctx = 已上屏文本（引擎自取尾段——B3-d 搭配钉序/联想/bigram 消费）
  // 候选池 = 每页 × POOL_PAGES，供翻页；首页 = 引擎序前 pageSize 条
  let cands = await adapter.query(pending, pageSize * POOL_PAGES, composer.committed);
  // 纯拉丁输入 = 英文模式（四小件英文通道）: 英文候选置前, 拼音面垫后
  if (/^[a-z]+$/.test(pending)) {
    try {
      const en = await adapter.enCandidates(pending, 8);
      if (en.length) cands = en.concat(cands);
    } catch { /* en 表缺席静默 */ }
  }
  if (seq !== querySeq) return; // 输入已变，丢弃过期结果
  lastQueryMs = performance.now() - t0;
  recordLatency(lastQueryMs);
  currentCandidates = cands;
  pageIdx = 0;
  renderCandidates();
  els.latency.textContent = `查询 ${lastQueryMs.toFixed(1)}ms`;
}

// ---------- T9 九宫开关（引擎 enableT9 + localStorage 持久） ----------
function applyT9(on) {
  settings.t9 = !!on;
  try { if (adapter.engine && adapter.engine.enableT9) adapter.engine.enableT9(settings.t9); } catch {}
  els.t9Toggle.checked = settings.t9;
  saveSettings(settings);
}
els.t9Toggle?.addEventListener('change', () => applyT9(els.t9Toggle.checked));

// ---------- 输入事件 ----------

els.pinyin.addEventListener('input', () => {
  const clean = composer.setPending(els.pinyin.value, settings.t9);
  if (clean !== els.pinyin.value) els.pinyin.value = clean; // 过滤非法字符后回写
  scheduleQuery();
});

els.pinyin.addEventListener('keydown', (e) => {
  if (e.key === ' ' || e.key === 'Enter') {
    if (currentCandidates.length) { e.preventDefault(); pick(pageIdx * pageSize); } // 当前页首选
    return;
  }
  if (/^[0-9]$/.test(e.key)) {
    const d = e.key === '0' ? 9 : Number(e.key) - 1;
    const idx = pageIdx * pageSize + d;
    if (d < pageSize && currentCandidates[idx]) { e.preventDefault(); pick(idx); }
    return;
  }
  if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
    // 多页时方向键翻页；单页保持原生光标移动
    if (totalPages() > 1 && flipPage(e.key === 'ArrowRight' ? 1 : -1)) e.preventDefault();
    return;
  }
  if (e.key === 'PageDown' || e.key === 'PageUp') {
    if (totalPages() > 1 && flipPage(e.key === 'PageDown' ? 1 : -1)) e.preventDefault();
    return;
  }
  if (e.key === 'Escape') {
    composer.clearPending();
    els.pinyin.value = '';
    currentCandidates = [];
    pageIdx = 0;
    renderCandidates();
    els.preedit.textContent = '';
    return;
  }
  if (e.key === 'Backspace' && els.pinyin.value === '' && composer.committed) {
    e.preventDefault();
    // 删插入点前的字符（默认句尾 = 原「删最后一个」；句首改删首字，保证退格始终有反馈）
    const removed = composer.uncommitLast(committedCaret);
    if (removed != null && committedCaret > 0) committedCaret -= 1;
    renderCommitted();
  }
});

// 候选翻页按钮（点击后焦点还给输入框——数字键/方向键继续可用）
els.pagePrev.addEventListener('click', () => { flipPage(-1); els.pinyin.focus(); });
els.pageNext.addEventListener('click', () => { flipPage(1); els.pinyin.focus(); });

// 移动端：候选区触摸滑动翻页（快拂翻页）与原生横滑滚动共存——
// 慢速横向拖动 = 行内滚动看本页；快速轻拂（>48px 且横向占优）= 翻页
let touchStart = null;
els.candidates.addEventListener('touchstart', (e) => {
  if (e.touches.length !== 1) { touchStart = null; return; }
  const t = e.touches[0];
  touchStart = { x: t.clientX, y: t.clientY, ts: performance.now() };
}, { passive: true });
els.candidates.addEventListener('touchend', (e) => {
  if (!touchStart) return;
  const t = e.changedTouches[0];
  const dx = t.clientX - touchStart.x;
  const dy = t.clientY - touchStart.y;
  const dt = performance.now() - touchStart.ts;
  touchStart = null;
  if (dt <= 450 && Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy) * 1.5) {
    flipPage(dx < 0 ? 1 : -1); // 左拂下一页 / 右拂上一页；越界时 flipPage 自行忽略
  }
}, { passive: true });
els.candidates.addEventListener('touchcancel', () => { touchStart = null; }, { passive: true });

// 示例 chips
for (const chip of els.examples.querySelectorAll('button')) {
  chip.addEventListener('click', () => {
    els.pinyin.value = '';
    composer.setPending(chip.dataset.example);
    els.pinyin.value = composer.pending;
    runQuery();
    els.pinyin.focus();
  });
}

// ---------- 设置面 ----------

els.settingsBtn.addEventListener('click', () => {
  const open = els.settingsPanel.hidden;
  els.settingsPanel.hidden = !open;
  els.settingsBtn.setAttribute('aria-expanded', String(open));
});

els.topkRange.addEventListener('input', () => {
  const v = clampTopk(els.topkRange.value);
  els.topkVal.textContent = String(v);
  if (v === pageSize) return;
  pageSize = v;
  settings.topk = v;
  saveSettings({ topk: v });
  pageIdx = 0;
  renderCandidates(); // 立即按新页宽重排当前候选池
  if (composer.pending) scheduleQuery(); // 池大小 = 页宽 × 3，需重查
});

for (const b of els.themeSeg.querySelectorAll('button')) {
  b.addEventListener('click', () => {
    settings.theme = b.dataset.themeOpt;
    saveSettings({ theme: settings.theme });
    applyThemeSetting();
  });
}

// 清除缓存
els.clearCache.addEventListener('click', async () => {
  if (!confirm('清除 IndexedDB 中的资产缓存？（下次访问将重新下载 69.6MB 词典）')) return;
  await adapter.clearAssetCache();
  updateCacheInfo();
  els.statusMsg.textContent = '资产缓存已清除（页面刷新后生效）';
});

// 重试引擎（失败/未部署时可见）
els.retry.addEventListener('click', async () => {
  els.statusMsg.textContent = '重新加载引擎…';
  adapter.mode = 'idle';
  adapter.loadError = null;
  try {
    await adapter.load({ forceRefresh: true });
  } catch { /* 状态已由 renderStatus 呈现 */ }
});

// ---------- 启动 ----------

els.topkRange.value = String(pageSize);
els.topkVal.textContent = String(pageSize);
applyThemeSetting();
renderCommitted();
adapter.onStatus = renderStatus;
renderStatus({ phase: 'probe', message: '正在探测引擎…' });
adapter.load().catch(() => { /* 状态已由 renderStatus 呈现 */ });
requestPersistence(); // 尽力申请持久化，降低缓存被逐出概率
updateCacheInfo();
els.pinyin.focus();
