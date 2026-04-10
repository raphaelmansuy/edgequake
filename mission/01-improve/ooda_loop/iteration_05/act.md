# Act

- Restored the two disputed assertions in `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs` from `OK || INTERNAL_SERVER_ERROR` back to strict `StatusCode::OK`.
- Added `clear_provider_detection_env()` at the start of every test in `e2e_query_http_workspace.rs` before any state or router construction.
- Verified the suite under the agent shell with `OPENAI_API_KEY` present:
	- `cargo test -p edgequake-api --test e2e_query_http_workspace -- --nocapture`
	- Result: 8 passed, 0 failed.
- Verified the suite with provider-selection env explicitly removed:
	- `env -u OPENAI_API_KEY -u EDGEQUAKE_LLM_PROVIDER -u EDGEQUAKE_EMBEDDING_PROVIDER cargo test -p edgequake-api --test e2e_query_http_workspace -- --nocapture`
	- Result: 8 passed, 0 failed.
- Re-ran the provider-related batch to confirm no regression in the shared-helper refactor:
	- `cargo test -p edgequake-api --test e2e_document_processing_pipeline --test e2e_document_workspace_provider --test e2e_provider_tracking_stats --test e2e_rebuild_lineage --test e2e_query_http_workspace`
	- Result: 38 passed, 0 failed.
- Formatting check passed:
	- `cargo fmt --check`

Conclusion: the real fix was removing ambient-provider-env nondeterminism from the query workspace suite and restoring the original strict success expectations.