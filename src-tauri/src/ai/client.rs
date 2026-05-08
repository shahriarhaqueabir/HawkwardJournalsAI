use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use crate::ai::{RawAnalysis, AnalysisResult};
use crate::error::AppError;
use futures_util::StreamExt;

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

        let raw: RawAnalysis = serde_json::from_str(response_text)
            .unwrap_or(RawAnalysis {
                summary: "Analysis failed to parse properly.".into(),
                mood: "neutral".into(),
                emotions: None,
                tasks: None,
                insights: None,
                triplets: None,
                facts: None,
            });

        Ok(AnalysisResult::from_raw(raw, id))
    }

    pub async fn chat_stream(
        &self,
        app: tauri::AppHandle,
        conversation_id: String,
        system_prompt: String,
        messages: Vec<serde_json::Value>,
    ) -> Result<(), AppError> {
        let response = self.http
            .post("http://127.0.0.1:11434/api/chat")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "system": system_prompt,
                "stream": true,
                "options": {
                    "temperature": 0.7,
                    "num_ctx": 16384
                }
            }))
            .send()
            .await
            .map_err(|e| AppError::AiError(format!("Connection failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::AiError(format!("Ollama error: {}", response.status())));
        }

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| AppError::AiError(e.to_string()))?;
            let line = String::from_utf8_lossy(&chunk);
            
            for json_str in line.split('\n').filter(|s| !s.trim().is_empty()) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(token) = val["message"]["content"].as_str() {
                        crate::events::emit(&app, crate::events::AppEvent::AiToken {
                            conversation_id: conversation_id.clone(),
                            token: token.to_string(),
                        });
                    }
                    if val["done"].as_bool().unwrap_or(false) {
                        crate::events::emit(&app, crate::events::AppEvent::AiResponseComplete {
                            conversation_id: conversation_id.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

fn truncate_content(content: &str) -> String {
    if content.len() <= 4000 {
        return content.to_string();
    }

    let head: String = content.chars().take(2000).collect();
    let tail: String = content.chars().rev().take(2000).collect::<String>()
        .chars().rev().collect();

    format!("{}\n...\n{}", head, tail)
}
