import base64
import hashlib
import http.server
import json
import pathlib
import ssl
import sys
import time

root = pathlib.Path(sys.argv[1])
cert = root / "cert.pem"
token = "a3" * 32
code = "b9" * 32


class Fixture(http.server.BaseHTTPRequestHandler):
    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        payload = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path == "/pair" and payload.get("code") == code:
            self.reply(200, {"token": token})
        else:
            self.reply(401, {})

    def do_GET(self):
        if self.path == "/status" and self.headers.get("Authorization") == "Bearer " + token:
            self.reply(200, {"downloads": [{"name": "test.zip"}], "transfers": []})
        else:
            self.reply(401, {})


server = http.server.ThreadingHTTPServer(("127.0.0.1", 42672), Fixture)
tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
tls.load_cert_chain(str(cert), str(root / "key.pem"))
server.socket = tls.wrap_socket(server.socket, server_side=True)
payload = {"host": "127.0.0.2:42672", "hosts": ["127.0.0.1:42672"],
           "pin": hashlib.sha256(ssl.PEM_cert_to_DER_cert(cert.read_text())).hexdigest(),
           "code": code, "expires": int(time.time()) + 600}
(root / "qr").write_text(base64.b64encode(("NDM1:" + json.dumps(payload)).encode()).decode())
server.serve_forever()
