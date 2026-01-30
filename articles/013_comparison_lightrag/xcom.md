# EdgeQuake vs LightRAG - A Technical Comparison (X.com Thread)

## Tweet 1/14

LightRAG vs EdgeQuake: An honest comparison of two Graph-RAG implementations.

One is research-grade Python.
One is production-grade Rust.

Both implement the same algorithm.

Here's when to use each 🧵

## Tweet 2/14

First: Thank you to Guo, Xia, Yu, Ao, and Huang.

Their LightRAG paper (arXiv:2410.05779) solved a fundamental problem: flat RAG can't capture relationships.

We read that paper and asked: "How do we make this production-ready?"

EdgeQuake is the answer.

## Tweet 3/14

The Algorithm (same in both):

Traditional RAG:
Documents → Chunks → Vectors → Search

LightRAG/EdgeQuake:
Documents → Entities + Relationships → Knowledge Graph → Graph + Vector Search

The graph preserves context. Answers are more coherent.

## Tweet 4/14

STORAGE: The biggest difference

LightRAG uses 4 systems:
• Neo4j (graph)
• Pinecone/Weaviate (vectors)
• Redis (cache)
• JSON files (metadata)

EdgeQuake uses 1:
• PostgreSQL with AGE + pgvector

4 backups vs 1 backup. Your ops team knows the difference.

## Tweet 5/14

Why 4 databases matters:

• 4 systems to deploy
• 4 systems to monitor
• 4 systems to backup
• No cross-system transactions
• Consistency challenges during failures

PostgreSQL gives you ACID across graph AND vectors. One connection string.

## Tweet 6/14

QUERY MODES: 3 vs 6

LightRAG: Local, Global, Hybrid
EdgeQuake: + Naive, Mix, Bypass

The additions matter:
• Naive: Skip graph for simple queries
• Bypass: Skip RAG for chat that doesn't need docs

Not every query needs the full pipeline.

## Tweet 7/14

PRODUCTION FEATURES

LightRAG:
❌ Health endpoints
❌ Connection pooling
❌ Multi-tenancy
❌ Cost tracking
❌ Graceful shutdown

EdgeQuake:
✅ GET /health, /ready, /live
✅ SQLx pooling built-in
✅ Row-Level Security
✅ Per-doc cost tracking
✅ Drain connections on shutdown

## Tweet 8/14

The 3am page story:

LightRAG in production → connection pool exhaustion → your SRE's phone rings.

EdgeQuake has pooling built-in with configurable limits:
• max_connections: 10
• acquire_timeout: 30s
• min_connections: 1

No 3am pages from connection storms.

## Tweet 9/14

Multi-tenancy:

LightRAG: All users share the same graph. DIY isolation.

EdgeQuake: Workspace-based isolation with PostgreSQL Row-Level Security.

Tenant A cannot see Tenant B's data. Enforced at the database level, not the application.

## Tweet 10/14

Cost visibility:

LightRAG: No idea what you're spending.

EdgeQuake: Per-document, per-operation cost tracking.

When extraction costs 60% and embedding costs 10%, you know where to optimize.

## Tweet 11/14

When to use LightRAG:

✅ Prototyping in Jupyter
✅ Python ecosystem essential
✅ Existing Neo4j infrastructure
✅ Single-user, local deployment
✅ Research and experimentation

It's excellent research software. Use it for validation.

## Tweet 12/14

When to use EdgeQuake:

✅ Production Kubernetes
✅ Multi-tenant SaaS
✅ PostgreSQL standardization
✅ Day-one ops patterns needed
✅ Cost tracking required
✅ Streaming responses

It's production software. Use it for deployment.

## Tweet 13/14

Our recommendation:

Don't choose one or the other.

1. Prototype with LightRAG (notebooks, fast iteration)
2. Validate graph-RAG improves your queries
3. Deploy with EdgeQuake (production patterns)

Same algorithm. Different stages of the journey.

## Tweet 14/14

Links:

📄 LightRAG Paper: arxiv.org/abs/2410.05779
🔗 LightRAG: github.com/HKUDS/LightRAG
🔗 EdgeQuake: github.com/raphaelmansuy/edgequake

Credit where due: EdgeQuake exists because of LightRAG research.

Thank you to the HKUDS team.
