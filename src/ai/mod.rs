pub mod claude;
pub mod deepseek;
pub mod ollama;
pub mod openai;
pub mod prompt;

use crate::errors::LogaiError;
use crate::types::{AiResponse, AnalysisSummary, Model};
use async_trait::async_trait;
use std::env;

#[async_trait]
pub trait AiBackend: Send + Sync {
    async fn analyze(&self, summary: &AnalysisSummary) -> anyhow::Result<AiResponse>;
    /// Send a free-form chat prompt and return the text response.
    /// Used by TUI AI panel for interactive Q&A.
    async fn chat(&self, prompt: &str) -> anyhow::Result<String>;
    fn model_name(&self) -> &str;
    fn actual_model(&self, deep: bool) -> &str;
}

/// Retry an async operation with exponential backoff.
/// - Max 3 attempts total (2 retries after initial failure)
/// - Backoff: 1s → 2s → 4s
/// - `on_retry` callback receives (attempt_number, error_message) for status reporting.
///   CLI passes eprintln, TUI passes a status-bar updater.
pub async fn with_retry<T, F, Fut, R>(f: F, on_retry: R) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
    R: Fn(u32, &str),
{
    const MAX_ATTEMPTS: u32 = 3;
    const BASE_DELAY_MS: u64 = 1000;

    let mut last_err = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                if attempt < MAX_ATTEMPTS {
                    let delay_ms = BASE_DELAY_MS * (1 << (attempt - 1)); // 1s, 2s, 4s
                    let msg = format!("{}", last_err.as_ref().unwrap());
                    on_retry(attempt, &msg);
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
    // All attempts exhausted — return the last error
    Err(last_err.unwrap())
}

/// Create the appropriate backend based on Model enum
pub async fn create_backend(model: Model, deep: bool) -> anyhow::Result<Box<dyn AiBackend>> {
    match model {
        Model::Claude => {
            let api_key = env::var("ANTHROPIC_API_KEY")
                .map_err(|_| LogaiError::missing_api_key("Claude", "ANTHROPIC_API_KEY"))?;
            Ok(Box::new(claude::ClaudeBackend::new(api_key, deep)))
        }
        Model::OpenAI => {
            let api_key = env::var("OPENAI_API_KEY")
                .map_err(|_| LogaiError::missing_api_key("OpenAI", "OPENAI_API_KEY"))?;
            Ok(Box::new(openai::OpenAiBackend::new(api_key, deep)))
        }
        Model::DeepSeek => {
            let api_key = env::var("DEEPSEEK_API_KEY")
                .map_err(|_| LogaiError::missing_api_key("DeepSeek", "DEEPSEEK_API_KEY"))?;
            Ok(Box::new(deepseek::DeepSeekBackend::new(api_key, deep)))
        }
        Model::Ollama => {
            let host =
                env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
            Ok(Box::new(ollama::OllamaBackend::new(host, deep)))
        }
        Model::Auto => Box::pin(auto_detect(deep)).await,
    }
}

/// Auto-detect available backend by checking env vars, priority: Claude > OpenAI > DeepSeek > Ollama
async fn auto_detect(deep: bool) -> anyhow::Result<Box<dyn AiBackend>> {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!("Auto-detected: Claude (ANTHROPIC_API_KEY)");
        return create_backend(Model::Claude, deep).await;
    }
    if std::env::var("OPENAI_API_KEY").is_ok() {
        eprintln!("Auto-detected: OpenAI (OPENAI_API_KEY)");
        return create_backend(Model::OpenAI, deep).await;
    }
    if std::env::var("DEEPSEEK_API_KEY").is_ok() {
        eprintln!("Auto-detected: DeepSeek (DEEPSEEK_API_KEY)");
        return create_backend(Model::DeepSeek, deep).await;
    }
    // Try Ollama as last resort (async probe)
    let host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    if client.get(&host).send().await.is_ok() {
        eprintln!("Auto-detected: Ollama ({})", host);
        return create_backend(Model::Ollama, deep).await;
    }
    Err(LogaiError::missing_api_key("auto-detect", "任何 AI 后端").into())
}

/// Parse AI response JSON with graceful degradation
pub fn parse_ai_response(text: &str) -> anyhow::Result<AiResponse> {
    // Try direct parse
    if let Ok(resp) = serde_json::from_str::<AiResponse>(text) {
        return Ok(resp);
    }
    // Try extracting JSON block from text
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            let json_str = &text[start..=end];
            if let Ok(resp) = serde_json::from_str::<AiResponse>(json_str) {
                return Ok(resp);
            }
        }
    }
    // Graceful fallback
    Ok(AiResponse {
        root_causes: vec![crate::types::RootCause {
            description: format!(
                "AI response parsing failed. Raw response:\n{}",
                &text[..text.len().min(500)]
            ),
            evidence: vec![],
            severity: crate::types::Severity::Medium,
        }],
        summary: "Unable to parse AI response as JSON".to_string(),
        fix_suggestions: vec![],
        confidence: 0.0,
    })
}
