mod export;
mod job_control;
mod keyring_store;
mod logger;
mod ocr_client;
pub mod receipt;
pub mod receipt_index;
pub mod scan;
mod settings;

use logger::{emit_log, LogLevel};
use receipt::ReceiptRecord;


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
async fn open_directory_picker(app: tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
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

fn handle_receipt_uri(
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let raw_path = request.uri().path().to_string();
    let file_path = urlencoding::decode(&raw_path)
        .map(|s| s.into_owned())
        .unwrap_or(raw_path);

    tauri::async_runtime::spawn_blocking(move || {
        let path = std::path::Path::new(&file_path);
        if !path.exists() {
            responder.respond(
                tauri::http::Response::builder()
                    .status(404)
                    .body(b"Not found".to_vec())
                    .unwrap(),
            );
            return;
        }
        match receipt::render_pdf_first_page(path) {
            Ok(png_bytes) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .header("Content-Type", "image/png")
                        .body(png_bytes)
                        .unwrap(),
                );
            }
            Err(e) => {
                responder.respond(
                    tauri::http::Response::builder()
                        .status(500)
                        .body(e.to_string().into_bytes())
                        .unwrap(),
                );
            }
        }
    });
}

#[tauri::command]
async fn load_receipt_index(app: tauri::AppHandle) -> Result<receipt_index::ReceiptIndex, String> {
    receipt_index::load_index(&app).await
}

#[tauri::command]
async fn save_receipt_index(
    app: tauri::AppHandle,
    index: receipt_index::ReceiptIndex,
) -> Result<(), String> {
    receipt_index::save_index(&app, &index).await
}

#[tauri::command]
async fn update_receipt_fields(
    app: tauri::AppHandle,
    source_path: String,
    fields: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let mut index = receipt_index::load_index(&app).await?;
    receipt_index::update_entry_fields(&mut index, &source_path, fields)?;
    receipt_index::save_index(&app, &index).await
}

#[tauri::command]
async fn delete_receipt_files(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut deleted: Vec<String> = Vec::new();
    let mut index = receipt_index::load_index(&app).await.unwrap_or_default();
    for path in &paths {
        match std::fs::remove_file(path) {
            Ok(_) => {
                index.remove(path);
                deleted.push(path.clone());
                emit_log(&app, LogLevel::Info, format!("Deleted {}", path));
            }
            Err(e) => {
                emit_log(&app, LogLevel::Error, format!("Delete failed for {}: {}", path, e));
            }
        }
    }
    if !deleted.is_empty() {
        let _ = receipt_index::save_index(&app, &index).await;
    }
    Ok(deleted)
}

// === Job control commands ===

#[tauri::command]
async fn start_job(
    app: tauri::AppHandle,
    registry: tauri::State<'_, job_control::JobRegistry>,
    config: job_control::JobConfig,
) -> Result<String, String> {
    let handle = job_control::spawn_job(app, config);
    let job_id = handle.job_id.clone();
    registry.0.lock().map_err(|e| e.to_string())?.insert(job_id.clone(), handle);
    Ok(job_id)
}

#[tauri::command]
async fn pause_job(
    registry: tauri::State<'_, job_control::JobRegistry>,
    job_id: String,
) -> Result<(), String> {
    let guard = registry.0.lock().map_err(|e| e.to_string())?;
    let handle = guard.get(&job_id).ok_or_else(|| format!("Job {} not found", job_id))?;
    handle.pause()
}

#[tauri::command]
async fn resume_job(
    registry: tauri::State<'_, job_control::JobRegistry>,
    job_id: String,
) -> Result<(), String> {
    let guard = registry.0.lock().map_err(|e| e.to_string())?;
    let handle = guard.get(&job_id).ok_or_else(|| format!("Job {} not found", job_id))?;
    handle.resume()
}

#[tauri::command]
async fn cancel_job(
    registry: tauri::State<'_, job_control::JobRegistry>,
    job_id: String,
) -> Result<(), String> {
    let guard = registry.0.lock().map_err(|e| e.to_string())?;
    let handle = guard.get(&job_id).ok_or_else(|| format!("Job {} not found", job_id))?;
    handle.cancel()
}

#[tauri::command]
async fn get_job_state(
    registry: tauri::State<'_, job_control::JobRegistry>,
    job_id: String,
) -> Result<job_control::JobSummary, String> {
    let guard = registry.0.lock().map_err(|e| e.to_string())?;
    let handle = guard.get(&job_id).ok_or_else(|| format!("Job {} not found", job_id))?;
    Ok(handle.summary())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use pdfium_render::prelude::Pdfium;
            use tauri::Manager;
            let lib_name = Pdfium::pdfium_platform_library_name();
            let dylib_path = if tauri::is_dev() {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(lib_name)
            } else {
                app.path().resource_dir()?.join(lib_name)
            };
            receipt::init_pdfium_path(dylib_path);
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("receipt", |_ctx, request, responder| {
            handle_receipt_uri(request, responder);
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(job_control::JobRegistry::default())
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
            scan::scan_all_receipt_dirs,
            export_csv,
            read_image_base64,
            render_pdf_preview,
            open_directory_picker,
            load_receipt_index,
            save_receipt_index,
            update_receipt_fields,
            delete_receipt_files,
            start_job,
            pause_job,
            resume_job,
            cancel_job,
            get_job_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
