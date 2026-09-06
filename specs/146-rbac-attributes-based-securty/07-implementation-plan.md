# 07 — Implementation Plan (SPEC-146)

Waves M0–M5 with **M1 split into M1a/M1b** (R7). Follow DRY/SOLID ([00-first-principles.md](00-first-principles.md)). See [14-roadblocks.md](14-roadblocks.md).

## Wave overview

```ascii
  M0   Foundations (flag, Cedar schema, role unify)     ── unit
  M1a  Schema + PDP + KV list/detail PEP + quarantine   ── e2e 404/list
       (+ deny audit LAW-146-23; policy_generation)     ── G-146-19/55
  M1b  Members/roles/attrs APIs + upload SecurityFields ── Playwright
  M2   Typed ANN pre-filter + over-fetch / allow-set cap ── e2e ANN
  M3   Provenance-gated hops + community                ── pivot harness
  M4   Caches + MCP + citations + parse IDOR            ── cache/MCP e2e
  M5   Break-glass TTL + OpenAPI + Acc measure          ── GA checklist
```

## M0 — Foundations

**Goal:** Safe scaffolding; no behavior change when flag off.

| Task | Detail |
|------|--------|
| Feature flag | `EDGEQUAKE_DOC_ABAC` default off; when on, **refuse start if auth off** (LAW-146-20) |
| Crate stub | `edgequake-authz` + workspace member; Cedar schema compile smoke |
| Role unify | WebUI `developer`→`user`, `viewer`→`readonly` |
| Permission | Add `DocumentSetLabels`, `PolicyManage`, `McpInvoke`, `BreakGlass`, `ChunkRead`, `GraphRead` — reuse `AuditLogRead` (no duplicate `AuditRead`) |
| PrincipalId | Stub enum `User\|ApiKey\|Master\|Worker` |
| Threat harness scaffold | Ignored until M2+ |
| Docs | Pack + honest assessment + design review |

**DoD:** `cargo test -p edgequake-authz`; role labels match; flag off ⇒ identical behavior; ABAC=1+auth-off fails boot in unit.

## M1a — Schema, PDP, document PEPs (no full Settings UX)

| Task | Detail |
|------|--------|
| Migrations 150+ | security columns, tagged ACL, attrs, roles, policies, **`workspace_authz_state.policy_generation`**, RLS GUC, break_glass_sessions stub |
| Backfill | `share_mode=workspace` |
| Seed | builtin roles + default Cedar templates (classified) |
| AllowSetProvider | SQL for workspace/acl/owner_only; Cedar **only** classified (LAW-146-18); cache by `(principal, ws, policy_generation, attr_hash)` |
| PEPs | list (**KV filter**), detail/download existence-hiding; allow-set **once** then ∩ filter (LAW-146-24) |
| Deny logging | `edgequake-audit` reason codes (LAW-146-23); never expose to client |
| status_counts | authorized-only when ABAC on (override SPEC-084) |
| Dual-write | admit writes security fields to SQL **and** KV metadata; bump `policy_generation` when labels/ACL change |
| Quarantine | `security_status` + phase chip SSOT |
| Wire `require_permission` | document read/create paths |
| Worker principal | claim as Worker; no query; ops note: worker DB role limited (G6) |

**DoD:** Unauthorized GET → 404; list omits unauthorized; counts don’t leak; quarantine not searchable; deny audit present server-side; `policy_generation` monotonic (G-146-55); flag on/off CI green.

## M1b — Members + upload UI

| Task | Detail |
|------|--------|
| Members API | CRUD members + attributes (tagged principals) |
| Roles / attr definitions / policies APIs | PAP surfaces; publish bumps `policy_generation` |
| Upload SecurityFields | API + UI; both Settings routes (R11) |
| Duplicate hash | replace/reject UX (EC-146-23 / R6) |

**DoD:** Playwright upload labels + settings members on both routes.

## M2 — ANN pre-filter + scale

| Task | Detail |
|------|--------|
| Typed chunk search | JOIN `chunks` + allow-set predicate |
| **ANN over-fetch** | `top_k * factor` (capped) and/or `hnsw.iterative_scan=relaxed_order` (LAW-146-21 / G-146-18); **M2 owns** |
| **Allow-set cardinality** | `ANY(uuid[])` below threshold; temp table/JOIN above (G-146-17) |
| Fleet entity/rel | provenance ∩ allow or seed drop |
| `document_filter` | ∩ allow-set **after** allow-set build (LAW-146-24); empty ≠ all-pass when ABAC on |
| Allow-set type | when ABAC on, `None` forbidden (R5) |
| Post-filter | safety net |
| Measure | JOIN vs denorm; filtered recall UNCONFIRMED (R4 / R13) |

**DoD:** e2e unauthorized chunk never in ANN hits; EC-146-03/04/36/37 green; G-146-17/18/20/21 green.

## M3 — Graph provenance

| Task | Detail |
|------|--------|
| Hop gate | `source_ids ∩ allow` fail-closed |
| Hub description | assemble authorized occurrences only |
| Community | skip/partition under ABAC (R9) |
| Graph HTTP | popular/search/neighborhood PEPs |
| Extract write | stop multi-doc secret denorm on hub |

**DoD:** Secret↛Public pivot harness TARGET 0; EC-146-05..07,24 green.

## M4 — Query surfaces hardening

| Task | Detail |
|------|--------|
| Cache keys | principal + **`policy_generation`** (+ allow fp); context never None under ABAC |
| MCP | bind `ret_*` to principal + `policy_generation`; reject `bypass_acl` |
| Citations/SSE | authorized only; no stream-then-redact |
| Parse jobs | bind job → minting principal (IDOR fix) |
| L2 union | cannot widen beyond allow-set |

**DoD:** cross-principal cache miss; stolen `ret_*` → 404; EC-146-08..09,20,25,26,38 green.

## M5 — Break-glass, audit, Acc measure, GA

| Task | Detail |
|------|--------|
| Break-glass | master key + session **TTL (default 15m)** + optional doc allow-list (LAW-146-25); **`edgequake-audit`**; UI banner |
| Auditor role | optional |
| OpenAPI | 401/403/404 matrix + codegen |
| OIDC claim map | subject attrs PIP |
| **Acc measurement** | **M5 owns**: SPEC-001 medical-mid slice under ABAC **or** written deferral with reason (F-146-32 / G8) — measure, don’t invent |
| Filtered ANN recall | record under ACL (UNCONFIRMED) |
| Ops docs | runtime-auth-hardening; worker DB role; break-glass TTL |

**DoD:** EC-146-11/40; G-146-50/54; harness TARGET 0; GA checklist; Acc measured or explicitly deferred.

## Dependency graph

```ascii
  M0 ──► M1a ──► M1b ──► M2 ──► M3 ──► M4 ──► M5
           │       │       │                   │
           │       │       └── over-fetch      └── Acc measure / BG TTL
           │       └── upload UI / members
           └── KV list PEP + deny audit + policy_generation
```

Do not claim “ABAC GA” before M4 DoD.

## SOLID checkpoints per wave

| Wave | Check |
|------|-------|
| M0–M1a | DIP: handlers use trait; LAW-146-18 single AllowSetProvider |
| M1a | DRY: one AllowSet builder; reuse edgequake-audit (incl. deny) |
| M1b | DRY: SecurityFields + Settings cards on both routes |
| M2–M3 | SRP: storage predicates vs authz decide; LAW-146-21 over-fetch |
| M4 | DRY: one cache key compositor (`policy_generation`) |
| M5 | OCP: new actions via Permission (+ Cedar when classified); LAW-146-25 |

## Rollback

Flag off restores pre-146 workspace-only behavior. Migrations retain columns (harmless defaults). Policy tables unused when flag off.

## Cross-refs

- Findings → [01-finding-register.md](01-finding-register.md)  
- E2E → [08-e2e-test-matrix.md](08-e2e-test-matrix.md)  
- Edge cases → [09-edge-cases.md](09-edge-cases.md)  
- Acceptance → [10-acceptance.md](10-acceptance.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  
- Design review → [15-design-review.md](15-design-review.md)  
