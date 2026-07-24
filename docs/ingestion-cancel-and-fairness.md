---
title: "Ingestion cancel, fairness, and restart semantics"
---

# Ingestion cancel, fairness, and restart semantics

> **Product: v0.19.0** · Contract: [OpenAPI snapshot](../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: this document (SPEC-057 SSOT)

Operational notes for the task worker pool (P0–P3 remediation).

## Cancel a task (canonical)

```http
POST /api/v1/tasks/{track_id}/cancel
```

Effects (all entry points share the same SSOT):

1. Task row → `Cancelled` via `apply_task_row_cancel` (terminal; no auto-retry)
2. `CancellationRegistry` records a **cancel intent** and signals any in-flight `CancellationToken`
3. Linked document KV → `cancelled` + `failure_class=cancelled` via `sync_doc_cancelled_for_task` (or track_id scan on HTTP)
4. Pending / fairness-parked copies of the same `track_id` are dropped on dequeue / never claimed
5. PDF row (when applicable) → `PdfProcessingStatus::Cancelled` (SPEC-057 — **not** `Failed`)

Also supported (task cancel + doc KV sync; PDF path also stamps PDF Cancelled):

| Path | Behavior |
|------|----------|
| `DELETE /api/v2/workspaces/{id}/jobs/{job_id}` | Same as task cancel |
| `DELETE /api/v1/documents/pdf/{pdf_id}/cancel` | Task cancel + PDF → `cancelled` + doc KV sync |
| `POST /api/v1/pipeline/cancel` | Cancels all registered in-flight tasks + doc KV sync |
| WebSocket `{ "type": "cancel", "track_id": "..." }` | Task cancel + doc KV sync |

UI should call `POST /tasks/{track_id}/cancel` and show “Stopping…” until status is terminal.

```
┌─────────────────────────────────────────────────────────────┐
│ Cancel SSOT (all entry points)                              │
│                                                             │
│  POST /api/v1/tasks/{track_id}/cancel                       │
│              |                                              │
│    +---------+---------+---------+                          │
│    v         v         v         v                          │
│  task row  cancel   doc KV    PDF row                       │
│  Cancelled intent  cancelled  Cancelled                     │
│            |                                                │
│            v                                                │
│  ui_phase: stopping --> terminal                            │
│  display_status: cancelled                                  │
└─────────────────────────────────────────────────────────────┘
```

### Status SSOT (SPEC-057 P4)

Document list/detail JSON includes presentation fields from `IngestionStatusMapper`:

| Field | Meaning |
| ----- | ------- |
| `display_status` | Badge key (`cancelled`, `failed`, `completed`, `extracting`, `converting`, …). Prefer over re-deriving from `status`/`current_stage`. |
| `ui_phase` | `idle` \| `running` \| `stopping` \| `terminal`. When `stopping`, UI shows **Stopping…** even if `display_status` is still a stage (e.g. `extracting`). |

Cancel intent (registry) while the doc is not yet terminal ⇒ `ui_phase=stopping`. Terminal cancel truth (`task`/`doc`/`pdf`/`failure_class=cancelled`) ⇒ `display_status=cancelled`, `ui_phase=terminal`. PDF `Completed` does **not** override an in-flight doc stage (convert artifact only).

Cancel is **cooperative**: vision convert, LLM extract, and embed calls abort via `select!` / token checks at `.await` points. Expect a short delay until the current HTTP round-trip is dropped.

## Tenant fairness (no requeue storm)

Fairness uses **two per-tenant lanes** (operation class):

| Lane | Task types | Env | Local Ollama/LM Studio default |
| ---- | ---------- | --- | ------------------------------ |
| **Workers** | pool size | `WORKER_THREADS` | capped at **4** |
| **Ingest** | PdfProcessing, Insert, Upload, Scan, Reindex, KnowledgeInjection | `MAX_TASKS_PER_TENANT` | **2** (protects LLM/vision) |
| **Lifecycle** | Deletion, WorkspaceWipe | `MAX_LIFECYCLE_TASKS_PER_TENANT` | **4** (DB/graph; not shared with ingest) |

When a lane max is `> 0`:

- Workers `try_acquire(tenant, fairness_class)` on that lane’s semaphore
- If at capacity, the task **parks** on `acquire()` in a background waiter (no channel bounce)
- Process-local park dedupe: a `track_id` already parked is released on reclaim **without** spawning another waiter; the worker **immediately re-claims** (bounded) so newer ingest work is not stuck behind a 2s poll
- Worker continues serving other tenants’ ready work (and the other lane for the same tenant)

Local providers (`ollama` / `lmstudio`) clamp **ingest** to **2** and workers to **4** unless `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1`. Do **not** raise that flag just to unblock deletes — use the lifecycle lane. SPEC-057 P2: the ingest clamp uses the **runtime extract provider** (`EDGEQUAKE_EXTRACT_PROVIDER` → `EDGEQUAKE_DEFAULT_EXTRACT_PROVIDER` → `EDGEQUAKE_DEFAULT_LLM_PROVIDER` → `EDGEQUAKE_LLM_PROVIDER`), so hybrid OpenAI LLM + Ollama extract still applies the local ingest clamp.

Queue metrics: `max_tasks_per_tenant` (ingest), `max_lifecycle_tasks_per_tenant`, `tenant_park_waiters`, `tenant_park_waiters_ingest`, `tenant_park_waiters_lifecycle`.

## Convert then ingest (SPEC-057 P2)

PDF admission enqueues `TaskType::PdfProcessing` (**convert only**). After durable `pdf_documents.markdown_content` + PDF `Completed`, the worker enqueues `TaskType::Insert` for KG ingest under a separate lease/timeout/fairness permit.

```
┌───────────────────────────────────────────────────────┐
│ Convert then ingest (SPEC-057)                        │
│                                                       │
│  POST /documents/pdf  -->  admit task_id              │
│              |                                        │
│              v                                        │
│  [1] PdfProcessing (convert only)                     │
│      vision / edgeparse --> markdown                  │
│      PDF row --> Completed (artifact)                 │
│              |                                        │
│              v  markdown barrier                      │
│  [2] Insert (KG ingest, new lease)                    │
│      chunk --> extract --> embed --> store            │
│              |                                        │
│              v                                        │
│  document display_status = completed                  │
└───────────────────────────────────────────────────────┘
```

| Phase | Task type | Timeout metadata key | PDF row on success |
| ----- | --------- | -------------------- | ------------------ |
| Convert | `pdf_processing` | `metadata.processing_timeout_secs` ← `LargeDocumentProfile::convert_timeout_secs` | `Completed` + markdown |
| Ingest | `insert` | `metadata.processing_timeout_secs` ← `LargeDocumentProfile::ingest_timeout_secs` | unchanged (convert survives) |

HTTP and WebSocket cancel share `cancel_track_with_doc_and_pdf_chain` (task row + doc KV + Convert∪Insert).

Cancel of convert **or** PDF cancel with an in-flight Insert cancels **both** linked Pending/Processing tasks for the same `pdf_id`. After convert has already Completed, cancelling ingest leaves the PDF `Completed` (markdown barrier kept). Doc KV stage continues through Insert until ingest finishes — PDF `Completed` means convert artifact only.

## Observability

`GET /api/v1/pipeline/queue-metrics` includes:

- `tenant_park_waiters` — tasks waiting for a tenant permit
- `cancel_intent_count` / `cancel_intent_total`
- `max_tasks_per_tenant`
- `store_contention` — nested SLO object (pool utilization + compensation quarantine)

### Store contention + compensate DLQ (SPEC-057 P3)

| Signal | Source | Critical action |
| ------ | ------ | --------------- |
| `store_contention.db_pool_utilization` | sqlx pool size/idle | Scale pool / reduce ingest |
| `store_contention.compensation_quarantine_total` | process counter + Prometheus `edgequake_compensation_quarantine_total` | Inspect KV DLQ keys `compensation_quarantine:{document_id}:*` |
| Queue `pressure=critical` | pending depth | Scale `WORKER_THREADS` |

`/ready` returns 503 when store contention is **critical** (same thresholds as queue-metrics).

Env thresholds (defaults): `EDGEQUAKE_DB_POOL_UTIL_WARN=0.75`, `EDGEQUAKE_DB_POOL_UTIL_CRITICAL=0.90`, `EDGEQUAKE_COMPENSATION_QUARANTINE_WARN=1`, `EDGEQUAKE_COMPENSATION_QUARANTINE_CRITICAL=5`.

**Park waiters vs merge/compensate:** high `tenant_park_waiters` means fairness is holding work (expected under local LLM clamp). Rising `compensation_quarantine_total` means merge cleanup failed — not a park issue; check AGE/pgvector delete errors and DLQ KV records.

## Multi-replica delivery (SPEC-057 P3)

| Env | Role |
| --- | ---- |
| `EDGEQUAKE_REPLICAS` | Intended API/worker process count (default `1`) |
| `EDGEQUAKE_TASK_DELIVERY` | `local` (default) \| `bridged` \| `notify_only` |

When `EDGEQUAKE_REPLICAS>1` and delivery is `local`, **boot fails** — set `bridged` or `notify_only`. Correctness remains `claim_next` + lease; Bridged/NotifyOnly are **wake** modes only. Never process from a channel payload without claim.

## Restart semantics (SPEC-057 P1 claim / lease)

Postgres task rows are the **delivery SSOT**. The in-memory channel is a **wake signal only**.

```
┌───────────────────────────────────────────────────────────┐
│ Task delivery SSOT (Postgres)                             │
│                                                           │
│  admit --> Pending row (wake channel optional)            │
│              |                                            │
│              v                                            │
│  worker: FOR UPDATE SKIP LOCKED claim                     │
│              |                                            │
│              v                                            │
│  lease + heartbeat (TTL default 120s)                     │
│              |                                            │
│       +------+------+                                     │
│       v             v                                     │
│   run handler    fairness park                            │
│                  (release claim)                          │
└───────────────────────────────────────────────────────────┘
```

| Status at boot | Default (unset / ON) | `EDGEQUAKE_STARTUP_AUTO_RESUME=0` |
| -------------- | -------------------- | -------------------------------- |
| **Pending** | Leave Pending (claimable via `claim_next` / poll) | Leave Pending (unchanged) |
| **Processing** (stale / this process) | → Pending (reclaimable) | → Failed (“Interrupted — use Reprocess”) |
| **Cancelled** | Never claimed | Never claimed |

Workers: wake or ~2s poll → `FOR UPDATE SKIP LOCKED` claim → lease (`EDGEQUAKE_TASK_LEASE_TTL_SECS`, default 120) → `refresh_lease` heartbeat every 60s. Fairness park **releases** the claim before waiting.

Cancel intents are process-local; **Cancelled** DB status is the source of truth after restart. Interrupted Processing stays Reprocess-eligible.
