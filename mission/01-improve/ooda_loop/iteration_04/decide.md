# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Repository HEAD under analysis: `27f403c06b340651b7497e1e36873837ad1415ed`

## Exact Scope

This iteration will:

1. Adjust the two failing OpenAI-query tests in `e2e_query_http_workspace.rs` so they accept the repository's real test-mode outcomes.
2. Keep body assertions only on the success path.
3. Re-run the targeted `e2e_query_http_workspace` suite and `cargo fmt --check`.

This iteration will not:

- change production query behavior
- change mock-path expectations that already pass
- broaden unrelated tests

## Why This Slice

The previous iteration already proved the helper extraction. The smallest high-value next move is to remove the brittle assumption that blocks the verification batch while preserving actual provider-routing signal.

## Verification Plan

- `cargo test -p edgequake-api --test e2e_query_http_workspace`
- `cargo fmt --check`
