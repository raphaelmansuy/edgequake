# Multi-Tenancy Architecture

**Version:** 1.0  
**Target Release:** EdgeQuake v2.0.0  
**Priority:** MEDIUM (Optional Feature)  
**Status:** Planning

---

## Overview

Implement optional multi-tenancy support to allow multiple isolated tenants (organizations) to share the same EdgeQuake instance while maintaining data isolation.

### Goals

1. **Data Isolation:** Complete separation of tenant data
2. **Tenant Management:** Create/manage tenants and workspaces
3. **Membership:** User-tenant-role assignments
4. **Backwards Compatible:** Optional feature flag (default: single-tenant)
5. **Performance:** Minimal overhead for single-tenant mode

---

## Architecture

### Tenant Hierarchy

```
Tenant (Organization)
  └── Workspaces (Knowledge Bases)
       └── Documents
            ├── Chunks
            ├── Entities
            └── Relationships
```

### Feature Flag

```toml
[features]
multi-tenant = ["auth"]
default = []
```

**Single-Tenant Mode (default):**
```rust
// Uses default tenant_id = "default"
// No tenant selection required
```

**Multi-Tenant Mode:**
```rust
// Requires tenant_id in all operations
// Tenant isolation enforced
```

---

## Data Models

### Tenant Schema

```sql
CREATE TABLE tenants (
    tenant_id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_tenants_slug ON tenants(slug);
CREATE INDEX idx_tenants_active ON tenants(is_active);
```

### Workspace (Knowledge Base) Schema

```sql
CREATE TABLE workspaces (
    workspace_id VARCHAR(100) PRIMARY KEY,
    tenant_id VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB,
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    UNIQUE(tenant_id, slug)
);

CREATE INDEX idx_workspaces_tenant ON workspaces(tenant_id);
```

### Membership Schema

```sql
CREATE TABLE memberships (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    tenant_id VARCHAR(100) NOT NULL,
    role VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    UNIQUE(user_id, tenant_id),
    CONSTRAINT valid_role CHECK (role IN ('owner', 'admin', 'member', 'readonly'))
);

CREATE INDEX idx_memberships_user ON memberships(user_id);
CREATE INDEX idx_memberships_tenant ON memberships(tenant_id);
```

---

## API Endpoints

### Tenant Management

```http
POST /api/v1/tenants
GET  /api/v1/tenants
GET  /api/v1/tenants/me
POST /api/v1/tenants/select
```

### Workspace Management

```http
POST   /api/v1/workspaces
GET    /api/v1/workspaces
GET    /api/v1/workspaces/{id}
PUT    /api/v1/workspaces/{id}
DELETE /api/v1/workspaces/{id}
GET    /api/v1/workspaces/{id}/stats
```

### Membership Management

```http
POST   /api/v1/memberships
GET    /api/v1/memberships/{tenant_id}
PUT    /api/v1/memberships/{tenant_id}/users/{user_id}
DELETE /api/v1/memberships/{tenant_id}/users/{user_id}
GET    /api/v1/users/me/tenants
```

---

## Tenant Context Injection

### Middleware

```rust
pub struct TenantContext {
    pub tenant_id: String,
    pub workspace_id: Option<String>,
    pub user_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        #[cfg(not(feature = "multi-tenant"))]
        {
            // Single-tenant mode: use default
            return Ok(TenantContext {
                tenant_id: "default".to_string(),
                workspace_id: Some("default".to_string()),
                user_id: "anonymous".to_string(),
            });
        }
        
        #[cfg(feature = "multi-tenant")]
        {
            // Get user from auth
            let auth = parts.extensions.get::<AuthUser>()
                .ok_or(ApiError::Unauthorized("Authentication required".to_string()))?;
            
            // Get tenant_id from header or session
            let tenant_id = parts
                .headers
                .get("X-Tenant-ID")
                .and_then(|h| h.to_str().ok())
                .ok_or(ApiError::BadRequest("Missing X-Tenant-ID header".to_string()))?
                .to_string();
            
            // Get workspace_id from header (optional)
            let workspace_id = parts
                .headers
                .get("X-Workspace-ID")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            
            // Verify user has access to tenant
            let membership_service = parts.extensions.get::<Arc<MembershipService>>()
                .ok_or(ApiError::Internal("Membership service not available".to_string()))?;
            
            membership_service
                .verify_access(&auth.0.sub, &tenant_id)
                .await?;
            
            Ok(TenantContext {
                tenant_id,
                workspace_id,
                user_id: auth.0.sub.clone(),
            })
        }
    }
}

// Usage in handlers
pub async fn create_document(
    State(state): State<AppState>,
    tenant: TenantContext,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<Json<UploadDocumentResponse>> {
    // tenant_id and workspace_id are automatically injected
    let document_id = state.pipeline
        .process_with_tenant(
            &tenant.tenant_id,
            &tenant.workspace_id.unwrap_or_default(),
            &request.content,
        )
        .await?;
    
    // ...
}
```

---

## Storage Isolation

### Document ID Format

```
Single-tenant: doc-{uuid}
Multi-tenant:  {tenant_id}:{workspace_id}:doc-{uuid}
```

### Graph Isolation (AGE)

```cypher
// Multi-tenant node creation
CREATE (e:Entity {
    tenant_id: $tenant_id,
    workspace_id: $workspace_id,
    id: $id,
    entity_name: $entity_name,
    ...
})

// Multi-tenant query
MATCH (e:Entity)
WHERE e.tenant_id = $tenant_id 
  AND e.workspace_id = $workspace_id
RETURN e
```

### Vector Storage Isolation

```rust
// Namespace by tenant and workspace
let namespace = format!("{}:{}", tenant_id, workspace_id);
vector_storage.upsert_with_namespace(&namespace, embeddings).await?;
```

---

## Configuration

```bash
# Multi-tenancy
EDGEQUAKE_MULTI_TENANT=false  # Feature flag
DEFAULT_TENANT_ID=default
DEFAULT_WORKSPACE_ID=default

# Tenant Limits
MAX_TENANTS=1000
MAX_WORKSPACES_PER_TENANT=100
MAX_MEMBERS_PER_TENANT=1000
```

---

**Status:** ✅ Specification Complete  
**Dependencies:** 05-authentication.md  
**Next:** Implementation with feature flag
