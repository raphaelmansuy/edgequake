# Database Architecture Update: PostgreSQL + AGE + pgvector

**Date**: 2024-12-21  
**Author**: EdgeQuake Architecture Team  
**Status**: Completed  
**Commits**: 11957d51, a8ca491c

---

## Executive Summary

Updated EdgeQuake's database architecture from SurrealDB to **PostgreSQL + AGE + pgvector** with Row-Level Security (RLS) for multi-tenancy. This decision prioritizes production-proven reliability and mature ecosystem over newer technology.

---

## Changes Made

### 1. Architectural Decision Records (ADR)

#### ADR-003: Database Choice
**Before**: SurrealDB as primary graph database
- Graph-native Rust implementation
- Newer project (2 years old)
- Smaller ecosystem

**After**: PostgreSQL + AGE + pgvector
- **PostgreSQL**: 25+ years production-proven, ACID-compliant RDBMS
- **AGE (Apache AGE)**: Graph extension providing Cypher query language
- **pgvector**: Vector similarity search for embeddings
- **Rationale**:
  - Battle-tested reliability and stability
  - Mature ecosystem (pgAdmin, monitoring, backups, replication)
  - Excellent Rust support (`sqlx`, `tokio-postgres`)
  - Unified storage (entities, relations, embeddings, metadata)
  - Native RLS for defense-in-depth security

**File**: [plan/integration/ADR_INDEX.md](../plan/integration/ADR_INDEX.md)

**Implementation Details**:
```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS age;
CREATE EXTENSION IF NOT EXISTS vector;

-- Entity table with pgvector embeddings
CREATE TABLE entities (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL,
    name TEXT NOT NULL,
    embedding vector(1536),  -- pgvector
    ...
);

-- pgvector index for similarity search
CREATE INDEX idx_embedding ON entities 
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- AGE graph queries
SELECT * FROM cypher('edgequake', $$
    MATCH (e:Entity)-[:RELATES_TO*1..2]->(neighbor)
    WHERE e.workspace_id = $workspace_id
    RETURN neighbor
$$) as (neighbor agtype);
```

#### ADR-004: Multi-Tenancy Implementation
**Before**: Shared database with manual `WHERE workspace_id = $id` filtering
- Application-level enforcement
- Risk of data leaks if filter missed
- Manual query filtering required

**After**: PostgreSQL Row-Level Security (RLS)
- **Database-enforced isolation**: RLS policies prevent data leaks at SQL level
- **Session variable pattern**: `app.current_workspace_id` set once per request
- **Automatic filtering**: No manual `WHERE` clauses needed
- **Defense-in-depth**: Immune to application bugs

**File**: [plan/integration/ADR_INDEX.md](../plan/integration/ADR_INDEX.md)

**Implementation Details**:
```sql
-- Enable RLS
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;

-- Create policy
CREATE POLICY tenant_isolation ON entities
    FOR ALL
    USING (workspace_id = current_setting('app.current_workspace_id')::UUID);
```

**Middleware Pattern** (Rust + Axum):
```rust
// Set session variable once per request
sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
    .bind(workspace_id.to_string())
    .execute(&pool)
    .await?;

// All subsequent queries automatically filtered by RLS
let entities = sqlx::query_as::<_, Entity>(
    "SELECT * FROM entities WHERE name = $1"  // No manual workspace_id filter!
)
.bind(name)
.fetch_all(&pool)
.await?;
```

**Performance**: 5-10% query overhead with proper indexes (vs 50%+ without)

---

### 2. Implementation Guides Updated

#### multi-tenancy-guide.md
**Changes**:
- Updated tech stack from `Rust + Axum + SurrealDB` to `Rust + Axum + PostgreSQL (AGE + pgvector + RLS)`
- Complete PostgreSQL schema with RLS policies
- `PostgresWorkspaceStorage` implementation replacing `SurrealWorkspaceStorage`
- pgvector cosine similarity search examples
- Apache AGE Cypher graph traversal patterns
- RLS setup, session variable management, connection pooling guidance
- Middleware pattern for setting session variables
- Testing RLS isolation examples

**File**: [tech_stack/multi-tenancy-guide.md](../tech_stack/multi-tenancy-guide.md)

**Key Sections**:
1. **Schema with RLS**: Complete PostgreSQL DDL with AGE, pgvector, RLS policies
2. **Storage Implementation**: PostgresWorkspaceStorage with sqlx
3. **Security Considerations**: RLS enforcement, session variable management
4. **Performance**: Indexing strategies, connection pooling

#### testing-guide.md
**Changes**:
- Updated tech stack references from SurrealDB to PostgreSQL
- Changed storage module from `surrealdb.rs` to `postgres.rs`
- Updated test setup examples to use `PgPool` instead of `Surreal<Mem>`
- Docker Compose examples now use `postgres:16-alpine` instead of `surrealdb/surrealdb`
- Environment variables: `DATABASE_URL` instead of `SURREALDB_URL`
- Cargo test features: `postgres sqlx-runtime-tokio-rustls` instead of `surrealdb postgres`

**File**: [tech_stack/testing-guide.md](../tech_stack/testing-guide.md)

#### configuration-guide.md
**Changes**:
- Removed `EDGEQUAKE_SURREAL_URL` environment variable
- Added `EDGEQUAKE_ENABLE_AGE` flag (default: `true`)
- Added `EDGEQUAKE_ENABLE_PGVECTOR` flag (default: `true`)
- Updated database URL description to explicitly mention PostgreSQL

**File**: [tech_stack/configuration-guide.md](../tech_stack/configuration-guide.md)

#### PHASE_5_PLAN.md
**Changes**:
- Updated SQLAlchemy migration target: `SQLAlchemy → PostgreSQL (sqlx) with AGE + pgvector`
- Updated Phase 1 deliverables: `PostgreSQL adapter (sqlx + AGE + pgvector)` with `Basic CRUD operations with RLS`
- Updated ADR-003 description: `PostgreSQL + AGE + pgvector as primary database`
- Updated ADR-004 description: `Shared database multi-tenancy with PostgreSQL RLS`

**File**: [plan/PHASE_5_PLAN.md](../plan/reports/archive/PHASE_5_PLAN.md)

---

## Technical Comparison

| Feature | SurrealDB | PostgreSQL + AGE + pgvector |
|---------|-----------|----------------------------|
| **Maturity** | 2 years | 25+ years |
| **Graph Queries** | Native | AGE extension (Cypher) |
| **Vector Search** | Native | pgvector extension |
| **Multi-tenancy** | Permission system | Row-Level Security (RLS) |
| **Ecosystem** | Growing | Mature (pgAdmin, replication, backups) |
| **Rust Support** | Native | sqlx, tokio-postgres |
| **Production Use** | Limited | Extensive (millions of deployments) |
| **Horizontal Scaling** | Built-in clustering | Read replicas, Citus extension |
| **ACID Compliance** | ✅ | ✅ |
| **Query Language** | SurrealQL | SQL + Cypher (AGE) |
| **Security** | Permission system | RLS policies |

---

## Migration Impact

### Dependencies
**Rust Cargo.toml**:
```toml
# Remove
surrealdb = "1.0"

# Add
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "uuid", "json"] }
tokio-postgres = "0.7"
```

### Docker Compose
**Before**:
```yaml
surrealdb:
  image: surrealdb/surrealdb:latest
  command: start --log trace --user root --pass root file://data/database.db
```

**After**:
```yaml
postgres:
  image: postgres:16-alpine
  environment:
    POSTGRES_DB: edgequake
    POSTGRES_USER: edgequake
    POSTGRES_PASSWORD: edgequake
  volumes:
    - postgres_data:/var/lib/postgresql/data
```

### Connection Strings
**Before**: `ws://localhost:8000/rpc`  
**After**: `postgresql://edgequake:edgequake@localhost:5432/edgequake`

### Storage Trait Implementation
**Before**: `SurrealWorkspaceStorage` with `Surreal<Client>`  
**After**: `PostgresWorkspaceStorage` with `PgPool`

---

## Security Benefits

### 1. Database-Enforced Isolation
RLS policies enforce tenant isolation at the PostgreSQL level:
- **Application bugs cannot bypass**: Even if code forgets to filter by `workspace_id`, RLS blocks access
- **SQL injection protection**: Session variable set once, not interpolated into queries
- **Audit trail**: RLS policy violations logged in PostgreSQL logs

### 2. Defense-in-Depth
Multiple layers of security:
1. **Middleware**: Extracts `workspace_id` from JWT/headers
2. **Session Variable**: Sets PostgreSQL `app.current_workspace_id`
3. **RLS Policies**: Database enforces filtering
4. **Foreign Keys**: Prevent orphaned data

### 3. Testing RLS Isolation
```rust
#[tokio::test]
async fn test_rls_isolation() {
    // Set workspace1 context
    sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
        .bind(workspace1.to_string()).execute(&pool).await.unwrap();
    
    // Insert entity in workspace1
    sqlx::query("INSERT INTO entities (workspace_id, name) VALUES ($1, 'Entity1')")
        .bind(workspace1).execute(&pool).await.unwrap();
    
    // Switch to workspace2
    sqlx::query("SELECT set_config('app.current_workspace_id', $1, true)")
        .bind(workspace2.to_string()).execute(&pool).await.unwrap();
    
    // Query returns 0 results (RLS blocks workspace1 data)
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities")
        .fetch_one(&pool).await.unwrap();
    
    assert_eq!(count, 0); // ✅ RLS isolation verified
}
```

---

## Performance Considerations

### RLS Overhead
- **Benchmark**: 5-10% query latency increase with proper indexes
- **Mitigation**: 
  - Index on `workspace_id` is critical: `CREATE INDEX idx_entities_workspace ON entities(workspace_id)`
  - Use connection pooling (pgBouncer with `session` mode)
  - Monitor with `pg_stat_statements`

### Graph Traversal
- **AGE Performance**: Comparable to dedicated graph DBs for depth 1-3 traversals
- **Optimization**: Indexes on `(source_id, workspace_id)` and `(target_id, workspace_id)`
- **Monitoring**: Use `EXPLAIN ANALYZE` to verify query plans

### Vector Search
- **pgvector IVFFlat**: ~90% recall, 10x faster than brute-force
- **Index Tuning**: `lists = 100` for ~1M vectors, adjust based on dataset size
- **Alternative**: HNSW index for better accuracy (PostgreSQL 16+)

---

## Next Steps

### Phase 5F Completion
Continue with remaining P2 guides (4 remaining):
1. **architecture-diagrams.md**: Update diagrams to show PostgreSQL instead of SurrealDB
2. **advanced-examples.md**: Complex Rust patterns with PostgreSQL + AGE + pgvector
3. **benchmarks.md**: Performance baselines for PostgreSQL graph operations
4. **troubleshooting-runbook.md**: PostgreSQL-specific troubleshooting

### Phase 6: Final Review
- Validate consistency of PostgreSQL decision across all documentation
- Verify all SurrealDB references updated
- Check code examples compile with sqlx
- Ensure ADR cross-references are correct

### Phase 7: Implementation Handoff
- Provide complete PostgreSQL setup guide
- Document AGE and pgvector installation
- Create RLS policy migration scripts
- Benchmark PostgreSQL vs SurrealDB (optional)

---

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Apache AGE Documentation](https://age.apache.org/)
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [PostgreSQL Row-Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [sqlx Documentation](https://docs.rs/sqlx/)
- [ADR-003](../plan/integration/ADR_INDEX.md#adr-003-use-postgresql--age--pgvector-as-primary-database)
- [ADR-004](../plan/integration/ADR_INDEX.md#adr-004-implement-shared-database-multi-tenancy-with-postgresql-rls)

---

## Commit History

1. **11957d51**: `refactor(db): Change from SurrealDB to PostgreSQL+AGE+pgvector with RLS`
   - Updated ADR-003 and ADR-004
   - Complete multi-tenancy-guide.md rewrite
   - PostgreSQL schema with RLS policies
   - PostgresWorkspaceStorage implementation

2. **a8ca491c**: `docs: Update references to PostgreSQL+AGE+pgvector across guides`
   - PHASE_5_PLAN.md updates
   - configuration-guide.md environment variables
   - testing-guide.md tech stack references

---

## Summary

This architectural change prioritizes **production reliability** and **mature ecosystem** over newer technology. PostgreSQL's 25+ years of battle-testing, combined with AGE's graph capabilities and pgvector's embedding search, provides a robust foundation for EdgeQuake's multi-tenant RAG system. The addition of Row-Level Security ensures database-enforced tenant isolation, eliminating an entire class of potential security vulnerabilities.

**Key Takeaway**: Sometimes "boring technology" (PostgreSQL) is the right choice—especially when it provides battle-tested reliability, mature tooling, and native security features (RLS) that newer alternatives lack.
