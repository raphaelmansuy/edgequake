# Show HN: Graph-RAG for multi-hop queries in regulated industries

**Title**: Show HN: EdgeQuake – Graph-RAG that understands relationships, not just keywords

---

## Post Body

Hi HN,

I've been working on EdgeQuake, a Rust-based Graph-RAG framework that addresses a specific limitation of baseline RAG: multi-hop reasoning.

**The Problem**

Vector search excels at "find documents similar to this query." But it fails when questions require understanding relationships:

- "Find contracts where Party A has unlimited liability AND termination allows exit without cause"
- "Find patients with diabetes + metformin + declining kidney function"
- "Show companies where revenue recognition changed within 90 days of CFO departure"

These queries require traversing relationships between entities across multiple documents. Vector similarity can't do that.

**The Technical Approach**

EdgeQuake builds a knowledge graph during document ingestion:

1. **Chunking**: Documents → 1200-token chunks with overlap
2. **Extraction**: LLM extracts entities and relationships from each chunk
3. **Graph Storage**: PostgreSQL + Apache AGE for graph queries
4. **Embeddings**: pgvector for vector similarity (hybrid approach)
5. **Query**: Keyword extraction → Graph traversal → Vector refinement → LLM synthesis

The key insight (from the LightRAG paper [1]) is combining graph structure with vector search. Graph traversal identifies structurally relevant entities; vector search refines by semantic similarity.

**Why Regulated Industries**

We've focused on legal, healthcare, and finance because:

1. **Multi-hop queries are common**: Contract clause relationships, drug interactions, financial risk signals
2. **Data sovereignty matters**: We support Ollama for on-premise processing (HIPAA, SOX)
3. **Audit trails are required**: Every query is logged with document references
4. **Multi-tenancy is expected**: Row-Level Security in PostgreSQL isolates client data

**Results**

In testing with legal document corpora:

- Query: "Unlimited liability + termination without cause" across 50K contracts
- Vector search: 2,000 results (too many to review)
- Graph-RAG: 47 results (validated as accurate)

Time savings: 3 weeks → 3 days for M&A due diligence.

**Trade-offs**

Honest acknowledgment:

1. **Higher indexing cost**: Graph extraction requires more LLM calls than vector-only (~$0.0014/doc vs ~$0.0002/doc)
2. **More complex setup**: Requires PostgreSQL with AGE extension, not just a vector DB
3. **Schema sensitivity**: Entity extraction quality varies by domain (prompt tuning helps)

For simple semantic search, baseline RAG is fine. For relationship-heavy queries in regulated industries, Graph-RAG is worth the overhead.

**Source**

- GitHub: https://github.com/raphaelmansuy/edgequake
- Apache 2.0 license
- Built in Rust with Axum, PostgreSQL, Apache AGE

---

Interested in feedback:

1. Has anyone else hit the "multi-hop reasoning" wall with baseline RAG?
2. For those in regulated industries—what compliance features are must-haves?
3. Any experience with graph extraction prompt tuning for specific domains?

---

[1] LightRAG: Simple and Fast Retrieval-Augmented Generation - https://arxiv.org/abs/2410.05779
[2] Microsoft GraphRAG - https://arxiv.org/abs/2404.16130
