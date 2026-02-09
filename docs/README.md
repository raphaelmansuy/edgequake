# EdgeQuake Documentation

> **High-Performance Graph-Enhanced RAG in Rust**

Welcome to EdgeQuake — an advanced Retrieval-Augmented Generation (RAG) framework that combines knowledge graphs with vector search for superior context retrieval.

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
│    [REST API]  [Next.js WebUI]  [Rust SDK]  [PostgreSQL/Memory]    │
└────────────────────────────────────────────────────────────────────┘
```

---

## 📚 Documentation Index

### 🚀 Getting Started

| Guide                                                 | Description                | Time   |
| ----------------------------------------------------- | -------------------------- | ------ |
| [Installation](getting-started/installation.md)       | Prerequisites and setup    | 5 min  |
| [Quick Start](getting-started/quick-start.md)         | First ingestion and query  | 10 min |
| [First Ingestion](getting-started/first-ingestion.md) | Understanding the pipeline | 15 min |

### 🏗️ Architecture

| Document                                | Description                           |
| --------------------------------------- | ------------------------------------- |
| [Overview](architecture/overview.md)    | System design and components          |
| [Data Flow](architecture/data-flow.md)  | How documents flow through the system |
| [Crate Reference](architecture/crates/) | 11 Rust crates explained              |

### 💡 Core Concepts

| Concept                                            | Description                       |
| -------------------------------------------------- | --------------------------------- |
| [Graph-RAG](concepts/graph-rag.md)                 | Why knowledge graphs enhance RAG  |
| [Entity Extraction](concepts/entity-extraction.md) | LLM-based entity recognition      |
| [Knowledge Graph](concepts/knowledge-graph.md)     | Nodes, edges, and communities     |
| [Hybrid Retrieval](concepts/hybrid-retrieval.md)   | Combining vector and graph search |

### 🔬 Deep Dives

| Article                                                    | Description                                  |
| ---------------------------------------------------------- | -------------------------------------------- |
| [LightRAG Algorithm](deep-dives/lightrag-algorithm.md)     | Core algorithm: extraction, graph, retrieval |
| [Query Modes](deep-dives/query-modes.md)                   | 6 modes with trade-offs explained            |
| [Entity Normalization](deep-dives/entity-normalization.md) | Deduplication and description merging        |
| [Gleaning](deep-dives/gleaning.md)                         | Multi-pass extraction for completeness       |
| [Entity Extraction](deep-dives/entity-extraction.md)       | LLM-based extraction pipeline                |
| [Community Detection](deep-dives/community-detection.md)   | Louvain clustering for global queries        |
| [Chunking Strategies](deep-dives/chunking-strategies.md)   | Token-based segmentation with overlap        |
| [Embedding Models](deep-dives/embedding-models.md)         | Model selection and dimension trade-offs     |
| [Graph Storage](deep-dives/graph-storage.md)               | Apache AGE property graph backend            |
| [Vector Storage](deep-dives/vector-storage.md)             | pgvector HNSW indexing and search            |
| [PDF Processing](deep-dives/pdf-processing.md)             | Text/Vision/Hybrid extraction pipeline       |
| [Cost Tracking](deep-dives/cost-tracking.md)               | LLM cost monitoring per operation            |
| [Pipeline Progress](deep-dives/pipeline-progress.md)       | Real-time progress tracking                  |

### 📊 Comparisons

| Comparison                                                | Key Insights                       |
| --------------------------------------------------------- | ---------------------------------- |
| [vs LightRAG (Python)](comparisons/vs-lightrag-python.md) | Performance and design differences |
| [vs GraphRAG](comparisons/vs-graphrag.md)                 | Microsoft's approach comparison    |
| [vs Traditional RAG](comparisons/vs-traditional-rag.md)   | Why graphs matter                  |

### 📖 Tutorials

| Tutorial                                                       | Description                     |
| -------------------------------------------------------------- | ------------------------------- |
| [Building Your First RAG App](tutorials/first-rag-app.md)      | End-to-end tutorial             |
| [PDF Ingestion](tutorials/pdf-ingestion.md)                    | PDF upload and configuration    |
| [Multi-Tenant Setup](tutorials/multi-tenant.md)                | Workspace isolation             |
| [Document Ingestion](tutorials/document-ingestion.md)          | Upload and processing workflows |
| [Migration from LightRAG](tutorials/migration-from-lightrag.md)| Python to Rust migration guide  |

### 🔌 Integrations

| Integration                                      | Description                          |
| ------------------------------------------------ | ------------------------------------ |
| [OpenWebUI](integrations/open-webui.md)          | Chat interface with Ollama emulation |
| [LangChain](integrations/langchain.md)           | Retriever and agent integration      |
| [Custom Clients](integrations/custom-clients.md) | Python, TypeScript, Rust, Go clients |

### 📖 API Reference

| API                                              | Description           |
| ------------------------------------------------ | --------------------- |
| [REST API](api-reference/rest-api.md)            | HTTP endpoints        |
| [Extended API](api-reference/extended-api.md)    | Advanced API features |

### 📓 Reference

| Resource                                             | Description                        |
| ---------------------------------------------------- | ---------------------------------- |
| [Cookbook](cookbook.md)                                | Practical recipes for common tasks |
| [FAQ](faq.md)                                        | Frequently asked questions         |

### 🛠️ Operations

| Guide                                                      | Description            |
| ---------------------------------------------------------- | ---------------------- |
| [Deployment](operations/deployment.md)                     | Production deployment  |
| [Configuration](operations/configuration.md)               | All config options     |
| [Monitoring](operations/monitoring.md)                     | Observability setup    |
| [Performance Tuning](operations/performance-tuning.md)     | Optimization guide     |

### 🔒 Security & Troubleshooting

| Guide                                                    | Description                  |
| -------------------------------------------------------- | ---------------------------- |
| [Security Best Practices](security/best-practices.md)   | Security guidelines          |
| [Common Issues](troubleshooting/common-issues.md)       | Debugging guide              |

---

## ⚡ Quick Links

**I want to...**

| Goal                          | Go To                                                  |
| ----------------------------- | ------------------------------------------------------ |
| Get running in 5 minutes      | [Quick Start](getting-started/quick-start.md)          |
| Understand the architecture   | [Overview](architecture/overview.md)                   |
| Learn how the algorithm works | [LightRAG Algorithm](deep-dives/lightrag-algorithm.md) |
| See API endpoints             | [REST API](api-reference/rest-api.md)                  |
| Deploy to production          | [Deployment](operations/deployment.md)                 |

---

## 🔧 Technology Stack

```
┌─────────────────────────────────────────────────────────────┐
│                        Backend (Rust)                       │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐ │
│  │   Tokio   │  │   Axum    │  │   SQLx    │  │ async-    │ │
│  │  (async)  │  │  (HTTP)   │  │ (database)│  │ openai    │ │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘ │
├─────────────────────────────────────────────────────────────┤
│                       Frontend (TypeScript)                 │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐ │
│  │ Next.js   │  │  React 19 │  │ Sigma.js  │  │  Zustand  │ │
│  │  16.1.0   │  │   19.2.3  │  │  (graph)  │  │  (state)  │ │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘ │
├─────────────────────────────────────────────────────────────┤
│                         Storage                             │
│  ┌───────────────────────┐  ┌───────────────────────────┐   │
│  │    PostgreSQL 15+     │  │      In-Memory (dev)      │   │
│  │ + pgvector + Apache AGE│  │   Fast prototyping       │   │
│  └───────────────────────┘  └───────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 📈 Key Metrics

| Metric             | Value        | Notes                                                   |
| ------------------ | ------------ | ------------------------------------------------------- |
| **Lines of Rust**  | ~130,000     | Across 11 crates                                        |
| **Query Modes**    | 6            | naive, local, global, hybrid, mix, bypass               |
| **Entity Types**   | 7 default    | PERSON, ORGANIZATION, LOCATION, CONCEPT, EVENT, TECHNOLOGY, PRODUCT |
| **Embedding Dims** | Configurable | 1536 (OpenAI), 768 (Ollama/LM Studio)                              |

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
