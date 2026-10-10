---
title: 'Performance Tuning Guide'
description: "Tune EdgeQuake query latency, ingestion throughput and PostgreSQL. Covers the env knobs and their code defaults, and how to measure before you change them."
---

> **Product: v0.32.2** · Contract: OpenAPI · Related: [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md)

# Performance Tuning Guide

This guide is for operators who need faster queries or higher ingestion throughput. It shows where time goes, which settings matter, and the defaults the code uses. Measure first, then change one setting at a time.

Start with the sizing table in [Product limits](../product-limits.md). It sets RAM, `shared_buffers` and pool sizes for your workload. Claim ladders (`make ceiling-proof`) are honesty gates, not day-2 sizing.

## Where query time goes

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  A["Query request<br/>POST /api/v1/query"] --> B["Embed query<br/>(embedding cache)"]
  B --> C["Vector search<br/>(pgvector)"]
  C --> D["Graph traversal<br/>(Apache AGE)"]
  D --> E["Rerank<br/>(optional)"]
  E --> F["Build context"]
  F --> G["LLM generation"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class B,G eqLlm
class C,D eqStore
```

Each step is a tuning point. `naive` mode skips the graph step, and `bypass` skips retrieval. The caption above is the typical path for `hybrid` and `mix`.

Every query response includes a `stats` object. Use it to find the slow step:

```json
{
  "stats": {
    "embedding_time_ms": 45,
    "retrieval_time_ms": 123,
    "generation_time_ms": 2890,
    "total_time_ms": 3058
  }
}
```

`keyword_time_ms` appears when keyword extraction ran.

## Quick wins

### 1. Choose the LLM for the workload

Latency depends on the provider, hardware and context size. Measure with your own models instead of relying on published numbers.

| Workload | Starting point |
| -------- | -------------- |
| Cloud ingest and query | `gpt-5-nano` (the repo's recommended model for extraction) |
| Local dev (`make dev`, no API key) | `ollama` / `gemma4:latest` |
| Vision PDF conversion, env unset | `ollama` / `gemma4:latest`; for cloud, set `EDGEQUAKE_VISION_*` explicitly |

Pin the model with `EDGEQUAKE_DEFAULT_LLM_MODEL`. See [Configuration](configuration.md#provider-and-model-selection).

### 2. Send fewer chunks per query

Two per-request fields control how much context reaches the LLM:

| Field | Default | Lower it to |
| ----- | ------- | ----------- |
| `max_results` | server `max_chunks` (20) | 5 to 8 |
| `rerank_top_k` | 20 | 10 |

Server-side engine defaults (`QueryEngineConfig`, not per-request fields): `max_chunks` 20, `max_entities` 60, `max_relationships` 60, `max_context_tokens` 30000, `graph_depth` 2.

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "...", "max_results": 5}'
```

### 3. Pick the query mode

| Mode | What it retrieves | Use it for |
| ---- | ----------------- | ---------- |
| `naive` | Vector search over chunks only | Simple factual questions (cheapest retrieval) |
| `local` | Entity and its neighbourhood | Questions about one entity |
| `global` | Relationship vectors | Themes across the corpus |
| `hybrid` | Local and global together | General questions |
| `mix` (server default) | Vector and graph results, fused with RRF | Production default |
| `bypass` | No retrieval; the LLM answers directly | Questions that need no corpus |

`global` and `mix` do the most work per query. Compare them with `stats` on your data.

```bash
# Fast mode for simple factual questions
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is X?", "mode": "naive"}'
```

## Document processing

### Worker threads and fairness

- `WORKER_THREADS` defaults to 4 times the CPU count (at least 4). Local providers (Ollama, LM Studio) cap it at 4.
- `MAX_TASKS_PER_TENANT` defaults to about three quarters of the worker count. `0` removes the cap.
- Local providers run one ingest task per tenant unless `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1`.
- If the LLM is cloud but extraction runs locally, set `EDGEQUAKE_EXTRACT_PROVIDER=ollama` so the local cap applies.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
  A["Worker claims ingest task"] --> B{"Local LLM provider?"}
  B -->|Yes| C{"EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1?"}
  C -->|No| D["Cap at 1 task per tenant"]
  C -->|Yes| E["Use MAX_TASKS_PER_TENANT"]
  B -->|No| E
  D --> F{"Tenant under cap?"}
  E --> F
  F -->|Yes| G["Run task"]
  F -->|No| H["Wait for a free slot"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class B eqLlm
```

Local providers are clamped to one ingest task per tenant by default, so a single GPU is not flooded.

```bash
# 8 workers on a 4-core machine (LLM calls are I/O-bound)
export WORKER_THREADS=8
# Per-tenant ingest cap (default is about 3/4 of WORKER_THREADS; 0 disables it)
export MAX_TASKS_PER_TENANT=6
```

### Task lease and multi-replica

| Variable | Default | Tuning note |
| -------- | ------- | ----------- |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | `120` (minimum 30) | Workers renew the lease every TTL/3, so 40 s by default (at least 5 s). A longer TTL gives slow workers more slack before another worker reclaims the task. |
| `EDGEQUAKE_STARTUP_AUTO_RESUME` | on | Set `0` to mark interrupted tasks as Failed at boot, then reprocess them by hand. |
| `EDGEQUAKE_REPLICAS` and `EDGEQUAKE_TASK_DELIVERY` | `1` and `local` | More than one replica needs `bridged` or `notify_only`. Boot fails otherwise. |

### Timeouts for large documents

Conversion and ingestion run as separate tasks. Each gets a timeout from `LargeDocumentProfile` (`edgequake-api/src/services/large_document_profile.rs`), which depends on page count:

| Phase | Task type | Timeout source |
| ----- | --------- | -------------- |
| Convert | `pdf_processing` | `convert_timeout_secs` (plus the Pass B budget) |
| Ingest | `insert` | `ingest_timeout_secs` |

To override both phases, set `TASK_PROCESSING_TIMEOUT_SECS` (the legacy single knob). The floor is 7200 s.

### Chunk size

The server default chunk size is 1200 (`chunk_size` in the pipeline config). Trade-offs:

- **Smaller chunks**: more precise retrieval and lower token cost per extraction call, but more LLM calls and less context per chunk.
- **Larger chunks**: fewer LLM calls and more context per chunk, but less precise retrieval and higher token cost per call.

Keep the default unless measurements show a clear gain.

### Batch uploads

- Text and image batches use `POST /api/v1/documents/upload/batch`. A request takes up to 20 files by default (`EDGEQUAKE_MAX_BATCH_UPLOAD_FILES`, range 1 to 500).
- PDFs use `POST /api/v1/documents/pdf` or `POST /api/v1/documents/pdf/batch`. `/documents/upload/batch` rejects PDFs.
- The Web UI uploads up to 3 files at a time (`MAX_CONCURRENT_FILE_UPLOADS`).

```bash
# Upload several PDFs in one request. Processing stays capacity-governed.
curl -X POST http://localhost:8080/api/v1/documents/pdf/batch \
  -F "files=@doc1.pdf" \
  -F "files=@doc2.pdf" \
  -F "files=@doc3.pdf"
```

Before you raise concurrency, measure with `GET /api/v1/pipeline/queue-metrics` and `make measure-bulk-ingest ARM=D N=5`. Local Ollama stays close to serial unless you also raise Ollama's `OLLAMA_NUM_PARALLEL`, add VRAM headroom, and set `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1`.

The recorded regression floor is at least 4.0 docs/min (HEAD 0.26.3, tenant 6, Mistral, N=5 small text). It detects regressions and is not a service-level target. See [the measurement summary](../../specs/122-implementation/measurements/20260830-summary.json).

## Graph UI optimization

The Web UI graph viewer uses Sigma.js and Graphology. Browser-side lifecycle mistakes often cost more than backend latency.

- Prefer `force` for general exploration. Use `circular` or `hierarchical` when you need a fast, stable layout.
- Keep layout logic in `edgequake_webui/src/lib/graph/layouts.ts` and edge-identity rules in `edgequake_webui/src/lib/graph/ids.ts`.
- Do not recreate the Sigma instance for a plain layout switch, and do not mutate every node on hover. Sigma reducers can express the same visual state.

## Database optimization

### PostgreSQL settings

These are starting points for a dedicated database host. Adjust them to your RAM and workload.

```ini
# Memory (adjust for your RAM)
shared_buffers = 4GB                  # about 25% of RAM
effective_cache_size = 12GB           # about 75% of RAM
work_mem = 64MB                       # per sort or hash, per connection: keep it modest
maintenance_work_mem = 1GB            # index builds

# Connections: must cover the pool budget (see configuration.md#database)
max_connections = 200

# Write-ahead log
wal_buffers = 64MB
checkpoint_completion_target = 0.9

# Query planning (SSD)
random_page_cost = 1.1
effective_io_concurrency = 200

# Parallel query
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
```

`work_mem` applies to every sort or hash in every connection. A high value multiplied by many connections can exhaust RAM, so raise it only after you measure.

### Connection pooling

Each EdgeQuake process opens four role-based pools (query 16, ingest 12, queue 4, admin 2 by default). The boot-time budget check counts them against `max_connections`. See [Configuration: Database](configuration.md#database).

Add PgBouncer only when the total across replicas exceeds what PostgreSQL should accept. Session pooling is the safe default. Test transaction pooling with your driver before you use it.

```ini
# pgbouncer.ini
[databases]
edgequake = host=localhost port=5432 dbname=edgequake

[pgbouncer]
pool_mode = session
max_client_conn = 1000
default_pool_size = 50
reserve_pool_size = 10
```

```bash
# Connect through PgBouncer (port 6432)
DATABASE_URL="postgresql://user:pass@localhost:6432/edgequake"
```

### pgvector indexes

EdgeQuake owns the vector schema. Migrations create the vector tables (for example `chunk_embeddings` and `entity_embeddings`) and their HNSW indexes. Do not create ad-hoc indexes on those tables.

- **Build time**: `m = 16` and `ef_construction = 128` by default. `EDGEQUAKE_HNSW_EF_CONSTRUCTION` changes the value for new indexes only.
- **Query time**: `EDGEQUAKE_HNSW_EF_SEARCH` (1 to 1000) sets `hnsw.ef_search`. Higher values improve recall and cost latency. Measure on your data.
- **Iterative scan**: `EDGEQUAKE_HNSW_ITERATIVE_SCAN` is `relaxed_order` by default. The other values are `strict_order` and `off`.
- **Per-workspace partial indexes**: `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE` is on by default. When a workspace partial index is ready and filters use only columns, EdgeQuake sets `enable_seqscan = off` and `random_page_cost = 1.1` for that query (SPEC-067).

To inspect the indexes, run:

```sql
SELECT indexname, indexdef
FROM pg_indexes
WHERE indexdef ILIKE '%hnsw%';
```

### Apache AGE

EdgeQuake creates the graph per namespace. Graph names follow `eq_<namespace>_graph` (for example `eq_eq_default_graph`). Do not create labels by hand.

- `EDGEQUAKE_NATIVE_GRAPH_WRITES` is `1` by default (native upserts). Set `0` to fall back to Cypher `MERGE`.
- For ad-hoc inspection in `psql`, load AGE first:

```sql
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
```

## Query optimization

### Caches

- **Query embeddings**: an in-process LRU cache of 10,000 entries with a 1-hour TTL. It is cleared on restart.
- **Keyword and answer caches**: `EDGEQUAKE_LLM_CACHE` is the master switch (on by default). `EDGEQUAKE_KEYWORD_CACHE` and `EDGEQUAKE_QUERY_ANSWER_CACHE` override each cache.
- **Provider prompt cache**: `EDGEQUAKE_PROMPT_CACHE` (on by default) sends cache hints to the provider. It does not skip generation.

Turn the LLM caches off for cold benchmark runs.

### Reranking

Reranking improves precision and adds a model call. It is on by default (`enable_rerank` is `true`, `rerank_top_k` is 20).

```bash
# Skip reranking for the lowest latency
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "...", "enable_rerank": false}'

# Rerank fewer candidates
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "...", "rerank_top_k": 5}'
```

### Streaming

For chat interfaces, stream the answer with `POST /api/v1/query/stream` to start showing text sooner:

```bash
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Accept: text/event-stream" \
  -H "Content-Type: application/json" \
  -d '{"query": "..."}'
```

## LLM provider optimization

### Cloud providers

- Keep `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` below your provider's requests-per-minute limit. The cloud default is 16.
- Set `EDGEQUAKE_LLM_TIMEOUT_SECS` for the HTTP call. Keep it at or above the chunk timeout.

### Ollama

- Check that the model runs on the GPU with `ollama ps`. The processor column shows CPU or GPU.
- `OLLAMA_NUM_PARALLEL` is Ollama's own setting for parallel requests. EdgeQuake assumes about one for local providers unless you lift the cap.
- `make dev` sets `OLLAMA_CONTEXT_LENGTH` to 8192 unless you override it.
- Pick a quantized model build that fits your VRAM with headroom. Smaller quantizations are faster but usually less accurate.

### Local versus cloud latency

Measure in your environment. Local GPUs avoid network round trips, but cloud models usually give better extraction quality and throughput at scale. Use `pressure` and `tenant_park_waiters` from the queue metrics to tell fairness waits apart from slow LLM calls.

## Scaling strategies

### Horizontal scaling

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  U["Clients"] --> L["Load balancer"]
  L --> A["EdgeQuake replica 1"]
  L --> B["EdgeQuake replica 2"]
  L --> C["EdgeQuake replica N"]
  A --> P["PostgreSQL<br/>(pgvector + AGE)"]
  B --> P
  C --> P
  P -.-> R["Read replica<br/>(DATABASE_READ_URL)"]
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class U eqActor
class P,R eqStore
```

- Run several API replicas behind a load balancer. Set `EDGEQUAKE_REPLICAS` to the highest replica count you expect.
- More than one replica needs `EDGEQUAKE_TASK_DELIVERY=bridged` or `notify_only`.
- All replicas share one PostgreSQL. Each process has its own pool budget, so count them together.

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: edgequake-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: edgequake
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Read replicas

Set `DATABASE_READ_URL` to send query-pool traffic to a replica. Writes still go to `DATABASE_URL`.

```bash
DATABASE_URL="postgresql://user:pass@primary:5432/edgequake"
DATABASE_READ_URL="postgresql://user:pass@replica:5432/edgequake"
```

## Monitoring performance

### Key metrics

These are example alert levels. Set your own after you record a baseline.

| Signal | Metric or endpoint | Example alert |
| ------ | ------------------ | ------------- |
| Query latency | `edgequake_query_duration_seconds` (histogram) | p99 above 30 s |
| Ingestion throughput | `edgequake_document_processing_total` | below 0.5 docs/min |
| HTTP errors | `edgequake_http_requests_total` with `status` 5xx | above 5% of requests |
| DB pool pressure | `EDGEQUAKE_DB_POOL_UTIL_WARN` (0.75) and `_CRITICAL` (0.90) | critical threshold |
| Queue backlog | `GET /api/v1/pipeline/queue-metrics` (`pending_count`, `pressure`, `tenant_park_waiters`) | sustained growth |

### Prometheus queries

```promql
# Query latency p99
histogram_quantile(0.99,
  rate(edgequake_query_duration_seconds_bucket[5m])
)

# Document processing rate
rate(edgequake_document_processing_total[5m])

# Share of 5xx responses
sum(rate(edgequake_http_requests_total{status=~"5.."}[5m]))
  / sum(rate(edgequake_http_requests_total[5m]))
```

### Benchmarking

`cargo bench` runs the benchmarks in `edgequake/benches/`. Run it from `edgequake/`. Record your own baselines. [`edgequake/benches/BASELINES.md`](../../edgequake/benches/BASELINES.md) lists the recorded ones.

## Performance checklist

### Before you tune

- [ ] Baseline metrics recorded
- [ ] Bottleneck identified from `stats` or queue metrics (usually the LLM)
- [ ] Resource monitoring in place

### Quick wins

- [ ] Model and provider chosen for the workload, after measuring
- [ ] `max_results` and `rerank_top_k` lowered for latency
- [ ] Query mode chosen (`naive` for simple questions)
- [ ] Streaming enabled for chat
- [ ] `tenant_park_waiters` understood under the local LLM cap

### Database

- [ ] PostgreSQL tuned for RAM, with `work_mem` checked against connection count
- [ ] Vector indexes created by migrations (`pg_indexes` shows HNSW)
- [ ] Pool budget checked against `max_connections`
- [ ] Read replica set with `DATABASE_READ_URL`, if needed

### Scaling

- [ ] `EDGEQUAKE_REPLICAS` and `EDGEQUAKE_TASK_DELIVERY` set for the replica count
- [ ] Autoscaling rules defined
- [ ] Load test completed

## Troubleshooting slow queries

Compare the `stats` fields from the response with the table:

| Symptom | Likely cause | Fix |
| ------- | ------------ | --- |
| Slow `embedding_time_ms` | Cold start or provider latency | Warm up with a test query; check the provider |
| Slow `retrieval_time_ms` | Missing or unready vector index | Check `pg_indexes` for HNSW; check migrations |
| Slow `generation_time_ms` | Large context | Lower `max_results` or `rerank_top_k` |
| Slow `generation_time_ms` | Slow model | Switch to a faster model |
| High latency variance | Connection pool or read path busy | Check pool utilization and `read_path_busy` (HTTP 503) |

## Ingestion pipeline tuning

On large documents or a slow local LLM (Ollama on one GPU, LM Studio on CPU), the default limits can cause `Timeout after 180s` failures (see [issue #194](https://github.com/raphaelmansuy/edgequake/issues/194)). These variables tune the pipeline:

| Variable | Default | Guidance |
| -------- | ------- | -------- |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS` | `180` cloud, `600` local | Set it to the time one LLM call takes on your biggest chunk, times 1.5. |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | `16` cloud, `1` local | Local providers stay at 1 unless `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1`. Cloud: stay under your rate limit. |
| `EDGEQUAKE_CHUNK_MAX_RETRIES` | `3` | Set `1` for fast failure while you debug. |
| `EDGEQUAKE_CHUNK_RETRY_DELAY_MS` | `1000` | Raise to `5000` if the LLM needs time to warm up. |
| `EDGEQUAKE_LLM_TIMEOUT_SECS` | `600` cloud, `900` local | Must be at least `EDGEQUAKE_CHUNK_TIMEOUT_SECS`. |

Local providers are clamped to one concurrent extraction unless you set `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1`. Set that flag in every local profile that needs more than one.

### Profiles

**GPU server (high throughput):**

```bash
export EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=120
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=32
export EDGEQUAKE_LLM_TIMEOUT_SECS=600
```

**Single-GPU workstation (balanced):**

```bash
export EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=300
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4
export EDGEQUAKE_LLM_TIMEOUT_SECS=1800
```

**CPU-only Ollama (conservative):** keep the local default of one extraction at a time.

```bash
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=600
export EDGEQUAKE_CHUNK_RETRY_DELAY_MS=5000
export EDGEQUAKE_LLM_TIMEOUT_SECS=3600
```

**Cloud LLM (OpenAI, Anthropic, Mistral; fast, rate-limited):**

```bash
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=60
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=8   # stay under your RPM limit
export EDGEQUAKE_LLM_TIMEOUT_SECS=120
```

> **Rule of thumb:** set `EDGEQUAKE_CHUNK_TIMEOUT_SECS` to the time one LLM call takes on your biggest chunk, times 1.5. Then set `EDGEQUAKE_LLM_TIMEOUT_SECS` to at least that value.

## See also

- [Configuration reference](configuration.md): all settings and defaults
- [Environment variable reference](env-reference.md): one-page lookup
- [Deployment guide](deployment.md): production setup
- [Monitoring guide](monitoring.md): observability
