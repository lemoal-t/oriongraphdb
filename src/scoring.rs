//! Scoring functions for candidate spans

use crate::types::*;

/// Normalize scores across all candidates per channel
pub fn normalize_scores(candidates: &mut [CandidateSpan]) {
    if candidates.is_empty() {
        return;
    }
    
    // Find min/max for each channel
    let mut sem_min = f32::MAX;
    let mut sem_max = f32::MIN;
    let mut lex_min = f32::MAX;
    let mut lex_max = f32::MIN;
    let mut struct_min = f32::MAX;
    let mut struct_max = f32::MIN;
    let mut graph_min = f32::MAX;
    let mut graph_max = f32::MIN;
    
    for cand in candidates.iter() {
        sem_min = sem_min.min(cand.scores.semantic);
        sem_max = sem_max.max(cand.scores.semantic);
        lex_min = lex_min.min(cand.scores.lexical);
        lex_max = lex_max.max(cand.scores.lexical);
        struct_min = struct_min.min(cand.scores.structural);
        struct_max = struct_max.max(cand.scores.structural);
        graph_min = graph_min.min(cand.scores.graph);
        graph_max = graph_max.max(cand.scores.graph);
    }
    
    // Normalize each channel
    for cand in candidates.iter_mut() {
        cand.scores.semantic = normalize_channel(cand.scores.semantic, sem_min, sem_max);
        cand.scores.lexical = normalize_channel(cand.scores.lexical, lex_min, lex_max);
        cand.scores.structural = normalize_channel(cand.scores.structural, struct_min, struct_max);
        cand.scores.graph = normalize_channel(cand.scores.graph, graph_min, graph_max);
    }
}

fn normalize_channel(x: f32, min: f32, max: f32) -> f32 {
    if max <= min + 1e-6 {
        // No spread; all values are the same. Return 1.0 if non-zero, 0.0 if zero
        if x > 1e-6 { 1.0 } else { 0.0 }
    } else {
        (x - min) / (max - min)
    }
}

/// Compute base score from weighted channel scores
pub fn compute_base_score(cand: &CandidateSpan, prefs: &SoftPreferences) -> f32 {
    let weights = &prefs.score_weights;
    
    let mut score = 
        weights.semantic * cand.scores.semantic +
        weights.lexical * cand.scores.lexical +
        weights.structural * cand.scores.structural +
        weights.graph * cand.scores.graph +
        weights.recency * cand.metadata.recency_score;
    
    // Stage boost
    if let Some(ref stage) = cand.metadata.stage {
        if prefs.prefer_stages.contains(stage) {
            score += weights.stage_boost;
        }
    }
    
    score
}

/// Cosine similarity for embeddings (assumes unit-normalized vectors)
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

