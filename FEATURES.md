# OrionGraphDB Features

**Current Version**: 0.1.0 (Alpha)

This document clarifies what's currently implemented vs. what's planned for future releases.

---

## ✅ Currently Implemented (v0.1.0)

### Core Context Compilation
- ✅ **Multi-channel candidate generation**
  - Structural generators (mock/HTTP)
  - Lexical generators with BM25 (HTTP-based)
  - Semantic generators (mock/HTTP)
- ✅ **MMR-based selection** (Maximal Marginal Relevance)
  - Diversity scoring
  - Relevance balancing
  - Token budget constraints
- ✅ **Token budget management**
  - Hard budget enforcement
  - Source diversity constraints
  - Utilization optimization
- ✅ **Span-level precision**
  - SpanRef with stable identifiers
  - Character offsets
  - Token cost tracking

### HTTP Server & API
- ✅ **REST API** (`/compile_workingset`)
  - JSON request/response
  - Async processing (Tokio + Axum)
  - Health check endpoint
- ✅ **Pluggable generators**
  - Trait-based architecture
  - HTTP-based remote generators
  - Mock generators for testing

### Integration Clients
- ✅ **Python client** (`oriongraph_client.py`)
  - Simple HTTP wrapper
  - Working examples
- ✅ **Session integration client** (Rust)
  - Query session context
  - Format for context compilation
- ✅ **Memory integration client** (Rust)
  - Query semantic memory
  - Format for retrieval

### Developer Experience
- ✅ **Quick start** with mock data
- ✅ **Cargo-based build** system
- ✅ **Basic test suite**
- ✅ **Example usage** in Python

---

## 🚧 Partially Implemented

### Scoring & Selection
- ⚠️ **Multi-channel scoring** - Infrastructure present, but weights hardcoded
  - TODO: Make weights configurable via request
  - TODO: Add per-channel score explanations
- ⚠️ **Explanations** - Basic rationale supported, but not fully detailed
  - Present: Final scores, selection rank
  - Missing: Per-channel contribution breakdown

### Storage & Indices
- ⚠️ **In-memory indices** - Current implementation uses runtime data structures
  - No persistent FAISS/HNSW indices yet
  - No persistent inverted index yet
  - Span registry is ephemeral

---

## 📋 Planned (Future Releases)

### v0.2.0 - Persistence
- 🔲 **Persistent semantic indices**
  - FAISS index storage
  - Incremental updates
  - Versioned embeddings
- 🔲 **Persistent lexical indices**
  - Inverted index on disk
  - BM25 statistics persistence
- 🔲 **Persistent structural indices**
  - Document structure cache
  - Span metadata storage

### v0.3.0 - Advanced Retrieval
- 🔲 **Graph-based retrieval**
  - Entity relationship graphs
  - Citation links
  - ADR (Architecture Decision Record) graphs
- 🔲 **Episodic context**
  - Session history integration
  - Temporal relevance scoring
  - User-specific context preferences

### v0.4.0 - Integration & Ecosystem
- 🔲 **OrionFS integration**
  - Direct filesystem layout support
  - Auto-indexing from `01_context/`, `02_knowledge/`, etc.
  - Front-matter parsing (YAML)
- 🔲 **Policy engine integration**
  - OrionFSGuard support
  - Content governance
  - Access control per span

### v0.5.0 - Optimization & Scale
- 🔲 **Working set compression**
  - Bullet-point summaries
  - LLM-based summarization
  - Backref preservation
- 🔲 **Learned re-ranking**
  - Fine-tuned model for final selection
  - User feedback loop
  - A/B testing infrastructure
- 🔲 **Multi-tenant support**
  - Tenant isolation
  - Per-tenant indices
  - Resource quotas

### Long-term Roadmap
- 🔲 **Embedded mode** (in-process library)
- 🔲 **Incremental indexing** (watch filesystem)
- 🔲 **Distributed deployment** (sharding, replication)
- 🔲 **Advanced analytics** (query performance, utilization stats)
- 🔲 **Client libraries** (Node.js, Go, Rust native)

---

## 🔍 Architecture Notes

### What OrionGraphDB IS
- A **context compilation engine** for AI agents
- A **retrieval system** optimized for LLM prompts
- A **standalone service** with HTTP API

### What OrionGraphDB IS NOT (Yet)
- Not a full document database (no CRUD on documents)
- Not a vector database (uses external embedding services)
- Not a general-purpose search engine (specialized for agent context)
- Not a filesystem manager (works with existing files)

### Design Philosophy
OrionGraphDB follows the **"Database for AI Context"** philosophy:
1. **Query-like interface** - One main operation: `compile_workingset`
2. **Budget-aware** - Always respects token limits
3. **Explainable** - Every selection has a rationale
4. **Diverse** - Avoids over-reliance on single sources
5. **Fast** - Sub-200ms for typical queries

---

## 📊 Feature Maturity Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Implemented and tested |
| ⚠️ | Partially implemented, needs work |
| 🔲 | Planned but not started |
| 🚧 | Work in progress |
| ❌ | Explicitly out of scope |

---

## 🤝 Contributing

Want to help implement a planned feature? 

1. Check the [Issues](https://github.com/servesys-labs/oriongraphdb/issues) for tracking
2. Comment on the issue to claim it
3. Submit a PR when ready

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines (coming soon).

---

**Last Updated**: November 14, 2025  
**Status**: Alpha (v0.1.0)

