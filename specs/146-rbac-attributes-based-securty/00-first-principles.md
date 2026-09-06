# 00 — First Principles (SPEC-146)

## Axioms

1. **Authorization is not retrieval convenience** — `document_filter` is never an ACL.  
2. **Fail closed** — missing attrs, missing policy, PDP error, empty allow-set → deny / empty, never workspace-all.  
3. **One decision path** — one PDP, one `AuthzContext`, one monotonic `policy_generation` stamped at request start.  
4. **Pre-filter beats post-filter** — ANN and hops enforce allow-set; post-filter is a safety net only.  
5. **Secrets live on provenance** — entity hubs are not classified; occurrences/edges carry `source_ids`.  
6. **Labels mint at admit** — classified share modes without required attrs → quarantine, not searchable.  
7. **Existence is a secret** — unauthorized IDs → 404; empty authz ≡ empty corpus copy. Ops may observe deny reasons server-side only.  
8. **Principals, not ambient authority** — JWT, API keys, workers, MCP inherit explicit principal kind.  
9. **Evidence beats vibes** — every F-146-* and EC-146-* maps to a unit, Rust e2e, or Playwright gate.  
10. **DRY / SOLID** — one authz crate; PEPs thin; no handler-local ACL copies.  
11. **Bounded authority** — allow-set cardinality, ANN over-fetch, and break-glass TTL are first-class limits, not afterthoughts.

## Laws

| Law | Statement |
|-----|-----------|
| **LAW-146-1** | Workspace walls remain: principal must satisfy tenant/workspace membership (or workspace-scoped key) **before** document ABAC. Document policy never replaces workspace isolation. |
| **LAW-146-2** | Policy language for **classified** share mode is Cedar in-process (`cedar-policy`). Compile `PolicySet` once per `policy_generation` (or per published Cedar hash when generation bumps). No OPA sidecar. No remote PDP round-trip per graph hop. Non-classified share modes use SQL/set algebra (LAW-146-18) — Cedar is not a second evaluator for those modes. |
| **LAW-146-3** | RBAC answers capability (`query.execute`, `document.create`, `policy.manage`, …). ABAC answers which documents/chunks/edges/titles. Workspace admin does **not** imply content `document.read` on classified corpora without break-glass. |
| **LAW-146-4** | One `AuthzContext` + `policy_generation` per request. PEPs = REST, query engine, workers, MCP. No second allow-set in prompt builders or handler-local ACL. |
| **LAW-146-5** | Client `document_filter` (ids/dates/pattern) is intersected with PDP allow-set. Intersection never widens. Empty filter ≠ all-pass when ABAC is on. Order: allow-set first, then ∩ filter (LAW-146-24). |
| **LAW-146-6** | Existence-hiding: unauthorized document/chunk/graph resource IDs → **404** + “Document not found”. Capability denials (ingest, policy, workspace manage) → **403**. |
| **LAW-146-7** | Zero authorized hits ≡ true-empty UX: “No matching results.” Never “N hidden”, “0 of N authorized”, or greyed Restricted titles. |
| **LAW-146-8** | Upload-time labels are first-class (classification, project, export_control, pii, share_mode, ACL). Not only `custom_metadata`. Classified missing required attrs → `security_status=quarantined` / `labeling_failed`; not searchable, not citable. |
| **LAW-146-9** | ANN pre-filter is mandatory on the typed path: `workspace_id` **and** `document_id ∈ allow-set` (JOIN `chunks` or denorm). Post-filter alone is insufficient. |
| **LAW-146-10** | Graph hops are provenance-gated: drop edge/occurrence unless `source_ids` intersects allow-set (fail-closed). Hub `description` assembled only from authorized occurrences. Ban label-union on hubs. |
| **LAW-146-11** | Community/global summaries must not mix classifications. Partition by authz set or disable for classified workspaces. Popular-node degree must not leak unauthorized topology. |
| **LAW-146-12** | Caches (keyword, answer, context, MCP `ret_*`, `llm_cache`) key by principal (or attr hash) + `policy_generation` + mode (+ allow-set fingerprint where applicable). Cross-principal reuse forbidden. |
| **LAW-146-13** | API keys are principals. Default minted key = workspace viewer / `query_agent`. `ingestion_service` cannot query. Master/env key = break-glass + loud audit — never silent all-doc Mix. Break-glass is time-boxed (LAW-146-25). |
| **LAW-146-14** | Legacy documents backfill `share_mode=workspace` (today’s behavior). No silent tightening. Operators opt into `acl` / `classified` / `owner_only`. |
| **LAW-146-15** | Role vocabulary is unified: API `admin`/`user`/`readonly`; membership `owner`/`admin`/`member`/`readonly`; WebUI labels map 1:1 (retire `developer`/`viewer` as distinct API values). Extend existing `Permission` enum — do not invent a parallel matrix. |
| **LAW-146-16** | Feature flag `EDGEQUAKE_DOC_ABAC`. When enabled, fail-closed end-to-end. When disabled, preserve pre-146 workspace-only behavior. CI proves both modes. |
| **LAW-146-17** | Document list/search/autocomplete PEPs run on the **KV metadata scan** path (`document_metadata_scan` + staging merge) and on SQL for detail/download. Postgres RLS on `documents` never substitutes for the list PEP. |
| **LAW-146-18** | Allow-set builder: SQL/set algebra for `share_mode ∈ {workspace, acl, owner_only}`; Cedar evaluates **only** `classified` documents. One `AllowSetProvider` — no dual Cedar↔SQL semantic forks. |
| **LAW-146-19** | Principals are tagged IDs: `User(Uuid) \| ApiKey(Uuid) \| Master \| Worker`. Master (`"master-api-key"`) and workers are **not** `users.user_id` UUIDs. ACL/bindings store kind + id, not UUID-only FK. |
| **LAW-146-20** | When `EDGEQUAKE_DOC_ABAC=1`, authentication **must** be on (refuse process start if DEV_MODE / auth-off). Workspace membership is verified even if `EDGEQUAKE_STRICT_TENANT_BIND` is false. |
| **LAW-146-21** | When ABAC is on, typed ANN **must** use over-fetch + filter: request `min(top_k * overfetch_factor, overfetch_cap)` candidates (or enable pgvector `hnsw.iterative_scan=relaxed_order` with documented `max_scan_tuples`), then retain only `document_id ∈ allow-set` until `top_k` or exhaustion. Allow-set cardinality is bounded: use `ANY(uuid[])` below a documented threshold; above it, use temp table / bitmap / JOIN strategy. Filtered ANN recall under ACL is **UNCONFIRMED** until measured. |
| **LAW-146-22** | `policy_generation` is a single monotonic `BIGINT` per workspace, incremented on any change that can alter allow-sets (policy publish, ACL grant/revoke, principal attr change, document security columns/share_mode that affect authz, quarantine transitions). `AuthzContext.policy_generation` stamps that value at request start. Document `policy_etag` is a **content hash of labels**, not the generation counter. |
| **LAW-146-23** | Deny decisions are logged to `edgequake-audit` (and/or structured tracing) with: principal kind+id, workspace_id, action, resource kind (+ resource id when known), `policy_generation`, and a stable **reason code** (e.g. `not_in_allow_set`, `capability_denied`, `quarantined`, `cedar_forbid`). Never return deny reason, attrs, or existence hints to the denied principal (LAW-146-6/7 still hold for clients). |
| **LAW-146-24** | Allow-set is computed **once** at request start (after workspace resolve). Client `document_filter` is applied only as a pure intersection afterward. No path may compute or enlarge allow-set after reading client filter ids. |
| **LAW-146-25** | Break-glass sessions have a TTL (default **15 minutes**, configurable) and an optional document allow-list. Audit row records TTL, scope, actor, and reason. No permanent break-glass binding that silently equals “all non-quarantined forever.” Expired session → deny / re-auth; not silent widen. |

## DRY / SOLID

| Principle | Application |
|-----------|-------------|
| **DRY** | One allow-set builder; one existence-hiding helper; one cache-key compositor; one upload SecurityFields component shared by text/file/PDF/batch; one audit logger (`edgequake-audit`). |
| **SRP** | `edgequake-auth` = identity; `edgequake-authz` = decide; handlers = PEP only; storage = enforce predicates; UI = collect attrs. |
| **OCP** | New action = extend `Permission` (+ Cedar schema when classified); PEPs unchanged. New share_mode = catalog + allow-set branch, not scattered ifs. |
| **LSP** | Memory/dev and Postgres authz share the same `AuthzDecision` trait shape. |
| **ISP** | Query engine depends on allow-set + decide trait — not Cedar entity types. |
| **DIP** | API/query/pipeline depend on `AuthzDecision` / `AllowSetProvider` abstractions; Cedar is an implementation detail of `edgequake-authz`. |

## Inheritance (do not break)

| Prior | Constraint |
|-------|------------|
| SPEC-027 | JWT / API key / OIDC / master key paths stay; identity SSOT = Postgres |
| SPEC-032 / 101 | Tenant → workspace ladder; wizard/onboarding unchanged |
| SPEC-091 | Typed fleet / `chunk_embeddings` remain production ANN path |
| SPEC-098 | Document lifecycle statuses + delete dual-SSOT remain |
| SPEC-103 | LLM caches remain; keys gain principal + policy_generation |
| SPEC-142 | Citation deeplinks remain; titles only from authorized set |
| RLS 009/096 | Workspace RLS remains; document ABAC adds backstop, does not remove |
| SPEC-084 / GH-319 | `status_counts` global **overridden** when ABAC on → authorized-only counts (R2) |
| `edgequake-audit` | Reuse for break-glass / authz / deny events; extend types, do not fork |

## Non-goals (v1) — LAW reminders

```ascii
  OUT: encrypted ANN | per-token ACL | image redaction perfection
  OUT: OPA sidecar | remote PDP per hop | per-principal material graphs
  OUT: raw client Cypher | inventing Acc from security filters
  OUT: workspace admin ⇒ content.read without break-glass
```

## Cross-refs

- Why → [00-why.md](00-why.md)  
- Catalog → [12-role-attribute-catalog.md](12-role-attribute-catalog.md)  
- Architecture → [04-target-architecture.md](04-target-architecture.md)  
- Edge cases → [09-edge-cases.md](09-edge-cases.md)  
- Design review → [15-design-review.md](15-design-review.md)  
- Honest assessment → [13-honest-assessment.md](13-honest-assessment.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
