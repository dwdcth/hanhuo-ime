# demo 部署 SOP（引擎演进后 wasm/词典/线上三件同步）

> 缘起（2026-11-14 实证）: 引擎连续两役（nihoa 回流筛/C3 漏按门）落地后
> demo 的 wasm 落后 4 天——用户在线上看不到修复。病根是三层: wasm 产物、
> boost 挂载、词典快照, 任何一层落后都会让线上行为与 CLI 脱节。

## 触发条件

引擎侧任何用户可见行为变更（typo/简拼/补全/boost 面）或词典构建变更
（open_build 逻辑/源数据）后, 除本地 CI 外需走本 SOP。

## 三层同步

### 1. wasm 重编

```bash
cargo build -p hanhuo-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir web target/wasm32-unknown-unknown/release/hanhuo_wasm.wasm
```

**坑（2026-11-14 实证, 两判据陷阱）**:

- **mbx mtime 冻结**: target 是 mbx 管理的符号链接, 产物更新后 mtime
  仍显示旧日期——**不可用 mtime 判产物新旧**。mbx 没有派发陈旧件
  （md5 与干净目录一致）, 只是时间戳显示误导。
- **字符串判据失效**: `C3_DBG` 等环境变量字符串在 wasm 下被常量折叠
  （wasm32 的 env::var 恒 Err）——**grep 不到 ≠ 旧产物**。
- **正确判据 = 行为**: nodejs 胶水直查（见下「快验」）。

如遇 mbx 疑虑, 绕开: `CARGO_TARGET_DIR=/mnt/s790/tmp/wasm_target
cargo build ...` 后从该目录 bindgen。

### 2. 词典快照同步

```bash
cp open/hanhuo_open.bin web/app/wxq_dict.bin
```

**坑**: app 的词典是快照拷贝（gitignore 产物）, 词典构建演进后不同步
则引擎行为裁决（C3 的 logp 域差、估频频次域）与本地 open bin 脱节
——2026-11-14 实证: 旧快照下 C3 差值不足 2.0 不翻转。

### 3. 线上 gh-pages 推送

```bash
# worktree 复用（/mnt/s790/tmp/ghpages, 已存在）
cd /mnt/s790/tmp/ghpages
cp <repo>/web/hanhuo_wasm.js web/hanhuo_wasm_bg.wasm demo/
cp <repo>/open/hanhuo_open.bin demo/
git add demo/ && git commit -m "demo: 引擎+词典同步 <main 提交号>" && git push origin gh-pages
```

线上身份验证（不必等浏览器）:

```bash
curl -sL <url>/hanhuo_wasm_bg.wasm | md5sum   # 对比 web/ 本地
curl -sL -r 0-1048575 <url>/hanhuo_open.bin | md5sum  # 对比 head -c 1M
```

Pages 首访下载 60MB 约 5 分钟（gzip 后 ~32MB, CDN 冷缓存正常）。

## 快验（部署前 node 行为验证）

```bash
wasm-bindgen --target nodejs --out-dir /mnt/s790/tmp/wasm_node \
  target/wasm32-unknown-unknown/release/hanhuo_wasm.wasm
node -e "
const w = require('/mnt/s790/tmp/wasm_node/hanhuo_wasm.js');
const fs = require('fs');
w.init(fs.readFileSync('open/hanhuo_open.bin'));
w.enable_boost();  // boost 必须显式开（无 boost 时 zhng/sm/zhongg 面全灭）
for (const q of ['zhng','sm','nihoa','zhongg','zhognguo']) {
  console.log(q, JSON.parse(w.query(q,'',5)).cands.slice(0,3).map(c=>c.word).join('|'));
}"
```

**验收锚（2026-11-14 基线）**: zhng→中|镇魂女鬼|脏, sm→什么|审美|说明,
nihoa→你好|拟好|你波啊, zhongg→中部|钟爱|中国, zhognguo→中国。

## 本地浏览器验收

```bash
python3 web/tests_browser_e2e.py   # e2e.html 4/4
# 主 app（playwright 直调引擎, 绕过 UI 层竞态）:
#   goto http://127.0.0.1:8765/web/app/?forceRefresh=1 → 等引擎就绪 →
#   Hanhuo.query('zhng',5,'') → 中 #1
```

**坑**: app boot 竞态——引擎就绪前查询返回空/英文面（enlm 先注入）。
验收用 `Hanhuo.query` 直调 + 等引擎就绪标志。

## boost 挂载律（2026-11-14 解耦后）

三处 glue（app engine-glue.js / e2e.html / 线上 demo/index.html）都是
**enable_boost 无条件开**, NNLM 结果独立记录。旧 `init_nnlm &&
enable_boost` 短路会让 NNLM 缺位时基础面（C3/简拼/B2.7e）全灭——
新增 glue 时勿回退此模式。
