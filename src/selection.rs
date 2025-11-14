//! MMR-based selection with token budget and diversity constraints

use crate::types::*;
use crate::scoring::cosine_similarity;
use anyhow::Result;
use std::collections::HashMap;

/// Select spans using MMR + knapsack constraints
pub async fn select_with_mmr(
    mut candidates: Vec<CandidateSpan>,
    budget_tokens: usize,
    prefs: &SoftPreferences,
    explain: bool,
) -> Result<(Vec<WSItem>, Vec<SpanExplanation>)> {
    
    if candidates.is_empty() {
        return Ok((vec![], vec![]));
    }
    
    // Sort by base_score descending for efficient pruning
    candidates.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());
    
    // Early pruning: keep top 5x budget (heuristic)
    let max_candidates = (budget_tokens / 100).max(200);
    candidates.truncate(max_candidates);
    
    let lambda = prefs.diversity_lambda;
    let max_source_ratio = prefs.max_single_source_ratio;
    
    let mut selected: Vec<WSItem> = Vec::new();
    let mut used_tokens: usize = 0;
    let mut source_tokens: HashMap<String, usize> = HashMap::new();
    let mut explanations: Vec<SpanExplanation> = Vec::new();
    
    // Track embeddings of selected spans for similarity computation
    let mut selected_embeddings: Vec<Vec<f32>> = Vec::new();
    
    while used_tokens < budget_tokens && !candidates.is_empty() {
        // Find best candidate by MMR
        let (best_idx, best_score, diversity_penalty) = find_best_mmr(
            &candidates,
            &selected_embeddings,
            lambda,
        )?;
        
        // Check constraints BEFORE removing candidate
        let candidate = &candidates[best_idx];
        let token_cost = candidate.span_ref.token_cost;
        
        // Check budget constraint
        if used_tokens + token_cost > budget_tokens {
            // Try to find smaller span that fits
            if let Some(smaller_idx) = find_smaller_span(&candidates, budget_tokens - used_tokens) {
                let smaller = candidates.remove(smaller_idx);
                // Re-check if it fits
                if used_tokens + smaller.span_ref.token_cost > budget_tokens {
                    break;
                }
                // Process smaller span below (use it as candidate)
                let candidate = smaller;
                let token_cost = candidate.span_ref.token_cost;
                let source = &candidate.metadata.filepath;
                
                // Check source constraint for smaller span
                let new_source_tokens = source_tokens.get(source).unwrap_or(&0) + token_cost;
                let new_total = used_tokens + token_cost;
                
                if new_source_tokens as f32 > max_source_ratio * new_total as f32 {
                    // Skip and continue
                    continue;
                }
                
                // Accept smaller span (code duplicated below - will refactor)
                let ws_item = WSItem {
                    span_ref: candidate.span_ref.clone(),
                    // For memory candidates, carry the full text from text_preview.
                    // Other sources are hydrated from the filesystem later.
                    text: if candidate.metadata.source_type == SourceType::Memory {
                        candidate.text_preview.clone()
                    } else {
                        String::new()
                    },
                    metadata: candidate.metadata.clone(),
                };
                
                selected.push(ws_item);
                used_tokens += token_cost;
                *source_tokens.entry(source.clone()).or_insert(0) += token_cost;
                
                if let Some(emb) = candidate.embedding.clone() {
                    selected_embeddings.push(emb);
                }
                
                if explain {
                    let reasons = explain_candidate(&candidate, best_score);
                    explanations.push(SpanExplanation {
                        span_ref: candidate.span_ref.clone(),
                        final_score: best_score,
                        base_score: candidate.base_score,
                        diversity_penalty,
                        reasons,
                    });
                }
                continue;
            }
            break; // no more fits
        }
        
        // Check source diversity constraint (clone source to avoid borrow issues)
        // Only apply after we have a minimum baseline (need multiple spans for diversity)
        // AND only if we actually have multiple sources available
        let source = candidate.metadata.filepath.clone();
        let new_source_tokens = source_tokens.get(&source).unwrap_or(&0) + token_cost;
        let new_total = used_tokens + token_cost;
        
        // Count unique sources in remaining candidates + selected
        let mut unique_sources: std::collections::HashSet<String> = 
            source_tokens.keys().cloned().collect();
        for c in &candidates {
            unique_sources.insert(c.metadata.filepath.clone());
        }
        
        // Apply constraint only after we have baseline AND multiple sources exist
        let min_baseline_tokens = (budget_tokens as f32 * 0.2) as usize;
        let min_baseline_spans = 3;
        let has_multiple_sources = unique_sources.len() > 1;
        
        if has_multiple_sources && (selected.len() >= min_baseline_spans || used_tokens >= min_baseline_tokens) {
            if new_source_tokens as f32 > max_source_ratio * new_total as f32 {
                // Remove this candidate and try next
                tracing::debug!("  SKIP: source diversity ({:.1}%)", new_source_tokens as f32 / new_total as f32 * 100.0);
                candidates.remove(best_idx);
                continue;
            }
        }
        
        // Now actually remove and accept the candidate
        let candidate = candidates.remove(best_idx);
        
        // Accept span
        let ws_item = WSItem {
            span_ref: candidate.span_ref.clone(),
            // For memory candidates, carry the full text from text_preview.
            // Other sources are hydrated from the filesystem later.
            text: if candidate.metadata.source_type == SourceType::Memory {
                candidate.text_preview.clone()
            } else {
                String::new()
            },
            metadata: candidate.metadata.clone(),
        };
        
        selected.push(ws_item);
        used_tokens += token_cost;
        *source_tokens.entry(source).or_insert(0) += token_cost;
        
        // Store embedding for future similarity checks
        if let Some(emb) = candidate.embedding.clone() {
            selected_embeddings.push(emb);
        }
        
        // Record explanation
        if explain {
            let reasons = explain_candidate(&candidate, best_score);
            explanations.push(SpanExplanation {
                span_ref: candidate.span_ref.clone(),
                final_score: best_score,
                base_score: candidate.base_score,
                diversity_penalty,
                reasons,
            });
        }
    }
    
    Ok((selected, explanations))
}

/// Find candidate with highest MMR score
fn find_best_mmr(
    candidates: &[CandidateSpan],
    selected_embeddings: &[Vec<f32>],
    lambda: f32,
) -> Result<(usize, f32, f32)> {
    let mut best_idx = 0;
    let mut best_mmr = f32::MIN;
    let mut best_penalty = 0.0;
    
    for (idx, cand) in candidates.iter().enumerate() {
        // Compute max similarity to already-selected spans
        let max_sim = if selected_embeddings.is_empty() {
            0.0
        } else if let Some(ref emb) = cand.embedding {
            selected_embeddings
                .iter()
                .map(|sel_emb| cosine_similarity(emb, sel_emb))
                .fold(0.0f32, f32::max)
        } else {
            0.0 // no embedding, assume no similarity
        };
        
        let diversity_penalty = (1.0 - lambda) * max_sim;
        let mmr_score = lambda * cand.base_score - diversity_penalty;
        
        // Tie-breaker: prefer lower token cost when scores are close
        let is_better = if (mmr_score - best_mmr).abs() < 0.01 {
            cand.span_ref.token_cost < candidates[best_idx].span_ref.token_cost
        } else {
            mmr_score > best_mmr
        };
        
        if is_better {
            best_mmr = mmr_score;
            best_idx = idx;
            best_penalty = diversity_penalty;
        }
    }
    
    Ok((best_idx, best_mmr, best_penalty))
}

fn find_smaller_span(candidates: &[CandidateSpan], max_tokens: usize) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.span_ref.token_cost <= max_tokens)
        .max_by(|(_, a), (_, b)| a.base_score.partial_cmp(&b.base_score).unwrap())
        .map(|(idx, _)| idx)
}

fn explain_candidate(cand: &CandidateSpan, final_score: f32) -> Vec<String> {
    let mut reasons = Vec::new();
    
    if cand.scores.semantic > 0.5 {
        reasons.push(format!("semantic match: {:.2}", cand.scores.semantic));
    }
    if cand.scores.lexical > 0.5 {
        reasons.push(format!("keyword match: {:.2}", cand.scores.lexical));
    }
    if cand.scores.structural > 0.5 {
        reasons.push(format!("structural relevance: {:.2}", cand.scores.structural));
    }
    if let Some(ref section) = cand.metadata.section_title {
        reasons.push(format!("section: {}", section));
    }
    if let Some(ref stage) = cand.metadata.stage {
        reasons.push(format!("stage: {}", stage));
    }
    
    reasons.push(format!("final MMR score: {:.2}", final_score));
    
    reasons
}
