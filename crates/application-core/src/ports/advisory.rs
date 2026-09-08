//! Advisory completion port. application-core has no HTTP and must not see the API key.

use async_trait::async_trait;

use crate::ports::platform::PlatformError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryCompletion {
    pub provider: String,
    pub model: String,
    pub recommendation: String,
}

#[async_trait]
pub trait Advisory: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<AdvisoryCompletion, PlatformError>;
}

/// Used by golden tests. Never calls the network.
pub struct StubAdvisory;

#[async_trait]
impl Advisory for StubAdvisory {
    async fn complete(&self, prompt: &str) -> Result<AdvisoryCompletion, PlatformError> {
        let _ = prompt;
        Ok(AdvisoryCompletion {
            provider: "stub".to_string(),
            model: "none".to_string(),
            recommendation: "advisory-only; does not post facts".to_string(),
        })
    }
}

/// Desktop fallback when `XAI_API_KEY` / `GROK_API_KEY` is unset.
pub struct MissingKeyAdvisory;

#[async_trait]
impl Advisory for MissingKeyAdvisory {
    async fn complete(&self, _prompt: &str) -> Result<AdvisoryCompletion, PlatformError> {
        Err(PlatformError::new(
            "missing_api_key",
            "Set XAI_API_KEY or GROK_API_KEY in the environment or a gitignored .env",
        ))
    }
}
