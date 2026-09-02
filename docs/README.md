---
title: 'EdgeQuake Documentation'
---

# EdgeQuake Documentation

> **Product: v0.26.5** · Contract: [`openapi.snapshot.json`](../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md)

High-performance Graph-Enhanced RAG in Rust. PostgreSQL (pgvector + Apache AGE) is required for all server modes. Auth is enabled by default unless `EDGEQUAKE_DEV_MODE=true` or `AUTH_ENABLED=false`.

```
┌──────────────────────────────────────────────────────────────────┐
│ EdgeQuake                                                        │
│                                                                  │
│  Document --> [Pipeline] --> Knowledge Graph --> Query           │
│                   |                |                |            │
│                   v                v                v            │
│               Chunks+Embed     Entities+Rels    Hybrid           │
│                                                                  │
│  REST API :8080   WebUI :3000   SDKs   PostgreSQL+AGE            │
└──────────────────────────────────────────────────────────────────┘
```

---

## Documentation Index

### Getting Started

| Guide                                              | Description                | Time   |
| ----------------------------------------------------| ----------------------------| --------|
| [Installation](getting-started/installation.md)    | Prerequisites and setup    | 5 min  |
| [Quick Start](getting-started/quick-start.md)      | First ingestion and query  | 10 min |
| [First Ingestion](tutorials/document-ingestion.md) | Understanding the pipeline | 15 min |

### Architecture

| Document                                   | Description                           |
| ------------------------------------------ | ------------------------------------- |
| [Overview](architecture/overview.md)       | System design and components          |
| [Data Flow](architecture/data-flow.md)     | Upload → convert → ingest → query     |
| [Crate Reference](architecture/crates/)    | 11 Rust crates (incl. tasks, auth)    |

### Core Concepts

| Concept                                          | Description                       |
| ------------------------------------------------ | --------------------------------- |
| [Graph-RAG](concepts/graph-rag.md)               | Why knowledge graphs enhance RAG  |
| [Entity Extraction](concepts/entity-extraction.md) | LLM-based entity recognition    |
| [Knowledge Graph](concepts/knowledge-graph.md)   | Nodes, edges, and communities     |
| [Hybrid Retrieval](concepts/hybrid-retrieval.md) | Combining vector and graph search |

### Deep Dives

| Article                                                  | Description                                  |
| -------------------------------------------------------- | -------------------------------------------- |
| [Data Layer](deep-dives/data-layer.md)                   | Postgres ER, KV, AGE, pgvector, FTS          |
| [LightRAG Algorithm](deep-dives/lightrag-algorithm.md)   | Core algorithm: extraction, graph, retrieval |
| [Query Modes](deep-dives/query-modes.md)                 | 6 modes with trade-offs                      |
| [Pipeline Progress](deep-dives/pipeline-progress.md)     | WebSocket / SSE progress (SPEC-048/057)      |
| [PDF Processing](deep-dives/pdf-processing.md)           | Vision and EdgeParse extraction              |
| [Entity Normalization](deep-dives/entity-normalization.md) | Deduplication and merging                  |
| [Gleaning](deep-dives/gleaning.md)                       | Multi-pass extraction                        |
| [Entity Extraction](deep-dives/entity-extraction.md)     | LLM extraction pipeline                      |
| [Community Detection](deep-dives/community-detection.md) | Louvain clustering                           |
| [Chunking Strategies](deep-dives/chunking-strategies.md) | Token-based segmentation                     |
| [Embedding Models](deep-dives/embedding-models.md)       | Model selection and dimensions               |
| [Graph Storage](deep-dives/graph-storage.md)             | Apache AGE property graph                    |
| [Vector Storage](deep-dives/vector-storage.md)           | pgvector HNSW / halfvec                      |
| [Cost Tracking](deep-dives/cost-tracking.md)             | LLM cost monitoring                          |

### Comparisons

| Comparison                                                  | Key Insights                       |
| ----------------------------------------------------------- | ---------------------------------- |
| [vs LightRAG (Python)](comparisons/vs-lightrag-python.md)   | Performance and design differences |
| [vs GraphRAG](comparisons/vs-graphrag.md)                   | Microsoft's approach               |
| [vs Traditional RAG](comparisons/vs-traditional-rag.md)     | Why graphs matter                  |

### Tutorials

| Tutorial                                                      | Description                     |
| ------------------------------------------------------------- | ------------------------------- |
| [Building Your First RAG App](tutorials/first-rag-app.md)     | End-to-end tutorial             |
| [PDF Ingestion](tutorials/pdf-ingestion.md)                   | PDF upload and configuration    |
| [Multi-Tenant Setup](tutorials/multi-tenant.md)               | Workspace isolation             |
| [Document Ingestion](tutorials/document-ingestion.md)         | Upload and processing workflows |
| [Migration from LightRAG](tutorials/migration-from-lightrag.md) | Python to Rust migration      |
| [Knowledge Injection](tutorials/knowledge-injection.md)       | Manual entity/relationship CRUD |
| [Query Optimization](tutorials/query-optimization.md)         | Mode and filter tuning          |
| [Tracing Entity Sources](tutorials/tracing-entity-sources.md) | Lineage and provenance          |

### Integrations

| Integration                                    | Description                          |
| ---------------------------------------------- | ------------------------------------ |
| [OpenWebUI](integrations/open-webui.md)        | Chat interface with Ollama emulation |
| [LangChain](integrations/langchain.md)         | Retriever and agent integration      |
| [Custom Clients](integrations/custom-clients.md) | Thin HTTP cookbook (prefer SDKs)   |

### SDKs

| Guide | Description |
| ----- | ----------- |
| [SDK index](sdks/README.md) | Python, TypeScript, Rust, Go, Java, Kotlin, Swift, C#, Ruby, PHP |
| [Brutal SDK assessment](sdks/BRUTAL-ASSESSMENT.md) | Parity gaps and tiering (honest) |

SDK packages are independently versioned (typically **0.4.0**) and are **not** the same number as the product release (**0.23.0**).

### API Reference

| API                                                         | Description                         |
| ----------------------------------------------------------- | ----------------------------------- |
| [REST API](api-reference/rest-api.md)                       | Guided overlay + key endpoints      |
| [Extended API](api-reference/extended-api.md)               | Tasks, progress, cancel, metrics    |
| [Document upload quick reference](api-reference/document-upload-quick-reference.md) | Text vs PDF vs batch |
| [Lineage endpoints](api-reference/lineage-endpoints.md)     | Provenance and source tracing       |
| OpenAPI snapshot | [`edgequake_webui/openapi/openapi.snapshot.json`](../edgequake_webui/openapi/openapi.snapshot.json) |

### Reference

| Resource              | Description                        |
| --------------------- | ---------------------------------- |
| [Cookbook](cookbook.md) | Practical recipes                |
| [FAQ](faq.md)         | Frequently asked questions         |
| [Feature registry](features.md) | FEAT IDs grounded in code    |
| [Changelog](../CHANGELOG.md) | Product release history       |

### Operations

| Guide                                                            | Description                              |
| ---------------------------------------------------------------- | ---------------------------------------- |
| [Docker quickstart](operations/docker-quickstart.md)             | GHCR images, one-command stack           |
| [Deployment](operations/deployment.md)                           | Production deployment                    |
| [Configuration](operations/configuration.md)                     | Env vars and model catalog               |
| [Runtime auth hardening](operations/runtime-auth-hardening.md)   | Auth-on-by-default, bootstrap admin      |
| [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md)  | SPEC-057 claim/lease, cancel, replicas   |
| [Migrate to v0.23.0](operations/migrate-to-0.23.md)              | Schema migrate: fresh install vs upgrade |
| [Release and CD](operations/release-and-cd.md)                   | Tag, GHCR, quality gates                 |
| [Monitoring](operations/monitoring.md)                           | Health, ready, metrics                   |
| [Performance Tuning](operations/performance-tuning.md)           | Optimization guide                       |
| [Metadata debugging](operations/metadata-debugging.md)           | Document status / mapper fields          |
| [Operations Overview](operations/index.md)                       | Local and CI/CD operating model          |
| [Observability](OBSERVABILITY.md)                                | OTEL / tracing                           |
| [Langfuse 3.1.x](operations/langfuse-3.1.md)                     | Native ingestion fallback (no OTLP)      |
| [SQLx offline mode](sqlx-offline-mode.md)                        | Offline query metadata                   |
| [SPEC-083 improvements](../specs/083-improvements/README.md)              | First-principles defect pack + register  |
| [Prod eq_* incident](../specs/083-improvements/INCIDENT-PROD-DIAGNOSIS.md) | Schema readiness / M092 maintenance     |

### Security & Troubleshooting

| Guide                                                   | Description         |
| ------------------------------------------------------- | ------------------- |
| [Security Best Practices](security/best-practices.md)   | Security guidelines |
| [Common Issues](troubleshooting/common-issues.md)       | Debugging guide     |

---

## Quick Links

| Goal                          | Go To                                                      |
| ----------------------------- | ---------------------------------------------------------- |
| Get running in 5 minutes      | [Quick Start](getting-started/quick-start.md)              |
| Pin Docker images to 0.23.0   | [Docker quickstart](operations/docker-quickstart.md)       |
| Cancel / claim / lease ops    | [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md) |
| See API contract              | [OpenAPI snapshot](../edgequake_webui/openapi/openapi.snapshot.json) |
| Use an official SDK           | [SDKs](sdks/README.md)                                     |
| Deploy to production          | [Deployment](operations/deployment.md)                     |

---

## Technology Stack

```
┌─────────────────────────────────────────────────────────────┐
│ Technology stack                                            │
│                                                             │
│  Backend:  Rust 1.95 | Axum | SQLx | Tokio                  │
│  Frontend: Next.js 16.2 | React 19 | Sigma                  │
│  Storage:  PostgreSQL 16/17/18                              │
│            pgvector 0.8.3 | Apache AGE 1.6/1.7              │
│  Images:   ghcr.io/raphaelmansuy/edgequake*:0.23.0          │
└─────────────────────────────────────────────────────────────┘
```

---

## One-Liner Start

```bash
# Clone and run with Ollama (free, local LLM)
git clone https://github.com/raphaelmansuy/edgequake.git && cd edgequake && make dev
```

Or pull prebuilt images:

```bash
EDGEQUAKE_VERSION=0.23.0 docker compose -f docker-compose.quickstart.yml up -d
```

- API: http://localhost:8080
- WebUI: http://localhost:3000

---

## License

Apache-2.0

## Links

- **GitHub**: [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)
- **Releases**: [v0.23.0](https://github.com/raphaelmansuy/edgequake/releases/tag/v0.23.0)
- **LightRAG Paper**: [arxiv.org/abs/2410.05779](https://arxiv.org/abs/2410.05779)
