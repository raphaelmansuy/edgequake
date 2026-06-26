# 02 — KV Store Model

> **Spec**: 021-storage-study  
> **File**: 02-schema/02-kv-store-model.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-storage/src/adapters/postgres/kv.rs`,  
> `edgequake-core/src/orchestrator/ingestion.rs`,  
> `edgequake-pipeline/src/pipeline/helpers/`

---

## Table Structure

The KV store creates a **dynamic table** at runtime per namespace:

```sql
CREATE TABLE IF NOT EXISTS public.eq_{prefix}_kv (
    key  TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- GIN index for JSONB path queries
CREATE INDEX IF NOT EXISTS eq_{prefix}_kv_value_gin
    ON public.eq_{prefix}_kv USING GIN (value);

-- Reverse-key index for O(log N) suffix scans
CREATE INDEX IF NOT EXISTS eq_{prefix}_kv_reverse_key_idx
    ON public.eq_{prefix}_kv (reverse(key) text_pattern_ops);
```

Default production table name: `public.eq_eq_default_kv`

---

## Key Taxonomy

The KV store is used for several **conceptually distinct** data types, all sharing
one table and distinguished only by key suffix conventions.

```
KEY PATTERN                      VALUE SCHEMA          PURPOSE
---------------------------------------------------------------------
{doc_id}-metadata                DocumentMetadata      Document metadata JSON
{doc_id}-chunk-{index}           ChunkContent          Chunk text + offsets
{doc_id}-chunk-{index}-summary   ChunkSummary          Chunk summary (if summarizer enabled)
{keyword}-cache                  CacheEntry            LLM extraction cache
{query_hash}-kwcache             KeywordCacheEntry     Keyword extraction cache
doc-summary-{doc_id}             SummaryRecord         Full-document summary
{workspace_id}-config            WorkspaceConfig       Workspace-level config blob
```

> **WARNING (R-DRY-03)**: These key patterns are scattered as string literals across:
> - `edgequake-core/src/orchestrator/ingestion.rs`
> - `edgequake-pipeline/src/cache.rs`
> - `edgequake-query/src/keywords/mod.rs`
>
> There is no single `KVKeySchema` module or constant set. Key construction is
> duplicated and cannot be refactored safely without full-text search.

---

## DocumentMetadata Value Schema

```json
{
  "id": "<uuid>",
  "title": "<string>",
  "content_hash": "<sha256-hex>",
  "created_at": "<iso8601>",
  "chunk_count": 5,
  "entity_count": 12,
  "relationship_count": 8,
  "tenant_id": "<uuid|null>",
  "workspace_id": "<uuid|null>",
  "source_type": "text|pdf",
  "file_size_bytes": 12345
}
```

---

## ChunkContent Value Schema

```json
{
  "id": "<uuid>",
  "document_id": "<uuid>",
  "content": "<chunk text>",
  "chunk_index": 3,
  "start_line": 42,
  "end_line": 65,
  "token_count": 800,
  "tenant_id": "<uuid|null>",
  "workspace_id": "<uuid|null>"
}
```

---

## CacheEntry Value Schema

```json
{
  "key": "<cache-key>",
  "value": "<extracted-text|json>",
  "cache_type": "entity_extraction|summary|keyword",
  "created_at": "<iso8601>",
  "expires_at": "<iso8601|null>"
}
```

---

## Row Count Stats (SPEC-011 O(1) counter)

A companion stats table tracks row counts to avoid `SELECT COUNT(*)` full scans:

```sql
CREATE TABLE IF NOT EXISTS public.eq_{prefix}_kv_stats (
    id   INTEGER PRIMARY KEY DEFAULT 1,
    row_count BIGINT NOT NULL DEFAULT 0
);
```

Insert/delete triggers maintain `row_count` atomically.  
Source: `edgequake-storage/src/adapters/postgres/row_count_stats.rs`

---

## Multiple KV Namespaces in Production

In practice, a production deployment may have multiple KV tables depending on
namespace configuration:

| Table                    | Namespace    | Usage                              |
| ------------------------ | ------------ | ---------------------------------- |
| `eq_eq_default_kv`       | `eq_default` | Default workspace documents/chunks |
| `eq_eq_default_kv_stats` | `eq_default` | Row counter for above              |

When workspace-scoped namespaces are used (future: per-workspace KV), the table
count grows with workspaces.

---

## KVStorage Trait Key Methods

| Method                     | SQL Operation                                         | Notes                                       |
| -------------------------- | ----------------------------------------------------- | ------------------------------------------- |
| `get_by_id(id)`            | `SELECT value WHERE key = $1`                         | O(1) primary key lookup                     |
| `filter_keys(keys)`        | `SELECT key WHERE key = ANY($1)`                      | Deduplication check                         |
| `upsert(data)`             | `INSERT ... ON CONFLICT DO UPDATE`                    | Batch UNNEST                                |
| `keys_with_prefix(prefix)` | `SELECT key WHERE key LIKE 'prefix%'`                 | B-tree friendly                             |
| `keys_with_suffix(suffix)` | `SELECT key WHERE reverse(key) LIKE reverse(suffix)%` | Requires `reverse(key)` index               |
| `ping()`                   | DEFAULT: calls `count()` → O(N)                       | **BUG**: default should SELECT 1, not COUNT |
