# SPEC-006: Ensure Performance & Resource Safety — Problem Statement

**Spec ID:** `006-ensure-perf`  
**Status:** Draft  
**Created:** 2026-06-06

---

## WHY — The Problem

EdgeQuake crashed in production with **exit code 137** — the Linux/Docker OOM killer.
Operators suspect a **very large workspace** ("monster database") triggered unbounded
memory growth in the API process.

### First-Principles Diagnosis

Exit 137 is not a logic bug. It is a **resource invariant violation**:

```text
Peak process RAM  >  cgroup / host available RAM
        ↓
   kernel SIGKILL (signal 9)
        ↓
   container exit 128 + 9 = 137
```

The system has **local caps** (graph viewer max 500 nodes, page_size clamp 100) but
**systemic unbounded loaders** (`get_all_nodes`, `get_all_edges`) on maintenance and
list paths. Pagination at the HTTP layer does not help when handlers materialize the
entire graph first.

### Impact

| Stakeholder | Pain |
|-------------|------|
| Production operators | Silent process death; no graceful 503 |
| Tenants with large graphs | Any list/delete/lineage call can kill the pod |
| Engineering | Limits scattered across 6+ crates; easy to regress |
| SRE | No `mem_limit`, no resource metrics on graph scans |

---

## Scope

### In Scope

1. **Memory safety** — eliminate unbounded in-process graph materialization on request paths
2. **Concurrency safety** — bound worker × extraction × connection multiplication
3. **Budget SSOT** — one catalog ([004](004_resource_budget_catalog.md)) referenced by all crates
4. **Regression contracts** — CI gates that fail on new `get_all_*` in API layer ([008](008_regression_contract.md))
5. **Operator controls** — Docker limits, env tuning, observability ([009](009_operator_runbook.md))
6. **DRY/SOLID remediation design** — `ResourceGuard`, pushed-down queries ([006](006_architecture_remediation.md))

### Out of Scope

- Query quality / retrieval accuracy trade-offs (unless caused by budget change)
- PostgreSQL server tuning (shared_buffers, AGE install) — noted in runbook only
- Frontend graph rendering perf (WebGL) — separate from backend OOM class
- Billing/quota SaaS limits (see SPEC-0001 tenant workspace limits)

---

## Success Criteria

| ID | Criterion | Verification |
|----|-----------|--------------|
| **NFR-006-SC-01** | API process survives 200k-node workspace list/delete simulation | `resource-proof` integration test |
| **NFR-006-SC-02** | Peak RSS under 80% of `mem_limit` during 10 concurrent graph queries | Load test + cgroup metrics |
| **NFR-006-SC-03** | No handler loads full graph when `node_count_fast() > GRAPH_SCAN_THRESHOLD` | Static allowlist + runtime guard |
| **NFR-006-SC-04** | All resource caps traceable to [004](004_resource_budget_catalog.md) | Audit script |
| **NFR-006-SC-05** | Zero regression on existing e2e upload/query/delete tests | CI `cargo test` + `make dev-bg` smoke |

---

## Non-Goals (explicit)

- Making community detection (Louvain) run on million-node graphs in-process
- Guaranteeing sub-100ms list-entities on 1M nodes without DB indexes (correctness > magic)
- Removing `get_all_*` from **test/bench** code (allowlisted in 008)
