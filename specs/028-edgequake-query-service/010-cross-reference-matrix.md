# 010 — Cross-Reference Matrix

**Spec:** 028-edgequake-query-service  
**Date:** 2026-06-28 (updated post-implementation)  
**Cross-ref:** [000-index.md](./000-index.md) | [012-code-is-law-verdict.md](./012-code-is-law-verdict.md) | [013-implementation-assessment.md](./013-implementation-assessment.md)

---

## Finding Status Legend

| Status | Meaning |
|--------|---------|
| **OPEN** | Not implemented — SPEC-028 target |
| **SPEC** | Specified in SPEC-028 — ready to implement |
| **FIXED** | Implemented and verified |
| **PARTIAL** | Implemented with known gap |
| **WONT** | Explicitly out of scope |
| **INHERITED** | Already solved — no 028 work |

---

## QRY Finding Matrix

| ID | Finding | Lens | Priority | Status | Phase |
|----|---------|------|----------|--------|-------|
| **QRY-001** | Default mode mismatch: `/query`=Mix vs `/chat`=Hybrid | 003 | P1 | **FIXED** | 3 |
| **QRY-002** | Duplicate source mapping query_execute + chat | 008 | P0 | **FIXED** | 3 |
| **QRY-003** | `include_references` dead field on QueryRequest | 003 | P2 | **FIXED** | 3 |
| **QRY-004** | Engine QueryContext stripped at HTTP boundary | 001 | P0 | **FIXED** | 2 |
| **QRY-005** | No agent-grade structured subgraph DTO | 005 | P0 | **FIXED** | 2 |
| **QRY-006** | Snippet-only chunk content in API | 005 | P0 | **FIXED** | 2 |
| **QRY-007** | No retrieval_id for MCP fetch | 007 | P0 | **FIXED** | 2 |
| **QRY-008** | No coverage / agent hint signals | 005 | P1 | **FIXED** | 4 |
| **QRY-009** | context_only coupled to generation endpoint | 001 | P0 | **FIXED** | 2 |
| **QRY-010** | No MCP search/fetch tools | 007 | P1 | **FIXED** | 5 |
| **QRY-011** | No QueryContextService module | 004 | P0 | **FIXED** | 1 |
| **QRY-012** | Stream v2 flat context only | 006 | P2 | **FIXED** | 4 |
| **QRY-013** | No retrieval_fingerprint for eval replay | 005 | P2 | **FIXED** | 4 |
| **QRY-014** | Bypass accepted on context endpoint (invalid) | 009 | P2 | **FIXED** | 2 |
| **QRY-015** | Truncation dropped counts not exposed | 009 | P1 | **PARTIAL** | 2 |

---

## Cross-Spec References

| QRY ID | Related Spec | Related Finding |
|--------|--------------|-----------------|
| QRY-001 | [024-003](../024-egdequake-audit/003-query-retrieval-audit.md) | F-05 |
| QRY-002 | [017-006](../017-dry-and-solid-audit/006-edgequake-query/001-audit.md) | QUERY-DRY-001 |
| QRY-004 | [021-02-query](../021-storage-study/03-pipelines/02-query-pipeline.md) | — |
| QRY-006 | [006-spec](../006-spec.md) | FR-002 snippets |
| QRY-007 | [007-mcp](./007-mcp-exposure-lens.md) | MCP 2026-07-28 |
| QRY-009 | [001-five-whys](./001-first-principles-five-whys.md) | FP-028-01 |
| document_filter | [005-filter](../005-filter.md) | FR-001..004 |
| auth isolation | [027-004](../027-api-edgequake-audit/004-security-oauth-lens.md) | SEC-014 |
| workspace | [027](../027-api-edgequake-audit/000-index.md) | phase 35 RLS |

---

## IMP Improvement Items

| ID | Item | Score | Compat | Phase | Status |
|----|------|-------|--------|-------|--------|
| **IMP-Q01** | QueryContextService SSOT | 10 | ascending | 1–2 | FIXED |
| **IMP-Q02** | ContextBundle DTO | 10 | additive | 2 | FIXED |
| **IMP-Q03** | POST /query/context | 9 | additive | 2 | FIXED |
| **IMP-Q04** | DRY handler refactor | 8 | internal | 3 | FIXED |
| **IMP-Q05** | Unify default mode to Mix | 7 | behavior | 3 | FIXED |
| **IMP-Q06** | Agent hints + coverage | 7 | additive | 4 | FIXED |
| **IMP-Q07** | Stream v3 bundle event | 6 | opt-in | 4 | FIXED |
| **IMP-Q08** | MCP search/fetch tools | 8 | additive | 5 | FIXED |
| **IMP-Q09** | Deprecate context_only | 5 | deprecation | 3+ | FIXED |
| **IMP-Q10** | Native Agentic Search mode | 9 | new feature | 6 | OPEN |

---

## Contract Test Coverage Map

| QRY ID | Test ID |
|--------|---------|
| QRY-004, QRY-005 | spec028_api_contract + ec_context_retrieve_returns_bundle |
| QRY-006 | ec_context_retrieve_returns_bundle |
| QRY-007 | ec_search_then_fetch_roundtrip + mcp_search_tool |
| QRY-009 | ec_context_only_parity_with_legacy_query |
| QRY-002 | spec028_source_reference_builder_dry |
| QRY-014 | ec_bypass_mode_rejected |
| QRY-015 | ec_truncation_metadata_present |
| QRY-010 | spec028_mcp_e2e |
