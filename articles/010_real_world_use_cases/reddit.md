# How we're using knowledge graphs for contract analysis (open source)

**Subreddit**: r/legaltech, r/MachineLearning, r/LocalLLaMA

---

## Post Title

How we built a Graph-RAG system for contract analysis (and what we learned)

---

## Post Body

Hey everyone,

I've been working on a RAG system specifically for legal document analysis and wanted to share some learnings. This isn't a product pitch—genuinely interested in how others are solving similar problems.

### The Problem We Hit

Our initial RAG setup used vector embeddings (like everyone else). Worked great for simple queries:

> "Find contracts mentioning indemnification"

But fell apart for relationship queries:

> "Find contracts where Party A has unlimited liability AND termination allows exit without cause"

Vector similarity can't cross-reference conditions across clauses. We needed to understand _structure_, not just words.

### The Solution: Knowledge Graphs

We built a system that extracts entities and relationships during ingestion:

```
CONTRACT_2024_001
    ├── involves → PARTY_A (Licensor)
    ├── involves → PARTY_B (Licensee)
    ├── has_clause → INDEMNIFICATION_UNLIMITED
    ├── has_clause → TERMINATION_30_DAYS_COC
    └── has_clause → CHANGE_OF_CONTROL_TRIGGER
```

Now we can query:

> "Find contracts where has*clause=INDEMNIFICATION_UNLIMITED AND has_clause=TERMINATION*\*\_COC"

### What Worked

**1. Entity normalization matters**

"Acme Corp", "ACME Corporation", and "Acme" should be the same node. We normalize to `ACME_CORP` with deterministic rules:

- Remove common suffixes (Inc, LLC, Corp)
- Uppercase
- Replace spaces with underscores

This alone reduced duplicate nodes by 40%.

**2. PostgreSQL + Apache AGE is surprisingly good**

We evaluated Neo4j, but the operational overhead for our team was too high. PostgreSQL with the AGE extension gives us:

- Cypher queries
- Same database for relational data
- Existing backup/monitoring tooling

**3. Hybrid retrieval is essential**

Graph-only missed semantically similar but differently named entities. We combine:

- Graph traversal (structural relationships)
- Vector similarity (semantic matching)

Best of both worlds.

### What Didn't Work (Initially)

**1. Free-form entity extraction**

Early versions let the LLM extract any entity types. Result: chaos. "CLAUSE_12" and "SECTION_12" as different entities.

Solution: Schema-guided extraction. We define allowed entity types (PARTY, CLAUSE_TYPE, TERM_DURATION, etc.) and the LLM conforms.

**2. One-shot extraction**

Single-pass extraction missed entities. The LightRAG paper calls this "gleaning"—multiple extraction passes on the same chunk.

We do 2 passes by default. First pass gets the obvious entities. Second pass catches the subtle ones.

### Numbers

On a corpus of 10,000 contracts:

| Metric                            | Vector RAG | Our Graph-RAG |
| --------------------------------- | ---------- | ------------- |
| Multi-hop queries                 | ❌ Failed  | ✅ Worked     |
| Precision on relationship queries | 23%        | 89%           |
| Indexing time                     | 2 hours    | 6 hours       |
| Indexing cost (gpt-4o-mini)       | $2         | $14           |
| Query latency                     | 50ms       | 200ms         |

The trade-off is clear: 3x higher indexing cost for dramatically better relationship queries.

### For the LocalLLaMA crowd

We integrated Ollama, so you can run everything on-premise:

```bash
ollama pull llama3:8b
export LLM_PROVIDER=ollama
```

For law firms with confidentiality requirements, this is huge. Client data never leaves the network.

### The Code

It's open source (Apache 2.0):
https://github.com/raphaelmansuy/edgequake

Written in Rust. Uses PostgreSQL + Apache AGE for graph storage, pgvector for embeddings.

---

### Questions for the community

1. How are you handling multi-hop queries in your RAG systems?
2. Anyone doing domain-specific entity extraction for legal/contracts?
3. For those using local models—how does extraction quality compare to GPT-4?

Would love to hear about your approaches.

---

_Based on the LightRAG algorithm (arXiv:2410.05779). Credit to the original researchers._
