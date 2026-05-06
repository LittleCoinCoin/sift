//! Programmatic end-to-end tests for the OCR + extraction pipeline.
//!
//! These tests exercise `receipt::process_receipt` without going through the
//! Tauri IPC layer, so they catch regressions like the "builder error" caused
//! by empty / malformed URLs.
//!
//! Two groups of tests are provided:
//!
//!   1. Default tests use a `wiremock` mock HTTP server and always run.
//!   2. An `#[ignore]`d "live" test hits a real OCR endpoint. It reads its
//!      configuration from env vars (with defaults provided by
//!      `src-tauri/.cargo/config.toml`'s `[env]` section), so the URL and API
//!      key are customizable without editing source code. Run it with:
//!          cargo test --test ocr_pipeline -- --ignored live_pipeline
//!
//! The configurable env vars are:
//!   OCR_TEST_API_URL, OCR_TEST_API_KEY, OCR_TEST_OCR_MODEL,
//!   OCR_TEST_EXTRACTION_URL, OCR_TEST_EXTRACTION_API_KEY,
//!   OCR_TEST_EXTRACTION_MODEL.

use std::io::Cursor;
use std::path::PathBuf;

use sift_lib::receipt::{process_receipt, ReceiptRecord};
use sift_lib::scan::ReceiptFile;
use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Write a tiny valid PNG to `dir/receipt.png` and return its path.
fn write_tiny_png(dir: &std::path::Path) -> PathBuf {
    let img = image::RgbImage::from_pixel(2, 2, image::Rgb([255, 255, 255]));
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("encode test png");
    let p = dir.join("receipt.png");
    std::fs::write(&p, &bytes).expect("write test png");
    p
}

fn schema_keys() -> Vec<String> {
    vec!["vendor".into(), "total".into(), "date".into()]
}

fn system_prompt() -> &'static str {
    "Extract fields: { {schema_keys} }"
}

fn ok_chat_response(content: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{
            "message": { "role": "assistant", "content": content }
        }]
    }))
}

/// End-to-end pipeline run against a mock server. Verifies that the pipeline
/// does not produce a reqwest "builder error" for valid URLs and returns the
/// expected extracted fields.
#[tokio::test]
async fn pipeline_succeeds_against_mock_server() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ok_chat_response(
            "Vendor: ACME\nTotal: 12.34\nDate: 2026-04-22",
        ))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer extraction-key"))
        .respond_with(ok_chat_response(
            "```json\n{\"vendor\":\"ACME\",\"total\":\"12.34\",\"date\":\"2026-04-22\"}\n```",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let png = write_tiny_png(tmp.path());
    let keys = schema_keys();

    let record: ReceiptRecord = process_receipt(
        ReceiptFile::Image(png.clone()),
        &server.uri(),
        "test-key",
        "test-ocr-model",
        &server.uri(),
        "extraction-key",
        "test-extraction-model",
        system_prompt(),
        &keys,
    )
    .await
    .expect("pipeline should succeed");

    assert_eq!(record.source_path, png.to_string_lossy());
    assert_eq!(record.fields.get("vendor").map(|s| s.as_str()), Some("ACME"));
    assert_eq!(record.fields.get("total").map(|s| s.as_str()), Some("12.34"));
    assert_eq!(
        record.fields.get("date").map(|s| s.as_str()),
        Some("2026-04-22")
    );
}

/// Regression test for the "builder error" symptom: an empty OCR URL used to
/// bubble up as a reqwest builder error. It must now surface as a clear,
/// user-facing validation error before any HTTP call is attempted.
#[tokio::test]
async fn pipeline_rejects_empty_ocr_url() {
    let tmp = TempDir::new().unwrap();
    let png = write_tiny_png(tmp.path());

    let err = process_receipt(
        ReceiptFile::Image(png),
        "",
        "k",
        "m",
        "http://localhost:1/v1",
        "k",
        "m",
        system_prompt(),
        &schema_keys(),
    )
    .await
    .expect_err("empty OCR URL should error");

    let msg = err.to_string();
    assert!(
        !msg.contains("builder error"),
        "should not surface opaque builder error, got: {msg}"
    );
    assert!(msg.to_lowercase().contains("empty"), "got: {msg}");
}

#[tokio::test]
async fn pipeline_rejects_empty_extraction_url() {
    let tmp = TempDir::new().unwrap();
    let png = write_tiny_png(tmp.path());

    let err = process_receipt(
        ReceiptFile::Image(png),
        "http://localhost:1/v1",
        "k",
        "m",
        "",
        "k",
        "m",
        system_prompt(),
        &schema_keys(),
    )
    .await
    .expect_err("empty extraction URL should error");

    let msg = err.to_string();
    assert!(!msg.contains("builder error"), "got: {msg}");
    assert!(msg.to_lowercase().contains("empty"), "got: {msg}");
}

#[tokio::test]
async fn pipeline_rejects_url_without_scheme() {
    let tmp = TempDir::new().unwrap();
    let png = write_tiny_png(tmp.path());

    let err = process_receipt(
        ReceiptFile::Image(png),
        "localhost:11434/v1",
        "k",
        "m",
        "http://localhost:1/v1",
        "k",
        "m",
        system_prompt(),
        &schema_keys(),
    )
    .await
    .expect_err("scheme-less URL should error");

    let msg = err.to_string();
    assert!(!msg.contains("builder error"), "got: {msg}");
    assert!(msg.contains("http://") || msg.contains("scheme"), "got: {msg}");
}

/// When the OCR endpoint returns non-200, the pipeline must surface a clear
/// error instead of panicking or producing a builder error.
#[tokio::test]
async fn pipeline_surfaces_ocr_http_errors() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let png = write_tiny_png(tmp.path());

    let err = process_receipt(
        ReceiptFile::Image(png),
        &server.uri(),
        "k",
        "m",
        &server.uri(),
        "k",
        "m",
        system_prompt(),
        &schema_keys(),
    )
    .await
    .expect_err("HTTP 500 should error");

    let msg = err.to_string();
    assert!(msg.contains("OCR API error"), "got: {msg}");
    assert!(msg.contains("500"), "got: {msg}");
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Live end-to-end test against a real OCR endpoint. Ignored by default; run
/// with `cargo test --test ocr_pipeline -- --ignored live_pipeline`.
///
/// Configuration is read from env vars. Defaults come from
/// `src-tauri/.cargo/config.toml`'s `[env]` block.
#[tokio::test]
#[ignore]
async fn live_pipeline() {
    let api_url = env_or("OCR_TEST_API_URL", "http://localhost:11434/v1");
    let api_key = env_or("OCR_TEST_API_KEY", "");
    let ocr_model = env_or("OCR_TEST_OCR_MODEL", "lightonocr");
    let extraction_url = env_or("OCR_TEST_EXTRACTION_URL", &api_url);
    let extraction_api_key = env_or("OCR_TEST_EXTRACTION_API_KEY", &api_key);
    let extraction_model = env_or("OCR_TEST_EXTRACTION_MODEL", "qwen2.5");

    let tmp = TempDir::new().unwrap();
    let png = write_tiny_png(tmp.path());

    let record = process_receipt(
        ReceiptFile::Image(png),
        &api_url,
        &api_key,
        &ocr_model,
        &extraction_url,
        &extraction_api_key,
        &extraction_model,
        system_prompt(),
        &schema_keys(),
    )
    .await
    .expect("live pipeline should succeed — check OCR_TEST_* env vars");

    assert!(!record.source_path.is_empty());
}
