# SPEC-087 — Implementation Roadmap

> DRY/SOLID waves. Wave 0 (this pack) is **done**. Code waves execute after operator go-ahead.

---

## Wave 0 — Spec pack (done 2026-07-24)

- [x] First principles + laws LAW-29…33  
- [x] Findings + lenses  
- [x] Register, cross-refs, verification, playbooks, contract pins  
- [x] E2E matrix + GitHub comment drafts  

---

## Wave 1 — Stats N+1 (#334) — P0

**Goal:** Cold-cache workspace stats return 200 within 4s at scale; embedding metric matches SSOT.

| Step | Change | Files |
|------|--------|-------|
| 1.1 | Add default `count_embedded_chunks_for_docs` on `KVStorage` | `edgequake-storage/src/traits/kv.rs` |
| 1.2 | Postgres override: one aggregate on `self.table_name`; empty → 0 | `adapters/postgres/kv.rs` |
| 1.3 | Replace loop in `try_kv_storage_stats` with trait call | `handlers/workspaces/stats.rs` |
| 1.4 | Align SSOT with relational `chunks` COUNT when PG workspace path available | `stats.rs` and/or reuse `pg_get_workspace_stats` fields |
| 1.5 | Unit + contract + scale smoke tests | storage lib tests; `e2e_dashboard_stats_*` / new `iss087` test |

**Exit:** `iss087_stats_n1`, `iss087_kv_count_trait`, `iss087_embedding_ssot` → FIXED.

**SOLID notes:** OCP via trait default; SRP keeps SQL in adapter; DRY one call site.

---

## Wave 2 — Anonymous identity (#335) — P1

**Goal:** No per-browser `anon_*` growth; auth-on uses JWT; admin trust restored.

| Step | Change | Files |
|------|--------|-------|
| 2.1 | Shared per-tenant guest ensure; stop minting client UUID | `identity_storage.rs`, `postgres_user_bootstrap.rs` |
| 2.2 | Bind JWT subject → `TenantContext.user_id` when auth on | `middleware.rs` / auth extractors |
| 2.3 | FE sync `localStorage.userId` to authenticated user | `client-context.ts`, login/logout hooks |
| 2.4 | `EDGEQUAKE_ALLOW_ANONYMOUS` (default: allow shared guest when auth off) | auth/config + bootstrap |
| 2.5 | Admin list default excludes anonymous; optional include + Guest label | `user_management` API + Users UI |
| 2.6 | Retarget SPEC-027 contract pin | `spec027_api_contract.rs` |

**Exit:** `iss087_anon_mint`, `iss087_jwt_userid`, `iss087_admin_anon_filter`, `iss087_allow_anonymous_flag` → FIXED.

---

## Wave 3 — Proof + ops

| Step | Change |
|------|--------|
| 3.1 | E2E matrix in [`e2e/README.md`](e2e/README.md) green |
| 3.2 | Orphan cleanup playbook executed/documented (`iss087_anon_cleanup`) |
| 3.3 | Post comments from [`07-github-issue-comments.md`](07-github-issue-comments.md) on #334 / #335 |
| 3.4 | Update register counts; link from release notes if cutting a version |

---

## Ordering rationale

1. **#334 first** — P0 product break at scale; smaller blast radius (storage trait + one handler).  
2. **#335 second** — identity touches auth middleware + FE + admin; needs careful guest UUID pin.  
3. **Cleanup last** — requires shared guest to exist before reassigning orphans.

---

## Risk register

| Risk | Mitigation |
|------|------------|
| Conversation ownership changes under shared guest | Document: auth-off chats are shared-tenant guest owned |
| SPEC-027 pin fails CI | Update pin in same PR as helper rename |
| `embedding_count` meaning changes | Contract pin + tooltip/docs |
| Birthday collision on `anon_{8}` during migration | Cleanup collapses to guest; stop creating short-prefix usernames |
