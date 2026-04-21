# receipt-processor

A desktop app for OCR processing of receipts using a local or remote LLM API (OpenAI-compatible). Built with Tauri 2 + Svelte 5.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- An OpenAI-compatible LLM API (e.g. [Ollama](https://ollama.com/) with a vision model like `llava`)
- macOS: Xcode Command Line Tools (`xcode-select --install`)

## Setup

```bash
# Install JS dependencies
pnpm install

# Run in development mode (hot-reload)
pnpm tauri dev

# Build a release binary
pnpm tauri build
```

## First-time configuration

1. Click the **⚙ Settings** button in the top-right corner.
2. Enter your **API Endpoint** URL (e.g. `http://localhost:11434` for Ollama).
3. Click **Ping** to verify connectivity.
4. Optionally enter your **API Key** — it is stored in the system keychain and never written to disk in plain text.
5. Click **Load models** and select a vision-capable model (e.g. `llava`).
6. Optionally set your **Receipt Directory** so it pre-fills on launch.
7. Click **Save settings**.

## Usage

1. Enter a directory path in the left panel and click **Scan** (or press Enter) to load receipt images and PDFs.
2. Select a file from the list.
3. Click **Process** to run OCR — extracted fields appear in the right panel.
4. Edit any field inline as needed.
5. Click **Export CSV** to save all processed receipts to a CSV file. Column order follows the **CSV Columns** setting.

## CSV columns

Customize which fields appear in the export (and their order) via Settings → CSV Columns. Drag to reorder, click × to remove, or type a name and click **Add** for custom columns.
