# OrionGraphDB

**A Context Database for AI Agents**

OrionGraphDB is an open-source context compilation engine that provides intelligent, budget-aware retrieval for AI agents. It combines semantic, lexical, and structural search with MMR-based selection to deliver optimal context within token budgets.

---

## 🎯 What is OrionGraphDB?

Think of OrionGraphDB as **"Postgres for AI Context"**:
- Stores and indexes code, documents, and knowledge bases
- Retrieves optimal context for any AI task
- Respects token budgets and diversity constraints
- Explains why each piece of context was selected

### Key Features

- **Multi-Channel Retrieval**: Semantic, lexical (BM25), and structural search
- **MMR Selection**: Maximal Marginal Relevance for diverse, relevant context
- **Token Budget Management**: Never exceed your LLM's context window
- **Citation-Friendly**: Every span has a stable reference and source
- **HTTP API**: Language-agnostic REST interface
- **Fast**: Built in Rust for production performance

---

## 🚀 Quick Start

### Start the Server

```bash
# Build and run
cargo build --release
./target/release/oriongraph-server

# Server runs at http://localhost:8081
```

### Use the Python Client

```python
from oriongraph_client import OrionGraphClient

client = OrionGraphClient("http://localhost:8081")

# Compile optimal context
result = client.compile_workingset(
    intent="Find rollback procedures for database migrations",
    budget_tokens=6000,
    workstream="migration",
)

# Use the context
for span in result["workingset"]["spans"]:
    print(f"📄 {span['span_ref']['doc_version_id']}")
    print(f"   {span['text'][:100]}...")
```

---

## 📚 Core Concepts

### Spans
A **span** is an addressable unit of context:
- Has a unique `span_id`
- Belongs to a `doc_version_id`
- Has a known `token_cost`
- Contains semantic, lexical, and structural metadata

### Working Set
A **working set** is a compiled selection of spans:
- Fits within a token budget
- Maximizes relevance to the intent
- Ensures diversity (via MMR)
- Includes explanations for each selection

### Multi-Channel Retrieval
OrionGraphDB uses three channels:
1. **Semantic**: Vector embeddings for meaning-based search
2. **Lexical**: BM25 for keyword/term matching
3. **Structural**: Project structure, imports, definitions

> **Implementation Status**: See [FEATURES.md](FEATURES.md) for what's currently implemented vs. planned.

---

## 🔧 Architecture

```
OrionGraphDB
├── Context Engine       (Core retrieval logic)
├── Generators           (Semantic, Lexical, Structural)
├── Scoring & Selection  (MMR algorithm)
└── HTTP Server          (REST API)
```

**Technology Stack**:
- **Rust** - Performance and safety
- **Axum** - HTTP framework
- **Tokio** - Async runtime
- **Serde** - Serialization

---

## 📖 API Reference

### `POST /compile_workingset`

Compile an optimal context working set.

**Request**:
```json
{
  "intent": "Find error handling patterns",
  "budget_tokens": 6000,
  "workstream": "backend",
  "explain": true
}
```

**Response**:
```json
{
  "workingset": {
    "spans": [
      {
        "span_ref": {
          "doc_version_id": "src/error.rs",
          "span_id": "fn_handle_error",
          "token_cost": 150
        },
        "text": "pub fn handle_error(err: Error) -> Response { ... }"
      }
    ],
    "total_tokens": 1243
  },
  "stats": {
    "candidates_generated": 47,
    "token_utilization": 0.82
  }
}
```

### `GET /health`

Check server health.

**Response**:
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

---

## 🔌 Integrations

### Python

See [`examples/python-client/`](examples/python-client/) for a full Python client.

```python
from oriongraph_client import OrionGraphClient
client = OrionGraphClient()
result = client.compile_workingset(intent="...", budget_tokens=6000)
```

### LangChain

```python
from oriongraph_client import OrionGraphClient

def get_context(query: str) -> str:
    client = OrionGraphClient()
    result = client.compile_workingset(intent=query, budget_tokens=4000)
    return "\n\n".join(span["text"] for span in result["workingset"]["spans"])
```

### CrewAI, AutoGPT, DeepAgents

OrionGraphDB works with any agent framework via its HTTP API.

---

## 🛠️ Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Run Locally

```bash
cargo run --release
```

### Docker

```bash
docker build -t oriongraphdb .
docker run -p 8081:8081 oriongraphdb
```

---

## 📦 Deployment

### Production Recommendations

- Run as a systemd service or Docker container
- Use a reverse proxy (Nginx, Caddy) for HTTPS
- Scale horizontally with load balancing
- Store indices on persistent volumes

---

## 🌟 Use Cases

### Code Assistants
Retrieve relevant code snippets for any programming task.

### RAG Systems
Compile optimal context for document Q&A.

### Multi-Agent Systems
Shared context database for collaborative agents.

### DevOps Bots
Query runbooks, deployment procedures, and infrastructure docs.

---

## 🤝 Contributing

We welcome contributions! OrionGraphDB is open-source under the Apache 2.0 license.

**How to Contribute**:
1. Fork the repo
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

**Areas We'd Love Help With**:
- Additional generators (e.g., time-based, git-history)
- Performance benchmarks
- Client libraries (Node.js, Go, Rust)
- Documentation and examples

---

## 📄 License

Apache 2.0 - See [LICENSE](LICENSE)

---

## 🔗 Links

- **GitHub**: [github.com/servesys-labs/oriongraphdb](https://github.com/servesys-labs/oriongraphdb)
- **Issues**: [github.com/servesys-labs/oriongraphdb/issues](https://github.com/servesys-labs/oriongraphdb/issues)
- **Orion Framework**: [github.com/servesys-labs/orion](https://github.com/servesys-labs/orion) (proprietary)

---

## 🙏 Acknowledgments

OrionGraphDB is part of the **Orion Stack** ecosystem:
- **OrionGraphDB** (this repo) - Context database
- **Orion Agents** - Agent framework (private)
- **Orion CLI** - Developer tooling (private)

Built with ❤️ by the Orion team.
