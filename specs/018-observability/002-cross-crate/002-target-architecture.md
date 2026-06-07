# Target Observability Architecture

**Spec:** 018-observability  
**Date:** 2026-06-05  
**Status:** Target state (not implemented)

---

## 1. End-to-End Correlation Flow

```
┌──────────────┐     traceparent / x-request-id      ┌─────────────────────┐
│  WebUI       │ ─────────────────────────────────▶ │  edgequake-api      │
│  (browser)   │     X-Tenant-ID / X-Workspace-ID   │  OtelAxumLayer      │
│              │                                     │  + request_id ext   │
└──────┬───────┘                                     └──────────┬──────────┘
       │                                                        │
       │  @vercel/otel or                                       │  tracing span
       │  @opentelemetry/sdk-trace-web (optional)               │  request_id field
       │                                                        │
       ▼                                                        ▼
┌──────────────┐                                     ┌─────────────────────┐
│  Browser     │                                     │  edgequake-core     │
│  RUM traces  │                                     │  pipeline / query   │
└──────────────┘                                     └──────────┬──────────┘
                                                                │
                    ┌───────────────────────────────────────────┼───────────────┐
                    │                                           │               │
                    ▼                                           ▼               ▼
           ┌──────────────┐                            ┌──────────────┐  ┌──────────────┐
           │  PostgreSQL  │                            │ edgequake-llm│  │ edgequake-   │
           │  (sqlx span) │                            │ traceparent  │  │ audit        │
           │              │                            │  injected    │  │ request_id   │
           └──────────────┘                            └──────────────┘  └──────────────┘
                    │                                           │
                    └─────────────────────┬─────────────────────┘
                                          ▼
                              ┌───────────────────────┐
                              │  OTel Collector         │
                              │  (gRPC 4317)            │
                              └───────────┬───────────┘
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    ▼                     ▼                     ▼
              ┌──────────┐         ┌──────────┐         ┌──────────┐
              │  Tempo   │         │  Loki    │         │Prometheus│
              │  traces  │         │  logs    │         │ metrics  │
              └──────────┘         └──────────┘         └──────────┘
```

---

## 2. Rust Workspace Layering (SOLID)

Proposed crate: **`edgequake-observability`** (or `edgequake-otel`):

```
edgequake-observability/
├── subscriber.rs      # init: EnvFilter + fmt/json + otel layer
├── propagation.rs     # W3C extract/inject for axum + reqwest
├── fields.rs          # DRY: request_id!, tenant_id! span extensions
└── metrics.rs         # prometheus registry (shared with /metrics handler)
```

| Consumer | Depends on | Does NOT |
|----------|------------|----------|
| `edgequake-api` | `observability` init + axum layers | Import `opentelemetry_sdk` directly |
| Other crates | `tracing` only | Configure global subscriber |
| `main.rs` | Calls `observability::init()` once | Duplicate fmt layer |

**OCP:** `EDGEQUAKE_OTEL_ENABLED=1` adds OTLP exporter without recompiling handlers.

---

## 3. Recommended Crates (2026)

| Concern | Crate | Role |
|---------|-------|------|
| Tracing bridge | `tracing-opentelemetry` | `tracing` span → OTEL span |
| Axum | `axum-tracing-opentelemetry` or `tower-http` + manual propagator | Extract `traceparent` |
| Export | `opentelemetry-otlp` | gRPC to collector |
| Metrics | `metrics` + `metrics-exporter-prometheus` | Replace static `/metrics` |
| Logs | `tracing-subscriber` json feature | Already in `api/Cargo.toml:39` |

Reference: [Uptrace propagation guide](https://uptrace.dev/get/opentelemetry-rust/propagation), `axum-tracing-opentelemetry` 0.33.x.

---

## 4. Middleware Order (Axum)

Correct layer order (outer → inner):

```
1. OtelAxumLayer          ← extract traceparent, start server span
2. request_id             ← honor inbound OR generate; set span field
3. rate_limit             ← existing
4. TraceLayer             ← optional: remove if redundant with OTEL
5. request_logging        ← demote to DEBUG or remove
6. Router (handlers)
7. OtelInResponseLayer    ← inject traceparent in response (B2B)
```

**DRY rule:** One layer owns HTTP server spans.

---

## 5. Log-Trace Correlation

Every production log line should carry (via `tracing` span):

```json
{
  "timestamp": "...",
  "level": "INFO",
  "target": "edgequake_api::handlers::query",
  "message": "Query completed",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "tenant_id": "t1",
  "workspace_id": "w1",
  "duration_ms": 842
}
```

Implementation: `tracing_opentelemetry::OpenTelemetrySpanExt` + custom `RequestIdLayer`.

---

## 6. WebUI Target

| Piece | Technology |
|-------|------------|
| Outbound API | Generate `x-request-id` per `apiClient` call; forward `traceparent` if RUM active |
| Server components | Optional `instrumentation.ts` (Next.js 16) |
| Errors | Map `x-request-id` from `ApiRequestError` into toast/support UI |
| Dev only | Keep `console.debug` behind `process.env.NODE_ENV === 'development'` |

File: `edgequake_webui/src/lib/api/client.ts` — single DRY header builder.

---

## 7. API ↔ OTEL Compatibility Checklist

| W3C / OTEL requirement | Target implementation |
|------------------------|----------------------|
| Propagate `traceparent` | Middleware extract + LLM inject |
| Propagate `tracestate` | Pass-through if present |
| Baggage (`tenant`) | `x-tenant-id` already; map to OTEL baggage optional |
| Span kind Server on HTTP | `OtelAxumLayer` |
| Span kind Client on LLM | `TracedClient` wrapper on edgequake-llm HTTP |
| `service.name` resource | `edgequake-api`, `edgequake-webui` |
| Semantic conventions | `http.route`, `http.status_code`, `db.system` |

---

## 8. Metrics Catalog (Prometheus)

Replace placeholder (`metrics.rs`) with live collectors:

| Metric | Type | Labels |
|--------|------|--------|
| `edgequake_http_requests_total` | counter | method, route, status |
| `edgequake_http_request_duration_seconds` | histogram | method, route |
| `edgequake_documents_total` | counter | status |
| `edgequake_queries_total` | counter | mode |
| `edgequake_query_duration_seconds` | histogram | mode |
| `edgequake_tasks_total` | counter | status |
| `edgequake_llm_requests_total` | counter | provider, model |
| `edgequake_llm_tokens_total` | counter | type |

Instrument at: middleware (HTTP), `DocumentTaskProcessor`, `SOTAQueryEngine`, `SafetyLimitedProviderWrapper`.
