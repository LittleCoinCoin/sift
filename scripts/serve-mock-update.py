#!/usr/bin/env python3
"""
Mock update server for Sift's updater tests (stdlib only).

Two modes:

  Notification only (no --artifact), for `pnpm tauri dev` smoke tests:
      uv run python scripts/serve-mock-update.py
    Serves a manifest with a dummy signature. The update toast appears, but
    clicking Install fails verification. Version defaults to 0.1.4.

  Real install (--artifact), used by scripts/e2e-updater.sh:
      uv run python scripts/serve-mock-update.py \\
          --artifact Sift.app.tar.gz --version 0.2.0 --port 1430
    Reads <artifact>.sig (the minisign signature, used verbatim as the
    manifest's `signature`) and serves the artifact with a Content-Length.

Endpoints:
  GET  /latest.json       update manifest (darwin-aarch64 and darwin-x86_64)
  GET  /artifact.tar.gz   the artifact (only with --artifact), streamed
  POST /e2e-log           append one line, timestamped (text/plain body)
  GET  /e2e-log           dump every line collected so far

Every request is logged to stderr with a timestamp.
"""

import argparse
import datetime
import http.server
import json
import pathlib
import sys
import threading
import time

# Signature of the notification-only mode: only validated by downloadAndInstall().
DUMMY_SIGNATURE = (
    "dW50cnVzdGVkIGNvbW1lbnQ6IGZha2Ugc2lnbmF0dXJlIGZvciB0ZXN0aW5nCmZha2VzaWduYXR1"
    "cmVmYWtlc2lnbmF0dXJlZmFrZXNpZ25hdHVyZQ=="
)
ARTIFACT_PATH = "/artifact.tar.gz"
CHUNK_SIZE = 64 * 1024

LOG_LINES: list[str] = []
LOG_LOCK = threading.Lock()


def now() -> str:
    return datetime.datetime.now().strftime("%H:%M:%S.%f")[:-3]


class Handler(http.server.BaseHTTPRequestHandler):
    # Set by main() before serving.
    manifest: dict = {}
    artifact: pathlib.Path | None = None
    chunk_delay: float = 0.0

    def _reply(self, status: int, body: bytes, content_type: str) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        if self.path == "/latest.json":
            body = json.dumps(self.manifest, indent=2).encode()
            self._reply(200, body, "application/json")
        elif self.path == "/e2e-log":
            with LOG_LOCK:
                body = ("\n".join(LOG_LINES) + ("\n" if LOG_LINES else "")).encode()
            self._reply(200, body, "text/plain; charset=utf-8")
        elif self.path == ARTIFACT_PATH and self.artifact is not None:
            self._stream_artifact(self.artifact)
        else:
            self._reply(404, b"not found", "text/plain")

    def _stream_artifact(self, path: pathlib.Path) -> None:
        size = path.stat().st_size
        self.send_response(200)
        self.send_header("Content-Type", "application/gzip")
        self.send_header("Content-Length", str(size))
        self.end_headers()
        try:
            with path.open("rb") as fh:
                while chunk := fh.read(CHUNK_SIZE):
                    self.wfile.write(chunk)
                    if self.chunk_delay:
                        time.sleep(self.chunk_delay)
        except (BrokenPipeError, ConnectionResetError):
            print(f"[mock-server {now()}] client aborted the artifact download", file=sys.stderr, flush=True)

    def do_POST(self) -> None:
        if self.path != "/e2e-log":
            self._reply(404, b"not found", "text/plain")
            return
        length = int(self.headers.get("Content-Length", "0"))
        line = self.rfile.read(length).decode("utf-8", errors="replace").strip()
        stamped = f"{now()} {line}"
        with LOG_LOCK:
            LOG_LINES.append(stamped)
        print(f"[e2e-log] {stamped}", file=sys.stderr, flush=True)
        self._reply(204, b"", "text/plain")

    def do_OPTIONS(self) -> None:
        # The driver sends simple requests, but answer a preflight anyway.
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, fmt: str, *args) -> None:
        print(f"[mock-server {now()}] {self.address_string()} {fmt % args}", file=sys.stderr, flush=True)


def build_manifest(version: str, port: int, signature: str, has_artifact: bool) -> dict:
    url = f"http://127.0.0.1:{port}{ARTIFACT_PATH if has_artifact else '/fake-sift.app.tar.gz'}"
    entry = {"signature": signature, "url": url}
    return {
        "version": version,
        "notes": "Local mock for the Sift updater tests",
        "pub_date": "2025-01-01T00:00:00Z",
        "platforms": {"darwin-aarch64": dict(entry), "darwin-x86_64": dict(entry)},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--artifact", type=pathlib.Path, help="signed .app.tar.gz to serve; <artifact>.sig must exist")
    parser.add_argument("--version", default="0.1.4", help="version the manifest claims (default 0.1.4)")
    parser.add_argument("--port", type=int, default=1430)
    parser.add_argument(
        "--chunk-delay-ms",
        type=int,
        default=0,
        help="pause after every 64 KiB of the artifact, so download progress is observable",
    )
    args = parser.parse_args()

    if args.artifact is not None:
        sig_path = args.artifact.with_name(args.artifact.name + ".sig")
        if not args.artifact.is_file() or not sig_path.is_file():
            parser.error(f"need both {args.artifact} and {sig_path}")
        signature = sig_path.read_text()  # verbatim: the .sig content is the manifest value
        Handler.artifact = args.artifact
    else:
        signature = DUMMY_SIGNATURE
    Handler.manifest = build_manifest(args.version, args.port, signature, args.artifact is not None)
    Handler.chunk_delay = args.chunk_delay_ms / 1000.0

    mode = f"artifact {args.artifact}" if args.artifact else "notification only (dummy signature)"
    print(f"Mock update server on http://127.0.0.1:{args.port}  version {args.version}  {mode}", file=sys.stderr, flush=True)
    # allow_reuse_address (set by HTTPServer) avoids "Address already in use" on quick restarts.
    with http.server.ThreadingHTTPServer(("127.0.0.1", args.port), Handler) as httpd:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
