# 10 — Acceptance (SPEC-146)

TARGET criteria only. No invented Acc / latency numbers.

## Product acceptance

| # | Criterion | Evidence |
|---|-----------|----------|
| A1 | Workspace walls remain; ABAC additive inside workspace | LAW-146-1 tests; membership required |
| A2 | Unauthorized chunk never in LLM context | Harness TARGET 0 (G-146-52) |
| A3 | Unauthorized title never in list, autocomplete, citations, SSE | G-146-11,42,43 + Playwright |
| A4 | Graph hop cannot pivot Secret→Public | G-146-30 |
| A5 | Cache miss across principals / policy_generation | G-146-40 |
| A6 | Ingestion service cannot query | G-146-53 |
| A7 | Break-glass audited + banner + TTL | G-146-50,54 |
| A8 | RLS denies direct SQL bypass for app role | G-146-15 |
| A9 | Upload-time labels; quarantine not searchable | G-146-12 |
| A10 | Legacy docs `share_mode=workspace` (no silent tighten) | migrate test + G-146-90 |
| A11 | Existence-hiding 404 for resource IDs; 403 for capability | G-146-10,14,51 |
| A12 | Zero-authz empty indistinguishable from true empty | Playwright empty |
| A13 | Client `document_filter` cannot widen allow-set (allow then ∩) | G-146-21 |
| A14 | Typed ANN pre-filters by document allow-set | G-146-20 |
| A15 | MCP `ret_*` bound; `bypass_acl` rejected | G-146-41 |
| A16 | Role vocabulary unified (WebUI ↔ API) | G-146-01 |
| A17 | Feature flag off preserves pre-146 behavior | G-146-90 |
| A18 | OpenAPI documents 401/403/404 matrix | G-146-51 |
| A19 | List/search PEP on KV path (not RLS-only) | G-146-11 |
| A20 | `status_counts` authorized-only when ABAC on | G-146-16 |
| A21 | ABAC=1 refuses boot if auth off | G-146-00 |
| A22 | Labels dual-written to SQL + KV | G-146-12 |
| A23 | Parse job IDOR fixed (bound to minting principal) | G-146-44 |
| A24 | Break-glass uses `edgequake-audit` | G-146-50 |
| A25 | Allow-set cardinality strategy when large | G-146-17 |
| A26 | ANN over-fetch / iterative_scan when ABAC on | G-146-18 |
| A27 | Deny reason codes audited, never returned to denied principal | G-146-19 |
| A28 | `policy_generation` monotonic; caches bind it | G-146-55 |
| A29 | Acc under ABAC measured on SPEC-001 slice **or** written deferral | M5 DoD / F-146-32 |

## Acc measurement deferral (F-146-32 / M5)

**Status (this cut):** SPEC-001 Acc measurement under ABAC is **explicitly deferred**.

| Item | Decision |
|------|----------|
| SPEC-001 medical-mid Acc under ABAC | **Deferred** — not measured in this M5 cut |
| Invented Acc / ΔAcc numbers | **Forbidden** — do not invent or publish |
| Quality gate for this cut | **G-146-52** unauthorized-context harness **TARGET 0** (SECRET_TOKEN absent from LLM context after allow-set ∩) |
| When Acc will be measured | Post-GA / follow-up wave on a SPEC-001 slice with ABAC on; record results then — never backfill fiction |

A29 remains open until either (a) Acc is measured on SPEC-001 under ABAC, or (b) this deferral stays the accepted M5 DoD (chosen here).

## Explicit non-acceptance (v1)

| Claim | Status |
|-------|--------|
| Acc improved due to ABAC | **UNCONFIRMED** — do not accept; Acc deferred this cut (F-146-32) |
| Encrypted ANN | Out of scope |
| Perfect timing side-channel immunity | Best-effort only |
| OPA PDP | Non-goal |
| Per-principal materialized graphs | Non-goal |
| “RLS alone secures the Documents list” | **False** — reject |
| Dual Cedar↔SQL equivalent evaluators | **Forbidden** (LAW-146-18) |
| Perfect filtered HNSW recall for tiny `p` | **UNCONFIRMED** — measure (LAW-146-21) |
| Permanent unbounded break-glass | **Forbidden** (LAW-146-25) |

## GA checklist

```ascii
  [ ] M0–M4 DoD complete (M1a + M1b)
  [ ] M5 break-glass TTL + OpenAPI
  [ ] All G-146-* gates green in CI (incl. 17–19, 54–55)
  [ ] Playwright spec146-* green
  [x] Unauthorized context harness TARGET 0 (G-146-52) — M5 quality gate
  [ ] Ops docs updated (runtime-auth-hardening; worker DB role)
  [x] Acc impact explicitly deferred (F-146-32) — no invented Acc numbers
  [ ] Honest assessment gaps closed (KV PEP, PrincipalId, dual-write)
  [ ] Design-review laws LAW-146-21..25 implemented
  [ ] Engineering + Security sign-off
```

## Sign-off

| Role | Name | Date | Result |
|------|------|------|--------|
| Product Owner | | | |
| Security | | | |
| Full Stack | | | |
| Database | | | |

## Cross-refs

- Plan → [07-implementation-plan.md](07-implementation-plan.md)  
- E2E → [08-e2e-test-matrix.md](08-e2e-test-matrix.md)  
- Threat → [11-threat-model.md](11-threat-model.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Design review → [15-design-review.md](15-design-review.md)  
