# E2E Proof 016 — P5 Bootstrap Size-Aware Migration

**Requirement:** NFR-006-004, TR-006-006  
**Layer:** bootstrap + postgres e2e  
**Status:** ✅ Verified 2026-06-06

---

## Claim

sqlx migration 038 is a **non-blocking marker**; size-aware index DDL runs only via `support/038/apply.sql` in bootstrap or ops. Large graphs defer without blocking startup; `/ready` returns 503 until indexes ready.

---

## Evidence

### Architecture (first principles)

| Layer | Responsibility |
|-------|----------------|
| `038_add_source_ids_gin_indexes.sql` | sqlx version marker only |
| `support/038/apply.sql` | SSOT index DDL + vertex threshold gate |
| `migration_bootstrap.rs` | sqlx + size-aware apply + audit |
| `/ready` | 503 when `migration_038.is_degraded()` |

### Static gate

```bash
./scripts/spec006_source_ids_migration.sh
```

### Unit tests

```bash
cargo test -p edgequake-api migration_038_apply_sql --features postgres --lib
cargo test -p edgequake-api is_ready_for_traffic --features postgres --lib
```

### Postgres E2E (requires DATABASE_URL)

```bash
cargo test -p edgequake-api migration_bootstrap_proof --features postgres
```

Validates:
- Bootstrap completes without error
- Migration 038 marker in `_sqlx_migrations`
- `is_ready_for_traffic` matches index audit
- Second bootstrap is idempotent (`pending_before == 0`)

---

## Regression

Included in `make resource-proof` when `DATABASE_URL` or `POSTGRES_PASSWORD` is set.
