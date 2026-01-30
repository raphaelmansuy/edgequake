# EdgeQuake vs LightRAG: A Technical Comparison

## When to Use Each and Why We Built a Rust Implementation

_An honest comparison of two Graph-RAG implementations: research excellence vs production readiness_

---

This article wouldn't exist without the LightRAG research team.

In October 2024, Guo, Xia, Yu, Ao, and Huang from the Hong Kong University of Data Science published ["LightRAG: Simple and Fast Retrieval-Augmented Generation"](https://arxiv.org/abs/2410.05779). Their paper elegantly solved a fundamental problem in retrieval-augmented generation: flat data representations can't capture the relationships that make context meaningful.

We read that paper and asked a different question: _How do we make this production-ready?_

The result is EdgeQuake—a Rust implementation of the LightRAG algorithm with additional production patterns. This article compares the two implementations honestly, helping you decide which fits your use case.

---

## What is LightRAG?

Traditional RAG systems treat documents as bags of chunks. You split text, embed chunks into vectors, and retrieve the most similar ones to a query. This works for simple factual questions but fails for complex queries that require understanding relationships.

LightRAG's innovation: incorporate graph structures into text indexing and retrieval.

```
Traditional RAG:              LightRAG:
┌──────────────┐             ┌──────────────┐
│  Documents   │             │  Documents   │
│      ↓       │             │      ↓       │
│   Chunks     │             │   Chunks     │
│      ↓       │             │      ↓       │
│   Vectors    │             │ Entities +   │
│      ↓       │             │ Relationships│
│   Search     │             │      ↓       │
└──────────────┘             │ Knowledge    │
   Flat data                 │   Graph      │
   No relations              │      ↓       │
                             │ Graph + Vec  │
                             │   Search     │
                             └──────────────┘
                             Graph-enhanced
                             Relations intact
```

The paper identifies three problems with existing approaches:

1. **Flat data representations** can't capture entity relationships
2. **Inadequate contextual awareness** leads to fragmented answers
3. **Microsoft's GraphRAG** works but costs 610,000+ tokens per query

LightRAG solves these with:

- **Dual-level retrieval**: Local (entities) + Global (communities)
- **Incremental updates**: New data integrates without full reprocessing
- **Efficient search**: Graph traversal + vector similarity combined

The research is excellent. The implementation is designed for experimentation.

---

## EdgeQuake: A Production Implementation

EdgeQuake implements the same algorithm with different goals. Where LightRAG optimizes for Python notebooks and rapid prototyping, EdgeQuake optimizes for production deployment.

Key differences:

| Aspect           | LightRAG              | EdgeQuake          |
| ---------------- | --------------------- | ------------------ |
| **Language**     | Python (asyncio)      | Rust (Tokio async) |
| **Target**       | Research, prototyping | Production, SaaS   |
| **Storage**      | Multiple databases    | Single PostgreSQL  |
| **Query Modes**  | 3                     | 6                  |
| **Ops Features** | DIY                   | Built-in           |

Both implement the same core algorithm. The differences are in what surrounds it.

---

## Storage Architecture: 4 Databases vs 1

This is the most significant architectural difference.

### LightRAG Storage

LightRAG uses multiple specialized databases:

```
┌─────────────────────────────────────────────┐
│              LightRAG Storage               │
├─────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────────────┐ │
│  │   Neo4j     │    │  Vector Database    │ │
│  │   (Graph)   │    │ (Pinecone/Weaviate) │ │
│  └─────────────┘    └─────────────────────┘ │
│  ┌─────────────┐    ┌─────────────────────┐ │
│  │   Redis     │    │    JSON Files       │ │
│  │  (Cache)    │    │   (Metadata)        │ │
│  └─────────────┘    └─────────────────────┘ │
└─────────────────────────────────────────────┘
         4 systems to manage
```

**Pros**:

- Best-in-class for each concern
- Flexible provider choices
- Scales each component independently

**Cons**:

- 4 systems to deploy, monitor, backup
- No cross-system transactions
- Consistency challenges during failures

### EdgeQuake Storage

EdgeQuake uses PostgreSQL with extensions:

```
┌─────────────────────────────────────────────┐
│              EdgeQuake Storage              │
├─────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────┐│
│  │              PostgreSQL                 ││
│  │  ┌───────────────┐ ┌─────────────────┐  ││
│  │  │  Apache AGE   │ │    pgvector     │  ││
│  │  │   (Graph)     │ │   (Vectors)     │  ││
│  │  └───────────────┘ └─────────────────┘  ││
│  │  ┌───────────────────────────────────┐  ││
│  │  │       Standard Tables             │  ││
│  │  │       (Metadata, Cache)           │  ││
│  │  └───────────────────────────────────┘  ││
│  └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
         1 system to manage
```

**Pros**:

- Single backup target
- ACID transactions across graph + vectors
- Existing PostgreSQL expertise applies
- Simpler deployment

**Cons**:

- Can't scale graph independently of vectors
- PostgreSQL limits apply to all stores
- Requires AGE and pgvector extensions

For most production deployments, the operational simplicity of a single database outweighs the theoretical flexibility of multiple specialized systems.

---

## Query Mode Comparison

LightRAG introduced three query modes. EdgeQuake extends this to six.

| Mode       | LightRAG | EdgeQuake | Description                      |
| ---------- | -------- | --------- | -------------------------------- |
| **Naive**  | ❌       | ✅        | Direct chunk vector search       |
| **Local**  | ✅       | ✅        | Entity-focused with neighborhood |
| **Global** | ✅       | ✅        | Community/cluster-based search   |
| **Hybrid** | ✅       | ✅        | Combines local + global          |
| **Mix**    | ❌       | ✅        | Weighted combination of modes    |
| **Bypass** | ❌       | ✅        | Skip RAG, direct to LLM          |

**Why the additions?**

- **Naive mode**: For simple factual queries where graph traversal is overkill
- **Mix mode**: For fine-tuning the balance between local and global retrieval
- **Bypass mode**: For chat interactions that don't need document context

In production, having `bypass` mode prevents the "every query hits the database" anti-pattern for conversational flows.

---

## Production Features

This is where the implementations diverge most. LightRAG leaves production concerns to the deploying team. EdgeQuake includes them.

### Health Endpoints

**LightRAG**: None. You build your own.

**EdgeQuake**:

```
GET /health → {"status": "healthy", "components": [...]}
GET /ready  → {"status": "ready"}
GET /live   → {"status": "live"}
```

Three endpoints for Kubernetes probes. Liveness checks if the process is alive. Readiness checks if it's ready for traffic. Health provides component-level status.

### Connection Pooling

**LightRAG**: Each database connection is managed separately (or not at all).

**EdgeQuake**: Built-in SQLx connection pooling with configurable limits:

```rust
PgPoolOptions::new()
    .max_connections(10)
    .min_connections(1)
    .acquire_timeout(Duration::from_secs(30))
```

The 3am page from connection pool exhaustion is a rite of passage for LightRAG production deployments. EdgeQuake prevents it by default.

### Multi-Tenancy

**LightRAG**: No built-in isolation. Multiple users share the same graph.

**EdgeQuake**: Workspace-based isolation with `workspace_id`:

```sql
CREATE POLICY workspace_isolation ON entities
    USING (workspace_id = current_setting('app.workspace_id'));
```

Row-Level Security ensures tenants can't see each other's data. Essential for SaaS.

### Cost Tracking

**LightRAG**: No visibility into LLM costs.

**EdgeQuake**: Per-document, per-operation cost tracking:

```rust
pub struct CostBreakdown {
    pub total_cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub operations: HashMap<String, OperationCost>,
}
```

When you're processing 10,000 documents, knowing that extraction costs 60% and embedding costs 10% tells you where to optimize.

### Additional Production Patterns

| Feature              | LightRAG | EdgeQuake |
| -------------------- | -------- | --------- |
| Graceful Shutdown    | ❌       | ✅        |
| Streaming (SSE)      | Limited  | Full      |
| Runbook              | ❌       | 316 lines |
| Docker (Multi-stage) | Basic    | ✅        |
| Non-root Container   | ❌       | ✅        |

The cumulative effect: EdgeQuake deployments take hours; LightRAG production deployments take months of additional engineering.

---

## When to Use Each

### Use LightRAG When:

1. **Rapid prototyping** in Jupyter notebooks
2. **Python ecosystem** integration is essential
3. **Existing Neo4j** infrastructure is in place
4. **Team expertise** is Python-focused
5. **Simple deployment** requirements (single-user, local)

LightRAG is excellent for proving that graph-enhanced RAG works for your use case. It's research-grade software optimized for experimentation.

### Use EdgeQuake When:

1. **Production Kubernetes** deployment is the target
2. **Multi-tenant SaaS** requires isolation
3. **PostgreSQL standardization** is preferred
4. **Cost tracking** and observability are needed
5. **Streaming responses** are required
6. **Single-database** architecture is desired
7. **Operational patterns** (health, pooling, shutdown) must exist day one

EdgeQuake is production-grade software optimized for operations.

---

## Performance Considerations

We won't claim "EdgeQuake is faster" without benchmarks. What we can say:

**Language-level differences**:

- Rust has no GIL—true parallelism across cores
- Rust has no garbage collector—predictable latency
- Rust compiles to a single binary—no runtime dependencies

**In practice**: The LLM provider is the bottleneck. Document processing is IO-bound (waiting for API responses), not CPU-bound. Language choice matters more for concurrent connection handling than raw processing speed.

The real performance gain is operational: EdgeQuake deployments don't require the 3-6 months of DevOps work that LightRAG production deployments typically need.

---

## Algorithm Fidelity

Both implementations follow the LightRAG paper's algorithm:

1. **Entity Extraction**: LLM extracts entities and relationships from chunks
2. **Graph Construction**: Entities become nodes; relationships become edges
3. **Dual-Level Retrieval**: Local (entity neighborhood) + Global (community summaries)
4. **Context Fusion**: Combine retrieved context for LLM generation

EdgeQuake adds:

- **Tuple format** for extraction (streaming-friendly vs JSON)
- **Progressive token scaling** (retry with more tokens if truncated)
- **Gleaning** (multi-pass extraction for dense chunks)
- **UPPERCASE_UNDERSCORE normalization** (consistent entity naming)

These are implementation improvements, not algorithm changes.

---

## Conclusion

LightRAG is excellent research. EdgeQuake makes it production-ready.

This isn't a competition—it's a pipeline. Prototype with LightRAG in a notebook. Validate that graph-enhanced RAG improves your queries. Then deploy with EdgeQuake when you need health endpoints, connection pooling, multi-tenancy, and operational patterns.

**Our recommendation**:

- **Evaluating graph-RAG?** → Start with LightRAG
- **Deploying to production?** → Switch to EdgeQuake
- **Python team, simple needs?** → Stay with LightRAG
- **Kubernetes, multi-tenant SaaS?** → Choose EdgeQuake

Both projects are open source. Both implement the same algorithm. Choose based on your stage.

---

## Research Credit

This work builds on foundational research:

> **LightRAG: Simple and Fast Retrieval-Augmented Generation**  
> Guo, Z., Xia, L., Yu, Y., Ao, T., & Huang, C. (2024)  
> arXiv:2410.05779

We thank the authors for their contribution to the field. EdgeQuake would not exist without their work.

---

**EdgeQuake**: github.com/raphaelmansuy/edgequake  
**LightRAG**: github.com/HKUDS/LightRAG
