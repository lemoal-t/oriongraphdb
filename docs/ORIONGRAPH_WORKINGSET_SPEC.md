# OrionGraph Working Set Compiler Specification

**Component:** `compile_workingset()`  
**Context:** OrionFS + OrionGraph, agent-first context infrastructure  
**Status:** Draft v1 (MVP-ready)

---

## Quick Reference

**What is this?** OrionGraph's Working Set Compiler transforms an agent's intent into an optimized context bundle. Instead of naive "top-k vector search," it runs retrieval-as-compilation: multi-channel candidate generation → scoring → MMR-based diversity selection → budget-constrained optimization. The result is a provenance-backed, explainable set of document spans that fit within token limits while maximizing relevance and source diversity.

---

## 1. Purpose

`compile_workingset()` is the **context compiler** for agents.

Given:

- an **intent** (natural language),
- a **token budget**, and
- a set of **hard constraints** and **soft preferences**,

it returns a **Working Set** — an ordered list of spans from documents that:

- fit within the token budget,
- are maximally useful for the task,
- are diverse across sources and perspectives, and
- come with **explanations** and **provenance**.

This replaces naive "top-k vector search" with **retrieval-as-compilation**.

---

## 2. High-Level Architecture

OrionGraph sits on top of **OrionFS**:

- **OrionFS** = filesystem-first project layout

  - Files under `01_context/`, `02_knowledge/`, `03_workstreams/`, `04_artifacts/`
  - Deterministic paths + YAML front matter

- **OrionGraph** = index + brain, over OrionFS:
  - Span registry
  - Structural index
  - Lexical index
  - Semantic index (ANN)
  - Graph index (entities, citations, ADR links)
  - Episodic logs

`compile_workingset()` lives inside `ContextEngine` and uses these indices to assemble an optimal Working Set.

---

## 3. Core Data Model

### 3.1 SpanRef

Addressable unit of reading.

```rust
pub struct SpanRef {
    pub doc_version_id: String,  // SHA256 of document bytes
    pub span_id: String,         // stable within version (UUID or derived)
    pub char_start: usize,
    pub char_end: usize,
    pub token_cost: usize,       // approximate tokens for default tokenizer
}
```

- `doc_version_id` ensures no drift: spans are immutable per version.
- `span_id` + offsets allow exact quoting & provenance.

### 3.2 CandidateSpan

Span with scores from different signals.

```rust
pub struct CandidateSpan {
    pub span_ref: SpanRef,
    pub scores: ScoreChannels,
    pub embedding: Option<Vec<f32>>,  // normalized, used for MMR
    pub text_preview: String,         // small preview for logging/debug
    pub metadata: SpanMetadata,
    pub base_score: f32,              // computed during compilation
    pub mmr_score: f32,               // computed during selection
}

pub struct ScoreChannels {
    pub semantic: f32,    // [0,1]
    pub lexical: f32,     // [0,1]
    pub structural: f32,  // [0,1]
    pub graph: f32,       // [0,1]
}

pub struct SpanMetadata {
    pub filepath: String,
    pub workstream: Option<String>,
    pub stage: Option<String>,        // requirements|design|research|impl|eval|final
    pub section_title: Option<String>,
    pub created_at: i64,              // unix timestamp
    pub recency_score: f32,           // [0,1]
    pub source_type: SourceType,      // Context|Knowledge|Workstream|Artifact
}
```

### 3.3 WorkingSet & Items

```rust
pub struct WorkingSet {
    pub spans: Vec<WSItem>,
    pub total_tokens: usize,
}

pub struct WSItem {
    pub span_ref: SpanRef,
    pub text: String,                  // hydrated after selection
    pub compression: Option<Compression>,
    pub source_weight: f32,            // contribution weight (debugging)
    pub selection_rank: usize,         // position in the ordered WS
}

pub enum Compression {
    Bullets { lines: Vec<String>, backref: SpanRef },
    Summary { text: String, backref: SpanRef },
}
```

### 3.4 Request/Response

```rust
pub struct CompileRequest {
    pub intent: String,
    pub task_id: Option<String>,
    pub query_signals: Vec<QuerySignal>,
    pub budget_tokens: usize,
    pub hard_filters: HardFilters,
    pub soft_prefs: SoftPreferences,
    pub explain: bool,
}

pub enum QuerySignal {
    NaturalLanguage(String),
    Keywords(Vec<String>),
    StructuralHints(StructHints),
    EpisodeContext(String), // episode_id
}

pub struct CompileResponse {
    pub workingset: WorkingSet,
    pub stats: CompileStats,
    pub rationale: Option<Vec<SpanExplanation>>,
}

pub struct SpanExplanation {
    pub span_ref: SpanRef,
    pub final_score: f32,
    pub base_score: f32,
    pub diversity_penalty: f32,
    pub reasons: Vec<String>, // human-readable tags
}
```

---

## 4. Generator Interface

Candidate generation is pluggable via a trait.

```rust
#[async_trait::async_trait]
pub trait CandidateGenerator: Send + Sync {
    fn name(&self) -> &'static str;

    async fn generate(
        &self,
        signals: &DerivedSignals,
        filters: &HardFilters,
        top_k: usize,
    ) -> anyhow::Result<Vec<CandidateSpan>>;
}
```

### 4.1 DerivedSignals

Pre-processed view of the request.

```rust
pub struct DerivedSignals {
    pub intent: String,               // normalized
    pub intent_embedding: Vec<f32>,   // sem_text space
    pub keywords: Vec<String>,
    pub struct_hints: StructHints,
    pub episode_context: Option<String>,
}
```

### 4.2 Example Generators (v1)

- **StructuralGen**: Uses structural index (headings, sections, ADRs) + StructHints.
- **LexicalGen**: Uses inverted index or FM-index with keywords & BM25.
- **SemanticGen**: Uses ANN (HNSW/FAISS) in sem_text vector space.
- **GraphGen** (optional for v1): Uses entity/citation graph to pull 1-hop neighbors of key nodes.

Generators run in parallel; failures are logged but do not abort the whole compile (unless all fail).

---

## 5. Scoring & Normalization

Each generator produces raw scores per channel. These are normalized per channel across all candidates:

- Use **min–max normalization**:

```
norm(x) = (x - min) / (max - min + ε)
```

If max ≈ min, the channel is treated as inactive (score = 1.0 if x > 0, else 0.0).

### 5.1 Base Score

The base utility score for a candidate:

```
base_score(span) =
  w_sem * semantic
+ w_lex * lexical
+ w_struct * structural
+ w_graph * graph
+ w_recency * recency_score
+ w_stage * stage_boost
```

Where `stage_boost` is derived from `soft_prefs.prefer_stages`.

**Default weights:**

```rust
pub struct ScoreWeights {
    pub semantic: f32,    // default 0.4
    pub lexical: f32,     // 0.2
    pub structural: f32,  // 0.2
    pub graph: f32,       // 0.1
    pub recency: f32,     // 0.05
    pub stage_boost: f32, // 0.05
}
```

---

## 6. Selection Algorithm: MMR + Token-Budget Knapsack

### 6.1 Goals

- **Maximize:**
  - Relevance to intent
  - Diversity across content
  - Source mix (avoid depending on 1 doc)
- **Subject to:**
  - `total_tokens <= budget_tokens`
  - `tokens_from_single_source <= max_single_source_ratio * total_tokens`

### 6.2 MMR (Maximal Marginal Relevance)

At each step:

```
MMR(span) = λ * base_score(span)
          - (1 - λ) * max_sim(span, selected)
```

- `λ` is `diversity_lambda` in SoftPreferences
- `max_sim` uses cosine similarity between embeddings

### 6.3 Algorithm Sketch

1. Sort candidates by `base_score` desc; truncate to O(5× budget) for pruning.
2. Maintain:
   - `selected`: Vec<WSItem>
   - `selected_embeddings`: Vec<Vec<f32>>
   - `used_tokens`: usize
   - `source_tokens`: HashMap<filepath, usize>
3. Loop:
   - Compute MMR for each remaining candidate.
   - Pick highest-scoring candidate that:
     - Fits remaining token budget.
     - Does not violate source_ratio (applied only when multiple sources exist).
   - Add to selected; update counters.
   - Stop when:
     - No candidate fits budget, or
     - MMR scores drop below a threshold (e.g., < 0.1), or
     - Budget nearly saturated.

**Tie-breaker:** when two candidates have similar MMR (e.g., within 0.01), prefer the one with lower token_cost.

---

## 7. Constraints & Edge Cases

- **Single source only:** source_ratio constraint is relaxed when there's only one source in play.
- **Very small budgets:** still return at least one high-utility span if possible.
- **No candidates:**
  - Relax filters (e.g., widen paths, drop recency).
  - If still none, return an error with suggestions (e.g., "add docs to 01_context/").
- **Identical scores edge case:**
  - Normalization can flatten scores; we still select by:
    - recency
    - token efficiency
    - tie-breaker ordering

---

## 8. Thread Safety & Concurrency

- `ContextEngine` is designed to be wrapped in `Arc<ContextEngine>` and used concurrently.
- All indices are read-only or internally synchronized.
- `CandidateGenerator` is Send + Sync; implementations must either be stateless or manage their own internal locking.
- Each call to `compile_workingset()` uses only local, per-request allocations (no shared mutable state).

---

## 9. Compression (Out of Scope for v1)

- `compile_workingset()` returns raw spans + text.
- Separate function `compress_workingset()` can:
  - Summarize spans
  - Create bullet lists
  - Always include backrefs to original SpanRef.
- Policies may control which compression strategies are allowed for which content types.

---

## 10. Testing Strategy

### Unit Tests

- **MMR Diversity**: Ensure spans from different "topics" are both represented when diversity is high.
- **Source Ratio**: Ensure no single source exceeds `max_single_source_ratio` when multiple sources exist.
- **Token Budget**: Ensure the total `token_cost` ≤ budget and utilization is ≥85–90% where possible.
- **End-to-End**: With mock generators, confirm:
  - candidates → fused → selected → Working Set
  - explanations are present when `explain = true`.

---

## 11. Performance Targets

On a modest machine:

- **Candidate generation** (with indices): < 100ms
- **MMR + knapsack selection** (500 candidates): < 50ms
- **End-to-end compile** (cold cache): < 200ms

**Token utilization:** ≥90% for typical budgets.

---

## 12. Future Extensions

- **Learned re-ranker** on top of MMR.
- **Policy integration:**
  - enforce min citations per selected span
  - enforce inclusion of ADRs for "final" tasks
- **Graph-augmented retrieval:**
  - incorporate ADR decision graph more heavily
- **Per-agent personalization:**
  - episodes as a signal in scoring

---

## 13. References

- Implementation: `/Users/agentsy/orion/oriongraph/`
- ADR: `03_workstreams/ws-orion/99_decisions/ADR-20251113-retrieval-as-compilation.md`
- Demo: `DEMO.md`
