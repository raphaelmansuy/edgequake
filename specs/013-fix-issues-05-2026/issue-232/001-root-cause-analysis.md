# Issue #232 — Root Cause Analysis

**GitHub:** [#232](https://github.com/raphaelmansuy/edgequake/issues/232)  
**Cross-ref:** [002-fix-specification.md](002-fix-specification.md)

## Symptom (fact)

`POST /api/v1/api-keys` creates keys; `GET /api/v1/api-keys` returns empty list.

## 5 WHY

| # | Why | Evidence |
|---|-----|----------|
| 1 | Why empty response? | Handler returns `keys: vec![]`, `total: 0` always |
| 2 | Why always empty? | Explicit `TODO: Implement listing with prefix scan` |
| 3 | Why not implemented? | Create/revoke paths work; list was deferred |
| 4 | Why POST still works? | `create_api_key` writes to KV with prefix `auth:api_key:` |
| 5 | Why KV can list now? | `keys_with_prefix` exists (used by `list_users` since SPEC-012) |

## Proof (code)

```135:151:edgequake/crates/edgequake-api/src/handlers/auth/api_keys.rs
    // TODO: Implement listing with prefix scan when KV storage supports it
    Ok(Json(ListApiKeysResponse { keys: vec![], total: 0, ... }))
```

## Fix summary

Implement `list_api_keys` using `keys_with_prefix(API_KEY_PREFIX)` + filter by `user_id` (mirror `list_users`).
