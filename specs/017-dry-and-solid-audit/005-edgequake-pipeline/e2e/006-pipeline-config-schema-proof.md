# P2 — Pipeline config/types split + schema trait

**Status:** ✅ Proven  
**Date:** 2026-06-04

## Changes

| ID | Module | Fix |
|----|--------|-----|
| PIPE-SOLID-I-001 | `pipeline/config.rs` (~163 LOC) | `PipelineConfig`, env defaults, `from_env()` |
| PIPE-SOLID-I-001 | `pipeline/types.rs` (~210 LOC) | `ProcessingResult`, stats, progress callbacks |
| PIPE-SOLID-I-001 | `pipeline/mod.rs` (~213 LOC) | Pipeline struct + builder only (was ~756 LOC) |
| PIPE-DRY-008 | `extractor/schema.rs` | `ConfigurableEntitySchema` trait; LLM + SOTA forwarders |

## LOC

| File | Before | After |
|------|--------|-------|
| `pipeline/mod.rs` | ~756 | ~213 |
| `pipeline/config.rs` | — | ~163 |
| `pipeline/types.rs` | — | ~210 |

## Commands

```bash
cargo test -p edgequake-pipeline --test spec017_pipeline_contract   # 10/10
cargo test -p edgequake-pipeline pipeline::                          # mod tests pass
cargo clippy --workspace --all-targets -- -D warnings
```

## Contract test

`spec017_configurable_entity_schema_trait` — both `LLMExtractor` and `SOTAExtractor` implement shared schema builders.
