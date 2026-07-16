# SPEC-054 — GitHub Issue Comments (Ready to Post)

Copy each comment block and paste it on the corresponding issue.

---

## Issue #292 — Docker image 0.15.1 not found

**CLOSE WITH:**

```
✅ Fixed — Docker image `ghcr.io/raphaelmansuy/edgequake:0.16.0` is now published.

Please update your deployment to use:
```yaml
image: ghcr.io/raphaelmansuy/edgequake:0.16.0
```

The `latest` tag also points to 0.16.0. Note: version 0.15.1 was never published as a separate image — the recommended upgrade path is to use the latest stable release.
```

---

## Issue #296 — Build Fails Due to proxyClientMaxBodySize Type Error

**CLOSE WITH:**

```
✅ Fixed in current codebase (will ship in next release).

**Root cause**: `DEV_PROXY_MAX_BODY` was a template string `"${n}mb"` which TypeScript widened to `string`. Next.js 16 requires `SizeLimit = number | \`${number}${FileSizeSuffix}\`` for `proxyClientMaxBodySize`.

**Fix applied** in `next.config.ts`:
```ts
// Before (broken):
const DEV_PROXY_MAX_BODY = `${50 * 1024 * 1024}b`  // type: string

// After (fixed):
proxyClientMaxBodySize: DEFAULT_MAX_UPLOAD_BYTES,  // type: number (50 * 1024 * 1024)
```

The fix uses a numeric byte value which satisfies the `SizeLimit` type constraint. The build now passes `next build` type checking cleanly.
```

---

## Issue #186 — Add Ollama Cloud Models

**CLOSE WITH:**

```
✅ Implemented — Ollama cloud/remote model support is available now.

**How to use Ollama Cloud or any authenticated Ollama endpoint:**

```env
OLLAMA_HOST=https://your-ollama-cloud-endpoint
OLLAMA_API_KEY=your-api-key
```

**For separate embedding host:**
```env
OLLAMA_EMBEDDING_HOST=https://embed.your-endpoint.com
OLLAMA_API_KEY=your-api-key
```

The `OLLAMA_API_KEY` is automatically forwarded to all Ollama provider builders (LLM, embedding, and vision paths). This was added in SPEC-033 hybrid provider mode and verified working against authenticated Ollama endpoints.

If you're using Ollama Cloud specifically, set `OLLAMA_HOST=https://ollama.com` and your cloud API key — the provider will use it for Bearer authentication.
```

---

## Issue #37 — Get Only Retrieval Chunks (No LLM Answer)

**CLOSE WITH:**

```
✅ Implemented — use `context_only: true` in your query request.

**API usage:**
```json
POST /api/v1/query
{
  "query": "your question",
  "context_only": true,
  "mode": "mix"
}
```

**Response with `context_only: true`:**
```json
{
  "answer": "",
  "context_chunks": [
    {
      "chunk_id": "...",
      "content": "...",
      "document_title": "...",
      "similarity": 0.92
    }
  ],
  "entities": [...],
  "relationships": [...]
}
```

This skips the LLM generation step entirely and returns the raw retrieval context — chunks, entities, and relationships — so you can pass them to your own agent or LLM. The `answer` field will be an empty string.

This was implemented in EdgeQuake core (SPEC-021) and is available in all versions ≥ 0.10.
```

---

## Issue #239 — Partial Failure Log Details in WebUI

**CLOSE WITH:**

```
✅ Implemented in v0.16 — detailed pipeline error information is now available.

**In the WebUI:**
1. Open the **Documents** page
2. Click on a document that shows "Partial Failure" or "Failed"
3. The **Pipeline Status** panel shows stage-by-stage progress with specific error messages (e.g., "Rate limit exceeded on chunk 3 of 12", "LLM timeout on entity extraction")

**Via API:**
```
GET /api/v1/documents/{document_id}
```
Response includes:
- `current_stage`: which pipeline stage failed (e.g., `"entity_extraction"`)
- `stage_message`: human-readable error description
- `pipeline_messages`: array of structured log entries per stage with timestamps, severity, and details

**Common partial failure causes:**
- LLM/embedding provider rate limits → reduce concurrency or add retry delays
- Large files exceeding chunk processing timeout → set `EDGEQUAKE_CHUNK_TIMEOUT_SECS`
- Network errors to Ollama/OpenAI → check provider connectivity

If you see "Partial Failure" that resolves on reprocess, it's typically a transient rate limit or network timeout. The pipeline retries up to 3 times automatically.
```

---

## Issue #294 — New API Keys Return 401 in ECS

**STATUS: Fix in progress (to ship in next release)**

**COMMENT:**

```
🔍 Root cause identified — this is a **multi-instance deployment issue**.

**5 WHY:**
1. New API key returns 401 → key not found in auth store on validating instance
2. Key not found → key stored in in-memory (KV) store, which is instance-local
3. Instance-local → in ECS with multiple tasks, create-task ≠ validate-task
4. Different instances → in-memory store is not shared across ECS tasks
5. Not shared → the fix requires `DATABASE_URL` to be configured so PostgreSQL is used as the shared auth backend

**Immediate workaround:**
```env
DATABASE_URL=postgresql://user:pass@your-rds/edgequake
```

When `DATABASE_URL` is set, API keys are stored in and retrieved from PostgreSQL — shared across all ECS instances.

**How to verify you have the fix:**
After deploying with `DATABASE_URL`, create a new key on instance A and call the API from instance B. It should return 200.

**Coming in next release:**
- A `WARN` log is now emitted when API keys fall back to in-memory storage: `SPEC-054/gh#294: API key persisted to in-memory (instance-local) store`
- This makes misconfiguration visible immediately on startup

**Root cause**: The older static API keys (set via `EDGEQUAKE_API_KEYS` env var) work because they're passed to every instance via environment variables. Keys created via `POST /api/v1/api-keys` are only stored in the auth backend — which must be PostgreSQL for multi-instance deployments.
```

---

## Issue #297 — Cross-workspace Contamination + Orphan Vectors

**STATUS: Fix shipped (orphan table drop added)**

**COMMENT:**

```
🔍 Analysis complete. Three separate issues reported — here's the status of each:

---

### Issue 1: Wrong `document_title` in search results (cross-workspace)

**Status: Cannot reproduce in v0.16.x**

The contamination you observed was on v0.12.11. Current versions (≥ v0.14) enforce strict workspace isolation via:
- Per-workspace vector tables (`eq_..._ws_{id}_vectors`) with `workspace_id` column filters
- All queries include `WHERE workspace_id = $workspace_id` — cross-workspace data cannot leak

If you can reproduce this on v0.16.0+, please provide:
1. The `X-Workspace-ID` headers sent on ingest AND query
2. The query response showing the mismatched `document_title`

---

### Issue 2: Orphan vector table after DeleteWorkspace

**Status: ✅ Fixed (ships in next release)**

Root cause: `delete_workspace` called `clear_workspace()` (deletes rows) but never `DROP TABLE` on the per-workspace vector table. The physical table remained as an orphan.

Fix: `WorkspaceVectorRegistry` now has a `drop_workspace_table()` method called during workspace cascade delete, which executes `DROP TABLE IF EXISTS eq_..._ws_{id}_vectors`.

**Manual cleanup for existing orphan tables:**
```sql
-- List orphan tables (workspaces that no longer exist)
SELECT tablename 
FROM pg_tables 
WHERE schemaname = 'public' 
  AND tablename LIKE 'eq_%_ws_%_vectors'
  AND tablename NOT IN (
    SELECT 'eq_eq_default_ws_' || LEFT(REPLACE(workspace_id::text, '-', ''), 8) || '_vectors'
    FROM workspaces
  );

-- Drop them (verify first!)
-- DROP TABLE IF EXISTS eq_eq_default_ws_XXXXXXXX_vectors;
```

---

### Issue 3: In-flight ingest lost on pod restart

**Status: Known limitation — durability improvement planned**

This is a known architectural constraint. The pipeline uses tokio async tasks which are not persisted across process restarts. Documents that were mid-pipeline at restart time will show as "Failed" or remain in "Processing" state.

**Workaround:** Use the Reprocess button on failed documents after restart. The pipeline is idempotent — reprocessing is safe.

**Planned improvement:** Persistent task queue with resume-on-restart semantics is on the roadmap.
```

---

## Issue #298 — Pipeline idle while documents stay Pending

**FIX READY COMMENT (post after patch release):**

```
✅ Fix tracked in SPEC-054 — ships in next patch after v0.17.0.

### Root cause
Document KV `pending` without matching Task rows → workers idle forever.
Especially severe on v0.12.11 (MemoryTaskStorage lost tasks on ECS recycle).

### What we fixed
1. **#298-A** — `pending_doc_task_reconcile` SSOT: after startup recovery, enqueue PDF/text tasks for orphan pending docs
2. **#298-B** — Workers start before channel hydrate; `startup_task_hydrate` runs in background (150-task soak test passes with capacity-100 channel)
3. **#298-C** — Reprocess returns `skipped` + `skip_reasons`; orphan pending reprocessable without `force`; delegates to SSOT when status is pending/queued

### Tests (local, live PostgreSQL where required)
- `e2e_spec054_pending_task_reconcile` — 7/7
- `startup_task_hydrate::requeue_over_channel_capacity_with_active_consumer` — 150 pending / capacity 100, no deadlock

### Upgrade path
Move off v0.12.11 once auth/build blockers cleared. Postgres task persistence is required for ECS durability.

Refs: `specs/054-fix-bugs-17/CODE_IS_LAW_ASSESSMENT.md`
```

---

## Issue #300 — v0.17.0 Vision upload stuck on loading

**FIX READY COMMENT (post after patch release):**

```
✅ Fix tracked in SPEC-054 — ships in next patch after v0.17.0.

### Root cause
WebUI subscribed to client batch `track_id`; workers wrote progress under server `task_id` (`pdf-<uuid>`). Client id got a permanent 0% skeleton.

### What we fixed
- Backend: `progress_identity` SSOT — seed progress under `task_id` only
- WebUI: `resolvePdfProgressTrackId()` — subscribe to `response.task_id`
- Client `track_id` remains batch correlation only

### Reproduced on published v0.17.0 + Mistral
See `specs/056-issue-release-17/` — backend completes; UI spinner was identity mismatch.

### Tests
- `e2e_spec054_pdf_progress_identity` — 4/4 on live PostgreSQL
- WebUI vitest upload — 9/9

### After upgrade
Upload response `task_id` is the progress key. Polling `response.track_id` alone will still show 0% (by design).

Refs: `specs/054-fix-bugs-17/CODE_IS_LAW_ASSESSMENT.md`
```

---

## Issue #298 — Pipeline idle (investigation comment — superseded by fix above)

**INVESTIGATION COMMENT (archived):**

```
Thanks for the detailed report and screenshots — we reproduced the *identity model* against current code and the v0.12.11 constraints you called out.

### What you are seeing is consistent with a document ↔ task desync

EdgeQuake has three layers:

1. **Document metadata** (KV) — can sit forever at `pending`
2. **Task registry** — workers only process *tasks*
3. **In-process delivery queue** — tasks must be loaded into the worker channel

`GET /pipeline/status` / activity report **idle** when there are no *working* documents and no `processing` tasks. That is intentional (SPEC-048): **queued/pending metadata alone does not make the pipeline "busy"**. So idle + hundreds Pending is not a UI contradiction — it means nothing is scheduled.

Your counters also mix scopes:
- Documents page ≈ **workspace** document KV
- Pipeline "1429 completed / 70 failed" ≈ **global** task statistics across tenants

### Why this bites hard on v0.12.11 (ECS)

On v0.12.11, task storage was still **in-memory**. After an ECS task recycle:

1. Startup `recover_orphaned_documents` rewrites in-flight docs to `pending` ("Auto-recovered after server restart…")
2. It does **not** create new tasks
3. `requeue_pending_tasks` only reloads tasks that still exist in task storage → **0** after memory loss
4. Workers start idle → Pending forever

Current code persists tasks in **Postgres** (`PostgresTaskStorage`) specifically to fix that loss-on-restart bug. Staying on 0.12.11 because of #288/#296 is understandable; that also keeps you on the broken task durability path.

### Why Reprocess can look like a no-op

`POST /documents/reprocess` **defaults to failed/cancelled only**. Pending docs need `document_id` + `force=true`. Current WebUI always sends `force=true`; older clients/bulk paths may return HTTP 200 with `requeued=0` and no toast. Even with force, enqueue can skip if content/`pdf_id` is missing.

### Immediate workaround on 0.12.11

```bash
# Prefer recover-stuck (recreates tasks for aged pending/processing)
curl -X POST "$API/api/v1/documents/recover-stuck" \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WS" -H "X-Tenant-ID: $TENANT" \
  -d '{"stuck_threshold_minutes": 5, "max_documents": 100}'

# Then force-reprocess specific IDs
curl -X POST "$API/api/v1/documents/reprocess" \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: $WS" -H "X-Tenant-ID: $TENANT" \
  -d '{"document_id":"<doc-uuid>","force":true,"max_documents":1}'

curl "$API/api/v1/tasks?status=pending"
```

### Planned fixes (SPEC-054)

Tracked in `specs/054-fix-bugs-17/ANALYSIS.md` as **#298-A/B/C**:
- After auto-recovery, **enqueue** PDF/text tasks (not metadata-only)
- Harden startup hydrate vs bounded channel for large backlogs
- Make reprocess/stuck UI report skip reasons when nothing was queued

Upgrade path once auth/build blockers (#288/#294/#296) are clear: move off 0.12.11 so Postgres task persistence + SPEC-048 stuck banner apply.
```
