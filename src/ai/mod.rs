pub mod claude;
pub mod deepseek;
pub mod ollama;
pub mod openai;
pub mod prompt;

use crate::types::{AiResponse, AnalysisSummary, Model};
use async_trait::async_trait;
use std::env;

#[async_trait]
pub trait AiBackend: Send + Sync {
    async fn analyze(&self, summary: &AnalysisSummary) -> anyhow::Result<AiResponse>;
    fn model_name(&self) -> &str;
    fn actual_model(&self, deep: bool) -> &str;
}

/// Create the appropriate backend based on Model enum
pub fn create_backend(model: Model, deep: bool) -> anyhow::Result<Box<dyn AiBackend>> {
    match model {
        Model::Claude => {
            let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| {
                anyhow::anyhow!("ANTHROPIC_API_KEY not set. Set it with: export ANTHROPIC_API_KEY=sk-ant-...")
            })?;
            Ok(Box::new(claude::ClaudeBackend::new(api_key, deep)))
        }
        Model::OpenAI => {
            let api_key = env::var("OPENAI_API_KEY").map_err(|_| {
                anyhow::anyhow!("OPENAI_API_KEY not set. Set it with: export OPENAI_API_KEY=sk-...")
            })?;
            Ok(Box::new(openai::OpenAiBackend::new(api_key, deep)))
        }
        Model::DeepSeek => {
            let api_key = env::var("DEEPSEEK_API_KEY").map_err(|_| {
                anyhow::anyhow!("DEEPSEEK_API_KEY not set. Set it with: export DEEPSEEK_API_KEY=sk-...")
            })?;
            Ok(Box::new(deepseek::DeepSeekBackend::new(api_key, deep)))
        }
        Model::Ollama => {
            let host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
            Ok(Box::new(ollama::OllamaBackend::new(host, deep)))
        }
        Model::Auto => auto_detect(deep),
    }
}

/// Auto-detect available backend by checking env vars, priority: Claude > OpenAI > DeepSeek > Ollama
fn auto_detect(deep: bool) -> anyhow::Result<Box<dyn AiBackend>> {
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        eprintln!("Auto-detected: Claude (ANTHROPIC_API_KEY)");
        return create_backend(Model::Claude, deep);
    }
    if env::var("OPENAI_API_KEY").is_ok() {
        eprintln!("Auto-detected: OpenAI (OPENAI_API_KEY)");
        return create_backend(Model::OpenAI, deep);
    }
    if env::var("DEEPSEEK_API_KEY").is_ok() {
        eprintln!("Auto-detected: DeepSeek (DEEPSEEK_API_KEY)");
        return create_backend(Model::DeepSeek, deep);
    }
    // Try Ollama as last resort
    let host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    if client.get(&host).send().is_ok() {
        eprintln!("Auto-detected: Ollama ({})", host);
        return create_backend(Model::Ollama, deep);
    }
    Err(anyhow::anyhow!(
        "No AI backend available. Set one of:\n  \
         ANTHROPIC_API_KEY (Claude)\n  \
         OPENAI_API_KEY (OpenAI)\n  \
         DEEPSEEK_API_KEY (DeepSeek)\n  \
         Or start Ollama: ollama serve"
    ))
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
            description: format!("AI response parsing failed. Raw response:\n{}", &text[..text.len().min(500)]),
            evidence: vec![],
            severity: crate::types::Severity::Medium,
        }],
        summary: "Unable to parse AI response as JSON".to_string(),
        fix_suggestions: vec![],
        confidence: 0.0,
    })
}
