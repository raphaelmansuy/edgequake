# Frequently Asked Questions

> **Common Questions About EdgeQuake**

---

## General

### What is EdgeQuake?

EdgeQuake is a **production-grade Graph-RAG framework** written in Rust. It combines:

- **Knowledge Graphs** for entity and relationship extraction
- **Vector Search** for semantic retrieval
- **LLM Integration** for natural language answers

Think of it as a smarter search engine that understands concepts, not just keywords.

### How is EdgeQuake different from vector-only RAG?

| Aspect                  | Vector-Only RAG     | EdgeQuake (Graph-RAG)    |
| ----------------------- | ------------------- | ------------------------ |
| Retrieval               | Semantic similarity | Semantic + structural    |
| Multi-hop               | ❌ Single retrieval | ✅ Follows relationships |
| Context                 | Flat chunks         | Connected entities       |
| "What connects X to Y?" | Cannot answer       | Native query type        |

### What's the relationship to LightRAG?

EdgeQuake is a **Rust implementation inspired by [LightRAG](https://github.com/HKUDS/LightRAG)**, a Python Graph-RAG research project.

Key differences:

- **Language**: Rust vs Python (10-50x faster)
- **Production Ready**: Multi-tenant, observability, deployment
- **Storage**: PostgreSQL + pgvector + Apache AGE
- **API**: REST with streaming support

---

## Deployment

### What are the minimum requirements?

**Development**:

- 4 GB RAM
- 2 CPU cores
- Rust 1.78+
- PostgreSQL 16 (optional)

**Production**:

- 8+ GB RAM
- 4+ CPU cores
- PostgreSQL 16 with pgvector + AGE
- LLM provider (OpenAI or Ollama)

### Can I run EdgeQuake without PostgreSQL?

**Yes**, for development and testing. EdgeQuake automatically selects storage:

```bash
# In-memory mode (no database)
cargo run

# PostgreSQL mode
DATABASE_URL="postgresql://..." cargo run
```

In-memory mode persists nothing between restarts.

### Can I run EdgeQuake without an LLM?

**Yes**, for testing with the mock provider:

```bash
# Uses mock provider (no API key needed)
cargo test
```

For production, you need a real LLM. Options:

- OpenAI (`OPENAI_API_KEY`)
- Ollama (local, free)
- LM Studio (local, free)

---

## Cost

### How much does it cost to run EdgeQuake?

EdgeQuake itself is **free and open source**. Costs come from:

| Component  | Cost                                   |
| ---------- | -------------------------------------- |
| EdgeQuake  | Free                                   |
| PostgreSQL | Free (self-hosted) or $15/mo (managed) |
| OpenAI     | ~$0.002 per document (~500 words)      |
| Ollama     | Free (local GPU)                       |

### How can I reduce LLM costs?

1. **Use cheaper models**:

   ```bash
   # gpt-4o-mini is 10x cheaper than gpt-4o
   EDGEQUAKE_LLM_MODEL=gpt-4o-mini
   ```

2. **Use local LLM** (Ollama):

   ```bash
   EDGEQUAKE_LLM_PROVIDER=ollama
   EDGEQUAKE_LLM_MODEL=gemma3:12b
   ```

3. **Reduce chunk size** (fewer LLM calls):
   - Configure smaller chunks in pipeline

### Is there a free tier for OpenAI?

OpenAI offers free credits for new accounts ($5-$18 depending on promotion). After that:

- gpt-4o-mini: ~$0.00015/1K input tokens
- text-embedding-3-small: ~$0.00002/1K tokens

---

## Performance

### How fast is EdgeQuake?

| Operation         | Typical Time              |
| ----------------- | ------------------------- |
| Document upload   | < 1s                      |
| Entity extraction | 2-5s per chunk (with LLM) |
| Vector search     | < 100ms                   |
| Graph traversal   | < 50ms                    |
| Full query        | 2-10s (depends on LLM)    |

### How does it scale?

EdgeQuake handles:

- **Documents**: 100,000+ per workspace
- **Entities**: 1,000,000+ per workspace
- **Concurrent users**: 100+ with connection pooling
- **Query throughput**: 50+ queries/second (without LLM bottleneck)

### How can I speed up queries?

1. **Use `naive` mode** for simple queries (vector-only, no graph)
2. **Reduce `max_chunks`** from 20 to 5-10
3. **Use faster LLM** (gpt-4o-mini vs gpt-4o)
4. **Pre-warm embeddings** with test query
5. **Use GPU** for Ollama embedding

---

## Multi-Tenancy

### Is EdgeQuake multi-tenant?

**Yes**. Each workspace is isolated:

- Separate document collections
- Separate knowledge graphs
- Per-workspace LLM configuration
- No data leakage between workspaces

### Can different tenants use different LLMs?

**Yes**. LLM is configured per-workspace:

```json
POST /api/v1/workspaces
{
  "name": "tenant-a",
  "llm_provider": "openai",
  "llm_model": "gpt-4o"
}

POST /api/v1/workspaces
{
  "name": "tenant-b",
  "llm_provider": "ollama",
  "llm_model": "llama3.2"
}
```

---

## Security

### Is my data encrypted?

| Level      | Status                       |
| ---------- | ---------------------------- |
| At rest    | Depends on PostgreSQL config |
| In transit | Yes (HTTPS recommended)      |
| API keys   | Never logged                 |

### Does EdgeQuake send data to external services?

Only to LLM providers you configure:

- **OpenAI**: Document chunks sent for extraction
- **Ollama**: Local, no external calls
- **No telemetry** sent by EdgeQuake itself

### How do I secure the API?

EdgeQuake doesn't include built-in authentication. Secure with:

1. **Reverse proxy** (nginx/Caddy) with API keys
2. **Network isolation** (private subnet)
3. **OAuth2 proxy** (like oauth2-proxy)

---

## Features

### What document formats are supported?

| Format     | Support                      |
| ---------- | ---------------------------- |
| Plain text | ✅ Full                      |
| Markdown   | ✅ Full                      |
| PDF        | ✅ Full (with edgequake-pdf) |
| HTML       | 🔄 Planned                   |
| DOCX       | 🔄 Planned                   |

### What LLM providers are supported?

| Provider     | Support    | Notes               |
| ------------ | ---------- | ------------------- |
| OpenAI       | ✅ Full    | GPT-4o, GPT-4o-mini |
| Ollama       | ✅ Full    | Any local model     |
| LM Studio    | ✅ Full    | OpenAI-compatible   |
| Azure OpenAI | ✅ Full    | Via base_url config |
| Anthropic    | 🔄 Planned | Claude models       |

### What query modes are available?

| Mode     | Use Case                          |
| -------- | --------------------------------- |
| `naive`  | Simple vector search              |
| `local`  | Entity-focused queries            |
| `global` | High-level summaries              |
| `hybrid` | Best of all modes (DEFAULT)       |
| `mix`    | Custom weighted blend             |
| `bypass` | Direct LLM, no retrieval (debug)  |

---

## Troubleshooting

### Why are my queries returning empty?

1. **Check documents exist**:

   ```bash
   curl http://localhost:8080/api/v1/documents?workspace_id=...
   ```

2. **Check entities extracted**:

   ```bash
   curl http://localhost:8080/api/v1/graph/entities?workspace_id=...
   ```

3. **Try `naive` mode** (vector-only):
   ```json
   { "query": "test", "mode": "naive" }
   ```

See [Troubleshooting Guide](troubleshooting/common-issues.md) for more.

### Why is document processing stuck?

1. Check LLM is running (Ollama: `ollama list`)
2. Check API key is valid (OpenAI)
3. Check logs: `tail -f /tmp/edgequake-backend.log`
4. Restart backend and retry

### How do I check if EdgeQuake is healthy?

```bash
# Basic health
curl http://localhost:8080/health

# Full readiness (checks database)
curl http://localhost:8080/health/ready
```

---

## Comparison

### EdgeQuake vs LightRAG (Python)?

| Aspect       | LightRAG | EdgeQuake     |
| ------------ | -------- | ------------- |
| Language     | Python   | Rust          |
| Speed        | Baseline | 10-50x faster |
| Memory       | Higher   | Lower (no GC) |
| Multi-tenant | No       | Yes           |
| Production   | Research | Production    |
| Algorithm    | Same     | Same          |

### EdgeQuake vs Microsoft GraphRAG?

| Aspect     | GraphRAG                 | EdgeQuake         |
| ---------- | ------------------------ | ----------------- |
| Approach   | Hierarchical communities | Flat entity graph |
| Cost       | Very high ($$$)          | Low-medium        |
| Index time | Hours-days               | Minutes           |
| Queries    | Global summaries         | Hybrid modes      |
| Use case   | Large corpora            | General purpose   |

### EdgeQuake vs Pinecone/Weaviate?

| Aspect     | Vector DBs        | EdgeQuake      |
| ---------- | ----------------- | -------------- |
| Type       | Storage only      | Full RAG stack |
| Retrieval  | Vector similarity | Vector + Graph |
| Extraction | Not included      | Built-in       |
| Multi-hop  | No                | Yes            |

---

## Contributing

### How can I contribute?

1. Fork the repository
2. Create a feature branch
3. Make changes following [AGENTS.md](../AGENTS.md)
4. Run `cargo clippy && cargo test`
5. Submit a pull request

### What's the development workflow?

```bash
# Clone
git clone https://github.com/your-fork/edgequake

# Setup
make dev

# Make changes, then test
cargo test
cargo clippy

# Format before commit
cargo fmt
```

---

## See Also

- [Getting Started](getting-started/installation.md)
- [Architecture Overview](architecture/overview.md)
- [API Reference](api-reference/rest-api.md)
- [Troubleshooting](troubleshooting/common-issues.md)
