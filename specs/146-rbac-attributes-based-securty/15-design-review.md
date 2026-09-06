# 15 — Design Review (SPEC-146)

First-principles + AI-engineering review of the SPEC-146 pack after the honest-update pass (2026-09-05). Spec-only.

## Verdict

The design is **sound for v1 GraphRAG document ABAC**: workspace walls stay, fail-closed allow-set, KV list PEP, tagged principals, typed ANN pre-filter, provenance-gated hops, principal-bound caches. Remaining gaps are **scale, observability, and operational bounds** — not a rewrite of the architecture.

```ascii
  Strengths (keep)                    Gaps (this review)
  ─────────────────                   ──────────────────
  LAW-1..20 fail-closed path          G1 allow-set / ANN underfill
  KV list PEP (17)                    G2 policy_generation ambiguous
  Tagged PrincipalId (19)             G3 deny not observable
  Cedar only classified (18)          G4 Cedar per-request cost
  M1a/M1b + R1–R12                    G5 filter ∩ allow-set order
                                      G6 worker SQL bypass
                                      G7 break-glass unbounded
                                      G8 Acc measure unscheduled
```

## First-principles scorecard

| Principle | Score | Notes |
|-----------|-------|-------|
| Single authority (one AllowSetProvider) | Strong | LAW-146-18 |
| Enforce at real SSOT | Strong | KV list + SQL ANN + provenance hops |
| Fail closed | Strong | Empty allow-set; ABAC requires auth |
| Existence-hiding | Strong | 404 / empty copy |
| DRY / SOLID crate split | Strong | auth / authz / audit |
| Observable deny (ops) | Weak → fix | G3 / LAW-146-23 |
| Bounded blast radius (BG, allow-set size) | Weak → fix | G1, G7 / LAW-146-21,25 |
| Cache coherence (generation) | Weak → fix | G2 / LAW-146-22 |
| AI quality honesty | Medium | Acc deferred; schedule in M5 (G8) |

## AI-engineering scorecard

| Concern | Assessment |
|---------|------------|
| LLM never PDP | Pass — PEP before retrieve/prompt |
| Prompt injection cannot widen allow-set | Pass if LAW-146-24 locked |
| ANN pre-filter (not post-only) | Pass design; underfill risk G1 |
| Hub secrets not denormalized | Pass (LAW-146-10); Acc UNCONFIRMED |
| Cache cross-principal | Pass if keys + generation correct |
| Agent/MCP confused deputy | Pass if Worker + ret_* bind hold |
| Measure quality under ACL | Gap — schedule M5 (G8) |

## Gaps G1–G8

### G1 — Allow-set scale / ANN underfill

`document_id = ANY($allow)` + HNSW can underfill when authorized fraction `p` is small. Large `|allow|` also costs.

**Lock LAW-146-21:** over-fetch (`top_k * factor`, capped) then filter; switch to temp table / bitmap when `|allow|` exceeds threshold. Measure recall UNCONFIRMED.

### G2 — `policy_version` ambiguous

“Max active versions or dedicated counter” is not implementable as one contract.

**Lock LAW-146-22:** monotonic `policy_generation` per workspace; bump on any allow-set-affecting change. `policy_etag` on documents = content hash only.

### G3 — Deny not observable

Users get 404 (correct). Operators cannot debug without a server-side trail.

**Lock LAW-146-23:** audit/trace deny with principal, workspace, action, resource kind, `policy_generation`, reason code. Never return reason to denied principal.

### G4 — Cedar per-request cost

Many classified docs ⇒ many Cedar evals on allow-set build.

**Mitigation:** cache allow-set by `(principal, workspace, policy_generation, attr_hash)`; short TTL; invalidate on generation bump. Cedar only on classified subset.

### G5 — Filter ∩ allow-set order

If allow-set were rebuilt after trusting client ids, scope could widen.

**Lock LAW-146-24:** compute allow-set once at request start; `document_filter` is pure intersection afterward.

### G6 — Worker SQL bypass

PEP denies query for Worker; DB role could still SELECT.

**Mitigation (ops):** worker DB role limited; RLS applies to workers. Document in ops + M1a DoD note.

### G7 — Break-glass unbounded

Master → all non-quarantined is too wide for standing keys.

**Lock LAW-146-25:** break-glass session TTL (default 15m) + optional doc allow-list; audit records TTL/scope; no permanent BG binding.

### G8 — Acc measurement unscheduled

F-146-32 says measure; no wave owns it.

**Patch:** M5 owns SPEC-001 medical-mid slice under ABAC **or** explicit written deferral.

## Normative deltas (summary)

| ID | Statement |
|----|-----------|
| LAW-146-21 | ANN over-fetch + filter; allow-set cardinality strategy |
| LAW-146-22 | Monotonic `policy_generation` per workspace |
| LAW-146-23 | Deny observability without user leakage |
| LAW-146-24 | Allow-set then intersect filter (order) |
| LAW-146-25 | Break-glass TTL + scoped allow-list |

## Gate ID remapping

Design-review plan suggested G-146-20/21 for break-glass TTL / `policy_generation`. Those IDs were already assigned to ANN JOIN / `document_filter` ∩. Pack uses:

| Gate | Meaning |
|------|---------|
| G-146-17 | Allow-set cardinality |
| G-146-18 | ANN over-fetch |
| G-146-19 | Deny audit |
| G-146-20 | Typed ANN JOIN (existing) |
| G-146-21 | Filter ∩ allow-set (existing; order clarified) |
| G-146-54 | Break-glass TTL |
| G-146-55 | `policy_generation` monotonic |

## What we still will not claim

```ascii
  ✗ Acc flat/improved under ABAC without measurement
  ✗ Perfect HNSW filtered recall for tiny p
  ✗ Timing side-channel immunity
  ✗ RLS alone as list PEP
```

## Cross-refs

- Laws → [00-first-principles.md](00-first-principles.md)  
- Architecture → [04-target-architecture.md](04-target-architecture.md)  
- Data model → [05-data-model.md](05-data-model.md)  
- Honest assessment → [13-honest-assessment.md](13-honest-assessment.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Plan → [07-implementation-plan.md](07-implementation-plan.md)  
