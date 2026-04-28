# Prevention Playbook — Migration Immutability

## The Golden Rule

> **Once a migration file has been applied to any deployed database, its content
> MUST NEVER be modified.** Not whitespace. Not comments. Not formatting. Nothing.

SQLx enforces this with a SHA-384 checksum stored in `_sqlx_migrations`. Any byte
change produces a different checksum → deployment failure.

---

## CI Gate: Add a checksum regression test

Add this check to CI (`.github/workflows/`):

```bash
#!/usr/bin/env bash
# scripts/check_migration_checksums.sh
# Fails if any committed migration was modified relative to its reference tag.

set -euo pipefail
BASELINE_TAG="${1:-v0.10.12}"

git fetch --tags
CHANGED=$(git diff "$BASELINE_TAG"..HEAD --name-only -- 'edgequake/migrations/*.sql' | grep -v '^$' || true)

if [[ -n "$CHANGED" ]]; then
  echo "ERROR: The following migration files were modified after baseline $BASELINE_TAG:"
  echo "$CHANGED"
  echo ""
  echo "Migration files are IMMUTABLE once deployed."
  echo "To add schema changes, create a NEW numbered migration file."
  exit 1
fi

echo "OK: No migration files modified relative to $BASELINE_TAG"
```

---

## Pre-commit hook

```bash
#!/usr/bin/env bash
# .git/hooks/pre-commit (chmod +x)
# Warn when a migration file is staged for modification (not addition).

STAGED=$(git diff --cached --name-only --diff-filter=M -- 'edgequake/migrations/*.sql')
if [[ -n "$STAGED" ]]; then
  echo ""
  echo "⛔  WARNING: You are modifying an existing migration file:"
  echo "$STAGED"
  echo ""
  echo "  Migration files are IMMUTABLE once deployed."
  echo "  To make schema changes, create a NEW migration file."
  echo ""
  echo "  To proceed anyway (e.g., reverting a broken migration):"
  echo "    git commit --no-verify"
  echo ""
  exit 1
fi
```

---

## Decision tree: What to do when you need to change a migration

```
Need to change schema behaviour that an existing migration covers?
│
├─► Has the migration been deployed to any non-ephemeral DB?
│       │
│       ├─► YES → Create a NEW migration file (next sequence number)
│       │           with idempotent ALTER TABLE / CREATE INDEX IF NOT EXISTS etc.
│       │
│       └─► NO  → You may edit the migration file.
│                  Coordinate with all team members who may have already run it.
│
└─► Is it a comment / whitespace only change?
        │
        └─► STILL NO — comments change the checksum.
```

---

## Recovery procedure (for affected deployments)

If migration 001 checksum mismatch has already been hit in production:

### Option A (safe — apply this fix)

Upgrade to the patched image (this fix branch / next patch release).  
The restored migration 001 checksum matches what is stored in the DB.  
No manual DB intervention required.

### Option B (emergency manual workaround — use only if Option A is not available)

```sql
-- Connect as superuser
-- Update the stored checksum to match the NEW (broken) file on disk
-- ONLY do this if you cannot use the patched binary

UPDATE _sqlx_migrations
SET checksum = decode(
  '9e44513e1b22ab482a3703f394d1f0e35fe24625b77eca236789a3b702bbf6c1ceb9ed8beed4e13c9c4ca4b28feae925',
  'hex'
)
WHERE version = 1;
```

⚠️  Option B is a temporary workaround. Always prefer upgrading to the patched binary.

---

## References

- [SQLx migration source — checksum verification](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/src/migrate.rs)
- [Issue #195](https://github.com/raphaelmansuy/edgequake/issues/195)
- [Root cause analysis](./root_cause_analysis.md)
- [Fix specification](./fix_specification.md)
