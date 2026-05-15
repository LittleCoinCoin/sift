#!/usr/bin/env python3
"""
Mock update server for testing the Sift updater notification flow.

What this tests:
  - The ~4 s startup auto-check fires and shows "Update 0.1.4 available" toast
  - The Install action button is present on the toast
  - Manually re-triggering via Sift → Check for Updates… works
  - Clicking Install will fail with a signature error (fake artifact) — expected.
    For full install testing you need a real signed artifact.

How to run:
  Terminal 1:  python3 scripts/serve-mock-update.py
  Terminal 2:  pnpm tauri dev

Pre-conditions (test/updater-flow branch already handles these):
  - tauri.conf.json version is 0.0.1  (app appears older than the mock)
  - tauri.conf.json endpoint points to http://localhost:1430/latest.json
  - tauri.conf.json pubkey is the real public key from pnpm tauri signer generate
"""

import http.server
import json
import datetime

PORT = 1430

# Manifest claiming version 0.1.4 — higher than the app's 0.0.1, so check() returns an Update.
# The signature is a dummy; it is only validated during downloadAndInstall(), not during check().
# Clicking Install will therefore trigger an error toast — that is intentional for this test.
MANIFEST = {
    "version": "0.1.4",
    "notes": "Local mock — testing updater notification flow only",
    "pub_date": "2025-01-01T00:00:00Z",
    "platforms": {
        "darwin-aarch64": {
            "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IGZha2Ugc2lnbmF0dXJlIGZvciB0ZXN0aW5nCmZha2VzaWduYXR1cmVmYWtlc2lnbmF0dXJlZmFrZXNpZ25hdHVyZQ==",
            "url": "http://localhost:1430/fake-sift.app.tar.gz"
        },
        "darwin-x86_64": {
            "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IGZha2Ugc2lnbmF0dXJlIGZvciB0ZXN0aW5nCmZha2VzaWduYXR1cmVmYWtlc2lnbmF0dXJlZmFrZXNpZ25hdHVyZQ==",
            "url": "http://localhost:1430/fake-sift.app.tar.gz"
        }
    }
}


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/latest.json":
            body = json.dumps(MANIFEST, indent=2).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            print(f"  -> served latest.json (version {MANIFEST['version']})")
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, fmt, *args):
        ts = datetime.datetime.now().strftime("%H:%M:%S")
        print(f"[mock-server {ts}] {fmt % args}")


if __name__ == "__main__":
    print(f"Mock update server listening on http://localhost:{PORT}")
    print(f"  Manifest version : {MANIFEST['version']}")
    print(f"  App version      : 0.0.1  (set in tauri.conf.json on this branch)")
    print()
    print("Expected flow after `pnpm tauri dev`:")
    print("  ~4 s  → 'Update 0.1.4 available' toast with [Install] button")
    print("  Install click → download error toast  (fake artifact — expected)")
    print("  Sift → Check for Updates… → re-triggers the same flow")
    print()
    print("Press Ctrl+C to stop.\n")

    # HTTPServer sets allow_reuse_address=True — avoids "Address already in use"
    # when restarting the server quickly after a previous run.
    with http.server.HTTPServer(("127.0.0.1", PORT), Handler) as httpd:
        httpd.serve_forever()
