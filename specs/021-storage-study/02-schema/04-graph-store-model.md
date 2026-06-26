# 04 — Graph Store Model (Apache AGE)

> **Spec**: 021-storage-study  
> **File**: 02-schema/04-graph-store-model.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-storage/src/adapters/postgres/graph/`,  
> `edgequake/migrations/013_add_age_graph.sql`,  
> `edgequake/migrations/014_add_graph_indexes.sql`

---

## Overview

EdgeQuake uses **Apache AGE** (A Graph Extension) for property graph storage.
AGE exposes a Cypher query interface on top of PostgreSQL. The graph is stored
in its own internal tables under the `ag_catalog` schema.

```
PostgreSQL
  |-- public schema (relational tables, KV, vector)
  |-- ag_catalog schema (AGE internals)
  |    |-- ag_graph (graph registry)
  |    |-- {graph_name}._ag_label_vertex (node storage per label)
  |    |-- {graph_name}._ag_label_edge   (edge storage per label)
  |-- search_path: public, ag_catalog, "$user"
```

---

## Graph Naming

Each `PostgresAGEGraphStorage` instance maps to a named AGE graph.  
Default graph name: `"edgequake"` (from namespace).

```
CYPHER: SELECT * FROM cypher('edgequake', $$...$$) as (...)
```

---

## Node Schema (`Node` label)

Every entity extracted from documents becomes a `Node` in the graph:

```
Node properties (stored as AGE agtype):
  node_id:      TEXT   -- MERGE key, = normalized entity name (UPPERCASE_UNDERSCORE)
  entity_type:  TEXT   -- e.g. "PERSON", "ORGANIZATION", "LOCATION", "CONCEPT"
  description:  TEXT   -- LLM-generated description, merged across documents
  source_ids:   TEXT   -- JSON array of contributing chunk IDs: ["doc1-chunk-0", ...]
  tenant_id:    TEXT   -- multi-tenancy isolation
  workspace_id: TEXT   -- workspace isolation
  created_at:   TEXT   -- ISO-8601 timestamp (stored as string in agtype)
  updated_at:   TEXT   -- ISO-8601 timestamp
  keywords:     TEXT   -- JSON array of associated keywords
```

**Identity key**: `node_id` (= normalized entity name)  
**MERGE pattern**: `MERGE (n:Node {node_id: '...'}) SET n = {...}`

---

## Edge Schema (`EDGE` label)

Relationships become directed `EDGE` entries:

```
EDGE properties (stored as AGE agtype):
  source_id:     TEXT   -- source node_id (MERGE key component)
  target_id:     TEXT   -- target node_id (MERGE key component)
  relation_type: TEXT   -- e.g. "WORKS_AT", "LOCATED_IN", "RELATED_TO"
  description:   TEXT   -- LLM-generated relationship description
  keywords:      TEXT   -- JSON array ["CEO", "leads", ...]
  weight:        FLOAT  -- relationship strength (0.0–1.0)
  source_ids:    TEXT   -- JSON array of contributing chunk IDs
  tenant_id:     TEXT
  workspace_id:  TEXT
  created_at:    TEXT
  updated_at:    TEXT
```

**Identity key**: `(source_id, target_id)` pair  
**MERGE pattern**:
```cypher
MERGE (a:Node {node_id: 'SRC'})
MERGE (b:Node {node_id: 'TGT'})
MERGE (a)-[r:EDGE {source_id: 'SRC', target_id: 'TGT'}]->(b)
SET r += {props}
```

---

## Indexes (AGE B-tree)

Created after first node insertion (lazy initialization):

```sql
-- Node lookup by node_id
CREATE INDEX IF NOT EXISTS eq_{ns}_age_node_id_idx
    ON {graph}._ag_label_vertex USING BTREE ((properties->>'node_id'));

-- Node lookup by entity_type
CREATE INDEX IF NOT EXISTS eq_{ns}_age_entity_type_idx
    ON {graph}._ag_label_vertex USING BTREE ((properties->>'entity_type'));

-- Edge lookup by source_id
CREATE INDEX IF NOT EXISTS eq_{ns}_age_edge_source_idx
    ON {graph}._ag_label_edge USING BTREE ((properties->>'source_id'));

-- Edge lookup by target_id
CREATE INDEX IF NOT EXISTS eq_{ns}_age_edge_target_idx
    ON {graph}._ag_label_edge USING BTREE ((properties->>'target_id'));
```

Source: `edgequake-storage/src/adapters/postgres/graph/lifecycle_ops.rs`

---

## Batch Operations (SC1 — Single Round-Trip)

### Node Batch Upsert (UNWIND)

```cypher
UNWIND [{node_id: 'A', entity_type: 'PERSON', ...}, ...] AS props
MERGE (n:Node {node_id: props.node_id})
SET n = props
```

Chunk size: 500 nodes per statement.

### Edge Batch Upsert (UNWIND)

```cypher
UNWIND [{source_id: 'A', target_id: 'B', ...}, ...] AS e
MERGE (a:Node {node_id: e.source_id})
MERGE (b:Node {node_id: e.target_id})
MERGE (a)-[r:EDGE {source_id: e.source_id, target_id: e.target_id}]->(b)
SET r += e
```

Chunk size: 500 edges per statement.

---

## Supported Cypher Operations

| Operation       | Cypher Pattern                                           | Method                  |
| --------------- | -------------------------------------------------------- | ----------------------- |
| Node exists     | `MATCH (n:Node {node_id: 'X'}) RETURN n LIMIT 1`         | `has_node()`            |
| Get node        | `MATCH (n:Node {node_id: 'X'}) RETURN n`                 | `get_node()`            |
| Get neighbors   | `MATCH (n:Node {node_id: 'X'})--(m) RETURN m`            | `get_neighbors()`       |
| Get edges       | `MATCH (:Node {node_id: 'X'})-[r:EDGE]-() RETURN r`      | `get_node_edges()`      |
| Search nodes    | `MATCH (n:Node) WHERE n.node_id CONTAINS '...' RETURN n` | `search_nodes()`        |
| Node degree     | `MATCH (n:Node {node_id: 'X'})-[r]-() RETURN count(r)`   | `node_degree()`         |
| K-hop traversal | BFS with visited set                                     | `get_knowledge_graph()` |
| Delete node     | `MATCH (n:Node {node_id: 'X'}) DETACH DELETE n`          | `delete_node()`         |

---

## Fallback Behavior (no AGE extension)

Migration 013 gracefully handles missing AGE:

```sql
DO $$ BEGIN
    CREATE EXTENSION IF NOT EXISTS age CASCADE;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'AGE not available — using relational fallback';
END $$;
```

However, the `PostgresAGEGraphStorage` Rust implementation does **NOT** have a
relational fallback. If AGE is unavailable, graph operations fail with a
`StorageError::Database` error.

The **only** real fallback is `MemoryGraphStorage` (for testing).

---

## Multi-Tenancy in the Graph

Tenant and workspace isolation in AGE is **property-based**, not schema-based:

- Every node carries `tenant_id` and `workspace_id` properties.
- Queries filter by these properties: `WHERE n.tenant_id = '...' AND n.workspace_id = '...'`
- There is **no row-level security** in AGE (unlike relational tables).
- Cross-tenant data leakage is possible if callers omit tenant filter arguments.

See [05-risks/03-data-consistency-risks.md](../05-risks/03-data-consistency-risks.md) for risk details.
