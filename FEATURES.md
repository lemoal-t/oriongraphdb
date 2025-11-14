# OrionGraphDB Features

**Current Version**: 0.1.0 (Production-Ready Alpha)

This document clarifies what's currently implemented vs. what's planned for future releases.

---

## ✅ Fully Implemented (v0.1.0)

### Core Context Compilation Engine

- ✅ **Multi-channel candidate generation**
  - `HttpSemanticGen` - Vector search via HTTP service
  - `HttpLexicalGen` - BM25 search via HTTP service
  - `MockSemanticGen`, `MockLexicalGen`, `MockStructuralGen` - Testing mocks
  - Trait-based `CandidateGenerator` for pluggability
  - Parallel generation with `futures::join_all`
- ✅ **MMR-based selection** (Maximal Marginal Relevance)
  - Lambda-weighted diversity vs. relevance tradeoff
  - Cosine similarity for duplicate detection
  - Embedding-based diversity scoring
  - Greedy knapsack selection algorithm
- ✅ **Token budget management**

  - Hard budget enforcement (never exceeds limit)
  - Source diversity constraints (configurable max ratio per source)
  - Graceful handling when budget exhausted
  - 85-90% utilization targets
  - Dynamic budget splitting (session context vs. retrieval)

- ✅ **Multi-channel scoring**

  - Per-channel normalization (semantic, lexical, structural, graph)
  - Configurable score weights (`ScoreWeights`)
  - Recency scoring
  - Stage boost (prefer specific workstream stages)
  - Fully weighted base score computation

- ✅ **Span-level precision**
  - `SpanRef` with stable identifiers
  - Character-based offsets (`char_start`, `char_end`)
  - Token cost pre-computation
  - Document version IDs for immutability

### Session & Memory Integration

- ✅ **Session-aware context compilation**
  - `SessionClient` for querying session history
  - `fetch_session_context()` method
  - Session spans prepended to workingset (highest priority)
  - Budget cap for session context (max 50% of total budget)
  - Automatic trimming when session context exceeds cap
- ✅ **Memory-aware retrieval**
  - `MemoryClient` for semantic memory search
  - `fetch_memory_candidates()` method
  - Memories participate in MMR selection alongside other candidates
  - Memory relevance scores integrated into scoring
  - Category-based tagging and stage assignment
  - Configurable max memory candidates via env var

### HTTP Server & API

- ✅ **Production-grade REST API**
  - `POST /compile_workingset` - Main compilation endpoint
  - `GET /health` - Health check
  - Async request handling (Tokio + Axum)
  - JSON request/response with full error handling
  - Request filtering (paths, workstreams, age)
- ✅ **Comprehensive request model**
  - Intent, session_id, user_id parameters
  - Hard filters (paths, workstreams, doc age)
  - Soft preferences (diversity, source ratio, weights)
  - Explain flag for rationale generation

### Explanations & Observability

- ✅ **Full explainability**
  - `SpanExplanation` with per-span rationale
  - Per-channel score contributions
  - Diversity penalty tracking
  - Selection reasons (semantic match, keyword match, structural relevance)
  - Token utilization statistics
  - Source distribution reporting
  - Generation time tracking

### Scoring & Normalization

- ✅ **Multi-channel score fusion**
  - Min-max normalization per channel
  - Weighted combination across channels
  - Recency scoring based on timestamps
  - Stage boost for preferred stages
  - Metadata-aware scoring (workstream, source type)

### Hydration & Text Extraction

- ✅ **Filesystem-based hydration**
  - File caching to avoid re-reads
  - Character-offset-based text extraction
  - Graceful error handling for missing files
  - Special handling for session/memory spans (text pre-loaded)
  - Bounds checking for span offsets

### Integration Clients

- ✅ **Python client** (`oriongraph_client.py`)
  - `OrionGraphClient` class
  - `compile_workingset()` method
  - Working examples in `examples/python-client/`
- ✅ **Rust integration clients**
  - `SessionClient` - Query session context via HTTP
  - `MemoryClient` - Query semantic memory via HTTP

### Developer Experience

- ✅ **Quick start** with mock data
- ✅ **Cargo-based build** system
- ✅ **Unit tests** for core algorithms
- ✅ **Binary packaging** for distribution
- ✅ **Configurable via CLI flags** (`--use-real`)

---

## 🔌 Architecture Dependencies

### External HTTP Services (Expected by Generators)

- ⚠️ **Semantic search service** - Expected at configurable URL
  - Called by `HttpSemanticGen`
  - POST `/search` endpoint with query + filters
  - Returns candidates with scores + embeddings
- ⚠️ **BM25 lexical service** - Expected at configurable URL
  - Called by `HttpLexicalGen`
  - POST `/search` endpoint with query + filters
  - Returns candidates with BM25 scores
- ⚠️ **Session API** - Optional, for session integration
  - GET `/session/{id}/context` endpoint
  - Returns recent conversation spans
- ⚠️ **Memory API** - Optional, for memory integration
  - GET `/memories?user_id={id}&query={query}` endpoint
  - Returns relevant long-term memories

> **Note**: OrionGraphDB is the **context compilation engine**. It orchestrates calls to these services but doesn't implement the indices themselves. This follows the "database for AI context" philosophy—indices can be swapped/upgraded without changing the core engine.

---

## 📋 Planned (Future Releases)

### v0.2.0 - Built-in Index Services

Currently, OrionGraphDB delegates to external HTTP services for semantic/lexical search. Future versions may include:

- 🔲 **Bundled semantic search**
  - Embedded FAISS/HNSW indices
  - Optional in-process embeddings
  - Persistent index storage
- 🔲 **Bundled lexical search**
  - Embedded inverted index
  - BM25 scoring implementation
  - Persistent index files

> **Design Note**: The current architecture (external services) is **intentional** and follows the "database for AI context" philosophy. Indices are pluggable and can be upgraded/replaced without touching the core engine.

### v0.3.0 - Advanced Retrieval Channels

- 🔲 **Graph-based retrieval**
  - Entity relationship graphs
  - Citation link graphs
  - ADR (Architecture Decision Record) dependency graphs
  - Graph channel currently reserved but not implemented

### v0.4.0 - Ecosystem Integration

- 🔲 **OrionFS filesystem conventions**
  - Auto-detection of `01_context/`, `02_knowledge/`, etc.
  - Front-matter parsing (YAML metadata)
  - Watch mode for auto-indexing
- 🔲 **Policy engine**
  - Content governance rules
  - Access control per span
  - Compliance tracking

### v0.5.0 - Optimization & Scale

- 🔲 **Working set compression**
  - Bullet-point summaries
  - LLM-based summarization (with backrefs)
  - Token budget expansion via compression
- 🔲 **Learned re-ranking**
  - Fine-tuned model for final MMR selection
  - User feedback loop integration
  - A/B testing infrastructure

### Long-term Roadmap

- 🔲 **Embedded mode** (in-process library via PyO3)
- 🔲 **Multi-tenant support** (isolation, quotas)
- 🔲 **Distributed deployment** (sharding, replication)
- 🔲 **Advanced analytics** (query perf, A/B testing)
- 🔲 **Additional client libraries** (Node.js, Go)

---

## 🔍 Architecture Philosophy

### What OrionGraphDB IS

- ✅ A **context compilation engine** for AI agents
- ✅ A **retrieval orchestration** system optimized for LLM prompts
- ✅ A **standalone microservice** with HTTP API
- ✅ A **multi-channel fusion** layer (semantic + lexical + structural + memory + session)
- ✅ An **explainable retrieval** system with full rationale

### What OrionGraphDB IS NOT

- ❌ Not a vector database (orchestrates external embedding services)
- ❌ Not an inverted index (orchestrates external BM25 services)
- ❌ Not a document store (reads from filesystem)
- ❌ Not a general-purpose search engine (specialized for agent context compilation)

### Design Philosophy

OrionGraphDB follows the **"Postgres for AI Context"** philosophy:

1. **Query-like interface** - One main operation: `compile_workingset`
2. **Budget-aware** - Always respects token limits (hard constraints)
3. **Explainable** - Every selection includes rationale
4. **Diverse** - MMR ensures variety, avoids over-reliance on single sources
5. **Composable** - Fuses multiple retrieval channels (semantic, lexical, session, memory)
6. **Fast** - Target <200ms for typical queries
7. **Pluggable** - Indices are external services, easily swapped/upgraded

### Why External Index Services?

The architecture deliberately **delegates** indexing to external HTTP services:

**Benefits:**

- 🔄 Upgrade indices without touching core engine
- 🔌 Swap semantic models (e.g., OpenAI → Cohere → local)
- 🎯 Different services for different projects
- 📊 Scale indices independently of compilation engine
- 🧪 A/B test different retrieval strategies

**Trade-offs:**

- Requires external services to be running
- Network latency (mitigated by async + parallel calls)
- More operational complexity

> **For Production**: Deploy semantic/lexical services alongside OrionGraphDB. For development, use mock generators.

---

## 📊 Feature Maturity Legend

| Symbol | Meaning                           |
| ------ | --------------------------------- |
| ✅     | Implemented and tested            |
| ⚠️     | Partially implemented, needs work |
| 🔲     | Planned but not started           |
| 🚧     | Work in progress                  |
| ❌     | Explicitly out of scope           |

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
