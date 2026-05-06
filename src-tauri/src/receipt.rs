use crate::scan::ReceiptFile;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Path to the pdfium shared library, resolved once at app startup.
/// Falls back to CARGO_MANIFEST_DIR at compile time when unset (dev mode).
static PDFIUM_LIB_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init_pdfium_path(path: PathBuf) {
    PDFIUM_LIB_PATH.set(path).ok();
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptRecord {
    /// Absolute path of the source receipt file on disk.
    pub source_path: String,
    /// Extracted key-value pairs, keyed by schema field name.
    pub fields: HashMap<String, String>,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    ImageUrl { image_url: ImageUrl },
    Text { text: String },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}


#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

const PROMPT: &str = "Transcribe all text visible on this receipt exactly as it appears. \
    Output only the raw text content — no commentary, no formatting, no JSON.";

const SCHEMA_KEYS_PLACEHOLDER: &str = "{schema_keys}";

/// Validate that a URL is not empty and has a valid scheme (http/https).
fn validate_url(url: &str, purpose: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(
            "{} endpoint URL is empty. Please configure a valid URL in settings.",
            purpose
        ));
    }

    let lower = trimmed.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(anyhow!(
            "{} endpoint URL must start with 'http://' or 'https://'. Got: {}",
            purpose,
            trimmed
        ));
    }

    Ok(())
}

/// Extract the first JSON object `{...}` from `s`, skipping any markdown fences.
///
/// Returns the substring from the first `{` to the last `}`, inclusive.
/// Returns an empty string if no `{` or `}` is found.
#[must_use]
pub fn extract_json(s: &str) -> String {
    let start = match s.find('{') {
        Some(i) => i,
        None => return String::new(),
    };
    let end = match s.rfind('}') {
        Some(i) => i,
        None => return String::new(),
    };
    if end < start {
        return String::new();
    }
    // INDEX: start/end are byte offsets returned by str::find/rfind on `s`, always valid UTF-8 boundaries
    s[start..=end].to_string()
}

/// Phase 1: send the file image to the OCR model and return the raw transcript.
///
/// Returns `(source_path, markdown_transcript)`.
///
/// # Errors
/// Returns an error if URL validation fails, the file cannot be read or rendered,
/// the HTTP request fails, the response is non-2xx, or the response body is empty.
pub async fn ocr_receipt(
    file: ReceiptFile,
    api_url: &str,
    api_key: &str,
    ocr_model: &str,
) -> Result<(String, String)> {
    validate_url(api_url, "OCR")?;

    let source_path = file.path().to_string_lossy().to_string();

    let (png_bytes, mime) = match &file {
        ReceiptFile::Image(path) => {
            let bytes = fs::read(path)?;
            let mime = image_mime(path.extension().and_then(|e| e.to_str()).unwrap_or(""));
            (bytes, mime)
        }
        ReceiptFile::Pdf(path) => {
            let bytes = render_pdf_first_page(path)?;
            (bytes, "image/png")
        }
    };

    let data_url = format!(
        "data:{};base64,{}",
        mime,
        general_purpose::STANDARD.encode(&png_bytes)
    );

    let ocr_request = ChatRequest {
        model: ocr_model,
        messages: vec![Message {
            role: "user".into(),
            content: vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrl { url: data_url },
                },
                ContentPart::Text {
                    text: PROMPT.into(),
                },
            ],
        }],
    };

    let client = Client::new();
    let ocr_chat_url = format!("{}/chat/completions", api_url.trim_end_matches('/'));

    let ocr_response = client
        .post(&ocr_chat_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&ocr_request)
        .send()
        .await?;

    if !ocr_response.status().is_success() {
        let status = ocr_response.status();
        let body = ocr_response.text().await.unwrap_or_default();
        return Err(anyhow!("OCR API error {}: {}", status, body));
    }

    let ocr_chat: ChatResponse = ocr_response.json().await?;
    let markdown = ocr_chat
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("empty choices in OCR API response"))?
        .message
        .content;

    Ok((source_path, markdown))
}

/// Phase 2: send the OCR transcript to the extraction model and return parsed fields.
///
/// # Errors
/// Returns an error if URL validation fails, the HTTP request fails, the response
/// is non-2xx, or the response body is empty. JSON parse failures are non-fatal
/// and produce a record with empty field values.
pub async fn extract_fields(
    markdown: String,
    source_path: String,
    extraction_url: &str,
    extraction_api_key: &str,
    extraction_model: &str,
    active_system_prompt: &str,
    json_schema_keys: &[String],
) -> Result<ReceiptRecord> {
    validate_url(extraction_url, "Text Processing")?;

    let key_list: Vec<String> = json_schema_keys
        .iter()
        .map(|c| format!("\"{c}\": \"...\""))
        .collect();
    let schema_keys_preview = key_list.join(", ");
    let system_prompt = active_system_prompt.replace(SCHEMA_KEYS_PLACEHOLDER, &schema_keys_preview);

    let extraction_request = ChatRequest {
        model: extraction_model,
        messages: vec![
            Message {
                role: "system".into(),
                content: vec![ContentPart::Text {
                    text: system_prompt,
                }],
            },
            Message {
                role: "user".into(),
                content: vec![ContentPart::Text {
                    text: markdown,
                }],
            },
        ],
    };

    let client = Client::new();
    let extraction_chat_url = format!(
        "{}/chat/completions",
        extraction_url.trim_end_matches('/')
    );

    let extraction_response = client
        .post(&extraction_chat_url)
        .header("Authorization", format!("Bearer {}", extraction_api_key))
        .json(&extraction_request)
        .send()
        .await?;

    if !extraction_response.status().is_success() {
        let status = extraction_response.status();
        let body = extraction_response.text().await.unwrap_or_default();
        return Err(anyhow!("Extraction API error {}: {}", status, body));
    }

    let extraction_chat: ChatResponse = extraction_response.json().await?;
    let raw_content = extraction_chat
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("empty choices in extraction API response"))?
        .message
        .content;

    let json_str = extract_json(&raw_content);
    let parsed_map: HashMap<String, String> = match serde_json::from_str(&json_str) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "Warning: failed to parse extraction response as JSON ({}). Raw: {}",
                e, raw_content
            );
            HashMap::new()
        }
    };

    let mut fields = HashMap::new();
    for key in json_schema_keys {
        fields.insert(key.clone(), parsed_map.get(key).cloned().unwrap_or_default());
    }

    Ok(ReceiptRecord { source_path, fields })
}

/// Convenience wrapper: validates both URLs upfront, then runs OCR → extraction
/// in sequence. Used by integration tests and any caller that does not need
/// per-phase progress events.
///
/// # Errors
/// Returns an error if either URL fails validation, or if the OCR or extraction
/// step returns an error (see [`ocr_receipt`] and [`extract_fields`]).
#[allow(clippy::too_many_arguments)]
pub async fn process_receipt(
    file: ReceiptFile,
    api_url: &str,
    api_key: &str,
    ocr_model: &str,
    extraction_url: &str,
    extraction_api_key: &str,
    extraction_model: &str,
    active_system_prompt: &str,
    json_schema_keys: &[String],
) -> Result<ReceiptRecord> {
    validate_url(api_url, "OCR")?;
    validate_url(extraction_url, "Text Processing")?;
    let (source_path, markdown) =
        ocr_receipt(file, api_url, api_key, ocr_model).await?;
    extract_fields(
        markdown,
        source_path,
        extraction_url,
        extraction_api_key,
        extraction_model,
        active_system_prompt,
        json_schema_keys,
    )
    .await
}

fn image_mime(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

/// Render the first page of a PDF file to a PNG byte buffer at up to 1200×1800 px.
///
/// # Errors
/// Returns an error if the `pdfium` library cannot be loaded, the PDF cannot be
/// opened, the first page cannot be retrieved or rendered, or PNG encoding fails.
pub fn render_pdf_first_page(path: &std::path::Path) -> Result<Vec<u8>> {
    use pdfium_render::prelude::*;

    let lib_path = PDFIUM_LIB_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| {
            // Dev fallback: dylib lives next to Cargo.toml
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(Pdfium::pdfium_platform_library_name())
        });

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(&lib_path)
            .map_err(|e| anyhow!("pdfium library not found at {:?}: {}", lib_path, e))?,
    );

    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| anyhow!("failed to load PDF: {}", e))?;

    let page = doc
        .pages()
        .get(0)
        .map_err(|e| anyhow!("failed to get first PDF page: {}", e))?;

    let config = PdfRenderConfig::new()
        .set_target_width(1200)
        .set_maximum_height(1800);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| anyhow!("failed to render PDF page: {}", e))?;

    let img = bitmap.as_image();
    let mut png_bytes = Vec::new();
    img.write_to(
        std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;
    Ok(png_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_plain() {
        assert_eq!(extract_json(r#"{"a":"b"}"#), r#"{"a":"b"}"#);
    }

    #[test]
    fn extract_json_fenced() {
        assert_eq!(
            extract_json("```json\n{\"a\":\"b\"}\n```"),
            "{\"a\":\"b\"}"
        );
    }

    #[test]
    fn extract_json_empty() {
        assert_eq!(extract_json("no json here"), "");
    }

    #[test]
    fn validate_url_accepts_http() {
        assert!(validate_url("http://localhost:11434", "OCR").is_ok());
    }

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://api.openai.com", "OCR").is_ok());
    }

    #[test]
    fn validate_url_rejects_empty() {
        let result = validate_url("", "OCR");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("empty"));
    }

    #[test]
    fn validate_url_rejects_invalid_scheme() {
        let result = validate_url("ftp://example.com", "OCR");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("http://"));
    }

    #[test]
    fn validate_url_rejects_no_scheme() {
        let result = validate_url("localhost:11434", "OCR");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("http://"));
    }
}
