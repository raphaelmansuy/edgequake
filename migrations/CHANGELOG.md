# Changelog (migrations)

All notable changes to the EdgeQuake migrations directory are tracked here. See the root CHANGELOG.md for workspace-wide changes.

## [Unreleased]

### Added

- **038_add_source_ids_gin_indexes.sql** — btree index on vertex `source_id` and GIN indexes on vertex/edge `source_ids` for bounded document-scoped graph queries (SPEC-006). Auto-applied by sqlx on backend start.
- **support/038/** ops package (not sqlx-scanned):
  - `preflight.sql` — read-only pre-apply checks (AGE, row counts, large-graph warning)
  - `concurrent.sql` — `CREATE INDEX CONCURRENTLY` variant for zero-downtime on large graphs
  - `rollback.sql` — `DROP INDEX IF EXISTS` (indexes only, no data loss)
  - `verify.sql` — post-apply verification gate
- **edgequake/scripts/migrations/apply_038.sh** — canonical production wrapper with `--dry-run`, `--apply`, `--concurrent`, `--verify`, `--rollback`.

### Changed

- Relocated auxiliary 038 SQL from top-level `038_*.sql` duplicates to `support/038/` to prevent sqlx version conflicts and clarify auto-apply vs manual ops boundaries.

### Documentation

- `edgequake/docs/migrations.md` — general migration guide
- `edgequake/docs/migrations/038-source-ids-indexes.md` — FAQ, edge cases, rollout
