//! AxonGraph HTTP server binary

use oriongraph::{ContextEngine, MockSemanticGen, HttpSemanticGen, HttpLexicalGen, CandidateSpan};
use tracing_subscriber;
use std::sync::Arc;

mod server {
    pub use oriongraph::server::*;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();
    
    println!("🚀 AxonGraph Context Compiler");
    println!("   Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    
    // Check for --use-real flag
    let use_real = std::env::args().any(|arg| arg == "--use-real");
    
    let engine = if use_real {
        println!("✓ Mode: REAL indices (Python services)");
        let semantic_service_url = std::env::var("SEMANTIC_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8083".to_string());
        let lexical_service_url = std::env::var("LEXICAL_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8084".to_string());
        
        println!("✓ Semantic service: {}", semantic_service_url);
        println!("✓ Lexical service: {}", lexical_service_url);
        
        // Test connection to semantic service
        let client = reqwest::Client::new();
        match client.get(format!("{}/health", semantic_service_url)).send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("✓ Semantic service is healthy");
            }
            Ok(resp) => {
                eprintln!("⚠️  Semantic service returned status: {}", resp.status());
            }
            Err(e) => {
                eprintln!("❌ Failed to connect to semantic service: {}", e);
                eprintln!("   Make sure it's running: python indexing/semantic_generator_service.py --port 8083");
                return Err(e.into());
            }
        }
        
        // Test connection to lexical service
        match client.get(format!("{}/health", lexical_service_url)).send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("✓ Lexical service is healthy");
            }
            Ok(resp) => {
                eprintln!("⚠️  Lexical service returned status: {}", resp.status());
            }
            Err(e) => {
                eprintln!("❌ Failed to connect to lexical service: {}", e);
                eprintln!("   Make sure it's running: python indexing/lexical_generator_service.py --index-path .orion/indices/lexical/bm25_index.pkl --port 8084");
                return Err(e.into());
            }
        }
        
        // Check for session/memory API URLs from environment
        let session_api_url = std::env::var("SESSION_API_URL").ok();
        let memory_api_url = std::env::var("MEMORY_API_URL").ok();
        
        if let Some(ref url) = session_api_url {
            println!("✓ Session API enabled: {}", url);
        }
        if let Some(ref url) = memory_api_url {
            println!("✓ Memory API enabled: {}", url);
        }
        
        ContextEngine::new(
            vec![
                Box::new(HttpSemanticGen::new(semantic_service_url)),
                Box::new(HttpLexicalGen::new(lexical_service_url)),
            ],
            session_api_url.clone(),
            memory_api_url.clone(),
        )
    } else {
        println!("✓ Mode: MOCK semantic index");
        println!("   (use --use-real to enable real FAISS index)");
        let mock_candidates = create_mock_candidates();
        
        // Check for session/memory API URLs from environment
        let session_api_url = std::env::var("SESSION_API_URL").ok();
        let memory_api_url = std::env::var("MEMORY_API_URL").ok();
        
        ContextEngine::new(
            vec![
                Box::new(MockSemanticGen::new(mock_candidates)),
            ],
            session_api_url,
            memory_api_url,
        )
    };
    
    println!("✓ Context engine initialized");
    println!("✓ Starting HTTP server on port 8081...");
    println!();
    
    server::run_server(engine, 8081).await?;
    
    Ok(())
}

/// Create mock candidates for evaluation with real OrionFS content
fn create_mock_candidates() -> Vec<CandidateSpan> {
    use oriongraph::*;
    use std::time::SystemTime;
    
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    vec![
        // Span 1: Retrieval-as-Compilation Core Concept
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_adr_retrieval".to_string(),
                span_id: "span_retrieval_as_compilation".to_string(),
                char_start: 0,
                char_end: 800,
                token_cost: 180,
            },
            scores: ScoreChannels {
                semantic: 0.95,
                lexical: 0.90,
                structural: 0.85,
                graph: 0.8,
            },
            embedding: Some(vec![0.8, 0.7, 0.6, 0.5]),
            text_preview: "We will implement a Working Set Compiler (compile_workingset()): Treat retrieval as a compilation step, not a simple query. Model context as a set of SpanRefs with (doc_version_id, span_id, char_start, char_end, token_cost). Generate candidate spans from multiple generators: structural, lexical, semantic, graph. Normalize multi-channel scores and compute a base utility score. Use Maximal Marginal Relevance (MMR) to balance relevance and diversity.".to_string(),
            metadata: SpanMetadata {
                filepath: "03_workstreams/ws-orion/99_decisions/ADR-20251113-retrieval-as-compilation.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("decisions".to_string()),
                section_title: Some("Decision".to_string()),
                created_at: now,
                recency_score: 0.95,
                source_type: SourceType::Workstream,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 2: Problems with Naive RAG
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_adr_retrieval".to_string(),
                span_id: "span_naive_rag_problems".to_string(),
                char_start: 800,
                char_end: 1400,
                token_cost: 140,
            },
            scores: ScoreChannels {
                semantic: 0.92,
                lexical: 0.88,
                structural: 0.80,
                graph: 0.75,
            },
            embedding: Some(vec![0.75, 0.65, 0.55, 0.45]),
            text_preview: "Naive Semantic Top-K: Chunk documents (e.g., 512-1024 tokens), embed, and return top-k most similar chunks. Cons: Fails to respect token budgets precisely. Over-represents a single source. Provides no explicit notion of diversity. No built-in explainability. Chunk boundaries are arbitrary; spans may cut across logical sections. Reason Rejected: Insufficient control and traceability for multi-agent, multi-step workflows.".to_string(),
            metadata: SpanMetadata {
                filepath: "03_workstreams/ws-orion/99_decisions/ADR-20251113-retrieval-as-compilation.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("decisions".to_string()),
                section_title: Some("Alternatives Considered: Naive Semantic Top-K".to_string()),
                created_at: now,
                recency_score: 0.95,
                source_type: SourceType::Workstream,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 3: Span Model Design
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_spec_workingset".to_string(),
                span_id: "span_spanref_design".to_string(),
                char_start: 0,
                char_end: 600,
                token_cost: 150,
            },
            scores: ScoreChannels {
                semantic: 0.93,
                lexical: 0.85,
                structural: 0.90,
                graph: 0.70,
            },
            embedding: Some(vec![0.72, 0.68, 0.62, 0.58]),
            text_preview: "SpanRef is the addressable unit of reading. It contains: doc_version_id (SHA256 of document bytes), span_id (stable within version, UUID or derived), char_start and char_end offsets, and token_cost for budget tracking. The doc_version_id ensures no drift: spans are immutable per version. The span_id plus offsets allow exact quoting and provenance. This design solves the chunk boundary problem by making spans explicitly addressable and version-stable.".to_string(),
            metadata: SpanMetadata {
                filepath: "docs/AXONGRAPH_WORKINGSET_SPEC.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("design".to_string()),
                section_title: Some("Core Data Model: SpanRef".to_string()),
                created_at: now - 3600,
                recency_score: 0.90,
                source_type: SourceType::Knowledge,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 4: MMR Algorithm Details
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_spec_workingset".to_string(),
                span_id: "span_mmr_algorithm".to_string(),
                char_start: 600,
                char_end: 1200,
                token_cost: 160,
            },
            scores: ScoreChannels {
                semantic: 0.94,
                lexical: 0.89,
                structural: 0.88,
                graph: 0.72,
            },
            embedding: Some(vec![0.78, 0.71, 0.65, 0.59]),
            text_preview: "MMR (Maximal Marginal Relevance) balances relevance and diversity. At each selection step: MMR(span) = λ * base_score(span) - (1 - λ) * max_sim(span, selected). Lambda is the diversity_lambda parameter in SoftPreferences. max_sim uses cosine similarity between embeddings. This ensures we don't select redundant spans that are too similar to already-selected content, maintaining diversity while respecting relevance.".to_string(),
            metadata: SpanMetadata {
                filepath: "docs/AXONGRAPH_WORKINGSET_SPEC.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("design".to_string()),
                section_title: Some("Selection Algorithm: MMR".to_string()),
                created_at: now - 3600,
                recency_score: 0.90,
                source_type: SourceType::Knowledge,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 5: Token Budget Constraints
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_spec_workingset".to_string(),
                span_id: "span_budget_constraints".to_string(),
                char_start: 1200,
                char_end: 1700,
                token_cost: 130,
            },
            scores: ScoreChannels {
                semantic: 0.90,
                lexical: 0.82,
                structural: 0.85,
                graph: 0.68,
            },
            embedding: Some(vec![0.70, 0.66, 0.60, 0.54]),
            text_preview: "Selection algorithm goals: Maximize relevance to intent, diversity across content, and source mix (avoid depending on 1 doc). Subject to constraints: total_tokens <= budget_tokens, and tokens_from_single_source <= max_single_source_ratio * total_tokens. This implements a knapsack-style optimization where each span has a cost (tokens) and utility (base_score), ensuring we stay within budget while maximizing value.".to_string(),
            metadata: SpanMetadata {
                filepath: "docs/AXONGRAPH_WORKINGSET_SPEC.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("design".to_string()),
                section_title: Some("Selection Algorithm: Goals and Constraints".to_string()),
                created_at: now - 3600,
                recency_score: 0.90,
                source_type: SourceType::Knowledge,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 6: Agent-First Design Goals
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_bold_vision".to_string(),
                span_id: "span_agent_first_goals".to_string(),
                char_start: 0,
                char_end: 700,
                token_cost: 170,
            },
            scores: ScoreChannels {
                semantic: 0.88,
                lexical: 0.80,
                structural: 0.82,
                graph: 0.65,
            },
            embedding: Some(vec![0.68, 0.64, 0.58, 0.52]),
            text_preview: "Agent-first I/O: Optimize for read patterns agents use: skim → narrow → deep read → quote spans → reason → write notes. Span precision: Address any byte/char/token range with stable IDs. Return exact snippets, not just whole chunks. Cost-aware retrieval: Compile a minimal working set context for a step—bounded by token budget and latency. Trust & provenance: Every span has lineage (source file, hash, time, transform pipeline).".to_string(),
            metadata: SpanMetadata {
                filepath: "bold.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("research".to_string()),
                section_title: Some("Design Goals".to_string()),
                created_at: now - 7200,
                recency_score: 0.85,
                source_type: SourceType::Context,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 7: Chunk Boundary Problems
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_bold_vision".to_string(),
                span_id: "span_chunk_problems".to_string(),
                char_start: 700,
                char_end: 1200,
                token_cost: 125,
            },
            scores: ScoreChannels {
                semantic: 0.91,
                lexical: 0.86,
                structural: 0.78,
                graph: 0.70,
            },
            embedding: Some(vec![0.73, 0.67, 0.61, 0.55]),
            text_preview: "Why span-centric? Chunking is brittle. Addressable spans let agents pull exactly what they need and stitch coherent narratives without re-reading whole files. Chunk boundaries are arbitrary and often cut across logical sections, breaking context. Spans with stable IDs and precise offsets solve this by allowing exact addressability and quotation, maintaining semantic coherence even as documents evolve.".to_string(),
            metadata: SpanMetadata {
                filepath: "bold.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("research".to_string()),
                section_title: Some("Why Span-Centric".to_string()),
                created_at: now - 7200,
                recency_score: 0.85,
                source_type: SourceType::Context,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 8: Multi-Channel Scoring
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_spec_workingset".to_string(),
                span_id: "span_multi_channel_scoring".to_string(),
                char_start: 1700,
                char_end: 2200,
                token_cost: 145,
            },
            scores: ScoreChannels {
                semantic: 0.89,
                lexical: 0.84,
                structural: 0.87,
                graph: 0.73,
            },
            embedding: Some(vec![0.71, 0.65, 0.59, 0.53]),
            text_preview: "Base score computation uses weighted multi-channel signals: base_score(span) = w_sem * semantic + w_lex * lexical + w_struct * structural + w_graph * graph + w_recency * recency_score + w_stage * stage_boost. Default weights: semantic 0.4, lexical 0.2, structural 0.2, graph 0.1, recency 0.05, stage_boost 0.05. Each channel is normalized using min-max normalization across all candidates before combining.".to_string(),
            metadata: SpanMetadata {
                filepath: "docs/AXONGRAPH_WORKINGSET_SPEC.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("design".to_string()),
                section_title: Some("Scoring & Normalization".to_string()),
                created_at: now - 3600,
                recency_score: 0.90,
                source_type: SourceType::Knowledge,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 9: Source Diversity Constraint
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_spec_workingset".to_string(),
                span_id: "span_source_diversity".to_string(),
                char_start: 2200,
                char_end: 2650,
                token_cost: 120,
            },
            scores: ScoreChannels {
                semantic: 0.87,
                lexical: 0.81,
                structural: 0.83,
                graph: 0.69,
            },
            embedding: Some(vec![0.69, 0.63, 0.57, 0.51]),
            text_preview: "Source diversity is enforced during selection: at each step, check if adding the candidate would cause tokens_from_source / total_tokens to exceed max_single_source_ratio. If so, skip this candidate and try the next highest-scoring one. This prevents over-reliance on a single document and ensures the working set draws from multiple perspectives, improving robustness and reducing bias.".to_string(),
            metadata: SpanMetadata {
                filepath: "docs/AXONGRAPH_WORKINGSET_SPEC.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("design".to_string()),
                section_title: Some("Source Diversity Enforcement".to_string()),
                created_at: now - 3600,
                recency_score: 0.90,
                source_type: SourceType::Knowledge,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
        
        // Span 10: WorkPlan Orchestration Concept
        CandidateSpan {
            span_ref: SpanRef {
                doc_version_id: "doc_bold_vision".to_string(),
                span_id: "span_workplan_orchestration".to_string(),
                char_start: 1200,
                char_end: 1700,
                token_cost: 135,
            },
            scores: ScoreChannels {
                semantic: 0.86,
                lexical: 0.79,
                structural: 0.81,
                graph: 0.66,
            },
            embedding: Some(vec![0.67, 0.61, 0.55, 0.49]),
            text_preview: "WorkPlan execution requires DAG validation to detect cycles, task state tracking (PENDING/READY/RUNNING/DONE/FAILED), and dependency resolution. Tasks become READY when all dependencies are DONE. The executor must identify ready tasks, dispatch them to appropriate role-specific agents, track completion, and handle failures. Integration with AxonGraph provides context for each task via compile_workingset.".to_string(),
            metadata: SpanMetadata {
                filepath: "bold.md".to_string(),
                workstream: Some("ws-orion".to_string()),
                stage: Some("design".to_string()),
                section_title: Some("Multi-Agent Orchestration".to_string()),
                created_at: now - 7200,
                recency_score: 0.85,
                source_type: SourceType::Context,
                tags: vec![],
            },
            base_score: 0.0,
            mmr_score: 0.0,
        },
    ]
}

