# E2E Proof — Conversation UI + PostgreSQL User Bootstrap (P3)

**Date:** 2026-06-03  
**Spec:** SPEC-017 P3-18 / P3-21 / P3-23

## First-principle objective

1. **Storage:** `POST /api/v1/conversations` works for anonymous UI user IDs on PostgreSQL (FK-safe).
2. **UI:** Query history lists a conversation created via the storage API.

## Code (DRY)

`edgequake-api/src/handlers/postgres_user_bootstrap.rs` — used by conversation CRUD and chat handlers.

## Run

```bash
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_playwright_proof.sh
```

**Spec:** `edgequake_webui/e2e/spec017-storage-conversations.spec.ts`  
**Artifacts:** `screenshots/07-conversations-query-panel.png`, `08-conversations-history-header.png`

**Rust:** `cargo test -p edgequake-api --test spec017_conversation_http_contract`
