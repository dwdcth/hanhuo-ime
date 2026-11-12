// tests/smoke.mjs — Node 冒烟测试（无需浏览器）
//
// 运行：node tests/smoke.mjs   （在 web/app 目录下）
// 覆盖：拼音分词 / mock 引擎查询 / 组合上屏链路 / fetch 进度 / 适配器降级与真引擎路径 / IDB 降级
//
// 这是对「输入 → 候选 → 上屏」交互链路的无 DOM 等价验证：
// composer + adapter 与 app.js 中使用的是同一份模块。

import assert from 'node:assert/strict';
import { segmentPinyin, MockEngine } from '../mock-engine.js';
import { Composer } from '../composer.js';
import { fetchWithProgress, loadAsset, urlExists } from '../assets.js';
import { EngineAdapter } from '../engine-adapter.js';
import { IdbAssetCache, loadSettings, saveSettings } from '../storage.js';

const realFetch = globalThis.fetch;
let passed = 0;
const failures = [];

async function test(name, fn) {
  try {
    await fn();
    passed += 1;
    console.log(`PASS ${name}`);
  } catch (err) {
    failures.push({ name, err });
    console.error(`FAIL ${name}\n  ${err && err.stack ? err.stack : err}`);
  }
}

// ---------- 工具：构造流式 Response 桩 ----------

function makeStreamRes(chunks, total, { status = 200 } = {}) {
  let i = 0;
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: { get: (k) => (k.toLowerCase() === 'content-length' && status === 200 ? String(total) : null) },
    body: {
      getReader() {
        return {
          async read() {
            return i < chunks.length ? { done: false, value: chunks[i++] } : { done: true, value: undefined };
          },
        };
      },
    },
  };
}

const notFound = () => ({ ok: false, status: 404, headers: { get: () => null }, body: null });

// ---------- 1. 拼音分词 ----------

await test('segmentPinyin 贪心分词', () => {
  assert.deepEqual(segmentPinyin('nihao'), ['ni', 'hao']);
  assert.deepEqual(segmentPinyin('jintiantianqibucuo'), ['jin', 'tian', 'tian', 'qi', 'bu', 'cuo']);
  assert.deepEqual(segmentPinyin('mulujiegou'), ['mu', 'lu', 'jie', 'gou']);
  assert.deepEqual(segmentPinyin('zhuang'), ['zhuang']);
  assert.deepEqual(segmentPinyin('a'), ['a']);
});

// ---------- 2. Mock 引擎 ----------

await test('mock 精确词条查询 nihao', async () => {
  const m = new MockEngine();
  const r = await m.init(null);
  assert.equal(r.ok, true);
  const cands = m.query('nihao', 10);
  assert.ok(cands.length >= 3, `候选数 ${cands.length} >= 3`);
  assert.equal(cands[0].word, '你好');
  assert.ok(cands[0].score > 0.9);
  for (const c of cands) {
    assert.ok(['word', 'score', 'kind', 'reading'].every((k) => k in c), '契约字段齐全');
  }
  // topk 截断
  assert.ok(m.query('nihao', 2).length <= 2);
  // 空输入
  assert.deepEqual(m.query('', 10), []);
  assert.deepEqual(m.query('NIHAO', 10)[0].word, '你好'); // 大写归一
});

await test('mock 长句拼合 jintiantianqibucuo', () => {
  const m = new MockEngine();
  const cands = m.query('jintiantianqibucuo', 10);
  assert.ok(cands.length > 0);
  assert.equal(cands[0].word, '今天天气不错');
  assert.equal(cands[0].kind, 3); // 拼句
  // 分数降序
  for (let i = 1; i < cands.length; i++) assert.ok(cands[i - 1].score >= cands[i].score);
});

// ---------- 3. Composer 组合链路 ----------

await test('composer 输入→候选→上屏 拼句', async () => {
  const engine = new MockEngine();
  await engine.init(null);
  const composer = new Composer();

  // 第一段：nihao → 你好
  composer.setPending('nihao');
  const c1 = engine.query(composer.pending, 10);
  assert.equal(c1[0].word, '你好');
  assert.equal(composer.pick(c1[0]), '你好');
  assert.equal(composer.committed, '你好');

  // 第二段：继续输入拼句
  composer.setPending('jintiantianqibucuo');
  const c2 = engine.query(composer.pending, 10);
  composer.pick(c2[0]);
  assert.equal(composer.committed, '你好今天天气不错');

  // 退格删已上屏（按码点）
  assert.equal(composer.uncommitLast(), '错');
  assert.equal(composer.committed, '你好今天天气不');

  // 非法字符过滤
  composer.setPending('Ni1Hao!');
  assert.equal(composer.pending, 'nihao');
  assert.equal(composer.needsQuery, true);
  composer.clearPending();
  assert.equal(composer.needsQuery, false);
});

// ---------- 4. fetch 进度 ----------

await test('fetchWithProgress 字节进度与累计', async () => {
  const chunks = [new Uint8Array(10).fill(1), new Uint8Array(15).fill(2), new Uint8Array(5).fill(3)];
  globalThis.fetch = async () => makeStreamRes(chunks, 30);
  const seen = [];
  const { bytes, total } = await fetchWithProgress('./fake.bin', {
    onProgress: (loaded, t) => seen.push([loaded, t]),
  });
  assert.equal(bytes.byteLength, 30);
  assert.equal(total, 30);
  assert.deepEqual(seen.map((x) => x[0]), [10, 25, 30]);
  assert.equal(bytes[29], 3); // 顺序拼接正确
});

await test('fetchWithProgress 非 2xx 抛错', async () => {
  globalThis.fetch = async () => makeStreamRes([], 0, { status: 404 });
  await assert.rejects(() => fetchWithProgress('./gone.bin'));
});

await test('loadAsset 无 IDB 环境直连 fetch', async () => {
  globalThis.fetch = async () => makeStreamRes([new Uint8Array(7).fill(9)], 7);
  const r = await loadAsset('./seven.bin');
  assert.equal(r.bytes.byteLength, 7);
  assert.equal(r.fromCache, false); // node 无 indexedDB，缓存读写均降级
});

await test('urlExists 404 -> false', async () => {
  globalThis.fetch = async () => notFound();
  assert.equal(await urlExists('./none.safetensors'), false);
});

// ---------- 5. IDB / 设置 降级 ----------

await test('IdbAssetCache 在 node 下安全降级', async () => {
  const c = new IdbAssetCache();
  assert.equal(c.available, false);
  assert.equal(await c.get('x'), null);
  assert.equal(await c.put('x', new ArrayBuffer(4)), false);
  assert.deepEqual(await c.entries(), []);
  assert.equal(await c.clear(), false);
});

await test('settings: node 下走内存兜底，读写一致', () => {
  const s1 = loadSettings({ topk: 10 });
  assert.equal(s1.topk, 10);
  const s2 = saveSettings({ topk: 8 });
  assert.equal(s2.topk, 8);
  assert.equal(loadSettings({ topk: 10 }).topk, 8);
});

// ---------- 6. 适配器：引擎缺失 → mock 降级（端到端链路） ----------

await test('adapter: 无引擎文件 → mock 模式，链路可用', async () => {
  delete globalThis.Hanhuo;
  globalThis.fetch = async () => notFound(); // 所有探测/词典请求 404
  const a = new EngineAdapter();
  const statuses = [];
  a.onStatus = (s) => statuses.push(s.phase);
  const { mode } = await a.load();
  assert.equal(mode, 'mock');
  assert.equal(a.mode, 'mock');
  assert.ok(a.loadError && a.loadError.includes('glue'), `失败原因保留: ${a.loadError}`);
  assert.ok(statuses.includes('mock'));

  // 完整链路：输入 → 候选 → 上屏
  const composer = new Composer();
  composer.setPending('nihao');
  const cands = await a.query(composer.pending, 10);
  assert.equal(cands[0].word, '你好');
  composer.pick(cands[0]);
  assert.equal(composer.committed, '你好');
});

// ---------- 7. 适配器：注入引擎对象（真引擎路径，跳过脚本注入） ----------

await test('adapter: engineObject 路径 init/query 契约', async () => {
  delete globalThis.Hanhuo;
  const dictBytes = new Uint8Array(1024).fill(7);
  const calls = { init: 0, query: 0, initArgs: null, opts: null };
  const fakeEngine = {
    wasmBytes: new Uint8Array(2048),
    async init(dict, opts) {
      calls.init += 1;
      calls.initArgs = dict;
      calls.opts = opts;
      return { ok: true };
    },
    query(input, topk) {
      calls.query += 1;
      return [{ word: `E:${input}`, score: 0.5, kind: 1, reading: input }];
    },
  };
  // fetch 桩：词典 URL 返回字节流；nnlm/mdltr HEAD 404
  globalThis.fetch = async (url) => {
    if (String(url).includes('wxq_dict.bin')) return makeStreamRes([dictBytes], dictBytes.byteLength);
    return notFound();
  };

  const a = new EngineAdapter({ engineObject: fakeEngine });
  const seen = [];
  a.onStatus = (s) => seen.push(s);
  const { mode } = await a.load();
  assert.equal(mode, 'engine');
  assert.equal(calls.init, 1);
  assert.ok(calls.initArgs instanceof ArrayBuffer);
  assert.equal(calls.initArgs.byteLength, 1024); // init 收到完整词典字节
  assert.deepEqual(calls.opts, {}); // 可选模型缺失不带键
  assert.equal(a.info.wasmBytes, 2048);
  assert.equal(a.info.dictSize, 1024);
  assert.ok(seen.some((s) => s.phase === 'fetch' && s.progress && s.progress.loaded > 0), '进度事件已上报');
  assert.ok(seen.some((s) => s.phase === 'ready'));

  const out = await a.query('abc', 5);
  assert.equal(calls.query, 1);
  assert.equal(out[0].word, 'E:abc');
});

await test('adapter: init 返回 ok:false → 降级 mock 并保留原因', async () => {
  delete globalThis.Hanhuo;
  globalThis.fetch = async (url) => {
    if (String(url).includes('wxq_dict.bin')) return makeStreamRes([new Uint8Array(16)], 16);
    return notFound();
  };
  const bad = {
    async init() { return { ok: false, error: '词典魔数不符' }; },
    query: () => [],
  };
  const a = new EngineAdapter({ engineObject: bad });
  const { mode } = await a.load();
  assert.equal(mode, 'mock');
  assert.ok(a.loadError.includes('词典魔数不符'));
});

await test('adapter: noMock 模式下失败不降级', async () => {
  delete globalThis.Hanhuo;
  globalThis.fetch = async () => notFound();
  const a = new EngineAdapter();
  a.onStatus = () => {};
  await assert.rejects(() => a.load({ noMock: true }), /glue/);
  assert.equal(a.mode, 'error');
});

// ---------- 汇总 ----------

globalThis.fetch = realFetch;
console.log(`\n${passed} passed, ${failures.length} failed`);
if (failures.length) {
  process.exit(1);
}
