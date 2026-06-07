# P2 — Extractor split + progress DRY + parser registry + processing dedupe

**Status:** ✅ Proven (partial — full progress collapse deferred)  
**Date:** 2026-06-04

## Changes

| ID | Module | Fix |
|----|--------|-----|
| PIPE-SOLID-S-002 | `extractor/{types,temperature,completion_options}.rs` | mod.rs ~194 LOC (was ~520) |
| PIPE-DRY-008 | `extractor/completion_options.rs` | `extraction_completion_options`, `assign_token_usage`, `recommended_chunk_size_for_bytes` |
| PIPE-DRY-006 | `progress/stage_status.rs` | Single `StageStatus` for `progress` + `ingestion_types` |
| PIPE-SOLID-O-001 | `prompts/parser/registry.rs` | `ExtractionResultParser` trait + `detect_format_markers` |
| PIPE-SOLID-I-001 | `pipeline/processing.rs` | `finish_document_processing` shared by process variants |

## Contract tests (9/9)

```bash
cargo test -p edgequake-pipeline --test spec017_pipeline_contract
```

New assertions:
- `spec017_extraction_completion_options_reasoning_models` — gpt-5-nano omits temperature
- `spec017_detect_format_markers_tuple_and_json`
- `spec017_shared_stage_status_across_modules`

## Limits

- SOTA vs LLM still have separate extract loops (retry vs single-shot) by design.
- `UnifiedIngestionProgress` vs `progress::IngestionProgress` structs not merged (different serde shapes).
