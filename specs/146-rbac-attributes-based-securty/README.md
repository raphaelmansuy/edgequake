# SPEC-146 — Document-Level RBAC + ABAC

> **Product pin**: EdgeQuake v0.26.5+ (target cut after funding)  
> **Status**: Spec pack — design review applied (5 Sep 2026)  
> **Folder note**: Keep directory name `146-rbac-attributes-based-securty` (typo preserved for path stability).  
> **Source capture**: [zz-raw.md](zz-raw.md)

> **Inherits**: [SPEC-027](../027-api-edgequake-audit/) auth · [SPEC-101](../101-wizard-mode-tenant-workspace/) workspace · [SPEC-091](../091-simplify-data-layer/) typed fleet · [SPEC-098](../098-data-access-hardening/) RLS/lifecycle · [SPEC-103](../103-llm-cache/) caches · [SPEC-142](../142-precise-links-on-query/) citations  
> **Peers**: [`docs/operations/runtime-auth-hardening.md`](../../docs/operations/runtime-auth-hardening.md) · NIST SP 800-162 ABAC · [SPEC-032 graph](../032-graph/) (hub/occurrence model context)

## One-screen verdict

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│  LAW: Workspace walls are necessary but not sufficient.                      │
│                                                                              │
│  Product path:                                                               │
│    Workspace membership  →  capability RBAC (query/ingest/policy)            │
│    Document attrs + ACL  →  ABAC allow-set (which docs/chunks/edges)         │
│    Cedar in-process PDP  →  one AuthzContext + policy_generation / request   │
│    ANN over-fetch + provenance hops  →  no Secret→Public graph pivot         │
│                                                                              │
│  Fail-closed. Existence-hiding 404. Caches bind principal + policy_generation│
│  Upload-time labels. Quarantine if classified labels missing.                │
│  Deny audit server-side. Break-glass TTL. Allow-set then ∩ filter.           │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Start here

1. [00-why.md](00-why.md) — Five WHYs + causal ASCII  
2. [00-first-principles.md](00-first-principles.md) — LAW-146-1…**25** + SOLID/DRY  
3. [01-finding-register.md](01-finding-register.md) — F-146-*  
4. [02-cross-ref-matrix.md](02-cross-ref-matrix.md) — law ↔ code ↔ test ↔ lens  
5. [03-code-as-is.md](03-code-as-is.md) — auth, upload, query, graph, caches, MCP, UI  
6. [04-target-architecture.md](04-target-architecture.md) — PEP/PDP/PIP  
7. [05-data-model.md](05-data-model.md) — tables, RLS, AGE, Cedar schema  
8. [06-ux-ui-spec.md](06-ux-ui-spec.md) — upload, members, roles, citations  
9. [07-implementation-plan.md](07-implementation-plan.md) — Waves M0–M5 (M1a/M1b) + DoD  
10. [08-e2e-test-matrix.md](08-e2e-test-matrix.md) — gates  
11. [09-edge-cases.md](09-edge-cases.md) — EC-146-*  
12. [10-acceptance.md](10-acceptance.md) — TARGET ACs (no invented Acc)  
13. [11-threat-model.md](11-threat-model.md) — STRIDE + graph channels  
14. [12-role-attribute-catalog.md](12-role-attribute-catalog.md) — roles, perms, attrs  
15. [13-honest-assessment.md](13-honest-assessment.md) — verified vs code  
16. [14-roadblocks.md](14-roadblocks.md) — R1–**R16** mitigations  
17. [15-design-review.md](15-design-review.md) — G1–G8 + scorecards  
18. Lenses → [`05-lenses/`](05-lenses/)  
19. Raw study → [zz-raw.md](zz-raw.md)

## Document map

```ascii
  README
    → 00-why (5 WHY)
    → 00-first-principles (LAW-146-1..25)
    → 01-finding-register (F-146-01..41)
    → 02-cross-ref-matrix
    → 03-code-as-is
    → 04-target-architecture
    → 05-data-model (policy_generation, allow-set cap, BG TTL)
    → 06-ux-ui-spec
    → 07-implementation-plan (M0–M5; M1a/M1b; M2 over-fetch; M5 Acc)
    → 08-e2e-test-matrix (G-146-17..19,54..55)
    → 09-edge-cases (EC-146-01..40)
    → 10-acceptance
    → 11-threat-model
    → 12-role-attribute-catalog
    → 13-honest-assessment
    → 14-roadblocks (R1–R16)
    → 15-design-review (G1–G8)
    → 05-lenses/ (PO, fullstack, DB, UX, front, doc-mgr,
                  security, API, AI, embedding/graph)
    → zz-raw (Ideas Lab capture — KEEP)
```

## Scope (locked)

| In | Out (v1) |
|----|----------|
| Document-level RBAC + ABAC inside workspace walls | Encrypted ANN |
| Role management + attribute catalog + policy store | Per-token ACL |
| Upload-time security labels + quarantine | Perfect image layout redaction |
| Cedar in-process PDP + Postgres PAP | OPA/Rego sidecar PDP |
| ANN pre-filter + over-fetch + provenance-gated hops | Per-principal materialized graphs |
| Cache/citation/MCP/SSE hardening | Raw client Cypher |
| Members API + Settings UX | Invented Acc wins from filters |
| Existence-hiding 404 + empty-state honesty | Replacing workspace isolation |
| Deny audit + break-glass TTL + policy_generation | Permanent unbounded break-glass |

## Locked decisions

1. **Workspace walls stay** — tenant → workspace isolation remains; document policy is additive inside a workspace.  
2. **Allow-set hybrid PDP** — SQL/set algebra for `workspace|acl|owner_only`; **Cedar only for `classified`** (LAW-146-18). No OPA. No dual-evaluator fork.  
3. **RBAC = capability; ABAC = which documents** — workspace admin does **not** imply `document.read` on classified corpora.  
4. **API keys / master / workers are tagged principals** (LAW-146-19) — default key = `query_agent` + viewer; master = break-glass + loud audit via `edgequake-audit`.  
5. **Existence-hiding** — unauthorized resource IDs → 404; capability denials → 403; zero-authz ≡ true-empty copy. Deny reason codes are **server-side only** (LAW-146-23).  
6. **Upload-time labels** — first-class at admit; **dual-write SQL + KV**; classified missing attrs → quarantine.  
7. **Migration honesty** — legacy rows backfill `share_mode=workspace`; no silent tightening.  
8. **One `AuthzContext` + `policy_generation`** — PEPs = REST (incl. KV list), query, workers, MCP; allow-set once then ∩ `document_filter` (LAW-146-24).  
9. **No per-principal materialized graphs in v1** — query-time provenance gates.  
10. **Feature flag** — `EDGEQUAKE_DOC_ABAC` (default off); when on, **auth must be on** (LAW-146-20) and fail-closed.  
11. **List PEP on KV** — RLS never substitutes for Documents list/search (LAW-146-17).  
12. **`status_counts` authorized-only when ABAC on** — overrides SPEC-084 global counts (R2).  
13. **ANN over-fetch + allow-set cardinality bound** (LAW-146-21) — M2 owns.  
14. **Break-glass TTL + optional doc allow-list** (LAW-146-25) — no permanent all-doc binding.  
15. **Acc under ABAC** — measure on SPEC-001 slice in M5 or write an explicit deferral (F-146-32).

## Status board

| ID | Item | Status |
|----|------|--------|
| D1 | Spec pack | **Done** |
| D2 | Engineering channel lock | Done |
| D3 | UX review lock | Done |
| D4 | Honest assessment + roadblocks | **Done** ([13](13-honest-assessment.md), [14](14-roadblocks.md)) |
| D5 | Design review (G1–G8 / LAW-21..25) | **Done** ([15](15-design-review.md)) |
| I1–I5 | Implementation waves M0–M5 (M1a/M1b) | Not started |
| T1 | E2E harness TARGET 0 | Not started |
| A1 | Acceptance | Spec TARGET only |

## Surfaces (blast radius)

| Surface | Role |
|---------|------|
| New crate `edgequake-authz` | AllowSetProvider + Cedar (classified) |
| `edgequake-auth` | Identity; extend `Permission`; unify roles |
| `edgequake-audit` | Reuse for break-glass / authz / deny events |
| `edgequake-api` | PEPs on KV list + SQL detail/query/graph/MCP/members |
| `edgequake-query` | ANN allow-set, hop gates, cache keys |
| `edgequake-storage` | Typed ANN JOIN, RLS backstop, migrations 150+ |
| `edgequake-pipeline` | Label mint dual-write; Worker principal |
| `edgequake_webui` | Upload labels, members/roles/attrs/policies (both Settings routes) |

## Verification (when coded)

```bash
cargo test -p edgequake-authz --lib
cargo test -p edgequake-api --features postgres --test e2e_spec146
cargo test -p edgequake-query --lib spec146
cargo test -p edgequake-storage --features postgres --test e2e_spec146
cd edgequake_webui && pnpm exec playwright test e2e/spec146-
make codegen-openapi-refresh
```

See [08-e2e-test-matrix.md](08-e2e-test-matrix.md).
