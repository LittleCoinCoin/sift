use crate::scan::ReceiptFile;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptRecord {
    pub date: String,
    pub category: String,
    pub entity: String,
    pub amount: String,
    pub payment_method: String,
    pub source_path: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
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

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: String,
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

#[derive(Deserialize)]
struct ExtractedFields {
    #[serde(default)]
    date: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    entity: String,
    #[serde(default)]
    amount: String,
    #[serde(default)]
    payment_method: String,
}

const PROMPT: &str = "Extract receipt information and return ONLY a JSON object with these exact keys: \
    date (ISO 8601 date), category (e.g. Food, Travel, Office), entity (merchant name), \
    amount (numeric string with currency symbol), payment_method (e.g. Cash, Visa, MasterCard). \
    Return nothing else — just the JSON object.";

pub async fn process_receipt(
    file: ReceiptFile,
    api_url: &str,
    model: &str,
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

    let request = ChatRequest {
        model,
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
        response_format: Some(ResponseFormat {
            kind: "json_object".into(),
        }),
    };

    let url = format!("{}/chat/completions", api_url.trim_end_matches('/'));
    let response = Client::new()
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("API error {}: {}", status, body));
    }

    let chat: ChatResponse = response.json().await?;
    let content = chat
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("empty choices in API response"))?
        .message
        .content;

    let fields: ExtractedFields = serde_json::from_str(&content)
        .map_err(|e| anyhow!("failed to parse model response as JSON: {}\n{}", e, content))?;

    Ok(ReceiptRecord {
        date: fields.date,
        category: fields.category,
        entity: fields.entity,
        amount: fields.amount,
        payment_method: fields.payment_method,
        source_path,
    })
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
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
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
