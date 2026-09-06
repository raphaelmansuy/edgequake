# 14 — Roadblocks & Mitigations (SPEC-146)

Operational register for implementation. Each roadblock maps to laws, findings, and gates.

```ascii
  R1  KV list vs SQL RLS
  R2  status_counts global (SPEC-084)
  R3  Cedar per-doc O(n) on list
  R4  HNSW + ANY(allow) underfill
  R5  Missed query arm (None = all-pass)
  R6  Unique (workspace, content_hash)
  R7  M1 too fat
  R8  Hub description Acc drop
  R9  Community skip vs Mix quality
  R10 New crate + OpenAPI matrix
  R11 Two Settings routes
  R12 Task worker identity
  R13 Allow-set scale (large ANY / JOIN)
  R14 Ambiguous policy_version → policy_generation
  R15 Deny observability without user leak
  R16 Cedar per-request cost (classified subset cache)
```

## R1 — KV list vs SQL RLS

| | |
|--|--|
| **Symptom** | Titles remain visible after RLS-only ABAC |
| **Root** | List/search use `document_metadata_scan`, not `SELECT FROM documents` |
| **Mitigation** | PEP filters KV entries by allow-set (LAW-146-17). Detail/download still SQL + existence-hiding. RLS = defense in depth only |
| **Proof** | G-146-11; Playwright existence-hiding |
| **Wave** | M1a |

## R2 — `status_counts` side channel

| | |
|--|--|
| **Symptom** | Viewer sees pending/failed counts for Secret docs |
| **Root** | SPEC-084 / GH-319: counts global, list items filtered |
| **Mitigation** | When `EDGEQUAKE_DOC_ABAC=1`, compute counts over **authorized** set only. Document inherit **override** of SPEC-084 |
| **Proof** | EC-146-33; e2e counts |
| **Wave** | M1a |

## R3 — Cedar per-document on list

| | |
|--|--|
| **Symptom** | List latency explodes; policy bugs hard to reason |
| **Root** | Evaluating Cedar for every workspace doc on every list |
| **Mitigation** | LAW-146-18: SQL/set algebra for `workspace|acl|owner_only`; Cedar **only** for `classified` subset. See also R16 cache. |
| **Proof** | Unit: allow-set builder paths; no Cedar call for workspace-mode docs |
| **Wave** | M1a |

## R4 — HNSW filtered underfill

| | |
|--|--|
| **Symptom** | Authorized recall collapses when allow-set is small |
| **Root** | Typed path `ORDER BY <=> LIMIT k` with late filter; no document predicate today |
| **Mitigation** | JOIN `chunks` + allow predicate; **over-fetch / iterative_scan** (LAW-146-21); measure; denorm only if p95 regresses. See R13 for large sets. |
| **Proof** | G-146-18,20; recall harness UNCONFIRMED (record, don’t invent) |
| **Wave** | M2 |

## R5 — Missed query arm

| | |
|--|--|
| **Symptom** | One mode/path still passes `allowed_document_ids = None` → workspace-all |
| **Root** | Optional field semantics today = all-pass |
| **Mitigation** | When ABAC on, use required allow-set type (empty vec OK; `None` forbidden). Fail compile/test if any arm drops it |
| **Proof** | Contract tests on all modes + MCP |
| **Wave** | M2–M4 |

## R6 — Unique `(workspace_id, content_hash)`

| | |
|--|--|
| **Symptom** | Same bytes admitted as Public then Secret collide |
| **Root** | Indexed unique hash is workspace-scoped, not classification-scoped |
| **Mitigation** | Explicit replace-with-label-confirm **or** reject second admit; never silent merge (EC-146-23) |
| **Proof** | e2e duplicate admit |
| **Wave** | M1b |

## R7 — M1 too fat

| | |
|--|--|
| **Symptom** | Schema + PDP + list + members + upload UI slips DoD |
| **Root** | Original M1 bundled control-plane UX with storage/PDP |
| **Mitigation** | **M1a** = migrations + authz crate + list/detail/download PEP + quarantine SSOT. **M1b** = members/roles/attrs APIs + SecurityFields UI |
| **Proof** | Separate DoD checklists in [07-implementation-plan.md](07-implementation-plan.md) |
| **Wave** | M1 |

## R8 — Hub description Acc drop

| | |
|--|--|
| **Symptom** | Mix/Local quality drops after authorized-only hub text |
| **Root** | Extract denormalizes multi-doc secrets onto hubs today |
| **Mitigation** | M3 write-path: stop multi-doc denorm; assemble at query. Measure Acc — **UNCONFIRMED**; M5 owns measure-or-defer |
| **Proof** | F-146-32; optional SPEC-001 slice post-M3 |
| **Wave** | M3 / M5 |

## R9 — Community skip vs Mix

| | |
|--|--|
| **Symptom** | Global/Mix quality regresses if community arm disabled |
| **Root** | Mixed-label community reports are unsafe |
| **Mitigation** | v1: skip community inject under ABAC when any classified docs exist (or reports not single-class). Measure later |
| **Proof** | G-146-32 |
| **Wave** | M3 |

## R10 — New crate + OpenAPI

| | |
|--|--|
| **Symptom** | Clippy/workspace/OpenAPI drift |
| **Root** | New `edgequake-authz` member; new routes |
| **Mitigation** | Add to workspace `Cargo.toml`; `cargo clippy -D warnings`; `make codegen-openapi-refresh`; extend spec027 matrix |
| **Proof** | G-146-51 |
| **Wave** | M0 / M5 |

## R11 — Two Settings routes

| | |
|--|--|
| **Symptom** | Members/roles cards land on one page only |
| **Root** | `(dashboard)/settings` and `w/[slug]/settings` |
| **Mitigation** | Shared card components (DRY); wire both entry points |
| **Proof** | Playwright settings on both routes |
| **Wave** | M1b |

## R12 — Task worker identity

| | |
|--|--|
| **Symptom** | Worker inherits ambient admin and can query |
| **Root** | No `Worker` principal kind on claim |
| **Mitigation** | Stamp `PrincipalId::Worker` / `ingestion_service` on claim; PEP denies `query.execute` / `graph.read`; ops: worker DB role no broad SELECT (G6) |
| **Proof** | G-146-53 |
| **Wave** | M1a / M5 |

## R13 — Allow-set scale

| | |
|--|--|
| **Symptom** | Latency / planner cost explode; or underfill when `|allow|` huge or tiny `p` |
| **Root** | Blind `ANY(uuid[])` + HNSW without cardinality strategy |
| **Mitigation** | LAW-146-21: threshold → temp table/JOIN; over-fetch for ANN. M2 owns |
| **Proof** | G-146-17,18; EC-146-36,37 |
| **Wave** | M2 |

## R14 — Ambiguous `policy_version`

| | |
|--|--|
| **Symptom** | Cache invalidation races; audit cannot correlate |
| **Root** | Spec said “max active versions or dedicated counter” |
| **Mitigation** | LAW-146-22: single `workspace_authz_state.policy_generation`; `policy_etag` = label content hash only |
| **Proof** | G-146-55; EC-146-38 |
| **Wave** | M1a |

## R15 — Deny observability

| | |
|--|--|
| **Symptom** | Ops cannot debug “why missing” without leaking existence to users |
| **Root** | Existence-hiding with no server-side trail |
| **Mitigation** | LAW-146-23: `edgequake-audit` / tracing reason codes; never to denied principal |
| **Proof** | G-146-19; EC-146-39 |
| **Wave** | M1a |

## R16 — Cedar per-request cost

| | |
|--|--|
| **Symptom** | Allow-set build slow when many classified docs |
| **Root** | Cedar eval per classified doc per request without cache |
| **Mitigation** | Cache allow-set by `(principal, workspace, policy_generation, attr_hash)` + short TTL; invalidate on generation bump (G4). Still Cedar-only for classified (R3) |
| **Proof** | Unit cache hit/miss; generation bump miss |
| **Wave** | M1a |

## Dependency sketch

```ascii
  R7 (split M1) ──► R1, R2, R3, R14, R15, R16 (M1a) ──► R11, R6 (M1b)
                         │
                         v
                    R5, R4, R13 (M2) ──► R8, R9 (M3) ──► caches/MCP (M4)
                         │
                         v
                    R10, R12 (M0/M5) + Acc measure (M5)
```

## Cross-refs

- Honest assessment → [13-honest-assessment.md](13-honest-assessment.md)  
- Plan → [07-implementation-plan.md](07-implementation-plan.md)  
- Edge cases → [09-edge-cases.md](09-edge-cases.md)  
- Design review → [15-design-review.md](15-design-review.md)  
