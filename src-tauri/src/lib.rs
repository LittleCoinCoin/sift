mod export;
mod keyring_store;
mod logger;
mod ocr_client;
mod receipt;
mod scan;
mod settings;

use logger::{emit_log, LogLevel, ProgressEvent};
use receipt::ReceiptRecord;
use scan::ReceiptFile;
use tauri::Emitter;

#[tauri::command]
async fn scan_receipts(
    app: tauri::AppHandle,
    dir: String,
) -> Result<Vec<serde_json::Value>, String> {
    emit_log(&app, LogLevel::Info, format!("Scanning {}", dir));
    let path = std::path::Path::new(&dir);
    let files = scan::list_receipts(path).map_err(|e| {
        emit_log(&app, LogLevel::Error, format!("Scan failed: {}", e));
        e.to_string()
    })?;
    emit_log(&app, LogLevel::Success, format!("Found {} receipt(s)", files.len()));
    let result = files
        .into_iter()
        .map(|f| match f {
            ReceiptFile::Image(p) => serde_json::json!({
                "type": "image",
                "path": p.to_string_lossy()
            }),
            ReceiptFile::Pdf(p) => serde_json::json!({
                "type": "pdf",
                "path": p.to_string_lossy()
            }),
        })
        .collect();
    Ok(result)
}

#[tauri::command]
async fn process_receipt(
    app: tauri::AppHandle,
    path: String,
    file_type: String,
    api_url: String,
    ocr_model: String,
    extraction_model: String,
    extraction_url: String,
    extraction_api_key: String,
    active_system_prompt: String,
    json_schema_keys: Vec<String>,
    api_key: String,
    done: Option<u32>,
    total: Option<u32>,
) -> Result<ReceiptRecord, String> {
    let p = std::path::PathBuf::from(&path);
    let file = match file_type.as_str() {
        "pdf" => ReceiptFile::Pdf(p),
        _ => ReceiptFile::Image(p),
    };
    emit_log(&app, LogLevel::Info, format!("Processing {}", path));
    let start = std::time::Instant::now();
    let record = receipt::process_receipt(
        file,
        &api_url,
        &api_key,
        &ocr_model,
        &extraction_url,
        &extraction_api_key,
        &extraction_model,
        &active_system_prompt,
        &json_schema_keys,
    )
        .await
        .map_err(|e| {
            emit_log(&app, LogLevel::Error, format!("Failed to process {}: {}", path, e));
            e.to_string()
        })?;
    let elapsed_ms = start.elapsed().as_millis() as f64;
    emit_log(&app, LogLevel::Success, format!("Processed {}", path));
    if let (Some(d), Some(t)) = (done, total) {
        let _ = app.emit("progress", ProgressEvent { done: d, total: t, avg_ms: elapsed_ms });
    }
    Ok(record)
}

#[tauri::command]
fn export_csv(
    app: tauri::AppHandle,
    records: Vec<ReceiptRecord>,
    keys: Vec<String>,
    output_path: String,
) -> Result<(), String> {
    emit_log(&app, LogLevel::Info, format!("Exporting {} record(s) to {}", records.len(), output_path));
    let resolved_keys = if keys.is_empty() {
        export::default_keys()
    } else {
        keys
    };
    export::export_csv(records, resolved_keys, std::path::Path::new(&output_path))
        .map_err(|e| {
            emit_log(&app, LogLevel::Error, format!("Export failed: {}", e));
            e.to_string()
        })?;
    emit_log(&app, LogLevel::Success, format!("Exported to {}", output_path));
    Ok(())
}

#[tauri::command]
fn render_pdf_preview(path: String) -> Result<String, String> {
    let bytes = receipt::render_pdf_first_page(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    use base64::Engine as _;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

#[tauri::command]
fn read_image_base64(path: String) -> Result<String, String> {
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png"  => "image/png",
        "webp" => "image/webp",
        "gif"  => "image/gif",
        _      => "image/jpeg",
    };
    use base64::Engine as _;
    Ok(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(&data)
    ))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::save_settings,
            keyring_store::set_api_key,
            keyring_store::get_api_key,
            keyring_store::delete_api_key,
            keyring_store::set_extraction_api_key,
            keyring_store::get_extraction_api_key,
            keyring_store::delete_extraction_api_key,
            ocr_client::ping_endpoint,
            ocr_client::list_models,
            scan_receipts,
            process_receipt,
            export_csv,
            read_image_base64,
            render_pdf_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
