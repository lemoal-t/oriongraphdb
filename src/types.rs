//! Core type definitions for AxonGraph context compilation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Immutable reference to a span within a specific document version
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpanRef {
    pub doc_version_id: String,  // SHA256 of doc bytes
    pub span_id: String,          // UUID or content-derived hash
    pub char_start: usize,
    pub char_end: usize,
    pub token_cost: usize,        // precomputed for default tokenizer
}

impl SpanRef {
    pub fn key(&self) -> SpanKey {
        SpanKey(self.doc_version_id.clone(), self.span_id.clone())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SpanKey(pub String, pub String); // (doc_version_id, span_id)

/// Candidate span with multi-channel scores
#[derive(Debug, Clone)]
pub struct CandidateSpan {
    pub span_ref: SpanRef,
    pub scores: ScoreChannels,
    pub embedding: Option<Vec<f32>>,  // normalized to unit length
    pub text_preview: String,          // first 100 chars
    pub metadata: SpanMetadata,
    
    // Computed during compilation
    pub base_score: f32,
    pub mmr_score: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ScoreChannels {
    pub semantic: f32,      // 0.0-1.0
    pub lexical: f32,
    pub structural: f32,
    pub graph: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpanMetadata {
    pub filepath: String,
    pub workstream: Option<String>,
    pub stage: Option<String>,
    pub section_title: Option<String>,
    pub created_at: i64,           // unix timestamp
    pub recency_score: f32,        // computed from created_at
    pub source_type: SourceType,
    pub tags: Vec<String>,         // NEW: Tags for categorization
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SourceType {
    Context,     // 01_context/
    Knowledge,   // 02_knowledge/
    Workstream,  // 03_workstreams/
    Artifact,    // 04_artifacts/
    Session,     // Session conversation history
    Memory,      // Long-term user memories
}

/// Final working set item
#[derive(Debug, Clone, Serialize)]
pub struct WSItem {
    pub span_ref: SpanRef,
    pub text: String,
    pub metadata: SpanMetadata,  // Added for hydration
}

#[derive(Debug, Clone, Serialize)]
pub enum Compression {
    Bullets { lines: Vec<String>, backref: SpanRef },
    Summary { text: String, backref: SpanRef },
}

/// Request to compile a working set
#[derive(Debug, Clone)]
pub struct CompileRequest {
    pub intent: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,  // NEW: For session-aware compilation
    pub user_id: Option<String>,     // NEW: For memory-aware compilation
    pub query_signals: Vec<QuerySignal>,
    pub budget_tokens: usize,
    pub hard_filters: HardFilters,
    pub soft_prefs: SoftPreferences,
    pub explain: bool,
}

#[derive(Debug, Clone)]
pub enum QuerySignal {
    NaturalLanguage(String),
    Keywords(Vec<String>),
    StructuralHints(StructHints),
    EpisodeContext(String), // episode_id
}

#[derive(Debug, Clone, Default)]
pub struct StructHints {
    pub section_patterns: Vec<String>,
    pub doc_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HardFilters {
    pub allowed_paths: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub max_doc_age_days: Option<u32>,
    pub required_workstreams: Vec<String>,
}

impl Default for HardFilters {
    fn default() -> Self {
        Self {
            allowed_paths: vec![],
            excluded_paths: vec![],
            max_doc_age_days: None,
            required_workstreams: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SoftPreferences {
    pub diversity_lambda: f32,        // 0.0 = pure utility, 1.0 = pure diversity
    pub max_single_source_ratio: f32, // e.g., 0.35
    pub prefer_recent: bool,
    pub prefer_stages: Vec<String>,
    pub score_weights: ScoreWeights,
}

impl Default for SoftPreferences {
    fn default() -> Self {
        Self {
            diversity_lambda: 0.3,
            max_single_source_ratio: 0.35,
            prefer_recent: false,
            prefer_stages: vec![],
            score_weights: ScoreWeights::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScoreWeights {
    pub semantic: f32,
    pub lexical: f32,
    pub structural: f32,
    pub graph: f32,
    pub recency: f32,
    pub stage_boost: f32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            semantic: 0.4,
            lexical: 0.2,
            structural: 0.2,
            graph: 0.1,
            recency: 0.05,
            stage_boost: 0.05,
        }
    }
}

/// Response from compilation
#[derive(Debug, Serialize)]
pub struct CompileResponse {
    pub workingset: WorkingSet,
    pub stats: CompileStats,
    pub rationale: Option<Vec<SpanExplanation>>,
}

#[derive(Debug, Serialize)]
pub struct WorkingSet {
    pub spans: Vec<WSItem>,
    pub total_tokens: usize,
}

#[derive(Debug, Serialize)]
pub struct CompileStats {
    pub candidates_generated: usize,
    pub candidates_after_dedup: usize,
    pub candidates_selected: usize,
    pub token_utilization: f32,
    pub source_distribution: HashMap<String, usize>,
    pub generation_time_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct SpanExplanation {
    pub span_ref: SpanRef,
    pub final_score: f32,
    pub base_score: f32,
    pub diversity_penalty: f32,
    pub reasons: Vec<String>,
}

/// Internal signals derived from request
#[derive(Debug, Clone)]
pub struct DerivedSignals {
    pub intent: String,
    pub intent_embedding: Vec<f32>,
    pub keywords: Vec<String>,
    pub struct_hints: StructHints,
    pub episode_context: Option<String>,
}
