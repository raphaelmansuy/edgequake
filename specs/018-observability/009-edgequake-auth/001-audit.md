# edgequake-auth — Observability Audit

**Path:** `edgequake/crates/edgequake-auth`  
**Tracing macros (src):** **0**  
**Role:** JWT validation, RBAC, Axum extractors

---

## Executive Summary

Auth crate is **completely silent** at runtime — failures return HTTP 401/403 without structured security logs. For enterprise (roadmap Q2 SSO/RBAC), this is a **P1 gap**.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| AUTH-OBS-001 | P1 | Zero tracing in src | `rg tracing edgequake-auth/src` → empty | `warn!` failed auth; `debug!` success |
| AUTH-OBS-002 | P1 | No audit event on auth failure | No bridge to `edgequake-audit` | Emit `AuditEvent` from API middleware |
| AUTH-OBS-003 | P2 | Token validation errors opaque | Extractors | `warn!(reason = "expired")` — no token payload |
| AUTH-OBS-004 | P3 | Depends on `tracing` in Cargo | `Cargo.toml` | Use the dependency |

---

## Security Logging Rules

| Event | Level | Fields |
|-------|-------|--------|
| Invalid JWT | WARN | `ip`, `request_id`, `reason` |
| Missing API key | WARN | path (not key value) |
| RBAC denied | WARN | `user_id`, `role`, `resource` |
| Successful login | INFO | `user_id`, `tenant_id` |

**Never log:** raw tokens, passwords, API keys.

---

## Target

```
auth_middleware
    ├── validate JWT
    ├── on fail: warn! + audit SecurityDenied
    └── on ok: debug! + span user_id
```

---

## Verify

```bash
rg 'tracing::' edgequake/crates/edgequake-auth/src || echo "CONFIRMED: silent"
```
