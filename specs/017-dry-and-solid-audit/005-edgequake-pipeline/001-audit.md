# edgequake-pipeline — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-pipeline`  
**LOC:** ~14,341 (src)  
**Role:** Document chunking, LLM entity extraction, merging, lineage

---

## Executive Summary

Two **P0 correctness bugs** dominate: (1) **dual entity normalizers** with divergent semantics between parse and merge paths, and (2) **`Chunker::chunk()` ignores configured strategy** — production always uses sync token splitting. JSON extraction and prompts are fragmented across three parallel extractor stacks.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| PIPE-DRY-001 | **P0** | Two `normalize_entity_name` implementations | `prompts/normalizer.rs:44-72` (title-case, articles); `merger/mod.rs:192-200` (strip non-alphanumeric only) | Delete merger copy; import `prompts::normalize_entity_name` |
| PIPE-DRY-002 | **P0** | Chunker strategy bypassed in production | `chunker/mod.rs:98-101` — `chunk()` → `chunk_sync`, ignores `self.strategy`; `pipeline/processing.rs` calls `chunk()` only | Make `chunk()` delegate to strategy |
| PIPE-DRY-003 | **P1** | `extract_json_from_response` behavioral drift | `extractor/mod.rs:331-369` (handles truncated arrays) vs `prompts/parser/json_parser.rs:211-241` | Single function with truncated-array support |
| PIPE-DRY-004 | **P1** | JSON prompt duplicated | `extractor/llm.rs:67-90` inline vs `prompts/entity_extraction.rs` + `json_parser.rs` | Prompts module only |
| PIPE-DRY-005 | **P1** | Gleaning re-implements JSON parse loop | `extractor/gleaning.rs:88-159` inline prompt + manual parse | Route through `JsonExtractionParser` |
| PIPE-DRY-006 | **P2** | Dual progress type hierarchies | `progress/mod.rs:40-309` vs `ingestion_types.rs:185-336` | Collapse or newtype mapping |
| PIPE-DRY-007 | **P2** | Entity/relationship vector metadata JSON duplicated | `merger/entity.rs:19-35` vs `merger/relationship.rs:42-60` | Shared metadata builder |
| PIPE-DRY-008 | **P2** | SOTA vs LLM extractor boilerplate | Temperature gating, token accounting in `sota.rs` and `llm.rs` | `LlmExtractionRunner` helper |
| PIPE-DRY-009 | **P3** | Chunker tests embedded in mod (~400 LOC) | `chunker/mod.rs:200-646` | Move to `tests/` |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence |
|----|---|-----------|-----------|----------|
| PIPE-SOLID-S-001 | **P1** | SRP | `pipeline/helpers.rs` ~1,003 LOC | Stats, embeddings, token budget, lineage, cost |
| PIPE-SOLID-S-002 | **P2** | SRP | `extractor/mod.rs` ~609 LOC | Types + JSON recovery + trait |
| PIPE-SOLID-S-003 | **P2** | SRP | `chunker/mod.rs` ~646 LOC | Facade + sync algo + tests |
| PIPE-SOLID-L-001 | **P0** | LSP | `with_strategy()` misleading — strategy stored but unused in sync path | `chunker/mod.rs:76-77, 98-101` |
| PIPE-SOLID-L-002 | **P0** | LSP | Merger keys ≠ parser keys | `O'Brien` → different outcomes per normalizer |
| PIPE-SOLID-O-001 | **P2** | OCP | New extraction format → HybridParser + extractors + gleaning | No parser registry |
| PIPE-SOLID-I-001 | **P2** | ISP | `Pipeline` accumulates chunker, extractor, embedder, progress, cost | God orchestrator struct |
| PIPE-SOLID-D-001 | **P2** | DIP | Gleaning uses raw JSON strings vs prompt abstractions | `extractor/gleaning.rs:88-121` |

---

## Normalizer Divergence (P0 Detail)

**Prompts normalizer** (`prompts/normalizer.rs`):
- Strips articles ("The Company" → "COMPANY")
- Title-case word handling
- Possessive handling ("John's" → "JOHN")

**Merger normalizer** (`merger/mod.rs:192-200`):
```rust
name.trim().to_uppercase()
    .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
    .split_whitespace().collect::<Vec<_>>().join("_")
```

**Impact:** Entity parsed as `"The Company"` may not merge with existing `"COMPANY"` node → duplicate graph nodes.

---

## Remediation Plan

### P0 (immediate)

1. Merger imports `crate::prompts::normalize_entity_name`; delete local fn
2. Add E2E test: parse → merge path for `"The Company"`, `"O'Brien"`, `"AI/ML"`
3. Fix `Chunker::chunk()` to use strategy (or document sync-only and remove strategy API)

### P1

4. Consolidate JSON extraction + prompts into `prompts/` module
5. Split `pipeline/helpers.rs` → `embeddings.rs`, `lineage_build.rs`, `stats.rs`

### P2/P3

6. Collapse progress type hierarchies; shared merger metadata builder
7. Move chunker tests; fix doc drift (1200 vs 800 token default)

---

## Verification

```bash
cargo test -p edgequake-pipeline --lib
cargo test -p edgequake-pipeline normalize_entity_name
# Integration: entity parsed name must equal merge key
```

**Acceptance:** Single `normalize_entity_name` definition; `ChunkingStrategy` changes output when configured on `Pipeline`.
