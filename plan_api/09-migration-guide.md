# Migration & Implementation Guide

**Version:** 1.0  
**Last Updated:** December 22, 2025  
**Purpose:** Step-by-step guide for implementing API v2.0

---

## Migration Path Overview

```
v1.0 (Current)     v1.1 (Phase 1)      v1.2 (Phase 2)      v2.0 (Phase 3)
11 endpoints   →   19 endpoints    →   34 endpoints    →   59 endpoints
3 months           +4 months           +6 months           +5 months
```

---

## Phase 1: Background Tasks (v1.1.0)

**Timeline:** 3-4 months  
**Endpoints:** +8 (11 → 19)

### Implementation Checklist

- [ ] **1. Task Queue System** (2 weeks)
  - [ ] Create `tasks` table schema
  - [ ] Implement `TaskQueue` trait
  - [ ] Implement `ChannelTaskQueue` (tokio channels)
  - [ ] Implement `RedisTaskQueue` (optional)
  - [ ] Add task status enum (Created, Pending, Processing, Indexed, Failed)
  - [ ] Track ID generation: `{type}-{uuid}`

- [ ] **2. Background Worker Pool** (1 week)
  - [ ] Implement `WorkerPool` with configurable threads
  - [ ] Task processing loop
  - [ ] Error handling and retry logic
  - [ ] Graceful shutdown

- [ ] **3. Document Status Tracking** (2 weeks)
  - [ ] Create `document_status` table
  - [ ] Add `track_id`, `status`, `progress_percent` columns
  - [ ] Implement status update logic
  - [ ] Add SHA-256 content hashing for deduplication

- [ ] **4. New Endpoints** (2 weeks)
  - [ ] `POST /documents/upload` (multipart file upload)
  - [ ] `POST /documents/text` (direct text insert)
  - [ ] `POST /documents/texts` (batch text insert)
  - [ ] `GET /documents/status` (query status)
  - [ ] `GET /tasks/{track_id}` (poll task)
  - [ ] `GET /tasks` (list tasks)
  - [ ] `POST /tasks/{id}/cancel` (cancel task)
  - [ ] `POST /tasks/{id}/retry` (retry failed)

- [ ] **5. Enhanced Query** (2 weeks)
  - [ ] Add token budget parameters to `QueryRequest`
  - [ ] Implement `TokenBudgetController`
  - [ ] Add conversation history support
  - [ ] Implement `ConversationHistoryManager`
  - [ ] Add keyword extraction (hl_keywords, ll_keywords)
  - [ ] Add bypass mode (direct LLM)
  - [ ] Add `POST /query/context` endpoint

- [ ] **6. Testing & Documentation** (1 week)
  - [ ] Unit tests for task queue
  - [ ] Integration tests for async workflows
  - [ ] API tests for new endpoints
  - [ ] Update OpenAPI spec
  - [ ] Write migration guide for users

### Database Migrations

```sql
-- Migration: 001_background_tasks.sql

CREATE TABLE tasks (
    task_id VARCHAR(100) PRIMARY KEY,
    track_id VARCHAR(100) UNIQUE NOT NULL,
    task_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    payload JSONB,
    result JSONB,
    error_message TEXT,
    progress_percent INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    metadata JSONB
);

CREATE INDEX idx_tasks_track_id ON tasks(track_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_created_at ON tasks(created_at);

-- Migration: 002_document_status.sql

CREATE TABLE document_status (
    document_id VARCHAR(100) PRIMARY KEY,
    filename VARCHAR(255),
    content_hash VARCHAR(64),
    status VARCHAR(50) NOT NULL,
    track_id VARCHAR(100),
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB,
    FOREIGN KEY (track_id) REFERENCES tasks(track_id)
);

CREATE INDEX idx_document_status_hash ON document_status(content_hash);
CREATE INDEX idx_document_status_status ON document_status(status);

-- Migration: 003_conversation_history.sql

CREATE TABLE conversation_history (
    id SERIAL PRIMARY KEY,
    session_id VARCHAR(100) NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_conversation_session ON conversation_history(session_id);
```

### Breaking Changes

**None** - All v1.0 endpoints remain unchanged. New endpoints are additive.

### Backwards Compatibility

```rust
// v1.0 synchronous API still works
POST /documents
{
  "content": "...",
  "document_id": "doc-123"
}

// Response: 201 Created (waits for indexing to complete)

// v1.1 async API (recommended)
POST /documents/text
{
  "content": "...",
  "document_id": "doc-123"
}

// Response: 202 Accepted + track_id
{
  "track_id": "upload-abc123",
  "status": "pending"
}

// Poll status
GET /tasks/upload-abc123
```

---

## Phase 2: Graph Management (v1.2.0)

**Timeline:** 4-6 months  
**Endpoints:** +15 (19 → 34)

### Implementation Checklist

- [ ] **1. Entity CRUD** (3 weeks)
  - [ ] `POST /graph/entities` (create entity)
  - [ ] `GET /graph/entities/{id}` (get entity)
  - [ ] `PUT /graph/entities/{id}` (update entity)
  - [ ] `DELETE /graph/entities/{id}` (delete entity)
  - [ ] `GET /graph/entities/exists` (check exists)
  - [ ] `POST /graph/entities/merge` (merge duplicates)

- [ ] **2. Relationship CRUD** (2 weeks)
  - [ ] `POST /graph/relationships` (create relationship)
  - [ ] `GET /graph/relationships/{id}` (get relationship)
  - [ ] `PUT /graph/relationships/{id}` (update relationship)
  - [ ] `DELETE /graph/relationships/{id}` (delete relationship)

- [ ] **3. Bulk Operations** (2 weeks)
  - [ ] `POST /documents/scan` (directory scanner)
  - [ ] `DELETE /documents/clear` (clear all)
  - [ ] `DELETE /documents/failed` (delete failed)
  - [ ] `POST /documents/reindex-failed` (retry failed)

- [ ] **4. Graph Analytics** (2 weeks)
  - [ ] `GET /graph/statistics` (node/edge counts, centrality)
  - [ ] `GET /graph/labels/popular` (most common labels)
  - [ ] `GET /graph/labels/search` (search labels)

- [ ] **5. Audit Logging** (1 week)
  - [ ] Create `audit_log` table
  - [ ] Log all graph mutations
  - [ ] Add `is_manual` flag to entities/relationships

- [ ] **6. Testing & Documentation** (2 weeks)
  - [ ] Unit tests for graph operations
  - [ ] Integration tests for CRUD
  - [ ] API tests for bulk operations
  - [ ] Update OpenAPI spec

### Database Migrations

```sql
-- Migration: 004_audit_log.sql

CREATE TABLE audit_log (
    id SERIAL PRIMARY KEY,
    operation VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(100),
    user_id VARCHAR(100),
    changes JSONB,
    reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_user ON audit_log(user_id);
CREATE INDEX idx_audit_created_at ON audit_log(created_at);

-- Migration: 005_manual_entities.sql

-- Add is_manual flag to Entity nodes in AGE
ALTER TABLE ag_catalog.ag_label 
ADD COLUMN is_manual BOOLEAN DEFAULT FALSE;
```

### Breaking Changes

**None** - Existing graph queries continue to work.

---

## Phase 3: Production Features (v2.0.0)

**Timeline:** 5-6 months  
**Endpoints:** +25 (34 → 59)

### Implementation Checklist

- [ ] **1. Authentication** (4 weeks)
  - [ ] Create `users`, `api_keys`, `refresh_tokens` tables
  - [ ] Implement JWT generation/validation
  - [ ] Implement API key authentication
  - [ ] Create auth middleware
  - [ ] Add `AuthUser` extractor
  - [ ] Endpoints: `/auth/token`, `/auth/refresh`, `/auth/logout`

- [ ] **2. Multi-Tenancy** (6 weeks)
  - [ ] Create `tenants`, `workspaces`, `memberships` tables
  - [ ] Implement tenant context middleware
  - [ ] Update storage adapters for tenant isolation
  - [ ] Add `X-Tenant-ID` and `X-Workspace-ID` headers
  - [ ] Feature flag: `multi-tenant`
  - [ ] Endpoints: `/tenants`, `/workspaces`, `/memberships`

- [ ] **3. RBAC** (2 weeks)
  - [ ] Define roles: admin, user, readonly
  - [ ] Implement permission checker
  - [ ] Add role-based endpoint guards
  - [ ] Admin endpoints: `/admin/*`

- [ ] **4. Observability** (3 weeks)
  - [ ] Integrate OpenTelemetry
  - [ ] Add Prometheus metrics (20+ metrics)
  - [ ] Implement distributed tracing
  - [ ] Structured JSON logging
  - [ ] Create Grafana dashboards
  - [ ] Endpoint: `/metrics`

- [ ] **5. Rate Limiting** (1 week)
  - [ ] Implement token bucket algorithm
  - [ ] Per-user and per-tenant limits
  - [ ] Return 429 status on limit exceeded

- [ ] **6. Testing & Documentation** (2 weeks)
  - [ ] Security tests (authentication, authorization)
  - [ ] Multi-tenant isolation tests
  - [ ] Load tests with rate limiting
  - [ ] Complete OpenAPI spec
  - [ ] Production deployment guide

### Database Migrations

```sql
-- Migration: 006_authentication.sql

CREATE TABLE users (
    user_id VARCHAR(100) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    metadata JSONB
);

CREATE TABLE api_keys (
    key_id VARCHAR(100) PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    name VARCHAR(255),
    scopes TEXT[],
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE TABLE refresh_tokens (
    token_id VARCHAR(100) PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

-- Migration: 007_multi_tenancy.sql

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
    UNIQUE(user_id, tenant_id)
);
```

### Breaking Changes

**Authentication Required:**
All endpoints (except `/health`, `/ready`, `/metrics`) now require authentication.

**Migration Steps:**
1. Create default admin user
2. Generate API keys for existing clients
3. Update client code to include `Authorization` header
4. Optional: Enable multi-tenant mode

```bash
# Create default admin
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "email": "admin@example.com",
    "password": "secure_password",
    "role": "admin"
  }'

# Login
curl -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secure_password"
  }'
```

---

## Testing Strategy

### Unit Tests

```bash
# Test individual components
cargo test --package edgequake-core
cargo test --package edgequake-storage
cargo test --package edgequake-llm
```

### Integration Tests

```bash
# Test component interactions
cargo test --package edgequake-core --test integration
```

### API Tests

```bash
# Test HTTP endpoints
cargo test --package edgequake-api --test api_tests
```

### E2E Tests

```bash
# Full system tests
cargo test --package edgequake --test e2e_tests
```

### Load Tests

```bash
# Stress test with vegeta or k6
vegeta attack -rate=100 -duration=60s -targets=targets.txt | vegeta report
```

---

## Deployment Checklist

- [ ] Set environment variables (JWT_SECRET, DATABASE_URL, etc.)
- [ ] Run database migrations
- [ ] Create default admin user
- [ ] Configure observability (Prometheus, Jaeger)
- [ ] Set up TLS/SSL certificates
- [ ] Configure rate limits
- [ ] Enable multi-tenant mode (optional)
- [ ] Set up backup strategy
- [ ] Configure log rotation
- [ ] Test health endpoints
- [ ] Load test with production-like data
- [ ] Set up monitoring alerts
- [ ] Document API keys for clients

---

## Rollback Plan

### v1.1 → v1.0

```bash
# Revert database migrations
psql -d edgequake -f migrations/rollback_001.sql
psql -d edgequake -f migrations/rollback_002.sql
psql -d edgequake -f migrations/rollback_003.sql

# Deploy previous version
docker pull edgequake:v1.0.0
docker-compose up -d
```

### v1.2 → v1.1

```bash
# Revert migrations
psql -d edgequake -f migrations/rollback_004.sql
psql -d edgequake -f migrations/rollback_005.sql

# Deploy v1.1
docker pull edgequake:v1.1.0
docker-compose up -d
```

### v2.0 → v1.2

```bash
# Revert authentication and multi-tenancy migrations
psql -d edgequake -f migrations/rollback_006.sql
psql -d edgequake -f migrations/rollback_007.sql

# Deploy v1.2
docker pull edgequake:v1.2.0
docker-compose up -d

# Update clients to remove Authorization headers
```

---

**Status:** ✅ Migration Guide Complete  
**Total Timeline:** 18 months  
**Risk Level:** Medium (requires careful testing)
