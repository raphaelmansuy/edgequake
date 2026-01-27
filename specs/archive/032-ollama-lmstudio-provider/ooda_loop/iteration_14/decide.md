# OODA Loop Iteration 14 - Decide Phase

**Date:** 2026-01-11  
**Focus:** Implementation Plan for Tenant Model Configuration  
**Status:** ✅ COMPLETE

## Implementation Plan

### Phase 1: Backend Domain Types

1. **Extend `Tenant` struct** ([multitenancy.rs#L15](../../edgequake/crates/edgequake-core/src/types/multitenancy.rs#L15))

   - Add `default_llm_model: String`
   - Add `default_llm_provider: String`
   - Add `default_embedding_model: String`
   - Add `default_embedding_provider: String`
   - Add `default_embedding_dimension: usize`

2. **Update `Tenant::new()`** to use `Workspace::default_llm_config()` and `Workspace::default_embedding_config()`

3. **Add builder methods:**
   - `with_llm_config(model, provider) -> Self`
   - `with_embedding_config(model, provider, dimension) -> Self`

### Phase 2: Backend API Types

1. **Update `CreateTenantRequest`** ([workspaces_types.rs#L16](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs#L16))

   - Add optional `default_llm_model`, `default_llm_provider`
   - Add optional `default_embedding_model`, `default_embedding_provider`, `default_embedding_dimension`

2. **Update `TenantResponse`** ([workspaces_types.rs#L191](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs#L191))
   - Add all default model fields including `*_full_id`

### Phase 3: Backend Handlers

1. **Update `create_tenant`** handler to:

   - Apply optional model config from request
   - Auto-detect provider from model name if not specified
   - Pass inherited config to default workspace creation

2. **Update `list_tenants`, `get_tenant`, `update_tenant`** to include new fields in response

### Phase 4: Storage Adapter

1. **Update `TenantRow::into_tenant()`** ([workspace_service_impl.rs#L816](../../edgequake/crates/edgequake-core/src/workspace_service_impl.rs#L816))
   - Extract model config from metadata JSONB
   - Use server defaults as fallback

### Phase 5: Workspace Inheritance

1. **Update `create_workspace`** handler:
   - Fetch parent tenant
   - If workspace model config not provided, use tenant defaults

### Phase 6: WebUI

1. **Update `Tenant` interface** in `types/index.ts`
2. **Update `CreateTenantRequest` interface** in `lib/api/edgequake.ts`
3. **Update `createTenant` function** to accept model config
4. **Add model selectors** to tenant creation dialog

## Risk Mitigation

| Risk                      | Mitigation                                          |
| ------------------------- | --------------------------------------------------- |
| Breaking existing tenants | Fallback to server defaults in TenantRow conversion |
| Test failures             | Update all test fixtures with new required fields   |
| Type errors in WebUI      | Use optional types for new fields                   |

## Acceptance Criteria

- [ ] Tenant struct has default model configuration fields
- [ ] Tenant creation API accepts optional model config
- [ ] Tenant responses include model configuration
- [ ] WebUI shows model selectors in tenant creation dialog
- [ ] New workspaces inherit tenant defaults if not specified
- [ ] All tests pass
- [ ] Clippy clean
