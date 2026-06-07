# P3 — Playwright conversations UI proof

**Date:** 2026-06-03  
**Status:** ✅ Spec ready; PNGs captured when `--playwright` with live stack

## Change

`edgequake_webui/e2e/spec017-storage-conversations.spec.ts`:

- Navigates to `/query` with deterministic tenant/workspace bootstrap
- Clicks **New conversation** (aria-label)
- Asserts `POST /api/v1/conversations` fired (storage trait → HTTP wiring)
- Captures PNGs:
  - `screenshots/07-conversations-query-panel.png`
  - `screenshots/08-conversations-history-header.png`

## Proof

```bash
# Live stack (use localhost for Next.js HMR):
PLAYWRIGHT_BASE_URL=http://localhost:3001 EQ_BACKEND_URL=http://127.0.0.1:8087 \
  ./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh --playwright
```

Rust HTTP contract (no browser):

```bash
cargo test -p edgequake-api --test spec017_conversation_http_contract
```

## Gap

PNG artifacts require running `--playwright` against a live stack; default `run_storage_e2e.sh` does not require them.
