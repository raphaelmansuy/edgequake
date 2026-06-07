# edgequake-auth — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-auth`  
**LOC:** ~2,960 (src)  
**Role:** JWT validation, password hashing, RBAC, Axum extractors

---

## Executive Summary

**Well-factored security boundary.** Minimal internal duplication. JWT, password, RBAC, and extractors are appropriately separated. No P0/P1 violations. Minor ISP pressure on `types.rs` (~489 LOC).

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| AUTH-DRY-001 | **P3** | None significant | — | — |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence | Remediation |
|----|---|-----------|-----------|----------|-------------|
| AUTH-SOLID-S-001 | **P3** | SRP | `types.rs` mixes roles, API keys, users | 489 LOC | Split when file grows |
| AUTH-SOLID-D-001 | **P3** | DIP | Axum extractor coupling | `extractors.rs` | Acceptable for API auth crate |

---

## Verdict

**Keep as standalone crate.** Clear security boundary, ~3k LOC, single consumer (API) is appropriate for isolation and auditability.

---

## Remediation Plan

| P | Action |
|---|--------|
| **P3** | Split `types.rs` → `user.rs`, `api_key.rs`, `role.rs` when >600 LOC |
| — | No merge recommended |

---

## Positive Patterns

- Argon2 password hashing isolated
- JWT validation separate from RBAC checks
- Error types in dedicated `error.rs`
