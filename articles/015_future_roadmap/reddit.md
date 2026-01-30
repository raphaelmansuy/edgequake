# We're Planning EdgeQuake's 2025 Roadmap. What Features Do You Want?

**TL;DR**: EdgeQuake is a Graph-RAG framework in Rust. We're planning 2025 features and want community input on priorities.

---

## What is EdgeQuake?

A quick summary for those who haven't seen it before:

- **Graph-RAG**: Extract knowledge graphs from documents, query using graph traversal + vectors
- **Rust backend**: 11 crates, async Tokio runtime, PostgreSQL with AGE + pgvector
- **React 19 frontend**: Interactive graph visualization, streaming responses
- **Production-ready**: <200ms latency, 1000+ concurrent users, multi-tenant

It's open source: github.com/raphaelmansuy/edgequake

---

## Current 2025 Roadmap Draft

Here's what we're thinking. I want feedback on priorities.

### Q1 2025: Foundation

| Feature              | Why                                            |
| -------------------- | ---------------------------------------------- |
| **Anthropic Claude** | Most requested after OpenAI, 200K context      |
| **Python SDK**       | 70% of ML community uses Python                |
| **CLI Tool**         | Developer experience (`eq ingest`, `eq query`) |
| **OpenTelemetry**    | Production observability                       |

### Q2 2025: Enterprise

| Feature             | Why                         |
| ------------------- | --------------------------- |
| **SSO (OIDC/SAML)** | Enterprise adoption blocker |
| **RBAC**            | Granular permissions        |
| **DOCX/XLSX**       | Common enterprise formats   |
| **Audit Export**    | Compliance requirement      |

### Q3-Q4 2025: Vision

| Feature                 | Why                    |
| ----------------------- | ---------------------- |
| **AI Agents**           | From search to action  |
| **Multi-hop Reasoning** | Complex query handling |
| **Graph Embeddings**    | Node2Vec, GraphSAGE    |
| **LangChain SDK**       | Ecosystem integration  |

---

## Questions for the Community

### LLM Providers

Currently: OpenAI, Ollama. Planned: Anthropic.

**What else?**

- Google Gemini (native)?
- AWS Bedrock?
- Azure OpenAI?
- Local GGUF via llama.cpp?
- Replicate?
- Together AI?

### Document Formats

Currently: PDF, TXT, MD. Planned: DOCX, XLSX, PPTX.

**What else?**

- Audio transcription (Whisper)?
- Video?
- EPUB?
- Specific domain formats?

### Enterprise Features

Planned: SSO, RBAC, Audit.

**What's missing for your adoption?**

- Compliance certifications?
- Data residency options?
- Specific SSO providers?
- API rate limiting?

### Developer Tools

Planned: Python SDK, CLI, LangChain integration.

**What would help you integrate?**

- TypeScript/Node SDK?
- Go SDK?
- VS Code extension?
- Jupyter integration?

---

## Contributing

If you want to help build these features:

| Level        | Areas                                                 |
| ------------ | ----------------------------------------------------- |
| Beginner     | Documentation, tests, i18n, bug fixes                 |
| Intermediate | LLM provider adapters, document parsers, CLI features |
| Advanced     | Graph embeddings, AI agents, storage backends         |

Good first issues are labeled on GitHub.

---

## The Goal

We want to build what the community actually needs, not what we assume you need.

Your input directly shapes priorities. Upvote features you care about, add suggestions I missed, tell me what's blocking your adoption.

**Repo**: github.com/raphaelmansuy/edgequake
**Discussions**: github.com/raphaelmansuy/edgequake/discussions
