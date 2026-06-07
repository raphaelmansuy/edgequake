# P0 — Query/chat shared execution (API-DRY-002, API-SOLID-D-001)

**Status:** ✅ Proven  
**Date:** 2026-06-04

## Claim

`/query`, `/chat/completions`, and streaming handlers share `execute_sota_query_with_auth_fallback` + `resolve_workspace_query_resources` and route LLM resolution through `WorkspaceProviderResolver`.

## Evidence

```bash
cargo test -p edgequake-api --test spec017_api_contract spec017_query_and_chat_share_execution_service
cargo test -p edgequake-api --test spec017_query_production_path_contract
cargo test -p edgequake-api --test e2e_query_routing_parity
```

| Test | Proves |
|------|--------|
| Source contract | Both handlers call shared service + resolver |
| `spec017_query_production_path_contract` | SOTA only — no legacy `query_engine` in production path |
| `test_query_and_chat_share_workspace_routing` | Same mode (`naive`) for identical workspace |
| `test_query_invalid_workspace_fails_closed` | 404 on bogus workspace |
| `test_chat_invalid_workspace_fails_closed` | Chat parity with query fail-closed |
