# edgequake-api — Observability Audit

**Path:** `edgequake/crates/edgequake-api`  
**Tracing macros (src):** ~172  
**Role:** HTTP boundary, middleware, metrics, LLM header bridge

---

## Executive Summary

| Area | Grade | Notes |
|------|-------|-------|
| HTTP access logs | B | Structured fields in `request_logging` |
| Correlation | D | `x-request-id` header only; not in spans |
| OTEL | F | Not present |
| Prometheus | F | Static placeholder |
| Error logging | C | Sparse `error!`; errors mostly JSON responses |
| Audit integration | F | Dep declared, never used |

---

## Architecture (as built)

```
  Incoming HTTP
       │
       ▼
  ┌─────────────────┐
  │ request_logging │── info/warn + method, uri, status, duration_ms
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │ request_id      │── UUID → x-request-id (always new)
  └────────┬────────┘
           ▼
  ┌─────────────────┐
  │ TraceLayer      │── tower-http (not OTEL)
  └────────┬────────┘
           ▼
     handlers ──▶ tracing::debug! (query)
                 ApiError JSON (no mandatory error! log)
```

Evidence: `server.rs:89-93`, `middleware.rs:39-86`.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| API-OBS-001 | P0 | Metrics are fake | `handlers/metrics.rs:43-147` | `metrics` crate + middleware hooks |
| API-OBS-002 | P0 | `edgequake-audit` unused | `Cargo.toml:26`; no `use` in `src/` | Wire `AuditLogger` in `AppState` |
| API-OBS-003 | P0 | Request ID not logged as field | `middleware.rs` — no `tracing` span | `RequestIdLayer` + extension |
| API-OBS-004 | P1 | Overwrites inbound request ID | `middleware.rs:72` always `Uuid::new_v4()` | Honor valid inbound header |
| API-OBS-005 | P1 | `extra_headers` not auto-filled | `query_execute.rs:178` from JSON body only | Harvest from `HeaderMap` in handler |
| API-OBS-006 | P1 | Triple HTTP instrumentation | `server.rs` + 2 custom middleware | Consolidate under OTEL layer |
| API-OBS-007 | P1 | `json` subscriber feature unused at runtime | `Cargo.toml:39`; init in `main.rs` fmt only | `EDGEQUAKE_LOG_FORMAT` |
| API-OBS-008 | P2 | Emoji in logs via main/handlers | Indirect | Plain messages |
| API-OBS-009 | P2 | 0 `#[instrument]` on hot handlers | `rg '#[instrument]'` → 0 | Add to query/upload/delete |
| API-OBS-010 | P3 | `ErrorResponse` lacks `request_id` | `error.rs:61-71` | Include in JSON for clients |

---

## Log-Level Sample (code is law)

| Location | Level | Quality |
|----------|-------|---------|
| `middleware.rs:50-64` | info/warn | Good fields; 2xx at INFO is noisy |
| `query_execute.rs:66-71` | debug | Good tenant/query context |
| `safety_limits.rs:438-491` | info/warn | Provider creation — useful |
| `handlers/documents/delete/*.rs` | warn/info mix | Heavy deletion logging — OK |

---

## OTEL / API Compatibility

| Header | Inbound read | Outbound to LLM | In logs |
|--------|--------------|-----------------|---------|
| `x-request-id` | ❌ | Only via `extra_headers` JSON | ❌ |
| `traceparent` | ❌ | Only via `extra_headers` | ❌ |
| `x-tenant-id` | ✅ `TenantContext` | ✅ resolver | Partial debug |
| `x-workspace-id` | ✅ | ✅ | Partial |

Docs: `query_types.rs:125-139`, `safety_limits.rs:449-494`.

---

## Verification Commands

```bash
rg 'edgequake_audit|AuditLogger' edgequake/crates/edgequake-api/src
curl -s localhost:8080/metrics | rg 'edgequake_http_requests_total.* [1-9]'
rg 'tracing::error!' edgequake/crates/edgequake-api/src -c
```

---

## Remediation Priority

1. API-OBS-003 + API-OBS-004 (correlation)
2. API-OBS-001 (metrics)
3. API-OBS-002 (audit)
4. API-OBS-005 (LLM auto-propagate)
