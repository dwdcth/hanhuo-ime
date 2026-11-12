#!/usr/bin/env python3
"""serve.py — web/app 静态演示服务器

用法（在 web/app 目录下）:
    python3 serve.py [端口]        # 默认 8000，绑定 0.0.0.0（手机同局域网可访问）

相比 `python3 -m http.server` 的差异：
  - 保证 .wasm -> application/wasm（Python <=3.12 的 http.server 不识别 wasm MIME，
    若引擎 glue 使用 instantiateStreaming 会被 MIME 检查拒绝）
  - 开发期禁用缓存（no-store），迭代资产不用强刷
"""
import sys
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer


class Handler(SimpleHTTPRequestHandler):
    extensions_map = {
        **SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.bin': 'application/octet-stream',
        '.safetensors': 'application/octet-stream',
        '.js': 'text/javascript; charset=utf-8',
        '.mjs': 'text/javascript; charset=utf-8',
    }

    def end_headers(self):
        self.send_header('Cache-Control', 'no-store')
        super().end_headers()

    def log_message(self, fmt, *args):
        sys.stderr.write('[serve] %s\n' % (fmt % args))


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    server = ThreadingHTTPServer(('0.0.0.0', port), partial(Handler, directory='.'))
    print(f'[serve] http://0.0.0.0:{port}/  (Ctrl+C 停止；手机访问 http://<本机局域网IP>:{port}/)')
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print('\n[serve] 已停止')


if __name__ == '__main__':
    main()
