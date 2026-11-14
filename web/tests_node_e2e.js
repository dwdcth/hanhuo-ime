// hanhuo wasm 端到端: 真词典 bin + wasm 查询 × 原生 CLI 对照
const fs = require('fs');
// nodejs 胶水目录（WASM_NODE 覆盖; 生成: wasm-bindgen --target nodejs
// --out-dir "$WASM_NODE" target/wasm32-unknown-unknown/release/hanhuo_wasm.wasm）
const GLUE = process.env.WASM_NODE || '/tmp/wasm_node';
const w = require(GLUE + '/hanhuo_wasm.js');
// nodejs target 的 glue require 即装载; target web 模块才需显式
// initSync（兼容双形态: 有则调）
if (typeof w.initSync === 'function') {
  w.initSync({ module: fs.readFileSync(GLUE + '/hanhuo_wasm_bg.wasm') });
}
const { execSync } = require('child_process');

const bin = fs.readFileSync(
  __dirname + '/../open/hanhuo_open.bin');
const n = w.init(bin);
console.log(`[init] 词条数: ${n}`);
if (!n) process.exit(1);

const cases = [
  ['jintiantianqihenhao', ''],
  ['nihao', ''],
  ['zhongguorenmin', ''],
  ['weixinshurufa', ''],
];
let pass = 0;
for (const [q, ctx] of cases) {
  const r = JSON.parse(w.query(q, ctx, 5));
  const native = execSync(
    `${__dirname}/../target/release/hanhuo query ${q} --bin ${__dirname}/../open/hanhuo_open.bin --topk 5`,  // CLI 默认同律（typo_retry 通道, wasm query 同源）
    { encoding: 'utf-8' });
  const natTop = [...native.matchAll(/^\s+1\. (\S+)/gm)][0]?.[1]?.replace(/[\x00-\x1f]/g, '');
  const wasmTop = r.cands[0]?.word;
  const ok = wasmTop === natTop;
  console.log(`${ok ? 'PASS' : 'FAIL'} ${q}: wasm=${wasmTop} native=${natTop}`);
  pass += ok;
}
console.log(`\n端到端: ${pass}/${cases.length} 一致`);
process.exit(pass === cases.length ? 0 : 1);
