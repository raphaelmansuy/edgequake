# Task Log: SQLx Migration Fix

**Date:** 2025-12-30T05:28 UTC
**Mode:** Beastmode
**Duration:** ~30 minutes

## Actions

- Investigated SQLx migration crash (`duplicate key violates unique constraint "_sqlx_migrations_pkey"`)
- Identified root causes: version 0 invalid + search_path schema conflict
- Renamed 13 migrations from 000-012 to 001-013
- Fixed `init-extensions.sql` to set user default search_path to public
- Fixed `state.rs` to set search_path before running migrations
- Created `migrations/scripts/reset_migrations.sql` utility
- Created `plan_improvement_workspace/migration-sota-plan.md` documentation
- Tested fresh database startup: PASSED
- Tested existing database restart: PASSED (3x)
- Committed and pushed fix (commit 65c9261)

## Decisions

- Used simple renumbering (001-013) instead of timestamp format for minimal disruption
- Fixed both PostgreSQL user-level and connection-level search_path for defense-in-depth
- Created reset utility script for development convenience

## Next Steps

- None required - migration fix is complete and verified working
- Full stack (`make dev`) starts successfully

## Lessons/Insights

- SQLx creates `_sqlx_migrations` table BEFORE running any migration files
- The table is created using the connection's current `search_path`
- PostgreSQL user's default `search_path = "$user", public` can cause schema conflicts
- Always ensure `search_path = public` is set before SQLx migrations run
- SQLx requires migration versions > 0 (000\_ files are invalid)
