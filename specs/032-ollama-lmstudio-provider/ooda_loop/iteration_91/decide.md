# OODA Iteration 91: Decide

## Solution Design

### Changes Required

1. **Store workspace object when validating**:

   ```rust
   let (workspace_id, workspace) = if let Some(ws_id) = workspace_id {
       match state.workspace_service.get_workspace(ws_id).await {
           Ok(Some(ws)) => (Some(ws_id), Some(ws)),  // Keep workspace!
           Ok(None) => (None, None),
           Err(e) => (None, None),
       }
   } else {
       (None, None)
   };
   ```

2. **Add workspace fallback in provider selection**:

   ```rust
   let (llm_override, used_provider, used_model) = if let Some(ref provider_id) = request.provider {
       // ... request provider logic
   } else {
       // No request.provider - use workspace provider if available
       if let Some(ref ws) = workspace {
           let provider_name = ws.llm_provider.clone();
           let model_name = ws.llm_model.clone();
           match ProviderFactory::create_llm_provider(&provider_name, &model_name) {
               Ok(llm) => (Some(llm), Some(provider_name), Some(model_name)),
               Err(e) => {
                   warn!("Workspace LLM provider failed, using server default");
                   (None, None, None)
               }
           }
       } else {
           (None, None, None)
       }
   };
   ```

3. **Clone workspace for streaming async task**:
   ```rust
   let workspace_clone = workspace.clone();
   ```

### No Breaking Changes

- Request-level provider override still takes priority
- Server default still works when no workspace
- Graceful fallback if workspace provider fails
