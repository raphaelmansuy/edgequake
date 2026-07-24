# SPEC-087 — First Principles

> **Status**: Active (Wave 0 docs)  
> **Product pin**: EdgeQuake v0.21.1  
> **Cross-refs**: [README](README.md) · [Register](01-finding-register.md) · [Roadmap](03-implementation-roadmap.md) · [Contract pins](06-contract-pins.md)  
> **Inherits**: [SPEC-017 DRY/SOLID](../017-dry-and-solid-audit/) · [SPEC-024 write-path SSOT](../024-egdequake-audit/) · [SPEC-027 FK bootstrap](../027-api-edgequake-audit/) · [SPEC-085 LAW-15…21](../085-fix-security/00-first-principles.md)

---

## 1. WHY this pack exists

Two independent production bugs share one theme: **correctness shortcuts that ignore scale and operator trust**.

1. **[#335](https://github.com/raphaelmansuy/edgequake/issues/335)** — every unauthenticated browser that chats or creates a conversation gets a real `users` row (`anon_*` / `@anonymous.local`). Operators see spam in Admin → Users; public instances grow unbounded.  
2. **[#334](https://github.com/raphaelmansuy/edgequake/issues/334)** — workspace stats compute `embedding_count` by scanning every document’s chunk payloads under a 4-second timeout. At ~5k–9k documents the dashboard dies with 500.

Both are reproducible on current mainline (v0.21.1). Neither is a “docs-only” misunderstanding.

---

## 2. Five WHYs — Issue #335 (anonymous users)

**Symptom:** Admin Users fills with `anon_xxxxxxxx` / `xxxxxxxx@anonymous.local` that the operator never created.

### WHY 1 — Why do rows appear?

Because chat/completion, chat/streaming, and conversation create call `ensure_postgres_user_exists`, which INSERTs into `users`.

### WHY 2 — Why insert at all?

Because `conversations.user_id` (and related FKs) reference `users(user_id)`. SPEC-027 treated bootstrap as FK safety.

### WHY 3 — Why a *new* user per browser?

Because the Web UI generates a random UUID in `localStorage.userId` (`getOrCreateUserId`) and sends it as `X-User-ID`. Bootstrap uses that UUID as the primary key.

### WHY 4 — Why doesn’t auth stop it?

Because local/demo defaults turn auth off (`EDGEQUAKE_DEV_MODE` / `EDGEQUAKE_AUTH_ENABLED=false` via Makefile/quickstart). Even when auth is on, middleware does not always overwrite `TenantContext.user_id` from JWT subject, so the header UUID can still mint.

### WHY 5 — Why do operators think accounts were hijacked?

Because the Users panel lists all rows with no anonymous filter/label, and username/email look like real accounts. Login is blocked (`password_hash = 'anonymous'` fails `is_login_capable_password_hash`) — so this is **trust/bloat**, not takeover.

**Root cause:** FK safety was implemented as **per-session identity minting** without a bounded guest model, admin visibility contract, or config gate. LAW-30 violated.

---

## 3. Five WHYs — Issue #334 (stats N+1)

**Symptom:** `GET /api/v1/workspaces/{id}/stats` returns 500; dashboard never loads at scale (~5k+ docs).

### WHY 1 — Why 500?

Because `STATS_FETCH_TIMEOUT` (4s) fires and there is no stale cache → `ApiError::Internal("Workspace stats temporarily unavailable…")`.

### WHY 2 — Why does fetch exceed 4s?

Because `try_kv_storage_stats` loops every workspace `doc_id`, calling `keys_with_prefix` then `get_by_ids` for full chunk JSON just to test `.get("embedding").is_some()`.

### WHY 3 — Why fetch full payloads?

Because there is no `KVStorage::count_embedded_chunks_for_docs` (or equivalent aggregate) in the trait/adapters; the handler inlines an O(docs) client-side scan.

### WHY 4 — Why is the metric often wrong *and* slow?

Because current `chunk_kv_value` (SPEC-024) **does not write** an `embedding` field into chunk KV — embeddings live in the vector/chunks path. The loop pays N+1 cost and frequently still returns `embedding_count = 0`.

### WHY 5 — Why wasn’t the relational O(1) path used?

Because `pg_get_workspace_stats` in `workspace_ops.rs` already does `COUNT(*) FROM chunks WHERE workspace_id = $1` in one round-trip, but the HTTP dashboard handler uses the KV scan path instead. Two SSOTs; the slow one is live.

**Root cause:** Dashboard aggregate implemented as **client-side N+1 payload inspection** instead of a server-side COUNT aligned with the write path. LAW-31 and LAW-32 violated.

**Issue accuracy note:** The report’s claim that `stats.rs` and the Postgres override were “already merged in 0.19.0” is **false** on this tree — all three fix pieces are missing.

---

## 4. Laws (SPEC-087)

Reuse LAW-1…LAW-28 from prior packs. SPEC-087 adds:

```
  LAW-29  Identity rows are operator-visible accounts unless explicitly guest-scoped and bounded
  LAW-30  FK safety must not imply unbounded INSERT (shared guest or real user — never per-browser mint)
  LAW-31  Dashboard aggregates are O(1) or O(log n) server-side — never O(docs) payload fetch
  LAW-32  Metric SSOT must match the write path (chunks/vector vs KV JSON fields)
  LAW-33  Trait defaults enable adapters; hot paths call the trait, not copy loops
```

### ASCII: causal stacks

```
#335
  localStorage UUID
       → X-User-ID
       → ensure_postgres_user_exists
       → INSERT anon_*
       → Admin Users clutter + unbounded growth

#334
  workspace_doc_ids
       → per-doc keys_with_prefix + get_by_ids
       → deserialize chunk JSON
       → check embedding key (often absent)
       → exceed 4s → 500 (cold cache)
```

---

## 5. SOLID / DRY constraints

| Principle | Application |
|-----------|-------------|
| **SRP** | Bootstrap owns identity ensure; stats owns aggregation; admin list owns visibility filter — do not bury policy in three handlers |
| **OCP** | New `KVStorage` method with default; Postgres overrides; handlers call trait once |
| **LSP** | Default fallback preserves semantics for non-PG adapters |
| **ISP** | Count method is narrow — do not expand into a second stats service |
| **DIP** | Handlers depend on `KVStorage` / identity helper abstractions, not raw SQL in HTTP layer |
| **DRY** | One bootstrap helper (shared guest); one count method; retarget SPEC-027 pin away from per-UUID mint string |

---

## 6. Non-goals

- Temporal / pipeline / SPEC-086 ingestion UX changes  
- Making `conversations.user_id` nullable (schema rot)  
- Removing chat when auth is off (unless `EDGEQUAKE_ALLOW_ANONYMOUS=false`)  
- Raising `STATS_FETCH_TIMEOUT` as a substitute for O(1) aggregates  
- Hardcoding `eq_eq_default_kv` in the Postgres override
