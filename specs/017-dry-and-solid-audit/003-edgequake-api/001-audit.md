# edgequake-api — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-api`  
**LOC:** ~51,427 (src) | ~130 handler files (~65% of crate)  
**Role:** HTTP entry point (Axum), state bootstrapping, document processors

---

## Executive Summary

The API crate correctly separates HTTP DTOs from domain types via `*_types` modules. The critical debt is **operational logic duplication** — three workspace pipeline builders, dual query execution paths, and a 25-field `AppState` god object. These are **P0 correctness risks**, not style issues.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| API-DRY-001 | **P0** | Three workspace pipeline builders | `state/mod.rs:340-416` (`create_workspace_pipeline`); `processor/workspace_resolver.rs:22-178` (`get_workspace_pipeline`); `get_workspace_pipeline_strict` | Extract `WorkspacePipelineFactory` with explicit `FallbackPolicy::Strict \| Lenient` |
| API-DRY-002 | **P0** | Query execution matrix duplicated | `handlers/query/query_execute.rs:205-265` mirrors `handlers/chat/completion.rs:356-401` | `QueryExecutionService::execute_with_workspace_config()` |
| API-DRY-003 | **P1** | Memory vs Postgres bootstrap ~150 LOC duplicate | `state/memory.rs:156-249` vs `state/postgres.rs:290-393` | `build_common_state(deps) -> AppStateCore` |
| API-DRY-004 | **P1** | Task create + enqueue copy-pasted 6+ times | `pdf_upload/helpers.rs:103-110`, `documents/recovery/*.rs`, `text_upload.rs:250-257` | `AppState::enqueue_task(&Task) -> ApiResult<String>` |
| API-DRY-005 | **P1** | Workspace ID resolution inconsistent | `middleware.rs:448` honors `default`; `query/workspace_resolve.rs:22` raw `Uuid::parse_str` | Single `parse_workspace_id(&str) -> Result<Uuid, ApiError>` |
| API-DRY-006 | **P1** | `QueryError` → generic 500 | Handlers use `.map_err(\|e\| ApiError::Internal(...))` despite `ApiError::Query(#[from] QueryError)` at `error.rs:155` | Use `?`; map `InvalidQuery`→400 |
| API-DRY-007 | **P2** | Hardcoded provider catalog ~250 LOC | `provider_types.rs:159-416` | Derive from `ModelsConfig` + env |
| API-DRY-008 | **P2** | Tenant context extraction boilerplate | Pattern repeated across conversations, pdf_upload, etc. | `TenantContext::require_tenant()?` helper |
| API-DRY-009 | **P2** | Manual API→core DTO mapping | `conversations_types/requests.rs` → `CreateConversationRequest` without `From` | `impl From<ApiRequest> for CoreRequest` |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence | Remediation |
|----|---|-----------|-----------|----------|-------------|
| API-SOLID-S-001 | **P0** | SRP | `AppState` god object | `state/mod.rs:118-221` — 25+ fields: storage, LLM, query, pipeline, auth, cache, rate limit | Split `StorageState`, `ProviderState`, `AuthState`, `TaskState` |
| API-SOLID-S-002 | **P1** | SRP | State owns operational methods | `state/mod.rs:232-416` pipeline factory + SQL bootstrap | `BootstrapService`, `WorkspacePipelineService` |
| API-SOLID-L-001 | **P0** | LSP | Pipeline resolution: silent fallback vs strict failure | `create_workspace_pipeline` returns global pipeline on miss; strict path fails task | One interface, two explicit policies |
| API-SOLID-L-002 | **P1** | LSP | Memory mode: `pdf_storage: None`; Postgres: `Some(...)` | `memory.rs:238` vs `postgres.rs:347` | Capability flags or mode-specific handler registration |
| API-SOLID-I-001 | **P1** | ISP | Every handler receives full `AppState` | Universal `State(AppState)` in `handlers/mod.rs:32` | `QueryHandlerState`, `FromRef` sub-states |
| API-SOLID-D-001 | **P0** | DIP | `query_execute` bypasses `WorkspaceProviderResolver` | `query_execute.rs:189-199` calls `create_safe_llm_provider_with_headers` directly; chat uses resolver | Route all provider creation through resolver |
| API-SOLID-D-002 | **P1** | DIP | Resolver constructed per request | `WorkspaceProviderResolver::new(...)` in chat, query_stream, workspace_resolve | `Arc<WorkspaceProviderResolver>` on `AppState` at boot |
| API-SOLID-O-001 | **P2** | OCP | `#[cfg(feature = "postgres")]` in handlers | `pdf_upload/helpers.rs:23-35` | Trait on `AppState` set at boot; handlers cfg-free |
| API-SOLID-O-002 | **P1** | OCP | New provider = edit API crate | `provider_types.rs` hardcoded list | Registry from `edgequake_llm` |

---

## Largest Files (SRP pressure)

| File | ~LOC | Concerns mixed |
|------|------|----------------|
| `pipeline_progress_callback.rs` | 1,326 | Progress + WebSocket + state |
| `processor/text_insert.rs` | 1,144 | Insert + pipeline + task |
| `handlers/injection.rs` | 1,014 | Injection + upload + pipeline |
| `processor/pdf_processing.rs` | 995 | PDF + vision fallback + provider |
| `middleware.rs` | 747 | Auth + tenant + workspace |

---

## Remediation Plan

### Phase 1 — P0 (1 sprint)

1. **Unify pipeline resolution** → `src/providers/workspace_pipeline_factory.rs`
2. **Extract `QueryExecutionService`** → shared by query + chat handlers
3. **Inject `WorkspaceProviderResolver` on AppState** → remove per-request construction
4. **Fix QueryError mapping** → semantic HTTP status codes

### Phase 2 — P1 (1-2 sprints)

5. Split `AppState` into composed sub-states
6. Shared memory/postgres bootstrap
7. Centralize task enqueue
8. Provider catalog from config

### Phase 3 — P2/P3

9. Tenant context helper; DTO `From` impls
10. Split oversized handler modules

---

## Verification

```bash
# Pipeline resolution
cargo test -p edgequake-api --test e2e_workspace_pipeline_integration

# Query/chat parity
cargo test -p edgequake-api --test e2e_query

# Provider resolution
cargo test -p edgequake-api --test e2e_workspace_provider_ingestion
```

**Acceptance:** grep shows single `create_workspace_pipeline` definition; query and chat handlers call same execution service.

---

## Positive Patterns (Keep)

- Handler / `*_types` module pairing (`handlers/mod.rs:4-5`)
- `From<edgequake_core::Error>` and `From<ProviderResolutionError>` in `error.rs`
- `WorkspaceProviderResolver` (OODA-226) as query-time abstraction
- `state/provider_setup.rs` embedding override consolidation (SPEC-140)
