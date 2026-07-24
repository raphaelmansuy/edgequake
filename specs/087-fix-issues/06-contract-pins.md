# SPEC-087 — Contract Pins

> Immutable product contracts for Waves 1–2. Tests must pin these behaviors.

---

## C-087-01 — Stats latency budget

| Pin | Value |
|-----|-------|
| Endpoint | `GET /api/v1/workspaces/{workspace_id}/stats` |
| Timeout constant | `STATS_FETCH_TIMEOUT = 4s` (may remain) |
| Cold-cache success | Must complete within timeout for D ≥ 500 in CI e2e; design target D ≥ 5000 |
| Failure without stale cache | 500 Internal with retry message (existing) — should become rare |
| Stale-if-error | P-G13 preserved |

---

## C-087-02 — Embedding / chunk count SSOT

| Pin | Value |
|-----|-------|
| Write path | Chunk content in KV via `chunk_kv_value` **without** required `embedding` JSON field |
| Vectors | Live in vector / relational chunks path (SPEC-024 lineage) |
| HTTP `embedding_count` | **Must match** `COUNT(*) FROM chunks WHERE workspace_id = $1` when PG relational path is available; otherwise count of chunk KV keys for workspace docs |
| Forbidden SSOT | Relying solely on `value ? 'embedding'` / `jsonb_exists(value, 'embedding')` as the only truth |

---

## C-087-03 — `KVStorage` count API

| Pin | Value |
|-----|-------|
| Trait | `KVStorage` |
| Method | `count_embedded_chunks_for_docs(&self, doc_ids: &[String]) -> Result<usize>` |
| Default | Per-doc prefix fallback (non-PG) |
| Postgres | Single query; table = `self.table_name` (qualified); **never** hardcode `eq_eq_default_kv` |
| Empty input | `Ok(0)` |

---

## C-087-04 — Shared guest identity

| Pin | Value |
|-----|-------|
| Auth off + allow anonymous | Exactly **one** guest user row per tenant for unauthenticated chat/conversation create |
| Guest markers | Sentinel non-login `password_hash` (e.g. `anonymous`) + stable email domain `@anonymous.local` (or successor) + stable username |
| Auth on | `user_id` = JWT subject; **zero** guest/anon mint for that request |
| `EDGEQUAKE_ALLOW_ANONYMOUS=false` | Chat/conversation create denied when unauthenticated; no INSERT |
| Admin list default | Excludes guest/anonymous rows unless `include_anonymous=true` |

---

## C-087-05 — SPEC-027 pin retarget

| Before | After |
|--------|-------|
| Source must contain `ensure_anonymous_user_in_postgres` | Source must contain shared-guest helper (final name pinned in Wave 2 PR) **and** must **not** INSERT using client-supplied UUID as guest PK when auth off |

---

## C-087-06 — Frontend identity sync

| Pin | Value |
|-----|-------|
| On login | `localStorage.userId` set to authenticated user id |
| On logout (auth on product) | Clear or replace; do not keep minting chats as random UUID against API without guest mapping |
| Header | `X-User-ID` must not invent a second identity when JWT is present |

---

## Explicit non-pins

- Exact guest UUID algorithm (v5 vs constant) — choose in Wave 2; document in PR.  
- Whether guest appears in admin with toggle default off — default **hidden**.  
- Raising the 4s timeout.
