use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier string as returned by the `/models` endpoint (e.g. `"gpt-4o"`).
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

/// Validate that a URL is not empty and has a valid scheme (http/https).
fn validate_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("API endpoint URL is empty. Please configure a valid URL in settings.".to_string());
    }

    let lower = trimmed.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!(
            "API endpoint URL must start with 'http://' or 'https://'. Got: {}",
            trimmed
        ));
    }

    Ok(())
}

/// Ping the `/models` endpoint of an OpenAI-compatible API to check reachability.
///
/// # Errors
/// Returns an error string if the URL fails validation or the HTTP request fails.
#[tauri::command]
pub async fn ping_endpoint(url: String) -> Result<bool, String> {
    validate_url(&url)?;
    let client = Client::new();
    let target = format!("{}/models", url.trim_end_matches('/'));
    client
        .get(&target)
        .send()
        .await
        .map(|r| r.status().is_success())
        .map_err(|e| e.to_string())
}

/// List models available at the given OpenAI-compatible endpoint.
///
/// # Errors
/// Returns an error string if the URL fails validation, the HTTP request fails,
/// the response status is non-2xx, or the response body cannot be parsed as a models list.
#[tauri::command]
pub async fn list_models(url: String, key: String) -> Result<Vec<ModelInfo>, String> {
    validate_url(&url)?;
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

    #[test]
    fn validate_url_accepts_http() {
        assert!(validate_url("http://localhost:11434").is_ok());
    }

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://api.openai.com").is_ok());
    }

    #[test]
    fn validate_url_rejects_empty() {
        let result = validate_url("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn validate_url_rejects_invalid_scheme() {
        let result = validate_url("ftp://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("http://"));
    }

    #[test]
    fn validate_url_rejects_no_scheme() {
        let result = validate_url("localhost:11434");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("http://"));
    }
}
