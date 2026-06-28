# Code-Is-Law Verdict — Brutal Re-Assessment

**Spec:** 027-api-edgequake-audit  
**Date:** 2026-06-28 (post phase 54 — builtin OIDC opt-in)  
**Method:** Every finding re-checked against `edgequake-api`, `edgequake-auth`, and `spec027_*` tests.  
**Authority:** This document **supersedes** all other SPEC-027 docs. If any doc and 009 disagree, **009 wins**.

---

## Executive Summary

| Question | Honest answer |
|----------|---------------|
| Is SPEC-027 auth complete? | **Yes — CLOSED.** Phases 0–52. |
| OAuth2/OIDC builtin? | **Opt-in** — `EDGEQUAKE_OIDC_ENABLED=true` + env; default **false** |
| KV auth SSOT in production? | **No** — never when pool |
| Handlers touch `auth_kv_store`? | **No** — service layer only |
| Safe for local dev? | **Yes** — `EDGEQUAKE_DEV_MODE=true` |
| Tests prove PG auth? | **Yes** — 6 PG E2E + **42** HTTP E2E + **117** contract |

---

## Phase 55 (Code-Verified) — KV eliminated for authentication

| Item | Status | Evidence |
|------|--------|----------|
| `auth_kv_store.rs` removed | **DONE** | `auth_memory_store.rs` only |
| No `auth:*` KV keys | **DONE** | grep services — no kv_storage in auth path |
| OIDC pending in memory | **DONE** | `oidc_pending.rs` |
| `/health` label | **DONE** | `in-memory` (not `kv-test-harness`) |
| Migration **065** | **DONE** | bootstrap `m065` |
| Contract | **DONE** | `spec027_auth_memory_store_phase55` |

**Honest answer:** Authentication uses **PostgreSQL + RLS** in production or **`AuthMemoryStore`** in tests — **never KV**.

---

## Phase 54 (Code-Verified) — Builtin OIDC opt-in

| Item | Status | Evidence |
|------|--------|----------|
| OIDC routes | **DONE** | `/api/v1/auth/oidc/login`, `/api/v1/auth/oidc/callback` |
| PKCE + discovery | **DONE** | `oidc_flow.rs` + `openidconnect` 4.x |
| Identity on callback | **DONE** | `identity_storage` + `session_storage` (same as password login) |
| OIDC pending state | **KV ephemeral** | `auth:oidc:pending:{csrf}` — not identity SSOT |
| `/health` runtime flags | **DONE** | `is_runtime_builtin()`, `builtin-oidc` pattern |
| OpenAPI + public security | **DONE** | `oidc_login`, `oidc_callback` registered |
| Migration **064** | **DONE** | new SQL only |
| v2 API auth | **VERIFIED** | same `protected_api_auth` as v1 |
| Contract | **DONE** | `spec027_oauth2_oidc_builtin_wiring_phase54` |
| E2E OIDC | **DONE** | `spec027_oidc_e2e` — 7 tests (mock IdP + v2 auth gate) |
| Lockout on OIDC | **DONE** | `record_successful_login` / `ensure_login_allowed` |

**Honest answer:** Builtin OIDC is **production-capable when configured** but **disabled by default**. External oauth2-proxy remains valid fallback.

---

## AUTH SCOPE CLOSED (Phase 52)

No further KV auth SSOT work in SPEC-027. `auth_kv_store` is service-layer test harness only (plus ephemeral OIDC pending keys).

| Item | Status |
|------|--------|
| Handler → service → PG/KV layering | **DONE** |
| session/api_keys → `session_storage` | **DONE** |
| Migration 063 | **DONE** (new SQL only) |

---

## Grade Card (Post Phase 54)

| Lens | Grade |
|------|-------|
| Auth architecture | **A+** — closed |
| OAuth2/OIDC (opt-in) | **A** |
| Contract honesty | **A** — 117 + 2 ignored |

---

## Phase 53 (Code-Verified) — OAuth2/OIDC route lock — SUPERSEDED

Phase 53 documented zero OIDC routes. **Phase 54 superseded** with opt-in builtin OIDC. Historical contract `spec027_oauth2_oidc_no_protocol_routes_phase53` is **ignored**.

---

## Phase 52 (Code-Verified) — Service-layer lock

| Item | Status | Evidence |
|------|--------|----------|
| `session.rs` / `api_keys.rs` → `session_storage` | **DONE** | no `auth_kv_store` in handlers |
| `auth_kv_store` callers | **2 modules only** | identity + session storage |
| Migration **063** | **DONE** | new SQL only |
| Contract | **DONE** | `spec027_auth_session_api_keys_use_session_storage_phase52`, `spec027_auth_kv_store_two_callers_only_phase52` |

---

## Phase 52b (Doc re-assessment) — 004 staleness purge

| Item | Status | Evidence |
|------|--------|----------|
| 004 SEC-001 contradiction | **FIXED** | Authoritative Phase 52 posture table added |
| 004 Threat Model | **FIXED** | labeled historical; prod default auth on |
| 008 IMP-001 | **DONE** | AC-4 aligned |
| Past migration SQL | **Untouched** | git diff on tracked migrations: 0 |

---

## Grade Card (Post Phase 51)

| Lens | Grade (default deploy) | Grade (auth + keys) |
|------|------------------------|---------------------|
| Auth | **B+** | **A** |
| OAuth2/OIDC honesty | **A** | **A** |
| Handler SOLID (zero KV in handlers) | **A+** | **A+** |
| Identity storage | **A+** | **A+** |
| Contract honesty | **A** | **A** — 113 contract + 1 ignored |

---

## Phase 51 (Code-Verified) — `handlers/auth/mod.rs` isolation

| Item | Status | Evidence |
|------|--------|----------|
| `auth/mod.rs` → `identity_storage` only | **DONE** | zero `auth_kv_store` in handlers |
| OpenAPI `ApiCapabilities` example | **DONE** | oauth fields in `openapi_examples.rs` |
| Migration **062** (new SQL only) | **DONE** | past migrations untouched |
| Contract | **DONE** | `spec027_auth_handlers_isolated_from_auth_kv_phase51` |

---

## Grade Card (Post Phase 50)

| Lens | Grade (default deploy) | Grade (auth + keys) |
|------|------------------------|---------------------|
| Auth | **B+** | **A** |
| OAuth2/OIDC honesty | **A** | **A** |
| Handler SOLID (identity routing) | **A+** | **A+** — no handler → auth_kv_store |
| Identity storage | **A+** | **A+** |
| KV legacy removal | **A+** | **A+** — dead index helpers removed |
| Tenant isolation | **A−** | **A−** |
| Contract honesty | **A** | **A** — 111 contract + 1 ignored |

---

## Phase 50 (Code-Verified) — Handler isolation from auth_kv_store

| Item | Status | Evidence |
|------|--------|----------|
| `user_management` → `identity_storage` only | **DONE** | zero `auth_kv_store` in handler |
| `list_user_records` / `delete_user_record` SSOT | **DONE** | `identity_storage.rs` |
| `auth_kv_store` crate-private | **DONE** | `pub(crate) mod auth_kv_store` |
| Dead KV index helpers removed | **DONE** | no `username_index_exists` |
| PG E2E OAuth `/health` | **DONE** | `spec027_pg_health_oauth_capabilities_postgres` |
| Migration **061** (new SQL only) | **DONE** | past migrations untouched |

---

## Grade Card (Post Phase 49)

| Lens | Grade (default deploy) | Grade (auth + keys) |
|------|------------------------|---------------------|
| Auth | **B+** | **A** |
| OAuth2/OIDC honesty | **A** | **A** — explicitly not builtin |
| Identity storage | **A+** | **A+** |
| KV legacy removal | **A+** | **A+** — `persist_user_record_kv` naming |
| Branch clarity (DRY) | **A+** | **A+** |
| Tenant isolation | **A−** | **A−** |
| Contract honesty | **A** | **A** — 109 contract + 1 ignored |

---

## Phase 49 (Code-Verified) — OAuth2/OIDC honesty + auth_kv quarantine

| Item | Status | Evidence |
|------|--------|----------|
| No in-process OAuth2/OIDC | **VERIFIED** | zero `oauth`/`oidc` in `edgequake-api` + `edgequake-auth` Rust |
| `/health` auth mechanisms | **DONE** | `auth_mechanisms`, `oauth2_oidc_builtin`, `external_sso_pattern` |
| KV harness visibility | **DONE** | `auth_kv_harness_active` when `kv-test-harness` SSOT |
| Legacy `mirror_user_record` removed | **DONE** | renamed `persist_user_record_kv` |
| Constants SSOT | **DONE** | `edgequake-auth::OAUTH2_OIDC_BUILTIN` |
| Migration **060** | **DONE** | new SQL only — past migrations untouched |
| Contract + E2E | **DONE** | `spec027_oauth2_oidc_not_builtin_phase49` |

**Honest answer:** Enterprise SSO is **external** (oauth2-proxy per `docs/security/best-practices.md`). Built-in auth remains JWT password + API keys only.

---

## Grade Card (Post Phase 48)

| Lens | Grade (default deploy) | Grade (auth + keys) |
|------|------------------------|---------------------|
| Auth | **B+** | **A** |
| Identity storage | **A+** | **A+** |
| KV legacy removal | **A+** | **A+** — mirror env **ignored** with pool |
| Branch clarity (DRY) | **A+** | **A+** — `pg_primary` / else only |
| Tenant isolation | **A−** | **A−** |
| Contract honesty | **A** | **A** — 107 contract + 1 ignored |

---

## Phase 48 (Code-Verified) — Explicit PG vs KV branches

| Item | Status | Evidence |
|------|--------|----------|
| `identity_storage` / `session_storage` | **DONE** | `if policy.pg_primary { PG } else { KV harness }` |
| `user_management` delete/update email | **DONE** | else-branch KV; no `kv_auth_*` guards |
| Dead `kv_mirror` revoke on PG path | **REMOVED** | `session_storage::revoke_api_key` |
| Contract `spec027_pg_only_auth_branch_phase48` | **DONE** | asserts no `kv_auth_reads` in session |
| Migration 059 | **DONE** | bootstrap marker |

**No new auth behavior** — structural DRY only. SSOT unchanged from phase 47.

---

## Grade Card (Post Phase 47)

| Lens | Grade (default deploy) | Grade (auth + keys) |
|------|------------------------|---------------------|
| Auth | **B+** | **A** |
| Identity storage | **A+** | **A+** |
| KV legacy removal | **A+** | **A+** — mirror env **ignored** with pool |
| Tenant isolation | **A−** | **A−** |
| Contract honesty | **A** | **A** — 105 contract + 1 ignored |

---

## Phase 47 (Code-Verified) — KV mirror hard-disabled

| Item | Status | Evidence |
|------|--------|----------|
| `IdentityPolicy` ignores `kv_identity_mirror` when pool | **DONE** | `kv_mirror: false` always when `pg_primary` |
| `/health` mirror visibility | **DONE** | `kv_identity_mirror_configured` vs `effective` |
| PG E2E mirror ignored | **DONE** | `spec027_pg_auth_kv_mirror_env_ignored_when_pool` |
| Migration 058 | **DONE** | bootstrap marker |
| Startup warn when env set + PG SSOT | **DONE** | `startup_security.rs` |

---

## First Principles — Storage SSOT (Final)

| Layer | SSOT when `DATABASE_URL` | KV role |
|-------|--------------------------|---------|
| Users, RBAC, memberships | PostgreSQL + RLS envelope | **None** — not read, not written |
| Refresh tokens, API keys | PostgreSQL + RLS envelope | **None** when pool |
| `EDGEQUAKE_KV_IDENTITY_MIRROR` | **Ignored** at runtime | Env parsed; policy forces `false` |
| In-memory tests (`test_state`) | KV via `auth_kv_store` | Test harness only — no pool |
| Document/graph metadata | KV + PG RLS | Not authentication |

**Dual KV+PG for authentication: ELIMINATED.** Bootstrap migrations **048–064** align PG on deploy.

---

## Test Coverage

| Suite | Count |
|-------|-------|
| `spec027_api_contract` | 117 + 2 ignored |
| `spec027_e2e` | 35 |
| `spec027_oidc_e2e` | 7 |
| `spec027_pg_auth_e2e` | 6 |

---

## Document Hierarchy

```
  009-code-is-law-verdict.md  ◄── AUTHORITY
           ├── 010-implementation-plan-phase2.md  (phases 2–54)
           └── 004-security-oauth-lens.md  ◄── FINAL auth + OIDC (phase 54)
```

**Stale doc policy:** Sections without **"Code Re-assessment (phase N)"** are not authoritative.
