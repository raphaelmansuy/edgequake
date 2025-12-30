# SQLx Migrations SOTA Plan

## Date: 2025-12-30

## Status: IN PROGRESS

---

## 🚨 CRITICAL ISSUE IDENTIFIED

### Root Cause Analysis

The backend panics with:

```
duplicate key value violates unique constraint "_sqlx_migrations_pkey"
Key (version)=(0) already exists
```

**Root Cause:** Migration file `000_init_database.sql` uses version `0`, but **SQLx requires versions > 0**.

From SQLx documentation:

> `<VERSION>` is a string that can be parsed into `i64` and **its value is greater than zero**

Additionally, the database has tables but `_sqlx_migrations` is empty (0 rows), indicating:

1. Tables were created by running SQL scripts directly (psql/manual)
2. SQLx has no record of applied migrations
3. When SQLx tries to run migrations, version 0 causes problems

---

## 📋 Issues To Fix

| #   | Issue                         | Severity | Impact               |
| --- | ----------------------------- | -------- | -------------------- |
| 1   | Version 0 is invalid for SQLx | CRITICAL | Startup crash        |
| 2   | Migration overlap/duplication | HIGH     | Data inconsistency   |
| 3   | Missing checksum tracking     | HIGH     | Re-run failures      |
| 4   | No migration reset support    | MEDIUM   | Dev experience       |
| 5   | Inconsistent timestamp format | LOW      | Convention violation |

---

## 🎯 SQLx Migration Best Practices

### Naming Convention

```
<YYYYMMDDHHMMSS>_<description>.sql
```

- Version: 14-digit timestamp (or sequential positive integer > 0)
- Description: lowercase, underscore-separated
- Example: `20250101000001_init_database.sql`

### Best Practices Checklist

- [ ] Versions must be > 0 (no 000\_\*)
- [ ] Use timestamp-based versioning for team collaboration
- [ ] Each migration is atomic and idempotent
- [ ] Never modify already-applied migrations
- [ ] Use reversible migrations for production
- [ ] Separate DDL and DML migrations
- [ ] Test fresh install + upgrade paths

---

## 🔧 Implementation Plan

### Phase 1: Rename Migrations (Version Fix)

**Goal:** Fix version 0 issue by renaming to proper timestamp format

| Old Name                                 | New Name                                            | Action |
| ---------------------------------------- | --------------------------------------------------- | ------ |
| `000_init_database.sql`                  | `20250101000001_init_database.sql`                  | Rename |
| `001_add_tasks_table.sql`                | `20250101000002_add_tasks_table.sql`                | Rename |
| `002_add_document_status_fields.sql`     | `20250101000003_add_document_status_fields.sql`     | Rename |
| `003_add_conversation_history_table.sql` | `20250101000004_add_conversation_history_table.sql` | Rename |
| `004_add_audit_log_table.sql`            | `20250101000005_add_audit_log_table.sql`            | Rename |
| `005_add_is_manual_flags.sql`            | `20250101000006_add_is_manual_flags.sql`            | Rename |
| `006_add_auth_tables.sql`                | `20250101000007_add_auth_tables.sql`                | Rename |
| `007_add_multi_tenancy_tables.sql`       | `20250101000008_add_multi_tenancy_tables.sql`       | Rename |
| `008_add_rls_policies.sql`               | `20250101000009_add_rls_policies.sql`               | Rename |
| `009_add_conversations_tables.sql`       | `20250101000010_add_conversations_tables.sql`       | Rename |
| `010_tenant_performance_indexes.sql`     | `20250101000011_add_tenant_indexes.sql`             | Rename |
| `011_audit_logs_table.sql`               | `20250101000012_add_security_audit.sql`             | Rename |
| `012_add_age_graph.sql`                  | `20250101000013_add_age_graph.sql`                  | Rename |

### Phase 2: Database Reset Script

**Goal:** Create utility to properly clean database for fresh migrations

Create `scripts/reset-migrations.sql`:

```sql
-- Drop _sqlx_migrations to allow fresh migration run
DROP TABLE IF EXISTS _sqlx_migrations CASCADE;
-- Optionally drop all tables for clean start
```

### Phase 3: Baseline Migration (For Existing Databases)

**Goal:** Handle databases that already have tables

Strategy: Create a baseline script that:

1. Checks if tables exist
2. Creates `_sqlx_migrations` with existing migrations marked as applied
3. Allows new migrations to run on top

### Phase 4: Test Scenarios

- [ ] Fresh database → all migrations apply
- [ ] Existing database (tables present, no \_sqlx_migrations) → baseline works
- [ ] Existing database (partial migrations) → continues from last
- [ ] Re-run same migrations → idempotent (no errors)

---

## 🔄 Alternative: Simple Renumber Strategy

Instead of full timestamp refactor, simply renumber:

- `000_` → `001_` (shift all versions +1)
- Minimal change, keeps sequential numbering

**Decision:** Use simple renumber (less disruption)

| Old Name                  | New Name                  |
| ------------------------- | ------------------------- |
| `000_init_database.sql`   | `001_init_database.sql`   |
| `001_add_tasks_table.sql` | `002_add_tasks_table.sql` |
| `002_...`                 | `003_...`                 |
| ...                       | ...                       |
| `012_add_age_graph.sql`   | `013_add_age_graph.sql`   |

---

## 📁 Files To Modify

1. **Migrations Directory:** Rename all files (+1 to version)
2. **Documentation:** Update any references to migration numbers
3. **Test Files:** Update any hardcoded migration references
4. **CI/CD:** Ensure migration order is correct

---

## ✅ Success Criteria

1. `make dev` starts without migration errors (blank database)
2. `make dev` starts without migration errors (existing database)
3. `cargo test` passes for storage integration tests
4. All migrations are idempotent (re-runnable)
5. SQLx tracks all migrations in `_sqlx_migrations`

---

## 📝 Execution Checklist

- [ ] Backup current migrations
- [ ] Rename migration files (000* → 001*, etc.)
- [ ] Create database reset script
- [ ] Create baseline migration helper
- [ ] Test: Fresh database scenario
- [ ] Test: Existing database scenario
- [ ] Update documentation
- [ ] Commit and push changes
