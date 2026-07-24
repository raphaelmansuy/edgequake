---
title: EdgeQuake Observability
---

# EdgeQuake Observability

> **Product: v0.19.0** · Spec: [SPEC-018](../specs/018-observability/README.md) · Ingestion ops: [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md)

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
| `EDGEQUAKE_QUEUE_PENDING_WARN` | Pending depth → elevated queue pressure | `100` |
| `EDGEQUAKE_QUEUE_PENDING_CRITICAL` | Pending depth → critical; `/health` degraded | `500` (or 5× warn) |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_WARN` | Quarantine counter → elevated store contention | `1` |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_CRITICAL` | Quarantine counter → `/ready` 503 | `5` |
| `EDGEQUAKE_DB_POOL_UTIL_WARN` | Pool utilization → elevated store contention | `0.75` |
| `EDGEQUAKE_DB_POOL_UTIL_CRITICAL` | Pool utilization → critical store contention | `0.90` |

**Build OTLP:** compile the workspace binary with the `otel` feature (not in default build). The feature wires `edgequake-observability/otel` and `edgequake-api/otel`:

```bash
# Release binary with OTLP + W3C parent linking
cd edgequake && cargo build --release --features otel,postgres
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export EDGEQUAKE_OTEL_ENABLED=1
export EDGEQUAKE_LOG_FORMAT=json
```

Domain metrics (Prometheus `GET /metrics`):

| Metric | Labels | Since |
|--------|--------|-------|
| `edgequake_http_*` | method, path, status | — |
| `edgequake_query_*` | mode, outcome | — |
| `edgequake_llm_*` | provider, operation, outcome | — |
| `edgequake_document_processing_*` | task_type, stage, outcome | — |
| `edgequake_storage_errors_total` | category, error_code | — |
| `edgequake_pipeline_errors_total` | category, error_code | — |
| `edgequake_db_pool_connections` | state=total\|idle\|active | — |
| `edgequake_rate_limit_exceeded_total` | scope | — |
| `edgequake_task_queue_pending` | — | v0.16+ |
| `edgequake_task_queue_processing` | — | v0.16+ |
| `edgequake_task_queue_failed` | — | v0.16+ |
| `edgequake_compensation_quarantine_total` | kind | v0.19 |
| `edgequake_ingestion_failures_total` | failure_class, workspace | v0.16+ |
| `edgequake_ingestion_chunk_strategy_total` | strategy | v0.16+ |
| `edgequake_graph_quality_*` | — | v0.17+ |
| `edgequake_faithfulness_*` | — | v0.17+ |
| `edgequake_query_sparse_retrieval_total` | backend | v0.17+ |
| `edgequake_storage_drift_*` | severity | v0.18+ |

DB pool gauges update every 15s (configurable via `EDGEQUAKE_DB_POOL_METRICS_INTERVAL_SECS`) and on each `/metrics` scrape.

### Queue pressure & store contention (v0.19)

`GET /api/v1/pipeline/queue-metrics` is the operator SSOT for backlog and ingestion health:

| Field | Meaning |
|-------|---------|
| `pressure` | `normal` \| `elevated` \| `critical` (from pending depth vs `EDGEQUAKE_QUEUE_PENDING_*`) |
| `store_contention.level` | Pool utilization + `compensation_quarantine_total` composite |
| `tenant_park_waiters` | Tasks parked on tenant fairness semaphore |
| `cancel_intent_count` | In-flight cancel intents (process-local accelerator) |

When `pressure=critical`, `/health` reports **degraded**. When store contention is **critical**, `/ready` returns **503**. Inspect KV DLQ keys `compensation_quarantine:{document_id}:*` when `edgequake_compensation_quarantine_total` rises.

See [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md#store-contention--compensate-dlq-spec-057-p3) for thresholds and remediation.

## Docker

```bash
# OTLP-enabled image + Jaeger UI (recommended — observability overlay)
cd edgequake/docker
docker compose -f docker-compose.yml -f docker-compose.observability.yml \
  --profile observability up --build

# Jaeger UI: http://localhost:16686
```

The overlay sets `ENABLE_OTEL=true`, JSON logs, span-close events, and `OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317`.

Domain metrics include `edgequake_db_pool_connections` and task-queue gauges (sampled on each `/metrics` scrape when PostgreSQL is enabled).

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
| Queue backlog critical | `error!` | `target=edgequake.task_queue`, `pressure=critical` |
| Store contention elevated/critical | `warn!`/`error!` | `store_contention` in queue-metrics |
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

- `GET /metrics` — Prometheus text (live counters after traffic)
- `GET /api/v1/pipeline/queue-metrics` — queue pressure, store contention, fairness park waiters
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
| `rag.retrieval` | edgequake-observability | `gen_ai.operation.name=retrieval`, `gen_ai.data_source.id`, `gen_ai.retrieval.top_k`, `rag.retrieval.arm`, `rag.retrieval.empty_result`, `rag.context.truncated`, `rag.retrieval.fallback` |
| `rag.generation` | edgequake-observability | `gen_ai.operation.name=chat`, `gen_ai.request.model`, `gen_ai.provider.name` |
| `task_process` | edgequake-tasks | `task_id`, `tenant_id`, `task_type` |
| `pipeline_chunk_extraction` | edgequake-pipeline | chunk index |

**GenAI / RAG spans (v0.17+):** Mix/Hybrid query arms and `pipeline_retrieve` wrap retrieval in `rag.retrieval` via `with_rag_retrieval_span`. LLM generation uses `with_rag_generation_span`. Spans emit in JSON logs always; OTLP export requires the `otel` feature and `EDGEQUAKE_OTEL_ENABLED=1`.
