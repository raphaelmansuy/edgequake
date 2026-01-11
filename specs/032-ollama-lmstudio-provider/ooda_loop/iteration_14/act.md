# OODA Loop Iteration 14 - Act Phase

**Date:** 2026-01-11  
**Focus:** Tenant-Level LLM and Embedding Configuration  
**Status:** ✅ COMPLETE

## Changes Made

### 1. Backend Domain Types

**File:** [edgequake-core/src/types/multitenancy.rs](../../edgequake/crates/edgequake-core/src/types/multitenancy.rs)

Added to `Tenant` struct:

```rust
// === Default LLM Configuration (SPEC-032) ===
pub default_llm_model: String,
pub default_llm_provider: String,

// === Default Embedding Configuration (SPEC-032) ===
pub default_embedding_model: String,
pub default_embedding_provider: String,
pub default_embedding_dimension: usize,
```

Added builder methods:

```rust
pub fn with_llm_config(mut self, model: impl Into<String>, provider: impl Into<String>) -> Self
pub fn with_embedding_config(mut self, model: impl Into<String>, provider: impl Into<String>, dimension: usize) -> Self
```

Added to `CreateWorkspaceRequest`:

```rust
pub fn with_llm_config(mut self, model: impl Into<String>, provider: impl Into<String>) -> Self
pub fn with_embedding_config(mut self, model: impl Into<String>, provider: impl Into<String>, dimension: usize) -> Self
```

### 2. Backend API Types

**File:** [edgequake-api/src/handlers/workspaces_types.rs](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs)

Updated `CreateTenantRequest`:

```rust
pub default_llm_model: Option<String>,
pub default_llm_provider: Option<String>,
pub default_embedding_model: Option<String>,
pub default_embedding_provider: Option<String>,
pub default_embedding_dimension: Option<usize>,
```

Updated `TenantResponse`:

```rust
pub default_llm_model: String,
pub default_llm_provider: String,
pub default_llm_full_id: String,
pub default_embedding_model: String,
pub default_embedding_provider: String,
pub default_embedding_dimension: usize,
pub default_embedding_full_id: String,
```

### 3. Backend Handlers

**File:** [edgequake-api/src/handlers/workspaces.rs](../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)

- `create_tenant`: Applies optional model config, auto-detects provider, creates default workspace with inherited config
- `list_tenants`: Returns full model config in response
- `get_tenant`: Returns full model config in response
- `update_tenant`: Returns full model config in response

### 4. Storage Adapter

**File:** [edgequake-core/src/workspace_service_impl.rs](../../edgequake/crates/edgequake-core/src/workspace_service_impl.rs)

Updated `TenantRow::into_tenant()` to extract model config from metadata JSONB with fallback to server defaults.

### 5. Workspace Inheritance

**File:** [edgequake-api/src/handlers/workspaces.rs](../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)

Updated `create_workspace` handler:

```rust
// SPEC-032: Fetch parent tenant to inherit default model configuration if not provided
let tenant = state.workspace_service.get_tenant(tenant_id).await...?;

// SPEC-032: Use tenant defaults if workspace-level config not provided
let llm_model = request.llm_model.clone().or_else(|| Some(tenant.default_llm_model.clone()));
let llm_provider = request.llm_provider.clone().or_else(|| Some(tenant.default_llm_provider.clone()));
let embedding_model = request.embedding_model.clone().or_else(|| Some(tenant.default_embedding_model.clone()));
// ... etc
```

### 6. WebUI Changes

**File:** [edgequake_webui/src/types/index.ts](../../edgequake_webui/src/types/index.ts)

Updated `Tenant` interface with default model fields.

**File:** [edgequake_webui/src/lib/api/edgequake.ts](../../edgequake_webui/src/lib/api/edgequake.ts)

Added `CreateTenantRequest` interface and updated `createTenant` function.

**File:** [edgequake_webui/src/components/shared/tenant-workspace-selector.tsx](../../edgequake_webui/src/components/shared/tenant-workspace-selector.tsx)

- Added `tenantDefaultLLM` and `tenantDefaultEmbedding` state
- Updated tenant creation dialog with `LLMModelSelector` and `EmbeddingModelSelector`
- Updated mutation to pass model config

## Test Results

```
cargo test --workspace
test result: ok. 20 passed; 0 failed; 20+ ignored
```

```
cargo clippy
warning: `edgequake-llm` generated 2 warnings (pre-existing)
warning: `edgequake-core` generated 1 warning (pre-existing)
Finished `dev` profile
```

## Files Changed

| File | Lines Changed | Type |
|------|--------------|------|
| `edgequake-core/src/types/multitenancy.rs` | +120 | Domain |
| `edgequake-core/src/workspace_service_impl.rs` | +35 | Storage |
| `edgequake-api/src/handlers/workspaces_types.rs` | +60 | API |
| `edgequake-api/src/handlers/workspaces.rs` | +80 | API |
| `edgequake_webui/src/types/index.ts` | +40 | UI |
| `edgequake_webui/src/lib/api/edgequake.ts` | +35 | UI |
| `edgequake_webui/src/components/shared/tenant-workspace-selector.tsx` | +60 | UI |
| **Total** | **~430** | |

## Acceptance Criteria Status

- [x] Tenant struct has default model configuration fields
- [x] Tenant creation API accepts optional model config
- [x] Tenant responses include model configuration
- [x] WebUI shows model selectors in tenant creation dialog
- [x] New workspaces inherit tenant defaults if not specified
- [x] All tests pass
- [x] Clippy clean (only pre-existing warnings)
