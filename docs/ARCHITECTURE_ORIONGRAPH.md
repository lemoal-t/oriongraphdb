# OrionGraph & Orion – Architecture Overview

This document defines the relationship between **OrionGraph** and **Orion**, and how they work together as a full stack for agentic systems.

---

## High-Level Mental Model

We treat:

- **OrionGraph** as a **database-like engine** for context:
  - It indexes content (OrionFS, memories, etc.)
  - It serves query-like requests (`compile_workingset`)
  - It returns optimized "working sets" of spans for agents

- **Orion** as an **agentic framework** (like LangChain/DeepAgents + CLI):
  - It manages sessions, tools, agents, workflows
  - It handles REPL / CLI UX
  - It orchestrates calls to OrionGraph and LLMs

Analogy:

- `OrionGraph : Orion :: Postgres : Django`

---

## Components

### 1. OrionGraph (Context Database)

**Role:** Provide high-quality, budget-aware context for agents.

**Responsibilities:**

- Maintain indices over content:
  - Semantic (FAISS/HNSW or equivalent)
  - Lexical (BM25 / inverted index)
  - Structural (headings/sections, spans)
- Expose a single main API:

  ```http
  POST /compile_workingset
  {
    "intent": "draft cutover plan",
    "budget_tokens": 8000,
    ...
  }

  -> {
    "workingset": {
      "spans": [ ... ],
      "total_tokens": 7943
    },
    "stats": { ... },
    "rationale": [ ... ]
  }
  ```

- Implement retrieval-as-compilation:
  - Multi-channel candidate generation
  - Scoring, diversity (MMR)
  - Token budget + source constraints
- Hydrate spans back to real text using OrionFS docs.

**Persistence:**

- Uses files as its physical storage:
  - `indices/semantic.faiss`
  - `indices/lexical.json`
  - `indices/structural.json`
  - Mappings to OrionFS docs
- OrionGraph itself is logically a DB, but it does not own a single `.db` file.

---

### 2. Orion (Agentic Framework)

**Role:** Orchestrate agents, sessions, tools, and workflows using OrionGraph as the context DB.

**Responsibilities:**

- **CLI / UX**
  - `orion chat`
  - `orion agent run ...`
  - `orion workplan run ...`
- **Agents & Tools**
  - Planner, coder, researcher, architect, etc.
  - Integration with LLMs (OpenAI, etc.)
  - Tools (compile_context, load_memory, etc.)
- **Sessions**
  - Track user <-> agent conversations
  - Log events, state, and tool calls
- **Memory**
  - Extract long-term facts from sessions
  - Write memory artifacts into knowledge space
- **Filesystem & Policy**
  - Work over OrionFS-structured repos
  - Enforce path + content policies via OrionFSGuard

**Persistence:**

- Uses a session DB (SQLite in `~/.orion/sessions.db`):
  - `sessions`
  - `session_events`
  - `session_state`
- Uses the filesystem for:
  - Working repos (OrionFS)
  - Memory artifacts (Markdown)
  - Configuration and indices

---

## Storage Model

We have three logical "planes" of state:

### 1. Context Plane (OrionGraph)

- **What:** Knowledge & content used to build prompts.
- **Where:**
  - OrionFS markdown files (checked into git)
  - Index files under `indices/`
- **Owner:** OrionGraph (for indexing + retrieval).

### 2. Session Plane (Session DB)

- **What:** Per-session event logs + working state.
- **Where:**
  - `~/.orion/sessions.db` (SQLite by default)
- **Owner:** Orion (REPL & CLI write here; memory ETL reads from here).

### 3. Memory Plane (Memory Artifacts)

- **What:** Long-term facts, preferences, decisions.
- **Where (v1):**
  - Markdown files, e.g.:
    - `02_knowledge/memory/user-<id>.md`
    - `02_knowledge/memory/workstream-<id>.md`
  - These get indexed by OrionGraph like any other doc.
- **Owner:** Orion (ETL writes them; OrionGraph indexes & serves them).

Later, memories can also be reflected into a `memories` table in SQLite/Postgres.

---

## Local Development Story

For local dev, we aim for a 1-command setup and minimal mental overhead.

**Directory layout:**

```
~/.orion/
├── sessions.db         # SQLite: sessions/events/state
├── indices/            # OrionGraph indices
│   ├── semantic.faiss
│   ├── lexical.json
│   └── structural.json
├── memory/             # Optional memory docs (if not in repo)
└── config.toml         # Orion + OrionGraph config
```

**Workflow:**

```bash
pip install orion  # (or equivalent)

orion init
# - Create ~/.orion/
# - Initialize sessions.db
# - Create indices/ directory
# - Write default config
# - (Optional) build initial indices

orion chat
# - Ensure oriongraph-server is running (auto-start if needed)
# - Use sessions.db for events/state
# - Use indices/ + OrionFS for context
# - Use memory docs as part of context
```

**OrionGraph in local dev:**

- Runs as a local HTTP service, started by the Orion CLI if not already running.
- Reads indices from `~/.orion/indices/` (or a configured path).
- Orion uses a thin client (`OrionGraphClient`) to talk to it.

---

## Production Story (Future)

In a more "deployed" setting, OrionGraph and Orion can be split:

```yaml
# docker-compose.yml (example)
services:
  oriongraph:
    image: oriongraph/oriongraph:latest
    volumes:
      - ./indices:/data/indices
    ports:
      - "8080:8080"

  orion-api:
    image: orion/orion:latest
    environment:
      ORIONGRAPH_URL: http://oriongraph:8080
    volumes:
      - ./data:/data           # sessions, memory docs, configs
```

- OrionGraph becomes a database-like service other apps can share.
- Orion is one of potentially many clients.

---

## Responsibilities & Boundaries

**OrionGraph:**

- **Owns:**
  - Indexing
  - Retrieval
  - Span-level context compilation
- **Does not own:**
  - Per-user sessions
  - Long-term memory lifecycle
  - Agent orchestration

**Orion:**

- **Owns:**
  - Sessions and events
  - Memory extraction and storage
  - Agent lifecycle and tools
  - Policy enforcement on writes
- **Delegates:**
  - Context construction to OrionGraph (`compile_workingset`)

---

## Future Extensions

- **Embedded mode:** Provide a Rust library / PyO3 bindings to use OrionGraph in-process (no HTTP) for specialized deployments.
- **Memory DB:** Optional dedicated memory tables + API (`/memory/query`) in addition to Markdown docs.
- **Multi-tenant OrionGraph:** Isolation via tenant IDs + ACLs so multiple apps/users can share a single OrionGraph instance.
- **Advanced policy engine:** YAML → WASM policy compilation for more complex governance.

---

**This architecture provides a clean separation of concerns while maintaining flexibility for both local development and production deployment.**

