use crate::ai::prompt::build_analysis_prompt;
use crate::ai::{parse_ai_response, AiBackend};
use crate::types::{AiResponse, AnalysisSummary};
use async_trait::async_trait;

pub struct OpenAiBackend {
    api_key: String,
    deep: bool,
    client: reqwest::Client,
}

impl OpenAiBackend {
    pub fn new(api_key: String, deep: bool) -> Self {
        Self {
            api_key,
            deep,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AiBackend for OpenAiBackend {
    async fn analyze(&self, summary: &AnalysisSummary) -> anyhow::Result<AiResponse> {
        let prompt = build_analysis_prompt(summary);
        let model = self.actual_model(self.deep);

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "Always respond with valid JSON only. No markdown."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error ({}): {}", status, text);
        }

        let json: serde_json::Value = resp.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        parse_ai_response(&content)
    }

    fn model_name(&self) -> &str {
        "OpenAI"
    }
    fn actual_model(&self, deep: bool) -> &str {
        if deep {
            "gpt-4o"
        } else {
            "gpt-4o-mini"
        }
    }
}
