# F-stats-n1-embedding — Workspace stats N+1 timeout

> **Finding IDs**: `iss087_stats_n1`, `iss087_kv_count_trait`, `iss087_embedding_ssot`  
> **Status**: FIXED  
> **Wave**: 1  
> **Laws**: LAW-31, LAW-32, LAW-33  
> **Issue**: [#334](https://github.com/raphaelmansuy/edgequake/issues/334)  
> **Verify**: `iss087_v_stats_under_timeout`, `iss087_v_count_trait`, `iss087_v_embedding_ssot`, `iss087_e_scale_stats`

---

## 1. Symptom

At scale (≈5,000+ documents), `GET /api/v1/workspaces/{id}/stats` returns **500** consistently and the dashboard never loads. Reporter verified ~9,241 documents / ~130k graph vertices on **v0.21.1**.

With a prior successful cache entry, P-G13 may serve `stale=true` instead of 500 — masking the bug until cold start / cache expiry.

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs` | `STATS_FETCH_TIMEOUT` L84 | 4 seconds |
| same | timeout → 500 L106–118 | No stale cache → Internal error |
| same | embedding loop L253–282 | Per-doc `keys_with_prefix` + `get_by_ids` + `.get("embedding")` |
| `edgequake/crates/edgequake-storage/src/traits/kv.rs` | `KVStorage` | **No** `count_embedded_chunks_for_docs` |
| `edgequake/crates/edgequake-storage/src/adapters/postgres/kv.rs` | `impl KVStorage` | **No** COUNT override / `jsonb_exists` |
| `edgequake/crates/edgequake-pipeline/src/chunk_storage.rs` | `chunk_kv_value` | Does **not** write `embedding` into KV JSON |
| `edgequake/crates/edgequake-core/src/workspace_service_impl/workspace_ops.rs` | `pg_get_workspace_stats` L421–468 | O(1) `COUNT(*) FROM chunks` — **unused by HTTP stats handler** |
| `edgequake/crates/edgequake-api/tests/e2e_dashboard_stats_issue81.rs` | stress | Only ~50 docs — cannot catch scale timeout |
| `git log -S count_embedded_chunks_for_docs` | (Wave 0) | **Zero** history — method never landed |

### Issue report correction

| Claim in #334 | Reality |
|---------------|---------|
| `stats.rs` already fixed in 0.19.0 | **FALSE** — N+1 loop still present |
| Postgres override already merged | **FALSE** — absent |
| Only trait default missing | **FALSE** — all three pieces missing |
| Trait name `KvStore` | Actual name: **`KVStorage`** |

---

## 3. Root cause

`embedding_count` is computed by pulling chunk documents into the API process and inspecting JSON keys. Cost is Θ(documents) round-trips plus large payload deserialization, against a fixed 4s budget. There is no trait-level aggregate, and the handler ignores the existing relational COUNT in `pg_get_workspace_stats`. Additionally, the inspected field often does not exist on the write path (SPEC-024), so the metric is both **slow and inaccurate**.

---

## 4. Fix (SOLID/DRY)

### Wave 1 (required)

1. **`KVStorage::count_embedded_chunks_for_docs(&self, doc_ids: &[String]) -> Result<usize>`**  
   - Default: current per-doc fallback (non-PG / tests).  
   - Document that default is for correctness, not scale.

2. **`PostgresKVStorage` override** — single aggregate on **`self.table_name`** (qualified), not hardcoded `eq_eq_default_kv`.  
   - Empty `doc_ids` → `Ok(0)`.  
   - Prefer counting keys matching `{doc_id}-chunk-%` for the given ids (chunk existence), **or** prefer calling/aligning with relational `chunks` COUNT when the HTTP path has workspace scope.  
   - Do **not** treat `jsonb_exists(value, 'embedding')` as SSOT if embeddings are not stored in KV.

3. **`stats.rs`**: replace L253–282 with one trait call (LAW-33).

### SSOT pin (LAW-32)

| Mode | `embedding_count` meaning |
|------|---------------------------|
| PostgreSQL product path | Align with `COUNT(*) FROM chunks WHERE workspace_id = $1` (same as `pg_get_workspace_stats`) when pool/workspace available; else count chunk KV keys for workspace docs |
| Fallback adapters | Trait default / chunk-key count |

### Non-goals

- Raising timeout to “fix” scale  
- Fetching embedding vectors into the API for counting  
- Global `keys_like` without workspace scoping

---

## 5. Edge cases

| Case | Expected |
|------|----------|
| Empty workspace / zero docs | 200, counts 0, &lt; timeout |
| Cold cache, 5k+ docs | 200 within 4s after fix |
| Warm cache then regression | Do not rely on stale; e2e clears cache or uses unique workspace |
| Huge `doc_ids` array | Single query with bound array; watch planner — prefer workspace_id COUNT when possible |
| Non-Postgres adapter | Default method; may remain O(n) but not used in product Docker path |
| Graph entity_type_count slow | Separate; already has aggregate helper — do not conflate |
| `stale=true` after timeout | Keep P-G13; fix makes timeout rare |
| Metric was 0 with embeddings present in vector store | SSOT alignment must make count non-zero when chunks exist |

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  cargo test -p edgequake-storage --lib count_embedded
  cargo test -p edgequake-api --test e2e_dashboard_stats_issue81 test_spec087
Result: pass — embedding_count matches chunk keys; 500-doc scale < 4s
```
