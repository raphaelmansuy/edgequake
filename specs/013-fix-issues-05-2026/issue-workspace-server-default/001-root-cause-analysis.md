# Workspace “Server default” reset — root cause

## Symptom

User selects **Server default** for LLM (and embedding) on the workspace page, clicks **Save**, but the view still shows **`mock/gemma4:latest`**. Documents keep **Partial Failure** (0 entities, 0 LLM tokens).

## Root cause (two bugs)

### 1. Frontend did not send a clear signal

`handleSave` only included `llm_model` / `llm_provider` when `selectedLLM` was set:

```typescript
if (selectedLLM) {
  data.llm_model = selectedLLM.model;
  data.llm_provider = selectedLLM.provider;
}
```

Choosing **Server default** sets `selectedLLM` to `undefined`, so the PUT body omitted LLM fields entirely → backend left stale metadata unchanged.

Vision/PDF parser already used `selectedVisionLLM?.provider ?? ''` to clear overrides; LLM/embedding did not.

### 2. Backend had no clear path for LLM/embedding

`update_workspace` only applied `Some(value)` updates. Unlike `vision_llm_*` (empty string removes override), `llm_provider` / `embedding_provider` could never be reset to server/env defaults once written (e.g. `mock` from SPEC-013 E2E tenant/workspace creation).

## Fix

| Layer | Change |
|-------|--------|
| WebUI | Always send `llm_*` / `embedding_*`; use `''` (and `embedding_dimension: 0`) for server default |
| Core | `workspace_model_update.rs` — empty/`none` resets to `Workspace::default_llm_config()` / `default_embedding_config()` and removes metadata keys |

## Proof

- Unit: `edgequake-core/src/workspace_model_update.rs` (`clear_llm_resets_to_env_defaults`)
- API E2E: `spec013_workspace_reset_llm_embedding_to_server_default` in `e2e_spec013_github_issues.rs`
