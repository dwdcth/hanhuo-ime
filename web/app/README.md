# web/app — 汉火输入法静态网页演示

纯静态演示页：原生 HTML/CSS/JS（ES modules），**无构建、无框架、无后端**。
引擎侧 WASM 未部署时自动降级内置 Mock（假数据），交互链路始终可演示。

## 文件清单

| 文件 | 作用 |
|---|---|
| `index.html` | 页面骨架（viewport 适配移动端; 设置面板/翻页条/字数徽标, 2026-11-20 UI 深化） |
| UI 能力（2026-11-20） | 候选翻页（按钮/方向键/触摸轻拂, 池=页宽×3）、topk 5-20 与主题三态设置（localStorage 持久）、上屏区点击定位+句中插入+句首退格删首字、延迟环形统计（mean/p95） | 子代理 B 产出, 冒烟 23/23 |
| `style.css` | 样式（亮/暗自适应，候选横滑，触屏目标 ≥44px） |
| `app.js` | UI 装配：输入 → 候选 → 上屏、状态栏、缓存按钮 |
| `engine-adapter.js` | **引擎适配器（UI 与引擎唯一边界）**：探测/注入 glue、拉词典、init、降级 mock |
| `mock-engine.js` | Mock 引擎（实现 Hanhuo 契约的假数据引擎） |
| `composer.js` | 组合文本状态机（pending 拼音 / committed 上屏，纯逻辑） |
| `assets.js` | fetch 字节进度 + IndexedDB cache-first 资产加载 |
| `storage.js` | IndexedDB 资产缓存 + localStorage 设置（见 `STORAGE.md`） |
| `serve.py` | 推荐的静态服务器（.wasm MIME 修正 + no-store） |
| `tests/smoke.mjs` | Node 冒烟测试（无浏览器，`node tests/smoke.mjs`） |

## 部署：把引擎产物放到本目录

以下文件**不入库**（`.gitignore` 已排除），部署时拷贝到 `index.html` 同级目录：

| 文件 | 说明 | 来源 |
|---|---|---|
| `hanhuo.js` | 引擎 glue：**classic script**，加载后提供 `window.Hanhuo`，自行同目录加载 wasm | `web/engine/hanhuo.js` |
| `hanhuo.wasm` | WASM 模块（2.7MB，由 glue 加载/实例化） | `web/engine/hanhuo.wasm` |
| `nnlm.safetensors` | 可选 NNLM 模型（存在才传给 `init` 的 `nnlmBytes`; 注入成功**即开宽档 boost**——尾槽+B7 句重排+beam150, 2026-11-20 升默认口径） | `data/nnlm_open_mobile_v2.safetensors`（自训开源, 推荐） |
| `mdltr.safetensors` | 可选 mdltr 模型（`init` 的 `mdltrBytes`） | `data/mdltr.safetensors` |
| `en_open.tsv` | 可选英文词表（`init` 的 `enBytes`; **纯拉丁输入=英文模式**——英文候选置前拼音面垫后, 2026-11-20） | `data/en_open.tsv`（49,130 词开源表） |
| `enlm.safetensors` | 可选英文 LM（`init` 的 `enlmBytes`; 注入后英文候选切 **LM 语境序** P(w|BOS)——wor→world,work 优于频序） | `data/enlm_open_v2_fp16.safetensors`（fp16 19MB; 自训开源） |

一键拷贝（在仓库根目录）：

```bash
cp web/engine/hanhuo.js web/engine/hanhuo.wasm web/app/
cp ~/tmp/wetype-win/wxq_dict.bin web/app/   # 词典仅本地使用
```

glue 探测顺序：`?engine=` 参数 > `./engine-glue.js` > `./engine.js` > `./glue.js` > `./hanhuo.js`，
所以直接放 `web/engine/hanhuo.js` 原名即可命中。

### 引擎契约（`window.Hanhuo`，不可改）

```js
await Hanhuo.init(dictBytes: ArrayBuffer|Uint8Array, opts?: {nnlmBytes?, mdltrBytes?}) -> {ok, error?}
Hanhuo.query(input: string, topk?: number) -> Array<{word, score, kind, reading}>  // kind 引擎侧为字符串
Hanhuo.wasmBytes: Uint8Array
```

`engine-adapter.js` 是 UI 与引擎的唯一边界：真引擎 / Mock 都经由它，签名一致。

## 起服务

```bash
cd web/app
python3 serve.py            # 默认 8000，绑定 0.0.0.0（手机同局域网可访问）
# 或
python3 -m http.server 8000 # Python ≥3.13 才自带 .wasm MIME；更早版本请用 serve.py
```

打开 <http://localhost:8000/>。

## URL 参数（联调）

| 参数 | 作用 |
|---|---|
| `?engine=<path>` | 指定 glue 路径（跳过候选探测） |
| `?dict=<path>` | 词典路径（默认 `./wxq_dict.bin`） |
| `?nnlm=` / `?mdltr=` | 指定/关闭（`=0`）可选模型 |
| `?forceRefresh=1` | 跳过 IDB 缓存强制重下并覆盖 |
| `?noMock=1` | 引擎失败时不降级 mock（保持错误状态，便于联调） |

## 引擎状态与 Mock 模式

- 状态栏实时显示：探测 glue / 下载进度（字节级进度条）/ 初始化 / 就绪 / 失败原因；
- 引擎文件缺失或 init 失败 → 自动切 **Mock 演示模式**（黄色角标），状态栏保留失败原因，
  输入→候选→上屏全链路用假数据照常演示；
- 「重载引擎」按钮（mock/失败态出现）强制重新走一遍加载流程。

## 缓存行为

词典等资产 IndexedDB cache-first（二次访问零下载），详见 **[STORAGE.md](./STORAGE.md)**
（localStorage vs IndexedDB 评估与数据支撑）。「清除缓存」按钮清空 IDB 资产（设置保留）。

## 冒烟测试

```bash
cd web/app && node tests/smoke.mjs
# 14 项：分词 / mock 查询 / 上屏链路 / fetch 进度 / 适配器降级与真引擎契约 / IDB 降级
```

## 移动端

已适配（viewport + 触屏候选点击 + 输入框 `inputmode=latin`）：
`python3 serve.py` 后手机访问 `http://<本机局域网IP>:8000/`。
