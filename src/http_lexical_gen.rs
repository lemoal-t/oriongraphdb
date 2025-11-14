//! HTTP-based lexical generator (BM25) that calls Python service

use crate::{CandidateGenerator, CandidateSpan, DerivedSignals, HardFilters, QuerySignal, ScoreChannels, SpanMetadata, SpanRef, SourceType};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Request to Python BM25 service
#[derive(Debug, Serialize)]
struct SearchRequest {
    query: String,
    k: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<SearchFilters>,
}

/// Filters for the Python BM25 service
#[derive(Debug, Serialize)]
struct SearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    workstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_paths: Option<Vec<String>>,
}

/// Response from Python BM25 service
#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: String,
    k: usize,
    results: Vec<SearchCandidate>,
    query_time_ms: f64,
}

/// Individual search candidate from Python service
#[derive(Debug, Deserialize)]
struct SearchCandidate {
    doc_id: usize,
    path: String,
    hash: String,
    score: f64,
    size: usize,
}

/// HTTP-based lexical (BM25) generator
pub struct HttpLexicalGen {
    service_url: String,
    client: reqwest::Client,
}

impl HttpLexicalGen {
    pub fn new(service_url: String) -> Self {
        Self {
            service_url,
            client: reqwest::Client::new(),
        }
    }

    /// Extract query text from derived signals
    fn extract_query(signals: &DerivedSignals) -> String {
        // Use keywords for lexical search (better for BM25)
        if !signals.keywords.is_empty() {
            return signals.keywords.join(" ");
        }
        
        // Fall back to natural language query
        if !signals.intent.is_empty() {
            return signals.intent.clone();
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
                .nth(1) // "03_workstreams/ws-orion/..." -> "ws-orion"
                .map(|s| s.to_string())
        } else {
            None
        };

        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: candidate.hash.clone(),
                span_id: format!("span_{}", candidate.doc_id),
                char_start: 0,
                char_end: candidate.size,
                token_cost: (candidate.size / 4).max(10),
            },
            scores: ScoreChannels {
                semantic: 0.0,
                lexical: candidate.score as f32,  // BM25 score goes here
                structural: 0.0,
                graph: 0.0,
            },
            embedding: None,
            text_preview: format!("Content from {}", candidate.path),
            metadata: SpanMetadata {
                filepath: candidate.path,
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
impl CandidateGenerator for HttpLexicalGen {
    fn name(&self) -> &'static str {
        "http_lexical"
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
            .context(format!("Failed to send request to BM25 service at {}", url))?;

        let response_text = response.text().await.context("Failed to get response text")?;
        
        let search_response: SearchResponse = serde_json::from_str(&response_text)
            .context(format!("Failed to parse BM25 service response: {}", response_text))?;

        tracing::info!(
            "Lexical search: {} candidates in {:.1}ms",
            search_response.results.len(),
            search_response.query_time_ms
        );

        let candidates = search_response
            .results
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
    fn test_extract_query_keywords() {
        let signals = DerivedSignals {
            intent: "".to_string(),
            intent_embedding: vec![],
            keywords: vec!["span".to_string(), "addressing".to_string()],
            struct_hints: Default::default(),
            episode_context: None,
        };
        let query = HttpLexicalGen::extract_query(&signals);
        assert_eq!(query, "span addressing");
    }

    #[test]
    fn test_extract_query_intent() {
        let signals = DerivedSignals {
            intent: "retrieval as compilation".to_string(),
            intent_embedding: vec![],
            keywords: vec![],
            struct_hints: Default::default(),
            episode_context: None,
        };
        let query = HttpLexicalGen::extract_query(&signals);
        assert_eq!(query, "retrieval as compilation");
    }
}

