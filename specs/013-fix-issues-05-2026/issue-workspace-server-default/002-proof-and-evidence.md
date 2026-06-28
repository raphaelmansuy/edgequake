# Workspace server-default reset — proof

## Automated

```bash
# Unit
cargo test -p edgequake-core workspace_model_update::tests::clear_llm_resets_to_env_defaults

# API E2E (PostgreSQL)
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cargo test -p edgequake-api --features postgres \
  spec013_workspace_reset_llm_embedding_to_server_default -- --nocapture
```

**E2E scenario**

1. Pin workspace to `mock` + `stale-stuck-model` (simulates leftover SPEC-013 test workspace).
2. `PUT` with `llm_provider: ""`, `llm_model: ""`, empty embedding fields.
3. Assert `llm_provider == ollama` (env set in test) and not `mock`.
4. `GET` proves persistence after reload.

## Manual (your Cancel WS)

1. Workspace → **Edit Configuration**.
2. Set LLM and Embedding to **Server default** → **Save**.
3. View mode should show **ollama** / **gemma4:latest** (matching `/health`), not **mock**.
4. **Reprocess** documents (or upload again) — entity count should be &gt; 0 if Ollama is running.

## Related

- Partial failure when stuck on mock: see tenant/workspace created under `SPEC013 cancel` E2E data.
