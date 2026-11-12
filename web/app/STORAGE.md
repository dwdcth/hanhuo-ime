# 存储方案评估：localStorage vs IndexedDB

> 结论先行：**大资产（词典/模型）必须走 IndexedDB——localStorage 装不下，这是决定性依据；
> 用户设置等小状态留在 localStorage。** 本页已按此实现（`storage.js`）。

## 决策表

| 数据 | 实际大小 | 存放 | 决定性依据 |
|---|---|---|---|
| `wxq_dict.bin` 词典 | **69.6 MB** | IndexedDB | localStorage 上限 2.5–10 MB/源，差一个数量级以上 |
| `nnlm.safetensors`（可选） | 3.9–15.1 MB | IndexedDB | fp32 版 15.1MB 已超多数浏览器 localStorage 上限 |
| `mdltr.safetensors`（可选） | 1.8 MB | IndexedDB | 与词典同管线，统一处理 |
| 用户设置（topk 等） | < 1 KB | localStorage | 同步读、无二进制、永不触限 |
| `engine.wasm` / glue js | MB 级 | HTTP（浏览器自带缓存） | 由 glue 脚本自行加载，不经本页 IDB 层 |

## 为什么词典不能放 localStorage（数据支撑）

1. **配额差一个数量级**。Web Storage 各浏览器实现为每源 **2.5–10 MB**：
   Firefox 自 v34 起为 10 MB（早期 5 MB）；Chrome 约 5 MB（按 UTF-16 码元计）；
   Safari 约 5 MB；iOS 隐私模式直接不可写。
   词典 69.6 MiB = 73,014,323 字节，是上限的 **7–28 倍**，任何 polyfill/分片都救不回来。
2. **只支持字符串**。二进制必须 base64，体积 **+33%**：69.6 MiB → 约 **97 MB**，进一步放大差距。
3. **同步 API**。localStorage 读写阻塞主线程——写入 97 MB 字符串会冻结页面数秒；
   IndexedDB 的 Structured Clone 是异步的，不卡 UI。

## 为什么 IndexedDB 可行

- **配额是磁盘级**：Chrome 单源最多约占总磁盘的 60%（浏览器全局上限为磁盘的 80%）、
  Firefox 约磁盘的 50%（超限部分可被逐出）、Safari 约 1 GB（更多需用户确认）。
  本机 Chromium 实测（页面页脚可见）：`navigator.storage.estimate()` 返回 **quota 10.00 GB**，
  是词典体积的 140 倍以上。
- **原生二进制**：`ArrayBuffer` 直接 Structured Clone 落盘，无 base64 开销。
- **可查询用量**：`navigator.storage.estimate()` 随时报告 usage/quota（本页页脚已展示）。
- **防逐出**：压力下浏览器可能清理"最佳努力"存储；本页启动时调用
  `navigator.storage.persist()` 申请持久化（`requestPersistence()`）。

## 设置为什么留在 localStorage

- 总量 < 1 KB，与配额相差 4 个数量级，localStorage 的唯一硬伤（容量）不构成问题；
- 页面启动需要**同步**读取一次（避免 IDB 的异步竞态），localStorage 天然同步；
- DevTools Application 面板直接可读可改，便于调试；
- 无二进制、无索引、无事务需求。

## 实现细节（`storage.js` + `assets.js`）

- 数据库 `hanhuo-assets` v1，objectStore `assets`；**key = 资产 URL**，
  value = `{ bytes: ArrayBuffer, ts, url, size }`。
- 加载流程（`loadAsset`）：
  1. 查 IDB → 命中直接返回（**二次访问零下载**，状态栏显示"缓存命中"）；
  2. 未命中 → `fetch` 流式读取（带字节进度回调，驱动状态栏进度条）→ 写回 IDB；
  3. IDB 不可用（隐私模式等）→ 仅 fetch 不缓存，功能不受影响（所有 IDB 调用都有降级分支）。
- **版本策略**：key 即 URL。更新词典 = 换 URL（如 `?dict=wxq_dict.v2.bin`）或让用户点
  「清除缓存」；`?forceRefresh=1` 可强制重下并覆盖。
- 「清除缓存」按钮：`confirm` 后清空 `assets` store（设置保留）。

## 参考来源

- MDN, *Storage quotas and eviction criteria* — <https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria>
- web.dev, *Storage for the Web*（Chrome/Firefox/Safari 配额） — <https://web.dev/articles/storage-for-the-web>
- Wikipedia, *Web storage*（localStorage 各浏览器上限） — <https://en.wikipedia.org/wiki/Web_storage>
- MDN, *Window.localStorage*（仅字符串、同步阻塞） — <https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage>
