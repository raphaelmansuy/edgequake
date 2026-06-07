# SPEC-006: Operator Runbook — Resource Safety

**Spec ID:** `006-ensure-perf`  
**Status:** Active (P8)  
**Audience:** SRE, DevOps, on-call

---

## 1. Incident: Exit Code 137

### Symptom

```text
exited (137) edgequake  ghcr.io/raphaelmansuy/edgequake:...
```

### Meaning

| Code | Signal | Cause |
|------|--------|-------|
| 137 | SIGKILL (9) | OOM killer (kernel or Docker cgroup) |

### Immediate triage

```bash
# Container memory at death
docker inspect edgequake --format '{{.HostConfig.Memory}}'
dmesg | tail -20 | grep -i oom    # Linux host

# Workspace size
curl -s http://localhost:8080/api/v1/workspaces/{id}/stats | jq .

# Recent expensive ops (logs)
grep -E 'get_all_nodes|delete_document|list_entities|Graph query timed out' /tmp/edgequake-backend.log | tail -50
```

### Likely triggers (priority order)

1. Document delete on large graph ([V-006-002](005_violation_registry.md))
2. Entities/relationships list ([V-006-001](005_violation_registry.md))
3. Graph viewer timeout fallback ([V-006-003](005_violation_registry.md))
4. Lineage panel open ([V-006-004](005_violation_registry.md))
5. Burst PDF upload × workers ([V-006-006](005_violation_registry.md))

### Mitigation (pre-fix)

```bash
# Reduce concurrency immediately
export WORKER_THREADS=4
export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=2
export MAX_TASKS_PER_TENANT=2

# Restart with memory cap (prevents host-wide OOM)
# Add to docker-compose.yml under edgequake service:
#   mem_limit: 4g
#   memswap_limit: 4g

make stop && make dev-bg
```

---

## 2. Recommended Production Settings

### 2.1 Docker (`docker-compose.yml`)

```yaml
edgequake:
  mem_limit: 4g          # OR-006-001 — tune: 2g dev, 8g large prod
  memswap_limit: 4g      # prevent swap thrashing
  cpus: 4
  environment:
    - WORKER_THREADS=8
    - EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4
    - MAX_TASKS_PER_TENANT=6
    - EDGEQUAKE_GRAPH_SCAN_THRESHOLD=50000
    - EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT=1
```

### 2.2 Environment variable reference

Full catalog: [004](004_resource_budget_catalog.md). High-signal subset:

| Variable | Dev | Prod | File anchor |
|----------|-----|------|-------------|
| `WORKER_THREADS` | 4 | 8 | `main.rs:572` |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | 4 | 4–8 | `pipeline/config.rs:128` |
| `MAX_TASKS_PER_TENANT` | 2 | 6 | `main.rs:589` |
| `TASK_PROCESSING_TIMEOUT_SECS` | 7200 | 7200 | `main.rs:599` |
| `EDGEQUAKE_LLM_MAX_TOKENS` | 16384 | 16384 | `safety_limits.rs:27` |
| `EDGEQUAKE_VISION_TIMEOUT_SECS` | 600 | 600 | `docker-compose.yml:67` |
| `EDGEQUAKE_GRAPH_SCAN_THRESHOLD` | 50000 | 50000 | `resource/budget.rs` |
| `EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT` | 1 | 1–2 | `resource/semaphore.rs` |
| `EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS` | 15 | 15–30 | `graph_materialization.rs` |
| `EDGEQUAKE_MAX_UPLOAD_BYTES` | 52428800 | 52428800 | `server.rs` via `resource_budget()` |
| `DATABASE_URL` | required | required | — |

### 2.3 Kubernetes (OR-006-002)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: edgequake
spec:
  template:
    spec:
      containers:
        - name: edgequake
          resources:
            requests:
              memory: "2Gi"
              cpu: "2"
            limits:
              memory: "4Gi"   # mirrors EDGEQUAKE_MEM_LIMIT / docker mem_limit
              cpu: "4"
```

**Rule:** set `limits.memory` ≥ [capacity formula](#3-capacity-planning-formula) × 2 for graph headroom.

Apply migration **038** after deploy for source-prefix index performance:

```bash
psql "$DATABASE_URL" -f edgequake/migrations/038_add_source_ids_gin_indexes.sql
```

### 2.4 PostgreSQL

```sql
-- Per-session safety (optional, via connection options)
SET statement_timeout = '30s';   -- graph analytics queries
```

Pool default: 32 connections (`postgres/config.rs:89`).  
**Rule:** `pool_size ≥ WORKER_THREADS + 8` ([BR-006-013](004_resource_budget_catalog.md)).

---

## 3. Capacity Planning Formula

```text
recommended_mem_limit =
    BASE_RUST_MB (200)
  + WORKER_THREADS × AVG_DOC_MB (50)
  + EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS × CHUNK_BUFFER_MB (2)
  + GRAPH_HEADROOM_MB (500)    # safety margin
```

Example (prod defaults):

```text
200 + 8×50 + 4×2 + 500 = 908 MB minimum
→ set mem_limit: 4g for headroom on large graph ops until SPEC-006 P0 ships
```

---

## 4. Monitoring & Alerts

### Required metrics (post SPEC-018 + SPEC-006)

| Alert | Condition | Action |
|-------|-----------|--------|
| `OOMRiskAdmissionReject` | `resource_admission_total{result=reject}` > 10/min | Scale RAM or finish P0 migration |
| `ContainerMemoryHigh` | cgroup > 85% for 5m | Reduce workers |
| `GraphQueryTimeout` | timeout rate > 5% | Check AGE perf; **do not** rely on fallback |
| `TaskQueueDepth` | queue > 80 sustained | Throttle uploads |

### Log queries

```bash
# Admission rejects (after SPEC-006)
grep 'resource_admission_rejected' /tmp/edgequake-backend.log

# Graph fallback (remove after P0)
grep 'falling back to simple node fetch' /tmp/edgequake-backend.log
```

---

## 5. Health Checks

```bash
# Service up
curl -f http://localhost:8080/health

# Workspace not monster (manual)
curl -s http://localhost:8080/api/v1/workspaces/{id}/stats \
  | jq '{entities: .entity_count, relationships: .relationship_count, docs: .document_count}'

# If entity_count > 50000: expect list/delete risk until SPEC-006 P0 deployed
```

---

## 6. Upgrade / Rollout Checklist

- [ ] `make resource-proof` passes on release candidate
- [ ] `mem_limit` set in compose/k8s manifest
- [ ] Env vars documented in `.env.example`
- [ ] `scripts/spec006_no_get_all_api.sh` allowlist shrinking — track in release notes
- [ ] Rollback: previous image tag + same mem_limit

---

## 7. Post-Remediation Verification

After ADR-006 ([007](007_adr.md)) deployed:

```bash
# Must return 0 unallowlisted
./scripts/spec006_no_get_all_api.sh

# Load test (staging)
# 200k node fixture: list + delete must complete without RSS > 80% mem_limit
make resource-proof
```

---

## Cross-refs

- [001 Problem](001_problem_statement.md)
- [004 Budget catalog](004_resource_budget_catalog.md)
- [005 Violations](005_violation_registry.md)
- [008 Regression gates](008_regression_contract.md)
- [docs/OBSERVABILITY.md](../../docs/OBSERVABILITY.md)
