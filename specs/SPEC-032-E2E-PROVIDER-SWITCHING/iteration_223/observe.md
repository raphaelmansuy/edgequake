# OODA 223: OBSERVE - Chat Handler with Workspace Providers

## Objective

Test that chat completions use the workspace's configured LLM provider. This is a critical user scenario where:

1. User sets workspace LLM to Ollama
2. User sends chat message
3. Chat response uses Ollama (not global default)
4. User switches to OpenAI
5. Next chat uses OpenAI

## Current Flow (from handlers/chat.rs)

```rust
// Line 254-258: Get workspace_id from tenant context
let workspace_id = tenant_ctx.workspace_id
    .as_ref()
    .and_then(|s| Uuid::parse_str(s).ok())
    .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

// Line 289-300: Validate workspace and get config
let (workspace_id, workspace) = if let Some(ws_id) = workspace_id {
    match state.workspace_service.get_workspace(ws_id).await {
        Ok(Some(ws)) => (Some(ws_id), Some(ws)),
        Ok(None) => {
            warn!(workspace_id = %ws_id, "Workspace not found");
            (None, None)
        }
        ...
    }
} else {
    (None, None)
};

// Line 371-411: Provider selection priority
// 1. Request-specified provider/model
// 2. Workspace-configured provider/model
// 3. Server default
```

## Key Integration Points

1. **TenantContext.workspace_id** - From X-Workspace-ID header
2. **workspace_service.get_workspace()** - Retrieves workspace config
3. **ProviderFactory.create_llm_provider()** - Creates provider from workspace config

## Test Scenarios Needed

### Scenario 1: Chat with Ollama workspace

- Create workspace with Ollama LLM config
- Send chat request with X-Workspace-ID
- Verify response (or connection error to Ollama)

### Scenario 2: Chat with OpenAI workspace (no API key)

- Create workspace with OpenAI config
- Without API key: should fail or fallback

### Scenario 3: Chat with mock provider

- Create workspace with mock provider
- Send chat request
- Verify mock response

### Scenario 4: Chat provider switch

- Create workspace with provider A
- Send chat
- Switch to provider B
- Send another chat

### Scenario 5: Request override workspace config

- Create workspace with Ollama
- Send chat with explicit "openai" in request
- Request should override workspace config

## Files to Create

`edgequake/crates/edgequake-api/tests/e2e_chat_workspace_provider.rs`
