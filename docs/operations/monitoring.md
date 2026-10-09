---
title: "Monitoring guide"
description: "Health endpoints, Prometheus metrics, logs, tracing, alerts and PostgreSQL checks for running EdgeQuake in production."
---

# Monitoring guide

This page is for operators who watch EdgeQuake in production. It shows what to probe, what to scrape, what to alert on, and how to read the answer when something is wrong. For slow systems also read [Performance tuning](performance-tuning.md).

## What EdgeQuake exposes

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  A["EdgeQuake API"] --> H["Probes: /live /ready /health"]
  A --> M["Prometheus: /metrics"]
  A --> L["Logs on stdout"]
  A --> T["Traces: OTLP, Langfuse"]
  A --> Q["Queue view: /pipeline/queue-metrics"]
  H --> LB["Load balancer, Kubernetes"]
  M --> P["Prometheus, Grafana"]
  L --> LG["Loki or ELK"]
  T --> J["Jaeger, Langfuse"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class Q eqStore
```

How to read it: each box on the left of an arrow is a signal. The box it points to is the usual consumer. Pick the signals you need. Probes and logs need no extra setup.

## Health endpoints

| Endpoint | Meaning | HTTP status | Use it for |
|----------|---------|-------------|-----------|
| `GET /live` | The process is up. | 200 | Liveness probe and the Docker healthcheck. |
| `GET /ready` | The API can take traffic. | 200, or 503 with blockers | Readiness probe and load balancer. |
| `GET /health` | Detailed status. | Always 200 | Dashboards and people. |
| `GET /api/v1/pipeline/queue-metrics` | Ingest backlog and fairness. | 200 | Capacity checks. |

`/live`, `/ready` and `/health` need no login. `/metrics` and `/api/v1/pipeline/queue-metrics` do when auth is on. Give Prometheus an API key or bearer token.

### /health

```bash
curl -s http://localhost:8080/health | jq .
```

```json
{
  "status": "healthy",
  "version": "0.32.2",
  "storage_mode": "postgresql",
  "workspace_id": "default",
  "components": {"kv_storage": true, "vector_storage": true, "graph_storage": true, "llm_provider": true},
  "llm_provider_name": "ollama"
}
```

`status` is `healthy` or `degraded`. Extra blocks appear when relevant: `schema` (latest applied migration, pending count), `providers`, `operational`, `capabilities`, `attribution`, `build_info`, and since v0.33.0 `security_posture`.

`security_posture` shows what the API decided at boot:

| Field | Meaning |
|-------|---------|
| `auth_enabled`, `dev_mode` | Authentication and open mode. |
| `secrets_key_configured` | `EDGEQUAKE_SECRETS_KEY` is set. |
| `jwt_secret_is_default` | You are still on the shipped JWT secret. Fix before production. |
| `rate_limit_enabled` | Rate limiting is on. |
| `swagger_enabled` | Swagger UI is served. |

The web UI shows `degraded` as a "Busy" pill and polls every 5 seconds until it clears.

### /ready

```json
{ "ready": false, "blockers": ["store_contention_critical(pool_util=Some(0.92),quarantine=6)"], "operator_action": "Scale DB pool or reduce ingest; inspect compensation quarantine DLQ" }
```

A ready API returns 200 with `"ready": true` and an empty `blockers` list.

| Blocker | Cause | Fix |
|---------|-------|-----|
| `migration_038`, `migration_042`, `missing_hnsw_index`, `pgvector_cve_floor`, `eq_id_schema`, `migration_092` and other `migration_NNN` | A schema or index step is missing or degraded. | Run `edgequake migrate` (see [Upgrading](upgrading.md)). For `pgvector_cve_floor` upgrade pgvector to 0.8.2 or newer. |
| `storage_ping_failed(...)` | KV, vector or graph ping failed or timed out. | Check `DATABASE_URL` and pool saturation. |
| `task_queue_critical(pending=N)` | Backlog above the critical level. | Raise `WORKER_THREADS` or slow ingestion. |
| `store_contention_critical(...)` | Pool use above 0.90 or compensation quarantine above 5. | Resize pools. Inspect the quarantine (see [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md)). |
| `wave2_ann_probe_error(...)` or a missing catalog ANN index | Vector index check failed. | `POST /api/v1/admin/ann/warmup`, or fix pgvector access. |

In `EDGEQUAKE_SCHEMA_GATE=wait` mode, `/ready` returns 503 until migrate finishes (see [Upgrading](upgrading.md#4-what-happens-at-api-boot)).

A separate HTTP 503 with code `read_path_busy` means the catalog read deadline expired (documents, search, tenants, workspaces). It carries `Retry-After` and does not mean `/ready` failed. See [common issues](../troubleshooting/common-issues.md).

### Queue metrics

```bash
curl -s http://localhost:8080/api/v1/pipeline/queue-metrics | jq .
```

| Field | Meaning |
|-------|---------|
| `pressure` | `normal`, `elevated` or `critical`. Matches the `/ready` queue gate. |
| `tenant_park_waiters` | Tasks waiting on the fairness limit. Expected with local LLMs. |
| `cancel_intent_count`, `cancel_intent_total` | Cancels in flight, and the lifetime total. |
| `max_tasks_per_tenant` | Effective cap. Local providers clamp it to 1. |
| `store_contention.level`, `.db_pool_utilization`, `.compensation_quarantine_total` | Database pressure and failed merge clean-ups. |

Many waiters with a quiet quarantine is fairness doing its job. A rising quarantine points at AGE or pgvector delete errors.

## Prometheus metrics

Scrape `GET /metrics`. Metric names start with `edgequake_`.

| Metric | Type | Labels | Meaning |
|--------|------|--------|---------|
| `edgequake_http_requests_total` | counter | `method`, `path`, `status` | HTTP requests. IDs in paths become `:id`. |
| `edgequake_http_request_duration_seconds` | histogram | `method`, `path` | HTTP latency. |
| `edgequake_query_requests_total`, `edgequake_query_duration_seconds` | counter, histogram | `mode`, `outcome` | RAG queries. |
| `edgequake_llm_requests_total`, `edgequake_llm_request_duration_seconds` | counter, histogram | `provider`, `operation`, `outcome` | LLM calls. |
| `edgequake_document_processing_total`, `edgequake_document_processing_duration_seconds` | counter, histogram | stage and outcome | Ingestion work. |
| `edgequake_ingestion_failures_total` | counter | `failure_class`, `workspace` | Failed documents. |
| `edgequake_task_queue_pending`, `_processing`, `_failed` | gauge | none | Task queue sizes. |
| `edgequake_db_pool_connections` | gauge | `role`, `state` (`total`, `idle`, `active`, `max`) | Pool use per role. |
| `edgequake_storage_errors_total`, `edgequake_pipeline_errors_total` | counter | `category`, `error_code` | Errors by class. |
| `edgequake_rate_limit_exceeded_total` | counter | `scope` | Requests answered 429. |
| `edgequake_compensation_quarantine_total` | counter | `kind` | Merge clean-up failures. |
| `edgequake_vector_ann_index_missing` | gauge | none | Vector tables without an ANN index. |
| `edgequake_storage_drift_violations_total`, `edgequake_storage_drift_critical` | counter, gauge | none | Storage drift checks. |

The list grows with each release. `GET /metrics` is the authority. Quality metrics (`edgequake_faithfulness_*`, `edgequake_citation_*`) exist for answer checks.

### Example alert rules

```yaml
groups:
  - name: edgequake
    rules:
      - alert: EdgeQuakeHighErrorRate
        expr: sum(rate(edgequake_http_requests_total{status=~"5.."}[5m])) / sum(rate(edgequake_http_requests_total[5m])) > 0.01
        for: 5m
        labels: { severity: critical }
        annotations: { summary: "More than 1% of requests fail" }

      - alert: EdgeQuakeSlowRequests
        expr: histogram_quantile(0.99, sum by (le) (rate(edgequake_http_request_duration_seconds_bucket[5m]))) > 2
        for: 10m
        labels: { severity: warning }
        annotations: { summary: "p99 latency above 2 seconds" }

      - alert: EdgeQuakePoolNearlyFull
        expr: max by (role) (edgequake_db_pool_connections{state="active"} / on(role) edgequake_db_pool_connections{state="max"}) > 0.8
        for: 5m
        labels: { severity: warning }
        annotations: { summary: "A connection pool is above 80% use" }

      - alert: EdgeQuakeQueueBacklog
        expr: edgequake_task_queue_pending > 100
        for: 15m
        labels: { severity: warning }
        annotations: { summary: "More than 100 tasks waiting" }

      - alert: EdgeQuakeLlmFailures
        expr: sum(rate(edgequake_llm_requests_total{outcome="failure"}[5m])) / sum(rate(edgequake_llm_requests_total[5m])) > 0.05
        for: 10m
        labels: { severity: warning }
        annotations: { summary: "More than 5% of LLM calls fail" }
```

Also alert on the blackbox probe: `/ready` returning non-200 for 2 minutes. Pool and quarantine thresholds come from `EDGEQUAKE_DB_POOL_UTIL_WARN` (0.75), `EDGEQUAKE_DB_POOL_UTIL_CRITICAL` (0.90), `EDGEQUAKE_COMPENSATION_QUARANTINE_WARN` (1) and `_CRITICAL` (5).

## Logs

Logs go to stdout through the `tracing` crate. The default format is plain text. Set `EDGEQUAKE_LOG_FORMAT=json` for structured logs. The Helm chart sets it.

| `RUST_LOG` | Use |
|------------|-----|
| `edgequake=info,tower_http=info,sqlx=warn` | Production. |
| `edgequake=debug,tower_http=debug` | Development. |
| `edgequake_pipeline=debug` | Ingestion. |
| `edgequake_query=debug` | Query engine. |
| `sqlx=debug` | SQL statements. |

Ship stdout to Loki, ELK or your cloud logger with your normal agent (Promtail, Filebeat, Fluent Bit). No EdgeQuake-specific config is needed. Docker: `docker compose logs -f api`.

## Tracing

OpenTelemetry tracing comes from `edgequake-observability`.

| Capability | How |
|------------|-----|
| HTTP spans | `http_request` with `request_id` and `trace_id`. |
| Pipeline spans | `pipeline_chunk_extraction`, `sota_query_pipeline`. |
| OTLP export | Set `OTEL_EXPORTER_OTLP_ENDPOINT` or `EDGEQUAKE_OTEL_ENABLED=1` on an image built with OTEL (the Docker build arg `ENABLE_OTEL`, default true in the source compose). |
| Correlation | `X-Request-ID` and W3C `traceparent`. |
| Langfuse | Set `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY`. See [Langfuse 3.1.x](langfuse-3.1.md). |

Jaeger on the source Compose stack:

```bash
cd edgequake/docker
docker compose -f docker-compose.yml -f docker-compose.observability.yml --profile observability up --build
# Jaeger UI: http://localhost:16686
```

Production settings: `EDGEQUAKE_LOG_FORMAT=json`, `OTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4317`, `OTEL_SERVICE_NAME=edgequake-api`. Operator guide: [OBSERVABILITY.md](../OBSERVABILITY.md).

## Check configuration with doctor

`edgequake doctor` (and `--json`) checks `DATABASE_URL`, the secrets key, `JWT_SECRET` and the bind host without starting the server. `POST /api/v1/providers/test` tests a provider before you save it. `GET /health` and `GET /api/v1/models/health` probe the live provider.

## PostgreSQL checks

```sql
-- Connections by state
SELECT state, wait_event_type, count(*) FROM pg_stat_activity
WHERE datname = 'edgequake' GROUP BY 1, 2;

-- Connections per EdgeQuake pool role
SELECT application_name, count(*) FROM pg_stat_activity
WHERE application_name LIKE 'edgequake:%' GROUP BY 1;

-- Long-running queries
SELECT pid, now() - query_start AS duration, left(query, 120) AS query
FROM pg_stat_activity
WHERE state <> 'idle' AND now() - query_start > interval '5 minutes';

-- Largest tables
SELECT relname, pg_size_pretty(pg_total_relation_size(relid)) AS size
FROM pg_catalog.pg_statio_user_tables ORDER BY pg_total_relation_size(relid) DESC LIMIT 10;

-- Vector indexes
SELECT indexname, pg_size_pretty(pg_relation_size(indexname::regclass)) AS size
FROM pg_indexes WHERE indexdef LIKE '%hnsw%' OR indexdef LIKE '%ivfflat%';

-- Applied schema
SELECT max(version) FROM public._sqlx_migrations WHERE success;
```

Graph size: `SELECT * FROM ag_catalog.cypher('<graph>', $$ MATCH (n) RETURN count(n) $$) AS (count agtype);` Use the graph name from `ag_catalog.ag_graph`. Alert on PostgreSQL itself (connections above 80% of `max_connections`, cache hit ratio under 95%, replication lag) with your usual exporter.

## Troubleshooting

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["Alert fired"] --> B{"/ready returns 200?"}
  B -->|No| C["Read blockers in the JSON"]
  B -->|Yes| D{"Errors or slow?"}
  D -->|Slow| E["Check pool use and queue depth"]
  D -->|Errors| F["Check LLM failure rate and logs"]
  C --> G["Fix the named blocker"]
%% eq-classes
classDef eqBad fill:#FEE2E2,stroke:#EF4444,color:#7F1D1D
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class D,F eqBad
class E eqStore
```

How to read it: start with `/ready`. It names its own cause. If it is green, split the problem into slow versus failing.

| Symptom | Check | Fix |
|---------|-------|-----|
| High memory | `edgequake_task_queue_processing`, pool sizes, `shared_buffers`, `work_mem` | Lower `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` or workers. |
| Slow queries | `RUST_LOG=edgequake_query=debug,sqlx=debug`. PostgreSQL `log_min_duration_statement = 1000`. | See [Performance tuning](performance-tuning.md). |
| LLM errors | `edgequake_llm_requests_total{outcome="failure"}`, provider status page, `edgequake doctor`, `POST /api/v1/providers/test`. | Check keys, quotas and `OLLAMA_HOST`. |
| Documents stuck | `/api/v1/pipeline/queue-metrics`, logs. | See [Local extract reliability](local-extract-reliability.md). |

## Backups

EdgeQuake has no built-in backup job. Back up PostgreSQL with `pg_dump -Fc` or volume snapshots, and also keep `EDGEQUAKE_SECRETS_KEY`. Test restores. Alert from your backup tool (for example when the last success is older than 24 hours).

## See also

- [Deployment](deployment.md)
- [Configuration](configuration.md)
- [Troubleshooting guide](../troubleshooting/common-issues.md)
