---
title: 'EdgeQuake Documentation'
---

# EdgeQuake Documentation

> **High-Performance Graph-Enhanced RAG in Rust**

Welcome to EdgeQuake — an advanced Retrieval-Augmented Generation (RAG) framework that combines knowledge graphs with vector search for superior context retrieval. The current stack is pinned for deterministic development and CI behavior, with PostgreSQL as the required storage backend and fail-closed workspace isolation on destructive and query flows.

```
┌────────────────────────────────────────────────────────────────────┐
│                         EdgeQuake                                  │
│                                                                    │
│    Document ──▶ [Pipeline] ──▶ Knowledge Graph ──▶ Query Engine    │
│                     │              │                    │          │
│                     ▼              ▼                    ▼          │
│               ┌─────────┐    ┌─────────┐         ┌─────────┐       │
│               │ Chunks  │    │ Entities│         │ Hybrid  │       │
│               │ + Embed │    │ + Rels  │         │ Results │       │
│               └─────────┘    └─────────┘         └─────────┘       │
│                                                                    │
│    [REST API]  [Next.js WebUI]  [Rust SDK]  [PostgreSQL + AGE]     │
└────────────────────────────────────────────────────────────────────┘
```

---

## 📚 Documentation Index

### 🚀 Getting Started

| Guide                                                  | Description                | Time   |
| ------------------------------------------------------ | -------------------------- | ------ |
| [Installation](/docs/getting-started/installation/)    | Prerequisites and setup    | 5 min  |
| [Quick Start](/docs/getting-started/quick-start/)      | First ingestion and query  | 10 min |
| [First Ingestion](/docs/tutorials/document-ingestion/) | Understanding the pipeline | 15 min |

### 🏗️ Architecture

| Document                                   | Description                           |
| ------------------------------------------ | ------------------------------------- |
| [Overview](/docs/architecture/overview/)   | System design and components          |
| [Data Flow](/docs/architecture/data-flow/) | How documents flow through the system |
| [Crate Reference](architecture/crates/)    | 11 Rust crates explained              |

### 💡 Core Concepts

| Concept                                                | Description                       |
| ------------------------------------------------------ | --------------------------------- |
| [Graph-RAG](/docs/concepts/graph-rag/)                 | Why knowledge graphs enhance RAG  |
| [Entity Extraction](/docs/concepts/entity-extraction/) | LLM-based entity recognition      |
| [Knowledge Graph](/docs/concepts/knowledge-graph/)     | Nodes, edges, and communities     |
| [Hybrid Retrieval](/docs/concepts/hybrid-retrieval/)   | Combining vector and graph search |

### 🔬 Deep Dives

| Article                                                        | Description                                  |
| -------------------------------------------------------------- | -------------------------------------------- |
| [LightRAG Algorithm](/docs/deep-dives/lightrag-algorithm/)     | Core algorithm: extraction, graph, retrieval |
| [Query Modes](/docs/deep-dives/query-modes/)                   | 6 modes with trade-offs explained            |
| [Entity Normalization](/docs/deep-dives/entity-normalization/) | Deduplication and description merging        |
| [Gleaning](/docs/deep-dives/gleaning/)                         | Multi-pass extraction for completeness       |
| [Entity Extraction](/docs/deep-dives/entity-extraction/)       | LLM-based extraction pipeline                |
| [Community Detection](/docs/deep-dives/community-detection/)   | Louvain clustering for global queries        |
| [Chunking Strategies](/docs/deep-dives/chunking-strategies/)   | Token-based segmentation with overlap        |
| [Embedding Models](/docs/deep-dives/embedding-models/)         | Model selection and dimension trade-offs     |
| [Graph Storage](/docs/deep-dives/graph-storage/)               | Apache AGE property graph backend            |
| [Vector Storage](/docs/deep-dives/vector-storage/)             | pgvector HNSW indexing and search            |
| [PDF Processing](/docs/deep-dives/pdf-processing/)             | Vision and EdgeParse extraction pipeline     |
| [Cost Tracking](/docs/deep-dives/cost-tracking/)               | LLM cost monitoring per operation            |
| [Pipeline Progress](/docs/deep-dives/pipeline-progress/)       | Real-time progress tracking                  |

### 📊 Comparisons

| Comparison                                                    | Key Insights                       |
| ------------------------------------------------------------- | ---------------------------------- |
| [vs LightRAG (Python)](/docs/comparisons/vs-lightrag-python/) | Performance and design differences |
| [vs GraphRAG](/docs/comparisons/vs-graphrag/)                 | Microsoft's approach comparison    |
| [vs Traditional RAG](/docs/comparisons/vs-traditional-rag/)   | Why graphs matter                  |

### 📖 Tutorials

| Tutorial                                                            | Description                     |
| ------------------------------------------------------------------- | ------------------------------- |
| [Building Your First RAG App](/docs/tutorials/first-rag-app/)       | End-to-end tutorial             |
| [PDF Ingestion](/docs/tutorials/pdf-ingestion/)                     | PDF upload and configuration    |
| [Multi-Tenant Setup](/docs/tutorials/multi-tenant/)                 | Workspace isolation             |
| [Document Ingestion](/docs/tutorials/document-ingestion/)           | Upload and processing workflows |
| [Migration from LightRAG](/docs/tutorials/migration-from-lightrag/) | Python to Rust migration guide  |

### 🔌 Integrations

| Integration                                          | Description                          |
| ---------------------------------------------------- | ------------------------------------ |
| [OpenWebUI](/docs/integrations/open-webui/)          | Chat interface with Ollama emulation |
| [LangChain](/docs/integrations/langchain/)           | Retriever and agent integration      |
| [Custom Clients](/docs/integrations/custom-clients/) | Python, TypeScript, Rust, Go clients |

### 📦 SDKs (by language)

| Guide | Description |
| ----- | ----------- |
| [SDK index](/docs/sdks/) | Rust, Python, TypeScript, Kotlin, Swift, Go, Java, C#, Ruby |
| [Brutal SDK assessment](/docs/sdks/BRUTAL-ASSESSMENT.md) | Parity gaps and tiering (honest) |

### 📖 API Reference

| API                                               | Description           |
| ------------------------------------------------- | --------------------- |
| [REST API](/docs/api-reference/rest-api/)         | HTTP endpoints        |
| [Extended API](/docs/api-reference/extended-api/) | Advanced API features |

### 📓 Reference

| Resource                    | Description                        |
| --------------------------- | ---------------------------------- |
| [Cookbook](/docs/cookbook/) | Practical recipes for common tasks |
| [FAQ](/docs/faq/)           | Frequently asked questions         |

### 🛠️ Operations

| Guide                                                      | Description                              |
| ---------------------------------------------------------- | ---------------------------------------- |
| [Deployment](/docs/operations/deployment/)                 | Production deployment                    |
| [Configuration](/docs/operations/configuration/)           | All config options                       |
| [Monitoring](/docs/operations/monitoring/)                 | Observability setup                      |
| [Performance Tuning](/docs/operations/performance-tuning/) | Optimization guide                       |
| [Operations Overview](/docs/operations/)                   | Reliable local and CI/CD operating model |

### 🔒 Security & Troubleshooting

| Guide                                                     | Description         |
| --------------------------------------------------------- | ------------------- |
| [Security Best Practices](/docs/security/best-practices/) | Security guidelines |
| [Common Issues](/docs/troubleshooting/common-issues/)     | Debugging guide     |

---

## ⚡ Quick Links

**I want to...**

| Goal                          | Go To                                                      |
| ----------------------------- | ---------------------------------------------------------- |
| Get running in 5 minutes      | [Quick Start](/docs/getting-started/quick-start/)          |
| Understand the architecture   | [Overview](/docs/architecture/overview/)                   |
| Learn how the algorithm works | [LightRAG Algorithm](/docs/deep-dives/lightrag-algorithm/) |
| See API endpoints             | [REST API](/docs/api-reference/rest-api/)                  |
| Use an official SDK           | [SDKs](/docs/sdks/)                                        |
| Deploy to production          | [Deployment](/docs/operations/deployment/)                 |

---

## 🔧 Technology Stack

```
┌─────────────────────────────────────────────────────────────┐
│                        Backend (Rust)                       │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐ │
│  │ Rust 1.95 │  │   Axum    │  │   SQLx    │  │ Tokio +   │ │
│  │ + Cargo   │  │  (HTTP)   │  │ (database)│  │ tracing   │ │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘ │
├─────────────────────────────────────────────────────────────┤
│                       Frontend (TypeScript)                 │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐ │
│  │ Next.js   │  │  React 19 │  │ Sigma.js  │  │  Zustand  │ │
│  │  16.2.x   │  │   19.2.x  │  │  (graph)  │  │  (state)  │ │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘ │
├─────────────────────────────────────────────────────────────┤
│                         Storage                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ PostgreSQL 15+ with pgvector + Apache AGE            │  │
│  │ Required for server mode; no in-memory fallback      │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 📈 Key Metrics

| Metric             | Value        | Notes                                                               |
| ------------------ | ------------ | ------------------------------------------------------------------- |
| **Lines of Rust**  | ~130,000     | Across 11 crates                                                    |
| **Query Modes**    | 6            | naive, local, global, hybrid, mix, bypass                           |
| **Entity Types**   | 7 default    | PERSON, ORGANIZATION, LOCATION, CONCEPT, EVENT, TECHNOLOGY, PRODUCT |
| **Embedding Dims** | Configurable | 1536 (OpenAI), 768 (Ollama/LM Studio)                               |

---

## 🏃 One-Liner Start

```bash
# Clone and run with Ollama (free, local LLM)
git clone https://github.com/raphaelmansuy/edgequake.git && cd edgequake && make dev
```

---

## 📄 License

Apache-2.0

---

## 🔗 Links

- **GitHub**: [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)
- **LightRAG Paper**: [arxiv.org/abs/2410.05779](https://arxiv.org/abs/2410.05779)
