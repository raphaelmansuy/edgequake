# Observability Remediation Roadmap

**Spec:** 018-observability  
**Date:** 2026-06-05

---

## Phase 0 — Honest Baseline (1–2 days)

| Task | Fixes | Verify |
|------|-------|--------|
| Update `docs/operations/monitoring.md` | Mark OTEL/metrics as **not shipped** | Doc review |
| Add `OBSERVABILITY.md` env table | `EDGEQUAKE_LOG_FORMAT=json` | README link |

No code required — stops doc/reality drift (OBS-P0-006).

---

## Phase 1 — Correlation MVP (3–5 days)

### 1.1 Request ID in tracing

```rust
// middleware.rs — pseudocode target
let id = incoming_x_request_id.or_else(generate_uuid);
Span::current().record("request_id", id.as_str());
request.extensions_mut().insert(RequestId(id.clone()));
```

**Fixes:** OBS-P0-002, OBS-P1-003 (partial)

### 1.2 Honor inbound ID

Do not overwrite client `x-request-id` when valid UUID.

**Fixes:** OBS-P0-003 (server half)

### 1.3 WebUI headers

```typescript
// client.ts buildHeaders()
headers.set("X-Request-ID", crypto.randomUUID());
```

**Fixes:** OBS-P0-003

### 1.4 JSON logs in production

```rust
// main.rs — when EDGEQUAKE_LOG_FORMAT=json
.with(tracing_subscriber::fmt::layer().json())
```

**Fixes:** OBS-P0-006

**Verify:**

```bash
EDGEQUAKE_LOG_FORMAT=json RUST_LOG=info make backend-bg
curl -H "X-Request-ID: test-123" http://localhost:8080/health
# grep logs for "test-123"
```

---

## Phase 2 — OpenTelemetry (1–2 weeks)

| Step | Work |
|------|------|
| Add workspace deps | `tracing-opentelemetry`, `opentelemetry-otlp`, `axum-tracing-opentelemetry` |
| `edgequake-observability` crate | Single `init_tracing()` |
| Axum layers | `OtelAxumLayer` + response injection |
| Env | `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME=edgequake-api` |
| Auto header → LLM | Middleware harvest `traceparent` + `x-request-id` into resolver `extra_headers` |

**Fixes:** OBS-P0-001, OBS-P1-001, OBS-P1-002

**Verify:** Trace visible in Jaeger for `POST /api/v1/query` including child LLM span.

---

## Phase 3 — Metrics & Errors (1 week)

| Step | Work |
|------|------|
| `metrics` crate | Wire counters/histograms |
| Replace `get_metrics()` stub | Scrape live registry |
| `ApiError` logging | `error!` in `IntoResponse` with request_id |
| Demote HTTP 200 logs | `request_logging` → DEBUG |

**Fixes:** OBS-P0-004, OBS-P1-004

---

## Phase 4 — Compliance Audit (3–5 days)

| Step | Work |
|------|------|
| Wire `AuditLogger` in `AppState` | Login, upload, delete, query |
| Pass `request_id` from extensions | `AuditEvent.request_id` |
| Implement `query_audit_logs` | Remove placeholder `Ok(vec![])` |

**Fixes:** OBS-P0-005, OBS-P2-004

---

## Phase 5 — WebUI OTEL (optional, 1 week)

| Step | Work |
|------|------|
| `instrumentation.ts` | Next.js server OTEL |
| Browser RUM | `@vercel/otel` or Honeycomb browser SDK |
| Strip prod `console.*` | ESLint `no-console` warn |

**Fixes:** OBS-P2-003, OBS-P3-002

---

## Acceptance Criteria (Definition of Done)

- [ ] Single trace for UI query: browser → API → LLM → DB child spans
- [ ] Logs in Loki: `{trace_id="$id"}` returns full story
- [ ] `/metrics` counters increment on health + query traffic
- [ ] `x-request-id` on response matches logs and audit row
- [ ] `cargo test -p edgequake-api` + Playwright smoke pass
- [ ] No new `println!` in `src/`

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| OTEL overhead | Sample 10% prod via `OTEL_TRACES_SAMPLER=parentbased_traceidratio` |
| Log volume | Remove duplicate TraceLayer + request_logging |
| Breaking SDK clients | Keep `x-request-id`; add `traceparent` additive |
| sqlx span noise | `RUST_LOG=sqlx=warn` default |
