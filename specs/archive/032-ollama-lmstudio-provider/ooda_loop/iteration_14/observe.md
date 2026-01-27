# OODA Loop Iteration 14 - Observe Phase

**Date:** 2026-01-11  
**Focus:** Tenant-Level LLM and Embedding Configuration  
**Status:** ✅ COMPLETE

## Observations

### Current State Analysis

From previous iterations (10-11), workspace-level LLM and embedding configuration was implemented:

- `Workspace` struct has `llm_model`, `llm_provider`, `embedding_model`, `embedding_provider`, `embedding_dimension`
- `CreateWorkspaceApiRequest` accepts optional model config
- `LLMModelSelector` and `EmbeddingModelSelector` components exist in WebUI
- Document ingestion uses workspace-specific LLM configuration

### Gap Identified

**Tenant struct lacks LLM/embedding configuration:**

```rust
// Current state (before iteration 14)
pub struct Tenant {
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub plan: TenantPlan,
    pub max_workspaces: usize,
    pub max_users: usize,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
    // MISSING: LLM and embedding configuration
}
```

### User Requirements (from SPEC-032)

1. **Tenant-level defaults:** "Ensure that when I create a new Tenant I can choose the default llm and embedding provider and model for that tenant/workspace"

2. **Workspace inheritance:** "Ensure that when I create a new workspace I can choose the default llm and embedding provider and model for that workspace"

3. **Model ID format:** `"provider/model_name"` (e.g., "ollama/gemma3:12b")

### Files Requiring Modification

| File                                                                  | Purpose          | Changes Needed                             |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| `edgequake-core/src/types/multitenancy.rs`                            | Domain types     | Add fields to Tenant struct                |
| `edgequake-api/src/handlers/workspaces_types.rs`                      | API DTOs         | Update CreateTenantRequest, TenantResponse |
| `edgequake-api/src/handlers/workspaces.rs`                            | API handlers     | Update all tenant handlers                 |
| `edgequake-core/src/workspace_service_impl.rs`                        | Postgres adapter | Update TenantRow conversion                |
| `edgequake_webui/src/lib/api/edgequake.ts`                            | API client       | Update createTenant signature              |
| `edgequake_webui/src/types/index.ts`                                  | TypeScript types | Update Tenant interface                    |
| `edgequake_webui/src/components/shared/tenant-workspace-selector.tsx` | UI component     | Add model selectors                        |

## Metrics

- **Files to modify:** 7
- **Estimated LOC:** ~400
- **Risk level:** Medium (domain type changes propagate widely)
