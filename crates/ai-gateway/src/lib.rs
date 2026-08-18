//! xAI Grok advisory adapter. The API key stays in process env, never SQLite.

use application_core::ports::advisory::{Advisory, AdvisoryCompletion};
use application_core::ports::platform::PlatformError;
use async_trait::async_trait;
use financial_domain::advisory::redact_for_advisory;
use serde::Deserialize;

const XAI_CHAT_URL: &str = "https://api.x.ai/v1/chat/completions";
const GROK_MODEL: &str = "grok-4.6";
const SYSTEM_PROMPT: &str = "You are an advisory assistant for a local personal finance app. \
Output recommendations and questions only. Do not instruct posting ledger facts, executing trades, \
moving money, or changing MAGI golden oracles.";

pub struct GrokAdvisory {
    api_key: String,
    client: reqwest::Client,
}

impl GrokAdvisory {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("XAI_API_KEY")
            .ok()
            .or_else(|| std::env::var("GROK_API_KEY").ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;
        Some(Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .ok()?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[async_trait]
impl Advisory for GrokAdvisory {
    async fn complete(&self, prompt: &str) -> Result<AdvisoryCompletion, PlatformError> {
        let prompt = redact_for_advisory(prompt);
        let body = serde_json::json!({
            "model": GROK_MODEL,
            "stream": false,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ]
        });
        let response = self
            .client
            .post(XAI_CHAT_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| PlatformError::new("provider_error", redact_for_advisory(&e.to_string())))?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Err(PlatformError::new(
                "provider_error",
                format!("xAI HTTP {status}"),
            ));
        }
        let parsed: ChatResponse = response
            .json()
            .await
            .map_err(|e| PlatformError::new("provider_error", e.to_string()))?;
        let recommendation = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "advisory-only; empty provider response".to_string());
        Ok(AdvisoryCompletion {
            provider: "xai".to_string(),
            model: GROK_MODEL.to_string(),
            recommendation: redact_for_advisory(&recommendation),
        })
    }
}
