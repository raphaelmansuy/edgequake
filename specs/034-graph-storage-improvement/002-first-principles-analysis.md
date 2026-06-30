# SPEC-034-002: First Principles Analysis

> **Lens**: Systems Theory / Algorithm Design  
> **Version**: 1.0.0 — 2026-06-30

---

## 1. What Must Be True About Graph Storage

Applying First Principles means asking: *what is the minimum irreducible work
required to store N entities into a graph of G existing nodes?*

### Principle 1 — Existence Check

Every entity must be checked for existence before deciding to INSERT or UPDATE.  
**Minimum cost**: O(log G) per entity using a btree index on the entity key.

### Principle 2 — Write Amplification

Every index on a table increases write cost proportionally.  
**Minimum cost**: O(K · log N) where K = number of active indexes, N = table rows.

### Principle 3 — Vector Index Maintenance

HNSW (Hierarchical Navigable Small World) graphs maintain O(M · log N) links
per node. Each INSERT must update these links.  
**Minimum cost**: O(M · log N) per vector inserted.

### Principle 4 — Batch Independence

If entities are logically independent, they can be merged in parallel or as a
single atomic batch operation.  
**Minimum cost**: O(N/batch_size · batch_cost) — linear in N for a fixed batch.

### Principle 5 — Isolation Overhead

Multi-tenant isolation (per-workspace filtering) adds a constant factor C to
every query, NOT O(N). Proper indexing collapses this to O(log G).

---

## 2. The Ideal Complexity Profile

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  OPERATION          │ IDEAL COMPLEXITY │ CURRENT COMPLEXITY               ║
╠══════════════════════════════════════════════════════════════════════════════╣
║  Node existence     │ O(log G)         │ O(G)  — full GIN scan            ║
║  Node upsert        │ O(log G)         │ O(G)  — GIN + K index writes     ║
║  Edge upsert        │ O(log E)         │ O(E)  — GIN + K index writes     ║
║  Vector insert      │ O(M · log V)     │ O(M · log V) — same, but bloated ║
║  KV upsert          │ O(log K)         │ O(K · GIN) — GIN on full text    ║
║  Batch N entities   │ O(N · log G)     │ O(N × G) — N individual scans    ║
╚══════════════════════════════════════════════════════════════════════════════╝

Where: G = graph nodes, E = edges, V = vectors, K = KV rows, M = HNSW param
```

---

## 3. The Irreducible Lower Bound

For a document with N entities stored into a graph with G nodes:

```
T_min = N × O(log G)                   [existence checks]
      + N × O(K_needed · log G)        [required index writes]
      + N_new × O(M · log G)           [new node allocations]
      + N_edges × O(log E)             [edge upserts]
```

Where K_needed = minimal set of indexes needed for reads (not all 18+).

**Current T_actual** = N × O(G) × K_redundant

This means with 52,915 nodes and 18 indexes:
- Current: N × 52,915 × 18 = **N × 952,470** operations
- Ideal: N × log₂(52,915) × 3 = **N × ~48** operations
- **Theoretical speedup: ~20,000×** (not all achieved in practice, but 10–100× is realistic)

---

## 4. First Principle Violations (Identified)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  FIRST PRINCIPLE VIOLATIONS IN CURRENT SYSTEM                               │
│                                                                             │
│  ❌ FPV-1: AGE Cypher MERGE ignores btree indexes                          │
│     Cypher {key: val} compiles to GIN @> containment, not btree =          │
│     → Violates Principle 1: using O(G) scan instead of O(log G)            │
│                                                                             │
│  ❌ FPV-2: 18 indexes maintained per label (Node, EDGE)                    │
│     Many are duplicates (node_id indexed 3× different ways)                │
│     → Violates Principle 2: K_redundant >> K_needed                        │
│                                                                             │
│  ❌ FPV-3: HNSW at 909 MB for 5,898 vectors (12.3× bloat)                 │
│     ef_construction=64 + m=16 at dim=1024 creates massive index            │
│     → Violates Principle 3: Index too large for bulk load phase            │
│                                                                             │
│  ❌ FPV-4: UNWIND MERGE does N GIN scans, not 1 batch lookup              │
│     AGE expands UNWIND into N separate Cypher merge sub-operations         │
│     → Violates Principle 4: No true batch path                             │
│                                                                             │
│  ❌ FPV-5: KV GIN on full 61 KB JSONB texts (no query benefit)            │
│     KV values are chunk text bodies — GIN is never used to search them     │
│     → Violates Principle 2: K_unnecessary                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. What a Correct Design Looks Like

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  TARGET DESIGN                                                              │
│                                                                             │
│  AGE Node Upsert:                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Native SQL path:                                                   │   │
│  │  INSERT INTO "graph"."Node" (id, properties)                        │   │
│  │  SELECT ... FROM unnest($node_ids) AS ...                           │   │
│  │  ON CONFLICT (node_id_btree_expr) DO UPDATE SET ...                 │   │
│  │  → O(N · log G) — one pass, btree index, no Cypher overhead        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  pgvector Batch:                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  COPY or multi-row INSERT, then rebuild HNSW index                  │   │
│  │  → O(N_total · log N_total) once, not O(N_per_insert) × N_docs     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Index Set (Node):                                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  1× btree on node_id text expression        (exact match)           │   │
│  │  1× btree on (workspace_id, node_id)        (tenant isolation)      │   │
│  │  1× GIN on properties                       (full-text search only) │   │
│  │  — drop: 15 duplicate/redundant indexes —                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```
