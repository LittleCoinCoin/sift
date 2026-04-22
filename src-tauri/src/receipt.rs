use crate::scan::ReceiptFile;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptRecord {
    pub source_path: String,
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

/// Extract the first JSON object `{...}` from `s`, skipping any markdown fences.
/// Returns the substring from the first `{` to the last `}`, inclusive.
/// Returns an empty string if no `{` or `}` is found.
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
    s[start..=end].to_string()
}

pub async fn process_receipt(
    file: ReceiptFile,
    api_url: &str,
    ocr_model: &str,
    extraction_model: &str,
    csv_columns: &[String],
    api_key: &str,
) -> Result<ReceiptRecord> {
    let source_path = file
        .path()
        .to_string_lossy()
        .to_string();

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

    let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));
    let client = Client::new();

    let ocr_response = client
        .post(&url)
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

    let column_list: Vec<String> = csv_columns
        .iter()
        .map(|c| format!("\"{c}\": \"...\""))
        .collect();
    let columns_preview = column_list.join(", ");

    let system_prompt = format!(
        "You are a data extraction assistant. Given a receipt transcription, extract ONLY these fields and return a single JSON object with no other text:\n  {{ {} }}\nFor any field you cannot find, use an empty string.",
        columns_preview
    );

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

    let extraction_response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
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
    for col in csv_columns {
        fields.insert(col.clone(), parsed_map.get(col).cloned().unwrap_or_default());
    }

    Ok(ReceiptRecord { source_path, fields })
}

fn image_mime(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

pub fn render_pdf_first_page(path: &std::path::Path) -> Result<Vec<u8>> {
    use pdfium_render::prelude::*;

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(Pdfium::pdfium_platform_library_name()),
        )
        .map_err(|e| anyhow!("pdfium library not found: {}", e))?,
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
}
