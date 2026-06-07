# edgequake-core — Observability Audit

**Path:** `edgequake/crates/edgequake-core`  
**Tracing macros (src):** ~42  
**Role:** Orchestration, workspace/tenant services, legacy query path

---

## Executive Summary

Core emits **lifecycle and tenant logs** but has **no awareness of HTTP request_id**. Long-running orchestration (ingestion, deletion) logs at `info!` without duration fields on all paths.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| CORE-OBS-001 | P1 | No request context propagation | Orchestrator methods lack `request_id` param | Thread `tracing::Span` parent from API |
| CORE-OBS-002 | P2 | `tenant_manager.rs` heavy info | ~11 tracing calls | Add `tenant_id` field consistently |
| CORE-OBS-003 | P2 | `orchestrator/ingestion.rs` errors sparse | 2 `error!` | Log extraction failures with doc_id |
| CORE-OBS-004 | P3 | `println!` in orchestrator test path | `orchestrator/mod.rs:1` in tests only | OK |

---

## Hot Paths

```
API handler
    └── WorkspaceService / Orchestrator
            ├── ingestion.rs   (info, warn, error)
            ├── deletion.rs    (info)
            └── tenant_manager (info, warn)
```

**DRY:** Duplicate workspace resolution logging exists in API + core — prefer single span in API, core logs at DEBUG.

---

## Target

- All orchestrator entry points: `#[instrument(skip(self), fields(tenant_id, workspace_id, document_id))]`
- Errors: `error!(error = %e, "ingestion failed")` — never swallow without log

---

## Verify

```bash
rg 'tracing::(info|warn|error|debug)!' edgequake/crates/edgequake-core/src -c
```
