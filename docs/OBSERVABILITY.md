---
title: EdgeQuake Observability
---

# EdgeQuake Observability

> **Product: v0.23.0** · Spec: [SPEC-018](../specs/018-observability/README.md) · Ingestion ops: [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md)

See [SPEC-018](../specs/018-observability/README.md) for the full audit and proof index.

## Environment variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_LOG` | Tracing filter | `edgequake=info,...` |
| `EDGEQUAKE_LOG_FORMAT` | `json` or plain | plain |
| `EDGEQUAKE_LOG_SPAN_EVENTS` | `1` / `true` — log span close events (duration) | off |
| `EDGEQUAKE_DB_POOL_METRICS_INTERVAL_SECS` | DB pool gauge sampling interval (min 5) | `15` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP gRPC endpoint (Jaeger / collector) | (disabled) |
| `OTEL_SERVICE_NAME` | Service name in traces | `edgequake-api` |
| `EDGEQUAKE_OTEL_ENABLED` | `1` / `true` to enable OTLP gRPC layer | off |
| `LANGFUSE_PUBLIC_KEY` | Langfuse public key (`pk-lf-…`) — SPEC-124 | (disabled) |
| `LANGFUSE_SECRET_KEY` | Langfuse secret key (`sk-lf-…`) — never logged | (disabled) |
| `LANGFUSE_BASE_URL` | Langfuse UI + OTLP base (alias `LANGFUSE_HOST`) | `https://cloud.langfuse.com` |
| `EDGEQUAKE_LANGFUSE_ENABLED` | Force on (`1`) / off (`0`); default = both keys set | auto |
| `EDGEQUAKE_LANGFUSE_API` | `auto` (probe OTLP, ingest on 404) / `otlp` / `ingestion` | `auto` |
| `EDGEQUAKE_PROMPT_CACHE` | Provider KV/prompt-cache (SPEC-126); observation meta `cache_hit_tokens` | on |
| `EDGEQUAKE_ENVIRONMENT` | `deployment.environment` on traces | `development` |
| `EDGEQUAKE_QUEUE_PENDING_WARN` | Pending depth → elevated queue pressure | `100` |
| `EDGEQUAKE_QUEUE_PENDING_CRITICAL` | Pending depth → critical; `/health` degraded | `500` (or 5× warn) |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_WARN` | Quarantine counter → elevated store contention | `1` |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_CRITICAL` | Quarantine counter → `/ready` 503 | `5` |
| `EDGEQUAKE_DB_POOL_UTIL_WARN` | Pool utilization → elevated store contention | `0.75` |
| `EDGEQUAKE_DB_POOL_UTIL_CRITICAL` | Pool utilization → critical store contention | `0.90` |

**Build OTLP:** the `otel` feature is **on by default** (SPEC-124) for the workspace binary, `edgequake-api`, and `edgequake-observability`. Export still needs runtime env (`LANGFUSE_*` and/or `OTEL_EXPORTER_OTLP_ENDPOINT`). To build without OTLP:

```bash
cd edgequake && cargo build --release --no-default-features --features postgres,vision
```

Default build (includes OTLP + Langfuse-ready HTTP exporter):

```bash
# Release binary — otel is already in default features
cd edgequake && cargo build --release
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317   # optional Jaeger
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
| `edgequake_compensation_quarantine_total` | kind | v0.23 |
| `edgequake_ingestion_failures_total` | failure_class, workspace | v0.16+ |
| `edgequake_ingestion_chunk_strategy_total` | strategy | v0.16+ |
| `edgequake_graph_quality_*` | — | v0.17+ |
| `edgequake_faithfulness_*` | — | v0.17+ |
| `edgequake_query_sparse_retrieval_total` | backend | v0.17+ |
| `edgequake_storage_drift_*` | severity | v0.18+ |

DB pool gauges update every 15s (configurable via `EDGEQUAKE_DB_POOL_METRICS_INTERVAL_SECS`) and on each `/metrics` scrape.

### Queue pressure & store contention (v0.23)

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

## Langfuse (SPEC-124)

**Self-hosted 3.1.x (no OTLP):** step-by-step wiring, local `:3320` stack, Helm, and verify `api_resolved=ingestion` — **[operations/langfuse-3.1.md](operations/langfuse-3.1.md)**. Kubernetes: [deploy/kubernetes/README.md](../deploy/kubernetes/README.md#existing-langfuse-31x).

Langfuse Cloud and self-hosted **≥ 3.22** accept **OTLP/HTTP** at `{LANGFUSE_BASE_URL}/api/public/otel/v1/traces` (not gRPC). When `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY` are set, EdgeQuake registers a BatchSpanProcessor on that path (`otel` is on by default). Programmatic OTLP exporters must use the full `/v1/traces` path — the SDK does not append it when `with_endpoint` is set.

Self-hosted **Langfuse 3.1.x** returns **404** on that OTLP path (added in 3.22.0). Default `EDGEQUAKE_LANGFUSE_API=auto` probes once at startup and falls back to `POST /api/public/ingestion` **only on HTTP 404**. That path maps LAW-124-13 types (`retriever` / `embedding` / `chain`) onto 3.1.1 envelope types (`span-create` / `generation-create`) — never `{type}-create` stringification. Ingestion is a **bridge**, not a replacement: Cloud sunsets it 2026-11-16; upgrade to **≥ 3.22** (or the in-repo v4 compose) remains recommended.

```bash
# Preferred: uncomment in repo-root `.env` (make dev sources it)
# Prefer unquoted values. Quoted Langfuse UI paste is OK — Make/Rust strip one "..." pair
# (Make `-include` otherwise keeps quotes and OTLP Basic auth gets HTTP 401).
LANGFUSE_PUBLIC_KEY=pk-lf-...
LANGFUSE_SECRET_KEY=sk-lf-...
LANGFUSE_BASE_URL=https://cloud.langfuse.com   # or US / self-hosted / http://localhost:3310
# EDGEQUAKE_LANGFUSE_API=auto   # otlp | ingestion | auto (default)
# Same shell alternative: export the three vars, then restart
make kill-app && make backend-bg   # or: make dev
# Docker stack: LANGFUSE_* is mapped in compose (quickstart / docker / api-only / prebuilt)
#
# Local Langfuse v4 (optional, not started by make dev):
#   make dev-langfuse / make dev-bg-langfuse
#     full EdgeQuake stack + isolated Langfuse v4 (UI http://localhost:3310)
#     injects Compose init keys into the backend (overrides .env Cloud/placeholder)
#   make langfuse-up          # UI only
#   make langfuse-smoke       # GET /api/public/projects with headless init keys
#   make spec124-langfuse-e2e # one-command: Langfuse + stack + Settings/sessions Playwright
#     Sessions test needs a working /api/v1/query (Ollama or OPENAI_API_KEY)
# Headless keys (must match edgequake/docker/docker-compose.langfuse.yml):
#   LANGFUSE_PUBLIC_KEY=pk-lf-edgequake-local
#   LANGFUSE_SECRET_KEY=sk-lf-edgequake-local-dev
#   LANGFUSE_BASE_URL=http://localhost:3310
#   LANGFUSE_PROJECT_ID=edgequake-local
# Login: dev@example.com / edgequake-local-dev
# make langfuse-down keeps volumes; make langfuse-reset CONFIRM=yes wipes them.
# make stop does not tear Langfuse down.
#
# Langfuse 3.1.x (how-to: docs/operations/langfuse-3.1.md):
#   make langfuse-3.1-up          # isolated 3.1.1 UI :3320
#   make spec124-langfuse-3.1-e2e # unfakable ingestion-fallback proof
# Langfuse ≥ 3.22 OTLP (OTLP starts at 3.22.0, not 3.2):
#   make langfuse-3.22-up            # isolated 3.22.0 UI :3330 (route+probe)
#   make spec124-langfuse-3.22-e2e
#   make langfuse-3.225-up           # isolated 3.225.5 UI :3340 (OTLP persist)
#   make spec124-langfuse-3.225-e2e
#   make spec124-langfuse-cloud-e2e  # current Cloud (keys in .env)
#   make spec124-langfuse-matrix     # 3.1.1 + 3.22.0 + 3.225.5 + Cloud
# Cost $0.00 on recent models is Langfuse's catalogue, not EdgeQuake (LAW-124-12).
#   make langfuse-sync-prices          # POST /api/public/models
#   make langfuse-sync-prices FORCE=1  # PUT existing rows
```

`make dev` / `backend-bg` / `backend-dev` call `APPLY_LANGFUSE_ENV`: source repo-root `.env`, apply Make/CLI overrides only when the shell var is empty (so bash-sourced values are not clobbered by Make-quoted includes), strip matching quotes, and never force `LANGFUSE_*=""`. Look for `LANGFUSE_* keys detected` in the make output.

- Settings → **Langfuse Observability** card shows status + **Open in Langfuse** (no secrets in UI).
- `GET /api/v1/settings/langfuse` returns the same status DTO.
- `/health.operational.observability.langfuse_enabled` + `langfuse_base_url` + `langfuse_api` / `langfuse_api_resolved`.
- Query responses may include `trace_id`. Deep links use `{LANGFUSE_BASE_URL}/project/{projectId}/traces/{traceId}` (and sessions `{base}/project/{id}/sessions/{sessionId}`). Bare `/sessions/{id}` is a Cloud 404.
- **Sessions:** chat turns bind durable `conversation_id` as Langfuse session / `gen_ai.conversation.id` (see [specs/124-langfuse-support/12-sessions-and-genai.md](../specs/124-langfuse-support/12-sessions-and-genai.md)). After two turns in the same conversation, open Langfuse → Observability → Sessions. Optional `/query` and `/query/stream` field `session_id` for API clients; never invent a session when omitted.
- **Tokens yes / cost never:** generation and embedding spans record `gen_ai.usage.input_tokens` / `output_tokens` when the LLM returns counts. EdgeQuake **never** emits `gen_ai.usage.cost` or `langfuse.observation.cost_details`. Observation types: `generation`, `retriever`, `embedding`, `chain` (ingest root). See [13-metadata-tokens-and-coverage.md](../specs/124-langfuse-support/13-metadata-tokens-and-coverage.md).
- **Observation Input/Output:** Langfuse UI reads `langfuse.observation.input` / `output` (not `gen_ai.retrieval.query.text`). **Generation observation I/O is the full LLM prompt + completion** (secret-redacted; **no length truncation by default**). Optional ceiling: set `EDGEQUAKE_LANGFUSE_IO_MAX_BYTES` to a positive byte budget (`0` / unset = unlimited). See [SPEC-145](../specs/145-fix-truncated-logs/). Retriever / embed / ingest stats stay compact Structured JSON; ingest document **content** stays Preview. Mapping details: [14-observation-io-and-full-observe.md](../specs/124-langfuse-support/14-observation-io-and-full-observe.md).
- **SPEC-125 / SPEC-135 `ingest.chunking` distribution:** output JSON is counts only (`chunks`, `token_min`, `token_p50`, `token_max`, `orphan_heading_chunks`, `fill_p50`, `mm_sidecar_appended`) — never chunk text. Same keys are observation metadata. See [specs/125-better-chunking](../specs/125-better-chunking/) and [specs/135-chunking](../specs/135-chunking/).

Jaeger gRPC (`OTEL_EXPORTER_OTLP_ENDPOINT`) remains independent — both exporters can be active.

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
| `rag.retrieval` | edgequake-observability | `gen_ai.operation.name=retrieval`, `langfuse.observation.type=retriever`, `gen_ai.data_source.id`, `gen_ai.retrieval.top_k`, `rag.retrieval.*` |
| `rag.generation` | edgequake-observability | `gen_ai.operation.name=chat`, `langfuse.observation.type=generation`, model/provider, `gen_ai.usage.input_tokens` / `output_tokens` (never cost) |
| `rag.embedding` | edgequake-observability | `gen_ai.operation.name=embeddings`, `langfuse.observation.type=embedding` |
| `feature.root` / `ingest.document` | edgequake-observability | `langfuse.observation.type=chain`, `langfuse.trace.tags=ingest` |
| `task_process` | edgequake-tasks | `task_id`, `tenant_id`, `task_type` |
| `pipeline_chunk_extraction` | edgequake-pipeline | chunk index |

**GenAI / RAG spans (v0.17+):** Mix/Hybrid query arms and `pipeline_retrieve` wrap retrieval in `rag.retrieval` via `with_rag_retrieval_span`. LLM generation uses `with_rag_generation_span`. Spans emit in JSON logs always; OTLP export requires the `otel` feature and `EDGEQUAKE_OTEL_ENABLED=1`.
