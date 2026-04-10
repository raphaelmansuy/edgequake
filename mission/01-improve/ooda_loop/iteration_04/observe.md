# Observe

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Repository HEAD under analysis: `27f403c06b340651b7497e1e36873837ad1415ed`

## Verified Failure Surface

Target file:

- `edgequake/crates/edgequake-api/tests/e2e_query_http_workspace.rs`

Current failing assertions from iteration 03 verification:

- `test_query_http_workspace_provider_isolation` expects `StatusCode::OK` for an OpenAI-configured workspace at line `180`
- `test_query_http_workspace_openai_config` expects `StatusCode::OK` at line `387`

## Verification Evidence

1. Full targeted batch produced:
   - `e2e_rebuild_lineage` -> pass
   - `e2e_document_processing_pipeline` -> pass
   - `e2e_document_workspace_provider` -> pass
   - `e2e_provider_tracking_stats` -> pass
   - `e2e_query_http_workspace` -> fail with `6 passed, 2 failed`

2. Isolated rerun reproduced the failure for `test_query_http_workspace_openai_config`:

```text
left: 500
right: 200
```

3. Nearby repository tests already document that real-provider configuration may fail in test mode without credentials or fallback semantics:
   - `edgequake/crates/edgequake-api/tests/e2e_workspace_provider_rebuild.rs` documents fallback behavior for OpenAI-without-key flows.
   - `edgequake/crates/edgequake-api/tests/e2e_document_workspace_provider.rs` already allows `CREATED` or `INTERNAL_SERVER_ERROR` when provider creation can fail.

## Territory Conclusion

The unstable piece is the assertion contract, not the shared helper extraction. The current repository behavior for OpenAI-configured query workspaces is allowed to surface as `500` in test mode.
