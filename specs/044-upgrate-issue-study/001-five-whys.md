# SPEC-044 — 5 WHYs: Post-Upgrade Ingest + Cypher Compensation Failure

**Evidence:** Graylog `quantalogic-prd-edgequake` (2026-07-06), EdgeQuake **v0.14.1**, post-migration schema [`edgequakeSchema.sql`](./edgequakeSchema.sql)

---

## Symptom

Document ingestion fails; later ingestions succeed. Log shows merge error + quarantine on orphan node rollback.

---

## WHY chain

```
WHY #1 — Why does document ingestion fail?
→ `ingestion_persister` receives merge result with stats.errors = 1
  ("1 knowledge-graph merge error(s) during persist") and returns GraphError.

WHY #2 — Why does merge report errors > 0?
→ `KnowledgeGraphMerger::merge_with_progress` increments stats.errors when:
  (a) one entity fails build_entity_node_batch_entry, OR
  (b) merge_entities_batch / merge_relationships_batch returns Err (batch upsert failure).
  Entity phase may still persist other nodes (e.g. C1236) before the counted error.

WHY #3 — Why is there a quarantine log for node C1236?
→ On merge failure, `compensate_merge_failure` (SPEC-021 P-G5) attempts to delete
  all IDs in MergeArtifacts.graph_nodes_created. C1236 was recorded as a NEW node.

WHY #4 — Why does delete_node fail during compensation?
→ `pg_delete_node` calls `cypher_execute_bound`, which builds SQL:
  cypher('graph', $$ MATCH ... $$, '{"node_id":"C1236"}'::agtype)
  AGE rejects the third argument: "must be a parameter" (not a literal).

WHY #5 — ROOT CAUSE
→ v0.14.0 (#278) regressed SPEC-022 P-H7: after failed attempts with $1::agtype and
  jsonb binds, code inlined agtype literals. AGE contract (prepared-statement pattern)
  requires bare $1 as the third argument inside a prepared SQL statement.
  sqlx::raw_sql + inline literal is NOT a valid AGE parameter binding.
```

---

## Secondary vs primary

| Layer | Failure | User-visible | Data residue |
| ----- | ------- | ------------ | ------------ |
| **Primary** | Merge `errors=1` | Document status **Failed** | Partial graph/vector writes possible |
| **Secondary** | Compensation `delete_node` | Quarantine log only | Orphan node C1236 may remain |

The Graylog line is the **secondary** failure. The **primary** merge warning (`merge_entities_batch_global` or `merge_relationships_batch_global`) must be correlated in logs at the same `task_id` / `document_id`.

---

## Why later ingestion worked

| Explanation | Mechanism |
| ----------- | --------- |
| Happy path | Merge returns `errors == 0` → compensation never runs → Cypher bind bug invisible |
| Transient primary | AGE M043 upgrade lock, index bootstrap, or LLM timeout on one entity; retry succeeds |
| Different document | No failing entity / relationship batch on second attempt |

---

## Not the root cause

| Ruled out | Evidence |
| --------- | -------- |
| `public` schema migration defect | `edgequakeSchema.sql` dump after migration — tables/indexes present |
| SPEC-039 missing labels | Error text is Cypher param, not `relation "…Node" does not exist` |
| SPEC-042 pgvector iterative scan | Would degrade `/ready`, not Cypher third-arg |
| v0.14.1-specific change | CHANGELOG: volume mount fix only |

---

## Manual proof (hypothesis validation)

Reproduce AGE rejection on any Postgres with AGE:

```sql
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

-- FAILS (matches production error):
SELECT * FROM cypher('your_graph', $$
  MATCH (n:Node {node_id: $node_id}) DETACH DELETE n
$$, '{"node_id":"C1236"}'::agtype) AS (a agtype);

-- SUCCEEDS (AGE prepared-statement contract):
PREPARE eq_del_node(agtype) AS
SELECT * FROM cypher('your_graph', $$
  MATCH (n:Node {node_id: $node_id}) DETACH DELETE n
$$, $1) AS (a agtype);
EXECUTE eq_del_node('{"node_id":"C1236"}');
```

External SSOT: [AGE Prepared Statements](https://age.apache.org/age-manual/master/advanced/prepared_statements.html), [apache/age#315](https://github.com/apache/age/issues/315).
