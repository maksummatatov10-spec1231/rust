#!/usr/bin/env python3
"""Static server for the game with the correct application/wasm MIME type."""
import functools
import http.server
import socketserver

PORT = 8081
HANDLER = functools.partial(
    http.server.SimpleHTTPRequestHandler,
    directory=".",
)
HANDLER.extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}

class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True

if __name__ == "__main__":
    with Server(("0.0.0.0", PORT), HANDLER) as httpd:
        print(f"Serving Forest Bloom 3D on http://0.0.0.0:{PORT}", flush=True)
        httpd.serve_forever()
