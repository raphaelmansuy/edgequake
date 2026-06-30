# SPEC-034-005: Complexity Model — O(N) Analysis

> **Lens**: Algorithm Expert / O(N) Specialist  
> **Version**: 1.0.0 — 2026-06-30

---

## 1. Notation

```
N   = number of entities extracted from the current document
E   = number of edges (relationships) extracted from the current document
G   = current number of nodes in the graph (52,915 at measurement time)
Gₑ  = current number of edges in the graph (67,636)
V   = current number of vectors (5,898 chunk vectors)
K   = number of indexes on the Node table
M   = HNSW m parameter (16)
D   = vector dimensions (1024)
```

---

## 2. Current Complexity Profile

### 2.1 Node Upsert Path (Cypher MERGE)

```
For each entity i in [1..N]:
  cypher_execute(MERGE (n:Node {node_id: entity_id}))
  └── AGE compiles to: Bitmap Index Scan on GIN idx_node_props_gin
      Cost = O(G) for GIN containment lookup
            + O(K) for writing K indexes on insert/update
            + O(1) network round-trip overhead

Total node write cost:
  T_nodes = N × [O(G) + O(K)]
           = O(N × G × K)          ← QUADRATIC in graph size!

With N=200, G=52915, K=18:
  T_nodes ∝ 200 × 52915 × 18 = 190,494,000 units
```

**Why this is quadratic**: as G grows (more documents ingested), each new
document's storage cost grows linearly with G. This is the definition of O(N×G).

### 2.2 UNWIND MERGE — Does Batching Help?

```
UNWIND batch of B entities:
  cypher_execute(UNWIND [...B items...] MERGE ...)
  └── AGE expands to: Nested Loop × B
      Each loop iteration: Bitmap Index Scan on GIN (O(G) cost)

Total cost = N/B × [B × O(G)] = N × O(G) — batch size B cancels out!

Conclusion: UNWIND MERGE saves N-1 network round-trips but does NOT
            reduce the database-internal O(N × G) scan work.
```

### 2.3 Edge Upsert Path

```
Each MERGE (a)-[r:EDGE]->(b) also matches endpoint nodes:
  MERGE (a:Node {node_id: src}) → O(G) scan
  MERGE (b:Node {node_id: tgt}) → O(G) scan
  MERGE (a)-[r:EDGE {...}]->(b)  → O(Gₑ) edge scan

T_edges = E × [2×O(G) + O(Gₑ)]
        = O(E × G)     ← also quadratic!
```

### 2.4 Vector Insert (HNSW)

```
HNSW INSERT for each vector v:
  Find M nearest neighbors in current V-vector graph = O(M × log V)
  Update M×2 outgoing links per level = O(M × log V)
  Total per insert = O(M × log V)

T_vectors = N_chunks × O(M × log V)
           = O(N_chunks × log V)   ← log-linear ✓ (acceptable)

BUT: at D=1024, each operation involves 1024-dim distance calculations.
The constant factor is ~1024× larger than for D=64.
At V=5898: O(M × log V) = O(16 × 12.5) ≈ 200 operations, each touching 4KB.
T_vectors ≈ 200 × 4KB = 800KB of vector memory touched per insert.
```

### 2.5 KV Upsert

```
KV upsert (key, value):
  INSERT ... ON CONFLICT ... → O(log K_rows) for btree key lookup ✓
  GIN index update on 61KB JSONB value: O(W) where W = word count in text
  
Typical W for 61KB text ≈ 15,000 words → O(15,000) GIN insertions
The GIN on value is O(W) per upsert, completely wasted (value never queried by content)

T_kv = N_chunks × O(log K_rows + W)  ← W dominates
```

---

## 3. Complexity Comparison Table

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  OPERATION         CURRENT          TARGET            SPEEDUP FACTOR        │
│                                                                              │
│  Node existence    O(G)             O(log G)          G/log G ≈ 3,800×     │
│                    ↑ GIN @>         ↑ btree =                               │
│                                                                              │
│  Node upsert       O(G × K)         O(K_min × log G)  G×K/(K_min×log G)   │
│  (per entity)      O(52915 × 18)    O(3 × 16)         = ~20,000×           │
│                                                                              │
│  Batch N nodes     O(N × G × K)     O(N × log G)      N×G×K/(N×log G)     │
│                                     single SQL unnest  ≈ K × G/log G       │
│                                                                              │
│  Edge upsert       O(G + Gₑ)        O(log G + log Gₑ) ~3,500×             │
│                                                                              │
│  Vector insert     O(M × log V)     O(M × log V)       1× (unchanged)     │
│  HNSW 1024-dim    (large constant)  (large constant)   optim: batch load   │
│                                                                              │
│  KV upsert         O(log K + W)     O(log K)           W ≈ 15,000×        │
│                    O(15,000)        (drop GIN)                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Projected Improvements (Conservative Estimates)

```
Document: 200 entities, 500 edges, 50 chunks
Graph state: 52,915 nodes

CURRENT STATE:
  Node upserts:   200 × 5.6ms = 1,120ms
  Edge upserts:   500 × 4.4ms = 2,200ms (2× node lookups per edge)
  Vector upserts: 50 × 3.5ms  =   175ms
  KV upserts:     50 × 2.0ms  =   100ms
  Overhead:                    = 1,000ms (community, serialization)
  ────────────────────────────────────────
  TOTAL ESTIMATED: ~4,600ms (minimum — observed 30-120s includes LLM)

TARGET STATE (after fixes):
  Node upserts:   1 SQL unnest INSERT = ~50ms (batch 200 rows)
  Edge upserts:   1 SQL unnest INSERT = ~80ms (batch 500 rows)
  Vector upserts: 50 × 2.5ms = 125ms (HNSW unchanged, but smaller overhead)
  KV upserts:     50 × 0.5ms =  25ms (no GIN maintenance)
  Overhead:                   = 200ms
  ────────────────────────────────────────
  TOTAL PROJECTED: ~480ms = 10× faster

For 2000-entity document:
  Current estimated minimum:  ~21,600ms
  Target projected:           ~2,000ms = 10× faster
```

---

## 5. Scaling Curve — Current vs Target

```
Processing time for graph storage (excluding LLM extraction)
─────────────────────────────────────────────────────────
Time (s)
  │
 60 │                                           ★ Current (O(N×G))
  │                                         /
 45 │                                     /
  │                                 /
 30 │                           /
  │                       /
 15 │                 /
  │           /
  5 │    / ──────────────────── ● Target (O(N·log G))
  │ /
  └──────────────────────────────────────────────── Entities (N)
   50    200    500   1000  1500  2000
```

---

## 6. When Does the System Hit Limits?

```
G (graph nodes)  | Current: time per document  | Target: time per document
─────────────────┼────────────────────────────────────────────────────────
10,000           | ~20s  (200 entities)         | ~2s
50,000           | ~120s (200 entities)         | ~3s (log grows slowly)
200,000          | ~480s = 8 minutes            | ~4s
1,000,000        | OOM / timeout                | ~6s
─────────────────┼────────────────────────────────────────────────────────
```

The current system becomes unusable at ~200,000 graph nodes.
The target system handles millions of nodes with sub-10s store times.

---

## 7. The Compounding Problem

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  VICIOUS CYCLE                                                              │
│                                                                             │
│  More documents stored                                                      │
│       │                                                                     │
│       ▼                                                                     │
│  Graph grows (G increases)                                                  │
│       │                                                                     │
│       ▼                                                                     │
│  Each new document stores slower (O(G) scans get worse)                    │
│       │                                                                     │
│       ▼                                                                     │
│  Users upload fewer documents (frustration)                                 │
│       │                                                                     │
│       ▼                                                                     │
│  Knowledge graph value degrades (fewer nodes, sparser connections)          │
│                                                                             │
│  BREAK THE CYCLE: Replace O(G) scan with O(log G) btree lookup             │
└─────────────────────────────────────────────────────────────────────────────┘
```
