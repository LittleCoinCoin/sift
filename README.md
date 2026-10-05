# Sift

Point Sift at a folder of documents, tell it which fields to extract, and get a CSV. It sends each document through a configurable LLM endpoint (local or remote) for OCR and field extraction — no cloud lock-in, no fixed schema. Receipts are one use case; invoices, forms, lab reports, or any other structured document collection work just as well.

<!-- screenshot -->

---

## Download & install

1. Go to the [Releases](../../releases) page and download the `.dmg` for your Mac:
   - **Apple Silicon** (any M-series chip): `Sift_*_aarch64.dmg`
   - **Intel**: `Sift_*_x64.dmg`
2. Open the `.dmg` and drag **Sift** into `/Applications`.
3. **First launch only** — Sift is not signed and notarized through Apple's developer pipeline, so macOS will block it on first open as a security precaution. To allow it: open **System Settings → Privacy & Security**, scroll to the bottom, and click **Open Anyway**.
   After that, Sift opens normally with a double-click.

### Updates

From version 0.2.0 on, Sift updates itself. About four seconds after launch, and whenever you choose **Sift → Check for Updates…**, it looks for a newer release. If one exists, an **Install** button appears. Sift then downloads and verifies the update, installs it, and offers **Restart Now**.

- **Coming from 0.1.4 or earlier?** Those versions have no updater. Download and install 0.2.0 once as described above; later updates arrive inside the app.
- **Run Sift from `/Applications`, not from the mounted `.dmg`.** An app running from the disk image cannot replace itself, and Sift will tell you to move it first.

---

## Configuration

Open **Settings** (gear icon, top-right) and fill in:

| Setting | What it is |
|---|---|
| OCR endpoint | Base URL of your LLM API (e.g. `http://localhost:11434/v1`) |
| OCR model | Model name to use for OCR (e.g. `lightonocr`) |
| Extraction endpoint | Base URL for field extraction (can be the same endpoint) |
| Extraction model | Model name for extraction (e.g. `qwen2.5`) |
| API key | Bearer token, if your endpoint requires one |

Both endpoints follow the OpenAI-compatible chat completions API. Any local server that speaks that protocol (Ollama, LM Studio, llama.cpp server, etc.) works out of the box.

---

## Building from source

```sh
# Prerequisites: Rust, Node.js ≥ 22, pnpm
pnpm install
pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'
# Output: src-tauri/target/release/bundle/dmg/
```

Release builds also produce signed update archives, which needs the updater's private signing key (`TAURI_SIGNING_PRIVATE_KEY`). The `--config` override above skips those archives, so you can build without the key. A local build cannot ship updates to other installs anyway.

`libpdfium.dylib` is downloaded automatically during the build for the correct target architecture.
