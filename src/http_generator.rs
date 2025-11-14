//! HTTP-based semantic generator that calls Python service

use crate::{CandidateGenerator, CandidateSpan, DerivedSignals, HardFilters, QuerySignal, ScoreChannels, SpanMetadata, SpanRef, SourceType};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Request to Python semantic service
#[derive(Debug, Serialize)]
struct SearchRequest {
    query: String,
    k: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<SearchFilters>,
}

#[derive(Debug, Serialize)]
struct SearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    workstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_paths: Option<Vec<String>>,
}

/// Response from Python semantic service
#[derive(Debug, Deserialize)]
struct SearchResponse {
    candidates: Vec<SearchCandidate>,
    query_time_ms: f64,
    num_results: usize,
}

#[derive(Debug, Deserialize)]
struct SearchCandidate {
    chunk_id: usize,
    path: String,
    score: f64,
    distance: f64,
    token_cost: usize,
    // Chunk-level fields (new format)
    #[serde(default)]
    doc_version_id: Option<String>,
    #[serde(default)]
    span_id: Option<String>,
    #[serde(default)]
    char_start: Option<usize>,
    #[serde(default)]
    char_end: Option<usize>,
    #[serde(default)]
    abs_path: Option<String>,
    // Document-level fields (legacy format)
    #[serde(default)]
    hash: Option<String>,
    #[serde(default)]
    size: Option<usize>,
}

/// HTTP-based semantic generator
pub struct HttpSemanticGen {
    service_url: String,
    client: reqwest::Client,
}

impl HttpSemanticGen {
    /// Create new HTTP semantic generator
    pub fn new(service_url: String) -> Self {
        Self {
            service_url,
            client: reqwest::Client::new(),
        }
    }

    /// Extract query text from derived signals
    fn extract_query(signals: &DerivedSignals) -> String {
        // Use natural language query if available
        if !signals.intent.is_empty() {
            return signals.intent.clone();
        }
        
        // Fall back to keywords
        if !signals.keywords.is_empty() {
            return signals.keywords.join(" ");
        }
        
        "".to_string()
    }

    /// Convert search candidate to CandidateSpan
    fn to_candidate_span(&self, candidate: SearchCandidate) -> CandidateSpan {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Determine source type from path
        let source_type = if candidate.path.contains("03_workstreams/") {
            SourceType::Workstream
        } else if candidate.path.contains("02_knowledge/") {
            SourceType::Knowledge
        } else if candidate.path.contains("01_context/") {
            SourceType::Context
        } else {
            SourceType::Artifact
        };

        // Extract workstream from path if present
        let workstream = if candidate.path.contains("03_workstreams/") {
            candidate.path
                .split('/')
                .nth(1)
                .map(|s| s.to_string())
        } else {
            None
        };

        // Use chunk-level fields if available, else fall back to document-level
        let (doc_version_id, span_id, char_start, char_end, token_cost) = 
            if let (Some(doc_id), Some(span), Some(start), Some(end)) = 
                (&candidate.doc_version_id, &candidate.span_id, candidate.char_start, candidate.char_end) {
                // Chunk-level metadata
                (doc_id.clone(), span.clone(), start, end, candidate.token_cost)
            } else {
                // Legacy document-level metadata
                let hash = candidate.hash.as_ref().map(|h| h.clone()).unwrap_or_else(|| "unknown".to_string());
                let size = candidate.size.unwrap_or(1000);
                let token_cost = (size / 4).max(10);
                (hash.clone(), format!("span_{}", candidate.chunk_id), 0, size, token_cost)
            };

        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id,
                span_id,
                char_start,
                char_end,
                token_cost,
            },
            scores: ScoreChannels {
                semantic: candidate.score as f32,
                lexical: 0.0,
                structural: 0.0,
                graph: 0.0,
            },
            embedding: None, // Not needed for now
            text_preview: format!("Content from {}", candidate.path),
            metadata: SpanMetadata {
                filepath: candidate.path.clone(),
                workstream,
                stage: None,
                section_title: None,
                created_at: now,
                recency_score: 0.9,
                source_type,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        }
    }
}

#[async_trait]
impl CandidateGenerator for HttpSemanticGen {
    fn name(&self) -> &'static str {
        "http_semantic"
    }
    
    async fn generate(
        &self,
        signals: &DerivedSignals,
        filters: &HardFilters,
        top_k: usize,
    ) -> Result<Vec<CandidateSpan>> {
        // Extract query from signals
        let query = Self::extract_query(signals);
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // Build filters
        let search_filters = if !filters.allowed_paths.is_empty()
            || !filters.required_workstreams.is_empty()
        {
            Some(SearchFilters {
                workstream: filters.required_workstreams.first().cloned(),
                allowed_paths: if filters.allowed_paths.is_empty() {
                    None
                } else {
                    Some(filters.allowed_paths.clone())
                },
            })
        } else {
            None
        };

        // Build search request
        let search_req = SearchRequest {
            query: query.clone(),
            k: top_k * 3, // Get more candidates for MMR selection
            filters: search_filters,
        };

        // Call Python service
        let url = format!("{}/search", self.service_url);
        let response = self
            .client
            .post(&url)
            .json(&search_req)
            .send()
            .await
            .context("Failed to call semantic service")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Semantic service error ({}): {}", status, error_text);
        }

        let search_response: SearchResponse = response
            .json()
            .await
            .context("Failed to parse semantic service response")?;

        tracing::info!(
            "Semantic search: {} candidates in {:.1}ms",
            search_response.num_results,
            search_response.query_time_ms
        );

        // Convert to CandidateSpans
        let candidates = search_response
            .candidates
            .into_iter()
            .map(|c| self.to_candidate_span(c))
            .collect();

        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_query() {
        let signals = DerivedSignals {
            intent: "test query".to_string(),
            intent_embedding: vec![],
            keywords: vec![],
            struct_hints: Default::default(),
            episode_context: None,
        };
        let query = HttpSemanticGen::extract_query(&signals);
        assert_eq!(query, "test query");
    }

    #[test]
    fn test_extract_query_keywords() {
        let signals = DerivedSignals {
            intent: "".to_string(),
            intent_embedding: vec![],
            keywords: vec!["foo".to_string(), "bar".to_string()],
            struct_hints: Default::default(),
            episode_context: None,
        };
        let query = HttpSemanticGen::extract_query(&signals);
        assert_eq!(query, "foo bar");
    }
}

