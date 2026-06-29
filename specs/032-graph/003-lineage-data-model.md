# SPEC-032-003: Lineage Data Model

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** `FEAT0011` `FEAT0019` `FEAT0020` `BR0007` `BR0019` `SPEC-021`  
**See also:** `edgequake-pipeline/src/lineage.rs` (current in-memory types)

---

## 1. First Principles — What Lineage Must Answer

```
Q1: "Which PDF page(s) did this entity come from?"
Q2: "What chunk(s) contributed to this entity's description?"
Q3: "If I delete document X, which entities should be cleaned up?"
Q4: "Which entities are shared between document X and document Y?"
Q5: "What was the original text that produced this vector embedding?"
Q6: "Show me entity E's description evolution across merge events."
```

Current system answers Q3 partially (source_id pipe-sep string), Q5 via KV
store (chunk text by key), Q1/Q2/Q4/Q6 not at all.

---

## 2. Target Lineage Data Model

### 2.1 Entity-Relationship Diagram

```
┌─────────────────────┐         ┌──────────────────────────────┐
│   pdf_documents      │ 1     N │   pdf_pages                  │
│  ─────────────────── │────────►│  ──────────────────────────  │
│  id UUID PK          │         │  id UUID PK                  │
│  workspace_id        │         │  document_id FK              │
│  tenant_id           │         │  page_number INT             │
│  status              │         │  char_start INT              │
│  ...                 │         │  char_end INT                │
└─────────────────────┘         └──────────────────────────────┘
          │ 1                              │ 1
          │ N                              │ N
          ▼                               ▼
┌─────────────────────────────────────────────────────────────────┐
│   chunks                                                         │
│  ─────────────────────────────────────────────────────────────  │
│  id TEXT PK  (= "{doc_id}-chunk-{index}")                       │
│  document_id UUID FK → pdf_documents                            │
│  chunk_index INT                                                │
│  content TEXT                                                   │
│  tokens INT                                                     │
│  char_start INT       ← NEW: byte offset in markdown           │
│  char_end INT         ← NEW: byte offset in markdown           │
│  page_start INT       ← NEW: first PDF page (1-indexed)        │
│  page_end INT         ← NEW: last PDF page (1-indexed)         │
│  embedding_id TEXT    ← NEW: FK to vector table id             │
│  tenant_id TEXT                                                 │
│  workspace_id TEXT                                              │
│  metadata JSONB                                                 │
└──────────────────────────────┬──────────────────────────────────┘
                               │ N:M
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│   chunk_entity_links  (NEW — replaces source_chunk_ids TEXT[])  │
│  ─────────────────────────────────────────────────────────────  │
│  chunk_id TEXT FK → chunks.id                                   │
│  entity_name TEXT       (UPPERCASE_UNDERSCORED)                 │
│  workspace_id TEXT                                              │
│  created_at TIMESTAMPTZ NOT NULL DEFAULT now()                  │
│  PRIMARY KEY (chunk_id, entity_name, workspace_id)              │
└─────────────────────────────────────────────────────────────────┘
                               │
                               │ N:1
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│   entities  (CQRS read model — migration 039)                    │
│  ─────────────────────────────────────────────────────────────  │
│  id UUID PK                                                     │
│  name TEXT UNIQUE (workspace-scoped)                            │
│  entity_type TEXT                                               │
│  description TEXT                                               │
│  tenant_id TEXT                                                 │
│  workspace_id TEXT                                              │
│  source_chunk_ids TEXT[]  (kept for backwards compat)           │
│  keywords TEXT[]                                                │
│  tsv tsvector GENERATED                                         │
│  description_history JSONB ← NEW: [{ts, desc, chunk_id}]       │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│   chunk_relation_links  (NEW — mirrors chunk_entity_links)      │
│  ─────────────────────────────────────────────────────────────  │
│  chunk_id TEXT FK → chunks.id                                   │
│  source_entity TEXT  (UPPERCASE_UNDERSCORED)                    │
│  target_entity TEXT  (UPPERCASE_UNDERSCORED)                    │
│  workspace_id TEXT                                              │
│  created_at TIMESTAMPTZ NOT NULL DEFAULT now()                  │
│  PRIMARY KEY (chunk_id, source_entity, target_entity, workspace_id) │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 AGE Graph Node — Updated Properties

```
:Node {
  node_id:       TEXT  (UPPERCASE_UNDERSCORED)     -- MERGE key
  entity_type:   TEXT
  description:   TEXT
  source_id:     TEXT  (pipe-sep chunk ids)        -- KEPT (backwards compat)
  source_chunks: TEXT[]  ← NEW (AGE agtype array)  -- for GIN containment
  tenant_id:     TEXT
  workspace_id:  TEXT
  keywords:      TEXT[]
  page_refs:     TEXT[]  ← NEW: ["doc_id:page_N", ...]
}

:EDGE {
  source_id:     TEXT
  target_id:     TEXT
  weight:        FLOAT
  relation_type: TEXT
  description:   TEXT
  keywords:      TEXT[]
  source_chunks: TEXT[]  ← NEW (same pattern as nodes)
  page_refs:     TEXT[]  ← NEW
}
```

---

## 3. Lineage Use Cases — Mapping to Implementation

### UC-L1: "Which pages does entity E come from?"

```sql
-- Via chunk_entity_links + chunks:
SELECT DISTINCT c.page_start, c.page_end, c.document_id
FROM chunk_entity_links cel
JOIN chunks c ON c.id = cel.chunk_id
WHERE cel.entity_name = 'ENTITY_NAME'
  AND cel.workspace_id = $1;

-- Response time target: < 50ms with index on (entity_name, workspace_id)
```

### UC-L2: "Delete document X — which entities become orphaned?"

```sql
-- Find entities whose ONLY source chunks are from document X:
WITH doc_chunks AS (
  SELECT id FROM chunks WHERE document_id = $doc_id
),
entity_source_counts AS (
  SELECT
    cel.entity_name,
    COUNT(*) FILTER (WHERE cel.chunk_id IN (SELECT id FROM doc_chunks)) AS doc_count,
    COUNT(*) AS total_count
  FROM chunk_entity_links cel
  WHERE cel.workspace_id = $workspace_id
  GROUP BY cel.entity_name
)
SELECT entity_name
FROM entity_source_counts
WHERE doc_count = total_count;  -- only sourced from this document
```

### UC-L3: "Cross-document entity — show all contributing documents"

```sql
SELECT DISTINCT pd.id, pd.filename, c.page_start, c.page_end
FROM chunk_entity_links cel
JOIN chunks c ON c.id = cel.chunk_id
JOIN pdf_documents pd ON pd.id = c.document_id
WHERE cel.entity_name = 'ENTITY_NAME'
  AND cel.workspace_id = $1
ORDER BY pd.id, c.page_start;
```

### UC-L4: "Show entity description evolution"

```sql
-- Via entities.description_history JSONB:
SELECT
  (elem->>'ts')::timestamptz AS merged_at,
  elem->>'chunk_id' AS from_chunk,
  elem->>'description' AS description_at_merge
FROM entities e,
     jsonb_array_elements(e.description_history) elem
WHERE e.name = 'ENTITY_NAME' AND e.workspace_id = $1
ORDER BY (elem->>'ts')::timestamptz;
```

---

## 4. Cross-Document Entity Lineage — Merge Strategy

When entity E appears in document A (chunks A1, A2) and later document B (chunk B1):

```
Before document B ingestion:
  Node(E).source_id = "A1|A2"
  Node(E).source_chunks = ["A1", "A2"]
  chunk_entity_links: [(A1, E), (A2, E)]

After document B ingestion (merge):
  Node(E).source_id = "A1|A2|B1"          ← APPEND (BR0007: append-only)
  Node(E).source_chunks = ["A1", "A2", "B1"]
  chunk_entity_links: [(A1, E), (A2, E), (B1, E)]  ← new row added
  entities.description_history appended
```

**DRY principle:** The append logic lives in ONE place: `merger/entity.rs:update_entity_node()`.  
The relational sink `upsert_entity()` receives `source_chunk_ids` and must UNION, not replace.

### Cross-Document Relation Lineage

Relations can link entities from different documents:

```
Document A extracts: ALICE --KNOWS--> BOB  (chunk A3)
Document B extracts: ALICE --WORKS_WITH--> BOB  (chunk B2)

After merge:
  EDGE(ALICE→BOB) is merged (same source/target):
    source_chunks = ["A3", "B2"]
    relation_type = "KNOWS"  (or most recent, or LLM-summarized)
    description merged

  chunk_relation_links: [(A3, ALICE, BOB), (B2, ALICE, BOB)]
```

---

## 5. Embedding Lineage

Each vector in `eq_*_vectors` must carry metadata that traces back to its source:

```jsonb
-- Chunk vector metadata (already partially correct):
{
  "type": "chunk",
  "document_id": "uuid-...",
  "chunk_id": "uuid-...-chunk-7",
  "chunk_index": 7,
  "page_start": 3,        ← NEW
  "page_end": 4,          ← NEW
  "start_offset": 1024,   ← NEW (char offset in markdown)
  "end_offset": 2048,     ← NEW
  "workspace_id": "...",
  "tenant_id": "..."
}

-- Entity vector metadata (currently sparse):
{
  "type": "entity",
  "name": "ALICE",
  "entity_type": "PERSON",
  "node_id": "ALICE",
  "source_chunk_ids": ["...-chunk-3", "...-chunk-7"],  ← NEW: all sources
  "workspace_id": "...",
  "tenant_id": "..."
}
```

---

## 6. Migration Plan for Lineage Tables

```
Migration 066: Add page span columns to chunks
  ALTER TABLE chunks
    ADD COLUMN IF NOT EXISTS char_start  INT,
    ADD COLUMN IF NOT EXISTS char_end    INT,
    ADD COLUMN IF NOT EXISTS page_start  INT,
    ADD COLUMN IF NOT EXISTS page_end    INT,
    ADD COLUMN IF NOT EXISTS embedding_id TEXT;

Migration 067: Add chunk_entity_links and chunk_relation_links
  CREATE TABLE IF NOT EXISTS chunk_entity_links (
    chunk_id     TEXT NOT NULL,
    entity_name  TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chunk_id, entity_name, workspace_id)
  );
  CREATE INDEX ON chunk_entity_links (entity_name, workspace_id);
  CREATE INDEX ON chunk_entity_links (chunk_id);

  CREATE TABLE IF NOT EXISTS chunk_relation_links (
    chunk_id      TEXT NOT NULL,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    workspace_id  TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chunk_id, source_entity, target_entity, workspace_id)
  );
  CREATE INDEX ON chunk_relation_links (source_entity, workspace_id);
  CREATE INDEX ON chunk_relation_links (target_entity, workspace_id);

Migration 068: Add description_history to entities
  ALTER TABLE entities
    ADD COLUMN IF NOT EXISTS description_history JSONB NOT NULL DEFAULT '[]'::jsonb;
  CREATE INDEX ON entities USING GIN (description_history jsonb_path_ops);
```

---

## 7. Lineage Preservation: SOLID Principles

| Principle                     | Application                                                                               |
| ----------------------------- | ----------------------------------------------------------------------------------------- |
| **S** — Single Responsibility | `ChunkLineage` struct owns span data; merger owns merge history; CQRS sink owns DB writes |
| **O** — Open/Closed           | `RelationalEntitySink` trait is extended, not modified, to accept `source_spans`          |
| **L** — Liskov                | All sinks (`NoopEntitySink`, `PostgresEntitySink`) honour the append-only contract        |
| **I** — Interface Segregation | Separate `LineageSink` trait from `RelationalEntitySink` to avoid bloating existing impls |
| **D** — Dependency Inversion  | Pipeline crate depends on `LineageSink` trait, not `sqlx::PgPool`                         |

### New `LineageSink` Trait (DIP)

```rust
// edgequake-pipeline/src/merger/mod.rs — extend RelationalEntitySink
#[async_trait]
pub trait LineageSink: Send + Sync {
    /// Record that chunk `chunk_id` contributed to entity `entity_name`.
    async fn record_chunk_entity_link(
        &self,
        chunk_id: &str,
        entity_name: &str,
        workspace_id: &str,
    ) -> Result<()>;

    /// Record that chunk `chunk_id` contributed to relation (src→tgt).
    async fn record_chunk_relation_link(
        &self,
        chunk_id: &str,
        source_entity: &str,
        target_entity: &str,
        workspace_id: &str,
    ) -> Result<()>;

    /// Append a description merge event to entity history.
    async fn append_description_history(
        &self,
        entity_name: &str,
        workspace_id: &str,
        description: &str,
        chunk_id: &str,
    ) -> Result<()>;
}
```
