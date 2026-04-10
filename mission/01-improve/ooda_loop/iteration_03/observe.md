# Observe

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Repository HEAD under analysis: `27f403c06b340651b7497e1e36873837ad1415ed`

## Territory Map

Existing shared harness:

- `edgequake/crates/edgequake-api/tests/common/mod.rs`

Duplicated helper definitions on the touched API test surface:

- `edgequake/crates/edgequake-api/tests/e2e_document_processing_pipeline.rs:19`
- `edgequake/crates/edgequake-api/tests/e2e_document_workspace_provider.rs:36`
- `edgequake/crates/edgequake-api/tests/e2e_document_workspace_provider.rs:48`
- `edgequake/crates/edgequake-api/tests/e2e_provider_tracking_stats.rs:16`
- `edgequake/crates/edgequake-api/tests/e2e_provider_tracking_stats.rs:28`
- `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:23`
- `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:35`
- `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:40`
- `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:51`
- `edgequake/crates/edgequake-api/tests/e2e_rebuild_lineage.rs:14`

## Verified Findings

1. `tests/common/mod.rs` already centralizes HTTP helpers and provider-environment cleanup, so adding workspace-creation helpers there is consistent with existing structure.
2. The duplicated test helpers differ mostly by which provider fields they populate, not by workflow.
3. `Tenant::new(name, slug)` accepts owned values via `impl Into<String>`, so passing owned `String`s directly is already idiomatic and avoids needless borrows.
4. `CreateWorkspaceRequest` already exposes builder methods such as `with_llm_config()` and `with_embedding_config()`, which can simplify helper construction and keep test intent explicit.

## First-Principles Constraint

The helper extraction must preserve behavior exactly: tenant creation, workspace creation, and provider settings cannot change, because the test surface is validating provider-routing semantics.

## Architecture Snapshot

```text
shared test harness (tests/common)
        |
        +--> app/router helpers
        +--> env cleanup helpers
        +--> workspace-provider helpers   <- new target slice
                    |
                    +--> document/pipeline tests
                    +--> query HTTP tests
                    +--> provider lineage tests
```
