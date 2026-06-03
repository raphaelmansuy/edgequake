# P0 — API production path uses SOTA only

**Status:** ✅ Proven  
**Date:** 2026-06-03

## Claim

REST query execution (`query_execution.rs`) and Ollama emulation do not invoke legacy `QueryEngine`.

## Evidence

```bash
cargo test -p edgequake-api --test spec017_query_production_path_contract
```

Source contract tests read `query_execution.rs`, `ollama/chat.rs`, `ollama/generate.rs` and assert:
- contains `sota_engine`
- does not contain `query_engine`

Note: `AppState` still **constructs** `query_engine` for backward compatibility — production handlers do not call it.
