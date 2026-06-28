# Security & OAuth Expert Lens

**Date:** 2026-06-28 (post phase 54 — builtin OIDC opt-in)  
**Cross-ref:** [006-rust-architecture-lens.md](./006-rust-architecture-lens.md) | [008-improvement-plan-ascending.md](./008-improvement-plan-ascending.md)  
**Crates:** `edgequake-api`, `edgequake-auth`, `edgequake-rate-limiter`, `edgequake-storage/rls`

---

## Verdict: B+ (default post-44) / A (auth + keys) / **A+ (auth architecture closed post-52)** / **A (OIDC opt-in post-54)**

EdgeQuake has **production-grade auth** when configured: JWT, Argon2, RBAC, PostgreSQL identity SSOT, RLS envelope, secure-by-default with DEV_MODE opt-out.

**OAuth2/OIDC:** **Opt-in builtin** when `EDGEQUAKE_OIDC_ENABLED=true` + issuer/client/redirect env vars — Authorization Code + PKCE via `openidconnect` 4.x (`GET /api/v1/auth/oidc/login`, `GET /api/v1/auth/oidc/callback`). Default deploy: `oauth2_oidc_builtin: false`; enterprise SSO may still use external **oauth2-proxy**.

**Brutal honesty (phase 52 — AUTH SCOPE CLOSED):** Phases 0–52 complete. Handlers route **only** through `identity_storage` + `session_storage`. `auth_kv_store` is `pub(crate)` and reachable **only** from those two service modules — never from handlers, never in production PG deploy.

---

## FINAL — Auth architecture (code is law)

```
handlers/auth/{mod,user_management,session,api_keys,oidc}
        │
        ├── identity_storage  ──► PG + RLS (pool)  OR  auth_kv_store (test harness)
        └── session_storage   ──► PG + RLS (pool)  OR  auth_kv_store (test harness)

OAuth2/OIDC (opt-in): GET /api/v1/auth/oidc/* → IdP PKCE → EdgeQuake JWT (same session path as password login)
External fallback: oauth2-proxy when EDGEQUAKE_OIDC_ENABLED=false
```

| Concern | Production SSOT | KV role |
|---------|-----------------|---------|
| Users, RBAC, memberships | PostgreSQL + RLS | **None** |
| Refresh tokens, API keys | PostgreSQL + RLS | **None** |
| Tenant / workspace | PostgreSQL `tenants`, `workspaces`, `memberships` | N/A |
| OIDC pending CSRF/PKCE state | `AuthMemoryStore` (in-process) | **Never KV** |
| OAuth2/OIDC protocol | **Builtin when env enabled** | N/A |
| In-memory CI (`test_state`) | `AuthMemoryStore` on `StorageRuntime` | **Never KV `auth:*`** |

**Dual KV+PG for authentication: ELIMINATED (phase 55).** KV is not used for users, sessions, API keys, or OIDC pending. Migrations **048–065**.

---

## FINAL — OAuth2/OIDC (code is law — phase 54)

*Brutal honesty: EdgeQuake implements **OIDC Authorization Code + PKCE** when operators opt in. Compile-time `OAUTH2_OIDC_BUILTIN` remains `false`; runtime `/health` reflects env-driven builtin OIDC.*

### What exists (protocol + honesty)

| Surface | Field / constant | Default deploy | OIDC enabled (`EDGEQUAKE_OIDC_*`) |
|---------|------------------|----------------|-----------------------------------|
| `edgequake-auth::config` | `OAUTH2_OIDC_BUILTIN` | `false` (compile-time) | unchanged |
| `OidcConfig::is_runtime_builtin()` | runtime gate | `false` | `true` when env valid |
| `/health` → `capabilities` | `oauth2_oidc_builtin` | `false` | `true` |
| | `auth_mechanisms` | `jwt_password`, `api_key` | + `oidc` |
| | `external_sso_pattern` | `oauth2-proxy` | `builtin-oidc` |
| Routes | `/api/v1/auth/oidc/login` | registered; 503 if disabled | redirect to IdP |
| | `/api/v1/auth/oidc/callback` | registered; 503 if disabled | JWT + refresh (PG SSOT) |
| Crate | `openidconnect` 4.x in `edgequake-api` | present | used at runtime |
| Migration **064** | marker + users comment | bootstrap apply | documents opt-in OIDC |
| Contract | `spec027_oauth2_oidc_builtin_wiring_phase54` | **DONE** | |
| E2E | `spec027_oidc_e2e` (7 tests) | **DONE** | mock IdP wiremock roundtrip |
| Lockout DRY | OIDC uses `login_lockout` | **DONE** | same as password login |

### Env vars (runtime OIDC)

| Variable | Required when enabled | Purpose |
|----------|----------------------|---------|
| `EDGEQUAKE_OIDC_ENABLED` | yes | `true` to activate routes |
| `EDGEQUAKE_OIDC_ISSUER_URL` | yes | IdP issuer (discovery) |
| `EDGEQUAKE_OIDC_CLIENT_ID` | yes | OAuth client id |
| `EDGEQUAKE_OIDC_CLIENT_SECRET` | optional | confidential clients |
| `EDGEQUAKE_OIDC_REDIRECT_URI` | yes | callback URL |
| `EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL` | optional | frontend redirect with tokens in query |

### What does NOT exist (honest limits)

| Capability | Status |
|------------|--------|
| Generic `/auth/oauth*` routes | **None** — only `/auth/oidc/*` |
| EdgeQuake as OIDC **provider** | **Not implemented** |
| OAuth2 token introspection endpoint | **Not implemented** |
| Social login without IdP config | **Not implemented** |

### Built-in auth routes

| Route | Purpose |
|-------|---------|
| `POST /api/v1/auth/login` | JWT password login |
| `POST /api/v1/auth/refresh` | Refresh token rotation |
| `POST /api/v1/auth/logout` | Revoke refresh token |
| `GET /api/v1/auth/me` | Current user |
| `GET /api/v1/auth/oidc/login` | OIDC redirect (opt-in) |
| `GET /api/v1/auth/oidc/callback` | OIDC callback → JWT |
| `POST/GET/PATCH/DELETE /api/v1/users/*` | User admin (JWT) |
| `GET /api/v1/api-keys` | List API keys |

### Enterprise SSO deployment (first principles)

**Option A — Builtin OIDC (phase 54):**

```
Browser → GET /api/v1/auth/oidc/login
              ↓ PKCE + discovery
         IdP (Google/Okta/Keycloak)
              ↓ authorization code
         GET /api/v1/auth/oidc/callback
              ↓ identity_storage (PG + RLS) + session_storage
         EdgeQuake JWT + refresh token
```

**Option B — External proxy (default):**

```
Browser → IdP → oauth2-proxy (EXTERNAL_SSO_PATTERN)
              ↓
         EdgeQuake API middleware (JWT / API key)
              ↓
         PostgreSQL identity SSOT + RLS
```

**Ascending-compat:** OIDC routes are additive; v1 password login unchanged. v2 API uses same `protected_api_auth` middleware as v1.

**Grade:** **A** — opt-in protocol + operator discovery + contract/OpenAPI parity.

---

## Code Re-assessment (phase 54) — superseded sections below

Sections above through **FINAL — OAuth2/OIDC (phase 54)** are authoritative. Older "OAuth not implemented" claims in this file are **historical** unless marked phase 54+.

---

## FINAL — OAuth2/OIDC (phase 53 — SUPERSEDED by phase 54)

*Historical: phase 53 locked zero OIDC routes. Phase 54 added opt-in builtin OIDC. See phase 54 section above.*


## First Principles — Service-layer SSOT lock (Phase 52)

| Item | Code is law |
|------|-------------|
| `session.rs` | `session_storage` + `identity_storage` only |
| `api_keys.rs` | `session_storage` only |
| `auth_kv_store` callers | **Only** `identity_storage.rs` + `session_storage.rs` |
| Migration | **063** (new SQL only) |
| Contract | `spec027_auth_session_api_keys_use_session_storage_phase52`, `spec027_auth_kv_store_two_callers_only_phase52` |

**Honest answer:** AUTH engineering scope for SPEC-027 is **closed**. Further OIDC work is a **new feature**, not audit debt.

---

## First Principles — Full handler layer isolation (Phase 51)

| Item | Code is law |
|------|-------------|
| `handlers/auth/mod.rs` | `identity_storage::{load,find,persist}_*` only |
| `handlers/auth/user_management.rs` | `identity_storage` only (phase 50) |
| `handlers/` grep `auth_kv_store` | **Zero** (health_types doc comment only) |
| OpenAPI example | `ApiCapabilities` includes oauth + auth_mechanisms |
| Migration | **062** (new SQL only) |

---

## First Principles — Handler isolation from auth_kv_store (Phase 50)

| Item | Code is law |
|------|-------------|
| `user_management.rs` | **Zero** `auth_kv_store` imports |
| List/delete users | `identity_storage::list_user_records`, `delete_user_record` |
| Duplicate login check | `find_user_record_by_login` (PG + KV unified) |
| `auth_kv_store` visibility | `pub(crate)` — not public API surface |
| Dead helpers removed | `username_index_exists`, `email_index_exists`, `email_index_user_id` |
| Migration | **061** (new SQL only) |
| PG E2E | `spec027_pg_health_oauth_capabilities_postgres` (6th PG test) |

**Honest answer:** SOLID boundary enforced — handlers depend on `identity_storage` abstraction; KV is implementation detail for test harness only.

---

## First Principles — OAuth2/OIDC (Phase 49)

*Code is law — **no OAuth2/OIDC protocol** in routes or auth handlers; honesty constants + `/health` only (see FINAL OAuth section above).*

| Question | Honest answer |
|----------|---------------|
| Is OAuth2/OIDC builtin? | **No** — `OAUTH2_OIDC_BUILTIN = false` |
| What auth exists? | `jwt_password` (login + refresh) + `api_key` |
| Enterprise SSO path? | External **oauth2-proxy** (`EXTERNAL_SSO_PATTERN`) |
| `/health` signals | `auth_mechanisms`, `oauth2_oidc_builtin`, `external_sso_pattern` |
| Future OIDC in EdgeQuake? | **Out of SPEC-027 scope** — roadmap articles only |

### Recommended deployment pattern (external SSO)

```
User → oauth2-proxy (OIDC) → EdgeQuake API (JWT/API-key middleware)
```

EdgeQuake validates **its own** JWTs and stored API keys. The proxy handles IdP login; EdgeQuake does **not** implement authorization-code flow, token exchange, or OIDC discovery.

| Item | Code is law |
|------|-------------|
| Constants | `edgequake-auth::BUILTIN_AUTH_MECHANISMS`, `OAUTH2_OIDC_BUILTIN` |
| Docs reference | `docs/security/best-practices.md`, `docs/faq.md` |
| Migration | **060** — table comment on `users` (**new SQL only**) |
| Contract | `spec027_oauth2_oidc_not_builtin_phase49`, `spec027_oauth2_oidc_no_protocol_routes_phase53` |

**Grade:** **A** for honesty (explicit false + operator fields). Missing in-process OIDC is **not** a defect for SPEC-027.

---

## First Principles — auth_kv_store quarantine (Phase 49)

| Item | Code is law |
|------|-------------|
| Module | `services/auth_kv_store.rs` — **in-memory test harness only** |
| User persist KV | `persist_user_record_kv` (renamed from legacy `mirror_user_record`) |
| Production PG deploy | Module **not called** when `IdentityPolicy::pg_primary` |
| `/health` | `auth_kv_harness_active: true` only when `auth_identity_ssot == kv-test-harness` |
| KV key prefixes | `auth:user:*`, `auth:refresh_token:*`, `auth:api_key:*` |

**Honest answer:** Not dual-storage — sole auth backend for `AppState::test_state()` (no PG pool).

---

## Implementation map (code is law — phase 48)

| Concern | Module | PG path | KV test-harness path |
|---------|--------|---------|----------------------|
| `IdentityPolicy` | `services/identity_storage.rs` | `pg_primary` when pool + `pg_identity_ssot` | `!pg_primary` → KV |
| User CRUD SSOT | `identity_storage.rs` | `*_pg` + RLS envelope | `auth_kv_store::*` |
| Refresh tokens | `session_storage.rs` | `*_pg` | `auth_kv_store::*` |
| API keys | `session_storage.rs` | `*_pg` | `auth_kv_store::*` |
| User admin handlers | `handlers/auth/user_management.rs` | via `identity_storage` only | **no direct KV** |
| KV quarantine | `services/auth_kv_store.rs` (`pub(crate)`) | **not called** when pool | sole SSOT without pool |
| RLS envelope | `services/tenant_isolation.rs` | `acquire_optional_pg_connection` | N/A |
| Bootstrap | `state/migration_bootstrap/` | migrations **048–063** | reconcile on deploy |
| Health ops | `services/health_schema.rs` | global `_sqlx_migrations` | no tenant RLS |
| Operator signals | `handlers/health_types.rs` | `auth_identity_ssot`, mirror flags | `/health` |

---

## First Principles — User, RBAC, Tenant, Workspace (Phase 48 SSOT)

*Authoritative storage map — code is law. Supersedes all prior dual-storage narratives.*

| Concern | SSOT (postgres deploy) | Isolation | KV role |
|---------|------------------------|-----------|---------|
| **Users** (credentials, lockout) | PostgreSQL `users` | RLS envelope + `tenant_id` column | **None** |
| **Global RBAC role** | PG `users.role` → JWT claim | Same row | **None** |
| **Tenant** | PostgreSQL `tenants` | bootstrap `ensure_default_tenant_workspace` | N/A |
| **Workspace** | PostgreSQL `workspaces` | scoped to `tenant_id` | N/A |
| **Membership / workspace rights** | PostgreSQL `memberships` | RLS envelope; `verify_membership_active` | **None** |
| **Refresh tokens** | PostgreSQL `refresh_tokens` | RLS envelope | **None** |
| **API keys** | PostgreSQL `api_keys` | RLS envelope | **None** |
| **Document/graph data** | KV metadata + PG RLS | handler filters + RLS | Not auth |

### Why not dual KV + PG for authentication?

| First principle | Verdict |
|-----------------|---------|
| Automatic bootstrap migrations **048–062** on deploy | PG schema always aligned — safe PG-only |
| `IdentityPolicy::pg_primary` when pool | KV reads **disabled** |
| Phase 47: `kv_mirror: false` always when `pg_primary` | KV writes **disabled** |
| In-memory CI (`AppState::test_state`, no pool) | KV via `auth_kv_store.rs` — **test harness only** |

**Honest answer:** Dual storage for authentication was legacy from pre-PG era. It is **eliminated** in production. Removing `auth_kv_store` entirely would break in-memory E2E without `DATABASE_URL` — quarantined, not deleted.

### RLS by default

- `EDGEQUAKE_PG_RLS_ENABLED=true` (default) — identity/session/pdf use `acquire_optional_pg_connection`
- Auth tables use tenant column filters + session context envelope (defense-in-depth)
- Global ops (`_sqlx_migrations`) — `health_schema.rs`; no tenant RLS by design

### Operator signals (`/health` capabilities)

| Field | Meaning |
|-------|---------|
| `auth_identity_ssot` | `postgresql` or `kv-test-harness` |
| `auth_enabled` / `dev_mode` | runtime auth flags |
| `kv_identity_mirror_configured` | env value |
| `kv_identity_mirror_effective` | **always false** when PG pool SSOT |
| `auth_mechanisms` | `["jwt_password","api_key"]` |
| `oauth2_oidc_builtin` | **always false** |
| `auth_kv_harness_active` | **true** when `kv-test-harness` SSOT |
| `external_sso_pattern` | `oauth2-proxy` |

---

## First Principles — Explicit PG vs KV branches (Phase 48)

*Supersedes scattered `kv_auth_reads_enabled` / `kv_auth_writes_enabled` branching in session storage — code is law.*

| Item | Code is law |
|------|-------------|
| Branch pattern | `if policy.pg_primary { PG } else { KV harness }` |
| `session_storage` | No `kv_auth_reads_enabled()` branches |
| `user_management` | delete + email index use else-branch KV |
| `IdentityPolicy::kv_auth_*` | Retained for tests/docs; equals `!pg_primary` |
| Contract | `spec027_pg_only_auth_branch_phase48` |
| Migration | **059** |

**Honest answer:** Phase 48 is **DRY cleanup** — behavior identical to phase 47. Removes dead `kv_mirror` revoke on PG api-key path and duplicate guard logic.

---

## First Principles — KV Mirror Hard-Disabled (Phase 47)

*Supersedes phase 46 mirror policy — code is law.*

| Item | Code is law |
|------|-------------|
| `IdentityPolicy::resolve` | `kv_mirror: false` when `pg_primary` — env ignored |
| KV auth writes with pool | **Never** — mirror path unreachable |
| `/health` | `kv_identity_mirror_configured` vs `kv_identity_mirror_effective` |
| PG E2E | `spec027_pg_auth_kv_mirror_env_ignored_when_pool` |
| Migration | **058** |

**Honest answer:** Last production path for dual KV+PG auth writes is **closed**. `auth_kv_store.rs` remains only for `AppState::test_state()` (no PG pool).

---

## First Principles — Legacy KV Removal (Phase 46)

*Supersedes phase 45 for KV mirror policy — code is law.*

| Item | Code is law |
|------|-------------|
| `EDGEQUAKE_KV_IDENTITY_MIRROR` | **Deprecated** — startup warns; migration **057** |
| KV auth reads (postgres) | **Never** when pool + `pg_identity_ssot` |
| KV auth writes (postgres) | **Never** when pool — **superseded by phase 47** |
| `auth_kv_store.rs` | Single module — test harness + deprecated mirror |
| Health schema ops | `health_schema.rs` — `_sqlx_migrations` global (no RLS) |
| `/health` E2E | Reports `auth_identity_ssot`, `auth_enabled`, `dev_mode` |

**First Principles verdict on dual KV+PG:** **Rejected for authentication.** Automatic bootstrap migrations 048–057 align PG on deploy. KV remains for **in-memory E2E** (`test_state`) and document metadata — not identity.

**Honest answer:** Legacy KV auth paths are not deleted (tests need them) but are **quarantined** in one module and **never consulted** for reads in production postgres mode.

---

## First Principles — KV Auth Consolidation (Phase 45)

*Supersedes scattered KV helpers in handlers — code is law.*

| Item | Code is law |
|------|-------------|
| KV auth SSOT module | `services/auth_kv_store.rs` — users, refresh tokens, API keys |
| Production reads/writes | Still **PG-only** when pool (`IdentityPolicy`) |
| KV role | **Test harness** (`AppState::test_state`) + opt-in mirror |
| Operator visibility | `/health` → `capabilities.auth_identity_ssot` |
| Migration | **056** — consolidation marker |
| DRY | Handlers no longer duplicate KV prefix scans / upserts |

**Honest answer:** IMP-026 removes the last structural duplication — KV auth logic lived in 4 modules. Behavior unchanged for postgres deploy; cleaner SRP.

---

## First Principles — Secure by Default + DEV Mode (Phase 44)

*Supersedes all prior "auth off by default" statements — code is law.*

| Item | Code is law |
|------|-------------|
| Default | `AuthConfig::default().auth_enabled == true` |
| Local dev opt-out | `EDGEQUAKE_DEV_MODE=true` → auth disabled |
| Legacy opt-out | `EDGEQUAKE_AUTH_ENABLED=false` or `EDGEQUAKE_AUTH_DISABLED=true` |
| `make dev` | Sets `EDGEQUAKE_DEV_MODE=true` when `DEV_AUTH_ENABLED=false` |
| Tests | `AppState::test_state()` simulates dev mode (auth off) |
| Production | Must set `JWT_SECRET` + `EDGEQUAKE_API_KEYS` or master key |
| Migration | **055** — secure-by-default marker |

**Honest answer:** Public internet deploy without env is **no longer world-readable**. Misconfigured deploy without API keys gets startup **warning** (fatal with `EDGEQUAKE_STRICT_STARTUP=1`).

**Dual KV + PG:** PG auth SSOT when pool; KV test-harness only (`test_state`).

---

---

---

## Code Re-assessment (phase 52)

| Item | Status | Evidence |
|------|--------|----------|
| AUTH scope closed | **YES** | handlers → services → PG/KV |
| session/api_keys verified | **DONE** | contract phase 52 |
| Migration 063 (new only) | **DONE** | git verified |
| OAuth2/OIDC | **Not builtin** | unchanged |

**Grade:** A+ auth architecture **closed**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 52b — doc purge)

| Item | Status | Evidence |
|------|--------|----------|
| SEC-001 stale "default open" text | **REMOVED** | Authoritative posture table |
| Threat Model default branch | **FIXED** | prod auth on |
| PG E2E in security matrix | **ADDED** | 4 pg auth tests listed |
| 008 IMP-001 | **DONE** | AC-4 |

**Grade:** Documentation honesty **A** (no code change).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 53 — OAuth route lock)

| Item | Status | Evidence |
|------|--------|----------|
| FINAL OAuth2/OIDC section | **DONE** | honesty vs protocol tables |
| No oauth/oidc routes | **VERIFIED** | `spec027_oauth2_oidc_no_protocol_routes_phase53` |
| Mechanisms | **jwt_password + api_key only** | `BUILTIN_AUTH_MECHANISMS` slice |
| Migration | **None** | 060 sufficient |

**Grade:** OAuth honesty **A+**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 51)

| Item | Status | Evidence |
|------|--------|----------|
| Zero `auth_kv_store` in `handlers/` | **DONE** | grep handlers (doc comment only in health_types) |
| `auth/mod.rs` identity SSOT | **DONE** | `identity_storage::*` |
| OpenAPI example oauth fields | **DONE** | `openapi_examples.rs` |
| Migration 062 (new only) | **DONE** | git verified |

**Grade:** B+ default / A auth+keys / **A+ handler isolation complete**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 50)

| Item | Status | Evidence |
|------|--------|----------|
| `user_management` isolated from KV | **DONE** | contract grep zero `auth_kv_store` |
| `identity_storage` list/delete SSOT | **DONE** | `list_user_records`, `delete_user_record` |
| `pub(crate) mod auth_kv_store` | **DONE** | `services/mod.rs` |
| PG OAuth `/health` E2E | **DONE** | 6 PG tests |
| Migration 061 (new only) | **DONE** | git: no edits to past SQL |

**Grade:** B+ default / A auth+keys / **A+ handler SOLID**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 49)

| Item | Status | Evidence |
|------|--------|----------|
| OAuth2/OIDC not in Rust crates | **VERIFIED** | grep zero in api + auth |
| `/health` OAuth fields | **DONE** | `health_types.rs` + E2E |
| `persist_user_record_kv` rename | **DONE** | no `mirror_user_record` |
| Migration 060 (new only) | **DONE** | past SQL untouched |
| Auth/identity scope | **COMPLETE** | 009 authority |

**Grade:** B+ default / A auth+keys / **A OAuth honesty**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

| Item | Status | Evidence |
|------|--------|----------|
| KV mirror hard-disabled with pool | **DONE** | `IdentityPolicy::resolve` → `kv_mirror: false` |
| `/health` mirror configured vs effective | **DONE** | `health_types.rs` |
| PG E2E env ignored | **DONE** | `spec027_pg_auth_kv_mirror_env_ignored_when_pool` |
| Migration 058 | **DONE** | bootstrap m058 |
| Auth/identity scope | **COMPLETE** | no KV read/write when pool |

**Grade:** B+ default / A auth+keys / **A+ identity SSOT**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Historical archive (phases 9–46)

Sections below document the **evolution** of auth hardening. **Only** sections tagged **"Code Re-assessment (phase N)"** at the bottom and the **FINAL architecture blocks at the top of this document** are authoritative for current deploy behavior. Pre-phase-44 statements claiming `auth_enabled: false` default or **"Default deploy: F"** are **historical** — superseded by AC-4 (phase 44) and phase 52 PG SSOT.

**Do not cite** the Threat Model or Critical Findings blocks in the next section for production posture — they are **pre-phase-44 snapshots** retained for audit trail only.

---

## Authoritative Security Posture (Phase 52 — code is law)

| ID | Status | Honest answer (2026-06-28) |
|----|--------|----------------------------|
| SEC-001 | **FIXED** (phase 44) | `AuthConfig::default().auth_enabled == true`; opt-out via `EDGEQUAKE_DEV_MODE` or explicit disable env |
| SEC-002 | **FIXED** | Stored API keys validated when auth on (default) |
| SEC-003 | **FIXED** | `ApiRequireAdmin` when auth on; bypass only in dev_mode |
| SEC-004 | **OPT-IN** | Strict tenant bind + membership verify |
| SEC-005 | **OPEN** | Default JWT secret still insecure until env set |
| SEC-006–009 | **OPT-IN** | Ollama, WS, CORS, rate limit — env gated |
| SEC-011 | **FIXED** | Login lockout 423 when auth on |
| SEC-014 | **IMPROVED** | RLS acquire SSOT; identity/session PG envelope |
| OAuth2/OIDC | **N/A (external)** | `OAUTH2_OIDC_BUILTIN = false`; use oauth2-proxy |
| KV auth SSOT | **ELIMINATED** (prod) | PG + RLS when pool; `auth_kv_store` test harness only |

```88:89:edgequake/crates/edgequake-auth/src/config.rs
            auth_enabled: true,
            dev_mode: false,
```

**Local dev:** `make dev` sets `EDGEQUAKE_DEV_MODE=true` when `DEV_AUTH_ENABLED=false`. **CI/tests:** `AppState::test_state()` simulates dev mode. **Production:** auth on unless explicitly disabled.

---

## First Principles — Identity, Rights & Isolation (Phase 43)

*Supersedes Phase 42 for auth PG RLS envelope — code is law.*

| Item | Code is law |
|------|-------------|
| Identity PG SSOT | `identity_storage` — every `users`/`memberships` query uses `acquire_optional_pg_connection` |
| Session PG SSOT | `session_storage` — refresh tokens + API keys same envelope |
| Membership bind | `verify_membership_active` with `PgIsolationScope::for_membership` |
| Anonymous chat users | `ensure_anonymous_user_in_postgres` (DRY via `postgres_user_bootstrap`) |
| PG list users E2E | `spec027_pg_auth_list_users_reads_from_postgres` |
| Migration | **054** — identity PG RLS envelope marker |

**Honest answer:** Auth PG layer is now **consistent** — no raw `pool.execute` on identity/session tables. Auth tables themselves have **no table RLS** (tenant column filters); envelope sets session context for defense-in-depth and future RLS on memberships.

**Dual KV + PG:** Eliminated for auth reads in postgres mode (phase 40). KV path remains **test-only** (`AppState::test_state()`).

---

## First Principles — Identity, Rights & Isolation (Phase 42)

*Supersedes Phase 41 for handler RLS wiring on pdf_documents.*

| Item | Code is law |
|------|-------------|
| RLS on pdf fallback | `pdf_lineage::fetch_pdf_extraction_metadata` uses `acquire_optional_pg_connection` |
| Handler wired | `get_document` — no raw `pool.fetch_optional` on `pdf_documents` |
| DRY acquire/release | `tenant_isolation::acquire_optional_pg_connection` + `release_optional_pg_connection` |
| PG logout E2E | `spec027_pg_auth_logout_revokes_refresh_token_in_postgres` |
| Migration | **None** — `pdf_documents` RLS from migration 022 |

**Honest answer:** First handler-level RLS wiring for direct `sqlx` in API layer. Conversations already use `acquire_rls_connection` in `edgequake-storage`. Auth tables (`users`, `refresh_tokens`, `api_keys`) use tenant column filters — no table RLS.

**Still open:** Other handlers with direct `sqlx` (health, admin, metrics) bypass RLS — ops/admin paths.

---

## First Principles — Identity, Rights & Isolation (Phase 41)

*Supersedes Phase 40 for E2E PG auth proof.*

| Item | Code is law |
|------|-------------|
| PG auth E2E | `spec027_pg_auth_e2e.rs` — login/refresh + API key roundtrip against real PG |
| Test harness | `AppState::test_state_with_pg_pool` — memory KV for docs, PG for auth |
| RLS helper | `with_optional_pg_rls` — RLS when enabled + scope; plain acquire otherwise |
| KV mirror in PG E2E | **Disabled** — asserts no `auth:user:` / `auth:api_key:` keys in KV |

**Honest answer:** Production auth path is now **proven E2E** when `DATABASE_URL` is set. In-memory `spec027_e2e` (33 tests) still uses KV auth — valid test-only path.

---

## First Principles — Identity, Rights & Isolation (Phase 40)

*Supersedes Phase 39 for KV auth SSOT verdict — code is law.*

### Storage SSOT (postgres deploy)

| Layer | SSOT | KV role |
|-------|------|---------|
| Identity, RBAC, memberships | PG `users` + `memberships` | **None** (reads) |
| Session artifacts | PG `refresh_tokens` + `api_keys` | **None** (reads) |
| Data isolation | PG RLS + handler filters | Document metadata only |

### Dual KV + PG — First Principles Verdict (Phase 40)

**KV is not auth SSOT in production.** `IdentityPolicy::kv_auth_reads_enabled()` is **false** when pool + `EDGEQUAKE_PG_IDENTITY_SSOT=true` (defaults).

| Factor | Verdict |
|--------|---------|
| Auth reads with PG pool | **PG only** — no silent KV fallback |
| Auth writes with PG pool | **PG only** unless `EDGEQUAKE_KV_IDENTITY_MIRROR=1` |
| KV auth path | **In-memory tests only** (`AppState::test_state()`, `pg_pool: None`) |
| Bootstrap | Migrations **048–053** on every deploy |
| RLS | **Default on** |

**Honest answer:** Dual storage for **authentication is eliminated** in postgres mode. KV `auth:*` keys are legacy/mirror only — not consulted on read when pool exists. Document/graph KV metadata remains (different concern).

**Migrations:** **053** PG-only auth reads marker + table comments.

Modules: `IdentityPolicy::kv_auth_reads_enabled`, `kv_auth_writes_enabled` — contracts `spec027_pg_auth_no_kv_read_fallback_phase40`, `spec027_migration_053_pg_auth_kv_reads_removed_wired`.

---

## First Principles — Identity, Rights & Isolation (Phase 39)

*Superseded by Phase 40 for KV read fallback. Retained for session PG wiring history.*

### Storage SSOT (code is law — phase 39)

| Layer | SSOT (postgres deploy) | Fallback (no PG pool) |
|-------|------------------------|------------------------|
| **Global identity** (credentials, lockout) | PostgreSQL `users` | KV `auth:user:{id}` (E2E/tests) |
| **Global RBAC** | PG `users.role` + JWT + `RbacService` | KV user `role` |
| **Tenant/workspace membership** | PG `memberships` | — |
| **Session artifacts** | PG `refresh_tokens` + `api_keys` | KV (E2E/tests) |
| **Data isolation** | PG RLS + handler filters | KV metadata filters |

### Dual KV + PG — First Principles Verdict (Phase 39)

**PostgreSQL is auth SSOT when pool available.** Bootstrap 048–052 aligns schema; all auth writes route through `IdentityPolicy`:

| Factor | Verdict |
|--------|---------|
| Identity + session artifacts | **PG first** via `identity_storage` + `session_storage` |
| KV role | Opt-in mirror only (`EDGEQUAKE_KV_IDENTITY_MIRROR=1`) |
| Refresh tokens | SHA-256 indexed lookup in PG (`token_hash`) |
| API keys | Prefix index + Argon2 verify (unchanged algorithm) |
| RLS | **Default on** — `EDGEQUAKE_PG_RLS_ENABLED=true` |

**Honest answer:** Dual storage is **minimal** — KV remains only for E2E `test_state()` (no pool) and optional mirror. IMP-026 full KV cutover is now a **test harness** concern, not production auth.

**Migrations:** 048–051 unchanged · **052 session artifacts SSOT** (token_hash index, table comments).

Modules: `session_storage.rs` — contracts `spec027_session_storage_pg_phase39`, `spec027_migration_052_session_artifacts_ssot_wired`.

---

## First Principles — Identity, Rights & Isolation (Phase 38)

*Superseded by Phase 39 for session artifacts. Retained for identity/RBAC history.*

| Layer | SSOT (postgres deploy) | Fallback (no PG pool) |
|-------|------------------------|------------------------|
| **Global identity** (credentials, lockout) | PostgreSQL `users` | KV `auth:user:{id}` (E2E/tests) |
| **Global RBAC** | PG `users.role` + JWT + `RbacService` | KV user `role` |
| **Tenant/workspace membership** | PG `memberships` | — |
| **Session artifacts** | KV refresh tokens + API keys | unchanged v1 |
| **Data isolation** | PG RLS + handler filters | KV metadata filters |

### Dual KV + PG — First Principles Verdict (Phase 38)

**PostgreSQL is identity SSOT when pool available.** Automatic bootstrap (048–051) makes PG-primary the correct default:

| Factor | Verdict |
|--------|---------|
| `DATABASE_URL` required | Yes — no in-memory server mode |
| Bootstrap aligns PG | Migrations 048–051 on every deploy |
| Identity read/write | **PG first** via `IdentityPolicy` |
| KV role | Opt-in mirror (`EDGEQUAKE_KV_IDENTITY_MIRROR=1`); refresh/api keys remain KV |
| RLS | **Default on** — `EDGEQUAKE_PG_RLS_ENABLED=true`; opt-out with `=0` |

**Honest answer:** Dual storage is **narrowed** — identity/RBAC/membership are PG-primary; KV is no longer the auth write path in production postgres mode. Full KV removal blocked by session token storage (IMP-026).

### Three Isolation Layers

| Layer | Mechanism | Default | SSOT |
|-------|-----------|---------|------|
| **1 — App** | `isolation.rs` strict + `isolation_context.rs` legacy alias | **On** | Handler/KV metadata filters |
| **2 — Auth bind** | JWT/header merge + membership verify | Opt-in (`STRICT_TENANT_BIND`) | `middleware.rs`, `identity_storage.rs` |
| **3 — PostgreSQL RLS** | `acquire_rls_connection` on dedicated conn | **On** (`PG_RLS_ENABLED`) | `rls.rs`, `conversation.rs` |

**RLS caveat:** Superuser `DATABASE_URL` bypasses RLS. Use non-superuser app role in production.

**Migrations:** 048 lockout · 049 membership · 050 RLS verify · **051 PG identity primary**.

Modules: `identity_storage.rs` (`IdentityPolicy`), `tenant_isolation.rs`, `rls.rs` — contracts `spec027_pg_identity_ssot_phase38`, `spec027_migration_051_pg_identity_primary_wired`.

---

## First Principles — Identity, Rights & Isolation (Phase 35–37)

*Superseded by Phase 38 section above for PG-primary verdict.*

### Storage SSOT

| Layer | SSOT (code is law) | Wired to API auth? |
|-------|-------------------|-------------------|
| **Global identity** (username, password, lockout) | KV `auth:user:{id}` — API write path | ✅ login/session/user CRUD |
| **PostgreSQL `users`** | Synced via `identity_storage::sync_auth_user_to_postgres` | ✅ FK for conversations |
| **Global RBAC** (admin/user/readonly) | `edgequake_auth::RbacService` + JWT `role` claim | ✅ `require_admin_request` uses `rbac.require_role` |
| **Tenant/workspace membership** | PG `memberships` | ✅ synced on persist; verified when strict bind + PG |
| **Data isolation (KV/graph)** | KV metadata + handler filters | ✅ Layer 1 — always on |

### Dual KV + PG — First Principles Verdict

**Keep dual write in v1 (ascending-compat).** Automatic migration bootstrap (048–050) keeps PG aligned; it does **not** justify removing KV:

| Factor | KV | PG sync |
|--------|-----|---------|
| Purpose | Credential + lockout SSOT | Relational FK + RLS-ready rows |
| Write path | Auth handlers (proven) | Additive upsert on persist |
| Breaking change to remove | Yes — all auth tests + deploys | No — already additive |
| Consolidation timing | v2 after IMP-026 + cutover plan | — |

**Honest answer:** Dual storage is **intentional and managed**, not accidental drift. PG-only auth is a future breaking change, not a bootstrap quick-win.

### Three Isolation Layers

| Layer | Mechanism | Default | SSOT |
|-------|-----------|---------|------|
| **1 — App** | `isolation.rs` strict + `isolation_context.rs` legacy alias | **On** | Handler/KV metadata filters |
| **2 — Auth bind** | JWT/header merge + membership verify | Opt-in (`STRICT_TENANT_BIND`) | `middleware.rs`, `identity_storage.rs` |
| **3 — PostgreSQL RLS** | `acquire_rls_connection` on dedicated conn | Opt-in (`PG_RLS_ENABLED`) | `edgequake-storage/rls.rs`, `conversation.rs` |

**RLS caveat:** Policies exist in migration 001/009 but **superuser `DATABASE_URL` bypasses RLS**. Production must use non-superuser app role + Layer 1 regardless.

**Migrations:** 048 lockout columns · 049 membership backfill · 050 RLS function verification.

Modules: `identity_storage.rs`, `tenant_isolation.rs`, `rls.rs` — contracts `spec027_tenant_isolation_ssot_phase35`, `spec027_migration_050_pg_rls_ssot_wired`, `spec027_conversation_rls_acquired_phase36`, `spec027_rls_acquire_ssot_phase37`.

---

## First Principles — Identity & Rights Storage (Phase 33–34)

*Superseded by Phase 35 section above for isolation + dual KV+PG verdict. Retained for history.*

| Layer | SSOT (code is law) | Wired to API auth? |
|-------|-------------------|-------------------|
| **Global identity** | KV `auth:user:{id}` | ✅ |
| **PostgreSQL `users`** | Synced on persist | ✅ |
| **Global RBAC** | JWT `role` + RbacService | ✅ |
| **Memberships** | PG `memberships` | ✅ strict bind + PG |
| **Data isolation** | KV metadata + handlers | ✅ |

---

## Threat Model (ASCII) — **HISTORICAL PRE-44 SNAPSHOT**

*Superseded by Phase 44 AC-4. Production default is auth **on**; diagram below shows middleware branching only.*

```
                    Internet
                       │
                       ▼
              ┌────────────────┐
              │  Reverse Proxy │  ← oauth2-proxy for enterprise OIDC (external)
              │  (optional)    │     EdgeQuake: JWT + API keys only
              └───────┬────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│ edgequake-api                                               │
│                                                             │
│  PUBLIC (no auth middleware path):                          │
│    /health /metrics /ws/* /api/generate /api/chat           │
│    /swagger-ui                                              │
│                                                             │
│  /api/v1/* ──► protected_api_auth                           │
│                  │                                          │
│                  ├─ dev_mode / auth off ──► OPEN (local CI) │
│                  └─ auth on (PROD DEFAULT) ──► token check  │
│                         validate_presented_token SSOT       │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
              PostgreSQL + RLS envelope (default on)
```

---

## OAuth / OIDC Assessment

| OAuth2 Flow | Implemented | Notes |
|-------------|-------------|-------|
| Authorization Code + PKCE | ❌ | Use external proxy |
| Client Credentials | ❌ | Static env API keys only |
| Device Code | ❌ | — |
| Refresh Token (native) | ✅ | UUID in KV, no rotation |
| OIDC Discovery | ❌ | — |
| Token introspection | ❌ | JWT verify only |

**SEC-OAUTH-001 | P2 | No first-class SSO** — acceptable for self-hosted; not for SaaS without integration plan.

**Ascending path:** OIDC via `openidconnect` crate as **optional feature** — new `/auth/oidc/callback` routes; v1 JWT login unchanged.

---

## Critical Findings — **HISTORICAL AUDIT SNAPSHOT (pre-phase 44)**

*For current status see **Authoritative Security Posture (Phase 52)** above. SEC-001 default-open narrative is **obsolete**.*

### SEC-001 | P0 | Auth disabled by default — **FIXED phase 44 (AC-4)**

**Historical claim (wrong today):** `auth_enabled: false` in `Default`.

**Current code:** `auth_enabled: true` in `AuthConfig::default()`; `resolve_auth_enabled_from_env` returns true unless `EDGEQUAKE_DEV_MODE` or explicit disable env.

Handlers upgrade to Admin when auth off (`handlers/auth/mod.rs`) — **intentional for local dev only**.

**E2E proof (dev mode):** `spec027_admin_accessible_without_auth_when_auth_disabled` — not production default.

---

### SEC-002 | P0 | Stored API keys — FIXED when auth on

Middleware delegates to `services/auth_validation::validate_presented_token` which calls `validate_stored_api_key` for KV-hashed keys.

**Status:** **OPT-IN** — works when `EDGEQUAKE_AUTH_ENABLED=true`.

**E2E proof:** `spec027_stored_api_key_authenticates_when_auth_enabled`

---

### SEC-003 | P0 | Admin routes — FIXED when auth on

`ApiRequireAdmin` extractor wired on admin handlers (phase 26). Bypass when auth off (by design for local dev).

**E2E proof:** `spec027_non_admin_receives_403_on_admin_endpoint` (auth enabled)

---

### SEC-004 | P0 | Spoofable tenant context — PARTIAL (memberships phase 34)

`TenantContext` from client headers (`X-Tenant-ID`, `X-Workspace-ID`). JWT claims now include default tenant/workspace on login/refresh (`access_token_claims`).

**Status:** **OPT-IN** — `EDGEQUAKE_STRICT_TENANT_BIND=true` rejects header/JWT mismatch; with PostgreSQL also verifies active `memberships` row.

**E2E proof:** `spec027_strict_tenant_bind_rejects_header_jwt_mismatch`

---

### SEC-005 | P0 | Default JWT secret

```6:6:edgequake/crates/edgequake-auth/src/config.rs
pub const DEFAULT_INSECURE_JWT_SECRET: &str = "change-me-in-production-256-bit-secret-key";
```

**Status:** **OPEN** — strict startup warns/exits when unset in production profile.

---

### SEC-006 | P1 | Ollama shim unauthenticated

`/api/generate`, `/api/chat` — LLM proxy without auth when compat enabled.

**Status:** **OPT-IN** — `EDGEQUAKE_ENABLE_OLLAMA_COMPAT=false` returns 503.

**E2E proof:** `spec027_ollama_compat_disabled_returns_503`

---

### SEC-007 | P1 | WebSocket progress unauthenticated

`/ws/pipeline/progress`, `/ws/progress/{track_id}` — no auth gate when auth off.

**Status:** **OPT-IN** when auth on.

**E2E proof:** `spec027_websocket_auth_rejects_missing_token_when_auth_enabled`

---

### SEC-008 | P1 | CORS permissive

```95:100:edgequake/crates/edgequake-api/src/server.rs
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);
```

**Status:** **OPT-IN** allowlist via env.

---

### SEC-009 | P1 | Rate limiting not wired by default

`tenant_rate_limit` middleware exists; inactive until env set. `AppState.rate_limiter` unused when off.

**Status:** **OPT-IN**

**E2E proof:** `spec027_rate_limit_returns_429_when_enabled`

---

### SEC-010 | P1 | Env API key compare — FIXED

`auth_validation.rs` and `middleware::SecurityConfig::validate_api_key` use `identity_storage::constant_time_str_eq` (phase 33).

**E2E/contract:** `spec027_sec010_constant_time_env_api_keys`

---

### SEC-011 | P2 | Login lockout — FIXED when auth on

`max_login_attempts` and `lockout_duration` from `AuthConfig` are enforced in `services/login_lockout.rs`. Failed attempts increment `UserRecord.failed_login_attempts`; threshold sets `locked_until` and returns HTTP **423** `ACCOUNT_LOCKED`.

**Status:** **FIXED** when `EDGEQUAKE_AUTH_ENABLED=true` (phase 32).

**Contract proof:** `spec027_login_lockout_sec011`

**E2E proof:** `spec027_login_lockout_returns_423_after_max_failed_attempts`

---

### SEC-012 | P2 | RBAC matrix — PARTIAL (admin gate wired)

`edgequake-auth/src/rbac.rs` — full permission model. Admin gate uses `rbac.require_role(&auth.role, &Role::Admin)` (phase 33). Granular `Permission` checks remain unused in most handlers.

`ApiAuthenticated`, `ApiRequireAdmin`, `ApiOptionalAuth` — **wired** on auth handlers.

**Contract proof:** `spec027_auth_extractors_arch_d001`, `spec027_identity_storage_ssot_phase33`

---

### SEC-013 | P2 | Dual isolation semantics

| Module | Semantics |
|--------|-----------|
| `isolation.rs` | Strict — missing props → deny |
| `workspace_scope.rs` | Legacy `"default"` UUID alias |

**Status:** **FIXED** — dual mode documented in `isolation_context.rs`

---

### SEC-014 | P2 | RLS — IMPROVED (SSOT DRY + legacy deprecated)

Migrations `001_init_database.sql`, `009_add_rls_policies.sql` define RLS policies + `set_tenant_context()`.

**Phase 35:** `with_acquired_tenant_context` (acquire connection — avoids pool session leak). API SSOT: `services/tenant_isolation.rs` + `PgIsolationScope` attached on authenticated requests.

**Phase 36:** `conversation.rs` migrated — all conversation/folder CRUD uses acquired-connection RLS.

**Phase 37:** `acquire_rls_connection` / `release_rls_connection` are the **single SSOT** in `rls.rs`. Conversation adapter delegates. Legacy `RlsContext` and pool-level `set_tenant_context` are **deprecated**.

**Status:** **OPT-IN** — `EDGEQUAKE_PG_RLS_ENABLED=true`. Handlers must call `run_with_pg_rls`; scope is attached but not yet consumed by most handlers.

**Caveats:** Superuser DB role bypasses RLS; Layer 1 app filters remain mandatory.

**Contract:** `spec027_rls_acquire_ssot_phase37`, `spec027_conversation_rls_acquired_phase36`, `spec027_tenant_isolation_ssot_phase35`

---

### SEC-015 | P2 | Shared conversation access

`GET /api/v1/shared/{share_id}` — share ID as capability token; UUID guessing risk (low if v4 random).

---

## E2E Security Test Matrix (Code-Verified — phase 52)

| E2E test | SEC finding | What it proves |
|----------|-------------|----------------|
| `spec027_admin_accessible_without_auth_when_auth_disabled` | SEC-001 | Dev-mode open (not prod default) |
| `spec027_stored_api_key_authenticates_when_auth_enabled` | SEC-002 | KV API keys work when auth on |
| `spec027_non_admin_receives_403_on_admin_endpoint` | SEC-003 | Admin gate when auth on |
| `spec027_strict_tenant_bind_rejects_header_jwt_mismatch` | SEC-004 | Strict bind opt-in |
| `spec027_ollama_compat_disabled_returns_503` | SEC-006 | Ollama gate opt-in |
| `spec027_websocket_auth_rejects_missing_token_when_auth_enabled` | SEC-007 | WS auth when auth on |
| `spec027_rate_limit_returns_429_when_enabled` | SEC-009 | Rate limit opt-in |
| `spec027_login_lockout_returns_423_after_max_failed_attempts` | SEC-011 | Login lockout when auth on |
| `spec027_pg_auth_login_refresh_stored_in_postgres` | PG SSOT | Login/refresh in PostgreSQL |
| `spec027_pg_auth_api_key_roundtrip_postgres` | PG SSOT | API keys in PostgreSQL |
| `spec027_pg_health_oauth_capabilities_postgres` | OAuth honesty | `oauth2_oidc_builtin: false` |
| `spec027_pg_auth_kv_mirror_env_ignored_when_pool` | KV eliminated | Mirror env ignored with pool |
| `spec027_document_list_scopes_metadata_by_tenant_workspace` | isolation | Tenant scoping in handlers |
| `spec027_track_status_scopes_by_tenant_workspace` | isolation | Track status scoping |
| `spec027_workspace_delete_scopes_kv_documents` | isolation | Workspace delete scoping |

---

## Security Headers

| Header | App layer | Docs (nginx) |
|--------|-----------|--------------|
| HSTS | ❌ | ✅ recommended |
| X-Frame-Options | ❌ | ✅ |
| X-Content-Type-Options | ❌ | ✅ |
| CSP | ❌ | ✅ |
| Referrer-Policy | ❌ | ✅ |

**Verdict:** App relies on reverse proxy for headers. Document as deployment requirement or add `tower-http` security headers layer.

---

## SQL / Cypher Injection Surface

| Path | Risk |
|------|------|
| `cypher_query_bound` | ✅ Parameterized |
| `cypher_query` (legacy) | ⚠️ Dollar-quoted embed |
| `query_ops.rs` search | ⚠️ Escaped strings + FTS |
| `properties_to_cypher` keys | ⚠️ Unescaped property keys |

No raw user Cypher endpoint found — **good**.

---

## Positive Security Signals

- Argon2id passwords with strength rules
- IDOR → 404 in `load_node_for_tenant_context`
- Refresh token revocation on logout
- Public registration cannot self-assign admin (tested)
- Upload body limit via `DefaultBodyLimit`
- Audit events on login success/failure via `ComplianceRuntime`
- Three-layer tenant isolation SSOT (phase 35)
- RLS acquire/release SSOT in `rls.rs` (phase 37)
- Conversation adapter acquired-connection RLS (phase 36)
- JWT tenant/workspace scope on login/refresh (phase 34)
- SEC-010 constant-time env API key compare (phase 33)
- RBAC `require_role` on admin gate (phase 33)
- Production hardening guide: `docs/operations/runtime-auth-hardening.md`

---

## OAuth Expert Recommendation (phase 52)

```
  Phase A (done):    Secure-by-default auth + PG identity SSOT + external SSO via oauth2-proxy
  Phase B (future):  Optional in-process OIDC module — NEW FEATURE (out of SPEC-027)
  Phase C (future):  OAuth2 client credentials for machine clients
  Never (v1 break):  Remove header-based tenant context — add claim binding first
```

Full plan: [008-improvement-plan-ascending.md](./008-improvement-plan-ascending.md) IMP-001..009.

---

## Code Re-assessment (phase 38)

**Default deploy: still F. Auth-enabled + PG deploy: B (identity A−).**

| Item | Status | Evidence |
|------|--------|----------|
| PG identity SSOT | **DONE** | `IdentityPolicy`, PG read/write paths |
| RLS default on | **DONE** | `pg_rls_enabled: true` |
| KV mirror opt-in | **DONE** | `EDGEQUAKE_KV_IDENTITY_MIRROR` |
| Migration 051 | **DONE** | bootstrap m051 |
| Refresh/api keys in KV | **OPEN** | session artifacts |
| Contract | **89+1** | +2 phase 38 |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 37)

**Default deploy: still F. Auth-enabled: B−. Isolation: B+ (RLS SSOT DRY).**

| Item | Status | Evidence |
|------|--------|----------|
| `acquire_rls_connection` SSOT | **DONE** | `rls.rs` + `postgres/mod.rs` re-export |
| Conversation adapter DRY | **DONE** | uses SSOT, no private acquire helpers |
| Legacy `RlsContext` | **DEPRECATED** | pool-level leak documented |
| `with_acquired_tenant_context` | **DRY** | delegates to acquire/release SSOT |
| Dual KV+PG verdict | **UNCHANGED** | keep v1 dual; bootstrap 048–050 |
| Contract | **87+1** | +`spec027_rls_acquire_ssot_phase37` |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 36)

**Default deploy: still F. Auth-enabled: B−. Isolation: B+ (conversation RLS acquire).**

| Item | Status | Evidence |
|------|--------|----------|
| Conversation adapter RLS | **DONE** | `acquire_tenant_conn` / `release_tenant_conn` |
| Legacy pool `set_context` | **REMOVED** | contract asserts absence |
| SEC-014 | **IMPROVED** | first production adapter on acquired-connection RLS |
| Dual KV+PG verdict | **UNCHANGED** | keep v1 dual; bootstrap 048–050 |
| Contract | **86+1** | +`spec027_conversation_rls_acquired_phase36` |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 35)

**Default deploy: still F. Auth-enabled: B−. Isolation: B (3 layers documented).**

| Item | Status | Evidence |
|------|--------|----------|
| Dual KV+PG verdict | **DOCUMENTED** | Keep v1 dual; bootstrap 048–050 |
| Three isolation layers | **DONE** | `tenant_isolation.rs` |
| Pool-safe RLS | **DONE** | `with_acquired_tenant_context` |
| PG RLS opt-in flag | **DONE** | `EDGEQUAKE_PG_RLS_ENABLED` |
| Migration 050 | **DONE** | bootstrap m050 |
| Conversation RLS migrate | **DONE** | phase 36 — acquired-connection pattern |
| Contract | **85+1** | +2 phase 35 tests |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 34)

**Default deploy: still F. Auth-enabled: B− (identity storage B+).**

| Item | Status | Evidence |
|------|--------|----------|
| Membership PG sync | **DONE** | `sync_default_membership_to_postgres` |
| JWT scope claims | **DONE** | `access_token_claims` in login/refresh |
| Strict bind membership verify | **DONE** | `membership_bind_scope` (PG only) |
| Migration 049 | **DONE** | bootstrap m049 |
| Multi-workspace membership RBAC | **OPEN** | default scope only |
| Contract | **83+1** | +migration 049 test |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 33)

**Default deploy: still F. Auth-enabled: B− unchanged.**

| Item | Status | Evidence |
|------|--------|----------|
| Identity storage SSOT | **DONE** | `identity_storage.rs` — First Principles table |
| KV → PG user sync | **DONE** | login lockout, create_user, `persist_user_record` |
| Migration 048 | **DONE** | bootstrap m048 + lockout columns |
| SEC-010 constant-time keys | **FIXED** | `constant_time_str_eq` |
| SEC-012 admin RBAC | **IMPROVED** | `rbac.require_role` — granular perms still open |
| memberships → API auth | **OPEN** | PG table exists; not consulted |
| Contract | **82+1** | +3 phase 33 tests |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

**Default deploy: still F. Auth-enabled + env hardened: B− (SEC-011 closed).**

| Item | Status | Evidence |
|------|--------|----------|
| SEC-011 login lockout | **FIXED** | `login_lockout.rs` + `UserRecord` fields |
| HTTP 423 ACCOUNT_LOCKED | **DONE** | `ApiError::AccountLocked` — OpenAPI documents 423 |
| `persist_user_record` DRY | **DONE** | shared KV write for lockout |
| Default `auth_enabled` | **false** | unchanged |
| Contract tests | **79+1** | +`spec027_login_lockout_sec011` |
| E2E security tests | **33 pass** | +lockout E2E |

**Brutal honesty:** Lockout is meaningless when auth is off (login is not the gate). Secure defaults (AC-4) remain the production blocker.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

**Default deploy: still F. Auth-enabled + env hardened: B− unchanged.**

| Item | Status | Evidence |
|------|--------|----------|
| `get_document` ISP | **DONE** | `StorageRuntime` + `PostgresRuntime` — no auth surface change |
| Document query module ISP | **COMPLETE** | list + scan + track_status + detail all ISP |
| Default `auth_enabled` | **false** | `config.rs:76` |
| Contract tests | **78+1** | +`spec027_get_document_isp` |
| E2E security tests | **32 pass** | unchanged |

**Brutal honesty:** All document **read** handlers now use ISP — security posture identical to phase 30. Secure defaults (AC-4) remain the blocker for production F→B grade.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

Auth session on runtime extractors (`AuthRuntime`, `StorageRuntime`, `ComplianceRuntime`). `login`/`refresh`/`logout`/`get_me` verified in `spec027_auth_extractors_arch_d001`. Default auth grade **F** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Historical Phases (9–29)

Phases 9–29 delivered OPT-IN mitigations: `auth_validation.rs` (IMP-002), admin extractors (IMP-003), strict tenant bind (IMP-004), Ollama gate (IMP-005), WS auth (IMP-006), CORS allowlist (IMP-007), rate limit (IMP-008). See 009 and 007 for evidence.

**Stale doc policy:** Sections without **"Code Re-assessment (phase N)"** are not authoritative.
