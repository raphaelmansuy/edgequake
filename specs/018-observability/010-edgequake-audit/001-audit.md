# edgequake-audit — Observability Audit

**Path:** `edgequake/crates/edgequake-audit`  
**Tracing macros (src):** 3 (`logger.rs`)  
**Role:** Compliance persistence (PostgreSQL `audit_logs`)

---

## Executive Summary

Crate design is sound (async worker, `request_id` column) but:

1. **Not imported** by `edgequake-api` production code (dead workspace dep).
2. **`query_audit_logs` returns empty** — placeholder.
3. Overlaps conceptually with **tracing logs** but serves different audience (SIEM/compliance).

---

## Architecture

```
  API handler ──X──▶ AuditLogger::log()     (not wired today)
                         │
                         ▼
                   mpsc unbounded
                         │
                         ▼
                   audit_worker ──▶ INSERT audit_logs
                         │
                   fail: error!(event_id, ...)
```

Evidence: `logger.rs:33-73`, `audit_logs` schema binds `request_id` at line 118.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| AUDIT-OBS-001 | P0 | Unwired from API | No `use edgequake_audit` in `api/src` | Init in `AppState::new_postgres` |
| AUDIT-OBS-002 | P0 | Query API stub | `logger.rs:204-206` | Implement dynamic SQL |
| AUDIT-OBS-003 | P1 | Unbounded channel | `logger.rs:36` | Document risk; metric `audit_queue_depth` |
| AUDIT-OBS-004 | P2 | Duplicates tracing concern | Two pipelines | DRY: audit = compliance; tracing = ops |
| AUDIT-OBS-005 | P3 | Version drift (017 audit) | `Cargo.toml` | Align workspace deps |

---

## DRY vs tracing

| Signal | Purpose | Retention |
|--------|---------|-----------|
| `tracing::*!` | Real-time ops | Days (Loki) |
| `AuditEvent` | Compliance / legal | Months (DB policy) |

**Same `request_id` must appear in both** (OBS-P0-002).

---

## Verify

```bash
rg 'edgequake_audit' edgequake/crates/edgequake-api/src
rg 'query_audit_logs' edgequake/crates/edgequake-audit -A3
```
