# OODA-04 Observe Phase

## Date: 2026-01-31

## Focus: Multiple UTF-8 Slicing Panics and Tokio Runtime Panic

## Observations

### Test Execution

After applying OODA-03 fixes, uploaded `AgenticPlatformReference Architecture.pdf` (40 pages) via Playwright UI.

### Backend Crash Analysis

Multiple panics observed in backend logs during PDF processing:

#### Panic 1: block_builder.rs:136

```
thread panicked at edgequake/crates/edgequake-pdf/src/backend/block_builder.rs:136:26
byte index 80 is not a char boundary
```

#### Panic 2: layout_processing.rs (multiple locations)

```
thread panicked at edgequake/crates/edgequake-pdf/src/processors/layout_processing.rs
byte index 50 is not a char boundary
```

#### Panic 3: text_cleanup.rs (multiple locations)

```
thread panicked at edgequake/crates/edgequake-pdf/src/processors/text_cleanup.rs
byte index 50 is not a char boundary
```

#### Panic 4: pipeline_progress_callback.rs:236

```
thread panicked: there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

### Root Cause Pattern

Same UTF-8 slicing pattern identified in OODA-03 exists in 3 additional files:

- `block_builder.rs` (1 location)
- `layout_processing.rs` (4 locations)
- `text_cleanup.rs` (3 locations)

Plus a completely different issue:

- `pipeline_progress_callback.rs`: `tokio::spawn` called from Rayon thread pool

### Evidence

Backend log output:

```
2026-01-31T16:04:14.xxx ERROR: byte index 80 is not a char boundary
```

Tokio panic:

```
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

## Key Findings

1. **UTF-8 Safe Slicing Not Applied Globally**: OODA-03 only fixed `text_grouping.rs`, but 3 other files had the same pattern
2. **Sync-to-Async Bridge Missing**: PDF extraction runs in Rayon thread pool (sync), but `tokio::spawn` requires Tokio runtime context
3. **No Defensive Boundary Checking**: All debug log statements use direct byte slicing without validation

## Impact

- PDF upload fails for any document containing multibyte UTF-8 characters near truncation boundaries
- Progress updates fail silently due to Tokio runtime panic
- System appears to hang during PDF processing

## Files Affected

1. `edgequake-pdf/src/backend/block_builder.rs`
2. `edgequake-pdf/src/processors/layout_processing.rs`
3. `edgequake-pdf/src/processors/text_cleanup.rs`
4. `edgequake-api/src/pipeline_progress_callback.rs`
