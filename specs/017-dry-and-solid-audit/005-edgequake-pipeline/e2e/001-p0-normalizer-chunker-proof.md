# P0 — Single normalizer + chunker strategy honored

**Status:** ✅ Proven  
**Date:** 2026-06-04 (re-verified)

## Claim

1. Parser and merger share one `normalize_entity_name` (`prompts/normalizer.rs`).
2. `Chunker::chunk()` delegates to configured `ChunkingStrategy` via `chunk_async()`.

## Commands

```bash
cargo test -p edgequake-pipeline --test spec017_pipeline_contract   # 6/6
cargo test -p edgequake-pipeline normalize_entity_name
```

## Evidence

| Case | Parser key | Merger key |
|------|------------|------------|
| `The Company` | `COMPANY` | `COMPANY` |
| `O'Brien` | `O'BRIEN` | `O'BRIEN` |
| `AI/ML` | `AI/ML` | `AI/ML` |

Sentence vs token chunking strategies produce different chunk counts (`spec017_chunker_strategy_changes_output`).
