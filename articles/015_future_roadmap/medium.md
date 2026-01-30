# The Future of Graph-RAG: EdgeQuake's 2025 Roadmap

_Where we're headed and how to get involved_

---

## Why Roadmaps Matter

Open source projects die in silence. Users hesitate to adopt something that might disappear. Contributors don't know where to help. Decision-makers can't plan for integration.

Roadmaps solve this. They show the project is alive, has direction, and welcomes participation.

EdgeQuake is building the future of Graph-RAG. Here's where we're headed.

---

## Current State: Production-Ready

Before we talk about the future, here's where EdgeQuake stands today:

**Backend** (11 Rust Crates):

- Full document ingestion pipeline
- Entity extraction and relationship mapping
- 6 query modes (naive, local, global, hybrid, mix, bypass)
- PDF processing (text, vision, hybrid)
- PostgreSQL storage with AGE + pgvector
- OpenAI and Ollama LLM providers
- REST API with OpenAPI 3.0 and streaming

**Frontend** (React 19 + Next.js 16):

- Interactive knowledge graph visualization
- Real-time streaming responses
- Document management with progress tracking
- Query interface with mode selection
- 100+ components built on shadcn/ui

**Performance**:

- Query latency <200ms (hybrid mode)
- 1000+ concurrent users
- 2MB memory per document
- 2-3x more entities extracted than baseline RAG

This is production-ready. Companies are using it. But we're just getting started.

---

## Q1 2025: Foundation

The first quarter focuses on expanding the foundation.

### Anthropic Claude Integration

**Why**: Claude is the second most requested LLM provider after OpenAI. Its long context window (200K tokens) is perfect for document processing.

**What we're building**:

- Native Claude adapter (not via Ollama emulation)
- Support for Claude 3 Opus, Sonnet, and Haiku
- Prompt optimization for Claude's strengths
- Cost tracking integration

### Python SDK

**Why**: 70% of the ML community works in Python. A Rust backend is great, but Pythonistas need a native interface.

**What we're building**:

- Sync and async APIs
- Pandas/Polars integration for bulk operations
- Type hints and documentation
- PyPI package with minimal dependencies

```python
from edgequake import EdgeQuake

eq = EdgeQuake(api_url="http://localhost:8080")

# Ingest documents
eq.ingest("research_papers/*.pdf")

# Query with hybrid mode
response = eq.query(
    "What are the main findings?",
    mode="hybrid"
)

print(response.answer)
print(response.sources)
```

### CLI Tool

**Why**: Developer experience matters. A good CLI accelerates adoption.

**What we're building**:

- `eq ingest <files>` - Batch document ingestion
- `eq query <question>` - Terminal-based querying
- `eq status` - Pipeline and health status
- `eq export` - Export graph data
- Piping support for Unix workflows

```bash
# Ingest and query in one pipeline
eq ingest *.pdf | eq query "Summarize the key points"
```

### OpenTelemetry Observability

**Why**: Production systems need observability. Without it, debugging is guesswork.

**What we're building**:

- Distributed tracing across all crates
- Span context propagation from API to LLM calls
- Prometheus metrics export
- Grafana dashboard templates
- Cost tracking integration

---

## Q2 2025: Enterprise

The second quarter focuses on enterprise adoption blockers.

### SSO Integration (OIDC/SAML)

**Why**: Enterprises don't adopt tools that require separate credentials. SSO is table stakes.

**What we're building**:

- OIDC integration (Okta, Auth0, Azure AD, Google)
- SAML 2.0 support
- Group-to-role mapping
- Session management
- Audit logging of auth events

### Role-Based Access Control (RBAC)

**Why**: Not everyone should access everything. Enterprises need granular permissions.

**What we're building**:

- Predefined roles (Admin, Editor, Viewer)
- Custom role definitions
- Permission inheritance
- API key scoping
- Workspace-level permissions

```
Roles:
├── Admin: Full access to workspace
├── Editor: Upload docs, run queries, no settings
├── Viewer: Query only, no modifications
└── Custom: Define your own
```

### Document Format Expansion

**Why**: PDF is common, but enterprises have DOCX, XLSX, PPTX everywhere.

**What we're building**:

- Microsoft Office (DOCX, XLSX, PPTX)
- HTML/Web pages
- Email (EML, MSG)
- Markdown (enhanced)
- Plain text with encoding detection

### Audit Export

**Why**: Compliance teams need audit trails in their SIEM.

**What we're building**:

- JSON export of all audit events
- Syslog integration
- Splunk/Datadog connectors
- Retention policy configuration
- PII redaction options

---

## Q3-Q4 2025: Vision

The second half of 2025 is about pushing the boundaries.

### AI Agents with Tool Use

**Why**: RAG is just retrieval. The future is agents that take action.

**What we're building**:

- Multi-turn conversations with memory
- Tool definitions and execution
- External API integration
- Workflow automation
- MCP (Model Context Protocol) compatibility

```
User: "Find all contracts expiring this quarter and draft renewal emails"

Agent:
1. Queries knowledge graph for contracts
2. Filters by expiration date
3. Drafts personalized emails
4. Returns drafts for approval
```

### Multi-hop Reasoning

**Why**: Complex questions require following chains of relationships.

**What we're building**:

- Iterative graph traversal
- Intermediate reasoning display
- Confidence scoring per hop
- Explanation of reasoning path
- Fallback to direct retrieval

```
Query: "Who manages the person who wrote the API spec?"

Reasoning:
1. "API spec" → authored by JOHN_DOE
2. JOHN_DOE → reports to SARAH_CHEN
3. Answer: SARAH_CHEN manages the person who wrote the API spec
```

### Graph Embeddings

**Why**: Node2Vec and GraphSAGE enable semantic graph search.

**What we're building**:

- Node2Vec embedding generation
- GraphSAGE for inductive learning
- Combined vector+graph retrieval
- Clustering by embedding similarity
- Visualization by embedding space

### LangChain/LlamaIndex SDKs

**Why**: The ecosystem matters. Integration with popular frameworks accelerates adoption.

**What we're building**:

- LangChain Retriever adapter
- LlamaIndex QueryEngine adapter
- Official documentation and examples
- Version compatibility testing

---

## Roadmap Timeline

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        EdgeQuake 2025 Roadmap                            │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Q1 2025                 Q2 2025                Q3-Q4 2025               │
│  ┌────────────────┐     ┌────────────────┐     ┌────────────────────┐   │
│  │ • Anthropic    │     │ • SSO (OIDC)   │     │ • AI Agents        │   │
│  │ • Python SDK   │     │ • RBAC         │     │ • Multi-hop        │   │
│  │ • CLI Tool     │     │ • DOCX/XLSX    │     │ • Graph Embeddings │   │
│  │ • OpenTelemetry│     │ • Audit Export │     │ • LangChain SDK    │   │
│  └────────────────┘     └────────────────┘     └────────────────────┘   │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Contributing

EdgeQuake is open source. Here's where you can help:

### Beginner-Friendly

- Documentation improvements
- Test coverage expansion
- Internationalization (i18n)
- Bug reports and fixes

### Intermediate

- New LLM provider adapters
- Document format parsers
- CLI tool features
- Python SDK contributions

### Advanced

- Graph embedding algorithms
- AI agent capabilities
- Storage backend adapters
- Performance optimization

**Start here**: github.com/raphaelmansuy/edgequake/issues

---

## Community Input

Roadmaps are living documents. Priorities shift based on community needs.

**What should we build first?**

- More LLM providers? Which ones?
- More document formats? Which ones?
- Enterprise features? Which matter most?
- Developer tools? What's missing?

**Tell us**: GitHub Discussions, Twitter/X (@raphaelmansuy), or LinkedIn.

---

## The Vision

Graph-RAG is the future of document intelligence. Not just retrieving chunks—understanding relationships, following chains of reasoning, taking action.

EdgeQuake is building that future:

- **2024**: Production-ready Graph-RAG
- **2025**: Enterprise features, SDKs, AI agents
- **2026**: The standard platform for document intelligence

We're not just building a tool. We're building a platform for the next generation of knowledge work.

---

**Join us**: github.com/raphaelmansuy/edgequake

_The roadmap is the promise. The code is the delivery._
