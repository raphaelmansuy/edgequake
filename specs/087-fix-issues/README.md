# SPEC-087 — Fix Issues #334 / #335

> **Product pin**: EdgeQuake v0.21.1  
> **Docs status**: Spec pack authored 2026-07-24 (Wave 0)  
> **Implementation**: Waves 1–3 landed 2026-07-24  
> **Sources**: [#334](https://github.com/raphaelmansuy/edgequake/issues/334) · [#335](https://github.com/raphaelmansuy/edgequake/issues/335)  
> **Inherits**: [SPEC-017](../017-dry-and-solid-audit/) · [SPEC-024](../024-egdequake-audit/) · [SPEC-027](../027-api-edgequake-audit/) · [SPEC-085 laws](../085-fix-security/00-first-principles.md) · [SPEC-086 pack shape](../086-improve-ingestion-ux/)

## Verification status (SSOT)

See [01-finding-register.md](01-finding-register.md): **7 FIXED / 1 PARTIAL / 0 OPEN**.

| Wave | Goal | Status |
|------|------|--------|
| **0** | Spec pack + lenses + findings + GitHub comment drafts | **done** |
| **1** | #334 stats N+1 → O(1) aggregate (trait + Postgres + stats.rs) | **done** |
| **2** | #335 shared guest + JWT bind + admin filter + config | **done** |
| **3** | E2E gates green + post issue comments + orphan cleanup docs | **done** |

---

## Start here

1. [00-first-principles.md](00-first-principles.md) — LAW-29…LAW-33 + five whys  
2. [01-finding-register.md](01-finding-register.md) — every finding with status  
3. [02-cross-ref-matrix.md](02-cross-ref-matrix.md) — `iss087_*` IDs  
4. [03-implementation-roadmap.md](03-implementation-roadmap.md) — DRY/SOLID waves  
5. [04-verification-matrix.md](04-verification-matrix.md) — gates  
6. [05-surface-playbooks.md](05-surface-playbooks.md) — repro + fix commands  
7. [06-contract-pins.md](06-contract-pins.md) — identity + stats contracts  
8. [07-github-issue-comments.md](07-github-issue-comments.md) — ready-to-post root-cause replies  
9. Lenses → [`lenses/`](lenses/README.md)  
10. Findings → [`findings/`](findings/README.md)  
11. E2E → [`e2e/README.md`](e2e/README.md)

---

## Locked decisions

### #335 — Anonymous identity

1. **Auth ON**: resolve `user_id` from JWT subject; sync WebUI `localStorage.userId` to authenticated user; **never** mint per-browser `anon_*` for authenticated requests.  
2. **Auth OFF (dev/demo)**: use a **single shared per-tenant guest user** (deterministic UUID + stable username/email + sentinel hash) so FK stays valid and row count stays O(1) per tenant.  
3. **Admin Users**: default list excludes (or clearly labels) anonymous sentinel hashes / `@anonymous.local`; optional `include_anonymous=true`.  
4. **Config**: `EDGEQUAKE_ALLOW_ANONYMOUS` — when `false` and auth off, chat/conversation create returns 401/403 instead of minting. Default path when auth off: shared guest (preserves open demo).  
5. **Cleanup**: documented SQL / admin action for existing `anon_%` orphans.

### #334 — Workspace stats

1. Add `count_embedded_chunks_for_docs` **default** on `KVStorage` (fallback loop for non-PG).  
2. Postgres override: **one** aggregate on `self.table_name` (never hardcode `eq_eq_default_kv`); empty `doc_ids` → `0`.  
3. Replace the per-doc loop in `stats.rs` with one trait call.  
4. **SSOT accuracy**: Prefer counting **chunk keys** for workspace docs, or align with relational `chunks` COUNT from `pg_get_workspace_stats` when PG is available — do **not** treat `jsonb_exists(value,'embedding')` as truth if embeddings live outside KV (SPEC-024).  
5. E2E scale smoke (≥500 synthetic chunk keys) must finish inside the 4s timeout.

---

## Code verification (Wave 0 audit)

| Claim | Reality on v0.21.1 |
|-------|-------------------|
| #335 silent anon mint | **CONFIRMED** — `ensure_anonymous_user_in_postgres` still inserts per `X-User-ID` |
| #334 N+1 embedding scan | **CONFIRMED** — loop in `stats.rs` L253–282 under 4s timeout |
| #334 “stats.rs + postgres override already merged in 0.19.0” | **FALSE** — neither merged; trait also missing; all three pieces required |
| `EDGEQUAKE_ALLOW_ANONYMOUS` exists | **FALSE** |
| Chunk KV stores `embedding` field | **FALSE** on current write path (`chunk_kv_value`) |

---

## Surfaces (blast radius)

| Surface | Role |
|---------|------|
| `edgequake-api` | Bootstrap, stats handler, users list, middleware user_id |
| `edgequake-storage` | `KVStorage` trait + Postgres KV adapter |
| `edgequake-core` | `pg_get_workspace_stats` relational SSOT (align HTTP path) |
| `edgequake_webui` | `localStorage.userId`, Users panel filter/label |
| Playwright / API e2e | Identity + scale stats gates |

---

## Success criteria

- Cold-cache `GET /api/v1/workspaces/{id}/stats` returns 200 within timeout at ≥5k docs.  
- `embedding_count` matches the pinned SSOT (chunk keys or relational `chunks`).  
- Auth-off chat does not create a new `anon_*` row per browser — at most one shared guest per tenant.  
- Auth-on chat uses JWT subject; admin Users default view is free of anonymous spam.  
- Spec cross-refs 100%: every finding ↔ law ↔ wave ↔ verify ID.  
- E2E matrix in [`e2e/README.md`](e2e/README.md) green after Waves 1–3.
