# OODA Loop Iteration #06 - Decide Phase

**Date:** 2026-01-11  
**Mission:** Implementation Plan & Technical Decisions  
**Phase:** Decide (Strategy Formulation & Action Planning)

---

## Executive Summary

**Core Decision:** Proceed with workspace-level embedding configuration using provider registry pattern. Implementation split into 4 phases across 44 remaining OODA loops.

**Critical Path:**

1. Workspace schema (blocks everything) → Iterations 07-08
2. Query engine refactor (highest complexity) → Iterations 16-20
3. Vector rebuild logic (highest risk) → Iterations 21-25

**Go/No-Go Decision:** ✅ GO - All technical blockers resolved, implementation path clear.

---

## 1. Strategic Decisions

### Decision 1: Architecture Pattern

**Options Considered:**

| Pattern                          | Pros                         | Cons                               | Decision       |
| -------------------------------- | ---------------------------- | ---------------------------------- | -------------- |
| **Global Provider** (current)    | Simple, fast                 | Cannot mix providers per workspace | ❌ REJECT      |
| **Provider Registry**            | Cached instances, performant | More complexity                    | ✅ **ACCEPT**  |
| **Provider Factory Per Request** | Flexible                     | Expensive, slow                    | ❌ REJECT      |
| **Workspace Service Hybrid**     | Balances concerns            | Medium complexity                  | 🟡 Alternative |

**CHOSEN:** Provider Registry with cached instances

**Rationale:**

- Performance: Creating provider per query = 50-100ms overhead
- Memory: Caching 3-5 providers ~10MB RAM (acceptable)
- Complexity: Manageable with RwLock + HashMap pattern
- Scalability: Works for 1000s of workspaces (all use same 3-5 providers)

### Decision 2: Workspace Lock Mechanism

**Options Considered:**

| Mechanism                    | Pros                    | Cons                | Decision      |
| ---------------------------- | ----------------------- | ------------------- | ------------- |
| **Database Lock (Postgres)** | Survives server restart | Requires Postgres   | ✅ **ACCEPT** |
| **In-Memory Lock (RwLock)**  | Fast                    | Lost on restart     | 🟡 Fallback   |
| **Distributed Lock (Redis)** | Multi-instance          | External dependency | ❌ REJECT     |

**CHOSEN:** Hybrid approach

```rust
// Postgres: UPDATE workspaces SET is_rebuilding = true WHERE id = $1
// In-Memory: HashSet<Uuid> for fast checks (cache DB state)

pub struct WorkspaceLockService {
    rebuilding_cache: RwLock<HashSet<Uuid>>, // Fast check
    db_pool: PgPool,                          // Source of truth
}

impl WorkspaceLockService {
    // Fast path: Check cache first
    pub async fn is_rebuilding(&self, workspace_id: Uuid) -> Result<bool> {
        if self.rebuilding_cache.read().unwrap().contains(&workspace_id) {
            return Ok(true);
        }

        // Slow path: Query database (in case cache stale)
        let is_rebuilding = sqlx::query_scalar!(
            "SELECT is_rebuilding FROM workspaces WHERE id = $1",
            workspace_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(is_rebuilding)
    }
}
```

### Decision 3: Default Embedding Model Strategy

**CHOSEN:** Server-level config with workspace override

```bash
# .env configuration
EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai
EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1536

# Fallback priority:
# 1. Workspace-specific config (from database)
# 2. Server default (from .env)
# 3. Auto-detect (from OPENAI_API_KEY / OLLAMA_HOST)
# 4. Error (require explicit configuration)
```

**Rationale:**

- Flexibility: Workspaces can override server default
- Safety: New workspaces get validated default, not random provider
- Migration: Existing workspaces backfilled with server default
- Error handling: Never fall back to mock in production

### Decision 4: LM Studio Provider Implementation

**CHOSEN:** Dedicated provider with OpenAI-compatible API + native extensions

**File Structure:**

```
edgequake-llm/src/providers/
├── openai.rs           (existing)
├── ollama.rs           (existing)
├── lmstudio.rs         (NEW - ~600 lines)
│   ├── LMStudioProvider struct
│   ├── Native model discovery (GET /v1/models)
│   ├── Health check (GET /health or /v1/models fallback)
│   ├── OpenAI-compatible API for completion/embedding
│   └── LM Studio-specific error handling
└── mod.rs              (add lmstudio export)
```

**Rationale:**

- OpenAI wrapper cannot access LM Studio-specific features
- Model discovery needed for UI dropdown
- Health checks required for provider status indicator
- Estimated 600 lines (same complexity as ollama.rs)

### Decision 5: Database Migration Strategy

**CHOSEN:** Additive migration with backfill

```sql
-- Migration 001: Add columns (NON-BREAKING)
ALTER TABLE workspaces
ADD COLUMN embedding_model VARCHAR(255),
ADD COLUMN embedding_provider VARCHAR(50),
ADD COLUMN embedding_dimension INTEGER,
ADD COLUMN is_rebuilding BOOLEAN DEFAULT false;

-- Migration 002: Backfill defaults from server config
UPDATE workspaces
SET
    embedding_model = COALESCE(
        (SELECT value FROM server_config WHERE key = 'default_embedding_model'),
        'text-embedding-3-small'
    ),
    embedding_provider = 'openai',
    embedding_dimension = 1536
WHERE embedding_model IS NULL;

-- Migration 003: Make columns NOT NULL (after backfill)
ALTER TABLE workspaces
ALTER COLUMN embedding_model SET NOT NULL,
ALTER COLUMN embedding_provider SET NOT NULL,
ALTER COLUMN embedding_dimension SET NOT NULL;

-- Rollback script (if needed)
ALTER TABLE workspaces
DROP COLUMN embedding_model,
DROP COLUMN embedding_provider,
DROP COLUMN embedding_dimension,
DROP COLUMN is_rebuilding;
```

**Safety Measures:**

1. Test on staging database copy
2. Backup before migration
3. Separate rollback script
4. Verify backfill before making NOT NULL

---

## 2. Implementation Plan (Iterations 06-50)

### Phase 1: Foundation (Iterations 06-15) - WEEKS 1-2

#### Iteration 06: Observation & Orientation ✅ COMPLETE

- [x] Gap analysis
- [x] Architecture review
- [x] Design decisions

#### Iteration 07-08: Workspace Schema Migration

**Goals:**

- Database migration scripts (Postgres + in-memory)
- Update workspace service to handle new fields
- Update API endpoints (backwards compatible)

**Files to Modify:**

- `edgequake-core/src/workspace_service.rs` (+50 lines)
- `edgequake-core/src/workspace_service_impl.rs` (+120 lines)
- `edgequake-api/src/handlers/workspaces_types.rs` (+30 lines)
- `edgequake-api/src/handlers/workspaces.rs` (+40 lines)
- `migrations/002_workspace_embeddings.sql` (NEW, 50 lines)

**Tests:**

- Create workspace with custom embedding model
- Create workspace without model (uses server default)
- Update workspace embedding model (should fail if vectors exist)
- List workspaces shows embedding configuration

**Acceptance Criteria:**

- [ ] POST /api/v1/workspaces accepts `embedding_model` field
- [ ] GET /api/v1/workspaces/:id returns `embedding_model`, `embedding_provider`, `embedding_dimension`
- [ ] Existing workspaces backfilled with server default
- [ ] Migration rollback script tested

#### Iteration 09-12: LM Studio Dedicated Provider

**Goals:**

- Create `lmstudio.rs` with native API support
- Model discovery and health checks
- Integration tests with real LM Studio instance

**Files to Create/Modify:**

- `edgequake-llm/src/providers/lmstudio.rs` (NEW, ~600 lines)
- `edgequake-llm/src/providers/mod.rs` (+5 lines)
- `edgequake-llm/src/lib.rs` (+2 lines)
- `edgequake-llm/tests/lmstudio_provider_tests.rs` (NEW, ~300 lines)

**Tests:**

- Connect to LM Studio server
- List available models
- Health check (online/offline detection)
- Chat completion
- Text embedding
- Dimension validation

**Acceptance Criteria:**

- [ ] LM Studio provider passes all trait tests
- [ ] Model discovery returns actual LM Studio models
- [ ] Health check works with and without server running
- [ ] Embedding dimensions auto-detected or configurable

#### Iteration 13-15: Provider Registry Service

**Goals:**

- Create provider registry for caching
- Workspace-to-provider mapping logic
- Integration with AppState

**Files to Create/Modify:**

- `edgequake-llm/src/provider_registry.rs` (NEW, ~250 lines)
- `edgequake-api/src/state.rs` (+80 lines)
- `edgequake-api/src/handlers/providers.rs` (NEW, ~150 lines)
- `edgequake-api/src/routes.rs` (+10 lines)

**Tests:**

- Provider cached on first use
- Cache hit on second use (no re-creation)
- Cache invalidated when provider config changes
- Thread-safe concurrent access

**Acceptance Criteria:**

- [ ] GET /api/v1/providers/available returns all configured providers
- [ ] Provider registry caches instances (verified with metrics)
- [ ] Workspace lookup returns correct provider
- [ ] No memory leaks (test with 1000 provider lookups)

---

### Phase 2: Query Engine Refactor (Iterations 16-25) - WEEKS 3-4

#### Iteration 16-20: Workspace-Aware Query Engine

**Goals:**

- Modify query engine to lookup workspace embedding model
- Create embedding provider per request
- Dimension validation before search

**Files to Modify:**

- `edgequake-query/src/engine.rs` (+150 lines)
- `edgequake-query/src/context.rs` (+30 lines)
- `edgequake-api/src/handlers/query.rs` (+50 lines)
- `edgequake-api/src/middleware.rs` (+20 lines)

**Tests:**

- Query workspace with OpenAI embedding model
- Query workspace with Ollama embedding model
- Query fails if workspace embedding model unavailable
- Dimension mismatch detected and reported

**Acceptance Criteria:**

- [ ] Query uses workspace-specific embedding provider
- [ ] Query fails gracefully if provider unavailable
- [ ] Dimension mismatch error includes expected vs actual
- [ ] Query performance < 5% slower than baseline

#### Iteration 21-25: Vector Database Rebuild Logic

**Goals:**

- API endpoint for triggering rebuild
- Progress tracking and status updates
- Handle Postgres and Memory storage backends

**Files to Create/Modify:**

- `edgequake-api/src/handlers/vector_rebuild.rs` (NEW, ~400 lines)
- `edgequake-api/src/routes.rs` (+5 lines)
- `edgequake-storage/src/adapters/postgres_age/vector.rs` (+80 lines)
- `edgequake-storage/src/adapters/memory/vector.rs` (+50 lines)
- `edgequake-api/src/services/rebuild_service.rs` (NEW, ~300 lines)

**Tests:**

- Rebuild with Postgres storage
- Rebuild with Memory storage
- Rebuild progress tracking
- Concurrent query blocked during rebuild
- Rebuild failure rollback

**Acceptance Criteria:**

- [ ] POST /api/v1/workspaces/:id/rebuild-embeddings triggers rebuild
- [ ] GET /api/v1/workspaces/:id/rebuild-status returns progress
- [ ] Queries blocked with 423 Locked status during rebuild
- [ ] Rebuild completes in < 5 minutes for 1000 documents
- [ ] Rollback works if rebuild fails mid-way

---

### Phase 3: WebUI Integration (Iterations 26-35) - WEEKS 5-6

#### Iteration 26-30: Provider Selector in Query Interface

**Goals:**

- Available providers API endpoint
- Provider dropdown component in query page
- Dynamic provider switching

**Files to Create/Modify:**

- `edgequake_webui/src/components/query/provider-selector.tsx` (NEW, ~300 lines)
- `edgequake_webui/src/app/(dashboard)/query/page.tsx` (+80 lines)
- `edgequake_webui/src/types/provider.ts` (+40 lines)
- `edgequake-api/src/handlers/providers.rs` (+100 lines)

**Tests:**

- Provider dropdown shows all available providers
- Provider selection persisted in localStorage
- Query request includes provider override
- Provider status indicator (connected/disconnected)

**Acceptance Criteria:**

- [ ] Provider dropdown renders in query page
- [ ] Provider list fetched from API on page load
- [ ] Selected provider sent with query request
- [ ] Provider status updates every 30 seconds

#### Iteration 31-35: Workspace Creation Embedding Selector

**Goals:**

- Embedding model selector during workspace creation
- Default model from server config
- Dimension mismatch warnings

**Files to Create/Modify:**

- `edgequake_webui/src/components/workspace/embedding-selector.tsx` (NEW, ~250 lines)
- `edgequake_webui/src/app/(dashboard)/workspaces/create/page.tsx` (+100 lines)
- `edgequake_webui/src/types/workspace.ts` (+20 lines)

**Tests:**

- Workspace creation with default embedding model
- Workspace creation with custom embedding model
- Warning shown if selecting model with different dimension
- Model list fetched from available providers API

**Acceptance Criteria:**

- [ ] Embedding selector shown in workspace creation form
- [ ] Default model pre-selected from server config
- [ ] Dimension warning shown if changing from default
- [ ] Created workspace reflects selected embedding model

---

### Phase 4: Testing & Documentation (Iterations 36-50) - WEEKS 7-8

#### Iteration 36-40: Edge Cases & Error Handling

**Test Scenarios:**

1. Empty vector database (no documents ingested)
2. Concurrent queries during rebuild
3. Provider unavailable mid-query
4. Dimension mismatch (query embedding vs stored)
5. Network timeout during rebuild
6. Server crash mid-rebuild (recovery)
7. Workspace deletion during rebuild
8. Invalid embedding model name

**Files to Create:**

- `edgequake-api/tests/e2e_edge_cases.rs` (NEW, ~600 lines)
- `edgequake-query/tests/error_handling.rs` (NEW, ~400 lines)

#### Iteration 41-45: Storage Backend Compatibility Testing

**Test Matrix:**

| Test Case                    | Postgres | Memory | Expected Result           |
| ---------------------------- | -------- | ------ | ------------------------- |
| Create workspace with OpenAI | ✅       | ✅     | Success                   |
| Create workspace with Ollama | ✅       | ✅     | Success                   |
| Query OpenAI workspace       | ✅       | ✅     | Correct results           |
| Query Ollama workspace       | ✅       | ✅     | Correct results           |
| Rebuild OpenAI → Ollama      | ✅       | ✅     | Vectors cleared + rebuilt |
| Rebuild with 1000 docs       | ✅       | ✅     | < 5 min completion        |
| Concurrent queries blocked   | ✅       | ✅     | 423 Locked status         |

**Files to Create:**

- `edgequake-api/tests/e2e_storage_backends.rs` (NEW, ~800 lines)

#### Iteration 46-48: Documentation & Setup Guides

**Documentation Files:**

1. `docs/providers/ollama-setup.md` (NEW, ~200 lines)
2. `docs/providers/lmstudio-setup.md` (NEW, ~200 lines)
3. `docs/providers/openai-setup.md` (UPDATE, +50 lines)
4. `docs/architecture/workspace-embeddings.md` (NEW, ~300 lines)
5. `docs/api/v2-migration-guide.md` (NEW, ~250 lines)
6. `README.md` (UPDATE, +100 lines)

**Setup Guide Contents:**

- Prerequisites (software versions)
- Installation steps
- Configuration (environment variables)
- Model selection guidelines
- Troubleshooting common issues
- Performance tuning
- Cost optimization (for cloud providers)

#### Iteration 49-50: Final Non-Regression Validation

**Full Test Suite:**

1. Run all unit tests (cargo test)
2. Run all E2E tests (cargo test --test e2e\_\*)
3. Run WebUI tests (npm test)
4. Manual smoke tests
5. Performance benchmarks
6. Memory leak detection
7. API compatibility check

**Acceptance Criteria:**

- [ ] All tests passing (0 failures)
- [ ] No clippy warnings
- [ ] No memory leaks (valgrind or similar)
- [ ] Query latency < 5% increase
- [ ] Documentation complete
- [ ] Migration guide tested

---

## 3. Technical Specifications

### API Endpoints (New)

```
POST   /api/v1/workspaces                     (UPDATE: accepts embedding_model)
GET    /api/v1/workspaces/:id                 (UPDATE: returns embedding config)
POST   /api/v1/workspaces/:id/rebuild         (NEW: trigger vector rebuild)
GET    /api/v1/workspaces/:id/rebuild-status  (NEW: rebuild progress)
GET    /api/v1/providers/available            (NEW: list providers + models)
GET    /api/v1/providers/:name/health         (NEW: health check)
```

### Database Schema Changes

```sql
-- workspaces table (add columns)
ALTER TABLE workspaces ADD COLUMN embedding_model VARCHAR(255) NOT NULL;
ALTER TABLE workspaces ADD COLUMN embedding_provider VARCHAR(50) NOT NULL;
ALTER TABLE workspaces ADD COLUMN embedding_dimension INTEGER NOT NULL;
ALTER TABLE workspaces ADD COLUMN is_rebuilding BOOLEAN DEFAULT false;

-- server_config table (NEW)
CREATE TABLE server_config (
    key VARCHAR(255) PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT NOW()
);

INSERT INTO server_config (key, value) VALUES
    ('default_embedding_model', 'text-embedding-3-small'),
    ('default_embedding_provider', 'openai'),
    ('default_embedding_dimension', '1536');
```

### Environment Variables (New)

```bash
# Server defaults
EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai
EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1536

# LM Studio configuration
LMSTUDIO_HOST=http://localhost:1234
LMSTUDIO_MODEL=gemma2-9b-it
LMSTUDIO_EMBEDDING_MODEL=text-embedding-ada-002
LMSTUDIO_EMBEDDING_DIM=1536

# Feature flags
EDGEQUAKE_ENABLE_PROVIDER_SWITCHING=true
EDGEQUAKE_ENABLE_VECTOR_REBUILD=true
```

---

## 4. Risk Management

### High-Risk Items

| Risk                                  | Impact      | Probability | Mitigation                                     |
| ------------------------------------- | ----------- | ----------- | ---------------------------------------------- |
| **Breaking API Changes**              | 🔴 HIGH     | Medium      | Versioned endpoints, deprecation warnings      |
| **Data Loss During Rebuild**          | 🔴 CRITICAL | Low         | Transactional rebuild, backup before operation |
| **Query Performance Degradation**     | 🟡 MEDIUM   | Medium      | Benchmark before/after, provider caching       |
| **Memory Leaks in Provider Registry** | 🟡 MEDIUM   | Low         | Weak references, cache eviction policy         |
| **Database Migration Failure**        | 🔴 CRITICAL | Low         | Test on staging, rollback script ready         |

### Mitigation Strategies

**API Compatibility:**

```rust
// Keep v1 endpoints working
#[deprecated(since = "2.0.0", note = "Use /api/v2/workspaces instead")]
pub async fn create_workspace_v1(...) -> Result<...> {
    // Delegate to v2 with default embedding model
    create_workspace_v2(...).await
}
```

**Rebuild Safety:**

```rust
// Transactional rebuild with rollback
let transaction = pool.begin().await?;

match rebuild_vectors(&transaction, workspace_id).await {
    Ok(_) => transaction.commit().await?,
    Err(e) => {
        transaction.rollback().await?;
        return Err(e);
    }
}
```

---

## 5. Success Metrics

### Code Quality

- [ ] Test coverage ≥ 80% for new code
- [ ] Zero clippy warnings
- [ ] All rustdoc comments present
- [ ] No unsafe code (except in battle-tested dependencies)

### Performance

- [ ] Query latency P95 < baseline + 50ms
- [ ] Provider lookup < 10ms (cached)
- [ ] Memory usage < baseline + 50MB
- [ ] Rebuild throughput ≥ 200 docs/minute

### Functionality

- [ ] Provider switching works without server restart
- [ ] Vector rebuild completes successfully (Postgres + Memory)
- [ ] Dimension mismatches detected before query
- [ ] Concurrent queries blocked during rebuild
- [ ] WebUI provider selector renders correctly
- [ ] Workspace creation includes embedding selector

---

## 6. Rollback Plan

### If Critical Issues Found After Deployment

**Step 1:** Feature flags to disable new functionality

```bash
EDGEQUAKE_ENABLE_PROVIDER_SWITCHING=false
EDGEQUAKE_ENABLE_VECTOR_REBUILD=false
```

**Step 2:** Database rollback script

```bash
psql -d edgequake -f migrations/rollback_002_workspace_embeddings.sql
```

**Step 3:** Revert to previous binary

```bash
git checkout v1.9.0
cargo build --release
systemctl restart edgequake-api
```

**Step 4:** Clear corrupted data (if needed)

```sql
DELETE FROM workspaces WHERE embedding_model IS NOT NULL;
DELETE FROM server_config WHERE key LIKE 'default_embedding_%';
```

---

## 7. Open Questions (To Resolve During Implementation)

### Question 1: LM Studio Model Names

**Issue:** Spec says `gemma-3n-e4b-it-mlxmodel` but this may not be actual LM Studio model name.

**Resolution Plan:**

- Install LM Studio locally
- Query GET /v1/models endpoint
- Document actual model names
- Update spec and defaults

### Question 2: Provider Registry Cache Eviction

**Issue:** If server has 100 providers configured, cache may grow unbounded.

**Resolution Plan:**

- Implement LRU cache with max size (default: 20 providers)
- Add cache metrics (hit rate, evictions)
- Monitor memory usage in production

### Question 3: Postgres vs Memory Behavior Differences

**Issue:** Some edge cases may behave differently between storage backends.

**Resolution Plan:**

- Create comparison test suite
- Document known differences
- Add storage-specific handling if needed

---

## 8. Next Actions (Act Phase)

### Iteration 07 Immediate Tasks

1. **Workspace Schema Design:**

   - [ ] Write SQL migration script
   - [ ] Write rollback script
   - [ ] Test on local Postgres instance
   - [ ] Verify backfill logic

2. **Workspace Service Updates:**

   - [ ] Add embedding_model field to CreateWorkspaceRequest
   - [ ] Update workspace_service trait
   - [ ] Implement for both Memory and Postgres

3. **API Endpoint Updates:**

   - [ ] Modify POST /api/v1/workspaces handler
   - [ ] Update WorkspaceResponse DTO
   - [ ] Add API documentation

4. **Tests:**
   - [ ] Unit tests for workspace service
   - [ ] Integration tests for API endpoints
   - [ ] Migration tests

**Estimated Time:** 8-12 hours
**Complexity:** Medium (database changes always risky)
**Blockers:** None (all dependencies resolved)

---

## 9. Conclusion

**Decision:** ✅ PROCEED with implementation plan

**Confidence Level:** HIGH (85%)

**Key Strengths:**

- Clear technical path identified
- All major risks have mitigation strategies
- Incremental implementation minimizes blast radius
- Rollback plan available at each phase

**Key Risks:**

- Database migration always risky (mitigated with testing + backups)
- Query performance may degrade (mitigated with benchmarking + caching)
- WebUI integration complexity (mitigated with feature flags)

**Timeline:** 8 weeks for full implementation + testing + documentation

---

**Commit Message for Iteration 06 Decide:**

```
docs(ooda-06): Implementation plan for workspace-level embeddings

Strategic decisions:
- Provider registry with caching (performance + scalability)
- Hybrid workspace locking (DB source of truth + memory cache)
- Server-level defaults with workspace override
- Dedicated LM Studio provider (not OpenAI wrapper)
- Additive database migration with backfill

Implementation phases:
- Phase 1 (07-15): Schema + LM Studio + Provider Registry
- Phase 2 (16-25): Query Engine + Vector Rebuild
- Phase 3 (26-35): WebUI Provider Selector + Workspace Creation
- Phase 4 (36-50): Testing + Documentation + Validation

Risk mitigation:
- Versioned API endpoints (backwards compatible)
- Transactional vector rebuild (rollback on failure)
- Feature flags for gradual rollout
- Comprehensive rollback procedures

Success metrics:
- Query latency < baseline + 50ms
- Test coverage ≥ 80%
- Zero clippy warnings
- Rebuild throughput ≥ 200 docs/min

Next: Act phase (iteration 07) - Workspace schema implementation
```
