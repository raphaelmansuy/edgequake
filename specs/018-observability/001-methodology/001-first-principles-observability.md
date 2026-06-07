# First-Principles Observability Framework

**Spec:** 018-observability  
**Date:** 2026-06-05  
**Method:** Code is law.

---

## 1. Why Observability Is Not “Logging”

For a multi-tenant RAG platform, operators need to answer four questions **without SSH**:

| Question | Signal | EdgeQuake today |
|----------|--------|-----------------|
| Is it up? | Health/readiness | ✅ `/health`, `/ready`, `/live` |
| Is it slow? | Latency histograms | ❌ `/metrics` zeros only |
| Is it failing? | Error rate + exemplars | ⚠️ `warn!` on HTTP ≥400; sparse `error!` |
| *Why* did request X fail? | Trace + log correlation | ❌ No trace IDs in logs |

**First principle:** A request identifier must bind **HTTP → handler → storage → LLM → audit** in one queryable key (`trace_id` or `request_id`).

---

## 2. Three Pillars (W3C / OTEL Model)

```
┌────────────────────────────────────────────────────────────────────┐
│                    OBSERVABILITY PILLARS                            │
├────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   LOGS (events)          METRICS (aggregates)     TRACES (causality) │
│   ─────────────          ──────────────────       ─────────────────  │
│   tracing::*!            Prometheus counters      Span parent/child  │
│   structured JSON        histograms, gauges       W3C traceparent    │
│                                                                     │
│         └──────────────────┬──────────────────────┘                  │
│                            │                                        │
│                    CORRELATION ID                                   │
│              (trace_id == request_id in MVP)                      │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

EdgeQuake implements **pillar 1 partially** (unstructured fmt logs). Pillars 2–3 are **documented but not implemented**.

---

## 3. Log-Level Contract (Code Is Law)

Apply consistently across crates (DRY — single policy, many emitters):

| Level | When to use | Must include fields |
|-------|-------------|---------------------|
| **ERROR** | Invariant violated; user/data loss risk; retry exhausted | `error`, `request_id`, `tenant_id`, `workspace_id`, `operation` |
| **WARN** | Degraded success; auto-corrected; rate limit; partial failure | same + `reason` |
| **INFO** | Lifecycle boundaries (start/complete), quota, provider selection | `duration_ms` on completes |
| **DEBUG** | Branch decisions, cache hits, retrieval counts | no PII in message body |
| **TRACE** | Parser token-level (avoid in hot paths) | pipeline parsers only |

### Anti-patterns found in codebase

| Anti-pattern | Example | Fix |
|--------------|---------|-----|
| Emoji in production logs | `main.rs` `"🐘 PostgreSQL..."` | INFO plain text; emojis break log parsers |
| Success path at WARN | — | Reserve WARN for degraded |
| `info!` for every HTTP 200 | `middleware.rs:50` | DEBUG or TraceLayer only; INFO at sampling |
| Missing `error!` on handler failures | ~15 `error!` workspace-wide | Map `ApiError` → `error!` with context |
| `println!` in tests only | OK in tests | Never in `src/` production paths |

---

## 4. SOLID for Observability Code

| Principle | Observability application |
|-----------|---------------------------|
| **S** | One module owns subscriber init (`edgequake-observability` or `main` bootstrap only) |
| **O** | Exporters (OTLP, stdout JSON) behind feature flags — no handler edits |
| **L** | All crates use `tracing` macros; no `log` crate mix |
| **I** | `TracingSpanExt` trait for `request_id` injection — thin middleware API |
| **D** | Handlers depend on `tracing::Span::current()`, not OTEL types |

**DRY violation today:** Three parallel HTTP instrumentation layers in API:

1. `request_logging` (`middleware.rs`)
2. `request_id` (`middleware.rs`)
3. `TraceLayer::new_for_http()` (`server.rs:93`)

They do not share context — triple cost, zero correlation.

---

## 5. Correlation Standards

### W3C Trace Context (target)

```
traceparent: 00-{trace-id}-{parent-span-id}-01
tracestate:  (vendor optional)
```

### EdgeQuake MVP (until OTEL shipped)

```
x-request-id: {uuid}     # already generated server-side
x-correlation-id: {uuid} # optional client alias
x-tenant-id / x-workspace-id  # already from WebUI
```

**Rule:** Server must **accept** inbound `x-request-id` / `traceparent` if present; only generate if absent (`middleware.rs` always overwrites — **bug**).

---

## 6. Evidence Standard

```
ID:       OBS-{CRATE}-{NNN} or OBS-P0-NNN
Priority: P0–P3
Claim:    One factual sentence
Evidence: path:line or ripgrep count
Impact:   Operator / SRE consequence
Fix:      Concrete change (crate, module, env var)
Verify:   Command or log grep proving fix
```

---

## 7. External Boundary: edgequake-llm

Outbound LLM calls support `with_extra_headers()` (v0.6.16). EdgeQuake API exposes `QueryRequest.extra_headers` but does **not** auto-harvest incoming HTTP headers into that field — correlation requires **explicit client JSON** today.

See [014-edgequake-llm](../014-edgequake-llm/001-audit.md).
