---
title: "Configuration reference"
description: "Environment variables and models.toml for EdgeQuake: server, database pools, providers, ingestion limits, caches, security, logging, with verified defaults."
---

# Configuration reference

This page is for operators who tune an EdgeQuake deployment. EdgeQuake reads environment variables, an optional `models.toml` catalog, and per-workspace settings. Defaults here come from the code at v0.32.2 and HEAD (v0.33.0). Provider setup has its own section of the docs: start at [Providers](../providers/index.md). A generated list of the SPEC-163 onboarding variables is in the [env reference](env-reference.md).

## How settings combine

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["API request"] --> B{"Request names a provider?"}
  B -->|Yes| R["Use the request"]
  B -->|No| C{"Workspace has its own?"}
  C -->|Yes| W["Use the workspace setting"]
  C -->|No| D{"Env var set?"}
  D -->|Yes| E["Use the env var"]
  D -->|No| F["Use models.toml defaults"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class B,F eqLlm
```

How to read it: the most specific setting wins. A request beats a workspace, a workspace beats the server environment, and the environment beats the catalog defaults. Saved Connections (see [Providers](../providers/index.md)) change where the credentials come from, not this order.

`models.toml` is searched in this order: `EDGEQUAKE_MODELS_CONFIG`, `./models.toml`, `~/.edgequake/models.toml`, then the copy built into the binary.

## Server

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | API bind address. |
| `PORT` | `8080` | API port. |
| `RUST_LOG` | `edgequake=info,edgequake_api=info,edgequake_query=info,edgequake_pipeline=info,edgequake_storage=warn,tower_http=warn,sqlx=warn` | Log filter. |
| `EDGEQUAKE_LOG_FORMAT` | `plain` | Set `json` for structured logs. |
| `EDGEQUAKE_LOG_SPAN_EVENTS` | unset | Log span open and close events. |
| `WORKER_THREADS` | 4 times CPU count, at least 4 | Background task workers. |
| `EDGEQUAKE_PORT`, `EDGEQUAKE_HOST` | n/a | Compose and Helm set these, and `edgequake doctor` reads `EDGEQUAKE_HOST`. The server bind uses `HOST` and `PORT`. In Compose `EDGEQUAKE_PORT` also sets the host-side published port. |

Log levels, from quiet to loud: `error`, `warn`, `info`, `debug`, `trace`. Example: `RUST_LOG="edgequake=info,tower_http=info"`. Observability and metrics are in [Monitoring](monitoring.md).

## Database

`DATABASE_URL` is required. There is no in-memory mode.

```bash
DATABASE_URL="postgresql://edgequake:pass@db.example.com:5432/edgequake?sslmode=require"
```

The server uses four connection pools so one workload cannot starve another. Idle connections still count against PostgreSQL `max_connections`, so size them for every process that shares the database. Full runbook: [`specs/112-connection-pool/07-ops-runbook.md`](../../specs/112-connection-pool/07-ops-runbook.md).

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_DB_POOL_SIZE_QUERY` | `16` | Query pool maximum (1 to 128). |
| `EDGEQUAKE_DB_POOL_SIZE_INGEST` | `12` | Ingest pool maximum. |
| `EDGEQUAKE_DB_POOL_SIZE_QUEUE` | `4` | Task queue pool. At boot it is raised to at least `WORKER_THREADS`. |
| `EDGEQUAKE_DB_POOL_SIZE_ADMIN` | `2` | Admin and migrate pool. |
| `EDGEQUAKE_DB_POOL_INSTANCE_COUNT` | `1` | Replica count for the startup budget check. Use the peak during rollouts. |
| `EDGEQUAKE_DB_POOL_BUDGET_MODE` | `warn` | `warn` or `fail` when `instances x pool sum` exceeds `max_connections - reserve - 10`. |
| `EDGEQUAKE_DB_POOL_IDLE_TIMEOUT_SECS` | `600` | Idle connection reap. |
| `EDGEQUAKE_DB_POOL_MAX_LIFETIME_SECS` | `1800` | Maximum connection age. |
| `EDGEQUAKE_DB_IDLE_IN_XACT_TIMEOUT_SECS` | `60` | `idle_in_transaction_session_timeout`. |
| `DATABASE_READ_URL` | unset | Optional read replica for the query pool. |
| `DATABASE_POOL_SIZE` | `32` | Sizes only the interactive read-path limiter: permits are `max(2, DATABASE_POOL_SIZE / 8)`. |

Backends set `application_name=edgequake:<role>`, so you can tell the pools apart in `pg_stat_activity`. A shared database might use `QUERY=8 INGEST=6 QUEUE=2 ADMIN=1` (17 connections per process).

If logs show `pool timed out` on `claim_next` followed by `SSLRequest: 0x00`, PostgreSQL probably restarted. Wait until it is healthy, then restart the API so the pools re-form.

### Interactive read limits

Catalog reads (document list, detail, search, tenants, workspaces) share one deadline. When it expires they return HTTP 503 `read_path_busy`, and PostgreSQL cancels the statement 250 ms earlier. See [common issues](../troubleshooting/common-issues.md).

| Variable | Default | Range | Description |
|----------|---------|-------|-------------|
| `EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS` | `2500` | 500 to 30000 | Wall clock for these reads, including the wait for a permit. |
| `EDGEQUAKE_COMMUNITY_STATEMENT_TIMEOUT_MS` | `30000` | 1000 to 300000 | Per-page budget for community scans. The applied value is 250 ms lower. |
| `EDGEQUAKE_COMMUNITY_BACKFILL_MAX_NODES` | `50000` | none | Skip automatic community refresh above this node count. |
| `EDGEQUAKE_COMMUNITY_MAX_NODES` | `50000` | 100 to 5000000 | Nodes loaded for detection. Larger graphs are sampled. |

## LLM providers and models

Provider guides, the role matrix and saved Connections are in [Providers](../providers/index.md). This table lists only what the server reads from the environment.

| Provider | Variables |
|----------|-----------|
| OpenAI | `OPENAI_API_KEY`, `OPENAI_BASE_URL` (default `https://api.openai.com/v1`) |
| OpenAI-compatible | `OPENAI_COMPATIBLE_BASE_URL`, `OPENAI_COMPATIBLE_API_KEY` |
| Ollama | `OLLAMA_HOST` (default `http://localhost:11434`), `OLLAMA_MODEL`, `OLLAMA_EMBEDDING_MODEL`, `OLLAMA_EMBEDDING_HOST` (default: `OLLAMA_HOST`) |
| LM Studio | `LMSTUDIO_HOST` (default `http://localhost:1234`) |
| oMLX | `OMLX_HOST`, `OMLX_BASE_URL` |
| Anthropic | `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL` (default `https://api.anthropic.com`) |
| Gemini (API key) | `GEMINI_API_KEY` or `GOOGLE_API_KEY` |
| Vertex AI | `GOOGLE_CLOUD_PROJECT` (required), `GOOGLE_CLOUD_REGION` (default `us-central1`), `GOOGLE_APPLICATION_CREDENTIALS`, `GOOGLE_ACCESS_TOKEN` |
| Mistral | `MISTRAL_API_KEY` |
| xAI | `XAI_API_KEY`, `XAI_BASE_URL` (default `https://api.x.ai/v1`) |
| OpenRouter | `OPENROUTER_API_KEY`, `OPENROUTER_BASE_URL` |
| MiniMax | `MINIMAX_API_KEY` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_VERSION` |

Inside a container, `localhost` is the container. Use `host.docker.internal` to reach Ollama or LM Studio on the host.

### Google Vertex AI (Enterprise)

`vertexai` authenticates with short-lived OAuth2 tokens from GCP identity, not a static key. Credentials are tried in this order: `GOOGLE_ACCESS_TOKEN`, the GCE, GKE or Cloud Run metadata server, the ADC file (`~/.config/gcloud/application_default_credentials.json`), the service account in `GOOGLE_APPLICATION_CREDENTIALS`, then `gcloud auth application-default print-access-token`.

```bash
gcloud auth application-default login      # not: gcloud auth login application-default
export GOOGLE_CLOUD_PROJECT=your-gcp-project
export GOOGLE_CLOUD_REGION=europe-west1     # optional
# If ~/.edgequake/models.toml lacks vertexai, use the bundled catalog:
export EDGEQUAKE_MODELS_CONFIG=/path/to/edgequake/edgequake/models.toml
```

In production prefer an attached workload service account with `roles/aiplatform.user`. An expired ADC file can look satisfied while health stays offline: log in again.

### Provider and model selection

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_DEFAULT_LLM_PROVIDER` | none | Wins over `EDGEQUAKE_LLM_PROVIDER` when both are set. `make dev` sets it. |
| `EDGEQUAKE_DEFAULT_LLM_MODEL` | none | Wins over `EDGEQUAKE_LLM_MODEL`. |
| `EDGEQUAKE_LLM_PROVIDER` | `openai` | LLM provider. |
| `EDGEQUAKE_LLM_MODEL` | `gpt-4.1-mini` (Ollama: `gemma4:latest`) | LLM model. |
| `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER` / `EDGEQUAKE_EMBEDDING_PROVIDER` | `openai` | Embedding provider. `DEFAULT` wins. |
| `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL` / `EDGEQUAKE_EMBEDDING_MODEL` | `text-embedding-3-small` (Ollama: `embeddinggemma:latest`) | Embedding model. |
| `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION` / `EDGEQUAKE_EMBEDDING_DIMENSION` | `1536` (detected from the model name) | Vector size. It must match the stored vectors. |
| `EDGEQUAKE_MODELS_CONFIG` | none | Path to a `models.toml`. |

LightRAG-style aliases are accepted: `MODEL_PROVIDER` or `CHAT_PROVIDER`, `CHAT_MODEL` or `LLM_MODEL`, `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, `EMBEDDING_DIMENSION`. Canonical `EDGEQUAKE_*` names win.

The same default can differ by how you start the server:

| Start method | LLM | Embedding |
|--------------|-----|-----------|
| Bundled catalog, `cargo run`, no env | `openai` / `gpt-4.1-mini` | `text-embedding-3-small` / 1536 |
| `.env.example` production pins | `openai` / `gpt-5-mini` | `text-embedding-3-small` / 1536 |
| `make dev`, no `OPENAI_API_KEY` | `ollama` / `gemma4:latest` | `embeddinggemma:latest` / 768 |
| `make dev`, with `OPENAI_API_KEY` | `openai` / `gpt-5-nano` | `text-embedding-3-small` / 1536 |

### Vision (PDF to Markdown)

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_VISION_PROVIDER` | first set of `EDGEQUAKE_VISION_LLM_PROVIDER`, `EDGEQUAKE_DEFAULT_LLM_PROVIDER`, `EDGEQUAKE_LLM_PROVIDER`, else `ollama` | Vision provider. |
| `EDGEQUAKE_VISION_MODEL` | first set of `EDGEQUAKE_VISION_LLM_MODEL`, `EDGEQUAKE_DEFAULT_LLM_MODEL`, `EDGEQUAKE_LLM_MODEL`, else the provider default (`gemma4:latest`, `gpt-4.1-nano`, `mistral-small-latest`) | Vision model. |
| `EDGEQUAKE_VISION_TIMEOUT_SECS` | `600` in the Compose files | Per-call timeout. |

With no vision or LLM variables set, the server uses Ollama with `gemma4:latest`, not the OpenAI pins in `models.toml`.

### Ollama thinking mode (SPEC-113)

EdgeQuake asks Ollama which models support `think` instead of guessing from names.

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_OLLAMA_THINK_CAPABILITY` | `auto` | `auto`, `force_off`, `force_on` (debug) or `legacy_name`. |
| `EDGEQUAKE_OLLAMA_CAPABILITY_TTL_SECS` | `300` | Cache lifetime of the capability answer. |
| `EDGEQUAKE_OLLAMA_CAPABILITY_TIMEOUT_MS` | `2000` | Probe timeout. On failure `think` is omitted. |

### Hybrid mode: separate embedding provider

Use one provider for the LLM and another, or another Ollama host, for embeddings.

```bash
export EDGEQUAKE_LLM_PROVIDER=openai OPENAI_API_KEY=sk-...
export EDGEQUAKE_EMBEDDING_PROVIDER=ollama
export OLLAMA_EMBEDDING_HOST=http://gpu-box:11434
export OLLAMA_EMBEDDING_MODEL=nomic-embed-text
```

### Application attribution (SPEC-043)

Identifies EdgeQuake to upstream providers (OpenRouter referer and title, Anthropic application ID, and so on).

| Variable | Description |
|----------|-------------|
| `EDGEQUAKE_APP_ID`, `EDGEQUAKE_APP_NAME`, `EDGEQUAKE_APP_URL` | Identity sent upstream. |
| `EDGEQUAKE_TENANT_ID` | Optional tenant identifier. |

Headers `x-edgequake-app-id`, `-app-name`, `-app-url`, `-tenant-id` and `-request-id` override these per request. Admins can change them live with `PATCH /api/v1/settings/app-attribution`. `GET /api/v1/settings/attribution` shows the effective values.

## Ingestion: timeouts and concurrency

These are the knobs for large documents and slow local LLMs. Out-of-range values are clamped. Non-numeric values are ignored.

| Variable | Default | Min | Max | Description |
|----------|---------|-----|-----|-------------|
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS` | `180` cloud, `600` local | `10` | none | Per-chunk LLM timeout (pipeline layer). |
| `EDGEQUAKE_CHUNK_MAX_RETRIES` | `3` | `1` | `20` | Retries per chunk. |
| `EDGEQUAKE_CHUNK_RETRY_DELAY_MS` | `1000` | `0` | `60000` | First backoff delay. |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | `16` cloud, `1` local | `1` | `32` | Parallel extraction calls per document. |
| `EDGEQUAKE_LLM_TIMEOUT_SECS` | `600` | `10` | `3600` | HTTP safety timeout. Keep it at or above the chunk timeout. |
| `EDGEQUAKE_LLM_MAX_TOKENS` | `16384` | `1` | `65536` | Maximum response tokens. |
| `EDGEQUAKE_MAX_EXTRACTION_ENTITIES`, `EDGEQUAKE_MAX_EXTRACTION_RECORDS` | `40`, `100` | | | Per-response caps (SPEC-117). |

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  A["Chunk sent to LLM"] --> B{"Answer within chunk timeout?"}
  B -->|Yes| C["Parse entities"]
  B -->|No| D{"Retries left?"}
  D -->|Yes| E["Wait, then retry"]
  D -->|No| F["Chunk fails"]
  E --> A
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqBad fill:#FEE2E2,stroke:#EF4444,color:#7F1D1D
class A eqLlm
class B,F eqBad
```

How to read it: the chunk timeout fires first. The HTTP timeout is only a safety net that must not be shorter. "Local" means Ollama or LM Studio. For stalls see [Local extract reliability](local-extract-reliability.md).

```bash
# Large document on a single-GPU Ollama
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=600
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=1
export EDGEQUAKE_LLM_TIMEOUT_SECS=3600
```

## Workers, leases and fairness (SPEC-057)

PostgreSQL task rows are the source of truth. The in-memory channel only wakes workers. See [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md).

| Variable | Default | Description |
|----------|---------|-------------|
| `WORKER_THREADS` | 4 x CPU, at least 4 | Worker count. Local providers cap it at 4. |
| `MAX_TASKS_PER_TENANT` | about three quarters of workers | Ingest tasks per tenant. `0` disables the cap. Local providers cap it at 1. |
| `MAX_LIFECYCLE_TASKS_PER_TENANT` | same as ingest cap (local: 4) | Lifecycle lane cap (delete, reprocess). |
| `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY` | `0` | Set `1` to lift the local caps. |
| `EDGEQUAKE_EXTRACT_PROVIDER`, `EDGEQUAKE_DEFAULT_EXTRACT_PROVIDER` | unset | Provider used to decide if the local caps apply. |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | `120` (min `30`) | Claim lease. The worker renews it every third of the TTL, at least 5 s. |
| `EDGEQUAKE_REPLICAS` | `1` | Intended API process count. |
| `EDGEQUAKE_TASK_DELIVERY` | `local` | `local`, `bridged` or `notify_only`. Boot fails if replicas is above 1 and this is `local`. |
| `EDGEQUAKE_STARTUP_AUTO_RESUME` | on | Reclaim stale Processing tasks at boot. `0`, `false` or `off` marks them Failed instead. |
| `EDGEQUAKE_STARTUP_RECONCILE_MAX` | `32` | Maximum orphan rows reconciled at boot. |
| `EDGEQUAKE_DB_POOL_UTIL_WARN`, `EDGEQUAKE_DB_POOL_UTIL_CRITICAL` | `0.75`, `0.90` | Store contention thresholds. Critical turns `/ready` into 503. |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_WARN`, `_CRITICAL` | `1`, `5` | Compensation dead-letter thresholds. Critical turns `/ready` into 503. |
| `EDGEQUAKE_NATIVE_GRAPH_WRITES` | `1` | Native AGE upserts. `0` falls back to Cypher MERGE. |
| `EDGEQUAKE_HNSW_ITERATIVE_SCAN` | `relaxed_order` | pgvector 0.8 iterative scan mode. |
| `EDGEQUAKE_HNSW_EF_CONSTRUCTION` | `128` | HNSW build parameter. Only affects new indexes. |

PDF admission queues a convert-only task first. After the Markdown is stored, a separate insert task runs with its own lease and timeout.

## Data layer and caches

Most defaults assume a database after all migrations and drops. Fleets in the middle of an upgrade should follow [Upgrading](upgrading.md) and the [SPEC-091 runbook](spec091-upgrade-from-v0.22.0.md).

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_MIGRATION_MODE` | `verify` | `off`, `verify` or `automatic`. The `edgequake migrate` console is the supported path. |
| `EDGEQUAKE_MIGRATION_CONFIRM_DROP` | `0` | Same as `edgequake migrate --confirm-drop`. Irreversible. Never set it in shared env files. |
| `EDGEQUAKE_VECTOR_BACKEND` | `typed_embeddings` | Vector storage. `legacy_tables` only before the drops. |
| `EDGEQUAKE_CHUNK_TEXT_AUTHORITY` | `relational` | Chunk text source. `kv` only before the drops. |
| `EDGEQUAKE_KV_FAMILY_*` | `relational` | Per-family KV routing. |
| `EDGEQUAKE_SERVING_FENCE` | `1` | Refuse to serve when typed tables are missing. |
| `EDGEQUAKE_OUTBOX_DRAIN` | `on` | `off`, `dry-run` or `on`. |
| `EDGEQUAKE_CITATION_REQUIRE` | `1` | Merges must carry `source_chunk_ids`. |
| `EDGEQUAKE_CONTEXTUAL_CHUNK` | `0` | Add a context preamble to chunks. |
| `EDGEQUAKE_EXTRACTION_LANGUAGE` | `English` | Default language for extraction. A workspace can override it. |
| `EDGEQUAKE_LLM_CACHE` | `1` | Master switch for the keyword and answer caches. |
| `EDGEQUAKE_KEYWORD_CACHE`, `EDGEQUAKE_QUERY_ANSWER_CACHE` | follow the master | Per-cache override. |
| `EDGEQUAKE_PROMPT_CACHE` | `1` | Provider-side prompt cache hints. Does not skip generation. |
| `EDGEQUAKE_PROMPT_CACHE_TTL` | `5m` | Anthropic and Bedrock cache TTL: `5m` or `1h`. |
| `EDGEQUAKE_LLM_OMIT_TEMPERATURE`, `EDGEQUAKE_LLM_OMIT_REASONING_EFFORT` | `0` | Never send those fields upstream (some Mantle models reject them). |
| `EDGEQUAKE_LLM_API_FORMAT` | `chat_completions` | Or `responses`. |

The release benchmark pins `EDGEQUAKE_LLM_CACHE=0` for cold runs.

### Langfuse (SPEC-124)

| Variable | Default | Description |
|----------|---------|-------------|
| `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY` | unset | With both set, trace export turns on. The secret is never logged. |
| `LANGFUSE_BASE_URL` (alias `LANGFUSE_HOST`) | `https://cloud.langfuse.com` | UI and OTLP base. |
| `LANGFUSE_PROJECT_ID` | auto | Project for UI deep links. |
| `EDGEQUAKE_LANGFUSE_ENABLED` | follows keys | Force on or off. |
| `EDGEQUAKE_LANGFUSE_API` | `auto` | `auto` tries OTLP and falls back to ingestion on HTTP 404. Or force `otlp` or `ingestion`. |

See [Langfuse 3.1.x](langfuse-3.1.md) and the [observability guide](../OBSERVABILITY.md).

### Decision extraction (SPEC-160, preview)

An unset `EDGEQUAKE_EXTRACTION_MODE` still means the chat-LLM extractor. Guide: [Decision extraction](../concepts/decision-extraction.md). A bad value fails startup.

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_EXTRACTION_MODE` | `llm` | Fleet default: `llm` or `decision`. Document beats workspace beats this value. |
| `EDGEQUAKE_DECISION_ENABLED` | on | `0` locks it off. `workspace` lets each workspace opt in. |
| `EDGEQUAKE_DECISION_BACKEND` | `ollama_system_one` | The only shipped backend. `openai_logprobs` is refused at boot. |
| `EDGEQUAKE_DECISION_BASE_URL` | `http://localhost:11434` | Does not follow `OLLAMA_HOST`. |
| `EDGEQUAKE_DECISION_MODEL` | `tev1:0.8b` | Default model tag. |
| `EDGEQUAKE_DECISION_PACK_SIZE` | `4` | Questions per request (1 to 16). |
| `EDGEQUAKE_DECISION_GATE_PRESET` | `balanced` | `strict`, `balanced` or `recall`. Uncalibrated. |
| `EDGEQUAKE_DECISION_TIMEOUT_SECS` | `600` | Per-request timeout. |
| `EDGEQUAKE_DECISION_KEEP_ALIVE` | `30m` | Ollama `keep_alive`. |
| `EDGEQUAKE_DECISION_CACHE_TTL_DAYS`, `EDGEQUAKE_DECISION_CACHE_MAX_ROWS` | `30`, `200000` | Answer cache limits. |

It needs schema 166 or newer (`decision_cache`, `decision_review`).

## Security and authentication

Full guides: [Enable login](auth-quickstart.md) and [Runtime auth hardening](runtime-auth-hardening.md).

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_AUTH_ENABLED` (alias `AUTH_ENABLED`) | on | Explicit setting. Wins over dev mode. |
| `EDGEQUAKE_AUTH_DISABLED` | unset | `true` turns auth off when `EDGEQUAKE_AUTH_ENABLED` is unset. |
| `EDGEQUAKE_DEV_MODE` | `false` | Open API for local use. Auth follows it when no explicit setting exists. |
| `JWT_SECRET` | insecure default | 32 or more bytes. Fatal at boot if weak and dev mode is off. |
| `JWT_EXPIRY_SECONDS` | `900` | Access token lifetime. |
| `REFRESH_TOKEN_EXPIRY_DAYS` | `30` | Refresh token lifetime. |
| `MAX_LOGIN_ATTEMPTS`, `LOCKOUT_DURATION_MINUTES` | `5`, `15` | Account lockout. |
| `ALLOW_REGISTRATION` | `true` | Set `false` in production. |
| `EDGEQUAKE_ALLOW_ANONYMOUS` | `true` | Shared guest user for chat when unauthenticated. |
| `EDGEQUAKE_MASTER_API_KEY` | none | Master key (aliases `EDGEQUAKE_GLOBAL_API_KEY`, `MASTER_API_KEY`). |
| `EDGEQUAKE_API_KEYS` | none | Comma-separated static keys. |
| `EDGEQUAKE_CORS_ORIGINS` | none | Allowed origins. Required with dev mode off and a remote database. |
| `EDGEQUAKE_RATE_LIMIT_ENABLED` | `false` | Turn it on in production. |
| `EDGEQUAKE_STRICT_STARTUP` | `false` | Turn startup warnings into fatal errors. |
| `EDGEQUAKE_SECRETS_KEY` | none | AES-256-GCM key for stored connection secrets (32 raw bytes, base64 or 64 hex characters). |
| `EDGEQUAKE_SECRETS_KEY_ID` | `v1` | Key label stored with each secret. |
| `EDGEQUAKE_SETUP_TOKEN` | none | When set, `POST /api/v1/setup/initialize` requires it. |
| `EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME`, `_PASSWORD`, `_EMAIL` | `admin`, none, `<user>@localhost` | First admin (auth on, dev mode off). |

Web UI variables (set at runtime unless noted): `NEXT_PUBLIC_AUTH_ENABLED` and `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` (build-time in custom builds), `EDGEQUAKE_API_URL` (runtime API URL), and `EDGEQUAKE_HEALTH_POLL_MS` (optional health poll interval; unset means one probe on load).

## models.toml

`models.toml` describes providers and model cards. The bundled copy is `edgequake/models.toml`. Copy and edit it, then point `EDGEQUAKE_MODELS_CONFIG` at it.

```toml
[defaults]
llm_provider = "openai"
llm_model = "gpt-4.1-mini"
embedding_provider = "openai"
embedding_model = "text-embedding-3-small"
vision_provider = "openai"
vision_model = "gpt-4o"

[[providers]]
name = "openai"
display_name = "OpenAI"
type = "openai"
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
enabled = true
priority = 10

[[providers.models]]
name = "gpt-4.1-mini"
display_name = "GPT-4.1 Mini"
model_type = "llm"                 # or "embedding"

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
embedding_dimension = 0            # 0 for LLMs, above 0 for embeddings

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006
```

Provider `type` values: `openai`, `anthropic`, `mistral`, `gemini`, `vertexai`, `xai`, `openrouter`, `minimax`, `azure`, `ollama`, `lmstudio`, `mock`. The `mock` provider is for tests: the server refuses it for real work unless `EDGEQUAKE_ALLOW_MOCK_PROVIDER=1`.

## Change providers at runtime

```bash
curl http://localhost:8080/api/v1/config/effective | jq .      # effective settings
curl http://localhost:8080/api/v1/settings/providers           # providers for the UI
curl http://localhost:8080/api/v1/models                       # models by provider
curl http://localhost:8080/api/v1/models/openai                # one provider
```

If auth is on, add `-H "Authorization: Bearer $TOKEN"` or `-H "X-API-Key: $KEY"`. A query can name a provider for one request:

```bash
curl -X POST http://localhost:8080/api/v1/query -H "Content-Type: application/json" \
  -d '{"query":"What is quantum computing?","mode":"hybrid","llm_provider":"openai","llm_model":"gpt-4.1-mini"}'
```

A workspace can store its own models. Create one with `POST /api/v1/tenants/{tenant_id}/workspaces` and fields such as `llm_provider`, `llm_model`, `embedding_provider` and `embedding_model`. Workspace settings apply to every operation in that workspace. Changing the embedding model of a workspace with data needs a rebuild: see [Embedding registry backfill](embedding-registry-backfill.md).

## Examples

```bash
# Local with Ollama
export OLLAMA_HOST="http://localhost:11434"
export EDGEQUAKE_DEFAULT_LLM_PROVIDER=ollama EDGEQUAKE_DEFAULT_LLM_MODEL="gemma4:latest"
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=ollama EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="embeddinggemma:latest"
make dev

# Production sketch
export DATABASE_URL="postgresql://edgequake:$DB_PASS@db.example.com:5432/edgequake?sslmode=require"
export OPENAI_API_KEY="$OPENAI_KEY"
export EDGEQUAKE_DEV_MODE=false EDGEQUAKE_AUTH_ENABLED=true
export JWT_SECRET="$(openssl rand -hex 32)" EDGEQUAKE_CORS_ORIGINS="https://app.example.com"
export EDGEQUAKE_STRICT_STARTUP=1
edgequake migrate && edgequake
```

## Check your configuration

Run `edgequake doctor` (add `--json` for scripts). It checks `DATABASE_URL`, the secrets key, `JWT_SECRET` and the bind host. Exit code 0 means all passed, 1 means the database check failed, and 2 means another check failed. The API also logs a warning or exits 1 at startup for the checks in [Runtime auth hardening](runtime-auth-hardening.md#what-the-api-checks-at-startup). `POST /api/v1/providers/test` tests a provider before you save it.

## See also

- [Deployment](deployment.md)
- [Monitoring](monitoring.md)
- [Performance tuning](performance-tuning.md)
- [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md)
- [REST API reference](../api-reference/rest-api.md)
