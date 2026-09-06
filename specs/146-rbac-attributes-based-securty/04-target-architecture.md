# 04 — Target Architecture (SPEC-146)

## NIST ACM → EdgeQuake

```ascii
  ┌─────────────────────────────────────────────────────────────────┐
  │                        Request / Tool call                      │
  └───────────────────────────────┬─────────────────────────────────┘
                                  │
                                  v
  ┌─────────────── PEP ───────────────────────────────────────────┐
  │  REST handlers · MCP gateway · query entry · ingest admit     │
  │  stamp AuthzContext { principal, subject_attrs,               │
  │                       workspace_id, policy_generation,        │
  │                       env_context }                           │
  └───────────────────────────────┬───────────────────────────────┘
                                  │ decide(action, resource)*
                                  v
  ┌─────────────── PDP ───────────────────────────────────────────┐
  │  edgequake-authz · AllowSetProvider (single)                  │
  │    share_mode workspace|acl|owner_only → SQL / set algebra    │
  │    share_mode classified → Cedar Authorizer (compiled once    │
  │      per policy_generation / published cedar_hash;            │
  │      deny/forbid wins; fail-closed)                           │
  │  LAW-146-18: no dual Cedar↔SQL “equivalent” fork              │
  │  Allow-set cache: (principal, ws, policy_generation,          │
  │                    attr_hash) + short TTL (G4)                │
  └───────────────────────────────┬───────────────────────────────┘
                                  ^
                                  │ entities + attrs
  ┌─────────────── PIP ───────────────────────────────────────────┐
  │  Postgres: users, memberships, principal_attributes,          │
  │  documents security columns, document_acl, attribute catalog, │
  │  policies / policy_versions, workspace_authz_state            │
  │  (policy_generation), optional OIDC claim map                 │
  └───────────────────────────────────────────────────────────────┘
                                  │
                                  v
                    authorized_document_ids (AllowSet)
                    (possibly empty — never "all workspace"
                     when EDGEQUAKE_DOC_ABAC=1)
                    cardinality strategy: LAW-146-21
```

\* Batch decisions for list/ANN use allow-set builder (set algebra), not N remote calls. Denies → audit reason codes (LAW-146-23), never client-visible.

## Crate split (SOLID)

```ascii
  edgequake-auth          identity: JWT, keys, Role, Permission enum
         │
         │  (Permission strings shared)
         v
  edgequake-authz  NEW    Cedar schema, compile, AuthzContext,
                          AuthzDecision trait, AllowSetProvider
         ^
         │  DIP: trait objects / generics
         │
  +------+------+---------------+
  |      |      |               |
  v      v      v               v
 api   query  pipeline      storage
 (PEP) (PEP)  (mint/worker) (SQL predicates)
```

| Crate | Owns | Must not own |
|-------|------|--------------|
| `edgequake-auth` | Credentials, JWT, login Role, `Permission` | Cedar, allow-set SQL |
| `edgequake-authz` | AllowSetProvider, Cedar (classified), AuthzContext | HTTP handlers, AGE Cypher |
| `edgequake-audit` | Break-glass / authz / **deny** event log (reuse) | Policy decide |
| `edgequake-api` | PEPs (incl. **KV list**), members/policy routes | Policy text evaluation |
| `edgequake-query` | Enforce allow-set in modes/hops/caches | Author policy |
| `edgequake-storage` | JOIN/RLS predicates, migrations | Business policy rules |
| `edgequake-pipeline` | Label mint (SQL+KV dual-write), worker principal | Query PDP |

## Request flow (happy path)

```ascii
  1. Authenticate (auth) → PrincipalId (tagged) + role + key kind
     ABAC on ⇒ auth must be on (LAW-146-20); refuse DEV_MODE
  2. Resolve workspace (headers + membership) — LAW-146-1 / LAW-146-20
  3. Load subject attrs + workspace_authz_state.policy_generation (PIP)
  4. Build AuthzContext; attach to request extensions
  5. Capability check: require_permission(action)  // RBAC
     deny → 403 + audit reason_code (LAW-146-23)
  6. AllowSet = authz.documents_for(principal, workspace)
     once at request start (LAW-146-24); cache hit OK if generation matches
     (SQL for workspace/acl/owner_only; Cedar for classified only)
  7. Optional document_filter → AllowSet ∩ filter_ids  (never widen)
  8. Handler / engine uses AllowSet only
     List: filter KV metadata entries (LAW-146-17)
     status_counts: authorized set only when ABAC on
     ANN: over-fetch + filter (LAW-146-21)
  9. Response: existence-hiding; citations from AllowSet
     resource deny → 404 + server-side deny audit (never reason to client)
```

## Enforcement layers (ordered)

```ascii
  P0  HTTP/MCP PEP     after auth, before retrieve/list/graph/parse
  P0b KV list PEP      document_metadata_scan filter (LAW-146-17)
  P1  ANN pre-filter   typed JOIN chunks + document_id IN allow-set
                       + over-fetch / iterative_scan (LAW-146-21)
  P2  Graph hops       source_ids ∩ allow-set (fail-closed)
  P3  KG→chunk         allow-set mandatory when ABAC on (None forbidden)
  P4  Context/cites    safety net strip + title omit
  P5  Caches           principal + policy_generation + fingerprint
  P6  Direct I/O       GET/download/graph/parse-jobs (IDOR bind)
  P7  RLS backstop     PG policies for app + worker roles (defense in depth)
  P8  Deny audit       edgequake-audit reason codes (LAW-146-23)
```

## Query mode enforcement

```ascii
  Bypass ──► no corpus (still capability-gated)
  Naive  ──► chunk ANN pre-filter only
  Local  ──► entity ANN ∩ allow-set → hops gated → KG chunks gated
  Global ──► rel ANN ∩ provenance → community partitioned/skipped
  Hybrid/Mix ──► all arms gated; merge never reintroduces denied ids
```

## Allow-set builder

```ascii
  Inputs: PrincipalId, workspace_id, policy_generation, subject_attrs
  Steps:
    0. If break-glass session active (TTL + scope) → scoped allow-set
       (LAW-146-25); else continue
    1. Candidates = documents in workspace WHERE security_status='ok'
                    AND NOT quarantined
    2. share_mode=workspace  → include if capability document.read   (SQL)
    3. share_mode=owner_only → include if owner matches PrincipalId  (SQL)
    4. share_mode=acl        → include if document_acl grants        (SQL)
    5. share_mode=classified → Cedar evaluate (attrs) only           (Cedar)
    6. Output: HashSet<Uuid>  (empty OK; never None when ABAC on)
       if |set| > ARRAY_THRESHOLD → stage for JOIN/temp (LAW-146-21)
  Cache: short TTL keyed by (principal, workspace, policy_generation, attr_hash)
         invalidate / miss on generation bump (G4)
  Order lock (LAW-146-24): this builder runs BEFORE client document_filter ∩
```

**LAW-146-18:** do **not** maintain a parallel “compiled SQL equivalent” of Cedar. One path per share_mode.

Prefer set SQL with indexes for hot paths (steps 2–4). Cedar for classified + PAP validate on publish.

## ANN over-fetch (LAW-146-21)

```ascii
  Problem: HNSW + WHERE document_id ∈ allow can underfill when p = |allow|/N small
  Mitigations (pick + measure; not mutually exclusive):
    A. Engine over-fetch: LIMIT top_k * factor (cap), then filter to top_k
    B. pgvector 0.8+: SET LOCAL hnsw.iterative_scan = 'relaxed_order'
       (+ max_scan_tuples documented; default 20000 is a bound, not a guarantee)
  Large |allow|: temp table / JOIN instead of huge ANY(uuid[])
  Recall under ACL = UNCONFIRMED until M2/M5
```

## Ingest / worker

```ascii
  Admit API (editor)
       │  SecurityFields required if workspace default classified
       v
  Mint document attrs + ACL in same TX as shell INSERT
  Dual-write security fields into KV {uuid}-metadata (F-146-36)
  Bump workspace policy_generation when labels/ACL affect allow-sets
       │
       ├─ labels OK ──► pending → pipeline as today
       └─ classified missing ──► quarantined / labeling_failed
                                  (not in ANN, not citable; not in KV list as ready)
  Worker principal = PrincipalId::Worker / ingestion_service
       │
       ├── can claim ingest tasks; CANNOT query.execute / graph.read
       └── DB role: no broad SELECT on documents/chunks (G6 ops)
```

## Break-glass (LAW-146-25)

```ascii
  Master / designated break-glass actor
       │  create break_glass_sessions (TTL default 15m, optional doc allow-list)
       │  audit: actor, reason, TTL, scope
       v
  AllowSet = scoped non-quarantined ∩ (allow-list or all-in-ws)
  Expired / revoked → empty for BG path; re-auth required
  ✗ permanent silent all-doc binding
```

## MCP

```ascii
  Tool call
    │  mcp_gateway_auth → same AuthzContext
    │  reject arguments.bypass_acl
    v
  edgequake_search   → AllowSet-bound retrieval
  edgequake_retrieve → same
  edgequake_fetch    → ret_* bound to (PrincipalId, policy_generation)
                       mismatch → 404
```

## Cache key target

```ascii
  keyword:  hash(principal|policy_gen|mode|query|model|lang)
  answer:   hash(principal|policy_gen|mode|full_prompt|effort)
  context:  hash(principal|policy_gen|ws|mode|allow_fp|query|weights)
            (ABAC on ⇒ allow_fp always present; never None)
  ret_*:    value includes PrincipalId + policy_generation; fetch checks both
  allowset: (principal, ws, policy_generation, attr_hash) + short TTL
```

## Deny observability (LAW-146-23)

```ascii
  Client: 404 / empty / 403 (capability) — no reason codes
  Server: edgequake-audit + tracing
          fields: principal, workspace, action, resource_kind,
                  policy_generation, reason_code
          ✗ full attrs dump to denied user
```

## Non-goals (architecture)

```ascii
  ✗ per-principal materialized AGE graphs (v1)
  ✗ OPA sidecar / remote PDP per hop
  ✗ handler-local second ACL
  ✗ client-supplied ids as authority
  ✗ workspace admin ⇒ content.read (without break-glass)
  ✗ dual Cedar↔SQL semantic forks
  ✗ treating RLS as list PEP
  ✗ permanent unbounded break-glass
  ✗ claiming perfect filtered HNSW recall without measure
```

## Cross-refs

- Data model → [05-data-model.md](05-data-model.md)  
- Catalog → [12-role-attribute-catalog.md](12-role-attribute-catalog.md)  
- Embedding/graph lens → [05-lenses/010-embedding-graph.md](05-lenses/010-embedding-graph.md)  
- Plan → [07-implementation-plan.md](07-implementation-plan.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Design review → [15-design-review.md](15-design-review.md)  
