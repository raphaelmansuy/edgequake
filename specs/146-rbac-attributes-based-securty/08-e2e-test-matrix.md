# 08 — E2E Test Matrix (SPEC-146)

Every F-146-* / EC-146-* maps to at least one gate. Naming: `spec146_*` (Rust), `e2e/spec146-*.spec.ts` (Playwright).

## Commands (target)

```bash
# Unit / lib
cargo test -p edgequake-authz --lib
cargo test -p edgequake-auth --lib rbac
cargo test -p edgequake-query --lib spec146
cargo test -p edgequake-storage --lib spec146

# Postgres e2e
cargo test -p edgequake-api --features postgres --test e2e_spec146_document_pep
cargo test -p edgequake-api --features postgres --test e2e_spec146_members_policies
cargo test -p edgequake-api --features postgres --test e2e_spec146_query_allowset
cargo test -p edgequake-api --features postgres --test e2e_spec146_mcp_ret_bind
cargo test -p edgequake-storage --features postgres --test e2e_spec146_ann_prefilter
cargo test -p edgequake-storage --features postgres --test e2e_spec146_graph_provenance
cargo test -p edgequake-query --features postgres --test e2e_spec146_cache_isolation
cargo test -p edgequake-api --features postgres --test e2e_spec146_break_glass_audit

# Harness TARGET 0
cargo test -p edgequake-api --features postgres --test e2e_spec146_unauthorized_context_rate

# Playwright
cd edgequake_webui
pnpm exec playwright test e2e/spec146-

# OpenAPI
make codegen-openapi-refresh
cargo test -p edgequake-api --test spec027_api_contract  # extend matrix
```

## Gate table

| Gate ID | Wave | Covers | Type |
|---------|------|--------|------|
| G-146-00 | M0 | Cedar schema compile; flag off noop; ABAC+auth-off refuse boot | unit |
| G-146-01 | M0 | Role vocabulary WebUI↔API | unit + Playwright |
| G-146-10 | M1a | Unauthorized GET → 404 | e2e |
| G-146-11 | M1a | List omits unauthorized titles (KV PEP) | e2e |
| G-146-12 | M1a/M1b | Upload labels dual-write; classified quarantine | e2e |
| G-146-13 | M1b | Members CRUD + attrs | e2e |
| G-146-14 | M1a | `require_permission` on ingest/policy | e2e |
| G-146-15 | M1a | RLS backstop empty allow (SQL paths) | e2e |
| G-146-16 | M1a | status_counts authorized-only under ABAC | e2e |
| G-146-17 | M2 | Allow-set cardinality strategy (`ANY` vs temp/JOIN) | e2e |
| G-146-18 | M2 | ANN over-fetch / iterative_scan under ABAC | e2e |
| G-146-19 | M1a | Deny audit/trace reason_code; not exposed to client | e2e |
| G-146-20 | M2 | Typed ANN JOIN pre-filter | e2e |
| G-146-21 | M2 | `document_filter` ∩ allow-set; None forbidden; order = allow then ∩ | e2e |
| G-146-22 | M2 | Post-filter safety net still drops stragglers | unit |
| G-146-30 | M3 | Secret↛Public hop pivot TARGET 0 | harness |
| G-146-31 | M3 | Hub description authorized-only | e2e |
| G-146-32 | M3 | Community skipped/partitioned | e2e |
| G-146-33 | M3 | Popular labels no leak | e2e |
| G-146-40 | M4 | Keyword/answer/context cache isolation (`policy_generation`) | e2e |
| G-146-41 | M4 | MCP `ret_*` bound; `bypass_acl` rejected | e2e |
| G-146-42 | M4 | Citations omit unauthorized titles | e2e |
| G-146-43 | M4 | SSE never stream-then-redact | e2e |
| G-146-44 | M4 | Parse job GET bound to minting principal (IDOR) | e2e |
| G-146-50 | M5 | Break-glass audit row (`edgequake-audit`) | e2e |
| G-146-51 | M5 | OpenAPI 401/403/404 matrix | contract |
| G-146-52 | M5 | Unauthorized context rate TARGET 0 | harness |
| G-146-53 | M5 | Ingestion_service / Worker cannot query | e2e |
| G-146-54 | M5 | Break-glass TTL + optional doc allow-list (LAW-146-25) | e2e |
| G-146-55 | M1a | `policy_generation` monotonic; cache keys bind it | unit + e2e |
| G-146-90 | all | Flag off regression (workspace-only) | e2e |

> **Gate ID note:** Design-review plan named G-146-20/21 for BG TTL / generation; those IDs were already ANN / filter ∩. New gates use **G-146-17..19** and **G-146-54..55** to avoid collision.

## Playwright scenarios

| Spec file | Scenario |
|-----------|----------|
| `spec146-upload-labels.spec.ts` | SecurityFields → chip on table |
| `spec146-existence-hiding.spec.ts` | Viewer cannot see Secret row; direct URL → not found |
| `spec146-quarantine.spec.ts` | Classified missing attrs → chip + retry |
| `spec146-query-empty.spec.ts` | Zero-authz copy = empty copy |
| `spec146-citations.spec.ts` | No Restricted/grey titles |
| `spec146-settings-members.spec.ts` | Members/roles/attrs happy path |
| `spec146-break-glass.spec.ts` | Banner + audit + TTL expiry (admin) |

## Harness fixture (pivot)

```ascii
  Workspace W
    Doc A Public  — text mentions ENTITY_X
    Doc B Secret  — text mentions ENTITY_X + SECRET_TOKEN
  Principal P — clearance internal, document.read on Public only

  Query Mix: "Tell me about ENTITY_X"
  Assert:
    context has no SECRET_TOKEN
    citations have no Doc B title
    graph neighborhood edges from B absent
```

## Flag matrix

| `EDGEQUAKE_DOC_ABAC` | Expect |
|----------------------|--------|
| `0` / unset | Pre-146: workspace list all; no Cedar path |
| `1` | Full PEPs; fail-closed |

## Cross-refs

- Edge cases → [09-edge-cases.md](09-edge-cases.md)  
- Plan → [07-implementation-plan.md](07-implementation-plan.md)  
- Acceptance → [10-acceptance.md](10-acceptance.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  
- Design review → [15-design-review.md](15-design-review.md)  
