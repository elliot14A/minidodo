import hashlib
import hmac
import json
import os
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

SIGNING_SECRET = os.environ.get("WEBHOOK_SIGNING_SECRET", "whsec_test_secret_12345")
PORT = int(os.environ.get("PORT", "9000"))


def verify(secret, timestamp, body):
    mac = hmac.new(secret.encode(), digestmod=hashlib.sha256)
    mac.update(timestamp.encode())
    mac.update(b".")
    mac.update(body)
    return mac.hexdigest()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(length)

        signature = self.headers.get("x-webhook-signature", "")
        timestamp = self.headers.get("x-webhook-timestamp", "")
        received = signature[len("sha256="):] if signature.startswith("sha256=") else signature
        expected = verify(SIGNING_SECRET, timestamp, body)
        ok = hmac.compare_digest(expected, received)

        try:
            parsed = json.loads(body)
            event_type = parsed.get("event_type", "?")
            event_id = parsed.get("event_id", "?")
            data = parsed.get("data", {})
        except json.JSONDecodeError:
            event_type = event_id = "unparseable"
            data = {}

        status = "VERIFIED" if ok else "SIGNATURE FAILED"
        print(f"[{status}] {event_type} event_id={event_id} data={json.dumps(data)}", flush=True)

        if ok:
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"OK")
        else:
            self.send_response(401)
            self.end_headers()
            self.wfile.write(b"invalid signature")


def main():
    server = HTTPServer(("0.0.0.0", PORT), Handler)
    print(f"mock webhook receiver listening on :{PORT}", flush=True)
    print(f"verifying with secret prefix {SIGNING_SECRET[:10]}...", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()
        sys.exit(0)


if __name__ == "__main__":
    main()
