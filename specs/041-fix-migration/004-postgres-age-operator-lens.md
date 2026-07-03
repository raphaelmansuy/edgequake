# SPEC-041 — PostgreSQL JSON / AGE Operator Lens

---

## PostgreSQL JSON operators (authoritative)

From [PostgreSQL JSON functions documentation](https://www.postgresql.org/docs/current/functions-json.html):

| Operator | Left type | Right | Result | M078 usage |
| -------- | --------- | ----- | ------ | ---------- |
| `->` | json/jsonb | int or text key | json/jsonb | ❌ Not used here |
| `->>` | json/jsonb | int or text key | **text** | ✅ Required for B-tree index on text key |
| `->>>` | — | — | **DOES NOT EXIST** | ❌ Bug in v0.13.2 |

**Error signature:**

```text
ERROR: operator does not exist: json ->>> unknown
HINT: No operator matches the given name and argument types.
```

---

## AGE property access chain

```text
properties (agtype column on "Node")
    ↓ ag_catalog.agtype_to_json()
json value
    ↓ ->>'workspace_id'
text (UUID string)
    ↓ B-tree index
Index Cond: ((agtype_to_json(properties) ->> 'workspace_id'::text) = $1)
```

**Why `->>` not `->`:** B-tree indexes on JSON need a scalar type. `->` returns json; `->>` returns text — matches query predicates in `query_ops.rs` and `analytics_ops.rs`.

---

## M078 index definitions (corrected)

```sql
CREATE INDEX idx_node_workspace_id ON {graph}."Node"
  ((ag_catalog.agtype_to_json(properties)->>'workspace_id'));

CREATE INDEX idx_node_tenant_id ON {graph}."Node"
  ((ag_catalog.agtype_to_json(properties)->>'tenant_id'));
```

Edge indexes (unchanged — never had typo):

```sql
CREATE INDEX idx_edge_start_id_text ON {graph}."EDGE" ((start_id::text));
CREATE INDEX idx_edge_end_id_text ON {graph}."EDGE" ((end_id::text));
```

---

## Quote escaping in PL/pgSQL `EXECUTE format()`

Inside `EXECUTE format('...')`, single quotes in SQL literals are doubled:

```sql
-- Source file shows:
->>''workspace_id''

-- Executed SQL becomes:
->>'workspace_id'
```

The bug was an **extra `>`**, not a quoting issue.

---

## Edge cases by environment

| Case | M078 path | Expected |
| ---- | --------- | -------- |
| `age` extension missing | Early RETURN line 26-28 | Success, no indexes |
| `ag_catalog` missing | Early RETURN line 31-33 | Success, no indexes |
| Graph with no `"Node"` rel | CONTINUE line 39-42 | Success, skip graph |
| Graph with `"Node"`, no `"EDGE"` | Node indexes + ANALYZE Node only | Success |
| Graph with both | All 4 indexes + ANALYZE both | Success |
| Index already exists | IF NOT EXISTS pg_indexes check | Skip CREATE |
| Second run (idempotent) | All IF NOT EXISTS | Success, no-op |

---

## Verification queries

```sql
-- Prove index expression uses ->> not ->>>
SELECT pg_get_indexdef(indexrelid)
FROM pg_indexes
WHERE indexname = 'idx_node_workspace_id'
  AND schemaname = 'eq_eq_default_graph';
-- MUST contain: ->> 'workspace_id'
-- MUST NOT contain: ->>>

-- Prove operator works in predicate (runtime query shape)
EXPLAIN SELECT COUNT(*)
FROM eq_eq_default_graph."Node" n
WHERE ag_catalog.agtype_to_json(n.properties)->>'workspace_id' = 'some-uuid';
```

---

## Concurrent vs inline (SPEC-040 design)

| Variant | Transaction | When |
| ------- | ----------- | ---- |
| `078_age_child_workspace_stats.sql` | Inside sqlx tx | Default startup |
| `support/078/concurrent.sql` | Autocommit, CONCURRENTLY | >100k nodes prod ops |

Both must use **identical index expressions** (DRY / Liskov).
