# Show HN: EdgeQuake – Graph-RAG on PostgreSQL + Apache AGE (no Neo4j)

**HackerNews Post**

---

## Title

Show HN: EdgeQuake – Graph-RAG on PostgreSQL + Apache AGE (no Neo4j)

## URL

https://github.com/raphaelmansuy/edgequake

## Text

Hey HN,

I've been building EdgeQuake, a Rust implementation of the LightRAG algorithm (arXiv:2410.05779) for Graph-RAG. The key architectural decision: **run everything on PostgreSQL**.

Most Graph-RAG implementations require Neo4j + Pinecone + PostgreSQL. That's three databases to sync, three backup strategies, and three bills. We replaced all of that with PostgreSQL extensions.

**The stack:**

- **Apache AGE** – Adds Cypher query language to PostgreSQL. Your Neo4j queries work as-is.
- **pgvector** – Native vector embeddings with HNSW indexes.
- **RLS** – Row-Level Security for multi-tenancy.

**Why it matters:**

1. **Single ACID boundary** – Graph and vector operations in one transaction
2. **No sync issues** – No eventual consistency between systems
3. **Cost reduction** – PostgreSQL vs $16k/month for Neo4j + Pinecone

**Cypher in PostgreSQL:**

```sql
SELECT * FROM cypher('my_graph', $$
  MATCH (a:Entity {name: 'PostgreSQL'})-[*1..3]->(b)
  RETURN a, b
$$) AS (a agtype, b agtype);
```

Variable-length paths (`[*1..3]`) work exactly like Neo4j. Multi-hop reasoning in a single query.

**Hybrid queries (graph + vector):**

```sql
WITH similar AS (
  SELECT id FROM entities
  WHERE embedding <=> query_vector < 0.3
)
SELECT * FROM cypher('graph', $$
  MATCH (e)-[r]->(related)
  WHERE e.id IN $similar_ids
  RETURN e, related
$$);
```

**Benchmarks:**

| Query Type  | EdgeQuake | Neo4j + Pinecone |
| ----------- | --------- | ---------------- |
| Vector only | ~50ms     | ~30ms (Pinecone) |
| Graph only  | ~150ms    | ~100ms (Neo4j)   |
| Combined    | ~200ms    | ~300ms+          |

The specialized databases are faster individually, but combined queries win because there's no network hop.

**Trade-offs:**

- Apache AGE is less mature than Neo4j
- pgvector is slower than Pinecone for massive vector counts (10M+)
- Cypher dialect has minor differences

For most production workloads under 1M vectors, the PostgreSQL stack is sufficient.

**Getting started:**

```bash
git clone https://github.com/raphaelmansuy/edgequake
docker-compose up -d  # PostgreSQL + AGE + pgvector
cargo run --example production_pipeline
```

Would love feedback from anyone running Graph-RAG in production. What's your database stack look like?

---

## HN Comment Preparation

**Expected questions:**

Q: How does AGE compare to Neo4j for complex Cypher queries?
A: 90% compatible. Variable-length paths work. Some procedural extensions differ. Migration is mostly find-replace on function names.

Q: Why not just use Neo4j embedded?
A: Neo4j Community Edition has GPL license. Enterprise is expensive. AGE is Apache 2.0 and runs inside your existing PostgreSQL.

Q: What about performance at scale?
A: AGE handles millions of nodes. For 10M+ vectors, you might want dedicated vector DB. But 90% of production workloads are under 1M.

Q: Is Apache AGE production-ready?
A: Used by Alibaba, Huawei, and others in production. Active development, regular releases.
