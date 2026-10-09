from http.server import HTTPServer, BaseHTTPRequestHandler
import sys

port = int(sys.argv[1]) if len(sys.argv) > 1 else 8081

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.end_headers()
        if self.path == "/health":
            self.wfile.write(b"OK")
        else:
            self.wfile.write(f"Antwort vom Backend auf Port {port}\n".encode())

print(f"Starte Test-Backend auf Port {port}...")
HTTPServer(("127.0.0.1", port), Handler).serve_forever()
