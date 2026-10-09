---
title: 'SQLx Offline Mode'
description: "How to use SQLx offline mode when building EdgeQuake."
---

# SQLx Offline Mode

> **Product: v0.23.0** · Related: [Migration checksum gate](#migration-checksum-gate-adjacency)

## Overview

EdgeQuake uses SQLx's compile-time query verification, which by default requires a live database connection during compilation. This document explains how we've configured offline mode to allow builds without a running PostgreSQL instance.

## Problem

When building the backend with `cargo build`, SQLx's `query!` and `query_scalar!` macros attempt to connect to PostgreSQL at compile time to verify SQL queries. If the database isn't running, you'll see errors like:

```
error: error communicating with database: Connection refused (os error 61)
  --> crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:50:9
```

## Solution

We use **SQLx offline mode**, which pre-generates query metadata when the database IS available, then uses this cached metadata for future builds.

### Configuration

1. **`edgequake/.cargo/config.toml`** — Sets SQLx offline mode by default:

   ```toml
   [env]
   SQLX_OFFLINE = "true"
   ```

2. **`edgequake/.sqlx/`** — Contains pre-generated query metadata (committed to git)

### Workflow

#### Initial Setup (One Time)

Generate SQLx metadata when you have database access:

```bash
# Start PostgreSQL (from repo root)
make db-start

# Generate SQLx metadata (from repo root)
make backend-sqlx-prepare

# Commit the metadata directory
git add edgequake/.sqlx/
git commit -m "chore: add SQLx offline metadata"
```

#### Regular Development

With offline mode configured, you can build without a database:

```bash
# Build works WITHOUT database running (from repo root)
make backend-build

# Or use cargo directly
cd edgequake && SQLX_OFFLINE=true cargo build --release
```

#### When to Regenerate Metadata

Regenerate SQLx metadata whenever you:

- Add new SQL queries using `sqlx::query!`, `sqlx::query_scalar!`, or `sqlx::query_as!`
- Modify existing SQL queries
- Change database schema (migrations)

```bash
# Regenerate metadata (from repo root)
make backend-sqlx-prepare
```

### Available Make Targets

All targets live in the **repo-root** [Makefile](../Makefile):

| Command                     | Description                               |
| --------------------------- | ----------------------------------------- |
| `make backend-build`        | Build backend in offline mode (DEFAULT)   |
| `make backend-build-online` | Build with live database verification     |
| `make backend-sqlx-prepare` | Generate SQLx metadata for offline builds |

`backend-sqlx-prepare` runs `cargo sqlx prepare --workspace` inside `edgequake/` with `DATABASE_URL` pointed at the local Postgres container.

## How It Works

1. **Offline Mode Enabled**: `SQLX_OFFLINE=true` tells SQLx macros to read from `edgequake/.sqlx/` instead of querying the database

2. **Metadata Files**: Each `sqlx::query!` invocation gets a JSON file in `edgequake/.sqlx/` containing:
   - Query text
   - Parameter types
   - Result column types
   - Nullability information

3. **Compile-Time Verification**: SQLx still validates queries at compile time, but uses cached metadata instead of live database connection

## Migration checksum gate (adjacency)

SQLx offline metadata and migration immutability are separate but related gates:

| Gate | Path / command | What it catches |
| ---- | -------------- | --------------- |
| **SQLx offline** | `edgequake/.sqlx/` + `make backend-sqlx-prepare` | Compile-time query/type drift without a live DB |
| **Migration checksum** | `edgequake/migrations/checksums.lock` + `./scripts/check_migration_checksums.sh` | Byte changes to already-deployed migration SQL (startup would fail with "migration N was previously applied but has been modified") |

When you **add or edit migration SQL**, you must:

1. Apply migrations locally (`make db-start` then restart backend, or run migrations manually)
2. Regenerate SQLx metadata if queries changed: `make backend-sqlx-prepare`
3. Update the checksum lockfile: `./scripts/update_migration_checksums.sh`
4. Commit `edgequake/.sqlx/`, `edgequake/migrations/checksums.lock`, and the migration file together

CI runs `check_migration_checksums.sh` in the **migration-checksum-guard** job. Install local hooks with `./scripts/install_migration_hooks.sh` to catch checksum drift before push.

Regression coverage: `scripts/test_migration_e2e.sh` (lease-view + checksum paths).

## Benefits

✅ **Faster CI/CD**: No need to spin up PostgreSQL in build pipelines  
✅ **Offline Development**: Build without database access  
✅ **Consistent Builds**: Same query verification across all environments  
✅ **Reduced Dependencies**: Build stage doesn't need database credentials

## Troubleshooting

### Error: "cached query must be loaded with `SQLX_OFFLINE=true`"

**Cause**: Query was added/modified but metadata not regenerated

**Fix**:

```bash
make backend-sqlx-prepare
```

### Error: "query not found in .sqlx/"

**Cause**: Using a query that hasn't been prepared yet

**Fix**:

```bash
# Ensure database is running
make db-start

# Regenerate metadata
make backend-sqlx-prepare
```

### Build fails with "Connection refused" even with SQLX_OFFLINE=true

**Cause**: Environment variable not set or `edgequake/.sqlx/` directory missing

**Fix**:

```bash
# Check config
cat edgequake/.cargo/config.toml | grep SQLX_OFFLINE

# Verify .sqlx/ exists
ls -la edgequake/.sqlx/

# Regenerate if missing
make backend-sqlx-prepare
```

### CI fails migration-checksum-guard after editing SQL

**Cause**: Migration file bytes changed but `checksums.lock` not updated

**Fix**:

```bash
./scripts/update_migration_checksums.sh
git add edgequake/migrations/checksums.lock
```

## References

- [SQLx Offline Mode Documentation](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query)
- [EdgeQuake Makefile](../Makefile) — `backend-sqlx-prepare` target
- [edgequake/.cargo/config.toml](../edgequake/.cargo/config.toml) — SQLx configuration
- [scripts/check_migration_checksums.sh](../scripts/check_migration_checksums.sh) — CI immutability gate
