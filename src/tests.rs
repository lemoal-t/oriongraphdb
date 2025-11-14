//! Unit tests for ContextEngine

use crate::*;
use std::time::SystemTime;

/// Helper to create mock candidate
fn mock_candidate(
    id: &str,
    sem_score: f32,
    lex_score: f32,
    tokens: usize,
    filepath: &str,
    embedding: Option<Vec<f32>>,
) -> CandidateSpan {
    CandidateSpan {
        span_ref: SpanRef {
            doc_version_id: format!("doc_{}", filepath),
            span_id: id.to_string(),
            char_start: 0,
            char_end: 100,
            token_cost: tokens,
        },
        scores: ScoreChannels {
            semantic: sem_score,
            lexical: lex_score,
            structural: 0.5,
            graph: 0.0,
        },
        embedding,
        text_preview: format!("Preview of span {}", id),
        metadata: SpanMetadata {
            filepath: filepath.to_string(),
            workstream: Some("ws-test".to_string()),
            stage: Some("research".to_string()),
            section_title: Some("Test Section".to_string()),
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            tags: vec![],
            recency_score: 0.8,
            source_type: SourceType::Workstream,
        },
        base_score: 0.0,
        mmr_score: 0.0,
    }
}

#[tokio::test]
async fn test_mmr_diversity() {
    // Create candidates: 5 about "risk" (high similarity), 5 about "rollback" (different topic)
    let risk_emb = vec![1.0, 0.0, 0.0]; // normalized
    let rollback_emb = vec![0.0, 1.0, 0.0];
    
    let mut candidates = vec![];
    
    // High-scoring risk spans
    for i in 0..5 {
        candidates.push(mock_candidate(
            &format!("risk_{}", i),
            0.9 - (i as f32 * 0.05),
            0.8,
            500,
            "risks.md",
            Some(risk_emb.clone()),
        ));
    }
    
    // Lower-scoring rollback spans
    for i in 0..5 {
        candidates.push(mock_candidate(
            &format!("rollback_{}", i),
            0.7 - (i as f32 * 0.05),
            0.7,
            500,
            "rollback.md",
            Some(rollback_emb.clone()),
        ));
    }
    
    // With high diversity (lambda=0.3), should get mix of both topics
    // Disable source ratio constraint for this test (testing MMR, not source diversity)
    let prefs = SoftPreferences {
        diversity_lambda: 0.3,
        max_single_source_ratio: 1.0, // No constraint
        ..Default::default()
    };
    
    // Score candidates first
    use crate::scoring::{normalize_scores, compute_base_score};
    normalize_scores(&mut candidates);
    for cand in &mut candidates {
        cand.base_score = compute_base_score(cand, &prefs);
    }
    
    let (selected, _) = selection::select_with_mmr(
        candidates,
        5000,
        &prefs,
        false,
    ).await.unwrap();
    
    // Check that we got both risk and rollback spans
    let risk_count = selected.iter().filter(|s| s.span_ref.span_id.starts_with("risk")).count();
    let rollback_count = selected.iter().filter(|s| s.span_ref.span_id.starts_with("rollback")).count();
    
    assert!(risk_count > 0, "Should include risk spans");
    assert!(rollback_count > 0, "Should include rollback spans for diversity");
    assert!(selected.len() >= 8, "Should select multiple spans");
}

#[tokio::test]
async fn test_source_diversity_constraint() {
    // 8 high-score candidates from doc_A, 2 low-score from doc_B
    let mut candidates = vec![];
    
    for i in 0..8 {
        candidates.push(mock_candidate(
            &format!("a_{}", i),
            0.9,
            0.8,
            500,
            "doc_A",
            None,
        ));
    }
    
    for i in 0..2 {
        candidates.push(mock_candidate(
            &format!("b_{}", i),
            0.5,
            0.5,
            500,
            "doc_B",
            None,
        ));
    }
    
    let prefs = SoftPreferences {
        max_single_source_ratio: 0.55, // Allow up to 55% (reasonable for 2 sources)
        diversity_lambda: 0.0, // pure utility to stress-test source constraint
        ..Default::default()
    };
    
    // Score
    use crate::scoring::{normalize_scores, compute_base_score};
    normalize_scores(&mut candidates);
    for cand in &mut candidates {
        cand.base_score = compute_base_score(cand, &prefs);
    }
    
    let (selected, _) = selection::select_with_mmr(
        candidates,
        5000,
        &prefs,
        false,
    ).await.unwrap();
    
    // Calculate doc_A token contribution
    let total_tokens: usize = selected.iter().map(|s| s.span_ref.token_cost).sum();
    let doc_a_tokens: usize = selected.iter()
        .filter(|s| s.span_ref.doc_version_id == "doc_doc_A")
        .map(|s| s.span_ref.token_cost)
        .sum();
    
    let doc_a_ratio = doc_a_tokens as f32 / total_tokens as f32;
    
    assert!(doc_a_ratio <= 0.56, "doc_A should not exceed 55% (allowing small margin): {:.2}", doc_a_ratio);
}

#[tokio::test]
async fn test_token_budget_respected() {
    let mut candidates = vec![];
    
    for i in 0..20 {
        candidates.push(mock_candidate(
            &format!("span_{}", i),
            0.8,
            0.7,
            300,
            "test.md",
            None,
        ));
    }
    
    let budget = 3000;
    let prefs = SoftPreferences::default();
    
    // Score
    use crate::scoring::{normalize_scores, compute_base_score};
    normalize_scores(&mut candidates);
    for cand in &mut candidates {
        cand.base_score = compute_base_score(cand, &prefs);
    }
    
    let (selected, _) = selection::select_with_mmr(
        candidates,
        budget,
        &prefs,
        false,
    ).await.unwrap();
    
    let total_tokens: usize = selected.iter().map(|s| s.span_ref.token_cost).sum();
    
    assert!(total_tokens <= budget, "Should not exceed budget");
    assert!(
        total_tokens >= (budget as f32 * 0.85) as usize,
        "Should achieve >85% utilization: {} / {}",
        total_tokens,
        budget
    );
}

#[tokio::test]
async fn test_end_to_end_compile() {
    // Create mock generators
    let mut candidates = vec![];
    for i in 0..10 {
        candidates.push(mock_candidate(
            &format!("span_{}", i),
            0.7 + (i as f32 * 0.02),
            0.6,
            400,
            "test.md",
            Some(vec![i as f32 / 10.0, 0.5, 0.5]),
        ));
    }
    
    let gen = MockSemanticGen::new(candidates);
    // Use simple constructor without session/memory integrations for tests
    let engine = ContextEngine::new_simple(vec![Box::new(gen)]);
    
    let req = CompileRequest {
        intent: "test intent".to_string(),
        task_id: None,
        session_id: None,
        user_id: None,
        query_signals: vec![],
        budget_tokens: 3000,
        hard_filters: HardFilters::default(),
        soft_prefs: SoftPreferences::default(),
        explain: true,
    };
    
    let response = engine.compile_workingset(req).await.unwrap();
    
    assert!(!response.workingset.spans.is_empty(), "Should select spans");
    assert!(response.workingset.total_tokens <= 3000, "Should respect budget");
    assert!(response.stats.token_utilization > 0.5, "Should use >50% of budget");
    assert!(response.rationale.is_some(), "Should include explanations");
    
    println!("✓ Selected {} spans", response.workingset.spans.len());
    println!("✓ Token utilization: {:.1}%", response.stats.token_utilization * 100.0);
}
