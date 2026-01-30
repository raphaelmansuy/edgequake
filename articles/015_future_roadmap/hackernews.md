# EdgeQuake 2025 Roadmap: Python SDK, SSO, AI Agents

**What it is**: EdgeQuake is a Graph-RAG framework in Rust. It builds knowledge graphs from documents and uses graph traversal + vector search to answer questions. Production-ready today with 11 crates, 6 query modes, and a React 19 frontend.

**Why this post**: I'm sharing the 2025 roadmap and looking for community input on priorities.

---

## Q1 2025: Foundation

### Anthropic Claude Integration

Claude is the most requested LLM provider after OpenAI. Native adapter (not Ollama emulation), support for all Claude 3 models, 200K context window.

### Python SDK

Most of you work in Python. We're building a proper SDK:

```python
from edgequake import EdgeQuake
eq = EdgeQuake(api_url="http://localhost:8080")
eq.ingest("*.pdf")
response = eq.query("What are the main findings?")
```

Sync and async APIs, type hints, minimal dependencies, PyPI package.

### CLI Tool

`eq ingest`, `eq query`, `eq status`, `eq export`. Unix pipe support for scripting.

### OpenTelemetry

Distributed tracing across all crates. Prometheus metrics. Grafana dashboards.

---

## Q2 2025: Enterprise

### SSO (OIDC/SAML)

Okta, Auth0, Azure AD, Google. Group-to-role mapping. Enterprises need this.

### RBAC

Admin/Editor/Viewer roles plus custom definitions. Workspace-level permissions.

### Document Formats

Adding Microsoft Office (DOCX, XLSX, PPTX), HTML, and email (EML, MSG).

### Audit Export

JSON export for SIEM. Splunk/Datadog connectors.

---

## Q3-Q4 2025: Vision

### AI Agents

Multi-turn conversations with memory. Tool use for external APIs. Workflow automation.

### Multi-hop Reasoning

Following relationship chains: "Who manages the person who wrote the API spec?"

### Graph Embeddings

Node2Vec, GraphSAGE for semantic graph search.

### LangChain/LlamaIndex SDKs

Official integrations for the broader ecosystem.

---

## Contributing

| Level        | Areas                                         |
| ------------ | --------------------------------------------- |
| Beginner     | Docs, tests, i18n                             |
| Intermediate | LLM providers, document parsers, CLI          |
| Advanced     | Graph embeddings, AI agents, storage backends |

---

## What Should We Prioritize?

The roadmap is flexible. I'm looking for input on:

1. LLM providers: Which ones matter most to you?
2. Document formats: What are you stuck on today?
3. Enterprise features: What's blocking your adoption?
4. Developer tools: What would help you integrate EdgeQuake?

Reply here or open a discussion on GitHub.

**Repo**: github.com/raphaelmansuy/edgequake
