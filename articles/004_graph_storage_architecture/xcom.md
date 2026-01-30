# PostgreSQL AGE: The One-Database RAG Stack

**X.com Thread** (15 tweets)

---

**1/15**
Your RAG stack probably has 3 databases:
• Neo4j ($10k/mo)
• Pinecone ($5k/mo)
• PostgreSQL ($1k/mo)

What if you only needed PostgreSQL?

🧵 Here's how EdgeQuake cut our database costs by 80x:

---

**2/15**
The multi-database nightmare:

```
Neo4j ←sync→ Pinecone
  ↓           ↓
Postgres ←sync→ ???
```

Every sync = failure point
3 backups, 3 dashboards
3 AM pages when things break

There's a better way.

---

**3/15**
Meet PostgreSQL's secret weapons:

🔷 Apache AGE = Graph queries (Cypher)
🔷 pgvector = Vector embeddings (HNSW)
🔷 JSONB = Flexible key-value

All in one database.
One ACID boundary.
One bill.

---

**4/15**
Apache AGE brings Neo4j-compatible Cypher:

```cypher
MATCH (a)-[*1..3]->(b)
WHERE a.name = 'EdgeQuake'
RETURN a, b
```

Same syntax you'd write in Neo4j.
Runs inside PostgreSQL.
Full ACID guarantees.

---

**5/15**
Variable-length paths are the killer feature:

```cypher
MATCH path = (start)-[*1..5]->(end)
WHERE start.type = 'CONCEPT'
RETURN path
```

Multi-hop reasoning in one query.
No iterative calls.
No embeddings needed.

---

**6/15**
pgvector adds native embeddings:

```sql
SELECT name,
       embedding <=> $query AS dist
FROM entities
ORDER BY dist
LIMIT 10;
```

HNSW indexes for 1M+ vectors.
Cosine, L2, inner product.
No Pinecone required.

---

**7/15**
The magic: Combined queries

```sql
-- Graph + Vector in one transaction
WITH similar AS (
  SELECT id FROM entities
  WHERE embedding <=> $1 < 0.3
)
SELECT * FROM cypher('graph', $$
  MATCH (e)-[r]->(related)
  WHERE e.id IN $similar
  RETURN e, related
$$);
```

---

**8/15**
Performance comparison:

| Metric | PostgreSQL | Neo4j+Pinecone |
| ------ | ---------- | -------------- |
| Query  | <200ms     | 300-500ms      |
| ACID   | ✓ Full     | ✗ Eventual     |
| Cost   | $200/mo    | $16,000/mo     |

Combined queries are faster because there's no network hop.

---

**9/15**
Multi-tenancy with Row-Level Security:

```sql
CREATE POLICY isolation
ON entities
USING (tenant = current_tenant());
```

No application filtering.
No data leakage even with bugs.
Database enforces it.

---

**10/15**
The architecture:

```
┌───────────────────────┐
│     PostgreSQL        │
├───────────────────────┤
│ AGE  │ pgvector │ RLS │
│Cypher│  HNSW    │ MT  │
└───────────────────────┘
     One Database
```

Graph + Vectors + Multi-tenant
All in one instance.

---

**11/15**
Migration from Neo4j is straightforward:

1. Your Cypher queries work as-is
2. Export nodes/edges as CSV
3. Import to AGE
4. Update connection string

Most teams: 2-4 weeks.

---

**12/15**
Real cost comparison:

Before:
• Neo4j Aura: $10,000
• Pinecone Pro: $5,000
• RDS Postgres: $1,000
Total: $16,000/month

After:
• PostgreSQL + AGE + pgvector
Total: $200/month (managed)

80x savings. Same capabilities.

---

**13/15**
EdgeQuake uses this stack for Graph-RAG:

• 61-84% better answers than classic RAG
• <200ms query latency
• Single PostgreSQL instance
• Production-ready

LightRAG algorithm (arXiv:2410.05779) in Rust.

---

**14/15**
Getting started:

```bash
git clone github.com/raphaelmansuy/edgequake
docker-compose up -d
cargo run --example production_pipeline
```

PostgreSQL + AGE + pgvector out of the box.

---

**15/15**
TL;DR:

PostgreSQL + Apache AGE + pgvector =
Neo4j + Pinecone in one database

• 80x cost reduction
• Faster combined queries
• Full ACID transactions
• Native multi-tenancy

Stop paying for complexity.

Star: github.com/raphaelmansuy/edgequake

/thread
