# 013 — Implementation Assessment

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28  
**Last verified:** 2026-06-28 (Phases 0–2, Phase 4 + markdown/PDF artifacts; code-is-law)  
**Assessor:** Code-is-law pass (Phases 0–4 primary; 5 cross-ref)  
**Verdict:** **Phases 0–2 COMPLETE** — artifact API incl. markdown/PDF; 43 REST tests green

---

## Executive Summary

SPEC-028 Phases 0–2 are implemented in `edgequake-api`: spec freeze (Phase 0), retrieval SSOT `QueryContextService` (Phase 1), and REST endpoints with contract + E2E gates (Phase 2). Phases 3–5 are also complete per [011-implementation-plan-phases.md](./011-implementation-plan-phases.md); Phase 6 remains deferred.

| Phase | Plan outcome | Status | Evidence |
|-------|--------------|--------|----------|
| 0 | Spec freeze | ✅ | 14 docs + `012-code-is-law-verdict.md` |
| 1 | QueryContextService | ✅ | `query_context.rs`, mapper, 4 unit tests |
| 2 | REST endpoints | ✅ | 4 routes + artifact API (5 types), 21 contract + 22 E2E |
| 3 | DRY generation path | ✅ | `query_generation.rs`, parity |
| 4 | Agent metadata + stream v3 | ✅ | coverage, fingerprint, hints, stream v3 bundle SSE |
| 5 | MCP exposure | ✅ | native `/mcp` + 58 MCP tests |
| 6 | Native agent crate | ⏸ | Out of scope |

---

## Phase 0–2 Code-Is-Law Assessment

### Phase 0 — Spec Freeze

| Artifact | Path | Status |
|----------|------|--------|
| First principles | `001-first-principles-five-whys.md` | ✅ |
| Agentic Search lens | `002-agentic-search-paradigm-lens.md` | ✅ |
| Pipeline audit | `003-code-is-law-current-pipeline.md` | ✅ |
| DTO contract | `005-dto-model-contract.md` | ✅ |
| Edge cases | `009-edge-cases-invariants.md` | ✅ |
| Authority verdict | `012-code-is-law-verdict.md` | ✅ |

**Gate:** No spec/code conflict on DTO shapes or endpoint paths for Phase 2 delivery.

### Phase 1 — QueryContextService

| Module | Responsibility | SRP |
|--------|----------------|-----|
| `handlers/context_types.rs` | Request/response DTO SSOT | ✅ |
| `services/context_bundle_mapper.rs` | Engine → `ContextBundle` mapping | ✅ |
| `services/query_context.rs` | Retrieve / search / fetch orchestration | ✅ |
| `services/retrieval_id_cache.rs` | TTL cache for `retrieval_id` | ✅ |

**Mapper unit tests** (`context_bundle_mapper.rs`):

- `agent_granularity_includes_full_chunk`
- `empty_context_zero_coverage`
- `fingerprint_is_stable`
- `build_stats_from_response`

**Bypass guard:** `retrieve_context` rejects bypass mode (QRY-014) — verified in contract + E2E.

### Phase 2 — REST API + Tests

| Endpoint | Handler | OpenAPI |
|----------|---------|---------|
| `POST /api/v1/query/context` | `retrieve_query_context` | ✅ |
| `POST /api/v1/query/context/search` | `search_query_context` | ✅ |
| `GET /api/v1/query/context/{retrieval_id}` | `fetch_query_context` | ✅ |
| `GET /api/v1/query/context/artifacts/{type}/{id}` | `get_context_artifact` | ✅ |

**Handler consolidation:** Plan listed three files; code uses `handlers/query/context.rs` — acceptable (single HTTP adapter module).

**Artifact retrieval (Phase 2 extension):**

| Module | Role |
|--------|------|
| `services/artifact_retrieval.rs` | SSOT: document / chunk / figure / markdown / pdf fetch |
| `services/document_body_loader.rs` | DRY: KV + PDF pipeline markdown hydration |
| `handlers/context_types.rs` | `ContextArtifactResponse` DTOs |

Agents use lineage IDs from `ContextBundle` → `GET /query/context/artifacts/...`. For PDF sources, fetch `markdown/{document_id}` or `pdf/{pdf_id}?include_content=true`.

---

## Phase 4 — Agent Metadata + Stream v3

| Component | File | Role |
|-----------|------|------|
| Coverage heuristic | `context_bundle_mapper.rs` | `compute_retrieval_quality` — top-5 score mean, `empty_context` flag |
| Agent hints | `context_bundle_mapper.rs` | `build_agent_hints` — `suggested_followups`, `documents_touched` |
| Fingerprint | `context_bundle_mapper.rs` | `compute_retrieval_fingerprint` — `sha256:` deterministic hash |
| Response wiring | `query_context.rs` | Populates `retrieval_quality`, `agent_hints`, `retrieval_fingerprint` |
| Stream v3 | `query_stream.rs` | `stream_format=v3` → `Context.bundle` via shared mapper |

**SOLID / DRY (Phase 4):**

| Principle | Grade | Notes |
|-----------|-------|-------|
| SRP | A | Heuristics in mapper; stream handler only toggles bundle emission |
| DRY | A | REST and stream v3 share `map_query_context_to_bundle` |
| OCP | A | v1/v2/v3 stream formats coexist without engine changes |

**QRY disposition (Phase 4):**

| ID | Status | Evidence |
|----|--------|----------|
| QRY-008 | **FIXED** | `retrieval_quality`, `agent_hints` E2E |
| QRY-012 | **FIXED** | Stream v3 `bundle` on context event E2E |
| QRY-013 | **FIXED** | `retrieval_fingerprint` deterministic E2E |

**Phase 4 E2E tests (6 new):**

- `ec_retrieval_quality_coverage_score_in_range`
- `ec_agent_hints_include_suggested_followups`
- `ec_retrieval_fingerprint_deterministic`
- `ec_expired_retrieval_id_returns_410` (EC-09)
- `ec_stream_v3_context_event_includes_bundle`
- `ec_stream_v2_context_event_omits_bundle`

---

## SOLID / DRY Assessment (Phases 1–2)

| Principle | Implementation | Grade |
|-----------|----------------|-------|
| **SRP** | Retrieval service ≠ HTTP handlers ≠ bundle mapper | A |
| **OCP** | MCP/REST add adapters; engine unchanged | A |
| **LSP** | Legacy `/query` + `context_only` preserve shapes | A |
| **ISP** | Context endpoints reject bypass; granularity optional | A |
| **DRY** | KV enrichment reused; `query_request_builder` for engine params; `document_body_loader` shared by artifact API + `GET /documents/{id}` | A |

---

## QRY Finding Disposition (Phases 1–2 scope)

| ID | Status | Evidence |
|----|--------|----------|
| QRY-004 | **FIXED** | `ContextBundle` full subgraph in mapper |
| QRY-005 | **FIXED** | `context_types.rs` DTO SSOT |
| QRY-006 | **FIXED** | `content_granularity=agent` full chunk text |
| QRY-007 | **FIXED** | `retrieval_id` + TTL cache + fetch E2E |
| QRY-009 | **FIXED** | Dedicated `/query/context`; `context_only` deprecated |
| QRY-011 | **FIXED** | `services/query_context.rs` |
| QRY-014 | **FIXED** | Bypass rejected — E2E + service guard |
| QRY-015 | **PARTIAL** | Truncation struct present; dropped counts heuristic-only |

Phases 3–5 additionally fix QRY-001..QRY-003, QRY-008, QRY-010, QRY-012, QRY-013 — see [012-code-is-law-verdict.md](./012-code-is-law-verdict.md).

---

## Test Coverage (Phase 2 gate)

```bash
cargo test -p edgequake-api --features postgres \
  --test spec028_api_contract \
  --test spec028_context_e2e
```

| Suite | Tests | Scope |
|-------|-------|-------|
| `spec028_api_contract` | 21 | Code-is-law static contracts (incl. artifact SSOT) |
| `spec028_context_e2e` | 22 | REST + stream v3 + artifact edge cases (incl. markdown/PDF) |
| **REST total** | **43** | |

**Result (2026-06-28):** 43/43 passed.

### MCP suites (Phase 5 cross-ref)

| Suite | Tests |
|-------|-------|
| `spec028_mcp_e2e` | 19 |
| `spec028_mcp_transport` | 21 |
| `spec028_mcp_oauth_e2e` | 15 |
| `spec028_mcp_registry` | 3 |
| **MCP total** | **58** |

**Regression gate:** `spec027_e2e` must remain green.

---

## Known Limitations

1. **QRY-015 partial:** `truncation.dropped_*` counts not yet sourced from engine metadata.
2. **EC-01..EC-18:** Core Phase 2 paths covered; full 18-case catalog not exhaustively E2E-tested.
3. **Phase 6:** No `edgequake-agent` crate — intentional deferral.

---

## Recommendation

**Phases 0–2:** Mark **COMPLETE** — code is law; artifact API live (document/chunk/figure/markdown/pdf); 43 REST tests green.

**Follow-up:** QRY-015 engine-sourced truncation counts; Phase 6 agent loop.
