# Multi-Tenancy Implementation Guide for EdgeQuake

**Technology Stack**: Rust + Axum + PostgreSQL (AGE + pgvector + RLS)  
**Date**: 2024-12-21  
**Status**: Complete  
**Related**: [technology_choice.md](./technology_choice.md), [postgresql.md](./postgresql.md), [axum.md](./axum.md), [ADR-004](../plan/integration/ADR_INDEX.md#adr-004-implement-shared-database-multi-tenancy-with-postgresql-rls)

---

## Overview

This guide provides comprehensive implementation patterns for multi-tenancy in EdgeQuake. Multi-tenancy allows multiple isolated workspaces (tenants) to coexist within a single application instance while maintaining complete data isolation and security using PostgreSQL Row-Level Security (RLS).

**Key Requirements** (from docs_retro):

- Workspace isolation at database level using PostgreSQL RLS policies
- Workspace management API (create, list, delete)
- Tenant-specific storage and queries with automatic RLS enforcement
- No cross-tenant data leakage (database-enforced security)
- Efficient resource utilization

**Architecture Decision**: See [ADR-004](../plan/integration/ADR_INDEX.md#adr-004-implement-shared-database-multi-tenancy-with-postgresql-rls) for rationale on PostgreSQL RLS approach.

---

## Architecture Patterns

### Pattern 1: Shared Database + PostgreSQL RLS (Recommended)

**Approach**: Single PostgreSQL database with `workspace_id` column in all tables, enforced by Row-Level Security policies

**Advantages**:

- Resource efficient (one database for all tenants)
- Simple deployment (no per-tenant infrastructure)
- Easy backup/restore (single database dump)
- Cost-effective (shared infrastructure)
- **Database-enforced security**: RLS policies prevent data leaks at SQL level (immune to application bugs)
- **Automatic filtering**: No manual `WHERE workspace_id = $id` clauses needed

**Disadvantages**:

- Slight performance overhead (~5-10% due to RLS policy evaluation)
- Requires PostgreSQL 9.5+ for RLS support
- Limited isolation for strict compliance requirements (e.g., HIPAA may require separate databases)

**Best For**: SaaS applications, development/staging environments, multi-tenant RAG systems

---

### Pattern 2: Database Per Tenant

**Approach**: Separate database/namespace for each tenant

**Advantages**:

- Complete data isolation
- Simple queries (no tenant filtering)
- Easy tenant backup/restore
- Compliance-friendly

**Disadvantages**:

- Higher resource usage
- More complex deployment
- Schema migration complexity

**Best For**: Enterprise deployments, compliance-heavy industries

---

### Pattern 3: Hybrid Approach

**Approach**: Shared tables for common data, separate tables/namespaces for tenant data

**Advantages**:

- Balance between isolation and efficiency
- Flexible based on data sensitivity

**Disadvantages**:

- Most complex to implement
- Requires careful design

---

## Recommended Implementation: Shared Database + Tenant ID

Based on the LightRAG architecture patterns, we recommend **Pattern 1** for the initial EdgeQuake implementation:

### Data Model

```rust
use serde::{Deserialize, Serialize};
use std::fmt;

/// Workspace/Tenant identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Entity with workspace isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub workspace_id: WorkspaceId,  // Tenant isolation
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
}

/// Relation with workspace isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub workspace_id: WorkspaceId,  // Tenant isolation
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub weight: f32,
    pub metadata: serde_json::Value,
}

/// Document with workspace isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub workspace_id: WorkspaceId,  // Tenant isolation
    pub content: String,
    pub chunks: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Workspace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub settings: WorkspaceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub chunk_size: usize,
    pub max_tokens: usize,
    pub llm_model: String,
    pub embedding_model: String,
}
```

---

## Storage Layer Implementation

### PostgreSQL Schema with RLS

```rust
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Initialize PostgreSQL schema with Row-Level Security
pub async fn initialize_workspace_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "
        -- Enable required extensions
        CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";
        CREATE EXTENSION IF NOT EXISTS \"age\";
        CREATE EXTENSION IF NOT EXISTS \"vector\";
        
        -- Workspace table
        CREATE TABLE IF NOT EXISTS workspaces (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW(),
            settings JSONB DEFAULT '{}'::jsonb,
            CONSTRAINT workspace_name_unique UNIQUE (name)
        );
        
        -- Entity table with workspace isolation
        CREATE TABLE IF NOT EXISTS entities (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            description TEXT,
            embedding vector(1536),  -- pgvector for embeddings
            metadata JSONB DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );
        
        -- Indexes for entity table
        CREATE INDEX IF NOT EXISTS idx_entities_workspace ON entities(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(workspace_id, name);
        CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(workspace_id, entity_type);
        CREATE INDEX IF NOT EXISTS idx_entities_embedding ON entities 
            USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
        
        -- Enable RLS on entities
        ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
        
        -- RLS policy: Only see entities from your workspace
        CREATE POLICY tenant_isolation ON entities
            FOR ALL
            USING (workspace_id = current_setting('app.current_workspace_id')::UUID);
        
        -- Relation table with workspace isolation
        CREATE TABLE IF NOT EXISTS relations (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            source_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            target_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            relation_type TEXT NOT NULL,
            weight FLOAT DEFAULT 1.0,
            metadata JSONB DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ DEFAULT NOW()
        );
        
        -- Indexes for relation table (critical for graph traversal)
        CREATE INDEX IF NOT EXISTS idx_relations_workspace ON relations(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id, workspace_id);
        CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id, workspace_id);
        CREATE INDEX IF NOT EXISTS idx_relations_type ON relations(relation_type, workspace_id);
        
        -- Enable RLS on relations
        ALTER TABLE relations ENABLE ROW LEVEL SECURITY;
        
        -- RLS policy: Only see relations from your workspace
        CREATE POLICY tenant_isolation ON relations
            FOR ALL
            USING (workspace_id = current_setting('app.current_workspace_id')::UUID);
        
        -- Document table with workspace isolation
        CREATE TABLE IF NOT EXISTS documents (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            filename TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata JSONB DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );
        
        -- Indexes for document table
        CREATE INDEX IF NOT EXISTS idx_documents_workspace ON documents(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_documents_filename ON documents(workspace_id, filename);
        
        -- Enable RLS on documents
        ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
        
        -- RLS policy: Only see documents from your workspace
        CREATE POLICY tenant_isolation ON documents
            FOR ALL
            USING (workspace_id = current_setting('app.current_workspace_id')::UUID);
        "
    )
    .execute(pool)
    .await?;
    
    Ok(())
}
```

**Key Points**:
- **RLS policies automatically filter all queries** by `workspace_id` - no manual `WHERE` clauses needed
- **Foreign key constraints** ensure referential integrity (CASCADE deletes)
- **pgvector index** (`ivfflat`) for fast embedding similarity search
- **Indexes on workspace_id** critical for RLS performance (5-10% overhead vs 50%+ without indexes)
- **JSONB for metadata** allows flexible schema extension without migrations
        
        DEFINE FIELD id ON document TYPE string;
        DEFINE FIELD workspace_id ON document TYPE string;
        DEFINE FIELD content ON document TYPE string;
        DEFINE FIELD chunks ON document TYPE array<string>;
        DEFINE FIELD metadata ON document TYPE object;
        DEFINE FIELD created_at ON document TYPE datetime;
        
        DEFINE INDEX document_id_idx ON document FIELDS id, workspace_id UNIQUE;
        DEFINE INDEX document_workspace_idx ON document FIELDS workspace_id;
    ").await?;
    
    Ok(())
}
```

### Storage Trait with Workspace Context

```rust
use async_trait::async_trait;

/// Storage backend with workspace isolation
#[async_trait]
pub trait WorkspaceStorage: Send + Sync {
    /// Create a new workspace
    async fn create_workspace(&self, workspace: &Workspace) -> Result<()>;
    
    /// Get workspace by ID
    async fn get_workspace(&self, id: &WorkspaceId) -> Result<Option<Workspace>>;
    
    /// List all workspaces
    async fn list_workspaces(&self) -> Result<Vec<Workspace>>;
    
    /// Delete workspace and all its data
    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<()>;
    
    /// Insert entity (workspace-scoped)
    async fn insert_entity(
        &self,
        workspace_id: &WorkspaceId,
        entity: &Entity,
    ) -> Result<()>;
    
    /// Query entities (workspace-scoped)
    async fn query_entities(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Entity>>;
    
    /// Insert relation (workspace-scoped)
    async fn insert_relation(
        &self,
        workspace_id: &WorkspaceId,
        relation: &Relation,
    ) -> Result<()>;
    
    /// Graph traversal (workspace-scoped)
    async fn traverse_graph(
        &self,
        workspace_id: &WorkspaceId,
        start_entity_id: &str,
        max_depth: usize,
    ) -> Result<Vec<(Entity, Relation)>>;
}

/// PostgreSQL implementation with workspace isolation using RLS
pub struct PostgresWorkspaceStorage {
    pool: Arc<PgPool>,
}

#[async_trait]
impl WorkspaceStorage for PostgresWorkspaceStorage {
    async fn create_workspace(&self, workspace: &Workspace) -> Result<()> {
        sqlx::query(
            "INSERT INTO workspaces (id, name, description, settings) 
             VALUES ($1, $2, $3, $4)"
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(&workspace.description)
        .bind(&workspace.settings)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }
    
    async fn get_workspace(&self, id: &WorkspaceId) -> Result<Option<Workspace>> {
        let workspace = sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces WHERE id = $1"
        )
        .bind(id.as_uuid())
        .fetch_optional(self.pool.as_ref())
        .await?;
        Ok(workspace)
    }
    
    async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let workspaces = sqlx::query_as::<_, Workspace>(
            "SELECT * FROM workspaces ORDER BY created_at DESC"
        )
        .fetch_all(self.pool.as_ref())
        .await?;
        Ok(workspaces)
    }
    
    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        // Foreign key CASCADE will automatically delete entities, relations, documents
        sqlx::query("DELETE FROM workspaces WHERE id = $1")
            .bind(id.as_uuid())
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }
    
    async fn insert_entity(
        &self,
        workspace_id: &WorkspaceId,
        entity: &Entity,
    ) -> Result<()> {
        // Validate workspace_id matches
        if &entity.workspace_id != workspace_id {
            return Err(anyhow::anyhow!("Workspace ID mismatch"));
        }
        
        // Set session variable for RLS (if not already set by middleware)
        sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
            .bind(workspace_id.to_string())
            .execute(self.pool.as_ref())
            .await?;
        
        // RLS automatically enforces workspace_id filtering
        sqlx::query(
            "INSERT INTO entities (id, workspace_id, name, entity_type, description, embedding, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(&entity.id)
        .bind(&entity.workspace_id)
        .bind(&entity.name)
        .bind(&entity.entity_type)
        .bind(&entity.description)
        .bind(&entity.embedding)
        .bind(&entity.metadata)
        .execute(self.pool.as_ref())
        .await?;
        Ok(()))
    }
    
    async fn query_entities(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Entity>> {
        let embedding = generate_embedding(query).await?;
        
        // Set session variable for RLS
        sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
            .bind(workspace_id.to_string())
            .execute(self.pool.as_ref())
            .await?;
        
        // RLS automatically filters by workspace_id, no manual WHERE clause needed
        // pgvector cosine similarity search
        let entities = sqlx::query_as::<_, Entity>(
            "SELECT id, workspace_id, name, entity_type, description, embedding, metadata,
                    1 - (embedding <=> $1) as similarity
             FROM entities
             ORDER BY embedding <=> $1
             LIMIT $2"
        )
        .bind(&embedding)
        .bind(top_k as i64)
        .fetch_all(self.pool.as_ref())
        .await?;
        
        Ok(entities)
    }
    
    async fn traverse_graph(
        &self,
        workspace_id: &WorkspaceId,
        start_entity_id: &str,
        max_depth: usize,
    ) -> Result<Vec<(Entity, Relation)>> {
        // Set session variable for RLS
        sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
            .bind(workspace_id.to_string())
            .execute(self.pool.as_ref())
            .await?;
        
        // Use Apache AGE for graph traversal (Cypher query)
        // RLS automatically filters by workspace_id
        let results: Vec<(Entity, Relation)> = sqlx::query_as(
            "SELECT * FROM cypher('edgequake', $$
                MATCH (e:Entity)-[r:RELATES_TO*1..$max_depth]-(neighbor:Entity)
                WHERE e.id = $start_id
                RETURN e, r, neighbor
            $$) as (entity agtype, relation agtype, neighbor agtype)"
        )
        .bind(max_depth as i32)
        .bind(start_entity_id)
        .fetch_all(self.pool.as_ref())
        .await?;
        
        Ok(results)
    }
}
```

---

## API Layer Implementation

### Workspace Context Extraction Middleware

```rust
use axum::{
    extract::{FromRequestParts, Path, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

/// Workspace context extracted from request
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub workspace_id: WorkspaceId,
}

/// Extract workspace ID from path parameter
#[async_trait]
impl<S> FromRequestParts<S> for WorkspaceContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);
    
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract from path parameter /workspace/:id/...
        let workspace_id = parts
            .extensions
            .get::<WorkspaceId>()
            .ok_or((
                StatusCode::BAD_REQUEST,
                "Missing workspace ID".to_string(),
            ))?
            .clone();
        
        Ok(WorkspaceContext { workspace_id })
    }
}

/// Middleware to extract and validate workspace ID
pub async fn workspace_middleware<B>(
    Path(workspace_id): Path<String>,
    State(storage): State<Arc<dyn WorkspaceStorage>>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let workspace_id = WorkspaceId::new(workspace_id);
    
    // Validate workspace exists
    let workspace = storage
        .get_workspace(&workspace_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Store in request extensions
    request.extensions_mut().insert(workspace_id);
    request.extensions_mut().insert(workspace);
    
    Ok(next.run(request).await)
}
```

### Workspace Management API

```rust
use axum::{
    routing::{delete, get, post},
    Router,
};

/// Workspace API routes
pub fn workspace_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/workspace/:id",
            get(get_workspace).delete(delete_workspace),
        )
        .route(
            "/workspace/:id/insert",
            post(insert_document).layer(axum::middleware::from_fn(workspace_middleware)),
        )
        .route(
            "/workspace/:id/query",
            post(query_workspace).layer(axum::middleware::from_fn(workspace_middleware)),
        )
}

/// Create a new workspace
async fn create_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<Json<Workspace>, ApiError> {
    let workspace = Workspace {
        id: WorkspaceId::new(uuid::Uuid::new_v4().to_string()),
        name: req.name,
        description: req.description,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        settings: req.settings.unwrap_or_default(),
    };
    
    state.storage.create_workspace(&workspace).await?;
    Ok(Json(workspace))
}

/// List all workspaces
async fn list_workspaces(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Workspace>>, ApiError> {
    let workspaces = state.storage.list_workspaces().await?;
    Ok(Json(workspaces))
}

/// Get workspace by ID
async fn get_workspace(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Workspace>, ApiError> {
    let workspace_id = WorkspaceId::new(id);
    let workspace = state
        .storage
        .get_workspace(&workspace_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Workspace not found".to_string()))?;
    Ok(Json(workspace))
}

/// Delete workspace
async fn delete_workspace(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ApiError> {
    let workspace_id = WorkspaceId::new(id);
    state.storage.delete_workspace(&workspace_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Insert document into workspace (workspace-scoped)
async fn insert_document(
    workspace_ctx: WorkspaceContext,
    State(state): State<Arc<AppState>>,
    Json(req): Json<InsertDocumentRequest>,
) -> Result<Json<InsertResponse>, ApiError> {
    // Process document with workspace context
    let result = state.edgequake
        .insert_with_workspace(&workspace_ctx.workspace_id, &req.content)
        .await?;
    
    Ok(Json(InsertResponse {
        document_id: result.document_id,
        entities_extracted: result.entities.len(),
        relations_extracted: result.relations.len(),
    }))
}

/// Query workspace (workspace-scoped)
async fn query_workspace(
    workspace_ctx: WorkspaceContext,
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    // Query with workspace context
    let result = state.edgequake
        .query_with_workspace(
            &workspace_ctx.workspace_id,
            &req.query,
            req.mode.unwrap_or(QueryMode::Hybrid),
        )
        .await?;
    
    Ok(Json(QueryResponse {
        content: result.content,
        entities: result.entities,
        relations: result.relations,
        sources: result.sources,
    }))
}
```

---

## Security Considerations

### 1. Workspace Isolation Validation

```rust
/// Validate that an entity belongs to the correct workspace
pub fn validate_workspace_ownership(
    entity_workspace_id: &WorkspaceId,
    context_workspace_id: &WorkspaceId,
) -> Result<()> {
    if entity_workspace_id != context_workspace_id {
        return Err(anyhow::anyhow!(
            "Workspace mismatch: entity belongs to different workspace"
        ));
    }
    Ok(())
}
```

### 2. PostgreSQL RLS Enforcement

**CRITICAL**: PostgreSQL RLS automatically filters all queries. The session variable `app.current_workspace_id` must be set at the start of each request.

**✅ CORRECT - RLS handles filtering automatically**:
```rust
// Set session variable once per request (in middleware)
sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
    .bind(workspace_id.to_string())
    .execute(&pool)
    .await?;

// Now all queries are automatically filtered by RLS
// NO manual WHERE workspace_id = $workspace_id needed
let entities = sqlx::query_as::<_, Entity>("SELECT * FROM entities WHERE name = $1")
    .bind(name)
    .fetch_all(&pool)
    .await?; // RLS ensures only current workspace entities returned
```

**❌ INCORRECT - Without RLS setup (insecure)**:
```rust
// This query returns entities from ALL workspaces if RLS not enabled
let entities = sqlx::query_as::<_, Entity>("SELECT * FROM entities WHERE name = $1")
    .bind(name)
    .fetch_all(&pool)
    .await?;
```

### 3. Database Permissions

Use SurrealDB's built-in permission system:

```sql
-- Row-level security in SurrealDB
DEFINE TABLE entity PERMISSIONS 
    FOR select, create, update, delete 
    WHERE workspace_id = $auth.workspace_id;
```

### 4. API Authorization

Implement workspace access control:

```rust
pub async fn check_workspace_access(
    user_id: &str,
    workspace_id: &WorkspaceId,
    storage: &dyn WorkspaceStorage,
) -> Result<bool> {
    // Check if user has access to workspace
    // Implementation depends on your auth system
    Ok(true) // Placeholder
}
```

---

## PostgreSQL AGE Implementation

For PostgreSQL + AGE extension:

```rust
pub async fn initialize_postgres_schema(pool: &PgPool) -> Result<()> {
    sqlx::query("
        CREATE TABLE IF NOT EXISTS workspace (
            id VARCHAR(255) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            settings JSONB NOT NULL DEFAULT '{}'::jsonb
        );
        
        CREATE TABLE IF NOT EXISTS entity (
            id VARCHAR(255) NOT NULL,
            workspace_id VARCHAR(255) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
            name VARCHAR(255) NOT NULL,
            entity_type VARCHAR(100) NOT NULL,
            description TEXT,
            embedding vector(1536),
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            PRIMARY KEY (id, workspace_id)
        );
        
        CREATE INDEX entity_workspace_idx ON entity(workspace_id);
        CREATE INDEX entity_vector_idx ON entity USING ivfflat (embedding vector_cosine_ops);
        
        -- AGE graph with workspace isolation
        SELECT create_graph('lightrag_graph');
        
        -- Graph queries must filter by workspace_id property
    ")
    .execute(pool)
    .await?;
    
    Ok(())
}
```

---

## Testing Multi-Tenancy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_workspace_isolation() {
        let storage = setup_test_storage().await;
        
        // Create two workspaces
        let ws1 = WorkspaceId::new("workspace-1");
        let ws2 = WorkspaceId::new("workspace-2");
        
        // Insert entity in workspace 1
        let entity1 = Entity {
            id: "entity-1".to_string(),
            workspace_id: ws1.clone(),
            name: "Test Entity".to_string(),
            // ...
        };
        storage.insert_entity(&ws1, &entity1).await.unwrap();
        
        // Query from workspace 2 should not return entity from workspace 1
        let results = storage
            .query_entities(&ws2, "Test Entity", 10)
            .await
            .unwrap();
        
        assert_eq!(results.len(), 0, "Cross-tenant data leak detected!");
    }
    
    #[tokio::test]
    async fn test_workspace_deletion_cascade() {
        let storage = setup_test_storage().await;
        let ws_id = WorkspaceId::new("test-workspace");
        
        // Create workspace and add data
        create_workspace_with_data(&storage, &ws_id).await;
        
        // Delete workspace
        storage.delete_workspace(&ws_id).await.unwrap();
        
        // Verify all data is deleted
        let entities = storage.query_entities(&ws_id, "", 100).await.unwrap();
        assert_eq!(entities.len(), 0, "Workspace data not deleted!");
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_workspace_api_isolation() {
    let app = test_app().await;
    
    // Create workspace 1
    let ws1 = create_test_workspace(&app, "Workspace 1").await;
    
    // Create workspace 2
    let ws2 = create_test_workspace(&app, "Workspace 2").await;
    
    // Insert document in workspace 1
    let response = app
        .post(&format!("/workspace/{}/insert", ws1.id))
        .json(&json!({"content": "Test document"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    
    // Query from workspace 2 should not find it
    let response = app
        .post(&format!("/workspace/{}/query", ws2.id))
        .json(&json!({"query": "Test document"}))
        .send()
        .await
        .unwrap();
    
    let result: QueryResponse = response.json().await.unwrap();
    assert_eq!(result.entities.len(), 0, "Cross-workspace leak!");
}
```

---

## Performance Optimization

### 1. Index Strategy

Ensure composite indexes for workspace queries:

```sql
-- SurrealDB
DEFINE INDEX entity_workspace_name_idx ON entity FIELDS workspace_id, name;

-- PostgreSQL
CREATE INDEX entity_workspace_name_idx ON entity(workspace_id, name);
```

### 2. Connection Pooling

Use separate connection pools per workspace for high-traffic scenarios:

```rust
pub struct WorkspaceConnectionManager {
    pools: Arc<RwLock<HashMap<WorkspaceId, PgPool>>>,
}
```

### 3. Caching

Implement workspace-scoped caching:

```rust
pub struct WorkspaceCache {
    cache: Arc<RwLock<HashMap<(WorkspaceId, String), CachedValue>>>,
}
```

---

## Migration from Single-Tenant

### Step 1: Add workspace_id Column

```sql
ALTER TABLE entity ADD COLUMN workspace_id VARCHAR(255);
ALTER TABLE relation ADD COLUMN workspace_id VARCHAR(255);
ALTER TABLE document ADD COLUMN workspace_id VARCHAR(255);
```

### Step 2: Backfill with Default Workspace

```sql
UPDATE entity SET workspace_id = 'default' WHERE workspace_id IS NULL;
UPDATE relation SET workspace_id = 'default' WHERE workspace_id IS NULL;
UPDATE document SET workspace_id = 'default' WHERE workspace_id IS NULL;
```

### Step 3: Add Constraints

```sql
ALTER TABLE entity ALTER COLUMN workspace_id SET NOT NULL;
-- Add indexes and foreign keys
```

---

## Best Practices

### DO:

- ✅ Always filter by workspace_id in queries
- ✅ Validate workspace_id on every data access
- ✅ Use database-level isolation features
- ✅ Test cross-tenant isolation thoroughly
- ✅ Implement cascade delete for workspace removal
- ✅ Use composite indexes (workspace_id + other fields)
- ✅ Log workspace context in all operations

### DON'T:

- ❌ Trust client-provided workspace_id without validation
- ❌ Skip workspace_id in WHERE clauses
- ❌ Allow cross-tenant data access through API design flaws
- ❌ Store workspace_id only in application state (always in data)
- ❌ Forget to test workspace isolation
- ❌ Use shared caches without workspace keys

---

## Conclusion

This guide provides a complete multi-tenancy implementation strategy for EdgeQuake. The recommended approach (Shared Database + Tenant ID) balances efficiency, simplicity, and security for most use cases.

**Key Takeaways**:

1. Add `workspace_id` to all data structures
2. Filter all queries by `workspace_id`
3. Use middleware for workspace context extraction
4. Implement comprehensive isolation testing
5. Leverage database-level security features

**Next Steps**:

- Implement workspace management API
- Add storage layer workspace support
- Create comprehensive test suite
- Document deployment considerations

---

**Status**: ✅ COMPLETE - Multi-tenancy implementation guide ready for Phase 0 development
