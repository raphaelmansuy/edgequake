---
title: "Configuration Reference"
---

> **Product: v0.26.5** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

# Configuration Reference

> **Complete EdgeQuake Configuration Options**

EdgeQuake is configured through environment variables and a `models.toml` file. This reference covers all available options.

---

## Configuration Sources

```
┌─────────────────────────────────────────────────────────────────┐
│                   CONFIGURATION PRIORITY                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Environment Variables (highest priority)                    │
│     │                                                           │
│  2. models.toml (for LLM/embedding configuration)               │
│     │   - EDGEQUAKE_MODELS_CONFIG env var path                  │
│     │   - ./models.toml (current directory)                     │
│     │   - ~/.edgequake/models.toml                              │
│     │   - Built-in defaults                                     │
│     │                                                           │
│  3. Compile-time defaults (lowest priority)                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Environment Variables

### Core Settings

| Variable         | Type    | Default           | Description             |
| ---------------- | ------- | ----------------- | ----------------------- |
| `HOST`           | String  | `0.0.0.0`         | Server bind address     |
| `PORT`           | Integer | `8080`            | Server port             |
| `RUST_LOG`       | String  | `edgequake=debug` | Log level filter        |
| `WORKER_THREADS` | Integer | CPU count         | Background task workers |

### Database

| Variable       | Type   | Default | Description                  |
| -------------- | ------ | ------- | ---------------------------- |
| `DATABASE_URL` | String | None    | PostgreSQL connection string |

**Connection String Format:**

```
postgresql://user:password@host:port/database?sslmode=require
```

**Examples:**

```bash
# Local development
DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"

# Production with SSL
DATABASE_URL="postgresql://edgequake:pass@db.example.com:5432/edgequake?sslmode=require"

# With connection pooling
DATABASE_URL="postgresql://edgequake:pass@pgbouncer:6432/edgequake"
```

#### Connection pool (SPEC-112)

Serving uses a four-role `PgPoolBundle` (query / ingest / queue / admin). Idle backends are **held capacity** on shared PostgreSQL — size for co-tenants. See [`specs/112-connection-pool/07-ops-runbook.md`](../../specs/112-connection-pool/07-ops-runbook.md).

| Variable | Default | Description |
| -------- | ------- | ----------- |
| `EDGEQUAKE_DB_POOL_SIZE_QUERY` | `16` | Query pool max (clamp 1–128) |
| `EDGEQUAKE_DB_POOL_SIZE_INGEST` | `12` | Ingest pool max |
| `EDGEQUAKE_DB_POOL_SIZE_QUEUE` | `4` | Queue pool max (boot floors to resolved `WORKER_THREADS` so `claim_next` cannot stampede) |
| `EDGEQUAKE_DB_POOL_SIZE_ADMIN` | `2` | Admin/migrate pool max |
| `EDGEQUAKE_DB_POOL_INSTANCE_COUNT` | `1` | Replica count for startup budget math (use peak overlap during rollouts) |
| `EDGEQUAKE_DB_POOL_BUDGET_MODE` | `warn` | `warn` or `fail` when `instances × pool_sum` exceeds `max_connections − reserve − 10` |
| `EDGEQUAKE_DB_POOL_IDLE_TIMEOUT_SECS` | `600` | sqlx idle reap |
| `EDGEQUAKE_DB_POOL_MAX_LIFETIME_SECS` | `1800` | sqlx max connection lifetime |
| `EDGEQUAKE_DB_IDLE_IN_XACT_TIMEOUT_SECS` | `60` | Session `idle_in_transaction_session_timeout` |
| `DATABASE_READ_URL` | unset | Optional read replica URL for the query pool |

Backends set `application_name=edgequake:<role>` for `pg_stat_activity` attribution. Graceful shutdown closes all role pools after HTTP drain.

**Queue vs workers:** `claim_next` uses the queue pool only. At boot, queue max becomes `max(EDGEQUAKE_DB_POOL_SIZE_QUEUE, resolved_worker_count)`. If logs show `pool timed out` on `claim_next` followed by `SSLRequest: 0x00`, Postgres likely restarted — wait until PG is healthy, then restart the API so pools re-form.

**Shared-DB starting point (co-tenant with QL):**

```bash
export EDGEQUAKE_DB_POOL_SIZE_QUERY=8
export EDGEQUAKE_DB_POOL_SIZE_INGEST=6
export EDGEQUAKE_DB_POOL_SIZE_QUEUE=2
export EDGEQUAKE_DB_POOL_SIZE_ADMIN=1
# sum = 17 per process
```

### LLM Providers

#### OpenAI

| Variable          | Type   | Default                     | Description                          |
| ----------------- | ------ | --------------------------- | ------------------------------------ |
| `OPENAI_API_KEY`  | String | None                        | OpenAI API key (required for OpenAI) |
| `OPENAI_BASE_URL` | String | `https://api.openai.com/v1` | API endpoint                         |
| `OPENAI_ORG_ID`   | String | None                        | Organization ID (optional)           |

#### Ollama

| Variable                 | Type   | Default                  | Description                                                                                                      |
| ------------------------ | ------ | ------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `OLLAMA_HOST`            | String | `http://localhost:11434` | Ollama server URL (LLM and embeddings)                                                                           |
| `OLLAMA_MODEL`           | String | `gemma4:latest`          | Default LLM model (`make dev` sets this when using Ollama)                                                       |
| `OLLAMA_EMBEDDING_MODEL` | String | `embeddinggemma:latest`  | Default embedding model (`make dev` sets this when using Ollama)                                                 |
| `OLLAMA_EMBEDDING_HOST`  | String | value of `OLLAMA_HOST`   | Dedicated Ollama host for embeddings only (closes [#140](https://github.com/raphaelmansuy/edgequake/issues/140)) |
| `EDGEQUAKE_OLLAMA_THINK_CAPABILITY` | String | `auto` | SPEC-113: Ollama `think` gate — `auto` (probe `/api/show` capabilities), `force_off`, `force_on` (debug), `legacy_name` (old substring heuristic) |
| `EDGEQUAKE_OLLAMA_CAPABILITY_TTL_SECS` | Integer | `300` | SPEC-113: TTL for cached Ollama thinking capability answers |
| `EDGEQUAKE_OLLAMA_CAPABILITY_TIMEOUT_MS` | Integer | `2000` | SPEC-113: timeout for `/api/show` capability probe; on failure Auto **omits** `think` |

> **SPEC-113:** EdgeQuake does **not** assume every `qwen3*` name supports thinking. Truth is Ollama `capabilities` (`thinking`). When capability is unknown or absent, the client omits the `think` parameter so non-thinking VL variants (e.g. many `qwen3-vl-*`) keep working.

#### LM Studio

| Variable             | Type   | Default                 | Description          |
| -------------------- | ------ | ----------------------- | -------------------- |
| `LM_STUDIO_BASE_URL` | String | `http://localhost:1234` | LM Studio server URL |

#### Anthropic

| Variable             | Type   | Default                     | Description                  |
| -------------------- | ------ | --------------------------- | ---------------------------- |
| `ANTHROPIC_API_KEY`  | String | None                        | Anthropic API key (required) |
| `ANTHROPIC_BASE_URL` | String | `https://api.anthropic.com` | API endpoint                 |

#### Google Gemini (Developer API)

| Variable          | Type   | Default                                     | Description       |
| ----------------- | ------ | ------------------------------------------- | ----------------- |
| `GEMINI_API_KEY`  | String | None                                        | Google AI API key |
| `GOOGLE_API_KEY`  | String | None                                        | Alias for Gemini  |
| `GEMINI_BASE_URL` | String | `https://generativelanguage.googleapis.com` | API endpoint      |

> **Not Vertex AI.** Gemini Developer API uses a static API key. Enterprise Vertex AI uses OAuth2 identity — see below.

#### Google Vertex AI (Enterprise)

Vertex AI (`vertexai` provider) authenticates with **short-lived OAuth2 bearer tokens** minted from GCP identity — not a static API key. The Settings → Provider Status Hub shows **Identity (ADC)** and structured requirements.

| Variable                         | Type   | Default        | Description                                                                 |
| -------------------------------- | ------ | -------------- | --------------------------------------------------------------------------- |
| `GOOGLE_CLOUD_PROJECT`           | String | None           | **Required.** GCP project ID                                                |
| `GOOGLE_CLOUD_REGION`            | String | `us-central1`  | Regional endpoint (`{region}-aiplatform.googleapis.com`)                    |
| `GOOGLE_CLOUD_LOCATION`          | String | —              | Alias for region (some Google SDKs)                                         |
| `GOOGLE_ACCESS_TOKEN`            | String | None           | Explicit bearer token (~1 h TTL; CI/debug only)                             |
| `GOOGLE_APPLICATION_CREDENTIALS` | String | None           | Path to service account JSON or WIF config                                  |

**Auth resolution ladder** (first match wins at runtime):

1. `GOOGLE_ACCESS_TOKEN` — use as-is
2. GCE/GKE/Cloud Run metadata server (attached service account; auto-refresh)
3. ADC file (`~/.config/gcloud/application_default_credentials.json`)
4. Service account key via `GOOGLE_APPLICATION_CREDENTIALS`
5. `gcloud auth application-default print-access-token` (local dev)

**Local development:**

```bash
# Correct ADC login (common mistake: swapping the last two words)
gcloud auth application-default login

export GOOGLE_CLOUD_PROJECT=your-gcp-project
export GOOGLE_CLOUD_REGION=europe-west1   # optional

# If ~/.edgequake/models.toml lacks vertexai, use the bundled catalog:
export EDGEQUAKE_MODELS_CONFIG=/path/to/edgequake/edgequake/models.toml

make dev
```

**Production:** Prefer an attached workload service account (GCE/GKE/Cloud Run) with `roles/aiplatform.user`. Avoid long-lived SA key files when metadata-based auth is available.

**Stale ADC:** An expired token file may show requirements as satisfied while health remains offline — re-run `gcloud auth application-default login`.

Design reference: [SPEC-043 §011 — Vertex AI Authentication](../specs/043-update-edgequake-llm/011-vertexai-authentication.md).

#### xAI (Grok)

| Variable       | Type   | Default               | Description  |
| -------------- | ------ | --------------------- | ------------ |
| `XAI_API_KEY`  | String | None                  | xAI API key  |
| `XAI_BASE_URL` | String | `https://api.x.ai/v1` | API endpoint |

#### OpenRouter

| Variable              | Type   | Default                     | Description                   |
| --------------------- | ------ | --------------------------- | ----------------------------- |
| `OPENROUTER_API_KEY`  | String | None                        | OpenRouter API key (required) |
| `OPENROUTER_BASE_URL` | String | `https://openrouter.ai/api` | API endpoint                  |

#### MiniMax

| Variable           | Type   | Default                     | Description                                                |
| ------------------ | ------ | --------------------------- | ---------------------------------------------------------- |
| `MINIMAX_API_KEY`  | String | None                        | MiniMax API key (required)                                 |
| `MINIMAX_BASE_URL` | String | `https://api.minimax.io/v1` | API endpoint (use `https://api.minimaxi.com/v1` for China) |

#### Azure OpenAI

| Variable                   | Type   | Default              | Description                 |
| -------------------------- | ------ | -------------------- | --------------------------- |
| `AZURE_OPENAI_API_KEY`     | String | None                 | Azure OpenAI key (required) |
| `AZURE_OPENAI_ENDPOINT`    | String | None                 | Azure resource endpoint     |
| `AZURE_OPENAI_API_VERSION` | String | `2024-02-15-preview` | API version                 |

### Models Configuration

> **Three layers of defaults** — docs and operators often conflate these:
>
> | Layer | When it applies | LLM provider / model | Embedding / dim |
> | ----- | --------------- | -------------------- | --------------- |
> | **Bundled catalog** (`models.toml` `[defaults]`, compiled constants) | No env overrides, direct `cargo run` | `openai` / `gpt-4.1-mini` | `text-embedding-3-small` / `1536` |
> | **`.env.example`** (copy to `.env`) | Explicit operator pins for production / CI | `openai` / `gpt-5-mini` | `text-embedding-3-small` / `1536` |
> | **`make dev`** (no `OPENAI_API_KEY`) | Local stack via Makefile | `ollama` / `gemma4:latest` | `embeddinggemma:latest` / `768` |
> | **`make dev`** (with `OPENAI_API_KEY`) | Local stack via Makefile | `openai` / `gpt-5-nano` | `text-embedding-3-small` / `1536` |
>
> **Makefile vs `.env.example`:** The Makefile pins models at launch time for local dev (`gpt-5-nano` when `OPENAI_API_KEY` is set, otherwise `gemma4:latest`). `.env.example` documents production-style pins (`gpt-5-mini`) — copy and edit for deployments; do not assume `make dev` reads your `.env` model pins unless exported before `make dev`.

**Primary variables** (recommended — `make dev` sets these):

| Variable                              | Type    | Default (bundled) | Description                         |
| ------------------------------------- | ------- | ----------------- | ----------------------------------- |
| `EDGEQUAKE_DEFAULT_LLM_PROVIDER`      | String  | `openai`          | Default LLM provider (priority 1)   |
| `EDGEQUAKE_DEFAULT_LLM_MODEL`         | String  | `gpt-4.1-mini`    | Default LLM model (priority 1)      |
| `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`| String  | `openai`          | Default embedding provider          |
| `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`   | String  | `text-embedding-3-small` | Default embedding model      |
| `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION` | Integer | `1536`          | Default embedding vector dimension  |

**Secondary / deployment variables** (single-env aliases, lower priority than `EDGEQUAKE_DEFAULT_*`):

| Variable                        | Type    | Default (bundled) | Description                                    |
| ------------------------------- | ------- | ----------------- | ---------------------------------------------- |
| `EDGEQUAKE_MODELS_CONFIG`       | String  | None              | Path to custom models.toml                     |
| `EDGEQUAKE_LLM_PROVIDER`        | String  | (see above)       | LLM provider alias                             |
| `EDGEQUAKE_LLM_MODEL`           | String  | None              | LLM model alias                                |
| `EDGEQUAKE_EMBEDDING_PROVIDER`  | String  | (see above)       | Embedding provider alias (hybrid mode)         |
| `EDGEQUAKE_EMBEDDING_MODEL`     | String  | None              | Embedding model alias                          |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Integer | `1536`            | Embedding vector dimension alias               |

**Vision / PDF extraction** (resolution chain in `vision_env.rs`):

| Variable                   | Type   | Default (when unset) | Description                          |
| -------------------------- | ------ | -------------------- | ------------------------------------ |
| `EDGEQUAKE_VISION_PROVIDER`| String | `EDGEQUAKE_VISION_PROVIDER` → `EDGEQUAKE_DEFAULT_LLM_PROVIDER` → `EDGEQUAKE_LLM_PROVIDER` → **`ollama`** | Vision LLM provider for PDF→Markdown |
| `EDGEQUAKE_VISION_MODEL`   | String | First compatible env model in chain, else provider default: **`gemma4:latest`** (ollama), `gpt-4.1-nano` (openai), `mistral-small-latest` (mistral) | Vision LLM model for PDF→Markdown    |

> **Unset vision env:** With no vision or LLM env vars, the server defaults to **`ollama` / `gemma4:latest`** — not the bundled `models.toml` openai pins. `make dev` sets vision to match the resolved `EDGEQUAKE_DEFAULT_*` pair. `.env.example` shows cloud pins (`openai` / `gpt-4.1-nano`) as commented production examples.

### Application Attribution (SPEC-043)

Identifies EdgeQuake to upstream LLM providers (OpenRouter HTTP referer, OpenAI client request ID, Anthropic application ID, Google `x-goog-api-client`, etc.). Built once per request via `ApplicationContext` and passed to `create_llm_provider_with_context`.

| Variable | Type | Default | Description |
| -------- | ---- | ------- | ----------- |
| `EDGEQUAKE_APP_ID` | String | None | Stable application identifier sent upstream |
| `EDGEQUAKE_APP_NAME` | String | None | Human-readable application name (OpenRouter title) |
| `EDGEQUAKE_APP_URL` | String | None | Application URL (OpenRouter HTTP-Referer) |
| `EDGEQUAKE_TENANT_ID` | String | None | Optional tenant identifier for multi-tenant attribution |

**Per-request overrides** (merged into `ApplicationContext` when present):

| Header | Maps to |
| ------ | ------- |
| `x-edgequake-app-id` | `app_id` |
| `x-edgequake-app-name` | `app_name` |
| `x-edgequake-app-url` | `app_url` |
| `x-edgequake-tenant-id` | `tenant_id` |
| `x-edgequake-request-id` | `request_id` |

**Example — identify EdgeQuake to OpenRouter:**

```bash
export EDGEQUAKE_APP_ID=edgequake
export EDGEQUAKE_APP_NAME="EdgeQuake"
export EDGEQUAKE_APP_URL=https://edgequake.example.com
```

**Resolution order for attribution fields:** `server_config.app_attribution` → overridden by env vars (`EDGEQUAKE_APP_*`) → overridden per-request by ingress headers.

**WebUI persistence:** Settings → Application Attribution → PATCH `/api/v1/settings/app-attribution` (admin, PostgreSQL). Applied immediately without restart. See [REST API — Application Attribution](/docs/api-reference/rest-api#application-attribution).

**Discovery:** `GET /api/v1/settings/attribution` returns the effective context plus per-provider header catalog. `/health` includes a compact `attribution` block (`app_id`, `app_name`, `active`).

### Compatibility aliases

EdgeQuake also accepts the following migration aliases. They are normalized at startup so the rest
of the application continues to use the canonical `EDGEQUAKE_*` names:

| Alias                 | Canonical variable              |
| --------------------- | ------------------------------- |
| `MODEL_PROVIDER`      | `EDGEQUAKE_LLM_PROVIDER`        |
| `CHAT_MODEL`          | `EDGEQUAKE_LLM_MODEL`           |
| `EMBEDDING_PROVIDER`  | `EDGEQUAKE_EMBEDDING_PROVIDER`  |
| `EMBEDDING_MODEL`     | `EDGEQUAKE_EMBEDDING_MODEL`     |
| `EMBEDDING_DIMENSION` | `EDGEQUAKE_EMBEDDING_DIMENSION` |

When both an alias and a canonical variable are set, the canonical variable wins.

### Hybrid Provider Mode (closes [#140](https://github.com/raphaelmansuy/edgequake/issues/140))

Run a different provider or Ollama instance for embeddings vs. LLM inference:

| Variable                        | Type    | Default                | Description                                         |
| ------------------------------- | ------- | ---------------------- | --------------------------------------------------- |
| `OLLAMA_EMBEDDING_HOST`         | String  | value of `OLLAMA_HOST` | Dedicated Ollama host for embeddings                |
| `EDGEQUAKE_EMBEDDING_PROVIDER`  | String  | (same as LLM)          | Explicit embedding provider (`ollama`, `openai`, …) |
| `EDGEQUAKE_EMBEDDING_MODEL`     | String  | provider default       | Model for the embedding override                    |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Integer | `1536` (OpenAI) / `768` (Ollama) | Vector dimension for the embedding override |

**Priority:** `EDGEQUAKE_EMBEDDING_PROVIDER` → `OLLAMA_EMBEDDING_HOST` → default (from `from_env()`).

**Example — OpenAI for LLM, dedicated Ollama node for embeddings:**

```bash
export EDGEQUAKE_LLM_PROVIDER=openai
export OPENAI_API_KEY=sk-...
export OLLAMA_EMBEDDING_HOST=http://gpu-box:11434
export OLLAMA_EMBEDDING_MODEL=nomic-embed-text
```

### Pipeline Timeout & Concurrency (fixes [#194](https://github.com/raphaelmansuy/edgequake/issues/194))

Controls how aggressively the ingestion pipeline calls the LLM and how long it waits for each
response. These are the knobs to reach for when processing **large documents** or using **slow
local LLMs** (Ollama, LM Studio on CPU or a single GPU).

| Variable                               | Type    | Default | Min  | Max     | Description                                                |
| -------------------------------------- | ------- | ------- | ---- | ------- | ---------------------------------------------------------- |
| `EDGEQUAKE_CHUNK_TIMEOUT_SECS`         | Integer | `180`   | `10` | ∞       | Per-chunk LLM call timeout in seconds                      |
| `EDGEQUAKE_CHUNK_MAX_RETRIES`          | Integer | `3`     | `0`  | `20`    | Max retry attempts per chunk on timeout or error           |
| `EDGEQUAKE_CHUNK_RETRY_DELAY_MS`       | Integer | `1000`  | `0`  | `60000` | Initial backoff delay between retries (milliseconds)       |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | Integer | `16`    | `1`  | `256`   | Max parallel LLM extraction calls per document             |
| `EDGEQUAKE_LLM_TIMEOUT_SECS`           | Integer | `600`   | —    | `3600`  | HTTP safety-layer timeout (Layer 2, supports up to 1 hour) |

**Two-layer timeout architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                   TIMEOUT LAYERS                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Layer 1 — EDGEQUAKE_CHUNK_TIMEOUT_SECS  (pipeline, fires first)│
│    └─ Set this to allow the LLM enough time per chunk           │
│                                                                 │
│  Layer 2 — EDGEQUAKE_LLM_TIMEOUT_SECS   (HTTP safety cap)      │
│    └─ Must be ≥ EDGEQUAKE_CHUNK_TIMEOUT_SECS                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Quick configuration for slow local LLMs:**

```bash
# Large document on a single-GPU Ollama instance
export EDGEQUAKE_CHUNK_TIMEOUT_SECS=600       # 10 minutes per chunk
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4  # reduce parallelism
export EDGEQUAKE_LLM_TIMEOUT_SECS=3600        # 1-hour HTTP cap
```

> **Note:** Values below the allowed minimum are automatically clamped.
> Non-numeric values are silently ignored and the default is used.

---

### Worker Pool, Task Lease & Fairness (SPEC-057)

Postgres task rows are the delivery SSOT; the in-memory channel is a wake signal only. See [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md).

| Variable | Type | Default | Description |
| -------- | ---- | ------- | ----------- |
| `WORKER_THREADS` | Integer | CPU count | Background worker count |
| `MAX_TASKS_PER_TENANT` | Integer | ≈ ¾ of `WORKER_THREADS` | Per-tenant concurrency cap; `0` disables |
| `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY` | Boolean | `0` | When unset/`0`, `ollama`/`lmstudio` clamp to **1** task/tenant |
| `EDGEQUAKE_EXTRACT_PROVIDER` | String | — | Hybrid extract provider for local clamp (P2) |
| `EDGEQUAKE_DEFAULT_EXTRACT_PROVIDER` | String | — | Fallback extract provider for clamp |
| `EDGEQUAKE_TASK_LEASE_TTL_SECS` | Integer | `120` (min `30`) | Claim lease TTL; heartbeat every 60s |
| `EDGEQUAKE_STARTUP_AUTO_RESUME` | Boolean | `1` (unset) | Default **ON**: reclaim stale **Processing** → Pending. Set `0`/`false`/`off` to mark Interrupted Failed (manual Reprocess) |
| `EDGEQUAKE_STARTUP_RECONCILE_MAX` | Integer | `32` | Max orphan rows reconciled at boot |
| `EDGEQUAKE_REPLICAS` | Integer | `1` | Intended API/worker process count |
| `EDGEQUAKE_TASK_DELIVERY` | String | `local` | `local` \| `bridged` \| `notify_only`; boot fails if `REPLICAS>1` and `local` |
| `EDGEQUAKE_DB_POOL_UTIL_WARN` | Float | `0.75` | Store contention warn threshold |
| `EDGEQUAKE_DB_POOL_UTIL_CRITICAL` | Float | `0.90` | Store contention critical (`/ready` 503) |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_WARN` | Integer | `1` | Compensation DLQ warn |
| `EDGEQUAKE_COMPENSATION_QUARANTINE_CRITICAL` | Integer | `5` | Compensation DLQ critical (`/ready` 503) |
| `EDGEQUAKE_NATIVE_GRAPH_WRITES` | Boolean | `1` | Native AGE upserts; `0` forces Cypher MERGE fallback |
| `EDGEQUAKE_HNSW_ITERATIVE_SCAN` | String | `relaxed_order` | pgvector ≥0.8 iterative scan mode |
| `EDGEQUAKE_HNSW_EF_CONSTRUCTION` | Integer | `32` local / `128` prod | HNSW build param for **new** indexes only |

**Multi-replica:** Set `EDGEQUAKE_REPLICAS>1` and `EDGEQUAKE_TASK_DELIVERY=bridged` (or `notify_only`). Correctness remains `claim_next` + lease — never process from channel payload alone.

**Convert vs ingest (P2):** PDF admission enqueues convert-only (`pdf_processing`); after durable markdown, a separate `insert` task runs under its own lease/timeout. Cancel semantics: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md).

---

### SPEC-091 Data Layer & SPEC-103 LLM Cache (v0.23.0)

Relational data-layer cutover (typed SSOT) and the LightRAG-parity response cache. Most defaults assume a **post-drop** database; mid-upgrade fleets must follow the [SPEC-091 runbook](spec091-upgrade-from-v0.22.0.md).

| Variable | Type | Default | Description |
| -------- | ---- | ------- | ----------- |
| `EDGEQUAKE_MIGRATION_MODE` | String | `verify` | Migration engine mode (`off` \| `verify` \| `automatic`); `edgequake migrate` console is the canonical path |
| `EDGEQUAKE_MIGRATION_CONFIRM_DROP` | Boolean | `0` | Acknowledge irreversible drops (**125** KV, **126/131** vectors); also via `edgequake migrate --confirm-drop` (alias `--drop-confirm`, SPEC-137) |
| `EDGEQUAKE_VECTOR_BACKEND` | String | `typed_embeddings` | Vector write SSOT; `legacy_tables` only for pre-drop fleets |
| `EDGEQUAKE_CHUNK_TEXT_AUTHORITY` | String | `relational` | Chunk-text SSOT (`relational` post-SPEC-091; `kv` pre-drop) |
| `EDGEQUAKE_KV_FAMILY_*` | String | `relational` | Per-family KV routing (`artifact`, `cache`, `checkpoint`, `compensation_quarantine`, `doc_hash`, `injection`, `metadata`, `wsdoc`) |
| `EDGEQUAKE_SERVING_FENCE` | Boolean | `1` | Fail-closed serving when typed tables are missing; set `off`/`false`/`0` to disable |
| `EDGEQUAKE_OUTBOX_DRAIN` | String | `on` | Outbox drain mode (`off` \| `dry-run` \| `on`) for ingest compensations |
| `EDGEQUAKE_CITATION_REQUIRE` | Boolean | `1` | Fail-closed citation requirement (`source_chunk_ids`) on merges |
| `EDGEQUAKE_CONTEXTUAL_CHUNK` | Boolean | `0` | Contextual chunk preamble injection (default off) |
| `EDGEQUAKE_LLM_CACHE` | Boolean | `1` | **Master** LLM cache switch (keywords + answers); set `0`/`false` to disable both |
| `EDGEQUAKE_KEYWORD_CACHE` | Boolean | follows master | Keyword-extraction cache override (SPEC-103) |
| `EDGEQUAKE_QUERY_ANSWER_CACHE` | Boolean | follows master | Query-answer cache override (SPEC-103) |
| `EDGEQUAKE_PROMPT_CACHE` | Boolean | `1` | **Provider KV / prompt-cache**. Native OpenAI (constructor, including proxies) and Azure send `prompt_cache_key` + GPT-5.6 explicit breakpoints; a structured `error.param` 400 disables them for that process. Compatible/Mistral/NVIDIA: key only. Anthropic: `cache_control`. OpenRouter: `cache_control` + `session_id`. Bedrock Converse: `cachePoint`. Default **on**. Does not skip generation. Acc leaves this on. |
| `EDGEQUAKE_PROMPT_CACHE_TTL` | String | `5m` | Anthropic `cache_control` and Bedrock Converse `cachePoint` TTL (`5m` or `1h`) |
| `EDGEQUAKE_LLM_OMIT_TEMPERATURE` | Boolean | `0` | **SPEC-131 / #379** — never send `temperature` upstream (Mantle Gemma/Grok reject it). |
| `EDGEQUAKE_LLM_OMIT_REASONING_EFFORT` | Boolean | `0` | **SPEC-131** — never send `reasoning_effort` upstream. |
| `EDGEQUAKE_LLM_API_FORMAT` | String | `chat_completions` | **SPEC-131** — upstream transport: `chat_completions` (default) or `responses` (GPT-5.6 Mantle; `store: false`). |
| `EDGEQUAKE_EXTRACTION_LANGUAGE` | String | `English` | Fleet default KG extraction NL language (SPEC-096); workspace metadata overrides |
| `LANGFUSE_PUBLIC_KEY` | String | (unset) | **SPEC-124** — Langfuse public key (`pk-lf-…`). With secret key, enables Langfuse export. |
| `LANGFUSE_SECRET_KEY` | String | (unset) | **SPEC-124** — Langfuse secret key (`sk-lf-…`); never logged / never shown in UI. |
| `LANGFUSE_BASE_URL` | String | `https://cloud.langfuse.com` | **SPEC-124** — Langfuse UI + OTLP base (alias `LANGFUSE_HOST`). Local v4: `http://localhost:3310` after `make langfuse-up`. |
| `LANGFUSE_PROJECT_ID` | String | (auto) | **SPEC-124** — optional project id for Settings deep-link; else fetched once from `/api/public/projects`. |
| `EDGEQUAKE_LANGFUSE_ENABLED` | Boolean | follows keys | **SPEC-124** — force-enable when keys present; see [OBSERVABILITY.md](../OBSERVABILITY.md). |
| `EDGEQUAKE_LANGFUSE_API` | String | `auto` | **SPEC-124** — `auto` probes OTLP and falls back to native ingestion on HTTP 404 (Langfuse 3.1.x). `otlp` / `ingestion` force a transport. How-to: [langfuse-3.1.md](langfuse-3.1.md). Upgrade to ≥ 3.22 remains recommended. |

> **Acc note:** The benchmark pins `EDGEQUAKE_LLM_CACHE=0` for fair cold peers (response cache). Provider KV cache (`EDGEQUAKE_PROMPT_CACHE`) stays **on** — it does not change answers. Irreversible drops (**125** KV, **126/131** vectors) are human-gated via `edgequake migrate --confirm-drop` (alias `--drop-confirm`) or `EDGEQUAKE_MIGRATION_CONFIRM_DROP=1` — never set it casually in shared env files.

---

### Security / Authentication

| Variable                    | Type    | Default | Description                                                                 |
| --------------------------- | ------- | ------- | --------------------------------------------------------------------------- |
| `EDGEQUAKE_AUTH_ENABLED`    | Boolean | `true`  | Enable API authentication (secure default)                                  |
| `EDGEQUAKE_DEV_MODE`        | Boolean | `false` | Set `true` for local open API without keys (`make dev` sets this)           |
| `EDGEQUAKE_MASTER_API_KEY`  | String  | None    | Master API key for authenticated requests                                   |
| `EDGEQUAKE_API_KEYS`        | String  | None    | Comma-separated API keys (alternative to master key)                        |
| `EDGEQUAKE_STRICT_STARTUP`  | Boolean | `false` | Exit on insecure production config when `1`                                 |
| `EDGEQUAKE_CORS_ORIGINS`    | String  | None    | Comma-separated allowed CORS origins (`None` = allow any, legacy default)   |

### Security / Frontend

| Variable                         | Type   | Default | Description                                                                                                                             |
| -------------------------------- | ------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `NEXT_PUBLIC_DISABLE_DEMO_LOGIN` | String | `false` | Set to `true` to hide the demo "skip login" button in production (closes [#139](https://github.com/raphaelmansuy/edgequake/issues/139)) |
| `EDGEQUAKE_HEALTH_POLL_MS`       | Number | unset   | WebUI periodic `/live`+`/health` poll (ms). Unset/`0`/`false`/`off` = one probe on load. `10000` restores the former 10s loop. Runtime-injected (not baked `NEXT_PUBLIC_*`). Playwright always disables the loop. |

> **Production tip:** Keep `EDGEQUAKE_AUTH_ENABLED=true`, unset `EDGEQUAKE_DEV_MODE`, configure `EDGEQUAKE_MASTER_API_KEY` or `EDGEQUAKE_API_KEYS`, and set `NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true` in your frontend build.

---

## models.toml Reference

The `models.toml` file configures LLM providers and model cards.

### Location Priority

1. `EDGEQUAKE_MODELS_CONFIG` environment variable
2. `./models.toml` (current working directory)
3. `~/.edgequake/models.toml` (user home)
4. Built-in defaults

### Structure

```toml
# Default provider selection (bundled models.toml)
[defaults]
llm_provider = "openai"
llm_model = "gpt-4.1-mini"
embedding_provider = "openai"
embedding_model = "text-embedding-3-small"
vision_provider = "openai"
vision_model = "gpt-4o"

# Provider definitions
[[providers]]
name = "openai"
display_name = "OpenAI"
type = "openai"
api_base = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
enabled = true
priority = 10
description = "OpenAI GPT models"

# Model definitions within provider
[[providers.models]]
name = "gpt-4.1-mini"
display_name = "GPT-4.1 Mini"
model_type = "llm"                   # or "embedding"
description = "Cost-effective model with 1M context"
deprecated = false
tags = ["recommended", "fast"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
supports_function_calling = true
supports_json_mode = true
supports_streaming = true
supports_system_message = true
embedding_dimension = 0              # 0 for LLMs, >0 for embeddings

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006
embedding_per_1k = 0.0
image_per_unit = 0.0
```

### Provider Types

| Type         | Description             | API Key Variable       |
| ------------ | ----------------------- | ---------------------- |
| `openai`     | OpenAI API compatible   | `OPENAI_API_KEY`       |
| `anthropic`  | Anthropic Claude models | `ANTHROPIC_API_KEY`    |
| `mistral`    | Mistral AI models       | `MISTRAL_API_KEY`      |
| `gemini`     | Google Gemini Developer API | `GEMINI_API_KEY` / `GOOGLE_API_KEY` |
| `vertexai`   | Google Vertex AI (enterprise) | **Identity** — `GOOGLE_CLOUD_PROJECT` + ADC/SA (no static API key) |
| `xai`        | xAI Grok models         | `XAI_API_KEY`          |
| `openrouter` | OpenRouter aggregator   | `OPENROUTER_API_KEY`   |
| `minimax`    | MiniMax AI models       | `MINIMAX_API_KEY`      |
| `azure`      | Azure OpenAI            | `AZURE_OPENAI_API_KEY` |
| `ollama`     | Ollama local models     | None (local)           |
| `lmstudio`   | LM Studio local         | None (local)           |
| `mock`       | Testing without costs   | None                   |

### Model Types

| Type        | Purpose           | Key Capability                        |
| ----------- | ----------------- | ------------------------------------- |
| `llm`       | Text generation   | `context_length`, `max_output_tokens` |
| `embedding` | Vector embeddings | `embedding_dimension`                 |

---

## Provider Configuration Examples

### OpenAI (Production)

```toml
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
model_type = "llm"
tags = ["recommended"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
supports_function_calling = true
supports_json_mode = true
supports_streaming = true

[[providers.models]]
name = "text-embedding-3-small"
display_name = "Text Embedding 3 Small"
model_type = "embedding"
tags = ["recommended"]

[providers.models.capabilities]
context_length = 8191
embedding_dimension = 1536
```

### Ollama (Local Development)

```toml
[[providers]]
name = "ollama"
display_name = "Ollama"
type = "ollama"
api_base = "http://localhost:11434"
enabled = true
priority = 20

[[providers.models]]
name = "gemma4:latest"
display_name = "Gemma 4 Latest"
model_type = "llm"
tags = ["recommended", "local"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[[providers.models]]
name = "embeddinggemma:latest"
display_name = "Embedding Gemma"
model_type = "embedding"

[providers.models.capabilities]
context_length = 8192
embedding_dimension = 768
```

### Azure OpenAI

```toml
[[providers]]
name = "azure-openai"
display_name = "Azure OpenAI"
type = "openai"  # Uses OpenAI-compatible API
api_base = "https://your-resource.openai.azure.com"
api_key_env = "AZURE_OPENAI_API_KEY"
enabled = true
priority = 5

[[providers.models]]
name = "gpt-4.1-mini"  # Your deployment name
display_name = "Azure GPT-4o Mini"
model_type = "llm"

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_function_calling = true
supports_json_mode = true
supports_streaming = true
```

### Anthropic Claude

```toml
[[providers]]
name = "anthropic"
display_name = "Anthropic"
type = "anthropic"
api_base = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
enabled = true
priority = 8

[[providers.models]]
name = "claude-sonnet-4-5-20250929"
display_name = "Claude Sonnet 4.5"
model_type = "llm"
tags = ["recommended", "fast"]

[providers.models.capabilities]
context_length = 200000
max_output_tokens = 128000
supports_vision = true
supports_streaming = true
supports_system_message = true

[providers.models.cost]
input_per_1k = 0.003
output_per_1k = 0.015
```

### Google Gemini

```toml
[[providers]]
name = "gemini"
display_name = "Google Gemini"
type = "gemini"
api_base = "https://generativelanguage.googleapis.com"
api_key_env = "GEMINI_API_KEY"
enabled = true
priority = 9

[[providers.models]]
name = "gemini-2.5-flash"
display_name = "Gemini 2.5 Flash"
model_type = "llm"
tags = ["recommended", "fast", "thinking"]

[providers.models.capabilities]
context_length = 1000000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006

[[providers.models]]
name = "gemini-embedding-001"
display_name = "Gemini Embedding"
model_type = "embedding"

[providers.models.capabilities]
context_length = 10000
embedding_dimension = 3072

[providers.models.cost]
input_per_1k = 0.00015
```

### Google Vertex AI (Enterprise)

Vertex uses IAM identity auth — leave `api_key_env` empty in `models.toml`:

```toml
[[providers]]
name = "vertexai"
display_name = "Google Vertex AI"
type = "vertexai"
api_base = "https://aiplatform.googleapis.com"
api_key_env = ""   # OAuth2 identity — not a static key
enabled = true
priority = 7
description = "Enterprise Gemini on Vertex AI; IAM / ADC authentication"

[[providers.models]]
name = "gemini-2.5-flash"
display_name = "Gemini 2.5 Flash (Vertex)"
model_type = "llm"
tags = ["recommended", "enterprise"]

[providers.models.capabilities]
context_length = 1000000
max_output_tokens = 8192
supports_vision = true
supports_streaming = true

[[providers.models]]
name = "gemini-embedding-001"
display_name = "Gemini Embedding (Vertex)"
model_type = "embedding"

[providers.models.capabilities]
context_length = 10000
embedding_dimension = 3072
```

Set `GOOGLE_CLOUD_PROJECT` and authenticate via ADC or an attached service account before selecting `vertexai` in the UI or `EDGEQUAKE_LLM_PROVIDER=vertexai`.

### xAI (Grok)

```toml
[[providers]]
name = "xai"
display_name = "xAI"
type = "xai"
api_base = "https://api.x.ai/v1"
api_key_env = "XAI_API_KEY"
enabled = true
priority = 7

[[providers.models]]
name = "grok-4-1-fast"
display_name = "Grok 4.1 Fast"
model_type = "llm"
tags = ["recommended", "fast", "large-context"]

[providers.models.capabilities]
context_length = 2000000
max_output_tokens = 16384
supports_vision = false
supports_streaming = true

[providers.models.cost]
input_per_1k = 0.0002
output_per_1k = 0.0005
```

### OpenRouter

```toml
[[providers]]
name = "openrouter"
display_name = "OpenRouter"
type = "openrouter"
api_base = "https://openrouter.ai/api"
api_key_env = "OPENROUTER_API_KEY"
enabled = true
priority = 6

[[providers.models]]
name = "openai/gpt-4o-mini"
display_name = "OpenRouter GPT-4o Mini"
model_type = "llm"
tags = ["recommended"]

[providers.models.capabilities]
context_length = 128000
max_output_tokens = 16384
supports_vision = true
supports_streaming = true

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006
```

---

## Runtime Provider Switching

EdgeQuake supports switching providers at runtime via API:

```bash
# Get current effective configuration (resolution chain)
curl http://localhost:8080/api/v1/config/effective | jq .

# List available providers (Settings UI source)
curl http://localhost:8080/api/v1/settings/providers

# List all models grouped by provider
curl http://localhost:8080/api/v1/models

# Get models for a specific provider
curl http://localhost:8080/api/v1/models/openai

# Query with specific provider (per-request)
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is quantum computing?",
    "mode": "hybrid",
    "llm_provider": "openai",
    "llm_model": "gpt-4.1-mini"
  }'
```

---

## Workspace-Level Configuration

Each workspace can have its own LLM/embedding configuration:

```bash
# Create workspace with custom providers
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Workspace",
    "llm_provider": "openai",
    "llm_model": "gpt-4o",
    "embedding_provider": "openai",
    "embedding_model": "text-embedding-3-large"
  }'
```

Workspace configuration overrides server defaults for all operations within that workspace.

---

## Logging Configuration

The `RUST_LOG` environment variable controls logging:

```bash
# Debug all EdgeQuake components
RUST_LOG="edgequake=debug"

# Production logging
RUST_LOG="edgequake=info,tower_http=info"

# Verbose debugging
RUST_LOG="edgequake=trace,sqlx=debug,tower_http=debug"

# Specific component debugging
RUST_LOG="edgequake_pipeline=debug,edgequake_query=debug"
```

### Log Levels

| Level   | Use Case              |
| ------- | --------------------- |
| `error` | Errors only           |
| `warn`  | Errors + warnings     |
| `info`  | Standard production   |
| `debug` | Development debugging |
| `trace` | Detailed tracing      |

---

## Performance Tuning

### Worker Threads

```bash
# Set worker count (default: CPU count)
WORKER_THREADS=8
```

Workers handle background document processing. More workers = faster ingestion but higher memory.

### Connection Pool (PostgreSQL)

Connection pooling is built into SQLx. For high-load scenarios, use an external pooler:

```bash
# Use PgBouncer
DATABASE_URL="postgresql://user:pass@pgbouncer:6432/edgequake?application_name=edgequake"
```

### Query Tuning

| Setting        | Scope   | Default | Description            |
| -------------- | ------- | ------- | ---------------------- |
| `max_results`  | Per query | 20   | Max chunks retrieved (falls back to engine `max_chunks`) |
| `max_entities` | Engine config | 60 | Max entities retrieved |
| `temperature`  | Chat API | 0.7 | LLM temperature (chat requests only; query requests have no temperature field) |
| `max_tokens`   | HTTP safety layer | 16384 | Max response tokens (`EDGEQUAKE_LLM_MAX_TOKENS` overrides) |

---

## Example Configurations

### Development (Minimal)

```bash
# Requires DATABASE_URL — set via .env or environment
# Bundled default: openai/gpt-4.1-mini (requires OPENAI_API_KEY)
# make dev without API key uses ollama/gemma4:latest instead
make dev
```

### Development with Ollama

```bash
export OLLAMA_HOST="http://localhost:11434"
export EDGEQUAKE_DEFAULT_LLM_PROVIDER=ollama
export EDGEQUAKE_DEFAULT_LLM_MODEL="gemma4:latest"
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=ollama
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="embeddinggemma:latest"
make dev
```

### Development with PostgreSQL

```bash
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake"
export OPENAI_API_KEY="sk-..."
cargo run
```

### Production

```bash
export DATABASE_URL="postgresql://edgequake:$DB_PASS@db.example.com:5432/edgequake?sslmode=require"
export OPENAI_API_KEY="$OPENAI_KEY"
export RUST_LOG="edgequake=info,tower_http=info"
export HOST="0.0.0.0"
export PORT="8080"
export WORKER_THREADS="8"
./edgequake
```

---

## Validation

EdgeQuake validates configuration at startup:

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   ⚡ EdgeQuake v0.23.0                                        ║
║                                                              ║
║   🐘 Storage: POSTGRESQL (persistent)
║   🌐 Server:  http://0.0.0.0:8080
║   📚 Swagger: http://0.0.0.0:8080/swagger-ui/
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

Validation errors are logged with actionable messages:

```
ERROR: DATABASE_URL is invalid: invalid connection string
HINT: Format: postgresql://user:password@host:port/database
```

---

## See Also

- [Deployment Guide](/docs/operations/deployment/) - Production deployment
- [Monitoring Guide](/docs/operations/monitoring/) - Observability setup
- [Ingestion cancel & fairness](/docs/ingestion-cancel-and-fairness/) - Worker lease, fairness, cancel
- [REST API Reference](/docs/api-reference/rest-api/) - API documentation
- [LLM Provider Docs](/docs/concepts/hybrid-retrieval/) - Provider integration
