# Iteration 33 - Decide

**Date:** 2026-01-08  
**Focus:** Ambiguous glob re-exports cleanup

## Decision

**Rename all conflicting helper functions to module-specific names.**

### Changes Required

| File                   | Old Name         | New Name                       |
| ---------------------- | ---------------- | ------------------------------ |
| chat_types.rs          | `default_stream` | `chat_default_stream`          |
| conversations_types.rs | `default_limit`  | `conversations_default_limit`  |
| conversations_types.rs | `default_stream` | `conversations_default_stream` |
| documents_types.rs     | `default_true`   | `documents_default_true`       |
| entities_types.rs      | `default_true`   | `entities_default_true`        |
| graph_types.rs         | `default_limit`  | `graph_default_limit`          |
| ollama_types.rs        | `default_stream` | `ollama_default_stream`        |
| workspaces_types.rs    | `default_limit`  | `workspaces_default_limit`     |

### Implementation Steps

1. Rename function definitions in each `*_types.rs` file
2. Update all `#[serde(default = "...")]` references
3. Run clippy to verify warnings resolved
4. Run tests to verify no regressions

### Risk Mitigation

- Only internal helpers are affected
- API contract unchanged
- Serde deserialization behavior unchanged
