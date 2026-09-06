# EdgeQuake — Document-Level RBAC + ABAC (Graph + Retrieval)

Captured: September 5, 2026
Core Concept: Document-level RBAC + ABAC for EdgeQuake GraphRAG: provenance-gated graph hops, ANN pre-filter (not post-filter-only), fail-closed query/MCP/citations/caches. Closes workspace-only isolation gap for mixed-classification enterprise corpora.
Domain: Architecture
Next Steps: 1) Engineering channel review (AI/DB/System Design/Algorithms/Backend + UI/UX). 2) Choose PDP (Cedar vs OPA). 3) M0 attr schema + threat harness. 4) Assign official SPEC when funded.
Priority: 5
Stage: Developing
Tags: High Impact, Long-term, Technical
Type: Framework

<aside>
🔐

**Status:** Ideas Lab draft. As-of Saturday 5 September 2026 HKT.

Baseline: workspace-walled GraphRAG (JWT/API keys, SPEC-027) — **no document-level RBAC/ABAC**. OIDC/SSO still leans oauth2-proxy; Vertex OAuth ≠ end-user ABAC.

Related: [EdgeQuake — Native Word (DOCX) Support in Rust](https://app.notion.com/p/EdgeQuake-Native-Word-DOCX-Support-in-Rust-3d2887c34164811fbcd5ff29338702c1?pvs=21) (security attrs must attach at document mint). Local SSOT: `/workspace/edgequake-security-spec/SPEC.md`.

No invented Acc/latency — **TARGET** / **UNCONFIRMED** only. Generic enterprise framing only.

</aside>

## WHY

Workspace walls (`tenant_id` / `workspace_id`) are necessary but **not sufficient** for mixed-classification corpora.

1. GraphRAG **amplifies** leakage: hybrid/global hops can surface facts the user never opened as documents.
2. Need-to-know / regulated patterns need **subject + resource + environment** attributes (NIST SP 800-162), not only coarse roles.
3. Fail-closed retrieval: post-LLM redaction cannot fix ANN + shared ontology side channels.
4. Ingestion workers, query agents, and MCP tools are confused-deputy risks if they inherit workspace admin.

**Outcome:** Document-level RBAC + ABAC that preserves GraphRAG quality for *authorized* neighborhoods while denying unauthorized chunks, edges, titles, and residual embeddings.

## Current baseline

Auth ON by default (SPEC-027): JWT, `X-API-Key`, master/env keys; users in Postgres. Isolation: tenant → workspace. Roles: at least `admin` documented; fine-grained matrix **UNCONFIRMED**. No document ACL, classification attributes, or PDP. `document_filter` is convenience (date/title), not authz. See `/workspace/edgequake-security-spec/research-notes.md`.

## Deep brainstorm — RBAC

### Resource hierarchy

```
Tenant → Workspace → Document → Chunk (+ embedding)
                  → EntityOccurrence / Edge (provenance)
                  → Query / Graph explore / Ingest / MCP
```

Parent permission does **not** auto-imply document read for sensitive corpora.

### Roles (proposed)

| Role | Intent |
| --- | --- |
| `viewer` | Read docs + query + graph (authorized set) |
| `editor` | Upload/update/reprocess; delete own |
| `admin` | Workspace settings, members, policies |
| `ingestion_service` | Admit/convert/insert only; no query |
| `query_agent` | Query/chat/MCP; no ingest/admin |
| `auditor` / `break_glass` | Optional; dual-control + loud audit |

API keys are **principals**, not superusers. Default key = workspace-scoped viewer/query unless minted otherwise. Master key = break-glass style, never silent all-doc query.

Permissions TARGET: `document.read/list_meta/write/delete/set_labels`, `chunk.read`, `graph.read`, `query.execute`, `workspace.manage`, `policy.manage`, `audit.read`, `mcp.invoke`. Prefer existence-hiding 404 over 403 for unauthorized document IDs when policy requires. `list_meta` must not leak unauthorized titles.

## Deep brainstorm — ABAC on documents

### Attribute classes

- **Subject:** clearance, dept, geo, employment, citizenship, need-to-know tags, IdP groups.
- **Resource (document):** classification, project, export-control, owner org, retention, PII flag, share mode.
- **Environment:** time window, network zone, device trust.

### Policy

Deny-overrides. Fail-closed on missing attrs for classified share modes. Policy language options: **Cedar** or **OPA Rego** (prefer app PDP) + versioned Postgres policy store; PostgreSQL RLS as backstop — **not** post-filter-only.

### Inheritance

Document → chunks → embeddings → EntityOccurrence / Edge provenance. Entity hub nodes stay non-secret; secret lives on occurrences/edges. Conflict: specificity + deny wins.

## Impact on the graph

**Ban label-union on entities.** If doc A (Secret) and doc B (Public) both mention Entity X, a Public viewer must not learn Secret facts via hops.

Design: **provenance-gated edges/occurrences** + **per-hop authz** (closes hybrid retrieval-pivot / RPR-class leakage). Query-time filter preferred over materializing per-principal graphs (cost), with optional materialized authorized views for hot workspaces (**TARGET**, measure later).

Ontology / global entities: keep hubs; never attach classification solely on the hub.

## Impact on retrieval

| Stage | Requirement |
| --- | --- |
| ANN / vector | Pre-filter / pushdown (metadata, ACL bitmap, or partitioned indexes). Post-filter alone is insufficient. |
| naive / local / global / hybrid / mix | Each mode gated; per-hop checks on graph expansion. |
| Rerank / LLM / SSE | Never include unauthorized chunks as “hints”. |
| Caches | Keyed by principal + `policy_version`. |
| Citations | Must not reveal unauthorized titles/paths. |

Unauthorized context rate in harness: **TARGET 0**.

## Architecture (PEP / PDP / PIP)

```
API / MCP (PEP) → PDP (Cedar/OPA) → PIP (Postgres attrs, IdP claims, policy store)
Workers (ingest) mint document attrs; never widen query principal
AGE queries carry authorized occurrence/edge predicates
RLS backstop on document/chunk tables
```

Checks live at: admit API, query/chat handlers, graph explore, parse endpoint, MCP tools, worker task claims.

## Data model (proposal)

- `documents`: classification, export_control, pii, project_id, owner_org, retention, share_mode, attr JSONB, `policy_etag`
- `chunks` / embeddings: inherit document_id; optional denormalized filter columns for ANN
- `document_acl` / principals / groups / role_bindings
- `policies` + `policy_versions` (immutable text + hash)
- AGE: EntityOccurrence + Edge carry `doc_id` + security label; Entity hub unlabeled for secrets

## Threat model

Confused deputy (workers/MCP), prompt-injection exfil, graph side channels, timing, deleted-doc residual embeddings, cross-principal cache hits, title leakage via citations/autocomplete.

## Milestones

M0 threat model + attr schema spike → M1 document ACL + PDP allow/deny on `document.read` → M2 chunk/ANN pre-filter → M3 provenance-gated graph hops → M4 query/MCP/citation/cache hardening → M5 break-glass, auditor, measured harness ACs.

## Non-goals (v1)

Encrypted ANN; per-token ACL; perfect layout redaction inside images; inventing Acc wins from security filters.

## Acceptance criteria (TARGET)

Unauthorized chunk never in LLM context (harness rate 0). Unauthorized title never in citations/list. Graph hop cannot pivot Secret→Public. Cache miss across principals/policy versions. Ingestion service cannot query. Break-glass audited. RLS denies direct SQL bypass for app role.

## Open questions

1. Cedar vs OPA as default PDP?
2. Existence-hiding 404 vs 403 default?
3. Materialized authorized graphs vs query-time only?
4. How to label vision-derived assets / figures?
5. SSO claim mapping when oauth2-proxy is the IdP edge?

## Sources

EdgeQuake FAQ / configuration / runtime-auth-hardening / SPEC-027 / REST API · NIST SP 800-162 ABAC · industry RAG permission-aware retrieval patterns · companion [research-notes.md](http://research-notes.md).

**Review:** EdgeQuake Engineering channel + EQ UI/UX (1:1) for empty states and citation UX.

## UX review notes (EQ UI/UX — 5 Sep 2026)

Folded from specialist review. Normative for WebUI when this SPEC is funded.

### Empty states (zero authorized hits)

- Same copy as a true-empty corpus: “No matching results.” Never “some results hidden,” “0 of N authorized,” or count badges that imply withheld hits.
- Existence-hiding: zero-authorized ≡ zero-results for query/search/browse. Help text = refine terms/filters only — never imply classified matches exist.
- List/browse with only unauthorized docs: same empty pattern as an empty workspace.

### Citation UI (no title leak)

- Citations only from post-PDP authorized chunks. Drop unauthorized cites entirely — no greyed “Restricted,” truncated title, or path.
- Prefer omit over “Source unavailable” if the unavailable id could be enumerated; if a generic fallback is needed, no title/path/doc id.
- Autocomplete / `list_meta` / SSE citation payloads: authorized set only; never stream then redact.

### 404 vs 403

- Default for document/chunk/graph resource IDs: **404** + operator copy “Document not found” (never Access denied) when policy requires existence-hiding.
- Keep **403** for capability denials that aren’t existence secrets (can’t ingest, can’t manage policy/workspace).
- Break-glass / auditor: explicit 403 + loud audit banner — separate journey from normal operators.

### `track_id` when ingest labels fail

- Fail-closed: unlabeled/classified-share mint failure → not searchable, not citable.
- Phase chip e.g. `labeling_failed` / `quarantined` on `track_id`; never “ready.”
- Error copy: “Security labels could not be applied. This document isn’t searchable until labeling succeeds.” + retry for editors. No PDP/policy internals in UI.

### UX acceptance

- Unauthorized title never in citations, lists, autocomplete, or empty-state copy.
- Zero-authz empty indistinguishable from true empty.
- Quarantined ingest never appears in search/citations.

## Engineering channel review lock (5 Sep 2026)

Consensus from EQ AI Engineer, EQ Database, EQ System Design, EQ Algorithms, EQ Backend (UI/UX notes already above).

### Normative joins

- **One PDP / one `AuthzContext` + `policy_version`** stamped at request start; PEPs = API, query engine, workers, MCP. No handler-local ACL; no second allow-set in prompt builders.
- **ANN fail-closed pre-filter**; post-filter = safety net only (underfill when authorized fraction `p` is small).
- **Provenance-gated** EntityOccurrence/edges; hub `description` assembled at query from authorized occurrences only — never denormalize multi-doc secrets at extract write.
- **Community/global summaries:** partitioned write (or control-plane rebuild by authz set) — mixed-label shared globals are a hard no.
- **API keys = principals** (default workspace viewer/query_agent); master = break-glass + audit; workers = `ingestion_service` only.
- **Existence-hiding 404/empty** for unauthorized resource IDs; MCP reject `bypass_acl`; no denied titles/counts in SSE/tool/system messages (degree side channel).
- **Caches** keyed principal (or attr hash) + `policy_version` + mode; tombstone vectors on delete.

### Architecture non-goals tightened

No per-principal materialized graphs in v1; no raw client Cypher; no “workspace admin ⇒ [content.read](http://content.read)” without break-glass; no remote PDP round-trip per hop (compile once).

### UNCONFIRMED / harness TARGETS

Filtered recall under ACL (SPEC-075-style); ListObjects vs predicate crossover as `|A|` grows; OpenAPI 401/403/404 matrix; key→principal migration; Acc impact of stricter entity descriptions — no invented benches.