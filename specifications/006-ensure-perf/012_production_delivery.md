# SPEC-006 — Production Delivery Assessment (P9)

**Date:** 2026-06-06  
**Verdict:** **GO** for API server deployment (exit-137 class mitigated)  
**Grade:** **A** (see [010-brutal-assessment.md](010-brutal-assessment.md))

---

## GO / NO-GO Checklist

| # | Gate | Command / check | Required |
|---|------|-----------------|----------|
| 1 | Resource proof (mock) | `make resource-proof` | ✅ |
| 2 | Resource proof (postgres) | `make resource-proof-postgres` | ✅ before prod |
| 3 | Clippy strict | `cargo clippy --workspace --lib -- -D warnings` | ✅ |
| 4 | Format | `cargo fmt --all -- --check` | ✅ |
| 5 | `/ready` after deploy | `curl -sf http://HOST:8080/ready` → 200 | ✅ |
| 6 | Migration 038 on large graphs | `apply_038.sh --apply --concurrent --yes` if `/ready` 503 | ✅ when degraded |
| 7 | Memory limit | Docker `mem_limit: 4g` or `EDGEQUAKE_MEM_LIMIT` set | ✅ |
| 8 | Worker caps | `WORKER_THREADS`, `MAX_TASKS_PER_TENANT` per [009 §2.2](009_operator_runbook.md) | ✅ |
| 9 | Runbook ↔ code sync | `scripts/spec006_runbook_env_sync.sh` | ✅ |
| 10 | Orchestrator deletion bounded | `scripts/spec006_no_get_all_orchestrator.sh` | ✅ |

**NO-GO** if any of 1–4 fail in CI, or `/ready` stays 503 after concurrent 038 on a graph >50k nodes.

---

## What Ships in This Version

| Capability | Status |
|------------|--------|
| API hot paths: zero `get_all_*` | ✅ |
| DRY `AppState::resource_budget()` | ✅ |
| Graph materialization semaphore (fail-fast 503) | ✅ all materialization endpoints |
| Document delete cascade (API) | ✅ `document_graph_cascade.rs` |
| Orchestrator `delete_document` (SDK) | ✅ bounded `GraphScanOps` (P9) |
| Migration 038 + size-aware bootstrap | ✅ |
| `/ready` index gate | ✅ |
| Community detection guard | ✅ |

---

## Residual Debt (non-blocking for API prod)

| Item | Risk | Mitigation |
|------|------|------------|
| `e2e_document_deletion.rs` uses `get_all_*` on tiny mocks | Test-only; no prod path | Future test refactor |
| `EDGEQUAKE_MEM_LIMIT` warn-only | Host OOM if no cgroup | Docker `mem_limit` |
| PDF/vision RAM spikes | Separate incident class | Vision timeout + upload cap |
| `search_labels` unguarded | Low — no full graph load | Accept |

---

## Operator Deploy Sequence

```bash
# 1. CI / pre-release
make resource-proof-postgres
cargo clippy --workspace --lib -- -D warnings

# 2. Deploy image
docker compose -f edgequake/docker/docker-compose.yml up -d edgequake

# 3. Post-deploy health
curl -sf http://localhost:8080/health | jq .
curl -sf http://localhost:8080/ready   # must be 200

# 4. If /ready 503 on large workspace
edgequake/migrations/apply_038.sh --apply --concurrent --yes
curl -sf http://localhost:8080/ready   # re-check

# 5. Smoke graph under load cap
curl -sf -H "X-Tenant-ID: $T" -H "X-Workspace-ID: $W" \
  "http://localhost:8080/api/v1/graph?max_nodes=50"
```

---

## First-Principles Delivery Statement

1. **Peak RAM** is bounded on API request paths (no full-graph materialization without semaphore + timeout).
2. **Single budget authority** — handlers and server read `AppState::resource_budget()`.
3. **Delete correctness** — document-scoped prefix scan, not workspace-wide load (API + orchestrator).
4. **Ops truth** — runbook env vars verified against code anchors (P9 lint).

**Bottom line:** Safe to deploy this version for production API workloads when checklist items 1–10 pass and ops follow [009](009_operator_runbook.md).
