# LENS — Postgres Expert

> **Laws**: LAW-29, LAW-30, LAW-31, LAW-32  
> **Findings**: `iss087_anon_mint`, `iss087_stats_n1`, `iss087_kv_count_trait`, `iss087_embedding_ssot`

---

## 1. Question

How should PostgreSQL encode guest identity and workspace aggregates so we preserve FK/RLS integrity without unbounded growth or N+1 payload scans?

---

## 2. Identity (#335)

### Facts

- `conversations.user_id` → `users(user_id)` is a real FK (migrations 001/010 family).  
- Current INSERT uses `ON CONFLICT (user_id) DO NOTHING` inside `with_optional_pg_rls` + `PgIsolationScope::for_membership`.  
- Username/email uniqueness: `anon_{uuid[..8]}` has a birthday-collision risk across distinct UUIDs sharing the same 8-char prefix.  
- Sentinel `password_hash = 'anonymous'` is not a login hash; good for authn rejection, useless for SELECT filtering unless the list query knows about it.

### Judgment

| Anti-pattern | Prefer |
|--------------|--------|
| Per-browser INSERT | One **shared guest** row per `tenant_id` (deterministic UUID) |
| Nullable `user_id` | Keep NOT NULL FK; guest is a real row |
| Hard-delete guest with children | `ON DELETE RESTRICT` + reassign, or soft-disable |

### Cleanup SQL (operator playbook sketch)

```sql
-- Inventory
SELECT user_id, username, email, created_at
FROM users
WHERE email LIKE '%@anonymous.local'
   OR password_hash = 'anonymous'
ORDER BY created_at;

-- After shared guest exists: reassign then delete orphans (run in txn; verify FKs)
-- UPDATE conversations SET user_id = $guest WHERE user_id = ANY($orphan_ids);
-- DELETE FROM users WHERE user_id = ANY($orphan_ids);
```

### RLS note

Guest UUID must remain a valid membership principal for `PgIsolationScope`. Do not invent a second “null user” path that bypasses RLS helpers.

---

## 3. Stats (#334)

### Facts

- Hot path scans KV JSON via many round-trips.  
- Product already has O(1) relational stats:  
  `COUNT(*) FROM chunks WHERE workspace_id = $1` in `pg_get_workspace_stats`.  
- Issue-proposed SQL hardcodes `eq_eq_default_kv` — **wrong** for multi-table/qualified names; adapter must use `self.table_name` / `qualified_kv_table_name`.  
- `jsonb_exists(value, 'embedding')` is a poor predicate if embeddings are not stored in KV (SPEC-024 / `chunk_kv_value`).

### Judgment

1. **Best SSOT for product PG**: workspace-scoped `COUNT(*) FROM chunks` (indexed `workspace_id`) — same as core workspace service.  
2. **KV trait method**: still valuable for non-PG + as a scoped chunk-key COUNT when relational table unavailable.  
3. Prefer key-pattern / relational counts over JSONB existence checks for this metric.  
4. For large `ANY($1::text[])` binds: when counting for a whole workspace, **prefer `workspace_id = $1`** over shipping thousands of doc ids.

### Index / planner

- Confirm chunk / KV key indexes support prefix or workspace filters used by the chosen query.  
- Avoid pulling TOASTed JSONB values into the backend for COUNT.

---

## 4. Acceptance for this lens

- [ ] No per-browser `users` INSERT on chat  
- [ ] Guest row count ≤ 1 per tenant (auth off)  
- [ ] Stats path performs ≤ 1 aggregate query for embedding/chunk count (PG)  
- [ ] No hardcoded KV table name in adapter override
