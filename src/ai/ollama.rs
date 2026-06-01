use crate::ai::prompt::build_analysis_prompt;
use crate::ai::{parse_ai_response, AiBackend};
use crate::types::{AiResponse, AnalysisSummary};
use async_trait::async_trait;

pub struct OllamaBackend {
    host: String,
    deep: bool,
    client: reqwest::Client,
}

impl OllamaBackend {
    pub fn new(host: String, deep: bool) -> Self {
        Self {
            host,
            deep,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AiBackend for OllamaBackend {
    async fn analyze(&self, summary: &AnalysisSummary) -> anyhow::Result<AiResponse> {
        let prompt = build_analysis_prompt(summary);
        let model = self.actual_model(self.deep);

        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.host))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            anyhow::bail!(
                "Ollama error ({}). Is Ollama running? Run: ollama serve\n\
                 Make sure you have pulled the model: ollama pull {}",
                status,
                model
            );
        }

        let json: serde_json::Value = resp.json().await?;
        let response_text = json["response"].as_str().unwrap_or("").to_string();
        parse_ai_response(&response_text)
    }

    async fn chat(&self, prompt: &str) -> anyhow::Result<String> {
        let model = self.actual_model(self.deep);
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.host))
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            anyhow::bail!(
                "Ollama error ({}). Is Ollama running? Run: ollama serve\n\
                 Make sure you have pulled the model: ollama pull {}",
                status,
                model
            );
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json["response"]
            .as_str()
            .unwrap_or("(empty response)")
            .to_string())
    }

    fn model_name(&self) -> &str {
        "Ollama"
    }
    fn actual_model(&self, deep: bool) -> &str {
        if deep {
            "llama3.2:latest"
        } else {
            "llama3.2"
        }
    }
}
