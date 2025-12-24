# EdgeQuake Full Implementation Plan

**Date:** December 22, 2025  
**Purpose:** Complete API implementation audit and e2e test formalization  
**Priority:** HIGH STAKE MISSION  
**Comparison Source:** LightRAG Python implementation

---

## Executive Summary

This document defines the complete implementation plan for EdgeQuake API v2.0, ensuring feature parity with LightRAG and full e2e test coverage with real PostgreSQL and OpenAI LLM.

---

## 1. Current State Audit

### 1.1 API Endpoints Status

| Endpoint            | Route                                   | Current Status           | Required Action     |
| ------------------- | --------------------------------------- | ------------------------ | ------------------- |
| **Health**          |                                         |                          |                     |
| Health Check        | GET /health                             | ✅ Implemented           | None                |
| Readiness           | GET /ready                              | ✅ Implemented           | None                |
| Liveness            | GET /live                               | ✅ Implemented           | None                |
| Metrics             | GET /metrics                            | ✅ Implemented           | Add real metrics    |
| **Authentication**  |                                         |                          |                     |
| Login               | POST /api/v1/auth/login                 | ⚠️ Stub (NotImplemented) | Full implementation |
| Refresh             | POST /api/v1/auth/refresh               | ⚠️ Stub                  | Full implementation |
| Logout              | POST /api/v1/auth/logout                | ⚠️ Stub                  | Full implementation |
| Get Me              | GET /api/v1/auth/me                     | ⚠️ Stub                  | Full implementation |
| **Users**           |                                         |                          |                     |
| Create User         | POST /api/v1/users                      | ⚠️ Stub                  | Full implementation |
| List Users          | GET /api/v1/users                       | ⚠️ Stub                  | Full implementation |
| Get User            | GET /api/v1/users/{id}                  | ⚠️ Stub                  | Full implementation |
| Delete User         | DELETE /api/v1/users/{id}               | ⚠️ Stub                  | Full implementation |
| **API Keys**        |                                         |                          |                     |
| Create API Key      | POST /api/v1/api-keys                   | ⚠️ Stub                  | Full implementation |
| List API Keys       | GET /api/v1/api-keys                    | ⚠️ Stub                  | Full implementation |
| Revoke API Key      | DELETE /api/v1/api-keys/{id}            | ⚠️ Stub                  | Full implementation |
| **Documents**       |                                         |                          |                     |
| Upload Document     | POST /api/v1/documents                  | ✅ Implemented           | Enhance with dedup  |
| List Documents      | GET /api/v1/documents                   | ✅ Implemented           | None                |
| Get Document        | GET /api/v1/documents/{id}              | ✅ Implemented           | None                |
| Delete Document     | DELETE /api/v1/documents/{id}           | ✅ Implemented           | None                |
| **Query**           |                                         |                          |                     |
| Execute Query       | POST /api/v1/query                      | ✅ Implemented           | Add LightRAG params |
| Stream Query        | POST /api/v1/query/stream               | ✅ Implemented           | Verify streaming    |
| **Graph**           |                                         |                          |                     |
| Get Graph           | GET /api/v1/graph                       | ✅ Implemented           | None                |
| Get Node            | GET /api/v1/graph/nodes/{id}            | ✅ Implemented           | None                |
| Search Labels       | GET /api/v1/graph/labels/search         | ✅ Implemented           | None                |
| **Entities**        |                                         |                          |                     |
| Create Entity       | POST /api/v1/graph/entities             | ✅ Implemented           | Verify              |
| Get Entity          | GET /api/v1/graph/entities/{name}       | ✅ Implemented           | Verify              |
| Update Entity       | PUT /api/v1/graph/entities/{name}       | ✅ Implemented           | Verify              |
| Delete Entity       | DELETE /api/v1/graph/entities/{name}    | ✅ Implemented           | Verify              |
| Entity Exists       | GET /api/v1/graph/entities/exists       | ✅ Implemented           | Verify              |
| Merge Entities      | POST /api/v1/graph/entities/merge       | ✅ Implemented           | Verify              |
| **Relationships**   |                                         |                          |                     |
| Create Relationship | POST /api/v1/graph/relationships        | ✅ Implemented           | Verify              |
| Get Relationship    | GET /api/v1/graph/relationships/{id}    | ✅ Implemented           | Verify              |
| Update Relationship | PUT /api/v1/graph/relationships/{id}    | ✅ Implemented           | Verify              |
| Delete Relationship | DELETE /api/v1/graph/relationships/{id} | ✅ Implemented           | Verify              |
| **Tasks**           |                                         |                          |                     |
| Get Task            | GET /api/v1/tasks/{id}                  | ✅ Implemented           | Verify              |
| List Tasks          | GET /api/v1/tasks                       | ✅ Implemented           | Verify              |
| Cancel Task         | POST /api/v1/tasks/{id}/cancel          | ✅ Implemented           | Verify              |
| Retry Task          | POST /api/v1/tasks/{id}/retry           | ✅ Implemented           | Verify              |

**Summary:** 38 endpoints total, 11 stubs needing full implementation

---

## 2. LightRAG Feature Comparison

### 2.1 Document Routes (lightrag/api/routers/document_routes.py)

| LightRAG Feature            | EdgeQuake Status     | Gap                          |
| --------------------------- | -------------------- | ---------------------------- |
| File upload (multipart)     | ❌ Missing           | Add POST /documents/upload   |
| Text insertion              | ✅ Via content field | None                         |
| Batch text insertion        | ❌ Missing           | Add POST /documents/texts    |
| Content deduplication (MD5) | ❌ Missing           | Add SHA-256 hashing          |
| Document status tracking    | ⚠️ Partial           | Add track_id workflow        |
| Directory scanning          | ❌ Missing           | Add POST /documents/scan     |
| Reprocess failed            | ❌ Missing           | Add POST /documents/reindex  |
| Cancel pipeline             | ❌ Missing           | Add POST /documents/cancel   |
| Clear all documents         | ❌ Missing           | Add DELETE /documents/clear  |
| Delete failed documents     | ❌ Missing           | Add DELETE /documents/failed |

### 2.2 Query Routes (lightrag/api/routers/query_routes.py)

| LightRAG Feature                                                        | EdgeQuake Status | Gap                            |
| ----------------------------------------------------------------------- | ---------------- | ------------------------------ |
| Query modes (local/global/hybrid/naive/mix/bypass)                      | ✅ Implemented   | Verify bypass mode             |
| Token budget (max_entity_tokens, max_relation_tokens, max_total_tokens) | ⚠️ Partial       | Verify implementation          |
| Conversation history                                                    | ❌ Missing       | Add conversation_history param |
| Keywords (hl_keywords, ll_keywords)                                     | ❌ Missing       | Add keyword params             |
| only_need_context                                                       | ⚠️ Partial       | Verify implementation          |
| only_need_prompt                                                        | ⚠️ Partial       | Verify implementation          |
| enable_rerank                                                           | ❌ Missing       | Add rerank param               |
| include_references                                                      | ❌ Missing       | Add references param           |
| Query data endpoint                                                     | ❌ Missing       | Add POST /query/data           |

### 2.3 Graph Routes (lightrag/api/routers/graph_routes.py)

| LightRAG Feature   | EdgeQuake Status | Gap                           |
| ------------------ | ---------------- | ----------------------------- |
| Entity CRUD        | ✅ Implemented   | Verify is_manual flag         |
| Relationship CRUD  | ✅ Implemented   | Verify is_manual flag         |
| Entity merge       | ✅ Implemented   | Verify merge strategy         |
| Get all labels     | ❌ Missing       | Add GET /graph/labels         |
| Get popular labels | ❌ Missing       | Add GET /graph/labels/popular |
| Graph statistics   | ❌ Missing       | Add GET /graph/statistics     |

### 2.4 Authentication (lightrag/api/auth.py)

| LightRAG Feature           | EdgeQuake Status           | Gap                     |
| -------------------------- | -------------------------- | ----------------------- |
| JWT generation             | ✅ In edgequake-auth crate | Integrate into handlers |
| JWT validation             | ✅ In edgequake-auth crate | Integrate into handlers |
| API key auth               | ✅ In edgequake-auth crate | Integrate into handlers |
| Password hashing (Argon2)  | ✅ In edgequake-auth crate | Integrate into handlers |
| RBAC (admin/user/readonly) | ✅ In edgequake-auth crate | Integrate into handlers |
| Auth middleware            | ✅ In edgequake-auth crate | Integrate into router   |

---

## 3. Implementation Plan

### Phase A: Complete Auth Handler Implementation (Priority 1)

**Files to modify:**

- `edgequake/crates/edgequake-api/src/handlers/auth.rs`
- `edgequake/crates/edgequake-api/src/state.rs`

**Tasks:**

1. Add auth services to AppState (JwtService, PasswordService)
2. Implement login handler with:
   - User lookup by username/email (mock for now, DB later)
   - Password verification using Argon2
   - JWT token generation
   - Refresh token generation and storage
3. Implement refresh_token handler
4. Implement logout handler (token revocation)
5. Implement get_me handler (return user from token)
6. Implement create_user handler (hash password, store user)
7. Implement list_users, get_user, delete_user handlers
8. Implement API key handlers (create, list, revoke)

### Phase B: Enhance Document Handlers (Priority 2)

**Files to modify:**

- `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Tasks:**

1. Add content deduplication (SHA-256 hash)
2. Add track_id generation for async processing
3. Add document status endpoint
4. Add batch text insertion endpoint

### Phase C: Enhance Query Handlers (Priority 2)

**Files to modify:**

- `edgequake/crates/edgequake-api/src/handlers/query.rs`

**Tasks:**

1. Add conversation_history support
2. Add keyword extraction (hl_keywords, ll_keywords)
3. Verify token budget implementation
4. Add include_references option
5. Verify bypass mode

### Phase D: Verify Entity/Relationship Handlers (Priority 3)

**Files to verify:**

- `edgequake/crates/edgequake-api/src/handlers/entities.rs`
- `edgequake/crates/edgequake-api/src/handlers/relationships.rs`

**Tasks:**

1. Verify is_manual flag handling
2. Verify merge strategy implementation
3. Verify cascade delete logic

### Phase E: Comprehensive E2E Tests (Priority 1)

**Files to create:**

- `edgequake/crates/edgequake-api/tests/e2e_auth.rs`
- `edgequake/crates/edgequake-api/tests/e2e_documents.rs`
- `edgequake/crates/edgequake-api/tests/e2e_query.rs`
- `edgequake/crates/edgequake-api/tests/e2e_graph.rs`
- `edgequake/crates/edgequake-core/tests/e2e_full_pipeline_postgres.rs`
- `edgequake/crates/edgequake-core/tests/e2e_full_pipeline_openai.rs`

**Test Categories:**

1. **Auth E2E Tests:**

   - Login with valid credentials
   - Login with invalid credentials
   - Token refresh
   - Token expiration
   - Logout
   - User CRUD
   - API key CRUD
   - RBAC enforcement

2. **Document E2E Tests:**

   - Upload document
   - Upload duplicate (deduplication)
   - List documents with pagination
   - Get document by ID
   - Delete document
   - Document status tracking

3. **Query E2E Tests:**

   - Query with each mode
   - Query with conversation history
   - Query with token budgets
   - Streaming query
   - Context-only query

4. **Graph E2E Tests:**

   - Entity CRUD
   - Relationship CRUD
   - Entity merge
   - Entity existence check
   - Label search

5. **Full Pipeline Tests (PostgreSQL):**

   - End-to-end document ingestion
   - Entity extraction verification
   - Relationship extraction verification
   - Query retrieval accuracy

6. **Full Pipeline Tests (OpenAI):**
   - Real LLM entity extraction
   - Real LLM relationship extraction
   - Real LLM query response

---

## 4. Database Requirements

### 4.1 Required Tables (from migrations)

| Table                | Purpose                     | Status           |
| -------------------- | --------------------------- | ---------------- |
| tasks                | Background task tracking    | ✅ Created       |
| document_status      | Document processing status  | ✅ Created       |
| conversation_history | Query conversation context  | ✅ Created       |
| audit_log            | Entity/relationship changes | ✅ Created       |
| users                | User accounts               | ✅ Created (006) |
| api_keys             | API key storage             | ✅ Created (006) |
| refresh_tokens       | JWT refresh tokens          | ✅ Created (006) |
| tenants              | Multi-tenant organizations  | ✅ Created (007) |
| workspaces           | Tenant workspaces           | ✅ Created (007) |
| memberships          | Tenant/workspace membership | ✅ Created (007) |

### 4.2 PostgreSQL Test Setup

```bash
# Start PostgreSQL with AGE extension
docker-compose -f docker/docker-compose.yml up -d postgres

# Run migrations
export DATABASE_URL="postgresql://edgequake:edgequake@localhost:5432/edgequake"
cargo run --package edgequake-storage -- migrate

# Verify connection
cargo test --package edgequake-storage --test postgres_integration
```

---

## 5. OpenAI Test Setup

```bash
# Set API key
export OPENAI_API_KEY="sk-..."

# Run with real LLM
cargo test --package edgequake-core --test e2e_full_pipeline_openai

# Verify entity extraction quality
# Expected: 20+ entities with real LLM vs 6-9 with mock
```

---

## 6. Success Criteria

### 6.1 Implementation Completeness

- [ ] All 38 endpoints return proper responses (not NotImplemented)
- [ ] All auth handlers use real auth services
- [ ] All document handlers have deduplication
- [ ] All query handlers support LightRAG params
- [ ] All entity/relationship handlers verified

### 6.2 Test Coverage

- [ ] 100+ e2e tests covering all endpoints
- [ ] PostgreSQL integration tests passing
- [ ] OpenAI integration tests passing
- [ ] All existing 377+ tests passing

### 6.3 Feature Parity

- [ ] All LightRAG document features implemented
- [ ] All LightRAG query features implemented
- [ ] All LightRAG graph features implemented
- [ ] All LightRAG auth features implemented

---

## 7. Execution Order

1. **Phase A:** Complete auth handlers (11 endpoints) - CRITICAL
2. **Phase E:** Create e2e test framework - CRITICAL
3. **Phase B:** Enhance document handlers - HIGH
4. **Phase C:** Enhance query handlers - HIGH
5. **Phase D:** Verify entity/relationship handlers - MEDIUM
6. **Final:** Run all tests with PostgreSQL + OpenAI - CRITICAL

---

## 8. Estimated Timeline

| Phase            | Duration | Endpoints |
| ---------------- | -------- | --------- |
| Phase A (Auth)   | 2 hours  | 11        |
| Phase E (Tests)  | 2 hours  | All       |
| Phase B (Docs)   | 1 hour   | 4         |
| Phase C (Query)  | 1 hour   | 2         |
| Phase D (Graph)  | 30 min   | 10        |
| Final Validation | 1 hour   | All       |

**Total: ~7.5 hours**

---

**Document Version:** 1.0  
**Author:** GitHub Copilot  
**Status:** READY FOR EXECUTION
