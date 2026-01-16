# OODA-229: Fix Config Error Messages

## Task Summary

Fix the error message "Cannot stream query for workspace: X. Embedding provider configuration error: Internal(...)" to provide clear, actionable guidance when OPENAI_API_KEY is missing.

## Problem

When a workspace is configured to use OpenAI embeddings but OPENAI_API_KEY is not set:

1. Error message was cryptic and confusing
2. System would either silently fallback to default provider (causing dimension mismatch)
3. Or return a generic internal error

## Root Cause

The `create_embedding_provider()` function in `factory.rs` returns a generic error when the API key is missing. The query/chat handlers were:

1. Catching this error and falling back to default (causing wrong embeddings)
2. Or wrapping it in Internal error (hiding the real issue)

## Solution Implemented

### 1. Added `ConfigError` variant to `ApiError` (error.rs)

```rust
/// Configuration error (e.g., missing API keys for workspace provider).
#[error("Configuration error: {0}")]
ConfigError(String),
```

- Returns HTTP 422 Unprocessable Entity
- Returns code `CONFIG_ERROR`

### 2. Improved error detection in query.rs

- Detect `OPENAI_API_KEY` in error string
- Return `ConfigError` with actionable message:
  ```
  Workspace 'X' is configured to use OpenAI embeddings (model: Y),
  but OPENAI_API_KEY is not set. Either:
  1. Set OPENAI_API_KEY environment variable and restart the server, or
  2. Update workspace settings to use a different provider (ollama, lmstudio)
  ```

### 3. Fixed error handling to return immediately instead of fallback

- query.rs: ConfigError is returned directly, other errors fallback
- chat.rs: Streaming handler passes through the error message

## Files Changed

- `edgequake/crates/edgequake-api/src/error.rs` - Added ConfigError variant
- `edgequake/crates/edgequake-api/src/handlers/query.rs` - Improved error handling
- `edgequake/crates/edgequake-api/src/handlers/chat.rs` - Simplified error message passing

## Testing

1. Started backend without OPENAI_API_KEY
2. Queried workspace configured for OpenAI embeddings
3. Received clear error message with HTTP 422 status

## Commits

- `def5e29` - fix(OODA-229): Clear error messages for missing API keys

## Related Issues

- OODA-228: Vector dimension mismatch detection
