# SQLx Offline Mode

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

1. **`.cargo/config.toml`** - Sets SQLx offline mode by default:

   ```toml
   [env]
   SQLX_OFFLINE = "true"
   ```

2. **`.sqlx/` directory** - Contains pre-generated query metadata (committed to git)

### Workflow

#### Initial Setup (One Time)

Generate SQLx metadata when you have database access:

```bash
# Start PostgreSQL
make db-start

# Generate SQLx metadata
make backend-sqlx-prepare

# Commit the .sqlx/ directory to git
git add .sqlx/
git commit -m "chore: add SQLx offline metadata"
```

#### Regular Development

With offline mode configured, you can build without a database:

```bash
# Build works WITHOUT database running
make backend-build

# Or use cargo directly
cd edgequake && cargo build --release
```

#### When to Regenerate Metadata

Regenerate SQLx metadata whenever you:

- Add new SQL queries using `sqlx::query!`, `sqlx::query_scalar!`, or `sqlx::query_as!`
- Modify existing SQL queries
- Change database schema (migrations)

```bash
# Regenerate metadata
make backend-sqlx-prepare
```

### Available Make Targets

| Command                     | Description                               |
| --------------------------- | ----------------------------------------- |
| `make backend-build`        | Build backend in offline mode (DEFAULT)   |
| `make backend-build-online` | Build with live database verification     |
| `make backend-sqlx-prepare` | Generate SQLx metadata for offline builds |

## How It Works

1. **Offline Mode Enabled**: `SQLX_OFFLINE=true` tells SQLx macros to read from `.sqlx/` instead of querying the database

2. **Metadata Files**: Each `sqlx::query!` invocation gets a JSON file in `.sqlx/` containing:
   - Query text
   - Parameter types
   - Result column types
   - Nullability information

3. **Compile-Time Verification**: SQLx still validates queries at compile time, but uses cached metadata instead of live database connection

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

**Cause**: Environment variable not set or `.sqlx/` directory missing

**Fix**:

```bash
# Check config
cat edgequake/.cargo/config.toml | grep SQLX_OFFLINE

# Verify .sqlx/ exists
ls -la .sqlx/

# Regenerate if missing
make backend-sqlx-prepare
```

## References

- [SQLx Offline Mode Documentation](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query)
- [EdgeQuake Makefile](../Makefile) - See backend-sqlx-prepare target
- [.cargo/config.toml](../edgequake/.cargo/config.toml) - SQLx configuration
