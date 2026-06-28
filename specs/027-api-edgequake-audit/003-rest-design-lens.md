# REST Design Expert Lens

**Spec:** 027-api-edgequake-audit  
**Date:** 2026-06-28 (post phase 30 — default REST-025 202)  
**Cross-ref:** [002-openapi-swagger-lens.md](./002-openapi-swagger-lens.md) | [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Executive Summary (Code Is Law)

EdgeQuake exposes **two REST surfaces** with **different maturity targets**:

| Surface | Maturity | Grade | Honest verdict |
|---------|----------|-------|----------------|
| **`/api/v1/*`** | Level 2–3 | **A** | Default HTTP 202; opt-out `EDGEQUAKE_V1_RPC_RETURN_202=0` |
| **`/api/v2/workspaces/{id}/jobs/*`** | **Level 4** | **Level 4 / A++** | Unpublished; workspace job resources shipped phase 20 |

**Do not conflate grades.** v1 clients are fine for existing integrations. New long-running work should use **v2 jobs**.

---

## v2 REST — Level 4 (A++)

### Resource model

```
  /api/v2/workspaces/{workspace_id}/
  └── jobs/
        ├── catalog          GET   — discover 12 job types
        ├── (collection)     GET   — list (paginated)
        │                    POST  — create → 202 + Location
        └── {job_id}         GET   — status
                               DELETE — cancel (not POST /cancel)
```

### Grade card (v2 only)

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Workspace-nested resources | ✅ | `routes.rs` — no flat `/api/v2/jobs` |
| Long-running ops as jobs | ✅ | 12 types via `submission.rs` |
| DELETE cancel | ✅ | `cancel_workspace_job` |
| 202 + Location | ✅ | E2E `spec027_v2_job_create_and_get_roundtrip` |
| HATEOAS (self/cancel/catalog) | ✅ | `JobLinks`, `JobCollectionLinks` |
| Catalog discovery | ✅ | `job_registry.rs` |
| OpenAPI parity | ✅ | workspace paths in OAS + snapshot |
| Catalog scope validation | ✅ **Phase 22** | `ensure_workspace_scope` on catalog GET |
| Job type validation (catalog SSOT) | ✅ **Phase 23** | `is_creatable_v2_job_type` |
| 202 Location + Link (self) | ✅ **Phase 23** | RFC 8288 on create |
| Delegates to v1 logic (DRY) | ✅ | all 6 `run_*` cores via `submission.rs` |

**Remaining v2 honesty gaps:**

| Gap | Severity | Note |
|-----|----------|------|
| `reanalyze` without task → synthetic job | Low | edge case when no task enqueued |
| No workspace `{id}` parent resource GET | Low | acceptable — jobs are the v2 entry point |

**Verdict: Level 4 achieved.** Safe to publish v2 when product is ready.

---

## v1 REST — A (Default 202, Legacy Opt-Out)

### Resource model (unchanged)

```
  /api/v1/
  ├── tenants/{id}/workspaces/{id}/…   ← hierarchy OK
  ├── documents                        ← GET/POST/DELETE(all!) overloaded
  ├── workspaces/{id}/rebuild-*        ← RPC POST (still primary for v1 clients)
  ├── documents/reprocess, recover-stuck
  ├── graph/entities/{name}            ← name-as-ID
  ├── entities/{id}/provenance         ← UUID/name ambiguity (IMP-026)
  └── admin/*                          ← discoverable via /health capabilities
```

### Grade card (v1 only)

| Criterion | Status | Grade impact |
|-----------|--------|--------------|
| Resource + verb mix (Level 1) | ✅ | Baseline |
| Uniform pagination | **Partial** | documents **fixed** (IMP-019); not all collections |
| Consistent errors | **Partial** | problem+json Content-Type; hybrid body |
| RPC POST for async work | **DONE** | default **202**; legacy 200 via `EDGEQUAKE_V1_RPC_RETURN_202=0` |
| DELETE /documents = delete all | **OPT-IN** guard | REST-002 when env set |
| Three entity ID spaces | ❌ Open | REST-003 / IMP-026 |
| Share URL | ✅ Fixed | REST-009 |
| Admin discovery | ✅ Fixed | REST-008 `/health` capabilities |
| v2 migration hints on RPC responses | ✅ **Phase 21** | `v2_migration` field + OAS extensions |
| Sunset + Link headers on RPC (REST-024) | ✅ **Phase 22** | `v1_rpc_migration.rs` |
| 202 + Location on RPC (REST-025) | ✅ **Phase 30** | default 202; env opt-out restores 200 |

**Verdict: A** — default 202 for async RPC; legacy 200 preserved via env opt-out.

---

## v1 Ascending Improvements (Without Breaking)

What **can** improve v1 without route/status breaks:

| ID | Improvement | Status | Breaking? |
|----|-------------|--------|-------------|
| REST-002 | Bulk DELETE confirm header | **OPT-IN** (`EDGEQUAKE_REQUIRE_DELETE_ALL_CONFIRM`) | No |
| REST-007 | Document list pagination | **DONE** (`list_pagination.rs`) | No |
| REST-008 | `/health` capabilities | **DONE** | No |
| REST-009 | Share URL v1 path | **DONE** | No |
| REST-010 | problem+json Content-Type | **DONE** (hybrid body) | No |
| REST-021 | `v2_migration` on RPC responses | **DONE** phase 21 | No — additive JSON |
| REST-022 | OAS `x-edgequake-v2-job-type` on v1 RPC | **DONE** phase 21 | No — extension only |
| REST-023 | Document upload 202 + Location everywhere | **Partial** | No — already 202 on async upload |
| REST-024 | Deprecation `Sunset` header on v1 RPC | **DONE** phase 22–23 (all 6 RPC) | No — additive headers |
| REST-025 | Change v1 rebuild 200→202 | **DONE** phase 30 | Default 202; `EDGEQUAKE_V1_RPC_RETURN_202=0` restores 200 |
| REST-003 | Unify entity ID spaces | **DEFERRED** | **Yes** — IMP-026 |

**Recommendation:** v1 at **A** (default 202). Use env opt-out for legacy 200 integrators. Point new integrators to **v2 jobs**.

---

## Findings (Cross-Version)

### REST-001 | RPC verbs (v1 open, v2 closed)

v1 RPC paths remain. v2 Level 4 replaces them for new clients. See grade cards above.

### REST-003 | Entity ID spaces (v1 only)

Three ID spaces — **IMP-026 partial** (`entity_graph_lookup` SSOT only). Breaking to fix.

### REST-006 | Versioning

`/api/v1/*` business, `/api/*` Ollama, unversioned probes — **intentional**.

### REST-007 | Pagination (v1)

| Endpoint | Status |
|----------|--------|
| `/documents` | **FIXED** — paginated (phase 2+) |
| `/tasks`, `/conversations` | Paginated |
| `/graph/entities` | Push-down page |

*(Earlier audit text claiming documents "full scan" is **stale** — code uses `paginate_vec`.)*

---

## REST Maturity Model

```
  Level 0 ─ RPC only
  Level 1 ─ Resources + verbs          ◄── v1 baseline
  Level 2 ─ Uniform pagination/errors ◄── v1 partial (A ceiling without breaks)
  Level 3 ─ Hypermedia                 ◄── v1: /health capabilities + RPC Link headers
  Level 4 ─ Job resources              ◄── v2 DONE (phase 20); v1 default 202 (phase 30)
```

---

## Code Re-assessment (phase 32)

No REST changes. Contract 79+1. Verdict unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

No REST surface changes. `get_document` ISP (SOLID). v1 **A** unchanged. Contract 78+1.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 29)

No REST surface changes. v1 grade **A−** unchanged (OpenAPI 202 documented; runtime default 200).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

REST-025 default 202 shipped. Legacy 200 via env opt-out. v1 grade **A− → A**. Link headers preserved on 202.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 28)

All 6 v1 RPC utoipa paths document `status = 202`. Runtime still defaults to 200 unless REST-025 opt-in or strict-startup bundle. v1 grade **B+ → A−** (honest OpenAPI; not yet default-breaking).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 27)

REST-025 strict-startup bundle: `EDGEQUAKE_STRICT_STARTUP=1` enables v1 202 when `V1_RPC_RETURN_202` unset. OpenAPI documents 202 on rebuild. v1 grade **B → B+**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 26)

REST-025 **FIXED (opt-in)** — `respond_v1_async_rpc` on all 6 v1 RPC; default 200; `EDGEQUAKE_V1_RPC_RETURN_202=1` returns 202 + Location + Link self when job/track id present. E2E: `spec027_v1_rpc_returns_202_when_opt_in`. v1 grade **B− → B**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 25)

No REST changes. Verdict unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 24)

No REST surface changes. ARCH-006 DRY in graph handlers only. Verdict unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 23)

| Item | v1 | v2 |
|------|----|----|
| `run_reanalyze_multimodal` | **DONE** | submission uses `run_*` |
| reanalyze v2_migration + REST-024 | **DONE** | n/a |
| Catalog SSOT job type validation | n/a | **DONE** |
| 202 Link header | n/a | **DONE** |
| New migration | **No** | **No** |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 22)

| Item | v1 | v2 |
|------|----|----|
| REST-024 Sunset + Link headers | **DONE** | n/a |
| Catalog workspace scope | n/a | **DONE** |
| `run_*` SOLID extract | **PARTIAL** (5/6 RPC) | submission uses `run_*` |
| New migration | **No** | **No** |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 21)

| Item | v1 | v2 |
|------|----|----|
| Migration hints on RPC responses | **DONE** | n/a |
| OAS v2 job type extensions | **DONE** | n/a |
| Separate grade documentation | **DONE** | **Level 4** |
| New migration | **No** | **No** |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 41)

No REST surface changes. PG auth E2E proves internal storage path.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

List users E2E proves GET `/api/v1/users` reads PG SSOT. No new routes.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 40)

No REST surface changes. Auth PG-only reads — internal storage only. Authority: 009.

---

## Code Re-assessment (phase 37)

No REST surface changes. v1/v2 grades unchanged. Authority: 009.

---

## Code Re-assessment (phase 20)

v2 Level 4 workspace jobs. v1 B− unchanged. Authority: 009.

---

## Historical re-assessments (phases 9–19)

See git history. Superseded by phase 20/21 tables above unless cited in 009.

---

## Code Re-assessment (phase 44)

AC-4: unauthenticated GET `/documents` returns 401 unless DEV_MODE. No new routes.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45–46)

`/health` capabilities include auth SSOT label. No route changes.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

`/health` reports `kv_identity_mirror_effective: false` with PG pool.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)
