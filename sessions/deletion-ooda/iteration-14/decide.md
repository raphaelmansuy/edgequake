# OODA Iteration 14 - Decide

## Selected Solution: Extend HealthResponse with Schema Info

### Implementation Plan

1. **Add SchemaHealth struct to health_types.rs**
   ```rust
   pub struct SchemaHealth {
       pub latest_version: Option<i64>,
       pub migrations_applied: usize,
       pub last_applied_at: Option<String>,
   }
   ```

2. **Add schema field to HealthResponse**
   ```rust
   pub struct HealthResponse {
       // existing fields...
       pub schema: Option<SchemaHealth>,
   }
   ```

3. **Query _sqlx_migrations in health_check handler**
   - Only when storage_mode is PostgreSQL
   - Memory mode returns None

4. **Update tests**

### Acceptance Criteria

- [ ] HealthResponse includes optional schema field
- [ ] PostgreSQL mode returns migration count and latest version
- [ ] Memory mode returns schema: None
- [ ] Health endpoint still returns 200 when migrations can't be queried
- [ ] All existing tests pass

### SQL Query

```sql
SELECT 
    COUNT(*) FILTER (WHERE success = true) as applied_count,
    MAX(version) FILTER (WHERE success = true) as latest_version,
    MAX(installed_on) FILTER (WHERE success = true) as last_applied_at
FROM _sqlx_migrations;
```

### Non-Goals

- Startup validation (future iteration)
- Detailed migration list
- Migration rollback detection
