# 05 — Dynamic Table Naming Conventions

> **Spec**: 021-storage-study  
> **File**: 02-schema/05-dynamic-tables.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-storage/src/adapters/postgres/config.rs`,  
> `edgequake-storage/src/adapters/postgres/vector/ddl.rs`,  
> `edgequake-storage/src/traits/workspace_vector.rs`

---

## Naming Formula

All dynamic tables follow a `eq_{prefix}_{type}` pattern:

```
eq_  +  {prefix}  +  _  +  {type}
 |         |                  |
 |    namespace-derived    kv | vectors | kv_stats | vectors_stats
 |
 "eq" = EdgeQuake namespace marker
```

### Prefix Derivation

```rust
// PostgresConfig::table_prefix()
fn table_prefix(&self) -> String {
    format!("eq_{}", self.namespace)  // namespace default = "default"
}
// Result: "eq_default"

// Full table name:
let table_name = format!("public.eq_{}_kv", prefix);
// Result: "public.eq_eq_default_kv"
```

> **NOTE (R-DRY-04)**: The double `eq_` prefix (`eq_eq_default_kv`) is an
> artifact of the namespace default being `"default"` while the function
> prepends `eq_`. This creates confusing table names in production.

---

## Table Name Catalogue

| Variable     | Default Value                        | Description                      |
| ------------ | ------------------------------------ | -------------------------------- |
| `namespace`  | `"default"`                          | From `PostgresConfig::namespace` |
| `prefix`     | `"eq_default"`                       | `table_prefix()`                 |
| KV table     | `public.eq_eq_default_kv`            | Document/chunk/cache data        |
| KV stats     | `public.eq_eq_default_kv_stats`      | O(1) row counter                 |
| Vector table | `public.eq_eq_default_vectors`       | Global embeddings                |
| Vector stats | `public.eq_eq_default_vectors_stats` | O(1) row counter                 |

### Per-Workspace Vector Tables

```rust
// WorkspaceVectorConfig::table_name()
fn table_name(&self) -> String {
    let short_id = &self.workspace_id.to_string()[..8];
    format!("eq_{}_ws_{}_vectors", self.namespace, short_id)
}
// namespace="default", workspace_id="4e32a055-..."
// Result: "eq_default_ws_4e32a055_vectors"
```

| Workspace UUID | Dimension | Table Name                       |
| -------------- | --------- | -------------------------------- |
| `4e32a055-...` | 1536      | `eq_default_ws_4e32a055_vectors` |
| `9b7c1234-...` | 768       | `eq_default_ws_9b7c1234_vectors` |

---

## Table Lifecycle

```
AppState::new_postgres()
    |
    +--> KVStorage::initialize()
    |    |-- CREATE TABLE IF NOT EXISTS eq_{prefix}_kv
    |    |-- CREATE INDEX (GIN, reverse-key)
    |    |-- CREATE stats table + triggers
    |
    +--> VectorStorage::initialize()  (global default table)
    |    |-- CREATE TABLE IF NOT EXISTS eq_{prefix}_vectors
    |    |-- CREATE INDEX (HNSW, GIN, btree for doc_id/tenant_ws)
    |    |-- CREATE stats table + triggers
    |
    +--> GraphStorage::initialize()
    |    |-- SELECT create_graph_safe('edgequake')
    |    |-- (AGE graph tables created lazily on first node write)
    |
    +--> WorkspaceVectorRegistry::get_or_create(workspace_id, dim)  [per request]
         |-- CREATE TABLE IF NOT EXISTS eq_{ns}_ws_{uuid8}_vectors
         |-- CREATE INDEX (HNSW, GIN)
         [called lazily when first workspace-scoped vector write occurs]
```

---

## Schema Impact on Migrations

Because KV and vector tables are created at runtime (not in static migration
files), they are **not tracked by SQLx migrations**. This has consequences:

1. **No migration rollback**: Dropping/altering KV or vector tables requires
   manual SQL or a dedicated cleanup script.
2. **No schema version**: The table structure can drift between deployments if
   `create_table()` logic changes without a migration marker.
3. **Workspace tables multiply**: Each workspace creates a new vector table.
   In multi-tenant deployments with hundreds of workspaces, this produces
   hundreds of tables with no automated cleanup for deleted workspaces.

---

## Row Count Stats Implementation

```sql
-- Stats table
CREATE TABLE IF NOT EXISTS public.eq_{prefix}_{kind}_stats (
    id        INTEGER PRIMARY KEY DEFAULT 1,
    row_count BIGINT NOT NULL DEFAULT 0
);

-- INSERT trigger
CREATE TRIGGER eq_{prefix}_{kind}_insert_trigger
    AFTER INSERT ON public.eq_{prefix}_{kind}
    FOR EACH ROW EXECUTE FUNCTION eq_{prefix}_{kind}_count_inc();

-- DELETE trigger
CREATE TRIGGER eq_{prefix}_{kind}_delete_trigger
    AFTER DELETE ON public.eq_{prefix}_{kind}
    FOR EACH ROW EXECUTE FUNCTION eq_{prefix}_{kind}_count_dec();
```

Source: `edgequake-storage/src/adapters/postgres/row_count_stats.rs`
