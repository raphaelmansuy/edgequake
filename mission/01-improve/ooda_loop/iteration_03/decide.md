# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Repository HEAD under analysis: `27f403c06b340651b7497e1e36873837ad1415ed`

## Exact Scope

This iteration will:

1. Add focused workspace/provider setup helpers to `edgequake-api/tests/common/mod.rs`.
2. Replace local duplicate setup helpers in the five touched E2E test files with shared helpers.
3. Replace narrower provider-env cleanup where safe with the shared deterministic cleanup helper.
4. Run targeted `edgequake-api` tests covering the refactored files.

This iteration will not:

- refactor unrelated API tests outside the touched surface
- change production code
- widen provider behavior assertions

## Why This Slice

The risk-adjusted payoff is high:

- small surface area
- no production runtime risk
- immediate DRY gain on currently edited files
- lower future maintenance cost for provider-routing work

## Verification Plan

- `cargo test -p edgequake-api --test e2e_rebuild_lineage`
- `cargo test -p edgequake-api --test e2e_document_processing_pipeline`
- `cargo test -p edgequake-api --test e2e_document_workspace_provider`
- `cargo test -p edgequake-api --test e2e_provider_tracking_stats`
- `cargo test -p edgequake-api --test e2e_query_http_workspace`
- `cargo fmt --check`
