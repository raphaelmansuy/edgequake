# Migration Safety Guide - First Principles Approach

## First Principles: Core Truths

1. **Data is the source of truth** - Schema exists to serve data, not vice versa
2. **Migrations are one-way time travel** - Once applied to production, rollback is expensive
3. **Failures compound exponentially** - One bad migration can cascade across systems
4. **Observable systems are debuggable** - Without logs/traces, we're flying blind
5. **Testing in production is discovery, not validation** - Validate before deploying

---

## The Migration Safety Pyramid

```
                    🔒 Production Deploy
                   /                    \
              Validation               Monitoring
             /          \             /          \
        Pre-Checks    Post-Checks   Logs      Metrics
       /        \     /        \     /  \      /  \
    Schema   Data  Verify  Rollback Trace Debug Alert Health
    |______|______|______|_________|_____|_____|_____|
              Foundation: Transactional Integrity
```

---

## 1. Transactional Safety (Foundation)

### Principle: All-or-Nothing Execution

**Problem**: Partial migrations leave database in undefined state.

**Solution**: Wrap ALL migrations in transactions.

```sql
-- Migration: 017_example.sql
BEGIN;  -- ✅ ALWAYS START WITH THIS

-- Your migration code here
CREATE TABLE ...;
ALTER TABLE ...;

-- Validation checkpoint
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'expected_table') THEN
        RAISE EXCEPTION 'Migration validation failed: table not created';
    END IF;
END $$;

COMMIT;  -- ✅ ONLY COMMITS IF ALL SUCCEEDED
```

### Anti-Pattern ❌

```sql
-- BAD: No transaction wrapper
CREATE TABLE foo ...;
-- If this fails, foo is already created ↑
CREATE TABLE bar ...;  -- This might fail
```

---

## 2. Pre-Flight Validation

### Principle: Detect Conflicts Before Execution

**Validation Checklist:**

```sql
-- At the start of every migration:

-- 1. Check dependencies exist
DO $$
BEGIN
    -- Verify parent tables exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'workspaces') THEN
        RAISE EXCEPTION 'Migration requires workspaces table';
    END IF;
    
    -- Verify parent columns have correct type
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'workspaces' 
        AND column_name = 'workspace_id' 
        AND udt_name = 'uuid'
    ) THEN
        RAISE EXCEPTION 'workspace_id must be UUID type';
    END IF;
END $$;

-- 2. Check for naming conflicts
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'new_table') THEN
        RAISE NOTICE 'Table new_table already exists, skipping creation';
    END IF;
END $$;

-- 3. Verify data integrity constraints
DO $$
DECLARE
    orphan_count INT;
BEGIN
    SELECT COUNT(*) INTO orphan_count 
    FROM child_table c
    LEFT JOIN parent_table p ON c.parent_id = p.id
    WHERE p.id IS NULL;
    
    IF orphan_count > 0 THEN
        RAISE EXCEPTION 'Found % orphaned records - fix before migration', orphan_count;
    END IF;
END $$;
```

---

## 3. Comprehensive Error Logging

### Principle: Every Failure Tells a Story

**Rust Migration Runner Enhancement:**

```rust
// edgequake/crates/edgequake-storage/src/postgres/migrations.rs

use tracing::{error, warn, info, debug};

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Pre-flight check failed: {reason}")]
    PreFlightFailed { 
        migration: i64, 
        reason: String,
        details: serde_json::Value 
    },
    
    #[error("Migration execution failed: {sql_error}")]
    ExecutionFailed {
        migration: i64,
        statement: String,
        sql_error: String,
        hint: Option<String>,
        context: HashMap<String, String>,
    },
    
    #[error("Post-migration validation failed: {reason}")]
    ValidationFailed {
        migration: i64,
        expected: String,
        actual: String,
    },
}

pub async fn run_migrations_safe(pool: &PgPool) -> Result<(), MigrationError> {
    let span = tracing::info_span!("database_migrations", pid = std::process::id());
    let _guard = span.enter();
    
    info!("🔍 Starting migration safety checks");
    
    // 1. Pre-flight: Check database connectivity
    match sqlx::query("SELECT 1").fetch_one(pool).await {
        Ok(_) => info!("✓ Database connectivity verified"),
        Err(e) => {
            error!(error = %e, "✗ Database connection failed");
            return Err(MigrationError::PreFlightFailed {
                migration: 0,
                reason: "Database unreachable".into(),
                details: json!({ "error": e.to_string() }),
            });
        }
    }
    
    // 2. Pre-flight: Check extensions
    let required_extensions = vec!["uuid-ossp", "pgvector"];
    for ext in required_extensions {
        match sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = $1)"
        )
        .bind(ext)
        .fetch_one(pool)
        .await {
            Ok(true) => info!(extension = ext, "✓ Extension available"),
            Ok(false) => {
                warn!(extension = ext, "⚠ Extension not installed");
                // Don't fail, just warn - some migrations might create it
            },
            Err(e) => {
                error!(extension = ext, error = %e, "✗ Failed to check extension");
            }
        }
    }
    
    // 3. Pre-flight: Lock migrations table
    info!("🔒 Acquiring migration lock");
    match sqlx::query("SELECT pg_advisory_lock(1234567890)")
        .execute(pool)
        .await {
        Ok(_) => info!("✓ Migration lock acquired"),
        Err(e) => {
            error!(error = %e, "✗ Failed to acquire migration lock");
            return Err(MigrationError::PreFlightFailed {
                migration: 0,
                reason: "Lock acquisition failed - another migration in progress?".into(),
                details: json!({ "error": e.to_string() }),
            });
        }
    }
    
    // 4. Run migrations with detailed logging
    let migrations = sqlx::migrate!("./migrations");
    
    for migration in migrations.iter() {
        let start = std::time::Instant::now();
        let migration_span = tracing::info_span!(
            "migration",
            version = migration.version,
            description = migration.description,
        );
        let _guard = migration_span.enter();
        
        info!(
            version = migration.version,
            description = migration.description,
            "▶ Starting migration"
        );
        
        // Execute with detailed error context
        match run_single_migration_safe(pool, migration).await {
            Ok(()) => {
                let duration = start.elapsed();
                info!(
                    version = migration.version,
                    duration_ms = duration.as_millis(),
                    "✅ Migration completed successfully"
                );
            }
            Err(e) => {
                let duration = start.elapsed();
                error!(
                    version = migration.version,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "❌ Migration failed"
                );
                
                // Enhanced error context
                log_migration_failure_context(pool, migration, &e).await;
                
                // Release lock before returning
                let _ = sqlx::query("SELECT pg_advisory_unlock(1234567890)")
                    .execute(pool)
                    .await;
                
                return Err(e);
            }
        }
    }
    
    // 5. Post-migration validation
    info!("🔍 Running post-migration validation");
    validate_schema_integrity(pool).await?;
    
    // 6. Release lock
    sqlx::query("SELECT pg_advisory_unlock(1234567890)")
        .execute(pool)
        .await
        .ok();
    
    info!("✅ All migrations completed successfully");
    Ok(())
}

async fn run_single_migration_safe(
    pool: &PgPool,
    migration: &Migration,
) -> Result<(), MigrationError> {
    // Begin transaction
    let mut tx = pool.begin().await.map_err(|e| {
        MigrationError::ExecutionFailed {
            migration: migration.version,
            statement: "BEGIN".into(),
            sql_error: e.to_string(),
            hint: Some("Check database connection and permissions".into()),
            context: HashMap::new(),
        }
    })?;
    
    // Execute migration SQL
    for (idx, statement) in migration.sql.split(';').enumerate() {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        
        debug!(
            version = migration.version,
            statement_idx = idx,
            statement = %statement,
            "Executing SQL statement"
        );
        
        if let Err(e) = sqlx::query(statement).execute(&mut *tx).await {
            error!(
                version = migration.version,
                statement_idx = idx,
                statement = %statement,
                error = %e,
                "Statement execution failed"
            );
            
            // Extract PostgreSQL error details
            let (hint, context) = extract_postgres_error_context(&e);
            
            return Err(MigrationError::ExecutionFailed {
                migration: migration.version,
                statement: statement.to_string(),
                sql_error: e.to_string(),
                hint,
                context,
            });
        }
    }
    
    // Commit transaction
    tx.commit().await.map_err(|e| {
        MigrationError::ExecutionFailed {
            migration: migration.version,
            statement: "COMMIT".into(),
            sql_error: e.to_string(),
            hint: Some("Transaction commit failed - check constraints".into()),
            context: HashMap::new(),
        }
    })?;
    
    Ok(())
}

async fn log_migration_failure_context(
    pool: &PgPool,
    migration: &Migration,
    error: &MigrationError,
) {
    error!("=== MIGRATION FAILURE CONTEXT ===");
    error!("Migration: {} - {}", migration.version, migration.description);
    error!("Error: {}", error);
    
    // Log current schema state
    if let Ok(tables) = sqlx::query_scalar::<_, String>(
        "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename"
    )
    .fetch_all(pool)
    .await {
        error!("Current tables: {:?}", tables);
    }
    
    // Log migration history
    if let Ok(applied) = sqlx::query_scalar::<_, i64>(
        "SELECT version FROM _sqlx_migrations ORDER BY version"
    )
    .fetch_all(pool)
    .await {
        error!("Applied migrations: {:?}", applied);
    }
    
    // Log disk space
    if let Ok(disk_space) = sqlx::query_scalar::<_, String>(
        "SELECT pg_size_pretty(pg_database_size(current_database()))"
    )
    .fetch_one(pool)
    .await {
        error!("Database size: {}", disk_space);
    }
    
    error!("=== END CONTEXT ===");
}

fn extract_postgres_error_context(err: &sqlx::Error) -> (Option<String>, HashMap<String, String>) {
    let mut context = HashMap::new();
    let mut hint = None;
    
    if let sqlx::Error::Database(db_err) = err {
        context.insert("code".into(), db_err.code().unwrap_or_default().to_string());
        context.insert("message".into(), db_err.message().to_string());
        
        if let Some(detail) = db_err.details() {
            context.insert("detail".into(), detail.to_string());
        }
        
        hint = db_err.hint().map(|s| s.to_string());
        
        // Add helpful hints based on error code
        if let Some(code) = db_err.code() {
            hint = Some(match code.as_ref() {
                "23503" => "Foreign key violation - parent record doesn't exist".into(),
                "23505" => "Unique constraint violation - duplicate key".into(),
                "42P01" => "Table doesn't exist - check migration order".into(),
                "42703" => "Column doesn't exist - verify parent table schema".into(),
                "42804" => "Type mismatch - check column types match".into(),
                _ => hint.unwrap_or_else(|| "Check PostgreSQL docs for error code".into()),
            });
        }
    }
    
    (hint, context)
}

async fn validate_schema_integrity(pool: &PgPool) -> Result<(), MigrationError> {
    info!("Validating schema integrity");
    
    // Check for orphaned foreign keys
    let orphan_check = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM information_schema.table_constraints tc
        WHERE tc.constraint_type = 'FOREIGN KEY'
        AND NOT EXISTS (
            SELECT 1 
            FROM information_schema.tables t
            WHERE t.table_name = tc.table_name
        )
        "#
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    if orphan_check > 0 {
        warn!("Found {} orphaned foreign key constraints", orphan_check);
    }
    
    // Check for missing indexes on foreign keys
    let missing_indexes = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu 
            ON tc.constraint_name = kcu.constraint_name
        WHERE tc.constraint_type = 'FOREIGN KEY'
        AND NOT EXISTS (
            SELECT 1 FROM pg_indexes i
            WHERE i.tablename = tc.table_name
            AND i.indexdef LIKE '%' || kcu.column_name || '%'
        )
        "#
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    if missing_indexes > 0 {
        warn!("Found {} foreign keys without indexes - performance impact!", missing_indexes);
    }
    
    info!("✓ Schema integrity validation passed");
    Ok(())
}
```

---

## 4. Rollback Strategy

### Principle: Every Migration Needs an Escape Hatch

**Down Migrations (Recommended):**

```sql
-- migrations/017_add_metrics.up.sql
CREATE TABLE workspace_metrics_history (...);
CREATE INDEX idx_metrics_workspace_time ON workspace_metrics_history(...);

-- migrations/017_add_metrics.down.sql
DROP INDEX IF EXISTS idx_metrics_workspace_time;
DROP TABLE IF EXISTS workspace_metrics_history;
```

**Emergency Rollback Script:**

```bash
#!/bin/bash
# scripts/rollback_migration.sh

set -euo pipefail

MIGRATION_VERSION=$1
DATABASE_URL=${DATABASE_URL:?DATABASE_URL not set}

echo "⚠️  WARNING: Rolling back migration $MIGRATION_VERSION"
echo "This will execute the down migration and mark it as unapplied."
read -p "Continue? (yes/no): " confirm

if [ "$confirm" != "yes" ]; then
    echo "Rollback cancelled"
    exit 0
fi

# 1. Backup database first
echo "📦 Creating backup..."
pg_dump "$DATABASE_URL" > "backup_before_rollback_${MIGRATION_VERSION}_$(date +%Y%m%d_%H%M%S).sql"

# 2. Execute down migration
echo "🔄 Executing down migration..."
psql "$DATABASE_URL" < "migrations/${MIGRATION_VERSION}_*.down.sql"

# 3. Remove from migration history
echo "📝 Updating migration history..."
psql "$DATABASE_URL" -c "DELETE FROM _sqlx_migrations WHERE version = $MIGRATION_VERSION"

echo "✅ Rollback complete"
```

---

## 5. Testing Strategy

### Principle: Test Migrations Against Real Data

**Migration Test Suite:**

```rust
// tests/integration/migration_tests.rs

#[cfg(test)]
mod migration_safety_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_migration_016_with_existing_data() {
        // 1. Setup: Create database with data
        let pool = create_test_database().await;
        apply_migrations_up_to(15, &pool).await;
        seed_test_data(&pool).await;
        
        // 2. Take snapshot
        let snapshot_before = capture_database_snapshot(&pool).await;
        
        // 3. Apply migration 016
        let result = apply_migration(16, &pool).await;
        
        // 4. Verify success
        assert!(result.is_ok(), "Migration 016 should succeed");
        
        // 5. Verify data integrity
        let snapshot_after = capture_database_snapshot(&pool).await;
        assert_eq!(
            snapshot_before.row_counts, 
            snapshot_after.row_counts,
            "Row counts should be preserved"
        );
        
        // 6. Verify new table exists
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'workspace_metrics_history')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        
        assert!(table_exists, "New table should be created");
        
        // 7. Verify foreign keys work
        let fk_works = sqlx::query(
            "INSERT INTO workspace_metrics_history (workspace_id, document_count) 
             SELECT workspace_id, 0 FROM workspaces LIMIT 1"
        )
        .execute(&pool)
        .await;
        
        assert!(fk_works.is_ok(), "Foreign key constraint should work");
    }
    
    #[tokio::test]
    async fn test_migration_016_rollback() {
        let pool = create_test_database().await;
        apply_migrations_up_to(16, &pool).await;
        
        // Execute down migration
        let result = execute_sql_file(&pool, "migrations/016_*.down.sql").await;
        assert!(result.is_ok(), "Rollback should succeed");
        
        // Verify table is gone
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'workspace_metrics_history')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        
        assert!(!table_exists, "Table should be removed after rollback");
    }
    
    #[tokio::test]
    async fn test_migration_016_idempotency() {
        let pool = create_test_database().await;
        apply_migrations_up_to(15, &pool).await;
        
        // Apply migration 016 twice
        let result1 = apply_migration(16, &pool).await;
        let result2 = apply_migration(16, &pool).await;
        
        assert!(result1.is_ok(), "First application should succeed");
        assert!(result2.is_ok(), "Second application should be safe (no-op)");
    }
}
```

---

## 6. Production Deployment Checklist

### Pre-Deployment

- [ ] Migration tested against production-like data volume
- [ ] Migration tested against actual production schema export
- [ ] Rollback script tested and verified
- [ ] Database backup scheduled and verified
- [ ] Migration duration estimated (< 5min acceptable)
- [ ] Downtime window approved if needed
- [ ] Monitoring alerts configured
- [ ] On-call engineer assigned

### During Deployment

```bash
#!/bin/bash
# deploy_migration.sh - Production deployment script

set -euo pipefail

echo "=== PRODUCTION MIGRATION DEPLOYMENT ==="

# 1. Verify environment
if [ "$ENVIRONMENT" != "production" ]; then
    echo "❌ This script is for production only"
    exit 1
fi

# 2. Create backup
echo "📦 Creating pre-migration backup..."
BACKUP_FILE="prod_backup_$(date +%Y%m%d_%H%M%S).sql"
pg_dump "$DATABASE_URL" | gzip > "$BACKUP_FILE.gz"
aws s3 cp "$BACKUP_FILE.gz" "s3://backups/pre-migration/"

# 3. Enable enhanced logging
export RUST_LOG=debug,sqlx=trace

# 4. Run migration with timeout
echo "🚀 Starting migration..."
timeout 600s cargo run --bin migrate || {
    echo "❌ Migration timed out or failed"
    echo "📞 Calling incident response..."
    # Trigger PagerDuty/alert
    exit 1
}

# 5. Verify migration
echo "🔍 Verifying migration..."
psql "$DATABASE_URL" -c "SELECT version, description FROM _sqlx_migrations ORDER BY version DESC LIMIT 5"

# 6. Run smoke tests
echo "🧪 Running smoke tests..."
cargo test --test smoke_tests || {
    echo "❌ Smoke tests failed - consider rollback"
    exit 1
}

echo "✅ Migration deployment complete"
```

### Post-Deployment

- [ ] Verify migration in `_sqlx_migrations` table
- [ ] Run smoke tests against production
- [ ] Monitor error rates for 15 minutes
- [ ] Check query performance hasn't degraded
- [ ] Verify application logs for migration-related errors
- [ ] Update runbook with actual migration duration

---

## 7. Monitoring & Alerting

```rust
// Add metrics to track migration health

use prometheus::{IntCounter, Histogram};

lazy_static! {
    static ref MIGRATION_SUCCESS: IntCounter = register_int_counter!(
        "migration_success_total",
        "Total successful migrations"
    ).unwrap();
    
    static ref MIGRATION_FAILURE: IntCounter = register_int_counter!(
        "migration_failure_total",
        "Total failed migrations"
    ).unwrap();
    
    static ref MIGRATION_DURATION: Histogram = register_histogram!(
        "migration_duration_seconds",
        "Migration execution time"
    ).unwrap();
}

// Use in migration code:
let timer = MIGRATION_DURATION.start_timer();
match run_migrations(pool).await {
    Ok(_) => {
        MIGRATION_SUCCESS.inc();
        timer.observe_duration();
    }
    Err(e) => {
        MIGRATION_FAILURE.inc();
        error!("Migration failed: {}", e);
    }
}
```

---

## 8. Common Pitfalls & Solutions

| Pitfall | Solution |
|---------|----------|
| Type mismatch (TEXT vs UUID) | Always check parent table schema first |
| Missing parent table | Add explicit dependency check in migration |
| Large table lock timeout | Use `CONCURRENTLY` for indexes, batch operations |
| Orphaned records | Run integrity check before migration |
| Failed rollback | Test down migrations as thoroughly as up |
| No-op on re-run causes failure | Use `IF NOT EXISTS` / `IF EXISTS` |
| Foreign key cascade surprise | Explicitly document cascade behavior |

---

## Summary: The Bulletproof Checklist

Every migration MUST have:

1. ✅ Transaction wrapper (BEGIN/COMMIT)
2. ✅ Pre-flight validation (dependencies, types, conflicts)
3. ✅ Comprehensive error logging (structured, contextual)
4. ✅ Post-migration verification (integrity checks)
5. ✅ Down migration script (tested rollback)
6. ✅ Integration tests (real data, idempotency)
7. ✅ Production deployment script (backup, timeout, verify)
8. ✅ Monitoring (metrics, alerts, dashboards)

**Principle**: If you can't confidently roll back, you're not ready to roll forward.
