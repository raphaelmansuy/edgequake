# Decide

- Revert the weakened assertions in `e2e_query_http_workspace.rs` back to strict `StatusCode::OK` expectations.
- Add explicit `clear_provider_detection_env()` calls at the start of each query-workspace test so the suite no longer depends on developer or CI credentials.
- Keep the helper extraction from `iteration_03`; only change the deterministic setup and the disputed assertions.
- Verify the suite under both ambient-env and env-unset process conditions.# Decide — Iteration 05

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`

## Scope

Replace `serde_json::to_value().unwrap()` → `.expect()` in these production files:

1. `crates/edgequake-api/src/handlers/workspaces/bulk_ops/mod.rs` (2 sites)
2. `crates/edgequake-api/src/handlers/documents/recovery/stuck.rs` (1 site)
3. `crates/edgequake-api/src/handlers/documents/recovery/reprocess.rs` (2 sites)
4. `crates/edgequake-api/src/handlers/documents/query/scan.rs` (1 site)
5. `crates/edgequake-api/src/handlers/documents/upload/text_upload.rs` (1 site)

## Verification Plan

- `cargo test -p edgequake-api --lib`
- `cargo clippy -p edgequake-api`
