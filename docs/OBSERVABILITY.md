# EdgeQuake Observability

See [SPEC-018](../specs/018-observability/README.md) for the full audit and proof index.

## Environment variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_LOG` | Tracing filter | `edgequake=info,...` |
| `EDGEQUAKE_LOG_FORMAT` | `json` or plain | plain |
| `EDGEQUAKE_LOG_SPAN_EVENTS` | `1` / `true` — log span close events (duration) | off |
| `EDGEQUAKE_DB_POOL_METRICS_INTERVAL_SECS` | DB pool gauge sampling interval (min 5) | `15` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP gRPC endpoint | (disabled) |
| `OTEL_SERVICE_NAME` | Service name in traces | `edgequake-api` |
| `EDGEQUAKE_OTEL_ENABLED` | `1` / `true` to enable OTLP layer | off |

**Build OTLP:** compile with `edgequake-observability` feature `otel` (not in default workspace build):

```bash
# Release binary with OTLP + W3C parent linking
cargo build -p edgequake --features otel,postgres
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export EDGEQUAKE_LOG_FORMAT=json
```

Domain metrics:

| Metric | Labels |
|--------|--------|
| `edgequake_http_*` | method, path, status |
| `edgequake_query_*` | mode, outcome |
| `edgequake_llm_*` | provider, operation, outcome |
| `edgequake_document_processing_*` | task_type, stage, outcome |
| `edgequake_storage_errors_total` | category, error_code |
| `edgequake_pipeline_errors_total` | category, error_code |
| `edgequake_db_pool_connections` | state=total\|idle\|active |
| `edgequake_rate_limit_exceeded_total` | scope |

DB pool gauges update every 15s (configurable via `EDGEQUAKE_DB_POOL_METRICS_INTERVAL_SECS`) and on each `/metrics` scrape.

## Docker

```bash
# OTLP-enabled image + Jaeger UI (recommended — observability overlay)
cd edgequake/docker
docker compose -f docker-compose.yml -f docker-compose.observability.yml \
  --profile observability up --build

# Jaeger UI: http://localhost:16686
```

The overlay sets `ENABLE_OTEL=true`, JSON logs, span-close events, and `OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317`.

Domain metrics include `edgequake_db_pool_connections` (sampled on each `/metrics` scrape when PostgreSQL is enabled).

## Log levels (when errors happen)

| Situation | Level | Fields |
|-----------|-------|--------|
| HTTP 5xx (`ApiError`) | `error!` | `request_id`, `error.code`, `error.message`, `error.source`, `error.details`, `http.status` |
| HTTP 4xx (`ApiError`) | `warn!` | same |
| HTTP transport (middleware) | `debug!` | status + duration only (body logged by `ApiError`) |
| SOTA query pipeline failure | `warn!` | `#[instrument(err)]` on `run_query_pipeline` |
| Sync query early `?` return | `warn!` / `error!` | `ApiError::into_response` only (guard = metrics) |
| Stream query/chat in-SSE failure | `error!` | `ErrorEvent::log_stream_error(source, …)` + `phase` |
| SSE client disconnect | `info!` | `ErrorEvent::log_stream_disconnect` (not a server error) |
| WebSocket transport failure | `error!`/`warn!` | `log_domain_error/warn("websocket", action, …)` |
| Task worker queue/storage | `error!` | Structured `error.source=task_worker`, `task_process` span |
| Startup recovery (non-fatal) | `warn!` | `log_domain_warn("startup", action, …)` |
| Auth login/refresh failure | `warn!` | `ApiError::auth_unauthorized` → `details.diagnostics` (`action`, `reason`, `subject`) — **one log line** |
| JWT verify failure | `warn!` | `error.code`, `error.source=jwt` in edgequake-auth |
| HTTP 5xx OTEL span status | `ERROR` | `Status::error` on span |
| HTTP 4xx OTEL span status | `OK` | Client errors keep span fields but avoid false ERROR in Jaeger |

JSON logs include `span` / `spans` when `EDGEQUAKE_LOG_FORMAT=json`. API error JSON `details` includes `request_id`, `error_code`, `diagnostics`, and `retryable`.

`retryable` is **smart**: true for rate limits, timeouts, transient storage/DB, LLM timeouts/overload, pipeline circuit-breaker — false for auth errors, not-found, invalid API keys.

## Correlation headers

| Header | Direction | Notes |
|--------|-----------|-------|
| `X-Request-ID` | Client → API → response | WebUI generates per request |
| `traceparent` | WebUI → API → response → WebUI | W3C format; new span id per request, trace id chained via `sessionStorage` |
| `X-Tenant-ID` / `X-Workspace-ID` | WebUI → API | Existing multitenancy |

## Endpoints

- `GET /metrics` — Prometheus text (live `edgequake_http_requests_total` after traffic)
- Health family — `/health`, `/ready`, `/live`

## Proof script

```bash
make observability-proof
# or: ./specs/018-observability/e2e/run_observability_proof.sh
```

## Trace spans (OTLP / JSON logs)

| Span | Crate | Fields |
|------|-------|--------|
| `http_request` | edgequake-api | `request_id`, `trace_id`, `http.method`, `error.*` |
| `query_execute` | edgequake-api | `request_id`, `query.mode` |
| `query_stream` | edgequake-api | `request_id`, `query.mode`, `stream.format` |
| `chat_stream` | edgequake-api | `request_id`, `query.mode` |
| `sota_query_pipeline` | edgequake-query | pipeline phases |
| `task_process` | edgequake-tasks | `task_id`, `tenant_id`, `task_type` |
| `pipeline_chunk_extraction` | edgequake-pipeline | chunk index |
