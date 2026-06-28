# OpenAPI & Swagger Expert Lens

**Spec:** 027-api-edgequake-audit  
**Cross-ref:** [003-rest-design-lens.md](./003-rest-design-lens.md) | [008-improvement-plan-ascending.md](./008-improvement-plan-ascending.md)  
**Re-assessed:** 2026-06-28 (phase 51 — ApiCapabilities oauth fields in examples)  
**Stack:** utoipa **=5.4.0** (pinned) + utoipa-swagger-ui → `/swagger-ui`, `/api-docs/openapi.json`, `/api-docs/asyncapi.json`

---

## Verdict: A++ (post phase 31)

Swagger UI documents the **full HTTP surface** with **triple parity CI**, **compile-time path SSOT validation**, **100% DTO example coverage**, standalone AsyncAPI, and **frontend codegen** (OAS-009).

**Honest A++ caveats:**

| Caveat | Detail |
|--------|--------|
| Macro SSOT | utoipa `#[openapi(paths(...))]` cannot use `include!` — drift fails **build.rs** compile, not macro |
| Enum examples | Domain edge enums use synthetic fallbacks where utoipa emits non-Object schema wrappers |
| Human docs lag | `docs/api-reference/rest-api.md` may trail CI — **not law** |

---

## Architecture (ASCII)

```
  handlers/#[utoipa::path] ──► build.rs SSOT scan ──► compile-error on drift
         │                           │
         │                           └── openapi_path_ssot.rs count assert (158)
  handlers/#[utoipa::path] ──► openapi_annotation_sync ──► contract CI
  routes.rs ──► route_registry ──► contract CI (routes ↔ OpenAPI)
  openapi.rs (paths list) ──► ApiDoc ──► enrichment + security + examples
         │                                    ├── servers, version
         │                                    ├── WS x-extensions
         │                                    ├── x-edgequake-asyncapi
         │                                    └── openapi_examples (100% DTOs)
         ▼
  SwaggerUi (persistAuthorization) → /swagger-ui
  GET /api-docs/openapi.json (E2E)
  GET /api-docs/asyncapi.json (standalone AsyncAPI 2.6)
  edgequake_webui/scripts/codegen-openapi.sh → schema.d.ts (OAS-009)
```

---

## Coverage Analysis (Code-Verified Phase 31)

| Metric | Value | Source |
|--------|-------|--------|
| Handler paths (build scan) | **158** | `build.rs` + `openapi_path_ssot.rs` |
| OpenAPI path entries | ≥105 `/api/v1` | `spec027_openapi_v1_path_coverage_threshold` |
| Routes ↔ OpenAPI ↔ utoipa | **0 drift** | triple parity CI |
| DTO schemas with examples | **100%** | `spec027_openapi_all_schemas_have_examples` |
| AsyncAPI standalone | ✅ | `/api-docs/asyncapi.json` E2E |
| utoipa pin (OAS-010) | **=5.4.0** | `spec027_utoipa_version_pinned` |
| PATCH user (OAS-011) | ✅ | `user_management.rs` + OpenAPI |
| Frontend codegen (OAS-009) | ✅ | `codegen-openapi.sh` + snapshot |
| v1 RPC 202 documented | ✅ 6/6 | `spec027_v1_rpc_openapi_v2_migration_extensions` |

---

## Findings Scorecard (Historical → Now)

| ID | Was (audit) | Now (code) |
|----|-------------|------------|
| OAS-001 drift | ~35% missing | **FIXED** — triple parity + build SSOT |
| OAS-002 models path | Bug | **FIXED** |
| OAS-003–008 | Various gaps | **FIXED** |
| OAS-009 frontend codegen | Open | **FIXED** — script + snapshot + openapi-typescript |
| OAS-010 utoipa pin | Hazard | **FIXED** — `=5.4.0` pinned |
| OAS-011 PATCH user | Missing | **FIXED** — PATCH documented + E2E contract |

---

## Swagger UI Expert Notes

| Check | Status |
|-------|--------|
| Try-it-out works for registered paths | ✅ |
| Server URL configurable | ✅ |
| `info.version` matches crate | ✅ |
| Auth persistence in UI | ✅ `persist_authorization(true)` |
| Example values on DTOs | ✅ **100% Object schemas** |
| Tags grouped logically | ✅ |
| Standalone AsyncAPI | ✅ `/api-docs/asyncapi.json` |
| Security schemes per-path | ✅ `openapi_security.rs` |

---

## Contract Test Coverage (Honest)

| Suite | Count | Proves |
|-------|-------|--------|
| `spec027_api_contract` | **78 pass + 1 ignored** | OpenAPI parity, examples, SSOT, auth ISP |
| `spec027_e2e` | **32 pass** | `/api-docs/openapi.json` serves valid doc |

OpenAPI-specific contract tests include: `spec027_openapi_path_ssot_build_validation`, `spec027_axum_routes_subset_of_openapi`, `spec027_openapi_paths_subset_of_axum_routes`, `spec027_openapi_all_schemas_have_examples`, `spec027_openapi_public_paths_have_empty_security`, `spec027_v1_rpc_openapi_v2_migration_extensions`.

---

## Code Re-assessment (phase 37)

No OpenAPI path changes. RLS SSOT is storage-layer only. Contract **87+1**. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

No OpenAPI path changes. Login 423 documented pre-existing. Contract **79+1**. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

**Verdict: A++ unchanged** — no new HTTP paths or schema surface.

| Item | Status | Evidence |
|------|--------|----------|
| `get_document` ISP | **DONE** | `detail.rs` uses `StorageRuntime` + `PostgresRuntime` — no OpenAPI change |
| Handler count | **158** | `REGISTERED_HANDLER_COUNT` unchanged |
| Contract tests | **78+1** | +`spec027_get_document_isp` |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 41)

No OpenAPI changes. PG auth E2E added. Contract **94+1** + 2 pg auth e2e.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

No OpenAPI changes. Contract **97+1** + 4 pg auth e2e. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 40)

No OpenAPI surface changes. Auth storage PG-only reads (phase 40) — no spec impact. Contract **93+1**. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

No OpenAPI surface changes. Default REST-025 202 documented on 6 v1 RPC. Contract **77+1**. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Historical Phases (14–29)

Phases 14–29 delivered: bidirectional CI, AsyncAPI sidecar, 100% examples, build SSOT, OAS-009 codegen, v2 Level 4 routes, v1 migration extensions, 202 status on RPC paths. See git history and 009 for evidence citations.

**Stale doc policy:** Sections without **"Code Re-assessment (phase N)"** are not authoritative.

---

## Code Re-assessment (phase 44)

AC-4 secure default — no OpenAPI surface change. 99+1 contract tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45–46)

`/health` mirror flags. 105 contract tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

No OpenAPI route changes. Identity SSOT documented in 004.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)
