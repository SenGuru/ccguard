"""Mock Anthropic Messages endpoint for verifying the triage pipeline end-to-end.
Returns a real Messages-API-shaped reply; the verdict is derived from the
assembled session context so the dashboard shows a realistic work/personal/unsure
mix. Everything EXCEPT the actual Claude model is the real server code path."""
import json
from http.server import BaseHTTPRequestHandler, HTTPServer

WORK = ("ccguard", "claresso", "erode", "axo", "attend", "altkey", "grove",
        "src/", "cargo", "rust", "migration", "dashboard", "/desktop/2027")
PERSONAL = ("portfolio", "resume", "cover letter", "leetcode", "hobby",
            "personal site", "my blog", "side project", "wedding")

def verdict(ctx: str):
    low = ctx.lower()
    if any(k in low for k in PERSONAL):
        return ("personal", 0.78, "References a personal side-project / job-hunt artifact.")
    if any(k in low for k in WORK):
        return ("work", 0.86, "Edits source in the user's own product repos / tooling.")
    return ("unsure", 0.5, "No clear work or personal signal in the captured context.")

class H(BaseHTTPRequestHandler):
    def log_message(self, *a):  # quiet
        pass
    def do_POST(self):
        n = int(self.headers.get("content-length", 0))
        body = self.rfile.read(n).decode("utf-8", "replace")
        try:
            req = json.loads(body)
            ctx = req["messages"][0]["content"]
        except Exception:
            ctx = ""
        label, conf, reason = verdict(ctx)
        inner = json.dumps({"label": label, "confidence": conf, "reason": reason})
        reply = {
            "id": "msg_mock", "type": "message", "role": "assistant",
            "model": req.get("model", "claude-haiku-4-5") if 'req' in dir() else "claude-haiku-4-5",
            "content": [{"type": "text", "text": inner}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 200, "output_tokens": 40},
        }
        out = json.dumps(reply).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

if __name__ == "__main__":
    HTTPServer(("127.0.0.1", 8899), H).serve_forever()
