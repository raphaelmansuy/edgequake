# 09 — Edge Cases (SPEC-146)

Each EC has mitigation + test gate. Status: **OPEN** until wave lands.

| ID | Case | Mitigation | Gate | Wave |
|----|------|------------|------|------|
| EC-146-01 | Empty allow-set ≡ empty corpus copy | Same UX string; no hidden-count | G-146-11, Playwright empty | M1 |
| EC-146-02 | Unauthorized UUID enumeration | 404 not 403; same copy as missing | G-146-10 | M1 |
| EC-146-03 | `document_filter.ids` ⊃ allow-set | Intersect; never widen | G-146-21 | M2 |
| EC-146-04 | Typed ANN ignores document_ids (regression) | JOIN pre-filter mandatory when ABAC on | G-146-20 | M2 |
| EC-146-05 | Shared entity Secret+Public | Provenance-gated hops; authorized hub text | G-146-30 | M3 |
| EC-146-06 | Community report mixed labels | Skip/partition under ABAC | G-146-32 | M3 |
| EC-146-07 | Popular-node degree side channel | Authorized degree or empty | G-146-33 | M3 |
| EC-146-08 | Keyword/answer/context cache cross-user | Key principal + policy_generation | G-146-40 | M4 |
| EC-146-09 | MCP stolen `ret_*` | Bind principal + policy_generation; 404 | G-146-41 | M4 |
| EC-146-10 | Ingestion worker queries corpus | `ingestion_service` lacks query.execute | G-146-53 | M1/M5 |
| EC-146-11 | Master key silent all-doc | Break-glass + audit + banner + TTL | G-146-50,54 | M5 |
| EC-146-12 | Labeling failure on classified mint | Quarantine; not searchable/citable | G-146-12 | M1 |
| EC-146-13 | Deleted doc residual embeddings | Cascade + tombstone; no ANN hit | storage delete e2e | M2–M4 |
| EC-146-14 | Policy generation bump | Invalidate authz/LLM caches | G-146-40,55 | M4 |
| EC-146-15 | Missing classified attrs | Fail-closed deny / quarantine on mint | G-146-12 | M1 |
| EC-146-16 | Auth off / DEV_MODE + ABAC on | **Refuse start** (LAW-146-20 / EC-146-35) | G-146-00 | M0 |
| EC-146-17 | Strict tenant bind off | ABAC verifies membership; no header-only trust | e2e | M1 |
| EC-146-18 | JWT default workspace ≠ header workspace | Resolve membership for header scope; mismatch → 403 if strict / documented | e2e | M1 |
| EC-146-19 | Vision/MM assets | Inherit parent document labels | unit + e2e | M1 |
| EC-146-20 | Figure/page titles in citations | Titles only if parent doc authorized | G-146-42 | M4 |
| EC-146-21 | Reprocess drops/widens labels | Preserve labels; widen needs set_labels | e2e reprocess | M1 |
| EC-146-22 | Batch upload mixed labels | Per-doc security; shared defaults overridable | Playwright batch | M1 |
| EC-146-23 | Duplicate content_hash across classifications | Explicit replace confirm or reject; no silent merge | e2e | M1 |
| EC-146-24 | `/graph/labels/popular` leak | Authorized topology only | G-146-33 | M3 |
| EC-146-25 | Parse jobs by UUID unauthenticated | Require auth; bind job to principal | G-146-44 | M4 |
| EC-146-26 | SSE stream-then-redact | Filter before enqueue | G-146-43 | M4 |
| EC-146-27 | Timing 403 vs 404 | Best-effort same path cost for resource IDs | optional soak | M5 |
| EC-146-28 | Prompt injection “ignore ACL” | PEP before LLM; allow-set immutable in request | harness | M4 |
| EC-146-29 | Stale OIDC groups | PIP refresh on login/refresh; TTL on attr cache | e2e oidc | M5 |
| EC-146-30 | Delete last workspace owner | Forbid or force transfer | e2e members | M1b |
| EC-146-31 | ACL principal deleted / key revoked | ACL rows cascade or deny; allow-set rebuild | e2e | M1a |
| EC-146-32 | Legacy backfill workspace mode | Migration honesty; members with document.read see legacy | G-146-90 + migrate test | M1a |
| EC-146-33 | `status_counts` leak unauthorized size | Authorized-only counts when ABAC on (override SPEC-084) | G-146-16 | M1a |
| EC-146-34 | SQL-only labels; KV list unlabeled | Dual-write security fields to KV at admit | G-146-12 | M1a |
| EC-146-35 | ABAC=1 with auth/DEV_MODE off | Refuse process start (LAW-146-20) | G-146-00 | M0 |
| EC-146-36 | Allow-set too large for `ANY(uuid[])` | Cap + temp table / JOIN strategy (LAW-146-21) | G-146-17 | M2 |
| EC-146-37 | ANN underfill under sparse allow-set | Over-fetch / iterative_scan; measure recall (LAW-146-21) | G-146-18 | M2 |
| EC-146-38 | Stale cache after concurrent policy/ACL change | Stamp + key by monotonic `policy_generation` (LAW-146-22) | G-146-55 | M1a/M4 |
| EC-146-39 | Deny reason leaked to denied principal | Audit/trace only; client 404/empty/403 (LAW-146-23) | G-146-19 | M1a |
| EC-146-40 | Break-glass after TTL / revoke | Session expire → deny; no permanent all-doc (LAW-146-25) | G-146-54 | M5 |

## Detail sketches (high risk)

### EC-146-05 — Shared hub

```ascii
  AllowSet = {DocA}
  Entity X ← occ(A), occ(B)
  Hop edge(B) ── DROP (source_ids ∩ allow = ∅)
  Description(X) ── text from occ(A) only
```

### EC-146-03 — Filter intersection

```ascii
  Client ids = {A, B, C}
  AllowSet   = {A, B}
  Effective  = {A, B}   // never add D from workspace
  Client ids = {} when ABAC on → AllowSet (not workspace-all)
  allowed_document_ids = None when ABAC on → BUG (R5)
```

### EC-146-33 — status_counts

```ascii
  ABAC off: counts global (SPEC-084)
  ABAC on:  counts over authorized docs only
            ✗ never show Secret pending count to Public viewer
```

### EC-146-35 — ABAC + auth-off

```ascii
  EDGEQUAKE_DOC_ABAC=1 AND (auth disabled | DEV_MODE)
    → refuse start with loud error
    ✗ no Option B silent workspace-all
```

### EC-146-36 / EC-146-37 — Scale & ANN underfill

```ascii
  |allow| huge ──► temp table / JOIN (not giant ANY)
  p = |allow|/N small ──► overfetch_k or iterative_scan
  recall measured UNCONFIRMED (do not invent)
```

### EC-146-39 — Deny observability

```ascii
  Client: 404 / empty / 403
  Server: audit(reason_code, policy_generation, …)
  ✗ never return reason_code to denied principal
```

### EC-146-40 — Break-glass TTL

```ascii
  BG session expires_at < now() ──► deny / re-auth
  ✗ permanent master = all non-quarantined forever
```

## Cross-refs

- Threat model → [11-threat-model.md](11-threat-model.md)  
- E2E → [08-e2e-test-matrix.md](08-e2e-test-matrix.md)  
- Findings → [01-finding-register.md](01-finding-register.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Design review → [15-design-review.md](15-design-review.md)  
