# OODA Iteration 91: Orient

## Analysis

### Code Path Investigation

**File**: `edgequake-api/src/handlers/chat.rs`

**Current Flow (BEFORE FIX)**:
```rust
// Validate workspace_id exists in database
let workspace_id = if let Some(ws_id) = workspace_id {
    match state.workspace_service.get_workspace(ws_id).await {
        Ok(Some(_)) => Some(ws_id),  // ⚠️ Workspace discarded!
        Ok(None) => None,
        Err(e) => None,
    }
} else {
    None
};

// Provider selection
let (llm_override, used_provider, used_model) = if let Some(ref provider_id) = request.provider {
    // ... request provider logic
} else {
    (None, None, None)  // ⚠️ Workspace provider never checked!
};
```

### Priority Order Should Be

1. **Request-specified** provider/model (explicit user selection)
2. **Workspace-configured** provider/model (from workspace settings)
3. **Server default** (sota_engine's default provider)

### Affected Endpoints

- `POST /api/v1/chat/completions` (non-streaming)
- `POST /api/v1/chat/completions/stream` (streaming)

Both have identical issue - need to fix both.
