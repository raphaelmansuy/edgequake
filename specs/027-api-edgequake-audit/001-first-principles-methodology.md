# First Principles — API Audit Methodology

**Spec:** 027-api-edgequake-audit  
**Re-assessed:** 2026-06-28 (phase 52 — AUTH CLOSED; PG SSOT; OAuth external only)  
**Cross-ref:** [000-index.md](./000-index.md) | [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## FP-001: What Is an API?

An HTTP API is a **contract** between independent release cycles:

```
  Producer (edgequake-api)          Consumer (WebUI, SDK, integrators)
  ─────────────────────────         ───────────────────────────────────
  routes.rs          ───────────►  client.ts (hand-written, no codegen)
  openapi.rs         ───────────►  Swagger UI / external SDKs
  error.rs           ───────────►  retry logic, error UX
  middleware.rs      ───────────►  auth headers, tenant scope
```

**First principle:** If the contract lies, consumers break silently. EdgeQuake **pre-phase-13** had three divergent contracts; **post phase 15** HTTP path parity is CI-enforced:

1. `routes.rs` — runtime truth (~100 routes)
2. `openapi.rs` + `build.rs` — published truth with compile-time drift guard
3. `docs/api-reference/rest-api.md` — human truth (may lag; not law)

**Invariant:** One SSOT for route inventory. Today: **FIXED for HTTP** (IMP-011 triple parity CI). Manual `openapi.rs` handler list remains but drift fails CI.

---

## FP-002: Security Is Default-Deny

```
  Request
     │
     ▼
  ┌─────────────────┐     auth_enabled=false (DEFAULT)
  │ protected_api   │ ──► pass through ──► require_authenticated_request
  │ _auth           │                         returns Admin demo-user
  └─────────────────┘
```

**First principle:** Absence of configuration must not grant privilege.

- `AuthConfig::default().auth_enabled = false` (`edgequake-auth/src/config.rs:76`)
- Handlers grant `Role::Admin` when auth disabled (`handlers/auth/mod.rs`)

**Invariant:** Production deployments default to authenticated. Today: **violated** (SEC-001).

---

## FP-003: Isolation Is a Predicate, Not a Header

Tenant scope must be **derived from authenticated identity**, not client-supplied headers alone.

```
  Current (broken trust model):

  Client ──► X-Tenant-ID: arbitrary ──► TenantContext ──► handler filter
                ▲
                └── not bound to JWT claims in middleware

  Target (first principles):

  JWT/API-key ──► AuthContext { tenant, workspace, role }
                       │
                       ▼
              reject if X-Tenant-ID ≠ claim.tenant_id
```

**Invariant:** One isolation predicate for all storage domains. Today: **FIXED for HTTP paths** — `isolation_context.rs` unifies graph (strict) and document (legacy alias) modes explicitly (ARCH-001). Tenant-only query filter retains global suffix scan by product constraint.

---

## FP-004: Complexity Budget

Every handler has an **allocated query budget** per request:

| Operation class | Budget | Violation signal |
|-----------------|--------|------------------|
| List (paginated) | O(page_size) DB round-trips | N+1 loops |
| List (collection) | O(1) index scan + O(n) filter in memory | `keys_like('%')` |
| Graph traversal | O(subgraph) with tenant push-down | fetch all, filter in handler |
| Bulk admin | O(batch) with cursor | full KV `keys()` |

**First principle:** Push predicates to storage; batch what remains.

Evidence (phase 16+): `entity_crud.rs`, `search.rs`, and `entity_neighborhood.rs` use `node_degrees_batch`; merge uses `upsert_edges_batch` (PERF-001 **FIXED**).

---

## Audit Lenses → First Principles Map

```
                    ┌─────────────────────────────────────┐
                    │         First Principles            │
                    │  Contract │ Default-Deny │ SSOT │ O() │
                    └─────┬───────────┬──────────┬────┬───┘
                          │           │          │    │
         ┌────────────────┼───────────┼──────────┼────┼────────────────┐
         ▼                ▼           ▼          ▼    ▼                ▼
    OpenAPI/Swagger    REST       OAuth/SEC   O(n)  DRY/SOLID    Systems
    (contract)      (resources)  (deny)    (budget) (SSOT)     (ops/RLS)
```

---

## Finding ID Convention

```
{LENS}-{NNN}  e.g. OAS-007, SEC-003, PERF-001

Cross-ref:  OAS-007 ──blocks──► IMP-012 (openapi registry automation)
            SEC-001  ──blocks──► IMP-001 (auth default flip + migration guide)
            ARCH-003 ──relates──► PERF-006 (isolation push-down)
```

Full matrix: [007-cross-reference-matrix.md](./007-cross-reference-matrix.md).

---

## Code Is Law — Evidence Hierarchy

| Rank | Source | Weight |
|------|--------|--------|
| 1 | `routes.rs`, `middleware.rs`, handler bodies | **Binding** |
| 2 | `openapi.rs` `paths(...)` registry | Binding for Swagger only |
| 3 | `#[utoipa::path]` annotations | Intent; may diverge from registry |
| 4 | `lib.rs` FEAT/BR comments | **Aspirational** — not enforced |
| 5 | `docs/api-reference/*.md` | Advisory |

**Example of law vs aspiration:**

- `lib.rs:21` — `BR0401`: Errors follow RFC 7807
- `error.rs:21-28` — Custom `{code, message, details}` JSON
- `middleware.rs:269-273` — Different `{error, message, request_id}` shape

**Verdict:** Documentation claims are **not law**. Code is.

---

## Ascending Compatibility Principle (IMP-000)

All remediation in [008-improvement-plan-ascending.md](./008-improvement-plan-ascending.md) MUST satisfy:

```
  v1 clients (today) ──► MUST keep working unchanged
         │
         ▼
  additive fixes (new headers, new optional fields, new routes)
         │
         ▼
  deprecation window (Sunset + Deprecation headers, min 2 minor releases)
         │
         ▼
  v2 routes (clean REST) — opt-in, parallel to v1
```

**Never:** Remove or rename v1 paths without deprecation cycle.  
**Never:** Change error `code` strings without alias mapping.  
**Always:** Feature-flag security tightening (`EDGEQUAKE_AUTH_ENABLED` already exists — extend pattern).

---

## Code Re-assessment (2026-06-28, phase 9)

| Principle | Status | Honest note |
|-----------|--------|-------------|
| FP-001 single contract SSOT | **PARTIAL** | Route CI (Axum ⊆ OpenAPI); manual `openapi.rs` list remains |
| FP-002 default-deny | **OPEN at default** | OPT-IN by ascending-compat (AC-4) |
| FP-003 O(workspace) metadata | **FIXED** | wsdoc index read + write SSOT + migration 047 |
| FP-004 handlers thin | **PARTIAL** | God files split; some handlers still >200 LOC |
| Documentation vs code | **Aligned in 009** | Lens docs re-assessed against source |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 10)

Query filter + workspace stats now use wsdoc-backed reads when workspace context is present. FP-003 fully **FIXED** for workspace-scoped HTTP paths; tenant-only query filter remains global suffix scan (non-UUID tenant string ids).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 11)

FP-001 DRY: zero raw `format!("{}-metadata")` in `src/` (contract-enforced). Metadata key construction is a single SSOT path via `metadata_key_for_document` → `kv_keys::doc_metadata`.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 12)

FP-003 O(n): last production handler `keys()` scan removed — admin user paths use `keys_with_prefix(USER_KEY_PREFIX)`.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 13)

FP-001 single contract SSOT: **PARTIAL → FIXED for HTTP paths** — bidirectional IMP-011 CI (`routes ⊆ openapi` and `openapi ⊆ routes`). OpenAPI lens grade **A** (see 002). Manual `openapi.rs` list remains but drift fails CI.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 14)

FP-001 triple parity: routes ↔ OpenAPI ↔ `#[utoipa::path]` annotations (`openapi_annotation_sync.rs`). E2E GET `/api-docs/openapi.json` validates live document. OpenAPI lens **A+** (002). Manual `openapi.rs` handler list still not compile-time SSOT — drift caught by triple CI only.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 15)

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 16)

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 17)

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 18)

FP-003: query filter now always uses scoped metadata loader (DRY with list/cost/tasks). Merge cold path extracted to `entity_merge` service (SOLID). No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

SEC-011 login lockout — FP-002 wiring improved when auth on; default-deny still OPEN. Contract 79+1, E2E 33.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

`get_document` ISP — SOLID only, no REST/security change. Document query module complete. Contract 78+1. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

Default REST-025 202 + auth session ISP. ComplianceRuntime audit path. Contract 77+1, E2E 32. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 29)

graph_stream ISP closes last graph AppState holdout. get_me uses ApiAuthenticated. No migration. Contract 75+1, E2E 31.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 28)

GraphQueryRuntime ISP (SOLID). OpenAPI 202 on all 6 v1 RPC (FP-001 contract honesty). api_keys auth extractors (FP-002 wiring improved; default-deny still OPEN). 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 38)

FP-003: **PG identity SSOT primary** when pool available. RLS default on. KV mirror opt-in only. 89+1 contract + 33 e2e. Migration 051.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 37)

FP-002 (default-deny): **still OPEN** — auth off by default unchanged.

FP-003 (tenant isolation): **IMPROVED** — RLS acquire/release SSOT in `rls.rs`; legacy pool API deprecated. Dual KV+PG verdict **unchanged** (keep v1).

87+1 contract + 33 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 27)

Strict-startup REST-025 bundle. ARCH-D-001 complete (create_user + ApiOptionalAuth). ISP bulk (+8 handlers). 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 26)

REST-025 opt-in 202 + ARCH-D-001 admin extractors + lineage ISP. 74+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 25)

API-SOLID-I-001 FromRef ISP wired. Relationships module migrated. 73+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 24)

ARCH-006 graph edge DTO SSOT (DRY). FP-004 handlers thin **FIXED**. Contract **72 pass + 1 ignored**, E2E **30 pass**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 23)

6/6 `run_*` SOLID complete. Catalog SSOT validation. FP-001 HTTP contract **FIXED**. FP-002 default-deny still **OPEN** (AC-4). FP-003 workspace metadata **FIXED**. FP-004 handlers thin **FIXED** (god files split). Contract **71 pass + 1 ignored**, E2E **30 pass**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 22)

`run_*` SOLID extract on 5 v1 RPC handlers; `v1_rpc_migration` service. Grade **A++** retained.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 21)

v1 RPC `v2_migration` hints + OAS extensions. 003 v1/v2 split. Contract 68.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 20)

FP-001: v2 Level 4 contract — workspace-nested job resources. v1 unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 41)

PG auth E2E (`spec027_pg_auth_e2e`) proves login/refresh/api_keys stored in PostgreSQL with no KV mirror. `with_optional_pg_rls` SSOT for handler RLS wiring.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

**First principle confirmed:** All identity/session PG queries use RLS envelope (`acquire_optional_pg_connection`). PG-only auth SSOT complete for production paths. KV auth = test harness only. Migration 054.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 44)

AC-4 closed: auth **on by default**; `EDGEQUAKE_DEV_MODE=true` for frictionless local dev (`make dev`). Explicit opt-out via `EDGEQUAKE_AUTH_ENABLED=false`. Migration 055.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 40)

**Auth storage first principle:** With bootstrap 048–058, **PostgreSQL is the sole auth SSOT** when pool available. KV auth reads and writes **eliminated** when pool (phase 47). KV remains only for in-memory tests (no pool). Document/graph KV unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45–46)

PG identity/session SSOT final. KV mirror env **ignored** when pool (phase 47). 5 PG auth E2E. **Auth scope complete.**

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

See [004-security-oauth-lens.md](./004-security-oauth-lens.md) — User/RBAC/Tenant/Workspace SSOT table.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)
