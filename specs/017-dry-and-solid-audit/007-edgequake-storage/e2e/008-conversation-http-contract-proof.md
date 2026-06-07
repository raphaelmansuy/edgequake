# P3 — Conversation HTTP contract (memory API)

**Date:** 2026-06-03  
**Status:** ✅ Proven

## Change

`edgequake-api/tests/spec017_conversation_http_contract.rs`:

1. `spec017_conversation_http_create_and_list_contract` — POST `/api/v1/conversations` → GET list contains created id
2. `spec017_conversation_http_requires_tenant_headers` — missing headers → 400

Uses `AppState::test_state()` (memory `ConversationStorage` path).

## Proof

```bash
cargo test -p edgequake-api --test spec017_conversation_http_contract
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

## Gap

No Playwright UI proof for conversations page (not required for storage trait HTTP wiring). Dashboard stats UI remains in `e2e/screenshots/`.
