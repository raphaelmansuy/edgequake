# OODA-10 Observe: Clean Tenant Setup

## Current Test Infrastructure

### In-Memory Test Pattern

- `AppState::test_state()` creates fresh in-memory storage per call
- Each test creates its own `Server` and `Router`
- Data isolation achieved by construction (separate HashMap instances)

### Tenant/Workspace API

- `POST /api/v1/tenants` → creates tenant + auto-creates default workspace
- `GET /api/v1/tenants/{tenant_id}/workspaces` → lists workspaces
- Response: `TenantResponse.id` (UUID), `WorkspaceListResponse.items[].id` (UUID)
- `X-Tenant-ID` and `X-Workspace-ID` headers set context for operations

### Key Discovery: Workspace Pipeline Issue

When `X-Workspace-ID` is sent, document handler tries to create workspace-specific
LLM/embedding providers. Workspace inherits default config (e.g., `embeddinggemma`).
Without real Ollama → embedding step fails. Solution: Use global mock pipeline.

## Files Analyzed

- `documents.rs:497` - upload_document handler
- `workspaces.rs:162` - create_tenant handler
- `workspaces_types.rs:36` - CreateTenantRequest
- `state.rs:996` - create_workspace_pipeline
- `middleware.rs:366` - TenantContext extractor
