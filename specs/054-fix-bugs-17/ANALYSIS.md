# SPEC-054 — Bug Triage Report (Issues #37, #186, #239, #292, #294, #296, #297, #298, #300)

Date: 2026-07-16
Branch: feat/spec047-vision-ingest-spec048-progress  
Analyst: GitHub Copilot (First Principles analysis)

---

## Summary

**Code-is-law grades:** see [CODE_IS_LAW_ASSESSMENT.md](./CODE_IS_LAW_ASSESSMENT.md).
Do not treat rows below as “shipped” unless that assessment says so.

**Query · Postgres · AGE · pgvector (performance SSOT):**  
[`README.md`](./README.md) (001–006) — First Principles, cross-ref, budgets,
test matrix (orthogonal to #300/#298 ingest UX).

| # | Title | Status (lawful) | Action |
| --- | --- | --- | --- |
| #298 | Pending forever while pipeline idle; Reprocess no-op | **IN PROGRESS (B+)** — 8/8 e2e + stampede guard; uncommitted | Restart `make dev`; commit + ship |
| #300 | v0.17.0 Vision upload stuck on loading | **IN PROGRESS (B−)** — 4/4 postgres e2e; uncommitted; OPEN | Commit + patch image + close |
| #297 | Cross-workspace contamination + orphan vector table | **IN TREE (B−)** — `drop_workspace_table` since `69bba616`; CLOSED | Prove DROP on failure path; keep CLOSED |
| #296 | proxyClientMaxBodySize Type Error (Next.js 16) | **IN TREE (B)** | Close if release build green |
| #294 | New API keys return 401 in ECS | **PARTIAL (C+)** — in-memory warn landed; not multi-instance proof; CLOSED | Prefer PG auth; do not call warn a fix |
| #292 | Docker image 0.15.1 not found | **OPS (A−)** | Use published ≥0.16 tags |
| #239 | Partial failure log details in WebUI | **UNVERIFIED (D as FIXED)** | Needs reporter-shaped proof before “close” rhetoric |
| #186 | Add Ollama cloud model support | **IN TREE (C)** | Cloud e2e still absent |
| #37 | Get only retrieval chunks (no LLM answer) | **IN TREE (B)** | Document `context_only=true` |

---

## Issue #300 — v0.17.0 Vision upload stuck on loading

### Status: IN PROGRESS — progress identity contract (uncommitted WIP)

Local code targets the invariant below. **Not law yet:** uncommitted, not in a
published image, GitHub #300 still OPEN. Remaining gate: commit + published-image
Docker/Mistral WebUI acceptance. See `CODE_IS_LAW_ASSESSMENT.md`.

### First-principles identity model

The system has four different identities. The defect comes from treating two
of them as interchangeable:

| Identity | Cardinality | Owner | Correct use |
| --- | --- | --- | --- |
| Client `track_id` | One per upload batch today | WebUI | Batch/request correlation only |
| Server `task_id` (`pdf-<uuid>`) | One per PDF processing job | Task queue | Canonical progress, cancellation, retry, WS/SSE identity |
| `pdf_id` | One per stored PDF | PDF storage | PDF resource identity |
| `document_id` | One per indexed document | Ingestion storage | Document lifecycle and query identity |

The client-generated `track_id` cannot be the canonical worker identity:
`use-file-upload.ts` creates one shared value before iterating all files in a
batch. Reusing that value as a per-job progress key would make multiple PDFs
overwrite or merge each other's progress.

**Required invariant:**

```text
For every admitted PDF job J:
  response.task_id
    = queued_task.track_id
    = document_metadata.track_id
    = PipelineProgressCallback progress key
    = WebUI subscription key
```

The optional client `track_id` remains batch correlation metadata and MUST NOT
be used as the PDF progress-store key.

### 5 WHY Analysis

```text
WHY 1: Why does the upload UI remain on loading?
  → The WebUI subscribes to response.track_id and waits for terminal progress.

WHY 2: Why does progress under that ID remain "Waiting for Upload"?
  → upload.rs seeds a progress record under the client batch track_id, but the
    worker emits all subsequent updates under its server task_id.

WHY 3: Why are different IDs used on each side?
  → PdfUploadResponse exposes task_id and track_id, but performFileUpload()
    normalizes track_id from the client field and ignores task_id for tracking.

WHY 4: Why not make the client ID canonical?
  → It is generated once per multi-file batch, not once per worker task.
    A canonical progress ID must be unique for each admitted PDF job.

WHY 5: Why is there no visible error?
  → Both IDs are syntactically valid and the client ID gets an empty progress
    skeleton. The request returns 200 and the real task completes, so no error
    path fires; the UI waits on a non-advancing record forever.
```

### Reproduction evidence

```text
POST /api/v1/documents/pdf
  response.track_id = ui_track_1784173469
  response.task_id  = pdf-71ab2f81-b63c-4357-8e94-d785c63b96c8

GET /documents/pdf/progress/ui_track_1784173469
  overall_percentage = 0.0; every phase pending

GET /documents/pdf/progress/pdf-71ab2f81-b63c-4357-8e94-d785c63b96c8
  PDF conversion and later phases advance

GET /documents
  document track_id = pdf-71ab2f81-...; status advances to completed
```

Full artifacts and Docker/Mistral steps:
`specs/056-issue-release-17/`.

### Root cause boundaries

- **In scope:** PDF upload response semantics, progress initialization,
  frontend subscription key, WebSocket/SSE/polling correlation, force-reindex.
- **Not root cause:** Mistral vision capability, PDF conversion, entity
  extraction, `/documents/upload` rejecting PDFs, global model health display,
  or migration applied-count reporting.

---

## Issue #298 — Pipeline idle while documents stay Pending (Reprocess no effect)

**Issue:** https://github.com/raphaelmansuy/edgequake/issues/298  
**Reporter environment:** AWS ECS Fargate, **v0.12.11** (stuck on older release
due to auth/#288/#296), large workspace (~811 docs, hundreds Pending).

### Status: IN PROGRESS — pending↔task reconcile (uncommitted WIP)

Local code targets #298-A/B and part of #298-C. **Not law yet:** uncommitted;
GitHub #298 OPEN with 0 comments; `bt045_sre_pdf_recover_stuck_wired` currently
fails after DRY extract; ECS-scale acceptance unmet. See
`CODE_IS_LAW_ASSESSMENT.md`.

Observed symptoms are consistent with first principles and current code,
not with a single UI flake:

| Signal | Reporter | Meaning in current architecture |
| --- | --- | --- |
| Pipeline Status = Idle | Yes | No *working* docs and no `TaskStatus::Processing` tasks |
| Documents Pending (hundreds) | Yes | KV metadata non-terminal; waiting for a worker **task** |
| Banner "Processing 807" | Yes (v0.12.11 UI) | Old banner lumped waiting+active; SPEC-048 now separates |
| Task counters: 1429 completed / 70 failed | Yes | **Global** task stats (`GET /pipeline/status`), not workspace docs |
| Reprocess appears to do nothing | Yes | Either no task enqueued, or force path missing on old UI |

### First-principles identity model (ingestion)

Three layers must stay coupled. The defect is a break between (1) and (2):

| Layer | Store | "Idle" meaning | "Pending doc" meaning |
| --- | --- | --- | --- |
| Document metadata | KV `*-metadata` | Terminal status | Waiting / recovered / admitted shell |
| Task registry | `TaskStorage` (PG today; **memory in v0.12.11**) | No `processing` tasks | Must have a `pending`/`processing` row to resume |
| Worker delivery | `ChannelTaskQueue` (in-process, capacity **100**) | Channel empty + workers idle | Tasks must be *in the channel* or hydratable |

**Invariant (required):**

```text
For every non-terminal document D in a running system:
  ∃ task T such that T references D
  AND T.status ∈ {pending, processing}
  AND T is reachable by a worker (channel / hydrator)

If D.status ∈ {pending, queued} AND no such T:
  → Pipeline correctly reports idle
  → D waits forever unless recover/reprocess creates T
```

SPEC-048 encodes idle-with-queued as intentional:

```rust
// progress_facade.rs — busy iff working docs OR processing tasks
let busy = !working.is_empty() || !tasks.is_empty();
// unit test: busy_false_when_only_queued
```

So "Pipeline is idle" + "docs Pending" is **not a contradiction** when the
task queue is empty. It is the observable form of document↔task desync.

### 5 WHY Analysis

```text
WHY 1: Why do documents stay Pending forever?
  → Document KV was set/left at pending|queued, but no worker picks them up.

WHY 2: Why does no worker pick them up?
  → Either (a) no Task row exists for those docs, or (b) Task rows exist as
    Pending in Postgres but were never (re)delivered into ChannelTaskQueue,
    or (c) workers died and tasks were never rehydrated.

WHY 3: Why are documents Pending without live tasks? (primary path for v0.12.11)
  → Startup `recover_orphaned_documents` (main.rs) rewrites stuck in-flight
    docs to status=pending with "Auto-recovered after server restart…" but
    does NOT create tasks. It only mutates metadata.
  → Companion `requeue_pending_tasks` only reloads tasks that already exist
    in TaskStorage as Pending.
  → On v0.12.11, AppState used MemoryTaskStorage → tasks evaporated on every
    ECS task recycle. After restart: docs recovered to pending, task table
    empty → permanent idle. Current tree fixed storage to PostgresTaskStorage
    (state/postgres.rs comment: "Previous bug: MemoryTaskStorage…").

WHY 4: Why does Reprocess appear to have no effect?
  → POST /documents/reprocess default filter is status ∈ {failed, cancelled}.
    Pending docs are skipped unless `document_id` + `force=true`.
  → Current WebUI always sends force=true (`reprocessDocument(id, true, mode)`).
    v0.12.11 UI / bulk path may have called without force → silent no-op
    (HTTP 200, requeued=0).
  → Even with force, enqueue can skip: soft reprocess single-flight skip,
    missing content KV (`no_content`), or PDF without usable `pdf_id`
    (build_reprocess_task returns None; bulk marks pending then skips).
  → `recover_stuck` can recreate tasks for aged active/pending docs, but the
    UI "Reprocess" button does not call that endpoint.

WHY 5: Why isn't this obvious in Pipeline Status / counters?
  → GET /pipeline/status mixes in-memory PipelineState with **global**
    TaskStatistics (all tenants) — reporter's 1429 completed vs 811 workspace
    docs is expected under that design.
  → GET /pipeline/activity busy=false when only queued metadata exists
    (proven by `busy_false_when_only_queued` test).
  → Old "Processing N" banner counted waiting docs as processing; SPEC-048
    `resolvePipelineUiState` + tests now distinguish stuck vs queued vs working.
```

### Code evidence (current tree)

**1. Startup recovers metadata, not work units**

```text
main.rs::recover_orphaned_documents
  non-terminal → status/current_stage = "pending"
  stage_message = "Auto-recovered after server restart…"
  // no enqueue_task

main.rs::requeue_pending_tasks
  list TaskStatus::Pending from TaskStorage → ChannelTaskQueue::send
  // if TaskStorage empty (v0.12.11 memory), requeues 0
```

**2. Task persistence fix (post-v0.12.11)**

```rust
// edgequake-api/src/state/postgres.rs:~322
// Previous bug: MemoryTaskStorage was used, causing tasks to be lost on restart.
let task_storage = Arc::new(PostgresTaskStorage::new(pool.clone()));
let task_queue = Arc::new(ChannelTaskQueue::new(100));
```

**3. Reprocess filter excludes pending unless forced**

```rust
// recovery/reprocess.rs::run_reprocess_failed
// default: status == failed || cancelled
// document_id + force=true: any status
```

**4. Residual risk: requeue after workers, non-blocking hydrate**

```text
main.rs startup order (SPEC-054/#298-B):
  recover_orphaned_documents
  → reconcile_pending_documents_missing_tasks
  → worker_pool.start()
  → tokio::spawn { requeue_pending_tasks(hydrate_*) }  // background, does not block HTTP bind

Workers must exist before channel fill (deadlock fix). Hydrate runs in background
so large Pending backlogs do not delay server.run().
```

**5. UI already knows this failure mode (current WebUI)**

```ts
// pipeline-document-state.ts + __tests__/pipeline-document-state.test.ts
// pending + idle queue + "Auto-recovered" → alertMode = 'stuck'
// pending + idle + aged + no track_id → stuck
// pending + idle + fresh upload → queued (grace), not stuck
```

### Test grounding

| Test / contract | What it proves for #298 |
| --- | --- |
| `progress_facade::tests::busy_false_when_only_queued` | Idle pipeline with queued docs is intentional |
| `pipeline-document-state.test.ts` stuck/queued cases | Banner must show "Needs attention", not fake "Processing N" |
| `spec045_periodic_orphan_doc_sync.rs` | Orphan *processing* tasks sync docs to failed |
| `e2e_document_deletion::test_recover_stuck_*` | recover_stuck requeues and cleans graph |
| `e2e_reindexing.rs` | force reprocess path for failed/PDF |
| `state/postgres.rs` comment + PostgresTaskStorage | v0.12.11 memory-loss root cause removed |

**Tests added (local WIP):** `e2e_spec054_pending_task_reconcile.rs` proves
reconcile / recover-stuck / reprocess create tasks for orphan pending docs in
in-memory `AppState`. **Still missing vs acceptance:** full `main.rs` startup
with N>100 Pending tasks, worker completion, and ECS recycle proof.

### Root cause boundaries

- **In scope for #298:** document↔task coupling after restart; reprocess/recover
  for pending orphans; busy/queued UI honesty; startup requeue vs channel capacity.
- **Not root cause:** embedding dimension mismatch alone, AGE/pgvector presence,
  or the #300 progress-identity bug (different symptom: spinner on one upload).
- **Reporter constraint:** remaining on v0.12.11 due to auth/build issues —
  upgrading past MemoryTaskStorage is part of the remediation.

### Fix plan

#### #298-A — Close the metadata→task gap (P0)

After `recover_orphaned_documents` (or inside a dedicated reconciler):

1. For each auto-recovered `pending` doc, ensure a Task exists (PDF →
   `PdfProcessing`, text → `Insert`), same routing as `recover_stuck` /
   `build_reprocess_task`.
2. Or: call the same enqueue path as `run_recover_stuck` for recovered IDs.
3. Idempotent: skip if `find_active_*` already has a pending/processing task.

#### #298-B — Startup requeue must not deadlock (P1)

1. Start workers (or a drain pump) **before** filling a bounded channel, **or**
2. Cap initial hydrate to `capacity` and keep a background loop that pulls
   remaining `TaskStatus::Pending` from Postgres as slots free (SSOT = PG).
3. Prefer StorageHydrating delivery for large backlogs (already in
   `edgequake-tasks` delivery module).

#### #298-C — Reprocess UX / API honesty (P1)

1. Treat `pending`/`queued` without active task as reprocessable without relying
   on callers remembering `force` (or document force as required).
2. Return explicit `requeued` / `skipped` / `skip_reasons` to the WebUI; surface
   toast when requeued=0.
3. Wire stuck banner CTA to `recover-stuck` and/or force reprocess (current
   `ingestion-alert-banner` already has a reprocess action — verify it uses force).

#### #298-D — Operator path for v0.12.11 reporters (immediate)

1. Upgrade off v0.12.11 once #288/#296/#294 auth/build blockers are cleared.
2. Meanwhile: `POST /api/v1/documents/recover-stuck` with workspace headers, then
   force reprocess selected IDs; check `GET /api/v1/tasks?status=pending`.
3. Do not trust global `GET /pipeline/status` completed counts for one workspace.

### Acceptance criteria for #298

- After simulated restart with N>0 in-flight docs and empty in-memory queue,
  every recovered pending doc has a Task and workers become busy or queued.
- `GET /pipeline/activity` shows those docs under `queued` (or `working` once
  claimed); WebUI shows Stuck or Queued — never permanent silent idle with
  aged pending and zero tasks.
- Force reprocess of a pending orphan returns `requeued >= 1` and advances status.
- Startup with >100 Pending Postgres tasks completes without hang and drains.
- Regression tests cover desync detection + recover enqueue.

---

## Issue #297 — Cross-workspace contamination + orphan vectors

### 5 WHY Analysis

```
WHY 1: Why did orphan vector table remain after DeleteWorkspace?
  → clear_workspace() executes DELETE FROM {table} (deletes rows)
    but does NOT DROP TABLE {table}

WHY 2: Why doesn't the DELETE cascade drop the table?
  → Vector storage uses per-workspace dynamic tables (eq_eq_ws_XXXXXXXX_vectors)
    The ORM abstraction treats table creation as DDL but delete as DML
    clear_workspace() was designed for "empty" not "destroy"

WHY 3: Why did cross-workspace title contamination occur?
  → Reporter is on v0.12.11. Current code uses workspace_id column filter
    on all queries. v0.12.11 may have lacked strict workspace scoping.
    Fixed in >= v0.14 with workspace_id FK enforcement.

WHY 4: Why is in-flight ingest lost on restart?
  → Pipeline state held in tokio channels + in-memory task queue
    No persistent task registry with "resume on restart" semantics

WHY 5: Why wasn't the orphan table cleaned up by workspace delete cascade?
  → The delete_workspace handler calls vector_registry.evict() (removes from
    cache) and clear_workspace() (empties rows) but has no DROP TABLE step
```

### Code Evidence

```
// workspace_crud.rs:~370
let vectors_cleared = match state.storage.vector_storage
    .clear_workspace(&workspace_id).await {
    Ok(count) => count,  // ← deletes ROWS, not table
    ...
```

```
// vector/storage_impl.rs:~409
async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
    let sql = format!("DELETE FROM {} WHERE workspace_id = $1 ...", self.table_name);
    // ← No DROP TABLE here
```

### Fix status (#297-A)

**Already in tree** (`workspace_crud.rs` step 4b →
`vector_registry.drop_workspace_table`, commit `69bba616`). ANALYSIS previously
understated this. Residual: DROP errors are only `warn!`’d as “benign.”

### Status of Cross-Workspace Contamination (#297-B)

NOT reproducible in current v0.16.x. All vector queries include
`WHERE workspace_id = $1` enforced via `WorkspaceVectorStorage` per-workspace
adapter with isolated table names. Closed for v0.12.11 reporters.

---

## Issue #296 — proxyClientMaxBodySize Type Error

### Status: ALREADY FIXED in current branch

**Evidence** (next.config.ts):
```ts
// Comment already in file:
// "Use numeric bytes (SizeLimit). Template strings like `${n}mb` widen to
//  `string` and fail `next build` typecheck (release-docker CD flake on Next 16.2)."
proxyClientMaxBodySize: DEFAULT_MAX_UPLOAD_BYTES,  // ← 50 * 1024 * 1024 (number)
```

`DEFAULT_MAX_UPLOAD_BYTES = 50 * 1024 * 1024` is type `number`, which satisfies
`SizeLimit = number | \`${number}${FileSizeSuffix}\``. The bug was template strings.

**Fix was applied before this issue was filed.** Will be in v0.17.0.

---

## Issue #294 — New API Keys Return 401 in ECS

### 5 WHY Analysis

```
WHY 1: Why do new API keys return 401?
  → validate_presented_token() first checks JWT, which fails (expected).
    Then calls validate_stored_api_key() which looks up by prefix.
    find_active_api_keys_by_prefix routes to either pg or kv store.

WHY 2: Why do OLD keys work but NEW ones fail?
  → The auth backend is likely in-memory KV store (not PostgreSQL).
    In-memory store is instance-local. In ECS, different instances handle
    CREATE (one instance) and VALIDATE (another instance) — key never found.

WHY 3: Why doesn't the key reach the validating instance?
  → session_storage::find_api_keys_by_prefix_kv uses auth_memory_store
    (in-process HashMap). No distributed cache or DB sync.

WHY 4: Why did old keys work?
  → Possibly created when there was one instance, or instance affinity by luck.
    Or: old keys were static API keys in config (EDGEQUAKE_API_KEYS env var)
    which ARE shared via env vars across all instances.

WHY 5: Why isn't this caught in tests?
  → Unit tests are single-instance. Multi-instance concurrency is not tested.
```

### Code Evidence

```rust
// auth_validation.rs:~60
// Step 1: check static keys (from env var — shared across instances ✓)
if state.auth.config.api_keys.iter().any(|k| ...) { return Ok(Some(...)) }
// Step 2: try JWT
if let Ok(claims) = state.auth.jwt.verify_token(token) { return Ok(Some(...)) }
// Step 3: stored API keys (MAY be in-memory if no PostgreSQL auth configured ✗)
validate_stored_api_key(state, token).await
```

### Fix status (#294-A)

`session_storage::persist_api_key` already emits
`tracing::warn!(... SPEC-054/gh#294 ...)` on in-memory fallback. That is
**observability**, not multi-instance correctness. Lawful remaining work:

1. When `DATABASE_URL` is set, API keys MUST hit PostgreSQL (verify no silent KV path).
2. Document ECS requirement: PostgreSQL auth backend required for >1 task.
3. Do not treat GitHub CLOSED as proof of ECS create/validate across instances.

---

## Issue #292 — Docker image 0.15.1 not found

### Status: FIXED — owner published v0.16.0

Evidence: `raphaelmansuy commented 4d ago: "I have published 0.16"`

**Close** with note: use `ghcr.io/raphaelmansuy/edgequake:0.16.0`

---

## Issue #239 — Partial failure log details in WebUI

### Status: FIXED in v0.16.x (PipelineStatusDialog)

The WebUI already has the `PipelineStatusDialog` component with:
- Structured pipeline stage messages (per-stage error reasons)
- Phase-level error details (chunk errors, extraction failures)
- Retry information and stage progression
- `current_stage` and `stage_message` fields in document API response

**Close** with reference to pipeline status dialog and `GET /api/v1/documents/{id}`.

---

## Issue #186 — Add Ollama cloud model support

### Status: FIXED in v0.14+

Evidence in current code (`create_safe_llm_provider`):
```rust
// OLLAMA_API_KEY forwarded to OllamaProvider
if let Ok(api_key) = std::env::var("OLLAMA_API_KEY") {
    if !api_key.is_empty() {
        builder = builder.api_key(&api_key);
    }
}
```

And for embeddings:
```rust
// OLLAMA_EMBEDDING_HOST + OLLAMA_API_KEY for cloud/remote Ollama
if let Ok(api_key) = std::env::var("OLLAMA_API_KEY") {
    builder = builder.api_key(&api_key);
}
```

**Close** with env var documentation.

---

## Issue #37 — Context-only retrieval (no LLM answer)

### Status: ALREADY IMPLEMENTED

Evidence: multiple tests use `context_only: true`:
```rust
// e2e_query_engine.rs:132
async fn test_context_only_query() {
    // context_only=true skips LLM generation, returns empty answer + context
    "context_only": true
```

**Close** with API documentation showing `context_only=true` in POST /query.

---

## Implementation Plan

### Priority and sequencing

1. **P0 — #300:** restore trustworthy PDF progress correlation (code landed;
   published-image Mistral acceptance still open).
2. **P0 — #298-A:** after restart recovery, enqueue tasks for pending orphans
   (document↔task coupling). Critical for ECS + large workspaces.
3. **P1 — #298-B/#298-C:** startup requeue vs channel capacity; reprocess API/UI
   honesty for pending without force ambiguity.
4. **P1 — #300 hardening:** reindex, duplicate, batch, reconnect, legacy fallback.
5. **P2 — release proof:** published-image topology with Mistral before patch tag.
6. Preserve #297 / #294 work; do not mix into the #300 hotfix unless already done.

### Phase 0 — Freeze the identity contract

Document and enforce these API semantics:

- `task_id`: unique server job ID and the authoritative progress/cancel/retry
  key.
- `track_id`: optional client batch/request correlation ID; never a worker
  progress key.
- `pdf_id` and `document_id`: resource IDs, not progress IDs.
- `PdfUploadResponse.task_id` MUST be non-empty for `queued`, `processing`, and
  `reindexing` responses.
- Duplicate responses with no new task MUST NOT create a pending progress
  record.

Do not alias a shared batch ID to multiple task progress records.

### Phase 1 — Backend P0 fix

#### 1. Seed progress under the task ID only

File:
`edgequake/crates/edgequake-api/src/handlers/pdf_upload/upload.rs`

For fresh upload and force-reindex paths:

- Remove `effective_track_id = options.track_id.unwrap_or(enqueue.track_id)`.
- Call `start_pdf_progress()` with `enqueue.track_id`.
- Keep returning `task_id: enqueue.track_id`.
- Retain `track_id: options.track_id` only as batch correlation data.
- Do not call `start_pdf_progress()` for a duplicate when no task was created.

This aligns upload initialization with `PipelineProgressCallback`, task cleanup,
document metadata, cancellation, retry, WS, and SSE, all of which already use
the server task ID.

#### 2. Preserve batch correlation without changing worker identity

File:
`edgequake/crates/edgequake-api/src/handlers/pdf_upload/helpers.rs`

If batch correlation is needed operationally, copy `options.track_id` into task
metadata as `client_track_id` (or `batch_track_id`). It must remain descriptive
metadata and must not replace `Task.track_id`.

#### 3. Make contract misuse fail loud

Files:

- `edgequake/crates/edgequake-api/src/handlers/pdf_upload/status.rs`
- `edgequake/crates/edgequake-api/src/handlers/websocket.rs`

Unknown batch IDs should return the existing unknown-track behavior (404 or
explicit not-found event), not a permanent all-pending progress skeleton. This
turns identity misuse into a diagnosable error instead of an infinite spinner.

### Phase 2 — WebUI P0 fix

#### 1. Normalize PDF progress to `task_id`

File:
`edgequake_webui/src/lib/upload/perform-file-upload.ts`

For PDF responses, set the normalized progress identity to:

```ts
track_id: pdfResponse.task_id || pdfResponse.track_id
```

`task_id` is authoritative for v0.17.0+; the `track_id` fallback keeps the
frontend compatible with older servers that may not return a task ID.

#### 2. Use the normalized task ID everywhere

File:
`edgequake_webui/src/hooks/use-file-upload.ts`

Ensure the same normalized ID is used for:

- `useIngestionStore.startTracking()`
- optimistic document `track_id`
- active upload-row `trackId`
- progress dialog, WS/SSE, completion removal, retry, and cancellation

Keep the client-generated batch ID only in the multipart request. Rename local
variables to `batchTrackId` and `progressTrackId` to prevent future conflation.

#### 3. Add a terminal fallback

The document list already receives `current_stage`, `stage_message`, and
terminal status from metadata. If a progress socket/poll misses completion,
reconcile the tracked upload row with the matching `document_id` terminal
state. This is defence in depth; it does not replace the identity fix.

### Phase 3 — Regression tests

#### Backend tests

Add focused coverage in the existing PDF/progress integration suites:

1. Upload with client `track_id=B`; assert response has `track_id=B` and a
   distinct non-empty `task_id=T`.
2. Assert progress is initialized and advances under `T`.
3. Assert `B` is not exposed as a permanently pending PDF progress record.
4. Upload without client `track_id`; assert `task_id=T` remains usable.
5. Force-reindex; assert the new `task_id` is the progress identity.
6. Duplicate without reindex; assert no phantom progress record is created.
7. Upload two PDFs using the same batch ID; assert two unique task IDs and
   isolated progress.
8. Assert WS and SSE events carry the same task ID returned by the upload.
9. Assert terminal cleanup removes progress for `T`, not `B`.

Candidate suites:

- `edgequake/crates/edgequake-api/tests/contract_spec048_progress.rs`
- `edgequake/crates/edgequake-api/tests/e2e_spec013_mistral_pdf_query.rs`
- `edgequake/crates/edgequake-api/tests/e2e_spec014_multi_upload.rs`
- `edgequake/crates/edgequake-api/tests/e2e_reindexing.rs`

#### Frontend tests

Files:

- `edgequake_webui/src/lib/upload/__tests__/perform-file-upload.test.ts`
- a focused `use-file-upload` hook test (create if absent)

Required cases:

1. When `task_id !== track_id`, normalized `track_id` equals `task_id`.
2. When a legacy response lacks `task_id`, fallback to `track_id`.
3. Two PDF responses in one batch track two distinct task IDs.
4. Completion for one task removes only its own upload row.
5. Failed progress and terminal document metadata end the loading state.

### Phase 4 — Verification gates

Run the smallest gates first, then expand:

```bash
cargo fmt --check
cargo test -p edgequake-api --lib
cargo test -p edgequake-api --test contract_spec048_progress
cargo test -p edgequake-api --test e2e_reindexing
cargo test -p edgequake-api --test e2e_spec014_multi_upload
cargo clippy -p edgequake-api --all-targets -- -D warnings

cd edgequake_webui
bun test src/lib/upload/__tests__/perform-file-upload.test.ts
bun test
bun run build
```

Then run the published-topology acceptance test:

1. Build API and frontend images from the fix on both supported architectures.
2. Start a clean PostgreSQL image and empty volume.
3. Configure Mistral:
   `mistral-small-latest` for chat/vision and `mistral-embed` for embeddings.
4. Upload the 1-page fixture and a multi-page PDF through the WebUI.
5. Verify transfer → conversion → extraction → graph storage → completed.
6. For every upload, record that response `task_id`, REST progress, WS/SSE
   events, document metadata, and UI row use the same progress ID.
7. Repeat with two PDFs in one selection and with force-reindex.
8. Verify no row remains loading after its document reaches a terminal state.

### Acceptance criteria for #300

- PDF upload returns promptly and processing continues asynchronously.
- The first progress update appears under `response.task_id`.
- Progress advances monotonically to a terminal state in REST and WebUI.
- A client batch ID cannot merge progress from separate files.
- Success, failure, duplicate, cancellation, and reindex all terminate their UI
  state.
- Mistral vision ingest still completes and extracted content remains available.
- No regression for text, image, or legacy-server upload paths.
- Published Docker topology passes on `linux/amd64` and `linux/arm64`.

### Rollout and observability

- Ship as a patch release because #300 affects the primary v0.17.0 ingest UX.
- Add a temporary structured warning when `task_id != client_track_id`, logging
  both values and explicitly naming `task_id` as the progress key.
- Add a counter for progress subscriptions to unknown IDs; it should fall to
  zero after the frontend rollout.
- Update API/OpenAPI descriptions and the issue comment with the patch version.
- Do not close #300 until a clean published-image installation passes the
  Mistral WebUI acceptance flow.

### Remaining SPEC-054 items

#### Fix #298-A — Enqueue tasks for auto-recovered pending docs

Files:

- `edgequake/src/main.rs` (`recover_orphaned_documents` / post-recovery reconciler)
- Reuse routing from `handlers/documents/recovery/stuck.rs` and
  `handlers/workspaces/bulk_ops/mod.rs::build_reprocess_task`

Add e2e: pending KV + zero tasks → recover path → `TaskStatus::Pending` exists
and `/pipeline/activity` lists the doc under `queued`.

#### Fix #298-B — Startup hydrate must not block forever

Reorder worker start vs bounded `ChannelTaskQueue` fill, or paginate hydrate
with a continuous pending-task pump (Postgres SSOT).

#### Fix #298-C — Reprocess pending orphans honestly

Ensure WebUI stuck CTA uses force reprocess or recover-stuck; return skip
reasons when `requeued=0`.

#### Fix A — #297: Drop vector table on workspace delete

File: `edgequake/crates/edgequake-api/src/handlers/workspaces/workspace_crud.rs`

After step 4 (`vector_registry.evict`), add step 4b to drop the orphan table.

#### Fix B — #294: Warn on instance-local API-key storage

File: `edgequake/crates/edgequake-api/src/state/runtime_extractors.rs`

Emit `tracing::warn!` when API-key persistence falls back to in-memory storage,
making multi-instance misconfiguration visible.
