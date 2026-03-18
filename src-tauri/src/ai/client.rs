use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use crate::ai::{RawAnalysis, AnalysisResult};
use crate::error::AppError;

pub struct OllamaClient {
    http: Client,
    model: String,
}

impl OllamaClient {
    pub fn new(model: String) -> Self {
        Self {
            http: Client::builder()
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(45))
                .build()
                .expect("HTTP client failed"),
            model,
        }
    }

    pub async fn analyze_journal(&self, content: &str, id: String) -> Result<AnalysisResult, AppError> {
        let safe_content = truncate_content(content);

        // Retry (cold start protection)
        for attempt in 0..2 {
            match self.try_request(&safe_content, id.clone()).await {
                Ok(res) => return Ok(res),
                Err(_e) if attempt == 0 => {
                    println!("[AI] Cold start or transient failure, retrying analysis for {}...", id);
                    tokio::time::sleep(Duration::from_millis(800)).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    async fn try_request(&self, content: &str, id: String) -> Result<AnalysisResult, AppError> {
        let system_prompt = crate::ai::prompt::get_analysis_system_prompt();
        
        let response = self.http
            .post("http://127.0.0.1:11434/api/generate")
            .json(&json!({
                "model": self.model,
                "system": system_prompt,
                "prompt": content,
                "stream": false,
                "format": "json",
                "options": {
                    "temperature": 0.2,
                    "num_ctx": 16384
                }
            }))
            .send()
            .await
            .map_err(|e| AppError::AiError(format!("Connection failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if error_text.contains("not found") {
                return Err(AppError::AiError(format!(
                    "Model '{}' not found. Please run 'ollama pull {}'",
                    self.model, self.model
                )));
            }
            return Err(AppError::AiError(format!(
                "HTTP error {}: {}",
                status, error_text
            )));
        }

        let body: serde_json::Value = response.json().await
            .map_err(|e| AppError::AiError(format!("Invalid response JSON: {}", e)))?;

        let response_text = body.get("response")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| AppError::AiError(format!("Empty or missing response field: {}", body)))?;

        // JSON safety fallback: if AI returns invalid schema, use a basic fallback
        let raw: RawAnalysis = serde_json::from_str(response_text)
            .unwrap_or(RawAnalysis {
                summary: "Analysis failed to parse properly.".into(),
                mood: "neutral".into(),
                emotions: None,
                tasks: None,
                insights: None,
            });

        Ok(AnalysisResult::from_raw(raw, id))
    }
}

// Smart truncation (Head 2k + Tail 2k)
fn truncate_content(content: &str) -> String {
    if content.len() <= 4000 {
        return content.to_string();
    }

    let head: String = content.chars().take(2000).collect();
    let tail: String = content.chars().rev().take(2000).collect::<String>()
        .chars().rev().collect();

    format!("{}\n...\n{}", head, tail)
}
