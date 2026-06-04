# edgequake-api — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-api`  
**LOC:** ~51,427 (src) | ~130 handler files (~65% of crate)  
**Role:** HTTP entry point (Axum), state bootstrapping, document processors  
**Last verified:** 2026-06-04T05:58Z (re-run #5 — all green)

---

## Executive Summary

**All P0 and in-scope P1 items fixed and proven.** Full pipeline proven at four layers: sync, async, PDF, and Playwright UI. P2 structural debt partially deferred.

| Priority | Open | Fixed / Improved |
|----------|------|------------------|
| P0 | 0 | 5 |
| P1 | 1 (deferred) | 7 |
| P2/P3 | 4 (deferred) | 2 |

---

## DRY Violations

| ID | P | Violation | Status | Evidence / fix |
|----|---|-----------|--------|----------------|
| API-DRY-001 | **P0** | Three workspace pipeline builders | ✅ **Fixed** | `workspace_pipeline_factory.rs`; Strict/Lenient policies |
| API-DRY-002 | **P0** | Query execution matrix duplicated | ✅ **Fixed** | `services/query_execution.rs` — shared by query + chat + stream |
| API-DRY-003 | **P1** | Memory vs Postgres bootstrap ~150 LOC duplicate | ✅ **Fixed** | `state/query_bootstrap.rs` — shared pipeline + query engines |
| API-DRY-004 | **P1** | Task create + enqueue copy-pasted | ✅ **Fixed** | `AppState::enqueue_task` → `TaskRuntime::enqueue` |
| API-DRY-005 | **P1** | Workspace ID resolution inconsistent | ✅ **Fixed** | `middleware::parse_workspace_id` + `resolve_workspace_uuid` |
| API-DRY-006 | **P1** | `QueryError` → generic 500 | ✅ **Fixed** | `From<QueryError>` + `validate_llm_override_pair` on query handlers |
| API-DRY-007 | **P2** | Hardcoded provider catalog ~250 LOC | ⬜ **Deferred** | `provider_types.rs` — registry from config planned |
| API-DRY-008 | **P2** | Tenant context extraction boilerplate | ⬜ **Deferred** | Pattern still repeated; low risk |
| API-DRY-009 | **P2** | Manual API→core DTO mapping | ⬜ **Deferred** | `From` impls not added |

---

## SOLID Violations

| ID | P | Principle | Violation | Status |
|----|---|-----------|-----------|--------|
| API-SOLID-S-001 | **P0** | SRP | `AppState` god object | ✅ **Fixed** | Composed: `StorageRuntime`, `QueryRuntime`, `AuthRuntime`, `TaskRuntime` |
| API-SOLID-S-002 | **P1** | SRP | State owns operational methods | 🟡 **Improved** | Pipeline factory + query bootstrap extracted; SQL bootstrap still on AppState |
| API-SOLID-L-001 | **P0** | LSP | Pipeline silent fallback vs strict | ✅ **Fixed** | `PipelineFallbackPolicy::Strict \| LenientGlobal` |
| API-SOLID-L-002 | **P1** | LSP | Memory `pdf_storage: None` vs Postgres `Some` | 🟡 **Improved** | `StorageRuntime::validate_postgres_adapters`; handlers still cfg-aware |
| API-SOLID-I-001 | **P1** | ISP | Every handler receives full `AppState` | ⬜ **Deferred** | Runtime bundles exist; `FromRef` sub-states not wired |
| API-SOLID-D-001 | **P0** | DIP | query bypasses resolver | ✅ **Fixed** | All query/chat paths use `WorkspaceProviderResolver` |
| API-SOLID-D-002 | **P1** | DIP | Resolver constructed per request | 🟡 **Improved** | `from_app_state()` — cheap clone; Arc on AppState deferred |
| API-SOLID-O-001 | **P2** | OCP | `#[cfg(feature = "postgres")]` in handlers | ⬜ **Deferred** | Trait-based boot registration planned |
| API-SOLID-O-002 | **P1** | OCP | New provider = edit API crate | ⬜ **Deferred** | Tied to API-DRY-007 |

---

## Largest Files (SRP pressure — unchanged)

| File | ~LOC | Status |
|------|------|--------|
| `pipeline_progress_callback.rs` | 1,326 | ⬜ Phase 3 split |
| `processor/text_insert.rs` | 1,144 | ⬜ Phase 3 split |
| `handlers/injection.rs` | 1,014 | ⬜ Phase 3 split |
| `processor/pdf_processing.rs` | 995 | ⬜ Phase 3 split |
| `middleware.rs` | 747 | ⬜ Phase 3 split |

---

## Verification (2026-06-04T05:48:43Z — re-run #4)

```bash
./specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/run_api_e2e.sh
./specs/017-dry-and-solid-audit/003-edgequake-api/001-audit/e2e/run_api_e2e.sh --playwright
```

| Suite | Result |
|-------|--------|
| `spec017_api_contract` | 8/8 |
| `spec017_query_production_path_contract` | 2/2 |
| `e2e_workspace_pipeline_integration` | pass |
| `e2e_query_routing_parity` | 3/3 |
| `e2e_workspace_provider_ingestion` | pass |
| `e2e_query` partial override | 400 ✅ |
| `edgequake-api --lib` | 596/596 |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo fmt --all -- --check` | pass |
| Playwright `spec017-api-query-documents.spec.ts` | 6/6 (19.4s — sync 4.7s, async 5.8s, PDF 6.1s) |

**E2E artifacts:** `001-audit/e2e/000-e2e-index.md`, `001`–`006-*.md`, `001-test-run.log`, `screenshots/01`–`06-*.png`.

**Acceptance:** grep shows single pipeline factory; query and chat call same execution service ✅

---

## Brutal Honest Assessment

**Ship-ready for audit scope:** Every P0 item and all actionable P1 DRY items are fixed and proven. Full pipeline is **proven** at four layers:

1. **Rust contract** (`spec017_api_contract` 8/8) — factory, shared query service, bootstrap dedup, error mapping.
2. **Rust integration** — workspace pipeline factory + query/chat routing parity.
3. **Live API** — sync (8 entities, 4.4s), async (8 entities, 7.8s poll), PDF text parser (6.0s).
4. **Playwright UI** — health, query, documents, sync Completed (`05`), async Completed (`06`).

**Honest limits:**

- **API-SOLID-I-001** — handlers still receive full `AppState`; runtime bundles are structural only.
- **API-SOLID-D-002** — resolver not cached on AppState (acceptable; clones one Arc).
- **API-DRY-007 / API-SOLID-O-002** — provider catalog still hardcoded (~250 LOC).
- **1k+ LOC handler modules** — not split (Phase 3).
- **Vision PDF** and **mock workspace partial extraction** — not tested in this crate folder.
- Storage-mode-specific handler `#[cfg]` remains (API-SOLID-O-001).

**Acceptance:** ✅ All P0 + P1 DRY items fixed; ✅ full sync/async/PDF pipeline proven via API; ✅ no regression; ✅ compile/clippy/fmt clean.

---

## Positive Patterns (Keep)

- Handler / `*_types` module pairing (`handlers/mod.rs:4-5`)
- `From<edgequake_core::Error>` and `From<ProviderResolutionError>` in `error.rs`
- `WorkspaceProviderResolver` (OODA-226) as query-time abstraction
- `state/provider_setup.rs` embedding override consolidation (SPEC-140)
- `WorkspacePipelineFactory` with explicit fallback policies (SPEC-017)
- `state/query_bootstrap.rs` shared memory/postgres engine wiring (SPEC-017)
