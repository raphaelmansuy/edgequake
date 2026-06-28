# 011 — Implementation Plan (Phases)

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Last verified:** 2026-06-28 (Phases 0–2 + artifact API incl. markdown/PDF; code-is-law)  
**Cross-ref:** [010-cross-reference-matrix.md](./010-cross-reference-matrix.md) | [012-code-is-law-verdict.md](./012-code-is-law-verdict.md) | [013-implementation-assessment.md](./013-implementation-assessment.md)

---

## Phase Overview

```
  Phase 0        Phase 1         Phase 2          Phase 3
  Spec freeze    Service crate   API endpoint     DRY refactor
      │              │               │                │
      ▼              ▼               ▼                ▼
  ┌────────┐    ┌──────────┐   ┌────────────┐   ┌─────────────┐
  │ DTO    │    │ query_   │   │ POST       │   │ /query uses │
  │ review │    │ context  │   │ /query/    │   │ service     │
  │        │    │ .rs      │   │ context    │   │ parity tests│
  └────────┘    └──────────┘   └────────────┘   └─────────────┘

  Phase 4         Phase 5          Phase 6
  Agent hints     MCP adapter      Native agent
      │               │                │
      ▼               ▼                ▼
  coverage_score  search/fetch     edgequake-agent
  stream v3       tool schemas     in-process loop
```

---

## Implementation Status — Phases 0–2, 4 (Code Is Law)

| Phase | Verdict | Evidence |
|-------|---------|----------|
| **0** | ✅ **COMPLETE** | 14 spec docs under `specs/028-edgequake-query-service/`; authority chain in `012-code-is-law-verdict.md` |
| **1** | ✅ **COMPLETE** | `services/query_context.rs`, `services/context_bundle_mapper.rs`, `handlers/context_types.rs`; 4 mapper unit tests |
| **2** | ✅ **COMPLETE** | REST handlers, routes, OpenAPI, retrieval cache, **artifact API** (document/chunk/figure/markdown/pdf); **43 tests green** (21 contract + 22 E2E) |
| **4** | ✅ **COMPLETE** | Agent metadata + stream v3; **35 tests green** (20 contract + 15 E2E incl. Phase 4) |

**Latest verification (2026-06-28, Phases 0–2 + artifacts incl. markdown/PDF):**

```bash
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract --test spec028_context_e2e
# 21 passed + 22 passed
```

**SOLID / DRY (Phases 1–2):**

| Principle | Implementation |
|-----------|----------------|
| **SRP** | Retrieval logic isolated in `query_context.rs`; HTTP in `handlers/query/context.rs`; mapping in `context_bundle_mapper.rs` |
| **OCP** | New consumers (MCP, REST) call service layer without engine changes |
| **DRY** | Document enrichment reuses KV helpers from `handlers/query/mod.rs`; engine request via `query_request_builder`; markdown/PDF body via `document_body_loader.rs` (shared with `GET /documents/{id}`) |

**Plan deviation (acceptable):** Phase 2 planned separate `context_retrieve.rs`, `context_search.rs`, `context_fetch.rs` — implemented as single `handlers/query/context.rs` (SRP preserved: one module, three handler fns).

---

## Phase 0 — Spec Freeze (This Deliverable)

**Outcome:** SPEC-028 approved; DTO JSON examples locked.

| Task | Owner | Done when | Status |
|------|-------|-----------|--------|
| 5 Whys + FP invariants | spec | ✅ 001 | ✅ |
| Agentic Search definition | spec | ✅ 002 | ✅ |
| Code audit | spec | ✅ 003 | ✅ |
| DTO contract | spec | ✅ 005 | ✅ |
| Edge cases catalog | spec | ✅ 009 | ✅ |
| Architecture lens | spec | 004 | ✅ |
| API surface lens | spec | 006 | ✅ |
| DRY generation lens | spec | 008 | ✅ |
| Cross-reference matrix | spec | 010 | ✅ |
| Code-is-law verdict | spec | 012 | ✅ |

**Gate:** Review 012-code-is-law-verdict.md authority chain — **PASSED**.

---

## Phase 1 — QueryContextService Module

**Outcome:** `edgequake-api/src/services/query_context.rs` with unit tests.

| Task | Files | Est. | Status |
|------|-------|------|--------|
| Create `ContextBundle` types | `handlers/context_types.rs` | 4h | ✅ |
| Create mapper | `services/context_bundle_mapper.rs` | 6h | ✅ |
| Implement `retrieve()` | `services/query_context.rs` | 8h | ✅ |
| Document enrichment | reuse `handlers/query/mod.rs` KV helpers | 2h | ✅ |
| Unit tests with mock engine response | `services/context_bundle_mapper.rs` | 4h | ✅ (4 tests) |

**Dependencies:** None — wraps existing `execute_sota_query`.

**Service surface (SSOT):**

| Function | Purpose |
|----------|---------|
| `retrieve_context` | Full bundle retrieval |
| `search_context` | Search summaries + retrieval handles |
| `fetch_context_by_id` | TTL cache fetch by `retrieval_id` |

**Verification:**

```bash
# Mapper unit tests (lib)
cargo test -p edgequake-api --features postgres context_bundle_mapper

# Static contract
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract spec028_query_context_service_exists \
  spec028_context_bundle_mapper_exists
```

---

## Phase 2 — API Endpoint + Contract Tests

**Outcome:** `POST /api/v1/query/context` live; OpenAPI documented.

| Task | Files | Est. | Status |
|------|-------|------|--------|
| Handlers (retrieve, search, fetch) | `handlers/query/context.rs` | 4h | ✅ |
| Agent artifact retrieval | `services/artifact_retrieval.rs`, `services/document_body_loader.rs`, `context.rs` | 4h | ✅ |
| Routes + OpenAPI | `routes.rs`, `openapi*.rs` | 4h | ✅ |
| In-memory retrieval_id cache | `services/retrieval_id_cache.rs` | 3h | ✅ |
| Contract tests | `tests/spec028_api_contract.rs` | 8h | ✅ (21 tests) |
| E2E with postgres | `tests/spec028_context_e2e.rs` | 8h | ✅ (22 tests) |

**Fixes:** QRY-004, QRY-005, QRY-006, QRY-007, QRY-009, QRY-014 — **FIXED**; QRY-015 — **PARTIAL** (truncation struct present; dropped counts heuristic-only)

**API delivered:**

| Endpoint | Handler |
|----------|---------|
| `POST /api/v1/query/context` | `retrieve_query_context` |
| `POST /api/v1/query/context/search` | `search_query_context` |
| `GET /api/v1/query/context/{retrieval_id}` | `fetch_query_context` |
| `GET /api/v1/query/context/artifacts/{type}/{id}` | `get_context_artifact` |

**Agent artifact retrieval** (`artifact_type`):

| Type | `artifact_id` | Query params | Returns |
|------|---------------|--------------|---------|
| `document` | `document_id` | `include_content=true` optional | metadata + optional full body, markdown, PDF paths |
| `chunk` | `chunk_id` | — | full chunk text + lineage |
| `figure` | manifest `item_id` | `document_id` **required** | VLM-analyzed figure text + status |
| `markdown` | `document_id` | — | full markdown body + `content_source` (`kv` or `pdf_storage`) |
| `pdf` | PDF UUID | `document_id` optional; `include_content=true` optional | PDF metadata, download/content paths, optional markdown |

Agents follow `bundle.chunks[].lineage.document_id` / chunk IDs from context retrieve, then fetch artifacts for deep document inspection. For PDF uploads, use `markdown/{document_id}` for extracted text or `pdf/{pdf_id}` for binary metadata and legacy download paths. Legacy endpoints (`GET /documents/{id}`, `GET /documents/pdf/{id}/download`, `GET /documents/pdf/{id}/content`, `GET /chunks/{id}`) remain for WebUI.

**E2E coverage (Phase 2 scope):**

| Test | Edge case |
|------|-----------|
| `ec_empty_query_returns_422` | Invalid input |
| `ec_context_retrieve_returns_bundle` | Happy path |
| `ec_bypass_mode_rejected` | QRY-014 |
| `ec_search_then_fetch_roundtrip` | QRY-007 |
| `ec_context_only_parity_with_legacy_query` | Legacy compat |
| `ec_invalid_retrieval_id_returns_400` | Malformed ID |
| `ec_unknown_retrieval_id_returns_404` | Expired/missing |
| `ec_retrieval_response_includes_agent_hints_when_requested` | Agent metadata |
| `ec_truncation_metadata_present` | QRY-015 partial |
| `ec_artifact_document_retrieve_metadata` | Artifact API document |
| `ec_artifact_document_include_content` | Artifact API full body |
| `ec_artifact_chunk_retrieve_content` | Artifact API chunk |
| `ec_artifact_figure_requires_document_id` | Artifact API figure guard |
| `ec_artifact_invalid_type_returns_400` | Artifact validation |
| `ec_artifact_markdown_retrieve_from_kv` | Markdown artifact from KV |
| `ec_artifact_pdf_retrieve_with_markdown` | PDF artifact + markdown hydration |

**Verification:**

```bash
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract --test spec028_context_e2e
```

---

## Phase 3 — DRY Refactor Generation Path

**Outcome:** `/query` and `/chat` use `QueryContextService`; parity tests pass.

| Task | Files | Est. | Status |
|------|-------|------|--------|
| `QueryGenerationService` | `services/query_generation.rs` | 6h | ✅ |
| Refactor `query_execute.rs` | handler | 4h | ✅ |
| Refactor `chat/mod.rs` — delete `build_sources` | handler | 4h | ✅ |
| Deprecation header for `context_only` | handler | 1h | ✅ |
| Fix QRY-001 default mode | chat handler | 1h | ✅ |
| Parity tests | spec028 + spec027 regression | 4h | ✅ |

**Fixes:** QRY-001, QRY-002, QRY-003 (wire or remove `include_references`)

**Gate:** Zero diff on spec027_e2e snapshot tests.

---

## Phase 4 — Agent Metadata + Stream v3

**Outcome:** `coverage_score`, `suggested_followups`, `retrieval_fingerprint`.

| Task | Module | Est. | Status |
|------|--------|------|--------|
| Coverage heuristic (chunk+entity scores) | `context_bundle_mapper.rs::compute_retrieval_quality` | 4h | ✅ |
| Follow-up suggestion (heuristic; keyword LLM optional) | `context_bundle_mapper.rs::build_agent_hints` | 6h | ✅ |
| Fingerprint hash (query+mode+filter) | `context_bundle_mapper.rs::compute_retrieval_fingerprint` | 2h | ✅ |
| Wire metadata into retrieve response | `query_context.rs` | — | ✅ |
| Stream format v3 with bundle event | `query_stream.rs` + `QueryStreamEvent::Context.bundle` | 6h | ✅ |
| Phase 4 E2E + contract tests | `spec028_context_e2e.rs`, `spec028_api_contract.rs` | 8h | ✅ |

**Fixes:** QRY-008, QRY-012, QRY-013 — **FIXED**

**SOLID / DRY (Phase 4):**

| Principle | Implementation |
|-----------|----------------|
| **SRP** | Quality/heuristic logic in mapper; stream handler only gates `use_v3` bundle emission |
| **DRY** | Stream v3 reuses `map_query_context_to_bundle` — same mapping as REST retrieve |
| **OCP** | v1/v2/v3 stream formats coexist via `stream_format` param |

**Deliverables (code-is-law):**

| Field / feature | Source | Response location |
|-----------------|--------|-------------------|
| `retrieval_quality.coverage_score` | Top-5 score mean heuristic | `ContextRetrievalResponse` |
| `agent_hints.suggested_followups` | Entity-based or empty-context fallback | `ContextRetrievalResponse` (when `include_agent_hints`) |
| `retrieval_fingerprint` | SHA-256 of query+mode+workspace+filter | Header `X-Retrieval-Fingerprint` + body |
| Stream v3 `bundle` | `map_query_context_to_bundle` on context event | `POST /query/stream?stream_format=v3` |

**E2E coverage (Phase 4 scope):**

| Test | EC / QRY |
|------|----------|
| `ec_retrieval_quality_coverage_score_in_range` | QRY-008 |
| `ec_agent_hints_include_suggested_followups` | QRY-008 |
| `ec_retrieval_fingerprint_deterministic` | QRY-013, INV-02 |
| `ec_expired_retrieval_id_returns_410` | EC-09 |
| `ec_stream_v3_context_event_includes_bundle` | QRY-012 |
| `ec_stream_v2_context_event_omits_bundle` | v2 compat |
| `spec028_phase4_coverage_heuristic_ssot` | contract |
| `spec028_phase4_stream_v3_emits_bundle` | contract |

**EC-01..EC-18 status:** Phase 4-critical paths covered (EC-09, EC-11 via Phase 2). Remaining catalog items (EC-03 filter zero, EC-08 cross-workspace, EC-13 provenance warnings, etc.) deferred — require postgres fixtures or live provider failures.

**Verification:**

```bash
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract \
  --test spec028_context_e2e
# 20 passed + 15 passed (2026-06-28)
```

---

## Phase 5 — MCP Exposure

**Outcome:** Documented tool schemas + optional native MCP mount.

| Task | Est. | Status |
|------|------|--------|
| `mcp/tool-schemas.json` | 2h | ✅ |
| `mcp/rest-adapter-guide.md` | 2h | ✅ |
| Example Cursor MCP config | 1h | ✅ |
| Native `/mcp` handler (5b) | 16h | ✅ |
| MCP integration test | 4h | ✅ (58 tests across MCP suites) |

**Fixes:** QRY-010

See [mcp/007-sota-implementation-roadmap.md](./mcp/007-sota-implementation-roadmap.md).

---

## Phase 6 — Native Agentic Search (Future)

**Outcome:** In-process agent loop in new crate.

| Task | Est. | Status |
|------|------|--------|
| `edgequake-agent` crate scaffold | 8h | ⏸ |
| Agent controller (plan→retrieve→evaluate loop) | 24h | ⏸ |
| Budget / step limits | 4h | ⏸ |
| WebUI "Agentic Search" mode | 16h | ⏸ |

**Out of scope for initial SPEC-028 delivery** — enabled by phases 1–5.

---

## Migration / Versioning

No database migrations required — API-only feature.

| Artifact | Version bump |
|----------|--------------|
| OpenAPI | minor — new endpoints |
| REST API | minor — additive |
| `context_only` deprecation | major (v2.0) |

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Large bundle payloads | Medium | Agent timeout | `content_granularity`; gzip |
| Parity regression | Medium | Broken UI | spec027 + spec028 gates |
| MCP spec churn | Low | Rework adapter | Phase 5a REST proxy first |
| Coverage heuristic wrong | Medium | Agent over/under iterates | Tunable thresholds; log evals |

---

## Definition of Done (SPEC-028 Complete)

- [x] **Phase 0** — spec freeze + authority chain (`012`)
- [x] **Phase 1** — `QueryContextService` + mapper unit tests
- [x] **Phase 2** — `/query/context` REST + artifact API (incl. markdown/PDF) + 43 automated tests
- [x] **Phase 4** — agent metadata + stream v3; QRY-008/012/013 fixed
- [x] 21 contract tests in spec028_api_contract
- [x] 22 E2E tests in spec028_context_e2e (Phase 2 + Phase 4 + artifacts incl. markdown/PDF)
- [x] OpenAPI documented for new endpoints
- [x] MCP tool schemas published + native `/mcp` handler
- [x] 012-code-is-law-verdict updated to FIXED for QRY-004..QRY-014
- [ ] Phase 6 — `edgequake-agent` crate (future)
- [ ] QRY-015 — full truncation dropped counts from engine metadata

---

## Estimated Total Effort

| Phase | Engineering days |
|-------|------------------|
| 1 | 3 |
| 2 | 4 |
| 3 | 3 |
| 4 | 3 |
| 5 | 3 |
| **Total (excl. phase 6)** | **~16 days** |
