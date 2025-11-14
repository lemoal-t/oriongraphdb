//! Candidate generators for multi-channel retrieval

use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for pluggable candidate generators
#[async_trait]
pub trait CandidateGenerator: Send + Sync {
    fn name(&self) -> &'static str;
    
    async fn generate(
        &self,
        signals: &DerivedSignals,
        filters: &HardFilters,
        top_k: usize,
    ) -> Result<Vec<CandidateSpan>>;
}

/// Mock semantic generator for testing
pub struct MockSemanticGen {
    candidates: Vec<CandidateSpan>,
}

impl MockSemanticGen {
    pub fn new(candidates: Vec<CandidateSpan>) -> Self {
        Self { candidates }
    }
}

#[async_trait]
impl CandidateGenerator for MockSemanticGen {
    fn name(&self) -> &'static str {
        "mock_semantic"
    }
    
    async fn generate(
        &self,
        _signals: &DerivedSignals,
        _filters: &HardFilters,
        top_k: usize,
    ) -> Result<Vec<CandidateSpan>> {
        Ok(self.candidates.iter().take(top_k).cloned().collect())
    }
}

/// Mock lexical generator for testing
pub struct MockLexicalGen {
    candidates: Vec<CandidateSpan>,
}

impl MockLexicalGen {
    pub fn new(candidates: Vec<CandidateSpan>) -> Self {
        Self { candidates }
    }
}

#[async_trait]
impl CandidateGenerator for MockLexicalGen {
    fn name(&self) -> &'static str {
        "mock_lexical"
    }
    
    async fn generate(
        &self,
        _signals: &DerivedSignals,
        _filters: &HardFilters,
        top_k: usize,
    ) -> Result<Vec<CandidateSpan>> {
        Ok(self.candidates.iter().take(top_k).cloned().collect())
    }
}

/// Mock structural generator for testing
pub struct MockStructuralGen {
    candidates: Vec<CandidateSpan>,
}

impl MockStructuralGen {
    pub fn new(candidates: Vec<CandidateSpan>) -> Self {
        Self { candidates }
    }
}

#[async_trait]
impl CandidateGenerator for MockStructuralGen {
    fn name(&self) -> &'static str {
        "mock_structural"
    }
    
    async fn generate(
        &self,
        _signals: &DerivedSignals,
        _filters: &HardFilters,
        top_k: usize,
    ) -> Result<Vec<CandidateSpan>> {
        Ok(self.candidates.iter().take(top_k).cloned().collect())
    }
}

