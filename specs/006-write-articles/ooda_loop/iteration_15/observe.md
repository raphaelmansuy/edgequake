# Observe Phase - Iteration 15: Future Roadmap

## Current State Summary

EdgeQuake is a production-ready Graph-RAG framework with:

### Backend (11 Rust Crates)

| Crate                  | Purpose                             |
| ---------------------- | ----------------------------------- |
| edgequake-core         | Orchestration & pipeline            |
| edgequake-llm          | OpenAI, Ollama, Mock providers      |
| edgequake-storage      | PostgreSQL AGE, Memory adapters     |
| edgequake-api          | REST API server (Axum)              |
| edgequake-pipeline     | Document ingestion                  |
| edgequake-query        | Query engine (6 modes)              |
| edgequake-pdf          | PDF extraction (text/vision/hybrid) |
| edgequake-auth         | Authentication & authorization      |
| edgequake-audit        | Compliance & audit logging          |
| edgequake-tasks        | Background job processing           |
| edgequake-rate-limiter | Rate limiting middleware            |

### Frontend

- React 19.2.3 with Next.js 16.1.0
- 100+ components
- Sigma.js graph visualization
- Real-time streaming responses

### Performance Benchmarks

| Metric                 | Value                    |
| ---------------------- | ------------------------ |
| Entity Extraction      | ~2-3x more than baseline |
| Query Latency (hybrid) | <200ms                   |
| Document Processing    | 25s (10k tokens)         |
| Concurrent Users       | 1000+                    |
| Memory Usage           | 2MB per doc              |

### Current Features

1. **5 Query Modes**: Naive, Local, Global, Hybrid, Mix
2. **PDF Processing**: Text, Vision, Hybrid modes
3. **Knowledge Graphs**: Entity extraction, relationship mapping
4. **REST API**: OpenAPI 3.0, SSE streaming
5. **Multi-Tenant**: Workspace isolation
6. **WebUI**: Full-featured React interface

## Potential Roadmap Areas

### 1. LLM Provider Expansion

**Current**: OpenAI, Ollama, Mock
**Potential additions**:

- Anthropic Claude
- Google Gemini (native, not via Ollama)
- AWS Bedrock
- Azure OpenAI
- Local GGUF via llama.cpp
- Replicate
- Together AI

### 2. Storage Backend Expansion

**Current**: PostgreSQL (AGE + pgvector), Memory
**Potential additions**:

- Neo4j native adapter
- SurrealDB (graph + document + vector)
- DGraph
- Redis (caching layer)
- S3/GCS for blob storage

### 3. Document Format Expansion

**Current**: PDF, TXT, MD
**Potential additions**:

- Microsoft Office (DOCX, XLSX, PPTX)
- HTML/Web pages
- Audio transcription (Whisper)
- Video (frame extraction + transcription)
- Images with OCR
- Email (EML, MSG)
- EPUB for books

### 4. Query Capabilities

**Current**: 6 modes + streaming
**Potential additions**:

- Multi-hop reasoning
- Temporal queries ("What was said last month?")
- Comparative queries ("How does X differ from Y?")
- Aggregation queries ("Top 10 entities by mentions")
- Natural language to Cypher translation
- Query explanation ("Why did you retrieve this?")

### 5. Graph Enhancements

**Current**: Entity extraction, relationships, communities
**Potential additions**:

- Ontology support (define entity types)
- Custom relationship types
- Graph embeddings (Node2Vec, GraphSAGE)
- Temporal graphs (time-evolving relationships)
- Hierarchical community detection
- Graph comparison between workspaces

### 6. Enterprise Features

**Current**: Multi-tenant, basic auth
**Potential additions**:

- SSO (SAML, OIDC)
- RBAC (Role-Based Access Control)
- Audit logging export
- Data retention policies
- Compliance certifications (SOC2, HIPAA)
- Enterprise support SLA

### 7. Developer Experience

**Current**: REST API, Rust SDK
**Potential additions**:

- Python SDK
- TypeScript/Node.js SDK
- Go SDK
- CLI tool
- VS Code extension
- Jupyter integration
- LangChain integration (official)
- LlamaIndex integration

### 8. Observability

**Current**: Basic logging
**Potential additions**:

- OpenTelemetry tracing
- Prometheus metrics
- Grafana dashboards
- Error tracking (Sentry integration)
- Cost monitoring dashboard
- Performance profiling

### 9. WebUI Enhancements

**Current**: Document upload, graph viewer, query interface
**Potential additions**:

- Collaborative workspaces (multiple users)
- Annotations on documents
- Graph editing (manual entity/relationship creation)
- Timeline view for temporal data
- Export to PDF/Word
- Mobile app (React Native)

### 10. AI Agent Capabilities

**Current**: Simple Q&A
**Potential additions**:

- Multi-turn reasoning
- Tool use (external APIs)
- Memory across conversations
- Personalization per user
- Autonomous research tasks
- Integration with MCP (Model Context Protocol)

## Community Contributions Welcome

From CONTRIBUTING.md, areas open for contribution:

1. Bug reports via GitHub Issues
2. Feature requests via GitHub Discussions
3. Documentation improvements
4. Test coverage expansion
5. Performance optimization PRs
6. Internationalization (i18n)

## Research Influences

The roadmap builds on:

- **LightRAG** (arXiv:2410.05779): Core algorithm
- **GraphRAG** (Microsoft): Community detection, hierarchical summarization
- **Anthropic's Claude**: Long context handling patterns
- **RAG research**: Hybrid retrieval, re-ranking, query expansion
