# P1 — JSON DRY + helpers SRP + LLM parse unification

**Status:** ✅ Proven  
**Date:** 2026-06-04 (re-verified)

## JSON / parser consolidation

| ID | Fix |
|----|-----|
| PIPE-DRY-003 | `prompts/json_extract.rs` — single `extract_json_from_response` |
| PIPE-DRY-004 | `prompts/json_prompts.rs` — shared extraction + gleaning prompts |
| PIPE-DRY-005 | `GleaningExtractor` → `JsonExtractionParser` |
| PIPE-DRY-007 | `merger/metadata.rs` — `TenantScope` + vector metadata builders |
| PIPE-SOLID-D-001 | `LLMExtractor::parse_response` → `JsonExtractionParser::parse_with_options` |

## `JsonParseOptions` (LLM path)

| Option | Purpose |
|--------|---------|
| `entity_schema` | Workspace type enforcement / remapping |
| `recover_truncated` | Suffix-based recovery for truncated LLM JSON |
| `empty_on_missing_json` | Tolerant empty result vs hard error |

## Helpers SRP split (PIPE-SOLID-S-001)

Monolithic `pipeline/helpers.rs` (~1,003 LOC) split into:

| Module | LOC | Responsibility |
|--------|-----|----------------|
| `helpers/stats.rs` | ~142 | Link extractions, aggregate stats, init chunk stats |
| `helpers/embeddings.rs` | ~763 | Token-budget embedding batching + `generate_all_embeddings` |
| `helpers/lineage.rs` | ~73 | `build_lineage` |
| `helpers/mod.rs` | ~12 | Module facade |

## Commands

```bash
cargo test -p edgequake-pipeline --test spec017_pipeline_contract   # 6/6
cargo test -p edgequake-pipeline extractor::llm::tests              # parse_response recovery + normalization
cargo test -p edgequake-pipeline prompts::parser::json_parser       # JsonParseOptions
cargo test -p edgequake-pipeline --lib                              # 201/201
```

## Evidence

- `test_parse_response_recovers_partial_json` — truncated JSON suffix recovery via shared parser.
- `test_llm_options_normalizes_entity_names` — `"The Company"` → `COMPANY` with schema.
- `e2e_pipeline_tests::test_llm_extractor_with_mock` expects normalized key `EDGEQUAKE` (not raw LLM casing).
