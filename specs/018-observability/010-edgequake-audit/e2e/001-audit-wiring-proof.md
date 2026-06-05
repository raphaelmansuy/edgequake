# SPEC-018 — Audit wiring proof

**Status:** ✅ Proven (compile + unit); DB integration requires PostgreSQL  
**Date:** 2026-06-05

## Claim

`AuditLogger` is initialized in `AppState::new_postgres`; query handlers emit `DocumentQuery` events; `query_audit_logs` uses `QueryBuilder` (no longer empty stub).

## Evidence

```bash
cargo test -p edgequake-audit --lib
rg 'audit_logger' edgequake/crates/edgequake-api/src/state/postgres.rs
rg 'record_audit' edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs
```

## Code (law)

- `edgequake/crates/edgequake-api/src/services/audit.rs`
- `edgequake/crates/edgequake-audit/src/logger.rs` — `query_audit_logs`
