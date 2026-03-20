use crate::ai::{AnalysisResult, RawAnalysis};
use crate::error::AppError;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

pub const DEFAULT_MODEL: &str = "llama3.2";
pub const OLLAMA_GENERATE_URL: &str = "api/generate";
pub const OLLAMA_CHAT_URL: &str = "api/chat";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaToolFunction {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaToolCall {
    pub function: OllamaToolFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    pub options: Value,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub done: bool,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponsePart {
    pub message: Option<ChatMessage>,
    pub done: bool,
    pub error: Option<String>,
}

pub struct OllamaClient {
    http: Client,
    model: String,
    base_url: String,
}

impl OllamaClient {
    pub fn new(model: String) -> Self {
        Self {
            http: Client::builder()
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(300)) // Increased for long chat turns
                .build()
                .expect("HTTP client failed"),
            model,
            base_url: "http://127.0.0.1:11434".to_string(),
        }
    }

    pub async fn analyze_journal(
        &self,
        content: &str,
        id: String,
    ) -> Result<AnalysisResult, AppError> {
        let safe_content = truncate_content(content);
        let url = format!("{}/{}", self.base_url, OLLAMA_GENERATE_URL);

        // Retry (cold start protection)
        for attempt in 0..2 {
            match self.try_request(&safe_content, id.clone(), &url).await {
                Ok(res) => return Ok(res),
                Err(_e) if attempt == 0 => {
                    println!(
                        "[AI] Cold start or transient failure, retrying analysis for {}...",
                        id
                    );
                    tokio::time::sleep(Duration::from_millis(800)).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    async fn try_request(
        &self,
        content: &str,
        id: String,
        url: &str,
    ) -> Result<AnalysisResult, AppError> {
        let system_prompt = crate::ai::prompt::get_analysis_system_prompt();

        let response = self
            .http
            .post(url)
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

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::AiError(format!("Invalid response JSON: {}", e)))?;

        let response_text = body
            .get("response")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                AppError::AiError(format!("Empty or missing response field: {}", body))
            })?;

        let raw: RawAnalysis = serde_json::from_str(response_text).unwrap_or(RawAnalysis {
            summary: "Analysis failed to parse properly.".into(),
            mood: "neutral".into(),
            emotions: None,
            tasks: None,
            insights: None,
        });

        Ok(AnalysisResult::from_raw(raw, id))
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Value>,
    ) -> Result<
        impl futures_util::stream::Stream<Item = Result<ChatResponsePart, AppError>>,
        AppError,
    > {
        let url = format!("{}/{}", self.base_url, OLLAMA_CHAT_URL);

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            tools,
            options: json!({
                "temperature": 0.7,
                "num_ctx": 16384
            }),
        };

        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::AiError(format!("Chat connection failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::AiError(format!(
                "HTTP error {}: {}",
                status,
                error_text
            )));
        }

        let stream = response.bytes_stream().map(|item| match item {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let mut parts = Vec::new();
                for line in text.split('\n') {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let part: ChatResponsePart =
                            serde_json::from_str(trimmed).map_err(|e| {
                                AppError::AiError(format!(
                                    "Failed to parse stream part: {} | Raw: {}",
                                    e, trimmed
                                ))
                            })?;
                        parts.push(Ok(part));
                    }
                }
                Ok(parts)
            }
            Err(e) => Err(AppError::AiError(format!("Stream error: {}", e))),
        });

        // Flatten the stream of vectors
        let flattened = stream.flat_map(|res| match res {
            Ok(parts) => futures_util::stream::iter(parts),
            Err(e) => futures_util::stream::iter(vec![Err(e)]),
        });

        Ok(flattened)
    }
}

fn truncate_content(content: &str) -> String {
    if content.len() <= 4000 {
        return content.to_string();
    }
    let head: String = content.chars().take(2000).collect();
    let tail: String = content
        .chars()
        .rev()
        .take(2000)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}\n...\n{}", head, tail)
}
