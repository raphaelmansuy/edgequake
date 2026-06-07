# edgequake-audit — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-audit`  
**LOC:** ~578 (src)  
**Role:** Compliance audit logging (PostgreSQL-backed)

---

## Executive Summary

Small, single-consumer crate with **weak justification as workspace member**. PostgreSQL-only with no storage trait. Version drift from workspace deps. **Merge candidate** into `edgequake-api/src/audit/`.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| AUDIT-DRY-001 | **P2** | Version drift from workspace | Pinned `sqlx 0.8`, `thiserror 2.0` vs workspace shared versions | Align `Cargo.toml` with workspace |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence | Remediation |
|----|---|-----------|-----------|----------|-------------|
| AUDIT-SOLID-S-001 | **P2** | SRP | `logger.rs` mixes async worker, SQL writes, query API | `logger.rs:33-225` | Split worker vs query |
| AUDIT-SOLID-O-001 | **P2** | OCP | PostgreSQL-only; no storage trait | Hard-coded `Pool<Postgres>` | `AuditStorage` trait if multi-backend needed |

---

## Crate Existence Verdict

| Metric | Value |
|--------|-------|
| LOC | ~578 |
| Consumers | `edgequake-api` only |
| Reuse elsewhere | None |

**Recommendation:** Merge into API unless second consumer appears (e.g., standalone audit service).

---

## Remediation Plan

| P | Action |
|---|--------|
| **P2** | Merge into `edgequake-api/src/audit/` |
| **P2** | Introduce `AuditStorage` trait if non-Postgres needed |
| **P3** | Align workspace dependency versions |

---

## Verification

After merge: `cargo test -p edgequake-api` includes audit tests; no separate crate in workspace.
