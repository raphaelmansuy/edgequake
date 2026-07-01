# SPEC-035 — Cross-Reference Matrix

**Purpose:** Maps every claim across all lenses to its evidence source.  
**Method:** Code is law — all claims traceable to a file + line.  

---

## Evidence Cross-Reference

| Claim                                            | File                                                        | Line(s)         | Verified |
| ------------------------------------------------ | ----------------------------------------------------------- | --------------- | -------- |
| Custom explorer has 30 hardcoded endpoints       | `edgequake_webui/src/components/shared/api-explorer.tsx`    | 48–95           | ✅        |
| Backend serves 169 documented API paths          | `edgequake/crates/edgequake-api/src/openapi.rs`             | `paths()` block | ✅        |
| Backend serves Swagger UI at `/swagger-ui/`      | `edgequake/crates/edgequake-api/src/server.rs`              | L127–135        | ✅        |
| Backend serves spec at `/api-docs/openapi.json`  | `edgequake/crates/edgequake-api/src/server.rs`              | L130–131        | ✅        |
| Auth token stored as `accessToken` in Zustand    | `edgequake_webui/src/stores/use-auth-store.ts`              | L38             | ✅        |
| Server base URL from `getRuntimeServerBaseUrl()` | `edgequake_webui/src/lib/runtime-config.ts`                 | L64             | ✅        |
| CORS is already configured in backend            | `edgequake/crates/edgequake-api/src/server.rs`              | L96–120         | ✅        |
| Explorer page is a 5-line import wrapper         | `edgequake_webui/src/app/(dashboard)/api-explorer/page.tsx` | 1–5             | ✅        |
| No OpenAPI client library in frontend            | `edgequake_webui/package.json`                              | —               | ✅        |
| OpenAPI spec enriched to A++ standard            | `edgequake/crates/edgequake-api/src/openapi_enrichment.rs`  | —               | ✅        |
| Spec has full schema examples                    | `edgequake/crates/edgequake-api/src/openapi_examples.rs`    | —               | ✅        |

---

## Requirement Traceability

| Requirement                                  | Source Spec                     | Implementation File                              |
| -------------------------------------------- | ------------------------------- | ------------------------------------------------ |
| REQ-035-01: Explorer shows 100% of endpoints | `003-product-owner-lens.md` AC1 | `hooks/use-api-explorer-config.ts` specUrl       |
| REQ-035-02: Auth token pre-populated         | `003-product-owner-lens.md` AC2 | `hooks/use-api-explorer-config.ts` bearerToken   |
| REQ-035-03: Dark mode visual consistency     | `004-ux-ui-designer-lens.md` P4 | `lib/api-explorer-theme.ts`                      |
| REQ-035-04: Zero maintenance per endpoint    | `002-first-principles.md` P9    | Architecture — spec URL only                     |
| REQ-035-05: Path parameter inputs            | `006-user-lens.md` US-004       | Handled by `@scalar/api-reference`               |
| REQ-035-06: Workspace base URL injection     | `003-product-owner-lens.md` AC3 | `hooks/use-api-explorer-config.ts` serverBaseUrl |
| REQ-035-07: Try-it-out works                 | `003-product-owner-lens.md` AC4 | Core library feature                             |

---

## Decision Justification Cross-Reference

| Decision                          | Justified In                    | Adversarially Tested In              |
| --------------------------------- | ------------------------------- | ------------------------------------ |
| Replace with Scalar over custom   | `002-first-principles.md` Opt B | `007-decision-matrix.md` Attack 1–6  |
| Reject iframe/redirect (Option A) | `007-decision-matrix.md`        | `007-decision-matrix.md` "Why NOT A" |
| Reject custom rewrite (Option C)  | `007-decision-matrix.md`        | `007-decision-matrix.md` "Why NOT C" |
| Reject status quo (Option D)      | `007-decision-matrix.md`        | `001-five-whys.md`                   |

---

## User Story Coverage

| User Story                          | Lens File          | Implementation Coverage                |
| ----------------------------------- | ------------------ | -------------------------------------- |
| US-001: Search for PDF endpoints    | `006-user-lens.md` | Scalar built-in search                 |
| US-002: See request body schema     | `006-user-lens.md` | OpenAPI schema rendered by Scalar      |
| US-003: Auth token pre-populated    | `006-user-lens.md` | `use-api-explorer-config.ts`           |
| US-004: Path parameter inputs       | `006-user-lens.md` | Scalar built-in param handling         |
| US-005: Response schema             | `006-user-lens.md` | OpenAPI schema rendered by Scalar      |
| US-006: Copy curl command           | `006-user-lens.md` | Scalar built-in code snippets          |
| US-010: Browse by category          | `006-user-lens.md` | OpenAPI `tags` → Scalar sidebar groups |
| US-011: Plain-language descriptions | `006-user-lens.md` | OpenAPI endpoint descriptions          |

---

## Acceptance Criteria Coverage

| AC   | Description                          | Verification Method                          |
| ---- | ------------------------------------ | -------------------------------------------- |
| AC1  | 100% endpoint coverage               | Count visible endpoints > 100                |
| AC2  | Auth token pre-populated             | Inspect auth field in logged-in session      |
| AC3  | Workspace base URL                   | Check specUrl in hook output                 |
| AC4  | Try-it-out works for GET /health     | Manual test: 200 response                    |
| AC5  | Dark mode consistency                | Visual comparison to dashboard               |
| AC6  | Path param inputs for `{id}` routes  | Test GET /documents/{id}                     |
| AC7  | POST body schema shown               | Test POST /api/v1/query fields               |
| AC8  | 200 response schema displayed        | Test any GET endpoint                        |
| AC9  | No frontend change for new endpoints | Add test endpoint to Rust, verify it appears |
| AC10 | Same URL `/api-explorer`             | Navigation test                              |

---

## DRY Audit Before / After

| Item                   | Before                                                 | After                                              |
| ---------------------- | ------------------------------------------------------ | -------------------------------------------------- |
| Endpoint definitions   | 30 entries in `api-explorer.tsx` + 169 in `openapi.rs` | 0 in frontend + 169 in `openapi.rs`                |
| Request body examples  | Hardcoded JSON strings in component                    | Defined once in Rust `#[utoipa::path]` annotations |
| Endpoint descriptions  | Hardcoded strings in component                         | Defined once in Rust handler doc comments          |
| Auth configuration     | Not implemented at all                                 | Once in `use-api-explorer-config.ts`               |
| Base URL configuration | Hardcoded (implicit `/`)                               | Once from `getRuntimeServerBaseUrl()`              |

**DRY ratio: 120 lines of duplicated API knowledge → 1 URL.**

---

## SOLID Compliance Verification

| Principle                 | Before                                                                    | After                                                         |
| ------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------- |
| S — Single Responsibility | `api-explorer.tsx` does: list, select, execute, display, define endpoints | Page renders. Hook computes config. Theme module maps tokens. |
| O — Open/Closed           | Adding an endpoint requires modifying `api-explorer.tsx`                  | Adding an endpoint requires no frontend change                |
| L — Liskov                | N/A                                                                       | N/A                                                           |
| I — Interface Segregation | `Endpoint` type mixes route info, example body, UI state                  | Each concern in separate type/module                          |
| D — Dependency Inversion  | Component depends on hardcoded array of endpoints                         | Component depends on spec URL abstraction                     |
