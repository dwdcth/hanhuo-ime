from playwright.sync_api import sync_playwright

with sync_playwright() as pw:
    b = pw.chromium.launch()
    p = b.new_page()
    p.goto("http://127.0.0.1:8765/web/e2e.html")
    p.wait_for_function(
        "() => document.getElementById('out')?.textContent.includes('端到端')",
        timeout=30000)
    out = p.text_content("#out")
    print(out)
    print("页面状态:", p.text_content("#status"))
    ok = "端到端 4/4" in out
    # 交互一测: 输入框查询
    p.fill("#q", "shurufa")
    p.click("#go")
    inter = p.text_content("#out")
    print("交互查询 shurufa →", inter.split("\n")[0])
    b.close()
    raise SystemExit(0 if ok and "输入法" in inter.split("\n")[0] else 1)
