# Act

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Implementation state: working tree (uncommitted)

## Completed Changes

1. Expanded the shared E2E harness in `edgequake/crates/edgequake-api/tests/common/mod.rs`.
   - Added shared app/server helpers at lines `97`, `108`, and `113`.
   - Added shared workspace-provider setup helpers at lines `134` and `165`.

2. Replaced file-local setup duplication with shared helpers in:
   - `edgequake/crates/edgequake-api/tests/e2e_document_processing_pipeline.rs:9`
   - `edgequake/crates/edgequake-api/tests/e2e_document_workspace_provider.rs:11`
   - `edgequake/crates/edgequake-api/tests/e2e_provider_tracking_stats.rs:9`
   - `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:10`
   - `edgequake/crates/edgequake-api/tests/e2e_rebuild_lineage.rs:9`

3. Replaced narrower provider-environment cleanup with the shared deterministic cleanup helper in:
   - `edgequake/crates/edgequake-api/tests/e2e_provider_tracking_stats.rs:11`
   - `edgequake/crates/edgequake-api/tests/e2e_rebuild_lineage.rs:11`

## Verification Run

Executed from `edgequake/` workspace root:

- `cargo test -p edgequake-api --test e2e_rebuild_lineage` -> passed (`6 passed`)
- `cargo test -p edgequake-api --test e2e_document_processing_pipeline` -> passed (`7 passed`)
- `cargo test -p edgequake-api --test e2e_document_workspace_provider` -> passed (`8 passed`)
- `cargo test -p edgequake-api --test e2e_provider_tracking_stats` -> passed (`9 passed`)
- `cargo test -p edgequake-api --test e2e_query_http_workspace` -> failed (`6 passed, 2 failed`)
- `cargo fmt --check` -> not reached because the command batch stopped on the failing query test

## Failure Evidence

`e2e_query_http_workspace` now reports two stable failures tied to OpenAI-configured embedding workspaces:

- `test_query_http_workspace_provider_isolation` at `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:180`
- `test_query_http_workspace_openai_config` at `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs:387`

Observed status mismatch in both cases:

```text
left: 500
right: 200
```

An isolated rerun of `test_query_http_workspace_openai_config` reproduced the same `500`, indicating this is not a cross-test artifact.

## Result

The DRY refactor succeeded for four targeted integration suites and exposed a pre-existing brittle assumption in the OpenAI query-path tests. The next iteration should make those query assertions reflect the repository's actual test-mode provider behavior.

## Commit Status

No commit created in this iteration.
