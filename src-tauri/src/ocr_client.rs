use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[tauri::command]
pub async fn ping_endpoint(url: String) -> Result<bool, String> {
    let client = Client::new();
    let target = format!("{}/models", url.trim_end_matches('/'));
    client
        .get(&target)
        .send()
        .await
        .map(|r| r.status().is_success())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_models(url: String, key: String) -> Result<Vec<ModelInfo>, String> {
    let client = Client::new();
    let target = format!("{}/models", url.trim_end_matches('/'));
    let response = client
        .get(&target)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let models: ModelsResponse = response.json().await.map_err(|e| e.to_string())?;
    Ok(models.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_info_serializes() {
        let m = ModelInfo { id: "gpt-4".into() };
        let json = serde_json::to_string(&m).unwrap();
        let back: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "gpt-4");
    }

    #[test]
    fn url_trailing_slash_stripped() {
        let url = "http://localhost:11434/";
        let target = format!("{}/models", url.trim_end_matches('/'));
        assert_eq!(target, "http://localhost:11434/models");
    }
}
