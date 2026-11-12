from playwright.sync_api import sync_playwright

with sync_playwright() as pw:
    b = pw.chromium.launch()
    p = b.new_page()
    errs = []
    p.on("pageerror", lambda e: errs.append(str(e)))
    p.goto("http://127.0.0.1:8765/web/app/index.html"
           "?engine=./engine-glue.js&dict=../../open/hanhuo_open.bin&noMock=1")
    # 等引擎就绪（状态文案含 词条/ready/就绪 任一）
    p.wait_for_function(
        "() => /词条|就绪|ready/i.test(document.body.innerText)",
        timeout=60000)
    body = p.text_content("body")
    print("状态行:", [l for l in body.split("\n")
                     if any(k in l for k in ("词条", "就绪", "ready", "引擎"))][:3])
    # 找输入框打字
    inp = p.query_selector("input[type=text], input:not([type])")
    if inp:
        inp.fill("nihao")
        inp.press("Enter")
        p.wait_for_timeout(800)
        body2 = p.text_content("body")
        print("nihao 后含'你好':", "你好" in body2)
        ok = "你好" in body2
        # 宽档断言（2026-11-20）: nnlm.safetensors 在场 → 长句应出
        # sentence 宽档面（native 宽档逐位一致锚: 下午几点…吃饭 #1）
        inp.fill("xiawujidianshimenkechifan")
        # 不按 Enter（上屏会清面板）; fill 触发 input 事件即查询
        p.wait_for_timeout(1200)
        body3 = p.text_content("body")
        wide = "下午" in body3 and "吃饭" in body3
        print("宽档长句 sentence 面:", wide)
        ok = ok and wide
        # ctx 链断言（B3-d 搭配钉序, 2026-11-20）: 上屏「时间」后打
        # zhou → 轴 #1（无 ctx 时轴 #3; native --ctx 时间 同律）
        # 重置已上屏（前面断言的 Enter/Space 已上屏词, 清场保 ctx 纯净）
        p.keyboard.press("Escape"); p.fill("#pinyin", "")
        p.keyboard.press("Backspace"); p.wait_for_timeout(300)
        inp.fill("shijian")
        p.wait_for_function("() => document.body.textContent.includes('时间')", timeout=5000)
        inp.press("Space"); p.wait_for_timeout(500)
        inp.fill("zhou"); p.wait_for_timeout(1000)
        body4 = p.text_content("body")
        b3d = body4.find("轴") != -1 and body4.find("轴") < body4.find("周")
        print("B3-d 时间+zhou→轴前置:", b3d,
              "| committed:", repr(p.text_content("#committed")))
        ok = ok and b3d
        # 英文通道断言（四小件部署完整性, 2026-11-20）: 纯拉丁输入
        # → 英文候选置前（hello 精确词 #1; wor → work/world 补全）
        p.keyboard.press("Escape")
        inp.fill("hello"); p.wait_for_timeout(900)
        bh = p.text_content("body")
        en1 = "hello" in bh and bh.find("hello1") != -1
        print("英文精确词 #1:", en1)
        inp.fill("wor"); p.wait_for_timeout(900)
        bw = p.text_content("body")
        en2 = "work" in bw and "world" in bw
        print("英文前缀补全 work/world:", en2)
        ok = ok and en1 and en2
    else:
        print("无输入框, body 预览:", body[:200])
        ok = False
    if errs:
        print("页面错误:", errs[:3])
    b.close()
    raise SystemExit(0 if ok else 1)
