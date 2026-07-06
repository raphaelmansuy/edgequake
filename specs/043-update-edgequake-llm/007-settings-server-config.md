# SPEC-043 — Settings Server Config Save

## Problem

Settings page shows read-only env snippets (`ProviderStatusCard`) with Copy button — no save path for operators using the WebUI.

## Solution

Persist server-wide LLM defaults and application attribution in existing `server_config` table (migration 030).

---

## Keys

| key | JSON shape |
| --- | ---------- |
| `llm_defaults` | `{ "llm_provider", "llm_model", "embedding_provider", "embedding_model", "vision_provider", "vision_model" }` |
| `app_attribution` | `{ "app_id", "app_name", "app_url" }` |

---

## API

### `GET /api/v1/settings/llm-defaults`

Resolution order documented in response:

```json
{
  "effective": {
    "llm_provider": "ollama",
    "llm_model": "gemma4:latest",
    "embedding_provider": "ollama",
    "embedding_model": "embeddinggemma",
    "vision_provider": "ollama",
    "vision_model": "gemma4:latest"
  },
  "sources": {
    "llm_provider": "server_config",
    "llm_model": "env:OLLAMA_MODEL"
  },
  "editable": true,
  "requires_restart": false
}
```

### `PATCH /api/v1/settings/llm-defaults` (admin)

Validates provider/model against `models.toml` + catalog. Writes `server_config`. Invalidates in-memory cache.

### `GET/PATCH /api/v1/settings/app-attribution` (admin)

Read/write `app_attribution` key; merged with env on startup into `AppState.application_context`.

---

## Resolution ladder (updated)

```
1. Request override (query/chat)
2. Workspace DB
3. server_config table     ← NEW
4. Environment variables
5. models.toml defaults
6. Compiled fallback (ollama)
```

**Code:** extend `Workspace::server_runtime_llm_config()` to query `server_config` first.

---

## UI: `ServerLlmConfigCard`

- Loads GET endpoints
- Uses `ModelPickerPanel` for each model field
- Save → PATCH → toast → invalidate React Query caches

---

## Security

- PATCH requires `ApiRequireAdmin`
- Secrets (API keys) **never** stored in server_config — only provider/model selection + app attribution metadata
