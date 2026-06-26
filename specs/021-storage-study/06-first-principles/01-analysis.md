# 01 — First Principles Analysis (Revised)

> **Spec**: 021-storage-study  
> **File**: 06-first-principles/01-analysis.md  
> **Date**: 2026-06-25 (revised 2026-06-25)  
> **Battle-tested against**: migrations 001-038, merger/entity.rs, deletion.rs,  
> scan_ops.rs, query strategies, KnowledgeGraphMerger

---

## The Core Purpose

At its core, EdgeQuake must answer one question efficiently:

> **Given a natural language query, retrieve the most relevant context from a
> large corpus of documents, and generate an accurate answer.**

All storage exists to serve this function. Everything else — multi-tenancy,
RLS, PDF handling, audit logs — is operational scaffolding.

---

## First Principle: What Data Must Be Stored?

### Minimal set for RAG

| Data                           | Why Needed                  | Minimum Store |
| ------------------------------ | --------------------------- | ------------- |
| Chunk text                     | Context for LLM generation  | 1 store       |
| Chunk embeddings               | Vector similarity search    | 1 store       |
| Entity names + descriptions    | Knowledge graph context     | 1 store       |
| Relationships                  | Graph traversal             | 1 store       |
| Entity/relationship embeddings | Mode-specific vector search | 1 store       |

**Minimum: 3 conceptually distinct stores** — text, vectors, graph.  
Current implementation: 5 (relational + KV + vectors + graph + stats counters).

### What is superfluous?

1. `entities` / `relationships` tables: Contain no data. Remove.
2. `chunks.embedding` / `entities.embedding` columns: Never written. Remove.
3. Duplicate document metadata in KV + `documents` table: Choose one.

---

## First Principle: What is the Source of Truth?

### Current (implicit, from code behavior)

| Domain             | Source of Truth   | Derived/Cached                |
| ------------------ | ----------------- | ----------------------------- |
| Document lifecycle | `documents` table | —                             |
| Chunk text         | `eq_*_kv`         | `chunks.content` (unused)     |
| Chunk embeddings   | `eq_*_vectors`    | `chunks.embedding` (NULL)     |
| Entity data        | AGE graph `Node`  | `entities` table (empty)      |
| Relationship data  | AGE graph `EDGE`  | `relationships` table (empty) |
| Entity embeddings  | `eq_*_vectors`    | `entities.embedding` (NULL)   |

### Desired (explicit, single source per domain)

```
Domain                Source of Truth     Access Path
------                ---------------     -----------
Document lifecycle    documents           SQL SELECT
Chunk text            eq_*_kv             KVStorage.get_by_id()
Chunk embeddings      eq_*_vectors        VectorStorage.query()
Entity data           AGE Node            GraphStorage.get_node()
Relationship data     AGE EDGE            GraphStorage.get_edge()
Entity embeddings     eq_*_vectors        VectorStorage.query_filtered(type=entity)
PDF raw bytes         pdf_documents       PdfDocumentStorage
Conversations         conversations       ConversationStorage
Workspace config      workspaces          WorkspaceService
```

---

## First Principle: What Does a Query Actually Read?

For the most common query mode (Hybrid), the read path is:

```
1. eq_*_kv (keyword cache lookup)
2. eq_*_vectors (entity ANN search)
3. eq_*_vectors (relationship ANN search)
4. AGE graph (get_node for each entity)
5. AGE graph (get_node_edges for each entity)
6. eq_*_kv (chunk text by source_id)
7. conversations + messages (history, if enabled)
```

**Three systems, 7 round trips** for a single query in the common case.

The relational `entities`, `relationships`, `chunks` tables are **never read
in the query path**. They are dead weight.

---

## First Principle: What Are the Real Storage Invariants?

These invariants MUST hold for the system to be correct:

```
INV-01: For every chunk vector in eq_*_vectors with type="chunk",
        there exists a KV entry with key "{doc_id}-chunk-{n}"

INV-02: For every entity vector in eq_*_vectors with type="entity",
        there exists an AGE Node with node_id = vector.id

INV-03: For every relationship vector in eq_*_vectors with type="relationship",
        there exists an AGE EDGE with source_id = vector.metadata.source
        and target_id = vector.metadata.target

INV-04: For every AGE Node, all source_ids point to valid KV chunk keys

INV-05: For every document with status="indexed",
        there exists at least one KV chunk key prefixed with "{doc_id}-"
```

**None of these invariants are currently enforced** by foreign keys, triggers,
or automated checks. They are only guaranteed by the application-level SAGA.

---

## First Principle: What Would an Ideal Storage Architecture Look Like?

### Option A: Unified PostgreSQL (pure relational + pgvector)

```
documents     (lifecycle, metadata)
chunks        (text, doc_id FK)   → replaces eq_*_kv for chunk text
entities      (name, type, desc)  → replaces AGE graph (flat, no traversal)
relationships (src FK, tgt FK)    → replaces AGE graph
embeddings    (id, type, vector, entity_id FK | chunk_id FK)  → replaces eq_*_vectors
```

**Pro**: ACID, FK constraints enforce INV-01 through INV-05, single codebase  
**Con**: Graph traversal requires recursive CTEs (slow for deep graphs), no Cypher

### Option B: Current Architecture (fixed)

Keep KV + Vector + AGE, but:
- Remove orphaned tables/columns (DRY fixes)
- Add cross-store invariant checks (CONS fixes)
- Formalize key schemas (DRY-03 fix)
- Apply ISP decomposition to GraphStorage (SOLID-01 fix)

**Pro**: Optimized for each use case, scalable  
**Con**: Cross-store consistency requires ongoing vigilance

### Option C: Single Vector Database (Qdrant/Weaviate)

```
Qdrant collection "edgequake"
  - Chunk points: {vector, payload: {type:"chunk", text, doc_id}}
  - Entity points: {vector, payload: {type:"entity", name, description, edges:[]}}
```

**Pro**: One system  
**Con**: Graph traversal impossible, loses multi-hop reasoning

---

## Verdict

**Option B (current architecture, fixed) is the right path.**

The three-tier architecture (KV + Vector + Graph) is well-suited to EdgeQuake's
retrieval requirements. The problem is not the architecture — it is the
accumulated debt from evolutionary development:

- Legacy tables never cleaned up
- Key schemas never formalized
- Trait boundaries never fully enforced

The improvement plan in [02-improvement-plan.md](02-improvement-plan.md) addresses
these issues in a prioritized, low-risk way.
