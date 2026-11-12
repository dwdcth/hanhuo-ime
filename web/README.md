# hanhuo wasm 浏览器端到端

- `hanhuo_wasm.{js,wasm}`: wasm-bindgen --target web 产物（2.8MB）
- `e2e.html`: 端到端页（fetch ../open/hanhuo_open.bin → init →
  自动对案四案 + 交互查询）
- 复现:
  1. `cargo build -p hanhuo-wasm --target wasm32-unknown-unknown --release`
  2. `wasm-bindgen --target web --out-dir web target/.../hanhuo_wasm.wasm`
  3. `python3 -m http.server 8765` → http://127.0.0.1:8765/web/e2e.html
- 无头验证: `python3 tests_browser_e2e.py`（playwright, 断言 4/4 +
  交互 shurufa→输入法）; node 侧: `node tests_node_e2e.js`
  （--target nodejs 胶水, 与原生 CLI 逐案对照）
