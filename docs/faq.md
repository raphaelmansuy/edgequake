---
title: "Frequently Asked Questions"
---

> **Product: v0.23.0** · Contract: [OpenAPI snapshot](../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md)

# Frequently Asked Questions

> **Common Questions About EdgeQuake**

---

## General

### What is EdgeQuake?

EdgeQuake is a **production-grade Graph-RAG framework** written in Rust. It combines:

- **Knowledge Graphs** for entity and relationship extraction
- **Vector Search** for semantic retrieval
- **LLM Integration** for natural language answers

Think of it as a smarter search engine that understands concepts, not just keywords.

### How is EdgeQuake different from vector-only RAG?

| Aspect                  | Vector-Only RAG     | EdgeQuake (Graph-RAG)   |
| ----------------------- | ------------------- | ----------------------- |
| Retrieval               | Semantic similarity | Semantic + structural   |
| Multi-hop               | ❌ Single retrieval  | ✅ Follows relationships |
| Context                 | Flat chunks         | Connected entities      |
| "What connects X to Y?" | Cannot answer       | Native query type       |

### What's the relationship to LightRAG?

EdgeQuake is a **Rust implementation inspired by [LightRAG](https://github.com/HKUDS/LightRAG)**, a Python Graph-RAG research project.

Key differences:

- **Language**: Rust vs Python (10–50× faster)
- **Production Ready**: Multi-tenant, auth, observability, deployment
- **Storage**: PostgreSQL + pgvector + Apache AGE
- **API**: REST with async ingestion, streaming, and WebSocket progress

---

## Deployment

### What are the minimum requirements?

**Development**:

- 4 GB RAM
- 2 CPU cores
- Rust 1.95+
- PostgreSQL **16, 17, or 18** with pgvector and Apache AGE (Docker image: `ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0`)

**Production (minimum to boot)**:

- **8+ GB RAM** is enough to *start* the stack — **not** enough for Proven 50k / Supported 100k filtered ANN
- Floors: [Product limits — Pick your size](product-limits.md) — ≥**16 GB** for ≤50k (**Proven**); ≥**32 GB** preferred for 100k Wave-2 (**Supported**); `shared_buffers` ≥**2 GB**
- 4+ CPU cores
- PostgreSQL 16+ with pgvector + AGE
- LLM provider (OpenAI, Ollama, or other supported provider)
- Vision-capable model for PDF ingestion (see [Vision & PDF Processing](#vision--pdf-processing))

**Ports** (defaults): API **8080**, WebUI **3000**.

### Can I run EdgeQuake without PostgreSQL?

**No** — `DATABASE_URL` is required for all server modes. In-memory storage was removed in v0.4.0. Running without a database causes the server to exit with error code 1.

For **development and testing**, use the Docker-based PostgreSQL setup:

```bash
# Start full stack (PostgreSQL + backend + frontend)
make dev

# Start PostgreSQL only, then run tests
make db-start
cargo test
```

For **production-style deployments**, use prebuilt GHCR images:

```bash
EDGEQUAKE_VERSION=0.23.0 docker compose -f docker-compose.quickstart.yml up -d
```

Images: `ghcr.io/raphaelmansuy/edgequake:0.23.0`, `ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0-pg18` (also `-pg16`, `-pg17`).

### Can I run EdgeQuake without an LLM?

**Yes**, for testing with the mock provider:

```bash
cargo test
```

For production, you need a real LLM. Options:

- OpenAI (`OPENAI_API_KEY`)
- Ollama (local, free)
- LM Studio (local, free)
- Mistral, Anthropic, Gemini, Vertex AI (see `.env.example`)

---

## Cost

### How much does it cost to run EdgeQuake?

EdgeQuake itself is **free and open source**. Costs come from:

| Component  | Cost                                   |
| ---------- | -------------------------------------- |
| EdgeQuake  | Free                                   |
| PostgreSQL | Free (self-hosted) or ~$15/mo (managed) |
| OpenAI     | ~$0.002 per document (~500 words)      |
| Ollama     | Free (local GPU)                       |

### How can I reduce LLM costs?

1. **Use cheaper models**:

   ```bash
   EDGEQUAKE_DEFAULT_LLM_MODEL=gpt-5-mini
   ```

2. **Use local LLM** (Ollama):

   ```bash
   EDGEQUAKE_DEFAULT_LLM_PROVIDER=ollama
   EDGEQUAKE_DEFAULT_LLM_MODEL=gemma4:latest
   ```

3. **Reduce chunk size** (fewer LLM calls) in pipeline configuration.

### Is there a free tier for OpenAI?

OpenAI offers free credits for new accounts. After that, `gpt-5-mini` and `text-embedding-3-small` are cost-effective defaults (see `.env.example`).

---

## Performance

### How fast is EdgeQuake?

| Operation              | Typical Time                          |
| ---------------------- | ------------------------------------- |
| Document admit (HTTP)  | < 1 s (returns **202** + `track_id`)  |
| Entity extraction      | 2–5 s per chunk (with LLM, async)     |
| Vector search          | < 100 ms                              |
| Graph traversal        | < 50 ms                               |
| Full query             | 2–10 s (depends on LLM)               |

Uploads are **async**: the API accepts the document immediately; poll `GET /api/v1/tasks/{track_id}` or subscribe via WebSocket/SSE for progress.

### How does it scale?

**SSOT:** [Product limits](product-limits.md) — start with **TL;DR** and **Pick your size**.

| Status | What you can promise | Recipe |
|--------|----------------------|--------|
| **Proven** | ≤**50k** chunk vectors @1536 under prod stress; Louvain gated at **50k** nodes | Defaults + `shared_buffers` ≥2 GB; host ≥16 GB |
| **Supported** (default) | **100k** filtered ANN @1536 (Q1-d) | **Wave-2**: `halfvec` + `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=1` + residency (host ≥32 GB preferred) |
| **Supported opt-in** | **150k** on dedicated DiskANN | `query_search_list_size≥400` + `query_rescore≈list/2` (pg18-vectorscale) — not a silent default |
| **Not promoted** | Wave-2 above 100k (250k+) | Mid-scale wall — do not sell |
| **Aspirational** | 100k+ documents / 1M+ entities per workspace | Not a latency-gated claim |

Hard caps: **50 MiB**/upload, HNSW dim ≤2000/4000. Concurrent tip @100k Wave-2: `EDGEQUAKE_HNSW_EF_SEARCH=240`. Mix/hybrid seeds ≪ ANN ladder.

There is **no** enforced per-workspace storage-GB quota. Workspace `max_documents` (when set) is fail-closed at upload / new PDF mint (SPEC-066). Size disk/RAM from [product-limits](product-limits.md). Ceiling ladder: `make ceiling-proof` ([SPEC-066](../specs/066-ceiling-proof/000-index.md)).

For multi-replica worker deployments, see [Ingestion, replicas & leases](#ingestion-cancel-fairness-replicas--convert--ingest).

### How can I speed up queries?

**Data plane (often the real cliff before LLM):**

1. **Wave-2 for ~100k filtered ANN** — `EDGEQUAKE_VECTOR_STORAGE=halfvec` + `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=1` (greenfield only); see [Product limits](product-limits.md)
2. **Residency** — keep `shared_buffers` ≥2 GB (4 GB class for large lab); cold ~1.5 s @100k default path is expected without this
3. **Warm** a filtered query after deploy so partial HNSW exists (or `./scripts/wave2_warmup.sh` / `POST /api/v1/admin/ann/warmup`)
3b. **Filtered recall underfill** — EdgeQuake sets `hnsw.iterative_scan=relaxed_order` + `max_scan_tuples` on filtered queries only; tune `EDGEQUAKE_HNSW_MAX_SCAN_TUPLES` / `EDGEQUAKE_HNSW_SCAN_MEM_MULTIPLIER` if needed ([SPEC-075](../specs/075-filtered-recall-gates/000-index.md)). Promote with **filtered** recall@20 (`make filtered-recall-gate`), never unfiltered-only.

**Query / LLM:**

4. **Use `naive` mode** for simple queries (vector-only, no graph)
5. **Reduce `max_chunks`** from 20 to 5–10
6. **Use faster LLM** (`gpt-5-mini` vs larger models)
7. **Use GPU** for Ollama embedding

### How do I enable the supported 100k shape?

Use the **Turnkey greenfield** recipe in [Product limits](product-limits.md):

```bash
eval "$(make -s wave2-greenfield-env)"
# or: WAVE2_GREENFIELD=1 make backend-bg
./scripts/wave2_warmup.sh <workspace_uuid>
```

That sets `halfvec` + workspace partial HNSW + optional `EDGEQUAKE_HNSW_EF_SEARCH=240` (concurrent tip — not a silent default). Do **not** silent-flip existing vector DBs. Dedicated `*_ws_*` + HNSW is dimension isolation only — not the 100k concurrent path.

### How do I enable opt-in DiskANN @150k / @250k?

See [Product limits — Opt-in DiskANN recipe](product-limits.md): `pg18-vectorscale` image, dedicated table + `USING diskann`. **@150k:** `query_search_list_size ≥ 400` + `query_rescore ≈ list/2`. **@250k (SPEC-082 floor):** list ≥ **800**, rescore ≈ **400**, prefer a higher-quality DiskANN build (`num_neighbors=64`, `search_list_size=200`). Wave-2 remains the default; DiskANN is opt-in only (`make diskann-recall-pareto` / `make push-scale-ladder` / `make diskann-rescore-smoke`).

### How do I gate filtered recall@20?

Use `make filtered-recall-gate` (SPEC-075). It archives **workspace-filtered** recall@20 for Wave-2 (+ iterative_scan-only compare). Soft-fails product floors; does not raise the 100k Wave-2 floor. See [Product limits — Filtered recall + iterative_scan](product-limits.md).

### How do I improve ranking precision without raising floors?

See [Product limits — Precision tips (SPEC-076)](product-limits.md):

1. **Opt-in ANN→exact reorder** — `EDGEQUAKE_ANN_EXACT_REORDER=1` + optional `EDGEQUAKE_ANN_REORDER_CANDIDATE_K=50` (default OFF; not a silent flip).
2. **Sparse FTS+ANN RRF tip** for codes/names — `EDGEQUAKE_SPARSE_FUSION=rrf` (default remains sparse-first weighted).

Gate: `make precision-layers-gate`. Mix/RRF does **not** raise the Wave-2 / DiskANN ANN floors.

### What about binary quantization for larger corpora?

Study-only (SPEC-077): `make binary-quantize-bakeoff` compares Wave-2 halfvec HNSW to pgvector `binary_quantize` + Hamming ANN + exact rerank under a **workspace filter**. Default remains Wave-2; do **not** silent-flip (`EDGEQUAKE_BINARY_QUANTIZE` stays off). See [Product limits — Binary quantize study](product-limits.md).

### What about Filtered-DiskANN labels for shared tables?

Study-only (SPEC-078): `make filtered-diskann-labels-bakeoff` compares Wave-2 to post-filter DiskANN and to pgvectorscale **Filtered-DiskANN** (`labels smallint[]` + `labels && …`) under a **workspace filter**. Default remains Wave-2; dedicated DiskANN @150k unchanged; do **not** silent-flip (`EDGEQUAKE_FILTERED_DISKANN_LABELS` stays off; no product labels migration). See [Product limits — Filtered-DiskANN labels study](product-limits.md).

Mid-scale archive: `make midscale-quantize-labels` (SPEC-079) — tips stay **Not promoted** unless a full concurrent gate says otherwise.

### Why are tiny workspaces forced onto HNSW?

They should not be. SPEC-080 skips Wave-2 `enable_seqscan=off` bias when workspace rows ≤ `EDGEQUAKE_ANN_EXACT_MAX_ROWS` (default 2000). Gate: `make tiny-slice-exact-gate`.

### Is there a serving view for “chunks that have vectors”?

Admin/debug only (SPEC-081): `eq_serving_chunk_presence` / `eq_serving_vector_presence`. This is **not** the RAG ANN path and does **not** unify dual-SSOT stores. Gate: `make serving-view-check`.

### How do we push performance tests / floors further?

`make push-scale-ladder` (SPEC-082) archives A6 Filtered-DiskANN @150k/250k, Wave-2 filtered spot @150k, and DiskANN **primary** full-gate @250k. Floors rise **only** when the full-gate is green; Wave-2 default stays 100k unless a separate full-gate says otherwise. Silent flip remains forbidden. See [Product limits](product-limits.md).

---

## Multi-Tenancy

### Is EdgeQuake multi-tenant?

**Yes**. Each workspace is isolated:

- Separate document collections
- Separate knowledge graphs
- Per-workspace LLM configuration
- No data leakage between workspaces

Use `X-Tenant-ID` and `X-Workspace-ID` headers (or JWT claims) on protected routes when auth is enabled.

### Can different tenants use different LLMs?

**Yes**. LLM provider and model are configured per workspace via the API or environment defaults.

---

## Security

### Is my data encrypted?

| Level      | Status                       |
| ---------- | ---------------------------- |
| At rest    | Depends on PostgreSQL config |
| In transit | Yes (HTTPS recommended)      |
| API keys   | Never logged                 |

### Does EdgeQuake send data to external services?

Only to LLM providers you configure:

- **OpenAI / cloud providers**: Document chunks sent for extraction and vision
- **Ollama**: Local, no external calls
- **No telemetry** sent by EdgeQuake itself

### How do I secure the API?

**Authentication is ON by default** (SPEC-027). Protected routes require a JWT (`Authorization: Bearer …`) or API key (`X-API-Key`).

| Mode | When to use | Configuration |
| ---- | ----------- | ------------- |
| **Production** | Deployed stacks | Auth enabled (default); set `JWT_SECRET`, bootstrap admin, disable demo login |
| **Local `make dev`** | Day-to-day development | Auth on with fixed credentials `admin` / `EdgeQuake1` (shown on `/login`) |
| **Open API escape hatch** | Rare open-API proofs | `make dev-open` or `DEV_AUTH_ENABLED=false make dev` |

**Bootstrap first admin** (before first login):

```bash
export EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
export EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='ChangeMe123!'
export EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
```

Or create a user with the master API key: `EDGEQUAKE_MASTER_API_KEY`.

Full hardening checklist: [Runtime auth hardening](operations/runtime-auth-hardening.md).

**Additional layers** for production:

1. **Reverse proxy** (nginx/Caddy) with TLS
2. **Network isolation** (private subnet)
3. **External SSO** via [oauth2-proxy](https://github.com/oauth2-proxy/oauth2-proxy) (recommended over in-process OIDC for enterprise)

Explicit opt-out (not recommended outside local dev): `EDGEQUAKE_AUTH_ENABLED=false` or `EDGEQUAKE_AUTH_DISABLED=true`.

---

## Ingestion: cancel, fairness, replicas & convert → ingest

Operational details live in [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md). Summary:

### How do I cancel an in-flight upload or ingestion?

Canonical endpoint:

```http
POST /api/v1/tasks/{track_id}/cancel
```

Also supported: `DELETE /api/v2/workspaces/{id}/jobs/{job_id}`, PDF cancel, pipeline-wide cancel, and WebSocket `{ "type": "cancel", "track_id": "…" }`.

Cancel is **cooperative** — expect a short delay until the current LLM/vision round-trip aborts. UI should show **Stopping…** until status is terminal (`display_status=cancelled`).

### What is tenant fairness / why is my second upload waiting?

Workers limit concurrency **per tenant and per fairness lane**:

- **Ingest** (`MAX_TASKS_PER_TENANT`): Pdf/Insert/… — local Ollama/LM Studio clamps to **1** (workers effective **4**) unless `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1`
- **Lifecycle** (`MAX_LIFECYCLE_TASKS_PER_TENANT`, local default **4**): Deletion/Wipe — separate from ingest so deletes do not serialize new uploads

Parked tasks wait on that lane’s semaphore — they are **not** requeued in a reclaim storm. Check `GET /api/v1/pipeline/queue-metrics` → `tenant_park_waiters`, `tenant_park_waiters_ingest`, `tenant_park_waiters_lifecycle`, `max_tasks_per_tenant`, `max_lifecycle_tasks_per_tenant`.

If a new PDF stays **Queued** while deletes run, that was the old shared-lane bug; with dual lanes the PDF should take the ingest slot while lifecycle deletes continue.

### Why do bulk uploads feel excessively slow? (SPEC-122 / #361 / #365)

**Admit ≠ searchable.** HTTP 202 / “uploaded” only means the file is queued. Documents become searchable when the **Insert** pipeline completes (PDF also pays a prior **PdfProcessing** convert step).

Throughput is `min(workers, MAX_TASKS_PER_TENANT, provider budget, vision jobs, extract fan-out, embed async)` — not the number of files you selected. WebUI transfers up to **3** files in parallel; that does **not** mean 3 documents finish ingest together under local Ollama.

| Profile | Typical ingest lane | Notes |
|---------|---------------------|-------|
| `make` local Ollama | `MAX_TASKS_PER_TENANT=1`, extract/embed/vision **1** | Intentionally near-serial (VRAM / `OLLAMA_NUM_PARALLEL`) |
| Docker compose defaults | workers **8**, tenant **6**, extract **4** | Wider, still LLM-bound |
| Cloud / Mistral / OpenAI (`make` with API key) | tenant **12**, extract up to **32** | Still O(chunks)×RTT |

Measured on v0.24.3 (small text fixtures, N=5): local Ollama ≈ **5 docs/min** (tenant=1); Mistral with tenant=6 ≈ **6.7 docs/min**. PDF vision adds a convert tax (1-page sample ≈ **11.5 s** convert alone) — a quality-path cost, not proof that PDF is “broken.” Full pack: [`specs/122-implementation/`](../specs/122-implementation/). Operator knobs: [performance-tuning](operations/performance-tuning.md).

### What are claim / lease semantics on restart?

Postgres task rows are the delivery SSOT. Workers **claim** via `FOR UPDATE SKIP LOCKED`, hold a **lease** (`EDGEQUAKE_TASK_LEASE_TTL_SECS`, default 120 s), and refresh every 60 s.

| Status at boot | Default (unset / ON) | `EDGEQUAKE_STARTUP_AUTO_RESUME=0` |
| -------------- | -------------------- | --------------------------------- |
| **Pending**    | Claimable            | Claimable                         |
| **Processing** (stale) | → Pending (reclaimable) | → Failed (`failure_code=server_restart_interrupted`; use Reprocess) |
| **Cancelled**  | Never claimed        | Never claimed                     |

### Can I run multiple API/worker replicas?

Set `EDGEQUAKE_REPLICAS` to your process count. When `EDGEQUAKE_REPLICAS>1`, **`EDGEQUAKE_TASK_DELIVERY=local` fails at boot** — use `bridged` or `notify_only`. Correctness remains `claim_next` + lease; delivery modes are wake signals only.

Monitor: `GET /api/v1/pipeline/queue-metrics` (`store_contention`, `cancel_intent_count`).

### Why does PDF processing have two phases (convert → ingest)?

PDF admission enqueues **convert only** (`TaskType::PdfProcessing`). After durable markdown is stored and the PDF row is `Completed`, a separate **Insert** task runs KG ingestion under its own lease, timeout, and fairness permit.

Cancelling convert **or** an in-flight ingest cancels both linked tasks for the same `pdf_id`. After convert completes, cancelling ingest leaves the PDF `Completed` (markdown barrier kept).

---

## Features

### What document formats are supported?

| Format | Support | Notes |
| ------ | ------- | ----- |
| Plain text (`.txt`) | ✅ Full | WebUI + API |
| Markdown (`.md`) | ✅ Full | WebUI + API |
| JSON (`.json`) | ✅ Full | WebUI + API (text path) |
| PDF (`.pdf`) | ✅ Full | WebUI + `POST /api/v1/documents/pdf` (vision LLM required for convert) |
| Images (PNG/JPG/GIF/WEBP) | ✅ Full | WebUI + multipart upload (vision LLM) |
| CSV / HTML / XML / YAML | ✅ API-only | Accepted on `POST /api/v1/documents/upload`; not on WebUI dropzone |
| DOCX (Word) | ❌ Not supported | Export to PDF or Markdown. See SPEC-121 future study |
| Excel (XLSX/XLS) | ❌ Not supported | Export to CSV, PDF, or Markdown. See SPEC-121 |

Product matrix SSOT: [specs/121-pdf-docx/](../specs/121-pdf-docx/README.md) (GitHub [#370](https://github.com/raphaelmansuy/edgequake/issues/370)).

### PDF uploads work for JSON but fail in Docker — runbook

JSON uses a small `POST /api/v1/documents` body. PDF uses multipart `POST /api/v1/documents/pdf` plus pdfium + vision. When JSON succeeds and PDF does not:

1. **Use the PDF endpoint** — never `POST /documents/upload` with `.pdf` (returns 400; whitelist is text/images only).
2. **Align proxy body size** with `EDGEQUAKE_MAX_UPLOAD_BYTES` (default 50 MiB). A reverse proxy `client_max_body_size` below that yields **413** on PDF while tiny JSON still works.
3. **Writable pdfium cache** — compose sets `PDFIUM_AUTO_CACHE_DIR=/tmp/edgequake-pdfium-cache`. Confirm the directory is writable by the runtime user.
4. **Vision / Ollama reachable from the container** — e.g. `OLLAMA_HOST=http://host.docker.internal:11434`. Admit can succeed then status sticks on **Converting** / `PDF_CONVERSION_FAILED` if vision is down (that is convert failure, not “unsupported format”).
5. **Workspace header** — PDF requests require a workspace UUID (`X-Workspace-Id` / product equivalent).

See [upload quick reference](api-reference/document-upload-quick-reference.md) and SPEC-121 [system lens](../specs/121-pdf-docx/05-lenses/007-system-engineer.md).

### What LLM providers are supported?

| Provider      | Support | Notes                                      |
| ------------- | ------- | ------------------------------------------ |
| OpenAI        | ✅ Full  | GPT-5.x series (catalog)                   |
| Anthropic     | ✅ Full  | Claude models (`ANTHROPIC_API_KEY`)        |
| Mistral       | ✅ Full  | Mistral models (`MISTRAL_API_KEY`)         |
| Google Gemini | ✅ Full  | Developer API (`GEMINI_API_KEY`)           |
| Vertex AI     | ✅ Full  | Enterprise GCP identity (ADC/SA)           |
| Ollama        | ✅ Full  | Local models (default: `gemma4:latest`)    |
| LM Studio     | ✅ Full  | OpenAI-compatible                          |
| Azure OpenAI  | ✅ Full  | Via `AZURE_OPENAI_*` env vars              |
| OpenRouter    | ✅ Full  | Model aggregator                           |

### What query modes are available?

| Mode     | Use Case                         |
| -------- | -------------------------------- |
| `naive`  | Simple vector search             |
| `local`  | Entity-focused queries           |
| `global` | Relationship-centric search      |
| `hybrid` | Local + Global + Naive (round-robin) |
| `mix`    | Weighted blend of all arms (**DEFAULT**) |
| `bypass` | Direct LLM, no retrieval (debug) |

---

## Troubleshooting

### Why are my queries returning empty?

1. **Check documents exist**:

   ```bash
   curl http://localhost:8080/api/v1/documents
   ```

2. **Check entities extracted** (graph namespace):

   ```bash
   curl http://localhost:8080/api/v1/graph/entities
   ```

3. **Try `naive` mode** (vector-only):

   ```json
   { "query": "test", "mode": "naive" }
   ```

When auth is enabled, add `-H "Authorization: Bearer $TOKEN"` or `-H "X-API-Key: $KEY"`.

See [Troubleshooting Guide](/docs/troubleshooting/common-issues/) for more.

### Why is document processing stuck?

1. Check LLM is running (Ollama: `ollama list`)
2. Check API key is valid (OpenAI)
3. Poll task status: `GET /api/v1/tasks/{track_id}`
4. Check queue metrics: `GET /api/v1/pipeline/queue-metrics`
5. Check logs: `tail -f /tmp/edgequake-backend.log`

### How do I check if EdgeQuake is healthy?

```bash
# Basic health (no auth)
curl http://localhost:8080/health

# Full readiness (checks database + store contention)
curl http://localhost:8080/ready
```

---

## Comparison

### EdgeQuake vs LightRAG (Python)?

| Aspect       | LightRAG | EdgeQuake     |
| ------------ | -------- | ------------- |
| Language     | Python   | Rust          |
| Speed        | Baseline | 10–50× faster |
| Memory       | Higher   | Lower (no GC) |
| Multi-tenant | No       | Yes           |
| Production   | Research | Production    |
| Algorithm    | Same     | Same          |

### EdgeQuake vs Microsoft GraphRAG?

| Aspect     | GraphRAG                 | EdgeQuake         |
| ---------- | ------------------------ | ----------------- |
| Approach   | Hierarchical communities | Flat entity graph |
| Cost       | Very high ($$$)          | Low–medium        |
| Index time | Hours–days               | Minutes           |
| Queries    | Global summaries         | Hybrid modes      |
| Use case   | Large corpora            | General purpose   |

### EdgeQuake vs Pinecone/Weaviate?

| Aspect     | Vector DBs        | EdgeQuake      |
| ---------- | ----------------- | -------------- |
| Type       | Storage only      | Full RAG stack |
| Retrieval  | Vector similarity | Vector + Graph |
| Extraction | Not included      | Built-in       |
| Multi-hop  | No                | Yes            |

---

## Contributing

### How can I contribute?

1. Fork the repository
2. Create a feature branch
3. Make changes following [AGENTS.md](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/AGENTS.md)
4. Run `cargo clippy && cargo test`
5. Submit a pull request

### What's the development workflow?

```bash
git clone https://github.com/your-fork/edgequake
make dev
cargo test
cargo clippy
cargo fmt
```

---

## Vision & PDF Processing

### Why is my PDF failing with "Vision extraction timed out" or "Circuit breaker tripped"?

This almost always means the **vision model does not match the vision provider**.
For example, `EDGEQUAKE_VISION_MODEL=gpt-4.1-nano` paired with `EDGEQUAKE_LLM_PROVIDER=ollama`
will fail because Ollama cannot serve OpenAI models.

**Diagnose** — check the effective config endpoint:

```bash
curl -s http://localhost:8080/api/v1/config/effective | jq '.areas[] | select(.name == "Vision")'
```

If `has_mismatch` is `true`, the response explains exactly which env var is wrong
and how to fix it.

**Fix** — pick one:

| Option | Action                                                                                     |
| ------ | ------------------------------------------------------------------------------------------ |
| A      | **Unset** the mismatched env var so the default takes over: `unset EDGEQUAKE_VISION_MODEL` |
| B      | **Change the provider** to match the model: `EDGEQUAKE_VISION_PROVIDER=openai`             |
| C      | **Change the model** to match the provider: `EDGEQUAKE_VISION_MODEL=gemma4:latest`         |

Then restart the backend.

### How does EdgeQuake decide which vision provider and model to use?

The resolution chain (highest priority first):

1. **Per-request form field** (`vision_provider` / `vision_model` in upload)
2. **`EDGEQUAKE_VISION_PROVIDER`** / **`EDGEQUAKE_VISION_MODEL`** env vars
3. **`EDGEQUAKE_VISION_LLM_PROVIDER`** / **`EDGEQUAKE_VISION_LLM_MODEL`** env vars
4. **`EDGEQUAKE_DEFAULT_LLM_PROVIDER`** / **`EDGEQUAKE_DEFAULT_LLM_MODEL`** env vars
5. **`EDGEQUAKE_LLM_PROVIDER`** / **`EDGEQUAKE_LLM_MODEL`** env vars
6. **Vision fallback** (when no env vars match): `ollama` / `gemma4:latest`

At each step, incompatible combinations are skipped with a warning log.

### Can I use a different model for vision than for text extraction?

**Yes**. Set the vision-specific env vars:

```bash
EDGEQUAKE_DEFAULT_LLM_PROVIDER=ollama
EDGEQUAKE_DEFAULT_LLM_MODEL=gemma4:latest

EDGEQUAKE_VISION_PROVIDER=openai
EDGEQUAKE_VISION_MODEL=gpt-4.1-nano
OPENAI_API_KEY=sk-...
```

### Where can I see the active configuration in the UI?

Open **Settings → Configuration Explainability**. The same data is available via `GET /api/v1/config/effective`.

---

## See Also

- [Getting Started](/docs/getting-started/installation/)
- [Runtime auth hardening](operations/runtime-auth-hardening.md)
- [Ingestion cancel & fairness](ingestion-cancel-and-fairness.md)
- [Architecture Overview](/docs/architecture/overview/)
- [API Reference](/docs/api-reference/rest-api/)
- [Troubleshooting](/docs/troubleshooting/common-issues/)
