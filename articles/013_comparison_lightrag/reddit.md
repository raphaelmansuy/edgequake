# LightRAG vs EdgeQuake: An Honest Comparison for r/rust and r/MachineLearning

**TL;DR**: LightRAG is excellent research software. EdgeQuake makes it production-ready. Use both for different stages of your journey.

---

## Background

If you're building RAG systems, you've probably hit the same problem: traditional vector search treats documents as flat chunks and loses the relationships between entities.

In October 2024, researchers at HKUDS published "LightRAG: Simple and Fast Retrieval-Augmented Generation" ([arXiv:2410.05779](https://arxiv.org/abs/2410.05779)). Their solution: incorporate graph structures into retrieval. Extract entities and relationships, build a knowledge graph, then search both the graph and vectors.

It works. Graph-enhanced RAG produces more coherent answers for complex queries.

EdgeQuake is a Rust implementation of the same algorithm with production patterns built-in. This post compares the two honestly—neither is "better," they're for different use cases.

---

## The Key Differences

### Storage Architecture

**LightRAG** uses 4 systems:

```
Neo4j (graph) + Pinecone/Weaviate (vectors) + Redis (cache) + JSON (metadata)
```

**EdgeQuake** uses 1:

```
PostgreSQL with Apache AGE (graph) + pgvector (vectors)
```

For r/rust folks: yes, this is the "one connection string" vs "four connection strings" meme. In production, fewer systems means fewer backup targets, fewer monitoring dashboards, and fewer things to debug at 3am.

### Query Modes

| Mode   | LightRAG | EdgeQuake |
| ------ | -------- | --------- |
| Naive  | ❌       | ✅        |
| Local  | ✅       | ✅        |
| Global | ✅       | ✅        |
| Hybrid | ✅       | ✅        |
| Mix    | ❌       | ✅        |
| Bypass | ❌       | ✅        |

The additions aren't feature creep. `Naive` skips the graph for simple factual queries. `Bypass` skips RAG entirely for chat that doesn't need documents. Not every query needs the full pipeline.

### Production Features

This is where they diverge:

**LightRAG provides**: The algorithm. Everything else is DIY.

**EdgeQuake includes**:

- Health endpoints (GET /health, /ready, /live for K8s probes)
- Connection pooling (SQLx with configurable limits)
- Multi-tenancy (Row-Level Security at the database level)
- Cost tracking (per-document, per-operation)
- Graceful shutdown (drain pattern for zero-downtime deploys)
- Streaming responses (SSE for real-time generation)
- Non-root Docker images (security baseline)

For r/MachineLearning folks: these aren't optional in production ML. They're the difference between a demo and a deployed system.

---

## When to Use Each

### LightRAG

- Prototyping in Jupyter notebooks
- Python ecosystem integration is essential
- You have existing Neo4j infrastructure
- Single-user, local deployment
- Research and experimentation

### EdgeQuake

- Production Kubernetes deployment
- Multi-tenant SaaS requirements
- PostgreSQL standardization
- Day-one operational patterns needed
- Cost tracking required for optimization

---

## The Recommendation

**Don't choose one. Use both.**

1. Validate your use case with LightRAG (fast iteration, Python notebooks)
2. Deploy to production with EdgeQuake (ops patterns, single database)

Both are open source. Both implement the same algorithm. The difference is where you are in the notebook-to-production journey.

---

## Links

- **LightRAG Paper**: [arXiv:2410.05779](https://arxiv.org/abs/2410.05779)
- **LightRAG Repo**: [github.com/HKUDS/LightRAG](https://github.com/HKUDS/LightRAG)
- **EdgeQuake Repo**: [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)

---

**Credit where due**: EdgeQuake exists because of LightRAG research. Thank you to Guo, Xia, Yu, Ao, and Huang at HKUDS for the foundational work.

Happy to answer questions about the implementation differences. For r/rust: the async story is Tokio-based, and the storage layer uses SQLx. For r/ML: the entity extraction follows the paper's algorithm with tuple-format output for streaming compatibility.
