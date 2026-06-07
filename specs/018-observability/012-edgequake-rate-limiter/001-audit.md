# edgequake-rate-limiter — Observability Audit

**Path:** `edgequake/crates/edgequake-rate-limiter`  
**Tracing macros (src):** ~2  
**Role:** Token-bucket rate limiting + Axum middleware

---

## Executive Summary

**Best-in-workspace middleware logging** for its size — exceeds auth/tasks crates. WARN on 429 with tenant/workspace/retry_after is exactly right.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| RL-OBS-001 | P3 | DEBUG on every check | `middleware.rs:45-50` | OK at debug; sample at INFO if needed |
| RL-OBS-002 | P3 | No metric for 429 count | — | `edgequake_rate_limit_exceeded_total` counter |
| RL-OBS-003 | ✅ | Good WARN shape | `middleware.rs:56-60` | Use as template for auth |

---

## Reference Pattern (copy to other middleware)

```rust
warn!(
    tenant_id = tenant_id,
    workspace_id = workspace_id,
    retry_after = retry_after,
    "Rate limit exceeded"
);
```

---

## Verify

```bash
rg 'tracing::' edgequake/crates/edgequake-rate-limiter/src
```
