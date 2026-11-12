// hanhuo wasm 端到端: 真词典 bin + wasm 查询 × 原生 CLI 对照
const fs = require('fs');
const w = require('/tmp/wasm_node/hanhuo_wasm.js');
// target web 模块需显式 initSync（node 无 fetch file）
w.initSync({ module: fs.readFileSync('/tmp/wasm_node/hanhuo_wasm_bg.wasm') });
const { execSync } = require('child_process');

const bin = fs.readFileSync(
  '/mnt/s790/project/vtype-linux-archive/hanhuo/open/hanhuo_open.bin');
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
    `/mnt/s790/project/vtype-linux-archive/hanhuo/target/release/hanhuo query ${q} --bin /mnt/s790/project/vtype-linux-archive/hanhuo/open/hanhuo_open.bin --topk 5  # CLI 默认同律（typo_retry 通道, wasm query 同源）`,
    { encoding: 'utf-8', cwd: '/mnt/s790/project/vtype-linux-archive/hanhuo' });
  const natTop = [...native.matchAll(/^\s+1\. (\S+)/gm)][0]?.[1]?.replace(/[\x00-\x1f]/g, '');
  const wasmTop = r.cands[0]?.word;
  const ok = wasmTop === natTop;
  console.log(`${ok ? 'PASS' : 'FAIL'} ${q}: wasm=${wasmTop} native=${natTop}`);
  pass += ok;
}
console.log(`\n端到端: ${pass}/${cases.length} 一致`);
process.exit(pass === cases.length ? 0 : 1);
