# Advanced Tenant Isolation Security Implementation Plan

**Status**: 🚧 IN PROGRESS
**Created**: 2024-12-29
**Goal**: Achieve SOTA (State of the Art) multi-tenant security

---

## Executive Summary

This plan builds upon the existing tenant isolation system (12 tests passing, PRODUCTION READY status) to implement:
1. Database-level tenant indexes for optimal performance
2. Tenant-based rate limiting middleware
3. Comprehensive audit logging for security monitoring
4. Real PostgreSQL tests with Row-Level Security (RLS) validation

---

## Phase 1: Database-Level Tenant Indexes

### Objective
Optimize query performance by adding database indexes on tenant_id and workspace_id fields across all tables.

### Implementation Tasks

#### 1.1 PostgreSQL Migration - Tenant Indexes
```sql
-- Create indexes for vector storage
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_tenant_workspace 
ON chunks(tenant_id, workspace_id) 
WHERE tenant_id IS NOT NULL;

-- Create indexes for graph storage
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_tenant_workspace 
ON entities(properties->>'tenant_id', properties->>'workspace_id');

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edges_tenant_workspace 
ON edges(properties->>'tenant_id', properties->>'workspace_id');

-- Create indexes for document metadata
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_documents_tenant_workspace 
ON documents(tenant_id, workspace_id);

-- Performance optimization: BRIN index for time-series queries
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_created_at_brin 
ON chunks USING BRIN(created_at, tenant_id);
```

#### 1.2 Storage Layer Updates
- Update `PostgresStorage` trait implementations to leverage indexes
- Add query hints for PostgreSQL planner optimization
- Implement index monitoring and statistics collection

### Test Scenarios

1. **Performance Baseline Test**
   - Measure query latency BEFORE index creation
   - Insert 10,000 documents across 100 tenants
   - Measure query latency AFTER index creation
   - **Expected**: >50% latency reduction

2. **Index Coverage Test**
   - Verify all tenant-filtered queries use indexes
   - Use EXPLAIN ANALYZE to confirm index usage
   - **Expected**: All queries show "Index Scan" not "Seq Scan"

3. **Concurrent Load Test**
   - 100 concurrent queries across different tenants
   - Measure throughput and p99 latency
   - **Expected**: Linear scaling with tenant count

---

## Phase 2: Tenant-Based Rate Limiting Middleware

### Objective
Prevent resource exhaustion attacks and ensure fair resource allocation across tenants.

### Implementation Architecture

```rust
// Rate limiter per tenant/workspace
pub struct TenantRateLimiter {
    // Token bucket algorithm per tenant
    tenant_buckets: DashMap<String, RateLimiterBucket>,
    workspace_buckets: DashMap<String, RateLimiterBucket>,
    
    // Configuration
    config: RateLimitConfig,
}

pub struct RateLimitConfig {
    // Per tenant limits
    max_requests_per_minute_per_tenant: u32,
    max_documents_per_hour_per_tenant: u32,
    max_queries_per_minute_per_tenant: u32,
    
    // Per workspace limits
    max_requests_per_minute_per_workspace: u32,
    
    // Burst allowance
    burst_size: u32,
}
```

### Implementation Tasks

#### 2.1 Create Rate Limiting Crate
- New crate: `edgequake-rate-limiter`
- Token bucket algorithm implementation
- DashMap-based concurrent access
- Sliding window for accurate rate limiting

#### 2.2 Axum Middleware Integration
```rust
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<TenantRateLimiter>>,
    tenant_ctx: Option<TenantContext>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let ctx = tenant_ctx.ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Check tenant rate limit
    if !limiter.check_tenant_limit(&ctx.tenant_id).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    // Check workspace rate limit
    if let Some(workspace_id) = ctx.workspace_id {
        if !limiter.check_workspace_limit(&workspace_id).await {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }
    
    Ok(next.run(req).await)
}
```

#### 2.3 Rate Limit Response Headers
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 856
X-RateLimit-Reset: 1735473600
Retry-After: 45
```

### Test Scenarios

1. **Basic Rate Limiting Test**
   - Send 1000 requests from Tenant A
   - **Expected**: First 1000 pass, rest return 429 TOO_MANY_REQUESTS

2. **Multi-Tenant Fairness Test**
   - Tenant A: 10,000 requests/min
   - Tenant B: 100 requests/min
   - **Expected**: Tenant B not affected by Tenant A's load

3. **Burst Handling Test**
   - Send 100 requests instantly (burst)
   - **Expected**: First 50 pass, rest rate-limited

4. **Rate Limit Reset Test**
   - Hit rate limit, wait for reset window
   - **Expected**: Requests succeed after window expires

5. **Workspace Isolation Test**
   - Workspace 1: Hit rate limit
   - Workspace 2: Same tenant
   - **Expected**: Workspace 2 unaffected

---

## Phase 3: Comprehensive Audit Logging

### Objective
Track all security-relevant events for compliance, forensics, and threat detection.

### Log Event Categories

1. **Authentication Events**
   - Tenant/Workspace context extraction
   - Missing or invalid tenant headers
   - Authorization failures

2. **Data Access Events**
   - Document uploads (with content hash)
   - Document retrievals (with filters applied)
   - Query executions (with tenant context)
   - Graph traversals

3. **Administrative Events**
   - Tenant creation/deletion
   - Workspace creation/deletion
   - Configuration changes

4. **Security Events**
   - Cross-tenant access attempts (blocked)
   - Rate limit violations
   - SQL injection attempts
   - Unusual query patterns

### Implementation Architecture

```rust
pub struct AuditLogger {
    // Async log writer (non-blocking)
    log_queue: tokio::sync::mpsc::UnboundedSender<AuditEvent>,
    
    // Structured logging backend
    backend: Arc<dyn AuditBackend>,
}

pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub result: AuditResult,
    pub metadata: serde_json::Value,
}

pub enum AuditEventType {
    Authentication,
    DataAccess,
    Administrative,
    Security,
}

pub enum AuditResult {
    Success,
    Failure(String),
    Blocked(String),
}
```

### Audit Log Storage

**Option 1: PostgreSQL Audit Table**
```sql
CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    tenant_id UUID,
    workspace_id UUID,
    user_id UUID,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    action VARCHAR(100) NOT NULL,
    result VARCHAR(50) NOT NULL,
    metadata JSONB,
    ip_address INET,
    user_agent TEXT,
    
    -- Indexes for fast querying
    INDEX idx_audit_tenant_timestamp (tenant_id, timestamp),
    INDEX idx_audit_event_type_timestamp (event_type, timestamp),
    INDEX idx_audit_result_timestamp (result, timestamp)
);

-- Partitioning by month for performance
CREATE TABLE audit_logs_2024_12 PARTITION OF audit_logs
FOR VALUES FROM ('2024-12-01') TO ('2025-01-01');
```

**Option 2: Write-Ahead Log (WAL) + S3**
- High-throughput async writes
- Archive to S3 for long-term retention
- Elasticsearch for search and analytics

### Test Scenarios

1. **Authentication Audit Test**
   - Upload document with tenant headers
   - **Expected**: Audit log entry with tenant_id, action="upload"

2. **Cross-Tenant Access Audit Test**
   - Attempt to access Tenant B document with Tenant A headers
   - **Expected**: Audit log entry with result=Blocked("cross_tenant_access")

3. **Security Event Detection Test**
   - Send SQL injection in tenant header
   - **Expected**: Audit log entry with event_type=Security, action="sql_injection_attempt"

4. **Audit Log Query Performance Test**
   - Insert 1 million audit entries
   - Query last 24 hours for specific tenant
   - **Expected**: Query completes <100ms

5. **Audit Log Retention Test**
   - Verify logs older than 90 days are archived
   - **Expected**: Active table contains only recent logs

---

## Phase 4: Real PostgreSQL Testing with RLS

### Objective
Validate Row-Level Security (RLS) policies enforce tenant isolation at the database level.

### PostgreSQL RLS Setup

```sql
-- Enable RLS on all tenant-aware tables
ALTER TABLE chunks ENABLE ROW LEVEL SECURITY;
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
ALTER TABLE edges ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

-- Create RLS policies for tenant isolation
CREATE POLICY tenant_isolation_policy ON chunks
FOR ALL
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

CREATE POLICY tenant_isolation_policy ON entities
FOR ALL
USING ((properties->>'tenant_id')::UUID = current_setting('app.current_tenant_id')::UUID);

CREATE POLICY tenant_isolation_policy ON edges
FOR ALL
USING ((properties->>'tenant_id')::UUID = current_setting('app.current_tenant_id')::UUID);

CREATE POLICY tenant_isolation_policy ON documents
FOR ALL
USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

-- Workspace-level policies (optional, more granular)
CREATE POLICY workspace_isolation_policy ON chunks
FOR ALL
USING (
    workspace_id = current_setting('app.current_workspace_id')::UUID
    AND tenant_id = current_setting('app.current_tenant_id')::UUID
);
```

### RLS Context Management

```rust
// Set session variables before each query
impl PostgresStorage {
    async fn set_tenant_context(
        &self,
        conn: &mut sqlx::PgConnection,
        tenant_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query("SET LOCAL app.current_tenant_id = $1")
            .bind(tenant_id)
            .execute(&mut *conn)
            .await?;
            
        if let Some(ws_id) = workspace_id {
            sqlx::query("SET LOCAL app.current_workspace_id = $1")
                .bind(ws_id)
                .execute(&mut *conn)
                .await?;
        }
        
        Ok(())
    }
}
```

### Test Infrastructure

#### 4.1 Docker Compose for Test PostgreSQL
```yaml
version: '3.8'
services:
  postgres-test:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: edgequake_test
      POSTGRES_PASSWORD: test_password
      POSTGRES_DB: edgequake_test
    ports:
      - "5433:5432"
    volumes:
      - ./migrations:/docker-entrypoint-initdb.d
    tmpfs:
      - /var/lib/postgresql/data  # In-memory for speed
```

#### 4.2 Test Fixtures
```rust
#[tokio::test]
async fn test_rls_prevents_cross_tenant_access() {
    let pool = setup_test_database().await;
    
    // Insert data as Tenant A
    let tenant_a_id = "tenant-a-uuid";
    set_tenant_context(&pool, tenant_a_id).await;
    insert_test_document(&pool, "doc-a-id").await;
    
    // Try to read as Tenant B
    let tenant_b_id = "tenant-b-uuid";
    set_tenant_context(&pool, tenant_b_id).await;
    let result = get_document(&pool, "doc-a-id").await;
    
    // RLS should prevent access
    assert!(result.is_none(), "RLS failed to block cross-tenant access");
}
```

### Test Scenarios

1. **RLS Basic Isolation Test**
   - Insert 100 documents for Tenant A
   - Set context to Tenant B
   - Query all documents
   - **Expected**: 0 results

2. **RLS Performance Test**
   - Insert 10,000 documents across 10 tenants
   - Measure query latency with RLS enabled
   - **Expected**: <10% overhead vs. no RLS

3. **RLS Bypass Attempt Test**
   - Try to set tenant context via SQL injection
   - **Expected**: Context not changed, injection blocked

4. **RLS + Application Filter Consistency Test**
   - Query with both RLS and app-level filters
   - **Expected**: Results identical

5. **RLS Concurrent Context Test**
   - 100 concurrent connections with different tenant contexts
   - **Expected**: No context bleed between connections

---

## Phase 5: Advanced E2E Test Scenarios

### Objective
Find edge cases and attack vectors that could compromise tenant isolation.

### Test Categories

#### 5.1 Concurrency & Race Conditions

**Scenario 1: Concurrent Document Upload**
```
- Tenant A: Upload 100 documents simultaneously
- Tenant B: Upload 100 documents simultaneously
- Query: Each tenant retrieves their documents
- Expected: Each tenant sees exactly 100 documents (their own)
```

**Scenario 2: Context Switching Under Load**
```
- 1000 requests alternating between Tenant A and Tenant B
- Verify no request sees wrong tenant's data
- Expected: 0% context bleed
```

#### 5.2 Attack Vectors

**Scenario 3: Header Injection Attack**
```
X-Tenant-ID: tenant-a'; DROP TABLE chunks; --
Expected: Request rejected, no SQL execution
```

**Scenario 4: Unicode Homograph Attack**
```
X-Tenant-ID: tenant-а (Cyrillic 'a')
Expected: Treated as different from tenant-a (Latin 'a')
```

**Scenario 5: Timing Attack**
```
- Query non-existent document in Tenant A
- Query non-existent document in Tenant B
- Measure response times
- Expected: Response times statistically identical (no info leak)
```

**Scenario 6: Resource Exhaustion Attack**
```
- Tenant A: Create 1 million entities
- Tenant B: Query with complex graph traversal
- Expected: Tenant B query not affected by Tenant A's data volume
```

#### 5.3 Data Integrity

**Scenario 7: Batch Processing Isolation**
```
- Batch upload 1000 documents with mixed tenant headers
- Expected: Each document tagged with correct tenant_id
```

**Scenario 8: Async Task Queue Isolation**
```
- Tenant A: Queue 100 async tasks
- Tenant B: Queue 100 async tasks
- Expected: No task processes wrong tenant's data
```

#### 5.4 Edge Cases

**Scenario 9: Missing Tenant Context**
```
- Send request without X-Tenant-ID header
- Expected: 401 Unauthorized (not 500 error)
```

**Scenario 10: Malformed UUID**
```
X-Tenant-ID: not-a-valid-uuid
Expected: 400 Bad Request with clear error message
```

**Scenario 11: Workspace Without Tenant**
```
X-Workspace-ID: workspace-123
(no X-Tenant-ID header)
Expected: 400 Bad Request (tenant required)
```

**Scenario 12: Very Long Tenant ID**
```
X-Tenant-ID: <10MB string>
Expected: Request rejected before processing
```

---

## Phase 6: Performance Optimization

### Query Performance Targets

| Query Type                  | Target Latency | Max Latency |
| --------------------------- | -------------- | ----------- |
| Document list (filtered)    | <50ms          | <100ms      |
| Single document retrieval   | <10ms          | <25ms       |
| Graph query (1-hop)         | <100ms         | <200ms      |
| Graph query (2-hop)         | <500ms         | <1000ms     |
| Hybrid query (vector+graph) | <200ms         | <500ms      |

### Optimization Strategies

1. **Prepared Statements**
   - Pre-compile tenant-filtered queries
   - Reduce PostgreSQL planning overhead

2. **Connection Pooling**
   - Per-tenant connection pools
   - Prevent connection exhaustion

3. **Caching Layer**
   - Redis cache for frequently accessed documents
   - Cache key includes tenant_id for isolation
   - TTL: 5 minutes

4. **Query Result Pagination**
   - Limit result set size
   - Cursor-based pagination for large datasets

---

## Phase 7: Compliance & Reporting

### Compliance Requirements

1. **GDPR Data Isolation**
   - Tenant data completely isolated
   - Ability to export/delete tenant data
   - Audit trail of data access

2. **SOC 2 Type II**
   - Access controls enforced
   - Change management tracked
   - Security monitoring active

3. **HIPAA (if applicable)**
   - Encryption at rest and in transit
   - Audit logging of PHI access
   - Access control enforcement

### Compliance Test Scenarios

**Scenario 13: Data Portability Test**
```
- Export all data for Tenant A
- Verify completeness (documents, entities, relationships)
- Verify no Tenant B data in export
- Expected: Complete, isolated export
```

**Scenario 14: Right to Deletion Test**
```
- Delete Tenant A
- Verify all Tenant A data deleted from all tables
- Verify Tenant B data unaffected
- Expected: Complete deletion, no cascade to other tenants
```

---

## Success Criteria for SOTA Declaration

### Functional Requirements ✅
- [x] Basic tenant isolation (12 tests passing)
- [ ] Database-level indexes implemented
- [ ] Rate limiting active and tested
- [ ] Audit logging comprehensive
- [ ] RLS policies enforced

### Performance Requirements
- [ ] Query latency within targets (>95% of queries)
- [ ] Throughput: 1000 requests/sec per tenant
- [ ] P99 latency: <500ms for all query types

### Security Requirements
- [ ] 0 cross-tenant data leaks in 1M test queries
- [ ] All attack vectors blocked and logged
- [ ] Timing attacks mitigated (statistical analysis)

### Scalability Requirements
- [ ] System scales to 1000 tenants
- [ ] Linear performance degradation (<10% per 100 tenants)
- [ ] Database size: Handles 1TB+ per tenant

### Reliability Requirements
- [ ] 99.9% uptime target
- [ ] Graceful degradation under load
- [ ] Zero data loss scenarios

---

## Implementation Timeline

### Week 1: Database & Rate Limiting
- Day 1-2: Implement database indexes + migration
- Day 3-4: Create rate limiting middleware
- Day 5: Test and fix issues

### Week 2: Audit Logging & PostgreSQL Tests
- Day 1-2: Implement audit logging system
- Day 3-4: Set up PostgreSQL test environment with RLS
- Day 5: Integration testing

### Week 3: Advanced Testing & Hardening
- Day 1-3: Implement all 14 E2E test scenarios
- Day 4: Security audit and penetration testing
- Day 5: Performance optimization

### Week 4: Documentation & SOTA Declaration
- Day 1-2: Complete documentation
- Day 3-4: Final review and fixes
- Day 5: SOTA declaration with evidence

---

## Risk Mitigation

### Risk 1: Performance Regression
**Mitigation**: Benchmark before/after, rollback if >20% degradation

### Risk 2: RLS Overhead
**Mitigation**: Use CONCURRENTLY for index creation, test on production-like data

### Risk 3: Rate Limiting False Positives
**Mitigation**: Generous burst allowance, clear error messages, monitoring

### Risk 4: Audit Log Storage Growth
**Mitigation**: Partitioning, archival strategy, compression

---

## Next Steps

1. ✅ Create this comprehensive test plan
2. ⏳ Begin implementation of Phase 1 (Database Indexes)
3. ⏳ Set up continuous testing infrastructure
4. ⏳ Document all findings in scratchpad.md
5. ⏳ Achieve SOTA status with evidence

**Status**: Ready to begin implementation. Starting with Phase 1...
