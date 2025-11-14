/// HTTP client for querying SessionStore API
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct SessionClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct SessionEvent {
    pub id: String,
    pub timestamp: String,
    pub role: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SessionContextSpan {
    pub text: String,
    pub role: String,
    pub timestamp: String,
    pub token_estimate: usize,
}

#[derive(Debug, Deserialize)]
pub struct SessionContextResponse {
    pub session_id: String,
    pub context_spans: Vec<SessionContextSpan>,
    pub total_tokens_estimate: usize,
}

#[derive(Debug, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub state: HashMap<String, serde_json::Value>,
    pub exists: bool,
}

impl SessionClient {
    /// Create a new session client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Get recent session context formatted for AxonGraph
    pub async fn get_session_context(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<SessionContextResponse> {
        let url = format!(
            "{}/session/{}/context?limit={}",
            self.base_url,
            session_id,
            limit.unwrap_or(10)
        );

        debug!("Fetching session context from {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Session API error {}: {}", status, body);
        }

        let context: SessionContextResponse = response.json().await?;
        debug!(
            "Retrieved {} context spans from session {}",
            context.context_spans.len(),
            session_id
        );

        Ok(context)
    }

    /// Get session state
    pub async fn get_session_state(
        &self,
        session_id: &str,
    ) -> Result<SessionState> {
        let url = format!("{}/session/{}/state", self.base_url, session_id);

        debug!("Fetching session state from {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Session API error {}: {}", status, body);
        }

        let state: SessionState = response.json().await?;
        debug!("Retrieved session state for {}", session_id);

        Ok(state)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running session API
    async fn test_session_client_integration() {
        let client = SessionClient::new("http://127.0.0.1:8085");

        // Test health check
        let health = client.health_check().await;
        assert!(health.is_ok());
    }
}

