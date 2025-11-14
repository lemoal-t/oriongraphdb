//! OrionGraphDB - Agent Context Compiler
//! 
//! Implements retrieval-as-compilation for agent working sets with:
//! - Span-level precision
//! - Multi-channel retrieval (structural, lexical, semantic, graph)
//! - MMR-based diversity + token budget constraints
//! - Provenance & explainability

pub mod types;
pub mod context_engine;
pub mod generators;
pub mod http_generator;
pub mod http_lexical_gen; // BM25 lexical generator
pub mod scoring;
pub mod selection;
pub mod server;
pub mod session_client;  // NEW: Session API client
pub mod memory_client;   // NEW: Memory API client

pub use types::*;
pub use context_engine::ContextEngine;
pub use generators::{CandidateGenerator, MockSemanticGen, MockLexicalGen, MockStructuralGen};
pub use http_generator::HttpSemanticGen;
pub use http_lexical_gen::HttpLexicalGen; // Export BM25 generator
pub use session_client::SessionClient;
pub use memory_client::MemoryClient;

#[cfg(test)]
mod tests;

