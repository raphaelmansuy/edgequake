# `GH-316` — Workspace processing serialized (cross-workspace wait)

> **Priority**: P1  
> **Audit status**: FIXED  
> **Sprint**: 2  
> **Laws**: LAW-13, LAW-3, LAW-8  
> **GitHub**: https://github.com/raphaelmansuy/edgequake/issues/316  
> **Verified against**: v0.21.0 / `19477c2d`

---

## 1. WHY

In multi-workspace (same tenant) deployments, a long ingest in Workspace A leaves Workspace B queued. Multi-tenant UX feels single-tenant. Throughput and perceived reliability collapse when teams share a tenant.

---

## 2. Audit (code is law)

| Field | Value |
|-------|-------|
| Claim | Global FIFO `ORDER BY created_at ASC` — **no workspace predicate** ([`postgres.rs`](../../../edgequake/crates/edgequake-tasks/src/postgres.rs) `claim_next`) |
| Fairness | `TenantConcurrencyLimiter` — **per tenant**, not per workspace ([`tenant_limiter.rs`](../../../edgequake/crates/edgequake-tasks/src/tenant_limiter.rs)) |
| Local clamps | Workers ≤4, ingest/tenant ≤2, process-wide LLM gate — amplify waits |
| Dual lanes | Ingest vs lifecycle — helps deletes vs PDFs **within** tenant, not across workspaces |
| Verdict | **CONFIRMED** (tenant fairness is partial mitigation only for **cross-tenant**, not #316) |

There is no hard “one workspace mutex,” but emergent serialization matches the bug report under backlog + local LLM.

---

## 3. Root cause (first principles)

**LAW-13**: When the product isolates data by `workspace_id`, scheduling fairness must include that key (within tenant budgets). Global oldest-first claim + tenant-wide semaphore ⇒ Workspace B waits behind Workspace A’s entire backlog (head-of-line blocking).

---

## 4. Multi-lens analysis

### Product Owner

- Acceptance: Two workspaces under one tenant make forward progress concurrently (subject to global LLM/hardware caps). Starvation bound: Workspace B starts within a defined fairness window while A is saturated.
- Do not promise unbounded parallel LLM calls on a laptop Ollama — document caps.

### Full Stack

| Component | Role |
|-----------|------|
| `claim_next` | Needs workspace-fair candidate selection or post-claim park by workspace lane |
| Limiter | Add `WorkspaceConcurrencyLimiter` or composite key `(tenant, workspace)` with reserved slots |
| Docs | FAQ already explains tenant fairness; extend for workspace |

### AI Engineer

- Local inference gate remains process-wide (protect Ollama). Workspace fairness allocates **task slots**, not unlimited model concurrency.
- Cloud providers can raise caps; law still holds.

### O(n) / Systems

- Classic HOL blocking: one queue, many logical tenants (workspaces).
- Fix patterns: deficit round-robin across workspace_id among pending tasks; or per-workspace queues with worker steal.
- Complexity: claim SQL must stay `SKIP LOCKED` safe under replicas.

### Postgres Expert

- Candidate CTE today:

```sql
WHERE status = 'pending' OR (status = 'processing' AND lease expired)
ORDER BY created_at ASC
FOR UPDATE SKIP LOCKED LIMIT 1
```

- Workspace-fair variant (conceptual): pick among workspaces with pending work using a fairness score / last_scheduled, then oldest task in that workspace — still `SKIP LOCKED`.
- Avoid full table sorts without index support: ensure `(status, workspace_id, created_at)` index exists or add migration.

---

## 5. ASCII causal diagram

```
  WS-A backlog (old created_at) ----+
                                    |
                                    v
                         global claim_next FIFO
                                    |
  WS-B new tasks -------------------+--> wait behind A
                                    |
                         tenant ingest semaphore (shared)
                                    |
                                    v
                         B stays Queued (reporter symptom)
```

---

## 6. Solution (SOLID + DRY)

| Principle | Application |
|-----------|-------------|
| S | Scheduler fairness module owns workspace lanes |
| O | Policy: `FairnessKey = Tenant` \| `TenantWorkspace` (feature/env) |
| L | Lease/claim contracts unchanged for workers |
| I | Limiter trait `try_acquire(tenant, workspace, class)` |
| D | Workers depend on limiter abstraction |
| DRY | Extend `tenant_limiter` rather than fork second worker pool |

### Implementation steps (locked approach)

1. Add **workspace lane** semaphore: max concurrent ingest tasks per `(tenant_id, workspace_id)` plus existing tenant cap.
2. Change `claim_next` to workspace-fair selection (round-robin / least-recently-served workspace with pending tasks), then oldest task in that workspace.
3. Index support migration if EXPLAIN shows sort/filter cost.
4. Keep local LLM process gate; document that fairness ≠ unlimited parallel inference.
5. Metrics: `tasks_claimed{workspace_id}` for proof.

---

## 7. Edge cases

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Single workspace | Behavior ≈ today |
| EC-2 | Many empty workspaces | Skip workspaces with no pending |
| EC-3 | Cross-tenant | Separate tenant semaphores unchanged |
| EC-4 | Lifecycle vs ingest | Preserve dual lanes per workspace or per tenant (document choice: **per tenant lanes + per workspace ingest slots**) |
| EC-5 | Lease expiry reclaim | Fairness must not starve reclaim |
| EC-6 | High workspace count | Cap distinct lane map; LRU lane table |
| EC-7 | `EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY` | Still respect workspace fairness |

---

## 8. E2E / contract tests

| Test | Assertion |
|------|-----------|
| `issue316_two_workspaces_interleaved_progress` | Same tenant; A has 20 pending, B has 2; B reaches processing before A drains |
| `issue316_tenant_cap_still_holds` | Sum of workspace in-flight ≤ tenant max |
| `issue316_claim_sql_uses_workspace_index` | EXPLAIN / schema has supporting index |

---

## 9. Cross-refs

- `docs/ingestion-cancel-and-fairness.md`  
- `docs/faq.md` tenant fairness  
- Local clamps in `edgequake-pipeline` config  
- Related: #318 (readiness) orthogonal; #331 (pool) can worsen multi-WS waits
