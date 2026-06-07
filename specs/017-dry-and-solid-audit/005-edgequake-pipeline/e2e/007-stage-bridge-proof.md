# E2E Proof — Stage Bridge (PIPE-DRY-006 partial)

**Date:** 2026-06-04  
**Result:** ✅ Contract tests + unit tests pass

## Problem

Three progress/stage hierarchies existed with duplicated conversion logic:

| Layer | Type | Wire format | Role |
|-------|------|-------------|------|
| Tasks | `PipelinePhase` | snake_case slugs | Live PDF upload WebSocket |
| Unified | `UnifiedStage` | lowercase | Frontend status badges |
| Internal | `PipelineStage` | enum | Async job tracker |

Full struct collapse (`IngestionProgress` variants) is **deferred** — incompatible serde shapes across crates.

## Fix

Centralized all stage mappings in `edgequake-pipeline/src/stage_bridge.rs`:

- `pipeline_stage_to_unified` / `unified_to_pipeline_stage`
- `tasks_phase_slug_to_unified` / `unified_to_tasks_phase_slug` (no `edgequake-tasks` dependency)
- `unified_stage_slug` for frontend wire contract
- `From<PipelineStage> for UnifiedStage`

`ingestion_types.rs` delegates `from_pipeline_stage` / `to_pipeline_stage` to the bridge.

Placed at crate root (not under `progress/`) to avoid circular dependency with `ingestion_types`.

## Contract tests

```bash
cargo test -p edgequake-pipeline --test spec017_pipeline_contract
# 12 passed (includes spec017_stage_bridge_*)
```

| Test | Proves |
|------|--------|
| `spec017_stage_bridge_pipeline_to_unified` | Internal `Extracting` ↔ unified `Extracting` |
| `spec017_stage_bridge_tasks_slug_to_unified` | `pdf_conversion` → `Converting`; slug `"extracting"` |

## Unit tests (in-module)

- Roundtrip for all `PipelineStage::all()` (Finalizing → Storing collapse)
- Tasks slug mapping
- Unified slug ↔ tasks phase slug alignment

## Honest limits

- **Not proven:** WebSocket payloads in live upload still use tasks crate types directly upstream.
- **Not proven:** Frontend badge rendering for every stage slug (Playwright covers documents empty state only).
- **Still open:** Merging the three `IngestionProgress`-like structs requires cross-crate API migration.
