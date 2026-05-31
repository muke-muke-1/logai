use crate::ai::prompt::build_analysis_prompt;
use crate::ai::{parse_ai_response, AiBackend};
use crate::types::{AiResponse, AnalysisSummary};
use async_trait::async_trait;

pub struct ClaudeBackend {
    api_key: String,
    deep: bool,
    client: reqwest::Client,
}

impl ClaudeBackend {
    pub fn new(api_key: String, deep: bool) -> Self {
        Self {
            api_key,
            deep,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AiBackend for ClaudeBackend {
    async fn analyze(&self, summary: &AnalysisSummary) -> anyhow::Result<AiResponse> {
        let prompt = build_analysis_prompt(summary);
        let model = self.actual_model(self.deep);

        let body = serde_json::json!({
            "model": model,
            "max_tokens": 2048,
            "messages": [{"role": "user", "content": prompt}]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Claude API error ({}): {}", status, text);
        }

        let json: serde_json::Value = resp.json().await?;
        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        parse_ai_response(&content)
    }

    fn model_name(&self) -> &str {
        "Claude"
    }

    fn actual_model(&self, deep: bool) -> &str {
        if deep {
            "claude-opus-4-8"
        } else {
            "claude-haiku-4-5-20251001"
        }
    }
}
