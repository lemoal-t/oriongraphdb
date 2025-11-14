/// HTTP client for querying Memory API
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct MemoryClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct Memory {
    pub text: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance: Option<f64>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MemoriesResponse {
    pub user_id: String,
    pub memories: Vec<Memory>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct FormattedMemoriesResponse {
    pub user_id: String,
    pub query: String,
    pub formatted_text: String,
    pub memory_count: usize,
}

impl MemoryClient {
    /// Create a new memory client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Get relevant memories for a user based on query
    pub async fn get_memories(
        &self,
        user_id: &str,
        query: &str,
        limit: Option<usize>,
    ) -> Result<MemoriesResponse> {
        let url = format!(
            "{}/memory/{}?query={}&limit={}",
            self.base_url,
            user_id,
            urlencoding::encode(query),
            limit.unwrap_or(5)
        );

        debug!("Fetching memories from {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Memory API error {}: {}", status, body);
        }

        let memories: MemoriesResponse = response.json().await?;
        debug!(
            "Retrieved {} memories for user {}",
            memories.count, user_id
        );

        Ok(memories)
    }

    /// Get formatted memories as a single text block
    pub async fn get_formatted_memories(
        &self,
        user_id: &str,
        query: &str,
        limit: Option<usize>,
    ) -> Result<FormattedMemoriesResponse> {
        let url = format!(
            "{}/memory/{}/formatted?query={}&limit={}",
            self.base_url,
            user_id,
            urlencoding::encode(query),
            limit.unwrap_or(5)
        );

        debug!("Fetching formatted memories from {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Memory API error {}: {}", status, body);
        }

        let formatted: FormattedMemoriesResponse = response.json().await?;
        debug!(
            "Retrieved formatted memories ({} memories) for user {}",
            formatted.memory_count, user_id
        );

        Ok(formatted)
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
    #[ignore] // Requires running memory API
    async fn test_memory_client_integration() {
        let client = MemoryClient::new("http://127.0.0.1:8086");

        // Test health check
        let health = client.health_check().await;
        assert!(health.is_ok());
    }
}
