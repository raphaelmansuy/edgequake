# EdgeQuake API Enhancement Master Plan

**Version:** 2.0.0  
**Target Release:** Q2 2026  
**Status:** Planning Phase  
**Last Updated:** December 22, 2025

---

## Executive Summary

This master plan defines the complete API enhancement roadmap for EdgeQuake to achieve feature parity with LightRAG while maintaining EdgeQuake's performance advantages and idiomatic Rust design patterns.

**Current State:** EdgeQuake 0.1.0 with 11 core endpoints  
**Target State:** EdgeQuake 2.0.0 with 50+ endpoints matching LightRAG functionality  
**Implementation Timeline:** 18 months across 3 major releases

---

## Vision & Goals

### Primary Objectives

1. **Feature Parity:** Achieve 95%+ feature parity with LightRAG API
2. **Performance:** Maintain/improve current performance characteristics (sub-second queries)
3. **Backwards Compatibility:** Ensure existing v1 endpoints remain functional
4. **Production Ready:** Add authentication, multi-tenancy, and observability
5. **Developer Experience:** Maintain clean, type-safe, well-documented APIs

### Non-Goals

- Replicating Python-specific behaviors or idioms
- Compromising performance for feature completeness
- Breaking changes to existing v1 endpoints
- Supporting deprecated LightRAG features

---

## Architecture Principles

### Design Philosophy

1. **Async-First:** All I/O operations use tokio async runtime
2. **Type-Safe:** Leverage Rust's type system for compile-time guarantees
3. **Modular:** Each feature area is a separate crate when appropriate
4. **Testable:** Unit tests for business logic, integration tests for APIs
5. **Observable:** Structured logging, metrics, and distributed tracing
6. **Scalable:** Horizontal scaling support via shared storage backends

### Technology Stack

```
┌─────────────────────────────────────────────────────┐
│                   API Layer (Axum)                   │
├─────────────────────────────────────────────────────┤
│  Auth     │  Tasks   │  Rate     │  Telemetry       │
│ (JWT)     │ (Queue)  │ Limiting  │ (OpenTelemetry)  │
├─────────────────────────────────────────────────────┤
│                Core Business Logic                   │
│  Pipeline  │  Query   │  Graph    │  Admin           │
├─────────────────────────────────────────────────────┤
│              Storage Abstraction Layer               │
│  KV Store  │  Vector  │  Graph    │  Task Queue      │
├─────────────────────────────────────────────────────┤
│                Storage Backends                      │
│  PostgreSQL (AGE + pgvector) │  Redis  │  S3        │
└─────────────────────────────────────────────────────┘
```

---

## Release Roadmap

### Phase 1: Core RAG Enhancements (v1.1.0) - Q4 2025/Q1 2026

**Duration:** 3 months  
**Priority:** HIGH  
**Focus:** Background processing, token controls, conversation history

**Deliverables:**

- ✅ Background task processing with track_id
- ✅ Document status tracking (pending/processing/indexed/failed)
- ✅ Token budget controls (entity, relation, total)
- ✅ Conversation history support
- ✅ Direct text insertion endpoints
- ✅ Enhanced query parameters (keywords, custom prompts)

**New Endpoints:** +8 endpoints (19 total)  
**Breaking Changes:** None  
**Documentation:** [01-background-tasks.md](./01-background-tasks.md), [03-advanced-query.md](./03-advanced-query.md)

---

### Phase 2: Graph Management & Bulk Operations (v1.2.0) - Q2 2026

**Duration:** 3 months  
**Priority:** MEDIUM  
**Focus:** Manual graph editing, bulk operations, enhanced document management

**Deliverables:**

- ✅ Entity CRUD operations (create, read, update, merge)
- ✅ Relationship CRUD operations
- ✅ Bulk document operations (delete all, clear failed)
- ✅ Directory scanning for documents
- ✅ Document statistics and health checks
- ✅ Enhanced graph analytics

**New Endpoints:** +15 endpoints (34 total)  
**Breaking Changes:** None  
**Documentation:** [04-graph-management.md](./04-graph-management.md), [02-document-enhancements.md](./02-document-enhancements.md)

---

### Phase 3: Production Features (v2.0.0) - Q3/Q4 2026

**Duration:** 6 months  
**Priority:** HIGH (for production deployments)  
**Focus:** Authentication, multi-tenancy, observability, scalability

**Deliverables:**

- ✅ JWT authentication + API key support
- ✅ Multi-tenancy with tenant isolation
- ✅ Admin APIs for tenant/KB management
- ✅ Membership & role-based access control (RBAC)
- ✅ OpenTelemetry integration
- ✅ Prometheus metrics export
- ✅ Rate limiting & quota management
- ✅ Horizontal scaling support

**New Endpoints:** +20 endpoints (54+ total)  
**Breaking Changes:** Optional (multi-tenant mode is opt-in)  
**Documentation:** [05-authentication.md](./05-authentication.md), [06-multi-tenancy.md](./06-multi-tenancy.md), [08-observability.md](./08-observability.md)

---

## Feature Mapping: LightRAG → EdgeQuake

### Health & Status

| LightRAG    | EdgeQuake v0.1 | EdgeQuake v2.0 |
| ----------- | -------------- | -------------- |
| GET /health | ✅ /health     | ✅ /health     |
| -           | ✅ /ready      | ✅ /ready      |
| -           | ✅ /live       | ✅ /live       |
| -           | ❌             | ✅ /metrics    |

### Document Management

| LightRAG                       | EdgeQuake v0.1            | EdgeQuake v2.0                       |
| ------------------------------ | ------------------------- | ------------------------------------ |
| POST /documents/upload         | ⚠️ /api/v1/documents      | ✅ /api/v1/documents/upload          |
| POST /documents/text           | ❌                        | ✅ /api/v1/documents/text            |
| POST /documents/texts          | ❌                        | ✅ /api/v1/documents/texts           |
| POST /documents/scan           | ❌                        | ✅ /api/v1/documents/scan            |
| GET /documents/status          | ⚠️ /api/v1/documents      | ✅ /api/v1/documents/status          |
| GET /documents/list            | ✅ /api/v1/documents      | ✅ /api/v1/documents                 |
| GET /documents/{id}            | ✅ /api/v1/documents/{id} | ✅ /api/v1/documents/{id}            |
| DELETE /documents/{id}         | ✅ /api/v1/documents/{id} | ✅ /api/v1/documents/{id}            |
| DELETE /documents/file/{name}  | ❌                        | ✅ /api/v1/documents/file/{filename} |
| DELETE /documents/clear        | ❌                        | ✅ /api/v1/documents/clear           |
| DELETE /documents/failed       | ❌                        | ✅ /api/v1/documents/failed          |
| GET /documents/stats           | ❌                        | ✅ /api/v1/documents/stats           |
| POST /documents/reindex-failed | ❌                        | ✅ /api/v1/documents/reindex-failed  |

### Query Operations

| LightRAG           | EdgeQuake v0.1          | EdgeQuake v2.0           |
| ------------------ | ----------------------- | ------------------------ |
| POST /query        | ✅ /api/v1/query        | ✅ /api/v1/query         |
| POST /query/stream | ✅ /api/v1/query/stream | ✅ /api/v1/query/stream  |
| POST /query/data   | ❌                      | ✅ /api/v1/query/context |

### Graph Operations

| LightRAG                    | EdgeQuake v0.1                 | EdgeQuake v2.0                      |
| --------------------------- | ------------------------------ | ----------------------------------- |
| GET /graphs                 | ✅ /api/v1/graph               | ✅ /api/v1/graph                    |
| GET /graph/label/list       | ❌                             | ✅ /api/v1/graph/labels             |
| GET /graph/label/popular    | ❌                             | ✅ /api/v1/graph/labels/popular     |
| GET /graph/label/search     | ✅ /api/v1/graph/labels/search | ✅ /api/v1/graph/labels/search      |
| GET /graph/nodes/{id}       | ✅ /api/v1/graph/nodes/{id}    | ✅ /api/v1/graph/nodes/{id}         |
| GET /graph/entity/exists    | ❌                             | ✅ /api/v1/graph/entities/exists    |
| POST /graph/entity/create   | ❌                             | ✅ /api/v1/graph/entities           |
| POST /graph/entity/edit     | ❌                             | ✅ /api/v1/graph/entities/{id}      |
| POST /graph/entities/merge  | ❌                             | ✅ /api/v1/graph/entities/merge     |
| POST /graph/relation/create | ❌                             | ✅ /api/v1/graph/relationships      |
| POST /graph/relation/edit   | ❌                             | ✅ /api/v1/graph/relationships/{id} |

### Multi-Tenancy (v2.0 Optional)

| LightRAG                     | EdgeQuake v0.1 | EdgeQuake v2.0             |
| ---------------------------- | -------------- | -------------------------- |
| GET /tenants                 | ❌             | ✅ /api/v1/tenants         |
| GET /tenants/me              | ❌             | ✅ /api/v1/tenants/me      |
| POST /tenants                | ❌             | ✅ /api/v1/tenants         |
| POST /tenants/select         | ❌             | ✅ /api/v1/tenants/select  |
| GET /knowledge-bases         | ❌             | ✅ /api/v1/workspaces      |
| GET /knowledge-bases/{id}    | ❌             | ✅ /api/v1/workspaces/{id} |
| PUT /knowledge-bases/{id}    | ❌             | ✅ /api/v1/workspaces/{id} |
| DELETE /knowledge-bases/{id} | ❌             | ✅ /api/v1/workspaces/{id} |

### Admin Operations (v2.0)

| LightRAG            | EdgeQuake v0.1 | EdgeQuake v2.0           |
| ------------------- | -------------- | ------------------------ |
| POST /admin/tenants | ❌             | ✅ /api/v1/admin/tenants |
| GET /admin/tenants  | ❌             | ✅ /api/v1/admin/tenants |
| GET /admin/stats    | ❌             | ✅ /api/v1/admin/stats   |

### Membership (v2.0)

| LightRAG                              | EdgeQuake v0.1 | EdgeQuake v2.0                                     |
| ------------------------------------- | -------------- | -------------------------------------------------- |
| POST /memberships                     | ❌             | ✅ /api/v1/memberships                             |
| GET /memberships/{tid}                | ❌             | ✅ /api/v1/memberships/{tenant_id}                 |
| PUT /memberships/{tid}/users/{uid}    | ❌             | ✅ /api/v1/memberships/{tenant_id}/users/{user_id} |
| DELETE /memberships/{tid}/users/{uid} | ❌             | ✅ /api/v1/memberships/{tenant_id}/users/{user_id} |
| GET /users/me/tenants                 | ❌             | ✅ /api/v1/users/me/tenants                        |

### Authentication (v2.0)

| LightRAG            | EdgeQuake v0.1 | EdgeQuake v2.0          |
| ------------------- | -------------- | ----------------------- |
| POST /token         | ❌             | ✅ /api/v1/auth/token   |
| POST /token/refresh | ❌             | ✅ /api/v1/auth/refresh |
| POST /logout        | ❌             | ✅ /api/v1/auth/logout  |

---

## API Versioning Strategy

### URL-Based Versioning

```
/api/v1/*  - Current stable API (EdgeQuake 0.1.0+)
/api/v2/*  - Future breaking changes (EdgeQuake 3.0+)
```

### Backwards Compatibility

- All v1.x releases maintain full backwards compatibility with v1.0
- New features add new endpoints or optional parameters
- Deprecated features get 12-month deprecation notice
- v2.0 introduces optional multi-tenancy (backwards compatible via feature flag)

### Version Support Policy

- **Current version (v2.x):** Full support
- **Previous major (v1.x):** Security updates for 24 months after v2.0 release
- **Older versions:** Community support only

---

## Implementation Dependencies

### New Crates

```toml
[workspace]
members = [
    "crates/edgequake-core",
    "crates/edgequake-storage",
    "crates/edgequake-llm",
    "crates/edgequake-pipeline",
    "crates/edgequake-query",
    "crates/edgequake-api",
    # New crates for v2.0:
    "crates/edgequake-auth",        # Authentication & authorization
    "crates/edgequake-tasks",       # Background task queue
    "crates/edgequake-admin",       # Admin operations
    "crates/edgequake-telemetry",   # Observability
]
```

### External Dependencies

```toml
# Background Tasks
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7"
async-channel = "2.1"

# Authentication
jsonwebtoken = "9.2"
argon2 = "0.5"
tower-http = { version = "0.5", features = ["auth"] }

# Rate Limiting
governor = "0.6"
tower-governor = "0.1"

# Observability
opentelemetry = { version = "0.21", features = ["trace", "metrics"] }
opentelemetry-jaeger = "0.20"
tracing-opentelemetry = "0.22"
prometheus = "0.13"

# Task Queue (optional: Redis-backed)
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# Multi-tenancy
uuid = { version = "1.6", features = ["v4", "serde"] }
```

---

## Database Schema Changes

### New Tables (Phase 1)

```sql
-- Task tracking
CREATE TABLE tasks (
    track_id VARCHAR(50) PRIMARY KEY,
    task_type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB
);

-- Document status tracking
CREATE TABLE document_status (
    doc_id VARCHAR(100) PRIMARY KEY,
    file_path TEXT,
    status VARCHAR(20) NOT NULL,
    track_id VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    indexed_at TIMESTAMPTZ,
    error_message TEXT,
    chunk_count INTEGER DEFAULT 0,
    entity_count INTEGER DEFAULT 0,
    relationship_count INTEGER DEFAULT 0,
    FOREIGN KEY (track_id) REFERENCES tasks(track_id)
);

-- Conversation history
CREATE TABLE conversation_history (
    id SERIAL PRIMARY KEY,
    session_id VARCHAR(100) NOT NULL,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);
CREATE INDEX idx_conv_session ON conversation_history(session_id, created_at);
```

### New Tables (Phase 3 - Multi-tenancy)

```sql
-- Tenants
CREATE TABLE tenants (
    tenant_id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

-- Workspaces (Knowledge Bases)
CREATE TABLE workspaces (
    workspace_id VARCHAR(100) PRIMARY KEY,
    tenant_id VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB,
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE
);

-- Users
CREATE TABLE users (
    user_id VARCHAR(100) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

-- Memberships (User-Tenant-Role)
CREATE TABLE memberships (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    tenant_id VARCHAR(100) NOT NULL,
    role VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    UNIQUE(user_id, tenant_id)
);
```

---

## Configuration Strategy

### Environment Variables

```bash
# Core
EDGEQUAKE_HOST=0.0.0.0
EDGEQUAKE_PORT=8080
EDGEQUAKE_LOG_LEVEL=info

# Storage
DATABASE_URL=postgresql://user:pass@localhost:5432/edgequake
REDIS_URL=redis://localhost:6379  # Optional: for distributed task queue

# LLM
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
EMBEDDING_MODEL=text-embedding-3-small

# Auth (v2.0)
JWT_SECRET=...
JWT_EXPIRY_HOURS=24
API_KEYS=key1,key2,key3  # Comma-separated

# Multi-tenancy (v2.0)
EDGEQUAKE_MULTI_TENANT=false  # Feature flag
DEFAULT_TENANT_ID=default
DEFAULT_WORKSPACE_ID=default

# Observability (v2.0)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
PROMETHEUS_PORT=9090

# Rate Limiting (v2.0)
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10
```

### Feature Flags

```toml
[features]
default = ["postgres"]

# Storage backends
postgres = ["dep:sqlx", "dep:deadpool-postgres"]
redis-tasks = ["dep:redis"]

# Phase 3 features
auth = ["dep:jsonwebtoken", "dep:argon2"]
multi-tenant = ["auth"]
telemetry = ["dep:opentelemetry", "dep:prometheus"]
rate-limiting = ["dep:governor"]

# All features
full = ["postgres", "redis-tasks", "auth", "multi-tenant", "telemetry", "rate-limiting"]
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    // Business logic tests (no I/O)
    #[test]
    fn test_token_budget_calculation() { }

    #[test]
    fn test_conversation_history_formatting() { }

    #[test]
    fn test_entity_normalization() { }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_background_task_execution() {
    // Full pipeline with mock storage
}

#[tokio::test]
async fn test_document_status_tracking() {
    // End-to-end document processing
}
```

### API Tests

```rust
#[tokio::test]
async fn test_query_with_conversation_history() {
    let client = TestClient::new().await;
    let response = client
        .post("/api/v1/query")
        .json(&QueryRequest { /* ... */ })
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
}
```

### Load Tests

```bash
# Using k6 for load testing
k6 run --vus 100 --duration 30s tests/load/query_stress.js
```

---

## Documentation Requirements

### API Documentation

- ✅ OpenAPI 3.1 spec via utoipa
- ✅ Interactive Swagger UI at `/swagger-ui`
- ✅ Redoc UI at `/redoc`
- ✅ Example requests/responses for all endpoints
- ✅ Authentication flow documentation
- ✅ Multi-tenant setup guide

### Developer Documentation

- ✅ Getting Started guide
- ✅ API reference (generated from OpenAPI)
- ✅ Architecture decision records (ADRs)
- ✅ Migration guide from v1.x to v2.0
- ✅ Deployment runbook
- ✅ Configuration reference

### User Documentation

- ✅ Query optimization guide
- ✅ Graph management best practices
- ✅ Multi-tenant setup tutorial
- ✅ Troubleshooting guide

---

## Success Metrics

### Phase 1 (v1.1.0)

- ✅ All tests pass (unit, integration, API)
- ✅ Zero breaking changes to v1.0 API
- ✅ Background tasks complete with <5% error rate
- ✅ Query response time <1s (p95)
- ✅ Token budget reduces LLM costs by 30%+

### Phase 2 (v1.2.0)

- ✅ Graph editing operations have <100ms latency
- ✅ Bulk operations handle 10,000+ documents
- ✅ Zero data loss in graph merge operations
- ✅ API documentation coverage 100%

### Phase 3 (v2.0.0)

- ✅ Authentication adds <50ms overhead
- ✅ Multi-tenant mode supports 1000+ tenants
- ✅ Horizontal scaling demonstrated (3+ nodes)
- ✅ Prometheus metrics exported (20+ metrics)
- ✅ OpenTelemetry traces captured (<1% sampling overhead)
- ✅ Rate limiting prevents abuse (99.9% accuracy)

---

## Risk Assessment

### Technical Risks

| Risk                             | Impact | Probability | Mitigation                                      |
| -------------------------------- | ------ | ----------- | ----------------------------------------------- |
| Background task queue complexity | HIGH   | MEDIUM      | Use proven libraries (tokio), extensive testing |
| Multi-tenant data isolation bugs | HIGH   | LOW         | Strict tenant_id checks, comprehensive tests    |
| Authentication vulnerabilities   | HIGH   | MEDIUM      | Security audit, follow OWASP guidelines         |
| Performance degradation          | MEDIUM | MEDIUM      | Continuous benchmarking, profiling              |
| Breaking changes slip through    | MEDIUM | LOW         | Strict API versioning, deprecation policy       |

### Schedule Risks

| Risk                   | Impact | Probability | Mitigation                          |
| ---------------------- | ------ | ----------- | ----------------------------------- |
| Phase 1 delays Phase 2 | MEDIUM | MEDIUM      | Parallel workstreams where possible |
| LLM API changes        | LOW    | LOW         | Abstraction layer insulates changes |
| Storage backend issues | MEDIUM | LOW         | Maintain backwards compatibility    |

---

## Related Documents

### Specification Documents

1. [01-background-tasks.md](./01-background-tasks.md) - Background task processing system
2. [02-document-enhancements.md](./02-document-enhancements.md) - Enhanced document management
3. [03-advanced-query.md](./03-advanced-query.md) - Advanced query features
4. [04-graph-management.md](./04-graph-management.md) - Graph CRUD operations
5. [05-authentication.md](./05-authentication.md) - Authentication & authorization
6. [06-multi-tenancy.md](./06-multi-tenancy.md) - Multi-tenant architecture
7. [07-api-reference.md](./07-api-reference.md) - Complete API reference (v2.0)
8. [08-observability.md](./08-observability.md) - Observability & monitoring

### Supporting Documents

- [../docs/API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md](../docs/API_COMPARISON_EDGEQUAKE_VS_LIGHTRAG.md) - Detailed comparison
- [../docs/API_COMPARISON_SUMMARY.md](../docs/API_COMPARISON_SUMMARY.md) - Quick reference
- [./MIGRATION_GUIDE.md](./MIGRATION_GUIDE.md) - v1 to v2 migration guide

---

## Approval & Sign-off

### Phase 1 Approval

- [ ] Architecture Review
- [ ] Security Review
- [ ] Performance Benchmark Baseline
- [ ] Documentation Review

### Phase 2 Approval

- [ ] Phase 1 Complete & Stable
- [ ] Graph Management Design Review
- [ ] Integration Test Coverage >80%

### Phase 3 Approval

- [ ] Phase 2 Complete & Stable
- [ ] Security Audit Complete
- [ ] Production Readiness Review
- [ ] Load Testing Results Acceptable

---

**Next Steps:**

1. Review this master plan with stakeholders
2. Prioritize Phase 1 specifications
3. Begin implementation of background task system
4. Set up CI/CD pipeline for automated testing

**Status:** ✅ Master Plan Complete - Ready for Phase 1 Implementation
