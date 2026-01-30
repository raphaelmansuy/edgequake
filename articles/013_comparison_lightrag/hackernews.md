# LightRAG vs EdgeQuake: Research Implementation vs Production Implementation

**An honest comparison for those evaluating graph-enhanced RAG systems**

---

In October 2024, researchers at HKUDS published "LightRAG: Simple and Fast Retrieval-Augmented Generation" (arXiv:2410.05779). The paper solved a real problem: traditional RAG treats documents as flat chunks, losing the relationships that make context meaningful.

EdgeQuake implements the same algorithm in Rust with different goals. This post compares the two honestly.

## The Architecture Difference

**LightRAG** uses multiple specialized databases:

- Neo4j for graph storage
- Pinecone/Weaviate for vectors
- Redis for caching
- JSON files for metadata

**EdgeQuake** uses PostgreSQL with extensions:

- Apache AGE for graph queries (Cypher-compatible)
- pgvector for vector similarity
- Standard tables for everything else

The LightRAG approach gives you best-in-class for each concern. The EdgeQuake approach gives you one system to backup, monitor, and debug. In production, the operational complexity of 4 systems often outweighs the theoretical benefits of specialization.

## Query Modes

LightRAG provides 3: Local (entity-focused), Global (community-based), Hybrid (combined).

EdgeQuake adds 3 more:

- **Naive**: Skip graph, pure vector search for simple queries
- **Mix**: Weighted combination with tunable parameters
- **Bypass**: Skip RAG entirely for chat that doesn't need documents

The additions reflect production realities. Not every query needs graph traversal. Sometimes you just want to chat.

## Production Features

This is where they diverge most:

| Feature            | LightRAG             | EdgeQuake                   |
| ------------------ | -------------------- | --------------------------- |
| Health endpoints   | DIY                  | GET /health, /ready, /live  |
| Connection pooling | Per-database, manual | Built-in SQLx with limits   |
| Multi-tenancy      | Not built-in         | Row-Level Security          |
| Cost tracking      | None                 | Per-document, per-operation |
| Graceful shutdown  | DIY                  | Built-in drain pattern      |
| Streaming          | Limited              | Full SSE support            |
| Docker             | Basic                | Multi-stage, non-root       |

These aren't features you can skip in production. They're table stakes. LightRAG leaves them to the deploying team; EdgeQuake includes them.

## Performance

We won't claim "EdgeQuake is faster" without benchmarks. The honest answer: the LLM is the bottleneck. Both implementations spend most of their time waiting for API responses. Language choice matters more for concurrent connection handling than raw speed.

The real performance gain is operational. EdgeQuake deployments don't require the 3-6 months of additional DevOps work that LightRAG production deployments typically need.

## When to Use Each

**LightRAG**:

- Prototyping in Jupyter notebooks
- Python ecosystem is essential
- Existing Neo4j infrastructure
- Single-user, local deployment
- Research and experimentation

**EdgeQuake**:

- Production Kubernetes
- Multi-tenant SaaS
- PostgreSQL standardization
- Day-one operational patterns
- Cost visibility required

## Recommendation

Don't choose one. Use both.

1. Validate graph-RAG with LightRAG (fast, Python, notebooks)
2. Deploy to production with EdgeQuake (ops patterns, single database)

Both are open source. Both implement the same algorithm. The difference is the journey from notebook to production.

---

**Credit**: EdgeQuake exists because of LightRAG research. Thank you to Guo, Xia, Yu, Ao, and Huang.

- Paper: https://arxiv.org/abs/2410.05779
- LightRAG: https://github.com/HKUDS/LightRAG
- EdgeQuake: https://github.com/raphaelmansuy/edgequake
