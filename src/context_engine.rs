//! Core ContextEngine for compiling working sets

use crate::generators::CandidateGenerator;
use crate::scoring::{normalize_scores, compute_base_score};
use crate::selection::select_with_mmr;
use crate::types::*;
use crate::session_client::{SessionClient, SessionContextSpan};
use crate::memory_client::{MemoryClient, Memory};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn, debug};

/// Main context engine (thread-safe via Arc)
pub struct ContextEngine {
    pub generators: Vec<Box<dyn CandidateGenerator>>,
    pub session_client: Option<SessionClient>,
    pub memory_client: Option<MemoryClient>,
}

pub type SharedContextEngine = Arc<ContextEngine>;

impl ContextEngine {
    /// Create a new context engine with generators and optional session/memory clients
    pub fn new(
        generators: Vec<Box<dyn CandidateGenerator>>,
        session_api_url: Option<String>,
        memory_api_url: Option<String>,
    ) -> SharedContextEngine {
        let session_client = session_api_url.map(|url| SessionClient::new(url));
        let memory_client = memory_api_url.map(|url| MemoryClient::new(url));
        
        Arc::new(Self { 
            generators,
            session_client,
            memory_client,
        })
    }
    
    /// Create a context engine without session/memory support (backward compatible)
    pub fn new_simple(generators: Vec<Box<dyn CandidateGenerator>>) -> SharedContextEngine {
        Self::new(generators, None, None)
    }
    
    /// Main entry point: compile a working set from intent
    pub async fn compile_workingset(
        &self,
        req: CompileRequest,
    ) -> Result<CompileResponse> {
        // Hard cap for how much of the total budget
        // can be consumed by pre-selected session context.
        // Remaining budget is reserved for retrieval (including memories).
        const MAX_CONTEXT_RATIO: f32 = 0.5;
        
        let start = Instant::now();
        
        info!("Compiling working set: intent='{}', budget={}, session_id={:?}, user_id={:?}",
            req.intent, req.budget_tokens, req.session_id, req.user_id);
        
        // Step 0: Derive signals from intent
        let signals = self.derive_signals(&req)?;
        
        // Step 0.5: Fetch session context (NEW)
        let mut session_spans = self.fetch_contextual_enrichment(&req).await?;
        
        // Compute and enforce a cap on contextual tokens (session only)
        let max_context_tokens = ((req.budget_tokens as f32) * MAX_CONTEXT_RATIO) as usize;
        let mut contextual_tokens: usize = session_spans
            .iter()
            .map(|s| s.span_ref.token_cost)
            .sum::<usize>();
        
        if contextual_tokens > max_context_tokens && max_context_tokens > 0 {
            // Trim session spans (keep most recent by removing from the front)
            while contextual_tokens > max_context_tokens && !session_spans.is_empty() {
                let removed = session_spans.remove(0);
                contextual_tokens = contextual_tokens.saturating_sub(removed.span_ref.token_cost);
            }
        }
        
        // Adjust retrieval budget to account for session context
        let retrieval_budget = if contextual_tokens > 0 {
            let adjusted = req.budget_tokens.saturating_sub(contextual_tokens);
            info!(
                "Budget allocation: {}/{} for retrieval (including memories), {}/{} for session context (capped at {} tokens)",
                adjusted,
                req.budget_tokens,
                contextual_tokens,
                req.budget_tokens,
                max_context_tokens
            );
            adjusted
        } else {
            req.budget_tokens
        };
        
        // Step 1: Generate candidates in parallel
        let mut candidates = self.generate_candidates(&signals, &req).await?;
        
        // Step 1.5: Fetch memory candidates so they participate in MMR selection
        let memory_candidates = self.fetch_memory_candidates(&req).await?;
        let total_generated: usize = candidates.iter().map(|v| v.len()).sum::<usize>() + memory_candidates.len();
        if !memory_candidates.is_empty() {
            candidates.push(memory_candidates);
        }
        
        // Step 2: Fuse and normalize
        let mut fused = self.fuse_candidates(candidates)?;
        let total_after_dedup = fused.len();
        
        // Step 3: Compute base scores
        self.score_candidates(&mut fused, &req.soft_prefs)?;
        
        // Step 4: MMR + knapsack selection (using adjusted budget)
        let (mut selected, explanations) = select_with_mmr(
            fused,
            retrieval_budget,
            &req.soft_prefs,
            req.explain,
        ).await?;
        
        // Step 4.5: Prepend session spans (priority order)
        // Session context comes first (most recent), then retrieval (including memories)
        let mut final_selected = Vec::new();
        final_selected.extend(session_spans);
        final_selected.extend(selected);
        
        // Step 5: Hydrate text
        let workingset = self.hydrate_workingset(final_selected).await?;
        
        // Step 6: Compile stats
        let source_distribution = compute_source_distribution(&workingset);
        let token_utilization = workingset.total_tokens as f32 / req.budget_tokens as f32;
        
        let stats = CompileStats {
            candidates_generated: total_generated,
            candidates_after_dedup: total_after_dedup,
            candidates_selected: workingset.spans.len(),
            token_utilization,
            source_distribution,
            generation_time_ms: start.elapsed().as_millis() as u64,
        };
        
        info!("Compilation complete: {} spans, {} tokens ({:.1}% utilization)",
            workingset.spans.len(), workingset.total_tokens, token_utilization * 100.0);
        
        Ok(CompileResponse {
            workingset,
            stats,
            rationale: if req.explain { Some(explanations) } else { None },
        })
    }
    
    /// Derive query signals from request
    fn derive_signals(&self, req: &CompileRequest) -> Result<DerivedSignals> {
        // TODO: Real implementation with embedding model
        // For now: simple keyword extraction
        let keywords: Vec<String> = req.intent
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() > 3)
            .collect();
        
        let intent_embedding = vec![0.0; 768]; // TODO: real embedding
        
        let struct_hints = StructHints {
            section_patterns: vec![],
            doc_types: vec![],
        };
        
        Ok(DerivedSignals {
            intent: req.intent.clone(),
            intent_embedding,
            keywords,
            struct_hints,
            episode_context: req.task_id.clone(),
        })
    }
    
    /// Run all generators in parallel
    async fn generate_candidates(
        &self,
        signals: &DerivedSignals,
        req: &CompileRequest,
    ) -> Result<Vec<Vec<CandidateSpan>>> {
        let top_k = self.estimate_top_k(req.budget_tokens);
        
        let mut tasks = Vec::new();
        for gen in &self.generators {
            let gen_ref = gen.as_ref();
            let signals = signals.clone();
            let filters = req.hard_filters.clone();
            
            // Spawn task for each generator
            let task = async move {
                gen_ref.generate(&signals, &filters, top_k).await
            };
            tasks.push(task);
        }
        
        // Wait for all generators
        let results = futures::future::join_all(tasks).await;
        
        let mut all_candidates = Vec::new();
        for res in results {
            match res {
                Ok(cands) => all_candidates.push(cands),
                Err(e) => {
                    tracing::warn!("Generator failed: {:?}", e);
                }
            }
        }
        
        // Fallback if all generators failed
        if all_candidates.iter().all(|v| v.is_empty()) {
            tracing::error!("All generators returned empty");
            anyhow::bail!("No candidates found for intent: '{}'", req.intent);
        }
        
        Ok(all_candidates)
    }
    
    /// Fuse candidates from multiple generators and deduplicate
    fn fuse_candidates(&self, candidate_vecs: Vec<Vec<CandidateSpan>>) -> Result<Vec<CandidateSpan>> {
        let mut map: HashMap<SpanKey, CandidateSpan> = HashMap::new();
        
        for candidates in candidate_vecs {
            for cand in candidates {
                let key = cand.span_ref.key();
                
                map.entry(key)
                    .and_modify(|existing| {
                        // Merge scores: take max for each channel
                        existing.scores.semantic = existing.scores.semantic.max(cand.scores.semantic);
                        existing.scores.lexical = existing.scores.lexical.max(cand.scores.lexical);
                        existing.scores.structural = existing.scores.structural.max(cand.scores.structural);
                        existing.scores.graph = existing.scores.graph.max(cand.scores.graph);
                    })
                    .or_insert(cand);
            }
        }
        
        Ok(map.into_values().collect())
    }
    
    /// Score all candidates
    fn score_candidates(&self, candidates: &mut [CandidateSpan], prefs: &SoftPreferences) -> Result<()> {
        // Normalize scores per channel
        normalize_scores(candidates);
        
        // Compute base score for each
        for cand in candidates.iter_mut() {
            cand.base_score = compute_base_score(cand, prefs);
        }
        
        Ok(())
    }
    
    /// Hydrate selected spans with full text
    async fn hydrate_workingset(&self, mut selected: Vec<WSItem>) -> Result<WorkingSet> {
        use std::collections::HashMap;
        use std::fs;
        
        let mut total_tokens = 0;
        
        // Cache file contents to avoid re-reading
        let mut file_cache: HashMap<String, String> = HashMap::new();
        
        for item in &mut selected {
            // Session and Memory spans already carry their text;
            // we do not attempt filesystem hydration for them.
            match item.metadata.source_type {
                SourceType::Session | SourceType::Memory => {
                    total_tokens += item.span_ref.token_cost;
                    continue;
                }
                _ => {}
            }
            
            // Read file content from filesystem
            let file_path = &item.metadata.filepath;
            
            // Get content from cache or read from disk
            let content = if let Some(cached) = file_cache.get(file_path) {
                cached.clone()
            } else {
                // Try to read file - handle errors gracefully
                match fs::read_to_string(file_path) {
                    Ok(content) => {
                        file_cache.insert(file_path.clone(), content.clone());
                        content
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read file {}: {}", file_path, e);
                        // Fallback to placeholder
                        format!("[ERROR: Could not read {} - {}]", file_path, e)
                    }
                }
            };
            
            // Slice the content using char offsets
            let char_start = item.span_ref.char_start;
            let char_end = item.span_ref.char_end;
            
            // Convert byte-based string to char indices
            let chars: Vec<char> = content.chars().collect();
            let total_chars = chars.len();
            
            // Bounds check
            if char_start >= total_chars {
                tracing::warn!(
                    "Span {}:{}-{} out of bounds (file has {} chars)",
                    item.span_ref.span_id,
                    char_start,
                    char_end,
                    total_chars
                );
                item.text = format!(
                    "[ERROR: Span offset {}-{} out of bounds for file {} ({} chars)]",
                    char_start, char_end, file_path, total_chars
                );
            } else {
                let end_idx = char_end.min(total_chars);
                let span_chars = &chars[char_start..end_idx];
                item.text = span_chars.iter().collect();
            }
            
            total_tokens += item.span_ref.token_cost;
        }
        
        Ok(WorkingSet {
            spans: selected,
            total_tokens,
        })
    }
    
    fn estimate_top_k(&self, budget_tokens: usize) -> usize {
        // Heuristic: request 10x budget from each generator
        (budget_tokens / 50).max(100)
    }
    
    /// Fetch session context if available
    async fn fetch_contextual_enrichment(&self, req: &CompileRequest) -> Result<Vec<WSItem>> {
        // Fetch session context
        let session_spans = if let (Some(ref client), Some(ref session_id)) = (&self.session_client, &req.session_id) {
            match self.fetch_session_context(client, session_id).await {
                Ok(spans) => {
                    info!("Retrieved {} session context spans", spans.len());
                    spans
                }
                Err(e) => {
                    warn!("Failed to fetch session context: {:?}. Continuing without session context.", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        
        Ok(session_spans)
    }
    
    /// Fetch and convert session context to WSItem spans
    async fn fetch_session_context(
        &self,
        client: &SessionClient,
        session_id: &str,
    ) -> Result<Vec<WSItem>> {
        debug!("Fetching session context for session_id={}", session_id);
        
        let context = client.get_session_context(session_id, Some(10)).await?;
        
        let mut spans = Vec::new();
        for (idx, span) in context.context_spans.iter().enumerate() {
            let span_id = format!("session-{}-{}", session_id, idx);
            let token_cost = span.token_estimate;
            
            let ws_item = WSItem {
                span_ref: SpanRef {
                    span_id: span_id.clone(),
                    doc_version_id: format!("session:{}", session_id),
                    char_start: 0,
                    char_end: span.text.len(),
                    token_cost,
                },
                text: span.text.clone(),
                metadata: SpanMetadata {
                    filepath: format!("session/{}", session_id),
                    workstream: None,
                    stage: None,
                    section_title: None,
                    created_at: 0,
                    recency_score: 1.0,
                    source_type: SourceType::Session,
                    tags: vec!["session".to_string(), "conversation".to_string()],
                },
            };
            
            spans.push(ws_item);
        }
        
        Ok(spans)
    }
    
    /// Fetch and convert memory context to candidate spans so it can
    /// participate in MMR selection alongside other generators.
    async fn fetch_memory_candidates(&self, req: &CompileRequest) -> Result<Vec<CandidateSpan>> {
        use std::time::SystemTime;
        
        let (client, user_id) = match (&self.memory_client, &req.user_id) {
            (Some(client), Some(user_id)) => (client, user_id),
            _ => return Ok(Vec::new()),
        };
        
        debug!("Fetching memory candidates for user_id={}, query={}", user_id, req.intent);
        
        // Allow tuning number of memory candidates via env var
        let max_candidates: usize = std::env::var("AXONGRAPH_MEMORY_MAX_CANDIDATES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        
        let response = client.get_memories(user_id, &req.intent, Some(max_candidates)).await?;
        
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let mut candidates = Vec::new();
        for (idx, mem) in response.memories.into_iter().enumerate() {
            let text = mem.text;
            if text.trim().is_empty() {
                continue;
            }
            
            let token_cost = (text.len() / 4).max(10);
            let span_ref = SpanRef {
                doc_version_id: format!("memory:{}", user_id),
                span_id: format!("memory-{}-{}", user_id, idx),
                char_start: 0,
                char_end: text.len(),
                token_cost,
            };
            
            let semantic_score = mem.relevance.unwrap_or(0.8) as f32;
            
            // Derive stage/tags from memory category and source
            let category = mem.category.clone();
            let (stage, mut tags) = if let Some(ref cat) = category {
                let stage = match cat.as_str() {
                    "user_preferences" | "preferences" => Some("memory_prefs".to_string()),
                    "project_context" => Some("memory_project".to_string()),
                    "decisions" | "project_decisions" => Some("memory_decisions".to_string()),
                    _ => Some("memory".to_string()),
                };
                let mut tags = vec!["memory".to_string(), mem.source.clone(), cat.clone()];
                (stage, tags)
            } else {
                let tags = vec!["memory".to_string(), mem.source.clone()];
                (Some("memory".to_string()), tags)
            };
            
            let candidate = CandidateSpan {
                span_ref,
                scores: ScoreChannels {
                    semantic: semantic_score,
                    lexical: 0.0,
                    structural: 0.0,
                    graph: 0.0,
                },
                // No embedding for now; treat as independent channel
                embedding: None,
                // For memories, store full text as preview so we can
                // carry it through to WSItem without filesystem hydration.
                text_preview: text,
                metadata: SpanMetadata {
                    filepath: format!("memory/{}", user_id),
                    workstream: None,
                    stage,
                    section_title: None,
                    created_at: now,
                    recency_score: 0.95,
                    source_type: SourceType::Memory,
                    tags,
                },
                base_score: 0.0,
                mmr_score: 0.0,
            };
            
            candidates.push(candidate);
        }
        
        info!("Retrieved {} memory candidates for user {}", candidates.len(), user_id);
        
        Ok(candidates)
    }
}

fn compute_source_distribution(ws: &WorkingSet) -> HashMap<String, usize> {
    let mut dist = HashMap::new();
    for item in &ws.spans {
        *dist.entry(item.span_ref.doc_version_id.clone()).or_insert(0) += item.span_ref.token_cost;
    }
    dist
}
