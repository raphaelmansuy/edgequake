# Rust Full-Stack Architecture — DRY, SOLID, First Principles

**Spec:** 027-api-edgequake-audit  
**Date:** 2026-06-28 (post phase 52 — auth service-layer SSOT closed)  
**Cross-ref:** [017-dry-and-solid-audit/003-edgequake-api](../017-dry-and-solid-audit/003-edgequake-api/001-audit.md) | [005-complexity-system-lens.md](./005-complexity-system-lens.md) | [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Verdict: A++

SPEC-017 critical DRY on query/pipeline paths is **closed**. Phases 4–31: **ARCH-D-001** complete, **API-SOLID-I-001 ISP (36 handlers / ~23% of handler surface)** incl. full document query module.

**Honest A++ caveats:**

| ID | Item | Why not A+++ |
|----|------|--------------|
| ISP bulk | ~77% handlers still `State<AppState>` (119 vs 36 ISP) | upload/delete/recovery/admin handlers |
| Secure defaults | Auth on by default (AC-4) | SEC-005 JWT secret still OPEN |

**ARCH-D-001 FIXED. API-SOLID-I-001 FIXED (36 ISP handlers). Document query module COMPLETE. Auth KV quarantine COMPLETE (phase 52).**

---

## Code Re-assessment (phase 52 — AUTH CLOSED)

| Item | Status | Evidence |
|------|--------|----------|
| `auth_kv_store` callers | **2 modules only** | `identity_storage.rs` + `session_storage.rs` |
| Handlers → service layer | **DONE** | zero `auth_kv_store::` in `handlers/` |
| Contract enforcement | **DONE** | `spec027_auth_kv_store_two_callers_only_phase52` |
| Migration | **063** | new SQL only — past migrations untouched |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

`login_lockout` service + `persist_user_record` DRY. SEC-011 closed when auth on. 79+1 contract.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

| Item | Status | Evidence |
|------|--------|----------|
| `get_document` ISP | **DONE** | `detail.rs` — `StorageRuntime` + `PostgresRuntime` |
| Document query module | **COMPLETE** | list + scan + track_status + detail |
| ISP count | **36** | 119 AppState (~77%) |
| Contract | **78+1** | +`spec027_get_document_isp` |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Layer Architecture (ASCII — Current)

```
  ┌─────────────────────────────────────────────────────────┐
  │ HTTP Layer (handlers/) — THIN post phase 6              │
  │   validate DTO → delegate to services/run_* → map JSON  │
  │   v2 jobs: handlers.rs → submission.rs → run_* cores    │
  └────────────────────────┬────────────────────────────────┘
                           │
  ┌────────────────────────▼────────────────────────────────┐
  │ Service Layer (services/) — SUBSTANTIAL                   │
  │   ✅ query_execution, entity_merge, entity_neighborhood   │
  │   ✅ cost_aggregation, isolation_context, tenant_guard    │
  │   ✅ workspace_document_index, document_metadata_scan     │
  │   ✅ job_registry, v1_rpc_migration, entity_graph_lookup  │
  └────────────────────────┬────────────────────────────────┘
                           │
  ┌────────────────────────▼────────────────────────────────┐
  │ State (AppState + runtimes) — IMPROVED post SPEC-017      │
  │   StorageRuntime, QueryRuntime, AuthRuntime, TaskRuntime  │
  │   FromRef: StorageRuntime, GraphQueryRuntime, AuthRuntime, ComplianceRuntime      │
  │   35 ISP handlers (relationships/lineage/graph/docs/auth)  │
  └────────────────────────┬────────────────────────────────┘
                           │
  ┌────────────────────────▼────────────────────────────────┐
  │ Storage traits (edgequake-storage) — ISP split good       │
  │   graph_read_ops / scan_ops / mutate_ops                  │
  └───────────────────────────────────────────────────────────┘
```

---

## DRY Scorecard (Code-Verified Phase 23)

| ID | Finding | Status | Evidence |
|----|---------|--------|----------|
| ARCH-001 | Dual isolation SSOT | **FIXED** | `services/isolation_context.rs` — dual `IsolationMode` + contract tests |
| ARCH-002 | Lineage entity normalize bypass | **FIXED** | `entity_name_normalize.rs` + `lineage/normalize.rs` |
| ARCH-003 | Tenant guard boilerplate | **FIXED** | `services/tenant_guard.rs` wired in handlers |
| ARCH-004 | Cost aggregation duplicated | **FIXED** | `services/cost_aggregation.rs` |
| ARCH-005 | Raw `-metadata` format! literals | **FIXED** | `metadata_key_for_document` — contract walks `src/` |
| ARCH-006 | Graph edge → DTO mapping duplicated | **FIXED** | `GraphEdgeResponse::from_storage_edge` — search/traversal/stream |
| ARCH-007 | v1 RPC logic duplicated in v2 | **FIXED** | 6/6 `run_*` cores; `submission.rs` dispatches to inner fns |
| ARCH-008 | Job type catalog validation | **FIXED** | `job_registry::is_creatable_v2_job_type` → `submission.rs` |
| ARCH-009 | v1 RPC migration headers | **FIXED** | `respond_v1_async_rpc` — all 6 RPC; REST-025 **default 202** |

---

## SOLID Scorecard (Code-Verified Phase 23)

| ID | Principle | Finding | Status | Evidence |
|----|-----------|---------|--------|----------|
| ARCH-S-001 | SRP | God handler files | **FIXED** | `injection/` split (max 376 LOC/file); `storage_helpers.rs` facade 131 LOC |
| ARCH-S-002 | SRP | Metadata write drift | **FIXED** | `upsert_metadata_kv_with_index` — 12 paths contract-tested |
| ARCH-S-003 | SRP | OpenAPI monolith | **FIXED** | `openapi_enrichment`, `openapi_security`, `openapi_annotation_sync`, `openapi_examples` |
| ARCH-D-001 | DIP | Auth extractors unused | **FIXED** | `ApiRequireAdmin`, `ApiOptionalAuth`, `ApiAuthenticated` |
| ARCH-I-001 | ISP | Full AppState everywhere | **FIXED (pattern)** | 35 handlers: Storage + GraphQuery + Auth + Compliance runtimes |
| ARCH-L-001 | LSP | Graph legacy edge mode | **DOCUMENTED** | `EdgeTenantFilterMode::LegacyNullAsWildcard` — explicit dual mode |
| ARCH-O-001 | OCP | New isolation dimension | **MITIGATED** | `IsolationContext` service; adding dims still touches filters |

---

## v1 RPC SOLID Extract (Phase 22–23)

All six v1 long-running RPC handlers expose `pub(crate) async fn run_*` cores. v2 `submission.rs` calls these directly — **not** HTTP wrappers.

| RPC handler | `run_*` core | REST-024 headers on v1 |
|-------------|--------------|------------------------|
| `rebuild_embeddings.rs` | `run_rebuild_embeddings` | ✅ |
| `rebuild_knowledge_graph.rs` | `run_rebuild_knowledge_graph` | ✅ |
| `reprocess_documents.rs` | `run_reprocess_all_documents` | ✅ |
| `reprocess.rs` | `run_reprocess_failed` | ✅ |
| `stuck.rs` | `run_recover_stuck` | ✅ |
| `reanalyze.rs` | `run_reanalyze_multimodal` | ✅ **phase 23** |

Contract: `spec027_run_reanalyze_multimodal_extracted`, `spec027_v1_rpc_migration_headers_ssot`.

---

## SPEC-017 Regression Status

| ID | Item | Status |
|----|------|--------|
| API-DRY-001 | Pipeline factory | ✅ Fixed |
| API-DRY-002 | Query execution shared | ✅ Fixed |
| API-SOLID-S-001 | AppState decomposition | ✅ Fixed (runtimes) |
| API-DRY-008 | Tenant boilerplate | ✅ Fixed → ARCH-003 |
| API-SOLID-I-001 | ISP extractors | ✅ Fixed → ARCH-I-001 |

Contract: `spec017_api_contract.rs` — 9 tests (SPEC-017 scope, incl. FromRef ISP + AuthState). SPEC-027 adds 78 contract tests (77 pass + 1 ignored) + 32 e2e.

---

## Positive Architecture Signals

```
  ✅ isolation_context.rs — centralized graph IDOR + dual document mode
  ✅ vertex_filter.rs — SQL SSOT aligned with isolation_context
  ✅ entities split: entity_crud vs entity_ops vs entity_neighborhood service
  ✅ relationships split: list/create/update/delete/helpers
  ✅ graph_query split: traversal/search/popular/node
  ✅ services/query_execution.rs — query/chat/stream parity
  ✅ WorkspacePipelineFactory — ingestion path SSOT
  ✅ ApiError semantic mapping from QueryError
  ✅ job_registry.rs — v2 catalog + migration hints + creatable type validation
  ✅ v1_rpc_migration.rs — Sunset/Link + REST-025 default 202 SSOT
  ✅ graph_query_runtime.rs — materialization semaphore + timeout budget ISP
  ✅ graph_materialization.rs — admits via GraphQueryRuntime (not AppState)
  ✅ GraphEdgeResponse::from_storage_edge — graph edge DTO SSOT (ARCH-006)
  ✅ build.rs OpenAPI path SSOT — compile-time drift guard
```

---

## First-Principles Target Module Map (Status)

```
  handlers/
    └── thin (≤400 LOC each)                    ✅ post phase 6
  services/
    ├── query_execution.rs                      ✅
    ├── entity_merge.rs                         ✅ phase 18
    ├── entity_neighborhood.rs                  ✅
    ├── cost_aggregation.rs                     ✅
    ├── isolation_context.rs                    ✅
    ├── workspace_document_index.rs             ✅ phases 8–9
    ├── document_metadata_scan.rs               ✅
    ├── job_registry.rs                         ✅ phase 19
    ├── v1_rpc_migration.rs                     ✅ phase 22
    └── document_lifecycle (split services)     ✅ partial — document_* modules
  middleware/
    └── auth_context.rs (attach AuthContext)    ✅ session ISP + extractors (ARCH-D-001)
```

---

## Rust-Specific Notes

| Topic | Assessment |
|-------|------------|
| Error handling | `ApiResult<T>`, `thiserror` — good |
| Async | `tokio`, no blocking in handlers — good |
| Cloning | AppState Arc — acceptable |
| `unwrap()` in handlers | Some `unwrap_or(0)` on degree — hides errors (P3) |
| Feature flags | `postgres` cfg in handlers — OCP debt (SPEC-017) |
| Tracing | `tracing` crate used — good |
| Test discipline | 77 contract pass + 1 ignored + 32 e2e + 9 spec017 |

---

## Cross-Ref to Performance

ARCH-001 isolation unification enabled PERF-006 traversal push-down. wsdoc index (phases 8–10) closed PERF-KV-001. See [005-complexity-system-lens.md](./005-complexity-system-lens.md).

---

## Historical Findings (Pre-Phase 9 — Superseded)

The sections below document **audit-time debt** that phases 4–23 closed. Retained for traceability only; **do not treat as open**.

<details>
<summary>Original ARCH-001..006 and ARCH-S-001 findings (pre-fix)</summary>

- ARCH-001 dual isolation: `isolation.rs` vs `workspace_scope.rs` — **FIXED** via `isolation_context.rs`
- ARCH-002 ad-hoc normalize in lineage — **FIXED**
- ARCH-003 tenant guard copy-paste — **FIXED**
- ARCH-004 cost aggregation dup — **FIXED**
- ARCH-005 graph tenant stamping dual paths — **FIXED** via metadata key SSOT
- ARCH-S-001 god files (875 LOC injection, 768 LOC storage_helpers) — **FIXED** via module split

</details>

---

## API-SOLID-I-001 — FromRef ISP (Phase 25–28)

| Runtime | `FromRef<AppState>` | Migrated handlers |
|---------|---------------------|-------------------|
| `StorageRuntime` | ✅ | 22 handlers (relationships, lineage, graph, track_status, list, scan, auth) |
| `GraphQueryRuntime` | ✅ | search_nodes, get_graph, get_popular_labels, graph_stream |
| `AuthRuntime` | ✅ | login, refresh, create_user, api_keys |
| `ComplianceRuntime` | ✅ | login, logout (audit path) |
| `PostgresRuntime` | ✅ | list_documents (relational backfill) |
| `TaskRuntime` | ✅ | scan_directory |
| `QueryRuntime` | ✅ | — (available) |
| `ResourceBudgetConfig` | ✅ | list_relationships, list_documents |
| `PathValidationConfig` | ✅ | scan_directory |
| `AppConfig` | ✅ | scan_directory |
| `AuthState` | ✅ | JWT-only handlers |

Contract: `spec017_runtime_from_ref_extractors_wired`, `spec027_relationship_handlers_use_storage_runtime_isp`, `spec027_auth_extractors_arch_d001`, `spec027_list_documents_isp`, `spec027_scan_directory_partial_isp`.

---

## Code Re-assessment (phase 37)

| Item | Status | Evidence |
|------|--------|----------|
| RLS acquire/release SSOT | **DONE** | `acquire_rls_connection` in `rls.rs` |
| Conversation DRY | **DONE** | delegates to SSOT |
| Legacy RlsContext | **DEPRECATED** | ascending-compat re-export with allow |
| Contract tests | **87+1** | +phase 37 |

**Verdict: A++ retained.**

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 36)

| Item | Status | Evidence |
|------|--------|----------|
| `conversation.rs` RLS acquire/release | **DONE** | DRY private helpers; 9 call sites |
| Legacy pool `set_context` | **REMOVED** | contract enforced |
| Clippy `query_ops.rs` | **FIXED** | `if let (Some, Some)` — no unnecessary unwrap |
| Contract tests | **86+1** | +conversation RLS phase 36 |

**Verdict: A++ retained.**

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 35)

| Item | Status | Evidence |
|------|--------|----------|
| `tenant_isolation.rs` | **DONE** | 3 layers + dual KV+PG verdict |
| RLS acquire helper | **DONE** | `edgequake-storage/rls.rs` |
| Contract tests | **85+1** | +m050 + tenant_isolation |

**Verdict: A++ retained.**

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 34)

| Item | Status | Evidence |
|------|--------|----------|
| `identity_storage` membership + JWT scope | **DONE** | DRY SSOT expanded |
| Middleware Send-safe membership bind | **DONE** | sync extract + async verify |
| Migration 049 | **DONE** | bootstrap m049 |
| Contract tests | **83 pass + 1 ignored** | +migration 049 |

**Verdict: A++ retained.** Identity storage SSOT complete for default scope.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

| Item | Status | Evidence |
|------|--------|----------|
| Auth session ISP | **DONE** | login/refresh/logout runtimes + ComplianceRuntime audit |
| Document list/scan ISP | **DONE** | StorageRuntime + PostgresRuntime/TaskRuntime/PathValidationConfig |
| ComplianceRuntime | **DONE** | audit path without AppState |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **77 pass + 1 ignored** | +auth + list/scan ISP |
| E2E tests | **32 pass** | +legacy opt-out |

**Verdict: A++ retained.** ISP 35 handlers (~78% AppState remainder).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 29)

| Item | Status | Evidence |
|------|--------|----------|
| graph_stream ISP | **DONE** | SSE spawn: StorageRuntime + GraphQueryRuntime |
| Legacy graph adapters removed | **DONE** | no `*_from_state` in graph_materialization |
| get_me auth extractor | **DONE** | ApiAuthenticated + StorageRuntime |
| track_status ISP | **DONE** | StorageRuntime |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **75 pass + 1 ignored** | expanded |
| E2E tests | **31 pass** | unchanged |

**Verdict: A++ retained.** ISP at 21 handlers. graph materialization fully on GraphQueryRuntime.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 28)

| Item | Status | Evidence |
|------|--------|----------|
| GraphQueryRuntime ISP | **DONE** | `graph_query_runtime.rs` + materialization refactor |
| Graph search/traversal/popular ISP | **DONE** | 4 handlers |
| OpenAPI 202 all 6 RPC | **DONE** | utoipa |
| api_keys auth extractors | **DONE** | ApiAuthenticated × 3 |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **75 pass + 1 ignored** | expanded REST-025 + ISP |
| E2E tests | **31 pass** | unchanged |

**Verdict: A++ retained.** ISP at 19 handlers.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 27)

| Item | Status | Evidence |
|------|--------|----------|
| REST-025 strict-startup bundle | **DONE** | `parse_bool_env_or_fallback` in `security_config.rs` |
| ARCH-D-001 complete pattern | **DONE** | `ApiOptionalAuth` + create_user ISP |
| ISP bulk (+8 handlers) | **DONE** | lineage queries/export, graph node, degrees batch |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **75 pass + 1 ignored** | +2 phase 27 |
| E2E tests | **31 pass** | unchanged |

**Verdict: A++ retained.** ARCH-D-001 **FIXED**. ISP at 15 handlers.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 26)

| Item | Status | Evidence |
|------|--------|----------|
| REST-025 opt-in 202 | **DONE** | `security_config.v1_rpc_return_202`, `respond_v1_async_rpc` |
| ARCH-D-001 extractors | **DONE** | `ApiRequireAdmin` on admin.rs + user_management admin CRUD |
| Lineage ISP migration | **DONE** | `entity_provenance.rs`, `chunk_detail.rs` |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **74 pass + 1 ignored** | +2 phase 26 |
| E2E tests | **31 pass** | +REST-025 opt-in 202 |

**Verdict: A++ retained.** ARCH-D-001 **FIXED (pattern)**. ISP bulk remains incremental optional work.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 25)

| Item | Status | Evidence |
|------|--------|----------|
| API-SOLID-I-001 FromRef wired | **DONE** | `state/runtime_extractors.rs` |
| Relationships ISP migration | **DONE** | 5 handlers, zero `State<AppState>` |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **73 pass + 1 ignored** | +1 ISP contract |
| E2E tests | **30 pass** | unchanged |

**Verdict: A++ retained.** API-SOLID-I-001 **FIXED**. ARCH-D-001 closed phase 26.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 24)

| Item | Status | Evidence |
|------|--------|----------|
| ARCH-006 graph edge DTO SSOT | **DONE** | `GraphEdgeResponse::from_storage_edge` in `graph_types.rs` |
| Call sites deduplicated | **DONE** | `search.rs`, `traversal.rs` (×2), `graph_stream.rs` |
| Relationship mapping | **Separate SSOT** | `edge_to_relationship_response` — different DTO domain |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **72 pass + 1 ignored** | +1 `spec027_graph_edge_response_from_storage_edge_ssot` |
| E2E tests | **30 pass** | unchanged |

**Verdict: A++ retained.** Only ARCH-D-001 and ARCH-I-001 remain OPEN (ascending-compat deferred).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 23)

| Item | Status | Evidence |
|------|--------|----------|
| `run_reanalyze_multimodal` extract | **DONE** | `recovery/reanalyze.rs` |
| v2 submission uses all 6 `run_*` | **DONE** | `v2/jobs/submission.rs` |
| `is_creatable_v2_job_type` catalog SSOT | **DONE** | `job_registry.rs` → `submission.rs` |
| v2 202 Link header (RFC 8288 self) | **DONE** | `create_workspace_job` in `handlers.rs` |
| Bootstrap migration | **NOT NEEDED** | — |
| Contract tests | **71 pass + 1 ignored** | 72 defined; snapshot refresh manual |
| E2E tests | **30 pass** | includes Link header + unknown job_type 400 |

**Verdict: A++ retained.** Remaining OPEN items at phase 23: ARCH-D-001, ARCH-I-001, ARCH-006 (closed phase 24).

---

## Code Re-assessment (phase 40)

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 41)

`test_state_with_pg_pool` + `with_optional_pg_rls` — PG auth E2E harness and RLS wiring SSOT. 94+1 contract + 2 pg auth e2e.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

DRY: shared `acquire_optional_pg_connection` across identity, session, pdf_lineage. Migration 054. 97+1 contract + 4 pg auth e2e.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 44)

`resolve_auth_enabled_from_env` + `AuthConfig.dev_mode`. Test harness struct-update pattern.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45)

`auth_kv_store.rs` — SRP for KV auth. `IdentityPolicy::identity_backend_label()`. Migration 056.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

`IdentityPolicy` hard-ignores `EDGEQUAKE_KV_IDENTITY_MIRROR` when PG pool. Migration 058. 5 PG auth E2E.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)
