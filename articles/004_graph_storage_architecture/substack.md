# The One-Database RAG Stack

_How PostgreSQL extensions eliminated our $16k/month database nightmare_

---

Dear Reader,

Last month I got an email from a friend at a Series B startup. Subject line: "Our AI infrastructure bill is killing us."

They'd built a knowledge retrieval system for their enterprise product. Neo4j for relationships. Pinecone for embeddings. PostgreSQL for user data. It was architecturally beautiful.

The bill was not.

**$23,000/month** — and they were still pre-revenue on this feature.

This week, I want to share what we learned building EdgeQuake, and why we made a controversial decision: **run everything on PostgreSQL**.

---

## The Multi-Database Trap

Here's what most AI teams build when they hear "we need RAG with knowledge graphs":

```
┌─────────────────────────────────────────────────────────────────┐
│                    THE STANDARD STACK                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│      Neo4j              Pinecone            PostgreSQL           │
│      (Graph)            (Vectors)           (Everything Else)    │
│                                                                   │
│      $10k/mo            $5-15k/mo           $1-3k/mo             │
│                                                                   │
│                    Sync? Eventually.                             │
│                    Consistent? Sometimes.                        │
│                    3 AM pages? Definitely.                       │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

It's not that these are bad databases. Neo4j is excellent. Pinecone is fast. But when you need to query across all three in a single user request, things get complicated.

User asks: _"What's connected to X that's also similar to Y?"_

Your system:

1. Query Pinecone for similar vectors
2. Take those IDs to Neo4j
3. Traverse the graph
4. Join with PostgreSQL for metadata
5. Hope nothing timed out

Each hop is a network call. Each service can fail independently. And the bill keeps growing.

---

## The PostgreSQL Revelation

I'd been dismissing PostgreSQL as "just a relational database" for years. Then I discovered what had happened while I wasn't looking.

**Apache AGE** adds Cypher query language — the same syntax Neo4j uses. You can literally copy your Neo4j queries and run them in PostgreSQL.

**pgvector** adds vector similarity search. HNSW indexes, cosine similarity, the works. It's not Pinecone-fast at 100M vectors, but for most workloads? It's plenty.

**Row-Level Security** handles multi-tenancy at the database level. No application code needed.

Suddenly, one database could do everything.

---

## What This Actually Looks Like

Here's a query that would require three databases in the traditional stack:

```sql
-- Find entities similar to the query, then traverse their relationships
WITH similar_entities AS (
    SELECT id, name,
           embedding <=> $query_vector AS distance
    FROM entity_embeddings
    WHERE embedding <=> $query_vector < 0.3
    ORDER BY distance
    LIMIT 10
)
SELECT * FROM cypher('knowledge_graph', $$
    MATCH (e:Entity)-[r*1..2]->(related:Entity)
    WHERE e.id IN $entity_ids
    RETURN e.name, type(r), related.name, related.description
$$) AS (source text, relation text, target text, description text);
```

One query. One transaction. One database.

If you're used to Neo4j, the Cypher part looks familiar. That's the point — Apache AGE implemented Cypher specifically so migrations would be straightforward.

---

## The Business Case

Let me show you the math my friend's startup did:

**Before (Traditional Stack)**

- Neo4j Aura Enterprise: $10,000/month
- Pinecone Standard: $5,000/month
- RDS PostgreSQL: $2,000/month
- DevOps for sync monitoring: $3,000/month (time)
- **Total: ~$20,000/month**

**After (PostgreSQL Stack)**

- RDS PostgreSQL (larger instance): $800/month
- AGE extension: $0
- pgvector extension: $0
- Simplified ops: -$2,000/month (time saved)
- **Total: ~$800/month**

That's **96% cost reduction**. Not 10%. Not 50%. Ninety-six percent.

Now, there are trade-offs. Neo4j's query optimizer is more mature. Pinecone is faster for billion-vector workloads. But for 90% of production use cases? PostgreSQL is enough.

---

## What We Built: EdgeQuake

EdgeQuake is our open-source implementation of this architecture. It implements the LightRAG algorithm (arXiv:2410.05779) for Graph-RAG entirely on PostgreSQL.

```
┌─────────────────────────────────────────────────────────────────┐
│                    EDGEQUAKE ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   ┌───────────────────────────────────────────────────────────┐│
│   │                      PostgreSQL 15+                         ││
│   ├───────────────────────────────────────────────────────────┤│
│   │                                                             ││
│   │   Apache AGE        pgvector          JSONB        RLS    ││
│   │   (Cypher)          (HNSW)            (KV)         (MT)   ││
│   │                                                             ││
│   │   • Graph nodes     • Embeddings      • Config     • Tenant││
│   │   • Relationships   • Similarity      • Metadata   • Isolate│
│   │   • Traversal       • Search          • Cache              ││
│   │                                                             ││
│   └───────────────────────────────────────────────────────────┘│
│                                                                   │
│   One database. One backup. One bill.                           │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

The storage layer is pluggable — you can swap in Neo4j if you need it — but the default PostgreSQL implementation handles everything most teams need.

---

## The Honest Trade-offs

I don't want to oversell this. Here's where PostgreSQL falls short:

1. **Massive vector scale**: Above 10M vectors, Pinecone is noticeably faster
2. **Complex graph algorithms**: Neo4j's GDS library has more built-in algorithms
3. **Maturity**: AGE is newer, has fewer Stack Overflow answers

But here's the thing: most production RAG systems have <1M vectors and relatively simple graph queries. The PostgreSQL stack is _good enough_ for 90% of use cases.

And "good enough" at $800/month beats "perfect" at $20,000/month.

---

## Getting Started

If you want to try this architecture:

```bash
# Clone EdgeQuake
git clone https://github.com/raphaelmansuy/edgequake

# Start PostgreSQL with extensions
docker-compose up -d

# Run the example
cargo run --example production_pipeline
```

The stack is:

- PostgreSQL 15+
- Apache AGE extension
- pgvector extension
- EdgeQuake (Rust) for the RAG pipeline

---

## What's Next

This is the fourth in a series about Graph-RAG architecture. Coming up:

- **Query Modes**: How EdgeQuake's five query strategies balance speed and depth
- **Rust Performance**: Why we chose Rust for production RAG
- **Entity Extraction**: The tuple-parsing trick that eliminated JSON failures

If you found this useful, the best thing you can do is **share it with someone paying too much for their RAG infrastructure**.

Until next week,

_Raphael_

---

_EdgeQuake is open source: [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)_

_Special thanks to the Apache AGE and pgvector teams for building these incredible extensions._

_LightRAG paper: arXiv:2410.05779_
