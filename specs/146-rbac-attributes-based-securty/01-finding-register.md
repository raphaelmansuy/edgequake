# 01 — Finding Register (SPEC-146)

Status legend: **OPEN** (still gaps) · **CLOSED** (code is law) · **LOCKED** (decision recorded) · **MITIGATE** (wave noted) · **SKIP** (documented honest skip).

Updated: 2026-09-06 polish wave (degrees/batch, chat ZERO_AUTHZ, list DTO, Cedar fail-closed, operator UX).

| ID | Finding | Severity | Law | Wave | Status |
|----|---------|----------|-----|------|--------|
| F-146-01 | No document ACL / classification columns; only `documents.metadata` JSONB | P0 | LAW-146-8,14 | M1a | **CLOSED** (migration 150 + dual-write) |
| F-146-02 | Upload UI sends no security labels (dropzone / PDF FormData) | P0 | LAW-146-8 | M1b | **CLOSED** (SecurityFieldsForm) |
| F-146-03 | `document_filter` trusted as-is; empty → all-pass | P0 | LAW-146-5 | M1a–M2 | **CLOSED** (allow-set ∩ filter) |
| F-146-04 | Typed ANN filters `workspace_id` only; `document_ids` ignored | P0 | LAW-146-9 | M2 | **CLOSED** (ANY/UNNEST path); live ANN seed SKIP if impractical |
| F-146-05 | `chunk_embeddings` has no `document_id` (requires JOIN `chunks`) | P0 | LAW-146-9 | M2 | **CLOSED** (JOIN + filter) |
| F-146-06 | Graph hops / BFS / PPR filter tenant+workspace only — shared entity pivot | P0 | LAW-146-10 | M3 | **CLOSED** (`edge_in_allow` + neighborhood) |
| F-146-07 | Hub descriptions denormalize multi-doc secrets at extract | P0 | LAW-146-10 | M3 | **CLOSED** (sanitize description + properties) |
| F-146-08 | Community / report embeddings workspace-scoped; mixed-label globals | P0 | LAW-146-11 | M3 | OPEN (deferred community reports) |
| F-146-09 | Popular-node / degree APIs are cross-doc side channels | P1 | LAW-146-11 | M3 | **CLOSED** (degrees/batch PEP + authorized degree) |
| F-146-10 | Keyword + answer caches omit principal / policy_generation | P0 | LAW-146-12 | M4 | **CLOSED** (authz cache scope) |
| F-146-11 | Context result cache hashes `allowed_document_ids` but default path passes **`None`** | P0 | LAW-146-12 | M4 | **CLOSED** |
| F-146-12 | MCP `RetrievalIdCache` / `ret_*` unbound to searcher | P0 | LAW-146-12,13 | M4 | **CLOSED** |
| F-146-13 | MCP workspace claim weak (missing claim → allow) | P1 | LAW-146-4,13 | M4 | **CLOSED** (fail-closed bind) |
| F-146-14 | No `/members` HTTP API; membership only via sync helpers | P1 | LAW-146-1,15 | M1b | **CLOSED** (authz members PAP) |
| F-146-15 | `Permission` enum unused at `edgequake-api` handlers | P1 | LAW-146-3,15 | M0–M1a | **CLOSED** (ingest + set_labels PEPs) |
| F-146-16 | WebUI roles ≠ API `admin`/`user`/`readonly` | P1 | LAW-146-15 | M0 | **CLOSED** (aligned; WS roles separate) |
| F-146-17 | Stored API-key scopes only toggle Admin vs User | P0 | LAW-146-13,19 | M1a | OPEN (tagged principals; deferred polish) |
| F-146-18 | Master/env key is ambient Admin; not break-glass audited | P0 | LAW-146-13,19 | M5 | MITIGATE (break-glass audited; master key policy deferred) |
| F-146-19 | Document list is **KV metadata scan**; no user ACL filter | P0 | LAW-146-6,17 | M1a | **CLOSED** (allow-set filter on list) |
| F-146-20 | Parse job GET is authenticated but **IDOR** | P1 | LAW-146-4,6 | M4 | **CLOSED** (bound + fail-closed) |
| F-146-21 | Graph REST bypasses query document filter | P0 | LAW-146-4,10 | M3–M4 | **CLOSED** (graph HTTP PEP) |
| F-146-22 | Citations / `document_name` can widen titles beyond prompt | P1 | LAW-146-7 | M4 | **CLOSED** (existence-hiding + ZERO_AUTHZ) |
| F-146-23 | RLS is tenant/workspace — not document allow-set | P1 | LAW-146-1,17 | M1a | **SKIP** (API PEP SSOT; LAW-146-17) |
| F-146-24 | JWT login stamps default tenant/workspace; strict bind default off | P1 | LAW-146-1,20 | M1a | MITIGATE (strict headers when ABAC on) |
| F-146-25 | Ingestion workers have no Worker principal kind | P0 | LAW-146-13,19 | M1a | OPEN (worker principal deferred) |
| F-146-26 | No quarantine / `labeling_failed` security phase | P1 | LAW-146-8 | M1a | **CLOSED** (quarantine + Retry UX) |
| F-146-27 | Deleted-doc residual embeddings lack tombstone authz | P1 | LAW-146-9 | M2–M4 | OPEN |
| F-146-28 | No policy store / policy_generation / Cedar schema | P0 | LAW-146-2,18,22 | M0–M1a | **CLOSED** (+ Cedar fail-closed on diagnostics) |
| F-146-29 | No attribute catalog or principal_attributes PIP | P0 | LAW-146-3,8 | M1b | **CLOSED** |
| F-146-30 | SSE / tool messages can stream then redact | P1 | LAW-146-7 | M4 | **CLOSED** (empty-allow short-circuit before LLM) |
| F-146-31 | OIDC claim → subject attr mapping undefined | P2 | LAW-146-1 | M1b–M5 | OPEN (Advanced/deferred) |
| F-146-32 | Acc impact of stricter entity descriptions UNCONFIRMED | P2 | — | M5 | LOCKED (measure, don’t invent) |
| F-146-33 | List/search/autocomplete ignore SQL RLS; KV-only path needs PEP | P0 | LAW-146-17 | M1a | **CLOSED** (PEP on list/search) |
| F-146-34 | `status_counts` global leaks unauthorized corpus size | P1 | LAW-146-7,17 | M1a | **CLOSED** (filtered counts / NaN fix) |
| F-146-35 | Master/worker cannot fit UUID-only `document_acl.principal_id` | P0 | LAW-146-19 | M1a | **CLOSED** (tagged ACL) |
| F-146-36 | Admit dual-writes SQL + KV; SQL-only labels leave list unlabeled | P0 | LAW-146-8,17 | M1a | **CLOSED** (list DTO + merge projection incl. export/pii/project) |
| F-146-37 | Dual Cedar↔SQL evaluators would fork PDP | P0 | LAW-146-18 | M1a | LOCKED (forbid) |
| F-146-38 | Allow-set scale: large `ANY(uuid[])` + HNSW underfill | P0 | LAW-146-21 | M2 | MITIGATE (over-fetch + filter; scale follow-up) |
| F-146-39 | `policy_version` ambiguous | P0 | LAW-146-22 | M1a | **CLOSED** (`policy_generation` monotonic) |
| F-146-40 | Deny decisions not observable to operators without leaking to users | P1 | LAW-146-23 | M1a | **CLOSED** (audit sink; existence-hiding to users) |
| F-146-41 | Cedar per-request cost on large classified corpora | P1 | LAW-146-18 + cache | M1a | MITIGATE (cache by generation) |

## Remaining OPEN (honest backlog)

- F-146-08 community/report embeddings
- F-146-17 API-key principal tagging polish
- F-146-25 worker principal kind
- F-146-27 delete tombstone authz
- F-146-31 OIDC claim map (Advanced)

## Finding detail (selected)

### F-146-23 — Document allow-set RLS

Honest **SKIP**: Postgres RLS on `documents` for allow-set is not the SSOT. API PEP + allow-set filter remain the gate (LAW-146-17). Live gate `g146_15` documents SKIP when no app-role RLS.

### F-146-09 — Degree topology (CLOSED 2026-09-06)

`POST /graph/degrees/batch` omits unauthorized nodes; popular/neighborhood advertise authorized degree only.
